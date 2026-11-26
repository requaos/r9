---
status: resolved
trigger: "aarch64 net-smoke gate regressed during Phase 5 Part 2: PASSES at 4ff046e (end of Part 1) and at ef9553d (05-08), FAILS at ceb6ecc (05-22 fix chain) and at HEAD. Deterministic under TCG and identical at R9_SMP=1, so it is not a concurrency flake. Panic message is unreadable (console truncates at 'panicked at ')."
created: 2026-11-26T17:20:00.000Z
updated: 2026-11-26T18:55:00.000Z
---

## Current Focus
<!-- OVERWRITE on each update - always reflects NOW -->

hypothesis: "CONFIRMED root cause: heap alloc/dealloc under the NetIrq-ranked INGRESS lock in port/src/netif.rs, made visible when 66a3548 activated the ranked-lock checker. Fixed in c42a66f; all gates green."
test: "End-to-end verification matrix on the committed fix"
expecting: "PASS on TCG net-smoke (x3), R9_SMP=1, genuine -smp 1, HVF boot smoke, xtask check+test"
next_action: "none — awaiting human verification of the fix in the real workflow"
bug_class: bohrbug
reasoning_checkpoint:
  hypothesis: "66a3548's `smp::register_port_smp()` call activated the ranked-lock assertions (before it, ARCH was never registered and every rank check was a no-op). With the checker live, `netif::conv_alloc` allocates `FrameStore::new()` from the global heap while holding INGRESS (NetIrq rank 90), so `GlobalQuickAlloc::alloc` requests heap_alloc (rank 80) out of order and rank_watch panics. `conv_close`'s `free = (...).collect()` and `rx_ingress`'s `VecDeque::push_back` are the same class and would fire next."
  confirming_evidence:
    - "Full panic text captured by raw-pty drive: `smp: LOCK RANK VIOLATION on cpu 0: holds \"netif_ingress\" (rank 90); requested \"heap_alloc\" (rank 80)`."
    - "Manual frame-pointer walk in the parked guest: panic_fmt <- GlobalQuickAlloc::alloc (globalalloc.rs:250) <- EtherDev::bind (phyadapt.rs:326, the conv_alloc call)."
    - "netif.rs:468 `ig.convs[id].store = Some(FrameStore::new())` runs with the INGRESS guard live; netif.rs:547 `store.free = (...).rev().collect()` does too; FrameStore::push's `queue.push_back` grows a capacity-0 VecDeque under INGRESS."
    - "cdd4e61 never calls register_port_smp (git grep: definition only); 66a3548 is the commit that added the main9 call; the cdd4e61 probe PASSES."
  falsification_test: "Build with the three heap operations moved out of INGRESS; if the net-smoke still panics with the same (or another) rank violation, the hypothesis is wrong."
  fix_rationale: "Removes every heap operation from under the NetIrq-ranked INGRESS lock (allocate the frame store before taking INGRESS, reset the free stack in place, preallocate the queue capacity) — restoring the documented 'NetIrq/Console never allocate' invariant rather than silencing the assertion."
  blind_spots: "Other rank>=90 allocation sites elsewhere (console path) could still fire later in the smoke; the end-to-end runs will reveal them."
  candidate_causes:
    - "code: heap alloc/dealloc under the NetIrq-ranked INGRESS lock (confirmed)"
    - "config: 66a3548 activated the debug/gate rank checker that had been dormant (contributing condition)"
  and_gate: "yes — the panic needs BOTH the latent allocation-under-INGRESS code AND the rank-checker activation from 66a3548; neither alone reproduces it."
tdd_checkpoint: null

## Session Loop

goal: find_and_fix
tdd_mode: false
tdd_phase: none
resume_count: 0
awaiting: none
question: none
last_next_action: none

## Symptoms
<!-- Written during gathering, then immutable -->

expected: "`nix run .#boot-r9-net-smoke` reaches `NET-SMOKE: ALL STAGES PASS` (as it did on 2026-11-25 and at commit 4ff046e)"
actual: "The kernel panics during the net-smoke program's bss page-in sweep (last marker `fault: bss page-in va=0x48000`). The panic message is truncated to `panicked at ` (12 chars) — the console write stops at the prefix/location boundary. The gate aborts on the panic marker."
errors: "No readable panic text. `panicked at ` only. Unmasking IRQs in the panic handler did not change the truncation."
reproduction: "cd /Users/neilskinner/code/r9 && R9_ACCEL=tcg nix run .#boot-r9-net-smoke. Deterministic; also fails with R9_SMP=1; also fails on HVF."
started: "Between ef9553d (2026-11-26 wave 6, PASSES) and ceb6ecc (05-22 fix chain, FAILS). Pre-05-22 trees between cannot boot at all (the .bootdata-mapped-RX defect fixed in ceb6ecc), so the window is 05-09, 05-10, or 05-22's fix chain."

## Eliminated
<!-- APPEND only - prevents re-investigating after /clear -->

- hypothesis: "Pre-existing failure, not caused by Phase 5"
  evidence: "target/gates/smoke-r9-net/smoke-r9-net.log dated 2026-11-25 records NET-SMOKE: ALL STAGES PASS; commit 4ff046e reproduces PASS under TCG"
  timestamp: 2026-11-26T17:20:00.000Z

- hypothesis: "05-07/05-08 (SMP-safe allocator + devip/netif locking) caused it"
  evidence: "ef9553d (wave 6, includes 05-07 and 05-08) PASSES the net-smoke under TCG"
  timestamp: 2026-11-26T17:20:00.000Z

- hypothesis: "It is a two-CPU concurrency bug"
  evidence: "Fails identically with R9_SMP=1 and default -smp 2, on both TCG and HVF"
  timestamp: 2026-11-26T17:20:00.000Z

- hypothesis: "The panic message is lost because IRQs are masked and the console TX FIFO cannot drain"
  evidence: "Adding trap::unmask_irq() before println! in the panic handler did not change the 12-char truncation (commit 299f8d6)"
  timestamp: 2026-11-26T17:20:00.000Z

- hypothesis: "The panic handler's lock-free early-UART write can show the message"
  evidence: "It faults at PA 0x09000018 (DataAbortSameEl) because the pre-MMU identity alias is not mapped in a user address space; the mapped PL011_RANGE path is not the one taken (commit 92ac6b9, reverted in 299f8d6)"
  timestamp: 2026-11-26T17:20:00.000Z

- hypothesis: "The regression is in 05-09 or 05-10 alone"
  evidence: "DISPROVED: a probe worktree at /private/tmp/r9net5 (05-10 merge cdd4e61 + only `git checkout ceb6ecc -- aarch64/lib/kernel.ld`) ran R9_ACCEL=tcg nix run .#boot-r9-net-smoke -> NET-SMOKE: ALL STAGES PASS. 05-09/05-10 are clean."
  timestamp: 2026-11-26T17:45:00.000Z

- hypothesis: "ceb6ecc's own (non-kernel.ld) hunks are the direct cause"
  evidence: "DISPROVED as the causal hunk: port/src/netif.rs (the guilty file) is identical at cdd4e61 and ceb6ecc, and the panic is a rank violation that only fires once 66a3548's smp::register_port_smp() activates the checker. The introducing commit is 66a3548; ceb6ecc's repair hunks are downstream of it in the same 05-22 chain. The net-smoke never even exercises the repair hunks before the panic (the bind fails first)."
  timestamp: 2026-11-26T18:15:00.000Z

## Evidence
<!-- APPEND only - facts discovered during investigation -->

- timestamp: 2026-11-26T17:20:00.000Z
  checked: "Bisect runs under TCG"
  found: "4ff046e PASS; ef9553d PASS; 41a7f8d does not boot (pre-.bootdata-fix); ceb6ecc FAILS; HEAD FAILS; HEAD with R9_SMP=1 FAILS; HEAD on HVF FAILS"
  implication: "Deterministic, CPU-count independent, window = 05-09 / 05-10 / 05-22 fix chain"

- timestamp: 2026-11-26T17:20:00.000Z
  checked: "Serial capture at the failure"
  found: "The last marker is `fault: bss page-in va=0x48000`; then `panicked at ` and the gate aborts. No trap frame in the panic path (except when the panic handler itself faults on the early-UART alias)."
  implication: "The panic happens in the page-in path or immediately after it; the console write wedges at the location boundary"

- timestamp: 2026-11-26T17:45:00.000Z
  checked: "Probe worktree /private/tmp/r9net5 = 05-10 merge cdd4e61 + ceb6ecc's aarch64/lib/kernel.ld only"
  found: "R9_ACCEL=tcg nix run .#boot-r9-net-smoke -> NET-SMOKE: ALL STAGES PASS"
  implication: "05-09 and 05-10 are clean; the regression is entirely inside ceb6ecc's remaining hunks (main.rs, trap.rs, timer.rs, gicv3.rs, proc.rs, smp.rs, syscall.rs, vm.rs, port/src/smp.rs). gicv3 is HVF-only and the failure reproduces on TCG, so the live suspects are main.rs, trap.rs, timer.rs, proc.rs, smp.rs, syscall.rs, vm.rs, port/src/smp.rs."

- timestamp: 2026-11-26T18:05:00.000Z
  checked: "Manual QEMU drive of the exact repro (raw pty serial, same kernel ELF, -smp 2 TCG) with the full panic message captured"
  found: "panicked at port/src/smp.rs:480:17: smp: LOCK RANK VIOLATION on cpu 0: holds \"netif_ingress\" (rank 90); requested \"heap_alloc\" (rank 80) — locks are taken in strictly increasing rank order (D-26)"
  implication: "The net-smoke panic is a lock-order violation: the netif ingress critical section allocates from the global heap. The console truncation in the gate run is a secondary artifact of the panic context, not the bug."

- timestamp: 2026-11-26T18:06:00.000Z
  checked: "Mechanism of the 12-char truncation vs the manual run"
  found: "The manual raw-pty run printed the whole panic message; the gate run truncates at `panicked at `. Same kernel ELF. The difference is the panic context, so the truncation is context-dependent console behaviour, not the root cause."
  implication: "Do not chase the console; chase the rank violation. The gate's console path is a symptom that will disappear once the panic does."


- timestamp: 2026-11-26T18:10:00.000Z
  checked: "Manual frame-pointer walk in the parked guest (lldb over the QEMU gdbstub)"
  found: "panic_fmt <- GlobalQuickAlloc::alloc (globalalloc.rs:250) <- EtherDev::bind (phyadapt.rs:326, the netif::conv_alloc call site)."
  implication: "The allocation is the FrameStore build inside conv_alloc, under the INGRESS guard — not the rx path, not the scheduler."

- timestamp: 2026-11-26T18:15:00.000Z
  checked: "When the rank checker became live (git grep register_port_smp / register_arch across the window)"
  found: "cdd4e61 defines smp::register_port_smp but never calls it; 66a3548 (a 05-22 commit, not ceb6ecc) adds the main9 call. port/src/netif.rs is byte-identical between cdd4e61 and ceb6ecc (the port/ diff in that range is fdt.rs + smp.rs only)."
  implication: "The introducing commit is 66a3548 (checker activation), not ceb6ecc's hunks; the latent violation has existed since the 05-08 netif locking conversion. The fix must remove the heap activity under INGRESS, not revert the checker."

- timestamp: 2026-11-26T18:20:00.000Z
  checked: "Truncation mechanism"
  found: "The expect driver's `expect_before` matches `panic` and kills qemu's process group immediately; a raw-pty drive of the SAME ELF prints the full message. 299f8d6's IRQ unmask and the lock-free early-UART attempt were treating a harness artifact."
  implication: "Do not change the panic handler for this. The gate's fast-fail truncates by design."

- timestamp: 2026-11-26T18:40:00.000Z
  checked: "Post-fix verification matrix on commit c42a66f"
  found: "TCG flake net-smoke PASS x3; R9_SMP=1 flake net-smoke PASS; genuine -smp 1 local net smoke PASS (0 secondary-online lines); HVF boot-r9-smoke PASS; fast local repro PASS x5; cargo xtask check rc=0; cargo xtask test rc=0; no LOCK RANK VIOLATION or `panicked at` in any post-fix capture."
  implication: "Fix confirmed end-to-end with a specified oracle; the mechanism is understood and the assertion was not silenced."

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: "Heap operations under the NetIrq-ranked INGRESS lock in port/src/netif.rs: conv_alloc built FrameStore (bytes Vec + free Vec) with INGRESS held, conv_close assigned a freshly collected free Vec (alloc + dealloc), and FrameStore::push grew a zero-capacity VecDeque from the rx IRQ retire. INGRESS is rank 90, above HeapAlloc 80, and the rank table's rule is that NetIrq/Console never allocate (the rx path runs in IRQ context). The violation was latent until 66a3548 added smp::register_port_smp() to main9, activating the ranked-lock checker (cdd4e61 never calls it; git grep finds only the definition). The first net-smoke ether bind then panicked in GlobalQuickAlloc::alloc: `smp: LOCK RANK VIOLATION on cpu 0: holds \"netif_ingress\" (rank 90); requested \"heap_alloc\" (rank 80)`. The console truncation was NOT a kernel defect — the expect driver's expect_before matches `panic` on the first chunk and kills qemu's group before the rest of the message drains."
fix: "c42a66f — in port/src/netif.rs: (1) conv_alloc chooses the slot under NETIF, reads the store's presence under INGRESS, RELEASES it, allocates FrameStore, then retakes INGRESS to install (no heap op under INGRESS); (2) conv_close resets the free stack in place with clear()+extend() instead of a collecting assignment (no alloc/dealloc under INGRESS); (3) FrameStore::new preallocates the VecDeque to FRAME_SLOTS and push debug-asserts len < capacity, so rx pushes from IRQ context never grow the heap. NETIF, the unranked lifecycle lock, is held throughout, so the slot and its store's presence are stable."
verification: "oracle: gate markers (specified). R9_ACCEL=tcg nix run .#boot-r9-net-smoke -> PASS x3 (flake c42a66f; ~20-61s each); R9_ACCEL=tcg R9_SMP=1 nix run .#boot-r9-net-smoke -> PASS; genuine -smp 1 net smoke (R9_SMP=1 + a local expect copy honoring it, same kernel) -> PASS with 0 'smp: cpu 1 started' lines; nix run .#boot-r9-net-smoke (HVF auto) -> PASS; nix run .#boot-r9-smoke (HVF auto) -> PASS; fast local repro (direct qemu+expect, no flake) -> PASS x5 total; cargo xtask check -> rc=0; cargo xtask test -> rc=0 (all legs ok). Falsification check done: the pre-fix ELF panicked deterministically at the first ether bind; the post-fix ELF completes every stage, so the mechanism is confirmed, not merely masked. No new 'panicked at' or LOCK RANK VIOLATION appears anywhere in the post-fix captures."
oracle_type: "specified (the gate's NET-SMOKE: ALL STAGES PASS + per-stage presence markers; the fix-site no-alloc guards are implicit debug_asserts)"
files_changed: ["port/src/netif.rs", ".planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/deferred-items.md"]
