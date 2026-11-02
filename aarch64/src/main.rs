#![allow(clippy::upper_case_acronyms)]
#![allow(internal_features)]
#![cfg_attr(not(any(test)), no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(alloc_error_handler)]
#![feature(core_intrinsics)]
#![feature(sync_unsafe_cell)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod allocator;
mod deviceutil;
mod io;
mod kmem;
mod pagealloc;
mod param;
mod platform;
mod pre_mmu;
mod registers;
mod swtch;
mod trap;
mod vm;
mod vmdebug;

extern crate alloc;

use alloc::boxed::Box;
#[cfg(not(platform = "virt"))]
use core::ptr::null_mut;
use param::KZERO;
use port::fdt::DeviceTree;
use port::mem::{PhysAddr, PhysRange, VirtRange};
use port::println;
use vm::{Entry, RootPageTableType, VaMapping};

use crate::kmem::{
    boottext_physrange, bss_physrange, data_physrange, rodata_physrange, text_physrange,
    total_kernel_physrange,
};
use crate::vm::PageSize;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("l.S"));

fn print_memory_range(name: &str, range: &PhysRange) {
    let size = range.size();
    println!("  {name}{range} ({size:#x})");
}

fn print_binary_sections() {
    println!("Binary sections:");
    print_memory_range("boottext:\t", &boottext_physrange());
    print_memory_range("text:\t\t", &text_physrange());
    print_memory_range("rodata:\t", &rodata_physrange());
    print_memory_range("data:\t\t", &data_physrange());
    print_memory_range("bss:\t\t", &bss_physrange());
    print_memory_range("total:\t", &total_kernel_physrange());
}

fn print_memory_info() {
    println!("Memory usage:");
    let (used, total) = pagealloc::usage_bytes();
    println!("  Used:\t\t{used:#016x}");
    println!("  Total:\t{total:#016x}");
}

// https://github.com/raspberrypi/documentation/blob/develop/documentation/asciidoc/computers/raspberry-pi/revision-codes.adoc
#[cfg(not(platform = "virt"))]
fn print_pi_name(board_revision: u32) {
    let name = match board_revision {
        0xa21041 => "Raspberry Pi 2B",
        0xa02082 => "Raspberry Pi 3B",
        0xb03115 => "Raspberry Pi 4B",
        0xa220a0 => "Raspberry Compute Module 3",
        _ => "Unrecognised",
    };
    println!("  Board Name:\t{name}");
}

#[cfg(not(platform = "virt"))]
fn print_board_info() {
    println!("Board information:");
    let board_revision = platform::mailbox::get_board_revision();
    print_pi_name(board_revision);
    println!("  Board Rev:\t{board_revision:#010x}");
    let model = platform::mailbox::get_board_model();
    println!("  Board Model:\t{model:#010x}");
    let serial = platform::mailbox::get_board_serial();
    println!("  Serial Num:\t{serial:#010x}");
    let platform::mailbox::MacAddress { a, b, c, d, e, f } = platform::mailbox::get_board_macaddr();
    println!("  MAC Address:\t{a:02x}:{b:02x}:{c:02x}:{d:02x}:{e:02x}:{f:02x}");
    let fw_revision = platform::mailbox::get_firmware_revision();
    println!("  Firmware Rev:\t{fw_revision:#010x}");
}

fn print_stacks() {
    unsafe extern "C" {
        static interruptstackbase: [u64; 0];
        static interruptstacksz: [u64; 0];
    }

    let interrupt_stack_base = unsafe { interruptstackbase.as_ptr().addr() };
    let interrupt_stack_max = interrupt_stack_base + unsafe { interruptstacksz.as_ptr().addr() };
    let range = VirtRange(interrupt_stack_base..interrupt_stack_max);
    let range_size = range.size();
    println!("Interrupt stack:\t {range} ({range_size:#x})");
}

/// dtb_va is the virtual address of the DTB structure.  The physical address is
/// assumed to be dtb_va-KZERO.
#[unsafe(no_mangle)]
pub extern "C" fn main9(dtb_va: usize) {
    trap::init();
    // Raspi has no interrupt handlers to arm, so unmask at the point the
    // combined trap::init always unmasked.  Virt deliberately does NOT unmask
    // here — 03-05 calls unmask_irq() after GIC + timer + UART RX are armed.
    #[cfg(not(platform = "virt"))]
    trap::unmask_irq();

    // Virt ELF boot: qemu 11.1.2 passes x0 = 0 and loads no DTB for ELF
    // kernels (phase-03 research P1).  init_vm (pre-MMU) already substituted
    // the fixed PA 0x40000000 and mapped the DTB at KZERO + 0x40000000; this
    // is the VA-side twin of that fallback.  Raspi delivers x0 via the boot
    // stub, so a null there is a genuine error, not this contract.
    #[cfg(platform = "virt")]
    let dtb_va = if dtb_va - KZERO == 0 { KZERO + 0x4000_0000 } else { dtb_va };

    // Parse the DTB before we set up memory so we can correctly map it
    let dt = unsafe { DeviceTree::from_usize(dtb_va).unwrap() };
    let dtb_physrange = PhysRange::with_pa_len(PhysAddr::new((dtb_va - KZERO) as u64), dt.size());

    // D-01 initrd discovery (amended 04-01): the host places the image in
    // guest RAM via qemu's generic loader at a fixed PA — qemu's -initrd
    // never delivers for ELF kernels (hw/arm/boot.c) — and xtask's patched
    // DTB carries linux,initrd-start/end (u64 BE cells) in /chosen.  Absent
    // /chosen or absent props is fine (config_virt.toml carries no initrd);
    // present-but-malformed is a host harness bug, so fail loudly (T-04-02).
    // The console UART is not registered until devcons::init below, so the
    // marker print is deferred to after that; the invalid-range panic path
    // uses the early-uart putstr so it can never be swallowed.
    let initrd = match dt.find_by_path("/chosen") {
        Some(chosen) => {
            let start = dt
                .property(&chosen, "linux,initrd-start")
                .and_then(|p| dt.property_value_as_u64(&p));
            let end = dt
                .property(&chosen, "linux,initrd-end")
                .and_then(|p| dt.property_value_as_u64(&p));
            match (start, end) {
                (Some(start), Some(end)) => {
                    // Validate against the DTB memory node before any use —
                    // the same first-memory-node parse pagealloc consumes
                    // below.  The initrd must sit inside RAM.
                    let ram_top = dt
                        .find_device_type("memory")
                        .flat_map(|m| {
                            dt.property_translated_reg_iter(m).flat_map(|r| r.regblock())
                        })
                        .map(|r| PhysRange::from(&r))
                        .next()
                        .map(|r| r.end().addr())
                        .unwrap_or(0);
                    if start < 0x4000_0000 || start >= end || end > ram_top {
                        pre_mmu::util::putstr("initrd: invalid range start=");
                        pre_mmu::util::putu64h(start);
                        pre_mmu::util::putstr(" end=");
                        pre_mmu::util::putu64h(end);
                        pre_mmu::util::putstr("\n");
                        panic!("initrd: invalid range start={start:#x} end={end:#x}");
                    }
                    let size = end - start;
                    // Pitfall 9 (T-04-01): reserve the initrd pages alongside
                    // dtb_physrange so pagealloc never hands the image bytes
                    // out later.
                    let physrange =
                        PhysRange::with_pa_len(PhysAddr::new(start), size as usize)
                            .round(PageSize::Page4K.size());
                    Some((start, end, size, physrange))
                }
                _ => None,
            }
        }
        None => None,
    };
    let initrd_physrange = initrd.as_ref().map(|(_, _, _, r)| r.clone());

    // Set up page allocator
    let mut physranges = [
        Some(dtb_physrange.round(PageSize::Page4K.size())),
        Some(boottext_physrange().add(&text_physrange())),
        Some(rodata_physrange()),
        Some(data_physrange().add(&bss_physrange())),
        initrd_physrange,
    ];
    physranges.sort_by_key(|a| a.as_ref().map(|r| r.start()));
    if let Err(err) = pagealloc::init_page_allocator(&dt, physranges.into_iter().flatten()) {
        panic!("error:Couldn't mark unused pages as free: err: {:?}", err);
    }

    platform::devcons::init(&dt);
    #[cfg(not(platform = "virt"))]
    platform::mailbox::init(&dt);

    // The console is live now: emit the D-01 probe marker.
    match initrd {
        Some((start, end, size, _)) => {
            println!("initrd: start={start:#x} end={end:#x} size={size}");
        }
        None => println!("initrd: none"),
    }

    println!();
    println!("r9 from the Internet");
    println!("DTB found at: {:#x}", dtb_va);
    println!("midr_el1: {:?}", registers::MidrEl1::read());

    print_stacks();

    print_binary_sections();

    #[cfg(not(platform = "virt"))]
    print_board_info();
    // The virt "what machine am I on" slot (D-03): DTB-derived machine info
    // instead of the mailbox-driven board info.
    #[cfg(platform = "virt")]
    platform::print_machine_info(&dt);
    print_memory_info();

    // vmdebug::print_recursive_tables(RootPageTableType::Kernel);
    // vmdebug::print_recursive_tables(RootPageTableType::User);

    {
        let page_table = vm::kernel_pagetable();
        let entry = Entry::rw_kernel_data();
        for i in 0..3 {
            let alloc_result = pagealloc::allocate_virtpage(
                page_table,
                "testkernel",
                entry,
                VaMapping::Offset(KZERO),
                RootPageTableType::Kernel,
            );
            match alloc_result {
                Ok(_allocated_page) => {}
                Err(err) => {
                    println!("Error allocating page in kernel space ({i}): {:?}", err);
                    break;
                }
            }
        }
    }

    // vmdebug::print_recursive_tables(RootPageTableType::Kernel);
    // vmdebug::print_recursive_tables(RootPageTableType::User);

    // User page table experiment — raspi-only; the virt userspace-exec path is
    // Phase 4 scope (research OQ-3).
    #[cfg(not(platform = "virt"))]
    {
        println!("Set up a user process");

        unsafe {
            vm::init_user_page_tables();
            vm::switch(vm::user_pagetable(), RootPageTableType::User);
        }
    }

    // test_sysexit is raspi-only for the same reason — Phase 4 revisits it.
    #[cfg(not(platform = "virt"))]
    test_sysexit();

    vmdebug::print_recursive_tables(RootPageTableType::Kernel);
    // The user page table exists only on raspi this phase (init_user_page_tables
    // is gated with the user-page-table experiment above); walking it on virt
    // dereferences an unmapped recursive root.  Phase 4 revisits.
    #[cfg(not(platform = "virt"))]
    vmdebug::print_recursive_tables(RootPageTableType::User);

    let _b = Box::new("ddododo");

    println!("looping now");

    // Virt interrupt bring-up (P5 ordering): arm the GIC, the generic timer
    // and the UART RX path, and only then unmask IRQs.  Any earlier unmask
    // would hit trap()'s sync-path wedge with no handler armed.
    #[cfg(platform = "virt")]
    {
        platform::gic::init(&dt);
        // CNTP generic timer: PPI 14 -> INTID 16+14 (SPIs need ITARGETSR,
        // PPIs don't -- enable() applies that rule itself).
        platform::gic::enable(30, 0xA0);
        platform::timer::init(&dt);
        // PL011 RX: SPI 1 -> INTID 32+1 -- an SPI, so enable() writes its
        // ITARGETSR cpu0 target (P6).  Arm the UART's RXIM|RTIM path first.
        platform::pl011::init_rx();
        platform::gic::enable(33, 0xA0);

        // D-12 deliberate panic (dev_panic configs, e.g. --config virtpanic;
        // the hook itself is platform-agnostic): raise a sync exception
        // through the REAL trap path BEFORE IRQs unmask, so the panic log is
        // deterministic -- zero IRQ interleaving.  trap()'s sync branch dumps
        // the full TrapFrame, prints the "kernel halted" clean-halt marker,
        // and parks in wfi (SC#5 -- the Phase 4 debug path).
        #[cfg(dev_panic)]
        {
            println!("virtpanic: deliberate sync exception via brk #0 (D-12 debug config)");
            unsafe { core::arch::asm!("brk #0") };
        }

        trap::unmask_irq();

        // Idle loop: wfi wakes on each tick; trap()'s IRQ branch erets back
        // here.  Prints nothing, which is what makes tick()'s println from
        // IRQ context safe against the Console lock (see timer.rs).
        loop {
            unsafe { core::arch::asm!("wfi") };
        }
    }

    #[cfg(not(platform = "virt"))]
    {
        #[allow(clippy::empty_loop)]
        loop {}
    }
}

mod runtime;

#[cfg(not(platform = "virt"))]
fn test_sysexit() {
    let page_table = vm::user_pagetable();

    // Allocate pages for a user process
    let user_text = {
        let user_text = pagealloc::allocate_virtpage(
            page_table,
            "usertext",
            Entry::rw_user_text(),
            VaMapping::Addr(0x1000),
            RootPageTableType::User,
        )
        .expect("couldn't allocate user_text");

        // Machine code and assembly to call syscall exit
        //   00 00 80 D2    ; mov x0, #0
        //   21 00 80 D2    ; mov x1, #1
        //   61 00 00 D4    ; svc #3
        let proc_text_bytes: [u8; 12] =
            [0x00, 0x00, 0x80, 0xd2, 0x21, 0x00, 0x80, 0xd2, 0x61, 0x00, 0x00, 0xd4];
        user_text.0[..proc_text_bytes.len()].copy_from_slice(&proc_text_bytes);
        user_text
    };
    let user_text_va = user_text as *const _ as u64;

    let user_stack = pagealloc::allocate_virtpage(
        page_table,
        "userstack",
        Entry::rw_user_data(),
        VaMapping::Addr(KZERO - 0x1000),
        RootPageTableType::User,
    )
    .expect("couldn't allocate user_stack");

    // Executing user process!
    println!("Executing user process");
    let proc_stack = unsafe { core::slice::from_raw_parts_mut(user_stack, 4096) };

    // Initialise a Context struct on the process stack, at the end of the proc_stack_buffer.
    let ps_addr = &proc_stack as *const _ as u64;
    let proc_stack_initial_ctx = ps_addr + (4096 - size_of::<swtch::Context>()) as u64;
    let proc_context_ptr: *mut swtch::Context = proc_stack_initial_ctx as *mut swtch::Context;

    // Need to push a context object onto the stack, with x30 populated at the
    // address of proc_textbuf
    let proc_context_ref: &mut swtch::Context = unsafe { &mut *proc_context_ptr };
    proc_context_ref.set_stack_pointer(&proc_context_ptr as *const _ as u64);
    proc_context_ref.set_return(user_text_va);

    let mut kernel_context: *mut swtch::Context = null_mut();
    let kernel_context_ptr: *mut *mut swtch::Context = &mut kernel_context;

    //println!("proc ctx: {:#?}", proc_context_ref);

    unsafe { swtch::swtch(kernel_context_ptr, &*proc_context_ptr) };

    //println!("x30: {:#016x}", proc_context_ref.x30);
}
