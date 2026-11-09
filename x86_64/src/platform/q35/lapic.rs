// LAPIC for qemu's q35 machine, CPUID-gated dual-mode (x2APIC primary,
// classic xAPIC MMIO fallback) — the gic/ registry contract re-keyed
// (aarch64 platform/virt/gic/mod.rs): init discovers and software-enables
// the unit, switches it to x2APIC when CPUID.01H:ECX[21] advertises it
// (KVM's -cpu host passes the host's x2APIC through), and publishes it
// once; eoi() and the timer-LVT programming surface run lock-free from
// IRQ/boot context thereafter — a lock taken in IRQ context could
// deadlock against an interrupted holder.
//
// Why x2APIC at all (the 13-03 dispatch-3 root cause): MSR 0x832 is the
// x2APIC-mode address of the LVT-timer register (0x800+(0x320>>4)), and
// KVM gates the whole MSR range 0x800-0xBFF on the guest APIC actually
// being in x2APIC mode — any wrmsr there from xAPIC mode injects #GP(0)
// on ANY host, independent of apicv/kernel (the two host red herrings
// chased first).  The kernel now (a) enters x2APIC mode when the CPU
// advertises it, so the register file rides the MSR map, and (b) arms
// TSC-deadline at its true address 0x6E0 (timer.rs), which is valid in
// EITHER mode.  Hardware without bit 21 (TCG's qemu64) keeps the
// classic MMIO path — the gate's fallback leg still exercises it.
//
// The MMIO base address is the architectural 0xFEE0_0000 fixed-function
// hardware fact of the machine model (an ACPI MADT type-0 entry would
// name the same value; 13-04's ACPI walk cross-checks it).  MMIO access
// is map-free: l.S's bootidentitypt3 identity-maps the first 4 GiB
// PA==VA, so the aarch64 io.rs read_reg/write_reg volatile family re-keys
// over the identity window with offset bounds checks.

use core::cell::SyncUnsafeCell;
use core::ptr::{read_volatile, write_volatile};

use crate::cpu;

#[cfg(not(test))]
use port::println;

/// The spurious-interrupt vector programmed into SVR.  A spurious
/// interrupt delivers this vector; per the LAPIC spec it is NOT EOI'd
/// (it was never in service) — the trap dispatcher's spurious arm
/// returns without the EOI write.
pub const SPURIOUS_VECTOR: u8 = 0xFF;

/// The LAPIC error-report vector (LVT_ERROR).  An internal LAPIC error
/// (e.g. an illegal register access) delivers it; an unexpected firing
/// rides the dispatcher's stray line (printed, still EOI'd) — loud,
/// never silent.
pub const ERROR_VECTOR: u8 = 0xFE;

// xAPIC MMIO register offsets (Intel SDM Vol 3 Table 10-1; the 9k
// apic.c register vocabulary).
pub(crate) const TPR: usize = 0x080; // Task Priority
pub(crate) const EOI_REG: usize = 0x0B0; // EOI
pub(crate) const SVR: usize = 0x0F0; // Spurious Interrupt Vector
pub(crate) const LVT_TIMER: usize = 0x320; // LVT timer
pub(crate) const LVT_LINT0: usize = 0x350; // LVT LINT0 (legacy INTR)
pub(crate) const LVT_LINT1: usize = 0x360; // LVT LINT1 (legacy NMI)
pub(crate) const LVT_ERROR: usize = 0x370; // LVT error
pub(crate) const TIMER_ICR: usize = 0x380; // timer initial count
pub(crate) const TIMER_CCR: usize = 0x390; // timer current count
pub(crate) const TIMER_DCR: usize = 0x3E0; // timer divide configuration

/// SVR software-enable bit (bit 8).
const SVR_ENABLE: u32 = 1 << 8;
/// LVT mask bit (bit 16).
pub const LVT_MASKED: u32 = 1 << 16;
/// LVT timer mode bit: periodic (bit 17).
pub const LVT_PERIODIC: u32 = 1 << 17;
/// LVT timer mode bit: TSC-deadline (bit 18) — the D-05 mode.  Written
/// only behind the CPUID.1.ECX[24] gate (an ungated write faults or
/// hangs under TCG, which never exposes the bit — RESEARCH Pitfall 2).
pub const LVT_TSC_DEADLINE: u32 = 1 << 18;

/// DCR divide-by-1 encoding (Linux APIC_TDR_DIV_1 = 0xB; 9k's DivX1).
/// The divisor is a fixed participant in the periodic-mode runtime
/// calibration, not a frequency assumption.
pub const TIMER_DIV1: u32 = 0xB;

/// IA32_APIC_BASE — the mode MSR: bit 11 globally enables the unit
/// (firmware sets it on the BSP; ensured at the switch anyway), bit 10
/// (EXTD) selects x2APIC mode.
const MSR_IA32_APIC_BASE: u32 = 0x01B;
/// IA32_APIC_BASE global-enable bit (bit 11).
const APIC_BASE_ENABLE: u64 = 1 << 11;
/// IA32_APIC_BASE x2APIC-enable bit (bit 10, EXTD).  Once set, the
/// register file is reachable ONLY at MSRs 0x800+(offset>>4) — MMIO
/// access is architecturally unavailable in x2APIC mode, so every
/// access after the switch goes through the MSR arm of the accessors.
const APIC_BASE_EXTD: u64 = 1 << 10;
/// The x2APIC register map's base: the register at xAPIC MMIO offset X
/// lives at MSR 0x800+(X>>4) (SDM Vol 3A 10.12.2).  The ICR is the one
/// full 64-bit register there (destination in the upper half); this
/// driver does not emit IPIs yet.
const X2APIC_MSR_BASE: u32 = 0x800;

/// The architectural LAPIC MMIO base — a fixed-function fact of the q35
/// machine model, like COM1 for the console.
const LAPIC_BASE: usize = 0xFEE0_0000;
/// The register span actually occupied (the first KiB of the 4 KiB
/// page) — the io.rs offset-bounds discipline re-keyed.
const LAPIC_SPAN: usize = 0x400;
/// The boot identity window's coverage: l.S's bootidentitypt3 maps the
/// first 4 GiB PA==VA, and init() refuses loudly if the base ever fell
/// outside it.
const BOOT_IDENTITY_LIMIT: usize = 1 << 32;

/// The LAPIC's access mode, CPUID-gated at discovery — the registry's
/// analog of timer::timer_mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApicMode {
    /// Classic xAPIC: MMIO at the base address (the TCG qemu64 leg —
    /// qemu64 exposes neither x2APIC nor TSC-deadline).
    XApic,
    /// x2APIC: registers at MSRs 0x800+(offset>>4), entered by setting
    /// EXTD in IA32_APIC_BASE after the standard enable (the KVM
    /// -cpu host leg).
    X2Apic,
}

/// The pure mode gate: leaf-1 ECX bit 21 set -> X2Apic, clear -> XApic —
/// XApic whenever the bit is absent, regardless of anything else
/// (timer_mode's invariant, re-keyed for the registry; host-testable
/// without a CPU).
pub fn apic_mode(cpuid1_ecx: u32) -> ApicMode {
    if cpu::has_x2apic(cpuid1_ecx) {
        ApicMode::X2Apic
    } else {
        ApicMode::XApic
    }
}

/// The initialized LAPIC.  Plain data: every access is a bounds-checked
/// volatile u32 at base+offset (xAPIC) or an MSR access at
/// 0x800+(offset>>4) (x2APIC), so IRQ-context readers take no locks.
struct Lapic {
    base: usize,
    mode: ApicMode,
}

impl Lapic {
    fn new() -> Result<Lapic, &'static str> {
        if LAPIC_BASE + LAPIC_SPAN > BOOT_IDENTITY_LIMIT {
            return Err("lapic base outside the boot identity window");
        }
        // Every BSP boots in xAPIC mode (the MMIO map); init() upgrades
        // the accessors to the x2APIC MSR map behind the CPUID bit-21
        // gate, after the standard enable.
        Ok(Lapic { base: LAPIC_BASE, mode: ApicMode::XApic })
    }

    fn offset_addr(&self, offset: usize) -> usize {
        if offset >= LAPIC_SPAN {
            panic!("offset outside bounds");
        }
        self.base + offset
    }

    /// The x2APIC MSR address for an xAPIC register offset (the SDM
    /// 10.12.2 map: 0x800+(offset>>4)).
    fn x2apic_msr(offset: usize) -> u32 {
        X2APIC_MSR_BASE + (offset >> 4) as u32
    }

    fn read(&self, offset: usize) -> u32 {
        match self.mode {
            ApicMode::XApic => {
                let src = self.offset_addr(offset);
                // SAFETY: src lies inside the identity-mapped LAPIC
                // register span (bounds-checked by offset_addr); the
                // volatile read is the MMIO-volatility floor — reads
                // may have side effects (the CCR class), never elided.
                unsafe { read_volatile(crate::proc::kaddr(src as u64) as *const u32) }
            }
            ApicMode::X2Apic => {
                if offset >= LAPIC_SPAN {
                    panic!("offset outside bounds");
                }
                // SAFETY: x2APIC register MSRs (0x800+(offset>>4)) are
                // architecturally valid once EXTD is set (init switched
                // the unit before publishing it); non-ICR registers
                // read their 32-bit value zero-extended.
                unsafe { cpu::_rdmsr(Self::x2apic_msr(offset)) as u32 }
            }
        }
    }

    fn write(&self, offset: usize, val: u32) {
        match self.mode {
            ApicMode::XApic => {
                let dst = self.offset_addr(offset);
                // SAFETY: dst lies inside the identity-mapped LAPIC
                // register span (bounds-checked by offset_addr); the
                // volatile write is the MMIO-volatility floor.
                unsafe { write_volatile(crate::proc::kaddr(dst as u64) as *mut u32, val) }
            }
            ApicMode::X2Apic => {
                if offset >= LAPIC_SPAN {
                    panic!("offset outside bounds");
                }
                // SAFETY: x2APIC register MSRs are architecturally
                // valid once EXTD is set (init switched the unit before
                // publishing it); non-ICR registers take the 32-bit
                // payload zero-extended (upper bits are reserved zero).
                unsafe { cpu::wrmsr(Self::x2apic_msr(offset), u64::from(val)) }
            }
        }
    }
}

/// Interrupt-context access to the LAPIC.  Written once at init (inside
/// the arming window, before sti), read lock-free thereafter -- taking
/// a lock in IRQ context could deadlock against an interrupted holder.
static LAPIC: SyncUnsafeCell<Option<Lapic>> = SyncUnsafeCell::new(None);

/// Discover and software-enable the LAPIC, per the gic/ tolerant idiom:
/// a failure prints and leaves the cell empty (eoi() then no-ops, so
/// the IRQ path degrades instead of panicking).
///
/// Sequence: TPR 0 and the SVR software-enable with the spurious vector
/// FIRST (the enable-first order is load-bearing: emulator APICs —
/// qemu's userspace model and KVM's in-kernel one — discard LVT writes
/// while the unit is software-disabled, observed live at 13-03; 9k
/// writes its LVTs after the SVR too), then the x2APIC switch when
/// CPUID.01H:ECX[21] advertises it (SDM 10.12.2 order: the unit enabled
/// — IA32_APIC_BASE bit 11, ensured in the same write — then EXTD,
/// bit 10), then the LVT programming — LINT0/LINT1 masked (D-04: there
/// is no PIC to receive their delivery, and none will ever be enabled)
/// and the error LVT set — then a drain EOI for anything left
/// in-service.  All of it runs while EFLAGS.IF=0 (the mb2 entry
/// contract; nothing raises it before the arming window's tail), so
/// no interrupt can observe a half-initialized unit.
pub fn init() {
    match Lapic::new() {
        Ok(mut lapic) => {
            // The discovery evidence both mode gates key off: what
            // CPUID leaf 1 handed the guest — x2APIC (bit 21, this
            // registry's mode) and TSC-deadline (bit 24, timer.rs's
            // mode).  Printed loud so every gate leg's log carries the
            // capability facts the mode selection ran on.
            let (_, _, ecx1, _) = cpu::cpuid(1, 0);
            println!(
                "lapic: cpuid leaf-1 ecx: x2apic={} tsc-deadline={}",
                cpu::has_x2apic(ecx1),
                cpu::has_tsc_deadline(ecx1)
            );
            let want = apic_mode(ecx1);
            lapic.write(TPR, 0);
            lapic.write(SVR, SVR_ENABLE | u32::from(SPURIOUS_VECTOR));
            if want == ApicMode::X2Apic {
                // Enter x2APIC mode.  From here the register file is
                // reachable ONLY at MSRs 0x800+(offset>>4) — every
                // access below rides the MSR arm of the mode-dispatched
                // accessors.
                // SAFETY: IA32_APIC_BASE is always valid; every bit
                // other than 10/11 is preserved verbatim from the read.
                unsafe {
                    let base = cpu::_rdmsr(MSR_IA32_APIC_BASE);
                    cpu::wrmsr(
                        MSR_IA32_APIC_BASE,
                        base | APIC_BASE_ENABLE | APIC_BASE_EXTD,
                    );
                }
                lapic.mode = want;
            }
            lapic.write(LVT_LINT0, LVT_MASKED);
            lapic.write(LVT_LINT1, LVT_MASKED);
            lapic.write(LVT_ERROR, u32::from(ERROR_VECTOR));
            // Drain any outstanding in-service state (apiconline's
            // post-enable EOI).
            lapic.write(EOI_REG, 0);
            let mode = lapic.mode;
            // SAFETY: the publish-once discipline — this write runs at
            // boot inside the arming window (IF=0), before any IRQ
            // reader can run; the cell is never written again.
            unsafe { *LAPIC.get() = Some(lapic) }
            match mode {
                ApicMode::X2Apic => {
                    println!("lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)")
                }
                ApicMode::XApic => {
                    println!("lapic: xapic {LAPIC_BASE:#x} live (lint0/1 masked, no pic)")
                }
            }
        }
        Err(msg) => println!("can't initialise lapic: {msg}"),
    }
}

/// Acknowledge the just-handled interrupt: write 0 to the EOI register.
/// Runs from IRQ context, lock-free.  Called strictly AFTER the device
/// work (the read-to-ack-before-EOI ordering Pitfall 5 pins for the
/// level-triggered lines that arrive in 13-05); an unpublished cell
/// no-ops (the tolerant degradation).
pub fn eoi() {
    with(|l| l.write(EOI_REG, 0));
}

/// Program the timer LVT: vector | mode/mask bits.  timer.rs owns the
/// CPUID-gated mode decision and the arming discipline; this is the
/// raw programming surface it drives.  Runs at boot (timer init) and
/// from the tick path — both lock-free per the publish-once contract.
pub fn timer_lvt(vector: u8, bits: u32) {
    with(|l| l.write(LVT_TIMER, u32::from(vector) | bits));
}

/// Write the timer's initial count (the periodic arm).
pub fn timer_init_count(count: u32) {
    with(|l| l.write(TIMER_ICR, count));
}

/// Set the timer's divide configuration.
pub fn timer_divide(div: u32) {
    with(|l| l.write(TIMER_DCR, div));
}

/// Read the timer's current count — the periodic-mode runtime
/// calibration's measurand (how far the countdown ran over a known TSC
/// window).
pub fn timer_current_count() -> u32 {
    with_or(0, |l| l.read(TIMER_CCR))
}

fn with(f: impl FnOnce(&Lapic)) {
    // SAFETY: LAPIC's write-once discipline — published at init inside
    // the arming window (IF=0), read lock-free thereafter.
    if let Some(l) = (unsafe { &*LAPIC.get() }).as_ref() {
        f(l);
    }
}

fn with_or<T>(default: T, f: impl FnOnce(&Lapic) -> T) -> T {
    // SAFETY: with's contract (lock-free read of the published cell).
    match (unsafe { &*LAPIC.get() }).as_ref() {
        Some(l) => f(l),
        None => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registry's mode gate, both ways (the 13-03 dispatch-3 fix's
    /// companion invariant): leaf-1 ECX bit 21 set -> X2Apic (the KVM
    /// -cpu host shape), clear -> XApic — regardless of any other
    /// feature bit, TSC-deadline's bit 24 included.  TCG's qemu64
    /// carries only SSE3|CX16 and keeps exercising the MMIO path.
    #[test]
    fn apic_mode_keys_off_cpuid_leaf1_ecx_bit21() {
        assert_eq!(apic_mode(1 << 21), ApicMode::X2Apic);
        assert_eq!(apic_mode((1 << 21) | (1 << 24)), ApicMode::X2Apic);
        assert_eq!(apic_mode((1 << 21) | (1 << 0) | (1 << 13)), ApicMode::X2Apic);
        // the qemu64 TCG shape verbatim: SSE3 (bit 0) | CX16 (bit 13)
        assert_eq!(apic_mode((1 << 0) | (1 << 13)), ApicMode::XApic);
        // deadline-capable but not x2APIC-capable: still the MMIO path
        assert_eq!(apic_mode(1 << 24), ApicMode::XApic);
        assert_eq!(apic_mode(0), ApicMode::XApic);
        // routed through the pure helper (cpu.rs)
        assert!(crate::cpu::has_x2apic(1 << 21));
        assert!(!crate::cpu::has_x2apic(1 << 24));
        assert!(!crate::cpu::has_x2apic(0));
    }

    /// The x2APIC register-map arithmetic: xAPIC MMIO offset X maps to
    /// MSR 0x800+(X>>4) — the LVT-timer 0x320 lands at 0x832, the very
    /// address the old constant mis-used as the TSC-deadline MSR (the
    /// #GP root cause: KVM gates 0x800-0xBFF on x2APIC mode).  The
    /// deadline itself lives at 0x6E0 in EITHER mode — see timer.rs.
    #[test]
    fn x2apic_register_map_arithmetic() {
        assert_eq!(Lapic::x2apic_msr(EOI_REG), 0x80B);
        assert_eq!(Lapic::x2apic_msr(TPR), 0x808);
        assert_eq!(Lapic::x2apic_msr(SVR), 0x80F);
        assert_eq!(Lapic::x2apic_msr(LVT_TIMER), 0x832);
        assert_eq!(Lapic::x2apic_msr(LVT_LINT0), 0x835);
        assert_eq!(Lapic::x2apic_msr(LVT_LINT1), 0x836);
        assert_eq!(Lapic::x2apic_msr(LVT_ERROR), 0x837);
        assert_eq!(Lapic::x2apic_msr(TIMER_ICR), 0x838);
        assert_eq!(Lapic::x2apic_msr(TIMER_CCR), 0x839);
        assert_eq!(Lapic::x2apic_msr(TIMER_DCR), 0x83E);
    }
}

