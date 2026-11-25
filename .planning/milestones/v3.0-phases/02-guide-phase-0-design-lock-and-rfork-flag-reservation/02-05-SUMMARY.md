---
phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
plan: 05
subsystem: evidence
tags: [kernel-bytes, aarch64, quick-pass, owner-stop, outputhash, evidence]

# Dependency graph
requires:
  - phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
    plan: 04
    provides: "Both arch crates importing the rfork flags from `port::rfork`, the hoisted `RFORK_SUPPORTED` masks pinned by host tests, and the commit the after-builds measure"
provides:
  - "Before/after sha256 of four kernel configs on one venue; `.text` identical on both x86_64 configs and `.text` plus `.text.boot` identical on both aarch64 configs (D-23 stop condition met)"
  - "The stripped-ELF differential (118 x86_64 bytes, 44 aarch64 bytes), matching the research's panic line-number-only change"
  - "The aarch64 quick pass green on the Mac (tier-1 HVF and TCG, smoke, net-smoke, cpu) and the four-command Mac suite green at the last code commit"
  - "`02-OWNER-STOP.md`: the ordered x86_64 Linux checklist with the one-commit rule, the exact `flake.nix:983` line and hash commit message, and the failure branches"
affects: [phase-02-plan-06]

# Actuals (#2632) — measured on the same chars/4 scale as the plan's estimate.
actuals:
  tokens: 4910
  tasks: 3
  commits: 3
  plan_head_before: 09e6901f24e5f1479f8657fc9b5809632d874a16
  plan_head_after: 43958eb450c21bd4191f8b2d9ba3dd628f3fc99b

tech-stack:
  added: []
  patterns:
    - "Before and after kernel hashes are built on one machine and the venue line is compared before any verdict is read"
    - "A whole-ELF difference is expected evidence, not a failure; only `.text` and `.text.boot` equality is the stop condition"
    - "Every quick-pass verdict is decided from its own leg's exit code and `ALL STAGES PASS` line, with the run log kept per leg"
    - "The owner checklist wraps every verdict-producing command in one `run_logged` function so each log starts with `HEAD=` and ends with `EXIT=`"

key-files:
  created:
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/kernel-bytes/after.sha256
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/kernel-bytes/KERNEL-BYTES.md
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/aarch64-quick-pass.md
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-OWNER-STOP.md
  modified: []

key-decisions:
  - "The after-builds measure HEAD 09e6901 (the wave-2 tracking commit), whose code under port, x86_64, aarch64, xtask and sys equals the last code commit 53ceb12; `git diff --quiet 09e6901 785ee19 -- port x86_64 aarch64 xtask sys` exits 0"
  - "No text/textboot verdict was DIFFERENT, so the D-23 stop did not fire and Tasks 2 and 3 ran"
  - "The whole ELFs differ (expected): panic line numbers moved in x86_64 .rodata and aarch64 .data; because the x86_64 ELF differs, the boot-iso outputHash still has to be re-derived at the owner stop"
  - "The quick-pass evidence names LEGS_SHA 785ee19, not the earlier AFTER_SHA; the code under the five paths is identical between them"
  - "The owner checklist uses a single `run_logged` wrapper rather than repeating a redirection per command, which is how it satisfies the plan's requirement that every verdict log begins with `HEAD=` and ends with `EXIT=`"

requirements-completed: [DSGN-04]

coverage:
  - id: D1
    description: "Before and after sha256 of the whole ELF, `.text` and (aarch64) `.text.boot` are recorded for q35churnshell, q35cpu, virtshell and virtcpu, all built on one venue"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "`grep -cE '^(q35churnshell|q35cpu|virtshell|virtcpu) (elf|text|textboot) [0-9a-f]{64}$' after.sha256` -> 10; the `venue` lines of both files are `Darwin arm64`"
        status: pass
    human_judgment: false
  - id: D2
    description: "`.text` and `.text.boot` are byte-identical before and after in all four configs"
    requirement: "DSGN-04"
    verification:
      - kind: other
        ref: "`diff` of the six `text`/`textboot` lines between before.sha256 and after.sha256 is empty; KERNEL-BYTES.md has six IDENTICAL text verdicts"
        status: pass
    human_judgment: false
  - id: D3
    description: "The aarch64 quick pass is green on the Mac, and the Mac suite (test, cargo test -p xtask, check, clippy) is green at the last code commit"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "five `VERDICT aarch64-...: PASS` lines each with exit code 0 and its `ALL STAGES PASS` line; `VERDICT mac-full-suite: PASS` with four exit codes 0; the verify re-runs of `cargo xtask test` and `cargo xtask clippy` also exited 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "An ordered owner checklist for the single end-of-phase stop is committed, with host tests before the hash step, the exact flake.nix line, the hash commit message and the one-commit rule"
    requirement: "DSGN-04"
    verification:
      - kind: integration
        ref: "the plan's six automated checks: KVM quick command appears once; all five step commands present; host-test command precedes `nix build .#boot-iso`; four `HEAD=` occurrences; flake.nix:983 still holds the b6OcAb hash; no attribution line"
        status: pass
    human_judgment: false

duration: 10min
completed: 2026-11-24
status: complete
---

# Phase 2 Plan 05: Mac-side closing evidence — kernel bytes, aarch64 quick pass, and the owner's checklist Summary

**All four kernel configs keep byte-identical executable sections after the rfork flag move (six of six `.text`/`.text.boot` verdicts IDENTICAL, whole ELFs differing only as expected), the five-leg aarch64 quick pass and the four-command Mac suite are green at the last code commit, and the ordered x86_64 Linux checklist for the single end-of-phase stop is committed with the one-commit rule and the exact `flake.nix:983` line.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-11-24T02:31Z (approximate)
- **Completed:** 2026-11-24T02:40Z (last task commit)
- **Tasks:** 3
- **Files created:** 4 (all under the Phase 2 directory)

## Accomplishments

- Built `q35churnshell`, `q35cpu`, `virtshell` and `virtcpu` at HEAD `09e6901` on the Mac (`Darwin arm64`, the same venue as the before file) and recorded the whole-ELF, `.text` and `.text.boot` sha256 in `evidence/kernel-bytes/after.sha256` (ten hash lines).
- Proved the D-23 stop condition met: every `text`/`textboot` line equals the before value. The six verdicts in `KERNEL-BYTES.md` are IDENTICAL; the four whole-ELF verdicts are DIFFERENT, as the research measured (panic `Location` line fields moved, plus non-loaded debug/symbol sections).
- Recorded the stripped-ELF differential as information: 118 differing bytes for each x86_64 config and 44 for each aarch64 config — exactly the research's measurement.
- Ran the aarch64 quick pass from the repository root, each leg to its own log under `target/phase02-quick-pass/`: tier-1 HVF and TCG, smoke, net-smoke and cpu all exited 0 with their `ALL STAGES PASS` line. No leg needed a rerun.
- Ran the full Mac suite at the same commit: `cargo xtask test`, `cargo test -p xtask`, `cargo xtask check` and `cargo xtask clippy` all exited 0; the rfork mask test line appears twice (aarch64 default and `virtshell` cfg legs).
- Wrote and committed `02-OWNER-STOP.md`: the branch push approval, the pull and tip comparison, the x86_64 host tests before the hash step (D-29), the `outputHash` build/comment/commit/`--rebuild` with the exact line 983 text and commit message (D-28), the single direct KVM quick gate (D-25, D-27), the TCG gate, the full gate set, and the report-back route. Every verdict-producing command runs through `run_logged`, so each log starts with `HEAD=` and ends with `EXIT=`; the header carries the one-commit rule and the section 8 return-commit exception.

## Task Commits

Each task was committed atomically:

1. **Task 1: After-hashes for four kernel configs; `.text` identical** — `785ee19` (evidence)
2. **Task 2: aarch64 quick pass and the full Mac suite** — `92adf8b` (evidence)
3. **Task 3: ordered owner checklist for the end-of-phase stop** — `43958eb` (docs)

The plan metadata commit follows this summary.

## Files Created/Modified

- `evidence/kernel-bytes/after.sha256` — `after <HEAD>`, `venue Darwin arm64`, ten hash lines
- `evidence/kernel-bytes/KERNEL-BYTES.md` — before/after table, ten verdict lines, stripped-ELF differential, the venue-not-comparable statement
- `evidence/aarch64-quick-pass.md` — LEGS_SHA, five leg verdicts with exit codes and decisive lines, the four-command suite verdict
- `02-OWNER-STOP.md` — the ordered x86_64 Linux checklist

## Decisions Made

- The after-builds measure HEAD `09e6901` rather than the last code commit `53ceb12`; the plan asks for HEAD, and the code under the five kernel paths is identical between them (`git diff --quiet` exits 0).
- The whole-ELF differences are recorded, not treated as a failure: D-23's stop condition is `.text`/`.text.boot` only. Because the x86_64 ELF differs, the checklist keeps the `outputHash` re-derivation.
- The quick-pass evidence names LEGS_SHA `785ee19` and records why it differs from AFTER_SHA.
- The checklist's `run_logged` wrapper is the mechanism for the plan's per-log `HEAD=`/`EXIT=` requirement, applied to every verdict-producing command including the witness driver invocations.

## Deviations from Plan

None - plan executed exactly as written. No source file was changed; if a check had failed, the plan required returning a new task rather than editing here.

## Recorded, not auto-fixed

- The plan's verify block re-ran `nix develop -c cargo xtask test` and `nix develop -c cargo xtask clippy` after the evidence file was written; both exited 0. The verdict in the evidence file is from the single suite run whose logs are kept under `target/phase02-quick-pass/mac-suite/`.
- The aarch64 quick-pass logs, the copied ELFs and the stripped copies live under `target/` and are not committed.

## Issues Encountered

- None. Every build, leg and suite command returned its expected result on the first attempt; the D-23 stop never fired.

## Known Stubs

None.

## Threat Flags

None beyond the planned surface. T-02-16 (executable code changed by the refactor) is mitigated by the six IDENTICAL text verdicts and the D-23 stop rule; T-02-17 (evidence compared across venues) by the matching `venue` lines and the explicit not-comparable statement; T-02-18 (a verdict without a run) by deciding each verdict from its own exit code and log; T-02-26 (a stale hash reaching `main`) by the checklist's one-commit rule, `HEAD=`-stamped logs and host-tests-before-hash order.

## Next Phase Readiness

- Plan 02-06 starts from the committed `02-OWNER-STOP.md` and performs the single owner stop; it must not run the boot-to-rio gate earlier (D-25).
- Nothing was pushed; `main` and the remote are unchanged. Harvey was read only.
- The working tree holds no change from this plan; the remaining untracked entries (`.claude/`, `.planning/config.json`, `.planning/graphs/`, `.planning/state.json`, `.planning/ui-reviews/`) pre-date this plan.

---

*Phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation*
*Completed: 2026-11-24*

## Self-Check: PASSED

All four listed evidence/checklist files exist; all three plan commits (`785ee19`, `92adf8b`, `43958eb`) are ancestors of HEAD.
