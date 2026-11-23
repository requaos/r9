---
phase: 01-stage-p0-cross-build-harness
plan: 31
subsystem: cross-build-harness
tags: [x86_64, gates, expect, qemu, kvm, boot-r9-x64, auth, cpu-bringup, drawterm, flake-app, image-assembler, harvey-tree, d-07, d-08, d-09, d-16, d-19, d-20, harn-08, t-01-77, t-01-97, t-01-98]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-24's x86_64 net gates and net-doctor (R9_NET_DOCTOR for the tap leg); 01-23's moved x86_64 composed-gate rewiring pattern and bootR9X64RuntimeInputs; 01-18's mkScriptApp helper, the 0600 R9_DEMO_PWFILE pattern and the drawterm gate; 01-16's auth image (target/images/x86_64/auth.paq + auth.creds) and the per-bake p9sk1 flow; 01-14's read-only amd64 tree; 01-08/01-11's image engine and cargo xtask image CLI"
provides:
  - "nix/gates/boot-r9-x64.sh: the moved x86_64 plain boot — image chain|cpu via r9's assembler, R9_NET_DOCTOR-gated tap mode, outputs under R9_GATE_OUT"
  - "nix/gates/smoke-r9-auth-x64.sh: the moved auth gate — image auth via r9's assembler, the DEMO password by 0600 file path only (R9_DEMO_PWFILE, T-01-77)"
  - "gates/x86_64/smoke-r9-auth.expect: Harvey bd7a698's driver with path/env-name changes plus the recorded creds-by-file read"
  - "gates/x86_64/cpu-bringup.expect: the §15 cpu bring-up probe with path-token-only diffs"
  - "nix/gates/drawterm-x64.sh: r9's drawterm developer app against r9's own cpu image (creds file by path, T-01-97)"
  - "flake.nix: apps boot-r9-x64, boot-r9-auth-x64 (x86_64-linux) and drawterm (Linux); every HARN-08 x86_64 app evaluates and all fifteen drivers are present"
  - "Mac evidence: ALL-X64-APPS (twelve apps), AARCH64-APPS-OK (five apps), fifteen drivers, shellcheck clean, by-value password counts 0, kernel bytes equal the 01-08 baseline, Harvey untouched"
affects: [01-27, HARN-08, D-07, D-08, D-09, D-16, D-19, D-20, T-01-77, T-01-97, T-01-98]

actuals:
  tokens: 22307
  tasks: 2
  commits: 3

plan_head_before: 7be6c242e534dc6006a84d836a5460c79b13c549
plan_head_after: cec6c5e09b5bda76d2bebf14603e32f0aa9f73ac

tech-stack:
  added:
    - "r9 flake apps boot-r9-x64, boot-r9-auth-x64 (x86_64-linux) and drawterm (Linux)"
  patterns:
    - "A moved x86_64 boot gate takes HARVEY=$R9_HARVEY_DIR_AMD64 with the HARVEY-TREE stamp, assembles ONLY the image its mode selects with `cargo xtask image --arch x86-64 --image <chain|cpu>`, composes through r9's iso-builder, and writes every output under R9_GATE_OUT; R9_SKIP_BUILD is deleted"
    - "A moved auth gate takes its password from r9's 0600 auth.creds by copying the parsed line into a 0600 file under R9_GATE_OUT and exporting only R9_DEMO_PWFILE (the driver reads it by path; the value is never env, argv or printed)"
    - "A moved developer app (drawterm) reads r9's creds FILE for presence only, pins the committed hostfwd pair, scrubs the connection-bearing env, and lets drawterm prompt for the password itself"
    - "The driver-equivalence record is `diff -u <harvey copy> <r9 copy>`: path tokens only, plus the one recorded creds-by-file hunk"

key-files:
  created:
    - nix/gates/boot-r9-x64.sh
    - nix/gates/smoke-r9-auth-x64.sh
    - nix/gates/drawterm-x64.sh
    - gates/x86_64/smoke-r9-auth.expect
    - gates/x86_64/cpu-bringup.expect
  modified:
    - flake.nix

key-decisions:
  - "boot-r9-x64 assembles ONLY the image its system mode selects (`chain` for terminal, `cpu` for cpu): Harvey's mkimage baked every contract and picked one at its image-selection seam, while r9's assembler is one invocation per image; the marker image is never selected by any mode and is therefore not assembled."
  - "The cpu-mode §15 'PASS: baked' log grep has no assembler analogue: r9's `cargo xtask image` returns its own rc and prints `image: x86_64/cpu sha256 … members …`, so the boot script relies on the command's exit status plus the non-empty cpu.paq (the tee-masked-pipeline reason for Harvey's grep is gone)."
  - "R9_AUTHDOMAIN_GEN is required by boot-r9-x64 only in cpu mode (the chain manifest carries no authdomain entry) and always by the auth gate (the auth image does)."
  - "The auth gate assembles `--image auth` alone; `--image substrate` is NOT assembled because Harvey's auth script never booted that contract (the plan's conditional)."
  - "The auth driver's creds-by-file read is the sole non-path behavioural change: the required-env list, the env-contract comment and the read itself name R9_DEMO_PWFILE; every stage, marker, assertion and timeout is byte-identical to Harvey's."
  - "KERNDATE keeps the 01-18/01-23 wall-clock default (`${KERNDATE:-$(date +%s)}`): the read-only tree is a store path with no .git and the image engine does not consume KERNDATE."
  - "runtimeInputs are bootR9X64RuntimeInputs for both boot apps and [pkgs.bash] for drawterm: the x64 set already carries the Rust toolchain (`cargo xtask image`), qemu, expect, procps, iproute2 and the text tools; Harvey's plan9port/namespace tools are no longer consumed because there is no in-checkout 9 build, and stdenv.cc is not needed because the authdomain generator is r9's prebuilt store package."
  - "The two boot apps are x86_64-linux only (their headers consume kernel-x64 and iso-builder, exported only there — X64-05); `drawterm` rides its own isLinux gate on both Linux systems because it consumes only Harvey's drawterm package and no r9 attr."
  - "The drawterm wrapper's binary is `r9-drawterm` (the net-doctor naming convention) while the flake app attribute stays `drawterm`, so `nix run .#drawterm` reads as the developer command."
  - "File modes match Harvey's 100644 for the moved scripts and drivers (they are embedded through readFile, never exec'd from a checkout); a housekeeping commit normalised the four files created with 100755."

patterns-established:
  - "Moved x86_64 boot/auth gate: HARVEY=$R9_HARVEY_DIR_AMD64 + HARVEY-TREE, `cargo xtask image --arch x86-64 --image <name>` for exactly the selected image, iso-builder compose, R9_GATE_OUT scratch, driver under gates/x86_64"
  - "Password by file path: the shell parses the 0600 creds line into `$R9_GATE_OUT/.<gate>-password.$$`, exports only the path, and the EXIT trap removes it; the driver reads the file at the feed stage"
  - "Developer drawterm app: creds FILE presence only, `unset cpu auth PASS USER`, `-s ''` before `-O`, the committed hostfwd pair as loopback literals, windowed default with a loud `-G` headless fallback"

requirements-completed: [HARN-08]

coverage:
  - id: D1
    description: "nix/gates/boot-r9-x64.sh is in r9 and reads only the read-only amd64 tree, r9's assembled image and R9_GATE_OUT; tap mode keeps the net-doctor preflight and nothing writes under $HARVEY"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- nix/gates/boot-r9-x64.sh nix/gates/smoke-r9-auth-x64.sh (rc 0, no output)"
        status: pass
      - kind: other
        ref: "cat nix/gates/boot-r9-x64.sh nix/gates/smoke-r9-auth-x64.sh | grep -vE '^[[:space:]]*#' | grep -cE 'stage-build\\.sh|subset-build\\.sh|mkimage\\.sh|R9_SKIP_BUILD' (0)"
        status: pass
      - kind: other
        ref: "grep '$HARVEY/HARVEY-TREE' and 'xtask image --arch x86-64' -> present in both scripts"
        status: pass
    human_judgment: false
  - id: D2
    description: "The auth gate and its driver are in r9; the DEMO password travels only by a 0600 file path (R9_DEMO_PWFILE), never by environment value or argv"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "cat nix/gates/smoke-r9-auth-x64.sh gates/x86_64/smoke-r9-auth.expect | grep -vE '^[[:space:]]*#' | grep -cE 'R9_DEMO_PW([^F_]|$)' (0)"
        status: pass
      - kind: other
        ref: "echo 'set f [open gates/x86_64/smoke-r9-auth.expect]; puts [info complete [read $f]]' | tclsh (1)"
        status: pass
      - kind: other
        ref: "diff -u harvey:build/x86_64/smoke-r9-auth.expect gates/x86_64/smoke-r9-auth.expect -> path/env-name tokens plus the recorded T-01-77 creds-by-file hunk (recorded below)"
        status: pass
    human_judgment: false
  - id: D3
    description: "cpu-bringup.expect is in r9 with path-token-only diffs; no assertion, marker, stage or timeout changed"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "echo 'set f [open gates/x86_64/cpu-bringup.expect]; puts [info complete [read $f]]' | tclsh (1)"
        status: pass
      - kind: other
        ref: "diff -u harvey:build/x86_64/cpu-bringup.expect gates/x86_64/cpu-bringup.expect -> two comment hunks only (recorded below)"
        status: pass
    human_judgment: false
  - id: D4
    description: "r9's drawterm developer app is in the flake, connects to r9's cpu image through the committed hostfwd pair and reads r9's creds file by path; the password is never printed or on argv"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix eval --raw .#apps.{x86_64-linux,aarch64-linux}.drawterm.program -> store path each"
        status: pass
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- nix/gates/drawterm-x64.sh (rc 0, no output)"
        status: pass
      - kind: other
        ref: "grep -vE '^[[:space:]]*#' nix/gates/drawterm-x64.sh | grep -cE '(DRAWTERM_BIN|drawterm)[^#]*(password=|R9_DEMO_PW([^F_]|$))' (0)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every HARN-08 app now evaluates from r9 and all fifteen drivers are present; Harvey is untouched, the kernel bytes are unchanged and D-12 is green"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "for a in the twelve x86_64 apps: nix eval --raw .#apps.x86_64-linux.$a.program (ALL-X64-APPS); the five aarch64 apps also evaluate (AARCH64-APPS-OK)"
        status: pass
      - kind: other
        ref: "ls gates/x86_64/*.expect gates/aarch64/*.expect | wc -l (15)"
        status: pass
      - kind: other
        ref: "cargo xtask build --arch x86-64 --config q35churnshell; shasum -a 256 -> 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f (the 01-08 baseline)"
        status: pass
      - kind: other
        ref: "git -C harvey rev-parse HEAD -> 25d97dc4d4976bfb7411f01970755acf6261c81b; status hash 041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e"
        status: pass
      - kind: other
        ref: "cargo xtask test (87 passed, 0 failed); cargo xtask clippy (rc 0); cargo xtask check (rc 0)"
        status: pass
    human_judgment: false
  - id: D6
    description: "boot-r9-x64 and boot-r9-auth-x64 pass under KVM from r9 on the owner's x86_64 host, with stage lists matching their Harvey-hosted evidence (stop C, plan 01-27)"
    requirement: HARN-08
    verification: []
    human_judgment: true
    rationale: "The x86_64 ISO composition and KVM boot are Linux-only, and the auth gate's listener observation needs the Linux venue; the Mac can only check statically. The KVM witness runs are batched at stop C (plan 01-27) by design; no KVM pass is claimed here."

duration: 3 min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 31: Move the x86_64 plain boot, auth gate, cpu bring-up driver and drawterm app Summary

**The last four gate artifacts now live in r9 — boot-r9-x64 (chain/cpu by system mode, net-doctor-gated tap), the auth smoke with its DEMO password travelling only by a 0600 file path, the unreferenced cpu-bringup driver, and r9's own drawterm developer app against r9's cpu image — so every composed output Harvey's flake owned evaluates from r9 and all fifteen drivers are present, with the KVM runs queued for stop C (01-27).**

## Performance

- **Duration:** 3 min (2026-11-23T14:24:32Z → 2026-11-23T14:27:12Z)
- **Tasks:** 2 of 2
- **Files modified:** 6 (5 created, 1 modified; 1,651 insertions, 0 deletions)
- **Plan estimate:** 60,000 tokens; **actual:** 22,307 tokens (chars/4 over 89,230 chars of realized diff).
- **Commits:** 3 (Task 1, Task 2, and a file-mode housekeeping commit).

## Accomplishments

- **boot-r9-x64 is in r9 and rewired onto r9's supply.** `nix/gates/boot-r9-x64.sh` takes `HARVEY=$R9_HARVEY_DIR_AMD64` and refuses a tree without the `HARVEY-TREE` stamp; the in-checkout `stage-build`/`subset-build`/`mkimage` trio is replaced by `cargo xtask image --arch x86-64 --image chain|cpu` for exactly the image the system mode selects; every output lives under `R9_GATE_OUT`; `R9_SKIP_BUILD` is deleted. The accel discipline, the `--net`/`--system` parse (single accepted form, duplicate = error, cross-venue rejected), the emitter argv and the `R9_NET_DOCTOR --mode tap` preflight are Harvey's, with the doctor now the moved `r9-net-doctor`.
- **The auth gate is in r9 with the password by file path only.** `nix/gates/smoke-r9-auth-x64.sh` assembles `--image auth` through r9's assembler, parses `target/images/x86_64/auth.creds` loudly, writes the password line to a 0600 file under `R9_GATE_OUT`, exports only `R9_DEMO_PWFILE`, and removes the file in its EXIT trap; the driver reads that file at the feed stage (T-01-77). The capture now defaults under `R9_GATE_OUT`. The script never prints the password.
- **The two drivers are in r9.** `gates/x86_64/smoke-r9-auth.expect` is Harvey bd7a698's driver with path/env-name changes plus the one recorded creds-by-file read; `gates/x86_64/cpu-bringup.expect` is the §15 bring-up probe with two comment-path hunks. All assertions, markers, stages and timeouts are unchanged.
- **r9's drawterm developer app is registered.** `nix/gates/drawterm-x64.sh` connects to r9's own cpu image: `DRAWTERM_BIN` is Harvey's drawterm package, the connection is the committed hostfwd pair's loopback side (`cpu=127.0.0.1, auth=tcp!127.0.0.1!5670, user=glenda`), the creds FILE is read for presence only, the credential-bearing env is scrubbed, and drawterm prompts for the password itself — never printed, never on argv (T-01-97). Headless falls back loudly to `-G`.
- **The three apps are registered and the set is complete.** `apps.x86_64-linux.{boot-r9-x64,boot-r9-auth-x64}` and `apps.<linux>.drawterm` evaluate (`ALL-X64-APPS` over twelve names; `AARCH64-APPS-OK` over the five aarch64 apps); `ls gates/x86_64/*.expect gates/aarch64/*.expect | wc -l` is **15** (ten x86_64, five aarch64).
- **The static proofs hold.** The locally built x86_64 virtshell kernel ELF sha256 is `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f`, the 01-08 baseline; Harvey is untouched (HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`); zero `port/src` files changed.
- **D-12 is green before each commit:** `cargo xtask test` (87 passed, 0 failed), `cargo xtask clippy` and `cargo xtask check` exit 0.

## Task Commits

Each task was committed atomically:

1. **Task 1: Copy and rewire boot-r9-x64 and the auth gate; copy the auth driver with the creds-by-file change** — `5415e03` (feat)
2. **Task 2: cpu-bringup driver and the drawterm developer app; register the three apps; confirm every composed output has an r9 home** — `71263bf` (feat)
3. **Housekeeping: match Harvey's 100644 file modes for the moved gates** — `cec6c5e` (chore)

**Plan metadata:** the `docs(01-31): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/gates/boot-r9-x64.sh` (created) — the moved plain boot: tree + HARVEY-TREE contract, `xtask image` chain/cpu by system mode, the unchanged accel/net/system parse, the net-doctor tap preflight, the emitter argv, r9's iso-builder compose, foreground qemu.
- `nix/gates/smoke-r9-auth-x64.sh` (created) — the moved auth gate: tree + HARVEY-TREE, `xtask image --image auth`, the creds parse and 0600 `R9_DEMO_PWFILE`, the slirp emitter argv, the compose and the expect driver, the survivor sweep and password-file removal in the EXIT trap.
- `gates/x86_64/smoke-r9-auth.expect` (created) — Harvey's auth driver with path/env-name changes plus the creds-by-file read.
- `gates/x86_64/cpu-bringup.expect` (created) — Harvey's §15 cpu bring-up probe with path-token-only diffs.
- `nix/gates/drawterm-x64.sh` (created) — r9's drawterm developer app: env scrub, `-s ''` + `-O` pin, printed connect lines, loud `-G` headless fallback.
- `flake.nix` (modified) — `boot-r9-x64`, `boot-r9-auth-x64` (x86_64-linux) and `drawterm` (Linux) app registrations.

## Driver equivalence records

### `smoke-r9-auth.expect` — path/env-name tokens plus the recorded creds-by-file read

Produced with `diff -u <(git -C /Users/neilskinner/code/harvey show bd7a6982a3e4f465f8ed3da5e0c7290521514ff0:build/x86_64/smoke-r9-auth.expect) gates/x86_64/smoke-r9-auth.expect`:

```diff
@@ -6,7 +6,7 @@
 # `rc /lib/auth-sweep.rc r9' from the boot flow).  The caller is
-# nix/smoke-r9-auth-x64.sh = the flake app boot-r9-auth-x64.
+# nix/gates/smoke-r9-auth-x64.sh = the flake app boot-r9-auth-x64.
@@ -64,10 +64,10 @@
-# Env (exported by nix/smoke-r9-auth-x64.sh — the flake app):
+# Env (exported by nix/gates/smoke-r9-auth-x64.sh — the flake app):
-#                iso-builder output: r9 kernel-x64 + the auth bake as
+#                iso-builder output: r9 kernel-x64 + the auth image as
+#   R9_DEMO_PWFILE REQUIRED — the caller's 0600 file carrying the
+#                per-bake DEMO password (the caller parses it from
+#                r9's target/images/x86_64/auth.creds and exports only
+#                the PATH, T-01-77).  ...
-# usage: env R9_ISO=… R9_SMOKE_LOG=… expect build/x86_64/smoke-r9-auth.expect
+# usage: env R9_ISO=… R9_SMOKE_LOG=… expect gates/x86_64/smoke-r9-auth.expect
@@ -97,17 +104,17 @@
-foreach v {R9_ISO R9_SMOKE_LOG R9_NETDEV_ARGV R9_DEMO_PW} {
+foreach v {R9_ISO R9_SMOKE_LOG R9_NETDEV_ARGV R9_DEMO_PWFILE} {
-# R9_DEMO_PW — the per-bake DEMO password (the driver's read of the
-# bake's host-side feed copy, mkimage 14f2b).  ...
+# R9_DEMO_PWFILE — the per-bake DEMO password's 0600 file (the
+# caller's parse of r9's target/images/x86_64/auth.creds,
+# T-01-77: the value is never in the environment or on argv).  ...
@@ -330,14 +337,29 @@
-#     ORDER (the two getpassm passwords — the R9_DEMO_PW sends; then
+#     ORDER (the two getpassm passwords — the R9_DEMO_PWFILE sends;
+#     then
-set pw $env(R9_DEMO_PW)
+# T-01-77: the password is read from the caller's 0600 file HERE, by
+# path only — it is never present in this process's environment.
+if {[catch {
+    set pwfh [open $env(R9_DEMO_PWFILE) r]
+    set pw [string trimright [read $pwfh] "\r\n"]
+    close $pwfh
+} pw_err]} {
+    puts stderr "smoke-r9-auth-x64: cannot read \$R9_DEMO_PWFILE: $pw_err"
+    cleanup
+    exit 2
+}
+if {$pw eq ""} {
+    puts stderr "smoke-r9-auth-x64: \$R9_DEMO_PWFILE is empty — the caller's 0600 creds file carried no password"
+    cleanup
+    exit 2
+}
```

The only behavioural change is the creds-by-file read (T-01-77), named in the diff; every stage, marker, assertion and timeout is byte-identical to Harvey's.

### `cpu-bringup.expect` — path-token-only

```diff
@@ -49,7 +49,8 @@
 #   R9_ISO         REQUIRED — the COMPOSED cpu ISO (the iso-builder
 #                  output with --append=init=/amd64/init -c and
-#                  bootfs-cpu.paq staged as /boot/bootfs.paq)
+#                  target/images/x86_64/cpu.paq staged as
+#                  /boot/bootfs.paq)
@@ -61,7 +62,7 @@
-#          expect build/x86_64/cpu-bringup.expect
+#          expect gates/x86_64/cpu-bringup.expect
```

No assertion, marker, stage or timeout line differs. No script spawns this driver (it is the §15 cpu image's manual bring-up probe); its `R9_BAKE_LOG` env contract is unchanged.

## Static evidence (this plan's Mac checks)

| Gate | Check | Result |
|---|---|---|
| boot/auth/drawterm scripts | `nix run nixpkgs#shellcheck -- …` | rc 0, no output |
| boot/auth scripts | forbidden tokens (`stage-build.sh|subset-build.sh|mkimage.sh|R9_SKIP_BUILD`) outside comments | 0 |
| boot/auth scripts | `$HARVEY/HARVEY-TREE` + `xtask image --arch x86-64` | present in both |
| auth/drawterm | by-value password uses (`R9_DEMO_PW[^F_]`, `drawterm…password=`) outside comments | 0 each |
| two drivers | `tclsh [info complete]` | 1 (`DRIVERS-OK`) |
| drivers | `ls gates/x86_64/*.expect gates/aarch64/*.expect \| wc -l` | 15 |
| twelve x86_64 apps | `nix eval --raw .#apps.x86_64-linux.<app>.program` | store path each (`ALL-X64-APPS`) |
| five aarch64 apps | `nix eval --raw .#apps.aarch64-darwin.<app>.program` | store path each (`AARCH64-APPS-OK`) |
| drawterm (both Linux systems) | `nix eval .#apps.{x86_64-linux,aarch64-linux}.drawterm.program` | store path each |
| kernel | `cargo xtask build --arch x86-64 --config q35churnshell` + sha256 | `50b01c9a…` = the 01-08 baseline |
| Harvey | HEAD + `status --porcelain` hash | `25d97dc4…` / `041d2929…` — unchanged |
| D-12 | `cargo xtask test` / `clippy` / `check` | 87 passed / rc 0 / rc 0 |

The two KVM runs (boot-r9-x64 under slirp and its cpu system mode; boot-r9-auth-x64 under slirp) are batched at stop C (plan 01-27); no KVM pass is claimed here.

## Verification commands (re-run)

- `nix run nixpkgs#shellcheck -- nix/gates/boot-r9-x64.sh nix/gates/smoke-r9-auth-x64.sh nix/gates/drawterm-x64.sh` → rc 0, no output.
- forbidden-token count over the two boot scripts → **0**; `R9_DEMO_PW[^F_]` non-comment count over the auth gate + driver → **0**; the drawterm command-line password count → **0**.
- `tclsh [info complete]` for `gates/x86_64/{smoke-r9-auth,cpu-bringup}.expect` → 1.
- `ls gates/x86_64/*.expect gates/aarch64/*.expect | wc -l` → **15**.
- `nix eval --raw .#apps.x86_64-linux.<the twelve names>.program` → store path each (`ALL-X64-APPS`); the five aarch64 apps → store path each (`AARCH64-APPS-OK`); `drawterm` on both Linux systems.
- `cargo xtask build --arch x86-64 --config q35churnshell` → rc 0; `shasum -a 256` → `50b01c9a…` (01-08 baseline).
- `cargo xtask test` → 87 passed, 0 failed; `cargo xtask clippy` → 0; `cargo xtask check` → 0.
- Harvey HEAD `25d97dc4…` and status hash `041d2929…`; zero `port/src` changes; no push.

## Decisions Made

- **Only the selected image is assembled.** `boot-r9-x64` runs `--image chain` for `--system=terminal` and `--image cpu` for `--system=cpu`; Harvey's script baked every contract and picked one at its selection seam, but r9's assembler is one invocation per image. The marker image is never selected and therefore never assembled.
- **The §15 PASS-log grep is gone by construction.** Harvey tee'd its 2,096-line bake and grepped the `PASS: baked … bootfs-cpu.paq` line because the pipeline rc was masked; r9's `cargo xtask image` returns its own rc and prints its sha256/member line, so the boot script checks the rc and the non-empty `cpu.paq` instead.
- **R9_AUTHDOMAIN_GEN is mode-dependent in boot-r9-x64.** Only the cpu manifest carries an authdomain entry; the terminal path does not require the generator. The auth gate always requires it.
- **The auth gate does not assemble `substrate`.** Harvey's auth script never booted the substrate contract, so the plan's conditional is not taken.
- **The auth driver keeps every assertion; only the creds source changed.** `R9_DEMO_PWFILE` is read at the feed stage by path; the required-env list and its documentation follow the env-name change (T-01-77).
- **runtimeInputs are the x64 gate set.** `bootR9X64RuntimeInputs` supplies the Rust toolchain, qemu, expect, procps, iproute2 and the text tools for both boot apps; `drawterm` needs only `[ pkgs.bash ]`. Harvey's plan9port/namespace tools are dead weight now, and `stdenv.cc` is unnecessary because `harvey-authdomain-gen` is a store package.
- **Platform gates follow attribute availability.** The two boot apps are x86_64-linux only (their `kernel-x64`/`iso-builder` attrs exist only there); `drawterm` rides `isLinux` on both Linux systems because it consumes only Harvey's drawterm package.
- **The drawterm wrapper binary is `r9-drawterm`,** the app attribute stays `drawterm` (the net-doctor naming precedent).

## Deviations from Plan

### Process / housekeeping

**1. [Housekeeping] File modes normalised to Harvey's 100644**
- **Found during:** post-Task-2 review of `git ls-files -s`
- **Issue:** the four new scripts/drivers were created with mode 100755; Harvey stores every `nix/*.sh` and `build/*/*.expect` as 100644 (they are embedded via `readFile`, never exec'd from a checkout), and all prior moved gates in r9 match that convention.
- **Fix:** `git update-index --chmod=-x` on the four files, committed as `cec6c5e` (chore). No content changed.
- **Files modified:** `nix/gates/boot-r9-x64.sh`, `nix/gates/smoke-r9-auth-x64.sh`, `gates/x86_64/{smoke-r9-auth,cpu-bringup}.expect` (mode only)
- **Verification:** `git ls-files -s` shows 100644 for all moved gate scripts and drivers.
- **Committed in:** `cec6c5e`

---

**Total deviations:** 0 auto-fixed code deviations; 1 housekeeping commit (file modes).
**Impact on plan:** None on scope or behaviour: the mode change is metadata only, and the moved files now match the rest of the phase.

## Issues Encountered

- **`nix build` of the Linux app wrappers is out of venue.** The wrappers are x86_64-linux (and Linux for drawterm); on the Mac the plan's verify is `nix eval` (green). The wrappers build and run on the owner's x86_64 host at stop C.
- **The auth and boot runs cannot execute on the Mac.** They require `/dev/kvm` (or an explicit `R9_ACCEL=tcg`) and the x86_64 ISO machinery; that is precisely the stop-C witness (01-27).

## Known Stubs

None. Every moved script reads a real tree fixture or r9's assembler output; every driver's env is supplied by its app (or, for the manual cpu-bringup probe, by its documented caller); no placeholder or unverified path remains.

## Threat Flags

None beyond the plan's threat model. T-01-77 is implemented by the 0600 `R9_DEMO_PWFILE` (removed by the EXIT trap) and the recorded driver read; T-01-97 by the drawterm wrapper's presence-only creds check, the env scrub and the `-s ''`/`-O` pin with no password on argv; T-01-98 by keeping the tap preconditions and the net-doctor preflight unchanged and confining every output to `R9_GATE_OUT`; T-01-SC by no package installs (the only new inputs are locked nixpkgs tools).

## User Setup Required

None — no external service configuration. The tap venue (bridge/ACL) remains the owner's stop-C setup, documented by net-doctor's remediation output.

## Next Phase Readiness

- **Every HARN-08 composed output now has an r9 home.** `cargo xtask gate --full` can drive the thirteen x86_64 gate apps, and the two aarch64 halves are already green; the full fifteen-driver set is present for stop C (01-27).
- **Stop C (01-27) owns the KVM proof:** `nix run .#boot-r9-x64` (terminal and `--system=cpu`) and `nix run .#boot-r9-auth-x64` run under the owner's KVM, with stage lists compared against Harvey's committed evidence.
- **The drawterm developer app is ready for the owner's session:** `nix run .#drawterm` dials the committed hostfwd pair once `boot-r9-x64 --system=cpu` has assembled the cpu image and printed `cpurc: LISTEN-17010`.
- **Harvey remains untouched** (HEAD `25d97dc4d…`, status hash `041d2929…`), the kernel baseline holds, and no `port/src` file changed.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/gates/{boot-r9-x64,smoke-r9-auth-x64,drawterm-x64}.sh`, `gates/x86_64/{smoke-r9-auth,cpu-bringup}.expect` — all FOUND; `flake.nix` modified.
- Task commits `5415e03` (Task 1) and `71263bf` (Task 2) plus the mode housekeeping `cec6c5e` are ancestors of HEAD; `git rev-list --count 7be6c24..HEAD` = 3 (measured against the plan ledger).
- Task 1 re-ran: shellcheck rc 0; forbidden-token count 0; `$HARVEY/HARVEY-TREE` and `xtask image --arch x86-64` present in both; driver diff recorded (paths/env names + the T-01-77 read).
- Task 2 re-ran: `DRIVERS-OK`; `ALL-X64-APPS`; `AARCH64-APPS-OK`; 15 drivers; drawterm shellcheck rc 0; by-value password count 0; the cpu-bringup diff is path-only.
- Plan verification: D-12 test/clippy/check 87/0/0; kernel sha256 `50b01c9a…`; Harvey HEAD `25d97dc4…` and status hash `041d2929…`; zero `port/src` changes; no push.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
