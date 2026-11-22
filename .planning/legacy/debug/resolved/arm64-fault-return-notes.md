---
status: resolved
trigger: "follow-up of notes-async-nsave-percpu-fp (owner, via coordinator, 2026-11-22): fix the aarch64 page-fault-return note delivery gap now — notes must be delivered on return to user mode from a page fault (as 9front does). Red/green: make the probe observable (a probe leg whose spin loop takes page faults with interrupts otherwise unlikely, or a deterministic check); host test if feasible; mutant; aarch64 HVF/TCG gates; x86_64 bytes unchanged (confirm); local commit, no push; keep it small"
created: 2026-11-22T05:45:00Z
updated: 2026-11-22T08:20:00Z
---

## Current Focus

hypothesis: CONFIRMED and fixed in scratch — trap()'s EC 0x20/0x24 arm skipped the note boundary; user-observable through the sched loop's bare resume (a note posted to a proc the sched loop then resumes stays pending; with the gap it waits for the next tick after the faults)
test: DONE — r9 local main e9e5365 → 5f7c4e1 (unpushed); Harvey fpprobe note-fault round (this commit); no new tier-1 marker (the round rides `T1-OK fpprobe`)
expecting: DONE — owner confirmed fixed 2026-11-22
next_action: archived; the scheduler-resume gap is a WINDOWS row; the noted IRQ-mask trust bug continues in its own session. The pin series needs Harvey's r9 pin at >= 5f7c4e1 (the arm64 `T1-OK fpprobe` now includes note-fault, which e9e5365 fails) and one subset-build (the probe changed)
bug_class: Bohrbug (missing call); observable on one CPU via the sched loop's bare resume
reasoning_checkpoint:
  hypothesis: "trap() returns from a resolved demand fault to EL0 without deliver_notes, so a note pending at that fault return (one that arrived while the proc was off the CPU and was resumed by the sched loop's bare trap_return) waits for the proc's next tick/syscall"
  confirming_evidence:
    - "code: trap.rs 565-574 (e9e5365) — the fault arm returns with no boundary; the SVC (562) and IRQ (543) arms deliver"
    - "instrumented boot (k-diag2, base): delivery happens at `DIAG irq` after fault #2-#3 (`fidx=2/3`); instrumented fix (k-diagfix): `DIAG fault-boundary` then `DIAG delivered` at fault #1, 6/6"
    - "uninstrumented: base e9e5365 and the mutant fail note-fault (HVF 3/6 trials fidx=3/4, TCG 6/6 fidx=3/4); fix passes on both"
  falsification_test: "the fix still showing fidx>0 in a trial, or base never failing the round — neither observed"
  fix_rationale: "the boundary belongs on every trap return to EL0 (donor notify() at the end of trap()); SyncRoute::returns makes the shared synchronous tail the single place, so the fault arm cannot skip it"
  blind_spots: "the sched loop's Resume::Return (proc.rs:1753) is itself a return to EL0 without a boundary — left as is (a deliver/kill from sched-loop context needs its own design; recorded as a follow-up); base red rate depends on per-fault console time vs the 1 ms tick"
  candidate_causes:
    - "code: missing deliver_notes on the fault return (confirmed)"
    - "code: sched-loop resume without a boundary (contributing window, out of scope)"
    - "environment: console print per fault ~ tick length hides the gap on slow-console runs (venue property)"
  and_gate: "yes for observability — the fault-return gap shows only when a note is pending at the fault (the sched-loop bare resume supplies that); the fix addresses the fault arm, the resume gap is a separate follow-up"

## Symptoms

expected: a note pending at a demand-fault entry is delivered when the fault returns to EL0 (the donor's notify() at the end of every user trap)
actual: trap()'s EC 0x20/0x24 arm returns without deliver_notes; the note waits for the proc's next tick or syscall
errors: none (latency only on one CPU; a correctness gap for SMP, where another CPU can post during the fault)
reproduction: arm64 fpprobe note-fault round (to be written)
started: always (the demand pager arm never had a note boundary)

## Eliminated

- hypothesis: the NSAVE/NRSTR rounds leave the note child running at EL0 with SPSR.I set (force_psr_el0t keeps the saved DAIF), so its fork child never takes ticks
  evidence: variant B (fault round before every other note round) fails identically; DIAG shows the child taking IRQs and the note delivered
  timestamp: 2026-11-22T06:35:00Z
- hypothesis: a boot probe can red this gap through the sched_loop Resume::Return window (note posted to a never-run child, child faults before its next tick)
  evidence: the window exists, but every printing fault outlasts the 1 ms tick, so the base kernel's IRQ boundary fires right after the first fault return (DIAG, 6/6) — same user-visible state as fault-return delivery
  timestamp: 2026-11-22T06:35:00Z

## Evidence

- timestamp: 2026-11-22T05:45:00Z
  checked: r9 e9e5365 aarch64/src/trap.rs trap() (521-601), aarch64/src/fault.rs header + user_fault/page_in_*
  found: IRQ arm delivers into CURFRAME (543), SVC arm delivers into frame (562), the EC 0x20/0x24 arm calls fault::user_fault and returns (573-574) — no boundary. The fault handler runs IRQ-masked, never unmasks, never sleeps (page_in_from_image reads a staged byte source, "the pager NEVER RPCs"); every zero/bss/stack/text/data page-in prints one console line
  implication: the missing call is the gap; on one CPU nothing can post a note DURING the fault, so the note must already be pending at the fault entry
- timestamp: 2026-11-22T05:47:00Z
  checked: aarch64/src/proc.rs sched()/sched_loop()/pick_next (1700-1760)
  found: sched_loop resumes a Ready proc that is not blocked_in_kernel (preempted at EL0, or a fork child's first run) with `trap::trap_return(frame)` directly — no deliver_notes on that return to EL0
  implication: a second return-to-EL0 path without a boundary (NOT in this session's scope; recorded as a follow-up — a deliver/kill from sched_loop context needs its own design). It is the window that makes the fault boundary observable on one CPU
- timestamp: 2026-11-22T05:48:00Z
  checked: x86_64/src/trap.rs trap() 773-830
  found: x86 has no trap-context pager (exec maps text/data/bss eagerly); every ring-3 sync exception kills via exit_tail("sys: trap"); a preempted x86 proc resumes inside its own IRQ trap() on its kstack, whose tail delivers
  implication: no fault-return or resume gap on x86 — x86 stays untouched (bytes must be unchanged)

- timestamp: 2026-11-22T06:20:00Z
  checked: scratch arm64 fpprobe round note-fault ($SP/fpf/fpprobe-faultround.c) on base e9e5365, HVF; first with a 1 MiB bss region, then with sbrk'd pages; variants A (round last) / B (round first)
  found: (1) exec maps data+bss eagerly (proc.rs bss-sweep comment: "exec maps data+bss eagerly; brk_ extends lazily") — the bss region was copied by rfork (bracket ~800 us HVF, > 900 us on all 40 TCG tries) and never faulted; sbrk'd pages fault (1648 bss page-ins, bracket ~80 us). (2) both variants: child exits "handler ran 0 times"
  implication: the NSAVE-leaves-IRQs-masked idea is eliminated (variant B fails identically); something else hides the delivery
- timestamp: 2026-11-22T06:35:00Z
  checked: instrumented scratch kernels k-diag / k-diag2 (DIAG prints in deliver_notes, the IRQ arm, sys_noted, the fault path) on base e9e5365, HVF, variant B
  found: every trial: `fault: bss page-in va=0x24000` (fault #1), then IMMEDIATELY `DIAG irq pid N` + `DIAG dn ... pc 0x10c4c` (pc = the faulting strb in faultchild, re-executed) + `DIAG delivered ... handler 0x10574 (nhandler)` + `DIAG noted pid N` — the note IS delivered, at the IRQ taken right after fault #1's eret, 6/6 trials. The "handler ran 0 times" was a PROBE bug: 7c registerized the volatile globals (`ldrsw x10,[x28,#0x110]` = fran loaded once at faultchild entry; loops and the final check test w10), so the child never re-read fran
  implication: on this venue the base kernel delivers at the IRQ boundary immediately following the first fault return, with fidx 0 — exactly what fault-return delivery would show. PL011 IMSC = RXIM|RTIM only (no TX IRQ), so the IRQ is the timer, pending because each printing fault outlasts the 1 ms tick. User space cannot distinguish the two delivery points (same pc, registers, memory, FP) — a user-level red is impossible on single-CPU r9 today

- timestamp: 2026-11-22T07:10:00Z
  checked: probe redesign after the DIAG runs — vget() accessor (every fran/fgot read through a call; disassembly shows `bl vget` at each read), then the fork-child window with an nsec bracket (void trial if rfork-to-post > 900 us)
  found: HVF: base red (3/6 trials fidx=3), fix green, mutant (returns() without DemandFault) red (fidx=2..4). TCG: rfork takes ~a tick, so 0-1 of 40 tries were unpreempted — the bracket design cannot run on TCG
  implication: replace the bracket with a race-free order
- timestamp: 2026-11-22T07:25:00Z
  checked: final round — RFMEM child (fgo flag shared), parent posts the note THEN sets fgo, child spins on fgo (no syscall, no fault) and walks fresh STACK pages (private even under RFMEM; the bss sweep shares the bss range); kernels base e9e5365 (store kernel-virt), fix scratch 4d9b8a9, mutant; HVF + TCG ($SP/fpf/runE-summary.txt)
  found: base HVF FAIL (trials 0,2,4 fidx=3), base TCG FAIL (6/6 fidx=3-4); fix HVF `fpprobe: ok`, fix TCG `fpprobe: ok` (~1 stack-growth line per trial); mutant HVF FAIL (fidx=2-3), mutant TCG FAIL (6/6 fidx=4)
  implication: red/green observable on both venues; the fix passes deterministically (the child faults only after the post, so delivery is at the first boundary after resume: a tick, a stack-growth fault, or fault #1 — all leave fidx 0)
- timestamp: 2026-11-22T07:40:00Z
  checked: r9 scratch 5f7c4e1 (final: SyncRoute + shared synchronous tail + host tests + comment) — darwin battery base e9e5365 vs fix, aarch64/x86-64 clippy diagnostic sets, x86-64 kernels (default + q35churnshell), host mutant
  found: check 63 = 63 warnings, clippy 82 = 82, xtask test + aarch64 host 289 → 291 passed (the 2 new trap tests), 0 failed; builds aarch64 virtshell/default + x86-64 OK; aarch64 clippy diagnostic set identical (the same 4 pre-existing missing-safety-comment errors in fault.rs/proc.rs); x86-64 kernel-x64 default AND q35churnshell byte-identical (cmp), x86_64 crate depends only on port → boot-iso FOD unchanged; host mutant returns() without DemandFault → `every_returning_el0_trap_is_a_note_boundary` FAILED (53/1)
  implication: arch-only, x86 bytes unchanged, no boot-iso re-derivation needed

- timestamp: 2026-11-22T08:00:00Z
  checked: aarch64 gates on scratch r9 5f7c4e1 with the rebuilt userland (harvey-sb + this probe, subset-build PASS 74 entries), $SP/fpf/series; landing; r9 aarch64 boot tests on the landed real checkout
  found: tier-1 TCG PASS 44 T1-OK `fpprobe: ok`; tier-1 HVF PASS 44 T1-OK `fpprobe: ok`; smoke HVF ALL STAGES PASS; net-smoke HVF ALL STAGES PASS (kernel package r9-kernel-virt-5f7c4e1 in each). Real r9 fast-forwarded e9e5365 → 5f7c4e1 (no push, no attribution). Boot tests virt, virtexec, virtexec3, virtmulti, virtnotes, virtpanic (the KernelHalt arm: register rows + clean halt), virtchurn, virtip, virtnet, virtlowecam PASS; virtshell rc=1 = base (no initrd by design)
  implication: verified on the landed content

## Resolution

root_cause: r9 aarch64 trap() returned from a resolved demand fault (EC 0x20/0x24 → fault::user_fault) to EL0 without the note boundary — only the SVC and IRQ arms called deliver_notes (the donor runs notify() at the end of trap() for every user trap). Observable on one CPU because the sched loop resumes a Ready proc that is not blocked_in_kernel (preempted at EL0, or a fork child's first run) with a bare trap_return, so a note posted while it was off the CPU is still pending when it runs; if it then takes demand faults, delivery waited for its next tick
fix: r9 5f7c4e1 — SyncRoute (ESR EC + SPSR mode → Syscall / DemandFault / UserKill / KernelHalt) routes trap()'s synchronous half; the returning routes (SyncRoute::returns: Syscall, DemandFault) share ONE note boundary after the match; host tests trap::tests::{every_returning_el0_trap_is_a_note_boundary, diverging_routes_are_not_boundaries}. Harvey: arm64 fpprobe note round "fault" — an RFMEM child spins on a shared go flag, the parent posts "fpprobe-f" then sets go and waits, the child walks fresh stack pages and its (inherited) handler records the page index; delivery must come no later than the first fault's return (fidx 0); vget() reads volatile globals through a call (7c registerizes them in call-free loops)
verification:
  target_test:
    result: pass
    detail: "note-fault round: base e9e5365 FAIL on HVF (3/6 trials fidx=3) and TCG (6/6 fidx=3-4); 5f7c4e1 `fpprobe: ok` on HVF and TCG (standalone and inside tier-1). Instrumented boots locate it: base delivers at `DIAG irq` after fault #2-#3, the fix at `DIAG fault-boundary` on fault #1, 6/6"
  mutation_check:
    result: pass
    mutant_killed: "returns() without DemandFault (the fault arm's boundary removed): host test every_returning_el0_trap_is_a_note_boundary FAILED (53/1); boot: note-fault FAIL on HVF (fidx=2-3) and TCG (6/6 fidx=4)"
  no_op_deletion:
    result: pass
    deletion_justified_by_rca: "the removed lines are the if/else-if chain replaced by the SyncRoute match (same arms, same prints, same divergence) and the per-arm SVC boundary folded into the shared tail"
  adjacent_tests:
    result: pass
    suites_run: ["darwin: check 63 = base, clippy 82 = base, aarch64 clippy diagnostic set identical to base, xtask test + aarch64 host 291 passed (base 289 + 2 new), builds aarch64 virtshell/default + x86-64", "aarch64 gates on 5f7c4e1 with rebuilt userland: tier-1 TCG + HVF (44 T1-OK), smoke + net-smoke HVF — ALL STAGES PASS", "r9 aarch64 boot tests 10/10 = base (virtshell no-initrd = base)"]
  revert_and_reconfirm:
    result: pass
    bug_returned_on_revert: true — "e9e5365 and the mutant fail the round on both venues"
    fixed_on_reapply: true — "5f7c4e1 passes on both venues"
  x86_64_bytes: "UNCHANGED — kernel-x64 default and q35churnshell builds byte-identical (cmp) at e9e5365 and 5f7c4e1; x86_64 depends only on port, so the boot-iso FOD hash stands (no container run needed)"
  guardrail_verdict: accepted (pending human verification)
files_changed: [r9 aarch64/src/trap.rs, Harvey build/aarch64/fpprobe.c]
follow_ups:
  - "aarch64 sched_loop Resume::Return (proc.rs ~1753) is itself a return to EL0 with no note boundary: a note posted to a preempted or new proc waits for its next tick, syscall or (now) fault. x86 has no such path (a preempted proc resumes inside its own IRQ trap()). Fixing it means delivering (or killing) from sched-loop context — its own design; the note-fault round's criterion (no later than the first fault) stays valid if it lands"
  - "aarch64 TrapFrame::force_psr_el0t (NOTED's sanitizer) clears only M[3:0], so DAIF bits from the user-writable saved Ureg pass through: a handler that sets I/F in ur->psr and returns NCONT runs at EL0 with IRQs masked (no tick preemption on one CPU). Check against the donor's noted() and clear DAIF there — not changed here"
  - "the note-fault round's red rate depends on the venue: a kernel without the boundary passes a trial whose tick lands inside the first fault (each fault prints a console line), so on a slow console it can pass; the fix passes deterministically"
