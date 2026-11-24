---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 12
subsystem: ds
tags: [ds, roaring, workspace, no_std, xtask, kernel-byte-evidence, lint-contract]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto/ workspace crate, xtask test legs, kernel byte baseline (@03-01)"
provides:
  - "ds/ workspace crate with the D-09 per-module forbid layout"
  - "roaring =0.11.5 as the only external crate, under [dev-dependencies] with a provenance comment"
  - "cargo xtask check legs for both r9 kernel specs with -Z build-std=core,alloc plus the two built-in bare-metal targets"
  - "cargo xtask test legs: crypto+ds host, crypto scalar backend, crypto --doc"
  - "kernel sha256 evidence: both kernels IDENTICAL after ds/ joined (@03-12), final Phase 3 Cargo.lock"
affects: [03-02, 03-05, 03-06, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 3782
  tasks: 2
  commits: 2
  plan_head_before: aa188cd7d694bd4f659d63774ea2bf7deea8ba9d
  plan_head_after: 2224868c7135522acde4ddcfde05d9205d6f5b87

tech-stack:
  added:
    - "roaring =0.11.5 (dev-dependency only, default-features = false)"
  patterns:
    - "Per-module forbid with a named crate-root #[path] exception reserved for the Phase 5 SIMD module"
    - "Cross legs use --lib only so dev-dependencies and std never reach a bare-metal target"
    - "Kernel byte comparison from moved-aside target directories on one venue"

key-files:
  created:
    - ds/Cargo.toml
    - ds/src/lib.rs
    - ds/src/tpht/mod.rs
    - ds/src/roaring.rs
    - ds/src/setops.rs
    - ds/src/clientset.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-12.md
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/after-03-12.sha256
  modified:
    - Cargo.toml
    - Cargo.lock
    - xtask/src/main.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/KERNEL-BYTES.md

key-decisions:
  - "The ds module declarations keep the plan's order; rustfmt's module reordering is not applied because cargo fmt --check is already red repo-wide on pre-existing files"
  - "roaring 0.11.5 with default-features = false resolves without new transitive crates: Cargo.lock gained exactly the ds and roaring entries (13 lines, 0 removed)"
  - "The two lib/*-unknown-none-elf.json legs are the FP/SIMD-free proof; the built-in legs are kept only to meet D-21's wording"
  - "CheckStep::run's fixed leg list hosts the new legs, so the runner loop and its Err(\"check failed\") contract are unchanged"
  - "Kernel target dirs moved to target/phase03-kernel-bytes/pre-03-12-targets/ before rebuilding; both hashes equal before.sha256, so D-21's stop did not fire"

requirements-completed: [LINT-01]

coverage:
  - id: D1
    description: "ds/ joins the workspace beside crypto/; roaring =0.11.5 is the only external crate, under [dev-dependencies] with a provenance comment, and Cargo.lock gained entries only"
    requirement: LINT-01
    verification:
      - kind: integration
        ref: "nix develop -c cargo xtask check --verbose (rc 0, 9 legs; host leg compiles both packages)"
        status: pass
      - kind: other
        ref: "git diff --numstat 4beb5c8 HEAD -- Cargo.lock => 212 added, 0 removed"
        status: pass
    human_judgment: false
  - id: D2
    description: "ds/src/lib.rs carries the D-09 layout: tpht with no forbid, #[forbid(unsafe_code)] directly before roaring, setops and clientset; the workspace scope comment names the four opted-in crates"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "grep -cE '^(pub mod tpht;|#\\[forbid\\(unsafe_code\\)\\]|pub mod roaring;|mod setops;|pub mod clientset;)' ds/src/lib.rs => 7 (>= 5)"
        status: pass
      - kind: other
        ref: "cat port/Cargo.toml aarch64/Cargo.toml x86_64/Cargo.toml riscv64/Cargo.toml | grep -cE '^(crypto|ds)\\b' => 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "cargo xtask check compiles crypto and ds for both r9 kernel specs (-Z build-std=core,alloc, --lib) and for the two built-in bare-metal targets; cargo xtask test runs crypto+ds, the scalar crypto leg and the crypto --doc leg"
    requirement: LINT-01
    verification:
      - kind: integration
        ref: "nix develop -c cargo xtask check --verbose (rc 0; four CROSS-LEG rc=0 lines in evidence/PINS-03-12.md)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test (rc 0)"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask clippy (rc 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Both kernels are byte-identical to the baseline after ds/ and the roaring dev-dependency joined, on the baseline venue"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "evidence/kernel-bytes/KERNEL-BYTES.md: VERDICT elf-virtshell@03-12 and elf-q35churnshell@03-12 IDENTICAL; cmp against target/phase03-kernel-bytes/before/ identical"
        status: pass
    human_judgment: false

# Metrics
duration: 2min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 12: `ds/` joins the workspace; check legs for both kernel specs; kernels byte-identical

**`ds/` joins beside `crypto/` with per-module forbids and a lone `roaring` dev-dependency; `cargo xtask check` proves both crates build for both r9 kernel specs, and both kernels stay byte-identical.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-24T14:25:31Z
- **Completed:** 2026-11-24T14:28:31Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- `ds/` is a workspace member beside `crypto/`. Its only external crate is the dev-dependency `roaring =0.11.5` (`default-features = false`) with a four-part provenance comment; `Cargo.lock` gained 13 lines and removed none (the `ds` package and `roaring` only).
- `ds/src/lib.rs` carries the D-09 layout: `pub mod tpht;` with no forbid, then `#[forbid(unsafe_code)]` directly before `roaring`, `setops` (private) and `clientset`; the header comment names the reserved Phase 5 SIMD module and says a third unsafe-holding module needs owner approval. The four module files hold their module docs only, as the plan specifies.
- The workspace lint scope comment now names the four opted-in crates (`aarch64`, `port`, `crypto`, `ds`), keeps `x86_64`/`riscv64` opted out, and records that extending the lint to `x86_64` is Phase 5's question.
- `cargo xtask test` runs `crypto` and `ds` together on the host (`--lib --tests`), keeps the scalar-backend `crypto` leg and adds the `crypto --doc` leg.
- `cargo xtask check` gained five legs: the host `crypto`+`ds` leg, the two r9 kernel-spec cross legs (`-Z build-std=core,alloc`, `--lib`) and the two built-in bare-metal legs. The aarch64 spec leg carries the two soft cfgs through its own `--config`.
- Both kernels, rebuilt from nothing after moving the target directories aside, are byte-identical to the baseline: `virtshell` `9218b998…`, `q35churnshell` `c9dcef0b…`. `cmp` against the baseline copies agrees.

## Task Commits

Each task was committed atomically:

1. **Task 1: `ds/` joins, forbid layout, check/test legs** — `25af196` (feat)
2. **Task 2: kernel bytes after `ds/` joins** — `2224868` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `Cargo.toml` — `members = ["aarch64", "x86_64", "port", "riscv64", "xtask", "crypto", "ds"]`; the lint scope comment rewritten for the four opted-in crates
- `Cargo.lock` — the `ds` package and `roaring 0.11.5`: 13 added lines, 0 removed
- `ds/Cargo.toml` — package `ds`, no `[dependencies]`, `roaring =0.11.5` under `[dev-dependencies]` with provenance, `[lints] workspace = true`
- `ds/src/lib.rs` — `no_std` crate root with `forbid(unsafe_op_in_unsafe_fn)`, the D-09 header contract and the four module declarations
- `ds/src/tpht/mod.rs`, `ds/src/roaring.rs`, `ds/src/setops.rs`, `ds/src/clientset.rs` — module doc comments only (contents in plans 03-05/03-06)
- `xtask/src/main.rs` — `TestStep`: crypto+ds host leg, `crypto --doc` leg; `CheckStep`: host, two r9-spec cross and two built-in legs with the C1 proof comment
- `evidence/PINS-03-12.md` — four `CROSS-LEG <target> rc=0` lines from the verbose check run
- `evidence/kernel-bytes/after-03-12.sha256` — `after 25af196…`, `venue Darwin arm64`, both ELF hashes
- `evidence/kernel-bytes/KERNEL-BYTES.md` — appended `@03-12` section; the `@03-01` section is unchanged
- `deferred-items.md` — the pre-existing repo-wide `cargo fmt --check` drift, out of scope for this plan

## Decisions Made

- The `ds` module declarations keep the plan's exact order (`tpht`, `roaring`, `setops`, `clientset`). rustfmt's default module reordering is not applied: `cargo fmt --check` is already red repo-wide on pre-existing files, and the plan names this order explicitly.
- `roaring 0.11.5` with `default-features = false` resolves without pulling `bytemuck`/`byteorder`: the lock gained exactly the `ds` and `roaring` package entries (13 lines).
- The two `lib/*-unknown-none-elf.json` legs are the FP/SIMD-free proof; the built-in legs are kept only to satisfy D-21's wording, and the code comment says the built-in aarch64 target has NEON on and proves nothing (RESEARCH C1).
- `CheckStep::run`'s fixed leg list hosts the new legs, so the existing runner loop and `Err("check failed")` contract are unchanged.
- Kernel target dirs were moved to `target/phase03-kernel-bytes/pre-03-12-targets/` before rebuilding. Both hashes equal `before.sha256`, so D-21's stop did not fire and the phase continues.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. `cargo xtask check`, `test` and `clippy` all exited 0; the only warnings are the pre-existing ones in the arch crates.

## Known Stubs

The four `ds` module files are the plan's intentional doc-only skeletons, not unfinished work for this plan's goal:

| File | Line | Reason | Resolved by |
|------|------|--------|-------------|
| `ds/src/tpht/mod.rs` | 1 | module doc only; TPHT contents are plan 03-06 | plan 03-06 |
| `ds/src/roaring.rs` | 1 | module doc only; Roaring contents are plan 03-05 | plan 03-05 |
| `ds/src/setops.rs` | 1 | module doc only; the D-20 hot loops are plan 03-05 | plan 03-05 |
| `ds/src/clientset.rs` | 1 | module doc only; `ClientSet` contents are plan 03-05 | plan 03-05 |

They are recorded in the WINDOWS ledger as four `stub` entries for phase 03.

## Threat Flags

None beyond the plan's `<threat_model>`: the new surface is the `roaring` dev-dependency and its lock entry (T-03-SC, T-03-47) and the host-vs-kernel build split (T-03-45), all mitigated as planned. No kernel crate depends on `crypto/` or `ds/`, and the cross legs use `--lib` only.

## Optional for the owner

OPTIONAL, not a stop and not a pass criterion. RESEARCH assumption A8 (the nix-sandboxed `kernel-x64` and `kernel-virt` derivations build with the enlarged `Cargo.lock`) is otherwise first tested at the end-of-phase owner stop (plan 03-11). The owner may test it earlier by running `nix build .#kernel-x64` and `nix build .#kernel-virt` on the Linux host at any commit from this plan's last one onward. This plan's last commit (`2224868`) is the **first one with the final Phase 3 `Cargo.lock`**: no later plan adds a crate, so this is the lock state the early build and the end-of-phase stop both vendor from. The executor pushes nothing for this and waits for nothing; getting the commits to the Linux host is the owner's own action under the standing push rule. Later plans proceed whether or not it is run. If the owner reports a result, record it as one line `A8-EARLY <sha> kernel-x64=<rc> kernel-virt=<rc>` in `evidence/kernel-bytes/KERNEL-BYTES.md`; a reported build failure is returned to the orchestrator as a checkpoint. The required check stays the single stop in plan 03-11.

## Next Phase Readiness

- The build shape every later plan builds in is proved: `ds/` is a member, the forbid layout is in place, `Cargo.lock` has its final Phase 3 shape, and both r9 kernel specs compile both crates `no_std` with no FP/SIMD.
- Both kernels are byte-identical after the workspace change, so no owner kernel-byte procedure is needed.
- Plan 03-02 starts next without waiting: it commits the DS-07/TPHT RED pins before any Funnel Hashing or TPHT behaviour lands.
- `zeroize =1.9.2` remains flagged for the owner's review at phase verification (plan 03-09 rows, plan 03-10 checklist); nothing here changes it.

## Self-Check: PASSED

All claimed files exist on disk and both task commits are ancestors of HEAD. `cargo xtask check` (rc 0, 9 legs), `cargo xtask test` (rc 0) and `cargo xtask clippy` (rc 0) ran at `25af196`; the two `@03-12` verdicts are `IDENTICAL` and the `@03-01` verdicts are unchanged.
