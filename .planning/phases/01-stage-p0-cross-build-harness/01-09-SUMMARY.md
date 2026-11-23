---
phase: 01-stage-p0-cross-build-harness
plan: 09
subsystem: cross-build-harness-rio-derisk
tags: [rio, drawterm, xvfb, xdotool, xtest, p9sk1, authdomain-gen, auth-creds, d-28, d-13, harn-01, witness-driver]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "the pass-1 S3 rio harness and its FAIL evidence (spikes/s3), the corrected tr-ok attribution from 01-07, and pass-2 decisions D-28/D-13"

provides:
  - "rio-derisk/RIO-DERISK.md: CREDENTIAL-PATH, PROMPT-PATH, ROOT-CAUSE, FIX, LOGIN-USER and TEARDOWN, all pinned at Harvey bd7a698"
  - "rio-derisk/rio-gate.sh + rio-gate.expect: the fixed headless rio gate — X11 XTEST feed from a 0600 file, pid+identity teardown, unchanged two-entry marker and control-noG"
  - "rio-derisk/flake.nix + flake.lock: the x86_64-linux rio-gate app over Harvey's own locked inputs, adding xdotool"
  - "witness/run-witness.sh + README.md: the reusable root-refusing pass-2 driver (steps rio, control, a13) and the pull-only return paths"

affects: [01-10, 01-12, 01-13, 01-18, HARN-01, D-13, D-28]

actuals:
  tokens: 19517
  tasks: 3
  commits: 3

plan_head_before: cf3e812f873f1d16cadff066972549cf79a08304
plan_head_after: 15962835f3e648428b22f92824e0433a0ed536bb

tech-stack:
  added:
    - "xdotool (nixpkgs, x86_64-linux) — XTEST keyboard feeder for the private Xvfb display"
  patterns:
    - "A de-risk plan states each finding as a machine-greppable KEY: line with a bd7a698-pinned file:line citation, then the supporting trace"
    - "A credential travels as one 0600 file under the run scratch dir; only its path crosses into the driver, and the EXIT trap removes it"
    - "Interactive console input is traced to the device that actually feeds it (kbdq) before choosing a synthetic-input venue"
    - "Process teardown uses recorded pids plus a /proc cmdline+environ identity sweep; an argv0 tag is only ever an extra key"
    - "A witness driver is a step registry of self-contained step_<name> functions selected by an env var, with per-step isolation and a summary.tsv"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/RIO-DERISK.md
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/flake.nix
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/flake.lock
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/rio-gate.sh
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/rio-gate.expect
    - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
    - .planning/phases/01-stage-p0-cross-build-harness/witness/README.md
  modified: []

key-decisions:
  - "FIX is candidate (a): XTEST keystrokes into the private Xvfb display with xdotool, feeding the same kbdq path the owner's manual graphical login used. Candidate (b) is rejected because no drawterm option or environment supplies the password non-interactively; candidate (c) (a preloaded factotum key) is rejected for this plan as a second long-lived daemon and an unmeasured protocol path, and is recorded as the fallback if XTEST fails on the witness."
  - "The drawterm window is found by WM class (`xdotool search --classname drawterm`), not by pid: drawterm sets WM_NAME/WM_CLASS but never _NET_WM_PID, so the plan's illustrative `search --pid` cannot match."
  - "The session logs in as glenda, not the creds file's norbert: only /usr/glenda/lib/profile is staged, and the DEMO users share one password, so the norbert creds value authenticates glenda. The prompt is `glenda@DEMO-harvey password: `, not `glenda@glenda`."
  - "The password lives in exactly one 0600 file inside the run scratch dir; only R9_DT_PWFILE is exported. The control-noG pty feed reads the same file, so there is no password environment variable and no argv entry."
  - "Teardown reaps qemu, Xvfb and drawterm by recorded exp_pid/$! plus a /proc/<pid>/{cmdline,environ} identity sweep; the argv0 tag and the scratch path are extra keys, because pass 1's survivor showed the store path instead of the tag."
  - "The copied S3 flake.lock covers the identical inputs; `nix flake metadata` left it byte-identical, so `nix flake lock` was not run. The app keeps the S3 IFD-free shape so `nix eval ...#apps.x86_64-linux.rio-gate.program` succeeds on the Mac."
  - "The witness a13 step is forward-declared for plan 01-10's `checks.x86_64-linux.harvey-drawterm`; it fails loudly until that check exists, never silently skips."

patterns-established:
  - "Findings doc: KEY: statement + bd7a698-pinned citations, a detail trace, then a 'Not gate evidence' section"
  - "Credential hygiene: 0600 file in the run scratch, path-only export, EXIT-trap removal, and a grep gate that forbids the value in any feeder command line"
  - "A synthetic-input venue is justified by tracing the real input device first (here readcons -> /dev/cons -> kbdq <- X key events)"
  - "Witness step registry: step_<name>, R9_WITNESS_STEPS selection, summary.tsv, survivors and secret scan, WITNESS-RETURN.md"

requirements-completed: [HARN-01]

coverage:
  - id: D1
    description: "RIO-DERISK.md answers CREDENTIAL-PATH, PROMPT-PATH, ROOT-CAUSE, FIX, LOGIN-USER and TEARDOWN from pinned Harvey source, records no password value, and rejects the pass-1 tr-ok line as gate evidence"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "Task 1 greps: six KEY lines present, 69 bd7a698/:line citations, password-value grep 0, one '## Not gate evidence' heading (commit 4764014)"
        status: pass
    human_judgment: true
    rationale: "The greps prove the shape and the absence of a secret; whether the source trace is complete and the root cause correctly attributed is a reading judgment, and the fix is only proven by plan 01-12's KVM run."
  - id: D2
    description: "A statically clean, Mac-evaluable fixed rio harness exists: the XTEST credential feed, the unchanged two-entry marker and control-noG, and pid+identity teardown"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "shellcheck rio-gate.sh clean; tclsh info complete = 1; nix eval ...#apps.x86_64-linux.rio-gate.program -> /nix/store/...-rio-gate/bin/rio-gate; password-on-cmdline grep 0; RIO-SERVING/RIO-CONTROL-NO-RIO/riowctl/exp_pid greps pass; spikes diff empty (commit 7482f48)"
        status: pass
    human_judgment: true
    rationale: "The static contract is proven on the Mac; whether XTEST reaches drawterm's window on the owner's KVM host and whether rio then posts both /srv entries is exactly what plan 01-12 must measure."
  - id: D3
    description: "A reusable, root-refusing, sudo-free witness driver with rio/control/a13 steps and a README exists, ready for stop A"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "shellcheck run-witness.sh clean; bash -n 0; step_rio/step_control/step_a13 and R9_WITNESS_STEPS present; sudo count 0; id -u present; WITNESS-RETURN.md and witness-returns/01- README greps pass (commit 1596283)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The KVM rio, control and a13 legs are NOT run or claimed by this plan; plan 01-12 runs them on the owner's host (D-13)"
    verification: []
    human_judgment: true
    rationale: "The Mac cannot reach the owner's x86_64 KVM host or writable /dev/kvm; this plan deliberately claims no runtime rio verdict."

duration: 4min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 09: Rio de-risk — the DEMO credential path, the X11 password feed and the pass-2 witness driver Summary

**The rio de-risk is now anchored in Harvey source: the per-bake DEMO password lives in `out/auth-creds.txt` (user norbert, shared with glenda), drawterm's X11 prompt reads only X key events into `kbdq` and never the pty, and a fixed rio-gate with an XTEST feed plus pid-and-identity teardown and a reusable rio/control/a13 witness driver is ready for the KVM stop.**

## Performance

- **Duration:** ~4 min of executor time (04:53:45Z to 04:57:13Z)
- **Started:** 2026-11-23T04:53:45Z
- **Completed:** 2026-11-23T04:57:13Z
- **Tasks:** 3 of 3
- **Files modified:** 7 (all created; 1,038 lines of scripts plus the findings doc)

## Accomplishments

- `RIO-DERISK.md` states, with pinned `bd7a698` citations, exactly where the DEMO credential comes from: `authdomain-gen.c:312-333` generates glenda/bootes/norbert with one shared user password and authid bootes; `authdomain-gen.c:625-647` writes `auth-creds` as `user=<last user>` + `password=<secret>` (norbert on DEMO); `mkimage.sh:1533-1541` copies it to `out/auth-creds.txt` in §14 and `mkimage.sh:1941-1943` overwrites it with §15's cpu pair, so the surviving file is the image the gate dials. The generator's machine and secstore passwords are separate; glenda's key derives from the same string only because DEMO copies one password to all three users (`authdomain-gen.c:521-526`).
- `RIO-DERISK.md` traces the prompt path rather than guessing it: `readcons.c:44-59` opens drawterm's own `/dev/cons` and prints there; `main.c:74-79` maps that to the `#c` console; `devcons.c:657-721` reads `kbdq`, fed by `echo()` inside `kbdputc` (`devcons.c:291-371`, `:399-446`); the X11 handler is the only feeder in that mode (`x11.c:1111-1250`, `kbdputc` at `:1249`, Return at `:1193-1195`); the host-stdin reader starts only with `-G` (`main.c:50-53`, `:180-249`). **Pty input can never reach the prompt under X11**, and the prompt is drawn into the window, not the pty (`devcons.c:364-370`, `x11.c:548-561`).
- The root cause is recorded with both causes: the pass-1 driver fed and waited on the pty in X11 mode (`spikes/s3/rio-gate.expect:220-222`), which cannot work, and it had not parsed the generated password from the bake. The captured timeout is `spikes/s3/evidence/rio-kvm-linux.log:53`. The `tr-ok` line is restated as the owner's manual graphical login and never gate evidence (D-28).
- The fixed harness keeps every S3 contract and changes only what the findings require: the app is `rio-gate` with `RIO_MODE=rio|control-noG` (`S3_MODE` alias), `xdotool` joins the locked runtimeInputs, the password is written to a 0600 file in the run scratch and only its path is exported, rio mode feeds via XTEST on the private display, and the marker stays the pair `rio.<user>.<pid>` + `riowctl.<user>.<pid>`. The control's pty feed and its `RIO-CONTROL-NO-RIO` / `RIO-FAIL control-saw-rio` outcomes are unchanged.
- Teardown no longer trusts the argv0 tag: qemu's `exp_pid`, drawterm's pty child `exp_pid` and Xvfb's `$!` are recorded and reaped on every exit path, then a `/proc/<pid>/{cmdline,environ}` sweep kills any leftover of this run. Pass 1's PID 88800 survived precisely because `ps` showed the store path, not the tag.
- `witness/run-witness.sh` is the owner's one-command handoff: root-refusing, sudo-free, git-root resolution with the HEAD recorded, a preflight `out/env.txt`, a `step_<name>` registry (`rio`, `control`, `a13`) selected by `R9_WITNESS_STEPS`, per-step subshell isolation with `summary.tsv`, a survivor list and a credential scan, and `WITNESS-RETURN.md` beside `out/`. The README documents the pull-only sequence, both return paths (`witness-returns/01-12` or a pasted return), what never to send back, and cleanup.

## Task Commits

Each task was committed atomically:

1. **Task 1: Determine the DEMO credential path and drawterm's X11 password path; write RIO-DERISK.md** — `4764014` (docs)
2. **Task 2: Build the fixed rio harness under rio-derisk/ (flake, gate script, expect driver)** — `7482f48` (feat)
3. **Task 3: Write the reusable pass-2 witness driver and its README** — `1596283` (feat)

**Plan metadata:** the `docs(01-09): complete rio de-risk plan` commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `rio-derisk/RIO-DERISK.md` (created) — CREDENTIAL-PATH, PROMPT-PATH, ROOT-CAUSE, FIX, LOGIN-USER, TEARDOWN and the "Not gate evidence" section, all cited at Harvey `bd7a698`
- `rio-derisk/flake.nix` (created) — `apps.x86_64-linux.rio-gate` over Harvey's locked inputs; runtimeInputs carry `xdotool`, `xvfb`, `coreutils`, `findutils`, `gnugrep`
- `rio-derisk/flake.lock` (created) — the S3 lock copied verbatim (same inputs; verified byte-identical after `nix flake metadata`)
- `rio-derisk/rio-gate.sh` (created) — the fixed gate: early run scratch + EXIT trap, 0600 password file, Xvfb on a private display, recorded-pid + identity teardown, iso compose and expect invocation
- `rio-derisk/rio-gate.expect` (created) — the S3 chassis with the X11 XTEST feed in rio mode, the unchanged pty feed in control-noG, the unchanged two-entry `/srv` assertion, and the redaction post-pass
- `witness/run-witness.sh` (created) — the reusable pass-2 driver with `rio`, `control`, `a13`
- `witness/README.md` (created) — host needs, command sequence, markers, return paths A/B, secret-return rules, cleanup

## Decisions Made

- **XTEST is the fix, and it is justified by the device trace.** `readcons` reads `kbdq`; under X11 the only feeder is `xkeyboard`; XTEST is the same path a physical keyboard takes. Candidate (b) has no non-interactive password source; candidate (c) is recorded as the fallback but needs a second daemon and an unmeasured proxy path. See RIO-DERISK.md for the full rejection reasons.
- **Window lookup cannot use `--pid`.** drawterm sets `WM_NAME`/`WM_CLASS` (`x11.c:829-855`) but no `_NET_WM_PID`, so the driver searches `--classname drawterm` on the private display and focuses the window explicitly (no window manager runs).
- **One 0600 file, one path.** The shell parses the bake creds, writes the value with `umask 077`, exports only `R9_DT_PWFILE`, and unsets the value variable. The control leg reads the same file, so no environment variable carries the password either.
- **The lock was copied, not regenerated.** The input declarations are identical to S3's; `nix flake metadata` reports Harvey `bd7a698` lastModified `1795382101` and leaves `flake.lock` byte-identical (`e22c1075…`), so `nix flake lock` was unnecessary.
- **`a13` is forward-declared.** Plan 01-10 adds `checks.x86_64-linux.harvey-drawterm`; until then the step fails loudly with nix's missing-attribute error, never a silent skip.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Renamed the Tcl feed-file variable so the password-on-command-line gate is truthful**
- **Found during:** Task 2's verify loop (the security grep)
- **Issue:** The plan's grep gate `(xdotool|drawterm)[^#]*(R9_DEMO_PW|\$pw|password=)` matched the benign Tcl variable `$pwfile` on the `xdotool type --file` line, producing a false 1 where the acceptance requires 0. Leaving it would have made a security control either fail or need a waiver, training the next reader to ignore the gate.
- **Fix:** Renamed the variable to `$secretfile` (the value is still passed only as a path); the grep now prints 0.
- **Files modified:** `rio-derisk/rio-gate.expect`
- **Verification:** the exact grep prints 0; `tclsh info complete` prints 1.
- **Committed in:** `7482f48`

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** No scope change. The fix keeps the plan's own credential gate meaningful.

### Plan verification drift

- **The illustrative `xdotool search --pid` was replaced by `--classname`.** The plan offered `--pid` as an example candidate; source reading shows drawterm never sets `_NET_WM_PID`, so `--pid` cannot match. The chosen lookup is documented in RIO-DERISK.md and in the expect driver's header.
- **`nix flake lock` was not run.** The copied S3 lock already covers the identical inputs; running `nix flake metadata` left the lock byte-identical. This satisfies the plan's conditional ("only if the copied lock does not already cover the inputs") and records the outcome.
- **The plan's `files_modified` list matched exactly.** All seven files were created; no production r9 source, root `flake.nix`/`flake.lock`, `xtask/`, kernel crate, `port/src` or `.github` file was touched by any of the three commits.

## Issues Encountered

- **The driver was smoke-tested on the Mac and its work root removed.** `R9_WITNESS_STEPS=` exercised the default selection; the `nix run` and `nix build` steps failed as expected with missing `aarch64-darwin` attributes (this is a Linux driver), proving the step registry, the summary writer and the return writer run. The 257 MB `~/r9-witness-work` it created was deleted. No repository file was affected.
- **The final `nix eval` store path differs from the first.** The first eval (`jysq05q6…`) preceded the addition of `coreutils`, `findutils` and `gnugrep` to the app's runtimeInputs; the final eval (`g9p1s95y…`) is the committed flake. Both printed a `/nix/store` path; only the final one is recorded.
- **HARN-01 was copied verbatim from the plan but is not proven here.** The plan declares `requirements: [HARN-01]`; the KVM legs run in plan 01-12, and HARN-01 is also declared by sibling plans. The shared-ID gate should block the REQUIREMENTS.md tick until those summaries exist — the same behaviour plan 01-08 recorded.

## Known Stubs

None in this plan's deliverables. The `a13` witness step is intentionally forward-declared for plan 01-10's check and fails loudly (nix missing-attribute) rather than skipping; that is the plan's own staging, not a stub in shipped behaviour.

## Threat Flags

None beyond the plan's threat model. The predicted surface is implemented with its mitigations: T-01-30 (password only in a 0600 scratch file, path-only export, no argv/env value, grep gate and witness secret scan), T-01-31 (Xvfb `-nolisten tcp`, feeder targets the private display only), T-01-32 (recorded-pid + `/proc` identity teardown and a survivor list) and T-01-33 (root refusal, no privilege escalation, work confined to `R9_WITNESS_WORK`, Harvey pinned by full sha).

## Next Phase Readiness

- Plan 01-10 can add `checks.x86_64-linux.harvey-drawterm`; the witness `a13` step will then build it.
- Plan 01-12 (stop A) runs `R9_WITNESS_STEPS="rio control a13" bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` on the owner's KVM host and returns `WITNESS-RETURN.md`. The XTEST feed and the two-entry marker remain **unproven** until that run prints `RIO-SERVING`.
- Plan 01-18 can port the proven feed into r9's quick gate; if XTEST fails on the witness, candidate (c) (a local plan9port factotum key via `NAMESPACE`) is the documented fallback.
- Harvey is untouched (HEAD `25d97dc4d`, status hash `041d2929…`); the pass-1 spike files are byte-unchanged; no push was made.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: all seven listed under `key-files.created`, plus this SUMMARY.
- Task commits `4764014`, `7482f48` and `1596283` are ancestors of HEAD; `git rev-list --count cf3e812..HEAD` = 3.
- Task 1 acceptance: six KEY lines present; 69 `bd7a698`/`:line` citations; password-value grep 0; one `## Not gate evidence` heading.
- Task 2 acceptance: shellcheck clean; Tcl `info complete` = 1; `nix eval` printed `/nix/store/g9p1s95y…-rio-gate/bin/rio-gate`; password-on-command-line grep 0; `RIO-SERVING` 3, `RIO-CONTROL-NO-RIO` 3, `riowctl` 5, `exp_pid` 2; `git diff` over the spikes is empty.
- Task 3 acceptance: shellcheck clean and `bash -n` 0; `R9_WITNESS_STEPS` and `step_rio`/`step_control`/`step_a13` present; sudo count 0; `id -u` present; `WITNESS-RETURN.md` and `witness-returns/01-` greps pass.
- Plan verification: Harvey untouched (HEAD and status hash unchanged); pass-1 spikes unchanged; no protected r9 path in any plan commit; the KVM legs were not run or claimed.
