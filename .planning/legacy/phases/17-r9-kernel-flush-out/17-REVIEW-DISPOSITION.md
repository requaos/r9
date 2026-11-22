---
phase: 17
review: 17-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "sys_brk_ regrowth over shrink-recorded pages creates duplicate mapping_log records and a stale semaphore PA key"
  - id: WR-02
    severity: warning
    disposition: open
    title: "conv_close's pending-incall sweep resets in-use convs unconditionally (donor closeconv decrements)"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Accept-pool pop leaks the member socket on two of its own error arms"
  - id: IN-01
    severity: info
    disposition: open
    title: "dev_churn's MAX_PROCS=2048 falsifies the \"WaitSet sized to the live proc table (512 on both arches)\" capacity claim"
  - id: IN-02
    severity: info
    disposition: open
    title: "churngate generation-count arithmetic in comments is wrong (39 generations, not 40)"
  - id: IN-03
    severity: info
    disposition: open
    title: "churngate spawn_and_wait's watchdog-expiry path leaves children_live elevated and a stale wait record possible"
  - id: IN-04
    severity: info
    disposition: open
    title: "sys_create's bare-path arm derives nomount but does not apply it to the dot-redirect consult"
  - id: IN-05
    severity: info
    disposition: open
    title: "Regenerated yacc output committed with developer-machine absolute paths (outside declared scope, present in the phase diff)"
open: 8
total: 8
recorded: 2026-11-13T07:01:52.374Z
---

# Phase 17: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
