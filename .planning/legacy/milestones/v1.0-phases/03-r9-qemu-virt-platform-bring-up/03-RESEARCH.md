# Phase 3: r9 qemu-virt Platform Bring-up - Research

**Researched:** 2026-11-01
**Domain:** aarch64 bare-metal kernel bring-up (qemu `-M virt`, GICv2, PL011, ARM generic timer) + nix flake tooling
**Confidence:** HIGH (core boot contract and all four SC mechanisms empirically verified on the target qemu 11.1.2 binary on this machine)

## Summary

Phase 3 adds a qemu `-M virt` platform to r9's aarch64 crate (sibling repo `../r9`, fork `requaos/r9`, branch `main`, HEAD `33a9273`) alongside raspi3b/4b, plus an r9-local `flake.nix`. Every risky mechanism was verified empirically this session against the exact SC#2 machine (`qemu-system-aarch64 11.1.2`, `-M virt,gic-version=2 -cpu cortex-a72 -smp 1`) using hand-built bare-metal probes: ELF `-kernel` entry at EL1, GICv2 init + timer IRQ (INTID 30) firing on schedule, PL011 RX IRQ (INTID 33) with typed-char echo, and the DTB delivery contract.

Three findings change what CONTEXT.md assumed, all verified with running probes or qemu source + monitor inspection:

1. **ELF `-kernel` + qemu's generated DTB = no DTB in guest RAM at all, and x0 = 0.** qemu treats ELF kernels as "not Linux" (`hw/arm/boot.c` v11.1.2): the aarch64 bootloader stub that sets x0=DTB is only written for `is_linux` (flat/uImage) boots, and the generated virt DTB blob is exactly 1 MiB (`totalsize=0x100000`) while the ELF path caps DTB placement below the kernel image (`dtb_limit = image_low_addr` = 0x40080000, a 512 KiB gap) — `arm_load_dtb` silently returns without loading it. **Fix verified end-to-end:** pass `-dtb` with a trimmed DTB (7785 bytes after a `dtc` round-trip) — it lands at 0x40000000 and the guest reads the FDT magic there. r9's virt platform must source the DTB from the fixed PA 0x40000000 when x0 is null (raspi keeps x0).
2. **HVF rejects GICv2 on this machine** (`qemu-system-aarch64: HVF does not support GICv2 emulation` — verified, qemu 11.1.2). HVF's default virt machine exposes GICv3 (`arm,gic-v3`, GICR at 0x080a0000). D-06's darwin HVF arm is therefore infeasible for a GICv2-only driver: **darwin-aarch64 runs TCG this phase** (Phase 1/2 TCG-acceptance precedent covers it). A GICv2 probe under HVF confirmed the failure shape: boot + early console work, GICC writes are dropped, no IRQs ever fire.
3. **`base_physaddr() = 0` in both `kmem.rs` and `pre_mmu/vminit.rs` becomes actively harmful on virt.** With the kernel at PA 0x40080000, `boottext_physrange()` = 0..~0x40090000: vminit would map >1 GiB as Normal-cacheable kernel text — including the GIC (0x08000000) and PL011 (0x09000000) MMIO windows — a speculative-access hazard, and the 1 GiB identity-map block (`physicalpt3.entries[0]`, PA 0–1 GiB) does not cover the virt kernel at all.

The full GICv2/timer/PL011 register sequence below is not from memory — it is extracted from a probe that demonstrably produced periodic INTID-30 ticks and interrupt-driven char echo on the milestone's exact qemu invocation.

**Primary recommendation:** virt platform = ELF `-kernel` (existing linker script unchanged in shape) + committed trimmed `virt-gicv2.dtb` passed via `-dtb` + DTB-at-fixed-0x40000000 fallback when x0==0; platform tree per D-01; GICv2 + generic timer + PL011 drivers in `aarch64/src/platform/virt/` using the verified register sequence; `cargo xtask boot-test` as a zero-new-dependency unix-socket harness; flake via oxalica/rust-overlay honoring `rust-toolchain.toml`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Adopt the **riscv64-style platform tree** in the aarch64 crate: `aarch64/src/platform/{mod.rs,virt/,raspi3b/,raspi4b/}`. `mailbox.rs`, `uartmini.rs`, GPIO and bcm DTB handling move under the raspi modules; the new `platform/virt/` module gets PL011/GIC/timer. The existing `[config] platform` TOML key drives `--cfg platform="virt"` (already implemented in `xtask/src/config.rs`).
- **D-02:** **Platform-gated early-boot console**: virt gets an early PL011 debug path at `0x09000000` (no init needed — qemu pre-configures it) under `platform/virt`; raspi keeps its miniuart path in `pre_mmu/util.rs`.
- **D-03:** **Gate `mailbox::init` + `print_board_info` to raspi platforms.** On virt, `main9` prints DTB-derived machine info instead (memory size, CPU, discovered GIC/PL011 addresses).
- **D-04:** **DTB-driven device discovery on virt** via the existing `port/fdt.rs` (`find_compatible("arm,pl011")`, `"arm,gic-400"`, `"arm,armv8-timer"` + translated reg iter). No hardcoded virt constant tables. ⚠️ **Research correction:** the actual qemu gic-version=2 compatible string is **`arm,cortex-a15-gic`**, not `arm,gic-400` (verified from dumped DTB) — match both.
- **D-05:** **ELF `-kernel` for virt; flat+gzip stays raspi-only.** The objcopy/gzip dist step is skipped for virt. ⚠️ **Research addendum:** ELF boots get **x0=0 and no auto-loaded DTB** (qemu 11.1.2) — pair with `-dtb` + fixed DTB PA (verified).
- **D-06:** **Native acceleration with TCG fallback**: `-accel hvf -cpu host` on darwin-aarch64, `-accel kvm -cpu host` on linux-aarch64, TCG + `-cpu cortex-a72` elsewhere; flag/env-overridable. ⚠️ **Research correction:** HVF+GICv2 is rejected by qemu 11.1.2 (verified) — on darwin the fallback arm is the operative one this phase; keep the accel key so the HVF path lights up only with a future GICv3 driver.
- **D-07:** **Extend the `[qemu]` TOML section** with `cpu`/`smp`/`memory`/`accel` keys alongside the existing `machine`/`dtb` — `config_virt.toml` declares `machine = "virt,gic-version=2"` etc.
- **D-08:** Phase 3 ships **devShell + apps** (`build`, `boot-virt` wrapping `cargo xtask`). `packages.kernel-*` outputs are **deferred to Phase 5**.
- **D-09:** **Honor `rust-toolchain.toml` via an overlay** (oxalica/rust-overlay or fenix — agent's pick). The pinned nightly-2026-01-30 + components (rustfmt, rust-src, clippy, llvm-tools) + three targets stay the single source of truth.
- **D-10:** **Manual host verification this phase**: the SC#1 command pair is run by hand on darwin-aarch64 and a linux host. r9's existing GH Actions stay as-is.
- **D-11:** Acceptance harness is a **Rust `cargo xtask boot-test` subcommand** — spawns qemu, captures the serial log, asserts banner / periodic timer ticks / typed-char echo / panic dump; human checkpoint approves the captured log at the gate. A rexpect-style crate dep is accepted for single-toolchain purity (research recommends a zero-dep socket harness instead — see Boot-test harness section).
- **D-12:** The SC#5 deliberate panic is a **boot-time panic config** — a `--config panic` TOML variant that panics at end of `main9`, printing registers/stack and halting cleanly.
- **D-13:** **Harness lives in r9; canonical evidence lives in Harvey `.planning/phases/03-*`/`** — the phase dir records the r9 commit SHA + evidence paths in r9.

### Carried forward (locked in roadmap/prior phases)

- qemu invocation shape: `-M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic`, ELF `-kernel`, DTB at `0x40000000`, GICv2 @ `0x08000000`, PL011 @ `0x09000000`, ARM generic timer (R9-01, SC#2–5). GICv2 (not v3) is the roadmap's explicit choice.
- raspi3b/4b keep working — virt lands *alongside*.
- Proof standard: **boot/execution-test with captured serial evidence + human checkpoint**.
- `aarch64` naming everywhere; r9 code follows r9's own `rustfmt.toml` (edition 2024, Unix newlines, use_small_heuristics=Max).
- minimega is amd64/KVM-only — aarch64 boots use plain qemu.
- r9 commits land directly on `main` of `requaos/r9`.
- Harvey user-side ABI frozen (`aarch64/ABI.md`, `ureg.h`) — **no userspace-exec work this phase** (planner decides `test_sysexit`'s fate in virt main9).

### the agent's Discretion

- Timer tick rate and log verbosity (SC#3 only requires visible periodic ticks).
- GICv2 + generic-timer driver file placement/style inside `platform/virt/` (DTB-discovered per D-04).
- Overlay choice (rust-overlay vs fenix) and exact devshell package list.
- rexpect-style crate choice for `boot-test`.
- Panic config plumbing (cargo feature vs `[config]` cfg flag) and where `panic!()` dumps registers.
- Early-PL011 print mechanics and the virt identity-map/load-address adjustment in `kernel.ld` + `pre_mmu/vminit.rs` (kernel loads at PA `0x40080000`).
- What virt `main9` looks like beyond the SCs — keep minimal.

### Deferred Ideas (OUT OF SCOPE)

virtio-9p host mount; SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM; Ctrl-P interactive panic; r9 flake `packages.*`; r9 CI-green-on-both-hosts (INT-02); upstream-r9 PR flow; userspace-exec/EL0/`svc`/root-fs (Phase 4); raspi5.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| R9-01 | r9 gains a qemu `-M virt` aarch64 platform: kernel entry contract (ELF `-kernel`, DTB at 0x40000000, `-cpu cortex-a72`), memory map, GICv2, PL011 console, generic timer — proven by kernel log output in qemu | Boot contract verified empirically (entry EL1, ELF high-half VMA works, x0=0 fix via `-dtb`+fixed PA); GICv2/timer/PL011 register sequences proven by probe; DTB nodes/compatable strings dumped from the real machine config |
| R9-05 | r9 repo carries its own `flake.nix` (pinned rust toolchain incl. llvm-tools, qemu, cargo xtask) — devshell + apps mirroring Harvey's conventions | Harvey flake.nix read as the convention template; rust-overlay `fromRustupToolchainFile` confirmed as the D-09 mechanism; qemu+dtc availability in nixpkgs verified on this machine |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Early-boot debug prints | aarch64 crate `pre_mmu` (physical MMIO, pre-MMU) | — | Must run before VM/init; per-platform (D-02) |
| DTB acquisition | `l.S`/entry path (x0 on raspi; fixed PA on virt) | `port/fdt.rs` (parser) | qemu ELF contract forces a per-platform DTB source |
| Identity + higher-half map | `pre_mmu/vminit.rs` + `kernel.ld` per-platform values | `vm.rs` | Load address 0x40080000 (virt) vs 0x80000 (raspi) |
| GICv2 driver | `aarch64/src/platform/virt/gic.rs` (MMIO via `map_device_register`) | — | virt-only device this phase; raspi untouched |
| Generic timer | `aarch64/src/platform/virt/timer.rs` (sysregs via aarch64-cpu) | — | CNTP PPI 14 → INTID 30 |
| Console output | `port/devcons.rs` Console lock + `println!` (unchanged) | `platform/virt/devcons.rs` sets a PL011 `Uart` | Output-only trait stays; mirrors riscv64 `platform/virt/devcons.rs` |
| Console input (SC#4) | `platform/virt` PL011 RX IRQ handler → echo/log | future console-input path (Phase 4+) | `Uart` trait is `putb`-only today; handler must bypass the Console lock (see Pitfall P8) |
| IRQ dispatch | `trap.rs` (dispatch on `interrupt_type`, return-for-IRQ) | `trap.S` (vector table, eret path already exists) | `trap()` spinning unconditionally today is the wedge to fix |
| Build/run orchestration | `xtask` (BuildStep/DistStep/QemuStep/new BootTestStep) | config TOML per platform | xtask stays generic; platforms are data (D-07) |
| Acceptance evidence | `xtask boot-test` (r9) | copies + SHA record in Harvey `.planning/phases/03-*/` (D-13) | milestone evidence consolidation |
| Dev environment | `../r9/flake.nix` (devShell + apps) | — | mirrors Harvey flake shape |

## Standard Stack

### Core (all already in r9 — no new kernel dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `aarch64-cpu` | 11.2.0 (Cargo.lock; docs.rs lists `CNTFRQ_EL0`, `CNTP_TVAL_EL0`, `CNTP_CTL_EL0`, `CNTPCT_EL0`, `VBAR_EL1`, `DAIF`) | System-register access for timer/VBAR | already r9's dep; tock-registers idiom [CITED: docs.rs/aarch64-cpu/11.2.0] |
| `port::fdt` | in-repo | DTB parse: `find_compatible`, `property_translated_reg_iter`, `property_value_as_u32_iter`, `find_device_type("memory")` | D-04 machinery; verified present [VERIFIED: ../r9/port/src/fdt.rs:389,300,170] |
| `port::devcons` | in-repo | `Console`/`Uart` output path | unchanged [VERIFIED: ../r9/port/src/devcons.rs:20-51] |
| `deviceutil::map_device_register` | in-repo | MMIO mapping for GICD/GICC/PL011 | verified [VERIFIED: ../r9/aarch64/src/deviceutil.rs:9-34] |

### Toolchain / host-side

| Package | Version | Purpose | Notes |
|---------|---------|---------|-------|
| qemu | 11.1.2 (nixpkgs, verified running here) | `qemu-system-aarch64` | all findings verified against this exact version |
| dtc | 1.8.1 (nixpkgs, verified in store) | regenerate/trim `virt-gicv2.dtb` | devshell only |
| oxalica/rust-overlay | master (pin in flake.lock) | `rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` | D-09 [CITED: github.com/oxalica/rust-overlay cheat sheet] |
| rust nightly | 2026-01-30 (rust-toolchain.toml) | kernel toolchain | single source of truth; overlay must resolve dated nightly |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rust-overlay | fenix (`fromToolchainFile`) | Both honor rust-toolchain.toml [CITED: github.com/nix-community/fenix]; rust-overlay's rustup-semantics API is the more standard, better-documented path — pick rust-overlay |
| Zero-dep socket harness | `expectrl` 0.7 / `rexpect` 0.6 | Both legitimate (see audit). PTY semantics unneeded for qemu `-serial unix:`; std-only keeps xtask deps at clap/serde/toml/target-lexicon. Recommend zero-dep; crates remain the accepted fallback (D-11) |
| `-dtb` + committed trimmed DTB | dumpdtb at every boot | Extra qemu spawn per boot + a trim step (needs dtc or an FDT rewriter); committed file matches the existing `aarch64/lib/bcm*.dtb` convention and qemu still injects `/memory` + `/chosen` at load time |

**Installation:** none for the kernel (deps unchanged). Devshell via `nix develop ../r9` once the flake lands.

## Package Legitimacy Audit

No new runtime/kernel packages. The only candidate new crate deps are for the optional expect-style harness (D-11), checked via the legitimacy seam 2026-11-01:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| rexpect | crates.io | 9.3 yrs (2017-07) | 34,324/wk | github.com/rust-cli/rexpect | OK | Approved fallback only (not recommended — zero-dep preferred) |
| expectrl | crates.io | 5.3 yrs (2021-07) | 32,087/wk | github.com/zhiburt/expectrl | OK | Approved fallback only (not recommended — zero-dep preferred) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*Both crate names came from training knowledge; legitimacy was established via the GSD package-legitimacy seam (crates.io registry + repo signals). They enter the plan only as the documented D-11 fallback. Nix-side packages (`pkgs.qemu`, `pkgs.dtc`, rust-overlay) are nixpkgs/github pinned by flake.lock — outside the crates.io gate.*

## Architecture Patterns

### Verified qemu virt (-M virt,gic-version=2) contract [VERIFIED: qemu 11.1.2 empirical on this machine]

```text
ELF -kernel (e_entry = KZERO+0x40080000, p_paddr = 0x40080000 via AT())
        │
        ▼  qemu loads segments at p_paddr, enters at the physical entry, EL1
        │  (measured CurrentEL=1; l.S's EL3→EL2→EL1 dispatch covers others)
        │  x0 = 0  ←── NOT the DTB pointer for ELF kernels (boot.c: is_linux=0,
        │              bootloader stub not written)
        ▼
DTB: NOT auto-loaded for ELF (generated blob is 1 MiB > 512 KiB gap limit,
     arm_load_dtb silently skips). With -dtb <trimmed.dtb> qemu places it at
     0x40000000 and still injects /memory + /chosen. → l.S/main9: if x0==0
     use platform-fixed 0x40000000.
        ▼
DTB (gicv2 machine) — dumped from qemu 11.1.2 on this host:
  intc@8000000   compatible "arm,cortex-a15-gic"   (NOT "arm,gic-400")
                 reg = GICD 0x08000000/0x10000, GICC 0x08010000/0x10000
                 #interrupt-cells = 3
  pl011@9000000  compatible "arm,pl011", "arm,primecell"
                 reg = 0x09000000/0x1000; interrupts = <0x00 0x01 0x04>
                 → SPI 1 → INTID 32+1 = 33, level-high
                 clocks → apb-pclk fixed-clock = 24 MHz (0x16e3600)
  timer          compatible "arm,armv8-timer", "arm,armv7-timer"
                 interrupts = <0x01 0x0d 0x104>  PPI 13 → INTID 29 (sec-phys)
                              <0x01 0x0e 0x104>  PPI 14 → INTID 30 (CNTP ← use)
                              <0x01 0x0b 0x104>  PPI 11 → INTID 27 (virt)
                              <0x01 0x0a 0x104>  PPI 10 → INTID 26 (hyp)
                 (DTB int cells: <type, num, flags>; INTID = 16+PPI or 32+SPI)
  memory@40000000 reg = 0x40000000 / size from -m (1 GiB at -m 1024)
  (also: flash@0 0x0–0x4000000, platform-bus@c000000, v2m MSI frame — unused)
```

CNTFRQ_EL0: **62.5 MHz under TCG, 24 MHz under HVF** (both measured) — always read the register, never hardcode.

### Recommended additions to r9 (structure)

```text
../r9/
├── flake.nix                       # NEW (D-08/D-09) — devShell + apps.{build,boot-virt}
├── aarch64/
│   ├── lib/
│   │   ├── config_default.toml     # gains [config] platform = "raspi3b"  (REQUIRED — currently absent)
│   │   ├── config_raspi4b.toml     # gains [config] platform = "raspi4b"
│   │   ├── config_virt.toml        # NEW: platform="virt", [qemu] machine/cpu/smp/memory/accel/dtb
│   │   ├── config_virtpanic.toml   # NEW (D-12): virt + dev=["panic"]
│   │   ├── virt-gicv2.dts          # NEW: qemu-dumped, cleaned, reviewable
│   │   └── virt-gicv2.dtb          # NEW: dtc-compiled (matches bcm*.dts/.dtb convention)
│   └── src/
│       ├── platform/
│       │   ├── mod.rs              # cfg-gated re-exports (riscv64 precedent)
│       │   ├── virt/{mod.rs, gic.rs, timer.rs, pl011.rs, devcons.rs}
│       │   ├── raspi3b/…           # mailbox.rs, uartmini.rs, devcons.rs move here
│       │   └── raspi4b/…
│       ├── trap.rs                 # dispatch + return-for-IRQ (today: always spins)
│       ├── pre_mmu/util.rs         # per-platform early uart (virt = PL011 no-init)
│       └── kmem.rs + pre_mmu/vminit.rs  # base_physaddr() fix (Pitfall P3)
└── xtask/src/
    ├── config.rs                   # [qemu] gains cpu/smp/memory/accel (D-07)
    └── main.rs                     # DistStep platform-aware (D-05); QemuStep serial/accel;
                                    # new boot-test subcommand (D-11)
```

### Pattern 1: GICv2 (GIC-400) init + EOI — verified probe sequence

**What:** the exact MMIO sequence proven on qemu 11.1.2 virt gicv2.
**When:** virt platform init, before unmasking IRQs.
**Register map** (ARM GICv2 arch spec ARM IHI 0048B; offsets verified live): GICD_CTLR 0x000, ISENABLERn 0x100+, IPRIORITYRn 0x400+, ITARGETSRn 0x800+, ICFGRn 0xC00+; GICC_CTLR 0x000, PMR 0x004, IAR 0x00C, EOIR 0x010.

```asm
@ --- verified probe (TCG, -M virt,gic-version=2, cortex-a72) ---
@ GICD = 0x08000000 (x24), GICC = 0x08010000 (x25)
movz w0, #0x00A0, lsl #16 ; str w0, [x24, #0x41C]  @ IPRIORITYR7: INTID 30 prio 0xA0
movz w0, #0x0000, lsl #16 ; movk w0, #0xA000
                          str w0, [x24, #0x420]    @ IPRIORITYR8: INTID 33 prio 0xA0 (byte1)
movz w0, #0x0100        ; str w0, [x24, #0x820]    @ ITARGETSR8: INTID 33 → cpu0 (SPIs NEED this; PPIs don't)
movz w0, #0x4000, lsl #16 ; str w0, [x24, #0x100]  @ ISENABLER0 bit 30 (timer)
movz w0, #0x2           ; str w0, [x24, #0x104]    @ ISENABLER1 bit 1 (INTID 33, PL011)
movz w0, #1             ; str w0, [x24]            @ GICD_CTLR enable
movz w0, #0xFF          ; str w0, [x25, #4]        @ GICC_PMR accept all
movz w0, #1             ; str w0, [x25]            @ GICC_CTLR enable
@ handler (VBAR_EL1+0x280 = IRQ EL1h):
@   ldr w0, [x25, #0x0C]   @ IAR → intid
@   ...handle...
@   str wIAR, [x25, #0x10] @ EOIR with the IAR value
@   eret
```

DTB driver shape (Rust): parse `arm,cortex-a15-gic`/`arm,gic-400` → map GICD+GICC via `map_device_register` (their DTB reg sizes are 0x10000 each; the driver only touches the first 4K of each) → `enable(intid, prio)` API doing ISENABLER/IPRIORITYR/ITARGETSR math (SPIs get ITARGETSR, PPIs skip it) → `ack()`/`eoi()`.

### Pattern 2: ARM generic timer — verified

```asm
mrs  x0, cntfrq_el0              @ 62.5 MHz TCG / 24 MHz HVF — read, never hardcode
@ arm: CNTP_TVAL_EL0 = freq / ticks_per_sec  (probe used 6250000 → ~10 Hz)
msr  cntp_tval_el0, x0
movz x0, #1
msr  cntp_ctl_el0, x0            @ ENABLE=1, IMASK=0
@ per tick: re-arm TVAL (TVAL is a countdown), EOI in GIC
```

aarch64-cpu 11.2 provides `CNTFRQ_EL0.get()`, `CNTP_TVAL_EL0.set(..)`, `CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET)` [CITED: docs.rs/aarch64-cpu/11.2.0]. Tick rate is agent's discretion — recommend 1 Hz print-every-tick (readable at TCG speed, SC#3-satisfying).

### Pattern 3: PL011 polled-out + IRQ-RX — verified

- qemu's PL011 transmits DR writes even with UARTEN clear (logs a guest-error once); reset state CR=0x300 (TXE|RXE). So **early print needs zero init** (D-02 confirmed empirically) [VERIFIED: qemu 11.1.2 hw/char/pl011.c + probe].
- r9's `putb` polls FR bit5 (TXFF) — safe against qemu's model [VERIFIED: ../r9/aarch64/src/uartpl011.rs:126-131].
- RX IRQ path (verified): `ICR = 0x7ff`; `LCRH = 0x70` (8-bit + FEN → 16-deep FIFO); `IMSC = 0x50` (RXIM|RTIM); `CR = 0x301` (UARTEN|TXE|RXE). Handler: `while (FR & 0x10) == 0 { c = DR; echo }`, then `ICR = 0x50`. qemu raises INT_RX on **every** received char (read_trigger=1) [VERIFIED: pl011.c + probe — typed "hi" produced INTID 33 and echo].
- Baud: DTB uartclk = 24 MHz on virt; qemu ignores divisors. If the driver writes IBRD/FBRD for hardware correctness: IBRD=13, FBRD=1 for 115200 [ASSUMED — real-hardware relevance only].

### Pattern 4: trap dispatch (minimal restructure of existing code)

`trap.S` already saves the full `TrapFrame` (x0–x30, ESR/ELR/FAR, interrupt_type), calls `trap_unsafe`, restores, `eret` — **the return path exists; only `trap()` never returns** [VERIFIED: ../r9/aarch64/src/trap.S:51-115, trap.rs:109-122]. Change:

```rust
fn trap(frame: &mut TrapFrame) {
    match frame.interrupt_type {
        t if t == IRQ_EL1H || t == IRQ_EL1T => {
            let intid = gic::ack();           // GICC_IAR
            match intid {
                30 => platform::timer::tick(),          // re-arm + print/log
                33 => platform::uart::rx_irq_echo(),    // drain+echo (lock-free path!)
                1023 => {}                              // spurious — no EOI
                _ => println!("unhandled IRQ {intid}"),
            }
            gic::eoi(intid);
            return;                          // ← eret back to the wfi loop
        }
        _ => { println!("{:#?}", frame); println!("Unhandled interrupt"); loop { spin } }
    }
}
```

(vector offset constants: IRQ_EL1h is VBAR+0x280; r9's `IRQ_INVALID_EL1h`=5 — reuse the existing `.equ` values.)

### Pattern 5: virt `main9` shape (D-03; keep minimal)

trap::init (VBAR only) → DTB parse (x0 if non-null else fixed 0x40000000) → pagealloc init → devcons init (virt PL011 from DTB) → print DTB-derived machine info (`/memory` size, MIDR, GIC/PL011 discovered addresses) → VM/tables → gic init → timer arm → uart RX arm → unmask IRQs → banner + `wfi` loop printing ticks. `test_sysexit` and `print_board_info`/mailbox stay on raspi configs (userspace-exec is Phase 4).

### Boot-test harness (D-11) — zero-dep socket design

`cargo xtask boot-test --arch aarch64 --config virt`:
1. BuildStep; spawn qemu: `-display none -chardev socket,id=cons,path=<tmp>.sock,server=on,wait=on -serial chardev:cons -kernel <ELF> -dtb aarch64/lib/virt-gicv2.dtb` (+ config's machine/cpu/smp/memory/accel).
2. Connect `std::os::unix::net::UnixStream` — **capture attaches before the guest starts** (wait=on) — the Phase 1 "capture-before-start" lesson for free.
3. Read with generous TCG timeouts; assert in order: banner (`r9 from the Internet`), ≥3 periodic tick lines; `write_all(b"h")` → assert IRQ-echo marker; then `--config virtpanic` run: assert panic/register dump + halt marker. `Child::kill`.
4. Write the captured log to the evidence dir + print summary; exit status is the gate.

Verified en route: socket chardev drops pre-connect data unless `wait=on` (that's why wait=on matters); `-nographic` stdio path also works for interactive `cargo xtask qemu`.

### flake.nix (D-08/D-09) — mirrors Harvey's shape

```nix
{
  description = "r9 dev environment — pinned rust toolchain, qemu, cargo xtask";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ];  # x86_64-darwin out of project scope
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };
    in {
      devShells = forAllSystems (system: let
        pkgs = pkgsFor system;
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in { default = pkgs.mkShell {
        packages = [ toolchain pkgs.qemu pkgs.dtc ];
        shellHook = ''
          # llvm-tools component lives in the toolchain's rustlib dir; xtask honors $OBJCOPY
          export OBJCOPY="''${OBJCOPY:-$(ls ${toolchain}/lib/rustlib/*/bin/llvm-objcopy 2>/dev/null | head -1)}"
          echo "r9 devshell: $(rustc --version); qemu $(qemu-system-aarch64 --version | head -1)"
        '';
      }; });
      apps = forAllSystems (system: let pkgs = pkgsFor system; in {
        build     = { type = "app"; program = "${pkgs.writeShellScript "r9-build" "exec cargo xtask build --arch aarch64 \"$@\""}"; };
        boot-virt = { type = "app"; program = "${pkgs.writeShellScript "r9-boot-virt" "exec cargo xtask boot-test --arch aarch64 --config virt \"$@\""}"; };
      });
    };
}
```

(llvm-tools-in-overlay path glob is the standard rust-overlay layout [ASSUMED exact subpath — verify at implementation]; `xtask` already resolves `OBJCOPY` env first [VERIFIED: ../r9/xtask/src/main.rs:246-271].)

### Anti-Patterns to Avoid

- **Waiting for x0 on virt+ELF:** x0 is 0 — always. Don't "fix" qemu; use the fixed DTB PA fallback.
- **Hardcoding 62.5 MHz or 24 MHz:** CNTFRQ differs TCG vs HVF (both measured) — read CNTFRQ_EL0.
- **Matching only `arm,gic-400`:** qemu gicv2's compatible is `arm,cortex-a15-gic` (dumped DTB). Match both, prefer DTB over constants.
- **Unmasking IRQs in `trap::init` before GIC/handler exist:** today's `DAIFClr #2` + always-spin `trap()` = first IRQ wedges the kernel (CONTEXT.md watch-out, confirmed by code read). Split VBAR-setup from unmask; unmask last.
- **Printing via `println!` inside the IRQ handler while the Console lock may be held:** `Console::putstr` takes the mcslock [VERIFIED: ../r9/port/src/devcons.rs:40-50] → self-deadlock if interrupted mid-print. Handler echoes via a raw `putb` path (or defers through a tiny ring to the main loop).
- **Mapping the kernel text range from PA 0:** see Pitfall P3 — start at `boottext_pa`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| System-register defs (timer, VBAR, DAIF, TCR…) | hand-typed asm constants | `aarch64-cpu` 11.2 (already a dep) | tock-registers safety, docs.rs-verified coverage |
| DTB parsing | byte twiddling | `port::fdt` | exists, translated-reg aware |
| Serial expect/PTY harness | PTY crates | unix-socket chardev + `std::os::unix::net` | qemu's socket serial gives capture-before-start with zero deps |
| Rust toolchain pinning in nix | hand-fetched tarballs | rust-overlay `fromRustupToolchainFile` | rust-toolchain.toml stays single source of truth (D-09) |
| GIC/timer register research | guess from memory | the verified probe sequence above | proven on the target qemu this session |

## Common Pitfalls

### P1: ELF `-kernel` ⇒ x0=0 and no DTB in RAM (the phase's #1 landmine)
**What goes wrong:** kernel waits for a DTB pointer that never comes / parses garbage at 0.
**Why:** qemu `hw/arm/boot.c` v11.1.2: ELF ⇒ `is_linux=0` ⇒ no aarch64 boot stub (which sets x0), and the 1 MiB generated DTB exceeds the 512 KiB `dtb_limit` gap under the kernel image ⇒ `arm_load_dtb` returns silently.
**Avoid:** `-dtb` with a trimmed committed DTB (verified: FDT magic readable at 0x40000000); virt platform reads DTB at fixed PA 0x40000000 when x0==0.
**Warning signs:** `DeviceTree::from_usize` fails / InvalidMagic; x0==0 in the entry asm.
**Evidence:** flat boot gets DTB at 0x48000000 (post-initrd, is_linux path) — ramdump-verified; ELF boot has no FDT anywhere in the 1 GiB RAM (pmemsave grep) — both on qemu 11.1.2.

### P2: HVF cannot host the GICv2 config on darwin
`qemu-system-aarch64: HVF does not support GICv2 emulation` (verified, 11.1.2; HVF+gic-version=3 also rejected — Phase 2 notes). HVF default machine = GICv3 (host vGIC). **Avoid:** accel default = TCG on darwin this phase; `accel` key exists for the future GICv3 driver and for KVM on linux-aarch64 (vGICv2 supported, host-dependent [CITED: kernel KVM logs/lkml v2-on-v3] — TCG fallback if a host lacks it).

### P3: `base_physaddr() = 0` poisons the virt map
`boottext_physrange()` = 0..eboottext in **both** `kmem.rs` and `pre_mmu/vminit.rs` [VERIFIED: ../r9/aarch64/src/kmem.rs:16-18,56-58; vminit.rs:36-42,84-86]. On virt that's 0..~0x40090000 — vminit would map >1 GiB as Normal kernel text, covering GIC/PL011 MMIO (speculative-fetch hazard), and burn early page-table pool. **Avoid:** start kernel ranges at the linker's `boottext_pa` (per-platform; raspi may keep 0 deliberately — the TODO there already contemplates unmapped page 0).

### P4: identity map misses the virt kernel
`physicalpt3.entries[0]` is a single 1 GiB block at PA 0 [VERIFIED: ../r9/aarch64/src/pre_mmu/vminit.rs:195-216]. Kernel at 0x40080000 is outside it → the MMU-on jump faults. **Avoid:** on virt add `physicalpt3.entries[1]` (1 GiB block at 0x40000000) or platform-parameterize the identity window. (CONTEXT.md said "PA 0–2MiB" — code says 0–1 GiB; either way 0x40080000 is uncovered.)

### P5: IRQ unmask ordering
`trap::init` sets VBAR **and** `DAIFClr #2` [VERIFIED: ../r9/aarch64/src/trap.rs:9-20]. With GIC not yet initialized, any pending/spurious IRQ hits the spin. **Avoid:** split into `init` (VBAR) + `unmask_irq()` called after GIC+timer+UART arm. Raspi keeps working whether or not it ever unmasks (nothing uses IRQs there today).

### P6: SPIs need ITARGETSR
PL011 (INTID 33) is an SPI: without `GICD_ITARGETSRn` the IRQ is never delivered to cpu0 (PPIs like the timer are hardwired per-CPU). Verified requirement in the probe.

### P7: serial routing is platform-specific
Current aarch64 qemu arm: `-nographic -serial null -serial mon:stdio` (PL011→null, miniuart→stdio) [VERIFIED: ../r9/xtask/src/main.rs:475-483]. On virt the PL011 **is** serial0 and must carry stdio: `-nographic -serial mon:stdio` (verified working). QemuStep needs a per-platform serial-routing branch fed by the config.

### P8: Console-lock reentrancy in IRQ context
Handler-side `println!` can self-deadlock on the mcslock. Use a lock-free echo (store the PL011 `VirtRange` in a static and write DR directly) or a ring drained by the main loop.

### P9: pagealloc ceiling is 4 GiB (docs say 2)
`BitmapPageAlloc<32, 4096>`: `end = 32 × 4096 × 8 × 4096 = 4 GiB` [VERIFIED: ../r9/port/src/bitmapalloc.rs:61-69]. CONTEXT.md/HowItWorks.md say 2 GiB — stale docs, not a bug. Keep `-m 1024M` (matches riscv64) anyway: RAM at 0x40000000 indexes the bitmap from 1 GiB, so ≤3 GiB guest RAM fits.

### P10: check-cfg warnings for `platform`/custom cfgs
nightly-2026-01-30 runs check-cfg; `--cfg platform="virt"` from rustflags triggers `unexpected_cfgs` warnings unless declared (riscv64 builds already hit this class). Cosmetic, but plan a `[lints.rust]`/build.rs `cargo::rustc-check-cfg` tidy-up if warnings bother CI. [ASSUMED — severity LOW]

### P11: qemu drift (Phase 2 lesson, carried)
Pin behavioral expectations to qemu 11.1.2 (nixpkgs). The committed `virt-gicv2.dts/.dtb` is regenerated via `-machine dumpdtb` + dtc round-trip; document that procedure in r9. Harness asserts banner/ticks/echo — never timings.

### P12: DistStep shape change
DistStep currently objcopy→gzip for **all** aarch64 [VERIFIED: ../r9/xtask/src/main.rs:350-390]. For virt it must be skipped (D-05): QemuStep for virt uses `target/aarch64-unknown-none-elf/<profile>/aarch64` (the ELF bin) directly; raspi keeps the `.gz` flat path. Make DistStep config-aware rather than arch-aware.

## Code Examples

### config_virt.toml (sketch per D-07 + findings)

```toml
[build]
target = "lib/aarch64-unknown-none-elf.json"
buildflags = ["-Z", "build-std=core,alloc"]

[config]
platform = "virt"
# no dtb key here: virt's DTB is qemu-facing, passed via [qemu].dtb

[link]
script = 'aarch64/lib/kernel.ld'
load-address = '0xffff800040080000'   # KZERO + 0x40080000; AT() gives p_paddr 0x40080000

[qemu]
machine = "virt,gic-version=2"
cpu = "cortex-a72"        # TCG reference CPU (HVF would need host, but HVF+GICv2 is rejected — P2)
smp = 1
memory = "1024M"
accel = "tcg"             # darwin default this phase; "kvm" viable on linux-aarch64, "hvf" only with a future GICv3 driver
dtb = "aarch64/lib/virt-gicv2.dtb"
```

### config_virtpanic.toml (D-12) — zero xtask schema change

```toml
# inherits virt shape; the existing generic [config] dev mechanism emits --cfg dev_panic
[config]
platform = "virt"
dev = ["panic"]           # xtask/src/config.rs already maps dev = [...] → --cfg dev_<key>
```

then in `main.rs`: `#[cfg(dev_panic)] platform::panic_at_end_of_main9();`
(Alternative: `features = ["panic"]` + a Cargo.toml feature — heavier, crosses the workspace; the `dev` cfg route reuses existing generic plumbing [VERIFIED: ../r9/xtask/src/config.rs:149-157].)

For the dump itself, two viable shapes (planner picks):
- **`brk #0` route (recommended):** a deliberate sync exception runs the *real* trap path; `trap()`'s sync branch prints the existing `TrapFrame` Debug (full registers) + a clean-halt marker + `wfi` loop. Reuses [VERIFIED: ../r9/aarch64/src/trap.rs:62-102] and exercises machinery Phase 4 needs.
- **`panic!()` route:** extend the panic handler [VERIFIED: ../r9/aarch64/src/runtime.rs:13-19] to also print captured SP/FP/LR + a short frame-pointer backtrace. Lighter, but only message+location by default.

### DTB-derived platform info (D-03/D-04)

```rust
// platform/virt/mod.rs
pub fn print_machine_info(dt: &DeviceTree) {
    if let Some(mem) = dt.find_device_type("memory")
        .flat_map(|m| dt.property_translated_reg_iter(m).flat_map(|r| r.regblock()))
        .next() {
        println!("  Memory:\t{:#x} .. +{:#x}", mem.addr, mem.len.unwrap_or(0));
    }
    for compat in ["arm,pl011", "arm,cortex-a15-gic", "arm,gic-400", "arm,armv8-timer"] {
        if let Some(n) = dt.find_compatible(compat).next() {
            println!("  found: {compat}");
        }
    }
}
```

### Raspi regression gates (compile-only, no hardware)

```bash
cargo xtask build  --arch aarch64 --config default    # raspi3b
cargo xtask build  --arch aarch64 --config raspi4b
cargo xtask clippy --arch aarch64 --config virt       # + default + raspi4b (CI parity)
cargo xtask check                                     # host-side tests keep compiling
```

These need only the devshell. Deeper optional check: `cargo xtask qemu --arch aarch64 --config default` still boots raspi3b under TCG (README documents that path works today).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| r9 aarch64 = raspi-only flat crate | riscv64-style `platform/` tree (D-01) | this phase | virt lands alongside raspi |
| xtask hardcodes raspi qemu arm | `[qemu]` TOML data-driven (D-07) + platform-aware serial/accel/dist | this phase | new platforms = TOML, not Rust edits |
| expect/pexpect acceptance harness (Harvey Phase 1/2) | Rust `xtask boot-test`, zero-dep unix-socket (D-11) | this phase | single-toolchain purity in r9 |
| rustup host installs | r9-local flake via rust-overlay on rust-toolchain.toml (D-08/D-09) | this phase | zero host setup (R9-05) |

**Deprecated/outdated:**
- "Qemu starts in EL3" comment in `l.S` — measured EL1 for both ELF and flat boots on qemu 11.1.2 (l.S dispatch handles all; comment only) [VERIFIED empirically].
- r9 README "aarch64: only Raspberry Pi 3" — update with the virt row this phase.
- HowItWorks.md aarch64 sections — must be updated with the platform tree + DTB contract (its header mandates it).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | rust-overlay resolves dated nightly-2026-01-30 + components (incl. llvm-tools) + 3 targets on aarch64-darwin/linux; llvm-objcopy lands under `lib/rustlib/<host>/bin/` | flake pattern | devshell needs a fallback (fenix or explicit component list); OBJCOPY hook path changes. Low — dated nightlies are a documented rust-overlay feature |
| A2 | KVM on linux-aarch64 hosts can run `-M virt,gic-version=2 -cpu host` (vGICv2 / v2-on-v3) | P2 / accel matrix | that host falls back to TCG (already the design); only affects D-10's linux manual run speed |
| A3 | IBRD=13/FBRD=1 for 115200 @ 24 MHz on virt's PL011 matters only on real hardware | Pattern 3 | none under qemu (divisors ignored) |
| A4 | qemu's ELF entry resolution uses the physical load address when e_entry is high-half (mechanism not fully source-confirmed; behavior empirically verified) | boot contract | none for r9 (its entry = start of first load segment either way) |
| A5 | check-cfg warnings appear for new `--cfg platform` values (cosmetic) | P10 | warnings in CI logs only |

## Open Questions (RESOLVED)

All three questions carry documented dispositions encoded in plans 03-02/03-03/03-04 — nothing remains open for planning or execution.

1. **Does the trimmed DTB stay valid across qemu bumps?** — DISPOSITION: resolved by the P11 drift policy; encoded in plan 03-04 Task 1 step (1) (committed `virt-gicv2.{dts,dtb}` with the regeneration-procedure header comment) and the 03-06 HowItWorks.md update.
   - Known: qemu injects `/memory` + `/chosen` even with `-dtb`; device addresses are machine-model constants.
   - Unclear: nothing functional — only drift-watch (P11). Regeneration procedure: `qemu-system-aarch64 -M virt,gic-version=2 -machine dumpdtb=… ` → dtc dtb→dts→dtb → commit both.
   - Recommendation: commit `virt-gicv2.{dts,dtb}`; regenerate on qemu pin bumps.
2. **linux-aarch64 KVM GICv2 availability on the fleet's linux host** (A2) — resolved by D-10's manual run; TCG fallback already specified. DISPOSITION: the fallback is wired in plan 03-03's accel override chain (flag → R9_ACCEL env → TOML → tcg default).
3. **`test_sysexit` fate in virt main9** — CONTEXT delegates to planner; research recommends gating it to raspi configs (Phase 4 scope), keeping virt main9 minimal. DISPOSITION: adopted verbatim — plan 03-02 Task 2 step (1) gates `test_sysexit` and the user-page-table block behind cfg(not(platform = "virt")) with a Phase-4-revisits comment.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-aarch64 | SC#2–5, boot-test | ✓ (nix store; Harvey devshell) | 11.1.2 — verified working | r9 flake's pkgs.qemu (same nixpkgs lineage) |
| nix | SC#1 flake | ✓ | 2.34.6 (Determinate) | — |
| dtc | DTB regeneration | ✓ (nix store) | 1.8.1 | add to r9 devShell |
| cargo/rustup on host PATH | building r9 today | ✗ absent | — | **the r9 flake itself (SC#1) is the bootstrap — sequence it early in the plan** |
| HVF (darwin accel) | D-06 darwin arm | ✗ for GICv2 (verified rejection) | — | TCG (precedent: Phase 1/2 TCG acceptance) |
| linux host (D-10 second verification) | SC#1 | ✓ via lima/other (Harvey devshell provides lima) | — | — |
| llvm-mc/clang (research only) | probes this session | ✓ (nix store) | 21.1.8 | not needed by the plan |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** host rust toolchain (flake supplies), HVF (TCG), linux KVM GICv2 (TCG).

## Security Domain

Bare-metal kernel bring-up; no network, no users, no secrets. Honest ASVS mapping:

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Auth / V3 Session / V4 Access Control | no | — (no userspace, no sessions this phase) |
| V5 Input Validation | marginal | two untrusted-ish inputs: the DTB (qemu-generated or committed — trusted channel this phase; `port::fdt` is bounds-checked with `ParseError`) and serial RX bytes (echo only — no command interpretation this phase; control bytes must not trigger actions) |
| V6 Cryptography | no | — |
| V7 Error Handling | yes-in-spirit | SC#5: panic path prints state and halts deterministically (no silent wedge); trap spin only for unexpected sync exceptions |
| V12 Files/Resources | marginal | MMIO ranges come from DTB and are mapped via `map_device_register` with bounds-checked `read_reg`/`write_reg` (panics on out-of-range offset) |

Notes for the planner: (1) the `virtpanic` config is a debug artifact — keep it out of any default/documented "production" config set; (2) IRQ handler echo must not interpret control bytes (Ctrl-P etc. are deferred per CONTEXT.md); (3) no new network listeners or host file access are introduced by the kernel changes; the boot-test harness writes evidence logs only under the repo/evidence dir.

## Sources

### Primary (HIGH confidence) — empirical, this machine, qemu 11.1.2
- Bare-metal probes (hand-assembled + clang-assembled flat/ELF kernels) run under `-M virt,gic-version=2 -cpu cortex-a72 -smp 1`: EL1 entry; high-half-VMA ELF entry resolution; x0=0; DTB absent for ELF; `-dtb` trimmed → FDT magic at 0x40000000; GICv2 register sequence → INTID 30 ticks; PL011 IMSC/ICR/DR/FR RX path → INTID 33 + echo; CNTFRQ 62.5 MHz (TCG) / 24 MHz (HVF); HVF+GICv2 rejection error text.
- qemu 11.1.2 sources: `hw/arm/boot.c` (is_linux/ELF/DTB-limit logic, bootloader stub, do_cpu_reset), `hw/arm/virt.c` (`skip_dtb_autoload`, `virt_machine_done` → `arm_load_dtb`), `hw/char/pl011.c` (DR/RX semantics, reset state CR=0x300, read_trigger=1).
- Dumped DTBs: `-M virt,gic-version=2` and HVF-default (`arm,gic-v3` contrast).
- In-repo reads (verbatim line refs inline): r9 `main.rs`, `l.S`, `trap.{rs,S}`, `vminit.rs`, `kmem.rs`, `uartpl011.rs`, `devcons.rs` (both), `deviceutil.rs`, `pagealloc.rs`, `bitmapalloc.rs`, `xtask/{main,config}.rs`, config TOMLs, `Cargo.lock`, target JSON, riscv64 platform tree, Phase 2 `build/aarch64/README.md` drift notes, Harvey `flake.nix`.

### Secondary (MEDIUM confidence)
- docs.rs aarch64-cpu 11.2.0 register listing (timer/VBAR/DAIF coverage) [CITED: docs.rs/aarch64-cpu/11.2.0/aarch64_cpu/registers]
- oxalica/rust-overlay cheat sheet (`fromRustupToolchainFile`) [CITED: github.com/oxalica/rust-overlay]; fenix `fromToolchainFile` [CITED: github.com/nix-community/fenix]
- qemu virt machine docs (gic-version values; GICv2 8-CPU limit) [CITED: qemu-project.gitlab.io/qemu/system/arm/virt.html]
- KVM arm64 vGICv2 / v2-on-v3 support evidence [CITED: lkml.iu.edu KVM patch 2510.0/04465; Fedora KVM log excerpts]

### Tertiary (LOW confidence)
- Package-name discovery for rexpect/expectrl (training knowledge) — elevated by seam legitimacy check to registry-confirmed; still fallback-only.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — kernel needs zero new deps; toolchain pieces verified locally
- Architecture: HIGH — every mechanism in the patterns section ran on the target qemu
- Pitfalls: HIGH — P1–P8 all verified in code or live; P9–P12 code-verified

**Research date:** 2026-11-01
**Valid until:** 2026-12-01 (qemu/nixpkgs pin bumps invalidate the DTB-dump specifics; the boot.c ELF behavior has been stable for a decade)
