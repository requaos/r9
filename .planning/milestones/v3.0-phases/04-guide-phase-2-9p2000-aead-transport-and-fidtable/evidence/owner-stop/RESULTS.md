# Phase 4 end-of-phase owner stop — results

Plan 04-15, Task 2. The owner approved the branch push, ran the x86_64 Linux checklist of
`04-OWNER-STOP.md` and returned the logs through the branch under
`evidence/owner-stop/return/`. Every verdict below is decided only from a line in those logs
or from the git checks this task names; no step was run on the Mac. All eleven verdicts are
PASS and none is FAIL, so the stop rules do not apply and Task 3's `main`-push decision follows.

- branch: `phase-04/aead-wire`
- pushed tip: the last Mac commit `a2ff50d2e7c0dbab90cf8061d15e240f8a6d8c04` (run 1's
  `REPORT.txt`: `pushed-tip: a2ff50d2e7c0dbab90cf8061d15e240f8a6d8c04`)
- hash commit: `ca2b5abff238f00f5c024d76091e5c9e1fbd816f` — `flake: re-derive boot-iso
  outputHash for the Phase 4 kernel (port links crypto and ds)`, `flake.nix` only
- return commits: `89ba121ebe1c15abd0ad207dcd87a73a274e894c` (run 1, files under
  `evidence/return/`, normalised here to `evidence/owner-stop/return/run1/`) and
  `405984d2c2bd391900c987668d14c78c23c23005` (run 2, files under
  `evidence/owner-stop/return/`); both add evidence files only
- `origin/main` is `a64352625efe9cb8a924d7941c6be09c4f738a11`, unchanged by this plan's tasks
- result: **11 of 11 verdicts PASS**

pushed-tip a2ff50d2e7c0dbab90cf8061d15e240f8a6d8c04

hash-commit ca2b5abff238f00f5c024d76091e5c9e1fbd816f

new-outputHash sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=

gated-commit ca2b5abff238f00f5c024d76091e5c9e1fbd816f

## Verdicts

VERDICT stop-branch-push: PASS
EVIDENCE stop-branch-push: `git ls-remote origin refs/heads/phase-04/aead-wire` ->
`405984d2c2bd391900c987668d14c78c23c23005`; the branch base is the last Mac commit
`a2ff50d2e7c0dbab90cf8061d15e240f8a6d8c04` (`git merge-base main
origin/phase-04/aead-wire`; run 1's `REPORT.txt`: `pushed-tip:
a2ff50d2e7c0dbab90cf8061d15e240f8a6d8c04`), and the three commits above it are the hash
commit plus the two return commits.

VERDICT stop-host-test: PASS
EVIDENCE stop-host-test: `host-tests.log` `HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c`
then `EXIT=0` — 36 `test result: ok` lines, every one `0 failed`, no `FAILED` line, and the
final line is `test result: ok. 140 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 0.01s`. Run 1's `host-tests.log` at `HEAD=a2ff50d2e7c0dbab90cf8061d15e240f8a6d8c04`
also ends `EXIT=0`.

VERDICT stop-host-test9p: PASS
EVIDENCE stop-host-test9p:
`host-test-9p.log` `HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c` then `EXIT=0`, with all
five `aead_*` legs run — `tests/aead_fid.rs`, `tests/aead_negotiate.rs`, `tests/aead_tlv.rs`,
`tests/aead_transport.rs` and `tests/aead_wire.rs` — and every `test result:` line
`0 failed`.

VERDICT stop-kernel-x64-build: PASS
EVIDENCE stop-kernel-x64-build: `tip-kernel-x64.log`
`HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c` then `EXIT=0` after
`building '/nix/store/zz01cmszbf7kvf5arhhv0r1jfmzw1kjl-r9-kernel-x64-89ba121.drv'...` — the
sandboxed `kernel-x64` derivation builds on the x86_64 host with `port` linking `crypto`
and `ds`.

VERDICT stop-kernel-virt-build: PASS
EVIDENCE stop-kernel-virt-build: `tip-kernel-virt.log`
`HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c` then `EXIT=0` after
`building '/nix/store/y3dr26892bpgkp340yg17kal6cad2mr0-r9-kernel-virt-89ba121.drv'...` — the
sandboxed `kernel-virt` derivation builds on the x86_64 host with the enlarged `Cargo.lock`.

VERDICT stop-bootiso-hash-commit: PASS
EVIDENCE stop-bootiso-hash-commit: own commit `ca2b5abff238f00f5c024d76091e5c9e1fbd816f`,
`git show --stat` lists `flake.nix | 3 ++-` (one deletion, two insertions — the dated comment
plus the value), authored by `Neil Skinner <reqpro@requaos.com>`, and `git show` changes only
`- outputHash = "sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=";` ->
`+ outputHash = "sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=";`; the message carries
no attribution line. Run 1's `boot-iso-first.log` at
`HEAD=a2ff50d2e7c0dbab90cf8061d15e240f8a6d8c04` is the mismatch that produced the value —
`specified: sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`, `got:
sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=`, `EXIT=1`.

VERDICT stop-bootiso-build: PASS
EVIDENCE stop-bootiso-build: run 1's `boot-iso-after.log`
`HEAD=ca2b5abff238f00f5c024d76091e5c9e1fbd816f` then `EXIT=0` after building
`r9-kernel-x64-ca2b5ab.drv` and `r9-boot-iso-ca2b5ab.drv` with the committed hash; run 2's
`boot-iso-first.log` `HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c` then `EXIT=0` after
`building '/nix/store/ffpf6hl03sb581zwxbvlmccxqg49ibsi-r9-boot-iso-89ba121.drv'...` — the ISO
builds green with the hash committed at `ca2b5ab`.

VERDICT stop-bootiso-rebuild: PASS
EVIDENCE stop-bootiso-rebuild: `boot-iso-rebuild.log`
`HEAD=ca2b5abff238f00f5c024d76091e5c9e1fbd816f`, then
`checking outputs of '/nix/store/rrgw80rjcfm2q73adnlwz2ysnnjsnp0j-r9-boot-iso-ca2b5ab.drv'...`,
then `EXIT=0`, with no `hash mismatch` and no `got:` line.

VERDICT stop-gate-kvm: PASS
EVIDENCE stop-gate-kvm: `gate-kvm.log` `HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c`:
`GATE PASS quick x86_64 run=34940-1795575414`, then `EXIT=0` — boot to rio under KVM.

VERDICT stop-gate-tcg: PASS
EVIDENCE stop-gate-tcg: `gate-tcg.log` `HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c`:
`GATE PASS quick x86_64 run=45066-1795575611`, then `EXIT=0` — run natively on the x86_64
host, as the kernel byte-change rule requires.

VERDICT stop-gate-full: PASS
EVIDENCE stop-gate-full: `gate-full.log` `HEAD=89ba121ebe1c15abd0ad207dcd87a73a274e894c`:
`GATE PASS full x86_64-linux run=55162-1795575813`, then `EXIT=0`.

Disclosure for the three gates: they ran at `89ba121ebe1c15abd0ad207dcd87a73a274e894c`, the
branch tip as it stood during run 2 — the hash commit plus the logs-only return commit — whose
source tree is identical to the gated commit `ca2b5abff238f00f5c024d76091e5c9e1fbd816f`
(`git diff --quiet 89ba121 405984d -- port crypto ds xtask aarch64 x86_64 Cargo.lock flake.nix`
exits 0, and the same check against `ca2b5ab` exits 0). The `boot-iso` rebuild at
`HEAD=ca2b5ab` exits 0 with the hash committed for `ca2b5ab`, which proves the kernel bytes did
not move between the gated commit and the tip the gates measured.

## Deviations

1. **Two return commits instead of at most one.** `89ba121` adds files under
   `evidence/return/` (the runner's first placement) and `405984d` adds the same logs under
   `evidence/owner-stop/return/`. Both add evidence files only: `git diff --quiet ca2b5ab
   89ba121 -- port crypto ds xtask aarch64 x86_64 Cargo.lock flake.nix` and the same check for
   `89ba121..405984d` both exit 0. The run-1 files were normalised in this task by `git mv` to
   `evidence/owner-stop/return/run1/` with their contents unchanged (sha256 checked per file),
   so the run-1 record — including the `boot-iso-first` `EXIT=1` with the `got:` value that
   justified the hash commit — is preserved in the canonical tree.
2. **The three gates were measured at the return commit (`89ba121`) rather than at the hash
   commit (`ca2b5ab`).** Disclosed above: the source tree is identical between the two commits,
   and the `boot-iso` rebuild at `ca2b5ab` is green with the committed hash.
3. **Run 2's `REPORT.txt` records `hash-commit: none`** ("boot-iso-first built green: no hash
   commit is needed") because the hash had already been committed by run 1 on the branch. Run
   1's `REPORT.txt` is the authoritative record of the hash commit and its value:
   `hash-commit: ca2b5abff238f00f5c024d76091e5c9e1fbd816f`, `new-outputHash:
   sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=`. No code changed between the runs.

## Notes

- Each verdict log begins with its `HEAD=` line and ends with its `EXIT=` line.
- No `hash mismatch` and no `got:` line appears after the hash commit: the committed value
  built as committed, and the `--rebuild` check confirms it.
- The only `got:` line in the return is run 1's `boot-iso-first`, which is the expected
  first-round mismatch named in section 5 of the checklist.
- No credential string appears in the returned logs; nothing needed redaction.

## Task 3: the `main` push ruling

The owner replied "push-main" at the checkpoint that showed `git log --oneline
origin/main..main` and the eleven verdict lines above. The sha shown to the owner was
`5d1549af07de7220d6f48aac0d0db221aa7c541c` (`git rev-parse HEAD` before the push), and the
executor ran exactly `git push origin main` once — no force, no other ref. `origin/main` moved
`a64352625efe9cb8a924d7941c6be09c4f738a11` -> `5d1549af07de7220d6f48aac0d0db221aa7c541c`, which
equals the sha shown. The branch `phase-04/aead-wire` is unchanged at
`405984d2c2bd391900c987668d14c78c23c23005`. This evidence commit stays local until a later
approved push.

VERDICT stop-main-push: PASS 5d1549af07de7220d6f48aac0d0db221aa7c541c
