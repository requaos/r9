//! User FPU/SSE context (x87, MMX, XMM0-15, MXCSR) and the kernel-SIMD
//! API — the x86_64 port of aarch64/src/fpu.rs, the reference design.
//!
//! # User state
//!
//! The kernel is built without FPU/SSE (lib/x86_64-unknown-none-elf.json:
//! "-mmx,-sse,+soft-float", rustc-abi x86-softfloat — 14-04's rule), so a
//! user proc's FPU/SSE state stays live in the registers across every
//! syscall, trap and interrupt it takes, and only a change of the proc
//! on the CPU needs a save and a restore.  The state is the 512-byte
//! FXSAVE image: CR4.OSXSAVE is never set, so AVX and later state is
//! unusable at CPL3 and FXSAVE covers everything a user can touch.  The
//! invariant, kept by the CPU's owner (`CpuFp`):
//!
//! - the registers hold the user state of exactly the `Proc::fpsave` slot
//!   the CPU's owner (`CpuFp`) names (null: nobody's — the registers are scratch);
//! - every other proc's state is in its own `fpsave`.
//!
//! proc.rs installs a proc's slot when it goes on the CPU ([`install`]:
//! activate_first, pick_next), parks the registers into it when it leaves
//! ([`leave`]: sched, reschedule's park) and drops them at exit
//! ([`discard`]); rfork copies the parent's current state into the child
//! ([`snapshot`]); exec ([`replace`] with [`FpState::DEFAULT`] — the
//! fninit + MXCSR 0x1F80 state with every register zero, so no previous
//! image's XMM or x87 contents leak) and note delivery / noted
//! (`Proc::note_fpsave`, [`snapshot`] then [`replace`], the 9front
//! notify/noted shape) rewrite it.  Eager, not lazy (no CR0.TS #NM trap):
//! with the kernel FPU-free the per-trap cost is already zero, and the
//! FXSAVE + FXRSTOR runs only at a switch, beside the CR3 reload.
//!
//! # Kernel SIMD
//!
//! Kernel code may use SSE only inside a section opened by
//! [`kernel_begin`] (a [`KernelSimd`] guard; the section ends when the
//! guard drops) or [`with_simd`] — the Linux kernel_fpu_begin/end shape:
//!
//! - begin parks the owner's live user state into its `fpsave` (nobody
//!   owns the registers during the section); end restores it and makes
//!   the owner live again, or, with no owner, loads the default state so
//!   no kernel data lingers in the registers;
//! - allowed in any kernel context — syscall, trap, interrupt handler —
//!   but only with interrupts DISABLED (IF=0, asserted at begin).  r9
//!   runs all kernel code IF=0 (the sched loop's sti;hlt;cli idle window
//!   is the one exception and opens no section), so a section can never
//!   be interrupted, preempted or entered again from an interrupt —
//!   there is no nesting to save for, and no per-CPU nesting area;
//! - no sleeping or switching inside: [`install`]/[`leave`] (the
//!   scheduler's only FPU hooks) panic by name inside a section;
//! - not reentrant: a nested [`kernel_begin`] panics by name;
//! - SSE instructions live in inline asm (the x86 assembler takes them
//!   whatever the target features) that takes `&KernelSimd` as its proof
//!   of a live section ([`xor_fold16`] is the example).  Not
//!   `#[target_feature(enable = "sse2")]`: on the x86-softfloat ABI that
//!   is the ABI-unsound feature rustc warns about.

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

/// One proc's saved user FPU/SSE state: the FXSAVE64 image (16-byte
/// aligned, 512 bytes).  Layout (SDM Vol 1, 10.5.1): FCW @0, FSW @2,
/// abridged FTW @4, FOP @6, FIP @8, FDP @16, MXCSR @24, MXCSR_MASK @28,
/// ST0-7/MM0-7 @32 (16-byte slots, 10 bytes used), XMM0-15 @160,
/// reserved/available @416.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct FpState {
    pub fx: [u8; 512],
}

const _: () = assert!(core::mem::size_of::<FpState>() == 512);
const _: () = assert!(core::mem::align_of::<FpState>() == 16);

const FCW: usize = 0;
const MXCSR: usize = 24;

impl FpState {
    /// The state a new image starts with: FCW 0x037F (fninit: every x87
    /// exception masked, 64-bit precision, round to nearest), MXCSR
    /// 0x1F80 (every SSE exception masked, round to nearest), every
    /// x87/MMX/XMM register, FSW and the tag word zero.
    pub const DEFAULT: FpState = {
        let mut fx = [0u8; 512];
        fx[FCW] = 0x7f;
        fx[FCW + 1] = 0x03;
        fx[MXCSR] = 0x80;
        fx[MXCSR + 1] = 0x1f;
        FpState { fx }
    };

    /// The image's MXCSR.
    pub fn mxcsr(&self) -> u32 {
        u32::from_le_bytes([
            self.fx[MXCSR],
            self.fx[MXCSR + 1],
            self.fx[MXCSR + 2],
            self.fx[MXCSR + 3],
        ])
    }

    /// Set the image's MXCSR (bits above 15 must stay clear: FXRSTOR
    /// faults on reserved MXCSR bits).
    pub fn set_mxcsr(&mut self, v: u32) {
        self.fx[MXCSR..MXCSR + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// The user-visible state two images agree on: FCW, FSW, the
    /// abridged tag word, MXCSR, the 80 bits of each ST/MM register and
    /// XMM0-15.  FOP/FIP/FDP are excluded (some CPUs store them only with
    /// an exception pending), as are the reserved and MXCSR_MASK bytes.
    pub fn same_user_state(&self, other: &FpState) -> bool {
        let (a, b) = (&self.fx, &other.fx);
        a[0..5] == b[0..5]
            && a[24..28] == b[24..28]
            && (0..8).all(|i| a[32 + 16 * i..42 + 16 * i] == b[32 + 16 * i..42 + 16 * i])
            && a[160..416] == b[160..416]
    }
}

/// One CPU's FP bookkeeping: which `fpsave` slot's user state is live
/// in this CPU's registers (the owner; null: none) and whether a kernel
/// SIMD section is open on it.  Per CPU because both describe this
/// CPU's register file: a second CPU has its own registers, its own
/// owner and its own sections.  Touched only from IF=0 kernel context
/// on the CPU it describes; atomics so a shared reference is sound.
/// All-zero is the empty state (FromZeros: the Mach that embeds it is
/// zero-constructible).
#[derive(zerocopy::FromZeros)]
pub struct CpuFp {
    owner: AtomicPtr<FpState>,
    in_section: AtomicBool,
}

impl CpuFp {
    pub const fn new() -> CpuFp {
        CpuFp { owner: AtomicPtr::new(core::ptr::null_mut()), in_section: AtomicBool::new(false) }
    }
    fn owner(&self) -> *mut FpState {
        self.owner.load(Ordering::Relaxed)
    }
    fn set_owner(&self, p: *mut FpState) {
        self.owner.store(p, Ordering::Relaxed)
    }
    fn in_section(&self) -> bool {
        self.in_section.load(Ordering::Relaxed)
    }
    /// Set the flag; returns its previous value.
    fn set_in_section(&self, v: bool) -> bool {
        self.in_section.swap(v, Ordering::Relaxed)
    }
}

/// This CPU's bookkeeping: the `fpu` field in this CPU's Mach, read
/// through `%gs:0` (`dat::this_mach()`, SMP-02) — each CPU has its own
/// Mach at its own kernel-half VA (CPU 0's at `dat::KMACH`, each AP's
/// heap-allocated), so the bookkeeping is per-CPU by construction.
#[cfg(not(test))]
fn this_cpu() -> &'static CpuFp {
    // SAFETY-FREE on the read path: this_mach() is a shared &Mach (its
    // own %gs:0 contract), and CpuFp is all atomics, so a shared
    // reference is sound.
    &crate::dat::this_mach().fpu
}

/// Host tests: every test thread is its own "CPU" (one leaked CpuFp per
/// thread), so tests that run in parallel never see each other's owners
/// or sections.
#[cfg(test)]
fn this_cpu() -> &'static CpuFp {
    std::thread_local! {
        static CPU: &'static CpuFp = Box::leak(Box::new(CpuFp::new()));
    }
    CPU.with(|c| *c)
}

/// Store the live FPU/SSE registers into `*dst`.
///
/// # Safety
/// `dst` names a live, writable, 16-byte-aligned FpState that no
/// reference aliases for the duration of the call.
unsafe fn save_raw(dst: *mut FpState) {
    // SAFETY: the caller's contract — FXSAVE64 writes at most the 512
    // bytes at dst (16-byte aligned, as FXSAVE requires).  CR4.OSFXSR is
    // set at boot (l.S), so FXSAVE saves the XMM state too.
    unsafe {
        core::arch::asm!(
            "fxsave64 ({dst})",
            dst = in(reg) dst,
            options(att_syntax, nostack, preserves_flags),
        );
    }
}

/// Load `*src` into the live FPU/SSE registers.
///
/// # Safety
/// `src` names a live, 16-byte-aligned FpState whose MXCSR has no
/// reserved bits set (FXRSTOR raises #GP otherwise).
unsafe fn restore_raw(src: *const FpState) {
    // SAFETY: the caller's contract — FXRSTOR64 reads the 512 bytes at
    // src.  The kernel target has no FPU/SSE registers to clobber (no
    // kernel code names one); host builds (the tests) do, and
    // clobber_abi("C") names them there.
    unsafe {
        core::arch::asm!(
            "fxrstor64 ({src})",
            src = in(reg) src,
            clobber_abi("C"),
            options(att_syntax, nostack, preserves_flags, readonly),
        );
    }
}

/// Store the live FPU/SSE registers into `dst`.
pub fn save(dst: &mut FpState) {
    // SAFETY: dst is a live, exclusively borrowed, aligned FpState.
    unsafe { save_raw(dst) }
}

/// Load `src` into the live FPU/SSE registers.  `src`'s MXCSR must
/// carry no reserved bits (every image here comes from FXSAVE or from
/// [`FpState::DEFAULT`]).
pub fn restore(src: &FpState) {
    assert!(src.mxcsr() >> 16 == 0, "fpu: MXCSR {:#x} has reserved bits set", src.mxcsr());
    // SAFETY: src is a live, aligned FpState with a valid MXCSR.
    unsafe { restore_raw(src) }
}

/// Load the default state (the exec reset) into the live registers.
pub fn reset() {
    restore(&FpState::DEFAULT);
}

/// The scheduler and the user-state helpers never run inside a kernel
/// SIMD section (no sleeping or switching under a live guard).
fn assert_outside_kernel_simd(cpu: &CpuFp, site: &str) {
    assert!(
        !cpu.in_section(),
        "fpu: {site} inside a kernel SIMD section (no sleeping or switching under a KernelSimd guard)"
    );
}

/// A proc goes on the CPU: its saved user state goes live and its slot
/// becomes the owner.
pub fn install(slot: &mut FpState) {
    install_on(this_cpu(), slot)
}

fn install_on(cpu: &CpuFp, slot: &mut FpState) {
    assert_outside_kernel_simd(cpu, "install");
    restore(slot);
    cpu.set_owner(slot);
}

/// The proc owning `slot` leaves the CPU: park the live registers in its
/// slot (if they are its state) and leave the registers unowned.
pub fn leave(slot: &mut FpState) {
    leave_on(this_cpu(), slot)
}

fn leave_on(cpu: &CpuFp, slot: &mut FpState) {
    assert_outside_kernel_simd(cpu, "leave");
    let p: *mut FpState = slot;
    if cpu.owner() == p {
        save(slot);
        cpu.set_owner(ptr::null_mut());
    }
}

/// `slot`'s proc exited: its live state (if any) is dropped unsaved.
pub fn discard(slot: &mut FpState) {
    discard_on(this_cpu(), slot)
}

fn discard_on(cpu: &CpuFp, slot: &mut FpState) {
    let p: *mut FpState = slot;
    if cpu.owner() == p {
        cpu.set_owner(ptr::null_mut());
    }
}

/// `slot`'s current user state: the live registers when it owns them,
/// otherwise the slot itself.
pub fn snapshot(slot: &FpState) -> FpState {
    snapshot_on(this_cpu(), slot)
}

fn snapshot_on(cpu: &CpuFp, slot: &FpState) -> FpState {
    assert_outside_kernel_simd(cpu, "snapshot");
    if ptr::eq(cpu.owner(), slot) {
        let mut st = FpState::DEFAULT;
        save(&mut st);
        st
    } else {
        *slot
    }
}

/// Make `new` `slot`'s user state — the live registers too when `slot`
/// owns them.
pub fn replace(slot: &mut FpState, new: &FpState) {
    replace_on(this_cpu(), slot, new)
}

fn replace_on(cpu: &CpuFp, slot: &mut FpState, new: &FpState) {
    assert_outside_kernel_simd(cpu, "replace");
    *slot = *new;
    let p: *mut FpState = slot;
    if cpu.owner() == p {
        restore(slot);
    }
}

/// True when interrupts are disabled (RFLAGS.IF clear).
#[cfg(not(test))]
fn interrupts_disabled() -> bool {
    let rflags: u64;
    // SAFETY: pushfq/pop reads RFLAGS through one stack slot of this
    // frame; no other memory, no state change.
    unsafe {
        core::arch::asm!("pushfq", "popq {}", out(reg) rflags, options(att_syntax, preserves_flags))
    };
    rflags & (1 << 9) == 0
}

/// A live kernel SIMD section (see the module doc for the rules).  Ends
/// when dropped.  Not Send: a section belongs to the CPU that opened it.
pub struct KernelSimd {
    /// The slot whose user state begin parked (null: the registers were
    /// unowned).
    parked: *mut FpState,
    /// The CPU the section belongs to (its owner is restored there).
    cpu: &'static CpuFp,
    _not_send: PhantomData<*mut ()>,
}

/// Open a kernel SIMD section.  Panics by name if one is already open or
/// if interrupts are enabled.
pub fn kernel_begin() -> KernelSimd {
    kernel_begin_on(this_cpu())
}

/// Initialize THIS CPU's FP bookkeeping (SMP-01: the AP's `ap_main`
/// bring-up).  The register file itself needs no init: the AP's
/// trampoline armed CR4.OSFXSR/OSXMMEXCPT at BSP parity, and the first
/// proc install FXRSTORs its seeded state (`FpState::DEFAULT` for a
/// fresh image), so any reset-state leftovers in the registers are dead
/// before they are read.  What must start clean is this CPU's `CpuFp`
/// bookkeeping — zeroed at the AP Mach's allocation and re-affirmed
/// here by name, so the bring-up contract is explicit (and MEM-18's
/// XSAVE switch has its per-CPU seam).
pub fn init_cpu() {
    let cpu = this_cpu();
    cpu.set_owner(ptr::null_mut());
    cpu.set_in_section(false);
}

fn kernel_begin_on(cpu: &'static CpuFp) -> KernelSimd {
    if cpu.set_in_section(true) {
        panic!(
            "fpu: nested kernel_begin (a KernelSimd section is already open; sections are not reentrant)"
        );
    }
    #[cfg(not(test))]
    assert!(
        interrupts_disabled(),
        "fpu: kernel_begin with interrupts enabled (a section must not be interruptible)"
    );
    let owner = cpu.owner();
    cpu.set_owner(ptr::null_mut());
    if !owner.is_null() {
        // SAFETY: owner names a live PROC_TABLE-resident (or boot
        // self-test) fpsave slot — install recorded it and leave/discard
        // clear it before the slot's proc stops existing; the slot's
        // OWNER is this CPU (CpuFp ownership is per-CPU, SMP-02), and
        // the section runs IF=0 on this CPU, so no other context on this
        // CPU touches the slot mid-section; a sibling CPU's CpuFp names
        // its own register file's owner, never this slot's writer.
        unsafe { save_raw(owner) };
    }
    KernelSimd { parked: owner, cpu, _not_send: PhantomData }
}

impl Drop for KernelSimd {
    fn drop(&mut self) {
        if self.parked.is_null() {
            // Nothing to give back: load the default state, so no kernel
            // data lingers in the registers for whoever is installed next.
            reset();
        } else {
            // SAFETY: kernel_begin's contract on the parked slot (its
            // image came from FXSAVE, so its MXCSR is valid).
            unsafe { restore_raw(self.parked) };
            self.cpu.set_owner(self.parked);
        }
        self.cpu.set_in_section(false);
    }
}

/// Run `f` inside a kernel SIMD section (the scoped form of
/// [`kernel_begin`]); the section ends when `f` returns.
pub fn with_simd<R>(f: impl FnOnce(&KernelSimd) -> R) -> R {
    let simd = kernel_begin();
    f(&simd)
}

/// The example kernel SIMD routine: the XOR of every 16-byte block of
/// `data` (its length a multiple of 16), on XMM0/XMM1 (SSE2).  Not on any
/// hot path yet — the host tests and the boot self-test are its only
/// callers.
pub fn xor_fold16(_simd: &KernelSimd, data: &[u8]) -> u128 {
    assert!(
        data.len().is_multiple_of(16),
        "xor_fold16: length {} is not a multiple of 16",
        data.len()
    );
    let (lo, hi): (u64, u64);
    // SAFETY: reads exactly data.len() bytes from data (rcx = the block
    // count); writes only XMM0/XMM1, rflags and the declared GPRs.
    // `_simd` proves a live section: the user state is parked and nothing
    // switches until it ends.  The kernel target has no SSE registers to
    // clobber; host builds do, and clobber_abi("C") names them.
    unsafe {
        core::arch::asm!(
            "pxor %xmm0, %xmm0",
            "test %rcx, %rcx",
            "jz 4f",
            "3:",
            "movdqu (%rsi), %xmm1",
            "pxor %xmm1, %xmm0",
            "add $16, %rsi",
            "dec %rcx",
            "jnz 3b",
            "4:",
            "movq %xmm0, %rax",
            "pshufd $0xee, %xmm0, %xmm1",
            "movq %xmm1, %rdx",
            inout("rsi") data.as_ptr() => _,
            inout("rcx") data.len() / 16 => _,
            out("rax") lo,
            out("rdx") hi,
            clobber_abi("C"),
            options(att_syntax, nostack, readonly),
        );
    }
    (u128::from(hi) << 64) | u128::from(lo)
}

/// [`xor_fold16`]'s scalar reference (each block read little-endian).
pub fn xor_fold16_scalar(data: &[u8]) -> u128 {
    data.chunks_exact(16).fold(0, |acc, b| acc ^ u128::from_le_bytes(b.try_into().unwrap()))
}

/// A valid FXSAVE image unique to `seed`: FCW with every exception
/// masked, reserved bit 6 set (as FINIT sets it — some implementations
/// force it) and the precision/rounding fields from the seed, MXCSR with
/// every exception masked plus rounding, FZ and sticky flags from the
/// seed (never DAZ, which old CPUs fault on), all eight ST registers
/// tagged valid with distinct 80-bit contents, distinct XMM0-15.
fn image(seed: u64) -> FpState {
    let mut st = FpState::DEFAULT;
    let fcw: u16 = 0x007f | (((seed as u16) & 3) << 8) | ((((seed as u16) >> 2) & 3) << 10);
    st.fx[0..2].copy_from_slice(&fcw.to_le_bytes());
    st.fx[4] = 0xff; // abridged FTW: ST0-7 valid
    let mxcsr: u32 = 0x1f80
        | (((seed as u32) & 3) << 13)
        | ((((seed as u32) >> 2) & 1) << 15)
        | ((seed as u32) & 0x3f);
    st.set_mxcsr(mxcsr);
    let mut x = seed ^ 0x9e37_79b9_7f4a_7c15;
    for i in 0..8 {
        x = x.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(0x1405_7b7e_f767_814f);
        st.fx[32 + 16 * i..40 + 16 * i].copy_from_slice(&x.to_le_bytes());
        st.fx[40 + 16 * i..42 + 16 * i].copy_from_slice(&((x >> 48) as u16 | 1).to_le_bytes());
    }
    for i in 0..16 {
        x = x.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(0x1405_7b7e_f767_814f);
        st.fx[160 + 16 * i..168 + 16 * i].copy_from_slice(&x.to_le_bytes());
        st.fx[168 + 16 * i..176 + 16 * i].copy_from_slice(&x.rotate_left(29).to_le_bytes());
    }
    st
}

/// Boot self-test: a user state survives a kernel SIMD section.  A
/// stand-in slot is installed as if a proc went on the CPU, the live
/// registers then change (as user code would change them), and a section
/// runs [`xor_fold16`] and loads the default state over everything.
/// Afterwards the live registers must hold the CHANGED state (begin
/// parked it, end restored it: a begin that failed to park would restore
/// the stale slot, an end that failed to restore would leave the
/// default), the registers inside the section must have held the default
/// (the section really clobbered them), and the fold must match the
/// scalar reference.  Runs before any process exists; leaves the
/// registers unowned in the default state.
pub fn selftest() -> Result<u128, &'static str> {
    static SLOT: core::cell::SyncUnsafeCell<FpState> =
        core::cell::SyncUnsafeCell::new(FpState::DEFAULT);
    let a = image(1);
    let b = image(6);
    // an LCG byte stream (a structured fill such as i * 37 folds to zero)
    let mut buf = [0u8; 512];
    let mut x: u32 = 0x1234_5678;
    for byte in buf.iter_mut() {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        *byte = (x >> 16) as u8;
    }

    let slot = SLOT.get();
    // SAFETY: SLOT is this self-test's private stand-in for a proc's
    // fpsave; it runs once at boot on CPU 0 BEFORE any AP starts
    // (main() runs the selftest ahead of start_aps, 05-12) — no other
    // CPU or user exists yet.
    unsafe {
        *slot = a;
        install(&mut *slot);
    }
    // "User code" changes its registers after the install.
    restore(&b);
    let mut want = FpState::DEFAULT;
    save(&mut want); // b as the hardware holds it
    let mut inside = FpState::DEFAULT;
    let sum = with_simd(|simd| {
        let s = xor_fold16(simd, &buf);
        reset();
        save(&mut inside);
        s
    });
    let mut got = FpState::DEFAULT;
    save(&mut got);
    // SAFETY: as above.
    unsafe { discard(&mut *slot) };
    reset();

    if sum != xor_fold16_scalar(&buf) || sum == 0 {
        return Err("xor_fold16 disagrees with the scalar fold");
    }
    if !inside.same_user_state(&FpState::DEFAULT) {
        return Err("the section did not clobber the registers");
    }
    if !got.same_user_state(&want) || !want.same_user_state(&b) {
        return Err("the user state was not parked and restored across the section");
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The live MXCSR — the observable host tests use for ownership moves
    /// (unlike the XMM registers, the host compiler never changes it
    /// between calls).
    fn live_mxcsr() -> u32 {
        let mut st = FpState::DEFAULT;
        save(&mut st);
        st.mxcsr()
    }

    fn with_mxcsr(v: u32) -> FpState {
        let mut st = FpState::DEFAULT;
        st.set_mxcsr(v);
        st
    }

    const RU: u32 = 0x1f80 | (2 << 13);
    const RD_FZ: u32 = 0x1f80 | (1 << 13) | (1 << 15);
    const RZ: u32 = 0x1f80 | (3 << 13);
    const DFLT: u32 = 0x1f80;

    /// Restore `src` and save into `dst` in ONE asm block — nothing may
    /// run between (the host build's own codegen uses the SSE
    /// registers), so the round trip measures exactly the scheduler's
    /// instructions.
    fn restore_then_save(src: &FpState, dst: &mut FpState) {
        // SAFETY: save_raw's and restore_raw's contracts, combined.
        unsafe {
            core::arch::asm!(
                "fxrstor64 ({src})",
                "fxsave64 ({dst})",
                src = in(reg) src as *const FpState,
                dst = in(reg) dst as *mut FpState,
                clobber_abi("C"),
                options(att_syntax, nostack, preserves_flags),
            );
        }
    }

    #[test]
    fn restore_then_save_round_trips_the_user_visible_state() {
        for seed in [1, 2, 7] {
            let want = image(seed);
            let mut got = FpState::DEFAULT;
            restore_then_save(&want, &mut got);
            reset();
            assert!(got.same_user_state(&want), "seed {seed}");
        }
    }

    #[test]
    fn default_is_the_fninit_state_with_zeroed_registers() {
        let d = FpState::DEFAULT;
        assert_eq!(&d.fx[0..2], &[0x7f, 0x03]);
        assert_eq!(d.mxcsr(), 0x1f80);
        assert!(d.fx[2..24].iter().all(|&b| b == 0) && d.fx[28..].iter().all(|&b| b == 0));
        let mut got = image(3);
        restore_then_save(&FpState::DEFAULT, &mut got);
        assert!(got.same_user_state(&FpState::DEFAULT));
    }

    /// The whole scheme rests on the kernel never touching these
    /// registers between a proc's install and its leave.
    #[test]
    fn kernel_target_spec_keeps_fpu_and_sse_out_of_kernel_code() {
        let spec = include_str!("../../lib/x86_64-unknown-none-elf.json");
        let features = spec.lines().find(|l| l.contains("\"features\"")).expect("features line");
        for f in ["-mmx", "-sse", "+soft-float"] {
            assert!(features.contains(f), "kernel target features must carry {f}: {features}");
        }
        assert!(spec.contains("\"rustc-abi\": \"x86-softfloat\""));
    }

    #[test]
    fn a_section_parks_the_owner_and_gives_the_changed_state_back() {
        let mut slot = with_mxcsr(RU);
        install(&mut slot);
        restore(&with_mxcsr(RD_FZ)); // user code changed its state after install
        {
            let simd = kernel_begin();
            assert!(this_cpu().owner().is_null(), "begin leaves the registers unowned");
            assert_eq!(
                slot.mxcsr(),
                RD_FZ,
                "begin parked the CHANGED live state in the owner's slot"
            );
            reset();
            assert_eq!(xor_fold16(&simd, &[9u8; 64]), 0);
            assert_eq!(live_mxcsr(), DFLT);
        }
        assert_eq!(live_mxcsr(), RD_FZ, "end restored the parked state");
        assert!(ptr::eq(this_cpu().owner(), &slot), "end made the owner live again");
        leave(&mut slot);
        assert!(this_cpu().owner().is_null());
        reset();
    }

    /// Per-CPU bookkeeping: each CPU's owner and section are its own (a
    /// second CPU has its own register file).  Bookkeeping only — the
    /// host has one register file, so register contents are not compared.
    #[test]
    fn each_cpu_keeps_its_own_owner_and_section() {
        let a: &'static CpuFp = Box::leak(Box::new(CpuFp::new()));
        let b: &'static CpuFp = Box::leak(Box::new(CpuFp::new()));
        let mut sa = FpState::DEFAULT;
        let mut sb = FpState::DEFAULT;
        install_on(a, &mut sa);
        assert!(ptr::eq(a.owner(), &sa) && b.owner().is_null(), "an install touches only its CPU");
        install_on(b, &mut sb);
        assert!(ptr::eq(a.owner(), &sa) && ptr::eq(b.owner(), &sb));
        {
            let _on_b = kernel_begin_on(b);
            assert!(b.in_section() && !a.in_section(), "a section is its CPU's");
            assert!(b.owner().is_null(), "B's section parked B's owner");
            assert!(ptr::eq(a.owner(), &sa), "A's owner is untouched by B's section");
            // A may still switch, and open its own section, while B is in one.
            leave_on(a, &mut sa);
            install_on(a, &mut sa);
            let _on_a = kernel_begin_on(a);
            assert!(a.in_section() && a.owner().is_null());
        }
        assert!(
            ptr::eq(a.owner(), &sa) && ptr::eq(b.owner(), &sb),
            "each section gave back its own CPU's owner"
        );
        assert!(!a.in_section() && !b.in_section());
        leave_on(a, &mut sa);
        leave_on(b, &mut sb);
        assert!(a.owner().is_null() && b.owner().is_null());
        reset();
    }

    #[test]
    fn this_cpu_is_stable() {
        assert!(ptr::eq(this_cpu(), this_cpu()));
    }

    #[test]
    fn a_section_without_an_owner_leaves_the_default_state() {
        assert!(this_cpu().owner().is_null());
        with_simd(|_| restore(&with_mxcsr(RZ))); // kernel data left behind
        assert_eq!(live_mxcsr(), DFLT, "end loaded the default state");
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
        let mut slot = FpState::DEFAULT;
        with_simd(|_| leave(&mut slot));
    }

    #[test]
    #[should_panic(expected = "fpu: install inside a kernel SIMD section")]
    fn installing_inside_a_section_panics_by_name() {
        let mut slot = FpState::DEFAULT;
        with_simd(|_| install(&mut slot));
    }

    #[test]
    fn snapshot_and_replace_follow_ownership() {
        let mut owner = with_mxcsr(RU);
        let mut other = with_mxcsr(RZ);
        install(&mut owner);
        restore(&with_mxcsr(RD_FZ));
        assert_eq!(snapshot(&owner).mxcsr(), RD_FZ, "an owner's snapshot is the live state");
        assert_eq!(snapshot(&other).mxcsr(), RZ, "a non-owner's snapshot is its slot");
        replace(&mut other, &with_mxcsr(RU));
        assert_eq!(
            (other.mxcsr(), live_mxcsr()),
            (RU, RD_FZ),
            "replacing a non-owner leaves the live state alone"
        );
        replace(&mut owner, &FpState::DEFAULT);
        assert_eq!(
            (owner.mxcsr(), live_mxcsr()),
            (DFLT, DFLT),
            "replacing the owner rewrites the live state"
        );
        discard(&mut owner);
        assert!(this_cpu().owner().is_null(), "discard drops the owner");
        reset();
    }

    #[test]
    fn leave_parks_only_the_owner() {
        let mut owner = FpState::DEFAULT;
        let mut stranger = with_mxcsr(RZ);
        install(&mut owner);
        restore(&with_mxcsr(RU));
        leave(&mut stranger);
        assert_eq!(stranger.mxcsr(), RZ);
        assert!(ptr::eq(this_cpu().owner(), &owner));
        leave(&mut owner);
        assert_eq!(owner.mxcsr(), RU, "leave parked the live state in the owner's slot");
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
    }

    #[test]
    #[should_panic(expected = "not a multiple of 16")]
    fn xor_fold16_rejects_a_ragged_length() {
        with_simd(|simd| xor_fold16(simd, &[0u8; 17]));
    }

    #[test]
    #[should_panic(expected = "reserved bits")]
    fn restoring_a_reserved_mxcsr_bit_panics_instead_of_faulting() {
        restore(&with_mxcsr(1 << 16));
    }
}
