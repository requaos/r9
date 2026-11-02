# How r9 Works

**Please update with any material code change!**

This document summarises the flow of each implementation as it currently stands.  We do this because r9 is a hobby project that is dipped in and out of, with months between dips.  In that context, it's really hard to remember how it works and why.  A document like this also serves as a useful introduction to the project.

## aarch64

The crate is a platform tree: `src/platform/{mod.rs,virt/,raspi3b/,raspi4b/}`, dispatched on `--cfg platform="<name>"`, which xtask emits from the `[config] platform` key of the selected `aarch64/lib/config_<name>.toml`.  The raspi modules keep the miniuart/mailbox/GPIO drivers; `platform/virt/` carries the qemu `-M virt,gic-version=2` target (PL011 console, GICv2, generic timer).

### Boot contract (qemu virt)

Virt boots the cargo-built **ELF** directly as `-kernel` (no objcopy/gzip flat-image step; `[dist] format = "elf"`), plus `-dtb aarch64/lib/virt-gicv2.dtb`.  On qemu 11.1.2 an ELF `-kernel` boot enters directly at EL1 and passes **x0 = 0** — no DTB pointer — so both DTB consumers substitute the fixed physical address `0x40000000` (where qemu loads the `-dtb` blob): `init_vm` pre-MMU (raw PA, with a putstr marker) and `main9` on the VA side.  Raspi still receives the DTB pointer in x0 from its boot stub.

The DTB is a committed pair: `aarch64/lib/virt-gicv2.dts` (reviewable source, regeneration procedure in its header) and `virt-gicv2.dtb` (the blob).  Regenerate on qemu pin bumps: `qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -machine dumpdtb=<out>` — the `-cpu cortex-a72` is load-bearing, the default-cpu dump lacks `arm,armv8-timer` and the SMC64 psci IDs — then round-trip through `dtc` (dtb→dts→dtb) and commit both.  Qemu injects `/memory` and `/chosen` at `-dtb` load time, so the committed blob stays minimal.

### Early initialisation (Assembly)

Execution begins in `l.S` at the `start` label.  The entry runs on raspi (stub-loaded) and on qemu virt (ELF entry, directly at EL1) alike.

The code in `l.S` exists to bring us to a state where we can run kernel code in the OS level (EL1).  In order to get there, we run through a number of steps:
1. Disable other cores
2. Ensure we're in EL1
3. Set up an early debug UART for printf-style debugging — cfg-selected per platform: the raspi miniuart body is unchanged; virt writes the PL011 at PA `0x09000000` with zero init (qemu pre-configures it).  Once initialisation has completed we output a single '.' so we at least know we've gotten that far.
4. Set up a set of initial page tables and enable the MMU.  Tables are configured to identity map and to map in kernel space, with a recursive page table entry.  The identity map ensures the code in l.S can continue to run once the MMU is enabled, and allows it to then jump into the higher half of the address space.  The identity window is per-platform: raspi maps PA 0–2MiB (kernel at `0x80000`); virt adds a second 1GiB block at PA `0x40000000` covering the kernel (`0x40080000`) and the DTB.  Note that qemu's ELF entry skips the EL2 path below, so enables l.S performs there (e.g. `CPACR_EL1.FPEN`) are re-done Rust-side on virt (`init_vm`).
5. Set up the initial stack (size set by STACKSZ - 4KiB), defined to be in BSS at the bottom of the file.
6. Clear the BSS.
7. Jump into the rust code, entrypoint `main.rs::main9`.

### Initialisation (Rust)

We're now in rust, in `main9`, and at a highlevel we:
1. Set up interrupt handlers early (`trap::init` installs the EL1 vector table) so that we get a bit of feedback if things go wrong.  IRQ unmask is deliberately split out (`trap::unmask_irq`) — see the ordering rule below.
2. Parse the device tree, enabling more informed initialisation.
3. Set up the mailbox (rpi-specific, but allows us to extract more information from the hardware, and interact directly in certain ways).
4. Initialise the [console](#devcons), ensuring we can use `port::println` to write to the UART.  On virt this is a `Pl011Uart` discovered from the DTB (`arm,pl011` → translated reg block → MMIO map), polled-TX, no GPIO/mailbox setup.
5. Write out some useful system information.  On virt the "what machine am I on" slot is DTB-derived (memory range from `/memory`, `found:` lines for `arm,pl011` / `arm,cortex-a15-gic` / `arm,armv8-timer`) instead of the mailbox board dump.  Right after the banner, an initrd configured via TOML data gets its probe marker (`initrd: start=… end=… size=…`, parsed from `/chosen` `linux,initrd-start/end`, validated against the `/memory` node, reserved in pagealloc) and a paqfs proof: the image range is mapped into the kernel table at KZERO+PA (the same `VaMapping::Offset(KZERO)` convention vminit uses for the DTB) and `port::paqfs` walks the image root, printing one `paqfs: <name>` line per root entry.  With no initrd configured the line is `initrd: none` and the paqfs path never runs.  qemu's `-initrd` is never used: on qemu 11.1.2 it does not deliver for ELF kernels, so xtask places the image with the generic loader and patches `/chosen` into a target/ DTB copy (fdtput, two BE cells per u64).
6. Set up [virtual memory](#virtual-memory) in rust.  (After switching we no longer need the pagetables set up in early initialisation.)
7. Write out the page tables.
8. On virt without `dev_exec`: arm interrupts, then idle in `wfi`.  Under `dev_exec` (04-05): arm interrupts, then run the first-process exec orchestration (below) — it never returns.  On raspi: loop.

### Interrupts (virt)

Device drivers are DTB-discovered, never hardcoded address tables:

- **GICv2** (`platform/virt/gic.rs`): `find_compatible("arm,cortex-a15-gic")` falling back to `arm,gic-400`; GICD/GICC reg blocks mapped via `map_device_register`.  `enable(intid, prio)` does the ISENABLER/IPRIORITYR read-modify-write and writes ITARGETSR (cpu0) for SPIs only — PPIs bank their own targets.  `ack()` reads GICC_IAR, `eoi()` writes GICC_EOIR; the spurious INTID 1023 is acked but never EOI'd.
- **Generic timer** (`platform/virt/timer.rs`): CNTP, PPI 14 → INTID 30.  `CNTFRQ_EL0` is read at runtime on every arm (62.5MHz under TCG, 24MHz under HVF — never hardcoded); the tick handler re-arms TVAL and prints ~1Hz `tick N` lines.  The tick print is `cfg(not(dev_exec))`-gated (04-05, Pitfall 3): with an exec profile, main9 prints after unmask and userland-era syscalls print from trap context, so an IRQ-context `println` can deadlock against the interrupted Console-lock holder; the tick itself keeps re-arming in every config (the D-13 preemption machinery stays live).
- **PL011 RX** (`platform/virt/pl011.rs`): INTID 33 (an SPI, so it gets the ITARGETSR write).  `init_rx` arms IMSC=RXIM|RTIM and CR=UARTEN|TXE|RXE; the handler drains the FIFO on FR RXFE and echoes bytes verbatim via raw DR writes — lock-free, because an IRQ-context lock can deadlock against its interrupted holder.

**Ordering rule:** `trap::unmask_irq()` is called LAST, after the GIC, timer and UART RX paths are armed — an IRQ taken before its handler exists falls into the generic trap path (dump + halt).  The idle loop is `wfi`; trap.S's `eret` resumes it after each IRQ.

`trap()` dispatches IRQs from both EL1 and EL0 origins (ack → timer tick / UART echo → exactly one EOI → return — an EL0-origin IRQ erets back into userland through the same shared restore tail) and dumps the full `TrapFrame` (x0–x30, sp/pc/psr/type) for anything else from EL1 — with esr/far printed from live `trap()` locals, not the frame — then prints `kernel halted` and parks in `wfi` — a deterministic stop rather than a silent wedge.  Lower-EL instruction/data aborts route to the demand-fault handler, EC 0x15 (SVC) prints the wire syscall number (`frame.x0`) and parks (04-05's deliberate endpoint; real dispatch is 04-06), and other EL0-origin sync exceptions kill the process loudly.

### Trap frame = Harvey's Ureg (Phase 4, D-05/D-08)

The `handle_interrupt` frame IS Harvey's `Ureg` (`harvey:aarch64/include/ureg.h`) field-for-field — 35 × u64: x0–x28, x29, x30, then `sp`, `pc`, `psr`, `type`; sizeof 280, align(16), so the frame stride stays 288 with the Ureg at the frame base.  One unified frame serves all 16 vectors.  The frame goes on the CURRENT kernel stack — there is no stack switch (04-05; the 9front arm64 shape): EL1h-origin traps build it on the interrupted kernel stack, EL0-origin traps on `SP_EL1` (which `trap_return` leaves at the trapping process's kernel-stack top).  An earlier switch-to-`interruptstackbase` entry clobbered x0 with the stack address before the first store — every frame's x0 held the interrupt-stack top, silently poisoning the SVC number and any EL0-context x0 — and never restored the interrupted EL1 sp on return; the 04-05 runtime gate exposed both.  `sp`@248 is sourced per trap origin: EL0-origin vectors read `SP_EL0`; EL1-origin vectors store the entry sp (`sp`+288 after the frame alloc — on EL1h, `SP_EL0` holds a stale user SP, not a kernel-context value).  `pc`@256 is `ELR_EL1`, `psr`@264 is `SPSR_EL1`, `type`@272 carries the vector index 0–15 (kernel-side informational).  `ESR_EL1`/`FAR_EL1` are no longer frame fields — `trap()` reads them live at entry and passes them as dispatcher arguments, so a stale-frame syndrome read cannot corrupt fault decode.  The restore tail is factored out of line as the exported `trap_return` label (04-05): the macro branches to it after `trap_unsafe`, and exec's first EL0 entry rides it via the `trap_return_exec` wrapper (x0 = the seeded frame → sp, branch in) — the ONLY way EL0 is ever entered; `swtch` never enters EL0.  The restore writes `ELR_EL1`/`SPSR_EL1` from the frame always, and `SP_EL0` only when `frame.psr & 0xF == 0` (returning to EL0t) — an IRQ that interrupted EL1 kernel code never scribbles a kernel address into the user stack pointer, and an EL1h return's `add sp, #288` reproduces the interrupted kernel sp exactly.

The layout is a published cross-repo contract, gated at build time in two layers (D-06): in-crate const asserts in `trap.rs` fail `cargo build` on any drift (always on, no Harvey checkout needed), and `cargo xtask ureg-check` parses the sibling Harvey checkout's `ureg.h` (`$R9_HARVEY_DIR`, default `../harvey`) and fails every boot-test profile on field/order/count drift.  The canonical statement is the "Kernel-side enforcement (Phase 4 appendix)" section of `harvey:aarch64/ABI.md`, which also carries the pinned offset table and the kernel-side argument-read rule (`args at ureg->sp + 8`; `SP+0` reserved).

### Userspace-exec machinery (Phase 4, R9-02/R9-03)

The proc/exec/fault spine is compiled into every build config; 04-05 wired the runtime path (`--config virtexec` = `dev = ["exec"]`): after IRQ arming + unmask, `exec_bootstrap` mounts the initrd paqfs as **devroot** (`port/src/devroot.rs` — attach/walk/open/read/stat over the stateless reader, D-15), sources `/init` through it (walk → open OEXEC — one mechanism), builds the first proc, and `exec_into`s it.  Then the D-10 teardown: the proc's fully-built root table replaces the TTBR0 identity map (`vm::switch(_, RootPageTableType::User)` — closes Phase-3 WR-01; the identity map now exists only for the pre-MMU→MMU transition), the immediate `exec: ttbr0 switched, console alive` print proves all runtime MMIO is TTBR1-side, and `trap::trap_return(frame)` erets into EL0 from the seeded forkret frame.  Serial evidence on the virtexec profile: banner → `initrd: start=…` → `paqfs:` root entries → `ctr_el0: 0x…` (the probed cache geometry) → `devroot: /init open` → `exec: entering EL0 pc=… sp=…` → ttbr0 proof → demand-fault markers → init's first SVC arriving as `syscall 51 unimplemented` (the loud-unimplemented endpoint; real dispatch is 04-06).  The D-12 experiments (the raspi-gated user-table singleton init and the syscall-exit experiment) are deleted — git history preserves them.

- **Process scaffolding** (`aarch64/src/proc.rs`): `Proc` carries its own 16 KiB kernel stack, its own root page table (one pagealloc'd page per process with the recursive slot-511 self-entry — never the `USER_PAGETABLE` singleton, which would give every process one address space), a 16-slot fd table, a mapping log (per-mapping va/phys/size/kind for rfork-copy/exit-free), the stashed trap frame pointer, and note/wait/errstr slots.  `PROC_TABLE` is a fixed 8-proc array behind the MCS-lock idiom, taken only briefly in process/boot context — never across the fault handler.  Trap-context code reads the current proc via `current_proc()`'s raw pointer (single CPU, IRQ-masked handlers — the pointer cannot change under a reader).
- **a.out loader** (`aarch64/src/exec.rs`): parses the big-endian Plan 9 header (R_MAGIC arm64 `0x8C47`, 40-byte HDR_MAGIC form, real entry the BE u64 at bytes 32..39) and validates the donor sysexec check set — entry inside text, text+data+bss below USTOP — before any mapping exists.  Exec maps EXACTLY two eager pages (demand-fault-everything, D-09): the top-of-stack seed page and a zeroed Tos page below it, written via their KZERO+PA kernel aliases (the proc's table is not active yet), with `[argc][argv0]…[nil]` + strings laid out per 9front's execregs.  A zeroed TrapFrame at `kstack_top - 288` (R0=tos, SP→user stack, PC=entry, PSR=0 EL0t, LR=0) is the forkret shape: the normal trap-restore path `eret`s into EL0 from it, and the first instruction fetch faults.  r9 owns the user address-space policy (D-11): USTACKTOP = `0x0000_8000_0000_0000`, 16 MiB stack bound, USTOP below it; text/data VAs come from the 7l-fixed header (text maps from 0x10000 with the header included, data at the 64K-aligned VA above text).
- **Demand-fault handler** (`aarch64/src/fault.rs`): trap()'s EC split routes lower-EL instruction/data aborts (EC 0x20/0x24) to `user_fault(frame, esr, far)` — ESR/FAR read live at trap entry, never frame fields.  The faultarm64 class table routes translation/access-flag/permission/TLB-conflict to the demand path; text/data pages are read from the proc's paqfs byte source at byte granularity (text file offset = pageVA − 0x10000; data = (pageVA − data_va) + 40 + textsize) and written THROUGH the user VA (the faulting process's TTBR0 is live — no aliasing, no PAN on cortex-a72), followed by the CTR_EL0-derived `DC CVAU` → `DSB ISH` → `IC IVAU` → `DSB ISH` → `ISB` sequence on the user VA after every text page-in (geometry always discovered, never hardcoded).  bss/stack/tos faults map zero pages; stack growth is bounded at 16 MiB; a fault on an already-mapped page or outside all segments kills the process loudly (print + Moribund + park), and a kernel-context FAR panics loudly (T-04-09).  Evidence markers (`fault: text page-in va=…`, `fault: data page-in va=…`, `fault: stack growth va=…`) prove demand paging on serial.  The EC-0x15 SVC placeholder now prints the wire syscall number from `frame.x0` (args live at `SP+8` per the ABI), and other EL0-origin sync exceptions take the faultnote-shaped kill path; the EL1-origin panic dump is unchanged.
- **Syndrome decode** (`aarch64/src/registers.rs`): `EsrEl1IssDataAbort` (WnR = bit 6, write-not-read) + `DataFaultStatusCode` mirror the instruction-abort idiom, with lower-EL constructors; host unit tests pin both decoders.

### Debug panic config (dev_panic)

`--config virtpanic` is `config_virt.toml` plus `dev = ["panic"]`, which the generic xtask config mechanism emits as `--cfg dev_panic` (no schema change).  It is a **debug artifact, never a default or production configuration**.  The hook sits at the end of `main9`, after all init but *before* `trap::unmask_irq()`, so the panic log is deterministic (zero IRQ interleaving): it prints a start marker and executes `brk #0`, running the real sync trap path above — full register dump, `kernel halted`, clean `wfi` park.  `cargo xtask boot-test --arch aarch64 --config virtpanic` builds the image and asserts the ordered markers (banner → start marker → ≥30 register rows → halt marker) against a captured serial log.

### Virtual Memory

VM initialisation proceeds as follows:
1. Set up the page allocator.  We use a bitmap allocator defined in `pagealloc.rs`.  This makes use of the `earlypages` reserved in the `kernel.ld` linker script.  The 16KiB reserved here allows us to manage 16\*4096\*8 pages.  Each bit represents 4096KiB, allowing us to map 2GiB of RAM.  The allocator assumes everything except the early pages are allocated at first.  This is to ensure we don't mistakenly allocate a page that should not be available (e.g. will be mapped below).
2. Build an initial memory map.  The mapped ranges are per-platform (`base_physaddr()`): virt starts kernel ranges at the linker's `boottext_pa` (`0x40080000`, so the GIC/PL011 MMIO windows never become Normal-cacheable kernel text) and maps kernel text with 4K pages (the load address is not 2MiB-aligned); raspi keeps base 0 and 2MiB pages.  Both map:
   - Device tree DTB
   - Kernel text, data and BSS
   - MMIO
3. We map each of the above ranges according to the defined entry flags.
4. We now mark all the unallocated pages free.

To document:
- The innards of page table mapping.

Future Improvements:
- The page table management code needs to be able to obtain new page frames while updating page tables.  It's simplest to ensure we have at least X page frames pre-mapped, and available in the page allocator.
- Replace the page allocator with something a little more sophisticated with a higher upper bound.
- If the page allocator made use of something like a free list, we could add the early pages to that free list, and avoid the weird hack of marking everything as allocated up front and having to later mark as unallocated.
- Move the page allocator to port.

## x86_64

## riscv64

## port

`port` is where all the shared subsytems are defined.

### devcons

### fdt

### mcslock

### paqfs

`paqfs.rs` is a `no_std` reader for Plan 9 paqfs/bootfs images (Harvey `sys/src/cmd/paqfs/` is the producer + reference reader), over a memory-resident `&[u8]` — the initrd root-fs format.  It parses the 44-byte header (magic `0x529AB12B`, small form only), the 28-byte trailer (magic `0x6B46E688`, root DirBlock offset; the sha1 is not verified), and 12-byte block headers (magic `0x198A1CBF`, type Dir/Data/Pointer, encoding NoEnc/DeflateEnc).  NoEnc payloads are always blocksize bytes on disk; DeflateEnc is raw RFC 1951 inflate (`miniz_oxide`, output hard-capped at blocksize), and every block's adler32 header field is checked against the DECODED payload — note Harvey's libflate `adler32(0, …)` is the seed-0 variant, reproduced via `adler2::Adler32::from_checksum(0)`, not the RFC 1950 seed.  The API is stateless-per-call (`open_image` / `read_block` / `root_dir` / `read_dir` / `walk` / `read_at`): no statics, no locks, all offsets bounds-checked before slicing, every failure a typed `Err` — reentrant by construction so the future demand-fault handler can call it in trap context.  Host tests (`port/tests/paqfs_test.rs`) verify byte-level against the committed mkpaqfs fixture `port/tests/fixtures/probe-tree.paq` plus a synthetic in-test image, including corrupt-input negatives.
