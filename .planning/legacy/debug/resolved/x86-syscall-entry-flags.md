---
status: resolved
trigger: "owner (via coordinator, 2026-11-22): fix WINDOWS #71 in r9 x86_64 with option A — IA32_FMASK (x86_64/src/cpu.rs fmask(), decision D-02a) masks only IF and syscallentry has no cld, so user TF/DF/NT/AC reach the kernel on SYSCALL (TF: #DB at the first kernel instruction on the user stack; DF: forward rep movs copy backwards; NT: set in a note handler reaches NOTED's IRETQ (0ffcec6) → #GP → kernel halt; AC reaches the kernel). Widen FMASK to IF|TF|DF|NT|AC as Linux does; user flags still return via R11/the saved frame; record the D-02a reversal; confirm NOTED/IRETQ does not carry user NT/IOPL into a bad state. Red/green probe on the x64 venue (DF copy, AC, NT in a note handler, TF via popf); host tests + mutants; x64 TCG auth/net-smoke/tier1 in an amd64 container; x86_64 host tests; r9 xtask check/clippy/test; re-derive boot-iso outputHash; aarch64 kernel bytes unchanged; arch-only; local commits; no WINDOWS.md edits; no AI attribution"
created: 2026-11-22T14:00:00Z
updated: 2026-11-22T18:00:00Z
---

## Current Focus

hypothesis: CONFIRMED and fixed — H1 (WINDOWS #71) IA32_FMASK = IF only let user TF/DF/NT/AC into ring 0; H2 the IDT entry kept the user's DF; H3 intrspecial's rdgsbase #UD'd on CPU0 (no CR4.FSGSBASE), so every #DB — incl. the legitimate ring-3 single-step — halted the kernel
test: DONE — r9 local main e6e5220 → 116bcc2 (10f675e FMASK, 6f8de40 intrcommon cld, f1cd71b intrspecial rdmsr, 116bcc2 boot-iso hash; unpushed); Harvey c15b1bbab (amd64 fpprobe flags leg, gated by the existing `T1-OK fpprobe`; no new marker)
expecting: DONE — owner confirmed fixed 2026-11-22 and kept all three commits; the coordinator pushed r9 main = 116bcc2
next_action: archived. Follow-ups done in Harvey right after: amd64/ABI.md entry-flags text, WINDOWS #71 fixed + two new rows (IDT DF, intrspecial RDGSBASE) fixed, lock-only pin e6e5220 → 116bcc2, quick pin pass recorded in 20-EVIDENCE.md
scratch: SP=/private/tmp/claude-501/-Users-neilskinner-code-harvey/55ded085-3397-4d2c-931d-1d3a7a81443a/scratchpad — $SP/xfl/{r9 (scratch r9 + mut-*/diag-* branches), r9.git, r9m, r9-base, fixture/, probe/, fx.sh, r1/r2/r3a/r3b scripts + *-summary.txt + captures, bat-base/bat-fix/bat-final, kernel-x64-{base,4bc5242,fix,mut-fmask-if}.elf, pa/fpprobe}
bug_class: Bohrbug (deterministic trust-boundary bug; every red/green/mutant boot reproduced first time, ~25-85 s each)
known_pattern_candidate: "noted-psr-trust — follow_ups DECISION (x86_64 entry flags, WINDOWS #71)"
reasoning_checkpoint:
  hypothesis: "H1: IA32_FMASK = IF only lets user TF/DF/NT/AC into ring 0 on SYSCALL (TF → #DB at syscallentry's first instruction; NT → #GP at the next kernel IRETQ — NOTED's, or another proc's because swtch carries no RFLAGS; DF → backwards compiler_builtins copies; AC → SMAP off once enabled). H2: the IDT gates keep DF and intrcommon has no cld, so a ring-3 interrupt runs the kernel with the user's DF. H3: intrspecial's rdgsbase #UDs on CPU0 (no CR4.FSGSBASE), so every #DB/NMI/#BP/#DF halts the kernel — which also blocks the TF fix's acceptance (a legitimate ring-3 #DB)"
  confirming_evidence:
    - "base e6e5220 red boots: nt/ntnoted/ntimage kernel `trap GP` at trap_return's iretq with flags 0x14086; dfirq kernel `trap PF` (smashed LockNode) with DF in the kernel flags; tf kernel `trap UD` at intrspecial's rdgsbase"
    - "DIAG kernels: base shows AC (alarm) and NT (NOTED) in ring 0 at syscall::entry; the fix shows none on any sub-leg"
    - "mutants on the fix: FMASK=IF → `trap DB` at syscallentry with the user RSP (#71's TF symptom), GP for NT; −NT → GP; −TF → DB; no cld → PF; rdgsbase → UD"
  falsification_test: "the final head failing any sub-leg, or a single-piece mutant passing the sub-leg that targets it (only the TCG-blind DF/AC FMASK bits are exempt at boot and are host-test-killed)"
  fix_rationale: "C1 masks the five bits where the CPU enters the kernel (the only place the TF trap can be prevented — an entry-stub reset runs after the #DB); R11/the frame still carry the user's flags back. C2 restores the SysV DF=0 invariant at the IDT entry (gates do not clear DF). C3 removes the FSGSBASE dependency from the special entry (rdmsr IA32_GS_BASE; same comparison)"
  blind_spots: "TCG drops DF on SYSCALL itself (QEMU env->df quirk), so the SYSCALL-DF damage is shown only by reasoning + host tests here (live on KVM/hardware); AC is inert without SMAP (DIAG-only); no NMI or #DF source exercises C3's other callers; single CPU (the AP path already sets FSGSBASE — rdmsr is correct there too); real hardware not run"
  candidate_causes:
    - "code: FMASK constant (D-02a inherited from a 6c kernel that never assumed DF=0)"
    - "code: IDT entry without cld"
    - "config/environment: CR4.FSGSBASE never set on CPU0 (TCG qemu64), with an entry path that assumes it"
    - "environment: TCG's SYSCALL clears DF regardless of FMASK — masks H1-DF on this venue (venue, not cause)"
  and_gate: "yes for the TF acceptance criterion: a ring-3 single-step survives only with BOTH C1 (no #DB in the kernel) and C3 (the ring-3 #DB entry does not #UD); the NT and DF findings are single-cause"

## Symptoms

expected: a user process entering the kernel via SYSCALL cannot make ring-0 code run with TF, DF, NT or AC set; user RFLAGS still come back intact on SYSRET (R11) or the NOTED IRETQ frame
actual: (from code audit in noted-psr-trust) FMASK = IF only; user TF/DF/NT/AC reach ring 0; not yet reproduced
errors: none observed yet (expected: #DB in kernel on TF, corrupted copies on DF, #GP halt on NT through NOTED's IRETQ)
reproduction: amd64 probe (to be written): DF + read into a buffer; AC; NT in a note handler before noted; TF via popf
started: always (D-02a since the x86_64 SYSCALL port)

## Eliminated

## Evidence

- timestamp: 2026-11-22T14:00:00Z
  checked: knowledge-base.md (MemPalace absent → keyword fallback), resolved/noted-psr-trust.md, resolved/fpu-x64-notes-kernel-simd.md
  found: noted-psr-trust follow_ups records this exact DECISION (option a = FMASK widen, b = entry-side reset which cannot prevent the TF #DB); x86 noted's image path is sanitized by force_user_return (cs/ss forced, IOPL cleared, IF set) — NT not mentioned; venue notes for the amd64 container (Docker Desktop amd64 ubuntu:24.04 --privileged, nix sandbox=false + filter-syscalls=false, non-root builder, gcc installed, docker exec -u builder -e HOME -e USER)
  implication: test the known candidate first; also check force_user_return for NT/RF/AC handling

- timestamp: 2026-11-22T14:10:00Z
  checked: r9 e6e5220 x86_64 — cpu.rs fmask() (+ main.rs arm_syscall_entry wrmsr), trap.rs syscallentry/syscallreturn/trap_return/intrcommon/intrregular/intrspecial, dat.rs Flags + force_user_return, syscall.rs entry/sys_noted, swtch.rs, l.S CR4
  found: fmask() = Flags::empty().with_intr(true) = 0x200 (doc cites D-02a "9k lets TF/DF pass through"); syscallentry's first instructions are swapgs / movq %rsp,%gs:SCRATCH / movq %gs:rsp0,%rsp — no cld, no flags reset; syscallreturn = SYSRETQ with r11 = frame.flags; a successful NOTED leaves via trap_return (IRETQ) with the kernel's LIVE RFLAGS; force_user_return forces cs/ss, clears IOPL, sets IF — NT/TF/AC/DF/RF in the restored image pass to ring 3 unchanged; swtch saves only rbx/rbp/r12-r15/sp/pc (NO RFLAGS), so whatever flags a kernel context carries flow to the next context the scheduler resumes (a parked reschedule → intrcommon iretq, or first_entry → trap_return iretq); intrcommon has no cld either (IDT gates clear TF/NT/RF/VM — and IF for interrupt gates — but architecturally KEEP DF and AC); CR4 = PGE|PAE|OSFXSR|OSXMMEXCPT (+FSGSBASE on APs): no SMAP
  implication: H1 (SYSCALL path) holds by reading; the NT #GP is not limited to NOTED — any kernel IRETQ after a NT-carrying syscall blocked and switched (no RFLAGS in swtch) can fault; NEW candidate H2: the IDT entry keeps user DF (gates do not clear it), so trap()/dispatch_irq/deliver_notes on the ring-3 interrupt path run with user DF; AC in ring 0 is inert without SMAP (alignment checks are CPL3-only) but would disable SMAP the day it is enabled

- timestamp: 2026-11-22T14:15:00Z
  checked: the donor's DF assumptions — Harvey sys/src/9k/k10/l64v.s (insb/inss/insl/outsb/outss/outsl), sys/src/libc/amd64/{memmove,memset,memcpy,strlen,strchr,...}.s, k10 syscall.c noted
  found: every donor string-instruction site sets DF itself (CLD before each REP; memmove's backward arm STD ... CLD) — the 6c kernel never assumes DF=0, which is why 9k could leave DF (and TF) out of Sfmask; r9's kernel is Rust: the SysV ABI requires DF=0 at every call and compiler_builtins' x86_64 mem routines (forced on for -none triples) issue rep movs/stos with no cld. Amd64 user code legitimately runs with DF=1 (libc memmove's backward copy), so a tick inside it enters the kernel with DF=1 via the IDT path
  implication: the D-02a premise ("whatever 9k emits") does not transfer to a Rust kernel — this is the reversal's "why"; H2 is reachable by ordinary programs (an overlapping memmove when the tick lands), not only by a hostile popf

- timestamp: 2026-11-22T14:25:00Z
  checked: base store kernel r9-kernel-x64-e6e5220 (kernel-x64.elf 01adbea3…, docker-cp'd from the container store) disassembled with llvm-objdump
  found: memcpy = rep movsb / rep movsq / rep movsb, memset = rep stosb / rep stosq / rep stosb, no cld in either; the kernel's only cld sites are l.S's three boot sites and memmove's own backward arm (std … cld); FMASK arming `movl $0x200, %eax; movl $0xc0000084, %ecx; wrmsr`; intrspecial+0xc = `rdgsbaseq %rax`; CR4 on CPU0 never gets FSGSBASE (l.S comment: TCG qemu64 cannot take it)
  implication: H1/H2 damage is real kernel-copy damage; NEW candidate H3: intrspecial's rdgsbase raises #UD on CPU0, so every #DB/NMI/#BP/#DF — including a legitimate ring-3 single-step after the FMASK fix — ends in a kernel `trap UD` halt

- timestamp: 2026-11-22T14:40:00Z
  checked: RED runs on base e6e5220 — amd64 fpprobe flags leg (scratch fixture sweep `fpprobe -F <legs>` + fixture expect that fast-fails on `kernel halted`; container r9x64fl, TCG, userland rebuilt once with the new probe, then R9_SKIP_BUILD=1; $SP/xfl/r1-summary.txt, r1-*.log)
  found: |
    ac,df: `fpprobe: ok` (both pass on base)
    nt: kernel `trap GP` pc=0xffffffff802bc9a1 cs=0x8 flags=0x14086 (NT set) — trap_return's `iretq`; the NT sleeper blocked, the scheduler resumed a fresh spinner through first_entry → trap_return with the sleeper's live NT (swtch carries no RFLAGS)
    ntnoted: the same `trap GP` at trap_return's iretq (NOTED's IRETQ)
    ntimage: the same `trap GP` (second NOTED with ring 3 holding the image's NT)
    dfirq: kernel `trap PF` pc=0xffffffff80265ab4 (port MCSLock::unlock, `movq (%rsi),%rax` with rsi=0x600003d40 — a smashed stack LockNode) flags=0x10446 (DF SET in ring 0), r9-r11 = dfspin's sentinels
    tf: kernel `trap UD` pc=0xffffffff802bc918 = intrspecial's rdgsbase, sp=0xffffff0000100fb8 (Mach region: the #DB gate's IST stack)
  implication: H1 confirmed for NT (three ways, deterministic, ~24 s/boot); H2 confirmed (kernel stack corruption → #PF on the interrupt path with DF=1); H3 confirmed (TF's #DB never reaches trap(): rdgsbase #UDs first, so TF cannot go green without fixing intrspecial). df passing on base contradicts the H1-DF prediction — see next entry

- timestamp: 2026-11-22T14:45:00Z
  checked: why flags-df passes on base under TCG — QEMU target/i386 helper_syscall (`env->eflags &= ~(env->fmask | RF_MASK); cpu_load_eflags(env, env->eflags, 0)`) and cpu_load_eflags (sets env->df from bit 10 of the value passed); TCG keeps DF in env->df and the arithmetic flags in CC_SRC, never in env->eflags
  found: on TCG the value masked by FMASK never holds DF, so cpu_load_eflags reloads DF=0 — TCG clears DF (and the arithmetic flags) on every SYSCALL regardless of IA32_FMASK; R11 still gets the true flags (cpu_compute_eflags), so DF comes back to the user
  implication: TCG is a BLIND venue for the SYSCALL-DF half of H1 (KVM/hardware would run the kernel copies backwards); the interrupt-path DF (H2) is live on TCG (gate delivery leaves env->df alone). To be confirmed directly with a DIAG kernel (live RFLAGS printed at syscall::entry): expect AC and NT visible, DF never, on base

- timestamp: 2026-11-22T15:30:00Z
  checked: scratch r9 $SP/xfl/r9 main = 10f675e (C1: fmask() = IF|TF|DF|NT|AC = 0x44700, D-02a reversal recorded in its doc; main.rs/trap.rs comments; force_user_return doc updated — NOTED leaves via IRETQ, image TF/DF/NT/AC are ring-3-only flags) → e4ecb9d (C2: `cld` first in intrcommon) → 4bc5242 (C3: intrspecial reads IA32_GS_BASE via rdmsr, rax/rcx/rdx saved; syscallentry doc updated). Darwin build: disassembly shows `movl $0x44700,%eax; movl $0xc0000084,%ecx; wrmsr`, `cld` at intrcommon+0, intrspecial = push rax/rcx/rdx; movl $0xc0000101,%ecx; rdmsr; …; no rdgsbase/wrgsbase anywhere in the kernel
  found: x86_64 host tests (container) 138/138 = 134 + cpu::tests::{fmask_clears_the_flags_the_kernel_must_not_inherit, fmask_is_exactly_if_tf_df_nt_ac}, trap::tests::{intrcommon_clears_df_before_calling_trap, entry_asm_uses_no_fsgsbase_instruction}. Host mutants: FMASK = IF only / −NT / −TF / −DF / −AC → both cpu tests FAIL (136/2 each); intrcommon cld removed → intrcommon test FAIL (137/1); ($SP/xfl/r2host-summary.txt)
  implication: every FMASK bit (incl. the two TCG cannot observe at boot, DF and AC) and the cld are pinned on the host

- timestamp: 2026-11-22T15:35:00Z
  checked: GREEN + DIAG boots (fixture, container, TCG; $SP/xfl/r2-summary.txt, r2-*.log). DIAG = scratch print in syscall::entry of the live RFLAGS when TF|DF|NT|AC (0x44500) is set
  found: |
    final 4bc5242 `-F all`: every sub-leg passes, `fpprobe: ok`; the tf child prints `user trap: unhandled vector 1 pc=0x2027d9` and exits `sys: trap` — 0x2027d9 = flagcall+0x1d, the PUSHFQ right after the SYSCALL (probe bytes `9d 0f 05 9c` = popfq; syscall; pushfq): TF came back through SYSRET's R11 and trapped in ring 3, never in the kernel
    diag-base (e6e5220+DIAG) `-F ac,df`: `DIAG syscall 6 entered ring 0 with rflags=0x40082` (AC in ring 0 on the alarm); NO line for the df leg's pread (50) / pwrite (51) although DF was set across both
    diag-base `-F ntnoted`: `DIAG syscall 29 entered ring 0 with rflags=0x4082` (NT in ring 0 on NOTED), then `trap GP` at trap_return's iretq
    diag-fix (4bc5242+DIAG) `-F all`: zero DIAG lines across all seven sub-legs, `fpprobe: ok`
  implication: AC reaching ring 0 is now observed (it was only inferred); TCG's DF-on-SYSCALL blindness is confirmed directly (the base kernel never saw DF at syscall entry); the fix leaves no TF/DF/NT/AC in ring 0 on any probed path

- timestamp: 2026-11-22T16:20:00Z
  checked: BOOT MUTANTS on 4bc5242 (one change each; fixture boots, TCG; $SP/xfl/r2-summary.txt), symbolized against each mutant's own store kernel where the layout moved
  found: |
    FMASK = IF only: tf → kernel `trap DB` pc=0xffffffff802bc9ac = syscallentry's first instruction (swapgs), flags 0x102 (TF), interrupted RSP 0x7fffffffecd0 = the USER stack — #71's TF symptom exactly, visible now that intrspecial no longer #UDs; nt / ntnoted / ntimage → kernel `trap GP` at trap_return's iretq (0xffffffff802bc9a9), flags 0x14086
    FMASK − NT: ntnoted → the same `trap GP`
    FMASK − TF: tf → the same `trap DB` at syscallentry on the user stack
    FMASK − DF: df → `fpprobe: ok` (TCG-blind, as predicted; host tests kill it)
    FMASK − AC: ac → `fpprobe: ok` (kernel-blind without SMAP; host tests kill it)
    intrcommon cld removed: dfirq → kernel `trap PF` in MCSLock::unlock (smashed LockNode), flags 0x10446 (DF)
    intrspecial back to rdgsbase: tf → kernel `trap UD` at the rdgsbase (0xffffffff802bc918)
  implication: every boot-observable piece of the fix is load-bearing (7/7 boot-visible mutants killed at boot; the two TCG-blind FMASK bits killed on the host); the darwin battery on 4bc5242 matched base (check 63, clippy 82 / x86-64 82 with identical diagnostic lines, aarch64 clippy pre-existing rc=1 with identical aarch64/port diagnostics — the 72→75 line count is xtask lib's two pre-existing collapsible-if warnings re-emitted by cargo cache order; xtask test 395/0; builds x86-64 default 22 / q35churnshell 60 / aarch64 default 20 / virtshell 38 / virtchurn 38 warnings = base; rustfmt: cpu.rs and dat.rs clean, trap.rs back to base's 3 hunks after folding rustfmt's shape of the two new tests into C2/C3 → head f1cd71b)

- timestamp: 2026-11-22T16:45:00Z
  checked: run 3a in the container on the code head f1cd71b ($SP/xfl/r3a-summary.txt); section compare of the 4bc5242 and f1cd71b store kernels (llvm-objcopy -O binary per section)
  found: asm-only rdgsbase host mutant (test kept) → trap::tests::entry_asm_uses_no_fsgsbase_instruction FAIL (137/1); kernel-x64 f1cd71b 3535848b… (28050432 B) vs 4bc5242 2a7e06aa… — .text/.rodata/.data identical, only debug sections differ (the DWARF line table carries each source file's checksum, so the cfg(test) reformat changed the ELF but not the code: run 2's green/mutant evidence transfers); kernel-virt 0b17869d… and kernel-virt-cpu b2da7e46… = base e6e5220 BYTE-IDENTICAL; boot-iso specified sha256-JQ/47nON4oZPk1SxxM5zkHARxCrH23ShgR9sJJCltCY= → got sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=
  implication: no aarch64 / HVF series needed; the hash is wired as scratch 116bcc2

- timestamp: 2026-11-22T17:15:00Z
  checked: run 3b — the exact landed content: Harvey 557c2ec6c + the committed probe (container copy a344123e3, files byte-identical to Harvey c15b1bbab's) with r9 = 116bcc2 ($SP/xfl/r3b-summary.txt, r3b-*.log, r3b-t1-final-capture.log); darwin xtask check/clippy/test on 116bcc2 ($SP/xfl/bat-final)
  found: x86_64 host tests 138/138; boot-r9-tier1-x64 (TCG, full default pipeline incl. the amd64 userland rebuild with the new probe) ALL STAGES PASS, `expected on r9: 39 T1-OK + 0 T1-SKIP (computed)`, 39 T1-OK incl. `T1-OK fpprobe` / `fpprobe: ok` (the default run now includes the flags leg; the capture carries the tf child's ring-3 `user trap: unhandled vector 1 pc=0x2027d9`, which the post-pass's `panic|Unhandled interrupt` does not match), 412 s; boot-r9-auth-x64 ALL STAGES PASS (235 s); boot-r9-net-smoke-x64 NET-SMOKE ALL STAGES PASS (100 s); wired boot-iso builds (boot.iso sha256 a7483ce1d3f363efe6ee16df4e4437ea5380507a9bc9791e97e95de2cf8f37cc), `--rebuild` deterministic; kernel-x64 final 3535848bdab363f226360c172caf95dc699a493e2c7a9ebf6af68ad10cb77ad4 (28050432 B; the hash commit left it unchanged); kernel-virt / kernel-virt-cpu = base. Darwin on 116bcc2: check 63 warnings, clippy 82 with diagnostic lines identical to base, xtask test 395/0, rustfmt cpu.rs/dat.rs 0 hunks, trap.rs 3 (= base)
  implication: verified on the landed content

- timestamp: 2026-11-22T17:25:00Z
  checked: landing and cleanup
  found: real r9 fast-forwarded e6e5220 → 116bcc2 (main, 4 ahead of origin/main, nothing pushed, author Neil Skinner, no attribution lines); Harvey c15b1bbab commits build/x86_64/fpprobe.c + fpprobeasm.s only (the 11 troff/postscript files untouched); container r9x64fl removed — docker containers / images / volumes lists identical to the pre-session snapshot (ubuntu:24.04 pre-existed); graphify update run (graphify-out is gitignored)
  implication: ready for human verification

## Prevention

five_whys:
  - branch: "code — FMASK = IF only (H1)"
    why1: "14-02 armed Sfmask per D-02a ('whatever 9k emits'), re-keying r9's old IF|TF|DF dead code down to the donor's If"
    why2: "D-02a treated the entry flags as part of the frozen user ABI, but they are a kernel-internal invariant: the donor's 6c kernel tolerated user DF (CLD before every REP), the Rust kernel cannot (SysV DF=0; compiler_builtins' bare rep movs/stos)"
    why3: "no probe ever entered the kernel holding TF/DF/NT/AC, and SYSRET (the only return path then) never checks NT; 0ffcec6's IRETQ NOTED made NT fatal without anyone re-auditing the entry flags"
    actionable: "FMASK = the Linux set; host tests pin each bit; boot probe flags leg on the x64 tier-1"
  - branch: "code — IDT entry kept DF (H2)"
    why1: "intrcommon was ported from the donor's l64idt.s, which needs no cld for a 6c kernel"
    why2: "no x64 gate interrupts ring-3 code that holds DF (libc memmove's backward arm is short; a tick landing inside it is rare and its damage diffuse)"
    actionable: "cld first in intrcommon; fpprobe flags-dfirq interrupts a DF-holding spin deterministically"
  - branch: "config/environment — RDGSBASE without CR4.FSGSBASE (H3)"
    why1: "CPU0's l.S path deliberately leaves FSGSBASE off (TCG qemu64), but intrspecial (pre-dating that choice) still used RDGSBASE"
    why2: "nothing ever raised #DB/#BP/NMI/#DF on an x64 gate, so the special entry was dead code in practice"
    actionable: "rdmsr IA32_GS_BASE; host test bans FSGSBASE instructions from trap.rs; fpprobe flags-tf takes a ring-3 #DB on every x64 tier-1"
why_not_caught: "no gate existed for this class — no probe set TF/DF/NT/AC before a syscall or an interrupt, no x64 test raised #DB, and the TCG venue drops DF on SYSCALL by itself (QEMU keeps DF in env->df, which helper_syscall reloads from an eflags value that never holds it), hiding the SYSCALL-DF damage even from a DF probe"
recurrence_guard: "Harvey `T1-OK fpprobe` on the x64 tier-1 now runs the flags leg (ac, df, nt, ntnoted, ntimage, dfirq, tf — base e6e5220 halts on nt/ntnoted/ntimage/dfirq/tf); r9 host tests x86_64/src/cpu.rs cpu::tests::{fmask_clears_the_flags_the_kernel_must_not_inherit, fmask_is_exactly_if_tf_df_nt_ac} (kill every FMASK narrowing, incl. the TCG-blind DF/AC) and x86_64/src/trap.rs trap::tests::{intrcommon_clears_df_before_calling_trap, entry_asm_uses_no_fsgsbase_instruction}"

## Resolution

root_cause: "three x86_64 kernel-entry defects, one class (the kernel ran with RFLAGS state it cannot run with): (H1, WINDOWS #71) IA32_FMASK = IF only (D-02a, the 9k donor's value) let the user's TF, DF, NT and AC into ring 0 on every SYSCALL — TF single-stepped the entry's first instruction (kernel `trap DB`, user RSP), NT made the kernel's next IRETQ #GP (NOTED's, or another process the scheduler resumed through first_entry/reschedule, because swtch carries no RFLAGS), DF ran compiler_builtins' bare rep movs/stos backwards, AC sat in ring 0 (inert without SMAP, SMAP-off once enabled); the donor could leave TF/DF alone because its 6c kernel sets DF before every string instruction, the Rust kernel relies on the SysV DF=0 invariant. (H2) the IDT gates keep DF and intrcommon had no cld, so any interrupt taken while ring 3 held DF (amd64 libc memmove's backward copy sets it) ran the kernel with backwards copies — kernel stack corruption, #PF. (H3) intrspecial decided its SWAPGS with RDGSBASE, which needs CR4.FSGSBASE that the boot CPU never sets: every #DB/NMI/#BP/#DF #UD'd inside the entry and halted the kernel — including the legitimate ring-3 single-step that H1's fix produces; AND-gate for the TF criterion: H1 and H3 both had to be fixed"
fix: "r9 scratch → local main e6e5220 → 10f675e (fmask() = IF|TF|DF|NT|AC, the Linux set; D-02a reversal + why in its doc; force_user_return doc: NOTED leaves via IRETQ, image TF/DF/NT/AC are ring-3-only flags because every kernel entry clears them and IRETQ's NT check reads the live RFLAGS — NT needs no clearing there, FMASK makes it moot) → 6f8de40 (cld first in intrcommon; AC left with a CLAC note for SMAP) → f1cd71b (intrspecial: rdmsr IA32_GS_BASE, rax/rcx/rdx saved, same KMACH comparison) → 116bcc2 (boot-iso outputHash sha256-JQ/47nON… → sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=). Host tests cpu::tests::{fmask_clears_the_flags_the_kernel_must_not_inherit, fmask_is_exactly_if_tf_df_nt_ac}, trap::tests::{intrcommon_clears_df_before_calling_trap, entry_asm_uses_no_fsgsbase_instruction}. Harvey: amd64 fpprobe flags leg (ac, df, nt, ntnoted, ntimage, dfirq, tf) in the default run — gated by the existing `T1-OK fpprobe`, no new marker; `-F sublegs` runs only flags sub-legs"
verification:
  target_test:
    result: pass
    detail: "fpprobe flags leg: base e6e5220 halts the kernel on nt / ntnoted / ntimage (`trap GP` at trap_return's iretq, NT live), dfirq (`trap PF`, DF live) and tf (`trap UD` at intrspecial's rdgsbase); 116bcc2 passes every sub-leg (fixture `-F all`, and the default run inside the x64 tier-1: 39 T1-OK). The tf child dies `sys: trap` with the #DB at flagcall+0x1d — the instruction after the SYSCALL, in ring 3. DIAG kernels: base shows AC and NT in ring 0 at syscall::entry, the fix none"
    oracle_type: "implicit (kernel halt) for nt/ntnoted/ntimage/dfirq/tf, specified for df/dfirq data + registers + returned RFLAGS bits, specified for tf (exit status `sys: trap`)"
  mutation_check:
    result: pass
    mutant_killed: "boot (TCG): FMASK=IF → tf `trap DB` at syscallentry (user RSP) + nt/ntnoted/ntimage GP; −NT → GP; −TF → DB; no cld → dfirq PF; rdgsbase → tf UD (7/7 boot-visible). FMASK −DF / −AC are boot-blind on TCG (QEMU drops DF on SYSCALL; AC inert without SMAP) and killed by cpu::tests (both FAIL); all FMASK narrowings, the cld removal and the asm-only rdgsbase revert are killed on the host"
  no_op_deletion:
    result: pass
    deletion_justified_by_rca: "the only removed code is intrspecial's rdgsbase sequence (H3), replaced by the rdmsr read with the same comparison"
  adjacent_tests:
    result: pass
    suites_run: ["container: x86_64 host tests 138/138 (134 + 4)", "x64 TCG gates on 116bcc2: tier-1 (userland rebuilt) 39 T1-OK, auth, net-smoke — ALL STAGES PASS", "darwin: xtask check 63 / clippy 82 (identical lines) / clippy x86-64 identical / clippy aarch64 pre-existing rc=1 with identical aarch64+port diagnostics / xtask test 395/0", "builds x86-64 default + q35churnshell, aarch64 default + virtshell + virtchurn at base warning counts", "rustfmt at base hunk counts"]
  revert_and_reconfirm:
    result: pass
    bug_returned_on_revert: true — "e6e5220 and each single-piece mutant red the sub-leg aimed at it"
    fixed_on_reapply: true — "4bc5242 / f1cd71b (code-identical) / 116bcc2 green"
  x86_64_bytes: "CHANGED — kernel-x64.elf 01adbea3… (e6e5220) → 3535848bdab363f226360c172caf95dc699a493e2c7a9ebf6af68ad10cb77ad4 (116bcc2, 28050432 B); boot-iso outputHash re-derived (sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=), wired build green, --rebuild deterministic"
  aarch64_bytes: "UNCHANGED — kernel-virt 0b17869d…, kernel-virt-cpu b2da7e46… byte-identical to e6e5220: no HVF series needed"
  guardrail_verdict: accepted — owner confirmed fixed 2026-11-22 (all three commits kept; r9 116bcc2 pushed by the coordinator)
noted_iretq_audit: "force_user_return clears IOPL and sets IF but does NOT clear NT (nor TF/DF/AC/RF/ID) in the restored image — and does not need to: IRETQ's #GP condition is the kernel's LIVE RFLAGS.NT, never the image, and after the fix no kernel context can hold NT (SYSCALL masks it, interrupt/exception gates clear it, swtch carries no flags but every kernel context starts clean); the image's NT/TF/DF/AC load into ring 3 only, where a CPL3 POPFQ could set them anyway (ntimage proves it: ring 3 runs with the image's NT, blocks, and takes a second NOTED IRETQ without fault). FMASK makes NT moot there"
follow_ups:
  - "Harvey amd64/ABI.md still documents the donor's entry mask (`Flags mask on entry: Sfmask = If ONLY ... TF/DF pass through untouched`, and `Sfmask = If (IF only — see Wire contract)` in the MSR-arming list): for r9 the mask is IF|TF|DF|NT|AC and the user's flags still return unchanged via R11 / the NOTED frame. Not edited here (this session commits only debug files and probe wiring) — for the coordinator"
  - "Harvey's r9 pin: the x64 tier-1 at c15b1bbab needs r9 >= 116bcc2; at the e6e5220 pin the flags leg halts the kernel (red by design until the pin moves)"
  - "ledger: H2 (IDT DF) and H3 (intrspecial RDGSBASE #UD — any user int3/single-step halted the kernel) are new findings fixed here under #71's class; WINDOWS.md left to the coordinator"
  - "TCG is blind to the SYSCALL-DF damage (QEMU's helper_syscall reloads DF from an eflags value that never holds it); the df sub-leg is live on KVM (the gates' default accel) and hardware"
  - "AC: inert at CPL0 until SMAP is enabled; intrcommon's comment marks where a CLAC goes then (and the SYSCALL path already masks AC)"
  - "r9 x86_64 kills a process on any ring-3 trap (`sys: trap`) instead of posting a `sys: trap: ...` note the donor delivers to a notify handler — pre-existing, unchanged; it is why tf asserts the exit status"
  - "every x64 tier-1 capture now contains the tf child's `user trap: unhandled vector 1` line by design; a future post-pass that matches `unhandled` must allow it"
files_changed: [r9 x86_64/src/cpu.rs, r9 x86_64/src/trap.rs, r9 x86_64/src/dat.rs (doc), r9 x86_64/src/main.rs (comment), r9 flake.nix (boot-iso hash), Harvey build/x86_64/fpprobe.c, Harvey build/x86_64/fpprobeasm.s]
