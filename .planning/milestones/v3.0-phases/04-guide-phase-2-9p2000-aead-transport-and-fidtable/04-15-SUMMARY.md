---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 15
subsystem: planning-evidence
tags: [9p2000-aead, owner-stop, x86_64, kvm, tcg, boot-iso, outputHash, push]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: the approved cumulative port/src change and the Mac closing evidence of plan 04-14
provides:
  - the x86_64 Linux owner-stop return (eleven PASS verdicts) and RESULTS.md with its decisive lines
  - the owner's flake.nix boot-iso outputHash re-derivation for the Phase 4 kernel
  - phase-04/aead-wire pushed on approval, and origin/main fast-forwarded to it on a second approval
affects: [phase-05]

# Actuals (#2632) — same estimateTokens scale as the plan's estimate (chars/4 over the realized diff).
# The realized diff is dominated by the owner's returned gate logs (~10k lines): the executor's own
# prose is a small fraction. Recorded as measured, not rounded toward the estimate.
actuals:
  tokens: 122129
  tasks: 3
  commits: 6
  plan_head_before: 8b67e09cb163a01d286049a3cccfe4e87f3ab66b
  plan_head_after: 2bc5e09c464c5f13e797da3e6518750b32aada46

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "An owner stop is decided only from lines the owner returned, each carrying the HEAD= it measured; a missing line is FAIL, not an assumption"
    - "A commit made off the Mac (the hash commit) reaches main only by --ff-only after its diff and message are checked"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-RUNNER.sh
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/owner-stop/return/ (twelve log files plus REPORT.txt, and run1/ with the first round)
  modified:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/owner-stop/RESULTS.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP.md
    - flake.nix

key-decisions:
  - "Eleven verdicts PASS: every step had a returned line measured at the pushed tip or at the owner's hash commit, so no stop rule fired and the main-push decision was put to the owner."
  - "The three gates were measured at the return commit 89ba121 rather than at the hash commit ca2b5ab; the source diff between them is empty and the boot-iso --rebuild at ca2b5ab is green, so the kernel bytes did not move (disclosed, deviation 2)."
  - "origin/main was fast-forwarded to 5d1549a only after the owner replied push-main, and exactly one push command was run."

patterns-established:
  - "RESULTS.md carries one VERDICT line per checklist step with an EVIDENCE line quoting the returned line and its HEAD= value"
  - "The post-push VERDICT commit is deliberately local: it records the ruling and is published only by a later approved push"

requirements-completed: [WIRE-12]

coverage:
  - id: D1
    description: "The branch push happened only on the owner's explicit approval, and the pushed tip was stated before and confirmed after."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "git ls-remote origin refs/heads/phase-04/aead-wire -> 405984d; EVIDENCE stop-branch-push"
        status: pass
    human_judgment: true
    rationale: "The approval is the owner's own reply; no command can assert a human decision."
  - id: D2
    description: "Eleven owner-stop verdicts recorded, each from a returned line with its HEAD= value, none FAIL."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "grep -cE '^VERDICT stop-(...): (PASS|FAIL)$' RESULTS.md -> 11; grep -cE '^VERDICT stop-[a-z0-9-]+: FAIL$' -> 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "x86_64 host tests: cargo xtask test (36 suites, final line 140 passed / 0 failed) and cargo xtask test-9p (14 result lines, all aead legs), both EXIT=0."
    requirement: WIRE-12
    verification:
      - kind: integration
        ref: "evidence/owner-stop/return/host-tests.log and host-test-9p.log: HEAD=89ba121, EXIT=0"
        status: pass
    human_judgment: false
  - id: D4
    description: "The sandboxed kernel-x64 and kernel-virt derivations build on the x86_64 host with port linking crypto and ds."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "evidence/owner-stop/return/tip-kernel-x64.log and tip-kernel-virt.log: HEAD=89ba121, EXIT=0"
        status: pass
    human_judgment: false
  - id: D5
    description: "The owner re-derived the boot-iso outputHash in one flake.nix-only commit (ca2b5ab) with no attribution line."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "git show --stat ca2b5ab -> flake.nix only; EVIDENCE stop-bootiso-hash-commit"
        status: pass
    human_judgment: false
  - id: D6
    description: "boot-iso builds at the hash commit and rebuilds reproducibly with the committed hash (no hash mismatch, no got:)."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "evidence/owner-stop/return/boot-iso-after.log and boot-iso-rebuild.log: HEAD=ca2b5ab, EXIT=0"
        status: pass
    human_judgment: false
  - id: D7
    description: "Boot to rio: the KVM quick gate, the native x86_64 TCG gate and the full gate set each passed with their GATE PASS line."
    requirement: WIRE-12
    verification:
      - kind: e2e
        ref: "GATE PASS quick x86_64 run=34940-1795575414 (kvm); GATE PASS quick x86_64 run=45066-1795575611 (tcg); GATE PASS full x86_64-linux run=55162-1795575813"
        status: pass
    human_judgment: false
  - id: D8
    description: "origin/main was pushed once, on the owner's second approval, to exactly the sha shown."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "git rev-parse origin/main -> 5d1549af07de7220d6f48aac0d0db221aa7c541c (the sha shown to the owner)"
        status: pass
    human_judgment: true
    rationale: "The push authorization is the owner's ruling; the sha equality is automated but the approval itself is not."

duration: 1h 6m
completed: 2026-11-24
status: complete
---

# Phase 4 Plan 15: The end-of-phase owner stop — x86_64 byte-change procedure and boot to rio, then `main` Summary

**The owner's x86_64 Linux host proved all eleven stop steps green — 36 host suites, both sandboxed kernels, a re-derived `boot-iso` `outputHash`, and KVM, native TCG and full gates all booting to rio — and `main` was fast-forwarded to that result on a second explicit approval.**

## Performance

- **Duration:** 1h 6m (plan's first commit `a2ff50d` at 21:06 to the final evidence commit at 22:12, -05:00; the owner's host run is inside this window)
- **Started:** 2026-11-24T21:06:31-05:00
- **Completed:** 2026-11-24T22:10:23-05:00
- **Tasks:** 3/3
- **Files modified:** 3 tracked paths by the executor (`RESULTS.md`, `04-OWNER-STOP.md`, `04-OWNER-STOP-RUNNER.sh`), plus 23 files across the plan's commit range including the returned logs and the owner's `flake.nix`

## Accomplishments

- **Eleven of eleven owner-stop verdicts PASS**, each decided from a line the owner returned and each carrying the `HEAD=` it measured: `stop-branch-push`, `stop-host-test`, `stop-host-test9p`, `stop-kernel-x64-build`, `stop-kernel-virt-build`, `stop-bootiso-hash-commit`, `stop-bootiso-build`, `stop-bootiso-rebuild`, `stop-gate-kvm`, `stop-gate-tcg`, `stop-gate-full`. None is FAIL, so no stop rule fired.
- **The D-25 byte-change procedure ran in full on the Linux host.** `boot-iso` failed once with the expected fixed-output mismatch (`specified: sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`, `got: sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=`), the owner committed the new value in a `flake.nix`-only commit, and both the plain build and `--rebuild` then passed with no `hash mismatch` and no `got:` line.
- **Boot to rio, three venues.** `GATE PASS quick x86_64 run=34940-1795575414` under KVM, `GATE PASS quick x86_64 run=45066-1795575611` on the host's native TCG, `GATE PASS full x86_64-linux run=55162-1795575813` for the full set — all `EXIT=0`.
- **The sandboxed derivations build with the new edge.** `kernel-x64` and `kernel-virt` both `EXIT=0` on the x86_64 host, so the enlarged `Cargo.lock` and `port`'s link to `crypto`/`ds` survive the nix sandbox.
- **`main` published on approval.** `origin/main` moved `a64352625efe9cb8a924d7941c6be09c4f738a11` -> `5d1549af07de7220d6f48aac0d0db221aa7c541c` with one `git push origin main` after the owner replied "push-main"; the branch is unchanged at `405984d`.
- **Knowledge graph refreshed.** `graphify update .` rebuilt `graphify-out/` (18194 nodes, 26543 edges, 1201 communities); the output is gitignored and its failure would have been non-blocking.

## Task Commits

Each task was committed atomically:

1. **Task 1: OWNER STOP — branch push and the Linux checklist** — `a2ff50d` (docs: one-command owner-stop runner), `ca2b5ab` (the owner's `flake: re-derive boot-iso outputHash …`), `89ba121` and `405984d` (evidence: the two return-log commits)
2. **Task 2: the eleven verdicts recorded** — `5d1549a` (evidence: end-of-phase owner stop results)
3. **Task 3: the `main` push ruling** — `2bc5e09` (evidence: main pushed after the owner stop)

**Plan metadata:** the SUMMARY commit (docs: complete 04-15 plan)

_Note: this is measured from the plan ledger — 6 commits between `8b67e09` and `2bc5e09`, excluding the SUMMARY commit._

## Files Created/Modified

- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-RUNNER.sh` — the one-command runner that scripts sections 2 to 9 and fails fast (Task 1)
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP.md` — the authoritative nine-section checklist, with the runner instructions added (Task 1)
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/owner-stop/return/` — the owner's twelve logs plus `REPORT.txt`, and `run1/` holding the first round normalised in place (Task 2)
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/owner-stop/RESULTS.md` — `pushed-tip`, `hash-commit`, `new-outputHash`, the eleven `VERDICT`/`EVIDENCE` pairs, the three disclosures, and the `stop-main-push` ruling (Tasks 2 and 3)
- `flake.nix` — one line: the `outputHash` value, with a dated comment (the owner's commit `ca2b5ab`)

## Task 2 verdict table

| Verdict | Decisive returned line | `HEAD=` |
|---|---|---|
| `stop-branch-push` | `git ls-remote` -> `405984d`; base is the last Mac commit `a2ff50d` | — |
| `stop-host-test` | 36 `test result: ok` lines, every one `0 failed`; ends `140 passed; 0 failed` and `EXIT=0` | `89ba121` |
| `stop-host-test9p` | all five `aead_*` legs run, every `test result:` `0 failed`, `EXIT=0` | `89ba121` |
| `stop-kernel-x64-build` | `building '…-r9-kernel-x64-89ba121.drv'…`, `EXIT=0` | `89ba121` |
| `stop-kernel-virt-build` | `building '…-r9-kernel-virt-89ba121.drv'…`, `EXIT=0` | `89ba121` |
| `stop-bootiso-hash-commit` | own commit `ca2b5ab`, `flake.nix \| 3 ++-`, no attribution line | `ca2b5ab` |
| `stop-bootiso-build` | `boot-iso-after` `EXIT=0`; run 2 `boot-iso-first` `EXIT=0` | `ca2b5ab` / `89ba121` |
| `stop-bootiso-rebuild` | `checking outputs of '…-r9-boot-iso-ca2b5ab.drv'…`, `EXIT=0`, no `got:` | `ca2b5ab` |
| `stop-gate-kvm` | `GATE PASS quick x86_64 run=34940-1795575414`, `EXIT=0` | `89ba121` |
| `stop-gate-tcg` | `GATE PASS quick x86_64 run=45066-1795575611`, `EXIT=0`, native | `89ba121` |
| `stop-gate-full` | `GATE PASS full x86_64-linux run=55162-1795575813`, `EXIT=0` | `89ba121` |

## Decisions Made

- **All eleven PASS.** Every checklist step had a returned line, each at the pushed tip `89ba121` or at the owner's hash commit `ca2b5ab`, so README's stop rules did not apply and Task 3's decision was put to the owner.
- **The gates were accepted at `89ba121`.** The source tree there is identical to `ca2b5ab` (`git diff --quiet` over `port crypto ds xtask aarch64 x86_64 Cargo.lock flake.nix` exits 0 for both), and the `--rebuild` at `ca2b5ab` is green, which proves the kernel bytes did not move between the gated commit and the tip the gates measured.
- **`main` pushed only on the second approval**, with the exact command the plan fixes (`git push origin main`), and the sha shown to the owner (`5d1549a`) then confirmed by `git rev-parse origin/main`.

## Deviations from Plan

### Disclosed deviations (Task 2, recorded in `RESULTS.md`)

**1. Two return commits instead of at most one**
- **Found during:** Task 2
- **Issue:** The plan allows at most one return commit adding files only under `evidence/owner-stop/return/`. The owner's runner placed the first round under `evidence/return/` (`89ba121`) and the second under `evidence/owner-stop/return/` (`405984d`).
- **Fix:** Both commits add evidence files only — `git diff --quiet ca2b5ab 89ba121 -- port crypto ds xtask aarch64 x86_64 Cargo.lock flake.nix` and the same check for `89ba121..405984d` both exit 0. The run-1 files were normalised by `git mv` to `evidence/owner-stop/return/run1/` with contents unchanged (per-file sha256 checked), so the `boot-iso-first EXIT=1` / `got:` record that justified the hash commit is preserved.
- **Files modified:** `evidence/owner-stop/return/run1/*`
- **Committed in:** `405984d` (and the normalisation in `5d1549a`)

**2. The three gates were measured at the return commit, not the hash commit**
- **Found during:** Task 2
- **Issue:** The one-commit rule names `ca2b5ab` as the gated commit; the logs carry `HEAD=89ba121`.
- **Fix:** Disclosed with proof that the source tree is identical between the two commits and that the `boot-iso --rebuild` at `ca2b5ab` is green with the committed hash.
- **Committed in:** `5d1549a`

**3. Run 2's `REPORT.txt` records `hash-commit: none`**
- **Found during:** Task 2
- **Issue:** Run 2's first ISO build succeeded because the hash was already committed by run 1, so its report says no hash commit was needed.
- **Fix:** Run 1's `REPORT.txt` is recorded as authoritative for the hash commit (`ca2b5ab`) and the value; no code changed between the runs.
- **Committed in:** `5d1549a`

---

**Total deviations:** 3 disclosed (all evidence-shape, none affecting a verdict, `main`'s content, or the kernel). **Impact:** none — every verdict still rests on a returned line measured at a permitted commit.

## Issues Encountered

None. No stop rule fired. The only red line in the return is run 1's `boot-iso-first` fixed-output mismatch, which is the expected first-round result of the D-25 procedure and is the evidence that justified the hash commit.

## Pushes

Two pushes, both behind `blocking-human` checkpoints and both exactly as the plan fixes them:

1. **Branch, on Task 1's approval:** `git push origin HEAD:refs/heads/phase-04/aead-wire`; `origin/phase-04/aead-wire` now holds `405984d2c2bd391900c987668d14c78c23c23005` (the Mac commits, the owner's hash commit and the two return commits).
2. **`main`, on Task 3's second approval:** `git push origin main`; `origin/main` moved from `a64352625efe9cb8a924d7941c6be09c4f738a11` to `5d1549af07de7220d6f48aac0d0db221aa7c541c`.

No other ref was pushed, and no force was used. The post-push `VERDICT stop-main-push: PASS 5d1549a…` commit (`2bc5e09`) **stays local** until a later approved push, as the plan directs.

## Requirements

`WIRE-12` — the golden invariant "the system still boots to rio with the Phase 4 kernel" — is now evidenced on the owner's x86_64 Linux host at three gate tiers, with the byte-change procedure run in full.

## User Setup Required

None. The owner's host run is complete and recorded.

## Next Phase Readiness

- Phase 4's seventeen plans all have a SUMMARY; nothing reached `main` without the owner's approval.
- `origin/main` and `origin/phase-04/aead-wire` both carry the gated state, including the re-derived `outputHash`; the branch can be deleted later by the owner.
- One local commit (`2bc5e09`) is unpublished by design; the next approved push of `main` publishes it.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-24*

## Self-Check: PASSED

Every file this plan names exists on disk (`04-OWNER-STOP-RUNNER.sh`, `RESULTS.md`,
`return/REPORT.txt`, `return/run1/host-tests.log`, this SUMMARY), all six plan commits
(`a2ff50d`, `ca2b5ab`, `89ba121`, `405984d`, `5d1549a`, `2bc5e09`) are ancestors of HEAD,
`RESULTS.md` carries 11 `stop-` verdicts with 0 FAIL, `origin/phase-04/aead-wire` is an
ancestor of `main`, and `git rev-parse origin/main` is
`5d1549af07de7220d6f48aac0d0db221aa7c541c`.
