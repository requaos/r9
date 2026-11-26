---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 04
subsystem: safety
tags: [unsafe, clippy, undocumented_unsafe_blocks, x86_64, aarch64, lintcontract, ci, unsafe-audit, corr-05]

# Dependency graph
requires:
  - phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools plan 01
    provides: green CI harness (CORR-01) and pure boot-iso (CORR-02) — green CI on the commit is the x86_64 kernel byte-change check (D-05)
  - phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools plan 03
    provides: 31 of 39 arch undocumented unsafe blocks documented (7/7 aarch64, 24/32 x86_64); virtshell clippy green; the SAFETY-comment conventions this plan reused
provides:
  - all 39 arch unsafe blocks documented: the last 8 x86_64 sites carry concrete // SAFETY: comments (trap.rs x2, uart16550.rs, proc.rs x2 SMP-03 audit sites, main.rs x3)
  - x86_64/Cargo.toml opted into [lints] workspace = true (clippy::undocumented_unsafe_blocks = deny) — the fifth gated crate
  - UNSAFE-AUDIT.md reconciled to the live tree: measured per-file inventories for both arch crates, stale deleted-file row dropped, uniform policy statement (crypto crate-wide forbid; ds tpht + reserved Roaring SIMD module; port floors globalalloc/mcslock/devip_waker)
  - xtask/src/lintcontract.rs R6 enforcing the opt-in on aarch64/Cargo.toml and x86_64/Cargo.toml with fixture + negative tests
  - .github/workflows/xtask.yml lint job running both arch clippy legs (aarch64 virtshell, x86-64 q35cpu)
affects: [phase-05 later plans (05-06 slab FidTable on port/src/fid.rs, SMP-03 audits, MEM), phase-07 riscv64 sync, ci, xtask]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (48000, confidence: low).
actuals:
  tokens: 8277
  tasks: 3
  commits: 3
  plan_head_before: 276e2e9c24def6d967bc1be5ac839340028db0e2
  plan_head_after: 222b6e1499703cb5602408bc5c51549c105419d3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "x86_64 privileged-init SAFETY shape: mb2 EFLAGS.IF=0 contract + exclusively borrowed Mach + the callee fn's own # Safety contract (main.rs:131), MSR-value/GDT consistency (arm_syscall_entry)"
    - "cfg-gated-out unsafe sites carry SAFETY comments the gate clippy legs cannot see; UNSAFE-AUDIT.md records them per-file and the deny lint catches them the moment a config builds them"
    - "R6 lintcontract extension pattern: add the manifest to the scan list, carry the opt-in in the fixture, extend the negative test per manifest"

key-files:
  created: []
  modified:
    - x86_64/src/trap.rs
    - x86_64/src/uart16550.rs
    - x86_64/src/proc.rs
    - x86_64/src/main.rs
    - x86_64/Cargo.toml
    - Cargo.toml
    - UNSAFE-AUDIT.md
    - xtask/src/lintcontract.rs
    - .github/workflows/xtask.yml

key-decisions:
  - "CI lint legs run bare cargo like the existing lint/tests/build jobs — rustup honors rust-toolchain.toml and clippy needs no devshell tools (fdtput/qemu serve the qemu/initrd steps only); nix develop -c remains the local venue spelling"
  - "UNSAFE-AUDIT.md's reconciliation now re-measured (2026-11-26) by a comment-stripping static scan: aarch64 186 / x86_64 158 / port 22 / ds 1 / crypto 0 production lint-visible blocks, zero undocumented tree-wide; the stale aarch64 129 and the deleted-file x86_64 row are gone"
  - "proc.rs:852/873 SAFETY comments carry the '(an SMP-03 audit site)' marker — the single-CPU IRQ-masked invariant those unwinds rest on, continuing 05-03's annotation pattern so the SMP-03 sweep finds them mechanically"

patterns-established:
  - "Per-file unsafe inventory as the count of record: blocks/fns/externs per file for every tracked arch .rs file, with curated per-identifier invariant tables kept for the landmark sites"

requirements-completed: [CORR-05]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "All remaining 8 undocumented unsafe blocks in x86_64/src/{trap.rs,uart16550.rs,proc.rs,main.rs} carry concrete // SAFETY: comments and x86_64/Cargo.toml opts into [lints] workspace = true"
    requirement: CORR-05
    verification:
      - kind: other
        ref: "grep -v '^[[:space:]]*#' x86_64/Cargo.toml | grep -c 'workspace = true' == 1"
        status: pass
      - kind: other
        ref: "nix develop -c cargo xtask clippy --arch x86-64 --config q35cpu → exit 0, zero undocumented_unsafe_blocks diagnostics"
        status: pass
    human_judgment: false
  - id: D2
    description: "UNSAFE-AUDIT.md inventories every actual x86_64/src and aarch64/src file, drops the deleted-file row, and states the crypto/ds/port policy uniformly (port floors globalalloc, mcslock, devip_waker)"
    requirement: CORR-05
    verification:
      - kind: other
        ref: "grep -v '^[[:space:]]*#' UNSAFE-AUDIT.md | grep -cE 'node0\\.rs|vsvm\\.rs' == 0; comment-stripping static scan over every tracked arch .rs file reproduced the inventory counts"
        status: pass
    human_judgment: false
  - id: D3
    description: "xtask/src/lintcontract.rs R6 enforces [lints] workspace = true on x86_64/Cargo.toml as well as aarch64/Cargo.toml (and crypto/ds), with fixture and negative coverage per arch manifest"
    requirement: CORR-05
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask lintcontract → exit 0, 13/13 (incl. lintcontract_workspace_passes and the per-arch missing-opt-in rejections)"
        status: pass
    human_judgment: false
  - id: D4
    description: ".github/workflows/xtask.yml lint job runs both arch clippy legs (aarch64 virtshell, x86-64 q35cpu) with errors failing the job; both legs plus cargo xtask test green locally"
    requirement: CORR-05
    verification:
      - kind: other
        ref: "nix develop -c cargo xtask clippy --arch aarch64 --config virtshell → exit 0; --arch x86-64 --config q35cpu → exit 0; nix develop -c cargo xtask test → exit 0 (56 legs ok)"
        status: pass
    human_judgment: true
    rationale: "Hosted-runner execution of the edited workflow is only observable at push time (Owner Stop 1, D-32); the local runs prove the commands and the grep proves the wiring"

# Metrics
duration: 10min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 04: Unsafe Policy — Last 8 x86_64 SAFETY Comments, Lint Opt-In, Audit Reconciliation and CI Legs (CORR-05 Part B) Summary

**The final 8 undocumented `unsafe` blocks in `x86_64/` carry concrete `// SAFETY:` comments and `x86_64/Cargo.toml` opts into the workspace `undocumented_unsafe_blocks = deny` lint — both arch clippy legs (`aarch64 virtshell`, `x86-64 q35cpu`) are green and now run in CI; `UNSAFE-AUDIT.md` is re-measured against the live tree with per-file inventories for every arch source file, and lintcontract R6 pins the opt-in on both arch manifests.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-11-26T05:07:00Z
- **Completed:** 2026-11-26T05:17:46Z
- **Tasks:** 3
- **Files modified:** 9

## Accomplishments

- **Task 1 — the 8 remaining sites documented, x86_64 opted in (CORR-05, D-02, D-03).** `trap.rs:69` (`Stub::as_thunk` — the transmuted usize names a real stub inside `intr_stubs`, installed only into IDT entries) and `trap.rs:136` (`stubs()` — the 4096-aligned naked emitter backs exactly 256 × 8 bytes; `Stub` is `repr(transparent)` over `usize`); `uart16550.rs:14` (`putb` — CPL 0 write to the architectural COM1 THR); `proc.rs:852` and `:873` (the brk-growth unwind pair — PROC_TABLE-resident member pointers, exclusive `&mut` under single-CPU IRQ-masked syscall context, annotated as SMP-03 audit sites per 05-03's pattern); `main.rs:131` (boot descriptor/MSR arming on the exclusively borrowed Mach under the mb2 EFLAGS.IF=0 contract), `:338` (`arm_syscall_entry`'s MSR program consistent with the just-loaded GDT) and `:374` (`halt`'s cli/hlt park). `x86_64/Cargo.toml`'s manual `[lints.rust]` check-cfg block is replaced by `[lints] workspace = true` — the workspace manifest already declares all seven x86_64 check-cfg entries — and the workspace `Cargo.toml` scope comment now names five opted-in crates. Verified: `cargo xtask clippy --arch x86-64 --config q35cpu` exits 0 with zero `undocumented_unsafe_blocks` diagnostics.
- **Task 2 — UNSAFE-AUDIT.md reconciled, R6 extended (CORR-05).** The audit is re-measured by a comment-stripping static scan over every tracked `.rs` file: production lint-visible blocks aarch64 **186** (replacing the stale Phase-3-era 129 — growth is real post-measurement code: virt pci.rs, gicv3.rs, fpu.rs, textcopy.rs, churngate.rs, the proc.rs rfork/brk machinery), x86_64 **158** (new row), port **22** (unchanged, matches Phase 4's measurement exactly: 14 blocks + 8 impls across globalalloc/mcslock/devip_waker), ds **1**, crypto **0**; test-only blocks aarch64 6, x86_64 8, port 11; unsafe fns 50; extern blocks 8. Zero undocumented production sites tree-wide. New measured per-file inventories list all 26 aarch64 and 23 x86_64 unsafe-bearing files with block/fn/extern counts and surface notes; the legacy x86_64 row naming deleted files is dropped and riscv64 stays count-only (Phase 7 horizon). The uniform policy is stated once: crypto crate-wide `#![forbid(unsafe_code)]`, ds per-module (`tpht/` + the one reserved Roaring SIMD module under MEM-17), port per-module with the three named floor modules (D-02), and the deny lint on all five opted-in crates. `xtask/src/lintcontract.rs` R6 now scans `aarch64/Cargo.toml` and `x86_64/Cargo.toml` alongside crypto/ds, the fixture carries the arch opt-ins, and the missing-opt-in test rejects both arch manifests. Verified: `cargo test -p xtask lintcontract` 13/13, exit 0.
- **Task 3 — both arch clippy legs in CI (CORR-05).** The `lint` job in `.github/workflows/xtask.yml` now runs `cargo xtask clippy --arch aarch64 --config virtshell --verbose` and `cargo xtask clippy --arch x86-64 --config q35cpu --verbose` after the default legs, with the run block's default `bash -eo pipefail` failing the job on any error. Bare cargo matches the job's existing mechanism (rustup installs the pinned toolchain and the `*-unknown-none` targets from `rust-toolchain.toml`; clippy needs no devshell tools). Verified locally: both arch legs exit 0 and `cargo xtask test` exits 0 (56 legs ok).

## Task Commits

Each task was committed atomically:

1. **Task 1: Document remaining 8 x86_64 unsafe blocks, opt into workspace deny lint (CORR-05)** — `78b0234` (fix)
2. **Task 2: Reconcile UNSAFE-AUDIT.md with the live tree, R6 covers both arch manifests (CORR-05)** — `c45b7bf` (fix)
3. **Task 3: Run both arch clippy legs in the CI lint job (CORR-05)** — `222b6e1` (feat)

**Plan metadata:** the SUMMARY commit (docs: complete plan).

## Files Created/Modified

- `x86_64/src/trap.rs` — SAFETY comments at `as_thunk` (+7 lines) and `stubs()` (+7 lines)
- `x86_64/src/uart16550.rs` — SAFETY comment at `putb` (+4 lines)
- `x86_64/src/proc.rs` — SAFETY comments at both brk-growth unwind sites (+18 lines), with SMP-03 audit markers
- `x86_64/src/main.rs` — SAFETY comments at the boot arming block, `arm_syscall_entry`, and `halt` (+21 lines)
- `x86_64/Cargo.toml` — `[lints] workspace = true` replaces the manual `[lints.rust]` check-cfg block; comment rewritten for the opt-in
- `Cargo.toml` — workspace lint comment names the five opted-in crates (x86_64 joined at CORR-05)
- `UNSAFE-AUDIT.md` — scope line and counting rule name five gated crates; reconciliation re-measured (2026-11-26); Phase 5 delta recorded; uniform policy statement; per-file inventories for both arch crates; stale gic.rs entry re-homed to gic/mod.rs + gicv3.rs; out-of-scope table is riscv64-only with the deleted-file row dropped
- `xtask/src/lintcontract.rs` — R6 scans all four manifests (crypto, ds, aarch64, x86_64); fixture carries arch opt-ins; missing-opt-in test covers both arch crates
- `.github/workflows/xtask.yml` — lint job runs the aarch64 virtshell and x86-64 q35cpu clippy legs

## Decisions Made

- **Bare cargo in the CI lint job:** the plan's action text spelled the legs `nix develop -c cargo xtask clippy …` (the local venue form), while the acceptance criterion names the commands without the prefix. The lint job's existing legs already run bare cargo on `ubuntu-latest` — rustup installs the pinned `nightly-2026-01-30` toolchain and the three `*-unknown-none` targets from `rust-toolchain.toml`, and the clippy step applies only platform cfg rustflags (no build-std, no link script, no fdtput/qemu — those serve the build/qemu steps). Adding the nix-installer to the lint job would tax every run for nothing the legs use.
- **Reconciliation by static scan, lint proof by the gate configs:** the audit's "zero undocumented production sites" claim rests on the comment-stripping scan (tree-wide, cfg-gated code included), while clippy's deny lint proves the compiled configs mechanically (virtshell exit 0, q35cpu exit 0). The scan's method and the per-file counts are stated in the document so the next re-measurement reproduces it.
- **SMP-03 markers on the proc.rs unwinds:** both sites rest on the single-CPU IRQ-masked invariant; they say so with 05-03's `(an SMP-03 audit site)` wording so the SMP-03 sweep finds them mechanically (05-PATTERNS.md shared pattern 2: never justify unsafe with "single CPU, IRQ-masked" once SMP lands).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated the stale scope comments the opt-in falsified**
- **Found during:** Task 1
- **Issue:** The workspace `Cargo.toml` header comment claimed "x86_64/riscv64 … stay opted out" and "Extending the lint and audit to x86_64 is Phase 5's question", and `x86_64/Cargo.toml`'s comment said the crate "is not lint-opted-in like riscv64" — both made false by this task's own change. The plan's `files_modified` does not list the workspace `Cargo.toml`.
- **Fix:** Updated both comments in the task commit: the workspace header names the five opted-in crates with the CORR-05 provenance; the x86_64 manifest comment now records the opt-in and keeps the per-cfg provenance of the seven check-cfg entries.
- **Files modified:** `Cargo.toml`, `x86_64/Cargo.toml`
- **Verification:** `grep -c 'workspace = true'` pin == 1 (comment-stripped) and q35cpu clippy exit 0 — behavior unchanged, comments-only delta on the workspace manifest
- **Committed in:** `78b0234` (part of task commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug)
**Impact on plan:** Documentation accuracy restored at the point the plan's own change falsified it; zero behavioral delta. No scope creep.

## Authentication Gates

None.

## Issues Encountered

None — the 05-03 summary's line numbers matched the live tree exactly, and the re-measurement instrument (comment-stripping scan with cfg(test) scope tracking) was built from the audit's own counting rule. `port`'s 22 re-measured identically to the Phase 4 record, cross-validating the instrument.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Comment, manifest, audit-document and CI edits only; no code paths stubbed, no TODO/FIXME introduced (verified over the plan diff).

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. T-05-06's mitigation (opt x86_64 into the deny lint, pin with R6, gate both arches in CI) is exactly what this plan delivers; T-05-SC is N/A (`Cargo.lock` untouched, zero external packages installed).

## Next Phase Readiness

- **CORR-05 is delivered in full** (Part A in 05-03, Part B here); the requirement stays open until 05-06 — which also declares CORR-05 — produces its SUMMARY (shared-ID gate). The lint contract now fails the build if either arch manifest drops the opt-in, and CI fails if an undocumented unsafe block lands in either compiled arch config.
- **SMP-03 (05-07+):** the audit markers on the proc.rs unwind pair join 05-03's aarch64 markers; UNSAFE-AUDIT.md's measured per-file inventories give the SMP-03 audit a trustworthy site map for both arch crates.
- **riscv64** stays deliberately count-only in the audit; its lint/audit horizon is the Phase 7 sync (recorded in the out-of-scope rationale).
- Kernel byte-change procedure (D-05): comment/manifest/docs/yml edits leave `.text` unchanged; with CORR-02 landed (05-01), green CI on the commit is the check.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 10 plan files (this SUMMARY + 9 modified) exist on disk.
- All 3 task commits (`78b0234`, `c45b7bf`, `222b6e1`) are ancestors of HEAD.
- Content pins: `workspace = true` in `x86_64/Cargo.toml`; SAFETY counts trap.rs 6 / uart16550.rs 5 / proc.rs 34 / main.rs 10; R6 scans both arch manifests; both arch clippy legs in `xtask.yml`.
- Measured commits from ledger (`gsd-plan-head-before-05-04` = `276e2e9c`): 3 task commits; `plan_head_after` = `222b6e1` (this SUMMARY commit is the fourth).
- Working tree carries no untracked files besides this SUMMARY (committed below); `Cargo.lock` untouched; Harvey untouched.
- Plan-level verification re-run at completion: `cargo xtask clippy --arch aarch64 --config virtshell` exit 0, `--arch x86-64 --config q35cpu` exit 0 with zero undocumented_unsafe_blocks, `cargo test -p xtask lintcontract` 13/13 exit 0, `cargo xtask test` exit 0 (56 legs ok).
