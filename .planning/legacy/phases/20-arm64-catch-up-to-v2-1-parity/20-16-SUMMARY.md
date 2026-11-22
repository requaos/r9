---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 16
subsystem: arm64 cpu fixture gate (boot-r9-cpu), the cpu boot path, and the batched r9 pin move
tags: [arm64, aarch64, cpurc, shebang, devcap, devssl, wstat, remove, bootconf, flake, pin, ARM-11]
status: complete
requires: ["20-15"]
provides:
  - "CPURC-RUNS: yes; AFTER-CPURC: prompt; CPU-CAP-LEG: write-ok; CPU-GATE-ROUTE: as-planned (owner: as-planned)"
  - "build/aarch64/root-tree/rc/bin/cpurc (100755) and lib/shebangecho (100755, 29 B): the #! cpu fixture"
  - "build/aarch64/cpuprobe.c: stat and cap legs with exact errstr checks, baked via subset-build.sh + mkimage.sh"
  - "nix run .#boot-r9-cpu (nix/cpu-r9.sh + build/aarch64/cpu-r9.expect) on r9 kernel-virt-cpu, lazily absent where the package is missing"
  - "CPU-FORKS-PER-BOOT = 17"
  - "Harvey pin r9 c94bc9b -> 821db85 (fd80f6a70, lock-only), quick pass green, full series deferred to phase end by owner"
affects: [20-17]
tech-stack:
  added: []
  patterns:
    - "a scratch measurement fixture can run a real gate marker binary (stacksmoke) so the existing driver goes on to characterize the post-script state instead of riding a stage timeout"
    - "a gate's red proof can run the app's own writeShellApplication script copied out of the store with only the R9_KERNEL_PKG line swapped, which keeps the app's PATH and PLAN9 without baking inside the devshell"
    - "a store-copied script is read-only: chmod u+w before editing, and check the diff before running"
key-files:
  created:
    - build/aarch64/root-tree/rc/bin/cpurc
    - build/aarch64/root-tree/lib/shebangecho
    - build/aarch64/cpuprobe.c
    - nix/cpu-r9.sh
    - build/aarch64/cpu-r9.expect
  modified:
    - build/aarch64/subset-build.sh
    - build/aarch64/mkimage.sh
    - flake.nix
    - flake.lock
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
key-decisions:
  - "Owner: as-planned at Task 2; the driver stages init: starting /bin/rc then the % prompt after cpurc: DONE"
  - "Owner: batch 20-15's pin move with 20-16's: one lock-only move to r9 821db85 plus the quick pass, then a push checkpoint"
  - "boot-r9-cpu sits in its own optionalAttrs keyed on hasAttr kernel-virt-cpu, so older pins evaluate with the four existing apps only"
requirements-completed: [ARM-11]
metrics:
  duration: "about 100 min of execution (2026-11-22T16:20Z to 17:05Z after the checkpoint; Task 1 16:20Z-16:30Z), plus the Task 2 owner checkpoint"
  completed: 2026-11-22
actuals:
  tokens: 12442
  tasks: 4
  commits: 5
plan_head_before: aedaada00f913c676b441382b7545e2c87438534
plan_head_after: f98f2c9e93a9c5b4d42f27efc1a55cc50d38df25
---

# Phase 20 Plan 16: the arm64 cpu fixture gate Summary

`nix run .#boot-r9-cpu` boots arm64 once on the devcap kernel and checks every cpu-port arm live. The boot string `init=/arm64/init -c` reaches the env seed, and init then runs a baked `#!` cpurc. That script calls a second `#!` script, checks the exact wstat, fwstat and remove refusals, writes and rejects devcap capabilities, removes caphash, and runs devssl through a round trip and a mount. The gate passed 3 times on HVF and once on TCG, and it fails at the `cap` leg on a kernel without devcap. r9 is now pinned at `821db85` (20-15 and 20-16 batched), and the quick pass is green there.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Measure the cpu boot path on the `cap` kernel | b64926c3e | 20-EVIDENCE.md |
| 2 | Owner confirms the gate design | answered `as-planned` | — |
| 3 | cpu fixture: cpurc, shebangecho, cpuprobe | ba43f574e | root-tree/rc/bin/cpurc, root-tree/lib/shebangecho, cpuprobe.c, subset-build.sh, mkimage.sh |
| 4 | `boot-r9-cpu` gate | 2deb0cc9e | nix/cpu-r9.sh, build/aarch64/cpu-r9.expect, flake.nix, 20-EVIDENCE.md |
| pin | lock-only r9 pin move and quick pass (owner-batched) | fd80f6a70, f98f2c9e9 | flake.lock, 20-EVIDENCE.md |

Every commit is local. Nothing is pushed.

## Measured (Task 1)

- A scratch `#!` cpurc booted on HVF and TCG through `boot-r9-smoke`, with `R9_BOOTARGS='init=/arm64/init -c'`, on a scratch `cap` virtshell kernel. Its dev list equals virtcpu's. The scratch edit was reverted and r9 is clean.
- The `devenv: bootconf init=/arm64/init -c` line comes before paqfs. The cpurc runs to its end. Every rc idiom the fixture needs works. The caphash write as `glenda` succeeds.
- After cpurc, init prints `init: starting /bin/rc`, then shows rc's `% ` prompt.

## Gate results

- First run (HVF): every leg printed its OK line. The previously unmeasured wstat and fwstat texts on `/lib/namespace` match what cpuprobe expects. The capture after `cpurc: DONE` matches the Task 1 record.
- Series: 3x HVF and 1x TCG, each `exit=0 pass=1 fail=0 done=1`, with 17 forks per boot.
- Red proof: on `kernel-virt` (no devcap) the gate exits 1, failing with `cpuprobe: FAIL caphash-open: namec: unknown # device` / `cpurc: FAIL cap`.
- The three existing gates pass with the fixture in the image (HVF: smoke 0, net-smoke 0, tier-1 0 with 44/23/0). Terminal boots never run cpurc.
- App count at the old pin is 4 (`boot-r9-cpu` is absent and evaluation does not error). At the new pin it is 5.

## Pin move (owner-batched)

- `fd80f6a70`: lock-only move `c94bc9b -> 821db85`. Between those revs only aarch64 and flake files changed, so there is no port/src or x86_64 byte change, no boot-iso re-derive and no x64 container run.
- Quick pass, no override, `KERNDATE=1795366613`. Every run exited 0:
  - tier-1 on HVF and TCG: 44/23/0 each
  - smoke and net-smoke on HVF
  - `boot-r9-cpu` on HVF and TCG: 17 forks each
- Recorded `SERIES-AT-PIN: deferred-to-phase-end (owner)`.

## Deviations from Plan

### Method notes

**1. [Rule 3 - Blocking] The Task 1 scratch cpurc also ran `/arm64/bin/stacksmoke`.** A cpu boot never runs termrc, so the smoke driver would have waited out its 600 s stacksmoke stage on every boot. Running the real binary let the driver go on to work the post-cpurc rc, which gave direct evidence for `AFTER-CPURC`. The smoke app's exit 0 on those boots was incidental. Recorded in the evidence.

**2. Gate runs after the first full build used `R9_SKIP_BUILD=1`.** The Task 3 smoke run rebuilt the userland after all the source edits. The later net-smoke, tier-1 and cpu-gate runs reused it, because no source changed in between. The plan's verify loops call the apps without the hatch.

**3. The red proof ran outside `nix develop`.** I copied the app's own wrapper script out of the store and changed only its `R9_KERNEL_PKG` line, to the `kernel-virt-821db85` store path. The venue rule forbids baking inside the devshell, and this gives the same "script outside the app wrapper" proof. The first attempt silently ran the unmodified script: the store copy was read-only, so the edit failed. That run passed on `kernel-virt-cpu` and is recorded only as a fifth green run. The corrected run is red at `cap`.

**4. The first cpu gate run happened before the Task 3 commit.** I wanted cpuprobe's errstr checks seen live before committing the fixture. The Task 3 and Task 4 commits are still separate, with exactly the plan's paths. Before the Task 3 commit, `nix/cpu-r9.sh` and `cpu-r9.expect` were staged so the flake could see them, and they were committed only in Task 4 (`git commit -- <paths>`).

**5. The pin move and quick pass are owner-directed additions** (the coordinator relayed them at the Task 2 answer). This plan's prohibition "no at-pin N series" still holds: only the quick pass ran.

**6. The mode-755 check uses `find -perm 755`, not `ls -l`.** A plan9port `ls` could appear on PATH, while BSD and GNU find agree on `-perm`.

No auth gates. No r9 file edited (Task 1's scratch edit was reverted). The 11 troff and postscript font files stayed unstaged.

## Known Stubs

None.

## Threat Flags

None. T-20-43 mitigated: every marker must appear exactly once, `cpurc: FAIL` fails the run at once, the red proof is recorded, and cpuprobe checks the errstr text. T-20-50 mitigated: Task 1 measured before any fixture existed. T-20-SC: no package added, and `cpuR9Pkg` reuses `smokeR9Pkg`'s dependency list.

## Still owed (not this plan)

- **Harvey push** of the local commits from 20-15 and 20-16 (through `f98f2c9e9`, plus this plan's metadata commit) needs owner approval. This is the closing checkpoint.
- The full HVF series at phase end, with `boot-r9-cpu` included. 20-17 derives `N-HVF boot-r9-cpu` from `CPU-FORKS-PER-BOOT = 17` and P-LO, with a floor of 36.
- 20-17 Task 1: the aarch64 host-test battery (HOST-TEST-ROUTE A).

## Self-Check: PASSED

- FOUND: build/aarch64/root-tree/rc/bin/cpurc, build/aarch64/root-tree/lib/shebangecho, build/aarch64/cpuprobe.c, nix/cpu-r9.sh, build/aarch64/cpu-r9.expect
- FOUND commits: b64926c3e, ba43f574e, 2deb0cc9e, fd80f6a70, f98f2c9e9
