---
phase: 19-cpu-service-drawterm-rio-desktop
padded: "19"
source: [19-REVIEW.md]
recorded: 2026-11-17
open: 0
fixed: 2
skipped: 1
deferred: 2
---

# Phase 19 Code Review Disposition

One row per finding in [19-REVIEW.md](19-REVIEW.md). Dispositions: `open` (untouched),
`fixed` (a fix landed), `skipped` (won't fix, reason in Source), `deferred` (reopener
recorded, reason in Source). Hand-edit the Disposition and Source cells to triage; keep
the enum vocabulary exact.

| ID | Title | Severity | Disposition | Source |
|----|-------|----------|-------------|--------|
| CR-01 | One-byte stack out-of-bounds write in `verify()`/`info()` when a staged file is exactly Filecap bytes | critical | fixed | fix(19) 13091fe41 — Filecap+1 manifest buffers in verify/info; self-test Filecap-boundary + ASan legs (see 19-REVIEW-FIX.md) |
| WR-01 | Modulo bias in generated passwords (`alph[r[i] % 62]`) | warning | fixed | fix(19) 13091fe41 — rejection sampling (redraw >= 248) in genpassword; self-test charset/reachability leg (see 19-REVIEW-FIX.md) |
| WR-02 | `FdGroup::op_done` merges the streaming offset into a possibly REFILLED slot — the comments' premise contradicts the op_ref design | warning | deferred | kernel concurrency fix (fdgrp op_done slot-identity) — reopener recorded in deferred-items.md; kernel bytes would require the full pin choreography, next r9-touching pass owns it |
| IN-01 | devssl secrets and RC4 state are freed without zeroization — key material persists in kernel heap | info | deferred | donor-parity zeroization hardening — reopener in deferred-items.md |
| IN-02 | Impossible-state guard — `check_data_bss_below_ustop` cannot fire for any parseable header | info | skipped | donor-carried defense-in-depth, kept by donor-fidelity judgment |

## Legend

- Rows default to `open`; a row records what happened to a finding, never whether it mattered.
- `deferred` rows carry their reopener in the Source cell.
