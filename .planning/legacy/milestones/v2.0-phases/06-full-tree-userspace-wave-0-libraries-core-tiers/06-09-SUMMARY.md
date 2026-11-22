---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 09
subsystem: testing
tags: [9k, k10, qemu, tier-venue, devroot, embedded-root, marker-sweep, 6l, lp64, g-06-1]

requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-08)
    provides: the poolpanic qemu harness shape + phase6.yml CI + the maintainer direction (Harvey kernel as the x86_64 donor)
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-03/06-06/06-07)
    provides: the D-10 sweep vocabulary + the D-13 tier data + the r9 venue + the marker-contract conventions
provides:
  - "the Harvey-kernel amd64 tier venue: sys/src/9k/k10/k10t1 (committed config) + build/mm/build-9k10t1.sh + build/mm/boot-9k10t1-sweep.sh — boots standalone under plain qemu, runs the D-10 sweep from the embedded root, asserts the marker set host-side with teeth"
  - "the sweep's 9k venue arm (build/aarch64/tier1-sweep.rc): $rcm rcmain override + the libtest SKIP arm; donor/r9 behavior byte-preserved"
  - "6l LP64 double-constant fix (sys/src/cmd/6l/asm.c) — the WR-01 class cured on amd64"
  - "k10 early KZERO map raised to 8MiB (sys/src/9k/k10/l32p.s)"
  - "devroot Nbootfiles 32→64 (sys/src/9k/port/devroot.c)"
  - "the local green proof + teeth record (evidence/06-09-tier-9k-local.log + 06-EVIDENCE.md's 06-09 section)"
affects: [06-10 (wires this venue into CI as the tier-9k job), phase-10 (the rc argv-stack residual audit item, sharpened)]

actuals:
  tokens: 14300
  tasks: 2
  commits: 13

tech-stack:
  added: []
  patterns:
    - "Harvey-kernel tier venue: k10root standalone-boot shape (rc script IS /boot/boot) + poolpanic-harness classification shape + the D-10 sweep's venue arms"
    - "embedded-root boot plumbing: path first, explicit -m rcmain on EVERY rc invocation (devroot has no /rc/lib/rcmain), RTC->/dev/time + hostowner (the boot.c legs an rc-script boot skips)"
    - "parse mkrootrules cname uniqueness: dual-name embeds need staged copies (cname keys off the SOURCE path)"
    - "the venue sweep's SKIP whitelist is a fixed enumerated list with a converse assert; per-venue SKIP sets must not leak into sibling gates' file-text extractions"

key-files:
  created:
    - sys/src/9k/k10/k10t1
    - sys/src/9k/k10/root/k10t1.rc
    - build/mm/build-9k10t1.sh
    - build/mm/boot-9k10t1-sweep.sh
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-09-tier-9k-local.log
  modified:
    - sys/src/9k/port/devroot.c (Nbootfiles 32->64)
    - sys/src/9k/k10/l32p.s (early KZERO map 4->8MiB)
    - sys/src/cmd/6l/asm.c (LP64 double-constant emission fix)
    - build/aarch64/tier1-sweep.rc (the 9k venue arm)
    - .gitignore (k10t1.c, systab.c, .tier1-stage/)
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/deferred-items.md

key-decisions:
  - "k10t1 inherits k10cpu's device/dbgflg/mach/port/dir/lib sections VERBATIM (the poolpanic-proven set; header bars pruning) — only rootdir/boot differ"
  - "the venue never writes tier-ledger lines: D-14's ledgers stay aarch64-tier-pure; amd64 CI regression evidence is not tier-graduation evidence (T-06-20)"
  - "the 9k venue's SKIP whitelist is fixed at 24 names (20 computed-set members + selfhost-compile/selfhost/libtest-tdraw/libtest-disktest) with the converse assert — the expected marker set requires those four extras' SKIP lines present (the done criterion's '24 whitelisted T1-SKIP lines' read as REQUIRED, not merely permitted)"
  - "the 9k libtest SKIP pair is emitted via a block-local fn so the r9 gate's sweep-text extraction stays venue-correct (nix/tier1-r9.sh + ledger-r9-append.sh grep the file's literal 'echo T1-SKIP' lines)"
  - "in-tree fixes over workarounds: the venue's failures were fixed at the source (6l, l32p.s, k10t1.rc) per the project's maintainer constraint — the assertion set was never weakened to force green"

patterns-established:
  - "venue kernel = proven device set verbatim + embedded-root boot script + Nbootfiles raise + early-map extent sized to the embed"
  - "marker gate with teeth: boundary-anchored T1-OK-or-whitelisted-SKIP per computed marker + converse SKIP whitelist assert + classify-only hook + marker-state digest on failure"
  - "lima guest iteration discipline: guest-local disk for namespace builds, host->guest rsync reverts guest-local relinks (vm-run.sh:65), sequential venue runs under TCG load"

requirements-completed: [USPACE-04, USPACE-06]

coverage:
  - id: D1
    description: "the venue kernel config + namespace build driver: k10t1 builds from the clean private-namespace path with the embedded-count assert (43) and the git-clean gate; k10cpu relinks against the raised devroot in the same build"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "build/mm/build-9k10t1.sh run at f2543ec8e in the lima guest -> rc=0, 'embedded root files: 43', 9k10t1.elf 4,225,158 B, sys/src/9k git-clean (log evidence/06-09-tier-9k-local.log's build side; 06-EVIDENCE.md 06-09 section)"
        status: pass
    human_judgment: false
  - id: D2
    description: "the venue boots standalone under plain qemu and runs the D-10 sweep green on Harvey's own kernel: 36 read-only T1-OK (float-d01 included), 24 whitelisted T1-SKIP, T1-DONE, zero FAIL/fault lines, pid 1 never exits"
    requirement: USPACE-06
    verification:
      - kind: integration
        ref: "build/mm/boot-9k10t1-sweep.sh at f2543ec8e -> 'T1-VENUE-PASS ok=36 skip=22+2libtest fail=0 done=yes accel=tcg'; retained log evidence/06-09-tier-9k-local.log"
        status: pass
    human_judgment: false
  - id: D3
    description: "the gate has teeth: classify-only reds on a marker-deleted log (naming the marker) and a T1-OK->T1-FAIL-flipped log; greens the retained log"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "T1_CLASSIFY_ONLY runs against the retained log (rc=0), a CR-aware marker-deleted copy (rc=1, 'missing markers: awk'), and a flipped copy (rc=1, 'T1-FAIL lines: 1')"
        status: pass
    human_judgment: false
  - id: D4
    description: "the sweep's donor/r9 arms are behavior-preserved (r9 gate green locally; execution-trace byte-parity proof; SKIP extraction venue-correct at 23 names)"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "nix run .#boot-r9-tier1 rc=0 (quiet run 9: 'ALL STAGES PASS', 'T1-SKIP set equals the known list (23 names)'); rc -x traces of the 06-07 and 06-09 sweeps with venue=r9 byte-identical modulo filename/pids"
        status: pass
    human_judgment: false
  - id: D5
    description: "the tier-9k CI job's first hosted run"
    verification: []
    human_judgment: true
    rationale: "06-10 wires the venue into phase6.yml; the first hosted run is PENDING FIRST PUSH per the 05-13/06-08 precedent — nothing fabricated, no local substitute possible for a hosted runner"

duration: 60 min
completed: 2026-11-05
status: complete
---

# Phase 06 Plan 09: Harvey-Kernel Tier Venue Summary

**The amd64 tier venue now runs on Harvey's OWN kernel: a committed 9k config (k10t1 — k10cpu's proven device set verbatim) embeds the 43-file tier-1 set + D-10 sweep in devroot and boots standalone under plain qemu (no minimega/network/srv9/9front), running the sweep green locally (36 T1-OK incl. float-d01, 24 whitelisted T1-SKIP, T1-DONE, zero faults) with a toothed host-side classifier — and the venue surfaced and fixed five real defects, including the WR-01 6l LP64 float-constant class on amd64.**

## Performance

- **Duration:** 60 min
- **Started:** 2026-11-05T20:52:16Z
- **Completed:** 2026-11-05T21:53:01Z
- **Tasks:** 2
- **Files modified:** 9 (5 created, 4 modified) + 3 planning/evidence files

## Accomplishments

- The Harvey-kernel tier venue exists and is locally green end to end: `sys/src/9k/k10/k10t1` + `root/k10t1.rc` + `build/mm/build-9k10t1.sh` + `build/mm/boot-9k10t1-sweep.sh`; the retained log is `evidence/06-09-tier-9k-local.log` (commit field f2543ec8e, TCG, ok=36 skip=22+2libtest fail=0 done=yes).
- The gate has demonstrated teeth: classify-only reds on both doctored shapes (dropped marker named; flipped T1-FAIL caught) and greens the retained log.
- Five venue-surfaced fixes, each in its own commit, re-run to green: 6l LP64 double constants (`778d4616b`), the k10 early KZERO map 4→8MiB (`bba1eeaa8`), the parse cname collision + namespace PATH tail (`36542bdbb`), the venue boot plumbing (`8b55abbda` + `8088471ba`), the r9-gate extraction interaction (`fd544ef42`).
- k10cpu (the sacred amd64 kernel) re-proven at RUNTIME with the shared-code changes: rebuilt with the raised devroot + 8MiB early map, then VERDICT=PASS no-panic-signature boot-continues on the poolpanic harness (local TCG) — plus the compile-level re-proof inside every driver run.
- The r9 venue gate stays green (`nix run .#boot-r9-tier1` rc=0 on the quiet run) AND the sweep's r9 execution path is byte-identical by `rc -x` trace parity — the 06-09 arms are inert on r9.
- G-06-1's substance is delivered for 06-10's CI wiring: the venue is reproducible from a clean checkout via one script pair.

## Task Commits

1. **Task 1 kernel-side** (k10t1 config + k10t1.rc + devroot.c) — `51d6b65dc` (feat)
2. **Task 1 driver + gitignore** (build-9k10t1.sh) — `f7af94697` (feat)
3. **Rule-3 fix: dual-name embeds ride staged copies + NSK PATH /rc/bin tail** — `36542bdbb` (fix)
4. **Rule-1 fix: k10 early KZERO map to 8MiB** — `bba1eeaa8` (fix)
5. **Rule-1 fix: 6l LP64 double-constant pool emission** — `778d4616b` (fix)
6. **Rule-1 fix: the 9k libtest SKIPs via a helper (r9 gate venue-correctness)** — `fd544ef42` (fix)
7. **Task 2 harness + sweep arm** — `160018e92` (feat)
8. **Rule-1 fixes: k10t1.rc venue plumbing (RTC clock + hostowner)** — `8b55abbda` (fix)
9. **Rule-1 fix: RTC via the device path (#r/rtc)** — `8088471ba` (fix)
10. **Refactor: T1-VENUE-PASS skip split print** — `36cff6c52` (refactor)
11. **Refactor: the build driver rides the D-02 GREP idiom** — `c7df67142` (refactor)
12. **Rule-1 fix: export GREP into the namespace** — `f2543ec8e` (fix)
13. **Task 2 evidence** (retained log + EVIDENCE section + deferred items) — `a24eb7eb5` (test)

**Plan metadata commit:** recorded below (the docs commit).

## Files Created/Modified

- `sys/src/9k/k10/k10t1` — the venue kernel config (43-entry rootdir embed; k10cpu's sections verbatim)
- `sys/src/9k/k10/root/k10t1.rc` — the embedded /boot/boot rc script (path first; binds; RTC clock; hostowner; child-rc sweep; infinite console block)
- `sys/src/9k/port/devroot.c` — Nbootfiles 32→64 (additive)
- `sys/src/9k/k10/l32p.s` — early KZERO map 4→8MiB (two more PD slots)
- `sys/src/cmd/6l/asm.c` — the LP64 D_FCONST case-8 fix (int[2] halves)
- `build/mm/build-9k10t1.sh` — the namespace build driver (nuke sandwich; embed-count assert; git-clean gate)
- `build/mm/boot-9k10t1-sweep.sh` — the boot + marker-assertion harness (T1_CLASSIFY_ONLY teeth hook)
- `build/aarch64/tier1-sweep.rc` — the additive 9k venue arm ($rcm + libtest SKIP helper + header)
- `.gitignore` — k10t1.c / systab.c / .tier1-stage/
- `.planning/phases/06-*/evidence/06-09-tier-9k-local.log` — the retained green serial log
- `.planning/phases/06-*/06-EVIDENCE.md`, `deferred-items.md` — the records

## Decisions Made

- **k10cpu verbatim inheritance** — the venue's device set is the poolpanic-proven one; pruning risk outweighs tidiness (the header comment bars it).
- **No amd64 tier-ledger lines (T-06-20)** — the D-14 ledgers pin aarch64 tier artifacts; amd64 CI regression evidence is not graduation evidence, so the grammar is honored by not writing.
- **The four extra SKIP markers are REQUIRED, not just permitted** — the done criterion's "the 24 whitelisted T1-SKIP lines" reads as presence-required, so the harness's expected set includes selfhost-compile/selfhost/libtest-tdraw/libtest-disktest (beyond the plan's literal "computed MINUS stacksmoke PLUS float-d01" — an absent SKIP line reds exactly like an absent T1-OK; documented in the harness header).
- **The 9k libtest SKIPs ride a block-local fn** — the r9 gate computes its SKIP want-set by grepping the sweep's TEXT; literal lines would pollute r9's set (measured red). The file's literal SKIP lines are again exactly r9's 23.
- **In-tree fixes over workarounds** — the venue's red runs were fixed at the source per the maintainer constraint; the assertion set was never weakened.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] The namespace script's PATH lacked the /rc/bin:. tail**
- **Found during:** Task 1's first build run
- **Issue:** bootmkfile's archive rule execs `membername` (in rc/bin, only visible inside the namespace via the /rc bind); without the tail, libboot.a6 built EMPTY and bootk10cpu.out failed to link (`main: undefined: boot` etc.)
- **Fix:** `PATH="$NS_HBIN:$PATH:/rc/bin:."` in the NSK script (the phase-01 rule; nix/build.sh:28-30's proven shape)
- **Files modified:** build/mm/build-9k10t1.sh
- **Commit:** `36542bdbb`

**2. [Rule 3 - Blocker] parse's cname collision on dual-name embeds**
- **Found during:** Task 1's second build run
- **Issue:** the plan's embed list names /amd64/bin/rc and /amd64/bin/echo twice each (hrc+rc, hecho+echo); mkrootrules keys each entry's C symbol name off the SOURCE path, so 6l failed with 'multiple initialization' on `_amd64_bin_rc{code,len}`
- **Fix:** the second name of each pair rides a staged copy (.tier1-stage/rc, .tier1-stage/echo); 43-entry count and /boot vocabulary unchanged
- **Files modified:** sys/src/9k/k10/k10t1, build/mm/build-9k10t1.sh
- **Commit:** `36542bdbb`

**3. [Rule 1 - Bug] The k10 early KZERO map (0-4MiB) is too small for the 43-file embed**
- **Found during:** Task 2's first boot (qemu exited pre-banner, rc=0, empty capture)
- **Issue:** the venue image's data+BSS ends at 5.16MiB physical; l32p.s's two 2MiB PDEs map only 0-4MiB at KZERO, so main()'s BSS memset faulted (v=0e write at CR2=0xfffffffff04e13d8 — the .data FileSiz end), double/triple fault, qemu -no-reboot exit (diagnosed via qemu -d int)
- **Fix:** two more PD slots (KZERO+[4,6) and +[6,8)MiB) — consecutive free entries; identity map + teardown untouched; mmuinit still builds the real map. k10cpu runtime re-proof: VERDICT=PASS boot-continues on the poolpanic harness
- **Files modified:** sys/src/9k/k10/l32p.s
- **Commit:** `bba1eeaa8`

**4. [Rule 1 - Bug] 6l LP64 double-constant pool emission (the WR-01 class)**
- **Found during:** Task 2's first green-boot attempt (T1-FAIL awk 'division by zero', float-d01, seq 'zero increment')
- **Issue:** 6l datblk's D_FCONST case-8 emitted via the embedded struct ieee (16 bytes on LP64 hosts) — the 8-byte emit wrote l's low half + l's zero high half; h never reached the binary (same latent class as 7l's 06-03 fix; deferred then as 'no current consumer')
- **Fix:** mirror 7l's 2bbca81ed — emit the two 32-bit halves via int[2]. Byte-level proof: test link carries correct 1/3, 1.0, 2^32 constants; the venue then greened the float family
- **Files modified:** sys/src/cmd/6l/asm.c
- **Commit:** `778d4616b`

**5. [Rule 1 - Bug] The sweep's 9k libtest SKIP lines polluted the r9 gate's want-set**
- **Found during:** the r9 parity re-run (tier1-r9 red: 'T1-SKIP set mismatch', want grew libtest-tdraw)
- **Issue:** nix/tier1-r9.sh + ledger-r9-append.sh compute r9's SKIP set by grepping the sweep's TEXT for `echo T1-SKIP` lines; my 9k arm's two literal lines aren't r9-venue lines
- **Fix:** emit the 9k-only pair via a block-local fn (runtime emission unchanged; the file's literal SKIP lines are exactly r9's 23 again)
- **Files modified:** build/aarch64/tier1-sweep.rc
- **Commit:** `fd544ef42`

**6. [Rule 1 - Bug] The rc-script boot skips boot.c's clock and hostowner legs**
- **Found during:** Task 2's venue runs (T1-FAIL date — read epoch 0; T1-FAIL ps — printed NOTHING)
- **Issue:** (a) nothing set the kernel clock (fixed via `hcat '#r/rtc' > /dev/time` — devrtc registers under letter 'r', unbound into /dev on this venue, so the first attempt's /dev/rtc path didn't exist); (b) init0 sets eve="" so every /proc status line carried an empty user field — ps.c's tokenize yields 11<12 fields and silently returns per process (proven by a guest-local T1-DBG probe: 'boot Await 20 10 100 0 0 0 244 13 13')
- **Fix:** k10t1.rc sets the clock from the RTC and writes `sys` to #c/hostowner (k10root.rc:28's idiom; sysauth.c's hostownerwrite sets eve AND the user)
- **Files modified:** sys/src/9k/k10/root/k10t1.rc
- **Commits:** `8b55abbda`, `8088471ba`

**7. [Rule 1 - Bug] GREP unexported into the namespace (self-review catch)**
- **Found during:** the final self-review pass of the committed driver
- **Issue:** the D-02 idiom refactor put `"$GREP"` inside the NSK namespace script, but GREP was unexported — the buildnamespace dump carries only exported vars, so the in-namespace embed-count assert would exec an empty command
- **Fix:** `export GREP`; full re-proof run green at `f2543ec8e`
- **Files modified:** build/mm/build-9k10t1.sh
- **Commit:** `f2543ec8e`

**Total deviations:** 7 auto-fixed (1 Rule-3 namespace-env, 1 Rule-3 embed-collision, 5 Rule-1 bugs — one shared-boot-path extent, one toolchain LP64 class, three venue-plumbing/interaction fixes). **Impact on plan:** the plan's architecture held at every layer; the deviations are the venue doing exactly what it exists to do (surfacing real amd64 findings — the 6l class was the plan's own named suspect) plus machinery hardening. No scope change; no assertion weakened.

## Issues Encountered

- **The r9 venue flake storm:** seven local r9 gate runs redded at six distinct random sites (missing-marker drops at tr/ipconfig-SKIP/echo/cal/sleep; float-canary stage deaths ×2) while the 06-09 venue work ran concurrently — plus one REAL interaction (my SKIP extraction pollution, fixed in `fd544ef42`). The quiet run greens on demand, and `rc -x` execution-trace parity proves the 06-09 sweep byte-neutral on r9. The flake class is the recorded rc argv-stack residual — sharpened in deferred-items.md (the class is live under load, and DID eat markers here).
- **plan9port mk's missing-intermediate tolerance:** `mk <name>.install` reports 'up to date' when the in-tree binary is deleted but the installed one exists — silently never relinking (the venue then embeds stale binaries). Iteration rule: delete both. Recorded in deferred-items.md + 06-EVIDENCE.md.
- **Guest-state resets:** vm-run.sh's host→guest rsync (subset-build.sh's darwin leg) reverts guest-local amd64/bin relinks on every r9 gate run — the venue binaries were relinked three times before the revert mechanism was identified. Run the venues sequentially.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **06-10 unblocked:** the venue is committed and locally green; 06-10 wires `build/mm/build-9k10t1.sh` + `build/mm/boot-9k10t1-sweep.sh` into phase6.yml as the tier-9k job (clean-checkout path: setup → toolchain → full amd64 build → venue build → boot/sweep → digest in the step summary) and retires the 9front tier-donor leg. The hosted first run is PENDING FIRST PUSH per the 05-13/06-08 precedent.
- **Host-class note for 06-10:** the venue runs on linux-x86_64 (KVM on the runner — budgets sized 60s/300s) and linux-aarch64 (TCG — 600s/1500s); both proven locally this session (the lima guest is aarch64).
- **Watch items:** the rc argv-stack residual under load (deferred; Phase-10 audit item); the mk missing-intermediate trap (procedural workaround documented); a local dev's amd64/bin must be fresh from the fixed 6l for the venue to be meaningful (the full clean-checkout build guarantees it; hand-iteration needs the both-deleted relink rule).

## Self-Check: PASSED

- Files exist: `sys/src/9k/k10/k10t1` ✓ `sys/src/9k/k10/root/k10t1.rc` ✓ `build/mm/build-9k10t1.sh` ✓ `build/mm/boot-9k10t1-sweep.sh` ✓ `evidence/06-09-tier-9k-local.log` ✓ (verified on disk)
- Commits exist: `51d6b65dc` `f7af94697` `36542bdbb` `160018e92` `bba1eeaa8` `778d4616b` `fd544ef42` `8b55abbda` `8088471ba` `36cff6c52` `c7df67142` `f2543ec8e` `a24eb7eb5` (all in `git log`)
- The retained log's marker census: 36 T1-OK + 24 T1-SKIP + T1-DONE, zero T1-FAIL, zero fault lines ✓
- The venue green run at the final committed state (f2543ec8e) ✓; teeth ×3 ✓; k10cpu runtime re-proof PASS ✓; r9 parity green ✓

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
