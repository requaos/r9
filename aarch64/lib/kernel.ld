/*
 * Linker script for R9.
 */

ENTRY(start)
KZERO = 0xffff800000000000;

SECTIONS {
	/* Entrypoint for Raspberry Pi will be at 0x80000 */
	. = ${LOAD-ADDRESS};


	/* Group boottext and text.  .bootdata is NOT grouped here: vminit
	   maps the boottext+text range with Entry::ro_kernel_text() (RX), so
	   the post-MMU publish_boot_state() writes to the SECONDARY_* globals
	   would fault against a read-only mapping (and wedge: VBAR_EL1 is not
	   installed yet in main9).  The globals keep the PA = VA - KZERO
	   identity wherever they link (every section's LMA tracks VMA-KZERO),
	   so l.S's secondary_start PA arithmetic is placement-independent —
	   .bootdata lands in .data (PrivRw) below instead. */
	.text.boot : AT (ADDR(.text.boot) - KZERO) {
		*(.boottext)
		. = ALIGN(4096);
	}
	PROVIDE(boottext_pa = LOADADDR(.text.boot));
	PROVIDE(eboottext_pa = LOADADDR(.text.boot) + SIZEOF(.text.boot));


	. = ALIGN(4096);
	.text : AT (ADDR(.text) - KZERO) {
		*(.text* .stub .gnu.linkonce.t.*)
		. = ALIGN(2097152);
	}
	PROVIDE(text_pa = LOADADDR(.text));
	PROVIDE(etext_pa = LOADADDR(.text) + SIZEOF(.text));


	/* RO data is in a separate page to other data */
	. = ALIGN(2097152);
	.rodata : ALIGN(2097152) {
		*(.rodata* .gnu.linkonce.r.*)
		. = ALIGN(2097152);
	}
	PROVIDE(rodata_pa = LOADADDR(.rodata));
	PROVIDE(erodata_pa = LOADADDR(.rodata) + SIZEOF(.rodata));


	/* Group data and bss.  .bootdata (the SECONDARY_* boot-publication
	   globals) links here — writable under vminit's PrivRw data mapping
	   for publish_boot_state()'s post-MMU writes; its PA (VMA-KZERO, the
	   delta inherited from .text.boot's AT) stays inside the kernel's
	   loaded physical range for l.S's pre-MMU secondary_start reads. */
	.data : ALIGN(2097152) {
		*(.bootdata)
		*(.data*)
		*(.got)
		*(.got.plt)
		. = ALIGN(4096);
	}
	PROVIDE(data_pa = LOADADDR(.data));
	PROVIDE(edata_pa = LOADADDR(.data) + SIZEOF(.data));


	.bss : ALIGN(4096) {
		*(.bss*)
		*(COMMON)
		. = ALIGN(4096);
	}

	/* Reserve section for early pagetables. */
	.earlyvm_pagetables : ALIGN(4096) {
		. += 32 * 4096;
	}
	PROVIDE(earlyvm_pagetables_pa = LOADADDR(.earlyvm_pagetables));
	PROVIDE(eearlyvm_pagetables_pa = LOADADDR(.earlyvm_pagetables) + SIZEOF(.earlyvm_pagetables));


/* TODO Should this be here???  Better to put into the bss section later */
	.end : ALIGN(2097152) {}

	PROVIDE(bss_pa = LOADADDR(.bss));
	PROVIDE(ebss_pa = LOADADDR(.end));


	/DISCARD/ : {
		*(.eh_frame .note.GNU-stack)
	}
}
