---
status: resolved
trigger: "owner (via coordinator, 2026-11-22): fix the aarch64 `noted` IRQ-mask trust bug now — noted must not let user-supplied PSR bits set DAIF (or other privileged bits); sanitize the restored SPSR to a safe EL0 value, checking against the donor's noted()/9front; also check every other path that restores a user-writable Ureg/SPSR (NRSTR, NSAVE chains, any sigreturn-like or debug paths) and x86_64's equivalent (RFLAGS IF/IOPL in noted via IRETQ) for the same class. Red/green probe: a handler sets the mask bits in the Ureg, returns NCONT, then spins — it must still be preemptible. Host tests; mutant; aarch64 HVF/TCG gates; if x86_64 changes, x64 container gates + boot-iso hash; local commits only; keep it focused"
created: 2026-11-22T08:40:00Z
updated: 2026-11-22T09:45:00Z
---

## Current Focus

hypothesis: CONFIRMED and fixed — aarch64 noted honoured every SPSR bit of the user-writable saved Ureg except M[3:0]
test: DONE — r9 local main 5f7c4e1 → c94bc9b (unpushed); Harvey fpprobe note round "mask" (this commit); no new tier-1 marker
expecting: DONE — owner confirmed fixed 2026-11-22; coordinator pushed r9 c94bc9b; the x86_64 entry-flags issue is DEFERRED (WINDOWS #71)
next_action: archived. The pin series needs r9 >= c94bc9b for the arm64 tier-1 (fpprobe's mask round) and one subset-build
bug_class: Bohrbug (deterministic trust-boundary bug)
reasoning_checkpoint:
  hypothesis: "force_psr_el0t clears only M[3:0], so user-set DAIF bits in the saved Ureg reach SPSR_EL1 and the eret masks IRQs at EL0"
  confirming_evidence:
    - "code: aarch64/src/trap.rs force_psr_el0t `self.psr &= !0xf`; aarch64/src/syscall.rs sys_noted calls restore_from_saved(&ureg) then force_psr_el0t (the only user-Ureg restore site on aarch64)"
    - "trap.S trap_return writes the frame psr to SPSR_EL1 unmodified before eret"
  falsification_test: "the mask round passing on the unfixed kernel (alarm delivered while the handler-set I bit is in the saved psr)"
  fix_rationale: "the donor rule is 'don't let user change system flags' (Harvey 9k k10 noted keeps only OF|DF|SF|ZF|AF|PF|CF from the user's flags, the rest from the kernel frame); on arm64 the user-writable PSR bits are NZCV"
  blind_spots: "the 9front arm64 trap.c text could not be fetched (no local donor tree, web search did not reach it) — the arm64 USERPSR set is derived from the documented rule and the k10/teg2 shape"
  candidate_causes:
    - "code: noted's PSR sanitizer too narrow (aarch64)"
    - "code: x86_64 kernel entry keeps user TF/DF/NT/AC (FMASK = IF only, no cld) — same trust class at the SYSCALL boundary; NT reaches the NOTED IRETQ"
  and_gate: "no — the aarch64 bug needs only the user-edited psr; the x86 entry issue is independent"

## Symptoms

expected: whatever a note handler writes into the saved Ureg, the process returns to EL0 with IRQs (and FIQ/SError/debug) unmasked, in AArch64 EL0t, with no IL/SS — only the condition flags are the user's
actual: (expected from code) D/A/I/F, M[4], IL, SS from the saved Ureg reach SPSR_EL1
errors: none (the symptom is a proc that cannot be preempted)
reproduction: arm64 fpprobe note round "mask" (to be written)
started: always (force_psr_el0t since the notes port)

## Eliminated

## Evidence

- timestamp: 2026-11-22T08:40:00Z
  checked: r9 5f7c4e1 — every site that copies user-writable state into a frame: grep restore_from_saved / force_psr_el0t / force_user_return (aarch64 + x86_64); devproc's files (note/status/args/ctl only — no regs/kregs/fpregs writes); exec (fresh kernel-built frames); rfork (copy of the kernel-captured frame)
  found: aarch64 — ONE site, sys_noted's shared commit (NCONT, NRSTR, NSAVE all pass through it), sanitized by force_psr_el0t = M[3:0] only. x86_64 — ONE site, sys_noted's commit, sanitized by force_user_return (cs/ss forced to the user selectors, IOPL cleared, IF set); no debug/regs-write or sigreturn-like path on either arch
  implication: the aarch64 fix is one sanitizer at one site; x86 noted already handles the image's IF/IOPL/CS/SS
- timestamp: 2026-11-22T08:45:00Z
  checked: donors — Harvey sys/src/9k/k10/syscall.c noted(), sys/src/9k/k10/trap.c setregisters(), sys/src/9/teg2/syscall.c noted(); 9front arm64 trap.c (web search; not reachable)
  found: k10 noted: "don't let user change system flags": nur->flags &= (Of|Df|Sf|Zf|Af|Pf|Cf); nur->flags |= cur->flags & ~(those) — plus a user cs/ss check; k10 setregisters keeps flags 0xff00 from the kernel frame. teg2's arm noted applies Psrsysbits (mode|FIQ|IRQ|abort) the INVERTED way (keeps the user's system bits) — a historical bug, not a model. 9front arm64 text not obtainable offline
  implication: arm64 rule = the user may set NZCV only; every other SPSR bit comes from the kernel-captured NOTED frame; DAIF, IL, SS, M[4:0] forced clear for an EL0t return
- timestamp: 2026-11-22T08:50:00Z
  checked: x86_64 equivalents — dat.rs force_user_return, cpu.rs fmask(), trap.rs syscallentry
  found: noted's image is sanitized for IF/IOPL/CS/SS; VM is ignored by a 64-bit IRETQ to an outer level and VIF/VIP are inert without CR4.VME/PVI. BUT the kernel ENTRY keeps user flags: IA32_FMASK = IF only (documented D-02a: "9k lets TF/DF pass through"), and syscallentry has no cld — a user can enter the kernel with TF (single-step #DB at the first kernel instruction, taken on the user stack), DF (Rust/SysV code assumes DF=0; compiler_builtins' forward rep movs would copy backwards), AC, or NT; NT set inside a note handler reaches the NOTED path's IRETQ (0ffcec6) with the kernel's live RFLAGS.NT=1 → #GP(0) in IA-32e mode (kernel-origin #GP → dump and park)
  implication: same trust class at a different boundary (entry, not the restored image); fixing it reverses D-02a's FMASK choice or adds an entry-side flags reset — a DECISION for the owner, not changed in this session; recorded for the report
- timestamp: 2026-11-22T09:00:00Z
  checked: scratch r9 c94bc9b — user_return_psr(saved, live) = ((saved & NZCV) | (live & !NZCV)) & !(M[4:0] | DAIF | IL | SS), applied inside TrapFrame::restore_from_saved (the one place user bytes enter a frame; NCONT/NRSTR/NSAVE share it); force_psr_el0t removed. Host tests trap::tests::{noted_psr_takes_only_the_condition_flags_from_the_saved_ureg, noted_psr_keeps_the_live_frames_other_bits_but_never_a_mask, restore_from_saved_rebuilds_the_psr}
  found: aarch64 host 57/57; mutant (sanitizer = the old `saved & !0xf`) → all 3 new tests FAIL (54/3)
  implication: the table is pinned on the host
- timestamp: 2026-11-22T09:05:00Z
  checked: arm64 fpprobe note round "mask" (handler ORs 0x3c0 into ur->psr, NCONT; then alarm(20) + a 2e8-iteration syscall-free spin reading nseen through vget) — base 5f7c4e1 (store kernel-virt), fix c94bc9b, mutant; HVF + TCG ($SP/fpf/runM-summary.txt)
  found: base HVF + TCG `BAD note-mask no tick after the handler set DAIF in the saved psr (alarm not delivered, 200000000 iterations)`; fix HVF + TCG `fpprobe: ok`; mutant HVF + TCG the same BAD as base
  implication: red/green on both venues, deterministic (no timing window: with I set nothing at EL0 takes the tick)
- timestamp: 2026-11-22T09:20:00Z
  checked: darwin battery c94bc9b vs 5f7c4e1; x86-64 kernels; aarch64 gates with the rebuilt userland (harvey-sb + this probe, subset-build PASS 74 entries), $SP/fpf/series-psr; landing; r9 aarch64 boot tests on the real checkout
  found: check 63 = 63, clippy 82 = 82, aarch64 clippy diagnostic set unchanged (no new, none gone), tests 294 passed (291 + 3), builds OK; x86-64 default + q35churnshell byte-identical to 5f7c4e1 (boot-iso FOD unchanged). Gates: tier-1 TCG PASS 44 T1-OK `fpprobe: ok`, tier-1 HVF PASS 44 `fpprobe: ok`, smoke HVF PASS, net-smoke HVF PASS (r9-kernel-virt-c94bc9b). Real r9 fast-forwarded 5f7c4e1 → c94bc9b (no push, no attribution). Boot tests 10/10 = base (virtnotes PASS; virtshell no-initrd = base)
  implication: verified on the landed content

## Resolution

root_cause: r9 aarch64 sys_noted copied the user-writable saved Ureg over the live frame and sanitized only SPSR M[3:0] (TrapFrame::force_psr_el0t), so D/A/I/F, M[4], IL and SS from user memory reached SPSR_EL1: a handler that set I/F in ur->psr and returned NCONT ran its process at EL0 with interrupts masked (no tick preemption on one CPU; every later syscall return re-erets the masked SPSR the SVC captured)
fix: r9 c94bc9b — TrapFrame::restore_from_saved (the single user-bytes-into-frame site; NCONT/NRSTR/NSAVE share it) rebuilds the psr with user_return_psr: NZCV from the saved Ureg, every other bit from the SPSR the NOTED syscall entered with, M[4:0]/DAIF/IL/SS forced clear (the donor rule "don't let user change system flags", 9k noted()); force_psr_el0t removed; 3 host tests. Harvey: arm64 fpprobe note round "mask"
verification:
  target_test:
    result: pass
    detail: "note-mask: base 5f7c4e1 FAIL on HVF and TCG (alarm never delivered over 2e8 iterations); c94bc9b `fpprobe: ok` on both (standalone and inside tier-1)"
  mutation_check:
    result: pass
    mutant_killed: "sanitizer reverted to `saved & !0xf`: host 3/3 new tests FAIL; boot note-mask FAIL on HVF and TCG"
  no_op_deletion:
    result: pass
    deletion_justified_by_rca: "force_psr_el0t (M[3:0]-only) is subsumed by user_return_psr inside restore_from_saved; its one call site goes with it"
  adjacent_tests:
    result: pass
    suites_run: ["darwin: check/clippy counts = base, aarch64 clippy set unchanged, xtask test + aarch64 host 294/0", "aarch64 gates with rebuilt userland: tier-1 TCG + HVF (44 T1-OK), smoke + net-smoke HVF — PASS", "r9 aarch64 boot tests 10/10 = base"]
  revert_and_reconfirm:
    result: pass
    bug_returned_on_revert: true — "5f7c4e1 and the mutant fail note-mask on both venues"
    fixed_on_reapply: true — "c94bc9b passes on both"
  x86_64_bytes: "UNCHANGED — kernel-x64 default and q35churnshell byte-identical to 5f7c4e1; boot-iso FOD hash stands"
  guardrail_verdict: accepted (pending human verification)
files_changed: [r9 aarch64/src/trap.rs, r9 aarch64/src/syscall.rs, Harvey build/aarch64/fpprobe.c]
other_restore_paths: "none besides noted on either arch — devproc serves note/status/args/ctl only (no regs/kregs/fpregs writes), exec builds fresh frames, rfork copies the kernel-captured frame; NRSTR and NSAVE share noted's restore"
x86_64_audit: "noted's image path is already sanitized (force_user_return: cs/ss forced to the user selectors, IOPL cleared, IF set); VM is ignored by a 64-bit IRETQ to an outer level and VIF/VIP are inert without CR4.VME/PVI, so no x86 change for the noted image. Stale doc reference: x86_64/src/dat.rs force_user_return's doc names aarch64's force_psr_el0t (left untouched to keep x86 bytes identical)"
follow_ups:
  - "DECISION (x86_64, same trust class, at the SYSCALL boundary rather than the restored image): IA32_FMASK = IF only (cpu.rs fmask(), documented D-02a: '9k lets TF/DF pass through') and syscallentry has no cld, so a user enters the kernel with its own TF (single-step #DB at the first kernel instruction, delivered on the still-user stack), DF (the Rust kernel assumes DF=0; compiler_builtins' forward rep movs would copy backwards), AC, and NT; NT set inside a note handler reaches the NOTED return's IRETQ (0ffcec6) with the kernel's live RFLAGS.NT=1 → #GP(0) in IA-32e mode → kernel-origin trap → halt. Not reproduced yet (audit + SDM/QEMU semantics). Fix options: (a) FMASK = IF|TF|DF|NT|AC (|IOPL) — reverses D-02a; user flags still come back via R11/the frame; (b) keep D-02a and reset RFLAGS in syscallentry before calling Rust (cld + clear NT/AC/TF), which cannot prevent the TF #DB at the entry's first instruction — only (a) does. Either changes x86 bytes: x64 container gates + boot-iso hash re-derivation"
