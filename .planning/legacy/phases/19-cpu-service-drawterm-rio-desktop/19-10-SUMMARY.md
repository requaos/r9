---
phase: 19-cpu-service-drawterm-rio-desktop
plan: "10"
subsystem: testing (DESK-07 human acceptance + INT-07 clean-checkout proof + the linux-leg D-11 fix loop)
tags: [DESK-07, INT-07, human-acceptance, 19-EVIDENCE, signed-rows, clean-checkout, drawterm, rio, D-11, pin-advance, owner-deferral, aarch64-reopener]
requirements: [DESK-07, INT-07]

# Dependency graph
requires:
  - phase: 19-cpu-service-drawterm-rio-desktop/19-09
    provides: --system=cpu full-system mode + the two-leg composed smoke — the linux session's substrate and the INT-07 proof's standing gates
  - phase: 19-cpu-service-drawterm-rio-desktop/19-08
    provides: the -G interop gate + the concurrency-corrected kernel at the df4ad78 pin (the loop's starting pin)
  - phase: 19-cpu-service-drawterm-rio-desktop/19-06
    provides: the drawterm wrapper + the README recipes the human sessions exercise
provides:
  - DESK-07's evidence record, closed: Row 1 (the linux graphical leg) SIGNED — human-accepted 2026-11-16, Neil Skinner, r9 0921a1bc, screenshot evidence/19-10-linux-rio.png; Row 2 (the darwin -G leg) DEFERRED BY OWNER DECISION 2026-11-17 with the aarch64-port reopener; the darwin graphical deferral stands (2026-11-14, gui-cocoa reopener)
  - INT-07 closed: the v2.1 chain reproduces from TWO independent fresh clones of harvey at e4001b383 (r9 solely via the flake.lock github pin df4ad78) — boot-r9-smoke-x64 and drawterm-gate-x64 both exit 0, ALL STAGES PASS
  - the linux leg's D-11 fix loop landed: the phase's longest kernel/userspace fix chain (ten layers), each live-verified by the owner before the next, advancing the r9 pin df4ad78 → 0921a1bc across six lock-only choreography advances (0921a1b == pin == origin/main)
affects: [the aarch64 cpu port reopener (owner-performed future work — the DESK-07 darwin -G row reopens against it), the gui-cocoa darwin reopener (future milestone), milestone close + phase verification]

actuals:
  tokens: 236494   # chars/4 over the realized plan-window diff (945,975 diff chars, e39585500..d4f0eeae5; evidence consoles + captures included). The 42K estimate anticipated records-only scope — the D-11 loop (10 fix layers, 6 pin advances, ~40 evidence artifacts) was the miss
  tasks: 3
  commits: 28      # MEASURED: git rev-list --count e39585500..d4f0eeae5
plan_head_before: e39585500392c203237e3b9fcf56feefab0800c7
plan_head_after: d4f0eeae52000c1c50b2766f5dd2501364f290e5

tech-stack:
  added: []         # zero packages (T-19-SC honored throughout)
  patterns:
    - "Signed-row evidence discipline (D-13): a human-acceptance row becomes evidence ONLY when signed (date, venue, r9 SHA, accepter) with a screenshot that resolves; an unsigned row is a scaffold and says so on its face"
    - "Owner-decision deferral records: the deferral block carries the verbatim decision intent, the declined alternatives (SSH-tunnel venue, mac-local x86_64 TCG), the phase-scale scoping of the reopener work, the reopener itself, and the standing-evidence pointer — never a silent completion"
    - "The D-11 fix loop as the acceptance path: every owner-reported residual is root-caused live, fixed push-before-pin with the full FOD choreography, pinned, and re-presented to the owner — never signed while broken"

key-files:
  created:
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/19-EVIDENCE.md   # Row 1 signed; Row 2 deferral block; the INT-07 clean-checkout record
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-10-linux-rio.png   # the signed linux-leg screenshot (PNG 2616x1690)
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-10-clean-smoke.console   # INT-07 smoke leg driver console (149140 bytes, zero DEMO-password occurrences)
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-10-clean-gate.console   # INT-07 -G gate leg driver console (144396 bytes, redacted)
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-10-leg1-*   # the D-11 loop's ~40 captures: authid repro/discriminator/fix-verify, substrate + grey desktop, continuation proofs (profile exoneration, identity-key, two-window, cursor sprite, exit-deletes), patches, proof records
    - authdomain.local.EXAMPLE   # the repo-root recipe for a developer's local p9sk1 domain (the 18-D-01 shape the owner's session used)
  modified:
    - nix/drawterm-x64.sh                  # the -s '' secstore-park fix (8a3fdcb5d)
    - sys/src/cmd/unix/drawterm/gui-x11/x11.c   # Shift+Left middle-button emulation (bd7ac465a)
    - build/x86_64/root-tree/usr/glenda/lib/profile   # the /mnt/wsys guard — the ghost-rio root cause (d70c4efb6)
    - build/x86_64/ (the §15 substrate: /bin/test return, rio/plumber/ramfs/syscall/awd suite, glenda's home, plumb rules — 27dbdb63d)
    - build/x86_64/README.md               # recipe re-keys to the staged-test truth (e218b6a3d)
    - flake.lock                           # six lock-only pin advances df4ad78 → 0921a1bc
    - .planning/REQUIREMENTS.md            # DESK-07 (accepted with recorded deferral) + INT-07 (complete)
    - .planning/STATE.md                   # decisions, deferred items, position 10/10

key-decisions:
  - "DESK-07 closes as ACCEPTED WITH RECORDED DEFERRAL per the owner resolutions: the linux graphical row signed 2026-11-16 (the D-13 discipline satisfied — who/when/venue/SHA/screenshot); the darwin -G row deferred by owner decision 2026-11-17 (verbatim intent: 'aarch64 cpu port, but just note it so that I can move back to the mac and perform the work natively on the aarch64 machine' — the SSH-tunnel venue and a mac-local x86_64 TCG substitute both declined); the darwin graphical row deferred 2026-11-14 (gui-cocoa reopener). Standing evidence: the signed Row 1 + INT-07's automated proof"
  - "The darwin -G deferral's reopener is the aarch64 cpu port, explicitly the OWNER's own future work on the mac — phase-scale work (DTB bootconf seam, auth-suite tiers, §15 mkimage contract, devssl #D registration, drawterm gate, all x86_64-only today); aarch64 stays frozen at the v2.0 pin until that port opens"
  - "Row 1 was signed only after the longest D-11 loop of the phase: ten fix layers, each live-verified by the owner before the next, six lock-only pin advances; the loop was plan-anticipated (Task 1's own routing: failures go to the D-11 fix loop before re-inviting acceptance — never sign a broken leg)"
  - "INT-07's proof is automated and venue-honest: two INDEPENDENT fresh clones, r9 consumed solely through the flake.lock github pin (no sibling clone), the DEMO domain bakes green on clean checkouts by design (18-D-01)"

patterns-established:
  - "Deferral-block shape for evidence files: Status = DEFERRED BY OWNER DECISION + date; the verbatim decision intent; the declined alternatives; the phase-scale scoping; the reopener; the standing-evidence pointer; the historical scaffold kept inside for the reopener (never exercised, never claimed)"

requirements-completed: [DESK-07, INT-07]

coverage:
  - id: D1
    description: "Task 3 — INT-07 clean-checkout proof: the full v2.1 chain reproduces from fresh clones of harvey on linux-x86_64 (r9 via the flake.lock github pin at the locked rev), plus the deferral records"
    requirement: INT-07
    verification:
      - kind: e2e
        ref: "CLEAN-SMOKE-GREEN: fresh clone at e4001b383 -> nix run .#boot-r9-smoke-x64 exit 0, ALL STAGES PASS (chain + cpu legs; console evidence/19-10-clean-smoke.console)"
        status: pass
      - kind: e2e
        ref: "CLEAN-GATE-GREEN: a second independent fresh clone at e4001b383 -> nix run .#drawterm-gate-x64 exit 0, ALL STAGES PASS (p9any->p9sk1, rc4_256/sha1 session, teardown, capture redacted; console evidence/19-10-clean-gate.console)"
        status: pass
      - kind: other
        ref: "RECORDS-OK: REQUIREMENTS.md DESK-07 deferral annotation + STATE.md deferred-items row + decision rows (the plan's Task 3 verify)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Task 1 — the linux graphical leg: the rio desktop in a drawterm window, human-accepted"
    requirement: DESK-07
    verification:
      - kind: manual_procedural
        ref: "19-EVIDENCE.md Row 1 — SIGNED 2026-11-16 by Neil Skinner (owner) at r9 0921a1bc, linux-x86_64 KVM venue; screenshot evidence/19-10-linux-rio.png resolves; session notes record in-session desktop start, keystrokes, contextual cursors, menu re-open, second window, exit-deletes-window, and the DESK-05 plumbing observation"
        status: pass
    human_judgment: true
    rationale: "A desktop session's acceptance is a human judgment by design (D-13): the owner exercised the session live and accepted it; no automation can substitute for the acceptance of an interactive graphical session (the plan's own blocking-human gate)"
  - id: D3
    description: "Task 2 — the darwin -G text leg over the SSH tunnel: DEFERRED BY OWNER DECISION 2026-11-17, not executed"
    requirement: DESK-07
    verification: []
    human_judgment: true
    rationale: "Deferred by owner decision 2026-11-17 (verbatim intent recorded in 19-EVIDENCE.md Row 2): the owner declined the SSH-tunnel venue and a mac-local x86_64 TCG substitute; the row reopens against the owner-performed aarch64 cpu port. No session ran and no screenshot exists — the verifier must see the recorded deferral and its reopener, never an auto-pass"

duration: ~46h wall across three passes (2026-11-15 first pass + checkpoint; 2026-11-15→16 the linux-leg D-11 loop; 2026-11-16 owner acceptance; 2026-11-17 this records pass)
completed: 2026-11-17
status: complete
---

# Phase 19 Plan 10: Human Legs + Clean-Checkout Proof (DESK-07, INT-07) Summary

**DESK-07 closed as accepted-with-recorded-deferral — the linux graphical rio leg human-accepted and SIGNED at r9 0921a1bc after the phase's longest D-11 fix loop (ten layers: the wrapper secstore park, the local-domain recipe, the authid server-key fix, the §15 desktop substrate + r9 iounit, Shift+Left middle-button emulation, identity-keyed mount consults, close-on-exec + the profile /mnt/wsys guard, the devmnt zero-length-write cursor fix, the exit-deletion fd-drain/closepgrp/mountmux fixes — six lock-only pin advances df4ad78 → 0921a1b), the darwin -G leg deferred by owner decision 2026-11-17 with the aarch64-port reopener, and INT-07's clean-checkout proof GREEN from two independent fresh clones**

## Performance

- **Duration:** ~46h wall across three passes (active executor time concentrated in the 2026-11-15→16 D-11 loop)
- **Started:** 2026-11-15T14:35Z (the scaffold commit e4001b383)
- **Completed:** 2026-11-17 (this records pass; Row 1 signed 2026-11-16T01:59Z at d4f0eeae5)
- **Tasks:** 3/3 (Task 3 automated-green; Task 1 human-accepted; Task 2 owner-deferred with reopener)
- **Files modified:** 62 (the plan window e39585500..d4f0eeae5 — production + records + ~40 evidence artifacts) + this records pass

## Accomplishments

- **Task 3 (first pass, 4f54e8c6c + e4001b383 + 28126b2db) — INT-07 clean-checkout proof, GREEN both gates.** Two INDEPENDENT fresh clones of harvey at e4001b383 (r9 consumed solely through the clone's flake.lock github pin at df4ad78 — the r9 input is type github, nix fetches the locked commit itself; no sibling clone participated): `nix run .#boot-r9-smoke-x64` exit 0 ALL STAGES PASS (the chain leg + the cpu-chain leg through the full cpurc ladder, zero FAIL, zero panic, zero desktop font-line markers) and `nix run .#drawterm-gate-x64` exit 0 ALL STAGES PASS (the encrypted -G session end-to-end, teardown, capture redacted). Consoles retained (evidence/19-10-clean-{smoke,gate}.console, both verified zero DEMO-password occurrences before retention). The deferral records landed the same pass (REQUIREMENTS DESK-07 annotation, STATE deferred-items row + decision rows).
- **Task 1 (the linux graphical leg) — human-ACCEPTED 2026-11-16 after the longest D-11 fix loop of the phase.** The owner followed the README recipe (`nix run .#boot-r9-x64 -- --system=cpu` left running, then `nix run .#drawterm` with the local-file domain credentials — the 18-D-01 shape, not DEMO), accepted the rio desktop in the drawterm window, exercised it (windows, commands, plumbing), and took the screenshot; Row 1 signed at d4f0eeae5 (r9 0921a1bc; screenshot evidence/19-10-linux-rio.png present). The loop's ten layers, each live-verified by the owner before the next:
  1. **the wrapper secstore park** — the human session's drawterm parked at a secstore prompt; the wrapper now carries `-s ''` (8a3fdcb5d)
  2. **authdomain.local.EXAMPLE** — the repo-root recipe for a developer's local p9sk1 domain, so the owner's real credentials ride a committed interface (d1ab2cd9e)
  3. **the authid server-key fix** — the cpu image's p9sk1 server key must be the authid's own, not the host's (the Leg-1 srvauth death root-caused with repro/discriminator/fix-verify/gate-regression evidence; 92be73fd2 + 97eb9dad5)
  4. **the §15 desktop substrate** — /bin/test returns to the image + the desktop suite (rio/plumber/ramfs/syscall/awd) + glenda's home + the plumb rules, with the r9 iounit fix under the dae4397 pin (27dbdb63d + a562a5b9d; recipe re-key e218b6a3d; substrate evidence adf46705c)
  5. **Shift+Left middle-button emulation** in drawterm gui-x11 — the venue has no middle button (bd7ac465a)
  6. **the mount-identity/unmount fixes** — r9's mount-table consults re-keyed from path-string to chan-identity (the donor findmount shape; the dual-cons root cause proven live on the owner's guest) + sys_unmount's arg resolutions aligned to the donor; pin dae4397 → 56109c3 (ff21c4786 + 855dd08fa; continuation evidence 9e6664784, 593787263, 402cb1153)
  7. **close-on-exec + the profile /mnt/wsys guard** — the ghost-rio root cause: every window shell re-sourced the cpu profile and spawned a NESTED rio because the guard checked only /mnt/term/mnt/wsys (d70c4efb6, the flat dialect-safe shape); r9 gained the donor OCEXEC/CEXEC sweep (r9 99714a6 → pin 3e5ed0c); the post-sweep mouse death landed with the two-window proof (c8a7db892 + 6d6190676 + a02d03637)
  8. **the devmnt zero-length-write cursor fix** — the cursor revert rides a ZERO-length write; r9's devmnt::write was while-loop (sent nothing) where the donor mntrdwr is do-while (one Twrite with count=0 observed); proven with real X-sprite observation; pin 3e5ed0c → c787ce9 (d109744b2 + a6b7e6a97)
  9. **the exit-deletion fixes** — `exit` never deleted the window: exit_tail's fd drain closed under a borrow + the donor closepgrp mount half was never ported; the new exit-time clunks exposed the mux regression (late replies killing healthy waiters) closed by restoring the donor mountmux DROP arm; pins c787ce9 → 543c375 → 0921a1b (a190e6bd3 + 3f449fd72; landed record 356aecb93)
  10. **the final pin 0921a1b** — 0921a1b == flake.lock pin == r9 origin/main; gates re-green at every advanced pin (58992b603); Row 1 signed at it (d4f0eeae5)
- **Task 2 (the darwin -G leg) — DEFERRED BY OWNER DECISION 2026-11-17, recorded with the reopener.** The owner declined the recorded SSH-tunnel venue and a mac-local x86_64 TCG substitute; the chosen path is a native aarch64 cpu port, owner-performed on the darwin/aarch64 machine — phase-scale work (DTB bootconf seam, auth-suite tiers, §15 mkimage contract, devssl #D registration, drawterm gate, all x86_64-only today). Row 2 rewritten as the deferral block (the historical scaffold kept inside for the reopener; the recipe honestly recorded as never-exercised); DESK-07's standing evidence is the signed Row 1 + the INT-07 record. The darwin GRAPHICAL deferral stands unchanged (owner decision 2026-11-14, gui-cocoa reopener, XQuartz rejected).

## Task Commits

Each task's commits landed atomically per layer (the D-11 loop's choreography: r9 fixes push-before-pin in the sibling repo — see deferred-items.md for their SHAs — with harvey-side pins + records enumerated here):

1. **Task 3: scaffold + deferral records + INT-07 clean-checkout proof** — `e4001b383` (docs: evidence scaffold), `4f54e8c6c` (feat: deferral records + INT-07 green both gates), `28126b2db` (docs: checkpoint-pending state)
2. **Task 1: the linux graphical leg — the D-11 loop + acceptance** — `8a3fdcb5d` (fix: wrapper -s ''), `d1ab2cd9e` (docs: authdomain.local.EXAMPLE), `92be73fd2` (fix: authid server key) + `97eb9dad5` (test: authid evidence), `27dbdb63d` (feat: §15 desktop substrate) + `a562a5b9d` (chore: pin df4ad78→dae4397) + `e218b6a3d` (docs: recipe re-key) + `adf46705c` (test: substrate evidence), `bd7ac465a` (feat: Shift+Left emulation), `9e6664784` + `593787263` (test/docs: continuation — profile exonerated), `402cb1153` (test: continuation -G gate capture), `ff21c4786` (chore: pin dae4397→56109c3, identity-keyed mounts) + `855dd08fa` (docs: dual-cons root cause), `58992b603` (docs: gates re-green at pin), `c8a7db892` (docs: post-sweep death root-caused) + `d70c4efb6` (fix: profile /mnt/wsys guard) + `6d6190676` (chore: pin 56109c3→3e5ed0c, close-on-exec) + `a02d03637` (docs: two-window proof), `d109744b2` (chore: pin 3e5ed0c→c787ce9, zero-count Twrite) + `a6b7e6a97` (docs: cursor fix proven), `a190e6bd3` (chore: pin c787ce9→543c375) + `3f449fd72` (chore: pin 543c375→0921a1b final) + `356aecb93` (docs: exit-deletion landed), `d4f0eeae5` (docs: Row 1 SIGNED)
3. **Task 2: the darwin -G deferral records** — no in-loop commits; the deferral record (19-EVIDENCE.md Row 2 + REQUIREMENTS + STATE) lands in this close-out commit

**Plan metadata:** (this SUMMARY + the record flips — the docs commit follows)

## Files Created/Modified

- `.planning/phases/19-cpu-service-drawterm-rio-desktop/19-EVIDENCE.md` — Row 1 signed (immutable history at d4f0eeae5); Row 2 the deferral block; the INT-07 record; the pin-history note; CLOSED status
- `evidence/19-10-linux-rio.png` + `evidence/19-10-clean-{smoke,gate}.console` + `evidence/19-10-leg1-*` (~40 artifacts) — the signed screenshot, the clean-checkout consoles, the D-11 loop's repro/discriminator/proof captures
- `nix/drawterm-x64.sh`, `sys/src/cmd/unix/drawterm/gui-x11/x11.c`, `build/x86_64/root-tree/usr/glenda/lib/profile`, the §15 substrate files, `build/x86_64/README.md`, `authdomain.local.EXAMPLE`, `flake.lock` — the loop's harvey-side production layers
- `.planning/REQUIREMENTS.md` — DESK-07 accepted-with-recorded-deferral; INT-07 complete (both traceability rows Complete)
- `.planning/STATE.md` / `.planning/ROADMAP.md` — position 10/10, decisions, deferred items, progress

## Decisions Made

See key-decisions. Additionally: the Row-1 signing was recorded at the owner's direction following their acceptance ("linux leg accepted", 2026-11-16) — the D-13 who/when/venue/SHA fields are the owner's own session facts; and the deferral records deliberately mirror the 2026-11-14 darwin-graphical precedent (verbatim intent + declined alternatives + reopener + REQUIREMENTS/STATE annotation, never a silent completion).

## Deviations from Plan

### Owner-decision deviation (checkpoint outcome, not an auto-fix)

**1. Task 2 (darwin -G leg) deferred by owner decision instead of signed**
- **Found during:** Task 2's blocking-human checkpoint (the darwin session never ran)
- **Issue:** The plan's must-have required the darwin -G row signed after a real session over the SSH tunnel. The owner declined that venue — and a proposed mac-local x86_64 TCG substitute — choosing instead to port the cpu stack to aarch64 natively and perform the darwin leg against that port (owner-performed, phase-scale: DTB bootconf seam, auth-suite tiers, §15 mkimage contract, devssl #D registration, drawterm gate)
- **Resolution:** Deferral recorded per the owner's 2026-11-17 decision (verbatim intent in 19-EVIDENCE.md Row 2): the row rewritten as a deferral block with the historical scaffold kept inside, the reopener named, REQUIREMENTS DESK-07 annotated accepted-with-recorded-deferral, STATE carries the decision + the Deferred Items row. The plan's own prohibition ("no unexercised recipe") is honored — the recipe is recorded as never-run, not as evidence
- **Files modified:** 19-EVIDENCE.md, REQUIREMENTS.md, STATE.md
- **Verification:** Row 2 carries DEFERRED BY OWNER DECISION 2026-11-17 + the verbatim intent + the reopener; DESK-07's annotation names all three legs; the deferral mirrors the 2026-11-14 precedent
- **Committed in:** the close-out docs commit

The linux leg's D-11 umbrella invocations were plan-anticipated (Task 1's own routing: "the executor routes failures into the D-11 fix loop before re-inviting acceptance") — each layer landed push-before-pin with the full FOD choreography and owner verification, so they are accomplishments, not deviations.

---

**Total deviations:** 1 owner-decision deviation (Task 2 deferral); 0 auto-fixed
**Impact on plan:** The deferral is owner-ratified scope resolution, not incomplete work — DESK-07 closes as accepted-with-recorded-deferral with the standing evidence named; the reopener (the owner-performed aarch64 cpu port) is recorded in three places (19-EVIDENCE Row 2, REQUIREMENTS DESK-07, STATE Deferred Items).

## Issues Encountered

The D-11 loop's residuals are recorded in the phase's deferred-items.md with named reopeners, none blocking the owner's acceptance: the login-time desktop auto-start skip on app-baked venues (manual `plumber; rio` green — the documented fallback; reopener: the next §15-bake plan); the §14 auth-sweep DEMO-dom hardcode (reopener: the next auth-gate run on a checkout carrying authdomain.local); the synthetic-XTEST mouse finding and the aborted-connection auth poisoning (reopener: the first drawterm-side plan / the next r9 kernel pass); the /srv and session-/tmp findings (reopener: the first r9 devsrv pass).

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced. The deferral records are explicit records, not stubs.

## User Setup Required

None remaining this milestone — the plan's user_setup (the darwin SSH-tunnel session) is superseded by the owner's 2026-11-17 deferral decision; the reopener carries the setup into the future aarch64 cpu port work.

## Next Phase Readiness

- Phase 19 execution is COMPLETE (10/10 plans); phase verification is the orchestrator's next step (not performed here, per dispatch)
- The v2.1 milestone's every requirement is now Complete in REQUIREMENTS.md (46/46; DESK-07 accepted-with-recorded-deferral, INT-07 green)
- Standing reopeners for future milestones, all recorded with owners: the aarch64 cpu port (owner-performed; the DESK-07 darwin -G row reopens against it), the gui-cocoa darwin backend (the graphical row), and the D-11 residuals above
- r9 stands at 0921a1b == flake.lock pin == origin/main; harvey at d4f0eeae5; both gates (drawterm -G + two-leg smoke) ALL STAGES PASS at the final pin

## Self-Check: PASSED

- Created/modified files exist: 19-EVIDENCE.md, evidence/19-10-linux-rio.png, evidence/19-10-clean-{smoke,gate}.console, the leg1 evidence set, REQUIREMENTS/STATE/ROADMAP, this SUMMARY — all FOUND
- Plan commits exist: all 16 spot-checked commits (e4001b383 … d4f0eeae5) are ancestors of HEAD; the measured count is 28 (git rev-list --count e39585500..d4f0eeae5)
- Row 1 (the signed linux row) is intact and untouched by this pass; Row 2 carries the deferral block with the verbatim decision + reopener
- REQUIREMENTS flips verified: DESK-07 + INT-07 checked, both traceability rows Complete
- The darwin -G deferral is noted everywhere it must be: 19-EVIDENCE Row 2, REQUIREMENTS DESK-07, STATE decisions + Deferred Items

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-17*
