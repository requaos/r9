//! Portable SMP coordination for r9 (SMP-02, SMP-07, SMP-12; D-22, D-25,
//! D-26, D-31): the per-CPU architecture hook boundary, the documented
//! lock-rank table with debug/gate assertions, the per-CPU progress
//! watchdog, and the lock-free TLB shootdown coordinator.
//!
//! # The SmpArch boundary (SMP-02, D-25)
//!
//! Everything architecture-specific the portable SMP core needs is one
//! [`SmpArch`] vtable of plain function pointers, registered once during
//! single-CPU early boot by the arch crate (`TPIDR_EL1` per-CPU access on
//! aarch64 in plan 05-09, `%gs:0` `Mach` on x86_64 in plan 05-11) through
//! [`register_arch`].  Before registration every hook is a safe single-CPU
//! default: cpu 0, one online cpu, no-op IRQ mask/restore, no-op IPIs, no
//! TLB flush, a frozen tick counter, no-op [`prepare_block`].  Host tests
//! override cpu id / ticks / IRQ state / online mask per thread (the
//! `#[cfg(test)] thread_local!` pattern of `aarch64/src/fpu.rs:201-210`),
//! so the lock ranker, the watchdog and the shootdown coordinator run
//! multi-threaded under `cargo test -p port` with zero arch dependencies.
//!
//! # The lock-rank table (SMP-12, D-26)
//!
//! Every lock in the kernel carries a rank from the single documented
//! [`LockRank`] table; a cpu takes locks only in strictly increasing rank
//! order.  In `#[cfg(debug_assertions)]` builds (dev, test and the gate
//! profiles) [`rank_before_acquire`] panics naming BOTH the held and the
//! requested lock when the order is violated, and the per-CPU progress
//! watchdog ([`watchdog_check_all`]) names a stuck cpu, the lock it waits
//! on and that lock's owner cpu, so a deadlock is a named failure and not
//! a gate timeout.  Release builds compile the rank stack and the watchdog
//! out completely: every entry point is an `#[inline(always)]` no-op, the
//! per-cpu state does not exist, and no Cargo feature or custom cfg is
//! declared (no `unexpected_cfgs`).
//!
//! # The shootdown coordinator (SMP-07, Pitfall 1)
//!
//! r9 runs kernel syscalls with IRQs masked, so a cpu waiting for a remote
//! TLB-shootdown IPI ack while holding a lock deadlocks against a remote
//! cpu spinning on that same lock — the spinner cannot take the IPI.  The
//! coordinator is therefore lock-free on the service side:
//! [`request_shootdown`] publishes a generation + target VA and waits for
//! every other online cpu's ack, while [`poll_shootdown_if_pending`] —
//! called inside BOTH `MCSLock` and `IrqSpinLock` spin loops
//! (`port/src/mcslock.rs`) — services a pending request inline from any
//! IRQ-masked spin.  The request path spins on the claim with
//! `poll_shootdown_if_pending` in its loop, so two racing initiators
//! cannot wedge each other either, and the claim holds the target VA
//! stable until every online cpu has acked the generation it belongs to.

use core::hint;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::mcslock::OnceStatic;

/// The most cpus the portable SMP core covers (D-23: never hard-code 2).
/// Per-cpu state is indexed by `SmpArch::current_cpu_id`; a cpu id at or
/// beyond MAX_CPUS is a loud panic, never another cpu's slot.
pub const MAX_CPUS: usize = 64;

// ---------------------------------------------------------------------------
// SmpArch: the per-CPU architecture hook boundary (SMP-02, D-25).
// ---------------------------------------------------------------------------

/// The architecture hooks the portable SMP core needs, one vtable of plain
/// function pointers registered by the arch crate before CPU 1 starts
/// (D-25: each arch crate supplies only CPU start, per-CPU register access
/// and the IPI; everything else lives here in `port/`).
pub struct SmpArch {
    /// The calling cpu's index (0..MAX_CPUS).
    pub current_cpu_id: fn() -> usize,
    /// The number of cpus currently online (dense 0..n).
    pub num_online_cpus: fn() -> usize,
    /// Save the IRQ state and mask interrupts; the returned value is
    /// opaque and pairs with `irq_restore`.
    pub irq_save_disable: fn() -> usize,
    /// Restore the IRQ state saved by `irq_save_disable`.
    pub irq_restore: fn(usize),
    /// Send the reschedule IPI to a remote cpu.
    pub send_resched_ipi: fn(usize),
    /// Send the TLB-shootdown IPI to a remote cpu.
    pub send_shootdown_ipi: fn(usize),
    /// Flush the calling cpu's TLB: None = full flush, Some(va) = the one
    /// page.
    pub local_tlb_flush: fn(Option<usize>),
    /// A monotonic tick counter (the arch timer), for the watchdog.
    pub ticks: fn() -> u64,
    /// Prepare the calling cpu's current proc to block (the SMP-03
    /// lost-wakeup seam; the proc-side wake_pending work lands with the
    /// scheduler plans).
    pub prepare_block: fn(),
}

fn default_current_cpu_id() -> usize {
    0
}
fn default_num_online_cpus() -> usize {
    1
}
fn default_irq_save_disable() -> usize {
    0
}
fn default_irq_restore(_saved: usize) {}
fn default_send_ipi(_cpu: usize) {}
fn default_local_tlb_flush(_va: Option<usize>) {}
fn default_ticks() -> u64 {
    0
}
fn default_prepare_block() {}

/// The safe single-CPU defaults every hook falls back to before
/// [`register_arch`] runs (and always, in host tests: registration state
/// is observable there but never swapped under a running test — see
/// [`arch`]).
static DEFAULT_ARCH: SmpArch = SmpArch {
    current_cpu_id: default_current_cpu_id,
    num_online_cpus: default_num_online_cpus,
    irq_save_disable: default_irq_save_disable,
    irq_restore: default_irq_restore,
    send_resched_ipi: default_send_ipi,
    send_shootdown_ipi: default_send_ipi,
    local_tlb_flush: default_local_tlb_flush,
    ticks: default_ticks,
    prepare_block: default_prepare_block,
};

/// The registered arch hooks; published exactly once during single-CPU
/// early boot, before CPU 1 starts (D-25).  The publish-once cell lives in
/// the `mcslock` floor module: this module is `#[forbid(unsafe_code)]`,
/// and the cell's one pointer dereference carries its SAFETY there.
static ARCH: OnceStatic<SmpArch> = OnceStatic::new();

/// Register the arch hooks.  Boot discipline: called exactly once, before
/// the second cpu is brought up; a second call panics.
pub fn register_arch(arch: &'static SmpArch) {
    ARCH.publish(arch);
}

/// The live hooks: the registered arch set, or the safe single-CPU
/// defaults before registration.  Host tests always see the defaults (plus
/// their per-thread overrides), so one test's registration cannot leak
/// custom hooks into a parallel test.
fn arch() -> &'static SmpArch {
    #[cfg(test)]
    {
        &DEFAULT_ARCH
    }
    #[cfg(not(test))]
    {
        ARCH.get().unwrap_or(&DEFAULT_ARCH)
    }
}

/// The calling cpu's index (0 on the single-CPU default path).
pub fn current_cpu_id() -> usize {
    #[cfg(test)]
    if let Some(id) = test_hooks::cpu_id_override() {
        return id;
    }
    (arch().current_cpu_id)()
}

/// The number of cpus currently online (1 on the single-CPU default path).
pub fn num_online_cpus() -> usize {
    #[cfg(test)]
    {
        return test_hooks::online_mask().count_ones() as usize;
    }
    #[cfg(not(test))]
    {
        (arch().num_online_cpus)()
    }
}

/// Save the IRQ state and mask interrupts — `IrqSpinLock`'s entry half
/// (`port/src/mcslock.rs`).
pub fn irq_save_disable() -> usize {
    #[cfg(test)]
    {
        return test_hooks::irq_save_disable();
    }
    #[cfg(not(test))]
    {
        (arch().irq_save_disable)()
    }
}

/// Restore the IRQ state saved by [`irq_save_disable`].
pub fn irq_restore(saved: usize) {
    #[cfg(test)]
    {
        return test_hooks::irq_restore(saved);
    }
    #[cfg(not(test))]
    {
        (arch().irq_restore)(saved)
    }
}

/// Send the reschedule IPI to a remote cpu.
pub fn send_resched_ipi(cpu: usize) {
    (arch().send_resched_ipi)(cpu)
}

/// Send the TLB-shootdown IPI to a remote cpu.
pub fn send_shootdown_ipi(cpu: usize) {
    (arch().send_shootdown_ipi)(cpu)
}

/// Flush the calling cpu's TLB: None = full flush, Some(va) = one page.
pub fn local_tlb_flush(va: Option<usize>) {
    #[cfg(test)]
    test_hooks::record_flush(va);
    (arch().local_tlb_flush)(va)
}

/// A monotonic tick counter for the watchdog (frozen at 0 before the arch
/// registers a real one).
pub fn ticks() -> u64 {
    #[cfg(test)]
    {
        return test_hooks::ticks();
    }
    #[cfg(not(test))]
    {
        (arch().ticks)()
    }
}

/// Prepare the calling cpu's current proc to block — the SMP-03
/// lost-wakeup seam.  A no-op on the single-CPU default path.
pub fn prepare_block() {
    (arch().prepare_block)()
}

// ---------------------------------------------------------------------------
// The lock-rank table (SMP-12, D-26).
// ---------------------------------------------------------------------------

/// The single documented lock-rank table.  A cpu takes locks only in
/// strictly increasing rank order; debug/gate builds enforce the order
/// (a panic naming both locks), release builds carry nothing.
///
/// | Rank | Lock | Guards |
/// |------|------|--------|
/// | 10 | `DevTop` | device/subsystem tops (devmnt, devssl, devpipe, devsrv, devenv, devcap, devdup, devproc, fdgrp, pgrp, envgroup, rendez) |
/// | 12 | `DevMid` | the documented parent->child mid layer (devip TABLES) |
/// | 14 | `DevNetStack` | the devip STACK beneath its TABLES |
/// | 20 | `PageCache` | the TPHT image page cache / rmap (MEM-16) |
/// | 30 | `VmShared` | shared vm state (SHARED_PAGES, RFMEM_FAMILIES, SEM_WAITERS) |
/// | 40 | `FidShard` | a FidTable shard's AllocMeta |
/// | 50 | `ProcTable` | the process table and per-proc state |
/// | 60 | `RunQueue` | one per-cpu run queue; steals try-lock, never nest |
/// | 70 | `PageAlloc` | the Roaring frame pools |
/// | 80 | `HeapAlloc` | the global heap — above every lock that may allocate while held |
/// | 90 | `NetIrq` | IRQ-path net state (INGRESS, NET_WAITERS, PENDING_WAKES) |
/// | 100 | `Console` | the console/UART leaf lock — never acquires anything |
///
/// HeapAlloc at 80 — above PageAlloc and below only NetIrq/Console, which
/// never allocate — is what lets a Roaring container upgrade allocate from
/// the heap while PAGE_ALLOC (70) is held, while the heap itself never
/// reaches back into the page allocator (its arena is the static
/// GlobalHeap), so no 70 <-> 80 cycle can exist (05-RESEARCH §3.2).
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockRank {
    DevTop = 10,
    DevMid = 12,
    DevNetStack = 14,
    PageCache = 20,
    VmShared = 30,
    FidShard = 40,
    ProcTable = 50,
    RunQueue = 60,
    PageAlloc = 70,
    HeapAlloc = 80,
    NetIrq = 90,
    Console = 100,
}

// ---------------------------------------------------------------------------
// Rank stack + progress watchdog (SMP-12, D-26) — debug/gate builds only.
// ---------------------------------------------------------------------------

#[cfg(debug_assertions)]
mod rank_watch {
    use super::{LockRank, MAX_CPUS, current_cpu_id, ticks};
    use core::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize, Ordering};

    /// Depth of the per-cpu held-lock rank stack.  The deepest documented
    /// nesting (DEV_TOP -> ... -> HEAP_ALLOC chains) never approaches this;
    /// an overflow is a loud panic, not a wrap.
    pub const RANK_DEPTH: usize = 16;
    /// Max lock-name bytes recorded per stack slot / wait.  Kernel lock
    /// names are short static strings; longer names truncate (diagnostics
    /// only).
    pub const NAME_CAP: usize = 48;
    /// `waiting_since` sentinel: this cpu is not waiting on any lock.
    pub const NOT_WAITING: u64 = u64::MAX;
    /// `owner_cpu` sentinel on a lock / wait: no owner recorded.
    pub const NO_OWNER: u32 = u32::MAX;

    /// One held lock on a cpu's rank stack: its rank plus its name bytes
    /// (stored as bytes so the module stays `#[forbid(unsafe_code)]` — a
    /// `&str` is rebuilt with the safe `core::str::from_utf8`, never with
    /// raw-pointer reconstruction).
    struct RankSlot {
        rank: AtomicU16,
        name_len: AtomicUsize,
        name: [AtomicU8; NAME_CAP],
    }

    impl RankSlot {
        const fn new() -> RankSlot {
            RankSlot {
                rank: AtomicU16::new(0),
                name_len: AtomicUsize::new(0),
                name: [const { AtomicU8::new(0) }; NAME_CAP],
            }
        }

        fn write(&self, rank: LockRank, name: &'static str) {
            let bytes = name.as_bytes();
            let len = bytes.len().min(NAME_CAP);
            for (i, slot) in self.name.iter().enumerate() {
                slot.store(if i < len { bytes[i] } else { 0 }, Ordering::Relaxed);
            }
            self.name_len.store(len, Ordering::Relaxed);
            self.rank.store(rank as u16, Ordering::Release);
        }

        fn read_name<'a>(&self, buf: &'a mut [u8; NAME_CAP]) -> &'a str {
            let len = self.name_len.load(Ordering::Relaxed).min(NAME_CAP);
            for (i, b) in buf.iter_mut().enumerate() {
                *b = if i < len { self.name[i].load(Ordering::Relaxed) } else { 0 };
            }
            core::str::from_utf8(&buf[..len]).unwrap_or("<name>")
        }
    }

    /// One cpu's rank/watchdog state.  Touched only by the owning cpu
    /// except the watchdog's cross-cpu reads (kernel build); atomics make
    /// the shared reference sound.  Fixed size, zero heap (D-26).
    pub struct CpuDebug {
        /// Bumped by `watchdog_heartbeat` (the arch timer tick).  A cpu
        /// whose heartbeat moves while it waits is making progress.
        heartbeat: AtomicU64,
        /// The ticks() value when the current wait began; NOT_WAITING when
        /// the cpu is not waiting on a lock.
        waiting_since: AtomicU64,
        /// The heartbeat value when the current wait began.
        waiting_heartbeat: AtomicU64,
        /// The cpu holding the lock this cpu waits on (NO_OWNER: unknown).
        waiting_owner: AtomicU32,
        waiting_name_len: AtomicUsize,
        waiting_name: [AtomicU8; NAME_CAP],
        /// Live entries in `stack` (strictly increasing ranks).
        stack_len: AtomicUsize,
        stack: [RankSlot; RANK_DEPTH],
    }

    impl CpuDebug {
        const fn new() -> CpuDebug {
            CpuDebug {
                heartbeat: AtomicU64::new(0),
                waiting_since: AtomicU64::new(NOT_WAITING),
                waiting_heartbeat: AtomicU64::new(0),
                waiting_owner: AtomicU32::new(NO_OWNER),
                waiting_name_len: AtomicUsize::new(0),
                waiting_name: [const { AtomicU8::new(0) }; NAME_CAP],
                stack_len: AtomicUsize::new(0),
                stack: [const { RankSlot::new() }; RANK_DEPTH],
            }
        }

        /// The top-of-stack (highest) held rank, or None when no lock is
        /// held.
        fn top_rank(&self) -> Option<u16> {
            let len = self.stack_len.load(Ordering::Relaxed);
            if len == 0 { None } else { Some(self.stack[len - 1].rank.load(Ordering::Relaxed)) }
        }

        /// The top-of-stack held lock's name (call only when top_rank() is
        /// Some).
        fn top_name<'a>(&self, buf: &'a mut [u8; NAME_CAP]) -> &'a str {
            let len = self.stack_len.load(Ordering::Relaxed);
            self.stack[len - 1].read_name(buf)
        }

        fn push(&self, rank: LockRank, name: &'static str) {
            let len = self.stack_len.load(Ordering::Relaxed);
            assert!(
                len < RANK_DEPTH,
                "smp: lock rank stack overflow (depth {RANK_DEPTH}) on cpu {}",
                current_cpu_id()
            );
            self.stack[len].write(rank, name);
            self.stack_len.store(len + 1, Ordering::Release);
        }

        /// Remove `rank` from the held set.  Releases need not be LIFO
        /// (SMP-12: "releasing in LIFO or non-LIFO order clears held ranks
        /// cleanly"), so the entry is found by rank — the stack is strictly
        /// increasing, so a rank occurs at most once — and the tail shifts
        /// down.  Releasing a lock this cpu does not hold panics by name.
        fn remove(&self, rank: LockRank, name: &'static str) {
            let len = self.stack_len.load(Ordering::Relaxed);
            let target = rank as u16;
            let mut found = None;
            for i in (0..len).rev() {
                if self.stack[i].rank.load(Ordering::Relaxed) == target {
                    found = Some(i);
                    break;
                }
            }
            let Some(i) = found else {
                panic!(
                    "smp: cpu {} released lock \"{name}\" (rank {target}) it does not hold",
                    current_cpu_id()
                );
            };
            for j in i..len.saturating_sub(1) {
                let src_rank = self.stack[j + 1].rank.load(Ordering::Relaxed);
                let src_len = self.stack[j + 1].name_len.load(Ordering::Relaxed).min(NAME_CAP);
                let mut buf = [0u8; NAME_CAP];
                for (k, b) in buf.iter_mut().enumerate() {
                    *b = if k < src_len { self.stack[j + 1].name[k].load(Ordering::Relaxed) } else { 0 };
                }
                for (k, b) in buf.iter().enumerate() {
                    self.stack[j].name[k].store(*b, Ordering::Relaxed);
                }
                self.stack[j].name_len.store(src_len, Ordering::Relaxed);
                self.stack[j].rank.store(src_rank, Ordering::Relaxed);
            }
            self.stack_len.store(len - 1, Ordering::Release);
        }

        fn read_waiting_name<'a>(&self, buf: &'a mut [u8; NAME_CAP]) -> &'a str {
            let len = self.waiting_name_len.load(Ordering::Relaxed).min(NAME_CAP);
            for (i, b) in buf.iter_mut().enumerate() {
                *b = if i < len { self.waiting_name[i].load(Ordering::Relaxed) } else { 0 };
            }
            core::str::from_utf8(&buf[..len]).unwrap_or("<name>")
        }
    }

    /// The per-cpu state table (kernel builds), indexed by cpu id.
    #[cfg(not(test))]
    static CPUS: [CpuDebug; MAX_CPUS] = [const { CpuDebug::new() }; MAX_CPUS];

    #[cfg(not(test))]
    pub fn this_cpu_debug() -> &'static CpuDebug {
        let id = current_cpu_id();
        assert!(id < MAX_CPUS, "smp: cpu id {id} at or beyond MAX_CPUS ({MAX_CPUS})");
        &CPUS[id]
    }

    /// Host tests: every test thread is its own "cpu" (the fpu.rs
    /// `thread_local!` pattern), so parallel tests never share rank or
    /// watchdog state.
    #[cfg(test)]
    std::thread_local! {
        static CPU_DEBUG: &'static CpuDebug = alloc::boxed::Box::leak(alloc::boxed::Box::new(CpuDebug::new()));
    }

    #[cfg(test)]
    pub fn this_cpu_debug() -> &'static CpuDebug {
        CPU_DEBUG.with(|c| *c)
    }

    /// Record a lock acquisition on the calling cpu's rank stack.
    pub fn acquire(rank: LockRank, name: &'static str) {
        let cpu = this_cpu_debug();
        // 05-07 RED scaffolding: the stack is recorded; the strictly-
        // increasing-rank panic (D-26) lands in the GREEN commit.
        if let Some(top) = cpu.top_rank() {
            let mut buf = [0u8; NAME_CAP];
            let _held = cpu.top_name(&mut buf);
            let _violation = (rank as u16) <= top;
        }
        cpu.push(rank, name);
    }

    /// Record a lock release on the calling cpu's rank stack.
    pub fn release(rank: LockRank, name: &'static str) {
        this_cpu_debug().remove(rank, name);
    }

    /// The calling cpu made progress (the arch timer tick calls this).
    pub fn heartbeat() {
        this_cpu_debug().heartbeat.fetch_add(1, Ordering::Relaxed);
    }

    /// The calling cpu begins waiting on a lock (records the lock name and
    /// its current owner for the watchdog's named panic).
    pub fn begin_wait(name: &'static str, owner_cpu: u32) {
        let cpu = this_cpu_debug();
        let bytes = name.as_bytes();
        let len = bytes.len().min(NAME_CAP);
        for (i, slot) in cpu.waiting_name.iter().enumerate() {
            slot.store(if i < len { bytes[i] } else { 0 }, Ordering::Relaxed);
        }
        cpu.waiting_name_len.store(len, Ordering::Relaxed);
        cpu.waiting_owner.store(owner_cpu, Ordering::Relaxed);
        cpu.waiting_heartbeat.store(cpu.heartbeat.load(Ordering::Relaxed), Ordering::Relaxed);
        // Published last: a cross-cpu reader that sees the wait sees its
        // fields.
        cpu.waiting_since.store(ticks(), Ordering::Release);
    }

    /// The calling cpu's wait ended (it acquired the lock).
    pub fn end_wait() {
        this_cpu_debug().waiting_since.store(NOT_WAITING, Ordering::Release);
    }

    fn check_one(cpu: &CpuDebug, id: usize, threshold_ticks: u64) {
        let since = cpu.waiting_since.load(Ordering::Acquire);
        if since == NOT_WAITING {
            return; // an idle cpu never trips the watchdog
        }
        if cpu.heartbeat.load(Ordering::Acquire) != cpu.waiting_heartbeat.load(Ordering::Relaxed)
        {
            return; // active: the cpu made progress during this wait
        }
        let elapsed = ticks().wrapping_sub(since);
        if elapsed <= threshold_ticks {
            return; // a brief wait is contention, not a deadlock
        }
        // 05-07 RED scaffolding: the traversal and the stuck/idle/active
        // classification run; the named panic (D-26) lands in GREEN.
        let mut buf = [0u8; NAME_CAP];
        let _name = cpu.read_waiting_name(&mut buf);
        let _owner = cpu.waiting_owner.load(Ordering::Relaxed);
        let _ = id;
    }

    /// Check every online cpu for a stuck wait.  The kernel build walks
    /// the dense online range; host tests check the calling thread's own
    /// simulated cpu (the test plays every cpu role on its own thread).
    pub fn check_all(threshold_ticks: u64) {
        #[cfg(not(test))]
        {
            let n = super::num_online_cpus().min(MAX_CPUS);
            for id in 0..n {
                check_one(&CPUS[id], id, threshold_ticks);
            }
        }
        #[cfg(test)]
        {
            check_one(this_cpu_debug(), current_cpu_id(), threshold_ticks);
        }
    }

    /// The calling thread's held-lock count (host tests only).
    #[cfg(test)]
    pub fn held_lock_count() -> usize {
        this_cpu_debug().stack_len.load(Ordering::Relaxed)
    }
}

/// A lock acquisition of `rank`/`name` begins on this cpu: panics naming
/// BOTH the held and the requested lock when the order is violated
/// (`rank <= ` the top held rank), then records the hold.
#[cfg(debug_assertions)]
pub fn rank_before_acquire(rank: LockRank, name: &'static str) {
    rank_watch::acquire(rank, name);
}
/// Release builds carry no rank stack (D-26): a zero-cost no-op.
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn rank_before_acquire(_rank: LockRank, _name: &'static str) {}

/// A lock release of `rank`/`name` on this cpu: removes it from the held
/// set (LIFO or not); releasing a lock not held panics by name.
#[cfg(debug_assertions)]
pub fn rank_after_release(rank: LockRank, name: &'static str) {
    rank_watch::release(rank, name);
}
/// Release builds carry no rank stack (D-26): a zero-cost no-op.
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn rank_after_release(_rank: LockRank, _name: &'static str) {}

/// The calling cpu made progress (the arch timer tick calls this).
#[cfg(debug_assertions)]
pub fn watchdog_heartbeat() {
    rank_watch::heartbeat();
}
/// Release builds carry no watchdog (D-26): a zero-cost no-op.
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn watchdog_heartbeat() {}

/// The calling cpu begins waiting on `name` (held by `owner_cpu`).
#[cfg(debug_assertions)]
pub fn watchdog_begin_wait(name: &'static str, owner_cpu: u32) {
    rank_watch::begin_wait(name, owner_cpu);
}
/// Release builds carry no watchdog (D-26): a zero-cost no-op.
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn watchdog_begin_wait(_name: &'static str, _owner_cpu: u32) {}

/// The calling cpu's wait ended.
#[cfg(debug_assertions)]
pub fn watchdog_end_wait() {
    rank_watch::end_wait();
}
/// Release builds carry no watchdog (D-26): a zero-cost no-op.
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn watchdog_end_wait() {}

/// Check every online cpu for a stuck wait, panicking with
/// `WATCHDOG: cpu <id> stuck for <ticks> ticks waiting on lock "<name>"
/// (held by cpu <owner>)` for the first cpu past `threshold_ticks`.
/// Idle cpus (no waiting lock) and active cpus (heartbeat moved during
/// the wait) never trip it.
#[cfg(debug_assertions)]
pub fn watchdog_check_all(threshold_ticks: u64) {
    rank_watch::check_all(threshold_ticks);
}
/// Release builds carry no watchdog (D-26): a zero-cost no-op.
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn watchdog_check_all(_threshold_ticks: u64) {}

/// The `owner_cpu` sentinel shared with the lock metadata in
/// `port/src/mcslock.rs`.
#[cfg(debug_assertions)]
pub use rank_watch::NO_OWNER;

/// The calling thread's held-lock count (host-test inspection only).
#[cfg(test)]
pub fn test_held_lock_count() -> usize {
    #[cfg(debug_assertions)]
    {
        rank_watch::held_lock_count()
    }
    #[cfg(not(debug_assertions))]
    {
        0
    }
}

// ---------------------------------------------------------------------------
// The lock-free TLB shootdown coordinator (SMP-07, Pitfall 1).
// ---------------------------------------------------------------------------

/// Serializes request VA publication (initiator side only; the service
/// path is lock-free).  A contender spins with
/// [`poll_shootdown_if_pending`] in its loop, so an IRQ-masked waiter
/// still services the in-flight request.
static SHOOTDOWN_CLAIM: AtomicBool = AtomicBool::new(false);
/// The current shootdown generation; 0 = no request has ever been issued.
static SHOOTDOWN_SEQ: AtomicU64 = AtomicU64::new(0);
/// The current generation's target: 0 = full flush, non-zero = one page VA.
static SHOOTDOWN_VA: AtomicUsize = AtomicUsize::new(0);
/// Per-cpu ack generations.
static SHOOTDOWN_ACK: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

/// Service a pending shootdown request aimed at this cpu, if any: flush
/// the target locally and ack the generation.  Lock-free and wait-free —
/// this is the call inside both `MCSLock` and `IrqSpinLock` spin loops
/// that keeps an IRQ-masked spinner from deadlocking a remote initiator
/// (SMP-07).
pub fn poll_shootdown_if_pending() {
    let seq = SHOOTDOWN_SEQ.load(Ordering::Acquire);
    if seq == 0 {
        return;
    }
    let cpu = current_cpu_id();
    if cpu >= MAX_CPUS {
        // A cpu id outside the table is an arch bring-up bug; skipping the
        // service leaves the initiator to the watchdog rather than
        // panicking inside a spin loop.
        return;
    }
    if SHOOTDOWN_ACK[cpu].load(Ordering::Relaxed) < seq {
        let va = SHOOTDOWN_VA.load(Ordering::Relaxed);
        // The VA write happens-before the seq bump (Release), and the
        // claim holds the VA stable until every online cpu has acked this
        // seq, so a poller always reads the VA belonging to the seq it
        // acks.
        local_tlb_flush(if va == 0 { None } else { Some(va) });
        SHOOTDOWN_ACK[cpu].store(seq, Ordering::Release);
    }
}

/// The IPI handler entry: the arch IRQ path calls this on the shootdown
/// vector (GIC SGI 1 on aarch64, LAPIC vector 0xF0 on x86_64; plans
/// 05-09/05-11 wire it).
pub fn service_shootdown_ipi() {
    poll_shootdown_if_pending();
}

/// Request a TLB shootdown of `target_va` (None = full flush) on every
/// other online cpu, returning the completed generation once all of them
/// have acked.  The wait loop polls its own pending requests, so an
/// initiator holding IRQ-masked locks can never wedge against a remote
/// spinner (Pitfall 1).  The caller flushes its own TLB; the initiator's
/// ack slot is pre-acked so its next poll does not reflush.
pub fn request_shootdown(target_va: Option<usize>) -> u64 {
    while SHOOTDOWN_CLAIM.swap(true, Ordering::AcqRel) {
        poll_shootdown_if_pending();
        hint::spin_loop();
    }
    SHOOTDOWN_VA.store(target_va.unwrap_or(0), Ordering::Relaxed);
    let seq = SHOOTDOWN_SEQ.load(Ordering::Relaxed) + 1;
    SHOOTDOWN_SEQ.store(seq, Ordering::Release);
    let my = current_cpu_id();
    if my < MAX_CPUS {
        SHOOTDOWN_ACK[my].store(seq, Ordering::Release);
    }
    each_online_remote(my, send_shootdown_ipi);
    loop {
        poll_shootdown_if_pending();
        let mut done = true;
        each_online_remote(my, |cpu| {
            if SHOOTDOWN_ACK[cpu].load(Ordering::Acquire) < seq {
                done = false;
            }
        });
        if done {
            break;
        }
        hint::spin_loop();
    }
    SHOOTDOWN_CLAIM.store(false, Ordering::Release);
    seq
}

/// Iterate every online cpu except `my` (the kernel build walks the dense
/// `0..num_online_cpus()` range the arch reports).
#[cfg(not(test))]
fn each_online_remote(my: usize, mut f: impl FnMut(usize)) {
    let n = num_online_cpus().min(MAX_CPUS);
    for cpu in 0..n {
        if cpu != my {
            f(cpu);
        }
    }
}

/// Host tests iterate the calling thread's own online mask, so two
/// shootdown tests running in parallel (on disjoint cpu-id sets) never
/// wait on — or get acked by — each other's cpus.
#[cfg(test)]
fn each_online_remote(my: usize, mut f: impl FnMut(usize)) {
    let mask = test_hooks::online_mask();
    for cpu in 0..MAX_CPUS {
        if cpu != my && mask & (1u64 << cpu) != 0 {
            f(cpu);
        }
    }
}

// ---------------------------------------------------------------------------
// Host-test hooks (cfg(test) only; never shipped).  Every hook is
// thread-local: each test thread simulates its own cpu, so `cargo test`'s
// parallel threads never observe one another's clocks, IRQ depths or
// registrations.
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod test_hooks {
    use super::{SHOOTDOWN_ACK, SHOOTDOWN_CLAIM, SHOOTDOWN_SEQ, SHOOTDOWN_VA};
    use core::cell::Cell;
    use core::sync::atomic::Ordering;

    std::thread_local! {
        static CPU_ID: Cell<Option<usize>> = const { Cell::new(None) };
        static TICKS: Cell<u64> = const { Cell::new(0) };
        static IRQ_DEPTH: Cell<usize> = const { Cell::new(0) };
        static ONLINE_MASK: Cell<u64> = const { Cell::new(0b1) };
        static FLUSH_COUNT: Cell<u64> = const { Cell::new(0) };
        static LAST_FLUSH_VA: Cell<Option<usize>> = const { Cell::new(None) };
    }

    /// The per-thread cpu-id override (None: the default arch's, cpu 0).
    pub fn cpu_id_override() -> Option<usize> {
        CPU_ID.with(|c| c.get())
    }
    pub fn set_cpu_id(id: usize) {
        CPU_ID.with(|c| c.set(Some(id)));
    }
    pub fn ticks() -> u64 {
        TICKS.with(|c| c.get())
    }
    pub fn set_ticks(v: u64) {
        TICKS.with(|c| c.set(v));
    }
    pub fn advance_ticks(d: u64) {
        TICKS.with(|c| c.set(c.get() + d));
    }
    /// The fake IRQ model: a nesting depth; save returns the previous
    /// depth, restore sets the depth back to the saved value (so a missed
    /// restore on a try_lock failure path is observable).
    pub fn irq_save_disable() -> usize {
        IRQ_DEPTH.with(|c| {
            let prev = c.get();
            c.set(prev + 1);
            prev
        })
    }
    pub fn irq_restore(saved: usize) {
        IRQ_DEPTH.with(|c| c.set(saved));
    }
    pub fn irq_depth() -> usize {
        IRQ_DEPTH.with(|c| c.get())
    }
    /// The calling thread's online-cpu mask (default: cpu 0 only).
    pub fn online_mask() -> u64 {
        ONLINE_MASK.with(|c| c.get())
    }
    pub fn set_online_mask(mask: u64) {
        ONLINE_MASK.with(|c| c.set(mask));
    }
    pub fn record_flush(va: Option<usize>) {
        FLUSH_COUNT.with(|c| c.set(c.get() + 1));
        LAST_FLUSH_VA.with(|c| c.set(va));
    }
    pub fn flush_count() -> u64 {
        FLUSH_COUNT.with(|c| c.get())
    }
    pub fn last_flush_va() -> Option<usize> {
        LAST_FLUSH_VA.with(|c| c.get())
    }

    /// Inject a pending shootdown without an initiator (single-threaded
    /// poll tests).  The caller holds the claim (`claim_shootdown`) so a
    /// parallel request test cannot interleave a VA bump.  Returns the new
    /// generation.
    pub fn inject_shootdown(va: Option<usize>) -> u64 {
        SHOOTDOWN_VA.store(va.unwrap_or(0), Ordering::Relaxed);
        let seq = SHOOTDOWN_SEQ.load(Ordering::Relaxed) + 1;
        SHOOTDOWN_SEQ.store(seq, Ordering::Release);
        seq
    }
    pub fn claim_shootdown() {
        while SHOOTDOWN_CLAIM.swap(true, Ordering::AcqRel) {
            std::thread::yield_now();
        }
    }
    pub fn release_shootdown() {
        SHOOTDOWN_CLAIM.store(false, Ordering::Release);
    }
    /// The ack generation of the calling thread's cpu slot.
    pub fn my_ack() -> u64 {
        SHOOTDOWN_ACK[super::current_cpu_id()].load(Ordering::Acquire)
    }
    /// The current global shootdown generation.
    pub fn current_seq() -> u64 {
        SHOOTDOWN_SEQ.load(Ordering::Acquire)
    }
    /// Whether an arch vtable has been registered (registration state is
    /// observable in tests; registered hooks are never called there).
    pub fn registered() -> bool {
        super::ARCH.get().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panic_message(payload: Box<dyn core::any::Any + Send>) -> String {
        if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else {
            String::new()
        }
    }

    // -- SMP-12: strictly increasing ranks acquire and release cleanly --

    #[test]
    fn lock_rank_valid_order_succeeds() {
        test_hooks::set_cpu_id(20);
        for (rank, name) in [
            (LockRank::DevTop, "dev_top"),
            (LockRank::FidShard, "fid_shard"),
            (LockRank::ProcTable, "proc_table"),
            (LockRank::PageAlloc, "page_alloc"),
            (LockRank::HeapAlloc, "heap_alloc"),
            (LockRank::Console, "console"),
        ] {
            rank_before_acquire(rank, name);
        }
        assert_eq!(test_held_lock_count(), 6);
        // Non-LIFO release clears held ranks cleanly (SMP-12 empty).
        rank_after_release(LockRank::ProcTable, "proc_table");
        rank_after_release(LockRank::DevTop, "dev_top");
        rank_after_release(LockRank::Console, "console");
        rank_after_release(LockRank::FidShard, "fid_shard");
        rank_after_release(LockRank::HeapAlloc, "heap_alloc");
        assert_eq!(test_held_lock_count(), 1);
        rank_after_release(LockRank::PageAlloc, "page_alloc");
        assert_eq!(test_held_lock_count(), 0);
    }

    // -- SMP-12: equal or lower rank panics naming BOTH locks (D-26) --

    #[test]
    fn lock_rank_inversion_panics_naming_both_locks() {
        test_hooks::set_cpu_id(21);
        rank_before_acquire(LockRank::ProcTable, "proc_table");
        let result = std::panic::catch_unwind(|| {
            rank_before_acquire(LockRank::FidShard, "fid_shard");
        });
        let Err(payload) = result else {
            panic!("rank inversion 50 -> 40 was not caught");
        };
        let msg = panic_message(payload);
        assert!(msg.contains("proc_table"), "names the held lock: {msg}");
        assert!(msg.contains("fid_shard"), "names the requested lock: {msg}");
        assert!(msg.contains("50") && msg.contains("40"), "names both ranks: {msg}");
        // Equal rank is an inversion too (50 -> 50).
        let result = std::panic::catch_unwind(|| {
            rank_before_acquire(LockRank::ProcTable, "proc_table_again");
        });
        assert!(result.is_err(), "equal-rank acquire 50 -> 50 must panic");
        // The panics fired before any push; only the original hold remains.
        rank_after_release(LockRank::ProcTable, "proc_table");
        assert_eq!(test_held_lock_count(), 0);
    }

    // -- SMP-12: the watchdog names the stuck cpu, lock and owner (D-26) --

    #[test]
    fn watchdog_reports_stuck_cpu_and_lock() {
        test_hooks::set_cpu_id(22);
        test_hooks::set_ticks(1_000);
        // A stuck cpu: waiting since tick 1_000, no heartbeat progress,
        // now tick 7_000 with a 5_000-tick threshold.
        watchdog_begin_wait("proc_table", 0);
        test_hooks::advance_ticks(6_000);
        let result = std::panic::catch_unwind(|| watchdog_check_all(5_000));
        let Err(payload) = result else {
            panic!("the watchdog did not trip on a stuck cpu");
        };
        let msg = panic_message(payload);
        assert!(msg.contains("WATCHDOG: cpu 22"), "names the stuck cpu: {msg}");
        assert!(msg.contains("6000 ticks"), "names the elapsed ticks: {msg}");
        assert!(msg.contains("\"proc_table\""), "names the waiting lock: {msg}");
        assert!(msg.contains("held by cpu 0"), "names the owner cpu: {msg}");
        watchdog_end_wait();

        // An idle cpu (no waiting lock) never trips, however stale.
        let result = std::panic::catch_unwind(|| watchdog_check_all(1));
        assert!(result.is_ok(), "an idle cpu must not trip the watchdog");

        // An active cpu (heartbeat advanced during the wait) does not trip.
        test_hooks::set_ticks(1_000);
        watchdog_begin_wait("heap_alloc", 1);
        test_hooks::advance_ticks(100_000);
        watchdog_heartbeat();
        let result = std::panic::catch_unwind(|| watchdog_check_all(5_000));
        assert!(result.is_ok(), "an active cpu must not trip the watchdog");
        watchdog_end_wait();

        // A brief wait (within threshold) does not trip.
        test_hooks::set_ticks(1_000);
        watchdog_begin_wait("page_alloc", 0);
        test_hooks::advance_ticks(10);
        let result = std::panic::catch_unwind(|| watchdog_check_all(5_000));
        assert!(result.is_ok(), "a brief wait must not trip the watchdog");
        watchdog_end_wait();
    }

    // -- SMP-07: the shootdown coordinator's poll services and acks --

    #[test]
    fn smp_shootdown_poll_flushes_target_va_and_acks() {
        test_hooks::set_cpu_id(23);
        test_hooks::claim_shootdown();
        let seq = test_hooks::inject_shootdown(Some(0x5000));
        poll_shootdown_if_pending();
        assert_eq!(test_hooks::flush_count(), 1);
        assert_eq!(test_hooks::last_flush_va(), Some(0x5000));
        assert_eq!(test_hooks::my_ack(), seq, "the poll acks the generation");
        // A full flush carries no VA.
        let seq = test_hooks::inject_shootdown(None);
        poll_shootdown_if_pending();
        assert_eq!(test_hooks::last_flush_va(), None);
        assert_eq!(test_hooks::my_ack(), seq);
        // No new request: polling is a no-op.
        let before = test_hooks::flush_count();
        poll_shootdown_if_pending();
        assert_eq!(test_hooks::flush_count(), before);
        test_hooks::release_shootdown();
    }

    #[test]
    fn smp_shootdown_request_completes_when_remote_polls() {
        use std::sync::{Arc, Barrier};
        test_hooks::set_cpu_id(24);
        test_hooks::set_online_mask((1 << 24) | (1 << 25));
        let start = Arc::new(Barrier::new(2));
        let stop = Arc::new(AtomicBool::new(false));
        let start_b = Arc::clone(&start);
        let stop_b = Arc::clone(&stop);
        let poller = std::thread::spawn(move || {
            test_hooks::set_cpu_id(25);
            start_b.wait();
            while !stop_b.load(Ordering::Relaxed) {
                poll_shootdown_if_pending();
                std::thread::yield_now();
            }
            test_hooks::flush_count()
        });
        start.wait();
        // Returns only after cpu 25's poll services and acks the request.
        let seq = request_shootdown(Some(0x9000));
        stop.store(true, Ordering::Relaxed);
        let remote_flushes = poller.join().expect("poller thread");
        assert!(remote_flushes >= 1, "the remote cpu flushed through its poll");
        assert_eq!(test_hooks::my_ack(), seq, "the initiator's ack is current");
    }

    // -- SMP-02: the arch vtable registers exactly once (D-25) --

    #[test]
    fn register_arch_publishes_once() {
        static TEST_ARCH: SmpArch = SmpArch {
            current_cpu_id: default_current_cpu_id,
            num_online_cpus: default_num_online_cpus,
            irq_save_disable: default_irq_save_disable,
            irq_restore: default_irq_restore,
            send_resched_ipi: default_send_ipi,
            send_shootdown_ipi: default_send_ipi,
            local_tlb_flush: default_local_tlb_flush,
            ticks: default_ticks,
            prepare_block: default_prepare_block,
        };
        register_arch(&TEST_ARCH);
        assert!(test_hooks::registered());
        let result = std::panic::catch_unwind(|| register_arch(&TEST_ARCH));
        assert!(result.is_err(), "a second registration must panic");
    }
}
