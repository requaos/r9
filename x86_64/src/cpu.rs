//! Architecture specific bits.  Mostly interfaces to specific
//! machine registers or instructions that must be accessed from
//! assembler.

use crate::dat::{Flags, Gdt, Idt};

use bit_field::BitField;
use core::arch::asm;

/// The SYSCALL/SYSRET entry-contract MSRs — the vsvm.c:174-185 arming
/// order's operands.  l.S already sets `MSR_EFER`'s Sce bit for
/// long-mode bring-up (leaving Star/Lstar/Sfmask at 0); the kernel-init
/// arming (`main::arm_syscall_entry`) writes the full donor set so the
/// routing every user SYSCALL depends on exists from boot.
pub const MSR_EFER: u32 = 0xc0000080;
pub const MSR_STAR: u32 = 0xc0000081;
/// The SYSCALL entry target — armed at the real entry asm (trap.rs's
/// `syscallentry`) by `main::arm_syscall_entry`; never at a stub or a
/// hole (the 14-02 rule: the missing-symbol link error is the presence
/// guard).
pub const MSR_LSTAR: u32 = 0xc0000082;
pub const MSR_SFMASK: u32 = 0xc0000084;
pub const MSR_FS_BASE: u32 = 0xc0000100;
pub const MSR_GS_BASE: u32 = 0xc0000101;

/// EFER bit 0 — SYSCALL/SYSRET enable (SDM Vol 2, EFER).
pub const EFER_SCE: u64 = 1;

/// The kernel-side of the SWAPGS pair: kernel GS base = the Mach, user
/// (this MSR's counterpart at boot) = 0 — the arming writes it
/// explicitly so the return-path SWAPGS lands the user on base 0.
pub const MSR_KERNEL_GS_BASE: u32 = 0xc0000102;

/// Retrieves a copy of the `RFLAGS` registers.
pub(crate) fn flags() -> Flags {
    const MB1: u64 = 0b10;
    // SAFETY: pushfq/popq are unprivileged and keep the stack balanced;
    // the out(reg) operand fully initializes raw; no memory is touched
    // and no other register is clobbered.
    unsafe {
        let raw: u64;
        asm!("pushfq; popq {};", out(reg) raw, options(att_syntax));
        Flags::new(raw | MB1)
    }
}

/// The IA32_FMASK value: the RFLAGS bits SYSCALL clears on the way into
/// the kernel — IF, TF, DF, NT and AC, the set Linux masks.  The user's
/// own RFLAGS are untouched by this: SYSCALL saves them in R11 before
/// masking, the entry stores R11 as the frame's flags, and SYSRETQ (or
/// NOTED's IRETQ) hands them back.
///
/// This reverses D-02a's If-only value (the 9k donor's `wrmsr(Sfmask,
/// If)`, vsvm.c:185).  The donor could leave TF and DF to the user
/// because its 6c kernel never relied on DF (every string instruction in
/// it sets the direction itself: l64v.s's CLD before each REP) and the
/// ABI says nothing about the flags a syscall runs with.  r9's kernel is
/// Rust: the SysV ABI requires DF=0 at every call, and compiler_builtins'
/// memcpy/memset are bare `rep movs`/`rep stos`, so a user DF ran every
/// kernel copy backwards.  The other bits are the same trust question
/// (WINDOWS #71): TF raised a single-step #DB on the entry's first
/// instruction, still on the user stack; NT made the kernel's next IRETQ
/// fault (#GP in IA-32e mode) — NOTED's return, or another proc resumed
/// by the scheduler, since swtch carries no RFLAGS; AC would turn SMAP
/// off in ring 0 once SMAP is enabled.  Masking at SYSCALL is the only
/// place the TF trap can be stopped (an entry-stub reset runs too late);
/// on the IDT path the gates clear TF and NT and intrcommon clears DF.
pub(crate) fn fmask() -> u64 {
    Flags::empty()
        .with_intr(true)
        .with_trap(true)
        .with_dir(true)
        .with_nestedt(true)
        .with_access(true)
        .bits()
}

/// Executes the `STI` instruction that enables interrupt
/// delivery on the current CPU, by setting the "Interrupt
/// Enable" bit (`IF`) in the `RFLAGS` register
pub(crate) fn sti() {
    // SAFETY: sti is a CPL 0 instruction and the kernel runs at CPL 0;
    // it takes no operands, touches no memory, and clobbers nothing.
    unsafe {
        asm!("sti");
    }
}

/// Executes the `CLI` instruction that disables interrupt
/// delivery on the current CPU, by clearing the "Interrupt
/// Enable" bit (`IF`) in the `RFLAGS` register
pub(crate) fn cli() {
    // SAFETY: cli is a CPL 0 instruction and the kernel runs at CPL 0;
    // it takes no operands, touches no memory, and clobbers nothing.
    unsafe {
        asm!("cli");
    }
}

/// Loads the "Task Register" (`TR`) with the given 16-bit
/// selector index, which identifies a "Task State Selector"
/// (that points to a "Task State Segment" [TSS]) in the Global
/// Descriptor Table (GDT).
///
/// # Safety
/// The given selector must identify a well-formed TSS selector
/// in the presently loaded GDT.
pub(crate) unsafe fn ltr(selector: u16) {
    // SAFETY: the caller's contract above — selector names a well-formed
    // TSS descriptor in the presently loaded GDT.  ltr is CPL 0-only
    // and the kernel runs at CPL 0; the instruction's architected write
    // (marking the descriptor busy) lands inside that valid GDT.
    unsafe {
        asm!("ltr {:x};", in(reg) selector);
    }
}

/// Loads the "Global Table Descriptor Register" (`GDTR`) with
/// the base address and inclusive limit of a "Global Descriptor
/// Table" (GDT).
///
/// # Safety
/// The referred GDT must be architecturally valid.
pub(crate) unsafe fn lgdt(gdt: &Gdt) {
    let ptr: *const Gdt = gdt;
    // SAFETY: the caller's contract — gdt is an architecturally valid
    // GDT.  The 16 bytes the sequence borrows below %rsp hold the
    // {limit, base} pseudo-descriptor plus the far-return frame that
    // lretq consumes, so %rsp comes back balanced; {base} is the live
    // Gdt's address — mapped readable memory for the load's duration —
    // and lgdt/lretq run at CPL 0.
    unsafe {
        asm!(r#"
            subq $16, %rsp;
            movq {base}, 8(%rsp);
            movw ${limit}, 6(%rsp)
            lgdt 6(%rsp);
            movq $8, 8(%rsp);
            lea 1f(%rip), %rax;
            movq %rax, (%rsp);
            lretq;
            1:
            "#,
            base = in(reg) u64::try_from(ptr.addr()).unwrap(),
            limit = const core::mem::size_of::<Gdt>().wrapping_sub(1) as u16,
            options(att_syntax)
        );
    }
}

/// Loads the "Interrupt Descriptor Table Register" (`IDTR`)
/// with the base address and inclusive limit of an "Interrupt
/// Descriptor Table" (IDT).
///
/// # Safety
/// The referred IDT must be architecturally valid.
pub(crate) unsafe fn lidt(idt: &Idt) {
    let ptr: *const Idt = idt;
    // SAFETY: the caller's contract — idt is an architecturally valid
    // IDT.  The 16 bytes borrowed below %rsp hold the {limit, base}
    // pseudo-descriptor and are restored by the matching addq; {base}
    // is the live Idt's address — mapped readable memory for the load's
    // duration — and lidt runs at CPL 0.
    unsafe {
        asm!(r#"
            subq $16, %rsp;
            movq {base}, 8(%rsp);
            movw ${limit}, 6(%rsp)
            lidt 6(%rsp);
            addq $16, %rsp;
            "#,
            base = in(reg) u64::try_from(ptr.addr()).unwrap(),
            limit = const core::mem::size_of::<Idt>().wrapping_sub(1) as u16,
            options(att_syntax)
        );
    }
}

/// Reads an MSR
///
/// # Safety
/// The caller must ensure that the MSR is valid.
pub unsafe fn _rdmsr(msr: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: the caller's contract — msr is a valid MSR.  rdmsr is
    // CPL 0-only, returns the value through the named eax/edx out
    // operands (fully initializing lo/hi), and touches no memory.
    unsafe {
        asm!("rdmsr",
            in("ecx") msr,
            out("eax") lo,
            out("edx") hi,
            options(att_syntax));
    }
    u64::from(hi) << 32 | u64::from(lo)
}

/// Executes `CPUID` with the given leaf and subleaf, returning
/// (eax, ebx, ecx, edx).  The timer's mode gate (leaf 1 ECX bit 24)
/// and its frequency leaves (0x15/0x16) ride this.
///
/// CPUID faults nowhere; an unimplemented leaf returns the maximum
/// leaf's values by architectural definition (SDM Vol 2, CPUID) — the
/// function is safe.  %rbx cannot be named as an inline-asm operand on
/// this toolchain (LLVM reserves it), so the value is walked through
/// scratch operands with the original %rbx saved and restored around
/// the instruction — cpuid writes ebx, and the compiler is free to
/// hold live state there.
pub fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let (a, c, d): (u32, u32, u32);
    let (_rbx_saved, rbx_val): (u64, u64);
    // SAFETY: CPUID faults nowhere and needs no privilege (SDM Vol 2 —
    // an unimplemented leaf returns the maximum leaf's values); %rbx is
    // saved and restored around the instruction because LLVM reserves
    // it, every output returns through a named operand, and no memory
    // is touched.
    unsafe {
        asm!(
            "movq %rbx, {saved}",
            "cpuid",
            "movq %rbx, {val}",
            "movq {saved}, %rbx",
            saved = out(reg) _rbx_saved,
            val = out(reg) rbx_val,
            in("eax") leaf,
            in("ecx") subleaf,
            lateout("eax") a,
            lateout("ecx") c,
            lateout("edx") d,
            options(att_syntax));
    }
    // 32-bit writes zero-extend in long mode: the upper half of %rbx is
    // zero after cpuid, so the plain u64 move is the full leaf value.
    (a, rbx_val as u32, c, d)
}

/// CR4 bit 18 — OSXSAVE: enables XGETBV/XSETBV and the XSAVE instruction
/// family (XSAVE/XSAVEOPT/XRSTOR).  Gated on CPUID reporting both XSAVE
/// (leaf 1 ECX[26]) and AVX (leaf 1 ECX[28]) — an ungated write is a
/// boot `#GP(0)` on CPUs without XSAVE (TCG's qemu64; MEM-18 Pitfall 8).
pub const CR4_OSXSAVE: u64 = 1 << 18;

/// Reads CR4 — fpu.rs's MEM-18 OSXSAVE arm is the one caller (a
/// read-modify-write preserving every l.S/trampoline-armed bit).
pub(crate) fn cr4() -> u64 {
    let val: u64;
    // SAFETY: mov-from-CR4 is CPL 0-only and the kernel runs at CPL 0;
    // it takes no operands, returns through the named out register
    // (fully initializing val), and touches no memory.
    unsafe {
        asm!("mov %cr4, {}", out(reg) val, options(att_syntax));
    }
    val
}

/// Writes CR4 — fpu.rs's MEM-18 OSXSAVE arm is the one caller.
///
/// # Safety
/// The value must keep every bit the loader/kernel armed (PAE, PGE,
/// OSFXSR, OSXMMEXCPT — l.S and ap_trampoline.S) set; callers
/// read-modify-write from [`cr4`].  Setting OSXSAVE is legal only when
/// CPUID reports XSAVE + AVX (the caller's gate).
pub(crate) unsafe fn write_cr4(val: u64) {
    // SAFETY: the caller's contract above.  mov-to-CR4 is CPL 0-only,
    // takes its operand in a register, and touches no memory.
    unsafe {
        asm!("mov {}, %cr4", in(reg) val, options(att_syntax));
    }
}

/// Executes XSETBV — write the extended-control register `xcr` (only
/// XCR0 exists) — fpu.rs's MEM-18 XCR0 = 0x7 arm (x87|SSE|AVX; AVX-512
/// bits stay off).
///
/// # Safety
/// CR4.OSXSAVE must be set first (XSETBV #UDs without it) and `val` must
/// name only bits CPUID leaf 0xD.0 EAX reports as settable plus bit 0
/// (x87, always required) — an unsupported bit or a cleared bit 0 #GPs.
/// The caller (fpu's arm path) checks exactly that.
pub(crate) unsafe fn xsetbv(xcr: u32, val: u64) {
    // SAFETY: the caller's contract above.  xsetbv is CPL 0-only, takes
    // the XCR number in ecx and the value split across edx:eax through
    // named in operands, and touches no memory.
    unsafe {
        asm!(
            "xsetbv",
            in("ecx") xcr,
            in("eax") val as u32,
            in("edx") (val >> 32) as u32,
            options(att_syntax));
    }
}

/// Reads the 64-bit time-stamp counter (the TSC-deadline arm and the
/// frequency calibration both read it).
///
/// # Safety
/// RDTSC is unprivileged and always available.
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: RDTSC is unprivileged and always available (the fn doc's
    // contract; the kernel never sets CR4.TSD); eax/edx return through
    // named lateout operands, fully initializing lo/hi, and no memory
    // is touched.
    unsafe {
        asm!("rdtsc", lateout("eax") lo, lateout("edx") hi, options(att_syntax));
    }
    u64::from(hi) << 32 | u64::from(lo)
}

/// Pure decision helper: does leaf-1 ECX bit 24
/// (`CPUID_EXT_TSC_DEADLINE_TIMER`) select TSC-deadline timer mode?
/// Host-testable without a CPU — the timer's mode gate routes through
/// this (RESEARCH Pitfall 2: TCG never exposes the bit).
pub fn has_tsc_deadline(cpuid1_ecx: u32) -> bool {
    cpuid1_ecx & (1 << 24) != 0
}

/// Pure decision helper: does leaf-1 ECX bit 21 (`CPUID_EXT_X2APIC`)
/// advertise x2APIC mode?  Host-testable without a CPU — lapic's mode
/// gate routes through this.  KVM's `-cpu host` passes the host's x2APIC
/// through (the i7-1260P gate host has it); TCG's qemu64 never exposes
/// the bit (the fallback leg keeps exercising classic xAPIC MMIO).
pub fn has_x2apic(cpuid1_ecx: u32) -> bool {
    cpuid1_ecx & (1 << 21) != 0
}

/// Pure decision helper: does leaf-1 ECX bit 30 (`CPUID_EXT_RDRAND`)
/// advertise the RDRAND instruction?  Host-testable without a CPU —
/// entropy's source gate routes through this (18-01/D-05).  KVM's
/// `-cpu host` passes the host's RDRAND through; TCG's qemu64 never
/// exposes the bit (the jitter fallback keeps exercising the TSC mix
/// there).
pub fn has_rdrand(cpuid1_ecx: u32) -> bool {
    cpuid1_ecx & (1 << 30) != 0
}

/// Pure decision helper: does leaf-7 subleaf-0 EBX bit 18
/// (`CPUID_LEAF7_EBX_RDSEED`) advertise the RDSEED instruction, given
/// the CPU's maximum standard leaf?  The max-leaf check comes FIRST:
/// a CPU whose maximum leaf is below 7 returns the maximum leaf's
/// values for any higher leaf request, so those EBX bits are
/// meaningless (SDM Vol 2, CPUID — the same recycled-register trap
/// the x2APIC probe guards against).  Host-testable without a CPU.
pub fn has_rdseed(max_leaf: u32, cpuid7_ebx: u32) -> bool {
    max_leaf >= 7 && cpuid7_ebx & (1 << 18) != 0
}

/// Executes RDRAND — one hardware-random qword with carry-set-on-
/// valid semantics (SDM Vol 2, RDRAND): CF=1 means the destination
/// holds a valid value; CF=0 is the bounded-retry signal (the retry
/// loop and its budget live at the caller, entropy.rs — never an
/// unbounded spin).  The `setc` captures CF immediately; only the
/// has_rdrand-gated paths call this (RDRAND without CPUID support is
/// #UD).
pub fn rdrand() -> Option<u64> {
    let val: u64;
    let ok: u8;
    // SAFETY: only the has_rdrand-gated paths call this, so the CPUID
    // bit guarantees the instruction does not #UD; setc captures CF
    // immediately, both results return through named operands, and no
    // memory is touched.
    unsafe {
        asm!("rdrand {val}",
            "setc {ok}",
            val = out(reg) val,
            ok = out(reg_byte) ok,
            options(att_syntax));
    }
    if ok != 0 { Some(val) } else { None }
}

/// Executes RDSEED — one conditioned-entropy qword for seeding a
/// PRBG (Intel DRNG Software Implementation Guide: RDSEED is the
/// seeding instruction; RDRAND is the PRNG output).  Same carry-only
/// result contract as `rdrand` (SDM Vol 2, RDSEED); the bounded
/// retry lives at the caller.  Only the has_rdseed-gated path calls
/// this (RDSEED without CPUID support is #UD).
pub fn rdseed() -> Option<u64> {
    let val: u64;
    let ok: u8;
    // SAFETY: only the has_rdseed-gated path calls this, so the CPUID
    // bit (behind its max-leaf check) guarantees the instruction does
    // not #UD; setc captures CF immediately, both results return
    // through named operands, and no memory is touched.
    unsafe {
        asm!("rdseed {val}",
            "setc {ok}",
            val = out(reg) val,
            ok = out(reg_byte) ok,
            options(att_syntax));
    }
    if ok != 0 { Some(val) } else { None }
}

/// Writes an MSR — the TSC-deadline timer's arm (MSR 0x6E0, 13-03:
/// written only inside the CPUID.1.ECX[24]-gated mode) and the x2APIC
/// mode switch (IA32_APIC_BASE 0x1B, lapic.rs).
///
/// # Safety
/// The caller must ensure that the value and MSR are
/// valid, and that the value makes sense for the MSR.
pub(crate) unsafe fn wrmsr(msr: u32, value: u64) {
    // SAFETY: the caller's contract — msr is valid and value makes
    // sense for it.  wrmsr is CPL 0-only, takes the MSR number in ecx
    // and the value split across edx:eax through named in operands, and
    // touches no memory.
    unsafe {
        asm!("wrmsr",
            in("ecx") msr,
            in("eax") value.get_bits(0..32),
            in("edx") value.get_bits(32..64),
            options(att_syntax));
    }
}

/// Writes the GS Base register.
///
/// Unused since the old-generation vsvm init retired (l.S's warp64 does
/// the boot-time wrgsbase in asm); user-process %gs swaps (Phase 14)
/// re-arm it.
///
/// # Safety
/// The caller must ensure that the given value makes sense as
/// a %gs segment base value.  Note that we assume we can use
/// the `WRGSBASE` instruction.
#[allow(dead_code)]
pub(crate) unsafe fn wrgsbase(value: u64) {
    // SAFETY: the caller's contract above — value makes sense as a %gs
    // segment base and WRGSBASE is assumed available; the instruction
    // is CPL 0-only, takes its operand in a register, and touches no
    // memory.  (Unused since the old vsvm init retired; kept for the
    // Phase 14 user %gs swap.)
    unsafe {
        asm!("wrgsbase {}", in(reg) value, options(att_syntax));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFLAGS bit positions (SDM Vol 1, 3.4.3), spelled out so the test
    // does not lean on the Flags field names it is checking.
    const IF: u64 = 1 << 9;
    const TF: u64 = 1 << 8;
    const DF: u64 = 1 << 10;
    const NT: u64 = 1 << 14;
    const AC: u64 = 1 << 18;

    /// WINDOWS #71: SYSCALL must reach the kernel's first instruction
    /// with every user-settable flag the kernel cannot run with cleared —
    /// TF (single-step #DB on the user stack), DF (backwards kernel
    /// copies), NT (#GP at the next IRETQ), AC (SMAP off) — plus IF.
    #[test]
    fn fmask_clears_the_flags_the_kernel_must_not_inherit() {
        let m = fmask();
        for (name, bit) in [("IF", IF), ("TF", TF), ("DF", DF), ("NT", NT), ("AC", AC)] {
            assert_ne!(m & bit, 0, "IA32_FMASK leaves {name} set on SYSCALL entry");
        }
    }

    /// Exactly the Linux set: the arithmetic flags, IOPL (a CPL3 POPFQ
    /// cannot change it) and ID stay the user's, and R11 carries the
    /// user's RFLAGS back either way.
    #[test]
    fn fmask_is_exactly_if_tf_df_nt_ac() {
        assert_eq!(fmask(), IF | TF | DF | NT | AC);
    }
}
