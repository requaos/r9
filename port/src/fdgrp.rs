//! fdgrp — the per-process-group fd table (the RFFDG/RFCFDG
//! semantics; the donor's Fgrp, 9k pgrp.c dupfgrp/closefgrp).
//!
//! The fd table is a fixed array of chan slots behind the mcslock idiom
//! (`Arc<Lock<FdGroup>>` on the Proc — the pgrp/env_group field shape).
//! rfork's fd-group semantics follow fork(2) + 9k sysproc.c (the donor
//! governs):
//!
//!   - NEITHER RFFDG nor RFCFDG: the child SHARES the parent's table
//!     (the Arc clone — a child's close is visible to the parent).
//!   - RFFDG: the child gets a fresh table COPIED slot-by-slot
//!     (dupfgrp — pipe chans clone their Arc, so both tables' pipe fds
//!     reference the SAME shared queue; slot identity is distinct).
//!   - RFCFDG: the child gets a fresh EMPTY table (dupfgrp(nil)).
//!
//! (fork(2):
//! "RFFDG: the invoker's file descriptor table is COPIED; otherwise the
//! two processes share a single table."  libc fork() =
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

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::Chan;

/// The fd group: N chan slots (None = free), HEAP-BACKED by a Vec — an
/// inline `[Option<Chan>; 32]` materialized ~5.7 KB on the kernel stack
/// inside Proc::new (the rfork → new_user_proc →
/// Proc::new chain on a 16 KiB kstack overflowed into the proc's
/// root-table page below it — the live stack then read as UART-register
/// garbage mid-print; a Vec's buffer is heap from the start).
pub struct FdGroup<const N: usize> {
    pub fds: Vec<Option<Chan>>,
}

impl<const N: usize> FdGroup<N> {
    /// A fresh, empty table (the RFCFDG arm; Proc construction).  The
    /// slot array is heap-allocated and filled in place (never a giant
    /// stack temporary — see the type's doc comment).
    pub fn new() -> FdGroup<N> {
        let mut fds = Vec::with_capacity(N);
        fds.resize_with(N, || None);
        FdGroup { fds }
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
    pub fn ref_clone(&self, fd: u64) -> Option<Chan> {
        self.get(fd).map(|c| c.dup())
    }

    /// The transient op-reference pair for blocking-capable syscalls
    /// (pread/pwrite — the donor's fdtochan incref + fgrp-unlock): the
    /// slot KEEPS its chan the whole time and the op rides a clone with
    /// NO open-count effects (Chan::op_clone — not an open).  The
    /// take/store pattern emptied the slot for the op's duration, so a
    /// shared-group sibling's concurrent op on the SAME fd saw a closed
    /// fd ("pwrite: bad fd") — the libthread shape (aan's blocked reader
    /// + writer procs ride one netfd) is exactly that interleave.
    pub fn op_ref(&self, fd: u64) -> Option<Chan> {
        self.get(fd).map(|c| c.op_clone())
    }

    /// The op-ref's return half: merge the (possibly advanced) streaming
    /// offset back into the slot when it's still occupied; a slot
    /// emptied mid-op (a close raced us — its close-dispatch already
    /// ran) just releases the clone.  The release runs devmnt's
    /// clunk_if_last (the donor's decref-fires-the-check shape) via
    /// Chan::drop_op_ref.
    pub fn op_done(&mut self, fd: u64, c: Chan) {
        if let Some(slot) = self.fds.get_mut(fd as usize).and_then(|s| s.as_mut()) {
            slot.offset = c.offset;
        }
        c.drop_op_ref();
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
    pub fn deep_copy(&self) -> FdGroup<N> {
        let mut to = FdGroup::new();
        for (i, slot) in self.fds.iter().enumerate() {
            if let Some(c) = slot {
                to.fds[i] = Some(c.dup());
            }
        }
        to
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

    /// The donor pexit closefgrp's table half: take
    /// EVERY occupied slot (leaving Nones) and return the chans in slot
    /// order; the group is left empty and reusable.  exit_tail drains the
    /// dying proc's group this way and close-dispatches each chan AFTER
    /// the group lock drops (never under it — the fdgrp leaf-lock rule).
    pub fn drain_all(&mut self) -> Vec<Chan> {
        let mut out = Vec::new();
        for slot in self.fds.iter_mut() {
            if let Some(c) = slot.take() {
                out.push(c);
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
