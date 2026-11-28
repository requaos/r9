---
status: awaiting_human_verify
trigger: "x86_64 -smp 2 on BOTH accels, after the AP bring-up fixes (debug/x86_64-ap-bringup): the two CPUs wedge the first time one queues on an MCS lock whose holder runs in process context, because the hand-off writes the predecessor's node at a kernel-stack VA that is private to the holder's process. No panic and nothing on the wire — the boot stops after `paqfs...`. Blocks every x86_64 -smp 2 gate."
created: 2026-11-27T20:45:31.000Z
updated: 2026-11-28T03:15:00.000Z
---

## Current Focus
<!-- OVERWRITE on each update -->

hypothesis: "CONFIRMED and fixed on branch fix/x86_64-smp2-kstack, in two parts. (1) MCS lock nodes live on the calling CPU's kernel stack and an acquire writes the PREDECESSOR's node; with every proc's kstack mapped at one VA under its own PML4, that write landed on the waiter's own stack (or nowhere), the holder never saw a successor and could not unlock, and both CPUs spun with interrupts masked. (2) A CPU with nothing to pick kept the last proc's page-table root in CR3 and the sched loop ran on it, so a root page freed at exit (by this CPU's reap, or by the sibling that stole the preempted proc) was zeroed for reuse under a CPU still translating through it: a triple fault the churn gate reached 600 cats in, at -smp 2 only."
test: "The composed x86_64 gates at the default -smp 2 under KVM on the committed tree, with single-CPU and main controls; a loop of the churn workload at -smp 2 under KVM; the same crash under TCG with -d int; host tests and clippy"
expecting: "The churn crash is gone (the loop stops in no run); each gate ends where main ends at -smp 1 (the open defects below) or passes; no new -smp 2 failure"
next_action: "Owner verifies on a KVM host on the branch (3ff89ab): nix run .#boot-r9-churn-x64 and .#boot-r9-net-smoke-x64 pass at the default -smp 2, and .#boot-r9-smoke-x64 passes the chain leg and the CPU ladder to cpu-ether-ok and stops only at cpu-keyfs-ok (defect 4); then decide how the branches land"
bug_class: concurrency
reasoning_checkpoint: null
tdd_checkpoint: null

## Session Loop
goal: find_and_fix
tdd_mode: false
tdd_phase: none
resume_count: 0
awaiting: human_verify
question: "Is the -smp 2 fix accepted (the scheduler runs on the boot page table; process kernel stacks share one address; the console line is atomic) so the session can be archived? The gates that stay red on the branch stop on defects main also has at one CPU: (4) below, fpprobe's sleep check (fixed on the stacked fix/x86_64-sleep-wake), and services' srv and exportfs."
last_next_action: "Run the composed gates at -smp 2 under KVM on the committed tree with single-CPU and main controls, and root-cause the churn reset the first batch exposed"

## Symptoms
<!-- IMMUTABLE -->

expected: "`nix run .#boot-r9-smoke-x64` with the default -smp 2 boots to the prompt on TCG and on KVM."
actual: "After the bring-up fixes the AP comes online and runs the first forked process; the first lock hand-off between the two CPUs then wedges both. KVM stops after `paqfs...`, TCG after `paqfs...version...`. Both CPUs spin in MCS code with nothing more on the wire."
errors: "No panic. Before the tripwire fix (fed9be7) a false-positive tripwire panic fired first and its console print then hit this fault: `trap PF` on the AP, ecode 0x2, at atomic_store::<*mut LockNode>, address 0xffffc80000003a00 (KVM) / 0xffffc80000003dc0 (TCG)."
reproduction: "branch debug/x86_64-ap-bringup at fed9be7 on an x86_64 Linux host: `nix run .#boot-r9-smoke-x64` (KVM) or `R9_ACCEL=tcg nix run .#boot-r9-smoke-x64`. The 20-second shape: compose the kernel into the chain ISO, boot `-M q35 -cpu host -smp 2 -accel kvm -no-reboot -no-shutdown -serial file:…`, and read both CPUs through the QEMU monitor when the wire stops."
started: "Latent since the 05-24 MCS port. First reached on 2026-11-27 when the AP bring-up was fixed; before that KVM never got an AP online (expand-down trampoline segment) and TCG's bring-up gave up before the AP finished (collapsed timer deltas)."

## Eliminated
<!-- APPEND only - prevents re-investigating after /clear -->

- hypothesis: "A real kernel-stack ownership race — the aarch64 smp2-hvf-corruption class, suggested by pick_next's tripwire firing."
  evidence: "The tripwire compared kstack VAs, which are the same window VA for every proc on x86_64: it reported pid 2 'still named by cpu 0' with 0xffffc80000004000 — pid 1's VA as well. Keyed on the kstack top page PA (fed9be7) it stays silent while the AP runs pid 2 on both accels."
  timestamp: 2026-11-27T20:45:31.000Z

- hypothesis: "KVM-specific timing."
  evidence: "TCG shows the same fault: `trap PF` on the AP with ax=0xffffc80000003dc0 right after `paqfs...`, once the bring-up pacing was fixed (3f1b76a)."
  timestamp: 2026-11-27T20:45:31.000Z

## Evidence
<!-- APPEND only - facts discovered -->

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "The AP's first synchronous exception under KVM (hardware breakpoint on trap.rs:914's print call; target/ap-debug/dumpsite.py)"
  found: "CPU 1: vector 14 (#PF), ecode 0x2 (write, not present), pc = core::sync::atomic::atomic_store::<*mut LockNode>, the written address 0xffffc80000003a00. CPU 0's rsp at that moment was 0xffffc80000003b10 — the same VA window, the BSP's process kstack. Below it on the AP's stack: the console print's mcslock acquire."
  implication: "The AP is linking its MCS node behind a node that lives on the BSP's kstack, at a VA its own CR3 does not map."

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "Both CPUs at the final wedge on the branch tip (gdb stub halt + frame-pointer walk + stack scan; target/ap-debug/stacks.py, capture stk-e)"
  found: "CPU 1, in sys_dup for the forked proc (CR3 0x6f67000): syscallentry → dispatch_proc → sys_dup → fdgrp drop → Chan::dup → alloc → GlobalQuickAlloc → MCSLock::lock → AtomicBool::load (mcslock.rs:65). CPU 0, in a namec walk: namec_walk → dev_walk → DevRoot::walk → drop Vec<u8> → dealloc → IrqLockGuard<QuickFit>::drop (mcslock.rs:489) → AtomicPtr<LockNode>::compare_exchange_weak."
  implication: "CPU 0 holds the heap lock and cannot unlock it: its node's `next` was never set. CPU 1 queued behind it, wrote `predecessor.next` into its own proc's kstack instead, and waits for `locked` forever. Textbook for the hypothesis."

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "The source facts"
  found: "port/src/mcslock.rs:24-27 LockNode { next: AtomicPtr<LockNode>, locked: AtomicBool }; :52 lock(&self, node: &LockNode); :56 self.queue.swap(p, AcqRel); :64 predecessor.next.store(p, Release). x86_64/src/proc.rs:178-194 KSTACK_PML4_INDEX 400, KSTACK_VA 0xffff_c800_0000_0000, 'Every proc's PML4 maps ITS OWN kstack at this SAME VA'. Stack nodes at every call site: port/src/sched.rs:88 and :116 (run queues), x86_64/src/proc.rs pick_next, and every `let node = LockNode::new()` in port/."
  implication: "A cross-CPU hand-off is only correct if every CPU can write every other CPU's node. With per-process kstack VAs it never can be."

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "Why single-CPU gates never saw it"
  found: "CI's main x86_64 smoke job pins R9_SMP=1 (.github/workflows/gates.yml:148-158); the avx2 TCG leg keeps the -smp 2 default. On TCG the AP bring-up collapsed until 3f1b76a (timer deltas of zero in periodic mode), so no composed gate has ever had an AP past bring-up on TCG, and KVM had none before b4ec82f. 05-12-SUMMARY.md:257 records that x86_64 boots could not run on the dev host."
  implication: "The two-CPU x86_64 path has not had a green run on any venue so far; CI history for the -smp 2 legs was not checked"

- timestamp: 2026-11-27T21:51:44.000Z
  checked: "The owner's own gate run at the tip (kernel 481a330, KVM, -smp 2), then that run's ISO booted under the gdb stub (target/ap-debug/wedge.py)"
  found: "In the owner's run the serial capture did not change for 7+ minutes while both vCPU threads took 3000 ms of CPU per 3 s window and QEMU's main thread took none. In the instrumented boot of the same ISO the wire stayed silent for 60 s with: CPU 0 at mcslock.rs:95 — MCSLock::unlock's compare_exchange_weak loop, holding the heap lock through IrqLockGuard<QuickFit>::drop from a buffer re-allocation in sys_open → namec_walk → paqfs read_block; CPU 1 at mcslock.rs:65 — MCSLock::lock's `while node.locked.load()`, waiting for that heap lock from a Vec dealloc in sys_dup, its node at kstack-window address 0xffffc80000003b40. IF=0 on both."
  implication: "The wedge is the one above, seen in the owner's artifact: the holder waits for a successor link that was written to the wrong stack, the waiter for a release that cannot come, both with interrupts masked. The contended lock varies with timing (heap lock here, the page-frame pool's lock in another run of the same ISO); the mechanism does not."

- timestamp: 2026-11-27T23:54:10.000Z
  checked: "KVM -cpu host -smp 2 on fix/x86_64-smp2-kstack: the composed smoke gate, and the same ISO booted with the gdb stub"
  found: "The chain boot reaches the prompt in 3.6 s and passes every chain stage (prompt, echo, empty line, ls/ps/cat, DEL, restart, pid-warning-absent, nic-bind); the CPU leg passes through cpu-ether-ok - 31 stages.  Both CPUs sit in process page tables of their own at the prompt.  Before the fix the gate stopped after `version`."
  implication: "The wedge is gone at two CPUs"

- timestamp: 2026-11-27T23:54:10.000Z
  checked: "The first two-CPU boot's console output"
  found: "Lines from the two CPUs interleaved mid-line (`process 2 exited: "fault: text page-in va=` - two lines in one).  Console::write_str took the console lock once per PIECE of a formatted line."
  implication: "Every gate marker is a line; fixed in 9a2b5a3 by holding the lock across the whole formatted output.  Whole lines afterwards."

- timestamp: 2026-11-27T23:54:10.000Z
  checked: "TCG qemu64 -smp 2 on the branch: the composed smoke gate"
  found: "The same 31 stages pass (chain leg complete, CPU leg through cpu-ether-ok)."
  implication: "Fixed on both accels"

- timestamp: 2026-11-27T23:54:10.000Z
  checked: "The gate's end point on the branch, and on main at -smp 1 (control)"
  found: "Both runs print `cpurc: FAIL keyfs` then `cpurc: FAIL authsrv-listen`, `FAIL factotum`, `FAIL listen-17010`, `FAIL sweep`, and reach a prompt; the gate waits 600 s for `cpurc: KEYFS-OK` and reports `TIMEOUT at stage: cpu-keyfs-ok`.  The keyfs rung's helper `hmntgen` (pid 17) hits `syscall 10 unimplemented`, then faults at pc=0x0, and its `mount /mnt` gets `devmnt: eof waiting for a reply`."
  implication: "Open defect (4) is not a hang at keyfs but a failing keyfs rung that cascades down the ladder - the same on main at one CPU.  Its cause is outside the -smp 2 path (an unimplemented system call number in hmntgen's path)."

- timestamp: 2026-11-28T00:53:00.000Z
  checked: "The composed x86_64 gates at the default -smp 2 under KVM on the branch at 9a2b5a3 (the stack arena and the console line, before the scheduler fix)"
  found: "net-smoke PASSES (47 stages, 14 s). Smoke passes the chain leg and the CPU ladder to cpu-ether-ok (31 stages) and then waits 600 s for cpurc: KEYFS-OK. tier-1 and services run to the prompt and fail their post-pass on one and two subjects. auth fails on AUTH-FAIL. churn RESETS the machine: 599 cats, `sched: switch 5->600`, then the SeaBIOS banner; the driver waits on the second boot's prompt until its 2400 s cap (rc 124)."
  implication: "Net of the boot gates, one two-CPU defect was left, and it is a crash, not a wedge."

- timestamp: 2026-11-28T00:55:00.000Z
  checked: "Single-CPU and main controls for the gates that failed at -smp 2 (KVM; the drivers take -smp from their spawn line, so -smp 1 ran from a copy with that one number changed)"
  found: "tier-1: the branch at -smp 1 and main (0e783a0) at -smp 1 both fail exactly `fpprobe: BAD flags-nt sleep N r=-1 rflags=<word>` (the word printed in hex: 0x4297 or 0x4293, NT set and handed back; only the -1 is wrong), post-pass missing T1-OK fpprobe. services: the branch and main at -smp 1 both print `SVC-SKIP-WHY no-rc-lib-rcmain-the-srv-child-rc-dies`, `SVC-FAIL srv`, `SVC-FAIL exportfs`, the rest SVC-OK. churn: the branch and main at -smp 1 both PASS (`churn: CATS-DONE 1400`)."
  implication: "The tier-1 and services verdicts are main's at one CPU, not two-CPU regressions. The churn reset is two-CPU only. (fpprobe's sub-leg is its own kernel bug, sys_sleep reporting a child's exit as an interruption; it is fixed on the stacked branch fix/x86_64-sleep-wake, outside this session.)"

- timestamp: 2026-11-28T01:41:00.000Z
  checked: "The churn crash under KVM, outside the gate: the branch kernel (9a2b5a3) in the churn ISO, -smp 2, -no-reboot -no-shutdown, `/bin/churncats` typed at the prompt, 25 runs"
  found: "The VM stops in 14 of 25 runs, 1.8 to 6.6 s after launch (731 to 5574 serial lines). In the seven stops read: one CPU is in Uart16550::putb (called from Locked::write_str) with SS=0 and access byte 0x00c00000, the other in idle_wait, in memcpy, or in RoaringBitmap::insert; the registers at the stop do not say which exception began it."
  implication: "A triple fault (the VM parks instead of rebooting). The exception sequence needs the emulation log."

- timestamp: 2026-11-28T01:46:00.000Z
  checked: "The same workload under TCG with -d int"
  found: "Reproduced (run 1: 445 cats). The log's last exceptions: the idle CPU services the wake IPI 0xF1 at idle_wait with CR3=0x0e8ac000; next is `v=0e e=0000 cpl=0 IP=ffffffff8020dd7e SP=ffffff000011fb70 CR2=ffffffff84ec5248`, a kernel-mode page fault, not present, read, on a KZERO address, under the same CR3; then `v=0e e=0010 ... CR2=ffffffff8026be90` repeated down the Mach stack (the fault handler's own fetch is unmapped); then a stack write at SP itself; `check_exception old: 0xe new 0xe`, v=08; `old: 0x8 new 0xe`."
  implication: "CR3 names a page whose kernel half is gone: a process root page that went back to the allocator and was zeroed, with the CPU still translating through it. reap_moribund's comment held the assumption: 'the page cannot be recycled (and zeroed by alloc_phys_page) before the next pick replaces CR3'."

- timestamp: 2026-11-28T02:10:00.000Z
  checked: "The scheduler's CR3 discipline in the code: pick_next installs the picked proc's table; nothing puts the boot table back; sched, reschedule and sched_from_moribund swtch into the scheduler on the table of the proc they leave"
  found: "A CPU with nothing to pick idles with the last proc's root in CR3, and the sched loop runs each iteration on the table of the proc it has just left. At one CPU the freed page is zeroed only by that CPU, whose TLB still carries what the loop touches. At two, the sibling zeroes it at once, or the proc was preempted here, stolen and exited there; the idle CPU then faults on the first address its TLB does not hold, the wake IPI's pick_next, and with the kernel half gone the handler faults too."
  implication: "Root cause, second part: the scheduler context ran on process page tables. The rule: a proc's table is in CR3 only while that proc is current."

- timestamp: 2026-11-28T02:20:00.000Z
  checked: "The first fix attempt (leave_proc_table at the three switch-out sites), one boot"
  found: "The kernel halted at the first process exit: `trap PF`, ecode 0x2, in fversion's copy of the six-byte version string into a user buffer. The halt report's new line read `cpu 0 cr3=0x401000 pid 1`: the boot table, with process 1 running. The mistake was the fix's: at sched and reschedule the call sat after the swtch, on the resume path, replacing the table pick_next had just installed. clippy::undocumented_unsafe_blocks then also caught the call sitting between a SAFETY comment and its block."
  implication: "Moved before the swtch at all three sites, above the SAFETY comments; the lint error went with it."

- timestamp: 2026-11-28T02:40:00.000Z
  checked: "The corrected fix with a temporary print at every leave and at every pick, one boot"
  found: "Every pick reads `was 0x401000`: the scheduler installs each proc's table from the boot table. Every leave precedes its switch. 33048 picks and 33063 leaves over the workload; `churn: CATS-DONE 1400`."
  implication: "The discipline holds across the workload."

- timestamp: 2026-11-28T02:55:00.000Z
  checked: "The churn workload at -smp 2 under KVM with the fix (3ff89ab's proc.rs): 40 runs, then 24 on the final text"
  found: "64 of 64 complete (`churn: CATS-DONE 1400`, about 3.6 s each), none stops; 14 of 25 stopped before. cargo xtask test exits 0 (1254 passed); clippy on q35churnshell exits 0 with no new warning."
  implication: "Fixed."

- timestamp: 2026-11-28T03:15:00.000Z
  checked: "The composed x86_64 gates at the default -smp 2 under KVM on the committed tree (3ff89ab)"
  found: "churn PASSES (ALL STAGES PASS, churn: CATS-DONE 1400 exactly once, the bounded-memory window; 74 s). net-smoke PASSES (47 stages, 19 s). Smoke passes the whole chain leg and the CPU ladder through cpu-ether-ok and ends in `TIMEOUT at stage: cpu-keyfs-ok` after its 600 s wait (627 s), as main does at one CPU. tier-1: all stages to the prompt, post-pass missing only T1-OK fpprobe (the sleep defect of the control above). services: all stages to the prompt, post-pass missing srv and exportfs, the same SKIP line (main at -smp 1). auth: AUTH-FAIL, `syscall 10 unimplemented (pid 11 hmntgen)` then a fault at pc=0, the defect (4) family. No log holds a second boot. (main at the default -smp 2 in the auth gate, run by mistake as a control since that driver ignores R9_SMP, rebooted 1869 times in its 600 s and timed out at paqfs: defect (3).)"
  implication: "Every gate that passes on main at one CPU passes here at two; the rest stop on main's own defects."

## Constraints

- `port/src/mcslock.rs` is the lock primitive aarch64 shares: options (b) and (c) change it and need the owner's decision. Option (a) is x86_64-only (proc.rs's kstack mapping).
- Keep `#[forbid(unsafe_code)]` discipline and the UNSAFE-AUDIT.md lockstep for any `unsafe` block change.
- No push without the owner.

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: "Two defects, both single-CPU invariants meeting a second CPU. (1) port/src/mcslock.rs keeps each lock's queue node on the calling CPU's kernel stack (mcslock.rs:52 lock(node), :56 queue.swap, :64 predecessor.next.store); x86_64/src/proc.rs mapped every proc's 16 KiB kernel stack at the SAME VA (KSTACK_VA, PML4[400], each under its own PML4), so a CPU queuing behind a holder in process context wrote predecessor.next at that VA under its OWN page table, the waiter's stack or nothing, and the holder's real node was never linked: both CPUs spun, holder in unlock (mcslock.rs:95), waiter in lock (:65), interrupts masked. (2) The scheduler context ran on process page tables: pick_next installed the picked proc's root and nothing put the boot table back, so a CPU idled, and the sched loop ran, on the root of the proc it had left. When that proc exited, on this CPU (reap_moribund) or on the sibling that stole it, its root page returned to the frame allocator and was zeroed for reuse; a CPU still translating through it faults on the first address its TLB does not hold, and with the kernel half gone the fault handler faults too: a triple fault. (2) is new to this record: it was hidden behind (1) until the churn gate ran at two CPUs."
fix: "Branch fix/x86_64-smp2-kstack (stacked on debug/x86_64-ap-bringup): 667b545, one arena of MAX_PROCS contiguous 16 KiB slots reserved at boot by the page allocator's seeding and reached at the KZERO alias every page table carries (x86_64/src/kstack.rs; new_user_proc takes a slot, the deferred reap returns it after the exiting CPU has left the stack; the private PML4[400] window and its three table pages per proc are gone). 9a2b5a3, the console lock is held across a whole formatted line (port/src/devcons.rs). 3ff89ab, leave_proc_table puts the boot table in CR3 before each swtch into the scheduler (sched, reschedule, sched_from_moribund): a proc's table is in CR3 only while that proc is current, and the kernel's halt report prints the CPU, CR3 and the current pid."
verification: "On an x86_64 KVM host, 2026-11-27. Composed gates at the default -smp 2 on 3ff89ab: churn and net-smoke pass; smoke passes the chain leg and the CPU ladder to cpu-ether-ok and stops at cpu-keyfs-ok (defect 4, main's at one CPU); tier-1 misses only T1-OK fpprobe; services misses srv and exportfs; auth fails on the hmntgen/fauth family, the defect (4) fault. The controls at -smp 1 (the branch and main) were run for tier-1, services, churn and auth: each ends on the same line as at -smp 2. The churn workload looped under KVM at -smp 2: 64 of 64 complete, against 14 stops in 25 before 3ff89ab. The single-CPU TCG smoke leg (R9_SMP=1 R9_ACCEL=tcg) passes 31 stages and stops at cpu-keyfs-ok as on main. The two-CPU TCG smoke leg passes the same 31 stages. cargo xtask test exits 0 (1254 passed); clippy exits 0."
oracle_type: "specified - the smoke gate's stage list at -smp 2 (the brief's expected), plus host tests pinning the arena (contiguous 16 KiB slots, lowest-first, exhaustion, a second return refused) and the absence of PML4[400]"
files_changed: "x86_64/src/kstack.rs (new), x86_64/src/pagealloc.rs, x86_64/src/proc.rs, x86_64/src/dat.rs, x86_64/src/main.rs, x86_64/src/trap.rs, port/src/devcons.rs, UNSAFE-AUDIT.md"
