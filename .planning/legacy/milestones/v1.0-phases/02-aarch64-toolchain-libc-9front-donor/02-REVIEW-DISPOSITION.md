---
phase: 02
review: 02-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "run-gate.sh's \"scaffolding-free\" assertion is stale — the TC-03 gate now always fails (after appending PASS evidence)"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "pre-gate-build.sh carries the same stale assertion and is obsolete-but-undocumented"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "nsrun.sh root mode runs the entire bind/staging sequence without `set -e`"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "gdbpoke9.py's 120s default deadline is far below the 600–1500s boot budgets — false gate failures on slow TCG boots"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "drive9.py's \"stronger\" prompt proof is satisfied by console echo alone; SHELL_RE is one 5-byte drop away from a false positive"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "9cc/BUILD has no `set -e` — mid-build failures can exit 0 against stale archives"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "9cc README's executable.c provenance omits the LP64 commonllp64 rewrite"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "donor-vm-build.sh's stale-object sweep deletes two tracked donor files"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "vm-build.sh reports success when the guest build fails"
  - id: WR-09
    severity: warning
    disposition: fixed
    title: "nix/build.sh lacks `set -e`; sudo-cp installs and the namespace build both fail softly"
  - id: WR-10
    severity: warning
    disposition: fixed
    title: "build/mkdirs declares `#!/bin/sh` but is bash (arrays)"
  - id: IN-01
    severity: info
    disposition: open
    title: "7a/BUILD misses 7c/BUILD's stale-object sweep"
  - id: IN-02
    severity: info
    disposition: open
    title: "7l/mkfile defines CFILES that nothing consumes"
  - id: IN-03
    severity: info
    disposition: open
    title: "\"term%%\" printed literally in all three serial drivers"
  - id: IN-04
    severity: info
    disposition: open
    title: "srv9.py's fid table is server-global, not per-connection"
  - id: IN-05
    severity: info
    disposition: open
    title: "srv9.py Twstat rename silently deletes an existing target; no Topen-before-I/O enforcement"
  - id: IN-06
    severity: info
    disposition: open
    title: "nsrun.sh exports file is world-readable in /tmp and sourced as root"
  - id: IN-07
    severity: info
    disposition: open
    title: "unixcompat.c calloc multiplication overflow / malloc(0) corner"
  - id: IN-08
    severity: info
    disposition: open
    title: "mkfs9.py crashes on a bare `d` proto mode (latent; unreachable with pinned protos)"
  - id: IN-09
    severity: info
    disposition: open
    title: "BOOTFSPROTOS splice hardcodes line numbers while the guard checks only presence"
  - id: IN-10
    severity: info
    disposition: open
    title: "cc/compat.h has no double-include guard though the vendored tools are also host-compiled"
  - id: IN-11
    severity: info
    disposition: open
    title: "three ~250-line forks of the serial driver diverge already"
  - id: IN-12
    severity: info
    disposition: open
    title: "dumpdtb relies on a fixed 2-second sleep"
  - id: IN-13
    severity: info
    disposition: open
    title: "pcc.c `allos` still lacks `4` and `x` though the table has mips64/spim64 rows (pre-existing)"
open: 13
total: 24
recorded: 2026-11-01T19:11:52.535Z
---

# Phase 02: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 02-REVIEW-FIX.md (re-titled heading; verified same finding — fix commit 064c6a185) |
| WR-01 | warning | fixed | 02-REVIEW-FIX.md |
| WR-02 | warning | fixed | 02-REVIEW-FIX.md |
| WR-03 | warning | fixed | 02-REVIEW-FIX.md (re-titled heading; verified same finding — fix commit f494b7727) |
| WR-04 | warning | fixed | 02-REVIEW-FIX.md |
| WR-05 | warning | fixed | 02-REVIEW-FIX.md |
| WR-06 | warning | fixed | 02-REVIEW-FIX.md |
| WR-07 | warning | fixed | 02-REVIEW-FIX.md |
| WR-08 | warning | fixed | 02-REVIEW-FIX.md |
| WR-09 | warning | fixed | 02-REVIEW-FIX.md |
| WR-10 | warning | fixed | 02-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| IN-08 | info | open | - |
| IN-09 | info | open | - |
| IN-10 | info | open | - |
| IN-11 | info | open | - |
| IN-12 | info | open | - |
| IN-13 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
