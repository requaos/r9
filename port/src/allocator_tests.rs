//! The allocator's test module: homed at the CRATE root so
//! `port::allocator` can carry #[forbid(unsafe_code)] — the pin tests'
//! arena constructions keep their unsafe blocks.  The pin
//! tests' BODIES are byte-verbatim from the pre-redesign module (the
//! zero-behavior-change proof); the later additions pin the index/arena
//! model's tripwire / exhaustion / realloc-stress behavior.

use crate::allocator::{Block, BumpAlloc, QuickFit};
use core::alloc::Layout;

/// Regression: the > MAX_QUICK_SIZE, non-power-of-two TAIL path
/// (devmnt's ~250 KB exec staging buffer tripped it).  Pre-fix the
/// BumpAlloc arg order ran `align_offset` on the SIZE ("align is not
/// a power-of-two" panic), and free_misc's fresh-header arm panicked
/// whenever the header malloc SUCCEEDED ("allocated header block").
#[test]
fn tail_alloc_big_nonpow2_free_and_realloc() {
    let mut arena = alloc::vec![0u8; 1024 * 1024];
    let start = arena.as_mut_ptr();
    let len = arena.len();
    let tail = BumpAlloc::new(unsafe { Block::new_from_raw_parts(start, len) });
    let mut qf = QuickFit::new(tail);
    let layout = Layout::from_size_align(200_000, 1).unwrap();
    let p = qf.malloc(layout);
    assert!(!p.is_null(), "a 200 KB non-pow2 tail allocation succeeds");
    unsafe { p.write_bytes(0xA5, 200_000) };
    qf.free(p, layout);
    let p2 = qf.malloc(layout);
    assert!(!p2.is_null(), "the freed tail block is reallocatable");
    unsafe { p2.write_bytes(0x5A, 200_000) };
    qf.free(p2, layout);
    // And a same-size triple cycle (the Vec-growth realloc pattern:
    // alloc bigger, free smaller, repeat).
    let small = Layout::from_size_align(100_000, 1).unwrap();
    let big = Layout::from_size_align(300_000, 1).unwrap();
    let a = qf.malloc(small);
    assert!(!a.is_null());
    let b = qf.malloc(big);
    assert!(!b.is_null());
    qf.free(a, small);
    qf.free(b, big);
}

/// Regression (the gate wedge's root cause):
/// unlink's mid-list removal returned the removed node as the list
/// head — the misc list then held a still-allocated header (a later
/// insert cycled it, and the next first-fit walk spun forever — the
/// restart-exec wedge at the ~130 KB staging growth), and a no-match
/// walk wiped the whole list (a permanent leak).  Pin both arms by
/// POINTER identity: blocks must be recycled from the free list,
/// never tail-bumped past a wipe.  (Pointer identity is
/// preserved in the index model — the materialized pointer is the
/// arena base + the recycled block's offset.)
#[test]
fn misc_midlist_removal_preserves_the_list() {
    let mut arena = alloc::vec![0u8; 1024 * 1024];
    let start = arena.as_mut_ptr();
    let len = arena.len();
    let tail = BumpAlloc::new(unsafe { Block::new_from_raw_parts(start, len) });
    let mut qf = QuickFit::new(tail);

    // Three misc blocks (all > MAX_QUICK_SIZE so they ride misc).
    let la = Layout::from_size_align(20_000, 1).unwrap();
    let lb = Layout::from_size_align(40_000, 1).unwrap();
    let lc = Layout::from_size_align(80_000, 1).unwrap();
    let a = qf.malloc(la);
    let b = qf.malloc(lb);
    let c = qf.malloc(lc);
    assert!(!a.is_null() && !b.is_null() && !c.is_null());

    // Arm 1 — the no-match wipe.  Free big→small → the LIFO misc
    // list is [a(20 000), b(40 000), c(80 000)].  A no-fit request
    // must leave the list untouched: the follow-on allocations come
    // BACK from the same blocks (the old shape wiped the list and
    // tail-bumped fresh memory instead — a ≠ a2 pointer split).
    qf.free(c, lc);
    qf.free(b, lb);
    qf.free(a, la);
    let huge = Layout::from_size_align(900_000, 1).unwrap();
    let h = qf.malloc(huge);
    assert!(!h.is_null(), "the no-fit request bumps the tail");
    let a2 = qf.malloc(la);
    let b2 = qf.malloc(lb);
    let c2 = qf.malloc(lc);
    assert_eq!(a2, a, "no-fit preserved the list: la reuses a's block");
    assert_eq!(b2, b, "no-fit preserved the list: lb reuses b's block");
    assert_eq!(c2, c, "no-fit preserved the list: lc reuses c's block");
    qf.free(a2, la);
    qf.free(b2, lb);
    qf.free(c2, lc);

    // Arm 2 — a two-deep first-fit removal.  Re-take the three
    // blocks (they come back from the misc list — pinned by
    // pointer), then free in the order big, small, mid so the LIFO
    // list is [q(40 000), r(20 000), p(80 000)]; a 60 000 request
    // skips the head AND the middle, matching the 80 000 tail node.
    // The skipped nodes must survive (the old shape returned the
    // removed node as the head — the survivors were orphaned, and
    // the removed node's header went back on the free list while
    // still hashed: the next insert cycled it).
    let p = qf.malloc(lc);
    let q = qf.malloc(lb);
    let r = qf.malloc(la);
    assert_eq!(p, c, "the recycled 80 000 block");
    assert_eq!(q, b, "the recycled 40 000 block");
    assert_eq!(r, a, "the recycled 20 000 block");
    qf.free(p, lc); // big first
    qf.free(r, la); // small
    qf.free(q, lb); // mid last → head: [q(40 000), r(20 000), p(80 000)]
    let big = Layout::from_size_align(60_000, 1).unwrap();
    let m = qf.malloc(big);
    assert_eq!(m, p, "two-deep first-fit removal matches the 80 000 node");
    let s = qf.malloc(la);
    assert_eq!(s, q, "the skipped head survived the removal");
    let t = qf.malloc(la);
    assert_eq!(t, r, "the skipped middle node survived the removal");
}

/// The tripwire (the safe-model equivalent): a cycled
/// index chain must panic LOUDLY (never the infinite-spin wedge).
/// The cfg(test) hook splices the cycle with pure index writes; a no-fit
/// first-fit request walks the cycle into the tripwire.
#[test]
#[should_panic(expected = "allocator: free-list cycle detected")]
fn free_list_cycle_tripwire_panics_loudly() {
    let mut arena = alloc::vec![0u8; 1024 * 1024];
    let start = arena.as_mut_ptr();
    let len = arena.len();
    let tail = BumpAlloc::new(unsafe { Block::new_from_raw_parts(start, len) });
    let mut qf = QuickFit::new(tail);
    // Two misc blocks on the free list (a two-node chain to splice).
    let la = Layout::from_size_align(20_000, 1).unwrap();
    let lb = Layout::from_size_align(40_000, 1).unwrap();
    let a = qf.malloc(la);
    let b = qf.malloc(lb);
    assert!(!a.is_null() && !b.is_null());
    qf.free(a, la);
    qf.free(b, lb);
    qf.test_hook_cycle_misc();
    // A request that fits NEITHER free block walks the (now cycled) misc
    // list to its end — the tripwire turns the infinite spin into the
    // loud panic.
    let huge = Layout::from_size_align(900_000, 1).unwrap();
    let _ = qf.malloc(huge);
}

/// Header-table exhaustion fails LOUDLY (the named
/// panic), never wraps, never corrupts.  More frees than slots (main +
/// reserve) trip it — each free of a fresh tail block consumes a slot.
#[test]
#[should_panic(expected = "allocator: header table exhausted")]
fn header_table_exhaustion_panics_loudly() {
    // The arena must hold (HEADER_CAP + RESERVE_CAP + 1) min-size blocks:
    // 5121 × 64 B = 328 KiB < 1 MiB.
    let mut arena = alloc::vec![0u8; 1024 * 1024];
    let start = arena.as_mut_ptr();
    let len = arena.len();
    let tail = BumpAlloc::new(unsafe { Block::new_from_raw_parts(start, len) });
    let mut qf = QuickFit::new(tail);
    let layout = Layout::from_size_align(64, 64).unwrap();
    let mut ptrs = alloc::vec::Vec::new();
    for _ in 0..5200 {
        let p = qf.malloc(layout);
        assert!(!p.is_null());
        ptrs.push(p);
    }
    // The (4096 + 1024 + 1)th free exhausts both regions and panics.
    for p in ptrs {
        qf.free(p, layout);
    }
}

/// The side-table model's stress proof: the Vec-doubling
/// growth pattern from the staging-buffer regression — repeated
/// grow-by-realloc with content preservation (the copy-then-free order)
/// plus interleaved free/realloc churn across the quick/misc boundary.
#[test]
fn realloc_growth_stress_preserves_contents() {
    let mut arena = alloc::vec![0u8; 1024 * 1024];
    let start = arena.as_mut_ptr();
    let len = arena.len();
    let tail = BumpAlloc::new(unsafe { Block::new_from_raw_parts(start, len) });
    let mut qf = QuickFit::new(tail);

    // Vec-doubling: 64 → 128 → … → 64 KiB, doubling via realloc, with
    // the written pattern verified after every growth (the copy must
    // precede the free or the source bytes die).  The fill byte
    // alternates between iterations (0x5A / 0xA5).
    let mut layout = Layout::from_size_align(64, 64).unwrap();
    let mut p = qf.malloc(layout);
    assert!(!p.is_null());
    let mut n = 64usize;
    let mut fill = 0x5Au8;
    unsafe { p.write_bytes(fill, n) };
    while n < 65536 {
        let new_layout = Layout::from_size_align(n * 2, 64).unwrap();
        let np = unsafe { qf.realloc(p, layout, n * 2) };
        assert!(!np.is_null(), "realloc to {} succeeds", n * 2);
        // The first n bytes survived the copy-then-free.
        unsafe {
            for i in 0..n {
                assert_eq!(np.add(i).read(), fill, "byte {} survived realloc to {}", i, n * 2);
            }
            fill = if fill == 0x5A { 0xA5 } else { 0x5A };
            np.write_bytes(fill, n * 2);
        }
        p = np;
        layout = new_layout;
        n *= 2;
    }
    qf.free(p, layout);

    // Interleaved churn across the quick/misc boundary (the
    // staging-buffer sizes): alloc bigger, free smaller, repeat.
    let small = Layout::from_size_align(100_000, 1).unwrap();
    let big = Layout::from_size_align(300_000, 1).unwrap();
    for _ in 0..8 {
        let a = qf.malloc(small);
        assert!(!a.is_null());
        let b = qf.malloc(big);
        assert!(!b.is_null());
        qf.free(a, small);
        qf.free(b, big);
    }
}

/// 17-02 (the churn gate's measured finding): sustained fork+exec-style
/// allocation ladders — a Vec<Record>'s doubling rungs plus per-page
/// read temps, all freed by iteration end — must reach a steady pool,
/// not strand one rung per class per iteration.  The kernel gate
/// measured ~4 parked blocks per exec cycle (live=0) until the header
/// side table exhausted; this is the host repro of that shape.
#[test]
fn churn_ladder_reaches_steady_pool() {
    let mut arena = alloc::vec![0u8; 1024 * 1024];
    let start = arena.as_mut_ptr();
    let len = arena.len();
    let tail = BumpAlloc::new(unsafe { Block::new_from_raw_parts(start, len) });
    let mut qf = QuickFit::new(tail);

    let rung = |n: usize| Layout::from_size_align(n * 40, 8).unwrap();
    let read_temp = Layout::from_size_align(4096, 1).unwrap();
    let mut final_cap = rung(4);
    let run_one_cycle = |qf: &mut QuickFit, final_cap: &mut Layout| {
        // The Vec-growth ladder: alloc the next rung, free the previous.
        let mut cur = qf.malloc(*final_cap);
        assert!(!cur.is_null());
        for &records in &[8usize, 16, 32, 64, 128] {
            let next = qf.malloc(rung(records));
            assert!(!next.is_null());
            qf.free(cur, *final_cap);
            cur = next;
            *final_cap = rung(records);
        }
        // Per-page read temps (blocksize read, freed each page).
        for _ in 0..6 {
            let t = qf.malloc(read_temp);
            assert!(!t.is_null());
            qf.free(t, read_temp);
        }
        // Cycle end: the final buffer dies.
        qf.free(cur, *final_cap);
    };

    // Warm 5 cycles, snapshot the census; 50 more; the pooled classes
    // must NOT keep climbing (live must be zero throughout).
    for _ in 0..5 {
        run_one_cycle(&mut qf, &mut final_cap);
    }
    let (live_a, pooled_a) = qf.split_census();
    let pooled_bytes_a: usize = pooled_a.iter().map(|&(s, n)| s * n).sum();
    assert!(live_a.is_empty(), "all cycle allocations are freed: live={live_a:?}");
    for _ in 0..50 {
        run_one_cycle(&mut qf, &mut final_cap);
    }
    let (live_b, pooled_b) = qf.split_census();
    let pooled_bytes_b: usize = pooled_b.iter().map(|&(s, n)| s * n).sum();
    assert!(live_b.is_empty(), "live must stay zero: {live_b:?}");
    assert!(
        pooled_bytes_b <= pooled_bytes_a + 4096,
        "the pool must reach steady state, not strand rungs per cycle: {pooled_bytes_a} -> {pooled_bytes_b} ({pooled_b:?})"
    );
}
