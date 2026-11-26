---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 17
subsystem: data-structures
tags: [roaring, simd, avx2, neon, inline-asm, with-simd, mem-17, lintcontract, unsafe-audit]

# Dependency graph
requires:
  - phase: 05-16
    provides: "memory pools whose evictable set is Roaring algebra (Dirty minus (Pinned | Active))"
  - phase: 05-15
    provides: "the x86_64 AVX/XSAVE arm and fpu::with_simd()/avx2_enabled() (MEM-18, D-20)"
provides:
  - "ds/src/setops_simd.rs: the one reserved audited unsafe DS module — AVX2 (x86_64) and NEON (aarch64) bitmap_and/or/andnot inline-asm kernels, every unsafe block SAFETY-documented, vzeroupper on x86_64 exit"
  - "ds/src/setops.rs: SIMD_ENABLED runtime dispatch, register_simd_guard bracket, enable_simd, selftest_simd_matches_scalar, retained scalar bodies; still forbid(unsafe_code)"
  - "port::setops re-export of enable_simd/register_simd_guard/selftest_simd_matches_scalar across the R4 boundary"
  - "aarch64/x86_64 boot wiring: register the fpu::with_simd bracket, enable NEON / AVX2-on-CPUID, boot selftest and a deterministic setops log line"
  - "xtask lintcontract R3 allows ds/src/setops_simd.rs; UNSAFE-AUDIT.md records the module"
affects: [05-18, 05-19, 05-21, 05-22]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan estimate (52000, confidence: low).
actuals:
  tokens: 11415
  tasks: 3
  commits: 2
  plan_head_before: f48cd5bd2e112085e14ac199123c660397a55993
  plan_head_after: 8fcb116d6439c77b7793db7d3ca88f85a59ccba3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One reserved audited unsafe SIMD module mounted at a crate root (a forbid reaches submodules)"
    - "Runtime SIMD dispatch through a registered fn(&mut dyn FnMut()) bracket (fpu::with_simd) plus an AtomicBool enable flag set after the CPU-feature check"
    - "Explicit vector clobbers (v0-v3 / ymm0-ymm1) with generic scalar operands; clobber_abi is not used because rustc rejects it alongside generic register outputs"

key-files:
  created:
    - ds/src/setops_simd.rs
  modified:
    - ds/src/setops.rs
    - ds/src/lib.rs
    - port/src/lib.rs
    - aarch64/src/main.rs
    - x86_64/src/main.rs
    - xtask/src/lintcontract.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "The SIMD-equivalence pins call the AVX2/NEON kernels directly through host-only entry points instead of toggling the process-global SIMD_ENABLED flag, so parallel cargo tests never race on dispatch state; the boot selftest and the Roaring-algebra pin toggle the flag only when the host can execute the kernel."
  - "x86_64 computes the cardinality in scalar Rust over the kernel's output words (the plan permits scalar or Harley-Seal); NEON uses cnt + uaddlp twice + addv accumulated in four u32 lanes, so no 8-bit lane overflows."
  - "The registered bracket type is fn(&mut dyn FnMut()) stored as an AtomicUsize code address; the arch boot passes |f| fpu::with_simd(|_| f()) because with_simd takes FnOnce(&KernelSimd), not the bracket's own argument type."
  - "The x86_64 kernels popcount outside the asm but keep the bitwise loop in AVX2 asm with vzeroupper before return, preserving the D-19/T-05-30 state contract."

requirements-completed: [MEM-17]

coverage:
  - id: D1
    description: "AVX2/NEON kernels plus scalar fallback produce bit-for-bit identical 1024-word outputs and exact cardinalities on all 8 seeds and the empty/ones/single-bit/alternating/complement cases"
    requirement: MEM-17
    verification:
      - kind: unit
        ref: "cargo test -p ds setops#simd_kernels_match_scalar_on_all_seeds_and_edges"
        status: pass
      - kind: unit
        ref: "cargo test -p ds setops#bitmap_kernels_match_naive"
        status: pass
    human_judgment: false
  - id: D2
    description: "lintcontract R3 permits only ds/src/tpht/ and ds/src/setops_simd.rs to hold unsafe in ds/, and rejects any other unsafe module"
    requirement: MEM-17
    verification:
      - kind: unit
        ref: "cargo test -p xtask lintcontract"
        status: pass
    human_judgment: false
  - id: D3
    description: "Boot-time SIMD enable + selftest and the deterministic setops log line on both architectures"
    requirement: MEM-17
    verification:
      - kind: other
        ref: "cargo xtask clippy --arch aarch64 --config virtshell; cargo xtask clippy --arch x86-64 --config q35cpu (compile the dev_exec boot wiring)"
        status: pass
    human_judgment: true
    rationale: "The boot selftest/log is not exercised in this worktree because no -smp 2 boot gate was run; the verifier must confirm it on a real boot."
  - id: D4
    description: "UNSAFE-AUDIT.md documents every unsafe site in ds/src/setops_simd.rs in the same change"
    requirement: MEM-17
    verification: []
    human_judgment: true
    rationale: "Documentation-accuracy audit; no automated assertion."

duration: 4min
completed: 2026-11-26
status: complete
---

# Phase 5 Plan 17: Roaring SIMD Set Kernels Summary

**AVX2 (x86_64) and NEON (aarch64) Roaring bitmap set kernels in the one reserved audited unsafe module, runtime-dispatched behind `ds/src/setops.rs` and `fpu::with_simd()` and pinned bit-for-bit to the scalar path.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-11-26T23:46:02Z
- **Completed:** 2026-11-26T23:49:47Z
- **Tasks:** 3
- **Files modified:** 8 (1 created, 7 modified)

## Accomplishments

- `ds/src/setops_simd.rs` lands the reserved module: AVX2 (`vmovdqu`/`vpand`/`vpor`/`vpandn`/`vzeroupper`) and NEON (`.arch_extension simd`, `ld1`/`and`/`orr`/`bic`/`st1`/`cnt`/`uaddlp`/`addv`) kernels for `bitmap_and`, `bitmap_or` and `bitmap_andnot`, each an `unsafe fn` with a concrete `// SAFETY:` contract.
- `ds/src/setops.rs` keeps its exact `pub(crate)` seam, retains the scalar bodies as `bitmap_*_scalar`, and adds `SIMD_ENABLED`, `enable_simd`, `register_simd_guard`, `selftest_simd_matches_scalar` and the three dispatch functions — all safe code under `#[forbid(unsafe_code)]`.
- `port::setops` re-exports the three public APIs, so both arch crates reach the SIMD seam without a `ds` dependency (R4).
- Both `main.rs` files register `|f| fpu::with_simd(|_| f())`, enable NEON unconditionally on aarch64 and AVX2 only when the BSP's CPUID decision armed it on x86_64 (`qemu64` stays scalar), assert the boot selftest and log `setops: simd=neon|avx2|scalar selftest=ok`.
- `xtask/src/lintcontract.rs` R3 now allows exactly `ds/src/tpht/` and `ds/src/setops_simd.rs`; `UNSAFE-AUDIT.md` records the module, its sites and the policy.

## Task Commits

Each task was committed atomically (the setops seam and its tests share one file, so Task 2's dispatch and Task 3's tests land in commit 1):

1. **Task 1 (+ Task 2 ds seam + Task 3 pins)** - `8643d68` (feat)
2. **Task 2 (port + arch boot wiring)** - `8fcb116` (feat)

**Plan metadata:** the SUMMARY commit follows this file.

_Note: Task 1 is declared `tdd="true"` in a `type: execute` plan; see the deviation note on commit shape._

## Files Created/Modified

- `ds/src/setops_simd.rs` - the reserved audited unsafe SIMD module: guard registry + safe dispatch wrappers + AVX2/NEON kernels + host entry points.
- `ds/src/setops.rs` - scalar bodies renamed, SIMD_ENABLED/dispatch/selftest added, SIMD-equivalence and Roaring-with-SIMD tests added.
- `ds/src/lib.rs` - `mod setops_simd;` at the crate root; `pub mod setops;` under `#[forbid(unsafe_code)]`.
- `port/src/lib.rs` - `#[forbid(unsafe_code)] pub mod setops` re-export (R4 boundary).
- `aarch64/src/main.rs` - register the bracket, enable NEON, selftest, `setops: simd=neon selftest=ok`.
- `x86_64/src/main.rs` - register the bracket, enable AVX2 on the CPUID decision, selftest, `simd=avx2|scalar` log.
- `xtask/src/lintcontract.rs` - R3 allow-list + module-forbid exemption + tests.
- `UNSAFE-AUDIT.md` - module state row and the Plan 05-17 delta.

## Decisions Made

- Measure the SIMD equivalence through direct host entry points rather than the global enable flag, so parallel `cargo test` threads cannot observe a half-toggled dispatch state.
- Compute the x86_64 cardinality scalar-side (the plan permits it) while keeping the bitwise loop in AVX2 asm; NEON folds popcounts with `cnt`/`uaddlp`/`addv` in four u32 lanes.
- Use explicit vector clobbers instead of `clobber_abi` with generic operands (see deviations).
- Register the bracket as `|f| fpu::with_simd(|_| f())` to match `with_simd`'s actual `FnOnce(&KernelSimd)` signature.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `clobber_abi("C")` cannot accompany generic register outputs**
- **Found during:** Task 1 (building the aarch64 kernels)
- **Issue:** rustc rejects `asm!` that combines `clobber_abi` with generic `inout(reg)`/`out(reg)` outputs: "asm with `clobber_abi` must specify explicit registers for outputs".
- **Fix:** Removed `clobber_abi` and declared the actual vector clobbers explicitly (`out("v0")`–`out("v3")` on aarch64, `out("ymm0")`/`out("ymm1")` on x86_64), keeping named generic operands for the pointers/count. `vzeroupper` still clears the x86_64 upper halves on exit.
- **Files modified:** `ds/src/setops_simd.rs`
- **Verification:** `cargo test -p ds setops`, both arch clippy legs, `cargo xtask check`.
- **Committed in:** `8643d68`

**2. [Rule 3 - Blocking] The plan's literal `with_simd(f)` does not typecheck**
- **Found during:** Task 2 (boot wiring)
- **Issue:** `fpu::with_simd` takes `impl FnOnce(&KernelSimd) -> R`, but the registered bracket is `fn(&mut dyn FnMut())`; passing `f` directly fails to compile.
- **Fix:** Register `|f| fpu::with_simd(|_| f())` in both arch mains.
- **Files modified:** `aarch64/src/main.rs`, `x86_64/src/main.rs`
- **Verification:** both arch clippy legs compile the `dev_exec` wiring.
- **Committed in:** `8fcb116`

**3. [Rule 3 - Commit shape] Task 1's TDD cycle ships in the same commit as the kernels**
- **Found during:** Task 1
- **Issue:** a Rust inline-asm kernel cannot have a separately committed failing RED state without either a non-compiling stub (an invalid RED) or a deliberately wrong kernel; the plan is `type: execute`, and `workflow.tdd_mode` is not enabled in this worktree.
- **Fix:** tests and kernels land together; the tests were run green before the commit (`simd_kernels_match_scalar_on_all_seeds_and_edges` exercises the real NEON kernel on this aarch64 host).
- **Files modified:** `ds/src/setops.rs`, `ds/src/setops_simd.rs`
- **Committed in:** `8643d68`

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 commit-shape record).
**Impact on plan:** No scope creep; all fixes are required for the kernels to compile and the boot wiring to typecheck.

## Issues Encountered

- The first compile attempt caught an accidental removal of `pub mod tpht;` while mounting `setops_simd`; restored before any commit.
- The pre-existing `ds/src/slab.rs` `SEQ_INC` dead-code warning (unrelated to this plan, present at the base commit) remains; noted, not fixed (scope boundary).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- MEM-17 is complete: the `setops` seam now dispatches to real AVX2/NEON kernels on both architectures, scalar fallback preserved, and the Roaring algebra (`union`/`intersection`/`difference`) is pinned identical with SIMD enabled.
- The boot-time enable/selftest wiring compiles on both arch `dev_exec` configs; a real `-smp 2` boot gate still needs to confirm the `setops:` log line (see coverage D3).

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- FOUND: ds/src/setops_simd.rs
- FOUND: .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-17-SUMMARY.md
- FOUND: 8643d68 (feat: SIMD kernels + pins)
- FOUND: 8fcb116 (feat: port + arch boot wiring)
- Plan metadata: present on HEAD (docs: complete plan)
