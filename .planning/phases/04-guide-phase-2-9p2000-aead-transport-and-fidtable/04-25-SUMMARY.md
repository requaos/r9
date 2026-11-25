---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 25
subsystem: planning-evidence
tags: [9p2000-aead, gap-closure, owner-stop, x86_64, boot-iso, outputHash, boot-to-rio, kvm, tcg, main-push]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: the reviewed gap-closure port/src change (6fd201c) and the @04-24 kernel byte evidence set
provides:
  - the eleven second-owner-stop verdicts for the gap-closure kernel on the x86_64 Linux host
  - the proof that the committed boot-iso outputHash still reproduces from the gap-closure kernel
  - a pushed `main` at bb68d98 holding the gated gap-closure state
affects: [phase-05]

# Actuals (#2632) — same estimateTokens scale as the plan's estimate (chars/4 over the realized diff).
actuals:
  tokens: 5910
  tasks: 3
  commits: 5
  plan_head_before: 611dbb435bf6ca6d725d97c07efe2b3bf5cac58c
  plan_head_after: 4d420c9fcd8c67a0ee94cfcbab07662b901dae1b

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A fixed-output hash that reproduces is proven by `nix build --rebuild`, not by a plain build, because a plain build can return a cached store path"
    - "A second owner stop records its verdicts beside the first stop's; the earlier return is disclosed as superseded, never rewritten"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/owner-stop-2/RESULTS.md
  modified:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-2-RUNNER.sh

key-decisions:
  - "Eleven second-stop verdicts PASS at 0dd1037, all decided from lines the owner returned; none was inferred on the Mac."
  - "No flake.nix hash commit was made: the committed outputHash sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I= reproduces from the gap-closure kernel, and --rebuild proves it rather than a cached fixed-output path."
  - "The gap fixes moved whole-ELF hashes but not the code they compile to; the delta is debug information (DWARF, line tables, file paths), which the sandboxed kernel derivation does not carry into the ISO. That is why the outputHash did not move."
  - "Round 1's return is disclosed as superseded because its `boot-iso-first` conclusion rested on a cached fixed-output path; round 2's runner (`0dd1037`) forces `--rebuild`."
  - "`main` was pushed on the owner's second explicit approval, publishing 68 commits; the `stop2-main-push` verdict commit stays local until a later approved push."

patterns-established:
  - "A second owner stop names its superseded round explicitly and states which round every verdict measures"
  - "The push of main is a separate blocking-human decision from the branch push, taken only after the results are green"

requirements-completed: [WIRE-12]

coverage:
  - id: D1
    description: "Eleven second-owner-stop verdicts recorded from the owner's returned round-2 logs, each with its decisive line and HEAD= value; none FAIL."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "grep -cE '^VERDICT stop2-(branch-push|host-test|host-test9p|kernel-x64-build|kernel-virt-build|bootiso-hash-commit|bootiso-build|bootiso-rebuild|gate-kvm|gate-tcg|gate-full): (PASS|FAIL)$' evidence/owner-stop-2/RESULTS.md -> 11; FAIL count -> 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "The golden invariant for the gap-closure kernel: boot to rio under KVM, the native TCG gate, and the full gate set, all at 0dd1037 with GATE PASS lines."
    requirement: WIRE-12
    verification:
      - kind: e2e
        ref: "round2/gate-kvm.log 'GATE PASS quick x86_64 run=88132-1795622365' EXIT=0; round2/gate-tcg.log 'GATE PASS quick x86_64 run=98267-1795622561' EXIT=0; round2/gate-full.log 'GATE PASS full x86_64-linux run=108390-1795622765' EXIT=0"
        status: pass
    human_judgment: false
  - id: D3
    description: "`main` published only on the owner's explicit second approval, at exactly the sha shown, with the verdict commit left local."
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "git push origin main (5d1549a..bb68d98); git rev-parse origin/main == bb68d988752ccb616a980b943ea86aecca8f25be == the shown sha; RESULTS.md 'VERDICT stop2-main-push: PASS bb68d98...'"
        status: pass
    human_judgment: false

duration: 1h 4m
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 25: the second owner stop — x86_64 byte-change procedure, boot to rio for the gap-closure kernel, and `main` Summary

**Eleven second-owner-stop verdicts PASS on the x86_64 Linux host at `0dd1037` — both host suites, both sandboxed kernels, `boot-iso` build and `--rebuild` on the unchanged `outputHash`, and the KVM, native-TCG and full gates — after which the owner approved publishing `main` at `bb68d98`.**

## Performance

- **Duration:** 1 h 4 min (10:14:54Z to 16:18:35Z)
- **Started:** 2026-11-25T10:14:54Z (first returned log commit `bdee8a8`)
- **Completed:** 2026-11-25T16:18:35Z
- **Tasks:** 3/3 (Task 1 owner stop 2, Task 2 verdicts, Task 3 `main` push decision)
- **Files modified:** 2 tracked files by this plan's executor (`RESULTS.md` created, runner corrected); the owner's return added 20 log/REPORT files under `evidence/owner-stop-2/return/`

## Accomplishments

- **All eleven second-stop verdicts PASS, none FAIL.** `stop2-branch-push`, `stop2-host-test` (140 passed, 0 failed), `stop2-host-test9p` (the `aead_*` legs on x86_64), `stop2-kernel-x64-build`, `stop2-kernel-virt-build`, `stop2-bootiso-hash-commit`, `stop2-bootiso-build`, `stop2-bootiso-rebuild`, `stop2-gate-kvm`, `stop2-gate-tcg`, `stop2-gate-full`.
- **The golden invariant is shown again for the gap-closure kernel.** The three gate pass lines, each at `HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348` with `EXIT=0`:

  ```text
  GATE PASS quick x86_64 run=88132-1795622365        (gate-kvm,  log gate-kvm.log)
  GATE PASS quick x86_64 run=98267-1795622561        (gate-tcg,  log gate-tcg.log, native on the host)
  GATE PASS full x86_64-linux run=108390-1795622765  (gate-full, log gate-full.log)
  ```

- **The `boot-iso` `outputHash` did not change, and that is measured, not assumed.** committed value `sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=`; `new-outputHash: unchanged`; `hash-commit: none`. The round-2 runner forces `nix build .#boot-iso --rebuild`, which actually checks the outputs of `p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv` and exits 0 with no `hash mismatch` and no `got:` line.
- **`main` is published on the owner's second explicit approval.** `git push origin main` ran exactly once, `5d1549a..bb68d98`, publishing **68 commits**; `git rev-parse origin/main` is `bb68d988752ccb616a980b943ea86aecca8f25be`, equal to the sha shown for the push.

## The eleven verdicts and their decisive lines

| Verdict | HEAD measured | Decisive returned line |
|---|---|---|
| `stop2-branch-push` | `0dd1037` | `REPORT.txt`: `pushed-tip: 0dd1037…` = `host-tip: 0dd1037…`; remote branch is `194c71d` whose parent is `0dd1037` |
| `stop2-host-test` | `0dd1037` | `test result: ok. 140 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`, then `EXIT=0` |
| `stop2-host-test9p` | `0dd1037` | last `aead_*` leg `Running tests/aead_negotiate.rs (target/debug/deps/aead_negotiate-f57b1c9a8f304d85)`, then `EXIT=0` |
| `stop2-kernel-x64-build` | `0dd1037` | `building '/nix/store/8rwc8mgpnmcdnnl567p9z7lpdfhb19qs-r9-kernel-x64-0dd1037.drv'...`, then `EXIT=0` |
| `stop2-kernel-virt-build` | `0dd1037` | `building '/nix/store/dypvcs2cl4rsq865hbiwi3gxl7czbb0s-r9-kernel-virt-0dd1037.drv'...`, then `EXIT=0` |
| `stop2-bootiso-hash-commit` | `0dd1037` | no change required — the rebuild reproduced the committed hash; `hash-commit: none`, `new-outputHash: unchanged` |
| `stop2-bootiso-build` | `0dd1037` | `building '/nix/store/p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv'...`, then `EXIT=0` |
| `stop2-bootiso-rebuild` | `0dd1037` | `checking outputs of '/nix/store/p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv'...`, then `EXIT=0`, no `got:` |
| `stop2-gate-kvm` | `0dd1037` | `GATE PASS quick x86_64 run=88132-1795622365`, then `EXIT=0` |
| `stop2-gate-tcg` | `0dd1037` | `GATE PASS quick x86_64 run=98267-1795622561`, then `EXIT=0` |
| `stop2-gate-full` | `0dd1037` | `GATE PASS full x86_64-linux run=108390-1795622765`, then `EXIT=0` |

Plus the final ruling:

```text
VERDICT stop2-main-push: PASS bb68d988752ccb616a980b943ea86aecca8f25be
```

- `pushed-tip` **`0dd1037dfe2067778fe4f731f5b30ea3d355d348`**
- `gated-commit` **`0dd1037dfe2067778fe4f731f5b30ea3d355d348`** (the pushed tip is the gated commit — no hash commit sits between section 5 and section 8)
- `hash-commit` **none**
- `old-outputHash` **`sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=`**; `new-outputHash` **unchanged**

## Why the ISO hash did not move

The gap fixes did move the Mac debug ELFs — `q35churnshell` went `c850fb64…748957` at `@04-14` to `2066521c…f24928` at `@04-24`. What did not move is the code: the defined text-symbol totals (`755220` aarch64, `760137` x86_64) and the FP/SIMD scan counts (`309`, `6`) are identical on both sides of the change. The whole-ELF delta is therefore debug information — DWARF, line tables, file paths — which the sandboxed kernel derivation does not carry into the image. Round 2 proves this rather than asserting it: `--rebuild` at `0dd1037` checks the derivation's outputs and exits 0.

## Disclosures

1. **`bdee8a8` — round-1 return logs, superseded.** Round 1's `boot-iso-first.log` ended `EXIT=0` at `HEAD=611dbb4…`, but a plain `nix build .#boot-iso` returns the fixed-output path already in the store from the first stop, so that conclusion rested on a cached path. Round 2's runner forces `--rebuild`; **round 2 supersedes round 1 for every verdict in `RESULTS.md`.**
2. **`0dd1037` — tooling-only runner fix, ahead of section 5.** This is the pushed tip every round-2 log measures. Its diff touches only `04-OWNER-STOP-2-RUNNER.sh` — planning tooling, not source. `git diff --quiet 611dbb4 194c71d -- port crypto ds xtask aarch64 x86_64 riscv64 Cargo.lock flake.nix` exits 0, so no kernel byte moved between the round-1 tip and this round's tip.
3. **`194c71d` — round-2 return logs.** Adds only the ten files under `evidence/owner-stop-2/return/round2/`; the only commit above the pushed tip, and it changes no source.
4. **No hash commit exists.** `REPORT.txt` records `hash-commit: none` and `new-outputHash: unchanged`; the branch holds `0dd1037` plus the return commit only.

## Both pushes, and what stays local

| Push | Approval | Command | Result |
|---|---|---|---|
| `phase-04/aead-wire-gaps` | owner replied "push" at Task 1 | `git push origin HEAD:refs/heads/phase-04/aead-wire-gaps` | remote branch now `194c71d` = Mac tip `0dd1037` + the round-2 return commit |
| `main` | owner replied "push-main" at Task 3 | `git push origin main` | `5d1549a..bb68d98`, **68 commits published** |

- **The post-push verdict commit stays local.** `4d420c9` (`evidence(04-25): main pushed after the second owner stop`) was made *after* the push, so `origin/main` is `bb68d98` and local `main` is one evidence commit ahead. It is published only by a later approved push.
- No other ref was pushed, and no tag was created.
- `origin/main` was `5d1549af07de7220d6f48aac0d0db221aa7c541c` before this plan and is `bb68d988752ccb616a980b943ea86aecca8f25be` after. `origin/phase-04/aead-wire` is unchanged at `405984d`.

## What is still unpushed

- Local `main` is ahead of `origin/main` only by this plan's close-out commits — the `stop2-main-push: PASS` verdict commit `4d420c9`, this SUMMARY, and the tracking commits that follow it. All are docs/evidence only; no source and no `flake.nix` change.
- `origin/phase-04/aead-wire-gaps` (`194c71d`) and `origin/phase-04/aead-wire` (`405984d`) are on the remote. The owner can delete the transport branches now that `main` carries the gated state.

## Task Commits

Each task was committed atomically:

1. **Task 1: owner stop 2 — branch push and the Linux-host checklist** — `bdee8a8` (evidence, round-1 return logs) and `0dd1037` (fix, runner `--rebuild` correction; owner stop)
2. **Task 2: take the owner's return — the eleven verdicts** — `194c71d` (evidence, round-2 return logs) and `bb68d98` (evidence, second owner stop results)
3. **Task 3: approve pushing `main`** — `4d420c9` (evidence, the verdict line; local only)

**Plan metadata:** the SUMMARY commit (docs: complete 04-25 plan)

_Measured from the plan ledger: 5 commits between `611dbb4` and `4d420c9`._

## Files Created/Modified

- `evidence/owner-stop-2/RESULTS.md` — the eleven `VERDICT`/`EVIDENCE` pairs, the `pushed-tip`/`gated-commit`/`hash-commit`/`old-outputHash`/`new-outputHash` lines, "Why the ISO hash did not move", four Disclosures, and the closing `VERDICT stop2-main-push`
- `04-OWNER-STOP-2-RUNNER.sh` — corrected at `0dd1037` so it forces `nix build .#boot-iso --rebuild` and pushes its own return commit
- `evidence/owner-stop-2/return/` and `.../round2/` — the owner's returned logs (20 files), each beginning `HEAD=` and ending `EXIT=`

## Decisions Made

- **Verdicts from returned lines only.** Every PASS quotes a returned `HEAD=`/`EXIT=` line or a git check; nothing was inferred on the Mac.
- **"No change" is a legitimate hash outcome.** The page allows it, and `--rebuild` is what makes it trustworthy: the value is checked against a real build, not a cached path.
- **Round 1 disclosed, not deleted.** Its cached-path weakness is stated in Disclosures and round 2 supersedes it by name.
- **`main` pushed as one decision, after green.** 68 commits went out at once because a push of `main` publishes every unpushed commit; the checkpoint printed the list and the owner ruled from it.

## Deviations from Plan

None - plan executed exactly as written. No stop rule fired: all eleven verdicts are PASS, so Task 3's decision was taken on green results.

## Issues Encountered

- **Round 1's `boot-iso` conclusion was unsound and was caught before it reached a verdict.** A plain `nix build .#boot-iso` returns the fixed-output path already in the store, so round 1 could report "no hash change" without rebuilding. Handled by the plan's own repeated-round rule: the runner was corrected (`0dd1037`) to force `--rebuild`, and the whole checklist was re-run as round 2. Round 1 is disclosed as superseded.
- **The plan ledger for `04-25` was absent.** Task 3 reconstructed it from `611dbb4`, the 04-24 SUMMARY commit that is the plan's base, so the `actuals.commits` figure measures the whole plan rather than only the continuation.

## User Setup Required

None. The owner's remaining actions are administrative:

- **Set `workflow.use_worktrees` back to `true`** in `.planning/config.json` (this plan ran with it `false` so the owner stop could use the main working tree).
- Optionally delete the transport branches `phase-04/aead-wire-gaps` and `phase-04/aead-wire` on the remote.

## Next Phase Readiness

- The golden invariant is in evidence for the gap-closure kernel at `0dd1037`, and the `boot-iso` `outputHash` still matches it. `main` is published at `bb68d98`.
- Phase 4's `gaps_found` state can now be re-verified: all 19 review findings were fixed in plans 04-18 to 04-23, reviewed in 04-23, closed in 04-24, and the kernel has been re-gated here.
- Still not run for Phase 4 and carried forward: `04-SECURITY.md` (`/gsd-secure-phase 4`) and the Nyquist audit (`/gsd-validate-phase 4`); UI review does not apply.
- Local `main` is one commit ahead of `origin/main` (`4d420c9`); carry that in STATE.md for the next approved push.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

Every file this plan names exists on disk (`04-25-SUMMARY.md`, `evidence/owner-stop-2/RESULTS.md`,
`evidence/owner-stop-2/return/round2/REPORT.txt`), all five plan commits (`bdee8a8`, `0dd1037`,
`194c71d`, `bb68d98`, `4d420c9`) are ancestors of HEAD, `RESULTS.md` carries eleven PASS
`stop2-` verdicts with no FAIL and ends on `VERDICT stop2-main-push: PASS
bb68d988752ccb616a980b943ea86aecca8f25be`, and `git rev-parse origin/main` equals that same sha,
with local `main` exactly one commit ahead (`4d420c9`, held local as the plan requires).
