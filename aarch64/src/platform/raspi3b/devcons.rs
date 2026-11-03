// Racy to start.

use super::uartmini::MiniUart;
use core::cell::SyncUnsafeCell;
use core::mem::MaybeUninit;
use port::devcons::Console;
use port::fdt::DeviceTree;
#[cfg(not(test))]
use port::println;
// The aarch64 devcons implementation is focussed on Raspberry Pi 3, 4 for now.

// Useful links
// - Raspberry Pi Processors
//     https://www.raspberrypi.com/documentation/computers/processors.html
// - Raspberry Pi Hardware
//     https://www.raspberrypi.com/documentation/computers/raspberry-pi.html
// - Raspi3 BCM2837
//     Datasheet (BCM2835) https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf
// - Raspi4 BCM2711
//     Datasheet https://datasheets.raspberrypi.com/bcm2711/bcm2711-peripherals.pdf
// - Mailbox
//     https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface

// Raspberry Pi 3 has 2 UARTs, Raspbery Pi 4 has 4:
// - UART0 PL011
// - UART1 miniUART
// - UART2 PL011 (rpi4)
// - UART3 PL011 (rpi4)

pub fn init(dt: &DeviceTree) {
    Console::set_uart(|| {
        let uart = MiniUart::new_with_map_ranges(dt);

        // Return a statically initialised MiniUart.  If that couldn't be done for some reason,
        // return None and hope that things work out regardless
        match uart {
            Ok(uart) => {
                uart.init();

                static UART: SyncUnsafeCell<MaybeUninit<MiniUart>> =
                    SyncUnsafeCell::new(MaybeUninit::uninit());
                // SAFETY: set_uart runs once at platform init (single CPU,
                // pre-IRQ-unmask); the cell is written before its address
                // escapes, and the published &'static dyn Uart only ever
                // reads it.
                unsafe {
                    let cons = &mut *UART.get();
                    cons.write(uart);
                    let cons: &'static MiniUart = cons.assume_init_mut();
                    Ok(cons)
                }
            }
            Err(msg) => {
                println!("can't initialise uart: {msg:?}");
                Err("can't initialise uart")
            }
        }
    });
}

/// devcons DEVICE write entry point (04-06 syscall-era shape).  The EL0
/// userspace path is virt-only this phase — no SVC can ever arrive on raspi
/// (no dev_exec configs exist for it), so these are unreachable stubs that
/// keep the platform-shared syscall.rs compiling on every platform.  The
/// write side still works if ever called: bytes go out through the Console
/// MCS lock like the virt variant.
pub fn cons_write(buf: &[u8]) -> u64 {
    port::devcons::write_bytes(buf) as u64
}

/// devcons device read — raspi has no console-RX ring this phase (the
/// virt dev_exec profile owns it).  Unreachable; returns 0 (empty).
pub fn cons_read(_buf: &mut [u8]) -> u64 {
    0
}
