use core::cell::SyncUnsafeCell;

use crate::deviceutil::map_device_register;
use crate::io::{read_reg, write_reg};
use crate::registers::{UART0_CR, UART0_DR, UART0_FR, UART0_ICR, UART0_IMSC, UART0_LCRH};
use crate::vm;
use port::Result;
use port::devcons::Uart;
use port::fdt::DeviceTree;
use port::mem::{PhysRange, VirtRange};

#[cfg(not(test))]
use port::println;

/// PL011's INTID on virt: the DTB carries interrupts = <0x00 0x01 0x04>
/// (type SPI, num 1, level-high) -> INTID 32+1.  Being an SPI it needs the
/// ITARGETSR cpu0 write, which gic::enable applies (research P6).
pub const UART_INTID: u32 = 33;

/// Handler-side access to the mapped PL011 registers.  Published once when
/// the console uart is created (before trap::unmask_irq), read lock-free in
/// IRQ context -- the echo path must never take the Console mcslock
/// (research P8).
static PL011_RANGE: SyncUnsafeCell<Option<VirtRange>> = SyncUnsafeCell::new(None);

/// PL011 on qemu's virt machine.  Unlike the raspi variant
/// (platform/raspi3b/uartpl011.rs) there is no GPIO/mailbox init -- qemu
/// pre-configures the UART (phase-03 research Pattern 3), so this driver is
/// DTB-discovered, map_device_register-mapped, polled-TX plus an IRQ-driven
/// RX echo path (SC#4).
pub struct Pl011Uart {
    pl011_virtrange: VirtRange,
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

        let uart = Pl011Uart { pl011_virtrange };
        // Publish the mapped range for the IRQ echo path: single writer,
        // pre-unmask; read lock-free thereafter.
        unsafe { *PL011_RANGE.get() = Some(uart.pl011_virtrange.clone()) };
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
        while read_reg(&self.pl011_virtrange, UART0_FR) & (1 << 5) != 0 {}
        write_reg(&self.pl011_virtrange, UART0_DR, b as u32);
    }
}

/// Arm the RX interrupt path (main9 calls this before trap::unmask_irq, after
/// the GIC is up).  The register sequence is the research Pattern 3 probe,
/// verified live on qemu 11.1.2.
pub fn init_rx() {
    let range = unsafe { &*PL011_RANGE.get() };
    let Some(range) = range.as_ref() else {
        println!("pl011 rx init: no mapped uart");
        return;
    };
    write_reg(range, UART0_ICR, 0x7ff); // clear all interrupt status
    write_reg(range, UART0_LCRH, 0x70); // 8-bit words + FIFO enable (16-deep RX FIFO)
    write_reg(range, UART0_IMSC, 0x50); // RXIM | RTIM: RX and RX-timeout interrupts
    write_reg(range, UART0_CR, 0x301); // UARTEN | TXE | RXE
}

/// INTID 33 handler: drain the RX FIFO and echo every byte verbatim via raw
/// DR writes.  LOCK-FREE by construction (research P8): no println!, no
/// Console access -- the Console mcslock may be held by the interrupted
/// context, and taking it here would self-deadlock.  Bytes are echoed, never
/// interpreted: no control-byte actions (Ctrl-P and friends stay deferred per
/// CONTEXT.md).
pub fn rx_irq_echo() {
    let range = unsafe { &*PL011_RANGE.get() };
    let Some(range) = range.as_ref() else { return };

    // FR bit4 (RXFE) clear => data available.  qemu raises INT_RX on every
    // received char (read_trigger=1), so this drains one byte per IRQ in
    // practice but loops for safety.
    while read_reg(range, UART0_FR) & 0x10 == 0 {
        let b = read_reg(range, UART0_DR) as u8;
        // Echo verbatim, polling FR bit5 (TXFF) exactly like putb.
        while read_reg(range, UART0_FR) & (1 << 5) != 0 {}
        write_reg(range, UART0_DR, b as u32);
    }
    write_reg(range, UART0_ICR, 0x50); // clear RX/RT
}
