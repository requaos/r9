---
phase: 15
review: 15-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Unvalidated syscall number indexes `systab` out of bounds in the newly-wired trace path (kernel OOB read → panic / kernel-memory leak)"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`startns` read uninitialized when the return-side trace arm happens mid-syscall"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "`-initrd` module basename overflows the fixed-size `Dirtab.name` in `addbootfile`"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "drawsmoke's pixel-readback assert does not accept \"either canonical byte order\" and false-passes palindromic garbage"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "GS-RENDER-OK ignores gs's exit status — a partial render passes the D-11 smoke"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "Venue harnesses overwrite the committed phase evidence on failing runs"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "tier-2 CI leg's failure path is dead — `cmd | tee || handler` without pipefail"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "The MUL2 round-to-nearest identity only holds for `s <= 65025`; the \"byte path equally covered\" claim overstates"
  - id: IN-01
    severity: info
    disposition: open
    title: "Lockless `/proc/N/syscall` read races the new per-syscall free sites"
  - id: IN-02
    severity: info
    disposition: open
    title: "Stale contradictory comment in the committed sam replay driver"
  - id: IN-03
    severity: info
    disposition: open
    title: "Dead no-op loop in build-9kdraw.sh"
  - id: IN-04
    severity: info
    disposition: open
    title: "Off-by-one entry count in the k10t1 config comment"
  - id: IN-05
    severity: info
    disposition: open
    title: "coverage-sweep \"worst status wins\" is order-dependent between fail and hang"
  - id: IN-06
    severity: info
    disposition: open
    title: "The passwordless (userns) venue is NixOS-only via hardcoded `/run/current-system/sw/bin`"
  - id: IN-07
    severity: info
    disposition: open
    title: "services-r9.expect degrades to a 900s stage timeout instead of the documented post-pass classification"
open: 7
total: 15
recorded: 2026-11-10T20:59:32.547Z
---

# Phase 15: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 15-REVIEW-FIX.md |
| WR-01 | warning | fixed | 15-REVIEW-FIX.md |
| WR-02 | warning | fixed | 15-REVIEW-FIX.md |
| WR-03 | warning | fixed | 15-REVIEW-FIX.md |
| WR-04 | warning | fixed | 15-REVIEW-FIX.md |
| WR-05 | warning | fixed | 15-REVIEW-FIX.md |
| WR-06 | warning | fixed | 15-REVIEW-FIX.md |
| WR-07 | warning | fixed | 15-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
