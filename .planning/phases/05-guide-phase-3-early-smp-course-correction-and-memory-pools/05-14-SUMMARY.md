---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 14
subsystem: gate-migration
tags: [smp, x86_64, aarch64, qemu, expect, gate-scripts, ci, smp-01, smp-04, smp-09, d-24, owner-stop-2]

# Dependency graph
requires:
  - phase: 05-13
    provides: "First 3 x86_64 smoke gates + both nix/gates launchers at -smp 2 (R9_SMP-aware); the migration pattern this plan replicates"
  - phase: 05-24
    provides: "7 core x86_64/lib/config_q35*.toml profiles at smp = 2"
  - phase: 05-25
    provides: "2 remaining q35 + 5 aarch64 virt profiles at smp = 2"
  - phase: 05-26
    provides: "7 remaining aarch64 virt profiles at smp = 2, completing the 21-profile migration"
provides:
  - "All 15 gates/x86_64/*.expect scripts at -smp 2 (7 migrated here: churn-r9, cpu-bringup, drawterm-gate, netsoak-r9, services-r9, stress-r9, tier1-r9)"
  - ".github/workflows/gates.yml: R9_SMP=1 smoke legs per architecture (aarch64 step in boot-r9-smoke, x86_64 job boot-r9-smoke-x64-smp1)"
  - "aarch64 -smp 2 quick pass evidence: smoke (HVF), cpu (HVF), tier-1 (HVF + TCG) all green; R9_SMP=1 smoke green"
  - "Pre-existing aarch64 net-smoke panic logged to deferred-items.md"
affects: [05-15 (AVX/XSAVE, gated on Owner Stop 2), Owner Stop 2 (D-30, D-32, D-33)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (45000, confidence: low).
actuals:
  tokens: 2070
  tasks: 2
  commits: 2
  plan_head_before: 84952bd2ab644a261c1c0b7dba8f5f14a3881976
  plan_head_after: 0df8cef1a1b0975410cd5faa70d1fbde368808ff

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CI single-CPU retention shape: one R9_SMP=1 smoke leg per architecture alongside the -smp 2 golden invariant (D-24)"

key-files:
  created: []
  modified:
    - gates/x86_64/churn-r9.expect
    - gates/x86_64/cpu-bringup.expect
    - gates/x86_64/drawterm-gate.expect
    - gates/x86_64/netsoak-r9.expect
    - gates/x86_64/services-r9.expect
    - gates/x86_64/stress-r9.expect
    - gates/x86_64/tier1-r9.expect
    - .github/workflows/gates.yml
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/deferred-items.md

key-decisions:
  - "The 7 remaining x86_64 gates take hardcoded -smp 2 with NO R9_SMP override — the retained single-CPU legs live only on each arch's smoke-r9.expect and the two nix/gates launchers (05-13's pattern, per the plan's acceptance criteria)"
  - "x86_64 R9_SMP=1 smoke leg is a new CI job (boot-r9-smoke-x64-smp1) since no x86_64 smoke job existed in gates.yml; aarch64 R9_SMP=1 is a step in the existing boot-r9-smoke job"

patterns-established:
  - "SMP-09 CI ordering: gates.yml header records the -smp 2 golden invariant + 1 R9_SMP=1 smoke leg per architecture"

requirements-completed: [SMP-01, SMP-04, SMP-09]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "All 7 remaining x86_64 gate scripts (churn-r9, cpu-bringup, drawterm-gate, netsoak-r9, services-r9, stress-r9, tier1-r9) migrated from -smp 1 to -smp 2; zero non-comment -smp 1 remains under gates/ (SMP-01, SMP-09, D-24)"
    requirement: SMP-01
    verification:
      - kind: automated
        ref: "grep -rn -- '-smp 1' gates/ | grep -v ':[[:space:]]*#' == 0; grep -rnE -- '-smp (2|$r9_smp)' gates/ | grep -v ':[[:space:]]*#' == 15"
        status: pass
    human_judgment: false
  - id: D2
    description: ".github/workflows/gates.yml runs all existing gates at -smp 2 AND keeps one dedicated R9_SMP=1 smoke leg per architecture (SMP-09, D-24)"
    requirement: SMP-09
    verification:
      - kind: automated
        ref: "grep -c 'R9_SMP=1' .github/workflows/gates.yml == 2 (one aarch64 step, one x86_64 job); YAML validated with python3 yaml.safe_load"
        status: pass
    human_judgment: false
  - id: D3
    description: "aarch64 -smp 2 quick pass: smoke (HVF), cpu (HVF), tier-1 (HVF + TCG) all green with smp: cpu 1 online marker; R9_SMP=1 smoke green with no SMP markers (SMP-04, SMP-09)"
    requirement: SMP-04
    verification:
      - kind: automated
        ref: "nix run .#boot-r9-smoke PASS (smp: cpu 1 online + tlb shootdown ok); R9_SMP=1 nix run .#boot-r9-smoke PASS (no smp markers); nix run .#boot-r9-cpu PASS; nix run .#boot-r9-tier1 PASS (HVF); env R9_ACCEL=tcg nix run .#boot-r9-tier1 PASS (TCG); nix develop -c cargo xtask test exit 0; 3 clippy legs exit 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "aarch64 net-smoke passes green at -smp 2"
    requirement: SMP-04
    verification:
      - kind: automated
        ref: "R9_ACCEL=tcg nix run .#boot-r9-net-smoke PASS; R9_ACCEL=tcg R9_SMP=1 PASS; nix run .#boot-r9-net-smoke (HVF) PASS — after c42a66f removed heap allocation from under the NetIrq-ranked INGRESS lock"
        status: pass
    human_judgment: false
  - id: D5
    description: "Owner Stop 2: cumulative port/src diff review, CI push approval, and Linux KVM -smp 2 verification (D-30, D-32, D-33)"
    verification:
      - kind: human
        ref: "Owner approved the cumulative port/src diff (4ff046e..HEAD: 20 files, +2678/-248) and the Part 2 push on 2026-11-26; pushed as 4ff046e..60d7477. The Linux KVM -smp 2 verdict was not supplied at approval time and remains an open owner item (recorded in STATE.md concerns)."
        status: pass
    human_judgment: true

# Metrics
duration: 11min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 14: Gate Migration Completion + Owner Stop 2 Summary

**All 15 x86_64 gate scripts now run at `-smp 2` (7 migrated here), CI runs every gate at `-smp 2` plus one dedicated `R9_SMP=1` smoke leg per architecture, and the aarch64 `-smp 2` quick pass is green on smoke, cpu, tier-1 and net-smoke (HVF + TCG). The net-smoke panic was root-caused to a ranked-lock violation (heap allocation under NetIrq INGRESS) and fixed in `c42a66f`. Owner Stop 2 approved on 2026-11-26 and Part 2 pushed as `4ff046e..60d7477`; the owner's Linux KVM `-smp 2` verdict remains an open owner item (SMP-01, SMP-04, SMP-09, D-24, D-30, D-32, D-33).**

## Performance

- **Duration:** 11 min (plus ~25 min of QEMU boot test time)
- **Started:** 2026-11-26T18:39:22Z
- **Completed:** 2026-11-26T18:51:21Z (Tasks 1-2) + 2026-11-26 (Owner Stop 2 approved)
- **Tasks:** 3 of 3 executed (Task 3 Owner Stop 2 approved by the owner)
- **Files modified:** 9

## Accomplishments

- **SMP-01/SMP-09 — all 15 x86_64 gate scripts at `-smp 2` (Task 1).** `churn-r9`, `cpu-bringup`, `drawterm-gate`, `netsoak-r9`, `services-r9`, `stress-r9` and `tier1-r9` each moved from `-smp 1` to `-smp 2` (hardcoded, per 05-13's pattern). Combined with 05-13's smoke trio and 05-22's aarch64 gates, every gate script in `gates/` now defaults to two CPUs. Zero non-comment `-smp 1` remains; exactly 15 `-smp 2`/`-smp $r9_smp` spawn lines exist.
- **SMP-09/D-24 — CI single-CPU retention (Task 1).** `.github/workflows/gates.yml` gains two `R9_SMP=1` legs: an aarch64 step in the existing `boot-r9-smoke` job, and a new `boot-r9-smoke-x64-smp1` job for x86_64 (explicit TCG). The header comment records the SMP-09 ordering.
- **SMP-04 — aarch64 `-smp 2` quick pass (Task 2).** Smoke (HVF) passed with `smp: cpu 1 online` and `smp: tlb shootdown ok` markers; `R9_SMP=1` smoke passed marker-free; cpu gate passed; tier-1 passed on both HVF and TCG; `cargo xtask test` exit 0; all three clippy legs exit 0.
- **Pre-existing net-smoke panic logged.** `boot-r9-net-smoke` fails identically at `-smp 1` and `-smp 2` (truncated panic during bss page-in) — not SMP-related, not caused by this plan's changes. Logged to `deferred-items.md`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate remaining 7 x86_64 gate scripts to -smp 2 and wire -smp 1 smoke legs in CI (SMP-01, SMP-09)** — `b585f06` (feat)
2. **Task 2: Run full aarch64 -smp 2 quick pass + record pre-existing net-smoke failure** — `0df8cef` (docs)

**Plan metadata:** this SUMMARY's own commit (docs) follows.

## Files Created/Modified

- `gates/x86_64/churn-r9.expect` — spawn line 107: `-smp 1` → `-smp 2`
- `gates/x86_64/cpu-bringup.expect` — spawn line 106: `-smp 1` → `-smp 2`
- `gates/x86_64/drawterm-gate.expect` — spawn line 366: `-smp 1` → `-smp 2`
- `gates/x86_64/netsoak-r9.expect` — spawn line 156: `-smp 1` → `-smp 2`
- `gates/x86_64/services-r9.expect` — spawn line 97: `-smp 1` → `-smp 2`
- `gates/x86_64/stress-r9.expect` — spawn line 177: `-smp 1` → `-smp 2`
- `gates/x86_64/tier1-r9.expect` — spawn line 90: `-smp 1` → `-smp 2`
- `.github/workflows/gates.yml` — header SMP-09/D-24 comment; aarch64 R9_SMP=1 step in boot-r9-smoke; new boot-r9-smoke-x64-smp1 job
- `.planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/deferred-items.md` — pre-existing net-smoke panic entry

## Decisions Made

- **Hardcoded `-smp 2` (no env override) on the 7 remaining x86_64 gates** — the plan's acceptance criteria place the retained single-CPU legs only on each architecture's `smoke-r9.expect` plus the two `nix/gates/` launchers (05-13's pattern, followed exactly).
- **x86_64 R9_SMP=1 smoke leg is a new CI job** (`boot-r9-smoke-x64-smp1`) rather than a step in an existing job, because no x86_64 smoke job existed in gates.yml. The aarch64 leg is a step in the existing `boot-r9-smoke` job.

## Deviations from Plan

None — plan executed exactly as written for Tasks 1 and 2.

## Issues Encountered

- **Pre-existing aarch64 net-smoke panic.** `nix run .#boot-r9-net-smoke` fails with a kernel panic (truncated at `panicked at` — the PL011 console buffer caps the message) during bss page-in. The failure is identical at `-smp 1` and `-smp 2`, confirming it is NOT SMP-related and NOT caused by this plan's changes (x86_64 gate scripts + CI YAML). Logged to `deferred-items.md` for owner triage.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None.

## Threat Flags

None new. The plan's threat register is implemented as written: T-05-25 (repudiation — CI silently staying on 1 CPU) is mitigated by the `-smp 2` defaults plus the dedicated `R9_SMP=1` legs; T-05-SC held (zero external packages installed, `Cargo.lock` untouched). No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes.

## Next Phase Readiness

- **Owner Stop 2 (Task 3): approved by the owner on 2026-11-26.** The cumulative `port/src` diff (`4ff046e..HEAD`: 20 files, +2678/-248), the `UNSAFE-AUDIT.md` diff (+412/-37) and the Part 2 push were approved; Part 2 pushed as `4ff046e..60d7477` so CI proves the x86_64 q35 `-smp 2` TCG gates and the `-smp 1` smoke legs. **Open owner item (D-33):** the x86_64 `-smp 2` Linux KVM verdict was not supplied at approval time; it stays a STATE.md concern until the owner runs `cargo xtask gate --arch x86-64` (+ `R9_SMP=1`) on their x86_64 host.
- **Plan 05-15 (AVX/XSAVE):** unblocked by Owner Stop 2 approval (D-32).
- **net-smoke panic (was "pre-existing"):** corrected — it was a Part 2 regression (heap allocation under the NetIrq-ranked INGRESS lock in `port/src/netif.rs`, latent since 05-08 and made fatal when `66a3548` activated the rank checker). Fixed in `c42a66f`; verified TCG ×3, `R9_SMP=1`, genuine `-smp 1`, and HVF. Debug session archived at `.planning/debug/resolved/aarch64-net-smoke-regression.md`.
- **Gate follow-up:** the net-smoke gate's `expect_before` kills QEMU on the first `panic` chunk, truncating the panic text; worth fixing separately so future panics are readable.

## Self-Check: PASSED

- Files verified: all 7 x86_64 gate scripts contain `-smp 2` on the spawn line; gates.yml contains `R9_SMP=1` legs for both architectures; deferred-items.md has the net-smoke entry.
- Commits verified: `b585f06` (Task 1) and `0df8cef` (Task 2) are both ancestors of HEAD.
- Measured commits from the ledger (`gsd-plan-head-before-05-14` = `84952bd2`): 2 at completion (the docs commit follows); `plan_head_after` = `0df8cef`.
- Plan-level verification re-run on the final tree: `grep -rn -- '-smp 1' gates/ | grep -v ':[[:space:]]*#'` returns 0; `grep -rnE -- '-smp (2|$r9_smp)' gates/ | grep -v ':[[:space:]]*#'` returns 15; `grep -rn 'smp = 1' x86_64/lib/config_q35*.toml aarch64/lib/config_virt*.toml | grep -v ':[[:space:]]*#'` returns 0.
- YAML syntax validated: `python3 yaml.safe_load` on gates.yml passes.
- Working tree clean at completion (pre-SUMMARY); `Cargo.lock` untouched (T-05-SC).
- Requirement IDs SMP-01/SMP-04/SMP-09 are shared with sibling plans: `requirements.ready-ids` gate (#2388) applies — the orchestrator evaluates after the wave merges.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*
