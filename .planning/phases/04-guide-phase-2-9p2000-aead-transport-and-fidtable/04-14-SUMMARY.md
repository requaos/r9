---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 14
subsystem: planning-evidence
tags: [9p2000-aead, code-review, kernel-bytes, miri, qemu, owner-stop]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: the cumulative port/src wire codec, transport, and fid table of plans 04-02 to 04-13
provides:
  - the one Phase 4 owner review of the cumulative port/src change, APPROVED
  - closing suite evidence (four suite commands twice, test-9p twice, Miri once)
  - the D-29 kernel byte evidence set for virtshell and q35churnshell
  - the aarch64 quick pass on HVF and TCG
  - the x86_64 Linux owner-stop checklist with the boot-iso outputHash step
affects: [04-15, phase-05]

# Actuals (#2632) — same estimateTokens scale as the plan's estimate (chars/4 over the diff).
actuals:
  tokens: 12052
  tasks: 3
  commits: 5
  plan_head_before: adb890d6e1e4f6854257e6b1312cf448a618c8b8
  plan_head_after: 5e1a62b4a4334143147ceabe4ccca000b5836e79

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Evidence at the reviewed commit: the review verdict names the reviewed sha and Task 3 re-checks that no source path moved since it"
    - "Measure the after-build by hashing the build output immediately before and after the metric commands, and require both to equal the recorded after ELF"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PORT-SRC-REVIEW.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/after-04-14.sha256
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/after.metrics
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/KERNEL-BYTES.md
  modified:
    - .planning/STATE.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/closing-suite.md
    - xtask/tests/threat_model.rs

key-decisions:
  - "Owner approved the cumulative port/src change at f72cc2a (VERDICT port-src-review: APPROVED); Task 3 then measured at the approval commit 3398df4 with no source change since."
  - "Both whole-ELF hashes differ from the baseline (virtshell 9218b998… -> 0ae5d574…, q35churnshell c9dcef0b… -> c850fb64…); Phase 2's .text-identical rule is not used (D-29)."
  - "Text symbols grew by only 988 bytes on aarch64 and 1,065 on x86_64; x86_64 uses 760,137 of 2,097,152 bytes, leaving 1,337,015 bytes of headroom."
  - "FP/SIMD scan counts are EQUAL to the baseline on both kernels (309 aarch64, 6 x86_64): no FP/SIMD instruction was introduced."
  - "Miri took 348 s, over the 120 s band; reported as the accepted Phase 3 deviation rather than cut."

patterns-established:
  - "Phase-end review verdict line carries HEAD=<reviewed sha>; Task 3 asserts the source diff from that sha to HEAD is empty"
  - "A from-nothing kernel build is proven to be what the metrics read by hashing the build output before and after the run"

requirements-completed: [WIRE-12, WIRE-13]

coverage:
  - id: D1
    description: "The owner reviewed the whole cumulative port/src change of Phase 4 and approved it before any push (D-24)."
    requirement: WIRE-12
    verification: []
    human_judgment: true
    rationale: "The approval is the owner's own reply; no test can assert a human decision."
  - id: D2
    description: "The four suite commands and test-9p passed twice each and Miri passed once at the reviewed commit."
    requirement: WIRE-13
    verification:
      - kind: integration
        ref: "target/phase04-closing/closing/status.txt and per-run logs; VERDICT suite-run-1/2, test-9p-run-1/2, miri"
        status: pass
    human_judgment: false
  - id: D3
    description: "The D-29 kernel byte evidence set for virtshell and q35churnshell, measured from a from-nothing build whose sha256 equals the recorded after ELF."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "evidence/kernel-bytes/after-04-14.sha256, after.metrics, KERNEL-BYTES.md; MEASURED hashes equal the elf lines"
        status: pass
    human_judgment: false
  - id: D4
    description: "The aarch64 quick pass: tier-1 on HVF and TCG, smoke, net-smoke and cpu, each ending ALL STAGES PASS."
    requirement: WIRE-12
    verification:
      - kind: e2e
        ref: "target/phase04-closing/quick/status.txt; VERDICT quick-tier1-hvf, quick-tier1-tcg, quick-smoke, quick-net-smoke, quick-cpu"
        status: pass
    human_judgment: false
  - id: D5
    description: "04-OWNER-STOP.md: the ordered x86_64 Linux checklist with run_logged verdicts, host tests before boot-iso, the owner's hash commit, and the native TCG gate."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "04-OWNER-STOP.md exists; grep 'gate --arch x86-64 --accel tcg' matches; flake.nix unchanged"
        status: pass
    human_judgment: true
    rationale: "Whether the owner's host run passes can only be known by running the page on that host; the page's content is checked here, the outcome is not."

duration: 23 min
completed: 2026-11-24
status: complete
---

# Phase 4 Plan 14: Phase-end review, closing evidence on the Mac, and the owner-stop page Summary

**The owner approved the whole `port/src` change, and at that commit the Mac recorded two green suite runs plus Miri, a from-nothing kernel byte change with EQUAL FP/SIMD counts and 1.27 MiB of x86_64 headroom, a five-boot aarch64 quick pass, and the x86_64 Linux owner-stop checklist.**

## Performance

- **Duration:** 23 min (continuation: Task 2 record through Task 3 and SUMMARY)
- **Started:** 2026-11-24T20:22:09-05:00 (Task 1's STATE.md commit)
- **Completed:** 2026-11-24T20:45:28-05:00 (Task 3 evidence commit)
- **Tasks:** 3/3
- **Files modified:** 8 across the plan (5 created, 3 modified)

## Accomplishments

- **D-24 review, approved.** The owner read the cumulative `port/src` change at `f72cc2a` and replied "approve"; `VERDICT port-src-review: APPROVED HEAD=f72cc2ad421600f2a727f25a9708e30867729461` is committed. Task 3 then confirmed `git diff --quiet f72cc2a HEAD -- port crypto ds xtask aarch64 x86_64 Cargo.lock` is clean at the measurement commit `3398df4`.
- **Closing suites twice.** `cargo xtask test`, `cargo test -p xtask`, `cargo xtask check`, `cargo xtask clippy`, and `cargo xtask test-9p` each passed twice; `cargo xtask miri` passed once in 348 s. Ten closing PASS verdicts are in `closing-suite.md`.
- **Kernel bytes by D-29.** Both kernels rebuilt from nothing. Whole-ELF sha256 changed on both (expected, D-25): virtshell `9218b998…` → `0ae5d574…`, q35churnshell `c9dcef0b…` → `c850fb64…`. Defined text symbol totals: virtshell 754,232 → **755,220** (+988); q35churnshell 759,072 → **760,137** (+1,065). x86_64 uses **760,137 / 2,097,152** bytes, leaving **1,337,015 bytes (≈1.27 MiB)** of headroom. FP/SIMD scan counts are **EQUAL** (309 aarch64, 6 x86_64). The measured file's sha256, taken before and after the metric commands, equals the after ELF on both configs.
- **aarch64 quick pass.** tier-1 on HVF (47 s) and TCG (13 s), smoke (11 s), net-smoke (17 s), cpu (46 s) — all exit 0 with `ALL STAGES PASS`.
- **Owner-stop page.** `04-OWNER-STOP.md` gives the ordered nine-section x86_64 Linux checklist, every verdict command wrapped in `run_logged` (logs under `~/r9-p04-stop/`), host tests before `boot-iso`, and the TCG gate run natively on the owner's host.

## Task Commits

Each task was committed atomically:

1. **Task 1: STATE.md rows, pre-review suite, review bundle** - `b84adca` (docs: STATE.md W1/W2/W4), `39d1a29` (fix: red-tree repair), `f72cc2a` (evidence: pre-review suite and review bundle)
2. **Task 2: owner approval recorded** - `3398df4` (evidence: owner approved the port/src change)
3. **Task 3: closing evidence and owner-stop page** - `5e1a62b` (evidence: closing suite, kernel byte evidence, aarch64 quick pass, owner-stop page)

**Plan metadata:** the SUMMARY commit (docs: complete 04-14 plan)

## Files Created/Modified

- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PORT-SRC-REVIEW.md` — the review bundle the owner read (Task 1)
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/closing-suite.md` — review verdict, suite/Miri verdicts, quick-pass verdicts
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/after-04-14.sha256` — four-line after record at `3398df4`
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/after.metrics` — four numeric lines, the unchanged `METRIC-COMMAND` lines, two `MEASURED` lines
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/KERNEL-BYTES.md` — the D-29 page with seven verdict lines
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP.md` — the owner's Linux-host checklist
- `.planning/STATE.md` — W1, W2 and W4 rows removed from the open-question table (D-10)
- `xtask/tests/threat_model.rs` — red-tree repair (test-only deviation, see below)

## Decisions Made

- Recorded the review verdict at `f72cc2a` and measured at `3398df4`, the `.planning`-only approval commit, because the plan makes the review evidence its own commit; the source diff between them is empty and was checked.
- Used the D-29 replacement evidence set (whole-ELF hashes, text symbol totals, FP/SIMD counts, 2 MiB check) rather than Phase 2's `.text`-identical rule, because linking `crypto`/`ds` changes `.text` by itself.
- Reported Miri's 348 s as the accepted Phase 3 deviation instead of trimming coverage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Red-tree repair in the threat-model pin (test-only)**
- **Found during:** Task 1 (pre-review suite)
- **Issue:** The tree was red at the dispatched HEAD: the pin at `xtask/tests/threat_model.rs:234` still required every section-5 row of `docs/threat-model.md` to say `not implemented`, while plan 04-13 commit `038ef15` had moved the six Phase 4 rows to the D-23 status under the owner's authorization.
- **Fix:** The pin now accepts the D-23 status and still rejects a Phase 4 row that claims the code is used by a booted kernel (`threat_model_protections_are_not_overclaimed`). Test-only: `xtask` is host-only and linked into neither kernel, so the kernel byte baseline is unaffected.
- **Files modified:** `xtask/tests/threat_model.rs`
- **Verification:** the pre-review suite (and both closing suite runs) pass; the reviewer was offered the graph at the repaired commit.
- **Committed in:** `39d1a29`

---

**Total deviations:** 1 auto-fixed (1 bug, owner-authorized). **Impact:** none on scope or kernel bytes; it was required to offer the review on a green tree, as the plan demands.

## Issues Encountered

None. No stop rule fired: both FP/SIMD verdicts are EQUAL, `text-under-2mib-q35churnshell@04-14` is PASS, and every suite run passed.

## Owner review

`VERDICT port-src-review: APPROVED HEAD=f72cc2ad421600f2a727f25a9708e30867729461`. The owner's reply was "approve"; no fix was requested, so nothing was changed in this plan beyond the recorded evidence.

## Verdict ids

- `pre-review-test`, `pre-review-xtask-tests`, `pre-review-check`, `pre-review-clippy`, `pre-review-test-9p` (Task 1)
- `port-src-review: APPROVED` (Task 2)
- `suite-run-1`, `suite-run-2`, `test-9p-run-1`, `test-9p-run-2`, `miri`, `MIRI-TIME closing 348` (Task 3)
- `quick-tier1-hvf`, `quick-tier1-tcg`, `quick-smoke`, `quick-net-smoke`, `quick-cpu` (Task 3)
- `elf-virtshell@04-14: DIFFERENT`, `elf-q35churnshell@04-14: DIFFERENT`, `text-symbols-virtshell@04-14: RECORDED before=754232 after=755220`, `text-symbols-q35churnshell@04-14: RECORDED before=759072 after=760137`, `fpsimd-virtshell@04-14: EQUAL`, `fpsimd-q35churnshell@04-14: EQUAL`, `text-under-2mib-q35churnshell@04-14: PASS`

## User Setup Required

None on the Mac. The owner's x86_64 Linux run is the next step, in `04-OWNER-STOP.md` (planned as plan 04-15's `human-action`/stop): push branch `phase-04/aead-wire`, run host tests, build the kernels, re-derive and commit the `boot-iso` `outputHash`, then the KVM, native TCG and full gates.

## Next Phase Readiness

- Everything the Mac can prove is in evidence at the owner-approved commit `5e1a62b` (measured at `3398df4`); `flake.nix` is unchanged and nothing was pushed.
- Plan 04-15 presents the branch push and this owner-stop page; the x86_64 kernel boot to rio and the `boot-iso` hash remain the Linux host's to prove.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-24*

## Self-Check: PASSED

All five files this plan created exist on disk (`PORT-SRC-REVIEW.md`, `04-OWNER-STOP.md`,
`after-04-14.sha256`, `after.metrics`, `KERNEL-BYTES.md`), and all five plan commits
(`b84adca`, `39d1a29`, `f72cc2a`, `3398df4`, `5e1a62b`) are ancestors of HEAD.
