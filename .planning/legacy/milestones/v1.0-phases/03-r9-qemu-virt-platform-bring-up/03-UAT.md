---
status: complete
phase: 03-r9-qemu-virt-platform-bring-up
source: [03-VERIFICATION.md]
started: 2026-11-01T23:59:00Z
updated: 2026-11-02T00:15:00Z
---

## Current Test

[testing complete]

## Tests

### 1. D-10 linux leg of SC#1 — r9 flake devshell reproduces the kernel build on a linux host
expected: |
  On a linux host from r9 55ca7c4: `nix develop` yields pinned nightly-2026-01-30 + qemu 11.1.2 + dtc; `nix develop -c cargo xtask build --arch aarch64` produces the kernel ELF; `nix run .#build` works bare (no devshell). Accel/KVM variance is out of scope for this leg — building only, no boot required.
result: pass

### 2. D-11/D-13 evidence-bundle phase-gate approval
expected: |
  Human reviews and approves the canonical evidence bundle in this phase dir:
  - evidence/virt-boot.log — banner ("r9 from the Internet"), DTB line at 0xffff800040000000, ≥3 periodic ticks, typed probe string echoed via the IRQ path
  - evidence/virtpanic-boot.log — full TrapFrame register dump (31 rows, ec: Ok(Brk)) + "kernel halted" clean-halt marker, zero IRQ interleaving
  - evidence/EVIDENCE.md — integrity record: r9 commit SHA (55ca7c4), qemu 11.1.2, host darwin-aarch64, exact commands, per-assertion results
  Mechanical integrity is already machine-proven: the verifier reproduced both logs byte-for-byte from r9 HEAD via `cargo xtask boot-test --config virt` / `--config virtpanic`. This test is the approval act itself (D-11/D-13 phase gate, same shape as the 02-05 gate).
result: pass

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
