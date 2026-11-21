//! User FP/SIMD context: q0-q31, FPCR and FPSR.
//!
//! The kernel itself is built without FP/SIMD (lib/aarch64-unknown-none-elf.json:
//! the softfloat ABI with -neon,-fp-armv8 — no compiled kernel code names
//! a vector register), so no kernel path touches these registers.  A
//! user proc's FP/SIMD state therefore stays live in the register file
//! across every trap, syscall and IRQ it takes, and only a change of the
//! proc on the CPU needs a save and a restore.  The invariant:
//!
//! - while a proc is on the CPU — from its install (proc.rs:
//!   activate_first, pick_next, reschedule) until it leaves (sched,
//!   reschedule, or exit) — its user FP/SIMD state IS the live register
//!   file and its `Proc::fpsave` is stale;
//! - every other proc's state is in its `fpsave`.
//!
//! So installing a proc restores its `fpsave`, leaving saves into it (an
//! exiting proc's state is simply dropped), rfork copies the parent's
//! live registers into the child's `fpsave` (the parent is on the CPU in
//! its own syscall), and exec zeroes the state: `fpsave`, plus the live
//! file when the exec'ing proc is the one on the CPU.
//!
//! Eager, not lazy (no CPACR_EL1 first-use trap): with the kernel FP-free
//! the per-trap cost is already zero, and the 528-byte save + restore runs
//! only at a switch — next to vm::switch's TTBR0 write and full TLB
//! invalidate, which dwarf it.  A lazy owner-tracking scheme would add a
//! trap path, a per-proc "used FP" bit and stale-owner bookkeeping across
//! exit for no measurable gain.  This is the 9front arm64 shape (an
//! FP-free kernel with procsave/procrestore) and the x86_64 twin's
//! SSE-free kernel.
//!
//! The asm text is shared by the kernel paths and the host round-trip
//! test (the macros below), so the test exercises the exact instructions
//! the scheduler runs.  `.arch_extension` re-enables the FP/SIMD
//! mnemonics for these blocks only: the kernel target disables them.

/// One proc's saved user FP/SIMD state.  The asm below hardcodes the
/// layout: q0..q31 at 0..512 (16 bytes each, low half first), FPCR at
/// 512, FPSR at 520; 16-byte alignment for the q-register pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct FpState {
    pub q: [u128; 32],
    pub fpcr: u64,
    pub fpsr: u64,
}

const _: () = assert!(core::mem::offset_of!(FpState, q) == 0);
const _: () = assert!(core::mem::offset_of!(FpState, fpcr) == 512);
const _: () = assert!(core::mem::offset_of!(FpState, fpsr) == 520);
const _: () = assert!(core::mem::size_of::<FpState>() == 528);
const _: () = assert!(core::mem::align_of::<FpState>() == 16);

impl FpState {
    /// The state a new image starts with: every vector register, FPCR
    /// (round to nearest, no traps, no flush-to-zero) and FPSR clear.
    pub const ZERO: FpState = FpState { q: [0; 32], fpcr: 0, fpsr: 0 };
}

/// Live registers → `[{dst}]` (the FpState layout).  x16 is the scratch:
/// a literal register, because clobber_abi (restore) demands explicit
/// outputs, and an `out` operand never shares a register with an input.
macro_rules! fp_save_asm {
    () => {
        concat!(
            ".arch_extension fp\n",
            ".arch_extension simd\n",
            "stp q0, q1, [{dst}, #0]\n",
            "stp q2, q3, [{dst}, #32]\n",
            "stp q4, q5, [{dst}, #64]\n",
            "stp q6, q7, [{dst}, #96]\n",
            "stp q8, q9, [{dst}, #128]\n",
            "stp q10, q11, [{dst}, #160]\n",
            "stp q12, q13, [{dst}, #192]\n",
            "stp q14, q15, [{dst}, #224]\n",
            "stp q16, q17, [{dst}, #256]\n",
            "stp q18, q19, [{dst}, #288]\n",
            "stp q20, q21, [{dst}, #320]\n",
            "stp q22, q23, [{dst}, #352]\n",
            "stp q24, q25, [{dst}, #384]\n",
            "stp q26, q27, [{dst}, #416]\n",
            "stp q28, q29, [{dst}, #448]\n",
            "stp q30, q31, [{dst}, #480]\n",
            "mrs x16, fpcr\n",
            "str x16, [{dst}, #512]\n",
            "mrs x16, fpsr\n",
            "str x16, [{dst}, #520]\n",
        )
    };
}

/// `[{src}]` (the FpState layout) → live registers; x16 is the scratch.
macro_rules! fp_restore_asm {
    () => {
        concat!(
            ".arch_extension fp\n",
            ".arch_extension simd\n",
            "ldp q0, q1, [{src}, #0]\n",
            "ldp q2, q3, [{src}, #32]\n",
            "ldp q4, q5, [{src}, #64]\n",
            "ldp q6, q7, [{src}, #96]\n",
            "ldp q8, q9, [{src}, #128]\n",
            "ldp q10, q11, [{src}, #160]\n",
            "ldp q12, q13, [{src}, #192]\n",
            "ldp q14, q15, [{src}, #224]\n",
            "ldp q16, q17, [{src}, #256]\n",
            "ldp q18, q19, [{src}, #288]\n",
            "ldp q20, q21, [{src}, #320]\n",
            "ldp q22, q23, [{src}, #352]\n",
            "ldp q24, q25, [{src}, #384]\n",
            "ldp q26, q27, [{src}, #416]\n",
            "ldp q28, q29, [{src}, #448]\n",
            "ldp q30, q31, [{src}, #480]\n",
            "ldr x16, [{src}, #512]\n",
            "msr fpcr, x16\n",
            "ldr x16, [{src}, #520]\n",
            "msr fpsr, x16\n",
        )
    };
}

/// Store the live FP/SIMD registers into `dst`.
pub fn save(dst: &mut FpState) {
    // SAFETY: dst is a live, exclusively borrowed, 16-byte-aligned
    // FpState; every store lands inside its 528 bytes (the layout
    // asserts above).  The asm reads FP/SIMD registers and writes only
    // *dst and the declared scratch.  CPACR_EL1.FPEN leaves FP/SIMD
    // untrapped at EL1 (l.S, pre_mmu/vminit.rs).
    unsafe {
        core::arch::asm!(
            fp_save_asm!(),
            dst = in(reg) dst as *mut FpState,
            out("x16") _,
            options(nostack, preserves_flags),
        );
    }
}

/// Load `src` into the live FP/SIMD registers.
pub fn restore(src: &FpState) {
    // SAFETY: src is a live, 16-byte-aligned FpState; every load reads
    // inside its 528 bytes.  The asm writes only FP/SIMD registers and
    // the declared scratch.  The kernel target has no vector registers
    // to clobber (no kernel code names one); host builds (the tests) do,
    // and clobber_abi("C") names them there.
    unsafe {
        core::arch::asm!(
            fp_restore_asm!(),
            src = in(reg) src as *const FpState,
            out("x16") _,
            clobber_abi("C"),
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// Zero the live FP/SIMD registers, FPCR and FPSR (the exec reset).
pub fn reset() {
    restore(&FpState::ZERO);
}

/// Restore `src` and save into `dst` in ONE asm block — nothing may run
/// between (a host build's own codegen uses the vector registers), so
/// the round trip measures exactly the scheduler's instructions.
#[cfg(test)]
fn restore_then_save(src: &FpState, dst: &mut FpState) {
    // SAFETY: save's and restore's contracts, combined: both pointers
    // name live, aligned FpStates; dst is exclusively borrowed.
    unsafe {
        core::arch::asm!(
            fp_restore_asm!(),
            fp_save_asm!(),
            src = in(reg) src as *const FpState,
            dst = in(reg) dst as *mut FpState,
            out("x16") _,
            clobber_abi("C"),
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A distinct value in every byte lane of every register, FPCR with
    /// RMode=RP, FZ and DN set (all architected and implemented on the
    /// hosts the tests run on), FPSR with QC and four cumulative flags.
    fn pattern(seed: u64) -> FpState {
        let mut st = FpState::ZERO;
        let mut x = seed ^ 0x9e37_79b9_7f4a_7c15;
        for (i, q) in st.q.iter_mut().enumerate() {
            x = x.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(0x1405_7b7e_f767_814f);
            *q = (u128::from(x) << 64) | u128::from(x.rotate_left(17) ^ i as u64);
        }
        st.fpcr = (1 << 22) | (1 << 24) | (1 << 25);
        st.fpsr = (1 << 27) | 0b1_0111;
        st
    }

    #[test]
    fn restore_then_save_round_trips_every_register_and_both_control_words() {
        let want = pattern(1);
        let mut got = FpState::ZERO;
        restore_then_save(&want, &mut got);
        assert_eq!(got, want);
        // and a second, different state over the first (no lane sticks)
        let want2 = pattern(2);
        restore_then_save(&want2, &mut got);
        reset(); // leave this test thread's FPCR at its default
        assert_eq!(got, want2);
    }

    /// The whole scheme rests on the kernel never touching these
    /// registers between a proc's install and its leave: the kernel
    /// target must keep FP/SIMD out of compiled code (a `+neon` kernel
    /// zeroes user q0 inside ordinary syscalls — fpprobe's syscall leg).
    #[test]
    fn kernel_target_spec_keeps_fp_simd_out_of_kernel_code() {
        let spec = include_str!("../../lib/aarch64-unknown-none-elf.json");
        assert!(
            spec.contains("\"abi\": \"softfloat\""),
            "kernel target must use the softfloat ABI"
        );
        let features = spec.lines().find(|l| l.contains("\"features\"")).expect("features line");
        for f in ["-neon", "-fp-armv8"] {
            assert!(features.contains(f), "kernel target features must carry {f}: {features}");
        }
        for f in ["+neon", "+fp-armv8"] {
            assert!(!features.contains(f), "kernel target features must not carry {f}: {features}");
        }
    }

    #[test]
    fn zero_round_trips() {
        let mut got = pattern(3);
        restore_then_save(&FpState::ZERO, &mut got);
        assert_eq!(got, FpState::ZERO);
    }

    #[test]
    fn save_reads_the_control_words_restore_wrote() {
        // save/restore as separate calls: only the control words are
        // stable across the call boundary in a host test (the vector
        // registers are the host compiler's to use between the calls).
        let want = pattern(4);
        restore(&want);
        let mut got = FpState::ZERO;
        save(&mut got);
        let (fpcr, fpsr) = (got.fpcr, got.fpsr);
        reset();
        save(&mut got);
        assert_eq!((fpcr, fpsr), (want.fpcr, want.fpsr));
        assert_eq!((got.fpcr, got.fpsr), (0, 0));
    }
}
