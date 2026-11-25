---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 07
subsystem: ds
tags: [ds, tpht, chained-overflow, two-choice, writing-sentinel, atomics, miri, no_std, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-06: feistel::mix32 and fingerprint, index_bits, TphtError and the miri_scaled sizing rule; plan 03-13: the floored Miri sizes and the 120-second feedback-budget precedent"
provides:
  - "ds::tpht::chained::ChainedTphtOverflow<const WORDS>: new, candidate_banks, insert, lookup, remove, fill"
  - "BANK_NODES = 255 and WRITING = 0xFF; two-choice placement over mix32(block) and mix32(block ^ 0x9E37_79B9); the two-phase publication (claim -> payload -> Release publish -> Release link)"
  - "evidence/PINS-03-07.md: chained-overflow and chained-publication RED, GREEN-1, GREEN-2 and ADVANCED, the FILL chained fractions, MIRI chained and the recorded MIRI-OVER chained time"
affects: [03-08, 03-10, phase-4]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 9770
  tasks: 2
  commits: 5
  plan_head_before: 53e41a5de8104e15b3e6bdce98a9483bcb7cef22
  plan_head_after: 0da79e1ecc58c332f40a154ea29c2a85b6c906ee

tech-stack:
  added: []
  patterns:
    - "A bank is exactly 255 nodes indexed 0..=254; a link is (bank choice << 8) | node index with 0xFFFF as the end marker, so a link's index byte is never 0xFF"
    - "Two-choice placement: a block's candidates are mix32(block) % num_banks and mix32(block ^ 0x9E37_79B9) % num_banks (adjusted apart when equal and more than one bank); the new node goes to the candidate with more free nodes, ties to the first, and falls back to the other when a claim fails"
    - "Two-phase publication: compare-exchange the state byte free -> WRITING (0xFF), store fingerprint/key/value/next Relaxed, store the state LIVE with Release, then link the node at the block's head with Release; a reader that sees a non-LIVE state treats the node as absent"
    - "Every node field is an atomic; the module holds no unsafe and copies neither listing 9.4's pointer-cast write nor its 7-bit pointer packing"
    - "A per-bank used counter plus a rotating claim cursor: used rises at the claim and falls on remove; fill() is (sum of used, num_banks * 255) and the claim scan starts at a rotating index"
    - "A test-only WritingHook counts nodes currently in WRITING and the highest claimed index; the concurrency pins use it to show a claim happened and none is left half-published"

key-files:
  created:
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-07.md
  modified:
    - ds/src/tpht/chained.rs

key-decisions:
  - "The node's 16-bit filter is feistel::fingerprint over the full key (this standalone overflow's whole remainder); the collision test re-derives the pair (37, 568), found by an independent sequential search, and asserts the shared fingerprint before inserting both"
  - "A link names a node by (candidate choice << 8) | node index, with 0xFFFF as the end marker; the 255 indices are 0..=254 and the index byte is never 0xFF, which the link scan and the test hook assert"
  - "Nodes are entirely atomic (state, fingerprint, key, [AtomicU64; WORDS] value, next); this satisfies DS-04's publication without any unsafe site and avoids the listing 9.4 undefined behaviour that RESEARCH Pitfall 6 names"
  - "The used counter rises at the claim, not at publication, so two-choice sees in-flight claims as occupied; fill() therefore equals the successful inserts in both the model test and the concurrent stress test"
  - "The two-choice fallback test constructs a full first candidate bank through the private claim path (255 WRITING nodes, unlinked) and then proves a real insert lands in the second bank; this is deterministic and keeps the test cheap under Miri"
  - "Miri-side sizes were taken to the plan's floors (4 blocks, 50 operations, 2 threads with 30 operations; one bank where two are not needed). The warm run still measured 230 s (a second run 235 s) against the 120 s budget; per the plan's latency rule no third round was tried, no seed or test was cut, and the time is recorded as MIRI-OVER chained 230 for the orchestrator"

requirements-completed: [DS-04]

coverage:
  - id: D1
    description: "ChainedTphtOverflow provides 255-node banks, two-choice placement, full-key verification on a fingerprint match, a BTreeMap model over insert/lookup/remove, exactly 255 nodes per bank and the fallback to the second candidate bank"
    requirement: DS-04
    verification:
      - kind: unit
        ref: "ds/src/tpht/chained.rs#ops_match_btreemap_model, fingerprint_collisions_are_verified_by_full_key, bank_holds_exactly_255_nodes, two_choice_fails_over_to_second_bank, empty_overflow_reads_nothing (`cargo test -p ds --lib tpht::chained` 9 passed; the six-task filter 6 passed twice)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri (64 seeds): rc 0, 22 tests per seed, no Undefined Behavior or Data race"
        status: pass
    human_judgment: false
  - id: D2
    description: "The WRITING (0xFF) publication is safe under concurrency: concurrent claimers in shared banks never share a node, a reader never returns a value from an unpublished node, and no link or claimed index is 0xFF"
    requirement: DS-04
    verification:
      - kind: unit
        ref: "ds/src/tpht/chained.rs#stress_concurrent_claims_never_share_a_node, stress_reader_never_sees_partial_node, writing_sentinel_is_never_a_node_index (two native stress runs, 4 threads each natively and 2 under Miri)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri (64 seeds): rc 0, no Undefined Behavior, Data race or FAILED line"
        status: pass
    human_judgment: false
  - id: D3
    description: "The banks reach over 95% fill before the first insert that finds both candidate banks full: 16,190..16,265 of 16,320 nodes (0.9920..0.9966) over 8 literal seeds with 64 banks and 4,096 blocks"
    requirement: DS-04
    verification:
      - kind: unit
        ref: "ds/src/tpht/chained.rs#fill_exceeds_95_percent_before_first_failure (`nix develop -c cargo test -p ds --lib tpht::chained::tests::fill_exceeds_95_percent_before_first_failure -- --nocapture`: FILL chained min=0.9920)"
        status: pass
      - kind: other
        ref: "evidence/PINS-03-07.md: `FILL chained seeds=8 min=0.9920` with the eight per-seed fractions"
        status: pass
    human_judgment: false
  - id: D4
    description: "A warm cargo xtask miri run stays inside 03-VALIDATION.md's 120-second feedback budget"
    verification:
      - kind: integration
        ref: "nix develop -c cargo xtask miri: rc 0, but 230 s on the first warm floor run and 235 s on the second"
        status: fail
    human_judgment: true
    rationale: "The suite grew from 13 to 22 tpht tests; at the plan's Miri size floors it takes about 230 s, and the plan forbids cutting seeds or tests. The plan's latency rule says to report the times to the orchestrator instead of a third resize round."

# Metrics
duration: 32min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 07: `ChainedTphtOverflow` — two-choice 255-node banks with the `WRITING` publication

**Two-choice 255-node dereference banks with a `WRITING` (0xFF) two-phase publication, full-key verification on a fingerprint match, and 99.2% fill before the first `BanksFull`, green under 64-seed Miri and native stress; the Miri suite runs 230 s at the resize floors and is reported against the 120-second budget.**

## Performance

- **Duration:** 32 min
- **Started:** 2026-11-24T15:27:04Z
- **Completed:** 2026-11-24T15:59:38Z
- **Tasks:** 2
- **Files modified:** 2 (one source file, one ledger)

## Accomplishments

- `ChainedTphtOverflow<const WORDS>` in `ds/src/tpht/chained.rs` owns one atomic chain head per home block and a boxed slice of banks. `BANK_NODES` is 255 and `WRITING` is `0xFF`; a node index is `0..=254`, so a link's index byte is never `0xFF`.
- Two-choice placement: `candidate_banks(block)` is `mix32(block) % num_banks` and `mix32(block ^ 0x9E37_79B9) % num_banks` (the second moved to the next bank when the two are equal and there is more than one bank). `insert` sends the new node to the candidate with more free nodes (ties to the first) and falls back to the other when a claim fails; both full returns `Err(TphtError::BanksFull)`.
- Two-phase publication: `claim` is a compare-exchange of the state byte from free to `WRITING`; the fingerprint, full key, value words and next link are stored `Relaxed`; the state goes to `LIVE` with `Release`; only then is the node linked at the block's head with `Release`. Readers load the head and state with `Acquire`, treat a non-`LIVE` node as absent, check the 16-bit fingerprint and then the full 64-bit key.
- `remove` unlinks the node first, then sets it free with `Release` and returns it to the bank's used counter. The module doc records the serialisation contract: claims in a shared bank are concurrent and covered by `WRITING`; mutations of one block's chain must be serialised by the caller (the block's seqlock in the flattened shard); concurrent readers of a chain with one inserting writer are safe.
- `fill()` is `(sum of the banks' used counters, num_banks * 255)`. The fill pin reports 16,190..16,265 of 16,320 nodes (fractions 0.9920..0.9966) over eight literal seeds with 64 banks and 4,096 blocks, and prints every fraction; the minimum is 0.9920, well over the 0.95 bar.
- The model test agrees with `BTreeMap<(usize, u64), [u64; 2]>` over seeded insert/lookup/remove sequences (64 blocks and 8 banks natively; 4 blocks and 1 bank under Miri); a second insert returns `Exists` and leaves the first value; the final `fill().0` equals the live model entries. The collision test proves two full keys with the same 16-bit fingerprint each resolve to their own value.
- `bank_holds_exactly_255_nodes` inserts 255 keys across 64 blocks of one bank, sees the 256th insert return `BanksFull`, sees `fill() == (255, 255)`, and resolves a stored key. `two_choice_fails_over_to_second_bank` fills the first candidate bank through the private claim path and proves a real insert lands in the second bank (the head link's choice bit is 1).
- The three concurrency pins run with 4 threads natively (2 under Miri): disjoint key sets in blocks that share banks all resolve to `[key, !key]` and `fill().0` equals the successful inserts; a writer inserting into one block's chain never lets a reader see a value other than `[key, !key]`; and no link, claimed index or leftover state is `0xFF`/`WRITING` after the run.
- `cargo xtask test` (30 `test result: ok` lines), `cargo xtask check` and `cargo xtask clippy` exit 0; `cargo xtask miri` exits 0 over 64 seeds with no `Undefined Behavior`, `Data race` or `FAILED` line. The native stress pair was run twice: 2 passed each time.
- **The module holds no `unsafe` site.** Every node field is an atomic, the claim is a compare-exchange and the publication is a `Release` store; nothing is written through a pointer cast from a shared reference and there is no 7-bit pointer packing (CONTEXT D-03; RESEARCH Pitfall 6).

## Task Commits

Each task was committed atomically, RED before GREEN:

1. **Task 1, RED: all nine tests against the compiling skeleton, plus the two RED ledger lines** — `adc6fb5` (test)
2. **Task 1, GREEN: banks, two-choice placement, full-key verification and fill** — `7da88d8` (feat)
3. **Task 1, pin advance (`FILL chained seeds=8 min=0.9920`, GREEN-1/2, `ADVANCED`)** — `7324ac7` (evidence)
4. **Miri-side size scaling to the plan's floors (no native, seed or test-list change)** — `e9b6fc9` (perf)
5. **Task 2, pin advance (GREEN-1/2, `MIRI`, `MIRI-OVER`, `ADVANCED`)** — `0da79e1` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `ds/src/tpht/chained.rs` — the module doc (banks, two-choice, publication, serialisation contract, no listing 9.4); `Node`, `Bank`, `WritingHook`, `ChainedTphtOverflow` (new, candidate_banks, insert, lookup, remove, fill, find, claim, fingerprint_of, the two test helpers); the nine tests and the link assertion helper
- `evidence/PINS-03-07.md` — `chained-overflow` and `chained-publication` RED, GREEN-1, GREEN-2 and ADVANCED (8 `^PIN` lines), `FILL chained seeds=8 min=0.9920` with the per-seed fractions, `MIRI chained seeds=0..64` and `MIRI-OVER chained 230`

## Decisions Made

See `key-decisions` in the frontmatter. The items the plan left to execution:

- **Fingerprint input.** The standalone overflow receives `(block, key)`, not a remainder, so the node's 16-bit filter is `feistel::fingerprint(key)` over the full key. The collision pair is re-derived by an independent search as `(37, 568)`, asserted in the test before the two inserts.
- **Link encoding.** `(candidate choice << 8) | node index`, with `0xFFFF` as the end marker. A node index is `0..=254`; the link scan, the hook and the post-stress scan all prove no index byte is `0xFF`.
- **Fallback test setup.** The first candidate bank is filled through the private claim path (255 `WRITING`, unlinked nodes), which is deterministic and cheap; the insert under test then takes a real node in the second bank.
- **Miri sizing.** The plan's initial Miri-side sizes did not complete (23 seed runs in 1,200 s), so the sizes went to the floors: 4 blocks, 50 operations and 2 threads with 30 operations, with one bank where two are not needed. Native sizes, the 64 seeds and the test list are unchanged.

## Unsafe Sites in `ds/src/tpht/` After This Plan

**None added.** `ds/src/tpht/chained.rs` uses atomics only and holds no `unsafe`. The one site that remains in `ds/src/tpht/` is plan 03-13's unchecked arena read at `ds/src/tpht/flattened.rs:258`, unchanged by this plan.

## Deviations from Plan

### Plan-authorized adjustments

**1. [Plan rule "Miri latency"] The Miri-side sizes were scaled to the floors and the 120 s budget was still missed**
- **Found during:** Task 2, after the first Miri attempts
- **Measurement:** at the plan's Miri-side sizes the run did not complete (23 seed runs in 1,200 s, about 52 s per seed). At the plan's floors (4 blocks, 50 operations, 2 threads with 30 operations, one bank where two are not needed) the first warm run took 230 s and a second warm run 235 s. The 03-13 baseline was 116 s for 13 tests; this suite has 22.
- **Adjustment:** the Miri-side sizes were lowered to the floors; native sizes, the 64 seeds and the test list are unchanged. No third round and no seed/test cut: the plan says those are not the executor's call. The times are recorded as `MIRI-OVER chained 230` and reported to the orchestrator.
- **Files modified:** `ds/src/tpht/chained.rs`
- **Verification:** `cargo xtask miri` exits 0 over 64 seeds with 22 tests per seed and no `Undefined Behavior`, `Data race` or `FAILED` line; the awk `MIRI-TIME` check in Task 2's verify does not pass because no run was inside the budget.
- **Committed in:** `e9b6fc9`

**2. [Sizing, no behaviour change] The 255-node capacity test spreads its inserts over 64 blocks**
- **Found during:** Task 2's Miri measurement
- **Issue:** one chain of 255 nodes made the insert-time `Exists` scan quadratic, which dominated the Miri run (7.6 s per seed).
- **Fix:** the test keeps one bank and 255 inserts but distributes the keys over 64 blocks (`key % 64`), so each chain scan is short; the 256th insert still finds the bank full and `fill()` still reports `(255, 255)`.
- **Verification:** the test passes natively and under Miri; the behaviour asserted is unchanged.
- **Committed in:** `e9b6fc9`

### Test-data correction

**3. [Accuracy] The fingerprint collision pair is `(37, 568)`, not the flattened header's `(473, 557)`**
- **Found during:** the RED run: `(473, 557)` share a fingerprint only after `feistel::permute`, which is the flattened shard's remainder at zero index bits, not this module's full-key input.
- **Fix:** an independent sequential search found `(37, 568)` for `feistel::fingerprint(key)`; the test re-derives and asserts the collision, so a change to `fingerprint` cannot silently remove it.
- **Committed in:** `adc6fb5` (the RED commit, before GREEN)

---

**Total deviations:** 3 (1 plan-authorized Miri resize with a recorded over-budget time, 1 test sizing change with no behaviour change, 1 test-data correction in the RED commit)
**Impact on plan:** none on the DS-04 behaviour, the native sizes, the 64 seeds or the test list. The Miri feedback budget is the one unmet plan rule; it is recorded and reported rather than fixed by cutting seeds or tests.

## Issues Encountered

- **Miri is now over budget.** The 03-13 suite was 116 s for 13 tests; the nine chained tests take the suite to about 230 s at the resize floors, because each test constructs a 255-node bank and the `tpht` Miri step runs all tests over 64 seeds. The plan's stop rule was followed: the floors were reached, no third round was tried, and the times are in the ledger and here.
- **The Miri timing is jittery.** The two warm floor runs measured 230 s and 235 s; the plan-size attempt was stopped after 23 seed runs in 1,200 s.
- **Miri many-seeds output interleaves.** The 64 seed runs print merged `test result:` lines; the run still reports 22 passed per seed and exits 0.

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-26 (a reader returning a partially written node) is mitigated by the two-phase publication and `stress_reader_never_sees_partial_node`; T-03-27 (two inserts claiming one node) by the compare-exchange claim and `stress_concurrent_claims_never_share_a_node`; T-03-28 (undefined behaviour copied from listing 9.4) by the all-atomic node and the absence of any `unsafe` in the module; T-03-29 remains accepted (`BanksFull` is returned, never a panic). T-03-SC does not apply (no package was added).

## Known Stubs

None. The RED skeleton was replaced by the GREEN implementation in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## For owner review

- **The Miri feedback budget.** At the plan's floors the suite takes 230-235 s against 120 s. The functional Miri gate passes (rc 0, 22 tests per seed, no UB or data race). The choice is yours or the orchestrator's: accept the longer Miri run, raise the budget, or authorize cutting seeds/tests (the plan forbids the executor from doing so). Nothing in DS-04's behaviour depends on this.
- **The serialisation contract.** `ChainedTphtOverflow` is safe for concurrent claimers and for readers racing one inserting writer. A reader that needs a consistent view across a concurrent remove must validate against the caller's seqlock; the module doc states this, and plan 03-08 wires the spill under the block's seqlock.
- **The fingerprint input.** The node's 16-bit filter is over the full key because this structure receives `(block, key)`, not a remainder. Plan 03-08 may keep that or pass a remainder-derived fingerprint; either way the full key is verified after a match.

## Next Phase Readiness

- Plan 03-08 can wire the spill path: `insert(block, key, value)` and `lookup(block, key)` take exactly the block and key the shard has, `fill()` reports the bank occupancy, and `BanksFull` is the allocation-failure signal the spill threshold needs.
- Plan 03-10 has no new `unsafe` site to audit from this plan; the only `tpht` site remains `ds/src/tpht/flattened.rs:258` from 03-13.
- The `tpht` Miri suite is 230 s at the floors; a later phase that adds `tpht` tests must expect the latency rule to bind again (and the orchestrator now has the measurement).

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

Both claimed files exist on disk (`ds/src/tpht/chained.rs`, `evidence/PINS-03-07.md`) and all five task commits (`adc6fb5`, `7da88d8`, `7324ac7`, `e9b6fc9`, `0da79e1`) are ancestors of HEAD. Plan-level verification at the final state: `cargo test -p ds --lib tpht::chained` reports 9 passed; the native stress pair passes twice; `cargo xtask test` (rc 0, 30 `test result: ok` lines), `cargo xtask check` (rc 0) and `cargo xtask clippy` (rc 0) all pass; `cargo xtask miri` (rc 0, 64 seeds, 22 passed per seed, no Undefined Behavior or Data race); the ledger holds 8 `^PIN` lines, one `^FILL chained ` line (`seeds=8 min=0.9920`) and `MIRI chained seeds=0..64`; `grep -E '#\[ignore\]|cfg\(not\(miri\)\)' ds/src/tpht/chained.rs` returns nothing; and there is no `as *const`/`as *mut` cast chain and no `unsafe` token in the module. **One task-level verify does not pass:** the awk check for a `MIRI-TIME chained` line inside 120 s, because the floors measured `MIRI-OVER chained 230`; that is the plan-authorized latency deviation recorded above and reported to the orchestrator.
