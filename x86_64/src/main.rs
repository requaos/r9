#![feature(alloc_error_handler)]
#![feature(fn_align)]
#![feature(sync_unsafe_cell)]
#![cfg_attr(not(any(test)), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(clippy::upper_case_acronyms)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod allocator;
mod cpu;
mod dat;
mod devcons;
mod node0;
mod pio;
mod proc;
mod syscall;
mod trap;
mod uart16550;
mod vsvm;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("l.S"), options(att_syntax));

use port::println;

#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn main(mach: &mut dat::Mach, _mbdata: u64) {
    unsafe {
        vsvm::init(mach);
    }
    syscall::init();
    devcons::init();
    println!();
    println!("r9 from the Internet");
    // The old-generation demonstration (swtch/jumpback/int3) is parked
    // (13-01): the mb2/GRUB2 channel boots this payload only to the
    // banner, so halt cleanly with IRQs masked — the boot-test gate's
    // negative panic scan stays green on the new channel.  The main9-x64
    // rewrite is 13-02.
    loop {
        unsafe { core::arch::asm!("cli; hlt") };
    }
}

mod runtime;
