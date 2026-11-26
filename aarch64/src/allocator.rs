#[cfg(not(test))]
mod global {
    use core::cell::SyncUnsafeCell;
    use core::mem;
    use port::allocator::{
        Block, BumpAlloc, QuickFit, global::GlobalHeap, global::GlobalQuickAlloc,
    };

    // The heap arena and the QuickFit state live in
    // SyncUnsafeCell statics (never `static mut`).  GlobalQuickAlloc's
    // IrqSpinLock (port::globalalloc, LockRank::HeapAlloc) is the sole
    // synchronized access path; the cells' pointers are published into it
    // here at static-init time, before any allocation can run (SMP-03).
    static HEAP: SyncUnsafeCell<GlobalHeap> = SyncUnsafeCell::new(GlobalHeap::new());
    static ALLOC: SyncUnsafeCell<QuickFit> = SyncUnsafeCell::new(QuickFit::new(
        // SAFETY: HEAP is a static 4 MiB-aligned arena owned exclusively by
        // this allocator; the Block covers it exactly once, for the
        // arena's lifetime (statics never move).
        BumpAlloc::new(unsafe {
            Block::new_from_raw_parts(HEAP.get().cast(), mem::size_of::<GlobalHeap>())
        }),
    ));

    #[global_allocator]
    static GLOBAL_ALLOCATOR: GlobalQuickAlloc = GlobalQuickAlloc::new(ALLOC.get());

    /// Read-only heap-arena stats off the live global allocator (the
    /// x86_64 15-03 probe, ported for the 20-09 churn gate): (bump-cursor
    /// bytes, arena size).  Serialized under the heap's IrqSpinLock and
    /// allocating nothing (SMP-03).  Only the churn driver calls it today,
    /// so the non-churn builds allow it unused.
    #[cfg_attr(not(dev_churn), allow(dead_code))]
    pub(crate) fn heap_stats() -> (usize, usize) {
        GLOBAL_ALLOCATOR.arena_stats()
    }

    /// The live/pooled block census (x86_64 17-02, the churn leak hunt;
    /// ported for 20-09): which size classes are genuinely outstanding
    /// vs parked on the free lists — the leak-vs-fragmentation
    /// fingerprint.  The tally runs under the heap lock into fixed stack
    /// arrays and the Vec copies happen after it is released, so the
    /// census never allocates under the lock (SMP-03).
    #[cfg_attr(not(dev_churn), allow(dead_code))]
    pub(crate) fn heap_census() -> (alloc::vec::Vec<(usize, usize)>, alloc::vec::Vec<(usize, usize)>)
    {
        GLOBAL_ALLOCATOR.split_census()
    }
}

// The module is kernel-only (the host test harness cannot link the
// global allocator), so the re-export rides the same cfg — an ungated
// `use global::…` is E0432 under cfg(test) and broke the canonical
// `cargo test -p x86_64 --bins` battery on x86_64 (its 15-regression
// catch).
#[cfg(not(test))]
#[cfg_attr(not(dev_churn), allow(unused_imports))]
pub(crate) use global::{heap_census, heap_stats};
