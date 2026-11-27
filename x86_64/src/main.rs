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
mod entropy;
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
mod fpu;
#[allow(dead_code)]
mod proc;
// SMP bring-up (SMP-01/02/07, 05-11/05-12): the MADT-driven
// INIT-SIPI-SIPI AP startup, per-CPU Mach allocation, LAPIC IPI senders
// and the SmpArch registration.  main() registers the hooks at the top
// (register_port_smp) and starts the APs after the publish-once boot
// init (start_aps_and_self_check); ap_main (below) is the AP entry.
#[allow(dead_code)]
mod smp;
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

use core::sync::atomic::Ordering;

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
/// bake was ~36 KB; the Phase-15 tier payloads grew it into the MBs;
/// the 15-08 DESK-03 font payload puts the services bake at ~26.5 MB —
/// the 8 MiB-era cap clamped that module and panicked the stash-length
/// reconciliation, so the era moves to 64 MiB, the same 8x step the
/// kernel heap took when the services venue outgrew its arena).  The
/// stash is .bss (NOBITS): no file weight, the loader zeroes it, and
/// the `end` symbol covers it so the page allocator never reuses it —
/// the cost is 64 MiB of allocatable RAM on every venue (all x64
/// configs boot -m 1024M or larger).
const MOD_STASH_SIZE: usize = 64 * 1024 * 1024;

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
pub static mut modstash: ModStash = ModStash([0; MOD_STASH_SIZE]);

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
    // SAFETY: runs once on the boot CPU at CPL 0 with interrupts masked
    // (the mb2 contract above); `mach` is the exclusively borrowed,
    // l.S-initialized Mach, so Mach::init's unsafe-fn contract (the
    // &mut self receiver names valid, live GDT/IDT/TSS tables) is met,
    // and arm_syscall_entry's contract (MSR values matching the
    // just-loaded GDT — its own # Safety section) is met by loading
    // the descriptors first.
    unsafe {
        mach.idt.init(trap::stubs());
        mach.gdt.init(&mut mach.tss);
        mach.init();
        // Arm the SYSCALL/SYSRET entry contract beside the descriptor
        // load — the 9k donor's vsvminit order (descriptors, then the
        // MSRs; vsvm.c:170-185).
        arm_syscall_entry(&mach);
    }

    // SMP-02 (D-25): the BSP's Mach identity (CPU 0) and the x86_64
    // SmpArch hooks registered with port::smp BEFORE any trap can fire
    // with locks held — from this point the IrqSpinLock IRQ
    // mask/restore, the lock-rank assertions and the watchdog run on
    // the real per-CPU hooks (the safe single-CPU defaults until now).
    // The LAPIC-derived apic_id lands after lapic::init() below; the
    // BSP's online flag with it.
    mach.cpu_id = 0;
    smp::register_port_smp();

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

    // MEM-18 (D-20): the BSP's FP save-format selection — CPUID-gated
    // CR4.OSXSAVE + XCR0 = 0x7 (x87|SSE|AVX; AVX-512 off) where XSAVE+AVX
    // report, the untouched FXSAVE fallback on qemu64, and the ONE
    // deterministic boot line (`fpu: format=... avx2=...`).  Runs BEFORE
    // start_aps (each AP's fpu::init_ap applies the published format)
    // and BEFORE fpu::selftest (the selftest exercises the live format),
    // in every config — kernel-only, exec and churn boots alike.
    fpu::init_bsp();

    // devcons's random-device hooks (D-05, AUTH-02, 18-01): the x86_64
    // entropy gather — CPUID-gated RDSEED/RDRAND with the two-sample
    // TSC fold fallback.  The aarch64 main.rs registration re-keyed:
    // there is no DTB on x86_64 and before 18-01 no hooks registered
    // at all (devcons's deterministic-seed era).  Registered
    // unconditionally — the functions branch on CPUID internally —
    // ahead of every consumer (the ether bind's smoltcp seed below and
    // the first exec both draw through the hooks; the first read's
    // `devcons: random seeded` once-marker stays the port side's).
    // The armed source publishes ONCE per boot here, at registration,
    // in the boot-facts family — D-07's gate vocabulary (RDSEED-armed
    // boots print the rdrand-family label; the two-way distinction is
    // RDRAND-family vs jitter).
    port::devcons::set_random_hooks(port::devcons::RandomHooks {
        rng_seed: entropy::rng_seed,
        jitter: entropy::jitter,
    });
    match entropy::armed_source() {
        entropy::Source::Rdseed | entropy::Source::Rdrand => {
            println!("random: source=rdrand");
        }
        entropy::Source::Jitter => {
            println!("random: source=jitter");
        }
    }

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
    // SMP-01/02: the BSP's LAPIC identity + online flag — readable now
    // that the LAPIC is live (x2APIC's MSR ID read needs the unit's
    // mode decision, made in init).
    mach.apic_id = platform::q35::lapic::current_apic_id();
    mach.online.store(true, Ordering::Release);
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
    //
    // SMP-01/SMP-07: the arming window's publish-once cells are all live
    // (ACPI/MADT, LAPIC, IOAPIC, timer, CONS, the virtio bind's
    // DISCOVERY/VIRTIO_VECTOR on dev_ether) — start the MADT-declared
    // APs and prove the LAPIC 0xF0 shootdown path BEFORE the unmask, so
    // an AP never observes a half-armed kernel (D-26's ordering); the
    // BSP's poll is TSC-paced (no tick dependence).
    // SMP-01/SMP-09: this MUST also run on the dev_exec boot path.  The
    // composed gates (smoke/net/rio/tier-1) all boot dev_exec kernels, and
    // the -smp 2 gates assert `smp: cpu 1 online`; leaving AP startup
    // cfg'd out here meant x86_64 never actually ran two CPUs on the
    // composed path, and every -smp 2 gate waited for a marker the kernel
    // could not print.  The arming-window ordering above is identical on
    // both paths, so the APs still never observe a half-armed kernel.
    // IF stays 0 (dev_exec unmasks at the ring-3 iret); start_aps polls
    // online with TSC-paced busy waits, no tick dependence.
    #[cfg(not(test))]
    start_aps_and_self_check();
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
    //
    // 17-02 (dev_churn without the boot chain): the churn profile's
    // bootstrap replaces the first ring-3 proc with the kernel-context
    // churn driver (same prologue; churngate's own walk/dispatch).  A
    // dev_churn BOOT-CHAIN config (the composed venue's kernel) keeps
    // the normal exec_bootstrap — its churn surface is the per-exit
    // heap marker, not the driver.
    // The kernel-SIMD guard: before any process exists, prove a user
    // FPU/SSE state survives a kernel SIMD section (parked at begin,
    // restored at end) and that the example routine computes right.  A
    // failure halts the boot by name, so every exec-profile gate runs it.
    #[cfg(dev_exec)]
    match fpu::selftest() {
        Ok(sum) => println!(
            "fpu: kernel-SIMD selftest ok (user state parked and restored across the section; xor_fold16 {sum:#034x})"
        ),
        Err(why) => panic!("fpu: kernel-SIMD selftest FAILED: {why}"),
    }

    // MEM-17: register the kernel-SIMD bracket, enable AVX2 only when the
    // BSP's CPUID decision armed it (qemu64 stays on the scalar path), and
    // prove the scalar and AVX2 set kernels agree before any process exists.
    // A failure halts the boot by name.
    #[cfg(dev_exec)]
    {
        port::setops::register_simd_guard(|f| fpu::with_simd(|_| f()));
        let avx2 = fpu::avx2_enabled();
        port::setops::enable_simd(avx2);
        if !port::setops::selftest_simd_matches_scalar() {
            panic!("setops: AVX2 kernel disagrees with the scalar reference (MEM-17)");
        }
        if avx2 {
            println!("setops: simd=avx2 selftest=ok");
        } else {
            println!("setops: simd=scalar selftest=ok");
        }
    }

    // SMP-13/D-28: start the in-kernel 9P2000.aead loopback server only when
    // the Multiboot2 command line (GRUB `-append` lineage, the x86_64 analog
    // of the DTB /chosen/bootargs) carries the exact `aeadloop=1` token.  A
    // default boot allocates nothing and exposes no #M.
    #[cfg(all(dev_exec, any(not(dev_churn), dev_bootchain)))]
    {
        if port::aead_loop::maybe_start_from_cmdline(
            core::str::from_utf8(facts.cmdline()).unwrap_or(""),
        ) {
            println!("aeadloop: active device=#M dialect=9P2000.aead");
        }
        exec_bootstrap(facts);
    }

    // SMP-01/SMP-07: the churn driver's venue starts the MADT-declared
    // APs before the bootstrap builds the driver — the APs idle in
    // their sched_loops (empty queues) until the driver's
    // seed_kernel_context enqueue IPIs one (Ready ⟺ enqueued), and the
    // LAPIC 0xF0 self-check proves the shootdown path before any churn
    // proc runs.  The publish-once cells main() wrote are all live;
    // the prologue's port hooks publish before the driver's first pick
    // (the enqueue/IPI pairing orders them).
    #[cfg(all(dev_exec, dev_churn, not(dev_bootchain)))]
    start_aps_and_self_check();
    #[cfg(all(dev_exec, dev_churn, not(dev_bootchain)))]
    platform::q35::churngate::bootstrap_and_run(facts);

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
    // SAFETY: the fn's own contract — runs once at init on the boot CPU
    // at CPL 0 with interrupts masked.  Every wrmsr writes an
    // architectural SYSCALL/SYSRET MSR with a value consistent with the
    // just-loaded GDT (STAR's selector arithmetic is Gdt::star's) or
    // preserves l.S's bits (the EFER read-modify-write keeps LME/NXE);
    // LSTAR names trap.rs's real `syscallentry` asm symbol.  cpu::wrmsr
    // /_rdmsr carry their own operand contracts (cpu.rs).
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

        // Sfmask = If|Tf|Df|Nt|Ac — not the donor's If-only value
        // (cpu::fmask's doc records the D-02a reversal, WINDOWS #71).
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
        // SAFETY: CPL 0; `cli` masks interrupts and `hlt` parks the CPU
        // until the next interrupt — with IF=0 that is forever, which
        // is exactly halt()'s `-> !` contract.  No operands, no memory
        // touched.
        unsafe { core::arch::asm!("cli; hlt") };
    }
}

/// SMP-01/SMP-07/SMP-12 (D-26): the ONE AP-start point, run only after
/// every publish-once boot cell this config writes is live (the
/// category-3 publish-once invariant — the arming window's ACPI/MADT,
/// LAPIC, IOAPIC, DISCOVERY, VIRTIO_VECTOR, TIMER::STATE, CONS, the
/// mb2/modstash stashes; plus, on the dev_exec paths, the prologue's
/// port hooks, the devroot image and the first proc's claim, whose
/// caller has just run them).  The release fence pairs with each AP's
/// SIPI entry so CPU 1 observes the published state.
///
/// Under `-smp 1` (or a MADT declaring the BSP alone) `start_aps`
/// starts nothing and the self-check skips cleanly; a STARTED AP that
/// never publishes online is a bring-up bug (start_one_ap names it).
/// After a CPU comes online, the BSP proves the LAPIC 0xF0 TLB
/// shootdown path end to end: `request_shootdown(None)` returns only
/// once every online remote has acked the generation (a delivered-but-
/// unserviced IPI wedges here loudly instead of passing silently).
#[cfg(not(test))]
fn start_aps_and_self_check() {
    use core::sync::atomic::fence;

    let madt = platform::q35::acpi_facts().and_then(|a| a.madt.as_ref());
    let Some(madt) = madt else {
        println!("smp: no MADT — single-CPU boot");
        return;
    };
    if madt.ncpus <= 1 {
        // The MADT declares the BSP alone (a `-smp 1` boot): no AP
        // startup, no self-check — the clean skip the plan names.
        return;
    }
    fence(Ordering::Release);
    let started = smp::start_aps(madt, proc::boot_pml4_pa());
    if started > 0 {
        // The boot-time TLB shootdown self-check (SMP-07): a full-range
        // shootdown proves CPU 1 takes vector 0xF0 and acks through the
        // port coordinator.
        port::smp::request_shootdown(None);
        println!("smp: tlb shootdown ok");
    }
}

/// The AP entry (SMP-01, SMP-02; 05-12): the SIPI trampoline jumps here
/// with this CPU's Mach VA and logical id (the 0x8F00 mailbox).  Bring
/// up the AP's own GDT/TSS/IDT/LAPIC/SYSCALL MSRs/FPU at BSP parity
/// (T-05-23: an AP must never run with a weaker descriptor/MSR profile
/// than the kernel it joins), publish online, and enter this CPU's
/// scheduler loop (never returns).
///
/// IRQs arrive MASKED (the trampoline's `cli`) and STAY masked through
/// the bring-up; the scheduler loop's `idle_wait` (sti;hlt;cli) is the
/// one deliberate unmask window, and ring-3 proc entries unmask at the
/// iret — the BSP's own discipline (kernel code runs IF=0).
#[cfg(not(test))]
#[unsafe(no_mangle)]
/// Raw, lock-free COM1 byte from the AP bring-up path.  The console
/// driver may be mid-write on the BSP, so a stray byte is possible —
/// this is a bring-up marker, not console output, and it is the only
/// signal that survives a triple fault in the AP.
#[cfg(not(test))]
pub(crate) fn ap_mark(b: u8) {
    // SAFETY: CPL 0 port I/O to the architectural q35 COM1 THR; the
    // poll reads the 16550's LSR (offset 5) THRE bit (0x20).
    unsafe {
        for byte in [b'\n', b'A', b'P', b':', b, b'\n'] {
            let mut spins = 0u32;
            while crate::pio::inb(crate::uart16550::COM1 + 5) & 0x20 == 0 {
                spins += 1;
                if spins > 1_000_000 {
                    break;
                }
                core::hint::spin_loop();
            }
            crate::pio::outb(crate::uart16550::COM1, byte);
        }
    }
}

pub extern "C" fn ap_main(mach_va: usize, cpu_id: usize) -> ! {
    // SAFETY: mach_va names this CPU's live, zeroed, exclusively-owned
    // Mach (start_aps allocated it and wrote its identity before the
    // INIT-SIPI sequence); no other CPU touches it here.
    let mach = unsafe { &mut *(mach_va as *mut dat::Mach) };

    // 0. Tell the BSP this AP is executing. This MUST be the first
    //    publish: start_one_ap only re-sends the SIPI while this is
    //    still false, because a SIPI delivered after this point
    //    restarts us at the real-mode vector with paging already on and
    //    triple-faults the machine (the KVM bring-up bug).
    mach.started.store(true, Ordering::Release);
    ap_mark(b'2');

    // 1. The per-CPU GS base FIRST (SMP-02): every kernel Mach access on
    //    this CPU from here on reads `%gs:0` (dat::this_mach()).  FS = 0
    //    and KernelGSbase = 0 mirror the BSP's arm_syscall_entry (the
    //    user GS/FS bases are 0 on r9 — the closed syscall list).
    // SAFETY: runs once on this AP at CPL 0 with IRQs masked (the
    // trampoline's cli); the MSR writes name this CPU's own Mach.
    unsafe {
        cpu::wrmsr(cpu::MSR_FS_BASE, 0);
        cpu::wrmsr(cpu::MSR_GS_BASE, mach_va as u64);
        cpu::wrmsr(cpu::MSR_KERNEL_GS_BASE, 0);
    }

    // 2. This CPU's own GDT/IDT/TSS — the BSP's main() sequence
    //    re-keyed: fill the descriptor tables, then Mach::init loads
    //    them (lgdt/lidt/ltr) and arms the TSS rsp0 at this Mach's own
    //    stack top.
    // SAFETY: as the BSP's main() — one-time init on this AP, IRQs
    // masked, `mach` the exclusively-borrowed live Mach (Mach::init's
    // unsafe-fn contract: the &mut receiver names valid, live
    // GDT/IDT/TSS tables).
    unsafe {
        mach.idt.init(trap::stubs());
        mach.gdt.init(&mut mach.tss);
        mach.init();
    }

    // 3. The SYSCALL/SYSRET MSR contract, identical to the BSP's
    //    (T-05-23): FS/GS/KernelGS (idempotent re-write), EFER.SCE,
    //    STAR, SFMASK, LSTAR.
    // SAFETY: arm_syscall_entry's own contract — CPL 0, IRQs masked,
    // values matching the just-loaded GDT (both sides derive from
    // seg::GdtData).
    unsafe { arm_syscall_entry(mach) };
    ap_mark(b'3');

    // 4. This CPU's FPU bring-up (MEM-18, D-20): apply the BSP's
    //    published format — CR4.OSXSAVE + XCR0 = 0x7 identically to the
    //    BSP when XSAVE is selected (the trampoline armed only
    //    CR4.OSFXSR/OSXMMEXCPT at BSP parity), with this AP's own CPUID
    //    leaf-1 bits re-verified — then start the CpuFp bookkeeping
    //    clean (05-12's init_cpu; the first proc install restores its
    //    seeded state).
    fpu::init_ap();
    ap_mark(b'4');

    // 5. This CPU's LAPIC + LAPIC timer — armed from the BSP's
    //    published calibration (never a re-run of the PIT/TSC
    //    calibration).
    platform::q35::lapic::init_ap();
    ap_mark(b'5');

    // 6. Publish, in order: this CPU's Mach slot (the IPI senders' cpu→
    //    APIC routing), the online flag (the BSP's start_one_ap poll
    //    pairs on it — Release, so the slot and the identity writes are
    //    visible first), and the online count (the run queues' and the
    //    shootdown coordinator's cpu count).
    smp::publish_ap_mach(cpu_id, mach_va);
    mach.online.store(true, Ordering::Release);
    smp::mark_cpu_online();
    ap_mark(b'6');

    // 7. Enter this CPU's scheduler loop on its own Mach stack (never
    //    returns): pick_next pops this CPU's run queue (stealing a
    //    neighbor's when empty); the idle arm hlt's with IRQs briefly
    //    unmasked — the LAPIC tick and the 0xF0/0xF1 IPIs deliver there.
    proc::sched_loop();
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

/// devcons's echo hook (the aarch64 cons_echo re-key): cooked-mode
/// input echo writes byte-exact through the device-write path — the
/// syscall-context Console-lock precedent; devcons calls it only AFTER
/// the INPUT lock drops (the INPUT→Console non-nesting rule).
#[cfg(dev_exec)]
fn cons_echo(bytes: &[u8]) {
    port::devcons::write_bytes(bytes);
}

/// The exec-bootstrap prologue's outputs (17-02's split): the stashed
/// module image, the devroot root attach chan, and the seeded ROOT env
/// group — everything both the first-ring-3-proc bootstrap and the
/// churn driver's bootstrap consume before their paths diverge.
#[cfg(dev_exec)]
pub(crate) struct ExecPrologue {
    pub(crate) img: &'static [u8],
    pub(crate) slash: port::dat::Chan,
    pub(crate) root_env: alloc::sync::Arc<port::mcslock::Lock<port::devenv::EnvGroup>>,
}

/// The shared first-exec prologue (14-05's body, split out 17-02 so
/// the churn driver's bootstrap consumes the IDENTICAL steps and
/// prints): the mb2 module stash reconciliation, the port-device hook
/// registrations, the ROOT env seed, and the devroot mount with its
/// root attach.  Runs entirely in the IF=0 boot window.
#[cfg(dev_exec)]
pub(crate) fn exec_prologue(facts: &mb2::Facts) -> ExecPrologue {
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
        set_private: proc::proc_set_private,
    });
    port::devcons::set_pid_hook(proc::current_pid);
    port::devcons::set_pool_stats_hook(pagealloc::pool_snapshot);
    // devcons's current-user hooks (18-02, AUTH-03): #c/user serves the
    // per-proc field (seeded glenda, fork-copied), devcap's eve gate
    // and capuse mutation ride the same pair.  NOT dev-gated — every
    // dev_exec platform serves #c/user.
    port::devcons::set_user_hooks(port::devcons::UserHooks {
        current: proc::current_user,
        set: proc::set_current_user,
    });
    // devcap's recorded-but-never-read ticks source (the devip set_clock
    // idiom), under the cap dev key — only the '#¤'-declaring config
    // ever reads it, and even there the value is dead state kept honest.
    #[cfg(dev_cap)]
    port::devcap::set_tick_hook(platform::timer::ticks);
    // devpipe's block/wake hooks — a read on an empty pipe parks the
    // caller via the sched machinery (registered before the first exec
    // so the very first blocked reader resolves).
    port::devpipe::set_hooks(port::devpipe::PipeHooks {
        current: proc::pipe_current_pid,
        block: proc::pipe_block_current,
        wake: proc::pipe_wake,
        note_pending: proc::note_pending,
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
            note_pending: proc::note_pending,
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
    // devssl's fd resolver (19-04, DESK-04) — the same idiom: the "fd
    // N" ctl verb resolves the CALLING proc's fd table through this
    // hook (donor buftochan's fdtochan; ref_clone = the open-ref
    // shape whose conv-free close balances).  #D itself is
    // unconditional in the registry (no dev key — no boot profile
    // joins it this era); only the fd seam needs the arch side.
    port::devssl::set_fd_resolve_hook(proc::current_fd_resolve);
    // devcons's read-path hooks — the aarch64 registration re-keyed
    // (main.rs:536 there): the console's input engine rides the 16550
    // RX ring (drain_rx — the tick's polled sweep is the producer, see
    // uart16550.rs), the sched machinery (block), and the note-group
    // post (the DEL path's foreground-group target).  The echo hook —
    // cooked-mode input echo rides the device-write path (write_bytes,
    // the syscall-context Console-lock precedent); devcons calls it
    // only AFTER the INPUT lock drops.  14-07: the composed chain's
    // interactive gate is the first console-READ consumer on x86_64 —
    // without these hooks rc's #c/cons read errors ("cons read path
    // not initialized") and the typed bytes never leave the UART.
    port::devcons::set_cons_hooks(port::devcons::ConsHooks {
        current: proc::current_pid,
        drain_rx: crate::uart16550::cons_rx_drain,
        block: proc::cons_block_current,
        post_group: proc::post_note_group_of_members,
        echo: cons_echo,
    });

    // The kernel-seeded ROOT env group — the ONE mechanism replacing
    // both 9k's #ec config env and an argv hint (the aarch64 seeding
    // re-keyed: cputype=amd64 feeds boot's execinit path build when
    // the boot chain arrives at 14-06).  19-01 (CPU-02): the mb2
    // cmdline tag's bootconf pairs MERGE into the compiled seed — the
    // GRUB `-append` lineage (the x86_64 /chosen/bootargs analog),
    // image-scoped per the ISO's cmdline, appended so the compiled
    // facts can never be replaced by loader input (T-19-01's
    // pairs-merge-never-replace arm).  An absent/empty cmdline leaves
    // the seed byte-identical to the pre-19-01 boot; a malformed or
    // overlong one is a loud panic HERE, at first boot, before boot(8)
    // can half-run on a mis-staged image.
    let bootconf = mb2::parse_bootconf_pairs(facts.cmdline())
        .unwrap_or_else(|err| panic!("bootconf: mb2 cmdline parse failed: {err:?}"));
    port::devenv::set_group_hook(proc::current_env_group);
    let mut seed: alloc::vec::Vec<(&[u8], &[u8])> = alloc::vec![
        (b"cputype", b"amd64"),
        (b"nobootprompt", b"embed"),
        (b"readparts", b"0"),
    ];
    seed.extend(bootconf.pairs());
    let root_env = port::devenv::seed_root_group(&seed);
    println!("devenv: seeded cputype=amd64 nobootprompt=embed readparts=0");
    // The cmdline-sourced keys on their OWN line (the gate pins the
    // merge here without the compiled line's bytes ever moving).  The
    // compose-time seam validates printable ASCII, so from_utf8 holds
    // for every sanctioned bootconf; a hand-crafted non-UTF8 line
    // prints <?> for the offending side (the PAIRS still seed — this
    // print is evidence, never the mechanism).
    if !bootconf.is_empty() {
        fn show(b: &[u8]) -> &str {
            core::str::from_utf8(b).unwrap_or("<?>")
        }
        port::print!("devenv: bootconf");
        for (k, v) in bootconf.pairs() {
            port::print!(" {}={}", show(k), show(v));
        }
        port::println!();
    }

    // The module paqfs is the root tree (D-04: the format carried
    // verbatim — port::devroot::mount_initrd + the miniz-inflate
    // reader, the collision-refusal discipline included).  exec
    // sources the entry binary through the devroot walk/open path (one
    // mechanism; the walk loops over the entry path's components —
    // devroot's walk is single-component).
    let root =
        port::devroot::mount_initrd(img).unwrap_or_else(|err| panic!("devroot: mount: {err}"));
    println!("q35: paqfs root mounted from module ({} bytes)", img.len());
    let slash = root.attach_root().unwrap_or_else(|err| panic!("devroot: attach: {err}"));
    ExecPrologue { img, slash, root_env }
}

/// The first-process exec orchestration (the aarch64 exec_bootstrap
/// port, 14-05): entered after the q35 IRQ-arming window with IF=0 —
/// the whole bootstrap is a non-preemptible boot critical section (the
/// inflate-heisenbug discipline; interrupts unmask only at the ring-3
/// handoff via the seeded frame's If flag).  Runs the shared
/// prologue, walks+opens BOOT_ENTRY, builds the first proc, execs,
/// switches to the proc's CR3, and hands off to ring 3 through
/// trap_return.  Never returns.
#[cfg(dev_exec)]
fn exec_bootstrap(facts: &mb2::Facts) -> ! {
    let pro = exec_prologue(facts);
    let root = port::devroot::root().expect("devroot: the prologue just mounted the root");
    let entry_path = BOOT_ENTRY;
    let entry_str = core::str::from_utf8(entry_path).unwrap_or("?");
    let slash_chan = pro.slash.dup();
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
    proc.env_group = pro.root_env;
    exec::exec_into(&mut proc, pro.img, &bin, name, argv)
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
    // SMP-01/SMP-07 (D-26): start the MADT-declared APs only NOW —
    // after the bootstrap proc is built AND claimed (activate_first
    // published it Running).  exec_into enqueued it Ready (the
    // Ready ⟺ enqueued contract), so a secondary already in sched_loop
    // would steal it inside the Ready→Running window and double-run
    // pid 1 — the start must wait for the claim (the aarch64 05-22
    // ordering).  The category-3 publish-once invariant holds (the
    // arming window's cells plus the prologue's port hooks, the tick
    // clock and the devroot image are all long live), and the boot-time
    // LAPIC 0xF0 shootdown self-check still lands before userland: pid
    // 1 has never run when it fires.  IRQs stay masked through this —
    // start_one_ap's online poll is TSC-paced, built for the masked
    // dev_exec tail.
    start_aps_and_self_check();
    let frame_ptr = proc.frame;
    // SAFETY: frame_ptr names the seeded forkret frame on the proc's
    // kstack; activate_first just published current + rsp0 + CURFRAME —
    // the single first ring-3 entry (the caller contract in trap.rs).
    unsafe { trap::trap_return(frame_ptr) }
}

mod runtime;
