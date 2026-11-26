#![cfg(not(test))]

extern crate alloc;

use alloc::alloc::Layout;
use core::panic::PanicInfo;

#[cfg(not(test))]
use port::println;

// TODO
//  - Add qemu integration test
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    // A panic normally runs with IRQs masked, and the console's TX FIFO is
    // drained from the TX interrupt. Masked, the message dies after the
    // ~16-byte FIFO — every capture read `panicked at po` and nothing more.
    // Re-enable IRQs so the console can flush the whole message. A panic is
    // already terminal, so an early unmask can only turn a silent hang into
    // a narrated one.
    crate::trap::unmask_irq();
    println!("{}\n", info);

    #[allow(clippy::empty_loop)]
    loop {}
}

#[alloc_error_handler]
fn oom(layout: Layout) -> ! {
    panic!("oom: size={} align={} (arena exhausted)", layout.size(), layout.align());
}
