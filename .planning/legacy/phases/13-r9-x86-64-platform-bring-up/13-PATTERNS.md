# Phase 13: r9 x86_64 Platform Bring-Up - Pattern Map

**Mapped:** 2026-11-06
**Files analyzed:** 26 (11 new + 10 modified + 5 old-gen retirement-sequenced, all in `../r9`; Harvey-side is evidence-only)
**Analogs found:** 24 / 26 (2 with no in-repo analog: grub-ISO derivation, mb2 header — spec-driven, see No Analog Found)

**Path conventions:** `r9/…` = relative to `/home/req/code/r9` (sibling repo, fork `requaos/r9`, branch `main`); `harvey/…` = relative to `/home/req/code/harvey`. Every analog named below was verified git-TRACKED (`git ls-files`) in its repo this session. `/home/req/code/minimega` was not needed — no analog below references it.

**Primary structural template:** `r9/aarch64/src/platform/virt/` — the x86_64 q35 platform tree is a structural mirror of it (CONTEXT "Primary recommendation", RESEARCH Pattern 1/2). **Secondary semantics references:** `harvey/sys/src/9k/k10/*.c` (C semantics for LAPIC/IOAPIC/RSDP, read-only — Plan 9 C style does NOT cross into r9).

## File Classification

| New/Modified File (r9/ unless noted) | Role | Data Flow | Closest Analog | Match Quality |
|--------------------------------------|------|-----------|----------------|---------------|
| `x86_64/src/platform/mod.rs` | platform dispatch (cfg route) | n/a (cfg-gated module dispatch) | `r9/aarch64/src/platform/mod.rs` | exact |
| `x86_64/src/platform/q35/mod.rs` | platform orchestrator | boot-time discovery (batch) | `r9/aarch64/src/platform/virt/mod.rs` | exact |
| `x86_64/src/platform/q35/mb2.rs` | firmware-data parser | transform (mb2 tags → memmap/RSDP PA) | `r9/port/src/fdt.rs` | role-match (parser discipline; format new) |
| `x86_64/src/platform/q35/acpi.rs` | firmware-data parser | transform (RSDP→XSDT→MCFG/MADT) | `r9/port/src/fdt.rs` + `harvey sys/src/9k/k10/devacpi.c` | role-match + semantics-ref |
| `x86_64/src/platform/q35/lapic.rs` | interrupt-controller driver | event-driven (IRQ ack/eoi from IDT ctx) | `r9/aarch64/src/platform/virt/gic/mod.rs` + `gic/gicv2.rs` | exact (registry contract) |
| `x86_64/src/platform/q35/ioapic.rs` | interrupt-controller driver | event-driven (RTE program at boot; level lines) | `r9/aarch64/src/platform/virt/gic/gicv2.rs` + `harvey sys/src/9k/k10/ioapic.c` | exact + semantics-ref |
| `x86_64/src/platform/q35/timer.rs` | timer driver | event-driven (one-shot tick; CPUID-gated mode) | `r9/aarch64/src/platform/virt/timer.rs` | exact |
| `x86_64/src/platform/q35/pci.rs` | bus driver + virtio bind | discovery (batch) + event-driven (IRQ handler) | `r9/aarch64/src/platform/virt/pci.rs` + `harvey sys/src/9k/386/pci.c` (recursion) | exact + semantics-ref |
| `x86_64/src/platform/q35/devcons.rs` | console glue | streaming (console bytes) | `r9/aarch64/src/platform/virt/devcons.rs` + `r9/x86_64/src/devcons.rs` (existing stub) | exact |
| `x86_64/src/main.rs` | kernel entry/orchestrator (REWRITE → main9-x64) | boot-sequence orchestration | `r9/aarch64/src/main.rs` (main9 + arming window) | exact |
| `x86_64/src/l.S` | boot asm (RETAIN+EXTEND: mb2 header) | boot-time transform (32→64 entry) | itself (mb1 header block, lines 107-115) + RESEARCH Code Examples §mb2 | self-extend (no mb2 exemplar in tree) |
| `x86_64/lib/kernel.ld` | linker script (EXTEND: header section) | build | itself (`.text.boot` first at line 17) | self-extend |
| `x86_64/src/pio.rs` | port-I/O utility (EXTEND: inb/inw/inl) | request-response (port access) | itself (out* exist lines 3-22) | self-extend |
| `x86_64/src/uart16550.rs` | uart driver (GROW: LSR-polled TX, Uart impl) | streaming | `r9/aarch64/src/platform/virt/pl011.rs` + itself | role-match |
| `x86_64/src/devcons.rs` | console glue (ABSORB into platform/q35) | streaming | itself (lines 1-21) + `port/src/devcons.rs` | self-extend |
| `x86_64/src/cpu.rs` | reg/instruction utility (EXTEND: cpuid) | request-response | itself (wrmsr lines 130-138) | self-extend |
| `x86_64/src/dat.rs` | kernel data substrate (retirement touch: line 1 re-export) | n/a | itself (line 1 `pub use crate::vsvm::{Gdt, Idt, Tss}`) | self |
| `x86_64/src/{trap,vsvm,proc,syscall,node0}.rs` | old-gen locals (retirement/cfg-dead sequencing — planner discretion) | n/a | `r9/aarch64/src/trap.rs` dispatch_irq (for the new IDT dispatch) | sequencing-ref |
| `x86_64/lib/config_q35.toml` | config (qemu-as-data) | data → argv emission | `r9/aarch64/lib/config_virt.toml` + `config_virtnet.toml` | exact |
| `x86_64/lib/config_q35lowmcfg.toml` | config (perturbation profile) | data | `r9/aarch64/lib/config_virtlowecam.toml` | exact |
| `xtask/src/config.rs` | build-tool config schema/emission (EXTEND: netdev-as-data, q35 keys) | transform (TOML → qemu argv) | itself (`apply_virt_netdev` 576-584, `Qemu` struct 74-105) | self-extend |
| `xtask/src/main.rs` | gate harness (EXTEND: TOML-driven X86_64 arm, -cdrom, boot-test x86_64) | request-response (spawn qemu → assert markers/pcap) | itself (aarch64 arms: QemuStep 580-633, BootTestStep 1315-1460) | self-extend |
| `flake.nix` | packaging (EXTEND: kernel-x64 + grub-ISO derivation) | build | itself (`kernel-virt` derivation 81-107) | self-extend (ISO derivation itself: no analog) |
| `Cargo.toml` (workspace) | build config (EXTEND: check-cfg `q35`) | build | itself (check-cfg list 31-39) | self-extend |
| Harvey `.planning/phases/13-*/` | evidence consolidation | n/a | `.planning/milestones/v2.0-phases/07-*` evidence layout | exact |
| (conditional) x86_64 drift-notes companion README | docs | n/a | `harvey/build/aarch64/README.md` | exact (only if drift found) |

## Pattern Assignments

### `x86_64/src/platform/mod.rs` (platform dispatch)

**Analog:** `r9/aarch64/src/platform/mod.rs` (22 lines — read in full)

**Core dispatch pattern** (lines 9-22): `#[cfg(platform = "…")] pub mod <name>;` + matching `pub use` re-export, with a `not(any(...))` fallback so bare `cargo build/test` keeps working. The header comment documents which platform is the test/default fallback:

```rust
#[cfg(platform = "virt")]
pub mod virt;
#[cfg(platform = "virt")]
pub use crate::platform::virt::*;

#[cfg(any(test, not(platform = "virt")))]
pub mod raspi3b;
```

For q35: `#[cfg(platform = "q35")] pub mod q35;` + re-export; decide the test fallback arm deliberately (aarch64 defaults to raspi3b when no cfg is emitted — x86_64 has no such second board, so `#[cfg(any(test, not(platform = "q35")))]` semantics need a planner decision or the module stays q35-only and tests live host-side in xtask/port).

---

### `x86_64/src/platform/q35/mod.rs` (platform orchestrator)

**Analog:** `r9/aarch64/src/platform/virt/mod.rs` (42 lines — read in full)

**Discovery-prints pattern** (lines 20-42): a `print_machine_info` that walks firmware data and prints one line per fact, ending in `pci::discover_and_print(dt)` — the "what machine am I on" slot. Loud-by-absence, no hardcoded tables (comment at lines 17-19 is the doctrine statement):

```rust
pub fn print_machine_info(dt: &DeviceTree) {
    println!("Machine information:");
    if let Some(mem) = dt.find_device_type("memory")... {
        println!("  Memory:\t{:#x} .. +{:#x}", mem.addr, mem.len.unwrap_or(0));
    }
    for compat in ["arm,pl011", "arm,cortex-a15-gic", ...] {
        if dt.find_compatible(compat).next().is_some() {
            println!("  found: {compat}");
        }
    }
    pci::discover_and_print(dt);
}
```

q35 re-key: the `dt` parameter becomes the mb2 info + ACPI tables (from `mb2.rs`/`acpi.rs`); the facts printed become memory map, RSDP→XSDT walk, MCFG base+buses, PCIEXBAR old→new, MADT IOAPIC base+gsib + each IntSrcOverride, LAPIC base (RESEARCH Pattern 2's extended fact list).

---

### `x86_64/src/platform/q35/mb2.rs` (firmware-data parser)

**Analog:** `r9/port/src/fdt.rs` — heap-less, bounds-checked, error-typed parser discipline.

**Parser error + extract idiom** (fdt.rs lines 7-13, 15-40): typed `ParseError` enum; every field extraction is a checked `bytes.get(..n)?` helper, never raw indexing:

```rust
#[derive(Debug)]
pub enum ParseError { InvalidHeader, InvalidMagic, BufferTooSmall, InvalidToken }
type Result<T> = core::result::Result<T, ParseError>;

fn bytes_to_u32(bytes: &[u8]) -> Option<u32> {
    let init_bytes = bytes.get(..4)?;
    Some(u32::from_be_bytes(init_bytes.try_into().unwrap()))
}
```

**Two-pass slice construction** (fdt.rs lines 62-81, `DeviceTree::new` / `header_totalsize`): pass 1 parses a fixed-size header prefix to learn totalsize; pass 2 forms the full slice. mb2.rs mirrors this exactly: `{ u32 total_size; u32 reserved; }` then 8-byte-aligned tags `{ u32 type; u32 size; }` walked to type 0 — validate `total_size`, per-tag `size >= 8`, tag alignment, and `entry_size` before any slice (RESEARCH Security V5).

**Semantics reference:** `harvey/sys/src/9k/k10/multiboot.c` lines 6-27 (the mb1 `Mbi` struct + `MMap` `{size, base[2], length[2], type}`) — the mb1 shape this replaces; mb2's memory tag entries are `{u64 base, u64 len, u32 type, u32 res}` (type 1 available / 3 ACPI reclaimable — keep type-3 ranges mapped, RESEARCH Code Examples).

---

### `x86_64/src/platform/q35/acpi.rs` (RSDP→XSDT→{MCFG,MADT,HPET} mini-parser)

**Analog:** `r9/port/src/fdt.rs` (parser discipline, as mb2.rs above) + `harvey/sys/src/9k/k10/devacpi.c` (search/checksum semantics — C reference only).

**RSDP discovery semantics** (devacpi.c lines 1236-1256): EBDA-first then ROM scan:

```c
static void*
rsdsearch(char* signature)
{
	/* 1) in the first KB of the EBDA; 2) in the BIOS ROM between 0xE0000 and 0xFFFFF. */
	if(strncmp((char*)KADDR(0xFFFD9), "EISA", 4) == 0){
		bda = BIOSSEG(0x40);
		if((p = (bda[0x0F]<<8)|bda[0x0E])){
			if(rsd = rsdscan(KADDR(p), 1024, signature))
				return rsd;
		}
	}
	return rsdscan(BIOSSEG(0xE000), 0x20000, signature);
}
```

And the checksum-before-use gate (devacpi.c lines 1274-1289): rev>=2 → `sdtchecksum(rsd, 36)` then XSDT PA from `xaddr`; else 20-byte checksum + RSDT. q35 order of preference per RESEARCH: **mb2 tag 15/14 (ACPI RSDP new/old) first** — loader-mediated, matching D-02's theme — EBDA scan as fallback/cross-check. Every table (XSDT, MCFG, MADT, HPET): verify signature + length + checksum before deriving any pointer (devacpi.c `sdtchecksum` idiom at line 416).

---

### `x86_64/src/platform/q35/lapic.rs` + `ioapic.rs` (interrupt controllers — the gic/ analog)

**Analog:** `r9/aarch64/src/platform/virt/gic/mod.rs` (81 lines, read in full) — the publish-once registry contract; `r9/aarch64/src/platform/virt/gic/gicv2.rs` (137 lines, read in full) — the backend driver shape.

**Registry contract** (gic/mod.rs lines 27-81 — THE pattern to copy verbatim for `LAPIC`/`IOAPIC` statics):

```rust
/// Interrupt-context access to the GIC.  Written once at init (before
/// trap::unmask_irq), read lock-free thereafter -- taking an mcslock in IRQ
/// context could deadlock against an interrupted holder.
static GIC: SyncUnsafeCell<Option<Gic>> = SyncUnsafeCell::new(None);

pub fn init(dt: &DeviceTree) {
    match probed {
        // SAFETY: the publish-once discipline — this write runs at boot
        // before trap::unmask_irq, so no IRQ reader can observe a torn
        // write; the cell is never written again.
        Ok(gic) => unsafe { *GIC.get() = Some(gic) },
        Err(msg) => { println!("can't initialise gic: {msg:?}"); }  // tolerant: boot continues
    }
}
pub fn ack() -> u32 { /* lock-free read from IRQ context */ }
pub fn eoi(intid: u32) { /* debug_assert!(intid != SPURIOUS_INTID); lock-free */ }
```

Contract per the module header (lines 1-6): "init discovers/maps/enables, enable arms an INTID, ack/eoi run from IRQ context lock-free, and the spurious INTID 1023 is acked but never EOI'd." x86 keys: LAPIC `ack()` ≈ vector already known from the IDT entry (no IAR read needed on x86 — the CPU pushed the vector), `eoi()` = write 0 to reg 0xB0; IOAPIC `enable(gsi, vector, level)` runs in boot context only, writes RTE unmasked LAST before `sti` (Phase-3 ordering rule).

**Backend driver shape** (gicv2.rs): module-header comment documents the register sequence verified live on qemu (lines 1-9); named register-offset constants at top (lines 22-32: `pub const GICD_CTLR: usize = 0x000;` …); `new()` maps MMIO via `map_device_register` and errors loudly per missing reg (lines 40-86); `enable()` does read-modify-write per register bank with bank-stride arithmetic in comments (lines 95-125); `ack`/`eoi` are single volatile accesses (lines 128-136).

**LAPIC semantics reference:** `harvey/sys/src/9k/k10/apic.c` — MMIO accessors lines 75-85 (`apicrget`/`apicrput` = raw u32 at `apicbase+r`; r9 re-keys these as `io.rs`-style `read_reg`/`write_reg` over a mapped `VirtRange`); `apiceoi` lines 94-99 (`apicrput(Eoi, 0)`); bring-up sequence `apiconline` lines 180-259 — TPR=0xff during setup, `apicrput(Siv, Swen|IdtSPURIOUS)` (software-enable + spurious vector, line 233), outstanding-EOI at line 238, timer calibration via TSC window lines 245-256 (the TSC-deadline calibration input).

**IOAPIC semantics reference:** `harvey/sys/src/9k/k10/ioapic.c` — indirect-register access (lines 29-40, 51-77):

```c
enum {			/* IOAPIC registers */
	Ioregsel	= 0x00,			/* indirect register address */
	Iowin		= 0x04,			/* indirect register data */
	Ioredtbl	= 0x10,			/* Redirection Table */
};
static void
rtblput(Apic* apic, int sel, u32int hi, u32int lo)
{
	sel = Ioredtbl + 2*sel;
	*(apic->addr+Ioregsel) = sel+1;
	*(apic->addr+Iowin) = hi;
	*(apic->addr+Ioregsel) = sel;
	*(apic->addr+Iowin) = lo;
}
```

**Watch-out (RESEARCH State of the Art, "deprecated"):** 9k uses `Iowin = 0x04`; the 82093AA/ACPI-era layout is **IOWIN = 0x10** with RTEs at `0x10 + 2*i` — use 0x10 and verify at bring-up against the MADT-reported address.

---

### `x86_64/src/platform/q35/timer.rs` (TSC-deadline / periodic, CPUID-gated)

**Analog:** `r9/aarch64/src/platform/virt/timer.rs` (140 lines — read in full)

**Surface + arming idiom** (lines 35-42, 69-87): `static TICKS: AtomicU64`, `pub fn ticks() -> u64` (deadlines API), `init()` arms then enables, `arm()` reads frequency at RUNTIME every time (never hardcoded — the comment cites measured TCG 62.5 MHz vs HVF 24 MHz; q35 analog: CPUID 0x15/0x16 → HPET → PIT chain):

```rust
pub fn ticks() -> u64 { TICKS.load(Ordering::Relaxed) }

pub fn init(dt: &DeviceTree) {
    let intid = discover_intid(dt);
    if intid != CNTP_INTID {
        println!("timer: DTB reports CNTP intid {intid}, expected {CNTP_INTID}");  // loud cross-check
    }
    arm();
    CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
}

fn arm() {
    let freq = CNTFRQ_EL0.get();      // runtime-read, never a constant
    CNTP_TVAL_EL0.set(freq / TICKS_PER_SEC);
}
```

**tick() handler shape** (lines 107-140): re-arm FIRST (countdown/one-shot must be re-armed or the timer fires exactly once — the TSC-deadline analog: re-arm = `wrmsr(0x832, rdtsc() + delta)` inside `tick()`), then `TICKS.fetch_add`, gated print, then the cfg-gated sweeps. For Phase 13 the dev_exec/dev_ip arms don't exist yet — copy only the `#[cfg(not(dev_exec))]` print pacing + tick count.

**CPUID gate (no tree analog — spec fact, RESEARCH Pitfall 2):** leaf 1 ECX bit 24 set → TSC-deadline (one-shot, arm = `wrmsr(0x832, rdtsc()+delta)`); clear → LAPIC periodic (DCR divide + ICR initial count, re-arm in handler — the `arm()` re-arm idiom above). Same `tick()`/`ticks()` surface either way. `wrmsr` already exists: `r9/x86_64/src/cpu.rs` lines 130-138. Timer-LVT mode bit 18 selects TSC-deadline on the LAPIC side.

---

### `x86_64/src/platform/q35/pci.rs` (ECAM discovery + virtio-net bind)

**Analog:** `r9/aarch64/src/platform/virt/pci.rs` (967 lines — read in full; the single most important file for this phase)

**Discovery → publish-once → bind arc**, by line range:

1. **Module header doctrine** (lines 1-13): "Everything here is data, never constants … Absence or malformation of any piece is a loud named print and no bind — there is deliberately NO hardcoded-window fallback."
2. **`VirtioNetDiscovery` struct** (lines 54-70): the discovered facts the bind consumes — q35 version swaps `spi/flags` for `gsi/pin` (PIRQ→GSI 16+ derivation, RESEARCH Pitfall 7) and carries `pin` from config offset 0x3D.
3. **`discover_and_print`** (lines 77-177): loud-by-absence prints per fact (`"pci: no pci node in dtb"`, `"pci: no virtio-net device found"`); maps only bus-0's span (`size.min(BUS_SPAN)` — q35 re-keys to recursive walk per D-08); vendor/device/rev reject logic verbatim (transitional 0x1000 rejected loudly line 126-129; modern 0x1041 rev 0 rejected line 130-133); **publish-once cell write at lines 168-176** with the SAFETY comment ("discovery runs once from print_machine_info (boot context, long before trap::unmask_irq)").
4. **The four port seams** (lines 234-312): `EcamCfg: PciCfg` (len/read_u8/read_u32 over ECAM at `base = dev<<15`), `Regs: CommonCfg` (width-matched via io.rs widths), `VolatileMem: RingMem` (keeps the trait's default `fence` — Pitfall 6 verdict: **no x86 override**, notify BAR mapped UC). Copy these structs nearly verbatim; only the range-mapping plumbing changes (q35 MMIO mapping is planner's VA-layout discretion).
5. **BAR sizing/assignment** (lines 366-385 `size_bar64`, 534-557 in bind): all-ones mask/restore (T-07-09), BAR assigned from firmware-provided window (q35: the q35 MMIO window — derive from MCFG/lived config, not a constant), PCI_COMMAND MEMORY|MASTER explicit (lines 209-216, 548-549 — qemu never auto-enables bus-master), BAR readback check (lines 550-557).
6. **`bind_virtio_net`** (lines 513-729): tolerant wrapper (`bind failed: {msg}` print, boot continues); capability walk with per-cap bounds checks against BAR size (T-07-11, lines 572-590); `virtio::negotiate` + offered/negotiated audit print (lines 611-621); queue bring-up order — select → size → program addresses → notify offset → `queue_enable` LAST (lines 461-504); RX pre-post + DRIVER_OK (lines 648-678); publish-once cell writes + `port::devether::publish_driver(...)` + `gic::enable(intid, 0xA0)` + `#l0:` bind print (lines 707-727).
7. **IRQ handler `virtio_net_irq`** (lines 838-967) with its discipline header (lines 809-836): **ISR byte read FIRST (read-to-ack §4.1.4.5) — on x86 this read must precede the LAPIC EOI write (Pitfall 5: EOI-first on a level-triggered still-asserted line = interrupt storm)**; bounded drain with re-check (DRAIN_CAP, T-07-13); no locks/no console on hot path; `rx_ingress`/`tx_retired` port-side entries; one-time config-change print carve-out.
8. **driver ops** (lines 753-807): `virtnet_tx_post` (copy into slot page, post, notify count) and `virtnet_ifstats` (the `[(avail, used, lastused, nintr, nnote); 2]` reconciliation surface for the X64-02 load proof).

**Recursive bus walk analog (D-08):** `harvey/sys/src/9k/386/pci.c` `pcilscan` recursion, lines 461-514 — after scanning a bus, walk the found list for `ccrb == 0x06 && ccru == 0x04` (bridge devices), read `PciSBN`/`PciUBN` (secondary/subordinate bus, config 0x18/0x1C), recurse into the secondary bus, propagate `maxubn` back up:

```c
for(p = head; p != nil; p = p->link){
	/*
	 * Find PCI-PCI bridges and recursively descend the tree.
	 */
	if(p->ccrb != 0x06 || p->ccru != 0x04)
		continue;
	sbn = pcicfgr8(p, PciSBN);
	ubn = pcicfgr8(p, PciUBN);
	...
	maxubn = pcilscan(sbn, &p->bridge);
```

q35 re-key: on SeaBIOS/GRUB the bridges are already programmed — read secondary bus from the bridge header, recurse with the swizzled pin (`(slot + pin) % 4` up the chain, root-bus slot applies PIRQ→GSI=16+pirq), print per-bridge secondary buses (RESEARCH Pattern 2 fact list).

**PCIEXBAR programming (q35-only step, no aarch64 analog — RESEARCH Pitfall 1):** after MCFG parse, write MCH config 0x60 = `base | LENGTH_256M | ENABLE` before the first ECAM access; print `pciexbar: <old> -> <new>` (the audit line). One-shot CF8/CFC use for this single register write is the recorded D-07 carve-out (RESEARCH Open Question 4).

---

### `x86_64/src/platform/q35/devcons.rs` (console glue)

**Analog:** `r9/aarch64/src/platform/virt/devcons.rs` (131 lines) + existing `r9/x86_64/src/devcons.rs` (21 lines)

**Existing x86_64 registration** (x86_64/src/devcons.rs lines 10-21) — the kernel of the pattern, absorb and grow:

```rust
struct Uart16550 { port: u16 }
impl Uart for Uart16550 {
    fn putb(&self, b: u8) { crate::uart16550::putb(self.port, b); }
}
pub fn init() {
    Console::set_uart(|| {
        static CONS: SyncUnsafeCell<Uart16550> = SyncUnsafeCell::new(Uart16550 { port: 0x3f8 });
        unsafe { Ok(&mut *CONS.get()) }
    });
}
```

The port trait is `port::devcons::Uart` (devcons.rs lines 20-22: `pub trait Uart { fn putb(&self, b: u8); }`) registered via `Console::set_uart` (lines 33-40). The aarch64 analog adds the error-printing arm (`"can't initialise uart: {msg:?}"` → `Err`, lines 20-38) — copy that tolerant shape. Phase-13 scope: TX path + echo (the aarch64 `cons_read` cfg arms at lines 56-64 show the not(dev_exec) stub to mirror — exec is Phase 14).

---

### `x86_64/src/main.rs` (REWRITE: main9-x64)

**Analog:** `r9/aarch64/src/main.rs` (676 lines — read in full), function `main9` (lines 127-427)

**The arc to copy** (line-anchored): trap init first (line 129) → **virt deliberately does NOT unmask at combined-init** (comment lines 137-138) → firmware-data parse (two-pass DTB slice, lines 162-167 — q35: mb2 info via EBX, same two-pass discipline) → pagealloc init with reserved ranges (lines 225-236 — q35: mb2 memory tags, keep type-3 ACPI ranges mapped) → `platform::devcons::init` (line 238) → banner (`println!("r9 from the Internet")` line 243) → machine-info slot `platform::print_machine_info(&dt)` (lines 324-325) → **the IRQ-arming window** (lines 364-420, THE ordering pattern):

```rust
#[cfg(platform = "virt")]
{
    platform::gic::init(&dt);
    platform::gic::enable(30, 0xA0);       // timer first
    platform::timer::init(&dt);
    platform::pl011::init_rx();              // console RX
    platform::gic::enable(33, 0xA0);
    #[cfg(dev_ether)]
    platform::pci::bind_virtio_net(&dt);     // NIC bind INSIDE the window
    #[cfg(not(dev_exec))]
    trap::unmask_irq();                      // unmask LAST — the ordering rule
    ...
}
```

q35 ordering: IDT load (`lidt`, cpu.rs lines 91-106) → LAPIC init+SVR → timer init (CPUID-gated) → console RX → IOAPIC RTEs + `pci::bind_virtio_net` → **`sti` LAST** (Phase-3 hazard applies verbatim; mb2 guarantees EFLAGS.IF=0 at entry). Note the existing entry contract to preserve/extend: `pub extern "C" fn main(mach: &mut dat::Mach, _mbdata: u64)` (x86_64/src/main.rs lines 41-42) — `_mbdata` (EBX saved at l.S line 120) becomes the live mb2 info pointer.

---

### `x86_64/src/l.S` + `x86_64/lib/kernel.ld` (mb2 header; boot path is a keeper)

**Analog:** itself. The mb1 header block to replace (l.S lines 107-115):

```asm
start:
	jmp	1f	// Jump over the multiboot header
	// Multiboot 1 header.
	.balign 8
	.long	MULTIBOOT_MAGIC      // 0x1BADB002
	.long	MULTIBOOT_FLAGS
	.long	MULTIBOOT_CHECKSUM
	.balign 16
1:
	cli
	cld
	// Save the multiboot information pointer.
	movl	%ebx, %ebp
	// Mask off the PIC.  We never use it.
	...
```

Everything after is retained: PIC mask (lines 122-127 — already the D-04 posture), zero Mach region, boot PML4s + 32→64 transition (lines 145-243), `warp64` passing `%rbp` (mbinfo) in `%rsi` to `main` (lines 220-243). The entry state assumption (cli/cld, save EBX, GDTR reload early at lines 180-181) already matches mb2's i386 entry contract (RESEARCH Pitfall 3 + Anti-Pattern "Loading segment registers before the own GDT"). Replace only the header block with the mb2 tagged header (magic `0xE85250D6`, arch 0, header_length, checksum, end tag type 0 size 8) — `.balign 8` inside `.text.boot`, which kernel.ld already emits FIRST (kernel.ld line 17: `*(.text.boot)`), keeping it within the first 32 KiB. Add a Wave-0 host-side ELF header-arithmetic unit test (RESEARCH Pitfall 3) — test home analog: `r9/xtask/src/config.rs` tests module (lines 586+) or `port` host tests.

---

### `x86_64/src/pio.rs` + `cpu.rs` (extend the substrate)

**Analog:** itself. pio.rs lines 3-22 show the exact out* shape to mirror for inb/inw/inl (same `asm!`, att_syntax, `#[cfg(not(test))]` guard):

```rust
pub unsafe fn outb(port: u16, b: u8) {
    #[cfg(not(test))]
    unsafe { core::arch::asm!("outb %al, %dx", in("dx") port, in("al") b, options(att_syntax)); }
}
```

cpu.rs: `sti`/`cli` (lines 29-42), `wrmsr`/`_rdmsr` (lines 112-138) exist verbatim for the TSC-deadline arm; add `cpuid` (leaf 1 ECX bit 24) and `rdtsc` following the same asm-block shape.

---

### `x86_64/lib/config_q35.toml` + `config_q35lowmcfg.toml` (config as data)

**Analogs:** `r9/aarch64/lib/config_virt.toml` (34 lines, read in full — the base shape), `config_virtnet.toml` (the dev-list + evidence-channel keys), `config_virtlowecam.toml` (the perturbation-profile shape).

**Base shape** (config_virt.toml lines 1-34): `[build]` target/buildflags → `[link]` script + `load-address` (x86_64 already: `0xffffffff80200000`) → `[config] platform = "virt"` + `dev = ["ether"]` → `[qemu]` machine/accel/cpu/smp/memory keys → `[dist] format = "elf"`. q35 values per D-01: `machine = "q35"`, `accel = "kvm"`, `cpu = "host"`, `smp = 1`, `memory = "1024M"`; dev list staging per D-01a (idle-unbound netdev first commit, `dev = ["ether", ...]` later — each commit bootable).

**Perturbation profile** (config_virtlowecam.toml lines 1-7 header comment + one machine-flag delta): "Identical to config_virt.toml except the machine flag … A driver that hardcoded the standard window would fail this boot loudly". q35 analog: base q35 config + the moved-MCFG qemu arg (RESEARCH Code Examples: `-global q35-pcihost.MCFG=0xe0000000` as an `[qemu] extra`-style data key — schema shape is planner discretion, the requirement is that it stays data).

---

### `xtask/src/config.rs` (EXTEND: netdev-as-data + q35 emission)

**Analog:** itself (817 lines — read in full; structs + emission + tests all in one file).

- **`Qemu` struct** (lines 74-105): machine/accel/cpu/smp/memory/dtb/initrd — extend with the netdev table and/or `extra` args (schema = planner discretion per Open Questions 2/4; keep Option-fields + emit-only-when-set so existing TOMLs parse unchanged — regression-guarded by `qemu_minimal_keys_emit_only_machine_and_dtb`, lines 674-679).
- **`apply_qemu_config`** (lines 371-409): the argv emitter — overrides beat TOML (line 378: `overrides.accel.as_ref().or(config.accel.as_ref())`).
- **`virt_resolved`** (lines 418-439): the accel/cpu resolution chain (`--accel/--cpu` flag > `R9_ACCEL`/`R9_CPU` env > TOML > built-in default, comment at main.rs 609-612) — the q35 analog resolves KVM default + TCG fallback CPU here or in a sibling fn.
- **`apply_virt_netdev`** (lines 576-584) — the stanza to generalize into TOML data:

```rust
pub fn apply_virt_netdev(cmd: &mut Command, config: &Configuration, pcap_path: &str) {
    let virt = config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("virt");
    if !virt { return; }
    cmd.arg("-netdev").arg("user,id=n0");
    cmd.arg("-device").arg("virtio-net-pci-non-transitional,netdev=n0");
    cmd.arg("-object").arg(format!("filter-dump,id=f0,netdev=n0,file={pcap_path}"));
}
```

Reuse the device-model string `virtio-net-pci-non-transitional` verbatim (already the PCI-transport modern-only shape). Tests to mirror: `virt_netdev_stanza_is_virt_only` (lines 686-713) and the argv-emission tests (lines 598-630).

---

### `xtask/src/main.rs` (EXTEND: X86_64 arm → TOML-driven + -cdrom; boot-test x86_64)

**Analog:** itself — the aarch64 arms are the template; the X86_64 arm to replace is lines 715-751 (hardcoded `-M q35`/`-smp 8`/`-m 8192`/`-kernel r9.elf32` via elf32-i386 objcopy — superseded by TOML emission + `-cdrom boot.iso`).

- **QemuStep aarch64 arm** (lines 580-633): load config → `virt_resolved` → `ensure_initrd_image`/`prepare_initrd_dtb` → `apply_to_qemu_step_with_overrides` → `apply_virt_netdev` → serial flags. q35 arm mirrors this with `-cdrom <iso>` in place of `-kernel`.
- **BootTestStep doc comment** (lines 758-776): socket chardev `server=on,wait=on` for capture-before-start; "Marker assertions only, never timings (TCG speed varies)" — the timing-tolerant doctrine.
- **The arch gate to open** (lines 1315-1318):

```rust
fn run(self) -> Result<()> {
    if self.arch != Arch::Aarch64 {
        return Err("boot-test currently supports aarch64 only".into());
    }
```

x86_64 arm: replace with the X86_64 branch (q35 kernel path, `-cdrom`, generous first-marker timeout for SeaBIOS+GRUB — budget ≥10-20 s wall clock, RESEARCH Pitfall 8; `timeout=0` in grub.cfg). Skip/extend `uregcheck::check_for_boot_test()` (line 1325) — the amd64 Ureg ABI is Phase 14's XEXEC-01, out of scope here.
- **Emission block** (lines 1419-1432): `apply_to_qemu_step_with_overrides` + `apply_virt_netdev` + `-display none` + `-chardev socket,id=cons,…server=on,wait=on` + `-serial chardev:cons` + kernel arg — swap kernel arg for `-cdrom`.
- **`BootProfile` enum** (lines 1245-1260): add q35 profiles (Ticks, Net, Lowmcfg) following the doc-commented variant pattern.
- **Teardown** (`kill_process_group`, lines 1269-1282): process-group SIGKILL + reap — reuse verbatim.
- **Pcap assert machinery:** `r9/xtask/src/pcap.rs` (git-tracked, not re-read this session) — the Phase-7 60-byte ARP + guest-MAC assertion re-keyed per Pitfall 10.

---

### `flake.nix` (EXTEND: kernel-x64 package + grub-ISO derivation)

**Analog:** itself — the `kernel-virt` derivation (lines 81-107):

```nix
kernel-virt = pkgs.stdenv.mkDerivation {
  pname = "r9-kernel-virt";
  version = self.shortRev or "wip";
  src = ./.;
  nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
  inherit cargoDeps;
  buildPhase = ''
    runHook preBuild
    ${objcopyExport toolchain}
    cargo xtask build --arch aarch64 --config virtshell
    runHook postBuild
  '';
  installPhase = '' ... cp target/... "$out/kernel-virt.elf" ... '';
};
```

The two-lockfile `cargoDeps` symlinkJoin (lines 70-78, with its load-bearing-name comment) is copied verbatim for the x86_64 package (`cargo xtask build --arch x86_64 --config q35`-equivalent + install the ELF). Devshell gains `grub2`/`xorriso` alongside `[ toolchain pkgs.qemu pkgs.dtc ]` (line 117). **The grub-ISO fixed-output derivation itself has no in-repo analog** — new machinery: `SOURCE_DATE_EPOCH` scrubbing, pinned `-volid`, BIOS-only (no EFI FAT/mtools), double-build hash check before wiring `outputHash` (RESEARCH Pitfall 4; KERNDATE reproducibility discipline from Phase 1 is the precedent — see No Analog Found).

---

### `Cargo.toml` (workspace) (EXTEND: check-cfg)

**Analog:** itself, lines 31-39 — the predeclare discipline (comment block lines 14-30 documents the rule: "each family member must be declared here when code first uses it"). The q35 platform value joins the list **in the same commit that first emits `--cfg platform="q35"`**:

```toml
[workspace.lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = [
    'cfg(platform, values("virt", "raspi3b", "raspi4b"))',
    'cfg(dev_panic)',
    ...
] }
```

Note: x86_64 crate is NOT lint-opted-in today (comment lines 6-10: "x86_64/riscv64 are never compiled by the gates and stay opted out" of the `undocumented_unsafe_blocks` deny) — that posture changes when x86_64 becomes a gated crate; planner sequences the opt-in with the SAFETY-comment discipline.

---

### Old-gen retirement: `x86_64/src/{trap,vsvm,proc,syscall,node0}.rs` + `dat.rs`

**Sequencing reference:** `r9/aarch64/src/trap.rs` `dispatch_irq` (lines 372-403) — the shape the new x86_64 IDT dispatch converges to: ack → match on the published IRQ id (guard `i if i == platform::virt::pci::virtio_irq_intid() && i != 0` — zero hardcoded vectors) → handler (ISR read-to-ack inside) → EOI, with `unhandled IRQ {n}` for strays. `dat.rs` line 1 (`pub use crate::vsvm::{Gdt, Idt, Tss};`) is the retirement coupling to untangle (Pitfall 9). Wave-0: record whether `cargo xtask build --arch x86_64` compiles today; keep every commit bootable on the aarch64 legs.

---

### Harvey-side (evidence + conditional README)

- Evidence: `.planning/phases/13-*/` consolidation with r9 SHA — analog layout: `.planning/milestones/v2.0-phases/07-r9-pci-ecam-virtio-net-devether/` evidence files (serial logs + pcap + SHA record).
- Conditional x86_64 drift-notes companion (only if qemu-11.1.2 drift found): analog `harvey/build/aarch64/README.md` (read header lines 1-30 this session — per-topic "contract" sections + pin-record discipline).

## Shared Patterns

### Publish-once SyncUnsafeCell registry (applies to: lapic.rs, ioapic.rs, pci.rs, devcons/uart, timer)
**Source:** `r9/aarch64/src/platform/virt/gic/mod.rs:27-50`, `virt/pci.rs:188-202`, `virt/devcons.rs:22-33`, `x86_64/src/devcons.rs:18-19`
Write once at boot (before the unmask path), read lock-free from IRQ context thereafter; every write carries a SAFETY comment naming the discipline; failure = print + leave the static empty (tolerant), never panic:

```rust
static GIC: SyncUnsafeCell<Option<Gic>> = SyncUnsafeCell::new(None);
// SAFETY: the publish-once discipline — this write runs at boot
// before trap::unmask_irq, so no IRQ reader can observe a torn
// write; the cell is never written again.
Ok(gic) => unsafe { *GIC.get() = Some(gic) },
Err(msg) => println!("can't initialise gic: {msg:?}"),
```

### Loud-by-absence discovery prints (applies to: q35/mod.rs, mb2.rs, acpi.rs, pci.rs, lapic/ioapic init)
**Source:** `r9/aarch64/src/platform/virt/pci.rs:77-143` — `"pci: no pci node in dtb"`, `"pci: no virtio-net device found"`; `virt/mod.rs:20-42`. One `pci: `-style line per fact; absence = named print + degraded boot; the gates (not the kernel) assert presence.

### IRQ ordering: handlers armed, RTE unmasked LAST, sti at the very end (applies to: main.rs, ioapic.rs, pci.rs bind)
**Source:** `r9/aarch64/src/main.rs:364-419` (arming window; comment "only then unmask IRQs. Any earlier unmask would hit trap()'s sync-path wedge with no handler armed") + `gicv2.rs:98-100` (trigger bits programmed BEFORE the enable write). x86 additions: IOAPIC RTE unmasked last before `sti`; **device ISR read before LAPIC EOI** (level-triggered INTx — `pci.rs:847` "FIRST device touch: the ISR byte (read-to-ack)" + trap.rs:380-401 ack→handler→EOI order).

### MMIO access via bounds-checked volatile wrappers (applies to: lapic.rs, ioapic.rs, pci.rs, uart)
**Source:** `r9/aarch64/src/io.rs:21-114` — `read_reg/write_reg/read_reg8/write_reg16/…` over a `VirtRange` with `offset_addr` bounds check ("offset outside bounds") and per-width SAFETY comments (the ISR-read side-effect comment at lines 68-73 is the read-to-ack precedent). x86_64 re-keys this family over its own mapped ranges (planner VA-layout discretion); port-I/O (`pio.rs`) stays the separate in/out family.

### Unsafe posture: forbid(unsafe_op_in_unsafe_fn) + SAFETY comments + clippy deny
**Source:** `r9/x86_64/src/main.rs:7` (`#![forbid(unsafe_op_in_unsafe_fn)]`), `r9/Cargo.toml:11-12` (`undocumented_unsafe_blocks = "deny"`), `r9/UNSAFE-AUDIT.md`. Every new unsafe block carries a SAFETY comment; unsafe concentrated at the MMIO/DMA boundary.

### Config-as-data with emit-only-when-set + override chain (applies to: config_q35*.toml, xtask config.rs/main.rs)
**Source:** `r9/xtask/src/config.rs:371-409` (Option fields, overrides.or(TOML)), `main.rs:551-565` (CLI > R9_ACCEL/R9_CPU env), tests at config.rs:598-745. New keys must not change existing argv shapes — extend the test table.

### Gate evidence: ordered serial markers + pcap, timing-tolerant, negative panic scan, process-group teardown
**Source:** `r9/xtask/src/main.rs:758-776` (BootTestStep doctrine), `1245-1260` (profiles), `1384-1404` (stale sock/log/pcap removal — "no dead evidence"), `1419-1432` (socket chardev + emission), `1269-1282` (kill_process_group). Phase-7 erratum carried verbatim: 60-byte ARP (ETHERMINTU), slirp-targets-gateway (`10.0.2.2`), pcap asserts exact frame + guest MAC.

### Port seam quartet (the ONLY arch-side device code)
**Source:** `r9/port/src/virtio.rs:116-121` (`PciCfg`), `233-240` (`CommonCfg`), `394-410` (`RingMem` + default `fence` — no x86 override per Pitfall 6), `r9/port/src/devcons.rs:20-22` (`Uart`). Implementation exemplars: `r9/aarch64/src/platform/virt/pci.rs:234-312`. Negotiation constants: virtio.rs:215-222 (`ACCEPT_LO = MAC|STATUS`, `ACCEPT_HI = VERSION_1`) — locked D-03 surface.

### Kernel-side ARP trigger seam (X64-04, exec-free)
**Source:** `r9/port/src/devether.rs:111-149` — `EtherDriverOps { mac, tx_post, ifstats }`, `publish_driver`, and `driver_ops()` (**pub(crate) today** — RESEARCH Open Question 1: widen or add a cfg-gated kernel self-test consumer; the devip adapter is the kernel-side-consumer precedent, comment lines 144-146). `ifstats()`'s `[(u16,u16,u16,u32,u32); 2]` per-queue tuples are the ping-pong/TX-bulk reconcile surface.

## No Analog Found

| File | Role | Data Flow | Reason — use RESEARCH.md instead |
|------|------|-----------|----------------------------------|
| `r9/flake.nix` grub-ISO fixed-output derivation (new sub-derivation) | packaging | build | No reproducible-ISO derivation exists in either repo. RESEARCH Pitfall 4 + Code Examples: `SOURCE_DATE_EPOCH`, pinned `-volid`, BIOS-only, double-build hash check before `outputHash`. Phase-1 KERNDATE discipline is the conceptual precedent only. |
| mb2 tagged header block (in `l.S`) | boot asm | boot-time | No mb2 exemplar in either tree (9k + old l.S are mb1). RESEARCH Code Examples §"mb2 header + entry contract" has the full spec-pinned asm block (magic `0xE85250D6`, checksum arithmetic, end tag); place `.balign 8` inside `.text.boot` (already first in kernel.ld). |
| `x86_64/src/platform/q35/mb2.rs` tag walk | parser | transform | Partial: `port/fdt.rs` supplies the parser discipline (see assignment); the mb2 tag format itself is spec-only (RESEARCH Code Examples §"mb2 memory-map tag"). |
| q35 PIRQ→GSI derivation (inside pci.rs) | platform data | boot-time | Chipset wiring fact, not code: `PIRQ[A-H] → GSI 16-23` (`ich9_pirq_to_gsi` = pirq+16), root-bus default `(slot+intx)%4+4`, RCBA IRR cross-check for D25-D31 — RESEARCH Pitfall 7 is the specification; document + print provenance (`pci: inta -> gsi N`, the `inta -> spi N` analog at pci.rs:162). |

## Metadata

**Analog search scope:** `/home/req/code/r9` (aarch64 platform tree, port crate, x86_64 crate, xtask, flake, Cargo.toml — all read this session) + `/home/req/code/harvey/sys/src/9k/{k10,386}` (C semantics references) + `build/aarch64/README.md`. `/home/req/code/minimega` not consulted (read-only reference, nothing load-bearing found missing).
**Files scanned:** 30+ (all named analogs read; large-file sections: `port/virtio.rs`, `port/devcons.rs`, `port/devether.rs`, `port/fdt.rs`, `xtask/main.rs` via targeted non-overlapping reads)
**Tracked-source gate:** every analog path verified via `git ls-files` in its repo (r9: all platform/port/xtask/flake paths printed tracked; harvey: k10 + 386/pci.c + README printed tracked). No gitignored/mirror paths emitted.
**Pattern extraction date:** 2026-11-06
