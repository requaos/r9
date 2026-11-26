---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 26
subsystem: infra
tags: [smp, aarch64, qemu-virt, toml, boot-test, smp-04, smp-09, d-24]

# Dependency graph
requires:
  - phase: 05-23
    provides: 2-CPU aarch64 virt DTB pairs (virt-gicv2.{dts,dtb}, virt-gicv3.{dts,dtb}, virt-gicv2-lowecam.{dts,dtb}) carrying /cpus/cpu@1 with PSCI enable-method, so -smp 2 boots discover the secondary CPU
  - phase: 05-12
    provides: aarch64 SMP bring-up machinery that smp = 2 exercises (per this plan's depends_on)
provides:
  - aarch64/lib/config_virt{ip,lowecam,multi,net,notes,panic,shell}.toml defaulting to [qemu].smp = 2, completing the 21-profile x86_64+aarch64 smp = 2 migration together with Plans 05-24/05-25 (D-24)
affects: [05-14 Owner Stop 2, every cargo xtask boot-test run on the 7 aarch64 virt profiles]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (20000, confidence: low).
actuals:
  tokens: 923
  tasks: 1
  commits: 1
  plan_head_before: a651459c951dd4d98dd251431e8e63b574159acc
  plan_head_after: 5cd58f7668fa6fe3b08e8c675d2411c30db88112

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TOML boot-test profile CPU-count migration: [qemu].smp = 2 per D-24, verified by a baseline grep gate (zero non-comment 'smp = 1' lines) plus cargo test -p xtask"

key-files:
  created: []
  modified:
    - aarch64/lib/config_virtip.toml
    - aarch64/lib/config_virtlowecam.toml
    - aarch64/lib/config_virtmulti.toml
    - aarch64/lib/config_virtnet.toml
    - aarch64/lib/config_virtnotes.toml
    - aarch64/lib/config_virtpanic.toml
    - aarch64/lib/config_virtshell.toml

key-decisions: []

patterns-established: []

requirements-completed: [SMP-04, SMP-09]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "All 7 remaining aarch64/lib/config_virt{ip,lowecam,multi,net,notes,panic,shell}.toml profiles configure smp = 2 under [qemu]"
    requirement: SMP-04
    verification:
      - kind: automated_ui
        ref: "grep -n 'smp = 1' on the 7 files (non-comment lines) → 0; grep -c 'smp = 2' → 1 per file"
        status: pass
    human_judgment: false
  - id: D2
    description: "cargo test -p xtask passes after updating the 7 aarch64 virt TOML configs"
    requirement: SMP-09
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask → exit 0, 233 passed / 0 failed across 5 suites"
        status: pass
    human_judgment: false

# Metrics
duration: 1 min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 26: Remaining aarch64 virt TOML profiles to smp = 2 Summary

**Final 7 `aarch64/lib/config_virt{ip,lowecam,multi,net,notes,panic,shell}.toml` boot-test profiles migrated from `smp = 1` to `smp = 2` under `[qemu]`, completing the 21-profile x86_64+aarch64 two-CPU default migration (SMP-04, SMP-09, D-24) alongside Wave-12 siblings 05-24/05-25.**

## Performance

- **Duration:** 1 min
- **Started:** 2026-11-26T18:06:19Z
- **Completed:** 2026-11-26T18:07:49Z
- **Tasks:** 1
- **Files modified:** 7

## Accomplishments

- All 7 remaining `aarch64/lib/config_virt*.toml` profiles (`virtip`, `virtlowecam`, `virtmulti`, `virtnet`, `virtnotes`, `virtpanic`, `virtshell`) now default to `smp = 2` under `[qemu]` — the last 7 of the 21 TOML boot-test profiles migrated across Plans 05-24..26
- Baseline grep gate passes: zero non-comment `smp = 1` entries remain in the 7 files; exactly one `smp = 2` per file
- `nix develop -c cargo test -p xtask` exits 0: 233 passed / 0 failed across all 5 suites (129 + 1 + 90 + 10 + 3)
- `virtlowecam` now pairs its `smp = 2` with the 2-CPU `virt-gicv2-lowecam.dtb` regenerated in Plan 05-23; the other six profiles ride the 2-CPU `virt-gicv2.dtb` from the same plan

## Task Commits

Each task was committed atomically:

1. **Task 1: Update remaining 7 aarch64/lib/config_virt*.toml profiles to 2 CPUs (SMP-04, SMP-09)** - `5cd58f7` (feat)

## Files Created/Modified

- `aarch64/lib/config_virtip.toml` - `[qemu].smp` 1 → 2 (NETIP-03 loopback gate profile)
- `aarch64/lib/config_virtlowecam.toml` - `[qemu].smp` 1 → 2 (NETDRV-01 moved-window perturbation gate; pairs with 2-CPU low-ECAM DTB from 05-23)
- `aarch64/lib/config_virtmulti.toml` - `[qemu].smp` 1 → 2 (MULTI profile, staged-init rfork gate)
- `aarch64/lib/config_virtnet.toml` - `[qemu].smp` 1 → 2 (NETDRV-03/04 net-gate profile)
- `aarch64/lib/config_virtnotes.toml` - `[qemu].smp` 1 → 2 (NOTES profile, note-delivery gate)
- `aarch64/lib/config_virtpanic.toml` - `[qemu].smp` 1 → 2 (debug-artifact deliberate-panic profile)
- `aarch64/lib/config_virtshell.toml` - `[qemu].smp` 1 → 2 (shell-chain profile, Harvey compose boot)

## Decisions Made

None - followed plan as specified. The adjacent comments above each `smp` line describe the TCG reference CPU (`cpu = "cortex-a72"` / HVF swap), not CPU count, so no stale single-CPU comment updates were needed — verified by grep before editing.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The plan's grep gate comment `[HEAD=7 EXPECT==0]` matched reality exactly: 7 `smp = 1` lines at HEAD, 0 after the edit.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None - this plan changes TOML configuration values only; no code paths, placeholders, or unwired data sources.

## Next Phase Readiness

- With Wave-12 siblings 05-24 (7 x86_64 q35 profiles) and 05-25 (2 x86_64 + 5 aarch64 profiles) merged, all 21 `x86_64/lib/config_q35*.toml` + `aarch64/lib/config_virt*.toml` boot-test profiles default to `-smp 2`, satisfying this plan's success criterion and D-24's gate migration ahead of Owner Stop 2 (Plan 05-14)
- Requirements SMP-04 and SMP-09 were NOT marked complete here: the shared-ID gate (#2388) correctly blocked both because sibling plans 05-24/05-25 also declare them and had no SUMMARY in this worktree at execution time — the orchestrator re-evaluates after the wave merges

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 7 modified TOML files exist on disk and contain `smp = 2` (7/7 grep hits)
- SUMMARY.md exists at `.planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-26-SUMMARY.md`
- Task commit `5cd58f7` is an ancestor of HEAD
- Acceptance criteria re-verified: zero non-comment `smp = 1` lines in the 7 files; `nix develop -c cargo test -p xtask` exit 0 (233 passed / 0 failed)
