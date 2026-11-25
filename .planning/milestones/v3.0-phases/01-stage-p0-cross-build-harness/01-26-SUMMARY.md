---
phase: 01-stage-p0-cross-build-harness
plan: 26
subsystem: cross-build-harness
status: complete
tags: [harvey-flip, scratch-clone, q35, own-kernel, d-26, d-20, d-19, d-10, witness, stop-c, equivalence, harn-08, harn-02]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-22/01-24/01-25/01-33's moved gates, witness steps and green stop-B; the committed S5 kernel copy and boot-q35.sh; Harvey at 25d97dc4d"
provides:
  - "$HOME/r9-harvey-flip/harvey: the prepared Harvey flip change (never committed by the executor)"
  - "$HOME/r9-harvey-flip/harvey-flip.diff: the full diff the owner reviews at stop C"
  - "harvey-flip/HARVEY-FLIP.md: DIFFSTAT, deletions, kept exception, D-19 check, validations, view commands, commit message"
  - "Harvey app q35 (nix/q35.sh): build+boot Harvey's own kernel on q35 to the 'root is from' prompt (D-26)"
  - "witness/run-witness.sh: step_full, step_soak, step_stress, step_equiv, step_store_verify"
  - "witness/equivalence.tsv: per-gate markers from the committed Harvey-hosted captures (churn = none)"
affects: [01-27, 01-28, HARN-02, HARN-08]

actuals:
  tokens: 135592
  tasks: 2
  commits: 3

plan_head_before: ade53993a83bc92a9b9dd14ba8de13f1988abb4b
plan_head_after: c34edebb41ba7fb1ceaf263ec1e7a84614461df4

tech-stack:
  added: []
  patterns:
    - "A flake-ownership flip is prepared in a scratch clone and reviewed as one diff; the executor never commits in the approved-exception repository"
    - "A deletion pass greps the retained files per basename and separates executable uses (keep) from doc/comment mentions (list, leave)"
    - "An own-kernel launcher reuses the spike's capture-before-start/kill-reap harness rather than growing a second launch path"
    - "An equivalence table stores occurrence counts (grep -oE), not line counts; a gate without a committed baseline says none and gets a fresh pinned-rev run"
    - "A case-insensitive checkout materialises case-colliding paths as M; the flip diff excludes them so the protected font set is provably untouched"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/harvey-flip/HARVEY-FLIP.md
    - .planning/phases/01-stage-p0-cross-build-harness/witness/equivalence.tsv
    - $HOME/r9-harvey-flip/harvey/nix/q35.sh (scratch clone, not committed)
    - $HOME/r9-harvey-flip/harvey-flip.diff (scratch, not committed)
  modified:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh

key-decisions:
  - "The Harvey change is prepared exactly as D-10 requires and left uncommitted: the owner approves the diff at stop C, then commits and (with r9) pushes."
  - "Only build/x86_64/drawterm-gate.expect is kept among the moved files (its comment in the retained nix/drawterm-x64.sh is the plan's expected exception); every other retained mention of a deleted basename is documentation in README/.rc/tier-list files and is listed as such, not treated as a use."
  - "The scratch clone's 11 sys/** modifications are APFS case-collision checkout artefacts; the flip diff is generated with ':(exclude)sys/**' so the 11 protected font/postscript paths appear zero times."
  - "step_full exports R9_ACCEL=kvm because xtask's full tier parses --accel but does not propagate it to the apps it drives; the child gates must run the KVM leg explicitly."
  - "equivalence.tsv counts marker occurrences (grep -oE). churn is the only gate without a committed Harvey-hosted capture, so it is 'none' and step_equiv runs the pinned Harvey app as the fresh baseline."
  - "The q35 launcher must run from the Harvey checkout root (HARVEY defaults to PWD's git toplevel); invoked from another checkout it fails loudly with not-a-checkout instead of booting the wrong tree."
  - "The q35 boot half is proven on the Mac under TCG against the committed S5 kernel (sha256 ecdcdadd…), reaching 'root is from (tcp)[tcp]:'; Harvey's build half was NOT run (sudo/lima prohibition)."

requirements-completed: [HARN-08, HARN-02]

coverage:
  - id: D1
    description: "The one owner-approved Harvey change is fully prepared in a scratch clone outside both repos; the owner's checkout is byte-for-byte untouched"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "HARVEY-UNTOUCHED guard: HEAD 25d97dc4d + status hash 041d2929…; diff 511797 bytes / sha256 f7b0272d…"
        status: pass
    human_judgment: false
  - id: D2
    description: "After the prepared flip the scratch Harvey flake has no r9 input or boot-r9*/gate outputs, keeps setup/toolchain/build/iso/drawterm/devshell, and adds the q35 own-kernel app"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix flake show --all-systems: forbidden-output grep count 0; flake.lock root inputs = nixpkgs; nix eval of setup/toolchain/build/iso/q35 (all systems) and drawterm (linux) EVAL-OK"
        status: pass
      - kind: e2e
        ref: "HARVEY_Q35_ACCEL=tcg nix run path:$HF/harvey#q35 -- --no-build -> Q35-RESULT banner=yes prompt=yes sigs=none, 'root is from (tcp)[tcp]:'"
        status: pass
    human_judgment: false
  - id: D3
    description: "Stop C can run the whole moved set from r9 (full, soak and stress on the owner's tap venue), compare it with Harvey-hosted results, and verify both trees afterwards"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "shellcheck clean; step_full/soak/stress/equiv/store_verify defined (5); equivalence.tsv names all 10 gates, every source path exists under .planning/legacy"
        status: pass
    human_judgment: false
---

# Phase 1 Plan 26: Prepare the Harvey flip change (scratch clone) and the stop-C witness steps

**One-liner:** The single D-10 Harvey commit is prepared and validated in `$HOME/r9-harvey-flip` — r9 input, composed outputs and 31 moved files removed, `q35` app added and proven booting Harvey's own kernel to `root is from` under Mac TCG — plus `step_full`/`step_soak`/`step_stress`/`step_equiv`/`step_store_verify` and a 10-gate `equivalence.tsv` for stop C.

## Performance

- **Duration:** 6 min (2026-11-23T17:21:58Z → 2026-11-23T17:28:11Z; nix store and kernel already warm)
- **Tasks:** 2
- **Files modified:** 3 committed (HARVEY-FLIP.md, run-witness.sh, equivalence.tsv) + the uncommitted scratch clone and diff
- **Commits:** 3 in the plan range

## Accomplishments

1. **The prepared Harvey flip (Task 1).** `flake.nix` loses `inputs.r9`, all 15 r9-consuming packages and the `netDoctorPkg`/`hostEchoX64Pkg` gate-only packages, plus every `boot-r9*`/gate app registration; it keeps the harness/minimega/drawterm packages, the setup/toolchain/build/iso/default apps, the devshell and the `drawterm` app, and gains `q35` (D-26). `flake.lock` drops the `r9` node and its three transitive inputs.
2. **The new own-kernel launcher.** `nix/q35.sh` builds through Harvey's platform build (`--build`; `build.sh` on Linux, `vm-build.sh` on darwin), records the kernel sha256, boots `-M q35` with virtio-net and serial capture, classifies banner/`root is from`/the 01-04 signatures, kills and reaps qemu on every exit, and requires explicit `HARVEY_Q35_ACCEL=tcg` when `/dev/kvm` is absent. Proven on this Mac: `Q35-RESULT machine=q35 accel=tcg banner=yes prompt=yes sigs=none`, no survivor.
3. **The deletion set, safely.** 16 gate scripts + `host-echo.c`, all 10 x86_64 and 5 aarch64 expect drivers, and `.github/workflows/r9-shell.yml` (whole) plus `phase6.yml`'s `tier-r9` job. Per-basename greps over the retained files found only the plan's expected exception (`drawterm-gate.expect`) as an executable-adjacent reference; the rest are doc/comment mentions, listed in HARVEY-FLIP.md.
4. **D-19 proven.** The deletion set's intersection with r9's 102 tree/reference read-set paths is empty; no `.expect` and no `nix/*.sh` appears in any read set.
5. **Stop-C witness steps (Task 2).** `step_full` (the eight-app D-14 tier with per-app capture retention), `step_soak`/`step_stress` (net-doctor `--mode tap` precondition, loud failure, no slirp substitute), `step_equiv` (r9 `target/gates/<app>` against `equivalence.tsv`; fresh pinned-rev Harvey run for `none`; `out/equiv/report.tsv`), `step_store_verify` (both trees, last).
6. **The equivalence table.** 10 gates, 24 marker rows with occurrence counts from the latest committed Harvey-hosted captures under `.planning/legacy`; churn is `none` because no capture is committed.

## Task Commits

| Task | Commit | Message |
| ---- | ------ | ------- |
| 1 | `0bff4db` | docs(01-26): prepare the Harvey flip change in a scratch clone |
| 2 | `de9c139` | feat(01-26): add the stop-C witness steps and the equivalence baselines |
| follow-up | `c34edeb` | docs(01-26): record the q35 launcher's cwd contract |

The plan metadata commit (this SUMMARY) is the remaining commit.

## Files Created/Modified

- **Created:** `.planning/phases/01-stage-p0-cross-build-harness/harvey-flip/HARVEY-FLIP.md` — the prepared change's review packet.
- **Created:** `.planning/phases/01-stage-p0-cross-build-harness/witness/equivalence.tsv` — 10 gates / 24 marker rows.
- **Modified:** `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` — stop-C header docs, `FULL_APPS`, the five steps, selection/mkdir wiring (+237/−3).
- **Scratch (not committed):** `$HOME/r9-harvey-flip/harvey` (36-path change: 285 insertions, 9972 deletions) and `$HOME/r9-harvey-flip/harvey-flip.diff` (511797 bytes, sha256 `f7b0272d…`).

## Decisions Made

- The Harvey change stays uncommitted; stop C is the approval gate (D-02 `costly`, D-10).
- `drawterm-gate.expect` is the only kept moved file (the plan's expected exception); other retained mentions are documentation, listed rather than silently left.
- The `sys/**` exclusion in the diff is the honest way to prove the protected font set is untouched on an APFS clone.
- `R9_ACCEL=kvm` is exported for `step_full` because xtask's full tier does not propagate `--accel` to its child apps.
- Equivalence counts are `grep -oE` occurrences; churn gets a fresh Harvey@bd7a698 baseline at stop C.
- The launcher runs from the Harvey checkout root (or with `HARVEY` set) and refuses any other tree loudly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical behavior] `step_full` exports `R9_ACCEL=kvm`**
- **Found during:** Task 2 (`step_full`)
- **Issue:** `cargo xtask gate --full --accel kvm` parses `--accel` but `run_full` runs `nix run .#<app>` without passing it; the child gates would fall back to their `auto` default.
- **Fix:** the step runs the gate with `R9_ACCEL=kvm` in the environment, so every driven app runs the KVM leg explicitly.
- **Files modified:** `witness/run-witness.sh`
- **Verification:** shellcheck clean; `FULL_APPS` matches `xtask/src/gate.rs`'s `FULL_APPS_X86_64`.
- **Committed in:** `de9c139`

**2. [Rule 2 - Consistency] `phase6.yml` doc lines updated with the `tier-r9` removal**
- **Found during:** Task 1 (CI deletion)
- **Issue:** removing the `tier-r9` job leaves the header block and the venue-pin echo describing a job that no longer exists.
- **Fix:** the header's `tier-r9` bullet is removed and the venue-pin echo names the remaining jobs; every functional assertion is unchanged.
- **Files modified:** the scratch clone's `.github/workflows/phase6.yml`
- **Verification:** YAML parses; `grep tier-r9` → none; the diff shows +2/−22 (the job block plus the two doc lines).
- **Committed in:** `0bff4db` (documented in HARVEY-FLIP.md)

**3. [Rule 1 - Bug] q35's shellcheck failure at derivation build**
- **Found during:** Task 1 (boot-half validation)
- **Issue:** `writeShellApplication` runs shellcheck; `prompt_seconds` was computed but never printed (SC2034), so the app derivation failed.
- **Fix:** the output contract gained `Q35-BANNER_SECONDS`/`Q35-PROMPT_SECONDS`, restoring S5's timing evidence and clearing the warning.
- **Files modified:** the scratch clone's `nix/q35.sh`
- **Verification:** the derivation builds; the boot half reaches the prompt; `nix run` from a non-Harvey cwd fails loudly with `not-a-checkout`.
- **Committed in:** `0bff4db` (the diff packet records the result)

---

**Total deviations:** 3 auto-fixed (1 bug, 2 missing critical/consistency)
**Impact on plan:** All three are necessary for the step to be truthful and the diff to be coherent; no scope creep beyond the plan's file set (deviations 2–3 are inside the scratch clone the plan owns).

## Issues Encountered

- **The plan's boot-half verify has a cwd implication.** `nix run "path:$HF/harvey#q35"` resolves `HARVEY` from the *caller's* git toplevel; run from the r9 checkout it fails `not-a-checkout` (loudly, exit 2). The validated invocation runs from `$HF/harvey` (or sets `HARVEY`), now recorded in HARVEY-FLIP.md.
- **APFS case collisions in the scratch clone.** Harvey's case-colliding `sys/**` paths materialise as 11 `M` entries in any fresh checkout on this Mac. The diff excludes `sys/**`, and HARVEY-FLIP.md explains why; the protected font set is provably untouched (`grep` count 0).
- **No committed churn capture.** `.planning/legacy` holds no green churn run, so `equivalence.tsv` marks it `none` and `step_equiv` will produce the baseline from the pinned Harvey app at stop C.
- **Harvey's build half was not run.** The plan forbids it (sudo on Linux, the owner's lima directory on the Mac); only `--no-build` was exercised.

## Authentication Gates

None.

## User Setup Required

None.

## Next Phase Readiness

- **Stop C (01-27) is the next owner action and was not started here.** The owner reviews `$HOME/r9-harvey-flip/harvey-flip.diff`, commits the prepared Harvey change, and runs the stop-C witness steps (`full`, `soak`, `stress`, `equiv`, `store_verify`).
- HARN-08 closes only when the owner commits the Harvey change, the moved gates pass from r9 with equivalent results, and r9 bumps its Harvey pin (01-27/01-28). HARN-02's read-only guarantee is exercised by `step_store_verify`.
- The scratch clone must survive until stop C; it is outside both repositories by design.

## Self-Check: PASSED

- `harvey-flip/HARVEY-FLIP.md`, `witness/equivalence.tsv`, `witness/run-witness.sh` all exist in the worktree.
- Commits `0bff4db`, `de9c139`, `c34edeb` are ancestors of HEAD.
- HARVEY-UNTOUCHED holds; the worktree is clean.
