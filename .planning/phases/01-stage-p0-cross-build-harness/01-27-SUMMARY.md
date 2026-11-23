---
phase: 01-stage-p0-cross-build-harness
plan: 27
subsystem: cross-build-harness
tags: [witness, stop-c, x86_64, kvm, harvey-flip, approval, halted, harn-02, harn-08, d-02, d-10, d-14, d-19, d-23]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-26's prepared Harvey flip and stop-C witness steps (full/soak/stress/equiv/store_verify); 01-31's last x86_64 apps; the green stop-B3 KVM leg; 01-30's aarch64 quick pass; r9 HEAD 3f717f7"
provides:
  - "witness/WITNESS-LOG.md: the Stop C section — 5 step verdicts, 8 app verdicts, decisive failure lines and a green-rerun prerequisite list"
  - "witness/evidence/stop-c/: trimmed, redacted evidence (full.log, full.tsv, soak.log, stress.log, equiv/report.tsv, equiv/rows.tsv, store_verify.log, summary.tsv, env.txt)"
  - "harvey-flip/HARVEY-FLIP.md: BLOCKED / NOT APPROVED state with HARVEY-COMMIT/APPROVED/PUSHED all none"
affects: [01-28, HARN-02, HARN-08, D-10, D-23]

actuals:
  tokens: 5930
  tasks: 2
  commits: 2

plan_head_before: 9f8a21315f3d9cd5c81f77724208a93a8b06318a
plan_head_after: 302613397144fa73a1a3fc79f9c2ca66cf582e3d

tech-stack:
  added: []
  patterns:
    - "A failed stop C is recorded as verdicts plus trimmed, redacted evidence; a non-green run or a missing APPROVE makes no Harvey change"
    - "A stop-C failure report separates harness defects (the tap preflight's HARVEY seam; the full runner's app-to-capture-dir map) from a gate failure (tier-1 fpprobe)"
    - "The full runner copies each app's actual R9_GATE_OUT directory; the six short-name dirs and the two matching names are recorded for the fix"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/full.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/full.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/soak.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/stress.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/equiv/report.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/equiv/rows.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/store_verify.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/env.txt
  modified:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md
    - .planning/phases/01-stage-p0-cross-build-harness/harvey-flip/HARVEY-FLIP.md

key-decisions:
  - "Stop C is FAILED: full 7/8 (boot-r9-tier1-x64 T1-FAIL fpprobe), soak and stress stop on the tap precondition, equiv fails on capture gaps plus boot-r9-x64 markers 0/1; store_verify is the only PASS."
  - "No Harvey commit was made: the gates were not green and the owner did not approve the diff. HARVEY-COMMIT, HARVEY-APPROVED and HARVEY-PUSHED are all none; HEAD remains 25d97dc4d and the status hash 041d2929…."
  - "The six FULL-NO-CAPTURE lines are a copy-map defect in step_full, not six gate failures: those apps write target/gates/<short-name> (smoke-r9-x64, tier1-r9-x64, smoke-r9-net-x64, services-r9-x64, churn-r9-x64, smoke-r9-auth-x64) while the runner copies target/gates/<app-name>."
  - "soak/stress never exercised the tap venue: venue_gate ran net-doctor without HARVEY, so the tap preflight read <r9>/build/... instead of the r9-built tree's seam; both flake apps already export HARVEY_NETADDR_SEAM correctly."
  - "The boot-r9-x64 0/1 equivalence markers do appear in full.log (lines 552, 565, 585); the terminal-mode boot app writes no target/gates/boot-r9-x64/*.log, so count_in_dir saw an empty directory. Recorded as a capture-path defect to confirm, not asserted as a boot regression."
  - "The tier-1 fpprobe failure is a genuine gate failure (note handlers ran 0 times, user trap vector 1, post-pass missing T1-OK fpprobe) and is the one item that needs kernel/note-delivery investigation before a green rerun."
  - "The plan is halted; no later plan (01-28) was started, and STATE.md/ROADMAP.md were not modified (worktree mode)."

requirements-completed: []

coverage:
  - id: D1
    description: "The stop-C result is recorded: five step verdicts, eight app verdicts, and trimmed evidence with zero credential matches"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "grep -cE '^VERDICT stopc-(full|soak|stress|equiv|store-verify): (PASS|FAIL)$' witness/WITNESS-LOG.md -> 5; stopc-app-* -> 8"
        status: pass
      - kind: other
        ref: "find witness/evidence/stop-c -type f: 9 files; grep -acE 'auth-creds|[Pp]assword=' each -> 0; line counts all <= 200"
        status: pass
    human_judgment: false
  - id: D2
    description: "No Harvey change was made without a green stop C and the owner's APPROVE"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "git -C /Users/neilskinner/code/harvey rev-parse HEAD -> 25d97dc4d…; status --porcelain sha256 -> 041d2929…; HARVEY-COMMIT/APPROVED/PUSHED all none"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every moved gate is proven from r9 on the KVM witness before Harvey changes (the stop-C gate set)"
    requirement: HARN-02
    verification:
      - kind: manual_procedural
        ref: "witness-src/01-27 6346c68 summary.tsv + full.tsv: full FAIL (7/8), soak FAIL, stress FAIL, equiv FAIL, store_verify PASS"
        status: fail
    human_judgment: true
    rationale: "The x86_64 KVM run is owner-host-only; this return shows the gate set is not yet green, so HARN-02/HARN-08 remain unwitnessed."
  - id: D4
    description: "The phase is halted at the owner checkpoint with no later plan started"
    verification: []
    human_judgment: true
    rationale: "The choice between fixing the harness defects and investigating fpprobe first, investigating fpprobe alone, or re-scoping HARN-08's x86_64 equivalence/tier1 gate belongs to the owner."

duration: 1h 32m
completed: 2026-11-23
status: halted
phase_status: halted-stop-c-failures
---

# Phase 01 Plan 27: Owner stop C — the full moved set, the Harvey approval, the commit and push Summary

**Stop C ran the whole moved x86_64 set from r9 on the owner's KVM host and is not green: 7/8 full apps pass but `boot-r9-tier1-x64` fails on `T1-FAIL fpprobe` (note handlers ran 0 times, `user trap: unhandled vector 1`), soak and stress stop on a tap-precondition `HARVEY`-seam defect before reaching the venue, equivalence fails on six capture-copy gaps plus `boot-r9-x64` 0/1 markers, and only `store_verify` passes — so the owner did not approve the Harvey diff, no Harvey change was made, and plan 01-28 was not started.**

## Performance

- **Duration:** ~1h 32m (prep 2026-11-23T17:29:58Z → evidence 2026-11-23T19:01:34Z), including the owner's 55-minute stop-C run (17:37:29Z → 18:32:12Z) and the return review.
- **Tasks:** 2 of 3 processed (Task 1 = owner return `6346c68`; Task 2 = this recording on its FAIL branch); Task 3 (owner push) was not reached.
- **Files modified:** 11 (9 evidence files created; WITNESS-LOG.md and HARVEY-FLIP.md modified).
- **Plan estimate:** 45,000 tokens (low confidence); **actual:** 5,930 tokens (chars/4 over the realized diff of the two executor commits, 23,720 chars).
- **Commits:** 2 executor commits in the plan range before this SUMMARY (prep `3f717f7`, record `3026133`).

## Accomplishments

- **The stop-C return is recorded with all five step verdicts.** `VERDICT stopc-full: FAIL`, `stopc-soak: FAIL`, `stopc-stress: FAIL`, `stopc-equiv: FAIL`, `stopc-store-verify: PASS`, plus one `stopc-app-*` verdict per row of `full.tsv` (7 PASS, `boot-r9-tier1-x64` FAIL). Every verdict has an `EVIDENCE` path that exists.
- **The evidence is trimmed and redacted.** Nine files under `witness/evidence/stop-c/` (`full.log` 168 lines, `full.tsv`, `soak.log`, `stress.log`, `equiv/report.tsv`, `equiv/rows.tsv`, `store_verify.log`, `summary.tsv`, `env.txt`); `grep -acE 'auth-creds|[Pp]assword='` is 0 on every file.
- **The failures are separated by cause.** The tier-1 `fpprobe` failure is a real gate failure; the six `FULL-NO-CAPTURE` lines are a `step_full` copy-map defect; soak/stress stopped at net-doctor's tap preflight because `HARVEY` was not set, so the venue was never exercised; `boot-r9-x64`'s 0/1 markers are present in `full.log` and absent only as a per-app `.log`.
- **The Harvey diff is explicitly blocked.** HARVEY-FLIP.md now records `BLOCKED — NOT APPROVED, not committed`, with `HARVEY-COMMIT: none`, `HARVEY-APPROVED: none`, `HARVEY-PUSHED: none`; the diff stays at `$HOME/r9-harvey-flip/harvey-flip.diff` (sha256 `f7b0272d…`).
- **Nothing moved in Harvey.** The owner's checkout is still at `25d97dc4d4976bfb7411f01970755acf6261c81b` with status-porcelain sha256 `041d2929…`; no staging, commit or push was attempted.

## Task Commits

| Task | Commit | Message |
| ---- | ------ | ------- |
| 1 | owner return `6346c68` on `witness-src/01-27` | `witness(01-27): stop C returns` (not merged; not a commit on this branch) |
| 2 | `3026133` | `docs(01-27): record the stop-C FAIL verdicts and the redacted evidence` |
| prep | `3f717f7` | `docs(01-27): prepare stop C — verified Harvey diff, KVM command, approval prompt` |

**Plan metadata:** the `docs(01-27): complete the halted stop-C plan` commit that carries this SUMMARY.

## Files Created/Modified

- `witness/WITNESS-LOG.md` — the Stop C section: status FAILED, the five step verdicts and eight app verdicts, evidence paths, decisive failure lines, and the green-rerun prerequisites.
- `witness/evidence/stop-c/full.log` — trimmed to 168 lines: preflight, every app header/verdict, the `boot-r9-tier1-x64` failure block, the `GATE-FAIL` and `FULL-NO-CAPTURE` list.
- `witness/evidence/stop-c/full.tsv` — the eight app results (7 PASS, `boot-r9-tier1-x64` FAIL).
- `witness/evidence/stop-c/soak.log` / `stress.log` — the two-line tap-precondition failure each.
- `witness/evidence/stop-c/equiv/report.tsv` — `drawterm-gate-x64 match`; all other apps `differ`.
- `witness/evidence/stop-c/equiv/rows.tsv` — the 24 marker rows, including `boot-r9-x64` 0/1 on `q35: paqfs root mounted from module`, `init: starting /bin/rc`, `^% `.
- `witness/evidence/stop-c/store_verify.log` — both tree paths `STORE-VERIFY-OK`.
- `witness/evidence/stop-c/summary.tsv` / `env.txt` — the returned step rows and the host preflight (`teeniebox`, `DEV_KVM=writable`).
- `harvey-flip/HARVEY-FLIP.md` — status changed to BLOCKED / NOT APPROVED with the stop-C outcome and the three `none` records.

## Decisions Made

- **Failed stop C is recorded, not repaired.** The plan's Task 2 failure branch says record the outcome and return a checkpoint; the three failures are named by cause, and the choice of fix iteration is left to the owner.
- **The tier-1 `fpprobe` is the one real gate failure.** `full.tsv` says FAIL; the capture shows `fpprobe: BAD note-spin handler ran 0 times`, `BAD note-regs handler ran 0 times`, `user trap: unhandled vector 1 pc=0x2027d9 addr=0x0`, `fpprobe: FAIL note`, `T1-FAIL fpprobe`, and `post-pass FAILED — missing T1-OK fpprobe`.
- **The six `FULL-NO-CAPTURE` rows are a harness copy-map defect.** The apps write `target/gates/<short-name>`; `step_full` copies `target/gates/<app-name>`. `drawterm-gate-x64` and `boot-r9-x64` are the only two names where the two agree, and only drawterm writes a `.log` there.
- **soak/stress did not test the venue.** `venue_gate` ran `nix run .#net-doctor -- --mode tap` from the r9 root with no `HARVEY`, so net-doctor defaulted to `$PWD` and read `<r9>/build/x86_64/root-tree/lib/netaddr.rc`. Both apps already export the correct `HARVEY_NETADDR_SEAM`; the step's doctor invocation is the defect.
- **The `boot-r9-x64` equivalence row needs confirmation.** The three markers are present in `full.log` at lines 552/565/585; the terminal-mode app writes no per-app `.log`, so the count saw an empty directory. It is recorded as a capture-path defect, not claimed as a boot regression.
- **Halted cleanly.** No owner approval was given, no Harvey commit exists, no push happened, and no later plan (01-28) was started; STATE.md/ROADMAP.md were not modified (worktree mode).

## Deviations from Plan

None - the plan executed exactly as written on its Task 2 failure branch (`If any verdict is FAIL … make no Harvey change. Record the outcome.`). The stop-C harness defects and the tier-1 gate failure are recorded as Issues Encountered, not as executor deviations.

## Issues Encountered

1. **`boot-r9-tier1-x64` — `T1-FAIL fpprobe`.** The fpprobe note legs did not run on the r9 x64 kernel: `BAD note-spin handler ran 0 times`, `BAD note-regs handler ran 0 times`, `BAD note child 129: bad`, then `user trap: unhandled vector 1 pc=0x2027d9 addr=0x0`, `fpprobe: FAIL note`, `T1-FAIL fpprobe`; the post-pass refused the missing `T1-OK fpprobe`. The rest of the tier-1 sweep passed (the gate reached `T1-DONE`).
2. **soak/stress — tap precondition.** Both steps stopped in 2 s / 0 s with `tap mode needs the netaddr seam: /home/req/code/r9/build/x86_64/root-tree/lib/netaddr.rc missing — run from the harvey repo root or set HARVEY`. The venue was never exercised; the fix is the witness step's `HARVEY` (or the tree path), not a venue change.
3. **`step_full` capture copy map.** Six apps' captures were not copied (`FULL-NO-CAPTURE`): `boot-r9-smoke-x64`, `boot-r9-tier1-x64`, `boot-r9-net-smoke-x64`, `boot-r9-services-x64`, `boot-r9-churn-x64`, `boot-r9-auth-x64`. Their gate dirs are the short names `smoke-r9-x64`, `tier1-r9-x64`, `smoke-r9-net-x64`, `services-r9-x64`, `churn-r9-x64`, `smoke-r9-auth-x64`.
4. **Equivalence gaps.** `equiv/report.tsv` is `drawterm-gate-x64 match` and `differ` everywhere else: the six `-1` rows are the copy gap; `boot-r9-x64` is 0/1 on three markers that do appear in `full.log` (the terminal-mode gate writes no per-app `.log`).
5. **The amd64 TCG check carried from stop B3 is untouched.** This plan did not rerun it; no new claim is made either way.

## Authentication Gates

None.

## Known Stubs

None. The failing tier-1 gate is a real gate result, not a stub; the evidence paths are complete.

## Threat Flags

None beyond the plan's threat model. The one approved-write boundary (T-01-86/T-01-87/T-01-88) was not crossed: no Harvey staging, commit or push occurred, and the retained diff is still the reviewed file.

## Next Phase Readiness

- **Halted at the owner checkpoint (D-10 step 4 not reached).** The Harvey diff remains prepared and uncommitted; a green stop-C rerun plus an explicit APPROVE are both required before Task 2's commit path can run.
- **01-28 was not started.** No r9-side pin bump, no lock of a Harvey commit, and no push occurred.
- **The owner's options:** (a) fix the stop-C harness defects (the soak/stress `HARVEY` seam; the full-runner capture copy and equivalence paths), investigate `boot-r9-tier1-x64`'s `fpprobe` failure and the `boot-r9-x64` markers, then rerun stop C; (b) investigate the tier-1 `fpprobe` kernel/FPU/note failure first; or (c) stop and re-scope HARN-08's x86_64 equivalence or the tier-1 gate. No option was chosen here.

## Self-Check: PASSED

- All nine evidence files exist under `witness/evidence/stop-c/`; every file is ≤200 lines; `grep -acE 'auth-creds|[Pp]assword='` is 0 on every file.
- WITNESS-LOG.md has exactly 5 `VERDICT stopc-(full|soak|stress|equiv|store-verify)` lines and 8 `VERDICT stopc-app-*` lines.
- HARVEY-FLIP.md records `HARVEY-COMMIT: none`, `HARVEY-APPROVED: none`, `HARVEY-PUSHED: none`.
- Commits `3f717f7` and `3026133` are ancestors of HEAD; `git rev-list --count 9f8a213..HEAD` = 2 before this SUMMARY.
- `/Users/neilskinner/code/harvey` is unchanged (HEAD `25d97dc4d…`, status hash `041d2929…`); no staging, commit or push.
- STATE.md and ROADMAP.md were not modified (worktree mode).
