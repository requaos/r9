use core::fmt;

#[cfg(platform = "virt")]
use crate::platform;
use crate::fault;
use crate::proc;
use crate::registers::EsrEl1;
use crate::syscall;
use aarch64_cpu::registers::FAR_EL1;
use aarch64_cpu::registers::Readable;
use port::println;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    /// exec's first EL0 entry — eret into EL0 with the seeded
    /// forkret-shaped frame (the ONLY way EL0 is ever entered; swtch never
    /// enters EL0).  Defined in trap.S (symbol trap_return_exec): moves the
    /// frame pointer into sp and rides the shared restore tail, so the
    /// seeded frame's psr=0 (EL0t) selects the SP_EL0 write.
    #[link_name = "trap_return_exec"]
    pub fn trap_return(frame: *mut TrapFrame) -> !;
}

pub fn init() {
    #[cfg(not(test))]
    // SAFETY: installing VBAR_EL1 with the address of trap.S's
    // exception_vectors (a valid, aligned vector table in kernel text) is
    // the trap-interface floor — a pure register write at EL1.
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
/// interrupt" spin in trap() and wedges the kernel.  main9 calls
/// this immediately after init() on raspi — the point where the combined
/// trap::init always unmasked — while virt defers it until its GIC, timer and
/// UART RX handlers are armed.
pub fn unmask_irq() {
    #[cfg(not(test))]
    // SAFETY: clearing DAIF.I is a PSTATE write — the caller contract
    // (handlers armed first, the doc comment) is the ordering discipline;
    // the instruction itself is always executable at EL1.
    unsafe {
        core::arch::asm!("msr DAIFClr, #2");
    }
}

/// Park the CPU forever (the clean-halt stop — the ONE wfi-halt home;
/// fault.rs's no-current-process park, syscall.rs's all-exited halt,
/// main.rs's pre-exec idle loop and trap()'s dump-and-halt all converge
/// here).
pub fn park_forever() -> ! {
    loop {
        // SAFETY: wfi parks the CPU until an interrupt; with IRQs masked
        // (trap/syscall context) this is a wedge-free stop.
        unsafe { core::arch::asm!("wfi") };
    }
}

/// The scheduler's idle wait (proc.rs's
/// sched_loop idle arm): briefly UNMASK IRQs, wfi (the tick fires and
/// re-arms), re-mask.  The sequence is atomic in intent: nothing else runs
/// between the unmask and the wfi on this CPU.
pub fn idle_wait() {
    // SAFETY: the DAIFClr/wfi/DAIFSet sequence is the sched idle arm's
    // sleep shape — a pure PSTATE + halt-until-interrupt sequence, no
    // memory operands.
    unsafe { core::arch::asm!("msr DAIFClr, #2", "wfi", "msr DAIFSet, #2") };
}

/// The deliberate sync exception (the dev_panic config's hook) —
/// raises a brk #0 through the REAL trap path.
#[cfg(dev_panic)]
pub fn debug_sync_trap() {
    // SAFETY: brk #0 raises a synchronous exception routed through the
    // real trap path — the deliberate debug trigger (never a memory
    // hazard).
    unsafe { core::arch::asm!("brk #0") };
}

/// Register frame at the time the trap was taken.  This struct IS Harvey's
/// `Ureg` (`arm64/include/ureg.h`) field-for-field — the layout is the
/// published Harvey↔r9 trap-frame ABI (aarch64/ABI.md's kernel-side
/// enforcement appendix), pinned by the const asserts below
/// (always-on) and the xtask `ureg-check` cross-repo gate.
///
/// ESR_EL1/FAR_EL1 are deliberately NOT frame fields: trap() reads them live
/// at entry and passes them to dispatchers as arguments — a
/// stale-frame syndrome read cannot silently corrupt fault decode.
#[repr(C, align(16))]
pub struct TrapFrame {
    /// Syscall access: syscall.rs reads the wire number and
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
    /// EL1 traps).  pub(crate) for the syscall arg-read formula
    /// (`frame.sp + 8` — aarch64/ABI.md).
    pub(crate) sp: u64,
    pc: u64, // ELR_EL1 — interrupted address
    psr: u64, // SPSR_EL1
    interrupt_type: u64, // Harvey ureg->type: r9 stores the vector index 0-15 (kernel-side informational)
}

// Pinned byte offsets from aarch64/ABI.md's kernel-side enforcement
// appendix.
// Always on — any layout drift fails `cargo build`, no Harvey checkout
// needed (the xtask ureg-check covers the header itself).
const _: () = assert!(core::mem::offset_of!(TrapFrame, x0) == 0);
const _: () = assert!(core::mem::offset_of!(TrapFrame, x28) == 224);
const _: () = assert!(core::mem::offset_of!(TrapFrame, link_register) == 240);
const _: () = assert!(core::mem::offset_of!(TrapFrame, sp) == 248);
const _: () = assert!(core::mem::offset_of!(TrapFrame, pc) == 256);
const _: () = assert!(core::mem::offset_of!(TrapFrame, psr) == 264);
const _: () = assert!(core::mem::offset_of!(TrapFrame, interrupt_type) == 272);
const _: () = assert!(core::mem::size_of::<TrapFrame>() == 288);

/// The frozen `sizeof(Ureg)` (Harvey aarch64/ABI.md, field-for-field with
/// ureg.h): 35 × u64 = 280 bytes — NOT r9's 288 frame stride (only FIELD
/// OFFSETS cross the ABI).  The note-delivery path pushes/copies exactly
/// this many bytes.
pub(crate) const UREG_SIZE: usize = 280;

impl TrapFrame {
    /// deliver_notes's frame push: the live frame's 35
    /// fields (280 bytes, the frozen sizeof(Ureg)) into the validated,
    /// resident user window.  Homed here with
    /// restore_from_saved so trap.rs carries no frame-reinterpretation
    /// unsafe at its call sites.
    pub(crate) fn copy_to_saved(&self, dst: &mut [u8]) {
        assert_eq!(dst.len(), UREG_SIZE, "saved ureg size");
        // SAFETY: self is a live 288-byte TrapFrame (288 > 280 — exactly
        // 280 bytes are read, never overrunning); dst is validated +
        // resident (usermem's constructor invariant).  Overlap is
        // impossible (src is kernel, dst is user).
        unsafe {
            core::ptr::copy_nonoverlapping(
                self as *const TrapFrame as *const u8,
                dst.as_mut_ptr(),
                UREG_SIZE,
            );
        }
    }

    /// NOTED's restore (NCONT, NRSTR, NSAVE): overwrite the live frame
    /// with the saved Ureg's 280 bytes (the frozen sizeof(Ureg), NOT r9's
    /// 288 stride — the frame's own stride is untouched).  Homed here so
    /// syscall.rs carries no frame-reinterpretation unsafe; the source is
    /// a usermem-validated resident slice.
    ///
    /// The saved Ureg is USER-WRITABLE (the notejmp contract lets a
    /// handler edit it) and trap.S writes the frame's psr straight to
    /// SPSR_EL1 before the eret, so the psr is rebuilt here, the one place
    /// user bytes enter a frame: see user_return_psr.
    pub(crate) fn restore_from_saved(&mut self, src: &[u8]) {
        assert_eq!(src.len(), UREG_SIZE, "saved ureg size");
        let live_psr = self.psr;
        // SAFETY: self is a live 288-byte TrapFrame (288 > 280 — the copy
        // writes exactly 280 bytes, never overflowing the frame); src is
        // validated + resident (usermem's constructor invariant) and read
        // for exactly its length.  Overlap is impossible (src is user
        // memory, dst is kernel).
        unsafe {
            core::ptr::copy_nonoverlapping(
                src.as_ptr(),
                self as *mut TrapFrame as *mut u8,
                UREG_SIZE,
            );
        }
        self.psr = user_return_psr(self.psr, live_psr);
    }
}

/// SPSR bits a note handler may set through the saved Ureg: the
/// condition flags N, Z, C, V (bits 31..28).
const USER_PSR: u64 = 0xF000_0000;

/// SPSR bits that must be clear on a return to EL0t, whatever their
/// source: M[4:0] (AArch64, EL0t), D A I F (bits 9..6), IL (20), SS (21).
const EL0T_RETURN_CLEAR: u64 = 0x1F | (0xF << 6) | (1 << 20) | (1 << 21);

/// The SPSR NOTED returns to EL0 with: the condition flags from the
/// user-writable saved Ureg (`saved`), every other bit from `live` — the
/// SPSR the NOTED syscall itself entered with, captured by the hardware
/// — and the privileged bits forced to a plain EL0t return.  The donor
/// rule is "don't let user change system flags" (9k noted() keeps only
/// the arithmetic flags of the user's Ureg and takes the rest from the
/// kernel's frame).  A handler that set I or F would otherwise run its
/// process at EL0 with interrupts masked, where the tick can never
/// preempt it; M would steer the eret to EL1 or AArch32, and IL or SS
/// would raise an illegal-state or single-step exception on return.
fn user_return_psr(saved: u64, live: u64) -> u64 {
    ((saved & USER_PSR) | (live & !USER_PSR)) & !EL0T_RETURN_CLEAR
}

/// Plan 9 note actions (Harvey sys/include/libc.h — frozen values).
pub(crate) const NCONT: u64 = 0;
pub(crate) const NDFLT: u64 = 1;
pub(crate) const NSAVE: u64 = 2;
pub(crate) const NRSTR: u64 = 3;

impl TrapFrame {
    /// A zeroed frame — the exec seed base (exec.rs fills x0/sp/pc
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
    /// exec entry-evidence print mirrors pc().
    pub fn sp(&self) -> u64 {
        self.sp
    }

    /// The saved program status (SPSR_EL1) — the origin EL lives in the low
    /// nibble (0 = EL0t), which the kernel-context-fault panic prints.
    pub fn psr(&self) -> u64 {
        self.psr
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
    // SAFETY: trap.S (the only caller) passes the saved frame on the
    // current kstack — a live, 288-byte TrapFrame by the entry
    // construction; the unwrap is the ABI proof.
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
        // PL011 UART RX (SPI 1 -> INTID 32+1): the devcons read-buffer
        // feed under dev_exec
        33 => platform::pl011::rx_irq(),
        // virtio-net (dev_ether configs): keyed on the INTID pci.rs
        // published at bind (0 = unbound — the guard never matches, so a
        // stray line falls to the unhandled print); zero hardcoded
        // interrupt numbers (NETDRV-01).  The handler's ISR-byte read
        // deasserts the level line before the EOI below.
        #[cfg(dev_ether)]
        i if i == platform::virt::pci::virtio_irq_intid() && i != 0 => {
            platform::virt::pci::virtio_net_irq()
        }
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

/// Note delivery at the return-to-EL0 boundary.  Called on
/// every return to EL0 from a trap — after syscall dispatch, after a
/// resolved demand fault, and after a handled IRQ — with the frame the
/// shared trap_return tail is about to restore (the caller resolved
/// CURFRAME and checked the frame is EL0t-bound).  Pops
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
/// 288 frame stride (only field offsets cross the ABI).
/// libc's handler signature void f(void *ureg, char *msg) is satisfied
/// exactly: R0 + the two stack arg homes per the arm64 convention.
///
/// The donor okaddr port: the WHOLE write window
/// [final_sp, nureg+280) is validated user-writable BEFORE any write and
/// its pages are forced resident (an in-segment-but-unmapped page would
/// EL1h-abort mid-delivery); an unwritable/misaligned target kills the
/// process loudly with the note printed — never a partial write, never a
/// kernel write to a bad user VA.  No handler installed → print the note
/// + kill via the exits path (the donor donotify pexit shape).
///
/// In-handler guard (the donor donotify `up->notified` check): a
/// second note is NOT delivered while a note handler is running — the
/// note stays queued; noted() clears `notified` and the next
/// return-to-EL0 boundary retries.  After noted(NSAVE) notes are enabled
/// again inside the trampoline, so a note may nest: the word just below
/// each saved Ureg holds the previous saved-Ureg VA (the donor notify's
/// `*(Ureg**)(nureg-BY2WD) = up->noteureg`), and NOTED pops back
/// through it.
pub(crate) fn deliver_notes(frame: &mut TrapFrame) {
    let Some(mut proc) = proc::current() else {
        return;
    };
    // Leave the note queued while a handler is active.
    if proc.notified {
        return;
    }
    let Some(note) = proc.note_queue.pop_front() else { return };
    proc.last_note = note.buf;
    proc.last_note_len = note.len;
    let buf = note.buf; // owned NUL-padded copy (the kill paths borrow proc)
    let msg = &buf[..note.len];

    let Some(handler) = proc.notify_handler else {
        println!("note: pid {} has no notify handler", proc.pid);
        syscall::exit_tail(&mut proc, msg);
    };
    if handler & 3 != 0 {
        println!("note: pid {} misaligned notify handler", proc.pid);
        syscall::exit_tail(&mut proc, msg);
    }

    // The donor stack construction (verbatim arithmetic; wrapping_sub so a
    // pathological frame.sp can't panic before the validation).
    let nureg = frame.sp.wrapping_sub(256 + UREG_SIZE as u64) & !7u64;
    let msg_va = nureg.wrapping_sub(8 + proc::ERRMAX as u64);
    let final_sp = msg_va.wrapping_sub(3 * 8);
    let window_end = nureg + UREG_SIZE as u64;

    // Validate the whole write window user-WRITABLE before any
    // write (never a partial write), then force its stack pages resident
    // (membership is a segment check — the pages may be unmapped).
    if proc.validate_user_range(final_sp, window_end - final_sp, true).is_err() {
        println!("note: pid {} undeliverable (bad user stack window)", proc.pid);
        syscall::exit_tail(&mut proc, msg);
    }
    fault::ensure_stack_resident(&mut proc, final_sp as usize, window_end as usize);

    // The three user-memory writes ride a single usermem
    // window over [final_sp, window_end) — the constructor re-runs the
    // fused validate+resident discipline (a no-op second walk here, the
    // explicit checks above carry the kill paths' messages).
    let prev_ureg = proc.note_ureg;
    let mut window = crate::usermem::UserSliceMut::new(&mut proc, final_sp, window_end - final_sp)
        .expect("note window validated + resident above");
    let w = window.as_slice_mut();
    // The live frame's 35 fields into the user nureg (280 bytes).
    let nureg_off = (nureg - final_sp) as usize;
    frame.copy_to_saved(&mut w[nureg_off..nureg_off + UREG_SIZE]);
    // The note bytes at nureg - (8+128): the full ERRMAX buffer
    // (NUL-padded — the handler prints the string).
    let msg_off = (msg_va - final_sp) as usize;
    w[msg_off..msg_off + proc::ERRMAX].copy_from_slice(&buf);
    // The arg homes: [sp+8] = nureg (arg0), [sp+16] = &msg (arg1).
    window.write_u64(8, nureg);
    window.write_u64(16, msg_va);
    // The chain word just below the saved Ureg (between it and the note
    // bytes): the previous saved Ureg, popped by NOTED NCONT/NRSTR.
    window.write_u64((nureg - 8 - final_sp) as usize, prev_ureg);
    frame.x0 = nureg;
    frame.sp = final_sp;
    frame.pc = handler;
    frame.link_register = 0;
    // The donor's up->noteureg: NOTED's restore source.
    proc.note_ureg = nureg;
    proc.notified = true;
    // ... and the donor's fpunotify: record the interrupted code's
    // FP/SIMD state; the handler runs on a copy of it (the live
    // registers stay as they are) and NOTED gives this one back.
    proc.note_fpsave = crate::fpu::snapshot(&proc.fpsave);
}

/// Where trap() sends a synchronous exception, from ESR_EL1.EC and the
/// interrupted context's SPSR mode bits (M[3:0] = 0 is EL0t).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SyncRoute {
    /// EC 0x15, SVC: syscall::dispatch.
    Syscall,
    /// EC 0x20/0x24, instruction/data abort from EL0: fault::user_fault
    /// (resolves the fault, or kills the process and never returns).
    DemandFault,
    /// Any other EL0-origin exception: named, then the process is killed.
    UserKill,
    /// Anything else (an EL1-origin exception): dump and halt.
    KernelHalt,
}

impl SyncRoute {
    fn of(ec: u8, psr: u64) -> SyncRoute {
        match ec {
            0x15 => SyncRoute::Syscall,
            0x20 | 0x24 => SyncRoute::DemandFault,
            _ if psr & 0xF == 0 => SyncRoute::UserKill,
            _ => SyncRoute::KernelHalt,
        }
    }

    /// The routes that come back to trap() and return to the interrupted
    /// context.  Each is a note boundary when that context is EL0 — the
    /// donor runs notify() at the end of trap() for every user trap
    /// (9front@350266104f sys/src/9/arm64/trap.c), so a note pending when
    /// a demand fault returns is delivered there, not at the next tick or
    /// syscall.
    fn returns(self) -> bool {
        matches!(self, SyncRoute::Syscall | SyncRoute::DemandFault)
    }
}

fn trap(frame: &mut TrapFrame) {
    // ESR_EL1/FAR_EL1 are read LIVE at trap entry and flow as
    // arguments to the dispatchers — never frame fields.  The fault
    // handler receives them as fault::user_fault(frame, esr, far); the IRQ
    // path never needs them.
    let esr = EsrEl1::read();
    let far = FAR_EL1.get();

    if dispatch_irq(frame) {
        // Note-delivery boundary (IRQ return path): the tick may
        // have preempted mid-trap — the proc about to return to EL0 is the
        // CURRENT one and its frame is this CPU's PerCpu cur_frame's, not
        // necessarily the frame this entry built.  Deliver ITS pending
        // notes into the frame the tail actually restores (guarded:
        // EL0t-bound only).
        let cf = proc::current_frame();
        if !cf.is_null() {
            // SAFETY: this CPU's PerCpu cur_frame names the live frame on
            // the returning proc's kstack (written by trap.S at entry on
            // this CPU, or by this CPU's locked scheduler swap); read in
            // IRQ-masked trap context on the owning CPU — no cross-CPU
            // access is possible.
            let cf = unsafe { &mut *cf };
            if cf.psr & 0xF == 0 {
                deliver_notes(cf);
            }
        }
        return;
    }

    let route = SyncRoute::of(esr.ec(), frame.psr);
    match route {
        // SVC: the real dispatch.  The WIRE syscall number is R0
        // (frame.x0), args sit at user SP+8 in 8-byte slots, and the result
        // is written back to frame.x0 (aarch64/ABI.md).  Dispatch itself
        // prints `syscall N unimplemented` + returns -1 for numbers outside
        // the measured set + named core — loud, never wedged.  EXITS
        // diverges (parks); every other path returns here and the shared
        // trap_return tail erets back into EL0.
        SyncRoute::Syscall => crate::syscall::dispatch(frame),
        // EC 0x20/0x24: lower-EL instruction/data abort — the demand-paging
        // path.  Returns with the fault resolved and the restore path
        // erets back into EL0; an unresolvable fault kills the process and
        // never returns.
        SyncRoute::DemandFault => fault::user_fault(frame, esr, far),
        SyncRoute::UserKill => {
            // Other EL0-origin sync (EL0t psr) — the donor faultnote shape:
            // name it, then kill the process loudly (a handlerless process
            // dies, never hangs).
            println!(
                "user trap: unhandled ec={:#04x} addr={far:#018x} pc={:#018x}",
                esr.ec(),
                frame.pc
            );
            fault::kill_current_process();
        }
        SyncRoute::KernelHalt => {
            println!("{:#?}", frame);
            // ESR/FAR print from the live trap() locals — outside the
            // frame — so the panic profile's x0..x30 row-count gate is
            // untouched.
            println!("esr_el1: {esr:#?}");
            println!("far_el1: {far:#018x}");
            println!("Unhandled interrupt");
            // Deterministic clean halt: the full register state is on
            // the console, then the marker, then park in wfi -- a
            // wedge-free stop that doesn't burn the host on a spin.
            // Applies to every platform; raspi's dump path gains the same
            // marker.
            println!("kernel halted");
            // The dump-and-halt parks via the consolidated primitive.
            park_forever()
        }
    }
    // Note-delivery boundary (synchronous return path): syscall dispatch
    // and the demand pager run IRQ-masked and never switch procs, so
    // `frame` IS the returning proc's live frame (== CURFRAME).  A note
    // can be pending at a fault return when the return to EL0 before the
    // fault skipped the boundary (the sched loop resumes a preempted or
    // new proc with a bare trap_return) or, with more than one CPU, when
    // another CPU posts it during the fault.
    if route.returns() && frame.psr & 0xF == 0 {
        deliver_notes(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::{SyncRoute, TrapFrame, UREG_SIZE, user_return_psr};

    const EL0T: u64 = 0x8000_0000; // N set, M[3:0] = 0
    const EL1H: u64 = 0x3c5;

    #[test]
    fn every_returning_el0_trap_is_a_note_boundary() {
        // SVC and both lower-EL aborts come back to trap() and return to
        // EL0: each must run the note boundary.
        for (ec, want) in [
            (0x15, SyncRoute::Syscall),
            (0x20, SyncRoute::DemandFault),
            (0x24, SyncRoute::DemandFault),
        ] {
            let r = SyncRoute::of(ec, EL0T);
            assert_eq!(r, want, "ec {ec:#x}");
            assert!(r.returns(), "ec {ec:#x} ({r:?}) must be a note boundary");
        }
    }

    #[test]
    fn diverging_routes_are_not_boundaries() {
        // Other EL0-origin classes kill the process; EL1-origin ones halt.
        for ec in [0x00, 0x07, 0x0e, 0x22, 0x26, 0x2f, 0x3c] {
            assert_eq!(SyncRoute::of(ec, EL0T), SyncRoute::UserKill, "ec {ec:#x}");
            assert_eq!(SyncRoute::of(ec, EL1H), SyncRoute::KernelHalt, "ec {ec:#x}");
        }
        assert!(!SyncRoute::UserKill.returns());
        assert!(!SyncRoute::KernelHalt.returns());
    }

    const DAIF: u64 = 0xF << 6;
    const IL: u64 = 1 << 20;
    const SS: u64 = 1 << 21;
    const SSBS: u64 = 1 << 12;
    const DIT: u64 = 1 << 24;

    #[test]
    fn noted_psr_takes_only_the_condition_flags_from_the_saved_ureg() {
        // Everything privileged a handler could write: DAIF, AArch32,
        // EL1h, IL, SS — none of it may reach SPSR_EL1.
        let hostile = 0xF000_0000 | DAIF | 0x10 | 0x5 | IL | SS;
        assert_eq!(user_return_psr(hostile, 0), 0xF000_0000);
        // The condition flags are the user's: cleared ones stay cleared.
        assert_eq!(user_return_psr(DAIF, 0xF000_0000), 0);
        assert_eq!(user_return_psr(0x6000_0000, 0x9000_0000), 0x6000_0000);
    }

    #[test]
    fn noted_psr_keeps_the_live_frames_other_bits_but_never_a_mask() {
        // Unprivileged PSTATE the process set itself (SSBS, DIT) rides
        // the live frame; even a live frame's mask/mode bits are dropped.
        assert_eq!(user_return_psr(0, SSBS | DIT), SSBS | DIT);
        assert_eq!(user_return_psr(DAIF, SSBS | DAIF | 0x5 | IL | SS), SSBS);
    }

    #[test]
    fn restore_from_saved_rebuilds_the_psr() {
        let mut saved = TrapFrame::zeroed();
        saved.x0 = 0x1234;
        saved.pc = 0x20_0000;
        saved.sp = 0x7fff_f000;
        saved.psr = 0x8000_0000 | DAIF | 0x5 | IL;
        let mut bytes = [0u8; UREG_SIZE];
        saved.copy_to_saved(&mut bytes);

        let mut live = TrapFrame::zeroed();
        live.psr = 0x2000_0000 | SSBS;
        live.restore_from_saved(&bytes);
        assert_eq!(live.x0, 0x1234);
        assert_eq!(live.pc, 0x20_0000);
        assert_eq!(live.sp, 0x7fff_f000);
        assert_eq!(live.psr, 0x8000_0000 | SSBS);
    }
}
