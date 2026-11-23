---
schema_version: 1
open_count: 0
waived_count: 0
fixed_count: 1
total_count: 1
last_updated: 2026-11-23T05:41:31.253Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | deviation | .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/mac-builder-probe.txt |  | Plan 01-12 added a sixth evidence file beyond files_modified so the mac-builder VERDICT has a real EVIDENCE path | fixed |  | 2026-11-23T05:41:27.470Z | 2026-11-23T05:41:31.253Z |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "01",
    "file": ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/mac-builder-probe.txt",
    "line": null,
    "description": "Plan 01-12 added a sixth evidence file beyond files_modified so the mac-builder VERDICT has a real EVIDENCE path",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-23T05:41:27.470Z",
    "resolved_at": "2026-11-23T05:41:31.253Z",
    "milestone": "v3.0"
  }
]
````
