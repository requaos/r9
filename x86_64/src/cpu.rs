//! Architecture specific bits.  Mostly interfaces to specific
//! machine registers or instructions that must be accessed from
//! assembler.

use crate::dat::{Flags, Gdt, Idt};

use bit_field::BitField;
use core::arch::asm;

/// Unused until user processes exist (Phase 14 XEXEC swaps %gs on
/// syscall entry/exit).
#[allow(dead_code)]
pub const MSR_KERNEL_GS_BASE: u32 = 0xc0000102;

/// Retrieves a copy of the `RFLAGS` registers.
pub(crate) fn flags() -> Flags {
    const MB1: u64 = 0b10;
    unsafe {
        let raw: u64;
        asm!("pushfq; popq {};", out(reg) raw, options(att_syntax));
        Flags::new(raw | MB1)
    }
}

/// The IA32_FMASK value syscalls enter with (the syscall entry's MSR
/// write retired with the old generation; Phase 14 XEXEC re-arms it).
#[allow(dead_code)]
pub(crate) fn fmask() -> u64 {
    Flags::empty().with_intr(true).with_trap(true).with_dir(true).bits()
}

/// Executes the `STI` instruction that enables interrupt
/// delivery on the current CPU, by setting the "Interrupt
/// Enable" bit (`IF`) in the `RFLAGS` register
pub(crate) fn sti() {
    unsafe {
        asm!("sti");
    }
}

/// Executes the `CLI` instruction that disables interrupt
/// delivery on the current CPU, by clearing the "Interrupt
/// Enable" bit (`IF`) in the `RFLAGS` register
pub(crate) fn cli() {
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
/// (eax, ebx, ecx, edx).
///
/// Unused until 13-03's timer lands (leaf 1 ECX bit 24 — the
/// TSC-deadline gate — and the 0x15/0x16 frequency leaves).
///
/// CPUID faults nowhere; an unimplemented leaf returns the maximum
/// leaf's values by architectural definition (SDM Vol 2, CPUID) — the
/// function is safe.  %rbx cannot be named as an inline-asm operand on
/// this toolchain (LLVM reserves it), so the value is walked through
/// scratch operands with the original %rbx saved and restored around
/// the instruction — cpuid writes ebx, and the compiler is free to
/// hold live state there.
#[allow(dead_code)]
pub fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let (a, c, d): (u32, u32, u32);
    let (_rbx_saved, rbx_val): (u64, u64);
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

/// Reads the 64-bit time-stamp counter.
///
/// Unused until 13-03's timer lands (the TSC-deadline arm and the
/// frequency calibration both read it).
///
/// # Safety
/// RDTSC is unprivileged and always available.
#[allow(dead_code)]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        asm!("rdtsc", lateout("eax") lo, lateout("edx") hi, options(att_syntax));
    }
    u64::from(hi) << 32 | u64::from(lo)
}

/// Pure decision helper: does leaf-1 ECX bit 24
/// (`CPUID_EXT_TSC_DEADLINE_TIMER`) select TSC-deadline timer mode?
/// Host-testable without a CPU — the timer's mode gate routes through
/// this (13-03 Task 2, RESEARCH Pitfall 2: TCG never exposes the bit).
#[allow(dead_code)]
pub fn has_tsc_deadline(cpuid1_ecx: u32) -> bool {
    cpuid1_ecx & (1 << 24) != 0
}

/// Writes an MSR — the TSC-deadline timer's arm (MSR 0x832, 13-03:
/// written only inside the CPUID.1.ECX[24]-gated mode).
///
/// # Safety
/// The caller must ensure that the value and MSR are
/// valid, and that the value makes sense for the MSR.
pub(crate) unsafe fn wrmsr(msr: u32, value: u64) {
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
    unsafe {
        asm!("wrgsbase {}", in(reg) value, options(att_syntax));
    }
}
