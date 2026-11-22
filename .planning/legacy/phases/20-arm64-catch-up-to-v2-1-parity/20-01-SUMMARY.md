---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 01
subsystem: r9-aarch64-kernel + arm64-gates
tags: [r9, aarch64, notes, devcons, user-hooks, expect, tracer]
status: complete
requires: []
provides:
  - "r9 aarch64 virtshell compiles again (note_pending in PipeHooks/DevipHooks)"
  - "push_note readies a Blocked target on aarch64 (donor postnote ready(p))"
  - "per-proc Proc.user + devcons UserHooks registered on aarch64 (no `hooks not registered`)"
  - "cargo test -p port --no-fail-fast fully green"
  - "three arm64 gates fail fast on `pushword but no argv` and assert absence of hook/unknown-device/unsupported-era lines"
affects: [20-02, 20-03, 20-04, 20-08, 20-15]
tech-stack:
  added: []
  patterns: ["x86_64 proc.rs template copied verbatim to aarch64", "whole-capture absence post-pass in expect drivers"]
key-files:
  created: []
  modified:
    - /Users/neilskinner/code/r9/aarch64/src/proc.rs
    - /Users/neilskinner/code/r9/aarch64/src/main.rs
    - /Users/neilskinner/code/r9/port/tests/devenv_test.rs
    - build/aarch64/smoke-r9.expect
    - build/aarch64/smoke-r9-net.expect
    - build/aarch64/tier1-r9.expect
decisions:
  - "The devenv_test rename fix covers two stale 10-byte literals (write count and stat length), not one"
  - "All nine baseline counts were 0, so all three absence checks went into all three arm64 drivers"
metrics:
  duration: "about 25 min"
  completed: 2026-11-17
actuals:
  tokens: 3956
  tasks: 3
  commits: 1
plan_head_before: c50c282bd612350d384579456b3fdc224ff738c9
plan_head_after: 3a2d3bc07e098a946451950716fd4b937b523fe5
r9_head_before: 8e4e730
r9_head_after: 46364d0
r9_commits: 2
---

# Phase 20 Plan 01: arm64 tracer, note fix, user hooks, gate fast-fail Summary

The r9 aarch64 `virtshell` kernel builds again. It now has the full donor note semantics (`note_pending` probe plus the `push_note` arm that readies a Blocked target) and per-proc `#c/user` hooks. The composed TCG smoke against the local r9 checkout passes with zero `hooks not registered` lines, `cargo test -p port` is fully green, and all three arm64 gates now fail within seconds on the rc argv abort.

`commits: 1` counts Harvey commits only (measured, `c50c282bd..HEAD`). The plan also made 2 local r9 commits (`8e4e730..46364d0`), so the cross-repo total is 3, as the plan expected.

## Tasks

| Task | Name | Repo | Commit | Files |
|------|------|------|--------|-------|
| 1 | End-to-end note fix + user hooks (tracer) | r9 | 1b5dd74 | aarch64/src/main.rs, aarch64/src/proc.rs |
| 2 | Rename test literal | r9 | 46364d0 | port/tests/devenv_test.rs |
| 3 | Gate post-passes + pushword fast-fail | Harvey | 3a2d3bc07 | build/aarch64/{smoke-r9,smoke-r9-net,tier1-r9}.expect |

## Evidence

- **Task 1 build:** `cargo xtask build --arch aarch64 --config virtshell` printed `VIRTSHELL-BUILD-OK` at 1b5dd74. The new code adds no warnings.
- **Task 1 tracer (TCG, one run):** `boot-r9-smoke` with `--override-input r9 git+file:///Users/neilskinner/code/r9` resolved to `rev=1b5dd74b8ad6…` and built `r9-kernel-virt-1b5dd74`. It printed `ALL STAGES PASS` and exited 0. The capture had 0 `hooks not registered` lines and 1 `devenv: seeded cputype=arm64` line, and `ps` shows procs as `glenda`. Tracer feedback gate: the run was re-verified end to end and passed, so the plan went on to the next tasks.
- **Task 1 source check:** `note_pending` appears 2 times in non-comment lines of main.rs.
- **Task 2:** `cargo test -p port --no-fail-fast` gave 13 `test result: ok` lines, 231 tests passed, 0 failed, with no `FAILED` or `panicked` lines.
- **Task 3 baseline (TCG, local r9 kernel, before the driver edits):**

  | Capture | `not registered` | `unknown device` | `unsupported this era` |
  |---------|------------------|------------------|------------------------|
  | smoke-r9.log (at 1b5dd74) | 0 | 0 | 0 |
  | smoke-r9-net.log (at 46364d0) | 0 | 0 | 0 |
  | tier1-r9.log (at 46364d0) | 0 | 0 | 0 |

  Every baseline was 0, so no offending lines are recorded and there is nothing to hand to 20-08 or 20-15 from this measurement.
- **Task 3 gates (TCG, edited drivers, local r9 at 46364d0):** `boot-r9-smoke exit=0`, `boot-r9-net-smoke exit=0`, `boot-r9-tier1 exit=0`. Each printed the three new `stage ok` lines and a final PASS line that lists them. Outputs are in `.build/p20-01-*.out`.
- **Task 3 negative check:** a synthetic spawn that prints `rc: pushword but no argv!` against the tier1 driver's `expect_before` block exited 1 in about 1 s with `tier1-r9: rc argv-stack abort on the wire (pushword but no argv) — WINDOWS #49`.
- `git -C ../r9 log origin/main..HEAD` lists exactly 1b5dd74 and 46364d0. Nothing was pushed in either repo.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Second stale rename literal in devenv_test.rs**
- **Found during:** Task 2
- **Issue:** after line 267 was changed to `assert_eq!(n, 8)`, the test still failed at line 293: `assert_eq!(length, 10, "stat length is the value's length")` got 8. Both literals come from the same 10-byte `aarch64-r9` → 8-byte `arm64-r9` rename.
- **Fix:** changed it to `assert_eq!(length, 8, …)`. Both edits are in the same file and the same commit.
- **Files modified:** /Users/neilskinner/code/r9/port/tests/devenv_test.rs
- **Commit:** 46364d0

### Notes

- The protocol 0c commit ledger was written after the Harvey commit, not before it. The base `c50c282bd` is the HEAD recorded at plan start, so the measured count is the same.
- `.planning/config.json` changed during execution (`use_worktrees: false` → `true`). This plan did not make that change, and the file was not staged.
- The prohibitions held: no edit under `r9/port/src/`, no N-run series, no new gate, explicit-path staging only, no attribution, nothing pushed, and the marker `hello aarch64 harvey` was not touched.

## Threat Flags

None. T-20-01: the eve gate in `port/src/devcons.rs` is untouched and this plan only registers hooks. T-20-02: `child.user = parent.user.clone()` gives each proc its own `Vec<u8>`, with no Arc. T-20-03: the ready arm is the x86_64 one, verbatim and limited to `ProcState::Blocked`.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: r9 1b5dd74 (aarch64/src/main.rs, aarch64/src/proc.rs only)
- FOUND: r9 46364d0 (port/tests/devenv_test.rs only)
- FOUND: Harvey 3a2d3bc07 (the three expect files only)
- FOUND: all six modified files
