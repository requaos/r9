---
status: resolved
trigger: "r9 aarch64 never saves or restores user FP/SIMD registers (V0-V31, FPCR, FPSR) across traps, syscalls or context switches, while the kernel itself is built with NEON; any FP/SIMD-using user program can see corrupted vector registers (side finding of .planning/debug/fork-child-pc-align-fault.md)"
created: 2026-11-21T20:05:00Z
updated: 2026-11-21T21:45:00Z
---

## Current Focus

hypothesis: CONFIRMED — r9 aarch64 keeps no user FP/SIMD context: trap frame and swtch Context are GPR-only, CPACR leaves FP untrapped, and the kernel itself is NEON-compiled, so (a) kernel codegen clobbers q0/q1 inside syscalls/fork/IRQs, (b) every switch leaves the previous proc's q0-q31/FPCR/FPSR live for the next, (c) fork children and exec'd images start with arbitrary state
test: DONE — fix r9 2edfdf8 (pushed by the coordinator) verified: probe red→green HVF+TCG (host-built and guest-built userland), 7/7 boot mutants killed, battery green, TCG+HVF smoke/net-smoke/tier1 green (T1-OK float-d01), x86_64 bytes identical
expecting: RESOLVED — owner: "keep this design, confirmed fixed"; `T1-OK fpprobe` wired (Harvey 13f00747b)
next_action: none here. Follow-ups (x86_64 per-proc FPU/SSE state, note-handler FP save on both arches, the explicit kernel-SIMD API) continue in .planning/debug/fpu-x64-notes-kernel-simd.md; the HVF pin series moves Harvey's r9 pin (105d48e) to >= 2edfdf8 — required now that tier-1 expects T1-OK fpprobe
bug_class: Bohrbug (deterministic: byte-identical BAD lines on HVF and TCG)

reasoning_checkpoint:
  hypothesis: "user FP/SIMD state is corrupted because the r9 aarch64 kernel (a) is compiled with NEON and writes vector registers in ordinary syscall/fork/IRQ paths, and (b) never saves/restores q0-q31/FPCR/FPSR when the proc on the CPU changes, nor sets them at fork/exec"
  confirming_evidence:
    - "fpprobe on stock 46a5c87: one worker, plain alarm(0) syscalls → q0 zeroed (nbad=2), HVF and TCG identical; the stock kernel disassembly has 90 FP/SIMD instructions (movi v0.2d #0 + stp q zeroing, cnt)"
    - "two pure spinners (preempt leg): each worker reads the OTHER worker's pattern in all 66 words (seed 9 got seed 10's b2ad405872ac0921) — the switch leaves the previous proc's state live"
    - "fork child/parent q0 = a kernel PA (0x4282a000) — rfork_copy's NEON code wrote it; exec'd image starts with the previous image's q1..q31/FPCR/FPSR (nbad=63)"
    - "with the kernel FP-free + save/restore at install/leave + fork copy + exec zero, all four legs pass on HVF and TCG (406/576 switches)"
  falsification_test: "any fpprobe BAD on the fix kernel; or a fix-kernel disassembly showing FP/SIMD outside fpu.rs's save/restore; or a hook-removal mutant that stays green (the hook would then be untested or unnecessary)"
  fix_rationale: "removes both causes: the softfloat target makes the kernel structurally unable to touch vector registers (compiler-enforced, no per-trap cost), and the per-proc fpsave with restore-at-install / save-at-leave keeps each proc's state across every switch path (reschedule, sched, pick_next, activate_first); fork copies the parent's live state, exec zeroes it — the donor/x86_64 shape"
  blind_spots: "note handlers share the interrupted code's FP state (9front saves/restores FP around notify/noted — not done here, follow-up); SMP absent (single CPU assumed throughout r9); raspi platforms not booted (same target JSON — their uartpl011 f32 math becomes soft-float calls); the stdarch future-incompat note may harden on a toolchain bump; x86_64 has the same switch-leak class (no FXSAVE on switch) — out of scope"
  candidate_causes:
    - "code: no save/restore of user FP/SIMD at switches, no fork copy / exec reset (CONFIRMED)"
    - "config: kernel target features +neon,+fp-armv8 let the compiler use vector registers in kernel paths (CONFIRMED — the q0 clobber with one worker and no switches)"
    - "environment: HVF vs TCG difference (ELIMINATED — byte-identical BAD lines)"
  and_gate: "yes — two independent contributing causes: even with switch save/restore, the NEON kernel would still clobber q0 inside syscalls; even with an FP-free kernel, switches/fork/exec would still leak. root_cause names both"
scratch: SP=/private/tmp/claude-501/-Users-neilskinner-code-harvey/55ded085-3397-4d2c-931d-1d3a7a81443a/scratchpad — $SP/fp/{probe (fpprobe.c, fpprobeasm.s, host-built fpprobe), harvey (scratch clone @bfc0085f6 + main userland rsynced + host-built fpprobe), harvey-sb (scratch clone, full subset-build in guest dir harvey-build-fp), r9 (fix 2edfdf8), r9-base (46a5c87), r9-mut (mutants), fpboot.sh/.expect + env.sh (boot driver), runs/, series/ (TCG gates), series-sb/ (HVF gates), bat-*.log (battery)}
SBFL: skipped (no per-test coverage of the kernel; the probe is deterministic)
known_pattern_candidate: none in knowledge-base.md (MemPalace absent → keyword fallback; no FP/SIMD entry). Origin: fork-child-pc-align-fault Evidence 2026-11-21T17:55 SIDE FINDING

## Symptoms

expected: a user program's V0-V31, FPCR and FPSR survive every trap, syscall, IRQ preemption and context switch unchanged (except where the ABI defines a result)
actual: (reported by reading) no save/restore anywhere in the aarch64 trap or switch path; the kernel itself is built with NEON
errors: none observed yet (latent); rc uses no FP/SIMD, so the existing gates never exercise it
reproduction: `/bin/fpprobe` at the r9 % prompt (Harvey build/aarch64/fpprobe.c + fpprobeasm.s, baked by mkimage.sh) — stock r9 46a5c87 prints `fpprobe: FAIL fork exec switch preempt` on HVF and TCG
started: always (aarch64 port never had FP/SIMD context)

## Eliminated

## Evidence

- timestamp: 2026-11-21T20:05:00Z
  checked: knowledge-base.md (5 entries; MemPalace CLI absent → keyword fallback)
  found: no FP/SIMD/vector/FPCR entry; fork-child-pc-align-fault (open, awaiting verify) eliminated FP/SIMD for rc and recorded this defect as a side finding (kernel target +neon,+fp-armv8, CPACR_EL1.FPEN=TrapNothing, TrapFrame/swtch Context carry no v0-v31/FPCR/FPSR)
  implication: new defect class for the KB; the side finding is the starting evidence

- timestamp: 2026-11-21T20:07:00Z
  checked: r9 aarch64 trap.S (handle_interrupt / trap_return), swtch.S + swtch.rs Context, l.S + pre_mmu/vminit.rs CPACR, lib/aarch64-unknown-none-elf.json, proc.rs scheduler (reschedule, sched, pick_next, sched_from_moribund, activate_first), trap.rs idle_wait
  found: frame = 35 GPR/system words only (Ureg ABI, 288 B); swtch saves x19-x30+sp only (not even the AAPCS64 callee-saved d8-d15 the NEON kernel's own code may rely on across sched()); CPACR_EL1.FPEN = 0b11 (no trap at EL0 or EL1); target features "+strict-align,+neon,+fp-armv8". Current proc changes at exactly three places: reschedule() (tick preemption at EL0 and sleep(0) yield), pick_next() (sched loop resume, Switch or Return), activate_first() (first exec); a proc leaves the CPU through sched() (blocked mid-syscall) or sched_from_moribund() (exit)
  implication: confirmed by reading — nothing anywhere preserves user V0-V31/FPCR/FPSR

- timestamp: 2026-11-21T20:08:00Z
  checked: disassembly of the stock aarch64 debug kernel (target/aarch64-unknown-none-elf/debug/aarch64, 189,817 lines)
  found: 90 FP/SIMD instructions — 46 stp q, 19 movi (v0.2d #0 for struct zeroing), 7 cnt (popcount), plus str/ldp q
  implication: the kernel really writes V registers (v0 at least) in ordinary paths; a user program's q0 can be zeroed by any syscall that zero-inits a struct

- timestamp: 2026-11-21T20:09:00Z
  checked: x86_64 comparison (x86_64/src/exec.rs fp_reset_for_exec, l.S CR4 14-08, lib/x86_64-unknown-none-elf.json)
  found: x86_64 kernel is SSE-free ("-mmx,-sse,+soft-float", rustc-abi x86-softfloat); user FP state survives traps in place because the kernel never touches it; exec runs fninit + ldmxcsr 0x1f80; the recorded era limit: NO per-proc FXSAVE/FXRSTOR on context switch (FP registers survive switches in place — wrong as soon as two FP users interleave)
  implication: the x86_64 precedent is "FP-free kernel + exec reset"; it shares half of this defect (switch leakage) but is out of scope here (x86_64 bytes must stay unchanged); report it as a follow-up

- timestamp: 2026-11-21T20:12:00Z
  checked: Harvey arm64 libc (main9.s, callmain.c, memset.s, memmove.s, getfcr.s), a probe text scan
  found: _main/_callmain/memset/memmove are GPR-only (MOVP = ldp/stp x); FP/SIMD in libc lives only in the float routines (xdtoa, modf, strtod, frexp, pow10, umuldiv, ...). fpprobe (host 7c/7a/7l) text scan: FP/SIMD only in fpload/fpstore and those libc float routines; the two other decodes (work, _fmtdispatch) are literal-pool words (0x5851f42d4c957f2d low half)
  implication: nothing between fpload and fpstore in the probe touches FP/SIMD — any difference is the kernel's

- timestamp: 2026-11-21T20:35:00Z
  checked: RED run — fpprobe (legs fork, exec, switch, preempt) on the stock r9 46a5c87 virtshell kernel, HVF (-cpu host) and TCG (cortex-a72), `/bin/fpprobe -w 1` then `/bin/fpprobe` ($SP/fp/runs/stock-{hvf,tcg}-3.log; earlier runs -1/-2 identical on the shared legs)
  found: `fpprobe: FAIL fork exec switch preempt` on BOTH accelerators, byte-identical BAD lines: fork — parent and child q0/q1 replaced (q0.lo = 0x4282a000 / 0x42833000, a kernel PA, or 0; nbad=4); exec — the new image starts with the previous image's q1..q31/FPCR/FPSR (want 0, got c9c052145fe90939…, nbad=63); syscall (alarm(0)), yield, block, errstr — q0 zeroed by the kernel even with ONE worker (nbad=2); with two workers yield/block nbad=66 (the peer's whole state incl. FPCR/FPSR); preempt (two pure spinners, 133/192 `sched: switch` lines) nbad=66, each worker holding the OTHER worker's pattern (seed 9 got seed 10's b2ad405872ac0921); TCG -w 1 preempt also nbad=2 (q0 zeroed by the tick/IRQ path)
  implication: CONFIRMED, deterministic, both accelerators: (a) the NEON kernel clobbers q0/q1 on syscall, fork and IRQ paths; (b) every switch leaks the previous proc's whole FP/SIMD state; (c) fork and exec give the child/new image arbitrary state. The switch leg's spin never failed on its own: reschedule() never switches to a Ready+blocked_in_kernel sleeper, so a spinner is preempted only by another EL0-preempted proc — hence the dedicated preempt leg

- timestamp: 2026-11-21T20:40:00Z
  checked: design — kernel without FP (softfloat target) vs NEON kernel with a per-trap save in trap.S
  found: rustc 1.95-nightly's builtin aarch64-unknown-none-softfloat spec = abi "softfloat", features "+v8a,+strict-align,-neon". Scratch r9 with lib/aarch64-unknown-none-elf.json → abi softfloat, "+strict-align,-neon,-fp-armv8" builds virtshell clean except one upstream future-incompat note (core's stdarch neon intrinsics carry #[target_feature(enable = "neon")], rust-lang/rust#134375 — never linked); disassembly: 0 FP/SIMD instructions (stock: 90). clobber_abi("C") is accepted on the FP-less target (vector registers filtered) but demands explicit-register outputs (x16 scratch)
  implication: an FP-free kernel is cheap and compiler-enforced, matches the x86_64 twin (SSE-free) and the 9front donor (procsave/procrestore), removes the per-trap cost and also removes two latent KERNEL-side NEON hazards a per-trap design would have to fix separately (d8-d15 not in swtch's Context; an EL1h IRQ during idle_wait clobbering the interrupted kernel's vector registers) — chosen

- timestamp: 2026-11-21T20:50:00Z
  checked: GREEN run — scratch r9 (46a5c87 + softfloat JSON + fpu.rs + hooks) virtshell kernel $SP/fp/k-fix1.elf, same image, HVF and TCG, `/bin/fpprobe -w 1`, `/bin/fpprobe`, `/bin/fpprobe -w 4 -p 12` ($SP/fp/runs/fix1-{hvf,tcg}-1.log)
  found: `fpprobe: ok` ×3 on both accelerators; 406 (HVF) / 576 (TCG) `sched: switch` lines; 46a5c87's I-cache selftest line unchanged (HVF 64/64 fresh, control 64/64 stale; TCG 64/64, control 0/64); fixed-kernel disassembly: FP/SIMD only at the 14 inlined fpu save/restore sites (224 q-pair moves, 28 FPCR/FPSR moves) in sched callers, preempt (reschedule), sched_loop (pick_next), rfork_clone, exec_bootstrap (activate_first), exec_with_shebang (exec_replace reset), sys_await/sleep/semacquire/rendezvous (sched); .data +270,336 B (= 512 × 528, PROC_TABLE lives in .data), .bss +4 KiB
  implication: fix candidate green end to end; the only FP the kernel executes is the save/restore itself

- timestamp: 2026-11-21T21:08:00Z
  checked: r9 battery on base 46a5c87 ($SP/fp/r9-base) vs fix ($SP/fp/r9), inside nix develop ($SP/fp/bat-{base,fix}-*.log)
  found: check rc 0 (63 → 64), clippy rc 0 (82 → 83), test rc 0, aarch64 host tests rc 0, builds aarch64 virtshell / aarch64 default / x86-64 q35churnshell rc 0; tests passed 270 → 274 (the 4 new fpu tests), 0 failed. The ONLY warning-set differences: nix's "Git tree has uncommitted changes" (dirty scratch tree) on every leg and, on the two aarch64 builds, cargo's future-incompat summary for core (rust-lang/rust#134375, stdarch neon intrinsics under a softfloat ABI; never linked). x86_64 q35churnshell ELF sha256 1126b075… IDENTICAL base vs fix; aarch64 virtshell changes (70a37f2e… → 6cb73470…). rustfmt --edition 2024: fpu.rs clean; proc.rs 13 / exec.rs 5 / vminit.rs 1 hunks = base (pre-existing drift, none added)
  implication: battery green; x86_64 bytes unchanged → no boot-iso hash re-derivation and no x64 gates needed

- timestamp: 2026-11-21T21:16:00Z
  checked: boot-level mutation check — r9-mut = fix with ONE hook removed (or the target spec reverted), virtshell rebuilt, `/bin/fpprobe` on HVF ($SP/fp/mutants.{py,sh}, $SP/fp/runs/mut-*-hvf.log)
  found: none (fix) → ok; reschedule save removed → FAIL switch preempt; reschedule restore removed → FAIL switch preempt; sched save removed → FAIL switch; pick_next restore removed → FAIL switch preempt; rfork copy removed → FAIL fork; exec reset removed → FAIL exec; target spec back to +neon,+fp-armv8 (no softfloat) → FAIL fork exec switch preempt
  implication: 7/7 mutants killed — every hook is load-bearing and the probe discriminates each one; the host test kernel_target_spec_keeps_fp_simd_out_of_kernel_code additionally kills the spec revert without a boot. Not separately killable: activate_first's restore (the first proc's fpsave is ZERO and the boot CPU's FP file is zero on qemu — equivalent at boot; kept for the invariant's uniformity)

- timestamp: 2026-11-21T21:20:00Z
  checked: Harvey gates on the fix kernel (nix r9-kernel-virt-2edfdf8 via R9_OVERRIDE git+file scratch r9 @2edfdf8, R9_SKIP_BUILD=1): TCG from $SP/fp/harvey (main userland + host-built fpprobe), HVF from $SP/fp/harvey-sb (fully rebuilt userland)
  found: TCG smoke PASS (8 forks), net-smoke PASS (71), tier-1 PASS (374 forks, 43 T1-OK incl. `T1-OK float-d01`, zero T1-FAIL, exact SKIP set); HVF smoke / net-smoke / tier-1 PASS (same counts, float-d01 ok); 46a5c87's I-cache selftest line present in every boot (HVF 64/64 fresh, control 64/64 stale; TCG 64/64, control 0/64)
  implication: no regression in the composed gates; user float formatting (the awk canary) works under the per-proc state

- timestamp: 2026-11-21T21:23:00Z
  checked: Harvey wiring with a real userland rebuild — scratch clone harvey-sb, `subset-build.sh` with HARVEY_GUEST_DIR=harvey-build-fp (fresh guest dir, full tier-0/tier-1 build; the main checkout's guest dir untouched), then mkimage.sh, then fpprobe on stock vs fix kernels
  found: subset-build PASS (74 entries in arm64/bin; fpprobe built by 7c+7a+7l in the guest, magic 00008c47, rsynced back); guest-built fpprobe text+data byte-identical to the host-built one (text 38160, data 5232); mkimage PASS; stock 46a5c87: `fpprobe: FAIL fork exec switch preempt` ×2 on HVF and TCG; fix 2edfdf8 (the nix kernel the gates ran): `fpprobe: ok` ×2 on HVF and TCG
  implication: the committed wiring builds and bakes the probe; red→green reproduces on the rebuilt image

- timestamp: 2026-11-21T21:26:00Z
  checked: r9 landing — real checkout fast-forwarded main 46a5c87 → 2edfdf8 (fetched from the scratch clone; no push); clippy, aarch64 host tests and xtask test re-run on the committed content (the battery above ran before rustfmt reflowed one assert)
  found: clippy rc 0, 82 warnings (= base); aarch64 host tests 37/37; xtask test 237 passed, 0 failed
  implication: committed content green

- timestamp: 2026-11-21T21:40:00Z
  checked: owner decision (via coordinator): design kept, confirmed fixed, r9 2edfdf8 pushed by the coordinator; `T1-OK fpprobe` approved (43 → 44 expected T1-OK). Wired: tier1-sweep.rc r9-venue entry + nix/tier1-r9.sh R9_EXPECT_OK; tier-1 run after a userland rebuild (subset-build.sh, guest dir harvey-build-fp) from scratch clone harvey-sb at e2f61268b + the edits, R9_OVERRIDE r9 2edfdf8 ($SP/fp/series-t1/, $SP/fp/series-t1red/)
  found: TCG (with rebuild) PASS 44 T1-OK incl. `T1-OK fpprobe` + `T1-OK float-d01`; HVF PASS 44 T1-OK; red control on r9 46a5c87 (TCG): `fpprobe: FAIL fork exec switch preempt`, `T1-FAIL fpprobe`, post-pass `missing T1-OK fpprobe`. Committed Harvey 13f00747b. Harvey's flake.lock still pins r9 105d48e → tier-1 at that pin now reds on fpprobe until the pin series moves it
  implication: the regression guard is a gate, not just a baked probe

## Prevention

five_whys:
  - branch: "code — no user FP/SIMD context"
    why1: "the aarch64 trap frame (= Harvey's Ureg, 35 GPR/system words) and swtch's Context carry no vector state"
    why2: "the aarch64 port grew GPR-first from the donor's ABI; rc and the early sweep binaries use no FP, so nothing failed"
    why3: "the x86_64 twin set the precedent 'FP survives in place because the kernel never touches it' (14-08) and recorded the switch half as an era limit — the aarch64 port never had even that half because its kernel DID touch FP"
    actionable: "a per-proc FP save area with install/leave hooks, and a probe that interleaves two FP users"
  - branch: "config — NEON kernel"
    why1: "lib/aarch64-unknown-none-elf.json carried +neon,+fp-armv8 (the hardfloat default), so LLVM used vector registers for struct zeroing and popcount"
    why2: "no rule said the kernel must be FP-free on aarch64 (x86_64's target spec is soft-float by construction; the aarch64 JSON was copied from the hardfloat builtin)"
    actionable: "softfloat target + a host test that pins it"
why_not_caught: "no gate existed for this class — every arm64 gate binary (rc, coreutils, net tools) is integer-only except awk's single float canary (one process, no interleaved FP peer, and FPCR default happens to be 0 everywhere); no probe held vector state across a kernel entry"
recurrence_guard: "Harvey tier-1 `T1-OK fpprobe` (build/aarch64/tier1-sweep.rc r9-venue entry; nix/tier1-r9.sh R9_EXPECT_OK) reds on any leg; r9 host tests aarch64/src/fpu.rs fpu::tests::{restore_then_save_round_trips_every_register_and_both_control_words, zero_round_trips, save_reads_the_control_words_restore_wrote, kernel_target_spec_keeps_fp_simd_out_of_kernel_code} (the last kills a +neon revert without a boot)"
reference_design: "this arm64 design — FP-free kernel (softfloat target), eager per-proc save at leave / restore at install, copy on fork, zero on exec — is the reference for the x86_64 port (owner decision 2026-11-21)"

## Resolution

root_cause: AND-gate, both in r9 aarch64 — (1) code: the kernel kept no user FP/SIMD context at all: the trap frame (Ureg, 288 B) and swtch's Context are GPR-only, nothing saved or restored q0-q31/FPCR/FPSR when the proc on the CPU changed (reschedule, sched/pick_next, activate_first), and rfork/exec did not set them; (2) config: lib/aarch64-unknown-none-elf.json built the kernel with +neon,+fp-armv8, so kernel codegen itself writes vector registers in ordinary paths (struct zeroing movi v0.2d #0 + stp q; cnt) — user q0 zeroed inside plain syscalls, a kernel PA left in a fork child's q0. Either alone corrupts user state (cause 2 with no switches at all — the one-worker run; cause 1 even with an FP-free kernel — the mutants)
fix: r9 2edfdf8 `fix(aarch64): keep user FP/SIMD state per proc; build the kernel without FP` — kernel target → softfloat ABI, -neon,-fp-armv8 (0 FP/SIMD instructions in compiled kernel code); new aarch64/src/fpu.rs (FpState 528 B, save/restore/reset; asm text shared with a host round-trip test); Proc::fpsave with the invariant "on the CPU = live registers, off = fpsave": restore at install (activate_first, pick_next, reschedule), save at leave (sched, reschedule; exit drops), rfork_clone saves the parent's live registers into the child's fpsave, install_image zeroes fpsave and exec_replace zeroes the live registers. Eager (save/restore per switch), not lazy: an FP-free kernel already makes the per-trap cost zero, and 528 B per switch is noise beside vm::switch's TLB invalidate
verification:
  target_test:
    result: pass
    detail: "fpprobe (fork, exec, switch, preempt legs): stock 46a5c87 FAIL all four on HVF and TCG (byte-identical); fix 2edfdf8 ok on HVF and TCG with 1, 2 and 4 workers (406/576 preemption switches), host-built and guest-built userland"
  mutation_check:
    result: pass
    reason_if_skipped: "no Stryker for the no_std kernel — boot-level manual mutants instead"
    mutant_killed: "7/7: reschedule save, reschedule restore, sched save, pick_next restore, rfork copy, exec reset each removed → the matching leg FAILs; target spec reverted to +neon → FAIL all four legs (also killed by host test fpu::tests::kernel_target_spec_keeps_fp_simd_out_of_kernel_code). activate_first's restore is equivalent at boot (first proc's fpsave and the boot FP file are both zero) — kept for invariant uniformity"
  no_op_deletion:
    result: pass
    deletion_justified_by_rca: n/a — additive (296+/4-; the 4 deleted lines are the old target features line and a reworded vminit.rs comment)
  adjacent_tests:
    result: pass
    suites_run: ["cargo xtask check rc 0 (63 = base)", "cargo xtask clippy rc 0 (82 = base)", "cargo xtask test 237/0", "cargo test -p aarch64 --bins --target aarch64-apple-darwin 37/37 (33 + 4 fpu tests)", "cargo xtask build aarch64 virtshell / aarch64 default / x86-64 q35churnshell rc 0", "Harvey TCG smoke / net-smoke / tier-1 (T1-OK float-d01) PASS", "Harvey HVF smoke / net-smoke / tier-1 PASS", "subset-build (fresh guest dir) + mkimage PASS"]
  revert_and_reconfirm:
    result: pass
    bug_returned_on_revert: true — "stock 46a5c87 (fix absent) FAIL fork exec switch preempt; each single-hook revert reds its own leg"
    fixed_on_reapply: true — "2edfdf8 ok on HVF and TCG, twice per venue on the rebuilt image"
  x86_64_bytes: "unchanged — q35churnshell ELF sha256 1126b075bc29d291… identical base vs fix → no boot-iso hash re-derivation, no x64 gates"
  guardrail_verdict: accepted (pending human verification)
files_changed: [r9 lib/aarch64-unknown-none-elf.json, r9 aarch64/src/fpu.rs (new), r9 aarch64/src/proc.rs, r9 aarch64/src/exec.rs, r9 aarch64/src/main.rs, r9 aarch64/src/pre_mmu/vminit.rs (comment), Harvey build/aarch64/fpprobe.c (new), Harvey build/aarch64/fpprobeasm.s (new), Harvey build/aarch64/subset-build.sh, Harvey build/aarch64/mkimage.sh]
follow_ups:
  - "x86_64 r9 shares the switch-leak class: SSE-free kernel + exec fninit/ldmxcsr, but no per-proc FXSAVE/FXRSTOR on context switch (recorded era limit in x86_64/src/exec.rs) — two interleaved SSE/x87 users corrupt each other; exec also leaves XMM contents from the previous image"
  - "notes: 9front saves/restores FP state around notify/noted; r9 aarch64 delivers notes with the interrupted code's live FP state, so an FP-using note handler that returns NCONT clobbers the interrupted code's registers"
  - "toolchain: the core future-incompat note (rust-lang/rust#134375, stdarch neon intrinsics under a softfloat ABI) may become an error on a future nightly — revisit at the next toolchain bump"
  - "Harvey hazard (KB 'orclose-local-devices'): with mkimage.sh now requiring out/fpprobe, an R9_SKIP_BUILD=1 bake over a userland built before this commit fails until one subset-build"
