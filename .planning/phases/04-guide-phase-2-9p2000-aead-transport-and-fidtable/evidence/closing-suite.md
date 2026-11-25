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

## Owner review (D-24)

The owner reviewed the whole cumulative `port/src` change of Phase 4 in one phase-end code
review and replied "approve". The reviewed commit is
`f72cc2ad421600f2a727f25a9708e30867729461` — the evidence commit carrying
`evidence/PORT-SRC-REVIEW.md`, whose only change after the reviewed source tree (`39d1a29`) is
planning evidence. No source file differs between the reviewed sha and the HEAD Task 3 measures
at; Task 3's precondition re-checks this and stops if it does not hold.

VERDICT port-src-review: APPROVED HEAD=f72cc2ad421600f2a727f25a9708e30867729461

## Closing runs (Task 3, at the reviewed commit)

All runs were made at HEAD `3398df49499d1b877fcf0ac92fa54c0872df9c71` — the Task 2 approval
commit — with no source change since the reviewed sha `f72cc2a` (Task 3 precondition checked
`git diff --quiet f72cc2a HEAD -- port crypto ds xtask aarch64 x86_64 Cargo.lock`, which was
clean). Logs are under `target/phase04-closing/closing/NAME.log` (gitignored); each log's first
line is its `HEAD=` and its last is `EXIT=`.

| Run | Command | Log | Exit | Seconds |
|-----|---------|-----|------|---------|
| 1 | `nix develop -c cargo xtask test` | `test-run-1.log` | 0 | 4 |
| 1 | `nix develop -c cargo test -p xtask` | `xtask-tests-run-1.log` | 0 | 4 |
| 1 | `nix develop -c cargo xtask check` | `check-run-1.log` | 0 | 3 |
| 1 | `nix develop -c cargo xtask clippy` | `clippy-run-1.log` | 0 | 2 |
| 1 | `nix develop -c cargo xtask test-9p` | `test-9p-run-1.log` | 0 | 3 |
| 2 | `nix develop -c cargo xtask test` | `test-run-2.log` | 0 | 5 |
| 2 | `nix develop -c cargo test -p xtask` | `xtask-tests-run-2.log` | 0 | 4 |
| 2 | `nix develop -c cargo xtask check` | `check-run-2.log` | 0 | 3 |
| 2 | `nix develop -c cargo xtask clippy` | `clippy-run-2.log` | 0 | 2 |
| 2 | `nix develop -c cargo xtask test-9p` | `test-9p-run-2.log` | 0 | 3 |
| — | `nix develop -c cargo xtask miri` | `miri.log` | 0 | 348 |

The four suite commands (`test`, `test -p xtask`, `check`, `clippy`) passed twice; `test-9p`
passed twice; `miri` passed once. `miri` took 348 seconds — over the 120-second band, which is
the accepted Phase 3 deviation: it is reported here, not cut for time.

VERDICT suite-run-1: PASS exit=0
VERDICT suite-run-2: PASS exit=0
VERDICT test-9p-run-1: PASS exit=0
VERDICT test-9p-run-2: PASS exit=0
VERDICT miri: PASS exit=0
MIRI-TIME closing 348

## Aarch64 quick pass (Task 3, at the reviewed commit)

Run on the Mac (`Darwin arm64`) at HEAD `3398df4`; logs under `target/phase04-closing/quick/`
(gitignored), each beginning `HEAD=` and ending `EXIT=`. Each passes on exit 0 with its decisive
line containing `ALL STAGES PASS`.

| Verdict id | Command | Log | Exit | Seconds | Decisive line |
|------------|---------|-----|------|---------|---------------|
| `quick-tier1-hvf` | `nix run .#boot-r9-tier1` | `quick-tier1-hvf.log` | 0 | 47 | `tier1-r9: ALL STAGES PASS` — boot, typed sweep, float canary, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era |
| `quick-tier1-tcg` | `R9_ACCEL=tcg nix run .#boot-r9-tier1` | `quick-tier1-tcg.log` | 0 | 13 | same tier-1 stage set, under TCG |
| `quick-smoke` | `nix run .#boot-r9-smoke` | `quick-smoke.log` | 0 | 11 | `smoke-r9: ALL STAGES PASS` — boot, echo, ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind, hooks, no-unknown-device |
| `quick-net-smoke` | `nix run .#boot-r9-net-smoke` | `quick-net-smoke.log` | 0 | 17 | `echo NET-SMOKE: ALL STAGES PASS` |
| `quick-cpu` | `nix run .#boot-r9-cpu` | `quick-cpu.log` | 0 | 46 | `cpu-r9: ALL STAGES PASS` — banner, bootconf, paqfs, cpurc-ran, shebang, stat, cap, remove, ssl, cpurc-done, init-rc, prompt, no-panic, marker-once, hooks, no-unknown-device |

VERDICT quick-tier1-hvf: PASS exit=0
VERDICT quick-tier1-tcg: PASS exit=0
VERDICT quick-smoke: PASS exit=0
VERDICT quick-net-smoke: PASS exit=0
VERDICT quick-cpu: PASS exit=0
