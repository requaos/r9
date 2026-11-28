---
status: awaiting_human_verify
trigger: "x86_64 cpu-chain stops at cpu-keyfs-ok at one CPU and at two (the owner's open defect (4)): the keyfs rung's helper hmntgen faults at pc=0, cpurc: FAIL keyfs cascades down the ladder, and the composed smoke gate's 600 s wait for KEYFS-OK reads as a hang. The same fault ends the auth gate (AUTH-FAIL keyfs) and the services gate's srv and exportfs. An earlier brief blamed hmntgen's fauth call; that was wrong."
created: 2026-11-28T15:10:00.000Z
updated: 2026-11-28T15:10:00.000Z
---

## Current Focus
<!-- OVERWRITE on each update -->

hypothesis: "CONFIRMED and fixed in d63c2a4. A shared-memory fork (RFMEM) swept the pages its parent lacked, from the page the data segment ends in, and zero-filled every one — in the parent as in the child.  That page carries the tail of the initialised data beside the head of bss.  x86_64's exec leaves the data segment demand-paged, so a program that had not touched the page lost its statically initialised data at its first RFMEM fork: the C library's heap pool lost its lock hook (a null function pointer), and every 9P server the boot chain forks that way died at pc=0.  A server that posts a service name writes the page first and survived, which is what made it look like a hang in some gates and a crash in others."
test: "Host tests over a mock parent with an image-backed data segment; the in-guest reproduction (hmntgen without -s at /tmp, /mnt and /rc); the composed x86_64 gates at -smp 2 and -smp 1 under KVM; a mutation check (the Data arm disabled must fail the new test)"
expecting: "The new test fails on main's sweep and passes on the fix; hmntgen mounts without a service name; smoke passes cpu-keyfs-ok and the rest of the ladder at one and two CPUs; auth and services pass their legs"
next_action: "Owner verifies on a KVM host on the stack (d63c2a4 or later): nix run .#boot-r9-smoke-x64 passes 38 stages at the default -smp 2 and 37 at R9_SMP=1; .#boot-r9-services-x64 passes 15; then decides how the stack lands"
bug_class: memory
reasoning_checkpoint: null
tdd_checkpoint: null

## Session Loop
goal: find_and_fix
tdd_mode: false
tdd_phase: none
resume_count: 0
awaiting: human_verify
question: "Is the sweep fix accepted (an absent Data page is read from the image and shared, an absent Bss page is born one zero page in both tables, any other page stays absent) so the record can be closed?"
last_next_action: "Root-cause defect (4) on the two-CPU stack, with single-CPU controls"

## Symptoms
<!-- IMMUTABLE -->

expected: "`nix run .#boot-r9-smoke-x64` passes its CPU ladder (cpu-keyfs-ok, cpu-authsrv-listen, cpu-factotum-ok, cpu-listen-17010, cpu-ladder-done) at -smp 1 and at -smp 2."
actual: "The ladder passes cpu-pair-ok and cpu-ether-ok and stops: `cpurc: FAIL keyfs`, then FAIL authsrv-listen, factotum, listen-17010, sweep, a prompt, and `TIMEOUT at stage: cpu-keyfs-ok` after 600 s.  Before them on the wire: `syscall 10 unimplemented (pid N hmntgen)` and a user fault at pc=0.0."
errors: "hmntgen's exit: `fault: ... pc=0x0` (a call through a zeroed function pointer).  The auth gate: `AUTH-FAIL keyfs` in 6 s.  The services gate: `SVC-FAIL srv`, `SVC-FAIL exportfs`."
reproduction: "main at 9daa0d5 (and every earlier two-CPU branch), KVM or TCG: `R9_SMP=1 R9_ACCEL=tcg nix run .#boot-r9-smoke-x64` passes 31 stages and times out at cpu-keyfs-ok; `nix run .#boot-r9-auth-x64` prints AUTH-FAIL keyfs."
started: "Latent since the RFMEM sweep was written.  Reached on 2026-11-27 as soon as x86_64 could run the CPU ladder at all (the AP bring-up fixes); recorded then as defect (4) with the wrong cause."

## Eliminated
<!-- APPEND only - prevents re-investigating after /clear -->

- hypothesis: "hmntgen's `fauth` (syscall 10, unimplemented) is the fault: the dispatch table leaves it a loud -1 and the program then jumps to zero."
  evidence: "The -1 is what lets the mount proceed: the kernel refuses an auth fd that is not -1 on mount, and every working boot (the services gate's own mounts, aarch64's gates) logs the same `syscall 10 unimplemented` line for boot, init and hfactotum.  The syscall number is a bystander; hmntgen dies after it, in library code, at a null function pointer."
  timestamp: 2026-11-28T15:10:00.000Z

- hypothesis: "Two CPUs: a race between the keyfs rung's processes."
  evidence: "The single-CPU leg (R9_SMP=1) stops at the same stage with the same fault, and so does main at one CPU (control)."
  timestamp: 2026-11-28T15:10:00.000Z

- hypothesis: "hmntgen is missing from the image (a `namec: does not exist` seen in an experiment)."
  evidence: "That experiment ran the tier image, which has no hmntgen; the shell failed to exec it.  The services and auth images carry it.  A second false lead: two `devmnt: server error` lines and the gate's own 12-fold `cat /lib/namespace` loop look like a boot loop and are neither."
  timestamp: 2026-11-28T15:10:00.000Z

## Evidence
<!-- APPEND only - facts discovered -->

- timestamp: 2026-11-28T15:10:00.000Z
  checked: "The dying process's instructions and data (r9dis.py over the services image's hmntgen and lib9p, the Plan 9 a.out header: data offset 12, bss 16)"
  found: "pc=0 is `poolalloc`'s call through the heap pool's lock hook: the C library's `sbrkmem` Pool struct keeps `.lock = plock`, a function pointer, in the data segment; after the fault the word reads zero."
  implication: "Initialised data was zeroed in a running process.  The question became which kernel path zeroes a data page of a process that did not write it."

- timestamp: 2026-11-28T15:10:00.000Z
  checked: "The RFMEM fork's sweep (x86_64/src/proc.rs, rfork_copy) and where the data segment ends"
  found: "The sweep ran from `(bss.start & !0xfff)` — the page the data segment ENDS in when data ends mid-page — to the end of bss, and mapped one fresh zero page into both procs for every page the parent lacked.  On x86_64 exec leaves data demand-paged, so that page is absent in a program that has never touched it; its bytes are the tail of the initialised data."
  implication: "Root cause.  With `-s name` the server posts a service name first: postfd's printf mallocs and touches the page, so it is present and shared, not zeroed; without `-s` nothing touches it and the sweep destroys it.  That is why some 9P servers survived and the ones the keyfs rung needs did not."

- timestamp: 2026-11-28T15:10:00.000Z
  checked: "The same sweep on aarch64 (aarch64/src/proc.rs)"
  found: "The same range and the same zero-fill.  aarch64's exec maps data eagerly, so there the page is absent only once the evictor has dropped it: latent, not reached by a gate."
  implication: "Mirror the fix on both."

- timestamp: 2026-11-28T15:10:00.000Z
  checked: "In the guest, before the fix: hmntgen without -s at /tmp, /mnt and /rc on the services image"
  found: "All three die at pc=0.  After the fix: all three mount."
  implication: "The reproduction and the proof."

- timestamp: 2026-11-28T15:10:00.000Z
  checked: "Host tests on the fix, and a mutation check"
  found: "`rfork_mem_shares_the_data_and_bss_pages_the_parent_never_touched` (Data, Bss, Bss records at the same PA in parent and child, the data bytes intact, the bss head zero) and `a_plain_fork_materialises_nothing` pass.  With the sweep's Data arm disabled the first fails: `the parent has 0x10002000`."
  implication: "The tests have teeth."

- timestamp: 2026-11-28T15:10:00.000Z
  checked: "The composed x86_64 gates under KVM on the fix (d63c2a4's tree), with single-CPU legs"
  found: "smoke PASSES 38 stages at -smp 2 (the first full pass of this gate on any two-CPU tree) and 37 at -smp 1; services PASSES 15 (srv and exportfs included); tier-1 PASSES at -smp 2 and at -smp 1; churn and net-smoke PASS.  auth reaches every leg of its sweep and then stops in the gate's own script (the secstore markers' order — a driver defect, 6808f2e); with that fixed it passes 33 stages, three runs of three."
  implication: "Defect (4) and the services failures have this one cause."

## Constraints

- The sweep's purpose stands: under RFMEM the whole data segment and bss must end up shared, a page the parent has not touched included (the donor's shared segment shows both procs the same page).  The fix keeps the sweep and changes what an absent page becomes.
- No push without the owner.

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: "The RFMEM sweep in rfork_copy (both architectures) zero-filled every page it found absent from the page the data segment ends in to the end of bss, in the parent as in the child.  On x86_64, where exec leaves the data segment demand-paged, that destroyed the tail of a program's statically initialised data at its first shared-memory fork; the C library's heap pool lost its lock hook and 9P servers forked without a posted service name died at pc=0, which is what the keyfs, auth and services gates were stopping on."
fix: "d63c2a4 (branch fix/rfork-bss-sweep-data-tail): the sweep runs from the data segment's start and materializes an absent page by what it holds — a Data page is read from the image for the parent and shared (rfork_share_mapping), a Bss page is born one zero page mapped into both tables, and any other page stays absent.  aarch64 carries the same rule.  Two tests pin it on x86_64."
verification: "KVM host, 2026-11-28.  Host: 1300 tests pass on the stack's tip.  Guest: hmntgen mounts without -s at /tmp, /mnt and /rc.  Gates on the fix tree at the default -smp 2: smoke 38 stages, services 15, tier-1, churn, net-smoke pass; smoke at -smp 1 passes 37; auth passes once its driver's marker order is freed (6808f2e)."
oracle_type: "specified - the gates' stage lists; the donor's RFMEM semantics (the shared segment shows both procs the same page); the C library's pool layout as read from the binary"
files_changed: "x86_64/src/proc.rs (sweep, rfork_share_mapping, tests), aarch64/src/proc.rs (sweep, rfork_share_mapping)"
