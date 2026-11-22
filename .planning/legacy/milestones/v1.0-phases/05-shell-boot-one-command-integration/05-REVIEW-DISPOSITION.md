---
phase: 05-shell-boot-one-command-integration
source: 05-REVIEW.md
updated: 2026-11-03T11:35:00Z
dispositions:
  open: 23
  fixed: 0
  skipped: 0
  deferred: 0
---

# Phase 05 Review Disposition Ledger

One row per finding from 05-REVIEW.md. Default disposition is `open` until triaged.
To triage: change the Disposition column to `fixed`, `skipped`, or `deferred` and put
the reason in the Source column (bare pipes in reasons must be escaped as `\|`).

| ID | Severity | Disposition | Source |
|----|----------|-------------|--------|
| CR-01 | critical | open | Six devices serve malformed stat(5) records (no muid string) |
| CR-02 | critical | open | sys_stat/sys_fstat skip the cold-buffer residency step — user-reachable kernel halt |
| WR-01 | warning | open | consctl parser silently ignores newline-terminated words |
| WR-02 | warning | open | sys_sleep wraps negative arguments into ~49-day sleeps |
| WR-03 | warning | open | cooked-mode ^D on an empty line never yields EOF |
| WR-04 | warning | open | pgrp lock held across blocking devmnt RPCs in most namespace syscalls |
| WR-05 | warning | open | new_user_proc leaks the root-table and kstack pages on mid-construction failure |
| WR-06 | warning | open | exec accepts truncated binaries (zero-filled text tail) and devmnt read_all rejects an exact-cap file |
| WR-07 | warning | open | pkill teardown patterns kill foreign r9 qemu processes |
| WR-08 | warning | open | devroot directories are unreadable |
| IN-01 | info | open | exec arg-count bounds disagree between the syscall layer and the loader |
| IN-02 | info | open | HowItWorks.md pins stale constants |
| IN-03 | info | open | /usr/bin/grep hardcoded in the build scripts |
| IN-04 | info | open | devsrv fd-number parse wraps on oversized input |
| IN-05 | info | open | trace-syscalls.py leaks a socket per poll iteration |
| IN-06 | info | open | CI workflow supply-chain rule applied narrowly; no cache/concurrency controls |
| IN-07 | info | open | allocator free_misc carve can register a zero-size block |
| IN-08 | info | open | formatting glitch in fault.rs |
| IN-09 | info | open | stale MAX_PROCS value in a proc.rs comment |
| IN-10 | info | open | leftover "XXX: Just for testing." markers in shipped devcons code |
| IN-11 | info | open | expect drivers open the capture after spawn |
| IN-12 | info | open | DEL does not wake already-blocked secondary console readers |
| IN-13 | info | open | flake.lock carries two nixpkgs pins |
