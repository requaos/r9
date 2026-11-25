---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 22
subsystem: wire-fid-table
tags: [fid, tpht, wire-08, wire-09, wr-05, concurrency, multi-block, tdd, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "ds::tpht::flattened::FlattenedTphtShard (insert, remove, slot_id, read_slot, deref_tiny), ds::roaring::RoaringBitmap, ds::tpht::Probe"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: "plans 04-09/04-10: FidEntry, FidShard/FidTable, the one-read resolve and the sweep; plan 04-19: the WR-09 duplicate probe and the WR-06 ptr guard"
provides:
  - "fid::ActiveSlots — the live slot ids plus the TPHT key recorded at insert, behind the shard's active lock"
  - "FidTable::sweep removes by the recorded key, never by reading the slot (WR-05)"
  - "insert/clunk/sweep hold a shard's active lock across the TPHT mutation and the bookkeeping; resolve takes none"
  - "two concurrency pins and four multi-block pins; evidence/PINS-04-22.md"
affects: ["04-VERIFICATION.md WIRE-09 re-check and the coincidental_reliance_items entry", "04-23 gap-closure review checkpoint (port/src diff)", "04-24/04-25 kernel byte evidence (port/src changed)"]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan estimate.
actuals:
  tokens: 8315
  tasks: 2
  commits: 4
  plan_head_before: 5fa066201025162cfb610c8fcaa28b468b1baf20
  plan_head_after: 483114d6adb00136a2e312a90edb49a9768c3b5b

tech-stack:
  added: []
  patterns:
    - "D-03 / IG-V01: RED pin against the old code, GREEN fix, GREEN run twice, ADVANCED hash in the ledger"
    - "Bookkeeping identity is (slot id, recorded TPHT key); the sweep's membership is still one bitmap and-not"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-22.md
  modified:
    - port/src/fid.rs
    - port/tests/aead_fid.rs

key-decisions:
  - "The sweep removes by the key recorded in ActiveSlots; the slot's current contents are never consulted (assumption_delta promote: (slot, recorded key) is the bookkeeping identity)"
  - "insert, clunk and sweep hold the shard's active lock across the TPHT mutation and the bookkeeping, then reclaimed, always active-first; resolve takes neither"
  - "The sweep's full-key check compares FidEntry word 0 to the recorded key (word 0 is exactly tpht_key), so sweep calls neither read_slot nor FidEntry::from_words"
  - "The concurrency pin releases its helpers from the sweep's probe hook when the sweep reads (old code) and from the test after the sweep when it does not (recorded-key sweep), so the pin is deterministic on both"

requirements-completed: [WIRE-08, WIRE-09]

# Coverage metadata (#1602) — per-deliverable traceability for verify-work.
coverage:
  - id: D1
    description: "The failover sweep removes only the entries its snapshot named, by the recorded key, so a live newcomer that reused a swept slot survives (WR-05)"
    requirement: WIRE-09
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_concurrent_clunk_and_reinsert_keeps_the_newcomer"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_bookkeeping_matches_live_entries_under_churn"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_matches_set_model_on_random_inputs"
        status: pass
    human_judgment: false
  - id: D2
    description: "A shard's TPHT mutation and bitmap bookkeeping are one critical section, so the active bitmap names exactly its live entries under churn"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_bookkeeping_matches_live_entries_under_churn"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_is_one_arena_read"
        status: pass
    human_judgment: false
  - id: D3
    description: "Block indices above 0 are exercised end to end through the fid table (the coincidental-reliance item): the sweep equals its BTreeSet model, resolve is one arena read, clunk clears one slot, and survivors resolve"
    requirement: WIRE-09
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_matches_set_model_on_multi_block_shards"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_is_one_arena_read_on_multi_block_shards"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test-9p (model test ok on the default and scalar-backend legs)"
        status: pass
    human_judgment: false

duration: 3min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 22: Sweep by recorded key, and the fid table pinned on multi-block shards Summary

**The failover sweep now removes by the TPHT key recorded at insert time, never by reading the slot, so a live newcomer that reused a swept slot survives (WR-05); and four pins drive block indices above 0 end to end through the fid table, closing the coincidental-reliance item.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-11-25T04:49:30Z
- **Completed:** 2026-11-25T04:53:17Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- **Gap WR-05 closed.** `FidShard`'s live set becomes `Lock<ActiveSlots>` — a `RoaringBitmap` of dense slot ids plus a `BTreeMap<u32, u64>` from slot id to the TPHT key recorded when the entry was inserted. `sweep` takes each collected id's recorded key and removes *that* key's entry; it never calls `read_slot` and never builds a key from slot contents, so a slot clunked and reused during the sweep keeps the live newcomer and a slot that reads as zeros can never name key `(0, 0)` (T-04-66).
- **Shard mutation and bookkeeping are one critical section.** `insert`, `clunk` and `sweep` all hold the shard's `active` lock across their TPHT mutation and their bitmap/key update, then its `reclaimed` lock — the same order everywhere (`active` first). `resolve` is untouched: no lock, exactly one `deref_tiny` arena read and zero header reads (T-04-67).
- **The pin was RED, then green.** `aead_fid_sweep_concurrent_clunk_and_reinsert_keeps_the_newcomer` fails on the old sweep (B is destroyed) and passes five consecutive runs on the fix; `aead_fid_sweep_bookkeeping_matches_live_entries_under_churn` guards the bitmap/live-entry agreement under churn.
- **The coincidental-reliance item is closed.** Four pins exercise block indices above 0 through the table: `aead_fid_sweep_matches_set_model_on_multi_block_shards` (64 seeds, 4 shards × 4 blocks, 120 entries, a per-seed assertion that a modelled entry sits above block 0), `aead_fid_resolve_is_one_arena_read_on_multi_block_shards`, `aead_fid_clunk_on_multi_block_shards_clears_only_its_slot`, and `aead_fid_sweep_survivors_on_multi_block_shards`. All four passed on their first run.
- **Model independence.** `model_slot` re-derives the slot id `(block << 4) | (ptr - 1)` from the documented formula `client_id ^ fmix32(fid_key)` using literal constants `0x85EB_CA6B` / `0xC2B2_AE35`; it calls nothing in `ds` or `port`.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: replant the concurrency pins** – `a0f8c5f` (test) — `test(04-22): RED pin for a clunk and reinsert during the sweep`
2. **Task 1 GREEN: sweep by recorded key under one lock** – `fcc01dc` (fix) — `fix(04-22): sweep by recorded key; shard mutation and bookkeeping under one lock (WR-05)`
3. **Task 2: multi-block fid table pins** – `1773deb` (test) — `test(04-22): fid table pins on multi-block shards`
4. **Ledger advance** – `483114d` (docs) — `docs(04-22): record multi-block pin GREEN runs and ADVANCED`

**Plan metadata:** the SUMMARY commit that follows this file.

_Note: TDD commits are `test` (RED) then `fix` (GREEN); the pin ADVANCED hashes are `fcc01dc` (`gap-wr05-sweep-recorded-key`) and `1773deb` (`gap-multi-block-fixture`)._

## Files Created/Modified

- `port/src/fid.rs` — `ActiveSlots` (ids + recorded keys) behind `Lock<ActiveSlots>`; `insert`/`clunk`/`mark_reclaimed`/`sweep` serialised per shard; `sweep` removes by recorded key
- `port/tests/aead_fid.rs` — `SweepWindowProbe`, the two concurrency pins, `fmix32`/`model_slot`, and the four multi-block pins (35 `aead_fid_` tests total)
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-22.md` — pin ledger (`gap-wr05-sweep-recorded-key`, `gap-multi-block-fixture`; `PASS-AT-FIRST-RUN` for all four multi-block tests)

## Multi-block pins: first-run result and production fixes

| Pin | First run | Production fix |
|---|---|---|
| `aead_fid_sweep_matches_set_model_on_multi_block_shards` | PASS-AT-FIRST-RUN | none |
| `aead_fid_resolve_is_one_arena_read_on_multi_block_shards` | PASS-AT-FIRST-RUN | none |
| `aead_fid_clunk_on_multi_block_shards_clears_only_its_slot` | PASS-AT-FIRST-RUN | none |
| `aead_fid_sweep_survivors_on_multi_block_shards` | PASS-AT-FIRST-RUN | none |

The multi-block pins exposed no defect in `port/src/fid.rs`; the table was already right on multi-block shards and the fixture now proves it end to end. No `ds/` edit was needed.

## Decisions Made

- **Bookkeeping identity promoted:** the pair (slot id, recorded TPHT key) names a live entry; the slot's current contents are no longer consulted. The bare slot id is kept only as the bitmap member the sweep's one and-not runs over.
- **Lock discipline:** a shard's `active` lock covers the TPHT mutation and the bookkeeping of `insert`, `clunk`, `mark_reclaimed` and `sweep`; `reclaimed` is always taken after `active`; `resolve` takes neither and stays one arena read.
- **Full-key check by word 0:** `FidEntry` word 0 is exactly `tpht_key(client_id, fid_key)`, so the sweep's `same` closure is `value[0] == recorded_key` — no unpacking, and the sweep calls neither `read_slot` nor `FidEntry::from_words`.
- **Deterministic concurrency pin:** the helper threads are released by the sweep's probe hook when the old sweep makes its arena read, and by the test itself once the sweep returns when the recorded-key sweep makes none — so the pin is deterministic in both directions.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The RED run reproduced the WR-05 failure on the first attempt; the churn guard passed at RED (recorded as such) and remains a regression guard. All four multi-block pins passed on their first run.

## User Setup Required

None - no external service configuration required.

## Threat Flags

None. No new trust boundary is introduced: the change strengthens the existing "failover sweep → live fids" boundary by removing by recorded key under the shard lock (T-04-66) and serialising shard mutation and bookkeeping (T-04-67).

## Next Phase Readiness

- WR-05 is closed and its pin is RED-then-GREEN; the `coincidental_reliance_items` entry is closed by the four multi-block pins. `WIRE-08`/`WIRE-09` are ready for the verification re-check.
- `port/src/fid.rs` changed, so kernel bytes change: plans 04-24 and 04-25 must redo the byte evidence and the owner stop.
- The `port/src` diff is available for the plan 04-23 gap-closure review checkpoint before any kernel evidence and before any push.

## Owner action before wave 3

Plans 04-23, 04-24 and 04-25 (waves 3 to 5) run only on the main working tree. Before continuing `/gsd-execute-phase 4 --gaps-only`, set `workflow.use_worktrees` to `false` in `.planning/config.json` (or through `/gsd-config`), and set it back to `true` after plan 04-25. The executor does not change the setting itself.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- Created/modified files exist on disk: `port/src/fid.rs`, `port/tests/aead_fid.rs`, `evidence/PINS-04-22.md`, `04-22-SUMMARY.md`
- Task commits exist on HEAD: `a0f8c5f` (RED), `fcc01dc` (GREEN), `1773deb` (multi-block pins), `483114d` (ledger advance)
- `port/tests/aead_fid.rs` runs 35 `aead_fid_` tests, all passing; concurrency pin 5/5; `cargo xtask test`/`check`/`clippy`/`test-9p` all rc 0
