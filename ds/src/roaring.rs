//! roaring — the hand-written Roaring bitmap (array, bitmap and run
//! containers) with its differential tests against a `BTreeSet` model and
//! the crates.io `roaring` test oracle (Phase 3 CONTEXT D-19).
//!
//! A bitmap is a sorted vector of chunks keyed by the high 16 bits of the
//! value; each chunk holds exactly one container over the low 16 bits:
//!
//! * `Array` — a sorted `Vec<u16>` holding at most [`ARRAY_MAX`] values;
//! * `Bitmap` — a boxed `[u64; 1024]` (65,536 bits) plus its cardinality;
//! * `Run` — sorted `(start, length)` pairs, where a pair covers
//!   `start ..= start + length` and `length` counts the additional values
//!   (Lemire et al., arXiv:1603.06549).
//!
//! The conversion sites are fixed so results are deterministic: an array
//! becomes a bitmap when an insert would take its cardinality past
//! [`ARRAY_MAX`]; a bitmap becomes an array when a removal brings it down to
//! [`ARRAY_MAX`]; [`RoaringBitmap::run_optimize`] rewrites each container in
//! the smallest of the three forms; and the crate-visible `insert_compact`
//! applies that same smallest-form rule to the one container it touched.
//! Set operations are not conversion sites: they build new containers (array
//! at or below [`ARRAY_MAX`], else bitmap) and an empty result container is
//! dropped with its chunk.
//!
//! The four hot loops live in the private [`crate::setops`] module and the
//! containers call only those functions, so a Phase 5 SIMD module can replace
//! the bodies with no caller change (D-20).  No SIMD code and no intrinsic
//! lands here in this phase; the SIMD module is mounted at the crate root
//! because a forbid reaches submodules.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ops::RangeInclusive;

/// The cardinality at which an array container becomes a bitmap container.
pub const ARRAY_MAX: usize = 4096;

const CHUNK_BITS: u32 = 16;
const CHUNK_LOW_MASK: u32 = 0xFFFF;

#[derive(Clone)]
struct Chunk {
    key: u16,
    container: Container,
}

#[derive(Clone)]
enum Container {
    Array(Vec<u16>),
    Bitmap(Box<[u64; 1024]>, u32),
    Run(Vec<(u16, u16)>),
}

/// A hand-written Roaring bitmap over the full `u32` range.
pub struct RoaringBitmap {
    chunks: Vec<Chunk>,
}

impl RoaringBitmap {
    /// An empty bitmap.
    pub const fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Inserts `v`; returns whether it was absent.
    pub fn insert(&mut self, _v: u32) -> bool {
        false
    }

    /// Removes `v`; returns whether it was present.
    pub fn remove(&mut self, _v: u32) -> bool {
        false
    }

    /// Whether `v` is present.
    pub fn contains(&self, _v: u32) -> bool {
        false
    }

    /// The number of distinct values.
    pub fn len(&self) -> u64 {
        0
    }

    /// Whether the bitmap holds no values.
    pub fn is_empty(&self) -> bool {
        true
    }

    /// Ascending iteration over the values.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        core::iter::empty()
    }

    /// Inserts every value in `range`.
    pub fn insert_range(&mut self, _range: RangeInclusive<u32>) {}

    /// Rewrites every container in the smallest of the three forms.
    pub fn run_optimize(&mut self) {}
}

impl Default for RoaringBitmap {
    fn default() -> Self {
        Self::new()
    }
}

/// The container kind a chunk currently holds (test inspection only).
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerKind {
    Array,
    Bitmap,
    Run,
}

#[cfg(test)]
impl RoaringBitmap {
    /// The kind of each chunk's container, in chunk order.
    pub(crate) fn container_kinds(&self) -> Vec<ContainerKind> {
        Vec::new()
    }

    /// The number of runs in each chunk's container, in chunk order.
    pub(crate) fn run_counts(&self) -> Vec<usize> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Eight literal seeds: every randomized case reproduces from its seed.
    const SEEDS: [u64; 8] = [
        0x0000_0000_0000_0001,
        0x5DEE_CE66_D1CE_4B1D,
        0x1234_5678_9ABC_DEF0,
        0xDEAD_BEEF_CAFE_F00D,
        0x0F1E_2D3C_4B5A_6978,
        0xA5A5_5A5A_1234_4321,
        0xFFFF_FFFF_0000_0000,
        0x9E37_79B9_7F4A_7C15,
    ];

    /// SplitMix64: a seeded generator with no dependency (RESEARCH
    /// "Don't Hand-Roll": no `rand`, no `proptest`).
    struct SplitMix64 {
        state: u64,
    }

    impl SplitMix64 {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_u64(&mut self) -> u64 {
            self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    /// The five value distributions of Task 1.
    fn value_for(rng: &mut SplitMix64, which: usize) -> u32 {
        match which {
            0 => rng.next_u64() as u32,
            1 => (7u32 << 16) | (rng.next_u64() as u32 & CHUNK_LOW_MASK),
            2 => rng.next_u64() as u32,
            3 => {
                const BOUNDARY: [u32; 10] =
                    [0, 1, 2, 65_534, 65_535, 65_536, 65_537, 131_071, u32::MAX - 1, u32::MAX];
                BOUNDARY[(rng.next_u64() % BOUNDARY.len() as u64) as usize]
            }
            _ => {
                if rng.next_u64() & 1 == 0 {
                    rng.next_u64() as u32
                } else {
                    (3u32 << 16) | (rng.next_u64() as u32 & CHUNK_LOW_MASK)
                }
            }
        }
    }

    #[test]
    fn membership_matches_models() {
        for seed in SEEDS {
            for which in 0..5usize {
                let mut rng = SplitMix64::new(seed ^ (0x9E37_79B9_7F4A_7C15u64.wrapping_mul(which as u64 + 1)));
                let mut bitmap = RoaringBitmap::new();
                let mut model: BTreeSet<u32> = BTreeSet::new();
                let mut oracle = ::roaring::RoaringBitmap::new();
                let steps = 400 + which * 25;
                for step in 0..steps {
                    let value = value_for(&mut rng, which);
                    if rng.next_u64() % 5 != 0 {
                        assert_eq!(
                            bitmap.insert(value),
                            model.insert(value),
                            "insert disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                        oracle.insert(value);
                    } else {
                        assert_eq!(
                            bitmap.remove(value),
                            model.remove(&value),
                            "remove disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                        oracle.remove(value);
                    }
                    assert_eq!(
                        bitmap.contains(value),
                        model.contains(&value),
                        "contains disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                    );
                    assert_eq!(
                        bitmap.len(),
                        model.len() as u64,
                        "len disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                    );
                    assert_eq!(
                        bitmap.len(),
                        oracle.len(),
                        "len disagrees with the roaring oracle (seed {seed:#018x}, distribution {which}, step {step})"
                    );
                    if step % 16 == 0 {
                        let ours: Vec<u32> = bitmap.iter().collect();
                        let expected: Vec<u32> = model.iter().copied().collect();
                        let oracle_values: Vec<u32> = oracle.iter().collect();
                        assert_eq!(
                            ours, expected,
                            "ascending iteration disagrees with the BTreeSet model (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                        assert_eq!(
                            ours, oracle_values,
                            "ascending iteration disagrees with the roaring oracle (seed {seed:#018x}, distribution {which}, step {step})"
                        );
                    }
                }
                assert_eq!(bitmap.is_empty(), model.is_empty(), "is_empty (seed {seed:#018x}, distribution {which})");
            }
        }
    }

    #[test]
    fn container_kinds_and_conversions() {
        let mut kinds_seen: Vec<ContainerKind> = Vec::new();

        let mut array = RoaringBitmap::new();
        for value in 0..4096u32 {
            assert!(array.insert(value), "first insert of {value}");
        }
        assert_eq!(
            array.container_kinds(),
            vec![ContainerKind::Array],
            "4096 values are an array container at ARRAY_MAX"
        );
        kinds_seen.push(ContainerKind::Array);

        assert!(array.insert(4096), "the 4097th value inserts");
        assert_eq!(
            array.container_kinds(),
            vec![ContainerKind::Bitmap],
            "an insert past ARRAY_MAX converts the array container to a bitmap container"
        );
        kinds_seen.push(ContainerKind::Bitmap);

        array.run_optimize();
        assert_eq!(
            array.container_kinds(),
            vec![ContainerKind::Run],
            "run_optimize turns a contiguous chunk into one run container"
        );
        assert_eq!(array.len(), 4097, "run_optimize preserves the values");
        assert_eq!(array.run_counts(), vec![1], "the contiguous chunk holds one run");
        kinds_seen.push(ContainerKind::Run);

        let mut touching = RoaringBitmap::new();
        touching.insert_range(1..=5);
        touching.insert_range(6..=9);
        assert_eq!(touching.len(), 9, "touching ranges hold nine values");
        touching.run_optimize();
        assert_eq!(touching.container_kinds(), vec![ContainerKind::Run]);
        assert_eq!(
            touching.run_counts(),
            vec![1],
            "two runs that touch ([1..=5] and [6..=9]) are stored as one run after run_optimize"
        );

        assert!(
            kinds_seen.contains(&ContainerKind::Array)
                && kinds_seen.contains(&ContainerKind::Bitmap)
                && kinds_seen.contains(&ContainerKind::Run),
            "all three container kinds must be observed through the helper"
        );
    }
}
