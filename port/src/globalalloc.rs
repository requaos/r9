//! The allocator's GlobalAlloc shim — the named
//! unsafe floor of the allocator pair.
//!
//! Everything raw-pointer lives here and ONLY here:
//! - `Block::new_from_raw_parts` — the arena-edge raw→Block constructor
//!   (aarch64/allocator.rs's static init calls it over the 4 MiB heap).
//! - `QuickFit::realloc` — the `unsafe fn` trait-edge surface (its
//!   caller contract), with the byte copy homed here too.
//! - `unsafe impl Sync for QuickFit` — the SyncUnsafeCell static's proof.
//! - `GlobalHeap` / `GlobalQuickAlloc` / the `GlobalAlloc` impl — the
//!   trait's unsafe fn surface + the with_allocator swap trick.
//!
//! `port::allocator` itself is unsafe-free and carries
//! `#[forbid(unsafe_code)]` (lib.rs's regression lock).

use alloc::alloc::Layout;
use core::ptr::NonNull;

use crate::allocator::{Block, BumpAlloc, QuickFit};

impl Block {
    /// Creates a new block from raw parts.  This is analogous
    /// to `core::slice::from_raw_parts`.
    ///
    /// # Safety
    /// The caller must ensure that the pointer and length given
    /// are appropriate for the construction of a new block.
    pub const unsafe fn new_from_raw_parts(ptr: *mut u8, len: usize) -> Block {
        // SAFETY: the caller contract (above) — ptr is non-null and names
        // len bytes of the owned region (every call site is an arena or a
        // host test buffer).
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Block::from_nonnull(ptr, len)
    }
}

// SAFETY: QuickFit's internals are now plain data (the index/arena model —
// no raw-pointer fields); the only sharing path is GlobalQuickAlloc's
// AtomicPtr swap discipline (one borrower at a time, single CPU this era).
// (The impl lives here so port::allocator stays forbid(unsafe_code)-clean;
// the SyncUnsafeCell<QuickFit> static in aarch64/allocator.rs needs it.)
unsafe impl Sync for QuickFit {}

/// Copy `n` live bytes from `src` to `dst` — the realloc byte move, homed
/// at the floor (both regions are live arena allocations of the caller's
/// proven sizes).
///
/// # Safety
/// Both regions must be valid for `n` bytes and non-overlapping (the
/// realloc contract: dst is a fresh allocation).
pub(crate) unsafe fn copy_live_bytes(src: *mut u8, dst: *mut u8, n: usize) {
    // SAFETY: the caller contract above.
    unsafe { core::ptr::copy(src, dst, n) };
}

impl QuickFit {
    /// Attempts to reallocate the given block to a new size.
    ///
    /// This has a small optimization for the most common case,
    /// where a block is being realloc'd to grow as data is
    /// accumulated: it's subtle, but if the original block was
    /// allocated from one of the quick lists, and the new size
    /// can be accommodated by the existing allocation, simply
    /// return the existing block pointer.  Otherwise, allocate
    /// a new block, copy, and free the old block.
    ///
    /// Note that the case of a reduction in size might result
    /// in a new allocation.  This is because we rely on the
    /// accuracy of the `Layout` to find the correct quicklist
    /// to store the block onto on free.  If we reduced below
    /// the size of the current block, we would lose the layout
    /// information and potentially leak memory.  But this is
    /// very uncommon.
    ///
    /// We make no effort to optimize the case of a `realloc` in
    /// a `misc` block, as a) it is relatively uncommon to do so
    /// and b) there may not be a buffer tag for such a block
    /// yet (one isn't allocated until the block is freed), and
    /// the implementation would need to be more complex as a
    /// result.
    ///
    /// # Safety
    /// Must be called with a valid block pointer, layout, and
    /// size.
    pub unsafe fn realloc(&mut self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if block.is_null() {
            return self.malloc(layout);
        }
        let new_layout = Layout::from_size_align(new_size, layout.align()).expect("layout");
        let (size, align) = Self::adjust(new_layout);
        if size == layout.size() && align == layout.align() {
            return block;
        }
        let np = self.malloc(new_layout);
        if !np.is_null() {
            // SAFETY: copy_live_bytes' contract — block names
            // layout.size() live bytes and np names new_size live bytes
            // (np was just allocated; the copy precedes the free — the
            // 05 copy-then-free order).
            unsafe {
                copy_live_bytes(block, np, usize::min(layout.size(), new_size));
            }
            self.free(block, layout)
        }
        np
    }
}

/// BumpAlloc implements the allocator interface, and is
/// suitable for e.g. page allocators and so forth.  Dealloc is
/// unimplemented and will panic.  (Homed here — the
/// trait impl's unsafe fn surface is the floor's.)
///
/// SAFETY: allocate returns arena memory owned by the BumpAlloc, never
/// aliased while the cursor discipline holds (each fetch_update carves a
/// fresh, non-overlapping region); deallocate's panic is the documented
/// contract.
unsafe impl alloc::alloc::Allocator for BumpAlloc {
    fn allocate(
        &self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, alloc::alloc::AllocError> {
        let (_, block) =
            self.try_alloc(layout.size(), layout.align()).ok_or(alloc::alloc::AllocError)?;
        Ok(NonNull::slice_from_raw_parts(block.as_nonnull(), block.len()))
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        // SAFETY: unreachable by the documented contract (bump allocators
        // never free); the panic is the implementation.
        unimplemented!();
    }
}

#[cfg(not(test))]
mod global {
    use crate::allocator::QuickFit;
    use alloc::alloc::{GlobalAlloc, Layout};
    use core::ptr;
    use core::sync::atomic::{AtomicPtr, Ordering};

    const GLOBAL_HEAP_SIZE: usize = 4 * 1024 * 1024;

    /// A GlobalHeap is an aligned wrapper around an owned
    /// buffer.
    #[repr(C, align(4096))]
    pub struct GlobalHeap([u8; GLOBAL_HEAP_SIZE]);
    impl GlobalHeap {
        pub const fn new() -> GlobalHeap {
            Self([0u8; GLOBAL_HEAP_SIZE])
        }
    }

    impl Default for GlobalHeap {
        fn default() -> Self {
            Self::new()
        }
    }

    /// GlobalQuickAlloc is a wrapper around a QuickFit over a
    /// GlobalHeap that uses interior mutability to implement
    /// the GlobalAlloc trait.
    pub struct GlobalQuickAlloc(pub AtomicPtr<QuickFit>);
    impl GlobalQuickAlloc {
        fn with_allocator<F, R>(&self, thunk: F) -> R
        where
            F: FnOnce(&mut QuickFit) -> R,
        {
            let a = self.0.swap(ptr::null_mut(), Ordering::Relaxed);
            assert!(!a.is_null(), "global allocator is nil");
            // SAFETY: the swap gives this context EXCLUSIVE ownership of
            // the allocator for the thunk's duration (any concurrent
            // caller sees nil and panics); single CPU this era.
            let r = thunk(unsafe { &mut *a });
            self.0.swap(a, Ordering::Relaxed);
            r
        }
    }

    // SAFETY: every trait method runs under with_allocator's exclusive
    // borrow; malloc/free/realloc's own contracts are their docs.  The
    // trait's caller contract (valid layouts, owned blocks) is the Rust
    // allocation protocol.
    unsafe impl GlobalAlloc for GlobalQuickAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            self.with_allocator(|quick| quick.malloc(layout))
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            self.with_allocator(|quick| quick.free(ptr, layout));
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: QuickFit::realloc's caller contract is met (ptr is a
            // live allocation of layout, per the trait protocol).
            self.with_allocator(|quick| unsafe { quick.realloc(ptr, layout, new_size) })
        }
    }
}

#[cfg(not(test))]
pub use global::{GlobalHeap, GlobalQuickAlloc};
