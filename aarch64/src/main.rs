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
// exec machinery: compiled into every build config.  The runtime call
// path runs under dev_exec (exec_bootstrap below); the module-level
// dead-code allowance stays because the non-exec configs compile the
// machinery without calling it, and some Proc fields (note_queue,
// notify_handler, wait_record, context) are carried for the scheduler
// and notes.  fault.rs carries no allowance: trap()'s EC split routes
// to it today.
#[allow(dead_code)]
mod exec;
mod fault;
#[allow(dead_code)]
mod proc;
mod regblock;
mod registers;
mod swtch;
mod syscall;
mod trap;
mod usermem;
mod vm;
mod vmdebug;

extern crate alloc;

use alloc::boxed::Box;
use param::KZERO;
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

    // SAFETY: trap.S reserves the interrupt stack and defines
    // interruptstacksz as an absolute (.equ) symbol — its "address" IS the
    // size value; only the zero-sized markers' addresses are read (the
    // link-sym class), never their contents.
    let interrupt_stack_base = unsafe { interruptstackbase.as_ptr().addr() };
    // SAFETY: as above — interruptstacksz's address IS the size value.
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
    // CURKSTACK (the trap.S entry-stack symbol) starts at the legacy
    // interrupt stack so pre-process traps behave exactly as before; the
    // first proc's activation repoints it.  Before any IRQ can unmask.
    proc::init_trap_stacks();
    // Raspi has no interrupt handlers to arm, so unmask at the point the
    // combined trap::init always unmasked.  Virt deliberately does NOT unmask
    // here — it calls unmask_irq() after GIC + timer + UART RX are armed.
    #[cfg(not(platform = "virt"))]
    trap::unmask_irq();

    // Virt ELF boot: qemu 11.1.2 passes x0 = 0 and loads no DTB for ELF
    // kernels.  init_vm (pre-MMU) already substituted
    // the fixed PA 0x40000000 and mapped the DTB at KZERO + 0x40000000; this
    // is the VA-side twin of that fallback.  Raspi delivers x0 via the boot
    // stub, so a null there is a genuine error, not this contract.
    #[cfg(platform = "virt")]
    let dtb_va = if dtb_va - KZERO == 0 { KZERO + 0x4000_0000 } else { dtb_va };

    // Record the DTB's VA for the D-17 random device's lazy rng-seed
    // parse (08-06 — the parse runs at the first /dev/random read, off
    // this recorded address; the DTB is reserved in pagealloc below and
    // its KZERO+PA mapping is never torn down).
    #[cfg(platform = "virt")]
    platform::devcons::set_dtb_va(dtb_va);

    // Parse the DTB before we set up memory so we can correctly map it.
    // The two-pass construction: header prefix learns totalsize, then the
    // full slice.
    // SAFETY: dtb_va names the DTB mapped at KZERO+PA by vminit (the
    // pre-MMU mapping is still live — main9 runs before any teardown);
    // valid and readable for the parse.  The header prefix sits inside the
    // first mapped page; the full totalsize range is inside the mapping.
    let dt = unsafe {
        let prefix = core::slice::from_raw_parts(dtb_va as *const u8, port::fdt::FDT_HEADER_SIZE);
        let total = port::fdt::DeviceTree::header_totalsize(prefix).unwrap();
        port::fdt::DeviceTree::new(core::slice::from_raw_parts(dtb_va as *const u8, total))
            .unwrap()
    };
    let dtb_physrange = PhysRange::with_pa_len(PhysAddr::new((dtb_va - KZERO) as u64), dt.size());

    // Initrd discovery: the host places the image in
    // guest RAM via qemu's generic loader at a fixed PA — qemu's -initrd
    // never delivers for ELF kernels (hw/arm/boot.c) — and xtask's patched
    // DTB carries linux,initrd-start/end (u64 BE cells) in /chosen.  Absent
    // /chosen or absent props is fine (config_virt.toml carries no initrd);
    // present-but-malformed is a host harness bug, so fail loudly.
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
                    // Reserve the initrd pages alongside
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

    println!();
    println!("r9 from the Internet");
    println!("DTB found at: {:#x}", dtb_va);
    println!("midr_el1: {:?}", registers::MidrEl1::read());

    // The console is live now: emit the probe marker + the paqfs
    // proof, AFTER the banner (banner-first emission order).  The mapped
    // image slice is kept for the dev_exec orchestration (devroot's
    // mount source).
    let initrd_img: Option<&'static [u8]> = match initrd {
        Some((start, end, size, physrange)) => {
            println!("initrd: start={start:#x} end={end:#x} size={size}");

            // Prove the delivery chain end-to-end with real
            // bytes.  Map the (page-rounded) initrd PA range into the kernel
            // table at KZERO+PA — the same VaMapping::Offset(KZERO)
            // convention vminit uses for the DTB — then parse the bytes as
            // paqfs and print one `paqfs: <name>` line per root entry.
            // pagealloc reserved the range above, so the image bytes
            // are intact; map_phys_range allocates only table pages.
            let mut physpage_allocator = vm::PhysPageAllocator {};
            let mut vmtrait_impl = vm::VmTraitImpl {};
            if let Err(err) = vm::kernel_pagetable().map_phys_range(
                &mut physpage_allocator,
                &mut vmtrait_impl,
                "initrd",
                &physrange,
                vm::VaMapping::Offset(KZERO),
                vm::Entry::rw_kernel_data(),
                vm::PageSize::Page4K,
                vm::RootPageTableType::Kernel,
            ) {
                panic!("initrd: map failed: {err:?}");
            }
            // SAFETY: the initrd's KZERO+PA range was just mapped r/o in
            // the kernel table above and reserved in pagealloc at boot
            // — the bytes are the live image for the kernel's
            // lifetime (devroot's mount source).
            let img = unsafe {
                core::slice::from_raw_parts((KZERO + start as usize) as *const u8, size as usize)
            };
            // With an initrd configured, an unreadable image is a
            // build/harness bug — fail loudly.
            match port::paqfs::Paqfs::open_image(img) {
                Ok(fs) => match fs.root_dir().and_then(|root| fs.read_dir(&root)) {
                    Ok(entries) => {
                        for entry in &entries {
                            println!(
                                "paqfs: {}",
                                core::str::from_utf8(&entry.name).unwrap_or("<bad-utf8>")
                            );
                        }
                    }
                    Err(err) => {
                        println!("paqfs: open failed: {err}");
                        panic!("paqfs: root walk failed: {err}");
                    }
                },
                Err(err) => {
                    println!("paqfs: open failed: {err}");
                    panic!("paqfs: {err}");
                }
            }
            Some(img)
        }
        None => {
            println!("initrd: none");
            None
        }
    };
    // Consumed only by the dev_exec exec orchestration below.
    #[cfg(not(dev_exec))]
    let _ = initrd_img;

    print_stacks();

    print_binary_sections();

    #[cfg(not(platform = "virt"))]
    print_board_info();
    // The virt "what machine am I on" slot: DTB-derived machine info
    // instead of the mailbox-driven board info.
    #[cfg(platform = "virt")]
    platform::print_machine_info(&dt);
    print_memory_info();

    // vmdebug::print_recursive_tables(RootPageTableType::Kernel);
    // vmdebug::print_recursive_tables(RootPageTableType::User);

    {
        let mut page_table = vm::kernel_pagetable();
        let entry = Entry::rw_kernel_data();
        for i in 0..3 {
            let alloc_result = pagealloc::allocate_virtpage(
                &mut page_table,
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

    vmdebug::print_recursive_tables(RootPageTableType::Kernel);

    let _b = Box::new("ddododo");

    println!("looping now");

    // Virt interrupt bring-up: arm the GIC, the generic timer
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
        // ITARGETSR cpu0 target.  Arm the UART's RXIM|RTIM path first.
        platform::pl011::init_rx();
        platform::gic::enable(33, 0xA0);

        // devether (dev_ether configs; D-10: the NIC is part of the
        // standard boot): bind the virtio-net NIC — pci probe → BAR/caps →
        // negotiate → queues → RX pre-post → publish → GIC arm → bind
        // print.  Inside the arming window (IRQs still masked) and ahead
        // of the dev_panic brk site, so the ring setup races nothing and
        // the panic profile's dump stays deterministic.  A missing NIC or
        // a rejected negotiation prints a loud named line and the boot
        // CONTINUES (kernel tolerant, gates strict).
        #[cfg(dev_ether)]
        platform::pci::bind_virtio_net(&dt);

        // Deliberate panic (dev_panic configs, e.g. --config virtpanic;
        // the hook itself is platform-agnostic): raise a sync exception
        // through the REAL trap path BEFORE IRQs unmask, so the panic log is
        // deterministic -- zero IRQ interleaving.  trap()'s sync branch dumps
        // the full TrapFrame, prints the "kernel halted" clean-halt marker,
        // and parks in wfi.
        #[cfg(dev_panic)]
        {
            println!("virtpanic: deliberate sync exception via brk #0 (D-12 debug config)");
            // The consolidated trap-interface primitive.
            trap::debug_sync_trap();
        }

        // Under dev_exec the boot continues into
        // the first-process exec orchestration and never returns — EL0 is
        // entered through trap_return with the seeded forkret-shaped frame
        // (swtch never enters EL0).  Without dev_exec the virt idle loop
        // stands: wfi wakes on each tick; trap()'s IRQ branch erets back
        // here.  It prints nothing, which is what makes tick()'s println
        // from IRQ context safe against the Console lock (see timer.rs).
        //
        // Under dev_exec the unmask moves to just before the first
        // EL0 entry (exec_bootstrap) — the exec orchestration is a
        // non-preemptible boot critical section (nothing in it needs
        // interrupts), and the tick's domain is EL0 userland.
        #[cfg(not(dev_exec))]
        trap::unmask_irq();
        #[cfg(dev_exec)]
        exec_bootstrap(initrd_img);
        #[cfg(not(dev_exec))]
        // The consolidated halt primitive (wfi wakes on each
        // tick; trap()'s IRQ branch erets back here).
        trap::park_forever()
    }

    #[cfg(not(platform = "virt"))]
    {
        #[allow(clippy::empty_loop)]
        loop {}
    }
}

mod runtime;

/// The cons echo sink — the cooked-mode input echo
/// writes byte-exact through the device-write path (write_bytes's
/// Console-lock precedent; devcons never calls it under the INPUT lock).
#[cfg(all(dev_exec, platform = "virt"))]
fn cons_echo(bytes: &[u8]) {
    port::devcons::write_bytes(bytes);
}

/// The pid-1 entry path.  dev_bootchain configs (e.g. the
/// virtshell profile) exec the real Harvey boot(8) at /boot/boot — a
/// two-component devroot walk; every pre-existing profile keeps /init
/// (the staged regression gates are untouched).
#[cfg(all(dev_exec, dev_bootchain))]
const BOOT_ENTRY: &[u8] = b"/boot/boot";
#[cfg(all(dev_exec, not(dev_bootchain)))]
const BOOT_ENTRY: &[u8] = b"/init";

/// The first-process exec
/// orchestration, entered after the virt IRQ arming + unmask block (the
/// GIC → timer → PL011 RX → unmask order is preserved above).  Requires the
/// initrd discovery to have mapped an image — dev_exec without an
/// initrd is a config bug and panics with a named message.  Mounts the
/// image as devroot, sources /init through it (walk → open OEXEC — one
/// mechanism), builds the first proc, execs, tears down the TTBR0 identity
/// map (complete-table-before-switch order), then erets into EL0 via
/// trap_return with the seeded forkret frame.  Never returns.
#[cfg(dev_exec)]
fn exec_bootstrap(initrd_img: Option<&'static [u8]>) -> ! {
    let img = initrd_img.unwrap_or_else(|| {
        panic!("dev_exec: config carries no initrd image (set [qemu] initrd to target/bootfs.paq)")
    });

    // The probed cache geometry the
    // fault handler's icache maintenance derives from (never hardcoded);
    // the consolidated register read (vm.rs's floor).
    let ctr = vm::ctr_el0();
    println!("ctr_el0: {ctr:#x}");

    // devproc-lite's note-post hook — the arch-side half of the
    // /proc device (port can't see PROC_TABLE; the same dependency
    // inversion as devcons's Console::set_uart).  Before the first exec so
    // a /proc/N/note write from the very first process resolves.
    port::devproc::set_post_hook(proc::post_note);
    // devproc's proc-table hooks — the ps(1) data source
    // (the /proc root's live-pid listing + the per-pid status/args) and
    // init's #p/N/ctl "pri 10" record — and devcons's #c/pid hook (libc
    // getpid's read, so the pri write names the REAL pid).  Same
    // dependency inversion; NOT virt-gated (the /proc + #c surface is
    // platform-independent — the boot chain runs on every dev_exec).
    port::devproc::set_proc_hooks(port::devproc::ProcHooks {
        list: proc::proc_list_live,
        info: proc::proc_info,
        set_pri: proc::proc_set_pri,
        set_private: proc::proc_set_private,
    });
    port::devcons::set_pid_hook(proc::current_pid);

    // devpipe's block/wake hooks — a read on an empty pipe
    // parks the caller via the sched machinery; a write or close
    // wakes the queue's waiters.  Registered before the first exec so
    // boot's connectembed (pipe → fork → exec paqfs, which blocks reading
    // its end) can block the very first pipe's readers.
    port::devpipe::set_hooks(port::devpipe::PipeHooks {
        current: proc::pipe_current_pid,
        block: proc::pipe_block_current,
        wake: proc::pipe_wake,
    });
    // devmnt's contention-wait hook (06-07): a concurrent RPC caller
    // parks ~1 tick and retries (the tier-1 sweep's pipelines are the
    // first measured concurrent mount-session callers) — the same
    // injected-hook idiom (port can't see PROC_TABLE).
    port::devmnt::set_contention_hook(proc::mnt_contention_wait);
    // devether's EtherHooks (07-07) — the same shape: a data-file read
    // on an empty conversation queue parks the caller; the RX ingress
    // (the virtio-net IRQ retire's port-side entry) wakes the waiters,
    // and a TX-full write blocks until the retire readies it.  Registered
    // before the first exec; the hooks are only ever needed once a
    // userspace caller blocks on #l.
    #[cfg(dev_ether)]
    port::devether::set_hooks(port::devether::EtherHooks {
        current: proc::pipe_current_pid,
        block: proc::pipe_block_current,
        wake: proc::pipe_wake,
    });
    // devip's DevipHooks + tick source (08-04, NETIP-02) — the same
    // {current, block, wake} shape: a conv's blocked reader/writer parks
    // via the sched machinery; the seam's waker bridge and the 1 ms
    // tick's net-deadline sweep (timer.rs's cfg(dev_ip) arm) wake them.
    // The clock is the arch's generic-timer tick count (1 tick = 1 ms —
    // the smoltcp Instant bridge).  Registered before the first exec.
    #[cfg(dev_ip)]
    {
        port::devip::stack::set_hooks(port::devip::stack::DevipHooks {
            current: proc::pipe_current_pid,
            block: proc::pipe_block_current,
            wake: proc::pipe_wake,
        });
        port::devip::stack::set_clock(crate::platform::timer::ticks);
    }
    // devdup's resolver/listing hooks and devsrv's fd resolver —
    // the `#d`/`#s` devices resolve the OPERATING proc's fd table through
    // these (port can't see PROC_TABLE — the devproc set_post_hook idiom).
    port::devdup::set_hooks(port::devdup::DupHooks {
        resolve: proc::current_fd_resolve,
        list: proc::current_fd_list,
    });
    port::devsrv::set_resolve_hook(proc::current_fd_resolve);

    // devcons's read-path hooks — the console's input
    // engine rides the arch's lock-free PL011 RX ring (drain_rx), the
    // sched machinery (block), and the note-group post (the DEL
    // path's foreground-group target).  virt-only this era (the PL011 RX
    // ring is a virt driver; dev_exec configs are all virt).
    // The echo hook — cooked-mode input echo rides
    // the device-write path (write_bytes, the syscall-context
    // Console-lock precedent); devcons calls it only AFTER the INPUT
    // lock drops.
    #[cfg(platform = "virt")]
    port::devcons::set_cons_hooks(port::devcons::ConsHooks {
        current: proc::current_pid,
        drain_rx: platform::pl011::cons_rx_drain,
        block: proc::cons_block_current,
        post_group: proc::post_note_group_of_members,
        echo: cons_echo,
    });
    // devcons's random-device hooks (D-17, 08-06): the DTB rng-seed read
    // (lazy — at the first /dev/random read, via port::fdt off the DTB VA
    // recorded in main9) + the generic-timer jitter sample.  virt-only
    // this era (the only dev_exec platform; the tick count is the virt
    // timer module's).  Registered before the first exec so the very
    // first ipconfig's truerand() resolves.
    #[cfg(platform = "virt")]
    {
        port::devcons::set_random_hooks(port::devcons::RandomHooks {
            rng_seed: platform::devcons::rng_seed_read,
            jitter: platform::devcons::random_jitter,
        });
    }

    // The kernel-seeded ROOT env group — the ONE mechanism
    // replacing both 9k's #ec config env and an argv hint.
    // cputype feeds boot's
    // execinit path build ("/aarch64/init -t"; without it boot builds
    // "//init" and dies); nobootprompt=embed selects the root-server
    // method WITHOUT the outin() prompt (rootserver reads #e/nobootprompt
    // first — a devenv failure here hangs at outin and the boot gate
    // TIMES OUT LOUDLY); readparts=0 skips the sd* partition scan
    // (boot.c partinit's getenv).  The hook + seed land BEFORE the first
    // proc exists; pid 1 gets the root group below, and rfork derives
    // every later proc's group from its parent.
    port::devenv::set_group_hook(proc::current_env_group);
    let root_env = port::devenv::seed_root_group(&[
        (b"cputype", b"aarch64"),
        (b"nobootprompt", b"embed"),
        (b"readparts", b"0"),
    ]);
    println!("devenv: seeded cputype=aarch64 nobootprompt=embed readparts=0");

    // The initrd paqfs is the root tree; exec sources the entry binary
    // through the devroot walk/open path (one mechanism).  The walk
    // generalizes to a PATH — devroot's walk is single-component, so loop it
    // over the entry path's components ("/boot/boot" walks boot→boot).
    let root =
        port::devroot::mount_initrd(img).unwrap_or_else(|err| panic!("devroot: mount: {err}"));
    let entry_path = BOOT_ENTRY;
    let entry_str = core::str::from_utf8(entry_path).unwrap_or("?");
    let slash_chan =
        root.attach_root().unwrap_or_else(|err| panic!("devroot: attach: {err}"));
    let mut entry_chan = slash_chan.dup();
    let mut leaf: &[u8] = entry_path;
    for comp in entry_path.split(|&b| b == b'/').filter(|c| !c.is_empty()) {
        leaf = comp;
        entry_chan = match root.walk(&entry_chan, comp) {
            Ok(c) => c,
            Err(err) => panic!("devroot: walk {entry_str}: {err}"),
        };
    }
    let entry_open = match root.open(&entry_chan, port::devroot::OEXEC) {
        Ok(c) => c,
        Err(err) => panic!("devroot: open {entry_str}: {err}"),
    };
    let bin = match root.entry(&entry_open) {
        Ok(bin) => bin,
        Err(err) => panic!("devroot: {entry_str} entry: {err}"),
    };
    println!("devroot: {entry_str} open ({} bytes)", bin.length);

    // Build the first process and exec the entry: a.out parse + segment
    // record + the eager data/bss pages + exactly
    // two eager stack-side pages (the seed stack and the zeroed Tos page) +
    // the forkret frame.  Text/stack/tos fault in.
    // argv is [boot] — the kernel-seeded env group above is
    // THE method-selection mechanism (boot's rootserver reads
    // #e/nobootprompt before any argv fallback).
    let name = core::str::from_utf8(leaf).unwrap_or("?");
    let argv: &[&[u8]] = &[leaf];
    let mut proc = proc::new_user_proc(name).unwrap_or_else(|err| panic!("proc: {err}"));
    // The first process's namespace root — slash and dot both
    // start at the devroot attach (the donor's slash/dot; rfork inherits
    // them, exec preserves them).  Every later proc gets its own Chan value
    // naming the same tree.
    proc.slash = Some(slash_chan.dup());
    proc.dot = Some(slash_chan);
    // pid 1 runs in the seeded ROOT env group (assigned
    // explicitly — the slash/dot seeding's twin; Proc::new's fresh empty
    // group is the never-seeded default).
    proc.env_group = root_env;
    exec::exec_into(&mut proc, img, &bin, name, argv)
        .unwrap_or_else(|err| panic!("exec: {entry_str}: {err}"));
    // SAFETY: proc.frame names the seeded forkret frame on the proc's
    // freshly-mapped kstack (exec_into's construction); read-only here.
    let frame = unsafe { &*proc.frame };
    let argv_print: alloc::vec::Vec<&str> =
        argv.iter().map(|a| core::str::from_utf8(a).unwrap_or("?")).collect();
    println!(
        "exec: entering EL0 pc={:#x} sp={:#x} path={entry_str} argv={argv_print:?}",
        frame.pc(),
        frame.sp()
    );

    // The proc's fully-built root table REPLACES
    // the TTBR0 identity map.  Order: the table is complete (its
    // recursive slot-511 self-entry was written at construction and the seed
    // pages are mapped) BEFORE the switch; the console print immediately
    // after proves all runtime MMIO is TTBR1-side — a dead console here is
    // an instant visible failure.
    // SAFETY: the proc's root table is fully built (the recursive
    // self-entry + the seed pages — the complete-before-switch
    // order) and owned
    // by the proc; this is the one-way TTBR0 switch into it.
    unsafe { vm::switch(&*proc.root_table, RootPageTableType::User) };
    println!("exec: ttbr0 switched, console alive");

    let frame_ptr = proc.frame;
    // Publish current + Running + CURKSTACK (the trap_return tail
    // loads SP_EL1 from CURKSTACK on the EL0t-bound return below).  IRQs
    // stay MASKED at EL1 through the tail: the seed frame's psr=0 unmasks
    // them at EL0 via the eret's SPSR restore, so the pending tick's first
    // delivery is a clean EL0-origin IRQ on the proc's kstack — never an
    // EL1h IRQ mid-tail, whose preempt() would see "Running" and stash a
    // transient boot-stack frame over proc.frame.
    proc::activate_first(&mut proc);
    // SAFETY: frame_ptr names the seeded forkret frame on the proc's
    // kstack (exec_into's construction); CURKSTACK/CURFRAME were just
    // published by activate_first — the single first EL0 entry.
    unsafe { trap::trap_return(frame_ptr) }
}
