---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 18
subsystem: memory-paging
tags: [aarch64, palloc, pagecache, palloc-hash, ro-text, write-protect, evictor, pinning, mem-07, mem-10, mem-15, mem-16]

# Dependency graph
requires:
  - phase: 05-16
    provides: "port::palloc PageAlloc (clean/dirty/active/pinned, High-Water refill, rank-safe alloc_clean), port::pagecache PageCache (palloc.hash, rmap, droppable FIFO, mark_data_dirty, select_eviction_victim) and read-only #c/sysstat counters"
  - phase: 05-17
    provides: "runtime-dispatched NEON Roaring set kernels behind fpu::with_simd() (setops: simd=neon selftest=ok)"
provides:
  - "aarch64/src/vm.rs: Entry::ro_user_text/ro_user_data plus leaf_entry_mut, retarget_kernel_leaf, set_user_leaf, clear_leaf_pte and tlbi vae1is"
  - "aarch64/src/pagealloc.rs: PAGE_ALLOC on port::palloc PageAlloc with palloc_max_frames=<N>, High-Water refill, reusable zero alias, live evictor callback and the one-shot palloc: boot marker"
  - "aarch64/src/fault.rs: shared RO text through palloc.hash with concurrent-insert dedup, RO untouched data with write-confirm promotion, reclaim-and-retry page-in, and the PAGE_CACHE -> PROC_TABLE -> shootdown -> PAGE_ALLOC live evictor"
  - "aarch64/src/proc.rs, exec.rs: image ids, shared-text rfork, untouched-data registration, evict_user_frame and cached-frame-aware exec/exit teardown"
  - "aarch64/src/usermem.rs, syscall.rs: RAII PinGuard on UserSlice/UserSliceMut, EL1 write promotion and the eight sleeping-path audit"
affects: [05-19 (x86_64 mirror), 05-21 (MEM-15 pressure workload), 05-22 (pin-guard follow-ups)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan estimate (55000, confidence: low).
actuals:
  tokens: 17848
  tasks: 3
  commits: 2
  plan_head_before: 47079e3a035e2d5c97fca326905c0741a2fb11ae
  plan_head_after: 412100188084d07a5c4f560af041d1ba9fedce8a

tech-stack:
  added: []
  patterns:
    - "Boot-published 'static handles: PageAlloc and PageCache are built once with Box::into_raw and read through AtomicPtr because their Roaring/Vec state is not const"
    - "One reusable kernel zero alias: retarget a single established leaf (write_volatile + tlbi vae1is) instead of a full map_to dance per zeroed frame"
    - "Image identity: paqfs qid mixed with the mapped image base (devroot) or an FNV-1a content hash (devmnt), so processes of one binary share text"
    - "Write-protect dirty tracking: RO data PTE -> EL0/EL1 write fault -> promote to AllRw + mark_data_dirty, preserving contents"
    - "Per-proc salted data cache keys (pid + exec generation) keep private untouched-data pages from aliasing across processes/execs"
    - "RAII PinGuard with an inline [u32; 4] fast path (<= 16 KiB) and Vec overflow, rolled back on a mid-span error"

key-files:
  created: []
  modified:
    - aarch64/src/vm.rs
    - aarch64/src/pagealloc.rs
    - aarch64/src/fault.rs
    - aarch64/src/exec.rs
    - aarch64/src/proc.rs
    - aarch64/src/usermem.rs
    - aarch64/src/syscall.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "PageAlloc and PageCache are boot-published AtomicPtr handles (Box::into_raw) rather than const statics: port::palloc's RoaringBitmaps and Vec pools cannot be built in a const initializer."
  - "Untouched initialised data is mapped eagerly at exec as AllRo and registered in palloc.hash (D-15/D-18); a later eviction plus re-fault reproduces demand loading without changing the cold-syscall-buffer guarantees."
  - "The evictor clears PTEs by walking PROC_TABLE and each proc's mapping_log (the only structure that names a proc's root table), while still maintaining the page cache rmap; EvictionCandidate.rmaps is used for cache bookkeeping, not for locating root tables."
  - "Dirty (written) data pages stay in palloc.hash after mark_data_dirty removes them from the droppable queue and are not freed at exec/exit; frames are reclaimed only by a future cache-removal path. Safe (keys are salted per pid+exec generation) but a known frame-retention limiter under sustained pressure."
  - "rfork shares Text through palloc.hash instead of copying, and a fresh text copy is inserted into the cache; RFMEM-shared Data pages are marked dirty so the evictor can never take them."

patterns-established:
  - "Rank-safe palloc wiring: PAGE_CACHE (20) -> PROC_TABLE (50) -> release -> request_shootdown -> PAGE_ALLOC (70); is_pinned may ascend 20 -> 70 but PAGE_ALLOC is never held across the evictor"
  - "Reclaim-and-retry at the fault layer: alloc_clean runs the evictor, and a persistent OutOfSpace kills one process (unresolvable) instead of panicking the kernel"
  - "Every user buffer entering a sleeping port/ path carries a PinGuard, so the concurrent evictor observes pin_count > 0 and skips it"

requirements-completed: [MEM-07, MEM-10, MEM-15, MEM-16]

coverage:
  - id: D1
    description: "aarch64 PAGE_ALLOC runs on portable Roaring pools with High-Water refill, optional palloc_max_frames, a reusable zero alias and the palloc boot marker"
    requirement: MEM-07
    verification:
      - kind: e2e
        ref: "target/gates/smoke-r9/smoke-r9.log: 'palloc: managed_frames=250919 low_water=12545'"
        status: pass
      - kind: integration
        ref: "cargo xtask clippy --arch aarch64 --config virtshell && --config virt; cargo xtask check"
        status: pass
    human_judgment: false
  - id: D2
    description: "User text maps strictly read-only and is shared through palloc.hash across faulting/forked processes; writes to text kill the process"
    requirement: MEM-15
    verification:
      - kind: e2e
        ref: "smoke-r9.log: 107 'fault: text page-in' lines, zero 'user fault'/Data Abort, full ls/ps/cat and restart sweep"
        status: pass
      - kind: unit
        ref: "cargo test -p port (palloc/pagecache pins, unchanged)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Initialised data maps AllRo and is promoted to AllRw + dirty on the first EL0/EL1 write, so untouched pages are evictable and written pages are not"
    requirement: MEM-15
    verification:
      - kind: e2e
        ref: "smoke-r9.log / tier1-r9.log: all typed reads/writes and the 44-program tier-1 sweep pass with zero EL1 permission aborts"
        status: pass
    human_judgment: true
    rationale: "Smoke/tier-1 prove the write-promotion path functionally, but no dedicated counter asserts that a promoted page left the droppable queue; the pressure workload (05-21) will assert dropped_data/pinned_skipped."
  - id: D4
    description: "UserSlice/UserSliceMut hold an RAII PinGuard over every resolved frame across the eight sleeping port/ paths"
    requirement: MEM-10
    verification:
      - kind: e2e
        ref: "smoke-r9.log and tier1-r9.log: pread/pwrite/pipe/cons/mnt-heavy sweeps complete with zero pin leaks or rank panics"
        status: pass
      - kind: integration
        ref: "cargo xtask clippy --arch aarch64 --config virtshell; cargo test -p xtask --lib lintcontract (13/13)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Two-CPU HVF and TCG boots pass twice consecutively with NEON setops, shared text, pinning and zero EL1 permission faults"
    requirement: MEM-16
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-smoke (HVF x2, R9_ACCEL=tcg x2) and nix run .#boot-r9-tier1 (HVF): all PASS"
        status: pass
    human_judgment: false

duration: 65min
completed: 2026-11-27
status: complete
---

# Phase 5 Plan 18: aarch64 Roaring Pools, Read-Only Text, Write-Protect Dirty Tracking and User-Frame Pinning Summary

**aarch64 now runs on the portable Roaring frame pools with shared read-only text through `palloc.hash`, write-protect dirty tracking for initialised data, a live evictor honoured by RAII `PinGuard` on every sleeping user buffer, and a green `-smp 2` HVF/TCG smoke plus tier-1 sweep.**

## Performance

- **Duration:** ~65 min
- **Started:** 2026-11-26 (worktree execution)
- **Completed:** 2026-11-27T00:04Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- `aarch64/src/pagealloc.rs` replaces the bitmap allocator with `port::palloc::PageAlloc`: the DTB memory map seeds the Roaring pools, `palloc_max_frames=<N>` is parsed only on the `virt` dev_exec profiles (defaulting to `None` everywhere else), the High-Water Pager prefills ~15% clean, a single reusable `KZERO` alias zeroes frames by leaf retarget, `alloc_clean` runs the live evictor and retries, and the console emits `palloc: managed_frames=250919 low_water=12545` once.
- `aarch64/src/vm.rs` adds the strictly read-only `Entry::ro_user_text()` (`AllRo`, `uxn=false`) and `Entry::ro_user_data()` (`AllRo`, `uxn=true`) constructors plus leaf-walk helpers (`leaf_entry_mut`, `retarget_kernel_leaf`, `set_user_leaf`, `clear_leaf_pte`) and `tlbi vae1is`.
- `aarch64/src/fault.rs` makes text faults look up `palloc.hash`, fill through the kernel `KZERO + pa` alias, deduplicate a concurrent insert by freeing the losing frame, and map every process read-only; data first-touch maps `ro_user_data()` and is registered as `UntouchedData`; a write fault promotes the page to `AllRw`, marks it dirty and never drops it; page-in reclaims-and-retries instead of panicking; the evictor walks `PAGE_CACHE (20) -> PROC_TABLE (50) -> request_shootdown -> PAGE_ALLOC (70)`.
- `aarch64/src/proc.rs`/`exec.rs` give each `ByteSource` a stable `image_id`, share text across `rfork` instead of copying, register untouched data under per-proc salted keys, tear down cached frames without double-freeing them on exec/exit, and add `evict_user_frame` for the evictor.
- `aarch64/src/usermem.rs` adds the RAII `PinGuard` (inline `[u32; 4]`, `Vec` overflow, rollback on mid-span failure) to `UserSlice`/`UserSliceMut`; `UserSliceMut::new` promotes clean data pages for EL1 writes; the eight sleeping `port/` paths keep their guards across `block()`/`sched()`.

## Task Commits

Each task was committed atomically (Task 3 was verification-only and needed no code change):

1. **Task 1: aarch64 RO text, palloc pools, palloc.hash and live evictor** — `6b94e34` (feat)
2. **Task 2: RAII PinGuard, EL1 write promotion and sleeping-path audit** — `4121001` (feat)
3. **Task 3: -smp 2 HVF/TCG smoke + HVF tier-1** — no code change (all gates green)

## Files Created/Modified

- `aarch64/src/vm.rs` — RO user PTE constructors and the leaf/TLB helpers.
- `aarch64/src/pagealloc.rs` — portable PageAlloc wiring, bootargs, High-Water refill, zero alias, live evictor callback, page-cache handle, boot marker.
- `aarch64/src/fault.rs` — shared RO text, RO untouched data, write promotion, retrying page-in, `evict_re_readable_pages`, writable `ensure_user_resident`.
- `aarch64/src/exec.rs` — `ByteSource::new`/image id, exec generation, eager RO data registration as untouched.
- `aarch64/src/proc.rs` — `exec_gen`, rmap slot, text/data cache keys, `evict_user_frame`, cached-aware free, shared-text rfork, RFMEM dirty marking.
- `aarch64/src/usermem.rs` — `PinGuard` and constructor changes.
- `aarch64/src/syscall.rs` — the MEM-10 sleeping-path audit note.
- `UNSAFE-AUDIT.md` — Plan 05-18 aarch64 delta.

## Decisions Made

- **Boot-published handles.** `PageAlloc` and `PageCache` are `AtomicPtr` handles created with `Box::into_raw` at boot; their Roaring/`Vec` state cannot be a const static.
- **Eager RO data + cache registration.** Data stays eagerly resident (preserving the cold-syscall-buffer fix) but read-only and droppable until written; eviction plus re-fault gives demand behaviour afterwards.
- **Evictor locates root tables through `PROC_TABLE` + `mapping_log`,** because the pagecache `proc_slot` does not name a root table in the current proc model; the rmap is still maintained for cache bookkeeping.
- **Image identity** is the paqfs `qid` mixed with the mapped image base, or an FNV-1a content hash for devmnt binaries.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `PageAlloc` cannot be a `static` initializer**
- **Found during:** Task 1 (first `cargo xtask check`)
- **Issue:** `E0015`: `PageAlloc::new()` is not const because its Roaring/`Vec` pools allocate.
- **Fix:** Published a `Box::into_raw(Box::new(PageAlloc::new()))` at boot through an `AtomicPtr`, mirroring the page-cache handle.
- **Files modified:** `aarch64/src/pagealloc.rs`
- **Committed in:** `6b94e34`

**2. [Rule 3 - Blocking] Cache keys needed proc/image identity fields**
- **Found during:** Task 1 (shared-text and untouched-data wiring)
- **Issue:** `ByteSource` had no executable identity and `Proc` had no exec generation, so `palloc.hash` could not key text sharing or salt private data pages.
- **Fix:** Added `ByteSource::new` + `image_id` and `Proc::exec_gen`; derived text/data keys in `proc.rs` and reused them in fault/exec/rfork.
- **Files modified:** `aarch64/src/proc.rs`, `aarch64/src/exec.rs`
- **Committed in:** `6b94e34`

**3. [Rule 3 - Commit shape] Task 1 carries the `ensure_user_resident`/promotion foundation**
- **Found during:** Task 2 staging
- **Issue:** `fault.rs` is shared by both tasks; the writable residency/promotion path is the producer `UserSliceMut` consumes.
- **Fix:** Task 1's commit includes those `fault.rs` changes; Task 2's commit carries `usermem.rs`, the constructor wiring and the syscall audit note.
- **Files modified:** —
- **Committed in:** `6b94e34`, `4121001`

---

**Total deviations:** 3 auto-fixed (3 blocking/shape).
**Impact on plan:** No scope creep; each change is required to publish the allocator, key the cache correctly, and keep the two shared-file tasks independently reviewable.

## Known Limiter (deferred)

Written initialised-data pages remain in `palloc.hash` after `mark_data_dirty` removes them from the droppable queue, and `free_user_mappings` deliberately does not return a cached frame to the allocator. Keys are salted per pid + exec generation, so a stale entry is unreachable, but the frames are only reclaimed by a future explicit cache-removal path. The 1 GiB `-smp 2` smoke and tier-1 runs are unaffected; the small-memory pressure workload (05-21) is the place to add `PageCache::forget`/eviction of dirty entries if the retention proves material.

## Issues Encountered

- None in the final runs. The initial `PageAlloc` static failed to compile (deviation 1); after the boot-published handle the non-virt and both virt configs clippy cleanly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- MEM-07/MEM-10/MEM-15/MEM-16 are live on `aarch64` `-smp 2`: HVF smoke passed twice, TCG smoke passed twice, and the HVF tier-1 sweep passed (44 OK, 23 known SKIP, zero FAIL/panic).
- `cargo xtask check`, aarch64 clippy (`virtshell` and `virt`) and `cargo test -p xtask --lib lintcontract` (13/13) are green; `setops: simd=neon selftest=ok` appears in both HVF and TCG captures.
- Plan 05-19 can mirror the same wiring onto x86_64; Plan 05-21 should assert `dropped_data > 0` / `pinned_skipped > 0` and decide whether to add dirty-cache eviction.

## Self-Check: PASSED

- Created files exist: all eight modified paths are present in the worktree.
- Commits are ancestors of HEAD: `6b94e34`, `4121001`.
- Both clippy legs, `cargo xtask check`, the five gate runs and lintcontract passed.
