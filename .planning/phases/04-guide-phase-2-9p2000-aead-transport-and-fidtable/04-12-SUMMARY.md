---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 12
subsystem: testing
tags: [xtask, test-9p, wire-pins, scalar-backend, unsafe-audit, WIRE-13]

requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: "the five port wire-pin files, the crypto session pins, and the test9p skeleton from plan 04-17"
provides:
  - "`cargo xtask test-9p` runs every wire pin on the default and scalar backends, refuses a missing listed file, and rejects a zero-test leg"
  - "UNSAFE-AUDIT.md Phase 4 delta with the measured per-crate unsafe counts and the reversed kernel-link statement"
affects: [phase 4 code review, phase 5]

actuals:
  tokens: 6483
  tasks: 2
  commits: 3
  plan_head_before: 5e1cf705e265b9c05798bd8ce6c9509b5fa062a7
  plan_head_after: 127df30d8a4a2a9162b9a91232111b771a8c064a

tech-stack:
  added: []
  patterns:
    - "a test step split into a pure `command_specs`, an injectable `run_specs` runner and a `check_leg_output` zero-test guard, so early-stop and empty-run behaviour are unit-tested without spawning"
    - "default/scalar-backend leg pairing: each default leg is followed by a twin that appends one `--config build.rustflags=[...]` argument in its own argv"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-12.md
  modified:
    - xtask/src/test9p.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "Re-measured `port`'s lint-visible unsafe sites as 22 (14 blocks + 8 impls), not the plan's expected 23; the reconciliation table's own module breakdown (globalalloc 8 + mcslock 12 + devip_waker 2) also sums to 22, so the measured value 22 is written and the discrepancy is stated in the delta"
  - "`preflight` checks seven files (five `port/tests` files, `crypto/src/session.rs`, `crypto/tests/session.rs`); the plan prose said 'six', which conflicts with its own action listing both crypto paths"
  - "The scalar-backend twin appends exactly one `--config` argument; the value is copied verbatim from the Phase 3 crypto leg in `xtask/src/main.rs`, never written to `.cargo/config.toml` (R5)"

requirements-completed: [WIRE-13]

coverage:
  - id: D1
    description: "`cargo xtask test-9p` runs the crypto `session` pins and the five `port` wire-pin files on the default and scalar backends, refuses any missing listed file before running, and errors when a leg reports no passed tests"
    requirement: WIRE-13
    verification:
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_command_specs_name_every_listed_file"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_lists_the_five_port_files_and_the_crypto_session_pins"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_has_a_scalar_backend_twin_for_every_leg"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_refuses_when_exactly_one_listed_file_is_missing"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_treats_zero_tests_as_an_error"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_reports_the_first_failing_leg"
        status: pass
      - kind: integration
        ref: "`nix develop -c cargo xtask test-9p` (exit 0, both runs; `aead_fid_sweep_matches_set_model_on_random_inputs` and `aead_negotiate_end_to_end_session` pass on both backends)"
        status: pass
    human_judgment: false
  - id: D2
    description: "`UNSAFE-AUDIT.md` carries the Phase 4 delta: `port` links `crypto/` and `ds/`, `port::chan`/`port::fid` are forbid modules, the measured per-crate unsafe counts, and the re-measured `ds` unsafe line"
    requirement: WIRE-13
    verification:
      - kind: other
        ref: "`grep -c 'Phase 4 delta' UNSAFE-AUDIT.md` = 1; ledger carries three `AUDIT-COUNT` lines matching the paragraph"
        status: pass
      - kind: integration
        ref: "`nix develop -c cargo xtask clippy` exit 0, no `unsafe block missing a safety comment`"
        status: pass
    human_judgment: true
    rationale: "Automated checks prove the paragraph, the ledger counts and a clean clippy; whether the counts and the reversed-link narrative are accurate and complete against the tree is the owner's judgment at the phase-end audit review"

metrics:
  duration: "3 min"
  started: "2026-11-25T01:18:00Z"
  completed: "2026-11-24"
  tasks: 2
  files_modified: 3
status: complete
---

# Phase 4 Plan 12: `cargo xtask test-9p` complete, and the unsafe audit delta Summary

**The named wire-pin step now runs every 9P2000.aead pin on both the host and scalar backends, refuses to pass empty, and the unsafe audit records the Phase 4 kernel-link delta with measured counts.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-11-25T01:18:00Z
- **Completed:** 2026-11-24
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `xtask/src/test9p.rs` lists all five `port` wire-pin files and both `crypto` `session` targets, runs each default leg plus a scalar-backend twin (six legs), and names every file in `preflight` so a rename or delete fails before any leg runs.
- `run_specs` stops at the first failing leg (no status is dropped or masked) and `check_leg_output` rejects a leg whose result lines report no passed tests, so a filter that matches nothing cannot pass.
- Eight `test9p_` unit tests pin the six-leg shape, the twins, per-file refusal, the zero-test guard and the early stop; `cargo xtask test-9p` passed twice with `aead_fid_sweep_matches_set_model_on_random_inputs` and `aead_negotiate_end_to_end_session` green on both backends.
- `UNSAFE-AUDIT.md` gains the dated Phase 4 delta: `port` links `crypto/` and `ds/` into the kernel images, `port::chan` (and submodules) and `port::fid` are `#[forbid(unsafe_code)]`, no new unsafe-holding module exists (no IG-D07 approval), and the re-measured counts are `port` 22, `crypto` 0, `ds` 1 at `flattened.rs:473`.

## Task Commits

Each task was committed atomically (Task 1 is TDD, so RED then GREEN):

1. **Task 1 RED: the complete test-9p step** — `499a24a` (test)
2. **Task 1 GREEN: test-9p on default and scalar backends** — `80afc9c` (feat)
3. **Task 2: UNSAFE-AUDIT Phase 4 delta** — `127df30` (docs)

**Plan metadata:** `docs(04-12): complete test-9p and audit plan` (this commit)

## Files Created/Modified

- `xtask/src/test9p.rs` — the complete `test-9p` step: `PORT_TESTS` (five), `CRYPTO_TESTS`, `command_specs` (six legs with scalar twins), `preflight` (seven files), `run_specs`, `check_leg_output`, `LegOutcome`, `run`, and the eight unit tests.
- `UNSAFE-AUDIT.md` — the Phase 4 delta paragraph, the Phase 4 kernel-link update near line 28, the re-measured `ds` line beside the Phase 3 `flattened.rs:440`, and the `port` module-states row naming `chan`/`fid`.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-12.md` — the `test9p-complete` RED/GREEN/ADVANCED records, the two `TEST9P run-N legs=6 EXIT=0` lines, and the three `AUDIT-COUNT` lines.

## Decisions Made

- **Measured `port` count is 22, not 23.** The plan expected `port` unchanged at 23 and instructed that a differing measurement be written as measured. The current tree holds 14 unsafe blocks + 8 unsafe impls = 22 (globalalloc 8, mcslock 12, devip_waker 2); the reconciliation table's own module breakdown also sums to 22, so the Phase 3 figure of 23 was one high. The delta writes 22 and states the discrepancy; no Phase 4 file moved it, since `chan`/`fid` are forbid modules with zero unsafe.
- **`preflight` covers seven files.** The plan prose said "six listed files", but its action names `crypto/src/session.rs`, `crypto/tests/session.rs` and each `port/tests/NAME.rs` = seven. The behaviour test iterates all seven, one removal at a time.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected an unmeasured/incorrect `port` unsafe count**
- **Found during:** Task 2 (the Phase 4 audit delta)
- **Issue:** The plan expected `port` "unchanged at 23", but measuring the tree at this commit yields 22 (14 blocks + 8 impls). The reconciliation table's own module breakdown (8 + 12 + 2) also sums to 22, so 23 was already inconsistent.
- **Fix:** Wrote the measured value 22 into the Phase 4 delta with the breakdown and a statement that the Phase 3 table row is one high; `crypto` 0 and `ds` 1 were confirmed. The plan's prohibition ("MUST NOT state a count that was not measured at this plan's commit") drove writing the measured figure rather than copying the expected one.
- **Files modified:** `UNSAFE-AUDIT.md`, `evidence/PINS-04-12.md`
- **Verification:** `AUDIT-COUNT port 22` / `crypto 0` / `ds 1` ledger lines equal the paragraph; `cargo xtask clippy` exit 0 with no safety-comment failure.
- **Committed in:** `127df30`

**2. [Rule 1 - Bug] Resolved the plan's "six listed files" vs seven-path split**
- **Found during:** Task 1 (the RED/GREEN file list)
- **Issue:** The behaviour prose said "six listed files" while the action and `preflight` requirement name seven (`port/tests/{5}`, `crypto/src/session.rs`, `crypto/tests/session.rs`).
- **Fix:** Implemented `preflight` over all seven and made `test9p_refuses_when_exactly_one_listed_file_is_missing` iterate every one of the seven; no behaviour is skipped.
- **Files modified:** `xtask/src/test9p.rs`
- **Verification:** `nix develop -c cargo test -p xtask test9p` — 8 passed; the per-file removal test covers all seven.
- **Committed in:** `80afc9c`

---

**Total deviations:** 2 auto-fixed (2 bug/consistency corrections)
**Impact on plan:** Both are honesty/completeness fixes required by the plan's own prohibitions; no scope creep, and no production source outside the plan's files changed.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Next Phase Readiness

- WIRE-13 is met: the guide's verification-matrix command exists and passes every wire pin on the default and scalar backends, and cannot pass empty.
- The unsafe audit matches the tree for Phase 4; the owner reviews the whole `port/src` change at the phase-end code review (D-24).
- Plan 04-13 (planning documents) shares no file with this plan and is unaffected.

## Owner action before wave 10

Waves 10 and 11 (plans 04-14 and 04-15) run only on the main working tree. Before continuing `/gsd-execute-phase 4`, set `workflow.use_worktrees` to `false` in `.planning/config.json` (or through `/gsd-config`), then set it back to `true` after plan 04-15. The executor did not change the setting itself; plan 04-14 stops at once with `## CHECKPOINT REACHED` if it is dispatched in a worktree.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-24*

## Self-Check: PASSED

- All four expected files exist on disk (`xtask/src/test9p.rs`, `UNSAFE-AUDIT.md`, `evidence/PINS-04-12.md`, `04-12-SUMMARY.md`).
- All three task commits (`499a24a`, `80afc9c`, `127df30`) are ancestors of HEAD.
