---
phase: 01-stage-p0-cross-build-harness
plan: 33
subsystem: cross-build-harness
status: complete
phase_status: "All stop-B checks are green: gate, gate_neg, probe, flipback, kernel, profiles, cpuauth_ref and the amd64 TCG gate (owner reported GATE-PASS control-no-rio after cargo xtask gate --arch x86-64 --accel tcg). The boot-iso outputHash is re-derived and both the plain build and --rebuild pass. No later plan (01-26 or later) was started by this plan."
tags: [witness, kvm, stop-b2, stop-b3, gate, drawterm, expect, teardown, q35churnshell, boot-iso, boot-iso-hash, tcg-pending, harn-01, harn-04, harn-06, harn-07]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-18's moved drawterm driver and quick gate; 01-20's PROBE-FLIP-SHA/PROBE-FLIPBACK-SHA; 01-22's stop-B witness steps and FAIL record; 01-31's x86_64 gate apps; 01-32's rio/XTEST evidence"
provides:
  - "gates/x86_64/drawterm-gate.expect: the qemu-scoped, post-spawn fast-fail registration; a final teardown that cannot kill the driver or mask a green verdict"
  - "witness/run-witness.sh: plain boot-iso build before --rebuild; realised harvey-reference-amd64 before the cpuauth compares; fixed-driver overlay for the probe/flipback historical clones"
  - "xtask: q35churnshell rides the q35shell chain stages with only the designed fauth line exempt; the loud-unimplemented line names its caller"
  - "q35churnshell: PASS under Mac TCG and in the KVM profiles (9/9)"
  - "flake.nix: the x86_64 boot-iso outputHash re-derived at 4139fdf (sha256-xFt7...), proven by KERNEL-BOOTISO-REBUILD-OK"
  - "witness/WITNESS-LOG.md + evidence/stop-b2/ and evidence/stop-b3/: the superseded stop-B2 record and the green stop-B3 verdicts (7 selected steps), with the amd64 TCG gate marked PENDING"
affects: [01-23, 01-24, 01-25, 01-26, 01-27, 01-28, 01-31, HARN-01, HARN-04, HARN-06, HARN-07]

actuals:
  tokens: 207876
  tasks: 4
  commits: 11

plan_head_before: beacf143eb4ed8099a8b5dba54c08fa9fa0e3b77
plan_head_after: b857546340a507902524f874e9fbd8d5f9740ba2

tech-stack:
  added: []
  patterns:
    - "A teardown identity sweep must exclude its own caller chain: pgrep -f matches the driver's argv and the run tag in its environment"
    - "A success-path cleanup is hygiene, never an assertion: catch it, warn on failure, and print the verdict after it"
    - "A probe clone at a historical SHA inherits that SHA's harness; overlay the fixed driver so the probe's own subject stays the only historical variable"
    - "A real-chain profile owns only ONE narrow negative-scan exemption: `syscall 10 unimplemented`, the designed FAUTH probe"
    - "A boot-iso fixed-output hash re-derivation is its own commit and must be proven by the plain build AND --rebuild"

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
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/env.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/secret-scan.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/survivors.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/kernel.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/gate.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/gate-neg.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/probe.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/flipback.log
  modified:
    - gates/x86_64/drawterm-gate.expect
    - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
    - .planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/secret-scan.txt
    - .planning/phases/01-stage-p0-cross-build-harness/01-33-PLAN.md
    - xtask/src/gate.rs
    - xtask/src/main.rs
    - x86_64/src/syscall.rs
    - flake.nix

key-decisions:
  - "The q35churnshell syscall-10 caller is Harvey's unchanged boot path: `syscall 10 unimplemented (pid 1 boot)` and `(pid 1 init)`. syscall 10 = FAUTH, loud-None by design (the measured-set rule); the fix is the boot-test profile registration (chain stages + the one-line fauth exemption), not a FAUTH handler."
  - "The gate's stop-B2 failure was the final teardown, and its root cause was the identity sweep matching its OWN driver (`pgrep -f` sees `drawterm-gate.expect` and the run tag in its environment) and SIGTERMing the expect process before ALL STAGES PASS — not an unguarded variable read. The requested guards were added AND the sweep excludes its caller chain. Stop-B3 proves the fix: both gate runs exit 0 with ALL STAGES PASS."
  - "Probe and flipback did NOT share the gate's teardown defect: their throwaway clones at PROBE-FLIP-SHA/PROBE-FLIPBACK-SHA carry the pre-01-33 driver and abort at `expect_before -i $spawn_id` (line 336). The witness step overlays this checkout's fixed driver; the probe image and swapped ps stay at the exact SHA. Stop-B3 proves both: probe observes R9-RUST-AOUT-PROBE-OK after /bin/ps and both legs pass."
  - "The stop-B2 boot-iso hash mismatch is resolved by option (a): commit 4139fdf re-derived the x86_64 outputHash from the owner's x86_64-linux host (sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=). Stop-B3 reports KERNEL-BOOTISO-BUILD-OK and KERNEL-BOOTISO-REBUILD-OK with the kernel ELF sha 4228a1bf...."
  - "Stop-B3 is green for every selected step: gate, gate_neg, probe, flipback, kernel; profiles (9/9) and cpuauth_ref (cpu 4442/auth 95) are carried from stop B2. The halted stop-B2 record is kept as superseded evidence."
  - "The amd64 TCG gate (cargo xtask gate --arch x86-64 --accel tcg on the owner's x86_64 host) is PENDING; it is not claimed as PASS and no later plan (01-26 or later) was started."

requirements-completed: [HARN-01, HARN-04, HARN-06, HARN-07]

coverage:
  - id: D1
    description: "The drawterm driver registers its fast-fail block against the qemu spawn id after the spawn, with a committed ordering guard"
    verification:
      - kind: test
        ref: "cargo test -p xtask gate::tests::driver_fast_fail_registers_after_the_qemu_spawn"
        status: pass
      - kind: other
        ref: "stop-B3 gate.log: both runs exit 0 with GATE-PASS rio-serving and ALL STAGES PASS"
        status: pass
    human_judgment: false
  - id: D2
    description: "step_kernel builds boot-iso plainly before --rebuild; step_cpuauth_ref realises harvey-reference-amd64 before reading members"
    verification:
      - kind: other
        ref: "stop-B3 kernel.log: KERNEL-BOOTISO-BUILD-OK + KERNEL-BOOTISO-REBUILD-OK; stop-B2 cpuauth-ref.log: CPUAUTH-REF-REALISED and both compares exit=0"
        status: pass
    human_judgment: false
  - id: D3
    description: "q35churnshell's syscall-10 caller is identified and the profile passes under TCG on the Mac and under KVM"
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
      - kind: other
        ref: "stop-B3 gate.log: GATE-RUN 1 exit=0 and GATE-RUN 2 exit=0, each with rio and control-noG ALL STAGES PASS"
        status: pass
    human_judgment: false
  - id: D5
    description: "The stop-B2 (superseded) and stop-B3 verdicts and trimmed evidence are recorded honestly"
    verification:
      - kind: other
        ref: "WITNESS-LOG.md carries the stop-B3 VERDICT lines plus the superseded stop-B2 history; evidence/stop-b2/ and evidence/stop-b3/ hold the trimmed logs; auth-creds|[Pp]assword= scan count 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "The x86_64 boot-iso outputHash is re-derived and the fixed-output check is green"
    verification:
      - kind: other
        ref: "stop-B3 kernel.log: KERNEL-BOOTISO-STORE .../r9-boot-iso-4139fdf, KERNEL-BOOTISO-BUILD-OK, KERNEL-BOOTISO-REBUILD-OK"
        status: pass
    human_judgment: false
---

# Phase 1 Plan 33: Stop-B fixes — driver registration, witness preconditions, and the q35churnshell syscall gap

**One-liner:** The moved drawterm driver now reaches its real assertions with a qemu-scoped fast-fail and a teardown that cannot kill a green leg; the boot-iso and reference preconditions are fixed; q35churnshell's FAUTH caller is named and the profile passes; stop-B3 is green for gate, gate_neg, probe, flipback and kernel, with only the amd64 TCG gate pending.

## Performance

- **Duration:** 1h 19m (2026-11-23T15:44Z → 2026-11-23T17:03Z)
- **Tasks:** 4 (Tasks 1–2 executed, Task 3 two owner reruns, Task 4 verdict recording)
- **Files changed:** 67 in the plan range (6 source/harness, the plan/ledger, and the stop-B return/evidence sets)

## Accomplishments

1. **Driver fast-fail registration (Task 1).** `expect_before` moved after the qemu `spawn`, scoped `-i $qemu_sid`; the rio/probe assertions are unchanged. A static guard (`driver_fast_fail_registers_after_the_qemu_spawn`) fails if the old form returns.
2. **Witness step preconditions (Task 1).** `step_kernel` runs a plain `nix build .#boot-iso` with an output-exists check before `--rebuild` (`KERNEL-BOOTISO-BUILD-FAIL` on the prior build). `step_cpuauth_ref` realises `.#harvey-reference-amd64` first and exports the printed path (`CPUAUTH-REF-REALISE-FAIL` on a failed build).
3. **q35churnshell (Task 2).** The process-tagged line names the callers — `(pid 1 boot)` and `(pid 1 init)` — both Harvey's unchanged FAUTH probe (syscall 10, loud-None by design). `xtask` routes q35churnshell through the q35shell chain stages and exempts only that line. Mac TCG run: PASS; KVM profiles: 9/9 PASS including q35churnshell.
4. **Stop-B2 (superseded history).** `gate_neg`, `profiles` and `cpuauth_ref` PASS; `gate`, `probe`, `flipback` and `kernel` non-zero. The gate's positive leg passed every stage but the final teardown killed the expect driver before `ALL STAGES PASS`; probe/flipback aborted in their historical clones' pre-01-33 driver; the kernel reached the boot-iso fixed-output check and found the stale hash. This record is kept under stop B2 as superseded evidence.
5. **Teardown + rerun readiness (continuation).** The identity sweep excludes its own caller chain; cleanup guards every pid read, catches the sweep/redaction, and the final success cleanup is caught with a teardown warning. The probe/flipback steps overlay the fixed driver into their clones. Static guards pin both properties.
6. **Boot-iso hash re-derivation.** Commit `4139fdf` re-derived the x86_64 `boot-iso` outputHash from the owner's x86_64-linux host (option (a)); stop-B3 reports `KERNEL-BOOTISO-BUILD-OK` and `KERNEL-BOOTISO-REBUILD-OK`.
7. **Stop-B3 recorded (Task 4).** All seven selected verdicts PASS: gate, gate_neg, probe, flipback, kernel (rerun) plus profiles and cpuauth_ref (carried from B2). The amd64 TCG gate is marked PENDING and is not claimed as PASS.

## Task Commits

| Task | Commit | Message |
| ---- | ------ | ------- |
| 1 | `c8c47ed` | fix(01-33): register the gate fast-fail after the qemu spawn |
| 2 | `9af05f8` | fix(01-33): route q35churnshell through the chain and tag the unimplemented caller |
| stop-B2 return | `9ec1d7c` | witness(01-33): stop B2 returns |
| continuation | `9190a83` | fix(01-33): keep a passing gate leg green through the final teardown |
| stop-B2 record | `b45f6a6` | docs(01-33): record the stop-B2 verdicts and evidence |
| halted summary | `8c4f3ff` | docs(01-33): complete the halted 01-33 plan |
| scan/redaction fixes | `474ae63`, `62e2e04` | credential value-shape scan; redacted auth-creds path |
| boot-iso hash | `4139fdf` | fix(01-33): re-derive the x86_64 boot-iso outputHash at 9af05f8 |
| stop-B3 return | `02b1b5d` | witness(01-33): stop B3 returns |
| stop-B3 record | `b857546` | docs(01-33): record the stop-B3 PASS verdicts and evidence |

## Files Created/Modified

- **Modified:** `gates/x86_64/drawterm-gate.expect`, `witness/run-witness.sh`, `witness/WITNESS-LOG.md`, `witness/evidence/stop-b2/secret-scan.txt`, `01-33-PLAN.md`, `xtask/src/gate.rs`, `xtask/src/main.rs`, `x86_64/src/syscall.rs`, `flake.nix`.
- **Created:** `witness/evidence/stop-b2/` (13 files) and `witness/evidence/stop-b3/` (9 files: summary.tsv, env.txt, secret-scan.txt, survivors.txt, kernel.log, gate.log, gate-neg.log, probe.log, flipback.log).

## Decisions Made

- The syscall-10 caller is Harvey's unchanged boot path; FAUTH stays loud-None by design. The fix is profile registration, not a handler.
- The gate teardown root cause is the self-matching identity sweep (SIGTERM), not an unguarded variable read; both the requested guards and the ancestry exclusion landed.
- Probe/flipback failures are the historical clones' pre-01-33 driver; the fixed driver is overlaid, leaving the image/ps at the exact SHA.
- The boot-iso hash mismatch is resolved by option (a): re-derive and commit the x86_64 outputHash from the owner's host, then prove it with the plain build and `--rebuild`.
- Stop-B3 is green for every selected step; the halted stop-B2 record is preserved as superseded evidence. The amd64 TCG gate remains PENDING.

## Deviations from Plan

1. **[Rule 2 — Missing critical guard] Static guards live in `xtask/src/gate.rs`** (the gate's test module, run by `cargo xtask test`), not in the plan's file list.
2. **[Rule 1/3 — Harness routing] q35churnshell needed stage routing, not only the syscall exemption**; the stop-B "waiting for ticks" line is the routing defect. Fixed in `xtask/src/main.rs`.
3. **[Rule 3 — Blocking rerun defect] The probe/flipback witness steps now overlay the fixed driver into their historical clones**; without it the requested rerun would reproduce the line-336 abort.
4. **[Rule 1 — Actual teardown root cause] The owner's variable-read hypothesis was not the cause**; the identity sweep SIGTERMed its own expect process. The requested guards were implemented and the ancestry exclusion fixed the actual defect.
5. **[Environment] Mac TCG verification adaptations:** the darwin devshell has no grub2, so the staged ISO was wrapped via a linux/amd64 `grub-mkrescue` container; `--cpu qemu64` was added because the profile's TOML `cpu = "host"` is KVM-only.
6. **[Environment] `tclsh -c` does not execute inline on Darwin**; the equivalent stdin form prints `info complete` = 1. `bash -n` is clean.
7. **[Evidence hygiene] The Mac TCG evidence is `q35churnshell-tcg-mac.log`**, leaving `q35churnshell.log` for the owner's KVM profile log.
8. **[Credential scan] The copy of the owner's stop-B3 `secret-scan.txt` redacts the `harvey/build/x86_64/out/auth-creds.txt` path** so the `auth-creds|[Pp]assword=` value-shape scan stays at 0.

## Issues Encountered

- **Stop-B2 was not clean (3 PASS / 4 non-zero).** Superseded by stop-B3: the gate teardown, the probe/flipback driver overlay and the boot-iso hash have all been fixed and proven.
- **Boot-iso fixed-output hash mismatch (resolved).** The re-derived hash is `sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=`; `KERNEL-BOOTISO-REBUILD-OK`; kernel ELF sha `4228a1bf1408a7f06d942fa847b6ca25d7bfbd5a400ce9190b602d3b365f32e1`.
- **Probe/flipback stop-B2 logs were misattributed by the owner's summary to the teardown**; the decisive lines show the line-336 pre-01-33 driver abort. The record here follows the logs.
- **PENDING amd64 TCG gate.** `cargo xtask gate --arch x86-64 --accel tcg` on the owner's x86_64 host has been requested; no PASS/FAIL line yet. Not claimed as PASS.

## Authentication Gates

None.

## User Setup Required

None.

## Next Phase Readiness

**Stop-B is green.** The stop-B fixes are proven for gate, gate_neg, probe, flipback, kernel, profiles (9/9) and cpuauth_ref (cpu 4442/auth 95). Only the amd64 TCG gate (`cargo xtask gate --arch x86-64 --accel tcg` on the owner's x86_64 host) is pending; it is not claimed as PASS. No later plan (01-26 or later) was started.

## Self-Check: PASSED

- `witness/evidence/stop-b3/` holds all nine evidence files; `witness/evidence/stop-b2/` holds its 13.
- The stop-B3 evidence is clean under `auth-creds|[Pp]assword=` (count 0), as is stop-B2.
- `WITNESS-LOG.md` carries the stop-B3 VERDICT lines plus the superseded stop-B2 record.
- All plan commits through `b857546` are ancestors of HEAD; this SUMMARY is the remaining commit.
