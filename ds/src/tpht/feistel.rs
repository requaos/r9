#![forbid(unsafe_code)]
//! feistel — one-round Feistel quotienting for TPHT (DS-02; IG-M05).
//!
//! A 64-bit key is split into a home block index and a remainder, and the
//! pair reconstructs the key losslessly.  The construction is bijective, so
//! DS-02 is a property of these functions alone and does not depend on any
//! table state.
//!
//! **Correction to compendium section 4.1 (RESEARCH C4).** The compendium
//! takes the home block index from the low `log2(num_blocks)` bits of
//! `P = (L' << 32) | R`.  Those are the key's own low bits with one Feistel
//! round: keys that differ only in the upper half (`(id << 32)` page keys,
//! for example) would all land in one block and the over-90% home-hit target
//! of DS-01 would be unreachable for structured keys.  This module takes the
//! index from the low bits of the mixed half `L'` instead and keeps every
//! remaining bit in the remainder.  The permutation stays bijective either
//! way.  This choice is outside the IG-D09 lock; plan 03-08 produces the
//! evidence for both variants and plan 03-09 records it with the other
//! planning-level corrections.
//!
//! `f(R)` is completed as MurmurHash3's 32-bit finalizer (Claude's
//! discretion; RESEARCH Open Question 2) and pinned by a known-answer test
//! whose values were computed independently of this code.

/// MurmurHash3's 32-bit finalizer: the mixing function `f(R)`.
pub fn mix32(r: u32) -> u32 {
    let _ = r;
    0
}

/// The one-round Feistel permutation of a key: `P = (L' << 32) | R` with
/// `L' = L ^ f(R)`.
pub fn permute(key: u64) -> u64 {
    let _ = key;
    0
}

/// The inverse of [`permute`].
pub fn unpermute(p: u64) -> u64 {
    let _ = p;
    0
}

/// Split a key into its home block index (the low `index_bits` bits of the
/// mixed half `L'`) and the remainder (every other bit, `64 - index_bits`
/// wide).  Nothing is discarded.
pub fn split(key: u64, index_bits: u32) -> (usize, u64) {
    let _ = (key, index_bits);
    (0, 0)
}

/// Rebuild a key from a home block index and a remainder produced by
/// [`split`].
pub fn join(index: usize, remainder: u64, index_bits: u32) -> u64 {
    let _ = (index, remainder, index_bits);
    0
}

/// A 16-bit fingerprint of the remainder: what the header's `quotients`
/// field stores.  It is a fingerprint and cannot rebuild the key; a matching
/// candidate is verified against the full key by the caller's closure
/// (RESEARCH Open Question 3).
pub fn fingerprint(remainder: u64) -> u16 {
    let _ = remainder;
    0
}

/// The compendium's wording, for contrast only: take the home block index
/// from the low bits of `P` and the remainder from the upper bits.  It exists
/// so the tests (and plan 03-08's evidence) can show why keys that differ
/// only in their upper half collapse into one block with that reading.
#[cfg(test)]
pub(crate) fn split_unmixed(key: u64, index_bits: u32) -> (usize, u64) {
    let _ = (key, index_bits);
    (0, key)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::tpht::miri_scaled;

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

    /// The four expected values were computed independently of this code with
    /// integer arithmetic masked to 32 bits; the command and output are
    /// recorded in `evidence/PINS-03-06.md` under `KAT mix32`.
    #[test]
    fn mix32_known_answers() {
        assert_eq!(mix32(0x0000_0000), 0x0000_0000, "mix32(0) must match the independent KAT");
        assert_eq!(mix32(0x0000_0001), 0x514e_28b7, "mix32(1) must match the independent KAT");
        assert_eq!(
            mix32(0xdead_beef),
            0x0de5_c6a9,
            "mix32(0xdead_beef) must match the independent KAT"
        );
        assert_eq!(
            mix32(0xffff_ffff),
            0x81f1_6f39,
            "mix32(0xffff_ffff) must match the independent KAT"
        );
    }

    #[test]
    fn permute_round_trips() {
        let count = miri_scaled(4096, 256);
        let mut rng = SplitMix64::new(0x0000_0000_0000_00a1);
        for _ in 0..count {
            let key = rng.next();
            let p = permute(key);
            assert_eq!(
                unpermute(p),
                key,
                "unpermute(permute(0x{key:016x})) must return the key"
            );
        }
        for key in [0u64, 1, u32::MAX as u64, 1 << 32, (1 << 32) - 1, u64::MAX] {
            assert_eq!(
                unpermute(permute(key)),
                key,
                "the boundary key 0x{key:016x} must round-trip"
            );
        }
    }

    #[test]
    fn split_join_round_trips() {
        let widths = [0u32, 1, 4, 14, 32];
        let boundary = [0u64, 1, u32::MAX as u64, 1 << 32, (1 << 32) - 1, u64::MAX];
        let count = miri_scaled(4096, 256);
        let mut rng = SplitMix64::new(0x0000_0000_0000_00b2);
        let mut keys: Vec<u64> = boundary.to_vec();
        keys.extend((0..count as u64).map(|_| rng.next()));
        keys.extend(0..count as u64);
        for &bits in &widths {
            for &key in &keys {
                let (index, remainder) = split(key, bits);
                assert!(
                    (index as u64) < (1u64 << bits),
                    "the home index {index} must fit in {bits} bits"
                );
                assert_eq!(
                    join(index, remainder, bits),
                    key,
                    "join(split(0x{key:016x}, {bits})) must return the key"
                );
            }
        }
    }

    #[test]
    fn split_is_injective_on_structured_keys() {
        let count = miri_scaled(65_536, 256);
        for &bits in &[0u32, 1, 4, 14, 32] {
            let mut sequential = BTreeSet::new();
            for i in 0..count as u64 {
                assert!(
                    sequential.insert(split(i, bits)),
                    "sequential key {i} collides at {bits} index bits"
                );
            }
            let mut high_half = BTreeSet::new();
            for i in 0..count as u64 {
                assert!(
                    high_half.insert(split(i << 32, bits)),
                    "high-half key {i} << 32 collides at {bits} index bits"
                );
            }
        }
    }

    #[test]
    fn index_comes_from_the_mixed_half() {
        let count = miri_scaled(4096, 256);
        let required = miri_scaled(2000, 125);
        let mut mixed = BTreeSet::new();
        let mut unmixed = BTreeSet::new();
        for i in 0..count as u64 {
            let key = i << 32;
            let (index, _) = split(key, 12);
            mixed.insert(index);
            let (index, _) = split_unmixed(key, 12);
            unmixed.insert(index);
        }
        assert!(
            mixed.len() >= required,
            "only {} distinct home indices for {} keys of the form i << 32; the index must come \
             from the mixed half (RESEARCH C4)",
            mixed.len(),
            count
        );
        assert_eq!(
            unmixed.len(),
            1,
            "the compendium's low-bits-of-P index collapses i << 32 keys into one block"
        );
        assert!(unmixed.contains(&0), "every i << 32 key lands on index 0 when the index is P's low bits");
    }
}
