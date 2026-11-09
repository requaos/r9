use crate::cpu;
use crate::dat;
use crate::dat::{
    Gdt, MACH_SCRATCH_OFFSET, MachMode, UREG_BP_OFFSET, UREG_CS_OFFSET, UREG_DS_OFFSET,
    UREG_SP_OFFSET, UREG_TRAPNO_OFFSET, Ureg,
};
use crate::seg::TSS_RSP0_OFFSET;

use core::arch::naked_asm;

pub const DEBUG_TRAPNO: u8 = 1;
pub const NMI_TRAPNO: u8 = 2;
pub const BREAKPOINT_TRAPNO: u8 = 3;
pub const DOUBLE_FAULT_TRAPNO: u8 = 8;

/// The LAPIC timer's IDT vector — the first vector past the
/// architectural exception range 0..=31.  The 0x20..=0x3F span is the
/// spare IRQ span 13-05's IOAPIC lines occupy; every gate already
/// exists (the stub array and Idt::init cover all 256).
pub const TIMER_VECTOR: u8 = 0x20;

// The retired SWAPGS witness-bit discipline (SWAPPED/SWAPPED_CS_OFFSET,
// CR-01's class): its bit lived in the pushed CS qword's upper bytes,
// which are only written on the from-user entry arm — a kernel-origin
// frame inherited stale stack bytes there, and a stale set bit made
// intrcommon's return swap %gs spuriously (the 14-07 root cause; the
// discipline is now the donor's: both sides cmpw the saved CS word,
// which carries only CPU-written selector bits).  Retired with the
// fix; the history lives in the git log and 14-07's SUMMARY.

// The SYSCALL entry contract's flat user selectors — the donor's
// SSEL(si, SsRPL3) re-keyed to r9's GDT order: udata@0x18 → 0x1B
// (DS/ES/FS/GS and the frame's ss slot), utext@0x20 → 0x23 (the cs
// slot — the user CS comparison value, amd64/ABI.md's entry state).
// pub(crate): the exec seed (dat.rs seed_exec_entry) plants the same
// selector values into the seeded Ureg.
pub(crate) const USER_DATA_SEL: u16 = Gdt::udatasel() | MachMode::User as u16;
pub(crate) const USER_TEXT_SEL: u16 = Gdt::utextsel() | MachMode::User as u16;

/// UTZERO — first address of user text (ABI.md's stack-top policy);
/// +0x28 (past the a.out header, HEADR=40) is the seeded first
/// process's linked entry — 6l's `INITTEXT = 0x200000+HEADR`.
/// Unused until 14-05's exec_bootstrap (touser's planted entry).
#[allow(dead_code)]
const UTZERO: u64 = 2 * 1024 * 1024;

/// The byte offset of the TSS's rsp0 within the Mach — the SYSCALL
/// entry's kernel-stack switch reads it gs-relative (see
/// `syscallentry`).  Computed from the structs, never hand-pinned.
const MACH_TSS_RSP0: usize = core::mem::offset_of!(dat::Mach, tss) + TSS_RSP0_OFFSET;

// Compile-time witnesses: the entry asm's slot arithmetic against the
// frozen 184-byte layout (the D-01 frame).  dat.rs's assert block pins
// the struct; these pin the asm's literals to the SAME numbers via the
// exported UREG_* offsets — bp@48 (the wire number's slot), the
// quartet qword@120, ip@144, cs@152, sp@168, and the carve+hardware
// halves summing to 184.
const _: () = assert!(core::mem::offset_of!(Ureg, pc) == 18 * 8);
const _: () = assert!(UREG_CS_OFFSET == 19 * 8);
const _: () = assert!(UREG_BP_OFFSET == 6 * 8);
const _: () = assert!(UREG_DS_OFFSET == 15 * 8);
const _: () = assert!(UREG_SP_OFFSET == 21 * 8);
const _: () = assert!(18 * 8 + 5 * 8 == core::mem::size_of::<Ureg>());
// The selector pins catch any GdtData reorder: udata@0x18|3, utext@0x20|3.
const _: () = assert!(USER_DATA_SEL == 0x1b);
const _: () = assert!(USER_TEXT_SEL == 0x23);

type Thunk = unsafe extern "C" fn();

#[repr(transparent)]
pub struct Stub(usize);

impl Stub {
    /// Returns a function pointer that represents a stub.
    pub unsafe fn as_thunk(&self) -> Thunk {
        unsafe { core::mem::transmute::<usize, Thunk>(self.0) }
    }
}

macro_rules! gen_stub {
    () => {
        r#".balign 8; pushq $0; callq {intrregular};"#
    };
    (spec) => {
        r#".balign 8; pushq $0; callq {intrspecial};"#
    };
    (err) => {
        r#".balign 8; callq {intrregular};"#
    };
    // A hardware error word is already on the frame, but the
    // entry needs the special GSBASE check (the double fault).
    (err_spec) => {
        r#".balign 8; callq {intrspecial};"#
    };
}

macro_rules! gen_trap_stub {
    // These cases include hardware-generated error words
    // on the trap frame.  The double fault (8) is ALSO one
    // of the special cases below — an exception inside
    // exception handling makes the CS-selector proxy
    // unreliable, so it takes the special path while keeping
    // its hardware error word (no fake error push).
    (8) => {
        gen_stub!(err_spec)
    };
    (10) => {
        gen_stub!(err)
    };
    (11) => {
        gen_stub!(err)
    };
    (12) => {
        gen_stub!(err)
    };
    (13) => {
        gen_stub!(err)
    };
    (14) => {
        gen_stub!(err)
    };
    (17) => {
        gen_stub!(err)
    };
    // These interrupts are special, in that GSBASE may not be properly
    // set when we enter them.
    (1) => {
        gen_stub!(spec)
    };
    (2) => {
        gen_stub!(spec)
    };
    (3) => {
        gen_stub!(spec)
    };
    // No hardware error
    ($num:literal) => {
        gen_stub!()
    };
}

pub fn stubs() -> &'static [Stub; 256] {
    unsafe { &*(intr_stubs as *const () as usize as *const [Stub; 256]) }
}

/// intr_stubs is just a container for interrupt stubs.  It is
/// a naked function for convenience.
///
/// # Safety
///
/// Container for thunks.
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
#[rustc_align(4096)]
pub unsafe extern "C" fn intr_stubs() {
    use seq_macro::seq;
    naked_asm!(
        seq!(N in 0..=255 {
            concat!( #( gen_trap_stub!(N), )* )
        }),
        intrregular = sym intrregular,
        intrspecial = sym intrspecial,
        options(att_syntax))
}

/// Some interrupts require special handling on delivery,
/// because there is no good way to detect whether `GSBASE` was
/// swapped with the contenst of `MSR_KERNEL_GS_BASE` by simply
/// looking at the exception frame on the stack: NMIs, double
/// faults, and the various debug exceptions fall into this
/// category.
///
/// To understand this, consider an NMI and how it may interact
/// with a non-NMI interrupt handler that is executing in kernel
/// mode: on receipt of an interrupt, the CPU will look up the
/// corresponding IDT entry and invoke a gate handler for the
/// interrupt.  On R9, the handler from the interrupt gate
/// defined in the IDT is one of the trap stubs created above,
/// and the trap stub will be invoked with interrupts disabled.
///
/// The trap stub ensures the stack is aligned vis the error
/// number and then immediately call into `intrcommon`, which
/// saves registers and then examines the CS segment selector
/// pushed onto stack by hardware.  If we're already in the
/// kernel, we just continue executing.  If not, then we
/// execute a `SWAPGS` instruction, so that `%GSBASE` points to
/// this CPU's `Mach` and we have access to the (kernel's)
/// per-CPU data.
///
/// While the interrupt gate prohibits delivery of other regular
/// interrupts, this does not apply to either synchronous
/// exceptions or to NMIs, which by definition are not maskable.
/// Since we are in a trap handler, any non-debug synchronous
/// exception will result in a double fault, and NMIs are
/// delivered normally.  (An exception is that, when handling
/// an NMI, delivery of subsequent NMIs is suspended until an
/// `IRETQ` is executed, but here we're considering the case of
/// NMI delivery while handling a maskable interrupt).
///
/// Thus, there is a race here: when handling a normal interrupt
/// or synchronous exception, an NMI may be delivered any time
/// after we have entered the kernel, but before we have
/// executed `SWAPGS`.  In that case, if we entered via the
/// normal interrupt handling path, we would see that we were
/// already in kernel mode and elide the `SWAPGS`, so that when
/// we entered the rest of the kernel, `%GSBASE` would still be
/// pointing to whatever it had been set to while running in
/// user mode, which is clearly wrong.
///
/// To address this, when we receive one of these traps, we need
/// to decide whether to invoke `SWAPGS` by testing something
/// more reliable than the code segment selector on the stack.
/// Fortunately on R9, the Mach is at a fixed virtual address
/// for each CPU, so we can look at `%GSBASE` itself (e.g., via
/// `RDGSBASE`) to see if it is already pointing to our `Mach`,
/// but is this safe?   There are two cases: if it is not equal
/// to our Mach, then we need to `SWAPGS`.  If it is pointing
/// to our Mach, then one of two things has happened: either
/// we have already executed `SWAPGS`, and whatever userspace
/// had set `%GSBASE` to is in `MSR_KERNEL_GS_BASE`, or we have
/// not executed `SWAPGS` and userspace had explicitly set
/// `%GSBASE` to the address of our `Mach`.  In the latter case,
/// `MSR_KERNEL_GS_BASE` will still hold the address of our
/// `Mach`, and so whether or not we run `SWAPGS` is moot.
///
/// There is also a separate, related, complication we must
/// handle in the return path.  When returning to userspace, we
/// want to make sure that we run `SWAPGS` again to restore the
/// user `%GSBASE` (and stash our `Mach` pointer in
/// `MSR_KERNEL_GS_BASE`).  But if we simply test the code
/// segment selector on the stack frame as a proxy for this, we
/// open ourselves up to the same race as when deciding whether
/// to invoke `SWAPGS` on entry or not.  Specifically, consider
/// the case of running in user mode when an ordinary interrupt
/// is received.  We test against the code segment selector as a
/// proxy for whether we should `SWAPGS`.  Now suppose that an
/// NMI is delivered after testing the CS selector, but before
/// `SWAPGS`: the NMI handler will correctly `SWAPGS` itself,
/// but when returning from the NMI, we would see that we came
/// from kernel mode and skip swapping when we go back to the
/// original interrupt handler.  But that handler had already
/// observed that we were coming from userspace, and should
/// will now execute `SWAPGS`, which will undo what the NMI
/// handler had done: we will now be running in the kernel with
/// the user `%GSBASE`.
///
/// To avoid this, we again need a more robust mechanism to
/// determine whether we should `SWAPGS` on return: in effect we
/// need some kind of witness to whether this invocation of the
/// handler swapped, and our invariant is that, if `%GSBASE` was
/// swapped on entry, it must be swapped on return.
///
/// To handle this, we need some place to stash knowledge that
/// we ran `SWAPGS`.  Fortunately, the 16-bit code segment
/// selector is zero-extended and pushed to the trap stack as a
/// 64-bit word, so we know that there are 48 zero bits at a
/// known location on the stack.  If we execute `SWAPGS`, we
/// set one of those bits, and test it on return; if set, we
/// `SWAPGS` in the return path.
///
/// Returning to the example of an NMI received after entry to
/// a handler due to an interrupt received while in user mode,
/// and after the comparison against the code segment selectors
/// but before `SWAPGS`, the NMI handler we will observe that
/// `%GSBASE` needs to be swapped and will `SWAPGS` and set a
/// bit in the high part of the CS selector on the stack.  On
/// return, it will see that the bit is set and `SWAPGS` again,
/// restoring the user `%GSBASE`.  Now back in the original
/// interrupt handler, we will `SWAPGS`, again properly setting
/// `%GSBASE` and setting a bit to restore it on return;
/// assuming no other intervening NMIs or synchronous exceptions
/// are delivered, we will observe the bit set in the segment
/// selector word on the stack, `SWAPGS` to restore the user
/// `%GSBASE`, clear the bit, and return normally.
///
/// # Safety
///
/// Invoked directly from trap stubs.
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
pub unsafe extern "C" fn intrspecial() {
    naked_asm!(r#"
        pushq %rax;
        pushq %rbx;
        movabsq ${KMACH}, %rbx;
        rdgsbase %rax;
        cmpq %rbx, %rax;
        popq %rbx;
        popq %rax;
        je 1f;
        swapgs;
        // No witness bit: the return side re-tests the saved CS word
        // itself (the donor's CMPW discipline — a stale upper byte in
        // the pushed CS qword can never forge the swap-back decision).
        1:
        pushq ${intrcommon};
        retq;
        "#,
        KMACH = const dat::KMACH,
        intrcommon = sym intrcommon,
        options(att_syntax))
}

/// The "regular" interrupt entry point.  It does `SWAPGS` if
/// necessary, based on the code segment selector pushed onto
/// the stack by hardware.  See the comments at the top of
/// `intrspecial` to understand what is going on here.
///
/// # Safety
///
/// Regular trap handler, called from (most) trap stubs.
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
unsafe extern "C" fn intrregular() {
    naked_asm!(r#"
        // Check if we need to execute `SWAPGS`.
        // Skip if we're already in kernel mode.
        cmpw ${KTEXT_SEL}, {CS_OFFSET}(%rsp);
        je 1f;
        swapgs;
        1:
        jmp {intrcommon}
        "#,
        KTEXT_SEL = const 8,
        CS_OFFSET = const 24,
        intrcommon = sym intrcommon,
        options(att_syntax));
}

/// intrcommon builds up a Ureg structure at the top of the
/// current proc's kernel stack and invokes the main trap
/// entry point.
///
/// See the icomments on `intrspecial` about return handling for
/// `SWAPGS` to understand `SWAPPED` etc.
///
/// # Safety
///
/// Common trap handler.  Called from interrupt/exception stub.
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
unsafe extern "C" fn intrcommon() {
    naked_asm!(r#"
        // Allocate space to save registers: 15 GP qwords
        // plus ONE packed segment qword below the vector
        // slot — the 9k syscallentry carve arithmetic (the
        // u16 ds/es/fs/gs quartet shares qword 15).
        subq $(16 * 8), %rsp
        // Save the general purpose registers.
        movq %r15, 14*8(%rsp);
        movq %r14, 13*8(%rsp);
        movq %r13, 12*8(%rsp);
        movq %r12, 11*8(%rsp);
        movq %r11, 10*8(%rsp);
        movq %r10, 9*8(%rsp);
        movq %r9, 8*8(%rsp);
        movq %r8, 7*8(%rsp);
        movq %rbp, 6*8(%rsp);
        movq %rdi, 5*8(%rsp);
        movq %rsi, 4*8(%rsp);
        movq %rdx, 3*8(%rsp);
        movq %rcx, 2*8(%rsp);
        movq %rbx, 1*8(%rsp);
        movq %rax, 0*8(%rsp);
        // Save the x86 segmentation registers.  The four
        // u16 selectors pack into ONE qword at 15*8 — exactly
        // 9k's syscallentry store arithmetic (MOVW to
        // (15*8+n)(SP)) and Harvey's amd64/include/ureg.h:
        // ds@15*8+0, es@15*8+2, fs@15*8+4, gs@15*8+6.
        // Direct word stores need no scratch register.
        movw %ds, 15*8+0(%rsp);
        movw %es, 15*8+2(%rsp);
        movw %fs, 15*8+4(%rsp);
        movw %gs, 15*8+6(%rsp);
        // Fix up the trap number.  We got here via a CALL,
        // so hardware pushed the address after the CALLQ
        // instruction onto the stack.  But we know that
        // each stub is aligned to an 8-byte boundary, at
        // some offset based on the vector number relative
        // to the 4096-byte aligned start of the trap stub
        // array.  Further, each stub is shorter than 8
        // bytes in length.  Thus, we can compute the
        // vector number by dividing the return address by
        // 8, masking off the high bits, and storing it back
        // into the save area.
        //
        // The vector number is an argument to the trap
        // function, along with the address of the Ureg
        // we have built at the top of the stack.
        shrw $3, {TRAPNO_OFFSET}(%rsp);
        movzbl {TRAPNO_OFFSET}(%rsp), %edi;
        movq %rdi, {TRAPNO_OFFSET}(%rsp);
        movq %rsp, %rsi;
        callq {trap};
        // If we're returning to kernel mode, don't swap %gs — the
        // donor's discipline (l64idt.s's return-side CMPW 48(SP)):
        // re-test the SAVED CS WORD itself instead of a witness bit.
        // A witness bit in the pushed CS qword's upper bytes is only
        // ever written on the from-user arm, so a kernel-origin frame
        // inherits whatever stale bytes sit at that stack slot — on
        // the Mach scheduler stack (the idle ticks) a stale set bit
        // made the return swap %gs spuriously and left the per-CPU GS
        // pair at ring-3 orientation mid-kernel, so the next SYSCALL
        // entry's Mach-scratch store faulted (the 14-07 composed-gate
        // root cause, caught by a GS-observation ring: the flip
        // happened inside an idle-hlt tick's return).  The 16-bit
        // compare reads only the selector bits the CPU pushed —
        // immune to the stale upper bytes by construction.
        cmpw ${KTEXT_SEL}, {RCS_OFFSET}(%rsp);
        je 1f;
        swapgs;
        1:
        // Restore the general purpose registers.
        movq 0*8(%rsp), %rax;
        movq 1*8(%rsp), %rbx;
        movq 2*8(%rsp), %rcx;
        movq 3*8(%rsp), %rdx;
        movq 4*8(%rsp), %rsi;
        movq 5*8(%rsp), %rdi;
        movq 6*8(%rsp), %rbp;
        movq 7*8(%rsp), %r8;
        movq 8*8(%rsp), %r9;
        movq 9*8(%rsp), %r10;
        movq 10*8(%rsp), %r11;
        movq 11*8(%rsp), %r12;
        movq 12*8(%rsp), %r13;
        movq 13*8(%rsp), %r14;
        movq 14*8(%rsp), %r15;
        // Restore the segmentation registers (the packed
        // u16 quartet in the qword at 15*8).
        movw 15*8+0(%rsp), %ds;
        movw 15*8+2(%rsp), %es;
        // %gs is restored via swapgs above.  The system never changes
        // it, so we don't bother restoring it here.  %fs is special.
        // We do save and restore it, for TLS if anyone ever uses green
        // threads.
        movw 15*8+4(%rsp), %fs;
        // movw 15*8+6(%rsp), %gs;
        // Pop GP registers, packed segment qword, vector
        // and error words (the iretq pops ip/cs/flags
        // [+sp/ss when returning to user] itself).
        addq $((2 + 1 + 15) * 8), %rsp;
        // Go back to whence you came.
        iretq
        "#,
        TRAPNO_OFFSET = const UREG_TRAPNO_OFFSET,
        trap = sym trap,
        KTEXT_SEL = const 8,
        RCS_OFFSET = const UREG_CS_OFFSET,
        options(att_syntax))
}

/// touser — the first ring-3 entry (the donor l64syscall.s:8-22).
///
/// Input contract (donor-verbatim): the user sp arrives in `%rbp`
/// (RARG, the 6c first-argument register — the donor's `MOVQ RARG,
/// SP`); the entry ip is fixed at `UTZERO+0x28` (the seeded first
/// process, past the a.out header) and the flags at `If` — exec'd
/// processes instead enter through the frame-return tail
/// (`syscallreturn`), which is why the donor can fix both here.
/// 14-05's exec_bootstrap hands the first process off through this.
///
/// CLI, SWAPGS (kernel GS base → the user base 0; the arming's
/// KernelGSbase=0 makes the pair land right), the flat user data
/// selector into DS/ES/FS/GS, CX=ip, R11=flags, SP=user sp, SYSRETQ
/// (CS/SS derive from STAR's +16/+8 arithmetic; the planted flags
/// carry IF=1 — the unmask-at-handoff discipline).
///
/// # Safety
///
/// Invoked at the ring-3 handoff with interrupts masked; the caller
/// must have a user address space live and the TSS rsp0 set to the
/// entering process's kernel stack.

// Unused until 14-05's exec_bootstrap hands the first process off —
// nothing can reach it while no user image exists (armed-but-idle).
#[allow(dead_code)]
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
pub unsafe extern "C" fn touser() {
    naked_asm!(
        r#"
        cli;
        swapgs;
        movl ${UDS}, %eax;
        movw %ax, %ds;
        movw %ax, %es;
        movw %ax, %fs;
        movw %ax, %gs;

        movq ${ENTRY}, %rcx;
        movq ${IF}, %r11;

        movq %rbp, %rsp;

        sysretq;
        "#,
        UDS = const USER_DATA_SEL,
        ENTRY = const UTZERO + 0x28,
        IF = const 0x200,    // If — the user enters with IF=1
        options(att_syntax))
}

/// syscallentry — the SYSCALL/LSTAR target (the donor
/// l64syscall.s:26-48; the r9 deltas per the 14-02 pattern map).
///
/// SYSCALL pushes NOTHING — no SS/RSP/FLAGS/CS/RIP.  The
/// architectural contract instead deposits: RCX = user rip, R11 = user
/// flags, RSP still the user sp, IF cleared (Sfmask = If), CPL 0,
/// CS/SS from STAR[47:32].  This entry software-builds the
/// hardware-frame half of the 184-byte Ureg from those, stores the GP
/// half, and calls the Rust dispatch — then jumps to the return tail.
///
/// Deltas from the donor (each documented; the frame arithmetic is
/// donor-exact):
/// - SWAPGS unconditional — SYSCALL only fires from ring 3, no
///   CS-check witness (unlike the IDT path).  NMI-safe: intrspecial
///   re-derives its swap from rdgsbase, so an NMI anywhere in the
///   entry window stays consistent either way.
/// - the kernel-stack switch reads the TSS rsp0 gs-relative — the
///   donor walks `m->proc->kstack`; r9's Proc is the 14-03 substrate,
///   and rsp0 is the ONE mechanism both ring-3 entry paths share (the
///   CPU loads it for IDT delivery; the entry loads it here since
///   SYSCALL switches no stack).  Boot-time rsp0 names the Mach's own
///   scheduler stack; the exec bootstrap re-points it at the entering
///   process's kstack (the donor's tssrsp0 idiom).
/// - the 15 GP registers are stored to the frame around the dispatch
///   call: 9k's C `syscall()` preserves everything-but-AX by 6c
///   convention, but the Rust dispatch follows the Rust ABI
///   (caller-saved registers clobbered freely).  Every slot gets the
///   TRUE user value — the donor sacrifices r13/r14/r15 as RMACH/
///   RUSER carriers, while the gs-relative switch below needs no
///   carrier register at all; the only scratch is the Mach's entry
///   word.
/// - the dispatch args are pushed donor-style (Ureg*, number) and
///   reloaded into the Rust convention's argument registers; the
///   return tail consumes them per the donor's ADDQ arithmetic.
///
/// # Safety
///
/// The LSTAR target, armed at kernel init (main::arm_syscall_entry).
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
pub unsafe extern "C" fn syscallentry() {
    naked_asm!(
        r#"
        swapgs;
        // Stash the user sp in the Mach's entry scratch word — the
        // stack switch below destroys RSP and no register may be
        // sacrificed.  IF=0 (Sfmask=If): the window is atomic.
        movq %rsp, %gs:{SCRATCH};
        // The kernel stack: the same rsp0 the CPU loads for a ring-3
        // IDT delivery.
        movq %gs:{TSS_RSP0}, %rsp;
        // The hardware-frame half, software-built (the donor's push
        // order): ss, sp, flags, cs, ip.
        pushq ${SS};
        pushq %gs:{SCRATCH};
        pushq %r11;
        pushq ${CS};
        pushq %rcx;
        // The carve: 15 GP qwords, the u16 quartet's qword, `type`,
        // and the error slot — 18 qwords below the hardware half
        // (the donor's SUBQ $(18*8); type/ecode stay unwritten on
        // this path, exactly as the donor leaves them).
        subq $(18 * 8), %rsp;
        // The GP half — every slot the true user value.
        movq %rax, 0*8(%rsp);
        movq %rbx, 1*8(%rsp);
        movq %rcx, 2*8(%rsp);
        movq %rdx, 3*8(%rsp);
        movq %rsi, 4*8(%rsp);
        movq %rdi, 5*8(%rsp);
        movq %rbp, 6*8(%rsp);
        movq %r8, 7*8(%rsp);
        movq %r9, 8*8(%rsp);
        movq %r10, 9*8(%rsp);
        movq %r11, 10*8(%rsp);
        movq %r12, 11*8(%rsp);
        movq %r13, 12*8(%rsp);
        movq %r14, 13*8(%rsp);
        movq %r15, 14*8(%rsp);
        // The packed segment quartet — the donor's (15*8+n) stores:
        // ds CONSTRUCTED as the flat user data selector (the donor
        // MOVW $SSEL(SiUDS, SsRPL3), (15*8+0)(SP)), es/fs/gs the live
        // selector registers.
        movw ${DS}, 15*8+0(%rsp);
        movw %es, 15*8+2(%rsp);
        movw %fs, 15*8+4(%rsp);
        movw %gs, 15*8+6(%rsp);
        // The dispatch args, donor-style (PUSHQ SP; PUSHQ RARG) —
        // %rbp IS the wire number (RARG = BP, ABI.md's wire
        // contract) — reloaded into the Rust convention's registers.
        pushq %rsp;
        pushq %rbp;
        movq 0(%rsp), %rdi;
        movq 8(%rsp), %rsi;
        // SysV wants %rsp 16-byte aligned at the call: the frame
        // build lands at 8 mod 16 and the two args restore 8, so pad
        // one qword (the boot rsp0 is 64KiB-aligned — 0 mod 16).
        subq $8, %rsp;
        callq {syscall};
        addq $8, %rsp;
        // Drop the pushed args — %rsp is the Ureg base — and take
        // the donor's fall-through into the return tail as a jump
        // (Rust naked fns cannot share a fall-through body).
        addq $(2 * 8), %rsp;
        jmp {syscallreturn};
        "#,
        SCRATCH = const MACH_SCRATCH_OFFSET,
        TSS_RSP0 = const MACH_TSS_RSP0,
        SS = const USER_DATA_SEL,
        CS = const USER_TEXT_SEL,
        DS = const USER_DATA_SEL,
        syscall = sym syscall,
        syscallreturn = sym syscallreturn,
        options(att_syntax))
}

/// syscallreturn — the SYSCALL-path return tail (the donor
/// l64syscall.s:50-68).  Entered with %rsp at the Ureg base: from the
/// entry's dispatch call (via its closing jmp), or from any future
/// first-entry site that plants a frame (the donor's sysrforkret
/// shape — 14-05's territory).
///
/// The dispatch has written its result into the frame's ax slot and
/// may have rewritten bp (the note-delivery convention passes the
/// handler's Ureg* through ureg->bp) — both are re-read from the
/// frame, never assumed live in registers.  The GP registers are
/// restored from the frame (the entry stored the true user values;
/// the Rust ABI clobbered the caller-saved set), then the donor's
/// post-ADDQ choreography: CLI (the dispatch ran spllo), SWAPGS (back
/// to the user GS base 0), the quartet loads, CX=ip, R11=flags, and
/// the user sp LAST — the load destroys the frame pointer — then
/// SYSRETQ (CS/SS derive from STAR).
///
/// # Safety
///
/// Tail-called only from the SYSCALL entry path (or a planted frame).
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
pub unsafe extern "C" fn syscallreturn() {
    naked_asm!(
        r#"
        // The result and the notify channel — through the frame (the
        // donor's MOVQ 16(SP), AX / (16+6*8)(SP), BP re-keyed to the
        // base-relative slots).
        movq 0*8(%rsp), %rax;
        movq 6*8(%rsp), %rbp;
        // The GP half — true user values.  rcx/r11 get overwritten by
        // the exit-state plants below; the last writes win, exactly
        // the donor's read order.
        movq 1*8(%rsp), %rbx;
        movq 2*8(%rsp), %rcx;
        movq 3*8(%rsp), %rdx;
        movq 4*8(%rsp), %rsi;
        movq 5*8(%rsp), %rdi;
        movq 7*8(%rsp), %r8;
        movq 8*8(%rsp), %r9;
        movq 9*8(%rsp), %r10;
        movq 10*8(%rsp), %r11;
        movq 11*8(%rsp), %r12;
        movq 12*8(%rsp), %r13;
        movq 13*8(%rsp), %r14;
        movq 14*8(%rsp), %r15;
        cli;
        swapgs;
        // The packed quartet loads (the donor's post-ADDQ 0/2/4/6
        // reads at the frame's 120..126).
        movw 15*8+0(%rsp), %ds;
        movw 15*8+2(%rsp), %es;
        movw 15*8+4(%rsp), %fs;
        movw 15*8+6(%rsp), %gs;
        // The SYSRETQ exit state: CX=ip, R11=flags (the donor's
        // 24/40 post-ADDQ reads — frame 144/160).
        movq 18*8(%rsp), %rcx;
        movq 20*8(%rsp), %r11;
        // The user sp last (the load destroys the frame pointer) —
        // the donor's MOVQ 48(SP), SP at frame 168.
        movq 21*8(%rsp), %rsp;
        sysretq;
        "#,
        options(att_syntax))
}

/// The SYSCALL dispatch target — the 14-02 fillable stub's body now
/// the real module entry (the SHAPE preserved: the asm's CALL target
/// and argument order unchanged; only the Rust-side body grew, 14-04).
/// Wire facts (amd64/ABI.md): the number travels in BP/RARG (passed
/// through as `number` — the same value the entry pushed and frame.bp
/// holds), arguments sit at the user sp+8 slots, and the result
/// belongs in the frame's ax slot — this function RETURNS normally;
/// the return tail reads the frame after.
///
/// Unreachable until 14-05's exec bootstrap lands a user image (the
/// entry is armed-but-idle: SYSCALL only executes from ring 3).
extern "C" fn syscall(number: u64, frame: &mut Ureg) {
    crate::syscall::entry(number, frame)
}

pub enum IntrStatus {
    Disabled = 0,
    Enabled = 1,
}

/// trap_return — resume (into ring 3) from a planted or preempted Ureg
/// (the aarch64 `trap_return_exec` role; intrcommon's restore tail as a
/// free function).  Entered from the scheduler's Resume::Return arm with
/// the frame pointer in `%rdi`; the frame sits at the top of the target
/// proc's kernel stack (the SYSCALL entry's software-built frame, an
/// IDT preempt's hardware frame, or the exec seed — one layout, D-01).
///
/// SWAPGS is conditional on the frame's saved CS RPL: every path that
/// reaches here runs with the kernel GS base loaded (the SYSCALL/IDT
/// entries swapped at entry and nothing swapped back), and a ring-3
/// resume must leave the user on the user base (0, the arming's
/// KernelGSbase pair).  This is the ONLY way the scheduler enters CPL3;
/// `swtch` never does.
///
/// # Safety
///
/// The caller (proc.rs's scheduler) must hold the PROC_TABLE-locked
/// switch invariant: the frame names a live Ureg on the target proc's
/// kernel stack, that proc's PML4 is the active CR3, and the TSS rsp0
/// names that same stack.
#[unsafe(link_section = ".trap")]
#[unsafe(naked)]
pub unsafe extern "C" fn trap_return(frame: *mut Ureg) -> ! {
    naked_asm!(
        r#"
        movq %rdi, %rsp;
        // Ring-3 target?  The saved CS selector's low bits (UREG_CS at
        // 19*8, byte 0) carry the RPL — swap back to the user GS base
        // only for a CPL3 resume (the entry paths left the kernel base
        // loaded; a kernel resume keeps it).
        testb $3, 19*8(%rsp);
        je 1f;
        swapgs;
        1:
        // The GP half — true user values, from the frame.
        movq 0*8(%rsp), %rax;
        movq 1*8(%rsp), %rbx;
        movq 2*8(%rsp), %rcx;
        movq 3*8(%rsp), %rdx;
        movq 4*8(%rsp), %rsi;
        movq 5*8(%rsp), %rdi;
        movq 6*8(%rsp), %rbp;
        movq 7*8(%rsp), %r8;
        movq 8*8(%rsp), %r9;
        movq 9*8(%rsp), %r10;
        movq 10*8(%rsp), %r11;
        movq 11*8(%rsp), %r12;
        movq 12*8(%rsp), %r13;
        movq 13*8(%rsp), %r14;
        movq 14*8(%rsp), %r15;
        // The packed segment quartet (frame 120..126).
        movw 15*8+0(%rsp), %ds;
        movw 15*8+2(%rsp), %es;
        movw 15*8+4(%rsp), %fs;
        movw 15*8+6(%rsp), %gs;
        // Skip the carve + type + error (18 qwords) to the hardware
        // half; iretq pops ip/cs/flags, plus sp/ss when the target is
        // ring 3 (every frame this era).
        addq $(18 * 8), %rsp;
        iretq;
        "#,
        options(att_syntax))
}

/// The scheduler's idle wait (proc.rs's sched_loop idle arm): briefly
/// unmask, `hlt` (the tick fires and re-arms), re-mask.  `sti`'s
/// one-instruction shadow makes the sti/hlt pair atomic — no interrupt
/// window opens before the halt parks (the canonical x86 idle idiom).
pub fn idle_wait() {
    // SAFETY: sti/hlt/cli carry no memory operands (sti/cli DO touch
    // EFLAGS.IF, so no preserves_flags claim); the caller contract
    // (the sched idle arm — a handler for every armed source exists)
    // is the ordering discipline.
    unsafe { core::arch::asm!("sti", "hlt", "cli", options(nostack)) };
}

/// Park the CPU forever (the clean-halt stop — the aarch64
/// park_forever role: syscall's all-exited halt, trap()'s dump-and-halt
/// and the fault paths all converge here).  IF stays masked: only an
/// NMI can wake it, and nothing this era arms one that returns.
pub fn park_forever() -> ! {
    loop {
        // SAFETY: cli;hlt parks; no memory operands.
        unsafe { core::arch::asm!("cli", "hlt", options(nostack)) };
    }
}

#[inline(always)]
fn intrstatus() -> IntrStatus {
    let flags = cpu::flags();
    if flags.intr() { IntrStatus::Enabled } else { IntrStatus::Disabled }
}

pub fn spllo() -> IntrStatus {
    let prev_level = intrstatus();
    cpu::sti();
    prev_level
}

pub fn splhi() -> IntrStatus {
    let prev_level = intrstatus();
    cpu::cli();
    prev_level
}

pub fn splx(x: IntrStatus) -> IntrStatus {
    match x {
        IntrStatus::Disabled => splhi(),
        IntrStatus::Enabled => spllo(),
    }
}

extern "C" fn trap(vector: u8, trap_frame: &mut Ureg) -> u32 {
    // Hardware interrupts (vectors >= 32): dispatch, never the
    // exception dump — the timer alone fires these until 13-05's
    // IOAPIC lines arrive.  Under dev_exec the return may have been
    // rescheduled onto another proc's frame (the tick's preempt) — the
    // frame this entry built IS the one the iretq tail restores (each
    // proc's frame sits on its own kstack), and the note boundary runs
    // in the syscall entry path.
    if vector >= 32 {
        dispatch_irq(vector);
        return 0;
    }
    // A ring-3-origin synchronous exception under dev_exec: the user
    // image faulted (this era has no trap-context pager — exec maps
    // text/data/bss eagerly, so an unhandled user fault is a user bug,
    // never a kernel one).  The donor faultnote shape: name it, then
    // kill the process loudly via the exit tail — never the whole-
    // kernel halt a kernel-origin fault still takes below.
    #[cfg(dev_exec)]
    {
        if trap_frame.cs & 3 == 3 {
            // CR2 in the print (the donor faultnote's addr field): a
            // #PF's faulting address is the first fact a post-mortem
            // needs — `pc` alone cannot distinguish a NULL deref from
            // a wild pointer (14-06's chain bring-up needed exactly
            // this to place boot(8)'s _tos read).
            let cr2 = {
                let v: u64;
                // SAFETY: a register read with no memory effect.
                unsafe { core::arch::asm!("mov %cr2, %rax", out("rax") v, options(att_syntax, nomem, nostack, preserves_flags)) };
                v
            };
            crate::println!(
                "user trap: unhandled vector {vector} pc={:#x} addr={:#x}",
                trap_frame.pc,
                cr2
            );
            match crate::proc::current() {
                Some(mut p) => crate::syscall::exit_tail(&mut p, b"sys: trap"),
                None => crate::println!("user trap with no current process"),
            }
        }
    }
    // Synchronous exception (0..=31): dump once, then park.  Returning
    // would re-execute the faulting instruction and loop the dump
    // forever (observed live at 13-03 bring-up: a 6 MiB serial storm
    // from one #GP) — the dump-and-park is the aarch64 trap() doctrine,
    // and the boot-test gate then fails with a small readable capture
    // instead of a log flood.
    crate::println!("trap {vector:?}", vector = crate::dat::Trap::from(vector));
    match vector {
        2 => {}
        8 => {}
        _ => crate::println!("frame: {trap_frame:#x?}"),
    }
    crate::println!("kernel halted");
    loop {
        // SAFETY: hlt faults nowhere and parks until an interrupt; the
        // interrupt path iretq's back here (an exception at boot is
        // terminal, the halt is the point).
        unsafe { core::arch::asm!("hlt") };
    }
}

/// IRQ dispatch, the aarch64 dispatch_irq shape re-keyed: the entry
/// stub encoded the vector (the stub array's return-address arithmetic
/// in intrcommon); the registered handler runs FIRST, THEN the LAPIC
/// EOI — device work before the acknowledge, never EOI-first (a
/// level-triggered line still asserted would re-deliver immediately:
/// the read-to-ack-before-EOI ordering Pitfall 5 pins for the
/// virtio INTx lines that arrive in 13-05).  Strays print and still
/// EOI — an EOI-less stray would wedge the LAPIC's in-service state.
/// The spurious vector skips the EOI per the LAPIC spec (it was never
/// in service).
fn dispatch_irq(vector: u8) {
    if vector == crate::platform::q35::lapic::SPURIOUS_VECTOR {
        return;
    }
    match vector {
        TIMER_VECTOR => crate::platform::q35::timer::tick(),
        // The virtio-net device line (dev_ether builds): guarded on the
        // vector the bind published — zero hardcoded vectors (the
        // aarch64 virtio_irq_intid guard re-keyed; the discovery cell
        // carries the GSI, and the bind derives the IDT vector and
        // publishes it).  None until a bind ran: the arm never matches
        // and a stray on the line prints below.
        #[cfg(dev_ether)]
        v if crate::platform::q35::pci::virtio_irq_vector() == Some(v) => {
            crate::platform::q35::pci::virtio_net_irq();
        }
        _ => crate::println!("unhandled IRQ {vector}"),
    }
    crate::platform::q35::lapic::eoi();
}
