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
fn oom(layout: Layout) -> ! {
    // The layout + arena high-water on the wire: an oom with the arena
    // full names a leak or an era-capacity gap; an oom with arena LEFT
    // names a fragmentation/size-class failure (the quick/misc lists
    // could not serve a fit the tail still had room for).
    let (used, total) = crate::allocator::heap_stats();
    panic!("oom: {layout:?} arena used {used:#x} of {total:#x}");
}
