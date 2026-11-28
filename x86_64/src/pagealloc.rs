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
/// 2. builds the kernel's direct map of the RAM above the first gibibyte
///    (directmap.rs) from a low-RAM reservation, before the seeding, and
///    withholds the RAM the map cannot reach,
/// 3. supplies the frame-zeroing callback (every guest frame is reachable
///    once the map exists, so no per-frame mapping is needed),
/// 4. wires the live evictor callback (`crate::proc::evict_re_readable_pages`),
///    and
/// 5. creates the live `palloc.hash` image page cache (`port::pagecache`),
///    published through a `'static` handle the fault and proc paths share.
///
/// The prohibition the construction honors: the allocator is seeded ONLY
/// from the map's type-1 (available) entries.  Type-3 ACPI-reclaimable
/// ranges, type-2/4 reserved/NVS ranges, and undeclared holes all stay OUT
/// of the free pool (13-04 walks ACPI tables inside type-3 ranges).
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use port::mem::{PhysAddr, PhysRange};
use port::pagealloc::PageAllocError;
use port::palloc::{self as palloc, FRAME_SIZE, PageAlloc};
use port::pagecache::PageCache;

use crate::platform::q35::mb2::{self, Facts};

/// TPHT home blocks backing `palloc.hash` (DEBT-04 bank sizing is applied
/// by `PageCache::new`).  Enough for the boot image's text/untouched-data
/// pages with headroom; the structure lives on the 32 MiB kernel heap.
const PAGE_CACHE_BLOCKS: usize = 1024;

/// The most type-1 ranges the seeding considers: the firmware map's own
/// cap (mb2::MAX_RANGES).
const MAX_AVAIL: usize = mb2::MAX_RANGES;

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

/// Why the seeding failed: the frame allocator's error, and — when the
/// direct map is what failed — its own words.  The caller prints it once
/// the console is up (the seeding runs before anything can print).
#[derive(Debug)]
pub struct InitError {
    kind: PageAllocError,
    detail: Option<&'static str>,
}

impl InitError {
    fn alloc(kind: PageAllocError) -> InitError {
        InitError { kind, detail: None }
    }
}

impl core::fmt::Display for InitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.kind)?;
        if let Some(detail) = self.detail {
            write!(f, ": {detail}")?;
        }
        Ok(())
    }
}

/// Round a physical address down to a frame boundary.
fn floor_frame(addr: u64) -> u64 {
    addr & !(FRAME_SIZE as u64 - 1)
}

/// Round a physical address up to a frame boundary (saturating).
fn ceil_frame(addr: u64) -> u64 {
    floor_frame(addr.saturating_add(FRAME_SIZE as u64 - 1))
}

/// A cursor over low RAM for the kernel's own boot reservations.  Each
/// one takes the lowest span that is aligned as asked, lies inside ONE
/// available range, starts at or above the kernel image and every
/// earlier reservation, and stays below the first gibibyte — the RAM
/// the boot tables already reach at KZERO, so a reservation needs no
/// page table of its own.  None when no range has room: the boot then
/// fails loudly rather than starting without it.
struct LowRam<'a> {
    avail: &'a [(u64, u64)],
    next: u64,
}

impl<'a> LowRam<'a> {
    fn new(avail: &'a [(u64, u64)], kernel_phys_end: u64) -> LowRam<'a> {
        LowRam { avail, next: ceil_frame(kernel_phys_end) }
    }

    /// Reserve `bytes` at an address that is a multiple of `align`;
    /// the span's start.
    fn reserve(&mut self, bytes: u64, align: u64) -> Option<u64> {
        let start = self.avail.iter().find_map(|&(lo, end)| {
            let start = lo.max(self.next).div_ceil(align) * align;
            let end = end.min(crate::proc::KZERO_MAP_LIMIT);
            start.checked_add(bytes).filter(|&top| top <= end).map(|_| start)
        })?;
        self.next = start + bytes;
        Some(start)
    }
}

/// Seed the Roaring pools from the mb2 type-1 map, build the direct map
/// of the RAM above the first gibibyte, and create the image page cache.
/// `kernel_phys_end` is the physical end of the kernel image (the `end`
/// link symbol's VA minus KZERO).  Everything from physical 0 up to that
/// end is marked used in one range.
///
/// Returns Err when the map declares no type-1 range at all, or when a
/// kernel reservation or the direct map cannot be made — the caller
/// prints that loudly once the console is up (there is nothing to boot
/// on).
pub fn init_page_allocator(facts: &Facts, kernel_phys_end: u64) -> Result<(), InitError> {
    // Collect and sort the type-1 ranges: GRUB passes the BIOS e820 order
    // through, which is NOT address-sorted.
    let mut avail = [(0u64, 0u64); MAX_AVAIL];
    let mut navail = 0;
    for r in facts.available_ranges() {
        if navail == MAX_AVAIL {
            return Err(InitError::alloc(PageAllocError::OutOfBounds));
        }
        avail[navail] = (r.base, r.base + r.len);
        navail += 1;
    }
    if navail == 0 {
        return Err(InitError::alloc(PageAllocError::OutOfBounds));
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
        return Err(InitError::alloc(PageAllocError::OutOfBounds));
    }
    let span = PhysRange::with_end(span_start, span_end);

    // The kernel's own reservations in low RAM, in order: the
    // kernel-stack arena (kstack.rs: MAX_PROCS contiguous slots,
    // withheld from the pools like the kernel image; 2 MiB-aligned so
    // the kernel alias can be split around it and a guard page left
    // below every stack), the page-table pages that split takes, then
    // the direct map's table pages — bounded before anything is built,
    // so exactly what each takes is withheld below and the rest returns
    // to the pools.
    let mut low = LowRam::new(&avail[..navail], kernel_phys_end);
    let arena_bytes = crate::kstack::arena_bytes();
    let arena_base = low
        .reserve(arena_bytes, 2 << 20)
        .ok_or(PageAllocError::OutOfBounds)
        .map_err(InitError::alloc)?;
    let guard_bytes = crate::directmap::guard_table_pages(arena_bytes) as u64 * FRAME_SIZE as u64;
    let guard_lo = low
        .reserve(guard_bytes, FRAME_SIZE as u64)
        .ok_or(PageAllocError::OutOfBounds)
        .map_err(InitError::alloc)?;
    let table_bytes = crate::directmap::table_pages_bound(navail, span_end) as u64 * FRAME_SIZE as u64;
    let table_lo = low
        .reserve(table_bytes, FRAME_SIZE as u64)
        .ok_or(PageAllocError::OutOfBounds)
        .map_err(InitError::alloc)?;

    // The direct map of the RAM above the first gibibyte (directmap.rs),
    // built BEFORE the seeding: its table pages come from the
    // reservation above, and the RAM it cannot reach is withheld below
    // like the kernel image — a frame is never handed out that the
    // kernel has no address for.
    #[cfg(not(test))]
    let (beyond_map, table_pages) = crate::directmap::init(
        crate::proc::boot_pml4_pa(),
        &avail[..navail],
        (table_lo, table_lo + table_bytes),
    )
    .map_err(|detail| InitError { kind: PageAllocError::OutOfBounds, detail: Some(detail) })?;
    #[cfg(test)]
    let (beyond_map, table_pages): (alloc::vec::Vec<PhysRange>, usize) = (alloc::vec::Vec::new(), 0);

    // The used set: [0, kernel_phys_end) first, then the gaps between the
    // type-1 ranges (every non-type-1 entry and undeclared hole inside the
    // span lands in a gap — that keeps them out of the pool), then the
    // kernel-stack arena, the table pages the map took, and the RAM the
    // map cannot reach.
    let mut used: Vec<PhysRange> = Vec::new();
    used.push(PhysRange::with_end(0, ceil_frame(kernel_phys_end)));
    let mut prev_end = span_start;
    for r in &avail[..navail] {
        let (start, end) = *r;
        if start > prev_end {
            used.push(PhysRange::with_end(floor_frame(prev_end), ceil_frame(start)));
        }
        if end > prev_end {
            prev_end = end;
        }
    }
    used.push(PhysRange::with_end(arena_base, arena_base + arena_bytes));
    used.push(PhysRange::with_end(guard_lo, guard_lo + guard_bytes));
    used.push(PhysRange::with_end(table_lo, table_lo + table_pages as u64 * FRAME_SIZE as u64));
    for r in &beyond_map {
        used.push(PhysRange::with_end(floor_frame(r.start().addr()), ceil_frame(r.end().addr())));
    }

    let max_frames = max_frames_from_facts(facts);
    let alloc = alloc::boxed::Box::new(PageAlloc::new());
    alloc.init_ranges(span, used.iter().cloned(), max_frames).map_err(InitError::alloc)?;
    PAGE_ALLOC.store(alloc::boxed::Box::into_raw(alloc), Ordering::Release);
    // The guard page below every kernel stack, then the arena itself
    // (its zeroing runs through the split alias: the proof every page
    // but the guards is still reachable).
    #[cfg(not(test))]
    crate::kstack::arm(arena_base, (guard_lo, guard_lo + guard_bytes))
        .map_err(|detail| InitError { kind: PageAllocError::OutOfBounds, detail: Some(detail) })?;
    #[cfg(not(test))]
    crate::kstack::init(arena_base);

    // The High-Water Pager: seed the clean pool to its target before the
    // first allocation.
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

/// The live pool cardinalities for `#c/sysstat` (diagnostics only).  Takes
/// `PAGE_ALLOC` briefly, so callers must not hold it.
pub fn pool_snapshot() -> port::devcons::PoolSnapshot {
    let (clean, dirty, active, pinned) = page_alloc().pool_lens();
    port::devcons::PoolSnapshot { clean, dirty, active, pinned }
}

/// One-shot markers for a large-memory leg's log: the first frame the
/// kernel hands out above 1 GiB (the direct map's first use) and the
/// first above 4 GiB.
static FIRST_ABOVE_1G: AtomicBool = AtomicBool::new(false);
static FIRST_ABOVE_4G: AtomicBool = AtomicBool::new(false);

/// Allocate one zeroed physical page.  Reclaims re-readable pages and retries
/// (bounded inside `alloc_clean`) instead of panicking under pressure.
pub fn allocate_physpage() -> Result<PhysAddr, PageAllocError> {
    let pa = page_alloc().alloc_clean(zero_frame, crate::proc::evict_re_readable_pages)?;
    ALLOCATED_FRAMES.fetch_add(1, Ordering::Relaxed);
    // Printed with no lock held: the pool's went back inside alloc_clean.
    let addr = pa.addr();
    let marker = if addr >= 1u64 << 32 {
        (!FIRST_ABOVE_4G.swap(true, Ordering::Relaxed))
            .then_some("palloc: first frame above 4 GiB handed out")
    } else if addr >= crate::proc::KZERO_MAP_LIMIT {
        (!FIRST_ABOVE_1G.swap(true, Ordering::Relaxed))
            .then_some("palloc: first frame above 1 GiB handed out")
    } else {
        None
    };
    if let Some(line) = marker {
        port::println!("{line}: {addr:#x}");
    }
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

/// Zero one dirty frame through the kernel's map of RAM (the KZERO alias
/// below the first gibibyte, the direct map above it), so this needs no
/// page-table edit; `phys_ptr_mut` refuses a frame that is in neither.
fn zero_frame(pa: PhysAddr) {
    // SAFETY: `pa` is a managed frame, and the seeding withheld every
    // frame the kernel cannot reach; writing FRAME_SIZE zero bytes is the
    // zero operation.
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

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{LowRam, MAX_AVAIL};
    use crate::directmap::{guard_table_pages, table_pages_bound};
    use crate::kstack::SLOT_BYTES;

    /// The arena's reservation alignment: 2 MiB, so the kernel alias can
    /// be split around it.
    const SLOT: u64 = 2 << 20;
    const K: u64 = 4096;

    /// The gate guest's map: the 640 KiB low range, then everything
    /// from 1 MiB to just under 1 GiB.
    fn qavail() -> Vec<(u64, u64)> {
        vec![(0, 0x9fc00), (0x100000, 0x3ffdf000)]
    }

    #[test]
    fn the_stack_arena_goes_above_the_kernel_image_in_2mib_pages() {
        let avail = qavail();
        let mut low = LowRam::new(&avail, 0xcb61001);
        let base = low.reserve(8 << 20, SLOT).unwrap();
        assert_eq!(base % SLOT, 0, "2 MiB-aligned, so the kernel alias can be split around it");
        assert!(base >= 0xcb61001, "never over the kernel image");
        assert_eq!(base, 0x0cc0_0000, "the lowest 2 MiB boundary past the image");
        // The low 640 KiB range has no room above a 200 MiB image.
    }

    #[test]
    fn a_reservation_skips_a_range_that_cannot_hold_it() {
        let avail = [(0x100000, 0x200000), (0x4000000, 0x8000000)];
        let base = LowRam::new(&avail, 0x180000).reserve(8 << 20, SLOT).unwrap();
        assert_eq!(base, 0x4000000, "the first range has 512 KiB; the second holds it");
    }

    #[test]
    fn a_reservation_stays_below_the_alias_ceiling() {
        // A range running past the first gibibyte (0x4000_0000) that the
        // boot tables reach at KZERO: the reservation must fit in the
        // covered part, or fail.
        let avail = [(0x3e000000u64, 0x140000000u64)];
        assert_eq!(LowRam::new(&avail, 0x100000).reserve(16 << 20, SLOT), Some(0x3e000000));
        // A range with only 4 MiB below the ceiling: it would fit
        // without the ceiling.
        let avail = [(0x3fc00000u64, 0x140000000u64)];
        assert!(
            LowRam::new(&avail, 0x100000).reserve(8 << 20, SLOT).is_none(),
            "only 4 MiB is covered"
        );
    }

    #[test]
    fn a_reservation_fails_loudly_when_nothing_fits() {
        assert!(LowRam::new(&[], 0x100000).reserve(8 << 20, SLOT).is_none());
        assert!(LowRam::new(&[(0x100000, 0x200000)], 0x100000).reserve(8 << 20, SLOT).is_none());
    }

    #[test]
    fn reservations_never_overlap_and_follow_the_alignment_asked() {
        // The arena, then page-aligned table pages, then another
        // arena-aligned span: each starts where the last ended, rounded
        // up.
        let avail = [(0x100000u64, 0x4000_0000u64)];
        let mut low = LowRam::new(&avail, 0xcb61001);
        let arena = low.reserve(8 << 20, SLOT).unwrap();
        assert_eq!(arena, 0x0cc0_0000);
        let tables = low.reserve(5 * K, K).unwrap();
        assert_eq!(tables, 0x0d40_0000, "the page after the arena's last");
        let next = low.reserve(SLOT, SLOT).unwrap();
        assert_eq!(next, 0x0d60_0000, "after the five pages, rounded up to 2 MiB");
        // The kernel image itself is never handed out.
        let mut low = LowRam::new(&avail, 0x4000_0000 - K);
        assert!(low.reserve(2 * K, K).is_none(), "one page is left above a kernel ending a page short of the gibibyte");
        assert_eq!(low.reserve(K, K), Some(0x3fff_f000));
    }

    #[test]
    fn the_guard_table_pages_are_one_pd_and_one_pt_per_2mib_of_arena() {
        let bytes = crate::kstack::arena_bytes();
        assert!(bytes.is_multiple_of(2 << 20), "the arena is whole 2 MiB pages");
        assert_eq!(guard_table_pages(bytes), 1 + (bytes >> 21) as usize);
    }

    #[test]
    fn the_table_page_bound_is_two_per_range_plus_one_per_512g() {
        // It only ever bounds a build (directmap.rs pins it against real
        // builds); here: two ranges, RAM under 512 GiB.
        assert_eq!(table_pages_bound(2, 0x19_8000_0000), 5);
        assert_eq!(table_pages_bound(MAX_AVAIL, 512 << 30), 2 * MAX_AVAIL + 2);
    }
}
