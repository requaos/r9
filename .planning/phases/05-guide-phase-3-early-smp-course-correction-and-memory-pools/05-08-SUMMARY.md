---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 08
subsystem: port
tags: [smp, sched, work-stealing, runqueue, irqspinlock, lock-rank, busycell, fid, prepare-block, wake-pending, smp-03, smp-11, tdd, forbid-unsafe]

# Dependency graph
requires:
  - phase: 05-06
    provides: slab-backed FidTable whose FidShard meta took the rank-40 wiring and whose alloc() routed CPU 0 through the current_cpu_hint seam
  - phase: 05-07
    provides: the SmpArch hook boundary (current_cpu_id, prepare_block, irq_save_disable/irq_restore), the documented LockRank table, IrqSpinLock/Lock::new_ranked/Lock::try_lock with rank+watchdog debug hooks, and the green baseline this plan built on
provides:
  - port/src/sched.rs (#[forbid(unsafe_code)]): RunQueues<MAX_CPUS=64> — cache-line-aligned per-CPU IrqSpinLock<VecDeque<u64>> queues at LockRank::RunQueue (60), local FIFO pop, victim steal from the BACK via try_lock after releasing the local lock (never two RunQueue locks held), idle_mask with the post-publish re-check that closes the enqueue-vs-idle lost-wake race, enqueue reporting Some(idle_cpu) for the reschedule IPI; 4 host pins incl. the 4-thread 10,000-PID conservation stress
  - port/src/fid.rs: alloc() routed via crate::smp::current_cpu_id() & shard_mask (D-12 complete) and FidShard meta as Lock::new_ranked(LockRank::FidShard=40)
  - port/src/devip/stack.rs: NET_WAITERS and PENDING_WAKES as IrqSpinLock(NetIrq=90), STACK as Lock::new_ranked(DevNetStack=14) — zero single-CPU BusyCells left in devip
  - port/src/netif.rs: INGRESS as IrqSpinLock(NetIrq=90) — the IRQ retire vs syscall handoff serializes through the ranked lock with IRQ mask/restore on both CPUs
  - port/src/devpipe.rs, devcons.rs, devip/conv.rs: the prepare_block/wake_pending 2-CPU lost-wakeup arm on every wait-then-block path (WaitPids::register carries it for all 12 devip registration sites in devip.rs/proto.rs/netlog.rs)
  - port/src/smp.rs: cpu_debug_identity_is_real() gate on the D-26 debug hooks — the rank/watchdog machinery now runs only where per-cpu identity is real (unit tests' per-thread simulation, or post-register_arch kernel builds); host integration tests no longer share one CPUS[0] rank stack
affects: [05-09 aarch64 SmpArch registration consumes sched::RunQueues + prepare_block, 05-10 aarch64 proc.rs wake_pending + block(), 05-11/05-12 x86_64 same, the devether check/register merge (deferred-items.md), Owner Stop 2]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (52000, confidence: low).
actuals:
  tokens: 23481
  tasks: 3
  commits: 4
  plan_head_before: 70779f5050194a9d8758ff995274760a465de9c5
  plan_head_after: e2c3d138f3ac7c4eef0d7d85aef7f6415f60dad4

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Work-stealing queue discipline: local IrqSpinLock guard scoped to drop BEFORE the victim scan; victims probed by try_lock in round-robin order and popped from the BACK, so a cpu never holds two rank-60 locks (equal-rank nesting is a D-26 panic) and never waits on a remote queue"
    - "Idle tracking with the flag/data race closed: pop_or_steal publishes the idle bit AFTER finding every queue empty, then RE-CHECKS once — enqueue's push (under the queue lock) happens-before its Acquire mask read, so a racing enqueue either sees the idle bit (and wakes) or is seen by the re-check (the Plan 9 / Linux idle-loop shape)"
    - "The devip-wide lost-wakeup seam lives in ONE place: WaitPids::register runs prepare_block() before the pid becomes wake-targetable, and every blocked-reader registration in devip.rs, proto.rs and netlog.rs funnels through it (12 sites, zero proto.rs churn)"
    - "BusyCell → ranked Lock/IrqSpinLock conversion shape: STACK.enter().expect(...) sites become a stack-allocated LockNode + lock(); the cfg(test) enter_stack() helper leaks one LockNode per call to preserve the pins' 'static guard shape (the smp.rs test_hooks leak precedent)"
    - "cfg(test) vs dependency-build split for debug-only per-cpu state: host INTEGRATION tests link port with cfg(test) unset, so every thread reports default cpu 0 — per-cpu debug tables must be gated on register_arch state there or parallel cargo tests corrupt them"

key-files:
  created:
    - port/src/sched.rs
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-08-RED-task1.json
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/deferred-items.md
  modified:
    - port/src/lib.rs
    - port/src/smp.rs
    - port/src/fid.rs
    - port/src/devip/stack.rs
    - port/src/netif.rs
    - port/src/devip.rs
    - port/src/devip/proto.rs
    - port/src/devip/conv.rs
    - port/src/devip/medium.rs
    - port/src/devip/ipifc.rs
    - port/src/devip/route.rs
    - port/src/devpipe.rs
    - port/src/devcons.rs

key-decisions:
  - "RunQueues is a generic <const MAX_CPUS: usize = 64> (smp::MAX_CPUS), const-asserted <= 64 because idle_mask is one AtomicU64; every entry point takes num_cpus and the conservation stress runs 4 threads, so nothing hard-codes 2 (D-23)"
  - "pop_or_steal re-checks the queues once AFTER publishing its idle bit: the plan's push-then-read-mask / publish-then-recheck pairing is the flag/data idiom that makes 'an enqueue racing an about-to-sleep cpu' impossible to lose (Rule 2 addition to the plan's literal step list, which had the window open)"
  - "The devip prepare_block arm lives inside WaitPids::register (conv.rs — the plan-named file), not at 12 call sites in proto.rs/devip.rs/netlog.rs: register is the single funnel every blocked-reader registration already passes through under its caller's lock, and the clear runs even on a no-op re-registration because a wake can have landed since the predicate re-test"
  - "The D-26 rank/watchdog debug hooks gate on cpu_debug_identity_is_real(): cfg(test) builds keep the per-thread simulation; non-test builds (the kernel) run the checks once register_arch has published real per-cpu ids; host integration tests (cfg(test) unset, all threads on default cpu 0) skip the bookkeeping entirely while still exercising the real MCS discipline (Rule 1 fix — see Deviations 1)"
  - "SMP-03 and SMP-11 are recorded as contributed, not checked off in REQUIREMENTS.md (05-07's convention): SMP-03's whole-kernel audit wording and SMP-11's 'each CPU steals' end-to-end wording complete only when 05-10/05-12 wire RUN_QUEUES and wake_pending into the arch sched loops"

patterns-established:
  - "RED-for-a-new-module shape: full API scaffolding + all four host pins committed with exactly the planned behavior omitted (steal, idle publish, idle-wake report), machine-classified RED evidence (RED_EVIDENCE_OK, target_test_failed); GREEN adds the behavior without touching the test file"
  - "Multi-threaded conservation stress over a shared RunQueues via Arc: one producer pins all 10,000 pids on cpu 0, three consumers must steal every pid (per-pid AtomicUsize counts; 'every consumer stole at least one' is the steal witness); a bounded empty-spin exit keeps a RED failure fast instead of wedging"

requirements-completed: [SMP-03, SMP-11]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "port/src/sched.rs (#[forbid(unsafe_code)]) implements per-CPU RunQueues<MAX_CPUS> with work stealing via try_lock on victim queues, host-tested under multi-threaded contention with zero lost or duplicated PIDs; enqueueing to an adjacent CPU's queue wakes an idle CPU and stealing works for any num_cpus >= 2 (num_cpus = 4 in the stress); pop_or_steal returns None cleanly when all queues are empty (transitioning the CPU into idle tracking); local pop preserves FIFO while victim steals pop from the back, never holding two RunQueue locks (SMP-11, D-23, D-25, D-26)"
    requirement: SMP-11
    verification:
      - kind: unit
        ref: "cargo test -p port sched (4/4): runqueues_local_pop_is_fifo, runqueues_steal_pops_victim_back_with_try_lock (with rank-stack-balance pin), runqueues_enqueue_wakes_an_idle_cpu, runqueues_four_cpu_steal_conserves_10k_pids"
        status: pass
      - kind: automated
        ref: "grep 'forbid(unsafe_code)' adjacency in port/src/lib.rs; cargo check -p port --release rc 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "SMP-03 (port/ half): adjacent concurrent accesses to NET_WAITERS, PENDING_WAKES, STACK (devip/stack.rs) and INGRESS (netif.rs) serialize through ranked IrqSpinLock/Lock instead of failing spuriously on BusyCell; wait queues in devpipe, devcons and devip/conv pair waiter registration with prepare_block()/wake_pending so no wake is lost; every converted lock obeys LockRank ordering (DevNetStack=14, FidShard=40, RunQueue=60, NetIrq=90) and FidTable::alloc routes via port::smp::current_cpu_id() & shard_mask (D-12, D-22, D-26)"
    requirement: SMP-03
    verification:
      - kind: automated
        ref: "grep -v comments: zero BusyCell tokens in devip/stack.rs and netif.rs; IrqSpinLock::new(\"net_waiters\"/\"pending_wakes\"/\"netif_ingress\", LockRank::NetIrq) and Lock::new_ranked(\"devip_stack\", LockRank::DevNetStack) / (\"fid_shard\", LockRank::FidShard) present; cargo test -p port 19 suites green; cargo xtask check rc 0; cargo xtask test-9p rc 0 (14 legs); cargo test -p xtask lintcontract 13/13; clippy aarch64 virtshell rc 0; clippy x86-64 q35cpu rc 0"
        status: pass
    human_judgment: false

# Metrics
duration: 33min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 08: Per-CPU Work-Stealing RunQueues, FidTable CPU Routing, and SMP-Safe Device Wait Paths Summary

**`port/src/sched.rs` (`#[forbid(unsafe_code)]`) now carries the shared per-CPU work-stealing scheduler — cache-line-aligned `IrqSpinLock<VecDeque<u64>>` queues at `LockRank::RunQueue`, local FIFO pop, victim back-steal via `try_lock` with the local lock released first, and an idle mask whose publish-then-recheck closes the enqueue-vs-idle race — proven by a 4-thread 10,000-PID conservation stress. `FidTable::alloc` routes by `crate::smp::current_cpu_id() & shard_mask` (D-12 complete) with `FidShard` meta ranked 40; every single-CPU `BusyCell` in `devip/stack.rs` and `netif.rs` is gone in favor of ranked `IrqSpinLock`/`Lock` (90/90/14); and every `port/` wait-then-block path named by the plan runs `prepare_block()` before its pid becomes wake-targetable. One Rule-1 fix to 05-07's rank/watchdog debug hooks keeps host integration tests sound. The full gate suite is green: 19 port suites, `xtask check`, `test-9p` (14 legs), both arch clippy legs, lintcontract 13/13.**

## Performance

- **Duration:** 33 min
- **Started:** 2026-11-26T06:59:02Z
- **Completed:** 2026-11-26T07:31:39Z
- **Tasks:** 3 of 3 executed (Task 1 TDD RED→GREEN; Tasks 2-3 auto)
- **Files modified:** 13 (1 created: port/src/sched.rs; plus the RED evidence record and the phase deferred-items.md)

## Accomplishments

- **SMP-11 — the portable run queues are host-proven.** `RunQueues<const MAX_CPUS: usize = 64>` holds one `#[repr(align(64))]` `CpuQueue` per cpu (`IrqSpinLock<VecDeque<u64>>`, rank 60) plus an `AtomicU64` idle mask. `enqueue` pushes to `target % num_cpus` and reports the idle cpu to kick (target first, then lowest idle bit); `pop_or_steal` pops the local front, releases the lock, then `try_lock`-scans neighbors `(my+offset) % num_cpus` popping the victim's BACK — a cpu never holds two RunQueue locks and never waits on a remote one. The all-empty path publishes idle and re-checks once, closing the lost-wake window between an enqueue and an about-to-sleep cpu. The stress pin runs 4 threads over `num_cpus = 4` with all 10,000 pids produced on cpu 0: every pid popped exactly once, every consumer stole at least one.
- **SMP-03 — the devip/netif seam is two-CPU clean.** `NET_WAITERS`, `PENDING_WAKES` and `INGRESS` are `IrqSpinLock`s at `NetIrq` (90) whose own IRQ mask/restore replaces the "syscalls run IRQ-masked, so the cell is uncontended" invariant; `STACK` is `Lock::new_ranked("devip_stack", DevNetStack=14)`, so two CPUs contending for a drive serialize on the MCS queue instead of tripping a fail-fast `.expect`. The wake-after-guard-drop discipline (never the wake hook with a lock held) is preserved verbatim, and the waker bridge's mid-poll `note_wake` under STACK is now a legal 14 < 90 rank nesting. 53 mechanical `STACK.enter()` call sites across `devip.rs`, `proto.rs`, `conv.rs`, `medium.rs`, `ipifc.rs` and `route.rs` moved to stack-allocated `LockNode` guards; `devmnt`'s `BusyCell` is untouched per the plan (Pitfall 5).
- **SMP-03 — no lost wakeups on the named wait paths.** `devpipe`'s read and write loops and `devcons`'s console read arm call `crate::smp::prepare_block()` under their queue locks before the pid enters the waiter list; in devip the arm lives in `WaitPids::register` (`conv.rs`), the single funnel all 12 blocked-reader registrations in `devip.rs`/`proto.rs`/`netlog.rs` pass through. Single-CPU comments in all three files now state the 2-CPU `wake_pending` invariant (consumed by the arch `block()` in plans 05-10/05-12).
- **D-12 complete.** `FidTable::alloc` starts at `crate::smp::current_cpu_id() & shard_mask` (CPU 0 until an arch registers its `SmpArch`), and every shard's `AllocMeta` lock is `Lock::new_ranked("fid_shard", LockRank::FidShard)`.

## Task Commits

Task 1 is `tdd="true"` (RED before GREEN, machine-classified evidence); Tasks 2 and 3 are `type="auto"`:

1. **Task 1, RED: failing SMP-11 work-stealing pins over sched.rs scaffolding** — `0157b8f` (test) — 1 passed, 3 failed on the planned steal/idle-wake/conservation assertions; RED evidence `05-08-RED-task1.json` classified `RED_EVIDENCE_OK` (`target_test_failed`)
2. **Task 1, GREEN: SMP-11 per-CPU work-stealing RunQueues in port/src/sched.rs** — `ff738e3` (feat) — 4/4 sched tests, full port suite green, release check rc 0
3. **Task 2: SMP-03 ranked locks replace single-CPU BusyCells; FidTable CPU-ID routing** — `5dc1e97` (feat) — 10 files; `cargo test -p port` 19 suites green, `cargo xtask check` rc 0
4. **Task 3: SMP-03 prepare_block lost-wakeup fixes in port/ device block loops** — `e2c3d13` (feat) — `cargo test -p port` green, `cargo xtask test-9p` rc 0 (14 legs)

**Plan metadata:** the SUMMARY + deferred-items commit (docs: complete plan), committed atomically with this file.

## Files Created/Modified

- `port/src/sched.rs` — **created** (~380 lines): module docs (steal discipline, idle tracking, the no-hardcoded-2 rule), `CpuQueue` with the align(64) const assert, `RunQueues<MAX_CPUS>` (`new`, `enqueue`, `pop_or_steal`, `steal_scan`, `mark_idle`, `mark_busy`, `take_idle_cpu`, `len_for_cpu`, `total_len`), and the four host pins
- `port/src/lib.rs` — `#[forbid(unsafe_code)] pub mod sched;` mount with provenance comment
- `port/src/smp.rs` — `cpu_debug_identity_is_real()` and the gate on all six debug entry points (`rank_before_acquire`, `rank_after_release`, `watchdog_heartbeat`, `watchdog_begin_wait`, `watchdog_end_wait`, `watchdog_check_all`); doc explains the host-integration-test corruption class (Deviation 1)
- `port/src/fid.rs` — `current_cpu_hint()` now delegates to `crate::smp::current_cpu_id()`; `FidShard.meta` is `Lock::new_ranked("fid_shard", LockRank::FidShard, ...)`; module/alloc docs updated
- `port/src/devip/stack.rs` — header LOCKING section rewritten for the ranked-lock era; `NET_WAITERS`/`PENDING_WAKES`/`STACK` statics converted; all in-file call sites including `tick_check`, `note_rx_pending`, `drive_stack`, `AcceptPool`, and the cfg(test) `install_test_stack`/`install_test_ether`/`enter_stack` (the last leaks one `LockNode` per call to keep the pins' `'static` guard shape)
- `port/src/netif.rs` — header LOCKING section rewritten; `INGRESS` converted; all 15 call sites converted (NETIF → INGRESS nested sites take a second `LockNode`); the `rx_ingress`/`tx_retired`/`conv_allocated` None-arms of the old fail-fast cell removed
- `port/src/devip.rs`, `proto.rs`, `conv.rs`, `medium.rs`, `ipifc.rs`, `route.rs` — the mechanical `STACK.lock(&node)` sweep (53 sites)
- `port/src/devpipe.rs` — header invariant rewritten; `prepare_block()` before the read-loop `waiters.push` and the write-loop `wwaiters.push` (both under the Pipe lock); loop comments updated
- `port/src/devcons.rs` — the LOCKING comment's single-CPU tick invariant replaced by the 2-CPU `wake_pending` rule; `prepare_block()` under the INPUT lock before `waiters.push`
- `port/src/devip/conv.rs` — the LOCKING comment updated; `WaitPids` doc names the single-funnel design; `register` runs `crate::smp::prepare_block()` first (even on a no-op re-registration)
- `05-08-RED-task1.json` — **created** (TDD RED evidence record, committed with the RED commit)
- `deferred-items.md` — **created** (the devether check/register gap, owned by 05-10/05-12)

## Decisions Made

- **The idle publish re-check is part of `pop_or_steal`, not the arch loop.** The plan's literal step list (publish idle, return None) leaves a lost-wake window: an enqueue landing between the empty scan and the idle publication would not wake the sleeper. The implemented pairing — enqueue's push happens-before its Acquire mask read; the idler publishes then re-checks once — is the standard flag/data idiom; a racing enqueue either sees the idle bit (IPI sent) or is seen by the re-check. Documented in the `enqueue`/`pop_or_steal` doc comments.
- **`WaitPids::register` owns the devip-wide seam.** All twelve blocked-reader registrations (`devip.rs` x2, `proto.rs` x9, `netlog.rs` x1) already funneled through it under their caller's lock, and `stack::register_net_waiter` always runs AFTER it at every site — so one `prepare_block()` inside `register` covers every devip wait-then-block path with zero proto.rs churn, and no second clear can eat a wake that landed between registrations. The clear deliberately runs even when the insert is a no-op (the loops re-register before EVERY block; a wake can have landed since the predicate re-test).
- **Debug rank/watchdog checks are gated on real per-cpu identity** (Deviation 1): unit tests keep the per-thread simulation, kernel debug/gate builds keep D-26's teeth after `register_arch`, and host integration tests skip the bookkeeping while still exercising the real MCS protocol. The gate is one atomic load per lock op in debug builds only; release is unchanged (zero-cost stubs).
- **`enter_stack()` leaks its `LockNode` (cfg(test) only).** The 66 pin call sites in `devip_tests.rs` were written against a `'static` guard; leaking one node per call preserves that shape without touching the pins (the `smp.rs` test_hooks `Box::leak` precedent).
- **REQUIREMENTS.md checkboxes stay Pending.** Following 05-07's convention (its four SMP requirements also stayed unchecked), SMP-03's whole-kernel audit and SMP-11's end-to-end "each CPU steals" complete only when 05-10/05-12 wire `RUN_QUEUES` and `wake_pending` into the arch sched loops; this plan's frontmatter records the contribution.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Gated the D-26 rank/watchdog debug hooks on real per-cpu identity (`port/src/smp.rs`)**
- **Found during:** Task 2 (wiring `FidShard` meta to `LockRank::FidShard` made `cargo test -p port --test aead_fid` fail with spurious rank panics — "holds \"fid_shard\" (rank 40); requested \"fid_shard\"" and "released lock it does not hold" — and intermittently wedge)
- **Issue:** Host INTEGRATION tests (`port/tests/*.rs`) link `port` as a plain dependency, so `cfg(test)` is unset there and `rank_watch::this_cpu_debug()` takes the kernel path: the shared `static CPUS: [CpuDebug; MAX_CPUS]` table, with every host thread reporting default cpu 0. Parallel cargo tests corrupted the single shared rank stack across threads. 05-07 never hit this because no integration test took a ranked lock until this plan (the heap lock is unused on host).
- **Fix:** `cpu_debug_identity_is_real()` — `true` in `cfg(test)` (per-thread simulated cpus), `ARCH.get().is_some()` otherwise — gates all six debug entry points. Unit tests and post-registration kernel debug/gate builds keep the full D-26 assertions; host integration tests skip only the bookkeeping and still exercise the real MCS mutual exclusion their threaded pins depend on.
- **Files modified:** `port/src/smp.rs`
- **Verification:** `aead_fid` 31/31 (was 27/31 with panics), full port suite 19 suites green, both arch clippy legs rc 0, `xtask check` rc 0
- **Committed in:** `5dc1e97` (Task 2 commit)

**2. [Rule 2 - Missing critical functionality] Closed the enqueue-vs-idle-publication lost-wake race in `pop_or_steal`**
- **Found during:** Task 1 GREEN (reviewing the plan's step-4 literal ordering against the idle mask)
- **Issue:** The plan's all-empty arm ("set `my_cpu` bit in `idle_mask` and return `None`") races with `enqueue`: push on cpu B between cpu A's empty scan and A's idle-bit store → B reads a not-yet-idle mask, sends no IPI, and A sleeps with work queued.
- **Fix:** After publishing the idle bit, `pop_or_steal` re-checks the local queue and re-runs the steal scan once before returning `None`. With enqueue's push ordered before its Acquire mask read, a racing enqueue either observes the idle bit (wake sent) or is observed by the re-check — the Plan 9 / Linux idle-loop pairing.
- **Files modified:** `port/src/sched.rs`
- **Verification:** 4/4 sched pins; the 4-thread conservation stress exercises the path on every empty transition
- **Committed in:** `ff738e3` (Task 1 GREEN commit)

**3. [Rule 3 - Blocking] The STACK type change forced a 53-site sweep across six files the plan's file list did not name**
- **Found during:** Task 2 (`STACK: BusyCell → Lock` changed the type of a `pub(crate)` static consumed across the devip subsystem)
- **Issue:** The plan names `devip/stack.rs` for the STACK conversion, but `STACK.enter().expect("devip: stack contended")` had 53 call sites in `devip.rs` (4), `proto.rs` (35), `conv.rs` (1), `medium.rs` (3), `ipifc.rs` (8) and `route.rs` (2) — the plan's acceptance criterion (`cargo xtask check` rc 0) cannot hold without converting them.
- **Fix:** Uniform mechanical conversion to the house two-line shape (`let node = LockNode::new();` + `.lock(&node)`); `medium.rs` gained the `LockNode` import. No semantics changed at any site (the fail-fast `.expect` arm was unreachable-by-design on single CPU; two-CPU contention now serializes, which is the point of the conversion).
- **Files modified:** the six files above
- **Verification:** `cargo test -p port` 19 suites green (incl. the 66 `enter_stack` pin callers, unchanged), `cargo xtask check` rc 0
- **Committed in:** `5dc1e97` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 bug — cross-cfg debug-state corruption; 1 missing-critical — the scheduler's own lost-wake race; 1 blocking — the type-change sweep). One out-of-scope discovery logged to the phase `deferred-items.md` (below).
**Impact on plan:** Each fix is required for the plan's own acceptance criteria to hold (integration tests green with a ranked `FidShard`, no lost wake in the scheduler itself, the workspace check compiling). No scope creep; no public API beyond the plan's artifact list; `Cargo.lock` unchanged (zero external packages — T-05-SC gate N/A).

### Out-of-scope discovery (logged, not fixed)

- **devether.rs's block loops keep a pre-existing 2-CPU check/register gap** (`pop_frame`/`tx_post` and the waiter registration are separate INGRESS acquisitions, so an IRQ on the sibling CPU can land between them and send no wake). Closing it needs check-and-register in one critical section — a structural change (Rule 4) outside this plan's three-file Task 3 scope. Logged in the phase `deferred-items.md`, owned by the arch-side SMP-03 plans (05-10/05-12) that wire `wake_pending` into `block()`/`wake()`.

## Issues Encountered

- None beyond the deviations above. The baseline suite was green at spawn (179 port lib tests); pre-existing warnings (ds SEQ_INC, devip unused import, allocator unused-mut, netlog dead code, proto TCPSTAT consts) were left untouched per the scope boundary.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every artifact is fully wired and exercised: `RunQueues` runs multi-threaded host proofs; the ranked locks guard the real devip/netif seam; `prepare_block` is called at every named registration site (a deliberate no-op until 05-10/05-12 register the arch hook — the plan's own flagged assumption, D-31 — not a stub). The broken-windows ledger (`.planning/WINDOWS.md`) has no entries to add: no stubs, no skipped tests, no unrun `<verify>`; all three deviations were fixed and verified in-commit. The devether gap is a pre-existing design item recorded in `deferred-items.md`, not a defect introduced or left by this plan's own changes.

## Threat Flags

None new. The plan's register is implemented as written: T-05-13 (2-CPU contention/lost wakeups on the device wait paths) is mitigated by the ranked-lock conversions plus the `prepare_block` arms, and T-05-14 (run-queue tampering/duplication) by the per-cpu `IrqSpinLock`s, the try-lock-after-release steal discipline, and the 10,000-PID conservation stress; both were `mitigate` dispositions and both are pinned by host tests. T-05-SC: `Cargo.lock` is byte-unchanged (verified) — zero external packages installed. No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes; the new public surface is exactly the plan's artifact list.

## Next Phase Readiness

- **Plan 05-09 (aarch64 bring-up):** register the `SmpArch` (TPIDR_EL1 cpu id, DAIF save/restore, GIC SGI senders, `local_tlb_flush`, 1 kHz ticks); `port::sched::RunQueues` is ready to back `RUN_QUEUES` and `send_resched_ipi` consumes `enqueue`'s `Some(idle_cpu)`.
- **Plans 05-10/05-12 (arch proc.rs):** implement `Proc::wake_pending`, register `prepare_block`, set `wake_pending = true` (Release) before taking `PROC_TABLE` in `wake(pid)`, and check `wake_pending.swap(false, AcqRel)` inside `block()` — the device-side arms this plan landed are inert until then (safe no-ops on the single-CPU default hook). Also owns the devether check/register merge from `deferred-items.md`.
- **Owner Stop 2 (D-32):** the cumulative `port/src` diff since the phase base now adds `sched.rs`, `smp.rs`, `fid.rs`, `devip/*`, `netif.rs`, `devpipe.rs`, `devcons.rs` on top of the stop-1 files; gates move to `-smp 2` after both arch bring-ups.

## TDD Gate Compliance

Task 1 is `tdd="true"` and followed RED → GREEN with committed failing pins before implementation (Tasks 2 and 3 are `type="auto"` without the attribute and produced one feat commit each):

| Task | RED commit | GREEN commit | RED evidence (failing assertion, not compile error) |
|------|-----------|--------------|-----------------------------------------------------|
| 1 (port/src/sched.rs) | `0157b8f` | `ff738e3` | `RED_EVIDENCE_OK` (`target_test_failed`) over `05-08-RED-task1.json`: 3 planned failures — `runqueues_steal_pops_victim_back_with_try_lock` ("left: None, right: Some(102)"), `runqueues_enqueue_wakes_an_idle_cpu` ("left: None, right: Some(2)"), `runqueues_four_cpu_steal_conserves_10k_pids` ("left: 0, right: 10000") — 1 plumbing pin green in the same binary (local FIFO + len + rank-stack balance) |

Semantic assessment (recorded in the RED JSON): every failing test executed and panicked on its planned SMP-11 behavior assertion — no compile errors, no zero-test discovery, no fixture crashes; the RED scaffolding deliberately omitted exactly the steal, the idle publish and the idle-wake report (RED-SCAFFOLD comments named both sites). GREEN (`ff738e3`) descends from RED (`0157b8f`) in ancestry (verified with `git merge-base --is-ancestor`). No REFACTOR commit was needed; the GREEN implementation was final-form. `workflow.tdd_mode` is `false` in this project, so the plan-level gate is informational here; the task-level cycle was followed with machine-classified RED evidence.

---

*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 3 created files exist on disk (verified with `[ -f ]`): `port/src/sched.rs`, `05-08-RED-task1.json`, `deferred-items.md`.
- All 4 plan commits are ancestors of HEAD (verified with `git merge-base --is-ancestor`): `0157b8f` (T1 RED), `ff738e3` (T1 GREEN), `5dc1e97` (T2), `e2c3d13` (T3).
- Content pins verified: `pub struct RunQueues` in `port/src/sched.rs`; `#[forbid(unsafe_code)] pub mod sched;` in `port/src/lib.rs`; `Lock::new_ranked("fid_shard", LockRank::FidShard` and `crate::smp::current_cpu_id()` in `port/src/fid.rs`; `IrqSpinLock::new("net_waiters", LockRank::NetIrq` in `port/src/devip/stack.rs`; `"netif_ingress"` in `port/src/netif.rs`; `crate::smp::prepare_block();` in `port/src/devpipe.rs`, `port/src/devcons.rs` and `port/src/devip/conv.rs`.
- Measured commits from ledger (`gsd-plan-head-before-05-08` = `70779f50`): 4 at completion; `plan_head_after` = `e2c3d138` (the last task commit before this SUMMARY's own docs commit).
- Working tree before this SUMMARY's commit: only this SUMMARY and `deferred-items.md` untracked (both staged in the metadata commit) — zero generated files left untracked, `Cargo.lock` untouched (byte-identical), `ds/src/tpht/` untouched.
- Plan-level verification re-run at completion: `cargo test -p port sched` 4/4 ✓, `cargo test -p port` 19 suites ok ✓, `cargo xtask test-9p` rc 0 (14 legs) ✓, `cargo xtask check` rc 0 ✓, `cargo test -p xtask lintcontract` 13/13 ✓, `cargo xtask clippy --arch aarch64 --config virtshell` rc 0 ✓, `cargo xtask clippy --arch x86-64 --config q35cpu` rc 0 ✓.
