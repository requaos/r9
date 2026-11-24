---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 08
subsystem: ds
tags: [ds, tpht, spill, chained-overflow, measurement, hit-rate, d-15, miri, no_std, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-06: TphtBucketHeader, FlattenedTphtShard, feistel::split/mix32/fingerprint, TinyPtr, the Probe seam and miri_scaled; plan 03-07: ChainedTphtOverflow (255-node banks, two-choice, WRITING publication, its serialisation contract); plan 03-13: deref_tiny and its unchecked arena read"
provides:
  - "ds::tpht::SPILL_THRESHOLD: usize = 16, the measured D-15 value, with evidence/SPILL-THRESHOLD.md holding the grid, the pre-committed rule and the chosen cell"
  - "the spill path: FlattenedTphtShard spills a block's entries past the threshold into its ChainedTphtOverflow under that block's seqlock; insert -> Result<Option<TinyPtr>, TphtError>, lookup -> Option<Found<WORDS>> with Found { value, home }"
  - "constructors: with_banks (zero banks = home-only shard), with_banks_and_probe (the general form), new/with_probe (default banks: one per 64 blocks, min 2) and the test-only with_test_config (threshold and index choice)"
  - "ds::tpht::measure (test only): five key distributions, the seeded generator, entries_at, exceeds_bar, banks_for, home_hit_fraction, the counting model for both index variants, GRID_BLOCKS and GRID_SEEDS"
  - "the over-90% hit-rate pin (all fifteen grid runs through the production constructor) and the index-bits comparison: mixed half L' versus the compendium's unmixed low bits of P"
  - "evidence/PINS-03-08.md: tpht-spill and tpht-hit-rate RED, GREEN-1, GREEN-2 and ADVANCED, the six REPIN lines, CELL-MATCH 15/15, MIRI and the two MIRI-OVER times"
affects: [03-09, 03-10, phase-4]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 28733
  tasks: 2
  commits: 8
  plan_head_before: 229ddfef4225ef33eb1e7a9f3579f95e77127073
  plan_head_after: c7b789337f9918c8f58dd0bc02b85eb4865b2367

tech-stack:
  added: []
  patterns:
    - "Spill under the home block's seqlock: insert checks the header, then the block's chain, places at home while the block holds fewer than SPILL_THRESHOLD entries, and otherwise calls ChainedTphtOverflow::insert; the overflow's serialisation contract is satisfied because every chain mutation happens under that block's lock"
    - "lookup scans the header and walks the block's chain inside one Boehm seqlock attempt and validates the generation after both, retrying on a change; Found::home says whether the entry came from the header or the chain"
    - "insert reports placement: Ok(Some(ptr)) at home, Ok(None) in a bank; a spilled entry has no tiny pointer and is reached only through lookup, never deref_tiny"
    - "A home-only shard (with_banks(n, 0)) keeps plan 03-06's 16-slot rule and reports HomeFull; a shard with banks never reports HomeFull (BanksFull when both candidates are full)"
    - "The D-15 rule and its pin are one computation: the rule was committed before the run, the pin re-runs the chosen cell through the production constructor, and the fifteen counts are diffed against the evidence's CELL lines"
    - "All measurement arithmetic is exact integers (entries_at, exceeds_bar = hits*10 > lookups*9, banks_for); printed decimals are for reading only"

key-files:
  created:
    - ds/src/tpht/measure.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/SPILL-THRESHOLD.md
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-08.md
  modified:
    - ds/src/tpht/mod.rs
    - ds/src/tpht/flattened.rs

key-decisions:
  - "SPILL_THRESHOLD = 16 at HIT_RATE_OCCUPANCY = 1.00, chosen by the rule committed before the measurement: at occupancy 1.00 T=16 is the only threshold over 0.90 for every distribution and seed (T=15 reaches 0.914 for uniform at 0.90 but 0.868 at 1.00); the weakest of the fifteen chosen runs is sequential 236,035/262,144 = 0.900402, recorded by MARGIN"
  - "The measurement agrees with RESEARCH F9's Python model: T=16, uniform 0.9010..0.9021 at occupancy 1.00, and T=12 below 0.90 already at 0.85"
  - "The mixed-half index is what makes the result hold for structured keys: at the chosen cell, pagekey is 0.9036 mixed versus 0.2500 with the compendium's unmixed low bits of P, and both upper-half-only families are 1.0000 versus about 0.0001; plan 03-09 records the correction in PROJECT.md"
  - "The duplicate check on insert is split: when the block can still place at home, the block's chain is searched so a spilled key is not stored twice; when the block spills, ChainedTphtOverflow::insert's own Exists check suffices"
  - "with_banks(n, 0) keeps plan 03-06's behaviour under a constructor that names it; the default overflow is one bank per 64 blocks (minimum 2), and the measurement/pin pass banks_for explicitly so no insert fails"
  - "The six tests advanced by plans 03-06 and 03-13 changed only their constructor calls, the unwrapping of the new insert/lookup results and the two placement assertions the new API makes possible; each is named by a REPIN line"
  - "Miri is over the 120-second budget and is reported rather than fixed by cutting coverage: Task 1 measured 981 s at the plan sizes and 315/329 s at the floors; Task 2 measured 628 s at the plan sizes and 402 s at the floors; seeds, tests and native sizes are unchanged, as plan 03-07's accepted 230 s precedent did"
  - "banks_full_is_an_error fills the one fixed 255-node bank with spills spread over 64 blocks (two home keys per block) so each chain stays short; the failing insert is the 256th spill and the table is shown unchanged"

requirements-completed: [DS-01]

coverage:
  - id: D1
    description: "Entries past SPILL_THRESHOLD are stored in and found from the dereference banks: insert returns Ok(Some) at home and Ok(None) in a bank, lookup returns Found with home Some/None, remove works on both, a home-only shard reports HomeFull and a shard with banks reports BanksFull"
    requirement: DS-01
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#spill_past_threshold_goes_to_a_bank, home_only_shard_never_spills, composite_ops_match_btreemap_model, banks_full_is_an_error (`cargo test -p ds --lib tpht::flattened` 13 passed)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri (64 seeds, rc 0): 29 passed per seed, no Undefined Behavior or Data race"
        status: pass
    human_judgment: false
  - id: D2
    description: "The spill threshold is measured by the pre-committed rule: T=16 at occupancy 1.00 is the choice, the weakest of the fifteen runs is 0.900402, and the source constant equals the evidence's CHOSEN line"
    requirement: DS-01
    verification:
      - kind: other
        ref: "evidence/SPILL-THRESHOLD.md: the rule commit b479d28 precedes the tables commit 4150faf; `sed -n 's/^CHOSEN SPILL_THRESHOLD=//p'` prints 16 and `grep -c 'pub const SPILL_THRESHOLD: usize = 16;'` prints 1"
        status: pass
      - kind: unit
        ref: "ds/src/tpht/measure.rs#measure_spill_threshold_table (the #[ignore]d generator, rc 0 in 6.93 s) and #table_agrees_with_counting_model"
        status: pass
    human_judgment: false
  - id: D3
    description: "Over 90% of present-key lookups resolve in the home block at the chosen occupancy for every grid distribution and seed, using the production SPILL_THRESHOLD: the pin re-runs the chosen cell and its fifteen counts equal the evidence's fifteen CELL lines"
    requirement: DS-01
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#hit_rate_home_block_exceeds_90_percent (`cargo test ...` 1 passed; fifteen CELL lines, weakest sequential 236035/262144)"
        status: pass
      - kind: other
        ref: "evidence/PINS-03-08.md: CELL-MATCH tpht-hit-rate 15/15 (both GREEN runs diffed clean against the evidence CELL lines)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The six tests advanced by plans 03-06 and 03-13 are re-pinned by name on a home-only shard with their assertions unchanged"
    requirement: DS-01
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#home_ops_match_btreemap_model, fingerprint_collisions_are_verified, stress_readers_never_see_torn_entry, deref_tiny_is_one_arena_read_and_no_header_read, deref_tiny_matches_btreemap_model, deref_tiny_wraps_with_slot_mask; `deref_tiny` filter 3 passed; `tpht::` 29 passed"
        status: pass
      - kind: other
        ref: "evidence/PINS-03-08.md: six `^REPIN (tpht-shard|deref-tiny) ` lines"
        status: pass
    human_judgment: false
  - id: D5
    description: "Both pins followed IG-V01: a committed RED that failed on assertions, a GREEN implementation, GREEN-1 and GREEN-2, and a pin advance"
    requirement: DS-01
    verification:
      - kind: other
        ref: "evidence/PINS-03-08.md: PIN tpht-spill and PIN tpht-hit-rate each have RED, GREEN-1, GREEN-2 and ADVANCED"
        status: pass
    human_judgment: false

# Metrics
duration: 51min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 08: The spill path and the measured spill threshold

**The flattened shard now spills into `ChainedTphtOverflow` under the home block's seqlock, and `SPILL_THRESHOLD` is fixed at the measured 16 by a rule committed before the run — over 90% of present-key lookups resolve at home at occupancy 1.00 for all five key distributions and three seeds, with the weakest run at 0.900402 and the mixed-half index exposed beside the compendium's collapsing unmixed reading.**

## Performance

- **Duration:** 51 min
- **Started:** 2026-11-24T16:01:10Z
- **Completed:** 2026-11-24T16:52:15Z
- **Tasks:** 2 (both TDD, RED before GREEN)
- **Files modified:** 5 (3 source files, 2 evidence files)

## Accomplishments

- **The spill path.** `FlattenedTphtShard` owns an `Option<ChainedTphtOverflow<WORDS>>`. A shard with banks places a block's first `SPILL_THRESHOLD` entries in the 16-slot header (`Ok(Some(ptr))`), then calls the overflow's `insert` for that block (`Ok(None)`), all under the block's seqlock. `lookup` scans the header, walks the block's chain inside the same Boehm attempt, and validates the generation after both; `remove` takes the entry from the header or the chain. A spilled entry has no tiny pointer and is reached only through `lookup`, never `deref_tiny`.
- **Constructors.** `with_banks(n, 0)` builds the home-only shard plan 03-06 had, kept under a name that says what it is: it applies no threshold, takes entries until all 16 slots are used and reports `HomeFull`. `with_banks_and_probe` is the general form; `new`/`with_probe` size the overflow at one bank per 64 blocks (minimum 2, 255 nodes per 1,024 home slots); `with_test_config` is the test-only threshold/index seam. `ChainedTphtOverflow::new` is untouched, and `TphtError::HomeFull`'s doc now says only a home-only shard returns it.
- **`insert` says where the entry went.** `Result<Option<TinyPtr>, TphtError>`: `Some(ptr)` at home, `None` in a bank, `Exists` for a duplicate, `BanksFull` when both candidate banks are full on a shard with banks. A duplicate whose home slot has since been freed is caught by a chain search before a home placement; when the block spills, the overflow's own `Exists` check covers it.
- **The measurement.** `ds/src/tpht/measure.rs` (test only) holds the five key distributions, the seeded SplitMix64 generator, `entries_at`, `exceeds_bar` (`hits*10 > lookups*9`), `banks_for`, `home_hit_fraction`, the counting model for both index variants, `GRID_BLOCKS = 16_384` and three literal `GRID_SEEDS`. The `#[ignore]`d generator prints the mixed, absent-key and unmixed tables for all five distributions, the `qualifies=` flags, the `RULE`, the fifteen `CELL` lines, `MARGIN` and `NEXT`.
- **The choice.** The rule committed at `b479d28` chose `SPILL_THRESHOLD = 16` at `HIT_RATE_OCCUPANCY = 1.00`: at occupancy 1.00 no other threshold is over 0.90 for all fifteen runs, and `T = 16` qualifies at every occupancy. The weakest run is sequential 236,035 / 262,144 = **0.900402**; the `MARGIN` line records it and the chosen occupancy is the top of the grid (`NEXT NONE`).
- **The pin.** `hit_rate_home_block_exceeds_90_percent` re-runs the chosen cell through the production `with_banks` constructor with the same generators, `entries_at`, `banks_for` and `exceeds_bar`. Its fifteen counts are byte-equal to the evidence's fifteen `CELL` lines in both GREEN runs (`CELL-MATCH tpht-hit-rate 15/15`); the 0.90 bar is asserted natively only (the Miri size is a 4-block table).
- **The index correction, side by side.** At `T = 16`, occupancy 1.00: `pagekey` 0.9036 mixed versus 0.2500 unmixed; `upper32` and `upper32_or7` 1.0000 versus about 0.0001; `sequential` 0.9004 versus 1.0000; `uniform` 0.9010..0.9021 versus 0.9003..0.9014. The evidence section states that plan 03-09 records the correction to compendium 4.1 (RESEARCH C4, Open Question 1), outside the IG-D09 lock.
- **The re-pins.** The six tests advanced by plans 03-06 and 03-13 (`home_ops_match_btreemap_model`, `fingerprint_collisions_are_verified`, `stress_readers_never_see_torn_entry`, `deref_tiny_is_one_arena_read_and_no_header_read`, `deref_tiny_matches_btreemap_model`, `deref_tiny_wraps_with_slot_mask`) moved to a home-only shard; each changed only its constructor, the unwrapping of `insert`/`lookup` and the two added `Some` placement assertions in `home_ops_match_btreemap_model`. Each has one `REPIN` line.
- **Verification.** `cargo test -p ds --lib tpht::` 29 passed, 1 ignored; `cargo xtask test` rc 0 with 30 `test result: ok` lines; `cargo xtask check` and `cargo xtask clippy` rc 0 with no `ds/` warning; `cargo xtask miri` rc 0 over 64 seeds with 29 tests per seed and no Undefined Behavior or Data race.

## Task Commits

Each task was committed atomically, RED before GREEN:

1. **Task 1, RED: `SPILL_THRESHOLD` at its unmeasured source value, `insert`/`lookup` return types, constructors, five failing tests and six re-pins** — `b0d458e` (test)
2. **Task 1, GREEN: the spill into `ChainedTphtOverflow` under the home block seqlock** — `9abb677` (feat)
3. **Task 1, pin advance (GREEN-1/2, `MIRI`, two `MIRI-OVER`, `ADVANCED`)** — `d6b1635` (evidence)
4. **Task 2, Step A: the measurement method and selection rule, before any run** — `b479d28` (evidence)
5. **Task 2, RED: the measurement harness and the failing hit-rate pin (all fifteen runs 0.2777..0.2778)** — `970b7d9` (test)
6. **Task 2, Step C: the measured grid, unedited, with venue, commit and command** — `4150faf` (evidence)
7. **Task 2, Step D: `SPILL_THRESHOLD = 16`, `HIT_RATE_OCCUPANCY = 1.00`, the `CHOSEN` lines and the index comparison** — `62675ab` (feat)
8. **Task 2, pin advance (`CELL-MATCH 15/15`, two `MIRI-OVER`, `ADVANCED`)** — `c7b7893` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `ds/src/tpht/flattened.rs` — module doc (home block and spill); `Found<WORDS>`; test-only `IndexChoice` and `with_test_config`; `with_banks_and_probe`, `with_banks`, `new`, `with_probe`; `split_key`; the spill-aware `insert`, `lookup` and `remove`; the five new tests and the six re-pins; `HIT_RATE_OCCUPANCY` and the hit-rate pin
- `ds/src/tpht/mod.rs` — the measured `SPILL_THRESHOLD = 16` with its D-15 doc; `#[cfg(test)] mod measure;`; `HomeFull`'s rewritten doc; the module doc's spill sentence
- `ds/src/tpht/measure.rs` — the test-only measurement harness: distributions, generator, exact arithmetic, both counting models, the two measure tests
- `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/SPILL-THRESHOLD.md` — the method and rule (unchanged after their own commit), the measured tables and the `CHOSEN` lines, the F9 comparison and the index-bits section
- `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-08.md` — the two pins' RED/GREEN-1/GREEN-2/ADVANCED, six `REPIN` lines, `CELL-MATCH 15/15`, `MIRI` and the `MIRI-OVER` times

## Decisions Made

See `key-decisions` in the frontmatter. The items the plan left to execution:

- **The rule's result.** `T = 16` at occupancy 1.00; the weakest run sits 0.000402 above the bar and the `MARGIN` line records it. No margin was added and no occupancy skipped.
- **`banks_full_is_an_error`'s construction.** One fixed 255-node bank shared by 64 blocks, two home keys per block, then 255 spills; this keeps Miri cheap without weakening the assertion (the 256th spill is `BanksFull`, the failed key is absent, home and spilled entries still resolve).
- **Default overflow size.** One bank per 64 blocks, minimum 2, documented in the constructor; the measurement and the pin size their banks explicitly so no insert fails at any `T`.

## Deviations from Plan

### Plan-authorized adjustments

**1. [Plan rule "Miri latency"] Task 1's Miri sizes were taken to the floors**
- **Found during:** Task 1, after GREEN
- **Measurement:** the first warm `cargo xtask miri` at the plan's sizes took 981 s; the run at the floors took 329 s and a second final-size run 315 s.
- **Adjustment:** composite 8 blocks/2 banks/200 operations → 4 blocks/2 banks/50 operations; the spill stress 60 → 50 operations per thread; `banks_full_is_an_error` fills one bank across 64 short chains. Native sizes, the 64 seeds and the test list are unchanged; no further round was tried because every sized test was at its floor. Recorded as `MIRI-OVER tpht-spill 981` and `MIRI-OVER tpht-spill 329`.
- **Files modified:** `ds/src/tpht/flattened.rs`
- **Verification:** `cargo xtask miri` rc 0, 27 tests per seed, no Undefined Behavior or Data race.
- **Committed in:** `9abb677` (code) and `d6b1635` (ledger)

**2. [Plan rule "Miri latency"] Task 2's pin and counting-model Miri sizes were taken to the floors**
- **Found during:** Task 2, after GREEN
- **Measurement:** the first timed run at the plan's sizes (16 blocks, 1 seed, 2 distributions) took 628 s; the run at the floors took 402 s.
- **Adjustment:** the hit-rate pin and `table_agrees_with_counting_model` now take 4 blocks, 1 seed and 1 distribution under Miri. Native sizes, the 64 seeds and the test list are unchanged. Recorded as `MIRI-OVER tpht-hit-rate 628` and `MIRI-OVER tpht-hit-rate 402`; still over the 120-second budget with every Miri size at its floor, so it is reported to the orchestrator rather than fixed by cutting coverage.
- **Files modified:** `ds/src/tpht/flattened.rs`, `ds/src/tpht/measure.rs`
- **Verification:** `cargo xtask miri` rc 0, 29 tests per seed, no Undefined Behavior or Data race.
- **Committed in:** `62675ab` (code) and `c7b7893` (ledger)

**3. [Sizing, no behaviour change] `banks_full_is_an_error` spreads its 255 spills over 64 blocks**
- **Found during:** Task 1's first Miri run (981 s)
- **Issue:** filling one bank from one block's chain made the insert-time scans quadratic and dominated the Miri run.
- **Fix:** two home keys per block, then spill 255 keys across the 64 blocks; the 256th spill still finds the bank full and the table is shown unchanged.
- **Verification:** the test passes natively and under Miri; the assertion is unchanged.

### Plan note

**4. The unsafe site's line moved.** Plan 03-13's unchecked arena read (`get_unchecked` in `deref_tiny`) is the only `unsafe` site in `ds/src/tpht/` after this plan; the new code shifted it from line 258 to line 440. Plan 03-10's `UNSAFE-AUDIT.md` entry must use the new line. No `unsafe` was added by this plan.

---

**Total deviations:** 3 (2 plan-authorized Miri resizes with recorded over-budget times, 1 test-sizing change with no behaviour change) plus the line-number note.
**Impact on plan:** none on the assertions, the native sizes, the 64 seeds or the test list. The Miri feedback budget is the one unmet plan rule; it is recorded and reported, not fixed by cutting coverage (the plan-03-07 precedent).

## Issues Encountered

- **Miri is over budget again.** The 03-07 suite was 230 s for 22 tests; the spill tests took it to 315-329 s and the hit-rate pin and counting-model test to 402 s at the resize floors. The functional gate is green (rc 0, 29 tests per seed, no Undefined Behavior or Data race), and the plan's latency rule says to report at the floors rather than cut seeds or tests. Task 2's verify includes an awk check for two `MIRI-TIME` lines inside 120 s; it does not pass because both pins recorded `MIRI-OVER` instead, exactly as plan 03-07 did.
- **The rule's margin is thin.** `SPILL_THRESHOLD = 16` at occupancy 1.00 puts the weakest run at 0.900402, only 0.000402 above the bar. The plan's no-margin rule forbids raising the bar or skipping a cell; the `MARGIN` line records it and the pin passes because it is the same computation on the same inputs.
- **RED-to-GREEN typing.** The first RED run showed the five new tests failing on `HomeFull`, as intended; the Green diff then added the chain checks. `rustfmt` was applied to the touched files before each commit (no separate formatting commit was needed).

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-30 (a threshold chosen to make the test pass) is mitigated by the rule committed at `b479d28` before the tables at `4150faf`, the raw tables, the source-to-evidence grep gate and `table_agrees_with_counting_model`; T-03-31 (a reader seeing the header and the chain from different generations) by the single-attempt header scan plus chain walk, the generation check after both, `stress_readers_with_spill` and Miri over 64 seeds; T-03-32 (structured keys defeating the home block) by the five distributions in the grid and the pin and the index-bits comparison. T-03-SC does not apply (no package was added).

## Known Stubs

None. Every RED skeleton was replaced by the GREEN implementation in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## For owner review

- **The chosen values.** `SPILL_THRESHOLD = 16`, `HIT_RATE_OCCUPANCY = 1.00`; the evidence is `evidence/SPILL-THRESHOLD.md` (rule at `b479d28`, measured tables at `4150faf`, `CHOSEN` lines in `62675ab`). The weakest of the fifteen runs is sequential 236,035/262,144 = 0.900402, and the chosen occupancy is the top of the grid.
- **The index-bits evidence.** At the chosen cell the mixed half gives `pagekey` 0.9036 and both upper-half families 1.0000, while the compendium's unmixed low bits of `P` give 0.2500 and about 0.0001. The correction is recorded for plan 03-09 (RESEARCH C4, Open Question 1); it is outside the IG-D09 lock.
- **A spilled entry has no tiny pointer.** A Phase 4 caller that wants `deref_tiny` for every entry must handle `insert`'s `Ok(None)`: the entry is reachable through `lookup` only. This is stated in `insert`'s and `deref_tiny`'s docs.
- **Miri over budget.** 315-329 s (Task 1) and 402 s (Task 2) at the resize floors against a 120-second budget; the functional gate is green and coverage is unchanged. This follows the plan-03-07 overage the orchestrator accepted.

## Next Phase Readiness

- Plan 03-09 can record the D-15 choice, the mixed-half correction and the Miri overage in PROJECT.md/STATE.md; the evidence file has the exact numbers and the index-bits table.
- Plan 03-10 has the one `unsafe` site to audit: `ds/src/tpht/flattened.rs:440`, the unchecked arena read in `deref_tiny` (its `// SAFETY:` comment names the invariant). No `unsafe` was added by this plan.
- Phase 4's `FidTable` and any `StatefulHandle` fast path must treat `insert -> Ok(None)` as a spilled entry with no tiny pointer, reached through `lookup`; the home path still returns `Some(ptr)` for `deref_tiny`.
- The `tpht` Miri suite is 402 s at the floors; a later phase that adds `tpht` tests must expect the latency rule to bind again.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All five claimed files exist on disk (`ds/src/tpht/flattened.rs`, `ds/src/tpht/mod.rs`, `ds/src/tpht/measure.rs`, `evidence/SPILL-THRESHOLD.md`, `evidence/PINS-03-08.md`) and all eight plan commits (`b0d458e`, `9abb677`, `d6b1635`, `b479d28`, `970b7d9`, `4150faf`, `62675ab`, `c7b7893`) are ancestors of HEAD. Plan-level verification: `cargo test -p ds --lib tpht::` 29 passed, 1 ignored; `cargo xtask test` rc 0 (30 `test result: ok` lines), `cargo xtask check` rc 0, `cargo xtask clippy` rc 0 with no `ds/` warning; `cargo xtask miri` rc 0 (64 seeds, 29 passed per seed, no Undefined Behavior or Data race, 402 s); the hit-rate pin passes with fifteen CELL lines; `CELL-MATCH tpht-hit-rate 15/15` and six `REPIN` lines are in the ledger; the evidence has 15 `CELL` lines, one `MARGIN`, the two `CHOSEN` lines and the unmixed column. **One task-level verify does not pass:** the awk check for two `MIRI-TIME` lines inside 120 s, because both pins recorded `MIRI-OVER` (981/329 and 628/402 s at the resize floors); that is the plan-authorized latency deviation recorded above and reported to the orchestrator.
