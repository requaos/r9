use core::fmt;

#[cfg(platform = "virt")]
use crate::platform;
use crate::fault;
use crate::proc;
use crate::registers::{EsrEl1, ExceptionClass};
use crate::syscall;
use aarch64_cpu::registers::FAR_EL1;
use aarch64_cpu::registers::Readable;
use port::println;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    /// 04-05: exec's first EL0 entry — eret into EL0 with the seeded
    /// forkret-shaped frame (the ONLY way EL0 is ever entered; swtch never
    /// enters EL0).  Defined in trap.S (symbol trap_return_exec): moves the
    /// frame pointer into sp and rides the shared restore tail, so the
    /// seeded frame's psr=0 (EL0t) selects the SP_EL0 write.
    #[link_name = "trap_return_exec"]
    pub fn trap_return(frame: *mut TrapFrame) -> !;
}

pub fn init() {
    #[cfg(not(test))]
    unsafe {
        // Set up a vector table for any exception that is taken to EL1
        core::arch::asm!(
            "adr {tmp}, exception_vectors",
            "msr vbar_el1, {tmp}",
            tmp = out(reg) _,
        );
    }
}

/// Unmask IRQs (DAIFClr #2).  Must only be called once the platform has armed
/// its interrupt handlers: an IRQ taken before then hits the "Unhandled
/// interrupt" spin in trap() and wedges the kernel (research P5).  main9 calls
/// this immediately after init() on raspi — the point where the combined
/// trap::init always unmasked — while virt defers it until its GIC, timer and
/// UART RX handlers are armed (plan 03-05).
pub fn unmask_irq() {
    #[cfg(not(test))]
    unsafe {
        core::arch::asm!("msr DAIFClr, #2");
    }
}

/// Register frame at the time the trap was taken.  This struct IS Harvey's
/// `Ureg` (`aarch64/include/ureg.h`) field-for-field — the layout is the
/// published Harvey↔r9 trap-frame ABI (aarch64/ABI.md, "Kernel-side
/// enforcement (Phase 4 appendix)"), pinned by the const asserts below
/// (always-on) and the xtask `ureg-check` cross-repo gate (D-06).
///
/// ESR_EL1/FAR_EL1 are deliberately NOT frame fields: trap() reads them live
/// at entry and passes them to dispatchers as arguments (D-05) — a
/// stale-frame syndrome read cannot silently corrupt fault decode.
#[repr(C, align(16))]
pub struct TrapFrame {
    /// Syscall-era access (04-06): syscall.rs reads the wire number and
    /// writes the result here (`frame.x0`), per the published ABI (result
    /// in R0).  pub(crate) — the const asserts still pin the layout.
    pub(crate) x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
    x8: u64,
    x9: u64,
    x10: u64,
    x11: u64,
    x12: u64,
    x13: u64,
    x14: u64,
    x15: u64,
    x16: u64,
    x17: u64,
    x18: u64,
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    frame_pointer: u64, // x29
    link_register: u64, // x30
    /// Interrupted context's stack pointer (SP_EL0 on EL0 traps, entry-sp on
    /// EL1 traps).  pub(crate) for the 04-06 syscall arg-read formula
    /// (`frame.sp + 8` — aarch64/ABI.md Phase-4 appendix).
    pub(crate) sp: u64,
    pc: u64, // ELR_EL1 — interrupted address
    psr: u64, // SPSR_EL1
    interrupt_type: u64, // Harvey ureg->type: r9 stores the vector index 0-15 (kernel-side informational)
}

// D-06 layer 1: pinned byte offsets from aarch64/ABI.md's Phase-4 appendix.
// Always on — any layout drift fails `cargo build`, no Harvey checkout
// needed (layer 2 is the xtask ureg-check against the header itself).
const _: () = assert!(core::mem::offset_of!(TrapFrame, x0) == 0);
const _: () = assert!(core::mem::offset_of!(TrapFrame, x28) == 224);
const _: () = assert!(core::mem::offset_of!(TrapFrame, link_register) == 240);
const _: () = assert!(core::mem::offset_of!(TrapFrame, sp) == 248);
const _: () = assert!(core::mem::offset_of!(TrapFrame, pc) == 256);
const _: () = assert!(core::mem::offset_of!(TrapFrame, psr) == 264);
const _: () = assert!(core::mem::offset_of!(TrapFrame, interrupt_type) == 272);
const _: () = assert!(core::mem::size_of::<TrapFrame>() == 288);

/// The frozen `sizeof(Ureg)` (Harvey aarch64/ABI.md, field-for-field with
/// ureg.h): 35 × u64 = 280 bytes — NOT r9's 288 frame stride (04-RESEARCH.md
/// Pitfall 6: only FIELD OFFSETS cross the ABI).  The note-delivery path
/// pushes/copies exactly this many bytes (04-08).
pub(crate) const UREG_SIZE: usize = 280;

/// Plan 9 note actions (Harvey sys/include/libc.h — frozen values).
pub(crate) const NCONT: u64 = 0;
pub(crate) const NDFLT: u64 = 1;
pub(crate) const NSAVE: u64 = 2;
pub(crate) const NRSTR: u64 = 3;

impl TrapFrame {
    /// A zeroed frame — the exec seed base (04-04's exec.rs fills x0/sp/pc
    /// via seed_el0_entry; everything else stays zero).
    pub const fn zeroed() -> TrapFrame {
        TrapFrame {
            x0: 0,
            x1: 0,
            x2: 0,
            x3: 0,
            x4: 0,
            x5: 0,
            x6: 0,
            x7: 0,
            x8: 0,
            x9: 0,
            x10: 0,
            x11: 0,
            x12: 0,
            x13: 0,
            x14: 0,
            x15: 0,
            x16: 0,
            x17: 0,
            x18: 0,
            x19: 0,
            x20: 0,
            x21: 0,
            x22: 0,
            x23: 0,
            x24: 0,
            x25: 0,
            x26: 0,
            x27: 0,
            x28: 0,
            frame_pointer: 0,
            link_register: 0,
            sp: 0,
            pc: 0,
            psr: 0,
            interrupt_type: 0,
        }
    }

    /// Seed the EL0-entry shape (9front execregs + touser: R0 = tos pointer,
    /// SP → the seeded user stack [argc][argv0]…[nil], PC = entry, PSR = 0
    /// (EL0t), LR = 0).  trap.S's restore path writes SP_EL0/ELR/SPSR from
    /// exactly these slots on the EL0-bound return (psr & 0xF == 0), so the
    /// seeded frame erets into EL0 with no special-case asm.
    pub fn seed_el0_entry(&mut self, tos_va: u64, user_sp: u64, entry: u64) {
        self.x0 = tos_va;
        self.sp = user_sp;
        self.pc = entry;
        self.psr = 0;
        self.link_register = 0;
    }

    /// The interrupted/interrupted-to-be PC — the fault handler prints it in
    /// its unresolvable marker.
    pub fn pc(&self) -> u64 {
        self.pc
    }

    /// The interrupted context's stack pointer (SP_EL0 on EL0 traps) — the
    /// exec entry-evidence print mirrors pc() (04-04's precedent for adding
    /// minimal getters as consumers need them).
    pub fn sp(&self) -> u64 {
        self.sp
    }

    /// The saved program status (SPSR_EL1) — the origin EL lives in the low
    /// nibble (0 = EL0t), which the kernel-context-fault panic prints.
    pub fn psr(&self) -> u64 {
        self.psr
    }

    /// Force the frame's saved psr to EL0t (clear the M field, bits [3:0]).
    /// WR-04: NOTED's NCONT/NRSTR copies the saved Ureg over the live frame
    /// from USER-WRITABLE memory — a note handler may legally edit it (the
    /// documented notejmp contract) — and trap.S's restore writes the psr
    /// straight to SPSR_EL1 before eret.  The privilege bits are kernel
    /// state crossing the trust boundary: without this mask a handler that
    /// set M = EL1h (0x5) would eret to EL1 at a (PXN-protected) user pc —
    /// a user-triggerable kernel panic.  The donor's noted() validates
    /// pc/sp; this closes the restore boundary.
    pub(crate) fn force_psr_el0t(&mut self) {
        self.psr &= !0xf;
    }
}

impl fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrapFrame")
            .field("x0", &format_args!("{:#018x}", self.x0))
            .field("x1", &format_args!("{:#018x}", self.x1))
            .field("x2", &format_args!("{:#018x}", self.x2))
            .field("x3", &format_args!("{:#018x}", self.x3))
            .field("x4", &format_args!("{:#018x}", self.x4))
            .field("x5", &format_args!("{:#018x}", self.x5))
            .field("x6", &format_args!("{:#018x}", self.x6))
            .field("x7", &format_args!("{:#018x}", self.x7))
            .field("x8", &format_args!("{:#018x}", self.x8))
            .field("x9", &format_args!("{:#018x}", self.x9))
            .field("x10", &format_args!("{:#018x}", self.x10))
            .field("x11", &format_args!("{:#018x}", self.x11))
            .field("x12", &format_args!("{:#018x}", self.x12))
            .field("x13", &format_args!("{:#018x}", self.x13))
            .field("x14", &format_args!("{:#018x}", self.x14))
            .field("x15", &format_args!("{:#018x}", self.x15))
            .field("x16", &format_args!("{:#018x}", self.x16))
            .field("x17", &format_args!("{:#018x}", self.x17))
            .field("x18", &format_args!("{:#018x}", self.x18))
            .field("x19", &format_args!("{:#018x}", self.x19))
            .field("x20", &format_args!("{:#018x}", self.x20))
            .field("x21", &format_args!("{:#018x}", self.x21))
            .field("x22", &format_args!("{:#018x}", self.x22))
            .field("x23", &format_args!("{:#018x}", self.x23))
            .field("x24", &format_args!("{:#018x}", self.x24))
            .field("x25", &format_args!("{:#018x}", self.x25))
            .field("x26", &format_args!("{:#018x}", self.x26))
            .field("x27", &format_args!("{:#018x}", self.x27))
            .field("x28", &format_args!("{:#018x}", self.x28))
            // Bare register names, not "x29 (frame_pointer)": the boot-test
            // panic profile counts dump rows by the `x<N>:` shape and must
            // see all of x0..x30 (the roles live on the struct fields).
            .field("x29", &format_args!("{:#018x}", self.frame_pointer))
            .field("x30", &format_args!("{:#018x}", self.link_register))
            .field("sp", &format_args!("{:#018x}", self.sp))
            .field("pc", &format_args!("{:#018x}", self.pc))
            .field("psr", &format_args!("{:#018x}", self.psr))
            .field("interrupt_type", &format_args!("{}", self.interrupt_type))
            .finish()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn trap_unsafe(frame: *mut TrapFrame) {
    unsafe { trap(frame.as_mut().unwrap()) }
}

// Interrupt-type constants, matching the .equ block in trap.S (lines 17-35).
#[cfg(platform = "virt")]
const IRQ_INVALID_EL1T: u64 = 1;
#[cfg(platform = "virt")]
const IRQ_INVALID_EL1H: u64 = 5;
/// IRQ while at EL0 (64-bit) — arrives once a process runs under dev_exec
/// (the timer tick preempts userland).  Inert elsewhere: with no EL0 in
/// existence the vector never fires.
#[cfg(platform = "virt")]
const IRQ_INVALID_EL0_64: u64 = 9;

/// IRQ dispatch (virt): ack via the GIC, dispatch, EOI, and RETURN -- trap.S's
/// shared trap_return tail erets back to the interrupted context (the wfi
/// idle loop at EL1, or userland at EL0 under dev_exec — the unified frame
/// save/restore makes both correct).  The spurious INTID 1023 is acked but
/// never EOI'd; any other unknown INTID is logged once and still EOI'd so a
/// stray interrupt can't wedge the kernel.  The INTID 33 (PL011 RX) arm
/// lands with its handler.
#[cfg(platform = "virt")]
fn dispatch_irq(frame: &TrapFrame) -> bool {
    if frame.interrupt_type != IRQ_INVALID_EL1T
        && frame.interrupt_type != IRQ_INVALID_EL1H
        && frame.interrupt_type != IRQ_INVALID_EL0_64
    {
        return false;
    }

    let intid = platform::gic::ack();
    match intid {
        // ARM generic timer CNTP (PPI 14 -> INTID 16+14)
        30 => platform::timer::tick(),
        // PL011 UART RX (SPI 1 -> INTID 32+1): echo in the Phase-3 profile,
        // devcons read-buffer feed under dev_exec (04-06)
        33 => platform::pl011::rx_irq(),
        platform::gic::SPURIOUS_INTID => {} // spurious: acked, no EOI
        _ => println!("unhandled IRQ {intid}"),
    }
    if intid != platform::gic::SPURIOUS_INTID {
        platform::gic::eoi(intid);
    }
    true
}

/// Raspi arms no interrupt handlers this phase, so every IRQ keeps the
/// historical dump-and-spin behavior below.
#[cfg(not(platform = "virt"))]
fn dispatch_irq(_frame: &TrapFrame) -> bool {
    false
}

/// 04-08 (D-16): note delivery at the return-to-EL0 boundary.  Called on
/// every return to EL0 — after syscall dispatch AND after a handled IRQ —
/// with the frame the shared trap_return tail is about to restore (the
/// caller resolved CURFRAME and checked the frame is EL0t-bound).  Pops
/// the current proc's pending note and redirects the frame into the
/// notify()-installed handler, donor notify() verbatim (9front@350266104f
/// sys/src/9/arm64/trap.c:209-238):
///
///   sp = ureg->sp - 256 (debug preserve) - sizeof(Ureg), aligned ~7
///   nureg = (Ureg*)sp; memmove(nureg, ureg, sizeof(Ureg));
///   sp -= BY2WD+ERRMAX; memmove((char*)sp, msg, ERRMAX);
///   sp -= 3*BY2WD;
///   [sp+16] = &msg (arg1 home); [sp+8] = nureg (arg0 home);
///   ureg->r0 = nureg; ureg->sp = sp; ureg->pc = handler; ureg->link = 0;
///
/// The pushed Ureg copy is the frozen sizeof(Ureg)=280 bytes — NOT r9's
/// 288 frame stride (Pitfall 6's field-offsets-only crossing rule).
/// libc's handler signature void f(void *ureg, char *msg) is satisfied
/// exactly: R0 + the two stack arg homes per the arm64 convention.
///
/// T-04-16 (the donor okaddr port): the WHOLE write window
/// [final_sp, nureg+280) is validated user-writable BEFORE any write and
/// its pages are forced resident (an in-segment-but-unmapped page would
/// EL1h-abort mid-delivery); an unwritable/misaligned target kills the
/// process loudly with the note printed — never a partial write, never a
/// kernel write to a bad user VA.  No handler installed → print the note
/// + kill via the exits path (the donor donotify pexit shape).
pub(crate) fn deliver_notes(frame: &mut TrapFrame) {
    let proc_ptr = proc::current_proc();
    if proc_ptr.is_null() {
        return;
    }
    let proc = unsafe { &mut *proc_ptr };
    let Some(note) = proc.note_queue.pop_front() else { return };
    proc.last_note = note.buf;
    proc.last_note_len = note.len;
    let buf = note.buf; // owned NUL-padded copy (the kill paths borrow proc)
    let msg = &buf[..note.len];

    let Some(handler) = proc.notify_handler else {
        println!("note: pid {} has no notify handler", proc.pid);
        syscall::exit_tail(proc, msg);
    };
    if handler & 3 != 0 {
        println!("note: pid {} misaligned notify handler", proc.pid);
        syscall::exit_tail(proc, msg);
    }

    // The donor stack construction (verbatim arithmetic; wrapping_sub so a
    // pathological frame.sp can't panic before the validation).
    let nureg = frame.sp.wrapping_sub(256 + UREG_SIZE as u64) & !7u64;
    let msg_va = nureg.wrapping_sub(8 + proc::ERRMAX as u64);
    let final_sp = msg_va.wrapping_sub(3 * 8);
    let window_end = nureg + UREG_SIZE as u64;

    // T-04-16: validate the whole write window user-WRITABLE before any
    // write (never a partial write), then force its stack pages resident
    // (membership is a segment check — the pages may be unmapped).
    if proc.validate_user_range(final_sp, window_end - final_sp, true).is_err() {
        println!("note: pid {} undeliverable (bad user stack window)", proc.pid);
        syscall::exit_tail(proc, msg);
    }
    fault::ensure_stack_resident(proc, final_sp as usize, window_end as usize);

    unsafe {
        // The live frame's 35 fields into the user nureg (280 bytes).
        core::ptr::copy_nonoverlapping(
            frame as *const TrapFrame as *const u8,
            nureg as *mut u8,
            UREG_SIZE,
        );
        // The note bytes at nureg - (8+128): the full ERRMAX buffer
        // (NUL-padded — the handler prints the string).
        core::ptr::copy_nonoverlapping(buf.as_ptr(), msg_va as *mut u8, proc::ERRMAX);
        // The arg homes: [sp+8] = nureg (arg0), [sp+16] = &msg (arg1).
        ((final_sp + 8) as *mut u64).write(nureg);
        ((final_sp + 16) as *mut u64).write(msg_va);
    }
    frame.x0 = nureg;
    frame.sp = final_sp;
    frame.pc = handler;
    frame.link_register = 0;
    // The donor's up->noteureg: NOTED's NCONT/NRSTR restore source.
    proc.note_ureg = nureg;
}

fn trap(frame: &mut TrapFrame) {
    // D-05: ESR_EL1/FAR_EL1 are read LIVE at trap entry and flow as
    // arguments to the dispatchers — never frame fields.  04-04's fault
    // handler receives them as fault::user_fault(frame, esr, far); the IRQ
    // path never needs them.
    let esr = EsrEl1::read();
    let far = FAR_EL1.get();

    if dispatch_irq(frame) {
        // 04-08 note-delivery boundary (IRQ return path): the tick may
        // have preempted mid-trap — the proc about to return to EL0 is the
        // CURRENT one and its frame is CURFRAME's, not necessarily the
        // frame this entry built.  Deliver ITS pending notes into the
        // frame the tail actually restores (guarded: EL0t-bound only).
        let cf = proc::current_frame();
        if !cf.is_null() {
            let cf = unsafe { &mut *cf };
            if cf.psr & 0xF == 0 {
                deliver_notes(cf);
            }
        }
        return;
    }

    if esr.ec() == 0x15 {
        // SVC (04-06): the real dispatch.  The WIRE syscall number is R0
        // (frame.x0), args sit at user SP+8 in 8-byte slots, and the result
        // is written back to frame.x0 (aarch64/ABI.md).  Dispatch itself
        // prints `syscall N unimplemented` + returns -1 for numbers outside
        // the D-14 measured set + named core — loud, never wedged.  EXITS
        // diverges (parks); every other path returns here and the shared
        // trap_return tail erets back into EL0.
        crate::syscall::dispatch(frame);
        // 04-08 note-delivery boundary (syscall return path): dispatch
        // runs IRQ-masked so no preemption happened — `frame` IS the
        // returning proc's live frame (== CURFRAME).
        if frame.psr & 0xF == 0 {
            deliver_notes(frame);
        }
        return;
    } else if matches!(
        esr.exception_class_enum(),
        Ok(ExceptionClass::InstructionAbortLowerEl) | Ok(ExceptionClass::DataAbortLowerEl)
    ) {
        // EC 0x20/0x24: lower-EL instruction/data abort — the demand-paging
        // path (D-09).  Returns with the fault resolved and the restore path
        // erets back into EL0; an unresolvable fault kills the process and
        // parks inside the handler (single-process era).
        fault::user_fault(frame, esr, far);
        return;
    } else if frame.psr & 0xF == 0 {
        // Other EL0-origin sync (EL0t psr) — the donor faultnote shape: name
        // it, then kill the process loudly (a handlerless process dies,
        // never hangs).
        println!(
            "user trap: unhandled ec={:#04x} addr={far:#018x} pc={:#018x}",
            esr.ec(),
            frame.pc
        );
        fault::kill_current_process();
    } else {
        println!("{:#?}", frame);
        // ESR/FAR print from the live trap() locals (D-08) — outside the
        // frame — so the panic profile's x0..x30 row-count gate is untouched.
        println!("esr_el1: {esr:#?}");
        println!("far_el1: {far:#018x}");
        println!("Unhandled interrupt");
        // Deterministic clean halt (SC#5/D-12): the full register state is on
        // the console, then the marker, then park in wfi -- a wedge-free stop
        // that doesn't burn the host on a spin.  Applies to every platform;
        // raspi's dump path gains the same marker (an improvement).
        println!("kernel halted");
    }

    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
