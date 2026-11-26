// Aarch64 entry (Raspberry Pi 4 focused)

// 16 KiB -> 64 KiB.  The tick (1 kHz, the preemption
// source) lands its 288-byte trap frame + handler frames on THIS stack
// whenever the timer fires in the unmasked boot window (IRQ-masked trap
// context otherwise) — and exec_bootstrap's deep chains (paqfs inflate,
// fmt printing) left < 1 KiB of headroom, so a tick mid-inflate spilled
// into adjacent BSS and corrupted the heap (observed: intermittent
// "paqfs: inflate failed" panics).  The stack serves only the boot path
// (main9 never returns; exec_bootstrap diverges into EL0), so the BSS
// cost is boot-time-only.
STACKSZ = 4096*16

// Per-CPU secondary boot stack size (64 KiB, matching the BSP's).
// Secondary CPUs switch to their own stack after the MMU is on.
SECONDARY_STACKSZ = 4096*16
// The most secondary CPUs the stack area covers (QEMU virt's GICv2
// limit; the per-CPU table's cap lives in aarch64/src/smp.rs's MAX_CPUS).
MAX_SECONDARY_CPUS = 8

CURRENTEL_EL			= (1<<3) | (1<<2)

SCR_EL3_NS			= (1<<0)
SCR_EL3_SMD			= (1<<7)
SCR_EL3_HCE			= (1<<8)
SCR_EL3_RW			= (1<<10)

SPSR_EL3_M_EL2H			= (1<<3) | (1<<0)	// Exception level and SP: EL2H
SPSR_EL3_F			= (1<<6)		// FIQ
SPSR_EL3_I			= (1<<7)		// IRQ
SPSR_EL3_A			= (1<<8)		// SError
SPSR_EL3_D			= (1<<9)		// Debug exception

HCR_EL2_RW			= (1<<31)

SPSR_EL2_M_EL1H			= (1<<2) | (1<<0)	// Exception level and SP: EL1h
SPSR_EL2_F			= (1<<6)		// FIQ
SPSR_EL2_I			= (1<<7)		// IRQ
SPSR_EL2_A			= (1<<8)		// SError
SPSR_EL2_D			= (1<<9)		// Debug exception

CPACR_EL1_FPEN			= (1<<21) | (1<<20)	// Don't trap FPU instr at EL1,0

SCTLR_EL1_I			= (1 << 12)		// Instruction access cacheability
SCTLR_EL1_C			= (1 << 2)		// Data cacheability
SCTLR_EL1_M			= (1 << 0)		// Enable MMU

// This defines the kernel's virtual address location.
// This value splits a 48 bit address space exactly in half, with the half
// beginning with 1 going to the kernel.
KZERO				= 0xffff800000000000

.section .boottext, "awx"
.globl start
start:
	// We use some registers throught this assembly code.  They shouldn't be
	// used by any code in this file.  Once we call main9, they can be
	// used again.  There's also a couple that are best avoided out of
	// principle.

	// x27: DTB address (physical address)
	// x28: Entrypoint address
	// x29: Frame pointer
	// x30: Link register
	mov	x27, x0			// Cache dtb pointer so we can pass to main9 later
	mov	x28, x4			// Cache entrypoint (offset)

	// All cores other than 0 should just hang
	mrs	x0, mpidr_el1
	and	x0, x0, #0xff
	cbnz	x0, dnr

	// Aarch64 has 4 exception levels:
	//  EL0 - Application level
	//  EL1 - OS
	//  EL2 - Hypervisor
	//  EL3 - Firmware
	// We want to be in EL1.  Qemu starts in EL3.  Raspi3 usually starts in EL2.

	// Dispatch to code to handle the current exception level
	mrs	x0, CurrentEL
	and	x0, x0, CURRENTEL_EL
	lsr	x0, x0, #2
	cmp	x0, #1
	beq	el1
	cmp	x0, #2
	beq	el2

	// Must be EL3, so prepare jump to EL2
	ldr	x0, =(SCR_EL3_NS|SCR_EL3_SMD|SCR_EL3_HCE|SCR_EL3_RW)
	msr	scr_el3, x0
	ldr	x0, =(SPSR_EL3_M_EL2H|SPSR_EL3_F|SPSR_EL3_I|SPSR_EL3_A|SPSR_EL3_D)
	msr	spsr_el3, x0

	// Return to EL2
	adr	x0, el2
	msr	elr_el3, x0
	eret

el2:	// Now in EL2, so prepare jump to EL1
	// Enable AArch64 in EL1
	ldr	x0, =HCR_EL2_RW
	msr	hcr_el2, x0
	ldr	x0, =(SPSR_EL2_M_EL1H|SPSR_EL2_F|SPSR_EL2_I|SPSR_EL2_A|SPSR_EL2_D)
	msr	spsr_el2, x0

	// Enable FPU in EL1, EL0
	ldr	x0, =CPACR_EL1_FPEN
	msr	cpacr_el1, x0

	// Return to EL1
	adr	x0, el1
	msr	elr_el2, x0
	eret

el1:	// In EL1

	// Set up the initial stack using the physical address
	ldr	x0, =stack
	ldr	x2, =(KZERO)
	sub	x0, x0, x2
	add	x0, x0, #STACKSZ
	mov	sp, x0

	// Clear BSS
	ldr	x0, =bss_pa
	ldr	x1, =ebss_pa
1:	str	xzr, [x0], #8
	cmp	x0, x1
	b.ne	1b
	// Clear early pagetables
	ldr	x0, =earlyvm_pagetables_pa
	ldr	x1, =eearlyvm_pagetables_pa
1:	str	xzr, [x0], #8
	cmp	x0, x1
	b.ne	1b

	bl	init_early_uart

	mov	x0, #'.'
	bl	init_early_uart_putc

	mov	x0, x27			// DTB pointer
	bl	init_vm

	// Force changes to be be seen before MMU enabled, then enable MMU
	dsb sy
	mrs	x0, sctlr_el1
	ldr	x1, =(SCTLR_EL1_I|SCTLR_EL1_C|SCTLR_EL1_M)
	orr	x0, x0, x1
	msr	sctlr_el1, x0		// Enable MMU!

	// Force changes to be be seen by next instruction.
	// At this point the PC is still in the lower half, so we need to jump
	// up to the higher half.
	isb
	mrs	x0, elr_el1
	ldr	x20, =(higher_half)
	br	x20

higher_half:
	// Now that the kernel is mapped, the MMU is enabled and we're in the
	// higher half, we can set up the initial stack using the virtual address.
	ldr	x0, =stack
	add	x0, x0, #STACKSZ
	mov	sp, x0

	// Jump to rust, passing DTB pointer (in x27, then map to upper half)
	ldr	x0, =(KZERO)
	add	x0, x0, x27
	bl	main9

.globl dnr
dnr:	wfe
	b	dnr

// ---------------------------------------------------------------------------
// Secondary CPU boot state (SMP-04) — published by the BSP's
// publish_boot_state() before PSCI CPU_ON, read by secondary_start at
// physical addresses (MMU off).  Four u64s, layout consumed as a block:
// [0] MAIR_EL1, [8] TCR_EL1, [16] TTBR0_EL1, [24] TTBR1_EL1.
// ---------------------------------------------------------------------------
.section .bootdata, "aw"
.balign 8
.globl SECONDARY_MAIR
SECONDARY_MAIR:		.space 8
.globl SECONDARY_TCR
SECONDARY_TCR:		.space 8
.globl SECONDARY_TTBR0
SECONDARY_TTBR0:	.space 8
.globl SECONDARY_TTBR1
SECONDARY_TTBR1:	.space 8

.section .boottext, "awx"
// ---------------------------------------------------------------------------
// Secondary CPU entry (SMP-04) — PSCI CPU_ON target.
//
// x0 = cpu_id (PSCI passes the CPU_ON x3 context as the started CPU's
// x0).  The CPU starts with MMU off, at EL2 (HVF) or EL1 (TCG).  The BSP
// has already set up the page tables and published the MMU state in the
// .bootdata globals below (read at physical addresses — the MMU is off).
// ---------------------------------------------------------------------------
.globl secondary_start
secondary_start:
	// Save cpu_id in a callee-saved register (x19 survives the Rust
	// call below).
	mov	x19, x0

	// Dispatch to code to handle the current exception level
	mrs	x0, CurrentEL
	and	x0, x0, CURRENTEL_EL
	lsr	x0, x0, #2
	cmp	x0, #1
	beq	secondary_el1
	cmp	x0, #2
	beq	secondary_el2

	// EL3: QEMU virt PSCI starts secondaries at EL2 or EL1; an EL3
	// arrival is a platform bug.
	b	dnr

secondary_el2:
	// Drop EL2 -> EL1: enable AArch64 in EL1, leave the timer
	// untrapped, zero the virtual-count offset.
	ldr	x0, =HCR_EL2_RW
	msr	hcr_el2, x0
	msr	cnthctl_el2, xzr
	msr	cntvoff_el2, xzr
	ldr	x0, =(SPSR_EL2_M_EL1H|SPSR_EL2_F|SPSR_EL2_I|SPSR_EL2_A|SPSR_EL2_D)
	msr	spsr_el2, x0

	// Enable FPU in EL1, EL0
	ldr	x0, =CPACR_EL1_FPEN
	msr	cpacr_el1, x0

	// Return to EL1
	adr	x0, secondary_el1
	msr	elr_el2, x0
	eret

secondary_el1:
	// Load the BSP's published MMU state.  The globals live in
	// .bootdata (identity-mapped pre-MMU); translate the link VAs
	// to PAs by subtracting KZERO.
	ldr	x0, =SECONDARY_MAIR
	ldr	x1, =(KZERO)
	sub	x0, x0, x1		// PA of SECONDARY_MAIR
	ldr	x2, [x0]		// MAIR_EL1 value
	ldr	x3, [x0, #8]		// TCR_EL1 value
	ldr	x4, [x0, #16]		// TTBR0_EL1 value (boot identity table)
	ldr	x5, [x0, #24]		// TTBR1_EL1 value (kernel root table)

	msr	mair_el1, x2
	msr	tcr_el1, x3
	msr	ttbr0_el1, x4
	msr	ttbr1_el1, x5

	// Invalidate local TLBs (not yet inner-shareable — the MMU is
	// off, so this CPU's TLB is the only one that matters).
	tlbi	vmalle1
	dsb	nsh
	isb

	// Enable MMU + I/D caches.
	dsb	ish
	mrs	x0, sctlr_el1
	ldr	x1, =(SCTLR_EL1_I|SCTLR_EL1_C|SCTLR_EL1_M)
	orr	x0, x0, x1
	msr	sctlr_el1, x0		// Enable MMU!

	// Force changes to be seen by next instruction, then jump to
	// the high-half KZERO virtual address.
	isb
	ldr	x20, =(secondary_higher_half)
	br	x20

secondary_higher_half:
	// Now running with MMU on at KZERO-mapped addresses.

	// Install the exception vectors.
	adr	x0, exception_vectors
	msr	vbar_el1, x0

	// Set TPIDR_EL1 = &PER_CPU[cpu_id].  The stride is the
	// Rust-exported PER_CPU_STRIDE (= sizeof(PerCpu), pinned by the
	// const assert in smp.rs).
	ldr	x0, =PER_CPU
	ldr	x1, =PER_CPU_STRIDE
	ldr	x1, [x1]		// sizeof(PerCpu)
	madd	x0, x19, x1, x0		// &PER_CPU[0] + cpu_id * stride
	msr	tpidr_el1, x0

	// Load this CPU's dedicated boot stack (top of its 64 KiB slot).
	ldr	x0, =SECONDARY_STACKS
	mov	x1, #SECONDARY_STACKSZ
	madd	x0, x19, x1, x0		// SECONDARY_STACKS + cpu_id * SZ
	add	x0, x0, #SECONDARY_STACKSZ	// top of the slot
	mov	sp, x0

	// Call secondary_main(cpu_id).
	mov	x0, x19
	bl	secondary_main

	// secondary_main never returns; park if it somehow does.
	b	dnr

.bss
.balign	4096
stack:	.space STACKSZ

// Per-CPU secondary boot stacks (64 KiB each).  Zeroed by the BSP's BSS
// clear at boot; the secondary CPU switches to its slot after the MMU is
// on (the slot VA is in the kernel's higher-half BSS mapping).
.balign	4096
.globl SECONDARY_STACKS
SECONDARY_STACKS:	.space SECONDARY_STACKSZ * MAX_SECONDARY_CPUS
