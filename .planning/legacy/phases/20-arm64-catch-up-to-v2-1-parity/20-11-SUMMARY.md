---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 11
subsystem: r9 pin, arm64 gates, r9 CI
tags: [r9, aarch64, pin, hvf-series, virtchurn, ci, ocexec, orclose, cexeccreate, rcloseprobe, donor-gate, boot-iso]
status: complete
requires: ["20-10"]
provides:
  - "r9 7d464ad: CI job build-aarch64-virtchurn (the dev_churn aarch64 code is now compiled in CI)"
  - "Harvey pin r9 1125a20 -> fa38402, lock-only (c7cd176d2), carrying the CI lane, the create(OCEXEC) fix (437b752) and the ORCLOSE fix (5629cf0, owner-approved port/src)"
  - "T1-OK cexeccreate (cexecprobe -c) and T1-OK rcloseprobe as required r9 tier-1 markers; expected OK set 41 -> 43"
  - "1808/1808 combined series at the pin, no override, at the recorded N"
  - "Harvey master pushed at 34941cc0a; owner checks (r9 xtask 8/8, Harvey r9-shell/phase6/harvey, tier-9k clean, linux boot-iso) approved"
affects: [20-12, 20-14, 20-17]
tech-stack:
  added: []
  patterns:
    - "export KERNDATE as the pin commit's time for a long series, so a commit by a concurrent session cannot move the image provenance mid-series"
    - "run xtask boot-tests from a short path: the console socket path must fit macOS's 104-byte sun_path, or qemu waits forever for its client"
key-files:
  created:
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-11-SUMMARY.md
  modified:
    - ../r9/.github/workflows/xtask.yml
    - build/aarch64/tier1-sweep.rc
    - nix/tier1-r9.sh
    - flake.lock
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
    - .planning/STATE.md
key-decisions:
  - "Owner option 3: push r9 7d464ad (CI-only) ahead of the pin; hold the pin move and the series for the debugger's OCEXEC and then ORCLOSE fixes, so one lock-only move and one series covered all of it"
  - "Owner: T1-OK cexeccreate and T1-OK rcloseprobe join the required r9 tier-1 markers (41 -> 43)"
  - "Owner: the ORCLOSE fix is an approved port/src edit; its linux boot-iso FOD pass at fa38402 cleared the push hold"
  - "Tier-1 N restated from the at-pin capture: 376 forks, 176 ordinary with forkstress excluded (STRESSOR-RATE: not-fired); the floor keeps N at 10"
requirements-completed: [ARM-03, ARM-05, ARM-07]
metrics:
  duration: "about 12 h 35 min wall (2026-11-20T12:45Z to 2026-11-21T01:20Z), of which 4 h 37 min was the series and most of the rest was waiting on the debugger sessions"
  completed: 2026-11-21
actuals:
  tokens: 6083
  tasks: 3
  commits: 17
plan_head_before: 699f3dcf9a1f5e27f01ec7e1c73ac536892349ca
plan_head_after: e717ff53f7150b9f3806aba9503cb041af193608
---

# Phase 20 Plan 11: group B closed at r9 fa38402 (create(OCEXEC), ORCLOSE, the virtchurn CI lane) with a lock-only pin, a 1808/1808 at-pin series, two new tier-1 pins (cexeccreate, rcloseprobe), and both remotes pushed Summary

Group B is live at the pin and green on HVF. Harvey's r9 input moved lock-only from `1125a20` to `fa38402`. At that pin, with no override, the combined series ran 1808 of 1808 green at the recorded N. The owner's hosted-CI and linux boot-iso checks are green, and both remotes are pushed.

The plan grew while it ran. The owner held the pin twice for debugger fixes: create(OCEXEC) in r9 `437b752`, then ORCLOSE plus a shared port remove dispatch in r9 `5629cf0`. Each fix got its own tier-1 pin, so the required r9 tier-1 set went from 41 to 43.

## Tasks

| Task | Name | Commits | Files |
|------|------|---------|-------|
| 1 | CI lane for virtchurn, local battery, push r9 | r9 `7d464ad` (pushed by the coordinator); Harvey `a71a45b8c`, `c3e601f89` | xtask.yml, 20-EVIDENCE.md |
| 2 | Lock-only pin, series at the pin, donor gate, STATE, push Harvey | Harvey `68119abf8`, `7bddb3e02`, `c7044a68b`, `a55fec5cb`, `6fa102769`, `c7cd176d2`, `34941cc0a`, `e717ff53f` | tier1-sweep.rc, tier1-r9.sh, flake.lock, 20-EVIDENCE.md, STATE.md |
| 3 | Owner confirms hosted CI and boot-iso | (records in the final docs commit) | 20-EVIDENCE.md |

## Evidence

The full record is in `20-EVIDENCE.md` under `## Group B at the pin`.

**Batteries.** All three ran in clean clones, with every step exiting 0: check, clippy, test, the five builds, ureg-check, and the virtchurn boot-test.

| Rev | virtchurn B/exec (ceiling 1772) | Notes |
|---|---|---|
| `7d464ad` | 168 | |
| `42203d8` | 143 | |
| `fa38402` | 158 | port lib tests 165 -> 168 |

**port/src.** The diff was empty through `42203d8`. At `fa38402` it is five files, from `5629cf0`, owner-approved. The evidence records it as `PORT-SRC-EDIT: YES (r9 5629cf0, owner-approved)`.

**The new pins, on TCG.** Each was red at the old pin and green at the fix:

| Probe | At `1125a20` (old pin) | At the fix |
|---|---|---|
| `cexeccreate` | `cexecprobe: LEAK create fd=3` | green at `42203d8` |
| `rcloseprobe` | `LEAK` on all five legs | green at `fa38402` |

At `fa38402` the tier-1 gate gives 43 OK, 23 SKIP and 0 FAIL.

**The pin.** Harvey `c7cd176d2` changes only `flake.lock`. The locked rev is `fa38402ec3c9e18817b4d5c9c8b9052794dc2bab`, which equals r9 `origin/main`.

**Series at the pin** (no override, `R9_STOP_ON_FAIL=1`, 2026-11-20T18:39:32Z to 23:16:58Z). Harvey HEAD was `c7cd176d2` at start and end:

```
SERIES boot-r9-tier1 accel=hvf runs=10 pass=10 fail=0 pushword=0 forks=3760
SERIES boot-r9-net-smoke accel=hvf runs=36 pass=36 fail=0 pushword=0 forks=2556
SERIES boot-r9-tier1 accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=1128
SERIES boot-r9-net-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=213
SERIES boot-r9-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=24
SERIES boot-r9-smoke accel=hvf runs=1753 pass=1753 fail=0 pushword=0 forks=14024
```

- Every run booted `y1gq491k…-r9-kernel-virt-fa38402` (ELF sha256 `bd0d9696…`), and `nix build --rebuild` reproduces it.
- No capture has a `pushword`, `Not registered`, `user fault`, `unresolvable` or `panic` line.
- The last tier-1 capture has `T1-OK forkstress`, `writeprobe`, `cexecprobe`, `cexeccreate`, `rcloseprobe` and `brkprobe` once each, and 0 `T1-FAIL`.
- The tier-1 N restatement: 376 forks per boot, minus forkstress's 200, gives 176. The formula gives 3, so N = max(10, 3) = 10.

**Donor gate.** It passed twice:
- at `699f3dcf9`;
- again at `c7cd176d2`, after both sweep edits.

Both runs had 61 members, 0 FAIL, and none of the six r9-only markers. Recorded as `DONOR-GATE: pass 61 members`.

**Owner checks.**
- r9 xtask at `fa38402`: 8 of 8 jobs green, `build-aarch64-virtchurn` among them.
- Harvey at `34941cc0a`: r9-shell, phase6-userspace (`tier-r9` and `tier-9k` among them) and harvey green.
- The tier-9k capture is `verdict=PASS`, with 0 r9-only marker lines and 0 parse errors.
- Linux `nix build .#boot-iso --rebuild` at `fa38402` passed.

## Deviations from Plan

### Scope changes (owner-directed)

**1. The r9 push and the pin move were split.** Task 1's push needed owner approval. The owner chose option 3: push `7d464ad` now, and hold the pin for the debugger. The pin then moved once, to `fa38402`, not to Task 1's sha. The must-have "r9 main carrying group B is pushed, then Harvey's pin advances" still holds: `fa38402` is on `origin/main`.

**2. Two tier-1 entries were added that the plan did not name.**
- `T1-OK cexeccreate` (Harvey `68119abf8`) runs the debugger's `cexecprobe -c`.
- `T1-OK rcloseprobe` (Harvey `6fa102769`) runs the debugger's `rcloseprobe`. The debugger's `1e6d8e79f` built and baked it but left it out of the sweep.

The plan's "four probe markers" therefore became six, and the expected set 43.

**3. A port/src edit entered group B.** The ORCLOSE fix was owner-approved. The 20-06 hold applied until the owner's linux boot-iso pass at `fa38402`. Harvey was pushed only after that pass and the owner's approval.

**4. The Harvey push carried the debugger's commits.** `699f3dcf9..34941cc0a` holds 9 of this plan's commits and 7 debugger commits. The owner checked the list before approving. The checkpoint's "11 and 5" split was a miscount, corrected in the evidence file.

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The virtchurn boot-test hung in the first battery clone.**
- **Found during:** Task 1.
- **Issue:** the scratchpad clone's console socket path was 145 bytes, over macOS's 104-byte `sun_path` limit. qemu, with `wait=on`, never got its client, and both processes sat at 0% CPU for about 59 minutes.
- **Fix:** I killed my two processes (checked by cwd) and re-ran from a short-path clone under `.build/`, where it passed in 31 s. Every later battery used a short path. This was a venue issue, not a kernel defect, and no repo file changed.

**2. [Rule 2 - Correctness] The donor gate was re-run after the sweep edits.** Its first run predated `68119abf8` and `6fa102769`, so it no longer proved the final shared sweep. The re-run at `c7cd176d2` passed.

### Notes

- **`commits`** is the measured Harvey count `699f3dcf9..HEAD` at the time of writing. It includes the 7 debugger commits that landed on `master` during the plan; 10 are this plan's. The final docs commit adds one more. The r9 commit `7d464ad` is in the r9 repo.
- **`actuals.tokens`** is chars/4 over this plan's own added diff lines: the sweep, tier1-r9.sh, flake.lock, the evidence file, STATE.md and the r9 xtask.yml. The debugger's probe sources are excluded.
- **During the series, no tracked Harvey file was edited.** The owner's boot-iso result arrived mid-series and was recorded after it.
- **No x86_64 kernel byte change was made by this plan.** The debugger's two x86_64 changes came with their own boot-iso hash bumps (`42203d8`, `fa38402`) and their own x64 TCG gate runs. The owner confirmed both boot-iso hashes on the linux host.

## Known Stubs

None.

## Threat Flags

None. The register's mitigations held:
- **T-20-28:** `fa38402` equals `origin/main` and the locked rev.
- **T-20-29:** the owner's linux boot-iso passed at `fa38402`, keyed by the recorded port/src diff.
- **T-20-30:** all staging used explicit paths, and the 11 font entries stayed untouched.
- **T-20-SC:** only the r9 lock node changed.

## Self-Check: PASSED

- FOUND: r9 `7d464ad` on `origin/main`, an ancestor of `fa38402`. `xtask.yml` has `build-aarch64-virtchurn`.
- FOUND (ancestors of HEAD): Harvey `a71a45b8c`, `c3e601f89`, `68119abf8`, `7bddb3e02`, `c7044a68b`, `a55fec5cb`, `6fa102769`, `c7cd176d2`, `34941cc0a`, `e717ff53f`.
- The pin commit's file list is exactly `flake.lock`. Its locked rev, `fa38402ec3c9e18817b4d5c9c8b9052794dc2bab`, equals r9 `origin/main`.
- The plan's series check prints `N=1753 hvf=1 tcg=1`, `N=36 hvf=1 tcg=1` and `N=10 hvf=1 tcg=1`. The Group B section has exactly one `HARVEY-PUSH: done`. `git status --porcelain | grep -c '^ M sys/'` is 11.
