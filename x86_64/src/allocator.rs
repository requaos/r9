#[cfg(not(test))]
mod global {
    use core::cell::SyncUnsafeCell;
    use core::mem;
    use port::allocator::{
        Block, BumpAlloc, QuickFit, global::GlobalHeap, global::GlobalQuickAlloc,
    };

    // The heap arena and the QuickFit state live in SyncUnsafeCell
    // statics (never `static mut`, matching aarch64).  GlobalQuickAlloc's
    // IrqSpinLock (port::globalalloc, LockRank::HeapAlloc) is the sole
    // synchronized access path; the cells' pointers are published into it
    // here at static-init time, before any allocation can run (SMP-03).
    static HEAP: SyncUnsafeCell<GlobalHeap> = SyncUnsafeCell::new(GlobalHeap::new());
    static ALLOC: SyncUnsafeCell<QuickFit> = SyncUnsafeCell::new(QuickFit::new(
        // SAFETY: the Block spans HEAP — a static arena alive for the
        // kernel's whole lifetime — from its base for its full size, so
        // the pointer is valid and exclusively owned (this one-time
        // static initializer runs before any allocation exists) and the
        // base is aligned to GlobalHeap's own alignment by the static's
        // type.
        BumpAlloc::new(unsafe {
            Block::new_from_raw_parts(HEAP.get().cast(), mem::size_of::<GlobalHeap>())
        }),
    ));

    #[global_allocator]
    pub(crate) static GLOBAL_ALLOCATOR: GlobalQuickAlloc = GlobalQuickAlloc::new(ALLOC.get());

    /// Read-only heap-arena stats off the live global allocator (the
    /// services-venue oom hunt's probe, 15-03): (bump-cursor bytes,
    /// arena size).  Serialized under the heap's IrqSpinLock and
    /// allocating nothing (SMP-03).
    pub(crate) fn heap_stats() -> (usize, usize) {
        GLOBAL_ALLOCATOR.arena_stats()
    }

    /// The live/pooled block census (17-02, the churn leak hunt): which
    /// size classes are genuinely outstanding vs parked on the free
    /// lists — the leak-vs-fragmentation fingerprint.  The tally runs
    /// under the heap lock into fixed stack arrays and the Vec copies
    /// happen after it is released, so the census never allocates under
    /// the lock (SMP-03).
    pub(crate) fn heap_census() -> (alloc::vec::Vec<(usize, usize)>, alloc::vec::Vec<(usize, usize)>) {
        GLOBAL_ALLOCATOR.split_census()
    }
}

// The module is kernel-only (the host test harness cannot link the
// global allocator), so the re-export rides the same cfg — an ungated
// `use global::…` is E0432 under cfg(test) and broke the canonical
// `cargo test -p x86_64 --bins` battery (15-regression catch).
#[cfg(not(test))]
pub(crate) use global::{heap_census, heap_stats};
