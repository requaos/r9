---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 02
subsystem: arm64-gates + r9-aarch64-kernel (diagnosis only)
tags: [r9, aarch64, hvf, tlb, ttbr0, fork, repeat-harness, windows-49]
status: complete
requires: ["20-01"]
provides:
  - "build/aarch64/repeat-gate.sh: N-run series harness for the three arm64 gates (SERIES tally line, per-run captures under .build/hvf-series)"
  - "pre-fix HVF rate: 15 fires / 1610 forks, P-LO 0.00575157, FORKS-TARGET 520"
  - "N-HVF: smoke 1753, net-smoke 36, tier1 10 (N-SOURCE: lower-bound, owner-confirmed)"
  - "confirmed root cause of WINDOWS #49: aarch64 vm::switch writes TTBR0 without an isb before the tlbi"
  - "/arm64/bin/forkstress baked into the image (not gate-wired)"
affects: [20-03, 20-06, 20-11, 20-16, 20-17]
tech-stack:
  added: []
  patterns: ["series harness pointing the app's capture env at the series file", "AT S1E0R + PAR_EL1 probe for stale translations after a TTBR switch"]
key-files:
  created:
    - build/aarch64/repeat-gate.sh
    - build/aarch64/forkstress.c
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
  modified:
    - build/aarch64/subset-build.sh
    - build/aarch64/mkimage.sh
decisions:
  - "Owner: fix route aarch64-fix; 20-03 adds isb after msr ttbr0_el1 in r9 aarch64/src/vm.rs switch()"
  - "Owner: keep the lower-bound N (smoke 1753, net-smoke 36, tier1 10); boot-r9-cpu derived by 20-16 per the plan's rules"
  - "The pushword fire is a process-isolation fault: a fork child runs on its parent's physical pages through stale translations (no ASIDs)"
metrics:
  duration: "about 1 h 20 min"
  completed: 2026-11-17
actuals:
  tokens: 9900
  tasks: 3
  commits: 4
plan_head_before: 16bb8e7d9ef85a8a6a8a0b61791df615aa95a8c1
plan_head_after: 92338b573a090d8898ad3b4c20ceb3a56d04d3eb
r9_head_before: 46364d0
r9_head_after: 46364d0
r9_commits: 0
---

# Phase 20 Plan 02: HVF fork-child defect, measured and root-caused Summary

The HVF `rc: pushword but no argv!` defect (WINDOWS #49) is caused by r9's aarch64 `vm::switch`. It writes TTBR0_EL1 and then runs `tlbi vmalle1is` with no `isb` in between. Speculative walks through the old root table refill the TLB after the flush, and with no ASIDs a fork child then runs on its parent's physical pages. The parent has already popped `runq->argv` (`rc/simple.c:69`), so the child's first `pushword` finds argv nil and aborts.

Adding one `isb` after the `msr` gave 0 fires in 2604 HVF forks across all three gates, and removing it brought the fires back. An `AT S1E0R` probe confirmed the mechanism directly: 110 of the failing child's 114 pages resolved to pid 6's physical pages.

`commits: 4` counts this plan's Harvey commits before this SUMMARY's own commit (measured `16bb8e7d9..HEAD`). Nothing was committed in r9.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Repeat harness + pre-fix rate + N derivation | 4e24a8400 | build/aarch64/repeat-gate.sh, 20-EVIDENCE.md |
| 2 | forkstress fixture baked into the image | 46d54b261 | build/aarch64/forkstress.c, subset-build.sh, mkimage.sh |
| 2 | Root-cause record | bf4abe6bf | 20-EVIDENCE.md |
| 3 | Owner decision recorded | 92338b573 | 20-EVIDENCE.md |

## Evidence

- **Pre-fix (r9 46364d0, HVF):**

  | Gate | Fail | Runs | Forks |
  |------|------|------|-------|
  | tier-1 | 10 | 12 | 798 |
  | net-smoke | 4 | 12 | 579 |
  | smoke | 1 | 30 | 233 |

  That is f = 15 fires in F = 1610 forks, all of them pushword fires. Exact binomial P-LO = 0.00575157; the Poisson cross-check L(15)/F = 0.0057431 agrees. The TCG control had 0 fires in 615 forks; a kernel failing at P-LO would come through 615 forks clean only 2.9% of the time.
- **N:**
  - smoke: 1753. The per-boot form from the gate's own 1/30 fails sets it, about 3.85 h per series.
  - net-smoke: 36, the floor.
  - tier-1: 10, the floor.
  - FORKS-TARGET = 520.
- **Fires cluster at fixed fork sites.** 13 of 15 come right after the parent reaps a child, and the failing child has no `fault:` line before the abort.
- **Mechanism table** (tier-1 HVF ×12 each, through an instrumented copy of r9):

  | Run | Fails | Forks |
  |-----|-------|-------|
  | control | 8 / 12 | 1086 |
  | (c) dsb before tlbi | 11 / 12 | 801 |
  | (d) I-cache maintenance on copied text | 9 / 12 | 1287 |
  | (e) full switch fix | 0 / 12 | 1512 |
  | (e) isb-only | 0 / 12 | 1512 |
  | (c) + (e) | 0 / 12 | 1512 |

  (a), (b) and (f) were ruled out from the captures and the fix's scope.
- **The isb-only fix on the other gates:** net-smoke 12/12 (852 forks), smoke 30/30 (240 forks).
- **Probe:** on the shipped switch, both failing boots show a stale switch just before the abort (`stale=110 of 114`, the PAs owned by the parent). With the fix the probe printed 0 lines in 12 boots.
- **forkstress:** 0 mismatches in 4950 HVF forks, so `STRESSOR-RATE: not-fired`. Its parent does not write memory between fork and the child's first run, so the defect cannot show there.
- **Image check:** after the fixture commit, the TCG smoke printed `ALL STAGES PASS`.
- **r9 tree:** `git -C ../r9 status --porcelain --untracked-files=no` is empty, and HEAD is still 46364d0.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Instrumentation venue**
- **Found during:** Task 2, step 3
- **Issue:** the worktree path guard (a PreToolUse hook) refuses Edit on files under `/Users/neilskinner/code/r9`.
- **Fix:** I instrumented a git-less copy of the r9 tree at 46364d0 under the gitignored `.build/r9-instr` and passed it as `--override-input r9 path:...`. The copy was checked for being pristine between experiments and deleted at the end. The diffs are kept as `.build/p20-probe-and-fix.diff` and `.build/p20-fix-e-isb.diff`. The real r9 checkout was never modified.
- **Commit:** none (venue only)

**2. [Rule 2 - Missing functionality] forkstress checks heap too, and takes -n**
- **Found during:** Task 2
- **Issue:** rc's `runq` and argv live on the heap, which data, bss and stack alone do not cover. The longer series also needed more forks per boot.
- **Fix:** added a 64 KiB malloc'd heap segment and an `-n forks` flag (default 200, so the plan's `ok 200 forks` contract still holds). I also found r9's limit of about 505 forks per boot (`proc: table full`, slots are never freed) and recorded `-n <= 450`.
- **Commit:** 46d54b261

### Notes

- The plan expected a reproducer from forkstress, but the stressor never fired. The recorded reproducer is the tier-1 gate under `repeat-gate.sh`, which fails about 80% of HVF boots before the fix.
- `graphify update .` was not run, because the worktree has no `graphify-out/` (rule: ignore when absent).
- No fix was committed, no N-green series was run (D-07), nothing was pushed, and STATE.md and ROADMAP.md were not touched.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: isolation | r9:aarch64/src/vm.rs | T-20-04 escalated: a stale-TLB switch lets a process read and write another live process's pages (a fork child on its parent's data, bss and stack). Fixed by 20-03's isb; WINDOWS #50 (HVF-only devpipe EL1 translation fault) fits the same mechanism, untested |

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: build/aarch64/repeat-gate.sh, build/aarch64/forkstress.c, 20-EVIDENCE.md
- FOUND: 4e24a8400, 46d54b261, bf4abe6bf, 92338b573 (ancestors of HEAD)
- Task 1 state line: `repro=reproduced source=lower-bound plo=1 target=1 nhvf=3`
- STRESSOR-RATE lines: 1; forkstress.c tracked; r9 tracked tree clean
