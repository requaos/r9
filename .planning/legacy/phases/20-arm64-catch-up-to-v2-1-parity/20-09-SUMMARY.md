---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 09
subsystem: r9 aarch64 churn leak gate (kernel driver, xtask profile, arm64 payloads)
tags: [r9, aarch64, churn, heap, leak-gate, exit-path, xtask, virtchurn, WINDOWS-44]
status: complete
requires: ["20-08"]
provides:
  - "r9 ff1b155: aarch64 churngate driver (platform/virt/churngate.rs), exec_prologue split, dev_churn proc helpers, heap_stats/heap_census wrappers, the per-exit heap marker, cfg(dev_churn) in the workspace check-cfg, config_virtchurn.toml"
  - "r9 3aa3eeb: xtask virtchurn profile + VIRT_CHURN_* markers, the churn post-pass shared by q35churn and virtchurn, churnmark/churnsem in INITRD_FILES"
  - "Harvey 783b13ed6: build/aarch64/churnmark.c, churnsem.c staged by stage-build.sh"
  - "The pre-fix reading: A5 holds for the drain-free exit path (16805 B/exec, red); green at the exit path as it stands before 20-10 (136-154 B/exec)"
affects: [20-10, 20-11]
tech-stack:
  added: []
  patterns:
    - "a kernel-context driver proc on aarch64: seed its swtch context and set blocked_in_kernel so pick_next's first resume goes through swtch, not trap_return"
    - "an exec'd child built in kernel context needs no first-resume seeding on aarch64: new_user_proc leaves it Ready, and pick_next trap_returns it into the frame exec_into seeded"
    - "an r9 boot-test run from a worktree under the session scratchpad never connects: the serial socket's absolute path exceeds the macOS 104-byte sun_path limit, so qemu waits forever; use a short path such as Harvey's .build/"
key-files:
  created:
    - ../r9/aarch64/src/platform/virt/churngate.rs
    - ../r9/aarch64/lib/config_virtchurn.toml
    - build/aarch64/churnmark.c
    - build/aarch64/churnsem.c
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-09-SUMMARY.md
  modified:
    - ../r9/aarch64/src/main.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/allocator.rs
    - ../r9/aarch64/src/platform/virt/mod.rs
    - ../r9/Cargo.toml
    - ../r9/xtask/src/main.rs
    - build/aarch64/stage-build.sh
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/deferred-items.md
key-decisions:
  - "The churn ceiling on aarch64 is x86_64's CEILING_BYTES_PER_EXEC = 1772, unchanged (pinned by the derive_ceiling test)"
  - "The watchdogs keep x86_64's wall-clock budgets at virt's 1000 Hz tick (WATCH_TICKS 3000, DRAIN_TICKS 30000)"
  - "The pre-fix gate was also run on r9 b82ee9d + the gate, because the debugger's 3a1c9e5/5893ae9 had already ported the exit drains the plan expected to be missing"
  - "The q35churn post-pass was factored into churn_post_pass over a ChurnMarkers set, so both profiles run the same checks"
requirements-completed: []  # contributes to ARM-07 (the churn pin); the requirement completes with later plans
metrics:
  duration: "about 41 min (2026-11-19T23:51Z to 2026-11-20T00:32Z)"
  completed: 2026-11-20
actuals:
  tokens: 23225
  tasks: 3
  commits: 2
plan_head_before: 7c1270bafc0af3397a44c6f340e35790405d6206
plan_head_after: ee8a54f30427c0fe53564409e3692bcefff62ec1
---

# Phase 20 Plan 09: the churn leak gate ported to aarch64 as the `virtchurn` boot-test. It reads 16.4 KiB per exec (red) on the exit path without the drains, and 136 to 154 bytes per exec (green) at the current r9 Summary

r9 now has an aarch64 churn gate, `cargo xtask boot-test --arch aarch64 --config virtchurn`. It is the x86_64 17-02 driver ported:
- a kernel-context proc runs 1400 `churnmark` execs, with a `churnsem` RFMEM contention generation every 35th iteration;
- it samples the heap bump cursor and asserts the steady-leg slope against x86_64's 1772 bytes/exec ceiling;
- the xtask post-pass re-derives that slope from the raw samples.

The plan expected to measure an exit path that still lacked the drains. That expectation was out of date: the debugger's r9 `3a1c9e5` and `5893ae9`, already in the pin `27f38a3`, are the exit-half drains 20-10 Task 1 plans to port. So the gate ran in two places:
- **At r9 `3aa3eeb`** (the exit path as it stands before 20-10): 3 of 3 PASS, measured 147, 154 and 136 bytes per exec.
- **At r9 `b82ee9d` + the gate** (the exit path without the drains, in a scratch worktree): 3 of 3 FAIL. The slope is 16805 bytes per exec in the baseline leg, and the kernel dies near exec 1170 with `allocator: header table exhausted`.

A5 holds: the leak exists on aarch64 exactly in the x86_64 shape, and the drains already in r9 fix it. The gate is a red-to-green proof across `b82ee9d` to `27f38a3`, and it serves 20-10 as a regression pin.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Kernel side: churngate driver and the virtchurn profile | r9 `ff1b155` | churngate.rs, virt/mod.rs, main.rs, proc.rs, syscall.rs, allocator.rs, Cargo.toml, config_virtchurn.toml |
| 2 | xtask profile + markers, and the arm64 churn payloads | r9 `3aa3eeb`, Harvey `783b13ed6` | xtask/src/main.rs; churnmark.c, churnsem.c, stage-build.sh |
| 3 | Run the gate against the exit path and record the slope | Harvey `ee8a54f30` | 20-EVIDENCE.md, deferred-items.md |

## Evidence

The full record is in `20-EVIDENCE.md` under `## Churn gate (pre-fix)`.

- **Builds:**
  - `virtchurn` and `virtshell` both build, and the build prints no `unexpected cfg condition name: dev_churn` warning.
  - The aarch64 warning count is unchanged at 25.
  - `cargo test -p xtask` passes, including 4 new churn post-pass tests. `cargo xtask test` passes. `cargo test -p aarch64 --bins` passes 29.
  - `git diff origin/main..HEAD -- port/src` in r9 is empty.
- **Readings (TCG):** at `3aa3eeb`, heap one 81920 B, base 100736 B, end 301312 / 311360 / 286784 B, steady 482 B/exec. Every run printed `DONE iterations=1400 semgens=39`, with 1361 churnmark prints counted and no panic, user fault or unresolvable line. At `b82ee9d`, base was 737344 B after 40 execs, and all three runs panicked before the end sample.
- **Kernel bytes:**
  - The aarch64 kernel-virt changes: `028044f5…` (60585720 bytes) at `3aa3eeb`, against the pin's `92cc905b…` (60584848 bytes). The churn code is all cfg-gated, but the `exec_prologue` split and line shifts move code.
  - The x86_64 kernel is byte-identical: the local `q35churnshell` build gives `b7a1e00b…` before and after.
  - So the boot-iso hash needs no re-derivation and no x64 gate run is owed. The pin did not move.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] aarch64 lacked the scheduler seams the driver needs**
- **Found during:** Task 1
- **Issue:** the x86_64 driver calls `proc::seed_kernel_context`, `enter_sched_loop_from_boot`, `first_resume_seed` and `any_other_active_procs`, and the 1400-exec loop needs more than 512 proc slots (husks are never reaped). None of this existed on aarch64, and `aarch64/src/proc.rs` was not in the plan's file list.
- **Fix:** added `seed_kernel_context`, `enter_sched_loop_from_boot` and `any_other_active_procs`, plus `MAX_PROCS = 2048`, all under `cfg(dev_churn)` in `aarch64/src/proc.rs`, the x86_64 17-02 shapes.
  - `seed_kernel_context` also sets `blocked_in_kernel`, because aarch64's pick_next only resumes through swtch when that flag is set.
  - `first_resume_seed` is not needed on aarch64: an exec'd Ready child is trap_returned into the frame exec_into seeded.
- **Commit:** r9 `ff1b155`

**2. [Rule 3 - Blocking] exec_bootstrap had no shareable prologue**
- **Found during:** Task 1
- **Issue:** the x86_64 driver consumes `exec_prologue`; aarch64's `exec_bootstrap` was one function.
- **Fix:** split its head into `exec_prologue` (hooks, env seed, devroot mount, root attach), which returns `ExecPrologue { img, root, slash, root_env }`. `exec_bootstrap` is otherwise unchanged. `BOOT_ENTRY`'s `/init` arm gained `not(dev_churn)` so the churn build has no dead constant.
- **Commit:** r9 `ff1b155`

**3. [Rule 2 - Correctness] the q35churn post-pass shared rather than duplicated**
- **Found during:** Task 2
- **Issue:** the plan asks for a virtchurn assertion set with the same checks as q35churn. A second copy of a ~100-line post-pass would drift.
- **Fix:** moved it verbatim into `churn_post_pass(capture, &ChurnMarkers)`. The X64Churn and VirtChurn arms both call it. The error text is unchanged.
- **Testing:** the shared function ran against three real aarch64 captures, and unit tests cover it. It has not been run against a live x86_64 q35churn capture, because none is available on this host.
- **Commit:** r9 `3aa3eeb`

**4. [Scope] an added measurement on the exit path without the drains**
- **Found during:** Task 3
- **Issue:** the plan's "pre-fix" premise (borrowed drain, no husk sweep) was out of date. `3a1c9e5` and `5893ae9` landed before 20-09. A reading only at HEAD would say "green" without saying whether the leak ever existed.
- **Fix:** also ran 3 gate runs on `b82ee9d` with the two 20-09 commits cherry-picked, in a throwaway worktree (since removed; nothing committed). Both readings are recorded, and the closing statement names both.

**5. [Minor] Task 3 also committed `deferred-items.md`.** It adds one out-of-scope finding: the census `live` column reads 0 on aarch64, and the census code is in `port/src`.

### Notes

- **commits: 2** is the measured Harvey count `7c1270baf..ee8a54f30`. The r9 commits `ff1b155` and `3aa3eeb` are in the r9 repo, local and unpushed.
- **Plan verify, Task 3:** `grep -A25 … | grep -cE 'A5 (holds|is false)'` prints 1, from an outcome line near the top of the section. The section also ends with the full A5 statement, which names both kernels.
- **The worktree socket path.** The first attempt at the drain-free runs used a worktree under the session scratchpad. qemu's serial socket path there was longer than macOS's 104-byte limit, so xtask could never connect. Those runs were killed and their outputs deleted. The worktree was moved to `.build/r9pd` and the runs repeated.
- **Host tests for churngate.** churngate's pure-half tests do not run on the host: the aarch64 host test build does not compile `platform::virt` (`--cfg platform="virt"` fails with an ambiguous `devcons`). The halves are x86_64's verbatim, and those tests run on x86_64.
- `graphify update .` was run after Task 2.

## Known Stubs

None.

## Threat Flags

None. The mitigations are in place as planned:
- **T-20-22:** the gate measures per-exec heap growth.
- **T-20-23:** the ceiling is x86_64's constant, and a test pins it to 1772.
- **T-20-24:** every churn path is `cfg(dev_churn)`, which only `config_virtchurn.toml` sets, and virtshell builds with an unchanged warning count.
- **T-20-SC:** no package was added.

## Self-Check: PASSED

- FOUND: ../r9/aarch64/src/platform/virt/churngate.rs and ../r9/aarch64/lib/config_virtchurn.toml (tracked, `git ls-files` = 2); build/aarch64/churnmark.c and churnsem.c (tracked, = 2); build/aarch64/smoke-out/churnmark and churnsem (fresh, untracked)
- FOUND: the allocator wrapper grep = 3; `cfg(dev_churn)` in r9 Cargo.toml; `virtchurn` in xtask
- FOUND (ancestors of HEAD): Harvey `783b13ed6`, `ee8a54f30`; r9 `ff1b155`, `3aa3eeb`
- The 11 font entries are unstaged; nothing was pushed in either repo
