---
phase: 01-stage-p0-cross-build-harness
plan: 23
subsystem: cross-build-harness
tags: [x86_64, gates, expect, qemu, kvm, smoke, tier1, churn, services, image-assembler, harvey-tree, flake-app, d-07, d-09, d-16, d-19, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-14's full amd64 harvey tree (HARVEY-TREE stamp + the D-19 fixtures: tier1-sweep.rc, services-sweep.rc, tiers/*.txt, termrc files); 01-15's amd64 image manifests (chain, tier, churn, services) and the checked assembler; 01-16's cpu image + authdomain; 01-18's mkScriptApp helper, the moved x86_64 gate rewiring pattern and the xtask gate seam; 01-08/01-11's image engine and cargo xtask image CLI"
provides:
  - "nix/gates/{smoke,tier1,churn,services}-r9-x64.sh: the moved x86_64 composed gate scripts, reading only the read-only amd64 tree, r9's assembler images and R9_GATE_OUT"
  - "gates/x86_64/{smoke-r9,tier1-r9,churn-r9,services-r9}.expect: Harvey bd7a698 copies with path-token-only diffs"
  - "flake.nix: apps.x86_64-linux.{boot-r9-smoke-x64,boot-r9-tier1-x64,boot-r9-churn-x64,boot-r9-services-x64} via mkScriptApp with r9's own supply and bootR9X64RuntimeInputs"
  - "Mac evidence: the four apps evaluate (FOUR-APPS), the four drivers parse (DRIVERS-OK), shellcheck is clean, and the kernel bytes equal the 01-08 baseline"
affects: [01-24, 01-27, HARN-08, D-07, D-16, D-19]

actuals:
  tokens: 27893
  tasks: 2
  commits: 2

plan_head_before: b86ea010c3af6f6385cfdc55c0f932655c924db3
plan_head_after: bccc9ce

tech-stack:
  added:
    - "r9 flake apps boot-r9-smoke-x64, boot-r9-tier1-x64, boot-r9-churn-x64 and boot-r9-services-x64 (writeShellApplication via mkScriptApp)"
  patterns:
    - "A moved x86_64 gate takes HARVEY=$R9_HARVEY_DIR_AMD64 with the HARVEY-TREE stamp; the stage-build/subset-build/mkimage trio becomes `cargo xtask image --arch x86-64 --image <chain|tier|churn|services>` (+ cpu for the smoke gate's second leg); the composed module is target/images/x86_64/<image>.paq; every capture and run scratch dir lives under R9_GATE_OUT; R9_SKIP_BUILD is deleted"
    - "The app header exports R9_KERNEL_X64_PKG, R9_ISO_BUILDER, R9_NETDEV_EMITTER (where Harvey's had it), R9_HARVEY_DIR_AMD64, R9_MKPAQFS and R9_AUTHDOMAIN_GEN; bootR9X64RuntimeInputs carries the Rust toolchain, qemu, expect, procps, iproute2 and the text tools"
    - "The drivers are copied with path-token-only diffs; their env contracts, spawn argv, assertions, markers, stages and timeouts are byte-identical to Harvey's"

key-files:
  created:
    - nix/gates/smoke-r9-x64.sh
    - nix/gates/tier1-r9-x64.sh
    - nix/gates/churn-r9-x64.sh
    - nix/gates/services-r9-x64.sh
    - gates/x86_64/smoke-r9.expect
    - gates/x86_64/tier1-r9.expect
    - gates/x86_64/churn-r9.expect
    - gates/x86_64/services-r9.expect
  modified:
    - flake.nix

key-decisions:
  - "The smoke gate assembles TWO images: `--image chain` and `--image cpu`.  Harvey's mkimage baked bootfs-chain.paq and bootfs-cpu.paq for the gate's two legs; the cpu-chain leg cannot run without the cpu image, so both are assembled (the plan's four-name shorthand names each gate's primary image)."
  - "KERNDATE is kept as a wall-clock default (the 01-18 drawterm pattern: `export KERNDATE=\"${KERNDATE:-$(date +%s)}\"`), not Harvey's `git log` block: the tree is a store path with no .git, and the image engine does not consume KERNDATE."
  - "Every capture moved under R9_GATE_OUT (smoke-r9.log + smoke-r9-cpu.log, tier1-r9.log, churn-r9.log, services-r9.log) and both run scratch dirs are mktemp'd under R9_GATE_OUT, so the gates write only to r9 scratch and never under $HARVEY (T-01-75)."
  - "R9_AUTHDOMAIN_GEN is exported by all four app headers per the plan; only the smoke script validates it in-script, because only the cpu manifest (smoke's second image) carries authdomain generator entries."
  - "The four apps are registered on x86_64-linux only (beside drawterm-gate-x64): the app header consumes iso-builder, which the flake exports only where the q35 BIOS ISO machinery is available (X64-05)."
  - "The accel discipline keeps Harvey's `[ -e /dev/kvm ]` test verbatim — no silent downgrade; a KVM-less venue fails loudly unless R9_ACCEL=tcg is set explicitly."
  - "The driver image-path comments were updated to the assembled artifacts (target/images/x86_64/{tier,services}.paq); every assertion, marker, stage and timeout is untouched."

patterns-established:
  - "Moved x86_64 composed gate: HARVEY=$R9_HARVEY_DIR_AMD64 + HARVEY-TREE, image via `cargo xtask image --arch x86-64 --image <name>` into target/images/x86_64/<name>.paq, captures under R9_GATE_OUT, driver under gates/x86_64"
  - "Flake app header for the x86_64 gates: R9_KERNEL_X64_PKG / R9_ISO_BUILDER / R9_HARVEY_DIR_AMD64 / R9_MKPAQFS / R9_AUTHDOMAIN_GEN (+ R9_NETDEV_EMITTER where Harvey's had it); bootR9X64RuntimeInputs on PATH"
  - "A moved driver's equivalence record is `diff -u <harvey copy> <r9 copy>` and must show path/env-name tokens only"

requirements-completed: [HARN-08]

coverage:
  - id: D1
    description: "The x86_64 smoke, tier-1, churn and services gate scripts live in r9 and read only the read-only amd64 tree, r9's assembled images and R9_GATE_OUT"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- nix/gates/{smoke,tier1,churn,services}-r9-x64.sh (clean, rc 0)"
        status: pass
      - kind: other
        ref: "cat nix/gates/{smoke,tier1,churn,services}-r9-x64.sh | grep -vE '^[[:space:]]*#' | grep -cE 'stage-build\\.sh|subset-build\\.sh|mkimage\\.sh|R9_SKIP_BUILD' (0)"
        status: pass
      - kind: other
        ref: "grep -q 'xtask image --arch x86-64 --image' and '$HARVEY/HARVEY-TREE' -> all four"
        status: pass
    human_judgment: false
  - id: D2
    description: "The four drivers are in r9 with path-token-only diffs against Harvey's copies; every assertion, marker, stage and timeout is unchanged"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "for f in smoke-r9 tier1-r9 churn-r9 services-r9: tclsh [info complete] -> 1 (DRIVERS-OK)"
        status: pass
      - kind: other
        ref: "diff -u harvey:build/x86_64/<name>.expect gates/x86_64/<name>.expect -> path tokens only (recorded below)"
        status: pass
    human_judgment: false
  - id: D3
    description: "apps boot-r9-smoke-x64, boot-r9-tier1-x64, boot-r9-churn-x64 and boot-r9-services-x64 are registered and evaluate from r9's own supply"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "for a in the four: nix eval --raw .#apps.x86_64-linux.$a.program -> store path (FOUR-APPS)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The four apps pass under KVM from r9 on the owner's x86_64 host, with stage lists matching their Harvey-hosted evidence"
    requirement: HARN-08
    verification: []
    human_judgment: true
    rationale: "x86_64 ISO composition and the KVM boot are Linux-only; the Mac can only check statically. The KVM witness runs are batched at stop C (plan 01-27) by the plan's own design."
  - id: D5
    description: "Harvey is untouched and the x86_64 kernel bytes are unchanged after the flake.nix change"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "git -C harvey rev-parse HEAD -> 25d97dc4d4976bfb7411f01970755acf6261c81b; status hash 041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e (unchanged)"
        status: pass
      - kind: other
        ref: "cargo xtask build --arch x86-64 --config q35churnshell; shasum -a 256 -> 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f (the 01-08 baseline)"
        status: pass
    human_judgment: false

duration: 18 min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 23: Move the x86_64 smoke, tier-1, churn and services gates into r9 Summary

**The four remaining composed x86_64 gates — smoke (with its cpu-chain leg), tier-1, churn and services — now live in r9 as four scripts, four expect drivers and four flake apps that assemble their images with r9's own `cargo xtask image`, read only the read-only amd64 tree, and write only to r9 scratch; their KVM proof is queued for stop C (01-27).**

## Performance

- **Duration:** 18 min (2026-11-23T14:00:00Z → 2026-11-23T14:18:00Z)
- **Tasks:** 2 of 2
- **Files modified:** 9 (8 created, 1 modified; 2,233 insertions, 0 deletions)
- **Plan estimate:** 70,000 tokens; **actual:** 27,893 tokens (chars/4 over 111,572 chars of realized diff). The wall time went to the flake evaluations and the D-12 suite, not to the move itself.

## Accomplishments

- **The four x86_64 composed gate scripts are in r9 and rewired onto r9's supply.** `nix/gates/{smoke,tier1,churn,services}-r9-x64.sh` take `HARVEY=$R9_HARVEY_DIR_AMD64` and refuse a tree without the `HARVEY-TREE` stamp; the in-checkout `stage-build` + `subset-build` + `mkimage` trio is replaced by `cargo xtask image --arch x86-64 --image <chain|tier|churn|services>` (plus `--image cpu` for the smoke gate's cpu-chain leg), writing `target/images/x86_64/<image>.paq`; every capture and both run scratch dirs live under `R9_GATE_OUT` (`target/gates/<gate>`); `R9_SKIP_BUILD` is gone. shellcheck is clean (rc 0) and the forbidden-token count is 0.
- **The four drivers are in r9.** `gates/x86_64/{smoke-r9,tier1-r9,churn-r9,services-r9}.expect` are copies of Harvey bd7a698's drivers with path-token-only changes — the caller paths, the usage lines and the two image-artifact comments; the env contracts, qemu spawn argv, assertions, markers, stage order and timeouts are byte-identical. All four pass `tclsh`'s completeness check (`DRIVERS-OK`).
- **The four apps are registered.** `apps.x86_64-linux.{boot-r9-smoke-x64,boot-r9-tier1-x64,boot-r9-churn-x64,boot-r9-services-x64}` use `mkScriptApp`; each header exports `R9_KERNEL_X64_PKG`, `R9_ISO_BUILDER`, `R9_HARVEY_DIR_AMD64`, `R9_MKPAQFS` and `R9_AUTHDOMAIN_GEN`, the smoke app adds `R9_NETDEV_EMITTER`, and `bootR9X64RuntimeInputs` carries the Rust toolchain, qemu, expect, procps, iproute2 and the text tools. All four evaluate (`FOUR-APPS`).
- **The static proofs hold.** The locally built x86_64 virtshell kernel ELF sha256 is `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f`, the 01-08 baseline; Harvey is untouched (HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`), and zero `port/src` files changed.
- **D-12 is green before every commit:** `cargo xtask test` (87 tests, 0 failed), `cargo xtask clippy` and `cargo xtask check` exit 0.

## Task Commits

Each task was committed atomically:

1. **Task 1: Copy and rewire smoke-r9-x64, tier1-r9-x64, churn-r9-x64 and services-r9-x64** — `ffa59bd` (feat)
2. **Task 2: Copy the four drivers and register the four apps** — `bccc9ce` (feat)

**Plan metadata:** the `docs(01-23): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/gates/smoke-r9-x64.sh` (created) — the composed smoke: tree + HARVEY-TREE contract, `cargo xtask image --arch x86-64 --image chain` and `--image cpu`, the chain and cpu ISOs through r9's iso-builder (the cpu one with `--append=init=/amd64/init -c`), `gates/x86_64/smoke-r9.expect`, both captures under `R9_GATE_OUT`, the unchanged accel discipline and run-tag sweep.
- `nix/gates/tier1-r9-x64.sh` (created) — the tier-1 gate: the tier image, the computed T1-OK/T1-SKIP sets from the tree's `tier1-cmds.txt`/`tier1-sweep.rc`, `gates/x86_64/tier1-r9.expect`.
- `nix/gates/churn-r9-x64.sh` (created) — the net-free churn gate: the churn image, `gates/x86_64/churn-r9.expect`, the unique `r9x64churn-` run tag.
- `nix/gates/services-r9-x64.sh` (created) — the services gate: the services image (including the 4303-file font tree), the computed SVC-OK/SVC-SKIP sets, `gates/x86_64/services-r9.expect`.
- `gates/x86_64/smoke-r9.expect` (created) — Harvey's two-leg driver with path-token-only changes.
- `gates/x86_64/tier1-r9.expect` (created) — Harvey's tier-1 driver with path-token-only changes.
- `gates/x86_64/churn-r9.expect` (created) — Harvey's churn driver with path-token-only changes.
- `gates/x86_64/services-r9.expect` (created) — Harvey's services driver with path-token-only changes.
- `flake.nix` (modified) — the four apps, `bootR9X64RuntimeInputs`, and the x86_64-linux-only registration comment.

## Driver equivalence record

The four diffs below are the equivalence record required by HARN-08. Each was produced with
`diff -u <(git -C /Users/neilskinner/code/harvey show bd7a6982a3e4f465f8ed3da5e0c7290521514ff0:build/x86_64/<name>.expect) gates/x86_64/<name>.expect`.

### `smoke-r9.expect` — path-token-only

```diff
@@ -1,6 +1,6 @@
 #!/usr/bin/expect -f
 # smoke-r9.expect — the COMPOSED smoke gate for the x86_64 one-command
-# artifact (14-07, XEXEC-03): build/aarch64/smoke-r9.expect's stage
+# artifact (14-07, XEXEC-03): gates/aarch64/smoke-r9.expect's stage
 # vocabulary re-keyed stage-for-stage to the composed GRUB ISO boot —
@@ -59,7 +59,7 @@
-# Hygiene: the caller (nix/smoke-r9-x64.sh) rm -f's BOTH captures
+# Hygiene: the caller (nix/gates/smoke-r9-x64.sh) rm -f's BOTH captures
@@ -77,7 +77,7 @@
-# Env (exported by nix/smoke-r9-x64.sh — the flake app):
+# Env (exported by nix/gates/smoke-r9-x64.sh — the flake app):
@@ -98,7 +98,7 @@
-#          R9_NETDEV_ARGV=… expect build/x86_64/smoke-r9.expect
+#          R9_NETDEV_ARGV=… expect gates/x86_64/smoke-r9.expect
```

### `tier1-r9.expect` — path-token-only

```diff
@@ -26,7 +26,7 @@
-# Hygiene: the caller (nix/tier1-r9-x64.sh) rm -f's the capture BEFORE
+# Hygiene: the caller (nix/gates/tier1-r9-x64.sh) rm -f's the capture BEFORE
@@ -36,9 +36,9 @@
-# Env (exported by nix/tier1-r9-x64.sh — the flake app):
+# Env (exported by nix/gates/tier1-r9-x64.sh — the flake app):
 #   R9_ISO         REQUIRED — the composed tier ISO (r9's iso-builder
-#                  over out/bootfs-tier.paq)
+#                  over target/images/x86_64/tier.paq)
@@ -55,7 +55,7 @@
-#          expect build/x86_64/tier1-r9.expect
+#          expect gates/x86_64/tier1-r9.expect
```

### `churn-r9.expect` — path-token-only

```diff
@@ -48,7 +48,7 @@
-# Env (exported by nix/churn-r9-x64.sh — the flake app):
+# Env (exported by nix/gates/churn-r9-x64.sh — the flake app):
@@ -58,7 +58,7 @@
-# usage: env R9_ISO=… R9_CHURN_LOG=… expect build/x86_64/churn-r9.expect
+# usage: env R9_ISO=… R9_CHURN_LOG=… expect gates/x86_64/churn-r9.expect
```

### `services-r9.expect` — path-token-only

```diff
@@ -32,7 +32,7 @@
-# Hygiene: the caller (nix/services-r9-x64.sh) rm -f's the capture
+# Hygiene: the caller (nix/gates/services-r9-x64.sh) rm -f's the capture
@@ -42,9 +42,9 @@
-# Env (exported by nix/services-r9-x64.sh — the flake app):
+# Env (exported by nix/gates/services-r9-x64.sh — the flake app):
 #   R9_ISO         REQUIRED — the composed services ISO (r9's
-#                  iso-builder over out/bootfs-services.paq)
+#                  iso-builder over target/images/x86_64/services.paq)
@@ -62,7 +62,7 @@
-#          R9_EXPECT_SVC_SKIP=… expect build/x86_64/services-r9.expect
+#          R9_EXPECT_SVC_SKIP=… expect gates/x86_64/services-r9.expect
```

No assertion, marker, stage or timeout line differs in any driver. No driver was shared with plans 01-21/01-30 (those moved the aarch64 drivers under `gates/aarch64/`), so each file is a single r9 copy.

## Static evidence (this plan's Mac checks)

| Gate | Check | Result |
|---|---|---|
| all four scripts | `nix run nixpkgs#shellcheck -- …` | rc 0, no output |
| all four scripts | forbidden tokens (`stage-build.sh|subset-build.sh|mkimage.sh|R9_SKIP_BUILD`) outside comments | 0 |
| all four scripts | `xtask image --arch x86-64 --image` / `$HARVEY/HARVEY-TREE` | present in every script |
| all four drivers | `tclsh [info complete]` | 1 (`DRIVERS-OK`) |
| all four apps | `nix eval --raw .#apps.x86_64-linux.<app>.program` | store path each (`FOUR-APPS`) |
| kernel | `cargo xtask build --arch x86-64 --config q35churnshell` + sha256 | `50b01c9a…` = the 01-08 baseline |
| Harvey | HEAD + `status --porcelain` hash | `25d97dc4…` / `041d2929…` — unchanged |

The four KVM witness runs are batched at stop C (plan 01-27), where `cargo xtask gate --full` drives the apps from the owner's x86_64 host; no KVM pass is claimed here.

## Verification commands (re-run)

- `nix run nixpkgs#shellcheck -- nix/gates/smoke-r9-x64.sh nix/gates/tier1-r9-x64.sh nix/gates/churn-r9-x64.sh nix/gates/services-r9-x64.sh` → rc 0, no output.
- forbidden-token count → **0**; `xtask image --arch x86-64 --image` and `$HARVEY/HARVEY-TREE` present in all four.
- `tclsh [info complete]` for all four drivers → 1 (`DRIVERS-OK`).
- `nix eval --raw .#apps.x86_64-linux.{boot-r9-smoke-x64,boot-r9-tier1-x64,boot-r9-churn-x64,boot-r9-services-x64}.program` → four store paths (`FOUR-APPS`).
- `cargo xtask build --arch x86-64 --config q35churnshell` → rc 0; `shasum -a 256` → `50b01c9a…` (01-08 baseline).
- `cargo xtask test` → 87 passed, 0 failed; `cargo xtask clippy` → 0; `cargo xtask check` → 0.
- Harvey HEAD `25d97dc4…` and status hash `041d2929…`; zero `port/src` changes; no push.

## Decisions Made

- **The smoke gate assembles the chain and cpu images.** Harvey's mkimage baked `bootfs-chain.paq` and `bootfs-cpu.paq` because the gate boots two legs; r9's assembler needs one invocation per image, so the script runs `--image chain` and `--image cpu`. The other three scripts assemble their single image (`tier`, `churn`, `services`).
- **KERNDATE is a wall-clock default.** The store tree has no `.git`, and the image engine does not consume KERNDATE; `export KERNDATE="${KERNDATE:-$(date +%s)}"` keeps the name without a doomed `git log` call (the 01-18 drawterm pattern).
- **Every output lives under R9_GATE_OUT.** The captures (`smoke-r9.log` + `smoke-r9-cpu.log`, `tier1-r9.log`, `churn-r9.log`, `services-r9.log`) and the mktemp run dirs (`$R9_GATE_OUT/run.XXXXXX`) are all r9 scratch; nothing is written under `$HARVEY` (T-01-75).
- **The apps are x86_64-linux-only.** The header consumes `iso-builder`, exported only where the q35 BIOS ISO machinery exists (X64-05); on the Mac they are evaluated statically, and the owner's Linux host realises the x86_64 tree and builds the wrappers.
- **The drivers' image comments name the assembled artifacts** (`target/images/x86_64/{tier,services}.paq`); this is a path-token change, and every assertion/marker/stage/timeout is untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] The smoke gate also assembles the cpu image**
- **Found during:** Task 1 (the smoke-r9-x64 rewire)
- **Issue:** the plan's shorthand names `chain` as the smoke gate's assembler image, but Harvey's mkimage baked `bootfs-cpu.paq` for the driver's second (cpu-chain) leg, and that leg cannot boot without it.
- **Fix:** the script assembles both `--image chain` and `--image cpu` and composes the chain ISO and the cpu ISO (the latter with `--append=init=/amd64/init -c`) as Harvey's did.
- **Files modified:** `nix/gates/smoke-r9-x64.sh`
- **Verification:** shellcheck rc 0; the forbidden-token count 0; the app evaluates.
- **Committed in:** `ffa59bd` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical).
**Impact on plan:** None on scope: the cpu image is the manifest the smoke gate's own second leg consumes, and the driver is unchanged.

## Issues Encountered

- **`nix build` of an app wrapper on the Mac is out of venue.** The four apps are x86_64-linux-only, so their header resolves `self.packages.x86_64-linux.harvey-tree-amd64`; on the Mac the realised tree is the aarch64-linux one, and the x86_64 tree's local proot build fails (`/bin/sh: can't open '/guest.sh'`). The plan's verify is `nix eval` (green, `FOUR-APPS`); the wrappers build and run on the owner's x86_64 host at stop C.
- **`cargo clippy --all-targets` is not this repo's command** — the bare-metal kernel bin cannot be built as a host test target (E0405 class). The project's D-12 command is `cargo xtask clippy`, which is green.

## Known Stubs

None. Every moved script reads a real tree fixture or r9's assembler output; every driver's env is supplied by its app; no placeholder or unverified path remains.

## Threat Flags

None beyond the plan's threat model. T-01-74 is implemented by shellcheck (clean) plus quoted expansions in the rewritten scripts; T-01-75 by every output living under `R9_GATE_OUT` and the image engine writing only `target/images/`; T-01-76 by the smoke driver's loopback-only slirp argv coming unchanged from the emitter; T-01-SC by no package installs (the only new inputs are locked nixpkgs tools).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 01-24 (the x86_64 net gates) can follow this shape exactly:** copy the script, `HARVEY=$R9_HARVEY_DIR_AMD64` + `HARVEY-TREE`, `cargo xtask image --arch x86-64 --image net`, `R9_GATE_OUT`, driver under `gates/x86_64/`, and a `mkScriptApp` header with the same exports and `bootR9X64RuntimeInputs`.
- **The x86_64 app list is now eight of fifteen drivers:** drawterm-gate-x64 (01-18), smoke/tier1/churn/services (this plan); 01-24 adds the net trio + net-doctor and 01-31 the boot/auth/cpu-bringup/drawterm pieces.
- **Stop C (01-27) owns the KVM proof:** the four apps are ready for `cargo xtask gate --full`, with the KVM green runs and the Harvey-hosted equivalence checks due there.
- **Harvey remains untouched** (HEAD `25d97dc4d…`, status hash `041d2929…`), the kernel baseline holds, and no `port/src` file changed.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/gates/{smoke,tier1,churn,services}-r9-x64.sh`, `gates/x86_64/{smoke-r9,tier1-r9,churn-r9,services-r9}.expect`, `01-23-SUMMARY.md` — all FOUND; `flake.nix` modified.
- Task commits `ffa59bd` (Task 1) and `bccc9ce` (Task 2) are ancestors of HEAD; `git rev-list --count b86ea01..HEAD` = 2 (measured against the plan ledger).
- Task 1 re-ran: shellcheck rc 0; forbidden-token count 0; `xtask image --arch x86-64 --image` and `$HARVEY/HARVEY-TREE` in all four.
- Task 2 re-ran: `DRIVERS-OK`; the four diffs are path-token-only (recorded above); `FOUR-APPS`.
- Plan verification: D-12 test/clippy/check 0/0/0; kernel sha256 `50b01c9a…`; Harvey HEAD `25d97dc4…` and status hash `041d2929…`; zero `port/src` changes; no push.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
