/* S6 arm64 probe linker script: one flat text payload at r9's INITTEXT.
 * The 40-byte a.out header is prepended afterwards by aout.py; the
 * payload starts at the entry VA (0x10028), so the first instruction is
 * _start. Text and rodata share the single output section: the probe has
 * no data or bss, and the flat `--oformat=binary` output must be exactly
 * the bytes the a.out header declares as `text`. */
ENTRY(_start)
SECTIONS
{
    . = 0x10028;
    .text : {
        KEEP(*(.text._start))
        *(.text .text.*)
        *(.rodata .rodata.*)
    }
    /DISCARD/ : {
        *(.eh_frame*)
        *(.comment)
        *(.note*)
        *(.debug*)
        *(.llvm_addrsig)
    }
}
