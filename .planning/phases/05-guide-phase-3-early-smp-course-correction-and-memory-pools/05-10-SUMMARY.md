---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 10
subsystem: aarch64
tags: [smp, aarch64, percpu, tpidr-el1, irqspinlock, lock-rank, runqueue, work-stealing, wake-pending, lost-wakeup, proc-table, pagealloc, pl011, virtio-net, unsafe-audit, smp-02, smp-03, smp-11]

# Dependency graph
requires:
  - phase: 05-07
    provides: the SmpArch hook boundary (register_arch, current_cpu_id, num_online_cpus, irq_save_disable/irq_restore, send_resched_ipi, prepare_block), the LockRank table, IrqSpinLock/IrqLockGuard, and the GlobalQuickAlloc HeapAlloc (80) precedent
  - phase: 05-08
    provides: port::sched::RunQueues (per-CPU IrqSpinLock<VecDeque> at LockRank::RunQueue 60, pop_or_steal, idle-mask re-check), and the port-side prepare_block call sites (devpipe/devcons/devip WaitPids::register)
  - phase: 05-09
    provides: aarch64/src/smp.rs PerCpu (TPIDR_EL1; cur_frame at offset 0 consumed by trap.S), the SmpArch registration fn (register_port_smp, called in 05-22), and the irq_save_disable/irq_restore DAIF hooks
provides:
  - aarch64/src/proc.rs: PerCpu-backed current_proc/cur_kstack/cur_frame/sched_context/sched_seeded/park_context (zero CURRENT_PROC/CURKSTACK/CURFRAME/SCHED_CONTEXT/SCHED_SEEDED/BOOT_CTX statics), PROC_TABLE IrqSpinLock at LockRank::ProcTable (50), SHARED_PAGES/RFMEM_FAMILIES/SEM_WAITERS IrqSpinLock at LockRank::VmShared (30), Proc::wake_pending AtomicBool + wake_locked critical section + block() flag-consume (Pitfall 4 closed), RUN_QUEUES (port::sched::RunQueues<64>) driving pick_next's pop_or_steal, enqueue_ready wiring every Ready transition with the reschedule-IPI report
  - aarch64/src/smp.rs: PerCpu gains park_context + sched_stack_top; prepare_block dispatches to proc::prepare_block_current (registered on SmpArch::prepare_block; live from 05-22's register_port_smp call)
  - aarch64/src/fpu.rs: this_cpu() reads &smp::this_cpu().fpu (the MPIDR-indexed CPUS static is gone)
  - aarch64/src/pagealloc.rs: PAGE_ALLOC is IrqSpinLock at LockRank::PageAlloc (70)
  - aarch64/src/platform/virt/pl011.rs: CONS_RX is IrqSpinLock<ConsRx> at LockRank::Console (100)
  - aarch64/src/platform/virt/pci.rs: VIRTIO_NET is IrqSpinLock<Option<VirtioNetState>> at LockRank::NetIrq (90); the IRQ handler never holds the guard across the port wake/ingress callbacks (equal-rank NetIrq-90 nesting would be a D-26 violation)
  - UNSAFE-AUDIT.md: Plan 05-10 deltas (aarch64 202 -> 191 blocks across the two tasks, SAFETY 408 -> 397), zero single-CPU IRQ-masked safety justifications in the audited files
affects: [05-11/05-12 (x86_64 twin of this migration), 05-22 (aarch64 -smp 2 boot: register_port_smp + start_secondaries + the real secondary_main entering this sched_loop), Owner Stop 2]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (52000, confidence: low).
actuals:
  tokens: 32695
  tasks: 2
  commits: 4
  plan_head_before: 41a7f8d33cd1283066811451fc52d0903f4d2da2
  plan_head_after: 4b1f07e4ea6d062aded562857a21bca9fbcaa5a6

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "wake/block contract (SMP-03, Pitfall 4): a waker publishes wake_pending (Release) INSIDE its PROC_TABLE critical section together with the condition mutation and the Blocked->Ready+enqueue; block() consumes it (swap(false, AcqRel)) under the same lock BEFORE marking Blocked and returns without sleeping when a wake already landed — the flag store and state transition sharing one critical section is what makes the pairing race-free"
    - "prepare_block discipline split: subsystem-owned conditions (pipe/devcons/devip wait sets, SEM_WAITERS) call port::smp::prepare_block() under the subsystem lock BEFORE registration (the 05-08 port contract); proc-owned conditions (await's wait_records, sleep's sleep_until, rendez) need no prepare_block because the waker mutates the condition inside PROC_TABLE itself — clearing the flag there would lose one-shot wakes"
    - "Ready = enqueued invariant: every Ready-producing transition (wake_locked, reschedule's preemption, exec_into's frame seed, rfork_clone's copy completion, seed_kernel_context) calls enqueue_ready exactly once; pick_next pops via RUN_QUEUES.pop_or_steal and skips stale entries (not-Ready) — a proc reschedule() switches to directly leaves a stale entry by design"
    - "publish-once cells are not IRQ-shared-mutable state: PL011_RANGE, PCI_DISCOVERY, VIRTIO_NET_INTID stay SyncUnsafeCell (write-once at boot, read-only thereafter); only post-publish-mutated IRQ-shared cells (CONS_RX, VIRTIO_NET) became IrqSpinLocks"
    - "IrqSpinLock IRQ handler phasing: virtio_net_irq locks VIRTIO_NET per phase (ISR ack + TX retire; per-frame RX extraction into a 2 KiB stack buffer; replenish) and calls rx_ingress/tx_retired/note_rx_pending only after the guard drops, because those take NetIrq-90 port locks (equal-rank nesting is a D-26 panic)"

key-files:
  created: []
  modified:
    - aarch64/src/proc.rs
    - aarch64/src/smp.rs
    - aarch64/src/syscall.rs
    - aarch64/src/fpu.rs
    - aarch64/src/pagealloc.rs
    - aarch64/src/exec.rs
    - aarch64/src/trap.rs
    - aarch64/src/main.rs
    - aarch64/src/l.S
    - aarch64/src/platform/virt/churngate.rs
    - aarch64/src/platform/virt/pl011.rs
    - aarch64/src/platform/virt/pci.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "wake_pending is stored INSIDE the waker's PROC_TABLE critical section (before the state check), not before acquiring the lock as the plan's shorthand reads: a pre-lock store would let a blocker consume the flag, re-check before the waker's condition mutation is visible to it, and re-block past a queued record (the consumed-flag/queued-record race). The lock-paired placement eliminates Pitfall 4; the ordering rationale is written into wake_locked's and post_wait_record's doc comments"
  - "No standalone pub fn wake(pid): every pid-keyed wake path (sem_wake, pipe_wake, rendez_wake, cons_input_wake, tick_timeouts, push_note, post_wait_record) funnels through the one wake_locked critical section on an already-resolved &mut Proc — a public pid-lookup wake would be dead code (the gate denies warnings) without adding a caller"
  - "reschedule() keeps its PROC_TABLE scan with the !blocked_in_kernel filter (the EL0-preemption path can only trap_return) rather than popping the run queue; the preempted proc's Ready marking now enqueues it, and direct-switch targets leave stale queue entries that pick_next filters"
  - "pick_next skips stale queue entries (pid not found or not Ready) instead of treating them as fatal; the Ready = enqueued invariant means a genuinely Ready proc always has an entry, so skips are bounded churn (a preempted proc carries one entry per preemption)"
  - "The churn driver's seed_kernel_context enqueues the proc (dev_churn profile): the kernel-context proc is Ready with no TrapFrame and was previously found by pick_next's table scan; under the queue it must be entered explicitly"
  - "sys_sleep arms with prepare_block (the only one-shot block caller): the tick re-evaluates sleep_until/note_queue every tick, so clearing a stale flag before arming cannot lose a wake, and it prevents a leftover flag from misreporting an early return as 'interrupted'"
  - "publish-once cells (PL011_RANGE, PCI_DISCOVERY, VIRTIO_NET_INTID) intentionally NOT converted: write-once-at-boot-then-read-only is not IRQ-shared-mutable state; converting them would add lock overhead to the IRQ dispatch hot path with no safety gain"

patterns-established:
  - "wake_locked(p) under PROC_TABLE: wake_pending.store(true, Release); if Blocked { state = Ready; enqueue_ready(pid) } — the single wake critical-section shape every wake path funnels through"
  - "block(): PROC_TABLE { if wake_pending.swap(false, AcqRel) return; state = Blocked; blocked_in_kernel = true; fpu::leave } then swtch to this CPU's sched_context — the flag-consume and Blocked-set are one critical section"
  - "enqueue_ready(pid): RUN_QUEUES.enqueue(current_cpu_id(), pid, num_online) + send_resched_ipi on the reported idle CPU — no-op IPI on the single-CPU default hooks until 05-22 registers the arch"
  - "RX IRQ handler with a ranked driver lock: per-frame locked extraction into an MTU-cap stack buffer (2048 B, over-cap = counted device fault), unlocked ingress, locked replenish — the no-guard-across-callbacks pattern for equal-rank (NetIrq-90) port locks"

requirements-completed: [SMP-02, SMP-03, SMP-11]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Zero single-CPU statics remain in aarch64/src/proc.rs (CURRENT_PROC/CURKSTACK/CURFRAME/SCHED_CONTEXT/SCHED_SEEDED/BOOT_CTX migrated to PerCpu via TPIDR_EL1); PROC_TABLE at LockRank::ProcTable (50), SHARED_PAGES/RFMEM_FAMILIES/SEM_WAITERS at VmShared (30), PAGE_ALLOC at PageAlloc (70) — all ranked IrqSpinLocks (SMP-02, SMP-03)"
    requirement: SMP-02
    verification:
      - kind: automated
        ref: "grep -E '(static|static mut).*(CURRENT_PROC|CURKSTACK|CURFRAME|SCHED_CONTEXT|SCHED_SEEDED|BOOT_CTX)' aarch64/src/proc.rs = 0 matches; grep static PROC_TABLE/SHARED_PAGES/RFMEM_FAMILIES/SEM_WAITERS = IrqSpinLock; static PAGE_ALLOC = IrqSpinLock(LockRank::PageAlloc); cargo xtask check rc 0; cargo xtask clippy --arch aarch64 --config virtshell rc 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "Proc::wake_pending AtomicBool + prepare_block_current (registered on SmpArch::prepare_block) + block()'s flag-consume under PROC_TABLE eliminate 2-CPU lost wakeups across every block path (semacquire, sleep, await, rendezvous, pipe, cons, devip, mnt contention, churn waits) (SMP-03, SMP-11)"
    requirement: SMP-03
    verification:
      - kind: automated
        ref: "wake_pending field + wake_locked + block() compile; cargo xtask clippy --arch aarch64 --config virtshell rc 0; cargo xtask test rc 0 (all host legs incl. the virtshell-cfg aarch64 leg; proc.rs sem/rfmem/fdgrp pins pass)"
        status: pass
    human_judgment: false
  - id: D3
    description: "aarch64 sched_loop runs port::sched::RunQueues work-stealing via pick_next's pop_or_steal with the idle wfi + reschedule-IPI report wiring (SMP-11, D-23, D-25); single-CPU operation preserved on the default hooks until 05-22 registers the arch and starts CPU 1"
    requirement: SMP-11
    verification:
      - kind: automated
        ref: "RUN_QUEUES.pop_or_steal in pick_next; enqueue_ready on every Ready transition; cargo xtask check rc 0; cargo xtask test rc 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "CONS_RX in aarch64/src/platform/virt/pl011.rs protected by IrqSpinLock at LockRank::Console (100); zero raw mutable IRQ-shared SyncUnsafeCell globals remain in platform/virt (SMP-03)"
    requirement: SMP-03
    verification:
      - kind: automated
        ref: "static CONS_RX: IrqSpinLock<ConsRx>(\"cons_rx\", LockRank::Console, ...); remaining platform/virt SyncUnsafeCell statics are all publish-once; cargo xtask clippy --arch aarch64 --config virtshell rc 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "VIRTIO_NET in aarch64/src/platform/virt/pci.rs protected by IrqSpinLock at LockRank::NetIrq (90); the IRQ handler never holds the guard across the NetIrq-90 port wake/ingress callbacks (D-26 rank-violation avoidance) (SMP-03)"
    requirement: SMP-03
    verification:
      - kind: automated
        ref: "static VIRTIO_NET: IrqSpinLock<Option<VirtioNetState>>(\"virtio_net\", LockRank::NetIrq, None); phased handler (locked extraction, unlocked ingress/wake); cargo xtask clippy --arch aarch64 --config virtshell rc 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "UNSAFE-AUDIT.md updated in lockstep in every task commit that modified arch unsafe blocks (D-03): aarch64 202 -> 191 blocks, SAFETY 408 -> 397, per-file inventory rows updated"
    verification:
      - kind: automated
        ref: "UNSAFE-AUDIT.md Plan 05-10 delta paragraphs in commits ae4d569 and 9cdd78c; cargo test -p xtask lintcontract rc 0 (13/13)"
        status: pass
    human_judgment: false

# Metrics
duration: 31min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 10: AArch64 SMP-03 Proc/Driver Migration + RunQueues Wiring Summary

**Every aarch64 single-CPU scheduler/proc/pager static migrated to TPIDR_EL1 PerCpu slots or ranked IrqSpinLocks (PROC_TABLE@50, VmShared@30, PageAlloc@70), Proc::wake_pending + block() closing the 2-CPU lost-wakeup race, pick_next consuming port::sched::RunQueues with work stealing, and the two IRQ-shared driver globals (CONS_RX@Console-100, VIRTIO_NET@NetIrq-90) locked with zero guard-held-across-callback violations.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-11-26T07:50:56Z
- **Completed:** 2026-11-26T08:22:38Z
- **Tasks:** 2 of 2 executed (+2 review-found fix commits)
- **Files modified:** 13 (0 created)

## Accomplishments

- **SMP-02 — the aarch64 per-CPU migration is complete in proc.rs/fpu.rs.**  `CURRENT_PROC`, `CURKSTACK`, `CURFRAME`, `SCHED_CONTEXT`, `SCHED_SEEDED` and the churn profile's `BOOT_CTX` statics are all gone: the current-proc pointer, trap-frame/entry-stack slots, scheduler context and park scratch now live in `smp::PerCpu` (TPIDR_EL1-indexed, the owning CPU only), and `fpu.rs`'s `this_cpu()` reads `&smp::this_cpu().fpu` (the MPIDR-indexed `CPUS` table and its `mrs` asm block are gone).  `PerCpu` gained `park_context` and `sched_stack_top` for the per-CPU scheduler-stack shape 05-22's secondary entry consumes.
- **SMP-03 — every audited global is a ranked IrqSpinLock, no BKL (D-22).**  `PROC_TABLE` at `LockRank::ProcTable` (50); `SHARED_PAGES`, `RFMEM_FAMILIES`, `SEM_WAITERS` at `LockRank::VmShared` (30); `PAGE_ALLOC` at `LockRank::PageAlloc` (70); `CONS_RX` at `LockRank::Console` (100); `VIRTIO_NET` at `LockRank::NetIrq` (90).  Zero "single CPU, IRQ-masked" safety justifications remain in the audited files — the rfmem family walks now argue from the break-move ordering and per-switch TLB invalidation instead.
- **SMP-03/SMP-11 — Pitfall 4 is closed by construction.**  `Proc::wake_pending: AtomicBool` is published (Release) inside every waker's one PROC_TABLE critical section (`wake_locked`: flag + Blocked→Ready + `enqueue_ready`), and `block()` consumes it (`swap(false, AcqRel)`) under the same lock before marking Blocked — a wake that lands between a proc's registration and its block() is consumed, never lost.  `prepare_block_current()` is wired into `aarch64/src/smp.rs`'s `SmpArch::prepare_block` hook (live once 05-22 calls `register_port_smp`).
- **SMP-11 — the scheduler runs the port run queues.**  `pick_next` pops pids via `RUN_QUEUES.pop_or_steal(my_cpu, num_online)` (local FIFO pop, neighbor steal via try_lock, idle-mask publish + re-check), and every Ready-producing transition enqueues exactly once with the idle-CPU reschedule-IPI report (a no-op on the default single-CPU hooks until 05-22).  The idle arm keeps the existing IRQs-briefly-unmasked wfi.
- **SMP-03 — the IRQ-shared driver globals are locked without rank inversions.**  `virtio_net_irq` was restructured into locked phases (ISR read-to-ack + TX retire; per-frame RX extraction into a 2048-byte stack buffer; replenish) so the guard is never held across `rx_ingress`/`tx_retired`/`note_rx_pending`, whose port-side locks are also NetIrq (90) — equal-rank nesting is a D-26 debug panic.  Publish-once cells (`PL011_RANGE`, `PCI_DISCOVERY`, `VIRTIO_NET_INTID`) deliberately stay `SyncUnsafeCell` (write-once-at-boot is not IRQ-shared-mutable state).
- **D-03 — UNSAFE-AUDIT.md moved in lockstep in both task commits:** aarch64 202 → 191 lint-visible blocks, SAFETY annotations 408 → 397; the per-file inventory rows and the encapsulated table's pl011/proc rows reflect the removed cell derefs.

## Task Commits

Each task was committed atomically:

1. **Task 1: proc/syscall/fpu/pagealloc PerCpu + ranked IrqSpinLock migration, RunQueues wiring** — `ae4d569` (feat) — proc.rs, smp.rs, syscall.rs, fpu.rs, pagealloc.rs, exec.rs, trap.rs, main.rs, l.S, churngate.rs, UNSAFE-AUDIT.md
2. **Task 2: CONS_RX + VIRTIO_NET ranked IrqSpinLocks** — `9cdd78c` (feat) — pl011.rs, pci.rs, UNSAFE-AUDIT.md
3. **Review fix: sys_sleep prepare_block arm** — `d52d944` (fix) — syscall.rs
4. **Review fix: churn driver enqueue at seed_kernel_context** — `4b1f07e` (fix) — proc.rs

## Files Created/Modified

- `aarch64/src/proc.rs` — the core: PerCpu-backed accessors, ranked locks, wake_pending + wake_locked + block(), RUN_QUEUES + enqueue_ready + pick_next rewrite, SMP-valid SAFETY wording
- `aarch64/src/smp.rs` — PerCpu + park_context/sched_stack_top fields; prepare_block → proc::prepare_block_current
- `aarch64/src/syscall.rs` — semacquire's prepare_block+register loop, sleep/await/rendezvous block() migration with the SMP-03 pairing comments
- `aarch64/src/fpu.rs` — this_cpu() → &smp::this_cpu().fpu; selftest slot comment
- `aarch64/src/pagealloc.rs` — PAGE_ALLOC IrqSpinLock at PageAlloc (70)
- `aarch64/src/exec.rs` — exec_into enqueues the frame-seeded proc (enqueue_ready)
- `aarch64/src/trap.rs` — note-boundary SAFETY comment re-pointed at PerCpu cur_frame
- `aarch64/src/main.rs` — boot-ordering comments re-pointed at the PerCpu slots
- `aarch64/src/l.S` — the MAXMACH comment now names smp.rs's MAX_CPUS
- `aarch64/src/platform/virt/churngate.rs` — sched() → block() renames + comment
- `aarch64/src/platform/virt/pl011.rs` — CONS_RX IrqSpinLock at Console (100); ring readers/writer/poll/gatedbg all take it
- `aarch64/src/platform/virt/pci.rs` — VIRTIO_NET IrqSpinLock at NetIrq (90); phased IRQ handler with unlocked wake/ingress; RX_INGRESS_CAP stack copy
- `UNSAFE-AUDIT.md` — Plan 05-10 Task 1 + Task 2 deltas, reconciliation table, per-file inventory rows

## Decisions Made

See `key-decisions` in the frontmatter.  The load-bearing one: **the wake_pending store lives INSIDE the waker's PROC_TABLE critical section, not before acquiring the lock.**  The plan's action text reads "set wake_pending = true (Release) in wake(pid) BEFORE acquiring PROC_TABLE"; implemented literally that order re-opens the race it exists to close — a blocker can consume the pre-lock flag, re-check before the waker's condition mutation (which happens under the lock) is visible to it, and re-block past a queued wake (traced in full in post_wait_record's doc comment).  With the store inside the section and block()'s consume in its own section, the lock serializes the two and Pitfall 4 has no interleave.  Also: no standalone `pub fn wake(pid)` exists — every pid-keyed wake funnels through `wake_locked` on an already-resolved `&mut Proc`, because a public pid-lookup wrapper would be dead code under the warning-free gate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] wake_pending store placed inside the PROC_TABLE critical section**
- **Found during:** Task 1 (block/wake design review)
- **Issue:** The plan's "set wake_pending in wake(pid) BEFORE acquiring PROC_TABLE" is unsound for proc-owned conditions (await's wait_records): the flag could be consumed by an early block() while the condition mutation (the record push) is not yet visible to the blocker, which then re-blocks past the queued record — a wedge.
- **Fix:** `wake_locked` stores the flag (Release) inside the PROC_TABLE section, before the Blocked check; post_wait_record pushes the record in the same section.  The plan's intent (a wake before the block commits is never lost) is preserved — the store still precedes the state transition and pairs against block()'s consume under the one lock.
- **Files modified:** aarch64/src/proc.rs
- **Verification:** full trace analysis in post_wait_record's doc comment; cargo xtask check/clippy/test all rc 0
- **Committed in:** `ae4d569`

**2. [Rule 3 - Blocking] Files beyond the plan's list needed small edits**
- **Found during:** Task 1
- **Issue:** The plan's files_modified names proc.rs/syscall.rs/fpu.rs/pagealloc.rs (+UNSAFE-AUDIT.md), but the migration's edges live in adjacent files: smp.rs's PerCpu needed the `park_context`/`sched_stack_top` fields (the BOOT_CTX/SCHED_CONTEXT migration targets) and its prepare_block stub needed the real dispatch; exec.rs's exec_into and proc.rs's seed_kernel_context (called from churngate.rs) are where newly schedulable procs must enter the run queue; churngate.rs/trap.rs/main.rs/l.S carried sched()-name and CURFRAME/CURKSTACK references.
- **Fix:** minimal edits in each (fields, the one-line dispatch, enqueue calls, comment renames).
- **Files modified:** aarch64/src/smp.rs, exec.rs, churngate.rs, trap.rs, main.rs, l.S
- **Verification:** cargo xtask check/clippy/test rc 0
- **Committed in:** `ae4d569`

**3. [Rule 1 - Bug] sys_sleep armed with prepare_block**
- **Found during:** post-commit review of the block() flag semantics
- **Issue:** sys_sleep blocks ONCE (no re-check loop) and reports an early return as "interrupted"; a stale wake_pending from an unrelated earlier wake would make block() return immediately and misreport.
- **Fix:** `port::smp::prepare_block()` before arming `sleep_until`.  Sound here specifically because the waker (tick_timeouts) re-evaluates the condition under PROC_TABLE every tick, so clearing the flag before arming cannot lose a wake.
- **Files modified:** aarch64/src/syscall.rs
- **Verification:** cargo xtask check/clippy/lintcontract rc 0
- **Committed in:** `d52d944`

**4. [Rule 3 - Blocking] seed_kernel_context enqueues the churn driver**
- **Found during:** post-commit review of the Ready⟺enqueued invariant (churngate driver creation path)
- **Issue:** the dev_churn orchestrator proc (Ready, blocked_in_kernel, no TrapFrame) was found by pick_next's old PROC_TABLE scan; under the run-queue pop it would never be picked and the virtchurn profile would idle-wedge.
- **Fix:** `seed_kernel_context` calls `enqueue_ready(p.pid)` — the seed is the moment the kernel-context proc becomes schedulable.
- **Files modified:** aarch64/src/proc.rs
- **Verification:** cargo xtask check/clippy rc 0 (the dev_churn cfg is not in the virtshell gate set; the change is the same call shape as every other enqueue site)
- **Committed in:** `4b1f07e`

---

**Total deviations:** 4 auto-fixed (2 bug, 2 blocking)
**Impact on plan:** All four are correctness/blocking necessities inside the plan's own scope; the one judgment call (deviation 1) preserves the plan's stated goal (no lost wakeups) against the plan's literal shorthand.  No scope creep.  `Cargo.lock` unchanged (zero external packages — T-05-SC gate N/A).

## Issues Encountered

- One compile error during Task 2 (`E0502` in pl011.rs's ring write through the IrqLockGuard — field-split borrows don't cross the guard's DerefMut); fixed by dereferencing once into a local.  One new clippy warning during Task 2 (a nested `if let`+`if` in the link-status arm, `collapsible_if`); fixed with a guard-return shape.  Both caught and fixed in-commit before the task commits.
- Pre-existing warnings observed and left untouched per the scope boundary: unused `FdGroup`/`MAX_FDS` imports in syscall.rs, the `///`-in-section-header typo above SEM_WAITERS (pre-existing at HEAD), proc.rs collapsible-if/manual-assign shapes, pci.rs's pre-existing cast/single_match/doc-indent warnings, and the port/xtask/ds warning sets.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None introduced.  Two intentional boundary states continue by design (unchanged by this plan's scope, both named in prior summaries):

- `register_port_smp()` is still not called from main9 (05-22's wiring) — so `port::smp`'s hooks stay the safe single-CPU defaults at runtime: `current_cpu_id()` = 0, `num_online_cpus()` = 1, `send_resched_ipi()` no-op, and `prepare_block()` dispatches to the real arch implementation only after registration.  The wake_pending machinery is fully wired and correct in this state (a stale flag costs one bounded spurious iteration in the looping callers; sys_sleep's arm clears it explicitly).
- CPU 1 does not schedule yet (05-22 boots it); the scheduler is SMP-shaped and rank-checked but currently runs one CPU.

The broken-windows ledger (`.planning/WINDOWS.md`) has no entries to add: no new stubs, no skipped tests, no unrun `<verify>`; every deviation was fixed and verified in-commit.

## Threat Flags

None new.  The plan's threat register is implemented as written: T-05-17 (single-CPU statics in proc/pagealloc/fpu/syscall) is mitigated by the PerCpu migration + ranked IrqSpinLocks; T-05-38 (CONS_RX/VIRTIO_NET IRQ-shared mutation) is mitigated by the Console(100)/NetIrq(90) IrqSpinLocks with the no-guard-across-callbacks phasing; T-05-SC: `Cargo.lock` is byte-unchanged (verified) — zero external packages installed.  The pci.rs IRQ handler's new 2048-byte stack copy is bounded by DRAIN_CAP per invocation and the over-cap fault path; no new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes.

**Recorded concern for 05-22 (not a regression, a pre-existing donor shape the 2-CPU gate will exercise):** the sem word's check-then-decrement fast path in `sys_semacquire` is a user-memory read-modify-write (the donor syssemacquire shape).  On two CPUs a racing pair can lose a decrement — the contended path's register/block/wake retry loop absorbs contended *misses*, but a genuinely lost unit is a mutual-exclusion question for the churnsem leg of the 05-22 `-smp 2` gate.  The comment on `sem_try_acquire` now describes the mechanism honestly; if the gate trips, the fix is a user-memory CAS primitive in usermem (LDAXR/STLXR), which is an architectural addition for that plan to schedule.

## Next Phase Readiness

- **Plan 05-11/05-12 (x86_64 twin):** the same migration shape applies to x86_64/src/proc.rs (GS-base Mach per-CPU slots) and x86_64/src/platform/q35/{uart16550,pci}.rs; the port-side contract (prepare_block call sites, RunQueues, IrqSpinLock) is already shared.
- **Plan 05-22 (aarch64 -smp 2 boot):** call `smp::register_port_smp()` + `smp::start_secondaries(dt)` from main9, replace `secondary_main` with the real bring-up tail (GIC secondary init, timer arm, per-CPU `sched_stack_top` seed, then sched_loop entry), and handle SGI 0/SGI 1 in trap.rs's IRQ dispatch (SGI 0 = reschedule: ack/eoi and let the sched loop re-pop; SGI 1 = `port::smp::service_shootdown_ipi`).  The enqueue→IPI path goes live the moment registration lands.
- **Owner Stop 2 (D-32):** the cumulative diff now includes this plan's proc.rs/syscall.rs/fpu.rs/pagealloc.rs/pl011.rs/pci.rs migration on top of 05-07/05-08/05-09; gates move to `-smp 2` after both arch bring-ups.

---

*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 13 modified files exist on disk (verified with `[ -f ]` equivalents through the git index): `aarch64/src/{proc,smp,syscall,fpu,pagealloc,exec,trap,main}.rs`, `aarch64/src/l.S`, `aarch64/src/platform/virt/{churngate,pl011,pci}.rs`, `UNSAFE-AUDIT.md`.
- All 4 plan commits are ancestors of HEAD: `ae4d569` (T1), `9cdd78c` (T2), `d52d944` (fix), `4b1f07e` (fix).
- Content pins verified: zero `CURRENT_PROC`/`CURKSTACK`/`CURFRAME`/`SCHED_CONTEXT`/`SCHED_SEEDED`/`BOOT_CTX` statics in proc.rs (grep = 0 matches); `PROC_TABLE`/`SHARED_PAGES`/`RFMEM_FAMILIES`/`SEM_WAITERS` = IrqSpinLock in proc.rs; `PAGE_ALLOC` = IrqSpinLock at PageAlloc in pagealloc.rs; `wake_pending: AtomicBool` on Proc; `RUN_QUEUES: port::sched::RunQueues<64>` + `pop_or_steal` in pick_next; `CONS_RX` = IrqSpinLock at Console in pl011.rs; `VIRTIO_NET` = IrqSpinLock at NetIrq in pci.rs.
- Measured commits from ledger (`gsd-plan-head-before-05-10` = `41a7f8d3`): 4 at completion; `plan_head_after` = `4b1f07e4`.
- Working tree clean before this SUMMARY's commit (only this SUMMARY untracked); `Cargo.lock` byte-identical (zero external packages — T-05-SC).
- Plan-level verification re-run at completion: `cargo xtask check` rc 0, `cargo xtask clippy --arch aarch64 --config virtshell` rc 0, `cargo test -p xtask lintcontract` rc 0 (13/13); additionally `cargo xtask test` rc 0 (all host legs, including both aarch64 host-test legs) and `cargo test --package aarch64 --bins --target aarch64-apple-darwin` 72/72.
