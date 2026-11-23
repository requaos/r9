---
phase: 01-stage-p0-cross-build-harness
plan: 12
subsystem: cross-build-harness-owner-stop
tags: [owner-stop-a, mac-builder, ssh-ng, lima, kvm, rio, control-noG, a13, drawterm, d-13, d-24, d-28, harn-01, harn-07]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-09's fixed headless rio gate (XTEST credential feed, pair marker, control-noG) and reusable witness driver (rio/control/a13); 01-10's harvey flake input, checks.x86_64-linux.harvey-drawterm and docs/harvey-supply.md builder runbook"

provides:
  - "VERDICT mac-builder: PASS — a fresh never-substitutable aarch64-linux derivation built through ssh-ng://r9-lima-builder and imported into the Mac store (the fix for S2's copy-back FAIL)"
  - "VERDICT rio-derisk-kvm: FAIL — the D-28 pair never appeared; RIO-FAIL rio-not-serving after 242 s"
  - "VERDICT control-noG-kvm: PASS — RIO-CONTROL-NO-RIO with the unchanged -G discriminator"
  - "VERDICT a13-x86_64-linux: PASS — /nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06"
  - "Redacted, line-trimmed stop-A evidence under witness/evidence/stop-a/, each excerpt <= 200 decisive lines"

affects: [01-13, 01-18, HARN-01, D-13, D-24, D-28]

actuals:
  tokens: 2862
  tasks: 2
  commits: 1

plan_head_before: 1610013947d8da9d5f9749b53c81bb1ae134312e
plan_head_after: e5c0faabc3305048f6da32cfdc28193e6ef1f621

tech-stack:
  added: []
  patterns:
    - "A remote-builder verdict is a fresh never-substitutable derivation whose printed path the Mac's own store accepts (nix path-info) and whose content matches"
    - "A witness verdict is recorded only from returned summary.tsv rows; excerpts are trimmed to decisive lines and scanned for credentials before commit"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/rio.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/control.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/a13.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/env.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/mac-builder-probe.txt
  modified:
    - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/RIO-DERISK.md

key-decisions:
  - "rio-derisk-kvm is FAIL: the return has no RIO-SERVING and does contain RIO-FAIL rio-not-serving. The pair marker is unchanged, and no cause is inferred from the absence of a named dt-feed-timeout — the returned log does not distinguish a login that did not complete from one that completed without rio serving."
  - "control-noG-kvm PASS keeps the discriminator meaningful: the -G leg printed RIO-CONTROL-NO-RIO with no rio entry in the serial listing."
  - "The STOP RULE is invoked: no plan 01-13 or later starts; the owner chooses among (a) a new de-risk iteration on the feed, (b) the research fallback marker as a new CONTEXT decision, or (c) a HARN-01 re-scope. The executor chooses none and never edits the marker."
  - "A sixth evidence file (mac-builder-probe.txt) backs the mac-builder VERDICT line with a real committed path; the plan's files_modified list named only five evidence files."

patterns-established:
  - "Owner-stop close-out: four machine-greppable VERDICT lines, each with a committed non-empty EVIDENCE path, plus an explicit STOP RULE paragraph in the findings doc"
  - "Evidence hygiene: the plan's own grep counts unredacted credential terms and must print 0 before the evidence commit"

requirements-completed: []

coverage:
  - id: D1
    description: "Mac builder proof: a fresh never-substitutable aarch64-linux derivation built through the registered ssh-ng builder and imported into the Mac store"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "witness/evidence/stop-a/mac-builder-probe.txt: build exit 0, nix path-info exit 0, output content ok"
        status: pass
    human_judgment: false
  - id: D2
    description: "KVM rio verdict recorded from the returned summary row: FAIL, with redacted evidence and the D-28 marker unchanged"
    requirement: HARN-01
    verification: []
    human_judgment: true
    rationale: "The verdict is deterministic from the returned log; how to make the headless leg pass is an owner decision and the phase is halted."
  - id: D3
    description: "control-noG and a13 verdicts: the negative control discriminates and the x86_64-linux drawterm check builds"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "witness/evidence/stop-a/control.log: RIO-CONTROL-NO-RIO; witness/evidence/stop-a/a13.log: /nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06"
        status: pass
    human_judgment: false
  - id: D4
    description: "STOP RULE: execution halted on the rio FAIL and the owner was asked to choose among three options; no later plan started"
    verification: []
    human_judgment: true
    rationale: "Which option proceeds is the owner's decision; the executor chooses none."

duration: ~1 min of continuation executor time (owner run ~14 min wall including the builder setup)
completed: 2026-11-23
status: halted
---

# Phase 01 Plan 12: Owner stop A — builder, push, rio/control/a13 witness Summary

**The Mac now builds never-substitutable aarch64-linux derivations through the registered `ssh-ng://r9-lima-builder` and imports them into its own store, and the fixed headless rio leg ran on the owner's x86_64 KVM host: `rio` is FAIL (`RIO-FAIL rio-not-serving`, no pair) while the unchanged `control-noG` PASSes (`RIO-CONTROL-NO-RIO`) and the `a13` drawterm check builds — so execution stopped under HARN-01 and the owner is asked for a decision.**

## Performance

- **Duration:** ~1 min of continuation executor time (2026-11-23T05:40:33Z to 2026-11-23T05:41:16Z); the owner's stop-A run covered the rest — witness preflight 05:24:16Z, return 05:29:55Z (rio 242 s, control 97 s, a13 0 s) plus the builder setup
- **Started:** 2026-11-23T05:40:33Z (continuation)
- **Completed:** 2026-11-23T05:41:16Z
- **Tasks:** 2 of 2 (Task 1 was the owner's three-part action; Task 2 executed here)
- **Files modified:** 7 (1 modified, 6 evidence files created)

## Accomplishments

- **`VERDICT mac-builder: PASS`.** A fresh never-substitutable probe (`r9-builder-probe-1795412439`, system `aarch64-linux`) built through the registered `ssh-ng://r9-lima-builder`; the Mac printed `/nix/store/bjjjy4hvdidvkw84wfl6md246lsw00iq-r9-builder-probe-1795412439`, `nix path-info` exited 0, and its content read `ok`. Because this host is aarch64-darwin and cannot realise an aarch64-linux derivation locally, the result proves the daemon-side build-and-import path that fixes S2's copy-back FAIL.
- **`VERDICT rio-derisk-kvm: FAIL`.** The returned `summary.tsv` row is `rio 1 242`. The serial `ls /srv` listing showed only `/srv/boot` and `/srv/factotum`, never the `rio.<user>.<pid>` + `riowctl.<user>.<pid>` pair, and the driver printed `RIO-FAIL rio-not-serving — no rio.<user>.<pid> + riowctl.<user>.<pid> pair in the serial /srv listing within the budget`. No `RIO-FAIL dt-feed-timeout` appears, so the named failure is the missing pair; the returned log does not distinguish a login that did not complete from one that completed without rio serving.
- **`VERDICT control-noG-kvm: PASS`.** `control 0 97`; the serial listing showed only `/srv/boot` and `/srv/factotum`, and the driver printed `RIO-CONTROL-NO-RIO` (no `RIO-FAIL control-saw-rio`), so the negative control still discriminates.
- **`VERDICT a13-x86_64-linux: PASS`.** `a13 0 0`; `checks.x86_64-linux.harvey-drawterm` built `/nix/store/y8h5ana2drnzs2fdx6fxgr74bwmzi8y9-drawterm-19.06`, confirming the nixpkgs follow holds on x86_64-linux.
- **Redacted evidence committed.** `witness/evidence/stop-a/{rio.log,control.log,a13.log,env.txt,summary.tsv,mac-builder-probe.txt}`, each trimmed to at most 200 decisive lines (largest 70). The plan's credential scan prints 0 unredacted `auth-creds|passw` lines; the return's `env.txt` shows `DEV_KVM=writable`, sandbox enabled, nix 2.35.6; `survivors.txt` lists none and `secret-scan.txt` reports clean.
- **STOP RULE invoked.** `rio-derisk-kvm` is FAIL, so no plan 01-13 or later started, the pair marker is unchanged, and the owner was returned three options. HARN-01 is **not** marked complete.

## Task Commits

Each task was committed atomically:

1. **Task 1: Owner stop A — builder registration (Mac), push (Mac), witness steps rio, control, a13 (Linux)** — owner-executed; no executor commit (the owner pushed `witness-src/01-12` at `24c7c3a01649125b8c0151f6bba1a656139b6ace`; the return is `cf8541744195ae98469cc570c22b088c443f00f0` on the same branch)
2. **Task 2: Verify the builder from the Mac, record the stop-A verdicts and evidence, and stop on a rio FAIL** — `e5c0faa` (docs)

**Plan metadata:** the `docs(01-12): complete owner-stop-A plan (halted on rio FAIL)` commit that carries this SUMMARY.

## Files Created/Modified

- `rio-derisk/RIO-DERISK.md` (modified) — appends `## Witness verdict (stop A)`: four VERDICT lines with EVIDENCE paths, the decisive rio line, the return environment, the teardown finding (none) and the STOP RULE paragraph
- `witness/evidence/stop-a/rio.log` (created) — the driver output and retained serial capture around `RIO-FAIL rio-not-serving` (70 lines)
- `witness/evidence/stop-a/control.log` (created) — the `-G` serial listing ending in `RIO-CONTROL-NO-RIO` (41 lines; the one per-bake credential phrase is `<redacted>`)
- `witness/evidence/stop-a/a13.log` (created) — the built drawterm store path (2 lines)
- `witness/evidence/stop-a/env.txt` (created) — the Linux host preflight (`DEV_KVM=writable`, sandbox true, nix 2.35.6, 16 CPUs, 361 GiB free)
- `witness/evidence/stop-a/summary.tsv` (created) — `rio 1 242`, `control 0 97`, `a13 0 0`
- `witness/evidence/stop-a/mac-builder-probe.txt` (created) — the fresh probe command, printed path, `nix path-info` result and content

## Decisions Made

- **The rio verdict is FAIL and nothing softer.** The plan's rule is PASS only on `RIO-SERVING` with no `RIO-FAIL`; the return has the opposite. The absence of `RIO-FAIL dt-feed-timeout` is noted as a fact, not used to infer a cause, and the marker is untouched (T-01-42).
- **The control and a13 verdicts are PASS from their rows only.** No verdict is given to a step without a row (T-01-41); all three steps have rows.
- **The STOP RULE is an owner decision.** The executor records the halt and presents (a) further feed investigation as a new de-risk iteration, (b) the research fallback marker as a new CONTEXT decision, (c) a HARN-01 re-scope — and chooses none.
- **Evidence paths are repo-root-relative and each is a real committed file**, so the acceptance check can resolve them without guessing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `mac-builder-probe.txt` so the `mac-builder` VERDICT line has a real EVIDENCE path**
- **Found during:** Task 2 step 1
- **Issue:** The plan requires "Four VERDICT lines with EVIDENCE lines whose paths exist and are non-empty", but its `files_modified` list names only five evidence files and none of them records the Mac probe. Pointing `mac-builder` at the Linux `summary.tsv` would be false evidence (T-01-41).
- **Fix:** Wrote `witness/evidence/stop-a/mac-builder-probe.txt` with the exact probe command, build exit, printed store path, `nix path-info` exit and the `ok` content.
- **Files modified:** `witness/evidence/stop-a/mac-builder-probe.txt` (created)
- **Verification:** the file exists, is non-empty (11 lines), and the `EVIDENCE mac-builder:` path resolves to it.
- **Committed in:** `e5c0faa`

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** No scope change; the added file makes the acceptance criterion truthful.

### Plan-vs-return drift

- **The return did not use a `witness-returns/01-12` branch.** The owner pushed the return commit `cf85417` onto `witness-src/01-12`; the precondition's object is present locally (`git show cf85417:<path>` works), so no fetch inside this worktree was needed.
- **The return lives at `spikes/linux-witness/return-01-12/WITNESS-RETURN.md`, not `witness/return-01-12` or the README's `witness/returns/01-12`.** It is self-contained (env, summary, survivors, secret scan and log excerpts are inlined); there is no separate `out/` tree, so the evidence files were built from the inlined content.
- **The plan's `out/` log names differ from the return's.** The return carries `control/control.log` (driver output) plus `control/rio-gate-control-noG.log` (capture), and the same pair for rio; the committed `rio.log`/`control.log` name their source file at the top and keep the decisive lines.

## Issues Encountered

- **The rio run's returned excerpt does not explain the failure.** It shows the serial `/srv` listing without the pair and `RIO-FAIL rio-not-serving`, but no `dt-feed-timeout` and no login transcript. The verdict is therefore exactly the returned failure, and the next investigation needs either a new de-risk iteration with finer capture (option a) or an owner marker decision (option b/c).
- **`requirements-completed` is empty on purpose.** The plan declares `[HARN-01, HARN-07]`, but HARN-01 cannot be met while the headless rio leg fails, and this plan halted. REQUIREMENTS.md was not modified.
- **No push and no Harvey change.** `git reflog --date=iso | grep -c 'push'` = 0 (the owner's push is recorded from their reply); the HARVEY-UNTOUCHED guard prints `HARVEY-UNTOUCHED` at HEAD `25d97dc4d` with status hash `041d2929…`.

## Known Stubs

None in the committed deliverables. The halted capability — a passing headless rio leg — is recorded as the open owner decision above, not as shipped behaviour.

## Threat Flags

None beyond the plan's threat model. The implemented surface matches its mitigations: T-01-40 (every excerpt scanned for `auth-creds`/`password`/`passwd` and the per-bake value; the plan's scan prints 0 unredacted lines, and the return's own scan is clean), T-01-41 (verdicts only from returned rows; every step had one), T-01-42 (explicit STOP RULE; the marker is unchanged), T-01-SC (no package installs; the probe used the locked nix).

## User Setup Required

The Mac builder registration is an owner-applied machine change and is now live (lima ssh port 60022, `/etc/nix/r9-lima-builder`, root `r9-lima-builder` alias, `determinateNix.customSettings`, guest `trusted-users`). Reverts are in `docs/harvey-supply.md` §2. No new `USER-SETUP.md` was generated because no env vars or accounts remain.

## Next Phase Readiness

- **Blocked on the owner's choice.** Plan 01-13 and every later plan are not started: HARN-01 cannot be met with the D-28 marker until the headless leg passes.
- The Mac build path is settled and proven; plans 01-13 onward can rely on Linux derivations building through the registered builder.
- The control and a13 legs are green, so the marker itself is still meaningful and the drawterm build check is real on x86_64-linux.
- If the owner picks option (a), the next de-risk iteration should capture the drawterm login/pair stages more finely; if (b), the fallback marker must be a recorded CONTEXT decision before any gate work; if (c), HARN-01 is re-scoped explicitly.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist and are non-empty: `rio.log` 70 lines, `control.log` 41, `a13.log` 2, `env.txt` 24, `summary.tsv` 3, `mac-builder-probe.txt` 11; `RIO-DERISK.md` modified.
- Task commit `e5c0faa` is an ancestor of HEAD; `git rev-list --count 1610013..HEAD` = 1 (measured against the plan ledger).
- Plan verify 1: `grep -cE '^VERDICT (mac-builder|rio-derisk-kvm|control-noG-kvm|a13-x86_64-linux): (PASS|FAIL)$' RIO-DERISK.md` printed **4**.
- Plan verify 2: `grep -rniE 'auth-creds|passw' witness/evidence/stop-a | grep -vc '<redacted>'` printed **0**.
- Every `EVIDENCE` path resolves to a non-empty committed file; no later plan's commit exists; STATE.md and ROADMAP.md are untouched.
