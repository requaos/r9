---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 20
subsystem: xtask
tags: [xtask, cargo, test-9p, wire-13, wr-08, verification, scalar-backend]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: plan 04-12's complete `test-9p` step (six legs over three command bases, the zero-test guard)
provides:
  - "`cargo xtask test-9p` runs one leg per listed `port/tests` file, so a file that lost every test fails by name"
  - "`check_leg_output` requires every `test result: ok.` line of a leg to report passed tests"
affects: [closing-suite evidence, plan 04-23 review bundle]

actuals:
  tokens: 3438
  tasks: 1
  commits: 2
  plan_head_before: 3041ded324249882e0dfd4af33da718711bd6eb1
  plan_head_after: 18ff8e5a16c78781c5f1202691145bf5293c9e9f

tech-stack:
  added: []
  patterns:
    - "One leg per listed test file, default and scalar-backend twin, so a zero count is tied to the file it came from"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-20.md
  modified:
    - xtask/src/test9p.rs

key-decisions:
  - "The listed file is the unit a leg checks: the combined five-file `port` leg is replaced, not kept alongside (assumption delta: promote)"
  - "A leg is green only when it has at least one `test result: ok.` line and every such line reports a passed count above zero"
  - "The step is still not part of `cargo xtask test`; `xtask/src/main.rs` and `.cargo/config.toml` are unchanged"

patterns-established:
  - "Per-file test legs: a result line carries no file name, so the leg is the only place a zero count can be attributed"

requirements-completed: [WIRE-13]

coverage:
  - id: D1
    description: "One leg per listed `port/tests` file: fourteen specs (seven legs, each with its scalar-backend twin), in `PORT_TESTS` order"
    requirement: WIRE-13
    verification:
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_runs_one_leg_per_listed_port_file"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_command_specs_name_every_listed_file"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test-9p (exit 0, 14 legs, 0 `test-9p:` lines)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A leg fails when any `test result: ok.` line reports zero passed tests, and when it has no result line"
    requirement: WIRE-13
    verification:
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_rejects_a_leg_with_one_zero_test_result_line"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_treats_zero_tests_as_an_error"
        status: pass
    human_judgment: false

duration: 3min
completed: 2026-11-25
status: complete
---

# Phase 04 Plan 20: `cargo xtask test-9p` — one leg per listed file Summary

**`cargo xtask test-9p` now runs one leg per listed `port/tests` file (fourteen legs with the scalar twins) and fails by name when any leg's result lines report zero passed tests**

## Performance

- **Duration:** 3 min
- **Started:** 2026-11-25T04:41:01Z
- **Completed:** 2026-11-25T04:44:22Z
- **Tasks:** 1
- **Files modified:** 2 (1 source file, 1 evidence ledger)

## Accomplishments

- `command_specs` returns fourteen specs: the two `crypto` `session` legs and one leg per `PORT_TESTS` entry (`aead_wire`, `aead_tlv`, `aead_transport`, `aead_fid`, `aead_negotiate`), each immediately followed by its scalar-backend twin carrying `--config` and `SOFT_BACKEND_CONFIG`.
- `check_leg_output` scans every `test result: ok.` line of a leg; the leg is `Ok` only when at least one such line exists and every one reports a passed count above zero. A file emptied of tests can no longer hide behind a sibling's positive line (WR-08, T-04-39).
- The two new unit tests were RED against the old step and are GREEN now: `test9p_runs_one_leg_per_listed_port_file` and `test9p_rejects_a_leg_with_one_zero_test_result_line`.
- `evidence/PINS-04-20.md` records `PIN gap-wr08-per-file-legs` RED, GREEN-1, GREEN-2 and ADVANCED, and the two `TEST9P run-N legs=14 EXIT=0` lines.

**Later evidence:** the step now has **fourteen legs**; a later count of `test-9p` legs expects 14, not the 6 plan 04-12 recorded. The total `test result: ok` line count is unchanged at 14 (seven test binaries per backend both before and after), so counting result lines alone no longer distinguishes the two shapes — count legs, or read the `TEST9P run-N legs=14` ledger lines.

## Task Commits

Each task was committed atomically:

1. **Task 1: WR-08 — one leg per listed `port` file** — RED `cf55de0` (test), GREEN `18ff8e5` (fix)

**Plan metadata:** `docs(04-20)` commit (this SUMMARY and the ledger's ADVANCED line)

_Note: TDD task — RED then GREEN, per D-03._

## Files Created/Modified

- `xtask/src/test9p.rs` — `command_specs` one leg per `PORT_TESTS` entry with its scalar twin; `check_leg_output` every-result-line rule; module doc and `CommandSpec::args` doc; two new tests and the eight earlier tests revised to the fourteen-spec shape
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-20.md` — the RED/GREEN ledger for `gap-wr08-per-file-legs`

## Decisions Made

- **Promote the listed file to the leg** (the plan's `<assumption_delta_decision>`): the combined five-file `port` leg is replaced, not kept alongside. A `test result:` line carries no file name, so only a leg per file can attribute a zero count to the file that lost its tests.
- **Every result line, not the first** (WR-08's fix option two applied inside option one): the leg-level rule makes a multi-line capture safe as well, so a future multi-binary leg cannot regress the guarantee.
- The step stays out of `cargo xtask test`, `xtask/src/main.rs` is untouched, and the soft-backend flags stay in the leg argument vectors (lint rule R5).

## Deviations from Plan

None - plan executed exactly as written.

The plan's `<action>` named the two commits; the ledger's `ADVANCED 18ff8e5` line could only be written once the GREEN commit hash existed, so it rides in the closing metadata commit (the form plan 04-12 used, where `ADVANCED` landed in the commit after GREEN). This is bookkeeping, not a behavioural deviation.

## Issues Encountered

None.

## Verification

- `nix develop -c cargo test -p xtask test9p` → `test result: ok. 10 passed; 0 failed` (all ten `test9p_` tests, including the two new ones).
- `nix develop -c cargo xtask test-9p` → exit 0 on two consecutive runs; 14 `test result: ok` lines and 0 `test-9p:` lines each; `test aead_negotiate_end_to_end_session ... ok` twice.
- `nix develop -c cargo xtask test` → exit 0.
- `nix develop -c cargo xtask check` → exit 0.
- `nix develop -c cargo xtask clippy` → exit 0 (pre-existing warnings only; none in `test9p`).
- `git diff --name-only <base>..HEAD` lists only `xtask/src/test9p.rs` and `evidence/PINS-04-20.md`, so `xtask/src/main.rs` and `.cargo/config.toml` are unchanged.

## Threat Flags

None — no new network endpoint, auth path, file-access pattern or schema change. The threat register's T-04-61 mitigation is the two new unit tests plus the per-leg rule.

## Known Stubs

None.

## Next Phase Readiness

- WR-08 is closed: `test-9p` can no longer report green when one listed file ran zero tests, when a leg printed no result line, or when only the default backend ran.
- Review findings outside the gaps (IN-05, the threat-model guard in `xtask/tests/threat_model.rs`) are untouched and stay open for the owner, as the plan requires.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- `04-20-SUMMARY.md`, `evidence/PINS-04-20.md`, `xtask/src/test9p.rs` present
- Commits `cf55de0` (RED) and `18ff8e5` (GREEN) are ancestors of HEAD
- Ledger carries `PIN gap-wr08-per-file-legs ADVANCED`
- Source defines ten `test9p_` tests; the suite reports ten passed
