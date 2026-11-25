---
phase: 01-stage-p0-cross-build-harness
plan: 22
subsystem: cross-build-harness
tags: [witness, kvm, x86_64, gate, probe, boot-iso, profiles, cpuauth, halted, harn-01, harn-04, harn-06, harn-07, d-13, d-28]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-18's cargo xtask gate and the moved drawterm driver; 01-20's PROBE-FLIP-SHA and PROBE-FLIPBACK-SHA; 01-29's nine x86_64 profiles on the assembled images; 01-16's Linux-only authdomain-gen and the native-compare conditional; 01-14/01-17's Harvey trees"
provides:
  - "witness/run-witness.sh: the stop-B steps trees, kernel, gate, gate_neg, probe, flipback, profiles and cpuauth_ref"
  - "witness/WITNESS-LOG.md: the pass-2 ledger with the stop-B section (16 per-check verdicts and the selected-step roll-up)"
  - "witness/evidence/stop-b/: trimmed, redacted evidence for every selected step; trees PASS, every other step FAIL"
affects: [01-23, 01-24, 01-25, 01-26, 01-27, 01-28, 01-31, HARN-01, HARN-04, HARN-06, HARN-07]

actuals:
  tokens: 9395
  tasks: 3
  commits: 2

plan_head_before: 8d5eb8f0432c07e3942b1556d1817a0d32288e9f
plan_head_after: 5d0f5273548c8d5149c9d6a0eca1d152cc12aa04

tech-stack:
  added: []
  patterns:
    - "A stop-B witness step is self-contained: build/verify, log under out/<step>/, and record summary.tsv; the driver's exit is the step's"
    - "The probe legs clone the witness checkout at the recorded sha under the work root, so the committed tree at that revision is what runs"
    - "A KVM verdict is recorded only from a returned summary.tsv row and its log; a driver defect is not a marker failure"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/trees.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/kernel.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/gate.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/gate-neg.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/probe.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/flipback.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/q35churnshell.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/cpuauth-ref.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/env.txt
  modified:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh

key-decisions:
  - "Recorded the plan's 16 per-check stopb-* verdicts and the eight selected-step roll-ups; no KVM verdict is claimed for HARN-01 or HARN-04."
  - "cpuauth_ref was included because 01-16 recorded authdomain-gen as Linux-only. Both compares failed because the lazy R9_HARVEY_REFERENCE_AMD64 path has neither cpu.members nor auth.members; the log shows the cpu compare was reached first and also failed."
  - "The kernel verdict is FAIL on the boot-iso --rebuild precondition. The recorded package ELF sha256 differs from the local-build 01-08 baseline; 01-21 recorded the same package-vs-local difference for aarch64, so no kernel-byte equality is claimed either way."
  - "The q35churnshell syscall 10 is a separate kernel/churn-gate defect; the other eight profiles passed under KVM."
  - "The plan is halted; no later plan was started, and STATE.md/ROADMAP.md were not modified (worktree mode)."

requirements-completed: []

coverage:
  - id: D1
    description: "The witness driver carries the stop-B steps (trees, kernel, gate, gate_neg, probe, flipback, profiles, cpuauth_ref) as self-contained functions"
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh (exit 0, no output)"
        status: pass
      - kind: other
        ref: "grep -cE '^step_(trees|kernel|gate|gate_neg|probe|flipback|profiles)\\(\\)' .../run-witness.sh -> 7"
        status: pass
    human_judgment: false
  - id: D2
    description: "The owner's stop-B KVM run records a verdict for every selected step"
    verification:
      - kind: manual_procedural
        ref: "witness-returns/01-22 9100fce summary.tsv + out/<step> logs; trees PASS, all other selected steps FAIL"
        status: fail
    human_judgment: true
    rationale: "The x86_64 KVM run is Linux-host-only and can only be produced by the owner; the return is judged from its rows and logs."
  - id: D3
    description: "The pass-2 ledger and the stop-B evidence are committed, trimmed, and free of credentials"
    verification:
      - kind: other
        ref: "grep -cE '^VERDICT stopb-[a-z0-9-]+: (PASS|FAIL)$' witness/WITNESS-LOG.md -> 16"
        status: pass
      - kind: other
        ref: "grep -rniE 'auth-creds|passw' witness/evidence/stop-b | grep -vc '<redacted>' -> 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "The phase is halted at the owner checkpoint with HARN-01/HARN-04 unwitnessed and no later plan started"
    verification: []
    human_judgment: true
    rationale: "The decision between a fix iteration, a separate q35churnshell investigation, and a re-scope belongs to the owner."

duration: 3h 21m
completed: 2026-11-23
status: complete
phase_status: halted-stop-b-failures
---

# Phase 01 Plan 22: Owner stop B — the x86_64 KVM witness batch Summary

**The stop-B driver is in place and its first KVM run passed the native tree builds, but the moved drawterm driver aborts at `expect_before -i $spawn_id` before every gate leg, the boot-iso `--rebuild` had no prior build to check, the lazy reference path has no cpu/auth member lists, and q35churnshell hits `syscall 10 unimplemented`; the plan is halted without HARN-01 or HARN-04.**

## Performance

- **Duration:** 3h 21m wall (2026-11-23T12:13:51Z → 2026-11-23T15:34:58Z), including the owner's 11-minute stop-B run (15:12:45Z → 15:24:06Z) and the return review
- **Tasks:** 3 of 3 processed (Task 1 executor, Task 2 owner, Task 3 executor); the plan is **halted**
- **Files modified:** 13 (1 modified, 12 created)
- **Plan estimate:** 45,000 tokens (low confidence); **actual:** 9,395 tokens (chars/4 over the realized diff of the two executor commits)

## Accomplishments

- **The driver can run the whole stop-B batch in one invocation.** `step_trees` builds and store-verifies both Harvey trees; `step_kernel` records `kernel-x64.elf` and runs the boot-iso fixed-output check; `step_gate` runs the quick gate twice; `step_gate_neg` requires a non-zero gate carrying `GATE-FAIL rio-not-serving`; `step_probe`/`step_flipback` clone at 01-20's recorded shas; `step_profiles` runs the nine x86_64 profiles; `step_cpuauth_ref` runs the native cpu/auth compares. Shellcheck is clean and the seven required `step_*()` definitions are present.
- **The native x86_64 tree supply is witnessed.** Both trees built on `teeniebox` and `nix store verify --no-trust` printed `TREES-VERIFY-OK`; both `HARVEY-TREE` stamps carry `rev=bd7a698…` and the expected `narHash`.
- **The return is recorded honestly.** `WITNESS-LOG.md` has one verdict and one evidence path per check (16 `stopb-*` checks) plus the eight selected-step roll-ups. The evidence is trimmed to the decisive lines and the credential scan prints 0.
- **The halt is explicit.** Gate, gate_neg and both probe legs stopped in the driver before their assertions; q35churnshell failed on a kernel syscall; the boot-iso and reference checks did not run. No KVM PASS is claimed for HARN-01 or HARN-04, and no later plan was started.

## Task Commits

1. **Task 1: Add the stop-B steps to the witness driver** — `8417a84` (feat)
2. **Task 2: Owner stop B — run the selected steps on the KVM host** — owner return `9100fce` on `witness-src/01-22` (not merged; not a commit on this branch)
3. **Task 3: Record the stop-B FAIL verdicts and evidence** — `5d0f527` (docs)

**Plan metadata:** the `docs(01-22): complete … (halted)` commit that carries this SUMMARY.

## Files Created/Modified

- `witness/run-witness.sh` — the eight stop-B steps, the two 01-20 sha constants, the nine-profile list, the extended step whitelist and the new output dirs
- `witness/WITNESS-LOG.md` — the pass-2 ledger; the stop-B section has 16 per-check verdicts and the selected-step roll-up
- `witness/evidence/stop-b/trees.log` — both store paths, stamps and `TREES-VERIFY-OK`
- `witness/evidence/stop-b/kernel.log` — `KERNEL-STORE`, the ELF sha256 and the `--rebuild` refusal
- `witness/evidence/stop-b/gate.log` — both gate runs' line-336 `spawn_id` failure and `GATE-RUN` lines
- `witness/evidence/stop-b/gate-neg.log` — the force-no-rio leg's wrong failure
- `witness/evidence/stop-b/probe.log` — the successful probe build and the same driver failure
- `witness/evidence/stop-b/flipback.log` — the 4442-member flip-back assembly and the same driver failure
- `witness/evidence/stop-b/profiles.log` — nine `PROFILE <name> exit=…` lines
- `witness/evidence/stop-b/q35churnshell.log` — the `syscall 10 unimplemented` serial tail
- `witness/evidence/stop-b/cpuauth-ref.log` — both compares' missing `cpu.members`/`auth.members`
- `witness/evidence/stop-b/summary.tsv` — the returned `step/exit/seconds` rows
- `witness/evidence/stop-b/env.txt` — `DEV_KVM=writable`, nix and host facts

## Decisions Made

- **Two verdict layers.** The plan's 16 per-check `stopb-*` lines are the ledger's canonical rows; the eight `*-kvm` lines are the selected-step roll-up the owner asked for. Every `VERDICT` has an `EVIDENCE` path that exists and is non-empty.
- **No marker verdict from a driver crash.** `GATE-FAIL rio-leg-failed` is a Tcl driver failure, not `rio-not-serving`; HARN-01's positive and failing directions and HARN-04's two legs remain unwitnessed.
- **The package ELF is not the local-build baseline.** The recorded `kernel-x64.elf` sha256 is `3535848b…`, while the 01-08 baseline is `50b01c9a…`. 01-21 documented the same package-vs-local difference for the aarch64 package (the sandbox's `--remap-path-prefix`), so the comparison is not used as a kernel-byte claim; the boot-iso check is the authority and it did not run.
- **cpuauth_ref's ordering is taken from the log, not the return note.** The log shows `CPUAUTH-REF cpu exit=nonzero` followed by `CPUAUTH-REF auth exit=nonzero`: both compares were reached and both lack their member list.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Evidence beyond the plan's Task 3 file list**
- **Found during:** Task 3 (the stop-B return includes `cpuauth_ref`, which the plan's `<files>` list omitted)
- **Issue:** the selected run had eight steps, but the plan's evidence list named only seven step logs. `cpuauth_ref`'s failure (the missing reference member lists) and q35churnshell's serial tail would otherwise have had no phase-local evidence.
- **Fix:** added `cpuauth-ref.log`, `q35churnshell.log` and `env.txt` (the `DEV_KVM=writable` proof) under `witness/evidence/stop-b/`. No source or driver behavior changed.
- **Files modified:** `witness/evidence/stop-b/{cpuauth-ref.log,q35churnshell.log,env.txt}`, `witness/WITNESS-LOG.md`
- **Verification:** every evidence path exists and is non-empty; the secret scan prints 0.
- **Committed in:** `5d0f527`

### Verification-scope notes

1. **The plan's Task 3 verdict names assumed a clean run.** The plan lists `stopb-gate-1`, `stopb-gate-2`, `stopb-probe-flip` and `stopb-probe-flipback`; the owner's return additionally asks for the selected-step roll-up (`gate-kvm`, `probe-kvm`, …). Both layers are recorded, so the plan's 16-count gate and the owner's eight step verdicts both hold.
2. **`cpuauth_ref` was conditional in the plan.** 01-16-SUMMARY.md records authdomain-gen as Linux-only and explicitly says plan 01-22's stop-B step "still runs them natively on the KVM host (its conditional is satisfied)", so the step was selected.
3. **The kernel sha comparison is not a verdict input.** The plan only asked to record the sha256; the FAIL comes from the `--rebuild` precondition, not from comparing the package artifact to the local-build baseline.

---

**Total deviations:** 1 auto-fixed (missing critical evidence). **Impact:** the added evidence makes the two unlisted failures auditable; no implementation behavior changed.

## Stop-and-return record

The plan's stop rule applies. The owner checkpoint was returned with these decisive lines:

- **gate (both runs):** `can't read "spawn_id": no such variable` at
  `gates/x86_64/drawterm-gate.expect:336` (`expect_before -i $spawn_id` runs before the
  qemu spawn), then `GATE-FAIL rio-leg-failed`; `GATE-RUN 1/2 exit=nonzero`.
- **gate_neg:** the same line-336 failure, `GATE-FAIL force-no-rio-leg-failed`, and
  `GATE-NEG-WRONG-FAILURE (GATE-FAIL rio-not-serving absent)`.
- **probe:** the probe built (`96 flat bytes -> 136 a.out bytes`) and the cpu image
  assembled (4442 members), then the same driver failure; the serial `ps` check never
  ran.
- **flipback:** the flip-back cpu image assembled (4442 members), then the same driver
  failure.
- **profiles:** 8/9 exit 0; q35churnshell ends `churngate: heap exit 96512 33554432`,
  `syscall 10 unimplemented`, and `boot-test: >= 3 lines beginning "tick" not observed`.
- **kernel:** `KERNEL-STORE /nix/store/2279…`, ELF sha256 `3535848b…`, then
  `some outputs … are not valid, so checking is not possible` and
  `KERNEL-BOOTISO-REBUILD-FAIL`.
- **cpuauth_ref:** both compares fail with
  `reference member list /nix/store/mxx5…-harvey-reference-amd64-0/{cpu,auth}.members:
  No such file or directory`.
- **trees:** PASS, both `TREES-VERIFY-OK`.

No later plan (01-23 to 01-28, 01-31) was started. `STATE.md` and `ROADMAP.md` were not
modified (worktree mode; the orchestrator owns them).

## Issues Encountered

1. **Moved-driver `spawn_id` defect (01-18 regression).** `gates/x86_64/drawterm-gate.expect:336` registers `expect_before -i $spawn_id` at top level before any `spawn`; every gate leg aborts. This blocks gate, gate_neg, probe and flipback.
2. **boot-iso `--rebuild` precondition.** `nix build .#boot-iso --rebuild` refuses because the derivation was never previously built or substituted; the kernel-byte check needs a prior plain build (or a realized path) first.
3. **Lazy reference path.** `R9_HARVEY_REFERENCE_AMD64` is a context-discarded devshell string; the stop-B step never realized `harvey-reference-amd64`, so `cpu.members`/`auth.members` are absent.
4. **q35churnshell `syscall 10 unimplemented`.** A separate kernel/churn-gate defect; the other eight profiles passed.

## Known Stubs

None. The driver steps, the ledger and every evidence file are real; the FAIL verdicts are recorded rather than papered over.

## Threat Flags

None beyond the plan's threat model. T-01-70 is mitigated by trimming each evidence file to decisive lines and by the zero-hit credential scan; T-01-71 by recording verdicts only from the returned rows and logs; T-01-72 by recording the boot-iso check as not run rather than re-deriving the hash.

## Next Phase Readiness

- **The phase is halted.** HARN-01 and HARN-04 have no KVM witness; HARN-06's x86_64 half is 8/9; HARN-07's tree supply is proven but the boot-iso fixed-output check did not run.
- **Owner options (not chosen here):**
  - (a) a new fix iteration for the moved driver's `spawn_id` bug plus the stop-B step bugs (boot-iso precondition, reference member list), followed by a rerun of the failed steps;
  - (b) investigate q35churnshell's `syscall 10 unimplemented` as a separate kernel/churn-gate defect before any rerun;
  - (c) stop and re-scope.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23 (halted)*

## Self-Check: PASSED

- Created files exist and are non-empty: `witness/WITNESS-LOG.md`, `witness/evidence/stop-b/{trees,kernel,gate,gate-neg,probe,flipback,profiles,q35churnshell,cpuauth-ref}.log`, `summary.tsv`, `env.txt`, `01-22-SUMMARY.md` — all FOUND.
- Task commits `8417a84` (Task 1) and `5d0f527` (Task 3) are ancestors of HEAD; `git rev-list --count 8d5eb8f..HEAD` = 2 (measured against the plan ledger).
- Plan verify: `grep -cE '^VERDICT stopb-[a-z0-9-]+: (PASS|FAIL)$' witness/WITNESS-LOG.md` = 16; `grep -rniE 'auth-creds|passw' witness/evidence/stop-b | grep -vc '<redacted>'` = 0.
- Acceptance: every `VERDICT` has an `EVIDENCE` line whose path exists and is non-empty.
- Plan verification: nothing was pushed from this worktree; Harvey was not modified.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
