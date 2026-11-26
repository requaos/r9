---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 24
subsystem: x86_64-smp
tags: [smp, x86_64, q35, qemu-config, boot-test, ci, smp-01, smp-09]

# Dependency graph
requires:
  - phase: 05-12
    provides: x86_64 two-CPU bring-up (MADT-driven start_aps, ap_main at BSP parity, per-CPU LAPIC timers, boot-time TLB shootdown self-check, -smp 1 clean skip) that the -smp 2 profiles now exercise
provides:
  - "All 7 core x86_64/lib/config_q35*.toml profiles (config_q35, config_q35lowmcfg, config_q35net, config_q35auth, config_q35cpu, config_q35exec, config_q35shell) default [qemu].smp to 2"
  - "CI boot-test-x86-64 (.github/workflows/xtask.yml) profiles q35/q35lowmcfg/q35net now launch QEMU with -smp 2"
  - "Stale SMP comments corrected: config_q35.toml no longer claims 'SMP stays a Future item'; config_q35cpu.toml venue line reads smp 2"
affects: [05-14 (Owner Stop 2: the live x86_64 -smp 2 boot proof on CI TCG + owner KVM runs these profiles), 05-25/05-26 (the remaining config migration: churn pair + aarch64 virt profiles), SMP-08 stress gate]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (20000, confidence: low).
actuals:
  tokens: 895
  tasks: 1
  commits: 1
  plan_head_before: a651459c951dd4d98dd251431e8e63b574159acc
  plan_head_after: 0f81f19d8c9d74de05bcea04d138d876f205c4b0

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TOML profile SMP migration shape: [qemu].smp flips to 2 and any adjacent comment claiming single-CPU/future-SMP status is reworded in the same edit so comments stay truthful (D-24)"

key-files:
  created: []
  modified:
    - x86_64/lib/config_q35.toml
    - x86_64/lib/config_q35lowmcfg.toml
    - x86_64/lib/config_q35net.toml
    - x86_64/lib/config_q35auth.toml
    - x86_64/lib/config_q35cpu.toml
    - x86_64/lib/config_q35exec.toml
    - x86_64/lib/config_q35shell.toml

key-decisions:
  - "config_q35.toml's D-01 comment reworded in place: '-smp 1 (SMP stays a Future item)' became '-smp 2 (05-24, SMP-01/SMP-09/D-24: two CPUs by default now that both schedule)' — the historical D-01 attribution is kept, the stale claim removed"
  - "config_q35cpu.toml's venue comment 'smp 1, 1 GiB' updated to 'smp 2, 1 GiB' (comment truthfulness per the task's action step 1)"
  - "config_q35churn.toml and config_q35churnshell.toml deliberately untouched at smp = 1 — they are Plan 05-25's files_modified; this plan stays strictly within its 7-file list (D-24's <= 8 files per plan)"

patterns-established:
  - "Comment-truthfulness rule on config migration: a [qemu].smp flip carries its stale adjacent comments in the same commit, never left for a later pass"

requirements-completed: [SMP-01, SMP-09]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "All 7 core x86_64/lib/config_q35*.toml profiles (including the 3 CI boot-test-x86-64 profiles config_q35, config_q35lowmcfg, config_q35net) set smp = 2 under [qemu], with zero non-comment 'smp = 1' lines remaining and stale SMP-future comments corrected (SMP-01, SMP-09, D-24)"
    requirement: SMP-09
    verification:
      - kind: automated
        ref: "grep -n 'smp = 1' on the 7 files | grep -v comments | wc -l == 0; grep -c '^smp = 2' == 1 per file; nix develop -c cargo test -p xtask rc 0 (233 tests: 129 lib + 1 bin + 90 main + 10 threat_model + 3 doctests, 0 failed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The live -smp 2 boot of these profiles (CPU 1 online, shootdown ack, rio green under QEMU q35 TCG/KVM)"
    requirement: SMP-01
    verification: []
    human_judgment: true
    rationale: "x86_64 guests cannot boot on this aarch64 macOS dev host (the 05-11/05-12-recorded constraint: grub-mkrescue and the authdomain generator are Linux-only); the live boot proof of the flipped profiles is Plan 05-14's scope on CI TCG + the owner's KVM host (D-33). This plan's must_haves cover the TOML edits and the xtask test gate only."

# Metrics
duration: 2min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 24: Core q35 Profiles to -smp 2 Summary

**All 7 core `x86_64/lib/config_q35*.toml` QEMU profiles — including the 3 that CI's `boot-test-x86-64` runs (`config_q35`, `config_q35lowmcfg`, `config_q35net`) — now default `[qemu].smp` to `2`, with the stale "SMP stays a Future item" and "smp 1" comments corrected, and `nix develop -c cargo test -p xtask` green at 233/233 (SMP-01, SMP-09, D-24).**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-26T18:05:53Z
- **Completed:** 2026-11-26T18:07:20Z
- **Tasks:** 1 of 1 executed
- **Files modified:** 7 (0 created)

## Accomplishments

- **SMP-01/SMP-09/D-24 — the 7 core q35 profiles default to 2 CPUs.** `config_q35`, `config_q35lowmcfg`, `config_q35net`, `config_q35auth`, `config_q35cpu`, `config_q35exec` and `config_q35shell` all set `smp = 2` under `[qemu]`. The baseline grep gate confirms zero non-comment `smp = 1` lines remain across the 7 files, and each carries exactly one `smp = 2` line.
- **CI moves with them.** `.github/workflows/xtask.yml`'s `boot-test-x86-64` job runs `cargo xtask boot-test` on `q35`, `q35lowmcfg` and `q35net` — all three now launch QEMU with `-smp 2`, alongside the Wave 12 expect-gate scripts and ahead of Owner Stop 2 (Plan 05-14), which owns the live boot proof (D-33).
- **Comments stay truthful.** `config_q35.toml`'s `[qemu]` comment no longer claims "SMP stays a Future item" (it now records `05-24, SMP-01/SMP-09/D-24` as the flip's provenance); `config_q35cpu.toml`'s venue line reads `smp 2, 1 GiB`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Update 7 core x86_64/lib/config_q35*.toml profiles to 2 CPUs and verify xtask tests (SMP-01, SMP-09)** — `0f81f19` (feat)

**Plan metadata:** this SUMMARY's own commit (docs) follows.

## Files Created/Modified

- `x86_64/lib/config_q35.toml` — `[qemu].smp` 1 → 2; D-01 comment reworded (SMP no longer a future item; flip provenance recorded)
- `x86_64/lib/config_q35lowmcfg.toml` — `[qemu].smp` 1 → 2
- `x86_64/lib/config_q35net.toml` — `[qemu].smp` 1 → 2
- `x86_64/lib/config_q35auth.toml` — `[qemu].smp` 1 → 2
- `x86_64/lib/config_q35cpu.toml` — `[qemu].smp` 1 → 2; venue comment `smp 1` → `smp 2`
- `x86_64/lib/config_q35exec.toml` — `[qemu].smp` 1 → 2
- `x86_64/lib/config_q35shell.toml` — `[qemu].smp` 1 → 2

## Decisions Made

- **The churn pair (`config_q35churn.toml`, `config_q35churnshell.toml`) stays at `smp = 1` in this plan** — they are Plan 05-25's `files_modified`; touching them here would break the phase's `<= 8` files-per-plan discipline (D-24) and collide with the sibling Wave 12 agent.
- **Comment provenance style:** the reworded `config_q35.toml` comment names the plan and requirement IDs (`05-24, SMP-01/SMP-09/D-24`) rather than silently deleting the historical D-01 attribution.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Both `<automated>` verifies ran green on the first pass: the baseline grep returned `0`, and `nix develop -c cargo test -p xtask` exited 0 (233 tests, 0 failed — embedded TOML parsing and the `r9-netdev-args` unit tests included).

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. The flipped profiles are complete and functional; the live `-smp 2` boot that consumes them is Plan 05-14's scope by design (this plan's must_haves cover the TOML edits plus the xtask test gate).

The broken-windows ledger (`.planning/WINDOWS.md`) has no entries to add: no stubs, no skipped tests, no unrun `<verify>` — both `<automated>` verifies ran and passed.

## Threat Flags

None new. The plan's threat register is implemented as written: T-05-41 (repudiation — CI silently staying on 1 CPU) is mitigated by exactly this flip plus the zero-single-CPU baseline grep gate, both landed. T-05-SC held: `Cargo.lock` is byte-unchanged, zero external packages installed. No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes.

## Next Phase Readiness

- **Plan 05-14 (Owner Stop 2):** the CI `boot-test-x86-64` profiles now launch at `-smp 2`, so the live x86_64 two-CPU boot proof on CI TCG + owner KVM exercises these configs directly.
- **Plans 05-25/05-26 (remaining config migration):** the churn pair and all aarch64 virt profiles follow the same shape established here; nothing in this plan blocks them.
- **Requirements note:** SMP-01 and SMP-09 are shared with sibling plans (05-14/05-25/05-26) and were NOT marked complete here per the shared-ID gate (#2388) — they flip `Complete` when the last declaring plan summarizes.

## Self-Check: PASSED

- All 7 modified files exist on disk (verified with `[ -f ]`-equivalent grep reads above): `x86_64/lib/config_q35.toml`, `config_q35lowmcfg.toml`, `config_q35net.toml`, `config_q35auth.toml`, `config_q35cpu.toml`, `config_q35exec.toml`, `config_q35shell.toml`.
- The task commit is an ancestor of HEAD (verified with `git merge-base --is-ancestor` at commit time): `0f81f19` (T1).
- Content pins verified: `smp = 2` present exactly once per file; zero non-comment `smp = 1` lines across the 7 files; out-of-scope churn pair untouched at `smp = 1`.
- Measured commits from ledger (`gsd-plan-head-before-05-24` = `a651459c`): 1 at SUMMARY write (the docs commit follows); `plan_head_after` = `0f81f19d`.
- Working tree clean before this SUMMARY's commit; zero generated files left untracked; `Cargo.lock` untouched (T-05-SC).
- Acceptance criteria both verified: (1) all 7 files set `smp = 2` under `[qemu]`; (2) `nix develop -c cargo test -p xtask` exits 0 (129 lib + 1 bin + 90 main + 10 threat_model + 3 doctests = 233 passed, 0 failed).

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*
