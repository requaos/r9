---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 25
subsystem: infra
tags: [smp, x86_64, aarch64, q35, virt, qemu, boot-test, config, toml, smp-01, smp-04, smp-09]

# Dependency graph
requires:
  - phase: 05-12
    provides: x86_64 CPU 1 bring-up (ap_main, start_aps_and_self_check) and the aarch64 two-CPU reference (05-22) that make -smp 2 boots real on both architectures
provides:
  - "x86_64/lib/config_q35{churn,churnshell}.toml: [qemu].smp = 2 (the last two q35 profiles migrated)"
  - "aarch64/lib/config_virt{,churn,cpu,exec,exec3}.toml: [qemu].smp = 2 (the first five virt profiles migrated)"
affects: [05-13/05-24/05-26 (the parallel Wave-12 config/gate migrations), 05-14 (Owner Stop 2: the -smp 2 boot proofs these configs drive), SMP-08 stress gate]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (20000, confidence: low).
actuals:
  tokens: 845
  tasks: 1
  commits: 1
  plan_head_before: a651459c951dd4d98dd251431e8e63b574159acc
  plan_head_after: cb09c1fbeb3ab588ed4c4c49eb1454fcfa8f8290

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Boot-test profile migration idiom (D-24): one-line [qemu].smp 1 -> 2 per TOML profile, baseline grep gate asserting zero residual single-CPU entries, cargo test -p xtask as the parsing proof"

key-files:
  created: []
  modified:
    - x86_64/lib/config_q35churn.toml
    - x86_64/lib/config_q35churnshell.toml
    - aarch64/lib/config_virt.toml
    - aarch64/lib/config_virtchurn.toml
    - aarch64/lib/config_virtcpu.toml
    - aarch64/lib/config_virtexec.toml
    - aarch64/lib/config_virtexec3.toml

key-decisions:
  - "None beyond the plan: all 7 profiles carried a bare 'smp = 1' line with no stale adjacent single-CPU comment, so each edit is exactly the one-line change the plan names"

patterns-established:
  - "Wave-12 zero-overlap split: this plan owns the 2 remaining q35 profiles + the first 5 virt profiles; 05-13/05-24/05-26 own the disjoint remainder so all run concurrently"

requirements-completed: [SMP-01, SMP-04, SMP-09]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The 2 remaining x86_64 q35 profiles (q35churn, q35churnshell) and the first 5 aarch64 virt profiles (virt, virtchurn, virtcpu, virtexec, virtexec3) default to smp = 2 under [qemu] (SMP-01, SMP-04, SMP-09, D-24)"
    requirement: SMP-01
    verification:
      - kind: automated
        ref: "grep -n 'smp = 1' over the 7 files | grep -v comments | wc -l == 0; grep -n 'smp = 2' shows all 7 lines (q35churn:42, q35churnshell:71, virt:25, virtchurn:49, virtcpu:40, virtexec:28, virtexec3:29)"
        status: pass
    human_judgment: false
  - id: D2
    description: "cargo test -p xtask passes after updating the 7 TOML configs (SMP-09) — xtask parses [qemu].smp = 2 cleanly for every profile"
    requirement: SMP-09
    verification:
      - kind: automated
        ref: "nix develop -c cargo test -p xtask: exit 0 (90 unit + 10 threat-model + 3 doctests, 0 failed)"
        status: pass
    human_judgment: false

# Metrics
duration: 2min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 25: Remaining q35 + First 5 virt Profiles to 2 CPUs Summary

**The last two q35 boot-test profiles (q35churn, q35churnshell) and the first five aarch64 virt profiles (virt, virtchurn, virtcpu, virtexec, virtexec3) now default to `smp = 2` under `[qemu]` — one-line migrations verified by the zero-single-CPU grep gate and a green `cargo test -p xtask` (SMP-01, SMP-04, SMP-09, D-24).**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-26T18:04:17Z
- **Completed:** 2026-11-26T18:06:10Z
- **Tasks:** 1 of 1 executed
- **Files modified:** 7

## Accomplishments

- **SMP-01/SMP-04/SMP-09, D-24 — 7 more boot-test profiles run 2 CPUs by default.** `x86_64/lib/config_q35churn.toml:42`, `x86_64/lib/config_q35churnshell.toml:71`, `aarch64/lib/config_virt.toml:25`, `aarch64/lib/config_virtchurn.toml:49`, `aarch64/lib/config_virtcpu.toml:40`, `aarch64/lib/config_virtexec.toml:28` and `aarch64/lib/config_virtexec3.toml:29` each moved from `smp = 1` to `smp = 2`. None of the 7 files carried a stale adjacent single-CPU comment, so the plan's comment-update clause had nothing to do.
- **Verification green.** The baseline grep gate (`smp = 1` outside comments across all 7 files) counts 0, and `nix develop -c cargo test -p xtask` exits 0 (90 unit tests, 10 threat-model tests, 3 doctests; 0 failed).

## Task Commits

Each task was committed atomically:

1. **Task 1: Update remaining 2 q35 configs and first 5 aarch64 virt configs to 2 CPUs (SMP-01, SMP-04, SMP-09)** — `cb09c1f` (feat)

**Plan metadata:** this SUMMARY's own commit (docs) follows.

## Files Created/Modified

- `x86_64/lib/config_q35churn.toml` — `[qemu].smp` 1 → 2 (q35 churn gate profile)
- `x86_64/lib/config_q35churnshell.toml` — `[qemu].smp` 1 → 2 (composed-venue churn kernel profile)
- `aarch64/lib/config_virt.toml` — `[qemu].smp` 1 → 2 (base virt boot-test profile)
- `aarch64/lib/config_virtchurn.toml` — `[qemu].smp` 1 → 2 (arm64 churn gate profile)
- `aarch64/lib/config_virtcpu.toml` — `[qemu].smp` 1 → 2 (arm64 cpu-fixture kernel profile)
- `aarch64/lib/config_virtexec.toml` — `[qemu].smp` 1 → 2 (virt exec profile, GICv2)
- `aarch64/lib/config_virtexec3.toml` — `[qemu].smp` 1 → 2 (virt exec profile, GICv3)

## Decisions Made

None — followed plan as specified.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. This plan changes config values only; no code paths were added or left unwired.

## Threat Flags

None new. The plan's threat register is implemented as written: T-05-42 (zero residual single-CPU entries in these 7 profiles) is enforced by the baseline grep gate that now counts 0; T-05-SC held (`Cargo.lock` untouched, zero external packages installed). No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes.

## Next Phase Readiness

- **Wave-12 siblings (05-13, 05-24, 05-26):** disjoint file sets by design; this plan's 7 profiles are done, so once the siblings land every `cargo xtask boot-test` profile on both architectures defaults to `-smp 2`.
- **Plan 05-14 (Owner Stop 2):** the migrated profiles are the configs the CI TCG + owner KVM `-smp 2` boot proofs will launch; the `smp: cpu 1 online` / `smp: tlb shootdown ok` markers they assert were built in 05-12 (x86_64) and 05-22 (aarch64).
- **Note for 05-14 gate authors (carried from 05-12):** the x86_64 print carries an `(apic 0x1)` suffix — match the `smp: cpu 1 online` prefix, not the whole line.

## Self-Check: PASSED

- Modified files verified on disk: all 7 TOML profiles contain `smp = 2` (grep output recorded under D1).
- Task commit `cb09c1f` is an ancestor of HEAD (`git merge-base --is-ancestor`).
- Measured commits from the ledger (`gsd-plan-head-before-05-25` = `a651459`): 1 at task completion (the docs commit follows); `plan_head_after` = `cb09c1f`.
- Working tree clean at completion (pre-SUMMARY); `Cargo.lock` untouched (T-05-SC); no file deletions in the task commit; no untracked files left behind.
- Acceptance criteria verified: all 7 listed TOML files set `smp = 2` under `[qemu]`; `nix develop -c cargo test -p xtask` exited 0.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*
