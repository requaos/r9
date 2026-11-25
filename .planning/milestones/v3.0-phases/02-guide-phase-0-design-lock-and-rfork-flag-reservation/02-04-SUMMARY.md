---
phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
plan: 04
subsystem: kernel
tags: [rfork, flags, imports, mask, xtask, host-tests, tdd, kernel-bytes]

# Dependency graph
requires:
  - phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
    plan: 01
    provides: "`port/src/rfork.rs` (sixteen flags, `ALL`, `FIRST_R9_ONLY`), the generated `sys/include/rfork.h`, `cargo xtask rfork-header --check`, and the pre-edit kernel hashes"
provides:
  - "Both arch `syscall.rs` files import the eleven implemented flags from `port::rfork`; no arch crate defines an rfork flag and no arch file re-exports one (D-18)"
  - "`RFORK_SUPPORTED` at module level in both arch `syscall.rs` files, value 0x3c7f, pinned by `rfork_mask_tests` (D-19, D-20)"
  - "`rfork_flags_have_one_definition` in xtask: the four arch sources cannot define or re-export an rfork flag again, and each mask declaration appears exactly once"
  - "`cargo xtask test` fails on a stale or missing `sys/include/rfork.h` before any cargo leg (D-12)"
  - "`cargo xtask test` selects the host triple for the x86_64 arch-crate leg on an x86_64 host without rustup (D-24)"
affects: [phase-02-plan-05, phase-02-plan-06, phase-06, phase-07]

# Actuals (#2632) — measured on the same chars/4 scale as the plan's estimate.
actuals:
  tokens: 5910
  tasks: 3
  commits: 4
  plan_head_before: 1abde5a0f1dbc429eedf491656757e33e52febc9
  plan_head_after: 6dd3f9a6a697a3aa9bfc52f4116080d9e6afbb27

tech-stack:
  added: []
  patterns:
    - "A shared constant file is imported privately by name; a `pub use` or `pub(crate) use` re-export is itself a red invariant"
    - "A function-local mask is hoisted to a named module-level const so a host test can assert the reserved bits are outside it"
    - "An xtask source invariant reads the four arch files with `include_str!` and builds its forbidden patterns from separate string pieces"
    - "The project test command runs the generated-header drift check before any cargo leg"

key-files:
  created: []
  modified:
    - aarch64/src/syscall.rs
    - aarch64/src/proc.rs
    - x86_64/src/syscall.rs
    - x86_64/src/proc.rs
    - xtask/src/rforkh.rs
    - xtask/src/main.rs
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/PINS.md

key-decisions:
  - "The two `rfork_mask_tests` modules are byte-identical; only the aarch64 copy was compiled and run on this Mac"
  - "The x86_64 mask test and the `RFPROC` import inside `x86_64/src/proc.rs` `mod tests` are unverified here; the owner's x86_64 Linux host is the first compile (D-24), recorded in PINS.md and the WINDOWS ledger"
  - "`uses_host_triple_fallback` is a pure function so the widened leg is unit-tested on the Mac without a rustup-free x86_64 host"
  - "`graphify update .` was run after each code change; `/graphify-out` stays ignored"

patterns-established:
  - "One-definition invariants live in xtask as tests over `include_str!` sources, collecting every violation so one failure names all files and lines"
  - "A drift check added to `TestStep` is proven by mutating the generated file, asserting exit 1 and the stale line, then restoring it"

requirements-completed: [DSGN-03, DSGN-04]

coverage:
  - id: D1
    description: "No arch crate defines or re-exports an rfork flag; x86_64/src/proc.rs and aarch64/src/syscall.rs import the eleven names from `port::rfork`, and aarch64/src/proc.rs reads them through `port::rfork`"
    requirement: "DSGN-03"
    verification:
      - kind: integration
        ref: "nix develop -c cargo test -p xtask rfork_flags_have_one_definition (green); grep counts 0/0/0/12, and no `pub(use)` re-export in any arch file"
        status: pass
    human_judgment: false
  - id: D2
    description: "Each arch `syscall.rs` has a module-level `pub(crate) const RFORK_SUPPORTED: u64` equal to 0x3c7f, and `sys_rfork` keeps the 32-bit mask, the three pair checks and the single rejection return"
    requirement: "DSGN-04"
    verification:
      - kind: unit
        ref: "nix develop -c cargo xtask test, twice: `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok` twice per run (aarch64 default and virtshell cfg legs)"
        status: pass
      - kind: other
        ref: "grep: RFORK_SUPPORTED declaration exactly once in each syscall.rs; `return err(proc, \"rfork: unsupported flags\");` exactly once in each; the sys_rfork diff per arch changes one code line besides the removed local constant"
        status: pass
    human_judgment: false
  - id: D3
    description: "An xtask test goes red when an arch file defines a flag constant again, reaches one through `crate::syscall`, keeps a function-local mask, declares the mask zero or twice, or re-exports a `port::rfork` name"
    requirement: "DSGN-03"
    verification:
      - kind: integration
        ref: "at the RED commit: 22 flag-constant lines across x86_64/src/proc.rs and aarch64/src/syscall.rs, 12 crate::syscall paths, two function-local masks and two zero declarations all named in one failure; green after Task 2"
        status: pass
    human_judgment: false
  - id: D4
    description: "`cargo xtask test` exits 1 with `is stale against port/src/rfork.rs` before any cargo leg when the generated header drifts"
    requirement: "DSGN-03"
    verification:
      - kind: integration
        ref: "mutate `(1<<21)` to `(1<<22)` in sys/include/rfork.h, run nix develop -c cargo xtask test, observe exit 1 and the stale line, restore: header clean"
        status: pass
    human_judgment: false
  - id: D5
    description: "The arch-crate host leg falls back to the host triple on x86_64 hosts as well as aarch64 when rustup reports no target"
    requirement: "DSGN-04"
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask host_triple_fallback: `host_triple_fallback_covers_aarch64_and_x86_64 ... ok`"
        status: pass
      - kind: other
        ref: "the x86_64 leg itself cannot be run on the Mac (no Rosetta, no x86_64 std in the nix toolchain); witness is the owner's x86_64 Linux host (D-24, plan 02-06), recorded as `X86_64-HOST-TESTS not-run-on-mac` and WINDOWS #5"
        status: pending-owner
    human_judgment: true

duration: 2min
completed: 2026-11-24
status: complete
---

# Phase 2 Plan 04: Both arch crates import from `port::rfork`; masks hoisted and pinned; the header check joins `cargo xtask test` Summary

**The sixteen rfork flags now have exactly one definition: both kernels import the eleven they implement from `port::rfork`, each mask is a module-level `RFORK_SUPPORTED` pinned to `0x3c7f` by an identical host test, and `cargo xtask test` now refuses a stale generated header before any cargo leg and reaches the x86_64 crate's tests on a rustup-free x86_64 host.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-24T02:33:17Z
- **Completed:** 2026-11-24T02:35:33Z
- **Tasks:** 3
- **Files modified:** 7 (four arch files, two xtask files, the pin ledger)

## Accomplishments

- `aarch64/src/syscall.rs` and `x86_64/src/proc.rs` no longer define any `rfork` flag; both `syscall.rs` files import the eleven implemented names from `port::rfork`, and `x86_64/src/proc.rs` imports the ten it uses plus `RFPROC` inside `mod tests`.
- `aarch64/src/proc.rs` reads all twelve flag sites through `port::rfork::RF*`; no `crate::syscall::RF*` path remains.
- Each `sys_rfork` mask is now the module-level `pub(crate) const RFORK_SUPPORTED: u64` with the same eleven names in the same order; the function keeps the 32-bit mask, the three "rfork: bad flags" pair checks and the single "rfork: unsupported flags" rejection.
- `mod rfork_mask_tests` in both arch `syscall.rs` files asserts `RFNOMNT`, `RFCKEY`, `RFROBASE`, `RFRANDMEM` and `RFNOLEGACY` are outside the mask and that the mask is `0x3c7f`; it runs green twice per `cargo xtask test` on the aarch64 crate (default and `virtshell` cfg legs) and is byte-identical in both files.
- `rfork_flags_have_one_definition` in `xtask/src/rforkh.rs` reads the four arch sources with `include_str!` and rejects a redefined flag constant, a `crate::syscall::RF*` path, a function-local mask, a missing or duplicated `RFORK_SUPPORTED` declaration, and any `pub`/`pub(crate)` re-export of a `port::rfork` name.
- `TestStep::run` now calls `xtask::rforkh::check(&workspace())?` first, so a stale `sys/include/rfork.h` fails the configured test command before any cargo leg; the drift proof (mutate, exit 1, restore) is recorded.
- `uses_host_triple_fallback("aarch64" | "x86_64")` widens the host-triple fallback for the arch-crate leg; the comment's former "fallback is aarch64-only" statement is gone and the riscv64 leg keeps its rustup-only selection.

## Task Commits

Each task was committed atomically:

1. **Task 1: RED pins for the rfork masks and the single definition** — `2da371d` (test)
2. **Task 2: import the rfork flags from `port::rfork`; hoist `RFORK_SUPPORTED`** — `37b1681` (kernel)
3. **Task 3: check `rfork.h` in `cargo xtask test`; host-triple fallback on x86_64 hosts** — `53ceb12` (xtask)
4. **Task 3: rfork mask and single-definition pins advanced** — `6dd3f9a` (evidence)

The plan metadata commit follows this summary.

## Files Created/Modified

- `aarch64/src/syscall.rs` — flag block deleted; `use port::rfork::{eleven}`; module-level `RFORK_SUPPORTED`; `rfork_mask_tests` appended
- `aarch64/src/proc.rs` — twelve `crate::syscall::RF*` paths replaced with `port::rfork::RF*`
- `x86_64/src/proc.rs` — flag block deleted; ten-name `use port::rfork::{...}`; `use port::rfork::RFPROC;` inside `mod tests`
- `x86_64/src/syscall.rs` — stale comment and `use proc::{...}` replaced by `use port::rfork::{eleven}`; module-level `RFORK_SUPPORTED`; `rfork_mask_tests` appended
- `xtask/src/rforkh.rs` — `rfork_flags_have_one_definition` plus the four-file `ARCH_SOURCES` table
- `xtask/src/main.rs` — `rforkh::check` first in `TestStep::run`; `uses_host_triple_fallback` and its unit test
- `evidence/PINS.md` — RED, GREEN-1, GREEN-2 and ADVANCED lines for both pins, plus the x86_64 not-run witness line

## Decisions Made

- The two mask-test modules are byte-identical; only the aarch64 copy compiled here, so the identical text is what the owner's x86_64 host will compile (D-24).
- The x86_64 host tests are not claimed green: PINS.md carries `X86_64-HOST-TESTS not-run-on-mac witness=owner-stop` and WINDOWS #5 records the unrun verify.
- `uses_host_triple_fallback` is a pure `matches!` function rather than an inline `||`, so the widening has a host-independent unit test.
- The one-definition test collects every violation into one message; that is why the RED run named both files and all 22 definitions rather than stopping at the first.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected the one-definition test's constant pattern before the RED commit**
- **Found during:** Task 1 (first RED run)
- **Issue:** The first pattern, built as `"const RF" + ": u64 = 1 <<"`, required the name to be exactly `RF`; the run named the `crate::syscall` and mask violations but missed all 22 flag definitions, so the RED evidence did not name `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` as the acceptance criterion requires.
- **Fix:** Replaced the substring test with a small parse: strip an optional `pub`/`pub(crate)`, strip `const `, split at `:`, and require the name to start with `RF` and the tail to start with `u64 = 1 <<`. The pieces are still built from separate strings.
- **Files modified:** `xtask/src/rforkh.rs`
- **Verification:** the re-run names `x86_64/src/proc.rs:171-181` and `aarch64/src/syscall.rs:92-108`; the GREEN run passes.
- **Committed in:** `2da371d` (the RED commit; the fix landed before it)

### Recorded, not auto-fixed

- **The x86_64 test code was not compiled on this Mac.** `x86_64/src/syscall.rs`'s `rfork_mask_tests` and the `RFPROC` import in `x86_64/src/proc.rs` `mod tests` need an x86_64 host (no Rosetta; the nix toolchain ships std only for `aarch64-apple-darwin`). They are textually identical to the aarch64 copies that run here; the first compile is plan 02-06's owner stop.
- **Kernel bytes were not re-measured in this plan.** D-23 accepts the panic-location line shifts; the before/after comparison is plan 02-05's job. This plan changed only constant/import/mask placement and test-only code, and `.text` was not touched.
- **`graphify update .` refreshed a large aggregate graph.** It reported 15,435 nodes (above the 5,000 limit) and rewrote `graphify-out`, which is ignored.

## Issues Encountered

- `cargo xtask test` was red at `2da371d` by design: `error[E0432]: unresolved import super::RFORK_SUPPORTED` in the aarch64 crate, recorded as the rfork-masks RED pin.
- `cargo xtask clippy --arch aarch64` remains red at HEAD with four `undocumented_unsafe_blocks` errors and was not used as a pass criterion, per the plan.

## Known Stubs

None. The x86_64 host-test leg is unrun, not stubbed; it is recorded in PINS.md and WINDOWS #5 with the owner's host as the witness.

## Threat Flags

None beyond the planned surface. T-02-12 (reserved flag accepted) is mitigated by the identical mask tests and the pinned `0x3c7f`; T-02-13 (a second definition) by `rfork_flags_have_one_definition`; T-02-14 (header drift) by the check at the top of `TestStep::run`; T-02-15 (a green test that never ran) by the ledger's `not-run-on-mac` line and the widened fallback.

## Next Phase Readiness

- Plan 02-05 can measure the four kernel configs against `evidence/kernel-bytes/before.sha256`; the arch sources now hold no flag copies, so the source shape matches the research's measured edit.
- Plan 02-06's owner stop must run `nix develop -c cargo xtask test --verbose` on the x86_64 Linux host and look for `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok` and an `Executing` line for package `x86_64`.
- Nothing was pushed; Harvey was read only.

---

*Phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation*
*Completed: 2026-11-24*

## Self-Check: PASSED

All eight listed source/evidence files exist; all four plan commits are ancestors of HEAD.
