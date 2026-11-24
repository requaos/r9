# Phase 3 end-of-phase owner stop — results

Plan 03-11, Task 2. The owner pushed the transport branch, ran the x86_64 Linux checklist of
`03-OWNER-STOP.md` and returned the logs through the branch under
`evidence/owner-stop/return/`. Every verdict below is decided only from a line in those logs;
no step was run on the Mac. All eleven verdicts are PASS and none is FAIL, so the stop rules
do not apply and Task 3's `main`-push decision follows.

- branch: `phase-03/crypto-ds-foundations`
- return-commits: `9af4f1efed7b075df0f2d51617bca530f25410b4` and
  `5f565ec047b5ef8168e450e85f80275f1823d8e6` (`evidence(03-11): owner-stop return logs`); the
  runner was executed twice, the later tree is authoritative, and both commits add files only
  under `evidence/owner-stop/return/` (`git diff --name-only 95fe66a origin/phase-03/crypto-ds-foundations`
  lists 12 such files and the same diff with `':(exclude)…/owner-stop/return'` is empty)
- returned logs: `SUMMARY.txt`, `tip-kernel-x64.log`, `tip-kernel-virt.log`,
  `base-kernel-x64.log`, `base-kernel-virt.log`, `kernel-hashes.log`, `boot-iso-first.log`,
  `boot-iso-rebuild.log`, `host-test.log`, `host-check.log`, `host-clippy.log`,
  `host-miri.log` — every log begins with its `HEAD=` line and ends with its `EXIT=` line
- `origin/main` is `3d01f071efda9ed0720f1a07c4a6db0c7af8f4c5`, unchanged by this plan's tasks
  to this point
- result: **11 of 11 verdicts PASS**

pushed-tip 95fe66aaadde14cb8b8fa8197f6b0547ea7b7898

base 4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1

The four Linux kernel sha256 values (`kernel-hashes.log`; these are Linux derivation hashes,
not comparable with the Mac devshell hashes in `evidence/kernel-bytes/`):

- tip `kernel-x64`: `6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5`
- base `kernel-x64`: `6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5`
- tip `kernel-virt`: `954e51102f2e3eeccb028b984fd32730375107d32e21c666aeeadff2030f4eee`
- base `kernel-virt`: `954e51102f2e3eeccb028b984fd32730375107d32e21c666aeeadff2030f4eee`

The pairs are equal for both architectures: no kernel byte changed on Linux, so the `boot-iso`
`outputHash` procedure does not apply (D-21) and the boot-to-rio gate keeps holding by identity
with the Phase 2 gated state.

## Verdicts

VERDICT stop-branch-push: PASS
EVIDENCE stop-branch-push: `git ls-remote origin refs/heads/phase-03/crypto-ds-foundations` ->
`5f565ec047b5ef8168e450e85f80275f1823d8e6`; the branch base is the last Mac commit
`95fe66aaadde14cb8b8fa8197f6b0547ea7b7898` (`SUMMARY.txt`: `PUSHED-TIP`
`95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`), and the two commits above it are the return
commits, which add files only under `evidence/owner-stop/return/`.

VERDICT stop-kernel-x64-build: PASS
EVIDENCE stop-kernel-x64-build: `tip-kernel-x64.log`:
`HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898` then `EXIT=0` — the sandboxed `kernel-x64`
derivation builds at the pushed tip with the enlarged `Cargo.lock` (the A8 risk retired).

VERDICT stop-kernel-virt-build: PASS
EVIDENCE stop-kernel-virt-build: `tip-kernel-virt.log`:
`HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898` then `EXIT=0` — the sandboxed `kernel-virt`
derivation builds at the pushed tip with the enlarged `Cargo.lock`.

VERDICT stop-kernel-x64-bytes: PASS
EVIDENCE stop-kernel-x64-bytes: `kernel-hashes.log`
`HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`:
`6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5` for
`…/tip-kernel-x64/kernel-x64.elf` and the same
`6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5` for
`…/base-kernel-x64/kernel-x64.elf`; log ends `EXIT=0`.

VERDICT stop-kernel-virt-bytes: PASS
EVIDENCE stop-kernel-virt-bytes: `kernel-hashes.log`
`HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`:
`954e51102f2e3eeccb028b984fd32730375107d32e21c666aeeadff2030f4eee` for
`…/tip-kernel-virt/kernel-virt.elf` and the same
`954e51102f2e3eeccb028b984fd32730375107d32e21c666aeeadff2030f4eee` for
`…/base-kernel-virt/kernel-virt.elf`; log ends `EXIT=0`.

VERDICT stop-bootiso-build: PASS
EVIDENCE stop-bootiso-build: `boot-iso-first.log`:
`HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898` then `EXIT=0`, with no `hash mismatch` and no
`got:` line: the fixed-output ISO builds with the committed `outputHash`
(`sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`).

VERDICT stop-bootiso-rebuild: PASS
EVIDENCE stop-bootiso-rebuild: `boot-iso-rebuild.log`:
`HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`, then
`checking outputs of '/nix/store/8qzi9466f9vm5jpxw4lzzyvw7f8fmp9c-r9-boot-iso-95fe66a.drv'...`,
then `EXIT=0`, with no `hash mismatch` and no `got:` line.

VERDICT stop-host-test: PASS
EVIDENCE stop-host-test: `host-test.log` `HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`:
29 `test result: ok` lines, every one `0 failed` (final line `test result: ok. 140 passed;
0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`), no `FAILED` line, and the
default and soft-backend `crypto` legs both ran (`Running unittests src/lib.rs
(target/debug/deps/crypto-4161b3aadeb5df48)` and `…(crypto-5085cebe8a21af39)`, with the RFC
4231/8439 pins `ok` on each); log ends `EXIT=0`.

VERDICT stop-host-check: PASS
EVIDENCE stop-host-check: `host-check.log` `HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`:
`Finished \`dev\` profile [optimized + debuginfo] target(s) in 1.18s`, no `error` line; log ends
`EXIT=0`.

VERDICT stop-host-clippy: PASS
EVIDENCE stop-host-clippy: `host-clippy.log` `HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`:
`Finished \`dev\` profile [optimized + debuginfo] target(s) in 3.10s` (warnings only, no
`error` line and no `unsafe block missing a safety comment`); log ends `EXIT=0`.

VERDICT stop-host-miri: PASS
EVIDENCE stop-host-miri: `host-miri.log` `HEAD=95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`:
64 seed lines `Trying seed: 0` through `Trying seed: 63`, each with
`test result: ok. 29 passed; 0 failed; 1 ignored; 0 measured; 13 filtered out`, and no
`Undefined Behavior`, `Data race` or `FAILED` line; log ends `EXIT=0`.

## Notes

- Every `HEAD=` line of the tip-side step logs is `95fe66aaadde14cb8b8fa8197f6b0547ea7b7898`;
  the two base-build logs of section 4 carry
  `HEAD=4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1` as the checklist requires. `grep -c '^HEAD='`
  and `grep -c '^EXIT='` are 1 on each log.
- `SUMMARY.txt` opens with `PUSHED-TIP 95fe66aaadde14cb8b8fa8197f6b0547ea7b7898` and
  `SCRIPT-EXIT 0`, then lists the same twelve logs' `HEAD=`/`EXIT=` pairs; it agrees with every
  quoted log.
- No `hash mismatch`, no `got:` value and no `cargo`/`nix` error line appears in any returned
  log. The clippy log's `xtask` warnings are pre-existing and do not fail the command.
- No credential string appears in the returned logs; nothing needed redaction.
