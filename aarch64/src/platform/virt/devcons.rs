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

/// devcons device write (04-06, D-15): the syscall PWRITE path for devcons
/// chans (fd 1/2, opened "cons").  Byte-verbatim through the Console MCS
/// lock + PL011 polled TX — shares the kernel print path's lock safely
/// because the timer tick never prints under dev_exec (Pitfall 3; timer.rs
/// gates it), so no IRQ-context code contends while this runs.
pub fn cons_write(buf: &[u8]) -> u64 {
    port::devcons::write_bytes(buf) as u64
}

/// devcons device read (04-06): drain the PL011 RX ring into `buf`,
/// returning the byte count — 0 on empty (no blocking console-read this
/// phase).  Under cfg(not(dev_exec)) no RX ring exists (bytes echo
/// instead), and no EL0 reader can exist either — return 0.
#[cfg(dev_exec)]
pub fn cons_read(buf: &mut [u8]) -> u64 {
    super::pl011::cons_rx_drain(buf) as u64
}

#[cfg(not(dev_exec))]
pub fn cons_read(_buf: &mut [u8]) -> u64 {
    0
}
