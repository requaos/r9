---
status: complete
phase: 04-r9-userspace-exec-harvey-r9-abi
source: [04-VERIFICATION.md]
started: 2026-11-02T19:50:00Z
updated: 2026-11-02T20:25:00Z
---

## Current Test

[testing complete]

## Tests

### 1. SC#2 gate UAT (planner-deferred, 04-06 Task 2)
Review the committed 04-06-sc2-boot.log: confirm the serial log shows banner →
initrd discovery → exec entry → 'init: r9 EL0 alive' → 'hello aarch64 harvey'
(printed by the committed Phase-2 binary via the real EL0 svc path) → clean
all-exited, with zero panic lines.
expected: The ordered marker chain is present; the log is consistent with a real EL0 execution of hello.7.out (sha256 1c89e099…). Verifier note: this gate was RE-RUN live at the pinned r9 HEAD during verification (`cargo xtask boot-test --config virtexec` → PASS, 64940 bytes captured) — the human review confirms a reproducible artifact, not a stale one.
result: pass

### 2. SC#4 gate UAT (planner-deferred, 04-08 Task 2)
Review the committed 04-08-sc4-boot.log: confirm the D-16 narrative ran
end-to-end from EL0 — rfork'd children, note posted through /proc/N/note,
delivery at the child's return boundary, noted(NCONT) resume, await records —
under visible preemption (7 'sched: switch' lines), zero
panic/note-timeout/unimplemented-syscall lines. Confirm the smoke suite
satisfies "rfork, notes, basic fd ops from EL0 without kernel panic" before
Phase 5 planning.
expected: The full marker set is present (smokefork fd ok / child running / await, smokenote got note / resumed, init reaped ×3, smoke complete, all-exited); negative scans are empty. Verifier note: re-run live at the pinned HEAD (PASS, 64940 bytes — byte-count identical to the committed log).
result: pass

### 3. Stack-growth exercise (behavior-unverified truth #9)
Decide whether the present-and-wired stack-growth machinery (runtime-proven
shared zero-page path via bss page-ins) is sufficient for Phase 4 closure, or
require an exercised demonstration (a staged binary that faults below the seed
page) before Phase 5 relies on deep-stack binaries.
expected: Human judgment: accept the machinery as proven-by-shared-path, or schedule a stack-growth smoke. (The growth trigger never fired in any evidence run — the shipped binaries fit the seed page; a kernel-side unit test cannot exercise the trap-context fault path on the host.)
result: pass

## Summary

total: 3
passed: 3
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
