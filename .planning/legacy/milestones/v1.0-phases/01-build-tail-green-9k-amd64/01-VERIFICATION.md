---
phase: 01-build-tail-green-9k-amd64
verified: 2026-10-31T13:20:00Z
status: passed
score: 13/13 must-haves verified
covered_files:
  - .planning/phases/01-build-tail-green-9k-amd64/01-01-PLAN.md
  - .planning/phases/01-build-tail-green-9k-amd64/01-01-SUMMARY.md
  - .planning/phases/01-build-tail-green-9k-amd64/01-02-PLAN.md
  - .planning/phases/01-build-tail-green-9k-amd64/01-02-SUMMARY.md
  - .planning/phases/01-build-tail-green-9k-amd64/01-03-PLAN.md
  - .planning/phases/01-build-tail-green-9k-amd64/01-03-SUMMARY.md
  - .planning/phases/01-build-tail-green-9k-amd64/01-04-PLAN.md
  - .planning/phases/01-build-tail-green-9k-amd64/01-04-SUMMARY.md
  - .gitignore
  - build/REPRODUCIBILITY.md
  - build/mkdirs
  - build/mm/boot-9k10cpu.mm
  - build/mm/boot-9k10cpu.sh
  - build/scripts/build
  - build/scripts/linux-build
  - dist/replica/plan9.proto
  - nix/build.sh
  - nix/toolchain.sh
  - nix/vm-build.sh
  - sys/src/9k/k10/mkfile
  - sys/src/9k/mk/portmkfile
  - sys/src/BUILD_SETUP_NAMESPACE
  - sys/src/cmd/6c/cgen.c
  - sys/src/cmd/6c/swt.c
  - sys/src/cmd/6l/span.c
  - sys/src/cmd/cc/cc.h
  - sys/src/cmd/jpg/mkfile
  - sys/src/cmd/jtagfs/mkfile
  - sys/src/cmd/mkfile
  - sys/src/cmd/spell/pcode.c
  - sys/src/nboot/efi/mkfile
covered_digest: "v2:sha256:d137cbabfb8973055e549323e6aa504d20f7b7cc44b9cf375973baa554a7462f"
behavior_unverified: 0
overrides_applied: 0
decision_coverage:
  honored: 6
  total: 6
  not_honored: []
---

# Phase 01: Build-Tail Green (9k amd64) Verification Report

**Phase Goal:** The Harvey namespace build is a trustworthy factory — a clean checkout produces the full 9k amd64 kernel set with zero manual intervention, and the tree speaks `aarch64` only.
**Verified:** 2026-10-31T13:20:00Z
**Status:** passed
**Re-verification:** No — initial verification

**Verification posture:** Host-side assertions (greps, file existence/content, git archaeology, bash syntax, scratch runs, derivation-logic behavioral checks) plus direct inspection of the phase's retained on-disk evidence: the second clean-checkout clone (retained at `/private/var/folders/.../T/opencode/harvey-clean-checkout`, commit `f254c35c6`), five build/demo logs in `/tmp`, and the accepted `boot-9k10cpu.log`. Guest builds were not re-run — the recorded evidence was inspected and found internally consistent (artifact byte sizes match SUMMARY figures exactly; logs end in exit 0; hash pairs and negative control match the quoted values), per the environment-notes policy.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `nix run .#build` from a clean checkout completes the full namespace build in one run, strict DIRS abort semantics unchanged (SC1, 01-01) | ✓ VERIFIED | Retained clean clone @ `f254c35c6` on disk with all build outputs; `/tmp/01-01-clean-build2.log` ends `SEQUENCE_EXIT=0` (plus first run log `/tmp/01-01-cleanclone.log`); `git diff` shows `dirs:V:` loop byte-identical (only the `checkall` block added) |
| 2 | `amd64/9k10cpu`, `amd64/9k10cpuf`, `amd64/9k10f` + `.elf` variants all produced by that single run (SC1, BUILD-01) | ✓ VERIFIED | All six files present in the working tree AND in the retained clean clone; clone byte sizes (2073033 / 3502788 / 3315571 + .elf) match 01-01-SUMMARY exactly |
| 3 | `/amd64/bin` carries executable ip/ipconfig, auth/factotum, fossil/fossil, venti/venti after the build (SC2, BUILD-02) | ✓ VERIFIED | `test -x` passes on all four in the working tree and in the retained clean clone; `amd64/.userland-ok` sentinel armed in both |
| 4 | `mk checkall` visits every dir in `cc $DIRS` and prints the full failure list instead of aborting on the first failure (D-03) | ✓ VERIFIED | `sys/src/cmd/mkfile:44-58` carries `checkall:V:` with `fail=()` accumulator, brace group, `checkall failures:` report, `exit 0` on green; recorded live run 145/145 dirs exit 0 with failure-capture A/B (spell surfaced mid-list, loop continued) |
| 5 | No live legacy-arm64 vestiges: zero matches in `build/`, `sys/src/BUILD_SETUP_NAMESPACE`, `dist/replica/plan9.proto`; root `arm64/` absent (SC3, BUILD-03) | ✓ VERIFIED | `grep -rln arm64` over the gated set exits 1; `test ! -d arm64` passes; scratch-tree `mkdirs` run creates no `arm64` dir; keeper strings (nix uname mapping, lima yaml URL, `a.out.h`/`mach.h`, `dynld-arm64.c`) intact |
| 6 | Namespace build still enters successfully after the quarantine (Pitfall 3 coupling, one commit) | ✓ VERIFIED | Quarantine landed as single commit `5af5e89bf` (5 files + dir removal, keeper list in message); `/tmp/01-02-postquarantine-build.log` completes with artifacts synced; later 01-04 green run corroborates |
| 7 | `build/scripts/linux-build` builds amd64 only by default (386 pass + dist/man gated, not deleted) | ✓ VERIFIED | Line 133 `arch=(amd64)` with revert comment; dist/man wrapped in `if(~ $WITH_DIST 1){` at line 161; zero `arch=(386 amd64)` literals; `exit build` failure idiom intact at line 156 |
| 8 | Kernel compile embeds pinned KERNDATE derived from git commit timestamp by default; env var overrides (BUILD-04) | ✓ VERIFIED | Identical `if(~ $#KERNDATE 0)` idiom at `sys/src/9k/k10/mkfile:20` and `sys/src/9k/mk/portmkfile:103` (exactly one in-idiom `date -n` per file); host-side derivation in `nix/build.sh:17-28` and `nix/vm-build.sh:23-32`, ssh pass-through at `vm-build.sh:58`; behavioral spot-check: unset → 1793452253 = `git log -1 --format=%ct` of HEAD; set → preserved |
| 9 | Two same-checkout-path kernel rebuilds with same KERNDATE produce bit-identical sha256 for 9k10cpu artifacts | ✓ VERIFIED | `/tmp/01-03-kerndate-demo.log`: two runs both `dea0cd7a…`/`93c25582…`; control log: KERNDATE=1700000001 → `fcba8b7a…` DIFFERS; restore → MATCHES |
| 10 | `build/REPRODUCIBILITY.md` documents scope, derivation, override, conffile exception, verification (BUILD-04) | ✓ VERIFIED | All five sections present; scope kernel-only ("userland … NOT claimed"); exceptions name conffile same-path and no-git wall-clock fallback |
| 11 | `nix build .#minimega` produces a working minimega binary on linux (BUILD-05) | ✓ VERIFIED | `nix/minimega.nix` committed (fetchFromGitHub tag 3.3.0 hash-pinned, buildGoModule, vendored deps) and wired into `flake.nix` packages; recorded guest build exit 0; `/nix/store/…-minimega-3.3.0/bin` visible in the demo log's realized devshell PATH |
| 12 | `build/mm/boot-9k10cpu.mm` boots the Phase-1 `amd64/9k10cpu.elf` under minimega; boot asserted by `Plan 9` banner in the serial-socket capture (BUILD-05) | ✓ VERIFIED | `.mm` carries the six vm commands with KERNEL_ELF placeholder; wrapper splits at `vm start` and attaches socat before start; `boot-9k10cpu.log` (4003 B) on disk contains `Plan 9` at line 4 plus MMU/APIC/PCI/memory init; TCG mode explicitly accepted by the human at the blocking checkpoint 2026-10-31 |
| 13 | The harness documents the BUILD-05 boundary: minimega manages amd64/KVM VMs only; aarch64/r9 boots stay on plain qemu | ✓ VERIFIED | Boundary comment in both `boot-9k10cpu.mm` (header) and `boot-9k10cpu.sh` (line 37); TCG fallback documented in-file |

**Score:** 13/13 truths verified (0 present-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `sys/src/cmd/jpg/mkfile` | generators on awk host-tool idiom | ✓ VERIFIED | `objtype=$cputype mk rgbycc.$cputype` / `rgbrgbv.$cputype` recursion; no `$c^c` derivation; `nuke-headers` removes headers + host binaries |
| `sys/src/cmd/jtagfs/mkfile` | generator on awk idiom | ✓ VERIFIED | `objtype=$cputype mk genrtab.$cputype`; no `Oo=` derivation; CLEANFILES covers `*.genrtab` + `rtab.c` |
| `sys/src/cmd/mkfile` | new checkall target | ✓ VERIFIED | `checkall:V:` with accumulator + report; `dirs:V:` byte-identical per git diff |
| `nix/build.sh` | preflight + KERNDATE export | ✓ VERIFIED | Sentinel-armed preflight lines 58-66 (arming 85-89); KERNDATE block 14-28 |
| `nix/vm-build.sh` | KERNDATE host-side, ssh pass-through | ✓ VERIFIED | Derivation 23-32 precedes rsync/ssh; `KERNDATE=$KERNDATE` prefix on remote command line 58; rsync `--exclude .git` line 52 |
| `nix/toolchain.sh` | wrapper-PATH 9cc fix | ✓ VERIFIED | In tree; referenced by commit `d7c256cc2` |
| `build/scripts/build` | fn build_aarch64 dormant | ✓ VERIFIED | Line 76, `objtype=aarch64`, `cmd/7?` glob intact |
| `build/scripts/linux-build` | fn build_aarch64 + narrowed scope | ✓ VERIFIED | Lines 63, 133, 161; bash -n clean |
| `build/mkdirs` | no legacy-arch entry | ✓ VERIFIED | bash -n clean; scratch run creates no `arm64` dir |
| `sys/src/BUILD_SETUP_NAMESPACE` | legacy bind-mount stanza dropped | ✓ VERIFIED | bash -n clean; zero `arm64` matches |
| `dist/replica/plan9.proto` | aarch64 stanza | ✓ VERIFIED | Line 38 `aarch64 d775 sys sys` |
| `sys/src/9k/k10/mkfile` | KERNDATE env idiom | ✓ VERIFIED | Line 20; exactly one `date -n` (in-idiom fallback) |
| `sys/src/9k/mk/portmkfile` | KERNDATE env idiom | ✓ VERIFIED | Line 103; identical idiom |
| `build/REPRODUCIBILITY.md` | policy doc | ✓ VERIFIED | All five required sections |
| `build/mm/boot-9k10cpu.mm` | minimega command script | ✓ VERIFIED | Six vm commands, KERNEL_ELF placeholder, boundary + TCG comments |
| `build/mm/boot-9k10cpu.sh` | capture/assert wrapper | ✓ VERIFIED | bash -n clean; KVM probe, MM_TCG/MM_QEMU/MM_BOOT_BUDGET/MM_LOG, split-mm ordering, banner grep, kill/flush cleanup — see CR-01/WR-03 in Anti-Patterns |
| `sys/src/cmd/cc/cc.h` | NVAR LP64 fix | ✓ VERIFIED | In tree (commit `735eadf3d`); review verified `NVAR=BITS*32` matches bits.c indexing |
| `sys/src/cmd/6c/{swt,cgen}.c`, `sys/src/cmd/6l/span.c` | LP64 truncation fixes | ✓ VERIFIED | In tree (commit `a63a6a90a`); review traced all three sites as correct |
| `sys/src/cmd/spell/pcode.c` | const-correct wcmp | ✓ VERIFIED | In tree (commit `5c78cae6e`) with D-02 toolchain-first diagnosis in message |
| `sys/src/nboot/efi/mkfile` | efiboot.fat explicit-only | ✓ VERIFIED | `efiboot.fat:D:` target out of TARG (test.iso/test.fat precedent) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| jpg/jtagfs generator recipes | host toolchains | `objtype=$cputype mk <tool>.$cputype` → `/unix/mkfile` resolves CC=9c LD=9l | ✓ WIRED | Idiom present in both mkfiles; green builds prove generators ran (jpg headers committed; guest-side `rtab.c` 1701 B recorded) |
| `nix/build.sh` preflight | kernel-critical binaries | sentinel-gated `[ -x ]` assertions after mkdirs, before RUN_BUILD_IN_PRIVATE_NAMESPACE | ✓ WIRED | Lines 48 → 58-66 → 81 ordering confirmed |
| Kernel rootdir embedding | `/amd64/bin` binaries | 9k/mk/parse → mkrootall → addbootfile at kernel link | ✓ WIRED | All three kernels (incl. k10cpuf/k10f embedding fossil+venti) link from clean checkout — the embedding found its inputs |
| BUILD_SETUP_NAMESPACE stanza removal | root `arm64/` dir removal | same commit | ✓ WIRED | Single commit `5af5e89bf`; post-quarantine build enters namespace green |
| `fn build_aarch64` | Phase 2 toolchain | `objtype=aarch64` + `/sys/src/cmd/7?` glob, dormant | ✓ WIRED (dormant by design, D-05) | Present in both build scripts |
| KERNDATE derivation | guest build | host-side compute before rsync (`.git` excluded), env prefix on ssh line | ✓ WIRED | vm-build.sh lines 23-32, 52, 58 |
| Serial capture | boot banner | socat attach BEFORE `vm start` via launch/start .mm split | ✓ WIRED | Wrapper lines 218-227; banner captured in practice (WR-03 records the missing synchronization hardening) |
| qemu multiboot | `amd64/9k10cpu.elf` | `vm config kernel` ← `.elf` variant | ✓ WIRED | Demonstrated boot; `.mm` header forbids the a.out |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| Kernel binaries | embedded root (factotum/ipconfig/fossil/venti bytes) | `/amd64/bin/*` build outputs via parse/mkrootall/addbootfile | Yes — kernels link only when inputs exist; strict abort semantics unchanged | ✓ FLOWING |
| KERNDATE | `-DKERNDATE=` compile flag | host-side `git log -1 --format=%ct` → env → mk recipe | Yes — behavioral spot-check returned HEAD's commit ts; negative control proves the pin moves the bytes | ✓ FLOWING |
| checkall report | `fail` list | per-dir `mk all` exit statuses | Yes — recorded failure-path A/B printed the real failing dir | ✓ FLOWING |
| boot assertion | `boot-9k10cpu.log` | minimega serial0 socket via socat | Yes — captured log on disk contains real boot output | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| bash syntax, all new/edited scripts | `bash -n nix/build.sh nix/vm-build.sh nix/toolchain.sh build/mm/boot-9k10cpu.sh build/mkdirs sys/src/BUILD_SETUP_NAMESPACE` | all clean | ✓ PASS |
| mkdirs no longer creates `arm64/` | scratch-tree run in mktemp dir | no arm64 dir | ✓ PASS |
| KERNDATE unset → git-derived | replay of build.sh derivation block | 1793452253 == `git log -1 --format=%ct` HEAD | ✓ PASS |
| KERNDATE set → preserved (override) | replay with KERNDATE=1700000000 | 1700000000 preserved | ✓ PASS |
| KERNDATE invalid → behavior | replay with KERNDATE=abc | silently wall-clock (reproduces open WR-01) | ✓ PASS (confirms recorded warning) |
| Legacy-token gate | `grep -rln arm64 build/ sys/src/BUILD_SETUP_NAMESPACE dist/replica/plan9.proto` | exit 1 | ✓ PASS |
| `mk checkall` live in namespace | — | SKIP host-side (needs namespace/plan9port); recorded live run 145/145 exit 0 + failure A/B inspected | ? SKIP (recorded evidence) |
| Pinned-kernel relink pair | — | SKIP host-side (needs namespace); demo + control logs inspected, hashes match SUMMARY | ? SKIP (recorded evidence) |
| `nix build .#minimega` | — | SKIP on darwin (linux-only per plan); recorded guest build + realized store path corroborate | ? SKIP (recorded evidence) |

### Probe Execution

N/A — no probes declared in any PLAN/SUMMARY and no `scripts/*/tests/probe-*.sh` exist in this repo.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| BUILD-01 | 01-01, 01-02 | Clean-checkout `nix run .#build` → all three 9k amd64 kernels | ✓ SATISFIED | Truths 1, 2, 6, 7 — retained clone + exit-0 logs + working-tree artifacts |
| BUILD-02 | 01-01 | `/amd64/bin` carries kernel-critical userland | ✓ SATISFIED | Truth 3 — `test -x` on all four, both trees |
| BUILD-03 | 01-02 | `arm64` vestiges quarantined to `aarch64` naming | ✓ SATISFIED | Truth 5 — grep gate, dir absence, single quarantine commit, keepers intact |
| BUILD-04 | 01-03 | KERNDATE/reproducibility policy decided + documented + honored | ✓ SATISFIED | Truths 8, 9, 10 — idiom, plumbing, policy doc, bit-identical demo with negative control |
| BUILD-05 | 01-04 | flake-packaged minimega + `.mm` boot harness with captured boot log | ✓ SATISFIED | Truths 11, 12, 13 — derivation wired, harness committed, banner captured, TCG mode human-accepted |

All five requirement IDs declared across the four plans ({01-01: BUILD-01,BUILD-02}, {01-02: BUILD-03,BUILD-01}, {01-03: BUILD-04}, {01-04: BUILD-05}) = {BUILD-01..05}; REQUIREMENTS.md maps exactly BUILD-01..05 to Phase 1. **No orphaned requirements.**

### Anti-Patterns Found

Debt-marker scan of all 22 phase-modified files: clean (two false positives — `mktemp …XXXXXX` template in `nix/toolchain.sh:26`; pre-existing compiler token constants `OXXX/TXXX/CXXX/GXXX` in `cc.h`). No stub patterns, no empty implementations, no console-log-only code. Commit structure is atomic per-blocker (19 summary-referenced commits all present in history).

**Open code-review findings (01-REVIEW.md, disposition: all OPEN) — assessed against the phase goal:**

| File | Finding | Severity | Impact Assessment |
|------|---------|----------|-------------------|
| `build/mm/boot-9k10cpu.sh:1` | CR-01: executable, no shebang, requires bash; documented `sudo build/mm/…` direct invocation fails on dash-sh systems | ⚠️ Warning (carried debt) | Confirmed present. Does NOT falsify a must-have: the harness ran via the documented `bash build/mm/…` invocation, the `.mm` script (BUILD-05's letter) is minimega input unaffected by the shell, and the boot was demonstrated + accepted. Debt to fix before first real KVM-host use. |
| `nix/build.sh:17-28`, `nix/vm-build.sh:23-32` | WR-01: invalid KERNDATE silently replaced by wall-clock | ⚠️ Warning | Confirmed present (reproduced in spot-check). Valid-value override works (verified); silent-degradation edge is debt. |
| `nix/build.sh:58-66,85-89` | WR-02: armed preflight can block the repair path without naming the escape (`rm amd64/.userland-ok`) | ⚠️ Warning | Confirmed present; sentinel design itself is plan-mandated (Task 1 NOTE). Recoverability-UX debt. |
| `build/mm/boot-9k10cpu.sh:221-227` | WR-03: socat attach unsynchronized before `vm start` | ⚠️ Warning | Confirmed present (no liveness check/settle). The ordering worked in the demonstration; race-hardening is debt. |
| `sys/src/cmd/mkfile:46-58` | WR-04: checkall `cd`s unchecked | ⚠️ Warning | Confirmed present. Live run visited 145/145 correctly; mis-attribution edge is debt. |
| IN-01..IN-05 | gitignore gaps, mkdirs shebang, KERNDATE duplication + empty-value edge, sed metachar escaping, dead `build_aarch64` guard | ℹ️ Info | Confirmed; all debt, none goal-blocking. |

None of the ten findings falsify a must-have truth or the phase goal; all are carried as recorded open debt in 01-REVIEW-DISPOSITION.md. (Deferred-items.md #1 dormant-arch LP64 audit and #2 post-banner pool panic are plan-sanctioned scope exclusions, tracked for Phase 2 — not gaps against Phase 1's contract.)

### Test Quality Audit

N/A — this phase ships build tooling verified by end-to-end builds, hash comparisons, and a captured boot log, not by a test suite. No requirement-linked test files exist to audit for disabled/circular/weak assertions. The phase's proofs are behavioral-level (exit-0 clean-clone builds, bit-identical hash pair with negative control, captured serial banner) — the strongest assertion class, directly inspected.

### Decision Coverage

All trackable CONTEXT.md decisions are honored by shipped artifacts. (6/6 — `check.decision-coverage-verify`, non-blocking gate)

### Human Verification Required

N/A — Infrastructure/foundation phase with no user-facing elements. All acceptance criteria were verifiable programmatically or via directly inspected recorded evidence. The phase's single human-judgment item (accepting TCG-degraded capture as BUILD-05's proof) was already exercised at 01-04's blocking human checkpoint on 2026-10-31 ("Accept TCG demonstration") — it is complete, not outstanding. No behavior-unverified or insufficient-spec truths required abstention.

---

_Verified: 2026-10-31T13:20:00Z_
_Verifier: the agent (gsd-verifier)_
