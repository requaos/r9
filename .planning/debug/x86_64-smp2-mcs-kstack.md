---
status: investigating
trigger: "x86_64 -smp 2 on BOTH accels, after the AP bring-up fixes (debug/x86_64-ap-bringup): the two CPUs wedge the first time one queues on an MCS lock whose holder runs in process context, because the hand-off writes the predecessor's node at a kernel-stack VA that is private to the holder's process. No panic and nothing on the wire — the boot stops after `paqfs...`. Blocks every x86_64 -smp 2 gate."
created: 2026-11-27T20:45:31.000Z
updated: 2026-11-27T20:45:31.000Z
---

## Current Focus
<!-- OVERWRITE on each update -->

hypothesis: "MCS locks keep their queue node on the calling CPU's kernel stack (`let node = LockNode::new(); lock.lock(&node)`) and the acquire writes `predecessor.next` (port/src/mcslock.rs:64). On x86_64 every proc's kstack is mapped at the SAME VA (the KSTACK_VA window, PML4[400]), each only under its own PML4 (x86_64/src/proc.rs:178-194). A CPU that queues behind a holder running in process context therefore writes at that VA under its OWN CR3 — unmapped while it is idle (#PF), its own proc's kstack while it is in process context (a silent 8-byte stack write) — and the holder's real node is never linked, so neither CPU completes."
test: "Pick the design, then prove it: (a) give every kstack a unique VA in a kernel-half window shared by all PML4s (the aarch64 model), (b) take the nodes off the stack — one node per lock per CPU inside the lock, or per-CPU nodes keyed by LockRank (ranks are strictly increasing per CPU, so one node per rank suffices), or (c) a lock with a global queue."
expecting: "With any of the three the hand-off reaches the real predecessor; `boot-r9-smoke-x64` at the default -smp 2 passes on TCG and on KVM, and `boot-r9-smp-stress-x64` stays green"
next_action: "Owner decision between (a), (b) and (c). The before-case capture is target/ap-debug/stk-e on the KVM host (gitignored; the stacks are transcribed under Evidence)"
bug_class: concurrency
reasoning_checkpoint: null
tdd_checkpoint: null

## Session Loop
goal: find_and_fix
tdd_mode: false
tdd_phase: none
resume_count: 0
awaiting: decision
question: "Which design for cross-CPU lock hand-off when kernel stacks are mapped at a per-process VA?"
last_next_action: none

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

## Constraints

- `port/src/mcslock.rs` is the lock primitive aarch64 shares: options (b) and (c) change it and need the owner's decision. Option (a) is x86_64-only (proc.rs's kstack mapping).
- Keep `#[forbid(unsafe_code)]` discipline and the UNSAFE-AUDIT.md lockstep for any `unsafe` block change.
- No push without the owner.

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause:
fix:
verification:
oracle_type:
files_changed: []
