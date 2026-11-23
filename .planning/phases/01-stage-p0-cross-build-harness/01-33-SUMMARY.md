---
phase: 01-stage-p0-cross-build-harness
plan: 33
subsystem: cross-build-harness
status: halted
tags: [witness, kvm, stop-b2, gate, drawterm, expect, teardown, q35churnshell, boot-iso, halt, harn-01, harn-04, harn-06, harn-07]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-18's moved drawterm driver and quick gate; 01-20's PROBE-FLIP-SHA/PROBE-FLIPBACK-SHA; 01-22's stop-B witness steps and FAIL record; 01-31's x86_64 gate apps; 01-32's rio/XTEST evidence"
provides:
  - "gates/x86_64/drawterm-gate.expect: the qemu-scoped, post-spawn-fast-fail registration; a final teardown that cannot kill the driver or mask a green verdict"
  - "witness/run-witness.sh: plain boot-iso build before --rebuild; realised harvey-reference-amd64 before the cpuauth compares; fixed-driver overlay for the probe/flipback historical clones"
  - "xtask: q35churnshell rides the q35shell chain stages with only the designed fauth line exempt; the loud-unimplemented line names its caller"
  - "q35churnshell: PASS under Mac TCG and in the stop-B2 KVM profiles (9/9)"
  - "witness/WITNESS-LOG.md + evidence/stop-b2/: the seven stop-B2 verdicts, trimmed evidence, and the pending boot-iso decision"
affects: [01-23, 01-24, 01-25, 01-26, 01-27, 01-28, 01-31, HARN-01, HARN-04, HARN-06, HARN-07]

actuals:
  tokens: 12383
  tasks: 4
  commits: 4

plan_head_before: beacf143eb4ed8099a8b5dba54c08fa9fa0e3b77
plan_head_after: c1199243835350de9e0c7771aa9c41de33bf5191

tech-stack:
  added: []
  patterns:
    - "A teardown identity sweep must exclude its own caller chain: pgrep -f matches the driver's argv and the run tag in its environment"
    - "A success-path cleanup is hygiene, never an assertion: catch it, warn on failure, and print the verdict after it"
    - "A probe clone at a historical SHA inherits that SHA's harness; overlay the fixed driver so the probe's own subject stays the only historical variable"
    - "A real-chain profile owns only ONE narrow negative-scan exemption: `syscall 10 unimplemented`, the designed FAUTH probe"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/q35churnshell-tcg-mac.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/env.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/secret-scan.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/survivors.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/kernel.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/profiles.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/q35churnshell.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/gate.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/gate-neg.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/probe.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/flipback.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/cpuauth-ref.log
  modified:
    - gates/x86_64/drawterm-gate.expect
    - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
    - .planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md
    - xtask/src/gate.rs
    - xtask/src/main.rs
    - x86_64/src/syscall.rs

key-decisions:
  - "The q35churnshell syscall-10 caller is Harvey's unchanged boot path: `syscall 10 unimplemented (pid 1 boot)` and `(pid 1 init)`. syscall 10 = FAUTH, loud-None by design (the measured-set rule); the fix is the boot-test profile registration (chain stages + the one-line fauth exemption), not a FAUTH handler."
  - "The gate's stop-B2 failure is the final teardown, but the root cause is the identity sweep matching its OWN driver (`pgrep -f` sees `drawterm-gate.expect` and the run tag in its environment) and SIGTERMing the expect process before ALL STAGES PASS — not an unguarded variable read. The requested guards were added AND the sweep now excludes its caller chain."
  - "Probe and flipback did NOT share the gate's teardown defect: their throwaway clones at PROBE-FLIP-SHA/PROBE_FLIPBACK-SHA carry the pre-01-33 driver and abort at `expect_before -i $spawn_id` (line 336). The witness step now overlays this checkout's fixed driver; the probe image and swapped ps stay at the exact SHA."
  - "The kernel step now reaches the boot-iso fixed-output check (the precondition fix worked) and finds `specified sha256-TIzw...` vs `got sha256-xFt7...`. This is the long-owed Linux kernel-byte check, not a harness defect. Options (a) re-derive/commit the outputHash, (b) investigate cross-builder reproducibility, (c) re-scope the check, are recorded as owner-pending; none is chosen."
  - "The plan is HALTED: stop-B2 is 3 PASS / 4 non-zero (gate, probe, flipback, kernel). No KVM verdict is claimed for HARN-01/HARN-04; the stop-B requirement stays open and no later plan started."

coverage:
  - id: D1
    description: "The drawterm driver registers its fast-fail block against the qemu spawn id after the spawn, with a committed ordering guard"
    verification:
      - kind: test
        ref: "cargo test -p xtask gate::tests::driver_fast_fail_registers_after_the_qemu_spawn"
        status: pass
    human_judgment: true
    rationale: "The KVM behavioural proof is the pending targeted rerun; the stop-B2 gate leg reached every stage but did not print ALL STAGES PASS."
  - id: D2
    description: "step_kernel builds boot-iso plainly before --rebuild; step_cpuauth_ref realises harvey-reference-amd64 before reading members"
    verification:
      - kind: other
        ref: "stop-B2 return: kernel.log reached the boot-iso FOD check; cpuauth-ref.log shows CPUAUTH-REF-REALISED and both compares exit=0"
        status: pass
    human_judgment: false
  - id: D3
    description: "q35churnshell's syscall-10 caller is identified and the profile passes under TCG on the Mac and under KVM in the stop-B2 profiles"
    verification:
      - kind: test
        ref: "cargo xtask boot-test --arch x86-64 --config q35churnshell --accel tcg --cpu qemu64 -> PASS; stop-B2 profiles q35churnshell exit=0 (9/9 PASS)"
        status: pass
    human_judgment: false
  - id: D4
    description: "A passing gate leg prints ALL STAGES PASS and exits 0 through the final teardown"
    verification:
      - kind: test
        ref: "cargo test -p xtask gate::tests::driver_final_teardown_cannot_mask_a_passing_leg"
        status: pass
    human_judgment: true
    rationale: "The KVM teardown proof is the pending targeted rerun; the stop-B2 gate.log shows the reproduced defect."
  - id: D5
    description: "The stop-B2 verdicts and trimmed evidence are recorded honestly"
    verification:
      - kind: other
        ref: "witness/WITNESS-LOG.md has 7 stop-B2 VERDICT lines; evidence/stop-b2/ holds summary.tsv, env.txt, secret-scan.txt (clean) and each step's decisive log"
        status: pass
    human_judgment: false
---

# Phase 1 Plan 33: Stop-B fixes — driver registration, witness preconditions, and the q35churnshell syscall gap

**One-liner:** The moved drawterm driver now reaches its real assertions with a qemu-scoped fast-fail and a teardown that cannot kill a green leg; the boot-iso and reference preconditions are fixed; q35churnshell's FAUTH caller is named and the profile passes under TCG and KVM; stop-B2 returned 3 PASS / 4 non-zero, so the plan is halted on a targeted rerun and the boot-iso hash decision.

## Performance

- **Duration:** 55 min (2026-11-23T15:44Z → 2026-11-23T16:39Z)
- **Tasks:** 4 (Tasks 1–2 executed, Task 3 owner rerun, Task 4 verdict recording)
- **Files changed:** 19 (6 source/harness + 13 evidence)

## Accomplishments

1. **Driver fast-fail registration (Task 1).** `expect_before` moved after the qemu `spawn`, scoped `-i $qemu_sid`; the rio/probe assertions are unchanged. A static guard (`driver_fast_fail_registers_after_the_qemu_spawn`) fails if the old form returns.
2. **Witness step preconditions (Task 1).** `step_kernel` runs a plain `nix build .#boot-iso` with an output-exists check before `--rebuild` (`KERNEL-BOOTISO-BUILD-FAIL` on the prior build). `step_cpuauth_ref` realises `.#harvey-reference-amd64` first and exports the printed path (`CPUAUTH-REF-REALISE-FAIL` on a failed build).
3. **q35churnshell (Task 2).** The process-tagged line names the callers — `(pid 1 boot)` and `(pid 1 init)` — both Harvey's unchanged FAUTH probe (syscall 10, loud-None by design). `xtask` now routes q35churnshell through the q35shell chain stages and exempts only that line. Mac TCG run: PASS; stop-B2 KVM profiles: 9/9 PASS including q35churnshell.
4. **Stop-B2 returned (Task 3).** `gate_neg`, `profiles` and `cpuauth_ref` PASS. The gate's positive leg passed every stage (both runs) but the final teardown killed the expect driver before `ALL STAGES PASS`; probe/flipback aborted in their historical clones' pre-01-33 driver; the kernel step reached the boot-iso fixed-output check and found a hash mismatch.
5. **Teardown + rerun readiness (continuation).** The identity sweep excludes its own caller chain; cleanup guards every pid read, catches the sweep/redaction, and the final success cleanup is caught with a teardown warning. The probe/flipback steps overlay the fixed driver into their clones. Static guards pin both properties.
6. **Verdicts recorded honestly (Task 4).** Seven `VERDICT` lines in WITNESS-LOG, trimmed redacted evidence under `evidence/stop-b2/`, and the credential **value-shape** scan (`auth-creds|[Pp]assword=`) clean. The only word-level `password` matches are status prose (`DEMO password redacted`, `password value`), not a credential value; the broad word scan is therefore not the gate. The boot-iso options are recorded as owner-pending.

## Task Commits

| Task | Commit | Message |
| ---- | ------ | ------- |
| 1 | `c8c47ed` | fix(01-33): register the gate fast-fail after the qemu spawn |
| 2 | `9af05f8` | fix(01-33): route q35churnshell through the chain and tag the unimplemented caller |
| continuation | `3cf8dc4` | fix(01-33): keep a passing gate leg green through the final teardown |
| 4 | `c119924` | docs(01-33): record the stop-B2 verdicts and evidence |

## Files Created/Modified

- **Modified:** `gates/x86_64/drawterm-gate.expect`, `witness/run-witness.sh`, `witness/WITNESS-LOG.md`, `xtask/src/gate.rs`, `xtask/src/main.rs`, `x86_64/src/syscall.rs`.
- **Created:** `witness/evidence/stop-b2/` (13 files: q35churnshell-tcg-mac.log, summary.tsv, env.txt, secret-scan.txt, survivors.txt, kernel.log, profiles.log, q35churnshell.log, gate.log, gate-neg.log, probe.log, flipback.log, cpuauth-ref.log).

## Decisions Made

- The syscall-10 caller is Harvey's unchanged boot path; FAUTH stays loud-None by design. The fix is profile registration, not a handler.
- The gate teardown root cause is the self-matching identity sweep (SIGTERM), not an unguarded variable read; both the requested guards and the ancestry exclusion landed.
- Probe/flipback failures are the historical clones' pre-01-33 driver; the fixed driver is overlaid, leaving the image/ps at the exact SHA.
- The boot-iso hash mismatch is the long-owed kernel-byte check; options (a)/(b)/(c) are owner-pending and none is chosen.

## Deviations from Plan

1. **[Rule 2 — Missing critical guard] Static guards live in `xtask/src/gate.rs`** (the gate's test module, run by `cargo xtask test`), not in the plan's file list.
2. **[Rule 1/3 — Harness routing] q35churnshell needed stage routing, not only the syscall exemption**; the stop-B "waiting for ticks" line is the routing defect. Fixed in `xtask/src/main.rs`.
3. **[Rule 3 — Blocking rerun defect] The probe/flipback witness steps now overlay the fixed driver into their historical clones**; without it the requested rerun would reproduce the line-336 abort.
4. **[Rule 1 — Actual teardown root cause] The owner's variable-read hypothesis was not the cause**; the identity sweep SIGTERMed its own expect process. The requested guards were implemented and the ancestry exclusion fixed the actual defect.
5. **[Environment] Mac TCG verification adaptations:** the darwin devshell has no grub2, so the staged ISO was wrapped via a linux/amd64 `grub-mkrescue` container; `--cpu qemu64` was added because the profile's TOML `cpu = "host"` is KVM-only.
6. **[Environment] `tclsh -c` does not execute inline on Darwin**; the equivalent stdin form prints `info complete` = 1. `bash -n` is clean.
7. **[Evidence hygiene] The Mac TCG evidence is `q35churnshell-tcg-mac.log`**, leaving `q35churnshell.log` for the owner's KVM profile log.

## Issues Encountered

- **Stop-B2 is not clean (3 PASS / 4 non-zero).** `gate`, `probe`, `flipback` and `kernel` failed. The gate teardown and the probe/flipback driver overlay are fixed; one targeted KVM rerun is required.
- **Boot-iso fixed-output hash mismatch.** `specified sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=` vs `got sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=`; the kernel ELF sha is `4228a1bf1408a7f06d942fa847b6ca25d7bfbd5a400ce9190b602d3b365f32e1`. Owner decision pending (options a/b/c). No later plan started.
- **Probe/flipback stop-B2 logs were misattributed by the owner's summary to the teardown**; the decisive lines show the line-336 pre-01-33 driver abort. The record here follows the logs.

## Authentication Gates

None.

## User Setup Required

None.

## Next Phase Readiness

**Blocked.** The stop-B requirement stays open and no later plan (01-26 or later) starts. The next action is one targeted KVM rerun of `gate`, `gate_neg`, `probe`, `flipback` after the teardown/overlay fixes, plus the owner's boot-iso decision (a: re-derive/commit the outputHash; b: investigate the cross-builder difference; c: re-scope the check). `profiles` and `cpuauth_ref` already pass and need not be rerun.

## Self-Check: PASSED

- All 13 evidence files and this SUMMARY exist on disk.
- All four plan commits (`c8c47ed`, `9af05f8`, `3cf8dc4`, `c119924`) are ancestors of HEAD.
- `WITNESS-LOG.md` carries the seven stop-B2 `VERDICT` lines; the credential scan is clean.
