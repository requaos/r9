---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 04
subsystem: infra
tags: [plan9, mk, coverage, dir-sweep, generator-audit, aarch64, r-magic, watchdog, set-reconciliation, d-15, uspace-02, uspace-03]

requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-01)
    provides: gethunk ×2 fixes (the sweep cannot wedge) + FPdbleword fix
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-02)
    provides: the D-13 objtype-gated mk filters with the COVERAGE_SWEEP=1 disengagement passthrough; checkall iterating $ALLDIRS
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-03)
    provides: nm shim idiom for 9l-autolib generator links, phase evidence bundle (06-EVIDENCE.md + evidence/)
provides:
  - sys/src/games/mkfile additive checkall:V: target (cmd/mkfile:44-57 shape over games' $DIRS)
  - build/aarch64/coverage-sweep.sh — the USPACE-02 report generator: COVERAGE_SWEEP=1 unfiltered checkall + mk-visitation shim + hang watchdog + second-chance visitation pass + set-reconciling find cross-check
  - build/aarch64/dir-coverage.txt — the committed, CI-regenerable coverage report (D-15): 221-dir universe = 206 visited + 15 named exclusions, reconciliation CLOSED, zero hangs
  - build/aarch64/generator-audit.txt — the committed G1-G9 generator inventory with host-toolchain proofs + not-swept section
  - build/aarch64/generator-scan.sh — the .7-magic mechanical red (teeth-proven), darwin guest-tree probing + linux local
affects: [06-05 (lib wave flips build rows → re-baselines the report), 06-06 (per-cmd fixes → re-baseline), 06-08 (CI consumes both scripts: regen+diff the report, run the scan), 10 (the 58-row build=fail inventory is its backlog)]

# Actuals (#2632) — chars/4 over the realized diff of the plan's two commits
actuals:
  tokens: 22005
  tasks: 2
  commits: 2   # the plan's own commits; the ledger range below also contains 1 concurrent phase-07 commit

# Plan-commit ledger (#3968) — measured, includes concurrent-session commits in range
commits: 3
plan_head_before: f4ce2326872414a464ac9fffd22fc4dd5ed2e2c0
plan_head_after: 04bf52fe30db36b6d1c5b9dbd4e7004a51255e08

tech-stack:
  added: []
  patterns:
    - "mk-visitation shim: PATH-prepended rc `mk` wrapper logging COVERAGE-VISIT/COVERAGE-EXIT <pwd> <status> — uniform reach evidence across every container echo shape (ip/auth per-dir echo, upas silent, cwfs cd-recursion, games/music bare names); plan9port rc leaves $status EMPTY on success — normalize to explicit 0"
    - "second-chance visitation pass: plan9port mk recipes abort at the first failing command (measured), so container loops stop at the first failing sibling — the sweep visits each remaining in-universe dir with its own shimmed, guarded `mk all` (SECOND-CHANCE log lines)"
    - "committed report regeneration with an authoring-run line preserved verbatim across regenerations — D-15 byte-stability on an unchanged tree while still naming the first committed run"
    - "namespace/dialect safety: Harvey's hostbin shadows sed/awk/date on the devshell PATH — Plan 9 sed garbles on `|` in a /-delimited pattern and silently fails \\(\\)\\1; Plan 9 date prints the epoch for -u +fmt — use awk index/substr, plain-^ seds, and a probe-verified ISO date (D-02 shadowing class)"
    - "ssh probes with </dev/null under a rows-loop (ssh eats the caller's stdin otherwise)"

key-files:
  created:
    - build/aarch64/coverage-sweep.sh
    - build/aarch64/dir-coverage.txt
    - build/aarch64/generator-audit.txt
    - build/aarch64/generator-scan.sh
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-04-coverage-log.txt
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-04-generator-scan.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-04-generator-scan-teeth.log
  modified:
    - sys/src/games/mkfile
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md

key-decisions:
  - "06-04: the sweep gained a second-chance visitation pass — the plan's checkall-only mechanism cannot satisfy its own adjacency truth under plan9port mk's measured abort-on-first-failure recipe semantics (with tier-0 libs outstanding, ip's loop stopped at httpd before ipconfig); the pass preserves the plan's fixed row grammar and the named-exclusion set exactly"
  - "06-04: jpg's committed ycbcr.h/rgbv.h satisfy mk without firing the generator chain, so the Wave-0 re-verification FORCES it (rm headers+generators in the scratch guest tree, rebuild via the mkfile's own rules, od-assert host-ELF) — state-independent by construction (concurrent-session churn proved real: the guest .build was wiped/rebuilt mid-run-1)"
  - "06-04: generator-audit re-confirmation corrected two plan/RESEARCH cells against live mkfiles (per the plan's own mandate): G5's artifact is o.pcode (cpuobjtype=o via /unix/mkfile), G6's genarch is pcc-built native-only (build guard gs/mkfile:164, run guard :149) — never built nor run in the aarch64 sweep"
  - "06-04: every ssh tree-probe in generator-scan.sh carries </dev/null (ssh drained the audit-rows while-loop's stdin after G1 — measured); darwin probes the swept guest tree, linux checks the local tree"

patterns-established:
  - "Set-reconciling coverage proof: find-universe == visited ∪ named-exclusions exactly, with stale-exclusion and phantom-visit as gate failures — a silent drop is impossible by construction"
  - "Teeth-proven CI reds: a synthetic-plant red demonstration (planted R_MAGIC binary at a negative-sweep name → RED + exit 1 → removed → re-green) is acceptance evidence, logged to the phase evidence bundle"

requirements-completed: [USPACE-02, USPACE-03]

coverage:
  - id: D1
    description: "dir-coverage report machinery + committed baseline: COVERAGE_SWEEP=1 unfiltered checkall over cmd+games with shim-observed visitation, hang watchdog, second-chance pass, and set-reconciling cross-check — 212 visits, 212 exits, 0 watchdog fires; universe 221 == visited 206 ∪ excluded 15 exactly, CLOSED"
    requirement: USPACE-02
    verification:
      - kind: integration
        ref: "sh build/aarch64/coverage-sweep.sh — exit 0, COVERAGE-SWEEP-OK; zero WATCHDOG lines; evidence/06-04-coverage-log.txt committed"
        status: pass
      - kind: other
        ref: "grep -c 'visited=' build/aarch64/dir-coverage.txt → 206 (≥200 floor); report tail universe_count=221 visited_count=206 excluded_count=15 verdict CLOSED"
        status: pass
    human_judgment: false
  - id: D2
    description: "games additive checkall target (cmd/mkfile:44-57 shape over $DIRS); no other games target touched"
    requirement: USPACE-02
    verification:
      - kind: other
        ref: "grep -c 'checkall:V:' sys/src/games/mkfile → 1; git diff 48224aee9^..48224aee9 -- sys/src/games/mkfile is the single additive hunk"
        status: pass
    human_judgment: false
  - id: D3
    description: "Wave-0 blocker verdicts re-verified in print: jpg's generator chain force-regenerated host-ELF (7f454c46 ×2 + headers present); mahjongg visited=1 build=ok (01-01-SUMMARY.md:156 citation); 4a visited=1 build=ok with the target-sweep-exclusion decision recorded (01-01-SUMMARY.md:150 + PITFALLS)"
    requirement: USPACE-02
    verification:
      - kind: integration
        ref: "BLOCKER-CHECK lines in the run log (magic=7f454c46 on unix.rgbycc/unix.rgbrgbv, headers present); report notes section carries the rows + citations"
        status: pass
    human_judgment: false
  - id: D4
    description: "generator audit: exactly 9 G<n> site= rows in fixed field order (G1-G9 re-confirmed against live mkfiles) + not-swept section; G6 carries the guard-line proof + tier-exclusion note"
    requirement: USPACE-03
    verification:
      - kind: other
        ref: "grep -c '^G[0-9] site=' build/aarch64/generator-audit.txt → 9; grep -c '^not-swept:' → 16"
        status: pass
    human_judgment: false
  - id: D5
    description: ".7-magic mechanical red: scan green on the swept tree (G1-G5 ELF, G7 #!, G9 text-NOTE, G6/G8 guarded-skip, whitelist exactly 7.genarch) and teeth-proven by a synthetic plant"
    requirement: USPACE-03
    verification:
      - kind: integration
        ref: "sh build/aarch64/generator-scan.sh → GENERATOR-SCAN-OK; teeth: planted sys/src/cmd/va/7.out (od-verified 00008c47) as sys/src/cmd/jpg/7.rgbycc → 'RED: target-built generator in the tree: sys/src/cmd/jpg/7.rgbycc' + exit 1; removed, re-green (evidence/06-04-generator-scan{,-teeth}.log)"
        status: pass
    human_judgment: false

duration: 37 min
completed: 2026-11-05
status: complete
---

# Phase 6 Plan 04: Dir-Coverage Report + Generator Audit + .7-Magic Scan Summary

**The complete-reach proof and the generator host-safety red are committed and CI-regenerable: a shim-observed, filter-disengaged aarch64 `mk checkall` over cmd+games (212 visits, zero hangs) reconciles EXACTLY against the 221-dir find universe (206 visited ∪ 15 named exclusions), and the G1-G9 generator audit pairs every build-then-run generator with its host-toolchain proof while generator-scan.sh mechanically reds on any R_MAGIC artifact — teeth-proven by a synthetic plant.**

## Performance

- **Duration:** 37 min
- **Started:** 2026-11-05T01:31:36Z
- **Completed:** 2026-11-05T02:08:37Z
- **Tasks:** 2
- **Files modified:** 2 (games/mkfile, 06-EVIDENCE.md); **Files created:** 7 (4 build artifacts + 3 evidence logs)

## Accomplishments

- **USPACE-02 committed and CI-regenerable (D-15):** `dir-coverage.txt` carries 206 sorted per-dir `visited=N build=ok|fail|hang` rows, 15 named in-universe exclusions with cited reasons, 3 boundary `excluded=decision` lines (ape/nboot/acme), and a reconciliation tail that CLOSED: `universe_count=221 visited_count=206 excluded_count=15` — phantom-free, zero stale exclusions. The sweep ran with `COVERAGE_SWEEP=1` (the D-13 filters disengaged at every depth — the sweep certifies the whole tree, never the allowlist to itself), a PATH-prepended mk shim logging every sub-mk with full paths, and a 10-minute hang watchdog (zero fires — 06-01's gethunk class stays closed).
- **The adjacency truth holds:** cmd/ip's container recursion visits ipconfig AND its 6 siblings, each recorded with full path (`visited=1` each; httpd/ip fail on missing tier-0 libs — data; ipconfig/ppp/snoopy/dhcpd/ftpfs/imap4d build=ok). cc visited=2 by construction; awk/jtagfs/spell visited=2 via their generator sub-mks.
- **USPACE-03 artifact-backed:** 9-row generator audit (G1 awk maketab, G2/G3 jpg rgbycc/rgbrgbv, G4 jtagfs genrtab, G5 spell pcode, G6 gs genarch guarded, G7 usbd mkdev rc-script, G8 bayes regen guarded, G9 7l mkcname ed-script) + the 16-line not-swept section; the scan asserts host-ELF/`#!` magic on every runs=always artifact (all green on the swept tree), whitelists exactly 7.genarch (derived from the audit's G6 row), and scans the build log for exec-format failures. Teeth proven: a planted `7.rgbycc` (a real R_MAGIC binary) produced `RED: target-built generator in the tree: sys/src/cmd/jpg/7.rgbycc` + exit 1; removed; re-green.
- **Wave-0 blocker verdicts re-verified in print:** jpg's ycbcr.h/rgbv.h force-regenerated by host-ELF unix.rgbycc/unix.rgbrgbv (the Phase-1 fix 7be7e98c8 holds for the aarch64 sweep); mahjongg `build=ok` (01-01-SUMMARY.md:156); 4a `build=ok` + target-excluded-by-decision (01-01-SUMMARY.md:150, PITFALLS notorious-dirs).
- **Phase-10 backlog measured:** 58 per-dir `build=fail` rows, overwhelmingly missing tier-0 libraries (libmemdraw/libhttpd/libplumb/libventi/libsunrpc…) that 06-05's wave unblocks.

## Reconciliation tail (verbatim from the committed report)

```
universe_count=221
visited_count=206
excluded_count=15
reconciliation: universe == visited ∪ excluded exactly; zero phantom visits; zero stale exclusions — CLOSED
```

The measured in-universe excluded set (each named with its reason in the report):

- `sys/src/games/blabs` — upstream-disabled (commented out of games' DIRS)
- `sys/src/cmd/cwfs/{9netics32.16k,9netics64.8k,choline,emelie,fs,fs64}` — upstream-omitted (cwfs's default recursion enters only cwfs/cwfs)
- `sys/src/cmd/postscript/{picpack,postbgi,postdaisy,postdmd,postmd,trofftable}` — upstream-disabled (commented out of postscript's TARGETS, UNIX remnants)
- `sys/src/cmd/aux/antiword` — upstream-omitted (not in aux's DIRS)
- `sys/src/cmd/tcs/font` — upstream-omitted (tcs/mkfile has no font recursion)

## build=fail inventory (Phase-10 backlog data, 58 rows)

Container rows (aggregate children) marked †; leaf rows are the actionable per-dir set:

`cmd† games† cmd/5i cmd/abaco cmd/acid cmd/acme cmd/aux† cmd/aux/na cmd/compress cmd/cpp cmd/db cmd/eqn cmd/faces cmd/fossil cmd/grap cmd/graph cmd/gs cmd/htmlfmt cmd/ip† cmd/ip/httpd cmd/jpg cmd/ki cmd/lp cmd/lzip cmd/map cmd/page cmd/pic cmd/plot cmd/plumb cmd/postscript† cmd/postscript/common cmd/postscript/download cmd/postscript/p9bitpost cmd/postscript/postgif cmd/postscript/postprint cmd/postscript/postreverse cmd/postscript/posttek cmd/postscript/tcpostio cmd/postscript/tr2post cmd/ptrace cmd/qi cmd/rio cmd/sam cmd/samterm cmd/scat cmd/snap cmd/spin cmd/troff cmd/upas† cmd/upas/fs cmd/upas/ned cmd/vac cmd/venti† cmd/venti/srv cmd/vi cmd/vnc cmd/webfs games/mp3enc`

Dominant failure class: missing tier-0 libraries at link (`cannot open file: /aarch64/lib/lib{memdraw,httpd,plumb,venti,sunrpc,…}.a`) — 06-05's lib wave flips most of these; the report's re-baseline standing rule names 06-05/06-06 as re-generators.

## Task Commits

1. **Task 1:** `48224aee9` — `feat(06-04): dir-coverage report machinery + committed baseline (USPACE-02, D-15)` (games checkall + coverage-sweep.sh + dir-coverage.txt + EVIDENCE append + committed reach log)
2. **Task 2:** `04bf52fe3` — `feat(06-04): generator audit artifact + .7-magic scan (USPACE-03)` (generator-audit.txt + generator-scan.sh + EVIDENCE scan/teeth records + 2 evidence logs)

**Plan metadata:** the final docs commit rides on top (SUMMARY + STATE/ROADMAP/REQUIREMENTS).

## Files Created/Modified

- `sys/src/games/mkfile` — additive `checkall:V:` target only (git show --stat 48224aee9 names the single hunk)
- `build/aarch64/coverage-sweep.sh` — new; the sweep driver (shim + watchdog + second-chance + reconciliation + report writer; NATIVE_LINUX split)
- `build/aarch64/dir-coverage.txt` — new committed report (D-15)
- `build/aarch64/generator-audit.txt` — new committed audit (G1-G9 + not-swept)
- `build/aarch64/generator-scan.sh` — new; the .7-magic red (darwin guest-tree probing, linux local)
- `evidence/06-04-coverage-log.txt` — the committed VISIT/EXIT/SECOND-CHANCE reach log (459 lines)
- `evidence/06-04-generator-scan.log` / `…-teeth.log` — the green scan + the teeth-proof red
- `06-EVIDENCE.md` — run records for both gates + dialect findings + the run-1 anomaly note

## Decisions Made

- **Second-chance visitation pass (the plan's mechanism couldn't reach its own truth):** plan9port mk recipes abort at the first failing command (toy-mkfile measured 2026-11-05; 06-02's documented "strict abort-on-first-failure" semantics), so a container's recursion stops at its first failing sibling — with tier-0 libs outstanding, ip's loop stopped at httpd before ipconfig, upas's at fs, postscript's at common, aux's inside its DIRS. The plan's adjacency truth REQUIRES ipconfig + all siblings visited, so the sweep runs a second pass: each in-universe dir the recursion never visited gets its own shimmed, guarded `mk all` (SECOND-CHANCE log lines; 35 dirs this run). The named-exclusion set stays exactly the 15 intentional-outs; the row grammar is unchanged; the report header documents both passes.
- **jpg re-verification is forced, state-independent:** the committed ycbcr.h/rgbv.h satisfy mk without firing the generator chain (and mk's missing-prereq chase — proven three ways to fire on a clean tree — did not fire during run 1's sweep, an anomaly attributed to proven concurrent-session guest churn: the guest `.build` was wiped/rebuilt mid-run). The re-verification rm's headers+generators in the scratch guest tree and rebuilds via the mkfile's own rules, then od-asserts 7f454c46.
- **Audit re-confirmation corrected two plan cells** (the plan mandates re-confirming rows against live mkfiles): G5's artifact is `o.pcode` (cpuobjtype=`o` via /unix/mkfile's O=o), and G6's genarch is **pcc-built native-only** (never built nor run for aarch64 — the run guard gs/mkfile:149 plus the build guard :164); the plan's "$CC-built BUT guarded" described the older file shape. The whitelist basis and Pitfall P6-5 defense stand.
- **Dialect hygiene for namespace/devshell tooling** (D-02 shadowing class, three new measured instances): Plan 9 sed garbles on `|` inside a /-delimited pattern and silently fails `\(\)`/`\1` extractions; Plan 9 date prints the epoch for `-u +%Y-%m-%d`. The scripts use awk index/substr, plain-`^` seds, and a probe-verified ISO date.
- **`</dev/null` on every ssh probe** in generator-scan.sh: ssh consumed the audit-rows while-loop's stdin after G1 (measured — only G1 was being checked).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Second-chance visitation pass added to the sweep**
- **Found during:** Task 1 (run 2's reconciliation: 35 in-universe dirs unvisited)
- **Issue:** the plan's mechanism (checkall + shim alone) cannot satisfy its own adjacency-edge truth under plan9port mk's abort-on-first-failure recipe semantics — ipconfig, a named-truth member, was never visited because httpd (missing libhttpd) aborted ip's loop first
- **Fix:** pass 2 visits each unvisited in-universe dir with its own shimmed guarded `mk all`; named exclusions skipped; watchdog coverage unchanged
- **Files modified:** build/aarch64/coverage-sweep.sh
- **Verification:** run 4 — 35 second-chance visits logged, reconciliation CLOSED, ipconfig + all 6 ip siblings recorded
- **Committed in:** 48224aee9

**2. [Rule 1 - Bug] Second-chance visits invoked the real mk directly — no VISIT lines**
- **Found during:** Task 1 (run 3's reconciliation failed identically: 35 claimed second-chance visits produced no shim lines)
- **Issue:** `$REAL_MK all` at the dir logs only NESTED sub-mks; a leaf runs none — the dir's own visit must go through the shim (bare `mk` with SHIMD on PATH)
- **Fix:** one-line invocation change in the pass-2 loop
- **Files modified:** build/aarch64/coverage-sweep.sh
- **Verification:** run 4 — 212 visits = 177 pass-1 + 35 pass-2, exits match, zero watchdog
- **Committed in:** 48224aee9

**3. [Rule 1 - Bug] Plan 9-tool dialect failures in the namespace/devshell path**
- **Found during:** Task 1 (runs 2-3)
- **Issue:** (a) the host-side coverage-log extraction's sed `{/pat/!p}` group is garbled by the host's sed; (b) the guest-side exclusion field-cut `sed 's/|.*$//'` is garbled by Harvey's hostbin Plan 9 sed (namespace PATH) — the exclusions were silently empty in run 2 (blabs got second-chanced); (c) the authoring-line's `date -u +%Y-%m-%d` printed the epoch under Plan 9 date
- **Fix:** awk for the extraction (BEGIN/END flag idiom) and the field cut (index/substr); the exclusion guest file is pre-computed host-side as plain namespace-absolute paths; the date is probe-verified ISO (gdate → /bin/date → date, output must match YYYY-MM-DD)
- **Files modified:** build/aarch64/coverage-sweep.sh
- **Verification:** run 4 green end-to-end; the run-2 guest intermediates showed the empty-excluded file, run-4's computation matches the host replay exactly
- **Committed in:** 48224aee9

**4. [Rule 1 - Bug] ssh probes drained the audit-rows loop's stdin**
- **Found during:** Task 2 (first scan run checked only G1)
- **Issue:** the `while read < rowsfile` loop's ssh calls consumed the loop's stdin
- **Fix:** `</dev/null` on all three ssh probe sites + a comment naming the hazard
- **Files modified:** build/aarch64/generator-scan.sh
- **Verification:** the re-run processes all 9 rows (5 ok + 2 skip + 1 #! + 1 NOTE) — GENERATOR-SCAN-OK
- **Committed in:** 04bf52fe3

**5. [Rule 1 - Bug] Audit row corrections against the live mkfiles (plan-mandated re-confirmation)**
- **Found during:** Task 2 authoring
- **Issue:** the plan/RESEARCH described G5's artifact as `$cpuobjtype.pcode`-generic and G6 as "$CC-built BUT guarded"; the live tree says artifact `o.pcode` (O=o in /unix/mkfile) and pcc-native-only (gs/mkfile:164 build guard)
- **Fix:** the audit rows record the live truth with the guard-line proofs; the G5 runs= field made exactly `always` (dual run sites noted in proof)
- **Files modified:** build/aarch64/generator-audit.txt
- **Verification:** the scan green against the rows; 9-row count gate exact
- **Committed in:** 04bf52fe3

---

**Total deviations:** 5 auto-fixed (all Rule 1 — the plan's own mechanisms/truths were unreachable or untestable as written; every fix preserves the plan's grammar, gates, and intents)
**Impact on plan:** the deviations ARE the plan's intent made real — total reach despite failure-abort semantics, a teeth-proven red, and dialect-safe tooling. No scope creep; no amd64-leg behavior change (no shared-source edits at all this plan).

## Issues Encountered

- **Concurrent-session guest churn is real and active:** run 1's jpg generator chain did not fire under conditions proven (three controlled reproductions) to fire it; the guest `.build` was wiped/rebuilt by the Phase-07 session at that run's head. Per the concurrent-session rule, probes were made state-independent (forced regen) and the run repeated clean. Phase-07 also landed `7e5cdb965` (07-07 bind) inside this plan's measured ledger range — counted, not mine.
- **06-02's recorded plan-gate defect (WINDOWS #35, the `mk -n` non-recursion gate text):** this plan's text directs no repair of it (its gates use none of that shape); the entry stays open for the next planning pass as recorded.
- **Run-1 leftover guest-side litter:** none — the guest tree is rsync-restored each vm-run; the teeth plant was explicitly removed and re-greened.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **06-05/06-06 unblocked:** the lib wave and per-cmd fixes re-baseline `dir-coverage.txt` as their own step (the report header names the standing rule); the 58-row build=fail inventory is their measured backlog.
- **06-08 unblocked:** CI gets two one-command checks — regen `coverage-sweep.sh` + `git diff --exit-code build/aarch64/dir-coverage.txt`, and `generator-scan.sh` (the wave0-host job). Note for 06-08's authoring: the scan's CI placement before the sweep regen (per the current 06-08 draft) will red on missing runs=always artifacts in a clean checkout — the scan belongs AFTER the sweep regen, or with artifacts present from the toolchain build; the 06-08 plan should reconcile its step order against the scan's missing-artifact-is-drift gate.
- **USPACE-02/USPACE-03:** ROADMAP SC#2 and SC#3 satisfiable; both requirement IDs are declared only by this plan (06-08's frontmatter declares USPACE-10 — the CI wiring consumes these artifacts without re-declaring the IDs), so the shared-ID gate marked them complete here.

## Self-Check: PASSED

- [x] `sys/src/games/mkfile` contains the additive `checkall:V:` (grep count 1); git show 48224aee9 --stat names exactly the 5 Task-1 files
- [x] `build/aarch64/coverage-sweep.sh` exists (563 lines) — exit 0 on the recorded run, COVERAGE-SWEEP-OK
- [x] `build/aarch64/dir-coverage.txt` exists — 206 `visited=` rows (≥200 floor), 15 in-universe `excluded=` lines each naming dir+reason, 3 boundary `excluded=decision` lines (ape/nboot/acme), toolchain-letters + gs notes, jpg/mahjongg/4a verdict notes with citations, reconciliation tail CLOSED with zero deltas
- [x] `build/aarch64/generator-audit.txt` — `grep -c '^G[0-9] site='` → 9, `grep -c '^not-swept:'` → 16; G6 row carries guard-lines proof + tier-exclusion note
- [x] `build/aarch64/generator-scan.sh` exits 0 on the swept tree; whitelist names exactly 7.genarch; teeth proof red line committed at evidence/06-04-generator-scan-teeth.log
- [x] Commits found on master: 48224aee9, 04bf52fe3
- [x] EVIDENCE.md names all run logs (coverage log, scan log, teeth log)
- [x] No pre-existing dirty-tree files staged in any commit (explicit path lists throughout; rc/bin/Kill, sys/lib/postscript/**, sys/lib/troff/**, sys/src/cmd/postscript/**, .planning/config.json, graphs/tmp/state.json, hostbin binaries, and the concurrent session's mkimage.sh/subset-build.sh edits all untouched by both commits)

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
