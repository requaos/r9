---
phase: 01-stage-p0-cross-build-harness
plan: 21
subsystem: cross-build-harness
tags: [aarch64, gates, expect, qemu, hvf, tcg, boot-r9, smoke, tier1, image-assembler, harvey-tree, flake-app, d-07, d-09, d-16, d-19, harn-06, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-17's harvey-tree-arm64 (the built userland and the D-19 source-side fixtures) and the 151-member bootfs manifest; 01-18's mkScriptApp helper, the moved-gate rewiring pattern and the xtask gate seam; 01-08/01-11's image engine and cargo xtask image CLI"
provides:
  - "nix/gates/boot-r9.sh, nix/gates/smoke-r9.sh, nix/gates/tier1-r9.sh: the moved aarch64 boot, smoke and tier-1 gate scripts, reading only the read-only tree, r9's assembler image and R9_GATE_OUT"
  - "gates/aarch64/{drive-r9,smoke-r9,tier1-r9}.expect: the moved drivers (smoke/tier1 path-only; drive-r9 the unreferenced interactive driver rewired onto nix/gates/boot-r9.sh)"
  - "flake.nix: apps boot-r9, boot-r9-smoke and boot-r9-tier1 on aarch64-darwin, aarch64-linux and x86_64-linux"
  - "Mac evidence: tier-1 PASS on HVF and TCG (44 T1-OK + 23 T1-SKIP, zero T1-FAIL) and smoke PASS on HVF from the moved apps"
affects: [01-22, 01-30, HARN-06, HARN-08, D-07, D-16, D-19]

actuals:
  tokens: 19481
  tasks: 3
  commits: 3

plan_head_before: ed91b673fac8f44720e5bf96ddcd857068b74002
plan_head_after: b93d9abd25df11e157b832a97fca7d77ad3fe1cb

tech-stack:
  added:
    - "r9 flake apps boot-r9, boot-r9-smoke and boot-r9-tier1 (writeShellApplication via the copied mkScriptApp helper)"
  patterns:
    - "A moved aarch64 gate keeps Harvey's env-contract header but HARVEY becomes $R9_HARVEY_DIR_ARM64 with the HARVEY-TREE stamp; the build+bake pair becomes `cargo xtask image --arch aarch64 --image bootfs`; the DTB copy and every capture go under R9_GATE_OUT"
    - "The app header exports R9_KERNEL_PKG, R9_HARVEY_DIR_ARM64 and R9_MKPAQFS; the Rust toolchain, qemu, dtc, expect and git ride runtimeInputs, with no lima and no hasAttr gating"
    - "On the Mac the app's arm64 tree is the aarch64-linux package (the devshell's treeSystem rule), so `nix run` realises the same store tree the devshell exports"
    - "Smoke/tier1 drivers are copied with comment-path-only diffs; their env contracts, spawn argv, assertions, markers, stages and timeouts are byte-identical to Harvey's"

key-files:
  created:
    - nix/gates/boot-r9.sh
    - nix/gates/smoke-r9.sh
    - nix/gates/tier1-r9.sh
    - gates/aarch64/drive-r9.expect
    - gates/aarch64/smoke-r9.expect
    - gates/aarch64/tier1-r9.expect
  modified:
    - flake.nix

key-decisions:
  - "HARVEY is R9_HARVEY_DIR_ARM64 with the HARVEY-TREE stamp; the tree is a store path, so the checkout-derived KERNDATE block is gone and `cargo xtask image --arch aarch64 --image bootfs` supplies the image (the engine does not consume KERNDATE; D-29 compares member lists)."
  - "R9_GATE_OUT defaults per gate (target/gates/boot-r9, /smoke-r9, /tier1-r9) and holds the patched DTB, the per-run scratch dir and the capture; R9_SKIP_BUILD is deleted."
  - "The flake apps resolve the arm64 tree through the devshell's treeSystem rule (aarch64-linux on the Mac), so one app definition serves all three systems and the Mac realises the registered builder's tree."
  - "drive-r9 is unreferenced by every gate script. Harvey's run-r9.sh launcher is not part of the move and is absent from the read-only tree, so drive-r9 now spawns nix/gates/boot-r9.sh and takes R9_KERNEL_PKG; every assertion, marker, stage and timeout is unchanged."
  - "Smoke and tier1 keep their driver-spawned qemu shape (the driver owns the spawn/teardown); only boot-r9 launches qemu from the script, as Harvey's did."
  - "The tier-1 expected set is computed from the pinned tree (tier1-cmds.txt minus the never-swept/SKIP members plus the nine fixed probes), which is 44 T1-OK + 23 T1-SKIP at bd7a698."
  - "git stays in runtimeInputs as the plan directed even though the store tree has no .git; the tool is inert now that KERNDATE is gone."

patterns-established:
  - "Moved aarch64 gate: HARVEY=$R9_HARVEY_DIR_ARM64 + HARVEY-TREE, image via `cargo xtask image --arch aarch64 --image bootfs`, outputs under R9_GATE_OUT, driver under gates/aarch64"
  - "Flake app header: R9_KERNEL_PKG / R9_HARVEY_DIR_ARM64 / R9_MKPAQFS exports plus the toolchain and qemu/dtc/expect/git on PATH; no lima, no hasAttr gating"
  - "The arm64 tree in an app is `self.packages.${treeSystem}.harvey-tree-arm64`, the same lazy-tree rule the devshell uses"

requirements-completed: [HARN-08, HARN-06]

coverage:
  - id: D1
    description: "The aarch64 boot, smoke and tier-1 gate scripts live in r9 and read only the read-only arm64 tree, r9's assembled bootfs image and R9_GATE_OUT"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- nix/gates/boot-r9.sh nix/gates/smoke-r9.sh nix/gates/tier1-r9.sh (clean, rc 0)"
        status: pass
      - kind: other
        ref: "cat nix/gates/{boot-r9,smoke-r9,tier1-r9}.sh | grep -vE '^[[:space:]]*#' | grep -cE 'limactl|subset-build\\.sh|mkimage\\.sh|stage-build\\.sh|R9_SKIP_BUILD' (0)"
        status: pass
      - kind: other
        ref: "grep -l 'xtask image --arch aarch64 --image bootfs' nix/gates/*.sh -> all three; grep -c HARVEY-TREE -> 3 in each"
        status: pass
    human_judgment: false
  - id: D2
    description: "The smoke and tier1 drivers are in r9 with path-only diffs against Harvey's copies; every assertion, marker, stage and timeout is unchanged"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "for f in drive-r9 smoke-r9 tier1-r9: tclsh [info complete] -> 1 (DRIVERS-OK)"
        status: pass
      - kind: other
        ref: "diff -u harvey:build/aarch64/{smoke-r9,tier1-r9}.expect gates/aarch64/... -> comment path tokens only (recorded below)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The unreferenced interactive drive-r9 driver is moved; it spawns r9's own boot gate and its assertions are unchanged"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "tclsh [info complete] -> 1; diff hunks recorded below"
        status: pass
    human_judgment: true
    rationale: "drive-r9's diff carries two support lines beyond path/env tokens (the R9_GATE_OUT capture expression and its mkdir) because no caller script creates the capture directory; a reviewer must accept that every assertion/marker/stage/timeout is unchanged (recorded as a Rule 2 deviation)."
  - id: D4
    description: "apps boot-r9, boot-r9-smoke and boot-r9-tier1 are registered on all three systems and evaluate/build from r9's own supply"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "for a in boot-r9 boot-r9-smoke boot-r9-tier1: nix eval --raw .#apps.aarch64-darwin.$a.program -> store path (THREE-APPS); the same three evaluate on x86_64-linux; the three writeShellApplication wrappers build (BOOT-R9-APP-BUILT, APPS built by nix run)"
        status: pass
    human_judgment: false
  - id: D5
    description: "tier-1 passes from the moved app on HVF and on TCG on the Mac (HARN-06's first two aarch64 legs)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "nix run .#boot-r9-tier1 -> rc 0, tier1-r9: ALL STAGES PASS, 44 T1-OK + 23 T1-SKIP, zero T1-FAIL (target/gate-tier1-hvf.log)"
        status: pass
      - kind: integration
        ref: "R9_ACCEL=tcg nix run .#boot-r9-tier1 -> rc 0, the same PASS line and 44/23/0 (target/gate-tier1-tcg.log)"
        status: pass
    human_judgment: false
  - id: D6
    description: "smoke passes from the moved app under HVF on the Mac (HARN-06's third aarch64 leg)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "nix run .#boot-r9-smoke -> rc 0, smoke-r9: ALL STAGES PASS, 24 named stage-ok lines (target/gate-smoke-hvf.log)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The moved gates leave the arm64 Harvey tree byte-identical (HARN-02 after-gate proof)"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "p=$(nix eval --raw .#packages.aarch64-linux.harvey-tree-arm64.outPath); nix store verify --no-trust \"$p\" -> TREE-UNMODIFIED"
        status: pass
    human_judgment: false
  - id: D8
    description: "The moved drivers/run match the latest committed Harvey-hosted evidence where one exists (HARN-08 equivalence)"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "tier-1: 44 OK / 23 SKIP / 0 FAIL matches .planning/legacy/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md:1858 and the stage line at :738; smoke: the stage list matches 20-VERIFICATION.md:37's hooks/no-unknown-device/no-unsupported-era list; driver diffs recorded below"
        status: pass
    human_judgment: false

duration: 5 min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 21: Move the aarch64 boot, smoke and tier-1 gates into r9 Summary

**The aarch64 boot, smoke and tier-1 gates now live in r9 — three scripts, three expect drivers and three flake apps that read only the read-only arm64 tree and r9's assembled bootfs image — and from those apps tier-1 passes on HVF and TCG (44 T1-OK + 23 T1-SKIP, zero T1-FAIL) and smoke passes on HVF.**

## Performance

- **Duration:** 5 min (2026-11-23T11:58:25Z → 2026-11-23T12:03:45Z)
- **Tasks:** 3 of 3
- **Files modified:** 7 (6 created, 1 modified; 1600 insertions, 0 deletions)
- **Plan estimate:** 60,000 tokens; **actual:** 19,481 tokens (chars/4 over 77,925 chars of realized diff). The wall time went to the kernel-virt build and the three QEMU boots (HVF tier-1, TCG tier-1, HVF smoke), which the token measure does not see.

## Accomplishments

- **The three aarch64 gate scripts are in r9 and rewired onto r9's supply.** `nix/gates/{boot-r9,smoke-r9,tier1-r9}.sh` take `HARVEY=$R9_HARVEY_DIR_ARM64` and refuse a tree without the `HARVEY-TREE` stamp; the in-checkout `subset-build` + `mkimage` pair is replaced by `cargo xtask image --arch aarch64 --image bootfs` (writing `target/images/aarch64/bootfs.paq`); the DTB copy, the per-run scratch and every capture live under `R9_GATE_OUT` (`target/gates/<gate>`); `R9_SKIP_BUILD` and every namespace-transport reference are gone. shellcheck is clean (rc 0) and the forbidden-token count is 0.
- **The three drivers are in r9.** `gates/aarch64/{smoke-r9,tier1-r9}.expect` are comment-path-only copies of Harvey's drivers — the env contracts, qemu spawn argv, assertions, markers, stages and timeouts are byte-identical. `drive-r9.expect`, the unreferenced interactive driver, now spawns `nix/gates/boot-r9.sh` and takes `R9_KERNEL_PKG`, with its capture under `R9_GATE_OUT`; its assertions are unchanged. All three pass `tclsh`'s completeness check.
- **The three apps are registered on all three systems.** `apps.{aarch64-darwin,aarch64-linux,x86_64-linux}.{boot-r9,boot-r9-smoke,boot-r9-tier1}` use `mkScriptApp`; each header exports `R9_KERNEL_PKG`, `R9_HARVEY_DIR_ARM64` and `R9_MKPAQFS`, and runtimeInputs carry the Rust toolchain, qemu, dtc, git, expect and the text tools (no lima, no `hasAttr` gating). On the Mac the tree is the aarch64-linux package through the devshell's `treeSystem` rule.
- **tier-1 is green from the moved app on both accelerators.** HVF and TCG each exit 0 with `tier1-r9: ALL STAGES PASS — boot, one typed sweep line, float canary on r9, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era`, `44 T1-OK markers + 23 T1-SKIP markers (computed)`, zero `T1-FAIL`, zero panic.
- **smoke is green from the moved app under HVF.** Exit 0 with `smoke-r9: ALL STAGES PASS — boot, echo (typed + answered), ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era` (24 named `stage ok` lines).
- **The after-gate proofs hold.** `nix store verify --no-trust` on `/nix/store/3dldw88190qd432b0yjv6jdp2wv92y0b-harvey-tree-arm64-0` prints `TREE-UNMODIFIED`; the locally built aarch64 virtshell kernel ELF sha256 is `76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2`, the 01-08 baseline; Harvey is untouched (HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`), and zero `port/src` files changed.
- **D-12 is green before every commit:** `cargo xtask test` (87-plan/168+ suite, 0 failed), `clippy` and `check` exit 0; `cargo xtask gate --arch x86-64` on the Mac prints `GATE-VENUE not-linux` (rc 1 by design).

## Task Commits

Each task was committed atomically:

1. **Task 1: Copy and rewire the boot-r9, smoke-r9 and tier1-r9 gate scripts** — `4942690` (feat)
2. **Task 2: Copy the drive-r9, smoke-r9 and tier1-r9 drivers with path-only changes** — `1367fe8` (feat)
3. **Task 3: Register boot-r9, boot-r9-smoke and boot-r9-tier1, and run tier-1 and smoke on the Mac** — `b93d9ab` (feat)

**Plan metadata:** the `docs(01-21): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/gates/boot-r9.sh` (created) — the plain aarch64 boot: tree + HARVEY-TREE contract, `cargo xtask image --arch aarch64 --image bootfs`, patched DTB under `R9_GATE_OUT`, the unchanged qemu argv/accel selection.
- `nix/gates/smoke-r9.sh` (created) — the packaged smoke: same image pipeline, `gates/aarch64/smoke-r9.expect`, `R9_GATE_OUT` capture, run-tag sweep.
- `nix/gates/tier1-r9.sh` (created) — the tier-1 gate: same pipeline, the computed marker sets from the tree's `tier1-cmds.txt`/`tier1-sweep.rc`, `gates/aarch64/tier1-r9.expect`.
- `gates/aarch64/drive-r9.expect` (created) — the unreferenced interactive driver; spawns `nix/gates/boot-r9.sh`, takes `R9_KERNEL_PKG`, capture under `R9_GATE_OUT`.
- `gates/aarch64/smoke-r9.expect` (created) — Harvey's driver with comment-path-only changes.
- `gates/aarch64/tier1-r9.expect` (created) — Harvey's driver with comment-path-only changes.
- `flake.nix` (modified) — the three apps, `treeSystem`/`harveyTreeArm64` and `bootR9RuntimeInputs` in the apps `let`.

## Quick-pass evidence (this plan's HARN-06 legs)

| Gate | Accel | rc | PASS line / counts | Capture |
|---|---|---|---|---|
| `nix run .#boot-r9-tier1` | HVF | 0 | `tier1-r9: ALL STAGES PASS — …`; `== expected on r9: 44 T1-OK markers + 23 T1-SKIP markers (computed)`; zero T1-FAIL; zero panic | `target/gates/tier1-r9/tier1-r9.log`; run log `target/gate-tier1-hvf.log` |
| `R9_ACCEL=tcg nix run .#boot-r9-tier1` | TCG | 0 | the same PASS line and the same 44/23/0 set | `target/gates/tier1-r9/tier1-r9.log` (overwritten); run log `target/gate-tier1-tcg.log` |
| `nix run .#boot-r9-smoke` | HVF | 0 | `smoke-r9: ALL STAGES PASS — …`; 24 named `stage ok` lines (banner → no-unsupported-era) | `target/gates/smoke-r9/smoke-r9.log`; run log `target/gate-smoke-hvf.log` |
| `nix run .#boot-r9` | — | — | interactive, no PASS marker; checked by evaluation (`nix eval` THREE-APPS) and its wrapper builds (`BOOT-R9-APP-BUILT`) | — |

## Driver equivalence record

The three diffs below are the equivalence record required by HARN-08. Each was produced with
`diff -u <(git -C /Users/neilskinner/code/harvey show bd7a6982a3e4f465f8ed3da5e0c7290521514ff0:build/aarch64/<name>.expect) gates/aarch64/<name>.expect`.

### `smoke-r9.expect` — path-token-only

```diff
@@ -23,7 +23,7 @@
-# Hygiene: the caller (nix/smoke-r9.sh) rm -f's the
+# Hygiene: the caller (nix/gates/smoke-r9.sh) rm -f's the
@@ -43,18 +43,18 @@
-# Env (exported by nix/smoke-r9.sh — the flake app):
+# Env (exported by nix/gates/smoke-r9.sh — the flake app):
-#   R9_INITRD    REQUIRED — build/aarch64/bootfs.paq (Harvey's bake)
+#   R9_INITRD    REQUIRED — target/images/aarch64/bootfs.paq (r9's assembler)
 # usage: env R9_KERNEL=… R9_DTB=… R9_INITRD=… R9_SMOKE_LOG=… \
-#          expect build/aarch64/smoke-r9.expect
+#          expect gates/aarch64/smoke-r9.expect
@@ -74,7 +74,7 @@
-# (nix/smoke-r9.sh) exports R9_RUN_TAG so its survivor sweep matches
+# (nix/gates/smoke-r9.sh) exports R9_RUN_TAG so its survivor sweep matches
```

### `tier1-r9.expect` — path-token-only

```diff
@@ -23,7 +23,7 @@
-# Hygiene: the caller (nix/tier1-r9.sh) rm -f's the capture BEFORE
+# Hygiene: the caller (nix/gates/tier1-r9.sh) rm -f's the capture BEFORE
@@ -36,13 +36,13 @@
-# Env (exported by nix/tier1-r9.sh — the flake app):
+# Env (exported by nix/gates/tier1-r9.sh — the flake app):
-#   R9_INITRD     REQUIRED — build/aarch64/bootfs.paq (Harvey's bake)
+#   R9_INITRD     REQUIRED — target/images/aarch64/bootfs.paq (r9's assembler)
@@ -53,7 +53,7 @@
-#          R9_EXPECT_OK=… R9_EXPECT_SKIP=… expect build/aarch64/tier1-r9.expect
+#          R9_EXPECT_OK=… R9_EXPECT_SKIP=… expect gates/aarch64/tier1-r9.expect
@@ -73,7 +73,7 @@
-# (nix/tier1-r9.sh) exports R9_RUN_TAG so its survivor sweep matches
+# (nix/gates/tier1-r9.sh) exports R9_RUN_TAG so its survivor sweep matches
```

### `drive-r9.expect` — path/env rewire, assertions unchanged

```diff
@@ -14,7 +14,7 @@
-# run-r9.sh's process GROUP (qemu is in it on darwin, which lacks
+# nix/gates/boot-r9.sh's process GROUP (qemu is in it on darwin, which lacks
@@ -32,11 +32,11 @@
-# Env: R9_KERNEL (the virtshell r9 ELF) and R9_DTB (the virt DTB) are
-# REQUIRED (run-r9.sh's contract — no implicit sibling-checkout paths);
-# R9_INITRD optional (run-r9.sh bakes it via mkimage.sh when absent).
+# Env: R9_KERNEL_PKG (the r9 kernel-virt package's store path) is
+# REQUIRED (nix/gates/boot-r9.sh's contract — no implicit
+# sibling-checkout paths); the gate assembles the bootfs image itself.
-# usage: R9_KERNEL=… R9_DTB=… expect build/aarch64/drive-r9.expect
+# usage: R9_KERNEL_PKG=… expect gates/aarch64/drive-r9.expect
@@ -47,26 +47,28 @@
-foreach v {R9_KERNEL R9_DTB} {
+foreach v {R9_KERNEL_PKG} {
-        puts stderr "drive-r9: \$$v unset — run-r9.sh requires it (no defaults by design)"
+        puts stderr "drive-r9: \$$v unset — nix/gates/boot-r9.sh requires it (no defaults by design)"
-# The capture is RETAINED at build/aarch64/drive-r9.log (gitignored).
-set CAPTURE build/aarch64/drive-r9.log
+# The capture is RETAINED under R9_GATE_OUT (default
+# target/gates/boot-r9/drive-r9.log), the gate's run scratch dir.
+set CAPTURE [expr {[info exists env(R9_GATE_OUT)] ? "$env(R9_GATE_OUT)/drive-r9.log" : "target/gates/boot-r9/drive-r9.log"}]
+exec mkdir -p [file dirname $CAPTURE]
 exec rm -f $CAPTURE
-# A per-run unique tag flows into qemu's argv via run-r9.sh's
+# A per-run unique tag flows into qemu's argv via nix/gates/boot-r9.sh's
-spawn sh build/aarch64/run-r9.sh
+spawn sh nix/gates/boot-r9.sh
```

No assertion, marker, stage or timeout line differs in any driver.

## Equivalence findings

- **tier-1 (HARN-08).** The moved run's stage list is Harvey's verbatim (`banner` → `paqfs` → `stacksmoke` → `prompt` → `float-canary` → `coreutils-sort` → `digest-sum` → `writeflow-skip` → `sweep-done` → the four post-passes) and its computed set is **44 T1-OK + 23 T1-SKIP, zero T1-FAIL** on both HVF and TCG. The latest committed arm64 Harvey-hosted capture that records the computed set — `.planning/legacy/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md:1858` — reports **44 OK / 23 SKIP / 0 FAIL** with `fpprobe: ok`; its stage line is at `20-EVIDENCE.md:738`. The counts match. One recorded difference: the 20-08 iteration in the same file (`:898`) reports 39 T1-OK at a later r9 pin whose `writeprobe` was red; that is a different r9 pin and sweep state, not a driver difference. The moved gate computes its set from the bd7a698 tree, so 44 is the pin-correct set, and the SKIP count (23) and every stage name match both captures.
- **smoke (HARN-08).** The moved run's final line carries the same stage set as the latest committed capture (`.planning/legacy/phases/20-arm64-catch-up-to-v2-1-parity/20-VERIFICATION.md:37` names `user-hooks-registered`, `no-unknown-device` and `no-unsupported-era`); the driver adds only `nic-bind`, which is in the same bd7a698 driver and was already part of the gate. Smoke has no pass count; the record is the 24 named `stage ok` lines plus the `ALL STAGES PASS` line. The driver diff above is the equivalence record.
- **drive-r9.** No gate script spawns it; it is recorded as the unreferenced interactive driver. Its diff is above and its assertions are unchanged.

## Verification commands (re-run)

- `nix run nixpkgs#shellcheck -- nix/gates/boot-r9.sh nix/gates/smoke-r9.sh nix/gates/tier1-r9.sh` → rc 0, no output.
- forbidden-token count (`limactl|subset-build\.sh|mkimage\.sh|stage-build\.sh|R9_SKIP_BUILD` outside comments) → **0**.
- `grep -l 'xtask image --arch aarch64 --image bootfs'` → all three; `HARVEY-TREE` present in all three.
- `tclsh [info complete]` for all three drivers → 1 (`DRIVERS-OK`).
- `nix eval --raw .#apps.aarch64-darwin.<app>.program` for the three → paths (`THREE-APPS`); the same eval on `x86_64-linux` succeeds; the three writeShellApplication wrappers build.
- `nix store verify --no-trust <harvey-tree-arm64>` → `TREE-UNMODIFIED`.
- `sha256sum target/aarch64-unknown-none-elf/debug/aarch64` → `76e6e153…` (01-08 baseline).
- `cargo xtask test` → 87-plan suite, 0 failed; `clippy` → 0; `check` → 0; `cargo xtask gate --arch x86-64` → `GATE-VENUE not-linux` (rc 1 by design on the Mac).

## Decisions Made

- **KERNDATE's checkout-derived provenance is gone.** The tree is a store path with no `.git`, and `cargo xtask image` does not consume `KERNDATE` (the engine bakes with mkpaqfs's own time field; D-29 compares member lists, A12). The `git` runtimeInput stays as the plan directed; it is inert.
- **The app's arm64 tree is `self.packages.${treeSystem}.harvey-tree-arm64`.** On the Mac that is the aarch64-linux package, the same rule the devshell uses, so `nix run` realises the registered builder's store tree and the app serves all three systems without `hasAttr` gating.
- **drive-r9 is rewired to r9's own launcher.** Harvey's `run-r9.sh` is not part of D-07's move and is absent from the read-only tree; spawning `nix/gates/boot-r9.sh` with `R9_KERNEL_PKG` makes the unreferenced driver internally consistent. Its assertions/markers/stages/timeouts are untouched.
- **Smoke/tier1 keep the driver-owned qemu spawn.** Only boot-r9 launches qemu from the script, exactly as Harvey's did.
- **The tier-1 set is computed, not hardcoded.** `44 T1-OK + 23 T1-SKIP` comes from the pinned tree's `tier1-cmds.txt` and `tier1-sweep.rc` plus the nine fixed probe names; the driver's post-pass asserts the exact set.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] drive-r9's capture directory had no creator**
- **Found during:** Task 2 (the drive-r9 rewire)
- **Issue:** the driver wrote its capture at `build/aarch64/drive-r9.log`, a directory that existed in Harvey's checkout but has no r9 equivalent; the plan requires the capture under `R9_GATE_OUT`, and no caller script creates that directory because no gate script spawns this driver.
- **Fix:** the capture is `$R9_GATE_OUT/drive-r9.log` (default `target/gates/boot-r9/drive-r9.log`) and the driver creates its directory with `exec mkdir -p [file dirname $CAPTURE]`.
- **Files modified:** `gates/aarch64/drive-r9.expect`
- **Verification:** tclsh completeness check passes; every assertion/marker/stage/timeout line is unchanged (diff recorded above); recorded in `.planning/WINDOWS.md`.
- **Committed in:** `1367fe8` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical), plus the drive-r9 path/env rewire recorded as a Decision.
**Impact on plan:** None on the moved gates or their evidence; drive-r9 remains unreferenced, and the support line only makes its capture path real.

## Issues Encountered

- **`nix build .#apps.<system>.<app>.program` cannot build an app's subpath string** ("string … is not the right placeholder"). The smoke/tier1 wrappers were built by `nix run`; boot-r9's wrapper was built from the derivation recorded in the program string's context (`BOOT-R9-APP-BUILT`). All three wrappers therefore pass `writeShellApplication`'s shellcheck gate.
- **The flake package's `kernel-virt.elf` sha256 is `90d0a4f9…`, not the baseline.** The sandbox build applies `--remap-path-prefix` (debug info), so the store package's ELF differs from the local build; the acceptance compares the locally built `target/aarch64-unknown-none-elf/debug/aarch64`, which is `76e6e153…` = the 01-08 baseline. The package boots the same source and the gates run green from it.
- **The tier-1 capture path is overwritten per run** (`R9_TIER1_LOG` defaults to the same file), so the HVF and TCG serial captures share `target/gates/tier1-r9/tier1-r9.log`; the two full run logs (`target/gate-tier1-hvf.log`, `target/gate-tier1-tcg.log`) retain both runs' PASS lines and counts.

## Known Stubs

None. Every moved script reads a real tree file or r9's assembler output; every driver's env is supplied by its app; no placeholder or unverified path remains.

## Threat Flags

None beyond the plan's threat model. T-01-67 is implemented by shellcheck (clean) plus quoted expansions in the rewritten scripts; T-01-68 by every output living under `R9_GATE_OUT` and `nix store verify --no-trust` printing `TREE-UNMODIFIED`; T-01-69 by Harvey's loopback-only slirp shape being kept unchanged; T-01-SC by no package installs (the only new inputs are the locked nixpkgs' qemu/dtc/expect/git/text tools).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 01-30 (the remaining aarch64 gates) can follow this shape exactly:** copy the script, `HARVEY=$R9_HARVEY_DIR_ARM64` + `HARVEY-TREE`, `cargo xtask image --arch aarch64 --image bootfs`, `R9_GATE_OUT`, driver under `gates/aarch64/`, and a `mkScriptApp` header with the same three exports and runtimeInputs.
- **The full-tier app list is closer:** `boot-r9`, `boot-r9-smoke` and `boot-r9-tier1` now exist on all three systems; `cargo xtask gate --full` still waits on `boot-r9-net-smoke` and `boot-r9-cpu` (01-30) and the x86_64 apps (01-23/24/31).
- **Harvey remains untouched** (HEAD `25d97dc4d…`, status hash `041d2929…`), both kernel baselines hold, and the arm64 tree is store-verified after the quick pass.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/gates/{boot-r9,smoke-r9,tier1-r9}.sh`, `gates/aarch64/{drive-r9,smoke-r9,tier1-r9}.expect`, `01-21-SUMMARY.md` — all FOUND; `flake.nix` modified.
- Task commits `4942690` (Task 1), `1367fe8` (Task 2) and `b93d9ab` (Task 3) are ancestors of HEAD; `git rev-list --count ed91b67..HEAD` = 3 (measured against the plan ledger).
- Task 1 re-ran: shellcheck rc 0; forbidden-token count 0; `xtask image --arch aarch64 --image bootfs` in all three; `HARVEY-TREE` in all three.
- Task 2 re-ran: `DRIVERS-OK`; the three diffs are recorded above.
- Task 3 re-ran: `THREE-APPS` (darwin + x86_64-linux evals); app wrappers build; tier-1 HVF and TCG rc 0 with 44/23/0; smoke HVF rc 0 with the full stage list; `TREE-UNMODIFIED`; virtshell kernel sha `76e6e153…`.
- Plan verification: D-12 test/clippy/check 0/0/0 and `GATE-VENUE` printed; Harvey HEAD `25d97dc4d…` and status hash `041d2929…`; zero `port/src` changes; no push.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them); `REQUIREMENTS.md` was not ticked (the shared-ID gate blocks HARN-06/HARN-08 while sibling plans lack SUMMARYs; the orchestrator owns the update).
