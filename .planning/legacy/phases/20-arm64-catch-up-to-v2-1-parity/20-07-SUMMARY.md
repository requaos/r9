---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 07
subsystem: arm64 tier-1 image (ramfs venue + group-B probes), r9 aarch64 measurements
tags: [r9, aarch64, ramfs, writeprobe, rfork, rffdg, rfcfdg, devmnt, dmdir, union-create, brk, rfmem, d-09, measurement]
status: complete
requires: ["20-06"]
provides:
  - "/arm64/bin/ramfs, /n/wp, /lib/writeprobe.rc, /arm64/bin/rfdprobe and /arm64/bin/brkprobe in the arm64 tier-1 image, with no gate wired to them"
  - "measured red steps on the group-A kernel: RED-STEP b4 fdsplit, b5 write, b6 mkdir, b7 union-create (HVF = TCG)"
  - "A-brk measured: the lone-member RFMEM brk arm works on aarch64 by demand paging (10/10 runs)"
  - "owner decision D-09 = ramfs-venue-brk-holds"
affects: [20-08, 20-10]
tech-stack:
  added: []
  patterns:
    - "an rc probe step that forks a long-lived or table-sharing child is not captured by backquote; report through $status"
    - "a one-line uncommitted expect driver (.build/p20-07-probe.*) for typing a single probe line at the r9 prompt"
key-files:
  created:
    - build/aarch64/rfdprobe.c
    - build/aarch64/brkprobe.c
    - build/aarch64/root-tree/lib/writeprobe.rc
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-07-SUMMARY.md
  modified:
    - build/aarch64/subset-build.sh
    - build/aarch64/mkimage.sh
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/deferred-items.md
key-decisions:
  - "Owner D-09: ramfs-venue-brk-holds (ramfs is the venue for b4-b7; the lone-member brk arm is not ported; brkprobe is a regression check only)"
  - "Owner: the shared-fd-table pipe leak (finding 1) goes to a separate gsd-debugger session, not fixed in 20-07"
  - "Owner: keep core.ignorecase=false in the lima guest checkout"
requirements-completed: []  # contributes to ARM-07 (measurement); ARM-07 completes with 20-08..20-11
metrics:
  duration: "about 108 min (2026-11-19T13:57:46Z to 15:46Z), including the owner checkpoint"
  completed: 2026-11-19
actuals:
  tokens: 9200
  tasks: 3
  commits: 2
plan_head_before: 63c83854bae892584ef969e4d4943ce7c0328c6a
plan_head_after: a7b2b1d6711645992f3ffa92577c00d46cabe33a
---

# Phase 20 Plan 07: ramfs writable venue and brk measured on the group-A kernel. Red steps exist for b4-b7, and the lone-member brk needs no port. Summary

ramfs builds for arm64, mounts on the baked `/n/wp` and creates files on the pinned r9 `57e2952` kernel. The write-flow probe is red only at the inventory arms, with the same output on HVF and TCG:

```
RED-STEP b4: fdsplit       rfork: bad flags for non-RFPROC (rfdprobe FAIL rffdg + rfcfdg)
RED-STEP b5: write         devmnt: the mounted tree is read-only this era
RED-STEP b6: mkdir         create: directories unsupported this era
RED-STEP b7: union-create  create landed on the read-only first member, not /n/wp
(b13 remove: syscall not implemented)
```

brkprobe passed 10 of 10 runs (5 HVF, 5 TCG), plus the no-fork and live-family controls. The lone member's 8 MiB growth took exactly 2048 demand bss page-ins, so A-brk holds. The tier-1 gate (TCG, at the pin) still passes with all the probes baked in: 38 T1-OK and 23 T1-SKIP, the same as before. The owner chose `ramfs-venue-brk-holds`.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Measure the ramfs writable venue on arm64 | `4c43a2d80` | rfdprobe.c, writeprobe.rc, subset-build.sh, mkimage.sh, 20-EVIDENCE.md |
| 2 | Measure the lone-member RFMEM brk arm on aarch64 | `a7b2b1d67` | brkprobe.c, subset-build.sh, mkimage.sh, 20-EVIDENCE.md |
| 3 | Owner decision D-09 | this SUMMARY's commit | 20-EVIDENCE.md, deferred-items.md |

## Evidence

The full record is in `20-EVIDENCE.md` under `## Group B measurements`.

- **Write-flow image:** bootfs.paq sha256 `6824c143…`. The write-flow transcripts are byte-identical on HVF and TCG. Steps `mount` and `create` pass; the rest map to b4, b5 (write, and readback after it), b6, b7 and b13.
- **brk image:** sha256 `30f61249…`. No `user fault`, `unresolvable` or `panic` line in any capture.
- **Tier-1 gate:** passed on TCG with each image (the first had ramfs and rfdprobe; the second also had brkprobe), with 38 + 23 markers and `exit=0 ramfs-in-tier-file=0`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The lima guest's nix fetch failed on every build (`troff/Hx`)**
- **Found during:** Task 1, the first tier-1 gate run
- **Issue:** the build failed 3 times out of 3, so a re-run did not help. The guest copy's `.git` carried darwin's `core.ignorecase=true` on a case-sensitive disk, so git listed `Hx` as present although it was missing.
- **Fix:** `git config core.ignorecase false` in the guest's `~/harvey-build` only. No repo file changed and the 11 font files were not touched. The owner approved keeping it. The record is in deferred-items.md.

**2. [Rule 1 - Bug] The fdsplit step's capture hung**
- **Found during:** Task 1, the first HVF writeprobe run
- **Issue:** a backquote over rfdprobe never saw end of file. The cause is kernel finding 1 below.
- **Fix:** `fdsplit` now reports through `$status`, which is rfdprobe's own FAIL line. rfdprobe gained `-s` as the reproducer.
- **Commit:** `4c43a2d80`

**3. [Rule 1 - Bug] brkprobe reported false mismatches**
- **Found during:** Task 2, the first two builds
- **Issue:** the false mismatches looked like `want=0x15 got=0x15`. 7c on arm64 does not narrow a `uchar` function result.
- **Fix:** mask the pattern with `& 0xff`, with a comment on why. Added the `-p` and `-f` controls and want/got detail. All 24 earlier runs are marked invalid in the evidence, and all 10 runs were repeated on the fixed build.
- **Commit:** `a7b2b1d67`

**4. [Rule 1 - Bug] `requirements.mark-complete ARM-07` was reverted**
- **Issue:** ARM-07 ("every ARM-06 port has a live arm64 pin") is shared with 20-08 through 20-11. This plan only measured the red steps, so the checkbox would have been premature.
- **Fix:** REQUIREMENTS.md left at Pending. `requirements-completed: []`.

### Findings (owner-routed)

- **Finding 1, the shared-fd-table pipe leak:** an fd table shared by a plain `rfork(RFPROC)` child keeps a pipe writer open after every sharer has exited, so a backquote reader hangs. It reproduces on HVF and TCG with `rfdprobe -s`; the `fork()` and no-pipe controls complete. It is outside b4-b7/b13 and was **handed to a separate gsd-debugger investigation**. It is not fixed here.
- **7c uchar narrowing:** recorded in deferred-items.md.

## Known Stubs

None.

## Threat Flags

None. The probes are unprivileged test fixtures that no gate calls (T-20-16 accepted).

## Self-Check: PASSED

- FOUND: build/aarch64/rfdprobe.c, build/aarch64/brkprobe.c, build/aarch64/root-tree/lib/writeprobe.rc (tracked); `## Group B measurements` with 4 RED-STEP lines, 8 WP- lines within 40 lines of the heading, and an A-brk conclusion within 60 lines
- FOUND (ancestors of HEAD): `4c43a2d80`, `a7b2b1d67`
- tiers/tier1-cmds.txt unchanged; the 11 font entries are unstaged
