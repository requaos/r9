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

use super::{HOME_SLOTS, NoProbe, Probe, TinyPtr, TphtError, feistel, index_bits};

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
/// `((block << 4) + (tiny_ptr - 1)) & slot_mask`.
pub struct FlattenedTphtShard<const WORDS: usize, P: Probe = NoProbe> {
    headers: Box<[TphtBucketHeader]>,
    arena: Box<[[AtomicU64; WORDS]]>,
    probe: P,
    bits: u32,
    slot_mask: usize,
}

impl<const WORDS: usize> FlattenedTphtShard<WORDS, NoProbe> {
    /// Build a shard with the zero-sized [`NoProbe`].
    pub fn new(num_blocks: usize) -> Result<Self, TphtError> {
        Self::with_probe(num_blocks, NoProbe)
    }
}

impl<const WORDS: usize, P: Probe> FlattenedTphtShard<WORDS, P> {
    /// Build a shard with an injected [`Probe`] (a test seam for DS-03).
    pub fn with_probe(num_blocks: usize, probe: P) -> Result<Self, TphtError> {
        let bits = index_bits(num_blocks)?;
        let slots = HOME_SLOTS.checked_mul(num_blocks).ok_or(TphtError::BadBlockCount)?;
        let headers =
            (0..num_blocks).map(|_| TphtBucketHeader::new()).collect::<Vec<_>>().into_boxed_slice();
        let arena = (0..slots)
            .map(|_| core::array::from_fn(|_| AtomicU64::new(0)))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self { headers, arena, probe, bits, slot_mask: slots - 1 })
    }

    /// Insert `value` under `key`; `same` is the full-key verification called
    /// on a fingerprint match.  Returns the entry's tiny pointer, or
    /// [`TphtError::Exists`] / [`TphtError::HomeFull`].
    pub fn insert(
        &self,
        key: u64,
        value: [u64; WORDS],
        same: impl Fn(&[u64; WORDS]) -> bool,
    ) -> Result<TinyPtr, TphtError> {
        let (block, remainder) = feistel::split(key, self.bits);
        let fingerprint = feistel::fingerprint(remainder);
        let header = &self.headers[block];
        let generation = Self::lock(header);
        let occupied = header.occupied_mask.load(Ordering::Relaxed);
        let mut free = None;
        let mut outcome = Err(TphtError::HomeFull);
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
                outcome = Err(TphtError::Exists);
                free = None;
                break;
            }
        }
        if let Some(slot) = free {
            let ptr = TinyPtr::new((slot + 1) as u8).expect("a home slot index is 1..=16");
            self.write_value(self.slot_index(block, ptr), &value);
            header.quotients[slot].store(fingerprint, Ordering::Release);
            header.tiny_ptrs[slot].store(ptr.get(), Ordering::Release);
            Self::set_spilled(header, slot, ptr);
            header.occupied_mask.store(occupied | (1u16 << slot), Ordering::Release);
            outcome = Ok(ptr);
        }
        Self::unlock(header, generation);
        outcome
    }

    /// Look `key` up in the home block; `same` is the full-key verification
    /// called on a fingerprint match.
    pub fn lookup(
        &self,
        key: u64,
        same: impl Fn(&[u64; WORDS]) -> bool,
    ) -> Option<([u64; WORDS], TinyPtr)> {
        let (block, remainder) = feistel::split(key, self.bits);
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
                    return Some((value, ptr));
                }
            }
            return None;
        }
    }

    /// Remove `key` from the home block; `same` is the full-key verification
    /// called on a fingerprint match.  Returns whether an entry was removed.
    pub fn remove(&self, key: u64, same: impl Fn(&[u64; WORDS]) -> bool) -> bool {
        let (block, remainder) = feistel::split(key, self.bits);
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
        let (block, _remainder) = feistel::split(key, self.bits);
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
    use crate::tpht::miri_scaled;

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
            let shard = FlattenedTphtShard::<2>::new(blocks).expect("a power-of-two block count");
            let mut model: BTreeMap<u64, [u64; 2]> = BTreeMap::new();
            for &key in &keys {
                assert_eq!(
                    shard.lookup(key, |v| v[0] == key),
                    None,
                    "the empty shard must not resolve key 0x{key:016x}"
                );
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
                                shard.lookup(key, |v| v[0] == key).map(|(v, _)| v),
                                model.get(&key).copied(),
                                "a refused insert must leave the first value (case {case}, step {step})"
                            );
                        } else {
                            let ptr = result.expect("a table holding eight keys has room");
                            assert_eq!(
                                shard.lookup(key, |v| v[0] == key),
                                Some((value, ptr)),
                                "an inserted key must resolve to its value and pointer (case {case}, step {step})"
                            );
                            model.insert(key, value);
                        }
                    }
                    1 => {
                        assert_eq!(
                            shard.lookup(key, |v| v[0] == key).map(|(v, _)| v),
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
                            shard.lookup(key, |v| v[0] == key).map(|(v, _)| v),
                            model.get(&key).copied(),
                            "a removed key must not resolve (case {case}, step {step})"
                        );
                    }
                }
            }
            for &key in &keys {
                assert_eq!(
                    shard.lookup(key, |v| v[0] == key).map(|(v, _)| v),
                    model.get(&key).copied(),
                    "the final state disagrees with the model (case {case}, key {key})"
                );
            }
        }

        // A full header: the seventeenth key for one block is refused.
        let shard = FlattenedTphtShard::<2>::new(1).expect("one block");
        for key in 0..HOME_SLOTS as u64 {
            shard.insert(key, [key, !key], |v| v[0] == key).expect("the first sixteen keys fit");
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
        let shard = FlattenedTphtShard::<2>::new(1).expect("one block");
        let ptr_a = shard.insert(a, [a, !a], |v| v[0] == a).expect("the first key stores");
        let ptr_b = shard
            .insert(b, [b, !b], |v| v[0] == b)
            .expect("the second key stores despite the fingerprint collision");
        assert_eq!(shard.lookup(a, |v| v[0] == a), Some(([a, !a], ptr_a)));
        assert_eq!(shard.lookup(b, |v| v[0] == b), Some(([b, !b], ptr_b)));
        assert_eq!(
            shard.lookup(a, |v| v[0] == b),
            Some(([b, !b], ptr_b)),
            "the fingerprint cannot tell the pair apart; `same` does"
        );
        assert_ne!(ptr_a, ptr_b, "the two entries hold different slots");
    }

    #[test]
    fn stress_readers_never_see_torn_entry() {
        let shard = Arc::new(FlattenedTphtShard::<2>::new(1).expect("one block"));
        let keys: [u64; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        // Key 0 stays present for the whole run, so every reader has a
        // guaranteed hit whose value must never be torn.
        shard
            .insert(keys[0], [keys[0], !keys[0]], |v| v[0] == keys[0])
            .expect("the sentinel stores");
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
                    if let Some((value, _ptr)) = shard.lookup(key, |v| v[0] == key) {
                        assert_eq!(value[0], key, "a reader got another key's value");
                        assert_eq!(value[1], !key, "a reader got a torn value for key {key}");
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
        let blocks = miri_scaled(1, 1);
        let probe = CountingProbe::new();
        let shard = FlattenedTphtShard::<2, CountingProbe>::with_probe(blocks, probe.clone())
            .expect("one block is a valid block count");
        let ptr = shard
            .insert(0x5EED, [0xAAAA_BBBB_CCCC_DDDD, 0x1111_2222_3333_4444], |v| {
                v[0] == 0xAAAA_BBBB_CCCC_DDDD
            })
            .expect("the first key fits in an empty block");
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
            Some(([0xAAAA_BBBB_CCCC_DDDD, 0x1111_2222_3333_4444], ptr)),
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
        let shard = FlattenedTphtShard::<2>::new(blocks).expect("a power-of-two block count");
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
                        let ptr = result
                            .expect("32 keys over at least 16 blocks leave a home slot each");
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
                shard.lookup(key, |v| v[0] == key).map(|(v, _)| v),
                Some(value),
                "the key path agrees with the model (key {key})"
            );
        }

        // A stale pair is not undefined behaviour: it returns whatever the
        // slot now holds.  A one-block shard reuses slot 0 after a removal.
        let shard = FlattenedTphtShard::<2>::new(1).expect("one block");
        let stale = shard.insert(7, [7, !7], |v| v[0] == 7).expect("the first key fits");
        assert!(shard.remove(7, |v| v[0] == 7), "the first key is present");
        let fresh = shard.insert(8, [8, !8], |v| v[0] == 8).expect("the slot is free again");
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
        let shard = FlattenedTphtShard::<2>::new(blocks).expect("two blocks");
        let mut stored = vec![[0u64; 2]; HOME_SLOTS * blocks];
        for &key in first.iter().chain(last.iter()) {
            let (block, _) = feistel::split(key, bits);
            let ptr = shard
                .insert(key, [key, !key], |v| v[0] == key)
                .expect("sixteen keys per block fit in its home window");
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
}
