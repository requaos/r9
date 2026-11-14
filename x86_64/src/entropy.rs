//! The x86_64 entropy gather — devcons's RandomHooks implementations
//! (D-05, AUTH-02, 18-01).  The aarch64 DTB-seeded design re-keyed for
//! x86_64: there is no DTB here, and before 18-01 the x86_64 kernel
//! registered NO hooks (devcons's deterministic-seed era).  The port
//! seam — RandomHooks, the ChaCha20 DRBG core, the per-read jitter
//! mix, the `devcons: random seeded` once-marker — is untouched; this
//! module only supplies the seed bytes and the jitter sample.
//!
//! Source selection is the 13-03 CPUID-gated dual-mode precedent
//! applied to entropy: RDSEED when CPUID leaf 7 advertises it (the
//! conditioned-source-for-seeding instruction per the Intel DRNG
//! Software Implementation Guide), RDRAND when only leaf-1 ECX bit 30
//! is present, and the two-sample TSC fold otherwise (TCG's qemu64
//! exposes neither bit — its emulated-TSC read variance is the entropy
//! source there, load-bearing on the CI leg).  Which source armed is
//! published once per boot by main's registration print
//! (`random: source=...` — the D-07 gate vocabulary; RDSEED-armed
//! boots print the rdrand-family label: the two-way distinction is
//! RDRAND-family vs jitter).

use crate::cpu;

/// The per-word RDSEED/RDRAND bounded-retry budget — the Intel DRNG
/// guide's retry-loop guidance: retry a small number of times, NEVER
/// spin unbounded (a transiently exhausted DRNG falls through to the
/// jitter mix rather than hanging the seed path).
const DRNG_RETRIES: u32 = 10;

/// The entropy source three-way classification — the D-05 gate.  The
/// two-way PUBLISHED label folds Rdseed and Rdrand together (the
/// rdrand family); the distinction this enum carries is which fill
/// path `rng_seed` drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Source {
    Rdseed,
    Rdrand,
    Jitter,
}

/// Pure decision core — host-testable without a CPU: RDSEED wins when
/// its bit is present AND the max-leaf check admits leaf 7, RDRAND is
/// the fallback instruction, and the TSC fold arms when neither bit
/// is exposed.
pub(crate) fn classify(max_leaf: u32, cpuid1_ecx: u32, cpuid7_ebx: u32) -> Source {
    if cpu::has_rdseed(max_leaf, cpuid7_ebx) {
        Source::Rdseed
    } else if cpu::has_rdrand(cpuid1_ecx) {
        Source::Rdrand
    } else {
        Source::Jitter
    }
}

/// The host-pin source override (entropy tests only).  The live probe
/// would classify the HOST CPU — this dev machine carries rdrand+
/// rdseed on every core — so pins force the source under test and
/// script the TSC through the seam below instead.
#[cfg(test)]
static TEST_SOURCE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(2);

/// The scripted TSC queue (cfg(test) only — module-level so the seam
/// and the test helper share ONE storage).  Pins push the exact
/// sample sequence the calls under test will draw; the queue drains
/// front-first and an empty queue is a loud test bug, never a silent
/// real-counter read.
#[cfg(test)]
mod test_tsc {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    thread_local! {
        pub static QUEUE: RefCell<VecDeque<u64>> = const { RefCell::new(VecDeque::new()) };
    }
}

/// The TSC sample seam: `rdtsc` in production; under cfg(test) the
/// scripted queue so the mixing pins NEVER read the real counter —
/// the pinned shape must be deterministic (the 18-01 host-pin
/// contract).
#[cfg(not(test))]
fn tsc() -> u64 {
    cpu::rdtsc()
}

#[cfg(test)]
fn tsc() -> u64 {
    test_tsc::QUEUE.with(|q| {
        q.borrow_mut().pop_front().expect(
            "entropy test: TSC queue empty — push the scripted samples before the call under test",
        )
    })
}

/// Probe the live CPUID state and classify the armed source.  Main's
/// registration consults this once, beside the hook registration
/// (before any consumer: the ether bind's smoltcp seed and the first
/// exec both draw through the hooks).
pub(crate) fn armed_source() -> Source {
    #[cfg(test)]
    {
        match TEST_SOURCE.load(core::sync::atomic::Ordering::Relaxed) {
            0 => Source::Rdseed,
            1 => Source::Rdrand,
            _ => Source::Jitter,
        }
    }
    #[cfg(not(test))]
    {
        let (max_leaf, _, _, _) = cpu::cpuid(0, 0);
        let (_, ebx7, ecx1, _) = cpu::cpuid(7, 0);
        classify(max_leaf, ecx1, ebx7)
    }
}

/// The pure two-sample fold — the x86_64 equivalent of aarch64's
/// CNTFRQ^CNTPCT^ticks mix, in the two-sample decorrelation shape of
/// devcons's `drbg_mix_jitter` (devcons.rs): the xor of the samples
/// (rotated apart) plus their difference, so both halves of the pair
/// and the distance between them each contribute.  Identical sample
/// pairs reproduce identically; differing pairs fold differently.
fn jitter_fold(a: u64, b: u64) -> u64 {
    a ^ b.rotate_left(17) ^ a.wrapping_sub(b).rotate_left(43)
}

/// devcons's jitter hook: `rdtsc` sampled twice with a fold — the
/// second sample advances between the reads even within one tick
/// (TCG's emulated-TSC read variance is the entropy source on the
/// no-RDRAND venue; on KVM the DRNG paths carry the entropy and this
/// is the per-read stirring term).
pub fn jitter() -> u64 {
    let a = tsc();
    let b = tsc();
    jitter_fold(a, b)
}

/// One jitter-derived word for slot `i` (the jitter fill's word shape
/// and the DRNG-exhaustion fallback): the two-sample fold whitened
/// against the slot index with a distinct rotation per slot, so
/// repeated words differ per position even on a slowly-moving
/// counter.
fn jitter_word(i: usize) -> u64 {
    jitter()
        .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .rotate_left((i % 64) as u32)
}

/// The DRNG fill: one qword per chunk under the bounded-retry rule.
/// A word that never validates after `DRNG_RETRIES` draws falls
/// through to a jitter fold — the fill ALWAYS writes the full buffer
/// (0 is legal only if truly nothing was sampled; this shape never
/// returns short).
fn fill_from_drng<D: FnMut() -> Option<u64>>(mut draw: D, buf: &mut [u8]) -> usize {
    for (i, chunk) in buf.chunks_mut(8).enumerate() {
        let mut word = None;
        for _ in 0..DRNG_RETRIES {
            if let Some(v) = draw() {
                word = Some(v);
                break;
            }
        }
        let w = match word {
            Some(v) => v,
            None => jitter_word(i),
        };
        chunk.copy_from_slice(&w.to_le_bytes()[..chunk.len()]);
    }
    buf.len()
}

/// The jitter fill (no RDRAND/RDSEED): repeated two-sample TSC folds,
/// one qword per chunk — same always-fill contract as the DRNG path.
fn fill_from_jitter(buf: &mut [u8]) -> usize {
    for (i, chunk) in buf.chunks_mut(8).enumerate() {
        let w = jitter_word(i);
        chunk.copy_from_slice(&w.to_le_bytes()[..chunk.len()]);
    }
    buf.len()
}

/// devcons's rng_seed hook: fill `buf` with seed bytes from the armed
/// source — RDSEED when present, RDRAND when only it is, the TSC/jitter
/// fold otherwise.  Returns the byte count written (always
/// `buf.len()` — see the fill contracts above).
pub fn rng_seed(buf: &mut [u8]) -> usize {
    match armed_source() {
        Source::Rdseed => fill_from_drng(cpu::rdseed, buf),
        Source::Rdrand => fill_from_drng(cpu::rdrand, buf),
        Source::Jitter => fill_from_jitter(buf),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Script the TSC seam: replace the queue with `samples` — the
    /// calls under test draw exactly these, front-first (two per
    /// jitter(); never the real counter).
    fn push_tsc(samples: &[u64]) {
        test_tsc::QUEUE.with(|q| *q.borrow_mut() = samples.iter().copied().collect());
    }

    fn set_source(s: Source) {
        TEST_SOURCE.store(s as u8, core::sync::atomic::Ordering::Relaxed);
    }

    /// has_rdrand: leaf-1 ECX bit 30, set and clear vectors — set
    /// alone or among others; clear on zero, the neighbouring bits,
    /// and the qemu64 TCG leaf-1 shape verbatim (SSE3|CX16 only).
    #[test]
    fn entropy_has_rdrand_keys_off_leaf1_ecx_bit30() {
        assert!(cpu::has_rdrand(1 << 30));
        assert!(cpu::has_rdrand((1 << 30) | (1 << 21) | (1 << 24)));
        assert!(!cpu::has_rdrand(0));
        assert!(!cpu::has_rdrand(1 << 29));
        assert!(!cpu::has_rdrand(1 << 31));
        assert!(!cpu::has_rdrand((1 << 0) | (1 << 13)));
    }

    /// has_rdseed: leaf-7 EBX bit 18 with the max-leaf check FIRST —
    /// a CPU whose maximum standard leaf is below 7 returns its
    /// maximum leaf's values for a leaf-7 request, so those EBX bits
    /// are meaningless (SDM Vol 2, CPUID).
    #[test]
    fn entropy_has_rdseed_keys_off_leaf7_ebx_bit18_maxleaf_first() {
        // set: bit 18, over plausible max-leaf values (7 .. 0x20)
        assert!(cpu::has_rdseed(7, 1 << 18));
        assert!(cpu::has_rdseed(0x16, 1 << 18));
        assert!(cpu::has_rdseed(0x20, (1 << 18) | (1 << 19)));
        // clear: bit 18 with max leaf admitted
        assert!(!cpu::has_rdseed(7, 0));
        assert!(!cpu::has_rdseed(0x16, (1 << 17) | (1 << 19)));
        // max-leaf reject: the bit "present" in the recycled EBX bits
        // but the CPU cannot answer leaf 7 — NOT rdseed; boundary at 7
        assert!(!cpu::has_rdseed(3, 1 << 18));
        assert!(!cpu::has_rdseed(6, 1 << 18));
        assert!(cpu::has_rdseed(7, 1 << 18));
    }

    /// The classification composition: RDSEED wins over RDRAND; the
    /// max-leaf reject demotes an RDSEED-bit shape to the RDRAND arm;
    /// neither bit arms the TSC fold.
    #[test]
    fn entropy_classify_orders_rdseed_rdrand_jitter() {
        assert_eq!(classify(0x16, 1 << 30, 1 << 18), Source::Rdseed);
        // rdseed bit present but leaf 7 unreachable: falls to rdrand
        assert_eq!(classify(6, 1 << 30, 1 << 18), Source::Rdrand);
        assert_eq!(classify(0x16, 1 << 30, 0), Source::Rdrand);
        // the KVM -cpu host shape on this machine (both bits) vs TCG
        // qemu64 (neither — only SSE3|CX16 on leaf 1, no leaf 7 bits)
        assert_eq!(classify(0x20, (1 << 30) | (1 << 21), 1 << 18), Source::Rdseed);
        assert_eq!(classify(3, (1 << 0) | (1 << 13), 0), Source::Jitter);
        assert_eq!(classify(0x20, 0, 0), Source::Jitter);
    }

    /// The mixing pin (the 18-01 must-have): two rng_seed fills over
    /// DIFFERING TSC inputs differ, while the SAME input reproduces —
    /// injected through the cfg(test) seam, never the real counter.
    #[test]
    fn entropy_rng_seed_fill_same_input_reproduces_differing_differs() {
        set_source(Source::Jitter);
        // 64-byte fill = 8 words x one jitter() = 16 TSC samples.
        let frozen: Vec<u64> = std::iter::repeat(11u64).take(16).collect();
        let advancing: Vec<u64> = (0..16).map(|i| 1000 + i as u64 * 37).collect();
        let mut a = [0u8; 64];
        push_tsc(&frozen);
        assert_eq!(rng_seed(&mut a), 64);
        let mut b = [0u8; 64];
        push_tsc(&frozen);
        assert_eq!(rng_seed(&mut b), 64);
        // same input reproduces byte-for-byte (the frozen counter is
        // the degenerate case — the fold still lands deterministic
        // bytes, never garbage)
        assert_eq!(a, b);
        // differing input differs somewhere across the fill
        let mut c = [0u8; 64];
        push_tsc(&advancing);
        assert_eq!(rng_seed(&mut c), 64);
        assert_ne!(a, c);
        // a non-degenerate script must land non-zero bytes (a seed of
        // all zeros would slip through the distinctness check above)
        assert!(c.iter().any(|&x| x != 0));
    }

    /// The two-sample fold itself: identical pairs reproduce, any
    /// perturbation of either half (or the pair distance) changes the
    /// fold.
    #[test]
    fn entropy_jitter_fold_reproduces_and_separates() {
        assert_eq!(
            jitter_fold(0x1234_5678_9abc_def0, 0x0f0f),
            jitter_fold(0x1234_5678_9abc_def0, 0x0f0f)
        );
        assert_ne!(jitter_fold(10, 11), jitter_fold(11, 10));
        assert_ne!(jitter_fold(10, 11), jitter_fold(10, 12));
        assert_ne!(jitter_fold(10, 11), jitter_fold(12, 11));
    }

    /// The DRNG retry shape: a never-validating draw exhausts the
    /// bounded budget (never spins) and still fills via the jitter
    /// fallback; a flaky draw's validated word lands verbatim.
    #[test]
    fn entropy_drng_fill_bounded_retry_and_fallback() {
        // exhausted retries -> jitter fallback fills every word: a
        // 24-byte buffer = 3 words x 2 TSC samples
        let script: Vec<u64> = (0..6).map(|i| 500 + i as u64 * 3).collect();
        let mut via_fallback = [0u8; 24];
        push_tsc(&script);
        let mut calls = 0u32;
        let n = fill_from_drng(
            || {
                calls += 1;
                None
            },
            &mut via_fallback,
        );
        assert_eq!(n, 24);
        assert_eq!(calls, 3 * DRNG_RETRIES);
        assert!(via_fallback.iter().any(|&x| x != 0));
        // a draw that validates on the third try: the word is the
        // draw's (little-endian), no fallback sample consumed
        let mut flaky = 0;
        let mut via_draw = [0u8; 8];
        let n = fill_from_drng(
            || {
                flaky += 1;
                if flaky >= 3 { Some(0xded0_beef_cafe_f00d) } else { None }
            },
            &mut via_draw,
        );
        assert_eq!(n, 8);
        assert_eq!(flaky, 3);
        assert_eq!(via_draw, 0xded0_beef_cafe_f00d_u64.to_le_bytes());
    }
}
