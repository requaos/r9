---
phase: 04
review: 04-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Demand pager silently corrupts data when bss faults precede data on the shared last data page"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`map_to` error path abandons the recursive slot-511 restore"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "rfork_clone / new_user_proc error paths leave a runnable null-frame child and leak pages"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "RFNOWAIT child exit corrupts the parent's AWAIT pivot forever"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "NOTED's NCONT/NRSTR restores `psr` from user-writable memory without sanitizing to EL0t"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "deliver_notes has no in-handler guard — a second note overwrites `note_ureg` mid-handler"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "fault-kill paths bypass the 04-07 lifecycle — parent AWAIT hangs, kernel wedges with IRQs masked"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "exec_replace's \"seed can no longer fail\" invariant is false — post-free failure strands the proc"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "user fault on a top-half VA panics the kernel (user-reachable DoS)"
  - id: IN-01
    severity: info
    disposition: open
    title: "Stale tick-rate comment in the scheduler header"
  - id: IN-02
    severity: info
    disposition: open
    title: "Stale \"never freed in the single-process era\" comment on Proc's raw-pointer fields"
  - id: IN-03
    severity: info
    disposition: open
    title: "Brace + first statement jammed on one line"
  - id: IN-04
    severity: info
    disposition: open
    title: "devcons pread advances the streaming offset even for explicit-offset reads"
  - id: IN-05
    severity: info
    disposition: open
    title: "map_one_user_page comment says \"(inactive)\" but exec_replace calls it on the active table"
  - id: IN-06
    severity: info
    disposition: open
    title: "trace ENTRY_RE silently drops zero-argument syscall fragments"
  - id: IN-07
    severity: info
    disposition: open
    title: "trace Log ring trim invalidates absolute search bases"
  - id: IN-08
    severity: info
    disposition: open
    title: "sha256 fallback only covers spawn failure, not tool failure"
open: 8
total: 17
recorded: 2026-11-02T20:10:18.530Z
---

# Phase 04: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 04-REVIEW-FIX.md |
| WR-01 | warning | fixed | 04-REVIEW-FIX.md |
| WR-02 | warning | fixed | 04-REVIEW-FIX.md |
| WR-03 | warning | fixed | 04-REVIEW-FIX.md |
| WR-04 | warning | fixed | 04-REVIEW-FIX.md |
| WR-05 | warning | fixed | 04-REVIEW-FIX.md |
| WR-06 | warning | fixed | 04-REVIEW-FIX.md |
| WR-07 | warning | fixed | 04-REVIEW-FIX.md |
| WR-08 | warning | fixed | 04-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| IN-08 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
