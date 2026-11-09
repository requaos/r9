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

// exec substrate (14-03): usermem is the validated user-access surface
// (the ONLY path a user VA may cross into kernel code through), exec the
// amd64 a.out loader (parse/validate, segment math, the Ureg seed).
// Process machinery (14-04): proc is the per-process CR3/TSS-rsp0
// substrate (Proc, the round-robin scheduler, rfork/await, notes) and
// swtch its kernel-context switch.  Compiled into every build config
// but call-site-free until the syscall wiring (14-04 Task 2/3) and the
// exec bootstrap (14-05) — the module-level dead-code allowance mirrors
// aarch64's `mod exec`/`mod proc` precedent (non-exec configs compile
// the machinery without calling it).
#[allow(dead_code)]
mod exec;
#[allow(dead_code)]
mod proc;
#[allow(dead_code)]
mod usermem;
#[allow(dead_code)]
mod swtch;
#[allow(dead_code)]
mod syscall;

extern crate alloc;

#[cfg(not(test))]
core::arch::global_asm!(include_str!("l.S"), options(att_syntax));

use port::println;

use crate::platform::q35::mb2;

/// KZERO — the higher-half base (kernel.ld); the kernel's physical
/// address of any linked symbol is its VA minus this.  Homed in proc
/// (14-04 — the process machinery's kernel-half alias constant); one
/// definition, both consumers.
use proc::KZERO;

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

/// The boot-module stash's size — l.S's MODSTASHMAX must match this.
/// Headroom over the composed bootfs images this era (the 14-05 leg-1
/// bake is ~36 KB; Phase-15 tier payloads grow it).  The stash is .bss
/// (NOBITS): no file weight, the loader zeroes it, and the `end` symbol
/// covers it so the page allocator never reuses it.
const MOD_STASH_SIZE: usize = 8 * 1024 * 1024;

/// The boot-module stash (D-03, the 13-02 stash discipline extended to
/// the payload): l.S walks the stashed mb2 info's tags in 32-bit early
/// boot and copies the FIRST type-3 module's bytes here BEFORE the
/// second-mebibyte zeroing — GRUB's heap places modules in low memory,
/// and the first live one landed at 0x141000, INSIDE the Mach's
/// mebibyte (the zeroing would destroy the only copy of the bootfs).
/// `modstash_len` carries the copied byte count; 0 = no module.
#[repr(C, align(8))]
pub struct ModStash([u8; MOD_STASH_SIZE]);

#[unsafe(no_mangle)]
pub static mut modstash: ModStash = ModStash([0; 8 * 1024 * 1024]);

#[unsafe(no_mangle)]
pub static mut modstash_len: u32 = 0;

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
        // Arm the SYSCALL/SYSRET entry contract beside the descriptor
        // load — the 9k donor's vsvminit order (descriptors, then the
        // MSRs; vsvm.c:170-185).
        arm_syscall_entry(&mach);
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

    // The IRQ-arming window (the Phase-3 ordering hazard applies
    // verbatim on x86: no interrupt may be deliverable before its
    // handler exists).  The IDT — stubs, gates, IST stacks — plus the
    // GDT and TSS loaded at entry, so what remains here is arming the
    // sources in order and unmasking LAST; the mb2 i386 entry contract
    // guarantees EFLAGS.IF=0 at entry and nothing above raised it.
    //
    // LAPIC first: SVR software-enable + the D-04 LINT mask policy —
    // every later arming step programs LVT entries through it.
    platform::q35::lapic::init();
    // The timer: CPUID-gated mode, runtime-resolved frequency, LVT
    // programming and the first arm.  Its tick sweep carries the
    // console-RX echo, so serial input goes live the moment ticks do.
    platform::q35::timer::init();

    // The IOAPIC: MADT-driven (base + GSI base + the IntSrcOverride
    // fold — D-06, zero hardcoded GSIs), every RTE masked for a known
    // state.  It initializes AHEAD of the device bind: the bind's
    // ioapic::enable is the only unmask path, and its RTE write must
    // land inside this window, before sti.
    platform::q35::ioapic::init();

    // The NIC bind (D-01a's bound step, cfg(dev_ether) — the aarch64
    // main9 window shape): the whole virtio-net bring-up — BAR
    // sizing/assignment, the modern-only negotiation, queue bring-up,
    // RX pre-post, DRIVER_OK, devether's publish — runs here with IRQs
    // still masked, and its ioapic::enable RTE write (the unmask) lands
    // inside this window too, so no interrupt can precede the published
    // state or its handler.
    #[cfg(dev_ether)]
    platform::pci::bind_virtio_net();

    // Unmask LAST (the Phase-3 hazard, x86 edition): sti happens here,
    // once, after every handler the armed sources can fire exists in
    // the dispatch.  Any earlier unmask would hit an empty dispatch
    // table — a wedge, not a print.  EXCEPT under dev_exec: the exec
    // bootstrap is a non-preemptible boot critical section (the
    // aarch64 inflate-heisenbug discipline, 14-RESEARCH Pattern 4) —
    // IF stays 0 through the whole bootstrap and the seeded frame's If
    // flag unmasks interrupts AT the ring-3 handoff (the pending tick
    // then delivers as a clean ring-3-origin IRQ on the proc's kstack,
    // never an EL1h-style window mid-bootstrap).
    #[cfg(not(dev_exec))]
    cpu::sti();

    // The kernel-side ARP trigger (13-05 Task 3, X64-04 — the
    // Open-Question-1 resolution: kernel-side ONLY this phase; the
    // userspace dd-through-the-file-contract proof is Phase 14's XEXEC
    // seam): posts the baked 60-byte frame, ping-pong xN + TX-bulk,
    // reconciles the ifstats counters fail-closed, and prints the
    // X64-NET-DONE marker the q35net gate asserts.  The seam stays
    // runnable under its own cfg (13-05's regression shape); the
    // exec profiles select exec instead.
    #[cfg(all(dev_ether, not(dev_exec)))]
    platform::q35::netgate::run();

    // Under dev_exec the boot continues into the first-process exec
    // orchestration and never returns — ring 3 is entered through
    // trap_return with the seeded forkret-shaped 184-byte Ureg (swtch
    // never enters ring 3).  Without dev_exec the q35 idle loop stands:
    // hlt wakes on each tick; trap()'s IRQ branch iretq's back here.  It
    // prints nothing, which is what makes tick()'s println from IRQ
    // context safe against the Console lock (see timer.rs).
    #[cfg(dev_exec)]
    exec_bootstrap(facts);

    // The idle loop (kernel-only boots).
    #[cfg(not(dev_exec))]
    loop {
        // SAFETY: hlt faults nowhere and parks until an interrupt.
        unsafe { core::arch::asm!("hlt") };
    }
}

/// Arm the SYSCALL/SYSRET entry contract at kernel init — the 9k
/// donor's vsvminit sequence (vsvm.c:174-185): the GS-base pair,
/// Efer.Sce, Star, Lstar, Sfmask.  The Lstar write is last-in-plan by
/// construction: the entry asm (trap.rs's `syscallentry`) and this
/// arming landed together so Lstar was never armed at a hole.
///
/// Provably inert while the kernel is kernel-only: SYSCALL executes
/// only from ring 3, and no user image exists until the exec bootstrap
/// (14-05) — which is what every boot-test gate re-proves with the
/// arming live.  l.S already sets Efer.Sce for long-mode bring-up (with
/// Star/Lstar/Sfmask at 0); this adds the selector/flag routing and
/// re-affirms Sce read-modify-write style.
///
/// # Safety
/// Runs once at init on the boot CPU with interrupts masked; the MSR
/// values must match the loaded GDT's selector order (they do — both
/// sides derive from `seg::GdtData`).
unsafe fn arm_syscall_entry(mach: &dat::Mach) {
    unsafe {
        // FSbase = 0, GSbase = the Mach pointer, KernelGSbase = 0 (the
        // user GS base is 0 — user code does not use GS).  l.S's
        // warp64 already set GSbase; the write is idempotent.  The
        // explicit KernelGSbase = 0 is what makes the return-path
        // SWAPGS land the user on base 0 (the donor's scheme).
        cpu::wrmsr(cpu::MSR_FS_BASE, 0);
        cpu::wrmsr(cpu::MSR_GS_BASE, mach as *const dat::Mach as u64);
        cpu::wrmsr(cpu::MSR_KERNEL_GS_BASE, 0);

        // Efer |= Sce — SYSCALL/SYSRET enabled.  Read-modify-write: the
        // MSR carries LME/NXE from l.S and must keep them.
        let efer = cpu::_rdmsr(cpu::MSR_EFER);
        cpu::wrmsr(cpu::MSR_EFER, efer | cpu::EFER_SCE);

        // Star: SYSRET loads CS from STAR[63:48]+16 and SS from
        // STAR[63:48]+8; SYSCALL's own CS/SS come from STAR[47:32](+8).
        // Gdt::star()'s udata@0x18/utext@0x20 arithmetic makes both
        // SYSRET loads land on the flat user selectors — the revived
        // dead code, already SYSRET-correct by GDT construction.
        cpu::wrmsr(cpu::MSR_STAR, dat::Gdt::star());

        // Sfmask = If only — the donor's value (cpu::fmask's re-key).
        cpu::wrmsr(cpu::MSR_SFMASK, cpu::fmask());

        // Lstar — the entry target, at the real entry asm (trap.rs's
        // `syscallentry`; the missing-symbol link error is the presence
        // guard).  Sequenced after the entry exists — never armed at a
        // hole — and it beats l.S's default of 0.
        cpu::wrmsr(cpu::MSR_LSTAR, trap::syscallentry as *const () as u64);
    }
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt") };
    }
}

/// The pid-1 entry path (the aarch64 BOOT_ENTRY selection, re-keyed to
/// the x86_64 image layout): dev_bootchain configs (14-06's q35shell)
/// exec the real Harvey boot(8) at /boot/boot — a multi-component
/// devroot walk; every pre-existing exec profile keeps the staged
/// marker at /amd64/init (D-08's leg 1: the first exec is a
/// purpose-built static marker binary, D-09).
#[cfg(all(dev_exec, dev_bootchain))]
const BOOT_ENTRY: &[u8] = b"/boot/boot";
#[cfg(all(dev_exec, not(dev_bootchain)))]
const BOOT_ENTRY: &[u8] = b"/amd64/init";

/// The first-process exec orchestration (the aarch64 exec_bootstrap
/// port, 14-05): entered after the q35 IRQ-arming window with IF=0 —
/// the whole bootstrap is a non-preemptible boot critical section (the
/// inflate-heisenbug discipline; interrupts unmask only at the ring-3
/// handoff via the seeded frame's If flag).  Finds the mb2 module
/// (D-03's bootfs delivery — the range was bounds-checked against the
/// memory map at parse time), reads it through the KZERO alias of all
/// RAM (no new mapping: the boot PML4's slot-511 entry maps every
/// guest page, the aarch64 initrd discovery/map step collapsed into
/// the alias), mounts it as devroot, walks+opens BOOT_ENTRY, builds
/// the first proc, execs, switches to the proc's CR3, and hands off to
/// ring 3 through trap_return.  Never returns.
#[cfg(dev_exec)]
fn exec_bootstrap(facts: &mb2::Facts) -> ! {
    // D-03: the bootfs module.  GRUB loaded bootfs.paq as mb2 tag 3;
    // the ISO builder staged it because this config's [dist] module
    // key named it — its absence is a config/staging bug, loud here.
    // The BYTES come from the l.S stash (modstash): GRUB's heap placed
    // this boot's module at 0x141000 — INSIDE the Mach mebibyte the
    // early boot zeroes — so l.S copied them to .bss before the
    // zeroing (the 13-02 info-stash discipline, extended to the
    // payload).  The facts-side range reconciles against the stash:
    // a clamp (module larger than MODSTASHMAX) or a length
    // disagreement is a loud named panic, never a truncated image.
    let module = facts.modules().next().unwrap_or_else(|| {
        panic!("dev_exec: no mb2 module tag found — stage bootfs.paq into the ISO (the config's [dist] module key; bake it with Harvey's build/x86_64/mkimage.sh)")
    });
    let module_len = (module.end - module.start) as usize;
    // SAFETY: l.S published both words before the mebibyte zeroing
    // (single writer, pre-kernel); volatile reads because the asm
    // wrote them outside Rust's memory model.
    let stash_len = unsafe { core::ptr::read_volatile(&raw const modstash_len) } as usize;
    if stash_len == 0 {
        panic!("dev_exec: l.S stashed no module bytes (tag walk found none — loader/config bug)");
    }
    if stash_len != module_len {
        panic!(
            "dev_exec: module stash length {stash_len:#x} != tag length {module_len:#x} \
             (MODSTASHMAX clamp — grow the stash)"
        );
    }
    println!("q35: module @{:#x} +{:#x} (mb2 tag 3, stashed)", module.start, stash_len);
    // SAFETY: modstash is a .bss static linked into the kernel image —
    // mapped for the kernel's lifetime (and its pages sit below
    // kernel_phys_end, so the allocator never reuses them); the length
    // was reconciled above; the slice lives for the kernel's lifetime
    // (the module IS the root filesystem's backing).
    let img: &'static [u8] = unsafe {
        core::slice::from_raw_parts((&raw const modstash).cast::<u8>(), stash_len)
    };

    // The port-device hooks (the aarch64 registration block re-keyed —
    // port/ is arch-neutral and expects these before the first exec).
    // devproc-lite's note-post hook — the arch-side half of the /proc
    // device (port can't see PROC_TABLE; the devcons set_uart
    // dependency-inversion shape).
    port::devproc::set_post_hook(proc::post_note);
    // devproc's proc-table hooks (ps(1)'s data source + init's
    // #p/N/ctl "pri" record) and devcons's #c/pid hook (getpid).
    port::devproc::set_proc_hooks(port::devproc::ProcHooks {
        list: proc::proc_list_live,
        info: proc::proc_info,
        set_pri: proc::proc_set_pri,
    });
    port::devcons::set_pid_hook(proc::current_pid);
    // devpipe's block/wake hooks — a read on an empty pipe parks the
    // caller via the sched machinery (registered before the first exec
    // so the very first blocked reader resolves).
    port::devpipe::set_hooks(port::devpipe::PipeHooks {
        current: proc::pipe_current_pid,
        block: proc::pipe_block_current,
        wake: proc::pipe_wake,
    });
    // devmnt's contention-wait hook (the same injected-hook idiom).
    port::devmnt::set_contention_hook(proc::mnt_contention_wait);
    // devether/devip hooks under their own cfgs (the q35exec profile
    // selects neither; q35shell-era profiles will).
    #[cfg(dev_ether)]
    port::devether::set_hooks(port::devether::EtherHooks {
        current: proc::pipe_current_pid,
        block: proc::pipe_block_current,
        wake: proc::pipe_wake,
    });
    #[cfg(dev_ip)]
    {
        port::devip::stack::set_hooks(port::devip::stack::DevipHooks {
            current: proc::pipe_current_pid,
            block: proc::pipe_block_current,
            wake: proc::pipe_wake,
        });
        port::devip::stack::set_clock(platform::timer::ticks);
    }
    // devdup's resolver/listing hooks and devsrv's fd resolver — the
    // #d/#s devices resolve the OPERATING proc's fd table through
    // these.
    port::devdup::set_hooks(port::devdup::DupHooks {
        resolve: proc::current_fd_resolve,
        list: proc::current_fd_list,
    });
    port::devsrv::set_resolve_hook(proc::current_fd_resolve);
    // NOTE vs aarch64: no devcons ConsHooks/RandomHooks registration —
    // the x86_64 console RX rides the tick's polled 16550 sweep (there
    // is no PL011-ring analog to drain this era) and there is no DTB
    // rng-seed source; the marker binary writes only (pwrite on fd 1,
    // no hook needed by the write path).

    // The kernel-seeded ROOT env group — the ONE mechanism replacing
    // both 9k's #ec config env and an argv hint (the aarch64 seeding
    // re-keyed: cputype=amd64 feeds boot's execinit path build when
    // the boot chain arrives at 14-06).
    port::devenv::set_group_hook(proc::current_env_group);
    let root_env = port::devenv::seed_root_group(&[
        (b"cputype", b"amd64"),
        (b"nobootprompt", b"embed"),
        (b"readparts", b"0"),
    ]);
    println!("devenv: seeded cputype=amd64 nobootprompt=embed readparts=0");

    // The module paqfs is the root tree (D-04: the format carried
    // verbatim — port::devroot::mount_initrd + the miniz-inflate
    // reader, the collision-refusal discipline included).  exec
    // sources the entry binary through the devroot walk/open path (one
    // mechanism; the walk loops over the entry path's components —
    // devroot's walk is single-component).
    let root =
        port::devroot::mount_initrd(img).unwrap_or_else(|err| panic!("devroot: mount: {err}"));
    println!("q35: paqfs root mounted from module ({} bytes)", img.len());
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

    // Build the first process and exec the entry: a.out parse + the
    // eager text/data/bss pages + the two seed pages + the
    // forkret-shaped Ureg at kstack_top-184 (14-03/14-04's substrate).
    let name = core::str::from_utf8(leaf).unwrap_or("?");
    let argv: &[&[u8]] = &[leaf];
    let mut proc = proc::new_user_proc(name).unwrap_or_else(|err| panic!("proc: {err}"));
    // The first process's namespace root — slash and dot both start at
    // the devroot attach (rfork inherits them, exec preserves them).
    proc.slash = Some(slash_chan.dup());
    proc.dot = Some(slash_chan);
    // pid 1 runs in the seeded ROOT env group (the slash/dot seeding's
    // twin — Proc::new's fresh empty group is the never-seeded
    // default).
    proc.env_group = root_env;
    exec::exec_into(&mut proc, img, &bin, name, argv)
        .unwrap_or_else(|err| panic!("exec: {entry_str}: {err}"));

    // The one-way CR3 switch into the proc's table.  Order discipline
    // (reschedule's locked-step rule): the frame pointer is only ever
    // dereferenced UNDER the new table — the kstack window VA maps
    // solely in the proc's PML4 — so the switch lands before any frame
    // read; the kernel half (510/511) is shared, so every print, the
    // console, and all runtime MMIO stay alive (the aarch64
    // complete-table-before-switch + console-alive proof, x86 edition).
    proc::switch_cr3(proc.pml4_pa);
    // SAFETY: proc.frame names the seeded forkret frame on the proc's
    // kstack (exec_into's construction), now mapped under the live
    // CR3; read-only here.
    let frame = unsafe { &*proc.frame };
    println!(
        "exec: entering ring 3 pc={:#x} sp={:#x} path={entry_str} argv=[{name}]",
        frame.pc,
        frame.sp
    );

    // Publish current + Running + rsp0 (+ CURFRAME — the live-frame
    // record the scheduler's preempt path stashes from) and hand off:
    // trap_return restores the seeded frame and iretq's into ring 3 —
    // the frame's If flag unmasks interrupts at the handoff, the
    // pending tick delivering as a clean ring-3-origin IRQ on this
    // proc's kstack.  IRQs never unmasked in kernel context on the way
    // in (the whole bootstrap ran IF=0).
    proc::activate_first(&mut proc);
    let frame_ptr = proc.frame;
    // SAFETY: frame_ptr names the seeded forkret frame on the proc's
    // kstack; activate_first just published current + rsp0 + CURFRAME —
    // the single first ring-3 entry (the caller contract in trap.rs).
    unsafe { trap::trap_return(frame_ptr) }
}

mod runtime;
