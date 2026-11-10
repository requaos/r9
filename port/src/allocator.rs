// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![allow(clippy::too_long_first_doc_paragraph)]

use alloc::alloc::Layout;
use core::ptr::NonNull;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The allocator works in terms of an owned region of memory
/// that is represented by a Block, which describes the region
/// in terms of a non-nil pointer and a length.  A Block is an
/// analogue of a mutable slice.
///
/// Block's fields are constructed
/// directly (never through raw-pointer reinterpretation) — the ONLY
/// raw-to-Block constructor is `new_from_raw_parts`, homed in the
/// GlobalAlloc shim module (`globalalloc.rs`) so this module is
/// unsafe-free (`#[forbid(unsafe_code)]`, lib.rs).
#[derive(Clone, Copy, Debug)]
pub struct Block {
    ptr: NonNull<u8>,
    len: usize,
}

impl Block {
    /// The crate-internal constructor over an already-valid NonNull region
    /// (the arena base, or a sub-block carve of one — the parent's validity
    /// covers the sub-region).
    pub(crate) const fn from_nonnull(ptr: NonNull<u8>, len: usize) -> Block {
        Block { ptr, len }
    }

    /// Splits a block into two sub-blocks.
    pub fn split_at_mut(self, offset: usize) -> Option<(Block, Block)> {
        let len = self.len();
        if offset > len {
            return None;
        }
        // Sub-regions of a live block: plain field construction — offset <=
        // len keeps both inside the owned region; the wrapping_add can never
        // leave the arena, let alone wrap to null (the arena invariant).
        let a = Block { ptr: self.ptr, len: offset };
        let b = Block {
            ptr: NonNull::new(self.ptr.as_ptr().wrapping_add(offset)).expect("arena sub-block"),
            len: len - offset,
        };
        Some((a, b))
    }

    /// Returns a raw mutable pointer to the beginning of the
    /// owned region.
    pub fn as_ptr(self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// The NonNull view (crate-internal — the shim's slice construction).
    pub(crate) const fn as_nonnull(self) -> NonNull<u8> {
        self.ptr
    }

    /// Returns the length of the region.
    pub(crate) const fn len(self) -> usize {
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
    /// The parameter ORDER was `(align, size)` while BOTH call
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
        // The fetch_update above proved next <= arena.len(), so both
        // sub-regions lie inside the arena (the cursor never passes the
        // end) — plain field construction.
        let prefix = Block::from_nonnull(NonNull::new(first).expect("arena cursor"), adjust);
        let ptr = first.wrapping_add(adjust);
        let block = Block::from_nonnull(NonNull::new(ptr).expect("arena block"), size);
        Some((prefix, block))
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

// The header side table's fixed capacity.  The// main region serves every taker; the RESERVE region serves only frees
// (the index-model equivalent of free_misc's carve-from-the-freed-block
// emergency header) so a full main table never wedges a free.  Both
// empty -> the loud named panic (never a wrap, never a corruption).
const HEADER_CAP: usize = 4096;
const RESERVE_CAP: usize = 1024;
const HEADER_TOTAL: usize = HEADER_CAP + RESERVE_CAP;

/// A header slot index into the side table (u16: HEADER_TOTAL << 65536).
type Idx = u16;

/// A block's metadata — a SIDE-TABLE entry (headers never
/// live inside blocks anymore; the intrusive-pointer era's `repr(C,
/// align(64))` Header is gone).  `next` links the slot's current chain —
/// a quick/misc/hash chain when used, a free-slot chain when free.
///
/// As in the pointer model: for irregularly sized allocations the header
/// tracks the block's layout data, its address (an arena OFFSET now), and
/// the link; such a header is either in no list (newly allocated and not
/// yet freed) or always in exactly one of two lists: the free list, or a
/// hash chain of allocated blocks.  The side table preserves the
/// allocation size without adding complexity to allocation.
#[derive(Debug, Clone, Copy)]
struct Slot {
    next: Option<Idx>,
    addr: u32, // the block's offset from the arena base
    size: usize,
    align: usize,
    used: bool,
}

const EMPTY_SLOT: Slot = Slot { next: None, addr: 0, size: 0, align: 0, used: false };

/// The QuickFit allocator itself — the index/arena model:
/// the bump tail plus the free lists and hash chains as Option<Idx>
/// chains over the fixed side table.  Blocks are referenced by arena
/// OFFSET (u32 from the arena base); the only pointer materialization is
/// base+offset at the public API edge (malloc's `*mut u8` return).
/// Internals carry no raw-pointer free-list surgery.
#[repr(C)]
pub struct QuickFit {
    tail: BumpAlloc,
    slots: [Slot; HEADER_TOTAL],
    /// Free-slot chains (through Slot.next of unused slots) + the
    /// never-linked watermarks.  The reserve chain covers slots
    /// [HEADER_CAP, HEADER_TOTAL).
    free_main: Option<Idx>,
    free_reserve: Option<Idx>,
    high_main: Idx,
    high_reserve: Idx,
    qlists: [Option<Idx>; NUM_QLISTS],
    misc: Option<Idx>,
    allocated_misc: [Option<Idx>; NUM_HASH_BUCKETS],
}

impl QuickFit {
    /// Constructs a QuickFit from the given `tail`.
    pub const fn new(tail: BumpAlloc) -> QuickFit {
        // The offset model requires the arena to be u32-sized.
        assert!(tail.arena.len() <= u32::MAX as usize, "arena exceeds the u32 offset model");
        let qlists = [None; NUM_QLISTS];
        let misc = None;
        let allocated_misc = [None; NUM_HASH_BUCKETS];
        QuickFit {
            tail,
            slots: [EMPTY_SLOT; HEADER_TOTAL],
            free_main: None,
            free_reserve: None,
            high_main: 0,
            high_reserve: HEADER_CAP as Idx,
            qlists,
            misc,
            allocated_misc,
        }
    }

    /// Allocates a block of memory of the requested size and
    /// alignment.  Returns a pointer to such a block, or nil if
    /// the block cannot be allocated.
    pub fn malloc(&mut self, layout: Layout) -> *mut u8 {
        let (size, align) = Self::adjust(layout);
        let p = self.alloc_quick(size, align);
        p.or_else(|| self.alloc_tail(size, align)).unwrap_or(ptr::null_mut())
    }

    /// Adjusts the given layout so that blocks allocated from
    /// one of the quick lists are appropriately sized and
    /// aligned.  Otherwise, returns the original size and
    /// alignment.
    pub(crate) fn adjust(layout: Layout) -> (usize, usize) {
        let size = layout.size();
        let align = layout.align();
        if size > MAX_QUICK_SIZE {
            return (size, align);
        }
        let size = usize::max(MIN_ALLOC_SIZE, size.next_power_of_two());
        let align = usize::max(layout.align(), size);
        (size, align)
    }

    /// Materialize the block pointer for an arena offset — the ONLY
    /// offset→pointer step, and it is pure arithmetic (the arena base is
    /// a live region; the offset was bounds-proven at its carving).
    fn block_at(&self, offset: u32) -> *mut u8 {
        self.tail.arena.as_ptr().wrapping_add(offset as usize)
    }

    /// The block's arena offset.  A bogus free (below the base / past the
    /// end) is a LOUD panic, never silent corruption — hardening on an
    /// invalid input (valid inputs are unchanged).
    fn offset_of(&self, block: *mut u8) -> Option<u32> {
        if block.is_null() {
            return None;
        }
        let base = self.tail.arena.as_ptr().addr();
        let off = block
            .addr()
            .checked_sub(base)
            .expect("allocator: free outside the arena (below base)");
        assert!(off < self.tail.arena.len(), "allocator: free outside the arena (past end)");
        Some(off as u32)
    }

    /// Take a header slot: the main free chain, then the main watermark,
    /// then (frees only — the emergency-header fallback's index
    /// equivalent) the reserve.  Both exhausted -> the loud named panic
    /// (never a wrap, never a corruption).
    fn take_slot(&mut self, for_free: bool) -> Idx {
        if let Some(i) = self.free_main {
            self.free_main = self.slots[i as usize].next;
            self.slots[i as usize].next = None;
            self.slots[i as usize].used = true;
            return i;
        }
        if self.high_main < HEADER_CAP as Idx {
            let i = self.high_main;
            self.high_main += 1;
            self.slots[i as usize].used = true;
            return i;
        }
        if for_free {
            if let Some(i) = self.free_reserve {
                self.free_reserve = self.slots[i as usize].next;
                self.slots[i as usize].next = None;
                self.slots[i as usize].used = true;
                return i;
            }
            if self.high_reserve < HEADER_TOTAL as Idx {
                let i = self.high_reserve;
                self.high_reserve += 1;
                self.slots[i as usize].used = true;
                return i;
            }
        }
        panic!("allocator: header table exhausted");
    }

    /// Return a slot to its region's free chain (main below HEADER_CAP,
    /// reserve at/above it — a slot never changes regions).
    fn release_slot(&mut self, i: Idx) {
        let s = &mut self.slots[i as usize];
        s.used = false;
        s.next =
            if (i as usize) < HEADER_CAP { self.free_main.replace(i) } else { self.free_reserve.replace(i) };
    }

    /// Attempts to allocate from an existing list: for requests
    /// that can be satisfied from one of the quick lists, try
    /// and do so; otherwise, attempt an allocation from the
    /// misc list.
    fn alloc_quick(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        if size <= MAX_QUICK_SIZE && align == size {
            let k: usize = size.ilog2() as usize - ALLOC_UNIT_SHIFT;
            let (node, list) = Self::unlink(&mut self.slots, self.qlists[k].take(), |_| true);
            self.qlists[k] = list;
            // The popped header's slot goes back to the free pool (the
            // pointer model's header-in-the-block vanishing act).
            node.map(|i| {
                let addr = self.slots[i as usize].addr;
                self.release_slot(i);
                self.block_at(addr)
            })
        } else {
            self.alloc_misc(size, align)
        }
    }

    /// Allocates a block from the misc list.  This is a simple
    /// first-fit allocator.
    fn alloc_misc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let (node, list) =
            Self::unlink(&mut self.slots, self.misc.take(), |n| size <= n.size && align <= n.align);
        self.misc = list;
        node.map(|i| {
            // Move the header onto its allocated hash chain (the slot
            // stays taken — the block's registration for free time).
            let k = Self::hash(self.slots[i as usize].addr);
            self.slots[i as usize].next = self.allocated_misc[k].take();
            self.allocated_misc[k] = Some(i);
            self.block_at(self.slots[i as usize].addr)
        })
    }

    /// Allocates an aligned block of size `size` from `tail`.
    /// If `tail` is not already aligned to the given alignment,
    /// then we try to free blocks larger than or equal in size
    /// to the minimum allocation unit into the quick lists
    /// until it is.
    fn alloc_tail(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let (prefix, block) = { self.tail.try_alloc(size, align)? };
        self.free_prefix(prefix);
        Some(block.as_ptr())
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
    /// The `offset <= len` assert fired on a ZERO-LENGTH
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
        // A sub-region of the prefix block — plain field construction.
        Block {
            ptr: NonNull::new(ptr.wrapping_add(offset)).expect("arena prefix"),
            len: len - offset,
        }
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

    /// Frees a block of memory characterized by the `layout`
    /// argument.  If the block can be freed to one of the
    /// quick lists, it is; otherwise, it is treated as a misc
    /// block and freed there.
    pub fn free(&mut self, block: *mut u8, layout: Layout) {
        let Some(addr) = self.offset_of(block) else {
            return;
        };
        let (size, align) = Self::adjust(layout);
        if size <= MAX_QUICK_SIZE && align == size {
            let k: usize = size.ilog2() as usize - ALLOC_UNIT_SHIFT;
            // The header lives in the side table, never in
            // the freed block — the pointer model's Header-alignment
            // assert on the block is moot here (no header is written
            // into block memory at all).
            let i = self.take_slot(true);
            self.slots[i as usize] =
                Slot { next: self.qlists[k].take(), addr, size, align, used: true };
            self.qlists[k] = Some(i);
        } else {
            self.free_misc(addr, size, align);
        }
    }

    /// Frees a block to the misc list.  This looks up the given
    /// address in the hash of allocated misc blocks to find its
    /// header.
    ///
    /// If the block header is not found in the hash table, the block
    /// was allocated from the tail and this is its first free: take a
    /// fresh side-table slot for it (the reserve when the main table is
    /// empty — the emergency carve's index equivalent; the
    /// loud panic when both are empty).
    ///
    /// The carve-from-the-freed-block variant is GONE: headers are
    /// table-resident, so a freed misc block is always freed WHOLE
    /// (the carve shrank it by a header's width — a semantic cleanup
    /// the pin tests never depended on).
    fn free_misc(&mut self, addr: u32, size: usize, align: usize) {
        let i = match self.unlink_allocated_misc(addr) {
            // Reuse the block's registered header — its size/align are
            // the FIRST free's record, preserved across the misc→hash→
            // misc journey exactly like the pointer model (the reused
            // header's fields were never rewritten).
            Some(i) => i,
            None => {
                let i = self.take_slot(true);
                self.slots[i as usize].size = size;
                self.slots[i as usize].align = align;
                i
            }
        };
        let slot = &mut self.slots[i as usize];
        slot.addr = addr;
        slot.next = self.misc.take();
        slot.used = true;
        self.misc = Some(i);
    }

    /// Unlinks the header for the given address from the hash
    /// table for allocated misc blocks and returns it, if such
    /// a header exists.  If the block associated with the
    /// address has not been freed yet, it's possible that no
    /// header for it exists yet, in which case we return None.
    fn unlink_allocated_misc(&mut self, addr: u32) -> Option<Idx> {
        let k = Self::hash(addr);
        let (node, list) = Self::unlink(&mut self.slots, self.allocated_misc[k].take(), |n| {
            n.addr == addr
        });
        self.allocated_misc[k] = list;
        node
    }

    /// Unlinks the first node matching the given predicate from
    /// the given list, if it exists, returning the node, or
    /// None, and the list head.  The list head will be None if
    /// the list is empty.
    fn unlink<F>(
        slots: &mut [Slot; HEADER_TOTAL],
        list: Option<Idx>,
        predicate: F,
    ) -> (Option<Idx>, Option<Idx>)
    where
        F: Fn(&Slot) -> bool,
    {
        let mut prev: Option<Idx> = None;
        let head = list;
        let mut cur = list;
        // Head-preserving mid-list removal (splice
        // prev→next, return the ORIGINAL head), head removal returns the
        // next node, and a no-match walk returns the list UNCHANGED
        // (never wiped).  The tripwire stays as a permanent guard while
        // the gate's latent-wedge hunt continues: a cycled chain panics
        // loudly instead of hanging the kernel silently.  (A valid
        // acyclic chain visits each of the HEADER_TOTAL slots at most
        // once; the 100_000 bound is the pointer model's verbatim one —
        // far past any legit chain, never reached in practice.)
        let mut spins = 0usize;
        while let Some(i) = cur {
            spins += 1;
            if spins > 100_000 {
                let n = slots[i as usize];
                crate::println!("allocator: unlink spin — free-list cycle (slot {} addr {} size {} align {})", i, n.addr, n.size, n.align);
                panic!("allocator: free-list cycle detected");
            }
            let (matches, next) = {
                let node = &mut slots[i as usize];
                (predicate(node), node.next)
            };
            if matches {
                let next = slots[i as usize].next.take();
                match prev {
                    // Mid-list removal: splice prev→next; head unchanged.
                    Some(p) => {
                        slots[p as usize].next = next;
                        return (Some(i), head);
                    }
                    // Head removal: the next node becomes the head.
                    None => return (Some(i), next),
                }
            }
            prev = Some(i);
            cur = next;
        }
        // No match: the list is returned UNCHANGED (never wiped).
        (None, head)
    }

    /// Hashes an arena offset.  This is the bit mixing algorithm
    /// from Murmur3 (the pointer model hashed the pointer; the offset
    /// mixes identically — the choice only affects lookup speed).
    fn hash(addr: u32) -> usize {
        let mut k = addr as usize;
        k ^= k >> 33;
        k = k.wrapping_mul(0xff51afd7ed558ccd);
        k ^= k >> 33;
        k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
        (k >> 33) % NUM_HASH_BUCKETS
    }

    /// The test hook (cfg(test) only, port's crate-root test
    /// module): splice a CYCLE into the misc list (its last node's next
    /// back to the head) — pure index writes, safe by construction.  The
    /// tripwire's no-fit walk then spins and panics loudly (never the
    /// wedge).
    #[cfg(test)]
    pub(crate) fn test_hook_cycle_misc(&mut self) {
        let Some(head) = self.misc else {
            return;
        };
        let mut last = head;
        while let Some(next) = self.slots[last as usize].next {
            last = next;
        }
        self.slots[last as usize].next = Some(head);
    }
}

/// The GlobalAlloc shim's re-export (the shim's unsafe lives
/// in the sibling `globalalloc` module so THIS module is unsafe-free;
/// the `port::allocator::global::...` path is preserved for the
/// aarch64/allocator.rs static init).
#[cfg(not(test))]
pub mod global {
    pub use crate::globalalloc::{GlobalHeap, GlobalQuickAlloc};
}

// Diagnostic surface (15-03, the services-venue oom hunt): read-only
// arena stats off the live QuickFit.  tail.cursor is the bump
// high-water mark (bytes carved from the arena's front, irreversibly
// for blocks still allocated AND for the never-reused prefix holes);
// arena.len() is the fixed GLOBAL_HEAP_SIZE.  The difference is NOT
// "free" in a bump sense (freed blocks recycle through the quick/misc
// lists, and the cursor only ever grows) — the pair still localizes a
// leak: a cursor that climbs with every proc while live allocations
// stay flat names a permanent-burn bug's slope.
impl QuickFit {
    /// The bump cursor (bytes carved from the arena front so far) and
    /// the arena size.  Read-only; single CPU this era.
    pub fn arena_stats(&self) -> (usize, usize) {
        (self.tail.cursor.load(core::sync::atomic::Ordering::Relaxed), self.tail.arena.len())
    }
}
