---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 12
subsystem: arm64 cpu-port measurements (group C), arm64 image fixtures
tags: [r9, aarch64, host-test, devssl, devcap, shebang, remove, wstat, bootconf, sslprobe, profile-decision]
status: complete
requires: ["20-11"]
provides:
  - "HOST-TEST-BASE: same (default cfg 32/32 at fa38402, 18/18 at 8e4e730; virtshell cfg fails E0659 at both)"
  - "HOST-TEST-ROUTE: A (owner-chosen; the measured route is B; the one-line platform/mod.rs fix plus the xtask cfg pass land in 20-17 Task 1)"
  - "PROFILE-FACT: kernel-virt builds --config virtshell; PROFILE-DECISION: separate-profile (virtcpu / kernel-virt-cpu / boot-r9-cpu)"
  - "DEVMNT-REMOVE: absent"
  - "build/aarch64/sslprobe baked into bootfs.paq (/arm64/bin/sslprobe), not gate-wired"
  - "consumer table with an HVF and TCG baseline for devssl, devcap, #!, REMOVE, WSTAT, and a source-only bootconf row"
affects: [20-13, 20-14, 20-15, 20-16, 20-17]
tech-stack:
  added: []
  patterns:
    - "type probe markers as PR-''X so the echoed command line never matches the done regexp"
    - "keep every typed console line under 256 bytes on HVF (the PL011 receive buffer, CONS_RX_CAP)"
    - "run base-revision r9 commands with the HEAD devShell (nix develop /Users/neilskinner/code/r9 -c ...) when the base's own devShell does not evaluate on darwin"
key-files:
  created:
    - build/aarch64/sslprobe.c
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-12-SUMMARY.md
  modified:
    - build/aarch64/subset-build.sh
    - build/aarch64/mkimage.sh
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
    - .planning/STATE.md
    - .planning/ROADMAP.md
key-decisions:
  - "Owner: the devcap profile route is separate-profile: new r9 profile virtcpu (virtshell devices plus cap), package kernel-virt-cpu, Harvey app boot-r9-cpu; the existing four arm64 apps keep kernel-virt"
  - "Owner: host-test route A for 20-17. The fix (narrow the cfg(test) raspi3b glob gate in aarch64/src/platform/mod.rs, pass the virtshell cfg from cargo xtask test) lands in 20-17 Task 1, not here, because this plan commits no r9 source"
  - "A #! fixture must be baked mode 755 into bootfs.paq; ramfs cannot host one, because it refuses OEXEC on a 0664 file and no chmod path exists on any device this era"
  - "A REMOVE that succeeds must target devcap, devenv or devsrv (the arms in port::namec::remove_chan); a ramfs file only ever shows the loud refusal, since devmnt has no Tremove"
requirements-completed: []
metrics:
  duration: "about 5 h 41 min wall (2026-11-21T03:32Z to 09:13Z), most of it waiting at the Task 3 checkpoint"
  completed: 2026-11-21
actuals:
  tokens: 5600
  tasks: 3
  commits: 2
plan_head_before: 0421d57c81d866649e680f31c036b9c08196553a
plan_head_after: b29958b7cdfa9858839cc5957b9422e3f96794d3
---

# Phase 20 Plan 12: group C measured (host-test route A by owner choice over a measured B, sslprobe baked, a consumer and baseline for every cpu-port arm, devcap packaged as a separate virtcpu profile) Summary

The group C unknowns are now recorded facts or owner decisions. The aarch64 crate runs as a host test binary at the default cfg, but not with the virtshell cfg. The cause is one line, and the owner chose route A, so 20-17 makes that fix. Every cpu-port arm has a command in the image that reaches it, and its baseline on the pinned kernel is the same on HVF and TCG. devcap gets its own kernel profile.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Host-test compilability | `c54d11245` | 20-EVIDENCE.md |
| 2 | sslprobe + consumer baselines | `b29958b7c` | sslprobe.c, subset-build.sh, mkimage.sh, 20-EVIDENCE.md |
| 3 | Owner chooses the devcap profile route | (final docs commit) | 20-EVIDENCE.md |

## Evidence

The full record is in `20-EVIDENCE.md` under `## Group C measurements`.

**Host tests (r9 `fa38402` and base `8e4e730`).**

| Rev | default cfg | virtshell cfg |
|---|---|---|
| `fa38402` | 32/32 | E0659 `devcons` is ambiguous (4 errors) |
| `8e4e730` | 18/18 | the same E0659, plus the two research compile-drift errors |

- `cargo xtask test` runs only the port leg. With no rustup, `std_supported_target("aarch64")` returns `None`.
- The obstacle is the `cfg(test)` arm of the raspi3b glob gate in `aarch64/src/platform/mod.rs`. With it narrowed (a scratch edit, since reverted), the virtshell cfg compiled and ran 38/38, including a test inside `exec.rs`.
- `new_user_proc` does not run on the host (`pagealloc ... OutOfSpace`), so the x86_64 `MockStack` pins do not port as they are.

**Consumers, on the pinned kernel (HVF = TCG).**

| Arm | Today |
|---|---|
| devssl | `sslprobe: FAIL pushssl0: pread: unknown device` |
| devcap | `namec: unknown # device` (no `cap` in virtshell) |
| #! | exec fails at ramfs's mode-bit check (`permission denied`) |
| REMOVE | `syscall 25 unimplemented` |
| WSTAT | `syscall 44 unimplemented` |
| bootconf | source only: no `bootargs` reader, no `cpurc`; `init=` is the only route to `init -c` |

The TCG smoke gate passed with sslprobe baked in (exit 0, `ALL STAGES PASS`).

## Deviations from Plan

### Scope changes (owner-directed)

**1. `HOST-TEST-ROUTE` records the owner's A, not the measured B.** The coordinator relayed the owner's choice. The evidence line reads `HOST-TEST-ROUTE: A` with the note "owner-chosen, fix pending", and the measured B and its reasoning stay in the paragraph below it. The fix is not in this plan: this plan commits no r9 source, and 20-17 Task 1 already owns `TestStep::run` and passing the cfg.

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The base worktree's devShell does not evaluate on darwin.**
- **Found during:** Task 1, step 4a.
- **Issue:** at `8e4e730`, `nix develop` fails on `grub-2.14` (research Pitfall 4).
- **Fix:** I ran the base commands with the HEAD devShell, `cd`'d into the worktree. `rust-toolchain.toml` is identical at both revisions, so it is the same compiler. No repo file changed.

**2. [Rule 3 - Blocking] One 295-byte typed line stalled on HVF.**
- **Found during:** Task 2, the `#!` / chmod / rm run.
- **Issue:** the echo stopped at byte 255 and the probe timed out. The cause is the 256-byte PL011 receive buffer (`CONS_RX_CAP`).
- **Fix:** I split the line into two (189 and 98 bytes), and the HVF text then matched TCG. Recorded as a venue note: arm64 fixture lines must stay under 256 bytes.

**3. [Rule 2 - Correctness] Added a REMOVE/WSTAT consumer on `/env`.** devmnt has no `Tremove` (`DEVMNT-REMOVE: absent`), and x86_64 refuses `rm` over a mount too, so a ramfs `rm` could never show a working remove. I added a devenv `rm` and `chmod` baseline, devenv being a device whose remove arm is already in `remove_chan`.

## Known Stubs

None. sslprobe is complete. It is not wired into a gate, which is intentional (plan prohibition); 20-16 wires it.

## Threat Flags

None. T-20-31: the sslprobe secrets are fixture constants, and the file-top comment says so. T-20-SC: no package was added, and the scratch r9 edits were reverted (r9 `git status --porcelain` is empty, one worktree).

## Notes

- `commits` is the measured Harvey count `0421d57c8..HEAD` when this was written. The final docs commit (this SUMMARY, the decision record, STATE, ROADMAP) adds one more.
- `actuals.tokens` is chars/4 over the added lines in `build/aarch64` and the evidence file.
- No r9 commit and no push. The kernel under test was the pin (`y1gq491k…-r9-kernel-virt-fa38402`) with no `--override-input`.
- The 11 pre-existing font/postscript modifications stayed unstaged.

## Self-Check: PASSED

- FOUND: `build/aarch64/sslprobe.c`. `build/aarch64/out/sslprobe` has magic `00008c47`, and `/arm64/bin/sslprobe` passes the mkimage manifest assert.
- FOUND (ancestors of HEAD): `c54d11245`, `b29958b7c`.
- `## Group C measurements` has one each of `HOST-TEST-BASE: same`, `HOST-TEST-ROUTE: A`, `PROFILE-FACT: kernel-virt builds --config virtshell`, `DEVMNT-REMOVE: absent` and `PROFILE-DECISION: separate-profile`, plus 6 consumer rows.
- r9: `dirty=0 worktrees=1`.
