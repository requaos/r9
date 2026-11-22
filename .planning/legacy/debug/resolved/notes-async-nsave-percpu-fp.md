---
status: resolved
trigger: "follow-ups of fpu-x64-notes-kernel-simd (owner, via coordinator, 2026-11-22): (a) x86_64 async note delivery — notes must interrupt ring-3 code (alarm/DEL), with noted returning through iretq for interrupted contexts (SYSRET clobbers rcx/r11); check aarch64 for the same gap and fix it there too if needed; (b) NSAVE on both arches per the 9front/Plan 9 semantics; (c) FP ownership per CPU — move the single static into the per-CPU structure, correct for SMP; r9 is single-CPU today, so a structural change verified by tests with no behaviour change"
created: 2026-11-22T02:20:00Z
updated: 2026-11-22T05:40:00Z
---

## Current Focus

hypothesis: CONFIRMED and fixed — (a) x86_64 delivered notes only at the syscall boundary and NOTED returned through SYSRETQ (rcx/r11 lost for an interrupted context); aarch64 had no gap. (b) NSAVE unimplemented on both arches (and no nested-note chain). (c) FP bookkeeping was a module static. Found on the way: x86_64's interrupt entry called trap() on a stack misaligned by 8 (latent ABI violation that the new interrupt-time note boundary turned into a kernel #GP)
test: DONE — r9 local main 3f69729 → e9e5365 (37b5f4e per-CPU, 0f7dabb x86 interrupt-stack alignment, 0ffcec6 notes, e9e5365 boot-iso hash; unpushed); Harvey 91f286f02 (fpprobe note rounds); no new tier-1 marker (T1-OK fpprobe already gates both arches and now covers the new rounds)
expecting: DONE — owner confirmed fixed 2026-11-22; coordinator pushed r9 e9e5365
next_action: archived; follow-up aarch64 demand-fault-return note delivery opened as its own session. The HVF pin series / Harvey push move Harvey's r9 pin to >= e9e5365: the aarch64 and x64 tier-1 `T1-OK fpprobe` now need the new note rounds (NSAVE on both arches; async + regs on x86)
bug_class: Bohrbug (deterministic; byte-identical HVF/TCG output)
scratch: SP=/private/tmp/claude-501/-Users-neilskinner-code-harvey/55ded085-3397-4d2c-931d-1d3a7a81443a/scratchpad — $SP/fp3/{r9 (final scratch + mut-x-* branches), r9-mut, harvey (fp3-wiring), pa/px (host-built probes), series*, run-*.log, mut-*}, $SP/x64fp2/{run4-summary.partial.txt, run5-summary.txt, run6-summary.txt, step1-summary.txt, r4-*/r5-*/r6-* captures}

## Symptoms

expected: (a) a note posted while a process computes in user mode interrupts it at the next interrupt return, and noted(NCONT) resumes it with every register intact; (b) noted(NSAVE) restarts a handler at the saved pc on the saved stack with notes re-enabled, and noted(NRSTR) later returns to the program (notify(2)); (c) per-CPU FP bookkeeping lives in a per-CPU structure
actual: (a) x86: notes wait for a syscall (fpu-x64-notes-kernel-simd Evidence 01:25, `note-alarm handler ran 0 times`); (b) `noted: NSAVE unimplemented` on both arches; (c) single statics
errors: `noted: NSAVE unimplemented`
reproduction: Harvey fpprobe note leg (to be extended)
started: always

## Eliminated

## Evidence

- timestamp: 2026-11-22T02:20:00Z
  checked: knowledge-base.md (MemPalace absent → keyword fallback), the 9front donor (sys/man/2/notify; sys/src/9/{pc64,arm64}/trap.c noted(); port/sysproc.c sysnoted/notify)
  found: KB: arm64-user-fpsimd-unsaved and (pending) fpu-x64-notes-kernel-simd — the ownership/note FP design this builds on. Donor NSAVE: the saved Ureg is copied over the live one (like NCONT), then sp = nureg - 4*BY2WD - ERRMAX (= the handler's original sp), [sp+8] = nureg, [sp+0] = 0, arg register (pc64 bp, arm64 r0) = nureg, pc stays the saved Ureg's (the first handler retargets it); up->notified = 0 (another note may come) while up->noteureg stays nureg; notify() writes the previous noteureg into the word just below each new Ureg and NCONT/NRSTR pop it (`up->noteureg = *(Ureg**)(nureg-BY2WD)`); sysnoted: NRSTR is legal while not notified, anything else is Ebadarg; fpunoted runs only `if(up->notified)` (NSAVE restores the interrupted FP state, a later NRSTR touches none). pc64 has noteret: after noted() the syscall returns through the full-context path
  implication: r9 needs `notified` separated from `note_ureg`, the noteureg chain through the word under the Ureg, and on x86 a full-context (iretq) return after NOTED

- timestamp: 2026-11-22T02:35:00Z
  checked: x86_64 SYSCALL entry/return (trap.rs syscallentry/syscallreturn), trap_return, trap()'s IRQ arm; x86 Mach (dat.rs) and aarch64 per-CPU state
  found: the SYSCALL entry builds a full Ureg at rsp0 (the same placement and layout as an IDT frame), so trap_return (full GPR restore + iretq) can leave from it; syscallreturn restores the GPRs then loads rcx := frame pc and r11 := frame flags for SYSRETQ. trap() for vectors >= 32 returns straight after dispatch_irq (no note boundary). x86 has a per-CPU Mach at the fixed per-CPU VA dat::KMACH (FromZeros, Mach::init); aarch64 has NO per-CPU structure
  implication: (a) = a deliver_notes call on trap()'s ring-3 IRQ return + NOTED leaving through trap_return; (c) = Mach.fpu on x86, a MPIDR-indexed table on aarch64

- timestamp: 2026-11-22T02:50:00Z
  checked: probes — fpprobe note leg extended on both arches (scratch Harvey fp3-wiring 92bdad7e1): rounds sync (write to own /proc/N/note), sleep (alarm interrupts sleep), spin (alarm interrupts integer code), regs (alarm interrupts gprspin, which holds sentinels in AX BX CX DX SI DI R8-R12 on x86 / R1-R16 R19-R23 on arm64 — asm verified by disassembly), nsave (handler retargets the saved pc at a trampoline, noted(NSAVE); trampoline checks it starts with the interrupted FP state, takes a nested note, restores the pc, noted(NRSTR)). arm64 run on r9 3f69729 (nix kernel), HVF and TCG ($SP/fp3/run-red-*.log)
  found: arm64 RED only on nsave: `fpprobe: BAD note-nsave noted(NSAVE) refused`, `FAIL note`; sync/sleep/spin/regs pass on both accelerators
  implication: aarch64 has NO async-delivery gap (notes interrupt EL0 code, NOTED's eret restores every register) — only (b) and (c) apply there

- timestamp: 2026-11-22T03:20:00Z
  checked: implementation in scratch r9 ($SP/fp3/r9): 37b5f4e (CpuFp per CPU: Mach.fpu on x86, MPIDR-indexed table on aarch64, *_on variants, thread-local CPU in host tests) + 000ad89 (notified split from note_ureg; chain word below each saved Ureg; NSAVE + NRSTR-outside-a-handler per the donor; plan_noted pure core; x86 trap() note boundary on the ring-3 IRQ return; entry() leaves through trap_return after a successful NOTED)
  found: builds aarch64 virtshell (39 warnings = base) and x86-64 default / q35churnshell (23 / 61 = base); aarch64 host tests 52/52 (+4 noted_plan_tests, +2 per-CPU); rustfmt: fpu.rs clean, syscall.rs below base hunk counts (the reformatted sys_noted region carried one old hunk each); arm64 probe on the new kernel GREEN on HVF and TCG (`fpprobe: ok`, also with -w 4 -p 12). aarch64 gates on 000ad89 (nix kernel via R9_OVERRIDE; tier-1 TCG after a userland rebuild — guest subset-build of the new probe): smoke/net-smoke/tier-1 TCG + HVF all PASS, tier-1 44 T1-OK with `fpprobe: ok`
  implication: aarch64 (b) + (c) green end to end; x86 verification running in the container

- timestamp: 2026-11-22T04:05:00Z
  checked: aarch64 boot-level mutants on HVF ($SP/fp3/mut-a64.sh): chain pop removed; NSAVE arg (x0) not set; NOTED's fpunoted removed
  found: none → ok; no-pop → the probe HANGS (TIMEOUT: the trampoline's NRSTR keeps restoring the nested note's saved context, re-entering the trampoline); no-arg → `note child: fault` (the trampoline writes through a garbage Ureg pointer); no-fpunoted → note-sync/sleep/spin/nsave BAD (nbad 65-66, the handler's pattern)
  implication: 3/3 killed

- timestamp: 2026-11-22T04:10:00Z
  checked: container run 4 ($SP/x64fp2/run4-summary.partial.txt): x86_64 host tests at 000ad89; x64 tier-1 (Harvey HEAD sweep + the new probe, userland rebuilt) red at 3f69729 and green at 000ad89
  found: host tests 134/134 (+ noted_plan_follows_the_donor_rules, noted_nsave_then_nrstr_runs_the_trampoline_and_returns, the 2 per-CPU tests). RED 3f69729: `note-spin handler ran 0 times`, `note-regs handler ran 0 times`, `note-nsave noted(NSAVE) refused`, T1-FAIL fpprobe. GREEN attempt 000ad89: the sweep never finished — kernel `trap GP` (vector 13, cs 0x8) at pc 0xffffffff80220558 in the note child; symbolized against the gate's own kernel-x64.elf (docker cp from the store): `fxsave64 (%r14)` in deliver_notes (snapshot's stack FpState), r14 = 0x…3c78 — NOT 16-byte aligned
  implication: NEW ROOT CAUSE (pre-existing, latent): x86_64 intrcommon calls trap() straight from the Ureg base, which sits at 8 mod 16 (23 qwords below the CPU's 16-aligned frame top) — trap() and everything under it always ran with the stack misaligned by 8 (the SYSCALL entry pads one qword for exactly this). Harmless until the note boundary moved onto the interrupt path (FXSAVE faults on a misaligned operand); a kernel-SIMD section in an IRQ handler would hit it too

- timestamp: 2026-11-22T04:20:00Z
  checked: fix — intrcommon aligns %rsp before `callq trap` and keeps the frame base in callee-saved %rbx (scratch r9 0f7dabb, ordered BEFORE the notes commit so no intermediate state faults; disassembly: `movq %rsp,%rbx; andq $-0x10,%rsp; callq trap; movq %rbx,%rsp`); x86 mutant branches rebuilt on the reordered head plus a no-align mutant; darwin battery on the head; clippy per arch vs base 3f69729 (real checkout)
  found: check 63 / xtask test 237/0 / aarch64 host 52/52 / builds aarch64 default, virtshell, virtchurn and x86-64 default, q35churnshell rc 0. clippy: the only new diagnostics were 4 `doc list item without indentation` per syscall.rs (the paragraph after sys_noted's doc list) — fixed with a blank doc line; aarch64 syscall.rs hits back to base (3), x86 to base (0). The doc fix was amended into the notes commit (scratch 0ffcec6); it shifts debug line numbers, so kernel bytes differ from eeb1e07 (a64 56681a6d… → 34e6fb9a…, x64 12a9c352… → 348cf86f…, code identical) — the boot-iso hash and one x64 + aarch64 tier-1 are taken on the final head
  implication: final scratch r9 = 37b5f4e (per-CPU) → 0f7dabb (x86 IRQ stack alignment) → 0ffcec6 (NSAVE/nested notes/x86 async delivery + IRETQ NOTED)

- timestamp: 2026-11-22T04:45:00Z
  checked: container run 5 ($SP/x64fp2/run5-summary.txt) at eeb1e07 (code = the final 0ffcec6 but for the doc line): x86_64 host tests; x64 tier-1 (Harvey HEAD sweep + the new probe, userland rebuilt); five x86 mutants (one change each); auth; net-smoke; boot-iso
  found: host tests 134/134. GREEN: tier-1 ALL STAGES PASS, 39 T1-OK, `fpprobe: ok` (async spin + regs + nsave on x86). Mutants: no IRQ-return delivery → `note-spin` / `note-regs handler ran 0 times`; NOTED through SYSRET → `note-regs cx want=1111111111111103 got=20189d` and `r11 want=111111111111110b got=10206` (cx = the return pc, r11 = rflags — exactly the clobber the IRETQ return prevents); no chain pop → the note child dies `sys: trap`; no fpunoted → note-sync/sleep/spin/nsave/tramp-entry FP BAD; no stack alignment → the same kernel `trap GP` at fxsave64 in deliver_notes (pc 0xffffffff80220558). auth ALL STAGES PASS (235 s), net-smoke ALL STAGES PASS; boot-iso for eeb1e07 got sha256-yx+dKsrixQVuGhu9b4ScUafyXksQ9XU48KjMBg0np0E= (superseded by the final head's)
  implication: x86 (a) + (b) + the alignment fix green; 5/5 x86 mutants killed

- timestamp: 2026-11-22T04:50:00Z
  checked: aarch64 on the exact final head 0ffcec6 (R9_SKIP_BUILD=1 over the rebuilt userland): tier-1 TCG + HVF, smoke + net-smoke HVF ($SP/fp3/series-final/)
  found: all PASS; tier-1 44 T1-OK with `fpprobe: ok` on both accelerators
  implication: the doc-line amend changed nothing observable on aarch64

- timestamp: 2026-11-22T05:15:00Z
  checked: container run 6 ($SP/x64fp2/run6-summary.txt) on the final head 0ffcec6: kernel-x64, boot-iso FOD, then the x64 tier-1 on a clone with the re-derived hash wired (= the final tree e9e5365's content); landing; r9 aarch64 boot tests on the landed tree (real checkout); container cleanup
  found: kernel-x64.elf 01adbea33e62d7d1… (28048696 B; 3f69729: 4b0cf307…); boot-iso specified sha256-x3KlbAkt… → got sha256-JQ/47nON4oZPk1SxxM5zkHARxCrH23ShgR9sJJCltCY=, wired build green (boot.iso b86cfd12…), --rebuild deterministic; x64 tier-1 on the wired tree ALL STAGES PASS, 39 T1-OK, `fpprobe: ok`. Real r9 fast-forwarded 3f69729 → e9e5365 (no push, no attribution). Boot tests: virt, virtexec, virtexec3, virtmulti, virtnotes, virtpanic, virtchurn, virtip, virtnet, virtlowecam PASS = base; virtshell rc=1 = base (no initrd by design). `docker rm -f r9x64fp2`; images / volumes / containers lists identical to the pre-session snapshot
  implication: verified on the exact landed content

## Resolution

root_cause: (a) x86_64: trap() ran no note boundary on the interrupt return to ring 3 (a pending note waited for the proc's next syscall), and NOTED returned through syscallreturn's SYSRETQ, which loads rcx := pc and r11 := rflags — an asynchronously interrupted context would lose both; aarch64 already delivered on the IRQ return and NOTED returns through a full-frame eret (no gap). (b) both arches: noted(NSAVE) answered "unimplemented"; `note_ureg != 0` doubled as the notified flag, so notes could not be re-enabled inside an NSAVE trampoline and no nested-note chain existed. (c) the FP owner / kernel-SIMD flag were module statics. (d, found on the way) x86_64 intrcommon called trap() straight from the Ureg base at 8 mod 16, so trap() and everything below it always ran with the stack misaligned by 8 — latent until the note boundary on the interrupt path FXSAVEd into a stack FpState (kernel #GP)
fix: r9 local main 3f69729 → e9e5365 (unpushed). 37b5f4e: CpuFp (owner + section) per CPU — x86 Mach.fpu (FromZeros, written in Mach::init), aarch64 an MPIDR_EL1.Aff0-indexed table (MAXMACH 8), thread-local in host tests; *_on(cpu) variants; KernelSimd records its CPU. 0f7dabb: intrcommon aligns %rsp before `callq trap` and keeps the frame base in %rbx. 0ffcec6: Proc.notified split from note_ureg; delivery writes the previous saved Ureg into the word below the new one (the donor chain), NCONT/NRSTR pop it; NSAVE per notify(2)/9front (saved Ureg over the live frame, handler restarts at its pc on sp = nureg - 4*8 - ERRMAX with arg0 = nureg in R0/bp and [sp+8], [sp] = 0, notes re-enabled, note_ureg kept); NRSTR legal outside a handler; fpunoted only while notified; plan_noted = the pure decision core on both arches; x86 trap() delivers notes on the ring-3 IRQ return and entry() leaves through trap_return (IRETQ) after a successful NOTED (the donor's noteret). e9e5365: boot-iso hash. Harvey 91f286f02: fpprobe note rounds sync / sleep / spin / regs (gprspin asm) / nsave on both arches
verification:
  target_test:
    result: pass
    detail: "red on 3f69729 — arm64: nsave refused only (sync/sleep/spin/regs pass: no async gap); x86: spin + regs 'handler ran 0 times', nsave refused. Green on the final head — arm64 `fpprobe: ok` HVF + TCG (tier-1 44 T1-OK), x86 `fpprobe: ok` (tier-1 39 T1-OK)"
  mutation_check:
    result: pass
    mutant_killed: "x86 5/5 (no IRQ-return delivery; NOTED via SYSRET → cx/r11 = pc/flags; no chain pop → child trap; no fpunoted → FP BAD; no stack alignment → kernel #GP at fxsave64), aarch64 3/3 (no chain pop → trampoline loop / timeout; NSAVE arg unset → child fault; no fpunoted → FP BAD). Per-CPU structure pinned by host tests (each_cpu_keeps_its_own_owner_and_section kills a guard that restores on this_cpu() instead of its own CPU)"
  no_op_deletion:
    result: pass
    deletion_justified_by_rca: "the removed lines are the 'NSAVE unimplemented' arm and the `note_ureg != 0` notified test (both replaced), plus the module statics moved into CpuFp"
  adjacent_tests:
    result: pass
    suites_run: ["darwin: xtask check 63, clippy default/aarch64/x86-64 = the 3f69729 sets (after a doc-list fix), xtask test 237/0, aarch64 host 52/52, builds aarch64 default/virtshell/virtchurn + x86-64 default/q35churnshell", "container: x86_64 host 134/134", "aarch64: smoke / net-smoke / tier-1 TCG (userland rebuilt) + HVF on 000ad89, and tier-1 TCG + HVF, smoke + net-smoke HVF on 0ffcec6 — all PASS", "x64 (container, TCG): tier-1 39 T1-OK on eeb1e07 and on the wired final tree, auth + net-smoke ALL STAGES PASS", "r9 aarch64 boot tests 10/10 = base"]
  revert_and_reconfirm:
    result: pass
    bug_returned_on_revert: true — "3f69729 reds the new rounds on both arches; each single-change mutant reds its own round"
    fixed_on_reapply: true — "final head green on every round, both arches"
  x86_64_bytes: "CHANGED (kernel-x64.elf 4b0cf307… → 01adbea3…) — boot-iso FOD re-derived in a fresh amd64 container (wired build green, --rebuild deterministic, x64 tier-1 on it) and committed (e9e5365)"
  guardrail_verdict: accepted (pending human verification)
files_changed: [r9 aarch64/src/{fpu.rs, syscall.rs, trap.rs, proc.rs, exec.rs}, r9 x86_64/src/{fpu.rs, dat.rs, syscall.rs, trap.rs, proc.rs, exec.rs}, r9 flake.nix, Harvey build/{aarch64,x86_64}/{fpprobe.c, fpprobeasm.s}]
follow_ups:
  - "aarch64 delivers notes on the IRQ and syscall returns but not on a demand-fault (EC 0x20/0x24) return to EL0 (9front notifies on every user trap return); with a 1 ms tick it is not observable — record, not fixed"
  - "SMP: the per-CPU structure is in place; bringing up a second CPU still needs per-CPU CURFRAME/CURKSTACK/CURRENT_PROC on aarch64 and a per-CPU Mach map on x86"

