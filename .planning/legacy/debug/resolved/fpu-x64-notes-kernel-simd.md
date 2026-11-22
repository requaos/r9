---
status: resolved
trigger: "follow-ups of arm64-user-fpsimd-unsaved (owner, via coordinator, 2026-11-21): (a) x86_64 per-process FPU/SSE save/restore at context switch, copy on fork, reset on exec, following the arm64 reference design (the x86_64 kernel is already SSE-free); (b) note handlers on aarch64 (and x86_64 if the same applies) must save the interrupted FP state at notify and restore it at noted, as 9front does; (c) an explicit kernel-SIMD API on aarch64 and x86_64 (Linux kernel_neon_begin/end, kernel_fpu_begin/end shape: scoped guard + with_simd closure, nested begin panics by name, preemption/IRQ policy explicit, kernel stays built without FP, one example SIMD routine used only by a host test and a boot self-test)"
created: 2026-11-21T21:55:00Z
updated: 2026-11-22T02:40:00Z
---

## Current Focus

hypothesis: CONFIRMED and fixed — (a) x86_64 kept the FPU/SSE registers in place across switches, fork and exec; (b) both arches handed a note handler's FP state to the interrupted code; (c) no kernel-SIMD API; plus a separate x86_64 defect the probe exposed: notify refused handlers not 4-byte aligned
test: DONE — r9 3f69729 (local main, unpushed: d07fdbe aarch64, 4eafbfc x86_64, d76d872 notify, 3f69729 boot-iso hash); Harvey b77d134fc (arm64 note leg) + the amd64 probe/wiring commit (no marker)
expecting: RESOLVED — owner: "confirmed fixed", x64 marker approved (A); r9 3f69729 pushed by the coordinator; x64 `T1-OK fpprobe` landed (Harvey 26910c78e)
next_action: none here — the three follow-ups (x86 async note delivery with an iretq NOTED, NSAVE on both arches, per-CPU FP bookkeeping) continue in .planning/debug/notes-async-nsave-percpu-fp.md
reference_design: "arm64 (r9 2edfdf8): FP-free kernel, eager per-proc save at leave / restore at install, copy on fork, zero on exec — the reference for x86_64 (owner decision)"
rules: "arch-only (port/src needs a checkpoint with the diff); red/green probes; host tests; aarch64 HVF/TCG gates; x86_64 bytes change → x64 TCG auth, net-smoke, tier1 in an amd64 container + x86_64 host tests + boot-iso outputHash re-derived and committed locally + cleanup; local commits only; checkpoint before any new tier-1 expected marker on either arch"

## Symptoms

expected: (a) on x86_64 a process's XMM0-15, MXCSR and x87 state survive every switch, a fork child inherits them, an exec'd image starts from the architectural default; (b) on both arches the interrupted code's FP state is intact after noted(NCONT/NRSTR) even if the handler used FP; (c) kernel code can run SIMD inside a guarded section without disturbing user state
actual: (a) by reading x86_64/src/exec.rs fp_reset_for_exec: "no per-proc FXSAVE/XRSTOR on context switch — the FP registers survive switches in place"; (b) by reading aarch64 trap.rs deliver_notes / syscall.rs sys_noted: Ureg only; (c) absent
errors: none observed yet
reproduction: to be built (probes)
started: always

## Eliminated

## Evidence

- timestamp: 2026-11-21T21:55:00Z
  checked: knowledge-base.md (MemPalace absent → keyword fallback) and 9front donor sys/src/9/{arm64,pc64}/fpu.c + port/sysproc.c
  found: KB entry arm64-user-fpsimd-unsaved (the reference design). Donor: notify() calls fpunotify(up) = fpuprocsave + FPnotify flag; the handler's first FP use gets a COPY of the interrupted state (notefpsave pushes a new fpsave node); sysnoted calls fpunoted(up) whenever up->notified (NCONT, NRSTR, NSAVE), popping back to the interrupted state (or FPinit if the interrupted code never used FP); NDFLT exits. Same in pc64
  implication: (b) target semantics: save at notify, handler works on a copy, restore at noted(NCONT/NRSTR); r9 has no NSAVE (unimplemented) so only NCONT/NRSTR restore

- timestamp: 2026-11-21T22:05:00Z
  checked: x86_64 r9 by reading (proc.rs scheduler, trap.rs idle_wait, l.S CR0/CR4, exec.rs fp_reset_for_exec, syscall.rs deliver_notes/sys_noted); amd64 container venue
  found: kernel context runs IF=0 (idle_wait's sti;hlt;cli the one window); install points pick_next (every resume) + activate_first; leave points reschedule (park into the sched loop) + sched; sched_from_moribund for exit; fork children start note_ureg = 0 (both arches); CR0.TS/MP cleared, CR4.OSFXSR|OSXMMEXCPT set, no OSXSAVE (AVX unusable at CPL3 → FXSAVE covers all user-visible state). Container r9x64fp (ubuntu:24.04 amd64, --privileged, nix 2.36; `docker exec` needs -e HOME and -e USER for nix.sh) baseline at r9 2edfdf8: x86_64 host tests 116/116; boot-r9-tier1-x64 TCG ALL STAGES PASS, 38 T1-OK, 467 s
  implication: the aarch64 shape maps 1:1 (install/leave/discard/snapshot/replace on an FXSAVE image)

- timestamp: 2026-11-21T22:15:00Z
  checked: aarch64 implementation in scratch r9 ($SP/fp2/r9 on 2edfdf8): fpu.rs gains LIVE_OWNER (the fpsave slot whose state is live) with install/leave/discard/snapshot/replace, the KernelSimd guard (kernel_begin / Drop / with_simd; nested begin panics by name; begin asserts IRQs masked; install/leave/snapshot/replace panic by name inside a section), xor_fold16 (ld1+eor inline asm under `.arch_extension simd`, takes &KernelSimd) + scalar reference, and a boot selftest; Proc.note_fpsave recorded by deliver_notes (snapshot) and put back by NOTED NCONT/NRSTR (replace); exec zeroing moved into install_image (replace); sched_from_moribund discards
  found: builds clean (virtshell 39 warnings = 2edfdf8); aarch64 host tests 46/46 (+9: section parks the owner and gives the CHANGED state back, no-owner section scrubs, nested begin / leave / install inside a section panic by name, snapshot/replace/leave follow ownership, xor_fold16 = scalar for 0..4096 B with nonzero folds, ragged length rejected). Boot selftest line on HVF and TCG: `fpu: kernel-SIMD selftest ok (... xor_fold16 0x1983c6d50a18d707c9d74d9a42c59f27)` (= the Python-computed fold of the LCG buffer; a first draft's structured fill folded to 0 and was replaced)
  implication: the guard works at EL1 on real registers; the checksum proves the vector routine computes

- timestamp: 2026-11-21T22:25:00Z
  checked: aarch64 fpprobe gains a note leg (Harvey scratch $SP/fp2/harvey build/aarch64/fpprobe.c: child with notify handler; note-sync via postnote to self, note-alarm via alarm(20) while spinning; handler stores its entry state, loads a different pattern, noted(NCONT)); host 7c/7a/7l build, FP scan unchanged (FP only in fpload/fpstore + libc float routines); baked; booted on r9 2edfdf8 (nix kernel) and the new scratch kernel
  found: RED on 2edfdf8, HVF and TCG identical: `BAD note-sync ... got=a1ad405872ac0921 nbad=66` and `BAD note-alarm ... nbad=66` (the interrupted code returns holding the HANDLER's pattern, seed 21), `fpprobe: FAIL note` (fork/exec/switch/preempt still ok). GREEN on the new kernel: `fpprobe: ok` on HVF and TCG; note-entry checks pass on both (the handler starts with a copy of the interrupted state, the 9front semantics)
  implication: (b) confirmed on aarch64 and fixed

- timestamp: 2026-11-21T23:30:00Z
  checked: x86_64 implementation (scratch r9): x86_64/src/fpu.rs = FXSAVE64 image FpState (DEFAULT = FCW 0x037F, MXCSR 0x1F80, rest zero), the same LIVE_OWNER API and KernelSimd guard (begin asserts IF=0), xor_fold16 in SSE2 inline asm, boot selftest; proc.rs install at activate_first/pick_next, leave at sched + reschedule's park, discard at exit, snapshot in rfork_clone; exec.rs fp_reset_for_exec → fpu::replace(DEFAULT) in install_image; syscall.rs deliver_notes/sys_noted note_fpsave. Darwin battery on the combined tree
  found: x86-64 builds (default + q35churnshell) clean; kernel SSE/x87 instructions only in fpu fns + their inlined sites (base: fninit + ldmxcsr only); cargo xtask check 64 / clippy 83 warnings = 2edfdf8's sets (after `#[allow(dead_code)] mod fpu` on aarch64 — the kernel-SIMD API has no caller outside the dev_exec selftest by design; first pass showed 8 dead-code warnings in non-exec configs); clippy --arch aarch64 rc=1 at base and new with identical aarch64/port diagnostics (pre-existing), --arch x86-64 identical; xtask test 237/0; aarch64 host tests 46/46; builds aarch64 default / virtchurn / virtshell rc 0; rustfmt: new fpu.rs files clean, touched files keep base hunk counts. Scratch commits r9 3f8043c (aarch64) + 3d6d905 (x86_64)
  implication: ready for the x64 container and the aarch64 gates

- timestamp: 2026-11-21T23:55:00Z
  checked: aarch64 gates on r9 3d6d905 (nix r9-kernel-virt-3d6d905 via R9_OVERRIDE) from scratch harvey-sb at the scratch wiring branch (830c9e995: note-leg fpprobe + amd64 probe wiring): tier-1 TCG WITH a userland rebuild (subset-build, guest dir harvey-build-fp), then smoke/net-smoke TCG and smoke/net-smoke/tier-1 HVF ($SP/fp2/series/)
  found: all 6 PASS; tier-1 44 T1-OK on TCG and HVF incl. `fpprobe: ok` (note leg included); every boot prints the icache selftest (HVF 64/64 + control 64/64 stale; TCG 64/64 + 0/64) and `fpu: kernel-SIMD selftest ok (... 0x1983c6d50a18d707c9d74d9a42c59f27)`
  implication: no aarch64 regression; T1-OK fpprobe now also pins the note semantics

- timestamp: 2026-11-21T23:58:00Z
  checked: aarch64 boot-level mutants on 3d6d905 (one change each, virtshell, `/bin/fpprobe` on HVF; $SP/fp2/mutants.{py,sh})
  found: none → ok; kernel_begin's park removed → boot panics `fpu: kernel-SIMD selftest FAILED: the user state was not parked and restored across the section`; Drop's restore removed → same named panic; deliver_notes snapshot removed → FAIL note; NOTED replace removed → FAIL note; reschedule leave removed → FAIL switch preempt; pick_next install removed → FAIL fork exec switch preempt note; rfork snapshot removed → FAIL fork; install_image replace removed → FAIL exec
  implication: 8/8 killed — every new/reworked hook is load-bearing; nested-begin and in-section scheduler panics are pinned by host tests (not boot-observable by design)

- timestamp: 2026-11-22T00:20:00Z
  checked: amd64 fpprobe (Harvey scratch build/x86_64/fpprobe.c + fpprobeasm.s: FXSAVE64/FXRSTOR64 BYTE-encoded, arg in BP; legs fork/exec/switch/preempt/note; the sync note goes through an fd opened before the window so only the write stub runs inside it) baked into the x64 TIER image (stage-build.sh builds it, mkimage.sh tier bake copies it to /amd64/bin/fpprobe); a SCRATCH-ONLY tier1-sweep.rc fixture entry runs it in the x64 tier-1 gate (container r9x64fp, TCG, full default pipeline incl. the amd64 namespace build). Host-built probe text scan: SSE/x87 only in fpload/fpstore + libc float routines (the host nm cannot read amd64 a.outs — a64scan.py parses the Plan 9 symbol table itself)
  found: RED on r9 2edfdf8: `fpprobe: FAIL exec switch note` — exec: the new image starts with the previous image's ST registers (`st0.byte0 want=00 got=21 nbad=335`: fninit+ldmxcsr leave ST/XMM contents), switch/block: `fcw want=09 got=0a` (the peer worker's FCW after a sched() switch, nbad 51/139), note: see next entry; preempt passed (x86's tick is 10 ms and a parked proc is Ready+blocked_in_kernel, so only one switch per pair can happen — the 2M-iteration spin fit inside a tick under TCG; default raised to 20M for x86). On the first x86 FPU build (3d6d905): fork/exec/switch/preempt pass, note still FAIL; auth / net-smoke / tier-1 (committed wiring, no fixture) all PASS with `fpu: kernel-SIMD selftest ok (... 0x1983c6d50a18d707c9d74d9a42c59f27)`; boot-iso FOD at 3d6d905 got sha256-2HVgVA8JatLutqrqTfmFSN9VIp+UpylrfWTMev/AsNg= (superseded — see below)
  implication: (a) confirmed on x86 and fixed by the per-proc FXSAVE design

- timestamp: 2026-11-22T00:30:00Z
  checked: the x86 note-leg failure (`note: pid 129 has no notify handler`, child exits "fpprobe") on BOTH kernels; x86_64/src/syscall.rs sys_notify / deliver_notes; the donor sysnotify
  found: SEPARATE x86_64 DEFECT — sys_notify refused any handler whose address is not 4-byte aligned (`notify: misaligned handler`, the aarch64 port's check carried over) and deliver_notes killed for the same reason; x86 code has no alignment rule and 6l packs functions at byte granularity (the probe's nhandler sat at an unaligned address), the donor only validates the address. The probe never saw notify() fail (it did not check the return)
  implication: fixed as its own commit (scratch r9 d76d872: drop the alignment tests, keep the in-text validation; host test notify_accepts_a_handler_at_an_odd_text_address); both probes now print `BAD note notify: %r` if notify() fails. Any amd64 program with an unaligned handler (e.g. rc's DEL handler, by luck of layout) was affected

- timestamp: 2026-11-22T00:35:00Z
  checked: x86_64 host tests in the container (r9 3d6d905)
  found: 125 passed / 4 failed — (1) three exec tests panicked `fpu: replace inside a kernel SIMD section`: the ownership statics were process-global, so an fpu test's open section on another test thread leaked into exec tests running in parallel (the aarch64 suite has no parallel caller of replace/snapshot, so it never showed); (2) restore_then_save failed at byte 0 (FCW): the container's x86 is Rosetta ("VirtualApple"), which forces the reserved FCW bit 6 to 1 (FINIT's 0x037F sets it)
  implication: both fpu.rs files now keep LIVE_OWNER/IN_KERNEL_SIMD behind a `percpu` module — atomics in the kernel, thread_local! in host tests (a test thread is a "CPU"); the x86 test images set FCW bit 6. Recommitted scratch d07fdbe (aarch64) + 4eafbfc (x86_64) + d76d872 (notify)

- timestamp: 2026-11-22T01:25:00Z
  checked: container run 2 ($SP/x64fp/x64run2-summary.txt) at scratch r9 d76d872 — x86_64 host tests; x64 tier-1 with the scratch fixture: red at 2edfdf8, green at d76d872, six x86 mutant branches (one hook removed each); auth / net-smoke / tier-1 on the committed wiring; boot-iso FOD
  found: x86_64 host tests 130/130 (116 + the fpu tests + the notify test); RED 2edfdf8 `FAIL exec switch note` (note: `BAD note notify: notify: misaligned handler`); green d76d872: every leg ok except `note-alarm handler ran 0 times` — r9 x86_64 delivers notes ONLY at the syscall boundary (trap.rs: "the note boundary runs in the syscall entry path"), so an alarm cannot interrupt ring-3 code until its next syscall. Mutants: note replace removed → `BAD note-sync ... fcw want=06 got=05` FAIL note; reschedule leave removed → `BAD spin ... fcw want=09 got=07` FAIL preempt (the 20M spin reds it); pick_next install removed → FAIL exec switch preempt note; rfork snapshot removed → FAIL fork; install_image replace removed → `BAD exec ... fcw want=7f got=3f` FAIL exec; kernel_begin park removed → boot panics (selftest). auth (382 s), net-smoke, tier-1 wiring ALL STAGES PASS with `fpu: kernel-SIMD selftest ok (... 0x1983c6d50a18d707c9d74d9a42c59f27)`. boot-iso at d76d872: specified sha256-B2nbReb8… (right for the 2edfdf8 bytes, kernel-x64.elf e791e863…) → got sha256-x3KlbAktFO0pjynnz0GGrDUfIfFd/a/jnhL7H5b1h8Y=; wired clone builds (boot.iso ebc93ee8…), --rebuild deterministic; kernel-x64.elf 4b0cf307… (28040560 B; +2.1 MB = the churn profile's 2048-entry PROC_TABLE gaining two 512-byte FXSAVE images)
  implication: x86 (a) and (b) pinned for syscall-boundary delivery; async note delivery on x86 is a SEPARATE pre-existing gap (and NOTED returns via SYSRET, which clobbers rcx/r11, so interrupt-time delivery also needs an iretq NOTED path) — follow-up, not widened here; the amd64 probe's second note round now interrupts a blocking sleep (delivered at the sleep's return, after a switch away and back) instead of spinning

- timestamp: 2026-11-22T01:40:00Z
  checked: r9 landing — scratch main fetched into the real checkout and fast-forwarded 2edfdf8 → 3f69729 (d07fdbe aarch64, 4eafbfc x86_64, d76d872 notify, 3f69729 boot-iso hash); nothing pushed; no attribution lines. r9's own aarch64 boot tests (`cargo xtask boot-test --arch aarch64 --config <c>`, R9_HARVEY_DIR = the main Harvey checkout) at base 2edfdf8 in the real checkout
  found: base: virt, virtexec, virtexec3, virtmulti, virtnotes, virtpanic, virtchurn, virtip, virtnet, virtlowecam PASS; virtshell rc=1 BY DESIGN (no initrd in its config — `dev_exec: config carries no initrd image`; Harvey's launcher supplies it). A first attempt from the scratch clone hung: the boot-test unix socket path ($SP/fp2/r9/target/boot-test-*.sock, 141 chars) exceeds macOS's 104-byte sun_path — the real checkout's short path works
  implication: boot-test baseline recorded; the new-side run follows on the landed tree

- timestamp: 2026-11-22T01:58:00Z
  checked: container run 3 ($SP/x64fp/x64run3-summary.txt, final probe sources): fixture tier-1 with a userland rebuild at d76d872; the note mutant; the committed wiring's tier-1 with a rebuild. Then the container was removed
  found: fixture tier-1 ALL STAGES PASS, 39 T1-OK incl. `T1-OK fpprobe` / `fpprobe: ok` (all five legs); note mutant → `BAD note-sync ... fcw want=06 got=05` + `BAD note-alarm ... fcw want=07 got=05` FAIL note; committed wiring tier-1 ALL STAGES PASS (38 T1-OK, rebuilt). Cleanup: `docker rm -f r9x64fp`; images / volumes / containers lists identical to the pre-session snapshot (ubuntu:24.04 pre-existed)
  implication: x86_64 verified on the final sources

- timestamp: 2026-11-22T02:06:00Z
  checked: final aarch64 gates on the LANDED r9 3f69729 (nix r9-kernel-virt-3f69729) from harvey-sb at the final scratch wiring: tier-1 TCG with a userland rebuild, smoke/net-smoke TCG, smoke/net-smoke/tier-1 HVF ($SP/fp2/series-final/); r9's own aarch64 boot tests on the landed tree (real checkout)
  found: all 6 gates PASS; tier-1 44 T1-OK with `fpprobe: ok` (note leg) on TCG and HVF; every boot prints the kernel-SIMD selftest ok line. Boot tests: virt, virtexec, virtexec3, virtmulti, virtnotes, virtpanic, virtchurn, virtip, virtnet, virtlowecam PASS — identical to base; virtshell rc=1 identical to base (no initrd by design; the selftest line prints before the panic)
  implication: no aarch64 regression, including the note path (virtnotes) and the 2048-proc churn profile (+1 MB .data per FpState field — churn's .data was already 4.58 MB)

- timestamp: 2026-11-22T02:38:00Z
  checked: owner decision (A) and the landing — x64 sweep entry + nix/tier1-r9-x64.sh R9_EXPECT_OK fpprobe committed (Harvey 26910c78e); x64 tier-1 run once in a FRESH amd64 container (r9x64fp2: ubuntu:24.04, nix 2.36) on Harvey 26910c78e, r9 3f69729 (pushed), full default pipeline incl. the amd64 userland build, TCG ($SP/x64fp2/step1-summary.txt)
  found: `== expected on r9: 39 T1-OK markers + 0 T1-SKIP markers (computed)`, tier1-r9-x64 ALL STAGES PASS, 39 T1-OK incl. `T1-OK fpprobe` (`fpprobe: ok`), `fpu: kernel-SIMD selftest ok (... 0x1983c6d50a18d707c9d74d9a42c59f27)`, 648 s
  implication: the x64 marker is a live gate; the x64 tier-1 now needs r9 >= 3f69729

## Prevention

five_whys:
  - branch: "code — x86_64 FPU/SSE never saved per proc"
    why1: "14-08 recorded 'no per-proc FXSAVE on context switch' as an era limit — correct only while FP-using images never interleave"
    why2: "the x64 gates ran one FP user at a time (the tier-1 sweep is sequential; awk's float canary is a single process)"
    actionable: "port the arm64 reference design; pin it with an interleaving probe on the x64 gate"
  - branch: "code — note handlers shared the interrupted FP state"
    why1: "the note path was ported Ureg-first from the donor's notify/noted, without fpunotify/fpunoted (the donor's FP half lives in fpu.c)"
    actionable: "note_fpsave recorded at delivery, restored at NOTED; a probe handler that uses FP"
  - branch: "code/config — x86 notify alignment"
    why1: "sys_notify's `handler & 3` check was copied from aarch64, where instructions are word-aligned"
    why2: "no x86 gate binary installed a notify handler at an unaligned address that was then exercised by a note (rc's handler, by layout luck)"
    actionable: "drop the alignment test; host test with odd text addresses"
why_not_caught: "no gate existed for these classes — no probe interleaved two FPU users on x86, no probe used FP inside a note handler, and no x86 test registered a notify handler at an unaligned address; the x86 host tests also could not run on the darwin host (they need the amd64 container)"
recurrence_guard: "Harvey tier-1 `T1-OK fpprobe` on BOTH gates (aarch64: build/aarch64/tier1-sweep.rc + nix/tier1-r9.sh, now with the note leg; x64: build/x86_64/tier1-sweep.rc + nix/tier1-r9-x64.sh); r9 host tests aarch64/src/fpu.rs + x86_64/src/fpu.rs fpu::tests::* (ownership, kernel-SIMD guard, xor_fold16, target spec) and x86_64/src/syscall.rs tests::notify_accepts_a_handler_at_an_odd_text_address; the dev_exec boot selftest `fpu: kernel-SIMD selftest ok` on every exec-profile boot of both arches"

## Resolution

root_cause: three defects plus one found on the way. (a) x86_64: FPU/SSE state was never saved per proc — the registers stayed in place across switches (the 14-08 era limit), a fork child inherited whatever was live, and exec ran only fninit + ldmxcsr (the previous image's ST/XMM contents leaked). (b) both arches: note delivery left the handler on the interrupted code's live FP state and NOTED(NCONT/NRSTR) restored only the Ureg, so an FP-using handler returned its own registers to the interrupted code. (c) no way for kernel code to use SIMD safely (the kernels are FP-free by construction). (d) x86_64: sys_notify / deliver_notes demanded a 4-byte-aligned handler (an aarch64-port leftover) — x86 code has no alignment rule, so real handlers were refused and their process died on its first note
fix: r9 local main 2edfdf8 → 3f69729 (unpushed). d07fdbe (aarch64): fpu.rs LIVE_OWNER ownership (install/leave/discard/snapshot/replace; per-CPU state in a `percpu` module — thread-local in host tests), Proc.note_fpsave recorded at deliver_notes and restored by NOTED NCONT/NRSTR, the KernelSimd guard (kernel_begin/Drop/with_simd; begin parks the owner's live state in its fpsave, end restores it or zeroes unowned registers; IRQ-masked asserted; nested begin and in-section install/leave/snapshot/replace panic by name), xor_fold16 (inline asm under `.arch_extension simd`, takes &KernelSimd) + scalar reference, a dev_exec boot selftest. 4eafbfc (x86_64): the same design on the FXSAVE64 image (DEFAULT = FCW 0x037F, MXCSR 0x1F80, registers zero; IF=0 asserted; SSE2 xor_fold16), fp_reset_for_exec removed. d76d872 (x86_64): notify accepts handlers at any text address. 3f69729: boot-iso outputHash sha256-B2nbReb8… → sha256-x3KlbAktFO0pjynnz0GGrDUfIfFd/a/jnhL7H5b1h8Y=. The kernels stay built without FP; no port/src edits. Harvey: b77d134fc (arm64 fpprobe note leg), the amd64 fpprobe commit (stage-build + tier bake; no gate runs it)
verification:
  target_test:
    result: pass
    detail: "aarch64 fpprobe note leg red on 2edfdf8 (HVF+TCG identical, nbad=66) → ok on 3f69729; amd64 fpprobe (scratch x64 tier-1 fixture) red on 2edfdf8 (exec, switch, note) → ok on d76d872 (39 T1-OK); kernel-SIMD boot selftest ok on every aarch64 and x86_64 exec-profile boot (fold 0x1983c6d50a18d707c9d74d9a42c59f27 on both arches = the Python fold)"
  mutation_check:
    result: pass
    mutant_killed: "aarch64 8/8 (guard park, guard restore → named selftest panic; note snapshot, note replace → FAIL note; reschedule leave → FAIL switch preempt; pick_next install → FAIL fork exec switch preempt note; rfork snapshot → FAIL fork; exec replace → FAIL exec). x86_64 6/6 (note replace → FAIL note; reschedule leave → FAIL preempt; pick_next install → FAIL exec switch preempt note; rfork snapshot → FAIL fork; exec replace → FAIL exec; guard park → selftest panic). Host tests kill nested begin / in-section install/leave, the notify alignment revert, and a +neon or SSE-enabled target spec"
  no_op_deletion:
    result: pass
    deletion_justified_by_rca: "the removed lines are fp_reset_for_exec (replaced by fpu::replace with DEFAULT) and the x86 handler-alignment tests (the RCA's defect d)"
  adjacent_tests:
    result: pass
    suites_run: ["darwin: cargo xtask check (= 2edfdf8 set), clippy (= set), clippy --arch aarch64 (pre-existing rc=1, identical diagnostics), --arch x86-64 (= set), xtask test 237/0, aarch64 host tests 46/46", "container: x86_64 host tests 130/130", "aarch64 gates on 3f69729: tier-1 TCG (userland rebuilt) / smoke / net-smoke TCG + HVF, all PASS, tier-1 44 T1-OK", "r9 aarch64 boot tests: 10/10 PASS = base (virtshell no-initrd by design)", "x64 gates on d76d872 in the amd64 container: auth, net-smoke, tier-1 (committed wiring, rebuilt) ALL STAGES PASS", "builds aarch64 default/virtchurn/virtshell, x86-64 default/q35churnshell rc 0; rustfmt: new files clean, touched files at base hunk counts"]
  revert_and_reconfirm:
    result: pass
    bug_returned_on_revert: true — "2edfdf8 (no fix) reds the note leg on aarch64 and exec/switch/note on x86; every single-hook mutant reds its own leg"
    fixed_on_reapply: true — "3f69729 / d76d872 green on every leg, both arches"
  x86_64_bytes: "CHANGED (kernel-x64.elf e791e863… → 4b0cf307…, 28040560 B) — boot-iso FOD re-derived in the container (wired clone builds, --rebuild deterministic) and committed (3f69729); x64 auth/net-smoke/tier-1 run"
  guardrail_verdict: accepted (pending human verification; the x64 tier-1 marker awaits the owner)
files_changed: [r9 aarch64/src/{fpu.rs, proc.rs, exec.rs, trap.rs, syscall.rs, main.rs}, r9 x86_64/src/{fpu.rs (new), proc.rs, exec.rs, syscall.rs, main.rs, l.S (comment)}, r9 flake.nix (boot-iso hash), Harvey build/aarch64/fpprobe.c, Harvey build/x86_64/{fpprobe.c (new), fpprobeasm.s (new), stage-build.sh, mkimage.sh}]
proposal_x64_marker: "build/x86_64/tier1-sweep.rc: an `if(/bin/fpprobe) echo T1-OK fpprobe / if not echo T1-FAIL fpprobe` entry before T1-DONE (with the forkstress-style comment); nix/tier1-r9-x64.sh: R9_EXPECT_OK=\"$T1_OK float-d01 fpprobe\" (38 → 39 expected). Measured as the scratch fixture: PASS at d76d872 (39 T1-OK), T1-FAIL at 2edfdf8"
follow_ups:
  - "x86_64 notes are delivered only at the syscall boundary — an alarm or DEL cannot interrupt ring-3 code until its next syscall (trap.rs runs no note boundary on the interrupt-return path); interrupt-time delivery also needs NOTED to return through iretq (SYSRET clobbers rcx/r11 of an interrupted context)"
  - "NSAVE stays unimplemented on both arches (no FP impact today: only NCONT/NRSTR restore)"
  - "SMP: the per-CPU FP bookkeeping is a single static today (one CPU); a second CPU needs it per Mach"
