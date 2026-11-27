---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 21
subsystem: testing
tags: [flake-nix, github-actions, xtask, smp-stress, mem-pressure, aeadloop, palloc-evictor, aarch64, halted]

# Dependency graph
requires:
  - phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
    provides: "05-27's four -smp 2 wrappers + expect drivers; 05-20's aeadloop=1 loopback server; 05-16/05-17 palloc evictor + page cache; 05-22's -smp 2 kstack-ownership fix"
provides:
  - "flake.nix apps boot-r9-smp-stress, boot-r9-mem-pressure, boot-r9-smp-stress-x64, boot-r9-mem-pressure-x64 wrapping nix/gates/*.sh"
  - "gates.yml jobs running the four stress/pressure gates on both architectures at -smp 2 (explicit TCG, no error masking)"
  - "xtask full-tier registration for the four apps (+ output-dir maps and Phase 1 witness map)"
  - "aead_loop now serves a real root directory listing and Tstat (ls '#M' works)"
affects: [Owner Stop 4, SMP-08, SMP-09, MEM-15, MEM-17, MEM-10]

# Actuals (#2632)
actuals:
  tokens: 5643
  tasks: 1
  commits: 1
  plan_head_before: 0c14fb11461a4ddc6b81dbba842440cd869108f3
  plan_head_after: 0da8b075ead1cf524794d61adbaf8b17852b1425

tech-stack:
  added: []
  patterns:
    - "Nix app wiring mirrors mkScriptApp/bootR9RuntimeInputs (aarch64) and bootR9X64RuntimeInputs (x86_64-linux only)"
    - "CI stress/pressure jobs set R9_ACCEL=tcg explicitly (no silent downgrade)"
    - "Synthetic 9P server serves stat(5) records via crate::dat::StatWriter, shared by Tstat and the directory listing"

key-files:
  created: []
  modified:
    - flake.nix
    - .github/workflows/gates.yml
    - xtask/src/gate.rs
    - .planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
    - nix/gates/smoke-r9.sh
    - port/src/aead_loop.rs
    - gates/aarch64/mem-pressure.expect
    - gates/x86_64/mem-pressure.expect

key-decisions:
  - "Tstat + directory listing belong in the synthetic loopback server: `ls '#M'` is a real directory read plus Tstat, not just attach/walk/open/read/clunk"
  - "The 15-minute wall-clock of a failing gate is dominated by the driver's stage timeout; the smp-stress abort is deterministic (HVF x2, TCG x1), not a venue flake"
  - "mem-pressure boot marker order is banner → palloc → smp-online: palloc's managed_frames line prints at memory init, long before CPU 1 online"

patterns-established:
  - "9P server stat(5) records built with crate::dat::StatWriter (41-byte fixed part + four length-prefixed strings)"

requirements-completed: []

coverage:
  - id: D1
    description: "flake.nix/CI/xtask expose and dispatch the four -smp 2 stress/pressure apps"
    requirement: SMP-08
    verification:
      - kind: other
        ref: "nix eval .#apps.*.{boot-r9-smp-stress,boot-r9-mem-pressure,...}.program; python3 yaml.safe_load gates.yml (13 jobs); cargo test -p xtask full_"
        status: pass
    human_judgment: false
  - id: D2
    description: "aarch64 smp-stress runs green twice consecutively under HVF and TCG"
    requirement: SMP-09
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-smp-stress (HVF x2, TCG x1)"
        status: fail
    human_judgment: true
    rationale: "The gate halts deterministically in the kernel; a dedicated concurrency debug session is required before any green claim."
  - id: D3
    description: "aarch64 mem-pressure boots the 8 MiB pool, pins a buffer, re-faults evicted pages, and reports positive evictor counters"
    requirement: MEM-15
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-mem-pressure (HVF)"
        status: fail
    human_judgment: true
    rationale: "Boot/pin/re-fault pass but dropped_text/dropped_data/pinned_skipped are all 0 — the gate workload does not force eviction at 2048 frames."

duration: ~55 min (including ~45 min of gate runs)
completed: 2026-11-27
status: halted
---

# Phase 05 Plan 21: Two-CPU Stress & Pressure Gate Wiring Summary

**Wired Plan 05-27's four `-smp 2` stress/pressure gates into `flake.nix`, CI and `xtask`, and repaired two gate-server/driver defects — but HALTED before Owner Stop 4 because the smp-stress gate halts deterministically inside the kernel and mem-pressure reports zero evictor activity.**

## Performance

- **Duration:** ~55 min (including ~45 min of QEMU gate runs)
- **Completed:** 2026-11-27 (halted)
- **Tasks:** 1 of 2 (Task 1 implementation committed; verification RED)
- **Files modified:** 8

## Accomplishments

- `flake.nix` exports `boot-r9-smp-stress`, `boot-r9-mem-pressure` (all systems), and `boot-r9-smp-stress-x64`, `boot-r9-mem-pressure-x64` (x86_64-linux), each wrapping Plan 05-27's `nix/gates/*.sh` with the established `mkScriptApp` supply contract.
- `.github/workflows/gates.yml` gains the four stress/pressure jobs on both architectures at `-smp 2`, explicit `R9_ACCEL=tcg`, and no error masking (T-05-37).
- `xtask/src/gate.rs` registers the four apps in the `cargo xtask gate --full` tier (with output-dir maps); the Phase 1 witness map is mirrored so `cargo test -p xtask` stays green.
- `port/src/aead_loop.rs` now serves a real root directory listing and `Tstat` (built with the shared `StatWriter` shape); `ls '#M'` enumerates `data0..data7` and `stat` succeeds. Added `aead_loop_root_enumerates_and_stats`.
- `nix/gates/smoke-r9.sh` asserts `aeadloop: active` is ABSENT on a default boot (SMP-13).
- mem-pressure drivers: corrected the boot marker order (palloc before smp-online) and switched the aarch64 pinned reader to the image's `hsleep`.

## Task Commits

1. **Task 1: Wire gates into flake/CI/xtask + gate-server/driver fixes** — `0da8b07` (feat)

**Plan metadata:** committed with this SUMMARY (docs).

## Files Created/Modified

- `flake.nix` — four new Nix apps.
- `.github/workflows/gates.yml` — four new CI jobs (both arches, explicit TCG).
- `xtask/src/gate.rs` — full-tier lists/maps/tests for the four apps.
- `.planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` — mirrored the two new x86_64 rows.
- `nix/gates/smoke-r9.sh` — SMP-13 default-dormant negative assertion.
- `port/src/aead_loop.rs` — directory listing + Tstat + unit test.
- `gates/aarch64/mem-pressure.expect` — stage order + `hsleep 60`.
- `gates/x86_64/mem-pressure.expect` — stage order.

## Decisions Made

- The synthetic loopback server must answer `Tstat` and a directory `Tread`; `ls '#M'` is not just attach/walk/open/read/clunk.
- Register the gates in `gate.rs` (where the full-tier list actually lives) rather than `main.rs` (the plan's named file); this is the real `cargo xtask gate` seam.
- Keep the task commit even though the live gate is RED: the wiring and the two server/driver fixes are correct, unit-tested, and required for the next attempt.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `aead_loop` refused `ls '#M'` (missing Tstat + directory listing)**
- **Found during:** Task 1 first `nix run .#boot-r9-smp-stress`
- **Issue:** `ls '#M'` printed `devmnt: server error: aeadloop: request refused`; `solve()` handled attach/walk/open/read/clunk only, and the root `Tread` returned an empty vector. `ls` needs `Tstat` and a stat-record directory listing.
- **Fix:** Added `stat_record()` (via `crate::dat::StatWriter`) and `directory_bytes()`, a `Msg::Tstat` arm, and a directory-aware `Tread`; added a unit test.
- **Files modified:** `port/src/aead_loop.rs`
- **Verification:** `cargo test -p port aead_loop` 5/5 incl. the new test; the gate now reaches the stress stage.
- **Committed in:** `0da8b07`

**2. [Rule 1 - Bug] mem-pressure driver stage order was impossible**
- **Found during:** Task 1 `nix run .#boot-r9-mem-pressure`
- **Issue:** The driver waited for `palloc: managed_frames=...` AFTER `smp: cpu 1 online`, but palloc prints at memory init (before CPU 1 online), so the stage could never match and the gate timed out at `palloc`.
- **Fix:** Reordered to banner → palloc → smp-online → paqfs (both arches).
- **Files modified:** `gates/aarch64/mem-pressure.expect`, `gates/x86_64/mem-pressure.expect`
- **Verification:** the gate now boots through to the pressure worker and re-fault stages.
- **Committed in:** `0da8b07`

**3. [Rule 1 - Bug] aarch64 mem-pressure used a non-existent `sleep`**
- **Found during:** Task 1 (gate wedged at `pinned-bytes`)
- **Issue:** The arm64 image stages `sleep(1)` as `/arm64/bin/hsleep`; the driver's bare `sleep 60` never resolved, so the pipe closed and `pinnedok` never arrived.
- **Fix:** `hsleep 60` in the aarch64 driver (the x86_64 chain image has no sleep at all — recorded as an open item).
- **Files modified:** `gates/aarch64/mem-pressure.expect`
- **Verification:** the pinned reader now delivers `pinnedok`.
- **Committed in:** `0da8b07`

**4. [Rule 3 - Blocking] xtask registration lives in `gate.rs`, not `main.rs`**
- **Found during:** Task 1
- **Issue:** The full-tier app list is `FULL_APPS_{X86_64,AARCH64}` in `xtask/src/gate.rs`; `main.rs` only dispatches.
- **Fix:** Registered the four apps in `gate.rs`, updated the three gate tests (lengths/lists), and mirrored the two x86_64 rows in the Phase 1 witness map (the test pins byte-equality).
- **Files modified:** `xtask/src/gate.rs`, `.planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh`
- **Verification:** `cargo test -p xtask full_` 3/3; `cargo xtask check` clean.
- **Committed in:** `0da8b07`

**5. [Rule 2 - Missing Critical] SMP-13 default-dormant assertion**
- **Found during:** Task 1
- **Issue:** No leg asserted the loopback server stays dormant without `aeadloop=1`.
- **Fix:** `nix/gates/smoke-r9.sh` greps the capture for the active marker and fails on a default boot (guarded for an explicit `R9_BOOTARGS` opt-in).
- **Files modified:** `nix/gates/smoke-r9.sh`
- **Verification:** `bash -n` clean; the default smoke run carries no `aeadloop=1` token.
- **Committed in:** `0da8b07`

---

**Total deviations:** 5 auto-fixed (4 Rule 1, 1 Rule 3, 1 Rule 2 — counted as 5 distinct fixes)
**Impact on plan:** All five are required for the gates to run at all. They do not resolve the kernel/gate-workload blockers below.

## Issues Encountered

### BLOCKER 1 — smp-stress halts deterministically inside the kernel

`nix run .#boot-r9-smp-stress` boots, enumerates `#M`, completes the 8×4096 read pass, then halts when the two concurrent `#M` reader workers and the exec-churn worker run:

```
TrapFrame { ... pc: 0x000000009b7728f0, x30: 0x000000009b7728f0, sp: 0xffff9000001c2e20, psr: 0x600003c5 }
esr_el1: ec=InstructionAbortSameEl (33), iss=0x5
far_el1: 0x000000009b7728f0
Unhandled interrupt
kernel halted
smp-stress-r9: TIMEOUT at stage: stress-response
```

Reproduced **deterministically** three times with the identical `far_el1`/`pc`: HVF gicv3 ×2 and TCG gicv2 ×1. The kernel is linked at `KZERO = 0xffff800000000000`; `0x9b7728f0` is neither a kernel symbol nor a user text address (harvey binaries load at ~0x10000), so the kernel returned/branched to a corrupted address at EL1h. This is a new deterministic concurrency/stack-ownership defect (adjacent to, but distinct from, the 05-22 HVF flake, which was fixed and green 8/8). It needs its own debug session — the same call the owner made for 05-22 — before any green claim.

### BLOCKER 2 — mem-pressure reports zero evictor activity at the 8 MiB cap

With the driver order/sleep fixed, the gate boots the 8 MiB pool, pins the pipe reader, runs the 12-iteration exec/read churn, re-faults `/lib/namespace` byte-identically, and reads `#c/sysstat`:

```
evictor: dropped_text=0 dropped_data=0 pinned_skipped=0
```

The 2048-frame pool is not driven below its low-water mark by the current churn (boot uses ~6 frames; repeated exec of the same three binaries reuses shared text). Lowering `R9_PALLOC_MAX_FRAMES` to 512 to force pressure instead hit `DataAbortSameEl far=0x20118` during boot — a second kernel defect. The gate workload must be strengthened (or the pool cap/eviction pinned) as part of the same debug effort.

### Deployment gaps

- The x86_64 gates cannot run on this aarch64 macOS host (Linux-only `grub-mkrescue`/authdomain venue); they remain for CI TCG and the owner's Linux KVM host (D-33).
- The x86_64 mem-pressure driver still uses `sleep 60`, which the minimal chain image does not stage; it needs an `hsleep` staging entry (or the same non-sleep technique) before it can pass.

### Verification actually run

- `nix run .#boot-r9-smp-stress` — **FAIL** (Blockers 1; HVF ×2, TCG ×1)
- `nix run .#boot-r9-mem-pressure` — **FAIL** (Blocker 2; boot/pin/re-fault pass, counters 0)
- `R9_PALLOC_MAX_FRAMES=512 nix run .#boot-r9-mem-pressure` — **FAIL** (DataAbortSameEl `far=0x20118`)
- `cargo xtask check` — PASS
- `cargo xtask test` — PASS (exit 0, all suites)
- `cargo test -p port aead_loop` — PASS (5/5 incl. new test)
- `cargo test -p xtask full_` — PASS (3/3)
- `flake.nix` eval of all four apps — PASS
- `gates.yml` YAML parse — PASS (13 jobs)
- **Not run:** the two clippy legs and `cargo xtask miri` (the plan is halted; they are recorded in `WINDOWS.md` as `unrun-verify`).

## Known Stubs

- `gates/x86_64/mem-pressure.expect` — pinned reader invokes `sleep`, absent from the amd64 chain image (`xtask/manifests/amd64/chain.toml`). Registered in `.planning/WINDOWS.md`; fix belongs with the x86_64 gate validation.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**NOT ready for Owner Stop 4.** Phase 5's SMP-08/SMP-09/MEM-15 proof needs a dedicated kernel debug session covering (a) the deterministic `InstructionAbortSameEl` under 2-CPU `#M` readers + exec churn, and (b) forcing the live evictor under the small-memory pool (and the `DataAbortSameEl far=0x20118` seen at a 512-frame cap). The wiring is committed and ready to re-run the moment those are fixed. Owner Stop 4 (Task 2) was not reached.

## Self-Check: PASSED

- All 8 modified files exist on disk.
- Task commit `0da8b07` is an ancestor of HEAD.
- Plan status is `halted` by design: the live gates are RED on the kernel/gate-workload blockers above, so Owner Stop 4 was not reached.
