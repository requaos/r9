---
phase: 01-stage-p0-cross-build-harness
plan: 24
subsystem: cross-build-harness
tags: [x86_64, net-gates, net-smoke, net-soak, net-stress, net-doctor, host-echo, expect, qemu, tap-venue, image-assembler, netaddr-seam, flake-app, d-07, d-09, d-16, d-19, harn-02, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-14's full amd64 tree (HARVEY-TREE stamp + build/x86_64/root-tree/lib/netaddr.rc, the netip-tap/netstress fixtures); 01-19's net image manifest (xtask/manifests/amd64/net.toml, 64 members, no bake-time generator); 01-16's netaddr seam keys (tap_gwv4=192.168.122.1, blastport=7777); 01-18's mkScriptApp helper, the moved x86_64 gate rewiring pattern and the xtask gate seam; 01-23's four moved x86_64 composed gates and bootR9X64RuntimeInputs; 01-08/01-11's image engine and cargo xtask image CLI"
provides:
  - "nix/gates/{smoke-r9-net-x64,soak-r9-net-x64,stress-r9-net-x64}.sh: the moved x86_64 net gate scripts, assembling the net image with r9's own `cargo xtask image --arch x86-64 --image net`, reading only the read-only amd64 tree and writing only R9_GATE_OUT"
  - "nix/gates/net-doctor.sh: the read-only host-state doctor (body byte-identical to Harvey bd7a698's copy) and nix/gates/host-echo.c: the soak/stress echo peer (byte-identical)"
  - "gates/x86_64/{smoke-r9-net,netsoak-r9,stress-r9}.expect: Harvey bd7a698 copies with path-token-only diffs"
  - "flake.nix: hostEchoX64Pkg and netDoctorPkg; apps net-doctor (Linux), boot-r9-net-smoke-x64, boot-r9-net-soak-x64 and boot-r9-stress-x64 (x86_64-linux) with R9_NET_DOCTOR, HOST_ECHO_X64 and the tree's HARVEY_NETADDR_SEAM"
  - "Mac evidence: FOUR-NET-APPS, DRIVERS-OK, HOST-ECHO-VERBATIM, shellcheck clean, forbidden-token count 0, kernel bytes equal the 01-08 baseline"
affects: [01-27, HARN-08, D-07, D-16, D-19, T-01-78, T-01-79]

actuals:
  tokens: 35326
  tasks: 2
  commits: 2

plan_head_before: de0c2024a9786b34d7c40c2bb103f5ae66db320c
plan_head_after: 477286e49701e31e03b91c006d2a8109731c6aff

tech-stack:
  added:
    - "r9 flake apps net-doctor (Linux), boot-r9-net-smoke-x64, boot-r9-net-soak-x64, boot-r9-stress-x64 (x86_64-linux)"
    - "hostEchoX64Pkg (stdenv.cc build of nix/gates/host-echo.c) and netDoctorPkg (writeShellApplication via mkScriptApp)"
  patterns:
    - "A moved x86_64 net gate takes HARVEY=$R9_HARVEY_DIR_AMD64 with the HARVEY-TREE stamp; the stage-build/subset-build/mkimage trio becomes `cargo xtask image --arch x86-64 --image net` (target/images/x86_64/net.paq); every capture, run scratch and retained evidence dir lives under R9_GATE_OUT; R9_SKIP_BUILD is deleted"
    - "HARVEY_NETADDR_SEAM names the seam file inside the read-only tree ($tree/build/x86_64/root-tree/lib/netaddr.rc), never a path in Harvey's build source (D-19); the doctor preflight is consumed through R9_NET_DOCTOR"
    - "The soak/stress evidence scratch (pcap, host-echo log, qemu -d cascade) is RETAINED under R9_GATE_OUT; TMPD is removed by the EXIT trap, which also sweeps qemu by this run's unique tag and kills the peer child"

key-files:
  created:
    - nix/gates/smoke-r9-net-x64.sh
    - nix/gates/soak-r9-net-x64.sh
    - nix/gates/stress-r9-net-x64.sh
    - nix/gates/net-doctor.sh
    - nix/gates/host-echo.c
    - gates/x86_64/smoke-r9-net.expect
    - gates/x86_64/netsoak-r9.expect
    - gates/x86_64/stress-r9.expect
  modified:
    - flake.nix

key-decisions:
  - "The net image is r9's `xtask/manifests/amd64/net.toml` (01-19, 64 members, no bake-time generator): all three gates assemble it with `cargo xtask image --arch x86-64 --image net`, so R9_AUTHDOMAIN_GEN is not exported by these headers and R9_SKIP_BUILD is deleted."
  - "HARVEY_NETADDR_SEAM is the tree path `${harvey-tree-amd64}/build/x86_64/root-tree/lib/netaddr.rc`; the scripts' fallback is the same tree-relative path under $HARVEY, so a gate run and the flake app read the same bytes (D-19)."
  - "The soak/stress retained evidence scratch moved from TMPDIR to `$R9_GATE_OUT/evidence.XXXXXX`: the pcap, the peer log and the qemu -d cascade are this run's evidence, and every output must live under r9 scratch (T-01-78); TMPD stays under `$R9_GATE_OUT` and is removed by the trap."
  - "KERNDATE is the 01-18/01-23 wall-clock default (`${KERNDATE:-$(date +%s)}`): the read-only tree is a store path with no .git, and the image engine does not consume KERNDATE."
  - "runtimeInputs are bootR9X64RuntimeInputs (the 01-23 pattern): Rust toolchain, qemu, expect, procps, iproute2 and the text tools. Harvey's plan9port and namespace-transport tools (util-linux/gnused/mount/binutils) are no longer consumed — there is no in-checkout 9 build left to run — and mkpaqfs arrives as R9_MKPAQFS."
  - "net-doctor keeps its own isLinux app gate (matching Harvey's 16-03 shape and the plan's 'Linux systems'), while the three gate apps ride the x86_64-linux gate: their headers consume iso-builder, exported only where the q35 BIOS ISO machinery exists (X64-05)."
  - "The doctor's wrapped binary is named `r9-net-doctor` (Harvey's was `harvey-net-doctor`); R9_NET_DOCTOR and the soak/stress expected-path text name the r9 binary. The script body is otherwise byte-identical to Harvey's."
  - "The doctor's script gained a `#!/usr/bin/env bash` shebang as part of its header: without it shellcheck's SC2148 makes the plan's own four-file shellcheck gate fail; the wrapped app supplies its own shebang regardless, and the body is unchanged."

patterns-established:
  - "Moved x86_64 net gate: HARVEY=$R9_HARVEY_DIR_AMD64 + HARVEY-TREE, image via `cargo xtask image --arch x86-64 --image net` into target/images/x86_64/net.paq, captures + retained evidence under R9_GATE_OUT, driver under gates/x86_64"
  - "Net-gate flake header: R9_KERNEL_X64_PKG / R9_ISO_BUILDER / R9_NETDEV_EMITTER / R9_HARVEY_DIR_AMD64 / R9_MKPAQFS (+ R9_NET_DOCTOR, HOST_ECHO_X64, HARVEY_NETADDR_SEAM for soak/stress); bootR9X64RuntimeInputs on PATH"
  - "A moved driver's equivalence record is `diff -u <harvey copy> <r9 copy>` and must show path/env-name tokens only"

requirements-completed: [HARN-08]

coverage:
  - id: D1
    description: "The x86_64 net smoke, net soak and stress gate scripts and net-doctor live in r9 and read only the read-only amd64 tree, r9's assembled net image and R9_GATE_OUT"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- nix/gates/{smoke-r9-net-x64,soak-r9-net-x64,stress-r9-net-x64,net-doctor}.sh (rc 0, no output)"
        status: pass
      - kind: other
        ref: "cat the three scripts | grep -vE '^[[:space:]]*#' | grep -cE 'stage-build\\.sh|subset-build\\.sh|mkimage\\.sh|R9_SKIP_BUILD' (0)"
        status: pass
      - kind: other
        ref: "each script: grep '$HARVEY/HARVEY-TREE' and 'xtask image --arch x86-64 --image net' -> present"
        status: pass
    human_judgment: false
  - id: D2
    description: "The three net drivers are in r9 with path-token-only diffs against Harvey's copies; every assertion, marker, stage and timeout is unchanged"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "for f in smoke-r9-net netsoak-r9 stress-r9: tclsh [info complete] -> 1 (DRIVERS-OK)"
        status: pass
      - kind: other
        ref: "diff -u harvey:build/x86_64/<name>.expect gates/x86_64/<name>.expect -> caller-path and usage-path tokens only (recorded below)"
        status: pass
    human_judgment: false
  - id: D3
    description: "host-echo.c is byte-identical and built by an r9 derivation; netDoctorPkg is wrapped; apps net-doctor, boot-r9-net-smoke-x64, boot-r9-net-soak-x64 and boot-r9-stress-x64 are registered and evaluate"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "cmp nix/gates/host-echo.c <(git -C harvey show bd7a698:nix/host-echo.c) -> HOST-ECHO-VERBATIM"
        status: pass
      - kind: other
        ref: "for a in net-doctor boot-r9-net-smoke-x64 boot-r9-net-soak-x64 boot-r9-stress-x64: nix eval --raw .#apps.x86_64-linux.$a.program -> store path (FOUR-NET-APPS)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The three x86_64 net apps pass under KVM (soak and stress on the owner's tap venue) from r9, with stage lists matching their Harvey-hosted evidence"
    requirement: HARN-08
    verification: []
    human_judgment: true
    rationale: "The x86_64 ISO composition and KVM boot are Linux-only; the tap venue needs the owner's bridge/ACL setup. The Mac can only check statically; the KVM witness runs are batched at stop C (plan 01-27) by design, and no KVM pass is claimed here."
  - id: D5
    description: "Harvey is untouched and the x86_64 kernel bytes are unchanged after the flake.nix change; D-12 stays green"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "git -C harvey rev-parse HEAD -> 25d97dc4d4976bfb7411f01970755acf6261c81b; status hash 041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e"
        status: pass
      - kind: other
        ref: "cargo xtask build --arch x86-64 --config q35churnshell; shasum -a 256 -> 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f (the 01-08 baseline)"
        status: pass
      - kind: other
        ref: "cargo xtask test (87 passed, 0 failed); cargo xtask clippy (rc 0); cargo xtask check (rc 0)"
        status: pass
    human_judgment: false

duration: 12 min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 24: Move the x86_64 net gates, net-doctor and host-echo into r9 Summary

**The three x86_64 network gates — net smoke, the tap-leg net soak and the CR-07 reduced stress — now live in r9 as scripts, expect drivers and flake apps that assemble r9's net image, read the netaddr seam from the read-only tree, preflight through the moved net-doctor, and run the moved host-echo peer, with their KVM proof queued for stop C (01-27).**

## Performance

- **Duration:** 12 min (2026-11-23T14:12:00Z → 2026-11-23T14:24:00Z)
- **Tasks:** 2 of 2
- **Files modified:** 9 (8 created, 1 modified; 2,965 insertions, 0 deletions)
- **Plan estimate:** 65,000 tokens; **actual:** 35,326 tokens (chars/4 over 141,307 chars of realized diff).

## Accomplishments

- **The three net gate scripts are in r9 and rewired onto r9's supply.** `nix/gates/{smoke-r9-net-x64,soak-r9-net-x64,stress-r9-net-x64}.sh` take `HARVEY=$R9_HARVEY_DIR_AMD64` and refuse a tree without the `HARVEY-TREE` stamp; the in-checkout `stage-build` + `subset-build` + `mkimage` trio is replaced by `cargo xtask image --arch x86-64 --image net` writing `target/images/x86_64/net.paq`; every capture, run scratch and retained evidence dir lives under `R9_GATE_OUT`; `R9_SKIP_BUILD` is gone. shellcheck is clean (rc 0) and the forbidden-token count is 0.
- **The venue contracts are preserved.** The soak still refuses slirp outright before any assembly or boot, the stress keeps tap-preferred/slirp-RACE-STRESS-labeled, both preflight through `R9_NET_DOCTOR --mode tap` and keep the seam parse, the emitter argv (pcap included) and the host-echo peer child; the smoke keeps the fixed-slirp emitter argv with the D-04 hostfwd pair. `HARVEY_NETADDR_SEAM` now names the seam file inside the read-only tree (`<tree>/build/x86_64/root-tree/lib/netaddr.rc`, D-19).
- **net-doctor and host-echo are in r9.** `nix/gates/net-doctor.sh` is Harvey's body byte-for-byte (plus the r9 header); `nix/gates/host-echo.c` is byte-identical (`cmp` verify). The flake now carries `netDoctorPkg` (the read-only doctor as `r9-net-doctor`) and `hostEchoX64Pkg` (a `stdenv.cc` build of the committed source, T-01-79).
- **The three drivers and four apps are in place.** `gates/x86_64/{smoke-r9-net,netsoak-r9,stress-r9}.expect` are Harvey bd7a698 copies whose only changes are caller/usage path tokens (recorded below); `apps.x86_64-linux.{net-doctor,boot-r9-net-smoke-x64,boot-r9-net-soak-x64,boot-r9-stress-x64}` all evaluate (`FOUR-NET-APPS`), with soak/stress exporting `R9_NET_DOCTOR`, `HOST_ECHO_X64` and the tree's `HARVEY_NETADDR_SEAM`.
- **The static proofs hold.** The locally built x86_64 virtshell kernel ELF sha256 is `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f`, the 01-08 baseline; Harvey is untouched (HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`), and zero `port/src` files changed.
- **D-12 is green after the change:** `cargo xtask test` (87 passed, 0 failed), `cargo xtask clippy` and `cargo xtask check` exit 0.

## Task Commits

Each task was committed atomically:

1. **Task 1: Copy and rewire smoke-r9-net-x64, soak-r9-net-x64, stress-r9-net-x64 and net-doctor** — `5cfbda6` (feat)
2. **Task 2: Copy the three net drivers and host-echo; add hostEchoX64Pkg and netDoctorPkg; register the four net apps** — `477286e` (feat)

**Plan metadata:** the `docs(01-24): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/gates/smoke-r9-net-x64.sh` (created) — the composed net smoke: tree + HARVEY-TREE contract, `cargo xtask image --arch x86-64 --image net`, the emitter's slirp argv, r9's iso-builder, `gates/x86_64/smoke-r9-net.expect`, the capture under `R9_GATE_OUT`, the unchanged accel discipline and `r9x64net-` run-tag sweep.
- `nix/gates/soak-r9-net-x64.sh` (created) — the tap-leg soak: net-mode parse with the slirp refusal, doctor preflight, seam parse, emitter tap argv + pcap, host-echo peer, net image, compose, `gates/x86_64/netsoak-r9.expect`, retained evidence scratch.
- `nix/gates/stress-r9-net-x64.sh` (created) — the reduced-stress gate: tap-preferred/slirp-labeled venues, doctor preflight in tap mode, the `-d int,guest_errors -D` cascade, `gates/x86_64/stress-r9.expect`, retained evidence scratch.
- `nix/gates/net-doctor.sh` (created) — Harvey's read-only host-state doctor, body byte-identical, header naming r9.
- `nix/gates/host-echo.c` (created) — the concurrent host echo peer, verbatim.
- `gates/x86_64/smoke-r9-net.expect` (created) — Harvey's driver with path-token-only changes.
- `gates/x86_64/netsoak-r9.expect` (created) — Harvey's driver with path-token-only changes.
- `gates/x86_64/stress-r9.expect` (created) — Harvey's driver with path-token-only changes.
- `flake.nix` (modified) — `netDoctorPkg`, `hostEchoX64Pkg`, the `net-doctor` app (Linux) and the three x86_64 net gate apps.

## Driver equivalence record

The three diffs below are the equivalence record required by HARN-08. Each was produced with
`diff -u <(git -C /Users/neilskinner/code/harvey show bd7a6982a3e4f465f8ed3da5e0c7290521514ff0:build/x86_64/<name>.expect) gates/x86_64/<name>.expect`.

### `smoke-r9-net.expect` — path-token-only

```diff
@@ -6,7 +6,7 @@
 # as an mb2 tag-3 module off the same ISO; the caller is
-# nix/smoke-r9-net-x64.sh = the flake app boot-r9-net-smoke-x64):
+# nix/gates/smoke-r9-net-x64.sh = the flake app boot-r9-net-smoke-x64):
@@ -94,7 +94,7 @@
-# Hygiene: the caller (nix/smoke-r9-net-x64.sh) rm -f's the capture
+# Hygiene: the caller (nix/gates/smoke-r9-net-x64.sh) rm -f's the capture
@@ -128,7 +128,7 @@
-# Env (exported by nix/smoke-r9-net-x64.sh — the flake app):
+# Env (exported by nix/gates/smoke-r9-net-x64.sh — the flake app):
@@ -149,7 +149,7 @@
-# usage: env R9_ISO=… R9_SMOKE_LOG=… expect build/x86_64/smoke-r9-net.expect
+# usage: env R9_ISO=… R9_SMOKE_LOG=… expect gates/x86_64/smoke-r9-net.expect
```

### `netsoak-r9.expect` — path-token-only

```diff
@@ -10,7 +10,7 @@
-# emitter's contract).  The caller is nix/soak-r9-net-x64.sh = the
+# emitter's contract).  The caller is nix/gates/soak-r9-net-x64.sh = the
@@ -67,7 +67,7 @@
-# Env (exported by nix/soak-r9-net-x64.sh — the flake app):
+# Env (exported by nix/gates/soak-r9-net-x64.sh — the flake app):
@@ -84,7 +84,7 @@
-# usage: env R9_ISO=… R9_SOAK_LOG=… R9_NETDEV_ARGV=… expect build/x86_64/netsoak-r9.expect
+# usage: env R9_ISO=… R9_SOAK_LOG=… R9_NETDEV_ARGV=… expect gates/x86_64/netsoak-r9.expect
```

### `stress-r9.expect` — path-token-only

```diff
@@ -75,7 +75,7 @@
-# Env (exported by nix/stress-r9-net-x64.sh — the flake app):
+# Env (exported by nix/gates/stress-r9-net-x64.sh — the flake app):
@@ -96,7 +96,7 @@
-# usage: env R9_ISO=… R9_STRESS_LOG=… R9_NETDEV_ARGV=… expect build/x86_64/stress-r9.expect
+# usage: env R9_ISO=… R9_STRESS_LOG=… R9_NETDEV_ARGV=… expect gates/x86_64/stress-r9.expect
```

No assertion, marker, stage, timeout, spawn token or env name differs in any driver.

## Static evidence (this plan's Mac checks)

| Gate | Check | Result |
|---|---|---|
| all four scripts | `nix run nixpkgs#shellcheck -- …` | rc 0, no output |
| three gate scripts | forbidden tokens (`stage-build.sh|subset-build.sh|mkimage.sh|R9_SKIP_BUILD`) outside comments | 0 |
| three gate scripts | `$HARVEY/HARVEY-TREE` + `xtask image --arch x86-64 --image net` | present in every script |
| three drivers | `tclsh [info complete]` | 1 (`DRIVERS-OK`) |
| four apps | `nix eval --raw .#apps.x86_64-linux.<app>.program` | store path each (`FOUR-NET-APPS`) |
| host-echo | `cmp` against Harvey bd7a698 | identical (`HOST-ECHO-VERBATIM`) |
| kernel | `cargo xtask build --arch x86-64 --config q35churnshell` + sha256 | `50b01c9a…` = the 01-08 baseline |
| Harvey | HEAD + `status --porcelain` hash | `25d97dc4…` / `041d2929…` — unchanged |
| D-12 | `cargo xtask test` / `clippy` / `check` | 87 passed / rc 0 / rc 0 |

The three KVM runs (net smoke under slirp; soak and stress on the owner's tap venue) are batched at stop C (plan 01-27); no KVM pass is claimed here.

## Verification commands (re-run)

- `nix run nixpkgs#shellcheck -- nix/gates/smoke-r9-net-x64.sh nix/gates/soak-r9-net-x64.sh nix/gates/stress-r9-net-x64.sh nix/gates/net-doctor.sh` → rc 0, no output.
- forbidden-token count → **0**; `$HARVEY/HARVEY-TREE` and `xtask image --arch x86-64 --image net` present in all three gate scripts.
- `tclsh [info complete]` for all three drivers → 1 (`DRIVERS-OK`).
- `nix eval --raw .#apps.x86_64-linux.{net-doctor,boot-r9-net-smoke-x64,boot-r9-net-soak-x64,boot-r9-stress-x64}.program` → four store paths (`FOUR-NET-APPS`); `.#apps.aarch64-linux.net-doctor` also evaluates; the app is absent on Darwin.
- `cmp nix/gates/host-echo.c <(git -C /Users/neilskinner/code/harvey show bd7a698…:nix/host-echo.c)` → identical.
- `cargo xtask build --arch x86-64 --config q35churnshell` → rc 0; `shasum -a 256` → `50b01c9a…` (01-08 baseline).
- `cargo xtask test` → 87 passed, 0 failed; `cargo xtask clippy` → 0; `cargo xtask check` → 0.
- Harvey HEAD `25d97dc4…` and status hash `041d2929…`; zero `port/src` changes; no push.

## Decisions Made

- **The net image is `xtask/manifests/amd64/net.toml`.** All three gates assemble it with one `cargo xtask image --arch x86-64 --image net`; because the net image has no bake-time generator (01-19), these headers do not export `R9_AUTHDOMAIN_GEN` and `R9_SKIP_BUILD` is deleted.
- **The seam is read from the tree.** `HARVEY_NETADDR_SEAM` is `${harvey-tree-amd64}/build/x86_64/root-tree/lib/netaddr.rc`; the scripts' fallback is the same tree-relative path under `$HARVEY`, so a standalone run and the flake app read identical bytes (D-19).
- **Retained evidence moved under `R9_GATE_OUT`.** Soak/stress create `$R9_GATE_OUT/evidence.XXXXXX` (pcap, `hostecho.log`, qemu `-D` cascade) and leave it; `TMPD` is under `$R9_GATE_OUT` and removed by the EXIT trap. Nothing is written under `$HARVEY` (T-01-78).
- **KERNDATE is a wall-clock default.** The store tree has no `.git`, and the image engine does not consume KERNDATE (`${KERNDATE:-$(date +%s)}` — the 01-18/01-23 pattern).
- **runtimeInputs are `bootR9X64RuntimeInputs`.** The 01-23 set already carries the Rust toolchain (`cargo xtask image`), qemu, expect, procps, iproute2 and the text tools; Harvey's plan9port and namespace-transport tools are no longer consumed because the in-checkout 9 build is gone, and mkpaqfs arrives as `R9_MKPAQFS`.
- **net-doctor rides its own Linux gate.** It consumes no r9 attrs, so it is registered for every Linux system (the 16-03 shape); the three gate apps stay x86_64-linux because their headers consume `iso-builder`, exported only where the q35 BIOS machinery exists (X64-05).
- **The doctor's binary is `r9-net-doctor`.** R9_NET_DOCTOR and the soak/stress expected-path messages name the r9 binary; the body is Harvey's byte-for-byte.
- **The doctor gained a shebang.** `#!/usr/bin/env bash` was added as part of its header so the plan's own shellcheck gate (SC2148) passes; `writeShellApplication` supplies its own shebang regardless and the body is unchanged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] net-doctor.sh had no shebang, failing the plan's shellcheck gate**
- **Found during:** Task 1 (the four-file shellcheck verify)
- **Issue:** Harvey's `nix/net-doctor.sh` begins with a comment, so `shellcheck` reports `SC2148` and the plan's `<verify>` command exits non-zero.
- **Fix:** added `#!/usr/bin/env bash` as the script's first line (header-only; the wrapped app supplies its own shebang, and the body is unchanged).
- **Files modified:** `nix/gates/net-doctor.sh`
- **Verification:** the four-file shellcheck now exits 0 with no output; `cmp`-level body review confirms only the header changed.
- **Committed in:** `5cfbda6` (Task 1 commit)

**2. [Rule 3 - Blocking] Nix would not see the untracked host-echo.c / drivers during app evaluation**
- **Found during:** Task 2 (the `FOUR-NET-APPS` verify)
- **Issue:** flake evaluation is pure over the git tree; `nix/gates/host-echo.c` (and the new drivers) were untracked, so `nix eval` failed with "Path … is not tracked by Git".
- **Fix:** `git add` the Task 2 files before the evaluation (they are the same files the task commits).
- **Files modified:** none beyond the task's own files
- **Verification:** `FOUR-NET-APPS` after staging.
- **Committed in:** `477286e` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 missing critical, 1 blocking).
**Impact on plan:** None on scope: the shebang is a header line required by the plan's own verify, and the staging order is a flake-evaluation mechanic.

## Issues Encountered

- **`nix build` of an app wrapper on the Mac is out of venue.** The apps are Linux-only (x86_64-linux for the gates, any Linux for net-doctor); on the Mac the x86_64 tree is realised by the registered builder, and the plan's verify is `nix eval` (`FOUR-NET-APPS`, green). The wrappers build and run on the owner's x86_64 host at stop C.
- **The soak/stress runs cannot execute on the Mac.** They require the owner's tap bridge/ACL venue and `/dev/kvm` (or an explicit `R9_ACCEL=tcg`); that is precisely the stop-C witness (01-27).

## Known Stubs

None. Every moved script reads a real tree fixture or r9's assembler output; every driver's env is supplied by its app; no placeholder or unverified path remains.

## Threat Flags

None beyond the plan's threat model. T-01-78 is implemented by keeping the venue preconditions and the doctor preflight verbatim and by confining every output to `R9_GATE_OUT` (the smoke's fixed-slirp D-04 hostfwd pair is unchanged); T-01-79 by the verbatim `cmp` copy plus the `stdenv.cc` derivation; T-01-SC by no package installs (the only new inputs are locked nixpkgs tools).

## User Setup Required

None — no external service configuration required. The tap venue itself (bridge/ACL) is the owner's stop-C setup, already documented by net-doctor's remediation output.

## Next Phase Readiness

- **The x86_64 net gates are ready for the stop-C witness (01-27):** `cargo xtask gate --full` can drive `boot-r9-net-smoke-x64` (slirp), `boot-r9-net-soak-x64 --net=tap` and `boot-r9-stress-x64 --net=tap` on the owner's host; `net-doctor --mode tap` is the preflight.
- **Plan 01-31 completes HARN-08's "r9 produces the composed outputs" half** (the plain x86_64 boot, auth smoke, cpu bring-up and drawterm pieces). The x86_64 app list is now twelve of fifteen: drawterm-gate-x64, smoke/tier1/churn/services (01-23), the net trio + net-doctor (this plan).
- **Harvey remains untouched** (HEAD `25d97dc4d…`, status hash `041d2929…`), the kernel baseline holds, and no `port/src` file changed.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/gates/{smoke-r9-net-x64,soak-r9-net-x64,stress-r9-net-x64,net-doctor}.sh`, `nix/gates/host-echo.c`, `gates/x86_64/{smoke-r9-net,netsoak-r9,stress-r9}.expect`, `01-24-SUMMARY.md` — all FOUND; `flake.nix` modified.
- Task commits `5cfbda6` (Task 1) and `477286e` (Task 2) are ancestors of HEAD; `git rev-list --count de0c2024..HEAD` = 2 (measured against the plan ledger).
- Task 1 re-ran: shellcheck rc 0; forbidden-token count 0; `$HARVEY/HARVEY-TREE` and `xtask image --arch x86-64 --image net` present in all three.
- Task 2 re-ran: `DRIVERS-OK`; the three diffs are path-token-only (recorded above); `FOUR-NET-APPS`; `HOST-ECHO-VERBATIM`.
- Plan verification: D-12 test/clippy/check 0/0/0; kernel sha256 `50b01c9a…`; Harvey HEAD `25d97dc4…` and status hash `041d2929…`; zero `port/src` changes; no push.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
