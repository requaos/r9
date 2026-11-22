# Phase 13: r9 x86_64 Platform Bring-Up - Research

**Researched:** 2026-11-06
**Domain:** r9 kernel x86_64 platform layer (multiboot2 boot, LAPIC/IOAPIC, TSC-deadline timer, ACPI mini-parser, PCI/ECAM, virtio-net on q35) + xtask gate machinery + nix flake packaging
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
Copied verbatim from `.planning/phases/13-r9-x86-64-platform-bring-up/13-CONTEXT.md`:

**Design theme (user's consistent through-line — honor downstream):** modern-only, KVM-native, no legacy compatibility. Every fork below resolved to the modern option, twice deliberately past the agent's recommendation (D-02 mb2, D-05 TSC-deadline).

**Machine + gate shape**
- **D-01:** **qemu machine = q35**; gate shape = **KVM-native default** (`-accel kvm -cpu host`) with TCG + a generic CPU model as the documented fallback (X64-05), `-smp 1` (SMP stays a Future item), `1024M` memory. Mirrors Phase 3's D-06/D-07 accel-chain shape and the platforms-as-TOML-data doctrine (`[qemu]` section keys in the x86_64 config TOML).
- **D-01a:** qemu invocation stays single-sourced as TOML data in the x86_64 config; the netdev stanza lands in the same idle-unbound-then-bound order Phase 7's D-10 established (each commit stays bootable).

**Boot protocol + platform data**
- **D-02:** **Multiboot2 via the GRUB2 chain** (deliberate user override of the mb1 recommendation): qemu's built-in `-kernel` loader speaks mb1 only, so the gate pipeline boots SeaBIOS → GRUB2 → kernel from a **nix-built `grub-mkrescue` ISO**; xtask gains `-cdrom` instead of `-kernel`. Rationale: mb2's tagged protocol, loader-mediated entry matching Harvey's EFI/ISO boot lineage, and qemu-independence (real hardware, other loaders). The protocol decision is recorded against 9k's existing multiboot(1) path — 9k stays mb1; r9 x86_64 takes mb2 as its own recorded contract (X64-01). — **Reversibility:** costly — the supply chain gains the ISO build step and the mb2 tag parser; reverting to mb1 drops the loader but rewrites the entry + info-structure code.
- **D-03:** **Platform data = mb2 tags + ACPI at runtime, no fw_cfg.** Memory map from the mb2 memory tags; PCI window from ACPI MCFG; interrupt routes from ACPI MADT (D-06/D-07). fw_cfg was considered and declined — a qemu-specific side channel is a second protocol to maintain; revisit only if ACPI parsing proves prohibitive (see Deferred).

**Interrupts + timers**
- **D-04:** **LAPIC + IOAPIC only — the 8259 PIC is a documented non-goal** (mirrors the GICv2-decision shape; KVM's in-kernel APIC is the accelerated path). virtio INTx rides IOAPIC routes with ISR read-to-ack carried from Phase 7.
- **D-05:** **TSC-deadline LAPIC timer** (deliberate user override of the periodic recommendation): one-shot arming per deadline — maps directly onto the existing one-shot-deadline idiom in `port/` (the NET_DEADLINE/poll_at pattern from Phase 8). Invariant-TSC handling and TCG-fallback behavior are researcher items. — **Reversibility:** costly — the tick source's arming model threads through scheduler wakeup paths.
- **D-06:** **IOAPIC routing overrides read from ACPI MADT at boot** — zero hardcoded GSI mappings; the x86 enforcement of Phase 7's no-hardcoded-IRQs discipline.

**PCI/ECAM**
- **D-07:** **PCI config space via the ACPI MCFG-located ECAM/MMCONFIG window — no CF8/CFC port path, no fixed base.** ONE ACPI mini-parser (RSDP → XSDT) serves both MCFG (D-07) and MADT (D-06) — the x86 analog of `port/fdt.rs`. The moved-window perturbation proof re-derived: qemu moves the q35 PCIEXBAR base, MCFG reports the new base, the kernel follows and still binds the device (X64-03).
- **D-08:** **Recursive bus walk from bus 0** reading bridge headers (carries over the aarch64 `pci.rs` ECAM scan shape; q35 root ports make recursion load-bearing, not theoretical).

**Carried forward (locked in roadmap/prior phases — do not re-litigate)**
- From-scratch virtio transport in `port/` (no crate), modern-only negotiation (VERSION_1 required, legacy/transitional rejected loudly), zero-offload mask MAC+STATUS, split virtqueues, ISR read-to-ack (Phase 7 D-01..D-04).
- Data-driven discovery, zero hardcoded addresses/IRQs + perturbation proof (Phase 7 NETDRV-01; D-06/D-07 above apply it to x86).
- Platform-tree structure per the riscv64/aarch64 precedent (Phase 3 D-01): `x86_64/src/platform/<name>/` + `[config] platform` TOML key; platform-as-data for qemu args.
- Gates stay r9-side (`cargo xtask boot-test` profiles, serial markers, negative panic scan); canonical evidence consolidated in Harvey `.planning/phases/13-*/` with the r9 commit SHA recorded; timing-tolerant presence assertions.
- push-before-pin: r9 pushes to `main` of `requaos/r9` before any `flake.lock` advance; the pin advance is its OWN commit + fresh hosted CI, never bundled.
- The `port` crate is shared: aarch64 (virt + raspi), riscv64, and x86_64 targets all stay green — x86_64-only additions are cfg-gated; unsafe concentrated at the MMIO/DMA boundary per `UNSAFE-AUDIT.md`.
- Phase-7 gate payload machinery (baked ARP frame `dd`'d to `/net/ether0/0/data`, filter-dump pcap asserted host-side, ping-pong + TX-bulk reconciliation) is re-keyed, not reinvented — the 60-byte ETHERMINTU padding erratum and slirp-targets-gateway watch-out carry over verbatim.
- qemu-11.1.2 drift accommodations live harness-side, never in vendored/pinned trees (Phase 2 D-12 pattern); x86_64 drift notes land in a companion README alongside `build/aarch64/README.md` if any are found.
- r9 follows its own `rustfmt.toml`; Harvey's Plan 9 C style does not cross the repo boundary; `aarch64` naming discipline unaffected (this phase is `x86_64`).

### Claude's Discretion
Copied verbatim from 13-CONTEXT.md:
- Kernel VA layout / KZERO-analog / linker script + load-address shape for the higher-half x86_64 kernel, and the early pre-MMU debug console mechanics (COM1 16550 already exists in-crate).
- Retirement sequencing of the old-generation `x86_64/src` locals (`proc.rs`, `syscall.rs`, `trap.rs`, `node0.rs`, `vsvm.rs`) as the `port/`-based system lands — keep every commit bootable (execution-gated ordering).
- The mb2 header placement mechanics (spec: 8-byte aligned, within the first 32 KiB of the image) in `l.S`/`kernel.ld`, and which mb2 tags are parsed beyond the memory map.
- The nix `grub-mkrescue` reproducibility mechanics (fixed-output derivation; scrubbed dates/UUIDs) and where the ISO build lives (r9 flake vs xtask).
- Config TOML names (`config_q35.toml`-style), the flake package name (`kernel-q35`/`kernel-x64`-style), and dev-list cfg keying for `dev_ether`/`dev_ip`.
- LAPIC/IOAPIC/TSC driver file layout inside `platform/`; RSDP discovery mechanics (EBDA scan); exact IRQ-accounting invariants for the X64-02 load proof (exact-reconcile for ping-pong, bounds for TX-bulk — Phase 7 D-08 shape).
- Whether the virtio ring memory-ordering override on x86 needs a `RingMem::fence` specialization beyond the core-fence default (Phase 7-03's trait) — researcher verifies x86 ordering semantics against the barrier sandwich.

### Deferred Ideas (OUT OF SCOPE)
Copied verbatim from 13-CONTEXT.md:
- fw_cfg platform-data channel — declined in D-03; revisit only if ACPI parsing proves prohibitive
- x2APIC — NOT this phase (classic LAPIC mode; x2APIC is future hardening)
- MSI-X — Out of Scope this milestone (INTx is the one truth)
- 8259 PIC — documented non-goal (D-04)
- SMP (INIT-SIPI-SIPI), virtio-blk/rng/9p, RTC — Future Requirements (REQUIREMENTS.md)
- Harvey-side compose/boot-r9 on x86_64 — Phase 14 (XEXEC) consumes the X64-05 package
- Invariant-TSC calibration strategy + TCG TSC-deadline verification — researcher scope, planner records
- Kernel VA layout + old-crate retirement sequencing — planner discretion per decisions section

Also NOT in this phase (from `<domain>`): amd64 userspace exec/ABI/boot-chain (Phase 14 — XEXEC-01..05), userspace tiers re-keying (Phase 15), host net modes / tap leg / NETIP-08 (Phase 16), kernel flush-out fixes (Phase 17), auth (Phase 18), cpu/drawterm (Phase 19). The old-generation local `proc.rs`/`syscall.rs`/`trap.rs`/`vsvm.rs` in `x86_64/src` are superseded by the `port/`-based system as the restructure proceeds — their retirement sequencing is planner discretion, not a deliverable.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| X64-01 | r9 boots on qemu x86_64 (q35-vs-pc decision recorded) via a recorded DTB-free boot protocol — Phase-3-style platform gate: banner, timer ticks, serial input echo | mb2 spec details (header/entry/tags — Pitfall 3); GRUB2 mb2 chain mechanics; the existing l.S long-mode substrate; q35 rationale; boot-test x86_64 arm shape (Arch gate at xtask main.rs:1317 must open); SeaBIOS wall-clock timeout tuning |
| X64-02 | APIC interrupt plumbing (LAPIC + IOAPIC) + HPET/PIT timers proven under load; INTx level-triggered virtio ISR read-to-ack re-proven | LAPIC bring-up semantics (9k apic.c reference); IOAPIC indirect-register programming (9k ioapic.c); TSC-deadline KVM-vs-TCG finding (TCG lacks it — CPUID-gate); q35 INTx→GSI derivation chain (qemu source-verified); EOI ordering hazard; Phase-7 D-08 reconcile shape |
| X64-03 | PCI/ECAM discovery without DTB (ACPI MCFG) with a moved-window perturbation proof | MCFG layout + RSDP→XSDT mini-parser scope; **SeaBIOS PCIEXBAR hardcode finding** (kernel must program PCIEXBAR from MCFG); qemu `-global q35-pcihost.MCFG` knob; recursive bridge walk per D-08; lowecam precedent config |
| X64-04 | virtio-net binds on x86_64 via arch-neutral `port/`; devether presents `/net/ether0` with guest ARP host-observable | port seam inventory (Caps/negotiate/PciCfg/CommonCfg/RingMem all arch-neutral); EtherDriverOps publish + kernel-side tx_post consumer (exec-free ARP trigger — exec is Phase 14); netdev stanza (virtio-net-pci-non-transitional + filter-dump); 60-byte ARP + slirp-gateway erratum carried verbatim |
| X64-05 | r9 flake ships an x86_64 kernel package; gates run KVM-native by default, TCG fallback documented | flake kernel-virt sandboxed-derivation template; grub-ISO reproducibility mechanics (SOURCE_DATE_EPOCH/--fixed-time); KVM default in TOML accel keys; TCG fallback needs the periodic-timer leg + generic CPU; CI leg = TCG (hosted runners lack /dev/kvm) |
</phase_requirements>

## Summary

r9's x86_64 crate today is an old-generation stub: an mb1-headered `l.S` that already performs the full 32→64-bit long-mode transition with hand-crafted boot PML4s and a KZERO = `0xffffffff80000000` higher-half link at `0xffffffff80200000`, plus a toy `main()` that context-switches once and hits `int3` (`x86_64/src/main.rs:42-67`). The aarch64 crate is the structural template for everything this phase adds: `platform/<name>/` dispatch via `--cfg platform`, the gic/ init-enable-ack-eoi registry contract, timer.rs's tick sweep, virt/pci.rs's ECAM discovery → publish-once → `bind_virtio_net` arc, and main9's hook-registration + IRQ-arming-window ordering. The entire device stack being re-proven — `port/virtio.rs` (Caps walk, §3.1.1 negotiate, split-virtqueue index model, `RingMem`), `port/devether.rs` (`EtherDriverOps`/`publish_driver`), `port/netif.rs` (`EtherHooks`, `rx_ingress`, `tx_retired`) — is arch-neutral by construction; the x86_64 side implements four port-side seams (`PciCfg`, `CommonCfg`, `RingMem`, `Uart`) exactly as `aarch64/src/platform/virt/pci.rs` does.

Research confirmed every locked decision's premise and surfaced two load-bearing facts the plan must encode. First, **TCG cannot do TSC-deadline**: qemu's `TCG_EXT_FEATURES` "missing" list explicitly names `CPUID_EXT_TSC_DEADLINE_TIMER` (target/i386/cpu.c:911-917) and the TCG APIC model has no deadline code — only KVM exposes it (KVM_CAP_TSC_DEADLINE_TIMER, kvm.c:479-484). The timer must CPUID-gate its mode: TSC-deadline on the KVM-native default, LAPIC divide-config+initial-count periodic on the documented TCG fallback — one architecture, secondary mode, exactly the fallback shape CONTEXT D-05 asked the researcher to verify. Second, the **moved-window perturbation has a SeaBIOS hazard**: `-global q35-pcihost.MCFG=<base>` moves only the ACPI MCFG *table* (acpi-build.c `acpi_get_mcfg` reads the host property); the hardware ECAM window follows guest writes to the MCH PCIEXBAR register (config 0x60), and SeaBIOS hardcodes `Q35_HOST_BRIDGE_PCIEXBAR_ADDR 0xb0000000` (seabios src/fw/dev-q35.h:11) and writes it on every boot. A kernel that only trusts MCFG breaks under the moved window; a kernel that *programs PCIEXBAR from the MCFG base* as part of ECAM bring-up follows the data and passes. Same theme in interrupts: MADT gives IOAPIC base + ISA overrides, but the q35 INTx→GSI wiring (PIRQ[A-H] → GSI 16-23 unconditionally, `ich9_pirq_to_gsi` = pirq+16) is a chipset fact with a partial read-back path (RCBA IRR registers for D25-D31) — the plan must pick and document the derivation chain.

**Primary recommendation:** Build the x86_64 platform layer as a structural mirror of `aarch64/src/platform/virt/` under `x86_64/src/platform/q35/`: mb2-tag + RSDP→XSDT parsers replace the DTB; `lapic.rs`+`ioapic.rs` replace `gic/` (same init/enable/ack/eoi publish-once contract); `timer.rs` CPUID-gates TSC-deadline vs periodic; `pci.rs` re-derives the ECAM scan with the kernel programming PCIEXBAR from MCFG and a recursive bridge walk; the virtio-net bind re-implements only the four port seams. Drive everything from a `config_q35.toml` ([qemu] accel=kvm/cpu=host/smp=1/memory=1024M + netdev stanza as data), boot SeaBIOS→GRUB2→kernel from a reproducible nix-built `grub-mkrescue` ISO, and gate via a new `boot-test --arch x86_64` arm mirroring the aarch64 Ticks/Net profiles.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Boot protocol (mb2 header, 32→64 entry, mb2 info parse) | x86_64 crate machine layer (`l.S` + boot module) | GRUB2/SeaBIOS (loader, out of repo) | Entry state is machine-defined; l.S already owns long-mode entry; mb2 info parse is firmware-data consumption at the platform boundary |
| Kernel VA / early page tables | `l.S` boot PML4s + `kernel.ld` | Rust `vm` re-derivation (planner scope) | Paging must be live before any Rust; higher-half link already in place (`x86_64/lib/kernel.ld:6` `KZERO = 0xffffffff80000000`) |
| Platform data (mb2 tags, RSDP→XSDT→MCFG/MADT/HPET) | `x86_64/src/platform/q35/` (acpi mini-parser = `port/fdt.rs` analog) | — | D-03/D-07: firmware-provided data parsed at boot, bounds-checked, loud-by-absence; no fw_cfg |
| Interrupt controllers (LAPIC, IOAPIC) | `platform/q35/lapic.rs`, `ioapic.rs` | IDT + trap entry (machine layer) | Same tier the GIC occupies on aarch64 (`gic/`); IDT stubs are machine layer (existing `trap.rs` substrate) |
| Timer tick (TSC-deadline primary, periodic fallback) | `platform/q35/timer.rs` | TSC/HPET/PIT calibration sources | The tick drives `proc::tick_timeouts`/`preempt` + devip `tick_check` sweep — arch platform module, per aarch64 timer.rs |
| PCI/ECAM discovery + bridge recursion | `platform/q35/pci.rs` | MCFG/PCIEXBAR programming | The aarch64 virt/pci.rs tier; discovery print + publish-once cell + bind |
| virtio-net transport, devether `#l`, netif | **`port` crate (carried whole)** | x86_64 seam impls (PciCfg/CommonCfg/RingMem) | Arch-neutral by construction (Phase 7/8); x86_64 implements the volatile seams only |
| Console (16550 COM1) | `platform/q35/` Uart impl of `port::devcons::Uart` | `pio.rs` (extend with `in*`) | Console registration is a port trait (`devcons.rs:20` `pub trait Uart`); 16550 + pio exist in-crate |
| qemu orchestration, gates, ISO build | r9 `xtask` (TOML-driven) + r9 `flake.nix` (kernel-x64 pkg + grub ISO) | Harvey `.planning/phases/13-*/` (evidence only) | Platforms-as-data doctrine; gates stay r9-side (Phase 5 D-12); flake consumes in Phase 14 |
| Harvey-side integration (compose, boot-r9) | Phase 14 — out of scope | — | X64-05 delivers the package; consumption is XEXEC |

## Standard Stack

### Core
| Library / Tool | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| r9 `port` crate | in-tree | virtio transport + devether/netif/devip/devcons + namec | Phase 7/8 deliverable, arch-neutral; carried whole per phase boundary [VERIFIED: ../r9/port/src/virtio.rs:394-410 read this session] |
| `x86_64` custom target spec | `r9/lib/x86_64-unknown-none-elf.json` (existing) | kernel-codegen target: `"disable-redzone": true, "code-model": "kernel", "features": "-mmx,-sse,+soft-float"`, rust-lld | Already present at workspace `lib/`; kernel-correct without a new spec [VERIFIED: r9/lib/x86_64-unknown-none-elf.json read this session] |
| rust pinned toolchain | nightly-2026-01-30 (targets incl. `x86_64-unknown-none`) | `-Z build-std=core,alloc` builds | Single source of truth D-09 [VERIFIED: r9/rust-toolchain.toml:4-9 read this session] |
| existing x86_64 crate deps (`x86` 0.52, `bitstruct`, `bit_field`, `seq-macro`, `zerocopy`, `static_assertions`) | locked in Cargo.lock | CPU/Register abstractions substrate | Existing, locked — no new crates needed [VERIFIED: r9/x86_64/Cargo.toml:9-16 read this session] |
| qemu | 11.1.2 (host) | q35/KVM/TCG gates | Project pin; host runs exactly 11.1.2 [VERIFIED: `qemu-system-x86_64 --version` on this host] |
| GRUB2 (`grub-mkrescue`) | 2.14 via `nixpkgs#grub2` | mb2 boot chain (SeaBIOS→GRUB2→kernel) | The only sanctioned mb2 loader; verified available via nix [VERIFIED: live `nix shell nixpkgs#grub2` probe this session] |
| xorriso | 1.5.8.pl02 via `nixpkgs#xorriso` | ISO writer under grub-mkrescue | Same probe [VERIFIED: live nix shell probe this session] |

### Supporting
| Library / Tool | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| nix 2.34.9 (host) | 2.34.9 | flake package + ISO derivation | X64-05 [VERIFIED: `nix --version` on this host] |
| Harvey 9k k10 sources | in-tree | C semantics reference for LAPIC/IOAPIC/HPET/multiboot | Read-only reference (main.c, apic.c, ioapic.c, mpacpi.c, devacpi.c, hpet.c, i8254.c) [VERIFIED: files read this session] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| GRUB2 mb2 ISO chain (D-02 locked) | qemu `-kernel` mb1 | Rejected by user decision — qemu -kernel is mb1-only anyway (see State of the Art) |
| Hand-built RSDP→XSDT mini-parser (D-07 locked) | acpica/amlcpp crate | Never: AML interpretation is a huge unsafe surface; only static tables are needed (MCFG/MADT/HPET) |
| Kernel-side ARP TX trigger | exec'd `dd` via boot chain | exec is Phase 14 (XEXEC); kernel-side `tx_post` consumer precedented by devip adapter (see X64-04 section) |
| `x86` crate 0.52 (existing dep) | raw asm | Keep what exists; extend `pio.rs`/`cpu.rs` rather than adding crates |

**Installation:** no `npm/pip/cargo add` — zero new crates.io dependencies. nix side: `grub2`, `xorriso` (and `mtools` only if an EFI image were built — BIOS-only ISO avoids it; do not add). Host-side dev tooling enters via the r9 devshell/flake, never global installs.

**Version verification:** performed this session on this host: qemu-system-x86_64 11.1.2; nix (Nix) 2.34.9; grub-mkrescue (GRUB) 2.14; xorriso 1.5.8.pl02 (both via `nix shell nixpkgs#grub2 nixpkgs#xorriso`). rust-toolchain.toml declares channel nightly-2026-01-30 with targets `aarch64-unknown-none`, `riscv64gc-unknown-none-elf`, `x86_64-unknown-none` [VERIFIED: r9/rust-toolchain.toml].

## Package Legitimacy Audit

> No new crates.io/PyPI/npm packages are installed by this phase. All Rust dependencies are existing, Cargo.lock-pinned members of the r9 workspace (`x86` 0.52, `bitstruct`, `bit_field`, `seq-macro`, `zerocopy`, `static_assertions`, `smoltcp` 0.14 in port) [VERIFIED: r9/x86_64/Cargo.toml + workspace Cargo.lock present this session]. New host inputs are nixpkgs packages, sourced from the locked nixpkgs flake input — registry legitimacy is delegated to nixpkgs:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| grub2 (grub-mkrescue) 2.14 | nixpkgs | decades | n/a | gnu.org/software/grub | OK (nixpkgs) | Approved |
| xorriso 1.5.8.pl02 | nixpkgs | ~16 yrs | n/a | gnu.org/software/xorriso | OK (nixpkgs) | Approved |
| qemu 11.1.2 | host system | decades | n/a | qemu.org | OK (host pin) | Approved |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.
*No `[ASSUMED]` package names are introduced; the plan must not add any crate without re-running this gate.*

## Architecture Patterns

### System Architecture Diagram

```text
                    HOST (NixOS, KVM-native default)
 ┌──────────────────────────────────────────────────────────────────────┐
 │ cargo xtask build --arch x86_64 --config q35                          │
 │   └─ rustc (lib/x86_64-unknown-none-elf.json, -Z build-std)           │
 │        └─ target/.../x86_64 ELF (higher-half, AT()-physical)          │
 │ nix grub-ISO derivation (grub-mkrescue 2.14 + xorriso, reproducible)  │
 │   └─ kernel ELF + grub.cfg (multiboot2) → boot.iso  ────────────┐     │
 │                                                                  │     │
 │ cargo xtask boot-test --arch x86_64 --config q35 --profile …     ▼     │
 │   qemu-system-x86_64 -M q35 [-accel kvm -cpu host | --accel tcg] ──────┤
 │     -cdrom boot.iso  -netdev user,id=n0 (idle→bound, D-01a)           │
 │     -device virtio-net-pci-non-transitional,netdev=n0                 │
 │     -object filter-dump,…pcap        (host-side pcap evidence)        │
 │     -serial socket (markers asserted in order, timing-tolerant)       │
 └──────────────────────────────┬───────────────────────────┬───────────┘
                                │ serial                    │ pcap
        GUEST (SeaBIOS → GRUB2 → r9 x86_64)                 │
 ┌──────────────────────────────▼───────────────────────────┴───────────┐
 │ l.S: mb2 header → PIC mask → boot PML4 → long mode → main9-x64        │
 │   ├─ mb2 info (EBX): memory-map tags ─→ pagealloc init               │
 │   ├─ ACPI: RSDP (mb2 tag 15 preferred / EBDA+ROM scan) → XSDT        │
 │   │    ├─ MCFG → ECAM base → program MCH PCIEXBAR → ECAM live        │
 │   │    └─ MADT → LAPIC(s), IOAPIC base + GSI base, IntSrcOverrides   │
 │   ├─ LAPIC enable (SVR) + IOAPIC RTEs (level, vector, unmasked LAST) │
 │   ├─ timer: CPUID.1.ECX[24]? TSC-deadline MSR 0x832 : periodic DCR/ICR│
 │   ├─ console: 16550 COM1 Uart → port::devcons                        │
 │   ├─ pci.rs: recursive bridge walk (ECAM) → virtio-net 1af4:1041 rev≥1│
 │   │    └─ INTx pin (cfg 0x3D) → q35 PIRQ→GSI derivation → IOAPIC RTE │
 │   ├─ port::virtio negotiate (VERSION_1|MAC|STATUS) + rings + IRQ     │
 │   └─ port::devether publish_driver → /net/ether0 counters            │
 │        └─ kernel-side ARP frame → tx_post → slirp gateway replies    │
 │             → IRQ retire (ISR read-to-ack → LAPIC EOI) → RX counters │
 └──────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

All kernel work in the sibling repo `/home/req/code/r9` (fork `requaos/r9`, branch `main`; origin confirmed `git@github.com:requaos/r9.git` this session):

```text
../r9/
├── x86_64/
│   ├── lib/
│   │   ├── config_q35.toml        # NEW — D-01 keys (machine/accel/cpu/smp/memory, netdev, dev list)
│   │   ├── config_q35lowmcfg.toml # NEW (or -global key) — the X64-03 perturbation profile
│   │   └── kernel.ld              # existing (KZERO 0xffffffff80000000; ${LOAD-ADDRESS} substitution)
│   └── src/
│       ├── l.S                    # RETAIN+EXTEND — mb2 header replaces mb1; long-mode path is a keeper
│       ├── main.rs                # REWRITE — main9-x64 (port-based bring-up, aarch64/main.rs arc)
│       ├── pio.rs                 # EXTEND — add inb/inw/inl (only out* exist today)
│       ├── uart16550.rs           # GROW — real 16550 driver (Uart impl; LSR-polled TX, RX int/poll)
│       ├── cpu.rs, dat.rs         # RETAIN substrate (sti/cli/lgdt/lidt/ltr/rdmsr/wrmsr; Ureg/Mach)
│       ├── trap.rs, vsvm.rs, proc.rs, syscall.rs, node0.rs  # OLD-GEN — retirement sequencing = planner
│       └── platform/
│           ├── mod.rs             # NEW — cfg(platform="q35") dispatch (aarch64/platform/mod.rs shape)
│           └── q35/
│               ├── mod.rs         # NEW — discovery prints ("what machine am I on")
│               ├── mb2.rs         # NEW — info-structure tag walk (mem map + RSDP tag 15)
│               ├── acpi.rs        # NEW — RSDP→XSDT→{MCFG,MADT,HPET} static-table mini-parser
│               ├── lapic.rs       # NEW — xAPIC MMIO (0xFEE00000-class, MADT-provided), SVR/EOI/LVT
│               ├── ioapic.rs      # NEW — indirect-regs RTE programming (MADT-provided base)
│               ├── timer.rs       # NEW — CPUID-gated TSC-deadline / periodic; ticks() + tick()
│               ├── pci.rs         # NEW — ECAM scan (MCFG base → PCIEXBAR program → recursive walk)
│               └── devcons.rs     # NEW/absorb — console glue (aarch64 platform/virt/devcons.rs shape)
├── xtask/src/
│   ├── config.rs                  # EXTEND — [qemu] netdev keys as data; q35 platform acceptance
│   └── main.rs                    # EXTEND — X86_64 QemuStep→TOML-driven + -cdrom; boot-test x86_64 arm
├── flake.nix                      # EXTEND — kernel-x64 package (kernel-virt template) + grub-ISO derivation
└── Cargo.toml                     # EXTEND — workspace unexpected_cfgs check-cfg gains the q35 platform value
```

### Pattern 1: Platform module as the only machine surface (gic/ → lapic+ioapic)
**What:** One directory per machine behind `platform/mod.rs` cfg-dispatch; a publish-once static registry with an init/enable/ack/eoi-style contract; failure = loud print + degraded (not panic). [VERIFIED: aarch64/src/platform/virt/gic/mod.rs:30-81 read this session — `static GIC: SyncUnsafeCell<Option<Gic>>`, "written once at init (before trap::unmask_irq), read lock-free thereafter — taking an mcslock in IRQ context could deadlock against an interrupted holder"]
**When to use:** verbatim for LAPIC+IOAPIC: `LAPIC`/`IOAPIC` cells published before `sti`, `ack()` reads IRR/write EOI lock-free from IRQ context. IOAPIC `enable(gsi, vector, level)` runs in boot context only.

### Pattern 2: Discovery → publish-once cell → bind (virt/pci.rs arc)
**What:** `discover_and_print()` walks firmware data and prints one `pci: `-style line per fact (loud-by-absence), stores the result in a `SyncUnsafeCell<Option<…>>`; the later bind (dev_ether configs) consumes the stored discovery without a second walk. [VERIFIED: aarch64/src/platform/virt/pci.rs:164-177 + comment "publish-once — discovery runs once from print_machine_info (boot context, long before trap::unmask_irq)"]
**When to use:** q35 pci.rs: MCFG-base/PCIEXBAR-program/bridge-walk discovery → cell → `bind_virtio_net()` implementing the four port seams. Extended x86 facts to print: MCFG base+buses, PCIEXBAR old→new, per-bridge secondary buses, the found device (vendor/device/rev/pin/GSI), MADT IOAPIC base+gsib, each IntSrcOverride.

### Pattern 3: The port-side seam quartet
**What:** The arch side of the whole device stack is exactly four traits: `PciCfg` (len/read_u8/read_u32 — ECAM volatile reads), `CommonCfg` (width-matched common-config access), `RingMem` (volatile ring memory + `fence(Ordering)`), and `Uart` (console putb). [VERIFIED: port/src/virtio.rs:116-121, 233-240, 394-410; port/src/devcons.rs:20]
**When to use:** x86_64 implements them over ECAM/BAR MMIO (VirtRange-analog wrappers; `aarch64/src/io.rs` read_reg/write_reg family is the idiom to re-key — q35 MMIO is mapped via its own vm layer, planner's VA-layout discretion).

### Pattern 4: Config TOML as the single qemu source (D-01/D-01a)
**What:** Every qemu fact lives in `lib/config_<name>.toml` sections `[build] [link] [config] [qemu] [dist]`; xtask emits argv from data; accel/cpu override chain = `--accel/--cpu` flag > `R9_ACCEL`/`R9_CPU` env > TOML > built-in default. [VERIFIED: xtask/src/config.rs:51-118 (Config/Qemu structs), main.rs:1296-1312 comment "virt: --accel/--cpu flag > R9_ACCEL/R9_CPU env > TOML > built-in default"]
**When to use:** `config_q35.toml`: `machine="q35"`, `accel="kvm"`, `cpu="host"`, `smp=1`, `memory="1024M"`, plus netdev-as-data (see Pattern 5) and `dev=["ether","ip"…]` per the idle-unbound-then-bound staging (D-01a). The perturbation config mirrors the lowecam precedent (`config_virtlowecam.toml` = base config + moved-window key + comment header [VERIFIED: read this session]).

### Pattern 5: Netdev stanza single-sourced as data
**What:** aarch64's stanza is Rust-emitted but virt-gated: `apply_virt_netdev` emits `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` + `-object filter-dump,id=f0,netdev=n0,file=<pcap>` only when platform == "virt". [VERIFIED: xtask/src/config.rs:576-586]
**When to use:** For x86_64 the CONTEXT (D-01a, and HNET-02 later) wants the stanza as TOML data — extend the `[qemu]` schema with a netdev table (user-mode + device model + pcap flag) consumed by a generalized emitter; keep the aarch64 emission as the legacy consumer or migrate both to the data path. The device model string `virtio-net-pci-non-transitional` is already the PCI-transport, modern-only shape — reuse verbatim.

### Pattern 6: Boot-test = ordered serial markers + pcap assert, timing-tolerant
**What:** `BootTestStep` builds, boots qemu with serial on a unix socket (`server=on,wait=on` for capture-before-start), asserts ordered markers (banner → ≥3 ticks → typed-char echo) — never timings; negative panic scan; profiles select assertions (Ticks/Panic/Exec/Multi/Notes/Net/Netip). [VERIFIED: xtask/src/main.rs:758-776 doc comment + BootProfile enum 1243-1258]
**When to use:** x86_64 arm: open the `arch != Arch::Aarch64 → Err("boot-test currently supports aarch64 only")` gate at main.rs:1317; add profiles (Ticks, Net, Lowmcfg); skip/extend `uregcheck::check_for_boot_test()` (the amd64 Ureg ABI is Phase 14's XEXEC-01 — a boot-test-time ureg check for x86_64 is out of scope this phase); tune generous timeouts for SeaBIOS+GRUB seconds (CONTEXT watch-out).

### Anti-Patterns to Avoid
- **Trusting MCFG without programming PCIEXBAR** — under the SeaBIOS chain the hardware window sits wherever SeaBIOS put it (0xb0000000 hardcode), not where a `-global`-moved MCFG table claims; the kernel must program the register from the parsed base (see Pitfall 1).
- **Unmasking (sti) before every handler exists** — Phase 3's ordering hazard applies verbatim; also the mb2 spec guarantees EFLAGS.IF=0 at entry, keep it masked until LAPIC+IOAPIC+timer+console-RX are armed (aarch64 precedent: unmask LAST, HowItWorks "Ordering rule").
- **Loading segment registers before the own GDT** — mb2 i386 entry state allows GDTR invalid; existing l.S already lgdt's early — preserve that ordering.
- **Assuming TSC-deadline exists** — probe CPUID.1.ECX[24]; under TCG it never does (see Pitfall 2).
- **EOI before the virtio ISR read** — with a level-triggered INTx line still asserted, EOI-first re-delivers immediately (storm); keep the Phase-7 read-to-ack order (ISR read deasserts device, then LAPIC EOI).
- **CF8/CFC as a device-discovery path** — D-07 rejects it for config-space access; the one legitimate use is programming the host-bridge PCIEXBAR register itself before ECAM is live at the target base (architectural fixed ports, not a device address — flag for user visibility; alternative is writing via the currently-live 0xb0000000 window, which requires knowing that base, i.e. a hardcode).
- **Adding crates** — the whole discipline is from-scratch transport + existing locked deps; any new crate reintroduces the audit surface Phase 7 D-01 rejected.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Boot loader / ISO mastering | custom El Torito/BIOS boot code | `grub-mkrescue` (nixpkgs#grub2) + grub.cfg `multiboot2 /boot/kernel` | Boot-sector correctness, filesystem walker, mb2 loader are all done; D-02 chose the loader-mediated lineage |
| virtio transport / negotiation / rings | any new ring code | `port/src/virtio.rs` verbatim (Caps::parse, negotiate, Ring) | Phase 7 built it; x86_64 implements seams only; negotiation mask constants are the locked D-03 surface [VERIFIED: virtio.rs:215-222 `VIRTIO_NET_F_MAC: u32 = 5; VIRTIO_NET_F_STATUS: u32 = 16; VIRTIO_F_VERSION_1: u32 = 32`] |
| Ethernet file tree / netif model | x86-specific ether files | `port/devether.rs` + `port/netif.rs` | Arch-neutral; `EtherDriverOps { mac, tx_post, ifstats }` is the only arch contract [VERIFIED: devether.rs:111-124] |
| Gate harness / pcap assertion | new expect scripts | xtask BootTestStep + pcap.rs | Ordered-marker + filter-dump machinery exists; the aarch64 profiles are the template |
| nix kernel package | ad-hoc build scripts | flake `kernel-virt` derivation pattern (importCargoLock two-lockfile vendoring + rust-toolchain.toml single source) | X64-05 mirrors the proven sandboxed shape [VERIFIED: flake.nix:81-107 read this session] |
| AML interpretation | DSDT/_PRT evaluation | static tables (MCFG/MADT/HPET) + q35 chipset wiring facts | AML is a language runtime, not data; the static-table subset covers every X64 need |

**Key insight:** the phase's novelty is confined to firmware-data formats (mb2 tags, ACPI static tables) and x86 interrupt/timer registers — all small, spec-pinned, host-testable parsers. Everything else is re-keying proven machinery.

## Common Pitfalls

### Pitfall 1: The moved-ECAM-window perturbation silently breaks under SeaBIOS (X64-03)
**What goes wrong:** `-global q35-pcihost.MCFG=0xe0000000` moves only the guest-visible ACPI MCFG *table* (qemu `hw/i386/acpi-build.c` `acpi_get_mcfg` reads the host's `PCIE_HOST_MCFG_BASE "MCFG"` property). The hardware ECAM window follows *guest writes* to the MCH PCIEXBAR register (config offset 0x60, 64-bit) via `mch_update_pciexbar` — and SeaBIOS writes `Q35_HOST_BRIDGE_PCIEXBAR_ADDR 0xb0000000 | ENABLE` on every boot [VERIFIED: qemu master hw/pci-host/q35.c:186-187, 304-339, 557-558; include/hw/pci/pcie_host.h:31; seabios src/fw/dev-q35.h:9-12; src/hw/pci.c:482-488 — quotes: `#define Q35_HOST_BRIDGE_PCIEXBAR_ADDR   0xb0000000`, `DEFINE_PROP_UINT64(PCIE_HOST_MCFG_BASE, Q35PCIHost, parent_obj.base_addr, MCH_HOST_BRIDGE_PCIEXBAR_DEFAULT)`]. A kernel that maps ECAM purely at the MCFG base reads unmapped MMIO under the perturbation and finds no device — or worse, silently on some firmwares.
**How to avoid:** make "program PCIEXBAR from the parsed MCFG base" a required step of ECAM bring-up: RSDP→XSDT→MCFG gives (base, start_bus, end_bus); write PCIEXBAR = base | LENGTH_256M | ENABLE before the first ECAM access (via CF8/CFC once — the MCH at bus 0/dev 0/fn 0 — or via the currently-live window). This is data-following, not a hardcode: the register write uses the table's value; under the default boot it rewrites the same 0xb0000000|EN SeaBIOS already set (harmless), under the perturbation it moves the window. Print old→new on serial as the audit line.
**Warning signs:** perturbation boot prints MCFG base but then `pci: no virtio-net device found`; all-0xFF vendor reads.

### Pitfall 2: TSC-deadline does not exist under TCG (D-05 fallback)
**What goes wrong:** qemu's TCG feature whitelist comment explicitly lists it as missing: `/* missing: … CPUID_EXT_TSC_DEADLINE_TIMER */` in `TCG_EXT_FEATURES` (target/i386/cpu.c:905-917, read via source fetch this session), the TCG APIC model (hw/intc/apic.c) implements only divide-config+initial-count timer modes, and `qemu64` carries only SSE3|CX16 (cpu.c:3547-3564). Writing MSR 0x832 under TCG hits the unhandled-MSR path. KVM exposes the CPUID bit only when `KVM_CAP_TSC_DEADLINE_TIMER` (kvm.c:479-484).
**How to avoid:** CPUID-gate the timer mode at init: leaf 1 ECX bit 24 set → TSC-deadline (one-shot, arm = write MSR 0x832 = tsc + delta); clear → LAPIC periodic (DCR divide + ICR initial count, re-arm in the handler — the aarch64 timer.rs `arm()` idiom). Same `tick()`/`ticks()` surface either way. TSC frequency also differs: prefer CPUID 0x15/0x16 when present (KVM `-cpu host` on Intel), else calibrate (HPET GCAP_ID period or PIT). Document the chain in the companion README.
**Warning signs:** TCG boot dead after the first timer arm; #GP storm at wrmsr.

### Pitfall 3: mb2 header placement is silent-failure (D-02)
**What goes wrong:** GRUB refuses the image with only "not a multiboot image" if the header is misaligned or past 32 KiB. Spec: header must be 8-byte aligned, entirely within the first 32768 bytes, magic `0xE85250D6`, checksum = −(magic+arch+header_length) mod 2³², terminated by end tag (type 0, size 8) [CITED: gnu.org multiboot2 spec, fetched this session]. The existing l.S places an mb1 header inside `.text.boot` right after `start:` — the mb2 replacement must live in its own 8-byte-aligned section referenced early by kernel.ld (`*(.text.boot)` is already first — keep the header inside it, or add a dedicated `.multiboot` section emitted first).
**How to avoid:** dedicated linker section + `.balign 8` in l.S; a Wave-0 host-side unit test that parses the built ELF's first 32 KiB and validates the header arithmetic (mirrors the const-assert discipline).
**Warning signs:** GRUB drops to its prompt; "error: no multiboot header found".

### Pitfall 4: Non-reproducible ISO breaks the fixed-output derivation (X64-05)
**What goes wrong:** ISO9660 volume creation date/UUID and file timestamps default to build time (grub-devel bug 787795; Debian 831379) — a `grub-mkrescue` ISO is not byte-reproducible by default.
**How to avoid:** set `SOURCE_DATE_EPOCH` for the derivation (honored by modern grub-mkrescue 2.14/xorriso 1.5.8); if the pinned grub lacks it, pass explicit xorriso args (`-volume_date`-family) or `--fixed-time`-equivalent; pin `-volid`; build BIOS-only (no EFI FAT image ⇒ no mtools variability). Verify by building twice and comparing hashes before wiring `outputHash`. [CITED: reproducible-builds.org/system-images; grub-devel/debian threads — MEDIUM confidence; the double-build check is the in-plan verification]
**Warning signs:** fixed-output derivation hash mismatch on rebuild.

### Pitfall 5: Level-triggered INTx + EOI ordering (X64-02)
**What goes wrong:** IOAPIC redirection entries for virtio INTx are level-triggered (ISA-level convention; the GIC flags-cell analog said "level-high" on aarch64). If the LAPIC EOI is written before the device's ISR cap is read (which deasserts the line), the IOAPIC re-delivers the still-asserted level → interrupt storm/wedge.
**How to avoid:** carry the Phase-7 read-to-ack order verbatim: IRQ entry → LAPIC IRR/vector → read virtio ISR cap (deassert) → retire rings/counters → LAPIC EOI write (0xB0). Under KVM the in-kernel APIC handles EOI broadcast + remote-IRR; the ordering discipline is identical.
**Warning signs:** nintr counter racing far past frame count; sustained-load wedge in TX-bulk.

### Pitfall 6: x86 fence lowering and the notify MMIO ordering (discretion item — verdict: no override)
**What goes wrong (in theory):** r9's barrier sandwich is aarch64-worded; on x86 `core::sync::atomic::fence(Release/Acquire)` lowers to a compiler-only barrier (no `mfence`), so correctness rests on TSO. The sandwich's one store→load adjacency (avail-idx store followed by used-flags read, virtio.rs:22-25) is the single pattern TSO permits to reorder — benign in r9 because EVENT_IDX (bit 29) is never negotiated and the avail no-notify flag is never set, so no notify-skip decision depends on that read [VERIFIED: virtio.rs:215-222 accept masks exclude bit 29; :380-381 `const USED_F_NO_NOTIFY`].
**How to avoid:** do NOT specialize `RingMem::fence` for x86 — the default core fence is correct; the notify BAR must simply be mapped strongly-ordered UC (never WC), which the kernel's device-mapping discipline already guarantees. Cheap in-plan verification: objdump the built kernel and confirm no `mfence` is required at the notify site / that fences compiled away as expected; add a virtio_tests-order assertion comment for the x86 case. If EVENT_IDX ever lands in a future mask, revisit with `fence(SeqCst)` at that point.
**Warning signs:** silent wedge under sustained TX — exactly what the Phase-7 host pins exist to catch; re-run them on x86_64.

### Pitfall 7: q35 INTx→GSI derivation is NOT in MADT
**What goes wrong:** MADT gives IOAPIC base + GSI base + ISA overrides only. The mapping from a PCI device's INTx pin to an IOAPIC GSI on q35 lives in (a) the DSDT _PRT (AML — out of scope), (b) qemu's internal `irr` table, (c) the ICH9 RCBA IRR registers — but only for slots 25-31. qemu's wiring, source-verified: `ich9_pirq_to_gsi` returns `pirq + 16` unconditionally (PIRQ[A-H] → GSI 16-23, hw/isa/lpc_ich9.c:238-241), root-bus default `irr[slot][intx] = (slot + intx) % 4 + 4` (PIRQ E-H; lpc.c:118-126 "avoid pirq A-D because they are used for pci express port"), D30 bridge INTA-D → PIRQ E-H (lpc.c:100-107), D25-D31 readable via RCBA IRR regs 0x3140/0x3142/…/0x3150 (include/hw/southbridge/ich9.h:84-90). The LPC PIRQ route registers (0x60-0x63/0x68-0x6B) affect only the PIC leg — irrelevant to an APIC-only kernel.
**How to avoid:** treat "PCI pin → PIRQ → GSI = 16 + PIRQ" as the documented q35 platform fact (the x86 analog of the GIC's SPI-bias-32), *cross-checked at boot* by reading the RCBA IRR registers for the D25-D31 slots where they exist and printing the derived route (`pci: inta -> gsi N` line, the aarch64 `inta -> spi N` analog). Devices behind PCIe root ports use the standard bridge swizzle into the root port's own slot mapping — the recursive walk (D-08) must carry (bus,dev,pin) up to the root-bus slot before applying the formula. Zero per-device hardcoded IRQs is preserved: nothing device-specific is hardcoded; the chipset wiring convention is platform data with provenance.
**Warning signs:** virtio IRQ never arrives; "Unhandled interrupt" storm on an unexpected vector.

### Pitfall 8: SeaBIOS+GRUB wall-clock delays the banner (X64-01)
**What goes wrong:** seconds of firmware time before the first kernel marker; the boot-test socket timeouts tuned for direct `-kernel` boots can expire.
**How to avoid:** generous per-marker timeouts (the aarch64 TCG discipline: assert order, never timing); optionally GRUB `timeout=0` in grub.cfg and `-no-reboot`. Budget the first marker wait at ≥ 10-20 s of wall-clock.
**Warning signs:** gate green on rerun after manual run — classic timeout-too-tight smell.

### Pitfall 9: Old-generation x86_64 locals rot the build
**What goes wrong:** `x86_64/src` still carries the old-generation `proc.rs`/`syscall.rs`/`trap.rs`/`vsvm.rs`/`node0.rs` wired into `main.rs`'s module list; months of `port/` churn may already break `cargo build -p x86_64` today (unverified). The old QemuStep X86_64 arm is also hardcoded (`-M q35`, `-smp 8`, `-m 8192`, `-kernel r9.elf32` via `--output-target=elf32-i386` objcopy) — inconsistent with D-01 and to be replaced by TOML emission [VERIFIED: xtask main.rs:715-751, 478-490].
**How to avoid:** Wave-0: run `cargo xtask build --arch x86_64` (config_default.toml) and record the actual state; then retire-or-cfg-dead the old locals in execution-gated order (planner discretion per CONTEXT), keeping every commit bootable on the aarch64 legs (workspace CI = all crates).
**Warning signs:** cfg mis-gating leaking old modules into the new main; `vsvm.rs` Gdt/Idt types still referenced by dat.rs (`pub use crate::vsvm::{Gdt, Idt, Tss}` — dat.rs:1) when vsvm retires.

### Pitfall 10: Phase-7 net-gate erratum carries verbatim (X64-04)
**What goes wrong:** (a) the baked ARP frame must be **60 bytes, not 42** — the netif-faithful devether write path enforces ETHERMINTU=60 (07-CONTEXT ERRATA: "pad 42+18 zero bytes"); (b) slirp's ARP responder answers only for its OWN addresses — the request must target the gateway `10.0.2.2` (sender `10.0.2.15`) or the ping-pong gate hangs [CITED: 07-CONTEXT.md:145,151 read this session].
**How to avoid:** re-key the payload with both facts baked in; the pcap assertion checks the exact 60-byte frame + guest MAC.

### Pitfall 11: Exec is Phase 14 — the ARP trigger must be kernel-side
**What goes wrong:** the aarch64 net gate drove the ARP `dd` through the exec'd boot chain (`config_virtnet.toml` `dev = ["exec","bootchain","ether"]` + bootfs.paq initrd). Porting that shape to x86_64 would silently pull Phase-14's XEXEC scope into Phase 13.
**How to avoid:** Phase 13's ARP trigger runs from kernel boot context after `publish_driver` — the precedent is devip's ether adapter consuming `driver_ops()`/`tx_post` kernel-side (devether.rs:144-148 "The published driver ops for the kernel-internal ether consumer"); note `driver_ops()` is `pub(crate)` today, so either widen it or add the small kernel-side test path in the x86_64 crate. The full userspace `dd`-through-the-file-contract re-proof belongs to Phase 14/16 gates. Planner should make this seam explicit so the "re-keyed, not reinvented" payload machinery (pcap assert, ping-pong ×N reconcile, TX-bulk) still runs: N kernel-side posts + IRQ-driven RX retire reconcile the same counters.
**Warning signs:** a Phase-13 plan that touches exec/staged-init on x86_64.

### Pitfall 12: workspace check-cfg must learn the new platform value
**What goes wrong:** the workspace `[lints.rust] unexpected_cfgs` allow-list enumerates `cfg(platform, values("virt","raspi3b","raspi4b"))`; emitting `--cfg platform="q35"` without declaring it warns (and violates the predeclare rule documented in r9/Cargo.toml).
**How to avoid:** extend the check-cfg list in the same commit that first emits the cfg (the Cargo.toml comment documents the predeclare discipline; `dev_ether`/`dev_ip` are already declared) [VERIFIED: r9/Cargo.toml:48-66 read this session].

## Code Examples

### mb2 header + entry contract (l.S re-key)
```asm
// Source: multiboot2 spec §3.1.1-3.1.2 (gnu.org manual, fetched this session)
.section .text.boot
.balign 8                  // 8-byte aligned, within first 32 KiB of the IMAGE
mb2_header:
    .long 0xE85250D6       // magic
    .long 0                // architecture: 0 = i386 (protected mode)
    .long mb2_header_end - mb2_header
    .long -(0xE85250D6 + 0 + (mb2_header_end - mb2_header))  // checksum
    // optional tags here (information requests); end tag required:
    .short 0               // type 0 = end
    .short 0               // flags
    .long 8                // size
mb2_header_end:
```
Entry (i386 machine state, spec §3.3): `EAX = 0x36d76289`, `EBX = physical address of the info structure`, PE=1 PG=0, IF=0, GDTR possibly invalid, ESP undefined. The existing l.S `start:` (jmp over header → cli/cld → save %ebx → mask PIC → …) already assumes exactly this state — only the header block changes. Info structure: `u32 total_size; u32 reserved;` then 8-byte-aligned tags `u32 type; u32 size;` — walk to type 0.

### mb2 memory-map tag → pagealloc (the /memory analog)
```rust
// Tag type 6: { u32 type=6, u32 size, u32 entry_size, u32 entry_version,
//   entries[] = { u64 base_addr, u64 length, u32 type, u32 reserved } }
// type: 1 available, 2 reserved, 3 ACPI reclaimable, 4 ACPI NVS, 5 bad
// (spec §3.9.4 — parse with bounds checks like port/fdt.rs; loud on unknown)
for e in entries {
    if e.entry_type == 1 { pagealloc_free_range(e.base, e.len); }
    if e.entry_type == 3 { /* RSDP/XSDT/MCFG may live here — keep mapped */ }
}
```
Note tag 14/15 carry ACPI RSDP old/new copies directly — the preferred RSDP source (loader-mediated, per D-02's theme); the EBDA/0xE0000 scan (9k devacpi.c:1237-1252 precedent: BDA 0x40:0x0E word → first KB of EBDA, then 0xE0000-0xFFFFF for `"RSD PTR "`) is the fallback/cross-check.

### ACPI static-table mini-parser surface (one parser, three consumers)
```text
RSDP (36-byte rev>=2 checksum; XSDT phys at offset 24) → map → XSDT
XSDT: { Sdthdr(36B), u64 ptrs[] } → find by signature:
  "MCFG" → entries[16B]: { u64 base; u16 seg; u8 start_bus; u8 end_bus; u32 rsvd }
  "APIC" (MADT) → entries { u8 type; u8 len }: type 1 IOAPIC {id, _, u32 addr, u32 gsib},
                   type 2 IntSrcOverride {bus=0, source, u32 gsi, u16 flags}
  "HPET" → { u32 blockid... u8 addr[8:16]=0xFED00000-class base, ... }
Every table: verify signature + length + checksum before use (9k sdtchecksum idiom).
```
MCFG entry consumption: program MCH PCIEXBAR (config 0x60) = `base | 0b00<<1 /*256M*/ | 1 /*EN*/`, then ECAM addr = `base + (bus<<20 | dev<<15 | fn<<12)` [VERIFIED: qemu q35.c LENGTH/MASK constants; osdev MCFG/MADT layouts].

### LAPIC minimum bring-up sequence (xAPIC, classic — x2APIC deferred)
```text
1. MADT type-0 → LAPIC phys addr (q35: 0xFEE00000); map device.
2. SPIV/SVR: enable unit + spurious vector (e.g. 0xFF), EOI-broadcast mode default.
3. TPR=0; LINT0/LINT1 masked (no PIC — D-04); error LVT set.
4. Timer LVT: vector=T; mode = TSC-deadline (bit 18) if CPUID.1.ECX[24], else periodic;
   periodic: DCR=divide-16-class, ICR=initial count (freq-calibrated).
   TSC-deadline: arm = wrmsr(0x832, rdtsc() + delta) — one-shot per deadline,
   re-armed in tick() (the NET_DEADLINE/poll_at one-shot idiom).
5. IOAPIC RTE (per device GSI): vector | delivery=fixed | level-triggered
   | unmasked — written LAST before sti (Phase-3 ordering rule).
6. IRQ context: read IRR/vector via IDT entry → device handler (ISR read-to-ack)
   → LAPIC EOI (reg 0xB0, write 0).
```
References: 9k apic.c (apicrget/apicrput via 4KiB vmap; apiceoi at apic.c:94-96) + ioapic.c (Ioregsel=0x00/Iowin=0x04-relative in 9k; qemu uses IOWIN at 0x10 — **use the ACPI/82093AA layout: IOREGSEL 0x00, IOWIN 0x10, RTEs at 0x10+2*i** [CITED: osdev IOAPIC + 9k ioapic.c:23-31 read; the discrepancy between 9k's Iowin=0x04 and the standard 0x10 is itself a pitfall — verify against MADT-reported-address hardware at bring-up]).

### The X64-03 perturbation config (lowecam precedent re-keyed)
```toml
# config_q35lowmcfg.toml — the X64-03 moved-window gate. Identical to
# config_q35.toml except the moved MCFG property; the kernel's
# PCIEXBAR-programming step follows the table (Pitfall 1).
[qemu]
machine = "q35"
accel = "kvm"
cpu = "host"
# extra args the x86_64 arm must learn to emit from data:
extra = ["-global", "q35-pcihost.MCFG=0xe0000000"]
smp = 1
memory = "1024M"
```
[VERIFIED mechanism: qemu `-global <type>.<prop>` targets `TYPE_Q35_HOST_DEVICE "q35-pcihost"` (include/hw/pci-host/q35.h:32) with property `"MCFG"` (pcie_host.h:31); the lowecam precedent config is config_virtlowecam.toml, read this session. Whether `[qemu]` gains an `extra` list or a dedicated key is planner/config-schema discretion — the requirement is that it stays data.]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| qemu `-kernel` multiboot (mb1, 0x1BADB002) — what old l.S + 9k use | GRUB2 in-guest chain for mb2 (0xE85250D6) — qemu's `hw/i386/multiboot.c` remains mb1-only (qemu issue #389 open) | long-standing, re-confirmed this session | D-02's premise holds: mb2 requires `-cdrom` + GRUB2; mb1 stays available as the 9k recorded-against path |
| MPS/`$MP` tables for INTx routing (9k mp.c; ioapic.c's rdtbus) | ACPI MADT + chipset wiring (q35 PIRQ→GSI 16+; _PRT for AML-capable OSes) | qemu never shipped MPS | 9k's mp.c path is dead on qemu — r9's D-06/MADT + documented q35 wiring is the correct re-derivation |
| 8259 PIC + IOAPIC both (SeaBIOS programs PIC-mode PIRQ routes) | APIC-only (D-04); SeaBIOS's PIC-mode route writes are harmless residue — the APIC leg (PIRQ→GSI16+) is unconditional in qemu | verified this session | Mask the PIC early (old l.S already does: `I8259A0/A1 ← 0xFF`), ignore the LPC route registers |
| LAPIC periodic timer as the default tick | TSC-deadline on KVM-class hosts (user override D-05); periodic remains the TCG fallback | verified this session | CPUID-gated mode selection is mandatory, not optional |

**Deprecated/outdated:**
- 9k `Iowin = 0x04` IOAPIC register offset (ioapic.c:26) vs the 82093AA/ACPI-era `IOWIN = 0x10` — use 0x10, verify at bring-up.
- The old xtask X86_64 arm (`-kernel r9.elf32`, elf32-i386 objcopy, `-smp 8 -m 8192`) — superseded by TOML-driven emission + `-cdrom`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `core::sync::atomic::fence(Release/Acquire)` lowers to compiler-only barriers on x86_64 (no mfence) | Pitfall 6 | Low: if it lowered stronger, behavior only gets safer; the objdump check in-plan closes it |
| A2 | GRUB2's mb2 ELF64 loader honors p_paddr (AT()-style physical load) for higher-half ELF64 kernels — matching the existing kernel.ld convention | Patterns/boot | Medium: if GRUB loaded at vaddr, the boot PML4 identity window (first 4 GiB, already in l.S) still covers it; entry point and headers align either way; verify at first boot |
| A3 | grub-mkrescue 2.14 honors `SOURCE_DATE_EPOCH` (xorriso 1.5.8 path) sufficiently for a fixed-output derivation | Pitfall 4 | Medium: fallback to explicit xorriso args / BIOS-only; double-build hash check is the in-plan gate |
| A4 | q35 HPET is present by default (base 0xFED00000, HPET table in XSDT; qemu tick 10 ns) | timer calibration | Low: calibration chain is tolerant (CPUID 0x15/0x16 → HPET → PIT); loud-by-absence print |
| A5 | KVM in-kernel APIC/IOAPIC EOI broadcast semantics match the LAPIC-EOI-write-then-remote-IRR-clear model for level-triggered INTx | Pitfall 5 | Low-Medium: the ISR-read-before-EOI order is correct under both models; a divergence would surface as the X64-02 load-run reconcile failing (that's what the gate is for) |
| A6 | Writing MSR 0x832 under TCG faults (unhandled MSR) rather than silently wrapping | Pitfall 2 | None for correctness: CPUID-gating means r9 never writes it without the bit |
| A7 | Hosted CI runners lack /dev/kvm, so the CI leg of the gates runs TCG | X64-05 / Environment | Low: gates default KVM on the dev host; CI documents the TCG fallback leg (mirrors aarch64 CI = linux leg) |
| A8 | The rx-side slirp behavior on x86_64 q35 matches aarch64 virt (ARP reply to gateway-targeted requests, pcap filter-dump identical) | X64-04 | Low: same slirp code; the 07 erratum + watch-out carry verbatim |

## Open Questions (RESOLVED — all five decided in the phase plans)

1. **Kernel-side ARP trigger exact shape (X64-04)** — What we know: `driver_ops()` is pub(crate), consumed kernel-side by devip's adapter; `tx_post(&[u8]) -> Result<bool>` posts one 60..=1514-byte frame. What's unclear: whether Phase 13 widens `driver_ops()` to pub, adds a cfg-gated kernel self-test module, or keys a tiny dev-list entry. Recommendation: the smallest seam that keeps "re-keyed, not reinvented" true — reuse the baked 60-byte frame bytes, post N times (ping-pong) + a bulk loop from boot context, reconcile via `ifstats()` counters + host pcap; leave the file-contract `dd` proof to Phase 14+. **RESOLVED → 13-05 Task 2** (driver_ops widened pub(crate) → pub, the one-line seam; recorded in-plan as the OQ-1 resolution) **+ 13-05 Task 3** (the cfg(dev_ether) netgate kernel-side trigger from boot context; kernel-side-only this phase per the prohibition).
2. **Netdev-as-data schema** — extend `[qemu]` with a netdev table consumed by a generalized emitter vs an `apply_q35_netdev` clone of the virt function. Recommendation: TOML data (D-01a wording + HNET-02 future consumption); migrate the aarch64 leg opportunistically, not in this phase (frozen-at-pin discipline). **RESOLVED → 13-01 Task 2** (netdev table + `extra` list as [qemu] data consumed by a generalized emitter; aarch64 `apply_virt_netdev` stays byte-untouched as the frozen-at-pin legacy consumer).
3. **Where the grub-ISO build lives** — r9 flake derivation (fixed-output, feeds kernel-x64 package) vs xtask side-build. Recommendation: flake (the X64-05 deliverable is the package; xtask boot-test can consume the flake-built ISO or build a dev copy via the devshell's grub-mkrescue for iteration speed) — planner decides the boundary. **RESOLVED → 13-06 Task 1** (flake owns the distributable reproducible boot-iso fixed-output derivation; the xtask gate keeps building its own dev copy for evidence freshness — the boundary is recorded in a flake comment).
4. **PCIEXBAR programming access path** — CF8/CFC one-shot (architectural ports) vs writing through the currently-live ECAM window at SeaBIOS's 0xb0000000 (a documented-but-hardcoded base). Recommendation: CF8/CFC once, framed as "host-bridge register programming", with the print `pciexbar: <old> -> <new>` making the data-following explicit; flag to user in the plan summary since D-07's "no CF8/CFC port path" wording needs this one carve-out recorded. **RESOLVED → 13-04 Task 2** (one isolated CF8/CFC helper for the single config-0x60 host-bridge write; carve-out recorded in the prohibition, the code comment, and the r9 commit message; surfaced to the user at plan review).
5. **TCG fallback CPU model** — `qemu64` lacks even x2apic/tsc-deadline (fine — CPUID-gated) but the old arm used `qemu64,pdpe1gb,xsaveopt,fsgsbase,apic,msr`; the fallback TOML cpu value is planner discretion. Recommendation: minimal `qemu64` + explicit `apic` (LAPIC always present in qemu x86 anyway) and document in the companion README. **RESOLVED → minimal `qemu64`** on every TCG leg (`--accel tcg --cpu qemu64`: 13-03 Task 3, 13-05 Task 3, 13-06 Task 2 — the CPUID-gated periodic timer makes tsc-deadline absence moot), documented in build/x86_64/README.md by 13-06 Task 2.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-x86_64 | all gates (q35/KVM/TCG) | ✓ | 11.1.2 (matches project pin) | — |
| /dev/kvm (rw) | KVM-native gate default (X64-05) | ✓ | host device, crw-rw-rw- | TCG fallback documented |
| grub-mkrescue | mb2 ISO build (D-02) | ✓ via `nix shell nixpkgs#grub2` (NOT on PATH) | 2.14 | flake devshell/package input |
| xorriso | ISO writer under grub-mkrescue | ✓ via `nixpkgs#xorriso` (NOT on PATH) | 1.5.8.pl02 | flake devshell/package input |
| nix | flake packages (X64-05) | ✓ | 2.34.9 | — |
| rust toolchain (r9 devshell) | kernel build | ✓ (flake devshell via rust-overlay; toolchain nightly-2026-01-30, target `x86_64-unknown-none` declared) | per rust-toolchain.toml | — |
| r9 flake evaluates | kernel-virt template reuse | ✓ (`nix eval .#packages.x86_64-linux.kernel-virt.pname` → "r9-kernel-virt") | — | — |
| cargo on bare PATH | — | ✗ (devshell-only) | — | intended: all builds via r9 devshell/xtask |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** grub-mkrescue/xorriso absent from PATH — always consumed via nixpkgs (devshell or derivation), never global installs.

**Note (A7):** hosted CI runners are assumed KVM-less; the r9 CI leg runs the TCG profile while the KVM-native default is exercised on this host. Also unverified this session: whether `cargo build -p x86_64` currently compiles (old-gen locals + months of port churn) — Wave-0 check (Pitfall 9).

## Security Domain

> `security_enforcement` is absent from `.planning/config.json` → treated as enabled; included.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Phase 18 (AUTH) |
| V3 Session Management | no | Phase 19 |
| V4 Access Control | no | no user processes (exec is Phase 14) |
| V5 Input Validation | **yes** | All firmware/loader data is untrusted input at the kernel boundary: mb2 info structure (bounds-check total_size, tag sizes, entry_size alignment before any slice), ACPI tables (signature + length + checksum before use — 9k `sdtchecksum` idiom), MADT entries (id/gsi bounds), MCFG (base within mapped window, bus range), PCI config reads (vendor 0xFFFF = absent, ECAM offset bounds — the aarch64 "bounded by the mapped span (T-07-04)" discipline), virtio used-ring values (already validated: id < qsize and even; 12 ≤ len ≤ capacity — port/virtio.rs V5 boundary) |
| V6 Cryptography | no | AUTH-02 re-keys /dev/random in Phase 18 |
| V8 Exception Handling | **yes (kernel analog)** | loud-by-absence prints, bounded spin (RESET_SPINS precedent), panic-dump-then-halt (trap.rs), negative panic scan in every gate |

### Known Threat Patterns for the x86_64 bring-up stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious/corrupt firmware tables (mb2, RSDP/MCFG/MADT) | Tampering | checksum + signature + strict bounds before any pointer derived from table data; loud failure not a walk off the end |
| Hostile device DMA (virtio rings) | Tampering/Elevation | `RingMem` mapped Normal at KZERO+PA (never device mappings); used-ring id/len validation (in-repo, Phase 7 V5); unsafe concentrated at the MMIO/DMA seam per UNSAFE-AUDIT.md |
| Interrupt-driven DoS (spurious/storm) | DoS | unmask-last ordering; ISR read-to-ack; reconcile accounting in the load gate (X64-02 exists to prove this) |
| ISR read-before-EOI inversion | DoS | ordering discipline (Pitfall 5) |

## Sources

### Primary (HIGH confidence)
- **qemu master source (fetched + read this session)**: `hw/pci-host/q35.c` (q35_host_props `PCIE_HOST_MCFG_BASE` default `MCH_HOST_BRIDGE_PCIEXBAR_DEFAULT`; mch_update_pciexbar; mch_reset), `include/hw/pci-host/q35.h` (`"q35-pcihost"`, PCIEXBAR 0x60/DEFAULT 0xb0000000/ADMSK/LENGTH bits), `include/hw/pci/pcie_host.h` (`PCIE_HOST_MCFG_BASE "MCFG"`), `hw/i386/acpi-build.c` (acpi_get_mcfg → build_mcfg), `hw/isa/lpc_ich9.c` (ich9_pirq_to_gsi = pirq+16 unconditional; irr table; ich9_lpc_set_irq), `include/hw/southbridge/ich9.h` (PIRQ regs 0x60-0x63/0x68-0x6B, RCBA 0xF0, D25IR-D31IR 0x3140-0x3150), `target/i386/cpu.c` (TCG_EXT_FEATURES missing-list incl. TSC_DEADLINE; qemu64 model), `target/i386/kvm/kvm.c` (KVM_CAP_TSC_DEADLINE_TIMER gate; MSR_IA32_TSCDEADLINE), `hw/intc/apic.c` (TCG APIC timer = periodic/one-shot only), `include/hw/timer/hpet.h` (HPET_BASE 0xfed00000, 10ns tick)
- **SeaBIOS master source (fetched this session)**: `src/fw/dev-q35.h` (PCIEXBAR_ADDR 0xb0000000 hardcode), `src/hw/pci.c:482-488` (PCIEXBAR enable write), `src/fw/pciinit.c` (LPC PIRQ route programming, pci_irqs, ELCR)
- **Multiboot2 Specification 2.0** (gnu.org manual, full text fetched this session): header/tag/entry/info-structure/mmap tag/ACPI tags 14-15
- **r9 repo (all read this session)**: x86_64 crate (main.rs/l.S/dat.rs/cpu.rs/pio.rs/uart16550.rs/trap.rs/Cargo.toml/lib/{kernel.ld,config_default.toml}), lib/x86_64-unknown-none-elf.json, rust-toolchain.toml, Cargo.toml workspace lints, aarch64 platform tree (mod.rs, virt/{mod.rs,pci.rs,timer.rs,devcons.rs,gic/}), aarch64/src/{main.rs,io.rs}, port/src/{virtio.rs,devether.rs,devcons.rs,dat.rs,lib.rs}, xtask/src/{main.rs,config.rs}, flake.nix, HowItWorks.md, aarch64/lib/config_{virt,virtnet,virtlowecam}.toml
- **Harvey 9k (read this session)**: sys/src/9k/k10/{main.c refs,apic.c,ioapic.c,mpacpi.c,devacpi.c (rsdsearch),hpet.c,acpi.h}, flake.nix r9 input
- **07-CONTEXT.md** (60-byte ARP erratum, slirp-gateway watch-out, D-05/D-07/D-08 gate machinery — quoted verbatim)

### Secondary (MEDIUM confidence)
- osdev wiki MCFG/MADT/IOAPIC pages (table layouts — consistent with ACPI 6.x structures and 9k's acpi.h)
- Linux kernel memory-barrier docs + virtio_ring barrier discussions (x86 compiler-barrier sufficiency)
- reproducible-builds.org/system-images + grub-devel bug 787795 + Debian 831379 (ISO reproducibility mechanics)

### Tertiary (LOW confidence)
- qemu issue #389 ("-kernel multiboot2" request — existence confirms mb1-only, found via search snippet)
- BSDCan Baldwin paper + mcastelino gist (PIRQ→GSI 16-19 APIC-mode picture — superseded by the primary qemu source reads)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — everything is existing in-tree machinery or verified host tooling; zero new crates
- Architecture: HIGH — structural template (aarch64 platform tree) read in full; port seams enumerated from source
- Pitfalls: HIGH — the three novel hazards (SeaBIOS PCIEXBAR vs MCFG, TCG TSC-deadline absence, q35 INTx wiring) are verified against qemu/SeaBIOS source, not folklore
- Gate/payload mechanics: HIGH — xtask BootTestStep/pcap/apply_virt_netdev read; Phase-7 erratum quoted from its CONTEXT
- Reproducible-ISO specifics: MEDIUM — approach well-sourced; exact grub 2.14 flag behavior to be proven by the double-build hash check in-plan

**Research date:** 2026-11-06
**Valid until:** 2027-01-06 (stable domain — qemu/GRUB/ACPI/mb2 specs move slowly; r9 tree is a moving target within the project's own control)
