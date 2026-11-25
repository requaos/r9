---
phase: 01-stage-p0-cross-build-harness
plan: 07
subsystem: cross-build-harness-spike
tags: [planning-pass-1, owner-checkpoint, decisions, d-22, d-04, d-11, s1, s2, s3, s4, s5, s6, evidence-correction]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "the six spike findings and the Linux witness verdicts from plans 01-01 to 01-06"

provides:
  - "SPIKE-RESULTS.md: the consolidated verdict matrix, A1-A16 status, seven decisions with evidence and recommendations, machine-change ledger, pass-wide prohibition audit and the deferred list"
  - "the owner's verbatim closing-checkpoint reply, dated 2026-11-23, DECISION 1-7 and PASS-2 READY: yes"
  - "D-04 answered as both mechanisms with lima primary; D-03 proot; D-20 option A; D-17 ps; D-11 serial /srv marker; D-16 member lists; the builder kept for pass 2"
  - "the corrected s3-rio attribution: the tr-ok line is the owner's manual graphical drawterm attempt, headless authentication never completed, and the fix is the generated DEMO password/credential path"
  - "a committed attribution note and corrected S3-FINDINGS so pass 2 does not inherit a false A6 confirmation"

affects: [pass-2 planning, HARN-01, HARN-02, HARN-03, HARN-04, HARN-06, HARN-07, HARN-08, D-03, D-04, D-11, D-16, D-17, D-20, D-22]

actuals:
  tokens: 11125
  tasks: 3
  commits: 3

plan_head_before: 671e1e7afefbcb4514b081a0b354e5ea6151d50b
plan_head_after: fedd167744fb8ff77e9b9da1225fd8b409003c34

tech-stack:
  added: []
  patterns:
    - "A closing checkpoint records the owner's reply verbatim and dated, then one machine-greppable DECISION line per decision"
    - "Evidence attribution is part of the evidence: a witness line produced by a manual attempt is annotated and never counted as gate evidence"
    - "A FAIL stays FAIL when the credential path was missing; it is not softened into a driver-timing issue"
    - "The pass-1 canonical results file supersedes contemporaneous per-plan wording without rewriting the historical plan summary"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md
  modified:
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/SPIKE-RESULTS.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/S3-FINDINGS.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/rio-kvm-linux.log
    - .planning/STATE.md
    - .planning/ROADMAP.md

key-decisions:
  - "D-04: document both Mac builder mechanisms with lima primary. Lima is the only measured venue for case handling and amd64 tiers and needs no third-party login; the native Determinate builder stays the documented alternative."
  - "D-03: use proot for the pass-2 tree derivations, with no unshare or privileged-builder dependency; both measured builders refused the unshare venue."
  - "D-20: build and boot Harvey's own kernel to the root-is-from prompt (option A); the rc prompt through u9fs (option B) is the next increment."
  - "D-17: the in-tree probe target is ps with the S6 recipe; echo would rewrite the boot marker vocabulary."
  - "D-11: keep the two /srv/rio.* and /srv/riowctl.* entries as the marker, but correct the s3-rio evidence: the tr-ok line is the owner's manual graphical attempt and headless authentication never completed."
  - "D-16: plan member-list comparison for all images now; add byte-identical differentials for the six deterministic images only after the hsecstored auth-suite resolver lets bake 2 complete."
  - "Decision 7 (pass-transition half of D-04/D-22): keep the lima VM and guest configuration for pass 2; no revert was requested or performed."
  - "A6 is UNTESTED, not CONFIRMED: the X11 backend is statically proven and the headless drawterm starts, but no headless login completed because the generated DEMO password for glenda was not known."

patterns-established:
  - "Owner replies are quoted verbatim in the canonical results file, then summarized as DECISION n (D-xx) lines that a grep can verify"
  - "When a captured witness line turns out to be a manual attempt, correct the header and add a named attribution note; leave the raw captured lines unchanged"
  - "Pass 1 closes without marking any HARN requirement implemented (D-22); the deferred list is the pass-2 backlog"

requirements-completed: []

coverage:
  - id: D1
    description: "Consolidated SPIKE-RESULTS.md with the 25-leg verdict matrix, A1-A16, seven decisions, machine-change ledger, prohibition audit and deferred list"
    verification:
      - kind: other
        ref: "Task 1 greps: 6 SPIKE ANSWERED lines, 8 required H2 sections, 16 ASSUMPTION lines, all 8 HARN IDs and all 23 D IDs cited (commit 2d5a2ce)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The owner's verbatim closing-checkpoint reply, dated, with DECISION 1-7 and PASS-2 READY: yes"
    verification:
      - kind: other
        ref: "grep -cE '^DECISION [1-7] \\(D-[0-9]{2}\\): ' = 7; grep -c '^PASS-2 READY: yes$' = 1; grep -c 'Pending: Task 2' = 0 (commit fedd167)"
        status: pass
    human_judgment: true
    rationale: "The greps prove the lines exist, not that they faithfully transcribe the owner's words; fidelity is a human judgment."
  - id: D3
    description: "The s3-rio evidence attribution is corrected everywhere: the tr-ok line is the owner's manual graphical attempt, headless auth never completed, and A6 is UNTESTED"
    verification:
      - kind: other
        ref: "SPIKE-RESULTS A6 = UNTESTED; S3-FINDINGS and rio-kvm-linux.log point at rio-kvm-linux.ATTRIBUTION-NOTE.md; the s3-rio verdict remains FAIL (commit fedd167)"
        status: pass
    human_judgment: true
    rationale: "Whether every prose mention now reads correctly is a semantic check against the owner's clarification."
  - id: D4
    description: "No machine revert: the lima VM and guest configuration are kept for pass 2, and the ledger's revert commands remain documented"
    verification:
      - kind: other
        ref: "SPIKE-RESULTS 'State after the closing checkpoint' says kept for pass 2; no revert command was run (commit fedd167)"
        status: pass
    human_judgment: true
    rationale: "The owner's Mac state is outside the repository; the ledger records the decision and the absence of a revert command is an executor assertion."

duration: 17min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 07: Pass-1 closing: consolidated spike results and the owner's decisions Summary

**Pass 1 is closed: six spikes answered across 25 legs, the owner chose both builders with lima primary and accepted five of six other recommendations, and the s3-rio tr-ok line is corrected to the owner's manual graphical attempt, not gate evidence.**

## Performance

- **Duration:** ~17 min of executor time, including a checkpoint pause for the owner's reply (Task 1 commit 02:48:36Z, Task 3 commit 03:02:51Z)
- **Started:** 2026-11-23T02:45:00Z (approx.; the plan ledger was written before the Task 1 commit)
- **Completed:** 2026-11-23T03:02:51Z (Task 3 commit)
- **Tasks:** 3 of 3 (Task 2 was the owner checkpoint; no executor commit)
- **Files modified:** 5 between base `671e1e7` and `fedd167`

## Accomplishments

- `SPIKE-RESULTS.md` holds the whole pass in one place: 25 legs from S1 to S6 with PASS or FAIL and an evidence path each, 20 PASS and 5 FAIL, and zero unanswered legs.
- A1 to A16 are classified: 11 CONFIRMED, 3 REFUTED (A1, A8, A12), and 5 UNTESTED (A6, A10, A13, A14, A16). A15 is settled by D-21.
- Seven decisions were put to the owner with evidence and a recommendation. The owner replied: D-04 both with lima primary; recommendations 2, 3, 4, 6 and 7 accepted as written; D-11's marker kept with a corrected evidence account. `PASS-2 READY: yes`.
- The s3-rio attribution is corrected: the gate's headless drawterm never authenticated because the generated DEMO password for glenda was not known. The `tr-ok glenda@glenda(unknown) -> glenda@bootes` line was the owner's own graphical drawterm login attempt while the run appeared hung. The verdict stays FAIL; the next action is to determine the DEMO password/credential path, fix the headless auth and password feed, and re-run the rio leg with `control-noG` unchanged.
- The correction is recorded in three places: SPIKE-RESULTS.md (A6 UNTESTED and DECISION 5), S3-FINDINGS.md, and `s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md`, with the log header and one curator comment adjusted. The raw captured lines are unchanged.
- The machine-change ledger now records the post-checkpoint state: kept for pass 2, no revert run, and every revert command still documented.
- The pass-wide prohibitions held at both audit points: Harvey untouched (`HARVEY-UNTOUCHED`, same HEAD and status hash), zero r9 protected-path changes, and no push from this executor.

## Task Commits

Each task was committed atomically:

1. **Task 1: Consolidate S1 to S6 into SPIKE-RESULTS.md** - `2d5a2ce` (docs)
2. **Task 2: Owner closing checkpoint (D-04 and recommendations 2-7)** - owner reply recorded, no executor commit
3. **Task 3: Record the owner's decisions and correct the s3-rio evidence** - `fedd167` (docs)

**Plan metadata:** pending (docs: complete plan)

_Note: the checkpoint pause itself was committed as `e03f760` (docs: record the pause at the owner closing checkpoint)._

## Files Created/Modified

- `spikes/SPIKE-RESULTS.md` - verdict matrix, A1-A16, seven decisions, machine-change ledger and its post-checkpoint state, prohibition audit re-run, deferred list, owner reply verbatim, DECISION 1-7, `PASS-2 READY: yes`
- `spikes/S3-FINDINGS.md` - s3-rio runtime paragraph, A5, A6 (now UNTESTED) and the pass-2 implication corrected; the DEMO-password caveat added to the login note
- `spikes/s3/evidence/rio-kvm-linux.log` - header correction and one curator comment; captured output unchanged
- `spikes/s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md` - the owner's clarification quoted verbatim with the effect on the verdict and the next action
- `.planning/STATE.md` - pass-1 position, owner decisions, the S3 attribution correction and the corrected A6 status
- `.planning/ROADMAP.md` - Phase 1 plan count advanced to 7/7 and the 01-07 bullet ticked

## Decisions Made

- **D-04: both, lima primary.** Lima is the only venue measured end to end for case handling (23/23 paths), host-toolchain and libc builds, and the amd64 tier 0/1 builds, and it needs no third-party login. The native Determinate builder is documented as the alternative; its case behaviour remains unmeasured.
- **D-03: proot.** The unshare venue is refused on both measured builders; proot passes on both. Pass 2 must carry S1's four host-toolchain adjustments and S4's two venue ports in the venue definition.
- **D-20: option A.** Build and boot Harvey's own kernel to the `root is from` prompt, reusing `s5/boot-q35.sh`; the rc prompt through u9fs on TCP 564 is the next increment.
- **D-17: ps.** `ps` is staged in every x86_64 chain-family image, absent from the boot path and used at two read-only gate sites. `echo` is not an rc builtin and would execute the replacement throughout cpurf.
- **D-11: keep the serial /srv marker, fix the credential path.** The `-G` control proves the discriminator; the positive leg never authenticated headlessly. The next action is the generated DEMO password/credential path and a re-run, not an expect-prompt tweak.
- **D-16: member lists now, byte differentials later.** Bake 2 cannot complete until the `hsecstored` auth-suite resolver is fixed; do not promise a whole-image byte test that cannot run.
- **Keep the builder.** The owner requested no revert, so the lima VM and its guest-only `trusted-users` line stay for pass 2; the ledger's revert commands remain documented.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Corrected the s3-rio evidence attribution after the owner's clarification**
- **Found during:** Task 3 (recording the owner's decisions)
- **Issue:** Task 1 and the Plan 06 records attributed `tr-ok glenda@glenda(unknown) -> glenda@bootes` to the gate's headless drawterm and marked A6 CONFIRMED. The owner clarified that the line came from their own graphical drawterm login attempt, that they did not know the generated DEMO password, and that the headless login likely never worked. Leaving A6 CONFIRMED would send pass 2 after an expect-prompt fix instead of the credential path.
- **Fix:** Recorded the reply verbatim; changed A6 to UNTESTED in both SPIKE-RESULTS.md and S3-FINDINGS.md; rewrote the D-11 decision evidence, options and recommendation; corrected the rio log header and one curator comment; added `rio-kvm-linux.ATTRIBUTION-NOTE.md` with the owner's paragraph quoted verbatim. The s3-rio verdict remains FAIL.
- **Files modified:** `spikes/SPIKE-RESULTS.md`, `spikes/S3-FINDINGS.md`, `spikes/s3/evidence/rio-kvm-linux.log`, `spikes/s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md`
- **Verification:** the Task 1 and Task 3 greps still pass (6 ANSWERED, 8 sections, 16 ASSUMPTION lines, 7 DECISION lines, 1 `PASS-2 READY: yes`, 0 `Pending: Task 2`); every `tr-ok` mention now states it is not gate evidence.
- **Committed in:** `fedd167`

**2. [Rule 3 - Blocking] Did not run `state advance-plan` past a last-plan phase-complete branch**
- **Found during:** the state update
- **Issue:** `state.advance-plan` sees `Plan: 7 of 7` and all seven summaries and would stamp "Phase complete — ready for verification". Phase 1 is not complete: pass 2 is unplanned by design (D-22).
- **Fix:** Kept `Plan: 7 of 7` and recorded the pass-1 closure and the pass-2 re-plan step directly in STATE.md.
- **Files modified:** `.planning/STATE.md`
- **Verification:** STATE.md says pass 1 is complete and pass 2 planning is the next action; no `Phase complete — ready for verification` status was written.
- **Committed in:** the plan metadata commit

**3. [Rule 2 - Missing Critical] Corrected two stale STATE.md decision bullets that still attributed tr-ok to the gate**
- **Found during:** the state update
- **Issue:** the accumulated-context bullets for Plan 01-06 and the A5/A6/A7/A11 summary still said the rio leg "tr-ok then password-feed timeout" and that drawterm "authenticates". Pass 2 reads STATE.md.
- **Fix:** amended both bullets in place with the 2026-11-23 correction, and added a dedicated decision entry for the attribution correction.
- **Files modified:** `.planning/STATE.md`
- **Verification:** `grep 'tr-ok' .planning/STATE.md` now shows only the corrected account.
- **Committed in:** the plan metadata commit

### Plan verification drift

- The plan's no-push check expects r9 `origin/main` at `116bcc2`; it is `c92a3f4` because Plan 06's owner-approved pull-only commits were pushed so the Linux host could pull them. `116bcc2` is an ancestor; this executor pushed nothing.
- The plan's `files_modified` frontmatter lists only SPIKE-RESULTS.md. The owner's clarification required the S3-FINDINGS and evidence corrections above; those files are listed in the SUMMARY and committed with Task 3.
- The Plan 06 SUMMARY's contemporaneous wording that the rio leg "authenticated (`tr-ok`)" is superseded by the owner's clarification. It is a completed plan's historical record and was not edited; SPIKE-RESULTS.md states the supersession.

---

**Total deviations:** 3 auto-fixed (2 missing critical, 1 blocking)
**Impact on plan:** The evidence correction is required for pass 2 to plan the right rio fix. No scope creep; no machine change.

## Issues Encountered

- **The owner's reply changed an assumption, not just a decision.** A6 had been recorded CONFIRMED from the Plan 06 reading. The owner's explanation moved it to UNTESTED and changed the D-11 next action from expect-prompt timing to the DEMO credential path. The verdict stays FAIL.
- **The s3-rio capture cannot prove headless authentication.** The `tr-ok` line is a manual login attempt made while the run appeared hung; pass 2 must obtain the generated DEMO password and prove the headless path before citing `S3-RIO-SERVING`.
- **The lima VM was not re-inspected.** No revert was requested, so the ledger records the kept state rather than running `limactl`; the revert commands remain valid.

## Known Stubs

None. The five FAILs are complete answers with committed evidence. The s3-rio credential-path fix and the `hsecstored` auth-suite resolver are pass-2 actions recorded in SPIKE-RESULTS.md, not stubs in shipped code.

## Threat Flags

None. The new attribution note contains no credential value; the DEMO password was never known to the executor and is not recorded.

## Next Phase Readiness

- Pass 1 is closed and `PASS-2 READY: yes`. The canonical input for pass 2 is `spikes/SPIKE-RESULTS.md`.
- Re-run `/gsd-plan-phase 1` to write pass 2. It must plan against: proot for the tree derivations, both Mac builders with lima primary, D-20 option A, `ps` as the probe, the serial `/srv` marker after the generated DEMO password/headless-auth fix, and D-16 member lists until bake 2 completes.
- No HARN requirement is implemented in pass 1 (D-22). HARN-01, HARN-02, HARN-03, HARN-04, HARN-05, HARN-06, HARN-07 and HARN-08 all remain for pass 2.
- The lima VM `harvey` and its guest `trusted-users` line remain available for pass 2; the ledger's revert commands are the owner's later option.
- Nothing was pushed in either repository.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- `SPIKE-RESULTS.md` exists; 6 ANSWERED lines, 8 required H2 sections, 16 ASSUMPTION lines, 7 DECISION lines, 1 `PASS-2 READY: yes`, 0 `Pending: Task 2`.
- `S3-FINDINGS.md` and `spikes/s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md` exist; the s3-rio verdict remains FAIL and A6 is UNTESTED.
- Task commits `2d5a2ce`, `e03f760` and `fedd167` are ancestors of HEAD.
- Harvey is untouched (`HARVEY-UNTOUCHED`), the r9 protected-path diff and status are empty, and no ref moved.
