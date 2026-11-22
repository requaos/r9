# Phase 3: r9 qemu-virt Platform Bring-up - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-01
**Phase:** 3-r9 qemu-virt Platform Bring-up
**Areas discussed:** Platform abstraction shape, Boot image + acceleration, r9 flake surface (R9-05), Acceptance evidence + panic trigger

---

## Platform abstraction shape

### Q1: How should the `virt` platform coexist with raspi3b/4b in the aarch64 crate?

| Option | Description | Selected |
|--------|-------------|----------|
| riscv64-style platform tree | aarch64/src/platform/{mod.rs,virt/,raspi3b/,raspi4b/} — mailbox/uartmini/gpio under raspi modules, virt module gets PL011/GIC/timer; in-repo precedent, matches existing config-TOML platform key | ✓ |
| Inline cfg blocks in flat files | Less file movement, but cfg noise spreads through every file | |
| Runtime DTB detection | Cleanest conceptually, but load-address/identity-map remain compile-time; raspi/virt differ too much for runtime branching | |

**User's choice:** riscv64-style platform tree (recommended option)

### Q2: How should early-boot debug output (pre-DTB, pre-MMU) work on virt?

| Option | Description | Selected |
|--------|-------------|----------|
| Platform-gated early PL011 | PL011 at 0x09000000 under platform/virt, no init needed (qemu configures it); debug lifeline before DTB/MMU | ✓ |
| No early prints — post-MMU console only | Less code, but silent wedge before devcons init gives no signal | |

**User's choice:** Platform-gated early PL011 (recommended option)

### Q3: How should mailbox::init + print_board_info (VideoCore/raspi-only) be handled?

| Option | Description | Selected |
|--------|-------------|----------|
| Gate mailbox to raspi; DTB info on virt | virt prints DTB-derived machine info (memory, CPU, GIC/PL011 addresses); no fake hardware calls | ✓ |
| Keep calls, stub mailbox on virt | Smallest diff, but dead firmware calls + confusing error log on every virt boot | |

**User's choice:** Gate mailbox to raspi; DTB info on virt (recommended option)

### Q4: How should virt device addresses (PL011, GICv2, timer IRQ numbers) be found?

| Option | Description | Selected |
|--------|-------------|----------|
| DTB-driven discovery | Existing port/fdt.rs with arm,pl011 / arm,gic-400 / arm,armv8-timer compatibles; survives qemu address drift | ✓ |
| Hardcoded virt constants | Simpler to read, duplicates what DTB provides, breaks silently on qemu machine changes | |

**User's choice:** DTB-driven discovery (recommended option)

---

## Boot image + acceleration

### Q1: What image format should the virt platform boot?

| Option | Description | Selected |
|--------|-------------|----------|
| ELF for virt, flat+gz stays raspi-only | qemu loads via program headers (AT() PAs already correct), gdb symbols for Phase 4, no objcopy/gzip step | ✓ |
| Flat binary + gzip everywhere | One dist mechanism, but loses ELF symbols and contradicts R9-01's "ELF -kernel boot" | |

**User's choice:** ELF for virt (recommended option)

### Q2: How should qemu acceleration work for aarch64 virt boots?

| Option | Description | Selected |
|--------|-------------|----------|
| Native accel w/ TCG fallback | hvf/-cpu host on darwin-aarch64, kvm/-cpu host on linux-aarch64, TCG+cortex-a72 elsewhere; flag/env overridable | ✓ |
| TCG-only everywhere | One canonical command, zero accel drift, but 10–50x slower dev loop on this M5 Max | |

**User's choice:** Native accel with TCG fallback (recommended option)

### Q3: Where should the virt qemu args (`virt,gic-version=2`, `-cpu`, `-smp`, `-m`) live?

| Option | Description | Selected |
|--------|-------------|----------|
| Extend [qemu] TOML keys | cpu/smp/memory/accel keys alongside machine/dtb; xtask stays generic, platforms are data | ✓ |
| Hardcode in xtask QemuStep | Fewer moving parts, but every platform variant means editing Rust | |

**User's choice:** Extend [qemu] TOML keys (recommended option)

---

## r9 flake surface (R9-05)

### Q1: What outputs should r9's flake.nix expose in Phase 3?

| Option | Description | Selected |
|--------|-------------|----------|
| Devshell + apps; packages deferred to Phase 5 | `build`/`boot-virt` apps wrap cargo xtask; packages.kernel-* designed with its INT-04 consumer in Phase 5 | ✓ |
| Devshell only | Smallest surface; loses nix-run ergonomics; R9-05 wording says devshell + apps | |
| Devshell + apps + packages now | Stronger repro claim, but package interface designed before its consumer exists | |

**User's choice:** Devshell + apps, packages deferred (recommended option)

### Q2: How should the flake source the pinned rust toolchain?

| Option | Description | Selected |
|--------|-------------|----------|
| Honor rust-toolchain.toml via overlay | oxalica/rust-overlay or fenix; single source of truth for nightly-2026-01-30 + components + targets | ✓ |
| nixpkgs rust-bin, duplicated pin | No extra input, but pin lives in two places and will drift | |

**User's choice:** Honor rust-toolchain.toml via overlay (recommended option)

### Q3: How is SC#1's 'reproduces on darwin-aarch64 and linux' verified, and what happens to r9's CI this phase?

| Option | Description | Selected |
|--------|-------------|----------|
| Manual host verification; CI deferred to Phase 5 | SC#1 commands by hand on both hosts; GH Actions untouched; CI-green is INT-02 | ✓ |
| Extend GH Actions this phase | Earlier regression net, but duplicates Phase 5 infrastructure work | |

**User's choice:** Manual host verification; CI deferred (recommended option)

---

## Acceptance evidence + panic trigger

### Q1: What shape should the acceptance harness take?

| Option | Description | Selected |
|--------|-------------|----------|
| Python pexpect harness in r9 | Proven Phase 2 pattern (srv9.py/gdbpoke9.py); python3+pexpect from devshell | |
| Rust xtask boot-test subcommand | Single-toolchain purity; rexpect-style dep added to workspace | ✓ |
| expect script (Phase 1 pattern) | Proven in Phase 1, clunkier for structured asserts | |

**User's choice:** **Rust xtask boot-test subcommand** — user override of the pexpect recommendation; rationale: single-toolchain purity, no python/expect in the r9 devshell.

### Q2: How should the deliberate kernel panic (SC#5) be triggered?

| Option | Description | Selected |
|--------|-------------|----------|
| Ctrl-P keypress → panic | Chains SC#4 into SC#5 in one keystroke (CTLP already defined in port/devcons.rs) | |
| Boot-time panic config | --config panic variant panics at end of main9; deterministic, decoupled from IRQ-input flakiness | ✓ |
| Both mechanisms | More coverage, slightly more code | |

**User's choice:** **Boot-time panic config** — user override of the Ctrl-P recommendation; rationale: deterministic scripted gate.

### Q3: Where should the acceptance harness and captured evidence live?

| Option | Description | Selected |
|--------|-------------|----------|
| Harness + evidence in r9 | Self-contained repo story; Phase 5 repro references one repo | |
| Harness in r9, evidence in Harvey .planning | Canonical evidence in .planning/phases/03-*/ alongside CONTEXT.md; consolidated milestone evidence | ✓ |

**User's choice:** **Harness in r9, evidence in Harvey .planning** — user override of the all-in-r9 recommendation; rationale: milestone evidence consolidated in the planning tree (phase dir records r9 commit SHA + evidence paths).

---

## the agent's Discretion

- Timer tick rate / log verbosity (SC#3 requires only visible periodic ticks)
- GICv2 + generic-timer driver placement inside platform/virt or crate root
- Overlay choice (rust-overlay vs fenix), devshell package list details
- rexpect-style crate choice for boot-test
- Panic-config plumbing (cargo feature vs cfg flag) and register-dump mechanics
- Early-PL011 print mechanics; virt load-address/identity-map adjustment in kernel.ld + vminit.rs
- virt main9 shape beyond the SCs (keep minimal)

## Deferred Ideas

- virtio-9p host mount — top v1.x candidate (REQUIREMENTS.md)
- SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM — v2
- Ctrl-P → panic as interactive debug path — may return once IRQ input exists
- r9 flake packages.* outputs — Phase 5 (INT-04)
- r9 CI green on both hosts — Phase 5 (INT-02)
- Upstream contribution flow to original r9 — unraised; requaos/r9 fork is the working repo
- raspi5 support — noted in r9's HowItWorks.md as straightforward; not this project
