---
phase: "2"
slug: "guide-phase-0-design-lock-and-rfork-flag-reservation"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-23"
validated: "2026-11-24"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `02-RESEARCH.md`, section "Validation Architecture".
>
> **Audit revision (2026-11-24).** All six rows were re-audited against the code, the
> committed tests and the phase's evidence. The auditor re-ran every command the Mac can
> run — the `port` pins, the full `xtask` suite, the header drift check with its missing
> and stale negative controls, `cargo xtask test` (the aarch64 mask test twice),
> `cargo xtask check`, and the committed kernel-byte and owner-stop evidence checks — and
> rebuilt two kernel configs, whose `.text`/`.text.boot` matched `after.sha256` exactly.
> No test file was added: every automated row already has a committed behavioural test,
> and DSGN-05 is the owner's ruling, recorded in `PROJECT.md` and `02-DECISION-TABLE.md`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests: `port` (via `cargo xtask test`), `xtask` (via `cargo test -p xtask`), `aarch64` host leg (via `cargo xtask test`), `x86_64` host leg (the owner's x86_64 Linux host; the Mac skips it, D-24); shell/expect gates for boot |
| **Config file** | none (workspace `Cargo.toml`; `.planning/config.json` sets `nix develop -c cargo xtask check` / `test`) |
| **Quick run command** | `nix develop -c cargo xtask test && nix develop -c cargo test -p xtask` (both halves; the configured test command is only the first) |
| **Full suite command** | quick, then `nix develop -c cargo xtask check`; the owner's end-of-phase gate set ran at the gated commit (plan 02-06) |
| **Estimated runtime** | quick: seconds once built (measured 2026-11-24: `cargo test -p xtask` 90 unit + 6 document tests; `cargo xtask test` four host legs; both well under the 60 s budget); full: several minutes from cold |

`cargo xtask test` does not run the xtask crate's own tests; `cargo test -p xtask` does. The x86_64 host test leg is skipped in the nix devshell on the Mac and compiles on the owner's x86_64 Linux host (after the xtask host-triple fallback fix, D-24). `cargo xtask clippy --arch aarch64` fails at HEAD with 4 errors and is not a pass criterion for this phase; the default-arch clippy is green (plan 02-05).

**Required dispatch mode (set before `/gsd-execute-phase 2`):** `workflow.use_worktrees` = `false` in `.planning/config.json`, set with `node "$HOME/.claude/gsd-core/bin/gsd-tools.cjs" query config-set workflow.use_worktrees false`, or set `"use_worktrees": false` under `"workflow"` in `.planning/config.json` by hand. The key is unset and defaults to true, which dispatches each plan into an agent worktree. With it off, every plan runs in sequence on the main working tree. With it on: plan 02-06 fails its main-tree check on first dispatch and stops (it merges and pushes nothing); 02-01 Task 4 and 02-02 Task 3 stop if the continuation worktree lacks the first agent's commits; 02-03's edits to ROADMAP.md and STATE.md, which are plan content, meet the worktree executor's instruction not to update those files. Each of those stops names this setting as the remedy; spawning another continuation agent without changing it repeats the stop.

---

## Sampling Rate

- **After every task commit:** Run the quick run command
- **After every plan wave:** Run the full suite command
- **Before `/gsd-verify-work`:** Full suite green on the Mac; byte evidence recorded for four configs with `.text`/`.text.boot` equal; aarch64 quick pass green; then the single owner stop on the x86_64 Linux host. **Completed:** the owner stop ran at gated commit `4624ea2` — nine of nine `stop-` verdicts PASS (`evidence/owner-stop/RESULTS.md`); the branch push and `main` push were each approved (D-26)
- **Max feedback latency:** 60 seconds for the quick command

---

## Per-Task Verification Map

Task IDs name the plan and task (`02-01-T1` is plan 02-01, Task 1); the rows below are the per-requirement contract.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-02-T1, 02-02-T3 | 02-02 | 1 | DSGN-01 | T-02-06, T-02-07, T-02-08, T-02-25 | Threat-model doc states in-scope and out-of-scope boundaries exactly, names SEV-SNP/TDX, states no protection is implemented yet, every statement cited or marked new | doc check | `nix develop -c cargo test -p xtask threat_model` | ✅ `xtask/tests/threat_model.rs`; `docs/threat-model.md` | ✅ green — 6/6 |
| 02-01-T2, 02-01-T4, 02-04-T3 | 02-01, 02-04 | 1, 2 | DSGN-02 | T-02-02, T-02-14 | `sys/include/rfork.h` equals the rendering of the Rust flag table; a mutated copy is rejected | unit + CLI | `nix develop -c cargo xtask rfork-header --check`; `nix develop -c cargo test -p xtask rfork` | ✅ `xtask/src/rforkh.rs`; `sys/include/rfork.h` | ✅ green — check matches, 5/5 tests; stale and missing copies exit 1 |
| 02-01-T2, 02-01-T4, 02-04-T1, 02-04-T2, 02-03-T1 | 02-01, 02-04, 02-03 | 1, 2 | DSGN-03 | T-02-01, T-02-04, T-02-13 | Sixteen flags defined once in the `port/` module; no definition left in the arch files; both arch crates build | port test + source-invariant test + build | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo test -p xtask rfork_flags_have_one_definition`; `nix develop -c cargo xtask check` | ✅ `port/src/rfork.rs`; `port/tests/rfork_test.rs`; `xtask/src/rforkh.rs` | ✅ green — 3/3 + invariant + both builds |
| 02-01-T2, 02-01-T4, 02-04-T1, 02-04-T2, 02-06-T1, 02-06-T2 | 02-01, 02-04, 02-06 | 1, 2, 4 | DSGN-04 | T-02-03, T-02-12, T-02-15 | Four values pinned; disjoint single bits; bit 14 is `RFNOMNT`; none of the five in either supported mask; mask value unchanged | unit (port; aarch64 on Mac; x86_64 on the owner's Linux host) | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo xtask test` | ✅ `port/tests/rfork_test.rs`; `aarch64/src/syscall.rs`; `x86_64/src/syscall.rs` | ✅ green — mask test twice on the Mac; x86_64 140 passed at `4624ea2` (W-2) |
| 02-01-T1, 02-05-T1, 02-05-T2, 02-05-T3, 02-06-T1, 02-06-T2 | 02-01, 02-05, 02-06 | 1, 3, 4 | DSGN-04 (behaviour unchanged) | T-02-16, T-02-23, T-02-26 | Kernel code unchanged around the move for both arches; boot still reaches rio | evidence check + live spot check | six `text`/`textboot` lines equal before/after; six `IDENTICAL` verdicts; nine `stop-` PASS and zero FAIL (exact commands in §Audit) | ✅ `evidence/kernel-bytes/{before,after}.sha256`; `KERNEL-BYTES.md`; `evidence/owner-stop/RESULTS.md` | ✅ green — evidence checks + q35cpu/virtcpu rebuilds match (W-5) |
| 02-03-T1, 02-03-T2, 02-03-T3 | 02-03 | 1 | DSGN-05 | T-02-09, T-02-10 | PROJECT.md records each of IG-D01..D13 as locked, locked-amended, or not-locked-deferred; no "confirm in Phase 2" left | manual (owner confirmation; record checked) | `grep -c "confirm in Phase 2" .planning/PROJECT.md` returns 0; thirteen-row Key Decisions gate | n/a — manual; record in `02-DECISION-TABLE.md` and `.planning/PROJECT.md` | 📋 manual — owner confirmed 2026-11-23; recorded verbatim |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · 📋 manual-only (evidence recorded)*

---

## Wave 0 Requirements

- [x] `port/src/rfork.rs` and `port/tests/rfork_test.rs` — delivered by 02-01; the owner approved the `port/src` diff (patch sha256 `998ba6a0eee57eaed83790575869f47a7ec4ca39fc866783260505a59bd2c786`) before commit
- [x] xtask header generator module with unit tests and negative control; `rfork-header` subcommand — delivered by 02-01 (`render`, `write`, `check`, four tests); wired into `TestStep` by 02-04
- [x] Supported-mask hoist and a test in each arch `syscall.rs` — delivered by 02-04 (`RFORK_SUPPORTED` = `0x3c7f`; the two `rfork_mask_tests` blocks are byte-identical)
- [x] `sys/include/rfork.h` (generated) and `docs/threat-model.md` — both committed and tracked; the header equals `render()`
- [x] xtask host-triple fallback widened to x86_64 hosts so the x86_64 host tests run on the owner's Linux host — delivered by 02-04 (`host_triple_fallback_covers_aarch64_and_x86_64 ... ok`); the leg ran in 02-06
- [x] Before-kernels for four configs (`q35churnshell`, `q35cpu`, `virtshell`, `virtcpu`) built from the pre-edit commit and copied out of `target/` — `evidence/kernel-bytes/before.sha256` names base `c002feb`, venue `Darwin arm64` and ten hashes
- [x] Framework install: none

---

## Manual-Only Verifications

These behaviours are manual **by design**, not because a test is missing. The phase is complete: every row below carries committed evidence that this audit read. DSGN-05 is the only **requirement** whose acceptance act is itself manual — the maintainer's ruling cannot be stood in for by a test.

| Behavior | Requirement | Why Manual | Recorded evidence |
|----------|-------------|------------|-------------------|
| The maintainer confirms IG-D01 to IG-D13 and PROJECT.md Key Decisions records the statuses | DSGN-05 | Requirement-level manual: the owner's ruling is the requirement (CONTEXT D-03; the D-08 rule makes a lock owner-only change). A test can check the record, not make the decision | `02-DECISION-TABLE.md` `## Owner ruling` (2026-11-23): `confirm`, `Landing-site question: yes`; PROJECT.md 13/13 rows (8 locked, 2 locked amended, 3 deferred); `confirm in Phase 2` = 0; `PENDING` = 0 |
| Owner approves the `port/src` diff before commit | DSGN-03 | Standing owner rule; T-02-04 (a change not shown could reach the tree) | `02-01-SUMMARY.md` records the reply `approve` against patch sha256 `998ba6a0…`; `evidence/pending-rfork.patch`; `evidence/PINS.md` `APPROVED` line |
| Owner reviews the threat-model draft | DSGN-01 | CONTEXT D-15; the document tests can prove a tag is present, not that the cited section supports the row | draft `evidence/threat-model-draft.md` (sha256 `a8f28c35…`); the committed document is byte-equal to it; `evidence/PINS-threat-model.md` |
| Branch push for transport to the Linux host | DSGN-04 | Standing owner rule; CONTEXT D-26 | `RESULTS.md` `stop-branch-push: PASS`; branch `phase-02/design-lock` at `f8d077d`; `origin/main` stayed `af46615` until the second approval |
| x86_64 host tests, including the rfork mask test | DSGN-04 | Not runnable on the Mac (no x86_64 std in the nix toolchain, no Rosetta); CONTEXT D-24 | `evidence/owner-stop/return/round2/host-tests.log`: `HEAD=4624ea2…` … `EXIT=0`; `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok`; `test result: ok. 140 passed; 0 failed` |
| KVM quick gate once, then the TCG gate and the full gate set | DSGN-04 | The gate refuses to run on the Mac; CONTEXT D-25, D-27 | `RESULTS.md` `stop-gate-kvm`, `stop-gate-tcg`, `stop-gate-full`: PASS, all logs `HEAD=4624ea2…`/`EXIT=0` |
| `boot-iso` `outputHash` re-derivation and the one-line commit | DSGN-04 | The fixed-output hash belongs to the x86_64 Linux host; CONTEXT D-28 | commit `4624ea2` (`flake.nix` only, one line); plain build and `--rebuild` at the gated commit; `RESULTS.md` `stop-bootiso-hash-commit: PASS` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — every row names a committed test or a committed evidence check; DSGN-05 is manual-only with the owner's ruling recorded
- [x] Sampling continuity: no 3 consecutive tasks without automated verify — the two owner stops (02-01 Task 3, 02-06 Tasks 1 and 3) are bracketed by automated plans and evidence commits
- [x] Wave 0 covers all MISSING references — all seven items delivered, as listed above
- [x] No watch-mode flags
- [x] Feedback latency < 60s — measured seconds for the quick command once built
- [x] `nyquist_compliant: true` set in frontmatter — five requirements plus the behaviour-unchanged sub-requirement are covered by tests that ran or by committed evidence that this audit re-checked; DSGN-05 is the sole justified manual-only requirement

**Approval:** audited 2026-11-24 — 6/6 map rows resolved (5 requirements plus behaviour-unchanged); warnings W-1 to W-5 recorded below, none of which leaves a requirement unverified.

---

## Audit — 2026-11-24

### Requirement results

| Requirement | Coverage that ran in this audit | Result |
|-------------|---------------------------------|--------|
| DSGN-01 | `cargo test -p xtask threat_model` — six document tests: status sentence, citation rule, AC 2.2 boundary quoted word for word (with SEV-SNP/TDX), required sections, every row/bullet cited or marked, every protection row `not implemented`, no `locked` claim | ✅ 6/6 |
| DSGN-02 | `cargo xtask rfork-header --check` prints `matches port/src/rfork.rs`; `cargo test -p xtask rfork` 5/5 (four render/check tests plus the one-definition invariant); CLI negative controls: one-character mutation exits 1 with `is stale against port/src/rfork.rs`, missing file exits 1 with `read …`, restore exits 0 | ✅ green |
| DSGN-03 | `cargo test -p port --test rfork_test` 3/3; `cargo test -p xtask rfork_flags_have_one_definition` ok; `cargo xtask check` exit 0; greps: 0 flag definitions in `x86_64/src/proc.rs` and `aarch64/src/syscall.rs`, 0 `crate::syscall::RF` paths, one `RFORK_SUPPORTED` declaration and one rejection return in each `syscall.rs` | ✅ green |
| DSGN-04 | port pins 3/3; `cargo xtask test` exit 0 with `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok` twice (aarch64 default and `virtshell`); the x86_64 copy ran on the owner's host at `HEAD=4624ea2…` — `test result: ok. 140 passed`, same mask test `ok` | ✅ green (x86_64 leg is owner-venue, W-2) |
| DSGN-04 (behaviour unchanged) | the six `text`/`textboot` lines are equal between `before.sha256` and `after.sha256`; six `IDENTICAL` verdicts; nine `stop-` verdicts PASS and zero FAIL; live rebuilds: q35cpu `.text` `3618c330…`, virtcpu `.text` `ea9b4bcd…`, virtcpu `.text.boot` `920bdd94…` — all equal to `after.sha256` | ✅ green (whole-ELF not reproducible, W-5; gates owner-run) |
| DSGN-05 | owner ruling `confirm` and landing-site `yes` recorded 2026-11-23; thirteen Key Decisions rows in PROJECT.md; `confirm in Phase 2` = 0; `PENDING` = 0; `Nothing is locked` = 0 | 📋 manual-only, record verified |

### Commands run (2026-11-24, aarch64 Mac)

| # | Command | Result |
|---|---------|--------|
| 1 | `nix develop -c cargo test -p port --test rfork_test` | **3 passed**, 0 failed |
| 2 | `nix develop -c cargo test -p xtask` | **90 unit + 6 document + 3 doctests passed**, 0 failed |
| 3 | `nix develop -c cargo test -p xtask rfork` | **5 passed** (four `rforkh_*` plus `rfork_flags_have_one_definition`) |
| 4 | `nix develop -c cargo xtask rfork-header --check` | exit 0, `matches port/src/rfork.rs` |
| 5 | mutate `(1<<21)` to `(1<<22)` in `sys/include/rfork.h`; run the check; restore | exit 1 with `is stale against port/src/rfork.rs`; missing file exit 1 with `read …`; restore exit 0; `git status` clean |
| 6 | `nix develop -c cargo xtask test` | **exit 0**; `port` 168 + integration suites + `aarch64` 72 and 88; mask test ok twice; no `FAILED` |
| 7 | `nix develop -c cargo xtask check` | **exit 0** (warnings only) |
| 8 | `nix develop -c cc -fsyntax-only -Wno-error=unused-command-line-argument -Wall -std=c99 -x c sys/include/rfork.h` | exit 0; the verbatim `-Werror` form is blocked by clang 21 treating nix `-L` flags as errors (W-3) |
| 9 | `diff` of the six ` text`/` textboot` lines between `before.sha256` and `after.sha256` | empty — `.text`/`.text.boot` identical |
| 10 | verdict counts in `KERNEL-BYTES.md` and `RESULTS.md` | 6 `IDENTICAL`; 9 `stop-` verdicts; 0 FAIL |
| 11 | `nix develop -c cargo xtask build --arch x86-64 --config q35cpu`; hash `.text` | `.text` `3618c330…` — equals `after.sha256`; whole ELF differs (W-5) |
| 12 | `nix develop -c cargo xtask build --arch aarch64 --config virtcpu`; hash `.text` and `.text.boot` | `ea9b4bcd…` and `920bdd94…` — both equal `after.sha256` |
| 13 | inspect `evidence/owner-stop/return/round2/host-tests.log` | `HEAD=4624ea2…` … `EXIT=0`; 140 passed; x86_64 mask test ok |
| 14 | DSGN-05 gates: `confirm in Phase 2`; thirteen-row gate; `PENDING`; `Nothing is locked` | 0; **13**; 0; 0 |
| 15 | one-definition greps and mask-block `diff` | 0/0 flag definitions; 0 `crate::syscall::RF`; 1/1 mask declarations; 1/1 rejection returns; mask test blocks byte-identical |

### Warnings (caveats, not uncovered requirements)

| # | Warning | Impact / disposition |
|---|---------|----------------------|
| W-1 | The configured test command (`cargo xtask test`) does **not** run the `xtask` crate's own tests, where DSGN-01's document checks and DSGN-03's one-definition invariant live. | The phase's quick command adds `cargo test -p xtask`, and every map row names its command explicitly. Both halves ran green in this audit; no coverage was lost. |
| W-2 | The x86_64 `rfork_mask_tests` cannot be compiled on the Mac (no Rosetta; the nix toolchain ships std only for `aarch64-apple-darwin`). | The leg ran on the owner's x86_64 Linux host at the gated commit (`host-tests.log`, 140 passed, mask test `ok`), and the block is byte-identical to the aarch64 copy that ran twice here. Environment-specific, evidenced, not an uncovered requirement. |
| W-3 | The supporting C syntax check is not reproducible **verbatim** in this audit shell: clang 21 promotes the nix stdenv's unused `-L` flags to errors under `-Werror`. | The same file passes with `-Wno-error=unused-command-line-argument`. DSGN-02's authoritative checks are the generator tests and `--check`, both green. Recorded so a toolchain-env failure is not read as a header defect. |
| W-4 | `.planning/REQUIREMENTS.md` still shows DSGN-02, DSGN-03 and DSGN-05 unchecked with traceability status `Pending`, although the work and evidence are complete (DSGN-01 and DSGN-04 are checked). | A tracking inconsistency, not a test-coverage gap. It was not edited because this audit commits only `02-VALIDATION.md`. Correct the checkboxes before `/gsd-audit-milestone` reads the requirement table. |
| W-5 | Whole-ELF sha256 is not reproducible across rebuilds on this machine: a fresh q35cpu build gave ELF `4f1c83a9…` against the recorded `610804e6…`, while `.text` matched exactly; virtcpu's `.text` and `.text.boot` also matched. | D-23's stop condition is `.text`/`.text.boot`, so the whole-ELF rows are informational only and must not be read as a code-change signal. |

### Residual owner actions

None required for validation. The phase's owner stop is complete (9/9 PASS at `4624ea2`); two local evidence commits (`fdfd3de` and the 02-06 summary) publish only on a later approved push; `phase-02/design-lock` may be deleted once it is no longer needed; W-4's REQUIREMENTS.md bookkeeping remains.

## Validation Audit 2026-11-24

| Metric | Count |
|---|---|
| Gaps found | 6 |
| Resolved | 6 |
| Escalated | 0 |
