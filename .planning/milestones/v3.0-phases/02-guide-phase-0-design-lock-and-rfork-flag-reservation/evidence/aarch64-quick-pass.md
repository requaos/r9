# aarch64 quick pass and Mac suite after the rfork flag move

The standing owner rule for an aarch64 kernel byte change: tier-1 on HVF and on TCG, and
smoke, net-smoke and cpu once on HVF. All five legs ran on this Mac on 2026-11-24, from the
repository root, at:

- LEGS_SHA: `785ee198c94eec60f715f28fe42cfc28abc96407` (`git rev-parse HEAD` when the legs ran)
- AFTER_SHA (the kernel-bytes after commit in `evidence/kernel-bytes/after.sha256`):
  `09e6901f24e5f1479f8657fc9b5809632d874a16`

The two shas differ because the kernel-bytes evidence commit lies between them; the code
under `port`, `x86_64`, `aarch64`, `xtask` and `sys` does not:
`git diff --quiet 09e6901 785ee19 -- port x86_64 aarch64 xtask sys` exited 0.

Each leg's full output is in `target/phase02-quick-pass/<leg>.log` (scratch, not committed).
A leg passes when it exits 0 and its log contains `ALL STAGES PASS`. No leg was rerun: every
leg passed on its first run.

VERDICT aarch64-tier1-hvf: PASS
- exit code: 0
- `tier1-r9: ALL STAGES PASS — boot, one typed sweep line, float canary on r9, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era`

VERDICT aarch64-tier1-tcg: PASS
- exit code: 0
- `tier1-r9: ALL STAGES PASS — boot, one typed sweep line, float canary on r9, marker parity + exact SKIP set, T1-DONE, user-hooks-registered, no-unknown-device, no-unsupported-era`

VERDICT aarch64-smoke-hvf: PASS
- exit code: 0
- `smoke-r9: ALL STAGES PASS — boot, echo (typed + answered), ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era`

VERDICT aarch64-net-smoke-hvf: PASS
- exit code: 0
- `smoke-r9-net: ALL STAGES PASS — boot, ping 127.0.0.1, udp echo, tcp/7 echo, ipconfig static, ipconfig dhcp (both NOT-L2-TRUTH labeled), completion marker, pid-warning-absent, nic-bind, user-hooks-registered, no-unknown-device, no-unsupported-era`

VERDICT aarch64-cpu-hvf: PASS
- exit code: 0
- `cpu-r9: ALL STAGES PASS — banner, bootconf, paqfs, cpurc-ran, shebang, stat, cap, remove, ssl, cpurc-done, init-rc, prompt, no-cpurc-fail, no-panic, bootconf-present, marker-once, user-hooks-registered, no-unknown-device`

## Full Mac suite

Run once, at the same LEGS_SHA, in the default-arch form hosted CI runs
(`.github/workflows/xtask.yml`). The `--arch aarch64` clippy form fails at HEAD and is not a
pass criterion for this phase, so it was not run. Output is in
`target/phase02-quick-pass/mac-suite/<command>.log`.

| Command | Exit code |
|---------|-----------|
| `nix develop -c cargo xtask test` | 0 |
| `nix develop -c cargo test -p xtask` | 0 |
| `nix develop -c cargo xtask check` | 0 |
| `nix develop -c cargo xtask clippy` | 0 |

`cargo xtask test` printed `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok`
twice (the aarch64 crate's default and `virtshell` cfg legs), so the reserved-bits pin ran on
both configs.

VERDICT mac-full-suite: PASS
- exit codes: 0, 0, 0, 0
