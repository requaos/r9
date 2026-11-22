---
phase: 261105-slx
plan: 01
subsystem: infra
tags: [ci, github-actions, nix, supply-chain, sha-pins, backlog-999-1]

requires:
  - phase: 06-full-tree-userspace (06-08)
    provides: phase6.yml — the proven self-provisioned nix CI shape (linux legs), the checkout SHA pin, the tier-9k toolchain/build steps copied here
  - phase: 05-shell-boot-one-command-integration (05-13)
    provides: r9-shell.yml — the nix-installer-action SHA pin (3138316d…) + the PENDING FIRST PUSH precedent for hosted runs
provides:
  - ".github/workflows/harvey.yml rewritten onto the repo's own nix pipeline (install nix → .#setup → .#toolchain → .#build) on ubuntu-latest — the base clean-checkout build gate is green-capable moving forward"
  - "02-12 exit-0 class closed in-job: toolchain products (pcc/cpp/ar/6c/6l/8c/8l) and build artifacts (amd64/9k10cpu.elf, amd64/bin/rc, amd64/bin/awk) asserted — a swallowed build failure can never green the job"
  - "backlog 999.1 Test 1 + the PROJECT.md carried-over line ticked with the PENDING FIRST PUSH caveat"
affects:
  - ci-signal: every push/PR now runs the nix pipeline as the base build gate (workflow name `harvey` unchanged — checks/badge identity stable)

actuals:
  tokens: 2105      # chars/4 over the realized diff (harvey.yml commit 6343 + bookkeeping 2079 chars)
  tasks: 2
  commits: 2        # c018b85d3 (workflow rewrite) + the bookkeeping commit (ROADMAP/PROJECT); SUMMARY/PLAN docs commit rides with the orchestrator per quick-task constraints

tech-stack:
  added: []
  patterns:
    - "CI green means built: artifact asserts after every nix app whose script tails swallow exit status (the 02-12 class) — copied verbatim from phase6.yml's tier-9k leg"
    - "Every action 40-char SHA pinned; zero NEW external references — both pins were already proven in-repo (phase6.yml header + r9-shell.yml), so nothing needed API resolution at execution time"

key-files:
  created:
    - ".planning/quick/261105-slx-fix-the-legacy-harvey-yml-workflow-to-bu/261105-slx-SUMMARY.md (this file)"
  modified:
    - ".github/workflows/harvey.yml (full replacement, 96 lines, 2 pinned actions, 1 job × 4 steps)"
    - ".planning/ROADMAP.md (scoped checkbox tick: backlog 999.1 Test 1)"
    - ".planning/PROJECT.md (scoped checkbox tick: the carried-over line)"

key-decisions:
  - "FULL REPLACEMENT, no legacy leg kept: (1) the red streak's fragility IS the harness — EOL-Ubuntu package sources plus a PPA, a vanity-URL go-get fetch, an old-branch Go source build for the plan9 target, and an interactive-automation-driven 9legacy ISO boot under qemu x86 running an in-guest build; repairing it means re-owning exactly that fragility; (2) the signal it provided (does the repo build on push/PR) is fully covered by the nix pipeline, the repo's actual supported build path, already proven on ubuntu-latest by phase6.yml's wave0-host/tier-9k/x86_64-poolpanic legs; (3) a retained-but-red legacy leg trains red-blindness (the 06-10 tier-donor retirement lesson); (4) the in-guest ISO build exercised the 9legacy toolchain more than this repo's current source — the signal was stale even when green"
  - "The build/ harness scripts (build.bash, the guest-driving command script, the .expect files) stay in-tree untouched — usable locally; only CI stops invoking them"
  - "Job id `build` and workflow name `harvey` kept — stable checks/badge identity across the replacement"

patterns-established: []

requirements-completed: ["261105-slx"]

coverage:
  - id: D1
    description: "harvey.yml runs the repo's own nix pipeline (nix run .#setup → .#toolchain → .#build) on ubuntu-latest from a clean checkout — legacy harness fully replaced; both actions 40-char SHA pinned; permissions contents:read; concurrency group harvey-<ref>; toolchain-product + build-artifact asserts make green mean built"
    requirement: "261105-slx"
    verification:
      - kind: integration
        ref: "PyYAML structural gate + comment-filtered legacy-residue grep + flake-app grep + neighbor byte-identity checks (HARVEY-YML-STRUCTURE-OK / LEGACY-RESIDUE-OK / FLAKE-APPS-OK / NEIGHBORS-OK)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Backlog 999.1 Test 1 (ROADMAP.md) + the PROJECT.md carried-over line ticked with the PENDING FIRST PUSH caveat"
    requirement: "261105-slx"
    verification:
      - kind: other
        ref: "Task-2 grep gate (BOOKKEEPING-OK) over .planning/ROADMAP.md + .planning/PROJECT.md"
        status: pass
    human_judgment: false
  - id: D3
    description: "Hosted GitHub run of the rewritten harvey workflow is green on ubuntu-latest"
    requirement: "261105-slx"
    verification: []
    human_judgment: true
    rationale: "The hosted leg is PENDING FIRST PUSH per the 05-13/06-08/06-10 precedent — the executor session never pushes harvey master; a human pushes and watches `gh run list --workflow harvey`. Nothing fabricated: only local validations actually run are recorded here."

duration: 8 min
completed: 2026-11-06
status: complete
---

# Phase 261105-slx Plan 01: Fix the legacy harvey.yml workflow to build via nix Summary

**Legacy harvey.yml CI harness fully replaced by the repo's own nix pipeline (install nix → .#setup → .#toolchain → .#build, products + artifacts asserted) on ubuntu-latest — backlog 999.1 ticked; local gates green; the hosted run is PENDING FIRST PUSH.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-11-06T01:41:17Z
- **Completed:** 2026-11-06
- **Tasks:** 2/2
- **Files modified:** 3 (`.github/workflows/harvey.yml`, `.planning/ROADMAP.md`, `.planning/PROJECT.md`) + 1 created (this SUMMARY)

## Accomplishments

- `.github/workflows/harvey.yml` rewritten wholesale (33 → 96 lines): one `build` job on `ubuntu-latest` (timeout 60) with four steps — pinned `actions/checkout`, pinned `DeterminateSystems/nix-installer-action`, host bin + toolchain provisioning with the `pcc cpp ar 6c 6l 8c 8l` product assert (`TOOLCHAIN-PRODUCTS-OK`), and the amd64 userspace + kernels build with the artifact assert (`amd64/9k10cpu.elf`, `amd64/bin/rc`, `amd64/bin/awk`; `BUILD-SWALLOWED-FAIL` digest + `BUILD-ARTIFACTS-OK` on success). Workflow `name: harvey`, `on: [push, pull_request]`, and job id `build` unchanged — checks/badge identity stable.
- Supply chain: exactly two `uses:` references, both 40-char SHA pins already proven in-repo — checkout `11d5960a…` (the phase6.yml pin, API-resolved 2026-11-05) and nix-installer-action `3138316d…` (the 05-13 pin). Zero NEW external references; zero package-manager installs remain (the whole apt/PPA/go-get chain is deleted). `permissions: contents: read`; `concurrency` group `harvey-${{ github.ref }}` cancels superseded runs (the T-06-18 pattern).
- The 02-12 exit-0 class is closed in-job: `.#toolchain`'s trailing echo and `.#build`'s break-glass tail both report exit 0 on failure, so the products/artifacts are asserted and a swallowed failure prints the diagnostic digest (bind-error context, missing-make targets, `ls amd64/bin/`, first-failure line context, last-30-lines tail) and exits 1.
- Bookkeeping: ROADMAP.md backlog 999.1 Test 1 and the PROJECT.md carried-over line ticked `[x]` with the suffix `— workflow rewritten onto the nix pipeline 2026-11-05; hosted green run PENDING FIRST PUSH (261105-slx)`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite harvey.yml onto the nix pipeline (full replacement)** — `c018b85d3` (ci) — `.github/workflows/harvey.yml`, +96/−25
2. **Task 2: Bookkeeping (ROADMAP + PROJECT ticks)** — see below (docs) — the two scoped checkbox edits only

**Plan metadata + SUMMARY:** committed by the orchestrator per the quick-task spawn constraints (docs artifacts are never committed by the executor).

## Gate evidence (the local validation matrix — actually run, in order)

```
HARVEY-YML-STRUCTURE-OK jobs={build} pins=2/2 apps=3 artifacts=3
LEGACY-RESIDUE-OK
FLAKE-APPS-OK (setup toolchain build)
NEIGHBORS-OK
BOOKKEEPING-OK
```

- **HARVEY-YML-STRUCTURE-OK** — PyYAML `safe_load` structural gate: `on: [push, pull_request]` (YAML 1.1 `True`-key quirk handled by the assert as written), `name: harvey`, `permissions: {contents: read}`, concurrency group `harvey-${{ github.ref }}` + `cancel-in-progress: true`, exactly one job `build` (no `defaults` at job or top level), `runs-on: ubuntu-latest`, `timeout-minutes: 60`, exactly two `uses:` references both 40-char lowercase-hex SHA pins in the mandated order, all three `nix run .#setup/.#toolchain/.#build` invocations present, all three artifacts + the `TOOLCHAIN-PRODUCTS-OK`/`BUILD-SWALLOWED-FAIL`/`BUILD-ARTIFACTS-OK` markers present.
- **LEGACY-RESIDUE-OK** — comment-filtered negative grep: zero `apt-get|add-apt-repository|qemu-system-i386|go1.16|cmd/ufs|checkout@v<digit>|command.bash` tokens in any non-comment line (the retired pieces are named descriptively in the header comment only, as the plan mandates).
- **FLAKE-APPS-OK** — `setup = mkApp`, `toolchain = mkApp`, `build = mkApp` all present in flake.nix (lines 132-134) — the workflow's `nix run` targets resolve to real flake apps.
- **NEIGHBORS-OK** — `.github/workflows/r9-shell.yml` and `.github/workflows/phase6.yml` byte-untouched: `git diff --quiet HEAD` clean + empty porcelain on both paths.
- **BOOKKEEPING-OK** — Task-2 grep gate: both checkboxes ticked with the `PENDING FIRST PUSH (261105-slx)` suffix; this SUMMARY records the pending hosted run.

## The hosted run — PENDING FIRST PUSH

The hosted GitHub Actions leg is explicitly OUT of the executor gate per the 05-13/06-08/06-10 precedent: the executor session never pushes harvey master. **A human pushes and watches `gh run list --workflow harvey`.** Until that first push, the hosted run is recorded as PENDING FIRST PUSH — nothing fabricated; only the local validations above were actually run. The pre-existing master red streak (since 2026-11-04T12:01Z) belongs to the retired harness this plan deletes; the first push of this commit is also the first chance for the `harvey` check to go green.

## Decisions Made

- **Full replacement, no legacy leg kept** (the plan's deliberate legacy-leg decision, executed as written): (1) the red streak's fragility IS the harness — EOL-Ubuntu package sources plus a PPA, a vanity-URL go-get fetch, an old-branch Go source build for the plan9 target, and an interactive-automation-driven 9legacy ISO boot under qemu x86 running an in-guest build; "repairing" it means re-owning exactly that fragility; (2) the signal it provided ("does the repo build on push/PR") is fully covered by the nix pipeline — the repo's actual supported build path, already proven on ubuntu-latest by phase6.yml's wave0-host / tier-9k / x86_64-poolpanic legs; (3) a retained-but-red legacy leg trains red-blindness (the 06-10 tier-donor retirement lesson); (4) the in-guest ISO build exercised the 9legacy toolchain more than this repo's current source — the signal was stale even when green.
- The build/ harness scripts stay in-tree untouched (usable locally); only CI stops invoking them.
- No upload-artifact step, no fetch-depth override, no third action, no darwin leg — CI-green means linux only (the r9-shell.yml/phase6.yml precedent); the deep-evidence legs live in phase6.yml.
- No actionlint or other validation tool installed (T-SLX-SC: zero new installs) — PyYAML is the validator.

## Deviations from Plan

**1. [Spawn-constraint — commit shape] Two executor commits instead of the plan's one combined commit**
- **Found during:** Task 2 commit step.
- **Issue:** the plan's Task 2 specifies a single commit carrying harvey.yml + ROADMAP.md + PROJECT.md + PLAN.md + SUMMARY.md; the orchestrator's spawn constraints forbid the executor committing docs artifacts (SUMMARY.md/PLAN.md — the orchestrator commits docs afterward) and require atomic per-task code commits.
- **Fix:** Task 1 committed alone (`c018b85d3`, code change only); Task 2 committed as the bookkeeping-only commit (ROADMAP.md + PROJECT.md); PLAN.md + SUMMARY.md left for the orchestrator's docs commit. Nothing else changed.
- **Files modified:** none beyond the plan's set.
- **Verification:** `git show --name-only` per commit matches the intended set exactly.

**Impact on plan:** none — same content, same files, split to satisfy the quick-task commit contract.

## Issues Encountered

None — the precondition (PyYAML importable; flake apps present) held, both copy-source steps in phase6.yml matched the plan's line references, and every gate passed on first run.

## Known Stubs

None — no empty-value stubs, placeholders, or skipped tests introduced; every `<verify>` gate ran green. The only unrun leg is the hosted GitHub run, recorded as PENDING FIRST PUSH by design (D3, human_judgment: true).

## Threat Flags

None — no new network endpoints, auth paths, file-access patterns, or schema changes at trust boundaries. The threat register's four mitigations landed as planned: T-SLX-01 (both pins 40-char SHA, gate-asserted 2/2), T-SLX-02 (artifact/product asserts close the 02-12 swallowed-failure class in-job), T-SLX-03 (concurrency group cancels superseded runs), T-SLX-SC (zero package-manager installs remain; zero new external references).

## Next Phase Readiness

- The base "clean checkout builds" gate is green-capable on ubuntu-latest moving forward; the first master push carrying `c018b85d3` is the first hosted run of the rewritten workflow.
- Follow-up (human): push, then `gh run list --workflow harvey` — record the first green run against this SUMMARY's D3.

---
*Phase: 261105-slx*
*Completed: 2026-11-06*

## Self-Check: PASSED

- `.github/workflows/harvey.yml` FOUND (rewritten, 96 lines).
- `261105-slx-SUMMARY.md` FOUND at the plan's output path.
- `c018b85d3` FOUND — commit contents verified: exactly `.github/workflows/harvey.yml` (+96/−25).
- `33d9dc8eb` FOUND — commit contents verified: exactly `.planning/ROADMAP.md` + `.planning/PROJECT.md` (2 insertions, 2 deletions — the two scoped checkbox ticks).
- No file deletions in either commit (`git diff --diff-filter=D HEAD~2 HEAD` empty).
- Neighbor workflows byte-clean after both commits (empty porcelain on r9-shell.yml + phase6.yml).
- Concurrent-session working-tree modifications (rc/bin, sys/lib/troff, sys/src/…) were never staged — both commits path-scoped to this plan's files only.
