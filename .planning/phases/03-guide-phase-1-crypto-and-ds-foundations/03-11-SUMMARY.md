---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 11
subsystem: process
tags: [owner-stop, push-approval, kernel-bytes, nix-sandbox, x86_64-linux, boot-iso, miri, evidence, release]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-10: the UNSAFE-AUDIT.md Phase 3 section, `suite-run-1` and `suite-run-2` PASS, the clippy probe verdict, the two `@03-10` IDENTICAL kernel verdicts, and the ordered 03-OWNER-STOP.md page that this plan consumes"
provides:
  - "`origin/phase-03/crypto-ds-foundations` at `5f565ec`: the Mac tip `95fe66a` plus the two owner return commits, pushed on the owner's first approval"
  - "`evidence/owner-stop/RESULTS.md`: the pushed tip and base shas, the four Linux kernel sha256 values, eleven `stop-` verdicts (all PASS) each with its `EVIDENCE` line quoting a returned `HEAD=`/`EXIT=` pair, and `VERDICT stop-main-push: PASS a643526…`"
  - "`origin/main` at `a643526…`: every unpushed commit on `main` published on the owner's second approval, so the remote matches the checked state"
  - "The A8 risk retired on the x86_64 Linux host: both sandboxed kernel derivations build with Phase 3's enlarged `Cargo.lock`, their bytes are identical at the tip and the base, and the fixed-output `boot-iso` builds and rebuilds on its committed `outputHash`"
affects: [phase-4, phase-5, verification]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 81374
  tasks: 3
  commits: 4
  plan_head_before: 95fe66aaadde14cb8b8fa8197f6b0547ea7b7898
  plan_head_after: 5226d1131c86442f1ae4ba8596523e7eb3fdb6f4

tech-stack:
  added: []
  patterns:
    - "Every owner-stop verdict is decided from one returned line, never from an impression: the line's `HEAD=` must equal the pushed tip (the base builds excepted by design) and its `EXIT=` must be 0, and a missing line is FAIL with the evidence `not returned`"
    - "Both pushes sit behind `blocking-human` checkpoints with the command fixed in the plan; the unpushed list is printed live before each, and `origin/main` is checked unchanged after Tasks 1 and 2 — so the owner approves a list, not a ref"
    - "The return travels through the branch and adds files only under `evidence/owner-stop/return/`, which is what lets `main` take the branch by `--ff-only` and keeps the verdicts auditable at the tip they measured"
    - "Kernel bytes are compared on one host and one nix store: the base builds run in a second worktree so their `HEAD=` lines carry the base commit, and the fixed-output ISO build plus `--rebuild` is the second, independent witness of the same bytes"
    - "The evidence commit that records the `main` push stays local by design, so the plan ends with `origin/main` one commit behind the working tree rather than rewriting a published ref"

key-files:
  created:
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/SUMMARY.txt
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/tip-kernel-x64.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/tip-kernel-virt.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/base-kernel-x64.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/base-kernel-virt.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/kernel-hashes.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/boot-iso-first.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/boot-iso-rebuild.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/host-test.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/host-check.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/host-clippy.log
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/return/host-miri.log
  modified:
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/owner-stop/RESULTS.md

key-decisions:
  - "The owner approved the branch push and ran the checklist; the return came by the branch route with two commits (9af4f1e and 5f565ec) that add files only under `evidence/owner-stop/return/`. The later tree is authoritative because the runner was executed twice; both commits are kept and named in RESULTS.md"
  - "All eleven verdicts are PASS, decided only from returned lines: `stop-branch-push`, the two tip builds, the two byte comparisons, the ISO build and rebuild, and `stop-host-test`/`-check`/`-clippy`/`-miri`. Every `HEAD=` line of the tip-side logs is the pushed tip 95fe66a; the two section-4 base logs carry 4beb5c8 as the checklist requires"
  - "No kernel byte changed on Linux: `kernel-x64` is 6a5f5a66…01cf6a5 at the tip and at the base, and `kernel-virt` is 954e5110…30f4eee at both, so the D-21 `boot-iso` `outputHash` procedure does not apply and the boot-to-rio gate keeps holding by identity with the Phase 2 gated state (4624ea2)"
  - "The fixed-output ISO is the second witness: `nix build .#boot-iso` and `nix build .#boot-iso --rebuild` both exit 0 with no `hash mismatch` and no `got:` line under the committed `sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`. The five F10-era `outputHash` values are untouched"
  - "The host suite passes on x86_64, where the default `crypto` leg links the AVX2/SHA-NI backends for the first time: `host-test` has 29 `test result: ok` lines (final line `140 passed; 0 failed`) and both the default and soft-backend legs; `host-check`, `host-clippy` and `host-miri` exit 0, the last over 64 seeds with 29 passed per seed and no Undefined Behavior or Data race"
  - "`main` was pushed exactly once, only after the owner's second ruling (\"push-main\"): `git push origin main` printed `3d01f07..a643526  main -> main`, and `git rev-parse origin/main` plus `git ls-remote origin refs/heads/main` both return a64352625efe9cb8a924d7941c6be09c4f738a11 — the sha shown at the checkpoint, before this task's evidence commit"
  - "The `VERDICT stop-main-push: PASS a643526…` line is committed as 5226d11 (`evidence(03-11): main pushed after the owner stop`) and stays local until a later approved push, exactly as the plan directs; `main` is therefore one evidence commit ahead of `origin/main` at the end of the plan"
  - "The two Miri overages accepted earlier stand with coverage unchanged: plan 03-07's chained run at 230 s and plan 03-08's tpht-spill/tpht-hit-rate at 315–329 s and 402 s after the size floors. Seeds (64), test list (29) and native sizes are unchanged, both plans' runs exited 0 with no Undefined Behavior or data race, and only the 120 s feedback-latency budget is missed"
  - "Plan 03-10's closing Miri runs (350 s and 389 s) are carried to the phase-end review unchanged, and the host's own `host-miri` run adds a third green measurement of the same 64 seeds; no source file was edited in this plan, so nothing was resized"
  - "The plan's `files_modified` names `return/paste.txt` as the paste-route artifact; the branch route was used instead, so the twelve returned log files are the evidence. RESULTS.md records that no credential string appeared in any log and nothing needed redaction"

requirements-completed: [LINT-01]

coverage:
  - id: D1
    description: "The transport branch `phase-03/crypto-ds-foundations` exists on the remote at the Mac tip plus the owner's return commits, and nothing else was published on the first approval"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "git ls-remote origin refs/heads/phase-03/crypto-ds-foundations -> 5f565ec047b5ef8168e450e85f80275f1823d8e6; git diff --name-only 95fe66a origin/phase-03/crypto-ds-foundations lists 12 files, all under evidence/owner-stop/return/ (the same diff with ':(exclude)…/return' is empty); VERDICT stop-branch-push: PASS"
        status: pass
    human_judgment: false
  - id: D2
    description: "The owner's x86_64 Linux return is recorded as eleven verdicts with their evidence lines, and the four Linux kernel sha256 values show no kernel byte changed at the tip versus the base"
    requirement: LINT-01
    verification:
      - kind: integration
        ref: "grep -cE '^VERDICT stop-(branch-push|kernel-x64-build|kernel-virt-build|kernel-x64-bytes|kernel-virt-bytes|bootiso-build|bootiso-rebuild|host-test|host-check|host-clippy|host-miri): (PASS|FAIL)$' evidence/owner-stop/RESULTS.md -> 11; grep -cE '^VERDICT stop-[a-z0-9-]+: FAIL$' -> 0"
        status: pass
      - kind: other
        ref: "kernel-hashes.log HEAD=95fe66a: 6a5f5a66…01cf6a5 for tip and base kernel-x64, 954e5110…30f4eee for tip and base kernel-virt; boot-iso-first.log and boot-iso-rebuild.log both EXIT=0 with no hash mismatch and no got: line; host-*.log each EXIT=0 (VERDICTs stop-kernel-*-bytes, stop-bootiso-*, stop-host-*)"
        status: pass
    human_judgment: false
  - id: D3
    description: "`main` is pushed to `origin` after the owner's second approval, and the pushed sha is the one the checkpoint showed"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "git push origin main -> `3d01f07..a643526  main -> main`; git rev-parse origin/main and git ls-remote origin refs/heads/main -> a64352625efe9cb8a924d7941c6be09c4f738a11; RESULTS.md last line `VERDICT stop-main-push: PASS a64352625efe9cb8a924d7941c6be09c4f738a11`"
        status: pass
    human_judgment: false

# Metrics
duration: 1h 32min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 11: The single end-of-phase owner stop — sandboxed kernel builds on Linux, then `main`

**The x86_64 Linux host retires the A8 risk — both sandboxed kernel derivations build with Phase 3's enlarged `Cargo.lock` and are byte-identical to the base, the fixed-output ISO builds and rebuilds on its committed `outputHash`, all four host suite commands pass, and `main` is pushed to `origin` at `a643526` on the owner's second approval.**

## Performance

- **Duration:** 1 h 32 min wall clock (17:12Z to 18:44Z). Task 1 is owner-run and most of the time is the Linux checklist; the executor's Task 2 and Task 3 commits are about three minutes apart.
- **Started:** 2026-11-24T17:12Z (the branch-push checkpoint opened as plan 03-10 closed)
- **Completed:** 2026-11-24T18:44Z
- **Tasks:** 3 (one `checkpoint:human-action`, one `type="auto"`, one `checkpoint:decision` — all reached and answered)
- **Files modified:** 14 (12 returned logs created, `RESULTS.md` created and then extended, this SUMMARY)

## Accomplishments

- **The one check the Mac cannot make is answered, and the answer is "nothing moved".** On the owner's x86_64 Linux host, `nix build .#kernel-x64` and `nix build .#kernel-virt` both exit 0 at the pushed tip `95fe66a` with the enlarged `Cargo.lock` — the A8 risk is retired. The same two derivations built at the base commit `4beb5c8` in a second worktree give the same bytes: `kernel-x64` is `6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5` at the tip and at the base, and `kernel-virt` is `954e51102f2e3eeccb028b984fd32730375107d32e21c666aeeadff2030f4eee` at both. No kernel byte changed on Linux, so the D-21 `boot-iso` `outputHash` procedure does not apply and the boot-to-rio gate keeps holding by identity with the Phase 2 gated state (`4624ea2`).
- **The fixed-output ISO is the independent second witness.** `nix build .#boot-iso` and `nix build .#boot-iso --rebuild` both exit 0 with the committed `sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`, the rebuild reporting `checking outputs of '/nix/store/8qzi9466f9vm5jpxw4lzzyvw7f8fmp9c-r9-boot-iso-95fe66a.drv'…` and no `hash mismatch` and no `got:` line. If the x86_64 kernel bytes had moved, this is where they would have shown.
- **The suite passes on the architecture the kernels actually run on.** `nix develop -c cargo xtask test`, `check`, `clippy` and `miri` all exit 0 on the host. The default `crypto` leg linked the x86_64 AVX2/SHA-NI backends for the first time (`crypto-4161b3aadeb5df48` and the soft-backend `crypto-5085cebe8a21af39`, both with the RFC 4231/8439 pins `ok`), 29 `test result: ok` lines appear with no `FAILED` (final line `140 passed; 0 failed`), and `cargo xtask miri` ran all 64 seeds over the 29 `tpht` tests with `29 passed; 0 failed; 1 ignored` on each and no Undefined Behavior or Data race.
- **Eleven verdicts, each decided from a returned line.** `RESULTS.md` carries `stop-branch-push`, `stop-kernel-x64-build`, `stop-kernel-virt-build`, `stop-kernel-x64-bytes`, `stop-kernel-virt-bytes`, `stop-bootiso-build`, `stop-bootiso-rebuild`, `stop-host-test`, `stop-host-check`, `stop-host-clippy` and `stop-host-miri`, all PASS, each with an `EVIDENCE` line quoting the decisive returned line and its `HEAD=` value. Every tip-side log's `HEAD=` is `95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`; the two section-4 base builds carry `HEAD=4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1`, as the checklist requires. `grep -c '^VERDICT stop-…: FAIL$'` is 0.
- **The transport branch is on the remote with only the return above the Mac tip.** `origin/phase-03/crypto-ds-foundations` is `5f565ec047b5ef8168e450e85f80275f1823d8e6`, two owner commits above `95fe66a` that add 12 files, all under `evidence/owner-stop/return/`; the same diff excluding that directory is empty. The owner ran the runner twice; the later tree is authoritative and both commits are named in `RESULTS.md`.
- **`main` is pushed, once, on the owner's second ruling.** `git push origin main` printed `3d01f07..a643526  main -> main`. `git rev-parse origin/main` and `git ls-remote origin refs/heads/main` both return `a64352625efe9cb8a924d7941c6be09c4f738a11` — the sha shown at the checkpoint before this task's evidence commit, so the push published exactly the list the owner approved (every unpushed commit on `main`, including the 14 that predated the phase).
- **The push verdict is recorded in the same file it belongs to.** `RESULTS.md` now ends with `VERDICT stop-main-push: PASS a64352625efe9cb8a924d7941c6be09c4f738a11`, committed as `5226d11` with the message `evidence(03-11): main pushed after the owner stop`. That commit stays local by design until a later approved push.
- **The knowledge graph is current.** `graphify update .` re-extracted the 9 changed code files and rebuilt `graphify-out/`: 16,667 nodes, 23,903 edges, 1,117 communities, written as `graph.json`, `graph.html` and `GRAPH_REPORT.md` (untracked, as before).

## Task Commits

Each task's work is a commit; Task 1's product commits are the owner's two return commits, fast-forwarded into `main` in Task 2:

1. **Task 1: OWNER STOP — approve the branch push and run the checklist on the Linux host** — owner-run; branch tip `95fe66a` was pushed, then `9af4f1e` and `5f565ec` (`evidence(03-11): owner-stop return logs`) added the 12 logs on the branch
2. **Task 2: Take the owner's return — record the verdicts, stop on any FAIL** — `a643526` (evidence)
3. **Task 3: OWNER STOP — approve pushing `main` now that the results are green** — `5226d11` (evidence)

**Plan metadata:** the final docs commit for this SUMMARY, `STATE.md` and `ROADMAP.md` (`REQUIREMENTS.md` needed no change: LINT-01 was already checked and Complete).

## Files Created/Modified

- `evidence/owner-stop/return/SUMMARY.txt` — the owner's run summary: `PUSHED-TIP 95fe66a…`, `SCRIPT-EXIT 0`, and the twelve logs' `HEAD=`/`EXIT=` pairs
- `evidence/owner-stop/return/tip-kernel-x64.log`, `tip-kernel-virt.log` — `HEAD=95fe66a…` and `EXIT=0` each: the sandboxed tip builds
- `evidence/owner-stop/return/base-kernel-x64.log`, `base-kernel-virt.log` — `HEAD=4beb5c8…` and `EXIT=0` each: the same builds at the base commit
- `evidence/owner-stop/return/kernel-hashes.log` — four `sha256sum` lines over `tip-*` and `base-*` kernel ELFs, two equal pairs
- `evidence/owner-stop/return/boot-iso-first.log`, `boot-iso-rebuild.log` — `EXIT=0` each, the rebuild naming the `r9-boot-iso-95fe66a.drv` output check, no hash mismatch
- `evidence/owner-stop/return/host-test.log` — 29 `test result: ok` lines, both `crypto` legs, final `140 passed; 0 failed`
- `evidence/owner-stop/return/host-check.log`, `host-clippy.log` — `Finished … target(s)` and `EXIT=0`, clippy warnings only
- `evidence/owner-stop/return/host-miri.log` — 64 seed lines, `29 passed; 0 failed; 1 ignored` per seed, no Undefined Behavior
- `evidence/owner-stop/RESULTS.md` — the pushed tip and base, the four kernel hashes, the eleven verdicts with their evidence lines, the notes on `HEAD=`/`EXIT=` counts and the credential scan, and `VERDICT stop-main-push: PASS a643526…`

## Decisions Made

- **The return route is the branch, not a paste.** The owner committed the logs under `evidence/owner-stop/return/`; `RESULTS.md` therefore names the two return commits and the `git diff --name-only 95fe66a origin/phase-03/…` check that proves only that directory moved.
- **The later of the owner's two runs is authoritative.** The runner was executed twice (`9af4f1e`, then `5f565ec`); both commits are kept, and every verdict quotes the later tree. No verdict uses the earlier run.
- **`main` was pushed exactly as approved, and the evidence commit was not pushed with it.** The checkpoint showed `a643526` and the owner ruled "push-main"; the push carried up to `a643526`, and `5226d11` (the `stop-main-push` line) stays local. Nothing else was pushed: the branch push happened in Task 1 on its own approval.
- **Nothing was fixed on the host or in the tree.** No FAIL occurred, so no stop rule fired; no `flake.nix`, `outputHash` or source file was touched, and `git status --porcelain --untracked-files=no` is clean apart from this plan's evidence and this SUMMARY.
- **The Miri overages stay accepted.** Plan 03-07 (230 s) and plan 03-08 (315–329 s and 402 s at the size floors) keep the verdicts the orchestrator accepted, with seeds, test list and native sizes unchanged; plan 03-10's closing runs (350 s, 389 s) and this plan's host run add measurements of the same coverage, not new deviations. The 120 s budget remains a phase-end review item.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The plan's commit ledger did not exist at continuation, so it was reconstructed**
- **Found during:** the summary step, before measuring `actuals.commits`
- **Issue:** protocol 0c creates `$(git rev-parse --git-dir)/gsd-plan-head-before-03-11` before the plan's first commit; no such file existed when this continuation agent started, so a bare `rev-list --count` would have had no base.
- **Fix:** the ledger was written with `95fe66aaadde14cb8b8fa8197f6b0547ea7b7898` — the last commit before the plan's first commit and the tip plan 03-10 handed over. The count and both head shas in the frontmatter are measured with `git rev-list`/`git rev-parse` from that value, not narrated.
- **Files modified:** `.git/gsd-plan-head-before-03-11` (untracked metadata, not a repository file)
- **Verification:** `git rev-list --count 95fe66a..HEAD` is 4 at summary time and names exactly `9af4f1e`, `5f565ec`, `a643526`, `5226d11`.
- **Committed in:** n/a (metadata only)

### Plan-authorized variations

**2. [Artifact route] The return came by the branch, so the twelve logs replace `return/paste.txt`**
- The plan's `files_modified` lists `return/paste.txt` as the paste-route artifact; its artifact table names both routes. The branch route was used, so `return/` holds the 12 logs instead and no `paste.txt` exists. `03-11-PLAN.md`'s own Task 1 verification allows "one return commit that adds files only under `evidence/owner-stop/return/`".

**3. [Measurement note] `graphify update .` was run after the push, not before it**
- The plan puts it at the end of Task 3; it ran after `5226d11` was committed, so the graph covers the tree including that commit. `graphify-out/` is untracked and no repository file changed.

---

**Total deviations:** 1 auto-fixed (blocking, metadata only) and 2 plan-authorized variations. **Impact on plan:** none on any verdict, on either push or on the tree; nothing was published that the owner did not approve.

## Issues Encountered

- **The live config disagrees with the plan's note.** `03-11-PLAN.md` says `workflow.use_worktrees` is `false`; the untracked `.planning/config.json` says `true`. The plan was dispatched sequentially on the main working tree anyway, and the main-tree check (`main` checked out, `--git-dir` equal to `--git-common-dir`, clean tracked tree) passed, so the check the rules actually require was satisfied. The config file is untracked and was not edited.
- **Two runs of the owner's runner.** The first return commit (`9af4f1e`) was superseded by `5f565ec`; both are on the branch and in `main`. `RESULTS.md` names the later tree as authoritative rather than rewriting the branch.
- **`host-clippy.log` is 47 KB of warnings.** They are pre-existing `xtask` warnings; the command exits 0 and no `error` or `unsafe block missing a safety comment` line appears, which is what the verdict uses.
- **Miri again dominates the clock.** 64 seeds over 29 `tpht` tests take minutes on the host as they do on the Mac; the suite is green and the overage carries to the phase-end review with the earlier ones.

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-41 (a push without approval, or of more than was approved) is mitigated by both pushes sitting behind `blocking-human` checkpoints with the command fixed, the live unpushed list printed first, `origin/main` checked unchanged after Tasks 1 and 2, and this push's sha compared against the checkpoint's; T-03-42 (a result measured at another commit) by the `HEAD=` line on every returned log and the two base logs carrying the base by construction; T-03-43 (kernel bytes differing on Linux) is now answered rather than mitigated — the tip/base pairs are equal and the ISO rebuild agrees; T-03-44 (a credential in a returned log) by the checklist's rule and `RESULTS.md`'s statement that no credential string appeared and nothing needed redaction. T-03-SC does not apply: no package was added.

## Known Stubs

None. This plan adds evidence and planning documents only; every number in `RESULTS.md` is copied from a returned log recorded beside it, and no source file was changed.

## Next Phase Readiness

- **Phase 3's owner stop is closed.** Eleven verdicts, all PASS; the branch `phase-03/crypto-ds-foundations` is on the remote at `5f565ec`, and `main` is published at `a643526` — the gated state the phase's checks measured. `origin/main` now matches the checked tree except for the one local evidence commit `5226d11`, which the plan deliberately leaves for a later approved push.
- **The golden invariant stands by identity.** No kernel byte changed on Linux (both architectures, tip versus base) and the fixed-output ISO rebuilt on its committed hash, so the boot-to-rio gate holds with the Phase 2 gated state (`4624ea2`) and no boot-to-rio run was owed here.
- **Phase 4 inherits a clean base:** the four `rfork` bits reserved in Phase 2, `crypto/`'s primitives and key hierarchy, `ds/`'s TPHT/Roaring/`ClientSet`, the audit contract, and the wire questions W1, W2 and W4 still open. `crypto::page`'s per-key-and-nonce limit (2^32 − 1 blocks, 274,877,906,880 bytes) sizes what Phase 4 may encrypt under one nonce.
- **Open for the phase-end review, not for execution:** the Miri 120 s budget (03-07, 03-08, 03-10 and this plan's host run all exceed it with coverage unchanged) and the two open choices `03-OWNER-STOP.md` put to the owner (`zeroize =1.9.2` versus `=1.9.1`; the ChaCha20 last-block limit). Neither blocks Phase 4 planning.
- **The local branch is one commit ahead of `origin/main`** (`5226d11`). A later approved push publishes it; until then the standing rule holds — nothing is pushed without the owner's word.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All five claimed files exist on disk (`03-11-SUMMARY.md`, `evidence/owner-stop/RESULTS.md`, `evidence/owner-stop/return/SUMMARY.txt`, `return/kernel-hashes.log`, `return/host-miri.log`) and all four plan commits (`9af4f1e`, `5f565ec`, `a643526`, `5226d11`) are ancestors of HEAD.

Plan-level verification at the task-commit state (before this plan's metadata commit):

- `git rev-parse origin/main` and `git ls-remote origin refs/heads/main` both print `a64352625efe9cb8a924d7941c6be09c4f738a11`; `RESULTS.md` ends with `VERDICT stop-main-push: PASS a64352625efe9cb8a924d7941c6be09c4f738a11`.
- `grep -cE '^VERDICT stop-(branch-push|…|host-miri): (PASS|FAIL)$'` on `RESULTS.md` → `11`; `grep -cE '^VERDICT stop-[a-z0-9-]+: FAIL$'` → `0`.
- `git merge-base --is-ancestor origin/phase-03/crypto-ds-foundations main` exits 0; `git rev-parse origin/phase-03/crypto-ds-foundations` is `5f565ec047b5ef8168e450e85f80275f1823d8e6`.
- `git log --format=%B origin/main..HEAD | grep -ciE 'co-authored-by|generated with'` → `0`.
- `git rev-list --count 95fe66a..HEAD` → `4` and names exactly `9af4f1e`, `5f565ec`, `a643526`, `5226d11`; `git status --porcelain --untracked-files=no` printed only this SUMMARY before its commit.
- `graphify update .` completed: `graphify-out/graph.json`, `graph.html` and `GRAPH_REPORT.md` rebuilt with 16,667 nodes and 23,903 edges; `graphify-out/` is untracked.
