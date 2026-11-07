// LAPIC (classic xAPIC MMIO mode; x2APIC is deferred hardening) for
// qemu's q35 machine — the gic/ registry contract re-keyed (aarch64
// platform/virt/gic/mod.rs): init discovers and software-enables the
// unit and publishes it once, eoi() and the timer-LVT programming
// surface run lock-free from IRQ/boot context thereafter — a lock taken
// in IRQ context could deadlock against an interrupted holder.
//
// The base address is the architectural 0xFEE0_0000 fixed-function
// hardware fact of the machine model (an ACPI MADT type-0 entry would
// name the same value; 13-04's ACPI walk cross-checks it).  Access is
// map-free: l.S's bootidentitypt3 identity-maps the first 4 GiB PA==VA,
// so the aarch64 io.rs read_reg/write_reg volatile family re-keys over
// the identity window with offset bounds checks.

use core::cell::SyncUnsafeCell;
use core::ptr::{read_volatile, write_volatile};

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

/// The initialized LAPIC.  Plain data: every access is a bounds-checked
/// volatile u32 at base+offset, so IRQ-context readers take no locks.
struct Lapic {
    base: usize,
}

impl Lapic {
    fn new() -> Result<Lapic, &'static str> {
        if LAPIC_BASE + LAPIC_SPAN > BOOT_IDENTITY_LIMIT {
            return Err("lapic base outside the boot identity window");
        }
        Ok(Lapic { base: LAPIC_BASE })
    }

    fn offset_addr(&self, offset: usize) -> usize {
        if offset >= LAPIC_SPAN {
            panic!("offset outside bounds");
        }
        self.base + offset
    }

    fn read(&self, offset: usize) -> u32 {
        let src = self.offset_addr(offset);
        // SAFETY: src lies inside the identity-mapped LAPIC register
        // span (bounds-checked by offset_addr); the volatile read is
        // the MMIO-volatility floor — reads may have side effects (the
        // CCR class), never elided.
        unsafe { read_volatile(src as *const u32) }
    }

    fn write(&self, offset: usize, val: u32) {
        let dst = self.offset_addr(offset);
        // SAFETY: dst lies inside the identity-mapped LAPIC register
        // span (bounds-checked by offset_addr); the volatile write is
        // the MMIO-volatility floor.
        unsafe { write_volatile(dst as *mut u32, val) }
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
/// Sequence (the 9k apiconline order, xAPIC-classic): TPR 0, then the
/// SVR software-enable with the spurious vector FIRST, and only then
/// the LVT programming — LINT0/LINT1 masked (D-04: there is no PIC to
/// receive their delivery, and none will ever be enabled) and the
/// error LVT set — then a drain EOI for anything left in-service.
/// The enable-first order is load-bearing: emulator APICs (qemu's
/// userspace model and KVM's in-kernel one) discard LVT writes while
/// the unit is software-disabled (observed live at 13-03 — LVT
/// programming silently vanished), and 9k writes its LVTs after the
/// SVR too.  All of it runs while EFLAGS.IF=0 (the mb2 entry
/// contract; nothing raises it before the arming window's tail), so
/// no interrupt can observe a half-initialized unit.
pub fn init() {
    match Lapic::new() {
        Ok(lapic) => {
            lapic.write(TPR, 0);
            lapic.write(SVR, SVR_ENABLE | u32::from(SPURIOUS_VECTOR));
            lapic.write(LVT_LINT0, LVT_MASKED);
            lapic.write(LVT_LINT1, LVT_MASKED);
            lapic.write(LVT_ERROR, u32::from(ERROR_VECTOR));
            // Drain any outstanding in-service state (apiconline's
            // post-enable EOI).
            lapic.write(EOI_REG, 0);
            // SAFETY: the publish-once discipline — this write runs at
            // boot inside the arming window (IF=0), before any IRQ
            // reader can run; the cell is never written again.
            unsafe { *LAPIC.get() = Some(lapic) }
            println!("lapic: xapic {LAPIC_BASE:#x} live (lint0/1 masked, no pic)");
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

