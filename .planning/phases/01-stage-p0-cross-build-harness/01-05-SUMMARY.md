---
phase: 01-stage-p0-cross-build-harness
plan: 05
subsystem: cross-build-harness-spike
tags: [plan9, rio, drawterm, xvfb, expect, kvm, harvey, mkimage, determinism, d-11, s3]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "the Harvey pin bd7a6982a and the D-11 marker design in 01-RESEARCH.md"

provides:
  - "S3 rio gate: non -G drawterm under Xvfb plus a serial `ls /srv` assertion for rio.<user>.<pid> and riowctl.<user>.<pid>, with named failures"
  - "control-noG mode that must see no rio srv entry, the discriminating negative for HARN-01's non-zero-when-rio-is-absent property"
  - "scratch flake apps.x86_64-linux.s3-rio-gate that evaluates on aarch64-darwin over Harvey bd7a698's locked r9 116bcc2 and drawterm package"
  - "determinism.sh for A12's two-bake comparison, splitting the six deterministic images from the auth/cpu bake-time p9sk1 pair"
  - "static PASS verdicts for A5 (kernel-wide srv table visible from the serial console) and A6 (drawterm X11 backend); three KVM legs PENDING-OWNER for Plan 06"

affects: [pass-2 quick gate and the D-11 marker, Plan 06 KVM witness, HARN-01, HARN-03]

actuals:
  tokens: 15533
  tasks: 2
  commits: 2

plan_head_before: fb11b8c028935f0801728c6f9ae47f2afc07bd93
plan_head_after: 10325aa

tech-stack:
  added: []
  patterns:
    - "Gate rewire: keep Harvey's qemu/expect chassis, accel discipline, creds parse, run tags and redaction; change only the drawterm flags and the terminal assertion"
    - "Serial-console observation: type `ls /srv` on init's post-cpurc rc and parse expect_out(buffer) for the two rio srv names"
    - "Xvfb with -displayfd and -nolisten tcp; DISPLAY exported to the expect subtree and the server killed/swept on every exit path"
    - "Locked-flake runtime package resolution: re-export r9's packages and resolve them with `nix build path:$S3_FLAKE#<attr>` so aarch64-darwin evaluation avoids r9's import-from-derivation"
    - "Determinism split: byte-compare the six non-auth images; compare auth/cpu by bake-log member list and size because each bake embeds a fresh p9sk1 domain"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/S3-FINDINGS.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/.gitignore
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/flake.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/flake.lock
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/rio-gate.sh
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/rio-gate.expect
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/determinism.sh
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/static-analysis.md
  modified: []

key-decisions:
  - "Rio mode does not wait for a drawterm shell prompt: the profile execs rio, so no prompt is printed. The driver waits for the serial console's post-cpurc rc prompt, allows S3_RIO_BUDGET, then polls `ls /srv` and requires both rio and riowctl entries."
  - "The gate logs in as glenda (S3_DT_USER), not the creds-file probe user norbert: only /usr/glenda/lib/profile is staged, and rcmain sources a profile only when rc -li runs as a user whose $home/lib/profile is readable. The DEMO domain shares one password, so the parsed password authenticates glenda."
  - "The flake re-exports Harvey's locked r9 packages and resolves them with `nix build` at run time instead of interpolating them into the wrapper: r9 116bcc2's kernel derivation reads rust-src's Cargo.lock through import-from-derivation, which cannot build on aarch64-darwin. The app therefore evaluates on the Mac as the plan requires, and every input is still locked."
  - "determinism.sh refuses a first bake whose log records a KERNDATE different from the requested value, so an all-DIFFERENT result cannot be blamed on the clock; both Plan 06 gate runs pass KERNDATE=1795382101."
  - "Xvfb is started only in rio mode (the -G control needs no display), always with -nolisten tcp, and is both directly killed and display-keyed-swept on every exit path."
  - "HARN-01 and HARN-03 stay open: this plan authors and statically checks the gate and the determinism split, but the KVM witness and the pass-2 assembler are Plan 06/pass-2 work (D-22)."

patterns-established:
  - "A gate's control mode is part of the proof: the -G run must see no rio srv entry, otherwise the rio assertion is not specific"
  - "A macOS-evaluable Linux app must keep import-from-derivation out of its own build inputs; resolve the IFD-bearing packages at run time from the same lock"
  - "Byte-determinism comparisons must pin the same KERNDATE on both sides and verify it from the bake log, not assume it"

requirements-completed: []

coverage:
  - id: D1
    description: "The S3 rio gate and its -G control authoried from Harvey's drawterm gate, with the serial /srv assertion, named S3-FAIL messages and the Xvfb/-G contracts"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- spikes/s3/rio-gate.sh; bash -n; tcl info complete rio-gate.expect = 1; acceptance greps (S3_MODE, control-noG, riowctl, ls /srv, all four S3-FAIL messages, nolisten tcp, zero creds-copy lines)"
        status: pass
    human_judgment: true
    rationale: "The gate's decisive assertion runs only on the owner's x86_64 Linux host under KVM in Plan 06 (s3-rio-kvm-linux and s3-norio-control-kvm-linux, PENDING-OWNER); the Mac can only prove the scripts are well-formed."
  - id: D2
    description: "The scratch flake evaluates on aarch64-darwin for x86_64-linux over Harvey bd7a698's own locked r9 116bcc2 and drawterm package"
    verification:
      - kind: other
        ref: "nix eval --raw path:.../spikes/s3#apps.x86_64-linux.s3-rio-gate.program -> /nix/store/...-s3-rio-gate/bin/s3-rio-gate; flake.lock records bd7a6982a3e4f465f8ed3da5e0c7290521514ff0"
        status: pass
    human_judgment: false
  - id: D3
    description: "determinism.sh implements A12's two-bake comparison with the auth/cpu member-list and size split and the credential-copy prohibition"
    requirement: HARN-03
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- spikes/s3/determinism.sh; functional smoke test on a fake scratch tree (6 IDENTICAL, auth/cpu DIFFERENT with IDENTICAL member lists and sizes, S3-DETERMINISM-DONE); KERNDATE mismatch refusal"
        status: pass
    human_judgment: true
    rationale: "The real two-bake run needs Harvey's native-Linux mkimage toolchain in Plan 06 (s3-determinism-kvm-linux, PENDING-OWNER); the Mac smoke test validates the logic, not the Harvey bake."
  - id: D4
    description: "Static answers for A5 and A6 with file:line citations, plus the exact conditions of the cpu profile's rio guard"
    verification:
      - kind: other
        ref: "spikes/s3/evidence/static-analysis.md (devsrv static SRV, /lib/namespace bind, init/cpu newns paths, drawterm.nix Make.unix GUI=x11)"
        status: pass
    human_judgment: false
  - id: D5
    description: "S3-FINDINGS.md with two static PASS verdicts, three PENDING-OWNER KVM legs, the Plan 06 commands, assumptions A5/A6/A12 and the pass-2 implications"
    verification: []
    human_judgment: true
    rationale: "The findings and the pass-2 recommendation are reviewed by the owner at the pass-1 checkpoint, alongside the Plan 06 run results."

# Metrics
duration: 5 min
completed: 2026-11-23
status: complete
---

# Phase 1 Plan 5: S3 headless rio proof and mkimage determinism Summary

**Harvey's drawterm gate rewired into a headless rio assertion (Xvfb + non -G drawterm + serial `ls /srv` for rio and riowctl) with a -G negative control, a Mac-evaluable locked flake, and a two-bake determinism script for A12; A5 and A6 pass statically and three KVM legs await Plan 06**

## Performance

- **Duration:** 5 min
- **Started:** 2026-11-23T00:52:50Z
- **Completed:** 2026-11-23T00:57:13Z
- **Tasks:** 2
- **Files modified:** 8 (8 created, 0 modified)

## Accomplishments

- **The headless rio proof is written.** `s3/rio-gate.sh` keeps every
  contract of Harvey's `drawterm-gate-x64.sh` (package checks, KVM-default
  accel with no silent downgrade, emitter argv, capture-before-start, creds
  parse, run-tag sweeps, teardown) and adds `S3_MODE=rio`: Xvfb on a free
  display with `-nolisten tcp`, DISPLAY exported, drawterm without `-G`, then
  the serial console types `ls /srv` and requires both
  `rio.<user>.<pid>` and `riowctl.<user>.<pid>`. `S3_MODE=control-noG` runs
  the identical pipeline with `-G` and must see no rio entry. Named failures:
  `S3-FAIL rio-not-serving`, `S3-FAIL drawterm-exited`, `S3-FAIL no-session`,
  `S3-FAIL control-saw-rio`; successes print `S3-RIO-SERVING` /
  `S3-CONTROL-NO-RIO`.
- **The scratch flake evaluates on this Mac.** `apps.x86_64-linux.s3-rio-gate`
  prints a store path from `nix eval` despite targeting x86_64-linux, using
  Harvey `bd7a698`'s own locked r9 `116bcc2` and drawterm package. The r9
  packages are re-exported and resolved at run time because r9 116bcc2's
  kernel derivation has an import-from-derivation that cannot build on
  aarch64-darwin.
- **A12 is scriptable.** `s3/determinism.sh` records `bake1.sha256`, moves
  the nine paqs aside, re-bakes with the same pinned KERNDATE, records
  `bake2.sha256`, and prints `DETERMINISM <image> IDENTICAL|DIFFERENT` plus
  member-list/size notes for auth and cpu (which embed fresh p9sk1 domains
  by design). A smoke test on a fake scratch tree produced six IDENTICAL and
  two DIFFERENT with matching member lists and sizes.
- **A5 and A6 have static PASS verdicts.** `port/src/devsrv.rs:59` is a
  kernel-wide static table; only the `#s` mount is per namespace
  (`port/src/pgrp.rs:56-59`). The cpu image binds `#s` at `/srv`
  (`root-tree/lib/namespace:25`) for both init's console rc
  (`init.c:63`) and the cpu session (`cpu.c:620` → `auth_chuid.c:37`).
  Harvey's `nix/drawterm.nix:51` builds `Make.unix` with `GUI=x11`, so the
  X11 backend is linked. The three runtime legs are queued for Plan 06.

## Task Commits

Each task was committed atomically:

1. **Task 1: rewire Harvey's drawterm gate into the S3 rio gate and its scratch flake** - `fadd78c` (feat)
2. **Task 2: determinism.sh, static answers for A5/A6, and S3 findings** - `10325aa` (feat)

**Plan metadata:** pending (docs: complete plan)

## Files Created/Modified

- `spikes/s3/rio-gate.sh` — S3_MODE rio/control-noG gate: Xvfb, non `-G`
  drawterm, bake/compose, scratch-copy refusals, Xvfb + qemu + drawterm
  sweeps. shellcheck clean.
- `spikes/s3/rio-gate.expect` — the chassis plus the serial `ls /srv`
  assertion, the rio budget, the four named failures, the redaction
  post-pass. `info complete` = 1.
- `spikes/s3/flake.nix`, `flake.lock` — `apps.x86_64-linux.s3-rio-gate` over
  Harvey bd7a698's inputs; `packages.x86_64-linux.{kernel-x64,iso-builder,netdev-args}`
  re-export the locked r9 packages.
- `spikes/s3/determinism.sh` — A12's two-bake comparison with the auth/cpu
  member-list and size split; refuses a real checkout and a KERNDATE
  mismatch; never copies a credential artifact.
- `spikes/s3/evidence/static-analysis.md` — A5, A6, the cpu profile guard and
  A12's citations.
- `spikes/S3-FINDINGS.md` — the two static PASS verdicts, the three
  PENDING-OWNER legs with the exact Plan 06 commands, assumptions A5/A6/A12
  and the pass-2 implications.
- `spikes/s3/.gitignore` — ignores `work/` and `result*`.

## Decisions Made

- **Rio mode has no shell prompt to wait for.** The cpu profile ends in
  `exec rio`, so rc never prints a prompt; rio draws and writes nothing to
  the pty. The driver therefore waits for the serial console's rc prompt,
  allows the rio budget, then polls `ls /srv`. The control mode does wait for
  the `cpu% ` prompt, because there rio is skipped.
- **Login is glenda.** Only `/usr/glenda/lib/profile` is staged; the baked
  creds file names the DEMO probe user (norbert), who has no profile, so a
  literal `user=$R9_DEMO_USER` session could never start rio. All DEMO users
  share one password, so the parsed password authenticates glenda.
  `S3_DT_USER` makes the choice explicit.
- **Runtime package resolution over eval-time interpolation.** It is the
  only way to satisfy both "the app evaluates on the Mac" and "the app uses
  Harvey's locked r9 pin": the IFD in r9's kernel derivation cannot build on
  darwin, but `nix build` on the owner's host can.
- **KERNDATE is verified, not assumed.** `determinism.sh` reads the value
  from the first bake's log and refuses a mismatch; the Plan 06 gate commands
  pass the same value to both runs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Flake r9 package interpolation broke the required Mac evaluation**
- **Found during:** Task 1 (`nix eval` verify)
- **Issue:** the planned `writeShellApplication` bound
  `${harvey.inputs.r9.packages.x86_64-linux.kernel-x64}` directly. Evaluating
  it forced r9 116bcc2's `cargoDeps`, which reads
  `${toolchain}/lib/rustlib/src/rust/library/Cargo.lock` through
  import-from-derivation. Nix tried to build the x86_64-linux rust-src on
  aarch64-darwin and failed with "platform mismatch", so the plan's own
  `nix eval ...program` check could not pass.
- **Fix:** the flake now re-exports the same locked r9 packages as
  `packages.x86_64-linux.{kernel-x64,iso-builder,netdev-args}` and exports
  `S3_FLAKE`; `rio-gate.sh` resolves them with
  `nix build --no-link --print-out-paths "path:$S3_FLAKE#<attr>"` at run
  time. The app wrapper still exports the same environment names, and the
  three package paths can still be supplied directly for a standalone run.
- **Files modified:** `spikes/s3/flake.nix`, `spikes/s3/rio-gate.sh`,
  `spikes/S3-FINDINGS.md`
- **Verification:** `nix eval --raw path:.../spikes/s3#apps.x86_64-linux.s3-rio-gate.program`
  prints `/nix/store/...-s3-rio-gate/bin/s3-rio-gate`; the lock still records
  bd7a6982a and the nested r9 116bcc2.
- **Committed in:** `fadd78c` (Task 1), with the trade-off documented in
  `10325aa` (Task 2 findings).

**2. [Rule 2 - Missing Critical] The literal creds user cannot start rio**
- **Found during:** Task 1 (profile analysis)
- **Issue:** keeping the original `-O ... user=$R9_DEMO_USER` would log in as
  norbert, the DEMO probe user. `rc -li` sources `$home/lib/profile` only when
  it is readable (`rc/lib/rcmain:19,23-24`), and only glenda's profile is
  staged; without the profile the guarded `plumber; exec rio` never runs, so
  `S3_MODE=rio` would always end in `S3-FAIL rio-not-serving`.
- **Fix:** `S3_DT_USER` (default glenda) drives the `-O user=` and the
  password-prompt anchor; the creds file still supplies the password (DEMO
  shares one across users). The rationale and the citation are in
  `evidence/static-analysis.md` and `S3-FINDINGS.md`.
- **Files modified:** `spikes/s3/rio-gate.sh`, `spikes/s3/rio-gate.expect`,
  `spikes/s3/evidence/static-analysis.md`, `spikes/S3-FINDINGS.md`
- **Verification:** static profile/rcmain/cpu.c analysis; the runtime
  confirmation is Plan 06's rio leg.
- **Committed in:** `fadd78c` (scripts), `10325aa` (analysis/findings).

**3. [Rule 2 - Missing Critical] Determinism could compare two different KERNDATEs**
- **Found during:** Task 2 (writing the Plan 06 command recipe)
- **Issue:** the gate's KERNDATE defaults to the wall clock when the scratch
  tree has no git metadata, while `determinism.sh` was to be given
  1795382101. If Plan 06 ran the gate without KERNDATE, every image would
  differ for the wrong reason and A12 would be misreported.
- **Fix:** `determinism.sh` extracts `KERNDATE <n>` from the first bake's log
  and refuses with `bake1-kerndate-mismatch` when it differs from the
  requested value; the Plan 06 commands now pass `KERNDATE=1795382101` to
  both gate runs.
- **Files modified:** `spikes/s3/determinism.sh`, `spikes/S3-FINDINGS.md`
- **Verification:** shellcheck clean; the smoke test passes on a matching
  fake log and refuses on a mismatched one.
- **Committed in:** `10325aa` (Task 2).

---

**Total deviations:** 3 auto-fixed (1 Rule 3 blocking, 2 Rule 2 missing
critical).
**Impact on plan:** all three were necessary for the plan's own checks to
pass and for the owner's single Linux session to produce a decisive result.
No scope creep; no changes outside `.planning/`; Harvey and `port/src` were
never written.

## Issues Encountered

- The required KVM legs cannot run on this Mac (no `/dev/kvm`, Harvey's
  native-Linux userland build, grub/xorriso and the Linux-only drawterm
  package). They are scripted, statically checked and PENDING-OWNER exactly
  as the plan anticipated (D-13).
- The flake's first design hit r9's import-from-derivation on darwin; the
  runtime-resolution fix keeps the Mac check meaningful without weakening
  the pin.
- `nix run nixpkgs#shellcheck` and the local `tclsh` were enough for the
  static checks; no full app build was attempted on the Mac (it is an
  x86_64-linux derivation).

## Pending Owner Work

- **Plan 06, rio leg:** `HARVEY=<scratch> S3_OUT=<dir> R9_ACCEL=kvm
  KERNDATE=1795382101 nix run path:<r9>/spikes/s3#s3-rio-gate`; expects
  `S3-RIO-SERVING` and the driver's PASS line.
- **Plan 06, control leg:** the same command with `S3_MODE=control-noG`;
  expects `S3-CONTROL-NO-RIO`.
- **Plan 06, determinism leg:** `HARVEY=<scratch> S3_OUT=<dir>
  KERNDATE=1795382101 nix develop -c bash spikes/s3/determinism.sh`;
  expects six IDENTICAL, auth/cpu DIFFERENT, two notes, `S3-DETERMINISM-DONE`.
- The scratch Harvey tree must be a writable copy of bd7a698 with no `.git`;
  the gate refuses a real checkout or a `/nix/store` path.

## User Setup Required

None — no external service configuration. The flake's inputs and the Docker
and KVM prerequisites are the owner-host ones already recorded in STATE.md.

## Next Phase Readiness

- Pass 2 can adopt D-11's marker as written: non `-G` drawterm under Xvfb,
  serial `ls /srv` for `rio` and `riowctl`, with the `-G` control as the
  specificity check. No pixel check is needed unless A5 fails at runtime.
- A12's pass-2 differential test is ready: byte-compare the six non-auth
  images and compare auth/cpu by member list and size. If a supposedly
  deterministic image differs, the assembler plan must use member hashes and
  the `boot-iso` FOD cannot inherit the bake.
- HARN-01 and HARN-03 remain open until the Plan 06 KVM runs and the pass-2
  assembler land; this plan supplies the scripts, the static verdicts and the
  commands, not the runtime proof.
- Harvey remains at `25d97dc4d` with status hash `041d2929...`; r9's
  protected paths are unchanged from `ce51503`; nothing was pushed.

---
*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

All eight created files exist; both task commits (`fadd78c`, `10325aa`)
are ancestors of HEAD.
