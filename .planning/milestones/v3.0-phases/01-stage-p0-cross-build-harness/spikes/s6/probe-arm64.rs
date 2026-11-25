//! S6 arm64 probe (D-17): the smallest real Rust program that runs as a
//! Plan 9 arm64 a.out under r9's loader.
//!
//! Wire facts, frozen from Harvey at the pin:
//! - syscall number in X0 (`arm64/ABI.md`, "Wire contract");
//! - arguments in 8-byte slots at user SP+8 upward, SP+0 the reserved
//!   link-register slot;
//! - trap instruction `SVC`;
//! - PWRITE = 51 and EXITS = 8 (`sys/src/libc/9syscall/sys.h:50`, `:9`).
//!
//! The probe ignores argc/argv and the Tos pointer in X0: PWRITE writes
//! the marker to fd 1, EXITS(0) returns. It has no data or bss; the
//! marker string rides the same flat text payload (see probe-arm64.ld).

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

core::arch::global_asm!(
    r#"
    .section .text._start,"ax",%progbits
    .global _start
_start:
    /* Reserve the Sargs slot block; SP+0 stays the reserved slot.
     * The subtraction keeps SP 16-byte aligned. */
    sub sp, sp, #64
    /* PWRITE(1, marker, len, -1): fd, buf, n, offset at SP+8..SP+32. */
    mov x1, #1
    str x1, [sp, #8]
    adrp x1, marker
    add x1, x1, :lo12:marker
    str x1, [sp, #16]
    adrp x2, marker_end
    add x2, x2, :lo12:marker_end
    sub x2, x2, x1
    str x2, [sp, #24]
    mov x1, #-1
    str x1, [sp, #32]
    mov x0, #51
    svc #0
    /* EXITS(0): nil status at SP+8. */
    str xzr, [sp, #8]
    mov x0, #8
    svc #0
1:
    b 1b

    .section .rodata,"a",%progbits
    .global marker
marker:
    .ascii "R9-RUST-AOUT-PROBE-OK\n"
marker_end:
    .global marker_len
    .set marker_len, marker_end - marker
    "#
);
