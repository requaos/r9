// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![allow(clippy::too_long_first_doc_paragraph)]

use alloc::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::{mem, ptr};

/// The allocator works in terms of an owned region of memory
/// that is represented by a Block, which describes the region
/// in terms of a non-nil pointer and a length.  A Block is an
/// analogue of a mutable slice.
///
/// At some point, it may make sense to replace this with a
/// slice pointer, but too many of the interfaces there are not
/// (yet) stable.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    ptr: NonNull<u8>,
    len: usize,
}

impl Block {
    /// Creates a new block from raw parts.  This is analogous
    /// to `core::slice::from_raw_parts`.
    ///
    /// # Safety
    /// The caller must ensure that the pointer and length given
    /// are appropriate for the construction of a new block.
    pub const unsafe fn new_from_raw_parts(ptr: *mut u8, len: usize) -> Block {
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Block { ptr, len }
    }

    /// Splits a block into two sub-blocks.
    pub fn split_at_mut(self, offset: usize) -> Option<(Block, Block)> {
        let len = self.len();
        if offset > len {
            return None;
        }
        let ptr = self.as_ptr();
        let a = unsafe { Block::new_from_raw_parts(ptr, offset) };
        let b = unsafe { Block::new_from_raw_parts(ptr.wrapping_add(offset), len - offset) };
        Some((a, b))
    }

    /// Returns a raw mutable pointer to the beginning of the
    /// owned region.
    pub fn as_ptr(self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Returns the length of the region.
    fn len(self) -> usize {
        self.len
    }
}

/// A Bump Allocator takes ownership a region of memory, called
/// an "arena", represented by a Block, and maintains a cursor
/// into that region.  The cursor denotes the point between
/// allocated and unallocated memory in the arena.
pub struct BumpAlloc {
    arena: Block,
    cursor: AtomicUsize,
}

impl BumpAlloc {
    /// Creates a new bump allocator over the given Block.
    /// Takes ownership of the provided region.
    pub const fn new(arena: Block) -> BumpAlloc {
        BumpAlloc { arena, cursor: AtomicUsize::new(0) }
    }

    /// Allocates the requested number of bytes with the given
    /// alignment.  Returns `None` if the allocation cannot be
    /// satisfied, otherwise returns `Some` of a pair of blocks:
    /// the first contains the prefix before the (aligned) block
    /// and the second is the requested block itself.
    ///
    /// (05-07: the parameter ORDER was `(align, size)` while BOTH call
    /// sites pass `(size, align)` — `align_offset` then ran on the SIZE
    /// and panicked ("align is not a power-of-two") on the first big
    /// non-power-of-two tail allocation (devmnt's ~250 KB exec staging
    /// buffer).  The names now match the call sites; the alignment
    /// (always a power of two, from the Layout) is what align_offset
    /// consumes.)
    pub fn try_alloc(&self, size: usize, align: usize) -> Option<(Block, Block)> {
        let base = self.arena.as_ptr();
        let mut first = ptr::null_mut();
        let mut adjust = 0;
        self.cursor
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                first = base.wrapping_add(current);
                adjust = first.align_offset(align);
                let offset = current.checked_add(adjust).expect("alignment overflow");
                let next = offset.checked_add(size).expect("size overflow");
                (next <= self.arena.len()).then_some(next)
            })
            .ok()?;
        let prefix = unsafe { Block::new_from_raw_parts(first, adjust) };
        let ptr = first.wrapping_add(adjust);
        let block = unsafe { Block::new_from_raw_parts(ptr, size) };
        Some((prefix, block))
    }
}

/// BumpAlloc<T> implements the allocator interface, and is
/// suitable for e.g. page allocators and so forth.  Dealloc is
/// unimplemented and will panic.
unsafe impl Allocator for BumpAlloc {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let (_, block) = self.try_alloc(layout.size(), layout.align()).ok_or(AllocError)?;
        Ok(NonNull::slice_from_raw_parts(block.ptr, block.len()))
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        unimplemented!();
    }
}

// # QuickFit allocator for small objects.
//
// This is an implementation of the QuickFit[Wei88] allocator
// for small objects, suitable for managing small heaps in
// memory constrained environments, such as boot loaders and
// standalone debuggers.
//
// [Wei88] Charles B. Weinstock and William A. Wulf. 1988.
// Quick Fit: An Efficient Algorithm for Heap Storage
// Allocation.  ACM SIGPLAN Notices 23, 10 (Oct. 1988),
// 141-148.  https://doi.org/10.1145/51607.51619

const ALLOC_UNIT_SHIFT: usize = 6;
const ALLOC_UNIT_SIZE: usize = 1 << ALLOC_UNIT_SHIFT;
const MIN_ALLOC_SIZE: usize = ALLOC_UNIT_SIZE;
const MAX_QUICK_SHIFT: usize = 14;
const MAX_QUICK_SIZE: usize = 1 << MAX_QUICK_SHIFT;

const NUM_QLISTS: usize = 14 - ALLOC_UNIT_SHIFT + 1;
const NUM_HASH_BUCKETS: usize = 31; // Prime.

/// A linked block header containing size, alignment, and
/// address information for the block.  This is used both for
/// linking unallocated blocks into one of the free lists and
/// for keeping track of blocks allocated from the `misc` list.
///
/// For irregularly sized allocations, the header keeps track of
/// the block's layout data, its virtual address, and a link
/// pointer.  Such a header is either not in any list, if newly
/// allocated and not yet freed, or always in exactly one of two
/// lists: the free list, or a hash chain of allocated blocks.
/// We do this because we need some way to preserve the
/// allocation size after the initial allocation from the tail,
/// and because misc blocks can be reused in a first-fit manner,
/// we cannot rely on a `Layout` to recover the size of the
/// block, so we must store it somewhere.  By allocating a tag
/// outside of the buffer, which we look up in a hash table as
/// needed, we can maintain this information without adding
/// additional complexity to allocation.
///
/// For blocks on one of the quick lists, the size, address and
/// alignment fields are redundant, but convenient.
///
/// We use the link pointer to point to the next entry in the
/// list in all cases.
#[derive(Debug)]
#[repr(C, align(64))]
struct Header {
    next: Option<NonNull<Header>>,
    addr: NonNull<u8>,
    size: usize,
    align: usize,
}

impl Header {
    /// Returns a new header for a block of the given size and
    /// alignment at the given address.
    fn new(addr: NonNull<u8>, size: usize, align: usize, next: Option<NonNull<Header>>) -> Header {
        Header { next, addr, size, align }
    }
}

/// The QuickFit allocator itself.  The allocator takes
/// ownership of a bump allocator for the tail, and contains a
/// set of lists for the quick blocks, as well as a misc list
/// for unusually sized regions, and a hash table of headers
/// describing current misc allocations.  As mentioned above,
/// these last data are kept outside of the allocations to keep
/// allocation simple.
#[repr(C)]
pub struct QuickFit {
    tail: BumpAlloc,
    qlists: [Option<NonNull<Header>>; NUM_QLISTS],
    misc: Option<NonNull<Header>>,
    allocated_misc: [Option<NonNull<Header>>; NUM_HASH_BUCKETS],
}

impl QuickFit {
    /// Constructs a QuickFit from the given `tail`.
    pub const fn new(tail: BumpAlloc) -> QuickFit {
        let qlists = [None; NUM_QLISTS];
        let misc = None;
        let allocated_misc = [None; NUM_HASH_BUCKETS];
        QuickFit { tail, qlists, misc, allocated_misc }
    }

    /// Allocates a block of memory of the requested size and
    /// alignment.  Returns a pointer to such a block, or nil if
    /// the block cannot be allocated.
    pub fn malloc(&mut self, layout: Layout) -> *mut u8 {
        let (size, align) = Self::adjust(layout);
        let p = self.alloc_quick(size, align);
        p.or_else(|| self.alloc_tail(size, align)).map(|p| p.as_ptr()).unwrap_or(ptr::null_mut())
    }

    /// Adjusts the given layout so that blocks allocated from
    /// one of the quick lists are appropriately sized and
    /// aligned.  Otherwise, returns the original size and
    /// alignment.
    fn adjust(layout: Layout) -> (usize, usize) {
        let size = layout.size();
        let align = layout.align();
        if size > MAX_QUICK_SIZE {
            return (size, align);
        }
        let size = usize::max(MIN_ALLOC_SIZE, size.next_power_of_two());
        let align = usize::max(layout.align(), size);
        (size, align)
    }

    /// Attempts to allocate from an existing list: for requests
    /// that can be satisfied from one of the quick lists, try
    /// and do so; otherwise, attempt an allocation from the
    /// misc list.
    fn alloc_quick(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        if size <= MAX_QUICK_SIZE && align == size {
            let k: usize = size.ilog2() as usize - ALLOC_UNIT_SHIFT;
            let (node, list) = Self::head(self.qlists[k].take());
            self.qlists[k] = list;
            node.map(|header| unsafe { header.as_ref() }.addr)
        } else {
            self.alloc_misc(size, align)
        }
    }

    /// Allocates a block from the misc list.  This is a simple
    /// first-fit allocator.
    fn alloc_misc(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let (node, list) =
            Self::unlink(self.misc.take(), |node| size <= node.size && align <= node.align);
        self.misc = list;
        node.map(|mut header| {
            let header = unsafe { header.as_mut() };
            let k = Self::hash(header.addr.as_ptr());
            header.next = self.allocated_misc[k].take();
            self.allocated_misc[k] = NonNull::new(header);
            header.addr
        })
    }

    /// Allocates an aligned block of size `size` from `tail`.
    /// If `tail` is not already aligned to the given alignment,
    /// then we try to free blocks larger than or equal in size
    /// to the minimum allocation unit into the quick lists
    /// until it is.
    fn alloc_tail(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let (prefix, block) = { self.tail.try_alloc(size, align)? };
        self.free_prefix(prefix);
        Some(block.ptr)
    }

    /// Frees a prefix that came from a tail allocation.  This
    /// attempts to store blocks into the quick lists.
    fn free_prefix(&mut self, prefix: Block) {
        let mut prefix = Self::align_prefix(prefix);
        while let Some(rest) = self.try_free_prefix(prefix) {
            prefix = rest;
        }
    }

    /// Aligns the prefix to the minimum allocation size.
    ///
    /// 05-REVIEW-FIX: the `offset <= len` assert fired on a ZERO-LENGTH
    /// prefix when the tail cursor was not 64-aligned (a small-align
    /// request after a non-64-multiple-sized tail block — e.g. the
    /// Vec-doubling 65 328-byte staging step — leaves the cursor
    /// misaligned; the next align-1 tail alloc has a 0-length prefix
    /// whose 64-alignment offset exceeds it).  Clamp instead: a prefix
    /// shorter than the alignment offset has nothing carveable — the
    /// bytes stay unused at the tail boundary (a tiny leak, never a
    /// panic, never corruption).
    fn align_prefix(prefix: Block) -> Block {
        let ptr = prefix.as_ptr();
        let len = prefix.len();
        let offset = ptr.align_offset(MIN_ALLOC_SIZE).min(len);
        unsafe { Block::new_from_raw_parts(ptr.wrapping_add(offset), len - offset) }
    }

    /// Tries to free the largest section of the prefix that it
    /// can, returning the remainder if it did so.  Otherwise,
    /// returns None.
    fn try_free_prefix(&mut self, prefix: Block) -> Option<Block> {
        let ptr: *mut u8 = prefix.as_ptr();
        for k in (0..NUM_QLISTS).rev() {
            let size = 1 << (k + ALLOC_UNIT_SHIFT);
            if prefix.len() >= size && ptr.align_offset(size) == 0 {
                let (_, rest) = prefix.split_at_mut(size)?;
                self.free(ptr, Layout::from_size_align(size, size).unwrap());
                return (rest.len() >= MIN_ALLOC_SIZE).then_some(rest);
            }
        }
        None
    }

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
            unsafe {
                ptr::copy(block, np, usize::min(layout.size(), new_size));
            }
            self.free(block, layout)
        }
        np
    }

    /// Frees a block of memory characterized by the `layout`
    /// argument.  If the block can be freed to one of the
    /// quick lists, it is; otherwise, it is treated as a misc
    /// block and freed there.
    pub fn free(&mut self, block: *mut u8, layout: Layout) {
        let Some(block) = NonNull::new(block) else {
            return;
        };
        let (size, align) = Self::adjust(layout);
        if size <= MAX_QUICK_SIZE && align == size {
            let k: usize = size.ilog2() as usize - ALLOC_UNIT_SHIFT;
            let header = Header::new(block, size, align, self.qlists[k].take());
            assert_eq!(block.align_offset(mem::align_of::<Header>()), 0);
            let p = block.cast::<Header>();
            unsafe {
                ptr::write(p.as_ptr(), header);
            }
            self.qlists[k] = Some(p);
        } else {
            self.free_misc(block, size, align);
        }
    }

    /// Frees a block to the misc list.  This looks up the given
    /// address in the hash of allocated misc blocks to find its
    /// header.
    ///
    /// If the block header is not found in the hash table, we
    /// assume that the block was allocated from the tail and
    /// this is the first time it's been freed, so we allocate a
    /// header for it and link that into the misc list.
    ///
    /// If we cannot allocate a header in the usual way, we take
    /// it from the block to be freed, which is guaranteed to be
    /// large enough to hold a header, since anything smaller
    /// would have been allocated from one of the quick lists,
    /// and thus freed through that path.
    fn free_misc(&mut self, mut block: NonNull<u8>, mut size: usize, mut align: usize) {
        let mut header = self
            .unlink_allocated_misc(block)
            .or_else(|| {
                // The usual way: malloc a fresh header.  (05-07: this arm
                // was INVERTED — `is_null().then(carve)` produced the
                // carved header only when malloc FAILED, and
                // `.expect("allocated header block")` panicked whenever
                // malloc SUCCEEDED — the first free of a tail-allocated
                // block (devmnt's ~250 KB exec staging buffer) died here.
                // Never exercised before: the kernel heap's allocation
                // pattern was alloc-never-free for every > MAX_QUICK_SIZE
                // block until exec staging.)
                let hblock = self.malloc(Layout::new::<Header>()).cast::<Header>();
                let hblock = if hblock.is_null() {
                    // The heap is too full for a fresh header: carve it
                    // from the block being freed (guaranteed big enough —
                    // anything smaller would have been allocated from a
                    // quick list and freed through that path).
                    let offset = block.align_offset(MIN_ALLOC_SIZE);
                    let hblock = block.as_ptr().wrapping_add(offset);
                    let next = hblock.wrapping_add(MIN_ALLOC_SIZE);
                    block = unsafe { NonNull::new_unchecked(next) };
                    size -= offset + MIN_ALLOC_SIZE;
                    align = MIN_ALLOC_SIZE;
                    hblock.cast::<Header>()
                } else {
                    hblock
                };
                let header = Header::new(block, size, align, None);
                unsafe {
                    ptr::write(hblock, header);
                }
                NonNull::new(hblock)
            })
            .expect("header");
        let header = unsafe { header.as_mut() };
        header.next = self.misc.take();
        self.misc = NonNull::new(header);
    }

    /// Unlinks the header for the given address from the hash
    /// table for allocated misc blocks and returns it, if such
    /// a header exists.  If the block associated with the
    /// address has not been freed yet, it's possible that no
    /// header for it exists yet, in which case we return None.
    fn unlink_allocated_misc(&mut self, block: NonNull<u8>) -> Option<NonNull<Header>> {
        let k = Self::hash(block.as_ptr());
        let list = self.allocated_misc[k].take();
        let (node, list) = Self::unlink(list, |node| node.addr == block);
        self.allocated_misc[k] = list;
        node
    }

    /// Unlinks the first node matching the given predicate from
    /// the given list, if it exists, returning the node, or
    /// None, and the list head.  The list head will be None if
    /// the list is empty.
    fn unlink<F>(
        list: Option<NonNull<Header>>,
        predicate: F,
    ) -> (Option<NonNull<Header>>, Option<NonNull<Header>>)
    where
        F: Fn(&Header) -> bool,
    {
        let mut prev: Option<NonNull<Header>> = None;
        let head = list;
        let mut cur = list;
        // 05-REVIEW-FIX (the WR-04 regression's root cause): the previous
        // shape returned the advanced cursor as the "list" — a mid-list
        // removal returned the JUST-REMOVED node as the new head
        // (orphaning the real head and tail, and putting the still-hashed
        // header back on the free list → the same header lived in two
        // lists → a later insert made it point at itself → a CYCLED list,
        // and the next first-fit walk spun forever — the restart-exec
        // wedge), and a no-match walk returned None (wiping the whole
        // list — a permanent leak).  Both are fixed by preserving the
        // head: a head removal returns the next node, a mid-list removal
        // returns the original head, and a no-match walk returns the list
        // untouched.  The spin guard stays as a permanent tripwire while
        // the gate's latent-wedge hunt continues: a cycled list panics
        // loudly instead of hanging the kernel silently (the gate's
        // panic marker catches it).
        let mut spins = 0usize;
        while let Some(mut node_ptr) = cur {
            spins += 1;
            if spins > 100_000 {
                let n = unsafe { node_ptr.as_ref() };
                crate::println!("allocator: unlink spin — free-list cycle (node addr {:?} size {} align {})", n.addr, n.size, n.align);
                panic!("allocator: free-list cycle detected");
            }
            let node = unsafe { node_ptr.as_mut() };
            if predicate(node) {
                let next = node.next.take();
                match prev {
                    // Mid-list removal: splice prev→next; head unchanged.
                    Some(mut p) => {
                        unsafe { p.as_mut() }.next = next;
                        return (Some(node_ptr), head);
                    }
                    // Head removal: the next node becomes the head.
                    None => return (Some(node_ptr), next),
                }
            }
            prev = Some(node_ptr);
            cur = node.next;
        }
        // No match: the list is returned UNCHANGED (never wiped).
        (None, head)
    }

    /// Splits the list into it's first element and tail and
    /// returns both.
    fn head(list: Option<NonNull<Header>>) -> (Option<NonNull<Header>>, Option<NonNull<Header>>) {
        Self::unlink(list, |_| true)
    }

    /// Hashes a pointer value.  This is the bit mixing algorithm
    /// from Murmur3.
    fn hash(ptr: *mut u8) -> usize {
        let mut k = ptr.addr();
        k ^= k >> 33;
        k = k.wrapping_mul(0xff51afd7ed558ccd);
        k ^= k >> 33;
        k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
        (k >> 33) % NUM_HASH_BUCKETS
    }
}

#[cfg(not(test))]
pub mod global {
    use super::QuickFit;
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
            let r = thunk(unsafe { &mut *a });
            self.0.swap(a, Ordering::Relaxed);
            r
        }
    }

    unsafe impl GlobalAlloc for GlobalQuickAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            self.with_allocator(|quick| quick.malloc(layout))
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            self.with_allocator(|quick| quick.free(ptr, layout));
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            self.with_allocator(|quick| unsafe { quick.realloc(ptr, layout, new_size) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::alloc::Layout;

    /// 05-07 regression: the > MAX_QUICK_SIZE, non-power-of-two TAIL path
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

    /// 05-REVIEW-FIX regression (the WR-04 gate wedge's root cause):
    /// unlink's mid-list removal returned the removed node as the list
    /// head — the misc list then held a still-allocated header (a later
    /// insert cycled it, and the next first-fit walk spun forever — the
    /// restart-exec wedge at the ~130 KB staging growth), and a no-match
    /// walk wiped the whole list (a permanent leak).  Pin both arms by
    /// POINTER identity: blocks must be recycled from the free list,
    /// never tail-bumped past a wipe.
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
}
