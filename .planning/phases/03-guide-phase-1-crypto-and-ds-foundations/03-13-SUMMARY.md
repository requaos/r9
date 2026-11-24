---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 13
subsystem: ds
tags: [ds, tpht, deref-tiny, unchecked-read, probe, atomics, miri, no_std, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-06: FlattenedTphtShard with slot_mask and the 16*num_blocks arena, TinyPtr, the Probe/NoProbe seam, feistel::split and miri_scaled"
provides:
  - "ds::tpht::flattened::FlattenedTphtShard::deref_tiny(key, ptr) -> [u64; WORDS]: one arena slot read, zero header reads and no seqlock"
  - "the only unsafe site in ds/src/tpht/ after this plan (the unchecked arena read, line 258) with its SAFETY comment, ready for plan 03-10's UNSAFE-AUDIT.md"
  - "a CountingProbe test seam and the three DS-03 tests: counting probe (1, 0), BTreeMap model, and the slot-mask wrap for pointers 1, 16, 17 and 255"
  - "evidence/PINS-03-13.md: the deref-tiny pin (RED, GREEN-1, GREEN-2, ADVANCED) plus MIRI, MIRI-OVER and MIRI-TIME"
affects: [03-08, 03-10, phase-4]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 4171
  tasks: 1
  commits: 4
  plan_head_before: e478dc656d85bc4c9e3fe11fe3e28c0a7654db0f
  plan_head_after: 3c88fd242e131228044817da6810472bcf668a75

tech-stack:
  added: []
  patterns:
    - "deref_tiny: the home block index from feistel::split in register arithmetic, the slot from ((block << 4) + (ptr - 1)) & slot_mask, then one masked get_unchecked arena read of WORDS relaxed atomics; no header field is loaded and the seqlock is never taken"
    - "CountingProbe: an Arc-shared two-counter Probe in the test module, so one test asserts the exact pair (arena_reads, header_reads) == (1, 0) and then shows the probe is wired by asserting lookup's header reads"
    - "The stale-pair contract: a (key, tiny_ptr) pair kept past its entry's removal reads the slot's current contents and is never undefined behaviour, because every arena word is an atomic"

key-files:
  created:
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-13.md
  modified:
    - ds/src/tpht/flattened.rs

key-decisions:
  - "\"One memory access\" is defined as one arena slot (a value may be several atomic words inside it), the RESEARCH Pattern 4 definition and the plan's flagged assumption; the counting probe therefore counts one arena_read per deref_tiny and zero header_reads"
  - "The unchecked read is get_unchecked on the arena, masked by slot_mask; its SAFETY comment names the constructor invariant (the arena holds exactly slot_mask + 1 = HOME_SLOTS * num_blocks slots, both fixed in with_probe). This is the D-13 site and the only unsafe site in ds/src/tpht/"
  - "The stale-pair behaviour is asserted rather than assumed: after a removal the next insert reuses slot 0 and deref_tiny with the old pair returns the new value, never undefined behaviour"
  - "The wrap test needs no scaling: two blocks is the smallest shard where b << 4 is non-zero and both wrap pointers land in another block's window, so miri_scaled(2, 2) states that the native and Miri sizes agree; the counting test likewise uses miri_scaled(1, 1)"
  - "Miri-side model sizes were halved once under the plan's 120-second latency rule (blocks 16 -> 8, operations 200 -> 100): the first warm run took 123 s, the resized run 116 s. Native sizes (1,024 blocks, 2,000 operations), the 64 seeds and the test list are unchanged"

requirements-completed: [DS-03]

coverage:
  - id: D1
    description: "`deref_tiny` performs exactly one arena slot read and zero header reads, shown by a counting probe: (arena_reads, header_reads) == (1, 0) after one call, while one `lookup` reads the header at least once, proving the probe is wired"
    requirement: DS-03
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#deref_tiny_is_one_arena_read_and_no_header_read (`cargo test -p ds --lib tpht::flattened::tests::deref_tiny` 3 passed)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri (64 seeds): 13 passed; no Undefined Behavior or Data race; MIRI-TIME deref-tiny 116"
        status: pass
    human_judgment: false
  - id: D2
    description: "`deref_tiny` agrees with a BTreeMap model: over a seeded insert/remove/lookup sequence (1,024 blocks and 2,000 operations natively; 8 blocks and 100 operations under Miri; 32 keys), every live key resolved through the pointer `insert` returned equals the model's value, and a stale pair reads the slot's current value"
    requirement: DS-03
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#deref_tiny_matches_btreemap_model"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri (64 seeds): 13 passed, no Undefined Behavior"
        status: pass
    human_judgment: false
  - id: D3
    description: "The slot formula is pinned by the mask: pointers 1, 16, 17 and 255 on the last block (b = 1, slot_mask = 31) read slot ((b << 4) + (ptr - 1)) & slot_mask, so pointers that run past the arena end wrap into the arena and never index out of bounds"
    requirement: DS-03
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#deref_tiny_wraps_with_slot_mask"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri (64 seeds): 13 passed, no Undefined Behavior"
        status: pass
    human_judgment: false

# Metrics
duration: 13min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 13: TPHT — `deref_tiny` in exactly one memory access

**`deref_tiny(key, tiny_ptr)` resolves a value from one masked arena slot read with zero header reads and no seqlock, pinned by a counting probe that asserts exactly `(1, 0)`, a `BTreeMap` model over seeded operations, and the `slot_mask` wrap for pointers 1, 16, 17 and 255.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-24T15:12:53Z
- **Completed:** 2026-11-24T15:25:28Z
- **Tasks:** 1 (TDD: one RED commit, one GREEN commit, one evidence commit, one ledger correction)
- **Files modified:** 2 (one source file, one ledger)

## Accomplishments

- `FlattenedTphtShard::deref_tiny(key, ptr)` takes the home block index from `feistel::split(key, self.bits)` in register arithmetic, computes the slot as `((block << 4) + (ptr - 1)) & slot_mask`, calls `probe.arena_read()` once and loads the slot's `WORDS` words with `Relaxed`. It loads no `TphtBucketHeader` field, never calls `probe.header_read()` and never takes the seqlock.
- The slot read is the module's one `unsafe` site — `self.arena.get_unchecked(slot)` — and its `// SAFETY:` comment names the invariant: `slot_index` masks by `self.slot_mask`, and `with_probe` allocates exactly `slot_mask + 1` (`HOME_SLOTS * num_blocks`) slots, both fixed in the constructor and never mutated. This is the unchecked read D-13 names, in the module D-09 allows.
- The doc comment defines the contract: "one memory access" is one arena slot (possibly several words); the caller holds a `(key, tiny_ptr)` pair for a live entry (for example a `StatefulHandle` in Phase 4) and must not use it after removal; a stale pair returns whatever the slot holds and is never undefined behaviour because every arena word is an atomic.
- `deref_tiny_is_one_arena_read_and_no_header_read` uses a `CountingProbe` (two `Arc<AtomicUsize>` counters, cloned into the shard) and asserts the exact pair `(arena_reads, header_reads) == (1, 0)` after one `deref_tiny`; one `lookup` afterwards shows `header_reads >= 1`, so the probe is wired to the reader.
- `deref_tiny_matches_btreemap_model` runs a seeded (SplitMix64) sequence of inserts, removes and lookups over 32 keys — 1,024 blocks and 2,000 operations natively, 8 blocks and 100 operations under Miri — and checks every live key against a `BTreeMap` model through the pointer `insert` returned. A final deterministic scenario removes key 7, inserts key 8 into the freed slot and asserts the stale `(7, stale)` pair returns `[8, !8]`.
- `deref_tiny_wraps_with_slot_mask` fills both blocks of a two-block shard (16 keys each) so the last block's window starts at slot 16 and `slot_mask` is 31: pointers 1 and 16 read slots 16 and 31, while 17 wraps to slot 0 and 255 to slot 14 — each checked against the value stored in that slot, so a pointer past the arena end wraps and never indexes out of bounds.
- All three tests take their sizes through `miri_scaled`; none is `#[ignore]`d or gated out of Miri. `cargo xtask miri` passes all 13 `tpht` tests over 64 seeds with no Undefined Behavior or Data race line, in 116 s (budget 120 s).
- `cargo xtask test` (30 `test result: ok` lines, 26 `ds` lib tests), `cargo xtask check` and `cargo xtask clippy` all exit 0 with no `ds/` warning and no `unsafe block missing a safety comment`.

## Task Commits

Each task was committed atomically, RED before GREEN:

1. **Task 1, RED: the three `deref_tiny` tests and the returning-zeros skeleton** — `8e6460d` (test)
2. **Task 1, GREEN: `deref_tiny`, one arena read and no header probe** — `45469da` (feat)
3. **Task 1, pin advance (`PIN deref-tiny ADVANCED 45469da`)** — `a53df77` (evidence)
4. **Ledger correction: the unsafe site's line numbers after the module-doc edit** — `3c88fd2` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `ds/src/tpht/flattened.rs` — the module doc gained the `deref_tiny` contract and the note that its unchecked arena read is the module's one unsafe site; `pub fn deref_tiny` (split, masked slot, one `probe.arena_read`, one `get_unchecked` under a `SAFETY` comment, `WORDS` relaxed loads); the test module gained `CountingProbe` and the three DS-03 tests
- `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-13.md` — pin id `deref-tiny`: RED (3 failed on assertions, the five plan 03-06 tests still pass), GREEN-1, GREEN-2, `MIRI` (64 seeds, 13 passed), `MIRI-OVER deref-tiny 123`, `MIRI-TIME deref-tiny 116`, `ADVANCED 45469da`

## Decisions Made

See `key-decisions` in the frontmatter. The two the plan left to execution:

- **"One memory access" is one arena slot.** A value may be several atomic words inside the slot; the probe counts one `arena_read` per call. This is RESEARCH Pattern 4's definition and the plan's flagged assumption, stated below for the owner.
- **The wrap test is not scaled.** Two blocks is the smallest shard that exercises `b << 4` (non-zero) and both wrap pointers, so `miri_scaled(2, 2)` and `miri_scaled(1, 1)` (counting test) route through the one sizing rule while stating that the native and Miri sizes agree; only the model test's blocks and operations scale.

## Unsafe Sites in `ds/src/tpht/` After This Plan

One site, added by this plan, for plan 03-10's `UNSAFE-AUDIT.md`:

| File | Line | Kind | Invariant |
|---|---|---|---|
| `ds/src/tpht/flattened.rs` | 258 (`get_unchecked`, `// SAFETY:` comment at 252) | unchecked slice index on the arena (`&self.arena[slot]` without the bound check) | `slot = ((block << 4) + (ptr - 1)) & slot_mask`, and `with_probe` allocates exactly `slot_mask + 1` slots (`HOME_SLOTS * num_blocks`); both are fixed in the constructor and never mutated, so `slot < self.arena.len()` for every block index and every tiny pointer, including one past the arena end |

`feistel.rs` still carries `#![forbid(unsafe_code)]`; `mod.rs` and `chained.rs` hold none. This is the first site in `ds/src/tpht/` and the only one after this plan (grep over `ds/src/tpht/` for the `unsafe` token returns only the doc mentions plus line 258).

## Deviations from Plan

None beyond the plan's own Miri latency rule and one evidence correction.

### Plan-authorized adjustments

**1. [Plan rule "Miri latency"] The model test's Miri-side sizes were halved once**
- **Found during:** Task 1, after the GREEN run
- **Measurement:** the first warm `cargo xtask miri` took 123 s, over the 120 s budget; recorded as `MIRI-OVER deref-tiny 123`.
- **Adjustment:** resize round 1 halved the Miri-side model sizes (blocks 16 -> 8, operations 200 -> 100). Native sizes, the 64 seeds, the test list and the counting/wrap tests' sizes are unchanged. The next warm run took 116 s and was recorded as `MIRI-TIME deref-tiny 116`; no second round was needed.

### Evidence correction

**2. [Accuracy] `evidence(03-13)` corrected the unsafe site's line numbers in the ledger**
- **Found during:** the SUMMARY's self-check, after the module-doc note shifted the file by five lines
- **Fix:** the ledger now cites `ds/src/tpht/flattened.rs:258` (and the `SAFETY` comment at 252) instead of 253/247; a one-line evidence commit, no source change.

---

**Total deviations:** 2 (1 plan-authorized Miri resize, 1 ledger line-number correction)
**Impact on plan:** none on the test list, the seed count, the native sizes or the assertions; the resize only reduces how much work Miri repeats per seed, exactly as the plan's latency rule directs.

## Issues Encountered

- **The Miri budget is tight.** The 03-06 baseline was 104 s for ten `tpht` tests; the three `deref_tiny` tests first measured 123 s and, after the resize, 116 s. The counting and wrap tests cost little (the wrap test runs 0.55 s per seed under Miri); the model test's loop was the cost, so it is the one that was halved. A later phase that adds `tpht` tests will need to watch this budget again.
- **Miri's many-seeds runs the whole filtered suite once per seed.** "Test result: ok. 13 passed" therefore appears once per seed (64 lines), which is why the ledger records the result line plus the seed count rather than a single summary line.
- **The RED skeleton needed no warnings suppressed.** `let _ = (key, ptr);` kept the skeleton silent; the only transient warnings in the tree are the pre-existing `xtask` dead-code ones.

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-23 is mitigated by the `slot_mask` on the index, the arena sized `slot_mask + 1`, the lint-enforced `SAFETY` comment naming that invariant, the Miri run over 64 seeds, and all-atomic arena words so a stale pointer is not UB. T-03-SC does not apply (no package was added).

## Known Stubs

None. The RED skeleton was replaced by the GREEN implementation in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## For owner review

- **DS-03's edge probe row `unclassified`.** The assumption taken here: "one memory access" means one arena slot (a value may be several words inside that slot), and the pointer range tested is 1..=255 with the wrap by `slot_mask`. Please confirm at verification.
- **The unsafe site's disclosure.** `deref_tiny` is the first `unsafe` site in `ds/src/tpht/`; it is surfaced here (file, line, kind, invariant) for plan 03-10's audit, which the must-have requires. D-12 says there is no owner diff stop for TPHT unsafe.

## Next Phase Readiness

- Plan 03-08 can re-pin the three tests here on a home-only shard by name when it changes `insert` to report home-versus-bank placement, leaving the assertions as written.
- Plan 03-10 has the one `unsafe` site to list in `UNSAFE-AUDIT.md`: `ds/src/tpht/flattened.rs:258`, unchecked arena index, invariant recorded above.
- Phase 4's `StatefulHandle` has the fast path it needs: `deref_tiny(key, tiny_ptr)` is one arena slot read with no header probe, and its stale-pair contract is documented and tested.
- The `tpht` Miri suite is at 116 s of the 120 s budget; the sizing rule and the resize procedure are in `evidence/PINS-03-13.md`.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

Both claimed files exist on disk (`ds/src/tpht/flattened.rs`, `evidence/PINS-03-13.md`) and all four task commits (`8e6460d`, `45469da`, `a53df77`, `3c88fd2`) are ancestors of HEAD. Plan-level verification at the final state: `cargo test -p ds --lib tpht::flattened::tests::deref_tiny` reports 3 passed; `cargo xtask miri` (rc 0, 64 seeds, 13 passed per seed, 114.95 s, no Undefined Behavior or Data race); `cargo xtask clippy` (rc 0, no `unsafe block missing a safety comment`); `cargo xtask test` (rc 0, 30 `test result: ok` lines, 26 `ds` lib tests) and `cargo xtask check` (rc 0); the ledger prints 4 `^PIN deref-tiny` lines and the `MIRI-TIME` awk budget check passes; `grep -rn unsafe ds/src/tpht/` shows only the doc mentions and the single `get_unchecked` at line 258; and no test carries `#[ignore]` or `cfg(not(miri))`.
