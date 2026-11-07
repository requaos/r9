/// This module acts as the interface between the portable bitmap page
/// allocator and the mb2 memory map — the x86_64 analog of
/// aarch64/src/pagealloc.rs, with the firmware data source re-keyed from
/// the DTB to `platform::q35::mb2::Facts`.
///
/// The prohibition the construction honors: the allocator is seeded ONLY
/// from the map's type-1 (available) entries.  The free set handed to
/// `free_unused_ranges` is built as the exact union of the type-1 ranges
/// minus the kernel image (and everything below it) — type-3 ACPI-
/// reclaimable ranges, type-2/4 reserved/NVS ranges, and undeclared holes
/// all stay OUT of the free pool (13-04 walks ACPI tables inside type-3
/// ranges; a compiled-in RAM size or base address never appears).
use port::bitmapalloc::BitmapPageAlloc;
use port::mem::{PhysAddr, PhysRange, PAGE_SIZE_4K};
use port::mcslock::{Lock, LockNode};
use port::pagealloc::PageAllocError;

use crate::platform::q35::mb2::Facts;

/// Set up the bitmap page allocator assuming everything is allocated.
/// 32 bitmaps of 4 KiB cover 4 GiB of physical memory.
static PAGE_ALLOC: Lock<BitmapPageAlloc<32, PAGE_SIZE_4K>> = Lock::new(
    "page_alloc",
    const { BitmapPageAlloc::<32, PAGE_SIZE_4K>::new_all_allocated(PAGE_SIZE_4K) },
);

/// The most type-1 ranges the seeding considers (mb2::MAX_RANGES).
const MAX_AVAIL: usize = 32;

/// Seed the allocator: `kernel_phys_end` is the physical end of the
/// kernel image (the `end` link symbol's VA minus KZERO).  Everything
/// from physical 0 up to that end is marked used in one range — it
/// covers the BIOS/EBDA low memory, the Mach at physical 1..2 MiB,
/// wherever GRUB stashed the mb2 info (observed at 0x16dc08 — inside
/// the Mach mebibyte), and the kernel image itself.
///
/// Returns Err(OutOfBounds) when the map declares no type-1 range at
/// all — the caller prints that loudly (there is nothing to boot on).
pub fn init_page_allocator(facts: &Facts, kernel_phys_end: u64) -> Result<(), PageAllocError> {
    // Collect and sort the type-1 ranges: GRUB passes the BIOS e820
    // order through, which is NOT address-sorted (the live 13-02 map
    // carries the 0xb0000000 ECAM reservation after 0x3ffdf000).
    let mut avail = [(0u64, 0u64); MAX_AVAIL];
    let mut navail = 0;
    for r in facts.available_ranges() {
        if navail == MAX_AVAIL {
            return Err(PageAllocError::OutOfBounds);
        }
        avail[navail] = (r.base, r.base + r.len);
        navail += 1;
    }
    if navail == 0 {
        return Err(PageAllocError::OutOfBounds);
    }
    avail[..navail].sort_unstable_by_key(|r| r.0);

    // The seeding span: first type-1 start to last type-1 end.  Merging
    // overlaps as we go (a well-formed map has none; a sloppy one must
    // not wedge the walk).
    let span_start = avail[0].0;
    let mut span_end = avail[0].1;
    for r in &avail[1..navail] {
        if r.1 > span_end {
            span_end = r.1;
        }
    }
    let span = PhysRange::with_end(span_start, span_end);

    // The used set, sorted: [0, kernel_phys_end) first, then the gaps
    // between the type-1 ranges (every non-type-1 entry and undeclared
    // hole inside the span lands in a gap — that is exactly what keeps
    // them out of the free pool).
    let mut used: [PhysRange; MAX_AVAIL + 1] = core::array::from_fn(|_| PhysRange::with_len(0, 0));
    used[0] = PhysRange::with_end(0, kernel_phys_end);
    let mut nused = 1;
    let mut prev_end = span_start;
    for r in &avail[..navail] {
        let (start, end) = *r;
        if start > prev_end {
            if nused == MAX_AVAIL + 1 {
                return Err(PageAllocError::OutOfBounds);
            }
            used[nused] = PhysRange::with_end(prev_end, start);
            nused += 1;
        }
        if end > prev_end {
            prev_end = end;
        }
    }

    let node = LockNode::new();
    let mut lock = PAGE_ALLOC.lock(&node);
    let page_alloc = &mut *lock;
    page_alloc.free_unused_ranges(span, used[..nused].iter().cloned())
}

/// Return a tuple of (bytes used, total bytes available) based on the
/// page allocator.
pub fn usage_bytes() -> (usize, usize) {
    let node = LockNode::new();
    let mut lock = PAGE_ALLOC.lock(&node);
    let page_alloc = &mut *lock;
    page_alloc.usage_bytes()
}

/// Allocate one free physical page (the aarch64 allocate_physpage shape).
/// Note the page is NOT mapped anywhere — the identity map of the first
/// 4 GiB (l.S's boot PML4s) already covers every page this allocator can
/// hand out, so the returned PA doubles as its kernel VA.
pub fn allocate_physpage() -> Result<PhysAddr, PageAllocError> {
    let node = LockNode::new();
    let mut lock = PAGE_ALLOC.lock(&node);
    lock.allocate()
}
