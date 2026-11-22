---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 08
subsystem: r9 aarch64 syscall arms (rfork, pwrite, create) and the arm64 tier-1 sweep
tags: [r9, aarch64, rfork, rffdg, rfcfdg, devmnt, dmdir, union-create, writeprobe, ramfs, tier1, pin]
status: complete
requires: ["20-07"]
provides:
  - "r9 b82ee9d: aarch64 non-RFPROC rfork(RFFDG/RFCFDG) fd-group split, DEV_MNT pwrite through port::devmnt::write, DMDIR creates and the walk_one_union/create_chan_union twins with the nomount bit"
  - "T1-OK writeprobe on the r9 tier-1 sweep, required through the fixed names in nix/tier1-r9.sh (expected OK set 38 -> 39)"
  - "Harvey pin r9 57e2952 -> 27f38a3 (lock-only), carrying b82ee9d and the debugger's fd-group fixes 3a1c9e5, f8c2ac9, 5893ae9, 7b598f2"
affects: [20-09, 20-10, 20-11, 20-14]
tech-stack:
  added: []
  patterns:
    - "an r9-venue sweep entry that starts a long-lived server pipes the probe into a short awk that exits at a done line; only awk's stdout is backquote-captured"
    - "r9's aarch64 exec caps one argument at 128 bytes (MAX_EXEC_ARG_LEN), so inline awk programs in the sweep stay short"
key-files:
  created:
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-08-SUMMARY.md
  modified:
    - ../r9/aarch64/src/syscall.rs
    - build/aarch64/tier1-sweep.rc
    - nix/tier1-r9.sh
    - flake.lock
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
key-decisions:
  - "Owner: proceed with the full Task 1 diff despite the overlap with the fd-group debugger (different functions in the same file)"
  - "Owner: stop the b82ee9d-only series and run one series at r9 7b598f2, covering this plan's ports and the debugger's fd-group fixes"
  - "Owner: pin Harvey to r9 27f38a3 (7b598f2 plus the boot-iso outputHash; aarch64 kernel bytes identical to the tested kernel), after the linux-host boot-iso --rebuild passed"
  - "The writeprobe sweep entry requires seven steps (fdsplit mount create write readback mkdir union-create); remove stays 20-14's"
requirements-completed: []  # contributes to ARM-06 (first half) and ARM-07 (writable-venue pin); both complete with later plans
metrics:
  duration: "about 7 h 20 min (2026-11-19T15:56Z to 23:16Z), of which about 4 h 15 min was the 7b598f2 series and the rest included three owner checkpoints"
  completed: 2026-11-19
actuals:
  tokens: 6546
  tasks: 2
  commits: 8
plan_head_before: f8de3c32e35bd5e77be443f53e618042f32ddb4e
plan_head_after: 83a508e18
---

# Phase 20 Plan 08: aarch64 rfork fd-group split, devmnt write and DMDIR/union creates, pinned by writeprobe on the r9 tier-1 sweep. The pin moved to r9 27f38a3 after an 1808-run series. Summary

r9 `b82ee9d` ports the four aarch64 arch halves:
- the non-RFPROC `rfork(RFFDG)`/`rfork(RFCFDG)` split, where the reject now narrows to `RFMEM|RFNOWAIT`;
- `DEV_MNT` writes through `port::devmnt::write`;
- DMDIR creates;
- the union create twins, with the `nomount` bit.

All of it is in `aarch64/src/syscall.rs`, with no `port/src` change. On the same image, the write-flow probe's red steps b4 to b7 (`fdsplit`, `write`/`readback`, `mkdir`, `union-create`) turn from WP-FAIL to WP-OK, on both HVF and TCG.

The tier-1 sweep now prints `T1-OK writeprobe`, and the gate requires it: the expected OK set went from 38 to 39, and the 23 SKIP lines are unchanged.

The owner had one series run at r9 `7b598f2`, which also carries the debugger's fd-group fixes. It went 1808 of 1808 green. Harvey is now pinned to r9 `27f38a3`, whose aarch64 kernel is byte-identical to the tested one, and the default gate is green at the pin.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Port rfork fd-group split, devmnt write, DMDIR and union creates to aarch64 | r9 `b82ee9d` | aarch64/src/syscall.rs |
| 2 | Pin the ports with writeprobe in the tier-1 sweep on r9 | `8bad466f1` | tier1-sweep.rc, tier1-r9.sh, 20-EVIDENCE.md |
| (owner) | Record of the stopped b82ee9d-only series | `50bb1b762` | 20-EVIDENCE.md |
| (owner) | The 7b598f2 series and the byte-identity report | `611f8bfaa` | 20-EVIDENCE.md |
| (owner) | Pin r9 57e2952 -> 27f38a3, lock-only | `83a508e18` | flake.lock |

## Evidence

The full record is in `20-EVIDENCE.md` under `## Group B ports`.

- **Write flow, same image (bootfs `7960d429…`):**
  - At the pin `57e2952`: fdsplit, write, readback, mkdir and union-create are WP-FAIL. This is identical to 20-07's transcript.
  - At `b82ee9d` and again at `7b598f2`: all seven owned steps are WP-OK.
  - `remove` stays WP-FAIL (`syscall not implemented`), as it should: 20-14 owns it.
  - HVF and TCG are byte-identical at every kernel.
- **Tier-1 gate:**
  - `exit=0 ok=1 fail=0 okall=39 skip=23` on HVF and on TCG, at `b82ee9d` and at `7b598f2`.
  - The default `nix run .#boot-r9-tier1` at the new pin (HVF) printed the same line and booted `193ys32r…-r9-kernel-virt-27f38a3`.
- **Series at 7b598f2** (20-06 shape):
  - tier-1 HVF 10/10, net-smoke HVF 36/36, tier-1 TCG 3/3, net-smoke TCG 3/3, smoke TCG 3/3 and smoke HVF 1753/1753.
  - That is 21,562 forks with `pushword=0`, and no capture has a `Not registered`, `user fault` or `unresolvable` line.
- **Byte identity:**
  - The `7b598f2` kernel ELF sha256 is `92cc905b…`, and `--rebuild` is clean.
  - `27f38a3` is `7b598f2` plus one `flake.nix` line (the boot-iso `outputHash`). Its kernel-virt, evaluated from the GitHub rev the pin locks, is `193ys32r…`, and `diff -r` against the series kernel is empty.
  - The owner's linux-host `nix build .#boot-iso --rebuild` at `27f38a3` passed.
  - The debugger reports the x64 gates green at `7b598f2` (`8c927e3a4`).
- **r9 boot-test profiles at b82ee9d (TCG):**
  - 9 of 10 exit 0.
  - `virtshell` exits 1 with `no initrd image`, the same as at 20-04, because it is not an xtask boot-test profile by design. The tier-1 runs boot the virtshell kernel instead.
- **Pin:** `flake.lock` is the only file in `83a508e18`. The locked rev `27f38a3a916d829307cce60bb09e4ce50f361ced` equals r9 `origin/main`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The first writeprobe sweep entry could not start awk**
- **Found during:** Task 2, the first tier-1 runs (HVF and TCG)
- **Issue:** the entry passed a ~500-byte awk program, but r9's aarch64 exec caps each argument at 128 bytes (`MAX_EXEC_ARG_LEN`). The runs logged `/bin/hawk: string too long` and `can't exec: string too long`, printed no WP lines, and the gate failed with `missing T1-OK writeprobe`.
- **Fix:** the shipped awk program is 67 bytes. It copies lines to `/dev/cons` and prints the WP-OK names at WP-DONE, and the seven-step check is done in rc. The probe runs through a fn producer under `rfork n`.
- **Commit:** `8bad466f1`

**2. [Rule 3 - Blocking] The probe's output cannot be backquote-captured**
- **Found during:** Task 2 design
- **Issue:** the ramfs server keeps a copy of the fd table, and so the pipe writer, for its life, so a backquote over the probe never sees end of file.
- **Fix:** the probe writes into a pipe that awk leaves at WP-DONE. Only awk's stdout is captured.
- **Commit:** `8bad466f1`

### Owner-directed changes (checkpoints)

**3. A checkpoint before Task 1.** Task 1's rfork hunk is fd-group (Fgrp) code, which the concurrent debugger was also working on. I built the diff in a scratch clone first and returned a checkpoint. The owner chose `proceed`.

**4. Series venue.** The plan named no series. The owner asked for the 20-06-shape HVF series plus a byte-identity report.
- A first series at `b82ee9d` was stopped by the owner at HVF smoke run 52 of 1753, with every run green so far. It is recorded as a record for `b82ee9d` alone.
- The full series then ran at `7b598f2`, so that one series covers this plan's ports and the debugger's `3a1c9e5`, `f8c2ac9`, `5893ae9` and `7b598f2`. `f8c2ac9` changed this plan's `sys_rfork` RFCFDG arm to give a fresh table; writeprobe stays green at `7b598f2`.

**5. Pin move.** The plan did not move the pin, but the owner chose `pin-27f38a3`. Without the move, `8bad466f1` would leave the default gate red at `57e2952` (`T1-FAIL writeprobe`).

### Notes

- **commits: 8** is the measured count `f8de3c32e..83a508e18`. Four of those commits are this plan's Harvey commits (`8bad466f1`, `50bb1b762`, `611f8bfaa`, `83a508e18`). The other four are the debugger's `.planning/debug` commits (`a1bcc4b3c`, `176723fc9`, `8c927e3a4`, `8baf6623a`), which landed on master while the plan ran. The r9 commit `b82ee9d` is in the r9 repo; the owner pushed it as part of `27f38a3`.
- `tier1-r9.expect` is unchanged, because the `no-unsupported-era` absence check already existed. `tiers/tier1-cmds.txt`, `subset-exec.sh` and `build/mm/boot-9k10t1-sweep.sh` are untouched.
- The bootfs bake is not byte-stable from run to run: the same sources baked `7960d429…` and later `180a3c86…`. This does not affect any result here.
- `graphify update .` was run after Task 2.

## Known Stubs

None.

## Threat Flags

None. T-20-18 to T-20-21 are mitigated as planned:
- The new arms read user memory only through `copy_user_str` and `UserSlice`.
- `nomount` is derived from the path's first byte and passed to both union twins.
- The pwrite `DEV_MNT` arm sits inside the existing `fd_op_done` discipline.
- RFFDG takes `deep_copy()` under the group lock into a new `Arc<Lock<..>>`.

## Self-Check: PASSED

- FOUND: build/aarch64/tier1-sweep.rc (writeprobe entry, 2 `fdsplit` lines), nix/tier1-r9.sh (`R9_EXPECT_OK=… writeprobe`), `## Group B ports` with 2 `WP-OK fdsplit` lines, flake.lock locked at 27f38a3
- FOUND (ancestors of HEAD): `8bad466f1`, `50bb1b762`, `611f8bfaa`, `83a508e18`; r9 `b82ee9d` is an ancestor of r9 origin/main `27f38a3`
- `git -C ../r9 diff --stat 57e2952..27f38a3 -- port/src` is empty; the 11 font entries are unstaged
