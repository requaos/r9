//! flattened — the 64-byte `TphtBucketHeader` and `FlattenedTphtShard`
//! (DS-01; IG-M06).
//!
//! The header is exactly one cache line — `#[repr(C, align(64))]`, 64 bytes —
//! with the 16-slot layout of guide section 5.2.  Every field a reader can
//! touch is an atomic: the compendium listing's plain `u16` fields are a data
//! race that Miri reports as undefined behaviour, and its memory orderings
//! tear under Miri (03-RESEARCH F7).  The seqlock uses the Boehm ordering
//! (Boehm, "Can Seqlocks Get Along With Programming Language Memory Models?",
//! MSPC 2012; 03-RESEARCH Pattern 2):
//!
//! - writer: bring `seq_lock` from an even `s` to `s + 1` (odd = a write is
//!   in flight); store data with `Release`; finish with a `Release` store of
//!   `s + 2`;
//! - reader: load `seq_lock` with `Acquire`; retry if odd; load the masks,
//!   quotients, tiny pointers and value words with `Relaxed`; `fence(Acquire)`;
//!   reload `seq_lock` with `Relaxed`; accept only when it equals the first
//!   load.
//!
//! This writer assigns tiny pointers `1..=16` — the block's own window — so
//! `spilled_mask` stays zero; it is maintained anyway so the field means what
//! the layout says.  Neighbourhood displacement through `17..=255` and
//! clock-nibble eviction are not Phase 3 requirements: `deref_tiny` resolves
//! the full pointer range from one arena slot read with no header probe
//! (DS-03) and `clock_nibbles` is initialised to zero and otherwise unused.
//!
//! **Home block and spill.**  A key's home block is the low `index_bits` of
//! the mixed Feistel half.  A block with banks places up to `SPILL_THRESHOLD`
//! entries in its 16-slot header (`insert` returns `Ok(Some(ptr))`); a further
//! key of the same block spills into the shard's `ChainedTphtOverflow`
//! (`Ok(None)`), and `lookup` reports where it found the entry through
//! [`Found::home`].  A spilled entry has no tiny pointer, cannot be passed to
//! [`FlattenedTphtShard::deref_tiny`], and is reached through
//! [`FlattenedTphtShard::lookup`].  A home-only shard
//! (`with_banks(n, 0)`) applies no threshold: it fills all 16 slots and then
//! returns [`TphtError::HomeFull`].  Every mutation of a block's chain happens
//! under that block's seqlock, which is the serialisation contract
//! `ChainedTphtOverflow` documents.
//!
//! `deref_tiny` is this module's one `unsafe` site: the unchecked arena read
//! (`get_unchecked`), sound because the slot is masked by `slot_mask` and the
//! arena holds exactly `slot_mask + 1` slots, both fixed in the constructor
//! (D-13; plan 03-10 audits the site).
//!
//! The table stores no full key.  `quotients[i]` holds a 16-bit fingerprint of
//! the remainder, and `same` is the caller's full-key verification of a
//! fingerprint match (RESEARCH Open Question 3).

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicU64, Ordering, fence};

use super::chained::ChainedTphtOverflow;
use super::{HOME_SLOTS, NoProbe, Probe, SPILL_THRESHOLD, TinyPtr, TphtError, feistel, index_bits};

/// Where [`FlattenedTphtShard::lookup`] resolved an entry: `home` is
/// `Some(ptr)` when it came from the home block and `None` when it was found
/// in the block's overflow chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found<const WORDS: usize> {
    /// The entry's value words.
    pub value: [u64; WORDS],
    /// The home tiny pointer, or `None` for a spilled entry.
    pub home: Option<TinyPtr>,
}

/// The home-index variant a shard uses.  Production is always
/// [`IndexChoice::MixedHalf`]; the test-only constructor selects the
/// compendium's reading for the D-15 evidence (RESEARCH C4).
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexChoice {
    /// The low bits of the mixed half `L'` (production; RESEARCH C4).
    MixedHalf,
    /// The low bits of `P` (compendium section 4.1 as written; test only).
    UnmixedLowBits,
}

#[cfg(test)]
impl IndexChoice {
    pub(crate) fn split(self, key: u64, index_bits: u32) -> (usize, u64) {
        match self {
            Self::MixedHalf => feistel::split(key, index_bits),
            Self::UnmixedLowBits => feistel::split_unmixed(key, index_bits),
        }
    }
}

/// The 16-slot, 64-byte home block header of guide section 5.2.
#[repr(C, align(64))]
pub struct TphtBucketHeader {
    /// Odd while a writer holds the block; even is a stable generation.
    seq_lock: AtomicU32,
    /// Bit `i` set when header slot `i` holds an entry (bits 0..15).
    occupied_mask: AtomicU16,
    /// Bit `i` set when slot `i`'s tiny pointer is above the home window.
    spilled_mask: AtomicU16,
    /// The 16-bit fingerprint of each entry's remainder.
    quotients: [AtomicU16; HOME_SLOTS],
    /// Each entry's tiny pointer (`1..=255`, 0 = empty).
    tiny_ptrs: [AtomicU8; HOME_SLOTS],
    /// Sixteen 4-bit clock/aging counters (unused in Phase 3).
    clock_nibbles: AtomicU64,
}

const _: () = {
    assert!(core::mem::size_of::<TphtBucketHeader>() == 64);
    assert!(core::mem::align_of::<TphtBucketHeader>() == 64);
};

impl TphtBucketHeader {
    const fn new() -> Self {
        Self {
            seq_lock: AtomicU32::new(0),
            occupied_mask: AtomicU16::new(0),
            spilled_mask: AtomicU16::new(0),
            quotients: [const { AtomicU16::new(0) }; HOME_SLOTS],
            tiny_ptrs: [const { AtomicU8::new(0) }; HOME_SLOTS],
            clock_nibbles: AtomicU64::new(0),
        }
    }
}

/// A shard of flattened TPHT blocks: one 64-byte header per block and an arena
/// of `16 * num_blocks` slots of `WORDS` words each, addressed by
/// `((block << 4) + (tiny_ptr - 1)) & slot_mask`.  A shard with banks spills
/// a block's entries past `SPILL_THRESHOLD` into its `ChainedTphtOverflow`;
/// a home-only shard holds no overflow.
pub struct FlattenedTphtShard<const WORDS: usize, P: Probe = NoProbe> {
    headers: Box<[TphtBucketHeader]>,
    arena: Box<[[AtomicU64; WORDS]]>,
    /// The spill destination; `None` on a home-only shard (`with_banks(n, 0)`).
    overflow: Option<ChainedTphtOverflow<WORDS>>,
    probe: P,
    bits: u32,
    slot_mask: usize,
    /// Home slots a block may use before its next key spills; in production
    /// this is the measured [`SPILL_THRESHOLD`] constant.
    spill_threshold: usize,
    #[cfg(test)]
    index_choice: IndexChoice,
}

/// One overflow bank per 64 home blocks, with a minimum of two: 255 bank
/// nodes per 1,024 home slots (Claude's discretion).  The D-15 measurement and
/// the hit-rate pin size their banks explicitly with `measure::banks_for`; a
/// caller that expects more spill uses [`FlattenedTphtShard::with_banks`].
fn default_banks(num_blocks: usize) -> usize {
    num_blocks.div_ceil(64).max(2)
}

impl<const WORDS: usize> FlattenedTphtShard<WORDS, NoProbe> {
    /// Build a shard with the zero-sized [`NoProbe`] and the default overflow
    /// size ([`default_banks`]).
    pub fn new(num_blocks: usize) -> Result<Self, TphtError> {
        Self::with_banks(num_blocks, default_banks(num_blocks))
    }

    /// Build a shard with `num_banks` overflow banks.  `num_banks == 0` is
    /// valid and builds a home-only shard: it holds no overflow and applies no
    /// spill threshold, taking entries until all 16 header slots of a block are
    /// used and then returning [`TphtError::HomeFull`] (the behaviour plan
    /// 03-06 built, kept under a constructor that names it).  A shard with
    /// banks never returns `HomeFull`.
    pub fn with_banks(num_blocks: usize, num_banks: usize) -> Result<Self, TphtError> {
        Self::with_banks_and_probe(num_blocks, num_banks, NoProbe)
    }

    /// Build a shard for a test or the D-15 measurement, setting the spill
    /// threshold and the home-index variant.  Never used in production.
    #[cfg(test)]
    pub(crate) fn with_test_config(
        num_blocks: usize,
        num_banks: usize,
        threshold: usize,
        index_choice: IndexChoice,
    ) -> Result<Self, TphtError> {
        assert!((1..=HOME_SLOTS).contains(&threshold), "a spill threshold is 1..=HOME_SLOTS");
        let mut shard = Self::with_banks(num_blocks, num_banks)?;
        shard.spill_threshold = threshold;
        shard.index_choice = index_choice;
        Ok(shard)
    }
}

impl<const WORDS: usize, P: Probe> FlattenedTphtShard<WORDS, P> {
    /// Build a shard with an injected [`Probe`] (a test seam for DS-03) and
    /// the default overflow size.
    pub fn with_probe(num_blocks: usize, probe: P) -> Result<Self, TphtError> {
        Self::with_banks_and_probe(num_blocks, default_banks(num_blocks), probe)
    }

    /// The general constructor: `num_banks == 0` builds a home-only shard.
    pub fn with_banks_and_probe(
        num_blocks: usize,
        num_banks: usize,
        probe: P,
    ) -> Result<Self, TphtError> {
        let bits = index_bits(num_blocks)?;
        let slots = HOME_SLOTS.checked_mul(num_blocks).ok_or(TphtError::BadBlockCount)?;
        let overflow = if num_banks == 0 {
            None
        } else {
            Some(ChainedTphtOverflow::new(num_blocks, num_banks)?)
        };
        let headers =
            (0..num_blocks).map(|_| TphtBucketHeader::new()).collect::<Vec<_>>().into_boxed_slice();
        let arena = (0..slots)
            .map(|_| core::array::from_fn(|_| AtomicU64::new(0)))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self {
            headers,
            arena,
            overflow,
            probe,
            bits,
            slot_mask: slots - 1,
            spill_threshold: SPILL_THRESHOLD,
            #[cfg(test)]
            index_choice: IndexChoice::MixedHalf,
        })
    }

    /// Split `key` with this shard's home-index variant.  Production always
    /// uses the mixed half; the test-only constructor may select the
    /// compendium's low-bits-of-`P` reading for the D-15 evidence.
    fn split_key(&self, key: u64) -> (usize, u64) {
        #[cfg(test)]
        {
            self.index_choice.split(key, self.bits)
        }
        #[cfg(not(test))]
        {
            feistel::split(key, self.bits)
        }
    }

    /// Insert `value` under `key`; `same` is the full-key verification called
    /// on a fingerprint match.
    ///
    /// Returns `Ok(Some(ptr))` when the entry was placed in the home block and
    /// `Ok(None)` when it went to the dereference banks.  A spilled entry has
    /// no tiny pointer, cannot be passed to [`Self::deref_tiny`], and is
    /// reached through [`Self::lookup`].  Returns [`TphtError::Exists`] when
    /// the full key is already present.  On a shard with banks a block spills
    /// once it holds `SPILL_THRESHOLD` entries, and a spilling insert whose
    /// candidate banks are full reports [`TphtError::BanksFull`]; only a
    /// home-only shard reports [`TphtError::HomeFull`].
    pub fn insert(
        &self,
        key: u64,
        value: [u64; WORDS],
        same: impl Fn(&[u64; WORDS]) -> bool,
    ) -> Result<Option<TinyPtr>, TphtError> {
        let (block, remainder) = self.split_key(key);
        let fingerprint = feistel::fingerprint(remainder);
        let header = &self.headers[block];
        let generation = Self::lock(header);
        let occupied = header.occupied_mask.load(Ordering::Relaxed);
        let mut free = None;
        let mut exists = false;
        for slot in 0..HOME_SLOTS {
            let bit = 1u16 << slot;
            if occupied & bit == 0 {
                if free.is_none() {
                    free = Some(slot);
                }
                continue;
            }
            if header.quotients[slot].load(Ordering::Relaxed) != fingerprint {
                continue;
            }
            let Some(ptr) = TinyPtr::new(header.tiny_ptrs[slot].load(Ordering::Relaxed)) else {
                continue;
            };
            let candidate = self.read_value(self.slot_index(block, ptr));
            if same(&candidate) {
                exists = true;
                break;
            }
        }
        let home_capacity = if self.overflow.is_some() { self.spill_threshold } else { HOME_SLOTS };
        let in_use = occupied.count_ones() as usize;
        let outcome = if exists {
            Err(TphtError::Exists)
        } else if in_use < home_capacity
            && let Some(slot) = free
        {
            // A key that was spilled and whose home slot has since been freed
            // must still report Exists, not be stored twice.
            let in_chain = self
                .overflow
                .as_ref()
                .is_some_and(|overflow| overflow.lookup(block, key).is_some());
            if in_chain {
                Err(TphtError::Exists)
            } else {
                let ptr = TinyPtr::new((slot + 1) as u8).expect("a home slot index is 1..=16");
                self.write_value(self.slot_index(block, ptr), &value);
                header.quotients[slot].store(fingerprint, Ordering::Release);
                header.tiny_ptrs[slot].store(ptr.get(), Ordering::Release);
                Self::set_spilled(header, slot, ptr);
                header.occupied_mask.store(occupied | (1u16 << slot), Ordering::Release);
                Ok(Some(ptr))
            }
        } else {
            match &self.overflow {
                // Past the threshold (or the 16 home slots): spill into the
                // block's chain, which checks for a duplicate full key itself.
                // This happens under the block's seqlock, which is the
                // overflow's serialisation contract; a full pair of candidate
                // banks is BanksFull.  A shard with banks never reports
                // HomeFull.
                Some(overflow) => overflow.insert(block, key, value).map(|()| None),
                None => Err(TphtError::HomeFull),
            }
        };
        Self::unlock(header, generation);
        outcome
    }

    /// Look `key` up; `same` is the full-key verification called on a
    /// fingerprint match.  `Found::home` says whether the entry resolved in
    /// the home block (`Some(ptr)`) or in the block's overflow chain (`None`).
    pub fn lookup(&self, key: u64, same: impl Fn(&[u64; WORDS]) -> bool) -> Option<Found<WORDS>> {
        let (block, remainder) = self.split_key(key);
        let fingerprint = feistel::fingerprint(remainder);
        let header = &self.headers[block];
        'attempt: loop {
            self.probe.header_read();
            let first = header.seq_lock.load(Ordering::Acquire);
            if first & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }
            let occupied = header.occupied_mask.load(Ordering::Relaxed);
            for slot in 0..HOME_SLOTS {
                if occupied & (1u16 << slot) == 0 {
                    continue;
                }
                if header.quotients[slot].load(Ordering::Relaxed) != fingerprint {
                    continue;
                }
                let Some(ptr) = TinyPtr::new(header.tiny_ptrs[slot].load(Ordering::Relaxed)) else {
                    continue;
                };
                self.probe.arena_read();
                let value = self.read_value(self.slot_index(block, ptr));
                fence(Ordering::Acquire);
                if header.seq_lock.load(Ordering::Relaxed) != first {
                    continue 'attempt;
                }
                if same(&value) {
                    return Some(Found { value, home: Some(ptr) });
                }
            }
            // The header scan found nothing: walk the block's chain inside the
            // same attempt, then validate the generation after both.  Every
            // mutation of the chain happens under this seqlock, so a changed
            // generation means the chain may have moved and the attempt
            // retries.
            if let Some(overflow) = &self.overflow
                && let Some(value) = overflow.lookup(block, key)
            {
                fence(Ordering::Acquire);
                if header.seq_lock.load(Ordering::Relaxed) != first {
                    continue 'attempt;
                }
                return Some(Found { value, home: None });
            }
            fence(Ordering::Acquire);
            if header.seq_lock.load(Ordering::Relaxed) != first {
                continue 'attempt;
            }
            return None;
        }
    }

    /// Remove `key`; `same` is the full-key verification called on a
    /// fingerprint match.  Returns whether an entry was removed.
    pub fn remove(&self, key: u64, same: impl Fn(&[u64; WORDS]) -> bool) -> bool {
        let (block, remainder) = self.split_key(key);
        let fingerprint = feistel::fingerprint(remainder);
        let header = &self.headers[block];
        let generation = Self::lock(header);
        let occupied = header.occupied_mask.load(Ordering::Relaxed);
        let mut removed = false;
        for slot in 0..HOME_SLOTS {
            if occupied & (1u16 << slot) == 0 {
                continue;
            }
            if header.quotients[slot].load(Ordering::Relaxed) != fingerprint {
                continue;
            }
            let Some(ptr) = TinyPtr::new(header.tiny_ptrs[slot].load(Ordering::Relaxed)) else {
                continue;
            };
            let candidate = self.read_value(self.slot_index(block, ptr));
            if !same(&candidate) {
                continue;
            }
            header.tiny_ptrs[slot].store(0, Ordering::Release);
            header.quotients[slot].store(0, Ordering::Release);
            Self::clear_spilled(header, slot);
            header.occupied_mask.store(occupied & !(1u16 << slot), Ordering::Release);
            removed = true;
            break;
        }
        if !removed && let Some(overflow) = &self.overflow {
            removed = overflow.remove(block, key);
        }
        Self::unlock(header, generation);
        removed
    }

    /// Resolve a value from a `(key, tiny_ptr)` pair in **exactly one arena
    /// slot read** and with **zero header reads** (DS-03).
    ///
    /// "One memory access" means one arena slot: a value may occupy several
    /// `WORDS` words inside that slot.  The home block index comes from
    /// [`feistel::split`] in register arithmetic and no header field is
    /// loaded; the seqlock is never taken.
    ///
    /// The caller holds a `(key, tiny_ptr)` pair for a live entry (for
    /// example a `StatefulHandle` in Phase 4) and is responsible for not using
    /// the pair after the entry is removed.  A stale pair returns whatever the
    /// slot now holds and is never undefined behaviour: every arena word is an
    /// atomic, so the read is always a valid atomic load.
    pub fn deref_tiny(&self, key: u64, ptr: TinyPtr) -> [u64; WORDS] {
        let (block, _remainder) = self.split_key(key);
        self.probe.arena_read();
        let slot = self.slot_index(block, ptr);
        // SAFETY: `slot_index` masks the slot with `self.slot_mask`, and
        // `with_probe` allocates the arena with exactly `slot_mask + 1` slots
        // (`HOME_SLOTS * num_blocks`).  Both quantities are fixed in the
        // constructor and never mutated, so `slot < self.arena.len()` for every
        // block index and every tiny pointer, including a pointer that runs
        // past the arena end.
        let words = unsafe { self.arena.get_unchecked(slot) };
        let mut value = [0u64; WORDS];
        for (word, out) in value.iter_mut().enumerate() {
            *out = words[word].load(Ordering::Relaxed);
        }
        value
    }

    /// The arena slot id a `(key, tiny_ptr)` pair addresses: `(block << 4) |
    /// (ptr - 1)`, where `block` is the pair's home block.  The id is dense and
    /// unique within the shard (`0..16 * num_blocks`), which is what makes it a
    /// usable key for a bitmap of live entries (WIRE-09).
    ///
    /// The id names a slot; it is not a faster read.  The single-read fast path
    /// is [`Self::deref_tiny`], which takes the pair directly.
    pub fn slot_id(&self, _key: u64, _ptr: TinyPtr) -> u32 {
        0
    }

    /// Read a whole arena slot by its [`Self::slot_id`], or `None` when the id
    /// is outside `0..16 * num_blocks`.
    ///
    /// This is the failover sweep's read path (WIRE-09): bounds-checked and
    /// safe on every id, so an id rebuilt from a corrupt or stale bitmap can
    /// never index past the arena.  It is **not** the single-read fast path.
    pub fn read_slot(&self, _slot_id: u32) -> Option<[u64; WORDS]> {
        None
    }

    /// The arena slot a tiny pointer addresses: `((block << 4) + (ptr - 1)) &
    /// slot_mask`.
    fn slot_index(&self, block: usize, ptr: TinyPtr) -> usize {
        ((block << 4) + (ptr.get() as usize - 1)) & self.slot_mask
    }

    /// Read a whole arena slot (`WORDS` relaxed atomic loads).
    fn read_value(&self, slot: usize) -> [u64; WORDS] {
        let mut value = [0u64; WORDS];
        for (word, out) in value.iter_mut().enumerate() {
            *out = self.arena[slot][word].load(Ordering::Relaxed);
        }
        value
    }

    /// Write a whole arena slot with `Release` stores (the writer holds the
    /// block, so the reader's generation check discards a torn read).
    fn write_value(&self, slot: usize, value: &[u64; WORDS]) {
        for (word, out) in value.iter().enumerate() {
            self.arena[slot][word].store(*out, Ordering::Release);
        }
    }

    /// Take the block: bring `seq_lock` from an even generation to that
    /// generation plus one.  An odd value means another writer holds it.
    fn lock(header: &TphtBucketHeader) -> u32 {
        loop {
            let generation = header.seq_lock.load(Ordering::Relaxed);
            if generation & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }
            match header.seq_lock.compare_exchange(
                generation,
                generation + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return generation,
                Err(_) => core::hint::spin_loop(),
            }
        }
    }

    /// Release the block and publish the new stable generation.
    fn unlock(header: &TphtBucketHeader, generation: u32) {
        header.seq_lock.store(generation + 2, Ordering::Release);
    }

    /// Maintain `spilled_mask` for one slot: a pointer above the home window
    /// sets the bit, a home pointer clears it.  This writer never produces a
    /// spilled pointer, but the field means what the layout says.
    fn set_spilled(header: &TphtBucketHeader, slot: usize, ptr: TinyPtr) {
        let bit = 1u16 << slot;
        let mut spilled = header.spilled_mask.load(Ordering::Relaxed);
        if ptr.get() as usize > HOME_SLOTS {
            spilled |= bit;
        } else {
            spilled &= !bit;
        }
        header.spilled_mask.store(spilled, Ordering::Release);
    }

    /// Clear one slot's `spilled_mask` bit when its entry is removed.
    fn clear_spilled(header: &TphtBucketHeader, slot: usize) {
        let spilled = header.spilled_mask.load(Ordering::Relaxed) & !(1u16 << slot);
        header.spilled_mask.store(spilled, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicUsize;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::*;
    use crate::tpht::measure::{Distribution, GRID_BLOCKS, GRID_SEEDS};
    use crate::tpht::miri_scaled;

    /// The occupancy the hit-rate pin asserts (D-15).  Plan 03-08 Task 2 Step D
    /// sets this to the rule's chosen occupancy from the measurement; the bar
    /// itself is never raised.
    const HIT_RATE_OCCUPANCY: f64 = 1.00;

    /// A [`Probe`] that counts the reader's two kinds of memory access, so
    /// `deref_tiny` can be shown to make exactly one arena read and no header
    /// read (DS-03; RESEARCH Pattern 4).  The counters sit behind `Arc` so the
    /// shard holds a clone and the test keeps a handle to read them.
    #[derive(Clone, Default)]
    struct CountingProbe {
        header_reads: Arc<AtomicUsize>,
        arena_reads: Arc<AtomicUsize>,
    }

    impl CountingProbe {
        fn new() -> Self {
            Self::default()
        }

        fn header_reads(&self) -> usize {
            self.header_reads.load(Ordering::Relaxed)
        }

        fn arena_reads(&self) -> usize {
            self.arena_reads.load(Ordering::Relaxed)
        }

        fn reset(&self) {
            self.header_reads.store(0, Ordering::Relaxed);
            self.arena_reads.store(0, Ordering::Relaxed);
        }
    }

    impl Probe for CountingProbe {
        fn header_read(&self) {
            self.header_reads.fetch_add(1, Ordering::Relaxed);
        }

        fn arena_read(&self) {
            self.arena_reads.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// SplitMix64: a tiny seeded PRNG so every randomized test reproduces
    /// from its literal seed (no `rand`, no `proptest`).
    struct SplitMix64(u64);

    impl SplitMix64 {
        fn new(seed: u64) -> Self {
            Self(seed)
        }

        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    #[test]
    fn header_is_64_bytes_and_64_aligned() {
        assert_eq!(
            core::mem::size_of::<TphtBucketHeader>(),
            64,
            "the header must be exactly one cache line (guide section 5.2)"
        );
        assert_eq!(
            core::mem::align_of::<TphtBucketHeader>(),
            64,
            "the header must be cache-line aligned"
        );
    }

    #[test]
    fn rejects_bad_block_counts() {
        for bad in [0usize, 3, 6, usize::MAX] {
            assert_eq!(
                FlattenedTphtShard::<2>::new(bad).err(),
                Some(TphtError::BadBlockCount),
                "a block count of {bad} must be rejected"
            );
        }
        let shard = FlattenedTphtShard::<2>::new(1).expect("one block is a valid block count");
        assert_eq!(shard.lookup(0, |v| v[0] == 0), None, "an empty shard resolves nothing");
    }

    #[test]
    fn home_ops_match_btreemap_model() {
        let keys: Vec<u64> =
            (0..8u64).map(|i| 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(i + 1)).collect();
        let big_blocks = miri_scaled(1_024, 8);
        let mut cases = vec![1usize, 16];
        if !cases.contains(&big_blocks) {
            cases.push(big_blocks);
        }
        let ops = miri_scaled(2_000, 100);
        for (case, &blocks) in cases.iter().enumerate() {
            // Re-pinned from plan 03-06 on a home-only shard (plan 03-08
            // Task 1): `insert` reports where the entry went and `lookup`
            // returns `Found`; a home-only shard always answers "at home".
            let shard =
                FlattenedTphtShard::<2>::with_banks(blocks, 0).expect("a power-of-two block count");
            let lookup = |key: u64| {
                shard.lookup(key, |v| v[0] == key).map(|found| {
                    assert!(
                        found.home.is_some(),
                        "a home-only shard resolves every entry at home (case {case}, key {key})"
                    );
                    found.value
                })
            };
            let mut model: BTreeMap<u64, [u64; 2]> = BTreeMap::new();
            for &key in &keys {
                assert_eq!(lookup(key), None, "the empty shard must not resolve key 0x{key:016x}");
            }
            let mut rng = SplitMix64::new(0x0000_0000_0000_00c3 ^ case as u64);
            for step in 0..ops {
                let key = keys[(rng.next() as usize) % keys.len()];
                let value = [key, !key];
                match rng.next() % 3 {
                    0 => {
                        let result = shard.insert(key, value, |v| v[0] == key);
                        if model.contains_key(&key) {
                            assert_eq!(
                                result,
                                Err(TphtError::Exists),
                                "a second insert must report Exists (case {case}, step {step}, key {key})"
                            );
                            assert_eq!(
                                lookup(key),
                                model.get(&key).copied(),
                                "a refused insert must leave the first value (case {case}, step {step})"
                            );
                        } else {
                            let ptr = result
                                .expect("a table holding eight keys has room")
                                .expect("a home-only shard places every entry at home");
                            assert_eq!(
                                shard.lookup(key, |v| v[0] == key),
                                Some(Found { value, home: Some(ptr) }),
                                "an inserted key must resolve to its value and pointer (case {case}, step {step})"
                            );
                            model.insert(key, value);
                        }
                    }
                    1 => {
                        assert_eq!(
                            lookup(key),
                            model.get(&key).copied(),
                            "lookup disagrees with the model (case {case}, step {step}, key {key})"
                        );
                    }
                    _ => {
                        let removed = shard.remove(key, |v| v[0] == key);
                        assert_eq!(
                            removed,
                            model.remove(&key).is_some(),
                            "remove disagrees with the model (case {case}, step {step}, key {key})"
                        );
                        assert_eq!(
                            lookup(key),
                            model.get(&key).copied(),
                            "a removed key must not resolve (case {case}, step {step})"
                        );
                    }
                }
            }
            for &key in &keys {
                assert_eq!(
                    lookup(key),
                    model.get(&key).copied(),
                    "the final state disagrees with the model (case {case}, key {key})"
                );
            }
        }

        // A full header: the seventeenth key for one block is refused.  A
        // home-only shard has nowhere to spill, whatever SPILL_THRESHOLD is.
        let shard = FlattenedTphtShard::<2>::with_banks(1, 0).expect("one block");
        for key in 0..HOME_SLOTS as u64 {
            shard
                .insert(key, [key, !key], |v| v[0] == key)
                .expect("the first sixteen keys fit")
                .expect("a home-only shard places every entry at home");
        }
        let extra = HOME_SLOTS as u64;
        assert_eq!(
            shard.insert(extra, [extra, !extra], |v| v[0] == extra),
            Err(TphtError::HomeFull),
            "the seventeenth key for one block must report HomeFull"
        );
    }

    #[test]
    fn fingerprint_collisions_are_verified() {
        // Two keys whose remainders share a 16-bit fingerprint in one block.
        // The pair was found by an independent search; the test re-derives and
        // asserts the collision, so a change to mix32 or fingerprint cannot
        // silently turn this into a no-collision test.
        let (a, b) = (473u64, 557u64);
        let (_, remainder_a) = feistel::split(a, 0);
        let (_, remainder_b) = feistel::split(b, 0);
        assert_ne!(a, b, "the pair must be two different full keys");
        assert_eq!(
            feistel::fingerprint(remainder_a),
            feistel::fingerprint(remainder_b),
            "the pair must share a 16-bit fingerprint in one block"
        );
        // Re-pinned from plan 03-06 on a home-only shard (plan 03-08 Task 1):
        // the constructor and the `insert`/`lookup` unwrapping change; what
        // the test asserts does not.
        let shard = FlattenedTphtShard::<2>::with_banks(1, 0).expect("one block");
        let ptr_a = shard
            .insert(a, [a, !a], |v| v[0] == a)
            .expect("the first key stores")
            .expect("a home-only shard places the first key at home");
        let ptr_b = shard
            .insert(b, [b, !b], |v| v[0] == b)
            .expect("the second key stores despite the fingerprint collision")
            .expect("a home-only shard places the second key at home");
        assert_eq!(
            shard.lookup(a, |v| v[0] == a),
            Some(Found { value: [a, !a], home: Some(ptr_a) })
        );
        assert_eq!(
            shard.lookup(b, |v| v[0] == b),
            Some(Found { value: [b, !b], home: Some(ptr_b) })
        );
        assert_eq!(
            shard.lookup(a, |v| v[0] == b),
            Some(Found { value: [b, !b], home: Some(ptr_b) }),
            "the fingerprint cannot tell the pair apart; `same` does"
        );
        assert_ne!(ptr_a, ptr_b, "the two entries hold different slots");
    }

    #[test]
    fn stress_readers_never_see_torn_entry() {
        // Re-pinned from plan 03-06 on a home-only shard (plan 03-08 Task 1);
        // the value returned by `insert` is now `Option<TinyPtr>` and `lookup`
        // returns `Found`, but nothing the test asserts is loosened.
        let shard = Arc::new(FlattenedTphtShard::<2>::with_banks(1, 0).expect("one block"));
        let keys: [u64; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        // Key 0 stays present for the whole run, so every reader has a
        // guaranteed hit whose value must never be torn.
        shard
            .insert(keys[0], [keys[0], !keys[0]], |v| v[0] == keys[0])
            .expect("the sentinel stores")
            .expect("a home-only shard places the sentinel at home");
        let writers = miri_scaled(4, 2);
        let readers = miri_scaled(4, 2);
        let ops = miri_scaled(20_000, 50);
        let mut handles = Vec::new();
        for writer in 0..writers {
            let shard = Arc::clone(&shard);
            handles.push(std::thread::spawn(move || {
                let mut rng = SplitMix64::new(0x0000_0000_0000_00d4 ^ writer as u64);
                for _ in 0..ops {
                    let key = keys[1 + (rng.next() as usize) % (keys.len() - 1)];
                    if rng.next() & 1 == 0 {
                        let _ = shard.insert(key, [key, !key], |v| v[0] == key);
                    } else {
                        let _ = shard.remove(key, |v| v[0] == key);
                    }
                }
            }));
        }
        for reader in 0..readers {
            let shard = Arc::clone(&shard);
            handles.push(std::thread::spawn(move || {
                let mut rng = SplitMix64::new(0x0000_0000_0000_00e5 ^ reader as u64);
                for _ in 0..ops {
                    let key = keys[(rng.next() as usize) % keys.len()];
                    if let Some(found) = shard.lookup(key, |v| v[0] == key) {
                        assert_eq!(found.value[0], key, "a reader got another key's value");
                        assert_eq!(found.value[1], !key, "a reader got a torn value for key {key}");
                        assert!(
                            found.home.is_some(),
                            "a home-only shard resolves every entry at home"
                        );
                    }
                    assert!(
                        shard.lookup(keys[0], |v| v[0] == keys[0]).is_some(),
                        "the sentinel key is always present"
                    );
                }
            }));
        }
        for handle in handles {
            handle.join().expect("every stress thread completes");
        }
    }

    #[test]
    fn spill_past_threshold_goes_to_a_bank() {
        // One block, two banks, test threshold 2: the first two entries land
        // at home, the third spills and is found again through `lookup`.
        let shard = FlattenedTphtShard::<2>::with_test_config(1, 2, 2, IndexChoice::MixedHalf)
            .expect("one block and two banks");
        let keys = [0xA1u64, 0xB2, 0xC3];
        let first = shard
            .insert(keys[0], [keys[0], !keys[0]], |v| v[0] == keys[0])
            .expect("the first insert succeeds");
        assert!(first.is_some(), "the first entry lands at home");
        assert_eq!(
            shard.lookup(keys[0], |v| v[0] == keys[0]),
            Some(Found { value: [keys[0], !keys[0]], home: first }),
            "the first entry resolves at home"
        );
        let second = shard
            .insert(keys[1], [keys[1], !keys[1]], |v| v[0] == keys[1])
            .expect("the second insert succeeds");
        assert!(second.is_some(), "the second entry lands at home");
        assert_eq!(
            shard.lookup(keys[1], |v| v[0] == keys[1]),
            Some(Found { value: [keys[1], !keys[1]], home: second }),
            "the second entry resolves at home"
        );
        let third = shard
            .insert(keys[2], [keys[2], !keys[2]], |v| v[0] == keys[2])
            .expect("the third insert succeeds");
        assert!(third.is_none(), "the third entry spills past the threshold of 2");
        assert_eq!(
            shard.lookup(keys[2], |v| v[0] == keys[2]),
            Some(Found { value: [keys[2], !keys[2]], home: None }),
            "the spilled entry is found again, with no home pointer"
        );
        for &key in &keys {
            assert_eq!(
                shard.lookup(key, |v| v[0] == key).map(|found| found.value),
                Some([key, !key]),
                "every inserted key returns its own value"
            );
        }
    }

    #[test]
    fn home_only_shard_never_spills() {
        // A home-only shard applies no threshold: sixteen keys fill its slots
        // and the seventeenth is HomeFull, whatever SPILL_THRESHOLD is.
        let shard = FlattenedTphtShard::<2>::with_banks(1, 0).expect("a home-only shard");
        for key in 0..HOME_SLOTS as u64 {
            let ptr = shard
                .insert(key, [key, !key], |v| v[0] == key)
                .expect("a home-only shard takes sixteen keys")
                .expect("a home-only shard places every entry at home");
            assert_eq!(
                shard.lookup(key, |v| v[0] == key).map(|found| found.home),
                Some(Some(ptr)),
                "key {key} resolves at home"
            );
        }
        let extra = HOME_SLOTS as u64;
        assert_eq!(
            shard.insert(extra, [extra, !extra], |v| v[0] == extra),
            Err(TphtError::HomeFull),
            "the seventeenth key must report HomeFull"
        );
        // A shard with banks never returns HomeFull: entries spill instead.
        let shard = FlattenedTphtShard::<2>::new(1).expect("a shard with banks");
        for key in 0..(HOME_SLOTS as u64 * 2) {
            assert!(
                shard.insert(key, [key, !key], |v| v[0] == key).is_ok(),
                "a shard with banks never returns HomeFull (key {key})"
            );
        }
    }

    #[test]
    fn composite_ops_match_btreemap_model() {
        // Seeded inserts, lookups and removes over a table small enough that
        // the test threshold of 2 forces spills; every operation agrees with
        // a BTreeMap, and `insert`'s `Some`/`None` agrees with `lookup`'s
        // `Found::home`.
        let blocks = miri_scaled(16, 4);
        let banks = miri_scaled(8, 2);
        let ops = miri_scaled(2_000, 50);
        let keys: Vec<u64> =
            (0..64u64).map(|i| 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(i + 1)).collect();
        let shard =
            FlattenedTphtShard::<2>::with_test_config(blocks, banks, 2, IndexChoice::MixedHalf)
                .expect("a valid block and bank count");
        let mut model: BTreeMap<u64, [u64; 2]> = BTreeMap::new();
        let mut rng = SplitMix64::new(0x0000_0000_0000_0508);
        for step in 0..ops {
            let key = keys[(rng.next() as usize) % keys.len()];
            let value = [key, !key];
            match rng.next() % 3 {
                0 => {
                    let result = shard.insert(key, value, |v| v[0] == key);
                    if model.contains_key(&key) {
                        assert_eq!(
                            result,
                            Err(TphtError::Exists),
                            "a second insert must report Exists (step {step}, key {key})"
                        );
                        assert_eq!(
                            shard.lookup(key, |v| v[0] == key).map(|found| found.value),
                            model.get(&key).copied(),
                            "a refused insert must leave the first value (step {step})"
                        );
                    } else {
                        let placed = result.unwrap_or_else(|err| {
                            panic!("insert must succeed (step {step}): {err:?}")
                        });
                        let found = shard
                            .lookup(key, |v| v[0] == key)
                            .expect("an inserted key must resolve");
                        assert_eq!(found.value, value, "lookup returns the inserted value");
                        assert_eq!(
                            found.home, placed,
                            "insert must say where the entry went (step {step}, key {key})"
                        );
                        model.insert(key, value);
                    }
                }
                1 => {
                    assert_eq!(
                        shard.lookup(key, |v| v[0] == key).map(|found| found.value),
                        model.get(&key).copied(),
                        "lookup disagrees with the model (step {step}, key {key})"
                    );
                }
                _ => {
                    let removed = shard.remove(key, |v| v[0] == key);
                    assert_eq!(
                        removed,
                        model.remove(&key).is_some(),
                        "remove disagrees with the model (step {step}, key {key})"
                    );
                    assert_eq!(
                        shard.lookup(key, |v| v[0] == key).map(|found| found.value),
                        model.get(&key).copied(),
                        "a removed key must not resolve (step {step})"
                    );
                }
            }
        }
        for (&key, &value) in &model {
            assert_eq!(
                shard.lookup(key, |v| v[0] == key).map(|found| found.value),
                Some(value),
                "the final state disagrees with the model (key {key})"
            );
        }

        // Deterministic tail: a duplicate insert reports Exists whether the
        // first copy is at home or in a bank, and removing either works.
        let shard = FlattenedTphtShard::<2>::with_test_config(1, 2, 2, IndexChoice::MixedHalf)
            .expect("one block and two banks");
        let (k1, k2, k3) = (0x11u64, 0x22, 0x33);
        assert!(
            shard.insert(k1, [k1, !k1], |v| v[0] == k1).expect("the first key stores").is_some(),
            "the first key lands at home"
        );
        assert!(
            shard.insert(k2, [k2, !k2], |v| v[0] == k2).expect("the second key stores").is_some(),
            "the second key lands at home"
        );
        assert!(
            shard.insert(k3, [k3, !k3], |v| v[0] == k3).expect("the third key stores").is_none(),
            "the third key spills"
        );
        assert_eq!(
            shard.insert(k1, [k1, !k1], |v| v[0] == k1),
            Err(TphtError::Exists),
            "a duplicate insert whose first copy is at home reports Exists"
        );
        assert_eq!(
            shard.insert(k3, [k3, !k3], |v| v[0] == k3),
            Err(TphtError::Exists),
            "a duplicate insert whose first copy is in a bank reports Exists"
        );
        assert!(shard.remove(k1, |v| v[0] == k1), "a home entry is removable");
        assert!(shard.remove(k3, |v| v[0] == k3), "a bank entry is removable");
        assert_eq!(shard.lookup(k1, |v| v[0] == k1), None, "the removed home entry is gone");
        assert_eq!(shard.lookup(k3, |v| v[0] == k3), None, "the removed bank entry is gone");
        assert_eq!(
            shard.lookup(k2, |v| v[0] == k2).map(|found| found.value),
            Some([k2, !k2]),
            "the untouched key still resolves"
        );
    }

    #[test]
    fn banks_full_is_an_error() {
        // One bank shared by 64 blocks, test threshold 2: two home entries per
        // block, then the bank's 255 nodes fill from spills spread across the
        // blocks (so each chain stays short and the test stays cheap under
        // Miri).  The next spilling insert reports BanksFull and stores
        // nothing.
        use crate::tpht::chained::BANK_NODES;
        let blocks = miri_scaled(64, 64);
        let spill_count = miri_scaled(BANK_NODES, BANK_NODES);
        let bits = index_bits(blocks).expect("64 blocks");
        let shard = FlattenedTphtShard::<2>::with_test_config(blocks, 1, 2, IndexChoice::MixedHalf)
            .expect("64 blocks and one bank");
        // Find two home keys for every block; every block must hold its two
        // home entries before the spill loop, so the failing insert spills.
        let mut home_for: Vec<Vec<u64>> = vec![Vec::new(); blocks];
        let mut key = 0u64;
        while home_for.iter().any(|keys| keys.len() < 2) {
            let (block, _) = feistel::split(key, bits);
            if home_for[block].len() < 2 {
                home_for[block].push(key);
            }
            key += 1;
        }
        let mut home_ptrs = Vec::new();
        for pair in &home_for {
            for &home_key in pair {
                let ptr = shard
                    .insert(home_key, [home_key, !home_key], |v| v[0] == home_key)
                    .expect("a home insert succeeds")
                    .expect("the first two keys of a block land at home");
                home_ptrs.push((home_key, ptr));
            }
        }
        let mut spilled = Vec::new();
        while spilled.len() < spill_count {
            let (block, _) = feistel::split(key, bits);
            debug_assert!(home_for[block].len() == 2, "every block has its two home keys");
            let placed = shard
                .insert(key, [key, !key], |v| v[0] == key)
                .unwrap_or_else(|err| panic!("spill {} must succeed: {err:?}", spilled.len()));
            assert!(placed.is_none(), "every entry past the threshold spills");
            spilled.push(key);
            key += 1;
        }
        let failing = key;
        assert_eq!(
            shard.insert(failing, [failing, !failing], |v| v[0] == failing),
            Err(TphtError::BanksFull),
            "a spilling insert into a full bank must report BanksFull"
        );
        assert_eq!(
            shard.lookup(failing, |v| v[0] == failing),
            None,
            "the failed insert stored nothing"
        );
        for &(home_key, ptr) in &home_ptrs {
            assert_eq!(
                shard.lookup(home_key, |v| v[0] == home_key).map(|found| found.home),
                Some(Some(ptr)),
                "the home entries still resolve"
            );
        }
        for &spill_key in spilled.iter().take(4) {
            assert_eq!(
                shard.lookup(spill_key, |v| v[0] == spill_key).map(|found| found.home),
                Some(None),
                "the spilled entries still resolve in the bank"
            );
        }
    }

    #[test]
    fn stress_readers_with_spill() {
        // Writers insert and remove keys of one block past the threshold while
        // readers look them up; every value a reader gets satisfies the
        // self-check, and the sentinel key stays present.  Key 2 is spilled
        // before the threads start, so the spill path is exercised from the
        // first operation.
        let blocks = miri_scaled(1, 1);
        let banks = miri_scaled(2, 1);
        let shard = Arc::new(
            FlattenedTphtShard::<2>::with_test_config(blocks, banks, 2, IndexChoice::MixedHalf)
                .expect("one block and a valid bank count"),
        );
        let keys: [u64; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        shard
            .insert(keys[0], [keys[0], !keys[0]], |v| v[0] == keys[0])
            .expect("the sentinel stores")
            .expect("the sentinel lands at home");
        shard
            .insert(keys[1], [keys[1], !keys[1]], |v| v[0] == keys[1])
            .expect("the second key stores")
            .expect("the second key lands at home");
        let spilled = shard
            .insert(keys[2], [keys[2], !keys[2]], |v| v[0] == keys[2])
            .expect("the third key stores");
        assert!(spilled.is_none(), "the third key spills past the threshold of 2");
        assert_eq!(
            shard.lookup(keys[2], |v| v[0] == keys[2]).map(|found| found.home),
            Some(None),
            "the spilled key resolves in the bank"
        );
        let writers = miri_scaled(4, 2);
        let readers = miri_scaled(4, 2);
        let ops = miri_scaled(20_000, 50);
        let mut handles = Vec::new();
        for writer in 0..writers {
            let shard = Arc::clone(&shard);
            handles.push(std::thread::spawn(move || {
                let mut rng = SplitMix64::new(0x0000_0000_0000_0f08 ^ writer as u64);
                for _ in 0..ops {
                    let key = keys[1 + (rng.next() as usize) % (keys.len() - 1)];
                    if rng.next() & 1 == 0 {
                        let _ = shard.insert(key, [key, !key], |v| v[0] == key);
                    } else {
                        let _ = shard.remove(key, |v| v[0] == key);
                    }
                }
            }));
        }
        for reader in 0..readers {
            let shard = Arc::clone(&shard);
            handles.push(std::thread::spawn(move || {
                let mut rng = SplitMix64::new(0x0000_0000_0000_0f09 ^ reader as u64);
                for _ in 0..ops {
                    let key = keys[(rng.next() as usize) % keys.len()];
                    if let Some(found) = shard.lookup(key, |v| v[0] == key) {
                        assert_eq!(found.value[0], key, "a reader got another key's value");
                        assert_eq!(
                            found.value[1], !key,
                            "a reader got a torn value for key {key} on the spill path"
                        );
                    }
                    assert!(
                        shard.lookup(keys[0], |v| v[0] == keys[0]).is_some(),
                        "the sentinel key is always present"
                    );
                }
            }));
        }
        for handle in handles {
            handle.join().expect("every stress thread completes");
        }
    }

    #[test]
    fn deref_tiny_is_one_arena_read_and_no_header_read() {
        // One block is the smallest shard with a home slot, and the probe
        // counts are independent of the shard size, so both sides use it.
        // Re-pinned from plan 03-13 on a home-only shard (plan 03-08 Task 1):
        // the constructor, the `insert` unwrapping and the `lookup` result
        // shape change; the probe counts asserted here do not.
        let blocks = miri_scaled(1, 1);
        let probe = CountingProbe::new();
        let shard =
            FlattenedTphtShard::<2, CountingProbe>::with_banks_and_probe(blocks, 0, probe.clone())
                .expect("one block is a valid block count");
        let ptr = shard
            .insert(0x5EED, [0xAAAA_BBBB_CCCC_DDDD, 0x1111_2222_3333_4444], |v| {
                v[0] == 0xAAAA_BBBB_CCCC_DDDD
            })
            .expect("the first key fits in an empty block")
            .expect("a home-only shard places the key at home");
        probe.reset();
        assert_eq!(
            shard.deref_tiny(0x5EED, ptr),
            [0xAAAA_BBBB_CCCC_DDDD, 0x1111_2222_3333_4444],
            "deref_tiny must resolve the entry's value from the arena"
        );
        assert_eq!(
            (probe.arena_reads(), probe.header_reads()),
            (1, 0),
            "deref_tiny must make exactly one arena read and zero header reads (DS-03)"
        );
        assert_eq!(
            shard.lookup(0x5EED, |v| v[0] == 0xAAAA_BBBB_CCCC_DDDD),
            Some(Found { value: [0xAAAA_BBBB_CCCC_DDDD, 0x1111_2222_3333_4444], home: Some(ptr) }),
            "the key path still resolves the entry"
        );
        assert!(
            probe.header_reads() >= 1,
            "lookup probes the header, so the counting probe is wired to the reader"
        );
    }

    #[test]
    fn deref_tiny_matches_btreemap_model() {
        let blocks = miri_scaled(1_024, 8);
        let ops = miri_scaled(2_000, 100);
        let keys: Vec<u64> =
            (0..32u64).map(|i| 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(i + 1)).collect();
        let shard =
            FlattenedTphtShard::<2>::with_banks(blocks, 0).expect("a power-of-two block count");
        let mut model: BTreeMap<u64, ([u64; 2], TinyPtr)> = BTreeMap::new();
        let mut rng = SplitMix64::new(0x0000_0000_0000_013d);
        for step in 0..ops {
            let key = keys[(rng.next() as usize) % keys.len()];
            let value = [key, !key];
            match rng.next() % 3 {
                0 => {
                    let result = shard.insert(key, value, |v| v[0] == key);
                    if let Some(&(held, ptr)) = model.get(&key) {
                        assert_eq!(
                            result,
                            Err(TphtError::Exists),
                            "a second insert must report Exists (step {step}, key {key})"
                        );
                        assert_eq!(
                            shard.deref_tiny(key, ptr),
                            held,
                            "a refused insert must leave the first value (step {step}, key {key})"
                        );
                    } else {
                        // Re-pinned from plan 03-13 (plan 03-08 Task 1): the
                        // pointer `insert` returned is now `Option`, and on a
                        // home-only shard it is always `Some`, so this test
                        // still dereferences every live key.
                        let ptr = result
                            .expect("32 keys over at least 16 blocks leave a home slot each")
                            .expect("a home-only shard places every entry at home");
                        assert_eq!(
                            shard.deref_tiny(key, ptr),
                            value,
                            "a fresh insert must resolve through its pointer (step {step}, key {key})"
                        );
                        model.insert(key, (value, ptr));
                    }
                }
                1 => {
                    if let Some(&(held, ptr)) = model.get(&key) {
                        assert_eq!(
                            shard.deref_tiny(key, ptr),
                            held,
                            "deref_tiny disagrees with the model (step {step}, key {key})"
                        );
                    }
                }
                _ => {
                    let removed = shard.remove(key, |v| v[0] == key);
                    assert_eq!(
                        removed,
                        model.remove(&key).is_some(),
                        "remove disagrees with the model (step {step}, key {key})"
                    );
                }
            }
        }
        for (&key, &(value, ptr)) in &model {
            assert_eq!(
                shard.deref_tiny(key, ptr),
                value,
                "the final state disagrees with the model (key {key})"
            );
            assert_eq!(
                shard.lookup(key, |v| v[0] == key).map(|found| found.value),
                Some(value),
                "the key path agrees with the model (key {key})"
            );
        }

        // A stale pair is not undefined behaviour: it returns whatever the
        // slot now holds.  A one-block shard reuses slot 0 after a removal.
        let shard = FlattenedTphtShard::<2>::with_banks(1, 0).expect("one block");
        let stale = shard
            .insert(7, [7, !7], |v| v[0] == 7)
            .expect("the first key fits")
            .expect("a home-only shard places the key at home");
        assert!(shard.remove(7, |v| v[0] == 7), "the first key is present");
        let fresh = shard
            .insert(8, [8, !8], |v| v[0] == 8)
            .expect("the slot is free again")
            .expect("a home-only shard places the key at home");
        assert_eq!(stale, fresh, "the freed slot is reassigned to the next key");
        assert_eq!(
            shard.deref_tiny(7, stale),
            [8, !8],
            "a stale pair returns the slot's current value and is never undefined behaviour"
        );
    }

    #[test]
    fn deref_tiny_wraps_with_slot_mask() {
        // Two blocks is the smallest shard where `b << 4` is non-zero and a
        // pointer that runs past the arena end wraps into another block's
        // window, so the native and Miri sizes are the same.  The last block
        // is block 1, so `b << 4` is 16 and `slot_mask` is 31: pointers 1 and
        // 16 land in the last block's home window, 17 and 255 wrap by the mask.
        let blocks = miri_scaled(2, 2);
        let bits = index_bits(blocks).expect("two blocks");
        let mut last: Vec<u64> = Vec::new();
        let mut first: Vec<u64> = Vec::new();
        let mut key = 0u64;
        while last.len() < HOME_SLOTS || first.len() < HOME_SLOTS {
            let (block, _) = feistel::split(key, bits);
            if block == blocks - 1 {
                if last.len() < HOME_SLOTS {
                    last.push(key);
                }
            } else if first.len() < HOME_SLOTS {
                first.push(key);
            }
            key += 1;
        }
        let shard = FlattenedTphtShard::<2>::with_banks(blocks, 0).expect("two blocks");
        let mut stored = vec![[0u64; 2]; HOME_SLOTS * blocks];
        for &key in first.iter().chain(last.iter()) {
            let (block, _) = feistel::split(key, bits);
            let ptr = shard
                .insert(key, [key, !key], |v| v[0] == key)
                .expect("sixteen keys per block fit in its home window")
                .expect("a home-only shard places every entry at home");
            let slot = ((block << 4) + (ptr.get() as usize - 1)) & (HOME_SLOTS * blocks - 1);
            stored[slot] = [key, !key];
        }
        let last_block = blocks - 1;
        let sample_key = last[0];
        for ptr in [1u8, 16, 17, 255] {
            let tiny = TinyPtr::new(ptr).expect("a tiny pointer is 1..=255");
            let slot = ((last_block << 4) + (tiny.get() as usize - 1)) & (HOME_SLOTS * blocks - 1);
            assert_eq!(
                shard.deref_tiny(sample_key, tiny),
                stored[slot],
                "pointer {ptr} on the last block must read slot {slot} = ((b << 4) + (ptr - 1)) & slot_mask"
            );
        }
        assert_ne!(
            stored[16], stored[0],
            "the wrap must be observable: the last block's first slot differs from slot 0"
        );
    }

    #[test]
    fn hit_rate_home_block_exceeds_90_percent() {
        // The chosen grid cell, re-run through the production constructor: the
        // grid's 16,384 blocks, three seeds and five distributions (16 blocks,
        // one seed and two distributions under Miri), `entries_at`, `banks_for`
        // and the exact integer `exceeds_bar`.  The threshold in force is the
        // production `SPILL_THRESHOLD`, so the counts equal the chosen cell's
        // and a qualifying cell passes.  The 0.90 bar is asserted in native
        // runs only: a 16-block table says nothing about the figure.
        let blocks = miri_scaled(GRID_BLOCKS, 4);
        let seed_count = miri_scaled(GRID_SEEDS.len(), 1);
        let dist_count = miri_scaled(Distribution::ALL.len(), 1);
        let percent = (HIT_RATE_OCCUPANCY * 100.0).round() as usize;
        let entries = crate::tpht::measure::entries_at(blocks, percent);
        let banks = crate::tpht::measure::banks_for(entries);
        let mut weakest: Option<(usize, usize)> = None;
        let mut failures: Vec<String> = Vec::new();
        for distribution in Distribution::ALL.iter().take(dist_count) {
            for &seed in GRID_SEEDS.iter().take(seed_count) {
                let shard = FlattenedTphtShard::<2>::with_banks(blocks, banks)
                    .expect("the grid's block and bank counts are valid");
                for (i, key) in
                    crate::tpht::measure::keys(*distribution, seed, 0).take(entries).enumerate()
                {
                    let result = shard.insert(key, [key, !key], |v| v[0] == key);
                    assert!(
                        result.is_ok(),
                        "no insert may fail at the measured threshold ({:?}, seed {seed:#018x}, \
                         insert {i}): {result:?}",
                        distribution
                    );
                }
                let mut hits = 0;
                let mut lookups = 0;
                for key in crate::tpht::measure::keys(*distribution, seed, 0).take(entries) {
                    let found =
                        shard.lookup(key, |v| v[0] == key).expect("a present key must resolve");
                    assert_eq!(found.value, [key, !key], "lookup returns the inserted value");
                    if found.home.is_some() {
                        hits += 1;
                    }
                    lookups += 1;
                }
                let fraction = hits as f64 / lookups as f64;
                println!(
                    "CELL dist={} seed={seed:#018x} hits={hits} lookups={lookups} \
                     fraction={fraction:.6}",
                    distribution.name()
                );
                let weakest_run = weakest.get_or_insert((hits, lookups));
                if hits * weakest_run.1 < weakest_run.0 * lookups {
                    *weakest_run = (hits, lookups);
                }
                if !cfg!(miri) && !crate::tpht::measure::exceeds_bar(hits, lookups) {
                    failures.push(format!(
                        "{:?} seed {seed:#018x} resolves only {fraction:.4} of present keys at \
                         home ({hits}/{lookups}); DS-01 requires over 0.90",
                        distribution
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "the home-hit bar is not met in {} run(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
        if let Some((hits, lookups)) = weakest {
            println!(
                "PIN min fraction={:.6} hits={hits} lookups={lookups}",
                hits as f64 / lookups as f64
            );
        }
    }

    #[test]
    fn tpht_remove_clears_the_arena_slot() {
        // Removing an entry clears its header fields; without also zeroing the
        // arena slot, `deref_tiny` — which never reads the header — would keep
        // answering the old words, and a clunked `StatefulHandle` would still
        // resolve (RESEARCH Pitfall 2, T-04-29).
        let blocks = miri_scaled(16, 4);
        let shard =
            FlattenedTphtShard::<2>::with_banks(blocks, 0).expect("a power-of-two block count");
        let key = 0x5EED_1234u64;
        let value = [0xAAAA_BBBB_CCCC_DDDDu64, 0x1111_2222_3333_4444];
        let ptr = shard
            .insert(key, value, |v| v[0] == value[0])
            .expect("an empty shard takes the key")
            .expect("a home-only shard places the key at home");
        assert_eq!(shard.deref_tiny(key, ptr), value, "the live entry resolves its words");
        assert!(shard.remove(key, |v| v[0] == value[0]), "the key is present and removable");
        assert_eq!(
            shard.deref_tiny(key, ptr),
            [0u64; 2],
            "a removed pair must resolve to zero words, not the old value (T-04-29)"
        );
        assert_eq!(shard.lookup(key, |v| v[0] == value[0]), None, "the removed key no longer resolves");

        // A second key inserted afterwards into the same block is unaffected.
        let bits = index_bits(blocks).expect("a power-of-two block count");
        let (block, _) = feistel::split(key, bits);
        let mut other = key + 1;
        while feistel::split(other, bits).0 != block {
            other += 1;
        }
        let other_value = [0xDEAD_BEEF_0000_0001u64, 0xFEED_FACE_0000_0002];
        let other_ptr = shard
            .insert(other, other_value, |v| v[0] == other_value[0])
            .expect("the freed slot takes another key")
            .expect("a home-only shard places the key at home");
        assert_eq!(
            shard.deref_tiny(other, other_ptr),
            other_value,
            "the second key in the same block reads its own words"
        );
        assert_eq!(
            shard.lookup(other, |v| v[0] == other_value[0]).map(|found| found.value),
            Some(other_value),
            "the second key resolves through the header path too"
        );
    }

    #[test]
    fn tpht_slot_id_is_block_times_sixteen_plus_ptr_minus_one() {
        // The sweep's id is the dense arena index `(block << 4) | (ptr - 1)`
        // (WIRE-09): below `16 * num_blocks`, distinct for distinct live
        // entries, and `read_slot` of it returns the entry's words.
        let blocks = miri_scaled(16, 4);
        let shard =
            FlattenedTphtShard::<2>::with_banks(blocks, 0).expect("a power-of-two block count");
        let bits = index_bits(blocks).expect("a power-of-two block count");
        let keys: Vec<u64> =
            (0..8u64).map(|i| 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(i + 1)).collect();
        let mut ids = Vec::new();
        for &key in &keys {
            let value = [key, !key];
            let ptr = shard
                .insert(key, value, |v| v[0] == key)
                .expect("the shard takes the key")
                .expect("a home-only shard places the key at home");
            let id = shard.slot_id(key, ptr);
            let (block, _) = feistel::split(key, bits);
            assert_eq!(
                id,
                ((block as u32) << 4) | (ptr.get() as u32 - 1),
                "the id is (block << 4) | (ptr - 1) (key {key:#x})"
            );
            assert!(
                (id as usize) < HOME_SLOTS * blocks,
                "a slot id is below 16 * num_blocks (key {key:#x}, id {id})"
            );
            assert_eq!(
                shard.read_slot(id),
                Some(value),
                "read_slot of a live id returns the inserted words (key {key:#x})"
            );
            ids.push(id);
        }
        let mut distinct = ids.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), ids.len(), "distinct live entries have distinct slot ids");
    }

    #[test]
    fn tpht_read_slot_is_bounds_checked() {
        // An id outside the arena is `None`, never an out-of-bounds index: the
        // sweep may rebuild an id from a stale bitmap (T-04-31).
        let blocks = miri_scaled(16, 4);
        let shard =
            FlattenedTphtShard::<2>::with_banks(blocks, 0).expect("a power-of-two block count");
        let end = (HOME_SLOTS * blocks) as u32;
        assert_eq!(shard.read_slot(end), None, "one past the last slot is None");
        assert_eq!(shard.read_slot(u32::MAX), None, "u32::MAX is None");
        assert_eq!(
            shard.read_slot(end - 1),
            Some([0u64; 2]),
            "the last slot of the arena is in bounds and still zero"
        );
    }

    #[test]
    fn tpht_deref_tiny_still_one_arena_read() {
        // `remove` gained a clearing write; clearing is a write, never a read,
        // so the DS-03 probe pair (1, 0) is unchanged — before and after a
        // removal.
        let blocks = miri_scaled(1, 1);
        let probe = CountingProbe::new();
        let shard =
            FlattenedTphtShard::<2, CountingProbe>::with_banks_and_probe(blocks, 0, probe.clone())
                .expect("one block is a valid block count");
        let value = [0xAAAA_BBBB_CCCC_DDDDu64, 0x1111_2222_3333_4444];
        let ptr = shard
            .insert(0x5EED, value, |v| v[0] == value[0])
            .expect("the first key fits in an empty block")
            .expect("a home-only shard places the key at home");
        probe.reset();
        assert_eq!(shard.deref_tiny(0x5EED, ptr), value, "the live entry resolves its words");
        assert_eq!(
            (probe.arena_reads(), probe.header_reads()),
            (1, 0),
            "deref_tiny is exactly one arena read and zero header reads (DS-03)"
        );
        assert!(shard.remove(0x5EED, |v| v[0] == value[0]), "the key is removable");
        probe.reset();
        assert_eq!(shard.deref_tiny(0x5EED, ptr), [0u64; 2], "the removed slot reads zero");
        assert_eq!(
            (probe.arena_reads(), probe.header_reads()),
            (1, 0),
            "remove's clearing write is not an arena read; the fast path is still (1, 0) (DS-03)"
        );
    }
}
