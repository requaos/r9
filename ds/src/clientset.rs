//! clientset — the `ClientSet` integer-set algebra built on the Roaring
//! bitmap (DS-06).
//!
//! A reader cohort is almost always small, so `ClientSet` carries its first
//! values inline and upgrades to [`crate::roaring::RoaringBitmap`] only when
//! it outgrows them.  The canonical forms, fixed here, are: `Empty` for zero
//! ids, `Single` for one, `Inline` for two to four (`ids[..count]` sorted
//! ascending, unused entries zero) and `Roaring` once a fifth distinct id is
//! inserted.  A `Roaring` value does not downgrade when ids are removed: a
//! lease cohort that grew large is expected to stay large, and the inline
//! forms were sized for the common case, not for shrinking.
//!
//! The no-heap property for up to four clients is shown by the variants and
//! by `core::mem::size_of::<ClientSet>() <= 24`: `Empty`, `Single` and
//! `Inline` hold no pointer, so no allocation can happen in them.  A
//! counting-allocator proof is deliberately not used; that would need a third
//! module holding the restricted feature outside `ds/src/tpht/` (D-09).
//!
//! Every `Roaring` insert goes through `RoaringBitmap::insert_compact`, which
//! applies the smallest-form rewrite to the one container it touched (the
//! same rule `run_optimize` applies to every container).  Contiguous ids
//! therefore collapse into one run container, which matters for lease
//! cohorts built from a range.  The cost is one pass over one container per
//! insert (at most 8 KiB scanned); the lease path reads the cohort far more
//! often than it changes it (T-03-21 accepts the trade).

use alloc::boxed::Box;

use crate::roaring::RoaringBitmap;

/// A small reader cohort that upgrades to a Roaring bitmap beyond four ids.
#[derive(Clone)]
pub enum ClientSet {
    Empty,
    Single(u32),
    Inline { count: u8, ids: [u32; 4] },
    Roaring(Box<RoaringBitmap>),
}

impl ClientSet {
    /// An empty set.
    pub const fn new() -> Self {
        Self::Empty
    }

    /// Inserts `id`; returns whether it was absent.
    pub fn insert(&mut self, _id: u32) -> bool {
        false
    }

    /// Removes `id`; returns whether it was present.
    pub fn remove(&mut self, _id: u32) -> bool {
        false
    }

    /// Whether `id` is present.
    pub fn contains(&self, _id: u32) -> bool {
        false
    }

    /// The number of distinct ids.
    pub fn len(&self) -> u64 {
        0
    }

    /// Whether the set holds no ids.
    pub fn is_empty(&self) -> bool {
        true
    }

    /// Ascending iteration over the ids.
    pub fn iter(&self) -> Iter<'_> {
        Iter { kind: IterKind::Empty }
    }
}

impl Default for ClientSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Ascending iterator over a [`ClientSet`].
pub struct Iter<'a> {
    kind: IterKind<'a>,
}

enum IterKind<'a> {
    Empty,
    Single(core::iter::Once<u32>),
    Inline(core::slice::Iter<'a, u32>),
    Roaring(crate::roaring::Iter<'a>),
}

impl Iterator for Iter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        match &mut self.kind {
            IterKind::Empty => None,
            IterKind::Single(values) => values.next(),
            IterKind::Inline(values) => values.next().copied(),
            IterKind::Roaring(values) => values.next(),
        }
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

    fn inline_ids(set: &ClientSet) -> [u32; 4] {
        match set {
            ClientSet::Inline { ids, .. } => *ids,
            _ => panic!("expected the Inline variant"),
        }
    }

    #[test]
    fn variant_progression() {
        let mut set = ClientSet::new();
        assert!(matches!(set, ClientSet::Empty), "a new set is Empty");
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());

        assert!(set.insert(10), "the first id inserts");
        assert!(matches!(set, ClientSet::Single(10)), "one id gives Single");
        assert!(!set.insert(10), "a duplicate insert returns false");
        assert!(matches!(set, ClientSet::Single(10)), "a duplicate insert changes nothing");

        assert!(set.insert(20), "the second distinct id inserts");
        assert!(matches!(set, ClientSet::Inline { count: 2, .. }), "two ids give Inline");
        assert_eq!(inline_ids(&set), [10, 20, 0, 0], "Inline keeps its ids sorted and zero-fills the rest");

        assert!(!set.insert(10), "a duplicate insert at count 2 returns false");
        assert_eq!(set.len(), 2, "a duplicate insert at count 2 changes nothing");

        assert!(set.insert(30), "the third distinct id inserts");
        assert!(matches!(set, ClientSet::Inline { count: 3, .. }), "three ids stay Inline");
        assert!(set.insert(40), "the fourth distinct id inserts");
        assert!(matches!(set, ClientSet::Inline { count: 4, .. }), "four distinct ids give Inline");
        assert_eq!(inline_ids(&set), [10, 20, 30, 40]);

        assert!(!set.insert(20), "a duplicate insert at count 4 returns false");
        assert!(matches!(set, ClientSet::Inline { count: 4, .. }), "a duplicate at four ids stays Inline");

        assert!(set.insert(50), "the fifth distinct id inserts");
        match &set {
            ClientSet::Roaring(bitmap) => {
                assert_eq!(bitmap.len(), 5, "Roaring holds all five ids");
                for id in [10, 20, 30, 40, 50] {
                    assert!(bitmap.contains(id), "Roaring holds {id}");
                }
            }
            _ => panic!("the fifth distinct id must upgrade the set to Roaring"),
        }
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn no_heap_up_to_four() {
        assert!(
            core::mem::size_of::<ClientSet>() <= 24,
            "ClientSet must stay at or below the RFC's 24 inline bytes, got {}",
            core::mem::size_of::<ClientSet>()
        );

        let mut set = ClientSet::new();
        for (inserted, id) in [7u32, 8, 9, 10].into_iter().enumerate() {
            assert!(matches!(
                set,
                ClientSet::Empty | ClientSet::Single(_) | ClientSet::Inline { .. }
            ));
            assert!(!matches!(set, ClientSet::Roaring(_)));
            assert!(set.insert(id), "insert {id}");
            assert_eq!(set.len(), inserted as u64 + 1);
            assert!(matches!(
                set,
                ClientSet::Empty | ClientSet::Single(_) | ClientSet::Inline { .. }
            ));
            assert!(!matches!(set, ClientSet::Roaring(_)), "no pointer up to four ids");
        }
        assert!(matches!(set, ClientSet::Inline { count: 4, .. }));
        assert!(set.insert(11), "the fifth id inserts");
        assert!(matches!(set, ClientSet::Roaring(_)), "the fifth distinct id is the first heap form");
    }

    #[test]
    fn removal_rules() {
        let mut set = ClientSet::new();
        assert!(set.insert(1));
        assert!(set.insert(2));
        assert!(matches!(set, ClientSet::Inline { count: 2, .. }));
        assert!(set.remove(1), "removing one of two inline ids succeeds");
        assert!(matches!(set, ClientSet::Single(2)), "Inline with two ids becomes Single");
        assert!(!set.remove(1), "removing an absent id returns false");
        assert!(set.remove(2), "removing the last single id succeeds");
        assert!(matches!(set, ClientSet::Empty), "Single becomes Empty");
        assert!(!set.remove(2), "removing from Empty returns false");

        let mut roaring = ClientSet::new();
        for id in [10u32, 20, 30, 40, 50] {
            assert!(roaring.insert(id));
        }
        assert!(matches!(roaring, ClientSet::Roaring(_)));
        for id in [10u32, 20, 30, 40, 50] {
            assert!(roaring.remove(id), "removing {id} from Roaring succeeds");
        }
        assert!(
            matches!(roaring, ClientSet::Roaring(_)),
            "Roaring does not downgrade when ids are removed"
        );
        assert!(roaring.is_empty(), "the emptied Roaring set has no ids");
        assert_eq!(roaring.len(), 0);
        assert!(!roaring.remove(10), "removing from an empty Roaring returns false");
    }

    #[test]
    fn contiguous_ids_collapse_to_one_run() {
        let ascending: Vec<u32> = (100..=4096).collect();

        let mut set = ClientSet::new();
        for &id in &ascending {
            assert!(set.insert(id), "ascending insert {id}");
        }
        match &set {
            ClientSet::Roaring(bitmap) => {
                assert_eq!(bitmap.len(), ascending.len() as u64, "all ids are present");
                assert_eq!(
                    bitmap.container_kinds(),
                    vec![crate::roaring::ContainerKind::Run],
                    "the contiguous cohort is one run container"
                );
                assert_eq!(bitmap.run_counts(), vec![1], "the run container holds exactly one run");
            }
            _ => panic!("4096 distinct ids must be a Roaring set"),
        }

        let mut rng = SplitMix64::new(SEEDS[0]);
        let mut shuffled = ascending.clone();
        for index in (1..shuffled.len()).rev() {
            let swap = (rng.next_u64() % (index as u64 + 1)) as usize;
            shuffled.swap(index, swap);
        }
        let mut set = ClientSet::new();
        for &id in &shuffled {
            assert!(set.insert(id), "shuffled insert {id}");
        }
        match &set {
            ClientSet::Roaring(bitmap) => {
                assert_eq!(bitmap.len(), shuffled.len() as u64, "all shuffled ids are present");
                assert_eq!(
                    bitmap.container_kinds(),
                    vec![crate::roaring::ContainerKind::Run],
                    "a shuffled contiguous cohort still collapses to one run container"
                );
                assert_eq!(bitmap.run_counts(), vec![1], "the shuffled cohort holds exactly one run");
            }
            _ => panic!("4096 distinct ids must be a Roaring set"),
        }
    }

    #[test]
    fn matches_btreeset_model() {
        for seed in SEEDS {
            let mut rng = SplitMix64::new(seed);
            let mut set = ClientSet::new();
            let mut model: BTreeSet<u32> = BTreeSet::new();
            for step in 0..300 {
                let id = match rng.next_u64() % 8 {
                    0 => 0,
                    1 => u32::MAX,
                    2 => rng.next_u64() as u32 & 0xFFFF,
                    _ => rng.next_u64() as u32,
                };
                if !rng.next_u64().is_multiple_of(4) {
                    assert_eq!(
                        set.insert(id),
                        model.insert(id),
                        "insert disagrees with the BTreeSet model (seed {seed:#018x}, step {step}, id {id})"
                    );
                } else {
                    assert_eq!(
                        set.remove(id),
                        model.remove(&id),
                        "remove disagrees with the BTreeSet model (seed {seed:#018x}, step {step}, id {id})"
                    );
                }
                assert_eq!(
                    set.contains(id),
                    model.contains(&id),
                    "contains disagrees with the BTreeSet model (seed {seed:#018x}, step {step}, id {id})"
                );
                assert_eq!(
                    set.len(),
                    model.len() as u64,
                    "len disagrees with the BTreeSet model (seed {seed:#018x}, step {step})"
                );
                if step % 16 == 0 {
                    let ours: Vec<u32> = set.iter().collect();
                    let expected: Vec<u32> = model.iter().copied().collect();
                    assert_eq!(
                        ours, expected,
                        "ascending iteration disagrees with the BTreeSet model (seed {seed:#018x}, step {step})"
                    );
                }
            }
        }
    }
}
