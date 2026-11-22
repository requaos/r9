---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 08
subsystem: infra
tags: [uspace-10, d-03, d-04, d-16, ci, github-actions, qemu, kvm, tcg, pool-panic, verdict-harness, supply-chain, sha-pins, 9k, amd64, x86_64]

# Dependency graph
requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-01..06-07)
    provides: every gate the CI workflow encodes — the gethunk ×2 archive-mode repros, the coverage sweep + committed report, the generator scan, the tier-0/1 donor gate (subset-build/subset-exec native-linux venue), the boot-r9-tier1 flake app
  - phase: 05-shell-boot-one-command-integration (05-13)
    provides: r9-shell.yml's proven CI shape + the no-push substitution precedent (author + locally validate; hosted run PENDING FIRST PUSH)
  - phase: v1.0 01-build-tail-green-9k-amd64 (01-04) + 02-aarch64-toolchain (pswt fix 7efff3aa3)
    provides: the recorded bad-magic pool-panic signature set (deferred-items #2) and the fixed 6c the re-test exercises
provides:
  - ".github/workflows/phase6.yml — the phase-6 full-pipeline CI (D-16): wave0-host (repros + coverage regen/diff + generator scan + hygiene + FPdbleword), tier-donor, tier-r9, x86_64-poolpanic; every action 40-char-SHA pinned; concurrency group; siblings byte-untouched"
  - "build/mm/boot-9k10cpu-qemu.sh — the minimega-free (D-03) plain-qemu serial capture + whole-log verdict classifier; exit 0 iff a verdict was PRODUCED, never gated on PASS; empty capture → FAIL no-banner + non-zero"
  - "THE USPACE-10 VERDICT (local TCG): VERDICT=PASS no-panic-signature boot-continues — the pswt-fixed 6c boots 9k10cpu PAST the 01-04 pool-panic point; closed WITH INFORMATION"
  - "evidence/poolpanic-verdict.md + evidence/06-08-poolpanic-serial.log — the committed verdict record + retained capture; the hosted KVM-classified leg PENDING FIRST PUSH"
affects: [verify-phase (USPACE-10 evidence is consolidated), milestone completion (ROADMAP SC#1's second half), the first-push hosted run of phase6-userspace]

# Actuals (#2632) — chars/4 over the realized diff (130534 chars ≈ 32633).
# The evidence logs (~90KB) dominate; the authored workflow+script+docs diff
# is ~40KB (≈10k tokens) vs the 50k low-confidence estimate.
actuals:
  tokens: 32633
  tasks: 2
  commits: 6   # MEASURED per #3968: git rev-list --count 3003b050c..09b4fd21d

# Plan-commit ledger (#3968) — measured
commits: 6
plan_head_before: 3003b050cd026bec94a4c3b5c8ad539e7d05a70d
plan_head_after: 09b4fd21de39145ad1fdb71775bf98aaef9c63e7 (pre-metadata; the docs commit rides on top)

tech-stack:
  added: []   # zero new packages (T-06-16's letter): the only new external references are SHA-pinned GH actions (checkout/upload-artifact/nix-installer)
  patterns:
    - "verdict harness contract: capture truncated before launch; whole-log classification (never a post-banner tail); verdict file written EITHER WAY; exit 0 iff verdict PRODUCED (banner + classification), never gated on PASS; empty capture → FAIL no-banner + non-zero (the leg never greens on an empty log)"
    - "healthy continuation markers key on the production binary's ACTUAL output vocabulary ('root is from', boot.c:342), verified against the compiled config — not source prints that are DEBUG-gated or post-connection"
    - "CI classification legs probe accel exactly once per run (/dev/kvm writable → kvm else tcg/-cpu qemu64 + 600s budgets) and record the path taken in the verdict — identical assertion sets on both"
    - "teeth-proving a verdict harness: canned-log classify-only runs (incident/empty/healthy/production shapes) + a deliberately-wrong-image real-qemu run — every classification path exercised with recorded exits"

key-files:
  created:
    - .github/workflows/phase6.yml                 # the four-job phase CI (D-16 + D-04)
    - build/mm/boot-9k10cpu-qemu.sh                # the plain-qemu verdict harness (USPACE-10)
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/poolpanic-verdict.md
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-08-poolpanic-serial.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-08-gethunk-verify.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-08-generator-scan.log
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-08-coverage-log.txt
  modified:
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md   # Task-1 (workflow + local validation matrix + PENDING FIRST PUSH) and Task-2 (USPACE-10 verdict) sections

key-decisions:
  - "06-08: phase6.yml is a NEW sibling workflow (r9-shell.yml + harvey.yml byte-untouched — verified by explicit-range git diff over the plan's commits) carrying the full phase-6 pipeline on linux legs (D-16); EVERY action 40-char-SHA pinned incl. actions/checkout (11d5960a…, API-resolved) and upload-artifact (ea165f8d…, API-resolved 2026-11-05); concurrency group phase6-<ref> cancels superseded runs (T-06-18)"
  - "06-08: USPACE-10 verdict = PASS no-panic-signature boot-continues (local TCG): the pswt-fixed 6c boots 9k10cpu past the 01-04 bad-magic pool panic — banner at 5s, boot.c's root-is-from prompt reached, zero signature lines; the Phase-2 hypothesis (same root cause as the donor's panic) CONFIRMED; hosted KVM-classified leg PENDING FIRST PUSH (05-13 precedent, nothing fabricated)"
  - "06-08: the verdict harness asserts verdict PRODUCTION (banner + classification file), never the outcome — a FAIL verdict is a successful job (information delivered); healthy markers corrected to the production boot's reachable prints (Rule 1)"

patterns-established:
  - "recorded-verdict CI leg: the job's run step asserts the verdict FILE + VERDICT= line; artifacts upload if: always(); a FAIL verdict greens, no verdict reds"
  - "no-push CI validation (05-13 precedent applied): author + parse-check + run the encoded commands locally; record the hosted leg as PENDING FIRST PUSH in EVIDENCE.md"

requirements-completed: [USPACE-10]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: ".github/workflows/phase6.yml authored: four jobs (wave0-host 45m, tier-donor 90m, tier-r9 90m, x86_64-poolpanic 45m), on [push, pull_request], permissions contents: read, concurrency group phase6-<ref> cancel-in-progress, every referenced script/app path verified in-tree, the coverage step regenerates in place + git diff --exit-code (D-15)"
    verification:
      - kind: unit
        ref: "python3 yaml.safe_load job assertions (JOBS-OK, all four jobs) + pin grep PINS-OK 11/11 + per-path existence enumeration"
        status: pass
    human_judgment: false
  - id: D2
    description: "wave0-host's encoded commands green locally on darwin-aarch64: gethunk-repro-cc archive+verify 4/4 (TEETH-OK from PREREF + GREEN fixed), gethunk-repro-6l 2/2, coverage regen via lima COVERAGE-SWEEP-OK + report byte-identical (COVERAGE-DIFF-CLEAN), generator-scan GENERATOR-SCAN-OK, GREP-HYGIENE-OK, FPDBL-OK"
    verification:
      - kind: integration
        ref: "evidence/06-08-gethunk-verify.log (TAP 4/4 + 2/2), evidence/06-08-generator-scan.log, evidence/06-08-coverage-log.txt + git diff --exit-code clean"
        status: pass
    human_judgment: false
  - id: D3
    description: "USPACE-10: the verdict harness proven (five teeth runs incl. FAIL-with-exit-0 production-not-outcome + wrong-image real-qemu no-banner exit 1) and the LOCAL verdict produced: VERDICT=PASS no-panic-signature boot-continues (TCG) — recorded either-way per D-04"
    requirement: USPACE-10
    verification:
      - kind: e2e
        ref: "sh build/mm/boot-9k10cpu-qemu.sh exit 0 + grep '^VERDICT=' .build/poolpanic-verdict.txt; committed record evidence/poolpanic-verdict.md + evidence/06-08-poolpanic-serial.log (zero signature lines)"
        status: pass
    human_judgment: false
  - id: D4
    description: "a hosted GitHub run of phase6-userspace going green on ubuntu-latest (incl. the KVM-classified x86_64-poolpanic verdict and the D-15 coverage diff on the linux-x86_64 host class)"
    requirement: USPACE-10
    verification: []
    human_judgment: true
    rationale: "harvey master push is prohibited this session (orchestrator rule), so no hosted run exists to observe. The workflow is authored and locally validated (D1/D2) and the verdict contract is proven end-to-end locally (D3); the first hosted run happens at first push (05-13 substitution precedent — recorded PENDING FIRST PUSH in 06-EVIDENCE.md and evidence/poolpanic-verdict.md, nothing fabricated). A human pushes and watches gh run list --workflow phase6-userspace."

# Metrics
duration: 4h 8m
completed: 2026-11-05
status: complete
---

# Phase 6 Plan 08: Full-Pipeline CI + the USPACE-10 Pool-Panic Verdict Summary

**`.github/workflows/phase6.yml` carries every gate phase 6 built (D-16: wave0-host repros/coverage-diff/generator-scan/hygiene/FPdbleword, tier-donor, tier-r9) plus the new `x86_64-poolpanic` leg, all actions 40-char-SHA-pinned with siblings byte-untouched — and the USPACE-10 re-test has its verdict: the pswt-fixed 6c boots 9k10cpu PAST the 01-04 bad-magic pool panic (`VERDICT=PASS no-panic-signature boot-continues`, local TCG, whole-log classified, recorded either-way per D-04); the hosted KVM-classified run is honestly PENDING FIRST PUSH.**

## Performance

- **Duration:** 4h 8m (context assembly + the coverage regen + two TCG boots dominate)
- **Started:** 2026-11-05T08:49:16Z
- **Completed:** 2026-11-05T12:57:18Z
- **Tasks:** 2
- **Files modified:** 8 (2 authored code/config + 6 evidence/doc files)

## Accomplishments

- **D-16 encoded:** every phase-6 gate runs in CI on ubuntu-latest — `wave0-host` (toolchain → gethunk ×2 archive-mode teeth+verify → coverage report regenerated IN PLACE + `git diff --exit-code` → generator `.7`-magic scan → grep-hygiene gate → FPdbleword assert), `tier-donor` (native-linux venue: nsrun sudo-unshare, donor pin 350266104f host-side, TCG aarch64), `tier-r9` (`nix run .#boot-r9-tier1`), `x86_64-poolpanic`.
- **Supply chain held (T-06-16):** PINS-OK 11/11 — actions/checkout `11d5960a…` (v4) and actions/upload-artifact `ea165f8d…` (v4) resolved via the GitHub API at authoring; nix-installer-action `3138316d…` (the 05-13 pin); `permissions: contents: read`; `concurrency: phase6-<ref>` cancels superseded runs (T-06-18).  r9-shell.yml + harvey.yml byte-untouched (explicit-range diff over the plan's commits: only phase6.yml).
- **USPACE-10 closed WITH INFORMATION:** the v1.0 carry-over re-test (deferred-items 02-8) has its verdict — the kernel built with the pswt-fixed 6c boots past the exact line where 01-04 died (`508M memory…` → boot.c's `root is from (tcp)[tcp]:` method prompt); ZERO `bad magic`/`panic`/`suicide: sys: trap` lines in the whole capture.  The Phase-2 audit's hypothesis (same pswt root cause as the donor's panic) is CONFIRMED.
- **The verdict contract is teeth-proven:** five recorded teeth runs — incident-shaped log → `FAIL pool-panic-signature` exit 0 (production-not-outcome); empty log → `FAIL no-banner` exit 1 (never greens on an empty capture); healthy logs (both vocabularies) → `PASS boot-continues` exit 0; a deliberately wrong image through REAL qemu → `FAIL no-banner` exit 1.
- **Local validation matrix recorded** in 06-EVIDENCE.md with retained logs; the hosted run is recorded PENDING FIRST PUSH (05-13 precedent — nothing fabricated).

## Task Commits

1. **Task 1 (ci):** `9dc202dd6` — ci(06-08): phase6 workflow — wave0-host, tier-donor, tier-r9 jobs (D-16)
2. **Task 1 (docs):** `79b1fd5a1` — docs(06-08): wave0-host local validation evidence + EVIDENCE record (PENDING FIRST PUSH)
3. **Task 2 (feat):** `30d9d5cd2` — feat(06-08): plain-qemu 9k10cpu verdict harness (USPACE-10, D-03/D-04)
4. **Task 2 (ci):** `c8f542246` — ci(06-08): x86_64-poolpanic job (USPACE-10)
5. **Task 2 (fix, Rule-1 deviation):** `cbdee1f87` — fix(06-08): verdict harness healthy markers — the production boot's actual vocabulary
6. **Task 2 (docs):** `09b4fd21d` — docs(06-08): USPACE-10 verdict record + EVIDENCE

**Plan metadata:** the final docs commit rides on top (SUMMARY + STATE/ROADMAP/REQUIREMENTS).

## Files Created/Modified

- `.github/workflows/phase6.yml` — the four-job phase CI (152 lines pre-job-4, 209 after)
- `build/mm/boot-9k10cpu-qemu.sh` — the plain-qemu verdict harness (accel probe, truncated-before-launch `-serial file:` capture, deadline banner wait, whole-log classification, verdict file, PP_CLASSIFY_ONLY teeth hook)
- `.planning/phases/06-*/evidence/poolpanic-verdict.md` — the committed USPACE-10 record (origin, harness contract, teeth, the verdict, PENDING FIRST PUSH)
- `.planning/phases/06-*/evidence/06-08-poolpanic-serial.log` — the retained run-2 capture
- `.planning/phases/06-*/evidence/06-08-gethunk-verify.log`, `06-08-generator-scan.log`, `06-08-coverage-log.txt` — the wave0-host local validation logs
- `.planning/phases/06-*/06-EVIDENCE.md` — the Task-1 + Task-2 sections (append-only)

## Decisions Made

- **A NEW sibling workflow** (per the plan/RESEARCH recommendation): r9-shell.yml and harvey.yml stay byte-untouched; the phase-6 pipeline lives in phase6.yml with the supply-chain upgrades (SHA-pinned checkout + upload-artifact + concurrency group) the older file lacks.
- **The verdict harness asserts PRODUCTION, never outcome** (D-04's letter): exit 0 iff banner + classification written; a FAIL verdict is a green job delivering information; no-banner (incl. empty capture) records FAIL no-banner and reds the leg.
- **Healthy continuation markers key on the production boot's reachable prints** (the Rule-1 fix below).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The plan-named healthy continuation markers are unreachable in the production build**
- **Found during:** Task 2's first local verdict run — the boot was visibly healthy (the `root is from (tcp)[tcp]:` prompt loop, past the panic point) but classified `banner-only` (healthy_markers=no)
- **Issue:** the plan named boot.c's `argc=` (line 69) and `version...` (line 139) as the healthy markers; `argc=` sits inside `#ifdef DEBUG` (boot.c:68-73 — compiled out in the production config) and `version...` prints only after a rootfs connection (boot.c:139, connectroot), which a capture-only serial never establishes
- **Fix:** corrected the marker set to the production binary's actual post-panic-point vocabulary — primary `root is from` (boot.c:342's method prompt, which the panicked 01-04 boot never reached), with `version...`/`argc=` retained as recognized alternatives; updated both `classify_log` and the observation window's early-break; re-ran the teeth (including a new production-shape case) and a FRESH TCG boot
- **Files modified:** build/mm/boot-9k10cpu-qemu.sh
- **Verification:** teeth A/B/C/E green; the re-run produced `VERDICT=PASS no-panic-signature boot-continues` (healthy_markers=yes); run-1's banner-only verdict retained in the evidence record (superseded, never silently rewritten)
- **Committed in:** cbdee1f87

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug)
**Impact on plan:** the deviation is the harness doing its job — the marker vocabulary was corrected against the compiled config so the classification means what D-04 intends; no scope creep, no architectural change.

## Issues Encountered

- **None environmental.**  The only substantive find was the marker-vocabulary bug above (caught precisely because the harness records its reasoning fields — `healthy_markers=no` on a visibly healthy boot was the tell).  TCG x86_64-on-aarch64 reached the banner in ~5s both runs — far inside the 600s budget.

## Authentication Gates

None — no external service auth was needed (GitHub API tag resolution is unauthenticated; no push performed).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Phase 6's evidence chain is complete:** every plan's record is in 06-EVIDENCE.md; USPACE-10's verdict is committed (the phase's last outstanding requirement).
- **ROADMAP SC#1's second half satisfiable:** the full pipeline is CI-encoded (D-16) and locally validated; the hosted first push is the only outstanding leg — PENDING FIRST PUSH, recorded honestly.
- **Hosted-run watch items (for the first push):** (a) the D-15 coverage diff runs on the linux-x86_64 host class (the committed report's venue claim is linux-aarch64 — a flipped row reds BY DESIGN; the report header's re-baseline rule applies); (b) the x86_64-poolpanic leg's KVM path engages on the runner (the TCG fallback is in-harness).
- **v1.0 deferred item 02-8 is CLOSED** by the recorded verdict (the re-test ran; PASS).  STATE.md's blocker line ("9k-amd64 post-banner bad-magic pool panic — re-test with the fixed 6c") can be struck at phase verification.

## Self-Check: PASSED

- [x] `.github/workflows/phase6.yml` exists with all four jobs, concurrency group, `permissions: contents: read`, PINS-OK 11/11 — committed (9dc202dd6, c8f542246)
- [x] `build/mm/boot-9k10cpu-qemu.sh` exists; never invokes minimega/.mm (comment-only mentions verified); teeth-proven — committed (30d9d5cd2, cbdee1f87)
- [x] `evidence/poolpanic-verdict.md` (the verdict record) + `evidence/06-08-poolpanic-serial.log` (the retained capture) — committed (09b4fd21d)
- [x] Task-1 verify gates: JOBS-OK, PINS-OK 8/8 (3-job) → 11/11 (4-job), gethunk cc+6l archive/verify green, generator-scan GENERATOR-SCAN-OK
- [x] Task-2 verify gates: `sh build/mm/boot-9k10cpu-qemu.sh` exit 0 (verdict produced), `grep -q '^VERDICT=' .build/poolpanic-verdict.txt` green, JOB-OK (x86_64-poolpanic)
- [x] wave0-host commands green locally: coverage regen COVERAGE-SWEEP-OK + `git diff --exit-code build/aarch64/dir-coverage.txt` CLEAN; GREP-HYGIENE-OK; FPDBL-OK
- [x] Explicit-range `git diff --name-only 3003b050c..09b4fd21d -- .github/workflows/` → ONLY phase6.yml (r9-shell.yml + harvey.yml untouched)
- [x] USPACE-10 verdict produced and recorded either-way (PASS, with the signature set + accel + commit fields); hosted leg recorded PENDING FIRST PUSH — nothing fabricated
- [x] All six commits found in git log (3003b050c..09b4fd21d); no pre-existing dirty-tree files staged (explicit path lists throughout; the untracked 06-PATTERNS.md, .planning/graphs/, state.json, tmp/, compiled binaries, and the postscript/troff/Kill/config.json edits were never touched)

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
