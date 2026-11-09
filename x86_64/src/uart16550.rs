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
#[cfg(not(dev_exec))]
pub fn rx_echo(port: u16) {
    for _ in 0..32 {
        match getb(port) {
            Some(b) => putb(port, b),
            None => break,
        }
    }
}

// The console RX ring (dev_exec) — the x64 re-key of aarch64's PL011
// RX ring (pl011.rs): the TICK's polled sweep is the single producer
// (the 16550 LSR/RBR reads below, IRQ-masked tick context — x64 has no
// UART IRQ this era, the tick's poll IS the producer where aarch64
// splits IRQ-handler + tick-poll) and the devcons input engine's drain
// hook is the single reader (syscall context, also IF=0).  The two
// contexts can never interleave, so the ring needs no lock — the
// aarch64 single-writer/single-reader discipline verbatim.
//
// The ring exists so typed bytes survive between the tick that
// collected them and the reader that drains them: rc's blocked #c/cons
// read is woken by the tick's cons_input_wake (proc.rs) and drains in
// its own syscall context when it re-enters the input engine.  NO echo
// ever happens from tick context (the Console MCS lock may be held by
// the interrupted syscall context — the aarch64 P8 no-Console-in-IRQ
// rule): the cooked echo moves to devcons's input engine, which emits
// through the echo hook (the device-write path) from READER context,
// after the INPUT lock drops.
//
// 14-07: without this the typed bytes were echoed to the wire by the
// bring-up sweep and DISCARDED — the chain's interactive gate (the
// composed smoke) is the first consumer of console RX on x86_64, and
// rc's read errored on the missing hooks instead ("cons read path not
// initialized", main.rs's registration note).
#[cfg(dev_exec)]
mod cons_rx {
    use core::cell::SyncUnsafeCell;

    const CAP: usize = 256;

    struct ConsRx {
        bytes: [u8; CAP],
        head: usize,
        tail: usize,
    }

    static RING: SyncUnsafeCell<ConsRx> =
        SyncUnsafeCell::new(ConsRx { bytes: [0; CAP], head: 0, tail: 0 });

    /// The tick's producer sweep: drain every LSR-ready byte into the
    /// ring.  Bounded (a flooding peer cannot starve the tick).  On a
    /// full ring the byte is DROPPED, never grown unbounded — nothing
    /// types fast enough to fill 256 bytes between a reader's drains,
    /// and a dropped byte is loud-in-effect (missing input), never
    /// corruption (the aarch64 overrun rule).
    pub fn rx_ring_buffer(port: u16) {
        for _ in 0..32 {
            let Some(b) = super::getb(port) else { break };
            // SAFETY: the ring's single-writer discipline — the tick is
            // the only writer, and the reader runs IF=0 in syscall
            // context (it can never interleave mid-sweep).
            let rx = unsafe { &mut *RING.get() };
            let next = (rx.head + 1) % CAP;
            if next != rx.tail {
                rx.bytes[rx.head] = b;
                rx.head = next;
            }
        }
    }

    /// devcons's drain hook: drain up to `buf.len()` bytes from the
    /// ring; 0 when empty.  Called only from IF=0 syscall context (the
    /// input engine), so the tick's write can never interleave.
    pub fn cons_rx_drain(buf: &mut [u8]) -> usize {
        // SAFETY: the reader side of the ring discipline — called only
        // from IF=0 syscall context, so the tick's write can never
        // interleave.
        let rx = unsafe { &mut *RING.get() };
        let mut n = 0;
        while n < buf.len() && rx.tail != rx.head {
            buf[n] = rx.bytes[rx.tail];
            rx.tail = (rx.tail + 1) % CAP;
            n += 1;
        }
        n
    }

    /// The tick's non-empty peek for the cons-wake hook (timer.rs →
    /// proc::cons_input_wake) — aarch64's lock-free peek: a torn
    /// head/tail read is benign (a missed wake retries at the next
    /// tick; a spurious wake re-blocks after the drain finds nothing).
    pub fn cons_rx_pending() -> bool {
        // SAFETY: the lock-free peek (a torn read is benign — see
        // cons_rx_drain's discipline).
        let rx = unsafe { &*RING.get() };
        rx.head != rx.tail
    }
}

#[cfg(dev_exec)]
pub use cons_rx::{cons_rx_drain, cons_rx_pending, rx_ring_buffer};
