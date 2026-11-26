//! pagecache — the Flattened-TPHT `palloc.hash` image page cache and its
//! shared-text reverse map (MEM-16, D-14, D-15, D-17, D-18, D-29).
//!
//! The cache maps `(image_id, page_idx)` to one physical frame that every
//! process running the image can map.  Keys are the exact 64-bit packing
//! `((image_id as u64) << 32) | (page_idx as u64)`; the full key is stored
//! in the TPHT value so a fingerprint match is verified against the whole
//! key (no collision aliasing).
//!
//! Only *re-readable* pages are cacheable and droppable (D-14, D-15):
//!
//! - `SharedText` — user text, mapped read-only,
//! - `UntouchedData` — initialised-data pages no process has written.
//!
//! Dirty anonymous memory (bss, stack, heap, written data) is never
//! inserted; `mark_data_dirty` removes an untouched page from the
//! droppable queue permanently.
//!
//! Every writer, rmap mutation and queue mutation runs under `Lock` at
//! `LockRank::PageCache` (Rank 20), strictly below `PROC_TABLE` (50) and
//! `PAGE_ALLOC` (70).  `select_eviction_victim` removes the victim's key
//! from `palloc.hash` **before** returning it, so no concurrent fault can
//! map the frame while the arch layer unmaps PTEs, shoots down TLBs and
//! returns the frame to `PageAlloc`.
//!
//! TPHT sizing follows DEBT-04's production rule:
//! `banks_for_occupancy(num_blocks, 85)` (twice the measured worst-case
//! spill at 85% occupancy).

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

use ds::tpht::TphtError;
use ds::tpht::flattened::{FlattenedTphtShard, banks_for_occupancy};

use crate::mcslock::{Lock, LockNode};
use crate::mem::PhysAddr;
use crate::palloc::{record_evict_data, record_evict_text, record_pinned_skipped};
use crate::smp::LockRank;

/// Physical addresses are packed below bit 52; the top 12 bits of the value
/// word carry the page metadata (`kind` and generation).
pub const PA_BITS: u32 = 52;
/// Mask of the physical address inside a TPHT value word.
pub const PA_MASK: u64 = (1 << PA_BITS) - 1;
/// Generation field width inside the packed metadata.
pub const GENERATION_BITS: u32 = 10;

/// The `(image_id, page_idx)` cache key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageKey {
    pub image_id: u32,
    pub page_idx: u32,
}

impl PageKey {
    /// A cache key.
    pub const fn new(image_id: u32, page_idx: u32) -> Self {
        Self { image_id, page_idx }
    }

    /// The exact 64-bit TPHT key: `(image_id << 32) | page_idx`.
    pub const fn packed(self) -> u64 {
        ((self.image_id as u64) << 32) | (self.page_idx as u64)
    }

    /// Recover a key from its packed form.
    pub const fn from_packed(packed: u64) -> Self {
        Self { image_id: (packed >> 32) as u32, page_idx: packed as u32 }
    }
}

/// What kind of image page a cache entry holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    /// User text, mapped read-only; droppable and re-readable.
    SharedText,
    /// Initialised data no process has written; droppable until written.
    UntouchedData,
}

impl PageKind {
    fn bit(self) -> u64 {
        match self {
            PageKind::SharedText => 0,
            PageKind::UntouchedData => 1,
        }
    }

    fn from_bit(bit: u64) -> Self {
        if bit & 1 == 0 { PageKind::SharedText } else { PageKind::UntouchedData }
    }
}

/// One process mapping of a shared frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RmapEntry {
    pub proc_slot: u16,
    pub page_va: usize,
}

/// A cache hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachedPage {
    pub key: PageKey,
    pub pa: PhysAddr,
    pub kind: PageKind,
}

/// A victim selected for architecture-side eviction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvictionCandidate {
    pub key: PageKey,
    pub pa: PhysAddr,
    pub kind: PageKind,
    /// Every mapping the arch layer must unmap and shoot down before the
    /// frame returns to `PageAlloc`.
    pub rmaps: Vec<RmapEntry>,
}

/// Cache errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageCacheError {
    /// The key is already cached.
    Exists,
    /// The TPHT's overflow banks are full.
    Full,
    /// The physical address is not page-aligned or does not fit the packed
    /// value word.
    BadValue,
    /// The TPHT rejected the block count.
    BadBlockCount,
}

impl From<TphtError> for PageCacheError {
    fn from(error: TphtError) -> Self {
        match error {
            TphtError::Exists => PageCacheError::Exists,
            TphtError::BadBlockCount => PageCacheError::BadBlockCount,
            _ => PageCacheError::Full,
        }
    }
}

/// A droppable queue entry: the key, its frame and its kind.
type Droppable = (PageKey, PhysAddr, PageKind);

/// The mutable cache state, always taken under `PAGE_CACHE` (Rank 20).
struct CacheState {
    shard: FlattenedTphtShard<2>,
    /// `key -> [(proc_slot, page_va)]` for shared frames.
    rmap: BTreeMap<u64, Vec<RmapEntry>>,
    /// FIFO of cacheable, currently droppable pages.
    droppable: VecDeque<Droppable>,
    /// Next packed metadata generation (wraps inside `GENERATION_BITS`).
    generation: u64,
}

/// The `palloc.hash` image page cache.
pub struct PageCache {
    state: Lock<CacheState>,
}

impl PageCache {
    /// Build a cache over `num_blocks` TPHT home blocks with DEBT-04's
    /// production bank sizing for 85% occupancy.
    pub fn new(num_blocks: usize) -> Result<Self, PageCacheError> {
        let banks = banks_for_occupancy(num_blocks, 85);
        let shard = FlattenedTphtShard::<2>::with_banks(num_blocks, banks)?;
        Ok(Self {
            state: Lock::new_ranked(
                "palloc_hash",
                LockRank::PageCache,
                CacheState {
                    shard,
                    rmap: BTreeMap::new(),
                    droppable: VecDeque::new(),
                    generation: 0,
                },
            ),
        })
    }

    /// Look up a cached image page.  A cold or evicted page is `None`.
    pub fn lookup(&self, key: PageKey) -> Option<CachedPage> {
        // RED-SCAFFOLD: lookup is not implemented yet.
        let _ = key;
        return None;
        #[allow(unreachable_code)]
        let node = LockNode::new();
        let state = self.state.lock(&node);
        let packed = key.packed();
        let found = state.shard.lookup(packed, |value| value[1] == packed)?;
        let (pa, kind) = decode(&found.value)?;
        Some(CachedPage { key, pa, kind })
    }

    /// Cache a shared text frame (mapped read-only by every process).
    pub fn insert_text(&self, key: PageKey, pa: PhysAddr) -> Result<(), PageCacheError> {
        self.insert_entry(key, pa, PageKind::SharedText)
    }

    /// Cache an initialised-data frame no process has written yet.
    pub fn register_untouched_data(&self, key: PageKey, pa: PhysAddr) -> Result<(), PageCacheError> {
        self.insert_entry(key, pa, PageKind::UntouchedData)
    }

    fn insert_entry(&self, key: PageKey, pa: PhysAddr, kind: PageKind) -> Result<(), PageCacheError> {
        // RED-SCAFFOLD: insertion is not implemented yet.
        let _ = (key, pa, kind);
        return Ok(());
        #[allow(unreachable_code)]
        if pa.addr() & (crate::palloc::FRAME_SIZE as u64 - 1) != 0 || pa.addr() > PA_MASK {
            return Err(PageCacheError::BadValue);
        }
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let packed = key.packed();
        let generation = state.generation & ((1 << GENERATION_BITS) - 1);
        state.generation = (generation + 1) & ((1 << GENERATION_BITS) - 1);
        let meta = (generation << 1) | kind.bit();
        let value = [pa.addr() | (meta << PA_BITS), packed];
        state.shard.insert(packed, value, |candidate| candidate[1] == packed)?;
        state.droppable.push_back((key, pa, kind));
        Ok(())
    }

    /// Mark a cached data frame as written: it leaves the droppable queue
    /// permanently and is never selected for eviction.  Returns whether a
    /// queue entry was removed.
    pub fn mark_data_dirty(&self, pa: PhysAddr) -> bool {
        // RED-SCAFFOLD: the droppable queue is not implemented yet.
        let _ = pa;
        return false;
        #[allow(unreachable_code)]
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let before = state.droppable.len();
        state.droppable.retain(|entry| entry.1 != pa);
        state.droppable.len() != before
    }

    /// Record that `proc_slot` maps `key` at `page_va`.
    pub fn add_rmap(&self, key: PageKey, entry: RmapEntry) {
        // RED-SCAFFOLD: rmap tracking is not implemented yet.
        let _ = (key, entry);
        return;
        #[allow(unreachable_code)]
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let entries = state.rmap.entry(key.packed()).or_default();
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }

    /// Drop every mapping owned by `proc_slot` (exit/exec).  Cached frames
    /// stay in the hash and stay cached for other processes.  Returns the
    /// number of removed mappings.
    pub fn remove_proc_rmaps(&self, proc_slot: u16) -> usize {
        // RED-SCAFFOLD: rmap tracking is not implemented yet.
        let _ = proc_slot;
        return 0;
        #[allow(unreachable_code)]
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let mut removed = 0;
        let mut empty = Vec::new();
        for (packed, entries) in state.rmap.iter_mut() {
            let before = entries.len();
            entries.retain(|entry| entry.proc_slot != proc_slot);
            removed += before - entries.len();
            if entries.is_empty() {
                empty.push(*packed);
            }
        }
        for packed in empty {
            state.rmap.remove(&packed);
        }
        removed
    }

    /// Select one unpinned droppable frame for eviction.
    ///
    /// The chosen key is removed from `palloc.hash` **before** this returns,
    /// so a concurrent fault cannot map the frame while the arch layer
    /// unmaps it.  Pinned frames are skipped and counted in
    /// `palloc::record_pinned_skipped()`.
    pub fn select_eviction_victim(&self, is_pinned: impl Fn(PhysAddr) -> bool) -> Option<EvictionCandidate> {
        // RED-SCAFFOLD: victim selection is not implemented yet.
        let _ = is_pinned;
        return None;
        #[allow(unreachable_code)]
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let scan = state.droppable.len();
        for _ in 0..scan {
            let (key, pa, kind) = state.droppable.pop_front()?;
            if is_pinned(pa) {
                record_pinned_skipped();
                state.droppable.push_back((key, pa, kind));
                continue;
            }
            let packed = key.packed();
            if !state.shard.remove(packed, |value| value[1] == packed) {
                // A stale queue entry (already removed): drop it and move on.
                continue;
            }
            let rmaps = state.rmap.remove(&packed).unwrap_or_default();
            match kind {
                PageKind::SharedText => record_evict_text(),
                PageKind::UntouchedData => record_evict_data(),
            }
            return Some(EvictionCandidate { key, pa, kind, rmaps });
        }
        None
    }

    /// The number of droppable queue entries (host tests).
    #[cfg(test)]
    pub(crate) fn droppable_len(&self) -> usize {
        // RED-SCAFFOLD: the droppable queue is not implemented yet.
        return 0;
        #[allow(unreachable_code)]
        let node = LockNode::new();
        self.state.lock(&node).droppable.len()
    }
}

/// Decode `(pa, kind)` from a TPHT value word pair.
fn decode(value: &[u64; 2]) -> Option<(PhysAddr, PageKind)> {
    let meta = value[0] >> PA_BITS;
    let kind = PageKind::from_bit(meta >> 1);
    Some((PhysAddr::new(value[0] & PA_MASK), kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palloc::{evictor_stats, reset_evictor_stats};

    fn cache() -> PageCache {
        PageCache::new(16).expect("16 home blocks")
    }

    fn pa(frame: u64) -> PhysAddr {
        PhysAddr::new(frame * crate::palloc::FRAME_SIZE as u64)
    }

    /// Test 1 (MEM-16): one shared text frame maps into two processes and
    /// both mappings are recorded.
    #[test]
    fn shared_text_maps_one_frame_into_two_processes() {
        let cache = cache();
        let key = PageKey::new(7, 3);
        cache.insert_text(key, pa(10)).unwrap();
        cache.add_rmap(key, RmapEntry { proc_slot: 1, page_va: 0x4000 });
        cache.add_rmap(key, RmapEntry { proc_slot: 2, page_va: 0x9000 });

        let a = cache.lookup(key).expect("process A lookup");
        let b = cache.lookup(key).expect("process B lookup");
        assert_eq!(a.pa, b.pa, "both processes map the same physical frame");
        assert_eq!(a.pa, pa(10));
        assert_eq!(a.kind, PageKind::SharedText);

        let victim = cache.select_eviction_victim(|_| false).expect("victim");
        assert_eq!(victim.key, key);
        assert_eq!(victim.rmaps.len(), 2);
        assert_eq!(cache.lookup(key), None, "key removed before the arch unmaps");
    }

    /// Test 2 (MEM-16): written data is never a victim.
    #[test]
    fn mark_data_dirty_removes_the_page_permanently() {
        let _stats = crate::palloc::stats_test_guard();
        reset_evictor_stats();
        let cache = cache();
        let text = PageKey::new(1, 1);
        let data = PageKey::new(1, 2);
        cache.insert_text(text, pa(20)).unwrap();
        cache.register_untouched_data(data, pa(21)).unwrap();
        assert_eq!(cache.droppable_len(), 2);

        assert!(cache.mark_data_dirty(pa(21)));
        assert_eq!(cache.droppable_len(), 1);
        assert!(!cache.mark_data_dirty(pa(21)), "already removed");

        let victim = cache.select_eviction_victim(|_| false).expect("victim");
        assert_eq!(victim.key, text, "only the text page is droppable");
        assert_eq!(victim.kind, PageKind::SharedText);
        assert!(cache.lookup(data).is_some(), "written data stays cached and mapped");
        assert!(evictor_stats().dropped_text >= 1);
        assert_eq!(evictor_stats().dropped_data, 0);
    }

    /// Test 3 (MEM-16): pinned frames are skipped and counted; the victim's
    /// key leaves the hash before it is returned.
    #[test]
    fn select_eviction_victim_skips_pinned_frames() {
        let _stats = crate::palloc::stats_test_guard();
        reset_evictor_stats();
        let cache = cache();
        let pinned = PageKey::new(2, 1);
        let free = PageKey::new(2, 2);
        cache.insert_text(pinned, pa(30)).unwrap();
        cache.insert_text(free, pa(31)).unwrap();

        let victim = cache
            .select_eviction_victim(|frame| frame == pa(30))
            .expect("an unpinned victim");
        assert_eq!(victim.key, free);
        assert_eq!(victim.pa, pa(31));
        assert!(evictor_stats().pinned_skipped >= 1);
        assert!(cache.lookup(pinned).is_some(), "pinned frame is still cached");
        assert_eq!(cache.lookup(free), None);
    }

    /// Test 4 (MEM-16): process exit drops its mappings but keeps the frames
    /// cached for every other process.
    #[test]
    fn remove_proc_rmaps_keeps_shared_text_cached() {
        let cache = cache();
        let key = PageKey::new(3, 0);
        cache.insert_text(key, pa(40)).unwrap();
        cache.add_rmap(key, RmapEntry { proc_slot: 0, page_va: 0x1000 });
        cache.add_rmap(key, RmapEntry { proc_slot: 1, page_va: 0x2000 });

        assert_eq!(cache.remove_proc_rmaps(0), 1);
        assert!(cache.lookup(key).is_some(), "shared text frame stays cached");
        let victim = cache.select_eviction_victim(|_| false).expect("victim");
        assert_eq!(victim.rmaps, vec![RmapEntry { proc_slot: 1, page_va: 0x2000 }]);
        assert_eq!(cache.remove_proc_rmaps(42), 0);
    }

    /// Test 5 (MEM-16 adjacency/encoding): adjacent page indices and image
    /// ids pack to distinct keys; a cold lookup is `None`.
    #[test]
    fn adjacent_keys_are_distinct_and_cold_lookup_is_none() {
        let keys = [PageKey::new(1, 1), PageKey::new(1, 2), PageKey::new(2, 1)];
        for (i, a) in keys.iter().enumerate() {
            for b in &keys[i + 1..] {
                assert_ne!(a.packed(), b.packed(), "{a:?} and {b:?} alias");
            }
        }
        assert_eq!(PageKey::new(0xDEAD_BEEF, 0xCAFE_F00D).packed(), 0xDEAD_BEEF_CAFE_F00D);
        assert_eq!(PageKey::from_packed(0xDEAD_BEEF_CAFE_F00D), PageKey::new(0xDEAD_BEEF, 0xCAFE_F00D));

        let cache = cache();
        assert_eq!(cache.lookup(PageKey::new(9, 9)), None);
        assert_eq!(cache.select_eviction_victim(|_| false), None, "empty queue");
        assert_eq!(cache.insert_text(PageKey::new(1, 1), PhysAddr::new(1)), Err(PageCacheError::BadValue));
    }
}
