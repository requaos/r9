---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 19
subsystem: memory-paging
tags: [x86_64, demand-paging, page-fault, palloc, pagecache, ro-text, pte-d, evictor, pin-guard, mem-07, mem-10, mem-14, mem-15, mem-16]

# Dependency graph
requires:
  - phase: 05-16
    provides: "port::palloc PageAlloc (clean/dirty/active/pinned, High-Water refill, rank-safe alloc_clean), port::pagecache PageCache (palloc.hash, rmap, mark_data_dirty, select_eviction_victim) and #c/sysstat counters"
  - phase: 05-18
    provides: "the proven aarch64 demand-pager/RO-text/write-protect/PinGuard contract this plan mirrors"
  - phase: 05-24
    provides: "the x86_64 kstack-ownership and PROC_TABLE/rank discipline the evictor rides"
provides:
  - "x86_64/src/fault.rs: ring-3 #PF demand pager, shared RO text through palloc.hash, PTE_D=0 data, zeroed bss/stack/tos, ensure_user_resident/ensure_stack_resident"
  - "x86_64/src/exec.rs: eager text/data/bss loading removed; image_id + exec_gen recorded"
  - "x86_64/src/trap.rs: ring-3 vector-14 #PF routed to crate::proc::fault::user_fault"
  - "x86_64/src/pagealloc.rs: PAGE_ALLOC on port::palloc with palloc_max_frames and the one-shot palloc boot marker"
  - "x86_64/src/proc.rs: PTE_D atomic-swap evictor, cache-aware free_user_mappings, image/exec cache keys"
  - "x86_64/src/usermem.rs: RAII PinGuard on UserSlice/UserSliceMut and demand-paging UserString"
affects: [05-21 (pressure workload), 05-22 (pin-guard follow-ups)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan estimate (55000, confidence: low).
actuals:
  tokens: 20163
  tasks: 3
  commits: 4
  plan_head_before: b61d9bff563613c594010dbd8fd6077e70e99cf7
  plan_head_after: d799ca3ab9fb0a3cb23d91313c2233d28e159998

tech-stack:
  added: []
  patterns:
    - "Ring-3 #PF demand paging: vector 14 + CR2 decode, fill from the proc byte source, install leaf, retry the faulting instruction"
    - "W^X text: PTE_P|PTE_US RO leaves shared through palloc.hash; data leaves RW|NX with PTE_D explicitly clear"
    - "Race-free dirty tracking: atomic leaf swap + immediate PTE_D inspect under PROC_TABLE, restore-on-dirty with no shootdown"
    - "Boot-published PageAlloc/PageCache AtomicPtr handles (Box::into_raw) because Roaring/Vec state is not const"
    - "RAII PinGuard with an inline [u32; 4] fast path and Vec overflow"

key-files:
  created:
    - x86_64/src/fault.rs
  modified:
    - x86_64/src/exec.rs
    - x86_64/src/trap.rs
    - x86_64/src/proc.rs
    - x86_64/src/pagealloc.rs
    - x86_64/src/usermem.rs
    - x86_64/src/syscall.rs
    - port/src/pagecache.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "Full demand loading: exec maps no text/data/bss page; the cold-syscall-buffer guarantee is preserved by making UserString force residency instead of relying on eager data mappings."
  - "Text is mapped strictly read-only (PTE_P|PTE_US) and shared through palloc.hash; rfork still copies text privately but maps it read-only."
  - "The evictor holds PROC_TABLE across the atomic leaf swap and the PTE_D check; the cache key is already removed under PAGE_CACHE by select_eviction_victim, so the cache-side race is closed without re-taking the cache lock."
  - "pagealloc zeroes frames through the boot tables' shared KZERO alias (the 1 GiB q35 window), so no per-frame alias setup is needed."

patterns-established:
  - "Rank order PAGE_CACHE (20) -> PROC_TABLE (50) -> release -> LAPIC 0xF0 shootdown -> PAGE_ALLOC (70)"
  - "Reclaim-and-retry at the fault layer: alloc failure kills one process, never panics the kernel"
  - "Every sleeping user buffer carries a PinGuard, so the live evictor observes pin_count > 0"

requirements-completed: [MEM-07, MEM-10, MEM-14, MEM-15, MEM-16]

coverage:
  - id: D1
    description: "x86_64 exec no longer maps text/data/bss eagerly; ring-3 #PF demand-loads every image page, maps text read-only and shares it through palloc.hash"
    requirement: MEM-14
    verification:
      - kind: integration
        ref: "cargo xtask build --arch x86-64 --config q35cpu; cargo xtask clippy --arch x86-64 --config q35cpu; cargo xtask check"
        status: pass
      - kind: unit
        ref: "x86_64/src/fault.rs#pf_error_code_bits_decode,kernel_address_rejection,pte_d_and_leaf_kinds"
        status: pass
    human_judgment: true
    rationale: "Compiled and host-tested, but no guest boot is part of this plan; the 05-21 pressure/stress gates are the live proof that demand-paged text/data/bss boots and runs."
  - id: D2
    description: "PAGE_ALLOC runs on the portable Roaring PageAlloc with palloc_max_frames and the palloc boot marker; the live evictor drops only untouched/shared frames and restores a concurrent PTE_D write"
    requirement: MEM-15
    verification:
      - kind: integration
        ref: "cargo xtask build --arch x86-64 --config q35cpu; cargo xtask clippy --arch x86-64 --config q35cpu"
        status: pass
    human_judgment: true
    rationale: "The PTE_D race path and the dropped_data/pinned_skipped counters are asserted by the deterministic small-memory pressure gate in 05-21, not by this plan."
  - id: D3
    description: "UserSlice/UserSliceMut carry an RAII PinGuard over every resolved frame and UserString demand-pages cold literals"
    requirement: MEM-10
    verification:
      - kind: integration
        ref: "cargo xtask check; cargo xtask clippy --arch x86-64 --config q35cpu; cargo test -p xtask lintcontract (13/13)"
        status: pass
    human_judgment: false

duration: 55min
completed: 2026-11-26
status: complete
---

# Phase 5 Plan 19: x86_64 Demand Paging, Roaring Pools, RO Shared Text, PTE_D Eviction and PinGuard Summary

**x86_64 now demand-pages every image page through the ring-3 #PF vector, shares read-only text through `palloc.hash`, tracks hardware `PTE_D` with a race-free atomic-swap evictor, runs PAGE_ALLOC on the portable Roaring pools, and pins every sleeping user buffer with an RAII `PinGuard`.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-11-26 (worktree execution)
- **Completed:** 2026-11-26T00:15Z
- **Tasks:** 3
- **Files modified:** 9 (1 created)

## Accomplishments

- `x86_64/src/fault.rs` is new: decodes the `#PF` error code (`P`, `W/R`, `U/S`, `I/D`), rejects `cr2 >= KZERO` and non-ring-3 faults, classifies the VA against the proc segment map, and pages text/data in from the byte source (text shared and RO, data `PTE_D=0`) or zeroes bss/stack/tos.  A cache miss allocates/fills outside `PAGE_CACHE` and deduplicates a concurrent insert.
- `exec.rs` no longer maps text, data or bss eagerly.  It records `text_source` (with a derived `image_id`) and bumps `exec_gen`; only the seed stack/Tos pages stay eager.
- `trap.rs` routes ring-3 vector 14 to `crate::proc::fault::user_fault(trap_frame, cr2)` and retries via the iretq tail.
- `pagealloc.rs` replaces the bitmap allocator with `port::palloc::PageAlloc` (boot-published `AtomicPtr`, `palloc_max_frames=<N>` bootarg, High-Water refill, ZERO through the shared KZERO alias) and creates the `palloc.hash` `PageCache`; `usage_bytes()` emits `palloc: managed_frames=<N> low_water=<L>` once after the console is live.
- `proc.rs` adds the live evictor: `AtomicU64::swap(0, AcqRel)` on the leaf PTE plus immediate `old_pte & PTE_D` inspection under `PROC_TABLE`; a dirty race restores the PTE and marks the page dirty with no shootdown, a clean frame is unmapped, the TLB is shot down (LAPIC 0xF0) and the frame returns to PAGE_ALLOC.  `free_user_mappings` is cache-aware and RFMEM-shared data is marked dirty.
- `usermem.rs` adds the RAII `PinGuard` (inline `[u32; 4]`, Vec overflow) to `UserSlice`/`UserSliceMut`, marks kernel-written Data pages dirty, and makes `UserString` force page residency so a cold data literal never faults the kernel.

## Task Commits

Each task was committed atomically:

1. **Task 1: demand pager, RO shared text, PTE_D data, palloc wiring** — `ae4ae8a` (feat)
2. **Task 2: PTE_D atomic-swap evictor and RAII PinGuard** — `1f69086` (feat)
3. **Task 3: #PF/PTE_D host tests + full verify** — `fb25c9b` (test)
4. **Follow-up: hold PAGE_CACHE across the evictor PTE swap** — `d799ca3` (fix)

## Files Created/Modified

- `x86_64/src/fault.rs` — ring-3 `#PF` demand pager, shared RO text, data/zero page-in, residency helpers, unit tests.
- `x86_64/src/exec.rs` — eager image mapping removed; `image_id`/`exec_gen` recorded.
- `x86_64/src/trap.rs` — ring-3 vector-14 dispatch.
- `x86_64/src/pagealloc.rs` — portable PageAlloc + PageCache + boot marker.
- `x86_64/src/proc.rs` — cache keys, cache-aware teardown, PTE_D evictor.
- `x86_64/src/usermem.rs` — PinGuard, writable residency, demand-paging UserString.
- `x86_64/src/syscall.rs` — eight-sleeping-path audit note; brk comment.
- `port/src/pagecache.rs` — `with_lock` for the evictor's atomic PTE swap.
- `UNSAFE-AUDIT.md` — Task 1 (net 0) and Task 2 (+3 blocks) deltas.

## Decisions Made

- **Full demand loading over aarch64's eager-data compromise.** The plan requires no eager text/data/bss mapping, so `UserString::copy_from` now demands residency per page (taking `&mut`), keeping the cold-literal guarantee without eager data.
- **Text remains shared through `palloc.hash`; rfork copies text privately but read-only.** Forked text is W^X-safe; cross-process sharing applies to demand-faulted text.
- **Cache and proc table are both held across the leaf swap.** `PageCache::with_lock` keeps `PAGE_CACHE` (Rank 20) held while `evict_user_frame` takes `PROC_TABLE` (50), swaps the leaf and inspects `PTE_D`; the candidate's key was already removed by `select_eviction_victim`.  This closes the concurrent-page-in race the plan calls out (T-05-34).
- **KZERO identity alias for zeroing.** Every q35 profile is 1 GiB, inside the boot KZERO window, so no per-frame alias map is needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `UserString` needed residency forcing under full demand loading**
- **Found during:** Task 2
- **Issue:** `UserString::copy_from` was validate-only (residency "by construction" from eager data mapping); with eager data removed, a cold literal page passed to a syscall would fault the kernel.
- **Fix:** `copy_from` now takes `&mut impl UserAddressSpace` and forces the page resident before each byte read; callers (`copy_user_str`, `copy_exec_args`, `sys_mount`) updated.
- **Files modified:** `x86_64/src/usermem.rs`, `x86_64/src/syscall.rs`
- **Commit:** `1f69086`

**2. [Rule 3 - Blocking] `sys_unmount` borrowed namespace chans across the new `&mut` copy**
- **Found during:** Task 2 (`cargo xtask check`)
- **Issue:** E0502 after the copy became `&mut`: `slash`/`dot` were borrowed before `copy_user_str`.
- **Fix:** copy both strings first, then borrow the namespace chans.
- **Files modified:** `x86_64/src/syscall.rs`
- **Commit:** `1f69086`

**3. [Rule 3 - Blocking] Atomic store ordering**
- **Found during:** Task 2 (`cargo xtask check`)
- **Issue:** `AtomicU64::store(..., AcqRel)` is rejected (`invalid_atomic_ordering`).
- **Fix:** restore with `Release`.
- **Files modified:** `x86_64/src/proc.rs`
- **Commit:** `1f69086`

### Plan-Wording Deviations (documented, not auto-fixable)

- **rfork text is a private RO copy, not a `palloc.hash` frame.** Correct and W^X-safe; a future plan can share forked text.
- **`sys_brk` growth pages stay eagerly mapped.** The plan scoped eager removal to exec.

---

**Total deviations:** 3 auto-fixed (3 blocking), 2 plan-wording deviations documented.
**Impact on plan:** All changes are required for correctness under full demand loading; no scope creep beyond the plan's memory architecture.

## Issues Encountered

- The x86_64 host unit tests in `fault.rs`/`usermem.rs` only run on an x86_64 host; this worktree executes on an aarch64 Darwin host, so `cargo xtask test` ran the host-arch legs. The x86 target build and both clippy legs compiled the production paths; CI's x86_64 leg runs the `#[cfg(test)]` units.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `cargo xtask check`, `cargo xtask build --arch x86-64 --config q35cpu`, both arch clippy legs and `cargo xtask test` (incl. lintcontract 13/13) are green.
- Both architectures now share the demand-paged, RO-text, `palloc.hash`, `PTE_D`-evicting, pinned-buffer model.  Plan 05-21 should assert `dropped_data > 0` / `pinned_skipped > 0` with the `palloc_max_frames=2048` pressure boot and decide whether dirty-cache retention needs a forget path.

## Self-Check: PASSED

- Created file `x86_64/src/fault.rs` exists.
- Commits `ae4ae8a`, `1f69086`, `fb25c9b`, `d799ca3` are ancestors of HEAD.
- Build, clippy (x86 q35cpu + aarch64 virtshell) and `cargo xtask test` exited 0.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*
