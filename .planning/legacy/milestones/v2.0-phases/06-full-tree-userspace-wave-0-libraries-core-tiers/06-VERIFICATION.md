---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
verified: 2026-11-06T00:22:18Z
status: passed
score: 12/12 must-haves verified
covered_files:
  - .github/workflows/phase6.yml
  - .gitignore
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-01-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-01-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-02-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-02-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-03-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-03-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-04-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-04-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-05-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-05-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-06-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-06-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-07-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-07-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-08-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-08-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-09-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-09-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-10-PLAN.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-10-SUMMARY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-REVIEW-DISPOSITION.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-REVIEW-FIX.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-SECURITY.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-UAT.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-UI-REVIEW.md
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-09-tier-9k-local.log
  - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/poolpanic-verdict.md
  - aarch64/include/u.h
  - build/aarch64/coverage-sweep.sh
  - build/aarch64/dir-coverage.txt
  - build/aarch64/generator-audit.txt
  - build/aarch64/gethunk-repro-6l.sh
  - build/aarch64/gethunk-repro-cc.sh
  - build/aarch64/markers/tier0.ledger
  - build/aarch64/markers/tier1.ledger
  - build/aarch64/mkimage.sh
  - build/aarch64/nsrun.sh
  - build/aarch64/subset-exec.sh
  - build/aarch64/tier1-sweep.rc
  - build/aarch64/tiers/tier0-libs.txt
  - build/aarch64/tiers/tier1-cmds.txt
  - build/mm/boot-9k10t1-sweep.sh
  - build/mm/build-9k10t1.sh
  - sys/src/9k/k10/k10t1
  - sys/src/9k/k10/l32p.s
  - sys/src/9k/k10/root/k10t1.rc
  - sys/src/9k/port/devroot.c
  - sys/src/cmd/6l/asm.c
  - sys/src/cmd/6l/obj.c
  - sys/src/cmd/cc/macbody

covered_digest: "v2:sha256:48ccdf8b8161b888f3622b8a443aafea382ee9b735dac6f7e067139a5ad34836"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 12/12
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers Verification Report

**Phase Goal:** The mk build factory sweeps the full tree for aarch64 without structural aborts or silent hangs, gated by tiered append-only allowlists whose only truth is artifact+execution markers — tier-0 libraries and the core command/build-chain tier run green on the 9front donor kernel first, then on r9.
**Amended venue note (maintainer direction 2026-11-05, 06-UAT.md):** on amd64/x86_64 CI, donor evidence comes from Harvey's OWN kernel (9k10cpu lineage — the k10t1 venue built by 06-09, wired into phase6.yml by 06-10); 9front remains the aarch64 LOCAL donor only. This verification is against the amended split.
**Verified:** 2026-11-06T00:22:18Z
**Status:** passed
**Re-verification:** Yes — REGENERATION run (fourth). The prior report (status passed, 12/12, commit 5e83da642) went stale when the `/gsd-code-review 06 --fix --all` run landed six `fix(06-review)` commits touching covered files (aed40c38d, 8b22723c4, 6df5295e3, 0b6be9e78, d0d186ccd, d7e8f316e) plus three phase-doc commits regenerating the covered 06-REVIEW-FIX.md (b256db8fd, c4de736d7) and 06-REVIEW-DISPOSITION.md (72521f59e) ledgers. This run verified each fix diff is exactly what is claimed — comment/dead-code/idiom-class harness hygiene that disturbs no verified truth and no emitted-artifact byte — re-ran every cheap behavioral spot-check against the live tree, and recomputed the fingerprint via the gsd-tools verb.

## Regeneration Delta (what changed since the prior report, and what was re-checked)

| Change | Commit | Material? | Re-check performed |
|--------|--------|-----------|--------------------|
| IN-01 fix: dead `trailer=$(dd if="$IMAGE" bs=1 skip=$((sz - 28)) count=4 …)` line deleted from mkimage.sh; the 07-04 comment (why dd was abandoned) kept verbatim; live `od -j` extraction untouched | aed40c38d | Yes — dead-code removal in a now-covered harness script | Re-read THIS RUN: diff is exactly ONE deletion (1 file, 1 line); surrounding header/trailer assert block intact; `sh -n` + `bash -n` pass; emitted-image and assert paths unchanged — dead code only. mkimage.sh (a phase-05 artifact first touched by phase-06 work in this fix) ADDED to covered_files per the nsrun.sh precedent (53 paths total) |
| IN-02 fix: eleven bare host-side `grep` invocations in subset-exec.sh routed through `"$GREP"` (resolved once at startup, :177) | 8b22723c4 | Yes — idiom-consistency fix in a covered gate script | Re-read THIS RUN: diff is exactly 11 insertions / 11 deletions — invocation shape and arguments unchanged at every site (termrc self-heal `-qF`, srv9 poll, gdbpoke check, marker extraction `-oE`, seven final gate assertions). Residual-bare-grep scan confirms ONLY the documented exception remains: the `grep -q 'Nbootfiles = 128'` inside the quoted GUEST_EOF heredoc (:596) — generated-script text where `$GREP` is not in scope, excluded by the reviewer by design. GREP resolution line intact at :177; `sh -n`/`bash -n` pass |
| IN-03 fix: sentinel fixture comment corrected 40 → 41 bytes | 6df5295e3 | No — comment-only | Re-read THIS RUN: single-line comment change at :315 (`2 lines, 4 words, 41 bytes`); the wc assert keys on `-l` (line count) only — zero behavior change |
| IN-04 fix: rebake comment corrected 66 → 70 bootdir entries, now cites the literal assert list as source of truth | 0b6be9e78 | No — comment-only inside an ephemeral generated script's heredoc | Re-read THIS RUN: 3 insertions / 2 deletions confined to the comment block at :582-585; the assert list itself untouched; generated rebake script is `$TMPD`-ephemeral, never byte-asserted |
| IN-05 fix: gethunk fixed-bin staleness key now watches the newest `.c`/`.h` across all of sys/src/cmd/6l (was obj.c only) | d0d186ccd | Yes — gate-correctness fix in a covered repro script | Re-read THIS RUN: the rebuild test at :230 is now `[ ! -x "$SCRATCH/fixed-bin/6l" ] \|\| [ -n "$(find … -name '*.[ch]' -newer … 2>/dev/null)" ]`; the `[ ! -x … ]` short-circuit preserves the missing-binary path; covers asm.c (the other half of the 06-09 LP64 fix). `sh -n`/`bash -n` pass; two-line comment added |
| IN-06 fix: devroot.c flush-left `print` + `panic("whoops")` debug remnant promoted to a tab-indented, brace-free guarded `panic("rootgen: negative index %d (qid %#llux)", t, c->qid.path);` | d7e8f316e | Yes — covered 9k kernel source (message-only on an unreachable path) | Re-read THIS RUN: full `rootgen` re-read at :148-158 — structure intact, guard tab-indented to block depth matching the adjacent `if(t >= l->ndir)` guard, single brace-free statement per file style; `%#llux` qid format is established 9k port usage. Condition remains unreachable by construction (Qdir/Qboot return early; rootlist t≥1, bootlist t≥0); zero reachable-behavior change. Nbootfiles=64 at :14 untouched |
| 06-REVIEW-FIX.md regenerated: --all scope, 8 in scope / 8 fixed / 0 skipped, status all_fixed (WR-01/WR-02 carried with re-verification notes + IN-01..IN-06 new) | b256db8fd, c4de736d7 | Yes — covered fix-record ledger changed | Read in full THIS RUN: all six new fix descriptions match the committed diffs line-for-line (verified above); frontmatter `findings_in_scope: 8 / fixed: 8 / skipped: 0 / status: all_fixed`; verification-venue note (static + CI/lima exercising gate) consistent with the harness class |
| 06-REVIEW-DISPOSITION.md regenerated: WR-01/WR-02/IN-01..IN-06 → `fixed` (Source: 06-REVIEW-FIX.md); open: 12 → 6, total: 15 | 72521f59e | Yes — covered ledger changed | Re-read THIS RUN: 9 rows `fixed` (8 this review + CR-01 retirement reason intact); 6 `open` rows are exactly WR-03..WR-08 flagged "(not in the current review)" — carried forward, none silently dropped; open: 6 / total: 15 internally consistent |
| Phase-08 work interleaved on master (netip-tcp.rc fixture + fix) | b947c4a13, 462712bfe | No — touches only `build/aarch64/root-tree/lib/netip-tcp.rc`, not a covered path | Confirmed via `git show --stat`: neither commit touches any covered path |

**Definitive staleness enumeration:** `git log --oneline 5e83da642..HEAD` over all 52 prior covered paths + mkimage.sh returns exactly nine commits — aed40c38d, 8b22723c4, 6df5295e3, 0b6be9e78, d0d186ccd, d7e8f316e, b256db8fd, 72521f59e, c4de736d7. No other covered file moved. `build/aarch64/mkimage.sh` changed under aed40c38d but was not previously covered; it is now added to covered_files as a phase-changed impl file (53 paths total).

**Coverage-report byte-stability claim — STILL HELD.** The six fix commits do not touch `build/aarch64/dir-coverage.txt`, the tier ledgers, or the tier files (last dir-coverage change remains 0d7c1f537, the 06-10 venue-jitter disclosure that predates the prior verification). Byte-identity between the generator emission (coverage-sweep.sh:569-578) and the committed report (:20-29) was RE-DIFFED THIS RUN → VENUE-BLOCK-IDENTICAL. No fix alters any emitted-artifact byte: IN-01 is dead code; IN-02 is invocation spelling with identical arguments; IN-03/IN-04 are comments; IN-05 is a rebuild-trigger predicate; IN-06 is a panic message on an unreachable path.

**No Phase-6 source drift.** Working-tree status over the phase source surface shows only the pre-existing untracked local-build products (6.out/7.out/cc.a6/cc.a7 — the ℹ️ Info local-namespace-build dirt class). Every tracked covered file is clean.

## Goal Achievement

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | SC#1a — cc/macbody + 6l/obj.c gethunk accumulate fixes in-tree with regression repros (USPACE-01) | ✓ VERIFIED | Regression re-checked THIS RUN: `sys/src/cmd/cc/macbody:877` + `sys/src/cmd/6l/obj.c:1321` both carry `if(h == hunk + nhunk)`; repro scripts on disk (6l script parse-clean post-IN-05; staleness key now watches the whole 6l tree — a strict improvement over the verified state); committed `evidence/06-08-gethunk-verify.log` TEETH-OK/GREEN (initial verification) |
| 2 | SC#1b — 9k-amd64 pool panic re-tested with recorded verdict (USPACE-10) | ✓ VERIFIED | `evidence/poolpanic-verdict.md` (114 lines, on disk): VERDICT contract + teeth matrix (4 classification paths incl. wrong-image real-qemu); `VERDICT=PASS no-panic-signature boot-continues` against the pswt-fixed 6c; k10cpu re-proven at runtime again during 06-09. Untouched by this delta (staleness enumeration) |
| 3 | SC#1c — aarch64/include/u.h FPdbleword uint lo/hi (D-01) | ✓ VERIFIED | Union body carries exactly 2 uint members (lo, hi), 0 ulong → FPDBL-OK (prior run; file untouched since — staleness enumeration this run) |
| 4 | SC#2 — mk DIRS recursion reaches every cmd dir; committed coverage report (USPACE-02) | ✓ VERIFIED | `dir-coverage.txt` (unchanged since 0d7c1f537): universe 221 = 206 visited ∪ 15 exclusions; venue block byte-identity between generator emission (coverage-sweep.sh:569-578) and committed report (:20-29) re-diffed THIS RUN → VENUE-BLOCK-IDENTICAL |
| 5 | SC#3 — generator audit + CI .7-magic red (USPACE-03) | ✓ VERIFIED | `generator-audit.txt` (52 lines) + `generator-scan.sh` teeth log committed; scan wired into wave0-host (phase6.yml); both untouched by this delta |
| 6 | SC#4a — tiered append-only allowlists (USPACE-04) | ✓ VERIFIED | Re-run this run: tier0-libs.txt 35 entries / tier1-cmds.txt 62 entries; append-only history verified prior run (tier0 pure-append 0 deletions ever; tier1's only deletion-commit the disclosed 2 comment lines in 5edf99e83); tier files untouched by this delta |
| 7 | SC#4b — artifact+execution markers as the only gates | ✓ VERIFIED | Ledger lint re-run THIS RUN (skip `#` comments/blanks, entries = 8 space-separated k=v fields): LEDGER-FORMAT-OK over both ledgers (tier0 72 lines / tier1 154 lines); marker-gated subset-build/subset-exec stack intact — the IN-02 `$GREP` routing strengthens the marker assertions' host-portability without changing their shape |
| 8 | SC#4c — per-tier execution gates, donor FIRST then r9, via measure-first traces (amended: + Harvey-kernel x86_64 venue) | ✓ VERIFIED | Donor→r9 ordering intact (06-03→06-07 committed logs). AMENDED VENUE: census re-run THIS RUN over `evidence/06-09-tier-9k-local.log`: 36 × T1-OK + 24 × T1-SKIP + 1 × T1-DONE, ZERO T1-FAIL and ZERO panic/trap/bad-magic lines; r9 parity proven (06-07: rc=0 + rc -x trace byte-parity) |
| 9 | SC#5a — 22 outstanding libs compile fresh via mksyslib fallback (USPACE-05) | ✓ VERIFIED | Initial verification (35 × T0-ART + TIER-BUILD-OK, .7-only members); tier0-libs.txt regression-checked intact (35 entries) |
| 10 | SC#5b — every lib execution-classified honestly | ✓ VERIFIED | tier0.ledger intact (72 lines, lint-clean this run); honest classifications incl. libmemdraw TEST-FAIL filed Phase-10 |
| 11 | SC#5c — shell/coreutils + guest build chain execute on the donor (USPACE-06) | ✓ VERIFIED | tier1-cmds.txt intact (62 entries); committed 06-06 donor log (58 × T1-OK + selfhost canary observed output) + 06-07 r9 log + 06-09 Harvey-kernel log (36 × T1-OK) |
| 12 | D-16 amended — the full phase pipeline runs in CI on linux legs, x86_64 tier venue = Harvey's own kernel | ✓ VERIFIED | STRUCTURE (re-run this run): yaml parses; job set exactly {wave0-host, tier-9k, tier-r9, x86_64-poolpanic} JOBS-OK; PINS-OK 11/11 40-char SHAs; venue-pin replays 0/0/1 (prior run; file untouched since); every tier-9k referenced path exists + executable. HOSTED BEHAVIOR: CLOSED by the maintainer's UAT PASS (06-UAT.md Test 1, 2026-11-05) — all four legs green on ubuntu-latest, tier-9k's first hosted run included |

**Score:** 12/12 truths verified (Truth 12 verified via the recorded human UAT pass; 0 present-behavior-unverified)

### Deferred Items

Items discovered during execution, recorded in `deferred-items.md`, explicitly filed to later milestone phases — not gaps against this phase's goal.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | libmemdraw drawtest/arctest aarch64 faults (Pitfall-3 LP64 class) | Phase 10 | deferred-items.md: "Phase 10's libmemdraw pixel-path LP64/struct-punning audit owns this"; ROADMAP Phase 10 covers the GUI stack tiers |
| 2 | rc argv-stack residual drain sites beyond Xbackq (live under load per the 06-09 sharpening) | Phase 10 | deferred-items.md: "A Phase-10 per-dir audit item: audit rc's pushlist/poplist balance across every op" |
| 3 | Stale mkfile test targets (liboventi vttest.c, libmemdraw test.c) + libdynld unsigned-compare note | Phase 10 | deferred-items.md: "Phase 10 decides: remove the targets or restore the sources" |
| 4 | 6l/8l/5l… LP64 double-constant audit sweep for dormant arch letters (the 6l consumer WAS fixed in 06-09, commit 778d4616b) | Phase 10 / on-consumer-appearance | deferred-items.md: "Fix when an amd64/386 consumer appears… the dormant-arch letters should be audited at the same time" |
| 5 | plan9port mk missing-intermediate tolerance; darwin sbrk 4MB cap; macexpand >16KB | procedural/platform facts | deferred-items.md: documented iteration rules and venue constraints, no tree change |
| 6 | Legacy harvey.yml nix rebuild (red streak pre-dates Phase 6, recorded since 2026-11-04T12:01Z) | ROADMAP Backlog Phase 999.1 | 06-UAT.md Deferred Follow-Ups; ROADMAP.md ## Backlog (d79132ace) — explicitly "pre-existing/unrelated to Phase 6" |

### Required Artifacts (06-09/06-10 gap-closure layer + both review-fix layers; earlier-layer artifacts re-verified in the initial pass and regression-checked above)

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `sys/src/9k/k10/k10t1` | venue kernel config | ✓ VERIFIED | 246 lines; dev/uart/ip/link/misc/dbgflg/amd64/port/dir/lib sections byte-IDENTICAL to k10cpu (10/10 section diff, prior run); rootdir carries exactly 43 embed entries; untouched since (staleness enumeration this run) |
| `sys/src/9k/k10/root/k10t1.rc` | embedded /boot/boot rc script | ✓ VERIFIED | 52 lines; first line exactly `#!/boot/rc -m /boot/rcmain`; path-first + binds + RTC clock + hostowner + child-rc sweep + infinite console block (untouched since prior verify) |
| `sys/src/9k/port/devroot.c` | Nbootfiles 32→64 + IN-06 guard promotion | ✓ VERIFIED | `Nbootfiles = 64` at :14 with the 06-09 comment (untouched). IN-06 fix re-read THIS RUN: `if(t < 0) panic("rootgen: negative index %d (qid %#llux)", t, c->qid.path);` at :154-155 — tab-indented, brace-free, matching the adjacent guard's style; condition unreachable by construction (zero reachable-behavior change); guard retained rather than deleted (the only thing between a corrupt qid and a negative Dirtab index) |
| `build/mm/build-9k10t1.sh` | namespace build driver | ✓ VERIFIED | 203 lines, executable; nuke sandwich + `embedded root files: 43` assert + git-clean gate; darwin→lima dispatch (untouched since prior verify) |
| `build/mm/boot-9k10t1-sweep.sh` | boot + marker harness with teeth | ✓ VERIFIED | 375 lines, executable; T1_CLASSIFY_ONLY hook; expected set computed from tier1-cmds.txt; 24-name SKIP whitelist with converse assert; boundary-anchored T1-OK regex; T1-VENUE-PASS print (untouched since prior verify) |
| `build/aarch64/tier1-sweep.rc` | 9k venue arm | ✓ VERIFIED | venue-9k blocks; $rcm rcmain override; libtest SKIP pair via block-local fn keeping r9's literal-SKIP extraction at 23 (untouched since prior verify) |
| `sys/src/cmd/6l/asm.c` | LP64 double-constant fix | ✓ VERIFIED | `fhw[0]=p->to.ieee.l; fhw[1]=p->to.ieee.h` two-int emit re-checked THIS RUN at :315-317; commit 778d4616b; untouched by this delta |
| `sys/src/9k/k10/l32p.s` | early KZERO map 0–8MiB | ✓ VERIFIED | header comment + 06-09 note; the +[6,8)MiB PDE (untouched since prior verify) |
| `.gitignore` | k10t1.c / systab.c / .tier1-stage | ✓ VERIFIED | entries at :78/:82/:84 (untouched since prior verify) |
| `evidence/06-09-tier-9k-local.log` | retained local serial log | ✓ VERIFIED | census re-run this run: 36 OK / 24 SKIP / 1 DONE / 0 FAIL / 0 panic-class lines |
| `.github/workflows/phase6.yml` | 4-job CI, tier-donor retired + pinned | ✓ VERIFIED | JOBS-OK exact set (re-run); PINS-OK 11/11 (re-run); venue-pin replays 0/0/1 (prior run, file untouched); tier-9k job fully self-provisioned with step-summary digests + `if: always()` log upload; hosted run green (UAT Test 1) |
| `build/aarch64/coverage-sweep.sh` | host-jitter venue wording + WR-02 fix | ✓ VERIFIED | venue block byte-identical to committed report (diff re-run this run → VENUE-BLOCK-IDENTICAL); WR-02 fix intact (:394 single-hunk state, `sudo cp` gone, hard error kept); untouched by this delta |
| `build/aarch64/nsrun.sh` | shared namespace entry + WR-02 cpp bind | ✓ VERIFIED | cpp bind at :107-121 mirrors the /bin/rc bind exactly, conditional on executable `$HARVEY_LINUX_BIN/cpp` with the `set -u`-safe guard; untouched by this delta |
| `build/aarch64/gethunk-repro-6l.sh` | 6l regression repro + WR-01 + IN-05 fixes | ✓ VERIFIED | WR-01 wrapper intact at :280-285 (prior run); IN-05 re-read THIS RUN: staleness key at :230 now `find "$HARVEY_ROOT/sys/src/cmd/6l" -name '*.[ch]' -newer` with the `[ ! -x … ]` short-circuit preserved — covers asm.c and any future 6l edit; `sh -n`/`bash -n` clean; success path unchanged |
| `build/aarch64/subset-exec.sh` | venue-split header + IN-02/03/04 fixes | ✓ VERIFIED | VENUE SPLIT note at :11-17 intact; stale donor-hint ABSENT. THIS RUN: GREP resolution at :177; exactly one residual bare grep (:596, the documented generated-script exception); sentinel comment 41 bytes (:315); rebake comment 70 + source-of-truth citation (:582-585); `sh -n`/`bash -n` clean |
| `build/aarch64/mkimage.sh` | image bake + IN-01 dead-line removal | ✓ VERIFIED | NEWLY COVERED this run (changed by aed40c38d): single-line deletion of the dead `dd` trailer extraction; live `od -j` extraction + header/trailer asserts intact; 07-04 rationale comment kept verbatim; `sh -n`/`bash -n` clean |
| `06-REVIEW-DISPOSITION.md` | CR-01 + WR-01/WR-02 + IN-01..06 fixed | ✓ VERIFIED | Regenerated by 72521f59e after the --all fix run; re-read this run: 9 rows `fixed` (8 via 06-REVIEW-FIX.md + CR-01 retirement); 6 `open` rows exactly WR-03..WR-08 "(not in the current review)"; open: 6 / total: 15, internally consistent |
| `06-REVIEW-FIX.md` | fix record, status all_fixed (8/8) | ✓ VERIFIED | Regenerated by b256db8fd (+c4de736d7 title alignment); read in full this run: 8 in scope / 8 fixed / 0 skipped; all six IN fix descriptions match the committed diffs line-for-line; WR-01/WR-02 carry intact re-verification notes |
| `06-SECURITY.md` | threat register closed | ✓ VERIFIED | 25/25 register rows `closed` at L1, threats_open: 0, status verified, audit trail + sign-off present; no accepted risks (untouched since prior verify) |
| `06-UI-REVIEW.md` | UI gate verdict | ✓ VERIFIED | Verdict NO UI SURFACE IN THIS PHASE with a file-census evidence table consistent with the phase's realized diff; honest null verdict (untouched since prior verify) |
| `06-UAT.md` | human items resolved | ✓ VERIFIED | Test 1 result: pass (maintainer-confirmed 2026-11-05, all four hosted legs green); summary 1 passed / 0 issues; G-06-1 status resolved (untouched since prior verify) |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| phase6.yml tier-9k job | build/mm/build-9k10t1.sh + boot-9k10t1-sweep.sh | `nix develop -c sh` run lines | ✓ WIRED | both scripts exist, executable, substantive (203/375 lines); phase6.yml untouched since prior verify |
| tier-9k job preconditions | nix setup/toolchain/build | in-job provisioning + artifact asserts | ✓ WIRED | TOOLCHAIN-PRODUCTS-OK assert loop + amd64/9k10cpu.elf+bin/rc+bin/awk assert with BUILD-SWALLOWED-FAIL digest |
| venue-pin step ↔ workflow content | anchored job-key greps + donor-script ERE | ✓ WIRED | `^  tier-donor:` = 0, `subset-(build\|exec)\.sh` = 0, `^  tier-9k:` = 1 (prior run; file untouched) |
| k10t1 rootdir (43 entries) | build-9k10t1.sh staging → devroot.c Nbootfiles=64 | `.tier1-stage` dual-name copies + embed-count assert | ✓ WIRED | 43 ≤ 64; count asserted from generated k10t1.c inside the namespace; IN-06 touched only an unreachable panic message, not the count or the embed path |
| boot harness expected set | tiers/tier1-cmds.txt | subset-exec.sh awk mapping, MINUS stacksmoke PLUS float-d01 | ✓ WIRED | D-13 data is the single source; no second hardcoded list; IN-02/03/04 left the mapping and every assertion's shape untouched |
| tier1-sweep.rc 9k arm | $rcm / libtest SKIP helper | venue token `9k` | ✓ WIRED | r9 extraction venue-correctness proven (literal SKIP lines back to exactly r9's 23) |
| UAT pass record → Truth 12 | 06-UAT.md Test 1 ↔ phase6.yml four legs | maintainer-confirmed hosted green 2026-11-05 | ✓ WIRED | the human item's evidence chain is recorded in the phase dir; G-06-1 reconciled status resolved |
| coverage-sweep.sh native leg → nsrun.sh cpp provisioning (WR-02) | coverage-sweep.sh:405 `--wrap` invocation → nsrun.sh:118-121 conditional bind | ✓ WIRED | the in-namespace bind on the shared entry point BOTH venue legs use; hard early error at :402 keeps the missing-cpp case a loud failure; both files untouched by this delta |
| subset-exec.sh marker assertions → host grep resolution (IN-02) | subset-exec.sh:177 `GREP=$(command -v /usr/bin/grep \|\| command -v grep)` → eleven `"$GREP"` call sites | ✓ WIRED | NEW LINK this run: every host-side gate assertion now flows through the D-02-resolved grep; the one residual bare grep is inside the generated rebake script where `$GREP` is not in scope (documented exception, reviewer-excluded) |

### Data-Flow Trace (Level 4)

The phase's dynamic-data surfaces are committed marker artifacts + the retained venue log + the CI run record. Traced: the 06-09 log's commit field resolves in git log; the harness's result/print vocabulary matches the log's census (36/24/1, re-counted this run); dir-coverage.txt's venue block is byte-identical to the generator's emission (re-diffed this run, :569-578 ↔ :20-29); ledger sha256/commit fields spot-checked on master in the initial pass; the hosted-green claim traces to the maintainer's UAT record (06-UAT.md), not to a SUMMARY assertion. None of the six IN fixes touches an emitted-artifact byte: IN-01 removes dead code whose value was overwritten unread; IN-02 changes invocation spelling only (identical arguments at all eleven sites); IN-03/IN-04 are comments; IN-05 is a rebuild-trigger predicate; IN-06 changes a panic message on an unreachable path. No static-fallback or disconnected chains found.

### Behavioral Spot-Checks (all re-run THIS verification)

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| phase6.yml parses; exact 4-job set | python3 yaml.safe_load + set assert | jobs = {tier-9k, tier-r9, wave0-host, x86_64-poolpanic} → JOBS-OK | ✓ PASS |
| Action pins | `uses:` 40-char-SHA regex over all 11 | PINS-OK 11/11 | ✓ PASS |
| Ledger grammar lint | awk 8-field k=v grammar (comments/blanks skipped) over both ledgers | LEDGER-FORMAT-OK (72 / 154 lines) | ✓ PASS |
| Venue log census | grep -c over 06-09-tier-9k-local.log | 36 OK / 24 SKIP / 1 DONE / 0 FAIL / 0 panic-class | ✓ PASS |
| Venue blocks byte-identical | diff coverage-sweep.sh:569-578 vs dir-coverage.txt:20-29 | empty diff → VENUE-BLOCK-IDENTICAL | ✓ PASS |
| Tier file entry counts | grep -vc '^#' over both tier files | 35 / 62 (unchanged) | ✓ PASS |
| gethunk fixes + 6l LP64 fix presence | sed line checks | macbody:877 + obj.c:1321 `if(h == hunk + nhunk)`; asm.c:315-317 two-int emit | ✓ PASS |
| IN-01 fix shape | git show aed40c38d; sh -n + bash -n mkimage.sh | exactly 1 deletion (the dead dd line); 07-04 comment kept; parse-clean | ✓ PASS |
| IN-02 fix shape + exception audit | git show 8b22723c4; residual-bare-grep scan | exactly 11+/11- spelling changes; only the documented heredoc exception (:596) remains bare; GREP resolution intact at :177; parse-clean | ✓ PASS |
| IN-03/IN-04 fix shape | git show 6df5295e3 0b6be9e78; grep comment lines | comment-only: 41 bytes at :315; 70 + source-of-truth citation at :582-585; assert lists untouched | ✓ PASS |
| IN-05 fix shape | git show d0d186ccd; re-read :225-231 | find-based newest-`.c`/`.h` key with `[ ! -x … ]` short-circuit; parse-clean | ✓ PASS |
| IN-06 fix shape | git show d7e8f316e; re-read rootgen :148-158 | guarded panic promoted, tab-indented, brace-free, `%#llux` established usage; condition unreachable by construction; Nbootfiles=64 untouched | ✓ PASS |
| Debt-marker gate | TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER scan over the four fixed files | zero real hits (only mktemp `XXXXXX` template false-positives, same class as prior runs) | ✓ PASS |
| Staleness enumeration over the covered set | `git log 5e83da642..HEAD -- <52 prior covered paths + mkimage.sh>` | exactly 9 commits: the six fixes + b256db8fd/72521f59e/c4de736d7 — no other covered file moved | ✓ PASS |
| Venue boot / sweep execution | — | requires lima VM multi-minute qemu run; committed retained log + recorded local green + hosted green UAT stand as the marker-as-truth evidence; the IN fixes are linux-venue harness changes whose exercising gate is CI/lima (REVIEW-FIX.md verification venue note) | ? SKIP |
| tier-9k hosted run | — | DONE by the maintainer — UAT Test 1 PASS (all four legs green) | ✓ PASS (human) |

### Probe Execution

No `scripts/**/tests/probe-*.sh` exist and no plan declares probes — N/A.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| USPACE-01 | 06-01 | gethunk ×2 fixes with regression proof | ✓ SATISFIED | fixes in-tree (re-checked this run); repro scripts + committed TEETH-OK/GREEN log; IN-05 strengthens the 6l repro's staleness key (whole-tree watch) without touching its teeth or success path |
| USPACE-02 | 06-04, 06-10 | full-tree sweep reaches every cmd dir; coverage report | ✓ SATISFIED | dir-coverage.txt 221=206+15; venue block byte-identical generator/report re-diffed THIS RUN; no commit in this delta touches the report or the generator |
| USPACE-03 | 06-04 | generator audit + CI .7-magic red | ✓ SATISFIED | generator-audit.txt; teeth log; wave0-host wiring (all untouched by this delta) |
| USPACE-04 | 06-02/03/07/09/10 | tiered append-only allowlists + donor-first-then-r9 gates + marker truth + amended venue split | ✓ SATISFIED | tier files intact (35/62); ledgers lint-clean (re-run); donor→r9 committed logs; Harvey-kernel x86_64 venue locally green with teeth AND hosted green (UAT); retirement CI-asserted; IN-02 hardens the marker gate's host-portability (plan9-grep-proof) with zero shape change |
| USPACE-05 | 06-05 | 22 libs compile fresh via mksyslib fallback | ✓ SATISFIED | 35/35 T0-ART; .7-only members; honest ledger classes |
| USPACE-06 | 06-06/07/09 | core cmds + build chain compile + donor-execute | ✓ SATISFIED | 58 × T1-OK donor; selfhost canary observed output; 37 × T1-OK r9; 36 × T1-OK on Harvey's own kernel |
| USPACE-10 | 06-08 | 9k-amd64 pool-panic re-test with recorded verdict | ✓ SATISFIED | VERDICT=PASS recorded; teeth-proven classifier; CI leg asserts production and is hosted-green (UAT) |

All 7 phase requirement IDs accounted for in REQUIREMENTS.md (each mapped Phase 6, marked Complete). No orphaned requirements (no other IDs map to Phase 6; USPACE-07/08/09 map to Phase 10 as expected).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| (four fixed files re-scanned THIS RUN) | — | TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER | — | zero real hits — debt-marker gate clean (only mktemp `XXXXXX` random-suffix template false-positives) |
| ~~sys/src/9k/port/devroot.c~~ | ~~154-157~~ | ~~pre-existing debug `panic("whoops")` remnant (IN-06)~~ | ✅ FIXED | d7e8f316e: promoted to a tab-indented guarded `panic("rootgen: negative index %d (qid %#llux)", …)` — no longer a debug remnant; zero reachable-behavior change; disposition `fixed` in the reconciled ledger |
| ~~build/aarch64/mkimage.sh~~ | ~~272~~ | ~~dead dd trailer extraction (IN-01)~~ | ✅ FIXED | aed40c38d: single-line deletion; live od extraction untouched; disposition `fixed` |
| ~~build/aarch64/subset-exec.sh~~ | ~~11 sites~~ | ~~bare host greps bypassing the D-02 `$GREP` resolution (IN-02)~~ | ✅ FIXED | 8b22723c4: all eleven routed through `"$GREP"`; only the documented generated-script exception remains; disposition `fixed` |
| ~~build/aarch64/subset-exec.sh~~ | ~~:315/:583~~ | ~~comment count drift 40→41, 66→70 (IN-03/IN-04)~~ | ✅ FIXED | 6df5295e3 + 0b6be9e78: comments corrected; IN-04 now cites the assert list as source of truth; disposition `fixed` |
| ~~build/aarch64/gethunk-repro-6l.sh~~ | ~~:230~~ | ~~staleness key watched only obj.c (IN-05)~~ | ✅ FIXED | d0d186ccd: key now watches the newest `.c`/`.h` across sys/src/cmd/6l; disposition `fixed` |
| working tree | sys/src/cmd/** | local full-tree build dirt: untracked 6.out/7.out/cc.a6/cc.a7 products | ℹ️ Info | produced by local namespace builds (they bind-mount the host tree); CI gates run from clean checkouts and are hosted-green (UAT Test 1); nothing in the phase's committed artifact set is dirty (staleness enumeration this run) |
| (open warnings WR-03..WR-08) | — | latent hygiene (G10+ audit-row selector, tracesup bounds, dead mkimage line — NOW FIXED as IN-01, unanchored greps on the DONOR gate, sh/bashism in ledger-r9-append) | ⚠️ Warning | carried forward open in the reconciled ledger (open: 6 / total: 15, re-read this run) — they belong to the prior review's finding set, "(not in the current review)"; the 9k venue harness already implements WR-06's boundary-anchored shape; non-blocking. NOTE: WR-05's "dead mkimage line" subject overlaps the now-fixed IN-01; the ledger keeps the WR-05 row open per its carry-forward rule — a ledger-bookkeeping observation, not a code issue |

### Human Verification Required

**None.** The single historical human item — the first hosted run of the rewritten phase6.yml — was PASSED by the maintainer on 2026-11-05 (06-UAT.md Test 1, result: pass; G-06-1 resolved) and remains closed. The six IN fixes are comment/dead-code/idiom-class harness hygiene (plus one unreachable-path panic message in 9k kernel source whose exercising gate is the CI/linux-venue kernel build, per REVIEW-FIX.md's verification-venue note); they alter no marker semantics, no emitted-artifact bytes, and no truth's evidence chain — static + diff-scope verification here is sufficient. The human verification section is empty, so `passed` is the correct status per the decision tree.

### Gaps Summary

**No gaps.** All five ROADMAP success criteria and all 7 requirement IDs remain verified after the review-fix `--all` delta. The delta since the prior (stale) report is exactly nine commits touching the covered set — six `fix(06-review)` commits and three phase-doc ledger regenerations — plus inert interleaved Phase-08 work outside the covered paths. All six fixes were re-read line-by-line and diff-scoped: IN-01 dead-code only; IN-02 exactly 11 spelling-identical substitutions with the single documented exception; IN-03/IN-04 comment-only; IN-05 a strictly stronger rebuild predicate; IN-06 message-only on an unreachable path with the guard retained. The coverage report, tier files, and ledgers are untouched by construction; venue-block byte-identity re-diffed clean. The review ledger now reads 9 fixed / 6 open (WR-03..WR-08 carried, not in the current review) of 15 total; the deferred-items register is filed to Phase 10 and the ROADMAP backlog (Phase 999.1). covered_files grew to 53 with the addition of mkimage.sh (first touched by phase-06 work in aed40c38d, per the nsrun.sh precedent).

---

_Verified: 2026-11-06T00:22:18Z_
_Verifier: the agent (gsd-verifier)_
