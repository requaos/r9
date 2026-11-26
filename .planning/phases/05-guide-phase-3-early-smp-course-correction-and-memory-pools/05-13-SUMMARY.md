---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 13
subsystem: gate-migration
tags: [smp, aarch64, x86_64, qemu, expect, gate-scripts, r9-smp, smp-01, smp-04, smp-09, d-24]

# Dependency graph
requires:
  - phase: 05-22
    provides: gates/aarch64/smoke-r9.expect already R9_SMP-aware (default 2, smp-online/smp-shootdown stages) — the pattern this plan replicates; the proven aarch64 two-CPU boot the aarch64 gates now exercise by default
  - phase: 05-12
    provides: x86_64 CPU 1 bring-up (ap_main, start_aps_and_self_check, the 'smp: cpu 1 online (apic 0x1)' / 'smp: tlb shootdown ok' boot markers) that the migrated x86_64 gates now boot by default
provides:
  - "gates/aarch64/{cpu-r9,smoke-r9-net,tier1-r9}.expect: -smp 2 default (hardcoded; the retained single-CPU aarch64 leg lives on 05-22's smoke-r9.expect)"
  - "nix/gates/boot-r9.sh: -smp \"${R9_SMP:-2}\" with the R9_SMP env contract documented (spawned by drive-r9.expect)"
  - "gates/x86_64/smoke-r9.expect: R9_SMP env (default 2), -smp $r9_smp on BOTH spawn legs (chain + cpu-chain), and the smp-online stage asserting 'smp: cpu 1 online' when $r9_smp >= 2"
  - "gates/x86_64/{smoke-r9-net,smoke-r9-auth}.expect: -smp 2 default (hardcoded)"
  - "nix/gates/boot-r9-x64.sh: -smp \"${R9_SMP:-2}\" with the R9_SMP env contract documented"
affects: [05-14 (the remaining 7 x86_64 gates + the live x86_64 -smp 2 boot proof on CI TCG / owner KVM), Owner Stop 2 (every gate on both architectures at -smp 2), SMP-08 stress gate]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (38000, confidence: low).
actuals:
  tokens: 2385
  tasks: 2
  commits: 2
  plan_head_before: a651459c951dd4d98dd251431e8e63b574159acc
  plan_head_after: 95151abbf8a0aedaba7657e767fbca5f4f55a55b

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "R9_SMP env-default idiom (expect): set r9_smp [expr {[info exists env(R9_SMP)] ? $env(R9_SMP) : 2}] then -smp $r9_smp on the spawn line; the marker stage fires only when $r9_smp >= 2"
    - "R9_SMP env-default idiom (bash launcher): -smp \"${R9_SMP:-2}\" in the qemu argv + an env-contract comment row"
    - "Gate migration without behavior change: only the -smp token and (where the single-CPU leg is retained) the env-default block change; every other qemu flag, the env(R9_RUN_TAG) handling and the stage vocabulary are byte-identical"

key-files:
  created: []
  modified:
    - gates/aarch64/cpu-r9.expect
    - gates/aarch64/smoke-r9-net.expect
    - gates/aarch64/tier1-r9.expect
    - gates/x86_64/smoke-r9.expect
    - gates/x86_64/smoke-r9-net.expect
    - gates/x86_64/smoke-r9-auth.expect
    - nix/gates/boot-r9.sh
    - nix/gates/boot-r9-x64.sh

key-decisions:
  - "The three aarch64 sweep gates (cpu/net/tier1) and the x86_64 net/auth gates take a hardcoded -smp 2 with NO R9_SMP override — per the plan's acceptance criteria the retained single-CPU legs live only on each architecture's smoke-r9.expect and the two nix/gates launchers"
  - "The x86_64 smp-online stage asserts only 'smp: cpu 1 online' (the plan's named marker), placed between banner and paqfs exactly like 05-22's aarch64 placement — the kernel prints it at the end of exec_bootstrap (main.rs:868, after activate_first, before userland's paqfs line); the '(apic 0x1)' suffix on the real line matches by prefix (05-12's watch item)"
  - "The cpu-chain leg (second spawn, R9_CPU_ISO) gets -smp $r9_smp too but no extra marker stage — the plan names the one assertion on the boot log; expect's per-spawn buffer keeps the legs independent"

patterns-established:
  - "Parse-run harness for expect gates: a tclsh slave interpreter with every command stubbed (exit a no-op) executes each script end-to-end to its ALL STAGES PASS line, proving whole-file parse soundness without a qemu boot (useful on a host that cannot boot the target arch)"

requirements-completed: [SMP-01, SMP-04, SMP-09]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "gates/aarch64/{cpu-r9,smoke-r9-net,tier1-r9}.expect and nix/gates/boot-r9.sh default to -smp 2 (boot-r9.sh via ${R9_SMP:-2}, honoring R9_SMP=1); zero non-comment -smp 1 literals remain in gates/aarch64/ or boot-r9.sh (SMP-04, SMP-09, D-24)"
    requirement: SMP-04
    verification:
      - kind: automated
        ref: "grep -rn -- '-smp 1' gates/aarch64/ | grep -v ':[[:space:]]*#' == 0 matches; grep -v '^[[:space:]]*#' nix/gates/boot-r9.sh | grep -- '-smp 1' == 0 matches; stubbed tclsh parse-run of all three expect scripts reaches ALL STAGES PASS"
        status: pass
    human_judgment: false
  - id: D2
    description: "gates/x86_64/smoke-r9.expect defaults to -smp 2 via R9_SMP on both spawn legs (lines 142 and 395) and stages smp-online {smp: cpu 1 online} when $r9_smp >= 2; smoke-r9-net.expect and smoke-r9-auth.expect default to -smp 2; nix/gates/boot-r9-x64.sh uses ${R9_SMP:-2}; zero non-comment -smp 1 literals remain (SMP-01, SMP-09, D-24)"
    requirement: SMP-01
    verification:
      - kind: automated
        ref: "grep -n -- '-smp 1' gates/x86_64/smoke-r9.expect gates/x86_64/smoke-r9-net.expect gates/x86_64/smoke-r9-auth.expect | grep -v ':[[:space:]]*#' == 0 matches; grep -v '^[[:space:]]*#' nix/gates/boot-r9-x64.sh | grep -- '-smp 1' == 0 matches; stubbed tclsh parse-run of smoke-r9.expect reaches ALL STAGES PASS through BOTH spawn legs and the smp-online stage"
        status: pass
    human_judgment: false
  - id: D3
    description: "SMP-09 empty/adjacency truths: with R9_SMP unset every migrated script and launcher defaults strictly to 2 (expect: info-exists ternary; bash: :-2 default), and env(R9_RUN_TAG) handling plus all other qemu flags are unchanged"
    requirement: SMP-09
    verification:
      - kind: automated
        ref: "grep confirms the defaulting expressions (boot-r9.sh:205, boot-r9-x64.sh:359, smoke-r9.expect:129, aarch64 smoke-r9.expect:89); the per-file diffs touch only the -smp token, the env-default blocks and the doc comments"
        status: pass
    human_judgment: false

# Metrics
duration: 7min
completed: 2026-11-26
status: complete
---

# Phase 5 Plan 13: Gate Migration to -smp 2 (aarch64 gates, both nix/gates launchers, first 3 x86_64 smoke gates) Summary

**Every remaining aarch64 expect gate (cpu-r9, smoke-r9-net, tier1-r9), both `nix/gates/` launchers (`boot-r9.sh`, `boot-r9-x64.sh`), and the first three x86_64 smoke gates (`smoke-r9`, `smoke-r9-net`, `smoke-r9-auth`) now default to `-smp 2`, with `R9_SMP=1` honored on the retained single-CPU legs (`boot-r9.sh`, `boot-r9-x64.sh`, and x86_64 `smoke-r9.expect` — which gains the `smp-online` marker stage at `$r9_smp >= 2`); zero non-comment `-smp 1` literals remain anywhere in scope (SMP-01, SMP-04, SMP-09, D-24).**

## Performance

- **Duration:** 7 min
- **Started:** 2026-11-26T18:04:55Z
- **Completed:** 2026-11-26T18:12:21Z
- **Tasks:** 2 of 2 executed
- **Files modified:** 8 (exactly the plan's `files_modified` list)

## Accomplishments

- **aarch64 sweep complete (Task 1, SMP-04/SMP-09):** `cpu-r9.expect`, `smoke-r9-net.expect` and `tier1-r9.expect` moved from `-smp 1` to a hardcoded `-smp 2`; `nix/gates/boot-r9.sh` (the launcher `drive-r9.expect` spawns) now passes `-smp "${R9_SMP:-2}"` and documents `R9_SMP` in its env contract. Combined with 05-22's `smoke-r9.expect`, every `gates/aarch64/` script and the launcher default to two CPUs.
- **x86_64 smoke trio complete (Task 2, SMP-01/SMP-09):** `smoke-r9.expect` gained the `set r9_smp [expr {[info exists env(R9_SMP)] ? $env(R9_SMP) : 2}]` default block, `-smp $r9_smp` on BOTH spawn legs (the chain leg and the 19-09 cpu-chain leg), and the `if {$r9_smp >= 2} { stage smp-online {smp: cpu 1 online} }` stage placed between banner and paqfs — exactly where the kernel emits the marker at the end of `exec_bootstrap` (x86_64/main.rs:868, the 05-12 claim-then-start ordering). `smoke-r9-net.expect` and `smoke-r9-auth.expect` moved to a hardcoded `-smp 2`; `nix/gates/boot-r9-x64.sh` passes `-smp "${R9_SMP:-2}"` with the env contract documented.
- **SMP-09 adjacency/empty truths hold by construction:** with `R9_SMP` unset every migrated script and launcher defaults strictly to 2; the diffs touch only the `-smp` token, the env-default blocks and the doc comments — every other qemu flag and the `env(R9_RUN_TAG)` handling are byte-identical.
- **CORR-01 hygiene audited:** every proc in all six expect scripts was checked — none accesses `env(...)` from a proc body (all env reads are top-level), so no `global env` additions were needed.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate remaining gates/aarch64/*.expect scripts and nix/gates/boot-r9.sh to -smp 2 default (SMP-04, SMP-09)** — `5724ab5` (feat)
2. **Task 2: Migrate x86_64 smoke, net-smoke, auth expect scripts and nix/gates/boot-r9-x64.sh to -smp 2 default (SMP-01, SMP-09)** — `95151ab` (feat)

**Plan metadata:** this SUMMARY's own commit (docs) follows.

## Files Created/Modified

- `gates/aarch64/cpu-r9.expect` — spawn line 60: `-smp 1` → `-smp 2`
- `gates/aarch64/smoke-r9-net.expect` — spawn line 178: `-smp 1` → `-smp 2`
- `gates/aarch64/tier1-r9.expect` — spawn line 89: `-smp 1` → `-smp 2`
- `nix/gates/boot-r9.sh` — line 205: `-smp "${R9_SMP:-2}"`; header shape comment and the env contract (`R9_SMP` row) updated
- `gates/x86_64/smoke-r9.expect` — `R9_SMP` env docs; `r9_smp` default block (line 129); both spawn legs (lines 142, 395) on `-smp $r9_smp`; the `smp-online` stage at `$r9_smp >= 2` between banner and paqfs
- `gates/x86_64/smoke-r9-net.expect` — spawn line 203: `-smp 1` → `-smp 2`
- `gates/x86_64/smoke-r9-auth.expect` — spawn line 151: `-smp 1` → `-smp 2`
- `nix/gates/boot-r9-x64.sh` — line 359: `-smp "${R9_SMP:-2}"`; header shape comment and the env contract (`R9_SMP` row) updated

## Decisions Made

- **Hardcoded `-smp 2` (no env override) on the sweep gates** (`cpu-r9`, `smoke-r9-net`, `tier1-r9`, and the x86_64 `smoke-r9-net`/`smoke-r9-auth`): the plan's acceptance criteria place the retained single-CPU legs only on each architecture's `smoke-r9.expect` plus the two `nix/gates/` launchers — followed exactly.
- **The x86_64 `smp-online` stage asserts only `smp: cpu 1 online`** (the plan's named marker; the shootdown marker is not staged here): placed between `banner` and `paqfs`, mirroring 05-22's aarch64 placement — the kernel prints the marker at the end of `exec_bootstrap` before userland's `paqfs...` line. The real print carries an `(apic 0x1)` suffix (05-12's watch item); the stage matches by prefix.
- **The cpu-chain leg gets `-smp $r9_smp` but no marker stage:** the plan names the one assertion on the boot log (the chain leg's capture); expect's per-spawn buffer keeps the two legs independent.

## Deviations from Plan

None - plan executed exactly as written. (The `global env` audit the plan asked for was performed and found all six scripts already clean — no edits needed; the header/env-contract doc updates in both launchers and x86_64 `smoke-r9.expect` record the new `R9_SMP` knob, matching 05-22's documentation shape.)

## Issues Encountered

None. Verification was by the plan's grep contracts (all three return 0 matches) plus a stubbed-`tclsh` parse-run harness that executed each modified expect script end-to-end to its `ALL STAGES PASS` line (every command no-op'd, `exit` neutralized) — proving whole-file parse soundness on this aarch64 dev host, where the x86_64 gates cannot boot (the 05-11/05-12-recorded constraint; the live x86_64 `-smp 2` boot proof is Plan 05-14's scope on CI TCG + the owner's Linux KVM host per D-33).

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None.

## Threat Flags

None new. The plan's threat register is implemented as written: T-05-24 (a silent 1-CPU fallback masking multi-core races) is mitigated by the strict `-smp 2` / `${R9_SMP:-2}` defaults plus the `smp-online` assertion when `R9_SMP >= 2`; T-05-SC held (zero external packages installed, `Cargo.lock` untouched). No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes.

## Next Phase Readiness

- **Plan 05-14 (the remaining 7 x86_64 gates + the live boot proof):** `cpu-bringup`, `drawterm-gate`, `stress-r9`, `netsoak-r9`, `churn-r9`, `tier1-r9` and `services-r9` still carry `-smp 1` — 05-14's named scope, together with the live x86_64 `-smp 2` boot verdict (the `smp: cpu 1 online` marker this plan's smoke gate now asserts) on CI TCG + owner KVM (D-33).
- **Owner Stop 2** (D-32): after 05-14 lands, every gate on both architectures runs at `-smp 2` and the golden invariant moves with it.
- **Watch item:** the x86_64 marker prints as `smp: cpu 1 online (apic 0x1)`; the new stage matches the prefix, so the suffix is tolerated (05-12's recorded guidance).

## Self-Check: PASSED

- Files verified: all 8 modified files exist at their paths with the expected content (`grep` confirms the `-smp 2` / `${R9_SMP:-2}` / `$r9_smp` landings and the `smp-online` stage).
- Commits verified: `5724ab5` (Task 1) and `95151ab` (Task 2) are both ancestors of HEAD (`git merge-base --is-ancestor`).
- Measured commits from the ledger (`gsd-plan-head-before-05-13` = `a651459c`): 2 at completion (the docs commit follows); `plan_head_after` = `95151ab`.
- Plan-level verification re-run on the final tree: all three grep contracts return 0 non-comment matches; the must_haves `contains` checks (`R9_SMP:-2` in both launchers, `R9_SMP` in x86_64 `smoke-r9.expect`) all pass.
- Requirement IDs SMP-01/SMP-04/SMP-09 are shared with sibling plans still lacking SUMMARYs (05-14, 05-21, 05-24..27): `requirements.ready-ids` reports all three `blocked`, so no `mark-complete` was run — they mark when the last declaring plan finishes (#2388).
- Working tree clean at completion (pre-SUMMARY); `Cargo.lock` untouched (T-05-SC).

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*
