// Space reserved for interrupt stack:
//    interruptstackbase to interruptstackbase+INTERRUPTSTACKSZ.
// Stack base address (bottom):
//    interruptstackbase+INTERRUPTSTACKSZ
// Stack pointer will reduce in value as data is pushed to the stack.
INTERRUPTSTACKSZ = 4096*16

.bss
.balign	4096
.globl	interruptstackbase
interruptstackbase:	.space INTERRUPTSTACKSZ
.globl	interruptstacksz
.equ	interruptstacksz,	INTERRUPTSTACKSZ

.section .text

.equ	SYNC_INVALID_EL1t,	0
.equ	IRQ_INVALID_EL1t,	1
.equ	FIQ_INVALID_EL1t,	2
.equ	ERROR_INVALID_EL1t,	3

.equ	SYNC_INVALID_EL1h,	4
.equ	IRQ_INVALID_EL1h,	5
.equ	FIQ_INVALID_EL1h,	6
.equ	ERROR_INVALID_EL1h,	7

.equ	SYNC_INVALID_EL0_64,	8
.equ	IRQ_INVALID_EL0_64,	9
.equ	FIQ_INVALID_EL0_64,	10
.equ	ERROR_INVALID_EL0_64,	11

.equ	SYNC_INVALID_EL0_32,	12
.equ	IRQ_INVALID_EL0_32,	13
.equ	FIQ_INVALID_EL0_32,	14
.equ	ERROR_INVALID_EL0_32,	15

// Ventry aligns to 128 bytes and branches to label, and is used for the interrupt vector table
.macro	ventry	label
.balign	128
	b	\label
.endm

// One unified frame serves all 16 vectors (D-08): the TrapFrame IS Harvey's
// Ureg (aarch64/include/ureg.h) field-for-field — x0..x28, x29, x30, sp, pc,
// psr, type (aarch64/ABI.md, "Kernel-side enforcement (Phase 4 appendix)").
// We save all general purpose registers, and these system registers:
// - sp  @248: the interrupted context's stack pointer.  EL0-origin traps:
//   SP_EL0, valid at any point after the stack switch.  EL1-origin traps:
//   the live entry-sp BEFORE the switch — on EL1h, MRS SP_EL0 would yield a
//   stale user SP, not a kernel-context value (Pitfall 1).
// - pc  @256: ELR_EL1 (exception link register — interrupted address)
// - psr @264: SPSR_EL1 (saved program status)
// - type @272: the vector index 0-15 (kernel-side informational)
// ESR_EL1/FAR_EL1 are NOT saved in the frame: trap() reads them live at
// entry and passes them to dispatchers as arguments (D-05).
//
// The second macro parameter selects the per-origin sp capture: 1 for
// EL1-origin vectors (el1t/el1h), 0 for EL0-origin vectors (el0_64/el0_32).
.macro handle_interrupt type, from_el1
.if \from_el1
	// Capture the live entry-sp BEFORE the stack switch destroys it.  x9
	// is the carrier, so park the interrupted x9 on the interrupted
	// kernel stack first and recover it into its frame slot below — the
	// published Ureg keeps every slot truthful (and the restore pop
	// doesn't resurrect the carrier value into the resumed context).
	sub	sp, sp, #16
	str	x9, [sp]
	mov	x9, sp
	add	x9, x9, #16
.endif

	// Switch to the interrupt stack
	ldr	x0, =interruptstackbase
	add	x0, x0, #INTERRUPTSTACKSZ
	mov	sp, x0

	sub 	sp, sp, #288

	// Caller-saved registers, FP
	stp 	x0, x1, [sp, #16 * 0]
	stp 	x2, x3, [sp, #16 * 1]
	stp	x4, x5, [sp, #16 * 2]
	stp	x6, x7, [sp, #16 * 3]
.if \from_el1
	str	x8, [sp, #16 * 4]
	// Recover the interrupted x9 into its frame slot (x3 is free — its
	// slot was stored above).  This must precede the type store: when the
	// trap re-enters on the interrupt stack itself, the parked bytes ARE
	// this frame's type slot (entry-sp == interruptstackbase+SZ there).
	ldr	x3, [x9, #-16]
	str	x3, [sp, #16 * 4 + 8]
.else
	stp	x8, x9, [sp, #16 * 4]
.endif
	stp	x10, x11, [sp, #16 * 5]
	stp	x12, x13, [sp, #16 * 6]
	stp	x14, x15, [sp, #16 * 7]
	stp	x16, x17, [sp, #16 * 8]
	stp	x18, x19, [sp, #16 * 9]
	stp	x20, x21, [sp, #16 * 10]
	stp	x22, x23, [sp, #16 * 11]
	stp	x24, x25, [sp, #16 * 12]
	stp	x26, x27, [sp, #16 * 13]
	stp	x28, x29, [sp, #16 * 14]

	// LR @240, sp @248
.if \from_el1
	stp	x30, x9, [sp, #16 * 15]
.else
	mrs	x9, sp_el0
	stp	x30, x9, [sp, #16 * 15]
.endif

	// pc @256 (ELR_EL1), psr @264 (SPSR_EL1)
	mrs	x1, elr_el1
	mrs	x10, spsr_el1
	stp	x1, x10, [sp, #16 * 16]

	// Interrupt type @272
	ldr	x3, =\type
	str	x3, [sp, #16 * 17]

	// Pass pointer to TrapFrame (on stack) as the first arg
	mov	x0, sp
	bl	trap_unsafe

	// Restore system registers from the frame (Pitfall 2): pc/psr always;
	// sp only when the frame returns to EL0t (psr & 0xF == 0) — an IRQ
	// that interrupted EL1 kernel code must never scribble a kernel
	// address into SP_EL0 (9front vtrap1/noteret restore asymmetry).
	// x9/x10 are scratch here; their frame slots restore in the pop below.
	ldr	x10, [sp, #16 * 16 + 8]	// psr
	tst	x10, #0xf
	b.ne	1f			// not EL0t — skip the SP_EL0 write
	ldr	x9, [sp, #16 * 15 + 8]	// sp
	msr	sp_el0, x9
1:
	ldr	x9, [sp, #16 * 16]	// pc
	msr	elr_el1, x9
	msr	spsr_el1, x10

	// Restore caller-saved registers
	ldp	x0, x1, [sp, #16 * 0]
	ldp	x2, x3, [sp, #16 * 1]
	ldp	x4, x5, [sp, #16 * 2]
	ldp	x6, x7, [sp, #16 * 3]
	ldp	x8, x9, [sp, #16 * 4]
	ldp	x10, x11, [sp, #16 * 5]
	ldp	x12, x13, [sp, #16 * 6]
	ldp	x14, x15, [sp, #16 * 7]
	ldp	x16, x17, [sp, #16 * 8]
	ldp	x18, x19, [sp, #16 * 9]
	ldp	x20, x21, [sp, #16 * 10]
	ldp	x22, x23, [sp, #16 * 11]
	ldp	x24, x25, [sp, #16 * 12]
	ldp	x26, x27, [sp, #16 * 13]
	ldp	x28, x29, [sp, #16 * 14]
	ldr	x30, [sp, #16 * 15]

	// Remaining stack frame slots are system registers, restored above
	add	sp, sp, #288

	eret
.endm

/// The exception vector table for exceptions taken to EL1.
/// Each entry is 16 instructions/128 bytes.
/// Ventry handles alignment of individual entries.
.balign	2048
.globl exception_vectors
exception_vectors:
	// Current EL with SP0
	ventry	sync_invalid_el1t			// Synchronous EL1t
	ventry	irq_invalid_el1t			// IRQ EL1t
	ventry	fiq_invalid_el1t			// FIQ EL1t
	ventry	error_invalid_el1t			// Error EL1t

	// Current EL with SPx
	ventry	sync_invalid_el1h			// Synchronous EL1h
	ventry	irq_invalid_el1h			// IRQ EL1h
	ventry	fiq_invalid_el1h			// FIQ EL1h
	ventry	error_invalid_el1h			// Error EL1h

	// Lower EL using AArch64
	ventry	sync_invalid_el0_64			// Synchronous 64-bit EL0
	ventry	irq_invalid_el0_64			// IRQ 64-bit EL0
	ventry	fiq_invalid_el0_64			// FIQ 64-bit EL0
	ventry	error_invalid_el0_64			// Error 64-bit EL0

	// Lower EL using AArch32
	ventry	sync_invalid_el0_32			// Synchronous 32-bit EL0
	ventry	irq_invalid_el0_32			// IRQ 32-bit EL0
	ventry	fiq_invalid_el0_32			// FIQ 32-bit EL0
	ventry	error_invalid_el0_32			// Error 32-bit EL0

sync_invalid_el1t:
	handle_interrupt  SYNC_INVALID_EL1t, 1

irq_invalid_el1t:
	handle_interrupt  IRQ_INVALID_EL1t, 1

fiq_invalid_el1t:
	handle_interrupt  FIQ_INVALID_EL1t, 1

error_invalid_el1t:
	handle_interrupt  ERROR_INVALID_EL1t, 1

sync_invalid_el1h:
	handle_interrupt  SYNC_INVALID_EL1h, 1

irq_invalid_el1h:
	handle_interrupt  IRQ_INVALID_EL1h, 1

fiq_invalid_el1h:
	handle_interrupt  FIQ_INVALID_EL1h, 1

error_invalid_el1h:
	handle_interrupt  ERROR_INVALID_EL1h, 1

sync_invalid_el0_64:
	handle_interrupt  SYNC_INVALID_EL0_64, 0

irq_invalid_el0_64:
	handle_interrupt  IRQ_INVALID_EL0_64, 0

fiq_invalid_el0_64:
	handle_interrupt  FIQ_INVALID_EL0_64, 0

error_invalid_el0_64:
	handle_interrupt  ERROR_INVALID_EL0_64, 0

sync_invalid_el0_32:
	handle_interrupt  SYNC_INVALID_EL0_32, 0

irq_invalid_el0_32:
	handle_interrupt  IRQ_INVALID_EL0_32, 0

fiq_invalid_el0_32:
	handle_interrupt  FIQ_INVALID_EL0_32, 0

error_invalid_el0_32:
	handle_interrupt  ERROR_INVALID_EL0_32, 0
