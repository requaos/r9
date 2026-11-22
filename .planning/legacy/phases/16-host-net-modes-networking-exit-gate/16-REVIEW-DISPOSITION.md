---
phase: 16
generated: 2026-11-11
review: 16-REVIEW.md
fix_report: 16-REVIEW-FIX.md
status: open
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
open: 4
fixed: 3
skipped: 0
deferred: 0
---

# Phase 16 Review Disposition

One row per finding from [16-REVIEW.md](16-REVIEW.md). Dispositions reconciled
from [16-REVIEW-FIX.md](16-REVIEW-FIX.md) (fix pass of 2026-11-11, iteration 1,
scope critical_warning — 3 fixed, 0 skipped). The 4 IN- findings stay open for
phase-17 triage per the phase 13–15 convention.

| ID | Severity | Title | Disposition | Source |
|----|----------|-------|-------------|--------|
| WR-01 | warning | Unpinned `actions/checkout@v4` in the boot-smoke job (supply-chain rule the file itself states) | fixed | 16-REVIEW-FIX.md — commit 67828a9ad (SHA-pinned to the file's own API-resolved SHA, actionlint clean) |
| WR-02 | warning | `r9-netdev-args` does not enforce its own no-spaces token contract for caller-supplied `--pcap`/`--config` values | fixed | 16-REVIEW-FIX.md — r9 commit 208ceba (push-before-pin, `token_safe()` fail-loud check + pinning test, 112 xtask tests green) + harvey lock-only pin advance 4d19a9e75 |
| WR-03 | warning | `boot-r9-net-smoke-x64` app lacks `procps` for its `pkill` survivor sweep — the sweep can silently no-op | fixed | 16-REVIEW-FIX.md — commit 21b499683 (runtimeInputs, soak-app shape, app evals) |
| IN-01 | info | `host-echo` accepts port arguments above 65535 via strtoul truncation | open | 16-REVIEW.md |
| IN-02 | info | `r9-netdev-args` silently accepts duplicate `--mode`/`--pcap`/`--config` flags (last wins) | open | 16-REVIEW.md |
| IN-03 | info | The five-TOML consistency test checks presence, not value equality — the documented "identical mode tables" invariant is not machine-checked | open | 16-REVIEW.md |
| IN-04 | info | net-doctor's default `--mode all` hard-fails outside the Harvey checkout, contradicting the header's "HARVEY optional" framing | open | 16-REVIEW.md |

**open: 4 of 7** (3 fixed, 0 skipped, 0 deferred — open rows are all Info-tier, left for phase-17 triage)

Fix-pass note: the full-workspace `cargo test` failure in r9's `riscv64` test
target, observed while verifying WR-02, was proven pre-existing (reproduced
identically on the stashed clean tree) — recorded here so the regression gate
reads it as background, not as a fix-pass regression.
