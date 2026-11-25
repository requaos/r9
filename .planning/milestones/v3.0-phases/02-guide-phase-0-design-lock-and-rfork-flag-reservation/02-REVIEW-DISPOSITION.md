---
phase: 02
review: 02-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "The one-definition invariant and the threat-model checks run in no gate and are not compiled in CI"
  - id: WR-02
    severity: warning
    disposition: open
    title: "The \"one definition\" invariant scans four hardcoded files and one exact spelling"
  - id: WR-03
    severity: warning
    disposition: skipped
    title: "The generated header has no include guard, so a second `#include` is a compile error"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Section 7 now misstates the repository about rfork bits 18-20"
  - id: IN-01
    severity: info
    disposition: open
    title: "The threat-model test hard-fails on any tree without `.planning/`"
  - id: IN-02
    severity: info
    disposition: open
    title: "The compendium comparison searches past section 2.2"
  - id: IN-03
    severity: info
    disposition: open
    title: "The re-measured FOD hash has no provenance beside it"
  - id: IN-04
    severity: info
    disposition: open
    title: "`rforkh_mutated_render_differs` is already covered by the staleness test"
open: 7
total: 8
recorded: 2026-11-24T05:10:00Z
---

# Phase 02: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | skipped | by design: 02-01 Task 2 specifies no include guard (Harvey's libc.h has none) and the header banner forbids including it beside libc.h; a guard would change the generated contract |
| WR-04 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
