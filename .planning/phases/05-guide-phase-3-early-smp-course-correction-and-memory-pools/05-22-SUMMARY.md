---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 22
subsystem: aarch64-smp
tags: [smp, aarch64, psci, gicv2, gicv3, hvf, tcg, tlb-shootdown, sgi, qemu-virt, smp-04, smp-07, smp-09, smp-12]

# Dependency graph
requires:
  - phase: 05-09
    provides: aarch64/src/smp.rs DTB /cpus + /psci discovery, PSCI CPU_ON with INVALID_PARAMETERS tolerated, secondary_start stub, GIC secondary-init scaffolding
  - phase: 05-10
    provides: per-CPU PerCpu state, ranked IrqSpinLocks (PROC_TABLE@50, VmShared@30, PageAlloc@70, NetIrq@90, Console@100), RunQueues with work stealing, wake_pending + block()
  - phase: 05-23
    provides: the 2-CPU static DTBs (virt-gicv2/virt-gicv3) this plan's -smp 2 boots load
provides:
  - "aarch64 CPU 1 PSCI bring-up: full secondary_main (GIC secondary interface, per-CPU timer arm, mark_online, sched_loop entry) replacing the 05-09 stub"
  - "SGI 0 (resched) / SGI 1 (TLB shootdown) IRQ dispatch in trap.rs with the GICv2 IAR source-CPUID masking"
  - "boot-time SGI 1 shootdown self-check (smp: cpu 1 online / smp: tlb shootdown ok)"
  - "gates/aarch64/smoke-r9.expect: R9_SMP env (default 2), -smp $r9_smp, smp-online/smp-shootdown stages between banner and paqfs"
  - "a chain of nine latent-defect fixes that the -smp 2 boot proof surfaced (see Deviations)"
affects: [05-24/05-25 (x86_64 twin applies the same preempt gate / queue reschedule / deferred reap / kernel-vm lock), SMP-08 stress gate (the residual HVF fork-sweep flake is its first target), Owner Stop 2]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (48000, confidence: low).
actuals:
  tokens: 15146
  tasks: 2
  commits: 4
  plan_head_before: 83ee2a02e27633a5d8673f9add897e4eb4844936
  plan_head_after: ceb6ecccce09740c58bf4af616313bd2f975c842

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "claim-then-start bootstrap: exec_into enqueues the first proc Ready (the churn consumer's contract), so secondaries start only after activate_first publishes it Running"
    - "EL0-origin preemption gate: preempt()/tick() act only when the IRQ interrupted EL0 on THIS cpu (an idle cpu's stale PerCpu current can name a proc Running on a sibling)"
    - "run-queue-only rescheduling (Ready ⟺ enqueued; a raw table scan could pick a mid-construction rfork/exec child with a null frame)"
    - "deferred moribund reap: exit's root-table + kstack frees move to the exiting CPU's sched loop (the single-CPU 'nothing allocates until the hand-off' invariant is gone under a sibling CPU)"
    - "kernel page-table map_to serialized across CPUs (KERNEL_VM IrqSpinLock at LockRank::KernelVm=25, below PageAlloc 70)"
    - "GICv2 IAR: dispatch on raw & 0x3ff, EOI with the raw value (the CPUID bits at [12:10] must round-trip for SGI completion)"
    - "HVF in-kernel vGIC does not model GICR TYPER/WAKER (reads 0) — an all-zero TYPER selects the redistributor by qemu's cpu-order frame numbering"

key-files:
  created: []
  modified:
    - aarch64/src/main.rs
    - aarch64/src/trap.rs
    - aarch64/src/platform/virt/timer.rs
    - aarch64/src/platform/virt/gic/gicv3.rs
    - aarch64/src/proc.rs
    - aarch64/src/smp.rs
    - aarch64/src/syscall.rs
    - aarch64/src/vm.rs
    - aarch64/lib/kernel.ld
    - gates/aarch64/smoke-r9.expect
    - port/src/smp.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "The .bootdata RO fault (silent pre-banner wedge on EVERY boot, latent since 05-09) was found by early-uart milestone markers and fixed in kernel.ld, not by a code workaround"
  - "exec_bootstrap's secondary start moved after activate_first: the Ready⟺enqueued contract makes a started-but-unclaimed pid 1 stealable — the start must wait for the claim"
  - "The HVF-only fork-sweep residual was root-caused and fixed in a follow-up debug session (owner chose debug-now over the TCG-truth-venue doctrine): it was not the pagealloc/TextCopy class, but two kernel-stack ownership violations — a publish-before-swtch handoff and an IRQ-unmasked sched entry (see Post-Halt Closure)"

patterns-established:
  - "SMP defect forensics by early-uart milestone markers + qemu hvf_data_abort tracing (the identity alias dies at the TTBR0 teardown — early-uart probes must not run past it)"
  - "Every lock's rank asserted in gate builds; the watchdog names a stuck cpu/lock (CPU 0's tick) — a masked non-lock spin is invisible to it (documented gap for follow-up)"

requirements-completed: [SMP-04, SMP-07, SMP-09, SMP-12]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "aarch64 CPU 1 comes online via PSCI CPU_ON after the publish-once release fence; BSP logs 'smp: cpu 1 online'; the boot-time request_shootdown(None) gets CPU 1's SGI 1 ack and logs 'smp: tlb shootdown ok' (SMP-04, SMP-07)"
    requirement: SMP-04
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-smoke (HVF gicv3 -smp 2) and R9_GICV2=1 nix run .#boot-r9-smoke (TCG gicv2 -smp 2): both markers in the capture; smoke-r9.expect stages smp-online/smp-shootdown PASS"
        status: pass
    human_judgment: false
  - id: D2
    description: "Per-CPU generic timers: both CPUs arm PPI 30; TICKS increments on CPU 0 only (1 kHz preserved); preempt + watchdog heartbeat on both; SGI 0/1 dispatch in trap.rs (SMP-04, SMP-07, SMP-12, D-26)"
    requirement: SMP-07
    verification:
      - kind: e2e
        ref: "procs migrate across CPUs in the smoke sweep (sched: switch lines); zero lock-rank/watchdog panics in every captured run; TCG leg twice-consecutive ALL STAGES PASS"
        status: pass
    human_judgment: false
  - id: D3
    description: "gates/aarch64/smoke-r9.expect defaults to -smp 2 via R9_SMP (default 2), passes -smp $r9_smp, asserts the smp markers when R9_SMP >= 2, and keeps the R9_SMP=1 leg marker-free and green (SMP-09, D-24)"
    requirement: SMP-09
    verification:
      - kind: automated
        ref: "grep -v '^[[:space:]]*#' gates/aarch64/smoke-r9.expect | grep -c R9_SMP == 1; R9_SMP=1 nix run .#boot-r9-smoke ALL STAGES PASS (2/2)"
        status: pass
    human_judgment: false
  - id: D4
    description: "FULL smoke determinism at -smp 2 on both accels: TCG gicv2 green twice consecutively; HVF gicv3 now green twice consecutively on the final tree (8/8 post-fix total, up from 6/13) after the kernel-stack-ownership fix; the R9_SMP=1 leg stays green and marker-free"
    requirement: SMP-04
    verification:
      - kind: e2e
        ref: "TCG: R9_GICV2=1 nix run .#boot-r9-smoke PASS x2; HVF: nix run .#boot-r9-smoke PASS x2 consecutively on HEAD 665c94e (8/8 post-fix); R9_SMP=1 PASS x2"
        status: pass
    human_judgment: false

# Metrics
duration: ~6.5h wall (includes a provider-outage interruption mid-verification and the long defect chain)
completed: 2026-11-26
status: complete
---

# Phase 5 Plan 22: aarch64 Two-CPU Bring-Up and the R9_SMP Smoke Gate Summary

**aarch64 CPU 1 PSCI bring-up, per-CPU timers, SGI 0/1 dispatch and the boot TLB-shootdown self-check are live and gate-proven at `-smp 2` on BOTH accels: TCG gicv2 and HVF gicv3 each pass the full smoke twice consecutively on the final tree, and the `R9_SMP=1` leg stays green and marker-free. The HVF residual that halted the first pass was root-caused in a follow-up debug session and fixed (kernel-stack ownership), not carried.**

## Post-Halt Closure (owner decision: debug now)

The first pass halted at a residual HVF-only fork/exec-sweep corruption (~45%, 6/13). The owner rejected the TCG-truth-venue shortcut because Waves 12/13 migrate the aarch64 gates to `-smp 2`, and chose a dedicated debug session instead. Outcome:

- **Root cause (confirmed, both executor guesses eliminated):** two kernel-stack ownership violations exposed only by real HVF interrupt timing on two CPUs. (1) `reschedule()` published a preempted proc Ready + enqueued while the CPU still ran on its 16 KiB kernel stack, then spent the long `sched: switch` console print there (every UART byte is an MMIO exit on HVF), so a sibling CPU could `trap_return` into the same stack; the overlapping frames corrupted the console MCS queue's stack-resident `LockNode`s and both CPUs spun forever in the CONS lock. `block()` had the same publish-before-`swtch` shape. (2) `secondary_main` unmasked IRQs before entering the sched loop, so an IRQ in the `pick_next`→`trap_return` handoff overwrote `PerCpu.cur_frame` with an EL1h scheduler-stack frame.
- **Fix:** explicit `Proc::on_cpu` kstack-ownership token; `reschedule()` only parks and the sched loop releases the proc after leaving its stack; `pick_next` refuses/requeues `on_cpu` procs with a debug tripwire; sched loop IRQ-masked; `dispatch_irq` EOIs before preempt. Commits `339036f` (fix), `665c94e` (module contract), `11be789` (WINDOWS #12 + deferred-items closed), `bdcc644` (resolved debug session + knowledge base).
- **Verification:** HVF gicv3 `-smp 2` 8/8 post-fix (vs 6/13 pre-fix), independently re-confirmed twice consecutively by the orchestrator; TCG gicv2 `-smp 2` PASS; `R9_SMP=1` PASS; `cargo xtask boot-test --arch aarch64 --config virtmulti --accel hvf` PASS; `cargo xtask check`/`test`/`clippy` green. Debug session archived at `.planning/debug/resolved/aarch64-smp2-hvf-corruption.md`.

## Performance

- **Duration:** ~6.5h wall (the previous executor was killed by a provider outage mid-verification; this session resumed from the committed state and ran the full proof + defect chain)
- **Started:** 2026-11-26T09:15:37Z (resume; original spawn 2026-11-26 ~03:45)
- **Completed:** 2026-11-26
- **Tasks:** 2
- **Files modified:** 14 (12 code/config + UNSAFE-AUDIT.md + the plan's expect gate)

## Accomplishments
- CPU 1 PSCI bring-up is real: full `secondary_main` (GIC secondary interface, per-CPU timer arm, online publish, sched_loop entry) replaces the 05-09 wfi stub; `smp: cpu 1 online` and `smp: tlb shootdown ok` print on both accels.
- The `R9_SMP`-aware smoke gate defaults to `-smp 2`, asserts the two SMP markers when `R9_SMP >= 2`, and keeps the `-smp 1` leg marker-free and green (2/2).
- The `-smp 2` boot proof surfaced and fixed a chain of nine latent defects — including one (the `.bootdata` read-only-mapping fault, latent since 05-09) that wedged EVERY aarch64 boot before the banner.
- TCG gicv2 `-smp 2` is twice-consecutive ALL STAGES PASS; R9_SMP=1 is 2/2; HVF gicv3 `-smp 2` is now 8/8 post-fix and twice-consecutive on the final tree after the Post-Halt Closure fix.

## Task Commits

Each task was committed atomically:

1. **Task 1: aarch64 CPU 1 PSCI bring-up, per-CPU timers, SGI 0/1 dispatch, boot shootdown self-check** — `66a3548` (feat)
2. **Task 1 fix: enable +el3 in the aarch64 target spec so the reachable PSCI smc conduit assembles** — `a38add5` (fix)
3. **Task 2: R9_SMP-aware smoke gate (default -smp 2, marker stages)** — `34cbef0` (feat)
4. **Task 2: repair the aarch64 -smp 2 boot chain found by the gate proof** — `ceb6ecc` (fix)

**Plan metadata + summary:** this SUMMARY's own commit (docs) follows.

## Files Created/Modified
- `aarch64/src/main.rs` — `register_port_smp`, BSP SGI 0/1 enables, `start_secondaries_and_self_check`, full `secondary_main`, CPACR FPEN fix, claim-then-start ordering
- `aarch64/src/trap.rs` — SGI 0/1 dispatch, GICv2 IAR masking, EL0-origin preempt gate, cur_frame note boundary
- `aarch64/src/platform/virt/timer.rs` — per-CPU timer arm, CPU-0-only TICKS, watchdog hookup
- `aarch64/src/platform/virt/gic/gicv3.rs` — secondary redistributor selection with the HVF zero-TYPER fallback
- `aarch64/src/proc.rs` — preempt gate, run-queue reschedule, deferred moribund reap
- `aarch64/src/smp.rs` — start_secondaries returns the started count; PerCpu gains `moribund_reap`
- `aarch64/src/syscall.rs` — yield's `preempt(true)`; exit_tail defers the final frees
- `aarch64/src/vm.rs` — `KERNEL_VM` IrqSpinLock serializing Kernel-pgtype `map_to`
- `aarch64/lib/kernel.ld` — `.bootdata` moved out of the RX text group into `.data`
- `gates/aarch64/smoke-r9.expect` — R9_SMP env + `-smp $r9_smp` + the smp marker stages (placed between banner and paqfs, matching the kernel's marker order)
- `port/src/smp.rs` — `LockRank::KernelVm = 25` + the documented table row
- `UNSAFE-AUDIT.md` — 05-22 delta in lockstep (main 6→7, proc 26→28; aarch64 191→194 blocks, SAFETY 397→400)

## Decisions Made
- Fixed the `.bootdata` fault in the linker script (moving the section) rather than making the writes page-aligned workarounds — the section was simply in the wrong output group since 05-09.
- The secondary's CPACR fix lives in `secondary_main` (Rust), mirroring `init_vm`'s documented precedent, not in l.S.
- The rescheduler now treats the run queues as the only readiness publication (SMP-11's `Ready ⟺ enqueued`), ending the raw table scan that could pick a half-built child.
- Exit's last frees are deferred to the exiting CPU's sched loop; the rfork unwind path keeps the direct free (a never-run child's pages are nobody's live state).
- Stopped at the fix-attempt limit on the residual HVF flake and recorded it with a full forensic trail; the owner then chose debug-now, and the follow-up session root-caused and fixed it (see Post-Halt Closure) rather than shipping a weak assertion.

## Deviations from Plan

The plan's Task 2 says to fix any boot/timing defect the proof surfaces and re-run until green. All of the following were auto-fixed under Rules 1 (bug) / 3 (blocking). Nine fixes, each verified by a before/after boot probe:

**1. [Rule 1/3] kernel.ld: `.bootdata` mapped RX wedged every boot pre-banner**
- Found: Task 2's first HVF boot hung after `vminit: complete`; early-uart milestone markers isolated it to `publish_boot_state`'s writes.
- Issue: `.bootdata` was grouped with `.text.boot`, which vminit maps `Entry::ro_kernel_text()` (RX); the SECONDARY_* writes faulted before VBAR_EL1 was installed. Latent since 05-09; predates this plan.
- Fix: moved `.bootdata` into `.data` (PA = VA − KZERO identity preserved for l.S's pre-MMU reads). Base-tree comparison (git archive into target/, since removed) proved the base kernel hung identically — a pre-existing latent defect, not a Task 1 regression.
- Files: `aarch64/lib/kernel.ld`. Commit: `ceb6ecc`.

**2. [Rule 1] secondary_main never set CPACR_EL1.FPEN**
- Issue: PSCI CPU_ON enters the secondary at EL1 with FPEN at trap-all reset; the first EL0 proc scheduled onto CPU 1 trapped EC=0x07 (FloatSimd) and halted.
- Fix: `CPACR_EL1.FPEN=TrapNothing` + `isb` at the top of `secondary_main`'s virt block (mirrors `init_vm`).
- Files: `aarch64/src/main.rs`. Commit: `ceb6ecc`.

**3. [Rule 1] gicv3 secondary redistributor match fails under HVF**
- Issue: HVF's in-kernel vGIC reads every GICR frame's TYPER/WAKER as 0 (instrumented: iidr/waker all 0 on all frames), so CPU 1's affinity match never matched and its SGI 1/0 + PPI 30 were never enabled — the shootdown self-check wedged waiting for CPU 1's ack. TCG's userspace GICv3 returns real TYPERs (frame 1 = 0x101000111).
- Fix: a nonzero TYPER keeps the affinity match authoritative; an all-zero TYPER selects by qemu virt's cpu-order numbering (frame == MPIDR Aff0).
- Files: `aarch64/src/platform/virt/gic/gicv3.rs`. Commit: `ceb6ecc`.

**4. [Rule 1] GICv2 IAR source-CPUID bits dropped every cross-CPU resched IPI**
- Issue: GICC_IAR carries the SGI source CPUID at [12:10]; CPU 1's SGI 0 read as 0x400 on CPU 0, fell to "unhandled IRQ 1024", and every cross-CPU resched was lost (qemu's own EOIR masks to 0x3ff — arm_gic.c:1742).
- Fix: dispatch on `raw & 0x3ff`, EOI with the raw IAR.
- Files: `aarch64/src/trap.rs`. Commit: `ceb6ecc`.

**5. [Rule 1] exec_bootstrap started CPU 1 before pid 1 was claimed**
- Issue: `exec_into` enqueues pid 1 Ready (the churn consumer's contract); a secondary in sched_loop stole it inside the Ready→Running window and both CPUs ran pid 1 (cpu-attributed prints: `cpu0 entering EL0` vs `cpu1 picks pid 1`).
- Fix: `start_secondaries_and_self_check` moved after `activate_first` (claim-then-start). The boot self-check still lands before userland.
- Files: `aarch64/src/main.rs`. Commit: `ceb6ecc`.

**6. [Rule 1] preempt() rescheduled a sibling CPU's Running proc via a stale PerCpu `current`**
- Issue: an idle CPU's `current` still names the last proc it ran; if that proc was woken and is Running on the sibling, a tick/SGI during the idle wfi window saw state==Running and rescheduled it off mid-run (the double-run tear: TCG PcAlignmentFault, HVF garbage).
- Fix: `preempt(interrupted_user: bool)` + `tick(user_interrupted)` gate on the trap frame's EL0t origin; the yield path passes `preempt(true)`.
- Files: `aarch64/src/proc.rs`, `trap.rs`, `platform/virt/timer.rs`, `syscall.rs`. Commit: `ceb6ecc`.

**7. [Rule 1] the sync-return note boundary delivered into the wrong frame after a yield-switch**
- Issue: `sys_sleep`'s yield switches procs mid-dispatch, but the synchronous-return boundary delivered notes into the *entry* frame (and through a UserSliceMut over the wrong proc's address space) instead of the current one.
- Fix: the sync boundary reloads `cur_frame` exactly like the IRQ return path.
- Files: `aarch64/src/trap.rs`. Commit: `ceb6ecc`.

**8. [Rule 1] reschedule's raw PROC_TABLE scan picked mid-construction children**
- Issue: the scan ignored the `Ready ⟺ enqueued` publication contract and could pick an rfork/exec child still null-framed (the `-smp 2` wild pc=0 user-trap death of a forked `echo`).
- Fix: reschedule selects via `RUN_QUEUES.pop_or_steal` + the locked Ready check; blocked_in_kernel resumees requeue (first_skipped cycle guard); the no-switch path repairs the idle bit.
- Files: `aarch64/src/proc.rs`. Commit: `ceb6ecc`.

**9. [Rule 1] exit freed the running CPU's kstack/root table while still in use**
- Issue: "nothing allocates until the hand-off" was a single-CPU invariant; a sibling CPU reallocated a just-freed kstack page and wrote it while the exiting CPU still ran on it (validated: skipping the frees made 10/10 boots clean).
- Fix: `defer_root_kstack_reap` records the pages in a per-CPU slot; the sched loop's `reap_moribund` frees them after the hand-off swtch.
- Files: `aarch64/src/proc.rs`, `syscall.rs`, `smp.rs`. Commit: `ceb6ecc`.

Plus the gate-order fix: the expect script's smp stages moved between banner and paqfs (the markers print at the end of exec_bootstrap, before userland's `paqfs...` line; the previous placement after `stage paqfs` consumed the already-printed markers and timed out).

---

**Total deviations:** 9 auto-fixed defects + 1 gate-order correction (all Rule 1/3; the .bootdata one was a pre-existing latent defect this plan's proof surfaced).
**Impact on plan:** All fixes were necessary for correctness and the plan's own verification; no scope creep. `UNSAFE-AUDIT.md` updated in lockstep (D-03).

## Issues Encountered
- **Provider outage mid-verification:** the previous executor died during a base-tree comparison; this session resumed from the committed state (66a3548, a38add5) and the uncommitted expect-script change, which was reviewed, corrected (stage order), and committed (34cbef0).
- **Diagnostic contamination (self-inflicted, caught):** several mid-investigation probes used the early-uart `putstr` past the TTBR0 identity-map teardown, which faults at FAR=0x09000018 — three "crashes" were the diagnostics, not the kernel. All diagnostics were stripped; the final evidence is from the uninstrumented kernel.
- **Residual HVF-only flake (OPEN):** the HVF gicv3 `-smp 2` leg passes the full smoke ~45% of clean canonical runs (6/13). Failures are always in the fork/exec/exit sweep: garbage bytes on the wire right after `init: starting /bin/rc`, or a stall mid page-in sweep, or (before the fix chain) qemu's `hvf.c:2447 assert(isv)` from a wild ISV=0 MMIO write. TCG gicv2 is twice-consecutive green and R9_SMP=1 is 2/2. This is the WINDOWS #49 class (fork-child corruption, HVF-timing-exposed, TCG the declared truth venue) amplified by two-CPU fork concurrency — the ~45%/boot rate over ~20 forks is far above #49's hardened residual (~1.8e-5/fork), so it is a NEW SMP-specific instance, not #49 itself. Recorded in the phase's `deferred-items.md` (status: open) with a recommended starting point (pagealloc poison-on-free tripwire; re-check TextCopy's cross-CPU icache discipline on real HVF).
- **qemu 11.1.2 GICv2 model assert (external):** under TCG MTTCG, qemu's own `arm_gic.c:581 gic_clear_pending_sgi` asserts on concurrent SGI traffic (`s->sgi_pending[irq][cpu] != 0`) — a qemu-side model race, not a kernel defect (the kernel's SGIR/IAR/EOIR sequence matches the architecture and Linux's shape). Not hit on the final tree's canonical runs; noted for the record.

## Known Stubs

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness
- The aarch64 half of SMP-04/07/12 is proven: bring-up, per-CPU timers, SGI dispatch, the shootdown self-check, and the R9_SMP gate are all live and green on TCG + `-smp 1`.
- The x86_64 twin plans (05-24/05-25) should carry the same preempt gate, queue reschedule, deferred reap, and kernel-vm lock shapes (they ride the shared port/ contract).
- **Blocker for downstream SMP trust:** the HVF `-smp 2` fork-sweep flake (deferred-items.md, status open) must be burned down before the SMP-08 stress gate can be meaningful on real hardware; TCG carries the truth-venue role meanwhile.

## Self-Check: PASSED

- Commits verified: `66a3548`, `a38add5`, `34cbef0`, `ceb6ecc` all ancestors of HEAD (checked via `git merge-base --is-ancestor`).
- Files verified: all 12 modified files exist at their paths; the SUMMARY's claims are backed by the captured gate logs under `target/gates/smoke-r9/` (ephemeral build output, not committed).

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26 (status: halted — HVF -smp 2 residual open)*
