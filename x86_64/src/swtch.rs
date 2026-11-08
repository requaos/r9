//! Kernel-context switch — the aarch64/src/swtch.rs {analog} re-keyed to
//! the x86_64 SysV callee-saved set.  Used ONLY for kernel-context
//! (blocked mid-syscall) switches: the sched machinery parks a proc by
//! swtch-ing into the scheduler context and resumes it the same way.
//! Ring-3 entry/exit NEVER rides this path — the SYSCALL entry/return
//! pair (trap.rs) and `trap_return` are the only ways CPL3 is entered.
//!
//! The saved shape is `dat::Label` (pc, sp, fp/rbp, rbx, r12-r15) — the
//! struct dat.rs already carries for exactly this purpose ("a label is
//! a place where we can `swtch` to"); the Label fields are pub(crate)
//! so the naked asm's slot arithmetic can be witnessed against them.

use core::arch::naked_asm;

use crate::dat::Label;

/// A kernel swtch context — `Label` under its process-scheduling name
/// (the aarch64 `swtch::Context` role).
pub type Context = Label;

impl Context {
    /// Seed the resume pc (first-activation seeds name the target fn).
    pub fn set_return(&mut self, addr: u64) {
        self.pc = addr;
    }

    /// Seed the stack pointer.
    pub fn set_stack_pointer(&mut self, addr: u64) {
        self.sp = addr;
    }
}

// The asm's slot arithmetic against the Label layout (repr(C): pc@0,
// sp@8, fp@16, rbx@24, r12@32, r13@40, r14@48, r15@56).
const _: () = assert!(core::mem::offset_of!(Label, pc) == 0);
const _: () = assert!(core::mem::offset_of!(Label, sp) == 8);
const _: () = assert!(core::mem::offset_of!(Label, fp) == 16);
const _: () = assert!(core::mem::offset_of!(Label, rbx) == 24);
const _: () = assert!(core::mem::offset_of!(Label, r12) == 32);
const _: () = assert!(core::mem::offset_of!(Label, r15) == 56);

/// Kernel-context switch: save the callee-saved registers + sp + a
/// resume pc into `from`, load them from `to`, and jump to `to`'s pc.
/// Symmetric — a Context saved by a prior swtch resumes after its own
/// call site (the shared `1:` resume label is inside this one function,
/// so every saved context's pc names the same code address).
///
/// # Safety
///
/// Both pointers must name valid, aligned Context storage stable across
/// the switch (PROC_TABLE-resident contexts — the proc.rs discipline);
/// single CPU, IRQ-masked contexts only.
#[cfg(not(test))]
#[unsafe(link_section = ".text")]
#[unsafe(naked)]
pub unsafe extern "C" fn swtch(from: *mut Context, to: *const Context) {
    naked_asm!(
        r#"
        // (1) Save into *from (rdi): the SysV callee-saved set in Label
        //     order, then our own sp and resume pc.
        movq %rbx, 8*3(%rdi);
        movq %rbp, 8*2(%rdi);
        movq %r12, 8*4(%rdi);
        movq %r13, 8*5(%rdi);
        movq %r14, 8*6(%rdi);
        movq %r15, 8*7(%rdi);
        movq %rsp, 8*1(%rdi);
        leaq 1f(%rip), %rax;
        movq %rax, 8*0(%rdi);
        // (2) Load from *to (rsi) — the same layout.
        movq 8*3(%rsi), %rbx;
        movq 8*2(%rsi), %rbp;
        movq 8*4(%rsi), %r12;
        movq 8*5(%rsi), %r13;
        movq 8*6(%rsi), %r14;
        movq 8*7(%rsi), %r15;
        movq 8*1(%rsi), %rsp;
        // (3) Jump to to.pc — a previously-saved context's pc is the
        //     resume label below (it "returns" to its own swtch call's
        //     caller); a first-activation seed names the target fn.
        jmpq *0(%rsi);
        1:
        ret;
        "#,
        options(att_syntax))
}

/// The host-test stand-in: no kernel stacks exist on the host, and no
/// pure-logic test may switch into one — a loud guard, never a silent
/// no-op that would fake a context switch.
#[cfg(test)]
pub unsafe extern "C" fn swtch(_from: *mut Context, _to: *const Context) {
    unreachable!("swtch is unavailable on the host (no kernel stacks)")
}
