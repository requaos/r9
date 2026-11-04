use core::cell::SyncUnsafeCell;

use crate::deviceutil::map_device_register;
use crate::regblock::RegBlock;
use crate::registers::{UART0_CR, UART0_DR, UART0_FR, UART0_ICR, UART0_IMSC, UART0_LCRH};
use crate::vm;
use port::Result;
use port::devcons::Uart;
use port::fdt::DeviceTree;
use port::mem::PhysRange;

#[cfg(not(test))]
use port::println;

/// PL011's INTID on virt: the DTB carries interrupts = <0x00 0x01 0x04>
/// (type SPI, num 1, level-high) -> INTID 32+1.  Being an SPI it needs the
/// ITARGETSR cpu0 write, which gic::enable applies (research P6).
pub const UART_INTID: u32 = 33;

/// Handler-side access to the mapped PL011 registers (the
/// typed RegBlock — register access by offset, never ad-hoc pointers).
/// Published once when the console uart is created (before
/// trap::unmask_irq), read lock-free in IRQ context -- the echo path must
/// never take the Console mcslock (research P8).  RegBlock is plain data,
/// so the publish-once discipline is unchanged.
static PL011_RANGE: SyncUnsafeCell<Option<RegBlock>> = SyncUnsafeCell::new(None);

/// PL011 on qemu's virt machine.  Unlike the raspi variant
/// (platform/raspi3b/uartpl011.rs) there is no GPIO/mailbox init -- qemu
/// pre-configures the UART, so this driver is
/// DTB-discovered, map_device_register-mapped, polled-TX plus an IRQ-driven
/// RX echo path.
pub struct Pl011Uart {
    regs: RegBlock,
}

impl Pl011Uart {
    pub fn new(dt: &DeviceTree) -> Result<Pl011Uart> {
        let pl011_physrange = Self::find_pl011_physrange(dt)?;
        let pl011_virtrange =
            match map_device_register("pl011", pl011_physrange, vm::PageSize::Page4K) {
                Ok(pl011_virtrange) => pl011_virtrange,
                Err(msg) => {
                    println!("can't map pl011 {:?}", msg);
                    return Err("can't create pl011");
                }
            };

        let uart = Pl011Uart { regs: RegBlock::new(pl011_virtrange) };
        // Publish the mapped range for the IRQ echo path: single writer,
        // pre-unmask; read lock-free thereafter.
        // SAFETY: the publish-once discipline — this write runs at driver
        // construction (boot context, before trap::unmask_irq), so no IRQ
        // reader can observe a torn write; the cell is never written again.
        unsafe { *PL011_RANGE.get() = Some(uart.regs.clone()) };
        Ok(uart)
    }

    fn find_pl011_physrange(dt: &DeviceTree) -> Result<PhysRange> {
        dt.find_compatible("arm,pl011")
            .next()
            .and_then(|uart| dt.property_translated_reg_iter(uart).next())
            .and_then(|reg| reg.regblock())
            .map(|reg| PhysRange::from(&reg))
            .ok_or("can't find pl011")
    }
}

impl Uart for Pl011Uart {
    fn putb(&self, b: u8) {
        // Wait for UART to become ready to transmit.
        while self.regs.read(UART0_FR) & (1 << 5) != 0 {}
        self.regs.write(UART0_DR, b as u32);
    }
}

/// Arm the RX interrupt path (main9 calls this before trap::unmask_irq, after
/// the GIC is up).  The register sequence is the research Pattern 3 probe,
/// verified live on qemu 11.1.2.
pub fn init_rx() {
    // SAFETY: PL011_RANGE was published at construction (before unmask);
    // this read runs in boot context — no writer is active.
    let range = unsafe { &*PL011_RANGE.get() };
    let Some(range) = range.as_ref() else {
        println!("pl011 rx init: no mapped uart");
        return;
    };
    range.write(UART0_ICR, 0x7ff); // clear all interrupt status
    range.write(UART0_LCRH, 0x70); // 8-bit words + FIFO enable (16-deep RX FIFO)
    range.write(UART0_IMSC, 0x50); // RXIM | RTIM: RX and RX-timeout interrupts
    range.write(UART0_CR, 0x301); // UARTEN | TXE | RXE
}

/// INTID 33 handler entry (trap.rs's dispatch): the RX byte path differs by
/// profile.  cfg(not(dev_exec)): the echo profile — bytes are echoed
/// verbatim over raw DR writes (the r9echo7 boot-test assertion).  Under
/// cfg(dev_exec) bytes instead feed the per-console read buffer that the
/// devcons device's read path drains — echoing userland's
/// console input back would corrupt the exec profile's serial evidence.
pub fn rx_irq() {
    #[cfg(not(dev_exec))]
    rx_irq_echo();
    #[cfg(dev_exec)]
    rx_irq_cons_buffer();
}

/// The dev_exec INTID 33 handler: drain the RX FIFO into the per-console
/// read buffer.  LOCK-FREE by construction (research P8): no println!, no
/// Console access — the Console mcslock may be held by the interrupted
/// context, and taking it here would self-deadlock.
///
/// Buffer discipline: single writer (this IRQ handler) / single reader (the
/// devcons read path, which runs in IRQ-masked syscall/trap context — the
/// writer can never preempt the reader this era).  Overflow drops bytes
/// (counted in OVERRUNS for diagnostics); nothing types fast enough to fill
/// 256 bytes during the exec gate, and a dropped byte is loud-in-effect
/// (missing input), never memory corruption.
#[cfg(dev_exec)]
pub fn rx_irq_cons_buffer() {
    // SAFETY: PL011_RANGE was published before IRQ unmask (the cell is
    // write-once); IRQ context reads it lock-free by construction.
    let range = unsafe { &*PL011_RANGE.get() };
    let Some(range) = range.as_ref() else { return };

    while range.read(UART0_FR) & 0x10 == 0 {
        let b = range.read(UART0_DR) as u8;
        // SAFETY: the ring's single-writer discipline — this IRQ handler
        // is the only writer, and the syscall-context reader runs
        // IRQ-masked (it can never interleave mid-drain).
        let buf = unsafe { &mut *CONS_RX.get() };
        let next = (buf.head + 1) % CONS_RX_CAP;
        if next != buf.tail {
            buf.bytes[buf.head] = b;
            buf.head = next;
        } else {
            buf.overruns += 1;
        }
    }
    range.write(UART0_ICR, 0x50); // clear RX/RT
}

/// The console RX ring (dev_exec only).  Head = IRQ writer, tail = syscall
/// reader; empty when head == tail.
#[cfg(dev_exec)]
struct ConsRx {
    bytes: [u8; CONS_RX_CAP],
    head: usize,
    tail: usize,
    overruns: u32,
}

#[cfg(dev_exec)]
const CONS_RX_CAP: usize = 256;

#[cfg(dev_exec)]
static CONS_RX: SyncUnsafeCell<ConsRx> =
    SyncUnsafeCell::new(ConsRx { bytes: [0; CONS_RX_CAP], head: 0, tail: 0, overruns: 0 });

/// devcons read side (dev_exec): drain up to `buf.len()` bytes from the RX
/// ring; return the count — 0 when empty.  Called only from IRQ-masked
/// syscall context (the devcons input engine's drain hook), so the
/// IRQ writer cannot interleave.
#[cfg(dev_exec)]
pub fn cons_rx_drain(buf: &mut [u8]) -> usize {
    // SAFETY: the reader side of the ring discipline — called only from
    // IRQ-masked syscall context (the devcons drain hook), so the IRQ
        // writer cannot interleave.
    let rx = unsafe { &mut *CONS_RX.get() };
    let mut n = 0;
    while n < buf.len() && rx.tail != rx.head {
        buf[n] = rx.bytes[rx.tail];
        rx.tail = (rx.tail + 1) % CONS_RX_CAP;
        n += 1;
    }
    n
}

/// Lock-free "RX ring non-empty" peek for the tick's
/// cons-wake hook (timer.rs → proc::cons_input_wake).  A torn read of
/// head/tail is benign: a missed wake retries at the next tick (1 ms),
/// and a spurious wake just re-blocks after the reader's drain finds
/// nothing.
#[cfg(dev_exec)]
pub fn cons_rx_pending() -> bool {
    // SAFETY: the lock-free peek (a torn head/tail read is benign — a
    // missed wake retries at the next tick; a spurious wake re-blocks).
    let rx = unsafe { &*CONS_RX.get() };
    rx.head != rx.tail
}

/// gatedbg: the UART FIFO's non-empty flag
/// (FR.RXFE bit clear) — the autopsy discriminator for a lost RX/RT edge:
/// bytes in the FIFO but never IRQ'd into the ring while a cons reader
/// stays blocked.
#[cfg(dev_exec)]
pub fn cons_rx_fifo_pending() -> Option<bool> {
    // SAFETY: PL011_RANGE's write-once discipline (IRQ-masked context).
    let range = unsafe { &*PL011_RANGE.get() };
    let range = range.as_ref()?;
    Some(range.read(UART0_FR) & 0x10 == 0)
}

/// gatedbg: the RX ring's head/tail (for the wedge dump).
#[cfg(dev_exec)]
pub fn cons_rx_ring_pos() -> (usize, usize) {
    // SAFETY: cons_rx_pending's lock-free-peek discipline (the wedgedbg
    // dump's diagnostic read).
    let rx = unsafe { &*CONS_RX.get() };
    (rx.head, rx.tail)
}

/// The tick's direct FIFO
/// poll — drain any FIFO bytes the RX/RT interrupt path missed into the
/// ring (the IRQ handler's own discipline, reused).  The observed wedge
/// signature: rc's cons read woke on a typed
/// command's byte PREFIX, the final byte never interrupted, and the
/// reader re-blocked forever.  qemu's PL011 model raises INT_RX per
/// received char, but a byte landing in the drain→clear window can miss
/// its interrupt; polling here bounds the delivery delay at one tick
/// (1 ms) instead of losing the byte until the next one arrives.
/// Tick context is IRQ-masked and same-priority IRQs never nest, so the
/// ring's single-writer discipline is preserved.
#[cfg(dev_exec)]
pub fn cons_rx_poll() {
    if cons_rx_fifo_pending() == Some(true) {
        rx_irq_cons_buffer();
    }
}

/// INTID 33 handler: drain the RX FIFO and echo every byte verbatim via raw
/// DR writes.  LOCK-FREE by construction (research P8): no println!, no
/// Console access -- the Console mcslock may be held by the interrupted
/// context, and taking it here would self-deadlock.  Bytes are echoed, never
/// interpreted: no control-byte actions (Ctrl-P and friends stay
/// deferred).
#[cfg(not(dev_exec))]
pub fn rx_irq_echo() {
    // SAFETY: PL011_RANGE was published before IRQ unmask (write-once);
    // IRQ context reads it lock-free by construction.
    let range = unsafe { &*PL011_RANGE.get() };
    let Some(range) = range.as_ref() else { return };

    // FR bit4 (RXFE) clear => data available.  qemu raises INT_RX on every
    // received char (read_trigger=1), so this drains one byte per IRQ in
    // practice but loops for safety.
    while range.read(UART0_FR) & 0x10 == 0 {
        let b = range.read(UART0_DR) as u8;
        // Echo verbatim, polling FR bit5 (TXFF) exactly like putb.
        while range.read(UART0_FR) & (1 << 5) != 0 {}
        range.write(UART0_DR, b as u32);
    }
    range.write(UART0_ICR, 0x50); // clear RX/RT
}
