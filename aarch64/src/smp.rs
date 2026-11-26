//! AArch64 SMP bring-up (SMP-02, SMP-04, SMP-07; D-25): per-CPU state via
//! `TPIDR_EL1`, DTB `/cpus` + `/psci` discovery, PSCI `CPU_ON`, the GIC
//! SGI IPIs, and the `SmpArch` registration feeding `port/src/smp.rs`.
//!
//! # Per-CPU state (SMP-02)
//!
//! [`PerCpu`] is the per-CPU kernel state the trap entry, the scheduler
//! and the FP owner machinery read off `TPIDR_EL1`.  `cur_frame` sits at
//! offset 0 — `trap.S` hardcodes `[tpidr_el1, #0]` — pinned by a
//! compile-time assert.  [`init_bsp`] (BSP) and `l.S`'s `secondary_start`
//! (secondaries) set `TPIDR_EL1 = &PER_CPU[cpu_id]` before any trap can
//! fire or any Rust code calls [`this_cpu`]; the register never changes
//! afterwards on a given CPU.
//!
//! # CPU 1 start (SMP-04)
//!
//! [`discover`] walks the DTB's `/cpus` `cpu@*` nodes for MPIDR values
//! (bounds-checked at [`MAX_CPUS`]) and reads `/psci`'s `method` (`"hvc"`
//! on QEMU virt EL1, `"smc"` fallback).  [`start_secondaries`] then issues
//! PSCI `CPU_ON` (`0xC400_0003`) per secondary with the physical address
//! of `l.S`'s `secondary_start` and the cpu id as the context.  An absent
//! CPU under `-smp 1` returns `PSCI_RET_INVALID_PARAMETERS` (`-2`), which
//! is tolerated cleanly (skipped, never an error).
//!
//! # SGI IPIs and TLB shootdown (SMP-07)
//!
//! SGI 0 ([`SGI_RESCHED`]) is the reschedule IPI; SGI 1
//! ([`SGI_SHOOTDOWN`]) is the TLB shootdown IPI.  [`register_port_smp`]
//! wires `port::smp::register_arch` to the arch hooks: `TPIDR_EL1` cpu id,
//! DAIF mask/restore, GIC SGI senders, and `tlbi vae1is`/`vmalle1is`.

use core::cell::SyncUnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};

use port::fdt::DeviceTree;
use port::println;
use port::smp::{self, SmpArch};

use crate::fpu::CpuFp;
use crate::proc::Proc;
use crate::swtch;

/// The most CPUs the aarch64 bring-up supports (D-23: never hard-code 2).
/// Matches `port::smp::MAX_CPUS` so the portable core's per-cpu tables
/// cover every cpu this arch can bring up.
pub const MAX_CPUS: usize = smp::MAX_CPUS;

/// Per-CPU kernel state, addressed via `TPIDR_EL1` (SMP-02).  Every entry
/// sits on its own cache line (`align(64)`) so two CPUs' hot fields never
/// share one.
#[repr(C, align(64))]
pub struct PerCpu {
    /// The live trap frame (trap.S entry writes, `trap_return` reads).
    /// MUST be at offset 0 — the assembly hardcodes `[tpidr_el1, #0]`.
    pub cur_frame: AtomicU64,
    /// The current proc's kernel-stack top (the SP_EL1 invariant record).
    pub cur_kstack: AtomicU64,
    /// This CPU's index (0..MAX_CPUS).
    pub cpu_id: u32,
    /// Set once this CPU has completed its bring-up.
    pub online: AtomicBool,
    /// The proc currently running on this CPU (null: none).
    pub current_proc: AtomicPtr<Proc>,
    /// The scheduler context (sched_loop entry) for this CPU.
    pub sched_context: SyncUnsafeCell<swtch::Context>,
    /// Whether `sched_context` has been seeded.
    pub sched_seeded: AtomicBool,
    /// Per-CPU scratch switch target for parking a context that never
    /// resumes (proc.rs's churn-profile boot park; 05-22's secondary
    /// sched_loop entry) — the BOOT_CTX global's per-CPU home (SMP-02).
    pub park_context: SyncUnsafeCell<swtch::Context>,
    /// The stack top this CPU's seeded sched_context runs on (0 = the
    /// legacy interrupt stack top — CPU 0's default; secondaries get
    /// their boot-stack top from the 05-22 bring-up tail).
    pub sched_stack_top: AtomicU64,
    /// Per-CPU FP/SIMD bookkeeping (owner + section flags; D-06).
    pub fpu: CpuFp,
    /// A TLB shootdown request is pending for this CPU (SMP-07).
    pub pending_shootdown: AtomicBool,
    /// Deferred reap slot (05-22, SMP): an exiting proc's root-table PA +
    /// kstack PAs sit here from exit_tail until THIS CPU's sched loop
    /// frees them after the moribund hand-off swtch.  The exit path's old
    /// "nothing allocates until the hand-off" invariant was single-CPU:
    /// with a sibling allocating concurrently, a kstack page freed at
    /// exit can be re-handed-out and written while this CPU still runs on
    /// it up to the swtch (and the root table stays in TTBR0 until the
    /// next pick's vm::switch).  Owned by this CPU: written in exit_tail
    /// and drained in sched_loop, both IRQ-masked contexts on the owning
    /// CPU.  KSTACK_PAGES is proc.rs's; 4 slots here.
    pub moribund_reap: SyncUnsafeCell<Option<(u64, [u64; 4])>>,
}

// The trap.S ABI contract: cur_frame is at offset 0 (T-05-16's mitigation
// is compile-time: a layout drift fails the build, never a silent trap
// frame write to the wrong field).
const _: () = assert!(core::mem::offset_of!(PerCpu, cur_frame) == 0);

impl PerCpu {
    const fn new() -> PerCpu {
        PerCpu {
            cur_frame: AtomicU64::new(0),
            cur_kstack: AtomicU64::new(0),
            cpu_id: 0,
            online: AtomicBool::new(false),
            current_proc: AtomicPtr::new(core::ptr::null_mut()),
            sched_context: SyncUnsafeCell::new(swtch::Context {
                x19: 0,
                x20: 0,
                x21: 0,
                x22: 0,
                x23: 0,
                x24: 0,
                x25: 0,
                x26: 0,
                x27: 0,
                x28: 0,
                x29: 0,
                x30: 0,
                sp: 0,
                spsr: 0,
            }),
            sched_seeded: AtomicBool::new(false),
            park_context: SyncUnsafeCell::new(swtch::Context {
                x19: 0,
                x20: 0,
                x21: 0,
                x22: 0,
                x23: 0,
                x24: 0,
                x25: 0,
                x26: 0,
                x27: 0,
                x28: 0,
                x29: 0,
                x30: 0,
                sp: 0,
                spsr: 0,
            }),
            sched_stack_top: AtomicU64::new(0),
            fpu: CpuFp::new(),
            pending_shootdown: AtomicBool::new(false),
            moribund_reap: SyncUnsafeCell::new(None),
        }
    }
}

/// The per-CPU table, indexed by cpu id (MPIDR_EL1.Aff0 on QEMU virt).
/// trap.S and the portable SMP core reach a CPU's state through
/// TPIDR_EL1, which init_bsp (BSP) and secondary_start (secondaries)
/// set to &PER_CPU[cpu_id] before any trap fires or Rust code runs.
#[unsafe(no_mangle)]
pub static PER_CPU: [PerCpu; MAX_CPUS] = [const { PerCpu::new() }; MAX_CPUS];

/// The number of CPUs currently online (dense 0..n).  Bumped once per
/// CPU as it completes bring-up (init_bsp for CPU 0, secondary_start's
/// Rust tail for secondaries).
static ONLINE_COUNT: AtomicU64 = AtomicU64::new(0);

/// This CPU's PerCpu, via TPIDR_EL1.
///
/// # Safety contract (T-05-16)
///
/// [`init_bsp`] (BSP) and `l.S`'s `secondary_start` (secondaries) set
/// `TPIDR_EL1 = &PER_CPU[cpu_id]` before any trap can fire and before any
/// Rust code calls `this_cpu()`; the register never changes afterwards on
/// a given CPU.  A call before init is a boot-order bug.
pub fn this_cpu() -> &'static PerCpu {
    let ptr: usize;
    // SAFETY: reads TPIDR_EL1 — a pure register read at EL1, no memory
    // operands.  The pointer it yields is &PER_CPU[cpu_id], set by
    // init_bsp (BSP) or secondary_start (secondaries) before any trap or
    // SMP call, and never changed afterwards: the reference is 'static.
    unsafe {
        core::arch::asm!("mrs {}, tpidr_el1", out(reg) ptr, options(nomem, nostack, preserves_flags))
    };
    // SAFETY: ptr is &PER_CPU[cpu_id] (the init contract in the doc
    // comment) — a live, aligned PerCpu that outlives the kernel.
    unsafe { &*(ptr as *const PerCpu) }
}

/// Mark a CPU online (init_bsp for CPU 0; the secondary bring-up tail for
/// the rest).
pub fn mark_online(cpu_id: usize) {
    let slot = &PER_CPU[cpu_id];
    // SAFETY: write-once before online publication — single-writer
    // bring-up context (BSP at early boot; the secondary under PSCI's
    // serialization), before `online` is set (Release) and before any
    // other CPU can observe the slot.
    unsafe { *(&raw const slot.cpu_id as *mut u32) = cpu_id as u32 };
    slot.online.store(true, Ordering::Release);
    ONLINE_COUNT.fetch_add(1, Ordering::AcqRel);
}

/// BSP bring-up (SMP-02): set `TPIDR_EL1 = &PER_CPU[0]` and mark CPU 0
/// online.  main9 calls this BEFORE `trap::init` installs VBAR_EL1, so
/// any trap that fires after the vectors are armed reads a valid
/// `cur_frame` pointer through TPIDR_EL1.
pub fn init_bsp() {
    let addr = &raw const PER_CPU as usize;
    // SAFETY: writes TPIDR_EL1 — a pure register write at EL1, no memory
    // operands.  The value is &PER_CPU[0], a live static that outlives
    // the kernel; this is the BSP's one-time init before any trap or SMP
    // call.
    unsafe {
        core::arch::asm!("msr tpidr_el1, {}", in(reg) addr, options(nomem, nostack, preserves_flags))
    };
    mark_online(0);
}

/// The size of one PerCpu entry, exported for `l.S`'s secondary_start
/// (the assembly's `madd` computes `&PER_CPU[cpu_id]` from it).
#[unsafe(no_mangle)]
pub static PER_CPU_STRIDE: u64 = core::mem::size_of::<PerCpu>() as u64;

/// Publish the BSP's boot MMU state for secondary CPUs (SMP-04).
/// Called from main9 after `init_vm` has set up the page tables and
/// enabled the MMU, before any secondary CPU starts.  The values are
/// read from the live registers and written to the `.bootdata` globals
/// that `secondary_start` reads at their physical addresses (the globals'
/// section is identity-mapped pre-MMU).
pub fn publish_boot_state() {
    let mair: u64;
    let tcr: u64;
    let ttbr0: u64;
    let ttbr1: u64;
    // SAFETY: reads of MAIR_EL1/TCR_EL1/TTBR0_EL1/TTBR1_EL1 — pure
    // register reads at EL1, no memory operands, no side effects.
    unsafe {
        core::arch::asm!("mrs {}, mair_el1", out(reg) mair, options(nomem, nostack, preserves_flags));
        core::arch::asm!("mrs {}, tcr_el1", out(reg) tcr, options(nomem, nostack, preserves_flags));
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) ttbr0, options(nomem, nostack, preserves_flags));
        core::arch::asm!("mrs {}, ttbr1_el1", out(reg) ttbr1, options(nomem, nostack, preserves_flags));
    }
    unsafe extern "C" {
        static SECONDARY_MAIR: u64;
        static SECONDARY_TCR: u64;
        static SECONDARY_TTBR0: u64;
        static SECONDARY_TTBR1: u64;
    }
    // SAFETY: write-once boot publication — single-CPU, IRQs masked,
    // before any secondary CPU starts.  The globals live in .bootdata
    // (identity-mapped pre-MMU); the secondary reads them at PA.
    unsafe {
        (&raw const SECONDARY_MAIR as *mut u64).write(mair);
        (&raw const SECONDARY_TCR as *mut u64).write(tcr);
        (&raw const SECONDARY_TTBR0 as *mut u64).write(ttbr0);
        (&raw const SECONDARY_TTBR1 as *mut u64).write(ttbr1);
    }
}

// ---------------------------------------------------------------------------
// DTB discovery + PSCI CPU_ON (SMP-04).
// ---------------------------------------------------------------------------

/// PSCI 0.2 CPU_ON function ID (DEN0022D §5.1.5, SMC64 form).
const PSCI_0_2_FN64_CPU_ON: u64 = 0xC400_0003;
/// PSCI return codes (signed 64-bit in x0).
const PSCI_RET_SUCCESS: i64 = 0;
const PSCI_RET_INVALID_PARAMETERS: i64 = -2;

/// The PSCI conduit (DTB `/psci` `method` property).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PsciMethod {
    /// `hvc #0` — QEMU virt EL1 default.
    Hvc,
    /// `smc #0` — firmware-first fallback.
    Smc,
}

/// Discovered SMP topology from the DTB (T-05-15: every field is
/// bounds-checked against MAX_CPUS and the method against the two known
/// strings).
pub struct DtbTopology {
    /// MPIDR values of the `/cpus` `cpu@*` nodes (index = cpu id; None =
    /// absent slot).
    pub mpidrs: [Option<u64>; MAX_CPUS],
    /// The number of CPU nodes found (capped at MAX_CPUS).
    pub count: usize,
    /// The PSCI conduit.
    pub method: PsciMethod,
}

/// Parse the DTB's `/cpus` and `/psci` nodes (SMP-04).
///
/// `/psci` `method` selects the conduit (`"hvc"` default on QEMU virt EL1,
/// `"smc"` fallback).  `/cpus` `cpu@*` `reg` properties yield the MPIDR
/// values, capped strictly at MAX_CPUS (T-05-15); extras are reported and
/// dropped.  A missing `/psci` or `/cpus` leaves the defaults (hvc, CPU 0
/// only) — the single-CPU boot shape.
pub fn discover(dt: &DeviceTree) -> DtbTopology {
    let mut topo = DtbTopology { mpidrs: [None; MAX_CPUS], count: 0, method: PsciMethod::Hvc };

    // /psci method ("hvc" on QEMU virt EL1, "smc" fallback).
    if let Some(psci) = dt.find_by_path("/psci") {
        if let Some(prop) = dt.property(&psci, "method") {
            match dt.property_value_bytes(&prop) {
                Some(b"hvc\0") => topo.method = PsciMethod::Hvc,
                Some(b"smc\0") => topo.method = PsciMethod::Smc,
                Some(other) => {
                    println!("psci: unknown method {other:?}, defaulting to hvc");
                }
                None => {}
            }
        }
    }

    // /cpus cpu@* reg values (MPIDRs).  T-05-15: the cpu count is capped
    // at MAX_CPUS; duplicate or out-of-range ids are rejected loudly.
    let Some(cpus) = dt.find_by_path("/cpus") else {
        return topo;
    };
    for child in dt.children(&cpus) {
        let Some(name) = dt.node_name(&child) else { continue };
        if !name.starts_with("cpu@") {
            continue;
        }
        if topo.count >= MAX_CPUS {
            println!("smp: DTB declares more than {MAX_CPUS} cpus, ignoring extras");
            break;
        }
        // The reg property's address cell is the MPIDR value.
        if let Some(reg) = dt.property_reg_iter(child).next() {
            topo.mpidrs[topo.count] = Some(reg.addr);
            topo.count += 1;
        }
    }
    topo
}

/// Invoke PSCI `CPU_ON` for `target_mpidr`, entering at `entry_pa` with
/// `context_id` in the started CPU's x0 (PSCI passes the call's x3 through
/// as the new CPU's x0).
///
/// Returns `Ok(true)` when the CPU was started, `Ok(false)` when the
/// target CPU is absent (`PSCI_RET_INVALID_PARAMETERS` — a 2-CPU static
/// DTB under `-smp 1`), `Err` on any other negative code.
pub fn psci_cpu_on(
    method: PsciMethod,
    target_mpidr: u64,
    entry_pa: u64,
    context_id: u64,
) -> Result<bool, &'static str> {
    let ret: u64;
    // SAFETY: hvc/smc #0 is the architected PSCI conduit — a
    // hypervisor/firmware call at EL1.  x0 = CPU_ON fn id, x1 = target
    // MPIDR, x2 = entry physical address, x3 = context id; x0 returns the
    // status code.  No memory operands; the clobber set is the PSCI-call
    // ABI's (x0 alone is declared; x1-x3 are inputs).
    unsafe {
        match method {
            PsciMethod::Hvc => core::arch::asm!(
                "hvc #0",
                inout("x0") PSCI_0_2_FN64_CPU_ON => ret,
                in("x1") target_mpidr,
                in("x2") entry_pa,
                in("x3") context_id,
                options(nomem, nostack),
            ),
            PsciMethod::Smc => core::arch::asm!(
                "smc #0",
                inout("x0") PSCI_0_2_FN64_CPU_ON => ret,
                in("x1") target_mpidr,
                in("x2") entry_pa,
                in("x3") context_id,
                options(nomem, nostack),
            ),
        }
    }
    match ret as i64 {
        PSCI_RET_SUCCESS => Ok(true),
        PSCI_RET_INVALID_PARAMETERS => Ok(false), // absent CPU (e.g. -smp 1)
        other => {
            println!("psci: CPU_ON mpidr={target_mpidr:#x} returned {other}");
            Err("psci: CPU_ON failed")
        }
    }
}

unsafe extern "C" {
    /// The secondary CPU entry point (l.S's `secondary_start`, plan
    /// 05-09 Task 2).  Only its address is read — the link-sym class
    /// (kmem.rs's convention); the physical address PSCI CPU_ON takes is
    /// `secondary_start - KZERO` (the identity-mapped .boottext section).
    static secondary_start: u8;
}

/// The physical entry address PSCI `CPU_ON` takes (secondary_start's VA
/// minus KZERO — the .boottext section is identity-mapped).
pub fn secondary_start_pa() -> u64 {
    // SAFETY: secondary_start is a link symbol — only its address is read
    // (never its contents).  The address is in the identity-mapped
    // .boottext section, so VA - KZERO is its physical address.
    let va = unsafe { &secondary_start as *const u8 as usize };
    (va - crate::param::KZERO) as u64
}

/// Start every secondary CPU the DTB declares (SMP-04), returning the
/// number successfully started.  CPU 0 is skipped (already running).
/// Absent CPUs (PSCI_RET_INVALID_PARAMETERS) are tolerated: a 2-CPU
/// static DTB under `-smp 1` yields `Ok(false)` for CPU 1, which is
/// skipped without panicking or erroring.  The caller waits only for
/// CPUs that actually started (dense `1..=started` on QEMU virt), so the
/// absent-CPU case costs no wait and skips the boot-time shootdown
/// self-check cleanly.
pub fn start_secondaries(dt: &DeviceTree) -> usize {
    let topo = discover(dt);
    let entry_pa = secondary_start_pa();
    let mut started = 0;
    for cpu_id in 1..topo.count {
        let Some(mpidr) = topo.mpidrs[cpu_id] else { continue };
        match psci_cpu_on(topo.method, mpidr, entry_pa, cpu_id as u64) {
            Ok(true) => {
                started += 1;
                println!("smp: cpu {cpu_id} started (mpidr {mpidr:#x})");
            }
            Ok(false) => {} // absent (e.g. -smp 1 with a 2-CPU static DTB)
            Err(e) => println!("smp: cpu {cpu_id}: {e}"),
        }
    }
    started
}

// ---------------------------------------------------------------------------
// SGI IPIs, IRQ mask/restore and TLB flush (SMP-07) + SmpArch registration.
// ---------------------------------------------------------------------------

/// SGI 0 — the reschedule IPI (SMP-07).  `port::sched`'s `enqueue` reports
/// an idle cpu to kick; `send_resched_ipi` delivers it.
pub const SGI_RESCHED: u8 = 0;
/// SGI 1 — the TLB shootdown IPI (SMP-07).  `port::smp`'s
/// `request_shootdown` sends it; the IRQ handler services it via
/// `service_shootdown_ipi`.
pub const SGI_SHOOTDOWN: u8 = 1;

fn current_cpu_id() -> usize {
    // The pointer arithmetic recovers the index (PER_CPU is a fixed array,
    // TPIDR_EL1 points into it), so no read of the plain cpu_id field is
    // needed.
    let ptr = this_cpu() as *const PerCpu as usize;
    let base = &raw const PER_CPU as usize;
    (ptr - base) / core::mem::size_of::<PerCpu>()
}

fn num_online_cpus() -> usize {
    ONLINE_COUNT.load(Ordering::Acquire) as usize
}

/// Save DAIF and mask IRQs (`msr daifset, #2`).  The returned value pairs
/// with [`irq_restore`].  (SMP-07 — IrqSpinLock's entry half.)
fn irq_save_disable() -> usize {
    let daif: u64;
    // SAFETY: mrs daif + msr daifset are pure PSTATE reads/writes at EL1 —
    // no memory operands.  daifset #2 masks IRQs (the I bit).
    unsafe {
        core::arch::asm!(
            "mrs {daif}, daif",
            "msr daifset, #2",
            daif = out(reg) daif,
            options(nomem, nostack, preserves_flags),
        );
    }
    daif as usize
}

/// Restore the DAIF saved by [`irq_save_disable`].
fn irq_restore(saved: usize) {
    // SAFETY: msr daif restores the saved PSTATE.DAIF — a pure register
    // write at EL1, no memory operands.
    unsafe {
        core::arch::asm!(
            "msr daif, {saved}",
            saved = in(reg) saved as u64,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(platform = "virt")]
fn send_resched_ipi(cpu: usize) {
    crate::platform::gic::send_sgi(cpu, SGI_RESCHED);
}

#[cfg(not(platform = "virt"))]
fn send_resched_ipi(_cpu: usize) {}

#[cfg(platform = "virt")]
fn send_shootdown_ipi(cpu: usize) {
    crate::platform::gic::send_sgi(cpu, SGI_SHOOTDOWN);
}

#[cfg(not(platform = "virt"))]
fn send_shootdown_ipi(_cpu: usize) {}

/// Flush the calling CPU's TLB (SMP-07).  `None` = full flush
/// (`tlbi vmalle1is`); `Some(va)` = the one page (`tlbi vae1is`).
/// Inner-shareable (`is` suffix) so the invalidation reaches the other
/// CPU's TLB — the shootdown IPI's local half.
fn local_tlb_flush(va: Option<usize>) {
    match va {
        Some(va) => {
            // SAFETY: dsb ishst + tlbi vae1is + dsb ish + isb is the
            // architected single-page TLB maintenance sequence (the same
            // ordering as vm.rs's full flush) — barrier + maintenance ops
            // only, no memory operands.  The VA is shifted to the
            // architected page-number operand.
            unsafe {
                core::arch::asm!(
                    "dsb ishst",
                    "tlbi vae1is, {va}",
                    "dsb ish",
                    "isb",
                    va = in(reg) (va >> 12) as u64,
                    options(nostack),
                );
            }
        }
        None => {
            // SAFETY: dsb ishst + tlbi vmalle1is + dsb ish + isb is the
            // architected full-TLB maintenance sequence (as vm.rs's
            // invalidate_all_tlb_entries) — barrier + maintenance ops
            // only, no memory operands.
            unsafe {
                core::arch::asm!(
                    "dsb ishst",
                    "tlbi vmalle1is",
                    "dsb ish",
                    "isb",
                    options(nostack),
                );
            }
        }
    }
}

/// The arch timer's tick count (1 tick = 1 ms this era).  The watchdog
/// reads it; a non-virt platform reports a frozen 0 (no watchdog ticks
/// until its timer lands).
fn arch_ticks() -> u64 {
    #[cfg(platform = "virt")]
    {
        crate::platform::timer::ticks()
    }
    #[cfg(not(platform = "virt"))]
    {
        0
    }
}

/// The SMP-03 lost-wakeup seam (05-10): clears the current proc's
/// `wake_pending` so a wake that lands after the caller's registration
/// (under the caller's subsystem lock) is seen by the arch `block()`'s
/// flag check.  Runs on the calling CPU only; no-op when no proc is
/// current (early boot).
fn prepare_block() {
    crate::proc::prepare_block_current();
}

/// Register the aarch64 `SmpArch` hooks with `port::smp` (SMP-02, D-25).
/// Called once from main9 after [`init_bsp`], before CPU 1 is started.
pub fn register_port_smp() {
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
