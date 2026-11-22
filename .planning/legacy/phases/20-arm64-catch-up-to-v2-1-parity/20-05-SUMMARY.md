---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 05
subsystem: donor venue (build/aarch64) + boot-chain trace distiller (build/harness)
tags: [arm64, donor, 9front, subset-exec, trace-syscalls, boot-chain, fversion, harness, rename]
status: complete
requires: ["20-04"]
provides:
  - "donor venue green after the rename: donor-vm-build.sh + subset-exec.sh exit 0, T1-DONE, 61 T1-OK, 0 forkstress lines"
  - "re-provision prerequisite documented in the donor-vm-build.sh header and build/aarch64/README.md"
  - "trace.rs frag_records: the boot-chain distiller splits glued console fragments at every tag"
  - "trace artifact defaults under .build/trace/ instead of the archived phase 04/05/06 paths"
affects: [20-06]
tech-stack:
  added: []
  patterns: ["split a serial line into its tagged records at every tag; never take rest-of-line after the first tag"]
key-files:
  created:
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/deferred-items.md
  modified:
    - build/aarch64/donor-vm-build.sh
    - build/aarch64/README.md
    - build/harness/src/trace.rs
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
key-decisions:
  - "D-06 is Branch A: research's failing run had one raw Fversion, glued to pid 1259's return fragment, and the old rest-of-line split skipped it as a return. The fix is in the distiller (frag_records), and REQUIRED_BC is unchanged"
  - "The boot-chain sweep-stage launch race in trace-boot.rc (1 of 6 runs) is out of scope and deferred to the owner, not fixed"
requirements-completed: [ARM-04]
metrics:
  duration: "about 27 min (12:01:44Z to 12:28Z)"
  completed: 2026-11-18
actuals:
  tokens: 7600
  tasks: 2
  commits: 2
plan_head_before: 9aae831a147d1971d4436846d67df5cd55673cf4
plan_head_after: 8781e101c38754475ec4fcb3a8ae96fc312548c5
---

# Phase 20 Plan 05: donor venue re-proven after the rename; boot-chain trace fversion miss diagnosed and fixed Summary

The donor venue passes after the `arm64` rename. `donor-vm-build.sh` and then `subset-exec.sh` both exit 0 with `T1-DONE` and 61 `T1-OK` markers, and the donor capture has zero `forkstress` lines. The re-provision trap is now documented next to the script and in the README. The boot-chain trace gate's `measured set lacks fversion` was a distiller defect (Branch A). Console fragments from concurrent per-pid trace procs get glued onto one line, and the old parser kept only the first record per line. `frag_records` now splits a line at every tag. The fixed harness passed three consecutive boot-chain runs, and each one exercised the `bind /boot/ourinit /arm64/init` path.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Donor re-provision, donor tier-1 gate, documented prerequisite | `053132965` | `build/aarch64/donor-vm-build.sh` (comments only), `build/aarch64/README.md`, `20-EVIDENCE.md` |
| 2 | Diagnose the boot-chain trace gate's fversion miss (D-06) | `8781e101c` | `build/harness/src/trace.rs`, `20-EVIDENCE.md`, `deferred-items.md` |

## Evidence

The full record is in `20-EVIDENCE.md`, sections `## Donor venue` and `## Boot-chain trace (D-06)`.

**Task 1**
- `provision exit=0`, `subset-exec exit=0`, `grep -c T1-DONE` printed 1. The donor pin is `350266104f1fed81c78c3d05fa44f523bec0ae4d`, verified by the script and by `rev-parse`.
- The gate's expected set is `56 members + float-d01 + libtest ×2 + selfhost ×2`. That is the same as research recorded before 20-03, and it matches the 61 distinct `T1-OK` names in the capture.
- `grep -cE 'T1-(OK|FAIL|SKIP) forkstress'` printed 0. The 20-03 r9-venue guard has no effect on the donor.
- The symptom text `No such file or directory: u.h` appears once in each of `README.md` and `donor-vm-build.sh`. The script diff has no non-comment lines.

**Task 2**
- Three-run table, pre-fix harness: base 1 PASS, base 2 FAIL at the sweep stage (not the distiller), base 3 PASS. Each run had one raw `Fversion` record, at a line start, and each raw log had 2 to 8 glued lines.
- Replaying the pre-fix distiller over research's saved log reproduces `measured set lacks fversion`. The fixed distiller measures fversion ×1 from the same bytes, and on every log it recovers the 1 or 2 entry records the old split dropped.
- Fixed harness (`v1anrpzg…-harness`): fix 1, fix 2 and fix 3 all PASS, `40 fversion (×1)`. These are the last three runs.
- `cargo test --manifest-path build/harness/Cargo.toml`: `test result: ok. 72 passed; 0 failed`. On a scratch copy with the old split restored, the two new tests FAIL with the gate's own message.
- `grep -c 'planning/phases/0[456]-' build/harness/src/trace.rs` printed 0. `REQUIRED_BC` still contains `fversion`. `git status --porcelain .planning/phases` showed only phase-20 files.

## Decisions Made

- **Branch A, fixed in `frag_records` / `parse_chain_region`.** The live miss did not recur in six runs, because fversion landed at a line start each time. The cause is still shown on research's failing bytes, both by replay and by the unit test.
- **Run 2 was stopped by hand.** About 6 minutes in, the sweep script had already aborted to `term%`, and the harness would only have timed out at 1800 s. I killed qemu, the harness and srv9. The run counts as a failure in the table.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] New default artifact dir does not exist on a fresh checkout**
- **Found during:** Task 2 (moving the defaults to `.build/trace/`)
- **Issue:** `std::fs::write` to `.build/trace/…` fails when the directory is missing. The old defaults pointed at directories that already existed.
- **Fix:** added `write_artifact`, which runs `create_dir_all` on the parent before writing, and used it at the three artifact write sites.
- **Files modified:** `build/harness/src/trace.rs`
- **Commit:** `8781e101c`

**2. [Rule 1 - Doc accuracy] The plan text has no README trace invocation, and `tier-exec.sh` does not exist**
- The plan points to the README for the trace command, but the README does not document one. I used research's invocation (`TRACE_OUT=… harness trace-syscalls --boot-chain`) and recorded it in the evidence. While drafting, I named `tier-exec.sh` as a donor gate, but no such script exists, so I dropped it before committing.

### Notes

- **Out of scope, deferred:** the sweep-stage launch race in `build/aarch64/trace-boot.rc:272`, logged in `deferred-items.md` with its cause and possible fixes. `gsd-tools windows append` refused the entry with the same pre-existing ledger count mismatch 20-04 hit (`52/3/13/68` vs `49/5/14/68`). The ledger is best-effort, so the entry lives in `deferred-items.md` and in this SUMMARY.
- `rustfmt` is not on PATH in Harvey's devshell, so `cargo fmt --check` could not run. The new code follows the file's existing formatting by hand.
- The devshell's `date` is plan9port's, so `run3.sh` printed 1970 timestamps. I took the run times from the file mtimes instead.
- `graphify update .` was run after each task. `graphify-out/` is gitignored. Nothing was pushed, and the pre-existing modified troff/postscript files were not staged.

## Known Stubs

None.

## Threat Flags

None. No new network, auth or file-access surface. The only new write path is the gitignored `.build/trace/` default.

## Self-Check: PASSED

- FOUND: `build/aarch64/donor-vm-build.sh`, `build/aarch64/README.md`, `build/harness/src/trace.rs`, `20-EVIDENCE.md`, `deferred-items.md`
- FOUND (ancestors of HEAD): `053132965`, `8781e101c`
- `git rev-list --count 9aae831a1..8781e101c` = 2
