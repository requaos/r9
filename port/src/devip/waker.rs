//! devip/waker.rs — the smoltcp socket-waker → r9 wake bridge (D-05;
//! contract §12.2).  smoltcp's sockets wake a `core::task::Waker`; r9 wakes
//! PIDs through the arch block/wake hooks.  This module is the translation
//! point — and the phase's ONE new unsafe site (constructing a `Waker` is
//! irreducibly unsafe: `Waker::from_raw` + the RawWaker vtable).
//!
//! MODULE MOUNT: this file is NOT a submodule of `devip` — it is mounted at
//! the crate root as `crate::devip_waker` via `#[path = "devip/waker.rs"]`
//! (see lib.rs), because the `#[forbid(unsafe_code)]` on the `devip` module
//! propagates into submodules and would make this named exception
//! uncompilable.  Every OTHER devip module carries the forbid.
//!
//! THE BRIDGE CONTRACT (P-C discipline, pinned host-side in
//! devip_tests.rs):
//!
//! - smoltcp wakers are one-per-direction, woken ONCE, spurious-allowed
//!   (0.14 docs, socket pages).  The CONV — not the waker — owns the waiter
//!   set: it arms a per-socket `WakeSlot` with the blocked reader's pid,
//!   registers a fresh waker, and RE-ARMS + RE-REGISTERS after EVERY wake,
//!   re-checking `can_recv`/`can_send` on each wake (spurious-wake
//!   discipline).
//! - `will_wake` (same vtable + same slot pointer) makes a re-registration
//!   for the same slot a cheap no-clone.
//! - Wake AFTER guard drop: the vtable's wake NEVER calls the arch wake
//!   hook directly — it fires mid-`poll` with the seam's STACK cell held,
//!   and the hook takes PROC_TABLE.  It pushes the pid into the seam's
//!   pending-wake set (`stack::note_wake`); `drive_stack` drains the set
//!   after the guard drops (the netif wake-after-drop discipline).
//!
//! The wakee-pid slot invariant (the UNSAFE-AUDIT.md entry's wording): the
//! slot is WRITTEN BEFORE REGISTRATION, READ AT WAKE, and NEVER OUTLIVES
//! the conv.

use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{RawWaker, RawWakerVTable, Waker};

/// A per-socket wakee slot: the pid to wake when the socket's waker fires.
/// The owning conv arms it BEFORE registering the waker built from it;
/// the vtable's wake reads it.  Slots live in conv-table storage (statics)
/// or test statics — the `&'static` bound on `waker_for` is the
/// never-outlives-the-conv invariant made mechanical.
pub struct WakeSlot {
    pid: AtomicU64,
}

impl WakeSlot {
    pub const fn new() -> WakeSlot {
        WakeSlot { pid: AtomicU64::new(0) }
    }

    /// Arm the slot with the pid to wake (0 = nobody).  Release-publishes
    /// BEFORE the registration that exposes it to the wake path.
    pub fn arm(&self, pid: u64) {
        self.pid.store(pid, Ordering::Release);
    }
}

impl Default for WakeSlot {
    fn default() -> WakeSlot {
        WakeSlot::new()
    }
}

/// The vtable's shared wake path: read the slot, and when it names a proc,
/// push the pid into the seam's pending-wake set.  NEVER the wake hook
/// directly (see the module header — wake-after-drop).
fn fire(data: *const ()) -> Option<u64> {
    // SAFETY: the RawWaker contract — `data` names a WakeSlot that is live
    // for the waker's whole registration lifetime (the conv owns both; the
    // slot is armed before registration and the registration is dropped by
    // smoltcp's one-shot wake before the conv can go away).
    let pid = unsafe { &*(data as *const WakeSlot) }.pid.load(Ordering::Acquire);
    if pid != 0 { Some(pid) } else { None }
}

/// # Safety
/// RawWaker vtable fn: `data` always names a live `WakeSlot` (armed by
/// `waker_for`; conv-owned, never freed through the waker).  Rebuilds the
/// waker trivially — the conv owns the registration and re-registers after
/// every wake (one-shot semantics), so clone bumps no refcount.
unsafe fn rw_clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &RW_VTABLE)
}

/// # Safety
/// RawWaker vtable fn: same `data` contract as `rw_clone`.
unsafe fn rw_wake(data: *const ()) {
    if let Some(pid) = fire(data) {
        crate::devip::stack::note_wake(pid);
    }
}

/// # Safety
/// RawWaker vtable fn: same `data` contract as `rw_clone`.  Identical to
/// `rw_wake` — the seam's one-shot discipline lives in the conv's
/// re-registration, not in the vtable.
unsafe fn rw_wake_by_ref(data: *const ()) {
    if let Some(pid) = fire(data) {
        crate::devip::stack::note_wake(pid);
    }
}

/// # Safety
/// RawWaker vtable fn: a no-op — WakeSlots are conv-owned and are never
/// allocated or freed through the waker.
unsafe fn rw_drop(_data: *const ()) {}

// RawWakerVTable::new is a SAFE const fn on this toolchain (nightly-2026-01-30)
// — the unsafe contract lives in the four vtable fns above and in
// Waker::from_raw below, each carrying its SAFETY comment.
static RW_VTABLE: RawWakerVTable =
    RawWakerVTable::new(rw_clone, rw_wake, rw_wake_by_ref, rw_drop);

/// Build a `Waker` that wakes the pid recorded in `slot` (via the seam's
/// pending-wake set).  Wakers are recreated per registration, never
/// shared — the conv re-arms the slot and rebuilds after EVERY wake.
pub fn waker_for(slot: &'static WakeSlot) -> Waker {
    let raw = RawWaker::new(slot as *const WakeSlot as *const (), &RW_VTABLE);
    // SAFETY: RW_VTABLE's contract holds for every Waker built here: the
    // slot outlives the registration (conv-owned; &'static), wake reads a
    // live slot, clone/drop carry no ownership.
    unsafe { Waker::from_raw(raw) }
}
