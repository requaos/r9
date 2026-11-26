---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 07
subsystem: port
tags: [smp, mcslock, irqspinlock, lock-rank, watchdog, tlb-shootdown, globalalloc, quickfit, smp-02, smp-03, smp-07, smp-12, tdd, forbid-unsafe]

# Dependency graph
requires:
  - phase: 05-06
    provides: slab-backed FidTable (FidShard meta is the rank-40 candidate) and the green owner-stop-1 baseline this builds on
provides:
  - port/src/smp.rs (#[forbid(unsafe_code)]): the SmpArch per-CPU hook boundary (register_arch/current_cpu_id/num_online_cpus/irq_save_disable/irq_restore/send_resched_ipi/send_shootdown_ipi/local_tlb_flush/ticks/prepare_block) with safe single-CPU defaults and thread_local host-test overrides; the documented LockRank table (10..100); the #[cfg(debug_assertions)] per-cpu rank stack + progress watchdog with zero-cost release stubs; the lock-free ShootdownCoordinator (request_shootdown/poll_shootdown_if_pending/service_shootdown_ipi)
  - port/src/mcslock.rs: OnceStatic publish-once boot cell; MCSLock::try_lock; Lock::new_ranked + rank/name/owner_cpu metadata with debug rank/watchdog hooks; Lock::try_lock; IrqSpinLock/IrqLockGuard (IRQ save/disable + restore on Drop, try_lock restores on miss, shootdown poll inside both spin loops)
  - port/src/globalalloc.rs: GlobalQuickAlloc guarded by IrqSpinLock at LockRank::HeapAlloc (80) — the single-CPU AtomicPtr::swap panic hazard is gone; arena_stats/split_census serialized under the lock
  - port/src/allocator.rs: QuickFit::split_census_snapshot (non-allocating 256-class fixed-array census; Vec copies after lock release)
  - aarch64/src/allocator.rs + x86_64/src/allocator.rs: GlobalQuickAlloc::new construction and heap_stats/heap_census delegation with the unlocked diagnostic reads removed (x86_64's static mut HEAP/ALLOC became SyncUnsafeCell)
  - UNSAFE-AUDIT.md: updated port/mcslock/globalalloc and arch allocator ledger (D-03)
affects: [05-08 port::sched run queues (rank 60 + try_lock steal), 05-09 aarch64 SmpArch registration (TPIDR_EL1/GIC SGI 1), 05-11 x86_64 SmpArch registration (GS base/LAPIC 0xF0), 05-12+ SMP-03 lock-ranking migration of existing Lock callers, Owner Stop 2]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (55000, confidence: low).
actuals:
  tokens: 26344
  tasks: 3
  commits: 5
  plan_head_before: 4ff046eebd922e6ba802d33e23d7af583c4dd025
  plan_head_after: c15362fe2c89d0e76095dd2d9e686d804f5528d4

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SmpArch vtable of plain fn pointers behind a publish-once OnceStatic cell (lock-free read, safe from IRQ-masked lock machinery); single-CPU defaults before registration; host tests override cpu-id/ticks/IRQ/online-mask per thread (thread_local) so parallel cargo tests never share simulated cpus"
    - "Per-cpu debug state: static [CpuDebug; MAX_CPUS] indexed by cpu id in kernel builds, per-thread leaked instances in host tests (the fpu.rs pattern); watchdog check_all walks the dense online range on kernel and the caller's own simulated cpu under test"
    - "Lock names travel as fixed-capacity atomic byte arrays, never fat pointers: &str is rebuilt with the safe core::str::from_utf8, keeping port/src/smp.rs #[forbid(unsafe_code)]"
    - "Shootdown coordinator: claim (initiator-side test-and-set with poll in its spin) + global seq/VA + per-cpu ack epochs; the claim holds the VA stable until every online cpu acks, and both spin loops service pending requests inline (Pitfall 1 deadlock class eliminated)"
    - "Rank metadata on Lock is Option<LockRank>: Lock::new stays unranked (equal-rank nesting panics make any real default rank wrong for legacy callers); the SMP-03 migration ranks callers in later plans"

key-files:
  created:
    - port/src/smp.rs
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-07-RED-task1.json
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-07-RED-task2.json
  modified:
    - port/src/mcslock.rs
    - port/src/lib.rs
    - port/src/globalalloc.rs
    - port/src/allocator.rs
    - aarch64/src/allocator.rs
    - x86_64/src/allocator.rs
    - riscv64/src/allocator.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "register_arch's one pointer bridge lives in the mcslock floor module as OnceStatic<T>: smp.rs must stay #[forbid(unsafe_code)] (locked must-have), and a lock-free read path is mandatory because the hooks are called from the lock machinery itself (a Lock-guarded cell would recurse/deadlock)"
  - "Lock::new keeps an unranked default (rank: Option<LockRank> = None): the equal-rank nesting panic makes any real default rank break legacy callers; IrqSpinLock is always ranked; SMP-03's caller-by-caller ranking lands in later plans"
  - "Shootdown requests serialize on a claim word (not just a seq bump): without it a second initiator could overwrite the target VA before the first generation finished flushing; the claim spin polls so IRQ-masked contenders never wedge"
  - "The watchdog trips on (waiting_since set) AND (elapsed > threshold) AND (heartbeat unmoved since the wait began): idle cpus (no wait) and active cpus (heartbeat moved) never trip, and the panic names cpu id, elapsed ticks, lock name and owner cpu"
  - "split_census_snapshot tallies into 256-class fixed stack arrays under the heap lock with an overflow fold ((usize::MAX, n) marker); the Vec copies happen after the guard drops, so the census never allocates under its own lock"

patterns-established:
  - "RED-for-a-refactor shape carried forward from 05-05/05-06: full new implementation committed as test() scaffolding with exactly the planned guards omitted, machine-classified RED evidence (RED_EVIDENCE_OK), GREEN adds the guards"
  - "cfg_attr(not(debug_assertions), allow(dead_code)) on debug-only lock metadata fields keeps release builds at zero added warnings while debug/gate builds run the rank/watchdog hooks"
  - "Multi-threaded host proofs of IRQ-masked deadlock freedom: Barrier-synchronized holder/spinner threads with disjoint cpu-id sets and thread-local online masks, asserting the spinner's poll serviced the initiator's barrier"

requirements-completed: [SMP-02, SMP-03, SMP-07, SMP-12]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "port/src/smp.rs (#[forbid(unsafe_code)]) provides the SmpArch hook boundary, the documented LockRank table (10..100), the #[cfg(debug_assertions)] rank stack and progress watchdog, and the lock-free ShootdownCoordinator; strictly increasing ranks succeed, equal/lower rank panics naming both locks, the watchdog names stuck cpu/ticks/lock/owner and never trips idle or active cpus (SMP-02, SMP-07, SMP-12, D-25, D-26)"
    requirement: SMP-02
    verification:
      - kind: unit
        ref: "cargo test -p port smp (6/6): lock_rank_valid_order_succeeds, lock_rank_inversion_panics_naming_both_locks, watchdog_reports_stuck_cpu_and_lock, smp_shootdown_poll_flushes_target_va_and_acks, smp_shootdown_request_completes_when_remote_polls, register_arch_publishes_once"
        status: pass
      - kind: automated
        ref: "cargo check -p port --release exits 0 with rank/watchdog compiled out (0 rank_watch/watchdog symbols in libport.rlib) and 0 unexpected_cfgs warnings"
        status: pass
    human_judgment: false
  - id: D2
    description: "port/src/mcslock.rs exports IrqSpinLock/IrqLockGuard/Lock::new_ranked/Lock::try_lock/IrqSpinLock::try_lock with IRQ save/restore on lock/Drop, IRQ restore + no rank leak on try miss, rank/watchdog hooks in debug, and poll_shootdown_if_pending inside BOTH MCSLock and IrqSpinLock spin loops (SMP-07, SMP-12)"
    requirement: SMP-07
    verification:
      - kind: unit
        ref: "cargo test -p port mcslock (5/5) + irq_spinlock filter (3/3): irq_spinlock_nested_save_restore, irq_spinlock_try_lock_restores_irq_on_miss, lock_try_lock_returns_none_when_held, lock_rank_hooks_track_ranked_lock_acquisition, irq_spinlock_shootdown_poll_prevents_deadlock (multi-threaded)"
        status: pass
      - kind: automated
        ref: "cargo test -p xtask lintcontract 13/13; UNSAFE-AUDIT.md matches the mcslock site count"
        status: pass
    human_judgment: false
  - id: D3
    description: "GlobalQuickAlloc guarded by IrqSpinLock at LockRank::HeapAlloc (80) with zero single-CPU AtomicPtr::swap panic paths; heap_stats/heap_census in both arch crates delegate without allocating under the heap lock; x86_64 static mut HEAP/ALLOC removed (SMP-03, D-22)"
    requirement: SMP-03
    verification:
      - kind: automated
        ref: "grep -v '^[[:space:]]*//' port/src/globalalloc.rs | grep -c IrqSpinLock = 3; cargo xtask check rc 0; cargo xtask clippy --arch aarch64 --config virtshell rc 0; cargo xtask clippy --arch x86-64 --config q35cpu rc 0; cargo xtask test rc 0 (56 ok legs)"
        status: pass
    human_judgment: false

# Metrics
duration: 27min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 07: Portable SMP Core — SmpArch, LockRank, Watchdog, Shootdown, IrqSpinLock, SMP-Safe Heap Summary

**`port/src/smp.rs` (`#[forbid(unsafe_code)]`) now carries the portable SMP substrate: the `SmpArch` hook boundary with single-CPU defaults and a publish-once `OnceStatic` registration cell, the documented `LockRank` table (DevTop 10 … Console 100) with a `#[cfg(debug_assertions)]` per-cpu rank stack that panics naming BOTH locks on equal-or-lower acquisition, a per-cpu progress watchdog whose panic names stuck cpu/elapsed ticks/lock name/owner cpu and never trips idle or active cpus, and the lock-free `ShootdownCoordinator` whose poll runs inside BOTH `MCSLock` and `IrqSpinLock` spin loops — plus `IrqSpinLock`/`try_lock`/rank hooks in `port/src/mcslock.rs` and an `IrqSpinLock(HeapAlloc)`-guarded `GlobalQuickAlloc` with non-allocating census on both arch allocators. Release builds compile the rank/watchdog out completely with zero `unexpected_cfgs` warnings; host tests prove every behavior multi-threaded; the full gate suite (xtask check, both arch clippy legs, xtask test's 56 legs) is green.**

## Performance

- **Duration:** 27 min
- **Started:** 2026-11-26T06:25:25Z
- **Completed:** 2026-11-26T06:53:03Z
- **Tasks:** 3 of 3 executed (Tasks 1-2 TDD RED→GREEN; Task 3 auto)
- **Files modified:** 11 (1 created: port/src/smp.rs; plus 2 RED evidence records)

## Accomplishments

- **SMP-02 — the portable per-CPU boundary.** `port/src/smp.rs` defines `MAX_CPUS = 64` (never hard-coding 2, D-23) and the `SmpArch` vtable (`current_cpu_id`, `num_online_cpus`, `irq_save_disable`, `irq_restore`, `send_resched_ipi`, `send_shootdown_ipi`, `local_tlb_flush`, `ticks`, `prepare_block`) with safe single-CPU defaults before `register_arch` runs; host tests override cpu-id/ticks/IRQ/online-mask per thread so the whole core runs multi-threaded under `cargo test -p port` with zero arch dependencies (D-31's host-first requirement).
- **SMP-12 — documented ranks with debug teeth, zero release cost.** `#[repr(u16)] LockRank` carries the single documented table; `rank_before_acquire` panics naming both held and requested locks with their ranks on `rank <= top`, `rank_after_release` clears LIFO or non-LIFO cleanly (releasing a lock not held panics by name), and `watchdog_check_all(threshold)` panics `WATCHDOG: cpu <id> stuck for <ticks> ticks waiting on lock "<name>" (held by cpu <owner>)` — while idle cpus (no wait) and active cpus (heartbeat moved during the wait) never trip. `#[cfg(not(debug_assertions))]` every entry point is an `#[inline(always)]` no-op and the per-cpu state does not exist (verified: zero rank/watchdog symbols in the release rlib, zero `unexpected_cfgs`).
- **SMP-07 — IRQ-safe locking and deadlock-proof shootdowns.** `IrqSpinLock` masks IRQs before acquiring and restores the saved state on guard Drop (nested save/restore pinned); `try_lock` on both `Lock` and `IrqSpinLock` fails fast with IRQ state restored and no rank leaked; both spin loops call `poll_shootdown_if_pending()` on every iteration, and the multi-threaded test proves a spinning cpu services the holder's `request_shootdown` barrier with zero deadlock (Pitfall 1). The coordinator's claim word holds the target VA stable until every online cpu acks the generation it belongs to.
- **SMP-03 — the global heap is SMP-safe.** `GlobalQuickAlloc` is `IrqSpinLock<*mut QuickFit>` at `LockRank::HeapAlloc` (80) instead of the single-CPU `AtomicPtr::swap` that panicked on a 2-cpu race; `arena_stats`/`split_census` run under the lock, the census tally uses the new non-allocating 256-class `split_census_snapshot` (Vec copies after the guard drops — no re-entrant self-deadlock), both arch allocators delegate with their unlocked diagnostic reads removed, and x86_64's `static mut HEAP`/`ALLOC` became `SyncUnsafeCell`.
- **Audit lockstep (D-03).** `UNSAFE-AUDIT.md` moved in the same commits: port 22 → 32 (OnceStatic + IrqSpinLock sites), aarch64 186 → 184, x86_64 158 → 156 (diagnostic reads delegated), SAFETY 384 → 390.

## Task Commits

Tasks 1 and 2 are `tdd="true"` (RED before GREEN, machine-classified evidence); Task 3 is `type="auto"`:

1. **Task 1, RED: failing SMP-12 rank/watchdog guards over smp.rs scaffolding (SMP-02, SMP-07)** — `0f10a2e` (test) — 4 passed, 2 failed on the planned panic assertions; RED evidence `05-07-RED-task1.json` classified `RED_EVIDENCE_OK` (`target_test_failed`)
2. **Task 1, GREEN: SMP-12 lock-rank and watchdog panics in port/src/smp.rs** — `508bb1b` (feat) — 6/6 smp tests, 174/174 port lib
3. **Task 2, RED: failing IrqSpinLock IRQ/rank guards over MCS scaffolding (SMP-07, SMP-12)** — `2c2492e` (test) — 2 passed, 3 failed on the planned IRQ/rank assertions; RED evidence `05-07-RED-task2.json` classified `RED_EVIDENCE_OK` (`target_test_failed`)
4. **Task 2, GREEN: IrqSpinLock IRQ wiring and rank/watchdog hooks in mcslock** — `ac52192` (feat) — 5/5 mcslock tests, 179/179 port lib, release clean
5. **Task 3: SMP-safe GlobalQuickAlloc under IrqSpinLock(HeapAlloc) (SMP-03)** — `c15362f` (feat) — xtask check + both arch clippy legs + xtask test (56 legs) all exit 0

**Plan metadata:** the SUMMARY + REQUIREMENTS.md commit (docs: complete plan), committed atomically with this file.

## Files Created/Modified

- `port/src/smp.rs` — **created** (~700 lines): module docs (the three boundaries), `MAX_CPUS`, `SmpArch` + defaults + `register_arch` + `arch()`, the public hook delegators, the `LockRank` table doc + enum, the `rank_watch` debug module (`CpuDebug`/`RankSlot`, acquire/release/heartbeat/begin_wait/end_wait/check_one/check_all), the zero-cost release stubs, the `ShootdownCoordinator` statics + `poll_shootdown_if_pending`/`service_shootdown_ipi`/`request_shootdown`, the `test_hooks` thread-local module, and the six host tests
- `port/src/mcslock.rs` — `OnceStatic<T>` publish-once cell; `MCSLock::lock`'s spin now polls shootdowns; `MCSLock::try_lock`; `Lock` gains `name`/`rank`/`owner_cpu` metadata, `new_ranked`, `try_lock`, and debug rank/watchdog hooks; `LockGuard` pops ranks on Drop; `IrqSpinLock`/`IrqLockGuard` (IRQ mask + rank + watchdog + shootdown-poll discipline); five host tests
- `port/src/lib.rs` — `#[forbid(unsafe_code)] pub mod smp;` mount with the module's provenance comment
- `port/src/globalalloc.rs` — `GlobalQuickAlloc` on `IrqSpinLock<*mut QuickFit>` (`LockRank::HeapAlloc`, 80), `with_allocator` under the guard, `arena_stats`/`split_census` added, the `Sync for QuickFit` proof re-justified for the lock discipline
- `port/src/allocator.rs` — `CensusSnapshot` + `CENSUS_SNAPSHOT_CAP` (256) + `QuickFit::split_census_snapshot`; `split_census` delegates
- `aarch64/src/allocator.rs` — `GlobalQuickAlloc::new` construction; `heap_stats`/`heap_census` delegate (two unlocked unsafe reads removed)
- `x86_64/src/allocator.rs` — `static mut HEAP`/`ALLOC` → `SyncUnsafeCell`; same delegation (two unlocked unsafe reads removed)
- `riscv64/src/allocator.rs` — constructor tracks the new `GlobalQuickAlloc::new` shape (`xtask check` compiles riscv64 bins); its `static mut` init stays until the Phase 7 audit horizon
- `UNSAFE-AUDIT.md` — reconciliation rows, the mcslock/globalalloc floor rows, both arch allocator per-file rows, and the 05-07 delta note
- `05-07-RED-task1.json`, `05-07-RED-task2.json` — **created** (TDD RED evidence records, committed with the RED commits)

## Decisions Made

- **`OnceStatic<T>` lives in the `mcslock` floor module.** The plan's must-have locks `port/src/smp.rs` to `#[forbid(unsafe_code)]`, but `register_arch(&'static SmpArch)` needs exactly one pointer bridge; a lock-guarded cell would recurse into the rank machinery it serves (and spin-deadlock an IRQ handler mid-hook-call on one cpu). The publish-once cell's lock-free one-atomic-load read is the only sound shape; its single dereference carries its SAFETY in the floor module per D-02.
- **`Lock::new` stays unranked.** The plan said "source-compatible with a default rank", but the SMP-12 equal-rank panic makes ANY real default rank break legacy callers that nest two device locks; `rank: Option<LockRank>` (`None` for `Lock::new`, `Some` for `new_ranked`) is the honest migration state. `IrqSpinLock` is always ranked. This is the "lock rank table's contents" discretion 05-CONTEXT.md leaves open; ranking every legacy caller is the later SMP-03 plans' work.
- **The shootdown request path takes a claim word.** A bare seq bump lets a second initiator overwrite the target VA before the first generation's flushes complete; the claim (with `poll_shootdown_if_pending` inside its spin, so IRQ-masked contenders never wedge) holds the VA stable until all online cpus ack. The service path stays lock-free.
- **The watchdog's active exemption is heartbeat-based.** A cpu trips only when `waiting_since` is set AND elapsed exceeds the threshold AND its heartbeat has not moved since the wait began — "an idle cpu with no waiting lock never trips" and an active one (progress during the wait) does not either, matching the must-haves exactly.
- **The census snapshot caps at 256 classes per side with an overflow fold.** Fixed stack arrays under the heap lock (no `Vec::push` re-entry); classes beyond the cap fold into a trailing `(usize::MAX, n)` marker appended after the sort, so a leak-hunting diagnostic loses precision loudly instead of silently.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `OnceStatic` publish-once cell added to `port/src/mcslock.rs` in Task 1**
- **Found during:** Task 1 (smp.rs construction)
- **Issue:** The plan's Task 1 file list is `port/src/smp.rs` + `port/src/lib.rs`, and its must-have locks smp.rs to `#[forbid(unsafe_code)]` — but the specified `register_arch(&'static SmpArch)` cannot store and later call a `&'static` vtable without one pointer dereference somewhere. No safe std/core type provides a no_std publish-once cell.
- **Fix:** Added a minimal `OnceStatic<T>` (one `AtomicPtr<T>`, publish panics on re-entry, lock-free `get`) to the `mcslock` floor module — the sync-primitives floor is the D-02-correct home — with its SAFETY comment and the same-commit `UNSAFE-AUDIT.md` delta.
- **Files modified:** `port/src/mcslock.rs`, `UNSAFE-AUDIT.md`
- **Verification:** `register_arch_publishes_once` passes (publish + double-publish panic); virtshell/q35cpu clippy green
- **Committed in:** `0f10a2e` (Task 1 RED commit)

**2. [Rule 3 - Blocking] `riscv64/src/allocator.rs` tracked the new `GlobalQuickAlloc::new` constructor in Task 3**
- **Found during:** Task 3 (arch allocator rewrite)
- **Issue:** The plan's file list omits riscv64, but `cargo xtask check` compiles `riscv64 --bins`, and the `GlobalQuickAlloc(pub AtomicPtr<..>)` → `GlobalQuickAlloc::new(*mut QuickFit)` shape change breaks its construction.
- **Fix:** Minimal constructor adaptation (the `static mut` init stays — riscv64 is never gate-compiled and is not lint-opted-in; its audit horizon is the Phase 7 sync, recorded in a comment); `UNSAFE-AUDIT.md`'s riscv64 row is count-only and unchanged (its 1 unsafe block remains the arena `Block` construction).
- **Files modified:** `riscv64/src/allocator.rs`
- **Verification:** `cargo xtask check` exit 0 (its riscv64 leg compiles)
- **Committed in:** `c15362f` (Task 3 commit)

**3. [Rule 1 - Bug] Task 2's try_lock test redesigned cross-cpu during GREEN**
- **Found during:** Task 2 GREEN (`irq_spinlock_try_lock_restores_irq_on_miss` failed)
- **Issue:** The RED test re-tried the same rank-80 lock on the same cpu while holding it — under GREEN's rank rules that is itself an equal-rank violation, so the panic (correctly) fired before the CAS miss the test meant to exercise.
- **Fix:** The contended try now misses against a holder thread on another simulated cpu (this cpu's rank stack empty), and the uncontended fast path runs on a second, free lock first — the IRQ-restore-on-miss and no-rank-leak assertions are unchanged from RED (and still fail on RED scaffolding for the same reason: no IRQ wiring).
- **Files modified:** `port/src/mcslock.rs` (test body)
- **Verification:** 5/5 mcslock tests pass
- **Committed in:** `ac52192` (Task 2 GREEN commit)

**4. [Rule 1 - Bug] Two compile warnings in Task 1's smp.rs fixed at Task 2 RED**
- **Found during:** Task 2 RED compile (the warnings were introduced by Task 1's commits)
- **Issue:** `MAX_CPUS` was imported but unused in test builds of the `rank_watch` module, and a `///` doc comment sat on the `std::thread_local!` macro invocation (rustdoc does not document macro invocations).
- **Fix:** `#[cfg(not(test))]` on the import; the doc comment became a `//` comment.
- **Files modified:** `port/src/smp.rs`
- **Verification:** `cargo check -p port --tests` shows zero smp.rs warnings
- **Committed in:** `2c2492e` (Task 2 RED commit)

**5. [Rule 1 - Bug] SAFETY comments moved to immediately precede each guard data-borrow block**
- **Found during:** Task 3 verification (`cargo xtask clippy --arch aarch64 --config virtshell` failed with 4 `undocumented_unsafe_blocks` errors)
- **Issue:** My multi-line `LockGuard`/`IrqLockGuard` struct expressions put the `data: unsafe { ... }` block on its own line with the SAFETY comment above the whole expression — clippy requires the comment immediately preceding EACH unsafe block (the pre-existing single-line form satisfied it by adjacency).
- **Fix:** A `// SAFETY:` line directly above each of the four `data:` field initializers.
- **Files modified:** `port/src/mcslock.rs`
- **Verification:** both arch clippy legs exit 0
- **Committed in:** `c15362f` (Task 3 commit)

---

**Total deviations:** 5 auto-fixed (2 blocking, 3 bugs — one test-design, two lint/comment hygiene in my own new code)
**Impact on plan:** Each fix is required for the plan's own acceptance criteria to hold (smp.rs unsafe-free AND registerable, the workspace check compiling, the lint contract green, the try_lock test exercising the miss path it names). No scope creep; no public API beyond the plan's artifact list; `Cargo.lock` unchanged (zero external packages — T-05-SC gate N/A).

## Issues Encountered

- None beyond the deviations above. The baseline suite was green at spawn (168 port lib tests); pre-existing warnings (devip unused import, allocator unused-mut closure binding carried byte-identical from the old `split_census`, ds SEQ_INC, etc.) were left untouched per the scope boundary.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every artifact is fully wired and exercised: the rank/watchdog/shootdown core runs multi-threaded host tests, `IrqSpinLock` guards the real global allocator on both architectures, and the release build carries zero rank/watchdog code. The deliberate seams for later plans are named, not stubbed: `SmpArch::prepare_block` and the default no-op hooks await arch registration in plans 05-09/05-11 (the plan's own flagged assumptions, D-31), and `port::fid`'s `current_cpu_hint()` rewires to `port::smp::current_cpu_id()` in 05-08. Nothing here blocks this plan's goal. The broken-windows ledger (`.planning/WINDOWS.md`) has no entries to add: no stubs, no skipped tests, no unrun `<verify>`; all five deviations were fixed and verified in-commit.

## Threat Flags

None new. The plan's register is implemented as written: T-05-11 (shootdown deadlock) is mitigated by the poll inside both spin loops plus the claim-with-poll request path, T-05-12 (lock-order deadlock opacity) by the debug rank stack + watchdog, and T-05-39 (2-cpu heap race) by the `IrqSpinLock(HeapAlloc)` guard and the non-allocating census; all three were `mitigate` dispositions and all are pinned by host tests or gate-suite legs. No new network endpoints, auth paths, or trust-boundary schema changes. The new public surface is exactly the plan's artifact list.

## Next Phase Readiness

- **Plan 05-08 (per-cpu run queues):** `IrqSpinLock`, `LockRank::RunQueue` (60) and `Lock::try_lock`/`IrqSpinLock::try_lock` are in place for the steal path (try-lock the victim queue, never nest run-queue locks); `port::smp::current_cpu_id()` is the seam `port::fid`'s `current_cpu_hint()` rewires to.
- **Plan 05-09 (aarch64 bring-up):** register an `SmpArch` with the `TPIDR_EL1` cpu id, DAIF save/restore, the GIC SGI 1 sender wiring `service_shootdown_ipi` into the IRQ path, `local_tlb_flush` (`tlbi`), the 1 kHz `ticks`, and call `watchdog_heartbeat` from the timer tick.
- **Plan 05-11 (x86_64 bring-up):** the same shape with `%gs:0` `Mach`, `pushfq/cli` IRQ save, LAPIC vector 0xF0, and the TSC ticks.
- **Owner Stop 2 (D-32):** the cumulative `port/src` diff since the phase base now adds `smp.rs`, `mcslock.rs`, `globalalloc.rs`, `allocator.rs`, `lib.rs` on top of the stop-1 files; gates move to `-smp 2` after both arch bring-ups.

## TDD Gate Compliance

Tasks 1 and 2 are `tdd="true"` and followed RED → GREEN with committed failing pins before implementation (Task 3 is `type="auto"` without the attribute and produced one feat commit):

| Task | RED commit | GREEN commit | RED evidence (failing assertion, not compile error) |
|------|-----------|--------------|-----------------------------------------------------|
| 1 (port/src/smp.rs) | `0f10a2e` | `508bb1b` | `RED_EVIDENCE_OK` (`target_test_failed`) over `05-07-RED-task1.json`: 2 planned failures — `lock_rank_inversion_panics_naming_both_locks` ("rank inversion 50 -> 40 was not caught") and `watchdog_reports_stuck_cpu_and_lock` ("the watchdog did not trip on a stuck cpu") — 4 plumbing pins green in the same binary |
| 2 (port/src/mcslock.rs) | `2c2492e` | `ac52192` | `RED_EVIDENCE_OK` (`target_test_failed`) over `05-07-RED-task2.json`: 3 planned failures — `irq_spinlock_nested_save_restore`, `irq_spinlock_try_lock_restores_irq_on_miss`, `lock_rank_hooks_track_ranked_lock_acquisition` (IRQ/rank hooks absent) — 2 plumbing pins green (try_lock Some/None, shootdown-poll no-deadlock) |

Semantic assessment (recorded in the RED JSONs): every failing test executed and panicked on its planned IRQ/rank/watchdog assertion — no compile errors, no zero-test discovery, no fixture crashes; the multi-threaded deadlock test passed in RED because the spin-loop poll is deadlock plumbing, not a guard (the 05-06 "plumbing green, behavior red" shape). GREEN commits descend from their RED commits in ancestry (verified with `git merge-base --is-ancestor`). No REFACTOR commits were needed; the GREEN implementations were final-form. `workflow.tdd_mode` is `false` in this project, so the plan-level gate is informational here; both task-level cycles were followed with machine-classified RED evidence. (Note: `git log --grep` also surfaces `91bfbbf`/`79d7185`, the archived pre-rescope 05-07 devmnt plan's commits — a phase-numbering coincidence in history, not this plan's gates.)

---

*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 3 created files exist on disk (verified with `[ -f ]`): `port/src/smp.rs`, `05-07-RED-task1.json`, `05-07-RED-task2.json`.
- All 5 plan commits are ancestors of HEAD (verified with `git merge-base --is-ancestor`): `0f10a2e` (T1 RED), `508bb1b` (T1 GREEN), `2c2492e` (T2 RED), `ac52192` (T2 GREEN), `c15362f` (T3).
- Content pins verified: `pub enum LockRank` in `port/src/smp.rs`; `pub struct IrqSpinLock` in `port/src/mcslock.rs`; `#[forbid(unsafe_code)] pub mod smp;` in `port/src/lib.rs`; `IrqSpinLock::new("heap_alloc", LockRank::HeapAlloc` in `port/src/globalalloc.rs`.
- Measured commits from ledger (`gsd-plan-head-before-05-07` = `4ff046ee`): 5 at completion; `plan_head_after` = `c15362fe` (the last task commit before this SUMMARY's own docs commit).
- Working tree before this SUMMARY's commit: only this SUMMARY untracked (plus REQUIREMENTS.md when marked) — zero generated files left untracked, `Cargo.lock` untouched, `ds/src/tpht/` untouched.
- Plan-level verification re-run at completion: `cargo test -p port smp` 6/6 ✓, `lock_rank` 3/3 ✓, `watchdog` 1/1 ✓, `mcslock` 5/5 ✓, `irq_spinlock` 3/3 ✓, `cargo check -p port --release` rc 0 with 0 `unexpected_cfgs` ✓, `cargo xtask check` rc 0 ✓, `cargo xtask clippy --arch aarch64 --config virtshell` rc 0 ✓, `cargo xtask clippy --arch x86-64 --config q35cpu` rc 0 ✓, `cargo xtask test` rc 0 (56 ok legs) ✓.
