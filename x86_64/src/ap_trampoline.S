// x86_64/src/ap_trampoline.S — the 16->32->64-bit SIPI AP bring-up
// trampoline (SMP-01, D-25; 05-RESEARCH Pattern 3.5).
//
// smp.rs's start_aps copies ap_trampoline_start..ap_trampoline_end to
// physical page 0x8000 (SIPI vector 0x08: the AP starts executing at
// vector << 12 in 16-bit real mode with CS=0x0800, IP=0) and publishes
// the handoff mailbox at 0x8F00 (inside the same page) before the
// INIT-SIPI-SIPI sequence:
//
//   0x8F00  u64  pml4_pa      — the BSP's bootpml4 PA (identity-maps
//                               the first 4 GiB, so this page keeps
//                               executing at PA == VA once paging is on)
//   0x8F08  u64  ap_stack_top — the AP's 64 KiB boot stack top (its
//                               Mach's own KStack; a high-half VA)
//   0x8F10  u64  ap_mach_va   — the AP's Mach VA (ap_main's rdi)
//   0x8F18  u64  cpu_id       — the AP's logical id (ap_main's rsi)
//   0x8F20  u64  ap_entry_va  — ap_main's high-half VA
//
// Position-independent: every internal reference is computed against the
// KNOWN load base (0x8000), never the link address, so the copy — not
// the kernel image — is what executes.  Like l.S's b1978 generation,
// this file assembles into .rodata on purpose: as far as the running
// kernel is concerned these are read-only data bytes, and no CPU ever
// jumps into the kernel's own copy.
//
// The code must fit below the mailbox: the compile-time assert at the
// bottom (and smp.rs's runtime assert) pins
// ap_trampoline_end - ap_trampoline_start < 0xF00.

.set AP_BASE,		0x8000
.set AP_MAILBOX,	0x8F00
.set MB_PML4_PA,	(AP_MAILBOX + 0x00)
.set MB_STACK_TOP,	(AP_MAILBOX + 0x08)
.set MB_MACH_VA,	(AP_MAILBOX + 0x10)
.set MB_CPU_ID,		(AP_MAILBOX + 0x18)
.set MB_ENTRY_VA,	(AP_MAILBOX + 0x20)

// Embedded GDT selectors (byte offsets into ap_gdt).
.set SelNULL,		(0<<3)
.set SelCODE64,		(1<<3)
.set SelCODE32,		(2<<3)
.set SelDATA32,		(3<<3)

// Segment descriptor bits (l.S's vocabulary, re-keyed).
.set SegREAD,		(1<<41)
.set SegWRITE,		(1<<42)
.set SegCODE,		(1<<43)
.set SegMB1,		(1<<44)
.set SegPRESENT,	(1<<47)
.set SegLONG,		(1<<53)
.set Seg32DEFAULT,	(1<<54)
.set Seg32GRAN,		(1<<55)
.set Seg32LIMIT,	((0xF<<48)+0xFFFF)
.set Seg32DEF,		(Seg32DEFAULT|Seg32GRAN|Seg32LIMIT)

.set Cr0PE,		(1<<0)		// Protected Mode Enable
.set Cr0WP,		(1<<16)		// Write Protect
.set Cr0PG,		(1<<31)		// Paging Enable

.set Cr4PAE,		(1<<5)		// Physical Address Extension
.set Cr4PGE,		(1<<7)		// Page-Global Enable
.set Cr4OSFXSR,		(1<<9)		// OS Support for FXSAVE/FXRSTOR+SIMD
.set Cr4OSXMMEXCPT,	(1<<10)		// OS Support for unmasked SIMD exceptions

.set IA32_EFER,		0xc0000080	// Extended Feature Enable
.set EferLME,		(1<<8)		// Long Mode Enable
.set EferNXE,		(1<<11)		// No-Execute Enable

.section .rodata
.balign 4096
.globl ap_trampoline_start
.globl ap_trampoline_end

.code16
ap_trampoline_start:
	// We start here in real mode.  Welcome to 1978.
	cli
	cld
	xorw	%ax, %ax
	movw	%ax, %ds
	movw	%ax, %es
	movw	%ax, %ss

	// Load the embedded GDT (16-bit lgdt reads a 6-byte
	// pseudo-descriptor: u16 limit then u32 base; the base field below
	// carries the GDT's run-time physical address inside this page).
	lgdtl	(AP_BASE + ap_gdt_desc - ap_trampoline_start)

	// Protected mode enable.
	movl	%cr0, %eax
	orl	$Cr0PE, %eax
	movl	%eax, %cr0

	// Far jump to 32-bit protected mode (loads CS = 32-bit code
	// segment and serializes the PE transition).
	ljmpl	$SelCODE32, $(AP_BASE + ap_prot32 - ap_trampoline_start)

.balign 16
.code32
ap_prot32:
	// Welcome to 1982.  Flat 32-bit data segments.
	movw	$SelDATA32, %ax
	movw	%ax, %ds
	movw	%ax, %es
	movw	%ax, %ss
	movw	%ax, %fs
	movw	%ax, %gs

	// CR4: PAE is the long-mode requirement; PGE + the user-SSE pair
	// mirror the BSP's l.S CR4 (CPU 0 sets
	// PGE|PAE|OSFXSR|OSXMMEXCPT — an AP must not run with a weaker
	// CR4 than the kernel it joins; FSGSBASE stays off, as on the
	// BSP — TCG's qemu64 lacks it).
	movl	%cr4, %eax
	orl	$(Cr4PAE|Cr4PGE|Cr4OSFXSR|Cr4OSXMMEXCPT), %eax
	movl	%eax, %cr4

	// CR3: the BSP's bootpml4 PA from the mailbox (identity-maps the
	// first 4 GiB, so this page stays executing at PA == VA).
	movl	(MB_PML4_PA), %eax
	movl	%eax, %cr3

	// EFER: LME + NXE (matching l.S's BSP write).
	movl	$IA32_EFER, %ecx
	rdmsr
	orl	$(EferLME|EferNXE), %eax
	wrmsr

	// CR0: paging on (PE already set).
	movl	%cr0, %eax
	orl	$(Cr0PG|Cr0WP), %eax
	movl	%eax, %cr0

	// Far jump to 64-bit long mode.
	ljmpl	$SelCODE64, $(AP_BASE + ap_long64 - ap_trampoline_start)

.code64
ap_long64:
	// Welcome to 2003.  Clean segment registers, matching l.S's
	// warp64 discipline.
	xorl	%eax, %eax
	movw	%ax, %ds
	movw	%ax, %es
	movw	%ax, %ss
	movw	%ax, %fs
	movw	%ax, %gs

	// The handoff — stack, args, entry — all from the mailbox
	// (identity VA == PA under bootpml4, absolute 64-bit reads).
	movabsq	$MB_STACK_TOP, %rax
	movq	(%rax), %rsp
	movabsq	$MB_MACH_VA, %rax
	movq	(%rax), %rdi
	movabsq	$MB_CPU_ID, %rax
	movq	(%rax), %rsi
	movabsq	$MB_ENTRY_VA, %rax
	movq	(%rax), %rax
	// Bring-up marker 'T' on COM1 (raw, lock-free): the AP reached
	// 64-bit mode with the BSP's page tables.  A capture that shows no
	// 'T' means the AP died in the trampoline; a 'T' with no later
	// marker localises the fault to ap_main.
	movw	$0x3F8, %dx
	movb	$0x0a, %al
	outb	%al, %dx
	movb	$0x41, %al
	outb	%al, %dx
	movb	$0x50, %al
	outb	%al, %dx
	movb	$0x3a, %al
	outb	%al, %dx
	movb	$0x54, %al
	outb	%al, %dx
	movb	$0x0a, %al
	outb	%al, %dx

	// Zero the frame-pointer chain root and jump to ap_main (it
	// never returns).
	xorl	%ebp, %ebp
	jmpq	*%rax

.balign 16
ap_gdt:
	.quad	0						// SelNULL
	.quad	(SegREAD|SegCODE|SegMB1|SegPRESENT|SegLONG)	// SelCODE64
	.quad	(SegREAD|SegCODE|SegMB1|SegPRESENT|Seg32DEF)	// SelCODE32
	.quad	(SegREAD|SegWRITE|SegMB1|SegPRESENT|Seg32DEF)	// SelDATA32
ap_gdt_end:

.balign 16
ap_gdt_desc:
	.word	ap_gdt_end - ap_gdt - 1
	.long	(AP_BASE + ap_gdt - ap_trampoline_start)

ap_trampoline_end:

// The trampoline must fit below the 0x8F00 mailbox inside its page.
// (LLVM MC evaluates .if before section layout, so a same-section
// symbol difference is not an absolute expression there — the pin is
// smp.rs's install_trampoline runtime assert on the copied span
// instead: ap_trampoline_end - ap_trampoline_start < 0xF00.)
