---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 14
subsystem: arm64 exec (#! pre-pass), REMOVE/WSTAT/FWSTAT dispatch, devmnt Tremove (port, both arches)
tags: [r9, aarch64, x86_64, port, devmnt, tremove, shebang, exec, remove, wstat, fwstat, boot-iso, fod, writeprobe, tier1, ARM-08]
status: complete
requires: ["20-13"]
provides:
  - "MAX-ARGS: equal 32; EXEC-DIVERGENCE: listed; EXEC-ROUTE: as-planned (owner-confirmed)"
  - "DEVMNT-REMOVE re-confirmed absent at de03bf7, then built: devmnt::remove (Tremove on a clone of the walked fid) + DEV_MNT arm in port::namec::remove_chan (r9 98feebc)"
  - "r9 aarch64 copy_exec_args, exec_backing_for (49ba196); SHEBANG_LINE_BOUND, MAX_SHEBANG_DEPTH, ShebangExec, shebang_pass, exec_with_shebang (7adb773)"
  - "r9 aarch64 REMOVE(25)/_WSTAT(26)/_FWSTAT(27)/WSTAT(44)/FWSTAT(45) with valid_stat_wire, remove_chan, wstat_chan, sys_remove, sys_wstat, sys_fwstat, wstat_stage_args (9173af0)"
  - "SHEBANG-LIVE: ok; REMOVE-ROUTE: devmnt-remove"
  - "boot-iso outputHash sha256-B2nbReb8JTQ5siowrpd7RoIniEb8Fs2mEkv04ldE810= (r9 105d48e, x86_64 kernel e791e863...)"
  - "T1-OK writeprobe requires all eight steps including remove (Harvey d67f23a52)"
affects: [20-15, 20-16, 20-17]
tech-stack:
  added: []
  patterns:
    - "a 9P op that consumes its fid (Tremove) rides a zero-name-walk clone, so the chan's own fid lifecycle (clunk_if_last at close) is untouched"
    - "devroot ignores mode bits on OEXEC: a baked #! file in /lib is exec-able, which makes a live #! proof possible without a mode-755 fixture"
    - "read a child's argv live from /proc/$apid/args to check exec argv order"
    - "docker exec into the amd64 container needs -e USER=builder as well as -e HOME, or nix.sh leaves nix off PATH"
    - "after a manual mutant, touch the restored file before rerunning cargo: a same-second restore keeps the stale artifact"
key-files:
  created:
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-14-SUMMARY.md
  modified:
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/exec.rs
    - ../r9/port/src/devmnt.rs
    - ../r9/port/src/namec.rs
    - ../r9/port/tests/devmnt_test.rs
    - ../r9/x86_64/src/syscall.rs
    - ../r9/flake.nix
    - build/aarch64/tier1-sweep.rc
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
key-decisions:
  - "Owner: remove route devmnt-remove (beyond x86_64 parity); port/src edit approved for exactly a devmnt Tremove plus a DEV_MNT arm in port::namec::remove_chan; rm works through mounts on both arches"
  - "Owner: EXEC-ROUTE as-planned confirmed; rm on /env and /srv expected to succeed (port's existing devenv/devsrv arms)"
  - "Owner: require-remove; T1-OK writeprobe now requires the remove step"
  - "aarch64 sys_remove routes through port::namec::remove_chan (the one dispatch since r9 5629cf0), not a second arch-local dispatch; 20-15 needs no remove_chan edit for devcap"
  - "devmnt::remove Tremoves a clone of the walked fid rather than the fid itself, because 9P clunks the Tremove fid and r9's close dispatch would then Tclunk a dead fid"
requirements-completed: [ARM-08]
metrics:
  duration: "about 2 h (2026-11-21T13:31Z to 15:31Z), including two owner checkpoints and the amd64 container battery"
  completed: 2026-11-21
actuals:
  tokens: 13500
  tasks: 4
  commits: 3
plan_head_before: 921c3ca831d1712443973e72246b6ebce1244d2e
plan_head_after: d67f23a529d11fe5211ae5c1f57ee50b1b08fef9
---

# Phase 20 Plan 14: arm64 #! exec and REMOVE/WSTAT/FWSTAT at parity, plus a devmnt Tremove so rm works through mounts on both arches Summary

arm64 exec now honors `#!`. This is proven live: the image's own `/lib/writeprobe.rc` runs, and the argv order is read back from `/proc`. REMOVE, WSTAT and FWSTAT are dispatched with x86_64's stat validation. The owner chose to go past parity, so a devmnt Tremove now makes `rm` work through mounted trees on both arches. writeprobe is fully green, and the tier-1 sweep now requires its remove step.

## Tasks

| Task | Name | Commits | Files |
|---|---|---|---|
| 1 | Exec and remove measurements | Harvey `a5945e533` | 20-EVIDENCE.md |
| 2 | Owner chooses the remove route | (decision: devmnt-remove, exec route as-planned) | — |
| 3 | Factor sys_exec, port the #! pre-pass | r9 `49ba196` (refactor), `7adb773` (feat) | aarch64 syscall.rs, exec.rs |
| 4 | Dispatch arms + devmnt Tremove (re-planned inline) | r9 `98feebc` (port), `9173af0` (aarch64), `105d48e` (boot-iso hash); Harvey `0c6754b56` (evidence), `d67f23a52` (tier-1 sweep) | see key-files |

All commits are local and unpushed. r9 `main` = `105d48e`, five commits on 20-13's `de03bf7` (seven ahead of `origin/main` `fa38402`). The Harvey pin is still `fa38402`.

## Evidence

The full record is in `20-EVIDENCE.md`, under `## Exec and remove measurements` and `## Group C ports`.

| Check | Result |
|---|---|
| arm64 gates smoke / net-smoke / tier1 | HVF 0/0/0 after each of `49ba196`, `7adb773`, `9173af0`; TCG 0/0/0 at `9173af0`; tier1 HVF and TCG at `105d48e` with the new requirement: 0 FAIL, 43 OK, 23 SKIP |
| `#!` live | `/lib/writeprobe.rc a1 a2`: before `bad a.out magic`, after it runs under `/bin/rc`; `/proc/$apid/args` = `writeprobe.rc /lib/writeprobe.rc a1 a2` (HVF = TCG) |
| rm / chmod live | ramfs `rm` and `/env` `rm` succeed; devroot and `#c/cons` give `remove: device does not implement remove`; both chmod lines give `wstat: device does not implement wstat` |
| writeprobe | all eight steps `WP-OK` (HVF = TCG); with `7adb773` (no REMOVE) the tier1 gate reds as `T1-FAIL writeprobe remove` |
| host (darwin) | `cargo xtask test` 237/0 (devmnt_test 16 -> 18; both new pins kill the no-clone and no-arm mutants); aarch64 crate 32/32; builds pass for virtshell, aarch64 and x86-64 |
| x86_64 (amd64 container) | host tests 116/116 at base and fix; kernel-x64 `8b9d216d…` -> `e791e863…`; x64 TCG auth / net-smoke / tier1 PASS (38 OK); boot-iso re-derived and green at the committed `105d48e`, `--rebuild` rc 0; xtask check/clippy/test rc 0 with unchanged warning counts; container removed and Docker state restored |

## Deviations from Plan

### Scope changes (owner-directed)

**1. Task 4 was re-planned inline for `devmnt-remove`.** The plan was written for `refuse-loudly` and told the executor to stop on any other answer. The coordinator relayed the owner's `devmnt-remove`, with a port/src approval for exactly a devmnt Tremove plus a `DEV_MNT` arm in `port::namec::remove_chan`, and told me to re-plan Task 4 inline. As a result:
- **THIS PLAN EDITS `port/src` AND CHANGES THE x86_64 KERNEL BYTES** (the plan's prohibitions assumed it would not). The boot-iso FOD was re-derived in r9 `105d48e`.
- 20-17's FOD pass must start from `sha256-B2nbReb8JTQ5siowrpd7RoIniEb8Fs2mEkv04ldE810=`.
- The owner's linux `nix build .#boot-iso --rebuild` is owed after the push.
- `REMOVE-ROUTE: devmnt-remove` replaces the planned `refuse-loudly`.

**2. `remove_chan` routes through port's existing dispatch.** The plan said aarch64 `remove_chan` would have "only the refusing default arm" and that 20-15 would add devcap. Since r9 `5629cf0`, x86_64's `remove_chan` is a one-line wrapper over `port::namec::remove_chan` (devcap, devsrv, devenv). The coordinator directed aarch64 to use the same function, so `rm` on `/env` and `/srv` succeeds and 20-15 needs no remove edit.

**3. The writeprobe/tier-1 expected set changed** (owner: `require-remove`, at a second checkpoint). `T1-OK writeprobe` requires `remove`. This is Harvey `d67f23a52`, with a userland rebuild, an image bake, and tier1 on HVF and TCG.

### Auto-fixed issues

**1. [Rule 1 - Bug] A Tremove would have left the chan to Tclunk a dead fid.** 9P clunks the fid a Tremove is sent on, but `sys_remove` closes the walked chan afterwards, and `clunk_if_last` would Tclunk the same fid again. The fix: `devmnt::remove` sends the Tremove on a zero-name-walk clone. Pinned by `devmnt_remove_rides_a_clone_and_close_clunks_the_walked_fid_once`. Commit `98feebc`.

**2. [Rule 3 - Blocking] The container gate script exited 127.** `docker exec` without `USER` leaves nix off PATH. I reran with `-e USER=builder`. No repo change.

### Findings that changed the record

- `SHEBANG-LIVE` is `ok`, not the predicted `deferred-to-20-16`. Task 1 reasoned from the ramfs mode-bit refusal, but devroot does not check mode bits on OEXEC, so a baked `#!/bin/rc` file in `/lib` is exec-able.
- The three-level `#!` chain could not be run live, because no image file has a `#!` script as its interpreter. That reject is pinned in 20-16 (fixture) and 20-17 (host).

## Known Stubs

None.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: new-remove-surface | ../r9/port/src/devmnt.rs | REMOVE(25) now reaches any mounted 9P server on both arches (x86_64 included). Permission is the server's to enforce, as for write and create; the donor's Eismtpt mount-point guard is still not carried (x86_64 parity). |

T-20-36 (the depth bound of 2 and the MAX_ARGS reject, ported unchanged), T-20-37 (`wstat_stage_args` bounds n before the copy and validates before routing) and T-20-38 (both routing functions end in a named Err) are mitigated as planned. T-20-SC: no crate or package was added.

## Notes

- `commits: 3` is the measured Harvey count `921c3ca83..d67f23a52`. The final docs commit adds one more. r9 commits: 5 (`49ba196`, `7adb773`, `98feebc`, `9173af0`, `105d48e`).
- `actuals.tokens` is chars/4 over the added lines in the r9 diff `de03bf7..105d48e`, the tier-1 sweep and the evidence file.
- The arm64 `kernel-virt` bytes are `2834f194…` (identical at `9173af0` and `105d48e`). They differ from the pin. The 20-06-shape HVF series covering 20-13 and 20-14 is held for the pin checkpoint.
- The 11 pre-existing font/postscript modifications stayed unstaged. On darwin's case-insensitive checkout they are case-collision artifacts.

## Self-Check: PASSED

- FOUND (ancestors of Harvey HEAD): `a5945e533`, `0c6754b56`, `d67f23a52`.
- FOUND (ancestors of r9 HEAD): `49ba196`, `7adb773`, `98feebc`, `9173af0`, `105d48e`.
- `## Exec and remove measurements` has `MAX-ARGS: equal 32`, `EXEC-DIVERGENCE: listed`, `EXEC-ROUTE: as-planned`.
- `## Group C ports` has `SHEBANG-LIVE: ok` and `REMOVE-ROUTE: devmnt-remove`.
- The aarch64 DISPATCH has 5 `Some(sys_(remove|wstat|fwstat))` slots, and `exec_with_shebang` appears 2 times outside comments.
- r9 `git status --porcelain` is empty.

## Addendum: pin checkpoint (2026-11-21 to 22)

- **r9 pushed** by the coordinator: `origin/main` went from `105d48e` to `c94bc9b`, with the debuggers' fixes on top.
- **First pin move** `fa38402 -> 105d48e` (Harvey `4888b718b`). Its series went **RED**: HVF net-smoke run 23 hit `user trap: unhandled ec=0x22` (a PC alignment fault in a forked rc child), recorded in `f98d21fad`. The debuggers root-caused it to missing I-cache maintenance on rfork's text copies (WINDOWS #49 residual; r9 `79c55b1`, `46a5c87`).
- **Final pin move** `105d48e -> c94bc9b` (Harvey `8160ec3f3`, lock-only).
- **Owner's linux `nix build .#boot-iso --rebuild` at `c94bc9b`: PASSED.**
- **The full 20-06-shape HVF series is deferred to the end of phase 20** by the owner: one series then covers everything. The partial run at `c94bc9b` was green on every leg that ran: tier1 HVF 10/10, net-smoke HVF 36/36, TCG 9/9, smoke HVF 355/355 when stopped.
- **Confidence pass at the pin, all exit 0:** tier1 on HVF and TCG (44 OK / 23 SKIP / 0 FAIL each), smoke HVF, net-smoke HVF.
- **Expected tier-1 counts at the pin:** 44 on arm64, 39 on x64 (both confirmed live).
- **No Harvey push yet.** It waits on the owner's push decision.
