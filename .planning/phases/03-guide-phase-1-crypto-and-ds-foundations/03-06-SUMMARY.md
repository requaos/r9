---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 06
subsystem: ds
tags: [ds, tpht, feistel, seqlock, atomics, miri, no_std, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "cargo xtask miri and the lint contract (@03-02); the ds/ crate and its gates (@03-12, @03-01)"
provides:
  - "ds::tpht::feistel: mix32 (MurmurHash3's 32-bit finalizer, KAT-pinned), permute, unpermute, split, join and fingerprint; the home index comes from the low bits of the mixed half L' (the RESEARCH C4 correction)"
  - "ds::tpht::TphtError, TinyPtr, index_bits, HOME_SLOTS and the Probe/NoProbe seam with separate header_read and arena_read hooks"
  - "ds::tpht::flattened::TphtBucketHeader: 64 bytes, #[repr(C, align(64))], six atomic fields in the locked 16-slot layout"
  - "ds::tpht::flattened::FlattenedTphtShard<WORDS, P>: new, with_probe, insert, lookup, remove; a 16*num_blocks arena addressed through slot_mask; Boehm-ordered seqlock readers verified by Miri and native stress"
  - "the one test sizing rule for every tpht test, miri_scaled(native, under_miri), which plans 03-13, 03-07 and 03-08 reuse"
affects: [03-07, 03-08, 03-13, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 10768
  tasks: 2
  commits: 7
  plan_head_before: 776e1955f446083ea90f4a95810c49e3ea9069b7
  plan_head_after: 8143473abe854cb56025728964ff96c7579b0dd9

tech-stack:
  added: []
  patterns:
    - "One-round Feistel quotienting with the home block index taken from the low bits of the mixed half L' and every other bit kept in a 64 - index_bits remainder"
    - "Keyed hash table with no stored full key: a 16-bit fingerprint in the header's quotients field plus a caller closure (same) that verifies a candidate against the full key"
    - "Boehm seqlock: writer CASes seq_lock even -> odd, stores data Release, publishes generation + 2 Release; reader loads seq Acquire, data Relaxed, fence(Acquire), reloads seq Relaxed and accepts only an unchanged generation"
    - "Every reader-visible header field is an atomic (the compendium listing 9.3 layout is not copied); the 64-byte size and alignment are const-asserted"
    - "miri_scaled(native, under_miri): one test body, two sizes; the size-dependent bound scales with it; nothing is gated out of Miri"
    - "IG-V01: committed RED pin, GREEN-1, GREEN-2 and ADVANCED, recorded in evidence/PINS-03-06.md"

key-files:
  created:
    - ds/src/tpht/feistel.rs
    - ds/src/tpht/flattened.rs
    - ds/src/tpht/chained.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-06.md
  modified:
    - ds/src/tpht/mod.rs

key-decisions:
  - "The home block index is the low index_bits of L' (the mixed half), not the low bits of P as compendium section 4.1 words it; the remainder is ((L' >> index_bits) << 32) | R, so 64 - index_bits bits survive and nothing is truncated. The test-only split_unmixed keeps the compendium reading visible for plan 03-08's evidence (RESEARCH C4 / Open Question 1)"
  - "mix32 is MurmurHash3's 32-bit finalizer and is pinned by four values computed with independent Python and Node implementations (recorded under KAT mix32 in the ledger); the function under test was never the source (RESEARCH Open Question 2)"
  - "DS-02 is met at the function level, and the header's 16-bit quotients field holds a fingerprint of the remainder, not the remainder itself; a matching candidate is verified by the caller's `same` closure (RESEARCH Open Question 3)"
  - "The seqlock uses the Boehm ordering because the compendium listing's orderings fail Miri (18 of 64 seeds in research) even though 50 native stress runs pass; an Acquire-only CAS for the odd transition is sufficient because every data store is Release, so observing new data implies observing the odd generation before the second seq load"
  - "This writer assigns tiny pointers 1..=16 only; spilled_mask is maintained as zero and clock_nibbles stays zero, so both fields mean what the locked layout says without the Phase 3 writer needing the neighbourhood or eviction paths"
  - "Miri-side sizes were halved once per task under the plan's own latency rule: Task 1 142 s -> 73 s, Task 2 121 s -> 104 s (both resizes and the measurements are in the ledger; native sizes, the 64 seeds and the test list are unchanged)"
  - "requirements-completed records DS-02 only: this plan delivers DS-01's structure and concurrency, while DS-01's over-90% home-hit measurement is plan 03-08's and its frontmatter claims the same requirement"

requirements-completed: [DS-02]

coverage:
  - id: D1
    description: "Feistel quotienting is bijective: join(split(key, bits), bits) returns the key for random, sequential and the six boundary keys at index widths 0, 1, 4, 14 and 32; unpermute(permute(key)) round-trips; both 65,536-key structured families give pairwise distinct pairs at every width"
    requirement: DS-02
    verification:
      - kind: unit
        ref: "ds/src/tpht/feistel.rs#split_join_round_trips, permute_round_trips, split_is_injective_on_structured_keys (`cargo test -p ds --lib tpht::feistel` 5 passed)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri (64 seeds): 10 passed; MIRI-TIME feistel 73"
        status: pass
    human_judgment: false
  - id: D2
    description: "The home index comes from the mixed half: 4,096 keys of the form i << 32 produce at least 2,000 distinct 12-bit home indices, while the compendium's low-bits-of-P reading (split_unmixed) puts all of them on index 0; mix32 matches four independently computed known answers"
    requirement: DS-02
    verification:
      - kind: unit
        ref: "ds/src/tpht/feistel.rs#index_comes_from_the_mixed_half, mix32_known_answers"
        status: pass
      - kind: other
        ref: "evidence/PINS-03-06.md: six `^KAT mix32 ` lines (Python and Node cross-check)"
        status: pass
    human_judgment: false
  - id: D3
    description: "TphtBucketHeader is exactly 64 bytes and 64-byte aligned with the six atomic fields in the locked order (seq_lock, occupied_mask, spilled_mask, quotients[16], tiny_ptrs[16], clock_nibbles), asserted both by a const assertion and by a test"
    requirement: DS-01
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#header_is_64_bytes_and_64_aligned plus the `const _` size/align assertion"
        status: pass
    human_judgment: false
  - id: D4
    description: "The home path matches a BTreeMap model over seeded insert/lookup/remove sequences on shards of 1, 16 and 1,024 blocks; a second insert reports Exists and leaves the first value; the seventeenth key for one block reports HomeFull; two keys sharing a 16-bit fingerprint are both stored and each lookup returns its own value"
    requirement: DS-01
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#home_ops_match_btreemap_model, fingerprint_collisions_are_verified, rejects_bad_block_counts (`cargo test -p ds --lib tpht::flattened` 5 passed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Concurrent readers never see a torn entry: writer threads insert and remove keys of one block with self-checking [key, !key] values while reader threads look them up and always find the sentinel; Miri passes all 10 tpht tests over 64 seeds and the native stress run uses 4 writers and 4 readers"
    requirement: DS-01
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#stress_readers_never_see_torn_entry"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask miri rc 0: no Undefined Behavior, Data race or 'no tests under ds/src/tpht'; MIRI-TIME tpht-shard 104"
        status: pass
    human_judgment: false
  - id: D6
    description: "The shard exposes what plan 03-13 needs for deref_tiny: the Probe trait with separate header_read and arena_read hooks (called once per reader attempt and once per arena slot read), TinyPtr, and an arena of exactly 16 * num_blocks slots addressed through slot_mask = 16 * num_blocks - 1"
    requirement: DS-01
    verification:
      - kind: other
        ref: "ds/src/tpht/mod.rs#Probe, NoProbe, TinyPtr; ds/src/tpht/flattened.rs#with_probe, slot_index; `grep -c 'probe.header_read()\\|probe.arena_read()'` shows both hooks on the reader path"
        status: pass
    human_judgment: false
  - id: D7
    description: "A warm cargo xtask miri run stays inside the 120-second feedback budget of 03-VALIDATION.md, with the measured times in the ledger"
    requirement: DS-01
    verification:
      - kind: other
        ref: "evidence/PINS-03-06.md: MIRI-TIME feistel 73 and MIRI-TIME tpht-shard 104; the awk budget check over the ledger passes"
        status: pass
    human_judgment: false

# Metrics
duration: 25min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 06: TPHT — Feistel quotienting and the 64-byte seqlock header and shard

**One-round Feistel quotienting whose home index comes from the mixed half, `mix32` pinned by independent known answers, and a 64-byte all-atomic bucket header whose Boehm-ordered seqlock readers pass Miri over 64 seeds and a native multi-threaded stress run.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-11-24T14:46:37Z
- **Completed:** 2026-11-24T15:11:41Z
- **Tasks:** 2
- **Files modified:** 5 (4 source files, 1 ledger)

## Accomplishments

- `ds::tpht::feistel` is a safe (`#![forbid(unsafe_code)]`) module with `mix32`, `permute`, `unpermute`, `split`, `join` and `fingerprint`. The home block index is the low `index_bits` of `L'` (the mixed half); the remainder is `((L' >> index_bits) << 32) | R`, so `64 - index_bits` bits survive and nothing is truncated. `join` reconstructs the key exactly for random, sequential and boundary keys at index widths 0, 1, 4, 14 and 32, and both 65,536-key structured families give pairwise-distinct pairs at every width.
- `mix32` is MurmurHash3's 32-bit finalizer: xor-shift 16, multiply `0x85EB_CA6B`, xor-shift 13, multiply `0xC2B2_AE35`, xor-shift 16, with `wrapping_mul`. Its four known answers were computed with independent Python and Node implementations and recorded in the ledger (`KAT mix32`), never taken from the function under test.
- `index_comes_from_the_mixed_half` shows the correction's effect: 4,096 keys of the form `i << 32` spread over at least 2,000 distinct 12-bit home indices, while the test-only `split_unmixed` (the compendium's low-bits-of-`P` reading) puts all 4,096 on index 0. This is the RESEARCH C4 correction, and the choice is outside the IG-D09 lock; plan 03-08 shows both variants in the spill evidence.
- `TphtBucketHeader` is `#[repr(C, align(64))]` and exactly 64 bytes, with the locked order `seq_lock: AtomicU32`, `occupied_mask: AtomicU16`, `spilled_mask: AtomicU16`, `quotients: [AtomicU16; 16]`, `tiny_ptrs: [AtomicU8; 16]`, `clock_nibbles: AtomicU64` (4 + 2 + 2 + 32 + 16 + 8). The size and alignment are asserted by a `const _` block and by a test; no field a reader touches is a plain integer.
- `FlattenedTphtShard<const WORDS, P = NoProbe>` owns one header per block and an arena of exactly `16 * num_blocks` slots of `[AtomicU64; WORDS]`, addressed by `((block << 4) + (ptr - 1)) & slot_mask` with `slot_mask = 16 * num_blocks - 1`. It exposes `new`, `with_probe`, `insert`, `lookup` and `remove`; `same` is the caller's full-key verification of a fingerprint match.
- The writer brings `seq_lock` from an even generation `s` to `s + 1` by compare-exchange, stores the value words, fingerprint, tiny pointer and mask with `Release`, and finishes with a `Release` store of `s + 2`. The reader loads `seq_lock` with `Acquire`, loads data `Relaxed`, issues `fence(Acquire)`, reloads `seq_lock` with `Relaxed`, and accepts only an unchanged generation. Every reader-visible field is an atomic; the listing 9.3 orderings and its plain `u16` fields are not used.
- `home_ops_match_btreemap_model` agrees with a `BTreeMap<u64, [u64; 2]>` over seeded insert/lookup/remove sequences on shards of 1, 16 and 1,024 blocks. A second insert returns `Err(Exists)` and leaves the first value; the seventeenth key for one block returns `Err(HomeFull)`; two keys with the same 16-bit fingerprint and different full keys are both stored and each lookup returns its own value.
- `stress_readers_never_see_torn_entry` runs writer threads (4 natively, 2 under Miri) inserting and removing keys of one block with `[key, !key]` values while reader threads look them up; every value a reader gets satisfies `v[0] == key && v[1] == !key`, and a sentinel key is always present. `cargo xtask miri` passes all 10 `tpht` tests over 64 seeds with no Undefined Behavior or Data race line.
- The one sizing rule `miri_scaled(native, under_miri)` lives in `ds/src/tpht/mod.rs` behind `#[cfg(test)]`; every sized `tpht` test uses it, no test is `#[ignore]`d or gated out, and the Miri-side numeric bound scales with the size the test ran at.
- `cargo xtask test` (30 `test result: ok` lines, 23 `ds` lib tests), `cargo xtask check` and `cargo xtask clippy` all exit 0 with no `ds/` warning.

## Task Commits

Each task was committed atomically, RED before GREEN:

1. **Task 1, RED: Feistel quotienting pin, module scaffolding, `TphtError`/`TinyPtr`/`index_bits`/`miri_scaled`** — `59ec703` (test)
2. **Task 1, GREEN: Feistel quotienting with the home index from the mixed half** — `ca2ae40` (feat)
3. **Task 1, pin advance (`KAT mix32`, GREEN-1/2, `MIRI`/`MIRI-TIME`)** — `8ee0757` (evidence)
4. **Task 2, RED: header, shard API and the five tests** — `c8e092c` (test)
5. **rustfmt reflow of the Feistel module** — `5e53f47` (style)
6. **Task 2, GREEN: `TphtBucketHeader` and `FlattenedTphtShard` with Boehm-ordered seqlock readers** — `b05fea6` (feat)
7. **Task 2, pin advance (GREEN-1/2, `MIRI`/`MIRI-OVER`/`MIRI-TIME`)** — `8143473` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `ds/src/tpht/feistel.rs` — `#![forbid(unsafe_code)]`; `mix32`, `permute`, `unpermute`, `split`, `join`, `fingerprint`, the test-only `split_unmixed`, and the five DS-02 tests (KAT, round-trip, injectivity, mixed-half index, boundary widths)
- `ds/src/tpht/flattened.rs` — `TphtBucketHeader` (six atomic fields, const size/align assertions), `FlattenedTphtShard` (`new`, `with_probe`, `insert`, `lookup`, `remove`, `slot_index`, the value/lock helpers), and the five tests (layout, bad block counts, BTreeMap model + HomeFull, fingerprint collisions, torn-read stress)
- `ds/src/tpht/mod.rs` — module doc (D-09 allowance, D-03 non-normative listings, the Boehm ordering), `pub mod` declarations, `TphtError`, `TinyPtr`, `index_bits`, `HOME_SLOTS`, `Probe`/`NoProbe`, `miri_scaled`
- `ds/src/tpht/chained.rs` — module doc only; plan 03-07 fills the two-choice dereference banks
- `evidence/PINS-03-06.md` — six `KAT mix32` lines and RED, GREEN-1, GREEN-2 and ADVANCED for `feistel` and `tpht-shard` (8 `^PIN` lines), plus `MIRI`, `MIRI-OVER` and `MIRI-TIME` records

## Decisions Made

See `key-decisions` in the frontmatter. The two items the plan left to Claude's discretion:

- **`f(R)` completion.** MurmurHash3's 32-bit finalizer, pinned by four independently computed values (Python and Node agreed). Any bijective `f` would satisfy DS-02; the finalizer is the standard completion of the two constants both source documents give before they stop.
- **Index bits.** The home index comes from the mixed half `L'`, not from `P`'s low bits. The permutation stays bijective either way; the correction is what makes structured keys (`i << 32`) spread across blocks, and `split_unmixed` keeps the compendium reading reproducible for plan 03-08's evidence.

## Unsafe Sites in `ds/src/tpht/` After This Plan

**None.** `feistel.rs` carries `#![forbid(unsafe_code)]`; `flattened.rs` implements the writer and the Boehm reader entirely with atomics and needs no `unsafe`; `mod.rs` and `chained.rs` hold none. Plan 03-13's `deref_tiny` unchecked read is the first expected site and is outside this plan.

## Not Built Here (named so it is not lost)

- The writer assigns tiny pointers `1..=16` only — the block's own window. Compendium section 4.3 item 3 (a writer that scans the neighbourhood `17..=255`) and item 4 (clock-nibble eviction) are not Phase 3 requirements. `deref_tiny` in plan 03-13 resolves the full `1..=255` range, and the `spilled_mask` and `clock_nibbles` fields exist and are maintained per the locked layout (`spilled_mask` stays zero with this writer; `clock_nibbles` is initialised to zero).
- The over-90% home-hit measurement and the spill threshold (D-15) are plan 03-08's; this plan delivers the structure and concurrency DS-01 needs first.

## Deviations from Plan

None beyond the plan's own Miri latency rule and one formatting commit.

### Plan-authorized adjustments

**1. [Plan rule "Miri latency"] Task 1 halved the Feistel Miri-side sizes**
- **Found during:** Task 1, after GREEN
- **Measurement:** the first warm `cargo xtask miri` took 142 s, over the 120 s budget; recorded as `MIRI-OVER feistel 142`.
- **Adjustment:** resize round 1 halved every Miri-side Feistel size (key families 256 -> 128, bound 125 -> 62); native sizes, the 64 seeds and the test list unchanged. The next warm run took 73 s and was recorded as `MIRI-TIME feistel 73`.

**2. [Plan rule "Miri latency"] Task 2 halved the shard's Miri-side sizes**
- **Found during:** Task 2, after GREEN
- **Measurement:** a warm run took 121 s (117 s and 118 s on the same tree earlier), over the 120 s budget; recorded as `MIRI-OVER tpht-shard 121`.
- **Adjustment:** resize round 1 halved this task's Miri-side sizes (model blocks 16 -> 8, model operations 200 -> 100, stress operations per thread 100 -> 50). The stress test keeps 2 writers and 2 readers — the concurrency it exists for is never scaled away. The next warm run took 104 s and was recorded as `MIRI-TIME tpht-shard 104`; no second round was needed.

**3. [Formatting] `style(03-06)` rustfmt reflow of `feistel.rs`**
- **Found during:** Task 2, before the GREEN commit: `rustfmt --check` reported reflows in the Task 1 file.
- **Fix:** `rustfmt` applied to the four `tpht` files and the whitespace-only change committed separately as `5e53f47`, keeping the Task 2 feat commit focused.

---

**Total deviations:** 3 (2 plan-authorized Miri resizes, 1 formatting-only commit)
**Impact on plan:** none on the test list, the seed count, the native sizes or the assertions; the resizes only reduce how much work Miri repeats per seed, exactly as the plan's latency rule directs.

## Issues Encountered

- **Miri timing is jittery on this host.** The same tree measured 117 s, 118 s and 121 s on successive warm runs, and a run that includes the Miri rebuild measured 123-124 s. The plan's resize rule resolved it: after halving the Task 2 Miri-side sizes the warm run settled at 104 s, and both `MIRI-TIME` records are inside the 120 s budget.
- **A dead assignment in the reader.** The first GREEN `lookup` kept a `start` index across attempts; rustc warned it was never read because the `for` range is fixed at loop entry. Removing it simplified the reader to a full 0..16 scan per attempt (still correct for fingerprint collisions, since `same` false continues the scan) and the warning is gone.
- **RED skeleton warnings.** The RED commits compile with transient warnings (`Ordering` unused, shard fields unread); the GREEN commits use them and the final tree is warning-free for `ds/`.

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-22 is mitigated by all-atomic reader fields, the Boehm ordering, Miri over 64 seeds and the native self-checking stress test; T-03-24 by the mixed-half index and `index_comes_from_the_mixed_half` (the distribution evidence is plan 03-08's); T-03-25 remains accepted (no kernel links `ds/` and no peer-chosen key reaches the table); T-03-SC does not apply (no package was added).

## Known Stubs

None. Every RED skeleton was replaced by the GREEN implementation in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## For owner review

- **DS-01's edge probe row `unclassified`.** The assumption taken here: the edges that matter are a full header (16 entries, `Err(HomeFull)`), an empty shard, a one-block shard, and a reader racing a writer on one block. Each has a test. Please confirm no other edge was meant.
- **The mixed-half index correction** (RESEARCH C4) is recorded in the `feistel.rs` module doc and in the ledger; plan 03-08 produces both-variant evidence and plan 03-09 records it beside the D-08 and D-17 rows. It is outside the IG-D09 lock and is surfaced for your review at phase verification.
- **`requirements-completed` lists DS-02 only.** DS-01 is part-delivered here (structure and concurrency); its over-90% home-hit measurement is plan 03-08's, whose frontmatter also claims DS-01, so its REQUIREMENTS.md checkbox stays open.

## Next Phase Readiness

- Plan 03-07 (`ChainedTphtOverflow`) can build on `TphtBucketHeader`, the `Probe` seam and the shared `TphtError`/`TinyPtr` types; `chained.rs` is a doc-only module today.
- Plan 03-08 has the home path it needs to wire the spill threshold and measure home hits; the shard's insert path is the place the spill decision attaches.
- Plan 03-13 has what `deref_tiny` needs: `TinyPtr`, the counting `Probe`, and an arena of exactly `16 * num_blocks` slots addressed through `slot_mask`.
- The `tpht` test suite is inside the Miri latency budget (73 s and 104 s recorded) and every test sizes itself through `miri_scaled`, so plans 03-07, 03-08 and 03-13 inherit the same rule.

---
*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All six claimed files exist on disk and all seven task commits are ancestors of HEAD. Plan-level verification at the final state: `cargo test -p ds --lib tpht::` reports 10 passed (5 feistel, 5 flattened); `cargo xtask test` (rc 0, 30 `test result: ok` lines, 23 `ds` lib tests), `cargo xtask check` (rc 0) and `cargo xtask clippy` (rc 0) all pass; `cargo xtask miri` (rc 0, 64 seeds, no Undefined Behavior or Data race); the ledger holds 8 `^PIN` lines, 6 `^KAT mix32 ` lines and `MIRI-TIME feistel 73` / `MIRI-TIME tpht-shard 104` (the awk budget check passes); `grep -cE 'fence\((Ordering::)?Acquire\)' ds/src/tpht/flattened.rs` prints 2; `grep -l miri_scaled ds/src/tpht/feistel.rs ds/src/tpht/flattened.rs | grep -c .` prints 2; and `head -1 ds/src/tpht/feistel.rs` is `#![forbid(unsafe_code)]`.
