---
gsd_state_version: '1.0'
milestone: v3.0
milestone_name: Cross-Build Harness and Wire Foundations
status: planning
progress:
  total_phases: 4
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-11-22)

**Core value:** The OS boots to rio on QEMU q35 at every commit.
**Current focus:** Phase 1: Stage P0 - Cross-Build Harness

## Current Position

Phase: 1 of 16 (Stage P0 - Cross-Build Harness); active milestone v3.0 covers Phases 1-4
Plan: 0 of TBD in current phase
Status: Ready to plan
Last activity: 2026-11-22 — Roadmap created from ingest; W11 decided by the owner

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**
- Total plans completed: 0

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

*Updated after each plan completion*

## Accumulated Context

### Decisions

Full log is the PROJECT.md Key Decisions table. Recent:

- [Owner 2026-11-22] `RFNOLEGACY` is a fourth `rfork` flag, `1 << 21`, assigned in guide Phase 0 (GSD Phase 2) with `RFCKEY` 18, `RFROBASE` 19, `RFRANDMEM` 20. Resolves warning W11; the guide's "3 flag bits" wording is corrected to four. Reserved in r9 only (`x86_64/src/proc.rs`, `aarch64/src/syscall.rs`); Harvey is not edited. Recorded deviation from the guide's "reserve bits 18-20 in `libc.h`". Phase 6 implements the behaviour.
- [Roadmap] Phase order is the owner's migration plan; Stage P0 is Phase 1; "Milestone S" and "Milestone STD" are source labels for Phases 8 and 9.
- [Ingest] Precedence IG > AC > RFC > PT. Nothing is locked; the guide's "non-negotiable" decisions are constraints pending confirmation in Phase 2.

### Open Questions (ten warnings, approved to be carried, not resolved)

Settle each when its phase is planned. Detail: `.planning/INGEST-CONFLICTS.md`. Eight roadmapper notes (N1-N8) are in PROJECT.md.

| ID | Settle in | Question |
|----|-----------|----------|
| W3 | Phase 3 | MAC primitive: Keyed BLAKE3 only, or where Poly1305 applies (also read by Phase 4) |
| W5 | Phase 3 | `unsafe` in `ds/src/tpht/` versus `forbid(unsafe_code)` on `ds/` |
| W6 | Phase 3 | TPHT home block 4 versus 16 slots |
| W1 | Phase 4 | Wire header layout and AAD contents (`Pad`, `ExtTLVs Length`) |
| W2 | Phase 4 | 24-byte `StatelessCap` in an 8-byte `FidHandle` field |
| W4 | Phase 4 | Session key scope: per FID versus per connection and direction |
| W7 | Phase 6 | `DiscoveryBeaconV2` 192 versus 202 bytes |
| W8 | Phase 13 | `/adm/users` UID 0 `adm` versus persona `glenda` |
| W9 | Phase 13 | Test 3.2 needs `alice` absent from `/adm/users` |
| W10 | Phase 13 | Suite 3 cross-VLAN mounts with no route |

### Pending Todos

None yet.

### Blockers/Concerns

- Three commits are unpushed by the owner's choice: r9 `5b8b70a` (planning documents plus `legacy/`, 53 MB) and `6ee45b5` (handoff note), and Harvey `25d97dc4d` (freeze note). Confirmed by git at roadmap time. The orchestrator's planning-bootstrap commit is also local until the owner approves a push. The owner has not decided whether `legacy/` stays in r9 history.
- Owed checks at checkpoint r9 `116bcc2`: the owner's linux `nix build .#boot-iso --rebuild` (expected `sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=`), and hosted CI on r9 `116bcc2` and Harvey `bd7a6982a`.
- Phase 20 of the Harvey-rooted work never ran `/gsd-secure-phase` (no SECURITY.md).
- Docs pin r9 at `c787ce9`; HEAD is 60 commits later with no `crypto/`, `ds/`, `kern/`, `sys/`, `venti/`. Re-verify every landing-site path when its phase is planned.

### Standing Owner Rules

- Pushes in either repo need explicit owner approval; check what is unpushed and push only the approved commits.
- Edits under r9 `port/src` need owner approval (show the diff first). `aarch64/`, `x86_64/`, `xtask/` edits do not.
- x86_64 kernel byte change: re-derive and commit the `boot-iso` `outputHash`, run the x64 TCG gates in an amd64 Docker container (clean up afterwards), and get the owner's linux boot-iso check.
- aarch64 kernel byte change: quick pass now (tier-1 on HVF and TCG; smoke, net-smoke, cpu once on HVF); full HVF series at Phase 8. Kernel builds are reproducible, so compare kernel sha256 to prove a pin move changes nothing.
- Kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is per process (save at switch, copy on fork, reset on exec); arm64 is the reference for x86_64.
- No AI attribution in commits or PRs. Never stage the 11 pre-modified troff/postscript font files in Harvey (a darwin case-collision artifact).

### Practical Notes

`gh` is not installed (`nix run nixpkgs#gh -- ...`). GNU sed is on PATH. macOS Unix socket paths cap at 104 bytes: run boot tests from a short path. The lima VM `harvey` is the Harvey namespace build venue (`core.ignorecase=false` on purpose). r9 exec caps each argument at 128 bytes; the PL011 console buffer is 256 bytes. Debug history: `.planning/legacy/debug/knowledge-base.md` (frozen).

## Deferred Items

Items carried over from the legacy-final checkpoint (HANDOFF.md):

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| Verification | Full arm64 HVF series (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each; about 4.4 h) | Deferred by owner to Phase 8 (SMP-05) | legacy-final | v3.2 |
| Defect | WINDOWS #70: aarch64 scheduler resume skips note delivery | Deferred to Phase 8 (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #49: I-cache, root-caused and fixed | Left open until the full series runs (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #66: x86_64 triple fault under 8-way soak | Fix on an x86_64 host venue (SMP-06) | legacy-final | v3.2 |
| Test gap | `#!` recursion cap has no host or live test | Unscheduled; add a test when exec is next touched | legacy-final | - |
| Prerequisite | SMP: aarch64 per-CPU current frame, kernel stack, proc pointers; x86_64 per-CPU `Mach` mapping (FP ownership already per-CPU) | Phase 8 (SMP-02) | legacy-final | v3.2 |

The WINDOWS ledger (51 open, 4 waived, 17 fixed, 72 total) is frozen at `.planning/legacy/WINDOWS.md`; re-home open entries to a live file before updating them.

## Session Continuity

Last session: 2026-11-22
Stopped at: Roadmap, requirements and state written from the ingest; ready to plan Phase 1
Resume file: None (`.planning/HANDOFF.md` says to delete it once the first r9 planning pass has absorbed it; it is left in place for the owner)
