---
schema_version: 1
open_count: 6
waived_count: 0
fixed_count: 4
total_count: 10
last_updated: 2026-11-24T14:45:25.044Z
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
| 6 | 03 | stub | ds/src/tpht/mod.rs | 1 | tpht module is a doc-only skeleton; implementation lands in plan 03-06 (DS-01..DS-05) | open |  | 2026-11-24T14:27:39.839Z |  |
| 7 | 03 | stub | ds/src/roaring.rs | 1 | roaring module is a doc-only skeleton; implementation lands in plan 03-05 (DS-06) | fixed |  | 2026-11-24T14:27:40.129Z | 2026-11-24T14:45:24.496Z |
| 8 | 03 | stub | ds/src/setops.rs | 1 | setops seam is a doc-only skeleton; the four hot loops land in plan 03-05 (D-20) | fixed |  | 2026-11-24T14:27:40.406Z | 2026-11-24T14:45:24.771Z |
| 9 | 03 | stub | ds/src/clientset.rs | 1 | clientset module is a doc-only skeleton; implementation lands in plan 03-05 (DS-06) | fixed |  | 2026-11-24T14:27:40.689Z | 2026-11-24T14:45:25.044Z |
| 10 | 03 | deviation | flake.nix |  | Plan 03-02 Task 2 appends $R9_MIRI_TOOLCHAIN/bin to PATH in the devshell shellHook (never prepends, so the pinned cargo/rustc are not shadowed) so the plan's literal MIRI-VERSION verify finds cargo-miri; recorded in 03-02-SUMMARY deviations | open |  | 2026-11-24T14:32:01.421Z |  |

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
  },
  {
    "id": 6,
    "kind": "stub",
    "phase": "03",
    "file": "ds/src/tpht/mod.rs",
    "line": 1,
    "description": "tpht module is a doc-only skeleton; implementation lands in plan 03-06 (DS-01..DS-05)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-24T14:27:39.839Z",
    "resolved_at": null,
    "milestone": "v3.0"
  },
  {
    "id": 7,
    "kind": "stub",
    "phase": "03",
    "file": "ds/src/roaring.rs",
    "line": 1,
    "description": "roaring module is a doc-only skeleton; implementation lands in plan 03-05 (DS-06)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-24T14:27:40.129Z",
    "resolved_at": "2026-11-24T14:45:24.496Z",
    "milestone": "v3.0"
  },
  {
    "id": 8,
    "kind": "stub",
    "phase": "03",
    "file": "ds/src/setops.rs",
    "line": 1,
    "description": "setops seam is a doc-only skeleton; the four hot loops land in plan 03-05 (D-20)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-24T14:27:40.406Z",
    "resolved_at": "2026-11-24T14:45:24.771Z",
    "milestone": "v3.0"
  },
  {
    "id": 9,
    "kind": "stub",
    "phase": "03",
    "file": "ds/src/clientset.rs",
    "line": 1,
    "description": "clientset module is a doc-only skeleton; implementation lands in plan 03-05 (DS-06)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-24T14:27:40.689Z",
    "resolved_at": "2026-11-24T14:45:25.044Z",
    "milestone": "v3.0"
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "03",
    "file": "flake.nix",
    "line": null,
    "description": "Plan 03-02 Task 2 appends $R9_MIRI_TOOLCHAIN/bin to PATH in the devshell shellHook (never prepends, so the pinned cargo/rustc are not shadowed) so the plan's literal MIRI-VERSION verify finds cargo-miri; recorded in 03-02-SUMMARY deviations",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-24T14:32:01.421Z",
    "resolved_at": null,
    "milestone": "v3.0"
  }
]
````
