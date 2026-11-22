---
phase: 14
review: 14-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "check_seed_fit does not mirror seed_user_pages — exec with argv in the gap kills the process post-free instead of returning -1"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "The ax@0 frozen-ABI const assert is swallowed by a line comment — the witness does not compile, and ABI.md's enforcement claim is false for ax"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "l.S module-tag walk performs no minimum tag-size validation — a corrupt size-0 tag loops forever at 32-bit boot, and module-header reads can run 4 bytes past the stash"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "ABI.md's enforcement appendix contradicts the checker it freezes — stale `../harvey` default, wrong pinned-table size, wrong mutation medium"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "`sched: park` println on every preemption — debug artifact in the scheduler hot path"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "Dead code: `touser` and `UTZERO` are never called, and their \"unused until 14-05\" comments are now false"
  - id: IN-03
    severity: info
    disposition: skipped
    title: "Impossible-state guard — check_data_bss_below_ustop cannot fire for any parseable header"
open: 0
total: 7
recorded: 2026-11-09T17:39:15.187Z
---

# Phase 14: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 14-REVIEW-FIX.md |
| WR-01 | warning | fixed | 14-REVIEW-FIX.md |
| WR-02 | warning | fixed | 14-REVIEW-FIX.md |
| WR-03 | warning | fixed | 14-REVIEW-FIX.md |
| IN-01 | info | fixed | 14-REVIEW-FIX.md |
| IN-02 | info | fixed | 14-REVIEW-FIX.md |
| IN-03 | info | skipped | 14-REVIEW-FIX.md |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
