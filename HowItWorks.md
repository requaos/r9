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
5. Write out some useful system information.  On virt the "what machine am I on" slot is DTB-derived (memory range from `/memory`, `found:` lines for `arm,pl011` / `arm,cortex-a15-gic` / `arm,armv8-timer`) instead of the mailbox board dump.
6. Set up [virtual memory](#virtual-memory) in rust.  (After switching we no longer need the pagetables set up in early initialisation.)
7. Write out the page tables.
8. On virt: arm interrupts, then idle in `wfi`.  On raspi: loop.

### Interrupts (virt)

Device drivers are DTB-discovered, never hardcoded address tables:

- **GICv2** (`platform/virt/gic.rs`): `find_compatible("arm,cortex-a15-gic")` falling back to `arm,gic-400`; GICD/GICC reg blocks mapped via `map_device_register`.  `enable(intid, prio)` does the ISENABLER/IPRIORITYR read-modify-write and writes ITARGETSR (cpu0) for SPIs only — PPIs bank their own targets.  `ack()` reads GICC_IAR, `eoi()` writes GICC_EOIR; the spurious INTID 1023 is acked but never EOI'd.
- **Generic timer** (`platform/virt/timer.rs`): CNTP, PPI 14 → INTID 30.  `CNTFRQ_EL0` is read at runtime on every arm (62.5MHz under TCG, 24MHz under HVF — never hardcoded); the tick handler re-arms TVAL and prints ~1Hz `tick N` lines.
- **PL011 RX** (`platform/virt/pl011.rs`): INTID 33 (an SPI, so it gets the ITARGETSR write).  `init_rx` arms IMSC=RXIM|RTIM and CR=UARTEN|TXE|RXE; the handler drains the FIFO on FR RXFE and echoes bytes verbatim via raw DR writes — lock-free, because an IRQ-context lock can deadlock against its interrupted holder.

**Ordering rule:** `trap::unmask_irq()` is called LAST, after the GIC, timer and UART RX paths are armed — an IRQ taken before its handler exists falls into the generic trap path (dump + halt).  The idle loop is `wfi`; trap.S's `eret` resumes it after each IRQ.

`trap()` dispatches IRQs (ack → timer tick / UART echo → exactly one EOI → return) and dumps the full `TrapFrame` (x0–x30, esr/elr/far) for anything else, then prints `kernel halted` and parks in `wfi` — a deterministic stop rather than a silent wedge.

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
