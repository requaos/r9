---
phase: 19-cpu-service-drawterm-rio-desktop
plan: 06
subsystem: infra
tags: [drawterm, plan9-c, nix, 9p, cpu-client, terminal]

requires:
  - phase: 18-in-guest-p9sk1-auth-domain
    provides: the baked auth domain (glenda user, tcp/567 auth listener) the pinned -O spec names
  - phase: 16-host-net-modes-networking-exit-gate
    provides: the committed D-04 hostfwd pair (host 17010->guest 17010, host 5670->guest 567) the wrapper's loopback literals derive from
provides:
  - vendored drawterm -O pinned-connection flag (argv-single-source; env reads skipped when present)
  - vendored drawterm -G no-window stdio session (raw tty, cons on real stdio, exit propagation)
  - gui-none backend (no X11, no Carbon; loud stubs) selectable via make CONF=<os> GUI=none
  - flake packages.*.drawterm (hermetic, no fetch, no patchPhase) + apps.*.drawterm wrapper
  - flagtest.sh TAP harness pinning the flag surface on both backends
  - README drawterm sections (flag table, linux session recipe, darwin -G recipe + deferral record)
affects: [19-08 (the -G interop gate drives this surface), 19-10 (DESK-07 evidence rows), DESK-01]

actuals:
  tokens: 9880      # chars/4 over the realized diff (plan estimate was 50000 — the vendor surface was smaller than estimated)
  tasks: 3
  commits: 6

tech-stack:
  added: [nix stdenv.mkDerivation over vendored C source (libX11/libXt/xorgproto merged X11 prefix)]
  patterns:
    - "argv-as-single-source: a pinned-connection flag (-O) that structurally disables the env reads (code-order, not just override)"
    - "pre-gui arg validation: an earlyflags() pre-scan before screeninit so usage errors are reachable headless"
    - "none-class backend member: GUI=none follows AUDIO=none (a selectable stub member of a device/backend set with loud failures)"
    - "TAP-from-bash outside-the-binary harness for C flag surface (timeout-guarded, hostile-env discriminating)"

key-files:
  created:
    - sys/src/cmd/unix/drawterm/gui-none/screen.c
    - sys/src/cmd/unix/drawterm/gui-none/Makefile
    - sys/src/cmd/unix/drawterm/flagtest.sh
    - nix/drawterm.nix
    - nix/drawterm-x64.sh
  modified:
    - sys/src/cmd/unix/drawterm/cpu.c
    - sys/src/cmd/unix/drawterm/main.c
    - sys/src/cmd/unix/drawterm/drawterm.h
    - sys/src/cmd/unix/drawterm/Make.config
    - sys/src/cmd/unix/drawterm/include/lib.h
    - flake.nix
    - build/x86_64/README.md

key-decisions:
  - "-O parsed on a private copy (parseospec smprints): getfields writes NULs over delimiters and the spec lives in argv — the earlyflags pre-scan and cpumain's canonical parse must both see it whole"
  - "usage errors terminate via argusage (OS write + exit(2)) before any gui init: the vendored usage()->exits() park spins forever, which would wedge both the headless venue and the pipe-based verifies"
  - "the wrapper's headless leg execs the -G line LOUDLY when DISPLAY is unset (printed note, never silent): the human default stays windowed (D-07), and the plan's wrapper verify (dial indicator in output) is satisfiable on a no-display venue exactly through this documented fallback"
  - "-G feeds the keyboard queue per-rune from real stdin with split-rune carry and \\r as \\n (the XK_Return->'\\n' gui precedent); the OS tty goes raw with output post-processing kept so remote \\n still renders as CRLF"
  - "gui-none's Memimage ops pass through to the plain libmemdraw in-memory implementations (no backing store) so libmemdraw consumers link unchanged; only screen/window/clipboard ops are loud-fatal or honestly-empty"

patterns-established:
  - "Early pre-scan + canonical re-parse: settle gui-affecting flags before any gui init while keeping ARGBEGIN the single grammar owner"
  - "flagtest.sh: outside-the-binary TAP for C surfaces where no in-tree test framework exists (RED classifiable via tdd-red-evidence's TAP adapter)"

requirements-completed: [DESK-01]

coverage:
  - id: D1
    description: "-O pinned-connection flag: spec grammar (three required keys), usage errors naming keys, env reads never executing with -O present"
    requirement: DESK-01
    verification:
      - kind: other
        ref: "sys/src/cmd/unix/drawterm/flagtest.sh tests 1,2,5 (usage reachability headless, missing-user reject, code-order env-read assert) — green on gui-x11 and gui-none"
        status: pass
    human_judgment: false
  - id: D2
    description: "-G no-window stdio session: no X touch, hostile env ignored, loud dial failure, terminating exit"
    requirement: DESK-01
    verification:
      - kind: other
        ref: "flagtest.sh tests 3,4 (DISPLAY-unset run with cpu/auth/USER=evil: no usage, no X error, no 'evil', dial/refused in output, rc!=124) + live runs on both backends"
        status: pass
    human_judgment: false
  - id: D3
    description: "gui-none backend: builds with no X11 headers and no Carbon; windowed use exits loudly at startup"
    requirement: DESK-01
    verification:
      - kind: other
        ref: "make CONF=unix GUI=none X11=/nonexistent (build OK) and the windowed run's named exit: 'drawterm: gui-none: screeninit called but this build has no screen; use -G...'"
        status: pass
    human_judgment: false
  - id: D4
    description: "Hermetic nix package + scrubbing wrapper + linux-only flake app (gate list untouched)"
    requirement: DESK-01
    verification:
      - kind: other
        ref: "NIX-PKG-OK (nix build .#drawterm, zero source fetches), APP-EVAL-OK (nix eval .#apps...program), WRAPPER-PRINTS-AND-DIALS (env -u DISPLAY nix run .#drawterm: pinned -O line + 'cpu: can't dial: 127.0.0.1: Connection refused'), hostile-env scrub clean, x64 hasAttr gate list byte-unchanged (diff-asserted)"
        status: pass
    human_judgment: false
  - id: D5
    description: "README drawterm sections: flag table, linux session recipe, darwin -G recipe with the exact ssh -L pair, deferral + reopener, -e clear debug-only wording"
    verification:
      - kind: other
        ref: "README-SECTIONS-OK (grep '## Drawterm', 'ssh -L 17010', 'deferred', XQuartz count) — build/x86_64/README.md"
        status: pass
    human_judgment: false

duration: 32 min
completed: 2026-11-14T23:08:03Z
status: complete
---

# Phase 19 Plan 06: Drawterm host half — -O/-G flags, gui-none, nix package + wrapper, README Summary

**Vendored drawterm gained the -O pinned-connection spec (argv single-source, env reads skipped) and the -G no-window stdio session, plus a gui-none backend and a hermetic nix package whose wrapper scrubs the credential env, pins the loopback -O spec, and prints the exact encrypted-default connect line**

## Performance

- **Duration:** 32 min
- **Started:** 2026-11-14T22:35:37Z
- **Completed:** 2026-11-14T23:08:03Z
- **Tasks:** 3
- **Files modified:** 12

## Accomplishments

- **Task 1 (TDD):** `-O cpu=,auth=,user=` and `-G` landed in the vendor as normal Harvey commits (no re-vendoring). RED first: `flagtest.sh` (TAP-13, timeout-guarded, hostile-env discriminating) failed 5/6 on the pristine tree (`evidence/19-06-t1-red.json`, RED_EVIDENCE_OK), then went 6/6 green on BOTH gui-x11 and gui-none. `earlyflags()` validates -O/-G before `screeninit` so usage errors are reachable headless; the env reads moved structurally behind the no-O path; default ealgs (`rc4_256 sha1`) and the windowed path untouched.
- **gui-none backend** (amended D-03): `make CONF=unix|osx GUI=none` — no X11 headers, no Carbon (proven building against a nonexistent X11 prefix); every screen/window op is a loud named exit; Memimage ops pass through to plain libmemdraw; clipboard honestly empty.
- **Task 2:** `nix/drawterm.nix` builds the committed vendor source hermetically (merged X11 prefix from split store outputs; no fetchurl/fetchgit/patchPhase); `nix/drawterm-x64.sh` scrubs `$cpu/$auth/$PASS/$USER`, takes no arguments, pins `cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda`, prints the windowed + `-G` connect lines (no `-e` anywhere), and execs — headless venues run the `-G` line loudly. Flake: package beside minimega (linux-only), app in its own linux-only optionalAttrs; the x64 `hasAttr` gate list is byte-unchanged.
- **Task 3:** README drawterm sections — flag table (incl. `-e clear` DEBUG-only, D-15), the D-07 linux session recipe, the darwin `-G`-only recipe with the exact `ssh -L 17010:127.0.0.1:17010 -L 5670:127.0.0.1:5670` pair, the 2026-11-14 deferral + Cocoa reopener, and the XQuartz-rejected record; `-G` CI-ability note points at 19-08.

## Task Commits

Each task was committed atomically (TDD: test -> feat):

1. **gcc-16 vendor build fix** - `630353e09` (fix)
2. **Task 1 RED: flag-surface TAP harness + evidence** - `1aff004af` (test)
3. **Task 1 GREEN: -O pinned spec + -G stdio session** - `681563b8a` (feat)
4. **Task 1 GREEN: gui-none backend + Make.config wiring** - `31239a3ef` (feat)
5. **Task 2: nix package + wrapper + flake app** - `9c29dfb9f` (feat)
6. **Task 3: README sections** - `d691371ee` (docs)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP commit)

## Files Created/Modified

- `sys/src/cmd/unix/drawterm/cpu.c` — -O/-G ARGBEGIN cases, parseospec (private copy), earlyflags pre-scan, env reads behind no-O, Gflag, exits()->gconsexit branch, shared usagestr
- `sys/src/cmd/unix/drawterm/main.c` — earlyflags call, gui branch (gconsinit vs screeninit; #m/#i binds skipped under -G), the -G runtime (raw tty + atexit restore, stdin rune reader, stdout console, real exit)
- `sys/src/cmd/unix/drawterm/drawterm.h` — the -G/-O extern surface
- `sys/src/cmd/unix/drawterm/gui-none/{screen.c,Makefile}` — the none backend (loud screen stubs, real memimage passthrough)
- `sys/src/cmd/unix/drawterm/Make.config` — GUI=none strips X11/Carbon flags (the AUDIO=none class)
- `sys/src/cmd/unix/drawterm/include/lib.h` — libstrtoll prototype (gcc>=14 hard-errors on the implicit decl)
- `sys/src/cmd/unix/drawterm/flagtest.sh` — the TAP harness (RED/GREEN pin)
- `nix/drawterm.nix`, `nix/drawterm-x64.sh`, `flake.nix` — package, wrapper, registration
- `build/x86_64/README.md` — the drawterm sections

## Decisions Made

- parseospec works on an smprint copy: getfields NULs the delimiters and the spec lives in argv — both the pre-scan and the canonical parse must see it whole (found live: the first green attempt rejected every valid spec).
- Pre-gui usage termination (argusage: OS write + exit(2)): the vendored usage()->exits() park would spin forever — wedging headless runs and every pipe-based check; windowed session behavior is untouched.
- Wrapper headless fallback is loud and printed (DISPLAY-unset note, then the -G line): keeps D-07's windowed human default AND makes the plan's dial-indicator verify true on no-display venues; documented in the README.
- `-G` stdin is read per-rune with split-rune carry and `\r`→`\n` (the gui backends' XK_Return precedent); raw tty keeps OPOST/ONLCR so remote `\n` renders correctly and ISIG is off so `^C` belongs to the remote session.
- The wrapper's connect lines print the spec unquoted (the plan's own grep form); the README recipe shows the quoted form for interactive bash (`!` history expansion).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Vendored drawterm did not build under gcc 16**
- **Found during:** Task 1 (RED baseline build)
- **Issue:** `include/lib.h` renames `strtoll`→`libstrtoll` without declaring it; gcc>=14 makes implicit function declarations hard errors — `libc/nsec.c:56` failed before anything else. Pre-existing vendor breakage, unrelated to the flags, but it blocked every build.
- **Fix:** 4-line `extern vlong libstrtoll(...)` declaration in `include/lib.h` (the definition in `libc/strtoll.c` is untouched — it already provides the renamed symbol).
- **Files modified:** sys/src/cmd/unix/drawterm/include/lib.h
- **Verification:** full builds of both backends; no behavior change.
- **Committed in:** 630353e09

**2. [Rule 3 - Blocking] Task-1 `<verify>` venue adaptation (bare `make CONF=unix`)**
- **Found during:** Task 1
- **Issue:** The plan's bare `make CONF=unix` assumes X11 headers at `/usr/X11R6` — absent on NixOS; the primary host has split store outputs.
- **Fix:** Verified the windowed build with `make CONF=unix X11=/tmp/x11merge` (a merged prefix of the store's libX11/libXt/xorgproto outputs — the same merge `nix/drawterm.nix` performs) and the none build with `make CONF=unix GUI=none` (proven against a nonexistent X11 prefix). The Task-2 nix build then covers the hermetic windowed leg.
- **Verification:** VENDOR-BUILD-OK equivalent green on both backends; flagtest 6/6 on both.
- **Committed in:** (venue adaptation — no code change; the merged-prefix merge itself is nix/drawterm.nix's buildPhase)

**3. [Rule 3 - Blocking] drawterm.h needed the -G/-O extern surface**
- **Found during:** Task 1
- **Issue:** `files_modified` did not list `drawterm.h`, but main.c/cpu.c share `Gflag`, `earlyflags`, `gconsinit`, `gconsexit`, `argusage`, `usagestr` — the tree's own cross-file contract header is their home.
- **Fix:** 8-line extern block in `drawterm.h`.
- **Files modified:** sys/src/cmd/unix/drawterm/drawterm.h
- **Verification:** builds clean.
- **Committed in:** 681563b8a

---

**Total deviations:** 3 auto-fixed (3 blocking)
**Impact on plan:** All three were prerequisites for the plan's own verifies passing; no scope creep.

## TDD Gate Compliance

Task 1 is `tdd="true"`: the RED commit (`1aff004af`, harness + evidence) precedes the GREEN feat commits (`681563b8a`, `31239a3ef`). RED evidence: `evidence/19-06-t1-red.json` — classified `RED_EVIDENCE_OK` (TAP adapter, target `-G with a valid -O spec dials with no window and no env reads` failing for the planned reason: screeninit died at XOpenDisplay before arg parsing, env reads ahead of ARGEND). The classifier first rejected exit code 0 (`unexpected_green`) and diagnostic-polluted test names — the harness was fixed to proper TAP producer behavior (clean names, nonzero exit on failure) before evidence was accepted. Semantic assessment is recorded in the evidence JSON. Plan-level `workflow.tdd_mode` is false (config), so no plan-type gate applies; the task-level cycle is complete. No REFACTOR phase was needed.

## Known Stubs

- `sys/src/cmd/unix/drawterm/gui-none/screen.c` — the screen/window ops (`screeninit`, `attachscreen`, `flushmemscreen`, `mouseset`, `setcursor`, `cursorarrow`, `getcolor`, `setcolor`) are intentional loud-fatal stubs, and `clipread`/`clipwrite` are intentionally empty: this backend exists precisely so a `-G`-only build has NO screen, with every misuse exiting loudly at startup (T-19-14's disposition — a design deliverable of this plan, not a placeholder; the `-G` path never calls them and windowed use is directed to `-G` by the message itself).

## Issues Encountered

- The first green attempt rejected every valid `-O` spec: `getfields` mutates its input and the early pre-scan had destroyed the argv string before cpumain's canonical parse. Fixed by parsing on a private copy (see Decisions).
- The primary host has a live display (`DISPLAY=:0.0`): a windowed wrapper run opens a real drawterm window on the user's desktop and parks (vendored behavior). The wrapper's automated verify therefore runs via the documented headless `-G` fallback (`env -u DISPLAY`); the windowed branch was observed live (window + parked session) before being killed.
- `nix build` initially staged the gui-none build artifact (`libgui.a`) via a directory `git add` — caught by the post-commit check, removed by amending the commit (the vendored tree keeps no tracked archives).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The `-G`/`-O` client surface the 19-08 interop gate drives is committed and pinned by `flagtest.sh`; the wrapper's pinned dial line (`cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda`) is the gate's connection contract.
- No r9 dependency was touched (this plan was Wave-1 parallel by design).
- The darwin graphical leg stays deferred with its reopener recorded (README); DESK-07's darwin row requires a real run on the machine.

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-14*

## Self-Check: PASSED

All key-files created/modified exist on disk; all 6 task commits are ancestors of HEAD; the plan `<verify>` commands and acceptance criteria re-ran green at close-out (flagtest 6/6 gui-none, usage rejects, NIX-PKG-OK / APP-EVAL-OK / WRAPPER-PRINTS-AND-DIALS, README-SECTIONS-OK).
