---
phase: 06
review: 06-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "gethunk libc-provisioning block's error handling is dead under `set -e`"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "coverage-sweep.sh unconditionally overwrites the host's /bin/cpp outside the namespace"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "mkimage.sh keeps a dead dd trailer extraction"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "subset-exec.sh defines the D-02 GREP idiom but barely uses it"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "subset-exec.sh sentinel comment says \"40 bytes\"; the fixture is 41"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "subset-exec.sh rebake comment undercounts the tier-1 bootdir (66 vs 70)"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "gethunk fixed-bin staleness key watches only obj.c"
  - id: IN-06
    severity: info
    disposition: fixed
    title: "devroot.c carries a pre-existing debug `panic(\"whoops\")` remnant"
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "tier-donor CI leg cannot pass on a clean runner — no step clones or builds the donor"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "generator-scan.sh's audit-row selector silently skips G10+ rows"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "tracesup.c armed/kids arrays grow without a bounds check"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "mkimage.sh line 235 is dead, known-broken code left armed"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "subset-exec.sh marker asserts are unanchored substring greps (mk⊂mkdir, cp⊂cpp)"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "ledger-r9-append.sh uses bash-only process substitution under #!/bin/sh, plus undisciplined grep/sha256sum"
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "gethunk-repro-6l.sh's \"built fresh every run\" claim is narrower than the code"
open: 0
total: 15
recorded: 2026-11-06T01:33:44.395Z
---

# Phase 06: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | 06-REVIEW-FIX.md |
| WR-02 | warning | fixed | 06-REVIEW-FIX.md |
| IN-01 | info | fixed | 06-REVIEW-FIX.md |
| IN-02 | info | fixed | 06-REVIEW-FIX.md |
| IN-03 | info | fixed | 06-REVIEW-FIX.md |
| IN-04 | info | fixed | 06-REVIEW-FIX.md |
| IN-05 | info | fixed | 06-REVIEW-FIX.md |
| IN-06 | info | fixed | 06-REVIEW-FIX.md |
| CR-01 | critical | fixed | closed by retirement 2026-11-05 — tier-donor CI leg removed per maintainer direction; the x86_64 venue is Harvey's own kernel (06-09/06-10); venue-pin step asserts the retirement (not in the current review) |
| WR-03 | warning | fixed | 06-REVIEW-FIX.md (not in the current review) |
| WR-04 | warning | fixed | 06-REVIEW-FIX.md (not in the current review) |
| WR-05 | warning | fixed | resolved by IN-01 fix `aed40c38d` — the dead dd trailer line it named was deleted (not in the current review) |
| WR-06 | warning | fixed | 06-REVIEW-FIX.md (not in the current review) |
| WR-07 | warning | fixed | 06-REVIEW-FIX.md (not in the current review) |
| WR-08 | warning | fixed | resolved by IN-05 fix `d0d186ccd` — staleness key now watches every 6l source (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
