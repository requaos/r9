---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 02
subsystem: gates
tags: [xtask, lint-contract, ds, miri, flake, no_std, pin-ledger, kernel-toolchain]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto/ and ds/ workspace crates plus their xtask test and check legs (@03-01, @03-12)"
provides:
  - "xtask::lintcontract::check — a six-rule static contract gate run before any cargo leg in cargo xtask test"
  - "DS-07 crate-documentation note and the absence check (no Funnel file or module under ds/src)"
  - "xtask::miri and cargo xtask miri [--seeds N], with a named no-tests refusal"
  - "toolchainMiri in flake.nix and the guarded R9_MIRI_TOOLCHAIN devshell export"
  - "evidence/PINS-03-02.md: RED, GREEN-1, GREEN-2 and ADVANCED for lint-contract, ds07-funnel-note and miri-step"
affects: [03-03, 03-04, 03-05, 03-06, 03-07, 03-08, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 13618
  tasks: 2
  commits: 6
  plan_head_before: 100a1daf605f2a2179d4e36eca7831d0edded88d
  plan_head_after: 0522586bc83ddc76ed614129a5efcb5c93119047

tech-stack:
  added:
    - "toolchainMiri — a second rust-overlay toolchain derivation (the four pinned components plus miri), exported as R9_MIRI_TOOLCHAIN"
  patterns:
    - "Static lint contract: one xtask module made of plain text scans, one `lint-contract:` error listing file:line problems, called before any cargo leg"
    - "Injected-lookup resolver: environment access is a closure, so the Miri named error and command shape are unit-testable without Miri installed"
    - "IG-V01: committed RED pin, GREEN twice, pin advance, recorded in evidence/PINS-03-02.md"

key-files:
  created:
    - xtask/src/lintcontract.rs
    - xtask/src/miri.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-02.md
  modified:
    - xtask/src/lib.rs
    - xtask/src/main.rs
    - flake.nix
    - ds/src/lib.rs

key-decisions:
  - "One `lintcontract::check` collects `problems: Vec<String>` across six rule groups and returns a single `lint-contract:` error naming Phase 3 CONTEXT; TestStep::run calls it directly after rforkh::check, so a broken contract stops the run before any cargo leg compiles"
  - "The `unsafe` token rule strips `//` comments and matches whole words only, so `unsafe_code`, `unsafe_op_in_unsafe_fn` and prose in comments never fire; the tests build every forbidden token with concat()"
  - "The DS-07 note occupies `//!` lines only at the top of ds/src/lib.rs; the GREEN commit adds doc lines only and states the locked IG-D09 restriction (write-once ephemeral scratchpads; never the page cache, FidTable or a Venti index)"
  - "Miri is a second derivation `toolchainMiri = toolchain.override { extensions = [... \"miri\"] }` exported as R9_MIRI_TOOLCHAIN; rust-toolchain.toml and the devshell `packages` list are untouched, so the kernel toolchain store path cannot move (RESEARCH C3, Pitfall 2)"
  - "cargo xtask miri runs `<toolchain>/bin/cargo miri test --package ds --lib tpht` with that bin first on PATH and MIRIFLAGS=-Zmiri-many-seeds=0..64, and refuses with `miri: no tests under ds/src/tpht` when the filter would match nothing (T-03-07)"
  - "The devshell appends `$R9_MIRI_TOOLCHAIN/bin` to PATH (never prepends) so cargo resolves `cargo-miri` even when invoked by absolute path, while the pinned cargo and rustc stay first (auto-fixed deviation, Rule 3)"

requirements-completed: [LINT-01, DS-07]

coverage:
  - id: D1
    description: "cargo xtask test runs the static lint contract before any cargo leg; the six rules (Funnel absence, crypto unsafe, ds unsafe and forbids, kernel dependencies, toolchain and cargo-config drift, lints and roaring placement) are fixture-tested"
    requirement: LINT-01
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask lintcontract — test result: ok. 12 passed; 0 failed"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test — rc 0, no `lint-contract:` output"
        status: pass
    human_judgment: false
  - id: D2
    description: "DS-07 is met by absence: no Funnel file or module under ds/src, and ds/src/lib.rs carries the Funnel Hashing restriction note"
    requirement: DS-07
    verification:
      - kind: unit
        ref: "lintcontract::tests::lintcontract_ds_crate_doc_carries_the_funnel_note and lintcontract_rejects_funnel_file_and_mod — ok"
        status: pass
      - kind: other
        ref: "test ! -e ds/src/funnel.rs && grep -c 'Funnel Hashing' ds/src/lib.rs => 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "Miri is supplied as cargo xtask miri through the second toolchain derivation, and refuses when ds/src/tpht/ holds no #[test]"
    requirement: LINT-01
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask miri — test result: ok. 5 passed; 0 failed"
        status: pass
      - kind: other
        ref: "MIRI-VERSION: miri 0.1.0 (842bd5be25 2026-01-29); MIRI-REFUSAL: exit 1, `miri: no tests under ds/src/tpht (the filter would match nothing)`"
        status: pass
    human_judgment: false
  - id: D4
    description: "The kernel toolchain is untouched: rust-toolchain.toml is byte-identical to the Phase 3 baseline, and toolchainMiri is absent from the devshell packages list"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "B=4beb5c8…; git diff --quiet \"$B\" HEAD -- rust-toolchain.toml => rc 0; grep -c toolchainMiri flake.nix => 2"
        status: pass
    human_judgment: false
  - id: D5
    description: "No Funnel Hashing implementation landed under another name anywhere in ds/"
    requirement: DS-07
    verification: []
    human_judgment: true
    rationale: "The plan's backstop: a reviewer confirms the mechanical name/`mod` checks were not evaded by an implementation under a different name; the static check pins the absence but cannot judge intent"

# Metrics
duration: 3min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 02: The gates — static lint contract and Miri step

**`cargo xtask test` now enforces a six-rule static lint contract before any cargo leg, and `cargo xtask miri` runs TPHT under a second Miri toolchain without moving the kernel toolchain; DS-07 is pinned by absence and a crate-doc note.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-11-24T14:28:59Z
- **Completed:** 2026-11-24T14:31:54Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `xtask::lintcontract::check` is the static contract gate: R1 Funnel absence plus the DS-07 note, R2 the `crypto/src` `unsafe` token rule and `#![forbid(unsafe_code)]`, R3 the same rule for `ds/src` outside `tpht/` plus the per-module forbid layout, R4 the kernel-crate dependency ban, R5 the `rust-toolchain.toml`/`.cargo/config.toml` drift checks, and R6 the lint opt-in plus `roaring` placement. `TestStep::run` calls it immediately after `rforkh::check`, so the first failure is `lint-contract:` and no cargo leg compiles.
- Twelve fixture tests cover all six rule groups, including the adjacency cases (`ds/src/tpht_extra.rs` is checked, `ds/src/tpht/anything.rs` is not; `unsafe_code` and `// unsafe` do not match; a case-insensitive Funnel file name and a `mod funnel;` line both fail). Every forbidden token the tests write into a fixture is built with `concat()`.
- The DS-07 note is in `ds/src/lib.rs` crate documentation, and the GREEN commit adds only `//!` lines: Funnel Hashing is not built, and a future implementation serves write-once ephemeral scratchpads only — never the page cache, `FidTable` or a Venti index (IG-D09, locked 2026-11-24).
- `flake.nix` gains `toolchainMiri = toolchain.override { extensions = [ "rustfmt" "rust-src" "clippy" "llvm-tools" "miri" ]; }` in the devshell `let`, exported in guarded form as `R9_MIRI_TOOLCHAIN`. The `packages` list and `rust-toolchain.toml` are untouched; the kernel toolchain store path is unchanged since the baseline.
- `xtask::miri` resolves the toolchain through an injected lookup (empty and whitespace count as unset), counts `#[test]` attributes under `ds/src/tpht/`, builds the exact command through a pure function, and refuses when the count is zero. `cargo xtask miri [--seeds N]` is a clap subcommand with a `MiriStep`; it is not part of `cargo xtask test` (D-13).
- The ledger records RED, GREEN-1, GREEN-2 and ADVANCED for all three pin ids, plus `MIRI-VERSION` and `MIRI-REFUSAL`. `MIRI-VERSION` is `miri 0.1.0 (842bd5be25 2026-01-29)`; `MIRI-REFUSAL` is the named refusal at this commit (tpht has no test until plan 03-06).

## Task Commits

Each task was committed atomically:

1. **Task 1, RED: lint-contract and DS-07 pins** — `32dbd05` (test)
2. **Task 1, GREEN: lint-contract check and the DS-07 note** — `cd85178` (feat)
3. **Task 1, pin advance** — `e925393` (evidence)
4. **Task 2, RED: Miri step pin** — `5804a4b` (test)
5. **Task 2, GREEN: toolchainMiri and cargo xtask miri** — `bdb4f4b` (feat)
6. **Task 2, pin advance** — `0522586` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `xtask/src/lintcontract.rs` — the six-rule contract, its `lint-contract:` error, the helper scanners (`strip_line_comment`, `find_whole_word`, `declared_module_name`, dependency-table parsing) and 12 fixture tests
- `xtask/src/miri.rs` — `TOOLCHAIN_VAR`, `CommandSpec`, `resolve_toolchain`, `tpht_test_count`, `command_spec`, `run`/`run_with_lookup`, and 5 tests
- `xtask/src/lib.rs` — `pub mod lintcontract;` and `pub mod miri;`
- `xtask/src/main.rs` — the `lintcontract::check(&workspace())?` call in `TestStep::run` (D-09/D-10/D-16/D-21 comment); the `miri` clap subcommand, its dispatch arm and `MiriStep`
- `flake.nix` — `toolchainMiri` in the devshell `let` with the "second derivation, never restate pins" rationale; the guarded `R9_MIRI_TOOLCHAIN` export; the append-only PATH line (deviation)
- `ds/src/lib.rs` — the DS-07/IG-D09 restriction note, added as `//!` lines only
- `evidence/PINS-03-02.md` — RED/GREEN-1/GREEN-2/ADVANCED for `lint-contract`, `ds07-funnel-note`, `miri-step`, plus `CHECK-ORDER`, `MIRI-VERSION` and `MIRI-REFUSAL`

## Decisions Made

- The contract is one function collecting all problems rather than failing on the first: a broken tree reports every violation with file and line, and the error names Phase 3 CONTEXT as the rule source.
- `unsafe` is matched as a whole word after stripping `//` comments. This deliberately does not strip `/* */` blocks or string literals: no such construct exists in the two crates today, and a future one is a problem for the rule's owner to classify rather than a silent pass.
- The module-forbid rule parses `ds/src/lib.rs` line by line and accepts the attribute on the same line or the line before; `tpht` is the one module that must not carry it.
- The static checks were written as a runtime gate (`check` called from `TestStep::run`), not an `include_str!` unit test, so a violation in any future file is caught without editing the test's file list; the fixture tests prove the rules themselves.
- `toolchainMiri` uses `toolchain.override { extensions = ... }` and repeats the pinned four components plus `miri`; the "never restate pins" rule is intentionally bent here because adding `miri` to `rust-toolchain.toml` would move the store path embedded in every kernel (RESEARCH C3).
- `xtask::miri::run` passes `PATH` and `MIRIFLAGS` on the spawned command and never reads `CARGO`/`cargo()`: inside `cargo xtask`, `CARGO` points at the devshell cargo, which has no Miri.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The devshell appends `$R9_MIRI_TOOLCHAIN/bin` to PATH so the plan's literal MIRI-VERSION verify can resolve `cargo-miri`**

- **Found during:** Task 2 (Miri supply)
- **Issue:** the plan's `<verify>` command `nix develop -c sh -c '"$R9_MIRI_TOOLCHAIN/bin/cargo" miri --version'` failed with `error: no such command: miri`. Cargo resolves subcommands (`cargo-miri`) through `PATH`, and the devshell's `PATH` held only the pinned toolchain's `bin`. The research command form prepended the Miri `bin` to `PATH`, but the plan's verify line did not.
- **Fix:** the devshell `shellHook` now appends `$R9_MIRI_TOOLCHAIN/bin` to `PATH`. The append (never prepend) keeps the pinned `cargo` and `rustc` first, so kernel builds and every other devshell command still use `toolchainFor`; `xtask::miri::run` continues to prepend the directory for the Miri step itself.
- **Files modified:** `flake.nix`
- **Verification:** the literal verify prints `miri 0.1.0 (842bd5be25 2026-01-29)`; `nix develop -c cargo xtask test` rc 0 with zero `miri` mentions in its output; `git diff --quiet "$B" HEAD -- rust-toolchain.toml` exits 0; `grep -c toolchainMiri flake.nix` is 2.
- **Committed in:** `bdb4f4b` (Task 2 feat commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** The only deviation is the PATH append needed to make the plan's own verify command runnable; it touches no pinned toolchain, no kernel path and no gate rule. No scope creep.

## Issues Encountered

- The literal MIRI-VERSION verify exposed cargo's PATH-based subcommand lookup (see the deviation). The research form had the prefix; the plan's verify line did not. Resolved with the append-only PATH line rather than editing the plan.
- `xtask/src/main.rs` line references in the plan are stale by a few dozen lines at this HEAD; the anchors (`rfork-header` block, dispatch arm, `RforkHeaderStep`, `workspace()`) were located by symbol.

## Known Stubs

None. This plan adds no placeholder values, TODOs or unwired data sources: the RED skeletons were replaced by the GREEN implementations in the same task, and the Miri refusal is a deliberate named guard, not a stub.

## Threat Flags

None beyond the plan's `<threat_model>`: T-03-05 is mitigated by the second derivation plus rule R5 and the unchanged `rust-toolchain.toml`; T-03-06 by R2 to R4 running before every test leg; T-03-07 by the `tpht_test_count` refusal; T-03-08 remains accepted (the variable is the developer's own shell trust, and the default is a nix store path).

## Next Phase Readiness

- Every later plan in Phase 3 adds code under these gates: a forbidden `unsafe`, a kernel dependency, toolchain drift or a Funnel module now fails `cargo xtask test` before compiling.
- `cargo xtask miri` refuses at this commit by design (`ds/src/tpht/` has no `#[test]`); plan 03-06 adds the TPHT tests and the step becomes a real check. `MIRI-VERSION` proves the toolchain is present now.
- The kernel toolchain and `.cargo/config.toml` are unchanged since the Phase 3 baseline, so D-21's stop condition is not met and the phase continues.
- Plan 03-03 runs next without waiting.

## Self-Check: PASSED

All seven claimed files exist on disk and all six plan commits are ancestors of HEAD. `cargo test -p xtask lintcontract` (12 passed) and `cargo test -p xtask miri` (5 passed) ran at the final state; `cargo xtask test` exited 0; `rust-toolchain.toml` equals the baseline; the ledger holds RED, GREEN-1, GREEN-2 and ADVANCED for all three pins.
