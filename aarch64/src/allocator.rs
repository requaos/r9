#[cfg(not(test))]
mod global {
    use core::cell::SyncUnsafeCell;
    use core::mem;
    use core::sync::atomic::AtomicPtr;
    use port::allocator::{
        Block, BumpAlloc, QuickFit, global::GlobalHeap, global::GlobalQuickAlloc,
    };

    // 261103-p64: the heap arena and the QuickFit state live in
    // SyncUnsafeCell statics (never `static mut`).  GlobalQuickAlloc's
    // AtomicPtr swap trick (port::allocator::global) is the sole
    // synchronized access path; the cells' pointers are published into it
    // here at static-init time, before any allocation can run.
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
    static GLOBAL_ALLOCATOR: GlobalQuickAlloc = GlobalQuickAlloc(AtomicPtr::new(ALLOC.get()));
}
