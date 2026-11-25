# Phase 4 gap-closure closing evidence (plan 04-23)

The evidence that must exist before the owner is shown the gap-closure `port/src` change.
This file starts with the five pre-review legs Task 1 runs; Task 2 appends the owner's
verdict. Every verdict below comes from the run recorded beside it, and no verdict is carried
over from an earlier HEAD.

## Amendment: the WR-09 insert lock (owner request)

The owner read the gap-closure diff and required one change before approving: the racing inserts
the bundle listed as an open question had to be resolved first. `FidTable::insert` now holds a
table-level `insert_lock` across the all-shard duplicate probe **and** the placement walk, so two
concurrent inserts of one `(client_id, fid_key)` cannot both pass the probe; `resolve` is untouched
and still lock-free with one arena read. That changed `port/src/fid.rs` and
`port/tests/aead_fid.rs`, so every verdict in this section was re-run at the new HEAD. The
`a669317` block below is history: it is kept as recorded and marked superseded.

HEAD 6fd201c072d9c42f4de6e6d617c0ac35cf20d013
(`fix(04-23): serialise FidTable inserts so a racing pair cannot both land (WR-09)`)

Venue: `Darwin arm64` (the same venue line as `evidence/closing-suite.md`).
Logs: `target/phase04-gaps/pre-review-amendment/NAME.log` (gitignored); each leg's log is the
whole command output.

Run in the order the amendment names, at HEAD `6fd201c`, so that the review is not offered on a
red tree.

| # | Command | Log | Exit | Result |
|---|---------|-----|------|--------|
| 0 | `nix develop -c cargo test -p port --test aead_fid` | `aead-fid.log` | 0 | 36 tests, one `test result: ok` line, 0 failed — the new racing-insert pin lives in this binary and passes |
| 1 | `nix develop -c cargo xtask test` | `xtask-test.log` | 0 | 37 `test result: ok` lines, 0 `test result: FAILED` |
| 2 | `nix develop -c cargo test -p xtask` | `xtask-tests.log` | 0 | 5 `test result: ok` lines (unit, integration, doctests), 0 failures |
| 3 | `nix develop -c cargo xtask check` | `check.log` | 0 | clean |
| 4 | `nix develop -c cargo xtask clippy` | `clippy.log` | 0 | clean (the workspace's pre-existing `warn`-level lints stay warnings; nothing is reported against `port/src/fid.rs` or `port/tests/aead_fid.rs`) |
| 5 | `nix develop -c cargo xtask test-9p` | `test-9p.log` | 0 | 14 legs (seven targets, each with its scalar-backend twin), 14 `test result: ok` lines, 0 `test-9p:` lines |

VERDICT pre-review-test: PASS exit=0
VERDICT pre-review-xtask-tests: PASS exit=0
VERDICT pre-review-check: PASS exit=0
VERDICT pre-review-clippy: PASS exit=0
VERDICT pre-review-test-9p: PASS exit=0

The new pin is a real pin, not a shape that passes either way. With the `insert_lock` acquisition
removed from `insert` (locally, not committed) `aead_fid_insert_racing_pair_lands_exactly_one_entry`
fails with two winners in two shards — rounds 1, 3, 5, 9, 13 and 20 observed across runs — and with
the lock in place the binary is green over repeated runs (`36 passed; 0 failed`). The window the
test drives is the fallover one: the key's home block is full, one racer walks it and falls over to
the next shard, a home slot is freed mid-race, and a racer that probed earlier lands at home.

## HEAD under review — a669317 (SUPERSEDED)

**Superseded** by the amendment section above: the insert lock changed `port/src/fid.rs` after this
run, so these verdicts are history and are not the tree the owner is reviewing. Kept as recorded.

HEAD a6693171aeac7675eea01af434f0adb75e57b4e5

That HEAD is the findings-record commit of this plan (`docs(04-23): record the twelve gap
findings as fixed`), a planning-document change only. Task 1's base check ran before it: the
first gap-closure commit that touches a source file is `38acbb5`, and `git diff --quiet` over
`port crypto ds xtask aarch64 x86_64 riscv64 Cargo.lock flake.nix` between that commit's parent
`3041ded` and `GAPBASE` `5d1549af07de7220d6f48aac0d0db221aa7c541c` is clean, so nothing other
than the gap closure changed source since the owner-approved and gated state.

Venue: `Darwin arm64` (the same venue line as `evidence/closing-suite.md`).
Logs: `target/phase04-gaps/pre-review/NAME.log` (gitignored); each leg's log is the whole
command output, and `NAME.exit` holds its exit code.

## Pre-review legs — at a669317 (SUPERSEDED)

Superseded by the amendment section above; kept as recorded.

Run in the order the plan names, at HEAD `a669317`, so that the review is not offered on a red
tree.

| # | Command | Log | Exit | Result |
|---|---------|-----|------|--------|
| 1 | `nix develop -c cargo xtask test` | `test.log` | 0 | 37 `test result: ok` lines, 0 `test result: FAILED` |
| 2 | `nix develop -c cargo test -p xtask` | `xtask-tests.log` | 0 | 5 `test result: ok` lines (unit, integration, doctests), 0 failures |
| 3 | `nix develop -c cargo xtask check` | `check.log` | 0 | clean |
| 4 | `nix develop -c cargo xtask clippy` | `clippy.log` | 0 | clean (the workspace's pre-existing `warn`-level lints stay warnings) |
| 5 | `nix develop -c cargo xtask test-9p` | `test-9p.log` | 0 | 14 legs (seven test files, each with its scalar-backend twin), 14 `test result: ok` lines, 0 `test-9p:` lines |

SUPERSEDED (a669317): VERDICT pre-review-test: PASS exit=0
SUPERSEDED (a669317): VERDICT pre-review-xtask-tests: PASS exit=0
SUPERSEDED (a669317): VERDICT pre-review-check: PASS exit=0
SUPERSEDED (a669317): VERDICT pre-review-clippy: PASS exit=0
SUPERSEDED (a669317): VERDICT pre-review-test-9p: PASS exit=0

The `test-9p` leg count of 14 is plan 04-20's per-file shape (seven legs, each with its
scalar-backend twin); the `test result: ok` count stays 14 because each leg runs one test binary.

## Owner review of the gap-closure `port/src` change (Task 2)

The owner answered: approve with the lock. The insert-lock amendment (`6fd201c`) was required
first and is the range end; every pre-review leg above was re-run at it. The verdict line, as the
plan requires:

VERDICT port-src-review-gaps: APPROVED HEAD=6fd201c072d9c42f4de6e6d617c0ac35cf20d013

## Closing suites at the reviewed commit (plan 04-24, Task 1)

HEAD 8df21a37dc15f637337057453aa470a7d407aede
(`docs(04-23): mark the plan summary self-check passed`)

No source file differs between that commit and the reviewed sha
`6fd201c072d9c42f4de6e6d617c0ac35cf20d013`: `git diff --quiet 6fd201c HEAD -- port crypto ds
xtask aarch64 x86_64 riscv64 Cargo.lock flake.nix` exits 0 (and the same check including
`rust-toolchain.toml` and `.cargo` exits 0), so the tree measured below is the tree the owner
approved in plan 04-23.

Venue: `Darwin arm64`.
Logs: `target/phase04-closing2/NAME.log` (gitignored); each log begins `HEAD=` and `CMD=` and ends
`EXIT=`, and is the whole command output.

Both suite runs and both `test-9p` runs were made at this one commit, in the order recorded.

| Run | Command | Log | Exit | Result |
|-----|---------|-----|------|--------|
| `suite-run-1` | `nix develop -c cargo xtask test` | `suite-run-1-test.log` | 0 | 37 `test result: ok` lines, 0 failed |
| `suite-run-1` | `nix develop -c cargo test -p xtask` | `suite-run-1-xtask-tests.log` | 0 | 5 `test result: ok` lines (unit, integration, doctests), 0 failed |
| `suite-run-1` | `nix develop -c cargo xtask check` | `suite-run-1-check.log` | 0 | clean |
| `suite-run-1` | `nix develop -c cargo xtask clippy` | `suite-run-1-clippy.log` | 0 | clean (the workspace's pre-existing `warn`-level lints stay warnings) |
| `test-9p-run-1` | `nix develop -c cargo xtask test-9p` | `test-9p-run-1.log` | 0 | 14 `test result: ok` lines (seven targets, each with its scalar-backend twin), 0 `test-9p:` lines |
| `suite-run-2` | `nix develop -c cargo xtask test` | `suite-run-2-test.log` | 0 | 37 `test result: ok` lines, 0 failed |
| `suite-run-2` | `nix develop -c cargo test -p xtask` | `suite-run-2-xtask-tests.log` | 0 | 5 `test result: ok` lines, 0 failed |
| `suite-run-2` | `nix develop -c cargo xtask check` | `suite-run-2-check.log` | 0 | clean |
| `suite-run-2` | `nix develop -c cargo xtask clippy` | `suite-run-2-clippy.log` | 0 | clean |
| `test-9p-run-2` | `nix develop -c cargo xtask test-9p` | `test-9p-run-2.log` | 0 | 14 `test result: ok` lines, 0 `test-9p:` lines |
| `miri` | `nix develop -c cargo xtask miri` | `miri.log` | 0 | tpht suite green (33 passed, 0 failed, 1 ignored by the D-15 generator), 64 seeds, no Undefined Behavior and no data race |

Miri is over the 120-second feedback-latency budget: `MIRI-TIME closing 357`. This is the
accepted Phase 3 deviation, reported and not fixed; `ds/` is unchanged by the gap closure, and
this is the same closing check plan 04-14 made.

VERDICT suite-run-1: PASS exit=0
VERDICT suite-run-2: PASS exit=0
VERDICT test-9p-run-1: PASS exit=0
VERDICT test-9p-run-2: PASS exit=0
VERDICT miri: PASS exit=0
MIRI-TIME closing 357

## aarch64 quick pass (plan 04-24, Task 2)

HEAD caff8d4d7190cbede8a21600e7fc4cd37b04adc7
(`evidence(04-24): closing suite and kernel byte evidence for the gap closure` — a `.planning`-only
commit. No source file differs from the reviewed sha `6fd201c072d9c42f4de6e6d617c0ac35cf20d013`,
and the kernels booted here are the ones recorded in `after-04-24.sha256` at `8df21a3`.)

Venue: `Darwin arm64`.
Logs: `target/phase04-closing2/quick/NAME.log` (gitignored); each log begins `HEAD=` and `CMD=` and
ends `EXIT=`.

D-25: the quick pass is the aarch64 evidence for this phase; the full HVF series is not run
(deferred to Phase 8). Each leg passes on exit 0 with an `ALL STAGES PASS` line.

| Leg | Command | Exit | Decisive line |
|-----|---------|------|---------------|
| `quick-tier1-hvf` | `nix run .#boot-r9-tier1` | 0 | `tier1-r9: ALL STAGES PASS — boot, one typed sweep line, float canary on r9, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era` |
| `quick-tier1-tcg` | `R9_ACCEL=tcg nix run .#boot-r9-tier1` | 0 | `tier1-r9: ALL STAGES PASS — boot, one typed sweep line, float canary on r9, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era` |
| `quick-smoke` | `nix run .#boot-r9-smoke` | 0 | `smoke-r9: ALL STAGES PASS — boot, echo (typed + answered), ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era` |
| `quick-net-smoke` | `nix run .#boot-r9-net-smoke` | 0 | `smoke-r9-net: ALL STAGES PASS — boot, ping 127.0.0.1, udp echo, tcp/7 echo, ipconfig static, ipconfig dhcp (both NOT-L2-TRUTH labeled), completion marker, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era` |
| `quick-cpu` | `nix run .#boot-r9-cpu` | 0 | `cpu-r9: ALL STAGES PASS — banner, bootconf, paqfs, cpurc-ran, shebang, stat, cap, remove, ssl, cpurc-done, init-rc, prompt, no-cpurc-fail, no-panic, bootconf-present, marker-once, user-hooks-registered, no-unknown-device` |

VERDICT quick-tier1-hvf: PASS exit=0 — `tier1-r9: ALL STAGES PASS — boot, one typed sweep line, float canary on r9, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era`
VERDICT quick-tier1-tcg: PASS exit=0 — `tier1-r9: ALL STAGES PASS — boot, one typed sweep line, float canary on r9, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era`
VERDICT quick-smoke: PASS exit=0 — `smoke-r9: ALL STAGES PASS — boot, echo (typed + answered), ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era`
VERDICT quick-net-smoke: PASS exit=0 — `smoke-r9-net: ALL STAGES PASS — boot, ping 127.0.0.1, udp echo, tcp/7 echo, ipconfig static, ipconfig dhcp (both NOT-L2-TRUTH labeled), completion marker, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era`
VERDICT quick-cpu: PASS exit=0 — `cpu-r9: ALL STAGES PASS — banner, bootconf, paqfs, cpurc-ran, shebang, stat, cap, remove, ssl, cpurc-done, init-rc, prompt, no-cpurc-fail, no-panic, bootconf-present, marker-once, user-hooks-registered, no-unknown-device`
