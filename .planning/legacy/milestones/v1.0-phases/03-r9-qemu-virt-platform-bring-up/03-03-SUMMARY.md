---
phase: 03-r9-qemu-virt-platform-bring-up
plan: "03"
subsystem: infra
tags: [rust, xtask, qemu, toml, config-schema, dist, elf, aarch64, virt, override-chain, tcg]

requires:
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 01)
    provides: "../r9 flake devshell — every cargo test/build in this plan ran through `nix develop -c`"
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 02)
    provides: "cfg-gated aarch64 platform tree — the raspi regression exercise ran against the restructured tree (declared sequential edge)"
provides:
  - "../r9/xtask/src/config.rs — Qemu gains cpu/smp/memory/accel Option fields; NEW Dist struct { format } + Configuration.dist; apply_qemu_config emits -accel/-cpu/-smp/-m after -M when present, nothing when absent (D-07/D-05)"
  - "QemuOverrides + apply_to_qemu_step_with_overrides — per-invocation accel/cpu beats TOML at the emission layer (D-06)"
  - "../r9/xtask/src/main.rs — DistStep config-aware (elf skips objcopy+gzip); QemuStep aarch64 virt arm (single -serial mon:stdio + ELF -kernel); --accel/--cpu flags + R9_ACCEL/R9_CPU env, precedence flag > env > TOML > tcg/cortex-a72 default"
  - "5 xtask unit tests pinning argv order, backward-compat absence, dist parsing, and override precedence"
affects: [03-04, 03-05, 03-06, phase-05-int-04]

# Actuals (#2632) — same scale as the plan's estimateTokens (chars/4 over realized diff).
# Cross-repo plan: commits = 3 r9 code commits (26ec344, 5452e33, 11196de) + 1 harvey docs commit.
# r9 leg: 6a309ae -> 11196de (diff 18,599 chars → 4,650 tokens). Harvey leg measured from ledger.
actuals:
  tokens: 4650
  tasks: 2
  commits: 4

# Commit ledger (#3968) — cross-repo plan: harvey base from the persisted ledger;
# the r9 leg is 6a309ae..11196de (3 commits). commits: 4 = 3 r9 + this harvey docs commit.
# plan_head_after names the pre-amend docs hash; the amend only added these ledger fields.
plan_head_before: 91d2237270b53f2ac9fbffc2a507438efdd15dea
plan_head_after: 2e905247ecb4ae372c5a4bfae595729a89119e53

tech-stack:
  added: []
  patterns:
    - "qemu invocation as data (D-07): new platforms become TOML keys consumed by apply_qemu_config, not Rust edits in QemuStep"
    - "per-invocation override chain (D-06): CLI flag > env (R9_ACCEL/R9_CPU) > TOML > built-in default, resolved before Command construction and applied through QemuOverrides at the emission layer"
    - "dist format as config: [dist] format = \"elf\" skips the objcopy+gzip flat-image pipeline; absent key is the flat-gzip default (D-05), keeping existing TOMLs byte-identical"

key-files:
  created: []
  modified:
    - "../r9/xtask/src/config.rs"
    - "../r9/xtask/src/main.rs"

key-decisions:
  - "tcg/cortex-a72 built-in defaults fire only on the virt arm — existing raspi TOMLs sprout no new argv (backward-compat guard holds end-to-end, verbose-probe verified); darwin runs TCG this phase because HVF rejects GICv2 on qemu 11.1.2 (research P2), hvf remains selectable for a future GICv3 driver"
  - "Old apply_to_qemu_step free function removed outright instead of kept as a delegating shim — xtask-internal API with one call site; a compat wrapper would be dead code"
  - "Written raspi boot-smoke gate satisfied by substituted A/B evidence (human decision A, 2026-11-01): identical pre-existing raspi3b-on-qemu-11.1.2 crash at FAR 0xfe215004 pre-plan and post-plan with byte-identical raspi argv proves NO REGRESSION; the upstream defect is deferred, not this plan's scope"
  - "darwin process-group isolation: setsid is absent on macOS — used the plan's own named alternative (backgrounded subshell + pkill -f on the qemu binary with the raspi3b machine arg), bracket-fixed against pkill self-match"

requirements-completed: [R9-01]
# R9-01 is shared by plans 03-02…03-06; the shared-ID gate keeps it open until the
# last declaring plan completes. This plan's copy of the ID is verbatim from the
# plan frontmatter; REQUIREMENTS.md stays Pending until 03-06 finishes.

coverage:
  - id: D1
    description: "D-07/D-05 schema extension: Qemu.{cpu,smp,memory,accel} + Dist.{format} parse from TOML; apply_qemu_config emits -accel/-cpu/-smp/-m in order after -M when present and nothing when absent (backward-compat regression guard for every existing TOML)"
    requirement: R9-01
    verification:
      - kind: unit
        ref: "cd ../r9 && nix develop -c cargo test -p xtask — 5 passed (full-keys argv order, minimal-keys backward compat, dist parse, override precedence)"
        status: pass
      - kind: other
        ref: "static greps: pub accel/cpu/smp/memory fields, pub struct Dist, pub dist: Option<Dist> present in config.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "DistStep config-aware (elf skips objcopy+gzip; raspi path byte-identical); QemuStep virt arm (single -serial mon:stdio, ELF -kernel); --accel/--cpu + R9_ACCEL/R9_CPU override chain in contractual precedence; riscv64/x86_64 arms untouched; no new crate deps"
    requirement: R9-01
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask — green after Task 2"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask build --arch aarch64 — green through the modified plumbing"
        status: pass
      - kind: other
        ref: "static greps: Some(\"elf\"), R9_ACCEL, R9_CPU, \"virt\" present in main.rs; verbose-probe diff shows byte-identical raspi argv with no overrides set"
        status: pass
    human_judgment: false
  - id: D3
    description: "Raspi3b boot-smoke regression through the modified QemuStep (written gate: banner 'r9 from the Internet' within 120s)"
    requirement: R9-01
    verification:
      - kind: other
        ref: "SUBSTITUTED (human decision A, 2026-11-01): controlled A/B — pre-plan xtask 6a309ae + pre-03-02 kernel ae4cd08 vs post-plan HEAD produce the IDENTICAL pre-existing raspi3b-on-qemu-11.1.2 guest data-abort at FAR 0xfe215004 with byte-identical raspi argv → NO REGRESSION proven; written banner gate unmeetable on this host for a pre-existing upstream reason"
        status: unknown
    human_judgment: true
    rationale: "The written banner gate cannot pass on qemu 11.1.2 for a pre-existing upstream raspi3b defect (rpi4 AUX_ENABLES MMIO base touched on the raspi3b machine), unrelated to this plan. The human reviewed the A/B evidence and accepted it as proof of no regression, substituting for the written gate (decision A, 2026-11-01). The upstream fix is deferred — see deferred-items.md."

duration: 30 min (incl. A/B evidence gathering and human-decision checkpoint)
completed: 2026-11-01
status: complete
---

# Phase 03 Plan 03: xtask qemu/dist plumbing (virt platform as data) Summary

**xtask now treats the qemu invocation as data, not code (D-07): the `[qemu]` TOML section carries cpu/smp/memory/accel alongside machine/dtb, a new `[dist] format` key lets ELF-platforms skip the objcopy+gzip flat-image pipeline entirely (D-05), QemuStep's aarch64 arm routes virt to a single `-serial mon:stdio` with the cargo-built ELF as `-kernel` (P7), and the D-06 override chain (flag > env > TOML > tcg/cortex-a72) is plumbed end-to-end — with the raspi path byte-identical and the raspi regression proven by human-accepted A/B evidence against a pre-existing upstream qemu-11.1.2 raspi3b defect.**

## Performance

- **Duration:** ~30 min (code ~21 min; A/B evidence + human-decision checkpoint thereafter)
- **Started:** 2026-11-01T22:43Z (after 03-02 close-out)
- **Completed:** 2026-11-01T23:14Z
- **Tasks:** 2
- **Files modified:** 2 (both in sibling repo `../r9`; harvey side carries only this SUMMARY + state files per D-13)

## Accomplishments

- **D-07 schema extension:** `Qemu` gains `cpu: Option<String>`, `smp: Option<u32>`, `memory: Option<String>`, `accel: Option<String>` alongside machine/dtb; new `Dist { pub format: Option<String> }` + `Configuration.dist`; `apply_qemu_config` emits `-accel`, `-cpu`, `-smp`, `-m` in that order after `-M` when the keys are present and emits nothing when absent — the minimal-TOML unit test is the regression guard for every existing config.
- **D-05 dist format awareness:** `DistStep` gained a `config` field via the `BuildStep` ctor idiom; `[dist] format = "elf"` skips the objcopy and gzip invocations for aarch64 and prints a one-line note that the ELF is the boot image; the absent/flat-gzip path is byte-identical to before.
- **P7 virt qemu arm:** `QemuStep`'s aarch64 arm branches on `[config] platform == Some("virt")` — virt gets `-nographic`, a single `-serial mon:stdio` (PL011 is serial0 on virt; the raspi null+second-serial pair is wrong there), and `-kernel` pointing at the raw ELF `target/{target}/{dir}/aarch64`; the raspi else-arm keeps today's dual-serial + gzipped-flat shape untouched, and `-d int` stays on both arms for bring-up visibility. riscv64/x86_64 arms untouched.
- **D-06 override chain:** `--accel`/`--cpu` clap args on the qemu subcommand plus `R9_ACCEL`/`R9_CPU` env via the `env_or` idiom; precedence flag > env > TOML > built-in `tcg`/`cortex-a72`, resolved before `Command` construction and applied through `QemuOverrides` at the emission layer. The built-in defaults fire only on the virt arm, so existing raspi TOMLs sprout no new argv — backward-compat holds end-to-end (verbose-probe diff of the constructed raspi command: byte-identical with no overrides set).
- **5 unit tests in `config.rs`** pin the contract: full-keys argv order, minimal-keys backward compat, dist format parsing (`Some("elf")` vs absent), and override precedence. `cargo test -p xtask` green in the 03-01 devshell; `cargo xtask build --arch aarch64` green through the modified plumbing.
- **No new crate dependencies** (threat T-03-SC vacuous, per the frontmatter prohibition).

## Task Commits

Each task was committed atomically (repo: `../r9`, branch `main`):

1. **Task 1 (RED): failing tests for qemu/dist schema extension** — `26ec344` (test)
2. **Task 1 (GREEN): [qemu] cpu/smp/memory/accel + [dist] format + override plumbing; 5 unit tests green** — `5452e33` (feat)
3. **Task 2: DistStep config-aware (D-05 elf skip); QemuStep virt arm (P7 `-serial mon:stdio` + ELF `-kernel`); --accel/--cpu + R9_ACCEL/R9_CPU chain (D-06)** — `11196de` (feat)

**Plan metadata:** harvey docs commit (see below; `docs(03-03): complete xtask qemu/dist plumbing plan`)

## Files Created/Modified

- `../r9/xtask/src/config.rs` (modified) — extended `Qemu` struct, new `Dist` struct + `Configuration.dist`, extended argv emission, `QemuOverrides` + `apply_to_qemu_step_with_overrides`, `#[cfg(test)]` module with 5 tests
- `../r9/xtask/src/main.rs` (modified) — `DistStep.config` + elf-format skip; `QemuStep` aarch64 virt branch; `--accel`/`--cpu` clap args + env override resolution

(Harvey repo: only this SUMMARY + state files, per D-13.)

## Decisions Made

- **Built-in defaults scoped to the virt arm** — the `tcg`/`cortex-a72` defaults apply only when the virt branch is taken, so existing raspi TOMLs (which set neither key) produce byte-identical argv. This is what makes the backward-compat guarantee hold end-to-end rather than only at the parsing layer.
- **`apply_to_qemu_step` removed, not shimmed** — the pre-existing free function was replaced by `apply_to_qemu_step_with_overrides`; with a single in-crate call site, a delegating compat wrapper would be dead code (see deviation 3).
- **TCG on darwin this phase** — HVF rejects GICv2 on qemu 11.1.2 (research P2, verified); the `hvf` path remains selectable for a future GICv3 driver. Recorded in a code comment per the plan.
- **Gate substitution (decision A, human-approved 2026-11-01)** — the written raspi3b banner-smoke verify gate was satisfied by substituted A/B evidence (see Issues Encountered).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `setsid` absent on darwin — used the plan's own named alternative**
- **Found during:** Task 2, raspi boot-smoke verify setup
- **Issue:** the written verify command backgrounds qemu via `setsid`, which does not exist on macOS
- **Fix:** the written command itself names the fallback ("use setsid-style isolation or pkill -f on the qemu binary name with the raspi3b machine arg as the match") — ran qemu backgrounded from a subshell with output redirected to `target/boot-logs/raspi-regression.log` and used `pkill -f` for teardown
- **Files modified:** none (verify-harness adaptation only)
- **Verification:** qemu launched and was killed by the pkill teardown; log captured
- **Committed in:** n/a (no code change)

**2. [Rule 1 - Bug] `pkill -f` self-match bracket fix**
- **Found during:** Task 2, smoke-run teardown
- **Issue:** the naive pattern `pkill -f "qemu-system-aarch64.*raspi3b"` can match the invoking shell's own argv (the pattern text appears in the shell command line), killing the harness itself
- **Fix:** bracket character-class idiom — `pkill -f "[q]emu-system-aarch64.*raspi3b"` — matches qemu but never the literal pattern string in any shell's argv
- **Files modified:** none (verify-harness adaptation only)
- **Verification:** teardown killed only the qemu process
- **Committed in:** n/a (no code change)

**3. [Rule 1 - Dead code] removed the superseded `apply_to_qemu_step` instead of keeping a delegating shim**
- **Found during:** Task 2 implementation (override plumbing)
- **Issue:** `QemuOverrides` made the old `apply_to_qemu_step(cmd, config)` signature obsolete; an initial backward-compat wrapper (delegating with `QemuOverrides::default()`) was dead on arrival — the function is xtask-internal with exactly one call site, which the task was already editing
- **Fix:** removed the old function outright; the single call site and the test module use `apply_to_qemu_step_with_overrides` directly
- **Files modified:** ../r9/xtask/src/config.rs, ../r9/xtask/src/main.rs
- **Verification:** `cargo test -p xtask` green; no remaining references (`grep` confirms only the `_with_overrides` name exists)
- **Committed in:** `5452e33` / `11196de` (part of the task commits)

### Checkpoint-approved substitution (not an auto-fix)

**4. [Human decision A, 2026-11-01] written raspi3b banner-smoke gate satisfied by substituted A/B evidence**
- **Found during:** Task 2, raspi boot-smoke verify gate
- **Issue:** the gate polls up to 120s for the banner "r9 from the Internet" from a raspi3b boot under qemu; the guest never reaches the banner on this host — it takes a data abort at FAR `0xfe215004` during early boot (see Issues Encountered). A controlled A/B proved the failure is byte-for-byte identical at pre-plan xtask `6a309ae` with pre-03-02 kernel `ae4cd08` — i.e., pre-existing, not caused by this plan — and the constructed raspi qemu argv is byte-identical pre/post plan
- **Fix:** none (not this plan's scope); the checkpoint presented the A/B evidence and the human selected **A — accept A/B evidence** as proof of NO REGRESSION, substituting for the written gate. The upstream defect is recorded in `deferred-items.md`
- **Files modified:** none
- **Verification:** A/B runs at both pins; verbose-probe argv diff empty
- **Committed in:** n/a

---

**Total deviations:** 3 auto-fixed (1 Rule 3, 2 Rule 1) + 1 human-approved gate substitution
**Impact on plan:** No scope creep and no code change beyond the tasks; the two harness adaptations execute the written verify's own named alternatives. The gate substitution preserves the gate's intent (prove the raspi path unregressed) with stronger evidence than the banner wait could provide on this host.

## Issues Encountered

**PRE-EXISTING upstream defect — raspi3b machine broken on qemu 11.1.2 (deferred, NOT this plan's scope).** The raspi3b boot under `qemu-system-aarch64` 11.1.2 dies with a guest data abort at FAR `0xfe215004` early in boot — the kernel touches the **rpi4** AUX_ENABLES MMIO base while running on the raspi3b machine (candidate suspect: `init_early_uart_rpi4` hardcoded rpi4 constants). Controlled A/B evidence:

- **A (pre-plan):** xtask at `6a309ae` (pre-03-03) + kernel at `ae4cd08` (pre-03-02) → identical abort at FAR `0xfe215004`.
- **B (post-plan):** xtask/kernel at post-plan HEAD → identical abort, same FAR.
- Raspi qemu argv byte-identical between the two pins (verbose-probe diff empty).

Conclusion: NO REGRESSION from this plan (or from 03-02); the written banner-smoke gate is unmeetable on this host for a reason that predates both. The human accepted the A/B evidence as a substitute for the written gate (decision A, 2026-11-01). The defect is recorded in `deferred-items.md` for a later fix; the virt platform (this phase's actual target) is unaffected — it boots `-M virt` with the PL011 at its own address.

No other issues. The declared sequential edge after 03-02 held: the smoke exercise ran against the restructured platform tree, so the argv-identity evidence covers the moved raspi drivers as well.

## User Setup Required

None — no external service configuration required. (D-10's linux-host verification legs remain queued for end-of-phase UAT from 03-01; this plan adds no new ones.)

## Next Phase Readiness

- **03-04 can declare `config_virt.toml` as pure data and boot through `cargo xtask qemu --config virt`** — the full invocation shape (`-M virt,gic-version=2 -accel tcg -cpu cortex-a72 -smp 1 -m 1024M -dtb …`, ELF `-kernel`, single `-serial mon:stdio`) is config-driven before the virt platform module exists. This was the plan's purpose (success criterion 2).
- **Override chain live:** `--accel kvm`/`R9_ACCEL=kvm` on a linux-aarch64 host resolves through the documented precedence for D-10's manual accel-matrix runs; TCG remains the always-safe default.
- **Watch item:** the deferred raspi3b/qemu-11.1.2 defect means raspi smoke gates in later plans (03-05/03-06) cannot rely on a banner wait on this host either — reuse the A/B/argv-identity pattern or fix the deferred item first.
- R9-01 remains open (shared by plans 03-02…03-06; the shared-ID gate marks it when the last declaring plan completes).

---
*Phase: 03-r9-qemu-virt-platform-bring-up*
*Completed: 2026-11-01*

## Self-Check: PASSED

- FOUND: ../r9/xtask/src/config.rs
- FOUND: ../r9/xtask/src/main.rs
- FOUND: .planning/phases/03-r9-qemu-virt-platform-bring-up/03-03-SUMMARY.md
- FOUND: r9 commit 26ec344 (`test(03-03): add failing tests for qemu/dist config schema extension`, branch main)
- FOUND: r9 commit 5452e33 (`feat(03-03): extend qemu/dist TOML schema (D-07/D-05) with argv emission`, branch main)
- FOUND: r9 commit 11196de (`feat(03-03): config-aware DistStep, virt qemu arm, accel/cpu override chain (D-05/D-06/P7)`, branch main)
