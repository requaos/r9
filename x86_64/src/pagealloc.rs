/// This module is the arch-side face of the portable Roaring frame
/// allocator (`port::palloc`, MEM-07/MEM-15/D-14/D-17) — the x86_64
/// analog of `aarch64/src/pagealloc.rs`, with the firmware data source
/// re-keyed from the DTB to `platform::q35::mb2::Facts`.
///
/// The portable `PageAlloc` owns the clean/dirty/active/pinned pools, the
/// High-Water Pager (~15% clean), the per-frame pin counts and the
/// rank-safe `alloc_clean` reclaim-and-retry loop.  This module:
///
/// 1. seeds the pools from the mb2 type-1 map (honouring an optional
///    `palloc_max_frames=<N>` bootarg for the small-memory pressure gate),
/// 2. supplies the frame-zeroing callback (the boot tables' shared KZERO
///    alias already maps every guest frame, so no per-frame map is needed),
/// 3. wires the live evictor callback (`crate::proc::evict_re_readable_pages`),
///    and
/// 4. creates the live `palloc.hash` image page cache (`port::pagecache`),
///    published through a `'static` handle the fault and proc paths share.
///
/// The prohibition the construction honors: the allocator is seeded ONLY
/// from the map's type-1 (available) entries.  Type-3 ACPI-reclaimable
/// ranges, type-2/4 reserved/NVS ranges, and undeclared holes all stay OUT
/// of the free pool (13-04 walks ACPI tables inside type-3 ranges).
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use port::mem::{PhysAddr, PhysRange};
use port::pagealloc::PageAllocError;
use port::palloc::{self as palloc, FRAME_SIZE, PageAlloc};
use port::pagecache::PageCache;

use crate::platform::q35::mb2::Facts;

/// TPHT home blocks backing `palloc.hash` (DEBT-04 bank sizing is applied
/// by `PageCache::new`).  Enough for the boot image's text/untouched-data
/// pages with headroom; the structure lives on the 32 MiB kernel heap.
const PAGE_CACHE_BLOCKS: usize = 1024;

/// The most type-1 ranges the seeding considers (mb2::MAX_RANGES).
const MAX_AVAIL: usize = 32;

/// The portable Roaring frame allocator (MEM-07).  Its internal pool state
/// runs under an `IrqSpinLock` at `LockRank::PageAlloc` (70).  Constructed
/// at boot because the Roaring/`Vec` pools are not `const`.
static PAGE_ALLOC: AtomicPtr<PageAlloc> = AtomicPtr::new(core::ptr::null_mut());

/// The live allocator (boot installs it before any `allocate_physpage`).
fn page_alloc() -> &'static PageAlloc {
    let ptr = PAGE_ALLOC.load(Ordering::Acquire);
    assert!(!ptr.is_null(), "pagealloc: allocate before init_page_allocator");
    // SAFETY: `init_page_allocator` stores a `Box::into_raw` pointer exactly
    // once at boot and never frees it; the `'static` borrow is valid.
    unsafe { &*ptr }
}

/// Frames handed out and not yet returned, for `usage_bytes()`.
static ALLOCATED_FRAMES: AtomicUsize = AtomicUsize::new(0);

/// One-shot boot marker guard (the marker prints after the console exists).
static PALLOC_INFO_LOGGED: AtomicBool = AtomicBool::new(false);

/// The live `palloc.hash` image page cache (MEM-16), set once at boot.
static PAGE_CACHE: AtomicPtr<PageCache> = AtomicPtr::new(core::ptr::null_mut());

/// The live image page cache, if boot created it.
pub fn page_cache() -> Option<&'static PageCache> {
    let ptr = PAGE_CACHE.load(Ordering::Acquire);
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the pointer was produced by `Box::into_raw` in
    // `init_page_allocator` and is published only once, before any user
    // process runs; it is never freed, so the `'static` borrow is valid.
    Some(unsafe { &*ptr })
}

/// Seed the Roaring pools from the mb2 type-1 map and create the image page
/// cache.  `kernel_phys_end` is the physical end of the kernel image (the
/// `end` link symbol's VA minus KZERO).  Everything from physical 0 up to
/// that end is marked used in one range.
///
/// Returns Err(OutOfBounds) when the map declares no type-1 range at all —
/// the caller prints that loudly (there is nothing to boot on).
/// Round a physical address down to a frame boundary.
fn floor_frame(addr: u64) -> u64 {
    addr & !(FRAME_SIZE as u64 - 1)
}

/// Round a physical address up to a frame boundary (saturating).
fn ceil_frame(addr: u64) -> u64 {
    floor_frame(addr.saturating_add(FRAME_SIZE as u64 - 1))
}

pub fn init_page_allocator(facts: &Facts, kernel_phys_end: u64) -> Result<(), PageAllocError> {
    // Collect and sort the type-1 ranges: GRUB passes the BIOS e820 order
    // through, which is NOT address-sorted.
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

    // The seeding span: first type-1 start to last type-1 end.
    let span_start = avail[0].0;
    let mut span_end = avail[0].1;
    for r in &avail[1..navail] {
        if r.1 > span_end {
            span_end = r.1;
        }
    }
    // The palloc contract is PAGE-GRANULAR ranges: every span and used
    // range must be FRAME_SIZE-aligned (init_ranges returns
    // MisalignedAddr otherwise).  The firmware map is byte-accurate — the
    // first type-1 range ends at 0x9FC00 (the EBDA boundary), which is not
    // a page multiple — so align at the boundary: floor the available
    // span's ends (never claim a partial page is fully free) and grow
    // every used range outward (never leave a used byte in a free page).
    // Conservative in both directions, exact in page terms.
    let span_start = floor_frame(span_start);
    let span_end = floor_frame(span_end);
    if span_end <= span_start {
        return Err(PageAllocError::OutOfBounds);
    }
    let span = PhysRange::with_end(span_start, span_end);

    // The used set, sorted: [0, kernel_phys_end) first, then the gaps
    // between the type-1 ranges (every non-type-1 entry and undeclared hole
    // inside the span lands in a gap — that keeps them out of the pool).
    let mut used: [PhysRange; MAX_AVAIL + 1] = core::array::from_fn(|_| PhysRange::with_len(0, 0));
    used[0] = PhysRange::with_end(0, ceil_frame(kernel_phys_end));
    let mut nused = 1;
    let mut prev_end = span_start;
    for r in &avail[..navail] {
        let (start, end) = *r;
        if start > prev_end {
            if nused == MAX_AVAIL + 1 {
                return Err(PageAllocError::OutOfBounds);
            }
            used[nused] = PhysRange::with_end(floor_frame(prev_end), ceil_frame(start));
            nused += 1;
        }
        if end > prev_end {
            prev_end = end;
        }
    }

    let max_frames = max_frames_from_facts(facts);
    let alloc = alloc::boxed::Box::new(PageAlloc::new());
    alloc.init_ranges(span, used[..nused].iter().cloned(), max_frames)?;
    PAGE_ALLOC.store(alloc::boxed::Box::into_raw(alloc), Ordering::Release);

    // The High-Water Pager: seed ~15% clean before the first allocation.
    let _ = page_alloc().refill_clean_pool(zero_frame);

    // The Flattened-TPHT image page cache (MEM-16).  A failure leaves the
    // cache absent rather than aborting the boot; the fault path then
    // degrades to private pages.
    if let Ok(cache) = PageCache::new(PAGE_CACHE_BLOCKS) {
        let raw = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(cache));
        PAGE_CACHE.store(raw, Ordering::Release);
    }

    Ok(())
}

/// Parse `palloc_max_frames=<N>` from the mb2 command line (the small-memory
/// pressure gate).  An absent or malformed token leaves the allocator
/// managing all usable RAM.
fn max_frames_from_facts(facts: &Facts) -> Option<usize> {
    core::str::from_utf8(facts.cmdline())
        .ok()
        .and_then(palloc::parse_max_frames_bootarg)
}

/// Allocate one zeroed physical page.  Reclaims re-readable pages and retries
/// (bounded inside `alloc_clean`) instead of panicking under pressure.
pub fn allocate_physpage() -> Result<PhysAddr, PageAllocError> {
    let pa = page_alloc().alloc_clean(zero_frame, || crate::proc::evict_re_readable_pages(1))?;
    ALLOCATED_FRAMES.fetch_add(1, Ordering::Relaxed);
    Ok(pa)
}

/// Return a physical page to the allocator's dirty pool.
pub fn deallocate_physpage(pa: PhysAddr) -> Result<(), PageAllocError> {
    page_alloc().free_frame(pa)?;
    ALLOCATED_FRAMES.fetch_sub(1, Ordering::Relaxed);
    Ok(())
}

/// Pin `pfn` across a sleep (MEM-10, D-21).
pub fn pin_frame(pfn: u32) -> Result<(), PageAllocError> {
    page_alloc().pin_frame(pfn)
}

/// Release one pin of `pfn` (RAII `PinGuard` Drop).
pub fn unpin_frame(pfn: u32) -> Result<(), PageAllocError> {
    page_alloc().unpin_frame(pfn)
}

/// Whether a physical page is pinned.  Legal from `PAGE_CACHE` (Rank 20):
/// ascending to `PAGE_ALLOC` (70) is the documented order.
pub fn is_pinned(pa: PhysAddr) -> bool {
    let ptr = PAGE_ALLOC.load(Ordering::Acquire);
    if ptr.is_null() {
        return false;
    }
    match palloc::phys_to_pfn(pa) {
        Ok(pfn) => page_alloc().is_pinned(pfn),
        Err(_) => false,
    }
}

/// Zero one dirty frame through the boot tables' shared KZERO alias.  The
/// identity map covers every guest frame (the 1 GiB profile window), so this
/// needs no page-table edit; `phys_ptr_mut` asserts the window bound loudly.
fn zero_frame(pa: PhysAddr) {
    // SAFETY: `pa` is a managed frame below the KZERO window (the 1 GiB q35
    // guest); writing PAGE_SIZE_4K zero bytes is the zero operation.
    unsafe { core::ptr::write_bytes(crate::proc::phys_ptr_mut(pa.addr()), 0, FRAME_SIZE) };
}

/// Return a tuple of (bytes used, total bytes available) based on the page
/// allocator, and emit the one-shot `palloc:` boot marker after the console
/// is live.
pub fn usage_bytes() -> (usize, usize) {
    let used_frames = ALLOCATED_FRAMES.load(Ordering::Relaxed);
    let total_frames = page_alloc().total_managed();
    let low_water = page_alloc().low_water_mark();
    // No allocator lock is held here: each accessor takes and releases
    // PAGE_ALLOC (Rank 70) before the UART write.
    if !PALLOC_INFO_LOGGED.swap(true, Ordering::AcqRel) {
        port::println!("palloc: managed_frames={total_frames} low_water={low_water}");
    }
    (used_frames.saturating_mul(FRAME_SIZE), total_frames.saturating_mul(FRAME_SIZE))
}
