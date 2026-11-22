---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 06
subsystem: trace-rig
tags: [syscall-trace, tracesup, replay-scripts, kbin, mousein, rio, acme, sam, samterm, drawtest, arctest, qemu, amd64, uspace-08]
duration: 146 min
completed: 2026-11-10
requires:
  - "15-05-complete draw donor venue (the four-marker gate, the fonts, the GUI binaries, the kbin/mousein surface, the /tmp scratch)"
  - "15-09-complete libmemdraw pixel-path fixes (the runtime re-join contract: DRAWTEST-OK / ARCTEST-OK)"
provides:
  - "the born-traced GUI syscall trace rig on the draw donor: hang-at-exec race-free launch, tracesup arming, replay sourcing, per-pid fragment egress over the serial capture, completion-marker classification (D-07)"
  - "the kernel trace substrate completed: syscall.c's Proc_tracesyscall stops format the entry/return fragments (the legacy pc/trap.c discipline), syscallfmt.$O links in every k10 CONF"
  - "the three committed replay scripts (rio/acme/sam) — the D-05 scripted-interaction drivers over /dev/mousein + /dev/kbin"
  - "the measure-first evidence set: raw per-pid fragment logs + distilled per-binary syscall tables (syscall→count→args-shape, PATH rows, TOTAL union) for rio/acme/sam(+samterm) — the tables r9 kernel-growth plans cite"
  - "the 15-09 runtime re-join green on the venue: DRAWTEST-OK + ARCTEST-OK on both committed captures"
  - "build/x86_64/tier3.ledger — the GUI ledger binding every tier-3 artifact digest to its TRACE: evidence pointer"
affects:
  - "15-08 (the evidence bundle: the raw/table sets, the captures, the distill repro proof, the ledger)",
  - "Phase 17+ r9 GUI plans (the measured syscall sets + PATH rows are the kernel-growth input)"
tech-stack:
  added: []
  patterns:
    - "mousein writes are RELATIVE (mousetrack deltas, clamped): deterministic replays home-clamp ('M -5000 -5000 0 0' pins the pointer at the origin) then move known absolute deltas — no arithmetic, no accumulated-state assumptions"
    - "race-free traced launch: `rc -m rcmain -c 'echo hang > /proc/$pid/ctl; exec CMD'` stops the image at its first instruction; waitstop blocks until it does; nohang clears the flag for descendants — replaces the KVM-racy spin window"
    - "rc sources an rcmain UNCONDITIONALLY: a venue that runs rc -i/rc -c children MUST bind a real /rc/lib/rcmain (rio windows, acme Execute, sam !cmd all died without it)"
    - "an rc redirect or builtin failure RAISES and unwinds the whole script — guest drivers guard every ctl write/redirect with its own test -e"
    - "completion-marker gating over settle-only evidence (D-07) and path-folding in distilled tables (pure-digit path components -> N) keep run-to-run tables comparable"
key-files:
  created:
    - build/mm/boot-9kdraw-trace.sh
    - build/x86_64/replay-rio.rc
    - build/x86_64/replay-acme.rc
    - build/x86_64/replay-sam.rc
    - build/x86_64/trace-distill.sh
    - build/x86_64/tier3.ledger
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-{rio,acme,sam}-{raw,table}.txt
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-{trace-venue,venue-bringup,distill-repro}.log
  modified:
    - sys/src/9k/k10/syscall.c
    - sys/src/9k/port/syscallfmt.c
    - sys/src/9k/k10/mkfile
    - sys/src/9k/k10/root/k10draw.rc
    - build/mm/build-9kdraw.sh
key-decisions:
  - "mousein measured RELATIVE on this tree (devmouse.c Qmousein -> mousetrack(dx,dy)): the first replay assumed absolute, clicked a clamped-corner menu, and no window was ever created — every committed replay homes first"
  - "the spin-window launch (06-07's shape) lost the race on KVM: the launcher rc outran the arming stop through the paq-served /bin and the first boot traced rio from mid-startup — the hang-at-exec launch is race-free and starts the trace at the exec'd image's first instruction (the launcher's rc noise excluded by construction)"
  - "syscallfmt.c was dead code until this plan (never compiled in 9k): its first compile carried a one-byte heap overflow in fmtuserstring (smalloc(n) then t[n]=0 — the malloc 'corrupt allocation arena' panic) and an unterminated %s buffer in fmtrwdata — both fixed in-tree as the trace substrate's completion"
  - "mntgen measured READ-ONLY (a synthetic mount-point generator; tracesup's fragment creates were 'create prohibited') — ramfs (sys/src/cmd/ramfs.c, host-built) serves the writable /tmp; the plan's mntgen assumption corrected in k10draw.rc"
  - "the acme Execute gesture is a full-line B2 sweep with the press INSIDE the first character: the body's left strip is the scroll bar, and a press there scrolls and warps instead of selecting (the mouse event stream on the capture pinned the geometry)"
  - "sam driven through its command window (B on a missing name -> new file, a/text/ inserts typed runes over the IPC, ,s edits, w writes /tmp/sam-replay.txt): the menu-New path and a post-B typed line both measured dead on this venue (samterm which/Hcurrent + the non-empty-dot cut-stall — see deferred items); the B1 dot-reset click after B is the fix the committed script carries"
  - "ARCTEST-OK gates on the timed loop's own completion output: arctest falls off main and Harvey's _main epilogue calls exits(\"main\") — a failure status to rc's && (drawtest has explicit exits(0) and gates directly)"
  - "the utility class in tier3.ledger carries exec=STAGED:draw-venue-payload, not a TRACE: pointer — a utility earns its pointer when a plan drives its own scripted interaction, never by inheritance"
patterns-established:
  - "Hang-at-exec traced launch (hang/waitstop/nohang): the born-traced boundary starts at the exec'd image, no launcher noise"
  - "Replay vocabulary: home-clamp + absolute-target relative moves; in-place clicks for lasthit-warped menus; word-expansion clicks at a text line's right end for tag commands; full-line sweeps for Execute"
  - "Evidence pairing: raw per-pid framed logs committed verbatim + distilled tables via the committed distiller; the reproducibility proof (byte-identical re-run) is committed beside them"
requirements-completed: [USPACE-08]
actuals:
  tokens: 14841   # chars/4 over the authored diff (59,363 chars: build/ + sys/src/); the evidence artifacts (836KB of captures/raws/tables) are run output committed as evidence, not authored change — estimate was 65000, the same large over-projection family as 15-01/04/05/09 (the bring-up debugging budget does not show in the diff)
  tasks: 3
  commits: 3      # measured: git rev-list --count f1382b8d3..87a831fff
status: complete
coverage:
  - id: D1
    description: "Task 1 (tracer) — rio traced end-to-end: committed replay script (menu New + swept rc window + kbin-typed marker + menu Exit, natural exit), born-traced arming from the exec'd image, per-pid egress, raw+table evidence layers with the completion-marker gate green"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "sh build/mm/boot-9kdraw-trace.sh: exit 0; RIO-SCRIPT-DONE + TRACE-EGRESS-DONE-rio + TRACE-GATE-PASS; evidence 15-06-rio-{raw,table}.txt non-empty (11 pid blocks, rio/rc/echo binaries, 643 entries); tracer gate re-run POST-commit green (twice)"
        status: pass
      - kind: command
        ref: "static greps: mousein + kbin + RIO-SCRIPT-DONE in replay-rio.rc; startsyscall in tracesup.c; git diff --quiet HEAD -- build/aarch64/tracesup.c (A5 held — built by 6c unmodified)"
        status: pass
  - id: D2
    description: "Task 2 — acme + sam/samterm replays: the remaining trace set with both evidence layers; the sam table covers BOTH pids (sam + samterm, the born-traced descendants); every replay committed with its in-guest vocabulary"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "sh build/mm/boot-9kdraw-trace.sh (rio acme sam one boot): exit 0; ACME-SCRIPT-DONE + SAM-SCRIPT-DONE; 15-06-{acme,sam}-{raw,table}.txt non-empty (acme 1256 entries / sam 693 entries across sam+samterm+rc+echo); acme's Exec of /bin/rc -c with the marker command visible in the raw; sam's write of the edited file visible in the raw"
        status: pass
  - id: D3
    description: "Task 3 — the drawtest/arctest runtime re-join + the tier-3 ledger + the organized evidence dir (9 files, all captures committed)"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "sh build/mm/boot-9kdraw-trace.sh (full set + rejoin): exit 0; DRAWTEST-OK + ARCTEST-OK on the capture (both committed captures carry them); tier3.ledger 12 tier=3 lines, TRACE: pointers on rio/acme/sam/samterm; ls evidence/15-06-* | wc -l = 9 (>= 8)"
        status: pass
      - kind: command
        ref: "distill reproducibility (T-15-14): evidence/15-06-distill-repro.log — all three tables reproduced byte-identical over the committed raw logs (checkout-independent basename header)"
        status: pass
---

# Phase 15 Plan 6: Userspace Tiers + Bootfs Payload (amd64) — GUI Syscall Traces + Runtime Re-join Summary

**The measure-first GUI syscall traces captured on the draw donor — tracesup armed over born-traced rio/acme/sam trees driven by committed replay scripts (completion markers as the gate), fragments egressing over serial and distilled into per-binary syscall tables with PATH rows — plus the kernel trace substrate completed (syscallfmt's first 9k compile, two heap bugs fixed) and the 15-09 pixel-path re-join green (DRAWTEST-OK + ARCTEST-OK).**

## Performance

- **Duration:** 146 min
- **Started:** 2026-11-10T11:52:29Z
- **Completed:** 2026-11-10T14:18:00Z
- **Tasks:** 3/3 (Task 1 tracer)
- **Files:** 18 (13 created incl. 9 evidence files, 5 modified)

## Accomplishments

- **rio traced end-to-end (Task 1, tracer):** the committed replay drives the real interaction — menu New (menuhit's lasthit warp under the pointer), a swept rc window, the marker command typed into the window (its rc runs `echo RIO-SCRIPT-DONE > '#c/cons'`), menu Exit (natural exit, no kill fallback). The trace covers the full tree from the exec'd image: 11 pids, 643 entries across rio/rc/echo, with the /mnt/wsys mount, /srv postings and font loads in the PATH rows. The tracer gate re-ran the verify post-commit green.
- **acme + sam join the set (Task 2):** acme's Execute gesture is real (the typed command swept-selected in a new window's body and run through acme's `rc -c` — the Exec fragment with the full marker command is on the capture); sam's two-process pair round-trips the IPC (typed runes -> Ttype -> the host's buffer -> `,s` edit -> `w` write of /tmp/sam-replay.txt -> the `!echo` shell escape). 1256 + 693 entries.
- **The 15-09 re-join + the ledger (Task 3):** drawtest's full stochastic verification green and arctest's timed memarc loop green on the venue (both markers on both committed captures); tier3.ledger binds 12 artifacts to their evidence; the evidence dir carries raw/table pairs, both captures, and the byte-identical distill repro proof.
- **The kernel substrate completed (the unplanned-but-necessary):** 9k's /proc/N/syscall served NOTHING — syscall.c had the Proc_tracesyscall stops but never called the formatters, and syscallfmt.c had never been compiled (its first compile fixed a one-byte heap overflow — the `malloc: corrupt allocation arena` panic — and an unterminated buffer). ~40 lines in syscall.c + the mkfile OBJ line; k10cpu re-proven green in every full build.

## Task Commits

1. **Task 1: rio traced end-to-end** — `3680ad88f` (feat)
2. **Task 2: acme + sam/samterm replays** — `c9d135a45` (feat)
3. **Task 3: drawtest/arctest re-join + ledger + evidence** — `87a831fff` (feat)

**Plan metadata:** this SUMMARY commit (docs)

## Files Created/Modified

- `build/x86_64/replay-{rio,acme,sam}.rc` — the committed scripted interactions (D-05's measurement drivers: mousein gestures + kbin typing + completion markers)
- `build/mm/boot-9kdraw-trace.sh` — the trace venue harness (four-marker provenness rule, per-session completion gates, raw/table extraction)
- `build/x86_64/trace-distill.sh` — the committed raw->table derivation (per-pid/per-binary/PATH/TOTAL blocks, deterministic ordering, checkout-independent repro)
- `build/x86_64/tier3.ledger` — the GUI execution ledger
- `build/mm/build-9kdraw.sh` — tracesup/ramfs/drawtest/arctest/rcmain staging, the paq mount points, the baked session driver (hang launch + egress), the rejoin leg
- `sys/src/9k/k10/syscall.c` — the trace stops format and free the entry/return fragments (validaddr of the user stack first, waserror-guarded)
- `sys/src/9k/port/syscallfmt.c` — first 9k compile fixes: the fmtuserstring one-byte overflow, fmtrwdata's termination, negative-length guards, the validalign note
- `sys/src/9k/k10/mkfile` — syscallfmt.$O rides every CONF's link (frozen configs untouched)
- `sys/src/9k/k10/root/k10draw.rc` — ramfs /tmp, the /rc rcmain bind, the trace-mode hook
- `evidence/15-06-*` — 3x raw+table, 15-06-trace-venue.log (full-set capture), 15-06-venue-bringup.log (the full build's own boot leg), 15-06-distill-repro.log

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones: mousein measured relative (home-clamp discipline), the hang-at-exec launch replacing the KVM-racy spin window, the syscallfmt heap fixes as the substrate's completion, ramfs over mntgen, and the honest STAGED exec pointer for the utility class.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The 9k /proc/N/syscall substrate served nothing**
- **Found during:** Task 1 (salvage of the interrupted WIP)
- **Issue:** syscall.c had the Proc_tracesyscall stops but never called syscallfmt/sysretfmt (dead code), and syscallfmt.c had never been compiled in 9k — tracesup would have armed processes that produce no fragments.
- **Fix:** the legacy pc/trap.c discipline ported into syscall.c's two trace stops (waserror-guarded, validaddr of the user stack before the fmt walk, fragments freed after the stop returns); syscallfmt.$O added to the k10 mkfile's OBJ (every CONF, frozen configs untouched; k10cpu re-proven in every full build).
- **Files modified:** sys/src/9k/k10/syscall.c, sys/src/9k/k10/mkfile
- **Verification:** the venue boots produce formatted fragments; full builds green with the k10 tree git-clean.
- **Committed in:** 3680ad88f

**2. [Rule 1 - Kernel bug] syscallfmt.c's first compile: a one-byte heap overflow and an unterminated buffer**
- **Found during:** Task 1 (the hang-launched full-startup trace panicked `malloc: corrupt allocation arena`)
- **Issue:** fmtuserstring did `t = smalloc(n); ... t[n] = 0;` — a one-byte write past the allocation that smashes arena metadata when n hits the round-up boundary (the corruption surfaced only when full rio startup traced many long font-path strings); fmtrwdata never NUL-terminated its sanitizer buffer (an unterminated %s read); neither guarded the user-controlled length.
- **Fix:** n+1 allocations with the terminator written, negative-length clamps.
- **Files modified:** sys/src/9k/port/syscallfmt.c
- **Verification:** the panic disappeared across all subsequent boots (7+ full session boots, zero panic/trap asserted by every gate).
- **Committed in:** 3680ad88f

**3. [Rule 1 - Venue measurement] mousein is RELATIVE; the spin launch loses the KVM race; mntgen is read-only; /rc must carry rcmain**
- **Found during:** Task 1 (salvage + bring-up; the interrupted executor's own capture held the evidence)
- **Issue:** (a) `M x y b m` writes feed mousetrack as DELTAS — the absolute-positioned replay clicked a clamped-corner menu and no window was created; (b) the 06-07 spin window: the launcher rc outran the arming stop through the paq-served /bin, so the trace began mid-rio (exec/fonts/mouse-kbd procs all pre-dated arming); (c) this tree's mntgen is a read-only synthetic mount-point generator — tracesup's fragment creates failed; (d) rc sources an rcmain unconditionally — rio's rc -i windows, acme's Execute and sam's !cmd (all `rc -i`/`rc -c`) died sourcing /rc/lib/rcmain before running anything.
- **Fix:** (a) home-clamp + known absolute deltas in every replay; (b) hang-at-exec launch (waitstop blocks until the image stops at its first instruction; nohang clears the flag for descendants); (c) ramfs serves /tmp; (d) rcmain staged in the paq and /mnt/rc bound at /rc.
- **Files modified:** build/x86_64/replay-*.rc, build/mm/build-9kdraw.sh, sys/src/9k/k10/root/k10draw.rc
- **Verification:** all three completion markers green; natural exits; the mouse event streams decoded from the captures confirm every gesture landed as designed.
- **Committed in:** 3680ad88f, c9d135a45

**4. [Rule 1 - Program geometry] The acme Execute sweep pressed on the scroll bar**
- **Found during:** Task 2 (acme bring-up)
- **Issue:** the B2 sweep's press at x=2 hit the body's left strip — acme scrolled and warped the pointer instead of selecting (the mouse events on the capture showed the press followed by a warp to the strip's center); the executed selection was empty and no marker fired.
- **Fix:** the press sits inside the first character (x=15); the drag runs the line's full width.
- **Files modified:** build/x86_64/replay-acme.rc
- **Verification:** ACME-SCRIPT-DONE with the full command in the Exec fragment.
- **Committed in:** c9d135a45

**5. [Rule 3 - Program state machine] sam's menu-New and post-B typing measured dead on this venue**
- **Found during:** Task 2 (sam bring-up, 5 boots)
- **Issue:** (a) the menu's New leaves samterm's `which` unset pending the host's Hcurrent — a second B3 press re-opens the menu (the +1px menuhit warps pin it) and swept windows never form; (b) after a `B` whose reply leaves a non-empty dot on the command window, `type()` takes the cut-selection branch and returns WITHOUT consuming the pending runes — the first typed line after the file command stalls behind the kbdblock mask (the kbd ioproc's reads stop on the capture).
- **Fix:** the committed replay drives the command window: `B` on a MISSING name (the new-file path — no readfile storm), a B1 dot-reset click after B, then single-line commands (`a/text/` types into the file over the IPC, `,s` edits, `w` writes). The root causes are logged as open deferred items (a real fix owns samterm's io.c/main.c).
- **Files modified:** build/x86_64/replay-sam.rc, build/mm/build-9kdraw.sh
- **Verification:** SAM-SCRIPT-DONE; the raw log shows sam's Create + Pwrite of "hello from the 15-06 sam REPLAY" and both processes' natural exits.
- **Committed in:** c9d135a45

**6. [Rule 3 - Runtime quirk] arctest falls off main and Harvey's _main epilogue exits "main"**
- **Found during:** Task 3 (the rejoin bring-up)
- **Issue:** `arctest 1000 && echo ARCTEST-OK` never fired the marker although the timings printed: main9.s's epilogue calls exits("main") for programs without an explicit exits — a failure status to rc's `&&` (drawtest has exits(0) and gated directly).
- **Fix:** ARCTEST-OK gates on the timed loop's own completion output (redirect, cat onto the capture, test -s), with the harness's zero-panic/trap assert as the without-faulting half — exactly the contract's letter. Logged as an open deferred item (a runtime-level decision).
- **Files modified:** build/mm/build-9kdraw.sh
- **Verification:** ARCTEST-OK on both committed captures.
- **Committed in:** 87a831fff

---

**Total deviations:** 6 auto-fixed (2 kernel bugs, 1 venue-measurement cluster, 1 program geometry, 2 program/runtime state machines) + 2 open deferred items. **Impact on plan:** all inside the plan's own disciplines — the measurement surface (scripted interactions, completion gates, both evidence layers) is exactly as planned; the kernel deltas complete a half-ported feature (the trace stops existed but never formatted).

## Authentication Gates

None.

## Known Stubs

None — every marker is earned by a real runtime path; the gates assert script completion (RIO/ACME/SAM-SCRIPT-DONE), egress completion (TRACE-EGRESS-DONE-*), and the rejoin contract (DRAWTEST-OK/ARCTEST-OK) on captures that also carry the four bring-up markers (the venue-provenness rule).

## Issues Encountered

- The interrupted-WIP salvage: the previous executor's kernel deltas (syscall stops + fmt calls) were coherent and proven by their own capture — kept and completed (the validaddr guard, the pre-nil leak fixes); the session driver's spin launch, unguarded redirects and missing rcmain were re-measured and replaced as documented above.
- Two throwaway calibration boots used qemu monitor screendumps (converted to PNG host-side) to read the acme layout (columns at x=614, tag lines at y≈8/24) and the rio panic — the monitor-socket option is a committed harness flag (TRACE_MONITOR).
- The plan's own verify command greps 'ARCTEST-OK' against the harness STDOUT, which would false-positive on the FAIL line "ARCTEST-OK missing"; verification here greps the CAPTURE (the harness classifies against the capture only — the gate itself is honest). The captures carry the markers natively.

## User Setup Required

None.

## Next Plan Readiness

READY for 15-07 (same-wave peer: this plan's only /amd64-touching phase was the payload bake under the build-tree lock — held throughout) and 15-08 (the evidence bundle: 9 committed 15-06-* evidence files, the raw/table grammar in 15-06-distill-repro.log, the ledger, and the A5 record — tracesup.c built by 6c unmodified at HEAD). NOTE for Phase 17+: the distilled tables (with PATH rows folding /proc/N and .N suffixes) are the measure-first input for r9 kernel growth; the replay vocabulary (home-clamp, word-expansion clicks, full-line sweeps) is the established pattern for any future GUI trace plan.

## Self-Check: PASSED

- files exist: replay-{rio,acme,sam}.rc, trace-distill.sh, boot-9kdraw-trace.sh, tier3.ledger + the 9 evidence files + the 5 modified ✓
- commits are ancestors of HEAD: 3680ad88f, c9d135a45, 87a831fff ✓
- plan-level verification green at HEAD: final full harness run (throwaway evidence dir) exit 0 — TRACE-GATE-PASS, all five session markers + both rejoin markers on one capture, zero panic/trap ✓

## Self-Check: PASSED

- files exist: all 20 key files (6 created scripts/ledger + 9 evidence + 5 modified) — checked with [ -f ] at SUMMARY time
- commits are ancestors of HEAD: 3680ad88f, c9d135a45, 87a831fff ✓
- plan-level verification green at HEAD: final full harness run exit 0, TRACE-GATE-PASS, all five session markers + both rejoin markers on one capture, zero panic/trap ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-10*
