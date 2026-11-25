---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 01
subsystem: wire-contract
tags: [9p2000-aead, wire-decisions, kernel-bytes, evidence, ig-d11]

requires: []
provides:
  - "Four-line kernel byte baseline at the phase base commit (base, venue, two ELF sha256) plus text-symbol-byte and FP/SIMD-line metrics with exact reproducible commands"
  - "Owner answers on the eight wire proposals P1 to P8 (all accepted) recorded as DECISION lines"
  - "Owner answer on the IG-D11 lock (lock-amended) recorded as a DECISION line, ready for plan 04-13 to write the PROJECT.md row"
affects:
  - "04-02 through 04-13 (each reads 04-WIRE-DECISIONS.md before pinning a wire byte)"
  - "04-14 (re-runs the four recorded METRIC-COMMAND lines on the after-builds)"

actuals:
  tokens: 1634
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Baseline-before-change evidence: four-line before.sha256 + per-config metric lines + exact METRIC-COMMAND lines reading repo-relative build outputs"
    - "Wire decision sheet: PROPOSAL Pn lines, each superseded-or-confirmed by a dated DECISION Pn line"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/before.sha256
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/before.metrics
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-WIRE-DECISIONS.md
  modified: []

key-decisions:
  - "P1 to P8 accepted as written; P6 stays off and P7 closes and zeroizes the connection on a sequence violation"
  - "IG-D11 locked as amended, no wording changes"
  - "Kernel baseline is the four-line before.sha256 taken at base 3ce5ed8, with devshell GNU nm/objdump metric commands"

patterns-established:
  - "Evidence reproducibility: every metric line is paired with a METRIC-COMMAND that names its config's bare repo-relative build output, so plan 04-14 re-runs them unchanged and checks the sha256 of what they read"

requirements-completed: [WIRE-01]

coverage:
  - id: D1
    description: "Kernel byte baseline at the phase base commit: four-line before.sha256 and four numeric + four METRIC-COMMAND lines in before.metrics"
    requirement: WIRE-01
    verification:
      - kind: automated
        ref: "grep -cE '^(virtshell|q35churnshell) elf [0-9a-f]{64}$' before.sha256 => 2; numeric metric lines => 4; METRIC-COMMAND lines => 4"
        status: pass
    human_judgment: false
  - id: D2
    description: "Owner answers on the eight wire proposals P1 to P8 recorded as dated DECISION lines"
    requirement: WIRE-01
    verification:
      - kind: automated
        ref: "grep -cE '^DECISION P[1-8]: (accepted|amended)' 04-WIRE-DECISIONS.md => 8"
        status: pass
    human_judgment: false
  - id: D3
    description: "Owner answer on the IG-D11 lock recorded as DECISION IG-D11, PROJECT.md left untouched"
    requirement: WIRE-01
    verification:
      - kind: automated
        ref: "tail -1 04-WIRE-DECISIONS.md matches '^DECISION IG-D11: (lock-amended|hold)'; git diff --quiet $B HEAD -- .planning/PROJECT.md exits 0"
        status: pass
    human_judgment: false

duration: ~2 min (continuation; Task 1 ran in the prior session)
completed: 2026-11-25
status: complete
plan_head_before: 3ce5ed8c4e309710e2157ab5b63a03a40797d185
plan_head_after: 55959d77678da5b931e9856ec7e0eefe483a1863
---

# Phase 04 Plan 01: Kernel byte baseline and the owner's wire decisions Summary

**Before-state kernel bytes recorded at base 3ce5ed8, and all eight wire proposals plus the IG-D11 lock answered by the owner.**

## Performance

- **Duration:** ~2 min in this continuation (Task 1 executed in the prior session; commits 63f39a4 at 00:15:30Z through 55959d7 at 00:17:28Z)
- **Started:** 2026-11-25T00:15:30Z (first task commit)
- **Completed:** 2026-11-25T00:17:32Z
- **Tasks:** 3 (two `blocking-human` checkpoints answered by the owner)
- **Files modified:** 3 (all created)

## Accomplishments

- Recorded the phase base-state of both kernels at base commit `3ce5ed8`: whole-ELF sha256 for `virtshell` and `q35churnshell`, plus text-symbol-byte and FP/SIMD-line counts with exact reproducible commands (D-25, D-29).
- Task 1's four metric commands used the devshell's GNU `nm`/`objdump`. The measured counts matched research F1 exactly: `754232` / `759072` text-symbol bytes and `309` / `6` FP-SIMD lines. The plan's D-25/D-29 baseline is the four-line `before.sha256`.
- The owner accepted all eight wire proposals P1 to P8 as written (P6 off, P7 closes and zeroizes on a sequence violation) and locked IG-D11 as amended with no wording changes.

## Task Commits

Each task was committed atomically:

1. **Task 1: Baseline kernel bytes and draft the proposal sheet** - `63f39a4` (evidence)
2. **Task 2 (owner decision, P1 to P8)** - `d9c52f8` (docs)
3. **Task 3 (owner decision, IG-D11)** - `55959d7` (docs)

**Plan metadata:** committed separately below (docs: complete plan).

## Files Created/Modified

- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/before.sha256` - `base`, `venue`, `virtshell elf`, `q35churnshell elf`.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/before.metrics` - four numeric lines and four `METRIC-COMMAND` lines.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-WIRE-DECISIONS.md` - eight `PROPOSAL` lines and nine `DECISION` lines.

## Decisions

All nine owner decisions, quoted verbatim from `04-WIRE-DECISIONS.md`:

```
DECISION P1: accepted (2026-11-25)
DECISION P2: accepted (2026-11-25)
DECISION P3: accepted (2026-11-25)
DECISION P4: accepted (2026-11-25)
DECISION P5: accepted (2026-11-25)
DECISION P6: accepted (2026-11-25)
DECISION P7: accepted (2026-11-25)
DECISION P8: accepted (2026-11-25)
DECISION IG-D11: lock-amended (2026-11-25) — locked with the proposed amended wording, no wording changes
```

- P1 to P8 accepted as written. P6 `blake3` `zeroize` stays off; P7 closes and zeroizes the connection on a sequence violation.
- IG-D11 locked as amended with no wording changes. Plan 04-13 writes the dated PROJECT.md row; this plan does not edit PROJECT.md.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 04-02 can start with every wire value it names either accepted or replaced by the owner.
- The `before.sha256` / `before.metrics` baseline is the exact four-line evidence plan 04-14 re-runs on the after-builds.
- PROJECT.md remains untouched by this plan; plan 04-13 owns the IG-D11 row.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- All four expected files exist on disk (`before.sha256`, `before.metrics`, `04-WIRE-DECISIONS.md`, `04-01-SUMMARY.md`).
- All three plan commits (`63f39a4`, `d9c52f8`, `55959d7`) are ancestors of HEAD.
