---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 03
subsystem: r9-aarch64-kernel + r9-port-pagealloc + arm64 tier-1 gate
tags: [r9, aarch64, hvf, tlb, ttbr0, isb, bitmapalloc, fork, forkstress, windows-49]
status: complete
requires: ["20-02"]
provides:
  - "r9 cb9cde3: isb after msr ttbr0_el1 / ttbr1_el1 in aarch64 vm::switch (WINDOWS #49 fixed)"
  - "r9 8958c9e: port bitmapalloc deallocate clears the bit it checked, plus a unit test (PORT-SRC-EDIT: YES)"
  - "forkstress with the parent-mutation trigger, pinned in the tier-1 sweep as T1-OK forkstress (r9 venue, gate-required)"
  - "post-fix series at r9 8958c9e: HVF smoke 1753/1753, net-smoke 36/36, tier-1 10/10; TCG 3/3 each"
affects: [20-06, 20-11, 20-16, 20-17]
tech-stack:
  added: []
  patterns: ["fixture marker kept in the fixed R9_EXPECT_OK names, outside the computed tier set", "parent rewrites its memory between fork and wait so a child on shared pages shows a mismatch"]
key-files:
  created: []
  modified:
    - ../r9/aarch64/src/vm.rs
    - ../r9/port/src/bitmapalloc.rs
    - build/aarch64/forkstress.c
    - build/aarch64/tier1-sweep.rc
    - nix/tier1-r9.sh
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
decisions:
  - "Fix site for WINDOWS #49 is r9 aarch64/src/vm.rs switch(): isb after the TTBR0 write (confirmed) and after the TTBR1 write (consistency); the dsb ishst hardening was not added"
  - "Owner approved a port/src edit: bitmapalloc deallocate cleared bit_idx of byte 0, which leaked every freed page and handed out the initrd pages; 20-06 redoes the x86_64 boot-iso FOD"
  - "Tier-1 N stays 10: STRESSOR-RATE is not-fired, so forkstress forks are not counted toward forks per boot"
metrics:
  duration: "about 12 h 18 min wall clock (23:14Z to 11:32Z); about 4 h 14 min of it is the post-fix series, and part of the rest was idle time waiting on the owner at the Task 2 checkpoint"
  completed: 2026-11-18
actuals:
  tokens: 6400
  tasks: 3
  commits: 5
plan_head_before: 92f2f6882cf6397d0c09df4fe8a51e8b41059964
plan_head_after: 4e856d439d3a7adf5791d4d2c63ccf86ad5fd82a
r9_head_before: 46364d0
r9_head_after: 8958c9e
r9_commits: 2
---

# Phase 20 Plan 03: vm::switch isb fix, bitmapalloc leak fix, forkstress pin, N-run HVF series Summary

The HVF fork-child defect (WINDOWS #49) is fixed at the approved site. An `isb` now follows each TTBR write in r9's aarch64 `vm::switch`, so the TLBI can no longer race a walk that is still using the outgoing root table. The pre-fix reproducer gave 11 fires in 995 forks; with the fix it gave 0 in 1512.

Pinning the fix in the tier-1 sweep exposed a second r9 defect. `BitmapPageAlloc::deallocate` cleared the wrong bit: no page was ever really freed, and the initrd pages at 0x48000000 were handed out. The owner approved a fix in `port/src/`. With both fixes, all three gates were green on HVF for their full N, 1753 + 36 + 10 runs with 0 failures, and the TCG control legs were green too.

`commits: 5` counts this plan's Harvey commits before this SUMMARY's own commit (measured `92f2f6882..HEAD`). The r9 commits are `cb9cde3` and `8958c9e`.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fix the defect at the approved site | r9 `cb9cde3`, Harvey `23ac16a44` | r9 `aarch64/src/vm.rs`, 20-EVIDENCE.md `## Fix` |
| 2 | Checkpoint: the pin exposes the bitmapalloc bug | Harvey `6fb504428` | 20-EVIDENCE.md, a temporary WIP patch |
| 2 | Port fix (owner option A) | r9 `8958c9e`, Harvey `fce2d744f` | r9 `port/src/bitmapalloc.rs`, 20-EVIDENCE.md |
| 2 | forkstress pinned under the tier-1 sweep | Harvey `905c2ef9d` | build/aarch64/forkstress.c, tier1-sweep.rc, nix/tier1-r9.sh (WIP patch deleted) |
| 3 | Post-fix series | Harvey `4e856d439` | 20-EVIDENCE.md `## Post-fix series (local override)` |

## Evidence

- **Reproducer** (tier-1 HVF ×12):

  | r9 | Pass | Fail | Pushword | Forks |
  |----|------|------|----------|-------|
  | `46364d0` (before) | 1 | 11 | 11 | 995 |
  | `cb9cde3` (after) | 12 | 0 | 0 | 1512 |

  1512 is at least `FORKS-TARGET` (520). It is also more than this re-run's own fork length: r_lo = 0.006212, which asks for 481 forks.
- **FIX-SITE:** `aarch64 r9/aarch64/src/vm.rs`. The virtshell build is OK. The count of `pushword but no argv` in `sys/src/cmd/rc/exec.c` is still 2.
- **bitmapalloc:**
  - The new test `bitmappagealloc_deallocate_outside_byte_0` fails on the old line and passes on the fix.
  - `cargo test -p port --no-fail-fast` passed 232 with 0 failures, and `cargo xtask check` passed.
  - Confirmed end to end in an instrumented copy before the owner decision: the tier-1 gate then printed ALL STAGES PASS.
- **Pin:**
  - At r9 `8958c9e`, the tier-1 HVF gate printed ALL STAGES PASS, with `T1-OK forkstress` once and no `T1-FAIL forkstress` or `NotAllocated` lines.
  - The expected set is now 38 OK markers and 23 SKIP markers, which is 20-01's computed set plus the fixed name `forkstress`.
  - With the entry silenced, the gate failed: `tier1-r9: post-pass FAILED — missing T1-OK forkstress`. The silencing change was reverted.
  - The commit touches nothing under `build/aarch64/tiers`, `subset-exec.sh` or `build/mm`.
- **Series at r9 `8958c9e`** (verbatim):

  ```
  SERIES boot-r9-tier1 accel=hvf runs=10 pass=10 fail=0 pushword=0 forks=3280
  SERIES boot-r9-net-smoke accel=hvf runs=36 pass=36 fail=0 pushword=0 forks=2556
  SERIES boot-r9-smoke accel=hvf runs=1753 pass=1753 fail=0 pushword=0 forks=14024
  SERIES boot-r9-tier1 accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=984
  SERIES boot-r9-net-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=213
  SERIES boot-r9-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=24
  ```

  The HVF gates made 17860 forks with 0 fires; this excludes the 2000 forkstress forks, because the stressor rate is `not-fired`. That is at least `FORKS-TARGET` (520). The series ran with `R9_STOP_ON_FAIL=1`, and no run failed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fix file differs from `files_modified`**
- **Found during:** Task 1
- **Issue:** the plan anticipated `aarch64/src/proc.rs`, but the approved site is `aarch64/src/vm.rs` `switch()`.
- **Fix:** I edited `vm.rs` as the 20-02 owner decision directs. `proc.rs` is untouched.
- **Commit:** r9 cb9cde3

**2. [Rule 2 - Missing functionality] forkstress gets the 20-02 trigger**
- **Found during:** Task 2
- **Issue:** 20-02 showed the stressor could not see the defect, because its parent never changed memory between `fork` and the child's first run.
- **Fix:** after `fork`, the parent rewrites all four arrays with a second pattern before `wait`, and the DETAIL line counts `parent=` words. The output contract (`ok 200 forks`, `MISMATCH seg=…`) is unchanged.
- **Commit:** 905c2ef9d

### Owner-approved deviation (Rule 4 checkpoint)

**3. Second r9 defect in `port/src/bitmapalloc.rs`, the PORT-SRC-EDIT**
- **Found during:** Task 2, the first HVF run of the pinned sweep
- **Issue:** `deallocate` cleared bit `bit_idx` of byte 0 instead of `8 * byte_idx + bit_idx`. Every freed page leaked. Once the leak passed 128 MiB, the initrd pages at 0x48000000 were handed out as a root table and kernel stack. The paqfs image was overwritten and the namespace stopped resolving (`namec: does not exist`).
- **Fix:** a one-line correction plus a unit test, approved by the owner (option A).
- **Impact:** x86_64 shares the allocator, so **20-06 must redo the x86_64 boot-iso FOD**, and the Harvey push stays held.
- **Commit:** r9 8958c9e

### Notes

- The 20-02 root cause lists two `dsb ishst` additions as hardening. They were not made: the owner's instruction limited the vm.rs change to the two `isb`s.
- An earlier full port-suite run showed the new test failing. That was a stale test binary, because the file was restored within the same mtime tick after the red check. After `touch`, the suite was re-run green.
- I did not re-measure `STRESSOR-RATE` for the triggered forkstress. Under the plan's rule it stays `not-fired`, so its forks are not counted toward N or `FORKS-TARGET`.
- Nothing was pushed. The pre-existing modified troff and postscript files were not staged.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: isolation | r9:port/src/bitmapalloc.rs | A kernel page allocator handed out reserved pages (the initrd image) and leaked every freed page. Fixed in 8958c9e. Before the fix, any long boot could overwrite the boot filesystem image or the DTB pages on aarch64, and the low 8 pages of each 128 MiB region on x86_64 |

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: forkstress.c, tier1-sweep.rc, nix/tier1-r9.sh, 20-EVIDENCE.md, this SUMMARY; the WIP patch is deleted
- FOUND (ancestors of HEAD): Harvey 23ac16a44, 6fb504428, fce2d744f, 905c2ef9d, 4e856d439; r9 cb9cde3, 8958c9e
- Task 3 verify: smoke N=1753 hvf=1 tcg=1, net-smoke N=36 hvf=1 tcg=1, tier-1 N=10 hvf=1 tcg=1
- r9 tracked tree clean
