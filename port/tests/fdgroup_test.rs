//! Host-side tests for the 05-06 fd-group semantics (D-03's
//! RFFDG/RFCFDG arms): the per-process fd table as a shareable/copyable
//! group object, and its interplay with devpipe's Arc-shared queue.
//!
//! DEVIATION RECORDED (05-06 Task 1) — the plan's <behavior> bullet says
//! "rfork default — parent and child fd tables hold distinct slots
//! referencing the SAME pipe queue; RFCFDG — child gets an empty table;
//! RFFDG — child shares the parent's table object".  That is INVERTED
//! relative to fork(2) (sys/man/2/fork: "RFFDG: the invoker's file
//! descriptor table is COPIED; otherwise the two processes SHARE a single
//! table"), 9k's sysproc.c sysrfork (RFPROC branch: RFFDG → dupfgrp COPY;
//! RFCFDG → dupfgrp(nil) EMPTY; NEITHER → incref SHARE), and libc's
//! fork() = rfork(RFPROC|RFFDG|RFREND) — connectembed's child
//! dup/close choreography is only safe if fork() COPIES.  This is the
//! phase's third plan-text flag inversion (05-04 RFNAMEG/RFCNAMEG, 05-05
//! RFENVG/RFCENVG); the donor governs.  These tests pin the DONOR
//! semantics: NEITHER = SHARE, RFFDG = COPY (distinct slots, shared pipe
//! queue), RFCFDG = EMPTY.
//!
//! The flag→branch wiring lives in aarch64/src/proc.rs (the arch crate —
//! not host-testable); these tests pin the PORT-crate primitives the
//! wiring mirrors (the 05-05 devenv pattern), and the pipe-queue
//! interplay that Pitfall 4 makes load-bearing.

use port::dat::{Chan, Qid, DEV_CONS, ORDWR};
use port::devpipe;
use port::fdgrp::FdGroup;
use port::namec;

/// The proc.rs table size (MAX_FDS = 32 — 05-06 Pitfall 3 sizing: init's
/// closefds closes 3..29, so the table must hold fds through 29).
const N: usize = 32;

/// A plain (non-pipe) chan for slot mechanics.
fn cons_chan() -> Chan {
    Chan::new(DEV_CONS, Qid { path: 1, vers: 0, typ: 0 }, ORDWR)
}

/// An open pipe end (end 0) with its write peer returned alongside.
fn open_pipe_end() -> (Chan, Chan) {
    let dir = devpipe::attach();
    let c0 = devpipe::walk(&dir, b"data").expect("walk data");
    let c1 = devpipe::walk(&dir, b"data1").expect("walk data1");
    let e0 = namec::open_chan(&c0, ORDWR).expect("open data");
    let e1 = namec::open_chan(&c1, ORDWR).expect("open data1");
    (e0, e1)
}

/// Slot mechanics: install at the lowest free slot, pairwise allocation
/// (sys_pipe's two-at-once), take/store round-trip, and the loud
/// exhaustion path (a full table allocs None — never a silent -1).
#[test]
fn fd_group_slot_mechanics() {
    let mut g = FdGroup::<N>::new();
    assert_eq!(g.alloc_lowest(cons_chan()), Some(0));
    assert_eq!(g.alloc_lowest(cons_chan()), Some(1));
    assert_eq!(g.alloc_lowest(cons_chan()), Some(2));

    // take removes; store puts back; get borrows.
    let taken = g.take(1).expect("slot 1 occupied");
    assert_eq!(taken.dev, DEV_CONS);
    assert!(g.get(1).is_none(), "a taken slot reads empty");
    assert!(g.store(1, taken).is_none(), "store into an empty slot evicts nothing");
    assert!(g.get(1).is_some(), "the stored chan is back");

    // alloc_lowest reuses the lowest free slot (dup(-1) semantics).
    g.take(1);
    assert_eq!(g.alloc_lowest(cons_chan()), Some(1));

    // free_pair finds two free slots (skipping occupied ones).
    g.take(0);
    assert_eq!(g.free_pair(), Some((0, 3)));

    // Exhaustion: fill the table, then alloc_lowest/free_pair fail.
    let mut full = FdGroup::<N>::new();
    for i in 0..N {
        assert_eq!(full.alloc_lowest(cons_chan()), Some(i as u64));
    }
    assert_eq!(full.alloc_lowest(cons_chan()), None, "a full table has no free fd");
    assert_eq!(full.free_pair(), None);
}

/// The rfork fd-group matrix (DONOR semantics — see the header):
/// NEITHER shares the parent's table OBJECT (a child's install is visible
/// via the parent's handle); RFFDG deep-copies (distinct slots, same
/// qids); RFCFDG empties.
#[test]
fn fd_group_matrix_share_copy_empty() {
    let mut parent = FdGroup::<N>::new();
    parent.alloc_lowest(cons_chan());
    parent.alloc_lowest(cons_chan());

    // NEITHER flag: the child SHARES the parent's handle (the proc.rs
    // wiring Arc-clones the group).  Modeled here as shared access to the
    // same object: an install is visible to both.
    {
        let shared = &mut parent;
        assert_eq!(shared.alloc_lowest(cons_chan()), Some(2));
    }
    assert!(parent.get(2).is_some(), "SHARE: the child's install is visible to the parent");

    // RFFDG: the child gets a fresh COPY — distinct slots, same contents.
    let mut child = parent.deep_copy();
    assert!(child.get(0).is_some() && child.get(2).is_some(), "COPY: slots survive");
    let evicted = child.take(0);
    assert!(evicted.is_some(), "COPY: the child's slot held a chan");
    assert!(parent.get(0).is_some(), "COPY: the child's close never touches the parent");
    assert!(child.get(0).is_none(), "COPY: the child's slot is closed");

    // RFCFDG: the child starts EMPTY.
    let fresh = FdGroup::<N>::new();
    assert!(fresh.get(0).is_none() && fresh.list_open().is_empty(), "RFCFDG: empty table");
}

/// Pitfall 4's fd-table half (DONOR RFFDG = COPY): a deep-copied table
/// holds DISTINCT slots whose pipe chans reference the SAME Arc-shared
/// queue — bytes written through the CHILD's copy are read from the
/// PARENT's table, and the child's close does not EOF the parent's end
/// (the copy bumped the open-end count — the donor's incref shape).
#[test]
fn fd_group_copy_shares_the_pipe_queue() {
    let (e0, e1) = open_pipe_end();
    let mut parent = FdGroup::<N>::new();
    assert_eq!(parent.alloc_lowest(e0), Some(0)); // fd 0 (a pipe write end)
    assert_eq!(parent.alloc_lowest(e1), Some(1)); // fd 1 (the read end)

    // The RFFDG arm: the child table is a slot-by-slot copy.
    let mut child = parent.deep_copy();

    // Bytes written through the CHILD's copy of end 0 cross to the
    // PARENT's end 1 — the queue is shared, the slots are not.
    let mut child_w = child.take(0).expect("child holds the write end");
    devpipe::write(&mut child_w, b"cross", u64::MAX).expect("write through the child's copy");
    child.store(0, child_w);

    let mut parent_r = parent.take(1).expect("parent holds the read end");
    let mut buf = [0u8; 8];
    let n = devpipe::read(&mut parent_r, &mut buf, u64::MAX).expect("read from the parent's copy");
    assert_eq!(&buf[..n as usize], b"cross", "the queue is shared across the copy boundary");
    parent.store(1, parent_r);

    // The child's close decrements the open count but does NOT EOF the
    // parent (the parent's copy keeps the write end open).
    let closed = child.take(0).expect("child still holds the write end");
    namec::close_chan(&closed);
    let mut parent_w = parent.take(0).expect("parent still holds the write end");
    devpipe::write(&mut parent_w, b"alive", u64::MAX)
        .expect("the parent's end survives the child's close (the Arc refcount)");
    parent.store(0, parent_w);
    let mut parent_r = parent.take(1).unwrap();
    let n = devpipe::read(&mut parent_r, &mut buf, u64::MAX).expect("read");
    assert_eq!(&buf[..n as usize], b"alive");
    parent.store(1, parent_r);
}

/// 261104-b4p (item 4 — the donor pexit closefgrp's table half):
/// drain_all takes EVERY occupied slot (leaving Nones) and returns the
/// chans in slot order, leaving a reusable empty group — exit_tail's
/// sweep shape (close dispatch happens on the drained Vec AFTER the
/// group lock drops, never under it).
#[test]
fn fd_group_drain_all_empties_every_slot() {
    let mut g = FdGroup::<N>::new();
    assert_eq!(g.alloc_lowest(cons_chan()), Some(0));
    assert_eq!(g.alloc_lowest(cons_chan()), Some(1));
    assert_eq!(g.alloc_lowest(cons_chan()), Some(2));
    g.take(1); // leave a hole — slots 0 and 2 occupied

    let drained = g.drain_all();
    assert_eq!(drained.len(), 2, "every occupied slot drains (order pinned: slot order)");
    assert!(drained.iter().all(|c| c.dev == DEV_CONS), "the slot chans are returned");
    assert!(g.get(0).is_none() && g.get(1).is_none() && g.get(2).is_none(), "the group is left empty");
    assert!(g.list_open().is_empty(), "no open slots remain");

    // The drained group is reusable (the Moribund proc holds it — a fresh
    // alloc lands at slot 0).
    assert_eq!(g.alloc_lowest(cons_chan()), Some(0), "the drained group is reusable");
}
