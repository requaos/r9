---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 23
subsystem: testing
tags: [code-review, gap-closure, review-disposition, owner-approval, fid-table, insert-lock, aead-transport]

requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: "the twelve gap findings closed by plans 04-18 to 04-22, each with an ADVANCED ledger pin; the owner-approved and gated GAPBASE source"
provides:
  - "04-REVIEW-DISPOSITION.md: twelve gap findings recorded fixed with plan+commit, seven info findings left open"
  - "evidence/closing-suite-gaps.md: five pre-review PASS verdicts at the amendment HEAD plus the owner's APPROVED verdict"
  - "evidence/PORT-SRC-REVIEW-GAPS.md: the one owner review page for the whole gap-closure port/src diff"
  - "the owner's approval of the gap-closure port/src change at 6fd201c, unblocking plans 04-24 and 04-25"
affects: [04-24, 04-25]

actuals:
  tokens: 11883
  tasks: 2
  commits: 6
  plan_head_before: 639005bcdc27e3005ba063fe4a6338017ea1031d
  plan_head_after: 9f74085f18b87d154f5e3bd8f5a6809891aa3efa

tech-stack:
  added: []
  patterns:
    - "One diff-first owner review at phase end (D-24) for all port/src edits, before any evidence that depends on the bytes"
    - "Table-level insert lock making an all-shard duplicate probe and the placement walk one indivisible step"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PORT-SRC-REVIEW-GAPS.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/closing-suite-gaps.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-23-SUMMARY.md
  modified:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-REVIEW-DISPOSITION.md

key-decisions:
  - "Approved with the insert lock: the owner required the fallover-window racing-insert case fixed before approving, then approved at 6fd201c"
  - "The reviewed HEAD moved from a669317 to 6fd201c because any port/src change after the first cut voids the pre-review evidence; all five legs were re-run at the new HEAD"
  - "The WR-09 Source cell stays machine-readable as 04-19 e0f7427; the racing-insert amendment is a note below the findings table"

patterns-established:
  - "A finding is 'fixed' only when its plan's ledger shows the pin ADVANCED; the table row carries that commit"
  - "An owner-requested fix inside a review stop is new source work; the pre-review evidence is re-run at the new HEAD rather than carried over"

requirements-completed: [WIRE-03, WIRE-04, WIRE-05, WIRE-06, WIRE-08, WIRE-09, WIRE-13]

coverage:
  - id: D1
    description: "04-REVIEW-DISPOSITION.md records the twelve gap findings CR-01..CR-03, WR-01..WR-09 as fixed with plan+commit and leaves the seven info findings open"
    requirement: WIRE-09
    verification:
      - kind: other
        ref: "grep -cE '^\\| (CR-0[1-3]|WR-0[1-9]) \\| (critical|warning) \\| fixed \\| 04-(18|19|20|21|22) [0-9a-f]{7,40} \\|$' 04-REVIEW-DISPOSITION.md => 12"
        status: pass
      - kind: other
        ref: "grep -cE '^\\| IN-0[1-7] \\| info \\| open \\|' 04-REVIEW-DISPOSITION.md => 7"
        status: pass
    human_judgment: false
  - id: D2
    description: "evidence/closing-suite-gaps.md holds five pre-review PASS verdicts at the amendment HEAD 6fd201c"
    verification:
      - kind: other
        ref: "grep -cE '^VERDICT pre-review-(test|xtask-tests|check|clippy|test-9p): PASS' evidence/closing-suite-gaps.md => 5"
        status: pass
    human_judgment: false
  - id: D3
    description: "evidence/PORT-SRC-REVIEW-GAPS.md is the single review page for the whole gap-closure port/src diff, range GAPBASE..6fd201c"
    verification:
      - kind: other
        ref: "grep -c 'git diff 5d1549af07de7220d6f48aac0d0db221aa7c541c .* -- port/src' evidence/PORT-SRC-REVIEW-GAPS.md => 1"
        status: pass
    human_judgment: true
    rationale: "Whether the page is an adequate presentation of the diff is the owner's judgment; it was reviewed and accepted."
  - id: D4
    description: "The owner approved the gap-closure port/src change at 6fd201c with the diff shown"
    verification:
      - kind: manual_procedural
        ref: "evidence/closing-suite-gaps.md#VERDICT port-src-review-gaps: APPROVED HEAD=6fd201c072d9c42f4de6e6d617c0ac35cf20d013"
        status: pass
    human_judgment: true
    rationale: "The approval is the owner's own reply; no automation can assert it."

duration: 48 min
completed: 2026-11-25
status: complete
---

# Phase 04 Plan 23: the gap-closure findings record and the owner's `port/src` approval Summary

**Twelve review findings recorded fixed with their pins, the fallover-window racing-insert case closed by a table-level insert lock on the owner's request, and the whole gap-closure `port/src` diff approved at `6fd201c` after all five pre-review legs were re-run there.**

## Performance

- **Duration:** 48 min
- **Started:** 2026-11-25T13:29:00Z
- **Completed:** 2026-11-25T14:18:00Z
- **Tasks:** 2 (Task 1 findings record + pre-review + bundle; Task 2 owner review, answered "approve with the lock")
- **Files modified:** 5 in the plan range (3 evidence/planning created or extended, 1 disposition document, plus the source/test files of the owner-requested fix)

## Accomplishments

- `04-REVIEW-DISPOSITION.md` now carries the twelve gap findings `fixed`, each with its plan and the `ADVANCED` commit of that plan's ledger, and the seven `IN` findings unchanged and `open`.
- The five pre-review legs (`cargo xtask test`, `cargo test -p xtask`, `cargo xtask check`, `cargo xtask clippy`, `cargo xtask test-9p`) passed at the amendment HEAD; all five verdicts are recorded at `6fd201c`.
- `evidence/PORT-SRC-REVIEW-GAPS.md` gives the owner one page for the whole `GAPBASE..HEAD` `port/src` diff, the `DECISION P9` line quoted, the deviations and open questions of the five gap summaries, and the seven info findings left open.
- The owner required the fallover-window racing-insert case fixed before approving. `FidTable::insert` now holds a table-level `insert_lock` across the all-shard probe and the placement walk, so two concurrent inserts of one `(client_id, fid_key)` cannot both land.
- The owner approved the gap-closure `port/src` change at `6fd201c` with the diff shown.

## Task Commits

Each task was committed atomically:

1. **Task 1: findings record** - `a669317` (docs: record the twelve gap findings as fixed)
2. **Task 1: pre-review suite and review bundle** - `68c2286` (evidence: pre-review suite and the gap-closure port/src review bundle)
3. **Task 2 (owner-requested fix): serialise FidTable inserts (WR-09)** - `6fd201c` (fix)
4. **Task 2: refresh pre-review suite and bundle after the insert lock** - `1ac030e` (evidence)
5. **Task 2 (continuation): normalise the WR-09 disposition cell** - `61e4f51` (docs)
6. **Task 2 (continuation): owner approved the gap-closure port/src change** - `9f74085` (evidence)

**Plan metadata:** the SUMMARY/tracking commit that follows this file (docs: complete plan)

_Note: `a669317` and `1ac030e` are the two evidence-envelope commits the review stop produced; the fix between them is real source and required re-running every pre-review leg._

## Files Created/Modified

- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-REVIEW-DISPOSITION.md` - twelve gap findings `fixed` with plan+commit, seven info findings `open`, and the WR-09 amendment note below the table.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/closing-suite-gaps.md` - five pre-review PASS verdicts at `6fd201c`, the superseded `a669317` block, the insert-lock RED/GREEN note, and the owner's `APPROVED` verdict.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PORT-SRC-REVIEW-GAPS.md` - the review page: range by two full shas, `git diff --stat`, the per-finding table, the reading commands, the twelve points worth the eye, the `DECISION P9` quote, deviations/open questions, and the seven info findings.
- `port/src/fid.rs` - `FidTable::insert_lock` (`"fid-insert"`) taken before the all-shard duplicate probe and held through the placement walk; lock order `insert_lock` → `active` → `reclaimed`; `resolve` untouched.
- `port/tests/aead_fid.rs` - `aead_fid_insert_racing_pair_lands_exactly_one_entry`: 32 scoped threads insert one key over 64 rounds while another frees a home-block slot; asserts exactly one `Ok`.

## The reviewed range

- **GAPBASE** `5d1549af07de7220d6f48aac0d0db221aa7c541c` — the commit `origin/main` holds, the source the owner approved and gated in plans 04-14 and 04-15.
- **HEAD** `6fd201c072d9c42f4de6e6d617c0ac35cf20d013` — the insert-lock amendment commit, the last source change in the range and the commit the owner approved.
- `port/src` holds exactly two changed files: `port/src/chan/transport.rs` (+483/−…) and `port/src/fid.rs` (+291/−…); `xtask/src/test9p.rs` is outside `port/src` and needs no approval. Task 1's base check proved the first gap-closure source commit `38acbb5` has a parent `3041ded` with no source difference from `GAPBASE`, so everything in the range is the gap closure and nothing else.

## The twelve fixed findings and their pins

| Finding | Plan | Commit | Ledger pin |
|---|---|---|---|
| CR-01 | 04-18 | `a5dca03` | `gap-cr01-exchange-secret` |
| CR-02 | 04-21 | `bb7e83c` | `gap-cr02-wr07-routing` |
| CR-03 | 04-21 | `62f14e7` | `gap-cr03-unpadded-transcript` |
| WR-01 | 04-18 | `6b8b043` | `gap-wr01-wr02-rekey-lifecycle` |
| WR-02 | 04-18 | `6b8b043` | `gap-wr01-wr02-rekey-lifecycle` |
| WR-03 | 04-18 | `6530fca` | `gap-wr03-helper-keystream` |
| WR-04 | 04-19 | `a9be163` | `gap-wr04-wr06-resolve-guards` |
| WR-05 | 04-22 | `fcc01dc` | `gap-wr05-sweep-recorded-key` |
| WR-06 | 04-19 | `a9be163` | `gap-wr04-wr06-resolve-guards` |
| WR-07 | 04-21 | `bb7e83c` | `gap-cr02-wr07-routing` |
| WR-08 | 04-20 | `18ff8e5` | `gap-wr08-per-file-legs` |
| WR-09 | 04-19 | `e0f7427` | `gap-wr09-fallover-duplicate` |

Each of the nine ledger pins carries an `ADVANCED` line, so each `fixed` row stands on the commit that actually advanced the pin.

## The owner's change request: racing inserts (WR-09) and the insert lock

The owner read the first cut of the diff (`a669317`) and required one thing before approving: the two-racing-inserts open question had to be resolved first. WR-09's all-shard probe covered a *repeated* insert after a fallover, and the single-lock rule covered mutation and bookkeeping per shard, but two inserts racing at the same instant could still both pass the probe and land in different shards.

The fix (`6fd201c`) adds a **table-level** `insert_lock` (`"fid-insert"`) that `insert` takes before the all-shard probe and holds until it returns, so the probe's read and the placement walk it licenses are one indivisible step. Lock order is `insert_lock` → `active` → `reclaimed`; `clunk`, `mark_reclaimed`, `sweep` and `bitmap_lens` take no `insert_lock`, so it never nests inside a shard lock; `resolve` stays lock-free with one arena read.

**The fallover window the pin drives:** the key's home block is full, one racer walks it and falls over to the next shard, a home slot is freed mid-race, and a racer that probed earlier lands at home. Without the lock the two racers land one entry in each shard.

**RED evidence:** with the `insert_lock` acquisition removed from `insert` (locally, not committed), `aead_fid_insert_racing_pair_lands_exactly_one_entry` fails with two winners in two shards — `got 2 on shard(s) [1, 0]` (rounds 1, 3, 5, 9, 13 and 20 observed across runs). **GREEN with the lock:** the binary is `36 passed; 0 failed` over repeated runs.

## Five fresh pre-review verdicts, and the superseded `a669317` block

Because the insert lock changed `port/src/fid.rs` and `port/tests/aead_fid.rs`, every pre-review leg was re-run at `6fd201c` rather than carried over:

```
VERDICT pre-review-test: PASS exit=0
VERDICT pre-review-xtask-tests: PASS exit=0
VERDICT pre-review-check: PASS exit=0
VERDICT pre-review-clippy: PASS exit=0
VERDICT pre-review-test-9p: PASS exit=0
```

The earlier `a669317` block is kept in `closing-suite-gaps.md` as history and marked **superseded** — the lock changed source after that run, so those verdicts are not the tree the owner reviewed.

## The owner's approval

The verdict line, quoted from `evidence/closing-suite-gaps.md`:

```
VERDICT port-src-review-gaps: APPROVED HEAD=6fd201c072d9c42f4de6e6d617c0ac35cf20d013
```

**The two owner answers:** first, "changes — resolve the racing inserts before I approve", which produced `6fd201c`; then, with the lock in place and the five legs green, **"approve with the lock"**.

## The seven info findings left `open`

None is in the gaps block and none is dispositioned by this plan; they are the owner's to disposition.

| Finding | Title |
|---|---|
| IN-01 | Module docs describe the pre-implementation state (partially corrected by plan 04-18) |
| IN-02 | `DataLayout::pages` is computed and never used; `crypt_body` recomputes it with a literal |
| IN-03 | `wire::is_t_message` has no production caller |
| IN-04 | `Msg`'s hand-written `PartialEq` ends in `_ => false`, which hides a new variant |
| IN-05 | The threat-model overclaim guard only fires on rows carrying the Phase 4 status |
| IN-06 | The end-to-end capability test claims "zero FID memory" but does not measure it |
| IN-07 | The `ds` removal test's "the freed slot takes another key" is not asserted |

## Decisions Made

- **Approved with the lock**, not without it: the first cut's racing-insert open question was closed as source work inside the review stop rather than deferred.
- **The review HEAD moved to `6fd201c`** because a source change after the first cut voids the pre-review evidence; all five legs were re-run rather than reused.
- **The WR-09 Source cell stays machine-readable** (`04-19 e0f7427`); the racing-insert amendment lives as a note below the findings table so the twelve-row anchored grep stays at 12.

## Deviations from Plan

The plan anticipated one review stop and no source edit of its own. The owner's change request is the only deviation, and the plan explicitly says an owner-requested fix is new work that restarts from Task 1:

**1. [Rule 4 - Owner decision] Insert lock added inside the review stop**
- **Found during:** Task 2 (owner review of the first cut at `a669317`)
- **Issue:** The bundle listed the fallover-window racing-insert case as an open question; the owner required it fixed before approving.
- **Fix:** Added `FidTable::insert_lock` and the racing-pair pin in `port/tests/aead_fid.rs` (`6fd201c`); re-ran all five pre-review legs at the new HEAD (`1ac030e`).
- **Files modified:** `port/src/fid.rs`, `port/tests/aead_fid.rs`, `evidence/closing-suite-gaps.md`, `evidence/PORT-SRC-REVIEW-GAPS.md`, `04-REVIEW-DISPOSITION.md`.
- **Verification:** RED then GREEN as quoted above; five PASS verdicts at `6fd201c`.
- **Committed in:** `6fd201c` (fix), `1ac030e` (evidence refresh).

**Total deviations:** 1 (owner-directed, not auto-fixed).
**Impact on plan:** No scope creep beyond what the owner asked for; the approval path stayed the same shape (review → verdict), only the reviewed HEAD advanced.

## Issues Encountered

None. The first-cut review surfaced a real defect through the owner's judgment; that was the review working as designed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 04-24** (kernel byte evidence) may start: it records the closing evidence at the approved commit `6fd201c`, and any later `port/src` change voids it.
- **Plan 04-25** (owner-stop / push offer) may start after 04-24; the `APPROVED` verdict is the gate those plans require.
- Nothing was pushed by this plan, and no file under `evidence/kernel-bytes/` was written.

## Self-Check: PASSED

- `04-REVIEW-DISPOSITION.md` anchored grep prints 12; IN rows print 7; amendment note below the table — PASSED
- `evidence/closing-suite-gaps.md` has five pre-review PASS verdicts and `VERDICT port-src-review-gaps: APPROVED HEAD=6fd201c...` — PASSED
- `evidence/PORT-SRC-REVIEW-GAPS.md` names `git diff 5d1549af07de7220d6f48aac0d0db221aa7c541c .. -- port/src` — PASSED

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*
