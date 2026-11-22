---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 04
subsystem: r9 CI + r9 xtask test + r9 devShell + aarch64 boot-test profiles
tags: [r9, ci, github-actions, xtask, nix, devshell, darwin, boot-test, aarch64, dev_exec]
status: complete
requires: ["20-03"]
provides:
  - "r9 29f0c3d: CI job build-aarch64-virtshell (cargo xtask build --arch aarch64 --config virtshell); cargo xtask test runs port --lib --tests"
  - "r9 5f3ddb0: default devShell gates grub2/xorriso behind hostPlatform.isLinux; nix develop enters on darwin-arm64"
  - "aarch64 boot-test profiles observed green under TCG at r9 5f3ddb0: virt virtlowecam virtpanic virtexec virtexec3 virtmulti virtnotes virtnet virtip (9/9); virtshell observed through Harvey's composed smoke instead"
affects: [20-06]
tech-stack:
  added: []
  patterns: ["a CI lane per non-default kernel config whose code the default config does not compile"]
key-files:
  created: []
  modified:
    - ../r9/.github/workflows/xtask.yml
    - ../r9/xtask/src/main.rs
    - ../r9/flake.nix
key-decisions:
  - "No marker re-key was needed: virtnotes passes with the push_note ready arm as it stands, so Task 3 commits nothing in r9"
  - "virtshell has no cargo xtask boot-test profile by design (its TOML has no [qemu] initrd key; Harvey's run-r9.sh delivers the image). It was observed through Harvey's composed smoke at the same r9 HEAD rather than by adding an initrd key to the profile"
requirements-completed: [ARM-01]
metrics:
  duration: "about 12 min (11:40Z to 11:52Z)"
  completed: 2026-11-18
actuals:
  tokens: 510
  tasks: 3
  commits: 0
plan_head_before: d8b8efbfe80c1689d20f7a75203d7df29bd6da17
plan_head_after: d8b8efbfe80c1689d20f7a75203d7df29bd6da17
r9_head_before: 8958c9e
r9_head_after: 5f3ddb0
r9_commits: 2
---

# Phase 20 Plan 04: r9 CI lane for aarch64 dev_exec, port integration tests, darwin devShell, boot-test re-green Summary

r9 CI now has a `build-aarch64-virtshell` job, so every push and PR to main compiles the aarch64 `dev_exec` code; the default raspi3b config never did, which is how the drift reached the pin. `cargo xtask test` now runs `port/tests/*.rs`. `nix develop` in r9 now enters on darwin. Nine of the ten named aarch64 boot-test profiles pass under TCG at r9 `5f3ddb0` without a re-key. The tenth, `virtshell`, was never an xtask boot-test profile; it boots green through Harvey's composed smoke.

`commits: 0` is the measured Harvey count before this SUMMARY's own commit. This plan's code lives in r9: `29f0c3d` and `5f3ddb0`, both local and unpushed.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | CI lane for aarch64 dev_exec; port integration tests under `cargo xtask test` | r9 `29f0c3d` | `.github/workflows/xtask.yml`, `xtask/src/main.rs` |
| 2 | r9 default devShell evaluates on darwin | r9 `5f3ddb0` | `flake.nix` |
| 3 | Re-green the aarch64 dev_exec boot-test profiles | none (no re-key needed) | none |

## Evidence

**Task 1**
- Non-comment lines with `--arch aarch64 --config virtshell` in `xtask.yml`: 1.
- Inside `nix develop .#kernel-virt`, `cargo xtask test` ran 13 test binaries, including `Running tests/devenv_test.rs`, and every `test result:` line says `ok`: 165 lib tests plus the integration binaries, 0 failed, and no `FAILED` line.
- The commit touches exactly the two files (+13 lines).

**Task 2**
- Before the change, `nix develop` at r9 `29f0c3d` failed with `Refusing to evaluate package 'grub-2.14' ... not available on the requested hostPlatform`.
- After it, `nix develop -c ...` printed cargo 1.95.0-nightly, QEMU 11.1.2, DTC 1.8.1 and `DEVSHELL-DARWIN-OK`.
- `git diff --stat origin/main -- flake.nix`: `1 file changed, 4 insertions(+), 2 deletions(-)`, which is 6 lines, all inside the default devShell's comment and `packages`. The `outputHash` lines are byte-identical to `origin/main`.

**Task 3**
- `sh build/aarch64/stage-build.sh` (inside `nix develop`, lima guest) printed `STAGE-BUILD-OK` and `PASS: staged ...`.
- `build/aarch64/smoke-out/*` now has mtime 2026-11-18 06:48 -0500. The rename commit `27cc1d25e` is 2026-11-17 08:46 -0500, so these are newer. The directory is gitignored (`.gitignore:116`) and nothing in it is staged.
- `target/bootfs.paq` was rebuilt from it (06:49). `virtnet` and `virtip` use `harvey:build/aarch64/bootfs.paq` (2026-11-17 23:10, after the rename).
- `R9_HARVEY_DIR=/Users/neilskinner/code/harvey cargo xtask boot-test --arch aarch64 --config <c>` at r9 `5f3ddb0`, TCG per each TOML, from `/Users/neilskinner/code/r9`:

  | Profile | Exit | Duration | Result line |
  |---------|------|----------|-------------|
  | virt | 0 | 7 s | PASS: banner, >= 3 ticks, echo probe |
  | virtlowecam | 0 | 4 s | PASS: banner, >= 3 ticks, echo probe (low ECAM) |
  | virtpanic | 0 | 3 s | PASS: virtpanic start, >= 30 register rows, clean halt |
  | virtexec | 0 | 4 s | PASS: exec entry, init, hello, all-exited |
  | virtexec3 | 0 | 2 s | PASS: exec entry, init, hello, all-exited |
  | virtmulti | 0 | 2 s | PASS: all smoke markers (hello + smokefork) |
  | virtnotes | 0 | 1 s | PASS: all smoke markers incl. smokenote got-note/resumed, >= 1 sched-switch, all-exited |
  | virtnet | 0 | 4 s | PASS: real boot chain, rc prompt, typed arp sweep |
  | virtip | 0 | 5 s | PASS: real boot chain, rc prompt, typed netip loopback sweep |
  | virtshell | 1 | 1 s | `dev_exec: config carries no initrd image` (see Deviation 1) |

- **virtshell, the substitute observation:** `R9_ACCEL=tcg nix run .#boot-r9-smoke --no-write-lock-file --override-input r9 git+file:///Users/neilskinner/code/r9` built `r9-kernel-virt-5f3ddb0`, which is the virtshell config (flake.nix:437). It exited 0 in 57 s with `smoke-r9: ALL STAGES PASS`. `flake.lock` is unchanged.
- No marker constant changed. Research's likeliest re-key, `virtnotes`, passes as the code stands.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] virtshell is not a `cargo xtask boot-test` profile**
- **Found during:** Task 3
- **Issue:** the plan names `virtshell` as the tenth boot-test profile. Its TOML says `NO initrd key: the shell image is baked and delivered Harvey-side (bootfs.paq via run-r9.sh's loader+fdtput argv), not by xtask`. `boot-test` has no initrd override and no `virtshell` marker profile (the x86_64 counterpart is `q35shell`; there is no aarch64 equivalent). So `dev_exec` panics at `aarch64/src/main.rs:460`, and the always-on negative scan fails the run. This follows from the profile's design and is not a regression from 20-01 or 20-03.
- **Fix:** I observed the virtshell kernel through the gate that does boot it, Harvey's composed smoke under TCG at the same r9 HEAD: ALL STAGES PASS. I did not add an initrd key or a new boot-test profile. That would change a deliberate profile design, so it is left as a possible follow-up for the owner.
- **Files modified:** none
- **Effect on the must-have:** the literal truth "ten profiles pass" is 9/10. The tenth is covered by the substitute gate. The verifier should judge whether that is enough.

### Notes

- `gsd-tools windows append` refused to record Deviation 1: `Ledger counts disagree with entries: frontmatter open/waived/fixed/total=52/3/13/68 but entries yield 49/5/14/68`. That count mismatch in `.planning/WINDOWS.md` was already there before this plan. The ledger is best-effort, so I did not repair it here. This SUMMARY records the entry.
- Nothing was pushed. Hosted CI for the new job is observed in 20-06. The pre-existing modified troff and postscript files were not staged.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: r9 `.github/workflows/xtask.yml` job `build-aarch64-virtshell`; `xtask/src/main.rs` has `"--tests"` after `"--lib"`; `flake.nix` has the `isLinux` gate
- FOUND (ancestors of r9 HEAD): `29f0c3d`, `5f3ddb0`
- r9 tracked tree clean; `build/aarch64/smoke-out/` gitignored and unstaged
