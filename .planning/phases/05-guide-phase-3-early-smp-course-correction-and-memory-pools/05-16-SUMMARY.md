---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 16
subsystem: memory-pools
tags: [palloc, roaring, superpage, tpht, page-cache, devcons, sysstat, mem-07, mem-08, mem-16, tdd]

# Dependency graph
requires:
  - phase: 05-05
    provides: "DEBT-04 production bank sizing: pub fn banks_for_occupancy(num_blocks, max_occ_pct) on FlattenedTphtShard (measured, BanksFull-free at the 90% target)"
  - phase: 05-07
    provides: "port::smp LockRank table (PageCache 20, ProcTable 50, PageAlloc 70) and the rank-asserting IrqSpinLock"
  - phase: 05-15
    provides: "Owner Stop 3 / D-20 FP-format work completed; memory-pool plans unblocked"
provides:
  - "port/src/palloc.rs: PageAlloc with Roaring clean/dirty/active/pinned pools, exact pfn_to_phys/phys_to_pfn, palloc_max_frames bootarg, per-frame atomic pin counts, Dirty \\ (Pinned | Active) evictable set, High-Water refill outside PAGE_ALLOC, rank-safe alloc_clean evictor retry, lock-free EvictorStats"
  - "port/src/superpage.rs: exact 64-byte/64-byte-aligned SuperpageResidency ([AtomicU64; 8], 512 sub-pages) with AcqRel CRUD, count/is_full and Release demotion reset"
  - "port/src/pagecache.rs: palloc.hash image page cache on FlattenedTphtShard<2> sized by banks_for_occupancy(n, 85), exact 64-bit keys, shared-text rmap, droppable FIFO, mark_data_dirty, unpinned eviction selector that removes the key before return"
  - "port/src/devcons.rs: read-only #c/sysstat (QID_SYSSTAT = 10, 0444) formatting lock-free palloc::evictor_stats()"
  - "port/src/lib.rs: #[forbid(unsafe_code)] mounts for palloc, superpage, pagecache"
affects: [05-18, 05-19 (arch evictor wiring), 05-21 (MEM-16 workload), 05-22 (pin guards)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (55000, confidence: low).
actuals:
  tokens: 15766
  tasks: 3
  commits: 5
  plan_head_before: 2106cc9156c78e9b07163e18c7e698cfaf49e6ed
  plan_head_after: fa5255962219f4b2298ba8c189c3b258d91c25f8

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pool algebra over Roaring bitmaps: every transition takes IrqSpinLock(PoolState) at LockRank::PageAlloc (70); callbacks that re-enter lower ranks run only after the guard is dropped"
    - "Batch zeroing outside the rank-70 lock: claim dirty frames into a local Vec under the lock, zero them unlocked, then re-lock to publish into clean"
    - "TPHT value carries the full key: word 1 is the exact packed (image_id, page_idx) key while word 0 packs pa (bits 0..51) plus kind/generation (bits 52..63), so fingerprint matches never alias"
    - "Global counters are lock-free atomics plus a host-test mutex serialising assertions across modules"

key-files:
  created:
    - port/src/palloc.rs
    - port/src/superpage.rs
    - port/src/pagecache.rs
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-16-RED-task1.json
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-16-RED-task2.json
  modified:
    - port/src/lib.rs
    - port/src/devcons.rs
    - port/src/pagealloc.rs (unchanged; PageAllocError reused)

key-decisions:
  - "The pin-count table lives inside the PAGE_ALLOC (Rank 70) PoolState: forbid(unsafe_code) rules out a runtime-sized atomic table behind a raw pointer, and PAGE_CACHE (20) -> PAGE_ALLOC (70) is the documented ascending order. EvictorStats are the lock-free read path for #c/sysstat."
  - "init_ranges seeds every managed frame into dirty and sizes target_clean/low_water; the boot path must call refill_clean_pool(zero_frame) once before allocating, so no unzeroed frame ever reaches clean"
  - "pagecache stores the full 64-bit key in TPHT value word 1 (pa+meta in word 0) because FlattenedTphtShard's same() callback sees only the value; this makes collision aliasing impossible"
  - "mark_data_dirty(pa) scans the droppable FIFO by physical address (the behaviour contract names pa), removing every queue entry for that frame permanently"
  - "PageCache::select_eviction_victim scans at most the queue length, re-queues pinned frames and counts them via record_pinned_skipped, and drops stale queue entries whose hash key is already gone"

patterns-established:
  - "Rank-safe evictor contract: alloc_clean drops PAGE_ALLOC before evict_fn; the evictor may take PAGE_CACHE (20) -> PROC_TABLE (50) and re-enter PAGE_ALLOC (70) to free reclaimed frames (pinned by a host test that takes a PageCache-rank lock inside evict_fn)"
  - "Every memory-pool pin runs in both a single-thread unit form and a 2-thread contention form; the contention pins assert zero duplicate frames and zero pinned evictions"

requirements-completed: [MEM-07, MEM-08, MEM-16]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Roaring frame pools: exact chunk/offset address formula, Dirty \\ (Pinned | Active), nested pin counts, High-Water refill, rank-safe evictor retry, palloc_max_frames cap (MEM-07)"
    requirement: MEM-07
    verification:
      - kind: unit
        ref: "port/src/palloc.rs: pfn_to_phys_is_exact_across_the_256mib_chunk_boundary, evictable_dirty_excludes_pinned_and_active_frames, nested_pin_unpin_transitions_once, high_water_refill_and_evictor_run_without_the_page_alloc_lock, max_frames_bootarg_caps_managed_frames, free_into_empty_pools_and_double_free_are_clean"
        status: pass
      - kind: integration
        ref: "port/src/palloc.rs: pin_guard_and_concurrent_alloc_evict (2 threads; nix develop -c cargo test -p port)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Exact 64-byte, 64-byte-aligned SuperpageResidency over 512 sub-pages with atomic CRUD and demotion reset (MEM-08)"
    requirement: MEM-08
    verification:
      - kind: unit
        ref: "port/src/superpage.rs: superpage_residency_is_exactly_64_bytes_and_counts_bits, superpage_bits_cross_word_boundaries_without_overflow"
        status: pass
    human_judgment: false
  - id: D3
    description: "Flattened-TPHT palloc.hash image page cache with exact keys, shared-text rmap, droppable FIFO, mark_data_dirty and unpinned eviction selection (MEM-16)"
    requirement: MEM-16
    verification:
      - kind: unit
        ref: "port/src/pagecache.rs: shared_text_maps_one_frame_into_two_processes, mark_data_dirty_removes_the_page_permanently, select_eviction_victim_skips_pinned_frames, remove_proc_rmaps_keeps_shared_text_cached, adjacent_keys_are_distinct_and_cold_lookup_is_none"
        status: pass
      - kind: integration
        ref: "port/src/pagecache.rs: concurrent_lookup_insert_rmap_and_eviction (writer + evictor threads)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Read-only #c/sysstat (QID_SYSSTAT = 10, 0444) formatting evictor: dropped_text/dropped_data/pinned_skipped, with offset discipline and write rejection (MEM-07, D-17)"
    requirement: MEM-07
    verification:
      - kind: unit
        ref: "port/src/devcons.rs: sysstat_reports_evictor_counters_read_only"
        status: pass
      - kind: automated
        ref: "nix develop -c cargo test -p port && cargo xtask check && cargo test -p xtask lintcontract"
        status: pass
    human_judgment: false

# Metrics
duration: 6min
completed: 2026-11-26
status: complete
---

# Phase 5 Plan 16: Portable Memory Pools, Superpage Residency and palloc.hash Page Cache Summary

**Roaring clean/dirty/active/pinned frame pools with rank-safe eviction, an exact 64-byte superpage bitmap, and a Flattened-TPHT image page cache with shared-text rmap and read-only `#c/sysstat` counters**

## Performance

- **Duration:** 6 min
- **Started:** 2026-11-26T23:37:00Z (worktree execution)
- **Completed:** 2026-11-26T23:42:44Z
- **Tasks:** 3
- **Files modified:** 7 (5 source/planning sources plus 2 RED evidence records)

## Accomplishments

- `port/src/palloc.rs` implements the `PageAlloc` pool algebra under `#[forbid(unsafe_code)]`: exact `(((chunk as usize) << 16) | offset) * 4096` addressing across the 256 MiB chunk boundary, `Dirty \ (Pinned ∪ Active)` through `RoaringBitmap::difference`/`union`, per-frame atomic pin counts with nested pins, a ~15% High-Water refill that zeroes unlocked, and `alloc_clean` that drops `PAGE_ALLOC` (Rank 70) before `evict_fn` and retries (bounded) instead of panicking.
- `port/src/superpage.rs` ships the exact `#[repr(C, align(64))] SuperpageResidency` (`[AtomicU64; 8]`, size 64, align 64) with `AcqRel` bit transitions, `count`/`is_full`, bounds-checked indices, and a per-word `Release` demotion `reset`.
- `port/src/pagecache.rs` builds `palloc.hash` on `FlattenedTphtShard<2>` with `banks_for_occupancy(n, 85)`, stores the full 64-bit key in the value so fingerprint matches cannot alias, tracks `(proc_slot, page_va)` rmap entries, removes written data from the droppable FIFO permanently, and removes a victim's key from the hash before returning it for arch unmap and TLB shootdown.
- `port/src/devcons.rs` serves read-only `#c/sysstat` (`QID_SYSSTAT = 10`, 0444) formatting the lock-free `palloc::evictor_stats()` line with offset/EOF discipline and `write` rejection.
- Both TDD tasks carry `RED_EVIDENCE_OK` records (6/6 palloc+superpage pins failing, 5/5 pagecache pins plus unwired `#c/sysstat` failing) and GREEN implementations; Task 3 adds 2-thread contention pins proving zero duplicate frame allocation, zero pinned evictions and rank-safe `PAGE_CACHE -> PAGE_ALLOC` operation.

## Task Commits

Each task was committed atomically (TDD tasks split RED/GREEN):

1. **Task 1 RED: failing pins for palloc frame pools and superpage residency** — `afdeb54` (test)
2. **Task 1 GREEN: Roaring frame pools + 64-byte superpage residency** — `b1648bc` (feat)
3. **Task 2 RED: failing pins for palloc.hash and #c/sysstat** — `7600c70` (test)
4. **Task 2 GREEN: palloc.hash page cache + read-only #c/sysstat** — `53ab20a` (feat)
5. **Task 3: multi-threaded pool and page-cache concurrency pins** — `fa52559` (test)

## Files Created/Modified

- `port/src/palloc.rs` — `PageAlloc`, `EvictorStats`, exact address helpers, bootarg parser, pool algebra, High-Water refill, rank-safe `alloc_clean`, 8 tests (7 unit + 1 concurrency)
- `port/src/superpage.rs` — 64-byte `SuperpageResidency`, 2 tests
- `port/src/pagecache.rs` — `PageCache`, `PageKey`, `PageKind`, `RmapEntry`, `EvictionCandidate`, 6 tests (5 unit + 1 concurrency)
- `port/src/lib.rs` — `#[forbid(unsafe_code)]` mounts for `palloc`, `superpage`, `pagecache`
- `port/src/devcons.rs` — `QID_SYSSTAT`, 10-entry `FILES`, read-only open/read/write/stat wiring, sysstat pin
- `.planning/.../05-16-RED-task1.json`, `05-16-RED-task2.json` — RED evidence records

## Decisions Made

- **Pin counts under `PAGE_ALLOC` rather than a raw lock-free table.** `forbid(unsafe_code)` forbids a runtime-sized `AtomicU32` table reachable without a lock; the table therefore lives in `PoolState` and `is_pinned` takes the rank-70 lock. `PAGE_CACHE (20) -> PAGE_ALLOC (70)` is the documented ascending order and no path holds 70 while taking 20. `EvictorStats` remain the lock-free `#c/sysstat` path.
- **`init_ranges` seeds `dirty`; the boot caller performs the first High-Water refill** with its real `zero_frame`, keeping unzeroed RAM out of `clean`.
- **The TPHT value carries the full key.** `same()` sees only the value, so word 1 is the exact packed key and word 0 packs pa plus kind/generation; this makes the MEM-16 adjacency requirement exact.
- **`mark_data_dirty` identifies a frame by PA** (the behaviour contract's argument) and removes every matching droppable entry permanently.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pin table is not literally lock-free**
- **Found during:** Task 1 (palloc implementation)
- **Issue:** The plan's action text asks for a "lock-free `pin_counts: Box<[AtomicU32]>` table", but the table is sized at `init_ranges` time and `port/` is `#[forbid(unsafe_code)]`; publishing a runtime-sized table without a lock needs an unsafe pointer cell or a `'static` leak.
- **Fix:** The atomic table lives in `PoolState` under `PAGE_ALLOC` (Rank 70); `is_pinned` takes that lock, which is legal from `PAGE_CACHE` (20) under the documented rank order. The counters themselves are atomic and nested pins are correct; `evictor_stats()` is fully lock-free for `#c/sysstat`.
- **Files modified:** `port/src/palloc.rs`
- **Verification:** `pin_guard_and_concurrent_alloc_evict` and `select_eviction_victim_skips_pinned_frames` pass with zero rank panics.
- **Committed in:** `b1648bc` (Task 1 GREEN)

**2. [Rule 3 - Blocking] Initial High-Water refill is explicit at boot**
- **Found during:** Task 1 (pool seeding)
- **Issue:** `init_ranges`'s fixed signature has no `zero_frame`, so it cannot zero frames before moving them to `clean`.
- **Fix:** `init_ranges` seeds all managed frames into `dirty` and sizes the target/low-water marks; the arch boot plan calls `refill_clean_pool(zero_frame)` once before allocating. Filling `clean` with unzeroed RAM would violate T-05-29.
- **Files modified:** `port/src/palloc.rs`
- **Verification:** `high_water_refill_and_evictor_run_without_the_page_alloc_lock` asserts the refill zeroes each frame and reaches 15%.
- **Committed in:** `b1648bc` (Task 1 GREEN)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both preserve the planned semantics exactly; arch plans 05-18/05-19 must call `refill_clean_pool` after `init_ranges` and treat `is_pinned` as rank-70-safe rather than lock-free.

## Issues Encountered

- The first GREEN build reused a stale test binary after `cp` (cargo missed the mtime); forced with `touch` before re-running. No code impact.
- The pool-algebra pin originally asserted `evictable.len() == 64 - active - pinned`, which ignored refilled clean frames; the test now derives the exact expectation from live pool cardinalities via a test-only `pool_lens()` accessor.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- MEM-07/MEM-08/MEM-16 portable cores are in place; Plan 05-18 (aarch64) and 05-19 (x86_64) can wire `PageAlloc::init_ranges` + `refill_clean_pool`, the `pagecache` victim selector into the arch evictor, and `#c/sysstat` is live for gate assertions.
- `cargo test -p port` (200 lib + all integration legs), `cargo xtask check` (rc 0) and `cargo test -p xtask lintcontract` (13/13) all pass in this worktree.
- Note for arch plans: `is_pinned` acquires `PAGE_ALLOC` (70) and must not be called from a path already holding a rank above 70.

## Self-Check: PASSED

- Created files exist: `port/src/palloc.rs`, `port/src/superpage.rs`, `port/src/pagecache.rs`, both RED JSON records.
- Commits `afdeb54`, `b1648bc`, `7600c70`, `53ab20a`, `fa52559` are ancestors of HEAD.
- `#[forbid(unsafe_code)]` present on all three new module mounts in `port/src/lib.rs`.
