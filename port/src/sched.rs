//! sched — the portable per-CPU work-stealing scheduler run queues
//! (SMP-11; D-23, D-25, D-26).
//!
//! One cache-line-aligned queue per cpu, guarded by an
//! [`IrqSpinLock`](crate::mcslock::IrqSpinLock) at
//! [`LockRank::RunQueue`] (60): [`RunQueues::pop_or_steal`] pops the
//! caller's own queue front (FIFO per cpu) and steals from the BACK of a
//! neighbor's queue, so the local FIFO order is preserved while a thief
//! takes the oldest-enqueued work's tail.  The steal discipline is the
//! load-bearing invariant: the local queue lock is RELEASED before any
//! victim queue is probed, and the victim probe is a `try_lock` — a cpu
//! NEVER holds two RunQueue locks at once (equal-rank nesting is a
//! rank-stack violation, D-26) and NEVER waits on a remote queue (a
//! contended victim is simply skipped; the next neighbor is tried).
//!
//! # Idle tracking (SMP-11 adjacency/empty, D-23)
//!
//! A cpu that finds every queue empty publishes itself in `idle_mask`
//! (one atomic bitmask — MAX_CPUS is capped at 64) and returns `None`;
//! the arch sched loop then sleeps (wfi/hlt) until the reschedule IPI.
//! [`RunQueues::enqueue`] consults the mask after pushing and returns
//! `Some(idle_cpu)` — preferring the target cpu itself when it is idle —
//! so the arch caller (plans 05-09/05-11) sends the reschedule IPI
//! (GIC SGI 0 / LAPIC vector 0xF1).  A successful pop clears the
//! popping cpu's bit.
//!
//! Nothing here hard-codes two cpus (D-23): every entry point takes the
//! live `num_cpus` and the host stress test runs four threads over
//! `RunQueues<4>`.

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::mcslock::{IrqSpinLock, LockNode};
use crate::smp::LockRank;

/// One cpu's run queue: an IRQ-safe ranked lock around a FIFO deque,
/// cache-line aligned so one cpu's queue lock never shares a line with
/// a neighbor's (the steal path would otherwise false-share the victim's
/// line on every probe).
#[repr(align(64))]
pub struct CpuQueue {
    q: IrqSpinLock<VecDeque<u64>>,
}

const _: () = {
    assert!(core::mem::align_of::<CpuQueue>() == 64);
};

/// The per-CPU run queues plus the idle-cpu bitmask (SMP-11).
///
/// `MAX_CPUS` defaults to 64 ([`crate::smp::MAX_CPUS`]) and is capped
/// there because `idle_mask` is one `AtomicU64`; the const assertion
/// fires at construction otherwise.
pub struct RunQueues<const MAX_CPUS: usize = 64> {
    queues: [CpuQueue; MAX_CPUS],
    /// Bit i set = cpu i is idle (its sched loop is about to sleep).
    idle_mask: AtomicU64,
}

impl<const MAX_CPUS: usize> RunQueues<MAX_CPUS> {
    /// An all-empty, all-busy set of run queues.
    pub const fn new() -> RunQueues<MAX_CPUS> {
        assert!(MAX_CPUS <= 64, "sched: idle_mask is one u64, so MAX_CPUS <= 64");
        RunQueues {
            queues: [const {
                CpuQueue { q: IrqSpinLock::new("runq", LockRank::RunQueue, VecDeque::new()) }
            }; MAX_CPUS],
            idle_mask: AtomicU64::new(0),
        }
    }

    /// Enqueue `pid` onto `target_cpu`'s queue (`target_cpu % num_cpus`),
    /// then report an idle cpu to kick with the reschedule IPI: the
    /// target itself when it is idle, else the lowest-numbered idle cpu
    /// below `num_cpus`, else None.  The caller (arch sched code) sends
    /// the IPI; a spurious IPI is benign (the target re-idles when it
    /// finds nothing).  The push (under the queue lock, Release on drop)
    /// happens-before this Acquire mask read, pairing the idle
    /// publication in [`Self::pop_or_steal`] — an enqueue that races an
    /// about-to-sleep cpu either sees its idle bit here (and wakes it)
    /// or is seen by that cpu's post-publish re-check, so a wake is
    /// never lost.
    pub fn enqueue(&self, target_cpu: usize, pid: u64, num_cpus: usize) -> Option<usize> {
        debug_assert!(num_cpus > 0 && num_cpus <= MAX_CPUS);
        let target = target_cpu % num_cpus;
        {
            let node = LockNode::new();
            self.queues[target].q.lock(&node).push_back(pid);
        }
        let mask = self.idle_mask.load(Ordering::Acquire);
        if mask & (1u64 << target) != 0 {
            return Some(target);
        }
        for cpu in 0..num_cpus {
            if mask & (1u64 << cpu) != 0 {
                return Some(cpu);
            }
        }
        None
    }

    /// Pop the next pid for `my_cpu`: the front of the local queue
    /// (FIFO per cpu — SMP-11 ordering).  When the local queue is
    /// empty, steal the BACK of a neighbor's queue via `try_lock`
    /// (never waiting, never holding two RunQueue locks — the local
    /// lock is released before the first probe).  When every queue is
    /// empty, publish `my_cpu` in the idle mask and return `None`
    /// (SMP-11 empty).
    pub fn pop_or_steal(&self, my_cpu: usize, num_cpus: usize) -> Option<u64> {
        debug_assert!(my_cpu < num_cpus && num_cpus <= MAX_CPUS);
        // 1. Local pop (FIFO per cpu).  The guard is scoped to drop
        //    BEFORE any steal probe: a cpu never holds two RunQueue
        //    locks (equal-rank nesting is a rank violation, D-26).
        {
            let node = LockNode::new();
            let mut q = self.queues[my_cpu].q.lock(&node);
            if let Some(pid) = q.pop_front() {
                self.idle_mask.fetch_and(!(1u64 << my_cpu), Ordering::AcqRel);
                return Some(pid);
            }
        }
        // 2. Steal the BACK of a neighbor's queue (the local FIFO is
        //    untouched by thieves).  try_lock only: a contended victim
        //    is skipped, never waited on.
        if let Some(pid) = self.steal_scan(my_cpu, num_cpus) {
            return Some(pid);
        }
        // 3. All empty: publish idle, then RE-CHECK once.  An enqueue
        //    whose push preceded its mask read (which saw us not-idle)
        //    lands in this re-check — the flag/data pairing that keeps
        //    a racing wakeup from being lost (the Plan 9 / Linux idle
        //    loop shape; SMP-11 empty).
        self.idle_mask.fetch_or(1u64 << my_cpu, Ordering::AcqRel);
        {
            let node = LockNode::new();
            let mut q = self.queues[my_cpu].q.lock(&node);
            if let Some(pid) = q.pop_front() {
                self.idle_mask.fetch_and(!(1u64 << my_cpu), Ordering::AcqRel);
                return Some(pid);
            }
        }
        self.steal_scan(my_cpu, num_cpus)
    }

    /// The steal scan shared by the hot path and the post-idle-publish
    /// re-check: neighbors in round-robin order starting at
    /// `my_cpu + 1`, victim lock via `try_lock`, victim pop from the
    /// BACK.  A successful steal clears `my_cpu`'s idle bit.
    fn steal_scan(&self, my_cpu: usize, num_cpus: usize) -> Option<u64> {
        for offset in 1..num_cpus {
            let victim = (my_cpu + offset) % num_cpus;
            let node = LockNode::new();
            let Some(mut q) = self.queues[victim].q.try_lock(&node) else {
                continue;
            };
            if let Some(pid) = q.pop_back() {
                self.idle_mask.fetch_and(!(1u64 << my_cpu), Ordering::AcqRel);
                return Some(pid);
            }
        }
        None
    }

    /// Mark `cpu` idle explicitly (the arch idle loop's explicit arm —
    /// e.g. before a wfi entered for reasons other than an empty pop).
    pub fn mark_idle(&self, cpu: usize) {
        self.idle_mask.fetch_or(1u64 << cpu, Ordering::AcqRel);
    }

    /// Mark `cpu` busy explicitly (the arch boot/wake path's clear).
    pub fn mark_busy(&self, cpu: usize) {
        self.idle_mask.fetch_and(!(1u64 << cpu), Ordering::AcqRel);
    }

    /// Claim the lowest-numbered idle cpu below `num_cpus`, clearing its
    /// bit atomically (the arch IPI-sender's take, so two concurrent
    /// senders never both wake the same cpu).  None when none are idle.
    pub fn take_idle_cpu(&self, num_cpus: usize) -> Option<usize> {
        let mut mask = self.idle_mask.load(Ordering::Acquire);
        loop {
            let mut cpu = None;
            for c in 0..num_cpus.min(MAX_CPUS) {
                if mask & (1u64 << c) != 0 {
                    cpu = Some(c);
                    break;
                }
            }
            let c = cpu?;
            match self.idle_mask.compare_exchange(
                mask,
                mask & !(1u64 << c),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(c),
                Err(observed) => mask = observed,
            }
        }
    }

    /// The number of pids queued for `cpu`.
    pub fn len_for_cpu(&self, cpu: usize) -> usize {
        let node = LockNode::new();
        self.queues[cpu].q.lock(&node).len()
    }

    /// Whether ANY cpu's queue holds an entry (busy=false when it may hold
    /// only stale entries — callers treat a false positive as "go look").
    /// The arch reschedule fast path probes with this before parking the
    /// current proc into its sched loop (a scheduler round trip it only
    /// owes when somebody is waiting); one queue lock at a time, never
    /// nested.
    pub fn any_enqueued(&self, num_cpus: usize) -> bool {
        for cpu in 0..num_cpus.min(MAX_CPUS) {
            let node = LockNode::new();
            if !self.queues[cpu].q.lock(&node).is_empty() {
                return true;
            }
        }
        false
    }

    /// The total number of pids queued across all cpus.
    pub fn total_len(&self) -> usize {
        let mut total = 0;
        for queue in self.queues.iter() {
            let node = LockNode::new();
            total += queue.q.lock(&node).len();
        }
        total
    }
}

impl<const MAX_CPUS: usize> Default for RunQueues<MAX_CPUS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use core::sync::atomic::AtomicUsize;

    // -- SMP-11 ordering: the local queue pops FIFO on its own cpu --

    #[test]
    fn runqueues_local_pop_is_fifo() {
        crate::smp::test_hooks::set_cpu_id(0);
        let rq = RunQueues::<4>::new();
        for pid in [7u64, 8, 9] {
            rq.enqueue(0, pid, 2);
        }
        assert_eq!(rq.len_for_cpu(0), 3);
        assert_eq!(rq.total_len(), 3);
        assert_eq!(rq.pop_or_steal(0, 2), Some(7));
        assert_eq!(rq.pop_or_steal(0, 2), Some(8));
        assert_eq!(rq.pop_or_steal(0, 2), Some(9));
        assert_eq!(rq.pop_or_steal(0, 2), None);
        assert_eq!(rq.total_len(), 0);
        assert_eq!(
            crate::smp::test_held_lock_count(),
            0,
            "every op balanced its rank push/pop (D-26)"
        );
    }

    // -- SMP-11 adjacency: a neighbor steals from the victim's BACK via
    // -- try_lock, never holding two RunQueue locks (D-23, D-26) --

    #[test]
    fn runqueues_steal_pops_victim_back_with_try_lock() {
        crate::smp::test_hooks::set_cpu_id(0);
        let rq = RunQueues::<4>::new();
        rq.enqueue(0, 100, 4);
        rq.enqueue(0, 101, 4);
        rq.enqueue(0, 102, 4);
        assert_eq!(rq.len_for_cpu(0), 3);
        crate::smp::test_hooks::set_cpu_id(1);
        // cpu 1's queue is empty: the steal takes the BACK of cpu 0's
        // queue (the local FIFO is untouched by thieves).
        assert_eq!(rq.pop_or_steal(1, 4), Some(102));
        assert_eq!(
            crate::smp::test_held_lock_count(),
            0,
            "the steal released every RunQueue lock (never two held)"
        );
        assert_eq!(rq.pop_or_steal(1, 4), Some(101));
        assert_eq!(rq.pop_or_steal(1, 4), Some(100));
        assert_eq!(rq.pop_or_steal(1, 4), None);
        assert_eq!(rq.total_len(), 0);
    }

    // -- SMP-11 adjacency: enqueue reports an idle cpu to wake (the
    // -- reschedule IPI target), preferring the target itself (D-23) --

    #[test]
    fn runqueues_enqueue_wakes_an_idle_cpu() {
        let rq = RunQueues::<4>::new();
        crate::smp::test_hooks::set_cpu_id(2);
        // cpu 2 finds every queue empty and transitions to idle tracking.
        assert_eq!(rq.pop_or_steal(2, 4), None);
        crate::smp::test_hooks::set_cpu_id(0);
        // Enqueueing to busy cpu 0 wakes the idle cpu 2.
        assert_eq!(rq.enqueue(0, 42, 4), Some(2));

        // Prefer the TARGET when it is the idle one.
        let rq2 = RunQueues::<4>::new();
        rq2.mark_idle(1);
        assert_eq!(rq2.enqueue(1, 7, 4), Some(1));
        // take_idle_cpu claims (clears) the bit — the IPI sender's take.
        assert_eq!(rq2.take_idle_cpu(4), Some(1));
        assert_eq!(rq2.take_idle_cpu(4), None);
        // mark_busy clears without a take.
        rq2.mark_idle(3);
        rq2.mark_busy(3);
        assert_eq!(rq2.take_idle_cpu(4), None);
    }

    /// The arch reschedule fast path's probe: false only when every queue
    /// is empty (a pending entry on ANY cpu means the scheduler owes a
    /// look), true after an enqueue, false again once popped.
    #[test]
    fn runqueues_any_enqueued_tracks_queued_work() {
        crate::smp::test_hooks::set_cpu_id(0);
        let rq = RunQueues::<4>::new();
        assert!(!rq.any_enqueued(4), "all queues start empty");
        rq.enqueue(3, 55, 4);
        assert!(rq.any_enqueued(4), "a remote queue's entry is seen");
        assert_eq!(rq.pop_or_steal(0, 4), Some(55), "stolen by cpu 0");
        assert!(!rq.any_enqueued(4), "empty again after the pop");
        assert!(
            !rq.any_enqueued(0),
            "no queues in range (0 cpus) is empty, never a panic"
        );
    }

    // -- SMP-11: four threads (num_cpus = 4 — never hardcoded 2, D-23)
    // -- push/pop/steal 10,000 pids; every pid is popped EXACTLY once
    // -- (zero lost, zero duplicated, zero stranded) --

    #[test]
    fn runqueues_four_cpu_steal_conserves_10k_pids() {
        use std::sync::Arc;
        use std::time::{Duration, Instant};
        const N: u64 = 10_000;
        const CONSUMERS: usize = 3;
        let rq = Arc::new(RunQueues::<4>::new());
        let counts: Arc<Vec<AtomicUsize>> =
            Arc::new((0..N as usize).map(|_| AtomicUsize::new(0)).collect());
        let produced = Arc::new(AtomicU64::new(0));
        let popped_total = Arc::new(AtomicU64::new(0));
        let took_first = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(std::sync::Barrier::new(4));
        // The producer (cpu 0) lands every pid on queue 0 — consumers on
        // cpus 1..4 see empty local queues and must STEAL every pid.
        let producer = {
            let rq = Arc::clone(&rq);
            let produced = Arc::clone(&produced);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                crate::smp::test_hooks::set_cpu_id(0);
                start.wait();
                for pid in 0..N {
                    rq.enqueue(0, pid, 4);
                    produced.fetch_add(1, Ordering::Release);
                }
            })
        };
        let mut consumers = Vec::new();
        for cpu in 1..=CONSUMERS {
            let rq = Arc::clone(&rq);
            let counts = Arc::clone(&counts);
            let produced = Arc::clone(&produced);
            let popped_total = Arc::clone(&popped_total);
            let took_first = Arc::clone(&took_first);
            let start = Arc::clone(&start);
            consumers.push(std::thread::spawn(move || {
                crate::smp::test_hooks::set_cpu_id(cpu);
                start.wait();
                // The host's scheduler is not what this test pins.  On a
                // four-CPU host (CI) other tests' threads can keep a consumer
                // off its CPU until its siblings have drained the queue, so
                // "each stole at least one" holds by construction instead:
                // no consumer takes a second pid before every one has its
                // first.  The waits are bounded by time, not by a count of
                // empty polls — a count of yields is spent in a fraction of a
                // second of host-side preemption.
                let give_up_at = Instant::now() + Duration::from_secs(60);
                let mut mine = 0u64;
                let mut gave_up = false;
                loop {
                    if mine > 0 && took_first.load(Ordering::Acquire) < CONSUMERS {
                        if Instant::now() > give_up_at {
                            gave_up = true;
                            break;
                        }
                        std::thread::yield_now();
                        continue;
                    }
                    match rq.pop_or_steal(cpu, 4) {
                        Some(pid) => {
                            counts[pid as usize].fetch_add(1, Ordering::Relaxed);
                            popped_total.fetch_add(1, Ordering::Relaxed);
                            mine += 1;
                            if mine == 1 {
                                took_first.fetch_add(1, Ordering::Release);
                            }
                        }
                        None => {
                            if mine > 0
                                && produced.load(Ordering::Acquire) == N
                                && popped_total.load(Ordering::Acquire) == N
                            {
                                break;
                            }
                            if Instant::now() > give_up_at {
                                // give up rather than wedge the host test
                                gave_up = true;
                                break;
                            }
                            std::thread::yield_now();
                        }
                    }
                }
                (mine, gave_up)
            }));
        }
        producer.join().expect("producer thread");
        let per_cpu: Vec<(u64, bool)> =
            consumers.into_iter().map(|h| h.join().expect("consumer thread")).collect();
        for (i, &(_, gave_up)) in per_cpu.iter().enumerate() {
            assert!(
                !gave_up,
                "cpu {} gave up after 60 s: the producer or a sibling consumer never ran",
                i + 1
            );
        }
        assert_eq!(
            popped_total.load(Ordering::Relaxed),
            N,
            "every pid popped exactly once — none stranded, lost or duplicated"
        );
        for (i, c) in counts.iter().enumerate() {
            assert_eq!(c.load(Ordering::Relaxed), 1, "pid {i} popped exactly once");
        }
        for (i, &(m, _)) in per_cpu.iter().enumerate() {
            assert!(m > 0, "cpu {} stole at least one pid from cpu 0", i + 1);
        }
    }
}
