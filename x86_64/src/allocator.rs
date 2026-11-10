#[cfg(not(test))]
mod global {
    use core::mem;
    use core::sync::atomic::AtomicPtr;
    use port::allocator::{
        Block, BumpAlloc, QuickFit, global::GlobalHeap, global::GlobalQuickAlloc,
    };

    #[global_allocator]
    pub(crate) static GLOBAL_ALLOCATOR: GlobalQuickAlloc = GlobalQuickAlloc(AtomicPtr::new({
        static mut HEAP: GlobalHeap = GlobalHeap::new();
        static mut ALLOC: QuickFit = QuickFit::new(BumpAlloc::new(unsafe {
            Block::new_from_raw_parts((&raw mut HEAP).cast(), mem::size_of::<GlobalHeap>())
        }));
        &raw mut ALLOC
    }));

    /// Read-only heap-arena stats off the live global allocator (the
    /// services-venue oom hunt's probe, 15-03): (bump-cursor bytes,
    /// arena size).  Single CPU this era; a null swap window (an
    /// allocation in flight on THIS context) cannot happen — this runs
    /// between syscalls.
    pub(crate) fn heap_stats() -> (usize, usize) {
        // SAFETY: read-only access for the diagnostic print path; the
        // pointer is the same static the global allocator serves.
        unsafe {
            let a = GLOBAL_ALLOCATOR.0.load(core::sync::atomic::Ordering::Relaxed);
            assert!(!a.is_null(), "global allocator is nil");
            (*a).arena_stats()
        }
    }
}

pub(crate) use global::heap_stats;
