//! chained — the two-choice dereference banks (`ChainedTphtOverflow`,
//! DS-04; IG-M07).
//!
//! Each bank is an arena of exactly [`BANK_NODES`] (255) overflow nodes,
//! indexed `0..=254`, so a node index always fits in one byte and `0xFF` is
//! never an index.  A home block maps to two candidate banks; a new node goes
//! to the candidate with more free nodes, and when both are full the insert
//! reports [`TphtError::BanksFull`].
//!
//! **Two-phase publication.**  A fresh node is claimed by a compare-exchange
//! of its state byte from free to [`WRITING`] (`0xFF`); the fingerprint, full
//! key, value words and chain link are stored; then the state byte is stored
//! to its published value with `Release`, and only after that is the node
//! linked at its block's chain head.  A reader that reaches a node whose
//! state is `WRITING` or free treats it as absent, so it never returns a
//! value from a node that is not published.
//!
//! **Serialisation contract.**  Claims of nodes in a shared bank by different
//! blocks are concurrent and are what the `WRITING` protocol covers.
//! Mutations of one block's chain (insert, remove) must be serialised by the
//! caller; in the flattened shard that is the block's seqlock, and a reader
//! that needs a consistent view across a concurrent remove validates against
//! that seqlock generation.  Standalone, concurrent readers of a chain with
//! one inserting writer are safe.
//!
//! Compendium listing 9.4 is not copied (CONTEXT D-03; RESEARCH Pitfall 6):
//! no node is written through a pointer cast from a shared reference, and the
//! listing's 7-bit `unpack_tiny_ptr` is not used.  Every field is an atomic,
//! and this module holds no `unsafe`.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU8, AtomicU16, AtomicU64, AtomicUsize, Ordering};

use super::{TphtError, feistel};

/// The number of overflow nodes in one bank: indices are `0..=254`, so a node
/// index always fits in one byte and `0xFF` is never an index.
pub const BANK_NODES: usize = 255;

/// The two-phase publication sentinel: a claimed node's state byte is
/// `WRITING` until its payload is stored and published.
pub const WRITING: u8 = 0xFF;

/// A free node's state byte.
const FREE: u8 = 0x00;

/// A published node's state byte.
const LIVE: u8 = 0x01;

/// The chain-end marker: no node is named by this link.
const END: u16 = 0xFFFF;

/// One overflow node: a state byte, the 16-bit fingerprint filter, the full
/// 64-bit key (verified after a fingerprint match), the value words and the
/// link to the next node of its chain.
struct Node<const WORDS: usize> {
    state: AtomicU8,
    fingerprint: AtomicU16,
    key: AtomicU64,
    value: [AtomicU64; WORDS],
    next: AtomicU16,
}

impl<const WORDS: usize> Node<WORDS> {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(FREE),
            fingerprint: AtomicU16::new(0),
            key: AtomicU64::new(0),
            value: [const { AtomicU64::new(0) }; WORDS],
            next: AtomicU16::new(END),
        }
    }
}

/// One bank: exactly [`BANK_NODES`] nodes, a counter of nodes in use and a
/// rotating claim cursor so filling a bank does not rescan from zero.
struct Bank<const WORDS: usize> {
    nodes: [Node<WORDS>; BANK_NODES],
    used: AtomicUsize,
    cursor: AtomicUsize,
}

impl<const WORDS: usize> Bank<WORDS> {
    const fn new() -> Self {
        Self {
            nodes: [const { Node::new() }; BANK_NODES],
            used: AtomicUsize::new(0),
            cursor: AtomicUsize::new(0),
        }
    }
}

/// A test-only counter of nodes currently in the [`WRITING`] state, raised at
/// the claim and lowered at publication.  The concurrency pins use it to show
/// that a claim really happened and that no node is left half-published.
#[cfg(test)]
#[derive(Default)]
struct WritingHook {
    writing: AtomicUsize,
    claims: AtomicUsize,
    max_index: AtomicU8,
}

#[cfg(test)]
impl WritingHook {
    fn claimed(&self, index: usize) {
        self.writing.fetch_add(1, Ordering::Relaxed);
        self.claims.fetch_add(1, Ordering::Relaxed);
        self.max_index.fetch_max(index as u8, Ordering::Relaxed);
    }

    fn published(&self) {
        self.writing.fetch_sub(1, Ordering::Relaxed);
    }

    fn writing(&self) -> usize {
        self.writing.load(Ordering::Relaxed)
    }

    fn claims(&self) -> usize {
        self.claims.load(Ordering::Relaxed)
    }

    fn max_index(&self) -> u8 {
        self.max_index.load(Ordering::Relaxed)
    }
}

/// The overflow structure entries spill into: 255-node banks with two-choice
/// placement and the two-phase [`WRITING`] publication.
pub struct ChainedTphtOverflow<const WORDS: usize> {
    /// One chain head per home block, `END` when the chain is empty.
    heads: Box<[AtomicU16]>,
    banks: Box<[Bank<WORDS>]>,
    #[cfg(test)]
    hook: WritingHook,
}

impl<const WORDS: usize> ChainedTphtOverflow<WORDS> {
    /// Build an overflow with `num_blocks` home blocks and `num_banks` banks.
    ///
    /// The block count follows [`super::index_bits`] (a power of two from 1
    /// to `1 << 32`); zero banks is [`TphtError::BadBlockCount`].
    pub fn new(num_blocks: usize, num_banks: usize) -> Result<Self, TphtError> {
        super::index_bits(num_blocks)?;
        if num_banks == 0 {
            return Err(TphtError::BadBlockCount);
        }
        let heads =
            (0..num_blocks).map(|_| AtomicU16::new(END)).collect::<Vec<_>>().into_boxed_slice();
        let banks = (0..num_banks).map(|_| Bank::new()).collect::<Vec<_>>().into_boxed_slice();
        Ok(Self {
            heads,
            banks,
            #[cfg(test)]
            hook: WritingHook::default(),
        })
    }

    /// The two candidate banks for a home block: `mix32(block) % num_banks`
    /// and `mix32(block ^ 0x9E37_79B9) % num_banks`; when they are equal and
    /// there is more than one bank, the second is the next bank.
    pub fn candidate_banks(&self, block: usize) -> (usize, usize) {
        let _ = block;
        (0, 0)
    }

    /// Insert `value` under `key` in `block`'s overflow chain.
    ///
    /// Returns [`TphtError::Exists`] when the full key is already present in
    /// the chain and [`TphtError::BanksFull`] when both candidate banks are
    /// full.  The caller serialises mutations of the block's chain.
    pub fn insert(&self, block: usize, key: u64, value: [u64; WORDS]) -> Result<(), TphtError> {
        let _ = (block, key, value);
        Err(TphtError::BanksFull)
    }

    /// Look `key` up in `block`'s overflow chain: the 16-bit fingerprint is
    /// the first check, the full 64-bit key stored in the node is the second.
    pub fn lookup(&self, block: usize, key: u64) -> Option<[u64; WORDS]> {
        let _ = (block, key);
        None
    }

    /// Remove `key` from `block`'s overflow chain, returning whether an entry
    /// was removed.  The caller serialises mutations of the block's chain.
    pub fn remove(&self, block: usize, key: u64) -> bool {
        let _ = (block, key);
        false
    }

    /// The nodes in use and the capacity: `(sum of the banks' used counters,
    /// num_banks * 255)`.
    pub fn fill(&self) -> (usize, usize) {
        (0, 0)
    }

    /// Claim every free node of one bank for a test, leaving them in
    /// `WRITING`; returns how many were claimed.  This constructs the state
    /// two-choice placement exists to handle: a full first candidate bank.
    #[cfg(test)]
    fn claim_bank_for_test(&self, bank: usize) -> usize {
        let _ = bank;
        0
    }

    /// The raw chain-head link of a test block.
    #[cfg(test)]
    fn head_link_for_test(&self, block: usize) -> u16 {
        let _ = block;
        END
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Barrier};
    use std::thread;

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

    /// Eight literal seeds for the fill pin (the first two are also the
    /// Miri-side run).
    const FILL_SEEDS: [u64; 8] = [
        0x0000_0000_0000_0307,
        0x1111_1111_1111_0307,
        0x2222_2222_2222_0307,
        0x3333_3333_3333_0307,
        0x4444_4444_4444_0307,
        0x5555_5555_5555_0307,
        0x6666_6666_6666_0307,
        0x7777_7777_7777_0307,
    ];

    /// A link is `(bank choice << 8) | node index`, or [`END`]; its index
    /// byte is never `0xFF` because a node index is `0..=254`.
    fn assert_link_is_a_node_index(link: u16) {
        if link != END {
            assert!(
                ((link & 0xFF) as usize) < BANK_NODES,
                "a link's index byte must be a node index, never 0xFF: {link:#06x}"
            );
            assert!(
                (link >> 8) as usize <= 1,
                "a link must name one of the block's two candidate banks: {link:#06x}"
            );
        }
    }

    #[test]
    fn ops_match_btreemap_model() {
        let blocks = miri_scaled(64, 8);
        let banks = miri_scaled(8, 2);
        let ops = miri_scaled(2_000, 200);
        let keys: Vec<u64> =
            (0..32u64).map(|i| 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(i + 1)).collect();
        let overflow =
            ChainedTphtOverflow::<2>::new(blocks, banks).expect("a power-of-two block count");
        let mut model: BTreeMap<(usize, u64), [u64; 2]> = BTreeMap::new();
        let mut rng = SplitMix64::new(0x0000_0000_0000_0307);
        for step in 0..ops {
            let block = (rng.next() as usize) % blocks;
            let key = keys[(rng.next() as usize) % keys.len()];
            let value = [key, !key];
            match rng.next() % 3 {
                0 => {
                    let result = overflow.insert(block, key, value);
                    if model.contains_key(&(block, key)) {
                        assert_eq!(
                            result,
                            Err(TphtError::Exists),
                            "a second insert must report Exists (step {step}, block {block}, key {key})"
                        );
                        assert_eq!(
                            overflow.lookup(block, key),
                            model.get(&(block, key)).copied(),
                            "a refused insert must leave the first value (step {step})"
                        );
                    } else {
                        result.expect("a table holding 32 keys has room");
                        assert_eq!(
                            overflow.lookup(block, key),
                            Some(value),
                            "an inserted key must resolve to its value (step {step})"
                        );
                        model.insert((block, key), value);
                    }
                }
                1 => {
                    assert_eq!(
                        overflow.lookup(block, key),
                        model.get(&(block, key)).copied(),
                        "lookup disagrees with the model (step {step}, block {block}, key {key})"
                    );
                }
                _ => {
                    let removed = overflow.remove(block, key);
                    assert_eq!(
                        removed,
                        model.remove(&(block, key)).is_some(),
                        "remove disagrees with the model (step {step}, block {block}, key {key})"
                    );
                    assert_eq!(
                        overflow.lookup(block, key),
                        None,
                        "a removed key must not resolve (step {step})"
                    );
                }
            }
        }
        assert_eq!(
            overflow.fill().0,
            model.len(),
            "the fill count must equal the live entries in the model"
        );
        for (&(block, key), &value) in &model {
            assert_eq!(
                overflow.lookup(block, key),
                Some(value),
                "the final state disagrees with the model (block {block}, key {key})"
            );
        }
    }

    #[test]
    fn fingerprint_collisions_are_verified_by_full_key() {
        // Two keys whose 16-bit fingerprints collide, found by an independent
        // search over sequential keys; the test re-derives and asserts the
        // collision, so a change to `fingerprint` cannot silently remove it.
        let (a, b) = (37u64, 568u64);
        assert_ne!(a, b, "the pair must be two different full keys");
        assert_eq!(
            feistel::fingerprint(a),
            feistel::fingerprint(b),
            "the pair must share a 16-bit fingerprint"
        );
        let overflow = ChainedTphtOverflow::<2>::new(1, 1).expect("one block and one bank");
        overflow.insert(0, a, [a, !a]).expect("the first key stores");
        overflow
            .insert(0, b, [b, !b])
            .expect("the second key stores despite the fingerprint collision");
        assert_eq!(
            overflow.lookup(0, a),
            Some([a, !a]),
            "the fingerprint collision cannot hide key a"
        );
        assert_eq!(
            overflow.lookup(0, b),
            Some([b, !b]),
            "the fingerprint collision cannot hide key b"
        );
    }

    #[test]
    fn bank_holds_exactly_255_nodes() {
        let overflow = ChainedTphtOverflow::<2>::new(1, 1).expect("one block and one bank");
        for key in 0..BANK_NODES as u64 {
            overflow.insert(0, key, [key, !key]).expect("a bank holds exactly 255 nodes");
        }
        assert_eq!(
            overflow.insert(0, BANK_NODES as u64, [0, 0]),
            Err(TphtError::BanksFull),
            "the 256th node must report BanksFull"
        );
        assert_eq!(overflow.fill(), (BANK_NODES, BANK_NODES), "the bank is full at 255 of 255");
        assert_eq!(
            overflow.lookup(0, BANK_NODES as u64 - 1),
            Some([BANK_NODES as u64 - 1, !(BANK_NODES as u64 - 1)]),
            "the last stored key still resolves"
        );
    }

    #[test]
    fn two_choice_fails_over_to_second_bank() {
        let overflow = ChainedTphtOverflow::<2>::new(2, 2).expect("two blocks and two banks");
        let (first, second) = overflow.candidate_banks(0);
        assert_ne!(first, second, "two banks give a block two distinct candidates");
        assert_eq!(
            overflow.claim_bank_for_test(first),
            BANK_NODES,
            "the test fills the first candidate bank"
        );
        assert_eq!(overflow.fill(), (BANK_NODES, 2 * BANK_NODES), "one bank is fully claimed");
        overflow
            .insert(0, 0x5EED, [0x5EED, !0x5EED])
            .expect("the second candidate bank still has room");
        assert_eq!(overflow.lookup(0, 0x5EED), Some([0x5EED, !0x5EED]));
        assert_eq!(
            overflow.fill().0,
            BANK_NODES + 1,
            "the insert claimed one node of the second bank"
        );
        let link = overflow.head_link_for_test(0);
        assert_ne!(link, END, "the block's chain head names the new node");
        assert_eq!((link >> 8) as usize, 1, "the node went to the second candidate bank");
        assert_link_is_a_node_index(link);
    }

    #[test]
    fn empty_overflow_reads_nothing() {
        // The empty answers (`None`, `false`) are right for an empty
        // structure; this test is a regression guard from the RED skeleton on.
        let overflow =
            ChainedTphtOverflow::<2>::new(miri_scaled(1, 1), 1).expect("one block and one bank");
        assert_eq!(overflow.lookup(0, 0xDEAD_BEEF), None, "an empty overflow resolves nothing");
        assert!(!overflow.remove(0, 0xDEAD_BEEF), "an empty overflow removes nothing");
    }

    #[test]
    fn fill_exceeds_95_percent_before_first_failure() {
        // Fill is nodes in use divided by (255 * num_banks) at the first
        // insert that finds both candidate banks full.  "Allocation failure"
        // means that insert, not a heap failure (RESEARCH Open Question 7).
        //
        // A two-bank table (the Miri size) cannot show a 64-bank fill
        // figure, so the fraction is asserted only in native runs; the insert
        // loop runs in both, and at least one insert must succeed in both.
        let blocks = miri_scaled(4_096, 32);
        let banks = miri_scaled(64, 2);
        let seed_count = miri_scaled(FILL_SEEDS.len(), 2);
        let capacity = banks * BANK_NODES;
        let mut min_fraction = 1.0f64;
        for &seed in &FILL_SEEDS[..seed_count] {
            let overflow =
                ChainedTphtOverflow::<2>::new(blocks, banks).expect("a power-of-two block count");
            let mut rng = SplitMix64::new(seed);
            let mut used = 0;
            let mut steps = 0;
            loop {
                assert!(
                    steps < capacity * 3,
                    "seed {seed:#018x} never reported both candidate banks full"
                );
                steps += 1;
                let block = (rng.next() as usize) % blocks;
                let key = rng.next();
                match overflow.insert(block, key, [key, !key]) {
                    Ok(()) => used += 1,
                    Err(TphtError::BanksFull) => break,
                    Err(TphtError::Exists) => continue,
                    Err(other) => panic!("unexpected insert error: {other:?}"),
                }
            }
            let (reported, reported_capacity) = overflow.fill();
            assert_eq!(
                reported, used,
                "fill() must report the successful inserts (seed {seed:#018x})"
            );
            assert_eq!(reported_capacity, capacity, "fill() must report 255 nodes per bank");
            assert!(used > 0, "at least one insert must succeed (seed {seed:#018x})");
            let fraction = used as f64 / capacity as f64;
            min_fraction = min_fraction.min(fraction);
            println!(
                "FILL chained seed={seed:#018x} used={used} capacity={capacity} fraction={fraction:.4}"
            );
            if !cfg!(miri) {
                assert!(
                    fraction > 0.95,
                    "seed {seed:#018x} filled only {fraction:.4} ({used}/{capacity}) before the first \
                     insert that found both candidate banks full; DS-04 requires over 0.95"
                );
            }
        }
        println!("FILL chained min={min_fraction:.4}");
    }

    #[test]
    fn stress_concurrent_claims_never_share_a_node() {
        let threads = miri_scaled(4, 2);
        let ops = miri_scaled(100, 60);
        let blocks = threads;
        // Two banks make every block share both candidate banks.
        let overflow =
            Arc::new(ChainedTphtOverflow::<2>::new(blocks, 2).expect("two banks are valid"));
        let barrier = Arc::new(Barrier::new(threads));
        let mut handles = Vec::new();
        for thread_index in 0..threads {
            let overflow = Arc::clone(&overflow);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let mut inserted = 0;
                for i in 0..ops {
                    let key = ((thread_index as u64) << 32) | i as u64;
                    overflow
                        .insert(thread_index, key, [key, !key])
                        .expect("disjoint key sets under the bank capacity must insert");
                    inserted += 1;
                }
                inserted
            }));
        }
        let total: usize =
            handles.into_iter().map(|handle| handle.join().expect("stress thread completes")).sum();
        assert!(total > 0, "at least one insert must succeed");
        assert_eq!(
            overflow.fill().0,
            total,
            "the fill count must equal the successful inserts, so no node was claimed twice"
        );
        for thread_index in 0..threads {
            for i in 0..ops {
                let key = ((thread_index as u64) << 32) | i as u64;
                assert_eq!(
                    overflow.lookup(thread_index, key),
                    Some([key, !key]),
                    "every inserted key resolves to its own self-checking value"
                );
            }
        }
    }

    #[test]
    fn stress_reader_never_sees_partial_node() {
        let readers = miri_scaled(3, 1);
        let writer_ops = miri_scaled(300, 60);
        // One block with two candidate banks holds up to 510 nodes, so the
        // writer's 300 native inserts fit.
        let overflow =
            Arc::new(ChainedTphtOverflow::<2>::new(1, 2).expect("one block and two banks"));
        let barrier = Arc::new(Barrier::new(readers + 1));
        let mut handles = Vec::new();
        {
            let overflow = Arc::clone(&overflow);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let mut inserted = 0;
                for i in 0..writer_ops {
                    let key = i as u64;
                    overflow
                        .insert(0, key, [key, !key])
                        .expect("one block with two banks holds every key");
                    inserted += 1;
                }
                assert!(inserted > 0, "the writer must insert at least one key");
            }));
        }
        for reader in 0..readers {
            let overflow = Arc::clone(&overflow);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let mut rng = SplitMix64::new(0x0000_0000_0000_0e07 ^ reader as u64);
                for _ in 0..writer_ops {
                    let key = rng.next() % writer_ops as u64;
                    if let Some(value) = overflow.lookup(0, key) {
                        assert_eq!(value[0], key, "a reader got another key's value");
                        assert_eq!(
                            value[1], !key,
                            "a reader got a partially written node for key {key}"
                        );
                    }
                }
            }));
        }
        for handle in handles {
            handle.join().expect("stress thread completes");
        }
    }

    #[test]
    fn writing_sentinel_is_never_a_node_index() {
        let threads = miri_scaled(4, 2);
        let ops = miri_scaled(50, 30);
        let overflow =
            Arc::new(ChainedTphtOverflow::<2>::new(threads, 2).expect("two banks are valid"));
        let barrier = Arc::new(Barrier::new(threads));
        let mut handles = Vec::new();
        for thread_index in 0..threads {
            let overflow = Arc::clone(&overflow);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                for i in 0..ops {
                    let key = ((thread_index as u64) << 32) | i as u64;
                    overflow
                        .insert(thread_index, key, [key, !key])
                        .expect("disjoint key sets under the bank capacity must insert");
                }
            }));
        }
        for handle in handles {
            handle.join().expect("stress thread completes");
        }
        assert!(
            overflow.hook.claims() >= 1,
            "at least one insert must claim a node before this pin can mean anything"
        );
        assert_eq!(overflow.hook.writing(), 0, "every claimed node was published");
        assert!(
            overflow.hook.max_index() < BANK_NODES as u8,
            "a claimed index is a node index, never 0xFF"
        );
        for head in overflow.heads.iter() {
            assert_link_is_a_node_index(head.load(Ordering::Relaxed));
        }
        for bank in overflow.banks.iter() {
            for node in bank.nodes.iter() {
                assert_link_is_a_node_index(node.next.load(Ordering::Relaxed));
                let state = node.state.load(Ordering::Relaxed);
                assert!(
                    state == FREE || state == LIVE,
                    "no node is left in WRITING after the run (state {state:#04x})"
                );
            }
        }
    }
}
