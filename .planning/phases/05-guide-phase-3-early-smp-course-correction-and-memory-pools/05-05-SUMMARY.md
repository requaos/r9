---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 05
subsystem: ds
tags: [ds, slab, seqlock, atomics, miri, roaring, tpht, no_std, forbid-unsafe]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations (v3.0, archived)
    provides: Boehm seqlock ordering proven on TphtBucketHeader, RoaringBitmap set algebra, miri_scaled sizing rule, SplitMix64 test pattern, the cargo xtask miri step
  - phase: 05-01
    provides: green CI harness and cargo xtask test as the trustworthy host suite
provides:
  - ds/src/slab.rs: generic #![forbid(unsafe_code)] lock-free Boehm seqlock slab (AtomicSlabSlot<7> exactly 64 B / align 64, SlabSlots, AllocMeta, SlabProbe/NoSlabProbe, next_generation)
  - ds/src/lib.rs: #[forbid(unsafe_code)] pub mod slab; registered per the D-09 module layout
  - ds/src/tpht/flattened.rs: measured DEBT-04 production bank-sizing rule on default_banks, pub fn banks_for_occupancy, and the non-ignored default_banks_production_sizing pin
  - xtask/src/miri.rs: Miri coverage extended from tpht-only to MIRI_MODULES = [tpht, slab]
affects: [05-06 (port/src/fid.rs wraps ds::slab), 05-16 palloc.hash bank sizing, 05-21 MEM-16 page cache live verification, phase-6 union dcache]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 15446
  tasks: 3
  commits: 5
  plan_head_before: bf9e7d0eed4c0dc657cec06ba5bf2fad2bf9fd20
  plan_head_after: eada265502ba19598b0051adaedd866efca22746

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Slab slot: 64-byte #[repr(C, align(64))] AtomicSlabSlot with packed state word (WRITING|OCCUPIED|gen:16|seq:46) and Boehm fence-paired publication, all safe Rust"
    - "AllocMeta holds free_list + active/reclaimed RoaringBitmaps; caller serializes writers (ds/ has no mutex — port/ wraps in Lock); resolve never touches it"
    - "Generation never zero: starts at 1, wraps 65535 -> 1 via next_generation; resolve rejects expected_gen == 0 (D-10)"
    - "cargo xtask miri covers a named MIRI_MODULES list with per-leg empty-filter refusal"

key-files:
  created:
    - ds/src/slab.rs
  modified:
    - ds/src/lib.rs
    - ds/src/tpht/flattened.rs
    - xtask/src/miri.rs

key-decisions:
  - "banks_for_occupancy rule: twice the measured worst-case spill (10% of entries at the 90% target) over 255-node banks — entries / (BANK_NODES * 5), minimum 2; measured BanksFull-free with >90% home-hit on all 5 distributions at 1,024/4,096/16,384 blocks"
  - "AllocMeta::sweep_unreclaimed returns the swept slot indices (Vec<u16>) and resets the reclaim window — WIRE-09's active \\ reclaimed difference restated for the slab"
  - "Miri covers the slab by extending xtask/src/miri.rs to MIRI_MODULES = [tpht, slab] rather than naming slab tests to match the tpht filter (a filter substring common to both module paths does not exist)"

patterns-established:
  - "SlabProbe/CountingSlabProbe: the WIRE-08 one-cache-line-read pin restated for the slab (1 slot_read per resolve attempt, 0 metadata touches)"
  - "DEBT-04 measurement shape: ignored on-demand generator (measure_default_banks_table) + non-ignored production pin (default_banks_production_sizing) re-running the chosen cell — mirrors D-15"

requirements-completed: [CORR-03, DEBT-04]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "ds/src/slab.rs compiles under #![forbid(unsafe_code)] with AtomicSlabSlot<7> exactly 64 bytes / 64-byte aligned, pinned by compile-time assertions and a host test (CORR-03, D-09, D-11)"
    requirement: CORR-03
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p ds slab (9 passed; slot_is_64_bytes_and_64_aligned)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Generation adjacency/empty/ordering/idempotency: gen starts at 1, wraps 65535 -> 1 skipping 0, resolve(idx, 0) is always None; clunk scrubs all 7 payload words; Boehm fence-paired writer/reader ordering; in-place updates bump seq not gen; duplicate clunk rejected cleanly (D-10, D-11)"
    requirement: CORR-03
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p ds slab (fresh_slot_alloc_publishes_occupied_and_resolves, update_keeps_generation_and_replaces_payload, clunk_scrubs_payload_bumps_generation_and_recycles, generation_wraps_65535_to_1_never_zero, mark_reclaimed_and_sweep_unreclaimed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Concurrency: CountingSlabProbe proves 1 cache-line read on uncontended resolve; multi-threaded stress (native 20k ops x 4 writers/4 readers + Miri-sized twin) asserts zero torn snapshots; cargo xtask miri proves zero UB/data races on x86_64 host (D-11)"
    requirement: CORR-03
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p ds slab (uncontended_resolve_is_one_slot_read, stress_concurrent_snapshots_are_never_torn, concurrent_snapshots_are_never_torn)"
        status: pass
      - kind: other
        ref: "nix develop -c cargo xtask miri (rc 0; 64 seeds x tpht 34 tests + slab 8 tests; zero UB/data-race lines)"
        status: pass
    human_judgment: false
  - id: D4
    description: "DEBT-04: home-hit and BanksFull ceiling measured at production default_banks across all 5 Distribution::ALL fixtures; documented bank-sizing rule, pub banks_for_occupancy, and non-ignored host pin default_banks_production_sizing (D-18, D-29)"
    requirement: DEBT-04
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p ds tpht::flattened::tests::default_banks (1 passed; weakest run 0.938235 at the 90% target, zero BanksFull, both default and rule bank counts)"
        status: pass
      - kind: other
        ref: "ignored generator measure_default_banks_table run on demand 2026-11-26 (grid 1,024/4,096/16,384 blocks x 5 distributions x 7 occupancies x 2 seeds; zero BanksFull everywhere; table recorded in the default_banks doc)"
        status: pass
    human_judgment: false

# Metrics
duration: 32min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 05: Generic Lock-Free Seqlock Slab and TPHT Production Bank Sizing Summary

**`ds/src/slab.rs` ships the `#![forbid(unsafe_code)]` Boehm fence-paired seqlock slab — 64-byte `AtomicSlabSlot<7>`, never-zero generations wrapping 65535→1, payload-scrubbing clunk, Roaring-backed `AllocMeta` with the WIRE-09 sweep — proven by probe-count, 65,536-cycle wrap, and torn-read stress pins plus a Miri step that now covers the slab; DEBT-04 closes with the measured production bank-sizing rule (`banks_for_occupancy`, weakest pinned run 0.938235 home-hit at the 90% target, zero `BanksFull` through 100% occupancy on all five distributions).**

## Performance

- **Duration:** 32 min
- **Started:** 2026-11-26T04:31:11Z
- **Completed:** 2026-11-26T05:03:28Z
- **Tasks:** 3
- **Files modified:** 4 (1 created)

## Accomplishments

- **CORR-03 — the generic slab exists and is proven.** `ds/src/slab.rs` (957 lines including tests) implements `AtomicSlabSlot<const WORDS: usize = 7>` at exactly 64 bytes / 64-byte aligned (compile-time asserted and host-pinned), the Boehm fence-paired seqlock (`resolve`: Acquire load, spin on WRITING, reject unoccupied/gen-mismatch/zero-gen, Relaxed payload reads, Acquire fence, Relaxed reload, retry on change; writers: `state|WRITING` Relaxed → `fence(Release)` → payload Relaxed → published state Release), `SlabSlots` (flat power-of-two `Box<[AtomicSlabSlot]>`, never reallocating, D-12), and `AllocMeta` (`free_list` + `active`/`reclaimed` Roaring bitmaps; `alloc_slot`, `update_slot`, `clunk_slot`, `mark_reclaimed`, `sweep_unreclaimed` = `active \ reclaimed`). `ds/src/lib.rs` registers `#[forbid(unsafe_code)] pub mod slab;`; the module file itself also opens with `#![forbid(unsafe_code)]` per the plan.
- **CORR-03 — concurrency is measured, not assumed.** `CountingSlabProbe` pins an uncontended `resolve` at exactly 1 slot read and 0 metadata touches (WIRE-08 restated). The 65,536-cycle wrap test pins generation `1 → … → 65535 → 1` never hitting 0 (D-10). The native stress test (20k ops × 4 writers × 4 readers on correlated `[k, k+1, …, k+6]` payloads) and its Miri-sized twin assert every `Some` snapshot is internally consistent — zero torn reads. `cargo xtask miri` exits 0 over 64 seeds × (34 tpht + 8 slab) tests with zero UB/data-race diagnostics.
- **DEBT-04 — production bank sizing is measured and pinned.** The on-demand generator (`measure_default_banks_table`, ignored) measured home-hit and `BanksFull` onset at production `default_banks` and at the rule count across `1,024 / 4,096 / 16,384` blocks × 5 distributions × occupancies 0.50–1.00 × 2 seeds: zero `BanksFull` everywhere; home-hit ≥ 0.9185 at ≤95% occupancy, 0.9382 weakest at the 90% production target, and at the 0.90 bar's edge at 100% (weakest 0.8977; home-hit is bank-independent — the D-15 cell reproduces at 0.9004). The rule is documented on `default_banks` and exported as `pub fn banks_for_occupancy(num_blocks, max_occ_pct) = entries / (BANK_NODES * 5)` (twice the measured worst-case spill, minimum 2). The non-ignored `default_banks_production_sizing` pin re-runs the 90% cell with both bank counts on every `cargo test -p ds` and under Miri at small size. Zero `unsafe` tokens added in `ds/src/tpht/`.

## Task Commits

Each task was committed atomically, RED before GREEN (IG-V01):

1. **Task 1, RED: failing slab behavior pins with module scaffolding (CORR-03)** — `47e68c3` (test) — 47 passed (46 pre-existing + the 64-byte layout pin), 4 failed on planned assertions
2. **Task 1, GREEN: Boehm fence-paired seqlock slab (CORR-03)** — `e66eb6d` (feat) — 51 passed, twice consecutively; lintcontract green
3. **Task 2: probe-count, wrap, and torn-read stress pins; Miri covers slab (CORR-03)** — `16a136f` (test) — 55 ds tests pass; `cargo xtask miri` rc 0 over both legs
4. **Task 3, RED: failing DEBT-04 production bank-sizing pin (DEBT-04)** — `b82b908` (test) — pin fails on `BanksFull` with the 2-bank skeleton (insert 45,086 at 90% occupancy), the planned-behavior failure
5. **Task 3, GREEN: production bank-sizing rule and `banks_for_occupancy` (DEBT-04)** — `eada265` (feat) — 56 ds tests pass twice; Miri rc 0; lintcontract green; full `cargo xtask test` rc 0 (56 `test result: ok` lines)

**Plan metadata:** the SUMMARY commit (docs: complete plan).

## Files Created/Modified

- `ds/src/slab.rs` — **created.** Module doc (D-09..D-12 contract), state-bit constants, `next_generation`, `SlabProbe`/`NoSlabProbe`, `AtomicSlabSlot` (new/resolve/publish_alloc/publish_update/publish_clunk/publish_clunk_any), `SlabSlots` (new/with_probe/capacity/slots/resolve), `AllocMeta` (new/counts/is_active/is_reclaimed/alloc_slot/update_slot/clunk_slot/mark_reclaimed/sweep_unreclaimed), and 9 tests
- `ds/src/lib.rs` — registered `#[forbid(unsafe_code)] pub mod slab;` with a note that slab.rs also carries the inner `#![forbid(unsafe_code)]`
- `ds/src/tpht/flattened.rs` — `default_banks` doc now records the measured DEBT-04 table; new `pub fn banks_for_occupancy`; tests gained the ignored `measure_default_banks_table` generator and the non-ignored `default_banks_production_sizing` pin; one new import (`BANK_NODES`)
- `xtask/src/miri.rs` — Miri covers `MIRI_MODULES = ["tpht", "slab"]` as per-module legs with the empty-filter refusal per leg; `command_spec` takes the module; `tpht_test_count` generalized to `module_test_count` (directory or single file); unit tests updated

## Decisions Made

- **`banks_for_occupancy` rule shape:** twice the measured worst-case spill volume (spill < 10% of entries at the 90% target; weakest home-hit 0.9382) over 255-node two-choice banks, `entries.div_ceil(BANK_NODES * 5).max(2)`. Chosen over reusing the D-15 grid's `measure::banks_for` (which sizes banks for 2× *full* occupancy — ≈10× more memory than the measurement justifies). The measurement validated the rule's count directly: rule-sized banks are `BanksFull`-free on the whole grid.
- **100% occupancy is documented as the edge, not the target:** at 100% the home-hit sits at the 0.90 bar (weakest 0.8977 at 4,096 sequential; 0.9004 at 16,384 — the D-15 cell) because home-hit is bank-independent; the rule's documented guarantee is >90% home-hit with zero `BanksFull` at ≤95% occupancy, with 90% as the production target for `palloc.hash` (MEM-16) and the Phase 6 union dcache.
- **Miri coverage mechanism:** extended `xtask/src/miri.rs` with a `MIRI_MODULES` list and per-leg filter/refusal instead of removing the filter (which would have pulled unscaled `roaring`/`clientset` tests into Miri) or renaming tests (no common substring exists between the `tpht` and `slab` module paths).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Extended `cargo xtask miri` to cover the slab module**
- **Found during:** Task 2 (Miri verification for CORR-03)
- **Issue:** The plan's verify command `nix develop -c cargo xtask miri` ran `miri test --package ds --lib tpht` — the `tpht` filter can never match `slab::tests::*`, so the CORR-03 must-have truth "cargo xtask miri prove zero torn reads, zero data races" was unprovable as written. `05-VALIDATION.md`'s sampling rule ("`cargo xtask miri` when `ds/src/tpht/` or `ds/src/slab.rs` changes") confirms the intent.
- **Fix:** Generalized `xtask/src/miri.rs`: `MIRI_MODULES = ["tpht", "slab"]` run as per-module legs, each with the existing empty-filter refusal; `command_spec` takes the module; `tpht_test_count` became `module_test_count` (handles both module directories and single-file modules like `slab.rs`); updated the module doc and unit tests. `xtask/src/main.rs` needed no change (`miri::run` signature unchanged).
- **Files modified:** `xtask/src/miri.rs`
- **Verification:** `cargo test -p xtask miri` (5 passed); `cargo xtask miri` rc 0 with slab tests running under Miri (274 slab test-result lines across 64 seeds × 8 tests)
- **Committed in:** `16a136f` (Task 2 commit)

**2. [Rule 1 - Bug] Fixed payload-constant typo in the sweep test**
- **Found during:** Task 1 GREEN implementation
- **Issue:** `mark_reclaimed_and_sweep_unreclaimed` allocated `payload(0x5E)` on the recycled slot but asserted `Some(payload(0x5))` — a test-side typo, not an implementation bug
- **Fix:** Asserted `Some(payload(0x5E))`; re-ran GREEN twice
- **Files modified:** `ds/src/slab.rs`
- **Verification:** `cargo test -p ds slab` 5 passed (GREEN-1 and GREEN-2)
- **Committed in:** `e66eb6d` (Task 1 GREEN commit; the RED commit `47e68c3` predates the fix but its RED validity is unaffected — the test failed at an earlier planned assertion)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both auto-fixes were necessary for the plan's own verification to mean what it says (Miri actually covering the slab) and for test correctness. No scope creep; no public API beyond the plan's artifact list; `Cargo.lock` unchanged (zero external packages installed — T-05-SC gate N/A).

## Issues Encountered

- **`gen` is a reserved keyword on the pinned toolchain** (`nightly-2026-01-30`, edition-2024 gen-blocks): local bindings named `gen` in the first skeleton draft failed to compile. Renamed locals to `generation` before the RED commit; no plan impact. Worth remembering for `port/src/fid.rs` in plan 05-06, which handles a `gen` field.
- **Miri run time:** with the slab leg added, `cargo xtask miri` runs two 64-seed legs (~3 s per slab seed at the Miri-scaled sizes). No budget breach observed; the slab's threaded test runs 30 ops × 2 writers/2 readers under Miri.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every artifact is fully wired: the slab is exercised by 9 tests, the bank-sizing rule by the pin and the on-demand generator, and the Miri step covers both modules.

## Next Phase Readiness

- **Plan 05-06** can build `port/src/fid.rs` on `ds::slab` immediately: `SlabSlots` + `AllocMeta` are the exact split 05-CONTEXT.md's "Claude's Discretion" anticipated (`ds/` exports the slot array and meta; `port/` wraps the meta in its MCS `Lock`). The owning-client check (D-08), `StatefulHandle` wire order (D-10), and shard routing (D-12) wrap this slab without touching it.
- **Plan 05-16/05-21** (`palloc.hash`, MEM-16): DEBT-04's production rule is available as `ds::tpht::flattened::banks_for_occupancy`; the flagged assumption (fixtures representative of `(image_id, page_idx)` keys) is verified live by the MEM-16 page-cache workload in plan 05-21.
- **Owner Stop 1** (`main` green, `boot-iso` pure, slab in): this plan completes the slab; CI proof lands on the GitHub-hosted runners per D-32/D-33.
- The `cargo xtask miri` step now guards both lock-free `ds/` structures; later Phase 5 plans (setops SIMD, page cache) inherit the extended coverage.

## TDD Gate Compliance

Both `tdd="true"` tasks followed RED → GREEN with committed failing pins before implementation (task 2 is `type="auto"` without the attribute and produced one test-type commit):

| Task | RED commit | GREEN commit | RED evidence (failing assertion, not compile error) |
|------|-----------|--------------|-----------------------------------------------------|
| 1 (slab) | `47e68c3` | `e66eb6d` | 4 failed on planned assertions: gen preserved on alloc (left 0, right 1), update succeeds, clunk scrubs/bumps, sweep bumps generation; 47 unaffected tests passed in the same binary |
| 3 (DEBT-04) | `b82b908` | `eada265` | pin failed on `BanksFull` at 90% occupancy with the 2-bank skeleton (insert 45,086) — the exact property the rule exists to guarantee |

No REFACTOR commits were needed; both GREEN implementations were final-form.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 5 files exist on disk: `ds/src/slab.rs` (created), `ds/src/lib.rs`, `ds/src/tpht/flattened.rs`, `xtask/src/miri.rs`, and this SUMMARY.
- All 5 commits are ancestors of HEAD: `47e68c3`, `e66eb6d`, `16a136f`, `b82b908`, `eada265`.
- Content pins verified: `pub mod slab;` in `ds/src/lib.rs`; `#![forbid(unsafe_code)]` inner attribute in `ds/src/slab.rs`; `pub fn banks_for_occupancy` and `default_banks_production_sizing` in `ds/src/tpht/flattened.rs`; `MIRI_MODULES` in `xtask/src/miri.rs`.
- Measured commits from ledger (`gsd-plan-head-before-05-05` = `bf9e7d0e`): 5 task commits; `plan_head_after` = `eada2655`.
- Working tree: only this SUMMARY untracked before its commit; zero `unsafe` tokens added in `ds/src/tpht/` (acceptance criterion); `Cargo.lock` untouched.
- Plan-level verification re-run at completion: `cargo test -p ds slab` ✓, `cargo test -p ds tpht::flattened::tests::default_banks` ✓, `cargo xtask miri` rc 0 ✓, `cargo test -p xtask lintcontract` ✓, `cargo xtask test` rc 0 ✓.
