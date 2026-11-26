//! MCS locks
//!
//! Reference:
//!
//! John M. Mellor-Crummey and Michael L. Scott. 1991. Algorithms
//! for Scalable Synchronization on Shared Memory Multiprocessors.
//! ACM Transactions on Computer Systems 9, 1 (Feb. 1991), 21–65.
//! DOI: https://doi.org/10.1145/103727.103729

use core::cell::UnsafeCell;
use core::hint;
use core::marker::{Send, Sized, Sync};
use core::ops::{Deref, DerefMut};
use core::ptr;
#[cfg(debug_assertions)]
use core::sync::atomic::AtomicU32;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use crate::smp::LockRank;

/// Represents a node in the lock structure.  Note, is cacheline
/// aligned.
#[repr(align(64))]
pub struct LockNode {
    next: AtomicPtr<LockNode>,
    locked: AtomicBool,
}

impl LockNode {
    pub const fn new() -> LockNode {
        LockNode { next: AtomicPtr::new(ptr::null_mut()), locked: AtomicBool::new(false) }
    }
}

impl Default for LockNode {
    fn default() -> Self {
        Self::new()
    }
}

/// An MCS lock.
pub struct MCSLock {
    _name: &'static str,
    queue: AtomicPtr<LockNode>,
}

impl MCSLock {
    pub const fn new(name: &'static str) -> MCSLock {
        MCSLock { _name: name, queue: AtomicPtr::new(ptr::null_mut()) }
    }

    pub fn lock<'a>(&self, node: &'a LockNode) -> &'a LockNode {
        node.next.store(ptr::null_mut(), Ordering::Release);
        node.locked.store(false, Ordering::Release);
        let p = node as *const _ as *mut _;
        let predecessor = self.queue.swap(p, Ordering::AcqRel);
        if !predecessor.is_null() {
            // SAFETY: predecessor names a live LockNode whose owner is
            // mid-lock on THIS queue (the MCS protocol: the queue swap
            // published our node; the predecessor waits on its own locked
            // flag until we link it).  The link outlives the handshake.
            let predecessor = unsafe { &*predecessor };
            node.locked.store(true, Ordering::Release);
            predecessor.next.store(p, Ordering::Release);
            while node.locked.load(Ordering::Acquire) {
                // SMP-07 (Pitfall 1): a cpu spinning here may have IRQs
                // masked; service any pending TLB shootdown inline so a
                // remote initiator's wait barrier can always complete.
                crate::smp::poll_shootdown_if_pending();
                hint::spin_loop();
            }
        }
        node
    }

    /// A non-blocking acquire: true when the queue was empty and `node`
    /// now holds the lock, false (without linking or waiting) otherwise.
    pub fn try_lock(&self, node: &LockNode) -> bool {
        node.next.store(ptr::null_mut(), Ordering::Release);
        node.locked.store(false, Ordering::Release);
        let p = node as *const _ as *mut _;
        // The uncontended fast path of the MCS protocol: claim a null
        // queue tail.  A miss never linked the node, so it stays reusable.
        self.queue
            .compare_exchange(ptr::null_mut(), p, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }

    pub fn unlock(&self, node: &LockNode) {
        if node.next.load(Ordering::Acquire).is_null() {
            let p = node as *const _ as *mut _;
            loop {
                if self
                    .queue
                    .compare_exchange_weak(p, ptr::null_mut(), Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }
                // A weak CAS may fail SPURIOUSLY (aarch64 stlxr: the
                // exclusive monitor clears on any interference — under hvf
                // every vcpu exit does it, so this fires constantly where
                // TCG never sees it).  If no successor has linked, the queue
                // tail is still us: retry the CAS — waiting on node.next
                // here would wedge on a successor that doesn't exist.
                if !node.next.load(Ordering::Acquire).is_null() {
                    break;
                }
                hint::spin_loop();
            }
        }
        let next = node.next.load(Ordering::Acquire);
        // SAFETY: next names the live successor LockNode the lock()
        // handshake linked (the spin above guarantees the link landed);
        // its owner spins on locked until this store releases it.
        let next = unsafe { &*next };
        next.locked.store(false, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// OnceStatic: the publish-once boot cell.
//
// The portable SMP core (`port/src/smp.rs`, SMP-02) keeps its arch hook
// vtable in one of these: registered exactly once during single-CPU early
// boot, read on hot paths thereafter.  The read is lock-free — one atomic
// load — so it is safe from IRQ-masked contexts where taking even an MCS
// lock could deadlock (the hooks are called from the lock machinery
// itself).  The cell lives in this floor module because `smp` is
// `#[forbid(unsafe_code)]` and the published-pointer dereference is the
// one unsafe site the mechanism needs.
// ---------------------------------------------------------------------------

/// A publish-once cell for a `&'static T` registered once during
/// single-CPU early boot (see the module note above).
pub struct OnceStatic<T> {
    ptr: AtomicPtr<T>,
}

impl<T> OnceStatic<T> {
    pub const fn new() -> OnceStatic<T> {
        OnceStatic { ptr: AtomicPtr::new(ptr::null_mut()) }
    }

    /// Publish `val`.  Boot discipline: called exactly once, before the
    /// second cpu starts; a second call panics.
    pub fn publish(&self, val: &'static T) {
        let p = val as *const T as *mut T;
        if self
            .ptr
            .compare_exchange(ptr::null_mut(), p, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            panic!("OnceStatic: published twice");
        }
    }

    /// The published value, or None before publication.
    pub fn get(&self) -> Option<&'static T> {
        let p = self.ptr.load(Ordering::Acquire);
        if p.is_null() {
            return None;
        }
        // SAFETY: p was published from a &'static T (publish above) and is
        // never replaced (publish panics on a second call), so it names a
        // live T for the rest of the program's lifetime; this load's
        // Acquire pairs with publish's AcqRel.
        Some(unsafe { &*p })
    }
}

impl<T> Default for OnceStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Lock<T: ?Sized> {
    lock: UnsafeCell<MCSLock>,
    /// The lock's static name (read only by the debug/gate rank and
    /// watchdog hooks; release builds carry neither — D-26).
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    name: &'static str,
    /// The documented lock rank (SMP-12, D-26): None for `Lock::new`'s
    /// unranked legacy callers (equal-rank nesting panics make any real
    /// default rank wrong for them; the SMP-03 plans rank each caller),
    /// Some for `new_ranked`.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    rank: Option<LockRank>,
    /// The cpu currently holding the lock (the watchdog's "held by cpu
    /// <owner>"), NO_OWNER when free.
    #[cfg(debug_assertions)]
    owner_cpu: AtomicU32,
    data: UnsafeCell<T>,
}
// SAFETY: the MCS protocol (lock/unlock above) serializes all access to
// the protected data — a Lock can cross threads because the queue
// discipline is the synchronization (the named floor module).
unsafe impl<T: ?Sized> Send for Lock<T> {}
// SAFETY: as above — &Lock sharing is safe because data access only ever
// happens under the queue's mutual exclusion.
unsafe impl<T: ?Sized> Sync for Lock<T> {}

impl<T> Lock<T> {
    pub const fn new(name: &'static str, data: T) -> Lock<T> {
        Lock {
            lock: UnsafeCell::new(MCSLock::new(name)),
            name,
            rank: None,
            #[cfg(debug_assertions)]
            owner_cpu: AtomicU32::new(crate::smp::NO_OWNER),
            data: UnsafeCell::new(data),
        }
    }

    /// A lock with a documented rank (SMP-12, D-26): debug/gate builds
    /// assert strictly increasing acquisition order and feed the per-cpu
    /// watchdog; `Lock::new` stays the unranked form for callers not yet
    /// ranked by the SMP-03 migration.
    pub const fn new_ranked(name: &'static str, rank: LockRank, data: T) -> Lock<T> {
        Lock {
            lock: UnsafeCell::new(MCSLock::new(name)),
            name,
            rank: Some(rank),
            #[cfg(debug_assertions)]
            owner_cpu: AtomicU32::new(crate::smp::NO_OWNER),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock<'a>(&'a self, node: &'a LockNode) -> LockGuard<'a, T> {
        #[cfg(debug_assertions)]
        if let Some(rank) = self.rank {
            crate::smp::rank_before_acquire(rank, self.name);
            crate::smp::watchdog_begin_wait(self.name, self.owner_cpu.load(Ordering::Relaxed));
        }
        // SAFETY: UnsafeCell::get on the lock is the protocol entry (the
        // MCS queue is internally synchronized); the guard's &mut data is
        // unique because the lock was just acquired.
        let node = unsafe { &mut *self.lock.get() }.lock(node);
        #[cfg(debug_assertions)]
        if self.rank.is_some() {
            crate::smp::watchdog_end_wait();
            self.owner_cpu.store(crate::smp::current_cpu_id() as u32, Ordering::Relaxed);
        }
        // SAFETY: the lock is held (above), so the data borrow is
        // exclusive for the guard's lifetime; the guard's Drop releases.
        LockGuard {
            lock: &self.lock,
            node,
            // SAFETY: as above — the lock is held, so the data borrow is
            // exclusive for the guard's lifetime.
            data: unsafe { &mut *self.data.get() },
            #[cfg(debug_assertions)]
            rank: self.rank,
            #[cfg(debug_assertions)]
            name: self.name,
            #[cfg(debug_assertions)]
            owner_cpu: &self.owner_cpu,
        }
    }

    /// A non-blocking acquire: Some(guard) when the lock was free, None
    /// (without waiting) otherwise.  The rank order is still enforced (a
    /// successful take would violate it); a miss pushes nothing.
    pub fn try_lock<'a>(&'a self, node: &'a LockNode) -> Option<LockGuard<'a, T>> {
        #[cfg(debug_assertions)]
        if let Some(rank) = self.rank {
            crate::smp::rank_before_acquire(rank, self.name);
        }
        // SAFETY: UnsafeCell::get on the lock is the protocol entry (the
        // MCS queue is internally synchronized).
        if !unsafe { &mut *self.lock.get() }.try_lock(node) {
            #[cfg(debug_assertions)]
            if let Some(rank) = self.rank {
                // Pop the rank pushed by rank_before_acquire above.
                crate::smp::rank_after_release(rank, self.name);
            }
            return None;
        }
        #[cfg(debug_assertions)]
        if self.rank.is_some() {
            self.owner_cpu.store(crate::smp::current_cpu_id() as u32, Ordering::Relaxed);
        }
        // SAFETY: the lock is held (the CAS above claimed the empty
        // queue), so the data borrow is exclusive for the guard's
        // lifetime; the guard's Drop releases.
        Some(LockGuard {
            lock: &self.lock,
            node,
            // SAFETY: as above — the lock is held, so the data borrow is
            // exclusive for the guard's lifetime.
            data: unsafe { &mut *self.data.get() },
            #[cfg(debug_assertions)]
            rank: self.rank,
            #[cfg(debug_assertions)]
            name: self.name,
            #[cfg(debug_assertions)]
            owner_cpu: &self.owner_cpu,
        })
    }
}

pub struct LockGuard<'a, T: ?Sized + 'a> {
    lock: &'a UnsafeCell<MCSLock>,
    node: &'a LockNode,
    data: &'a mut T,
    #[cfg(debug_assertions)]
    rank: Option<LockRank>,
    #[cfg(debug_assertions)]
    name: &'static str,
    #[cfg(debug_assertions)]
    owner_cpu: &'a AtomicU32,
}
impl<T> Deref for LockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<T> DerefMut for LockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<T: ?Sized> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        if let Some(rank) = self.rank {
            self.owner_cpu.store(crate::smp::NO_OWNER, Ordering::Relaxed);
            crate::smp::rank_after_release(rank, self.name);
        }
        // SAFETY: the guard's existence proves the lock is held by this
        // context; unlock publishes our node's release through the MCS
        // queue.
        unsafe { &mut *self.lock.get() }.unlock(self.node);
    }
}

// ---------------------------------------------------------------------------
// IrqSpinLock: the IRQ-safe ranked spin lock (SMP-07, SMP-12; D-25, D-26).
//
// The lock for IRQ-context shared state: lock() masks interrupts BEFORE
// acquiring (so an IRQ on this cpu can never re-enter a lock this cpu
// holds), takes the MCS queue (whose spin polls TLB shootdowns — see
// MCSLock::lock), and the guard's Drop restores the saved IRQ state after
// releasing.  Every IrqSpinLock carries a documented LockRank; debug/gate
// builds assert strictly increasing acquisition order and feed the
// per-cpu progress watchdog (port/src/smp.rs, D-26).
// ---------------------------------------------------------------------------

/// An IRQ-safe ranked spin lock (see the module note above).
pub struct IrqSpinLock<T: ?Sized> {
    /// The lock's static name (read only by the debug/gate rank and
    /// watchdog hooks; release builds carry neither — D-26).
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    name: &'static str,
    /// The documented lock rank (SMP-12, D-26); every IrqSpinLock is
    /// ranked.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    rank: LockRank,
    lock: UnsafeCell<MCSLock>,
    /// The cpu currently holding the lock (the watchdog's "held by cpu
    /// <owner>"), NO_OWNER when free.
    #[cfg(debug_assertions)]
    owner_cpu: AtomicU32,
    data: UnsafeCell<T>,
}

// SAFETY: the MCS protocol serializes all access to the protected data,
// and the IRQ mask/restore pairing closes the same-cpu IRQ re-entry
// window — an IrqSpinLock can cross threads because the queue discipline
// plus the IRQ mask is the synchronization (the named floor module).
unsafe impl<T: ?Sized> Send for IrqSpinLock<T> {}
// SAFETY: as above — &IrqSpinLock sharing is safe because data access
// only ever happens under the queue's mutual exclusion with IRQs masked.
unsafe impl<T: ?Sized> Sync for IrqSpinLock<T> {}

impl<T> IrqSpinLock<T> {
    pub const fn new(name: &'static str, rank: LockRank, data: T) -> IrqSpinLock<T> {
        IrqSpinLock {
            name,
            rank,
            lock: UnsafeCell::new(MCSLock::new(name)),
            #[cfg(debug_assertions)]
            owner_cpu: AtomicU32::new(crate::smp::NO_OWNER),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> IrqSpinLock<T> {
    /// Mask IRQs, check the rank, and acquire; the guard's Drop pops the
    /// rank, releases and restores the saved IRQ state.
    pub fn lock<'a>(&'a self, node: &'a LockNode) -> IrqLockGuard<'a, T> {
        let saved_irq = crate::smp::irq_save_disable();
        #[cfg(debug_assertions)]
        {
            crate::smp::rank_before_acquire(self.rank, self.name);
            crate::smp::watchdog_begin_wait(self.name, self.owner_cpu.load(Ordering::Relaxed));
        }
        // SAFETY: UnsafeCell::get on the lock is the protocol entry (the
        // MCS queue is internally synchronized); the guard's &mut data is
        // unique because the lock was just acquired.  IRQs are masked
        // (above), so no IRQ on this cpu can re-enter the acquisition.
        let node = unsafe { &mut *self.lock.get() }.lock(node);
        #[cfg(debug_assertions)]
        {
            crate::smp::watchdog_end_wait();
            self.owner_cpu.store(crate::smp::current_cpu_id() as u32, Ordering::Relaxed);
        }
        // SAFETY: the lock is held (above), so the data borrow is
        // exclusive for the guard's lifetime; the guard's Drop releases.
        IrqLockGuard {
            lock: &self.lock,
            node,
            // SAFETY: as above — the lock is held, so the data borrow is
            // exclusive for the guard's lifetime.
            data: unsafe { &mut *self.data.get() },
            saved_irq,
            #[cfg(debug_assertions)]
            name: self.name,
            #[cfg(debug_assertions)]
            rank: self.rank,
            #[cfg(debug_assertions)]
            owner_cpu: &self.owner_cpu,
        }
    }

    /// A non-blocking acquire: Some(guard) when the lock was free, None
    /// (without waiting) otherwise.  A miss restores the saved IRQ state
    /// immediately and leaks no rank.
    pub fn try_lock<'a>(&'a self, node: &'a LockNode) -> Option<IrqLockGuard<'a, T>> {
        let saved_irq = crate::smp::irq_save_disable();
        #[cfg(debug_assertions)]
        crate::smp::rank_before_acquire(self.rank, self.name);
        // SAFETY: UnsafeCell::get on the lock is the protocol entry.
        if !unsafe { &mut *self.lock.get() }.try_lock(node) {
            #[cfg(debug_assertions)]
            crate::smp::rank_after_release(self.rank, self.name);
            crate::smp::irq_restore(saved_irq);
            return None;
        }
        #[cfg(debug_assertions)]
        self.owner_cpu.store(crate::smp::current_cpu_id() as u32, Ordering::Relaxed);
        // SAFETY: the lock is held (the CAS claimed the empty queue), so
        // the data borrow is exclusive for the guard's lifetime.
        Some(IrqLockGuard {
            lock: &self.lock,
            node,
            // SAFETY: as above — the lock is held, so the data borrow is
            // exclusive for the guard's lifetime.
            data: unsafe { &mut *self.data.get() },
            saved_irq,
            #[cfg(debug_assertions)]
            name: self.name,
            #[cfg(debug_assertions)]
            rank: self.rank,
            #[cfg(debug_assertions)]
            owner_cpu: &self.owner_cpu,
        })
    }
}

/// The IrqSpinLock ownership guard (the LockGuard analogue): Drop pops the
/// rank, releases the MCS queue and restores the IRQ state saved at
/// acquisition — in that order, so the rank stack and the IRQ mask always
/// balance.
pub struct IrqLockGuard<'a, T: ?Sized + 'a> {
    lock: &'a UnsafeCell<MCSLock>,
    node: &'a LockNode,
    data: &'a mut T,
    saved_irq: usize,
    #[cfg(debug_assertions)]
    name: &'static str,
    #[cfg(debug_assertions)]
    rank: LockRank,
    #[cfg(debug_assertions)]
    owner_cpu: &'a AtomicU32,
}

impl<T> Deref for IrqLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<T> DerefMut for IrqLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<T: ?Sized> Drop for IrqLockGuard<'_, T> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.owner_cpu.store(crate::smp::NO_OWNER, Ordering::Relaxed);
            crate::smp::rank_after_release(self.rank, self.name);
        }
        // SAFETY: the guard's existence proves the lock is held by this
        // context; unlock publishes our node's release through the MCS
        // queue.
        unsafe { &mut *self.lock.get() }.unlock(self.node);
        crate::smp::irq_restore(self.saved_irq);
    }
}

// ---------------------------------------------------------------------------
// BusyCell: the try-enter exclusivity cell.
//
// The devmnt session core's discipline: exactly one outstanding RPC
// owns the state; a concurrent attempt is a LOUD error, never a wedge.  An
// MCS Lock is WRONG here — a lock held across the blocking 9P receive would
// spin-deadlock an IRQ-masked contender (the holder is sched-blocked on the
// pipe; the spinner never yields the CPU).  BusyCell is the non-spinning
// sibling: `enter()` claims the cell or fails immediately.
// ---------------------------------------------------------------------------

/// A cell with try-enter exclusivity (see the module note above): at most
/// one live guard owns the data; contention fails fast instead of spinning.
pub struct BusyCell<T> {
    busy: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: the busy flag is the ownership discipline (exactly one guard owns
// the data at a time; every other enter() gets None) — the same protocol
// shape as Lock, with failure instead of the spin.
unsafe impl<T: Send> Send for BusyCell<T> {}
// SAFETY: as above — &BusyCell sharing is safe because data access only
// ever happens through the one live guard the flag admits.
unsafe impl<T: Send> Sync for BusyCell<T> {}

impl<T> BusyCell<T> {
    pub const fn new(data: T) -> BusyCell<T> {
        BusyCell { busy: AtomicBool::new(false), data: UnsafeCell::new(data) }
    }

    /// Try to take exclusive ownership: Some(guard) when the cell is free,
    /// None when another context holds it (the caller's discipline decides —
    /// devmnt errors loudly).  The guard's Drop releases the cell on ANY
    /// exit path.
    pub fn enter(&self) -> Option<BusyGuard<'_, T>> {
        if self.busy.swap(true, Ordering::AcqRel) {
            return None;
        }
        Some(BusyGuard { cell: self })
    }
}

/// The BusyCell ownership guard (the LockGuard analogue).
pub struct BusyGuard<'a, T> {
    cell: &'a BusyCell<T>,
}

impl<T> Deref for BusyGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this guard's construction is the only live ownership of
        // the cell (enter()'s swap excludes every other contender for its
        // lifetime) — the borrow is exclusive in practice.
        unsafe { &*self.cell.data.get() }
    }
}

impl<T> DerefMut for BusyGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: Deref's invariant — the guard is the cell's single owner.
        unsafe { &mut *self.cell.data.get() }
    }
}

impl<T> Drop for BusyGuard<'_, T> {
    fn drop(&mut self) {
        self.cell.busy.store(false, Ordering::Release);
    }
}

// SAFETY: the io halves are !Send trait objects (the MntIo trait
// can't carry the bound — the unmodified host tests' FakeServer
// shares its fid table by Rc), so the proof lands here: only a
// BusyCell<MntIoBox> guard ever touches a half, and the guard is the
// cell's single owner (enter()'s swap excludes every other contender
// for its lifetime — the read half doubly so, the donor's
// one-reader-at-a-time token).  The BusyCell Send/Sync impls above
// extend the discipline to the session.  (Homed with the primitive
// because devmnt.rs is #[forbid(unsafe_code)].)
unsafe impl Send for crate::devmnt::MntIoBox {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smp::{self, LockRank};

    // -- SMP-07/SMP-12: IrqSpinLock masks IRQs on entry and restores the
    // -- saved state on drop, nesting cleanly (D-25) --

    #[test]
    fn irq_spinlock_nested_save_restore() {
        smp::test_hooks::set_cpu_id(30);
        static L_A: IrqSpinLock<u8> = IrqSpinLock::new("test_irq_a", LockRank::ProcTable, 0);
        static L_B: IrqSpinLock<u8> = IrqSpinLock::new("test_irq_b", LockRank::RunQueue, 0);
        let node_a = LockNode::new();
        let node_b = LockNode::new();
        assert_eq!(smp::test_hooks::irq_depth(), 0);
        {
            let mut g_a = L_A.lock(&node_a);
            assert_eq!(smp::test_hooks::irq_depth(), 1, "lock masks IRQs");
            *g_a = 1;
            {
                let mut g_b = L_B.lock(&node_b);
                assert_eq!(
                    smp::test_hooks::irq_depth(),
                    2,
                    "a nested lock saves the masked state"
                );
                *g_b = 2;
            }
            assert_eq!(smp::test_hooks::irq_depth(), 1, "drop restores the saved state");
            assert_eq!(*g_a, 1);
        }
        assert_eq!(smp::test_hooks::irq_depth(), 0, "the outermost drop unmasks");
        assert_eq!(smp::test_held_lock_count(), 0, "the rank stack is empty");
    }

    // -- SMP-07: try_lock fails fast when held, restoring the saved IRQ
    // -- state and leaking no rank (SMP-12) --

    #[test]
    fn irq_spinlock_try_lock_restores_irq_on_miss() {
        use std::sync::{Arc, Barrier};
        static L: IrqSpinLock<u8> = IrqSpinLock::new("test_irq_try", LockRank::HeapAlloc, 0);
        // The holder takes L on another cpu, so the try below misses on the
        // queue — never on this cpu's rank stack (re-trying a lock at a
        // rank this cpu already holds is itself a violation, D-26).
        let hold = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let hold_h = Arc::clone(&hold);
        let release_h = Arc::clone(&release);
        let holder = std::thread::spawn(move || {
            smp::test_hooks::set_cpu_id(36);
            let node = LockNode::new();
            let g = L.lock(&node);
            hold_h.wait(); // L held
            release_h.wait(); // main has tried and seen the miss
            drop(g);
        });
        smp::test_hooks::set_cpu_id(31);
        // Uncontended fast path on a FREE lock first.
        static L2: IrqSpinLock<u8> = IrqSpinLock::new("test_irq_try_free", LockRank::HeapAlloc, 0);
        let n1 = LockNode::new();
        {
            let g = L2.try_lock(&n1).expect("an uncontended try_lock succeeds");
            assert_eq!(smp::test_hooks::irq_depth(), 1, "try_lock masks IRQs");
            assert_eq!(smp::test_held_lock_count(), 1, "the rank is recorded");
            drop(g);
            assert_eq!(smp::test_hooks::irq_depth(), 0);
            assert_eq!(smp::test_held_lock_count(), 0);
        }
        // The contended try: L is held on the other cpu; this cpu's rank
        // stack is empty, so the rank check passes and the CAS misses.
        hold.wait();
        let n2 = LockNode::new();
        assert!(L.try_lock(&n2).is_none(), "a held lock fails immediately");
        assert_eq!(
            smp::test_hooks::irq_depth(),
            0,
            "a miss restores the saved IRQ state immediately"
        );
        assert_eq!(smp::test_held_lock_count(), 0, "a miss leaks no rank");
        release.wait();
        holder.join().expect("holder thread");
    }

    // -- The plain Lock gains a non-blocking acquire (SMP-11 work
    // -- stealing's victim-queue probe) --

    #[test]
    fn lock_try_lock_returns_none_when_held() {
        smp::test_hooks::set_cpu_id(32);
        static L: Lock<u8> = Lock::new("test_lock_try", 0);
        let n1 = LockNode::new();
        let n2 = LockNode::new();
        let g = L.try_lock(&n1).expect("an uncontended try_lock succeeds");
        assert!(L.try_lock(&n2).is_none(), "a held lock fails immediately");
        drop(g);
        assert!(L.try_lock(&n2).is_some(), "the queue is clean after release");
    }

    // -- SMP-12: ranked Lock acquisitions push/pop the per-cpu rank stack;
    // -- unranked locks never touch it (D-26) --

    #[test]
    #[cfg(debug_assertions)]
    fn lock_rank_hooks_track_ranked_lock_acquisition() {
        smp::test_hooks::set_cpu_id(33);
        static L: Lock<u8> = Lock::new_ranked("test_ranked", LockRank::FidShard, 0);
        let node = LockNode::new();
        {
            let _g = L.lock(&node);
            assert_eq!(smp::test_held_lock_count(), 1, "a ranked lock holds a rank");
        }
        assert_eq!(smp::test_held_lock_count(), 0, "guard drop pops the rank");
        static L2: Lock<u8> = Lock::new("test_unranked", 0);
        let node2 = LockNode::new();
        {
            let _g = L2.lock(&node2);
            assert_eq!(smp::test_held_lock_count(), 0, "an unranked lock holds no rank");
        }
        assert_eq!(smp::test_held_lock_count(), 0);
    }

    // -- SMP-07 (Pitfall 1): a cpu spinning on an IrqSpinLock with IRQs
    // -- masked still services a remote TLB shootdown, so the initiator's
    // -- wait barrier always completes (no deadlock) --

    #[test]
    fn irq_spinlock_shootdown_poll_prevents_deadlock() {
        use std::sync::{Arc, Barrier};
        static L: IrqSpinLock<u8> = IrqSpinLock::new("test_shootdown", LockRank::HeapAlloc, 0);
        smp::test_hooks::set_cpu_id(34);
        smp::test_hooks::set_online_mask((1 << 34) | (1 << 35));
        let start = Arc::new(Barrier::new(2));
        let start_b = Arc::clone(&start);
        let spinner = std::thread::spawn(move || {
            smp::test_hooks::set_cpu_id(35);
            start_b.wait();
            let node = LockNode::new();
            // Spins on L while the main thread holds it; the MCS spin
            // loop's poll_shootdown_if_pending() services the shootdown
            // the holder is waiting on — no deadlock.
            let guard = L.lock(&node);
            let flushes = smp::test_hooks::flush_count();
            drop(guard);
            flushes
        });
        let node = LockNode::new();
        let guard = L.lock(&node);
        start.wait();
        // The spinner is (or is about to be) spinning on L; requesting a
        // shootdown must not deadlock: its spin polls and acks.
        smp::request_shootdown(Some(0x12000));
        drop(guard);
        let remote_flushes = spinner.join().expect("spinner thread");
        assert!(remote_flushes >= 1, "the spinning cpu serviced the shootdown");
    }
}

