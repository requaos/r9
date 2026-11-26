---
status: resolved
trigger: "aarch64 -smp 2 smoke under HVF (gicv3) passes only ~45% (6/13); failures are fork/exec-sweep corruption. TCG gicv2 -smp 2 is twice-consecutive green and R9_SMP=1 is solid. Debug now per owner decision."
created: 2026-11-26T15:10:00.000Z
updated: 2026-11-26T16:40:00.000Z
---

## Current Focus
<!-- OVERWRITE on each update - always reflects NOW -->

hypothesis: "CONFIRMED (root cause): (1) `reschedule()` publishes the preempted proc Ready+enqueued while the preempting CPU still runs on its kstack (through the `sched: switch` console print) — a sibling CPU steals it and traps on the same kernel stack (two CPUs, one kstack, overlapping frames → corrupted console MCS queue → `paqfs...s` deadlock / wild MMIO); (2) secondary_main unmasks IRQs before the sched loop, so an IRQ in the pick_next→trap_return handoff rewrites PerCpu.cur_frame with an EL1h frame from the scheduler stack and the tail restores it (SP_EL1 drifts into a kstack the CPU does not own)"
test: "DONE — on_cpu kstack-ownership token + sched-loop release handoff + IRQ-masked sched loop; all venues re-run on the final tree"
expecting: "DONE — HVF -smp 2 8/8 post-fix (was 6/13), TCG 2/2, R9_SMP=1 2/2, virtmulti boot-test PASS, check/test/clippy green"
next_action: "AWAITING the owner's real-workflow confirmation, then archive: move this file to .planning/debug/resolved/, append the knowledge-base entry, and mark 05-22 complete"
bug_class: concurrency
reasoning_checkpoint:
  hypothesis: "Two CPUs end up on one kernel stack because a preempted (or block-window-woken) proc is published runnable before its owning CPU has left the stack, and because the secondary's sched loop is interruptible so a handoff-window IRQ replaces PerCpu.cur_frame with a scheduler-stack EL1h frame"
  confirming_evidence:
    - "qemu monitor at the hang: both CPUs in port::mcslock::Lock<Option<&dyn Uart>>::lock (CONS), queue tail = a LockNode on a kstack"
    - "PerCpu dump: CPU1's cur_frame (0xffff90000016da20) inside CPU0's kstack [0x16a000,0x16e000) while its cur_kstack named another 16 KiB stack"
    - "an earlier hang: both CPUs' live SPs 0x800 apart in the SAME 16 KiB kstack (print frame overlapping trap-handler frames)"
    - "the new pick_next tripwire fired on the first boot of the first fix build, naming a real publish violation (stale-current release)"
    - "post-fix: HVF 8/8 vs pre-fix 6/13; TCG 2/2; R9_SMP=1 2/2"
  falsification_test: "If the hypothesis were wrong, an unmodified kernel would show no cross-CPU cur_kstack/cur_frame aliasing at a hang, and the fixed kernel would still fail at the pre-fix rate; instead the aliasing is directly observable and the failure rate drops to 0/8"
  fix_rationale: "The root cause is the publish/ownership window, so the fix makes kstack ownership explicit (Proc::on_cpu) and moves the Ready+enqueue publish to the sched loop's release, which provably runs after the CPU has switched off the stack; the IRQ-mask fix restores the sched loop's documented context discipline"
  blind_spots: "8 HVF runs cannot prove a sub-1% residual; the fix relies on the sched loop always running on the scheduler stack (verified: sched_context seeds it); the x86_64 twin still publishes Ready before its swtch but has no AP bring-up yet"
  candidate_causes:
    - "code: reschedule/block publish before the stack handoff (confirmed)"
    - "config: none (the same kernel+DTB+gate is green on TCG)"
    - "environment: HVF's real interrupt timing/MMIO exits only widen the windows — not a cause in themselves"
    - "data: none (no pagealloc/icache defect: both probes eliminated)"
  and_gate: "yes — both conditions contribute: the publish window (needs 2 procs ready + a preempting tick) AND the HVF timing that makes the window long; the unmasked sched loop is an independent second contributor that can corrupt cur_frame even when the publish path is careful"
tdd_checkpoint: null

## Session Loop

goal: find_and_fix
tdd_mode: false
tdd_phase: none
resume_count: 1
awaiting: none
question: none
last_next_action: "final-tree gates all green; awaiting the owner's real-workflow confirmation"

## Symptoms
<!-- Written during gathering, then immutable -->

expected: "aarch64 -smp 2 smoke under HVF (gicv3, -cpu host) reaches the shell cleanly twice consecutively, logging `smp: cpu 1 online` and `smp: tlb shootdown ok` with zero rank/watchdog panics"
actual: "HVF -smp 2 passes only ~45% (6/13 canonical runs). Failures always sit in the fork/exec/exit sweep: garbage bytes on the serial wire right after `init: starting /bin/rc` (e.g. `sched: switch` interleaved with 8-bit garbage), a mid page-in sweep stall, or (pre-fix-chain) qemu `hvf.c:2447 assert(isv)` from an ISV=0 MMIO write (wild pointer to a device PA)"
errors: "No kernel panic text. Diagnostic signatures: torn serial output, mid-sweep hang, and qemu's hvf.c:2447 assert(isv) before the 05-22 fix chain"
reproduction: "nix run .#boot-r9-smoke (HVF, gicv3, -smp 2) repeatedly; ~45% failure rate over full smoke runs (~20 forks per boot). TCG: R9_GICV2=1 nix run .#boot-r9-smoke passes twice consecutively. Single CPU: R9_SMP=1 nix run .#boot-r9-smoke passes 2/2"
started: "Introduced with two-CPU scheduling: 05-22 is the first plan where CPU 1 runs user processes. TCG does not expose it; HVF timing does. The class is adjacent to WINDOWS #49 (fork-child corruption, HVF-timing-exposed) but measured ~50%/boot versus #49's ~1.8e-5/fork, so it is a new SMP-specific instance"

## Eliminated
<!-- APPEND only - prevents re-investigating after /clear -->

- hypothesis: "The HVF failure is the TCG-only defect / a TCG misconfiguration"
  evidence: "TCG gicv2 -smp 2 is twice-consecutive ALL STAGES PASS; R9_SMP=1 passes 2/2 on the same build"
  timestamp: 2026-11-26T15:10:00.000Z

- hypothesis: "Lock-rank inversion or a watchdog stall is the direct cause"
  evidence: "The failing runs show no rank or watchdog panic; the 05-22 SUMMARY reports zero rank/watchdog panics in every HVF failure"
  timestamp: 2026-11-26T15:10:00.000Z

- hypothesis: "Pagealloc hands out a page that is still referenced (the pagealloc poison-on-free probe)"
  evidence: "The hang reproduces with both CPUs spinning in the CONSOLE MCS lock's wait loop (`port::mcslock::Lock<Option<&dyn Uart>>::lock`), with the lock queue pointing at a LockNode on a kernel stack — a stack/frame ownership failure, not a pagealloc double-free; the console lock's nodes live on kstacks"
  timestamp: 2026-11-26T15:45:00.000Z

- hypothesis: "I-cache incoherence / TextCopy's cross-CPU sync is the cause"
  evidence: "The boot-time selftest reports 64/64 fresh (control 64/64 stale) on the failing venue; the failure signature is a kernel-stack/frame collision (below), not stale instructions"
  timestamp: 2026-11-26T15:45:00.000Z

## Evidence
<!-- APPEND only - facts discovered during investigation -->

- timestamp: 2026-11-26T15:10:00.000Z
  checked: "05-22 SUMMARY + phase deferred-items.md + WINDOWS #12"
  found: "Failure signature is confined to the fork/exec/exit sweep; pre-fix-chain failures included qemu hvf.c:2447 assert(isv) (ISV=0 MMIO write = wild pointer to a device PA). Nine latent boot defects were already fixed in ceb6ecc; the residual survives that chain"
  implication: "Start from memory ownership (pagealloc reuse) and copy/icache ordering, not from the scheduler queues (those are exercised green on TCG)"

- timestamp: 2026-11-26T15:10:00.000Z
  checked: "Suggested next probes recorded by the 05-22 executor"
  found: "pagealloc poison-on-free tripwire, and a TextCopy cross-CPU icache/dcache review under real HVF"
  implication: "Two concrete, falsifiable probes exist; run them before broad instrumentation"

- timestamp: 2026-11-26T15:30:00.000Z
  checked: "Reproduced the failure with a bare qemu HVF -smp 2 boot (monitor socket attached), 20s budget to the first prompt. 3/13 boots wedged before the shell prompt; serial ends mid-line (e.g. `paqfs...s` or `...va=0x58000` + `s`)"
  found: "qemu monitor `info registers -a` at the hang: CPU0 PC=ffff8000400dabe4, CPU1 PC=ffff8000400dabe0 — both inside `port::mcslock::Lock<Option<&dyn Uart>>::lock` (the CONS console lock's MCS wait loop, symbol in the kernel ELF), PSTATE EL1h with DAIF masked. Both CPUs wait on CONS"
  implication: "The deadlock is a corrupted MCS queue / lost wakeup on the console lock, not a device hang: a context that held CONS abandoned it with a LockNode that a stack collision overwrote"

- timestamp: 2026-11-26T15:38:00.000Z
  checked: "CONS lock memory at 0xffff80004077c408 (qemu monitor `x`) at the hang"
  found: "MCS queue tail = 0xffff90000016db80 — a LockNode address inside the KVA bump arena, i.e. on a kernel stack (kstacks are pagealloc pages mapped there); `owner_cpu` was still the unranked-lock sentinel (CONS is a plain Lock, so the debug owner field is never set) — the queue tail is a waiter, and both CPUs are waiters, so the chain's head's predecessor never cleared it"
  implication: "A node was published into the lock queue from a context that never completed the handoff — consistent with two CPUs sharing one kernel stack and overwriting each other's LockNodes/frames"

- timestamp: 2026-11-26T15:45:00.000Z
  checked: "PerCpu state at a hang (`x /6gx` PER_CPU[0]=0xffff80004088a040, PER_CPU[1]=+0x180): cur_frame, cur_kstack, cpu_id|online, current_proc"
  found: "CPU0: cur_frame=0xffff90000016dee0 cur_kstack=0xffff90000016e000 current_proc=0xffff8000406011e0; CPU1: cur_frame=0xffff90000016da20 cur_kstack=0xffff900000172000 current_proc=0xffff800040601b40. CPU1's cur_kstack is a DIFFERENT 16 KiB kstack from CPU0's, but its cur_frame (0x16da20) lies INSIDE CPU0's kstack [0x16a000,0x16e000) — and is 0x4c0 below CPU0's own frame. In an earlier hang both CPUs' live SPs were in the SAME 16 KiB kstack, 0x800 apart (CPU0's print frame overlapping CPU1's trap-handler frames)"
  implication: "ROOT CAUSE CONFIRMED: a Proc's saved `frame` was a live kernel frame from another CPU's kstack; trap_return then sets SP_EL1 = frame+288, so a CPU ends up running on a kernel stack it does not own → two CPUs on one kstack → overlapping frames → corrupted console lock → hang"

- timestamp: 2026-11-26T15:50:00.000Z
  checked: "The publish handoff in reschedule(): curp.frame=read_curframe(); curp.state=Ready; enqueue_ready(curp.pid) — then the CPU goes on to print `sched: switch` and unwind the trap on that same kstack"
  found: "A sibling CPU (an idle one IPI'd by enqueue_ready, or a stealing one) can pop the preempted pid and trap_return into it immediately, taking traps on the kstack the first CPU is still using (the console print is a long window on HVF: every byte is an MMIO exit). block() has the same shape: it publishes Blocked, drops PROC_TABLE and only then swtches; a wake in that window readies the proc and a sibling can swtch into a context the blocking CPU has not saved yet"
  implication: "Fix: a proc's kstack ownership must be an explicit token (`Proc::on_cpu`) that pick_next enforces, released only after the owning CPU has switched off the kstack (the sched loop's release step, which runs on the scheduler stack)"

- timestamp: 2026-11-26T15:52:00.000Z
  checked: "secondary_main: trap::unmask_irq() runs before enter_sched_loop_from_boot; sched_loop/pick_next/trap_return are documented IRQ-masked contexts"
  found: "On CPU1 the sched loop therefore runs with IRQs UNMASKED (swtch does not touch DAIF, and IrqSpinLock's save/restore preserves the unmasked state). An IRQ taken between pick_next's write_curframe(p.frame) and trap_return's reload overwrites PerCpu.cur_frame with an EL1h frame on the SCHEDULER stack; the tail then restores it (eret to EL1h with SP = sched-stack frame+288) — the loop livelocks, and an IRQ landing after `mov sp, x9` but before the eret (PSTATE.I is 0) builds a frame deep on the proc kstack, which reschedule later saves as that proc's frame"
  implication: "Second fix: mask IRQs on sched-loop entry (and drop the secondary's premature unmask); EOI before preempt() now that reschedule parks the CPU (an active GIC line would otherwise gate the CPU while the proc is off-CPU)"

- timestamp: 2026-11-26T15:56:00.000Z
  checked: "First fix build: the kernel now panics with the new ownership tripwire in pick_next"
  found: "`sched: pid 1 picked while cpu 0 still names its kernel stack 0xffff90000013a000 — two cpus on one kstack`. The tripwire has teeth (it caught a publish violation on the first boot). The violation was second-order: release_left_proc read this CPU's STALE current_proc (last proc the CPU ran) after it had gone idle, and released/re-enqueued a proc that had since migrated to another CPU's kstack"
  implication: "The release must be owner-scoped: consume PerCpu.current_proc (set it null) in the same PROC_TABLE critical section, so a stale pointer can never release another CPU's live proc"

## Resolution
<!-- OVERWRITE as understanding evolves -->

confirmed: 2026-11-26T16:40:00.000Z — the owner/orchestrator independently re-ran the final tree (HEAD 665c94e): HVF gicv3 -smp 2 ALL STAGES PASS twice consecutively, TCG gicv2 -smp 2 ALL STAGES PASS, R9_SMP=1 ALL STAGES PASS.  Archived to .planning/debug/resolved/.

root_cause: "Two independent kernel-stack ownership violations, both exposed only by real (HVF) interrupt timing on 2 CPUs. (1) `reschedule()` published a preempted proc Ready+enqueued (and `block()` published Blocked, then dropped PROC_TABLE before its swtch) while the CPU still ran on that proc's 16 KiB kernel stack; the `sched: switch` console print made that window long on HVF (each UART byte is an MMIO exit), so a sibling CPU could pop the pid and trap_return into it — two CPUs on one kstack, overlapping frames, which corrupted the console MCS queue's LockNodes (both CPUs then spin acquiring CONS forever; the serial wire stops mid-line, e.g. `paqfs...s`) and, pre-05-22-chain, produced the wild user PTE/MMIO pointer. (2) `secondary_main` unmasked IRQs before entering the scheduler loop, so an IRQ in the `pick_next`→`trap_return` handoff rewrote PerCpu.cur_frame with an EL1h frame from the scheduler stack and the tail restored it, leaving SP_EL1 inside a kstack the CPU did not own (the qemu monitor showed CPU1's cur_frame inside CPU0's kstack while its cur_kstack named another)."
fix: "Kernel-stack ownership became an explicit token. `Proc::on_cpu` is set by pick_next/activate_first and cleared only by the owning CPU's new sched-loop `release_left_proc`, which runs on the scheduler stack (after the CPU has left the proc's kstack) and, in one PROC_TABLE critical section, clears cur_frame/cur_kstack/on_cpu and marks a PREEMPTED proc Ready + enqueues it (consuming PerCpu.current_proc makes the release owner-scoped, so a stale pointer can never release a proc another CPU is running). `reschedule()` now only parks this CPU's kernel context with swtch (`blocked_in_kernel`, the same mid-kernel resume shape block() uses); `block()`'s wake-during-swtch window is covered by the same token. `pick_next` refuses (requeues) an on_cpu proc and carries a debug-build tripwire that panics when another CPU's cur_kstack still names the candidate's stack. `sched_loop` masks IRQs on entry; `secondary_main` no longer unmasks before it; `dispatch_irq` EOIs before calling `preempt` (which now parks the CPU). The `sched: switch` marker survives (pick_next prints it from the release's recorded source pid)."
verification: "HVF gicv3 -smp 2 (`nix run .#boot-r9-smoke`, -cpu host, -smp 2): 6/6 ALL STAGES PASS in the first post-fix streak, then 2/2 consecutively on the final tree (HEAD 665c94e) — 8/8 post-fix total versus the pre-fix 6/13 (~45%). TCG gicv2 -smp 2 (R9_GICV2=1): 2/2 consecutively on the final tree. R9_SMP=1: 2/2 on the final tree (marker-free leg: 0 `smp: cpu 1 online` lines, 7-8 `sched: switch` lines). Final-tree captures (v-hvf1/2, v-tcg1/2, v-smp1-1/2): every run ALL STAGES PASS; HVF shows `smp: cpu 1 online`, `smp: tlb shootdown ok` and 8-10 `sched: switch` marker lines per boot. `cargo xtask boot-test --arch aarch64 --config virtmulti --accel hvf` PASS (it asserts >= 1 `sched: switch` line, so the marker's new emission point is gate-clean). `cargo xtask check` exit 0; `cargo xtask test` exit 0 (all suites; the new port::sched `runqueues_any_enqueued_tracks_queued_work` runs and passes); `cargo xtask clippy` exit 0. The pick_next kstack-ownership tripwire fired on a real publish violation during development (a first-cut release read a stale PerCpu current_proc: `sched: pid 1 picked while cpu 0 still names its kernel stack 0xffff90000013a000`), proving the guard has teeth; it is silent on the final tree. Commits: 339036f (fix), 11be789 (WINDOWS #12 + deferred-items closed), 665c94e (module-header contract)."
oracle_type: "derived (two documented invariants — a proc whose kstack a CPU still owns is not runnable on another CPU; the sched loop runs IRQ-masked) + a runtime invariant tripwire in pick_next + e2e (the HVF -smp 2 gate's boot/fork/exec sweep, previously ~45%)"
files_changed:
  - aarch64/src/proc.rs
  - aarch64/src/smp.rs
  - aarch64/src/trap.rs
  - aarch64/src/main.rs
  - aarch64/src/platform/virt/timer.rs
  - port/src/sched.rs
  - UNSAFE-AUDIT.md
