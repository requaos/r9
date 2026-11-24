---
phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
verified: 2026-11-24T05:10:00Z
status: passed
score: 41/42 must-haves verified
covered_files:
  - .planning/PROJECT.md
  - .planning/REQUIREMENTS.md
  - .planning/ROADMAP.md
  - .planning/STATE.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-01-PLAN.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-01-SUMMARY.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-02-PLAN.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-02-SUMMARY.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-03-PLAN.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-03-SUMMARY.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-04-PLAN.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-04-SUMMARY.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-05-PLAN.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-05-SUMMARY.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-06-PLAN.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-06-SUMMARY.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-DECISION-TABLE.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-OWNER-STOP.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/PINS-threat-model.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/PINS.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/aarch64-quick-pass.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/kernel-bytes/KERNEL-BYTES.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/kernel-bytes/after.sha256
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/kernel-bytes/before.sha256
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/RESULTS.md
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2/boot-iso.log
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2/gate-full.log
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2/gate-kvm.log
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2/gate-tcg.log
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2/host-tests.log
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2/kernel-step.log
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2/venue.log
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/pending-rfork.patch
  - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/threat-model-draft.md
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
covered_digest: "v4:sha256:79b17999c8bc4275b884919ca85df4b8021bf852db014991c978a2c8d4ab0ab9"
behavior_unverified: 0
overrides_applied: 0
decision_coverage:
  honored: 29
  total: 29
  not_honored: []
human_verification:
  - test: "Confirm whether the x86_64 host tests ran before the `boot-iso` `outputHash` commit `4624ea2` (D-29). The only returned host-test log is the round-2 log at `HEAD=4624ea2` (the hash commit itself), generated after the commit; round 1 returned no host-test log at all."
    expected: "The owner either attests that a pre-commit host-test run happened in round 1 (and, if available, produces that log), or accepts the deviation. If accepted: the hash remains valid because the round-2 host tests passed at the gated commit and no compile fix was needed; record the acceptance as an override or a new dated decision. No code change is required."
    why_human: "The ordering is a human process step, not code. No test exercises it, and the returned logs cannot prove a run that was not returned. The hash commit timestamp (2026-11-23T22:22:53-05:00 / 03:22:53Z) precedes the round-2 return (2026-11-24T04:48:35Z), and the host-test log's `HEAD=` is exactly the hash commit, so the only recorded run is after the commit."
---

# Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation Verification Report

**Phase Goal:** The design decisions that Phases 3 to 7 build on are confirmed by the maintainer, and the four `rfork` flag bits the crypto work needs are reserved in r9, with no change to runtime behaviour.

**Verified:** 2026-11-24T05:10:00Z
**Status:** human_needed (phase goal achieved; one process-order must-have needs owner confirmation/acceptance)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

#### Plan 02-01 — one flag definition and a generated C header

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | One `port` module, `port/src/rfork.rs`, defines sixteen `rfork` flags as `pub const ... : u64`: the eleven existing ones, `RFNOMNT` 1<<14, and `RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21 | ✓ VERIFIED | `port/src/rfork.rs:14-47`: the `rfork_flags!` table lists all sixteen with those exact shifts, plus `ALL` and `FIRST_R9_ONLY = 12` |
| 2 | `cargo test -p port --test rfork_test` passes three tests that pin the four reserved values, pin the twelve Harvey values literally and in order, and prove all sixteen flags are disjoint single bits | ✓ VERIFIED | Ran it: `3 passed; 0 failed`; the three tests are `rfork_reserved_values_are_pinned`, `rfork_harvey_values_are_pinned`, `rfork_flags_do_not_overlap` |
| 3 | Every one of the sixteen flags is a distinct single bit; the union of all sixteen is exactly `0x3c7c7f` | ✓ VERIFIED | `rfork_flags_do_not_overlap` asserts `is_power_of_two`, no overlap, and `seen == 0x3c7c7f`; the bits are 0-6, 10-14, 18-21 |
| 4 | `cargo xtask rfork-header --check` exits 1 for a missing, empty or one-character-stale header and 0 for the committed `sys/include/rfork.h` | ✓ VERIFIED | Ran the check: `rfork-header: sys/include/rfork.h matches port/src/rfork.rs`; `rforkh_check_rejects_missing_empty_and_stale` exercises all three negative controls and the restore |
| 5 | The header lists the sixteen enumerators in ascending bit order, the rendering is stable, and the first twelve enumerator lines are pinned literally in Harvey's order | ✓ VERIFIED | `rforkh_render_pins_harvey_lines` pins all twelve Harvey lines and the four reserved lines; `rforkh_render_is_stable_and_ascending` asserts two renders are equal and shifts strictly increase; the committed header is in that order |
| 6 | `sys/include/rfork.h` is generated from the Rust constants, holds the `RF*` enum and `extern int rfork(int);` only, names the Harvey `libc.h` lines it mirrors, and passes a C syntax check | ✓ VERIFIED | The file holds only the banner, the enum and `extern	int	rfork(int);`; the banner names `libc.h:613-624` and `:684`; `clang -fsyntax-only -Wno-unused-command-line-argument -std=c99 -x c sys/include/rfork.h` exits 0; `--check` proves it is the renderer output |
| 7 | The `port/src` change was shown to the owner as a diff and approved before commit; it crossed the owner stop as the committed patch `evidence/pending-rfork.patch`; the committed change is byte-equal to that patch | ✓ VERIFIED | `PINS.md` has `APPROVED rfork-patch sha256 998ba6a0…`; the patch file is committed before the applied commits (`abed9b8` then `b139419`/`01922de`); `git diff --full-index abed9b8 01922de -- port/src sys/include xtask/src` is byte-for-byte equal to the patch (`cmp` exits 0) |
| 8 | Every task ends with a clean tree and everything committed; the plan gives the same result in a worktree and on the main tree; a continuation agent that cannot prove it holds the approved patch commits nothing | ✓ VERIFIED | The six task commits exist and are ancestors of HEAD; the working tree has only pre-existing untracked GSD paths; the patch ledger plus the byte-equality check are the hold condition (summary and `PINS.md`); no uncommitted phase work exists |
| 9 | Whole-ELF, `.text` and (aarch64) `.text.boot` sha256 values are recorded from the commit before the first kernel-affecting commit | ✓ VERIFIED | `before.sha256` is from `c002feb`, the parent of the first phase commit `7ca2086` and an ancestor of the first kernel-affecting commit `b139419`; ten hash lines plus base/venue |

#### Plan 02-02 — the threat model document

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 10 | `docs/threat-model.md` is committed and states the in-scope protections and out-of-scope attacks with the two AC 2.2 sentences word for word (SEV-SNP or TDX as the remedy) | ✓ VERIFIED | `docs/threat-model.md:31` and `:33` quote the AC lines; `threat_model_quotes_the_ac_boundary` compares them to `.planning/r9-architecture-and-code-v4.md` at run time; the test passes 6/6 and asserts `SEV-SNP` and `TDX` |
| 11 | The document is a full analysis with an asset list, attacker classes with capabilities, trust boundaries and a protection table, not a restatement of AC 2.2 | ✓ VERIFIED | Sections 2-5: 18 assets, 13 attacker classes, 13 trust boundaries, 25 protection rows; `threat_model_has_required_sections` passes |
| 12 | Every table row and bullet carries a source tag or the mark `**[new]**` | ✓ VERIFIED | `threat_model_rows_and_bullets_are_cited_or_marked` scans sections 2-7 and fails on a bare row/bullet; the test passes |
| 13 | The document says it is a living document, that nothing is fixed, and that no protection it describes is implemented; every protection row has the status `not implemented` | ✓ VERIFIED | The `**Status.**` paragraph says all three; `threat_model_protections_are_not_implemented` requires at least 10 rows and every row to contain `not implemented`; `threat_model_makes_no_lock_claim` rejects the word `locked`; all pass |
| 14 | The owner reviewed the draft at a checkpoint before `docs/threat-model.md` was committed; the draft crossed the owner stop as the committed file `evidence/threat-model-draft.md`; the committed document is byte-equal to the draft the owner last saw | ✓ VERIFIED | `PINS-threat-model.md` records the owner reply on draft `c6896d5…`: "ensure all the contradictions a listed as open questions and then approve"; the revised record `a8f28c35…` was committed, and `cmp` shows `docs/threat-model.md` is byte-identical to the committed draft (`a8f28c35…`). Caveat: the reply approved the four-line revision conditionally in one round; the owner did not re-review the rendered revision, but the diff is exactly the requested contradiction tracking (four lines plus the sources line; `git diff 0dd9a2d ed54dc5` confirms) |
| 15 | `cargo test -p xtask threat_model` passes and fails when the boundary sentences, a citation mark or the status statement is removed | ✓ VERIFIED | Ran it: `6 passed; 0 failed`; the tests assert the exact boundary lines, the citation rule, the status sentence and every row's `not implemented` status; each assertion is on the document content, so removing any of them fails the corresponding test |

#### Plan 02-03 — the design lock

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 16 | The owner confirmed the status of all thirteen guide decisions (IG-D01 to IG-D13) at one checkpoint, from one table, ruling individually only where the owner chose to | ✓ VERIFIED | `02-DECISION-TABLE.md` has the thirteen-row proposal and `## Owner ruling` (2026-11-23): `confirm`, plus `Landing-site question: yes`; the table is the single checkpoint |
| 17 | PROJECT.md Key Decisions has exactly one row per decision IG-D01 to IG-D13, and each outcome says locked, locked amended, or not locked and deferred to a named phase | ✓ VERIFIED | `grep -c` of IG-D rows is 13; outcomes: 8 `Locked 2026-11-23 (owner)`, 2 `Locked amended 2026-11-23 (owner)`, 1 each deferred to Phase 3, Phase 4 and Phase 6 |
| 18 | IG-D01, 02, 04, 05, 06, 08, 10, 12 locked; IG-D03 locked with four flag bits; IG-D07 locked amended to the per-module forbid with three named modules | ✓ VERIFIED | PROJECT.md rows 125-137: D03 row says four flag bits; D07 row says per-module `forbid(unsafe_code)` with `globalalloc`, `mcslock`, `devip_waker`; the other eight rows are `Locked` |
| 19 | IG-D09, IG-D11 and IG-D13 are not locked and deferred to Phases 3, 4 and 6; W1 to W10 are not settled by this plan | ✓ VERIFIED | PROJECT.md rows 127, 129, 138: `Not locked; deferred to Phase 3 (W5, W6)`, `Phase 4 (W1-W4)`, `Phase 6 (W7)`; the decisions table leaves W1-W10 open |
| 20 | PROJECT.md states what locked means: owner-only change, work stops and the owner is asked if a locked decision looks wrong, and a change is a new dated row, never a silent edit | ✓ VERIFIED | `PROJECT.md:117`: the lead-in states owner-only change, stop-and-ask, new dated row, and the landing-site limit |
| 21 | ROADMAP Phase 2 success criterion 2, DSGN-03, PROJECT.md note N3 and the `RFNOLEGACY` row, and the STATE.md decision line all say the flags live in one `port/` module, `port/src/rfork.rs`, imported by both arch crates | ✓ VERIFIED | ROADMAP line 147 and open-questions line 167; REQUIREMENTS line 27; PROJECT lines 105 and 126; STATE line 76; all name `port/src/rfork.rs` and the both-arch import |

#### Plan 02-04 — both arch crates import from `port`; masks hoisted and pinned

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 22 | No `rfork` flag constant is defined in any arch crate; `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` import the eleven flags they use from `port::rfork`; `aarch64/src/proc.rs` reads them through `port::rfork`; no re-export | ✓ VERIFIED | `grep` finds no `const RF*: u64 = 1 <<` in either arch crate; `x86_64/src/proc.rs:56` and `x86_64/src/syscall.rs:91`, `aarch64/src/syscall.rs` import eleven names; `aarch64/src/proc.rs` has twelve `port::rfork::RF*` reads; the xtask one-definition test passes and no `pub use port::rfork` exists |
| 23 | Each arch `syscall.rs` has a module-level `pub(crate) const RFORK_SUPPORTED: u64` holding the same eleven flags, value `0x3c7f`, and `sys_rfork` rejects any other bit with "rfork: unsupported flags" | ✓ VERIFIED | Both files declare it once; the mask is `RFPROC|RFMEM|RFFDG|RFNOWAIT|RFREND|RFNOTEG|RFNAMEG|RFCNAMEG|RFENVG|RFCENVG|RFCFDG` = `0x3c7f`; each `sys_rfork` has `if flag & !RFORK_SUPPORTED != 0 { return err(proc, "rfork: unsupported flags"); }` after the 32-bit mask |
| 24 | A host test in each arch crate asserts `RFNOMNT`, `RFCKEY`, `RFROBASE`, `RFRANDMEM` and `RFNOLEGACY` are outside `RFORK_SUPPORTED` and the mask equals `0x3c7f`; the aarch64 test passes on the Mac under both cfg legs | ✓ VERIFIED | Both `rfork_mask_tests` modules assert all five bits zero against the mask and `RFORK_SUPPORTED == 0x3c7f`; ran the aarch64 leg directly (1 passed) and `cargo xtask test` printed the mask test `ok` twice (default and virtshell cfg legs) |
| 25 | An xtask test fails if any arch source file defines an `rfork` flag constant again or re-exports one | ✓ VERIFIED | `rfork_flags_have_one_definition` scans `x86_64/src/{proc,syscall}.rs` and `aarch64/src/{syscall,proc}.rs` with `include_str!`, rejects a flag-constant shape, a `crate::syscall::RF*` path, a function-local mask, a missing/duplicate mask and a re-export; it passes 5/5. Scope caveat: the scan is the four arch files that hold rfork code, not every `.rs` in both crates (code-review WR-02, warning below) |
| 26 | `cargo xtask test` fails before any cargo leg when `sys/include/rfork.h` is stale | ✓ VERIFIED | `xtask/src/main.rs:3556` is `xtask::rforkh::check(&workspace())?;` as the first statement of `TestStep::run`; the unit test proves the check rejects stale/missing/empty, and the committed header passes |
| 27 | `cargo xtask test` selects the host triple for the arch-crate leg on x86_64 hosts as well as aarch64 hosts when rustup reports no target | ✓ VERIFIED | `uses_host_triple_fallback` matches `aarch64` and `x86_64`; the unit test passes; the owner's `host-tests.log` shows the `--package x86_64 --bins --target x86_64-unknown-linux-gnu` leg |

#### Plan 02-05 — Mac-side closing evidence

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 28 | Before and after sha256 of the whole ELF and of `.text` are recorded for `q35churnshell`, `q35cpu`, `virtshell` and `virtcpu`, plus `.text.boot` for the two aarch64 configs, all built on one machine | ✓ VERIFIED | `before.sha256` and `after.sha256` each have ten hash lines; both name `venue Darwin arm64`; `KERNEL-BYTES.md` table repeats them |
| 29 | `.text` (and `.text.boot` on aarch64) is byte-identical before and after for all four configs; a difference stops the phase | ✓ VERIFIED | Six `VERDICT text-*: IDENTICAL` and `VERDICT textboot-*: IDENTICAL` lines; the before/after values match exactly; no DIFFERENT text verdict |
| 30 | The aarch64 quick pass is green on the Mac after the change: tier-1 on HVF and on TCG, and smoke, net-smoke and cpu once on HVF | ✓ VERIFIED | `aarch64-quick-pass.md` records five PASS verdicts with exit 0 and `ALL STAGES PASS`; the five scratch logs under `target/phase02-quick-pass/` each contain the pass line |
| 31 | `cargo xtask test`, `cargo test -p xtask`, `cargo xtask check` and `cargo xtask clippy` are green on the Mac at the last code commit | ✓ VERIFIED | The four logs under `target/phase02-quick-pass/mac-suite/` show completed runs; I independently re-ran `cargo test -p xtask` (96 tests plus 6 document tests) and the full `cargo xtask test` (green, mask test twice); the phase record gives exit 0 for all four |
| 32 | An ordered owner checklist for the single end-of-phase stop is committed, with the exact `flake.nix` line to change and the commit message for the `boot-iso` `outputHash` commit | ✓ VERIFIED | `02-OWNER-STOP.md` has sections 1-8, names `flake.nix` line 983 with the `b6OcAb…` text, and supplies the commit message (no attribution); section 4 is the hash step |
| 33 | The checklist runs the x86_64 host tests before the `outputHash` step, and makes the hash a measurement of the gated commit: the hash build, the witness `kernel` step and every gate run at one commit, each log names that commit, and any later commit voids the round | ✓ VERIFIED | Checklist sections 3 (host tests) then 4 (hash), the one-commit rule and the `run_logged` wrapper (`HEAD=`/`EXIT=`); section 4 states the gated commit and the void-and-repeat rule |

#### Plan 02-06 — the end-of-phase owner stop

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 34 | The phase's unpushed commits were shown to the owner and pushed to the branch `phase-02/design-lock` only after approval; nothing was pushed before this stop and `main` was not pushed at it | ✓ VERIFIED | `origin/phase-02/design-lock` is `f8d077d`, based on the Mac tip `2b41588`; the round records show `ls-remote` empty before approval and `origin/main` unchanged at `af46615`; the branch push is `2b41588` |
| 35 | The owner made the `boot-iso` `outputHash` commit on the x86_64 Linux host as its own commit, before the gate run, and pushed it with the branch; the plain build and `--rebuild` both passed at that commit | ✓ VERIFIED | `4624ea2` is `flake.nix | 2 +-` (one line); its parent is the Mac tip `2b41588`; it is on the branch and an ancestor of origin/main; `kernel-step.log` at `HEAD=4624ea2` has `KERNEL-BOOTISO-BUILD-OK` and `KERNEL-BOOTISO-REBUILD-OK` |
| 36 | The committed `outputHash` is a measurement of the gated commit: build-ok, rebuild-ok, the KVM gate, the TCG gate and the full set all come from logs whose `HEAD=` is one commit, and that commit is the tip of the code `main` receives; a result at another commit is FAIL | ✓ VERIFIED | All six logs start with `HEAD=4624ea2` and end `EXIT=0`; `git diff 4624ea2 origin/main -- . ':(exclude)…/evidence/owner-stop'` is empty; `4624ea2` is an ancestor of `origin/main` (`3d01f07`) |
| 37 | The x86_64 host tests ran on the owner's host before the `outputHash` build and commit (D-29) | ? UNCERTAIN | The only returned host-test log is the round-2 log with `HEAD=4624ea2` and `EXIT=0`, i.e. a run at the hash commit itself; the hash commit is `2026-11-23T22:22:53-05:00` (03:22:53Z) and the round-2 return is `2026-11-24T04:48:35Z`; round 1 returned no host-test log. The log proves the test passed at the gated commit but not that it ran before the commit. Owner confirmation or acceptance is required — see Human Verification |
| 38 | The merge into `main` and the push of `main` were made from the main working tree with `main` checked out; in any other execution mode the plan stopped without merging or pushing | ✓ VERIFIED | Current branch is `main`; `git rev-parse --git-dir` equals `--git-common-dir`; the reflog shows the fast-forward `a0454ff merge origin/phase-02/design-lock` and the later `3d01f07` commit; no worktree/agent merge path was used |
| 39 | The x86_64 host test `syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported` printed `ok` on the owner's x86_64 Linux host | ✓ VERIFIED | `return/round2/host-tests.log:831`: `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok`; the preceding `Executing` line names `--package x86_64 --bins --target x86_64-unknown-linux-gnu` |
| 40 | The boot-to-rio quick gate passed once under KVM, called directly; the quick gate passed under TCG; the full gate set passed | ✓ VERIFIED | `gate-kvm.log` has exactly one `Running target/debug/xtask gate --arch x86-64 --accel kvm` and one `GATE PASS quick x86_64 run=765355-…`; `gate-tcg.log` has `GATE PASS quick`; `gate-full.log` has `GATE PASS full x86_64-linux run=785121-…` and `full.tsv` has eight apps `PASS` |
| 41 | `main` was pushed only after every gate was green and the owner approved again | ✓ VERIFIED | `origin/main` is `3d01f07`, which contains the gated `4624ea2` and the round-2 evidence; `RESULTS.md` records `VERDICT stop-main-push: PASS 3d01f071…`; the summary records the second approval |
| 42 | (backstop) The hash commit, the x86_64 host tests, the KVM quick gate, the TCG gate and the full gate set all pass on the owner's x86_64 Linux KVM host | ✓ VERIFIED | `venue.log`: `venue: Linux x86_64`; six returned logs with `HEAD=4624ea2`/`EXIT=0`; full gate eight of eight PASS; this is directly observed behaviour, not an inference |

**Score:** 41/42 truths verified (1 uncertain). No `⚠️ PRESENT_BEHAVIOR_UNVERIFIED` code truths.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `port/src/rfork.rs` | The one definition of sixteen flags and `ALL` | ✓ EXISTS + SUBSTANTIVE | 47 lines, `macro_rules! rfork_flags`, sixteen constants, `ALL`, `FIRST_R9_ONLY` |
| `port/tests/rfork_test.rs` | Value, Harvey-prefix and overlap pins | ✓ EXISTS + SUBSTANTIVE | 57 lines, three value/overlap tests, `0x3c7c7f` union |
| `xtask/src/rforkh.rs` | render, write, check and the one-definition test | ✓ EXISTS + SUBSTANTIVE | 275 lines, `#[path]` compile of the same file, four render/check tests and `rfork_flags_have_one_definition` |
| `sys/include/rfork.h` | Generated C statement of the rfork interface | ✓ EXISTS + SUBSTANTIVE | 36 lines: banner, sixteen enumerators ascending, `extern	int	rfork(int);`; `--check` matches |
| `evidence/kernel-bytes/before.sha256` | Before hashes for four configs | ✓ EXISTS + SUBSTANTIVE | base `c002feb`, venue `Darwin arm64`, ten hashes |
| `evidence/pending-rfork.patch` | The six-file change shown to the owner | ✓ EXISTS + SUBSTANTIVE | sha256 `998ba6a0…`; byte-equal to the committed range with `--full-index` |
| `docs/threat-model.md` | The threat model document | ✓ EXISTS + SUBSTANTIVE | 203 lines, sections 1-7 plus Sources, 25 protection rows |
| `xtask/tests/threat_model.rs` | Six document checks | ✓ EXISTS + SUBSTANTIVE | 258 lines; all six pass |
| `evidence/threat-model-draft.md` | The draft record | ✓ EXISTS + SUBSTANTIVE | sha256 `a8f28c35…`; `cmp` equal to `docs/threat-model.md` |
| `.planning/PROJECT.md` | Key Decisions rows, lock rule, amended N3/Safety | ✓ EXISTS + SUBSTANTIVE | 13 IG-D rows, 8/2/3 outcomes, lock lead-in, amended Safety bullet |
| `02-DECISION-TABLE.md` | Thirteen-decision table and owner ruling | ✓ EXISTS + SUBSTANTIVE | 35 lines; `## Owner ruling` 2026-11-23 `confirm`; landing-site `yes` |
| `.planning/ROADMAP.md` | Phase 2 SC2 amended per D-17 | ✓ EXISTS + SUBSTANTIVE | line 147 names `port/src/rfork.rs`; open-questions line 167 matches |
| `.planning/REQUIREMENTS.md` | DSGN-03 amended per D-17 | ✓ EXISTS + SUBSTANTIVE | line 27 names `port/src/rfork.rs`; five DSGN requirements present |
| `x86_64/src/syscall.rs` | Module-level `RFORK_SUPPORTED`; mask test | ✓ EXISTS + SUBSTANTIVE | `pub(crate) const RFORK_SUPPORTED: u64` at line 95; `rfork_mask_tests` |
| `aarch64/src/syscall.rs` | Module-level `RFORK_SUPPORTED`; mask test | ✓ EXISTS + SUBSTANTIVE | same declaration and test; runs twice on the Mac |
| `xtask/src/main.rs` | Header check first; host-triple fallback | ✓ EXISTS + SUBSTANTIVE | `rforkh::check` at line 3556; `uses_host_triple_fallback` at line 396 |
| `evidence/kernel-bytes/KERNEL-BYTES.md` | Before/after table and verdicts | ✓ EXISTS + SUBSTANTIVE | 64 lines; six IDENTICAL text/textboot verdicts; stripped-ELF differential |
| `evidence/aarch64-quick-pass.md` | Five quick-pass verdicts | ✓ EXISTS + SUBSTANTIVE | five PASS verdicts, four-command Mac suite PASS |
| `02-OWNER-STOP.md` | Ordered owner checklist | ✓ EXISTS + SUBSTANTIVE | sections 1-8, `flake.nix:983`, commit message, one-commit rule |
| `evidence/owner-stop/RESULTS.md` | Nine stop verdicts | ✓ EXISTS + SUBSTANTIVE | nine `VERDICT stop-*: PASS`; `stop-main-push: PASS 3d01f07…` |
| `flake.nix` | Re-derived `boot-iso` `outputHash` | ✓ EXISTS + SUBSTANTIVE | line 983 `sha256-x07uJKM8…`; hash commit `4624ea2` one line |
| `evidence/owner-stop/return/paste.txt` | Paste route output | — N/A | The owner used the branch/commit route, not the paste route (plan allowed either); no file expected |

**Artifacts:** 21 verified, 1 not applicable.

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `xtask/src/rforkh.rs` | `port/src/rfork.rs` | `#[path = "../../port/src/rfork.rs"] mod flags;` | ✓ WIRED | Line 15 compiles the same file the kernels compile |
| `xtask/src/main.rs` | `xtask/src/rforkh.rs` | `rfork-header` subcommand calls `write`/`check` | ✓ WIRED | Clap block line 207; dispatch line 315; `RforkHeaderStep` lines 4765-4784 |
| `port/src/lib.rs` | `port/src/rfork.rs` | `#[forbid(unsafe_code)] pub mod rfork;` | ✓ WIRED | Lines 101-102 |
| `evidence/pending-rfork.patch` | `port/src/rfork.rs` | Task 4 applies the patch and commits on byte equality | ✓ WIRED | `git diff --full-index abed9b8 01922de -- port/src sys/include xtask/src` equals the patch |
| `xtask/tests/threat_model.rs` | `docs/threat-model.md` | Reads `CARGO_MANIFEST_DIR/../docs/threat-model.md` | ✓ WIRED | `DOC_REL` line 12; read at run time; six tests green |
| `xtask/tests/threat_model.rs` | `.planning/r9-architecture-and-code-v4.md` | Reads section 2.2 and requires it verbatim | ✓ WIRED | `AC_REL` line 13; `threat_model_quotes_the_ac_boundary` green |
| `evidence/threat-model-draft.md` | `docs/threat-model.md` | Task 3 copies the committed draft and checks `cmp` | ✓ WIRED | Both files sha256 `a8f28c35…`; `cmp` exits 0 |
| `02-DECISION-TABLE.md` | `.planning/PROJECT.md` | Each confirmed table line becomes one Key Decisions row | ✓ WIRED | Thirteen rows, same 8/2/3 outcomes; `grep -c` is 13 |
| `evidence/kernel-bytes/KERNEL-BYTES.md` | `evidence/kernel-bytes/before.sha256` | Each verdict compares the before/after line of the same config/section | ✓ WIRED | Six text/textboot pairs equal; values match `after.sha256` |
| `02-OWNER-STOP.md` | `flake.nix` | Names line 983 as the one line the owner changes | ✓ WIRED | `flake.nix:983` is the `outputHash`; the checklist quotes the old value |
| `x86_64/src/syscall.rs` | `port/src/rfork.rs` | `use port::rfork::{...}` feeding `RFORK_SUPPORTED` | ✓ WIRED | Line 91 import; mask lines 95-105 |
| `aarch64/src/syscall.rs` | `port/src/rfork.rs` | `use port::rfork::{...}` feeding `RFORK_SUPPORTED` | ✓ WIRED | Import and mask lines 88-100; `aarch64/src/proc.rs` uses `port::rfork::RF*` |
| `xtask/src/main.rs` | `xtask/src/rforkh.rs` | `xtask::rforkh::check(&workspace())?` first in `TestStep::run` | ✓ WIRED | Line 3556 |
| `evidence/owner-stop/RESULTS.md` | `flake.nix` | Records the hash commit and the committed value | ✓ WIRED | `stop-bootiso-hash-commit` quotes `sha256-x07uJKM8…`; `flake.nix` has that value |
| `02-OWNER-STOP.md` | `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` | The `kernel` and `full` steps use `R9_WITNESS_STEPS` | ✓ WIRED | Script exists (39 KB); sections 4 and 7 set `R9_WITNESS_STEPS`; returned logs are from that driver |

**Wiring:** 15/15 connections verified.

### Data-Flow Trace (Level 4)

The phase's artifacts are compile-time constants, a generated header and a document; there is no dynamic rendering path. The relevant chains are:

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `port::rfork` constants | `pub const RF*: u64` | `port/src/rfork.rs` `rfork_flags!` table | Yes — sixteen literal shifts, `ALL` table, used by both arch masks | ✓ FLOWING |
| `sys/include/rfork.h` | enum values and `extern int rfork(int);` | `rforkh::render()` from the same `port/src/rfork.rs` | Yes — `--check` matches the renderer; C syntax check passes | ✓ FLOWING |
| `docs/threat-model.md` | boundary sentences, rows and statuses | The document body; tests read it and the AC source at run time | Yes — six document tests pass against the real files | ✓ FLOWING |
| owner stop logs | build/gate outcomes | `evidence/owner-stop/return/round2/*.log` | Yes — real command output with `HEAD=`/`EXIT=`; eight of eight apps PASS | ✓ FLOWING |

**Data flow:** 4/4 traced; no static fallback or hollow prop.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| The four values, twelve Harvey values and disjoint union are pinned | `nix develop -c cargo test -p port --test rfork_test` | `3 passed; 0 failed` | ✓ PASS |
| The generated header matches the Rust constants | `nix develop -c cargo xtask rfork-header --check` | `rfork-header: sys/include/rfork.h matches port/src/rfork.rs` | ✓ PASS |
| The threat-model document checks pass | `nix develop -c cargo test -p xtask threat_model` | `6 passed; 0 failed` | ✓ PASS |
| The renderer, stale/missing/empty controls and one-definition invariant pass | `nix develop -c cargo test -p xtask rfork` | `5 passed; 0 failed` (`rfork_flags_have_one_definition` included) | ✓ PASS |
| The aarch64 host mask test runs under both cfg legs | `nix develop -c cargo xtask test` | `rfork_reserved_bits_are_unsupported ... ok` twice; full run green | ✓ PASS |
| The committed header is valid C | `clang -fsyntax-only -Wno-unused-command-line-argument -std=c99 -x c sys/include/rfork.h` (devshell flags neutralised) | exit 0 | ✓ PASS |
| The approved patch equals the committed change | `git diff --full-index abed9b8 01922de -- port/src sys/include xtask/src \| cmp - evidence/pending-rfork.patch` | `cmp` exit 0 | ✓ PASS |
| The threat-model draft equals the committed document | `cmp evidence/threat-model-draft.md docs/threat-model.md` | exit 0 | ✓ PASS |
| The gated commit is the code `main` receives | `git diff 4624ea2 origin/main -- . ':(exclude)…/evidence/owner-stop'` | empty | ✓ PASS |
| The KVM quick gate ran once, directly | `grep -c 'Running target/debug/xtask gate' gate-kvm.log`; `grep -c 'GATE PASS' gate-kvm.log` | `1`; `1` | ✓ PASS |

### Probe Execution

No probe-based verification is declared or present for this phase. `.planning/phases/02-…/` has no `scripts/*/tests/probe-*.sh` references and the plans' `<verify>` blocks use cargo tests, evidence checks and the owner logs. **Probe execution:** N/A.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| DSGN-01 | 02-02 | Threat model document states in-scope protections and out-of-scope attacks, with SEV-SNP/TDX as the remedy | ✓ SATISFIED | `docs/threat-model.md` committed; boundary test passes; 6/6 document tests; SEV-SNP/TDX present |
| DSGN-02 | 02-01 | Corrected C prototypes committed inside r9, landing location decided against HEAD | ✓ SATISFIED | `sys/include/rfork.h` (D-10 landing, D-11 contents, D-12 generated) is committed, generated, `--check` clean and valid C; no Harvey edit |
| DSGN-03 | 02-01, 02-03, 02-04 | Four flags (`RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21) in one `port/` module imported by both arch crates; Harvey unchanged | ✓ SATISFIED | `port/src/rfork.rs`; no arch definitions; both arch imports; 3/3 port tests; 5/5 xtask tests; D-17 wording at six sites |
| DSGN-04 | 02-01, 02-04, 02-05, 02-06 | Host test pins the four values and proves no overlap; `rfork` behaviour unchanged | ✓ SATISFIED | Three port pins; two arch mask tests; `RFORK_SUPPORTED` 0x3c7f; six `.text`/`.text.boot` IDENTICAL verdicts; owner x86_64 mask test `ok`; nine stop verdicts PASS |
| DSGN-05 | 02-03 | Maintainer confirms IG-D01 to IG-D13; PROJECT.md Key Decisions records which are locked | ✓ SATISFIED | `02-DECISION-TABLE.md` owner ruling `confirm`; PROJECT.md 13 rows (8 locked, 2 locked amended, 3 deferred); lock rule recorded |

**Coverage:** 5/5 requirements satisfied. No orphaned requirement IDs: every DSGN ID appears in at least one plan's `requirements` field (DSGN-01 in 02-02; DSGN-02/03/04 in 02-01; DSGN-03/05 in 02-03; DSGN-03/04 in 02-04; DSGN-04 in 02-05/02-06).

**Bookkeeping note:** `REQUIREMENTS.md` still has `[ ]` and `Pending` for DSGN-02, DSGN-03 and DSGN-05. Those are updated by `phase.complete`, not by the plans; the artifact evidence above is the substantive check. DSGN-01 and DSGN-04 are already checked.

### Decision Coverage

`gsd_run query check.decision-coverage-verify` reports: **All trackable CONTEXT.md decisions are honored by shipped artifacts.** `{ honored: 29, total: 29, not_honored: [] }`. This covers D-01 to D-29 in `02-CONTEXT.md`, including the owner amendments and the stop rules.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|-----------|-----------|--------|---------|----------|-----------------|---------|
| `port/tests/rfork_test.rs` | DSGN-03, DSGN-04 | 3 | 0 | No — literal values and `0x3c7c7f` are the oracle | Value / behavioural | ✓ PASS |
| `xtask/src/rforkh.rs` (`mod tests`) | DSGN-02, DSGN-03 | 5 | 0 | No — the header renderer is checked against literal Harvey lines; the one-definition test reads real sources | Value / source invariant | ✓ PASS |
| `xtask/tests/threat_model.rs` | DSGN-01 | 6 | 0 | No — the boundary is compared against the external AC source; the report/status assertions are literal | Document / behavioural | ✓ PASS |
| `x86_64/src/syscall.rs` (`rfork_mask_tests`) | DSGN-04 | 1 | 0 | No — asserts literal `0x3c7f` and bit-zero checks | Value | ✓ PASS (owner log) |
| `aarch64/src/syscall.rs` (`rfork_mask_tests`) | DSGN-04 | 1 (twice) | 0 | No — same literal/value assertions | Value | ✓ PASS |

**Disabled tests on requirements:** 0 → no blocker.
**Circular patterns detected:** 0 → no blocker.
**Insufficient assertions:** 0 → no warning. The value pins are exact-value assertions, not existence/type checks.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `xtask/src/main.rs` | 709 | `TODO Choose UART at cmdline` | ℹ️ Info | Pre-existing: blame `11196de8` (2026-11-01), in the raspi QEMU command builder, not in a phase-added hunk; not phase debt |
| `xtask/src/main.rs` | 749-750 | `FIXME` (riscv `disk.bin`) | ℹ️ Info | Pre-existing: blame `c1568ee5` (2023-12-03) / `9b95ad2d` (2024-07-21), in the riscv QEMU builder, not in a phase-added hunk; not phase debt |
| — | — | `TBD`/`FIXME`/`XXX` added by this phase | — | None: `git diff c002feb..HEAD` added-lines grep for the three markers is empty; the debt-marker gate is satisfied for phase-introduced lines |
| `xtask/src/main.rs:3554-3602`, `.github/workflows/xtask.yml` | — | The one-definition invariant and the threat-model checks are not run by `cargo xtask test` or CI (WR-01) | ⚠️ Warning | The tests exist, pass manually and are the phase's guard rails; the required port/arch/header checks are wired. Open disposition in `02-REVIEW-DISPOSITION.md` |
| `xtask/src/rforkh.rs:175-213` | — | The one-definition invariant scans four hardcoded arch files and one exact spelling (WR-02) | ⚠️ Warning | It covers the four files that hold rfork code but not every `.rs` in the crates; no redefinition exists today. Open disposition |
| `sys/include/rfork.h:15-34` | — | No include guard; a second include is a compile error (WR-03) | ℹ️ Info | Skipped by design: 02-01 Task 2 specifies no guard (Harvey's `libc.h` has none) and the banner forbids including it beside `libc.h`; accepted in `02-REVIEW-DISPOSITION.md` |
| `docs/threat-model.md:177` | — | Section 7 still reads as if bits 18-20 are not reserved, after this phase reserved 18-21 in r9 (WR-04) | ⚠️ Warning | The document's scope boundary and protection rows are correct; this open-point bullet now misstates the r9 half (Harvey is still untouched). Open disposition; does not affect the phase goal |
| `xtask/tests/threat_model.rs:12-13` | — | The document test hard-fails without `.planning/` (IN-01) | ℹ️ Info | CI checks out the full tree; a planning-filtered export would fail. Open info disposition |
| `xtask/tests/threat_model.rs:158-164` | — | The AC comparison searches past section 2.2 (IN-02) | ℹ️ Info | A moved boundary line could be satisfied by a later line; the current source passes. Open info disposition |
| `flake.nix:974-983` | — | The re-measured FOD hash has no provenance beside it (IN-03) | ℹ️ Info | The measurement is in `RESULTS.md`; the hash commit body is empty and the comment names an older value. Open info disposition |
| `xtask/src/rforkh.rs:169-173` | — | `rforkh_mutated_render_differs` is redundant with the staleness test (IN-04) | ℹ️ Info | Necessity-only; no behavioural defect. Open info disposition |

**Anti-patterns:** 0 blockers, 3 warnings (all pre-existing code-review findings, open dispositions), 6 info. No stubs, placeholder implementations, hardcoded empty data or unimplemented handlers were introduced by the phase.

### Human Verification Required

#### 1. D-29 host-test / hash-commit order

**Test:** Confirm whether the x86_64 host tests ran before the `boot-iso` `outputHash` commit `4624ea2` (D-29). The only returned host-test log is the round-2 log at `HEAD=4624ea2` (the hash commit itself), generated after the commit; round 1 returned no host-test log at all.

**Expected:** The owner either attests that a pre-commit host-test run happened in round 1 (and, if available, produces that log), or accepts the deviation. If accepted: the hash remains valid because the round-2 host tests passed at the gated commit and no compile fix was needed; record the acceptance as an override or a new dated decision. No code change is required.

**Why human:** The ordering is a human process step, not code. No test exercises it, and the returned logs cannot prove a run that was not returned. The hash commit timestamp (`2026-11-23T22:22:53-05:00` / `03:22:53Z`) precedes the round-2 return (`2026-11-24T04:48:35Z`), and the host-test log's `HEAD=` is exactly the hash commit, so the only recorded run is after the commit.

### Gaps Summary

**No code gaps found.** The phase goal is achieved in the codebase: the four `rfork` bits are reserved once in `port::rfork`, both arch crates import them, both masks remain `0x3c7f`, the generated C header matches the Rust constants, the threat model is committed with the correct boundary and statuses, the thirteen design decisions are recorded with the owner's confirmation, and the before/after kernel evidence shows `.text`/`.text.boot` unchanged. The nine owner-stop verdicts are all PASS and `4624ea2` is an ancestor of `origin/main` (`3d01f07`).

The one open item is a **process-order must-have**, not a code defect: D-29 says the x86_64 host tests run before the `outputHash` build and commit, but the only returned host-test log is at the hash commit itself, after it was created. The phase's hash is still valid because the round-2 tests passed at the gated commit and no compile fix was needed. Owner confirmation or acceptance will close the item; if accepted, add an override (or a new dated decision) to record it.

The code review's four warnings and four info items are recorded in `02-REVIEW.md` / `02-REVIEW-DISPOSITION.md`; none is a blocker for this phase, and WR-03 is skipped by design. The one-definition invariant's narrow scope (WR-02) and its absence from `cargo xtask test`/CI (WR-01) are hardening follow-ups, not phase-goal failures; the required value/overlap pins and arch mask tests are wired and green.

### Verification Metadata

**Verification approach:** Goal-backward, from the ROADMAP Phase 2 goal and success criteria, the six plans' frontmatter must-haves, and the phase requirement IDs.
**Must-haves source:** 02-01 to 02-06 PLAN frontmatter + ROADMAP Phase 2 success criteria.
**Automated checks:** `cargo test -p port --test rfork_test` 3/3; `cargo test -p xtask threat_model` 6/6; `cargo test -p xtask rfork` 5/5; `cargo xtask test` (full; mask test twice); `cargo xtask rfork-header --check`; aarch64 mask test; `clang -fsyntax-only`; patch `cmp`; draft `cmp`; gated-commit code diff. All green except the one process-order evidence check (not a code check).
**Human checks required:** 1 (D-29 order).
**Total verification time:** ~25 minutes.

---

_Verified: 2026-11-24T05:10:00Z_
_Verifier: the agent (gsd-verifier)_
