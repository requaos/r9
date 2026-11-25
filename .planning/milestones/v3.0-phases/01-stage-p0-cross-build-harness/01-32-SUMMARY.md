---
phase: 01-stage-p0-cross-build-harness
plan: 32
subsystem: cross-build-harness-rio-derisk
tags: [rio, drawterm, xvfb, xdotool, xtest, rio-diag, login-marker, srv, d-28, d-30, d-13, harn-01, kvm, witness-driver, stop-a2]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-09's fixed headless rio gate (XTEST feed, D-28 pair, control-noG) and reusable witness driver; 01-12's stop-A verdicts and evidence (Mac builder PASS, rio FAIL, control/a13 PASS)"

provides:
  - "VERDICT rio-login-kvm: PASS — the X11 feed authenticated; the serial #s marker /srv/r9-login-ok and /srv/r9-login-pid.51 appeared"
  - "VERDICT rio-derisk-kvm: PASS — /srv/rio.glenda.76 + /srv/riowctl.glenda.76 on the diag leg and /srv/rio.glenda.74 + /srv/riowctl.glenda.74 on the rio leg; RIO-SERVING"
  - "VERDICT control-noG-kvm: PASS and a13-x86_64-linux: PASS — carried from stop A (not rerun)"
  - "VERDICT rio-start: LOGIN-OK-RIO-SERVING — the login and the rio start are now separately proven"
  - "A rio-diag gate leg with a serial #s login marker, X11 window/PNG evidence, a documented manual plumber; rio fallback for the 19-10 draw-guard residual, and file-tail path normalization"

affects: [01-13, HARN-01, D-28, D-30]

actuals:
  tokens: 17244
  tasks: 3
  commits: 11

plan_head_before: 7b2e9b73708ed4d51e422e306058c09b1e336614
plan_head_after: 9770d34fac052674ce1cfc814d7d30c4f43d0847

tech-stack:
  added:
    - "xwd, imagemagick, xprop, xwininfo (nixpkgs, x86_64-linux) — the rio-diag window and PNG capture set"
  patterns:
    - "A login marker that must cross namespaces rides the kernel-wide #s table as an unposted service entry; the entry NAME is the payload because devsrv create is OWRITE-only and its write posts a decimal fd"
    - "A diagnostic capture is taken before any input and after Return only; raw-mode readcons never echoes, so no secret is ever on screen"
    - "A serial listing is normalized with file tail before matching, because ls prints full /srv/... paths on this image"
    - "An owner stop iteration records every gate fix as its own commit and stages the return under spikes/linux-witness/return-<plan>/"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a2/rio-diag.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a2/serial-tail.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a2/x11.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a2/env.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a2/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/return-01-32/ (iteration staging: WITNESS-RETURN.md, out logs, x11.txt, two PNGs)
  modified:
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/rio-gate.sh
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/rio-gate.expect
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/flake.nix
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/RIO-DERISK.md
    - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
    - .planning/phases/01-stage-p0-cross-build-harness/witness/README.md
    - .planning/phases/01-stage-p0-cross-build-harness/01-32-PLAN.md

key-decisions:
  - "The login marker is not /tmp and not an ordinary file: it is an unposted #s service entry created with /amd64/bin/syscall create at /srv/r9-login-ok and /srv/r9-login-pid.<pid>. r9's devsrv create is OWRITE-only and its write expects a decimal fd, so echo/login text cannot be written; the entry name carries the payload and the serial console's ls /srv sees it through the kernel-wide table (port/src/devsrv.rs:59)."
  - "The marker sits at the TOP of the cpu profile, not before exec rio. The stop-A2 screenshots show the password was accepted (the typed prompt becomes a cpu% shell) while the profile's test -e /mnt/term/dev/draw guard skipped exec rio (the 19-10 residual); a marker before exec rio was a false LOGIN-FAIL."
  - "The 19-10 residual is covered by the documented manual start: when the first serial /srv poll shows no pair the driver types plumber; rio into the authenticated cpu% shell. It is not a marker change; the D-28 pair remains the only PASS condition and the fallback never fires when the profile auto-started rio."
  - "The parser now normalizes every ls /srv token with file tail before matching. The stop-A2 manual-start run already carried /srv/rio.glenda.74 and /srv/riowctl.glenda.74, but the first driver's bare-name regex missed the full paths the image prints."
  - "PNG capture prefers magick (ImageMagick 7) over the convert shim; a runtime conversion failure is a warning, not an abort. The serial #s marker is the authoritative login evidence and x11.txt the supplementary identity/geometry record."
  - "control-noG and a13 were not rerun at stop A2: their stop-A rows (control 0 97, a13 0 0) remain valid and are carried from witness/evidence/stop-a/, never re-inferred from the stop-A2 return."
  - "requirements-completed is empty on purpose: HARN-01 needs the one documented cargo xtask command, which plans 01-16/01-18 still own, and the shared-ID gate would block the mark while those siblings have no SUMMARY. The rio leg itself is now green."

patterns-established:
  - "Owner-stop evidence: five machine-greppable VERDICT lines, each with a committed non-empty EVIDENCE path; carried verdicts name the stop they came from"
  - "Diagnostic evidence: the serial listing proves presence, x11.txt proves window identity/focus, and the PNGs prove the accepted session; no capture between the first keystroke and Return"
  - "Every gate defect found during an owner run is a separate fix commit, so the rerun diff is auditable"

requirements-completed: []

coverage:
  - id: D1
    description: "rio-diag mode: serial #s login marker, X11 window identity and two PNG captures, plus the unchanged D-28 pair poll"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "witness/evidence/stop-a2/rio-diag.log (RIO-DIAG login-ok, RIO-SERVING); witness/evidence/stop-a2/x11.txt (window 4194305, geometry, xprop/xwininfo before typing and after Return)"
        status: pass
    human_judgment: false
  - id: D2
    description: "rio-login-kvm PASS: the X11 password feed authenticated and the profile reached its login marker"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "witness/evidence/stop-a2/rio-diag.log: /srv/r9-login-ok + /srv/r9-login-pid.51 in the serial listing; RIO-DIAG login-ok"
        status: pass
    human_judgment: false
  - id: D3
    description: "rio-derisk-kvm PASS: the D-28 pair is present on the diag and rio legs and both drivers print RIO-SERVING"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "witness/evidence/stop-a2/serial-tail.log: /srv/rio.glenda.76 + /srv/riowctl.glenda.76 and /srv/rio.glenda.74 + /srv/riowctl.glenda.74"
        status: pass
    human_judgment: false
  - id: D4
    description: "control-noG and a13 remain PASS, carried from the stop-A evidence"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "witness/evidence/stop-a/control.log (RIO-CONTROL-NO-RIO); witness/evidence/stop-a/a13.log (/nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Root cause recorded: password accepted, chdir line benign, draw guard skipped exec rio, manual plumber; rio produced the pair"
    requirement: HARN-01
    verification: []
    human_judgment: true
    rationale: "The interpretation reads the two PNGs (the typed prompt and the accepted cpu% shell with the two syscall creates); the pair itself is machine-proven, but whether the screenshots show an accepted session is a reading judgment."

duration: ~1 h 10 min (Task 1 plus the owner's stop-A2 iteration and this close-out)
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 32: Rio de-risk iteration 2 — separate the X11 login from rio startup Summary

**The rio de-risk is green on the owner's x86_64 KVM host: the diagnostic bake marked the login at the profile top with the kernel-wide `#s` entries `/srv/r9-login-ok` + `/srv/r9-login-pid.51`, the X11 feed authenticated (the typed prompt became a `cpu%` shell), and the documented manual `plumber; rio` produced `/srv/rio.glenda.76` + `/srv/riowctl.glenda.76` — `RIO-DIAG login-ok` and `RIO-SERVING` in one run — while `control-noG` and `a13` stay PASS, carried from stop A. The D-28 pair marker was never weakened and is now proven on KVM.**

## Performance

- **Duration:** ~1 h 10 min wall (Task 1 from ~00:44 to 00:49, the owner's stop-A2 iteration through 01:38, the parser fix and this close-out to 01:53)
- **Started:** 2026-11-23T05:44Z (worktree spawn; Task 1 commit 00:48:45 local)
- **Completed:** 2026-11-23T06:53Z (01:53 local)
- **Tasks:** 3 of 3 (Task 1 executed; Task 2 was the owner's KVM run; Task 3 this close-out)
- **Files modified:** 12 authored files, 867 insertions / 54 deletions (68,974 chars ≈ 17,244 tokens); this excludes the owner-return staging tree `spikes/linux-witness/return-01-32/` (315,929 chars of raw logs and PNGs, which is data, not authored work)

## Accomplishments

- **`VERDICT rio-login-kvm: PASS`.** The diagnostic log prints `RIO-DIAG login-ok`; the serial console's `ls /srv` lists `/srv/r9-login-ok` and `/srv/r9-login-pid.51`. `x11.txt` records the drawterm window `4194305` on `:1`, 768×576, `WM_CLASS "drawterm","Drawterm"`, `WM_NAME "drawterm"`, and the focus transition (the before capture's `getwindowfocus` returns the root-window bug; the after capture returns `4194305`).
- **`VERDICT rio-derisk-kvm: PASS`.** The diag leg's serial listing carries `/srv/rio.glenda.76` + `/srv/riowctl.glenda.76`; the rio leg's carries `/srv/rio.glenda.74` + `/srv/riowctl.glenda.74`. Both drivers print `RIO-SERVING`, both steps exit 0 (`rio-diag 0 161`, `rio 0 162`).
- **`VERDICT rio-start: LOGIN-OK-RIO-SERVING`.** The login and the rio start are now separately proven: the marker answers "did the feed reach the profile"; the pair answers "did rio serve". No weaker marker was adopted.
- **`VERDICT control-noG-kvm: PASS` and `VERDICT a13-x86_64-linux: PASS`, carried.** `control 0 97` with `RIO-CONTROL-NO-RIO` and `a13 0 0` with `/nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06` come from the stop-A evidence; they were not rerun, and no stop-A2 row is invented for them.
- **Confirmed root cause and fix.** The password was accepted (the before PNG shows `glenda@DEMO-harvey password:`; the after PNG shows the accepted session with two `syscall: return 4, no error` marker creates and a `cpu%` shell). `cpu: failed to chdir to '/home/req/code/r9'` is benign — the guest's `/` fallback still sources `/usr/glenda/lib/profile`, which is why the marker fired. The profile's `test -e /mnt/term/dev/draw` guard then skipped `exec rio` (the 19-10 residual), so the driver's documented manual `plumber; rio` produced the pair. `ls /srv` prints full `/srv/...` paths on this image, so the parser now normalizes every token with `file tail`.
- **The D-28 marker is untouched and now proven on KVM**: the pair is still the only PASS condition, `control-noG` is unchanged, and the diagnostic cannot produce a PASS from the login marker alone.
- **Evidence set committed and clean.** `witness/evidence/stop-a2/{rio-diag.log,serial-tail.log,x11.txt,env.txt,summary.tsv}`, each ≤200 decisive lines; the plan's credential scan prints 0 unredacted `auth-creds|passw` lines; the return's own secret scan is clean and `survivors.txt` lists none.

## Task Commits

Each task/iteration step was committed atomically on `worktree-agent-p01-32-1795412673`:

1. **Task 1: rio-diag gate leg, serial marker, X11 capture, witness step** — `8b7a71f` (feat)
2. **Stop-A2 iteration fixes and returns:**
   - `7a0f6d2` (fix) dispatch hyphenated witness steps to their functions
   - `bdfca24` (witness) stop A2 returns
   - `ebace4f` (witness) stop A2 diagnostic returns
   - `8e1974c` (fix) prefer magick, keep PNG capture non-fatal
   - `02ad79f` (witness) stop A2 diagnostic rerun
   - `0b37326` (witness) stop A2 X11 capture
   - `6fb6c60` (fix) start rio manually when the profile draw guard skips it
   - `364f78a` (witness) stop A2 manual-start returns
   - `46d0677` (fix) normalize full `/srv` paths before matching
3. **Task 3: verdicts, root cause, redacted evidence** — `9770d34` (docs)
4. **Plan metadata:** the `docs(01-32): complete …` commit that carries this SUMMARY.

The final return is `c95ae4494056cafad66d3121951762590a796c3c` on `witness-src/01-32`; it was read with `git show`, never merged. The local `spikes/linux-witness/return-01-32/` staging tree holds the manual-start return; the final parser-fix return's decisive excerpts are the committed `witness/evidence/stop-a2/` set.

## Files Created/Modified

- `rio-derisk/rio-gate.sh` (modified) — `RIO_MODE=rio-diag` accepted; the staged profile is patched at the top with `/amd64/bin/syscall create /srv/r9-login-ok` and `/srv/r9-login-pid.$pid`, backed up under `build/x86_64/r9-diag-profile.orig` and restored on every exit path; capture-tool preflight; Xvfb for the diag leg
- `rio-derisk/rio-gate.expect` (modified) — diag env/contract checks; `xwait_window`/`xfeed_type` split; `diag_capture` (x11.txt) and `capture_png` (magick → convert → import); the manual `plumber; rio` fallback after the first empty poll; `file tail` normalization; `RIO-DIAG login-ok` / `RIO-FAIL login-not-reached` plus the unchanged pair verdict
- `rio-derisk/flake.nix` (modified) — `xwd`, `imagemagick`, `xprop`, `xwininfo` in the app `runtimeInputs`
- `rio-derisk/RIO-DERISK.md` (modified) — the diagnostic design, the confirmed root cause and fixes, and the five stop-A2 VERDICT lines with EVIDENCE paths
- `witness/run-witness.sh` (modified) — the `rio-diag` step, the hyphen dispatch fix, and the restore-before-reuse guard for a killed diagnostic's staged profile
- `witness/README.md` (modified) — the `rio-diag` step, the marker contract and interpretation, the stop-A2 run/return commands
- `01-32-PLAN.md` (modified) — the marker corrected to the serial `#s` table, the `.program` flake check, and the manual-start/parser corrections recorded during the iteration
- `witness/evidence/stop-a2/*` (created) — the trimmed, redacted evidence set
- `spikes/linux-witness/return-01-32/*` (created during the iteration) — the owner's returned WITNESS-RETURN.md, per-step logs, `x11.txt` and the two PNGs

## Decisions Made

- **The marker is an `#s` entry, not a file.** `echo login-ok >/srv/...` cannot work: devsrv create is OWRITE-only and its write expects a decimal fd; an unposted entry is listed but unreadable. `syscall create` performs just the create, and the pid rides the second entry's name.
- **The marker goes at the top of the profile.** The stop-A2 screenshots proved the login was accepted while the draw guard skipped `exec rio`; a marker before `exec rio` would have been a false `login-not-reached`. The pair still answers the rio question.
- **The 19-10 residual is handled by the documented manual start**, not by changing the marker or the profile's guard. The fallback fires only after an empty first poll and is visible in the log.
- **The parser normalizes full paths with `file tail`** for both the pair and the marker; the first manual-start run proved the pair existed and was missed by the bare-name regex.
- **PNG capture is supplementary and non-fatal at runtime**; the serial marker is authoritative. `magick` is preferred over the IM7 `convert` shim.
- **control/a13 are carried, not re-inferred.** The stop-A2 return ran `rio-diag` and `rio` only.

## Deviations from Plan

### Auto-fixed Issues

**1. [Orchestrator design correction] The login marker crosses namespaces through `/srv`, not `/tmp`**
- **Found during:** Task 1
- **Issue:** a per-process `/tmp` marker is invisible to the serial console, so it cannot prove the login from `ls /srv`.
- **Fix:** the marker is created in the kernel-wide `#s` table; the serial console lists it with the same mechanism as the D-28 pair.
- **Files modified:** `rio-gate.sh`, `rio-gate.expect`, `RIO-DERISK.md`, `README.md`, `01-32-PLAN.md`
- **Commit:** `8b7a71f`

**2. [Rule 1 – Bug] `echo login-ok >/srv/<name>` cannot work on r9's devsrv**
- **Found during:** Task 1
- **Issue:** create is OWRITE-only and the write side posts a decimal fd; text writes fail and unposted entries cannot be read. The original plan wording was a regular-file assumption.
- **Fix:** `/amd64/bin/syscall create` creates the entry; the entry name is the payload; the pid rides `r9-login-pid.<pid>`. Documented with `port/src/devsrv.rs` citations.
- **Files modified:** `rio-gate.sh` (the first Task 1 form used `syscall create`; the continuation hardened the absolute path)
- **Commit:** `8b7a71f`

**3. [Rule 1 – Plan defect] The plan's flake check named `apps.x86_64-linux.rio-gate.name`, which does not exist**
- **Found during:** Task 1 static checks
- **Issue:** the app attrset has `type`/`program` only.
- **Fix:** the check (and the plan) uses `.program`; it prints the store path and exits 0.
- **Files modified:** `01-32-PLAN.md`
- **Commit:** `8b7a71f`

**4. [Rule 1 – Bug] Hyphenated witness steps were never dispatched**
- **Found during:** the first owner stop-A2 run
- **Issue:** `run_one` calls `"step_${step}"`; for `rio-diag` that is `step_rio-diag`, an invalid function name, so the step aborted before running.
- **Fix:** dispatch hyphenated names via the underscored function (`step_rio_diag`).
- **Commit:** `7a0f6d2`

**5. [Rule 1 – Bug] ImageMagick 7 has no default `convert`; a failed PNG conversion aborted the diagnostic**
- **Found during:** the diagnostic rerun
- **Issue:** the nixpkgs ImageMagick 7 package exposes `magick` (and `convert` as a deprecated shim); a runtime conversion failure aborted before the serial marker check.
- **Fix:** prefer `magick`, fall back to `convert`/`import`; a runtime conversion failure is a `diag capture skipped` warning. A missing tool at startup is still a named `RIO-FAIL diag-capture`.
- **Commit:** `8e1974c`

**6. [Rule 2 – Missing critical] The marker before `exec rio` was a false LOGIN-FAIL**
- **Found during:** the stop-A2 screenshots
- **Issue:** the login was accepted (the typed prompt became a `cpu%` shell) but the profile's draw guard skipped `exec rio` (the 19-10 residual), so the marker never fired.
- **Fix:** the marker moved to the top of the profile; the driver types the documented manual `plumber; rio` after the first empty poll. The D-28 pair stays the only PASS condition.
- **Commit:** `6fb6c60`

**7. [Rule 1 – Bug] `ls /srv` prints full `/srv/...` paths and the pair regex missed them**
- **Found during:** the manual-start return
- **Issue:** the pair was present (`/srv/rio.glenda.74`, `/srv/riowctl.glenda.74`) but the bare-name regex never matched.
- **Fix:** normalize every token with `file tail` before matching, for the pair and the marker.
- **Commit:** `46d0677`

**8. [Rule 2 – Missing critical] A killed diagnostic could leave the staged profile patched**
- **Found during:** Task 1
- **Issue:** the backup lived in the deleted run scratch dir, so a SIGKILL outran the restore.
- **Fix:** the backup lives at `build/x86_64/r9-diag-profile.orig` (outside the baked tree); the gate restores on every controlled exit and the witness restores before reusing the scratch tree.
- **Commit:** `8b7a71f`

---

**Total deviations:** 8 auto-fixed (1 orchestrator design correction, 5 bugs, 2 missing-critical). **Impact:** the plan's goal is met with the same D-28 marker; the deviations are the stop-A2 diagnosis and the gate hardening it forced.

## Issues Encountered

- **The 19-10 residual remains a known behavioural gap:** on app-baked venues the profile's draw guard skips `exec rio`, so the gate uses the documented manual `plumber; rio`. The pair is proven; a future plan may make the auto-start work, but that is not a gate change and was not attempted here.
- **The stop-A2 return's `summary.tsv` has two rows (`rio-diag`, `rio`).** `control` and `a13` were not rerun; their rows live in `witness/evidence/stop-a/summary.tsv` and are cited from there. No stop-A2 row is fabricated for them.
- **Five owner cycles were needed** (step dispatch, PNG tooling, the draw guard, the path parser). Each is its own commit, so the rerun diff is auditable.
- **The local `spikes/linux-witness/return-01-32/` tree is the manual-start return, not the final parser-fix return** (`c95ae44`, not merged by instruction). The final return's decisive excerpts are the committed `witness/evidence/stop-a2/` files.

## Known Stubs

None in the committed deliverables. The manual-start fallback is a documented remediation for the 19-10 residual, not a stub; it is exercised and proven by the stop-A2 pair.

## Threat Flags

None beyond the plan's threat model. T-01-30: every text excerpt was re-scanned for `auth-creds`, `password`, `passwd` and the DEMO value, and the plan's scan prints 0; the gate's own redaction post-pass mapped 0 occurrences. T-01-32: the password is never in argv, an exported variable, a log, an x11.txt line or an image (raw-mode `readcons` does not echo; captures are before typing and after Return). T-01-SC: no package installs; the tools come from the flake's locked nixpkgs.

## User Setup Required

None. The owner's KVM host already has the flake-provided tools; the two new PNG/window tools ride the app's `runtimeInputs`.

## Next Phase Readiness

- **The rio KVM leg is green**: HARN-01's rio evidence exists, but HARN-01 is not marked complete because its one documented `cargo xtask` command is still owned by plans 01-16/01-18, and the shared-ID gate blocks the mark until those siblings produce SUMMARYs.
- **Plan 01-13 and later can proceed once the orchestrator merges this worktree and unblocks them.** The rio gate, the diagnostic leg and the witness driver are all in place; the D-28 pair is proven, and the control/a13 legs remain green.
- The 19-10 auto-start residual is documented in `RIO-DERISK.md` with the manual fallback; it does not block the phase but should be revisited if a future gate must prove the auto-start path itself.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist and are non-empty: `witness/evidence/stop-a2/rio-diag.log` (40 lines), `serial-tail.log` (40), `x11.txt` (52), `env.txt` (24), `summary.tsv` (2).
- Commits are ancestors of HEAD: `8b7a71f`, `7a0f6d2`, `bdfca24`, `ebace4f`, `8e1974c`, `02ad79f`, `0b37326`, `6fb6c60`, `364f78a`, `46d0677`, `9770d34`; `git rev-list --count 7b2e9b7..HEAD` = 11 (measured against the plan ledger).
- Plan verify 1: `grep -cE '^VERDICT (rio-login-kvm|rio-derisk-kvm|control-noG-kvm|a13-x86_64-linux|rio-start):' rio-derisk/RIO-DERISK.md` printed **8** (the four stop-A lines, the five stop-A2 lines; the stop-A `rio-derisk-kvm: FAIL` is marked historical and superseded).
- Plan verify 2: `grep -rniE 'auth-creds|passw' witness/evidence/stop-a2 | grep -vc '<redacted>'` printed **0**.
- Every `EVIDENCE` path resolves to a non-empty committed file; the return branch `c95ae44` was read with `git show`, never merged; STATE.md and ROADMAP.md are untouched; no plan 01-13 commit exists.
