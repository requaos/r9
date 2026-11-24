---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 05
subsystem: ds
tags: [ds, roaring, bitmap, clientset, set-algebra, differential-testing, no_std, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "ds/ workspace crate with the D-09 per-module forbid layout and `roaring =0.11.5` as its only crate, under [dev-dependencies] (@03-12)"
provides:
  - "ds::roaring::RoaringBitmap: array, bitmap and run containers with membership, ascending iteration, insert_range and run_optimize"
  - "the four DS-05 set operations (union, intersection, difference, andnot) matching a BTreeSet model and the crates.io roaring oracle on seeded randomized inputs across all nine container pairings"
  - "ds::setops: the D-20 seam — the four scalar hot loops (bitmap and/or/andnot, array intersection) a Phase 5 SIMD module can replace"
  - "ds::clientset::ClientSet: Empty | Single | Inline { count, ids } | Roaring, heap-free to four clients, at most 24 bytes, upgrading at the fifth distinct id"
  - "RoaringBitmap::insert_compact (crate-visible): per-insert smallest-form rewrite so contiguous client ids collapse into one run container"
affects: [03-06, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 20397
  tasks: 3
  commits: 9
  plan_head_before: ae67657b48801a655634f3776eff446c275b5c00
  plan_head_after: 3842fbac8c0fbe2603677128464bd231eb84309b

tech-stack:
  added: []
  patterns:
    - "Roaring chunk list: sorted Vec<Chunk> keyed by the high 16 bits, one of three containers per chunk; conversion sites are fixed and enumerated in the module doc"
    - "Set operations are chunk-wise merges that dispatch by container pairing: bitmap|bitmap through setops kernels, array|array intersection through array_intersect, array union/difference by two-pointer merge, mixed pairings by probe/paint"
    - "Result containers are array at or below ARRAY_MAX else bitmap; an empty result yields None so no empty container survives"
    - "smallest_form byte accounting with a deterministic tie order (array, then run, then bitmap) used by run_optimize and insert_compact"
    - "IG-V01: committed RED pin, GREEN twice, ADVANCED commit in evidence/PINS-03-05.md; eight literal SplitMix64 seeds per distribution, seed printed in every assert message"

key-files:
  created:
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-05.md
  modified:
    - ds/src/roaring.rs
    - ds/src/setops.rs
    - ds/src/clientset.rs

key-decisions:
  - "A run container stores (start, length) where a pair covers start..=start+length (length counts the additional values, as in Lemire et al.); runs are sorted, non-overlapping and non-adjacent"
  - "Container conversion happens only at the four sites the plan fixes: array to bitmap past ARRAY_MAX, bitmap to array at ARRAY_MAX on removal, run_optimize, and insert_compact's single-container rewrite. Set operations never convert an operand; they build new array/bitmap results"
  - "smallest_form compares payload bytes (array 2*card, bitmap 8192, run 4*runs) and breaks ties array, then run, then bitmap, so the outcome is deterministic and a 4096-cardinality set stays an array"
  - "Set-op dispatch: bitmap|bitmap uses setops::bitmap_and/bitmap_or/bitmap_andnot; array|array intersection uses setops::array_intersect; array|array union and difference are two-pointer merges in roaring.rs; array with bitmap/run probes or paints the sorted array, so a sparse 1,500-chunk set never pays an 8 KiB scratch copy per chunk"
  - "Run|run and run|bitmap mixed pairings convert to bitmap words and call the kernel; those containers exist only for dense chunks, so the conversion is bounded"
  - "zero_words builds the boxed [u64; 1024] through Vec::into_boxed_slice plus try_into, avoiding an 8 KiB stack temporary in a no_std crate that Phase 5 links into the kernel"
  - "PartialEq compares contents, so two bitmaps with equal values and different container kinds are equal; Debug prints only the length to keep a failing assert readable"
  - "Claude's discretion (DS-06): insert_compact rewrites the touched container in its smallest form after every insert; one pass over one container (at most 8 KiB) per insert, accepted because the lease path reads the cohort far more often than it changes it (T-03-21)"
  - "Claude's discretion (DS-06): Roaring does not downgrade when ids are removed — a cohort that outgrew four clients stays a Roaring set, documented in the module doc and pinned by removal_rules"
  - "The no-heap property is proved by variant and size assertions (size_of::<ClientSet>() <= 24, and Empty/Single/Inline are pointer-free), not by a counting allocator, which would need a third restricted-feature module outside ds/src/tpht/ (D-09)"

requirements-completed: [DS-05, DS-06]

coverage:
  - id: D1
    description: "RoaringBitmap holds array, bitmap and run containers; membership, len and ascending iteration agree with a BTreeSet model and the crates.io roaring oracle over seeded insert/remove sequences in five distributions; the array-to-bitmap conversion fires at 4097; run_optimize collapses a contiguous chunk and touching runs into one run"
    requirement: DS-05
    verification:
      - kind: unit
        ref: "ds/src/roaring.rs#membership_matches_models, container_kinds_and_conversions (8 seeds x 5 distributions; `cargo test -p ds --lib roaring::` 6 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "union, intersection, difference and andnot equal both oracles by contents and len for every pairing of the five distributions, before and after run_optimize on either side; the empty-set identities hold and an empty result leaves no container; every one of the nine container pairings is exercised; 65535 and 65536 survive across the chunk boundary"
    requirement: DS-05
    verification:
      - kind: unit
        ref: "ds/src/roaring.rs#set_algebra_matches_models, set_algebra_with_empty, set_algebra_container_pairings, chunk_boundary_values_survive (`cargo test -p ds --lib roaring::tests::set_algebra` 3 passed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The four hot loops live in ds/src/setops.rs as plain slice functions and match bit-by-bit / nested-loop references; roaring.rs reaches the seam on 7 lines and contains no SIMD, no core::arch, no target_feature and no intrinsic"
    requirement: DS-05
    verification:
      - kind: unit
        ref: "ds/src/setops.rs#bitmap_kernels_match_naive, array_intersect_matches_naive (2 passed)"
        status: pass
      - kind: other
        ref: "grep -c 'setops::' ds/src/roaring.rs => 7 (>= 4); grep -cE 'core::arch|target_feature|intrinsic' ds/src/{roaring,setops}.rs => 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "ClientSet progresses Empty -> Single -> Inline (counts 2, 3, 4) -> Roaring at the fifth distinct id, ignores duplicate inserts, keeps size_of <= 24 bytes with no pointer in the first three variants, applies the removal rules (Inline to Single to Empty; Roaring never downgrades), collapses contiguous ids to one run container in ascending and shuffled orders, and matches a BTreeSet model over seeded insert/remove sequences including 0 and u32::MAX"
    requirement: DS-06
    verification:
      - kind: unit
        ref: "ds/src/clientset.rs#variant_progression, no_heap_up_to_four, removal_rules, contiguous_ids_collapse_to_one_run, matches_btreeset_model (`cargo test -p ds --lib clientset::` 5 passed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "insert_compact and the no-downgrade rule are documented in the module doc together with the per-insert cost (T-03-21), and ds/src/clientset.rs holds no counting-allocator machinery"
    requirement: DS-06
    verification:
      - kind: other
        ref: "ds/src/clientset.rs module doc; grep -cE 'global_allocator|GlobalAlloc' ds/src/clientset.rs => 0; removal_rules pins no-downgrade and contiguous_ids_collapse_to_one_run pins insert_compact"
        status: pass
    human_judgment: false

# Metrics
duration: 4min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 05: Roaring bitmap and `ClientSet`

**A hand-written Roaring bitmap with array, bitmap and run containers whose four set operations match both a `BTreeSet` model and the crates.io `roaring` oracle over seeded randomized inputs, plus a `ClientSet` that is heap-free to four clients and collapses contiguous ids into one run container.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-11-24T14:40:21Z
- **Completed:** 2026-11-24T14:44:41Z
- **Tasks:** 3
- **Files modified:** 4 (3 source files, 1 ledger)

## Accomplishments

- `ds::roaring::RoaringBitmap` is a sorted chunk list keyed by the high 16 bits, one container per chunk: `Array` (sorted `Vec<u16>`, at most `ARRAY_MAX = 4096`), `Bitmap` (boxed `[u64; 1024]` plus cardinality) and `Run` (sorted `(start, length)` pairs where a pair covers `start..=start+length`). It provides `new`, `insert`, `remove`, `contains`, `len`, `is_empty`, `iter`, `insert_range`, `run_optimize`, the four set operations, the crate-visible `insert_compact`, and the `#[cfg(test)]` helpers `container_kinds` and `run_counts`.
- Membership, `len` and ascending iteration agree with two independent oracles over seeded insert/remove sequences in five distributions (sparse random, dense inside one 65,536-value chunk, random ranges, boundary values, and a mix), with eight literal seeds each and the seed printed in every assert message.
- Conversion sites are the four the plan fixes and nowhere else: array→bitmap when an insert passes `ARRAY_MAX`, bitmap→array when a removal reaches `ARRAY_MAX`, `run_optimize` for every container, and `insert_compact` for the one container it touched. A 4097th value turns an array into a bitmap and `run_optimize` turns a contiguous 4097-value chunk (and touching runs `[1..=5]`/`[6..=9]`) into one run container.
- `union`, `intersection`, `difference` and `andnot` equal both oracles by contents and `len` for all 25 distribution pairings, each with four `run_optimize` variants (none, either side, both). The nine container pairings are exercised explicitly; the empty-set identities hold; an empty result drops its container; values 65,535 and 65,536 stay in different chunks and survive every operation that should keep them.
- `ds::setops` holds exactly four non-test functions: `bitmap_and`, `bitmap_or`, `bitmap_andnot` and `array_intersect`. The three bitmap kernels are word loops using `count_ones`; the array intersection is a two-pointer merge. They match bit-by-bit and nested-loop references over seeded inputs including empty, disjoint and all-ones cases. `roaring.rs` reaches the seam on 7 `setops::` lines; no word loop over two bitmap containers and no array intersection is written anywhere else.
- `ClientSet` is `Empty | Single(u32) | Inline { count, ids: [u32; 4] } | Roaring(Box<RoaringBitmap>)`. `size_of::<ClientSet>() <= 24` and the first three variants hold no pointer, so no heap is reachable with up to four clients; the fifth distinct id upgrades to `Roaring`. Removal rules are pinned (`Inline` with two ids → `Single`, `Single` → `Empty`, `Roaring` never downgrades), duplicates change nothing, and ids 0 and `u32::MAX` work in every variant.
- `insert_compact` makes contiguous client ids collapse: inserting 100..=4096 in ascending order and again in a seeded shuffled order both leave exactly one run container holding one run, because every `Roaring` insert rewrites only its touched container in the smallest form.
- `cargo xtask test` (twice), `cargo xtask check` and `cargo xtask clippy` all exit 0 with no `ds/` warning. `ds/Cargo.toml` and `Cargo.lock` are unchanged: the `roaring` oracle stays a dev-dependency and never links into a kernel. The 13 `ds` lib tests pass (2 setops, 6 roaring, 5 clientset).

## Task Commits

Each task was committed atomically, RED before GREEN:

1. **Task 1, RED: containers, membership and the `setops` seam** — `6c337aa` (test)
2. **Task 1, GREEN: Roaring containers and scalar setops** — `9677948` (feat)
3. **Task 1, pin advance** — `5d5042e` (evidence)
4. **Task 2, RED: set algebra against two oracles** — `95401f3` (test)
5. **Task 2, GREEN: union, intersection, difference, andnot** — `599c7f6` (feat)
6. **Task 2, pin advance** — `5d807eb` (evidence)
7. **Task 3, RED: `ClientSet`** — `8fa90b7` (test)
8. **Task 3, GREEN: `ClientSet`, heap-free to four clients** — `60b5988` (feat)
9. **Task 3, pin advance** — `3842fba` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `ds/src/roaring.rs` — `RoaringBitmap`, the three containers, `ARRAY_MAX`, membership/iteration/`insert_range`/`run_optimize`/`insert_compact`, the four set operations, `PartialEq` by contents, a length-only `Debug`, and the `#[cfg(test)]` `ContainerKind`/`container_kinds`/`run_counts` helpers; the module doc records D-19, the fixed conversion sites, the `(start, length)` run encoding and the D-20 seam
- `ds/src/setops.rs` — the four D-20 kernels over slices (`&[u64; 1024]` for the bitmap pairs, `&[u16]` for array intersection) with their differential tests
- `ds/src/clientset.rs` — `ClientSet` with its canonical forms, `insert`/`remove`/`contains`/`len`/`is_empty`/`iter`, the no-downgrade rule, the no-heap proof by variant and `size_of`, and the `insert_compact` cost note; five tests including the ascending and shuffled run-collapse cases
- `evidence/PINS-03-05.md` — RED, GREEN-1, GREEN-2 and ADVANCED for `roaring-membership`, `roaring-set-algebra` and `clientset` (12 `^PIN` lines)

## Decisions Made

See `key-decisions` in the frontmatter. The two items the plan asked to list under "Claude's discretion":

- **`insert_compact` rule.** Every `ClientSet` insert in the `Roaring` state inserts and then rewrites only the touched container in its smallest form — the same rule `run_optimize` applies to all containers. The cost is one pass over one container per insert (at most 8 KiB scanned); the lease path reads a cohort far more often than it changes it, so the trade is accepted (T-03-21, revisit with lease measurements in Phase 6).
- **No-downgrade rule.** A `Roaring` set stays `Roaring` when ids are removed, even when it becomes empty. The inline forms were sized for the common small cohort, not for shrinking, and a cohort that outgrew four clients is expected to stay large. The module doc states the rule and `removal_rules` pins it.

Other decisions worth recording: mixed set-operation pairings probe or paint the sorted array instead of converting sparse containers to 8 KiB bitmaps (which keeps the 1,500-chunk distributions fast); union/difference of two arrays are two-pointer merges in `roaring.rs` because only array *intersection* is one of the four hot loops; and `zero_words` builds its box without an 8 KiB stack temporary.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The plan-commit ledger sentinel (`$(git rev-parse --git-dir)/gsd-plan-head-before-03-05`) was not created before the first RED commit. It was written after the fact with the true plan start — `ae67657`, the parent of `6c337aa` (the first RED commit) — so the measured `commits` count is 9. The same omission was recorded by plans 03-03 and 03-04.
- The first version of the differential helper used `assert_eq!` on whole value vectors, which printed thousands of values at RED. It was changed to report the first diverging index and both values while still naming the seed, and the RED pin was then recorded against that form.
- Clippy reported three advisories on the new code (a collapsible `if let`, two redundant `u16` casts, a manual `is_multiple_of`). All were fixed; `cargo xtask clippy` exits 0 with no `ds/` warning.

## Known Stubs

None. Every RED skeleton was replaced by the GREEN implementation in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-19 is mitigated by the two-oracle differential tests over all nine container pairings and the five distributions with fixed seeds; T-03-20 by `#[forbid(unsafe_code)]` on all three modules, the lint-contract rules, and the acceptance check that `roaring.rs`/`setops.rs` contain no SIMD or target-feature attribute; T-03-21 is accepted with the cost documented in the module doc. No dependency was added and `Cargo.lock` is unchanged (T-03-SC).

## For owner review

- **`insert_compact` cost (T-03-21).** The per-insert smallest-form rewrite is O(one container) and is accepted for Phase 3. Phase 6's lease work should measure it against a large cohort and either keep it or batch the rewrite.
- **Smallest-form byte accounting** counts container payload only (array 2 bytes/value, bitmap 8192, run 4 bytes/run), not `Vec`/`Box` overhead; ties prefer array, then run, then bitmap. This is documented in `smallest_form`'s doc comment and is what makes the run/array conversions deterministic.
- Plan 03-06 (TPHT) can proceed: `ds/src/roaring.rs`, `setops.rs` and `clientset.rs` are complete and no other `ds` module changed.

## Next Phase Readiness

- DS-05 and DS-06 are complete on the host for both crate targets the check legs cover; `cargo xtask check` still compiles the crate for both r9 kernel specs with `-Z build-std=core,alloc`, so the new code is `no_std`-clean.
- The Phase 5 SIMD seam is in place: the four hot loops are plain functions in `setops.rs` and no other code writes a word loop over two bitmap containers, so replacing the bodies needs no caller change. No SIMD, no intrinsics and no restricted-feature token landed in this phase.
- The pager (Phase 5), the `FidShard` sweep (Phase 4) and lease cohorts (Phase 6) can be built on `RoaringBitmap` and `ClientSet`; `insert_compact` keeps contiguous lease ids compact today and Phase 6 may revisit its cost with measurements.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All four claimed files exist on disk and all nine task commits are ancestors of HEAD. Plan-level verification at the final state: `nix develop -c cargo test -p ds --lib` reports 13 passed; `cargo xtask test` (rc 0, 30 `test result: ok` lines), `cargo xtask check` (rc 0) and `cargo xtask clippy` (rc 0) all pass; `grep -c 'setops::' ds/src/roaring.rs` prints 7; the ledger holds 12 `^PIN` lines (RED, GREEN-1, GREEN-2, ADVANCED for each of the three pin ids).
