---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "17"
subsystem: xtask-host-verification
tags: [test-9p, xtask, wire-pins, tracer, preflight]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-02)
    provides: "port/tests/aead_transport.rs (the sealed-packet tracer) and crypto/src/session.rs"
provides:
  - "cargo xtask test-9p — the named subset of wire pins that exists so far"
  - "xtask::test9p — PORT_TESTS, CRYPTO_LIB_FILTER, CommandSpec, command_specs, preflight, run"
affects: [04-12 test-9p completion, phase gate verification matrix]

actuals:
  tokens: 2900
  tasks: 2
  commits: 4
  plan_head_before: f2455ef104611209b24b3a1c29006e93229b58e1
  plan_head_after: 29511005b6389b0d0820fdeb47e08fb959a61023

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A step module modeled on miri.rs: a pure command builder unit tests inspect, a preflight refusal, and run as the only spawn site"
    - "A listed-but-absent test file is a named test-9p: error before any leg runs, so a rename cannot shrink the step into a silent pass"
    - "run returns at the first spawn error or non-zero status, so no leg's verdict is dropped or masked by a later one"

key-files:
  created:
    - xtask/src/test9p.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-17.md
  modified:
    - xtask/src/lib.rs
    - xtask/src/main.rs

key-decisions:
  - "CommandSpec carries exactly program and args: test-9p uses the ordinary cargo the bin hands it, so no environment lookup lives in the module"
  - "The port leg carries one --test NAME pair per PORT_TESTS entry rather than a glob, so the list is the contract preflight enforces"
  - "The crypto leg is `test --package crypto --lib session`; it currently matches zero tests because crypto/tests/session.rs is plan 04-03's and the zero-tests guard is plan 04-12's"

patterns-established:
  - "Step refusal: preflight names the first missing listed path with the step prefix before any spawn"
  - "Test-naming: test9p_<behaviour> unit tests, one per behaviour, using a scratch tree"

requirements-completed: [WIRE-13]

coverage:
  - id: D1
    description: "xtask::test9p lists the wire-pin files, builds the commands purely, and refuses when a listed file is missing"
    requirement: WIRE-13
    verification:
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_command_specs_name_every_listed_file"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_refuses_when_a_listed_file_is_missing"
        status: pass
      - kind: unit
        ref: "xtask/src/test9p.rs#test9p_uses_the_program_it_is_given"
        status: pass
    human_judgment: false
  - id: D2
    description: "cargo xtask test-9p exists and runs the tracer pin on the host, exiting non-zero when a leg fails"
    requirement: WIRE-13
    verification:
      - kind: e2e
        ref: "nix develop -c cargo xtask test-9p (twice, rc 0; output carries `test aead_transport_tracer_one_sealed_packet ... ok`, no `test-9p:` line)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo test -p xtask (rc 0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The test9p-step pin records RED, GREEN-1, GREEN-2 and ADVANCED"
    requirement: WIRE-13
    verification:
      - kind: other
        ref: "grep -cE '^PIN test9p-step (RED|GREEN-1|GREEN-2|ADVANCED)' evidence/PINS-04-17.md → 4"
        status: pass
    human_judgment: false

# Metrics
duration: 2min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 17: `cargo xtask test-9p` — the step, running the tracer pin Summary

**`cargo xtask test-9p` now runs the sealed-packet tracer on the host from a listed, preflighted file set, and fails loudly when a listed file is missing or a leg fails.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-25T00:24:35Z
- **Completed:** 2026-11-25T00:26:36Z
- **Tasks:** 2
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments
- `xtask::test9p` with `PORT_TESTS` (`aead_transport`), `CRYPTO_LIB_FILTER` (`session`), `CommandSpec`, a pure `command_specs`, `preflight` and `run`; three unit tests pin the command shape, the refusal and the injected program.
- The `test-9p` subcommand and `Test9pStep` in `xtask/src/main.rs`, passing the workspace and the ordinary cargo through; not added to `cargo xtask test`.
- `nix develop -c cargo xtask test-9p` runs the crypto leg and the port `--test aead_transport` leg; its output carries `test aead_transport_tracer_one_sealed_packet ... ok`.
- Pin `test9p-step` records RED (three behavioural failures, not compile errors), GREEN-1, GREEN-2 and ADVANCED.

## Task Commits

Each task was committed atomically (TDD: RED then GREEN):

1. **Task 1 RED: the three failing tests and stubs** - `5dcd512` (test)
2. **Task 1 GREEN: file list, command builder and preflight** - `0b37258` (feat)
3. **Task 2: the `test-9p` subcommand** - `c9617cc` (feat)
4. **Task 2: advance the pin ledger** - `2951100` (docs)

**Plan metadata:** see the final commit on this branch (docs: complete plan).

## Files Created/Modified
- `xtask/src/test9p.rs` — the step module: file list, pure command builder, preflight refusal, `run`, three unit tests
- `xtask/src/lib.rs` — `pub mod test9p;`
- `xtask/src/main.rs` — the `test-9p` subcommand, its dispatch arm and `Test9pStep`
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-17.md` — the `test9p-step` pin ledger

## Decisions Made
- `CommandSpec` is exactly `{ program, args }`; the program is injected by `main.rs` (the `cargo()` helper is private to the bin), so the module reads no environment variable and `command_specs` spawns nothing.
- The port leg lists `--test NAME` per `PORT_TESTS` entry rather than globbing — the list is the contract `preflight` enforces and plan 04-12 completes.
- `run` preflights first, then returns at the first spawn error or non-zero status (T-04-51, T-04-52). The zero-tests-ran guard is deliberately not here; the module doc says so and plan 04-12 adds it.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- The crypto `--lib session` leg matches zero tests at this point: `crypto/tests/session.rs` is plan 04-03's (running in the same wave) and nothing in the `crypto` library is named `session` yet. This is expected and recorded in the ledger; the guard against a leg that ran zero tests is plan 04-12's. The plan's verify (exit 0, tracer line present, no `test-9p:` line) is satisfied.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `cargo xtask test-9p` exists from the first point a wire pin exists; plan 04-12 completes the file list, adds the scalar-backend legs and the zero-tests guard.
- `TestStep::run` in `xtask/src/main.rs` is unchanged and `git diff` from the plan base shows no change under `port/`, `crypto/` or `ds/`.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED
- `xtask/src/test9p.rs` — FOUND
- `evidence/PINS-04-17.md` — FOUND
- commits `5dcd512`, `0b37258`, `c9617cc`, `2951100` — all ancestors of HEAD
