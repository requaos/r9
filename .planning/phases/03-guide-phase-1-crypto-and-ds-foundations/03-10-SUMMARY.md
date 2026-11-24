---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 10
subsystem: ds
tags: [lint, unsafe-audit, clippy-probe, evidence, kernel-bytes, full-suite, miri, coverage, owner-stop, no_std]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-13: the one unsafe site in ds/src/tpht/ (deref_tiny's unchecked arena read) with its SAFETY invariant; plan 03-07 and 03-08: the chained and flattened modules, the Miri suite and the recorded MIRI-OVER precedent; plan 03-09: the PROJECT.md rows the owner reviews; plans 03-01 and 03-12: the six crypto pins and the ds member with its roaring dev-dependency"
provides:
  - "UNSAFE-AUDIT.md: the four-gated-crate counting rule and scope line, the ds row (1) and crypto row (0), SAFETY 152 -> 153 with the aarch64 staleness note, the Phase 3 delta paragraph, and the '## ds/ and crypto/ (Phase 3)' section with the module-state table, the tpht site classification and the third-party dependency record"
  - "evidence/closing-suite.md: the clippy probe verdict, the audit count reconciliation, suite-run-1 and suite-run-2 (five exit codes each) and the two MIRI-TIME lines"
  - "evidence/kernel-bytes/after-03-10.sha256 and the KERNEL-BYTES.md end-of-phase section: elf-virtshell@03-10 and elf-q35churnshell@03-10 IDENTICAL"
  - "COVERAGE.md: the no-external-API declaration"
  - "03-OWNER-STOP.md: the ordered x86_64 Linux checklist (branch push, sandboxed kernel builds at the tip and base, the four-hash comparison, the ISO build and rebuild, the host suite, the report-back, the open choices) that plan 03-11 consumes"
affects: [03-11, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 8738
  tasks: 3
  commits: 3
  plan_head_before: e39cb1b7fe2382ef099f0febce8d38c5e8e2c808
  plan_head_after: 7e42046268bc20935dc147a006b935487fbaf94f

tech-stack:
  added: []
  patterns:
    - "The audit's count is proved mechanical: a comment-stripping scan of ds/src/tpht/ yields TPHT_SITES, the evidence carries AUDIT-COUNT tpht source=N audit=N, and a bash gate fails when the two differ"
    - "The lint contract is proved by a scratch probe, not asserted: deleting one // SAFETY: comment makes `nix develop -c cargo xtask clippy` exit 1 with 'unsafe block missing a safety comment', and the probe is reverted before any commit"
    - "The end-of-phase kernel comparison is 'after' = the phase's last code commit; evidence and planning commits that follow it cannot move a kernel and are named as such in the record"
    - "A supplementary byte-for-byte cmp against the copies plan 03-12 kept sits beside the sha256 comparison, so a hash collision or a transcription error in the sha256 file cannot pass unnoticed"
    - "Every owner-stop verdict command runs through one run_logged function, so each log under ~/r9-p03-stop/ begins with its HEAD= line and ends with EXIT=, and a step measured in the base worktree logs the base commit"
    - "The third-party dependency record counts upstream unsafe with one labelled method (raw grep -ow over each crate's src/), and says that the counts include backends r9 never selects"

key-files:
  created:
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/closing-suite.md
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/after-03-10.sha256
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/COVERAGE.md
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/03-OWNER-STOP.md
  modified:
    - UNSAFE-AUDIT.md
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/KERNEL-BYTES.md

key-decisions:
  - "TPHT_SITES is 1: ds/src/tpht/flattened.rs:440, the deref_tiny unchecked arena read (block x1); there is no unsafe fn, impl, extern block or test-only block in ds/, and crypto/ has 0 sites of every kind, so the reconciliation reads 129 + 23 + 1 + 0 = 153 SAFETY annotations"
  - "The aarch64 count of 129 is left exactly as measured, with a sentence naming the 7 pre-existing undocumented sites `cargo xtask clippy --arch aarch64 --config virtshell` finds at HEAD; it was not re-derived, because that is outside Phase 3 (RESEARCH C5, the plan's prohibition)"
  - "The dependency record lists all six direct pins with their exact feature sets, all 17 transitive crates in Cargo.lock, and raw upstream unsafe counts. F10's figures are reproduced exactly where F10 lists the crate (blake3 238, chacha20 66, arrayvec 58, constant_time_eq 54, hybrid-array 43, sha2 39, cmov 28, block-buffer 22, zeroize 21, cpufeatures 11, ctutils 1, and hmac/hkdf/digest/crypto-common/typenum/cfg-if 0); the crates F10 omits (inout 31, libc 810, cc 36, shlex 7, find-msvc-tools 55, cipher 0) were counted the same way at audit time and labelled so"
  - "The clippy probe deleted the six-line SAFETY comment above the arena read, ran the default `cargo xtask clippy` (the form hosted CI runs) and got exit 1 with `error: unsafe block missing a safety comment` at flattened.rs:434; the revert left `git status --porcelain -- ds` empty, so the probe is in no commit"
  - "The kernel 'after' commit is 1be121c, the plan's Task 1 commit and the phase's last code commit; the two suite runs and the kernel measurement were made there, and the record says that the Task 2 and Task 3 commits after it change only evidence and planning documents"
  - "Both kernels are byte-identical to the baseline at the end of the phase on the one venue: 9218b998...fa083e (virtshell) and c9dcef0b...8b78b (q35churnshell), and a supplementary cmp against target/phase03-kernel-bytes/before/ also reports no difference; D-21's stop did not fire"
  - "Both suite runs are green: suite-run-1 and suite-run-2 each have five exit codes 0, 30 `test result: ok` lines in cargo xtask test, the RFC 4231/5869/8439, BLAKE3 and keys pins on both the default and the soft-backend crypto legs, the doctest pair, and the ds leg (roaring:: 6, clientset:: 5, setops:: 2, tpht:: 30 with the D-15 generator ignored)"
  - "Miri is over the 120-second budget: MIRI-TIME closing-run-1 350 and closing-run-2 389 for 64 seeds over 29 tests. No source file may change in this plan, so this is reported in the SUMMARY as the plan directs, not fixed, and the suite verdicts stand (both runs rc 0 with no Undefined Behavior or Data race)"
  - "One documentation pointer was added under the audit's '## Remaining sites — classification (aarch64 + port)' heading telling the reader that the two Phase 3 crates are classified in the new section above; no existing count, row or sentence was changed"
  - "Plan 03-11's prerequisites are in place: the audit section, two suite-run PASS verdicts, two @03-10 IDENTICAL verdicts and the checklist; nothing was pushed and origin/main is unchanged"
  - "The owner checklist adds two things Phase 2's checklist did not need: the base builds run in a second worktree so their HEAD= lines carry the base commit, and the ISO build is paired with --rebuild as the second witness for the x86_64 kernel bytes"

requirements-completed: [LINT-01]

coverage:
  - id: D1
    description: "UNSAFE-AUDIT.md covers the two new crates: the four-gated-crate counting rule and scope line, the ds row (1 lint-visible block) and crypto row (0), SAFETY 152 -> 153, the Phase 3 delta, the module-state table, the tpht site classification with its SAFETY invariant, and the third-party dependency record; the ds count equals the source scan"
    requirement: LINT-01
    verification:
      - kind: unit
        ref: "grep -c '^## ds/ and crypto/ (Phase 3)' UNSAFE-AUDIT.md -> 1; grep -c 'four gated crates' -> 2; AUDIT-COUNT tpht source=1 audit=1 parity gate passes; `git status --porcelain -- ds crypto xtask` prints nothing"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask clippy after the scratch deletion of one // SAFETY: comment: exit 1, `error: unsafe block missing a safety comment` at ds/src/tpht/flattened.rs:434; reverted (VERDICT clippy-rejects-undocumented-unsafe: PASS)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The full suite is green twice at the phase's last code commit, and both kernels are byte-identical to the pre-phase baseline on one venue; the no-external-API declaration is recorded"
    requirement: LINT-01
    verification:
      - kind: integration
        ref: "nix develop -c cargo xtask test (30 `test result: ok` lines), cargo test -p xtask, cargo xtask check, cargo xtask clippy, cargo xtask miri (64 seeds, 29 passed per seed) — all rc 0 in both runs (VERDICT suite-run-1: PASS, VERDICT suite-run-2: PASS; MIRI-TIME 350 and 389)"
        status: pass
      - kind: other
        ref: "evidence/kernel-bytes/after-03-10.sha256 equals before.sha256 on both ` elf ` lines; VERDICT elf-virtshell@03-10: IDENTICAL and VERDICT elf-q35churnshell@03-10: IDENTICAL; cmp against target/phase03-kernel-bytes/before/ exits 0 twice; protected paths diff quiet against 4beb5c8"
        status: pass
    human_judgment: false
  - id: D3
    description: "03-OWNER-STOP.md is the ordered page for the owner's x86_64 Linux host covering the one check the Mac cannot make, with every verdict command logged, and the two open choices that do not block the stop"
    requirement: LINT-01
    verification:
      - kind: automated_ui
        ref: "the checklist's six step commands are present (kernel-x64, kernel-virt, boot-iso --rebuild, HEAD:refs/heads/phase-03/crypto-ds-foundations, git worktree add, cargo xtask miri); the kernel build precedes the ISO build; 7 HEAD= mentions; =1.9.1 and 274,877,906,880 both present; no attribution line; no `push origin main`"
        status: pass
    human_judgment: false

# Metrics
duration: 15min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 10: The audit, the closing evidence, and the owner's checklist

**`UNSAFE-AUDIT.md` now accounts for `ds/`'s one unsafe site and `crypto/`'s zero (SAFETY 152 → 153), clippy is shown to reject an undocumented block with a reverted scratch probe, both suite runs and both kernels are green and byte-identical at the phase's last code commit — and the single Linux check the Mac cannot make is written as one ordered page for the owner.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-11-24T16:55:42Z
- **Completed:** 2026-11-24T17:11:07Z
- **Tasks:** 3 (all `type="auto"`, no checkpoints in the plan)
- **Files modified:** 6 (2 modified, 4 created)

## Accomplishments

- **The audit closes LINT-01's ledger.** The counting rule and the scope line name the four gated crates (`aarch64`, `port`, `crypto`, `ds`); the reconciliation table gains `crypto` 0 and `ds` 1 and restates SAFETY as 153; a dated Phase 3 delta paragraph sits in the form of the 08-02 one; and the new `## ds/ and crypto/ (Phase 3)` section carries the module-state table (`tpht` the named unsafe-holding module, `feistel`/`roaring`/`setops`/`clientset` forbidden, the reserved Roaring SIMD module approved with no code until Phase 5, `crypto/` crate-level forbid with 0 sites), the one classification row (`flattened.rs:440`, `deref_tiny`'s `get_unchecked`, with the constructor invariant from its `// SAFETY:` comment), and the third-party dependency record.
- **The count is proved, not asserted.** A comment-stripping scan of `ds/src/tpht/` yields one site; the same scan finds nothing in `crypto/src` or in `ds/src` outside `tpht/`; and the evidence carries `AUDIT-COUNT tpht source=1 audit=1`, which a bash gate compares for equality.
- **The lint contract is proved by probe.** Deleting the six-line `// SAFETY:` comment above the arena read made `nix develop -c cargo xtask clippy` (the default form hosted CI runs) exit 1 with `error: unsafe block missing a safety comment` at `flattened.rs:434`. The edit was reverted with `git checkout -- ds/src/tpht`, `git status --porcelain -- ds` printed nothing, and no probe reached a commit (`VERDICT clippy-rejects-undocumented-unsafe: PASS`).
- **The suite is green twice.** At `1be121c` — the phase's last code commit — `cargo xtask test` (30 `test result: ok` lines), `cargo test -p xtask`, `cargo xtask check`, `cargo xtask clippy` and `cargo xtask miri` (64 seeds, 29 passed per seed) all exit 0 in both runs. The evidence copies the result lines for the RFC 4231, RFC 5869 and RFC 8439 pins on both the default and the soft-backend `crypto` legs, the BLAKE3 pins, the `keys` tests, the passing and `compile fail` doctest pair, and the `ds` leg (`roaring::` 6, `clientset::` 5, `setops::` 2, `tpht::` 30 with the D-15 generator ignored).
- **Both kernels are byte-identical at the end of the phase.** Rebuilt from nothing after their target directories were moved aside, `virtshell` is `9218b998…fa083e` and `q35churnshell` is `c9dcef0b…8b78b`, both equal to the baseline taken before the first workspace change on the same venue. `VERDICT elf-virtshell@03-10: IDENTICAL` and `VERDICT elf-q35churnshell@03-10: IDENTICAL`; a supplementary `cmp` against the copies kept from plan 03-12 also reports no difference; the protected-path diff against the base is quiet. D-21's stop did not fire.
- **The one check the Mac cannot make is now one ordered page.** `03-OWNER-STOP.md` walks the owner from the branch-push approval through the two sandboxed kernel builds at the tip, the same two at the base commit in a second worktree, the four-hash comparison, the `boot-iso` build and `--rebuild`, and the four suite commands on the x86_64 host, to a report-back that maps one-to-one onto plan 03-11's eleven verdicts. Every verdict command goes through `run_logged`, so its log under `~/r9-p03-stop/` begins with `HEAD=` and ends with `EXIT=`. Nothing was pushed: `origin/main` is unchanged and the branch does not exist on the remote yet.
- **The open choices are in front of the owner without blocking.** The `zeroize` pin (`=1.9.2`, with `=1.9.1` as the older alternative) and the ChaCha20 per-key-and-nonce limit (2^32 − 1 blocks, 274,877,906,880 bytes) each have a tick box and a stated cost; the section says nothing there changes a verdict or delays the stop.
- **`COVERAGE.md` records the reasoned no-external-API declaration** in one line, so the plan-time API-coverage checkpoint has its answer.

## Task Commits

Each task was committed atomically:

1. **Task 1: `UNSAFE-AUDIT.md` — the `ds/` and `crypto/` section, and the clippy probe** — `1be121c` (docs)
2. **Task 2: the full suite twice, kernel bytes at the end of the phase, and the API-coverage declaration** — `484533a` (evidence)
3. **Task 3: the ordered owner checklist for the end-of-phase stop** — `7e42046` (docs)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `UNSAFE-AUDIT.md` — the four-gated-crate counting rule and scope line; the reconciliation table with the `crypto` 0 and `ds` 1 rows and SAFETY 153; the aarch64 staleness sentence; the Phase 3 delta paragraph; the `## ds/ and crypto/ (Phase 3)` section (module states, classification, dependency record); a one-line pointer under the aarch64+port classification heading
- `evidence/closing-suite.md` — the source enumeration table, `AUDIT-COUNT tpht source=1 audit=1`, the clippy probe with its decisive output and `VERDICT clippy-rejects-undocumented-unsafe: PASS`, then `suite-run-1` and `suite-run-2` with five exit codes each, the result lines per leg, `VERDICT suite-run-1/2: PASS`, `MIRI-TIME closing-run-1 350`, `MIRI-TIME closing-run-2 389`, the Miri budget note and the kernel-bytes pointer
- `evidence/kernel-bytes/after-03-10.sha256` — the four-line record in the baseline's format, `after 1be121c…` first and the same `venue` line
- `evidence/kernel-bytes/KERNEL-BYTES.md` — the "End of the phase (plan 03-10)" section: the base and after commits, the venue check, the before/after table, the two `@03-10` verdicts, the supplementary `cmp`, the protected-path result, the `flake.nix` devshell-only note, and what the section does not prove
- `COVERAGE.md` — the single `No external API integration:` declaration line
- `03-OWNER-STOP.md` — sections 0 to 7 in order, the `run_logged` setup, the pass condition and failure meaning per step, the report-back list, the open-choices section and the not-part-of-this-stop note

## Decisions Made

See `key-decisions` in the frontmatter. The items the plan left to execution:

- **What counts as a site.** The comment-stripping scan counts the `unsafe` token outside `//` comments; `ds/src/tpht/` yields exactly one block and `crypto/src` yields none, matching what plan 03-13's summary and the lint report said.
- **Where the "after" commit sits.** The measurement was made at `1be121c`, the last commit that touches code; the record says so rather than implying the phase's literal last commit builds a different kernel.
- **How far the dependency record goes.** Every crate `Cargo.lock` shows reachable from `crypto/` gets a row, with F10's counts where F10 measured the crate and the same method's counts where it did not, all labelled as raw grep counts that include unselected backends.
- **How the base builds are logged.** They run inside the base worktree so their `HEAD=` lines prove they measured the base commit, which is what makes the four-hash comparison meaningful.

## Deviations from Plan

### Plan-authorized reporting

**1. [Plan rule "Miri latency"] Both Miri runs are over the 120-second budget, and no fix is possible in this plan**
- **Found during:** Task 2, the two suite runs
- **Measurement:** `MIRI-TIME closing-run-1 350` and `MIRI-TIME closing-run-2 389` for 64 seeds over 29 `tpht` tests, against the 116 s that plan 03-13 measured for 13 tests and the 402 s plan 03-08 recorded for this 29-test suite at its resize floors.
- **Adjustment:** none. The plan forbids changing any source file, and the plan itself says a time above the budget does not fail the suite verdict but is reported in the SUMMARY. Both runs exit 0 with 29 passed per seed and no Undefined Behavior or Data race.
- **Files modified:** none
- **Verification:** `VERDICT suite-run-1: PASS` and `VERDICT suite-run-2: PASS`, each decided from that run's own five exit codes.
- **Committed in:** `484533a` (the evidence file)

### Documentation refinements (no count or verdict changed)

**2. [Completeness] The dependency record covers every crate in `Cargo.lock`, not only the ones F10 counted**
- **Found during:** Task 1, writing the third-party record
- **Issue:** RESEARCH F10 gives raw counts for eleven crates and lists six as zero, but `Cargo.lock` also brings `inout`, `libc`, `cc`, `shlex` and `find-msvc-tools` into the graph.
- **Fix:** those five were counted with the same method at audit time (`inout` 31, `libc` 810, `cc` 36, `shlex` 7, `find-msvc-tools` 55) and the table labels the whole column as raw grep counts over upstream `src/` that include backends r9 never selects.
- **Files modified:** `UNSAFE-AUDIT.md`
- **Verification:** every F10 figure that the table reproduces is byte-equal to RESEARCH F10.
- **Committed in:** `1be121c`

**3. [Readability] A one-line pointer was added under the aarch64+port classification heading**
- **Found during:** Task 1
- **Issue:** the new section sits above `## Remaining sites — classification (aarch64 + port)`, so a reader could expect the Phase 3 crates there.
- **Fix:** one parenthetical sentence points at the new section. No existing row, count or sentence changed.
- **Files modified:** `UNSAFE-AUDIT.md`
- **Verification:** the aarch64 row of 129 and the SAFETY sum are unchanged and still reconcile.
- **Committed in:** `1be121c`

---

**Total deviations:** 1 plan-authorized reporting item (the Miri overage, with no fix permitted) and 2 documentation refinements with no effect on any count or verdict.
**Impact on plan:** none on LINT-01, on either suite verdict or on the kernel verdicts. The Miri latency is the one unmet target, it is recorded in the evidence and here, and it follows the overages plans 03-07 and 03-08 reported and the orchestrator accepted.

## Issues Encountered

- **Miri takes about six minutes on this suite.** 350 s and 389 s for 64 seeds over 29 tests. The two full runs therefore took about 25 minutes together. Nothing was cut: seeds, tests and native sizes are unchanged (the plan forbids source edits).
- **The Miri many-seeds output interleaves.** The 64 seed runs merge their `test result:` lines in places; each run still reports 29 passed / 1 ignored per seed and exits 0, which is what the verdict uses.
- **The `before/` snapshot names differ from the build outputs.** The copies plan 03-12 kept are `virtshell.elf` and `q35churnshell.elf`, so the supplementary `cmp` pairs those with `debug/aarch64` and `debug/x86_64`; both exit 0.
- **The Miri budget cannot be brought back inside 120 s in this plan.** Every previous resize was a source change; this plan changes no source, so the overage is reported for the phase-end review instead.

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-36 (an `unsafe` site missing from the audit) is mitigated by the comment-stripping scan and the `AUDIT-COUNT` equality gate; T-03-37 (kernel bytes changed somewhere in the phase) by the from-nothing rebuild at the phase's last code commit, the sha256 comparison and the supplementary `cmp`; T-03-38 (the sandboxed derivations failing with the enlarged lock) is handed to the owner stop's sections 3 to 5, since it cannot be measured on the Mac; T-03-39 (a checklist command writing outside its work directory) by confining the checklist to `git`, `nix build`, `nix develop -c cargo xtask …` and `sha256sum` with outputs under `~/r9-p03-stop/`; T-03-40 (a verdict written without a run) by deciding every verdict from that run's own exit codes and naming the commit. T-03-SC does not apply (no package was added).

## Known Stubs

None. No placeholder value, TODO or unwired data source was introduced: the plan creates evidence and planning documents only, and every number in them is copied from a run recorded beside it.

## Next Phase Readiness

- **Plan 03-11 has every precondition its Task 1 checks:** `UNSAFE-AUDIT.md` has the Phase 3 section, `closing-suite.md` has `suite-run-1` and `suite-run-2` PASS plus the clippy probe PASS, `KERNEL-BYTES.md` has two `@03-10` IDENTICAL verdicts, and `03-OWNER-STOP.md` is committed. `git status --porcelain --untracked-files=no` prints nothing, and the tree is on `main` in the main working tree.
- **Plan 03-11's eleven verdict ids map onto the checklist:** `stop-branch-push` (section 1), `stop-kernel-x64-build` / `stop-kernel-virt-build` (section 3), `stop-kernel-x64-bytes` / `stop-kernel-virt-bytes` (section 4's four hashes), `stop-bootiso-build` / `stop-bootiso-rebuild` (section 5), `stop-host-test` / `stop-host-check` / `stop-host-clippy` / `stop-host-miri` (section 6).
- **Nothing is pushed.** `origin/main` is unchanged and `origin/phase-03/crypto-ds-foundations` does not exist; the branch is created by the approved push in the checklist's section 1. Between that push and the owner's return, no commit may be made on the Mac, so `main` can take the branch by fast-forward.
- **The Miri overage is the phase's open latency item.** 350–389 s against 120 s; a later phase that adds `tpht` tests will meet the same rule, and the phase-end review now has the number for the 29-test suite.
- **Phase 4 and Phase 5 inherit the audit's contract:** any new `unsafe` in `crypto/` needs owner approval, a third unsafe-holding module in `ds/` needs owner approval, and extending the lint and audit to `x86_64` is Phase 5's question.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All six claimed files exist on disk (`UNSAFE-AUDIT.md`, `evidence/closing-suite.md`, `evidence/kernel-bytes/after-03-10.sha256`, `evidence/kernel-bytes/KERNEL-BYTES.md`, `COVERAGE.md`, `03-OWNER-STOP.md`) and all three task commits (`1be121c`, `484533a`, `7e42046`) are ancestors of HEAD.

Plan-level verification at the final state:

- `grep -c '^## ds/ and crypto/ (Phase 3)' UNSAFE-AUDIT.md` → `1`; `grep -c 'four gated crates'` → `2`; the audit's `ds` row is 1 and its `crypto` row is 0; SAFETY is 153 and `129 + 23 + 1 + 0 = 153`; the aarch64 row still reads 129 beside the 7-site staleness sentence.
- `AUDIT-COUNT tpht source=1 audit=1` parses to equal values (the bash gate passes), and `git status --porcelain -- ds crypto xtask` prints nothing — the clippy probe was reverted.
- `grep -cE '^VERDICT suite-run-[12]: PASS$'` → `2`; `grep -cE '^VERDICT elf-(virtshell|q35churnshell)@03-10: IDENTICAL$'` → `2`; the ` elf ` lines of `before.sha256` and `after-03-10.sha256` diff clean; the `venue` lines are equal; the protected-path diff against `4beb5c8` is quiet; `nix develop -c cargo xtask test` exits 0 with 30 `test result: ok` lines.
- `COVERAGE.md` is one declaration line beginning `No external API integration:` with no table row.
- The checklist's six step commands are all present, the kernel build precedes the ISO build, it has 7 `HEAD=` mentions, both `=1.9.1` and `274,877,906,880` appear, it carries no attribution line, and it contains no `push origin main`.
- Each task's commit stat lists only that task's files (`1be121c`: `UNSAFE-AUDIT.md` and the evidence file; `484533a`: the three evidence files and `COVERAGE.md`; `7e42046`: the checklist alone), and no commit deletes a tracked file.
- Nothing was pushed: `origin/main` is unchanged and the transport branch does not exist on the remote.
