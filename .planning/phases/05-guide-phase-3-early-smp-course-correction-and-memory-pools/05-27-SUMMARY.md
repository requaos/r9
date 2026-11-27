---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 27
subsystem: testing
tags: [expect, qemu, smp, aeadloop, 9P2000.aead, palloc, evictor, fdtput, r9-iso-builder, aarch64, x86_64]

# Dependency graph
requires:
  - phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
    provides: "aeadloop=1 boot switch + in-kernel 9P2000.aead loopback (#M) (05-20); palloc evictor + #c/sysstat counters (05-14/05-16); SMP bring-up markers + narrow smoke wrappers (05-12/05-22)"
provides:
  - "aarch64 -smp 2 smp-stress expect driver + wrapper (fdtput R9_BOOTARGS aeadloop=1)"
  - "x86_64 -smp 2 smp-stress expect driver + wrapper (r9-iso-builder --append=aeadloop=1)"
  - "aarch64 -smp 2 small-memory mem-pressure expect driver + wrapper (patched /memory + palloc_max_frames=2048)"
  - "x86_64 -smp 2 small-memory mem-pressure expect driver + wrapper (palloc_max_frames=2048 bootconf append)"
affects: [05-21, flake.nix, xtask, .github/workflows/gates.yml]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
actuals:
  tokens: 16942
  tasks: 2
  commits: 2
  plan_head_before: 3eff0a1d6dbb5cfc95101e35e4b5040ee38dbead
  plan_head_after: c106a815e354bf4ae3cac62832f6a6da8e283f94

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-arch bootargs delivery: aarch64 via fdtput -t s /chosen bootargs; x86_64 via r9-iso-builder --append (never QEMU -append with -cdrom)"
    - "Small-memory pressure gate: DTB /memory shrink + palloc_max_frames cap + pinned-blocked pipe reader vs exec-churn worker"
    - "Serial-clean stress reads: wc -c on aarch64, /tmp redirect + ls -l on x86_64 (no binary payload on the cooked wire)"

key-files:
  created:
    - nix/gates/smp-stress-r9.sh
    - nix/gates/mem-pressure-r9.sh
    - gates/aarch64/smp-stress.expect
    - gates/aarch64/mem-pressure.expect
    - nix/gates/smp-stress-r9-x64.sh
    - nix/gates/mem-pressure-r9-x64.sh
    - gates/x86_64/smp-stress.expect
    - gates/x86_64/mem-pressure.expect
  modified: []

key-decisions:
  - "Stress load launched as several short background rc lines (not one long line) to respect the measured 16550 RX-ring burst limit"
  - "aarch64 reads verified with `cat '#M/dataN' | wc -c` (serial-clean text); x86_64 redirects binary reads to /tmp and checks size with ls -l (chain image has no wc)"
  - "Mem-pressure pinned reader is `{ sleep 60; echo pinnedok } | cat &` — holds a pipe-read PinGuard across the sleep, then proves the pinned buffer receives intact bytes"

patterns-established:
  - "Bootargs-gated stress gate: assert aeadloop: active before driving #M, so an unactivated loopback fails at the marker not deep in the load"
  - "Whole-capture deadlock post-pass (WATCHDOG:/panic/lock rank/unresolvable fault) alongside expect_before fast-fail"

requirements-completed: [SMP-08, SMP-09, MEM-10, MEM-15, MEM-16, MEM-17]

coverage:
  - id: D1
    description: "aarch64 nix gate wrapper patches /chosen/bootargs with aeadloop=1 via fdtput -t s and launches the -smp 2 stress driver"
    requirement: "SMP-08"
    verification:
      - kind: automated
        ref: "grep 'fdtput' nix/gates/smp-stress-r9.sh (6 non-comment hits; bash -n clean)"
        status: pass
    human_judgment: false
  - id: D2
    description: "aarch64/x86_64 smp-stress expect drivers spawn QEMU with -smp 2 and drive #M read/clunk + exec churn to SMP_STRESS_OK"
    requirement: "SMP-09"
    verification:
      - kind: automated
        ref: "grep -rn -- '-smp 2' gates/aarch64/*.expect gates/x86_64/*.expect (one spawn per file)"
        status: pass
    human_judgment: false
  - id: D3
    description: "x86_64 nix gate wrapper composes the chain ISO via r9-iso-builder --append=aeadloop=1 (no QEMU -append)"
    requirement: "SMP-08"
    verification:
      - kind: automated
        ref: "grep -- '--append=' nix/gates/smp-stress-r9-x64.sh (invocation line present)"
        status: pass
    human_judgment: false
  - id: D4
    description: "mem-pressure wrappers/drivers boot a bounded pool (palloc_max_frames=2048) and assert dropped_text/dropped_data/pinned_skipped > 0 via cat '#c/sysstat'"
    requirement: "MEM-15"
    verification:
      - kind: automated
        ref: "structural: bash -n + grep dropped_text/-smp 2/palloc_max_frames across the four mem files"
        status: pass
    human_judgment: true
    rationale: "The evictor counters are only realised at runtime under QEMU; 05-21 wires and executes the gates, so the live positive-counter assertion is validated then, not at authoring time."
  - id: D5
    description: "Both smp-stress drivers fail fast and post-pass on WATCHDOG:/panic/lock rank/unresolvable fault, proving no deadlock under 2-CPU load"
    requirement: "SMP-09"
    verification:
      - kind: automated
        ref: "grep 'WATCHDOG:|panic|lock rank|unresolvable fault' in both *smp-stress.expect"
        status: pass
    human_judgment: false

# Metrics
duration: 1min
completed: 2026-11-27
status: complete
---

# Phase 05 Plan 27: Two-CPU Stress & Small-Memory Pressure Gates Summary

**Authored four `-smp 2` expect stress/pressure drivers and four Nix wrappers for aarch64 and x86_64, delivering `aeadloop=1` through each architecture's real bootargs mechanism and asserting the SMP, 9P2000.aead, SIMD and live-evictor markers.**

## Performance

- **Duration:** ~1 min
- **Started:** 2026-11-27T00:30:26Z
- **Completed:** 2026-11-27T00:31:20Z
- **Tasks:** 2
- **Files modified:** 8 created, 0 modified

## Accomplishments
- aarch64 `nix/gates/smp-stress-r9.sh` patches `R9_BOOTARGS="aeadloop=1"` onto `/chosen/bootargs` with `fdtput -t s` (the smoke-r9.sh:215-220 recipe); `gates/aarch64/smp-stress.expect` boots `-smp 2` with `-dtb $env(R9_DTB)` and drives `'#M/data0'..'#M/data7'` reads/clunks plus exec churn.
- x86_64 `nix/gates/smp-stress-r9-x64.sh` composes the chain ISO via `r9-iso-builder --append="aeadloop=1"`; `gates/x86_64/smp-stress.expect` boots `-smp 2` on `-cdrom $env(R9_ISO)` (no QEMU `-append`).
- Both mem-pressure wrappers boot a bounded 8 MiB pool (`palloc_max_frames=2048`; aarch64 also shrinks `/memory@40000000` to 256 MiB) and their drivers run a pinned-blocked pipe reader against an exec-churn worker, re-fault evicted pages, then parse `cat '#c/sysstat'` for positive `dropped_text`/`dropped_data`/`pinned_skipped`.
- Every driver asserts `smp: cpu 1 online`, `smp: tlb shootdown ok`, `aeadloop: active device=#M dialect=9P2000.aead`, and `setops: simd=… selftest=ok`, and fails fast on `WATCHDOG:`/`panic`/`lock rank`/`unresolvable fault` (stream + whole-capture post-pass).

## Task Commits

Each task was committed atomically:

1. **Task 1: aarch64 smp-stress + mem-pressure gates** - `a3c5099` (feat)
2. **Task 2: x86_64 smp-stress + mem-pressure gates** - `c106a81` (feat)

**Plan metadata:** committed with SUMMARY.md (docs).

## Files Created/Modified
- `nix/gates/smp-stress-r9.sh` — aarch64 wrapper: bootfs image, DTB initrd cells + `fdtput -t s /chosen bootargs aeadloop=1`, runs the stress driver.
- `gates/aarch64/smp-stress.expect` — `-smp 2` boot; #M enumeration, 8×4096 read pass, 2 read/clunk workers + exec-churn worker, `SMP_STRESS_OK`.
- `nix/gates/mem-pressure-r9.sh` — aarch64 wrapper: `/memory@40000000` → 256 MiB, `palloc_max_frames=2048`, `R9_MEM`/`-m` plumbing.
- `gates/aarch64/mem-pressure.expect` — `-smp 2`, `-m $r9_mem`; pinned reader + pressure worker, re-fault, `#c/sysstat` counters, `MEM_PRESSURE_OK`.
- `nix/gates/smp-stress-r9-x64.sh` — x86_64 wrapper: chain image, `r9-iso-builder --append="aeadloop=1"`.
- `gates/x86_64/smp-stress.expect` — `-smp 2` on `-cdrom`; #M enumeration, /tmp-redirected read pass, concurrent stress, `SMP_STRESS_OK`.
- `nix/gates/mem-pressure-r9-x64.sh` — x86_64 wrapper: `--append="palloc_max_frames=2048"`, `R9_MEM` default 256M.
- `gates/x86_64/mem-pressure.expect` — `-smp 2`, `-m $r9_mem`; pinned reader + pressure worker, re-fault, `#c/sysstat` counters, `MEM_PRESSURE_OK`.

## Decisions Made
- Stress loads are launched as several short background rc lines rather than one long line, honoring the measured ~120-char 16550 RX-ring burst-truncation limit documented in `gates/x86_64/stress-r9.expect`.
- aarch64 read verification uses `cat '#M/dataN' | wc -c` (serial-clean); x86_64's chain image lacks `wc`, so reads redirect to `/tmp` and the byte count is proved with `ls -l` (binary 9P payloads never reach the cooked serial wire).
- The mem-pressure pinned reader is `{ sleep 60; echo pinnedok } | cat &`, which both holds a pipe-read PinGuard across the sleep and asserts the pinned buffer later receives intact bytes.

## Deviations from Plan
None - plan executed exactly as written.

## Known Stubs
None — all eight files are complete gate drivers/wrappers. Runtime execution of the gates is intentionally deferred to Plan `05-21` (flake.nix / xtask / CI wiring), per the plan's success criteria ("ready for Plan 05-21 … wiring and execution").

## Issues Encountered
None. All `<automated>` verifications passed: `bash -n` clean on all four wrappers; `fdtput` count 6 (≥3); `--append=` present; exactly one non-comment `-smp 2` per expect driver (4 total); zero `vendor/harvey` modifications.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The four expect drivers and four wrappers are ready for Plan `05-21` to wire the `flake.nix` apps, `xtask` steps and CI jobs, and for Owner Stop 4.
- The mem-pressure gates carry a runtime `human_judgment` coverage entry: their positive `#c/sysstat` counter assertions are only realised under QEMU and will be validated when 05-21 executes them.

## Self-Check: PASSED

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-27*
