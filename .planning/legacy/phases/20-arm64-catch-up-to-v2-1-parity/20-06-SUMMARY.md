---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 06
subsystem: r9 pin + arm64 composed gates (HVF/TCG) + r9 aarch64 vm::switch
tags: [r9, pin, flake-lock, hvf, tcg, vm-switch, dsb-ishst, isb, windows-49, boot-iso, fod, reproducible-build]
status: complete
requires: ["20-03", "20-04", "20-05"]
provides:
  - "r9 origin/main 57e2952 carries group A (20-01, 20-03, 20-04, the owner's ff2b632 FOD re-wire and 2226cc4 path remap, cec2299, and this plan's vm::switch dsb ishst hardening)"
  - "Harvey r9 pin at 57e2952, lock-only (0dafd07ad), pushed; origin/master = 0dafd07ad"
  - "at-pin series on the pinned kernel (sha256 da4ab2da...): HVF smoke 1753/1753, net-smoke 36/36, tier-1 10/10; TCG 3/3 each"
  - "reference kernel-virt sha256 for byte-comparing later aarch64-neutral pins (r9 kernel-virt is reproducible since 2226cc4)"
affects: [20-07, 20-11, 20-16, 20-17]
tech-stack:
  added: []
  patterns:
    - "check an aarch64-neutral pin change by kernel-virt ELF sha256 before deciding whether to re-run the series"
    - "rev-pinned git+file override (?rev=) so concurrent uncommitted r9 edits cannot leak into a gate series"
key-files:
  created:
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-06-SUMMARY.md
  modified:
    - flake.lock
    - flake.nix
    - .planning/WINDOWS.md
    - .planning/STATE.md
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
    - ../r9/aarch64/src/vm.rs
key-decisions:
  - "Owner: push r9 with the boot-iso FOD presumed stale; the owner re-wired it on the linux host (r9 ff2b632)"
  - "Owner: waive the at-pin re-run on ff2b632 (its diff was the FOD line only); the kernel-virt build was then made reproducible (r9 2226cc4) and the full series re-run on it"
  - "Owner decision A: the 2226cc4 net-smoke HVF fire (fork-child user fault on a corrupt rc argv word) reopens WINDOWS #49; harden vm::switch with dsb ishst (r9 57e2952)"
  - "Owner decision B: accept the 57e2952 series at the plan's N, pin it, keep WINDOWS #49 open pending ~2,300 clean net-smoke HVF boots"
requirements-completed: [ARM-03, ARM-05]
metrics:
  duration: "about 25 h wall clock (2026-11-18T12:44:33Z to 2026-11-19T13:50Z); about 16.8 h is four at-pin series blocks, the rest is owner checkpoints and build/compare work"
  completed: 2026-11-19
actuals:
  tokens: 9400
  tasks: 3
  commits: 23
plan_head_before: 5fa4c0f691693976fc27f5d08aca2b407ce7b1fb
plan_head_after: 0dafd07adb2026169583d56872bb12117f6fd804
r9_head_before: 5f3ddb0
r9_head_after: 57e2952
r9_commits: 1
---

# Phase 20 Plan 06: group A pushed and pinned at r9 57e2952 — at-pin HVF series green at N, WINDOWS #49 hardened but kept open Summary

r9 group A went to `origin/main`, and the Harvey pin moved lock-only four times: `8e4e730 → 5f3ddb0 → ff2b632 → 2226cc4 → 57e2952`. Each pinned kernel had its own at-pin series, or a documented owner decision instead. Two outside changes and one red series shaped the path:

- The bitmapalloc `port/src` fix made the x86_64 `boot-iso` FOD stale. The owner re-wired it at `ff2b632`.
- r9's kernel-virt build was not reproducible: the sandbox path was embedded in the binary. The owner fixed that at `2226cc4`.
- At `2226cc4`, net-smoke HVF went red at run 21. A fork child faulted on a corrupt rc argv word, the WINDOWS #49 class. That reopened #49. This plan added the `dsb ishst` hardening to `vm::switch` (r9 `57e2952`), and its series was green at the plan's N:

```
SERIES boot-r9-tier1 accel=hvf runs=10 pass=10 fail=0 pushword=0 forks=3280
SERIES boot-r9-net-smoke accel=hvf runs=36 pass=36 fail=0 pushword=0 forks=2560
SERIES boot-r9-smoke accel=hvf runs=1753 pass=1753 fail=0 pushword=0 forks=14024
SERIES boot-r9-tier1 accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=984
SERIES boot-r9-net-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=213
SERIES boot-r9-smoke accel=tcg runs=3 pass=3 fail=0 pushword=0 forks=24
```

The owner accepted the result and the pin (decision b). #49 stays open. The isb-only kernels' residual was about 1.8e-5 per fork, so 17,864 clean forks cannot separate the hardening from it. About 2,300 clean net-smoke HVF boots would.

`commits: 23` is the measured count `5fa4c0f69..0dafd07ad`. 13 of those commits are this plan's. Two are the orchestrator's `deferred-items.md` notes (`5dbb0bfc7`, `68ac71155`). Eight are outside work that landed on `master` while the plan ran (`b41e7129a`, `fb6498f8b`, `ecc5a7659`, `cecf1bfca`, `261553217`, `b0e3cef8f`, `0d01baa43`, `17df85ebc`).

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | r9 checks before the push, and the r9 push | Harvey `1748a29de`, `b1bc73213`; r9 `46364d0..5f3ddb0` pushed (owner-approved) | 20-EVIDENCE.md |
| 2 | Pin 8e4e730 → 5f3ddb0 (lock-only), at-pin series, records | `980f09c4e` (flake.lock only), `645963ed8` | flake.lock, flake.nix (comments only), WINDOWS.md, STATE.md, 20-EVIDENCE.md |
| 3 | Owner checks: boot-iso re-wire and r9 CI; pin → ff2b632; re-run waived | `38ec00ad7` (flake.lock only), `5661ee6e3`, `e2354a2d2` | flake.lock, 20-EVIDENCE.md |
| 3 | Pin → 2226cc4 (reproducible kernel), kernel identity, series (net-smoke red) | `adcd47cd3` (flake.lock only), `51d0502a4`, `fd80a5691` | flake.lock, 20-EVIDENCE.md |
| 3 | Decision A: reopen #49, harden vm::switch, series | `a334e1edc`, r9 `57e2952`, `658f7faee` | WINDOWS.md, STATE.md, r9 aarch64/src/vm.rs, 20-EVIDENCE.md |
| 3 | Decision B: pin → 57e2952, Harvey push, records | `0dafd07ad` (flake.lock only), this SUMMARY's commit | flake.lock, 20-EVIDENCE.md, WINDOWS.md, STATE.md |

## Evidence

The full record is in `20-EVIDENCE.md` under `## Group A at the pin`.

- **r9 checks before the push, at 5f3ddb0:** check, clippy, test, the aarch64 virtshell and default builds, the x86-64 and riscv64 builds, and ureg-check all exited 0. The plan's verify printed `R9-BATTERY-GREEN` and, after the push, `PUSHED-CLEAN`.
- **`port/src` flag:** `port/src/bitmapalloc.rs` (+35/-1) was flagged in capitals, so the Harvey push was held (`HARVEY-PUSH: pending-fod`). The owner's linux-host `nix build .#boot-iso` mismatched as expected and was re-wired at r9 `ff2b632` (the `outputHash` line only). It still holds at `2226cc4` (`--rebuild` clean).
- **Hosted CI:** r9 was green on `ff2b632` for every job, including `build-aarch64-virtshell` (owner report). Harvey hosted CI at the final push has not been observed yet.
- **Pin commits:** each of `980f09c4e`, `38ec00ad7`, `adcd47cd3` and `0dafd07ad` contains exactly `flake.lock`, and each locked rev equalled r9 `origin/main` at the time.
- **Series history:**

  | Pin | Kernel | Result |
  |-----|--------|--------|
  | 5f3ddb0 | `684a280w…` (not reproducible) | all six green (net-smoke 36/36) |
  | ff2b632 | `fv5wyb0b…` | tier-1 10/10, net-smoke 12 clean, then stopped (owner waiver) |
  | 2226cc4 | `9hm0innc…`, sha256 `d3537304…` | net-smoke HVF **red at run 21**; the other five green |
  | 57e2952 | `qc1i4nhl…`, sha256 `da4ab2da…` | all six green at N (the lines above) |

- **The 2226cc4 fire:** `netip-loopback.rc` fork child pid 49 hit `user fault: unresolvable … addr=0x1 pc=0x20a24`. The pc is in rc's `strncmp`, called from `searchpath`, with a junk argv word. It came right after the parent reaped a child.
- **Not registered:** 0 lines in every capture of every series.
- **Reproducibility:** `nix build --rebuild` of the 5f3ddb0 derivation reported `may not be deterministic`, because the sandbox path was embedded 41 times. From `2226cc4` on, `--rebuild` is clean and no sandbox path is left. The pinned `57e2952` kernel is byte-identical to the series kernel (same store path, same ELF sha256).
- **Unstaged files:** the 11 font/postscript ` M sys/…` entries are unstaged and untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The ff2b632 kernel could not be byte-compared with 5f3ddb0's**
- **Found during:** Task 3 (the orchestrator's identical-bytes shortcut)
- **Issue:** the kernel-virt build was not deterministic, and two builds of the same derivation differed. So the "identical bytes, skip the re-run" rule could not be applied.
- **Fix:** recorded the cause (the sandbox path in panic-location strings and module hashes) and started the re-run as the rule required. The owner then waived the re-run and fixed reproducibility at r9 `2226cc4`.
- **Commit:** `5661ee6e3`, `e2354a2d2`

**2. [Rule 3 - Blocking] Tier-1 HVF run 1 of the 57e2952 block did not boot**
- **Found during:** the hardened series
- **Issue:** the lima guest's nix fetch failed on `sys/lib/postscript/troff/Hx`. That file collides with `HX` on darwin's case-insensitive disk, the same collision behind the 11 pre-existing font entries. No fork ran.
- **Fix:** re-ran tier-1 HVF ×10 on the same image and the same rev-pinned override: 10/10 green. The failed attempt is recorded and not counted. The venue collision itself is left for the owner.
- **Commit:** `658f7faee`

### Owner-directed changes (checkpoints)

**3. Three extra pin advances (5f3ddb0 → ff2b632 → 2226cc4 → 57e2952).** The plan expected one pin. Each extra pin was owner-directed, for the FOD re-wire, the reproducible build and the #49 hardening. Each is lock-only and documented.

**4. WINDOWS #49 is open, not re-closed.** The plan's truth "WINDOWS #49 is reopened and re-closed" holds for `645963ed8`, but the 2226cc4 fire reopened it (decision A). It stays open with the residual-rate note and the long-measurement follow-up (decision b).

**5. r9 code edit in this plan.** `57e2952` (`aarch64/src/vm.rs`, +17/-3) was not in `files_modified`. It was directed by the owner (option A, the 20-03 hardening) and committed locally. The push was not made by this plan; it was presumably the owner's.

### Notes

- **Outside commits:** the coordinator, on the owner's instructions, pushed r9 up to `cec2299` and made Harvey `b0e3cef8f` (pin → `cec2299`) and `17df85ebc` (the debug doc). The coordinator made `0d01baa43` and the owner pushed it. The orchestrator added `b41e7129a` and the `deferred-items.md` notes. None of these was reverted or amended.
- **gsd windows ledger:** `.planning/WINDOWS.md` frontmatter counts were already inconsistent before this plan (52/3/13 against entries 49/5/14). The #49 reopen moved them by a delta (open +1, fixed -1) rather than recounting.
- `graphify update .` was run after the Task 2 records commit. `graphify-out/` is gitignored.

## Known Stubs

None.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: isolation | r9:aarch64/src/vm.rs | WINDOWS #49 is a cross-process memory exposure: a fork child reading its parent's state. It is hardened at 57e2952 but not yet shown closed. About 2,300 clean net-smoke HVF boots are the close criterion. |

## Self-Check: PASSED

- FOUND: `flake.lock` (rev `57e295242484237464a93a665577899e3aa4bc50`), `flake.nix`, `.planning/WINDOWS.md` (row 49 `open`, table and JSON), `.planning/STATE.md`, `20-EVIDENCE.md` (`## Group A at the pin`), this SUMMARY; r9 `aarch64/src/vm.rs`
- FOUND (ancestors of HEAD): `1748a29de`, `b1bc73213`, `980f09c4e`, `645963ed8`, `38ec00ad7`, `5661ee6e3`, `e2354a2d2`, `adcd47cd3`, `51d0502a4`, `fd80a5691`, `a334e1edc`, `658f7faee`, `0dafd07ad`; r9 `57e2952` is an ancestor of r9 `origin/main`
- Task 2 verify: the pin commit file list is `flake.lock `; smoke N=1753, net-smoke N=36 and tier-1 N=10 each have hvf≥1 and tcg≥1 in the section; one `HARVEY-PUSH: done` line; `HARVEY-PUSHED` (origin/master = HEAD before this commit); 11 ` M sys/` entries
