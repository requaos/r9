---
phase: 03-r9-qemu-virt-platform-bring-up
verified: 2026-11-02T01:49:14Z
status: passed
score: 22/22 must-haves verified
covered_files:
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-01-PLAN.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-02-PLAN.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-03-PLAN.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-04-PLAN.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-05-PLAN.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-06-PLAN.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-01-SUMMARY.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-02-SUMMARY.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-03-SUMMARY.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-04-SUMMARY.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-05-SUMMARY.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-06-SUMMARY.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-CONTEXT.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-REVIEW.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/03-REVIEW-DISPOSITION.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/deferred-items.md
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/evidence/virt-boot.log
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/evidence/virtpanic-boot.log
  - .planning/phases/03-r9-qemu-virt-platform-bring-up/evidence/EVIDENCE.md

covered_digest: "v2:sha256:11e959ee508fc496a82a49e44aa677d16d0a45b2b132be11f7bf3e10fab8106d"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 21/22
  gaps_closed:
    - "SC#1 (R9-05) linux leg — D-10 manual host verification: PASSED by human on a linux host (03-UAT.md test 1, result: pass)"
    - "D-11/D-13 evidence-bundle phase-gate approval: human-approved (03-UAT.md test 2, result: pass)"
  gaps_remaining: []
  regressions: []
---

# Phase 3: r9 qemu-virt Platform Bring-up Verification Report

**Phase Goal:** r9's aarch64 crate boots on qemu `-M virt` — kernel console, interrupts, and timer proven on the milestone's actual target machine before any userspace-exec work begins.
**Verified:** 2026-11-02T01:49:14Z
**Status:** passed
**Re-verification:** Yes — regeneration run. The prior report (2026-11-02T00:49:49Z, `human_needed`, 21/22) was never committed, so its digest could not be re-validated against git state; this run re-verified everything against the live trees. Since that report, the phase's UAT session **completed with both human tests PASS** (commit `cfd692202`), closing the two items that held the prior verdict at `human_needed`.

**Cross-repo scope:** code in sibling repo `/Users/neilskinner/code/r9` (branch `main`, phase commits ae4cd08..55ca7c4, HEAD `55ca7c458952d88245912ad7ad31ca3f55b0e5a6`, tree clean — re-confirmed this run); planning + evidence in this repo. Both were verified.

**Digest note:** the recomputed `covered_digest` is byte-identical to the prior report's value — the covered file set (19 files) has not changed since the prior run, which confirms digest-scope stability. The staleness was a git-state validation failure (the report file itself was untracked), not content drift.

## Verification headline

This verifier re-ran the phase's own acceptance harness **live, in its own process**, against the current r9 HEAD (unchanged from the prior run, tree clean):

- `nix develop -c cargo xtask boot-test --arch aarch64 --config virt` → **`boot-test: PASS -- banner, >= 3 ticks and the echo probe observed in order`** (60,146 bytes captured)
- `nix develop -c cargo xtask boot-test --arch aarch64 --config virtpanic` → **`boot-test: PASS -- banner, virtpanic start marker, >= 30 register rows and the clean-halt marker observed in order`** (61,369 bytes)
- Both fresh captures are **byte-identical** (`cmp` clean) to the committed evidence logs — the D-13 bundle is exactly reproducible from the tree it names (T-03-09's integrity contract holds empirically)
- `nix develop -c cargo test -p xtask` → **11/11 passed**
- `nix develop -c cargo xtask build --arch aarch64` (raspi3b default) and `--config raspi4b` → both `Finished` (alongside-compatibility holds)
- `nix run .#build` with the ELF pre-deleted, outside any devshell → exit 0, kernel ELF produced
- r9 HEAD `55ca7c458952d88245912ad7ad31ca3f55b0e5a6` == EVIDENCE.md's recorded SHA; r9 tree clean (0 modified files)

The two items the prior run routed to human verification are now resolved with human pass evidence recorded in `03-UAT.md` (`status: complete`, 2 passed / 0 issues):

1. **D-10 linux leg of SC#1** — run by hand on a linux host: `nix develop -c cargo xtask build --arch aarch64` produced the kernel ELF, `nix run .#build` worked bare, devshell yielded the pinned nightly + qemu + dtc. **Result: pass.**
2. **D-11/D-13 evidence-bundle phase-gate approval** — human reviewed and approved `evidence/virt-boot.log`, `evidence/virtpanic-boot.log`, and `evidence/EVIDENCE.md` against SC#1–5. **Result: pass.**

SUMMARY.md claims were treated as unverified until re-derived; every behavioral claim above was reproduced from source in this run.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | **SC#1 (R9-05):** `../r9` carries its own flake.nix; `nix develop` + `cargo xtask build --arch aarch64` reproduces the kernel with zero host setup on darwin-aarch64 **and linux** | ✓ VERIFIED | darwin leg re-verified live this run (flake.nix `fromRustupToolchainFile`, flake.lock pins nixpkgs `1c89083e451e` + rust-overlay `c85ccd343087`, single nixpkgs lineage; `nix run .#build` exit 0 with ELF produced). **linux leg VERIFIED by human pass evidence — 03-UAT.md test 1, result: pass (D-10's locked manual leg, executed on a linux host).** |
| 2 | **SC#2:** serial log shows the r9 banner on `-M virt,gic-version=2 -cpu cortex-a72 -smp 1` via ELF `-kernel` with DTB at 0x40000000 | ✓ VERIFIED | Live this run: harness PASS (60,146 B); committed `virt-gicv2.dtb` (7,785 B) is the `-dtb`; log shows `r9 from the Internet` + `DTB found at: 0xffff800040000000`; config_virt.toml declares machine/cpu/smp/memory/accel/dtb as pure data |
| 3 | **SC#3:** GICv2 initialized; ARM generic timer IRQ fires on schedule (periodic ticks) | ✓ VERIFIED | Live this run: harness PASS requires ≥3 ticks; committed log carries exactly 3 `^tick` lines; `gic.rs` (DTB multi-compatible discovery `arm,cortex-a15-gic` or_else `arm,gic-400`, GICC_IAR ack / GICC_EOIR eoi, ITARGETSR for SPIs only) and `timer.rs` (runtime `CNTFRQ_EL0.get()`, `CNTP_TVAL_EL0` arm/re-arm, CTL ENABLE) substantive and wired through `trap.rs` dispatch |
| 4 | **SC#4:** PL011 receives typed input interrupt-driven, visible in kernel log | ✓ VERIFIED | Live this run: harness PASS requires the `r9echo7` probe echoed in post-write bytes (qemu socket serial does not self-echo — bytes reappear only via the kernel's INTID-33 RX IRQ path); committed log carries 1 occurrence; `rx_irq_echo` is lock-free raw DR writes |
| 5 | **SC#5 (D-12):** deliberate kernel panic prints registers/stack and halts cleanly | ✓ VERIFIED | Live this run: virtpanic harness PASS (61,369 B) — committed log carries 31 `^ *x[0-9]+:` rows, `ec: Ok(Brk)`, `interrupt_type: 4` (real sync path), 1 `kernel halted` marker, zero tick interleaving (hook fires before `trap::unmask_irq()`, main.rs:239-245) |
| 6 | rust-toolchain.toml stays the single pin source; flake references it via rust-overlay (D-09) | ✓ VERIFIED | `flake.nix:21` `pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml`; no channel/components/targets literals restated; `rust-overlay.inputs.nixpkgs.follows = "nixpkgs"`; flake.lock pins re-read this run |
| 7 | `nix run .#build` works bare — apps embed runtimeInputs (D-08) | ✓ VERIFIED | Live this run: ELF deleted, `nix run .#build` exit 0 and produced the ELF outside any devshell; `writeShellApplication` with `runtimeInputs = [ toolchain pkgs.qemu ]` (flake.nix:68-70); `apps.boot-virt` and `apps.default` also present |
| 8 | Platform tree `platform/{mod.rs,virt/,raspi3b/,raspi4b/}` with cfg dispatch and raspi3b default fallback (D-01) | ✓ VERIFIED | All 12 files present at new paths; old flat paths (`devcons.rs`, `mailbox.rs`, `uartmini.rs`, `uartpl011.rs`) confirmed absent; raspi4b is a re-export shim over raspi3b |
| 9 | mailbox/board-info/test_sysexit/user-page-table compile only on raspi (D-03) | ✓ VERIFIED | `main.rs` gates all four behind `cfg(not(platform = "virt"))`; virt builds green with the DTB-derived `print_machine_info` filling the slot |
| 10 | trap init/unmask split — init() sets VBAR only; raspi unmasks at its historical point (P5) | ✓ VERIFIED | `trap.rs` `init()` body carries only the `msr vbar_el1` asm; `unmask_irq()` at trap.rs:29 carries the DAIFClr; `main.rs:117` calls it immediately after init on `not(virt)` |
| 11 | raspi3b and raspi4b still compile (virt lands alongside) | ✓ VERIFIED | Live this run: both `Finished` through the devshell |
| 12 | `[qemu]` gains cpu/smp/memory/accel + `[dist] format`; backward-compat argv emission (D-07/D-05) | ✓ VERIFIED | `config.rs` fields present (`pub accel/cpu/smp/memory`, `pub struct Dist { pub format }`); unit tests in the 11/11 live `cargo test -p xtask` pass this run |
| 13 | accel/cpu override chain flag → R9_ACCEL/R9_CPU env → TOML → tcg/cortex-a72 (D-06) | ✓ VERIFIED | clap args + `env::var` resolution in both QemuStep and BootTestStep (config.rs:290-291); TOML `accel = "tcg"`; never-hvf-by-default honored |
| 14 | QemuStep virt arm: single `-serial mon:stdio` + ELF `-kernel`; raspi arm untouched (P7/D-05) | ✓ VERIFIED | `main.rs` virt branch emits `mon:stdio` + raw ELF path; DistStep skips objcopy/gzip on `Some("elf")`; riscv64/x86_64 arms untouched (`git diff --stat ae4cd08~1..55ca7c4 -- riscv64/ x86_64/` empty — re-run this verification) |
| 15 | x0=0 ELF boot → fixed PA 0x40000000 DTB fallback at BOTH consumers (P1) | ✓ VERIFIED | `vminit.rs:152-153` substitutes pre-MMU with putstr marker; `main.rs:125` substitutes the VA twin (`KZERO + 0x4000_0000`); live log's first line is the marker and the banner's DTB line shows `0xffff800040000000` |
| 16 | virt kernel at 0x40080000 covered by identity + higher-half maps; kernel ranges start at boottext_pa, MMIO never Normal kernel text (P3/P4) | ✓ VERIFIED (with open caveat WR-01) | `vminit.rs:266-272` second 1 GiB block at PA 0x40000000 (virt-only); `base_physaddr()` = `boottext_pa` on virt, 0 on raspi. Caveat: review WR-01 (open) — the TTBR0 identity map is never torn down on virt, so the MMIO window stays Normal-cacheable/EL1-executable through TTBR0; the higher-half mitigation this truth asserts is real, the residual is a hardening gap (see Anti-Patterns/Warnings) |
| 17 | Early-boot PL011 debug console works pre-MMU on virt (D-02) | ✓ VERIFIED | `pre_mmu/util.rs` cfg-selected bodies; live log's pre-banner `vminit:` lines prove pre-MMU printing |
| 18 | virt arms GIC → timer → UART RX, then `trap::unmask_irq()` LAST (P5 ordering) | ✓ VERIFIED | `main.rs:221-245` order re-confirmed this run (gic::init → gic::enable(30) → timer::init → pl011::init_rx → gic::enable(33) → [dev_panic hook] → unmask_irq); zero `Unhandled interrupt`/`panicked` markers in any live or committed log |
| 19 | PL011 (INTID 33, SPI) gets ITARGETSR cpu0 write; timer PPI does not; 1023 acked without EOI (P6) | ✓ VERIFIED | `gic.rs:120` writes ITARGETSR only for `intid >= 32`; `main.rs` calls `enable(30,…)`/`enable(33,…)`; `trap.rs:149-152` skips EOI on `SPURIOUS_INTID` and EOIs everything else exactly once |
| 20 | `cargo xtask boot-test` builds the image it tests, captures before start, asserts in order, writes the log (D-11) | ✓ VERIFIED | Dispatch runs BuildStep then BootTestStep; socket `server=on,wait=on` holds guest start until connect (xtask/main.rs:900); both profiles PASSed live this run and wrote `target/boot-test-{virt,virtpanic}.log` |
| 21 | Canonical evidence committed, SHA-pinned to the producing tree (D-13) | ✓ VERIFIED | `evidence/{virt-boot.log, virtpanic-boot.log, EVIDENCE.md}` committed (harvey `1edf2b9d0`); EVIDENCE.md's SHA equals live `git -C ../r9 rev-parse HEAD` (re-checked this run); committed logs byte-identical to this verifier's fresh harness captures (`cmp` clean ×2, this run) |
| 22 | r9 docs describe the shipped reality (README invocation table + status; HowItWorks platform tree/boot contract/drivers) | ✓ VERIFIED | README:16 carries the `--config virt` row, :50 the boot-test row; `grep -c virtpanic README.md` = 0 (T-03-08 holds); HowItWorks covers the platform tree, the ELF+`-dtb` contract with the 0x40000000 fallback, GICv2/timer/PL011 drivers, unmask ordering, dev_panic |

**Score:** 22/22 truths verified (0 present-but-behavior-unverified). Truth 1's linux leg carries human pass evidence from the completed UAT; all other truths were machine-verified live in this run.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `../r9/flake.nix` | devShells.default + apps.build/boot-virt via rust-overlay | ✓ VERIFIED | substantive, all keys present, live-proven this run |
| `../r9/flake.lock` | pinned nixpkgs + rust-overlay | ✓ VERIFIED | committed; nodes re-read this run: nixpkgs `1c89083e451e`, rust-overlay `c85ccd343087`, single root lineage |
| `../r9/aarch64/src/platform/mod.rs` | cfg dispatch + raspi3b fallback | ✓ VERIFIED | present; riscv64 shape mirrored |
| `../r9/aarch64/src/platform/raspi3b/{devcons,mailbox,uartmini,uartpl011}.rs` | moved raspi drivers | ✓ VERIFIED | present at new paths, absent at old, builds green live this run |
| `../r9/aarch64/src/platform/raspi4b/mod.rs` | re-export shim | ✓ VERIFIED | thin shim; raspi4b build green live this run |
| `../r9/aarch64/Cargo.toml` | check-cfg block | ✓ VERIFIED | only Cargo.toml touched in the phase range; Cargo.lock untouched (`git diff --name-only` re-run) |
| `../r9/aarch64/src/trap.rs` | init/unmask split + IRQ dispatch + halt marker | ✓ VERIFIED | vbar-only init, `unmask_irq()`, spurious-1023 no-EOI, `kernel halted` marker all grepped this run |
| `../r9/xtask/src/config.rs` | Qemu.{cpu,smp,memory,accel} + Dist | ✓ VERIFIED | fields present; 11/11 unit tests live this run |
| `../r9/xtask/src/main.rs` | DistStep elf skip, QemuStep virt arm, BootTestStep | ✓ VERIFIED | all present; boot-test live-proven twice this run |
| `../r9/aarch64/lib/config_virt.toml` | virt platform as pure data | ✓ VERIFIED | full contents re-read: platform/machine/cpu/smp/memory/accel/dtb/dist all correct |
| `../r9/aarch64/lib/config_virtpanic.toml` | D-12 debug variant | ✓ VERIFIED | `dev = ["panic"]` at line 29; debug-artifact header |
| `../r9/aarch64/lib/virt-gicv2.{dts,dtb}` | committed DTB pair + regeneration header | ✓ VERIFIED | dts 10,818 B with `arm,cortex-a15-gic`; dtb 7,785 B, loaded at 0x40000000 in live runs |
| `../r9/aarch64/src/platform/virt/{mod,devcons,pl011,gic,timer}.rs` | virt platform drivers | ✓ VERIFIED | all present, DTB-discovered, wired through main9/trap |
| `../r9/aarch64/src/pre_mmu/{vminit,util}.rs` + `kmem.rs` | memory-map fixes + early uart | ✓ VERIFIED | DTB fallback twins + second identity block grepped this run |
| `../r9/README.md` + `HowItWorks.md` | docs current | ✓ VERIFIED | greps pass this run; virtpanic absent from README |
| `evidence/{virt-boot.log,virtpanic-boot.log,EVIDENCE.md}` | D-13 bundle | ✓ VERIFIED | committed, markers present (3 ticks / 1 echo / 31 register rows / 1 halt marker — re-counted this run), SHA-pinned, byte-reproduced live |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `flake.nix` | `rust-toolchain.toml` | `fromRustupToolchainFile` | ✓ WIRED | flake.nix:21; devshell resolves the pinned nightly (live builds this run) |
| `flake.nix` apps.build | xtask BuildStep | `writeShellApplication` + runtimeInputs | ✓ WIRED | `nix run .#build` ran bare this run, exit 0, ELF produced |
| `flake.nix` shellHook | xtask objcopy() | `OBJCOPY` export | ✓ WIRED | OBJCOPY export logic at flake.nix:29-31 (glob over toolchain rustlib) |
| `xtask/src/config.rs` platform key | `platform/mod.rs` | `--cfg platform="x"` | ✓ WIRED | virt/raspi builds both correct live this run |
| `main.rs` main9 | `platform/*/devcons.rs` | `platform::devcons::init(&dt)` | ✓ WIRED | banner prints through virt PL011 live this run |
| `config_virt.toml` load-address | `kernel.ld` `${LOAD-ADDRESS}` | config.rs substitution | ✓ WIRED | `load-address = '0xffff800040080000'` in the TOML; live log's physmap lines confirm |
| `vminit.rs` init_vm / `main.rs` main9 | DTB at fixed PA 0x40000000 | twin cfg-gated substitutions | ✓ WIRED | vminit.rs:152-153 + main.rs:125 grepped; marker + DTB line in live log |
| `trap.rs` IRQ branch | `gic.rs` ack/eoi | GICC_IAR/GICC_EOIR | ✓ WIRED | dispatch arms grepped; live ticks + echo prove the return path (multiple ticks require eret) |
| `timer.rs` | aarch64-cpu registers | CNTFRQ_EL0/CNTP_TVAL_EL0/CNTP_CTL_EL0 | ✓ WIRED | runtime frequency read at timer.rs:70-71; zero new crates (Cargo.lock untouched) |
| BootTestStep | qemu socket chardev | `server=on,wait=on` + UnixStream | ✓ WIRED | live captures this run are full-boot (60,146 / 61,369 bytes) |
| `config_virtpanic.toml` dev key | `main.rs` `#[cfg(dev_panic)]` hook | `dev = ["panic"]` → `--cfg dev_panic` | ✓ WIRED | hook lexically before `trap::unmask_irq()` (main.rs:239-245); live panic log deterministic (0 tick lines) |
| `evidence/` | r9 `target/boot-test-*.log` | D-13 copies | ✓ WIRED | byte-identical to this run's fresh captures (`cmp` ×2) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| virt devcons/pl011 | UART register window | DTB `find_compatible("arm,pl011")` → translated reg → `map_device_register` | Yes — live banner/ticks/echo flow through it this run | ✓ FLOWING |
| gic.rs | GICD/GICC windows | DTB (`arm,cortex-a15-gic` or_else `arm,gic-400`) → translated regs | Yes — live IRQ delivery this run | ✓ FLOWING |
| timer.rs | tick count, CNTFRQ | `CNTFRQ_EL0.get()` at runtime; tick counter | Yes — `tick` lines live this run | ✓ FLOWING |
| print_machine_info | Memory/found lines | `port::fdt` walks of the live DTB | Yes — matches the committed DTB's contents | ✓ FLOWING |
| BootTestStep assertions | captured serial bytes | UnixStream from qemu socket chardev | Yes — evidence logs are full boot captures, byte-reproduced this run | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| SC#2/3/4 harness gate | `nix develop -c cargo xtask boot-test --arch aarch64 --config virt` | PASS, 60,146 B captured | ✓ PASS (this run) |
| SC#5 harness gate | `nix develop -c cargo xtask boot-test --arch aarch64 --config virtpanic` | PASS, 61,369 B captured | ✓ PASS (this run) |
| Evidence integrity (virt) | `cmp target/boot-test-virt.log evidence/virt-boot.log` | byte-identical | ✓ PASS (this run) |
| Evidence integrity (virtpanic) | `cmp target/boot-test-virtpanic.log evidence/virtpanic-boot.log` | byte-identical | ✓ PASS (this run) |
| xtask unit tests | `nix develop -c cargo test -p xtask` | 11 passed, 0 failed | ✓ PASS (this run) |
| raspi3b compile | `nix develop -c cargo xtask build --arch aarch64` | Finished | ✓ PASS (this run) |
| raspi4b compile | `nix develop -c cargo xtask build --arch aarch64 --config raspi4b` | Finished | ✓ PASS (this run) |
| SC#1 bare app | `nix run .#build` (ELF pre-deleted, no devshell) | exit 0, ELF produced | ✓ PASS (this run) |
| Evidence SHA currency | `git -C ../r9 rev-parse HEAD` vs EVIDENCE.md | identical (`55ca7c4…`); tree clean | ✓ PASS (this run) |
| riscv64/x86_64 untouched | `git diff --stat ae4cd08~1..55ca7c4 -- riscv64/ x86_64/` | empty | ✓ PASS (this run) |
| D-10 linux leg | human-run on a linux host per 03-UAT.md test 1 | pass (UAT complete) | ✓ PASS (human) |
| D-11/D-13 evidence approval | human review per 03-UAT.md test 2 | pass (UAT complete) | ✓ PASS (human) |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| R9-01 | 03-02, 03-03, 03-04, 03-05, 03-06 | qemu `-M virt` aarch64 platform alongside raspi3b/4b: ELF entry contract, DTB at 0x40000000, cortex-a72, memory map, GICv2, PL011 console, generic timer — proven by kernel log output | ✓ SATISFIED | every element live-proven this run; raspi builds remain green (alongside); raspi3b qemu *runtime* boot is a pre-existing upstream qemu-11.1.2 defect, A/B-proven not-a-regression, human-accepted (deferred-items.md #1). REQUIREMENTS.md marks R9-01 Complete (line 68) |
| R9-05 | 03-01 | r9 carries its own flake.nix (pinned toolchain + llvm-tools, qemu, cargo xtask) — devshell + apps, reproducible standalone | ✓ SATISFIED | flake.nix/flake.lock verified live; darwin build + bare app live-proven this run; **linux leg human-passed (03-UAT.md test 1)**. REQUIREMENTS.md marks R9-05 Complete (line 72) |

Both phase requirement IDs are accounted for; REQUIREMENTS.md maps no other IDs to Phase 3 — no orphans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `aarch64/src/platform/raspi3b/mailbox.rs` | 86, 322 | `FIXME` ×2 | ℹ️ Info | pre-existing upstream markers — re-confirmed this run: identical count (2) at pre-phase base 33a9273; carried verbatim by the 03-02 git-mv; not phase debt |
| `xtask/src/main.rs` | 633-634 | `FIXME` ×2 | ℹ️ Info | pre-existing — identical count (2) at base 33a9273, untouched by the phase |
| `aarch64/src/platform/virt/pl011.rs` | 18 | clippy `constant UART_INTID is never used` | ℹ️ Info | doc constant; main.rs/trap.rs use the literal 33 (same family as open WR-05) |
| `aarch64/src/pre_mmu/vminit.rs` | 254-280 | WR-01 (open review warning): TTBR0 identity map never torn down on virt — GIC/PL011 stay Normal-cacheable + EL1-executable through TTBR0 | ⚠️ Warning | undercuts the P3 mitigation's full intent; benign under qemu TCG today; does not falsify any SC (all boots/ticks/echo/panic proven live this run). Recommend Phase 4 (userspace VM work) tear down or PXN/Device-harden the identity root |
| `xtask/src/main.rs` | 851-853 | WR-02 (open): boot-test rejects only non-aarch64; a raspi config burns 300 s to a misleading failure | ⚠️ Warning | harness usability wart; both intended configs (virt, virtpanic) proven correct live this run |
| `aarch64/src/platform/raspi3b/uartpl011.rs` | 113-126 | WR-03 (open): `gpiosetpull` writes GPIO offsets through the PL011 window | ⚠️ Warning | pre-existing, latent (raspi devcons uses MiniUart; driver is `#[allow(dead_code)]`); moved-not-modified this phase |
| `aarch64/src/pre_mmu/vminit.rs` | 427-443 | WR-04 (open): `map_phys_range` swallows mid-range `map_to` failures | ⚠️ Warning | pre-existing error-path weakness; the virt path demonstrably maps cleanly (boots proven live) |
| `aarch64/src/platform/virt/timer.rs` | 33-58 | WR-05 (open): DTB-rediscovered INTID advisory-only; enable/dispatch hardcode 30 | ⚠️ Warning | cosmetic on the committed DTB (ticks fire live); false-confidence critique stands for future drift |
| review IN-01..IN-10 | various | dead asm reads, redundant zeroing, comment/impl mismatch (IN-08), HowItWorks figures (IN-09 — the "0–2MiB" claim confirmed still present at HowItWorks.md:25 this run), empty-OBJCOPY edge (IN-10), etc. | ℹ️ Info | all open in 03-REVIEW-DISPOSITION.md; none touch phase success criteria |

**Review weighing (WR-01..WR-05 vs. phase SCs):** 0 critical findings; all five warnings are quality/hardening findings on code that provably delivers every SC (re-proven live this run). None is a blocker for this phase's goal. All remain `open` in the disposition for later triage; WR-01 deserves explicit consideration in Phase 4 planning (its userspace-VM work touches the same page-table lifetime question).

### Substituted Gate (human-approved, for the record)

03-03's written raspi3b qemu boot-smoke gate ("banner within 120 s") is unmeetable on qemu 11.1.2 for a **pre-existing upstream defect** (guest data-abort at FAR `0xfe215004` — rpi4 AUX_ENABLES MMIO touched on the raspi3b machine; A/B-proven identical at pre-plan xtask `6a309ae` + pre-03-02 kernel `ae4cd08` vs post-plan HEAD, with byte-identical raspi argv). The human accepted the A/B no-regression evidence as a substitute gate (decision A, 2026-11-01; `deferred-items.md` #1). This verifier re-confirms: raspi argv plumbing untouched (git diff), raspi builds green live this run, and the defect predates the phase — non-regression, which is what the "raspi keeps working alongside" constraint requires, is proven. The upstream fix remains deferred and is **not** claimed by any later milestone phase; it should stay visible in backlog triage.

### Human Verification Required

None. The two items the prior report routed here are resolved with human pass evidence recorded in `03-UAT.md` (`status: complete`, total: 2, passed: 2, issues: 0; commit `cfd692202`):

1. **D-10 linux leg of SC#1 (R9-05)** — human ran the leg on a linux host from r9 `55ca7c4`: devshell builds the kernel ELF, `nix run .#build` works bare, toolchain pins resolve. **Result: pass.**
2. **D-11/D-13 evidence-bundle phase-gate approval** — human reviewed and approved the committed evidence bundle against SC#1–5. **Result: pass.** (Mechanical integrity was machine-proven again this run: both committed logs reproduced byte-for-byte from live harness runs at the pinned HEAD.)

### Gaps Summary

No gaps. All five roadmap success criteria are behaviorally proven: SC#1 by the live darwin leg (this run) plus the human-passed linux leg (UAT), SC#2–SC#5 by this verifier's own live harness runs at the pinned r9 HEAD, with the committed D-13 evidence reproduced byte-for-byte. Both phase requirements (R9-01, R9-05) are satisfied and marked Complete in REQUIREMENTS.md. The phase goal — console, interrupts, and timer proven on the milestone's actual target machine before userspace-exec work — is achieved.

Five open review warnings (WR-01..WR-05) and ten info findings are recorded for later-phase triage in 03-REVIEW-DISPOSITION.md; none negates a success criterion. One pre-existing upstream raspi3b-on-qemu defect is human-accepted as a substituted gate and tracked in deferred-items.md.

---

_Verified: 2026-11-02T01:49:14Z_
_Verifier: the agent (gsd-verifier)_
