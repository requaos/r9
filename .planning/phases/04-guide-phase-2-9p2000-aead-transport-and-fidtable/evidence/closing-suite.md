# Phase 4 closing evidence (plan 04-14)

The evidence the Mac can produce before the owner stop.  This file starts with the pre-review
legs Task 1 runs, and Task 3 appends the closing runs at the reviewed commit.  Every verdict
below comes from the run recorded beside it; nothing is written for a command that did not run,
and no verdict is carried over from an earlier HEAD.

## HEAD under review

HEAD 39d1a29806d98fbb8aa3df2c4f240eb368109174

That is the commit this plan's own change produced — the red-tree repair to
`xtask/tests/threat_model.rs` (commit `39d1a29`, below) on top of the STATE.md commit
`b84adca`.  The tree was red at the HEAD this plan was dispatched on: the pin at
`xtask/tests/threat_model.rs:234` still required every section-5 row of `docs/threat-model.md`
to say `not implemented`, while plan 04-13 commit `038ef15` had moved the six Phase 4 rows to
the D-23 status under the owner's authorization.  The owner chose to fix in place and continue;
the repair is test-only (`xtask` is host-only and is linked into neither kernel), so the kernel
byte baseline recorded at `evidence/kernel-bytes/before.sha256` is unaffected.

Venue: `Darwin arm64` (the same venue line as the baseline).
Logs: `target/phase04-closing/pre-review/NAME.log` (gitignored); each leg's log is the whole
command output.

## Pre-review legs

Run in the order the plan names, at HEAD `39d1a29`, all after the repair commit so that the
review is not offered on a red tree.

| # | Command | Log | Exit | Result |
|---|---------|-----|------|--------|
| 1 | `nix develop -c cargo xtask test` | `test.log` | 0 | 37 test binaries/doctest groups, all `test result: ok` |
| 2 | `nix develop -c cargo test -p xtask` | `xtask-tests.log` | 0 | unit, integration and doctests, all `test result: ok` |
| 3 | `nix develop -c cargo xtask check` | `check.log` | 0 | clean |
| 4 | `nix develop -c cargo xtask clippy` | `clippy.log` | 0 | clean (the workspace's pre-existing `warn`-level lints stay warnings) |
| 5 | `nix develop -c cargo xtask test-9p` | `test-9p.log` | 0 | crypto session, `aead_fid` 24, `aead_negotiate` 10, `aead_tlv` 11, `aead_transport` 35, `aead_wire` 19 — all pass |

VERDICT pre-review-test: PASS exit=0
VERDICT pre-review-xtask-tests: PASS exit=0
VERDICT pre-review-check: PASS exit=0
VERDICT pre-review-clippy: PASS exit=0
VERDICT pre-review-test-9p: PASS exit=0

The `xtask test` and `test-9p` legs exercise the repaired pin: `cargo test -p xtask` runs
`threat_model_protections_are_not_overclaimed` (was
`threat_model_protections_are_not_implemented`), which now accepts the D-23 status and rejects
a Phase 4 row that claims the code is used by a booted kernel.

## Closing runs

Task 3 appends `VERDICT suite-run-1`, `suite-run-2`, `test-9p-run-1`, `test-9p-run-2`, `miri`,
the kernel-byte verdicts' companions and the aarch64 quick-pass verdicts at the reviewed
commit, after the owner's `VERDICT port-src-review` line.  Nothing in this section may be
written before the review returns `APPROVED`.
