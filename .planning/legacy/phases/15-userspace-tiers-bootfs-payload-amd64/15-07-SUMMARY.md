---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 07
subsystem: tier-machinery
tags: [tiers, tier4, coverage-proof, dir-coverage, mk-filters, fossil, venti, gs, pcc, ape, paqfs, rc-dialect, uspace-09, x86_64]
duration: "continuation ~50 min + the interrupted executor's unrecorded start (wall ≤ 3h 44m; first commit 16:10Z, interruption ~17:10Z, continuation first commit 17:28Z)"
completed: 2026-11-10
requires:
  - "15-01-complete tier machinery (the COVERAGE_SWEEP=1 escape verified at all six filter sites, the phase build-tree lock, the aux leaf filter pattern)"
  - "15-04-complete three-file filter union + tier data this plan widens by one file per site"
  - "15-05-complete draw donor venue (the -initrd payload channel + k10draw.rc trace-session hook the gs smoke rides)"
provides:
  - "build/x86_64/coverage-sweep.sh — the amd64 full-tree DIRS-coverage driver (COVERAGE_SWEEP=1, per-dir VISIT/EXIT markers, per-invocation timeout watchdog, error digests, the tier4 exclusion record as its single source, the gs harvest, the droppings sweep)"
  - "build/x86_64/dir-coverage.txt — the machine-verified GREEN proof: 702 rows, zero not-ok, 1353 VISIT / 1353 EXIT (D-09: full-set, never sampled; fossil/venti rows carry the runtime-unverifiable annotation inline)"
  - "build/x86_64/tiers/tier4-tail.txt — the long-tail allowlist (112 cmd dirs + 6 aux subdirs incl. the aarch64 'x86-hardware-only' class re-derived as MEMBERS) + the re-derived exclusion record (7 named '# exclude:' entries with measured digests)"
  - "the six mk filter sites on the four-file union awk (one TIER4A variable + one filename per site; no tier-4 driver loop exists BY DESIGN — the D-09 proof is the tier's gate)"
  - "three in-tree class-fix commits (4c's rfork/bind $LIB portable lex-object override + its ambiguous-recipes repair, 5c's stale committed enam.c regenerated, libdynld's dead ULONG<0 check dropped — the Phase-6 checklist item re-derived live)"
  - "build/x86_64/gs-render.ps + build/mm/boot-9kgs-smoke.sh — the D-11 render smoke: gs executes IN GUEST on the donor venue and renders the font-free fixture to a 43269-byte PPM bitmap through the FILE device (-sDEVICE=ppmraw; gdevbit is not in this build's link set, gdevpbm is) — the 753-file LP64-fossil habitat proven beyond compile green, with zero font payloads staged anywhere"
affects:
  - "15-08 (the evidence bundle consumes the green dir-coverage.txt + the tier4 record + the gs capture)"
  - "future fossil/venti runtime plans (the documented ceiling: no block device in r9 — virtio-blk is a future milestone)"
  - "Phase-17+ r9 plans (the coverage-proof venue + the paqfs blocksize/rc-dialect patterns recorded here)"
tech-stack:
  added: []
  patterns:
    - "mkpaqfs caps every file at blocksize^2/4 and silently DROPS oversize files (warn, exit 0) — any payload bake carrying a multi-MiB member must pass -b and fail on mkpaqfs diagnostics (gs 15.5MB vs the 4MiB default cap, measured)"
    - "Harvey's rc lexer (lex.c wordchr) ends a word at every bare '=' — =-bearing argv (-sDEVICE=, -sOutputFile=) must be single-quoted in in-guest rc scripts; plan9port's rc accepts the bare form, so host-side rc checks never see the class"
    - "rc's Execute prints only the LAST path candidate's errstr — absolute-path invocations in guest sessions keep the real failure visible"
    - "an added recipe for a target mkone/mkmany already defines (clean:V:) is 'ambiguous recipes', a parse-time mk failure — extend by prerequisite (clean: dep), never by a second recipe"
    - "gs's init chain opens non-.ps config tables unconditionally (xlatmap via gs_fntem.ps, FAPIconfig via gs_fapi.ps) — the init library is the .ps set plus the measured-minimal config set, never the font payloads"
key-files:
  created:
    - build/x86_64/coverage-sweep.sh
    - build/x86_64/dir-coverage.txt
    - build/x86_64/tiers/tier4-tail.txt
    - build/x86_64/gs-render.ps
    - build/mm/boot-9kgs-smoke.sh
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-07-gs-smoke.log
  modified:
    - sys/src/cmd/4c/mkfile
    - sys/src/cmd/5c/enam.c
    - sys/src/libdynld/dynld.c
    - sys/src/mkfile
    - sys/src/cmd/mkfile
    - sys/src/cmd/ip/mkfile
    - sys/src/cmd/auth/mkfile
    - sys/src/cmd/auth/factotum/mkfile
    - sys/src/cmd/aux/mkfile
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-01-scoping-proof.sh
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-01-scoping-proof.log
key-decisions:
  - "The 7 not-fixable failures are re-derived upstream-disabled/dead-tool exclusions, each with its measured digest in tier4's named '# exclude:' record — independently re-verified at continuation time against the tree (postscript/mkfile's literal '# UNIX remnants' TARGETS block, tcs/font's no-`all`/links--lg-with-no-libg mkfile, games/mkfile's commented-out blabs whose mkface drives the nonexistent cmd/fb); the excluded dirs are never visited by the sweep and a stale exclusion fails the run"
  - "sys/src/cmd/games does not exist in this tree (the aarch64 plan's named fact, re-verified); the games ROOT stays outside the default amd64 build surface exactly as tier-1 recorded it — the coverage proof visits and proves every games dir anyway (mahjongg/mp3dec/mp3enc/music/sokoban/sudoku build=ok)"
  - "The aarch64 'x86-hardware-only' exclusion class is RE-DERIVED as amd64 MEMBERS: aux/vga, aux/realemu, aux/na, aux/gps, aux/mnihongo, aux/flashfs all compile natively on amd64 (measured by the proof) and join tier4"
  - "fossil + venti are compile-gated with the runtime-unverifiable note committed in THREE places (tier4 entries, trailing # comments on their dir-coverage rows via the sweep's report writer, and the SUMMARY) — no block device in r9, virtio-blk is a future milestone; never a silent gap"
  - "The rc argv-stack residual (the Phase-6 checklist item): NOT proven live on amd64 — zero 'pushword' occurrences across the full-sweep driver log (1353 mk invocations through the venue rc) and the in-guest sessions; the audit item stays open on the Phase-10-class checklist, no amd64-specific fix landed (the honest dead-disposition the plan's flagged assumption asked for)"
  - "The gs smoke's FILE device is ppmraw, not the plan's example 'bit': the committed fake-make products link gdevpbm.o, not gdevbit.o (measured in ld.tr) — ppmraw is in the plan's own named FILE-device class (bit/bitrgb/ppmraw), raw PPM bytes to -sOutputFile, no display, no draw use"
  - "The smoke's payload is name-keyed (bootfs-gs.paq) and fed to the venue through a scratch copy at the channel's fixed bootfs-draw.paq name (qemu registers -initrd by basename; k10draw.rc's channel contract names bootfs-draw.paq)"
patterns-established:
  - "The coverage-proof driver shape: universe minus tier4-exclusions drives per-dir clean+all under per-invocation timeout; the mk shim logs VISIT/EXIT for nested recursion; the report is DATA ROWS ONLY with annotations as trailing # comments; rig failures never destroy the classified inventory"
  - "Exclusion records are data (tier4 named comments) consumed by tooling (the sweep's universe-build list + stale-exclusion converse assert) — never prose-only"
requirements-completed: [USPACE-09]
actuals:
  tokens: 18200   # chars/4 over the authored diff (72686 chars excl. the generated dir-coverage.txt + evidence logs; all-diff 109624 chars/17 files, 2051 insertions — estimate was 76000, the same over-projection family as every Phase-15 plan; the D-10 landmine budget was again mostly unspent)
  tasks: 3
  commits: 7      # measured: git rev-list --count 1d74692b2..HEAD (ledger reconstructed from the first plan commit's parent — the interrupted executor never wrote it)
plan_head_before: 1d74692b263e06a5b37d603f242433ad739d1b0f
plan_head_after: 149bbb7af8deb4fd9bf6ad7b6442f55677f1da11
status: complete
coverage:
  - id: D1
    description: "Task 1 (tracer) — the amd64 coverage sweep re-keyed from the aarch64 driver (COVERAGE_SWEEP=1 passthrough, VISIT/EXIT markers, timeout watchdog, digests, the venue shapes the first runs measured) producing the first full classified inventory: 709 rows, 8 red = 1 in-tree class (4c) + 7 re-derived upstream-disabled exclusions"
    requirement: USPACE-09
    verification:
      - kind: command
        ref: "baseline re-run at continuation (nix develop -c sh build/x86_64/coverage-sweep.sh): exit 0, COVERAGE-SWEEP-OK, visit=1353 exit=1353 rows=702, VISIT>=rows and EXIT>=1; grep COVERAGE_SWEEP present; git diff --quiet HEAD -- build/aarch64/{coverage-sweep.sh,dir-coverage.txt} (frozen)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Task 2 — the proof ends GREEN with the class-fix commits + tier4 data + the $T4 wiring at all six sites; fossil/venti compile-gated with the documented runtime-unverifiable note; the 15-01 scoping proof widened to the four-file union and re-run green"
    requirement: USPACE-09
    verification:
      - kind: command
        ref: "final sweep: exit 0, not-ok=0, ok-rows=702 >= 130 sys/src/cmd dirs; tier4 carries cmd/gs + fossil + venti + 2 runtime-unverifiable note hits; grep tier4-tail >= 1 at all six sites (tier4-wiring-ok); scoping proof: SCOPING-PROOF-OK 10/10 incl. the new MK:aux-DIRS row"
        status: pass
    human_judgment: false
  - id: D3
    description: "Task 3 — the gs render smoke: the committed font-free fixture rendered IN GUEST to a non-empty bitmap FILE on the 15-05 donor venue, zero panic/trap, capture retained in the phase evidence dir"
    requirement: USPACE-09
    verification:
      - kind: command
        ref: "nix develop -c sh build/mm/boot-9kgs-smoke.sh: exit 0, GS-GATE-PASS — GS-RENDER-OK + GS-BITS 43269 bytes on the serial capture (evidence/15-07-gs-smoke.log: banner, four venue markers, GS-BIN-PRESENT, GS-SMOKE-DONE); fixture grep -ci font = 0; harness greps sOutputFile; fresh name-keyed bootfs-gs.paq (-newer fixture = 1)"
        status: pass
    human_judgment: false
---

# Phase 15 Plan 7: Userspace Tiers + Bootfs Payload (amd64) — Coverage Proof + Long Tail Summary

**The machine-verified full DIRS-coverage proof green for amd64 (702 rows, zero not-ok, every dir visited; three in-tree class fixes landed; 7 upstream-disabled failures re-derived as named exclusions in tier4-tail.txt wired at all six filter sites) — and gs, the 753-file LP64-fossil pcc/APE habitat, both compiled through the stock route and PROVEN EXECUTING headless on the donor venue: the font-free fixture rendered to a 43269-byte PPM bitmap with the FILE device, fossil+venti compile-gated behind the documented no-block-device-in-r9 ceiling.**

## Performance

- **Duration:** continuation ~50 min active + the interrupted executor's unrecorded start (wall ≤ 3h 44m)
- **Started:** original executor ≤ 2026-11-10T14:20:36Z (after 15-06's close; true start not recorded — first commit 16:10:26Z)
- **Interrupted:** ~2026-11-10T17:09:53Z (last WIP mtime), mid-Task-2
- **Continuation first commit:** 2026-11-10T17:28:41Z
- **Completed:** 2026-11-10T18:03Z
- **Tasks:** 3/3 (Task 1 tracer, executed by the interrupted executor)
- **Files:** 17 (6 created; 2051 insertions)

## Accomplishments

- **The proof (D-09):** `build/x86_64/dir-coverage.txt` asserts every sys/src/cmd + sys/src/games dir, every lib, every cmd-root single and every mkmany container member compiles for amd64 under COVERAGE_SWEEP=1 — 702 rows, zero without build=ok, 1353 VISIT / 1353 EXIT (a killed run leaves a classifiable partial that can never pass as proof; the full-set universe assertion is the gate's teeth).
- **The fix discipline (D-10):** every failure the first run surfaced was root-caused — one commit per class: 4c's rfork/bind `$LIB` hack replaced by the portable lex-object override (plus the repair of that fix's own clean-override mistake), 5c's committed enam.c regenerated from 5.out.h (39 missing opcode names), libdynld's dead ULONG<0 entry check dropped; the remaining 7 failures are re-derived upstream-disabled/dead-tool exclusions, each a named tier4 comment with its measured digest (independently re-verified at continuation), never a skip.
- **The long tail joined the machinery:** tier4-tail.txt (112 cmd dirs + 6 aux subdirs — the 'x86-hardware-only' aarch64 class re-derived as native members) wired into all six filter sites as the four-file union; the widened 15-01 scoping proof re-run green (SCOPING-PROOF-OK, 10/10).
- **The honest ceiling:** fossil + venti compile green and carry the runtime-unverifiable note in three committed places — compile is their gate this milestone (no block device in r9).
- **gs beyond compile (D-11):** the self-contained smoke bakes its own name-keyed bootfs-gs.paq (gs + ramfs + wc + the ghostscript init library + the measured-minimal config tables), boots the 15-05 donor kernel with it through the -initrd channel, and proves gs renders headless: GS-BIN-PRESENT, GS-BITS 43269, GS-RENDER-OK, zero panic/trap — with zero font payloads staged anywhere.

## The class table (D-10: class → commit → dirs)

| # | Class | Commit | Dirs |
|---|-------|--------|------|
| 1 | 4c's rfork/bind `$LIB` hack unbuildable on Linux namespace venues (plan9port rc: no rfork/bind) → the portable lex-object override (lex.$O explicit, ahead of ../cc/cc.a$O) | `09ea2f525` | sys/src/cmd/4c |
| 1b | the same fix's clean override was ITSELF a bug — a second recipe for mkone's clean:V: = mk 'ambiguous recipes' (parse-time); reverted to upstream's prerequisite form (mkone's rm recipe runs anyway — dry-run proven both ways) | `a7c43035` | sys/src/cmd/4c |
| 2 | 5c's committed enam.c stale vs 5.out.h (mkenam regenerated it +39 opcode names: the ARM barrier/clz/exclusive family) | `c8d635e59` | sys/src/cmd/5c |
| 3 | libdynld's dead `entry < 0` half-guard (ulong; the Phase-6-filed checklist item, re-derived live: 6c emits the identical diagnostic) | `379677dc6` | sys/src/libdynld |
| 4 | upstream-disabled / dead-tool exclusions, re-derived with measured digests (NOT in-tree-fixable classes): postscript picpack/postbgi/postdaisy/postdmd/postmd (commented out of postscript/mkfile's TARGETS under a literal `# UNIX remnants` block), tcs/font (no `all` target; links -lg which does not exist; tcs never recurses into font), games/blabs (commented out of games/mkfile's DIRS; mkface drives the nonexistent sys/src/cmd/fb) | `3a86653fd` (the tier4 record) | the 7 dirs — never visited by the sweep, never rows |

Checklist re-derivations: libdynld's ULONG<0 **fixed live** (class 3); the rc argv-stack residual **not proven live** — zero `pushword` occurrences in the full-run driver log across 1353 mk invocations — recorded as the dead-on-amd64-sweep disposition (the audit item stays open on the Phase-10-class checklist).

## Task Commits

Executed across two executors (the first four by the interrupted executor, verified by the continuation):

1. **Task 1: the coverage sweep + the first classified red map (tracer)** — `36e985996` (feat)
2. **Task 2 (fix class 1): 4c's rfork/bind $LIB hack** — `09ea2f525` (fix)
3. **Task 2 (fix class 2): 5c's stale enam.c** — `c8d635e59` (fix)
4. **Task 2 (fix class 3): libdynld's dead ULONG<0 check** — `379677dc6` (fix)
5. **Task 2 (fix class 1b): 4c clean-override ambiguous-recipes repair** — `a7c430352` (fix)
6. **Task 2: tier4 data + the six-site $T4 wiring + the GREEN proof + the widened scoping proof** — `3a86653fd` (feat)
7. **Task 3: the gs render smoke (fixture + harness + capture)** — `149bbb7af` (feat)

**Plan metadata:** this SUMMARY commit (docs)

## Files Created/Modified

- `build/x86_64/coverage-sweep.sh` — the amd64 full-tree driver (created Task 1; Task 2 added the tier4-exclusion mechanism + the fossil/venti row annotation)
- `build/x86_64/dir-coverage.txt` — the machine-verified proof (709-row red map at Task 1 → 702-row green at Task 2)
- `build/x86_64/tiers/tier4-tail.txt` — the long-tail membership + the re-derived exclusion record
- `sys/src/{mkfile,cmd/mkfile,cmd/ip/mkfile,cmd/auth/mkfile,cmd/auth/factotum/mkfile,cmd/aux/mkfile}` — the four-file union awk at all six sites
- `sys/src/cmd/4c/mkfile` — the portable lex-object override + the upstream-form clean
- `sys/src/cmd/5c/enam.c` — the regenerated table
- `sys/src/libdynld/dynld.c` — the dead-check drop
- `build/x86_64/gs-render.ps` — the committed font-free fixture
- `build/mm/boot-9kgs-smoke.sh` — the self-contained render-smoke harness
- `evidence/15-07-gs-smoke.log` — the retained capture; `evidence/15-01-scoping-proof.{sh,log}` — the widened proof + its green log

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones: the 7-exclusion re-derivation (independently re-verified against the tree at continuation), the x86-hardware-only class flipped to members, the three-place fossil/venti ceiling, the ppmraw-vs-bit device re-derivation (measured against ld.tr), the rc argv residual's dead disposition, and the name-keyed paq discipline for the smoke.

## Deviations from Plan

**The interruption + WIP salvage (the dispatch's first requirement):** the previous executor terminated abnormally mid-Task-2, leaving 11 modified files + 3 untracked drafts. Salvage decisions, each verified before building on it: the six-site $T4 wiring and tier4-tail.txt were kept (the widened scoping proof re-run green validated them); the sweep's exclusion mechanism was kept (it re-ran green); the 4c clean-override revert was kept and ROOT-CAUSED (the ambiguous-recipes proof below); the scoping-proof log's missing PASS lines were determined to be the script's design (PASS lines go to console, not the log), not a truncated run; the Task-3 drafts were treated as unproven (never run — correct: they carried four real bugs). The tier build sanity was confirmed before building (the dispatch's parse-error warning): the 12:04 sweep and the 11:59 scoping proof had both parsed all mkfiles green.

### Auto-fixed Issues

**1. [Rule 1 - Bug, salvage] 4c's committed clean override was itself a bug — mk ambiguous recipes**
- **Found during:** continuation Task 2 salvage (the 12:04 sweep's shim markers: mk clean EXIT 1, mk all EXIT 0 — the only red row)
- **Issue:** the fix commit's `clean:V: cleanlib` + rm added a SECOND recipe for a target mkone already defines (`clean:V:` with its own rm) — plan9port mk aborts 'ambiguous recipes for clean' at parse time; and its premise was wrong (upstream's `clean: cleanlib` adds a prerequisite, so mkone's rm recipe runs anyway).
- **Fix:** reverted to the exact upstream form; namespace dry-run proof at both forms (upstream exit=0 with both rm recipes shown; the override exit=1 with mk's ambiguous-recipes message).
- **Files modified:** sys/src/cmd/4c/mkfile
- **Verification:** the probe script's both-ways dry run; the final sweep's 4c row build=ok.
- **Committed in:** a7c430352

**2. [Rule 3 - Blocking, salvage] the gs harness computed a nonexistent hostbin path**
- **Found during:** continuation Task 3, first harness run
- **Issue:** `hostbin-$(uname -m)` vs the real `hostbin-linux-<arch>` — the exact 15-05 deviation-2 class.
- **Fix:** build-9kdraw.sh's computed discipline verbatim.
- **Files modified:** build/mm/boot-9kgs-smoke.sh
- **Committed in:** 149bbb7af

**3. [Rule 2 - Missing critical, salvage] wc was never staged — the GS-BITS size line could never fire**
- **Found during:** continuation Task 3 review (before the first boot)
- **Issue:** the venue kernel's embedded root carries echo/cat/test/sleep/read/rc/bind/paqfs only; the draft bake staged gs + ramfs but not wc, so `wc -c < /tmp/render.bit` had no binary.
- **Fix:** wc staged with a magic check (the plan's "minimal utility set").
- **Committed in:** 149bbb7af

**4. [Rule 1 - Bug] Harvey's rc lexer kills the gs invocation — bare '=' ends a word**
- **Found during:** Task 3, first boot (`rc: /lib/trace-session.rc:12: token '=sDEVICE': syntax error`)
- **Issue:** lex.c's wordchr excludes '=', so `-sDEVICE=ppmraw` lexes WORD = WORD and the sourced session dies whole; plan9port's rc accepts the bare form, so host-side checks never see the class.
- **Fix:** the two =-bearing switches single-quoted.
- **Committed in:** 149bbb7af

**5. [Rule 3 - Diagnosability] rc's Execute masked the real /bin/gs failure**
- **Found during:** Task 3, second boot (`gs: './gs' file does not exist`)
- **Issue:** Execute overwrites errstr per path candidate and prints the last — the trailing-dot candidate's message hid what /bin/gs actually said.
- **Fix:** absolute-path /bin/gs invocation (single attempt, true errstr) + GS-BIN-PRESENT/-MISSING markers on the capture; rc's plan9.c noted in the phase deferred items.
- **Committed in:** 149bbb7af

**6. [Rule 1 - Bug] mkpaqfs silently dropped the 15.5MB gs from the paq**
- **Found during:** Task 3, third boot (GS-BIN-MISSING despite a green bake)
- **Issue:** mkpaqfs caps each file at blocksize^2/4 (4 MiB at the 4096 default) and only WARNS before dropping the file — exit 0, image silently missing gs.
- **Fix:** -b 16384 (64 MiB cap; the format allows 512K and paqfs reads the blocksize from the header), an arithmetic fit assert, and any mkpaqfs diagnostic fails the bake; the general hazard for every other bake script logged in the phase deferred items.
- **Committed in:** 149bbb7af

**7. [Rule 2 - Missing critical] gs's init chain reads two non-.ps config tables unconditionally**
- **Found during:** Task 3, fourth boot (`Unrecoverable error: undefined in .uninstallpagedevice`, gs_fntem.ps on the operand stack, file position 1008 = line 27)
- **Issue:** `gs_fntem.ps:27 (xlatmap) .runlibfile` and `gs_fapi.ps:59 (FAPIconfig) .runlibfile` — both registered as init files in gconfig.h; the *.ps-only staging omitted them.
- **Fix:** the measured-minimal set staged (xlatmap 1750B + FAPIconfig 778B — plain-text name/path tables, never typeface payloads; a file joins the set only when a boot proves the init opens it).
- **Committed in:** 149bbb7af

**8. [Rule 3 - Robustness] an rc parse error burned the full 600s marker budget**
- **Found during:** Task 3, first boot (the harness waited the whole budget behind a dead session)
- **Fix:** a fast-fail arm on 'syntax error' in the capture (classify now instead of waiting out a session that can never produce a marker).
- **Committed in:** 149bbb7af

---

**Total deviations:** 8 auto-fixed (3 salvage-completions of the interrupted executor's work, 5 bring-up bugs in the never-run Task-3 draft) + the interruption/WIP reconciliation above. **Impact on plan:** all inside the plan's own disciplines (D-10 root-cause, measured-minimal staging, the licensing boundary); the terminal truths (green full-set proof, class-fix commits, the D-11 render) are met exactly as planned — the smoke's five bring-up fixes are the never-run-draft risk materialized, each closed with its measured evidence in-code.

## Authentication Gates

None.

## Known Stubs

None — no placeholder paths, no unwired data, no skipped verifications. fossil/venti's runtime-unverifiable status is the documented ceiling (committed note), not a stub; FONT-CHANNEL-FAIL on the gs venue capture is D-11's design (no font tree in the gs paq), not a gap.

## Issues Encountered

- The interrupted executor's mid-Task-2 death left the tier4 wiring half-committed (data + mkfiles unwritten to git, sweep green-run evidence only in the working tree); salvage verified each piece before building on it (see Deviations preamble). The continuation held the atomic-close-out invariant: no SUMMARY existed, so the plan resumed cleanly at Task 2.
- The 12:04 sweep's one red row was a false-adjacent signal: mk all had SUCCEEDED (exit 0) and only the shim-observed mk clean exit 1 poisoned the row — the repair (#1) closed it at the mkfile, not in the parser (worst-status-wins stays correct for real failures).

## User Setup Required

None.

## Next Plan Readiness

READY for 15-08 (the evidence bundle): the green dir-coverage.txt, the tier4 record, the widened scoping proof, and the gs smoke capture are all committed. The deferred-items file gained two entries (mkpaqfs silent-drop; rc Execute errstr masking) — out of scope here, recorded for their owning plans. fossil/venti runtime waits on virtio-blk (a future milestone's kernel work).

## Self-Check: PASSED

- files exist: all 17 created/modified files checked with [ -f ] — FOUND ✓
- commits are ancestors of HEAD: 36e985996, 09ea2f525, c8d635e59, 379677dc6, a7c430352, 3a86653fd, 149bbb7af ✓
- measured: commits=7 (git rev-list --count 1d74692b2..HEAD), plan_head_after=149bbb7af ✓
- plan-level verification green at HEAD (re-run): coverage sweep exit 0 / COVERAGE-SWEEP-OK, not-ok=0, ok-rows=702 >= 130 dirs, tier4 wiring at all six sites, SCOPING-PROOF-OK, GS-GATE-PASS (GS-BITS 43269), fixture font-free (grep -ci font = 0), aarch64 originals diff-quiet ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-10*

