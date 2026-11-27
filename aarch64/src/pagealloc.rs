/// This module is the arch-side face of the portable Roaring frame
/// allocator (`port::palloc`, MEM-07/MEM-15/D-14/D-17).
///
/// The portable `PageAlloc` owns the clean/dirty/active/pinned pools, the
/// High-Water Pager (~15% clean), the per-frame pin counts and the
/// rank-safe `alloc_clean` reclaim-and-retry loop.  This module:
///
/// 1. seeds the pools from the DTB memory map (honouring an optional
///    `palloc_max_frames=<N>` bootarg on `virt`), and
/// 2. supplies the frame-zeroing callback (through a reusable kernel alias)
///    and the live evictor callback (`crate::fault::evict_re_readable_pages`).
///
/// The live `palloc.hash` image page cache (`port::pagecache`) is created
/// here at boot and published through a `'static` handle the fault and proc
/// paths share.
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use crate::vm::Entry;
use crate::vm::PhysPageAllocator;
use crate::vm::RootPageTable;
use crate::vm::RootPageTableType;
use crate::vm::VaMapping;
use crate::vm::VirtPage4K;
use crate::vm::VmTraitImpl;
use port::fdt::DeviceTree;
use port::mem::PhysAddr;
use port::mem::PhysRange;
use port::pagealloc::PageAllocError;
use port::palloc::{self as palloc, FRAME_SIZE, PageAlloc};
use port::pagecache::PageCache;
use port::{
    mcslock::{Lock, LockNode},
    mem::PAGE_SIZE_4K,
};

#[cfg(not(test))]
use port::println;

/// TPHT home blocks backing `palloc.hash` (DEBT-04 bank sizing is applied by
/// `PageCache::new`).  Enough for the boot image's text/untouched-data pages
/// with headroom; the whole structure lives on the 4 MiB kernel heap.
const PAGE_CACHE_BLOCKS: usize = 1024;

/// The portable Roaring frame allocator (MEM-07).  Its internal pool state
/// runs under an `IrqSpinLock` at `LockRank::PageAlloc` (70).  Constructed at
/// boot because the Roaring/`Vec` pools are not `const`.
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

/// The reusable kernel VA used to zero/retarget one frame at a time.
static ZERO_ALIAS_VA: AtomicUsize = AtomicUsize::new(0);
/// 0 = uninitialised, 1 = initialising, 2 = ready.
static ZERO_ALIAS_STATE: AtomicUsize = AtomicUsize::new(0);
/// Serialises the single reusable alias between CPUs.  Unranked: the caller
/// already holds IRQs masked (syscall/trap) or is the single boot CPU, and
/// `PAGE_ALLOC` is never held across `zero_frame`.
static ZERO_ALIAS_LOCK: Lock<()> = Lock::new("palloc_zero_alias", ());

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

/// Seed the Roaring pools from the DTB and create the image page cache.
pub fn init_page_allocator(
    dt: &DeviceTree,
    used_ranges: impl Iterator<Item = PhysRange>,
) -> Result<(), PageAllocError> {
    let Some(available_mem) = dt
        .find_device_type("memory")
        .flat_map(|memory| dt.property_translated_reg_iter(memory).flat_map(|r| r.regblock()))
        .map(|memory| PhysRange::from(&memory))
        .next()
    else {
        panic!("error:pagealloc:free_unused_ranges: can't get memory from dtb\n");
    };

    let max_frames = max_frames_from_dtb(dt);
    let alloc = alloc::boxed::Box::new(PageAlloc::new());
    alloc.init_ranges(available_mem, used_ranges, max_frames)?;
    PAGE_ALLOC.store(alloc::boxed::Box::into_raw(alloc), Ordering::Release);

    // The High-Water Pager: seed ~15% clean before the first allocation.
    // `zero_frame` establishes the reusable alias on its first call.
    let _ = page_alloc().refill_clean_pool(zero_frame);

    // The Flattened-TPHT image page cache (MEM-16).  A failure is loud at
    // this point (the heap is live), but a 4 MiB arena could be tight on a
    // heavily loaded profile: leave the cache absent rather than abort the
    // boot, and the fault path degrades to private pages.
    if let Ok(cache) = PageCache::new(PAGE_CACHE_BLOCKS) {
        let raw = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(cache));
        PAGE_CACHE.store(raw, Ordering::Release);
    }

    Ok(())
}

/// Parse `palloc_max_frames=<N>` from DTB `/chosen/bootargs` on the `virt`
/// platform's dev_exec profiles (the small-memory pressure gate).  Every
/// other build leaves the allocator managing all physical RAM: `bootconf`
/// is deliberately gated to `#[cfg(all(platform = "virt", any(test,
/// dev_exec)))]`, so a non-virt `cargo xtask check` or a `--config virt`,
/// `virtpanic`, `virtlowecam` build must not name it.
#[cfg(all(platform = "virt", any(test, dev_exec)))]
fn max_frames_from_dtb(dt: &DeviceTree) -> Option<usize> {
    let bootargs = crate::platform::bootconf::read_bootargs(dt);
    core::str::from_utf8(bootargs.as_bytes())
        .ok()
        .and_then(palloc::parse_max_frames_bootarg)
}

#[cfg(not(all(platform = "virt", any(test, dev_exec))))]
fn max_frames_from_dtb(_dt: &DeviceTree) -> Option<usize> {
    None
}

/// Try to allocate a zeroed physical page.  Reclaims re-readable pages and
/// retries (bounded inside `alloc_clean`) instead of panicking under
/// pressure.
pub fn allocate_physpage() -> Result<PhysAddr, PageAllocError> {
    let pa = page_alloc().alloc_clean(zero_frame, || crate::fault::evict_re_readable_pages(1))?;
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
    match palloc::phys_to_pfn(pa) {
        Ok(pfn) => page_alloc().is_pinned(pfn),
        Err(_) => false,
    }
}

/// Zero one dirty frame through the reusable kernel alias.  Best-effort:
/// every consumer of `allocate_physpage` also zero-fills or overwrites its
/// page, so a failed alias setup can never expose stale data to userland.
fn zero_frame(pa: PhysAddr) {
    if ZERO_ALIAS_STATE.load(Ordering::Acquire) != 2 {
        // First call establishes the alias; a re-entrant call from the
        // page-table allocation inside it sees state 1 and only skips (the
        // frame's consumer zero-fills it anyway).
        let _ = ensure_zero_alias(pa);
    }
    let node = LockNode::new();
    let _guard = ZERO_ALIAS_LOCK.lock(&node);
    if ZERO_ALIAS_STATE.load(Ordering::Acquire) != 2 {
        return;
    }
    let va = ZERO_ALIAS_VA.load(Ordering::Relaxed);
    if crate::vm::retarget_kernel_leaf(va, pa).is_ok() {
        // SAFETY: `va` now maps `pa` as one writable 4K kernel page;
        // writing PAGE_SIZE_4K zero bytes is the zero operation.
        unsafe { core::ptr::write_bytes(va as *mut u8, 0, PAGE_SIZE_4K) };
    }
}

/// Establish the one-time reusable alias to `anchor`.  Re-entrant calls
/// (page-table allocation during the mapping) see state 1 and skip.
fn ensure_zero_alias(anchor: PhysAddr) -> Option<usize> {
    if ZERO_ALIAS_STATE
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return None;
    }
    let va = crate::vm::next_free_device_page4k().map(anchor);
    let mut physalloc = PhysPageAllocator {};
    let mut vmt = VmTraitImpl {};
    let mapped = crate::vm::kernel_pagetable().map_phys_range(
        &mut physalloc,
        &mut vmt,
        "palloczero",
        &PhysRange::with_pa_len(anchor, PAGE_SIZE_4K),
        VaMapping::Addr(va),
        Entry::rw_kernel_data(),
        crate::vm::PageSize::Page4K,
        RootPageTableType::Kernel,
    );
    if mapped.is_ok() {
        ZERO_ALIAS_VA.store(va, Ordering::Release);
        ZERO_ALIAS_STATE.store(2, Ordering::Release);
        Some(va)
    } else {
        ZERO_ALIAS_STATE.store(0, Ordering::Release);
        None
    }
}

/// A guard handle to a freshly allocated+mapped 4 KiB virtual page
/// (allocate_virtpage never hands out a `&'static mut`).
/// Deref/DerefMut reach the page for the guard's lifetime; the mapping is
/// permanent this era (no unmap path), so the handle's target stays valid.
pub struct VirtPage4KGuard(*mut VirtPage4K);

impl core::ops::Deref for VirtPage4KGuard {
    type Target = VirtPage4K;

    fn deref(&self) -> &VirtPage4K {
        // SAFETY: the pointer names a page allocate_virtpage just mapped
        // r/w in the named root table; the mapping is never torn down this
        // era, and the caller treats the guard as the page's unique handle.
        unsafe { &*self.0 }
    }
}

impl core::ops::DerefMut for VirtPage4KGuard {
    fn deref_mut(&mut self) -> &mut VirtPage4K {
        // SAFETY: Deref's invariant.
        unsafe { &mut *self.0 }
    }
}

/// Try to allocate a physical page and map it into virtual memory at va.
pub fn allocate_virtpage(
    page_table: &mut RootPageTable,
    debug_name: &str,
    entry: Entry,
    va: VaMapping,
    pgtype: RootPageTableType,
) -> Result<VirtPage4KGuard, PageAllocError> {
    let page_pa = allocate_physpage()?;
    let range = PhysRange::with_pa_len(page_pa, PAGE_SIZE_4K);

    let mut physpage_allocator = PhysPageAllocator {};
    let mut vmtrait_impl = VmTraitImpl {};

    if let Ok(page_va) = page_table.map_phys_range(
        &mut physpage_allocator,
        &mut vmtrait_impl,
        debug_name,
        &range,
        va,
        entry,
        crate::vm::PageSize::Page4K,
        pgtype,
    ) {
        Ok(VirtPage4KGuard(page_va.start() as *mut VirtPage4K))
    } else {
        println!("error:pagealloc:allocate_virtpage:unable to map");
        Err(PageAllocError::UnableToMap)
    }
}

/// Return a tuple of (bytes used, total bytes available) based on the page
/// allocator, and emit the one-shot `palloc:` boot marker after the console
/// is live.
pub fn usage_bytes() -> (usize, usize) {
    let used_frames = ALLOCATED_FRAMES.load(Ordering::Relaxed);
    let total_frames = page_alloc().total_managed();
    let low_water = page_alloc().low_water_mark();
    #[cfg(not(test))]
    if !PALLOC_INFO_LOGGED.swap(true, Ordering::AcqRel) {
        // No allocator lock is held here: total_managed/low_water each take
        // and release PAGE_ALLOC (Rank 70) inside their accessor.
        println!("palloc: managed_frames={total_frames} low_water={low_water}");
    }
    (used_frames.saturating_mul(FRAME_SIZE), total_frames.saturating_mul(FRAME_SIZE))
}
