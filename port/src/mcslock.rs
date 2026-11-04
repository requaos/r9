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
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

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
                hint::spin_loop();
            }
        }
        node
    }

    pub fn unlock(&self, node: &LockNode) {
        if node.next.load(Ordering::Acquire).is_null() {
            let p = node as *const _ as *mut _;
            if self
                .queue
                .compare_exchange_weak(p, ptr::null_mut(), Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
            while node.next.load(Ordering::Acquire).is_null() {
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

pub struct Lock<T: ?Sized> {
    lock: UnsafeCell<MCSLock>,
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
        Lock { lock: UnsafeCell::new(MCSLock::new(name)), data: UnsafeCell::new(data) }
    }

    pub fn lock<'a>(&'a self, node: &'a LockNode) -> LockGuard<'a, T> {
        // SAFETY: UnsafeCell::get on the lock is the protocol entry (the
        // MCS queue is internally synchronized); the guard's &mut data is
        // unique because the lock was just acquired.
        let node = unsafe { &mut *self.lock.get() }.lock(node);
        // SAFETY: the lock is held (above), so the data borrow is
        // exclusive for the guard's lifetime; the guard's Drop releases.
        LockGuard { lock: &self.lock, node, data: unsafe { &mut *self.data.get() } }
    }
}

pub struct LockGuard<'a, T: ?Sized + 'a> {
    lock: &'a UnsafeCell<MCSLock>,
    node: &'a LockNode,
    data: &'a mut T,
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
        // SAFETY: the guard's existence proves the lock is held by this
        // context; unlock publishes our node's release through the MCS
        // queue.
        unsafe { &mut *self.lock.get() }.unlock(self.node);
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

// SAFETY: MntCore's io transport is a !Send trait object (the MntIo trait
// can't carry the bound — the unmodified host tests' FakeServer shares its
// fid table by Rc), so the proof lands here: only a BusyCell<MntCore>
// guard ever touches an MntCore, and the guard is the cell's single owner
// (enter()'s swap excludes every other contender for its lifetime).  The
// BusyCell Send/Sync impls above extend the discipline to the session.
// (Homed with the primitive because devmnt.rs is
// #[forbid(unsafe_code)].)
unsafe impl Send for crate::devmnt::MntCore {}
