---
schema_version: 1
open_count: 4
waived_count: 0
fixed_count: 1
total_count: 5
last_updated: 2026-11-24T02:35:39.402Z
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
| 4 | 01 | deviation | gates/aarch64/drive-r9.expect |  | Plan 01-21's unreferenced drive-r9 rewire needed two support lines beyond path/env tokens (the R9_GATE_OUT capture expression and its mkdir) because the Harvey launcher run-r9.sh was not moved; every assertion, marker, stage and timeout is unchanged (recorded in 01-21-SUMMARY) | open |  | 2026-11-23T12:03:50.195Z |  |
| 5 | 02 | unrun-verify | x86_64/src/syscall.rs |  | The x86_64 rfork_reserved_bits_are_unsupported mask test and the RFPROC import in x86_64/src/proc.rs mod tests were not compiled or run on the Mac; first witness is the owner's x86_64 Linux host at the end-of-phase stop (D-24, plan 02-06) | open |  | 2026-11-24T02:35:39.402Z |  |

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
  },
  {
    "id": 4,
    "kind": "deviation",
    "phase": "01",
    "file": "gates/aarch64/drive-r9.expect",
    "line": null,
    "description": "Plan 01-21's unreferenced drive-r9 rewire needed two support lines beyond path/env tokens (the R9_GATE_OUT capture expression and its mkdir) because the Harvey launcher run-r9.sh was not moved; every assertion, marker, stage and timeout is unchanged (recorded in 01-21-SUMMARY)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-23T12:03:50.195Z",
    "resolved_at": null,
    "milestone": "v3.0"
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "02",
    "file": "x86_64/src/syscall.rs",
    "line": null,
    "description": "The x86_64 rfork_reserved_bits_are_unsupported mask test and the RFPROC import in x86_64/src/proc.rs mod tests were not compiled or run on the Mac; first witness is the owner's x86_64 Linux host at the end-of-phase stop (D-24, plan 02-06)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-24T02:35:39.402Z",
    "resolved_at": null,
    "milestone": "v3.0"
  }
]
````
