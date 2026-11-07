#![feature(alloc_error_handler)]
#![feature(fn_align)]
#![feature(sync_unsafe_cell)]
#![cfg_attr(not(any(test)), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(clippy::upper_case_acronyms)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod allocator;
mod cpu;
mod dat;
mod pagealloc;
mod pio;
mod platform;
mod seg;
mod trap;
mod uart16550;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("l.S"), options(att_syntax));

use port::println;

use crate::platform::q35::mb2;

/// KZERO — the higher-half base (kernel.ld); the kernel's physical
/// address of any linked symbol is its VA minus this.
const KZERO: u64 = 0xffff_ffff_8000_0000;

/// The mb2 info stash's size — l.S's MB2INFOMAX must match this.
const MB2_STASH_SIZE: usize = 4096;

/// The mb2 info stash: l.S copies the GRUB-handed structure here in
/// 32-bit early boot, BEFORE it zeroes the second mebibyte of RAM for
/// the Mach — GRUB 2.14's heap can place the structure inside that
/// mebibyte (observed live at 0x16dc08 on this channel, 13-02), and the
/// zeroing would destroy the only copy of the firmware data the kernel
/// boots from.  The asm clamps the copy to the buffer size.
#[repr(C, align(8))]
pub struct Mb2InfoStash([u8; MB2_STASH_SIZE]);

#[unsafe(no_mangle)]
pub static mut mb2info: Mb2InfoStash = Mb2InfoStash([0; 4096]);

fn print_memory_info() {
    println!("Memory usage:");
    let (used, total) = pagealloc::usage_bytes();
    println!("  Used:\t\t{used:#016x}");
    println!("  Total:\t{total:#016x}");
}

/// The physical end of the kernel image (the `end` link symbol's VA
/// minus KZERO — the .bss end, which kernel.ld 2 MiB-aligns).
fn kernel_phys_end() -> u64 {
    unsafe extern "C" {
        static end: [u64; 0];
    }
    // SAFETY: kernel.ld PROVIDEs this zero-sized marker at exactly the
    // image end; only the symbol's ADDRESS is read (the kmem link-sym
    // class), never its contents.
    unsafe { end.as_ptr().addr() as u64 - KZERO }
}

#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn main(mach: &mut dat::Mach, mbdata: u64) {
    // Machine substrate: fill and load the Mach's GDT/IDT/TSS so an
    // early exception prints via trap() instead of triple-faulting
    // silently — the fill moved here from node0 (whose l.S-called
    // init0 built page tables and filled separate pages aliased at the
    // Mach's descriptor addresses; retired with the old generation).
    // Interrupts stay masked: the mb2 i386 entry contract guarantees
    // EFLAGS.IF=0 and l.S never raises it — sti arrives with the timer
    // in 13-03, after the handlers exist (the Phase-3 ordering rule).
    unsafe {
        mach.idt.init(trap::stubs());
        mach.gdt.init(&mut mach.tss);
        mach.init();
    }

    // Parse the stashed mb2 info — the two-pass fdt.rs discipline.
    // The Result is held (not unwrapped) so a parse failure prints on
    // the live console once devcons is up: loud by construction, never
    // a silent halt before the banner.
    //
    // SAFETY: mbdata names l.S's .bss stash (warp64 passes its linked
    // address, mapped by the boot PML4s the port-based system runs
    // on); the 8-byte prefix sits inside the 4 KiB buffer, the total is
    // bounds-checked against the buffer before the full slice is
    // formed, and the walk inside parse is fully checked — the
    // firmware pointer is untrusted input (T-13-03).
    let parsed = unsafe {
        let prefix = core::slice::from_raw_parts(mbdata as *const u8, mb2::MB2_PREFIX_SIZE);
        match mb2::header_total_size(prefix) {
            Ok(total) if total <= MB2_STASH_SIZE => {
                mb2::parse(core::slice::from_raw_parts(mbdata as *const u8, total))
            }
            // A structure larger than the stash: l.S clamps its copy to
            // the buffer, so the stash cannot hold it whole.
            Ok(_) => Err(mb2::ParseError::BufferTooSmall),
            Err(e) => Err(e),
        }
    };

    // Seed the page allocator from the type-1 entries (type-3 ACPI-
    // reclaimable ranges stay reserved — 13-04 walks tables there).
    // Held too: the error prints after the console is live.
    let alloc_err = match &parsed {
        Ok(facts) => pagealloc::init_page_allocator(facts, kernel_phys_end()).err(),
        Err(_) => None,
    };

    // The console: COM1 16550 into port::devcons (the tolerant probe
    // prints loudly if the uart is missing).
    platform::devcons::init();

    println!();
    println!("r9 from the Internet");

    // Loud-by-absence firmware-data errors, on the live console, after
    // the banner so the gate's marker always lands first.
    let facts = match (&parsed, alloc_err) {
        (Ok(facts), None) => facts,
        (Err(e), _) => {
            println!("q35: mb2 parse error: {e:?}");
            halt();
        }
        (Ok(_), Some(err)) => {
            println!("q35: page allocator init failed: {err:?}");
            halt();
        }
    };

    // The "what machine am I on" slot: one q35: line per mb2 fact.
    platform::print_machine_info(facts);
    print_memory_info();

    // Park cleanly with IRQs masked — the boot-test gate's negative
    // panic scan stays green.  The next bring-up step (13-03) arms the
    // LAPIC/timer before the first sti.
    halt();
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt") };
    }
}

mod runtime;
