use core::fmt;

#[cfg(platform = "virt")]
use crate::platform;
use crate::registers::EsrEl1;
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

/// Register frame at time interrupt was taken
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
    esr_el1: EsrEl1,
    elr_el1: u64,
    far_el1: u64,
    interrupt_type: u64,
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
            .field("x29 (frame_pointer)", &format_args!("{:#018x}", self.frame_pointer))
            .field("x30 (link_register)", &format_args!("{:#018x}", self.link_register))
            .field("esr_el1", &format_args!("{:#?}", self.esr_el1))
            .field("elr_el1", &format_args!("{:#018?}", self.elr_el1))
            .field("far_el1", &format_args!("{:#018?}", self.far_el1))
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
    if dispatch_irq(frame) {
        return;
    }

    if frame.esr_el1.ec() == 0x15 {
        // Syscall
        let syscallid = frame.esr_el1.iss();
        println!("Syscall {syscallid}");
    } else {
        println!("{:#?}", frame);
        println!("Unhandled interrupt");
    }

    loop {
        core::hint::spin_loop();
    }
}
