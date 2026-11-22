---
phase: 01-build-tail-green-9k-amd64
plan: 02
subsystem: infra
tags: [plan9, mk, build-system, namespace-build, aarch64, quarantine, amd64, 9k, lima, nix]

requires:
  - phase: 01-build-tail-green-9k-amd64
    provides: "01-01: green end-to-end namespace build (3 kernels + populated /amd64/bin) that this plan's quarantine had to keep green"

provides:
  - "Zero live legacy-arm64 vestiges in build machinery: build/, sys/src/BUILD_SETUP_NAMESPACE, dist/replica/plan9.proto all clean (BUILD-03, D-05)"
  - "fn build_aarch64 (correct-but-dormant, objtype=aarch64 + cmd/7? glob) in build/scripts/build and build/scripts/linux-build"
  - "Phase 1 green gate scoped: linux-build defaults to arch=(amd64); dist/man stages behind WITH_DIST=1 (RESEARCH OQ-2/OQ-3)"
  - "Post-quarantine full build proven green: namespace enters without the /arm64 stanza, all six kernel artifacts relinked"

affects:
  - 01-03, 01-04 (build on the narrowed green gate)
  - Phase 2 (donor imports land on a tree that speaks aarch64 only; D-06 rename-on-import builds on this quarantine)

actuals:
  tokens: 949       # chars/4 over the realized diff (3797 diff chars, base..HEAD at SUMMARY write)
  tasks: 2
  commits: 2        # MEASURED: git rev-list --count 9bd72821d..HEAD at SUMMARY write (final docs commit lands after)
plan_head_before: 9bd72821d6b8b4707f558506ff5dde191299bae7
plan_head_after: 755c1741537899f8068340c70e86fab4f412793f

tech-stack:
  added: []
  patterns:
    - "Six-site rename/remove quarantine in ONE atomic commit when a bind-mount couples sites (Pitfall 3): mount --bind fails on a missing source, so stanza removal and dir removal can never be split"
    - "Opt-in env-flag gating for deferred build stages: if(~ $WITH_DIST 1){ ... } with a revert comment — stage retained intact, off by default"
    - "Verify-gate-aware comment phrasing: when a plan's grep gate counts a literal string, revert comments must describe the revert without embedding the literal"

key-files:
  created: []
  modified:
    - build/scripts/build
    - build/scripts/linux-build
    - build/mkdirs
    - sys/src/BUILD_SETUP_NAMESPACE
    - dist/replica/plan9.proto

key-decisions:
  - "All six quarantine sites landed in ONE commit (5af5e89bf) per D-05 + Pitfall 3 — the BUILD_SETUP_NAMESPACE bind-mount source existence is coupled to the root arm64/ dir removal; splitting them breaks every subsequent namespace build"
  - "dist/replica/plan9.proto arm64 stanza renamed to aarch64 in place (planner call per RESEARCH Pitfall 3) — completes 'tree speaks aarch64 only' ahead of Phase 2 donor imports"
  - "Task 2 revert comments phrased without the literal 'arch=(386 amd64)' assignment string — the plan's own verify gate (grep -c ... | grep -q '^0$') counts that substring per-line including comments, so the comment describes the revert ('restoring the 386 element to this assignment') instead of embedding the literal"
  - "Legitimate keeper strings documented in the quarantine commit message: nix/{setup,toolchain,build}.sh + flake.nix uname host-mapping, nix/lima-harvey.yaml image URL, sys/include/{a.out.h,mach.h}, sys/src/cmd/unix/9cc/**, sys/src/libdynld/dynld-arm64.c, AGENTS.md — none touched"

patterns-established:
  - "See tech-stack.patterns"

requirements-completed: [BUILD-03, BUILD-01]

coverage:
  - id: D1
    description: "Six-site legacy-arm64 quarantine in one atomic commit; zero 'arm64' matches in build/, sys/src/BUILD_SETUP_NAMESPACE, dist/replica/plan9.proto; root arm64/ dir removed; fn build_aarch64 (objtype=aarch64, cmd/7? glob) dormant in both build scripts"
    requirement: BUILD-03
    verification:
      - kind: integration
        ref: "grep -rln 'arm64' build/ sys/src/BUILD_SETUP_NAMESPACE dist/replica/plan9.proto -> exit 1; test ! -d arm64 && bash -n both scripts && grep -q 'fn build_aarch64' x2; scratch-tree mkdirs run creates no arm64 dir"
        status: pass
    human_judgment: false
  - id: D2
    description: "Namespace build still enters and completes green after the quarantine (Pitfall 3 coupling honored — no dangling bind-mount source)"
    requirement: BUILD-01
    verification:
      - kind: e2e
        ref: "nix run .#build post-quarantine (lima guest, log /tmp/01-02-postquarantine-build.log): exit 0; BUILD_SETUP_NAMESPACE ran via unshare -m; all six kernel artifacts relinked fresh (amd64/9k10{cpu,cpuf,f} + .elf)"
        status: pass
    human_judgment: false
  - id: D3
    description: "linux-build narrowed to Phase 1 scope: arch=(amd64) with revert comment; dist/man stages wrapped in if(~ $WITH_DIST 1) with revert comment; failure idioms (if(! ~ $status '') exit build) byte-stable"
    requirement: BUILD-01
    verification:
      - kind: e2e
        ref: "grep gates pass (arch=(amd64) present, WITH_DIST present, zero 'arch=(386 amd64)' literals); post-quarantine build log: 'build amd64' ran, zero 'build 386', zero cddisk, zero lookman"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-10-31
status: complete
---

# Phase 01 Plan 02: Legacy-Arch Quarantine + Phase-1-Scoped linux-build Summary

**All six live `arm64` vestiges quarantined to `aarch64` naming in one atomic commit (D-05 + Pitfall 3 bind-mount coupling), and `linux-build` narrowed to the Phase 1 green gate — amd64-only arch loop with dist/man stages behind an opt-in `WITH_DIST=1` flag — proven by a post-quarantine `nix run .#build` that entered the namespace and relinked all six kernel artifacts with exit 0.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-10-31T02:01:03Z
- **Completed:** 2026-10-31T02:07:00Z
- **Tasks:** 2/2
- **Files modified:** 5 (+31/−27 across both task commits)

## Accomplishments

- The tree's build machinery speaks `aarch64` only: zero `arm64` matches in `build/`, `sys/src/BUILD_SETUP_NAMESPACE`, and `dist/replica/plan9.proto` (comments included); the untracked root `arm64/` dir is gone
- `fn build_aarch64` (with `objtype=aarch64` and the `/sys/src/cmd/7?` glob intact) sits correct-but-dormant in both build scripts, ready for Phase 2's 7? toolchain vendoring (D-05)
- The Pitfall 3 coupling was honored: dropping the `/arm64` bind-mount stanza and removing the root dir landed in the same commit, so `mount --bind` never faced a missing source — the post-quarantine build proves the namespace still enters
- `linux-build` defaults to `arch=(amd64)`; the 386 pass and the dist/man stages (`/sys/lib/dist` cddisk, lookman/man-index) are gated off, not deleted — both carry revert comments and are trivially reversible
- Post-quarantine end-to-end evidence: `nix run .#build` exit 0, `build amd64` ran, zero `build 386`/`cddisk`/`lookman` occurrences in the log, all three kernels + `.elf` variants relinked with fresh timestamps

## Task Commits

Each task was committed atomically:

1. **Task 1: Legacy-arch quarantine — all six sites in ONE atomic commit** — `5af5e89bf` (refactor)
2. **Task 2: Narrow linux-build to Phase 1 scope (amd64-only + WITH_DIST gate)** — `755c17415` (feat)

**Plan metadata:** committed after this summary (docs: complete arm64-quarantine plan)

## Files Created/Modified

- `build/scripts/build` — `fn build_arm64` → `fn build_aarch64`; `objtype=aarch64`; `cmd/7?` glob kept (dormant per D-05)
- `build/scripts/linux-build` — same rename; plus `arch=(amd64)` narrowing and the `WITH_DIST` gate on the dist/man tail
- `build/mkdirs` — `./arm64/bbin` array entry dropped (continuation backslashes stay balanced; `bash -n` clean)
- `sys/src/BUILD_SETUP_NAMESPACE` — `/arm64` bind-mount stanza dropped (per RESEARCH: Phase 2 adds `/aarch64` wiring when the arch is real)
- `dist/replica/plan9.proto` — `arm64 d775 sys sys` stanza renamed to `aarch64` in place
- *(filesystem only, untracked)* — root `arm64/` dir removed (held only an empty `bbin/`; `git ls-files arm64` was empty)

## Decisions Made

- **Single-commit quarantine (plan-mandated, D-05 + Pitfall 3):** all five file edits plus the dir removal in `5af5e89bf`; the commit message enumerates the legitimate-keeper list (nix host-mapping idiom, lima image URL, vendored a.out.h/mach.h + unix/9cc object definitions, dynld-arm64.c, AGENTS.md) as the documented quarantine boundary.
- **plan9.proto rename (not drop):** the `arm64` arch stanza became `aarch64` — the planner's call per RESEARCH Pitfall 3, completing "tree speaks aarch64 only" before Phase 2 donor imports (D-06).
- **Revert-comment phrasing (see Deviations):** Task 2's comments describe how to restore the 386 pass without embedding the literal assignment string the plan's own grep gate counts.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan's prescribed revert-comment text would trip the plan's own verify gate**
- **Found during:** Task 2 (narrow linux-build)
- **Issue:** the action text prescribes a comment "re-enable by restoring `arch=(386 amd64)`", but the verify gate runs `grep -c 'arch=(386 amd64)' build/scripts/linux-build | grep -q '^0$'` — a per-line substring count that matches comments too, so the prescribed comment text would make the gate fail
- **Fix:** phrased the comment "re-enable by restoring the 386 element to this assignment" — same revert instruction, zero gate-tripping literal; the gate's fails_when note ("comments are filtered by the exact-line match on the assignment form") confirms the intent was to detect live code, not comments
- **Files modified:** build/scripts/linux-build
- **Verification:** all three Task 2 verify greps pass; the comment is directly above the `arch=(amd64)` line as prescribed
- **Committed in:** 755c17415 (part of task commit)

**2. [Rule 3 - Blocking-equivalent] Scratch-mkdirs verify adapted for fresh-shell execution**
- **Found during:** Task 1 verification
- **Issue:** the plan's third verify uses `$OLDPWD/build/mkdirs`, but each executor Bash call is a fresh shell with `$OLDPWD` unset
- **Fix:** ran the identical check with the absolute repo path (`bash /Users/neilskinner/code/harvey/build/mkdirs` inside the mktemp dir) — same semantics: scratch tree must not gain an `arm64` dir
- **Files modified:** none (verify invocation only)
- **Verification:** scratch run created no `arm64` dir; exit 0
- **Committed in:** n/a (no repo change)

---

**Total deviations:** 2 (1 verify-gate self-consistency fix, 1 verify-invocation adaptation)
**Impact on plan:** No scope creep; both deviations preserve the plan's checked contract exactly.

## Issues Encountered

None. Every file matched the plan's read_first line numbers exactly; the prior plan's BUILD_SETUP_NAMESPACE chown fix (01-01, commit 9a0f9e264) sits in a different hunk of the same file and was preserved untouched.

## User Setup Required

None.

## Next Phase Readiness

- The green gate is now exactly the amd64 kernel loop — 01-03 (KERNDATE/reproducibility) and 01-04 (minimega harness) build on a `nix run .#build` that is green by construction at phase scope
- Phase 2 precondition satisfied: no live `arm64` naming remains for donor imports to collide with (D-06 rename-on-import normalizes onto a clean tree)
- `WITH_DIST=1 linux-build` restores the dist/man stages and `arch=(386 amd64)` restoration re-enables the legacy pass when the ISO/dist milestone arrives — both revert paths are one-line edits documented in-place
- Carry-forward note: the quarantine grep gate covers `build/`, `sys/src/BUILD_SETUP_NAMESPACE`, and `dist/replica/plan9.proto`; any NEW legacy-token occurrence beyond the documented keeper list fails that gate and must be triaged manually (plan Flagged Assumptions, BUILD-03)

---
*Phase: 01-build-tail-green-9k-amd64*
*Completed: 2026-10-31*

## Self-Check: PASSED

- SUMMARY.md exists at .planning/phases/01-build-tail-green-9k-amd64/01-02-SUMMARY.md
- All five modified files exist on disk; root arm64/ confirmed absent
- Both task commits found in git history (5af5e89bf, 755c17415)
- Quarantine commit 5af5e89bf touches exactly the five declared files (acceptance criterion 1)
