#![cfg(not(test))]

extern crate alloc;

use alloc::alloc::Layout;
use core::panic::PanicInfo;
use port::println;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    // LOUD (the aarch64 runtime shape): a silent panic loop once hid a
    // failed bootfs mount behind a motionless serial line (14-05
    // bring-up) — the panic message IS the boot's diagnosis, on the
    // same wire every gate asserts.
    println!("{info}\n");
    #[allow(clippy::empty_loop)]
    loop {}
}

#[alloc_error_handler]
fn oom(_layout: Layout) -> ! {
    panic!("oom");
}
