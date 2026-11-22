---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 05
subsystem: infra
tags: [plan9, tier0, libraries, aarch64, fresh-archive, libdynld, libmemdraw, donor-gate, ledger, classification, d-05, d-06, d-07, d-08, d-15, uspace-05]

requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-01)
    provides: FPdbleword fix (the float path) + gethunk fixes (the sweep cannot wedge)
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-02)
    provides: the committed tier data files + header-only ledgers + the objtype-gated mk filter
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-03)
    provides: the tracer-proven tier-0 loop (D-07 fresh-archive + T0-ART markers) + the sourced donor sweep + the ledger grammar
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-04)
    provides: the committed dir-coverage report this plan re-baselines (D-15 standing rule)
provides:
  - all 22 outstanding libraries compile fresh for aarch64 via the mksyslib portable-C fallback — 35/35 T0-ART markers, zero per-lib compile fixes, zero committed .6/.8 members in any fresh archive (D-07 proven by member listing, not just by construction)
  - the libdynld dynld-aarch64 rename verdict (D-08 recommendation (i)): probe-built green BEFORE the wave; the rename kept
  - the execution classification of all 22 (D-06/D-08): 2 × TEST:donor, 1 × TEST-FAIL:donor (libmemdraw — filed Pitfall-3-class Phase-10 checklist), 1 × PENDING:nm-tier1 (libmach), 2 × EXCLUDED:no-consumer (libaml, libdynld), 16 × EXCLUDED:no-tier1-consumer — never silently dropped
  - in-tree lib test binaries discovered, built, and run on the donor (libdraw tdraw + libdisk disktest green in the standing sweep; libmemdraw drawtest/arctest fault evidence captured; liboventi/libmemdraw stale targets documented)
  - the re-baselined dir-coverage report: 40 cmd rows flipped build=fail → build=ok (62 → 22), byte-stable across regenerations
  - the CONF bootdir always-patch fix (a stale-block skip-guard silently never landed new members)
affects: [06-06 (tier-1 coreutils + build chain + self-host canary; supersedes libmach's PENDING:nm-tier1 to CARRIED:nm), 06-07 (r9 venue; the sweep's libtest members run there), 06-08 (CI consumes the green standing gate + the re-baselined report diff), 10 (the 22 remaining build=fail rows + the libmemdraw Pitfall-3 audit + tiers 2-4)]

# Actuals (#2632) — chars/4 over the realized diff of the plan's seven task commits
actuals:
  tokens: 39002
  tasks: 2
  commits: 7   # the plan's own task commits; the metadata (SUMMARY/STATE/ROADMAP/REQUIREMENTS) commit rides on top; the ledger range below also contains 5 concurrent phase-07 commits

# Plan-commit ledger (#3968) — measured, includes concurrent-session commits in range
commits: 12
plan_head_before: d1a08212dc7c5a7ae860618e03ac31a7c424cf22
plan_head_after: ccc979757

tech-stack:
  added: []
  patterns:
    - "per-lib archive-name map in the tier-0 driver: a lib's artifact is its mkfile's LIB= basename, not always lib<dir-minus-lib>.a (liblex → libl.a, the lex runtime's historical name)"
    - "T0-XFAIL-class evidence capture for filed faults: faulting in-tree tests run on the donor once with the log retained (evidence/06-05-libtest-donor.log), then stay OUT of the standing sweep because a faulting binary trips drive9's deliberate shared panic fast-fail — the gate is never weakened for expected failures"
    - "rc ~ dialect (measured host-side): quoting DISABLES wildcarding (a quoted * is literal); multi-star spans over literal text are unreliable even bare — sweep assertions use proven edge-glob shapes or split conditions only"
    - "CONF bootdir patch as unconditional drop+insert: converges from any prior block version; skip-guards keyed on an old member name silently never land new members"

key-files:
  created:
    - sys/src/libdynld/dynld-aarch64.c  # renamed from dynld-arm64.c (D-08 verdict)
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-05-tier0-wave-build.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-05-libtest-donor.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-05-tier1-gate-donor.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-05-coverage-log.txt
  modified:
    - build/aarch64/tiers/tier0-libs.txt  # +21 appends (libbin … libaml, libdynld last) = 35 live entries
    - build/aarch64/markers/tier0.ledger  # +22 PENDING-CLASSIFY seeds +22 final classification supersedes (58 tier=0 lines total)
    - build/aarch64/subset-build.sh  # liblex→libl.a map; darwin lib rsync-back; libtest build block + magic asserts
    - build/aarch64/subset-exec.sh  # libtest staging/bootdir/asserts; CONF always-patch fix
    - build/aarch64/tier1-sweep.rc  # the libtest block (tdraw + disktest in the standing sweep)
    - sys/src/libdraw/mkfile  # additive $O.test rule (never default-built)
    - build/aarch64/dir-coverage.txt  # D-15 re-baseline: 62 → 22 build=fail rows
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md  # the wave record + Phase-10 checklist + dialect notes
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/deferred-items.md  # the wave's filed findings

key-decisions:
  - "06-05: libdynld dynld-aarch64 rename KEPT (D-08 recommendation (i)) — probe-built green in the namespace venue before the wave (fresh 3-member .7-only archive); zero dynld.h consumers tree-wide → exec=EXCLUDED:no-consumer with the artifact recorded"
  - "06-05: liblex's archive is libl.a (the lex runtime's historical name — consumers link it by that name); the tier-0 driver maps per-lib archive names explicitly — the first sweep run's abort at liblex was machinery, not compilation"
  - "06-05: CONF bootdir patch is unconditional drop+insert — the tier1-sweep.rc-keyed skip-guard treated a stale 06-03-era block as current and silently never landed the new libtest members (observed as 'file does not exist' T1-FAILs at the gate)"
  - "06-05: rc ~ dialect measured and encoded: quoting disables wildcarding in ~ patterns; multi-star spans over literal text unreliable — disktest asserts in two proven-shape halves"
  - "06-05: libmemdraw's in-tree tests fault on aarch64 (drawtest abort()s its own pixel verification; arctest wild-pointer write inside memsetl) — filed as Pitfall-3-class Phase-10 per-dir checklist per the plan's no-drive-by prohibition; the standing sweep carries only green libtest members (drive9 fast-fail interaction); exec=TEST-FAIL:donor is the honest ledger class"
  - "06-05: the plan's literal Task-2 no-PENDING-CLASSIFY grep contradicts its own Task-1 seed step under the ledger's append-only rule — ran the intent gate (latest line per dir classified); filed for the next planning pass"

patterns-established:
  - "wave commit structure: verdict commit at the head (libdynld), wave machinery commit, ledger seed commit (PENDING-CLASSIFY), machinery+evidence commit, classification commit (supersede lines), chore re-baseline commit — each gate's evidence retained under evidence/"
  - "sha256 ledger honesty: values name the gate-run artifacts matching the committed log's T0-ART emissions; archives are not byte-reproducible across runs (D-14's not-an-oracle note holds — later rebuilds drift bytes, never members)"

requirements-completed: [USPACE-05]

coverage:
  - id: D1
    description: "22 outstanding libs compile fresh for aarch64 via the mksyslib portable-C fallback: 35/35 T0-ART markers, TIER-BUILD-OK, zero per-lib compile fixes; member sanity proves every object member across all 35 fresh archives is .7 (zero committed .6/.8 cargo-culted); host archives byte-match the guest gate-time sha256s"
    requirement: USPACE-05
    verification:
      - kind: integration
        ref: "sh build/aarch64/subset-build.sh — exit 0, TIER-BUILD-OK; evidence/06-05-tier0-wave-build.log (35 T0-ART lines); ar-t member listings (zero .6/.8); host sha256 == log T0-ART for all 22"
        status: pass
    human_judgment: false
  - id: D2
    description: "libdynld verdict settled at the wave's head (D-08): dynld-arm64.c → dynld-aarch64.c rename kept — probe-built green before the sweep (DYNLD-PROBE-OK; dynld-aarch64.7 dynloadfd.7 dynld.7 only)"
    requirement: USPACE-05
    verification:
      - kind: integration
        ref: "vm-run.sh namespace probe: rm -f /aarch64/lib/libdynld.a && objtype=aarch64 mk install → DYNLD-PROBE-OK; verdict commit 2d2fa5725 precedes the wave commits"
        status: pass
    human_judgment: false
  - id: D3
    description: "all 22 libs carry a final exec classification (D-06/D-08): 2 TEST:donor + 1 TEST-FAIL:donor + 1 PENDING:nm-tier1 + 2 EXCLUDED:no-consumer + 16 EXCLUDED:no-tier1-consumer; ledger lint + per-dir coverage green; no dir's latest line reads PENDING-CLASSIFY; every EXCLUDED lib's tier-file comment names the basis"
    requirement: USPACE-05
    verification:
      - kind: other
        ref: "awk NF!=8 lint → LEDGER-FORMAT-OK (both ledgers); grep -c '^tier=0 ' → 58 (≥35); per-dir coverage over both tier files → PER-DIR-COVERAGE-OK; latest-line-per-dir awk gate → LATEST-LINES-CLASSIFIED-OK"
        status: pass
    human_judgment: false
  - id: D4
    description: "in-tree lib test binaries discovered among the 22 and run on the donor (D-06): libdraw tdraw + libdisk disktest green in the standing sweep (21 × T1-OK, zero T1-FAIL, zero panic|trap:, T1-DONE, float 0.333); libmemdraw drawtest/arctest faults captured with pc symbolization; liboventi/libmemdraw stale test targets documented"
    requirement: USPACE-05
    verification:
      - kind: e2e
        ref: "sh build/aarch64/subset-exec.sh — PASS; evidence/06-05-tier1-gate-donor.log (green standing gate) + evidence/06-05-libtest-donor.log (all four libtest markers + both fault lines)"
        status: pass
    human_judgment: false
  - id: D5
    description: "the libmemdraw test faults are FILED for Phase 10 (per-dir checklist in 06-EVIDENCE.md + deferred-items.md) and kept out of the standing sweep — applying the plan's locked no-drive-by-fix prohibition for Pitfall-3 classes, with exec=TEST-FAIL:donor as the honest ledger class"
    requirement: USPACE-05
    verification:
      - kind: manual_procedural
        ref: "06-EVIDENCE.md Phase-10 checklist entries (two fault signatures + pcs + symbolization); deferred-items.md open entries; the sweep comment naming the rejoin condition"
        status: pass
    human_judgment: true
    rationale: "the file-vs-fix call on real aarch64 pixel-path bugs is a judgment the plan delegates to its prohibition + Phase 10; a human should confirm the XFAIL-shaped evidence-capture disposition reads honestly"
  - id: D6
    description: "dir-coverage re-baseline (D-15): the wave's fresh libs flipped 40 cmd rows build=fail → build=ok (62 → 22); reconciliation still CLOSED (universe 221 == visited 206 ∪ excluded 15); byte-stable across regenerations"
    requirement: USPACE-05
    verification:
      - kind: integration
        ref: "sh build/aarch64/coverage-sweep.sh → COVERAGE-SWEEP-OK (twice); second regen + git diff --exit-code build/aarch64/dir-coverage.txt → DIFF-CLEAN at the final tree state; evidence/06-05-coverage-log.txt"
        status: pass
    human_judgment: false

duration: 31 min
completed: 2026-11-05
status: complete
---

# Phase 6 Plan 05: Tier-0 Library Wave — 22 Libs Compile Fresh for aarch64 Summary

**All 22 outstanding libraries swept to aarch64 in one wave: zero compile fixes needed, every archive fresh-built from portable C with a member-level proof that zero committed .6/.8 objects were cargo-culted, the libdynld aarch64 port settled by a kept rename probe-built before the wave, every lib execution-classified with grep/test evidence (two libmemdraw in-tree tests faulted on aarch64 and are filed as Phase-10 Pitfall-3 findings with donor-run proof), and the committed dir-coverage report re-baselined with 40 cmd rows flipped to build=ok.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-11-05T02:17:31Z
- **Completed:** 2026-11-05T02:48:48Z
- **Tasks:** 2
- **Files modified:** 9 (+ 5 created: the renamed libdynld source + 4 evidence logs)

## Accomplishments

- **The wave is green with zero per-lib compile fixes.** 35/35 `T0-ART` sha256 markers (13 inherited + 22), `TIER-BUILD-OK`, and the D-07 rule proven at member level: `ar t` across all 35 fresh archives shows `.7` objects only — the stale committed `.6`/`.8` members (10-112 per lib dir) were structurally unreachable and never archived.  Host archives byte-match the guest gate-time emissions.  The Pitfall-3 bug classes did not bite at COMPILE time — they live at runtime, which is exactly what D-06's execution evidence exists to measure (and it caught two).
- **libdynld verdict (D-08 recommendation (i)):** `dynld-arm64.c → dynld-aarch64.c` rename kept — probe-built through the namespace venue BEFORE the sweep (`DYNLD-PROBE-OK`; the mkfile's `dynld-$objtype.$O` prerequisite resolves unchanged).  Classified `EXCLUDED:no-consumer` (zero dynld.h includers outside libdynld, tree-wide grep) with its artifact sha256 recorded.
- **Honest classification of all 22** (D-06/D-08; supersede lines in exact tier-file order): `TEST:donor` libdraw (additive `$O.test` rule — test.c had none) + libdisk (`$O.test` via opendisk's plain-file fallback); `TEST-FAIL:donor` libmemdraw (below); `PENDING:nm-tier1` libmach (nm.c:8 includes mach.h — nm lands in 06-06); `EXCLUDED:no-consumer` libaml + libdynld; `EXCLUDED:no-tier1-consumer` the remaining 16 (grep evidence in 06-EVIDENCE.md; every EXCLUDED lib's tier-file comment names its basis).
- **The phase caught what it exists to catch:** libmemdraw's in-tree tests fault on aarch64 — drawtest abort()s its own stochastic pixel verification (the fault pc symbolizes INSIDE abort), arctest takes a wild-pointer write inside `memsetl`.  Filed as Phase-10 per-dir checklist entries (06-EVIDENCE.md + deferred-items.md) per the plan's locked no-drive-by prohibition; donor-run evidence retained; the standing sweep carries only the green members because a faulting binary trips drive9's deliberate shared panic fast-fail.
- **Machinery fixes with teeth:** liblex's archive is `libl.a` (driver name-map fix); the CONF bootdir patch is now unconditional drop+insert (a skip-guard keyed on the 06-03-era block silently never landed new members); the darwin leg pulls `aarch64/lib` back host-side (the plan's host artifact gate was unreachable without it); the rc `~` quoting dialect measured and encoded (quoting disables wildcarding; multi-star spans over literals unreliable).
- **D-15 re-baseline:** the fresh libs flipped 40 cmd dir rows `build=fail → build=ok` (62 → 22 remaining: rio, sam, acme, webfs, venti/srv, httpd, upas/fs, fossil, vac, …), reconciliation still CLOSED, and the report is byte-stable across a fresh regeneration at the final tree (`git diff --exit-code` green).

## Task Commits

1. **Verdict (Task 1 step 1):** `2d2fa5725` — `feat(06-05): libdynld dynld-aarch64 rename verdict (D-08)` (pure rename, 100% similarity)
2. **Wave (Task 1 steps 2-4):** `fa19c15f7` — `feat(06-05): tier-0 library wave — all 22 outstanding libs compile fresh for aarch64 (USPACE-05)` (tier file +21, driver name-map + lib rsync-back)
3. **Ledger seed (Task 1 step 5):** `90224e2e8` — `feat(06-05): seed tier-0 wave ledger lines (22 × PENDING-CLASSIFY, D-14)` (+ the committed wave build log)
4. **Libtest machinery + gate (Task 2 steps 1-3):** `9397b6823` — `feat(06-05): tier-0 lib test binaries on the donor (D-06) + CONF bootdir always-patch fix` (4 machinery files + 2 evidence logs)
5. **Classification (Task 2 steps 1-5):** `784d4662c` — `feat(06-05): tier-0 wave execution classification (D-06/D-08, USPACE-05)` (22 supersede lines + the EVIDENCE wave record)
6. **Re-baseline (Task 2 step 6):** `b7344296d` — `chore(06-05): re-baseline dir-coverage after the tier-0 wave (D-15)` (report + reach log)
7. **Register:** `ccc979757` — `docs(06-05): file the wave's findings in the phase deferred-items register`

**Plan metadata:** the final docs commit rides on top (SUMMARY + STATE/ROADMAP/REQUIREMENTS).

## Files Created/Modified

- `sys/src/libdynld/dynld-aarch64.c` — renamed from dynld-arm64.c (the D-08 verdict; thestring-consistent aarch64 name for the Inferno import)
- `build/aarch64/tiers/tier0-libs.txt` — +21 appends in the research dependency sort (libbin … libaml, libdynld last); 35 live `^lib` entries; each wave line's comment names its consumer basis
- `build/aarch64/subset-build.sh` — liblex→libl.a archive-name map; darwin `aarch64/lib` rsync-back; the libtest build/harvest/clean block; host od R_MAGIC asserts for the four test binaries
- `build/aarch64/subset-exec.sh` — libtest staging (both legs) + bootdir lines + asserts + preconditions; CONF patch unconditional drop+insert; libtest marker asserts
- `build/aarch64/tier1-sweep.rc` — the libtest block (tdraw output-check + disktest fixture check in the standing sweep; the faulting tests documented out)
- `sys/src/libdraw/mkfile` — additive `$O.test` rule (never default-built; amd64 output unchanged)
- `build/aarch64/markers/tier0.ledger` — 58 tier=0 lines (14 inherited + 22 seeds + 22 supersedes)
- `build/aarch64/dir-coverage.txt` — the D-15 re-baseline
- `06-EVIDENCE.md` — the wave record + Phase-10 per-dir checklist + the rc dialect discovery + the drive9 interaction note
- `deferred-items.md` — the wave's filed findings (libmemdraw faults, stale test targets, dynld.c:128, the plan-gate-text defect)
- 4 evidence logs under `evidence/` — the wave build, the libtest donor run (markers + faults), the green standing gate, the coverage reach log

## Decisions Made

- **libdynld rename kept** (D-08 recommendation (i)): probe-built green before the wave; classified EXCLUDED:no-consumer with the artifact recorded.
- **liblex archive-name map** in the driver (libl.a is the historical lex-runtime name; renaming the artifact would break consumer conventions — the driver adapts, not the lib).
- **CONF patch unconditional**: drop+insert converges from any prior state; the old skip-guard's failure mode was silent non-landing of new members.
- **rc `~` dialect**: quoting disables wildcarding; multi-star spans over literal text unreliable — assertions use proven edge-glob shapes or split conditions (measured host-side with plan9port rc; consistent with the donor's green sweep idioms).
- **libmemdraw filed, not fixed**: the plan's prohibition governs — Pitfall-3-class runtime bugs are Phase-10 per-dir checklist work; evidence captured on the donor; exec=TEST-FAIL:donor is the honest class; the standing sweep stays zero-expected-faults (drive9's shared fast-fail untouched).
- **Plan-gate-text defect handled by intent**: the literal no-PENDING-CLASSIFY grep cannot pass after the plan's own mandated seed step under append-only ledgers; ran the latest-line-per-dir intent gate; filed for the next planning pass.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] liblex archive-name mapping (driver machinery, not compilation)**
- **Found during:** Task 1 first sweep run — `TIER-BUILD-FAIL: lib liblex: no artifact /aarch64/lib/liblex.a` after a GREEN compile (`ar: creating /aarch64/lib/libl.a`)
- **Issue:** the driver's `lib${d#lib}.a` name map assumes lib<dir-minus-lib>.a; liblex's mkfile builds `libl.a` (the lex runtime's historical name)
- **Fix:** explicit per-lib archive-name case in subset-build.sh's tier-0 loop (verified all 22 mkfiles' LIB= lines — only liblex deviates)
- **Files modified:** build/aarch64/subset-build.sh
- **Verification:** re-run green; libl.a present, `.7`-only members, sha256 recorded
- **Committed in:** fa19c15f7

**2. [Rule 3 - Blocking] darwin leg never delivered lib archives host-side**
- **Found during:** Task 1 verify prep — the plan's `<automated>` artifact gate asserts host-side `$HARVEY/aarch64/lib/*.a`, but subset-build.sh's darwin rsync-back covered only bin/ + out/ (host aarch64/lib carried 13 stale libs, no libavl — the 06-03 gap, invisible until now)
- **Fix:** the darwin leg now pulls `aarch64/lib/` with `--delete` (mirrors the guest whose D-07 fresh-archive set is the truth); native-linux leg already in-place
- **Files modified:** build/aarch64/subset-build.sh
- **Verification:** all 22 host-side `-s` asserts pass; host sha256 == guest T0-ART for all 22
- **Committed in:** fa19c15f7

**3. [Rule 1 - Bug] CONF bootdir skip-guard treated stale blocks as current**
- **Found during:** Task 2 first donor gate — `T1-FAIL libtest-*` "file does not exist" markers
- **Issue:** `grep -qF 'tier1-sweep.rc' $CONF` skipped the patch whenever ANY prior block version was present; the 06-05 libtest lines never landed in the rebake
- **Fix:** the self-healing drop+insert awk now runs unconditionally (it already converged from any prior state; the guard only skipped work)
- **Files modified:** build/aarch64/subset-exec.sh
- **Verification:** the next gate run embedded all four libtest binaries (T1-OK libtest-tdraw on that run)
- **Committed in:** 9397b6823

**4. [Rule 1 - Bug] sweep disktest block: wrong cp + rc `~` quoting dialect**
- **Found during:** Task 2 gate runs 2-3 — `/bin/cp: file does not exist` (the donor's served /bin is minimal), then a silent FAIL on valid output
- **Issue:** (a) fixture creation used the donor's absent /bin/cp instead of the staged /boot/hcp; (b) my pattern `'...from '*'` left an unterminated quote (rc parse EOF, killed the sweep pre-T1-DONE), and the fully-quoted multi-star pattern could never match — measured host-side: quoting DISABLES wildcarding in `~`, and multi-star spans over literal text are unreliable
- **Fix:** /boot/hcp fixture with `||`-wrapped failure handling; the assertion split into two proven-shape conditions (`*' from '*` AND `[0-9]*`)
- **Files modified:** build/aarch64/tier1-sweep.rc
- **Verification:** T1-OK libtest-disktest in the green gate; the pattern shapes proven host-side with plan9port rc (positive + negative + empty controls)
- **Committed in:** 9397b6823

### Plan-accuracy resolutions (judgment, documented)

**5. Faulting in-tree tests (drawtest/arctest) — filed, not fixed, not standing-sweep members**
- **Issue:** the plan's step 3 assumed the discovered lib tests would run green ("recording the marker as the lib's exec evidence (exec=TEST:donor)"); libmemdraw's two tests FAULT on aarch64 (real Pitfall-3-class findings: drawtest's own pixel verification aborts; arctest wild-pointer write inside memsetl)
- **Resolution:** the plan's OWN locked prohibition ("MUST NOT fix Pitfall-3 bug-class findings as drive-by edits … unless they block THIS lib's compile") governs — filed as Phase-10 per-dir checklist entries with donor-run evidence (evidence/06-05-libtest-donor.log; pc symbolization in 06-EVIDENCE.md); exec=TEST-FAIL:donor added to the ledger vocabulary (honest: tested, faulted, filed); kept OUT of the standing sweep because a faulting binary trips drive9's deliberate shared panic fast-fail (06-03's documented interaction — the harness behavior untouched); tdraw/disktest carry the green TEST:donor evidence

**6. The plan's literal Task-2 no-PENDING-CLASSIFY grep contradicts its own Task-1 seed step**
- **Issue:** Task 1 step 5 mandates exec=PENDING-CLASSIFY seeds; D-14 makes the ledger append-only (superseded seeds remain as history); Task 2's `grep ... PENDING-CLASSIFY | wc -l | grep -q '^0$'` can never pass after a seeded run
- **Resolution:** ran the intent gate — latest line per dir (the ledger's own last-line-wins rule) never reads PENDING-CLASSIFY (`LATEST-LINES-CLASSIFIED-OK`); the literal gate's failure mode is recorded here and filed in deferred-items.md for the next planning pass (same class as 06-02's recorded gate-2 text defect)

---

**Total deviations:** 4 auto-fixed (3 Rule 1 bugs, 1 Rule 3 blocking) + 2 documented plan-accuracy resolutions
**Impact on plan:** every deviation is gate-machinery correctness or the phase's stated purpose made real (the libmemdraw faults are exactly the evidence D-06 exists to surface).  No scope creep; zero amd64-leg behavior change (the only source-level change is the arch-keyed rename; the libdraw mkfile rule is additive and never default-built — the amd64-safety gate was evaluated and not triggered).

## Issues Encountered

- **Concurrent-session git index races (the shared-index hazard, twice):** the Phase-07 session's `reset`+recommit briefly unstaged my staged libdynld rename mid-flight, and foreign staged files (run-r9.sh, smoke-r9.expect, nix/boot-r9.sh) appeared in the index during Task 2.  Mitigation adopted from the first incident: `git add` + `git commit` in one invocation, and pathspec-limited `git commit -m … -- <my files>` whenever the index carries foreign staged entries; every commit's `--stat` verified clean (no foreign file rode any 06-05 commit; the rename content was never lost).  The concurrent session's 5 commits inside the measured ledger range are counted, not mine.
- **drive9's panic fast-fail vs expected-fault tests:** the harness's shared `panic|trap:` fast-fail is deliberate and untouched (06-03's stance); the XFAIL-in-standing-sweep shape was built, run, and then deliberately replaced with evidence-capture so the standing gate stays zero-expected-faults — the intermediate XFAIL run IS the retained fault evidence (nothing fabricated; the log names both fault pcs).
- **`aarch64/bin` extras (`ip/httpd` empty dir, stale `mkpaqfs`, `auth/disable|enable|status`)** pre-date this plan (Nov-4 19:04 state; empty mkdirs dirs) — not this plan's products; the tier gates never assert on them.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **06-06 unblocked:** the tier-1 coreutils + build-chain expansion extends a wave-proven stack; libmach's `PENDING:nm-tier1` resolves to `CARRIED:nm` when nm lands (06-06's own plan already names this); the 22 remaining `build=fail` rows are its measured fix backlog.
- **06-07:** the sweep's libtest members (tdraw/disktest are all-venue; disktest is donor-only by venue guard) run r9-side as-is.
- **06-08:** CI consumes the green standing gate + the byte-stable coverage regen/diff; the wave0-host job's fast checks are unchanged.
- **Phase 10:** owns the libmemdraw Pitfall-3 audit (drawtest/arctest fault signatures + pcs filed), the stale test targets (liboventi/libmemdraw), dynld.c:128, and the 22-lib runtime-harder backlog the coverage report measures.
- **USPACE-05:** complete — the ROADMAP SC#5 library half is satisfiable; the shared-ID gate cleared (USPACE-05 is declared only by this plan).

## Self-Check: PASSED

- [x] All key files exist on disk (tier0-libs.txt, tier0.ledger, dynld-aarch64.c, dir-coverage.txt, 4 evidence logs, 06-EVIDENCE.md)
- [x] Commits found on master: 2d2fa5725, fa19c15f7, 90224e2e8, 9397b6823, 784d4662c, b7344296d, ccc979757
- [x] `grep -c '^lib' tier0-libs.txt` = 35; tail order libaml → libdynld; verdict commit precedes the wave commits
- [x] 22 new T0-ART lines in the committed wave log; D-07 `rm -f` visible per lib; zero `.6`/`.8` members across all 35 fresh archives
- [x] All 22 host artifacts exist (`-s`), sha256 == guest gate-time values
- [x] Ledger: 58 tier=0 lines, NF=8 lint OK both ledgers, per-dir coverage OK both tier files, latest-line-per-dir never PENDING-CLASSIFY
- [x] Donor gate PASS: 21 × T1-OK (incl. libtest-tdraw + libtest-disktest), zero T1-FAIL, zero panic|trap:, T1-DONE, float canary 0.333
- [x] Coverage re-baseline committed; fresh regen + `git diff --exit-code` = DIFF-CLEAN
- [x] No pre-existing dirty-tree files staged or committed in any of the 7 commits (per-commit --stat verified; pathspec-limited commits used once the concurrent session's staged files appeared; rc/bin/Kill, sys/lib/postscript/**, sys/lib/troff/**, sys/src/cmd/postscript/**, .planning/config.json, graphs/tmp/state.json, compiled binaries all untouched)

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
