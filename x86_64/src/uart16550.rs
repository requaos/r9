//! Simple UART driver to get started.

/// qemu q35's -serial rides the architectural COM1 16550 — a fixed
/// platform fact of the machine model (devcons registers this port).
pub const COM1: u16 = 0x3f8;

// 16550 register offsets from the base port.
const RBR: u16 = 0; // receive buffer (read; the read pops the byte)
const LSR: u16 = 5; // line status
/// LSR bit 0: receiver data ready.
const LSR_DATA_READY: u8 = 0x01;

pub fn putb(port: u16, b: u8) {
    unsafe {
        crate::pio::outb(port, b);
    }
}

/// Read one received byte if the 16550 holds one: LSR data-ready poll,
/// then the RBR read (the read has the pop side effect, so it runs only
/// behind the ready bit).
pub fn getb(port: u16) -> Option<u8> {
    // SAFETY: port I/O to the fixed COM1 offsets — the LSR read is
    // side-effect-free and the RBR read pops exactly the ready byte.
    unsafe {
        if crate::pio::inb(port + LSR) & LSR_DATA_READY != 0 {
            Some(crate::pio::inb(port + RBR))
        } else {
            None
        }
    }
}

/// The polled console-RX sweep: drain every LSR-ready byte and echo it
/// back.  This is the bring-up console's RX path — the timer's tick
/// sweep calls it (IRQ-driven RX is NOT this phase), and qemu's socket
/// serial never self-echoes, so the gate's typed probe can only
/// reappear on the wire through this path: the echo proves the bytes
/// entered the kernel.  The drain is bounded (a flooding peer cannot
/// starve the tick).
pub fn rx_echo(port: u16) {
    for _ in 0..32 {
        match getb(port) {
            Some(b) => putb(port, b),
            None => break,
        }
    }
}
