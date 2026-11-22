//! User FP/SIMD context (q0-q31, FPCR, FPSR) and the kernel-SIMD API.
//!
//! # User state
//!
//! The kernel is built without FP/SIMD (lib/aarch64-unknown-none-elf.json:
//! the softfloat ABI with -neon,-fp-armv8 — no compiled kernel code names
//! a vector register), so a user proc's FP/SIMD state stays live in the
//! register file across every trap, syscall and IRQ it takes, and only a
//! change of the proc on the CPU needs a save and a restore.  The
//! invariant, kept by `LIVE_OWNER` (in `percpu`):
//!
//! - the registers hold the user state of exactly the `Proc::fpsave` slot
//!   `LIVE_OWNER` (in `percpu`) names (null: nobody's — the registers are scratch);
//! - every other proc's state is in its own `fpsave`.
//!
//! proc.rs installs a proc's slot when it goes on the CPU ([`install`]:
//! activate_first, pick_next, reschedule), parks the registers into it
//! when it leaves ([`leave`]: sched, reschedule) and drops them at exit
//! ([`discard`]); rfork copies the parent's current state into the child
//! ([`snapshot`]); exec ([`replace`] with [`FpState::ZERO`]) and note
//! delivery / noted (`Proc::note_fpsave`, [`snapshot`] then [`replace`],
//! the 9front notify/noted shape) rewrite it.
//!
//! Eager, not lazy (no CPACR_EL1 first-use trap): with the kernel FP-free
//! the per-trap cost is already zero, and the 528-byte save + restore runs
//! only at a switch — next to vm::switch's TTBR0 write and full TLB
//! invalidate, which dwarf it.  This is the 9front arm64 shape (an FP-free
//! kernel with procsave/procrestore), and the reference the x86_64 port
//! follows (x86_64/src/fpu.rs).
//!
//! # Kernel SIMD
//!
//! Kernel code may use the vector unit only inside a section opened by
//! [`kernel_begin`] (a [`KernelSimd`] guard; the section ends when the
//! guard drops) or [`with_simd`] — the Linux kernel_neon_begin/end shape:
//!
//! - begin parks the owner's live user state into its `fpsave` (nobody
//!   owns the registers during the section); end restores it and makes
//!   the owner live again, or, with no owner, zeroes the registers so no
//!   kernel data lingers in them;
//! - allowed in any kernel context — syscall, trap, IRQ handler — but only
//!   with IRQs MASKED (asserted at begin).  r9 runs all kernel code
//!   IRQ-masked (the sched loop's idle wfi window is the one exception and
//!   opens no section), so a section can never be interrupted, preempted
//!   or entered again from an IRQ — there is no nesting to save for, and
//!   no per-CPU nesting area;
//! - no sleeping or switching inside: [`install`]/[`leave`] (the
//!   scheduler's only FP hooks) panic by name inside a section, so a
//!   sched(), reschedule() or pick_next() under a live guard is loud;
//! - not reentrant: a nested [`kernel_begin`] panics by name;
//! - vector instructions live in inline asm that enables them locally
//!   (`.arch_extension simd`) and takes `&KernelSimd` as its proof of a
//!   live section ([`xor_fold16`] is the example).  Not
//!   `#[target_feature(enable = "neon")]`: on the softfloat target that is
//!   the ABI-unsound feature rustc warns about (rust-lang/rust#134375).
//!
//! The asm text is shared by the kernel paths and the host round-trip
//! test (the macros below), so the test exercises the exact instructions
//! the scheduler runs.

use core::marker::PhantomData;
use core::ptr;

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

/// The per-CPU FP bookkeeping: which `fpsave` slot's user state is
/// live in the registers (LIVE_OWNER; null: none) and whether a kernel
/// SIMD section is open (IN_KERNEL_SIMD).  The kernel has one CPU and
/// touches these only from IRQ-masked kernel context; host tests give every
/// test thread its own copy (a thread is a "CPU" there), so tests that
/// run in parallel never see each other's sections or owners.
#[cfg(not(test))]
mod percpu {
    use super::FpState;
    use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

    static LIVE_OWNER: AtomicPtr<FpState> = AtomicPtr::new(core::ptr::null_mut());
    static IN_KERNEL_SIMD: AtomicBool = AtomicBool::new(false);

    pub fn owner() -> *mut FpState {
        LIVE_OWNER.load(Ordering::Relaxed)
    }
    pub fn set_owner(p: *mut FpState) {
        LIVE_OWNER.store(p, Ordering::Relaxed)
    }
    pub fn in_section() -> bool {
        IN_KERNEL_SIMD.load(Ordering::Relaxed)
    }
    /// Set the flag; returns its previous value.
    pub fn set_in_section(v: bool) -> bool {
        IN_KERNEL_SIMD.swap(v, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod percpu {
    use super::FpState;
    use std::cell::Cell;

    std::thread_local! {
        static LIVE_OWNER: Cell<*mut FpState> = const { Cell::new(core::ptr::null_mut()) };
        static IN_KERNEL_SIMD: Cell<bool> = const { Cell::new(false) };
    }

    pub fn owner() -> *mut FpState {
        LIVE_OWNER.with(|c| c.get())
    }
    pub fn set_owner(p: *mut FpState) {
        LIVE_OWNER.with(|c| c.set(p))
    }
    pub fn in_section() -> bool {
        IN_KERNEL_SIMD.with(|c| c.get())
    }
    pub fn set_in_section(v: bool) -> bool {
        IN_KERNEL_SIMD.with(|c| c.replace(v))
    }
}

/// Store the live FP/SIMD registers into `*dst`.
///
/// # Safety
/// `dst` names a live, writable, 16-byte-aligned FpState that no
/// reference aliases for the duration of the call.
unsafe fn save_raw(dst: *mut FpState) {
    // SAFETY: the caller's contract — every store lands inside the 528
    // bytes at dst (the layout asserts above).  The asm reads FP/SIMD
    // registers and writes only *dst and the declared scratch.
    // CPACR_EL1.FPEN leaves FP/SIMD untrapped at EL1 (l.S,
    // pre_mmu/vminit.rs).
    unsafe {
        core::arch::asm!(
            fp_save_asm!(),
            dst = in(reg) dst,
            out("x16") _,
            options(nostack, preserves_flags),
        );
    }
}

/// Load `*src` into the live FP/SIMD registers.
///
/// # Safety
/// `src` names a live, 16-byte-aligned FpState.
unsafe fn restore_raw(src: *const FpState) {
    // SAFETY: the caller's contract — every load reads inside the 528
    // bytes at src.  The asm writes only FP/SIMD registers and the
    // declared scratch.  The kernel target has no vector registers to
    // clobber (no kernel code names one); host builds (the tests) do,
    // and clobber_abi("C") names them there.
    unsafe {
        core::arch::asm!(
            fp_restore_asm!(),
            src = in(reg) src,
            out("x16") _,
            clobber_abi("C"),
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// Store the live FP/SIMD registers into `dst`.
pub fn save(dst: &mut FpState) {
    // SAFETY: dst is a live, exclusively borrowed, aligned FpState.
    unsafe { save_raw(dst) }
}

/// Load `src` into the live FP/SIMD registers.
pub fn restore(src: &FpState) {
    // SAFETY: src is a live, aligned FpState.
    unsafe { restore_raw(src) }
}

/// Zero the live FP/SIMD registers, FPCR and FPSR.
pub fn reset() {
    restore(&FpState::ZERO);
}

/// The scheduler and the user-state helpers never run inside a kernel
/// SIMD section (no sleeping or switching under a live guard).
fn assert_outside_kernel_simd(site: &str) {
    assert!(
        !percpu::in_section(),
        "fpu: {site} inside a kernel SIMD section (no sleeping or switching under a KernelSimd guard)"
    );
}

/// A proc goes on the CPU: its saved user state goes live and its slot
/// becomes the owner.
pub fn install(slot: &mut FpState) {
    assert_outside_kernel_simd("install");
    restore(slot);
    percpu::set_owner(slot);
}

/// The proc owning `slot` leaves the CPU: park the live registers in its
/// slot (if they are its state) and leave the registers unowned.
pub fn leave(slot: &mut FpState) {
    assert_outside_kernel_simd("leave");
    let p: *mut FpState = slot;
    if percpu::owner() == p {
        save(slot);
        percpu::set_owner(ptr::null_mut());
    }
}

/// `slot`'s proc exited: its live state (if any) is dropped unsaved.
pub fn discard(slot: &mut FpState) {
    let p: *mut FpState = slot;
    if percpu::owner() == p {
        percpu::set_owner(ptr::null_mut());
    }
}

/// `slot`'s current user state: the live registers when it owns them,
/// otherwise the slot itself.
pub fn snapshot(slot: &FpState) -> FpState {
    assert_outside_kernel_simd("snapshot");
    if ptr::eq(percpu::owner(), slot) {
        let mut st = FpState::ZERO;
        save(&mut st);
        st
    } else {
        *slot
    }
}

/// Make `new` `slot`'s user state — the live registers too when `slot`
/// owns them.
pub fn replace(slot: &mut FpState, new: &FpState) {
    assert_outside_kernel_simd("replace");
    *slot = *new;
    let p: *mut FpState = slot;
    if percpu::owner() == p {
        restore(slot);
    }
}

/// True when IRQs are masked (PSTATE.I).
#[cfg(not(test))]
fn irqs_masked() -> bool {
    let daif: u64;
    // SAFETY: a read of PSTATE.DAIF — no memory operands, always
    // executable at EL1.
    unsafe {
        core::arch::asm!("mrs {}, daif", out(reg) daif, options(nomem, nostack, preserves_flags))
    };
    daif & (1 << 7) != 0
}

/// A live kernel SIMD section (see the module doc for the rules).  Ends
/// when dropped.  Not Send: a section belongs to the CPU that opened it.
pub struct KernelSimd {
    /// The slot whose user state begin parked (null: the registers were
    /// unowned).
    parked: *mut FpState,
    _not_send: PhantomData<*mut ()>,
}

/// Open a kernel SIMD section.  Panics by name if one is already open or
/// if IRQs are unmasked.
pub fn kernel_begin() -> KernelSimd {
    if percpu::set_in_section(true) {
        panic!(
            "fpu: nested kernel_begin (a KernelSimd section is already open; sections are not reentrant)"
        );
    }
    #[cfg(not(test))]
    assert!(
        irqs_masked(),
        "fpu: kernel_begin with IRQs unmasked (a section must not be interruptible)"
    );
    let owner = percpu::owner();
    percpu::set_owner(ptr::null_mut());
    if !owner.is_null() {
        // SAFETY: owner names a live PROC_TABLE-resident (or boot
        // self-test) fpsave slot — install recorded it and leave/discard
        // clear it before the slot's proc stops existing; nothing else
        // touches the slot while IRQs are masked on the single CPU.
        unsafe { save_raw(owner) };
    }
    KernelSimd { parked: owner, _not_send: PhantomData }
}

impl Drop for KernelSimd {
    fn drop(&mut self) {
        if self.parked.is_null() {
            // Nothing to give back: scrub, so no kernel data lingers in
            // the registers for whoever is installed next.
            reset();
        } else {
            // SAFETY: kernel_begin's contract on the parked slot.
            unsafe { restore_raw(self.parked) };
            percpu::set_owner(self.parked);
        }
        percpu::set_in_section(false);
    }
}

/// Run `f` inside a kernel SIMD section (the scoped form of
/// [`kernel_begin`]); the section ends when `f` returns.
pub fn with_simd<R>(f: impl FnOnce(&KernelSimd) -> R) -> R {
    let simd = kernel_begin();
    f(&simd)
}

/// The example kernel SIMD routine: the XOR of every 16-byte block of
/// `data` (its length a multiple of 16), on v0/v1.  Not on any hot path
/// yet — the host tests and the boot self-test are its only callers.
pub fn xor_fold16(_simd: &KernelSimd, data: &[u8]) -> u128 {
    assert!(
        data.len().is_multiple_of(16),
        "xor_fold16: length {} is not a multiple of 16",
        data.len()
    );
    let (lo, hi): (u64, u64);
    // SAFETY: reads exactly data.len() bytes from data (x1 = the block
    // count); writes only v0/v1 and the declared GPRs.  `_simd` proves a
    // live section: the user state is parked and nothing switches until
    // it ends.  The kernel target has no vector registers to clobber;
    // host builds do, and clobber_abi("C") names them.
    unsafe {
        core::arch::asm!(
            ".arch_extension simd",
            "movi v0.2d, #0",
            "cbz x1, 4f",
            "3:",
            "ld1 {{v1.16b}}, [x0], #16",
            "eor v0.16b, v0.16b, v1.16b",
            "subs x1, x1, #1",
            "b.ne 3b",
            "4:",
            "mov x2, v0.d[0]",
            "mov x3, v0.d[1]",
            inout("x0") data.as_ptr() => _,
            inout("x1") data.len() / 16 => _,
            out("x2") lo,
            out("x3") hi,
            clobber_abi("C"),
            options(nostack, readonly),
        );
    }
    (u128::from(hi) << 64) | u128::from(lo)
}

/// [`xor_fold16`]'s scalar reference (each block read little-endian).
pub fn xor_fold16_scalar(data: &[u8]) -> u128 {
    data.chunks_exact(16).fold(0, |acc, b| acc ^ u128::from_le_bytes(b.try_into().unwrap()))
}

/// Restore `src` and save into `dst` in ONE asm block — nothing may run
/// between (a host build's own codegen uses the vector registers), so
/// the round trip measures exactly the scheduler's instructions.
#[cfg(test)]
fn restore_then_save(src: &FpState, dst: &mut FpState) {
    // SAFETY: save_raw's and restore_raw's contracts, combined: both
    // pointers name live, aligned FpStates; dst is exclusively borrowed.
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

/// Boot self-test: a user state survives a kernel SIMD section.  A
/// stand-in slot is installed as if a proc went on the CPU, the live
/// registers then change (as user code would change them), and a section
/// runs [`xor_fold16`] and zeroes every register.  Afterwards the live
/// registers must hold the CHANGED state (begin parked it, end restored
/// it: a begin that failed to park would restore the stale slot, an end
/// that failed to restore would leave zeros), the registers inside the
/// section must have been zero (the section really clobbered them), and
/// the fold must match the scalar reference.  Runs before any process
/// exists; leaves the registers unowned and zeroed.
pub fn selftest() -> Result<u128, &'static str> {
    static SLOT: core::cell::SyncUnsafeCell<FpState> =
        core::cell::SyncUnsafeCell::new(FpState::ZERO);
    let mut a = FpState::ZERO;
    let mut b = FpState::ZERO;
    for i in 0..32 {
        a.q[i] = 0x0123_4567_89ab_cdef_0000_0000_0000_0000 | (i as u128) << 8 | 0xa5;
        b.q[i] = 0xfedc_ba98_7654_3210_0000_0000_0000_0000 | (i as u128) << 8 | 0x5a;
    }
    a.fpcr = 1 << 22; // RMode = RP
    b.fpcr = (2 << 22) | (1 << 25); // RMode = RM, DN
    a.fpsr = 1; // IOC
    b.fpsr = (1 << 27) | (1 << 4); // QC, IXC
    // an LCG byte stream (a structured fill such as i * 37 folds to zero)
    let mut buf = [0u8; 512];
    let mut x: u32 = 0x1234_5678;
    for b in buf.iter_mut() {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        *b = (x >> 16) as u8;
    }

    let slot = SLOT.get();
    // SAFETY: SLOT is this self-test's private stand-in for a proc's
    // fpsave; boot context, single CPU, no other user.
    unsafe {
        *slot = a;
        install(&mut *slot);
    }
    // "User code" changes its registers after the install.
    restore(&b);
    let mut want = FpState::ZERO;
    save(&mut want); // b as the hardware holds it (unimplemented bits read 0)
    let mut inside = FpState::ZERO;
    let sum = with_simd(|simd| {
        let s = xor_fold16(simd, &buf);
        reset();
        save(&mut inside);
        s
    });
    let mut got = FpState::ZERO;
    save(&mut got);
    // SAFETY: as above.
    unsafe { discard(&mut *slot) };
    reset();

    if sum != xor_fold16_scalar(&buf) || sum == 0 {
        return Err("xor_fold16 disagrees with the scalar fold");
    }
    if inside != FpState::ZERO {
        return Err("the section did not clobber the registers");
    }
    if got != want {
        return Err("the user state was not parked and restored across the section");
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The live FPCR — the observable host tests use for ownership moves
    /// (unlike the vector registers, the host compiler never changes it
    /// between calls).
    fn live_fpcr() -> u64 {
        let mut st = FpState::ZERO;
        save(&mut st);
        st.fpcr
    }

    fn with_fpcr(fpcr: u64) -> FpState {
        FpState { fpcr, ..FpState::ZERO }
    }

    const RP: u64 = 1 << 22;
    const RM_DN: u64 = (2 << 22) | (1 << 25);
    const RZ_FZ: u64 = (3 << 22) | (1 << 24);

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

    #[test]
    fn a_section_parks_the_owner_and_gives_the_changed_state_back() {
        let mut slot = with_fpcr(RP);
        install(&mut slot);
        restore(&with_fpcr(RM_DN)); // user code changed its state after install
        {
            let simd = kernel_begin();
            assert!(percpu::owner().is_null(), "begin leaves the registers unowned");
            assert_eq!(slot.fpcr, RM_DN, "begin parked the CHANGED live state in the owner's slot");
            reset(); // the section clobbers everything
            let data = [7u8; 64];
            assert_eq!(xor_fold16(&simd, &data), 0);
            assert_eq!(live_fpcr(), 0);
        }
        assert_eq!(live_fpcr(), RM_DN, "end restored the parked state");
        assert!(ptr::eq(percpu::owner(), &slot), "end made the owner live again");
        leave(&mut slot);
        assert!(percpu::owner().is_null());
        reset();
    }

    #[test]
    fn a_section_without_an_owner_scrubs_the_registers() {
        assert!(percpu::owner().is_null());
        with_simd(|_| restore(&with_fpcr(RZ_FZ))); // kernel data left behind
        assert_eq!(live_fpcr(), 0, "end zeroed the unowned registers");
    }

    #[test]
    #[should_panic(expected = "fpu: nested kernel_begin")]
    fn a_nested_kernel_begin_panics_by_name() {
        let _outer = kernel_begin();
        let _inner = kernel_begin();
    }

    #[test]
    #[should_panic(expected = "fpu: leave inside a kernel SIMD section")]
    fn leaving_the_cpu_inside_a_section_panics_by_name() {
        let mut slot = FpState::ZERO;
        with_simd(|_| leave(&mut slot));
    }

    #[test]
    #[should_panic(expected = "fpu: install inside a kernel SIMD section")]
    fn installing_inside_a_section_panics_by_name() {
        let mut slot = FpState::ZERO;
        with_simd(|_| install(&mut slot));
    }

    #[test]
    fn snapshot_and_replace_follow_ownership() {
        let mut owner = with_fpcr(RP);
        let mut other = with_fpcr(RZ_FZ);
        install(&mut owner);
        restore(&with_fpcr(RM_DN)); // the owner's live state moved on
        assert_eq!(snapshot(&owner).fpcr, RM_DN, "an owner's snapshot is the live state");
        assert_eq!(snapshot(&other).fpcr, RZ_FZ, "a non-owner's snapshot is its slot");
        replace(&mut other, &with_fpcr(RP));
        assert_eq!(
            (other.fpcr, live_fpcr()),
            (RP, RM_DN),
            "replacing a non-owner leaves the live state alone"
        );
        replace(&mut owner, &FpState::ZERO);
        assert_eq!(
            (owner.fpcr, live_fpcr()),
            (0, 0),
            "replacing the owner rewrites the live state"
        );
        discard(&mut owner);
        assert!(percpu::owner().is_null(), "discard drops the owner");
        reset();
    }

    #[test]
    fn leave_parks_only_the_owner() {
        let mut owner = with_fpcr(0);
        let mut stranger = with_fpcr(RZ_FZ);
        install(&mut owner);
        restore(&with_fpcr(RP));
        leave(&mut stranger); // not the owner: no save, owner unchanged
        assert_eq!(stranger.fpcr, RZ_FZ);
        assert!(ptr::eq(percpu::owner(), &owner));
        leave(&mut owner);
        assert_eq!(owner.fpcr, RP, "leave parked the live state in the owner's slot");
        reset();
    }

    #[test]
    fn xor_fold16_matches_the_scalar_fold() {
        let mut data = [0u8; 4096];
        let mut x: u32 = 0x9e37_79b9;
        for b in data.iter_mut() {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            *b = (x >> 16) as u8;
        }
        for len in [0, 16, 32, 48, 512, 4096] {
            let got = with_simd(|simd| xor_fold16(simd, &data[..len]));
            assert_eq!(got, xor_fold16_scalar(&data[..len]), "len {len}");
            assert!(len == 0 || got != 0, "len {len}: a zero fold proves nothing");
        }
        assert_eq!(xor_fold16_scalar(&data[..0]), 0);
    }

    #[test]
    #[should_panic(expected = "not a multiple of 16")]
    fn xor_fold16_rejects_a_ragged_length() {
        with_simd(|simd| xor_fold16(simd, &[0u8; 17]));
    }
}
