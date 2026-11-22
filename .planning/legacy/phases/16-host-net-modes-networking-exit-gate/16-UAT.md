---
status: passed
phase: 16-host-net-modes-networking-exit-gate
source: [16-VERIFICATION.md]
started: 2026-11-11T15:30:00Z
updated: 2026-11-12T00:00:00Z
---

## Current Test

number: 1
name: Hosted-CI observation of the pushed phase (net-smoke-x64 green on ubuntu-latest/TCG + pin-advance observation at 208ceba)
expected: |
  Push harvey master, then observe on GitHub Actions:
  - the `net-smoke-x64` job (r9-shell.yml) green on ubuntu-latest at TCG;
  - the pin-advance CI observation at r9 208ceba.
awaiting: (none — satisfied)

## Tests

### 1. Hosted-CI observation of the pushed phase
expected: Push harvey master and observe the `net-smoke-x64` CI job green on hosted TCG, plus the pin-advance observation at r9 208ceba.
result: pass — run 43121894436 (r9-shell, pushed master, 2026-11-11T23:42:41Z) completed success with net-smoke-x64 green; boot-smoke + net-smoke green in the same run; three further green runs since; the run's commit carries the phase-16 surface byte-identical to HEAD (verified via gh by the re-run verifier, 16-VERIFICATION.md fingerprint v4:sha256:7282...963).

## Summary

total: 1
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
