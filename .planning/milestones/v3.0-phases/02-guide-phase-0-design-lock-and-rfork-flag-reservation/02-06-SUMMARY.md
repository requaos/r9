---
phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
plan: 06
subsystem: evidence
tags: [owner-stop, x86_64-linux-kvm, boot-iso, outputhash, host-tests, rfork-mask-test, gate-kvm, gate-tcg, gate-full, transport-branch, main-push]

# Dependency graph
requires:
  - phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
    plan: 05
    provides: "The ordered owner checklist (02-OWNER-STOP.md), the six IDENTICAL .text/.text.boot verdicts and the five-leg aarch64 quick pass that gate this stop"
provides:
  - "The branch `phase-02/design-lock`, pushed with the owner's first approval, holding the Mac commits, the owner's `boot-iso` hash commit and two return commits"
  - "`evidence/owner-stop/RESULTS.md`: nine `stop-` verdicts, all PASS and all measured at the gated commit, plus `VERDICT stop-main-push: PASS 3d01f07…`"
  - "The owner's `outputHash` commit `4624ea2` on `main`, paired with the Linux `kernel-x64.elf` sha256 `6a5f5a66…`"
  - "`main` pushed to `origin` as `3d01f07` after the second owner approval; the two evidence commits after it stay local pending a later approved push"
affects: [phase-03, phase-06, phase-07]

# Actuals (#2632) — chars/4 over the executor-authored realized diff (RESULTS.md + this
# summary + the DSGN-04 checkbox); the owner's ~12k-line return logs are excluded.
actuals:
  tokens: 5676
  tasks: 3
  commits: 4
  plan_head_before: a0454ffed4f80473866239c4164a1918bf0d216c
  plan_head_after: fdfd3de95e31a25d9016203833bbaaaeecd48187

tech-stack:
  added: []
  patterns:
    - "Every verdict-producing command runs through one `run_logged` wrapper so its log opens `HEAD=<sha>` and closes `EXIT=<rc>`; a verdict is read only from such a log"
    - "The gated commit is the branch tip with every return commit left out, and the hash build, rebuild and all three gates must carry that same `HEAD=` value"
    - "The transport branch may carry only the owner's hash commit (`flake.nix` only) and return commits (`evidence/owner-stop/return/` only); anything else stops the round"
    - "The executor pushes only what the owner approved, at the moment of approval, and never edits `flake.nix`"

key-files:
  created:
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/RESULTS.md
  modified:
    - flake.nix
    - .planning/REQUIREMENTS.md

key-decisions:
  - "Round 1 was voided by the plan's own rule: no returned file carried a `HEAD=` line, so the build, rebuild and three gate verdicts were FAIL 'not measured at the gated commit' and the host-test steps were not returned at all"
  - "Round 2 ran at the gated commit 4624ea2 under the `run_logged` wrapper; all six verdict logs carry `HEAD=4624ea2…` and `EXIT=0`, so all nine verdicts are PASS"
  - "The hash commit is proven by measurement, not by a `got:` line: the committed value built at the gated commit (`boot-iso.log` EXIT=0) and the witness kernel step's plain build and `--rebuild` both passed under the same `HEAD=`"
  - "The late full-set reruns in round 1 (one tier1 FAIL, then three green) were not carried into round 2; round 2's full set was green on its first run, eight of eight apps PASS"
  - "`main` was pushed as 3d01f07 only after the second approval (D-26); the Task-3 evidence commit and this summary stay local until a later approved push"

patterns-established:
  - "Owner return: one wrapper log per step, `HEAD=` first and `EXIT=` last, committed under `evidence/owner-stop/return/`"
  - "Verdict recording: one `VERDICT id: PASS|FAIL` line per step with an `EVIDENCE id:` line quoting the decisive returned line and its `HEAD=` value"

requirements-completed: [DSGN-04]

coverage:
  - id: D1
    description: "The phase's commits were published to the branch `phase-02/design-lock` only after the owner's approval, and `main` was not moved at that stop"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "`git push origin HEAD:refs/heads/phase-02/design-lock` exit 0 at `2b41588`; before/after/remote shas equal; `git rev-parse origin/main` unchanged at `af46615`; `ls-remote` printed nothing before the approval"
        status: pass
    human_judgment: false
  - id: D2
    description: "The owner made the `boot-iso` `outputHash` commit on the x86_64 Linux host as its own one-line `flake.nix` commit, and the plain build and `--rebuild` passed at that gated commit"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "commit `4624ea2` shows `flake.nix | 2 +-` with `outputHash = \"sha256-x07uJKM8…\"`; `boot-iso.log` `HEAD=4624ea2…`/`EXIT=0`; `kernel-step.log` `KERNEL-BOOTISO-BUILD-OK` and `KERNEL-BOOTISO-REBUILD-OK` at `HEAD=4624ea2…`; Linux kernel sha256 `6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5`"
        status: pass
    human_judgment: false
  - id: D3
    description: "The x86_64 host tests ran on the owner's x86_64 Linux host and the rfork reserved-bits mask test printed ok"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "`host-tests.log` `HEAD=4624ea2…`/`EXIT=0`; `cargo test --package x86_64 --bins --target x86_64-unknown-linux-gnu`; `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok`; `test result: ok. 140 passed; 0 failed`"
        status: pass
    human_judgment: false
  - id: D4
    description: "The boot-to-rio quick gate passed once under KVM, the TCG gate passed, and the full gate set passed, all at the gated commit"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "`gate-kvm.log` `GATE PASS quick x86_64 run=765355-1795495125`; `gate-tcg.log` `GATE PASS quick x86_64 run=775293-1795495319`; `gate-full.log` `GATE PASS full x86_64-linux run=785121-1795495519` + `R9-WITNESS-RUN-OK: every selected step passed`; all `HEAD=4624ea2…`/`EXIT=0`"
        status: pass
    human_judgment: false
  - id: D5
    description: "Nine stop verdicts were recorded from the returned lines and `main` was pushed only after the second owner approval"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "`RESULTS.md`: 9/9 `stop-` verdicts PASS, FAIL count 0; `git push origin main` published `af46615..3d01f07`; `git rev-parse origin/main` equals the sha shown; `VERDICT stop-main-push: PASS 3d01f071efda9ed0720f1a07c4a6db0c7af8f4c5`"
        status: pass
    human_judgment: false

duration: 2h17m
completed: 2026-11-24
status: complete
---

# Phase 2 Plan 06: The single end-of-phase owner stop Summary

**The phase's work reached the owner's x86_64 Linux KVM host over a pushed branch, the owner re-derived and committed the `boot-iso` `outputHash` (`4624ea2`) and returned `HEAD=`-stamped logs, all nine stop verdicts are PASS at that gated commit, and `main` was pushed as `3d01f07` after the second approval — with round 1 voided and re-run because its return carried no `HEAD=` stamps.**

## Performance

- **Duration:** ~2h17m (mostly owner-host run time and one voided round)
- **Started:** 2026-11-24T02:41Z
- **Completed:** 2026-11-24T04:58Z (last task commit)
- **Tasks:** 3 (Task 1 and Task 3 are owner stops; Task 2 ran twice)
- **Files:** `RESULTS.md` created then rewritten; the owner changed one line of `flake.nix` and added 21 + 24 return files

## Accomplishments

- Verified every precondition for the stop before presenting it: main-tree check, clean tracked tree, `docs/threat-model.md` committed, no `PENDING` in `02-DECISION-TABLE.md`, six `IDENTICAL` text verdicts in `KERNEL-BYTES.md`, five aarch64 PASS verdicts plus `mac-full-suite: PASS`.
- Pushed the branch only on the owner's `push`: `git push origin HEAD:refs/heads/phase-02/design-lock` created the branch at `2b41588`, and the before/after/remote shas were all equal; `origin/main` stayed `af46615`.
- Took the owner's `outputHash` commit (`4624ea2`, `flake.nix` only, one line, no attribution) and two return commits through the allowed-shape check, fast-forwarded the branch into `main`, and recorded the gated commit as the tip with the return commits left out.
- Recorded all nine stop verdicts from the round-2 return: branch push, the hash commit (proven by the successful build at the committed value), the boot-iso plain build and `--rebuild`, the x86_64 host tests (140 passed), the rfork mask test, the KVM quick gate (once), the TCG gate and the full set (eight of eight apps PASS).
- Recorded the Linux `kernel-x64.elf` sha256 `6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5`, which pairs with the committed `outputHash`.
- Pushed `main` as `3d01f07` only after the owner's second approval and appended `VERDICT stop-main-push: PASS 3d01f07…` in a one-line evidence commit.

## Task Commits

Each task was committed atomically:

1. **Task 1: OWNER STOP — approve the branch push and run the checklist** — no commit; the branch `phase-02/design-lock` moved from `2b41588` (Mac) to `f8d077d` (owner's hash and return commits)
2. **Task 2, round 1: Take the owner's return (voided)** — `40aef09` (evidence; `58037d3` before the rebase onto the branch, superseded by round 2)
3. **Task 2, round 2: Take the round-2 return and record nine PASS verdicts** — `3d01f07` (evidence)
4. **Task 3: OWNER STOP — approve pushing `main`** — `fdfd3de` (evidence, one line)

**Plan metadata:** this summary (docs).

Measured `commits`: `git rev-list --count a0454ff..fdfd3de` = 4. The range includes the owner's round-2 return commit `f8d077d`; the executor's own commits are `40aef09`, `3d01f07` and `fdfd3de`.

## Files Created/Modified

- `evidence/owner-stop/RESULTS.md` — the nine stop verdicts with their `EVIDENCE` lines, the gated commit, the hash commit, the kernel sha256 and the `stop-main-push` PASS line
- `flake.nix` — one line, by the owner on the Linux host: `outputHash` `b6OcAb…` → `x07uJKM8…`
- `evidence/owner-stop/return/` — the owner's round-1 files (`a0454ff`) and round-2 logs (`f8d077d`, under `round2/`)
- `.planning/REQUIREMENTS.md` — DSGN-04 checked complete (checkbox and traceability row)

## Decisions Made

- Round 1 was stopped, not bent: the plan's `HEAD=` rule made the build, rebuild and three gate verdicts FAIL and the host-test steps "not returned", so Task 3 was not started and `main` was not pushed.
- Round 2 was asked for at the gated commit (`4624ea2`), not at the branch tip (`a0454ff`), because the gated commit is defined as the tip with the return commits left out and every `HEAD=` stamp must equal it.
- The hash verdict rests on measurement: the committed value built at the gated commit and the `--rebuild` re-checked the `r9-boot-iso-4624ea2` outputs; no `got:` line exists because there was no mismatch to report.
- The full set was re-run in round 1 after its first run failed `boot-r9-tier1-x64`; only round 2's first-run green set was recorded.

## Deviations from Plan

None — the plan ran as written. No source file was changed by the executor; the only `flake.nix` change is the owner's commit. The round-1 stop and round-2 re-run are the plan's own stop rule firing, not a deviation.

## Round 1 → round 2

Round 1's return (commit `a0454ff`) contained only the witness driver's `full/` work directory and a pass-2 bundle whose kernel and gate excerpts named `r9-kernel-x64-af46615` — pre-Phase-2 code. `grep -c '^HEAD='` over all 21 files was 0; the host tests, KVM gate and TCG gate were absent; and no `got:` value was returned. Eight of nine verdicts were FAIL, `main` was not pushed, and the owner was asked to run sections 3–7 again at `4624ea2` with the `run_logged` wrapper. Round 2 (commit `f8d077d`) returned six wrapper logs, each `HEAD=4624ea2…` → `EXIT=0`, and all nine verdicts are PASS.

## Issues Encountered

- Round 1's missing `HEAD=` stamps: resolved by the round-2 re-run at the gated commit; no code fix was needed.
- Observational: the owner's hash commit message is `boot-iso hash bump` rather than the checklist's long suggested message, and the recommended dated comment above line 983 is absent. The plan's Task 2 check is the allowed shape plus no attribution, both satisfied; neither affects the measurement.
- Two commits share the subject `evidence(02-06): end-of-phase owner stop results` (`40aef09` is the rebased round-1 record, `3d01f07` the round-2 record); the file at HEAD is round 2 only.

## Known Stubs

None.

## Threat Flags

None beyond the planned surface. T-02-20 (a push without approval) held: each push ran only after its approval and published exactly the reviewed list. T-02-21 (an unexpected branch commit) held: only the hash commit and the return commits arrived. T-02-22 (a verdict without a run) held: round 1's unstamped lines were rejected. T-02-23 (a silent kernel byte change) held: the fixed-output build and `--rebuild` ran at the gated commit and the Linux kernel sha256 is recorded. T-02-24 (credentials in returned logs): `secret-scan.txt` clean and no credential string appears; nothing needed redaction.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `origin/main` is `3d01f07`; the gated commit `4624ea2` is an ancestor, so the pushed main carries exactly the code the gates measured.
- Two local commits remain unpushed by design: `fdfd3de` (the `stop-main-push` evidence line) and this summary. They publish only on a later approved push.
- `phase-02/design-lock` may be deleted by the owner once the branch is no longer needed.
- DSGN-04 is witnessed and marked complete; Phase 2's design lock and the four reserved `rfork` bits are ready for Phase 3.

---

*Phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation*
*Completed: 2026-11-24*

## Self-Check: PASSED

`evidence/owner-stop/RESULTS.md` and `02-06-SUMMARY.md` exist; the plan's commits `4624ea2`
(the owner's hash commit), `f8d077d` (the round-2 return), `40aef09` (round-1 record),
`3d01f07` (round-2 verdicts) and `fdfd3de` (the `stop-main-push` line) are ancestors of HEAD;
`RESULTS.md` holds nine `stop-` verdicts with zero FAIL and exactly one `stop-main-push` line;
`origin/main` is `3d01f071efda9ed0720f1a07c4a6db0c7af8f4c5`.
