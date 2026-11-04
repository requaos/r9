#![feature(alloc_error_handler)]
#![feature(sync_unsafe_cell)]
#![cfg_attr(not(any(test)), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(clippy::upper_case_acronyms)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod allocator;
mod platform;
mod runtime;
mod sbi;
mod uart16550;

use port::println;

use crate::platform::{devcons, platform_init};
use port::fdt::DeviceTree;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("l.S"));

#[unsafe(no_mangle)]
pub extern "C" fn main9(hartid: usize, dtb_ptr: usize) -> ! {
    // 261103-p64 (fdt internalization — fdt::from_usize is gone): the
    // two-pass construction inlines here (header prefix → totalsize →
    // full slice).
    // SAFETY: dtb_ptr names the live, mapped DTB (the boot contract).
    let dt = unsafe {
        let prefix = core::slice::from_raw_parts(dtb_ptr as *const u8, port::fdt::FDT_HEADER_SIZE);
        let total = port::fdt::DeviceTree::header_totalsize(prefix).unwrap();
        port::fdt::DeviceTree::new(core::slice::from_raw_parts(dtb_ptr as *const u8, total))
            .unwrap()
    };
    crate::devcons::init(&dt);
    platform_init();

    println!();
    println!("r9 from the Internet");
    println!("Domain0 Boot HART = {hartid}");
    println!("DTB found at: {dtb_ptr:#x}");

    #[cfg(not(test))]
    sbi::shutdown();
    #[cfg(test)]
    loop {}
}
