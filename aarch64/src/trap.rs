use core::fmt;

#[cfg(platform = "virt")]
use crate::platform;
use crate::fault;
use crate::registers::{EsrEl1, ExceptionClass};
use aarch64_cpu::registers::FAR_EL1;
use aarch64_cpu::registers::Readable;
use port::println;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("trap.S"));

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
    x0: u64,
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
    sp: u64,  // interrupted context's stack pointer (SP_EL0 on EL0 traps, entry-sp on EL1 traps)
    pc: u64,  // ELR_EL1 — interrupted address
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

// Interrupt-type constants, matching the .equ block in trap.S (lines 17-25).
#[cfg(platform = "virt")]
const IRQ_INVALID_EL1T: u64 = 1;
#[cfg(platform = "virt")]
const IRQ_INVALID_EL1H: u64 = 5;

/// IRQ dispatch (virt): ack via the GIC, dispatch, EOI, and RETURN -- trap.S's
/// existing restore + eret path resumes the interrupted context (the wfi
/// loop).  The spurious INTID 1023 is acked but never EOI'd; any other
/// unknown INTID is logged once and still EOI'd so a stray interrupt can't
/// wedge the kernel.  The INTID 33 (PL011 RX) arm lands with its handler.
#[cfg(platform = "virt")]
fn dispatch_irq(frame: &TrapFrame) -> bool {
    if frame.interrupt_type != IRQ_INVALID_EL1T && frame.interrupt_type != IRQ_INVALID_EL1H {
        return false;
    }

    let intid = platform::gic::ack();
    match intid {
        // ARM generic timer CNTP (PPI 14 -> INTID 16+14)
        30 => platform::timer::tick(),
        // PL011 UART RX (SPI 1 -> INTID 32+1): drain + lock-free echo
        33 => platform::pl011::rx_irq_echo(),
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

fn trap(frame: &mut TrapFrame) {
    // D-05: ESR_EL1/FAR_EL1 are read LIVE at trap entry and flow as
    // arguments to the dispatchers — never frame fields.  04-04's fault
    // handler receives them as fault::user_fault(frame, esr, far); the IRQ
    // path never needs them.
    let esr = EsrEl1::read();
    let far = FAR_EL1.get();

    if dispatch_irq(frame) {
        return;
    }

    if esr.ec() == 0x15 {
        // Syscall — the WIRE syscall number is R0 (frame.x0), NOT the ISS
        // (aarch64/ABI.md: number in R0, args at SP+8, result in R0).
        // Real dispatch lands in 04-06; stay loud until then (D-14).
        println!("syscall {} unimplemented", frame.x0);
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
