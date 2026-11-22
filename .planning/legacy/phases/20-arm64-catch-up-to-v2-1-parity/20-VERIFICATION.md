---
phase: 20-arm64-catch-up-to-v2-1-parity
verified: 2026-11-22T18:45:58Z
status: passed
score: 13/13 must-haves verified (11 VERIFIED, 2 PASSED by override)
covered_files: [".github/workflows/harvey.yml", ".github/workflows/phase6.yml", ".github/workflows/r9-shell.yml", ".planning/REQUIREMENTS.md", ".planning/STATE.md", ".planning/WINDOWS.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-01-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-01-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-02-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-02-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-03-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-03-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-04-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-04-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-05-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-05-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-06-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-06-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-07-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-07-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-08-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-08-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-09-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-09-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-10-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-10-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-11-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-11-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-12-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-12-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-13-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-13-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-14-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-14-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-15-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-15-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-16-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-16-SUMMARY.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-17-PLAN.md", ".planning/phases/20-arm64-catch-up-to-v2-1-parity/20-17-SUMMARY.md", "build/aarch64/README.md", "build/aarch64/brkprobe.c", "build/aarch64/cexecprobe.c", "build/aarch64/churnmark.c", "build/aarch64/churnsem.c", "build/aarch64/cpu-r9.expect", "build/aarch64/cpuprobe.c", "build/aarch64/donor-vm-build.sh", "build/aarch64/forkstress.c", "build/aarch64/fpprobe.c", "build/aarch64/fpprobeasm.s", "build/aarch64/mkimage.sh", "build/aarch64/rcloseprobe.c", "build/aarch64/repeat-gate.sh", "build/aarch64/rfdprobe.c", "build/aarch64/root-tree/lib/shebangecho", "build/aarch64/root-tree/lib/writeprobe.rc", "build/aarch64/root-tree/rc/bin/cpurc", "build/aarch64/smoke-r9-net.expect", "build/aarch64/smoke-r9.expect", "build/aarch64/sslprobe.c", "build/aarch64/stage-build.sh", "build/aarch64/subset-build.sh", "build/aarch64/tier1-r9.expect", "build/aarch64/tier1-sweep.rc", "build/harness/src/trace.rs", "build/x86_64/fpprobe.c", "build/x86_64/fpprobeasm.s", "build/x86_64/mkimage.sh", "build/x86_64/stage-build.sh", "build/x86_64/tier1-sweep.rc", "flake.lock", "flake.nix", "nix/boot-r9.sh", "nix/build.sh", "nix/cpu-r9.sh", "nix/smoke-r9.sh", "nix/tier1-r9-x64.sh", "nix/tier1-r9.sh"]
covered_digest: "v4:sha256:7b6f63f4c53062f8001fead25785e12c1221d7b7b9d3e9f63f0b896fe5e6b2c0"
behavior_unverified: 0
overrides_applied: 2
overrides:
  - must_have: "ARM-03: boot-r9-smoke, boot-r9-net-smoke and boot-r9-tier1 are reliably green under HVF at the pin, each gate green N consecutive runs, WINDOWS #49 root-caused and fixed"
    reason: "The full N series was run green at the group A pin (57e2952) and the group B pin (fa38402): 1808/1808 each time, 0 pushword. At the final pin e6e5220 the owner stopped the series and deferred it to the upcoming SMP work. The partial series (tier-1 HVF 10/10, net-smoke HVF 12/12) and the confidence pass were green. WINDOWS #49 is root-caused (TTBR isb, then the I-cache residual fixed in r9 79c55b1 with the 46a5c87 guard, both in e6e5220) but stays open until the full series runs. The owed series is recorded with its N values: smoke 1753, net-smoke 36, tier-1 10, cpu 36."
    accepted_by: "Neil Skinner (owner)"
    accepted_at: "2026-11-22T17:41:00Z"
  - must_have: "ARM-08: the #! exec pre-pass recursion cap (MAX_SHEBANG_DEPTH) is pinned"
    reason: "Known gap, recorded in 20-EVIDENCE.md under Host-test battery, in the 20-17 SUMMARY Known gaps, and in STATE.md. The x86_64 pin needs new_user_proc, which cannot run on the aarch64 host (pagealloc OutOfSpace), and no gate makes a third #! hop. The cap code is ported (exec.rs:76, :421-428). The owner accepted the parity line, including the list of pins not ported, at the 20-17 Task 4 approval."
    accepted_by: "Neil Skinner (owner)"
    accepted_at: "2026-11-22T18:40:00Z"
human_verification: []
---

# Phase 20: arm64 catch-up to v2.1 parity — Verification Report

**Phase goal:** Bring the r9 aarch64 kernel and Harvey arm64 userspace back to parity with the v2.1 x86_64 work, so that `nix run .#boot-r9-smoke` is green at the pin. This is the first end-to-end proof of the objtype rename aarch64 -> arm64.
**Verified:** 2026-11-22T18:45:58Z, at Harvey `f7bd7c156` (= origin/master) and r9 `e6e5220` (= origin/main = the flake.lock pin)
**Status:** passed, with record warnings (none of them blocks the goal)
**Re-verification:** No. This is the initial verification.

ROADMAP gives no separate Success Criteria list for phase 20. The contract here is the goal sentence plus ARM-01..ARM-12.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence (verifier's own unless marked "recorded") |
|---|-------|--------|------|
| G | `nix run .#boot-r9-smoke` is green at the pin, with cputype=arm64 | ✓ VERIFIED | I ran it once on HVF with no override and `R9_SKIP_BUILD=1`. It booted `r9-kernel-virt-e6e5220`, exited 0 and printed `smoke-r9: ALL STAGES PASS` (including user-hooks-registered, no-unknown-device and no-unsupported-era). The capture has `devenv: seeded cputype=arm64`. |
| 1 | ARM-01: virtshell builds at HEAD; a CI lane compiles aarch64 dev_exec; `cargo xtask test` runs the port integration tests; the devShell evaluates on darwin | ✓ VERIFIED | `nix develop --command cargo xtask build --arch aarch64 --config virtshell` exits 0 at e6e5220. `nix develop` enters on darwin. `cargo xtask test` is green: port lib 168 plus 12 integration binaries (devenv_test 6 among them), aarch64 71 + 87. `xtask.yml` has the `build-aarch64-virtshell`, `-virtchurn`, `-virtcpu` and `test-aarch64-host` jobs. `xtask/src/main.rs:3459` adds `--tests`. |
| 2 | ARM-02: note_pending, the push_note ready arm and per-proc user hooks behave as on x86_64; no `hooks not registered` | ✓ VERIFIED | `main.rs:674` has `set_user_hooks`, and `:694`/`:725` set `note_pending` in PipeHooks/DevipHooks. `proc.rs:2302-2304` takes Blocked to Ready. All four arm64 expect drivers carry the `not registered` post-pass, and my smoke and cpu runs passed it. virtnotes boot-test green (recorded, 20-04). |
| 3 | ARM-03: the three gates are reliably green under HVF at the pin at the derived N; #49 root-caused and fixed; TCG is the control leg | PASSED (override) | Full N series green at 8958c9e, 57e2952, 7b598f2, 1125a20 and fa38402 (1808/1808 each, recorded). At e6e5220: partial series plus confidence pass green (recorded), and my own single HVF smoke run is green. The full series is owner-deferred to the SMP work, and #49 is left open. See the override. |
| 4 | ARM-04: rename proven on every venue: `cargo test -p port`, donor `subset-exec.sh` after re-provision, and the boot-chain trace with fversion diagnosed | ✓ VERIFIED | Port tests green (my run, devenv_test included). Donor gate 61 T1-OK (recorded at 20-05, re-run at 20-11 `c7cd176d2`). D-06 is Branch A: `frag_records` is in `build/harness/src/trace.rs` with unit tests (recorded, 72 passed). The sweep-stage launch race is deferred in deferred-items.md. |
| 5 | ARM-05: each group reaches Harvey push-before-pin as a lock-only flake.lock commit, and the gates are green at the pin with no override | ✓ VERIFIED | `97cb3c7bb` changes only `flake.lock`. Its locked rev `e6e522047a30…` equals r9 `origin/main`. Groups A and B did the same (`0dafd07ad` and `c7cd176d2`, recorded). My smoke and cpu runs at the pin used no override. |
| 6 | ARM-06: the aarch64 halves of the v2.1 syscall-path fixes are ported | ✓ VERIFIED (see the record warning W1) | rfork split: `syscall.rs:1998` and `proc.rs:2081`. devmnt write: `syscall.rs:355`. DMDIR and union creates with nomount: `syscall.rs:1771-1874`. Exit drains and husk sweep: `exit_tail` at `syscall.rs:2480/2489`, `proc.rs:2170`. fd-drain cascade (ab0a977's two halves): the one-at-a-time owning drain at `proc.rs:2047-2063` and the closepgrp mount half (`release_pgrp`/`take_mount_chans`) at `proc.rs:2147`. Close-on-exec: `exec.rs:381/497`, plus the create(OCEXEC) fix 437b752 (`syscall.rs` create_cexec_tests). Lone-member brk: measured absent (D-09, recorded). Host pins pass in my run, including `last_sharer_drain_closes_every_duplicate` and `last_holder_release_closes_the_mount_table`. |
| 7 | ARM-07: each ARM-06 port has a live arm64 pin: a writable venue in the tier-1 sweep, and a churn gate pinning the exit-leak fix | ✓ VERIFIED | `nix/tier1-r9.sh:221` requires `forkstress writeprobe cexecprobe cexeccreate rcloseprobe brkprobe fpprobe`. The probes are baked (subset-build.sh, mkimage.sh) and swept (tier1-sweep.rc). virtchurn has a CI lane, and its post-fix runs are 3/3 under the 1772 B/exec ceiling (recorded). Each new pin was shown red at the old pin (recorded). |
| 8 | ARM-08: the `#!` exec pre-pass and the REMOVE/WSTAT/FWSTAT arms with stat validation are ported | ✓ VERIFIED (the recursion-cap pin is covered by override) | `exec.rs:106` has `shebang_pass` and `:421` the depth loop. The dispatch slots are at `syscall.rs:158-162`, with `valid_stat_wire` at `:1131`. Host tests: 6 shebang and 8 stat_wire/routing, all passing. My live cpu run printed `SHEBANG-OK`, `STAT-OK` and `REMOVE-OK`. |
| 9 | ARM-09: devcap and devssl served on arm64 (arms, tick hook, fd-resolve hook, mount over `#D`); `check_cap_extents` ported | ✓ VERIFIED | DEV_CAP/DEV_SSL read/write arms at `syscall.rs:363-379, 488-491`. Mount accepts DEV_SSL at `syscall.rs:998`. `main.rs:682-683` has the `cfg(dev_cap)` tick hook and `:741` the fd-resolve hook. `config_virtcpu.toml` has dev `cap`. `check_cap_extents` is at `pci.rs:546` and called at `:629`, with 5 host tests. My live cpu run printed `CAP-OK` and `SSL-OK` (sslprobe -m mounts over ssl). |
| 10 | ARM-10: bootconf reaches the ROOT env seed through DTB `/chosen/bootargs`, merged and never replacing; an absent string leaves the boot identical | ✓ VERIFIED | `bootconf.rs:45/99/156`. `main.rs:795-803` does `seed.extend(bootconf.pairs())`, which merges after the compiled pairs. My cpu run has `devenv: bootconf init=/arm64/init -c` once. My smoke run, which has no bootargs, has 0 bootconf lines and the unchanged seed line. 6 parser host tests pass. |
| 11 | ARM-11: an arm64 fixture gate exercises bootconf, `#!`, REMOVE over devcap and devssl live, green on HVF at the pin | ✓ VERIFIED | I ran `nix run .#boot-r9-cpu` on HVF with no override: kernel `r9-kernel-virt-cpu-e6e5220`, exit 0, `cpu-r9: ALL STAGES PASS`. The seven markers appear once each, with 0 `cpurc: FAIL`. Harvey CI has a `cpu-smoke` job in `r9-shell.yml:80`. |
| 12 | ARM-12: an aarch64 host-test battery pins shebang, stat validation, the bootconf parser and cap extents, by the measured route | ✓ VERIFIED | My `cargo xtask test` run executed 6 shebang (both legs), 8 stat_wire/routing (both legs), 6 bootconf and 5 cap_extents tests (virtshell leg). All passed, with no `#[ignore]` in aarch64/src or port. The route-A fix is in `platform/mod.rs`. |

**Score:** 13/13. 11 are VERIFIED and 2 are PASSED (override). 0 are present but behavior-unverified.

### Required Artifacts

| Artifact | Status | Details |
|---|---|---|
| All `must_haves.artifacts` across the 17 plans | ✓ VERIFIED | `verify.artifacts` passed for every plan (3/3, 3/3, 2/2, …, 3/3). |
| r9 `aarch64/src/{exec,syscall,proc,main,vm}.rs`, `platform/virt/{bootconf,pci}.rs`, `aarch64/lib/config_virtcpu.toml` | ✓ VERIFIED | Substantive, and wired to their call sites (see the truths above). |
| Harvey `build/aarch64/{cpu-r9.expect,cpuprobe.c,sslprobe.c,cexecprobe.c,rcloseprobe.c,brkprobe.c,rfdprobe.c,forkstress.c,repeat-gate.sh}`, `root-tree/{rc/bin/cpurc,lib/shebangecho,lib/writeprobe.rc}`, `nix/cpu-r9.sh` | ✓ VERIFIED | Baked by subset-build.sh and mkimage.sh. Consumed by tier1-sweep.rc, tier1-r9.sh and cpu-r9.sh. `repeat-gate.sh` accepts `boot-r9-cpu`. |

### Key Link Verification

`verify.key-links` cannot check most of the declared links. Cross-repo `../r9/...` sources are rejected as "outside the project", and runtime relationships (bake, pin, sweep) report "Target not referenced". I checked these by hand:

| From | To | Status | Evidence |
|---|---|---|---|
| flake.lock | r9 origin/main | WIRED | Locked rev e6e5220 equals origin/main. My gates booted `-e6e5220` kernels. |
| subset-build.sh / mkimage.sh | probe binaries | WIRED | Each of the 7 probes is referenced 2-8 times in each script, with manifest asserts. |
| tier1-sweep.rc / tier1-r9.sh | probe markers | WIRED | Sweep entries exist for forkstress, writeprobe, cexecprobe, brkprobe and rcloseprobe. `R9_EXPECT_OK` requires them. |
| cpurc | cpuprobe / sslprobe / shebangecho / `#¤/caphash` | WIRED | The legs run live in my cpu run. |
| aarch64 main.rs | port hooks (user, note_pending, tick, fd-resolve) | WIRED | See truths 2 and 9. |
| exit_tail / sys_rfork | release_fd_group / sweep_husk_payloads / split_fd_group_in_place | WIRED | `syscall.rs:1998, 2480, 2489`. |
| xtask TestStep | aarch64 crate (default + virtshell cfg) | WIRED | `xtask/src/main.rs:3475-3496`. The legs ran in my run. |
| r9-shell.yml | `nix run .#boot-r9-cpu` | WIRED | `cpu-smoke` job, `r9-shell.yml:80-96`. |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|---|---|---|---|---|
| bootconf to ROOT env seed | `init=/arm64/init -c` | DTB `/chosen/bootargs` written by cpu-r9.sh's fdtput, read by `read_bootargs` | Yes: the live `devenv: bootconf` line, and boot execs `init -c`, so cpurc runs | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| r9 host tests (port + aarch64 battery) | `nix develop --command cargo xtask test` in r9 | exit 0; 14 `test result: ok` lines, 0 failed; aarch64 71 + 87 | ✓ PASS |
| virtshell builds at HEAD | `nix develop --command cargo xtask build --arch aarch64 --config virtshell` | exit 0 | ✓ PASS |
| Phase goal: smoke at the pin | `R9_SKIP_BUILD=1 R9_ACCEL=hvf nix run .#boot-r9-smoke` (no override) | exit 0, ALL STAGES PASS, `r9-kernel-virt-e6e5220` | ✓ PASS |
| cpu fixture at the pin | `R9_SKIP_BUILD=1 R9_ACCEL=hvf nix run .#boot-r9-cpu` (no override) | exit 0, ALL STAGES PASS, 7 markers once each, 0 FAIL | ✓ PASS |
| HVF N-series | not re-run, per the instruction and the owner's deferral | — | ? SKIP (owner-deferred, override) |

`R9_SKIP_BUILD=1` reuses the userland that the at-pin tier-1 run built (20-17). No Harvey source has changed since then. Both captures went to the session scratchpad. No tracked file changed, and flake.lock is unchanged.

### Probe Execution

The phase declares no `scripts/*/tests/probe-*.sh`. Step 7c is not applicable. The gate apps above serve as the phase's runnable checks.

### Requirements Coverage

| Req | Source plans | Status | Evidence |
|---|---|---|---|
| ARM-01 | 20-01, 20-04 | ✓ SATISFIED | Truth 1 |
| ARM-02 | 20-01 | ✓ SATISFIED | Truth 2 |
| ARM-03 | 20-02, 03, 06, 11, 17 | ✓ SATISFIED (override) | Truth 3 |
| ARM-04 | 20-01, 20-05 | ✓ SATISFIED | Truth 4 |
| ARM-05 | 20-06, 11, 17 | ✓ SATISFIED | Truth 5 |
| ARM-06 | 20-08, 20-10 | ✓ SATISFIED in code, but REQUIREMENTS.md still has it `[ ]` / Pending (W1) | Truth 6 |
| ARM-07 | 20-07..20-11 | ✓ SATISFIED | Truth 7 |
| ARM-08 | 20-14 | ✓ SATISFIED (the cap pin by override) | Truth 8 |
| ARM-09 | 20-13, 20-15 | ✓ SATISFIED | Truth 9 |
| ARM-10 | 20-13 | ✓ SATISFIED | Truth 10 |
| ARM-11 | 20-12, 20-16, 20-17 | ✓ SATISFIED | Truth 11 |
| ARM-12 | 20-12, 20-17 | ✓ SATISFIED | Truth 12 |

No requirement is orphaned. All 12 IDs mapped to Phase 20 appear in the plans.

### Decision Coverage

All 8 trackable CONTEXT.md decisions are honored by the shipped artifacts (`check.decision-coverage-verify`: honored 8/8, not_honored []).

### Test Quality Audit

| Test set | Linked req | Active | Skipped | Circular | Assertion level | Verdict |
|---|---|---|---|---|---|---|
| aarch64 `exec::shebang_tests` | ARM-08/12 | 6 (×2 legs) | 0 | no | value (exact argv/errors) | OK |
| aarch64 `syscall::stat_wire_tests` | ARM-08/12 | 8 (×2) | 0 | no | value (exact errstrs, slot arming) | OK |
| aarch64 `bootconf_tests` | ARM-10/12 | 6 | 0 | no | value (pairs, caps, error kinds) | OK |
| aarch64 `cap_extents_tests` | ARM-09/12 | 5 | 0 | no | value; the live-shape case is from a real capture | OK |
| aarch64 `fdgrp_release_tests`, `husk_sweep_tests`, `create_cexec_tests` | ARM-06 | 7 + 3 + 2 | 0 | no | behavioral (close counts, EOF, mount close) | OK |
| Live markers (tier-1, cpu) | ARM-07/11 | 7 + 7 markers | 0 | no | behavioral; each new pin was shown red at the old pin (recorded) | OK |

No tests are disabled and no circular patterns were found. Insufficient assertions: 0. The recursion cap has no test at all, which is a known gap covered by override.

### Anti-Patterns Found

Debt markers: none. The phase diffs (r9 `8e4e730..e6e5220` over aarch64/xtask/port/.github/flake.nix, and Harvey `c50c282bd..f7bd7c156` over build/nix/flake/.github) add no `TBD`/`FIXME`/`XXX`/`TODO`/`HACK` line and no `todo!`/`unimplemented!`.

| File | Issue | Severity |
|---|---|---|
| `.planning/REQUIREMENTS.md:96,191` | ARM-06 is still `[ ]` / Pending. The code and pins are delivered (truth 6), but no SUMMARY claims it: 20-08 and 20-10 list `requirements-completed: []`, and 20-11 claims only ARM-03/05/07. | ⚠️ Warning (W1) |
| `20-10-SUMMARY.md:38` | "20-11 still owes the fd-drain cascade" contradicts the SUMMARY's own Task 1 table and the code. Both ab0a977 halves are in aarch64, from 3a1c9e5/5893ae9 in the 27f38a3 pin. | ⚠️ Warning (W1, same root cause) |
| `deferred-items.md:47` | The create(OCEXEC) item still says "in progress (debug)". It was fixed in r9 437b752, which is in the pin, and is pinned by `T1-OK cexeccreate` and the host tests. | ℹ️ Info (W2) |
| `.planning/WINDOWS.md` row 49 | The latest note calls 79c55b1 and 46a5c87 "local and unpushed". Both are ancestors of e6e5220 = origin/main. The status stays open by owner decision. | ℹ️ Info (W3) |
| `.planning/WINDOWS.md` frontmatter vs table (#47/#50/#66 JSON vs table) | Known inconsistency, left for the owner. Entries #70 and #71 are open and owner-deferred. | ℹ️ Info (owner-known) |
| `deferred-items.md` 20-07 (7c uchar narrowing), 20-09 (aarch64 heap census reads live 0), 20-05 (trace sweep launch race) | Recorded and outside the phase requirements | ℹ️ Info |

### Human Verification Required

None. This is an infrastructure phase with no user-facing elements, and no truth is behavior-unverified. The two truths that would otherwise have routed to a human (the at-pin N series and the recursion-cap pin) are owner decisions, recorded as overrides.

The hosted CI results (r9 `e6e5220` green including `test-aarch64-host` and `build-aarch64-virtcpu`; Harvey `3f16b791d` green including `cpu-smoke`) are owner-reported, with run IDs in 20-EVIDENCE.md. I could not re-observe them because `gh` is not installed here, so I am noting this rather than raising a human item.

### Gaps Summary

No blocking gaps. The phase goal holds in the codebase. `boot-r9-smoke` is green at the pin, and I re-ran it. Every ARM-01..ARM-12 item has code at r9 e6e5220 and Harvey f7bd7c156. The pure logic is pinned by aarch64 host tests that I ran green, and the cpu-port arms are exercised live by `boot-r9-cpu`, which I also re-ran green on HVF.

Record fixes for the owner (non-blocking):
- **W1:** tick ARM-06 in REQUIREMENTS.md (the checkbox and the traceability row), and correct the stale "still owes the fd-drain cascade" note in 20-10-SUMMARY. The fd-drain cascade is `proc.rs:2047-2063` plus `release_pgrp` at `:2147`, with host pins.
- **W2:** mark the deferred-items create(OCEXEC) entry resolved (r9 437b752).
- **W3:** refresh the WINDOWS #49 note, since 79c55b1 and 46a5c87 are now pushed and pinned. The status stays open pending the SMP-work series, as the owner decided.

Owed by owner decision, not counted as gaps: the full HVF series at the pin (smoke 1753, net-smoke 36, tier-1 10, cpu 36); closing WINDOWS #49; WINDOWS #70 and #71; a pin for the `#!` recursion cap.

---

_Verified: 2026-11-22T18:45:58Z_
_Verifier: Claude (gsd-verifier)_
