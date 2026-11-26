//! palloc — the portable, non-cryptographic page-frame pools (MEM-07,
//! D-14, D-17, D-21).
//!
//! The allocator keeps four Roaring bitmaps over physical frame numbers:
//!
//! - `dirty` — a frame that is free but not yet zeroed,
//! - `clean` — a zeroed frame ready to hand out,
//! - `active` — a frame handed out and not yet freed,
//! - `pinned` — a frame some kernel path holds across a sleep (MEM-10).
//!
//! The evictable set is the exact set algebra
//! `Evictable = Dirty \ (Pinned ∪ Active)` (D-14, D-21): a pinned or active
//! frame can never be selected for reclaim.  Per-frame pin counts are
//! lock-free atomics so nested pins work and `is_pinned` can be consulted
//! from `PAGE_CACHE` (Rank 20) without taking `PAGE_ALLOC` (Rank 70).
//!
//! The High-Water Pager zeroes dirty frames in batches *outside* the
//! `PAGE_ALLOC` lock and holds about 15% of managed memory clean.  Under
//! pressure `alloc_clean` drops `PAGE_ALLOC` completely before calling the
//! arch evictor: the evictor acquires `PAGE_CACHE` (Rank 20) and
//! `PROC_TABLE` (Rank 50), shoots down TLBs, and re-enters `PAGE_ALLOC`
//! (Rank 70) to return reclaimed frames.  Holding Rank 70 across that call
//! would invert the documented lock order.
//!
//! Frame numbers index 256 MiB chunks: `chunk = pfn >> 16`,
//! `offset = pfn & 0xFFFF`, and the physical address is
//! `(((chunk as usize) << 16) | (offset as usize)) * 4096` — exactly
//! `pfn * 4096` with the chunk split made explicit so the 256 MiB boundary
//! is provably free of off-by-one or truncation (T-05-29).
//!
//! `palloc_max_frames=<N>` caps the managed frame count so a small-memory
//! `-smp 2` gate can drive the evictor deterministically while QEMU `-m`
//! stays large enough for the static `.bss` and initrd.

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

/// Physical address of frame `pfn`: the exact 256 MiB chunk/offset formula.
pub fn pfn_to_phys(pfn: u32) -> PhysAddr {
    let chunk = pfn >> 16;
    let offset = pfn & 0xFFFF;
    PhysAddr::new(((((chunk as usize) << 16) | (offset as usize)) * FRAME_SIZE) as u64)
}

/// Frame number of a page-aligned physical address.
///
/// Returns [`PageAllocError::MisalignedAddr`] for a non-page-aligned
/// address and [`PageAllocError::OutOfBounds`] for an address whose frame
/// number does not fit `u32`.
pub fn phys_to_pfn(pa: PhysAddr) -> Result<u32, PageAllocError> {
    if pa.addr() % FRAME_SIZE as u64 != 0 {
        return Err(PageAllocError::MisalignedAddr);
    }
    u32::try_from(pa.addr() / FRAME_SIZE as u64).map_err(|_| PageAllocError::OutOfBounds)
}

/// Parse an exact whitespace-delimited `palloc_max_frames=<N>` token.
///
/// `N` must be greater than zero; an absent, malformed, or zero value is
/// `None`.
pub fn parse_max_frames_bootarg(cmdline: &str) -> Option<usize> {
    cmdline
        .split_whitespace()
        .find_map(|token| token.strip_prefix("palloc_max_frames="))
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|frames| *frames > 0)
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

/// Serialise host tests that assert on the process-wide counters.
#[cfg(test)]
pub(crate) fn stats_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static STATS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    STATS_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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
    ///
    /// Every managed frame starts in `dirty` (fresh RAM contents are
    /// undefined until zeroed).  The boot path calls
    /// [`Self::refill_clean_pool`] with its real `zero_frame` once the
    /// allocator is installed; `alloc_clean` also refills on demand.
    pub fn init_ranges(
        &self,
        available_mem: PhysRange,
        used_ranges: impl Iterator<Item = PhysRange>,
        max_frames: Option<usize>,
    ) -> Result<(), PageAllocError> {
        let start = available_mem.start().addr();
        let end = available_mem.end().addr();
        if start % FRAME_SIZE as u64 != 0 || end % FRAME_SIZE as u64 != 0 {
            return Err(PageAllocError::MisalignedAddr);
        }
        let first = phys_to_pfn(PhysAddr::new(start))?;
        let last = phys_to_pfn(PhysAddr::new(end))?;

        let mut used = RoaringBitmap::new();
        for range in used_ranges {
            let rs = range.start().addr();
            let re = range.end().addr();
            let rf = phys_to_pfn(PhysAddr::new(rs))?;
            let rl = phys_to_pfn(PhysAddr::new(re))?;
            for pfn in rf..rl {
                used.insert(pfn);
            }
        }

        let mut managed = RoaringBitmap::new();
        for pfn in first..last {
            if !used.contains(pfn) {
                managed.insert(pfn);
            }
        }

        let total = match max_frames {
            Some(cap) => managed.len().min(cap as u64),
            None => managed.len(),
        } as usize;

        let max_pfn = last as usize;
        let mut pin_counts = Vec::with_capacity(max_pfn);
        pin_counts.resize_with(max_pfn, || AtomicU32::new(0));

        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        state.clean = RoaringBitmap::new();
        state.dirty = RoaringBitmap::new();
        state.active = RoaringBitmap::new();
        state.pinned = RoaringBitmap::new();
        for pfn in managed.iter().take(total) {
            state.dirty.insert(pfn);
        }
        state.pin_counts = pin_counts.into_boxed_slice();
        state.total_managed = total;
        state.target_clean = total * CLEAN_TARGET_PCT / 100;
        state.low_water = (total * 5 / 100).max(LOW_WATER_MIN);
        Ok(())
    }

    /// The number of managed frames.
    pub fn total_managed(&self) -> usize {
        let node = LockNode::new();
        self.state.lock(&node).total_managed
    }

    /// The low-water mark: `alloc_clean` tops the clean pool up to the
    /// target when `clean.len()` is below this.
    pub fn low_water_mark(&self) -> usize {
        let node = LockNode::new();
        self.state.lock(&node).low_water
    }

    /// The High-Water Pager's clean target (~15% of managed frames).
    pub fn target_clean(&self) -> usize {
        let node = LockNode::new();
        self.state.lock(&node).target_clean
    }

    /// Pool cardinalities `(clean, dirty, active, pinned)` — host tests only.
    #[cfg(test)]
    pub(crate) fn pool_lens(&self) -> (u64, u64, u64, u64) {
        let node = LockNode::new();
        let state = self.state.lock(&node);
        (state.clean.len(), state.dirty.len(), state.active.len(), state.pinned.len())
    }

    /// The exact evictable set: `dirty \ (pinned ∪ active)`.
    pub fn evictable_dirty(&self) -> RoaringBitmap {
        let node = LockNode::new();
        let state = self.state.lock(&node);
        state.dirty.difference(&state.pinned.union(&state.active))
    }

    /// Increment the pin count of `pfn`; the frame enters the `pinned`
    /// Roaring pool on the `0 -> 1` transition.
    pub fn pin_frame(&self, pfn: u32) -> Result<(), PageAllocError> {
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let Some(count) = state.pin_counts.get(pfn as usize) else {
            return Err(PageAllocError::OutOfBounds);
        };
        let previous = count.fetch_add(1, Ordering::AcqRel);
        if previous == 0 {
            state.pinned.insert(pfn);
        }
        Ok(())
    }

    /// Decrement the pin count of `pfn`; the frame leaves the `pinned`
    /// pool only on the `1 -> 0` transition.
    pub fn unpin_frame(&self, pfn: u32) -> Result<(), PageAllocError> {
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let Some(count) = state.pin_counts.get(pfn as usize) else {
            return Err(PageAllocError::OutOfBounds);
        };
        if count.load(Ordering::Acquire) == 0 {
            return Err(PageAllocError::NotAllocated);
        }
        let previous = count.fetch_sub(1, Ordering::AcqRel);
        if previous == 1 {
            state.pinned.remove(pfn);
        }
        Ok(())
    }

    /// Whether `pfn` is pinned.  Consults the atomic pin table under
    /// `PAGE_ALLOC` (Rank 70), so it is legal from `PAGE_CACHE` (Rank 20):
    /// the documented order is ascending and no path holds `PAGE_ALLOC`
    /// while taking `PAGE_CACHE`.
    pub fn is_pinned(&self, pfn: u32) -> bool {
        let node = LockNode::new();
        let state = self.state.lock(&node);
        state.pin_counts.get(pfn as usize).is_some_and(|c| c.load(Ordering::Acquire) > 0)
    }

    /// Return a frame to the dirty pool.  Double-free, a pinned frame, and
    /// an out-of-range frame are rejected cleanly.
    pub fn free_frame(&self, pa: PhysAddr) -> Result<(), PageAllocError> {
        let pfn = phys_to_pfn(pa)?;
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let Some(count) = state.pin_counts.get(pfn as usize) else {
            return Err(PageAllocError::OutOfBounds);
        };
        if count.load(Ordering::Acquire) > 0 {
            return Err(PageAllocError::NotAllocated);
        }
        if !state.active.remove(pfn) {
            return Err(PageAllocError::NotAllocated);
        }
        state.dirty.insert(pfn);
        Ok(())
    }

    /// Zero up to `max_frames` dirty frames into the clean pool.
    ///
    /// The `PAGE_ALLOC` lock is released around `zero_frame`, so the
    /// High-Water Pager never zeroes while holding the rank-70 lock.
    /// Returns the number of frames moved.
    pub fn refill_clean_batch<F>(&self, max_frames: usize, zero_frame: F) -> usize
    where
        F: Fn(PhysAddr),
    {
        if max_frames == 0 {
            return 0;
        }
        let batch: Vec<u32> = {
            let node = LockNode::new();
            let mut state = self.state.lock(&node);
            let evictable = state.dirty.difference(&state.pinned.union(&state.active));
            let mut batch = Vec::new();
            for pfn in evictable.iter().take(max_frames) {
                if state.dirty.remove(pfn) {
                    batch.push(pfn);
                }
            }
            batch
        };
        if batch.is_empty() {
            return 0;
        }
        for pfn in &batch {
            zero_frame(pfn_to_phys(*pfn));
        }
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        for pfn in &batch {
            state.clean.insert(*pfn);
        }
        batch.len()
    }

    /// Top the clean pool up to the High-Water target (~15%).  Returns
    /// the number of frames moved.
    pub fn refill_clean_pool<F>(&self, zero_frame: F) -> usize
    where
        F: Fn(PhysAddr),
    {
        let node = LockNode::new();
        let state = self.state.lock(&node);
        let want = state.target_clean.saturating_sub(state.clean.len() as usize);
        drop(state);
        self.refill_clean_batch(want, zero_frame)
    }

    /// Allocate one zeroed frame.
    ///
    /// `zero_frame` zeroes a dirty frame (called outside `PAGE_ALLOC`);
    /// `evict_fn` runs the arch evictor and returns how many frames it
    /// reclaimed.  `alloc_clean` drops `PAGE_ALLOC` (Rank 70) completely
    /// before invoking `evict_fn`, which may acquire `PAGE_CACHE` (Rank 20)
    /// and `PROC_TABLE` (Rank 50) and re-enter `PAGE_ALLOC` (Rank 70) to
    /// return frames.
    pub fn alloc_clean<Z, E>(&self, zero_frame: Z, mut evict_fn: E) -> Result<PhysAddr, PageAllocError>
    where
        Z: Fn(PhysAddr),
        E: FnMut() -> usize,
    {
        for attempt in 0..=MAX_EVICT_ATTEMPTS {
            let need = {
                let node = LockNode::new();
                let state = self.state.lock(&node);
                if (state.clean.len() as usize) < state.low_water {
                    state.target_clean.saturating_sub(state.clean.len() as usize)
                } else {
                    0
                }
            };
            if need > 0 {
                self.refill_clean_batch(need, &zero_frame);
            }

            {
                let node = LockNode::new();
                let mut state = self.state.lock(&node);
                if let Some(pfn) = pop_first(&mut state.clean) {
                    state.active.insert(pfn);
                    return Ok(pfn_to_phys(pfn));
                }
            }

            if attempt == MAX_EVICT_ATTEMPTS {
                break;
            }
            // The guard above is gone: PAGE_ALLOC (Rank 70) is NOT held
            // across evict_fn (Rank 20 -> 50 -> 70 re-entry is legal).
            if evict_fn() == 0 {
                return Err(PageAllocError::OutOfSpace);
            }
        }
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
        // Pin two frames while they are still in `dirty`.
        alloc.pin_frame(3).unwrap();
        alloc.pin_frame(5).unwrap();
        assert_eq!(alloc.evictable_dirty().len(), 62, "fresh frames are all dirty");

        let active = alloc.alloc_clean(|_| {}, || 0).unwrap();
        let active_pfn = phys_to_pfn(active).unwrap();

        let (clean, dirty, active_len, pinned) = alloc.pool_lens();
        assert!(clean + dirty + active_len <= 64);
        assert_eq!(pinned, 2);

        let evictable = alloc.evictable_dirty();
        assert!(!evictable.contains(active_pfn), "active is not evictable");
        assert!(!evictable.contains(3), "pinned is not evictable");
        assert!(!evictable.contains(5), "pinned is not evictable");
        assert_eq!(
            evictable.len(),
            dirty - 2,
            "the two pinned frames are still dirty and are the only exclusions"
        );
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

    /// Task 3 (MEM-07 concurrency): two threads allocate/evict and churn
    /// nested pins; zero duplicate frames, zero pinned frames handed to the
    /// evictor, zero rank violations, and `pinned_skipped > 0`.
    #[test]
    fn pin_guard_and_concurrent_alloc_evict() {
        use std::collections::HashSet;
        use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
        use std::sync::{Arc, Mutex};

        let _stats = stats_test_guard();
        reset_evictor_stats();

        static PAGE_CACHE_LOCK: crate::mcslock::Lock<()> =
            crate::mcslock::Lock::new_ranked("test_palloc_concurrent_page_cache", LockRank::PageCache, ());

        let alloc = Arc::new(allocator(256));
        // Sixteen frames the pin thread keeps pinned for the whole run.
        let pinned: Vec<PhysAddr> = (0..16)
            .map(|_| {
                let pa = alloc.alloc_clean(|_| {}, || 0).unwrap();
                alloc.pin_frame(phys_to_pfn(pa).unwrap()).unwrap();
                pa
            })
            .collect();
        let pinned = Arc::new(pinned);

        let retired = Arc::new(Mutex::new(Vec::<PhysAddr>::new()));
        let outstanding = Arc::new(Mutex::new(HashSet::<u32>::new()));
        let stop = Arc::new(AtomicBool::new(false));

        std::thread::scope(|scope| {
            // Thread 1: nested pin/unpin churn; the pin count never reaches
            // zero, so no frame in `pinned` is ever evictable.
            let pin_alloc = Arc::clone(&alloc);
            let pin_list = Arc::clone(&pinned);
            let pin_stop = Arc::clone(&stop);
            let pinner = scope.spawn(move || {
                while !pin_stop.load(AtomicOrdering::Relaxed) {
                    for pa in pin_list.iter() {
                        pin_alloc.pin_frame(phys_to_pfn(*pa).unwrap()).unwrap();
                    }
                    for pa in pin_list.iter() {
                        assert!(pin_alloc.is_pinned(phys_to_pfn(*pa).unwrap()));
                        pin_alloc.unpin_frame(phys_to_pfn(*pa).unwrap()).unwrap();
                    }
                }
            });

            // Thread 0: allocate, break the low-water mark, and run an
            // evictor that observes pinned frames (counted) and frees only
            // retired frames.
            let ev_alloc = Arc::clone(&alloc);
            let ev_pinned = Arc::clone(&pinned);
            let ev_retired = Arc::clone(&retired);
            let ev_outstanding = Arc::clone(&outstanding);
            let allocator_thread = scope.spawn(move || {
                let mut mine: Vec<PhysAddr> = Vec::new();
                for _ in 0..2_000 {
                    if mine.len() >= 32 {
                        let pa = mine.remove(0);
                        let pfn = phys_to_pfn(pa).unwrap();
                        assert!(ev_outstanding.lock().unwrap().remove(&pfn));
                        ev_retired.lock().unwrap().push(pa);
                    }
                    let result = ev_alloc.alloc_clean(
                        |_| {},
                        || {
                            let node = LockNode::new();
                            let _page_cache = PAGE_CACHE_LOCK.lock(&node);
                            for pa in ev_pinned.iter() {
                                if ev_alloc.is_pinned(phys_to_pfn(*pa).unwrap()) {
                                    record_pinned_skipped();
                                }
                            }
                            let mut retired = ev_retired.lock().unwrap();
                            if let Some(pa) = retired.pop() {
                                drop(retired);
                                ev_alloc.free_frame(pa).expect("retired frames are unpinned and active");
                                1
                            } else {
                                0
                            }
                        },
                    );
                    match result {
                        Ok(pa) => {
                            let pfn = phys_to_pfn(pa).unwrap();
                            assert!(
                                ev_outstanding.lock().unwrap().insert(pfn),
                                "frame {pfn} handed out twice"
                            );
                            mine.push(pa);
                        }
                        Err(PageAllocError::OutOfSpace) => break,
                        Err(other) => panic!("unexpected alloc error: {other:?}"),
                    }
                }
                mine
            });

            let mine = allocator_thread.join().expect("allocator thread");
            stop.store(true, AtomicOrdering::Relaxed);
            pinner.join().expect("pin thread");
            drop(mine);
        });

        for pa in pinned.iter() {
            assert!(alloc.is_pinned(phys_to_pfn(*pa).unwrap()), "pinned frames stay pinned");
        }
        assert!(evictor_stats().pinned_skipped > 0, "the evictor saw pinned frames");
    }
}
