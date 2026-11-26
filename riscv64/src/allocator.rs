#[cfg(not(test))]
mod global {
    use core::mem;
    use port::allocator::{
        Block, BumpAlloc, QuickFit, global::GlobalHeap, global::GlobalQuickAlloc,
    };

    // The GlobalQuickAlloc::new constructor publishes the arena-owned
    // QuickFit into the IrqSpinLock-guarded global allocator (SMP-03);
    // riscv64 keeps its `static mut` init until its Phase 7 audit/sync
    // horizon (it is never compiled by the gates and is not lint-opted-in).
    #[global_allocator]
    static GLOBAL_ALLOCATOR: GlobalQuickAlloc = GlobalQuickAlloc::new({
        static mut HEAP: GlobalHeap = GlobalHeap::new();
        static mut ALLOC: QuickFit = QuickFit::new(BumpAlloc::new(unsafe {
            Block::new_from_raw_parts((&raw mut HEAP).cast(), mem::size_of::<GlobalHeap>())
        }));
        &raw mut ALLOC
    });
}
