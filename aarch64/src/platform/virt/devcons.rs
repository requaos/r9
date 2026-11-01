// Racy to start.

use core::cell::SyncUnsafeCell;
use core::mem::MaybeUninit;

use super::pl011::Pl011Uart;
use port::devcons::Console;
use port::fdt::DeviceTree;
#[cfg(not(test))]
use port::println;

pub fn init(dt: &DeviceTree) {
    Console::set_uart(|| {
        let uart = Pl011Uart::new(dt);

        // Return a statically initialised Pl011Uart.  If that couldn't be
        // done for some reason, return Err and hope that things work out
        // regardless.
        match uart {
            Ok(uart) => {
                static UART: SyncUnsafeCell<MaybeUninit<Pl011Uart>> =
                    SyncUnsafeCell::new(MaybeUninit::uninit());
                unsafe {
                    let cons = &mut *UART.get();
                    cons.write(uart);
                    Ok(cons.assume_init_mut())
                }
            }
            Err(msg) => {
                println!("can't initialise uart: {msg:?}");
                Err("can't initialise uart")
            }
        }
    });
}
