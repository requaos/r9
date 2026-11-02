// Space reserved for the interrupt stack:
//    interruptstackbase to interruptstackbase+INTERRUPTSTACKSZ.
// (04-05: traps no longer switch stacks — the frame goes on the current
// kernel stack, see handle_interrupt below.  The reservation stays for
// print_stacks()'s reporting and any future dedicated-irq-stack need.)
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
//   SP_EL0, valid at any point after entry.  EL1h-origin traps: the entry
//   sp (recomputed as sp+288 after the frame alloc — Pitfall 1).
// - pc  @256: ELR_EL1 (exception link register — interrupted address)
// - psr @264: SPSR_EL1 (saved program status)
// - type @272: the vector index 0-15 (kernel-side informational)
// ESR_EL1/FAR_EL1 are NOT saved in the frame: trap() reads them live at
// entry and passes them to dispatchers as arguments (D-05).
//
// The frame goes on the CURRENT kernel stack — no stack switch (the 9front
// arm64 shape): EL1h-origin traps land on the interrupted kernel stack, and
// EL0-origin traps land on SP_EL1, which trap_return leaves at the trapping
// process's kernel stack top (main9's stack before the first exec).  04-05
// replaced the old switch-to-interruptstackbase entry: it clobbered x0 with
// the stack address BEFORE the first store (every frame's x0 slot held the
// interrupt stack top — fatal to the SVC number and to EL0 returns), and
// the switched sp was never restored on EL1h returns (harmless only because
// the wfi idle loop was the sole EL1 resident).  With no switch, no scratch
// register is needed before the first store and every slot stays truthful.
//
// 04-07 (D-13, T-04-14): the "current kernel stack" for an EL0-origin
// entry is the CURRENT PROCESS's kernel stack.  SP_EL1 is kept equal to
// the current proc's kstack top by the trap_return tail, which loads it
// from CURKSTACK on every EL0t-bound return (see below) — an explicit
// entry-side stack switch is impossible on aarch64 without clobbering a
// user GPR before its store (the 04-05 bug above): there is no GPR-free
// SP move, so the return-side load is the single enforcement point.  The
// entry RECORDS the frame it built into CURFRAME and the tail RELOADS the
// frame base from CURFRAME, because preemption (proc::preempt from the
// timer tick) swaps CURFRAME/CURKSTACK/TTBR0 mid-trap and the tail must
// pop the NEW current proc's frame.  Both symbols are Rust statics in
// proc.rs, written only under the PROC_TABLE lock or in IRQ-masked trap
// context (single CPU); CURKSTACK initialises to the legacy interrupt
// stack so pre-process traps (early boot, virtpanic) behave as before.
//
// The second macro parameter selects the per-origin sp capture: 1 for
// EL1-origin vectors (el1t/el1h), 0 for EL0-origin vectors (el0_64/el0_32).
.macro handle_interrupt type, from_el1
	sub	sp, sp, #288

	// Caller-saved registers, FP — nothing is clobbered before the saves.
	stp 	x0, x1, [sp, #16 * 0]
	stp 	x2, x3, [sp, #16 * 1]
	stp	x4, x5, [sp, #16 * 2]
	stp	x6, x7, [sp, #16 * 3]
	stp	x8, x9, [sp, #16 * 4]
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

	// LR @240, then the per-origin sp @248
	str	x30, [sp, #16 * 15]
.if \from_el1
	add	x30, sp, #288		// EL1h: the entry sp
.else
	mrs	x30, sp_el0		// EL0: the user sp
.endif
	str	x30, [sp, #16 * 15 + 8]

	// pc @256 (ELR_EL1), psr @264 (SPSR_EL1) — x30 is the scratch now
	// (its slot was stored above)
	mrs	x30, elr_el1
	str	x30, [sp, #16 * 16]
	mrs	x30, spsr_el1
	str	x30, [sp, #16 * 16 + 8]

	// Interrupt type @272
	ldr	x30, =\type
	str	x30, [sp, #16 * 17]

	// 04-07: record the live frame for trap_return's CURFRAME reload
	// (preemption may swap it mid-trap).  x9/x10's frame slots were
	// stored above, so they are safe scratch here.
	mov	x9, sp
	ldr	x10, =CURFRAME
	str	x9, [x10]

	// Pass pointer to TrapFrame (on stack) as the first arg
	mov	x0, sp
	bl	trap_unsafe

	// The restore tail is factored out of line (04-05): shared with exec's
	// first EL0 entry below.  sp still names the frame here.
	b	trap_return
.endm

// 04-05: the shared trap-return tail — restore the frame and eret.
// The handle_interrupt macro branches here after trap_unsafe, and exec's
// first EL0 entry arrives via the trap_return_exec wrapper at the bottom;
// this eret path is the ONLY way EL0 is ever entered (swtch never enters
// EL0 — D-12 era rule).
//
// 04-07 (D-13): the tail RELOADS the frame base from CURFRAME as its
// first instruction — a timer tick may have preempted the interrupted
// process mid-trap, swapping CURFRAME/CURKSTACK/TTBR0 to the next
// process, so the frame to restore is not necessarily the one the entry
// built.  The final stack-pointer step is per-origin: an EL1h-bound
// return reproduces the interrupted kernel sp exactly (frame+288); an
// EL0t-bound return loads SP_EL1 from CURKSTACK — the CURRENT process's
// kernel stack top — the single enforcement point of the per-process
// entry-stack invariant (see the file-header comment).
//
// Rust-callable contract (declared in trap.rs):
//	pub extern "C" fn trap_return(frame: *mut TrapFrame) -> !
//
// Restore asymmetry (04-RESEARCH.md Pitfall 2): pc/psr always; SP_EL0 only
// when the frame is EL0t-bound (psr & 0xF == 0) — an IRQ that interrupted
// EL1 kernel code must never scribble a kernel address into SP_EL0 (the
// 9front vtrap1/noteret restore asymmetry).  x9/x10 are scratch here; their
// frame slots restore in the pop below.
.globl	trap_return
trap_return:
	// Reload the current frame — preemption may have swapped it.
	// (SP cannot be an ldr destination on aarch64 — go via x9.)
	ldr	x9, =CURFRAME
	ldr	x9, [x9]
	mov	sp, x9

	// Restore system registers from the frame.
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

	// Remaining frame slots are system registers, restored above.  The
	// final sp is per-origin (the pops used offset addressing, so sp
	// still names the frame base here).
	ldr	x9, [sp, #16 * 16 + 8]	// psr
	tst	x9, #0xf
	b.ne	2f
	ldr	x9, =CURKSTACK		// EL0t-bound: SP_EL1 := the CURRENT proc's
	ldr	x9, [x9]		// kernel stack top (explicit, not frame+288
	mov	sp, x9			// arithmetic) — the entry-stack invariant
	b	3f
2:
	add	sp, sp, #288		// EL1h-bound: the interrupted kernel sp
3:
	eret

// exec's first EL0 entry (04-05): x0 = the seeded forkret-shaped frame on
// the proc's kernel stack; publish it as the current frame (04-07 — the
// tail reloads from CURFRAME) and ride the shared tail above.  Bound in
// Rust as trap_return (trap.rs).
.globl	trap_return_exec
trap_return_exec:
	ldr	x9, =CURFRAME
	str	x0, [x9]
	b	trap_return


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
