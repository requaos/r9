// Racy to start.

use core::cell::SyncUnsafeCell;
use port::devcons::{Console, Uart};
#[cfg(not(test))]
use port::println;

use crate::pio;
use crate::uart16550;
use crate::uart16550::COM1;

const SCR: u16 = 7;

struct Uart16550 {
    port: u16,
}

impl Uart for Uart16550 {
    fn putb(&self, b: u8) {
        uart16550::putb(self.port, b);
    }
}

/// Register COM1 as the console uart, in the aarch64 tolerant shape: a
/// scratch-register probe before publishing — a readback mismatch means
/// no 16550 at COM1 (a machine-model surprise), printed loudly with the
/// console left unset rather than a silent garbage sink.
pub fn init() {
    Console::set_uart(|| {
        // SAFETY: port I/O to the fixed architectural COM1 offsets —
        // non-destructive scratch-register readback; the same fixed
        // platform fact every q35 invocation carries.
        unsafe {
            pio::outb(COM1 + SCR, 0x5a);
            let got = pio::inb(COM1 + SCR);
            pio::outb(COM1 + SCR, 0);
            if got != 0x5a {
                println!("can't initialise uart: no 16550 at COM1 (scratch readback)");
                return Err("no 16550 at COM1");
            }
        }
        static CONS: SyncUnsafeCell<Uart16550> = SyncUnsafeCell::new(Uart16550 { port: COM1 });
        // SAFETY: set_uart runs once at platform init (single CPU,
        // IRQs masked); the cell is written before its address escapes
        // and the published &'static dyn Uart only ever reads it.
        Ok(unsafe { &mut *CONS.get() })
    });
}
