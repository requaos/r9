//! setops — the Roaring bitmap set-algebra seam.
//!
//! The module holds the four hot loops (bitmap-container `and`, `or`,
//! `andnot`; array intersection) behind one internal boundary.  Each takes
//! slices and writes its result through `out` (the bitmap kernels also
//! return the result cardinality), so the SIMD replacement needs no caller
//! change.
//!
//! Phase 3 CONTEXT D-20 reserved the seam; Phase 5 MEM-17 (D-19) fills it.
//! The AVX2/NEON kernels live in the one reserved audited unsafe module
//! `crate::setops_simd` (mounted at the crate root because a `forbid`
//! reaches submodules).  `bitmap_and`, `bitmap_or` and `bitmap_andnot`
//! dispatch to that module when the arch boot has enabled
//! SIMD (`enable_simd`, set only after the CPU-feature check) and the
//! registered kernel bracket is in place (`register_simd_guard`, the
//! `fpu::with_simd` trampoline on a kernel).  The scalar bodies remain and
//! are pinned bit-for-bit against the vector path by the host tests and the
//! boot self-test.
//!
//! This module stays strictly `#[forbid(unsafe_code)]`: the SIMD
//! instructions live in `setops_simd`, and everything here is safe code.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

/// A bitmap container holds 1024 64-bit words, 65,536 bits in all.
pub(crate) const BITMAP_WORDS: usize = 1024;

/// Whether the SIMD kernels are active on this boot.  Set once by the arch
/// crate after the CPU-feature decision and the kernel-bracket registration
/// (MEM-17); false keeps every operation on the scalar loops.
static SIMD_ENABLED: AtomicBool = AtomicBool::new(false);

/// Whether the SIMD kernels are enabled on this boot.
pub(crate) fn simd_enabled() -> bool {
    SIMD_ENABLED.load(Ordering::Relaxed)
}

/// Enable or disable the SIMD kernels.  Called once at boot by the arch
/// crate (through `port::setops`), after registering the `fpu::with_simd`
/// bracket and checking the CPU feature.
pub fn enable_simd(enabled: bool) {
    SIMD_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Register the kernel-SIMD bracket the SIMD kernels run inside.  A kernel
/// passes `|f| fpu::with_simd(|_| f())`; host tests leave it unregistered
/// and the kernels run directly.
pub fn register_simd_guard(guard: fn(&mut dyn FnMut())) {
    crate::setops_simd::register_simd_guard(guard);
}

/// Word-wise `out = a & b`; returns the cardinality of the result.
pub(crate) fn bitmap_and(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    crate::setops_simd::run_and_if_enabled(a, b, out)
}

/// Word-wise `out = a | b`; returns the cardinality of the result.
pub(crate) fn bitmap_or(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    crate::setops_simd::run_or_if_enabled(a, b, out)
}

/// Word-wise `out = a & !b`; returns the cardinality of the result.
pub(crate) fn bitmap_andnot(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    crate::setops_simd::run_andnot_if_enabled(a, b, out)
}

/// The scalar `out = a & b` reference body; the boot self-test and the
/// SIMD-equivalence pins compare against it.
pub(crate) fn bitmap_and_scalar(
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

/// The scalar `out = a | b` reference body.
pub(crate) fn bitmap_or_scalar(
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

/// The scalar `out = a & !b` reference body.
pub(crate) fn bitmap_andnot_scalar(
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

/// SplitMix64's next output — the seeded generator the boot self-test uses.
/// The host tests carry the same generator in their own `SplitMix64` struct.
fn next_splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
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

/// Boot self-test (MEM-17): every case below produces bit-for-bit identical
/// words and an identical cardinality on the scalar reference and the active
/// path (`bitmap_*`, which dispatches to SIMD when `enable_simd(true)` was
/// called before it).  Cases: all-zero, all-ones, single bits at words
/// 0/511/1023, alternating 0x5555…/0xAAAA…, seeded random, and `b = !a`.
/// Returns false on the first mismatch.
pub fn selftest_simd_matches_scalar() -> bool {
    let mut state = 0x5DEE_CE66_D1CE_4B1Du64;
    for case in 0..6u64 {
        let mut a = alloc::vec![0u64; BITMAP_WORDS];
        let mut b = alloc::vec![0u64; BITMAP_WORDS];
        match case {
            0 => {}
            1 => {
                a.fill(u64::MAX);
                b.fill(u64::MAX);
            }
            2 => {
                a[0] = 1;
                a[BITMAP_WORDS / 2 - 1] = 1 << 63;
                a[BITMAP_WORDS - 1] = 1 << 63;
                b[0] = 1;
                b[BITMAP_WORDS / 2 - 1] = 1 << 63;
                b[BITMAP_WORDS - 1] = 1 << 63;
            }
            3 => {
                for (i, word) in a.iter_mut().enumerate() {
                    *word = if i % 2 == 0 { 0x5555_5555_5555_5555 } else { 0xAAAA_AAAA_AAAA_AAAA };
                }
                for (i, word) in b.iter_mut().enumerate() {
                    *word = if i % 2 == 0 { 0xAAAA_AAAA_AAAA_AAAA } else { 0x5555_5555_5555_5555 };
                }
            }
            4 => {
                for word in &mut a {
                    *word = next_splitmix64(&mut state);
                }
                for word in &mut b {
                    *word = next_splitmix64(&mut state);
                }
            }
            _ => {
                for word in &mut a {
                    *word = next_splitmix64(&mut state);
                }
                for (i, word) in b.iter_mut().enumerate() {
                    *word = !a[i];
                }
            }
        }

        let a_ref: &[u64; BITMAP_WORDS] = a.as_slice().try_into().unwrap();
        let b_ref: &[u64; BITMAP_WORDS] = b.as_slice().try_into().unwrap();
        let mut out_scalar = alloc::vec![0u64; BITMAP_WORDS];
        let mut out_active = alloc::vec![0u64; BITMAP_WORDS];

        let scalar_card = {
            let out: &mut [u64; BITMAP_WORDS] = out_scalar.as_mut_slice().try_into().unwrap();
            bitmap_and_scalar(a_ref, b_ref, out)
        };
        let active_card = {
            let out: &mut [u64; BITMAP_WORDS] = out_active.as_mut_slice().try_into().unwrap();
            bitmap_and(a_ref, b_ref, out)
        };
        if out_scalar != out_active || scalar_card != active_card {
            return false;
        }

        let scalar_card = {
            let out: &mut [u64; BITMAP_WORDS] = out_scalar.as_mut_slice().try_into().unwrap();
            bitmap_or_scalar(a_ref, b_ref, out)
        };
        let active_card = {
            let out: &mut [u64; BITMAP_WORDS] = out_active.as_mut_slice().try_into().unwrap();
            bitmap_or(a_ref, b_ref, out)
        };
        if out_scalar != out_active || scalar_card != active_card {
            return false;
        }

        let scalar_card = {
            let out: &mut [u64; BITMAP_WORDS] = out_scalar.as_mut_slice().try_into().unwrap();
            bitmap_andnot_scalar(a_ref, b_ref, out)
        };
        let active_card = {
            let out: &mut [u64; BITMAP_WORDS] = out_active.as_mut_slice().try_into().unwrap();
            bitmap_andnot(a_ref, b_ref, out)
        };
        if out_scalar != out_active || scalar_card != active_card {
            return false;
        }
    }
    true
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
            next_splitmix64(&mut self.state)
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

    /// Whether this host can execute the vector kernel directly: NEON is
    /// mandatory on aarch64; x86_64 needs the AVX2 feature.
    fn host_simd_available() -> bool {
        #[cfg(target_arch = "aarch64")]
        {
            true
        }
        #[cfg(target_arch = "x86_64")]
        {
            std::arch::is_x86_feature_detected!("avx2")
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            false
        }
    }

    /// Pin one input pair on all three operations: the vector kernel
    /// (called directly, so the process-global flag never races) against
    /// the scalar reference — exact words AND exact cardinality.
    fn check_simd_case(a: &[u64; BITMAP_WORDS], b: &[u64; BITMAP_WORDS]) {
        let mut out = [0u64; BITMAP_WORDS];
        let mut want = [0u64; BITMAP_WORDS];

        let card = crate::setops_simd::and_direct(a, b, &mut out);
        let want_card = bitmap_and_scalar(a, b, &mut want);
        assert_eq!(out, want, "SIMD AND words differ");
        assert_eq!(card, want_card, "SIMD AND cardinality differs");

        out = [0u64; BITMAP_WORDS];
        let card = crate::setops_simd::or_direct(a, b, &mut out);
        let want_card = bitmap_or_scalar(a, b, &mut want);
        assert_eq!(out, want, "SIMD OR words differ");
        assert_eq!(card, want_card, "SIMD OR cardinality differs");

        out = [0u64; BITMAP_WORDS];
        let card = crate::setops_simd::andnot_direct(a, b, &mut out);
        let want_card = bitmap_andnot_scalar(a, b, &mut want);
        assert_eq!(out, want, "SIMD ANDNOT words differ");
        assert_eq!(card, want_card, "SIMD ANDNOT cardinality differs");
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

    /// MEM-17: the host CPU's SIMD path produces bit-for-bit identical words
    /// and cardinalities to the scalar reference on all 8 seeds and every
    /// edge case (all-zero, all-ones, single bits, alternating, complement).
    #[test]
    fn simd_kernels_match_scalar_on_all_seeds_and_edges() {
        if !host_simd_available() {
            // x86_64 host without AVX2: the kernel cannot run here; the
            // scalar path is pinned bit-for-bit by bitmap_kernels_match_naive.
            return;
        }

        let zero = [0u64; BITMAP_WORDS];
        let ones = [u64::MAX; BITMAP_WORDS];
        let mut alternating = [0u64; BITMAP_WORDS];
        for (i, word) in alternating.iter_mut().enumerate() {
            *word = if i % 2 == 0 { 0x5555_5555_5555_5555 } else { 0xAAAA_AAAA_AAAA_AAAA };
        }
        let mut single = [0u64; BITMAP_WORDS];
        single[0] = 1;
        single[BITMAP_WORDS / 2 - 1] = 1 << 63;
        single[BITMAP_WORDS - 1] = 1 << 63;

        check_simd_case(&zero, &zero);
        check_simd_case(&zero, &ones);
        check_simd_case(&ones, &ones);
        check_simd_case(&ones, &zero);
        check_simd_case(&single, &single);
        check_simd_case(&alternating, &ones);
        check_simd_case(&alternating, &alternating);

        for seed in SEEDS {
            let mut rng = SplitMix64::new(seed);
            let a = random_words(&mut rng);
            let b = random_words(&mut rng);
            check_simd_case(&a, &b);

            let mut complement = [0u64; BITMAP_WORDS];
            for (dst, src) in complement.iter_mut().zip(a.iter()) {
                *dst = !src;
            }
            check_simd_case(&a, &complement);
        }
    }

    /// MEM-17: with the SIMD path enabled, the full Roaring set algebra
    /// (union / intersection / difference, whose bitmap containers run the
    /// kernels) stays bit-identical to the scalar path.
    #[test]
    fn roaring_algebra_matches_oracle_with_simd_enabled() {
        use crate::roaring::RoaringBitmap;

        if !host_simd_available() {
            return;
        }

        let mut a = RoaringBitmap::new();
        let mut b = RoaringBitmap::new();
        // Dense enough to build bitmap containers (> ARRAY_MAX values in a
        // chunk), which is the only place the set kernels run.
        for v in 0..80_000u32 {
            if v % 3 == 0 {
                a.insert(v);
            }
            if v % 5 == 0 {
                b.insert(v);
            }
        }

        crate::setops::enable_simd(false);
        let scalar = (a.union(&b), a.intersection(&b), a.difference(&b));
        crate::setops::enable_simd(true);
        let vector = (a.union(&b), a.intersection(&b), a.difference(&b));
        crate::setops::enable_simd(false);

        let values = |m: &RoaringBitmap| m.iter().collect::<Vec<u32>>();
        assert_eq!(values(&scalar.0), values(&vector.0), "union differs with SIMD enabled");
        assert_eq!(
            values(&scalar.1),
            values(&vector.1),
            "intersection differs with SIMD enabled"
        );
        assert_eq!(
            values(&scalar.2),
            values(&vector.2),
            "difference differs with SIMD enabled"
        );
        assert_eq!(scalar.0.len(), vector.0.len());
        assert_eq!(scalar.1.len(), vector.1.len());
        assert_eq!(scalar.2.len(), vector.2.len());
    }

    #[test]
    fn selftest_agrees_with_the_scalar_path() {
        // Exercises the dispatch seam end to end.  Enabling only when the
        // host can run the kernel keeps a non-AVX2 host on the scalar path
        // (where the self-test is still a meaningful word-for-word check).
        let simd = host_simd_available();
        crate::setops::enable_simd(simd);
        let ok = selftest_simd_matches_scalar();
        crate::setops::enable_simd(false);
        assert!(ok, "selftest_simd_matches_scalar disagreed (simd={simd})");
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
