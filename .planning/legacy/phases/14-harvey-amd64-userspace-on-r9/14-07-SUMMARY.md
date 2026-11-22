---
phase: 14-harvey-amd64-userspace-on-r9
plan: 07
subsystem: compose-gate
tags: [iso-builder, flake, boot-r9-x64, smoke-r9, q35, kvm, x86_64, r9, xexec-03, d-05, d-06]
duration: 142 min
completed: 2026-11-09
requires:
  - "14-06-complete r9 HEAD 8aa9ba2 (the chain bake, the q35shell gate green twice — the compose consumes the chain image + the gate template)"
provides:
  - "r9's parameterized ISO builder (D-05): packages.iso-builder — the argv-driven r9-iso-builder script (KERNEL-ELF OUTDIR [MODULE-PATH MODULE-NAME ...]; the 13-06 FOD body factored verbatim) + the flake-lib bootIso function deriving through the SAME script; packages.boot-iso re-expressed kernel-only through it; kernel-x64 re-keyed to the q35shell config (the compose's kernel)"
  - "the boot-iso FOD hash RE-WIRED with the full proof discipline (the stale-since-14-01 deferred item closed): old body ≡ new builder over FOUR independent fresh kernel builds — file sha256 cdcfa008…, NAR 944aad86… (sha256-lEqthp7h…) — and `nix build .#boot-iso --rebuild` re-verifies on every rebuild"
  - "Harvey's boot-r9-x64 app: the one-command x64 compose (kernel contract → KERNDATE → namespace userland build → both bakes → r9's iso-builder → qemu-system-x86_64 -M q35 -accel kvm -cdrom with the always-on NIC, -nographic; NO DTB patch, NO qemu loader-device flag) — boots to the interactive `% ` prompt"
  - "the composed smoke gate (D-06): build/x86_64/smoke-r9.expect — the aarch64 stage vocabulary carried stage-for-stage, re-keyed to -cdrom/q35, PLUS the empty-line stage (the flagged assumption landed); nix/smoke-r9-x64.sh (capture hygiene, the R9_RUN_TAG survivor sweep) + the boot-r9-smoke-x64 app — GREEN TWICE CONSECUTIVELY, both captures retained"
  - "the sweep set in the chain image (ls/cat/echo/ps staged at amd64/bin where /bin binds) — the gate's ls/ps/cat assertions have real binaries; two bakes byte-identical (1b0196cc…)"
  - "TWO r9 kernel defects root-caused and fixed under the gate: (1) console RX never delivered — the 16550 RX ring + the ConsHooks registration (r9@8a338a6); (2) the SWAPGS witness bit leaked stale stack bytes on kernel-origin frames — a spurious return swap left the per-CPU GS pair at ring-3 orientation mid-kernel and the next SYSCALL entry faulted; fixed donor-style by cmpw-ing the saved CS word on both sides (r9@e5def72)"
affects:
  - "../r9/flake.nix, ../r9/x86_64/src/{uart16550.rs,main.rs,trap.rs}, ../r9/x86_64/src/platform/q35/timer.rs"
  - "flake.nix, flake.lock, nix/boot-r9-x64.sh, nix/smoke-r9-x64.sh, build/x86_64/{smoke-r9.expect,stage-build.sh,mkimage.sh}, .gitignore"
  - "14-08/14-09 (the tier gates and the net smoke compose through the same apps/bake)"
tech-stack:
  added: []
  patterns:
    - "FOD reproducibility proofs must compare like-for-like realizations: the mismatch report names the NAR hash (outputHashMode=recursive), not the file sha256 — a file-hash comparison against it 'differs' even when the bytes are identical (measured: three proof builds agreed at both layers once compared correctly)"
    - "a SWAPGS witness discipline must never live in stack bytes that only one entry arm writes: the pushed CS qword's upper bytes are NOT CPU-zeroed on all flows, so a kernel-origin frame inherits stale bits — the donor's return-side 16-bit CMPW of the saved CS word is immune by construction (selector bits the CPU itself pushed)"
    - "the per-CPU GS pair is a scheduler-wide invariant, not a per-context one: any spurious swapgs anywhere poisons every later kernel context until a gs-relative access faults — debug with an observation ring (rdmsr 0xC0000101 at scheduler/syscall sites), not print-based reasoning"
    - "plan9port mk stages a recipe's command as ._<name> when it cannot resolve on PATH inside a chroot — the staged dynamic binary then has no interpreter; link-only targets (mk 6.<cmd>) + outer-script cp avoid the whole class"
key-files:
  created:
    - nix/boot-r9-x64.sh
    - nix/smoke-r9-x64.sh
    - build/x86_64/smoke-r9.expect
  modified:
    - flake.nix
    - flake.lock
    - build/x86_64/stage-build.sh
    - build/x86_64/mkimage.sh
    - .gitignore
    - ../r9/flake.nix
    - ../r9/x86_64/src/uart16550.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/trap.rs
    - ../r9/x86_64/src/platform/q35/timer.rs
key-decisions:
  - "OQ-1 resolved BOTH-shapes-from-one-source: the argv-driven iso-builder script (packages.iso-builder, what Harvey's app consumes) and the flake-lib bootIso function BOTH derive from the same writeShellApplication text — ONE implementation, no forked wrapper logic (D-05)"
  - "kernel-x64 re-keyed to --config q35shell: the compose consumes this kernel to boot Harvey's chain (exec+bootchain devs required; plain q35 carries no user mode); the shipped artifact is the gated artifact — the kernel-only boot-iso hash rides the same bytes"
  - "the stale boot-iso hash re-wired per the 13-06 discipline, not casually: the refactor's byte-preservation was proven by running the OLD 13-06 FOD body and the new builder over independent kernel builds in both FOD and non-FOD realizations (identical bytes everywhere), then the fresh hash wired and --rebuild-verified"
  - "the stacksmoke stage maps to boot(8)'s version... print (the 14-06 x64 chain drops stacksmoke — no demand-paged stack this era; the stage vocabulary kept, the expected string re-keyed to the chain's actual output)"
  - "the flagged assumption LANDED as a stage: the empty-line echo stage (bare newline at the prompt → fresh % without losing the marker anchor) is now part of the x64 gate — the aarch64 template lacked it (recorded here for 14-EVIDENCE.md)"
  - "the r9 pin advanced THREE times, each its own lock-only commit after a push (fe39d83 the builder; 8a338a6 the RX fix; e5def72 the GS fix) — push-before-pin held throughout; one mixed staging attempt was caught by the plan's own verify gate and split before push"
requirements-completed: [XEXEC-03]
commits: 6
plan_head_before: 17896a72990220c48f683eba463dafe7b5ae7950   # harvey repo; r9 ledger 8aa9ba2
plan_head_after: 93236a5f38ae9d47cfc4b899bc46ba0e49662ebf   # harvey repo; r9 e5def72
actuals:
  tokens: 66000   # chars/4 over the realized diff (harvey 6 commits + r9 3 + docs)
  tasks: 3
  commits: 6      # harvey (f1e977a5c fd960ee5e d3a833f74 ff510457e 7189829d5 93236a5f3) + r9 (fe39d83 8a338a6 e5def72) + this docs commit
status: complete
coverage:
  - deliverable: "Task 1 — r9's parameterized ISO builder + boot-iso re-expression + the push (tracer)"
    verification:
      - kind: command
        ref: "nix build .#boot-iso (wired hash verified) && nix build .#boot-iso --rebuild && nix build .#kernel-x64 — all exit 0 on the committed tree (r9@fe39d83, shortRev-stamped derivations)"
        status: pass
      - kind: command
        ref: "the proofs: old 13-06 FOD body (HEAD worktree, q35shell-re-keyed, hash-removed/hashed variants) vs the new builder — FOUR independent kernel builds, identical ISOs at BOTH hash layers (file cdcfa008…, NAR sha256-lEqthp7h… = 944aad86…); iso-builder smoke: kernel-only emission byte-equal to Phase 13's staged cfg, module-bearing expands the @R9-MODULE@ placeholder in place (extracted from both ISOs and cmp'd)"
        status: pass
      - kind: command
        ref: "nix flake check --no-build (all checks passed); nix eval .#lib.bootIso --apply 'f: builtins.functionArgs f' → {kernel-elf, modules, outputHash, system}; grep -c modules flake.nix = 8 (≥2); git status clean; pushed cc29c7a..fe39d83 (origin/main..HEAD = 0)"
        status: pass
    human_judgment: false
  - deliverable: "Task 2 — Harvey's boot-r9-x64 app + the pin advance"
    verification:
      - kind: command
        ref: "the interactive smoke: nix run .#boot-r9-x64 </dev/null — full pipeline (userland build + bake + iso-builder + qemu -cdrom) reached the interactive `% ` prompt (capture /tmp/bootr9x64.log; the rc EOF/restart cycle after is the /dev/null-stdin artifact, the marker was reached)"
        status: pass
      - kind: command
        ref: "the pin discipline: THREE lock-only pin commits (fd960ee5e, ff510457e, 7189829d5) — `git show --name-only` on each touches only flake.lock (PIN-COMMIT-OK ×3); each pin names a PUSHED r9 rev; nix eval .#apps.x86_64-linux.boot-r9-x64.type = \"app\""
        status: pass
      - kind: command
        ref: "acceptance greps: boot-r9-x64×2 in flake; qemu-system-x86_64×2 + cdrom×3 in the script; forbidden patterns (loader-device/DTB-arg) = 0; git diff --stat HEAD -- nix/boot-r9.sh prints nothing (aarch64 untouched)"
        status: pass
    human_judgment: false
  - deliverable: "Task 3 — the composed smoke gate, green twice consecutively (XEXEC-03)"
    verification:
      - kind: command
        ref: "THE GATE: nix run .#boot-r9-smoke-x64 with R9_SMOKE_LOG=…run1.log then …run2.log — BOTH ALL STAGES PASS (22 stage-oks each; banner → paqfs → version → prompt → echo round-trip → empty-line → ls/ps/cat sweep → DEL safety → restart → pid-warning-absent → nic-bind); both captures retained (4730 bytes each)"
        status: pass
      - kind: command
        ref: "the determinism spot-check (T-14-12): both gate runs' composed ISOs byte-identical — r9-iso-builder printed sha256 2951bbb0… in BOTH runs (pinned kernel + identical KERNDATE bake); chain-bake double-bake byte-identical (1b0196cc…)"
        status: pass
      - kind: command
        ref: "the r9 regression battery at the fix commits: q35exec, q35shell, q35, q35net, aarch64 virt all PASS; 101 port tests; check/clippy clean; the TCG repro of the GS fault survives the typed echo round-trip (401 marker occurrences, no halt)"
        status: pass
      - kind: command
        ref: "acceptance greps: boot-r9-smoke-x64×2 in flake; nix/smoke-r9-x64.sh exists with R9_RUN_TAG×4; smoke-r9.expect: cdrom×3, forbidden (-dtb/loader-device) = 0, strip_echoed_line/two×7, R9_ISO×4; aarch64 gate + smoke-r9.sh untouched"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 7: The Composed One-Command Artifact — ISO Builder + boot-r9-x64 + the Smoke Gate Summary

Composed the x86_64 one-command artifact end-to-end: r9's flake now exports the parameterized ISO builder (the argv-driven r9-iso-builder script + a flake-lib function, both from one implementation — D-05), Harvey's boot-r9-x64 app drives userland-build → bake → builder → `qemu -cdrom` to the interactive `% ` prompt, and the composed expect gate — the aarch64 stage vocabulary carried verbatim plus the flagged empty-line stage — ran GREEN TWICE CONSECUTIVELY with both captures retained (XEXEC-03 complete). The gate did its job twice over: it surfaced two real r9 kernel defects no prior gate could see — console RX was never delivered on x86_64 (the 14-05 era registered no ConsHooks and the tick's poll echoed bytes into the void; rc's read errored and init restart-looped), and the SWAPGS witness-bit discipline leaked stale stack bytes on kernel-origin interrupt frames (a spurious return-swap left the per-CPU GS pair at ring-3 orientation mid-kernel; the next SYSCALL faulted at the Mach-scratch store) — both root-caused with the 14-06 playbook (a GS-observation ring; the flip pinned inside an idle-hlt tick's return) and fixed donor-faithfully, with the full Phase-13/14 regression battery green at each fix. The stale-since-14-01 boot-iso FOD hash was re-wired with the full two-derivation proof discipline, and the push-before-pin discipline held across all three pin advances.

**Duration:** 142 min (2026-11-09T06:05:44Z → 08:27:50Z)
**Tasks:** 3/3 complete
**Files:** 3 created + 5 modified (harvey) + 5 modified (r9)

## Commits

| Repo | Hash | Message |
|------|------|---------|
| r9 | fe39d83 | feat(14-07): the parameterized ISO builder — one implementation, two consumers (D-05) |
| harvey | f1e977a5c | feat(14-07): the boot-r9-x64 app — userland build → bake → r9's iso-builder → qemu -cdrom |
| harvey | fd960ee5e | chore(14-07): advance the r9 pin to fe39d83 (push-before-pin) |
| harvey | d3a833f74 | fix(14-07): the iso-builder contract is the binary path, not the package root |
| r9 | 8a338a6 | fix(14-07): console RX delivery — the 16550 RX ring + the ConsHooks registration |
| harvey | ff510457e | chore(14-07): advance the r9 pin to 8a338a6 (the console RX delivery fix) |
| r9 | e5def72 | fix(14-07): retire the SWAPGS witness bit — cmpw the saved CS word (the donor's discipline) |
| harvey | 7189829d5 | chore(14-07): advance the r9 pin to e5def72 (the SWAPGS witness-bit fix) |
| harvey | 93236a5f3 | feat(14-07): the composed smoke gate — smoke-r9.expect re-keyed + boot-r9-smoke-x64, green twice (XEXEC-03) |
| harvey | (docs) | docs(14-07): this file + state |

## Accomplishments

### Task 1: r9's parameterized ISO builder (tracer)

- **The builder** (flake.nix): `isoBuilderFor` wraps ONE `writeShellApplication` — `r9-iso-builder KERNEL-ELF OUTDIR [MODULE-PATH MODULE-NAME ...]` — the 13-06 FOD body factored verbatim (the pinned-date xorriso wrapper, the grub moddir shebang scrub, strip-all + the store-hash zero-zap, the templated grub.cfg emission). Kernel-only emission drops the @R9-MODULE@ lines (byte-identical to Phase 13's staged config — cmp-proven against extracted ISO contents); module-bearing emission expands the placeholder in place, one module2 line per staged module. Two consumers, one source: `packages.iso-builder` (the script, what Harvey's app calls) and `lib.bootIso` (the derivation former for pure-eval consumers); `packages.boot-iso` is `builder.bootIo` with the kernel-only args.
- **kernel-x64 re-keyed to q35shell** — the compose's kernel must carry exec+bootchain; the plain-q35 package could never boot the chain.
- **The hash re-wire with the full discipline**: the old FOD body (HEAD worktree) and the new builder were run over FOUR independent kernel builds in both FOD and non-FOD realizations — byte-identical everywhere (file sha256 cdcfa008…, NAR 944aad86…/lEqthp7h…). One measurement lesson recorded: the FOD mismatch report names the NAR hash (recursive mode), not the file sha — comparing across layers phantom-"differs".
- **Pushed r9** (cc29c7a..fe39d83 — the whole unpushed phase backlog went up push-before-pin).

### Task 2: Harvey's boot-r9-x64 app

- **nix/boot-r9-x64.sh**: boot-r9.sh's step structure re-keyed — step 0 asserts the kernel-x64 + iso-builder store-path contracts and derives KERNDATE; steps 1-2 run the namespace userland build + both bakes; step 3 calls r9's builder (the chain bake staged as /boot/bootfs.paq) and launches qemu `-M q35 -accel kvm -m 1024M -cdrom` with the always-on virtio-net NIC, `-nographic`. The accel chain carries (kvm default, loud TCG fallback, no silent downgrade). NO DTB patch, NO qemu loader-device flag anywhere.
- **flake.nix**: bootR9X64Pkg under `boot-r9-x64`, gated where r9 exports BOTH kernel-x64 and iso-builder (the hasAttr discipline re-keyed to the consumed attrs — dormant at the old pin, live at the new one). The aarch64 apps are byte-untouched.
- **The pin advanced as its own commit** — then TWICE more as the kernel fixes landed (each lock-only, each after a push; one mixed-staging attempt was caught by the plan's own verify gate and split before it could ship).
- **The interactive smoke reached `% `** through the full one-command pipeline.

### Task 3: the composed smoke gate — GREEN TWICE (XEXEC-03)

- **build/x86_64/smoke-r9.expect**: stage-for-stage from the aarch64 gate — env contract with exit-2, fresh-capture discipline, stage-named failures, the panic fast-fail (with the 14-06 real-chain syscall-10 exemption class), the two-occurrence echo round-trip, strip_echoed_line sweeps, DEL safety, init restart, and the whole-capture post-passes (pid-warning-absent, nic-bind presence — the x64 `#l0: virtio: 1000Mbps:` bind line is the identical shape, pci.rs). The stacksmoke slot re-keyed to boot(8)'s `version...` print. **The empty-line stage landed** (the flagged assumption): a bare newline at the prompt must return a fresh `% ` — now asserted, not assumed.
- **The sweep set joined the chain image**: ls/cat/echo/ps build per-target (link-only) and stage at amd64/bin (where `/bin` binds through the namespace); mkimage's manifest/magic asserts extended; two bakes byte-identical.
- **nix/smoke-r9-x64.sh**: the aarch64 driver re-keyed — same capture hygiene (rm -f before spawn; the driver re-removes), the unique R9_RUN_TAG in qemu's argv, the bracket-fixed survivor pkill (never a kernel-name class), exit 0/1.
- **THE GATE ran twice consecutively — ALL STAGES PASS both times**, captures retained at build/x86_64/smoke-r9-run{1,2}.log (22 stage-oks each). Both runs' composed ISOs were byte-identical (sha256 2951bbb0… — the T-14-12 spot-check).

## The two r9 kernel defects the gate surfaced (and the root-cause stories)

**1. Console RX was never delivered (r9@8a338a6).** The first typed interaction in r9-x86_64's history: rc's `#c/cons` read ERRORED instantly ("cons read path not initialized" — exec_bootstrap never registered devcons's ConsHooks; the 14-05 note in main.rs said so out loud), every rc exited "false" on its first read, and init restart-looped. Beneath that, the tick's polled sweep (`rx_echo`) echoed typed bytes to the wire and DISCARDED them — Phase 13's bring-up contract (the typed-probe gate needed only the echo). The fix re-keys aarch64's PL011 RX ring to the tick-polled 16550: a 256-byte single-writer/single-reader ring (the tick's sweep produces, the input engine's drain hook reads — both IF=0, never interleaving), the tick feeds the ring and wakes cons readers under dev_exec (the bring-up echo profile keeps rx_echo under not(dev_exec) — the Phase-13 gates' typed-probe assertion unchanged), and the ConsHooks registration (current/drain_rx/block/post_group/echo; the echo hook rides write_bytes from reader context, the INPUT→Console non-nesting rule).

**2. The SWAPGS witness bit leaked stale stack bytes (r9@e5def72).** With RX working, the gate's echo stage still died: `syscallentry`'s Mach-scratch store page-faulted — GSBASE wrong after the entry swapgs. Static analysis of every ring-transition path balanced on paper (each entry's swapgs pairs with its return's), so per the 14-06 playbook an observation ring went in: rdmsr(0xC0000101) at scheduler/syscall/tick sites, plus gs-relative canary stores at the asm return paths. The ring caught the flip in flagrante: a healthy idle-hlt tick (`gs=KMACH`) followed by a sched-loop iteration at `gs=0` — the corruption happened inside the idle tick's OWN interrupt return. The mechanism: r9's witness bit (CR-01's discipline) lived in the pushed CS qword's upper bytes — written (orb) ONLY on the from-user entry arm — so a kernel-origin frame inherited whatever stale bytes occupied that stack slot; on the Mach scheduler stack (whose top bytes were last dirtied by the boot context), a stale set bit made intrcommon's return-side btr take the swap-back arm — one spurious SWAPGS left the per-CPU GS pair at ring-3 orientation mid-kernel, poisoning every later kernel context until a gs-relative access faulted. On proc kstacks the slot's prior life was a previous frame whose own btr had cleared the bit — which is exactly why no earlier gate ever fired. The fix is the 9k donor's own discipline (l64idt.s's return-side `CMPW 48(SP)`): both entry and return now 16-bit-compare the SAVED CS WORD — only selector bits the CPU pushed, immune to stale upper bytes by construction. The witness consts are retired with a pointing comment.

Also en route (instrumentation finding, recorded for the follow-up): **rdgsbase #UDs under TCG** — CR4.FSGSBASE was never enabled (l.S documents the deliberate choice), so intrspecial's rdgsbase-based witness would fault if an NMI ever arrived; nothing executes it today (the latent finding is documented here, not fixed — NMIs are not in this phase's surface).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The app exported the iso-builder package ROOT, not the binary**
- **Found during:** Task 2's first interactive smoke
- **Issue:** `R9_ISO_BUILDER="${r9.packages...iso-builder}"` is a directory; the script's plain `-x` check accepts directories — the compose died "Is a directory" at step 3.
- **Fix:** export `<pkg>/bin/r9-iso-builder` and require an executable FILE.
- **Files modified:** flake.nix, nix/boot-r9-x64.sh
- **Commit:** harvey@d3a833f74

**2. [Rule 2 - Missing-critical] Console RX delivery absent on x86_64 (Task 3's gate)**
- **Found during:** the gate's first run (rc exited "false" on first read; init restart-looped)
- **Issue/Fix/Evidence:** see "The two r9 kernel defects" #1. The composed gate is x86_64's first console-READ consumer; the write-only marker era never exercised the read path.
- **Files modified:** ../r9/x86_64/src/{uart16550.rs,main.rs}, ../r9/x86_64/src/platform/q35/timer.rs
- **Commit:** r9@8a338a6

**3. [Rule 1 - Latent kernel defect] The SWAPGS witness bit's stale-byte leak (Task 3's gate)**
- **Found during:** the gate's second failure (the echo-response stage; kernel #PF at syscallentry's scratch store)
- **Issue/Fix/Evidence:** see "The two r9 kernel defects" #2 — the GS-observation ring pinned the flip inside an idle-hlt tick's return; the donor's CS-word cmpw closes the class.
- **Files modified:** ../r9/x86_64/src/trap.rs
- **Commit:** r9@e5def72

**4. [Rule 2 - Missing-critical] The sweep binaries were not in the chain image**
- **Found during:** Task 3 preparation (the aarch64 gate's ls/ps/cat sweep needs binaries; 14-06's chain image shipped only rc)
- **Fix:** the sweep set (ls/cat/echo/ps) builds per-target and stages at amd64/bin; manifest + magic asserts extended; determinism re-proven (two bakes byte-identical).
- **Files modified:** build/x86_64/{stage-build.sh,mkimage.sh}
- **Commit:** harvey@93236a5f3

**5. [Rule 3 - Venue] mkmany's $BIN recipe cp does not resolve inside the userns chroot; `mk clean` recurses into DIRS**
- **Found during:** Task 3's first stage-build run
- **Issue:** plan9port mk staged /bin/cp as `._cp` (a dynamic nix binary with no interpreter in the chroot) and the recipe died; the follow-up blanket `mk clean` recursed into every cmd subdir and deleted jpg's COMMITTED generated tables (rgbv.h/ycbcr.h — the Phase-6 workaround).
- **Fix:** link-only targets (`mk 6.<cmd>`) + the outer script's cp harvest; TARGETED droppings rm (never a blanket clean); the jpg tables restored from git (verified 0 deletions).
- **Files modified:** build/x86_64/stage-build.sh (+ git restore of the two tables)
- **Commit:** harvey@93236a5f3

**6. [Rule 3 - Process] One pin commit briefly bundled staged Task-3 files**
- **Found during:** the second pin advance — the plan's own verify gate (`the last commit touching flake.lock touches only flake.lock`) FAILED it
- **Fix:** soft-reset and re-commit by pathspec (`git commit flake.lock`); the discipline held (PIN-COMMIT-OK on all three pin commits).
- **Files modified:** none retained
- **Commit:** n/a (caught pre-push, split)

**7. [Rule 3 - Venue] Standalone stage-build invocations need PLAN9 + plan9port-on-PATH**
- **Found during:** Task 3's pipeline runs (the scaffold inherits the caller's env; the flake apps provide it)
- **Fix:** execution-environment provisioning only (exported PLAN9 + PATH for the standalone runs; no code change — the documented standalone contract).

**Total deviations:** 7 auto-fixed (1 missing-critical kernel feature, 1 latent kernel defect, 1 missing-critical image content, 2 blocking/venue, 1 process caught by the gate, 1 env). **Impact:** both kernel fixes are permanent improvements proven by the full regression battery (q35exec/q35shell/q35/q35net/virt/101 tests green at each fix commit); the plan's terminal truth (the gate green twice consecutively) is met.

## Authentication Gates

None.

## Known Stubs

None in the shipped code. Two recorded observations for the future: (a) intrspecial's rdgsbase witness would #UD if an NMI ever arrived (CR4.FSGSBASE deliberately unset — l.S documents why; no NMI path exists in these gates — the latent finding is documented in the SUMMARY, nothing retired); (b) `wrgsbase`/`write_gs_base`-family helpers remain dead code pending the FSGSBASE decision.

## Issues Encountered

None open — the two kernel defects above are fixed, regression-proven, and pushed.

## TDD Gate Compliance

N/A — no task carries tdd="true".

## Next Plan Readiness

READY for 14-08 (the tier gates compose through the same apps/bake — the sweep-set staging pattern extends) and 14-09 (the net smoke re-keys this gate's driver shape onto smoke-r9-net's fixture discipline; the nic-bind presence-only stage documented here is the venue-honesty line 14-09 owns).

## Self-Check: PASSED

- files exist: nix/{boot-r9-x64.sh,smoke-r9-x64.sh} + build/x86_64/smoke-r9.expect created; {flake.nix,flake.lock,stage-build.sh,mkimage.sh,.gitignore} modified in harvey@{f1e977a5c,fd960ee5e,d3a833f74,ff510457e,7189829d5,93236a5f3}
- r9 files exist: flake.nix @fe39d83; {uart16550.rs,main.rs,timer.rs} @8a338a6; trap.rs @e5def72
- commits are ancestors of HEAD: harvey ×6 ✓; r9 ×3 ✓ (both repos' working trees clean)
- plan-level verification green: Task 1's rebuild-proof + flake checks; Task 2's interactive boot to `% ` + three lock-only pin commits; Task 3's twice-green gate with both captures retained + the byte-identical composed-ISO spot-check (2951bbb0… both runs)
