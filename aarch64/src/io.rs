use core::ptr::{read_volatile, write_volatile};
use port::mem::VirtRange;

#[allow(dead_code)]
pub enum GpioPull {
    Off = 0,
    Down,
    Up,
}

/// Delay for count cycles
#[allow(dead_code)]
pub fn delay(count: u32) {
    for _ in 0..count {
        core::hint::spin_loop();
    }
}

/// Write val into the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
pub fn write_reg(range: &VirtRange, offset: usize, val: u32) {
    let dst = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: dst lies inside the driver's mapped MMIO VirtRange (bounds-
    // checked by offset_addr); the volatile write is the MMIO-volatility
    // floor — hardware semantics, never elided.
    unsafe { write_volatile(dst as *mut u32, val) }
}

/// Write val|old into the reg RegBlock at offset from reg.addr,
/// where `old` is the existing value.
/// Panics if offset is outside any range specified by reg.len.
#[allow(dead_code)]
pub fn write_or_reg(range: &VirtRange, offset: usize, val: u32) {
    let dst = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile read-modify-write of the
    // mapped register.
    unsafe {
        let old = read_volatile(dst as *const u32);
        write_volatile(dst as *mut u32, val | old)
    }
}

/// Read from the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
pub fn read_reg(range: &VirtRange, offset: usize) -> u32 {
    let src = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile read of the mapped
    // register (the read may have hardware side effects, e.g. PL011 DR).
    unsafe { read_volatile(src as *const u32) }
}

/// Write a u8 into the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
#[allow(dead_code)]
pub fn write_reg8(range: &VirtRange, offset: usize, val: u8) {
    let dst = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile 8-bit write of the mapped
    // register; width-matched per the virtio common-cfg access discipline
    // (8-bit accesses for 8-bit fields, virtio-v1.3 §4.1.3.1).
    unsafe { write_volatile(dst as *mut u8, val) }
}

/// Read a u8 from the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
#[allow(dead_code)]
pub fn read_reg8(range: &VirtRange, offset: usize) -> u8 {
    let src = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile 8-bit read of the mapped
    // register.  The read may have hardware side effects: for a virtio
    // ISR-status byte the read IS the acknowledge (virtio-v1.3 §4.1.4.5 —
    // the read resets the ISR to 0 and deasserts the interrupt line; the
    // PL011 DR comment above is the same side-effect class).
    unsafe { read_volatile(src as *const u8) }
}

/// Write a u16 into the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
#[allow(dead_code)]
pub fn write_reg16(range: &VirtRange, offset: usize, val: u16) {
    let dst = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile 16-bit write of the mapped
    // register (queue_select/queue_enable, PCI_COMMAND).
    unsafe { write_volatile(dst as *mut u16, val) }
}

/// Read a u16 from the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
#[allow(dead_code)]
pub fn read_reg16(range: &VirtRange, offset: usize) -> u16 {
    let src = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile 16-bit read of the mapped
    // register.
    unsafe { read_volatile(src as *const u16) }
}

/// Write a u64 into the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
#[allow(dead_code)]
pub fn write_reg64(range: &VirtRange, offset: usize, val: u64) {
    let dst = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile 64-bit write of the mapped
    // register (virtqueue descriptor addresses).
    unsafe { write_volatile(dst as *mut u64, val) }
}

/// Read a u64 from the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
#[allow(dead_code)]
pub fn read_reg64(range: &VirtRange, offset: usize) -> u64 {
    let src = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile 64-bit read of the mapped
    // register.
    unsafe { read_volatile(src as *const u64) }
}
