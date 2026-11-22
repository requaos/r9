---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 10
subsystem: r9 aarch64 exec and exit paths, and the arm64 tier-1 sweep
tags: [r9, aarch64, close-on-exec, ocexec, exit-path, husk-sweep, brk, churn, virtchurn, tier1, cexecprobe, brkprobe]
status: complete
requires: ["20-09"]
provides:
  - "r9 1125a20: aarch64 close_on_exec_fds (take_cexec under the fd-group lock, close_chan outside it), called once in exec_replace between check_seed_fit and free_user_mappings; the D-09 no-brk comment above rfmem_brk_set's one-member arm"
  - "Harvey 61ad20f43: build/aarch64/cexecprobe.c baked to /arm64/bin/cexecprobe; T1-OK cexecprobe and T1-OK brkprobe on the r9 tier-1 sweep, required through the fixed names (expected OK set 39 -> 41)"
  - "The post-fix virtchurn reading at the final r9 rev: 3/3 PASS, 146/154/155 B/exec against 1772"
  - "Task 1's exit-path drains recorded as already in r9 (3a1c9e5, 5893ae9, in the pin 27f38a3), item by item"
affects: [20-11, 20-14, 20-17]
tech-stack:
  added: []
  patterns:
    - "a self-exec probe checks fd state across exec by passing the fd numbers as argv (each well under r9's 128-byte exec argument cap) and testing them with dirfstat in the new image"
    - "a gate run that derives KERNDATE from the Harvey HEAD commit time must not straddle a Harvey commit; commits wait until a running series ends"
key-files:
  created:
    - build/aarch64/cexecprobe.c
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-10-SUMMARY.md
  modified:
    - ../r9/aarch64/src/exec.rs
    - ../r9/aarch64/src/proc.rs
    - build/aarch64/subset-build.sh
    - build/aarch64/mkimage.sh
    - build/aarch64/tier1-sweep.rc
    - nix/tier1-r9.sh
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/deferred-items.md
key-decisions:
  - "Task 1 made no r9 commit: every item it planned (the three husk singletons, sweep_husk_payloads, the stack-buffer sys_exits, the one-at-a-time owning drain, the explicit fd-group release, the exit_tail call) is in r9 3a1c9e5 and 5893ae9, already in the pin 27f38a3"
  - "aarch64's sweep_husk_payloads call stays before free_user_mappings (5893ae9's placement), not after the queue resets as on x86_64: a mount chan's Tclunk can RPC-block and needs the address space live"
  - "D-09 = ramfs-venue-brk-holds: the brk change is one doc-comment sentence; the x86_64 eager one-member mapping is not ported"
  - "The post-fix churn reading ran at the final r9 rev (1125a20), so it covers this plan's exec change as well"
  - "One 20-06-shape series at r9 1125a20 covers 20-09 and 20-10 together, rev-locked through R9_OVERRIDE ?rev="
requirements-completed: []  # contributes to ARM-06 (exit drains, close-on-exec, brk by measurement) and ARM-07 (the churn and cexec/brk pins); 20-11 still owes the fd-drain cascade
metrics:
  duration: "about 5 h 20 min wall (2026-11-20T00:37Z to about 05:58Z), of which 4 h 22 min was the series"
  completed: 2026-11-20
actuals:
  tokens: 7196
  tasks: 3
  commits: 3
plan_head_before: 6d0d49f887aefcb9c39e0a354d72a6fcd92bf9f2
plan_head_after: b3058291705554b31e80e414fe33d86119333d56
---

# Phase 20 Plan 10: aarch64 close-on-exec sweep (r9 1125a20) pinned by cexecprobe on the tier-1 sweep, brkprobe made a required regression pin, and the exit-path drains confirmed already in r9 with virtchurn green at the final rev Summary

The plan had three parts: port the aarch64 exit-path drains, port the close-on-exec sweep, and settle brk by measurement. Only the close-on-exec sweep was still missing.

- **Exit path (Task 1):** nothing to port. The debugger's r9 `3a1c9e5` and `5893ae9`, already in the pin `27f38a3`, carry every item Task 1 listed. Task 1 became its measurement: `virtchurn` at the final r9 rev passed 3 of 3, measured 146, 154 and 155 bytes per exec against the 1772 ceiling (pre-fix 147, 154, 136).
- **Exec path (Task 2):** r9 `1125a20` adds `close_on_exec_fds` to `aarch64/src/exec.rs` in x86_64's placement. brk takes the D-09 no-code branch: one comment sentence.
- **Pins (Task 3):** `cexecprobe` is baked into the r9 image, and the sweep now requires `T1-OK cexecprobe` and `T1-OK brkprobe`. Both are green on HVF and TCG at `1125a20`. At the pin `27f38a3` the probe prints `cexecprobe: LEAK fd=3` and the gate fails, so the pin bites.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Exit-path drains and the husk sweep on aarch64; churn gate green | Harvey `8bc4ac895` (no r9 commit: already in `3a1c9e5`, `5893ae9`) | 20-EVIDENCE.md |
| 2 | Close-on-exec sweep, and the brk branch 20-07 selected | r9 `1125a20` | aarch64/src/exec.rs, aarch64/src/proc.rs |
| 3 | Pin exec and brk in the tier-1 sweep | Harvey `61ad20f43` | cexecprobe.c, subset-build.sh, mkimage.sh, tier1-sweep.rc, tier1-r9.sh |
| (records) | Tier-1, pinned-kernel, kernel-bytes and series evidence; deferred item | Harvey `b30582917` | 20-EVIDENCE.md, deferred-items.md |

## Task 1: what was planned against what r9 had

| Planned | In r9 | Commit |
|---|---|---|
| `empty_pgrp`, `empty_env_group`, `empty_rendez_group` | `aarch64/src/proc.rs` | `5893ae9` |
| `sweep_husk_payloads` (last-holder mount close, the group swaps, slash/dot drop) | `aarch64/src/proc.rs` (`release_pgrp` + `sweep_husk_payloads`) | `5893ae9` |
| `sys_exits` stack buffer, `Vec` dropped before the tail | `aarch64/src/syscall.rs` | `5893ae9` |
| Owning one-at-a-time drain | `proc::release_fd_group` | `5893ae9`, `3a1c9e5` |
| Detached fd group dropped before the tail | `release_fd_group` returns, so the `Arc` drops there | `3a1c9e5` |
| `exit_tail` calls the sweep | `aarch64/src/syscall.rs` | `5893ae9` |

The only difference from the plan's text is where the sweep sits. It runs before `free_user_mappings` rather than after the queue resets, deliberately, as the comment at the call explains. The plan's grep (`fn sweep_husk_payloads` = 1) holds. I did not return a checkpoint for this. The orchestrator had already given the instruction "do only the remaining parts", Task 1's measurement half still had to run, and Tasks 2 and 3 were unaffected.

## Evidence

The full record is in `20-EVIDENCE.md`, under `## Churn gate (post-fix)` and `## Exec and brk pins (20-10)`.

- **Churn gate at r9 `1125a20` (TCG):**
  - 3 of 3 exit 0, measured 146, 154 and 155 B/exec, steady 482. Every run printed `DONE iterations=1400 semgens=39`, with 1361 churnmark prints and no panic, fault or unresolvable line.
  - Post-fix is no worse than pre-fix within run-to-run noise. The pre-fix spread is 18 B, and the means differ by 6 B, which is 0.3% of the ceiling.
  - No improvement is claimed. churnmark opens no OCEXEC fd, so neither commit should move the churn slope.
- **Other aarch64 boot-test profiles at `1125a20` (TCG):** virt, virtlowecam, virtpanic, virtexec, virtexec3, virtmulti, virtnotes, virtnet and virtip all exit 0. `virtshell` exits 1 with the by-design `dev_exec: config carries no initrd image` panic, as at 20-04 and 20-08.
- **Builds and tests at `1125a20`:**
  - `virtshell` builds, with aarch64 warnings unchanged at 25 and none from exec.rs or proc.rs.
  - `cargo xtask test` is green, and `cargo test -p aarch64 --bins --target aarch64-apple-darwin` passes 29.
  - `git diff 27f38a3..1125a20 -- port/src` is empty.
- **Tier-1 gate at `1125a20`:** `hvf exit=0 cexec=1 brk=1 fail=0` and `tcg exit=0 cexec=1 brk=1 fail=0`. That is 41 T1-OK (39 + 2) and 23 T1-SKIP, with `ALL STAGES PASS`.
- **Pinned kernel (`27f38a3`, no override, HVF):** exit 1, with `cexecprobe: LEAK fd=3`, `T1-FAIL cexecprobe` and `post-pass FAILED — missing T1-OK cexecprobe`. `T1-OK brkprobe` is still present, because D-09 changed no brk code.
- **Kernel bytes:**
  - The aarch64 `kernel-virt` at `1125a20` is `3e86d2f1…`, 60598080 bytes. The pin's is `92cc905b…`, 60584848 bytes.
  - x86_64 is untouched: `27f38a3..1125a20` changes nothing under `x86_64/` or `port/src`, and 20-09 measured the x64 ELF byte-identical through `3aa3eeb`. So no boot-iso `outputHash` re-derivation and no x64 TCG gate run are owed.
- **Series at `1125a20`** (the 20-06 shape, rev-locked, `R9_STOP_ON_FAIL=1`):

```
SERIES boot-r9-tier1 accel=hvf runs=10 pass=10 fail=0 pushword=0 forks=3710
SERIES boot-r9-net-smoke accel=hvf runs=36 pass=36 fail=0 pushword=0 forks=2556
SERIES boot-r9-tier1 accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=1113
SERIES boot-r9-net-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=213
SERIES boot-r9-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=24
SERIES boot-r9-smoke accel=hvf runs=1753 pass=1753 fail=0 pushword=0 forks=14024
```

  1808 of 1808 green, with 21640 forks, `pushword=0`, and no `Not registered`, `user fault` or `unresolvable` line. Every block booted `j0d1q06r…-r9-kernel-virt-1125a20`, and `nix build --rebuild` of it is clean. The run took 2026-11-20T01:28:39Z to 05:50:32Z.

## Deviations from Plan

### Scope changes (orchestrator-directed)

**1. Task 1 had no code to write.** Its r9 commit (`fix(20-10): aarch64 exit-path drains + husk payload sweep`) does not exist, because r9 already had the work. Its acceptance criterion "the r9 commit lists exactly `aarch64/src/syscall.rs` and `aarch64/src/proc.rs`" is therefore moot. The other criteria were met: virtchurn passed 3 times, the post-fix section is written, and the other profiles exit 0.

**2. The churn measurement moved after Task 2.** It ran at `1125a20` rather than between Tasks 1 and 2. With Task 1 code-free, measuring the final rev covers more and costs the same.

**3. The series.** The plan names none. The orchestrator asked for one 20-06-shape HVF series covering 20-09 and 20-10 at the final r9 rev, before the pin checkpoint. It ran at `1125a20`.

### Auto-fixed Issues

None in code. One drafting slip was caught before commit: the first insertion put `close_on_exec_fds` between `exec_replace`'s doc comment and its `fn`, which would have attached the doc to the wrong item. It was moved above the doc block.

### Notes

- **commits** is the measured Harvey count `6d0d49f88..HEAD`. r9 `1125a20` is in the r9 repo, local and unpushed, on top of 20-09's unpushed `ff1b155` and `3aa3eeb`.
- **The default gate is red from `61ad20f43` until the pin moves.** At the pin `27f38a3`, the sweep's `T1-FAIL cexecprobe` reds `nix run .#boot-r9-tier1`. This is the same situation 20-08 had with writeprobe. The pin checkpoint follows this summary.
- **No Harvey commit during the series.** `nix/tier1-r9.sh` derives KERNDATE from the HEAD commit time, so the records commit waited for the series to end.
- `graphify update .` was run after Task 3.

## Known Stubs

None.

## Threat Flags

None. The mitigations are in place as planned:
- **T-20-25:** `close_on_exec_fds` is in `exec_replace`, and cexecprobe is in the sweep, shown red at the pin.
- **T-20-26:** the mount close runs only for the last pgrp holder (`release_pgrp`'s strong-count test, `5893ae9`). The fork-heavy sweep and the series exercise shared pgrps.
- **T-20-27:** virtchurn passed 3 consecutive times at the final rev.
- **T-20-SC:** no package was added.

## Pin move (after the checkpoint)

The owner chose `pin-1125a20`, and the coordinator pushed r9, so `origin/main` is `1125a20`. The pin move is lock-only: `flake.lock`'s `r9` node goes `27f38a3` -> `1125a20`, the only node changed. The default `nix run .#boot-r9-tier1` (HVF, no override) then passed with 41 OK, 23 SKIP and no `T1-FAIL`, booting the series kernel's own store path (`j0d1q06r…-r9-kernel-virt-1125a20`). The OCEXEC create defect in `deferred-items.md` is now in progress through a separate debugger session.

## Self-Check: PASSED

- FOUND: build/aarch64/cexecprobe.c (tracked) and build/aarch64/out/cexecprobe (fresh, gitignored); `R9_EXPECT_OK=… cexecprobe brkprobe` grep = 1; `cexecprobe` in subset-build.sh = 8 and in mkimage.sh = 4
- FOUND: r9 aarch64/src/exec.rs `take_cexec` = 1 and `close_on_exec_fds(proc)` = 1; aarch64/src/proc.rs `fn sweep_husk_payloads` = 1
- FOUND (ancestors of HEAD): Harvey `8bc4ac895`, `61ad20f43`, `b30582917`; r9 `1125a20`
- `git -C ../r9 diff --stat 27f38a3..1125a20 -- port/src` is empty; the 11 font entries are unstaged; nothing was pushed (r9 main ahead 3, Harvey master ahead 6)
