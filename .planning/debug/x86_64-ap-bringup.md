---
status: awaiting_human_verify
trigger: "x86_64 -smp 2 under real KVM: the AP never comes online. The machine triple-faults while the AP is in 32-bit protected mode, immediately after the CR4 write, at the point the AP reads the handoff mailbox. TCG passes; KVM fails. Owner Stop 4 and x86_64 SMP-01 are blocked on this."
created: 2026-11-27T18:20:00.000Z
updated: 2026-11-27T21:23:57.000Z
---

## Current Focus
<!-- OVERWRITE on each update -->

hypothesis: "CONFIRMED and fixed on branch debug/x86_64-ap-bringup: the SIPI trampoline's 32-bit data segment was EXPAND-DOWN — `SegREAD|SegWRITE` sets descriptor bit 42, which is expand-down for a DATA descriptor — so every ds/es/ss access in 32-bit protected mode was a limit violation. TCG does not enforce the limit; KVM does. Two more bring-up defects sat behind it: 91187e8 started the APs twice on the dev_exec path, and the bring-up delays collapsed on the periodic timer venue."
test: "The composed smoke gate from the branch tip on KVM and on TCG, plus the single-shot experiments recorded under Evidence"
expecting: "KVM and TCG print `smp: cpu 1 online` and `smp: tlb shootdown ok` and the gate's smp-online stage passes. The gate still stops later on a separate defect (x86_64-smp2-mcs-kstack), which is NOT this session's."
next_action: "Owner verifies on an x86_64 KVM host: on the branch, `nix run .#boot-r9-smoke-x64` prints the two markers and passes stage smp-online; then fold the branch into main (no push without the owner)"
bug_class: bohrbug
reasoning_checkpoint:
  hypothesis: "SelDATA32 is expand-down because `SegWRITE` (1<<42) is the expand-down bit for DATA descriptors; with limit 0xFFFFFFFF the valid range is empty, so every ds/es/ss/fs/gs access in 32-bit protected mode raises #GP/#SS. The first such access is the mailbox load at 0x80B3; with IDTR = the post-INIT base 0 / limit 0xffff the fault is delivered through the firmware IVT and triple-faults."
  confirming_evidence:
    - "KVM register dump at the shutdown (QEMU monitor): CPU 1 EIP=0x80b3 (`mov eax,[0x8f00]`), CS =0010 base 0 limit ffffffff CS32, DS/ES/SS/FS/GS =0018 base 0 limit ffffffff AR 00c09700 (type 7 = expand-down, writable, accessed), CR0=0x11, CR3=0, CR4=0x6a0, EFER=0, IDT base 0 limit 0xffff, CPL 0"
    - "The GDT in the trampoline page: entry 3 = 0x00cf97000000ffff (access byte 0x97 once the CPU set Accessed; 0x96 in the source) = type 6/7 expand-down"
    - "gdb-stub single-step at 0x80b3: `xor eax,eax` (no memory) runs; a read via DS in every encoding (moffs, ModRM+SIB, byte, word, register-indirect), a read via FS and a `push` via SS all shut the VM down; the IDENTICAL read with a CS override (`2e a1 00 8f 00 00`) runs and returns the mailbox word 0x401000"
    - "With ONLY GDT entry 3's access byte rewritten in guest memory (0x96 -> 0x92) before the AP loads its segments, the AP runs the whole trampoline (AP:6 AP:C AP:T AP:J) and ap_main (AP:2..AP:6) and the BSP prints `smp: cpu 1 online` and `smp: tlb shootdown ok`"
  falsification_test: "If the data segments were usable the DS read and the CS-override read would behave alike; if the descriptor were not the cause, rewriting its access byte alone would not carry the AP through the trampoline. Both were tested; neither falsification occurred."
  fix_rationale: "Encode SelDATA32 as an expand-up writable data segment (type 2 = descriptor bit 41 only). That removes the limit violation itself. Nothing else on the path is wrong: flat CS, CR0/CR4/EFER, the trampoline copy and the mailbox were all verified correct at the fault."
  blind_spots: "Only the in-memory byte rewrite has been exercised; the rebuilt kernel has not yet run under KVM or TCG. After the AP comes online a SECOND defect shows: exec_bootstrap (main.rs:956, 7977d1c) starts the APs again, so 91187e8's early call in the arming window makes the composed (dev_exec) boot start them TWICE — the second start INIT-resets the online AP, re-copies the trampoline and (with the old descriptor) dies again right after the `exec: entering ring 3` print, which is the original STATE.md symptom. That confirms 91187e8's premise ('the composed path never started APs') was false."
  candidate_causes:
    - "code: the SelDATA32 descriptor encoding (confirmed)"
    - "code, contributing on the composed path: the double AP start (91187e8 against 7977d1c's claim-then-start ordering)"
    - "environment: KVM enforces segment limits and TCG does not — this explains TCG green / KVM red but is not a cause"
    - "data: excluded — the trampoline copy at 0x8000 and the mailbox at 0x8f00 were dumped and are byte-exact"
  and_gate: "yes for the composed gates: the dev_exec boot needs BOTH the descriptor fix and a single AP start. With only the descriptor fix the bootstrap-tail start INIT-resets the online AP and double-counts it; the early start alone would carry the gate but violates the 05-22 claim-then-start ordering (a secondary already in sched_loop can steal pid 1 inside its Ready->Running window)."
tdd_checkpoint: null

## Session Loop
goal: find_and_fix
tdd_mode: false
tdd_phase: none
resume_count: 0
awaiting: human_verify
question: "Is the bring-up fix accepted (AP online + shootdown self-check on KVM and TCG) so the session can be archived? The composed gate stays red at -smp 2 on x86_64-smp2-mcs-kstack."
last_next_action: "Owner verifies the branch on a KVM host and decides whether to fold it into main; separately, the design decision x86_64-smp2-mcs-kstack is waiting on"

## Symptoms
<!-- IMMUTABLE -->

expected: "On an x86_64 Linux KVM host, `nix run .#boot-r9-smoke-x64` (and tier-1/rio) boots two CPUs: `smp: cpu 1 online` and `smp: tlb shootdown ok` appear, and the gate reaches its prompt stages."
actual: "The BSP sends INIT + SIPI; the AP starts and prints the trampoline markers AP:R, AP:G, AP:P, AP:4, AP:5 and then the machine triple-faults (QEMU reports `qemu exited (EOF) at stage: smp-online` with -no-reboot). No AP:C, no `smp: cpu 1 online`. TCG passes, so the path works under emulation and fails on real KVM."
errors: "QEMU exits (EOF) — a triple fault. Neither `-d int` nor `-d guest_errors` reports the guest exception under KVM (KVM handles it in-kernel)."
reproduction: "On an x86_64 Linux host with a writable /dev/kvm: `git pull`, then `nix run .#boot-r9-smoke-x64` (defaults to KVM, -cpu host, -smp 2). scripts/kvm-ap-debug.sh wraps this with log capture; add --gdb to expose the QEMU gdbstub."
started: "The x86_64 AP path was never exercised on KVM until Owner Stop 2: the composed gates boot dev_exec kernels, and start_aps was cfg'd out of dev_exec until 91187e8. The first real AP start exposed this."

## Marker legend (instrumentation as it was during this session — removed in 5c22f03)

`AP:` lines are raw COM1 bytes emitted by the AP itself (x86_64/src/ap_trampoline.S and
x86_64/src/main.rs::ap_mark). The BSP is silent during the AP window so the sequence is clean.

| Marker | Meaning |
|---|---|
| AP:R | real-mode entry, flat segments |
| AP:G | GDT loaded (lgdtl) |
| AP:P | 32-bit protected mode entered (far jump done, data segments loaded) |
| AP:4 | CR4 read (`movl %cr4,%eax`) |
| AP:5 | CR4 written (`movl %eax,%cr4`, bits PAE|PGE|OSFXSR|OSXMMEXCPT) |
| AP:6 | mailbox PML4 PA read (`movl (0x8F00),%eax`) |
| AP:C | CR3 loaded |
| AP:T | 64-bit long mode entered |
| AP:J | about to jump to ap_main |
| AP:2..6 | ap_main stages (entry, IDT/GDT/TSS, SYSCALL MSRs, FPU, LAPIC, online) |

## Eliminated
<!-- APPEND only - prevents re-investigating after /clear -->

- hypothesis: "The second SIPI retry kills the AP (a SIPI to an AP already in protected mode restarts it without resetting CR0/CR4)."
  evidence: "Real and fixed: the retry is now removed (ac2eea3) and a missed SIPI fails cleanly. The trace is unchanged afterwards — the AP still dies at AP:5 — so the retry was a genuine second bug, not this one."
  timestamp: 2026-11-27T18:20:00.000Z

- hypothesis: "The QEMU `-d int` / `-d guest_errors` log will name the exception."
  evidence: "Both logs contain only the power-on CPU resets (112 lines). Under KVM the exception never surfaces to QEMU's log path."
  timestamp: 2026-11-27T18:20:00.000Z

- hypothesis: "The mailbox is written to the wrong address (base double-added)."
  evidence: "The MB_* constants in smp.rs are OFFSETS (0x00,0x08,...) and `mb.add(off/8)` is correct; the trampoline's MB_* values are absolute (0x8F00+off). Verified by inspection and by disassembly (`a1 00 8f 00 00`)."
  timestamp: 2026-11-27T18:20:00.000Z

- hypothesis: "The trampoline instruction stream is broken by the inserted markers."
  evidence: "Disassembly of the built ELF shows `89 d8` (mov %ebx,%eax), `a1 00 8f 00 00` (mov eax,[0x8F00]), `89 c3`, then the AP:6 marker — all correct."
  timestamp: 2026-11-27T18:20:00.000Z

- hypothesis: "bootpml4 does not identity-map the low page, so enabling paging faults."
  evidence: "bootpml4[0] = bootidentitypt3 maps the first 4 GiB as 1 GiB leaves (l.S). Not reached anyway: the fault is before CR3 is loaded."
  timestamp: 2026-11-27T18:20:00.000Z

- hypothesis: "The AP is not in the flat 32-bit protected-mode state the trampoline assumes (CS.L=1 or a CR0/CR3 inconsistency), so the 32-bit moffs read is a wild address."
  evidence: "QEMU monitor dump of CPU 1 frozen at the shutdown: CS =0010 base 0 limit ffffffff AR 00c09b00 (CS32, D=1, L=0), CR0=0x11, CR3=0, CR4=0x6a0, EFER=0. The mode is exactly as assumed."
  timestamp: 2026-11-27T19:40:03.000Z

- hypothesis: "The fault is a guest exception an early IDT could name (vector + error code)."
  evidence: "A diagnostic IDT (32 386 interrupt gates at 0x7200) loaded by a patched trampoline, with ESP moved onto RAM first: the stack page stays untouched (nothing is ever pushed) and the handler never runs, yet the VM still ends `paused (shutdown)` at EIP=0x80b3. That was the clue: the fault is not in what the load reads but in the segment it goes through."
  timestamp: 2026-11-27T19:40:03.000Z

- hypothesis: "KVM exits to QEMU with an MMIO or exception exit for the AP's access."
  evidence: "QEMU trace of every KVM exit (`-d trace:kvm_run_exit`): CPU 1 makes exactly its 30 marker port exits and then one terminal exit; no MMIO exit and no userspace exception exit reaches QEMU."
  timestamp: 2026-11-27T19:40:03.000Z

## Evidence
<!-- APPEND only -->

- timestamp: 2026-11-27T18:20:00.000Z
  checked: "Clean marker trace from the KVM smoke boot (kvm-evidence-20261127T170057Z)"
  found: "AP:a AP:b AP:a AP:b (INIT and SIPI ICR writes returned) then AP:R AP:G AP:P AP:4 AP:5, ending with a COMPLETE `AP:5\\r\\n`; no AP:6. QEMU then exited (EOF) at stage smp-online. BSP evidence: MADT ncpus=2, bsp apic=0x0."
  implication: "The AP is alive through real mode, GDT load, the protected-mode far jump, the data-segment loads, the CR4 read and the CR4 write. The fault is the 32-bit moffs read of 0x8F00 or the instruction immediately around it."

- timestamp: 2026-11-27T18:20:00.000Z
  checked: "The four instructions between AP:5 and AP:6"
  found: "`movl %ebx,%eax` (89 d8), `movl (0x8F00),%eax` (a1 00 8f 00 00), `movl %eax,%ebx` (89 c3). Only the moffs read touches memory. With CR0.PG=0 it is a plain physical read of conventional RAM and cannot fault; under 64-bit mode the same bytes are a `movabs` with a wild 8-byte address and fault immediately."
  implication: "The AP is very likely NOT in the flat 32-bit state assumed — that is the experiment to settle first"

- timestamp: 2026-11-27T18:20:00.000Z
  checked: "Why x86_64 two-CPU was never seen failing before"
  found: "start_aps_and_self_check was `#[cfg(all(not(test), not(dev_exec)))]`; every composed gate boots a dev_exec kernel (q35churnshell dev list includes exec). Commit 91187e8 enabled it for dev_exec; this fault appeared immediately."
  implication: "The AP path has effectively never run on KVM until now"

- timestamp: 2026-11-27T19:40:03.000Z
  checked: "Baseline re-run on the KVM host at HEAD 0e783a0 through the composed smoke gate with a -no-reboot wrapper"
  found: "Same marker chain (AP:R G P 4 5, then `qemu exited (EOF) at stage: smp-online`); the wire ends on a complete AP:5 marker with no stray byte. Capture: target/ap-debug/baseline-gate.log."
  implication: "The fault is inside the five instructions between AP:5's last outb and AP:6's first outb"

- timestamp: 2026-11-27T19:40:03.000Z
  checked: "Disassembly of the trampoline bytes from the built kernel ELF (nix store kernel-x64 for 0e783a0)"
  found: "Real-mode part ends at 0x804c, ap_prot32 at 0x8050, the mailbox load `a1 00 8f 00 00` at 0x80b3, AP:6 at 0x80ba. The copy in guest memory at 0x8000 matches byte for byte."
  implication: "The stream is intact; the faulting instruction is the first explicit data access of the AP's 32-bit life"

- timestamp: 2026-11-27T19:40:03.000Z
  checked: "CPU 1 frozen at the triple fault: `qemu -accel kvm -cpu host -smp 2 -no-reboot -no-shutdown`, then HMP `cpu 1` + `info registers`"
  found: "EIP=000080b3 EAX=EBX=0x6a0 EDX=000903f8 ESP=0; CS =0010 00000000 ffffffff 00c09b00 CS32; ES/SS/DS/FS/GS =0018 00000000 ffffffff 00c09700; GDT 00008180/1f; IDT 00000000/ffff; CR0=00000011 CR2=0 CR3=0 CR4=000006a0 EFER=0; CPL 0. Mailbox at 0x8f00 = 0x401000, stack top, Mach VA, cpu 1, ap_main 0xffffffff80201300. GDT entry 3 in memory = 0x00cf97000000ffff."
  implication: "Flat 32-bit protected mode with paging off, CPL 0 — yet the load faults. Every data segment carries type 0x7 (expand-down)."

- timestamp: 2026-11-27T19:40:03.000Z
  checked: "Hardware breakpoint on CPU 1 at 0x80b3, then single-step variants written over the instruction through the gdb stub (target/ap-debug/flex.py)"
  found: "`31 c0` (xor, no memory): runs. Via DS: `a1 00 8f 00 00`, `a1 00 80 00 00`, `8b 04 25 00 8f 00 00`, `a0 00 8f 00 00`, `66 a1 ..`, `8b 03` (all shutdown). Via FS `64 a1 ..` (shutdown). `push eax` via SS with ESP=0x7100 (shutdown). Via CS `2e a1 00 8f 00 00`: RUNS, EAX=0x401000."
  implication: "Every access through a data-segment register is a fault; the same access through CS is fine. The data segments are unusable: expand-down with an empty valid range."

- timestamp: 2026-11-27T19:40:03.000Z
  checked: "Break CPU 1 at ap_prot32 (0x8050), rewrite only GDT entry 3's access byte in guest memory 0x96 -> 0x92, continue (target/ap-debug/confirm.py)"
  found: "AP:R G P 4 5 6 C T J 2 3 4 5 6, `lapic: ap 1 online-ready (mode X2Apic)`, BSP prints `smp: cpu 1 online (apic 0x1)` and `smp: tlb shootdown ok`, boot proceeds through fpu/setops selftests, paqfs mount and devroot open; then `exec: entering ring 3`, a second INIT+SIPI (exec_bootstrap's start_aps_and_self_check re-copies the ORIGINAL trampoline), and AP:R G P 4 5 then shutdown."
  implication: "The one-byte descriptor change is sufficient to bring the AP online. A second, independent defect remains on the composed path: the APs are started twice."

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "Descriptor fix alone (early start of 91187e8 still in place), KVM, chain-leg shape"
  found: "`smp: cpu 1 online` and `smp: tlb shootdown ok` in the arming window; then the AP double-faulted right after the IPI that follows exec_into, and the bootstrap tail's start INIT-reset it: `smp: shootdown ipi to unregistered cpu 2 skipped` — three CPUs counted on a two-CPU machine."
  implication: "91187e8's premise was false (exec_bootstrap already starts the APs); with both sites live the composed boot starts every AP twice and a secondary in sched_loop takes the pid-1 enqueue before the BSP claims it"

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "Descriptor fix + single start (main.rs cfg restored), KVM"
  found: "Clean bring-up at the tail; `exec: entering ring 3`, paqfs, version, chain.stage; then `panicked at` cut to 12 chars and both CPUs in AtomicBool::load (MCS wait). The panic's location read from PanicInfo: x86_64/src/proc.rs:2794:21 — pick_next's kstack-ownership tripwire, args pid 2, cpu 0, kstack 0xffffc80000004000."
  implication: "The tripwire compares kstack VAs, and on x86_64 every proc's kstack is the same window VA (proc.rs:178-194): a false positive for any proc a second CPU picks while the first runs a process. Its console print then deadlocked on a separate defect (x86_64-smp2-mcs-kstack)."

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "Composed smoke gate under TCG on the branch (qemu64, -smp 2): wall-clock timeline from the ring-3 line (target/ap-debug/timeline.py)"
  found: "`smp: cpu 1 (apic 0x1): AP did not come online (online flag timeout)` at +1.3 ms; the AP's `lapic: ap 1 online-ready` at +2.2 ms, unsupervised. Banner: `timer: … Periodic mode`. timer.rs's periodic arm publishes `tsc_delta: 0`; start_one_ap derives both delays from it (`ten_ms = tsc_delta.max(1)`, `online_timeout = tsc_delta*10 … .max(1_000_000)`)."
  implication: "On the periodic venue the SIPI goes out one cycle after INIT and the poll gives up ~0.4 ms later. The brief's 'TCG passes' did not hold for the two-CPU path: CI's x86_64 smoke job pins R9_SMP=1 (gates.yml:148-158)."

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "After 3f1b76a (tsc_delta recorded in periodic mode), TCG timeline"
  found: "online-ready +12.0 ms, `smp: cpu 1 online` +12.3 ms, `smp: tlb shootdown ok` +13.4 ms; then `paqfs...panicked at x86_64/src/proc.rs:2794:21:` — the tripwire again."
  implication: "Bring-up is correct on TCG too; the tripwire false positive is the next thing in the way"

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "After fed9be7 (tripwire keyed on the kstack top page PA), KVM and TCG"
  found: "No tripwire. TCG: `paqfs...version...fault: text page-in va=0x202000` — the AP runs the forked process. KVM: CPU 1 in process context (CR3 0x6f67000); both boots then stop silently: CPU 1 in Chan::dup → alloc → heap MCS lock → AtomicBool::load (mcslock.rs:65); CPU 0 in a namec walk → dealloc → IrqLockGuard<QuickFit>::drop (mcslock.rs:489) → compare_exchange_weak, unable to unlock the heap lock it holds."
  implication: "That is x86_64-smp2-mcs-kstack: the MCS hand-off writes the predecessor's node at a VA private to the holder's process. Recorded as its own prepared session."

- timestamp: 2026-11-27T20:45:31.000Z
  checked: "Composed smoke gate on the branch tip fed9be7, KVM"
  found: "stage ok — banner, fpu-format, smp-online, paqfs, version; then the wire stops at stage prompt."
  implication: "The stage this session was opened for (smp-online: `qemu exited (EOF) at stage: smp-online`) passes under KVM"

- timestamp: 2026-11-27T21:11:12.000Z
  checked: "Composed smoke gate, KVM, branch tip fed9be7 (target/ap-debug/gate-kvm-smoke-2.log)"
  found: "stage ok — banner, fpu-format, smp-online, paqfs, version; then TIMEOUT at stage: prompt (GATE_EXIT=1)."
  implication: "The stage this session was opened for (`qemu exited (EOF) at stage: smp-online`) passes under KVM. What remains is x86_64-smp2-mcs-kstack."

- timestamp: 2026-11-27T21:11:12.000Z
  checked: "Composed smoke gate, TCG, R9_SMP=1 — CI's job boot-r9-smoke-x64-smp1 — on the branch (target/ap-debug/gate-tcg-smp1.log)"
  found: "Chain leg: every stage through nic-bind. CPU leg through cpu-ether-ok — 31 stages ok in all — then TIMEOUT at stage: cpu-keyfs-ok (GATE_EXIT=1)."
  implication: "The single-CPU boot is healthy on the branch. cpu-keyfs-ok is the open defect (4) in STATE.md (new since 5a3d5e4); the same leg at main: identical on `main` (0e783a0, its own kernel build, R9_SMP=1 R9_ACCEL=tcg): the same 31 stages pass and the run ends in the same `TIMEOUT at stage: cpu-keyfs-ok` — the branch does not change the single-CPU leg."

- timestamp: 2026-11-27T21:11:12.000Z
  checked: "`cargo xtask test` on the host"
  found: "Before 2e1ae23/4342436: 40 test binaries pass and the x86_64 crate's leg fails to compile — `PhysAddr` imported under cfg(not(test)) while evict_user_frame (1f69086) names it in every build. With the import fixed: 152 pass, 1 fails — pf_error_code_bits_decode's 'unrelated high bits' input 0xffff_ffff_ffff_fff0 has bit 4 (I/D) set. After both: exit 0, 55 test binaries."
  implication: "Pre-existing, visible only on an x86_64 host (05-19's 'cargo xtask test exited 0' was recorded on the aarch64 host). Committed on the owner's instruction rather than shimmed and reverted."

- timestamp: 2026-11-27T21:11:12.000Z
  checked: "Kernel impact of the two test repairs (reproducible nix builds: fed9be7 f3d1cda3…, tip a5f2789c…)"
  found: ".text, .rodata and .data byte-identical; identical LOAD segments; 4062 defined symbols at the same addresses; .strtab 59 bytes shorter — only LLVM's `.llvm.<hash>` suffixes on private symbols of the proc and smp codegen units."
  implication: "Kernel code and data are unchanged, so the gate runs on fed9be7 stand for the tip. A first draft of 2e1ae23's message said 'the kernel does not change'; that was wrong as measured and the commit was recreated with the measured statement."

## Experiment plan (run on the Linux KVM host)

Ordered by cost. Stop at the first one that resolves the fault; record the result in Evidence.

1. **gdb the AP fault (decisive).** `bash scripts/kvm-ap-debug.sh --gdb`, then in another shell
   `gdb -q target/x86_64-unknown-none-elf/debug/x86_64 -ex 'target remote :1234' -ex 'continue'`
   and capture `info registers cs cr0 cr3 efer eip` at the stop. QEMU reports guest exceptions as
   signals; if it stops on SIGSEGV/SIGTRAP at the AP, that is the fault. Record CS.L (bit 53 of the
   CS descriptor is not in the register; use `info registers` + QEMU monitor `info cpus`), CR0, CR3, EFER.
2. **Register-indirect mailbox read (one-line code change).** Replace
   `movl (MB_PML4_PA), %eax` with `movl $MB_PML4_PA, %ebx; movl (%ebx), %eax`. This encoding is
   valid in 32- and 64-bit mode, so it isolates the moffs form. If AP:6 now appears, the AP was in a
   mode where moffs is wrong — find why (CS descriptor, EFER, INIT state).
3. **CR4 bit isolation.** Set only CR4.PAE first, then add PGE, then OSFXSR|OSXMMEXCPT, one marker
   each. A #GP at one bit names it.
4. **Verify the AP's INIT actually reset it.** Mark the AP entry with CR0/CR3/EFER captured into low
   memory (e.g. 0x7000) before CR4, and read those cells from the BSP after the online timeout (or
   via the QEMU monitor). If CR0.PG or EFER.LMA is already set, the AP was never INITed — fix the
   INIT delivery/verification.
5. **Fallback staging stub.** Have the AP's first long-mode act be a jump to a pre-linked 64-bit stub
   that does the whole handoff in Rust (`ap_main` already exists); i.e. keep the trampoline to the
   minimum needed to enter long mode and never touch a moffs.

## Constraints

- Keep `#[forbid(unsafe_code)]` discipline and the UNSAFE-AUDIT.md lockstep for any `unsafe` block change.
- No push of product code without the owner: commit locally, push `ap-debug-*` evidence branches.
- Remove the AP markers and the silent-window instrumentation once the fix lands (they are diagnostic).
- The committed gate expectations are correct: the smoke gate SHOULD see `smp: cpu 1 online` at `-smp 2`.

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: "(1) x86_64/src/ap_trampoline.S spelled the SIPI trampoline's 32-bit data segment `SegREAD|SegWRITE` (descriptor bits 41|42). For a DATA descriptor bit 42 is expand-down and bit 41 is write-enable, so SelDATA32 was an expand-down segment (type 6) with base 0 and limit 0xFFFFF — a valid offset range of nothing. The AP's first access through a data-segment register in 32-bit protected mode (the mailbox load at 0x80B3) was a limit fault; IDTR was still the post-INIT one, so delivery went through the firmware IVT and triple-faulted. TCG does not enforce segment limits, which is why no TCG run showed it. (2) Contributing on the composed path: 91187e8 made the arming window start the APs under dev_exec, where exec_bootstrap (7977d1c) already starts them after pid 1 is claimed — a double start, and a secondary in sched_loop for the whole bootstrap. (3) Found by the verification, in the same function: timer::init recorded `tsc_delta` only in TSC-deadline mode, so on the periodic venue start_one_ap's 10 ms INIT→SIPI wait and 100 ms online poll collapsed to ~0 and the BSP reported `AP did not come online`. (4) Found by the verification, debug builds only: pick_next's kstack-ownership tripwire compared kstack VAs, the same window VA for every proc on x86_64."
fix: "Branch debug/x86_64-ap-bringup, main untouched at 0e783a0, nothing pushed: b4ec82f encode SelDATA32 writable expand-up (type 2) and document the positional type bits; 5652bec restore `#[cfg(all(not(test), not(dev_exec)))]` on the arming-window call so each boot path starts the APs exactly once; 5c22f03 remove the COM1 AP markers, ap_mark and the ICR markers and correct the SIPI-retry comments ac2eea3 left (the old retry was conditional on `started` and cannot have killed an AP that died of the segment fault — the code keeps the single SIPI); 3f1b76a record tsc_delta in the periodic timer arm too; fed9be7 key the tripwire on the kstack top page PA; 2e1ae23 and 4342436 repair the x86_64 crate's host-test leg, which did not compile before (pre-existing since 1f69086) — no change to kernel code or data."
verification: "KVM (i7-1260P, -cpu host, -smp 2), composed smoke gate from fed9be7: stages banner, fpu-format, smp-online, paqfs and version pass, with `smp: cpu 1 online (apic 0x1)` and `smp: tlb shootdown ok` on the wire; the gate then stops at stage prompt on x86_64-smp2-mcs-kstack. TCG (qemu64, -smp 2) timeline from the ring-3 line: online-ready +12.0 ms, `smp: cpu 1 online` +12.3 ms, `smp: tlb shootdown ok` +13.4 ms (before 3f1b76a: `AP did not come online` at +1.3 ms). TCG with R9_SMP=1 (CI's x86_64 smoke job), branch: the whole chain leg passes and the CPU leg passes through cpu-ether-ok (31 stages), then TIMEOUT at stage cpu-keyfs-ok — the open defect (4) in STATE.md; the same leg at main: identical on `main` (0e783a0, its own kernel build, R9_SMP=1 R9_ACCEL=tcg): the same 31 stages pass and the run ends in the same `TIMEOUT at stage: cpu-keyfs-ok` — the branch does not change the single-CPU leg. Root-cause oracles: rewriting only GDT entry 3's access byte in guest memory (0x96 → 0x92) carried the AP through the trampoline and ap_main; single-stepping variants at 0x80B3 faulted for every DS/ES/FS/SS access and succeeded with a CS override (EAX = 0x401000). build and `cargo xtask clippy --arch x86-64 --config q35churnshell` clean (65 pre-existing warnings, unchanged); `cargo xtask test` exits 0 (55 test binaries, the x86_64 leg 153 tests). The tip's reproducible kernel (nix build, a5f2789c…) has .text, .rodata and .data byte-identical to fed9be7's (f3d1cda3…): only LLVM's `.llvm.<hash>` private-symbol suffixes differ, so the gate runs on fed9be7 stand for the tip. NOT green: the composed gate at -smp 2 on either accel, blocked by x86_64-smp2-mcs-kstack; and at -smp 1 past cpu-ether-ok, on the pre-existing defect (4)."
oracle_type: "specified — the gate's committed expectation `smp: cpu 1 online` at -smp 2 (gates/x86_64/smoke-r9.expect) and the marker pair named in Symptoms.expected"
files_changed: [x86_64/src/ap_trampoline.S, x86_64/src/main.rs, x86_64/src/smp.rs, x86_64/src/platform/q35/lapic.rs, x86_64/src/platform/q35/timer.rs, x86_64/src/proc.rs, x86_64/src/dat.rs, x86_64/src/fault.rs]
