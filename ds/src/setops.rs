//! setops — the Phase 3 CONTEXT D-20 seam: the four scalar hot loops
//! (bitmap-container and, or, andnot; array intersection) that a Phase 5
//! SIMD module can replace behind one internal boundary.
//!
//! The module holds exactly four non-test functions.  Each takes slices and
//! writes its result through `out` (the bitmap kernels also return the result
//! cardinality), so a replacement body needs no caller change.  A Phase 5
//! SIMD module is mounted at the crate root — a `forbid` reaches submodules —
//! and swaps these bodies behind `fpu::kernel_begin()` / `with_simd()`
//! (D-20).
//!
//! No SIMD code lands here in this phase (D-09, D-20).

use alloc::vec::Vec;

/// A bitmap container holds 1024 64-bit words, 65,536 bits in all.
pub(crate) const BITMAP_WORDS: usize = 1024;

/// Word-wise `out = a & b`; returns the cardinality of the result.
pub(crate) fn bitmap_and(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let mut card = 0u32;
    for word in 0..BITMAP_WORDS {
        let both = a[word] & b[word];
        out[word] = both;
        card += both.count_ones();
    }
    card
}

/// Word-wise `out = a | b`; returns the cardinality of the result.
pub(crate) fn bitmap_or(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let mut card = 0u32;
    for word in 0..BITMAP_WORDS {
        let either = a[word] | b[word];
        out[word] = either;
        card += either.count_ones();
    }
    card
}

/// Word-wise `out = a & !b`; returns the cardinality of the result.
pub(crate) fn bitmap_andnot(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let mut card = 0u32;
    for word in 0..BITMAP_WORDS {
        let only_a = a[word] & !b[word];
        out[word] = only_a;
        card += only_a.count_ones();
    }
    card
}

/// Two-pointer intersection of two ascending arrays; `out` is replaced.
pub(crate) fn array_intersect(a: &[u16], b: &[u16], out: &mut Vec<u16>) {
    out.clear();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            core::cmp::Ordering::Less => i += 1,
            core::cmp::Ordering::Greater => j += 1,
            core::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Op {
        And,
        Or,
        AndNot,
    }

    /// Bit-by-bit reference: deliberately not a word loop, so it shares no
    /// code with the kernels under test.
    fn naive_bitmap(op: Op, a: &[u64; BITMAP_WORDS], b: &[u64; BITMAP_WORDS]) -> ([u64; BITMAP_WORDS], u32) {
        let mut out = [0u64; BITMAP_WORDS];
        let mut card = 0u32;
        for bit in 0..(BITMAP_WORDS * 64) {
            let av = (a[bit / 64] >> (bit % 64)) & 1;
            let bv = (b[bit / 64] >> (bit % 64)) & 1;
            let v = match op {
                Op::And => av & bv,
                Op::Or => av | bv,
                Op::AndNot => av & !bv,
            };
            out[bit / 64] |= v << (bit % 64);
            card += v as u32;
        }
        (out, card)
    }

    fn random_words(rng: &mut SplitMix64) -> [u64; BITMAP_WORDS] {
        let mut words = [0u64; BITMAP_WORDS];
        for word in &mut words {
            *word = match rng.next_u64() % 4 {
                0 => 0,
                1 => u64::MAX,
                _ => rng.next_u64(),
            };
        }
        words
    }

    fn sorted_unique(rng: &mut SplitMix64, len: usize) -> Vec<u16> {
        let mut values: Vec<u16> = (0..len).map(|_| rng.next_u64() as u16).collect();
        values.sort_unstable();
        values.dedup();
        values
    }

    #[test]
    fn bitmap_kernels_match_naive() {
        for seed in SEEDS {
            let mut rng = SplitMix64::new(seed);
            let a = random_words(&mut rng);
            let b = random_words(&mut rng);

            let mut out = [0u64; BITMAP_WORDS];
            let (naive, naive_card) = naive_bitmap(Op::And, &a, &b);
            let card = bitmap_and(&a, &b, &mut out);
            assert_eq!(out, naive, "bitmap_and words differ (seed {seed:#018x})");
            assert_eq!(card, naive_card, "bitmap_and cardinality differs (seed {seed:#018x})");

            out = [0u64; BITMAP_WORDS];
            let (naive, naive_card) = naive_bitmap(Op::Or, &a, &b);
            let card = bitmap_or(&a, &b, &mut out);
            assert_eq!(out, naive, "bitmap_or words differ (seed {seed:#018x})");
            assert_eq!(card, naive_card, "bitmap_or cardinality differs (seed {seed:#018x})");

            out = [0u64; BITMAP_WORDS];
            let (naive, naive_card) = naive_bitmap(Op::AndNot, &a, &b);
            let card = bitmap_andnot(&a, &b, &mut out);
            assert_eq!(out, naive, "bitmap_andnot words differ (seed {seed:#018x})");
            assert_eq!(card, naive_card, "bitmap_andnot cardinality differs (seed {seed:#018x})");
        }
    }

    #[test]
    fn array_intersect_matches_naive() {
        let empty: Vec<u16> = Vec::new();
        let mut out = Vec::new();
        array_intersect(&empty, &empty, &mut out);
        assert!(out.is_empty(), "empty with empty must be empty");

        let evens: Vec<u16> = (0..=1000u16).map(|v| v * 2).collect();
        let odds: Vec<u16> = (0..=1000u16).map(|v| v * 2 + 1).collect();
        array_intersect(&evens, &odds, &mut out);
        assert!(out.is_empty(), "disjoint inputs must give an empty result");
        array_intersect(&empty, &evens, &mut out);
        assert!(out.is_empty(), "empty with non-empty must be empty");

        for seed in SEEDS {
            let mut rng = SplitMix64::new(seed);
            let a = sorted_unique(&mut rng, 200);
            let b = sorted_unique(&mut rng, 300);
            let expected: Vec<u16> = a.iter().copied().filter(|x| b.iter().any(|y| y == x)).collect();
            array_intersect(&a, &b, &mut out);
            assert_eq!(out, expected, "array_intersect differs from the nested-loop reference (seed {seed:#018x})");
        }
    }
}
