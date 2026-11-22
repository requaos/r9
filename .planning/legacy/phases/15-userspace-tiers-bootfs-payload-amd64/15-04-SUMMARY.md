---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 04
subsystem: tier-machinery
tags: [tiers, tier3, gui, libdraw, subset-build, mk-filters, uspace-08, x86_64, compile-only]
duration: 26 min
completed: 2026-11-10
requires:
  - "15-01-complete tier machinery (the six-site multi-file-awk filter pattern + the tier driver's uniform entry dispatch + per-entry fresh builds this plan widens to three files)"
provides:
  - "build/x86_64/tiers/tier3-gui.txt — the append-only GUI allowlist (11 entries: page, rio, acme, sam, samterm, plumb, stats, winwatch, proof, screenlock, ip/gping) with per-entry provenance and the named fgui/libcontrol exclusion record"
  - "tier0-libs.txt's libdraw-family closure appends (libmemlayer, libmemdraw, libdraw, libframe, libplumb — k10f's lib order — plus libcomplete caught at rio's 6l): 24 T0-ART markers"
  - "the six mk filter sites' amd64 awk widened to the tier1+tier2+tier3 UNION in one multi-file call (aux renamed to the uniform TIER1A/TIER2A/TIER3A variable set)"
  - "subset-build.sh's tier-3 loop (the uniform entry dispatch + per-entry fresh builds) with T3-ART sha256 markers + 00008a97 magic checks + the D-07 compile-vs-execution boundary recorded in-code"
  - "USPACE-08's compile half proven: 11 entries -> 12 T3-ART markers (the plumb dir yields plumber+plumb), TIER-BUILD-OK, byte-stable across re-runs (page sha256 identical on the tracer gate's post-commit re-run)"
affects:
  - "build/x86_64/tiers/{tier3-gui,tier0-libs}.txt, build/x86_64/subset-build.sh, sys/src/{mkfile,cmd/mkfile,cmd/{ip,auth,aux}/mkfile,cmd/auth/factotum/mkfile}"
  - "15-05 (the draw donor kernel consumes the tier-0 libdraw family at link), 15-06 (the trace gates compute expectations from tier3-gui.txt), 15-07 (the scoping/coverage proof's computed expectations must widen to tier3), 15-09 (the libmemdraw pixel-path audit — compile green is NOT its input)"
tech-stack:
  added: []
  patterns:
    - "the three-file union awk: every filter site reads tier1+tier2+tier3 in ONE single-line call — even sites whose anchor can never match the new tier's entries (uniformity over cleverness; a future member can live in any tier file)"
    - "the tier-3 marker vocabulary inherits D-07's boundary by construction: T3-ART is artifact evidence ONLY; execution evidence for the GUI tier is 15-06's traces alone (a GUI binary that cannot start still earns T3-ART if it linked)"
    - "closure appends follow the measured autolib chain, not the mkfile's LIB line: page's mkfile names only libdraw but its sources include <plumb.h>/<bio.h>; rio's names four libs but wind.c also carries <complete.h>"
key-files:
  created:
    - build/x86_64/tiers/tier3-gui.txt
  modified:
    - build/x86_64/tiers/tier0-libs.txt
    - build/x86_64/subset-build.sh
    - sys/src/mkfile
    - sys/src/cmd/mkfile
    - sys/src/cmd/ip/mkfile
    - sys/src/cmd/auth/mkfile
    - sys/src/cmd/auth/factotum/mkfile
    - sys/src/cmd/aux/mkfile
key-decisions:
  - "The libdraw family lands in tier-0 as DATA closure in k10f's lib order (memlayer, memdraw, draw, then frame, plumb): the memdraw/memlayer halves are the KERNEL-side closure (no userspace GUI source reaches them by autolib — measured, no cmd/* GUI file includes <memdraw.h>), while libdraw/frame/plumb/close are the consumers' link closures. The tier-0 file feeds the 15-05 draw-donor kernel build and the LIBS filter from the same lines"
  - "gping re-derived IN (the mirror of 15-01's D-10 exclusion): 15-01's tier-2 record named tier 3 as gping's destination once libdraw closed — the wall is gone, so the promise is kept in this plan's data with provenance"
  - "fgui stays excluded, re-derived and recorded: fgui.c includes <control.h> -> libcontrol, a whole widget-toolkit closure outside this plan's file budget and USPACE-08's named set — it appends when a plan owns the libcontrol closure, never silently"
  - "The tracer comment corrected before commit: the plan called page a 'libdraw+libmemdraw consumer' but no page source touches memdraw (measured) — committed data records the measured truth (libdraw+plumb+bio by autolib)"
  - "libcomplete appended on the gate's own catch (rio wind.c / acme text.c <complete.h>) — the 15-01 catching-provenance pattern, one run, one line"
patterns-established:
  - "Three-file union filters: widening a tier set is adding one TIERnA variable + one filename to each site's existing awk call — never a second call, never a second keep-rule"
  - "Tier marker prefixes are tier-local but the loop shape is copy-exact: t3art() is t2art() with the prefix swapped; the dispatch table is shared verbatim"
requirements-completed: [USPACE-08]
actuals:
  tokens: 6100   # chars/4 over the realized diff (24451 chars, 9 files, 224 insertions — estimate was 38000: the same large over-projection direction as 15-01's 60000->10700)
  tasks: 2
  commits: 2      # measured: git rev-list --count 754e99e3b..HEAD
status: complete
coverage:
  - id: D1
    description: "Task 1 (tracer) — the libdraw family closed into tier-0, tier3-gui.txt created, all six filter sites on the three-file union awk, the tier-3 driver loop, proven end-to-end by the page tracer"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "nix develop -c sh build/x86_64/subset-build.sh: exit 0, 'T3-ART page bf0f53bc...' + TIER-BUILD-OK in the log, amd64/bin/page magic 00008a97 at the checkout path; 23 T0-ART (5 new family libs) / 40 T1-ART / 91 T2-ART unchanged"
        status: pass
      - kind: command
        ref: "tracer feedback gate (end-of-phase mode): both <automated> verifies re-run post-commit — VERIFY1-PASS + tier3-wiring-ok, page sha256 byte-identical on the re-run"
        status: pass
  - id: D2
    description: "Task 2 — the full 11-entry GUI membership compiles green with 12 T3-ART markers (parity M>=N), the libcomplete closure append recorded, tier1/tier2 data byte-frozen, sys/src clean"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "driver exit 0; entries=11 markers=12, TIER-BUILD-OK, all five named members (rio acme sam samterm plumb) + plumber/page/stats/winwatch/proof/screenlock/gping markers present; git diff HEAD -- sys/src/ and build/aarch64/ both clean; zero 6c/6l errors"
        status: pass
      - kind: command
        ref: "host-side magic spot-checks: all 12 binaries present at amd64/bin with magic 00008a97 (no MISSING, no BAD-MAGIC)"
        status: pass
---

# Phase 15 Plan 4: Userspace Tiers + Bootfs Payload (amd64) — GUI Tier Compile Summary

**The GUI stack as committed append-only tier data: the libdraw family (libmemlayer/libmemdraw/libdraw/libframe/libplumb + libcomplete caught at rio's 6l) closed into tier-0, the six mk filters on the tier1+tier2+tier3 union awk, and the tier-3 driver loop proving 11 entries -> 12 T3-ART markers — USPACE-08's compile half, with the compile-vs-execution boundary (D-07) recorded in the marker vocabulary itself.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-11-10T00:39:52Z
- **Completed:** 2026-11-10T01:06:17Z
- **Tasks:** 2/2 (Task 1 tracer)
- **Files modified:** 9 (1 created)

## Accomplishments

- **The tracer (Task 1)** rode one entry through the whole widened machinery before expansion: `cmd/page` -> the three-file union filters -> the namespace build with the libdraw family fresh in tier-0 (23 T0-ART) -> `/amd64/bin/page` -> `T3-ART page bf0f53bc...`, and the tracer feedback gate's post-commit re-run reproduced the sha256 byte-identically (the per-entry fresh-build discipline holding).
- **The full membership (Task 2)**: rio, acme, sam (text-only host half), samterm (installs to bin/aux — the mtime-stamp harvest found it), plumb (mkmany pair: plumber + plumb), the stats/winwatch/proof/screenlock utility class, and gping (15-01's named tier-3 destination, honored) — 11 entries, 12 markers, zero tool errors, sys/src git-clean after every run, aarch64 byte-frozen.
- **The closure grew exactly where the gate caught it**: one append (libcomplete — rio's wind.c/acme's text.c autolib), exactly the 15-01 catching-provenance pattern; no member was excluded to dodge a failure (D-10 held — the only exclusion, fgui/libcontrol, is re-derived and named, not compile-driven).

## Task Commits

1. **Task 1: the GUI tier tracer** — `e42069fbb` (feat)
2. **Task 2: the full GUI membership** — `d83bf9995` (feat)

## Decisions Made

See key-decisions in the frontmatter — the load-bearing ones: the family-as-data tier-0 closure (kernel-side memdraw/memlayer vs consumer-side draw/frame/plumb/close), gping's promise kept, fgui's libcontrol wall re-derived, and the corrected tracer comment (committed data records measured truth, not the plan's loose phrasing).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Data correctness] The plan's tracer comment was factually loose**
- **Found during:** Task 1 (tier3-gui.txt creation)
- **Issue:** The plan calls page a "libdraw+libmemdraw consumer"; measured, no page source includes <memdraw.h> and no libdraw archive member carries a memdraw autolib — page's link closure is libdraw + libplumb + libbio.
- **Fix:** The committed provenance comment records the measured closure; the memdraw/memlayer halves are documented as the kernel-side closure (tier-0 data for the 15-05 draw donor), which is also why the plan's family appends are still correct as data.
- **Files modified:** build/x86_64/tiers/tier3-gui.txt
- **Verification:** grep for memdraw/memlayer/Memimage across page sources returns nothing; the run's 6l line for page names libdraw.a only.
- **Committed in:** e42069fbb

**2. [Rule 2 - Missing closure] libcomplete needed at rio's link**
- **Found during:** Task 2 (the first full run)
- **Issue:** rio's mkfile names four libs but wind.c includes <complete.h> (acme's text.c too) — 6l aborted "cannot open libcomplete.a".
- **Fix:** Append-only tier0 append with catching-provenance (the 15-01 pattern); second run green.
- **Files modified:** build/x86_64/tiers/tier0-libs.txt
- **Verification:** 12/12 markers, zero 6l errors.
- **Committed in:** d83bf9995

**3. [Rule 1 - Membership] gping re-derived IN (the plan's enumerated list omitted it)**
- **Found during:** Task 2 (membership authoring)
- **Issue:** 15-01's tier-2 exclusion record explicitly names tier 3 as gping's destination ("joins with tier 3 ... the factotum:fgui precedent"), but this plan's Task-2 enumeration does not list it — omitting it would strand a recorded promise.
- **Fix:** cmd/ip/gping appended with provenance citing the 15-01 record; it rode the nested-single dispatch to /amd64/bin/ip/gping and earned its marker.
- **Files modified:** build/x86_64/tiers/tier3-gui.txt
- **Verification:** T3-ART gping in the run log; the ip container filter's keep set now includes gping.
- **Committed in:** d83bf9995

**4. [Rule 3 - Blocking verify] The plan's bare `sh subset-build.sh` verify cannot run in the ambient shell**
- **Found during:** Task 1 verify
- **Issue:** The ambient executor shell lacks plan9port mk (the driver's own header requires the nix devshell) — the same 15-01 deviation #2 class.
- **Fix:** The verify executed as `nix develop -c sh build/x86_64/subset-build.sh` (the driver itself unchanged); the greps ran against the same /tmp log.
- **Verification:** VERIFY1-PASS both at Task 1 and on the tracer gate's post-commit re-run.
- **Committed in:** e42069fbb (the driver header already documents the devshell requirement)

---

**Total deviations:** 4 auto-fixed (2 data-correctness/membership, 1 missing closure, 1 verify-environment). **Impact on plan:** all four are inside the plan's own disciplines (measured provenance, append-only closure growth, D-10 named records); no scope creep — the terminal truths (tracer end-to-end with byte-stable sha256, 11->12 parity, tree clean) are met exactly as planned.

## Authentication Gates

None.

## Known Stubs

None — no placeholder paths, no unwired data, no skipped verifications. T3-ART being compile-only is not a stub: it is D-07's designed boundary (the trace is the gate; the ledger's exec field carries the distinction until 15-06's evidence lands).

## Issues Encountered

- The first full tier-3 run failed at rio on the libcomplete closure gap (expected behavior — the gate catching a gap is the gate working); one append, second run green. No other member surfaced a gap.

## Next Plan Readiness

READY for 15-05 (the draw donor venue): the libdraw family is built and installed at /amd64/lib (fresh archives, sha256-marked), and 15-05's kernel build consumes exactly these. 15-06's trace gates compute expectations from tier3-gui.txt (11 entries; the plumb dir yields two markers). NOTE for 15-07: the 15-01 scoping proof's computed expectations read tier1+tier2 only — the phase-wide coverage proof must widen to the three-file union (a re-run of the 15-01 script as-is would now fail its cmd-TARG/cmd-DIRS checks by exactly the GUI membership). 15-09 (libmemdraw pixel-path audit) remains sequenced before 15-06's trace capture; compile green does not touch its premises.

## Self-Check: PASSED

- files exist: tier3-gui.txt, tier0-libs.txt, subset-build.sh, the six mkfiles ✓
- commits are ancestors of HEAD: e42069fbb, d83bf9995 ✓
- plan-level verification green at HEAD: fresh full run exit 0, entries=11 markers=12, TIER-BUILD-OK, all named members marked, sys/src clean, aarch64 frozen ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-10*
