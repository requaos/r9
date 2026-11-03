//! fdgrp — the per-process-group fd table (05-06, D-03's RFFDG/RFCFDG
//! semantics; the donor's Fgrp, 9k pgrp.c dupfgrp/closefgrp).
//!
//! The fd table is a fixed array of chan slots behind the mcslock idiom
//! (`Arc<Lock<FdGroup>>` on the Proc — the pgrp/env_group field shape).
//! rfork's fd-group semantics follow fork(2) + 9k sysproc.c (the 05-04
//! RFNAMEG / 05-05 RFENVG precedent — the donor governs):
//!
//!   - NEITHER RFFDG nor RFCFDG: the child SHARES the parent's table
//!     (the Arc clone — a child's close is visible to the parent).
//!   - RFFDG: the child gets a fresh table COPIED slot-by-slot
//!     (dupfgrp — pipe chans clone their Arc, so both tables' pipe fds
//!     reference the SAME shared queue; slot identity is distinct).
//!   - RFCFDG: the child gets a fresh EMPTY table (dupfgrp(nil)).
//!
//! (The 05-06 plan's bullets invert RFFDG vs default — fork(2):
//! "RFFDG: the invoker's file descriptor table is COPIED; otherwise the
//! two processes share a single table."  This is the phase's third
//! plan-text flag inversion; the donor governs.  libc fork() =
//! rfork(RFPROC|RFFDG|RFREND) = COPY, which is exactly what
//! connectembed's dup/close choreography requires.)
//!
//! Slot ops use the CHECKED-OUT-CHAN pattern (the donor's fdtochan
//! incref + fgrp unlock): read/write/stat TAKE the chan out of its slot
//! (`take`) and put it back after the op (`store`) — a blocking pipe op
//! never holds the group lock, so a sibling in a shared group can never
//! deadlock behind it.  `ref_clone` is the reference-creating dup (the
//! pipe end's open count bumps — dat.rs's dup rule); `take`/`store` are
//! move semantics and never touch open counts.
//!
//! RED SKELETON (05-06 Task 1 RED): the copy/ref operations are stubbed
//! (deep_copy returns EMPTY) — the tests pin the donor semantics first.

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::Chan;

/// The fd group: a fixed array of chan slots (None = free).
pub struct FdGroup<const N: usize> {
    pub fds: [Option<Chan>; N],
}

impl<const N: usize> FdGroup<N> {
    /// A fresh, empty table (the RFCFDG arm; Proc construction).
    pub fn new() -> FdGroup<N> {
        FdGroup { fds: core::array::from_fn(|_| None) }
    }

    /// The chan at `fd` (borrowed; None for out-of-range/closed).
    pub fn get(&self, fd: u64) -> Option<&Chan> {
        self.fds.get(fd as usize)?.as_ref()
    }

    /// Take the chan OUT of its slot (move semantics — the checked-out
    /// pattern's removal half; sys_close's pop).  None for
    /// out-of-range/closed.
    pub fn take(&mut self, fd: u64) -> Option<Chan> {
        self.fds.get_mut(fd as usize)?.take()
    }

    /// Put a checked-out chan BACK after its op (the pattern's return
    /// half).  Returns the EVICTED chan if the slot was meanwhile
    /// occupied (a shared-group interleave — impossible this era because
    /// the only blocking op (pipe) has no shared-table caller; the
    /// caller close-dispatches the evictee defensively).
    pub fn store(&mut self, fd: u64, c: Chan) -> Option<Chan> {
        self.fds.get_mut(fd as usize)?.replace(c)
    }

    /// A REFERENCE clone of the chan at `fd` (Chan::dup — bumps an open
    /// pipe end's count; the donor's fdtochan incref).  sys_dup,
    /// rfork's fd copy and devsrv's store ride this.
    ///
    /// RED SKELETON: stubbed to None (the real clone lands in GREEN).
    pub fn ref_clone(&self, fd: u64) -> Option<Chan> {
        let _ = fd;
        None
    }

    /// Install `c` at the lowest free slot; None when the table is full
    /// (the caller names the error: "open: no free fd" etc. — loud,
    /// never a silent -1).
    pub fn alloc_lowest(&mut self, c: Chan) -> Option<u64> {
        let slot = self.fds.iter().position(|s| s.is_none())?;
        self.fds[slot] = Some(c);
        Some(slot as u64)
    }

    /// Two free slots at once (sys_pipe's pair — donor newfd2): find
    /// BOTH before installing anything (a one-slot table must fail the
    /// pipe without consuming an fd).
    pub fn free_pair(&self) -> Option<(usize, usize)> {
        let first = self.fds.iter().position(|s| s.is_none())?;
        let second = self.fds.iter().skip(first + 1).position(|s| s.is_none())? + first + 1;
        Some((first, second))
    }

    /// The RFFDG arm (donor dupfgrp): a fresh table, each occupied slot
    /// dup'd (pipe chans clone their Arc — BOTH tables' pipe fds
    /// reference the SAME shared queue; slot identity is distinct).
    ///
    /// RED SKELETON: returns an EMPTY table (wrong — GREEN lands the
    /// slot-by-slot dup); the fd-group matrix test pins the semantics.
    pub fn deep_copy(&self) -> FdGroup<N> {
        FdGroup::new()
    }

    /// The open slots as (fd, mode) pairs (devdup's directory listing).
    pub fn list_open(&self) -> Vec<(u64, u32)> {
        let mut out = Vec::new();
        for (i, slot) in self.fds.iter().enumerate() {
            if let Some(c) = slot {
                out.push((i as u64, c.mode));
            }
        }
        out
    }
}

impl<const N: usize> Default for FdGroup<N> {
    fn default() -> FdGroup<N> {
        FdGroup::new()
    }
}
