# Phase 3 closing evidence (plan 03-10)

The evidence the Mac can produce before the owner stop: the clippy probe that proves the lint
contract rejects an undocumented `unsafe` block in `ds/` (Task 1), the audit count
reconciliation (Task 1), the full suite twice (Task 2) and the Miri timings (Task 2).  Every
verdict below comes from the run recorded beside it; nothing is written for a command that did
not run.

## Task 1 — the `UNSAFE-AUDIT.md` count and the clippy probe

### Source enumeration (the audit's `ds/` count)

Method: for every `.rs` file under `ds/src/tpht/`, strip `//` line comments and list each
remaining occurrence of the `unsafe` token, with file, line and kind.  The same scan over
`crypto/src` and over `ds/src` outside `tpht/` finds nothing.

| Scope | Lint-visible blocks | `unsafe impl` | `unsafe fn` | `unsafe extern` | Test-only blocks |
|---|---|---|---|---|---|
| `ds/src/tpht/` | 1 (`flattened.rs:440`, `deref_tiny`'s `get_unchecked`) | 0 | 0 | 0 | 0 |
| `crypto/src` | 0 | 0 | 0 | 0 | 0 |
| `ds/src` outside `tpht/` | 0 | 0 | 0 | 0 | 0 |

AUDIT-COUNT tpht source=1 audit=1

### The clippy probe (scratch edit, not committed)

A clean tree at the start (`git status --porcelain -- ds crypto xtask` printed nothing).  One
scratch edit: the six-line `// SAFETY:` comment above the `unsafe` block in
`ds/src/tpht/flattened.rs` (`get_unchecked` in `deref_tiny`) was deleted; nothing else changed.

Command: `nix develop -c cargo xtask clippy` (the default arch form, which is what hosted CI
runs; `--arch aarch64 --config virtshell` is pre-existing red and is not a criterion).

Decisive output:

```text
error: unsafe block missing a safety comment
   --> ds/src/tpht/flattened.rs:434:21
    |
434 |         let words = unsafe { self.arena.get_unchecked(slot) };
    |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = help: consider adding a safety comment on the preceding line
    = note: requested on the command line with `-D clippy::undocumented-unsafe-blocks`

error: could not compile `ds` (lib) due to 1 previous error
clippy failed
EXIT=1
```

VERDICT clippy-rejects-undocumented-unsafe: PASS

Exit code: `1`.  Reverted with `git checkout -- ds/src/tpht`; `git status --porcelain -- ds`
then printed nothing, so the probe is not in any commit.

## Task 2 — the full suite twice, and the kernel bytes at the end of the phase

Both runs were made at the phase's last code commit,
`1be121c11fc712648fe5876fb70e591eefde1a00` (Task 1's commit; Task 2 and Task 3 change only
evidence and planning documents), on the venue `Darwin arm64` — `uname -sm` was compared with
the `venue` line of `evidence/kernel-bytes/before.sha256` before the first run, and the tree
was clean (`git status --porcelain --untracked-files=no` printed nothing).  Each run ran, in
this order, `nix develop -c cargo xtask test`, `nix develop -c cargo test -p xtask`,
`nix develop -c cargo xtask check`, `nix develop -c cargo xtask clippy` and
`nix develop -c cargo xtask miri`; each command's full output was kept under
`target/phase03-closing/run-{1,2}/` (scratch, not committed).  A `PASS` needs all five exit
codes to be 0, and each verdict below is decided only from that run's own exit codes and
output.

### suite-run-1

- commit: `1be121c11fc712648fe5876fb70e591eefde1a00`
- exit codes: `xtask-test` 0, `cargo test -p xtask` 0, `xtask-check` 0, `xtask-clippy` 0,
  `xtask-miri` 0
- `cargo xtask test`: 30 `test result: ok` lines, no `FAILED` line, no `lint-contract:`
  error
- default `crypto` leg (`crypto-732a131f1e6ef9e8`): `rfc4231` 7 passed, `rfc5869` 6 passed,
  `rfc8439` 6 passed, `blake3_vectors` 5 passed, `keys` 8 passed
- doctest pair: `test crypto/src/key.rs - key::KeyCapability<ReadOnly> (line 201) ... ok` and
  `test crypto/src/key.rs - key::KeyCapability<ReadOnly> (line 212) - compile fail ... ok`
- soft-backend `crypto` leg (`crypto-ec5be3eeea38434d`, the same five binaries recompiled with
  `--cfg chacha20_backend="soft"` and `--cfg sha2_backend="soft"`): `rfc4231` 7 passed,
  `rfc5869` 6 passed, `rfc8439` 6 passed, `blake3_vectors` 5 passed, `keys` 8 passed
- `ds` lib leg: `test result: ok. 42 passed; 0 failed; 1 ignored` — `roaring::` 6,
  `clientset::` 5, `setops::` 2, `tpht::` 30 (29 passed and the `#[ignore]`d D-15 generator)
- Miri: `test result: ok. 29 passed; 0 failed; 1 ignored; 0 measured; 13 filtered out` on each
  of the 64 seeds, with no `Undefined Behavior`, `Data race` or `FAILED` line

VERDICT suite-run-1: PASS
MIRI-TIME closing-run-1 350

### suite-run-2

- commit: `1be121c11fc712648fe5876fb70e591eefde1a00`
- exit codes: `xtask-test` 0, `cargo test -p xtask` 0, `xtask-check` 0, `xtask-clippy` 0,
  `xtask-miri` 0
- `cargo xtask test`: 30 `test result: ok` lines, no `FAILED` line, no `lint-contract:`
  error
- default `crypto` leg (`crypto-732a131f1e6ef9e8`): `rfc4231` 7 passed, `rfc5869` 6 passed,
  `rfc8439` 6 passed, `blake3_vectors` 5 passed, `keys` 8 passed
- doctest pair: both the passing twin (line 201) and the `compile fail` twin (line 212) `ok`
- soft-backend `crypto` leg (`crypto-ec5be3eeea38434d`): `rfc4231` 7 passed, `rfc5869` 6
  passed, `rfc8439` 6 passed, `blake3_vectors` 5 passed, `keys` 8 passed
- `ds` lib leg: `test result: ok. 42 passed; 0 failed; 1 ignored` — `roaring::` 6,
  `clientset::` 5, `setops::` 2, `tpht::` 30 (29 passed and the `#[ignore]`d D-15 generator)
- Miri: `test result: ok. 29 passed; 0 failed; 1 ignored; 0 measured; 13 filtered out` on each
  of the 64 seeds, with no `Undefined Behavior`, `Data race` or `FAILED` line

VERDICT suite-run-2: PASS
MIRI-TIME closing-run-2 389

### The Miri feedback budget

Both Miri runs are over the 120-second budget of `03-VALIDATION.md`: 350 s and 389 s for the
64-seed `tpht` suite (29 tests per seed), against the 116 s that the 13-test suite measured in
plan 03-13 and the 402 s that plan 03-08 reported for the 29-test suite at its resize floors.
No source file is changed in this plan, so this does not fail either suite verdict; it is
recorded here and reported in the plan's SUMMARY for the phase-end review, as plans 03-07 and
03-08 recorded their own overages.

### Kernel bytes at the end of the phase

Recorded in `evidence/kernel-bytes/after-03-10.sha256` (the baseline's four-line format, the
`after` line first) and in `evidence/kernel-bytes/KERNEL-BYTES.md` ("End of the phase (plan
03-10)"), with the two verdicts `elf-virtshell@03-10` and `elf-q35churnshell@03-10`.  Both
kernels were rebuilt from nothing after their target directories were moved aside, and both
sha256 values equal the baseline taken before the first workspace change on this venue
(`9218b998…fa083e` for `virtshell`, `c9dcef0b…8b78b` for `q35churnshell`); a byte-for-byte
`cmp` against the copies kept under `target/phase03-kernel-bytes/before/` also reports no
difference.

