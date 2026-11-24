# Phase 2 end-of-phase owner stop — results (round 2)

Plan 02-06, Task 2, second round. Round 1 was voided because no returned log carried a
`HEAD=` line; this file records round 2 only. The owner returned the round-2 logs on the
branch as commit `f8d077d`, all under `evidence/owner-stop/return/round2/`. Every verdict
below is decided only from a line in those logs; no step was run on the Mac.

- branch: `phase-02/design-lock`
- branch-tip: `f8d077dd0912daec976a862c9d8689783d6fe81d` (`evidence(02-06): owner stop round 2 logs at the gated commit`)
- hash-commit: `4624ea2f0cb45c81870dcc881d544d7d36a2e0bf` — `boot-iso hash bump`, `flake.nix` only, one line changed
- return-commits: `a0454ffed4f80473866239c4164a1918bf0d216c` (round-1 files) and
  `f8d077dd0912daec976a862c9d8689783d6fe81d` (round-2 files); both add files only under
  `evidence/owner-stop/return/`
- host (return/round2/venue.log): `venue: Linux x86_64`, `gated: 4624ea2…`, Nix 2.35.6
- returned logs: `host-tests.log`, `boot-iso.log`, `kernel-step.log`, `gate-kvm.log`,
  `gate-tcg.log`, `gate-full.log`, each `HEAD=4624ea2…` first and `EXIT=0` last, plus
  `venue.log` and the `full/` artifacts
- result: **9 of 9 verdicts PASS**

gated-commit 4624ea2f0cb45c81870dcc881d544d7d36a2e0bf

hash-commit 4624ea2f0cb45c81870dcc881d544d7d36a2e0bf (not "no change")

kernel-x64-elf-sha256 (Linux): 6a5f5a66b6f1e35bdb8d36ad4346ce6fc7f106d28fa43c25ed74c4a3301cf6a5
(this is the Linux build that pairs with the committed `outputHash`; it is not comparable with
the Mac hashes in `evidence/kernel-bytes/`)

## Verdicts

VERDICT stop-branch-push: PASS
EVIDENCE stop-branch-push: `git ls-remote origin refs/heads/phase-02/design-lock` ->
`f8d077dd0912daec976a862c9d8689783d6fe81d`; the branch base is the last Mac commit
`2b415882aca30e3965ae3fb9612e91d05081879d`, and the three commits above it are the hash commit
plus the two return commits.

VERDICT stop-bootiso-hash-commit: PASS
EVIDENCE stop-bootiso-hash-commit: own commit `4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`,
`git show --stat` lists `flake.nix | 2 +-` (one line):
`- outputHash = "sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=";` ->
`+ outputHash = "sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=";`, and the build at the gated
commit succeeded with that committed value — `boot-iso.log`: `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`
then `EXIT=0`, with `KERNEL-BOOTISO-BUILD-OK` and `KERNEL-BOOTISO-REBUILD-OK` under the same
`HEAD=` in `kernel-step.log` (no hash mismatch and no `got:` line: the committed value matches).

VERDICT stop-kernel-bootiso-build: PASS
EVIDENCE stop-kernel-bootiso-build: `kernel-step.log` `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`:
`KERNEL-STORE /nix/store/91m374sd8fgc8qhbgsh1jjdqp50xx0ch-r9-kernel-x64-4624ea2`,
`KERNEL-BOOTISO-STORE /nix/store/sacvdr3h3gapqkbpi0j8kw93k7yfiycm-r9-boot-iso-4624ea2`,
`KERNEL-BOOTISO-BUILD-OK`; log ends `EXIT=0`.

VERDICT stop-kernel-bootiso-rebuild: PASS
EVIDENCE stop-kernel-bootiso-rebuild: `kernel-step.log` `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`:
`checking outputs of '/nix/store/z9cdam743vfcjmm0h20bdmrr9xdbi7dx-r9-boot-iso-4624ea2.drv'...`
then `KERNEL-BOOTISO-REBUILD-OK`; per-step summary `kernel 0 1` and log ends `EXIT=0`.

VERDICT stop-x86-host-tests: PASS
EVIDENCE stop-x86-host-tests: `host-tests.log` `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`:
`Executing cd "/home/req/r9-p02-stop/run" && ".../cargo" "test" "--package" "x86_64" "--bins"
"--target" "x86_64-unknown-linux-gnu"`, `test result: ok. 140 passed; 0 failed; 0 ignored;
0 measured; 0 filtered out`; log ends `EXIT=0`.

VERDICT stop-x86-rfork-mask-test: PASS
EVIDENCE stop-x86-rfork-mask-test: `host-tests.log` `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`,
x86_64 leg (the `--target x86_64-unknown-linux-gnu` command above):
`test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok` — from
`cargo xtask test --verbose`.

VERDICT stop-gate-kvm: PASS
EVIDENCE stop-gate-kvm: `gate-kvm.log` `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`:
`Running `target/debug/xtask gate --arch x86-64 --accel kvm``, one run, then
`GATE PASS quick x86_64 run=765355-1795495125`; log ends `EXIT=0`.

VERDICT stop-gate-tcg: PASS
EVIDENCE stop-gate-tcg: `gate-tcg.log` `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`:
`Running `target/debug/xtask gate --arch x86-64 --accel tcg``, one run, then
`GATE PASS quick x86_64 run=775293-1795495319`; log ends `EXIT=0`.

VERDICT stop-gate-full: PASS
EVIDENCE stop-gate-full: `gate-full.log` `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf`:
`Running `target/debug/xtask gate --arch x86-64 --full --accel kvm``, then
`GATE PASS full x86_64-linux run=785121-1795495519`, all eight apps `PASS` in
`full/out/full/785121-1795495519/full.tsv`, and the log closes
`R9-WITNESS-RUN-OK: every selected step passed` with `EXIT=0`.

## Notes

- Each of the six verdict logs starts with `HEAD=4624ea2f0cb45c81870dcc881d544d7d36a2e0bf` and
  ends with `EXIT=0`; `grep -c '^HEAD='` and `grep -c '^EXIT='` are 1 on each.
- `boot-iso.log` is two lines (`HEAD=…`, `EXIT=0`) because `nix build .#boot-iso` succeeded
  with no output; the `kernel` step then did the plain build and the `--rebuild` check of
  `r9-boot-iso-4624ea2`, which is the byte measurement.
- No `got:` line and no `hash mismatch` appears in round 2: the committed hash built as
  committed.
- The gated commit is the branch tip with both return commits left out:
  `git diff 4624ea2f0cb45c81870dcc881d544d7d36a2e0bf origin/phase-02/design-lock -- .
  ':(exclude)…/owner-stop/return'` is empty.
- Round 1's files remain under `return/` (commit `a0454ff`) and were not used for any verdict.
- `full/out/secret-scan.txt` reports a clean credential scan, `full/out/survivors.txt` reports
  no surviving qemu/Xvfb/drawterm processes, and no credential string appears in the round-2
  logs; nothing needed redaction.
VERDICT stop-main-push: PASS 3d01f071efda9ed0720f1a07c4a6db0c7af8f4c5
