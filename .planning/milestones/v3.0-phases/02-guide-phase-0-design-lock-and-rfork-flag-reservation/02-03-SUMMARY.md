---
phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
plan: 03
subsystem: planning
tags: [design-lock, decisions, d-17, owner-checkpoint, project-md, roadmap, requirements, state]

# Dependency graph
requires:
  - phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
    provides: the phase context decisions D-01 to D-08 and D-17, and the research status table in 02-RESEARCH.md Q7
provides:
  - "PROJECT.md Key Decisions: exactly one row per IG-D01 to IG-D13 — 8 locked, 2 locked amended, 3 not locked and deferred — with fixed outcome strings"
  - "PROJECT.md Key Decisions lead-in: the D-08 owner-only-change rule and the landing-site clarification (owner answered yes)"
  - "PROJECT.md Safety constraint and footer: amended IG-D07 per-module forbid wording; Phase 2 design-lock date"
  - "ROADMAP Phase 2 success criterion 2 and open-questions sentence, DSGN-03, and the STATE.md RFNOLEGACY and precedence lines: the D-17 one-`port/`-module wording"
  - "02-DECISION-TABLE.md: the thirteen-row table, the meaning of locked, the landing-site question and the owner's ruling verbatim"
affects: [phase-03, phase-04, phase-05, phase-06, phase-07]

# Actuals (#2632) — measured on the same chars/4 scale as the plan's estimate.
actuals:
  tokens: 7217
  tasks: 3
  commits: 7
  plan_head_before: abed9b8e825cbc5011a7acd2073cd557854f340a
  plan_head_after: 05585c9c4dd6442f4859e3973e3ca22fbf59ed81

tech-stack:
  added: []
  patterns:
    - "D-17 wording is pinned by counts: zero arch source paths in ROADMAP and REQUIREMENTS, exactly one in PROJECT (N3's report of the AC claim), and `port/src/rfork.rs` counts of 2 / 1 / 2"
    - "Lock outcomes use three fixed strings (`Locked DATE (owner)`, `Locked amended DATE (owner)`, `Not locked; deferred to Phase N (W..)`) so one grep can check all thirteen rows"
    - "The owner rules once on a committed table; the reply is recorded verbatim before any Key Decisions row is written"

key-files:
  created:
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-DECISION-TABLE.md
  modified:
    - .planning/PROJECT.md
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
    - .planning/STATE.md

key-decisions:
  - "Owner confirmed the table in bulk on 2026-11-23; the landing-site question was answered yes. The ruling is recorded verbatim as `confirm` and `Landing-site question: yes`."
  - "The combined TPHT/Roaring row was split: IG-D09 (TPHT primary, Funnel scratchpad only) is not locked and deferred to Phase 3 (W5, W6); IG-D10 (Roaring) is locked."
  - "IG-D03 is one of the two amended locks (four `rfork` flag bits, not three); IG-D07 is the other (per-module forbid at HEAD, three named `unsafe` modules)."
  - "The D-17 wording was applied before the owner stop as plan content; no Key Decisions row for an IG decision was written before the reply."

patterns-established:
  - "A design lock is recorded as a machine-checkable thirteen-row gate over fixed outcome strings, with the owner's reply kept beside the proposal it confirmed"

requirements-completed: [DSGN-05, DSGN-03]

coverage:
  - id: D1
    description: "The D-17 amendment is applied at its six wording sites: the flags live in one `port/` module, `port/src/rfork.rs`, imported by both arch crates"
    requirement: "DSGN-03"
    verification:
      - kind: other
        ref: "grep counts: 0 arch paths in ROADMAP+REQUIREMENTS; 1 in PROJECT (N3's AC claim); 2/1/2 `port/src/rfork.rs` in ROADMAP/REQUIREMENTS/PROJECT; 1 `Reserved in r9 only, in one` in STATE"
        status: pass
    human_judgment: false
  - id: D2
    description: "The thirteen-decision table with proposed statuses, the meaning of locked and the landing-site question was committed and presented at the owner stop"
    requirement: "DSGN-05"
    verification: []
    human_judgment: true
    rationale: "The owner read the thirteen rows and answered the landing-site question; the table's substance (shortened decisions that keep the guide's meaning) is a judgment call."
  - id: D3
    description: "The owner's ruling is recorded verbatim with its date, and PROJECT.md Key Decisions records exactly one row per IG-D01 to IG-D13 in the three fixed outcome forms"
    requirement: "DSGN-05"
    verification:
      - kind: other
        ref: "13-row gate = 13; locked/as-written = 8; locked amended = 2; not locked and deferred = 3; `confirm in Phase 2` = 0; `PENDING` = 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "PROJECT.md states the lock rule and the landing-site limit, its Safety constraint matches the amended IG-D07, and STATE.md records the Phase 2 lock result"
    requirement: "DSGN-05"
    verification:
      - kind: other
        ref: "lead-in contains `owner-only change`, `new dated row` and `landing-site file paths`; `Nothing is locked` in STATE = 0; footer carries the design-lock date"
        status: pass
    human_judgment: false

duration: 5min
completed: 2026-11-23
status: complete
---

# Phase 2 Plan 03: The Design Lock Summary

**The guide's thirteen decisions are recorded in PROJECT.md — eight locked as written, IG-D03 and IG-D07 locked amended, IG-D09, IG-D11 and IG-D13 not locked and deferred — and the `rfork` flags now read as one `port/` module at all six D-17 wording sites.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-11-24T02:27:02Z
- **Completed:** 2026-11-24T02:31:48Z
- **Tasks:** 3 (Task 2 is the blocking owner stop; the ruling lands in Task 3)
- **Files modified:** 5 (4 planning files and 1 new table)

## Accomplishments

- D-17 applied at six sites: ROADMAP Phase 2 success criterion 2 and its open-questions sentence, DSGN-03, PROJECT.md note N3 and the `RFNOLEGACY` row, and the STATE.md `RFNOLEGACY` line. ROADMAP and REQUIREMENTS name no arch source file; PROJECT keeps exactly one, N3's report of what AC section 1.1 claims.
- `02-DECISION-TABLE.md` committed with thirteen rows (8 `Locked as written`, 2 amended, 3 `Not locked; deferred to Phase N (W..)`), the D-08 meaning of locked and the landing-site question.
- Owner stop: the owner replied `confirm` for the table as proposed and `yes` to the landing-site question. The reply is recorded verbatim under `## Owner ruling` with the date.
- PROJECT.md Key Decisions now has exactly one row per IG-D01 to IG-D13, the D-08 lock rule, the landing-site clarification, an amended Safety constraint and a design-lock footer. STATE.md no longer says nothing is locked.

## Task Commits

1. **Task 1: Apply the D-17 wording and write the thirteen-decision table**
   - `acc87ad` — the six D-17 wording sites (ROADMAP, REQUIREMENTS, PROJECT, STATE)
   - `59374d2` — `02-DECISION-TABLE.md` for owner confirmation
2. **Task 2: OWNER STOP — thirteen-decision table** — no commit (the ruling is recorded in Task 3's commit)
3. **Task 3: Record the owner's ruling** — `05585c9` (PROJECT.md Key Decisions and Safety bullet, STATE.md precedence line, owner ruling in the table)

**Plan metadata:** `docs(02-03): complete the design lock plan` (this SUMMARY)

The plan ledger recorded `abed9b8` before the first task commit. The range `abed9b8..05585c9` counts 7 commits: this plan's 3 plus 4 from plan 02-01, which completed on the same branch between Task 1 and Task 3. `actuals.commits` is the measured range, per the instrument.

## Files Created/Modified

- `.planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-DECISION-TABLE.md` — thirteen decisions, proposed statuses, D-08 rule, landing-site question, owner ruling
- `.planning/PROJECT.md` — Key Decisions rows for IG-D01 to IG-D13, the lock lead-in, the amended Safety bullet, the N3 wording, the `RFNOLEGACY` rationale, the footer
- `.planning/ROADMAP.md` — Phase 2 success criterion 2 and the open-questions sentence per D-17 and D-23
- `.planning/REQUIREMENTS.md` — DSGN-03 per D-17
- `.planning/STATE.md` — the `RFNOLEGACY` decision line per D-17 and the precedence line's Phase 2 result

## Decisions Made

- Owner confirmed all thirteen proposed statuses in bulk (2026-11-23); no individual ruling was needed.
- The landing-site answer was yes: a lock covers the decision, not the guide's landing-site file paths, which are re-verified when each phase is planned.
- The existing combined TPHT/Roaring row was split so no Key Decisions row cites two IG ids.
- `IG-D03` and `IG-D07` are the two `Locked amended` rows; the remaining locked rows use `Locked` with the owner date.

## Deviations from Plan

### Auto-fixed Issues

**1. [Process note] Task 1's STATE.md commit also carries the orchestrator's phase-transition edits**
- **Found during:** Task 1
- **Issue:** `.planning/STATE.md` already had uncommitted phase-transition edits before this plan started (`current_phase: 02`, timestamps, `state_head`, Current Position). The plan requires each task to end clean and committed.
- **Fix:** staged the whole file with the task's edit. The plan's own change is the single `RFNOLEGACY` line; no progress-tracking field was authored by this plan.
- **Files modified:** `.planning/STATE.md`
- **Verification:** `git show acc87ad -- .planning/STATE.md` shows the transition hunk plus the one-line D-17 edit; the orchestrator keeps the progress fields.
- **Committed in:** `acc87ad`

---

**Total deviations:** 0 rule-driven auto-fixes; 1 process note.
**Impact on plan:** none on content. The file is committed as the plan requires, and no progress tracking was changed by this plan's authoring.

## Issues Encountered

- Plan 02-01 completed four commits on the same branch between Task 1 and Task 3, so the ledger range counts those commits too. This is why `actuals.commits` is 7 while this plan made 3; the split is named under Task Commits.
- The pre-existing untracked GSD/tooling paths (`.claude/`, `.planning/config.json`, `.planning/graphs/`, `.planning/state.json`, `.planning/ui-reviews/`) were present before this plan and were not committed or modified by it.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The lock is in force: Phases 3 to 7 read PROJECT.md Key Decisions to learn what is fixed and what is open. Changing a locked row now requires the owner and a new dated row.
- Plans 02-04, 02-05 and 02-06 remain in Phase 2. This plan does not complete DSGN-03's reservation itself; 02-01 created the module and 02-04/02-05 close the import, test and evidence legs.
- No blockers introduced.

---
*Phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation*
*Completed: 2026-11-23*

## Self-Check: PASSED

- `02-DECISION-TABLE.md` exists; `02-03-SUMMARY.md` exists.
- Commits `acc87ad`, `59374d2`, `05585c9` are ancestors of HEAD.
- Thirteen-row gate = 13; `confirm in Phase 2` = 0; `PENDING` = 0.
