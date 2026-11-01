#![allow(dead_code)]

// Functions to be called in early bootup phase, typically before the MMU has been enabled.
// This shouldn't normally be used for anything other than debugging at very early init.

use core::ops::{BitAndAssign, BitOrAssign};
use core::ptr::{read_volatile, write_volatile};

use crate::vm::Table;

const MMIO_BASE: u32 = 0xfe000000;
const AUX: u32 = MMIO_BASE + 0x00215000;
const AUX_ENABLES: u32 = AUX + 0x04;
const AUX_MU: u32 = AUX + 0x40;
const AUX_MU_IO: u32 = AUX_MU + 0x00; // AUX IO data register
const AUX_MU_IER: u32 = AUX_MU + 0x04;
const AUX_MU_IIR: u32 = AUX_MU + 0x08;
const AUX_MU_LCR: u32 = AUX_MU + 0x0c;
const AUX_MU_MCR: u32 = AUX_MU + 0x10;
const AUX_MU_LSR: u32 = AUX_MU + 0x14;
const AUX_MU_CNTL: u32 = AUX_MU + 0x20;
const AUX_MU_BAUD: u32 = AUX_MU + 0x28;

const GPIO: u32 = MMIO_BASE + 0x00200000;
const GPFSEL1: u32 = GPIO + 0x04;
const GPIO_PUP_PDN_CNTRL_REG0: u32 = GPIO + 0xe4;

// Set up a very early uart.  The full drivers live under platform/; this
// code is just enough to help debug the early stage.
//
// Raspi: the miniuart (see platform/raspi3b/uartmini.rs).
// Virt: nothing -- qemu pre-configures the PL011 (DR writes work even with
// UARTEN clear; phase-03 research Pattern 3).
#[cfg(not(platform = "virt"))]
#[unsafe(no_mangle)]
pub extern "C" fn init_early_uart() {
    // Calculate the baudrate to be inserted into AUX_MU_BAUD
    const UART_CLOCK: u32 = 500000000;
    const UART_BAUDRATE: u32 = 115200;
    const UART_BAUDRATE_REG: u32 = (UART_CLOCK / (UART_BAUDRATE * 8)) - 1;

    // Initialise the miniuart
    unsafe {
        write_or_volatile(AUX_ENABLES as *mut u32, 1);
        write_volatile(AUX_MU_CNTL as *mut u32, 0);
        write_volatile(AUX_MU_LCR as *mut u32, 3);
        write_volatile(AUX_MU_MCR as *mut u32, 0);
        write_volatile(AUX_MU_IER as *mut u32, 0);
        write_volatile(AUX_MU_IIR as *mut u32, 0xc6);
        write_volatile(AUX_MU_BAUD as *mut u32, UART_BAUDRATE_REG);
    }

    // Initialise GPIO pins for minuart
    unsafe {
        // Set up GPIO pin 14 pull up/down state
        // Mask all but bits 28:29
        write_and_volatile(GPIO_PUP_PDN_CNTRL_REG0 as *mut u32, 0xcfffffff);

        // Set GPIO pins 14 to be used for ALT5 - UART1 (miniuart)
        let mut gpfsel1_val = read_volatile(GPFSEL1 as *const u32);
        // Mask all but bits 12:14 (pin 14)
        gpfsel1_val &= 0xffff8fff;
        // Pin 14, ALT5
        gpfsel1_val |= 0x00002000;
        write_volatile(GPFSEL1 as *mut u32, gpfsel1_val);

        // Set up GPIO pin 15 pull up/down state
        // Mask all but bits 30:31
        write_and_volatile(GPIO_PUP_PDN_CNTRL_REG0 as *mut u32, 0x3fffffff);

        // Set GPIO pins 15 to be used for ALT5 - UART1 (miniuart)
        let mut gpfsel1_val = read_volatile(GPFSEL1 as *const u32);
        // Mask all but bits 15:17 (pin 15)
        gpfsel1_val &= 0xfffc7fff;
        // Pin 15, ALT5
        gpfsel1_val |= 0x00010000;
        write_volatile(GPFSEL1 as *mut u32, gpfsel1_val);
    }

    // Enable miniuart
    unsafe {
        write_volatile(AUX_MU_CNTL as *mut u32, 3);
    }
}

#[cfg(platform = "virt")]
#[unsafe(no_mangle)]
pub extern "C" fn init_early_uart() {}

// Extern no mangle so we can call from asm if necessary
//
// Raspi: miniuart -- poll LSR bit5 (THRE), then write the IO data register.
// Virt: PL011 at the fixed PA 0x09000000 -- poll FR bit5 (TXFF), then write
// DR (offset 0x00).
#[cfg(not(platform = "virt"))]
#[unsafe(no_mangle)]
pub extern "C" fn init_early_uart_putc(b: u8) {
    unsafe {
        while read_volatile(AUX_MU_LSR as *const u32) & (1 << 5) == 0 {
            core::hint::spin_loop();
        }
        write_volatile(AUX_MU_IO as *mut u32, b as u32);
    }
}

#[cfg(platform = "virt")]
#[unsafe(no_mangle)]
pub extern "C" fn init_early_uart_putc(b: u8) {
    const PL011_BASE: u32 = 0x0900_0000;
    const PL011_DR: u32 = PL011_BASE; // DR is at offset 0x00
    const PL011_FR: u32 = PL011_BASE + 0x18;

    unsafe {
        while read_volatile(PL011_FR as *const u32) & (1 << 5) != 0 {
            core::hint::spin_loop();
        }
        write_volatile(PL011_DR as *mut u32, b as u32);
    }
}

pub fn putstr(s: &str) {
    for b in s.bytes() {
        init_early_uart_putc(b);
    }
}

pub fn putu64h(v: u64) {
    putstr("0x");
    for i in 0..16 {
        let a = ((v >> ((15 - i) * 4)) & 0xf) as u8;
        if a < 10 {
            init_early_uart_putc(('0' as u8 + a) as u8);
        } else {
            init_early_uart_putc(('a' as u8 + a - 10) as u8);
        }
    }
}

pub fn puttable(table: &Table) {
    putstr("table (");
    putu64h(table as *const _ as u64);
    putstr("):\n");
    for i in 0..512 {
        let entry = table.entries[i as usize].0;
        if entry == 0 {
            continue;
        }
        putstr("  [");
        putu64h(i);
        putstr("]: ");
        putu64h(entry);
        putstr("\n");
    }
}

unsafe fn write_or_volatile<T: BitOrAssign>(dst: *mut T, val: T) {
    unsafe {
        let mut new_val = read_volatile(dst);
        new_val |= val;
        write_volatile(dst, new_val);
    }
}

unsafe fn write_and_volatile<T: BitAndAssign>(dst: *mut T, val: T) {
    unsafe {
        let mut new_val = read_volatile(dst);
        new_val &= val;
        write_volatile(dst, new_val);
    }
}
