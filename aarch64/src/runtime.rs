#![cfg(not(test))]

extern crate alloc;

use alloc::alloc::Layout;
use core::panic::PanicInfo;

/// Panic output writer over the POLLING early UART.
///
/// The normal console path (`println!`) is interrupt-driven: it drains the
/// PL011 TX FIFO from the TX interrupt. A panic almost always runs with IRQs
/// masked, so that FIFO drains never run and the message dies after the
/// ~16-byte FIFO — which is exactly what every panic capture showed
/// (`panicked at po`). The early-UART putc polls TXFF and writes DR directly,
/// needs no lock and no interrupt, and the pre-MMU alias at PA 0x0900_0000 is
/// kept mapped by vminit. Panic output therefore survives a wedged console.
struct PanicUart;

impl core::fmt::Write for PanicUart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            #[cfg(platform = "virt")]
            crate::platform::virt::pl011::panic_putc(b);
            #[cfg(not(platform = "virt"))]
            crate::pre_mmu::util::init_early_uart_putc(b);
        }
        Ok(())
    }
}

// TODO
//  - Add qemu integration test
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;
    let mut uart = PanicUart;
    let _ = writeln!(uart, "{}", info);

    #[allow(clippy::empty_loop)]
    loop {}
}

#[alloc_error_handler]
fn oom(layout: Layout) -> ! {
    panic!("oom: size={} align={} (arena exhausted)", layout.size(), layout.align());
}
