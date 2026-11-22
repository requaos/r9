---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 06
subsystem: infra
tags: [plan9, tier1, coreutils, build-chain, self-host, canary, d-09, d-11, uspace-06, aarch64, donor-gate, ledger, yacc, lex, mk, cpp, nm, libmach, bc, a3, a7]

requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-03)
    provides: the tracer-proven tier gate stack (sourced sweep, CONF bootdir patch, ledger grammar, TIER1_TERMCMD diagnostic hook)
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-05)
    provides: the tier-0 lib wave (fresh libmach.a + libl.a et al.), the CONF always-patch fix, the rc ~ dialect measurements
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-04)
    provides: the committed dir-coverage report this plan re-verified (D-15 standing rule)
provides:
  - the full D-09 tier-1 membership (62 live entries) with comment banners and the verified xargs/dirname/kill absence record
  - the full per-binary sweep: 33 coreutils + cpp/mk/yacc/lex build-chain checks with fixed inputs, venue-guarded write flows with named T1-SKIP lines for r9
  - the donor gate green at full scale: 61 x T1-OK, zero T1-FAIL, zero panic|trap:, T1-DONE (twice: Task-1 gate + canary gate)
  - the D-11 guest self-host canary on the FULL path: in-guest h7c/h7l compile+link the committed hello.c in /tmp and the product RUNS from the srv9 writable prefix, printing the known string (A3 verified, no fallback)
  - every SC#5 build-chain member carries execution evidence (7a tiny.s assemble, 7c/7l canary, cc via cc.a7-linked-into-7c, cpp/mk/yacc/lex per-tool fixed inputs) — no artifact-only members (RESEARCH OQ-2)
  - A7 settled by observation: this bc prints 0.333 WITH the leading zero
  - nm consumer-carries libmach (tier0.ledger superseded to CARRIED:nm; no PENDING-CANARY/PENDING:nm-tier1 survives as a latest line)
  - the computed expected-marker-set assertion (from tier1-cmds.txt — never a second hardcoded list)
  - venue machinery findings encoded: donor devroot Nbootfiles raise, srvroot /sys skeleton+include staging, subshell-free sweep rc, bc's /bin/dc union shim
affects: [06-07 (r9 venue: the same sweep + T1-SKIP vocabulary ready; PENDING-CHAIN resolves; bc runs unshimmed if the image carries /bin/dc), 06-08 (CI consumes the green full-scale gate + the diff-clean coverage regen), 10 (tiers 2-4 append onto the locked membership)]

# Actuals (#2632) — chars/4 over the realized diff of the plan's fifteen commits
actuals:
  tokens: 16057
  tasks: 2
  commits: 15  # the plan's own commits; the metadata commit rides on top

# Plan-commit ledger (#3968) — measured, includes 3 concurrent phase-07 commits in range
commits: 18
plan_head_before: 5637a1e0f538559fb2864d95fb305bd7c58790de
plan_head_after: 775990a72a61b4384c967bb9abf4dc11c1e8fb09

tech-stack:
  added: []
  patterns:
    - "computed expected-marker set: the host assertion derives T1-OK names from tier1-cmds.txt (basename / dir:target / driver-only skips), so the gate can never drift from the D-13 membership"
    - "observation markers (T1-BCOBS/T1-XDOBS/T1-SELFHOST-OBS): the exact observed output rides the retained serial log beside its shape-assert — evidence and assertion are separate lines"
    - "sweep rc dialect (measured on BOTH rc implementations, 06-06): no subshells with separators inside backquotes (plan9port rc rejects); no bare '=' in argument words (9front rc terminates words there); fn producers carry multi-command input streams; guard every output redirect with an htest -d probe (redirect-open failure is an uncatchable Xerror)"
    - "donor-venue accommodations stay scripted + idempotent + disclosed in subset-exec.sh (devroot Nbootfiles raise, srvroot skeleton/include staging) — the donor is the execution venue, never the artifact under test"
    - "writable-prefix shim pattern: union-mount a /tmp shim dir over /bin with the donor's own /bin/bind to satisfy a binary's hardcoded /bin/<helper> path (bc's execl(/bin/dc))"

key-files:
  created:
    - build/aarch64/tiny.y            # yacc's committed one-rule grammar input
    - build/aarch64/tiny.l            # lex's committed one-rule scanner input (%% sections)
    - build/aarch64/tiny.mk           # mk's committed trivial-mkfile input
    - build/aarch64/tiny.s            # 7a's committed assemble stub
    - build/aarch64/nm-input.7        # nm's committed baked object (host 7c over hello.c at authoring time; *.7-gitignore force-add per the plan)
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-06-tier1-build.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-06-tier1-donor.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-06-selfhost.log
  modified:
    - build/aarch64/tiers/tier1-cmds.txt      # +37 entries (build chain + coreutils) + verified-absence record; 62 live
    - build/aarch64/tier1-sweep.rc            # the full per-binary check vocabulary + canary block (607 lines)
    - build/aarch64/subset-exec.sh            # 70-file staging, srvroot /sys + aarch64 staging, devroot cap raise, computed expected set, scratch cleanup
    - build/aarch64/markers/tier1.ledger      # +37 expansion lines + 4 canary supersedes (95 lines total)
    - build/aarch64/markers/tier0.ledger      # libmach supersede to CARRIED:nm
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md

key-decisions:
  - "06-06: A3 verified on the FULL path — the guest-compiled binary executed from the srv9 writable prefix and printed 'hello aarch64 harvey' (T1-SELFHOST-OBS); the documented COMPILE-ONLY fallback was NOT needed"
  - "06-06: A7 settled by observation — this bc prints 0.333 WITH the leading zero (T1-BCOBS 0.333)"
  - "06-06: the donor kernel's devroot /boot cap (Nbootfiles=32, devroot.c:13) is raised to 128 in the donor-COPY tree by a scripted idempotent patch in subset-exec.sh — the 70-entry bootdir needs it; disclosed harness-side accommodation (the donor is the venue, not the artifact)"
  - "06-06: the served rootfs carries no /sys — the gate stages Harvey's sys/lib/yaccpar, sys/lib/lex/ncform, and sys/include into it (yacc/lex open skeletons by compiled-in absolute paths; 7c's include search covers /sys/include for libc.h)"
  - "06-06: the sweep is subshell-free — plan9port rc rejects separated subshells in backquotes and 9front rc terminates words at a bare '=' (both measured this plan); fn producers carry multi-command input streams"
  - "06-06: bc's hardcoded execl(/bin/dc) vs the donor's minimal served /bin resolved by a union-mount shim (/tmp/bc-bin/dc <- our hdc, bind -b over /bin by the donor's own bind) — an early exercise of the A3 mechanic"

patterns-established:
  - "computed expected-set assertion from the D-13 tier file"
  - "observation markers beside shape-asserts (exact output in the retained log)"
  - "subshell-free sweep rc with fn producers + dir-probe-guarded redirects"
  - "idempotent scripted donor-venue accommodations"

requirements-completed: [USPACE-06, USPACE-04]

coverage:
  - id: D1
    description: "full D-09 tier-1 membership locked in tier1-cmds.txt: 62 live entries (24 v1.0 baseline + awk + 4 build chain + 33 coreutils incl. nm) with comment banners + the verified xargs/dirname/kill absence record (missing from BOTH trees — factual, not a scope cut)"
    requirement: USPACE-06
    verification:
      - kind: other
        ref: "grep -c '^cmd/\\|^9k/\\|^stage/' build/aarch64/tiers/tier1-cmds.txt -> 62 (>= 58 gate); ls verification of every appended path's on-disk form at authoring time"
        status: pass
    human_judgment: false
  - id: D2
    description: "the full sweep executes the membership on the donor with per-binary fixed inputs: 58 x T1-OK at the Task-1 gate (55 computed members + float-d01 + libtest-tdraw + libtest-disktest), zero T1-FAIL, zero panic|trap:, T1-DONE; write flows venue-guarded with named T1-SKIP lines for r9"
    requirement: USPACE-06
    verification:
      - kind: e2e
        ref: "sh build/aarch64/subset-exec.sh -> PASS; evidence/06-06-tier1-donor.log (marker set + T1-BCOBS 0.333 + T1-XDOBS)"
        status: pass
    human_judgment: false
  - id: D3
    description: "build-chain execution evidence per RESEARCH OQ-2's letter: cpp -D smoke, mk -f tiny.mk echo, yacc tiny.y -> y.tab.c greps yyparse, lex tiny.l -> lex.yy.c greps yylex — all in-guest on the donor, all marker-asserted; no SC#5 chain member lands artifact-only"
    requirement: USPACE-06
    verification:
      - kind: e2e
        ref: "T1-OK cpp / mk / yacc / lex in evidence/06-06-tier1-donor.log; committed inputs tiny.y/tiny.l/tiny.mk staged as bootdir data"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-11 guest self-host canary on the FULL path: the guest's own h7c+h7l compiled and linked the committed hello.c in the writable /tmp scratch (T1-OK selfhost-compile) and the product RAN from the srv9 writable prefix printing 'hello aarch64 harvey' (T1-OK selfhost + T1-SELFHOST-OBS in the retained log); 7a assembled tiny.s (T1-OK 7a); A3 verified, no fallback"
    requirement: USPACE-06
    verification:
      - kind: e2e
        ref: "sh build/aarch64/subset-exec.sh -> PASS (61 x T1-OK, zero T1-FAIL); evidence/06-06-selfhost.log carries T1-SELFHOST-OBS hello aarch64 harvey (grep-verified)"
        status: pass
    human_judgment: false
  - id: D5
    description: "ledger honesty (D-14): 37 expansion lines exec=T1-OK:donor + 4 canary supersedes (7a/7c/7l/cc — no PENDING-CANARY survives as a latest line) + tier-0 libmach superseded to exec=CARRIED:nm (nm.c:8 <mach.h>); 8-field lint green over both ledgers"
    requirement: USPACE-04
    verification:
      - kind: other
        ref: "awk NF!=8 lint -> LEDGER-FORMAT-OK (both ledgers); latest-line-per-dir audit -> NO-PENDING-CANARY-LATEST; ledger lines name the gate-ran commits (bbe2c4e82 / fab7377c4)"
        status: pass
    human_judgment: false
  - id: D6
    description: "dir-coverage D-15 standing rule: regenerated after the plan's commits — UNCHANGED (the plan's members are single-file targets or already-ok dirs; no dir-level build rows flipped), recorded here per the plan's unchanged-case instruction; git diff --exit-code green"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "sh build/aarch64/coverage-sweep.sh -> COVERAGE-SWEEP-OK (206 visited, 15 excluded, universe 221); git diff --exit-code build/aarch64/dir-coverage.txt -> DIFF-CLEAN"
        status: pass
    human_judgment: false
  - id: D7
    description: "venue machinery findings encoded in the gate (each caught by a gate run, each fixed additively): devroot Nbootfiles 32->128, srvroot /sys skeleton+include staging, subshell-free sweep dialect, bc /bin/dc union shim, scratch cleanup on both /tmp branches, guest-script literal assert lists"
    requirement: USPACE-04
    verification:
      - kind: e2e
        ref: "the final two consecutive green gates (Task-1 + canary) run the fixed machinery end-to-end; each fix commit quotes its measured failure"
        status: pass
    human_judgment: false

# Metrics
duration: 65 min
completed: 2026-11-05
status: complete
---

# Phase 6 Plan 06: Tier-1 Completion + Guest Self-Host Canary Summary

**Tier-1 is complete and donor-green at full scale: 62 committed members (the v1.0 baseline, awk, the cpp/mk/yacc/lex build chain, and 33 researcher-proposed coreutils) each carrying per-binary donor execution evidence, and the D-11 self-host canary landed on its strongest reading — the guest's OWN aarch64 7c/7l compiled the committed hello.c inside the donor guest and the produced binary RAN from the srv9 writable prefix, printing `hello aarch64 harvey` (A3 verified full-path, no fallback; A7 settled: bc prints `0.333` with the leading zero).**

## Performance

- **Duration:** 65 min
- **Started:** 2026-11-05T02:54:51Z
- **Completed:** 2026-11-05T04:00:04Z
- **Tasks:** 2
- **Files modified:** 14 (8 created incl. 4 committed fixed inputs + 3 evidence logs; 6 modified)

## Accomplishments

- **The membership is locked (D-09):** tier1-cmds.txt carries 62 live entries with comment banners, including the verified-absence record for xargs/dirname/kill (missing from BOTH Harvey and the 9front donor baseline — factual, not a scope cut). All 37 new entries compiled fresh for aarch64 with **zero per-cmd compile fixes** (62 T1-ART markers; the Pitfall-3 classes again live at runtime, and the donor gate measured none).
- **The sweep is the full check vocabulary (D-10):** round-trips on the baked sentinel, exact known-content asserts (md5sum/sha1sum digests computed at authoring time match Plan 9's exactly), usage/output markers, nm reading the committed baked nm-input.7, write-flow checks under the donor guard with named T1-SKIP lines for r9 (D-12 preparation), and the per-tool build-chain evidence checks (cpp -D smoke; mk -f tiny.mk; yacc tiny.y → y.tab.c/yyparse; lex tiny.l → lex.yy.c/yylex) — no SC#5 chain member is artifact-only (RESEARCH OQ-2).
- **The donor gate is green at full ~58-entry scale, twice:** Task-1 gate 58 × T1-OK; canary gate 61 × T1-OK (56 computed members — cmd/7a now carries its own marker — plus float-d01, libtest ×2, selfhost-compile, selfhost); zero T1-FAIL, zero panic|trap:, T1-DONE both times. The expected set is now **computed from tier1-cmds.txt** — never a second hardcoded list to rot.
- **D-11 landed on the strong reading:** `T1-OK selfhost-compile` + `T1-OK selfhost` + `T1-SELFHOST-OBS hello aarch64 harvey` in the retained serial log (T-06-12: separate compile/run markers; the known-output string is in the evidence). The one unproven mechanic (A3, writable-prefix exec) works — the pre-authorized COMPILE-ONLY fallback was not needed.
- **A7 settled by observation:** `T1-BCOBS 0.333` — this bc prints the fraction WITH the leading zero.
- **Ledgers are the honest machine-readable truth (D-14):** 37 expansion lines + 4 canary supersedes in tier1.ledger (no PENDING-CANARY survives as any dir's latest line); tier0.ledger's libmach superseded to `exec=CARRIED:nm` (nm.c:8 <mach.h>), closing 06-05's pending line; 8-field lint green over both ledgers; lines name the gate-ran commits.
- **The gate got stronger from what it caught:** every failed full-scale run produced an additive, error-quoting machinery fix (devroot cap, skeleton staging, rc dialect, bc shim, scratch cleanup) — the sweep now reaches T1-DONE with a 70-entry embedded bootdir.

## Task Commits

1. **Task 1 machinery:** `5edf99e83` — `feat(06-06): tier-1 build chain + coreutils sweep (USPACE-06, D-09)`
2. **Gate fix:** `30914b2b0` — guest-script staged-file assert literal list
3. **Gate fix:** `4e16a8055` — donor devroot Nbootfiles 32 → 128
4. **Gate fix:** `4ceb04875` — plan9-dialect-safe devroot patch (no sed -i)
5. **Gate fix:** `ce779749f` — restore the dropped `export DONOR SCRATCH DIST`
6. **Gate fix:** `a9c8ff7c3` — sweep donor-block parse (`=` quoting) + bc shim + xd -4x
7. **Gate fix:** `0ef2825d0` — re-create /tmp/tier1 after the rm check; guard look's redirect
8. **Gate fix:** `d932360ee` — yacc/lex skeleton staging, split/xd flag+value fixes, scratch-branch cleanup
9. **Gate fix:** `bbe2c4e82` — xd second anchor (4-byte-aligned word 'char')
10. **Task 1 ledger + evidence:** `f8b0b790e` — `feat(06-06): tier-1 ledger lines for the 37-entry expansion + libmach CARRIED:nm (D-14)`
11. **Task 2 machinery:** `17370bba1` — `feat(06-06): guest self-host canary — in-guest 7a/7c/7l compile+run (D-11)`
12. **Gate fix:** `cd96f5ac7` — tiny.s frame syntax + sys/include staging
13. **Gate fix:** `fab7377c4` — T1-SELFHOST-OBS observation marker (T-06-12)
14. **Task 2 ledger + evidence:** `775990a72` — `feat(06-06): canary evidence + 7a/7c/7l/cc ledger supersedes (D-11, D-14)`

**Plan metadata:** the final docs commit rides on top (SUMMARY + STATE/REQUIREMENTS).

_(The measured plan range `5637a1e0f..775990a72` also contains 3 concurrent phase-07 commits — `cf0db856c`, `01880756e`, `f6e2b5490` — counted in the #3968 ledger, not this plan's work.)_

## Files Created/Modified

- `build/aarch64/tiers/tier1-cmds.txt` — +37 entries with banners; the stale "awaiting 06-06" exclusion comment now records the landing; 62 live entries
- `build/aarch64/tier1-sweep.rc` — 607 lines: the coreutils read-only class, the donor-guarded write-flow + build-chain + canary blocks, observation markers, venue T1-SKIP vocabulary
- `build/aarch64/subset-exec.sh` — SUBSET_CORE/SUBSET_CHAIN/SUBSET_SELFHOST/SUBSET_DATA staging sets; srvroot staging (/sys skeletons + sys/include + aarch64/include + libc.a); scripted idempotent devroot Nbootfiles raise; computed expected-marker assertion; two-branch scratch cleanup
- `build/aarch64/tiny.{y,l,mk,s}` + `build/aarch64/nm-input.7` — the committed fixed inputs (nm-input.7 produced by the build venue's host 7c over hello.c; force-added past the *.7 gitignore per the plan)
- `build/aarch64/markers/tier{0,1}.ledger` — the append-only truth (95 and 72 lines)
- `06-EVIDENCE.md` — the 06-06 run record + venue findings for later plans
- `evidence/06-06-{tier1-build,tier1-donor,selfhost}.log` — the retained gate evidence

## Decisions Made

- **A3 full-path verdict:** the guest-compiled binary executed from the srv9 writable prefix and printed the known string — no COMPILE-ONLY fallback. Recorded in EVIDENCE.md with the retained log.
- **A7 verdict:** bc prints `0.333` WITH the leading zero (RESEARCH's open shape question settled by `T1-BCOBS 0.333`).
- **Donor-venue accommodations are scripted, idempotent, and disclosed in subset-exec.sh** (the devroot Nbootfiles raise + the srvroot /sys + aarch64 staging) — the donor is the execution venue, never the artifact under test; the pin gate keys on HEAD, not the working tree.
- **The sweep is subshell-free** (fn producers + cd-once + absolute paths + dir-probe-guarded redirects) — two measured rc dialect constraints (plan9port rejects separated subshells in backquotes; 9front terminates words at a bare `=`).
- **bc's hardcoded `/bin/dc`** (bc.y:988) vs the donor's minimal served /bin is resolved by a union-mount shim (`/tmp/bc-bin/dc` ← our hdc, `bind -b` by the donor's own /bin/bind) — an early in-guest exercise of the A3 mechanic.
- **dir-coverage unchanged** (D-15): the plan's members are single-file targets or already-ok dirs; no dir-level build rows flipped; regeneration + `git diff --exit-code` green; no re-baseline commit needed (the plan's recorded-unchanged case).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Guest-script assert must be a literal list**
- **Found during:** Task 1 first gate run — `SUBSET_CORE: unbound variable` aborted the wrap phase before the rebake
- **Issue:** the generated guest script runs under `set -u`; host-side SUBSET_* variables never cross the ssh/namespace boundary (only exported symbols ride the EXPORTS capture)
- **Fix:** literal staged-file list inside the generated script, with a comment pinning the constraint
- **Files modified:** build/aarch64/subset-exec.sh
- **Verification:** next gate run passed the assert phase
- **Committed in:** 30914b2b0

**2. [Rule 3 - Blocking] Donor devroot /boot cap (Nbootfiles=32) overflowed by the 70-entry bootdir**
- **Found during:** Task 1 gate run 2 — `panic: too many root files` at boot (devroot.c addlist)
- **Issue:** the donor kernel's embedded devroot caps /boot at 32 files (devroot.c:13); the full tier-1 bootdir embeds 70
- **Fix:** scripted, idempotent `Nbootfiles = 128` raise in the donor-COPY tree (guest scratch; the disclosed harness-side accommodation class per Phase-2 D-12), with a post-patch grep assert
- **Files modified:** build/aarch64/subset-exec.sh
- **Verification:** the next run booted past devroot embed (and the failure surfaced two follow-on machinery bugs below)
- **Committed in:** 4e16a8055 (+ `4ceb04875` for the plan9-dialect sed — the devshell's hostbin sed has no `-i`, measured as 'Unknown flag: i'; + `ce779749f` restoring the `export DONOR SCRATCH DIST` line my first insertion dropped)

**3. [Rule 1 - Bug] 9front guest rc terminates words at a bare `=`**
- **Found during:** Task 1 gate run 3 — `/boot/tier1-sweep.rc:409: token '=': syntax error`; the compound donor if-block never executed (zero write-flow/chain markers), `init: rc exit status: syntax error`
- **Issue:** rc's wordchr excludes `=` (donor lex.c:14) — cpp's `-DVALUE=41` argument broke the block's parse; plan9port rc accepted it host-side (an rc-implementation dialect delta)
- **Fix:** the flag is quoted (`'-DVALUE=41'`); the dialect finding is recorded in the sweep's block-head comment
- **Files modified:** build/aarch64/tier1-sweep.rc
- **Verification:** the donor block parsed and ran end-to-end on the next run
- **Committed in:** a9c8ff7c3

**4. [Rule 1 - Bug] bc produced empty output (silent dead subprocess)**
- **Found during:** Task 1 gate run 3 — empty `T1-BCOBS`, `T1-FAIL bc`
- **Issue:** bc.y's `execl("/bin/dc")` (line 988) is hardcoded and the donor's served /bin is the minimal 20-entry set with no dc — the front-end's subprocess never started
- **Fix:** bc moved into the donor guard behind a union-mount shim (`/tmp/bc-bin/dc` ← our hdc, `bind -b` over /bin by the donor's own bind); r9 takes the named T1-SKIP until 06-07 (its Harvey image carries /bin/dc natively)
- **Verification:** `T1-BCOBS 0.333` + `T1-OK bc` on the next run (and A7 settled by the observation)
- **Committed in:** a9c8ff7c3

**5. [Rule 1 - Bug] xd's -c char format spaces every character; the -4x words are byte-order, not endian-composed**
- **Found during:** Task 1 gate runs 3-4 — `T1-FAIL xd` twice
- **Issue:** `-c` prints `   %c` per byte (no contiguous 'alpha'); my first -4x anchors assumed LE-composed longs AND miscounted 'alpha' as four letters
- **Fix:** assert the 4-byte-aligned byte-order words measured via a T1-XDOBS observation marker ('alph'→616c7068, 'char'→63686172); the marker stays in the sweep (observation class)
- **Files modified:** build/aarch64/tier1-sweep.rc
- **Verification:** `T1-OK xd` on the fifth run
- **Committed in:** a9c8ff7c3, d932360ee, bbe2c4e82

**6. [Rule 1 - Bug] The 06-03 rm check deletes /tmp/tier1 itself; my extension wrote into it afterward**
- **Found during:** Task 1 gate run 4 — `file does not exist: '/tmp/tier1'` down the whole 06-06 block; look's output redirect then aborted the sweep (uncatchable rc Xerror — 07-04's lesson)
- **Fix:** the extension re-creates the scratch dir at its head, and look's redirect is preceded by an `htest -d` guard
- **Files modified:** build/aarch64/tier1-sweep.rc
- **Verification:** all write-flow checks green on the fifth run; T1-DONE held throughout (never-wedge)
- **Committed in:** 0ef2825d0

**7. [Rule 3 - Blocking] yacc/lex need their compiled-in skeletons; the served root has no /sys at all**
- **Found during:** Task 1 gate run 5 — `fatal error: cannot find parser /sys/lib/yaccpar` (yacc.c:16); lex's `/sys/lib/lex/ncform` (lmain.c:17) is the same class
- **Fix:** stage Harvey's matching skeletons into the srv9-served root (additive, read-only guest-side) — the plan's srvroot-staging precedent, extended by Task 2's include/libc.a
- **Files modified:** build/aarch64/subset-exec.sh
- **Verification:** T1-OK yacc + T1-OK lex on the fifth run
- **Committed in:** d932360ee

**8. [Rule 1 - Bug] split's dialect is `-n lines -f stem` (no -l); scratch cleanup missed the usr/glenda/tmp branch**
- **Found during:** Task 1 gate run 5 — split usage error; and bc's shim broke on the NEXT run from a stale /tmp/bc-bin (the guest's /tmp is the MCREATE bind of /usr/glenda/tmp)
- **Fix:** split check uses `-n 1 -f`; cleanup mirrors BOTH /tmp branches
- **Files modified:** build/aarch64/tier1-sweep.rc, build/aarch64/subset-exec.sh
- **Verification:** T1-OK split; consecutive green runs (idempotence restored)
- **Committed in:** d932360ee

**9. [Rule 3 - Blocking] Canary inputs: 7a TEXT frame syntax + 7c's /sys/include search**
- **Found during:** Task 2 first canary run — `/boot/tiny.s:6 syntax error, last name: TEXT` (my `$0` frame reads as `$unk`; the in-tree idiom is `$-4`) and `7c: file does not exist: 'libc.h'` (the include search covers /sys/include beyond /aarch64/include)
- **Fix:** tiny.s uses `TEXT main(SB), 1, $-4` (reproduced + fixed host-side with the hostbin 7a — no gate cycle spent); the committed sys/include tree joins the srvroot staging
- **Files modified:** build/aarch64/tiny.s, build/aarch64/subset-exec.sh
- **Verification:** full-path canary green on the next run
- **Committed in:** cd96f5ac7

**10. [Rule 2 - Missing Critical] The known-output string never hit the serial wire**
- **Found during:** Task 2 first green canary run — T1-OK selfhost proved the match, but `hello aarch64 harvey` rode /tmp/hello.out only; the plan's verify greps the string from the retained log (T-06-12)
- **Fix:** T1-SELFHOST-OBS observation marker echoes the measured output into the serial log (same class as T1-BCOBS/T1-XDOBS)
- **Files modified:** build/aarch64/tier1-sweep.rc
- **Verification:** `T1-SELFHOST-OBS hello aarch64 harvey` in evidence/06-06-selfhost.log (grep-verified)
- **Committed in:** fab7377c4

---

**Total deviations:** 10 auto-fixed (5 Rule 1 bugs, 4 Rule 3 blocking, 1 Rule 2 missing-critical)
**Impact on plan:** every deviation is gate-machinery correctness surfaced BY the gate at scale — exactly what the full-scale run exists to catch. Zero changes to plan scope, membership, or marker semantics; zero amd64-leg behavior change (all edits are build/aarch64/ harness + committed inputs). No structural/architectural changes (no Rule 4 escalations).

## Issues Encountered

- **Six gate iterations to green at scale** — each caught a real machinery defect (recorded above); the sweep's never-wedge property held throughout (T1-DONE in every log, markers reported on every failure path).
- **plan9port rc vs 9front rc grammar deltas** bit twice in one block (subshell-in-backquote host-side; bare-`=` guest-side) — the sweep is now written to the proven intersection, with the measurements recorded in the block-head comment and EVIDENCE.md for every later sweep author.
- **Concurrent phase-07 session:** 3 of its commits (docs/test) interleaved the measured plan range (counted, not mine); the shared lima guest stayed usable; no interference with the donor gate (guest-state partitioning held).
- **My own fat-finger:** the first ledger-append attempt hardcoded a mis-transcribed HEAD SHA — caught by the `git rev-parse` guard before anything was written; redone with the measured value.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **06-07 (r9 venue) unblocked:** the same sweep serves r9 via the named T1-SKIP lines (write-flow + build-chain + bc + selfhost are donor-guarded this era); the PENDING-CHAIN boot-chain lines resolve there; bc runs unshimmed if the r9 image carries /bin/dc.
- **06-08 (CI) unblocked:** the green full-scale donor gate + the diff-clean coverage regen are the pipeline's slow legs; the NATIVE_LINUX leg (06-03) + scripted venue accommodations are all in the committed scripts.
- **USPACE-06/USPACE-04:** this plan's evidence is complete; both IDs are shared with 06-07, so the shared-ID gate (#2388) correctly leaves them unmarked until 06-07's SUMMARY exists (0/2 ready at this plan's close).
- **ROADMAP SC#5's command half is satisfiable on the donor** — shell/coreutils + 7a/7c/7l/cc/cpp/mk/yacc/lex exist as aarch64 binaries and donor-execute with evidence.

## Self-Check: PASSED

- [x] `build/aarch64/tiers/tier1-cmds.txt` exists (134 lines; 62 live entries; absence record present)
- [x] `build/aarch64/tier1-sweep.rc` exists (607 lines; venue param; fn check; canary block; observation markers)
- [x] `build/aarch64/tiny.{y,l,mk,s}` + `nm-input.7` exist and are staged (the green gates consumed them)
- [x] `build/aarch64/markers/tier1.ledger` (95 lines) + `tier0.ledger` (72 lines) — 8-field lint green; no PENDING-CANARY/PENDING:nm-tier1 as a latest line
- [x] `evidence/06-06-tier1-build.log` (62 T1-ART + 35 T0-ART + TIER-BUILD-OK), `evidence/06-06-tier1-donor.log` (58 × T1-OK, T1-DONE), `evidence/06-06-selfhost.log` (61 × T1-OK, `T1-SELFHOST-OBS hello aarch64 harvey`, T1-DONE) all committed
- [x] `git diff --exit-code build/aarch64/dir-coverage.txt` clean after a fresh regeneration (coverage-sweep.sh COVERAGE-SWEEP-OK)
- [x] Commits found on master: 5edf99e83, 30914b2b0, 4e16a8055, 4ceb04875, ce779749f, a9c8ff7c3, 0ef2825d0, d932360ee, bbe2c4e82, f8b0b790e, 17370bba1, cd96f5ac7, fab7377c4, 775990a72
- [x] Donor gate green TWICE (Task-1 full-scale + canary): complete computed marker set, zero T1-FAIL, zero panic|trap:, T1-DONE
- [x] No pre-existing dirty-tree files staged in any commit (explicit path lists throughout; rc/bin/Kill, sys/lib/postscript/**, sys/lib/troff/**, sys/src/cmd/postscript/**, .planning/config.json, .planning/WINDOWS.md, graphs/tmp/state.json, compiled binaries all untouched — per-commit --stat verified)

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
