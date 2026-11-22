# Phase 17: r9 Kernel Flush-Out - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-12
**Phase:** 17-r9-kernel-flush-out
**Areas discussed:** Fix-vs-disposition split, Pin architecture per class, Adjacent windows scope, Ledger closure shape

---

## Fix-vs-disposition split

| Option | Description | Selected |
|--------|-------------|----------|
| Pin-first, fix-on-failure (Recommended) | Build x86_64 pins for all 12 already-fixed shared-code findings; a finding reopens as a fix-item only if its pin fails; the pins ARE the audit | ✓ |
| Audit-then-pin | Researcher re-reads each of the 12 fix sites in the x64 tree before any pin is written | |
| Disposition-as-carried | Mark all 12 carried-fixed with aarch64 pins cited; no x64 re-proof | |

**User's choice:** Pin-first, fix-on-failure
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Audit + port + prove (Recommended) | Audit x86_64/src/proc.rs against the aarch64 fix family, port what's missing, THEN the leak-repro gates run | ✓ |
| Prove-only | Build leak-repro gates first, fix only what they catch | |

**User's choice:** Audit + port + prove
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Root-cause + fix + pin (Recommended) | The phase's centerpiece; #66 gives a live x64 repro shape; fix lands with a race-stress gate | ✓ |
| Fix-or-gated-disposition | Attempt fix; if intractable, explicit disposition + wedge-DETECTION gate | |
| Disposition-only | Document as known-open | |

**User's choice:** Root-cause + fix + pin
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Fix live classes (Recommended) | Fix where code is LIVE on x86_64 (WaitSet, used-id retire, cap-walk holes audited on the x64 ECAM walk); WR-08 + WR-07 get frozen/superseded dispositions | ✓ |
| Conservative — only provably-live | Fix only what x64 provably shares; ambiguous gets disposition | |
| Disposition-all | No Phase-7 code fixes; written rationale for every WR | |

**User's choice:** Fix live classes
**Notes:** None

---

## Pin architecture per class

| Option | Description | Selected |
|--------|-------------|----------|
| Host-first, boot-when-needed (Recommended) | Cheapest pin that can actually fail: host cargo test where host-compilable (devip cfg(test) precedent exists); boot profile only where kernel context essential | ✓ |
| Always both | Every finding lands host test AND boot gate | |
| Boot gates only | Mirror the aarch64 Phase-8 discipline | |

**User's choice:** Host-first, boot-when-needed
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Tiered: r9 pin + composed churn (Recommended) | r9-side boot profile (marker-binary exec loop, memory-bound assert) PLUS one Harvey composed churn leg (the #44 /bin/cat ×1400 shape) | ✓ |
| r9-side only | Self-contained in r9; may miss ByteBacking::Mnt-only drains | |
| Composed only | Real-load fidelity but harder localization | |

**User's choice:** Tiered: r9 pin + composed churn
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Deterministic + stress (Recommended) | Host-deterministic tests for race logic + in-kernel stress legs (green-twice-plus on KVM) | ✓ |
| Stress-only | Boot profile hammering contention shape | |
| Deterministic-only | Fastest, never proves under real IRQ timing | |

**User's choice:** Deterministic + stress
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Rules here, mapping to research (Recommended) | CONTEXT locks venue rules; researcher maps each finding to its concrete pin; planner sequences | ✓ |
| Full table now | Lock per-finding pin table in CONTEXT immediately | |

**User's choice:** Rules here, mapping to research
**Notes:** None

---

## Adjacent windows scope

| Option | Description | Selected |
|--------|-------------|----------|
| Fold all three (Recommended) | #47 + #50 + #66 formally join the CR-07 work item; one root-cause pass, close or disposition together | ✓ |
| #47+#66, #50 separate | #50 may be a different class (devpipe read vs RX wake) | |
| CR-07 only | Window rows stay open regardless | |

**User's choice:** Fold all three
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Fix in 17 (Recommended) | Last live namec/pgrp wedge, same subsystem as #45; 18/19 will contend it hard | ✓ |
| Disposition, fix later | Fix belongs to the phase that first contends | |
| Research-gated | Size it first; small joins 17, else disposition | |

**User's choice:** Fix in 17
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to userspace (Recommended) | rc rows are Harvey userspace; ride phase 19 or a future pass | |
| Fix in-phase | Tiny and rc is the load driver — close as a side item | ✓ |

**User's choice:** Fix in-phase (**deliberate override of the recommendation**)
**Notes:** Captured as D-11 with rationale "rc is the load driver" — a flush-out phase that leaves the load driver flaky hasn't flushed.

| Option | Description | Selected |
|--------|-------------|----------|
| Close-what-you-touch (Recommended) | Only rows 17 actually touches close; other ~45 stay as-is | ✓ |
| Full re-triage pass | One plan re-triages all 55 open rows | |
| Appendix notes only | Researcher appends triage notes without changing statuses | |

**User's choice:** Close-what-you-touch
**Notes:** None

---

## Ledger closure shape

| Option | Description | Selected |
|--------|-------------|----------|
| Flip rows in-place (Recommended) | Hand-edit 07/08 REVIEW-DISPOSITION rows; Source cites the x64 pin or rationale | ✓ |
| New doc supersedes | A 17-side disposition doc replaces the ledgers | |
| WINDOWS only | Review ledgers untouched | |

**User's choice:** Flip rows in-place
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Native resolve fields (Recommended) | status→resolved, reason = r9 SHA + pin name, resolved_at stamped | ✓ |
| Prose appendix | Closure list in prose | |

**User's choice:** Native resolve fields
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| 3-part test (Recommended) | WHERE the class lives / WHY not fixed / WHAT reopens it | ✓ |
| One-line reason | Faster, weaker decision record | |

**User's choice:** 3-part test
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| 17-FLUSHOUT.md table (Recommended) | One row per finding/window: action, pin, commit, ledger refs; VERIFICATION walks it | ✓ |
| VERIFICATION only | No new artifact; verification reads ledgers directly | |

**User's choice:** 17-FLUSHOUT.md table
**Notes:** None

---

## Claude's Discretion

Host-test placement/names, boot-profile names/shapes, composed churn app shape, CR-07 investigation methodology (within the memory-side-probe doctrine), cap-walk audit depth, wave decomposition, 17-FLUSHOUT.md columns, rc fix shapes, CI placement of new host tests.

## Deferred Ideas

Full WINDOWS.md re-triage; console RX-ring truncation class (#48) absent a live x64 symptom; era notes (#13/#53/#54/#55) unless collided with; r9-side tap profiles for their own sake (16-D-07 offer stands); any aarch64-side fix (frozen — disposition only).
