//! S6 amd64 probe (D-17): the smallest real Rust program that runs as a
//! Plan 9 amd64 a.out under r9's loader.
//!
//! Wire facts, frozen from Harvey at the pin:
//! - syscall number in BP (`amd64/ABI.md`, "Wire contract");
//! - arguments in 8-byte slots at user SP+8 upward, SP+0 reserved;
//! - trap instruction `SYSCALL`;
//! - PWRITE = 51 and EXITS = 8 (`sys/src/libc/9syscall/sys.h:50`, `:9`).
//!
//! The probe ignores argc/argv and the Tos pointer in AX: PWRITE writes
//! the marker to fd 1, EXITS(0) returns. It has no data or bss; the
//! marker string rides the same flat text payload (see probe-amd64.ld).

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

core::arch::global_asm!(
    r#"
    .section .text._start,"ax",@progbits
    .global _start
_start:
    /* Reserve the Sargs slot block; SP+0 stays the reserved slot. */
    sub rsp, 64
    /* PWRITE(1, marker, len, -1): fd, buf, n, offset at SP+8..SP+32. */
    mov qword ptr [rsp + 8], 1
    lea rax, [rip + marker]
    mov qword ptr [rsp + 16], rax
    lea rcx, [rip + marker_end]
    sub rcx, rax
    mov qword ptr [rsp + 24], rcx
    mov qword ptr [rsp + 32], -1
    mov ebp, 51
    syscall
    /* EXITS(0): nil status at SP+8. */
    mov qword ptr [rsp + 8], 0
    mov ebp, 8
    syscall
1:
    jmp 1b

    .section .rodata,"a",@progbits
    .global marker
marker:
    .ascii "R9-RUST-AOUT-PROBE-OK\n"
marker_end:
    .global marker_len
    .set marker_len, marker_end - marker
    "#
);
