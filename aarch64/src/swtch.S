// Context switch
//
//   pub unsafe extern "C" fn swtch(from: &mut Context, to: &Context)
//
// Save the callee-saved registers + sp + lr into `from` (the repr(C)
// Context struct layout in swtch.rs), load them from `to`, and `ret` to
// to.x30.  Symmetric: a Context saved here resumes correctly as a later
// `to`.
//
// The retired sequence below was never called on aarch64 and could not
// round-trip: it pushed x19..x30 in the SAME order it popped them (a
// descending stack then maps every register to a different slot on
// resume), stored a mid-sequence sp (not the Context base) into *from,
// discarded the saved sp on resume, and sourced SPSR_EL1/ELR_EL1 from
// stack garbage via a trailing eret.  Only kernel-context (blocked
// mid-syscall) switches ride this path — EL0 entry/exit NEVER does
// (trap_return's tail is the only EL0 entry).

.section .text

.globl swtch
swtch:
	// (1) Save into *from (x0): Context field order x19..x28, x29, x30,
	//     sp, spsr (offsets 0..104).
	mov	x9, sp
	stp	x19, x20, [x0, #16 * 0]
	stp	x21, x22, [x0, #16 * 1]
	stp	x23, x24, [x0, #16 * 2]
	stp	x25, x26, [x0, #16 * 3]
	stp	x27, x28, [x0, #16 * 4]
	stp	x29, x30, [x0, #16 * 5]
	str	x9, [x0, #96]		// context.sp (spsr slot unused this era)

	// (2) Load from *to (x1) — the same layout.
	ldp	x19, x20, [x1, #16 * 0]
	ldp	x21, x22, [x1, #16 * 1]
	ldp	x23, x24, [x1, #16 * 2]
	ldp	x25, x26, [x1, #16 * 3]
	ldp	x27, x28, [x1, #16 * 4]
	ldp	x29, x30, [x1, #16 * 5]
	ldr	x9, [x1, #96]
	mov	sp, x9

	// (3) Return to to.x30 — for a previously-saved context, the
	//     instruction after its own swtch call.
	ret
