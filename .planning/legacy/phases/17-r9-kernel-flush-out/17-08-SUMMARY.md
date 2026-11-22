---
phase: 17-r9-kernel-flush-out
plan: 08
subsystem: kernel-net
tags: [r9, cr-07, windows-66, triple-fault, tcg, slirp, disposition, maintainer-ratification, records-only, d-15]

requires:
  - phase: 17-r9-kernel-flush-out (17-07)
    provides: the CR-07 root-cause record (sections 1-7), the committed reproduction instrument (boot-r9-stress-x64 + the R9_STRESS_* shape hatches), the open-enriched WINDOWS #66 row, and the refuted-cure honesty this plan's terminus consummates
provides:
  - the completed last-avenue probe: the TCG x slirp N=10 serialized diagnosis series (R9_ACCEL=tcg R9_NET=slirp, fresh uninstrumented pinned-kernel boot per run, host-side -d capture only), NO-FIRE-IN-SERIES recorded as 17-CR07-ROOTCAUSE.md section 8 with the weak-negative semantics (avenue exhaustion, never absence) stated in the record itself
  - the maintainer-ratified CR-07/#66 terminus — ratify-disposition + FLUSH-04 yes, Decided-by Neil Skinner, Decided-at 2026-11-13T14:42:08Z — recorded verbatim as 17-CR07-ROOTCAUSE.md section 9 and executed consistently across all five record surfaces: WINDOWS #66 (waived, 3-part D-15 reason line, resolved_at stamped; counts exactly 50/4 -> 49/5), the 08-ledger CR-07 row (Disposition deferred + ratification cite, stale open-row-carrier parenthetical rewritten), 17-FLUSHOUT.md (both action cells maintainer-ratified, verdict rewritten, exactly one records-only lineage row: 40 -> 41 table lines), the 17-VERIFICATION.md frontmatter overrides block (accepted_by/accepted_at verbatim; every byte outside hash-intact), and REQUIREMENTS.md (FLUSH-01 [x] + Complete, FLUSH-04 [x] + Complete)
  - the WR-03 Source-cell re-cite in the 08 ledger (17-02's both-tiers-green-twice discharge) — the verifier's flagged ledger inconsistency closed on every terminus branch
  - FLUSH-01 and FLUSH-04 complete: Phase 17's requirement set is fully closed, and the #66 reopener conditions + reproduction instrument survive the disposition intact
affects: [Phase 18 (auth substrate — inherits a fully-closed flush-out ledger; the waived #66 class stays live ~8%/run under full 8-way concurrent bulk transfer, a load shape auth's loopback p9sk1 legs do not create), Phase 19 (cpu/drawterm — aan/exportfs over real net is the #66 load family; the venue-conditional reopener applies), any future plan that reaches a context-reaching capture venue or builds non-healing instrumentation]

actuals:
  tokens: 291000    # chars/4 over the realized production diff (1,164,067 chars): retained per-run serial-capture evidence ~276.5k + the record edits themselves ~14.5k; estimate was 55000 — the overrun is retained evidence volume (10 x ~110 KB serials), not prose
  tasks: 3
  commits: 3        # MEASURED: git rev-list --count 17d916360..HEAD (the SUMMARY/metadata commit follows)

# Plan-commit ledger (#3968) — measured
commits: 3
plan_head_before: 17d9163605deeac934b255e938e6590a6cfc5827
plan_head_after: af7a0061065146156559718041c606373c9befaa

tech-stack:
  added: []         # records-only — no packages, no derivations, no kernel bytes, no pin mechanics
  patterns: ["the blocking-human terminus pattern: a live crash class is dispositioned ONLY by a named maintainer acceptance recorded verbatim (section 9's Terminus token keys every closure gate; the halt branches assert their frozen baselines by hash, so a sneaked closure fails exactly like a missing one)", "final-state gates, never edit-transition gates: the Task 3 gates assert tokens/counts/reason-content/hashes of the end state, so re-applying the ratified edits yields identical results"]

key-files:
  created:
    - .planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-probe.log     # the series loop log: 10 [run N] lines + scratch-dir paths + cascade-disposition lines
    - .planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run01..10.serial.log   # 10 per-run serial captures, all green, SeaBIOS x 1 each
    - .planning/phases/17-r9-kernel-flush-out/evidence/17-08-rc-preterminus.sha     # the record-through-section-8 sha256 (RC-EVIDENCE-BASE-INTACT baseline)
    - .planning/phases/17-r9-kernel-flush-out/17-VERIFICATION.md                    # the verifier's 2026-11-13T07:11:20Z report — previously untracked, committed by this plan carrying the invited overrides block
    - .planning/phases/17-r9-kernel-flush-out/17-08-SUMMARY.md
  modified:
    - .planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md   # section 8 (Task 1) + section 9 (Task 3, pure tail append)
    - .planning/WINDOWS.md                                           # #66 open -> waived (3-part D-15 reason + resolved_at)
    - .planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW-DISPOSITION.md   # CR-07 deferred + cite; WR-03 re-cite
    - .planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md         # both action cells + verdict + one 17-08 lineage row
    - .planning/REQUIREMENTS.md                                      # FLUSH-01/FLUSH-04 [x] + traceability Complete

key-decisions:
  - "17-08: the CR-07/#66 terminus is ratify-disposition — decided by Neil Skinner at 2026-11-13T14:42:08Z at the plan's blocking-human checkpoint, with FLUSH-04: yes in the same decision. The waiver is of the CURE this era, never a claim the bug is gone: the class stays live (~8%/run under the committed instrument) and the section-7 reopener conditions + the reproduction instrument stand untouched as the disposition's evidence"
  - "17-08: the TCG x slirp avenue is EXHAUSTED, not absence-proof — N=10 serialized green runs (all driver-green, exactly one SeaBIOS pass per serial, no reset chain anywhere) recorded NO-FIRE-IN-SERIES with the weak-negative semantics stated in the record itself: P(0 fires in 10 | the ~8-10%/run KVM rate) is ~35-43% even if the class were present at full strength, and TCG timing likely dilutes the per-event race further"
  - "17-08: the five record surfaces move as ONE decision — section 9's Terminus token gates every closure edit, so WINDOWS #66, the 08-ledger CR-07 row, FLUSHOUT's cells/verdict/lineage, the VERIFICATION overrides block, and the REQUIREMENTS checkboxes cannot diverge; exactly one WINDOWS row moved (50/4 -> 49/5) and FLUSHOUT moved exactly 40 -> 41 table lines"

patterns-established:
  - "The maintainer-terminus record shape: gate-checkable literal lines (Terminus/Decided-by/Decided-at/FLUSH-04) in the root-cause record's own section 9, hash-baselined against the pre-terminus record so the terminus cannot mangle its own evidence base"
  - "The invited-overrides contract: a verifier report's frontmatter accepts ONLY the overrides block from the executor (accepted_by/accepted_at verbatim from the human decision, never template placeholders), with the whole rest of the file asserted hash-intact"

requirements-completed: [FLUSH-01, FLUSH-04]

coverage:
  - id: D1
    description: "The TCG x slirp last-avenue probe: N=10 serialized runs of the committed instrument on the pinned uninstrumented kernel, all green, recorded as 17-CR07-ROOTCAUSE.md section 8 (exactly one outcome token NO-FIRE-IN-SERIES, RACE-STRESS/DIAGNOSIS venue label, stated N rationale, retained evidence)"
    requirement: FLUSH-01
    verification:
      - kind: other
        ref: "plan Task 1 verify blocks 1-5 all green at execution (PROBE-RUNS / SECTION8-RECORDED / SERIES-EVIDENCE-RETAINED / RUN-CAPTURES-RETAINED / R9-UNTOUCHED); evidence under .planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-*"
        status: pass
    human_judgment: false
  - id: D2
    description: "The maintainer terminus decision itself: ratify-disposition + FLUSH-04 yes, accepted by Neil Skinner at 2026-11-13T14:42:08Z, recorded verbatim as 17-CR07-ROOTCAUSE.md section 9 with the acceptance metadata as literal gate-checkable lines"
    requirement: FLUSH-01
    verification:
      - kind: manual_procedural
        ref: "TERMINUS-RECORDED-ratify-disposition gate (section 9 token + Decided-by/Decided-at/FLUSH-04 lines); decision provenance: the orchestrator dispatch's checkpoint_resolution carrying the maintainer's answer"
        status: pass
    human_judgment: true
    rationale: "The waiver of a live ~8%/run crash class is a human acceptance, not an automated pass — no automated gate can decide a blocking-human checkpoint. The ratified decision (Neil Skinner, 2026-11-13T14:42:08Z) IS the verification; it is recorded verbatim with who/when so the acceptance is auditable"
  - id: D3
    description: "The ratified closure executed across all five record surfaces + the WR-03 re-cite: WINDOWS #66 waived (3-part D-15 on the reason line + resolved_at; counts exactly 49/5), 08-ledger CR-07 deferred (+ratification cite, stale parenthetical rewritten), FLUSHOUT ratified cells/verdict + exactly one lineage row (exactly 41 table lines, zero surviving pre-ratify literals), VERIFICATION frontmatter overrides with the rest of the file hash-intact, REQUIREMENTS FLUSH-01/FLUSH-04 checked + Complete"
    requirement: FLUSH-04
    verification:
      - kind: other
        ref: "plan Task 3 gates 1-11, all green: TERMINUS-RECORDED, RC-EVIDENCE-BASE-INTACT, W66-DISPOSITIONED, CR07-DEFERRED, WR03-RECITED, FLUSHOUT-RATIFIED (N=41), OVERRIDE-RECORDED, VERIF-INTACT, REQ-MIRRORED, D12-GUARD-HELD (open=49 waived=5), RECORDS-ONLY (r9 clean at 8f3ff11 == origin/main)"
        status: pass
    human_judgment: false

duration: 48 min wall across two executor sessions (Task 1's series + verify 14:17-14:35Z under the prior executor; the continuation's terminus + closure 14:46-15:05Z)
completed: 2026-11-13
status: complete
---

# Phase 17 Plan 08: The CR-07/#66 Terminus Summary

**TCG x slirp N=10 avenue exhausted (NO-FIRE-IN-SERIES) + maintainer-ratified #66/CR-07 disposition — 3-part D-15 rationale, Neil Skinner 2026-11-13T14:42:08Z, FLUSH-04 yes — closing FLUSH-01/FLUSH-04 records-only, r9 untouched at 8f3ff11 == origin/main**

## Performance

- **Duration:** 48 min wall across two executor sessions (Task 1 series 2026-11-13T14:17-14:35Z under the prior executor — 10 serialized runs + the tracer verify re-run; continuation 14:46-15:05Z — checkpoint resolution recorded, Task 3 executed, all gates)
- **Started:** 2026-11-13T14:17:32Z (the series begin; Task 1's build/sanity preceded it)
- **Completed:** 2026-11-13T15:05:00Z
- **Tasks:** 3/3 (Task 1 tracer + Task 2 blocking-human checkpoint answered + Task 3 records pass)
- **Files modified:** 18 production files (10 evidence serials + probe log + baseline sha + 5 records + the record itself) + this SUMMARY

## Accomplishments

- **The last named host-side capture avenue has a recorded outcome:** N=10 serialized TCG x slirp runs of the committed boot-r9-stress-x64 instrument (the wrapper's `-d int,guest_errors -D` capture NOT blind under TCG — ~3.6k full-CPU-state event records per green run), every run green (driver ALL STAGES PASS, `netstress: STRESS-DONE`, exactly one SeaBIOS pass per serial, no reset chain anywhere) — section 8 carries `Outcome: NO-FIRE-IN-SERIES` with the weak-negative semantics in the record itself: avenue exhaustion, never absence (P(0 fires in 10 | the ~8-10%/run KVM rate) ≈ 35-43% even if present; TCG timing dilutes further). Reopener condition (a) of section 7 is NOT met by this venue at this N.
- **The maintainer ratified the explicit disposition:** ratify-disposition + FLUSH-04 yes, Decided-by Neil Skinner, Decided-at 2026-11-13T14:42:08Z — recorded verbatim as section 9 (gate-checkable literal lines), the evidence base hash-baselined before the append (RC-EVIDENCE-BASE-INTACT green: sections 1-8 byte-identical through the terminus).
- **All five record surfaces carry one consistent ratified terminus:** WINDOWS #66 open -> waived (the 3-part D-15 rationale on the reason line — WHERE the 8-way-transfer exception-delivery window / WHY the venue-blocked cure: the KVM capture ceiling completed by section 8 + the refuted strongest-candidate fix / Reopener a context-reaching venue or non-healing instrumentation — plus resolved_at stamped; open/waived counts exactly 50/4 -> 49/5, only #66 flips); the 08-ledger CR-07 row skipped -> the exact token `deferred` with the ratification cite and the open-row-carrier parenthetical rewritten to the waived state; FLUSHOUT's 08-CR-07 and #66 action cells `dispositioned (deferred, maintainer-ratified 17-08)` with the verdict rewritten (FLUSH-01 complete via the disposition arm; FLUSH-04 per the literal yes) and exactly one records-only 17-08 lineage row (40 -> 41 table lines, all three pre-ratify literals gone); the VERIFICATION frontmatter overrides block with accepted_by/accepted_at verbatim and every byte outside it hash-intact (sha e0747290... asserted); REQUIREMENTS FLUSH-01 [x] + Complete and FLUSH-04 [x] + Complete.
- **The verifier's secondary finding closed:** the 08 ledger's WR-03 Source cell re-cites 17-02's discharge (both tiered leak-gate tiers green twice consecutively — the r9-side q35churn profile + the composed /bin/cat churn leg) instead of the stale not-run warning prose; the re-cite ran on every terminus branch (WR03-RECITED is unconditional).
- **Records-only invariant held:** r9 clean at 8f3ff11 == origin/main through every step, Harvey's flake.lock untouched, no new gate/app/profile/driver — RECORDS-ONLY green.

## Task Commits

Each task was committed atomically:

1. **Task 1: The last-avenue probe — the TCG x slirp diagnosis series (tracer)** - `e20ecccea` (docs; prior executor) — section 8 + the probe log + 10 serial captures
2. **Task 2: The maintainer terminus decision (checkpoint:decision, blocking-human)** - answered out-of-band by the maintainer; no commit (the decision's recording is Task 3's section 9)
3. **Task 3: Execute the ratified terminus — the records pass** - `0eed1d432` (docs: section 9 terminus + WR-03 re-cite + the pre-terminus baseline sha) and `af7a00610` (docs: the five-surface closure edits, including committing the previously-untracked 17-VERIFICATION.md)

**Plan metadata:** follows (docs: SUMMARY + STATE/ROADMAP)

## Files Created/Modified

- `.planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md` - section 8 (the TCG x slirp avenue: protocol, per-run outcomes, the outcome token, the weak-negative semantics, evidence paths) + section 9 (the terminus: the verbatim decision lines + the five-surface map + the not-a-cure statement)
- `.planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-probe.log` - the series loop log (one [run N] line per run with venue/accel/shape/result + scratch-dir paths, the cascade-disposition lines, the tracer verify re-run)
- `.planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run01..10.serial.log` - ten per-run serial captures (all green)
- `.planning/phases/17-r9-kernel-flush-out/evidence/17-08-rc-preterminus.sha` - the sha256 of the record through section 8, captured immediately before the section-9 append
- `.planning/WINDOWS.md` - row #66: status waived, the one-line 3-part D-15 reason with the ratification cite, resolved_at 2026-11-13T14:42:08Z (description untouched — the historical record)
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW-DISPOSITION.md` - CR-07 row: Disposition `deferred` + the ratification cite + the waived-state parenthetical; WR-03 row: the 17-02 both-tiers-green-twice discharge replaces the stale warning prose
- `.planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md` - the 08-CR-07 and #66 action cells, the ratified completion verdict, the 17-08 lineage row
- `.planning/phases/17-r9-kernel-flush-out/17-VERIFICATION.md` - the frontmatter overrides block (accepted_by "Neil Skinner", accepted_at "2026-11-13T14:42:08Z"); committed here (the verifier's report was previously untracked)
- `.planning/REQUIREMENTS.md` - FLUSH-01 and FLUSH-04 checked, both traceability rows Complete

## Decisions Made

- **The ratified terminus (the plan's central output, decided by the maintainer — not the executor):** `Terminus: ratify-disposition`, `FLUSH-04: yes`, **Decided-by: Neil Skinner**, **Decided-at: 2026-11-13T14:42:08Z** — the CR-07 / WINDOWS #66 class is explicitly dispositioned (waived-of-cure / deferred) with the 3-part D-15 rationale, on the evidence of the root-cause record + the completed avenue + the committed reproduction instrument. The class is NOT claimed gone; the reopener conditions (section 7) and the instrument survive intact, and any `fixed` vocabulary requires a future cure pass under D-03's evidence bar.
- **FLUSH-04 checked per the literal yes** (the verifier-confirmed letter: #44/#45 closed, every landed fix pinned) — REQUIREMENTS.md and its traceability row now read Complete.
- **The WR-03 re-cite lands on every branch** (it predates and survives the terminus): the discharge the stale prose said was missing is 17-02's, cited in the cell.
- **Scope conventions followed by the letter:** the 08 ledger's generated frontmatter counts (`skipped: 1`) were left untouched — the plan's D-12 confinement is row-cells-only, the same convention 17-07 followed editing this row; the dated 17-01 preamble note likewise stays as historical narrative (the row carries the current truth).

## Deviations from Plan

None - plan executed exactly as written. (Commit-shape note: Task 3 landed as two docs commits — `0eed1d432` for the every-branch (A)-(C) sequence exactly as the plan orders it, `af7a00610` for the ratify-only closure edits (1)-(5), which the plan sequences after (C) but assigns no commit of their own; the atomic per-task commit protocol supplies it.)

## Issues Encountered

- The prior executor's probe log self-documents a scratch-dir extraction defect in the cascade-cleanup step (the path sed captured a trailing parenthetical); it was corrected in-log with `[post-corrected]` lines and all ten non-fired ~143 MB cascades were deleted before this continuation began — no action needed here, recorded for the audit trail.
- WINDOWS.md is markdown-with-frontmatter wrapping a JSON-array body: a mid-execution JSON.parse sanity probe failed on the frontmatter — not a defect (the ledger's line-scoped gates are the arbiter and all passed; the edit preserved the one-field-per-line shape of the #50 precedent).
- The 08 ledger's frontmatter aggregate counts now lag the CR-07 row (see Decisions Made) — per-row cells remain the ledger's truth per the D-12 scope; a future ledger regeneration will recompute them.

## Next Phase Readiness

- **Phase 17 is complete:** all four FLUSH requirements checked in REQUIREMENTS.md, FLUSHOUT walkable at exactly 41 table lines, both review ledgers consistent with WINDOWS.md, and the verifier's single blocking gap closed by the named acceptance it asked for (escalation items 1 and 2 both resolved by the same decision).
- Phase 18 (auth) starts from a fully-closed flush-out ledger at pin 8f3ff11. The waived #66 class remains live (~8%/run under the full 8-way concurrent bulk transfer — a load shape the auth loopback p9sk1 legs do not create); the venue-conditional reopener (a context-reaching capture venue or non-healing instrumentation) re-opens the hunt from 17-CR07-ROOTCAUSE's shoulders the moment such a venue exists, with the committed instrument firing the class at will in ~25s.

---
*Phase: 17-r9-kernel-flush-out*
*Completed: 2026-11-13*

## Self-Check: PASSED

All claimed files exist on disk and all four plan commits (e20ecccea, 0eed1d432, af7a00610, dbd357e3e) are ancestors of HEAD; all eleven Task 3 gates re-verified green at close.
