---
phase: 14-harvey-amd64-userspace-on-r9
plan: 06
subsystem: boot-chain-gate
tags: [boot(8), bootfs, chain, mkfile.amd64, q35shell, x86_64, r9, xexec-03]
duration: 146+196+75 min
completed: 2026-11-09
requires:
  - "14-05-complete r9 HEAD ae4fb7c (the bake pipeline, the module delivery, exec_bootstrap with BOOT_ENTRY's bootchain arm pre-wired)"
provides:
  - "sys/src/9k/boot/mkfile.amd64 — boot(8) builds under 6c via the aarch64 template (A4 RESOLVED: bootr9.c compiles unchanged; the BOOTFILES closure is identical — embed in, paq out — linker-evidenced by the green 6l link)"
  - "build/x86_64/stage-build.sh's boot-chain section — chain libs (bio auth ip mp sec flate authsrv) + boot(8)/real-init/rc/paqfs harvested to out/ with 00008a97 asserted on each; build/x86_64/mkimage.sh's chain bake — bootfs-chain.paq with /boot/boot + /boot/paqfs + the REAL /amd64/init + /amd64/bin/rc, rcmain verbatim (cmp guard), termrc/namespace/profile, the INVERTED init-identity guard, factotum absence, KERNDATE determinism (two bakes byte-identical 8d2e2d08)"
  - "r9 config_q35shell.toml (exec+bootchain+ether+ip) + the xtask X64Shell gate (bind → module/paqfs → exec entry → boot(8)'s paqfs/version prints → init's rc handoff → the `% ` prompt; the syscall-10 real-chain exemption) — GREEN TWICE (four consecutive passes total)"
  - "FOUR latent kernel defects found and fixed by the chain gate, each invisible before the first multi-binary boot: (1) ktext was a CONFORMING code segment (0x9E) — ring-3 interrupt delivery stayed at CPL 3 and fault-looped to triple-fault; (2) the exec seed planted the Tos in bp (a 14-03 mis-reading) instead of AX (the donor's exec-return contract main9.s consumes) — boot(8)'s fused main9 prologue stored a zero _tos and malloc's plock fault-killed at addr 0x40; (3) the stack seed was two pages — boot(8) ran ~8.6 KiB deep (now 32 pages eager + the donor's Tos placement at USTKTOP-88, above the args); (4) brk growth relied on the absent pager (now mapped eagerly)"
  - "Harvey in-tree: embed.c drops the waitpid that deadlocked the paqfs handoff (a sleeping 4e bug — embed was never exercised on 4e/9k); ABI.md records the AX/Tos first-instruction contract"
  - "AUTHORIZED CYCLE (r9@c166164): fix #5 — reschedule parks via the scheduler context; fix #6 — the shared 4-GiB device-MMIO alias window (PML4[510], 0xFFFFFF00_80000000) + every pci/lapic/ioapic volatile deref re-keyed through it; documented: KZERO+pa overflows u64 for pa >= 2 GiB. AUTHORIZED CYCLE 3 (r9@a82d5a0): the wedge-autopsy dump survives corrupt proc fields"
  - "CYCLE 4 ROOT CAUSE + FIX (r9@8aa9ba2): sched_loop's Resume::Return arm (trap_return) diverged without saving a resumable scheduler state — the next blocked proc's swtch re-entered SCHED_CONTEXT at the LAST Switch-arm save, whose saved sp/return slot the scheduler's own continuation had already reused — wild execution, the deterministic post-init 'lying panic'. Fix: every resume (fresh forks included, via a first_entry trampoline seeded into the fork child's context with the frame pointer in rbx) rides the ONE swtch arm, re-saving SCHED_CONTEXT each resume — the donor Plan 9 scheduler discipline. The pipe queue-lifetime/qreopen suspicion ELIMINATED by the pid-keyed probe rings (no EOF/err/reopen in the whole run; paqfs never died — it was readied and never resumed)"
affects:
  - "../r9/x86_64/src/{seg,dat,exec,syscall,trap,main,proc}.rs, ../r9/x86_64/src/l.S, ../r9/x86_64/src/platform/q35/{pci,lapic,ioapic}.rs, ../r9/xtask/src/main.rs, ../r9/x86_64/lib/config_q35shell.toml"
  - "sys/src/9k/boot/{mkfile.amd64,embed.c}, build/x86_64/{stage-build.sh,mkimage.sh,root-tree/}, amd64/ABI.md"
  - "14-07 (the composed one-command artifact consumes the chain bake + the gate — unblocked)"
tech-stack:
  added: []
  patterns:
    - "the chain-gate debugging doctrine: qemu -d int,cpu_reset under TCG names the vector/state class (the ring-3 delivery storm at the IDT stubs with CS=gate-sel|3), the monitor-over-unix-socket dumps live statics, and binary disassembly of the 6.out (a.out parse + objdump) places the faulting instruction — all three were load-bearing this plan"
    - "6l fuses a main9-equivalent prologue (`mov %rax, _tos` + argc/argv reloaded from the stack) into the entry `main` even when _main is absent — kernels seeding a first process must plant the Tos pointer in AX (the exec-return register), never in an argument register"
    - "memory-side probe rings (pid-keyed plain volatile stores read back through the qemu monitor) are the ONLY trustworthy instrumentation when print/format machinery goes wild under corrupted control state — cycle 4's rings proved both the elimination (no pipe-queue misbehavior anywhere) and the kill (the scheduler ring's last events pinned the failure window exactly)"
    - "a scheduler loop that resumes procs must NEVER diverge from its own loop (trap_return/iretq) without re-saving its context: a diverging arm leaves the saved context stale against a stack the intervening iterations reuse — every resume (first entry included) must ride the one swtch arm (the donor Plan 9 discipline)"
key-files:
  created:
    - sys/src/9k/boot/mkfile.amd64
    - build/x86_64/root-tree/rc/bin/termrc
    - build/x86_64/root-tree/lib/namespace
    - build/x86_64/root-tree/usr/glenda/lib/profile
    - ../r9/x86_64/lib/config_q35shell.toml
  modified:
    - build/x86_64/stage-build.sh
    - build/x86_64/mkimage.sh
    - sys/src/9k/boot/embed.c
    - amd64/ABI.md
    - ../r9/x86_64/src/seg.rs
    - ../r9/x86_64/src/dat.rs
    - ../r9/x86_64/src/exec.rs
    - ../r9/x86_64/src/syscall.rs
    - ../r9/x86_64/src/trap.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/proc.rs
    - ../r9/xtask/src/main.rs
key-decisions:
  - "TWO IMAGES, not one: the chain bakes to out/bootfs-chain.paq while q35exec keeps out/bootfs.paq — the marker IS the leg-1 image's /amd64/init and the real init IS the chain image's; one image would break q35exec's regression. Chain binaries harvest to out/ (the boot chain's home, the aarch64 separation) — never smoke-out/, where the marker owns the init name"
  - "rcmain ships from rc/lib/rcmain (the aarch64 bake's cmp source — the rc-main that sets the '% ' prompt the gate asserts), NOT sys/src/9k/root/rcmain (the plan's read_first pointer is the kernel-tree reference; the operative source matches the shipped rc)"
  - "the x64 termrc drops stacksmoke (the aarch64 sibling's first line): it is the demand-paged-stack proof and x86_64 has no pager this era — it rejoins when the pager lands"
  - "the AX/Tos seed correction is an ABI fix, not a change: the donor's sysexecregs RETURNS the Tos (syscallreturn lands ar0.p in AX) and main9.s stores AX — 14-03's bp planting mis-read RARG (bp is 6c's FIRST-ARG register); ABI.md now records the contract"
  - "CYCLE 4: the named devpipe/qreopen suspicion was run live and ELIMINATED (the mandate's protocol) — the pid-keyed rings redirected to the scheduler resume path, where the stale-SCHED_CONTEXT root cause was proven and fixed (r9@8aa9ba2); the cycle-2/3 'lying panic' syndrome is fully explained by wild execution resuming a stale scheduler save"
requirements-completed: [XEXEC-03]
commits: 6
plan_head_before: 5ba1424c4f0dc3ec10f8c9f9a1a277b7fb995c6a   # harvey repo; r9 ledger ae4fb7c
plan_head_after: (this docs cycle)   # harvey repo; r9 ac7aa92 + c166164 + a82d5a0 + 8aa9ba2
actuals:
  tokens: 64000  # chars/4 over the realized diff across all cycles (harvey 3 code commits + r9 ac7aa92/c166164/8aa9ba2 + docs)
  tasks: 3       # all three (Task 3's gate green twice after the cycle-4 root-cause fix)
  commits: 5     # harvey (66cd0fc1b, a8cdb4c45, 86422fdea) + r9 (ac7aa92, c166164, a82d5a0, 8aa9ba2) + this docs commit
status: complete   # q35shell green twice consecutively (four consecutive passes) — XEXEC-03's r9-side leg closed
coverage:
  - deliverable: "Task 1 — mkfile.amd64 + the stage-build chain section (tracer)"
    verification:
      - kind: command
        ref: "direct mk in-namespace: boot 159356 bytes, magic 00008a97 (DIRECT-MK-VERIFY-OK; the plan's host-form re-keyed into the userns chroot — the 14-01..05 environment class); A4: bootr9.c compiled by 6c unchanged (git diff empty), the closure identical to aarch64's (green 6l link)"
        status: pass
      - kind: command
        ref: "sh build/x86_64/stage-build.sh green twice (final file state); /tmp/x64stage2.log magic-count 6 (2 marker + 4 chain binaries); acceptance greps: bootr9×3 (>=2), mkfile.amd64 in driver ×4 (>=1), bootr9.c untouched"
        status: pass
    human_judgment: false
  - deliverable: "Task 2 — the chain bootfs (mkimage chain bake + root-tree)"
    verification:
      - kind: command
        ref: "sh build/x86_64/mkimage.sh — bootfs.paq 37004 bytes + bootfs-chain.paq 238849 bytes, header 529ab12b / trailer 6b46e688; determinism: sha256 8d2e2d08… twice; the manifest log lists /boot/boot and /amd64/init"
        status: pass
      - kind: command
        ref: "acceptance greps: rcmain in mkimage ×9 (>=2), factotum ×7 (>=1), termrc exists; the chain bake's INVERTED identity guard (out/init IS the real init; the marker string ABSENT) green"
        status: pass
    human_judgment: false
  - deliverable: "Task 3 — config_q35shell + BOOT_ENTRY + the chain gate green twice"
    verification:
      - kind: command
        ref: "cargo xtask boot-test --arch x86-64 --config q35shell ×2 consecutive: PASS both (banner → facts → discovery → bind → module/paqfs → exec entry → paqfs/version → root-chain time → init's rc handoff → the interactive `% ` prompt, in order); two more passes on the committed tree = four consecutive greens total"
        status: pass
      - kind: command
        ref: "full regression at r9@8aa9ba2: q35exec PASS, q35 PASS, q35net PASS, aarch64 virt PASS, ureg-check + negative control PASS, cargo xtask check/clippy green, cargo xtask test 101 port tests pass (the host-bin leg is skipped in this rustup-less nix devshell — xtask's documented venue behavior)"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 6: The Boot Chain — boot(8)→init→rc to the Interactive Prompt Summary

Built the whole chain leg and drove it through the gate to GREEN: boot(8) compiles for amd64 (A4 resolved — arch-neutral, unchanged), the chain bake carries /boot/boot + paqfs + the real init + rc + rcmain verbatim with every guard green and byte-deterministic, and the q35shell gate reaches the interactive `% ` prompt — green twice consecutively (four consecutive passes on the final kernel). Getting there surfaced FOUR latent x86_64 defects no prior gate could see (a conforming ktext segment that broke ring-3 interrupt delivery, the Tos seeded in the wrong register, a two-page stack seed, lazy brk with no pager), one sleeping 4e bug in Harvey's embed.c (a waitpid that deadlocked the paqfs handoff), two more kernel defects (the preempt-park CR3 aliasing, the device-MMIO window), and — the last one, root-caused in cycle 4 with pid-keyed memory-side probe rings — a scheduler-resume design flaw: sched_loop's trap_return arm diverged without re-saving the scheduler context, so the next blocked proc's swtch re-entered a STALE save whose stack slot the intervening iterations had reused — wild execution, the deterministic post-init "lying panic". The fix (every resume rides the one swtch arm; fresh forks enter via a first_entry trampoline) closed it; the full regression battery is green.

**Duration:** 146+196+75 min (2026-11-08T22:59:30Z → 2026-11-09T05:10Z, three authorized cycles)
**Tasks:** 3/3 complete
**Files:** 8 harvey-side (4 created, 4 modified) + 9 r9-side (1 created, 8 modified)

## Commits

| Repo | Hash | Message |
|------|------|---------|
| harvey | 66cd0fc1b | feat(14-06): boot(8) builds for amd64 — mkfile.amd64 + the stage-build chain section |
| harvey | a8cdb4c45 | feat(14-06): the chain bootfs — bootfs-chain.paq baked with rcmain/termrc/namespace staged |
| r9 | ac7aa92 | feat(14-06): the q35shell chain profile + four chain-gate kernel fixes |
| harvey | 86422fdea | fix(14-06): embed.c drops the waitpid that deadlocks the paqfs handoff + ABI.md records the AX/Tos exec-return |
| r9 | c166164 | fix(14-06): preempt parks via the scheduler context + the shared device-MMIO alias window (authorized fix cycle) |
| r9 | a82d5a0 | fix(14-06): the wedge-autopsy dump survives corrupt proc fields (authorized cycle 3) |
| r9 | 8aa9ba2 | fix(14-06): every proc resume rides the swtch arm — the stale-SCHED_CONTEXT chain panic (cycle 4 root cause) |
| harvey | (docs) | docs(14-06): this file + state |

## Accomplishments

### Task 1: mkfile.amd64 — boot(8) under 6c (tracer)

- **mkfile.amd64** from the aarch64 template, comments re-keyed (the r9 boot has no kernel config here either — hand-written bootr9 bootconf; harvested to out/; printstub named on the link line; the autolib closure — libc/bio/auth/ip — documented for the stage driver). One real delta from the template text: my first draft carried a trailing `>` on the `</$objtype/mkfile` include (a display artifact of the read); the tree's mkfiles have none and plan9port mk treats the bracketed form as a missing file — caught on the first run, fixed, and re-verified.
- **A4 RESOLVED**: bootr9.c compiles under 6c byte-unchanged (git diff empty); the BOOTFILES closure is identical to aarch64's (embed in, paq out — the plan's "paq.$O stays" parenthetical was a misnomer for the embed member; linker evidence: the green 6l link with no missing externs and no needed additions). The plan's direct-mk verify leg re-keyed into the namespace (this host has no /amd64 at the root — the documented 14-01..05 class): DIRECT-MK-VERIFY-OK, boot 159356 bytes, magic 00008a97.
- **stage-build.sh's boot-chain section**: the chain libs first (bio, auth, ip, mp, sec, flate, authsrv — the autolib sets of boot/init/paqfs), then boot(8) via mk -f mkfile.amd64 (build → harvest → clean), the real init via cmd's per-target init.install (harvested from /amd64/init), rc, and paqfs per-target — all to out/ with the 00008a97 magic asserted and printed per binary. Chroot-venue fixes: /rc/bin on PATH (membername is an rc script) and tr in the /bin farm; explicit cleans for the cmd-tree droppings (the checkout IS the build tree here).

### Task 2: the chain bootfs

- **Two images by design**: mkimage.sh keeps the leg-1 marker bake (bootfs.paq for q35exec) and adds the chain bake (bootfs-chain.paq for q35shell) — one image would have broken q35exec's regression (the marker IS the leg-1 image's /amd64/init). The marker tree strips the new /rc+/usr skeleton subtrees so the 14-05 contract keeps its letter.
- **The chain manifest** (aarch64 re-keyed to /amd64): /boot/boot + /boot/paqfs, the REAL /amd64/init, /amd64/bin/rc, rcmain verbatim from rc/lib/rcmain (the cmp guard; the '% '-prompt rc-main matching the shipped rc), termrc/namespace/profile from the extended root-tree, /adm/timezone/local, the mount-point dirs; the INVERTED init-identity guard (real-init string PRESENT, smokesleep string ABSENT — the marker guard's mirror), factotum absence, KERNDATE determinism (two bakes byte-identical, sha256 8d2e2d08…), the od -j header/trailer checks. The x64 termrc drops stacksmoke (no pager this era — documented in the file).
- The bake log prints the staging manifest (the plan's log-grep verify: /boot/boot and rcmain both non-zero).

### Task 3: the q35shell profile + gate — GREEN TWICE (four consecutive passes)

- **config_q35shell.toml**: q35exec's shape with dev = ["exec", "bootchain", "ether", "ip"] (virtshell's membership; ether/ip per the Phase-13 placement — the gate asserts the bind half), module = the CHAIN bake (harvey:build/x86_64/out/bootfs-chain.paq).
- **xtask**: the X64Shell profile — Q35PciDiscovery → X64Bind (ether is on) → Q35ModulePaqfs → Q35ExecEntry → Q35ChainBoot (boot(8)'s own "paqfs..." + "version..." prints) → Q35ChainInit ("init: starting /bin/rc") → Q35ChainPrompt ("% "); order-anchored stages, the real-chain syscall-10 exemption (the Net-gate precedent), the finish-time whole-capture scan. main.rs's BOOT_ENTRY bootchain arm was already pre-wired by 14-05 (verified; comment refreshed to present tense).
- **The gate ran and did its job** — the chain's progress across the fix iterations: first ring-3 instruction (fault storm at the IDT stubs) → after the ktext fix: real user code, a fault at _tos → after the AX seed: paqfs.../version... → after embed.c: time... + **init: starting /bin/rc** → after the cycle-4 stale-SCHED_CONTEXT fix: **the interactive `% ` prompt**, green twice consecutively (plus two more passes re-run on the committed tree).

## The four kernel fixes (each gate-surfaced, each invisible before multi-binary boots)

1. **ktext was CONFORMING (seg.rs)**: `Descr::ktext64()` set `.with_conforming(true)` (access 0x9E). For a conforming gate target the interrupt delivery sets dpl:=cpl — the first RING-3-origin interrupt in r9 x86_64's history delivered its stub AT CPL 3 (no stack switch: the hardware frame pushed on the user stack, CS = ktext|3 = 0x0B) and fault-looped to triple-fault. Diagnosed from qemu TCG's -d int trace (the stub-fetch faults with CS=gate-sel|3, SP=user-0x30 = the exact 5-push frame) and confirmed against TCG's delivery logic. At CPL 0 (every prior gate) the bit is unobservable. Fixed to 0x9A (non-conforming, every real kernel's shape).
2. **The exec seed planted the Tos in bp, not ax (dat.rs)**: 14-03 read "bp=Tos via RARG" — but bp is 6c's FIRST-ARG register and the donor's contract is the exec RETURN: sysexecregs returns the Tos pointer, syscallreturn lands ar0.p in AX, and main9.s opens with `MOVQ AX, _tos(SB)`. 6l fuses that same prologue into `main` even when _main is absent (observed in boot's disassembly: `mov %rax, 0x400088` as the entry's second instruction) — so boot(8) stored AX=0 into _tos and malloc's `plock` (`pv->pid = _tos->pid`) fault-killed at addr 0x40 (the CR2 print added to trap.rs named it in one run). The exec syscall path was already correct (sys_exec returns exec_replace's tos_va through the AX writeback); only the first-process seed was wrong. Test expectations updated; ABI.md now records the contract.
3. **The stack seed was two pages (exec.rs)**: boot(8) faulted its stack at USTKTOP-0x21b8 (~8.6 KiB deep). Now 32 pages (128 KiB) map eagerly, and the Tos struct moved to the donor's placement — the very top of the seed page (USTKTOP-88, above the arg block) — so a downward-growing stack can never overwrite it (the old below-the-seed placement sat in the growth path).
4. **brk growth relied on the pager (syscall.rs)**: malloc's first sbrk faulted on the untouched growth page (the 14-04 no-pager era-limit, now hit by real binaries). sys_brk_'s private arm maps the grown range eagerly (same policy as map_text_eager; the pager remains the follow-up).

Plus the Harvey in-tree fix: **embed.c's `waitpid()`** — paqfs -i is a persistent server that never exits, so the 4e-original wait deadlocked boot(8) at the first fversion. The embed method was never exercised on 4e/9k (their boots use tcp/local) and k10t1's /boot/boot was an rc script (initcode's shell-script contract) — the bug slept until this chain. 9front's embedpaq.c has no wait; dropped with the provenance comment.

## The cycle-4 root cause and fix (r9@8aa9ba2) — the scheduler's diverging resume arm

The mandate's named probe (pid-keyed memory-side records in devpipe's write/drain arms) ran live, plus a second ring across the scheduler path (pick_next/park/wake/set_current_proc/curframe), both read back through the qemu monitor at the panic park. The evidence:

- **The pipe queue-lifetime suspicion (qreopen/qref dropping in-flight bytes under a stale chan) is ELIMINATED.** The full pid-keyed ring shows the complete healthy choreography — pipe(2) opens, embed's fork dup/close accounting exact, paqfs's rfork arms, 70+ whole 9P RPC exchanges (every write atomic and length-consistent, every drain matching), qref never reaching [0,0], and NOT ONE EOF, read-error, or reopen event in the entire run. paqfs never died: its server (pid 3) blocked waiting for the next request and was READIED by init's first-RPC wake — and never resumed. The earlier cycles' "read9pmsg short-BODY / dead server" reading was an artifact of the lying print machinery.
- **The scheduler ring pins the failure window exactly**: pid 4 (init, forked by boot) is picked for its FIRST entry via `Resume::Return` → `trap_return` (which diverges — it never returns); init runs its first RPC write (waking pid 3), blocks on the reply read; its `sched()` swtch'es into SCHED_CONTEXT — and the very next scheduler iteration dies. No probe between pid 4's block and the panic.
- **The mechanism**: `sched_loop`'s Return arm never re-saved the scheduler context (the swtch that saves it lives in the Switch arm only). After a Return-arm divergence, SCHED_CONTEXT still holds the LAST Switch-arm save — but the scheduler's own continuation (the iteration that picked the fresh fork) already popped and reused that save's sp/return-address slot (the next `pick_next` call lands its return address on exactly that slot). The next blocked proc's `swtch(..., SCHED_CONTEXT)` re-enters the loop at the stale save: the `ret` pops a reused slot, the match reads its discriminant from whatever RAX carried, and execution goes WILD with corrupted stack locals. Every prior mystery is explained by this one flaw: the different panic message per build layout ("misaligned 0x...5010" where 0x5010 IS 8-aligned; "add with overflow" at a provably inert expression), the self-contradictory register prints (cold-block-merged panic entries under a garbage register state), the truncated panic prints (the wild path landing inside the console lock's usage), cycle-2's "register-file values written onto the statics" and "divergent back-to-back loads", and the H2-class garbage parent pointer. The page-table walk at the park (PML4[511]→PDPT[510] 1-GiB page, identical across every proc table) re-confirmed H1/H3 stay eliminated.
- **The fix (the donor Plan 9 scheduler discipline)**: EVERY resume — a fresh fork's first entry included — rides the ONE swtch arm. The fork child's context is seeded to a `first_entry` trampoline (pc = the shim, sp = a kstack scratch never dereferenced, rbx = the planted Ureg pointer — the shim moves rbx to rdi and jumps to trap_return on the child's OWN kstack); `pick_next` returns the saved context for every pick; `Resume` (and its Return arm) is gone. Each resume re-saves SCHED_CONTEXT, so every `sched()` re-entry lands on a current save.

Instrumentation that decided it (all temporary, removed before the commit): the two memory-side probe rings, the pre-print panic stash (location + formatted message into static memory — which also PROVED the misattribution live: the stash's location agreed with the print while both named a merged cold-block entry), the parked-machine register/stack dumps symbolized against the ELF, and the page-table walk across proc CR3s.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] mkfile.amd64's include line carried a trailing `>`**
- **Found during:** Task 1's first build
- **Issue:** plan9port mk parsed `</$objtype/mkfile>` as a missing include file named `/amd64/mkfile>` — the build compiled bootr9.c with empty CC/CFLAGS. The tree's mkfiles use the bracket-less form.
- **Fix:** line 1 corrected to `</$objtype/mkfile`; re-run green end-to-end.
- **Files modified:** sys/src/9k/boot/mkfile.amd64
- **Commit:** harvey@66cd0fc1b

**2. [Rule 1 - Latent kernel defect] Conforming ktext broke ring-3 interrupt delivery (Task 3)**
- **Found during:** the first q35shell run (fault storm at the tick stub; qemu reset)
- **Issue/Fix/Evidence:** see "The four kernel fixes" #1. TCG's -d int trace + the delivery-source read pinned it; the fault arithmetic (CS=0x0B, SP=user-0x30, IPs at the stubs) matched the no-privilege-switch path exactly.
- **Files modified:** ../r9/x86_64/src/seg.rs
- **Commit:** r9@ac7aa92

**3. [Rule 1 - ABI bug] The exec seed's Tos rode bp instead of ax (Task 3)**
- **Found during:** the chain's fault at addr=0x40 (boot(8)'s _tos deref in malloc's plock)
- **Issue/Fix/Evidence:** see "The four kernel fixes" #2. The 6.out disassembly (the fused main9 prologue storing AX) was the decisive evidence; ABI.md corrected/extended.
- **Files modified:** ../r9/x86_64/src/{dat.rs,exec.rs,syscall.rs}, amd64/ABI.md
- **Commit:** r9@ac7aa92, harvey@86422fdea

**4. [Rule 2 - Missing-critical for the era] Two-page stack seed + lazy brk vs the no-pager reality (Task 3)**
- **Found during:** the chain's stack fault at USTKTOP-0x21b8, then the first malloc
- **Issue/Fix:** 32-page eager stack + the donor's Tos placement at USTKTOP-88; sys_brk_ maps growth eagerly. The 14-04 module docs record the pager as the follow-up.
- **Files modified:** ../r9/x86_64/src/{exec.rs,syscall.rs}
- **Commit:** r9@ac7aa92

**5. [Rule 1 - Sleeping upstream bug] embed.c's waitpid deadlock (Task 3)**
- **Found during:** the chain stalled between "paqfs..." and "version..."
- **Issue/Fix:** the 4e embed method waited for the never-exiting paqfs server. Dropped (the 9front shape), provenance commented in-tree.
- **Files modified:** sys/src/9k/boot/embed.c
- **Commit:** harvey@86422fdea

**6. [Rule 3 - Diagnostics] The user-trap print lacked the faulting address (Task 3)**
- **Found during:** the _tos hunt (pc alone could not distinguish NULL from a wild pointer)
- **Fix:** trap.rs's ring-3-origin exception print names CR2 (the donor faultnote's addr field).
- **Files modified:** ../r9/x86_64/src/trap.rs
- **Commit:** r9@ac7aa92

**7b. [Rule 1 - Latent kernel defect] reschedule's in-place CR3 switch aliased the kstack window (authorized cycle)**
- **Found during:** the authorized root-cause cycle (the "sched: switch 0->0" print reading clobbered locals was the giveaway)
- **Issue:** the old reschedule flipped CR3 INSIDE the PROC_TABLE lock while executing on the current proc's kstack at the FIXED window VA (PML4[400]) — after the switch the same VA aliased the NEXT proc's kstack pages, destroying every spilled local, the MCS LockNode, and the interrupt-return chain mid-flight.
- **Fix:** park-and-swtch — the preempted proc stashes its frame, marks Ready+blocked_in_kernel, and swtches into the scheduler context (the Mach stack, CR3-immune); pick_next drives the resume (the sched() discipline proven since 14-05's await). The park is now observed live: "sched: park N" precedes healthy continued execution.
- **Files modified:** ../r9/x86_64/src/proc.rs
- **Commit:** r9@c166164

**7c. [Rule 1 - Latent kernel defect] device MMIO unreachable under per-proc CR3s (authorized cycle)**
- **Found during:** the authorized cycle's TCG flight recorder: a kernel-mode WRITE fault at CR2=0xFEE00380 (the LAPIC page) in timer::tick, SP on the kstack window — under a per-proc CR3
- **Issue:** the device bases (ECAM 0xB0000000, virtio BARs 0xFE000000, IOAPIC 0xFEC00000, LAPIC 0xFEE00000) live in PA [1,4) GiB, covered ONLY by the boot table's PML4[0] identity window — per-proc tables carry no [0] (the user half) — every post-fork device access (the tick's xAPIC EOI, an ether IRQ's virtio queue work) faults or corrupts under the running proc's CR3.
- **Fix:** the 4-GiB device-MMIO alias at 0xFFFFFF00_80000000 inside PML4[510] (the slot every proc table shares) — l.S bootmachpt3 leaves at PDPT entries 2..5 + `proc::kaddr(pa)`; every pci/lapic/ioapic volatile deref re-keyed. TCG now boots the FULL chain past its old first-ring-3-entry divergence — direct proof the fault class existed and is closed.
- **Discovered en route (documented in l.S):** KZERO+pa OVERFLOWS u64 for every pa >= 2 GiB (KZERO sits 2 GiB below the top of the canonical space) — the KZERO alias can never host the device window; the first fix attempt used it and panicked at the ECAM base itself (checked_add → None), caught by the in-kernel bounds check before any wild write.
- **Files modified:** ../r9/x86_64/src/{l.S,proc.rs}, ../r9/x86_64/src/platform/q35/{pci,lapic,ioapic}.rs
- **Commit:** r9@c166164

**7d. [Rule 3 - Instrumentation] The panic format/location machinery prints stale registers**
- **Found during:** every probe of the authorized cycle
- **Issue:** misattributed panic locations (inlined cold blocks) and stale format args — "misaligned ... 0xffffffff806f5010" for a value that is 8-aligned, "attempt to add with overflow" at provably-inert expressions; only MEMORY-side records (the volatile store read back via the qemu monitor) are trustworthy.
- **Files modified:** none retained (probe code removed before the commit; the technique recorded here for the follow-up)
- **Commit:** n/a (method note)

**7. [Rule 3 - Venue] The chroot scaffold lacked membername's PATH/tools (Task 1)**
- **Fix:** /rc/bin on the chroot PATH + tr in the /bin symlink farm (membername is an rc script using tr+sed); cmd-tree droppings explicitly cleaned after harvest.
- **Files modified:** build/x86_64/stage-build.sh
- **Commit:** harvey@66cd0fc1b

**8. [Rule 3 - Verify re-key] The plan's host-form direct-mk cannot run on this host (the 14-01..05 class)**
- **Fix:** the verify leg ran inside the same userns chroot scaffold via a one-off harness (never committed): `mk -f mkfile.amd64` + `test -s boot` + the od magic — DIRECT-MK-VERIFY-OK.
- **Files modified:** none (execution-only)

**9. [Rule 1 - Latent kernel defect] sched_loop's Return arm left SCHED_CONTEXT stale (Task 3, cycle 4)**
- **Found during:** the mandate's pid-keyed probe cycle — the scheduler ring's last events (pid 4's first pick via Return, its first block) pinned the failure between the block's swtch and the next pick; the parked-machine stack symbolization named sched_loop's loop head as the panicking site
- **Issue/Fix/Evidence:** see "The cycle-4 root cause and fix". The named devpipe/qreopen suspicion was eliminated live first (the probe ring is the evidence); the actual flaw — the diverging trap_return arm — closed by making every resume ride the one swtch arm (fresh forks via the first_entry trampoline).
- **Files modified:** ../r9/x86_64/src/proc.rs
- **Commit:** r9@8aa9ba2

**Total deviations:** 12 auto-fixed across the four cycles (7 latent-defect/bug class, 1 missing-critical, 2 blocking, 2 venue/diagnostic). **Impact:** seven kernel/userland fixes are permanent improvements proven by the full regression battery; the plan's terminal truth (q35shell green twice) is met.

## Authentication Gates

None.

## Known Issues

**RESOLVED (cycle 4, r9@8aa9ba2)** — the scheduler-context corruption that killed the chain after "init: starting /bin/rc". The full story is in "The cycle-4 root cause and fix" above; the resolution summary:

- The named devpipe/qreopen suspicion was run live per the mandate and ELIMINATED (no queue misbehavior in the whole pid-keyed run — the probe ring is the proof).
- The REAL root cause — sched_loop's `Resume::Return` arm diverging via trap_return without re-saving the scheduler context, leaving SCHED_CONTEXT stale against a stack the intervening iterations reuse — was proven with the scheduler-path ring (the failure window pinned between pid 4's block and the next pick), the parked-machine register/stack symbolization (the panic entry reached only from sched_loop's loop head; the "misaligned" value self-contradictory), and the page-table walk (mapping divergence re-eliminated — H1/H3 stay dead).
- The fix (every resume rides the one swtch arm; fork children seeded to a first_entry trampoline) is committed, the gate is green twice consecutively (four passes total), and the full regression battery is green at r9@8aa9ba2.
- The previously-recorded H2 "garbage parent pointer" and the cycle-2 "register-file writes onto the statics" observations are explained as collateral of the same wild execution (no separate bug remains open).

The WINDOWS.md unmet-truth entry for 14-06 should be closed with this resolution.

## TDD Gate Compliance

N/A — no task carries tdd="true".

## Next Plan Readiness

READY for 14-07's composed one-command artifact: the chain bake (bootfs-chain.paq + its guards), the q35shell profile, and the green gate are all landed; 14-07 consumes the bake + the template conventions this plan proved, wraps the one-command artifact, and runs the Harvey-side interactive smoke that closes XEXEC-03's composed leg.

## Self-Check: PASSED

- harvey files exist: mkfile.amd64, root-tree/{rc/bin/termrc,lib/namespace,usr/glenda/lib/profile} created; {stage-build.sh,mkimage.sh,embed.c,ABI.md} modified in harvey@{66cd0fc1b,a8cdb4c45,86422fdea}
- r9 files exist: config_q35shell.toml created; {seg,dat,exec,syscall,trap,main}.rs + xtask/main.rs modified in r9@ac7aa92; {l.S,proc.rs,platform/q35/{pci,lapic,ioapic}.rs} modified in r9@c166164; proc.rs's cycle-4 fix (the first_entry trampoline + the one-swtch-arm sched loop) in r9@8aa9ba2
- commits are ancestors of HEAD: harvey 66cd0fc1b/a8cdb4c45/86422fdea ✓; r9 ac7aa92/c166164/a82d5a0/8aa9ba2 ✓
- plan-level verification: all three tasks' verifies green (Task 3's gate twice consecutively, four passes total; the full regression battery green at r9@8aa9ba2)
