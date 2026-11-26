---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 01
subsystem: infra
tags: [ci, nix, flake, github-actions, xtask, expect, tcl, boot-iso, test-9p]

requires:
  - phase: 01-stage-p0-cross-build-harness (v3.0, archived)
    provides: the witness script and gate map the xtask gate tests pin
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (v3.0, archived)
    provides: test-9p's wire pins and the cargo xtask harness
provides:
  - drawterm-gate.expect start_xvfb with global env (headless CI DISPLAY fix)
  - all 7 gates.yml CI gate jobs failing visibly (no continue-on-error masking)
  - packages.x86_64-linux.boot-iso as a pure Nix derivation (no fixed-output outputHash)
  - CI enforcement of pure boot-iso buildability via nix build --no-link .#boot-iso
  - cargo xtask test covering cargo test -p xtask and all 14 test-9p legs
affects: [phase-05 later plans (CORR-03..05, DEBT-02..05, SMP, MEM), ci, xtask]

actuals:
  tokens: 8426
  tasks: 3
  commits: 3
  plan_head_before: 0d08e6cb86af0bee94f6ab67b4e126692773ecd7
  plan_head_after: b4d05e2b607a5d9d7262de6dbbc2382cea0ef491

tech-stack:
  added: []
  patterns:
    - "CI gate jobs fail visibly: no continue-on-error on witness legs; owner Linux KVM runs remain the Stop 2-4 hardware witness (D-33)"
    - "boot-iso pure derivation: Nix computes output identity from pinned workspace inputs; green CI is the per-commit x86_64 kernel byte-change check (D-05)"

key-files:
  created: []
  modified:
    - gates/x86_64/drawterm-gate.expect
    - .github/workflows/gates.yml
    - flake.nix
    - .github/workflows/xtask.yml
    - xtask/src/gate.rs
    - xtask/src/main.rs
    - xtask/src/test9p.rs

key-decisions:
  - "Placed the pure boot-iso CI verification as a step in the existing boot-test-x86-64 job (nix already installed, x86_64-linux runner) rather than a new job"
  - "test-9p runs as the closing leg of cargo xtask test after the per-package legs: the per-package legs cover the same files but not the per-file zero-test guard or the port scalar-backend twins, so both run"

patterns-established:
  - "Any expect proc touching env(...) declares 'global env' (CORR-01 root-cause pattern, now enforced by the gates.yml failure propagation)"

requirements-completed: [CORR-01, CORR-02, DEBT-01]

coverage:
  - id: D1
    description: "start_xvfb declares global env so headless CI runners set env(DISPLAY) and quick-gate-tcg can pass"
    requirement: CORR-01
    verification:
      - kind: other
        ref: "grep -v '^[[:space:]]*#' gates/x86_64/drawterm-gate.expect | grep -c 'global env run_tag gate_dir' == 1; expect -c 'source gates/x86_64/drawterm-gate.expect' parses cleanly"
        status: pass
    human_judgment: false
  - id: D2
    description: "All 7 CI gate jobs in gates.yml fail visibly when red (continue-on-error removed)"
    requirement: CORR-01
    verification:
      - kind: other
        ref: "grep -v '^[[:space:]]*#' .github/workflows/gates.yml | grep -c 'continue-on-error' == 0"
        status: pass
    human_judgment: true
    rationale: "End-to-end red-visibility of a hosted CI job is only provable on a GitHub runner at push time (flagged assumption: proved at Owner Stop 1, D-32); the local grep proves the masking flag is gone"
  - id: D3
    description: "packages.x86_64-linux.boot-iso builds as a pure derivation with zero outputHash attributes; CI verifies nix build --no-link .#boot-iso"
    requirement: CORR-02
    verification:
      - kind: other
        ref: "grep -v '^[[:space:]]*#' flake.nix | grep -c 'outputHash =' == 0; nix eval --raw .#packages.x86_64-linux.boot-iso.name -> r9-boot-iso-*; nix eval .#packages.x86_64-linux.boot-iso.outputHash fails missing-attribute"
        status: pass
    human_judgment: true
    rationale: "Full x86_64-linux nix build executes on x86_64-linux CI runners at Owner Stop 1 (macOS dev host lacks Linux grub-mkrescue builder); the pure-eval proof is local"
  - id: D4
    description: "cargo test -p xtask passes with archived witness paths fixed; cargo xtask test runs xtask unit tests plus all test-9p legs"
    requirement: DEBT-01
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask (exit 0, 129+90+6+3+1 tests, run twice green)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test (exit 0, xtask legs + 14 test-9p legs, run twice green)"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 01: Course Correction — Green CI, Pure boot-iso, xtask/test-9p Wiring Summary

**CI gate failures unmasked (`global env` fix in drawterm-gate.expect + continue-on-error removed from all 7 gate jobs), `packages.x86_64-linux.boot-iso` converted to a pure Nix derivation with CI build verification, and `cargo xtask test` now runs the xtask unit tests plus all fourteen test-9p legs with the archived witness paths repaired.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-11-26T04:18:31Z
- **Completed:** 2026-11-26T04:26:51Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- **CORR-01:** `gates/x86_64/drawterm-gate.expect` `proc start_xvfb` now declares `global env run_tag gate_dir`, so `set env(DISPLAY) ":$number"` updates the process environment on headless CI runners (the root cause of the red `quick-gate-tcg` at `900f58b`). All 7 gate jobs in `.github/workflows/gates.yml` lost `continue-on-error: true`, so a red hosted gate now turns the workflow red on the commit; the header comment records the Phase 5 CORR-01 / D-01 / D-05 venue discipline with owner Linux KVM runs remaining the Stop 2-4 hardware witness (D-33).
- **CORR-02:** `flake.nix` `packages.x86_64-linux.boot-iso` no longer passes `outputHash` to `builder.bootIso` (which already defaulted `outputHash ? null`), making it a pure derivation whose identity Nix computes from the pinned workspace inputs; the 357-line historical FOD re-derivation log was replaced by a concise CORR-02 note. `.github/workflows/xtask.yml` `boot-test-x86-64` gained a `nix build --no-link .#boot-iso` step, so pure `boot-iso` buildability is enforced on every CI run as the per-commit x86_64 kernel byte-change check (D-05).
- **DEBT-01:** `xtask/src/gate.rs` witness-map tests now point at the archived `.planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` (fixing the two red tests from the v3.0 archive move). `TestStep::run` gained a `cargo test --package xtask` leg and invokes `xtask::test9p::run` after the unit legs, so `cargo xtask test` — and CI's `tests` job — covers the xtask unit tests and all fourteen test-9p legs (v3.0 audit INT-03/INT-04). `xtask/src/test9p.rs`'s doc comment now states the step is the closing leg of `cargo xtask test`; the leg-count assertions are unchanged.

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix start_xvfb global env scope and unmask all 7 gate jobs (CORR-01)** - `473844b` (fix)
2. **Task 2: Convert boot-iso to a pure Nix derivation and add CI build verification (CORR-02)** - `9e82faf` (feat)
3. **Task 3: Fix archived witness paths in xtask/src/gate.rs and wire xtask tests plus test-9p into cargo xtask test (DEBT-01)** - `b4d05e2` (fix)

**Plan metadata:** this SUMMARY commit (docs: complete plan)

## Files Created/Modified

- `gates/x86_64/drawterm-gate.expect` — `global env run_tag gate_dir` in `proc start_xvfb` (line 319)
- `.github/workflows/gates.yml` — `continue-on-error` removed from all 7 gate jobs; header comment updated to Phase 5 CORR-01 / D-01 / D-05
- `flake.nix` — `outputHash` removed from `packages.x86_64-linux.boot-iso`; historical FOD log replaced with CORR-02 note
- `.github/workflows/xtask.yml` — `nix build --no-link .#boot-iso` verification step in `boot-test-x86-64`
- `xtask/src/gate.rs` — archived witness path at lines 885 and 918
- `xtask/src/main.rs` — `TestStep::run` runs `cargo test --package xtask` and `xtask::test9p::run`
- `xtask/src/test9p.rs` — module doc comment updated for DEBT-01 inclusion

## Decisions Made

- **CI step placement:** the pure `boot-iso` build verification rides the existing `boot-test-x86-64` job (which already installs nix on an `x86_64-linux` runner) instead of a new job — one less checkout + nix install per run, and the byte-change check sits next to the boot tests that consume the ISO.
- **test-9p placement:** `xtask::test9p::run` runs as the closing leg after the per-package unit legs. The per-package `port`/`crypto` legs cover the same files in aggregate, but test-9p adds the per-file zero-test guard (WR-08/T-04-39) and the port scalar-backend twins, so both run (the plan's "cleanly after the unit legs" option).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- macOS BSD `sed` rejected the multi-line `c` command for the 357-line comment replacement in `flake.nix`; switched to a Python line-splice with assertions on the boundary lines. No plan impact.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Next Phase Readiness

- `main` CI is positioned to go green: the `quick-gate-tcg` root cause is fixed and no gate job can be silently red again. End-to-end proof lands on the GitHub-hosted runners at Owner Stop 1 (D-32), per the plan's flagged assumptions.
- With CORR-02 landed, green CI on the commit is now the x86_64 kernel byte-change check (D-05) — the manual `outputHash` re-derivation and owner boot-iso check procedure is retired for subsequent Phase 5 plans.
- `cargo xtask test` is the single local and CI entry point for the host suite; later Phase 5 plans (CORR-03 slab, SMP, MEM) inherit a trustworthy test harness.
- `Cargo.lock` unchanged; zero external packages installed (T-05-SC package-legitimacy gate N/A).

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 8 plan files (SUMMARY + 7 modified) exist on disk.
- All 4 commits (473844b, 9e82faf, b4d05e2, 23129ec) are ancestors of HEAD.
- Working tree clean; exactly the 7 target files modified; vendor/harvey untouched.
- Measured commits from ledger: 3 task commits + 1 docs commit = 4 total.
