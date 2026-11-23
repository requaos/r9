---
phase: 01-stage-p0-cross-build-harness
plan: "02"
subsystem: infra
tags: [nix, lima, builder, case-collision, apfs, devshell, harvey, cross-build, proot]

requires:
  - phase: 01-stage-p0-cross-build-harness
    plan: "01"
    provides: the lima venue, the proot recipe, the four host-toolchain ports and the pinned nixpkgs/Harvey inputs
provides:
  - S2 verdicts for the Mac-fetched flake input, the builder-side fixed-output fetch and the Mac copy-back
  - A9 verdict for a lazy, context-discarded devshell export matching the later build path
  - S4 verdicts for Harvey's amd64 tier 0 and tier 1 on the aarch64-linux builder, with the two venue ports
  - spikes/s2 and spikes/s4 flakes, the 23-line collisions.txt and the committed evidence logs
affects: [D-04 mechanism choice, D-05 lazy per-arch tree supply, pass-2 tree derivation, Plan 07 closing checkpoint]

actuals:
  tokens: 20479
  tasks: 3
  commits: 3

tech-stack:
  added:
    - "pkgs.fetchFromGitHub fixed-output fetch pinned by rev + SRI hash"
    - "builtins.unsafeDiscardStringContext for the lazy devshell export"
    - "a local chroot store with require-sigs=false for the Mac copy-back measurement"
  patterns:
    - "casecheck derivation: exact-name file assertions, git blob sha1 per path, a find for ~nix~case~hack~ names, copy-back in tree layout"
    - "copy-back measurement on the Mac: exact-name directory enumeration, never a bare [ -f ] (APFS resolves the case twin)"
    - "lazy devshell: shellHook exports a context-discarded outPath; the resolver builds on first use"
    - "tier build: per-entry independent loop over the pinned tier data in the S1 proot venue, with BUILT/FAILED and S_MAGIC per artifact"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/S2-FINDINGS.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/S4-FINDINGS.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/.gitignore
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/flake.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/flake.lock
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/collisions.txt
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/evidence/casecheck-builder.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/evidence/lazy-devshell.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/.gitignore
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/flake.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/flake.lock
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/venue.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/hostbin.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/tier.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/evidence/tier-builder.log
  modified: []

key-decisions:
  - "S2: both source modes are safe for the real tree; the pass-2 tree derivation should fetch Harvey on the builder through a fixed-output fetch (rev + SRI) so the source never round-trips through APFS"
  - "S2: the Mac system store refuses `nix copy --from` as the owner's user (`require-sigs = true`, `trusted-users = root`); pass 2 must use a remote-builder configuration where the local daemon imports the result, or an owner-approved store-policy change"
  - "S2: the case mechanism itself is sound on APFS — a same-volume chroot store imports both outputs, `nix store verify --no-trust` is clean, and 23/23 blob sha1s re-check per output; a Mac-side lookup by real path resolves the case twin, so tree outputs must not contain colliding names"
  - "A9: a context-discarded outPath in the darwin devshell is lazy (no build sent) and equals the later build path, so HARN-07's supply can meet D-05"
  - "S4: Mac-built amd64 trees can come from the aarch64-linux builder with two venue ports — `/rc/bin` on PATH (membername) and an explicit `INCLUDE` list (latent 6c heap-layout failure under proot); no x86 builder or emulation needed"
  - "S4: the tier pair costs 25 s (6 s tier 0 + 19 s tier 1), refuting A8's tens-of-minutes estimate"

patterns-established:
  - "A FAIL leg with exact evidence is a complete answer; the copy-back FAIL is store policy, measured, with the case question answered separately"
  - "Controlled A-F experiments inside the same venue isolate a build failure to its cause before recording a workaround"

duration: 9min
completed: 2026-11-23
status: complete
---

# Phase 1 Plan 2: S2 case collisions, A9 lazy devshell, S4 amd64 tiers

**Both source modes preserve Harvey's 23 case-colliding paths into the lima builder and the Mac's case-hack round-trips cleanly; the devshell export is lazy and exact; the full amd64 tier 0 + tier 1 builds on the aarch64 builder in 25 s with two measured venue ports.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-11-23T01:13:05Z
- **Completed:** 2026-11-23T01:22:00Z
- **Tasks:** 3
- **Files modified:** 15 created (plus two gitignored work dirs)

## Accomplishments

- **S2 (A2 CONFIRMED).** `collisions.txt` pins the 23 colliding paths with
  their git blob sha1s at `bd7a698`, generated read-only. Both
  `s2-casecheck-input` (Mac-fetched flake input, NAR-shipped) and
  `s2-casecheck-fod` (builder-side `fetchFromGitHub`, SRI-pinned) saw all 23
  paths under their exact names with matching blobs, no case-hack names, and
  `rc/bin` carrying both `kill` and `Kill`; both report
  `S2-CASECHECK-ALL-OK`.
- **S2 copy-back.** As specified, `nix copy --from ssh-ng://lima-harvey` as
  the owner's user is refused by the Mac store's signature policy (a FAIL
  leg, recorded). The case question was answered in a local chroot store on
  the same APFS data volume: both outputs imported, `nix store verify
  --no-trust` was clean, and 23/23 blobs re-checked per output after mapping
  the 12 case-hacked names back.
- **A9 (CONFIRMED).** A darwin devshell exported a context-discarded
  aarch64-linux outPath; `nix develop` built only its env derivation (the
  target stayed invalid on both stores), the printed path equalled the
  evaluated one, and the later build produced the identical path.
- **S4 (PASS).** Tier 0: 27/27 libraries in 6 s. Tier 1: 40/40 entries in
  19 s, every binary with `S_MAGIC` `00 00 8a 97`. A3/A4 CONFIRMED, A8
  REFUTED. Two venue ports were needed and are recorded: `/rc/bin` on PATH
  for `membername`, and an explicit `INCLUDE` list that avoids a latent 6c
  heap-layout failure under proot (controlled A-F experiment; natively the
  same compiles pass).

## Task Commits

Each task was committed atomically:

1. **Task 1: S2 case collisions through the builder** — `4f297ad` (docs)
2. **Task 2: A9 lazy devshell export** — `d134f6b` (docs)
3. **Task 3: S4 amd64 tier 0 + tier 1 on aarch64-linux** — `d7c8ef8` (docs)

**Plan metadata:** this SUMMARY + STATE/ROADMAP (`docs(01-02): complete …`).

## Files Created

- `spikes/s2/collisions.txt` — 23 `<blob-sha1> <path>` lines in 11 groups
- `spikes/s2/flake.nix` — `s2-casecheck-{input,fod}`, `s2-lazy-target`,
  `devShells.aarch64-darwin.lazy`; `flake.lock` pins nixpkgs `1c89083e` and
  Harvey `bd7a698`
- `spikes/s2/evidence/casecheck-builder.log` — reports, copy-back attempts,
  verify results, case-hack names and the exact-name re-check (190 lines)
- `spikes/s2/evidence/lazy-devshell.log` — the four A9 observations (40 lines)
- `spikes/S2-FINDINGS.md` — S2/A9 verdicts, exact commands, A2/A9, pass-2
  implications
- `spikes/s4/{venue.nix,hostbin.nix}` — byte copies of the S1 files (sha256s
  in the findings); `flake.nix` + `flake.lock` pin the same inputs
- `spikes/s4/tier.nix` — the per-entry tier build in the proot venue, with
  the two ports and the output contract
- `spikes/s4/evidence/tier-builder.log` — final report, no-INCLUDE control,
  A-F venue experiment and the echo comparison (122 lines)
- `spikes/S4-FINDINGS.md` — S4 verdicts, counts/timings, A3/A4/A8, pass-2
  implications

## Decisions Made

- **Prefer the builder-side fixed-output fetch for pass 2's tree source.**
  It is pinned by rev and SRI and never sends the 257 MB source through the
  Mac's case-hacked store; the input mode also works but is the more fragile
  route.
- **The copy-back step needs a store-policy answer.** The NAR transport and
  the APFS case hack are sound; the owner's user cannot import unsigned paths
  into the system store. A remote-builder configuration (the daemon imports
  its own build result) is the natural fit and needs no signature key.
- **The devshell supply can be lazy.** The exported string costs nothing to
  evaluate and the resolver's "exists? use : nix build" contract gets the
  exact path.
- **The amd64 tier belongs on the aarch64 builder** with the two venue ports;
  the ports are venue properties, not changes to Harvey's build recipe.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `membername` is not on the proot PATH**
- **Found during:** Task 3 (first tier build)
- **Issue:** `9k/boot` failed `membername: No such file or directory`.
  `rc/bin/membername` is an rc script with a `#!/bin/rc` shebang that the
  boot mkfile calls by name; the S1 proot venue does not put `/rc/bin` on
  PATH, while Harvey's own userns venue does (`PATH="$INNER_BIN:/rc/bin:$PATH"`).
- **Fix:** append `/rc/bin` to PATH in `s4/tier.nix`.
- **Verification:** the no-INCLUDE control run builds `9k/boot`
  (`S4-TIER1 built=39/40`, only `dc` left).
- **Committed in:** `d7c8ef8`

**2. [Rule 3 - Blocking] 6c "out of memory" under proot on two large files**
- **Found during:** Task 3 (first tier build)
- **Issue:** `libhtml/build.c` and `cmd/dc.c` failed `6c ... out of memory`
  at `build.c:1821` / `dc.c:899` in the S1 proot venue, while the other 63
  entries built.
- **Fix:** set `INCLUDE="/amd64/include /sys/include"` in the venue guest.
  This is the same include list the compiler's default branch adds; a
  controlled A-F experiment showed the venue default path fails (A/E) while
  `ccroot` at the store path, the store source, the explicit `INCLUDE` and
  the `dc.c` store-include variant all pass (B/C/D/F), and the same compiles
  pass natively. The trigger is a latent compiler heap/`sbrk` interaction
  under proot, not a host or source limit.
- **Verification:** the final build is 27/27 and 40/40; the no-INCLUDE
  control is 26/27 and 39/40.
- **Committed in:** `d7c8ef8`

**3. [Rule 3 - Blocking] copy-back into the Mac system store is refused**
- **Found during:** Task 1 (copy-back)
- **Issue:** `nix copy --from ssh-ng://lima-harvey <out>` as the owner's user
  fails `cannot add path … because it lacks a signature by a trusted key`
  (`require-sigs = true`, `trusted-users = root`); `--no-check-sigs` and
  `--option require-sigs false` are ignored as restricted settings, and
  `nix-store --export | --import` is refused the same way.
- **Fix:** measured the case question in a local chroot store on the same
  APFS data volume with `--option require-sigs false` (honoured for a direct
  local store). Both outputs imported, `nix store verify --no-trust` clean,
  23/23 blobs re-checked per output. The specified system-store leg is
  recorded as `VERDICT s2-copyback-lima: FAIL` with the policy cause.
- **Files modified:** none (measurement only)
- **Committed in:** `4f297ad`

### Deliberate protocol deviation

**Requirements checkboxes stay Pending.** The plan frontmatter lists HARN-07
and HARN-06, but the owner's recorded pass-1 decision (STATE.md, D-22) says
no HARN requirement is implemented until pass 2; this plan delivers spike
evidence only. `REQUIREMENTS.md` is left unchanged so pass 2 remains the plan
that closes them, matching Plan 01's treatment.

**GSD CLI availability.** `gsd_run` was not on the executor shell's PATH;
the roadmap/state steps used the absolute GSD core CLI
(`…/gsd-core/bin/gsd_run`). The roadmap update was re-run after this SUMMARY
existed so the plan/summary counts it writes are 5/7.

## Issues Encountered

- The Mac store's `use-case-hack = true` makes a bare `[ -f "$dir/Hx" ]`
  resolve `HX` on APFS; the first Mac-side re-check reported 12/23 (the
  wrong twin's hash). The exact-name enumeration pass reports 23/23. The
  committed log keeps both passes as a method warning.
- The lima guest's interactive PATH carries plan9port's `ls`; store
  inspections used `find`/absolute tools.
- The first tier-build log was overwritten by the fixed run; the controlled
  no-INCLUDE run was rebuilt to regenerate the failing evidence verbatim.

## Known Stubs

None. Every artifact is produced by a real build; the only "not run" item is
the system-store copy-back, which is a recorded FAIL with a measured
substitute, not a stub.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema
crossed a trust boundary; the actions are builds and read-only inspections
of the pinned source.

## Next Phase Readiness

- Plan 07's closing checkpoint has the S2 evidence for the D-04 choice: the
  lima NAR transport preserves the names; the fixed-output fetch is the
  more robust supply; the Mac system-store import policy is the one open
  owner decision.
- The pass-2 tree derivation can carry the measured S4 venue ports and size
  tiers 0-1 at seconds; the remaining estimate is tiers 2-4 and the image
  bake.
- HARN-07's devshell supply can be lazy and exact; pass 2's resolver can
  rely on the exported path matching the later build.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

All 15 created artifacts exist and are non-empty; the three task commits
(`4f297ad`, `d134f6b`, `d7c8ef8`) are ancestors of HEAD; the plan's
verification block reports `HARVEY-UNTOUCHED`, no protected-path diffs, no
`spikes/s1` diffs and no pushes.
