---
phase: 01
review: 01-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Unvalidated image name escapes the staging root (path traversal / directory deletion)"
  - id: WR-01
    severity: warning
    disposition: open
    title: "`ureg-check` exits 0 when one architecture's tree is unresolvable"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`cargo xtask gate --full` silently ignores `--accel`"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Quick gate's tree-unmodified proof is skipped on every early error path"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Gate password file mode is only applied at creation"
  - id: WR-05
    severity: warning
    disposition: open
    title: "Per-bake authdomain (with the DEMO password) is left under `target/` on error paths"
  - id: WR-06
    severity: warning
    disposition: open
    title: "Drawterm driver identity sweep can match and kill its own helper"
  - id: WR-07
    severity: warning
    disposition: open
    title: "`eval` of the netaddr seam in the doctor and both tap gates"
  - id: WR-08
    severity: warning
    disposition: open
    title: "`host-echo` accepts out-of-range ports (unsigned-short wrap)"
  - id: WR-09
    severity: warning
    disposition: open
    title: "Gate scripts and the image engine can resolve different Harvey trees"
  - id: WR-10
    severity: warning
    disposition: open
    title: "Witness reuses the Harvey scratch tree and equivalence baselines without pin provenance"
  - id: WR-11
    severity: warning
    disposition: open
    title: "Full tier includes an interactive qemu boot with no timeout or driver"
  - id: WR-12
    severity: warning
    disposition: open
    title: "Credential scans delete leaks but never fail the witness"
  - id: IN-01
    severity: info
    disposition: open
    title: "`tree_digest` is a non-cryptographic hash and omits modes and symlink targets"
  - id: IN-02
    severity: info
    disposition: open
    title: "`[[tree]]` members-file modes are not bounded like manifest modes"
  - id: IN-03
    severity: info
    disposition: open
    title: "Generated `ndb-template` inputs bypass the symlink containment checks"
  - id: IN-04
    severity: info
    disposition: open
    title: "`Dist.module` documentation still claims only the `harvey:` scheme"
  - id: IN-05
    severity: info
    disposition: open
    title: "`run_full` checks one flake system but runs another"
  - id: IN-06
    severity: info
    disposition: open
    title: "Unused `control_leg` variable in the drawterm driver"
  - id: IN-07
    severity: info
    disposition: open
    title: "`drawterm_bin()` does not validate `DRAWTERM_BIN` as an executable file"
  - id: IN-08
    severity: info
    disposition: open
    title: "Several `kvm` selections only check existence of `/dev/kvm`"
  - id: IN-09
    severity: info
    disposition: open
    title: "S6 spike teardown patterns never match, and one final check is host-wide"
  - id: IN-10
    severity: info
    disposition: open
    title: "`boot-q35.sh` only scans for failure signatures after the banner"
  - id: IN-11
    severity: info
    disposition: open
    title: "`--serial-check` markers are injected as Tcl regular expressions"
  - id: IN-12
    severity: info
    disposition: open
    title: "`run_full` silently ignores `--force-no-rio`/`--serial-check`, and capture freshness is not checked"
  - id: IN-13
    severity: info
    disposition: open
    title: "`aout.py` computes an unused `declared` length"
open: 25
total: 26
recorded: 2026-11-23T00:00:00.000Z
---

# Phase 01: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | fixed in b36b633 with the no-deletion regression test: `validate_image_name` guards the manifest, `--image`, `image:`, and manifest/request equality; 4 tests; `cargo xtask check` + 96 lib tests green |
| WR-01 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-02 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-03 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-04 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-05 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-06 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-07 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-08 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-09 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-10 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-11 | warning | open | owner disposition 2026-11-23: record open, no code change |
| WR-12 | warning | open | owner disposition 2026-11-23: record open, no code change |
| IN-01 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-02 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-03 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-04 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-05 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-06 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-07 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-08 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-09 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-10 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-11 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-12 | info | open | owner disposition 2026-11-23: record open, no code change |
| IN-13 | info | open | owner disposition 2026-11-23: record open, no code change |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.

