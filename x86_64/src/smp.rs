//! x86_64 SMP bring-up (SMP-01, SMP-02, SMP-07; D-25): the
//! INIT-SIPI-SIPI AP startup from the ACPI MADT, per-CPU `Mach`
//! allocation on distinct kernel-half VAs, the LAPIC IPI senders, and
//! the `SmpArch` registration feeding `port/src/smp.rs`.
//!
//! # The SIPI trampoline (SMP-01)
//!
//! [`start_aps`] copies `ap_trampoline.S`'s position-independent
//! 16→32→64-bit entry to physical page [`AP_TRAMPOLINE_PA`] (SIPI vector
//! 0x08 — the AP starts executing at `vector << 12` in real mode) and
//! publishes the handoff mailbox at [`AP_MAILBOX_PA`] (inside the same
//! page): the BSP's `bootpml4` PA (which identity-maps the first 4 GiB,
//! so the trampoline executes at PA == VA through the mode switches), the
//! AP's 64 KiB boot stack top (its own Mach's `KStack`), the AP's Mach
//! VA, its cpu id, and `ap_main`'s high-half VA.  Per AP: `INIT` IPI →
//! 10 ms → `SIPI(0x08)` → 200 µs → one retry `SIPI` → poll `mach.online`
//! (SDM Vol 3A §8.4.4's MP initialization sequence).
//!
//! # Per-CPU Mach allocation (SMP-02)
//!
//! Each AP's Mach is a zeroed, 64 KiB-aligned heap block (the global
//! heap arena is a `.bss` static inside the shared kernel half, so the
//! block is mapped under every CR3 with NO PML4 cloning — the RFMEM
//! hazard 05-RESEARCH Pattern 3.5 names).  CPU 0's Mach stays at
//! `dat::KMACH` (l.S's mapping); every kernel Mach access on any CPU
//! reads `%gs:0` ([`crate::dat::mach_ptr`]) instead of hardcoding it.
//!
//! # IPIs (SMP-07)
//!
//! [`TLB_SHOOTDOWN_VECTOR`] (`0xF0`) and [`RESCHED_VECTOR`] (`0xF1`) are
//! delivered by `lapic::send_ipi` (dual xAPIC/x2APIC); the trap
//! dispatch (trap.rs) services them.

use core::alloc::Layout;
use core::sync::atomic::{AtomicU64, Ordering, fence};

use port::smp::{self, SmpArch};
#[cfg(not(test))]
use port::println;

use crate::cpu;
use crate::dat::{self, Mach, Stack};
use crate::platform::q35::{acpi, lapic};

// The SIPI AP bring-up trampoline (SMP-01) — assembled into .rodata (it
// is copied to PA 0x8000 for execution; the kernel's own copy is never
// jumped into).  Matching main.rs's l.S include discipline.
#[cfg(not(test))]
core::arch::global_asm!(include_str!("ap_trampoline.S"), options(att_syntax));

/// The TLB-shootdown IPI vector (SMP-07) — trap.rs's dispatch services
/// `port::smp::service_shootdown_ipi()` on it.
pub const TLB_SHOOTDOWN_VECTOR: u8 = 0xF0;
/// The reschedule IPI vector (SMP-07) — `port::sched`'s idle-cpu kick.
pub const RESCHED_VECTOR: u8 = 0xF1;

/// The SIPI trampoline's physical page (SIPI vector 0x08).  Well inside
/// q35's low RAM (0x0..0x9FC00, verified against live boot maps) and
/// below the Mach mebibyte (0x100000+) and kernel image — and the page
/// allocator marks everything below the kernel image used, so the page
/// is never handed out.
#[cfg(not(test))]
pub const AP_TRAMPOLINE_PA: u64 = 0x8000;
/// The BSP→AP mailbox inside the trampoline page.  The code occupies
/// 0x8000..0x8F00 (the compile-time assert in ap_trampoline.S and
/// install_trampoline's runtime assert both pin the non-overlap).
#[cfg(not(test))]
pub const AP_MAILBOX_PA: u64 = 0x8F00;

// Mailbox word offsets (u64 words).
#[cfg(not(test))]
const MB_PML4_PA: usize = 0x00; // BSP's bootpml4 PA
#[cfg(not(test))]
const MB_STACK_TOP: usize = 0x08; // the AP's boot stack top (a VA)
#[cfg(not(test))]
const MB_MACH_VA: usize = 0x10; // the AP's Mach VA
#[cfg(not(test))]
const MB_CPU_ID: usize = 0x18; // the AP's logical cpu id
#[cfg(not(test))]
const MB_ENTRY_VA: usize = 0x20; // ap_main's high-half VA

/// The SIPI vector naming the trampoline page (`0x8000 >> 12`).
#[cfg(not(test))]
const SIPI_VECTOR: u8 = (AP_TRAMPOLINE_PA >> 12) as u8;

/// The most CPUs this bring-up supports (D-23: never hard-code 2) — the
/// portable core's bound, so its per-cpu tables cover every cpu here.
pub const MAX_CPUS: usize = smp::MAX_CPUS;

/// The per-CPU Mach registry: slot `cpu_id` holds that CPU's Mach VA
/// (0 = not registered).  Slot 0 is the BSP's Mach at `dat::KMACH`,
/// registered by [`register_port_smp`]; each AP publishes its own slot
/// in `ap_main` ([`publish_ap_mach`]).  Read by the IPI senders (cpu id
/// → APIC id).
static AP_MACH_VAS: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

/// The number of CPUs currently online (SMP-01): the BSP registers as
/// CPU 0 at [`register_port_smp`]; each AP increments the count at the
/// end of its bring-up ([`mark_cpu_online`]), after publishing its Mach
/// slot and its online flag.  The run queues and the shootdown
/// coordinator size their walks from this count.
static ONLINE_CPUS: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// The SIPI trampoline install + INIT-SIPI-SIPI startup (SMP-01).
// ---------------------------------------------------------------------------

#[cfg(not(test))]
unsafe extern "C" {
    /// The SIPI trampoline's byte span (ap_trampoline.S) — only the
    /// ADDRESSES are read (the link-sym class, kmem.rs's convention):
    /// the bytes they bracket are copied to AP_TRAMPOLINE_PA for the
    /// real-mode AP entry.
    static ap_trampoline_start: u8;
    static ap_trampoline_end: u8;
}

/// Copy the trampoline into its physical page (SMP-01).  Idempotent —
/// the bytes are read-only constants.
#[cfg(not(test))]
fn install_trampoline() {
    // SAFETY: ap_trampoline_start/end are the link symbols bracketing
    // ap_trampoline.S's emitted bytes — only their addresses are read
    // here (the link-sym class).
    let (start, end) = unsafe {
        (
            &raw const ap_trampoline_start as usize,
            &raw const ap_trampoline_end as usize,
        )
    };
    let len = end - start;
    // The ap_trampoline.S .if assert is the compile-time half; this is
    // the runtime half — the code must never overlap the 0x8F00 mailbox.
    assert!(len < 0xF00, "smp: ap trampoline {len:#x} overlaps the 0x8F00 mailbox");
    let dst = crate::proc::phys_ptr_mut(AP_TRAMPOLINE_PA);
    // SAFETY: dst names physical page 0x8000 through the KZERO alias
    // (phys_ptr_mut's contract — in-window, writable, kernel-owned);
    // src names the trampoline's `len` linked bytes; the regions cannot
    // overlap (kernel image vs a low page).  x86 instruction fetch is
    // coherent with these WB data writes, so no cache maintenance is
    // needed before the AP executes the copy.
    unsafe { core::ptr::copy_nonoverlapping(start as *const u8, dst, len) };
}

/// Spin for `tsc_ticks` TSC cycles (the INIT/SIPI delays, SMP-01).
/// Invariant TSC under q35 makes the wrap-safe subtraction monotonic
/// for these millisecond-class waits.
#[cfg(not(test))]
fn busy_wait_tsc(tsc_ticks: u64) {
    let start = cpu::rdtsc();
    while cpu::rdtsc().wrapping_sub(start) < tsc_ticks {
        core::hint::spin_loop();
    }
}

/// Start one AP (SMP-01): allocate its Mach (zeroed, 64 KiB-aligned,
/// shared kernel half), publish the mailbox, and run INIT → 10 ms →
/// SIPI(0x08) → poll `mach.online` with a ~100 ms timeout.  There is
/// deliberately NO second SIPI: a retry delivered to an AP already in
/// protected mode triple-faults the machine (see the comment at the
/// send site).  The 10 ms/200 µs delays derive from the
/// BSP's published timer calibration (TICK_MS = 10 ms, so tsc_delta is
/// exactly 10 ms of TSC ticks) — the AP bring-up never re-calibrates.
#[cfg(not(test))]
fn start_one_ap(cpu_id: usize, apic_id: u32, bootpml4_pa: u64) -> Result<(), &'static str> {
    let Some(timer) = crate::platform::q35::timer::state() else {
        return Err("no BSP timer calibration (the delays need the TSC rate)");
    };
    let ten_ms = timer.tsc_delta.max(1);
    let online_timeout = timer.tsc_delta.saturating_mul(10).max(1_000_000);

    // The AP's Mach — a zeroed heap block in the shared kernel half.
    let layout = Layout::new::<Mach>();
    // SAFETY: layout names Mach's size/align (64 KiB); the global
    // allocator honors both (QuickFit's align_offset discipline).  On
    // success the block is zeroed (alloc_zeroed), page-aligned beyond
    // requirement, and never freed — a CPU's Mach is kernel-lifetime
    // state.  Mach's fields are all zero-valid by construction (atomics,
    // raw pointers, integer arrays), so the zeroed block is a valid Mach.
    let mach_va = unsafe { alloc::alloc::alloc_zeroed(layout) } as usize;
    if mach_va == 0 {
        return Err("ap Mach allocation failed");
    }
    // SAFETY: mach_va names the live, zeroed, exclusively owned Mach
    // allocated above (Mach is zero-valid by construction).
    let mach = unsafe { &mut *(mach_va as *mut Mach) };
    mach.init_ap_identity(cpu_id as u32, apic_id);
    let stack_top = mach.stack.top() as usize;

    // Publish the mailbox (the AP reads it from real/protected mode,
    // outside Rust's memory model — the volatile-write discipline of
    // the mb2 stash).
    let mb = crate::proc::phys_ptr_mut(AP_MAILBOX_PA) as *mut u64;
    // SAFETY: mb names the mailbox words inside physical page 0x8000
    // (phys_ptr_mut's in-window contract); the five u64 words written
    // are the documented mailbox layout, fully overwritten per start.
    unsafe {
        mb.add(MB_PML4_PA / 8).write_volatile(bootpml4_pa);
        mb.add(MB_STACK_TOP / 8).write_volatile(stack_top as u64);
        mb.add(MB_MACH_VA / 8).write_volatile(mach_va as u64);
        mb.add(MB_CPU_ID / 8).write_volatile(cpu_id as u64);
        mb.add(MB_ENTRY_VA / 8).write_volatile(crate::ap_main as *const () as usize as u64);
    }
    // The mailbox stores happen-before the INIT IPI (the AP's read side
    // is ordered by the INIT reset itself).
    fence(Ordering::Release);

    lapic::send_init(apic_id);
    busy_wait_tsc(ten_ms);
    lapic::send_sipi(apic_id, SIPI_VECTOR);
    // NO second SIPI.  The SDM's retry exists for an AP that missed the
    // first SIPI while still in the INIT shadow, but a SIPI delivered to
    // an AP that has ALREADY started is fatal: it sets CS:IP back to the
    // real-mode vector without resetting CR0/CR4, so the trampoline's
    // first `mov ds, ax` (selector 0) faults in protected mode and
    // triple-faults the machine.  Measured on KVM: the AP reaches
    // protected mode in well under the old 1 ms retry window (every
    // bring-up marker byte is a VM exit), so the retry reliably killed
    // it.  A genuinely missed SIPI now surfaces as the clean
    // "AP did not come online" timeout below instead of a machine reset.
    let start = cpu::rdtsc();
    while !mach.online.load(Ordering::Acquire) {
        if cpu::rdtsc().wrapping_sub(start) > online_timeout {
            return Err("AP did not come online (online flag timeout)");
        }
        core::hint::spin_loop();
    }
    // The AP published its own Mach slot (ap_main's publish_ap_mach,
    // BEFORE its online store — the Acquire pairing above makes it
    // visible here), so the IPI senders can name this CPU immediately.
    println!("smp: cpu {cpu_id} online (apic {apic_id:#x})");
    Ok(())
}

/// Start every non-BSP enabled CPU the MADT declares (SMP-01), up to
/// MAX_CPUS, returning the number started.  The BSP is identified by
/// APIC ID (the MADT's table order is NOT a cpu index), never started
/// twice.  All publish-once boot state must be complete before the call
/// (05-12's main.rs sequences it: the APs read the shared kernel half
/// the moment they run).
#[cfg(not(test))]
pub fn start_aps(madt: &acpi::Madt, bootpml4_pa: u64) -> usize {
    install_trampoline();
    let bsp_apic = lapic::current_apic_id();
    let mut started = 0;
    for i in 0..madt.ncpus {
        let cpu = madt.cpus[i];
        if cpu.apic_id == bsp_apic {
            continue; // the BSP itself
        }
        if started >= MAX_CPUS - 1 {
            println!("smp: more enabled CPUs than MAX_CPUS ({MAX_CPUS}); skipping the rest");
            break;
        }
        let cpu_id = started + 1;
        match start_one_ap(cpu_id, cpu.apic_id, bootpml4_pa) {
            Ok(()) => started += 1,
            Err(e) => println!("smp: cpu {cpu_id} (apic {:#x}): {e}", cpu.apic_id),
        }
    }
    if madt.dropped_cpus > 0 {
        println!("smp: {} enabled CPUs dropped by the MAX_CPUS cap", madt.dropped_cpus);
    }
    started
}

// ---------------------------------------------------------------------------
// The SmpArch hooks + registration (SMP-02, SMP-07; D-25).
// ---------------------------------------------------------------------------

/// The calling cpu's index via `%gs:0` (`Mach.cpu_id`, written once at
/// the CPU's bring-up before the online flag publishes it).
fn current_cpu_id() -> usize {
    dat::this_mach().cpu_id as usize
}

/// The number of cpus currently online: the registered-online count
/// (BSP = 1 at [`register_port_smp`]; each AP increments at its
/// bring-up tail).  Pre-registration callers (early boot) read the
/// single-CPU floor.
fn num_online_cpus() -> usize {
    let n = ONLINE_CPUS.load(Ordering::Acquire) as usize;
    if n == 0 { 1 } else { n.min(MAX_CPUS) }
}

/// Publish an AP's Mach into the registry (cpu id → Mach VA — the IPI
/// senders' routing).  Called BY the AP in `ap_main` BEFORE it
/// publishes its online flag (05-12): the BSP's `start_one_ap` observes
/// online with Acquire, so this Release store happens-before any
/// BSP-side read of the slot.
#[cfg(not(test))]
pub(crate) fn publish_ap_mach(cpu_id: usize, mach_va: usize) {
    AP_MACH_VAS[cpu_id].store(mach_va as u64, Ordering::Release);
}

/// Count this CPU online (05-12: `ap_main`'s bring-up tail; the BSP is
/// counted at [`register_port_smp`]).  Ordered AFTER the Mach-slot
/// publish and the online flag, so the count's readers (the run queues,
/// the shootdown coordinator) only ever walk registered CPUs.
#[cfg(not(test))]
pub(crate) fn mark_cpu_online() {
    ONLINE_CPUS.fetch_add(1, Ordering::AcqRel);
}

/// Save RFLAGS and mask IRQs (`pushfq; pop; cli`) — IrqSpinLock's entry
/// half (SMP-07).  The returned value pairs with [`irq_restore`].
fn irq_save_disable() -> usize {
    let flags: u64;
    // SAFETY: pushfq/popq move RFLAGS through the stack (so NOT
    // nostack); cli masks interrupts at CPL 0.  No flags claim:
    // cli itself modifies EFLAGS.IF.
    unsafe {
        core::arch::asm!(
            "pushfq",
            "popq {flags}",
            "cli",
            flags = out(reg) flags,
            options(att_syntax),
        );
    }
    flags as usize
}

/// Restore the IRQ state saved by [`irq_save_disable`]: `sti` only when
/// the saved IF (bit 9, 0x200) was set — the exact inverse of the
/// save/cli pair, never unmasking a context that entered masked.
fn irq_restore(saved: usize) {
    // SAFETY: test/sti at CPL 0, no memory operands; the saved flags
    // value is the only input.
    unsafe {
        core::arch::asm!(
            "testq ${IF_BIT}, {saved}",
            "jz 1f",
            "sti",
            "1:",
            IF_BIT = const 0x200u64,
            saved = in(reg) saved as u64,
            options(att_syntax, nostack),
        );
    }
}

/// A registered cpu's APIC ID (None: unknown slot — the IPI is then
/// skipped loudly rather than misrouted).
fn apic_id_of(cpu: usize) -> Option<u32> {
    let va = AP_MACH_VAS.get(cpu)?.load(Ordering::Acquire);
    if va == 0 {
        return None;
    }
    // SAFETY: a non-zero slot names a live, kernel-lifetime Mach
    // (register_port_smp / start_aps registered it; never removed).
    Some(unsafe { &*(va as *const Mach) }.apic_id)
}

/// A registered cpu's kstack-ownership record (05-24, the aarch64
/// `PER_CPU[c].cur_kstack` tripwire read re-keyed): the Mach's
/// `cur_kstack` slot — 0 when that cpu owns no proc kstack; None when
/// the slot was never registered.  proc.rs's pick_next debug tripwire
/// reads it for every OTHER cpu before starting a proc; debug builds
/// only (the tripwire compiles out in release, and so does this).
#[cfg(debug_assertions)]
pub(crate) fn cur_kstack_of(cpu: usize) -> Option<u64> {
    let va = AP_MACH_VAS.get(cpu)?.load(Ordering::Acquire);
    if va == 0 {
        return None;
    }
    // SAFETY: a non-zero slot names a live, kernel-lifetime Mach
    // (register_port_smp / publish_ap_mach registered it; never
    // removed).  The field is an AtomicU64, so the cross-CPU read is
    // race-free; the caller holds PROC_TABLE, pairing with the owning
    // CPU's locked write.
    Some(unsafe { &*(va as *const Mach) }.cur_kstack.load(Ordering::Acquire))
}

/// The reschedule IPI sender (SMP-07): LAPIC vector 0xF1 to the target
/// cpu's APIC ID.
fn send_resched_ipi(cpu: usize) {
    match apic_id_of(cpu) {
        Some(id) => lapic::send_ipi(id, RESCHED_VECTOR),
        None => {
            #[cfg(not(test))]
            println!("smp: resched ipi to unregistered cpu {cpu} skipped")
        }
    }
}

/// The TLB-shootdown IPI sender (SMP-07): LAPIC vector 0xF0.
fn send_shootdown_ipi(cpu: usize) {
    match apic_id_of(cpu) {
        Some(id) => lapic::send_ipi(id, TLB_SHOOTDOWN_VECTOR),
        None => {
            #[cfg(not(test))]
            println!("smp: shootdown ipi to unregistered cpu {cpu} skipped")
        }
    }
}

/// Flush the calling cpu's TLB (SMP-07): `Some(va)` invalidates the one
/// page (`invlpg`), `None` reloads CR3 (a full flush of non-global
/// entries — the boot tables carry no global pages).
fn local_tlb_flush(va: Option<usize>) {
    match va {
        Some(va) => {
            // SAFETY: invlpg invalidates the TLB entry for the page
            // containing the operand's effective address on the calling
            // CPU; the memory contents at va are neither read nor
            // written.  No `nomem`: a TLB flush must not let the
            // compiler move memory accesses across it.
            unsafe {
                core::arch::asm!(
                    "invlpg ({va})",
                    va = in(reg) va as u64,
                    options(att_syntax, nostack),
                );
            }
        }
        None => {
            // SAFETY: reading CR3 and writing it back at CPL 0 flushes
            // the calling CPU's non-global TLB entries; no memory
            // contents change.  No `nomem` (same ordering discipline).
            unsafe {
                core::arch::asm!(
                    "movq %cr3, {tmp}",
                    "movq {tmp}, %cr3",
                    tmp = out(reg) _,
                    options(att_syntax, nostack),
                );
            }
        }
    }
}

/// The arch timer's tick count (the watchdog's clock) — the q35 timer's
/// 100 Hz TICKS.
fn arch_ticks() -> u64 {
    crate::platform::q35::timer::ticks()
}

/// The SMP-03 lost-wakeup seam (05-12): clear the calling CPU's current
/// proc's `wake_pending` BEFORE its pid becomes wake-targetable (the
/// port-side register-under-subsystem-lock contract); the waker's
/// wake_locked re-publishes it under PROC_TABLE, and the blocker's
/// sched() consumes it under the same lock.
fn prepare_block() {
    crate::proc::prepare_block_current();
}

/// Register the x86_64 `SmpArch` hooks with `port::smp` (SMP-02, SMP-07,
/// D-25).  Called once from main.rs after the BSP's Mach is initialized
/// and before CPU 1 is started (05-12 wires the call).  Publishes the
/// BSP's own Mach slot (the IPI routing's CPU 0) and counts the BSP
/// online.
pub fn register_port_smp() {
    // Slot 0 is the BSP's Mach at dat::KMACH (l.S's mapping contract).
    AP_MACH_VAS[0].store(dat::KMACH as u64, Ordering::Release);
    // The BSP is online from the start (no handshake to poll).
    ONLINE_CPUS.store(1, Ordering::Release);
    static ARCH: SmpArch = SmpArch {
        current_cpu_id,
        num_online_cpus,
        irq_save_disable,
        irq_restore,
        send_resched_ipi,
        send_shootdown_ipi,
        local_tlb_flush,
        ticks: arch_ticks,
        prepare_block,
    };
    smp::register_arch(&ARCH);
}
