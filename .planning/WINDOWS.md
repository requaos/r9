---
schema_version: 1
open_count: 2
waived_count: 0
fixed_count: 1
total_count: 3
last_updated: 2026-11-23T11:15:32.026Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | deviation | .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-a/mac-builder-probe.txt |  | Plan 01-12 added a sixth evidence file beyond files_modified so the mac-builder VERDICT has a real EVIDENCE path | fixed |  | 2026-11-23T05:41:27.470Z | 2026-11-23T05:41:31.253Z |
| 2 | 01 | deviation | xtask/src/gate.rs |  | Task 2 (tdd=true) landed its behaviour tests with the implementation in one feat commit; no separate RED commit exists (recorded in 01-18-SUMMARY) | open |  | 2026-11-23T11:15:31.637Z |  |
| 3 | 01 | unrun-verify | xtask/src/gate.rs |  | The quick gate's KVM witness runs (green twice, force-no-rio non-zero) are queued for stop B (plan 01-22) | open |  | 2026-11-23T11:15:32.026Z |  |

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
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "01",
    "file": "xtask/src/gate.rs",
    "line": null,
    "description": "Task 2 (tdd=true) landed its behaviour tests with the implementation in one feat commit; no separate RED commit exists (recorded in 01-18-SUMMARY)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-23T11:15:31.637Z",
    "resolved_at": null,
    "milestone": "v3.0"
  },
  {
    "id": 3,
    "kind": "unrun-verify",
    "phase": "01",
    "file": "xtask/src/gate.rs",
    "line": null,
    "description": "The quick gate's KVM witness runs (green twice, force-no-rio non-zero) are queued for stop B (plan 01-22)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-23T11:15:32.026Z",
    "resolved_at": null,
    "milestone": "v3.0"
  }
]
````
