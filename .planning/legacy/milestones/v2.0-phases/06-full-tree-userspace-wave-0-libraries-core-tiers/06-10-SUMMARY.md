---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 10
subsystem: infra
tags: [ci, github-actions, phase6, tier-venue, 9k, k10t1, coverage, g-06-1, cr-01, venue-split]

requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-09)
    provides: the Harvey-kernel tier venue pair (build/mm/build-9k10t1.sh + build/mm/boot-9k10t1-sweep.sh), locally green at f2543ec8e
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-08)
    provides: phase6.yml itself (sibling-workflow conventions, the SHA-pin set, the PENDING FIRST PUSH precedent) + the hosted linux-x86_64 jitter finding
provides:
  - "phase6.yml's tier-9k job — the x86_64 CI tier venue on Harvey's OWN kernel, structurally green-capable from a clean ubuntu-latest runner (every precondition self-provisioned in-job, failure digests in the step summary)"
  - "the 9front tier-donor CI leg retired and CI-asserted: wave0-host's venue-pin step reds on reintroduction (anchored job-key grep + self-hosting-safe donor-script ERE + tier-9k positive-grep)"
  - "the committed coverage report's venue line truthful under both linux host classes (reach/verdicts host-independent; visit counts disclosed as jitter, kept as data)"
  - "CR-01 resolved in the disposition record (fixed — closed by retirement); 06-EVIDENCE.md's 06-10 section (decision + validation matrix + regen record + PENDING FIRST PUSH)"
affects: [verify-work (G-06-1's final hosted-green confirmation after the next push), phase-10, 08-* (the venue-split direction: Harvey + r9 only)]

actuals:
  tokens: 6523
  tasks: 2
  commits: 3
  plan_head_before: 176375239e683def843f159edf872e5dc72ba9da
  plan_head_after: 6d053613bf939c3610811111cc15975e63b3437f

tech-stack:
  added: []
  patterns:
    - "venue retirement CI-asserted, not prose: an anchored negative-grep + a self-hosting-safe ERE (pattern text never matches itself) + a positive-grep on the replacement venue, riding the fast leg"
    - "clean-runner job self-provisioning: setup + toolchain + missing-tool assert loop before any build step, with the 02-12 BUILD-SWALLOWED-FAIL artifact assert + digest"
    - "measurement-noise disclosure wording: counts stay embedded as data, the normalized gate carries the tolerance"

key-files:
  created:
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-10-SUMMARY.md
  modified:
    - .github/workflows/phase6.yml (tier-9k job in, tier-donor job out, venue-pin step, header venue-split rewrite)
    - build/aarch64/coverage-sweep.sh (venue-block emission + header prose pointer)
    - build/aarch64/dir-coverage.txt (regenerated: new venue block byte-identical to the generator's; five visited= count flips, all build=ok unchanged)
    - build/aarch64/subset-exec.sh (venue-split header note; stale CI-ownership hint corrected)
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md (06-10 section)
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-REVIEW-DISPOSITION.md (CR-01 fixed)

key-decisions:
  - "tier-donor REMOVED, not kept red as a tripwire (maintainer direction 2026-11-05): a permanently-red leg trains red-blindness; the wave0-host venue-pin step is the honest tripwire"
  - "CR-01's provisioning arm (wire a native 9front donor build into CI) REJECTED: 9front on x86_64 CI is the deliberate anti-goal — donor evidence there comes from Harvey's own kernel (9k10cpu lineage, poolpanic-proven on the runner)"
  - "the jpg note line KEEPS its count embedding: normalization already carries the jitter tolerance; dropping data to quiet a diff the gate no longer trips on is the wrong direction"

patterns-established:
  - "venue-pin step: retirement asserted by grep in the fast leg (anchored job-key + ERE whose literal pattern text cannot self-match + required-venue positive grep)"
  - "PENDING FIRST PUSH honesty convention extended to a rewritten workflow leg: local validation matrix recorded, hosted observation explicitly deferred to the next push"

requirements-completed: [USPACE-02, USPACE-04]

coverage:
  - id: D1
    description: "phase6.yml wires the tier-9k venue (Harvey's own kernel) into CI and retires the 9front tier-donor leg: exact four-job set, all actions 40-char-SHA pinned, the retired leg absent by grep, the venue-pin step present and self-hosting-safe, the three green-proven jobs byte-preserved"
    requirement: USPACE-04
    verification:
      - kind: other
        ref: "local validation matrix: yaml.safe_load JOBS-OK {wave0-host,tier-9k,tier-r9,x86_64-poolpanic}; PINS-OK 11/11; venue-pin replays 0/0/1; referenced-path enumeration all-exist; byte-preservation diff review clean (tier-r9 679B, x86_64-poolpanic 3852B, wave0-host modulo the additive 1660B pin step) — commit a6ef4dbba"
        status: pass
    human_judgment: false
  - id: D2
    description: "the committed coverage report's venue claim is truthful under both linux host classes: the new venue block discloses count jitter, is byte-identical between generator emission and report, and the regenerated report's normalized drift-gate replay is a no-op"
    requirement: USPACE-02
    verification:
      - kind: integration
        ref: "nix develop -c sh build/aarch64/coverage-sweep.sh (lima guest, linux-aarch64) -> COVERAGE-SWEEP-OK (206 visited / 15 exclusions / universe 221); post-commit `git show HEAD:... | sed visited=N | diff` replay: empty — commit 0d7c1f537"
        status: pass
    human_judgment: false
  - id: D3
    description: "the tier-9k leg's first hosted run on ubuntu-latest (green-capable from a clean checkout)"
    requirement: USPACE-04
    verification: []
    human_judgment: true
    rationale: "PENDING FIRST PUSH per the 05-13/06-08 precedent — harvey master is never pushed from the executor session; no local substitute exists for a hosted runner. 06-EVIDENCE.md's 06-10 section records the honesty line; a human pushes and watches `gh run list --workflow phase6-userspace`."

duration: 8 min
completed: 2026-11-05
status: complete
---

# Phase 06 Plan 10: CI Venue Wiring + Donor-Leg Retirement Summary

**G-06-1 closed Harvey-side: phase6.yml's x86_64 tier evidence now comes from Harvey's OWN kernel — a new `tier-9k` job provisions a clean runner end-to-end (setup → toolchain → full amd64 build → k10t1 venue kernel → boot/sweep with self-printed digests), the 9front tier-donor leg is REMOVED per the maintainer's 2026-11-05 direction with a CI-asserted venue-pin tripwire against its return, CR-01 is resolved by retirement, and the coverage report's venue line now tells the truth about host-class count jitter.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-11-05T22:10:51Z
- **Completed:** 2026-11-05T22:19:08Z
- **Tasks:** 2
- **Files modified:** 6 (0 created besides this SUMMARY)

## Accomplishments

- **tier-9k job in phase6.yml** (`a6ef4dbba`): checkout → nix → setup/toolchain with the TOOLCHAIN-PRODUCTS-OK assert loop (the clean-runner class the first hosted run redded on) → full amd64 build with the BUILD-SWALLOWED-FAIL artifact assert + poolpanic-style digest (02-12 exit-0 class) → `build/mm/build-9k10t1.sh` → `build/mm/boot-9k10t1-sweep.sh` with failure digests captured to `$GITHUB_STEP_SUMMARY` (D-16's self-print convention) → venue logs uploaded `if: always()`. Structurally green-capable from a clean ubuntu-latest runner: every prerequisite is provisioned in-job.
- **tier-donor leg retired and pinned** (`a6ef4dbba`): the whole job block removed; the wave0-host venue-pin step negative-greps the retired job key (anchored) and the donor-gate script basenames (an ERE whose literal pattern text cannot match itself), and positive-greps the tier-9k key — a silent reintroduction reds the fast 45m leg. Header rewritten to the venue split: aarch64 tier gates stay LOCAL-only (9front donor pin, hand-driven from build/aarch64/); tier-r9 remains the second CI venue (D-12).
- **Byte-preservation proven**: tier-r9 (679B) and x86_64-poolpanic (3852B) job bodies byte-identical HEAD-vs-new; wave0-host byte-identical modulo the additive 1660B pin step; all 11 `uses:` pins 40-char SHAs; concurrency/permissions/triggers untouched.
- **Coverage venue wording + regen** (`0d7c1f537`): the venue block discloses the linux-x86_64 hosted finding — reach set + per-dir verdicts host-independent, visit COUNTS jitter as measurement noise with the normalized gate carrying the tolerance — byte-identical in generator emission and committed report. Regenerated on the lima guest (COVERAGE-SWEEP-OK, 206 visited / 15 exclusions / universe 221); raw diff = venue block + five visited= flips (all build=ok unchanged); the post-commit normalized gate replay is a no-op. The jpg note line keeps its count embedding as data.
- **Records** (`6d053613b`): subset-exec.sh's header carries the venue split and the stale "06-08's CI job owns the native donor build" hint is corrected (no provisioning driver exists for the native leg this era); CR-01 reads `fixed` with the retirement reason; 06-EVIDENCE.md's 06-10 section records the decision, the rejected alternatives, the validation matrix, the regen record, and the PENDING FIRST PUSH honesty line.

## Task Commits

Each task was committed atomically (Task 2 split per the plan's two-commit shape):

1. **Task 1: phase6.yml surgery** (tier-9k in, tier-donor out, venue-pin, header) — `a6ef4dbba` (feat)
2. **Task 2a: coverage venue wording** (generator + regenerated report) — `0d7c1f537` (fix)
3. **Task 2b: subset-exec header + CR-01 disposition + EVIDENCE record** — `6d053613b` (docs)

**Plan metadata:** the docs commit landing this SUMMARY + STATE/ROADMAP/REQUIREMENTS updates (hash in `git log`).

## Files Created/Modified

- `.github/workflows/phase6.yml` — tier-9k job added after tier-r9; tier-donor job removed; wave0-host gains the venue-pin step; header rewritten to the venue split
- `build/aarch64/coverage-sweep.sh` — venue-block emission wording + header prose pointer to the report's venue block
- `build/aarch64/dir-coverage.txt` — regenerated; new venue block byte-identical to the generator's
- `build/aarch64/subset-exec.sh` — venue-split header note; stale CI-ownership hint corrected
- `.planning/phases/06-*/06-EVIDENCE.md` — the 06-10 record section
- `.planning/phases/06-*/06-REVIEW-DISPOSITION.md` — CR-01 open → fixed

## Decisions Made

- **Removal over keep-red**: the maintainer's 2026-11-05 direction favors removal/replacement over 9front provisioning; a permanently-red leg trains red-blindness, so the venue-pin step is the tripwire. Recorded in the commit message, the EVIDENCE section, and the disposition row.
- **CR-01's provisioning arm rejected**: wiring a native 9front donor build into CI is the anti-goal (borrowing 9front-specific design decisions); the first hosted run also showed the arm's real cost — the leg redded earlier than CR-01's pin gate, inside the host toolchain bootstrap under nixpkgs gcc hardening flags.
- **jpg count embedding stays**: normalization carries the jitter tolerance; the count is information.

## Deviations from Plan

None - plan executed exactly as written. (The regen's five visited= count flips are the disclosed jitter class the plan predicts — "only counts may jitter" — not a deviation.)

## Issues Encountered

None. The lima guest was already running; the regen greened on the first pass; every acceptance criterion passed without a fix cycle.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Phase 06 complete Harvey-side**: all 10 plans have SUMMARYs. G-06-1's final hosted-green confirmation lands at the verify-work re-run after the next push — the tier-9k leg's first hosted observation is PENDING FIRST PUSH (05-13/06-08 precedent); a human pushes and watches `gh run list --workflow phase6-userspace`.
- **No blockers.** The venue-pin step makes the retirement load-bearing: any future edit reintroducing a hosted 9front dependency reds wave0-host in seconds.

## Self-Check: PASSED

- Files exist: `.github/workflows/phase6.yml` ✓ (4-job set) `build/aarch64/coverage-sweep.sh` ✓ `build/aarch64/dir-coverage.txt` ✓ `build/aarch64/subset-exec.sh` ✓ `06-EVIDENCE.md` ✓ (06-10 section at line 780) `06-REVIEW-DISPOSITION.md` ✓ (CR-01 fixed) — verified on disk
- Commits exist: `a6ef4dbba` `0d7c1f537` `6d053613b` (all in `git log`)
- Acceptance battery re-run at the final committed state: JOBS-OK / PINS-OK 11 / venue-pin replays 0-0-1 / normalized gate replay no-op / venue blocks byte-identical / stale hint absent / CR-01 fixed line count 1 ✓

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
