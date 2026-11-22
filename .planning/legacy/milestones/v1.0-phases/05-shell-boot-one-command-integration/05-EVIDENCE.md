# Phase 05 Evidence Bundle — Shell Boot + One-Command Integration

**Phase:** 05-shell-boot-one-command-integration
**Consolidated:** 2026-11-03 (plan 05-13, task 3)
**Status:** complete — all five success criteria have execution evidence. The Core Value is delivered: `nix run .#boot-r9` boots r9 + the real Harvey aarch64 boot chain (boot(8) → init → rc -i) to an interactive `/bin/rc` shell on qemu virt, and `nix run .#boot-r9-smoke` proves it self-asserted, from clean checkouts, on macOS and Linux.

## Pinned commits (D-13 ordering: all r9 commits landed FIRST; this evidence commit records the live r9 HEAD)

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`cbbc8e41719493e151540b224990ec9ef273d607`** | r9 HEAD at evidence-commit time — 05-13's docs tour (HowItWorks namespace-core + flake-package sections; README virtshell/kernel-virt/R9_HARVEY_DIR rows). Verified: `git -C ../r9 log -1 --format=%H` at consolidation. |
| r9 | `9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6` | **the flake.lock pin** (INT-04/INT-03): r9 main's phase-code HEAD — all r9-side code work (05-01 + 05-04..05-10), pushed to origin 2026-11-03 (`33a9273..9549cee`, 80 commits). The `9549cee..cbbc8e4` delta is docs-only (HowItWorks.md + README.md, 32 insertions — verified `git diff --stat`): same kernel image, the 04-era docs-on-top precedent. |
| harvey | `dc82995758c24c52aca7feba019c0206a65315bb` | 05-13 Task 1: `.github/workflows/r9-shell.yml` (the D-13 CI leg) — the HEAD both clean-clone legs ran at |
| harvey | `cdc0ec775813bc88ad292b8f5b4750c156736c90` | 05-13 Task 2: REPRODUCIBILITY.md INT-03 section + the darwin clean-clone evidence log |
| harvey | `a57917193…` / `e97bc2245` | 05-12: the packaged smoke + the two-consecutive-run gate evidence |
| harvey | `862ea0403` / `73072d4f7` / `58d1b1077` | 05-11: the compose app, the compose script, the inputs.r9 pin |
| r9 | `83cbbbe` | 05-01: the D-14 note-race root-cause fix (see the record below) |

## Success-criterion → evidence mapping

### SC#1 — one command boots to the interactive `/bin/rc` prompt

`nix run .#boot-r9` (05-11, INT-01): userland build → mkpaqfs bake →
fdtput DTB patch → the flake-input kernel under qemu to the `% `
prompt. Evidence: `05-11-compose-gate.log` (committed; the darwin compose
gate), quoted verbatim with line numbers:

```
1727:== qemu argv: qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -m 1024M -accel tcg
      -kernel /nix/store/ylgcqwiadjgxiss1bv8ks9y6l32v01iq-r9-kernel-virt-9549cee/kernel-virt.elf
      -dtb /tmp/boot-r9.C2Q4ua/virt-patched.dtb
      -device loader,file=…/build/aarch64/bootfs.paq,addr=0x48000000 -nographic
1738:r9 from the Internet
2471:paqfs...
2543:init: starting /bin/rc
2609:stacksmoke: depth 64 ok, checksum 2088960
      …then the literal `% ` prompt (the gate's asserted terminal marker)
```

The argv line self-describes the composition: the kernel arrives ONLY as
the flake-input store path (`…-r9-kernel-virt-9549cee`), the image is
Harvey's own bake. Zero qemu orphans post-run (process-sweep verified).

The 05-08 interactive-gate narrative (drive-r9.expect, the smoke's
dev-harness ancestor): the prompt is a *conversation* — `echo r9shell7`
answers `r9shell7`; a partial command + DEL (0x7f) posts `interrupt` to
the console readers' note groups, fails the read `-1`/`interrupted`, and
rc re-prompts WITHOUT dying (the fresh `r9delok` answer proves the
partial line was discarded); `exit` triggers init's `for(;;)` restart —
`init: starting /bin/rc` → a fresh `% `. Evidence:
`05-08-drive-r9.log` (committed).

### SC#2 — rc builtins and external binaries execute correctly at the prompt

The D-09 command sweep, first driven by 05-09's extended drive-r9.expect
(`05-09-drive-r9.log`, committed) and packaged as the smoke in 05-12.
Quoted from `05-12-smoke.log` (committed; run 2 of the two-consecutive
gate — the hygiene-proving pass):

```
11:r9 from the Internet
816:init: starting /bin/rc
882:stacksmoke: depth 64 ok, checksum 2088960
936:r9shell7                      ← `echo` exact-string round-trip
       /bin/auth /bin/cat /bin/cp /bin/date /bin/echo /bin/grep /bin/ip
       /bin/ls /bin/mkdir /bin/mv /bin/ps /bin/rc /bin/rm /bin/sed
       /bin/stacksmoke /bin/test /bin/wc        ← `ls /bin` (subset present)
       glenda  1  … Blocked  init
       glenda  3  … Blocked  paqfs
       glenda  4  … Blocked  rc
       glenda  8  … Running  ps                  ← `ps` (devproc status/args)
1050:bind -c #e /env              ← `cat /lib/namespace` (the scoped file)
1070:r9delok                      ← DEL-safety answer
1075:init: starting /bin/rc       ← exit → init's restart loop
```

### SC#3 (as amended by D-13) — expect-style smoke green: CI = linux; darwin documented-and-run

D-13 amends the literal "CI green on both": the GH Actions leg is
linux-only; darwin is verified by the documented manual command.

**darwin leg (this host):** TWO CONSECUTIVE green
`nix run .#boot-r9-smoke` runs (05-12's literal gate), each the FULL
default pipeline (lima userland build → bake → DTB patch → expect
sweep). Run 2's capture is `05-12-smoke.log` above; both runs were
verified per-marker (the retained log carries every headline marker;
`init: starting /bin/rc` appears twice — boot + the exit-triggered
restart). Zero qemu orphans after both.

**linux leg (CI):** `.github/workflows/r9-shell.yml` (harvey `dc8299575`)
— clean checkout → DeterminateSystems nix-installer-action pinned to
commit SHA `3138316df39ed29be04236d7ffc686fa525866aa` (v23; T-05-24,
gate-enforced by the plan's verify) → `nix run .#boot-r9-smoke` on
ubuntu-latest, 90-minute budget, alongside the untouched legacy
harvey.yml.

**GH run status: GREEN — run 41825728636 (2026-11-04).**
`https://github.com/requaos/harvey/actions/runs/41825728636` —
conclusion: success, headSha `68c49ebcc` (the push carrying the G-05-1
fix `d758c2071`: nsrun.sh binds `$PLAN9/bin/rc` at `/bin/rc` so the
`#!/bin/rc` `membername` recipe resolves on a fresh host — the lima VM's
undocumented manual `/bin/rc` had masked the gap on every prior leg; see
`.planning/debug/DEBUG-G-05-1.md`). The first hosted run
(`41750071242`, failure) found the gap; the second run is green.
The native aarch64-linux substitute evidence below stands as the
pre-push proof of the same pipeline:

```
2:=== clone HEAD: dc82995758c24c52aca7feba019c0206a65315bb
3:=== host: Linux 6.8.0-147-generic aarch64
2076:SUBSET-BUILD-OK
2091:== PASS: baked …/r9-ci-linux/build/aarch64/bootfs.paq (1016045 bytes; header 529ab12b trailer 6b46e688)
3229:smoke-r9: ALL STAGES PASS — boot, echo, ls/ps/cat sweep, DEL safety, restart
3231:=== exit: 0
```

Independence from the darwin captures is structural: a different bake
(1016045 vs 1016222 bytes — KERNDATE-derived metadata) and a different
paqfs root-walk order. The input-wedge flake (05-09's documented top
debug candidate) did not fire in any 05-12/05-13 run (4/4 clean).

The one linux sub-case NOT yet executed anywhere: x86_64-linux
(ubuntu-latest's actual arch). 05-11 measured the boundary — r9's
kernel-virt embeds genuine IFD (importCargoLock reads a Cargo.lock
inside the built rust toolchain), so evaluating a foreign-system package
from darwin builds a foreign-system toolchain (platform mismatch);
aarch64-linux is proven natively (this leg), and x86_64-linux runs
natively for the first time on the GH runner at first push. Recorded
here so the gap is visible, never silently claimed.

### SC#4 — clean-checkout reproduction with pinned toolchains (INT-03)

`build/REPRODUCIBILITY.md`'s INT-03 section records: the policy
(process-level reproducibility; the Phase-1 KERNDATE policy applies to
the aarch64 userland build), the scope (pinned: r9 via flake.lock rev +
narHash, nixpkgs via flake.lock, r9's rust toolchain via
rust-toolchain.toml + rust-overlay, qemu/plan9port/expect via nixpkgs;
NOT claimed: bit-identical images — paqfs bytes carry KERNDATE-derived
metadata), the per-host derivation (the documented command sequence:
`git clone` → `nix run .#boot-r9-smoke`; darwin lima path, linux native
unshare path, the CI leg as the linux proof), and the overrides
(`--override-input r9` dev flow, `R9_SKIP_BUILD=1` iteration hatch —
both non-default).

The two clean-checkout runs evidencing it:

| Leg | Checkout | Host | Command | Result | Evidence |
|-----|----------|------|---------|--------|----------|
| darwin | pristine clone @ `dc8299575` | darwin-aarch64 (this host, lima build) | `nix run .#boot-r9-smoke` | rc=0, ALL STAGES PASS | `05-13-darwin-clean-clone.log` (committed) |
| linux | pristine clone @ `dc8299575` | aarch64-linux (lima guest, native unshare) | `nix run .#boot-r9-smoke` | rc=0, ALL STAGES PASS | `05-13-linux-clean-clone.log` (committed) |

### SC#5 — Harvey's flake consumes r9 as a flake input (INT-04)

`flake.lock` pins `inputs.r9 = github:requaos/r9` at rev
**`9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6`** (narHash
`sha256-g9OBd1Mwll2bx5bNjzxkuREB0X3H7uTPTyQ62PGrzQc=`), proven equal to
r9 main's live HEAD at pin time and resolvable from a clean clone (the
80 phase commits were pushed to origin BEFORE locking — 05-11). The
no-relative-paths gate (05-11, quoted from its SUMMARY):
`grep -vE '^\s*#' nix/boot-r9.sh | grep -c '../r9'` == 0 — zero
parent-relative sibling-checkout references in code lines; kernel/DTB
arrive only via the nix-controlled read-only `R9_KERNEL_PKG` store path
(T-05-21, presence-validated before launch).

## The D-14 note-race fix record (plan 05-01)

- **Root cause (one mechanism, two symptoms):** the shared `trap_return`
  tail restored x9 from the frame and then re-used x9 as scratch (the
  psr re-read + the EL0t CURKSTACK load) before `eret` — every
  EL0t-bound trap return resumed userland with x9 destroyed. A tick
  landing with x9 live produced the wild-write kills (smokefork storing
  through x9 = its own kernel-stack top) and the `note-timeout` exits
  (smokenote's resumed loop counter destroyed).
- **Fix:** the tail's final sp is uniform `frame + 288` for BOTH origins
  — every restored frame sits at entry-stack-top − 288 by construction,
  so no register is touched after the pops. Frozen TrapFrame layout
  untouched (ureg-check 35/35 green at the fix).
- **Fix SHA:** r9 `83cbbbe`.
- **Burial count:** **20/20 virtnotes + 20/20 virtmulti consecutive
  clean** (40/40), zero `note-timeout` lines — at the pre-fix ~1/3
  failure rate, 20 clean pairs hide the bug with probability < 0.03%.
  The shell path leans on notes at every DEL; the smoke's DEL-safety
  stage re-proves the fix under the real chain on every run.

## The 05-03 measured-set summary

The real boot chain (boot(8) → init → rc + the D-09 sweep binaries) was
traced on the 9front donor kernel at pin
`350266104f1fed81c78c3d05fa44f523bec0ae4d` (rig:
`build/aarch64/trace-syscalls.py --boot-chain`; born-traced arming via
`pidwait -w`). **26 distinct syscalls measured** (`05-03-trace.txt`,
committed):

```
 2 bind      3 chdir    4 close    5 dup      6 alarm    7 exec
 8 exits    10 fauth   14 open    17 sleep   19 rfork   21 pipe
22 create   23 fd2path 24 brk_    28 notify  35 unmount 39 seek
40 fversion 41 errstr  42 stat    43 fstat   46 mount   47 await
50 pread    51 pwrite
```

16 were NEW vs the Phase-4 set (bind, chdir, dup, alarm, fauth, sleep,
pipe, create, fd2path, brk_, unmount, seek, fversion, stat, fstat,
mount). Phase 5's dispatch table implements exactly this set plus the
named core; FAUTH(10) stays loud-None (D-02's designed fall-through —
the boot logs show `syscall 10 unimplemented` ×2 by design);
REMOVE/WSTAT/FWSTAT/OSEEK never measured → loud-None (the kernel's
syscall attack surface IS the measured set — T-05-18).

## Regression matrix (r9-side gates; all green at the pinned code HEAD `9549cee`, docs-only delta to `cbbc8e4`)

| Leg | Command | Result |
|-----|---------|--------|
| virt profile | `R9_HARVEY_DIR=<harvey> cargo xtask boot-test --arch aarch64 --config virt` | PASS |
| virtpanic profile | `… --config virtpanic` | PASS (register dump + clean halt) |
| virtexec profile | `… --config virtexec` | PASS (staged exec chain + stacksmoke) |
| virtmulti profile | `… --config virtmulti` | PASS (rfork/fd-ops smoke + preemption) |
| virtnotes profile | `… --config virtnotes` | PASS (notes end-to-end; 05-01's 40/40 burial on top) |
| build legs | `cargo xtask build --arch aarch64 --config {virtshell,default,raspi4b}` + `nix build .#kernel-virt` | green (4 legs) |
| ureg-check | `R9_HARVEY_DIR=<harvey> cargo xtask ureg-check` | 35/35 — the TrapFrame≡Ureg contract live on every boot-test |
| host suites | `cargo test -p port` (namespace/devenv/devmnt/devcons/devroot suites incl.) + `cargo test -p xtask` (19) | green |
| clippy | `cargo xtask clippy --arch aarch64 --config virtexec` | green (no new warnings) |

Harvey-side: the amd64 9k path is untouched all phase (additive,
objtype-gated edits only — the Phase-1 green is sacred); the aarch64
subset build's host-side asserts (16 binaries + boot/init/paqfs/
stacksmoke R_MAGIC) run on every compose/smoke invocation.

## Known-deferred inventory (milestone backlog — none goal-blocking)

- Phase-04 deferreds: **#3** (cold-pointer syscall aborts — the class was
  CLOSED in-phase by `fault::ensure_user_resident`, 05-06; the residual
  is the generalization audit) and **#4** (the contained exec_bootstrap
  heisenbug — closed by construction via IRQ masking; root cause open).
- 05-07's documented era limit: the pgrp-lock-across-blocking-walk
  hazard (the namec chain-snapshot fix lands with the first measured
  concurrent caller).
- The 05-09 input-wedge flake: documented-not-fixed, never reproduced in
  05-12/05-13 (4/4 clean) — the top debug candidate if the CI leg ever
  wedges at a prompt read.
- The stat(5) three-string latent shape in devcons/devroot/devenv/
  devsrv/devdup listings (no dirreadall consumer this era).
- 8 open Info findings (04-REVIEW-DISPOSITION.md) + phase-02 deferred #8
  (the 9k-amd64 pool-panic re-test) + the raspi3b-on-qemu-11.1.2
  upstream defect (human-accepted 2026-11-01).
- The GH-hosted first run of r9-shell.yml (this bundle's SC#3 gap) —
  lands at first push; the native aarch64-linux clean-checkout proof
  above stands in the interim.

---
*Phase: 05-shell-boot-one-command-integration*
*Consolidated: 2026-11-03*
