//! palloc — the portable, non-cryptographic page-frame pools (MEM-07,
//! D-14, D-17, D-21).
//!
//! RED SCAFFOLD (05-16 Task 1): the API, the Roaring pools and the tests
//! are in place; the bodies marked `RED-SCAFFOLD` are inert so the planned
//! assertions fail.  The GREEN commit implements the exact chunk/offset
//! formula, the `Dirty \ (Pinned ∪ Active)` evictable set, nested pin
//! counts, the High-Water refill and the lock-dropped evictor retry.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use ds::roaring::RoaringBitmap;

use crate::mcslock::{IrqSpinLock, LockNode};
use crate::mem::{PhysAddr, PhysRange};
use crate::pagealloc::PageAllocError;
use crate::smp::LockRank;

/// Frames per 256 MiB chunk (`1 << 16`).
pub const FRAMES_PER_CHUNK: u32 = 1 << 16;
/// The frame size this allocator manages.
pub const FRAME_SIZE: usize = 4096;
/// Clean-frame target as a percentage of managed frames (D-14/D-17).
pub const CLEAN_TARGET_PCT: usize = 15;
/// Low-water floor, in frames (D-17, Claude's discretion).
pub const LOW_WATER_MIN: usize = 16;
/// How many evictor round trips `alloc_clean` attempts before `OutOfSpace`.
pub const MAX_EVICT_ATTEMPTS: usize = 4;

/// Physical address of frame `pfn`.
pub fn pfn_to_phys(pfn: u32) -> PhysAddr {
    // RED-SCAFFOLD: the chunk/offset formula is not implemented yet.
    let _ = pfn;
    PhysAddr::new(0)
}

/// Frame number of a page-aligned physical address.
pub fn phys_to_pfn(pa: PhysAddr) -> Result<u32, PageAllocError> {
    // RED-SCAFFOLD: alignment and range checks are not implemented yet.
    let _ = pa;
    Ok(0)
}

/// Parse an exact whitespace-delimited `palloc_max_frames=<N>` token.
pub fn parse_max_frames_bootarg(cmdline: &str) -> Option<usize> {
    // RED-SCAFFOLD: the bootarg parser is not implemented yet.
    let _ = cmdline;
    None
}

/// Lock-free evictor counters served by `#c/sysstat` (D-17).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvictorStats {
    pub dropped_text: u64,
    pub dropped_data: u64,
    pub pinned_skipped: u64,
}

static EVICT_DROPPED_TEXT: AtomicU64 = AtomicU64::new(0);
static EVICT_DROPPED_DATA: AtomicU64 = AtomicU64::new(0);
static EVICT_PINNED_SKIPPED: AtomicU64 = AtomicU64::new(0);

/// Count one dropped re-readable text page.
pub fn record_evict_text() {
    EVICT_DROPPED_TEXT.fetch_add(1, Ordering::Relaxed);
}

/// Count one dropped untouched initialised-data page.
pub fn record_evict_data() {
    EVICT_DROPPED_DATA.fetch_add(1, Ordering::Relaxed);
}

/// Count one eviction pass that skipped a pinned frame.
pub fn record_pinned_skipped() {
    EVICT_PINNED_SKIPPED.fetch_add(1, Ordering::Relaxed);
}

/// A lock-free snapshot of the evictor counters.
pub fn evictor_stats() -> EvictorStats {
    EvictorStats {
        dropped_text: EVICT_DROPPED_TEXT.load(Ordering::Relaxed),
        dropped_data: EVICT_DROPPED_DATA.load(Ordering::Relaxed),
        pinned_skipped: EVICT_PINNED_SKIPPED.load(Ordering::Relaxed),
    }
}

/// Reset the counters (host tests only).
#[cfg(test)]
pub(crate) fn reset_evictor_stats() {
    EVICT_DROPPED_TEXT.store(0, Ordering::Relaxed);
    EVICT_DROPPED_DATA.store(0, Ordering::Relaxed);
    EVICT_PINNED_SKIPPED.store(0, Ordering::Relaxed);
}

/// The mutable pool state, always taken under `PAGE_ALLOC` (Rank 70).
struct PoolState {
    clean: RoaringBitmap,
    dirty: RoaringBitmap,
    active: RoaringBitmap,
    pinned: RoaringBitmap,
    /// One atomic count per managed frame; empty before `init_ranges`.
    pin_counts: Box<[AtomicU32]>,
    total_managed: usize,
    low_water: usize,
    target_clean: usize,
}

impl PoolState {
    fn empty() -> Self {
        Self {
            clean: RoaringBitmap::new(),
            dirty: RoaringBitmap::new(),
            active: RoaringBitmap::new(),
            pinned: RoaringBitmap::new(),
            pin_counts: Vec::new().into_boxed_slice(),
            total_managed: 0,
            low_water: LOW_WATER_MIN,
            target_clean: 0,
        }
    }
}

/// The portable frame allocator.
pub struct PageAlloc {
    state: IrqSpinLock<PoolState>,
}

impl Default for PageAlloc {
    fn default() -> Self {
        Self::new()
    }
}

impl PageAlloc {
    /// An allocator with empty pools; call [`Self::init_ranges`] before use.
    pub fn new() -> Self {
        Self {
            state: IrqSpinLock::new("page_alloc", LockRank::PageAlloc, PoolState::empty()),
        }
    }

    /// Seed the managed frames from `available_mem` minus `used_ranges`,
    /// optionally capped to `max_frames`.
    pub fn init_ranges(
        &self,
        available_mem: PhysRange,
        used_ranges: impl Iterator<Item = PhysRange>,
        max_frames: Option<usize>,
    ) -> Result<(), PageAllocError> {
        // RED-SCAFFOLD: range seeding and the High-Water sizing are not
        // implemented yet.
        let _ = (available_mem, used_ranges, max_frames);
        Ok(())
    }

    /// The number of managed frames.
    pub fn total_managed(&self) -> usize {
        let node = LockNode::new();
        self.state.lock(&node).total_managed
    }

    /// The low-water mark.
    pub fn low_water_mark(&self) -> usize {
        let node = LockNode::new();
        self.state.lock(&node).low_water
    }

    /// The High-Water Pager's clean target (~15% of managed frames).
    pub fn target_clean(&self) -> usize {
        let node = LockNode::new();
        self.state.lock(&node).target_clean
    }

    /// The exact evictable set: `dirty \ (pinned ∪ active)`.
    pub fn evictable_dirty(&self) -> RoaringBitmap {
        // RED-SCAFFOLD: the set algebra is not implemented yet.
        RoaringBitmap::new()
    }

    /// Increment the pin count of `pfn`.
    pub fn pin_frame(&self, pfn: u32) -> Result<(), PageAllocError> {
        // RED-SCAFFOLD: pin counts are not implemented yet.
        let _ = pfn;
        Ok(())
    }

    /// Decrement the pin count of `pfn`.
    pub fn unpin_frame(&self, pfn: u32) -> Result<(), PageAllocError> {
        // RED-SCAFFOLD: pin counts are not implemented yet.
        let _ = pfn;
        Ok(())
    }

    /// Whether `pfn` is pinned.  Lock-free.
    pub fn is_pinned(&self, pfn: u32) -> bool {
        // RED-SCAFFOLD: pin counts are not implemented yet.
        let _ = pfn;
        false
    }

    /// Return a frame to the dirty pool.
    pub fn free_frame(&self, pa: PhysAddr) -> Result<(), PageAllocError> {
        // RED-SCAFFOLD: accounting is not implemented yet.
        let _ = pa;
        Ok(())
    }

    /// Zero up to `max_frames` dirty frames into the clean pool.
    pub fn refill_clean_batch<F>(&self, max_frames: usize, zero_frame: F) -> usize
    where
        F: Fn(PhysAddr),
    {
        // RED-SCAFFOLD: the High-Water refill is not implemented yet.
        let _ = (max_frames, zero_frame);
        0
    }

    /// Top the clean pool up to the High-Water target (~15%).
    pub fn refill_clean_pool<F>(&self, zero_frame: F) -> usize
    where
        F: Fn(PhysAddr),
    {
        // RED-SCAFFOLD: the High-Water refill is not implemented yet.
        let _ = zero_frame;
        0
    }

    /// Allocate one zeroed frame.
    pub fn alloc_clean<Z, E>(&self, zero_frame: Z, mut evict_fn: E) -> Result<PhysAddr, PageAllocError>
    where
        Z: Fn(PhysAddr),
        E: FnMut() -> usize,
    {
        // RED-SCAFFOLD: allocation, the lock-drop and the retry are not
        // implemented yet.
        let _ = (&zero_frame, &mut evict_fn);
        Err(PageAllocError::OutOfSpace)
    }
}

/// Remove and return the smallest member of `bitmap`.
fn pop_first(bitmap: &mut RoaringBitmap) -> Option<u32> {
    let value = bitmap.iter().next()?;
    bitmap.remove(value);
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames_phys(frames: usize) -> PhysRange {
        PhysRange::with_end(0, (frames * FRAME_SIZE) as u64)
    }

    fn allocator(frames: usize) -> PageAlloc {
        let alloc = PageAlloc::new();
        alloc.init_ranges(frames_phys(frames), core::iter::empty(), None).unwrap();
        alloc
    }

    /// Test 1 (MEM-07 adjacency): the exact chunk/offset formula across the
    /// 256 MiB boundary.
    #[test]
    fn pfn_to_phys_is_exact_across_the_256mib_chunk_boundary() {
        // chunk 0, last offset.
        assert_eq!(pfn_to_phys(0xFFFF).addr(), 0xFFFF * 4096);
        // chunk 1, offset 0 — the next 256 MiB chunk.
        assert_eq!(pfn_to_phys(0x1_0000).addr(), 0x1000_0000);
        assert_eq!(pfn_to_phys(0x1_0000).addr(), 256 * 1024 * 1024);
        // The two adjacent frames are distinct and contiguous.
        assert_eq!(pfn_to_phys(0xFFFF).addr() + 4096, pfn_to_phys(0x1_0000).addr());
        // Round trip.
        for pfn in [0u32, 1, 0xFFFF, 0x1_0000, 0x2_3456] {
            assert_eq!(phys_to_pfn(pfn_to_phys(pfn)).unwrap(), pfn);
        }
        assert_eq!(phys_to_pfn(PhysAddr::new(1)), Err(PageAllocError::MisalignedAddr));
    }

    /// Test 2 (MEM-07): `Evictable = Dirty \ (Pinned ∪ Active)`.
    #[test]
    fn evictable_dirty_excludes_pinned_and_active_frames() {
        let alloc = allocator(64);
        assert_eq!(alloc.evictable_dirty().len(), 64, "fresh frames are all dirty");

        let active = alloc.alloc_clean(|_| {}, || 0).unwrap();
        let pinned_pa = alloc.alloc_clean(|_| {}, || 0).unwrap();
        let pinned = phys_to_pfn(pinned_pa).unwrap();
        alloc.pin_frame(pinned).unwrap();

        let evictable = alloc.evictable_dirty();
        assert!(!evictable.contains(phys_to_pfn(active).unwrap()), "active is not evictable");
        assert!(!evictable.contains(pinned), "pinned is not evictable");
        assert_eq!(evictable.len(), 62);
        for pfn in evictable.iter() {
            assert!(!alloc.is_pinned(pfn));
        }
    }

    /// Test 3 (MEM-07 idempotency): nested pins leave the pool only on the
    /// `1 -> 0` transition.
    #[test]
    fn nested_pin_unpin_transitions_once() {
        let alloc = allocator(8);
        alloc.pin_frame(3).unwrap();
        alloc.pin_frame(3).unwrap();
        assert!(alloc.is_pinned(3));
        assert!(!alloc.evictable_dirty().contains(3));

        alloc.unpin_frame(3).unwrap();
        assert!(alloc.is_pinned(3), "one nested pin remains");
        assert!(!alloc.evictable_dirty().contains(3));

        alloc.unpin_frame(3).unwrap();
        assert!(!alloc.is_pinned(3));
        assert!(alloc.evictable_dirty().contains(3));

        assert_eq!(alloc.unpin_frame(3), Err(PageAllocError::NotAllocated));
        assert_eq!(alloc.pin_frame(99), Err(PageAllocError::OutOfBounds));
    }

    /// Test 4 (MEM-07 ordering): the High-Water Pager zeroes outside
    /// `PAGE_ALLOC`, and `alloc_clean` drops Rank 70 before `evict_fn`,
    /// which legally acquires `PAGE_CACHE` (Rank 20) and re-enters
    /// `PAGE_ALLOC` (Rank 70).
    #[test]
    fn high_water_refill_and_evictor_run_without_the_page_alloc_lock() {
        use core::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        static PAGE_CACHE_LOCK: crate::mcslock::Lock<()> =
            crate::mcslock::Lock::new_ranked("test_palloc_page_cache", LockRank::PageCache, ());

        let alloc = allocator(64);
        let zeroed = AtomicUsize::new(0);
        let moved = alloc.refill_clean_pool(|pa| {
            assert_eq!(pa.addr() % FRAME_SIZE as u64, 0);
            zeroed.fetch_add(1, AtomicOrdering::Relaxed);
        });
        assert_eq!(moved, alloc.target_clean());
        assert_eq!(moved, 64 * CLEAN_TARGET_PCT / 100);
        assert_eq!(zeroed.load(AtomicOrdering::Relaxed), moved);
        assert!(moved >= alloc.low_water_mark().min(moved));

        // Exhaust every managed frame.
        let mut first = None;
        loop {
            match alloc.alloc_clean(|_| {}, || 0) {
                Ok(pa) => {
                    first.get_or_insert(pa);
                }
                Err(PageAllocError::OutOfSpace) => break,
                Err(other) => panic!("unexpected alloc error: {other:?}"),
            }
        }
        let first = first.expect("at least one frame");

        // evict_fn is called with PAGE_ALLOC released: taking the Rank-20
        // lock here would panic on a rank assertion if Rank 70 were held.
        let pa = alloc
            .alloc_clean(
                |_| {},
                || {
                    let node = LockNode::new();
                    let _page_cache = PAGE_CACHE_LOCK.lock(&node);
                    alloc.free_frame(first).expect("reclaim one active frame");
                    1
                },
            )
            .expect("reclaim and retry");
        assert_eq!(pa, first);
    }

    /// Test 5 (MEM-07 cap): the bootarg parser and the managed-frame cap.
    #[test]
    fn max_frames_bootarg_caps_managed_frames() {
        assert_eq!(parse_max_frames_bootarg("palloc_max_frames=2048"), Some(2048));
        assert_eq!(parse_max_frames_bootarg("quiet palloc_max_frames=7 console=0"), Some(7));
        assert_eq!(parse_max_frames_bootarg("palloc_max_frames=0"), None);
        assert_eq!(parse_max_frames_bootarg("palloc_max_frames=abc"), None);
        assert_eq!(parse_max_frames_bootarg("xpalloc_max_frames=9"), None);
        assert_eq!(parse_max_frames_bootarg("palloc_max_frames"), None);

        let alloc = PageAlloc::new();
        alloc
            .init_ranges(frames_phys(64), core::iter::empty(), Some(20))
            .unwrap();
        assert_eq!(alloc.total_managed(), 20);
        assert_eq!(alloc.evictable_dirty().len(), 20);
        assert_eq!(alloc.target_clean(), 20 * CLEAN_TARGET_PCT / 100);
    }

    /// Test 5b (MEM-07 empty): freeing into empty pools and double-free.
    #[test]
    fn free_into_empty_pools_and_double_free_are_clean() {
        let alloc = allocator(8);
        let pa = alloc.alloc_clean(|_| {}, || 0).unwrap();
        alloc.free_frame(pa).unwrap();
        assert!(alloc.evictable_dirty().contains(phys_to_pfn(pa).unwrap()));
        assert_eq!(alloc.free_frame(pa), Err(PageAllocError::NotAllocated));

        let pinned = alloc.alloc_clean(|_| {}, || 0).unwrap();
        alloc.pin_frame(phys_to_pfn(pinned).unwrap()).unwrap();
        assert_eq!(alloc.free_frame(pinned), Err(PageAllocError::NotAllocated));

        let empty = PageAlloc::new();
        assert_eq!(
            empty.alloc_clean(|_| {}, || 0),
            Err(PageAllocError::OutOfSpace),
            "empty pools ask the evictor, then fail cleanly"
        );
    }
}
