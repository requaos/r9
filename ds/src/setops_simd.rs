//! setops_simd — the ONE reserved audited Roaring SIMD module in `ds/`
//! (Phase 3 CONTEXT D-09; Phase 5 MEM-17, D-19).
//!
//! The module holds the AVX2 (`x86_64`) and NEON (`aarch64`) versions of
//! the four bitmap-container hot loops behind [`crate::setops`].  It is
//! mounted at the crate root in `ds/src/lib.rs` because a
//! `#[forbid(unsafe_code)]` on `setops`/`roaring` reaches their submodules:
//! the reserved module cannot live under either.
//!
//! Every kernel executes raw vector instructions and is therefore an
//! `unsafe fn` with a concrete `// SAFETY:` contract.  Two rules make the
//! code sound:
//!
//! * The kernels are legal only when the CPU feature is on.  `x86_64`
//!   requires AVX2 (which the BSP arms only on the XSAVE path,
//!   [`crate::setops`] is enabled by the arch crate only after that
//!   decision; `qemu64` keeps the scalar path); `aarch64` NEON is
//!   mandatory on ARMv8.
//! * On a kernel build the kernels run strictly inside the registered
//!   kernel-SIMD bracket ([`register_simd_guard`]), which the arch boot
//!   passes as `fpu::with_simd` — so user FP/SIMD state is parked, IRQs
//!   are masked, and the `vzeroupper` the x86_64 kernel issues on exit
//!   clears the upper halves before the user state is restored (D-06,
//!   D-19, T-05-30).  Host tests register no bracket and the kernels run
//!   directly on the test CPU.
//!
//! `#[target_feature(enable = "avx2"/"neon")]` is deliberately NOT used:
//! on the soft-float kernel targets it silently emits scalar code on
//! `x86_64` and is ABI-unsound on `aarch64` (05-RESEARCH Pattern 5.5).
//! `core::arch::asm!` assembles the real instructions on both.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::setops::BITMAP_WORDS;

/// The registered kernel-SIMD bracket as its code address (0 = none):
/// a `fn(&mut dyn FnMut())` whose body runs the closure inside
/// `fpu::with_simd`.  Stored behind an atomic so `register_simd_guard` is
/// safe code in this one reserved unsafe module.
static GUARD: AtomicUsize = AtomicUsize::new(0);

/// Register the bracket the SIMD kernels run inside.  Kernel boot calls
/// this once with `|f| fpu::with_simd(|_| f())`; host tests leave it
/// unregistered and [`run_guarded`] calls the closure directly.
pub fn register_simd_guard(guard: fn(&mut dyn FnMut())) {
    GUARD.store(guard as usize, Ordering::Release);
}

/// Run `f` inside the registered bracket, or directly when none is
/// registered (the host-test path).
pub(crate) fn run_guarded(f: &mut dyn FnMut()) {
    let addr = GUARD.load(Ordering::Acquire);
    if addr == 0 {
        f();
    } else {
        // SAFETY: `GUARD`'s only writer, `register_simd_guard`, stores a
        // `fn(&mut dyn FnMut())` value's code address with no other value
        // possible; a function pointer is a code address of usize width, so
        // reconstructing the identical fn-pointer type from it is valid.
        // The call supplies an `&mut dyn FnMut()` of the type the pointer
        // was created from, and the bracket owns the section for the call.
        let guard: fn(&mut dyn FnMut()) = unsafe { core::mem::transmute(addr) };
        guard(f);
    }
}

/// Word-wise `out = a & b` through the active path; returns the result
/// cardinality.  Disabled → the scalar body in [`crate::setops`].
pub(crate) fn run_and_if_enabled(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    if !crate::setops::simd_enabled() {
        return crate::setops::bitmap_and_scalar(a, b, out);
    }
    let mut card = 0u32;
    {
        let written = &mut card;
        let mut f = || {
            // SAFETY: `run_guarded` invokes this inside the registered
            // kernel bracket (`fpu::with_simd` on a kernel), and
            // `simd_enabled()` is true only after boot verified the CPU
            // feature, so the AVX2/NEON instructions are legal.  The
            // kernel's own contract covers the exact-size buffers.
            *written = unsafe { bitmap_and_simd(a, b, out) };
        };
        run_guarded(&mut f);
    }
    card
}

/// Word-wise `out = a | b` through the active path; see
/// [`run_and_if_enabled`].
pub(crate) fn run_or_if_enabled(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    if !crate::setops::simd_enabled() {
        return crate::setops::bitmap_or_scalar(a, b, out);
    }
    let mut card = 0u32;
    {
        let written = &mut card;
        let mut f = || {
            // SAFETY: as `run_and_if_enabled`.
            *written = unsafe { bitmap_or_simd(a, b, out) };
        };
        run_guarded(&mut f);
    }
    card
}

/// Word-wise `out = a & !b` through the active path; see
/// [`run_and_if_enabled`].
pub(crate) fn run_andnot_if_enabled(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    if !crate::setops::simd_enabled() {
        return crate::setops::bitmap_andnot_scalar(a, b, out);
    }
    let mut card = 0u32;
    {
        let written = &mut card;
        let mut f = || {
            // SAFETY: as `run_and_if_enabled`.
            *written = unsafe { bitmap_andnot_simd(a, b, out) };
        };
        run_guarded(&mut f);
    }
    card
}

// -------------------------------------------------------------------------
// x86_64 AVX2 (256 bits = 4 u64 words per iteration; 8192 / 32 = 256 iterations)
// -------------------------------------------------------------------------

/// `out = a & b` on AVX2; returns the exact cardinality.
///
/// # Safety
/// The caller guarantees AVX2 and the XSAVE/XCR0.YMM arm are active (a
/// boot-time CPUID decision) and that this runs inside the registered
/// kernel bracket.  `a`, `b` and `out` are the exact 1024-word (8192-byte)
/// buffers; `out` does not alias `a` or `b`.
#[cfg(target_arch = "x86_64")]
pub(crate) unsafe fn bitmap_and_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    // SAFETY: the fn contract — the asm reads exactly 8192 bytes from each
    // of a and b and writes exactly 8192 bytes to out (256 x 32-byte
    // iterations; the pointers advance only within those buffers).
    // `vzeroupper` clears the upper halves before returning.  clobber_abi
    // names the vector and caller-saved GPR clobbers.
    unsafe {
        core::arch::asm!(
            "3:",
            "vmovdqu ({ap}), %ymm0",
            "vmovdqu ({bp}), %ymm1",
            "vpand %ymm1, %ymm0, %ymm0",
            "vmovdqu %ymm0, ({op})",
            "add $32, {ap}",
            "add $32, {bp}",
            "add $32, {op}",
            "dec {n}",
            "jnz 3b",
            "vzeroupper",
            ap = inout(reg) a.as_ptr() => _,
            bp = inout(reg) b.as_ptr() => _,
            op = inout(reg) out.as_mut_ptr() => _,
            n = inout(reg) (BITMAP_WORDS * 8 / 32) => _,
            out("ymm0") _,
            out("ymm1") _,
            options(att_syntax, nostack),
        );
    }
    out.iter().fold(0u32, |acc, &word| acc + word.count_ones())
}

/// `out = a | b` on AVX2; returns the exact cardinality.
///
/// # Safety
/// As [`bitmap_and_simd`].
#[cfg(target_arch = "x86_64")]
pub(crate) unsafe fn bitmap_or_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    // SAFETY: as bitmap_and_simd; only the vector op differs.
    unsafe {
        core::arch::asm!(
            "3:",
            "vmovdqu ({ap}), %ymm0",
            "vmovdqu ({bp}), %ymm1",
            "vpor %ymm1, %ymm0, %ymm0",
            "vmovdqu %ymm0, ({op})",
            "add $32, {ap}",
            "add $32, {bp}",
            "add $32, {op}",
            "dec {n}",
            "jnz 3b",
            "vzeroupper",
            ap = inout(reg) a.as_ptr() => _,
            bp = inout(reg) b.as_ptr() => _,
            op = inout(reg) out.as_mut_ptr() => _,
            n = inout(reg) (BITMAP_WORDS * 8 / 32) => _,
            out("ymm0") _,
            out("ymm1") _,
            options(att_syntax, nostack),
        );
    }
    out.iter().fold(0u32, |acc, &word| acc + word.count_ones())
}

/// `out = a & !b` on AVX2; returns the exact cardinality.
///
/// # Safety
/// As [`bitmap_and_simd`].
#[cfg(target_arch = "x86_64")]
pub(crate) unsafe fn bitmap_andnot_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    // SAFETY: as bitmap_and_simd.  AT&T reverses the Intel operands:
    // `vpandn %ymm0, %ymm1, %ymm0` computes NOT(ymm1=b) AND ymm0=a, i.e.
    // a & !b (the plan's operand-order warning).
    unsafe {
        core::arch::asm!(
            "3:",
            "vmovdqu ({ap}), %ymm0",
            "vmovdqu ({bp}), %ymm1",
            "vpandn %ymm0, %ymm1, %ymm0",
            "vmovdqu %ymm0, ({op})",
            "add $32, {ap}",
            "add $32, {bp}",
            "add $32, {op}",
            "dec {n}",
            "jnz 3b",
            "vzeroupper",
            ap = inout(reg) a.as_ptr() => _,
            bp = inout(reg) b.as_ptr() => _,
            op = inout(reg) out.as_mut_ptr() => _,
            n = inout(reg) (BITMAP_WORDS * 8 / 32) => _,
            out("ymm0") _,
            out("ymm1") _,
            options(att_syntax, nostack),
        );
    }
    out.iter().fold(0u32, |acc, &word| acc + word.count_ones())
}

// -------------------------------------------------------------------------
// aarch64 NEON (128 bits = 2 u64 words per iteration; 8192 / 16 = 512 iterations)
// -------------------------------------------------------------------------

/// `out = a & b` on NEON; returns the exact cardinality (`cnt` + `uaddlp`
/// + `addv`, accumulated in four u32 lanes so no 8-bit lane overflows).
///
/// # Safety
/// NEON is mandatory on ARMv8 and enabled at EL1 by CPACR_EL1.FPEN.  The
/// caller guarantees this runs inside the registered kernel bracket and
/// that `a`, `b` and `out` are the exact 1024-word buffers with `out` not
/// aliasing `a` or `b`.
#[cfg(target_arch = "aarch64")]
pub(crate) unsafe fn bitmap_and_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let card: u32;
    // SAFETY: the fn contract — 512 iterations x 16 bytes read from each of
    // a and b and written to out (8192 bytes each; the post-index increments
    // stay inside the buffers).  cnt/uaddlp reduce 16 per-byte popcounts to
    // four u32 lane sums; addv folds them.  clobber_abi names v0-v7 (all we
    // use) and the caller-saved GPRs.
    unsafe {
        core::arch::asm!(
            ".arch_extension simd",
            "movi v3.4s, #0",
            "3:",
            "ld1 {{v0.2d}}, [{ap}], #16",
            "ld1 {{v1.2d}}, [{bp}], #16",
            "and v2.16b, v0.16b, v1.16b",
            "st1 {{v2.2d}}, [{op}], #16",
            "cnt v2.16b, v2.16b",
            "uaddlp v2.8h, v2.16b",
            "uaddlp v2.4s, v2.8h",
            "add v3.4s, v3.4s, v2.4s",
            "subs {n}, {n}, #1",
            "b.ne 3b",
            "addv s3, v3.4s",
            "fmov {card:w}, s3",
            ap = inout(reg) a.as_ptr() => _,
            bp = inout(reg) b.as_ptr() => _,
            op = inout(reg) out.as_mut_ptr() => _,
            n = inout(reg) (BITMAP_WORDS / 2) => _,
            card = out(reg) card,
            out("v0") _,
            out("v1") _,
            out("v2") _,
            out("v3") _,
            options(nostack),
        );
    }
    card
}

/// `out = a | b` on NEON; returns the exact cardinality.
///
/// # Safety
/// As [`bitmap_and_simd`].
#[cfg(target_arch = "aarch64")]
pub(crate) unsafe fn bitmap_or_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let card: u32;
    // SAFETY: as bitmap_and_simd; only the vector op differs.
    unsafe {
        core::arch::asm!(
            ".arch_extension simd",
            "movi v3.4s, #0",
            "3:",
            "ld1 {{v0.2d}}, [{ap}], #16",
            "ld1 {{v1.2d}}, [{bp}], #16",
            "orr v2.16b, v0.16b, v1.16b",
            "st1 {{v2.2d}}, [{op}], #16",
            "cnt v2.16b, v2.16b",
            "uaddlp v2.8h, v2.16b",
            "uaddlp v2.4s, v2.8h",
            "add v3.4s, v3.4s, v2.4s",
            "subs {n}, {n}, #1",
            "b.ne 3b",
            "addv s3, v3.4s",
            "fmov {card:w}, s3",
            ap = inout(reg) a.as_ptr() => _,
            bp = inout(reg) b.as_ptr() => _,
            op = inout(reg) out.as_mut_ptr() => _,
            n = inout(reg) (BITMAP_WORDS / 2) => _,
            card = out(reg) card,
            out("v0") _,
            out("v1") _,
            out("v2") _,
            out("v3") _,
            options(nostack),
        );
    }
    card
}

/// `out = a & !b` on NEON (`bic`, which is AND NOT); returns the exact
/// cardinality.
///
/// # Safety
/// As [`bitmap_and_simd`].
#[cfg(target_arch = "aarch64")]
pub(crate) unsafe fn bitmap_andnot_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let card: u32;
    // SAFETY: as bitmap_and_simd; `bic v2, v0, v1` is v0 AND NOT v1.
    unsafe {
        core::arch::asm!(
            ".arch_extension simd",
            "movi v3.4s, #0",
            "3:",
            "ld1 {{v0.2d}}, [{ap}], #16",
            "ld1 {{v1.2d}}, [{bp}], #16",
            "bic v2.16b, v0.16b, v1.16b",
            "st1 {{v2.2d}}, [{op}], #16",
            "cnt v2.16b, v2.16b",
            "uaddlp v2.8h, v2.16b",
            "uaddlp v2.4s, v2.8h",
            "add v3.4s, v3.4s, v2.4s",
            "subs {n}, {n}, #1",
            "b.ne 3b",
            "addv s3, v3.4s",
            "fmov {card:w}, s3",
            ap = inout(reg) a.as_ptr() => _,
            bp = inout(reg) b.as_ptr() => _,
            op = inout(reg) out.as_mut_ptr() => _,
            n = inout(reg) (BITMAP_WORDS / 2) => _,
            card = out(reg) card,
            out("v0") _,
            out("v1") _,
            out("v2") _,
            out("v3") _,
            options(nostack),
        );
    }
    card
}

// -------------------------------------------------------------------------
// Other targets: no vector path, so the "SIMD" kernels are the scalar loops.
// -------------------------------------------------------------------------

/// `out = a & b` scalar fallback (non-AVX2/NEON targets).
///
/// # Safety
/// No preconditions: the body is the scalar reference.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(crate) unsafe fn bitmap_and_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    crate::setops::bitmap_and_scalar(a, b, out)
}

/// `out = a | b` scalar fallback (non-AVX2/NEON targets).
///
/// # Safety
/// No preconditions: the body is the scalar reference.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(crate) unsafe fn bitmap_or_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    crate::setops::bitmap_or_scalar(a, b, out)
}

/// `out = a & !b` scalar fallback (non-AVX2/NEON targets).
///
/// # Safety
/// No preconditions: the body is the scalar reference.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(crate) unsafe fn bitmap_andnot_simd(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    crate::setops::bitmap_andnot_scalar(a, b, out)
}

// -------------------------------------------------------------------------
// Host-test entry points: the SIMD kernel without the boot-time flag, so
// the scalar/vector equivalence pins never race on the process-global flag.
// -------------------------------------------------------------------------

/// Host-test: run the AND kernel directly (the caller has verified the CPU
/// feature).  Not compiled into a kernel.
#[cfg(test)]
pub(crate) fn and_direct(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let mut card = 0u32;
    {
        let written = &mut card;
        let mut f = || {
            // SAFETY: host test only; the caller gates on the CPU feature
            // (NEON is mandatory on aarch64 hosts, AVX2 is feature-detected
            // on x86_64 hosts), and no kernel bracket is registered on the
            // host so `run_guarded` calls this directly.
            *written = unsafe { bitmap_and_simd(a, b, out) };
        };
        run_guarded(&mut f);
    }
    card
}

/// Host-test: run the OR kernel directly.  See [`and_direct`].
#[cfg(test)]
pub(crate) fn or_direct(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let mut card = 0u32;
    {
        let written = &mut card;
        let mut f = || {
            // SAFETY: as and_direct.
            *written = unsafe { bitmap_or_simd(a, b, out) };
        };
        run_guarded(&mut f);
    }
    card
}

/// Host-test: run the ANDNOT kernel directly.  See [`and_direct`].
#[cfg(test)]
pub(crate) fn andnot_direct(
    a: &[u64; BITMAP_WORDS],
    b: &[u64; BITMAP_WORDS],
    out: &mut [u64; BITMAP_WORDS],
) -> u32 {
    let mut card = 0u32;
    {
        let written = &mut card;
        let mut f = || {
            // SAFETY: as and_direct.
            *written = unsafe { bitmap_andnot_simd(a, b, out) };
        };
        run_guarded(&mut f);
    }
    card
}
