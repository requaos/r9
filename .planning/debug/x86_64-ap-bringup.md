---
status: investigating
trigger: "x86_64 -smp 2 under real KVM: the AP never comes online. The machine triple-faults while the AP is in 32-bit protected mode, immediately after the CR4 write, at the point the AP reads the handoff mailbox. TCG passes; KVM fails. Owner Stop 4 and x86_64 SMP-01 are blocked on this."
created: 2026-11-27T18:20:00.000Z
updated: 2026-11-27T18:20:00.000Z
---

## Current Focus
<!-- OVERWRITE on each update -->

hypothesis: "The AP is executing the trampoline in a CPU mode/state where the 32-bit `movl moffs` mailbox read is invalid (wild address), i.e. the AP is not in the flat 32-bit protected-mode state the trampoline assumes — every other instruction in the block is valid in both 32- and 64-bit mode, and the failure is exactly at the one mode-sensitive instruction"
test: "Attach gdb to the QEMU gdbstub, let the AP triple-fault, and read CS/CR0/CR3/EFER at the fault; or swap the moffs read for a register-indirect read (`movl $0x8F00,%ebx; movl (%ebx),%eax`) which is valid in both modes"
expecting: "gdb shows CS.L=1 (or a CR0/CR3 state inconsistent with flat 32-bit) at the fault; the register-indirect variant reaches AP:6 / AP:C"
next_action: "Run scripts/kvm-ap-debug.sh --gdb on the KVM host, attach gdb, and capture CS/CR0/CR3/EFER at the AP fault"
bug_class: bohrbug
reasoning_checkpoint: null
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
<!-- IMMUTABLE -->

expected: "On an x86_64 Linux KVM host, `nix run .#boot-r9-smoke-x64` (and tier-1/rio) boots two CPUs: `smp: cpu 1 online` and `smp: tlb shootdown ok` appear, and the gate reaches its prompt stages."
actual: "The BSP sends INIT + SIPI; the AP starts and prints the trampoline markers AP:R, AP:G, AP:P, AP:4, AP:5 and then the machine triple-faults (QEMU reports `qemu exited (EOF) at stage: smp-online` with -no-reboot). No AP:C, no `smp: cpu 1 online`. TCG passes, so the path works under emulation and fails on real KVM."
errors: "QEMU exits (EOF) — a triple fault. Neither `-d int` nor `-d guest_errors` reports the guest exception under KVM (KVM handles it in-kernel)."
reproduction: "On an x86_64 Linux host with a writable /dev/kvm: `git pull`, then `nix run .#boot-r9-smoke-x64` (defaults to KVM, -cpu host, -smp 2). scripts/kvm-ap-debug.sh wraps this with log capture; add --gdb to expose the QEMU gdbstub."
started: "The x86_64 AP path was never exercised on KVM until Owner Stop 2: the composed gates boot dev_exec kernels, and start_aps was cfg'd out of dev_exec until 91187e8. The first real AP start exposed this."

## Marker legend (instrumentation currently in the tree)

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

root_cause:
fix:
verification:
oracle_type:
files_changed: []
