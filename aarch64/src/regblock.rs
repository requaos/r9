//! Typed MMIO register blocks.
//!
//! A `RegBlock` is a mapped device register block: a VirtRange naming MMIO
//! pages mapped r/w in the kernel table by `deviceutil::map_device_register`.
//! Drivers access registers BY OFFSET through these typed methods — the
//! volatile reads/writes themselves live only in `io.rs`'s primitives (the
//! named MMIO-volatility floor), and no driver does ad-hoc pointer
//! arithmetic.  The block is plain data (a range), so it rides the
//! publish-once SyncUnsafeCell cells (pl011's PL011_RANGE, gic's GIC) into
//! IRQ context unchanged — the lock-free echo path takes no lock.

use crate::io;
use port::mem::VirtRange;

/// A mapped MMIO register block — see the module header.
#[derive(Clone)]
pub struct RegBlock {
    range: VirtRange,
}

impl RegBlock {
    /// Wrap a mapped register range (deviceutil's product).
    pub fn new(range: VirtRange) -> RegBlock {
        RegBlock { range }
    }

    /// The underlying mapped range (the publish-once cells store blocks;
    /// some init paths still report the range for diagnostics).
    #[allow(dead_code)]
    pub fn range(&self) -> &VirtRange {
        &self.range
    }

    /// Read the 32-bit register at `offset` (volatile — the hardware owns
    /// the value; the read may have side effects, e.g. PL011 DR pops the
    /// RX FIFO).  Panics if the offset is outside the mapped block.
    pub fn read(&self, offset: usize) -> u32 {
        io::read_reg(&self.range, offset)
    }

    /// Write `val` to the 32-bit register at `offset` (volatile).  Panics
    /// if the offset is outside the mapped block.
    pub fn write(&self, offset: usize, val: u32) {
        io::write_reg(&self.range, offset, val)
    }

    /// Read-modify-write: set the bits in `val` at `offset` (volatile).
    /// Panics if the offset is outside the mapped block.
    #[allow(dead_code)]
    pub fn write_or(&self, offset: usize, val: u32) {
        io::write_or_reg(&self.range, offset, val)
    }
}
