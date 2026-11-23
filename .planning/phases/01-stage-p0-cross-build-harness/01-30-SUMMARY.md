---
phase: 01-stage-p0-cross-build-harness
plan: 30
subsystem: cross-build-harness
tags: [aarch64, gates, expect, qemu, hvf, tcg, net-smoke, cpu, image-assembler, harvey-tree, flake-app, d-07, d-16, d-19, harn-06, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-17's harvey-tree-arm64 (built userland, D-19 fixtures + HARVEY-TREE stamp) and the 151-member bootfs manifest; 01-21's moved-gate rewiring pattern, gates/aarch64 layout and the three aarch64 apps; 01-08/01-11's image engine and cargo xtask image CLI"
provides:
  - "nix/gates/smoke-r9-net.sh and nix/gates/cpu-r9.sh: the moved aarch64 net-smoke and cpu gate scripts, reading only the read-only tree, r9's assembler image and R9_GATE_OUT"
  - "gates/aarch64/smoke-r9-net.expect and gates/aarch64/cpu-r9.expect: Harvey bd7a698 copies with path-token-only diffs"
  - "flake.nix: apps boot-r9-net-smoke and boot-r9-cpu on aarch64-darwin, aarch64-linux and x86_64-linux"
  - "Mac evidence: the full aarch64 quick pass (tier-1 HVF + TCG, smoke HVF, net-smoke HVF, cpu HVF) green from r9, with marker-count equivalence against the committed Harvey-hosted captures"
affects: [01-27, HARN-06, HARN-08, D-07, D-16, D-19]

actuals:
  tokens: 13784
  tasks: 2
  commits: 2

plan_head_before: 8d5eb8f0432c07e3942b1556d1817a0d32288e9f
plan_head_after: 5dd9cf5cdb3ceaab58ec6f104a0f6b4cd08f7022

tech-stack:
  added:
    - "r9 flake apps boot-r9-net-smoke and boot-r9-cpu (writeShellApplication via mkScriptApp)"
  patterns:
    - "The second aarch64 gate share follows 01-21 exactly: HARVEY=$R9_HARVEY_DIR_ARM64 + the HARVEY-TREE stamp, `cargo xtask image --arch aarch64 --image bootfs`, R9_GATE_OUT for the DTB copy and every capture, driver under gates/aarch64, app header with R9_KERNEL_PKG/R9_HARVEY_DIR_ARM64/R9_MKPAQFS"
    - "boot-r9-net-smoke takes kernel-virt; boot-r9-cpu takes kernel-virt-cpu (the devcap profile) through the same store-path contract; both ride bootR9RuntimeInputs (toolchain, bash, coreutils, gawk, gnugrep, gnused, findutils, qemu, dtc, git, expect), no lima, no hasAttr gating"

key-files:
  created:
    - nix/gates/smoke-r9-net.sh
    - nix/gates/cpu-r9.sh
    - gates/aarch64/smoke-r9-net.expect
    - gates/aarch64/cpu-r9.expect
  modified:
    - flake.nix

key-decisions:
  - "Both scripts keep Harvey's env seam (R9_KERNEL_PKG, R9_SMOKE_LOG / R9_CPU_LOG, R9_ACCEL/R9_GICV2, R9_BOOTARGS) and drop KERNDATE with 01-21: the tree is a store path with no .git and the image engine does not consume it."
  - "The net-smoke driver is unchanged beyond path tokens, including its load-bearing stage order (ether legs first, two orphan-free ether unbinds, loopback legs last) and the <=256-byte typed-line discipline; the script only supplies the shape and the capture path."
  - "The cpu gate keeps Harvey's always-written boot string (R9_BOOTARGS default init=/arm64/init -c on /chosen/bootargs), because the driver's bootconf stage asserts the exact line."
  - "Both apps run on all three systems via forAllSystems with no hasAttr gating; kernel-virt and kernel-virt-cpu are r9's own package attributes and resolve on every system."
  - "The quick pass ran from the worktree root with the apps built from the dirty working tree (the same bytes as the Task 2 commit); every gate compiled its wrapper through writeShellApplication's shellcheck checkPhase on the way."

patterns-established:
  - "A moved aarch64 gate keeps Harvey's assertions byte-identical; only path tokens change in the driver, and `diff -u <harvey copy> <r9 copy>` is the equivalence record"
  - "Equivalence against Harvey-hosted evidence compares the named stage list plus the marker counts in the committed capture, not just an exit code"

requirements-completed: [HARN-08, HARN-06]

coverage:
  - id: D1
    description: "The aarch64 net-smoke and cpu gate scripts live in r9 and read only the read-only arm64 tree, r9's assembled bootfs image and R9_GATE_OUT"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- nix/gates/smoke-r9-net.sh nix/gates/cpu-r9.sh (clean, rc 0)"
        status: pass
      - kind: other
        ref: "cat nix/gates/smoke-r9-net.sh nix/gates/cpu-r9.sh | grep -vE '^[[:space:]]*#' | grep -cE 'limactl|subset-build\\.sh|mkimage\\.sh|stage-build\\.sh|R9_SKIP_BUILD' (0)"
        status: pass
      - kind: other
        ref: "grep -c 'xtask image --arch aarch64 --image bootfs' and 'HARVEY-TREE' -> 3/3 in each script"
        status: pass
    human_judgment: false
  - id: D2
    description: "The net-smoke and cpu drivers are in r9 with path-token-only diffs against Harvey's copies; every assertion, marker, stage and timeout is unchanged"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "for f in smoke-r9-net cpu-r9: tclsh [info complete] -> 1 (DRIVERS-OK)"
        status: pass
      - kind: other
        ref: "diff -u harvey:build/aarch64/{smoke-r9-net,cpu-r9}.expect gates/aarch64/... -> path tokens only (recorded below)"
        status: pass
    human_judgment: false
  - id: D3
    description: "apps boot-r9-net-smoke and boot-r9-cpu are registered on all three systems and evaluate from r9's own supply"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix eval --raw .#apps.<system>.<app>.program for {aarch64-darwin,aarch64-linux,x86_64-linux} x {boot-r9-net-smoke,boot-r9-cpu} -> all OK; aarch64-darwin also passes the five-app loop (FIVE-APPS)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The full aarch64 quick pass is green from the moved apps on the Mac: tier-1 HVF and TCG, smoke, net-smoke and cpu under HVF (HARN-06)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "nix run .#boot-r9-tier1 && R9_ACCEL=tcg nix run .#boot-r9-tier1 && nix run .#boot-r9-smoke && nix run .#boot-r9-net-smoke && nix run .#boot-r9-cpu -> rc 0, AARCH64-QUICK-PASS (target/aarch64-quick-pass.log)"
        status: pass
      - kind: integration
        ref: "tier-1 HVF and TCG each print tier1-r9: ALL STAGES PASS; the capture carries 44 unique T1-OK + 23 unique T1-SKIP, 0 T1-FAIL"
        status: pass
      - kind: integration
        ref: "smoke-r9: ALL STAGES PASS with 24 named stage-ok lines; smoke-r9-net: ALL STAGES PASS with 41 stage-ok lines; cpu-r9: ALL STAGES PASS with 18 stage-ok lines and 17 forks"
        status: pass
    human_judgment: false
  - id: D5
    description: "The moved gates leave the arm64 Harvey tree byte-identical (HARN-02 after-gate proof) and both kernel baselines hold"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "p=$(nix eval --raw .#packages.aarch64-linux.harvey-tree-arm64.outPath); nix store verify --no-trust \"$p\" -> TREE-UNMODIFIED"
        status: pass
      - kind: other
        ref: "cargo xtask build --arch aarch64 --config virtshell -> sha256 76e6e153… (the 01-08 baseline); --config virtcpu -> sha256 10fd3320…"
        status: pass
    human_judgment: false
  - id: D6
    description: "Each moved gate's stage list and marker counts match the latest committed Harvey-hosted evidence (HARN-08 equivalence)"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "net-smoke: marker counts in target/gates/smoke-r9-net/smoke-r9-net.log equal the committed 08-09-net-smoke-gicv3-hvf.log (static ok 1, arp view 1, dhcp ok 1, random ok 1, ping ok 1, udp echo 1, tcp echo 2, NOT-L2-TRUTH 2, NET-SMOKE 2)"
        status: pass
      - kind: other
        ref: "cpu: the seven cpurc markers appear exactly once, 0 cpurc: FAIL, 17 forks — matching 20-EVIDENCE.md:2164-2165 and :2295-2296"
        status: pass
    human_judgment: false

duration: 6 min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 30: Move the aarch64 net-smoke and cpu gates into r9 and run the aarch64 quick pass Summary

**The last two aarch64 gates — the composed net smoke and the cpu fixture gate — now live in r9 as two scripts, two expect drivers and two flake apps, and the whole aarch64 quick pass is green from r9 on the Mac: tier-1 on HVF and TCG (44 T1-OK + 23 T1-SKIP, zero T1-FAIL), smoke, net-smoke (all five D-13 flows, marker counts equal to the committed 08-09 capture) and cpu (seven cpurc markers once each, 17 forks, zero cpurc FAIL) on HVF.**

## Performance

- **Duration:** 6 min (2026-11-23T12:12Z → 2026-11-23T12:18Z)
- **Tasks:** 2 of 2
- **Files modified:** 5 (4 created, 1 modified; 1,129 insertions, 0 deletions)
- **Plan estimate:** 55,000 tokens; **actual:** 13,784 tokens (chars/4 over 55,136 chars of realized diff). The wall time is dominated by the five QEMU boots and the image assemblies, which the token measure does not see.

## Accomplishments

- **The two gate scripts are in r9 and rewired onto r9's supply.** `nix/gates/{smoke-r9-net,cpu-r9}.sh` take `HARVEY=$R9_HARVEY_DIR_ARM64` and refuse a tree without the `HARVEY-TREE` stamp; the in-checkout `subset-build` + `mkimage` pair is replaced by `cargo xtask image --arch aarch64 --image bootfs` (writing `target/images/aarch64/bootfs.paq`); the DTB copy, the per-run scratch and every capture live under `R9_GATE_OUT` (`target/gates/<gate>`); `R9_SKIP_BUILD` and every lima/limactl reference are gone. shellcheck is clean (rc 0) and the forbidden-token count is 0.
- **The two drivers are in r9 with path-token-only diffs.** `gates/aarch64/smoke-r9-net.expect` and `gates/aarch64/cpu-r9.expect` are Harvey bd7a698 copies: the net-smoke driver keeps its load-bearing stage order (ether legs first, two orphan-free ether unbinds, loopback legs last), the NOT-L2-TRUTH venue labels, the typed-line discipline and every timeout; the cpu driver keeps its twelve stages and six whole-capture post-passes. Both pass `tclsh`'s completeness check.
- **The two apps are registered on all three systems.** `apps.{aarch64-darwin,aarch64-linux,x86_64-linux}.{boot-r9-net-smoke,boot-r9-cpu}` use `mkScriptApp`; each header exports `R9_KERNEL_PKG`, `R9_HARVEY_DIR_ARM64` and `R9_MKPAQFS`, with the Rust toolchain, qemu, dtc, git, expect and text tools on PATH (no lima, no `hasAttr` gating). net-smoke takes `kernel-virt`; cpu takes `kernel-virt-cpu`.
- **The full aarch64 quick pass is green from the moved apps.** One chain, exit 0, `AARCH64-QUICK-PASS`: tier-1 HVF, tier-1 TCG, smoke HVF, net-smoke HVF, cpu HVF. Every app wrapper also compiled through `writeShellApplication`'s shellcheck checkPhase on the way.
- **The after-gate proofs hold.** `nix store verify --no-trust` on `/nix/store/3dldw88190qd432b0yjv6jdp2wv92y0b-harvey-tree-arm64-0` prints `TREE-UNMODIFIED`; the locally built aarch64 virtshell kernel ELF sha256 is `76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2`, the 01-08 baseline, and the virtcpu ELF is `10fd33201b83e8b22cb0ba780a9a8ccb6a416195ea6e6a2c214723a06c1b012a`. Harvey is untouched (HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`), and zero `port/src` files changed.
- **D-12 is green before every commit:** `cargo xtask test` (14 suites, 0 failed), `clippy` and `check` exit 0; `cargo xtask gate --arch x86-64` on the Mac prints `GATE-VENUE not-linux` (rc 1 by design).

## Task Commits

Each task was committed atomically:

1. **Task 1: Copy and rewire the smoke-r9-net and cpu-r9 scripts, and copy their drivers with path-only changes** — `9d8b8e2` (feat)
2. **Task 2: Register boot-r9-net-smoke and boot-r9-cpu, and run the whole aarch64 quick pass on the Mac** — `5dd9cf5` (feat)

**Plan metadata:** the `docs(01-30): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/gates/smoke-r9-net.sh` (created) — the composed net smoke: tree + HARVEY-TREE contract, `cargo xtask image --arch aarch64 --image bootfs`, patched DTB under `R9_GATE_OUT`, `gates/aarch64/smoke-r9-net.expect`, the unchanged `r9netsmoke-` run tag and qemu shape.
- `nix/gates/cpu-r9.sh` (created) — the cpu gate: same pipeline, the always-written `R9_BOOTARGS` (default `init=/arm64/init -c`) on `/chosen/bootargs`, `gates/aarch64/cpu-r9.expect`, the unchanged `r9cpu-` tag.
- `gates/aarch64/smoke-r9-net.expect` (created) — Harvey's driver with path-token-only changes.
- `gates/aarch64/cpu-r9.expect` (created) — Harvey's driver with path-token-only changes.
- `flake.nix` (modified) — the two apps beside boot-r9/smoke/tier1 in the 01-21 block.

## Quick-pass evidence (this plan's HARN-06 legs)

The whole chain ran as the plan's verify command, from the r9 worktree root:

`nix run .#boot-r9-tier1 && R9_ACCEL=tcg nix run .#boot-r9-tier1 && nix run .#boot-r9-smoke && nix run .#boot-r9-net-smoke && nix run .#boot-r9-cpu && echo AARCH64-QUICK-PASS` → rc 0, `AARCH64-QUICK-PASS` (`target/aarch64-quick-pass.log`).

| Gate | Accel | rc | PASS line / counts | Capture |
|---|---|---|---|---|
| `nix run .#boot-r9-tier1` | HVF | 0 | `tier1-r9: ALL STAGES PASS — …`; `== expected on r9: 44 T1-OK markers + 23 T1-SKIP markers (computed)`; 44 unique T1-OK + 23 unique T1-SKIP in the capture; zero T1-FAIL; zero panic | `target/gates/tier1-r9/tier1-r9.log` (overwritten by the TCG run); run log `target/aarch64-quick-pass.log` |
| `R9_ACCEL=tcg nix run .#boot-r9-tier1` | TCG | 0 | the same PASS line and the same 44/23/0 set | same capture path; run log `target/aarch64-quick-pass.log` |
| `nix run .#boot-r9-smoke` | HVF | 0 | `smoke-r9: ALL STAGES PASS — …`; 24 named `stage ok` lines | `target/gates/smoke-r9/smoke-r9.log` |
| `nix run .#boot-r9-net-smoke` | HVF | 0 | `smoke-r9-net: ALL STAGES PASS — boot, ping 127.0.0.1, udp echo, tcp/7 echo, ipconfig static, ipconfig dhcp (both NOT-L2-TRUTH labeled), completion marker, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era`; 41 named `stage ok` lines | `target/gates/smoke-r9-net/smoke-r9-net.log` |
| `nix run .#boot-r9-cpu` | HVF | 0 | `cpu-r9: ALL STAGES PASS — banner, bootconf, paqfs, cpurc-ran, shebang, stat, cap, remove, ssl, cpurc-done, init-rc, prompt, no-cpurc-fail, no-panic, bootconf-present, marker-once, user-hooks-registered, no-unknown-device`; 18 named `stage ok` lines; 17 forks; zero `cpurc: FAIL` | `target/gates/cpu-r9/cpu-r9.log` |

The two tier-1 legs carry plan 01-21's equivalence finding: 44 T1-OK + 23 T1-SKIP / 0 T1-FAIL matches `.planning/legacy/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md:1858` and the stage line at `:738`. Smoke carries 01-21's finding: its stage list matches `20-VERIFICATION.md:37`.

## Driver equivalence record

The two diffs below are the equivalence record required by HARN-08. Each was produced with
`diff -u <(git -C /Users/neilskinner/code/harvey show bd7a6982a3e4f465f8ed3da5e0c7290521514ff0:build/aarch64/<name>.expect) gates/aarch64/<name>.expect`.

### `smoke-r9-net.expect` — path-token-only

```diff
@@ -4,7 +4,7 @@
 # script spawns qemu DIRECTLY with serial on stdio — the
 # build/build.bash heredoc-expect precedent; the caller is
-# nix/smoke-r9-net.sh = the flake app boot-r9-net-smoke):
+# nix/gates/smoke-r9-net.sh = the flake app boot-r9-net-smoke):
@@ -92,7 +92,7 @@
-# Hygiene: the caller (nix/smoke-r9-net.sh) rm -f's the capture BEFORE
+# Hygiene: the caller (nix/gates/smoke-r9-net.sh) rm -f's the capture BEFORE
@@ -126,18 +126,18 @@
-# Env (exported by nix/smoke-r9-net.sh — the flake app):
+# Env (exported by nix/gates/smoke-r9-net.sh — the flake app):
-#   R9_INITRD    REQUIRED — build/aarch64/bootfs.paq (Harvey's bake)
+#   R9_INITRD    REQUIRED — target/images/aarch64/bootfs.paq (r9's assembler)
-#          expect build/aarch64/smoke-r9-net.expect
+#          expect gates/aarch64/smoke-r9-net.expect
@@ -157,7 +157,7 @@
-# (nix/smoke-r9-net.sh) exports R9_RUN_TAG so its survivor sweep matches
+# (nix/gates/smoke-r9-net.sh) exports R9_RUN_TAG so its survivor sweep matches
```

### `cpu-r9.expect` — path-token-only

```diff
@@ -1,6 +1,6 @@
 # cpu-r9.expect — the arm64 cpu fixture gate driver (20-16, ARM-11),
-# in smoke-r9.expect's shape.  nix/cpu-r9.sh (the boot-r9-cpu flake
+# in smoke-r9.expect's shape.  nix/gates/cpu-r9.sh (the boot-r9-cpu flake
@@ -27,7 +27,7 @@
-# Env (exported by nix/cpu-r9.sh):
+# Env (exported by nix/gates/cpu-r9.sh):
```

No assertion, marker, stage or timeout line differs in either driver.

## Equivalence findings

- **net-smoke (HARN-08).** The moved run's stage list is Harvey's verbatim (banner → paqfs → stacksmoke → prompt → seam → static ipconfig → unbind → dhcp → unbind → loopback ping/udp → inline tcp/7 → completion marker → pid-warning-absent → nic-bind → user-hooks-registered → no-unknown-device → no-unsupported-era). The latest committed Harvey-hosted capture, `.planning/legacy/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-09-net-smoke-gicv3-hvf.log`, carries the identical marker counts: `netip: static ok` 1, `netip: arp view ok` 1, `netip: dhcp ok` 1, `netip: random ok` 1, `netip: ping ok` 1, `netip: udp echo ok` 1, `netip: tcp echo ok` 2, `NOT-L2-TRUTH` 2, `NET-SMOKE: ALL STAGES PASS` 2. The moved capture matches every count. `08-EVIDENCE.md:73/97-99` records the same stage set and PASS verdicts (gicv3+hvf, gicv2+tcg and at-the-pin packaged).
- **cpu (HARN-08).** The moved run's stage set is Harvey's verbatim (banner → bootconf → paqfs → cpurc-ran → shebang → stat → cap → remove → ssl → cpurc-done → init-rc → prompt, plus the six post-passes). `20-EVIDENCE.md:2164-2165` and `:2295-2296` record `cpu-r9: ALL STAGES PASS`, zero `cpurc: FAIL` and 17 forks on HVF and TCG; its transcript at `:2082-2120` names the same stages and markers. The moved run reproduces the seven markers exactly once, zero `cpurc: FAIL`, and 17 forks.
- **tier-1 and smoke.** Carried from 01-21: 44 T1-OK + 23 T1-SKIP / 0 T1-FAIL on both accelerators, and the smoke stage list unchanged (see the quick-pass table above).

## Verification commands (re-run)

- `nix run nixpkgs#shellcheck -- nix/gates/smoke-r9-net.sh nix/gates/cpu-r9.sh` → rc 0, no output.
- forbidden-token count (`limactl|subset-build\.sh|mkimage\.sh|stage-build\.sh|R9_SKIP_BUILD` outside comments) → **0**.
- `grep -c 'xtask image --arch aarch64 --image bootfs'` and `HARVEY-TREE` → 3/3 in each script.
- `tclsh [info complete]` for both drivers → 1 (`DRIVERS-OK`).
- `ls gates/aarch64/*.expect | wc -l` → **5**.
- `nix eval --raw .#apps.aarch64-darwin.<app>.program` for the five apps → paths (`FIVE-APPS`); `boot-r9-net-smoke` and `boot-r9-cpu` also evaluate on `aarch64-linux` and `x86_64-linux`.
- `nix store verify --no-trust <harvey-tree-arm64>` → `TREE-UNMODIFIED`.
- `sha256sum target/aarch64-unknown-none-elf/debug/aarch64` → `76e6e153…` (virtshell, the 01-08 baseline); `10fd3320…` (virtcpu).
- `cargo xtask test` → 14 suites, 0 failed; `clippy` → 0; `check` → 0; `cargo xtask gate --arch x86-64` → `GATE-VENUE not-linux` (rc 1 by design on the Mac).

## Decisions Made

- **Both scripts drop KERNDATE, as smoke/tier1 did.** The tree is a store path with no `.git`, and `cargo xtask image` does not consume the stamp.
- **The cpu gate keeps the always-written boot string.** `R9_BOOTARGS` defaults to `init=/arm64/init -c` and is written to `/chosen/bootargs`; the driver's `bootconf` stage asserts that exact line, so the behaviour is part of the gate, not a caller option to remove.
- **The net-smoke driver's stage order stays untouched.** It sidesteps the measured multi-waiter RX-wake defect and the two-core wildcard demux; the script only supplies the image, DTB and shape.
- **The apps take `kernel-virt` (net-smoke) and `kernel-virt-cpu` (cpu).** The cpu profile carries devcap; the same store contract (kernel-virt.elf + both DTBs) serves both, so the app definitions differ only in the package they export.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **The tier-1 serial capture is overwritten per run** (the HVF and TCG legs share `target/gates/tier1-r9/tier1-r9.log`), so the capture on disk is the TCG run; both runs' PASS lines and counts are in `target/aarch64-quick-pass.log`. This is the same capture-path behaviour 01-21 recorded.
- **`nix run` built the apps from the dirty working tree** (flake.nix plus the two new scripts before their commit). The bytes are identical to the committed tree, and every wrapper passed writeShellApplication's shellcheck checkPhase.

## Known Stubs

None. Every moved script reads a real tree file or r9's assembler output; every driver's env is supplied by its app; no placeholder or unverified path remains.

## Threat Flags

None beyond the plan's threat model. T-01-94 is implemented by writeShellApplication's shellcheck gate (every app built) plus the explicit shellcheck run, with quoted expansions in the rewritten scripts; T-01-95 by every output living under `R9_GATE_OUT` and `nix store verify --no-trust` printing `TREE-UNMODIFIED`; T-01-96 by keeping Harvey's slirp netdev stanza unchanged (`user,id=n0`, no hostfwd); T-01-SC by no package installs.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **HARN-08's aarch64 gate move is complete:** five scripts, five drivers and five apps across plans 01-21 and 01-30, all reading the read-only tree and r9's assembled image.
- **HARN-06's aarch64 quick pass is Mac-verified from r9**, so plan 01-27 can rely on it before Harvey's copies are deleted (D-10).
- **`cargo xtask gate --full` now has every aarch64 app it needs;** the remaining wait is the x86_64 apps (01-23/24/31).
- **Harvey remains untouched** (HEAD `25d97dc4d…`, status hash `041d2929…`), both kernel baselines hold, and the arm64 tree is store-verified after the quick pass.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/gates/{smoke-r9-net,cpu-r9}.sh`, `gates/aarch64/{smoke-r9-net,cpu-r9}.expect` — all FOUND; `flake.nix` modified.
- Task commits `9d8b8e2` (Task 1) and `5dd9cf5` (Task 2) are ancestors of HEAD; `git rev-list --count 8d5eb8f..HEAD` = 2 (measured against the plan ledger).
- Task 1 re-ran: shellcheck rc 0; forbidden-token count 0; `xtask image --arch aarch64 --image bootfs` and `HARVEY-TREE` present 3/3 in both; `DRIVERS-OK`; both driver diffs path-token-only and recorded above.
- Task 2 re-ran: `FIVE-APPS`; the two new apps evaluate on all three systems; the quick pass rc 0 with `AARCH64-QUICK-PASS`; tier-1 44/23/0 on HVF and TCG; smoke 24 stage-ok; net-smoke 41 stage-ok with marker counts equal to the 08-09 capture; cpu 18 stage-ok, 17 forks, 0 cpurc FAIL; `TREE-UNMODIFIED`; virtshell kernel sha `76e6e153…` and virtcpu `10fd3320…`.
- Plan verification: D-12 test/clippy/check 0/0/0 and `GATE-VENUE` printed; Harvey HEAD `25d97dc4d…` and status hash `041d2929…`; zero `port/src` changes; no push.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
