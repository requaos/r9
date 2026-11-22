---
phase: 01
review: 01-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: open
    title: "boot-9k10cpu.sh is executable with no shebang and requires bash — documented direct invocation breaks on dash-sh systems"
  - id: WR-01
    severity: warning
    disposition: open
    title: "User-supplied invalid KERNDATE is silently replaced by wall-clock time"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Userland preflight sentinel can deadlock the only documented repair path"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Serial capture attach is unsynchronized — lost race silently drops the boot banner"
  - id: WR-04
    severity: warning
    disposition: open
    title: "`checkall` does not check its `cd`s — a failed cd mis-attributes or mis-builds everything after it"
  - id: IN-01
    severity: info
    disposition: open
    title: ".gitignore misses artifacts newly produced by the host-generator mkfile pattern"
  - id: IN-02
    severity: info
    disposition: open
    title: "build/mkdirs declares `#!/bin/sh` but is bash-only"
  - id: IN-03
    severity: info
    disposition: open
    title: "KERNDATE policy is duplicated across four sites; mkfile fragment mishandles set-but-empty KERNDATE"
  - id: IN-04
    severity: info
    disposition: open
    title: "Unescaped path interpolation in the .mm sed render"
  - id: IN-05
    severity: info
    disposition: open
    title: "`build_aarch64` is dead scaffolding that fails confusingly if ever reached"
open: 10
total: 10
recorded: 2026-10-31T13:10:52.570Z
---

# Phase 01: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
