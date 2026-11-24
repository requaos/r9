---
phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
reviewed: 2026-11-24T05:04:57Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - aarch64/src/proc.rs
  - aarch64/src/syscall.rs
  - docs/threat-model.md
  - flake.nix
  - port/src/lib.rs
  - port/src/rfork.rs
  - port/tests/rfork_test.rs
  - sys/include/rfork.h
  - x86_64/src/proc.rs
  - x86_64/src/syscall.rs
  - xtask/src/lib.rs
  - xtask/src/main.rs
  - xtask/src/rforkh.rs
  - xtask/tests/threat_model.rs
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
  necessity: 1
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-11-24T05:04:57Z
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

The production-code half of this phase is a clean, behaviour-preserving move. I verified the eleven flag values in both arch crates are byte-for-byte the pre-move values (`git show 1772a373:aarch64/src/syscall.rs` vs `port/src/rfork.rs:22-44`), that both `RFORK_SUPPORTED` masks are the same eleven bits as the old function-local `SUPPORTED` constants and equal `0x3c7f`, that the reserved bits are rejected before any side effect in `sys_rfork` (mask check precedes the non-RFPROC branch and `rfork_clone`), that `flag` is 32-bit-masked at entry so no bit above 31 can reach either path, and that the moved constants have no other consumers left behind (`crate::syscall::RF*` / `proc::RF*` are gone). The committed generated header is byte-identical to what `rforkh::render()` produces (I reproduced the renderer), so the drift check is truthful today. No security defect, no data-loss path, and no BLOCKER was found in the runtime code.

The defects are in the new guard rail and the new documents. The phase's central regression lock — "the sixteen flags have one definition" — lives in `xtask`'s own test target, which no gate runs and no CI job even compiles (WR-01); the lock itself is a narrow one-line text scan that would not notice a redefinition in `port/src` or a wrapped declaration (WR-02). The generated C header cannot be included twice (WR-03, reproduced with clang). And the threat model's contradiction list now misstates the repository it describes, because the phase made the reservation it calls untrue (WR-04). Four smaller items follow.

## Warnings

### WR-01: The one-definition invariant and the threat-model checks run in no gate and are not compiled in CI

**File:** `xtask/src/main.rs:3554-3602`, `.github/workflows/xtask.yml:13-28`
**Issue:** `TestStep::run` builds exactly two kinds of leg — `cargo test --package port --lib --tests` (`main.rs:3561-3572`) and `cargo test --package <host-arch> --bins --target <t>` (`main.rs:3585-3602`). Neither names the `xtask` package, so `rfork_flags_have_one_definition` (`xtask/src/rforkh.rs:189-274`), the four other `rforkh` unit tests, `host_triple_fallback_covers_aarch64_and_x86_64` (`main.rs:5371-5379`) and every test in `xtask/tests/threat_model.rs` execute only when someone types `cargo test -p xtask` by hand. `CheckStep` (`main.rs:3691-3712`) checks only `aarch64`/`riscv64`/`x86_64`/`port`, and `ClippyStep` runs `cargo clippy --workspace` without `--all-targets` (`main.rs:3649-3651`), so the new test code is not even type-checked by CI. The CI `tests` job (`xtask.yml:28`) and the aarch64-host job (`xtask.yml:98`) run `cargo xtask test`, so they inherit the gap.
This matters because the phase's own risk table credits T-02-13 ("a second definition") to `rfork_flags_have_one_definition` (`02-04-SUMMARY.md:191`), and its evidence records the test being run manually (`02-04-SUMMARY.md:65`). The header drift check *is* wired (`main.rs:3554`) and the arch mask test *is* wired through the `--bins` leg, which makes the omitted leg easy to overlook. As written, the invariant is green because nobody runs it.

**Fix:**
```rust
// TestStep::run, after the port leg
all_cmd_args.push((
    vec!["test".to_string(), "--package".to_string(), "xtask".to_string()],
    None,
));
```
Run the new leg once before committing to clear any pre-existing failures in `main.rs`'s 5k-line test module, then record in the phase evidence which CI job now covers the invariant.

### WR-02: The "one definition" invariant scans four hardcoded files and one exact spelling

**File:** `xtask/src/rforkh.rs:175-213`
**Issue:** `ARCH_SOURCES` is a fixed four-entry `include_str!` table (`:177-182`) and check 1 requires the literal single-line shape `const RF<name>: u64 = 1 <<` (`:194-212`, `shift_ty = "u64 = 1 <<"`). Consequences:
- a second definition outside those four files passes silently — including in `port/src/*`, the module that now *owns* the definition (the constant moved into `port` in this phase, so it is the likeliest place for a copy), and in any future `riscv64/src/proc.rs`;
- a definition split across lines (`const RFPROC: u64 =\n    1 << 4;`), or written with another type/alias (`u32`, `0x10`), passes;
- check 2 (`:217-227`) looks for `crate::syscall::RF` only in `aarch64/src/proc.rs`.

The test's name, and the summary's claim that "the four arch sources cannot define or re-export an rfork flag again" (`02-04-SUMMARY.md:15`), therefore overstate what it proves: it proves four files, not one definition.

**Fix:** scan every `.rs` under `port/src`, `aarch64/src`, `x86_64/src` and `riscv64/src` (a small `read_dir` recursion, no new crates), collapse whitespace per logical statement, and match `const` + an `RF*` name + `:` + a type + `=` as separate tokens instead of one literal string. At minimum add `port/src` and name the covered roots in the failure message.

### WR-03: The generated header has no include guard, so a second `#include` is a compile error

**File:** `sys/include/rfork.h:15-34`, generator `xtask/src/rforkh.rs:24-55`
**Issue:** `render()` emits `enum { ... };` with no `#ifndef`/`#define` guard. A translation unit that reaches the header twice — an ordinary outcome once two subsystem headers include it — fails to compile. Reproduced with clang:
```
sys/include/rfork.h:17:2: error: redefinition of enumerator 'RFNAMEG'
  note: './sys/include/rfork.h' included multiple times, additional include site here
```
The generator's comment (`rforkh.rs:21-23`) justifies the omission with "Harvey's `libc.h` has none and this file is not meant to be included beside it", which addresses duplicate enumerators *against libc.h*, not duplicate inclusion of this file. Nothing in the repo compiles the header (the phase's `cc -fsyntax-only` run was a manual step, `02-01-PLAN.md:184`), so the defect is currently latent — and therefore cheap to fix before the first C consumer lands.

**Fix:** emit a guard in `render()`, and update `rforkh_render_pins_harvey_lines`/`rforkh_render_is_stable_and_ascending` (which currently assert the exact body and `ends_with("extern\tint\trfork(int);\n")`):
```rust
out.push_str("#ifndef RFORK_H\n#define RFORK_H\n\n");
// ... enum ...
out.push_str("};\n\nextern\tint\trfork(int);\n\n#endif /* RFORK_H */\n");
```
Add a host check that compiles the header twice in one TU (skip cleanly when no `cc` is present) so the guard cannot regress.

### WR-04: Section 7 now misstates the repository about rfork bits 18-20

**File:** `docs/threat-model.md:177`
**Issue:** Under "Statements about the repository that are not true of it", the bullet says the compendium's claim that "`rfork` bits 18 to 20 are already reserved" is not true of the repository. After this phase the repository reserves bits 18, 19, 20 and 21 in `port/src/rfork.rs:36-43` and publishes them in `sys/include/rfork.h:29-33`, so the *reservation* half is now true in r9 (Harvey is still untouched, `port/src/rfork.rs:3-8`). What remains false is the compendium's stated *home* — Harvey's `sys/include/libc.h` and `x86_64/src/proc.rs` — and `x86_64/src/proc.rs` now defines no rfork flag at all because this phase deleted that block.
The document test suite cannot catch this: `threat_model_rows_and_bullets_are_cited_or_marked` (`xtask/tests/threat_model.rs:195-222`) only checks that the bullet carries a source tag. A reader of the security statement of record for Phases 5-7 is told the opposite of the tree they are reading.

**Fix:** split the two halves and cite the resolution that already exists in `.planning/PROJECT.md:105` (note N3):
```markdown
- **Statements about the repository that are not true of it.** The compendium says `port` forbids
  `unsafe` in every module, which is still not true at HEAD (note N2). It also says `rfork` bits 18
  to 20 are reserved in Harvey's `sys/include/libc.h` and `x86_64/src/proc.rs`; neither is so.
  r9 reserves bits 18-21 itself, in `port/src/rfork.rs`, with Harvey unchanged (note N3). Not
  settled here.  [AC 1.1]
```

## Info

### IN-01: The threat-model test hard-fails on any tree without `.planning/`

**File:** `xtask/tests/threat_model.rs:12-13`, `:31-37`
**Issue:** `read()` panics when `.planning/r9-architecture-and-code-v4.md` is absent. CI checks out the full tree, so today's jobs are unaffected, but any tree built without planning artifacts — a source export, or a PR branch produced by the project's own planning-filtered branch workflow — fails a code test because a planning document is missing. The coupling is deliberate (the document's Sources section makes `.planning/` normative); the hard panic is not.
**Fix:** keep the live comparison but degrade gracefully when the compendium is absent: print `compendium not present; boundary comparison skipped` and return, leaving the document-side assertions intact.

### IN-02: The compendium comparison searches past section 2.2

**File:** `xtask/tests/threat_model.rs:158-164`
**Issue:** `let after = &ac_lines[heading + 1..];` then `find(|l| l.starts_with("* **In Scope:**"))` scans to end of file rather than to the next heading. If section 2.2's boundary lines are moved or removed, any later line with the same prefix satisfies the test while the document no longer mirrors section 2.2 — the exact drift the test exists to catch. The module already has the bounded helper (`between()`, `:67-79`).
**Fix:** truncate `after` at the first `## ` or `### ` line following `heading` before searching.

### IN-03: The re-measured FOD hash has no provenance beside it

**File:** `flake.nix:983` (comment block `:974-982`)
**Issue:** The line now reads `sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`, but the comment directly above still names `sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=` as the value "measured" and says the pin "replaces the 06b94f8-era TIzw value". The hash commit (`4624ea2 boot-iso hash bump`) has an empty body, although `02-05-PLAN.md:194` requires the message to name the commit, the `got` value and the replaced value. The measurement survives only in `evidence/owner-stop/RESULTS.md`, far from the line a maintainer edits when re-deriving this derivation.
**Fix:** extend the comment with the Phase 2 measurement and the replaced value (or point at the evidence file), e.g. `# 2026-11-23 (Phase 2): re-measured at <sha> -> sha256-x07uJKM8...; replaces sha256-b6OcAb2p...; see .planning/phases/02-.../evidence/owner-stop/RESULTS.md`.

### IN-04: `rforkh_mutated_render_differs` is already covered by the staleness test

**File:** `xtask/src/rforkh.rs:169-173`
**Category:** necessity
**Issue:** Subtype: already-defended-elsewhere. The test asserts `render().replace("(1<<21)", "(1<<22)") != render()` — that `str::replace` changes a string. `rforkh_check_rejects_missing_empty_and_stale` (`:162-163`) already writes exactly that mutated text to the scratch header and asserts `check` rejects it, so the mutation's effect on the comparison is covered; the standalone test can only fail if the standard library breaks.
**Fix:** delete the test, or fold its `assert_ne!` into the staleness test as a sanity check on the fixture it mutates.

---

_Reviewed: 2026-11-24T05:04:57Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
