# Phase 3: r9 qemu-virt Platform Bring-up - Context

**Gathered:** 2026-11-01
**Status:** Ready for planning

<domain>
## Phase Boundary

r9's aarch64 crate boots on qemu `-M virt` — kernel entry, memory map, GICv2, PL011 console, generic timer — proven by kernel log output in qemu, with raspi3b/4b continuing to work alongside. Plus R9-05: r9 carries its own `flake.nix` (devshell + apps) so `nix develop` + `cargo xtask build --arch aarch64` reproduces the kernel on darwin-aarch64 and linux with zero host setup. **All code work happens in the sibling repo `../r9`** (fork `requaos/r9`, branch `main`); planning artifacts and acceptance evidence live in this repo.

NOT in this phase: userspace-exec / EL0 / `svc` dispatch (Phase 4), Harvey userland on r9 (Phase 5), root-fs delivery (Phase 4), flake `packages.*` outputs and CI-green (Phase 5 INT-02/INT-04), virtio-anything / SMP-PSCI / RTC / PCIe (v2 deferrals), upstream-r9 PR flow.

</domain>

<decisions>
## Implementation Decisions

### Platform abstraction shape
- **D-01:** Adopt the **riscv64-style platform tree** in the aarch64 crate: `aarch64/src/platform/{mod.rs,virt/,raspi3b/,raspi4b/}`. `mailbox.rs`, `uartmini.rs`, GPIO and bcm DTB handling move under the raspi modules; the new `platform/virt/` module gets PL011/GIC/timer. The existing `[config] platform` TOML key drives `--cfg platform="virt"` (already implemented in `xtask/src/config.rs`). — **Reversibility:** costly — undoing means reverting a file-tree restructuring across the crate, but no published contract depends on it.
- **D-02:** **Platform-gated early-boot console**: virt gets an early PL011 debug path at `0x09000000` (no init needed — qemu pre-configures it) under `platform/virt`; raspi keeps its miniuart path in `pre_mmu/util.rs`. Early prints are the bring-up debug lifeline before DTB/MMU are up.
- **D-03:** **Gate `mailbox::init` + `print_board_info` to raspi platforms.** On virt, `main9` prints DTB-derived machine info instead (memory size, CPU, discovered GIC/PL011 addresses) — same "what machine am I on" value, no fake VideoCore calls.
- **D-04:** **DTB-driven device discovery on virt** via the existing `port/fdt.rs` (`find_compatible("arm,pl011")`, `"arm,gic-400"`, `"arm,armv8-timer"` + translated reg iter) — matches how raspi devices are already found; survives qemu machine-address drift. No hardcoded virt constant tables.

### Boot image + acceleration
- **D-05:** **ELF `-kernel` for virt; flat+gzip stays raspi-only.** qemu loads the ELF via program headers (the linker script's `AT()` already emits correct physical addresses), gdb gets full symbols for the Phase 4 debug path, and the objcopy/gzip dist step is skipped for virt. Matches R9-01's "ELF `-kernel` boot" wording.
- **D-06:** **Native acceleration with TCG fallback**: `-accel hvf -cpu host` on darwin-aarch64, `-accel kvm -cpu host` on linux-aarch64, TCG + `-cpu cortex-a72` elsewhere; flag/env-overridable. (HVF cannot emulate cortex-a72 — under HVF the cpu must be `host`.) The canonical SC#2 command (`-M virt,gic-version=2 -cpu cortex-a72`) remains the TCG reference form.
- **D-07:** **Extend the `[qemu]` TOML section** with `cpu`/`smp`/`memory`/`accel` keys alongside the existing `machine`/`dtb` — `config_virt.toml` declares `machine = "virt,gic-version=2"` etc. xtask stays generic; new platforms are data, not Rust edits. Note: qemu's `-M` accepts the full `virt,gic-version=2` string as-is.

### r9 flake surface (R9-05)
- **D-08:** Phase 3 ships **devShell + apps** (`build`, `boot-virt` wrapping `cargo xtask` — the two things a fresh checkout needs, mirroring Harvey's app style). `packages.kernel-*` outputs are **deferred to Phase 5**, whose INT-04 flake-input consumer defines their shape. (R9-05's "devshell + apps mirroring this repo's conventions" wording is satisfied without packages.)
- **D-09:** **Honor `rust-toolchain.toml` via an overlay** (oxalica/rust-overlay or fenix — agent's pick). The pinned nightly-2026-01-30 + components (rustfmt, rust-src, clippy, llvm-tools) + three targets stay the single source of truth; r9's history of "bump rust" commits must not require dual-pin edits.
- **D-10:** **Manual host verification this phase**: the SC#1 command pair is run by hand on darwin-aarch64 and a linux host. r9's existing GH Actions (check/clippy/test/build on ubuntu) stay as-is; CI-green-on-both-hosts is INT-02, Phase 5.

### Acceptance evidence + panic trigger
- **D-11:** Acceptance harness is a **Rust `cargo xtask boot-test` subcommand** (user decision, overriding the pexpect recommendation) — spawns qemu, captures the serial log, asserts banner / periodic timer ticks / typed-char echo / panic dump; human checkpoint approves the captured log at the gate (Phase 1/2 evidence standard). Adds a rexpect-style crate dep to the workspace — accepted for single-toolchain purity (no python/expect in the r9 devshell).
- **D-12:** The SC#5 deliberate panic is a **boot-time panic config** (user decision, overriding the Ctrl-P recommendation) — a `--config panic` TOML variant that panics at end of `main9`, printing registers/stack and halting cleanly. Deterministic and scriptable; decoupled from IRQ-input flakiness.
- **D-13:** **Harness lives in r9; canonical evidence lives in Harvey `.planning/phases/03-*/`** (user decision, overriding the all-in-r9 recommendation) — milestone evidence stays consolidated in the planning tree; the phase dir records the r9 commit SHA + evidence paths in r9.

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- qemu invocation shape: `-M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic`, ELF `-kernel`, DTB at `0x40000000`, GICv2 @ `0x08000000`, PL011 @ `0x09000000`, ARM generic timer (R9-01, SC#2–5). GICv2 (not v3) is the roadmap's explicit choice.
- raspi3b/4b keep working — virt lands *alongside* (roadmap wording).
- Proof standard: **boot/execution-test with captured serial evidence + human checkpoint** (Phase 1 BUILD-05 / Phase 2 TC-gate pattern).
- `aarch64` naming everywhere; r9 code follows r9's own `rustfmt.toml` and conventions (Harvey's Plan 9 C style rules do not cross the repo boundary).
- minimega is amd64/KVM-only — aarch64 boots use plain qemu (PROJECT.md key decision).
- r9 commits land directly on `main` of the `requaos/r9` fork (Phase 1's `allow_default_branch_commits` fork workflow carried across repos).
- Harvey user-side ABI is frozen (`aarch64/ABI.md`, `aarch64/include/ureg.h`) — Phase 4 consumes it; **no userspace-exec work in this phase** (r9's existing `test_sysexit` experiment in `main.rs` is not a deliverable; planner decides its fate in the virt main9).

### the agent's Discretion
- Timer tick rate and log verbosity (e.g. print every Nth tick ≈ 1Hz) — SC#3 only requires visible periodic ticks.
- GICv2 + generic-timer driver file placement/style inside `platform/virt/` or the aarch64 crate root (DTB-discovered per D-04).
- Overlay choice (rust-overlay vs fenix) and exact devshell package list (`pkgs.qemu` provides `qemu-system-aarch64`).
- rexpect-style crate choice for `boot-test`.
- How the panic config plumbs (cargo feature vs `[config]` cfg flag) and where `panic!()` dumps registers (existing `TrapFrame` Debug is available).
- Early-PL011 print mechanics and the virt identity-map/load-address adjustment in `kernel.ld` + `pre_mmu/vminit.rs` (kernel loads at PA `0x40080000`, not raspi's `0x80000`).
- What virt `main9` looks like beyond the SCs (banner, memory info, tick loop, echo) — keep minimal.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 3 — goal, success criteria 1–5, pre-planning flag (map r9 before planning — partially satisfied by this phase's scout; researcher completes it)
- `.planning/REQUIREMENTS.md` — R9-01, R9-05 full text; v2 deferral list (virtio-9p, SMP/PSCI, virtio-net/blk, RTC, PCIe ECAM)
- `.planning/PROJECT.md` — Key Decisions (r9 adoption, aarch64 naming, minimega boundary, r9-own-flake decision "r9 gets its own flake; Harvey consumes r9 via flake input")

### Prior-phase decisions (Harvey repo)
- `.planning/phases/02-aarch64-toolchain-libc-9front-donor/02-CONTEXT.md` — D-02 (HVF on the M5 Max proven as execution locus), D-03 (Apple 16K host pages constrain guest granule; r9 cache maintenance must use architectural discovery via `CTR_EL0`, not hardcoded geometry), D-06/D-08 (execution-gate proof standard)
- `.planning/phases/01-build-tail-green-9k-amd64/01-CONTEXT.md` — boot-test-not-build-test lesson; serial capture/assert harness pattern (banner grep is source of truth; capture attaches before start); TCG-accepted human checkpoint precedent
- `aarch64/ABI.md` + `aarch64/include/ureg.h` — the frozen Harvey user-side ABI. **Context only for Phase 3** (no syscall work this phase); the trap-frame layout r9 grows in Phase 4 must match `ureg.h`, so the researcher should know it exists.

### r9 repo (sibling, `../r9` — where all code work happens)
- `../r9/README.md` — build/qemu invocation table; "raspi3b only" aarch64 status
- `../r9/HowItWorks.md` — maintainer's design tour (aarch64 early init, VM init, port crate); **update it with material changes** per its own header
- `../r9/rust-toolchain.toml` — nightly-2026-01-30 pin, components (llvm-tools!), targets
- `../r9/xtask/src/main.rs` — BuildStep/DistStep/QemuStep; the aarch64 qemu arm currently hardcodes `-nographic -serial null -serial mon:stdio -d int` + gzipped-flat `-kernel` (serial routing there is raspi-miniuart-specific: PL011→null, miniuart→stdio — virt needs the opposite)
- `../r9/xtask/src/config.rs` — TOML schema: `[build]`, `[config]` (platform/dev/ip/link/nodev/nouart cfg keys), `[link]` (`${LOAD-ADDRESS}` substitution), `[qemu]` (machine/dtb) — D-07 extends this
- `../r9/riscv64/src/platform/{mod.rs,virt/mod.rs,virt/devcons.rs,nezha/...}` + `../r9/riscv64/lib/config_default.toml` — the platform-tree precedent D-01 adopts (`[config] platform = "virt"`)
- `../r9/aarch64/src/main.rs` — `main9`: trap init → DTB parse → pagealloc → devcons (MiniUart!) → mailbox → prints → VM → test_sysexit → loop. The raspi couplings D-03 gates.
- `../r9/aarch64/src/l.S` — entry `start`: EL3→EL2→EL1 dispatch, DTB pointer in x0→x27, calls `init_vm` then `main9`
- `../r9/aarch64/src/pre_mmu/vminit.rs` + `../r9/aarch64/lib/kernel.ld` — early page tables: kernel VA = KZERO + load-address, identity map currently covers PA 0–2MiB (raspi kernel at `0x80000`); virt kernel lands at PA `0x40080000` so both need per-platform adjustment
- `../r9/aarch64/src/trap.rs` + `trap.S` — VBAR setup + `DAIFClr #2` unmask IRQs already, but **any IRQ today hits "Unhandled interrupt" and spins**; `TrapFrame` (x0–x30 + ESR/ELR/FAR + interrupt_type) is the panic-dump foundation
- `../r9/aarch64/src/uartpl011.rs` — existing PL011 driver is raspi-flavored (bcm2835-gpio pin setup, mailbox clock set): reuse register-level code, bypass GPIO/mailbox init on virt (qemu pre-configures 115200)
- `../r9/aarch64/src/devcons.rs` + `../r9/port/src/devcons.rs` — `Console`/`Uart` trait is **output-only** (`putb`); interrupt-driven input needs a new input path (SC#4)
- `../r9/port/src/fdt.rs` — the DTB parser D-04 builds on (`find_compatible`, `property_translated_reg_iter`, `find_by_path`)
- `../r9/aarch64/src/deviceutil.rs` — `map_device_register` helper for MMIO mapping (GICD/GICC/PL011)
- `../r9/aarch64/src/pagealloc.rs` — bitmap allocator: 16KiB early pages ⇒ **2GiB RAM ceiling**; keep qemu `-m` at/below 1G (riscv64 uses 1024M)
- `../r9/aarch64/src/vm.rs` — 4K granule (Page4K/Page2M), 48-bit VA assumption, KZERO split
- `../r9/.github/workflows/xtask.yml` — existing CI left untouched per D-10

### qemu virt machine facts (researcher verifies against installed qemu)
- `-M virt,gic-version=2` memory map: PL011 `0x09000000` (INTID 33), GICv2 GICD `0x08000000` / GICC `0x08010000`, ARM generic timer CNTP INTID 30 (PPI 14), RAM at `0x40000000`, `-kernel` load at `0x40080000`, qemu-generated DTB placed at `0x40000000`
- HVF on Apple silicon requires `-cpu host`; Phase 2 donor gates ran HVF on this M5 Max (`build/aarch64/` harness notes, incl. qemu-11.1.2 drift accommodations — read before assuming qemu behavior)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `port/fdt.rs` DTB parser — full node/property/translated-reg machinery; virt device discovery needs only new compatible strings (`arm,pl011`, `arm,gic-400`, `arm,armv8-timer`, `/memory`)
- `port/devcons.rs` `Console` lock + `println!` macro — output path unchanged; input path is new
- `aarch64/uartpl011.rs` register interface — reusable minus its raspi GPIO/mailbox init
- `aarch64/deviceutil.rs` `map_device_register` — MMIO mapping for GICD/GICC/PL011 post-VM
- xtask config TOML system — per-platform config files + `${LOAD-ADDRESS}` linker substitution already exist; D-07 extends `[qemu]`
- riscv64 `platform/` module tree — the exact structural precedent for D-01
- `TrapFrame` Debug impl — ready-made register dump for the D-12 panic path
- Harvey Phase 1/2 harness evidence shape (captured serial log + asserted markers + human checkpoint) — the acceptance standard D-11 reimplements in Rust

### Established Patterns
- One `config_{name}.toml` per platform per arch crate (`config_default.toml`, `config_raspi4b.toml`); `cargo xtask … --config <name>` selects
- Early-boot debug uart per platform in `pre_mmu` (raspi miniuart) — virt adds its own per D-02
- `main9` orchestration: traps → DTB → pagealloc → console → info prints → VM — virt keeps the shape, swaps the raspi steps
- r9 commits on `main`; HowItWorks.md updated with material changes (its own header mandates it)

### Integration Points
- `xtask/src/main.rs` — DistStep (skip objcopy/gzip for ELF-per-D-05), QemuStep (aarch64 arm: platform-aware serial routing + accel + `-cpu`/`-smp`/`-m` from TOML), new `boot-test` subcommand per D-11
- `aarch64/src/main.rs` — platform dispatch for devcons/mailbox/early-info (D-02/D-03)
- `aarch64/src/l.S` + `pre_mmu/vminit.rs` + `kernel.ld` — virt load address `0x40080000` and identity-map window
- `aarch64/src/trap.rs`/`trap.S` — IRQ vector dispatch to GIC ack → timer/UART handlers (greenfield; no GIC/timer code exists anywhere in r9 today — confirmed by grep)
- New files: `aarch64/lib/config_virt.toml` (+ panic variant per D-12), `aarch64/src/platform/**`, GICv2 + generic-timer drivers, `../r9/flake.nix`

</code_context>

<specifics>
## Specific Ideas

- Canonical TCG acceptance command (SC#2 wording): `qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic` — the harness asserts against this shape; darwin native form is `-M virt,gic-version=2 -accel hvf -cpu host`.
- `config_virt.toml` sketch: `[config] platform = "virt"` (no `dtb` key — qemu generates virt's DTB), `[qemu] machine = "virt,gic-version=2"` plus D-07's new keys, `[link] load-address` adjusted so VA = `KZERO + 0x40080000`.
- User overrides to honor downstream (deliberate, don't second-guess): Rust `boot-test` over pexpect (single-toolchain purity); boot-time panic config over Ctrl-P (deterministic gate); evidence copies in Harvey `.planning` over all-in-r9 (consolidated milestone evidence).
- xtask's current aarch64 serial routing (`-serial null -serial mon:stdio`) is raspi-miniuart-specific — on virt the PL011 is serial0 and must be the stdio console; platform-aware routing is part of D-07's plumbing.
- Watch-out: pagealloc's 2GiB ceiling (bitmap over 16KiB early pages) — keep qemu `-m` ≤ 1G this phase.
- Watch-out: `trap::init` already unmasks IRQs (`DAIFClr #2`) with no handler — any spurious IRQ wedges the kernel before GIC init lands. Order matters: GIC + handler before/en atomically with unmask.
- Phase 2's qemu-11.1.2 drift notes (GIC wake poke, ECAM/DTB loader behavior) live in `build/aarch64/` — researcher reads these before assuming qemu behavior; per D-06, drift accommodations stay harness-side, never in vendored/pinned trees.

</specifics>

<deferred>
## Deferred Ideas

- virtio-9p host mount (collapses userland iteration time) — top v1.x candidate per REQUIREMENTS.md
- SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM usage — v2
- Ctrl-P → panic as an *interactive* debug path (user chose the deterministic boot-time config for the gate; the keypress path may return as a dev nicety once the IRQ input path exists)
- r9 flake `packages.kernel-*` outputs — Phase 5 (INT-04 consumer defines the shape)
- CI-green-on-both-hosts for r9 — Phase 5 (INT-02)
- Upstream contribution flow to original r9 (r9ops/rminnich lineage) — unraised this phase; fork `requaos/r9` is the working repo
- r9 userspace-exec, EL0 entry, `svc` dispatch, root-fs delivery — Phase 4
- raspi5 support — noted in HowItWorks.md as straightforward; not this project

</deferred>

---

*Phase: 3-r9 qemu-virt Platform Bring-up*
*Context gathered: 2026-11-01*
