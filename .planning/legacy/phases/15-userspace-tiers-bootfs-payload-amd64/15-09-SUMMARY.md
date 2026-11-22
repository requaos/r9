---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 09
subsystem: libmemdraw
tags: [libmemdraw, drawtest, arctest, pixel-path, lp64, sign-extension, compositing-arithmetic, mkfile-cleanup, uspace-08]
duration: 11 min
completed: 2026-11-10
requires:
  - "15-04-complete tier machinery (libmemdraw + libmemlayer as tier-0 members since 15-04 — the rebuild gate's input; the k10 kernel venue links these same sources)"
provides:
  - "the inherited libmemdraw pixel-path audit closed at the source level: drawtest's stochastic pixel-verification abort and arctest's wild-pointer write root-caused as two arch-independent C bugs and fixed in-tree as two distinct class commits"
  - "sys/src/libmemdraw/draw.c — the byte-path compositing calculators (alphacalc3679/alphacalc11/alphacalcS/boolcalc236789) round the combined numerator ONCE (MUL2, bias +128, exactly (s+127)/255) instead of summing two per-product roundings"
  - "sys/src/libmemdraw/drawtest.c — the reference drawonepixel rounds to nearest (+127), matching the library's integer policy on every input"
  - "sys/src/libmemdraw/addr.c — byteaddr's row offset wraps in int, sign-extending into 64-bit pointers (the +2^32 wild-address fossil)"
  - "the stale mkfile test-target checklist closed (liboventi vttest/test:, libmemdraw $O.test + same-class $O.drawtime) with the real $O.drawtest/$O.arctest targets kept verbatim"
  - "the 15-06 runtime re-join contract on record (DRAWTEST-OK / ARCTEST-OK, below)"
affects:
  - "15-06 (the trace rig: Task 3 stages+runs the fixed drawtest/arctest on the draw venue — DRAWTEST-OK/ARCTEST-OK is the audit's runtime proof stage, and the traces capture over fixed pixel paths)",
  - "every consumer of libmemdraw's alpha compositing (rio/acme/sam text and fill rendering: sub-1-LSB channel deltas change on ~half of blended pixels — the correct round-to-nearest value)"
tech-stack:
  added: []
  patterns:
    - "round-to-nearest compositing is a two-sided contract: the library's integer macro and the test's reference must implement the SAME rounding ((s+127)/255), proven equal by exhaustive coefficient-pair search before committing"
    - "pointer+offset arithmetic with unsigned 32-bit products is a 64-bit-pointer fossil site: any product that can go negative must be forced through int at the pointer boundary (the plan9port (int) shape)"
key-files:
  created: []
  modified:
    - sys/src/libmemdraw/draw.c
    - sys/src/libmemdraw/drawtest.c
    - sys/src/libmemdraw/addr.c
    - sys/src/libmemdraw/mkfile
    - sys/src/liboventi/mkfile
key-decisions:
  - "drawtest's abort is NOT memory corruption: the verification DETECTED a real arithmetic disagreement — the test's reference truncated ((s*m+d*M)/255) while the library rounded per product, and the byte-path calculators rounded TWICE (MUL+MUL), which disagrees with round-to-nearest of the sum on ~49% of random pixels (measured host-side: 97575/200000 divergent). Both sides fixed to the one 9front/plan9port-converged arithmetic; verified equal on every reachable coefficient pair"
  - "arctest's wild write is a zero-extension fossil in byteaddr, test-side was CORRECT: sizeof(ulong)*p.y*i->width is an unsigned 32-bit product, so memarc's negative-origin GREY1 temporaries (Rect(-a-w,-b-w,...) filled via memfillcolor->wordaddr->byteaddr) produce a wrapped unsigned that zero-extends into the 64-bit pointer — the destination lands exactly +2^32 out. The measured fault address proves it: 0x1c001374bc = 0x1b001374bc + 4GiB. Fix is the plan9port shape: the row offset wraps in int"
  - "RGB2K needed NO change: the test's /1000 and the library's >>19 forms are provably identical for all r,g,b < 256 (1000A - 524288B = 888r+944g+168b is in [0, 524288), never crossing a /1000 step — verified numerically on 2M random triples, 0 divergences)"
  - "the rgba word-lane fast path (MUL0123+MUL0123, the delta==4 arms of alphacalc3679/alphacalc11) still double-rounds — same family, but NO reproducer exists in the default r8g8b8 test and redesigning the 16-bit-lane macros without an executing reproducer is unverifiable refactor risk; recorded for a future class (plan9port's CALC42 is the reference shape)"
  - "libmemdraw's $O.drawtime (nonexistent drawtime.c) removed with the plan-named $O.test row — same stale class, same file; the plan's acceptance greps stay meaningful (vttest 3->0, $O.test 1->0) and the prohibition held: $O.drawtest/$O.arctest verbatim"
patterns-established:
  - "The fault address is evidence: a wild pointer whose displacement from a plausible base is exactly 2^32 is a 32-bit unsigned wrap zero-extended at a 64-bit pointer boundary — grep for unsigned products feeding pointer arithmetic, not for buffer overruns"
  - "Before 'fixing' a stochastic test, model BOTH sides numerically (host python over the full input range): it distinguishes test-side arithmetic, library-side arithmetic, and provably-equivalent red herrings (RGB2K) in minutes"
requirements-completed: [USPACE-08]
actuals:
  tokens: 2175    # chars/4 over the realized sys/src diff (8701 chars, 5 files, 51 insertions — estimate was 42000: the same large over-projection direction as 15-01/15-04/15-05; the root-cause budget was spent in analysis, not diff)
  tasks: 3
  commits: 3      # measured: git rev-list --count 018731130..HEAD
status: complete
coverage:
  - id: D1
    description: "Task 1 (tracer) — drawtest's stochastic pixel-verification abort root-caused (compositing-arithmetic mismatch, both sides) and fixed in-tree as class commit 506ca9605, tier-0 rebuild green end-to-end"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "git log --grep='fix.15-09' -- sys/src/libmemdraw/: 2 class commits; git diff --quiet HEAD -- sys/src/libmemdraw/ clean; nix develop -c sh build/x86_64/subset-build.sh: exit 0 + TIER-BUILD-OK (24 T0-ART / 40 T1-ART / 91 T2-ART / 12 T3-ART) re-run green POST-commit (tracer gate)"
        status: pass
      - kind: command
        ref: "host-side identity proof (python, exhaustive 256x256 single-term + constrained two-term + 2M random composites): fixed test == fixed library on every input; current formulas diverged on 97575/200000 random pixels (the abort's mechanism); RGB2K /1000 vs >>19 equal on 2M triples"
        status: pass
      - kind: command
        ref: "namespace one-off: mk 6.drawtest links green (sha256 f84ad041...) with the edited drawtest.c against the fresh archive"
        status: pass
  - id: D2
    description: "Task 2 — arctest's wild-pointer write root-caused (byteaddr zero-extension, pixel-path source side) and fixed as its own distinct class commit e8df41c7d; two fix(15-09) commits total, tree clean, rebuild green"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "class-commits=2 (distinct); post-commit full tier rebuild exit 0 + TIER-BUILD-OK; arctest.c present with $O.arctest target intact; mk 6.arctest links green in the namespace (sha256 073cc360...)"
        status: pass
      - kind: command
        ref: "the fault-address arithmetic: 0x1c001374bc - 2^32 = 0x1b001374bc exactly — the +4GiB displacement signature of a zero-extended 32-bit wrap, derived from the donor measurement before the fix was designed"
        status: pass
  - id: D3
    description: "Task 3 — stale mkfile test targets cleaned (vttest 3->0, $O.test 1->0, same-class $O.drawtime 0) with the real targets kept, and the 15-06 re-join contract recorded"
    requirement: USPACE-08
    verification:
      - kind: command
        ref: "grep counts vttest=0 stale-test=0 on the committed mkfiles; $O.drawtest/$O.arctest present; liboventi builds green from the cleaned mkfile (namespace one-off, OVENTI-ART asserted); 6.drawtest/6.arctest link byte-identically post-cleanup; final tier rebuild exit 0 + TIER-BUILD-OK"
        status: pass
---

# Phase 15 Plan 9: Userspace Tiers + Bootfs Payload (amd64) — libmemdraw Pixel-Path Audit Summary

**The inherited drawtest/arctest faults closed as two root-caused class commits — a two-sided compositing-arithmetic mismatch (truncating reference vs double-rounding library, both now round-to-nearest) and a byteaddr zero-extension fossil that put memsetl's destination exactly +2^32 bytes out — plus the stale mkfile test-target cleanup, with the 15-06 runtime re-join contract (DRAWTEST-OK / ARCTEST-OK) on record.**

## The 15-06 re-join contract (the record 15-06 Task 3 consumes)

15-06 stages the fixed `drawtest`/`arctest` binaries into the draw-venue payload
(tier-0 lib members; the mkfile rules `$O.drawtest`/`$O.arctest` are kept and
link green) and runs them on the k10draw venue via
`build/mm/boot-9kdraw-trace.sh`; the audit's runtime proof stage is:

- **`DRAWTEST-OK`** — drawtest completes its full stochastic verification
  (single-pixel, line, rect, repl variants) without aborting. The re-join run
  uses the test's DEFAULT channels (`r8g8b8 r8g8b8 r8g8b8`, the 06-05 donor
  invocation shape); the fix is proven bit-exact for every coefficient pair,
  so any channel arguments the rig passes are equally covered on the byte path.
  A failure is a 15-09 fix regression — root-cause again, never a skip.
- **`ARCTEST-OK`** — arctest completes its timed memarc loop without faulting
  (the wild-pointer write is gone at the source: byteaddr sign-extends row
  offsets; arctest.c itself was correct and needed no change).

Root-cause notes (where each fault lived):

| Fault | Lived in | Root cause | Fix commit |
|---|---|---|---|
| drawtest stochastic abort | BOTH the test's verification arithmetic AND the pixel-path sources it exercises (`drawtest.c` + `draw.c`) | reference truncated / library double-rounded (MUL+MUL); ~49% of random pixels disagreed by 1 LSB | `506ca9605` |
| arctest wild-pointer write (pc in memsetl) | the pixel-path source (`addr.c` byteaddr); `arctest.c` was correct | unsigned 32-bit row-offset product for negative rows zero-extends into 64-bit pointers: destination = base + 2^32 exactly (fault addr 0x1c001374bc = 0x1b001374bc + 4GiB) | `e8df41c7d` |

Scope note for 15-06: the rgba word-lane fast path (`MUL0123+MUL0123`, channels
with 4-byte pixels, e.g. r8g8b8a8) still double-rounds — same family, no
default-channel reproducer, left for a future class (see Deviations #3). The
default r8g8b8 venue run does not reach it.

## Performance

- **Duration:** 11 min (analysis-heavy: the diff is 5 files / 51 insertions)
- **Started:** 2026-11-10T05:51:25Z
- **Completed:** 2026-11-10T06:02:18Z
- **Tasks:** 3/3 (Task 1 tracer)
- **Files modified:** 5

## Accomplishments

- **drawtest (Task 1, tracer):** rode one fault through root-cause → fix →
  class commit → tier-0 rebuild green end-to-end, then the tracer feedback
  gate re-ran the full verify post-commit (byte-stable marker set: 24/40/91/12
  T-ART counts, fresh libmemdraw archive sha). The root cause is an arithmetic
  CONTRACT violation between the test and the library, not corruption — the
  abort was the test working exactly as designed against a library whose
  byte-path calculators rounded twice.
- **arctest (Task 2):** the wild-pointer write pinned to byteaddr's unsigned
  row-offset product — the +2^32 displacement visible in the donor fault
  address is the fossil's signature (32-bit wrap zero-extended at a 64-bit
  pointer). Fixed with the plan9port `(int)` shape; arctest.c untouched. The
  9k kernel links these same libmemdraw sources, so the draw venue's kernel
  side is covered by the same fix.
- **stale targets (Task 3):** liboventi's vt-suite rows and libmemdraw's
  nonexistent-source rules removed (drawtime included, same class), the real
  test targets re-proven by linking byte-identical binaries post-cleanup, and
  the Phase-6 deferred-items entries for both checklist items flipped to
  resolved.

## Task Commits

1. **Task 1: drawtest root-caused + fixed + class commit + tier-0 green** — `506ca9605` (fix)
2. **Task 2: arctest wild-pointer fix, its own class** — `e8df41c7d` (fix)
3. **Task 3: stale test targets cleaned** — `9ecac570b` (chore)

**Plan metadata:** this SUMMARY commit (docs)

## Files Created/Modified

- `sys/src/libmemdraw/draw.c` — MUL2 corrected (+128 bias, rathole arg) and adopted at every byte-path two-rounding site; single-term MUL sites untouched (already exact)
- `sys/src/libmemdraw/drawtest.c` — drawonepixel rounds to nearest (+127), M included
- `sys/src/libmemdraw/addr.c` — byteaddr row offset wraps in int (sign-extends into 64-bit pointers)
- `sys/src/libmemdraw/mkfile` — stale `$O.test` + same-class `$O.drawtime` rules removed; `$O.drawtest`/`$O.arctest` verbatim
- `sys/src/liboventi/mkfile` — stale `$O.vttest` rule + `test:` target removed

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones: the two-sided
arithmetic contract (with the exhaustive numeric proof that the fixed pair
agrees on every input), the fault-address-as-evidence method that pinned the
zero-extension fossil, and the deliberate scope boundary on the rgba lane path
(no reproducer, no untestable refactor).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking verify] The plan's bare `sh subset-build.sh` verify cannot run in the ambient shell**
- **Found during:** Task 1 verify
- **Issue:** The ambient executor shell lacks the nix devshell (plan9port mk, hostbin toolchain, qemu deps) — the same 15-04 deviation #4 / 15-05 deviation #7 class.
- **Fix:** Every build verify ran as `nix develop -c sh build/x86_64/subset-build.sh` (the driver's own documented invocation); the greps ran against the same /tmp logs the plan names.
- **Files modified:** none (invocation only)
- **Verification:** TIER-BUILD-OK in /tmp/t0re{1,2,3}.log, one per task gate + the tracer gate's post-commit re-run.
- **Committed in:** n/a (environment shape, documented in the driver header since 15-04)

**2. [Rule 2 - Class completeness] libmemdraw's `$O.drawtime` removed alongside the plan-named `$O.test` row**
- **Found during:** Task 3 (mkfile edit)
- **Issue:** The plan names only `$O.test` (line 44) and liboventi's vttest rows, but `$O.drawtime` (line 35) is the identical stale class — a rule over a `drawtime.c` that does not exist in the tree.
- **Fix:** Removed in the same cleanup commit; the plan's acceptance greps (vttest=0, `$O.test`=0) hold, the real targets are verbatim, and the prohibition (never remove drawtest/arctest) is untouched.
- **Files modified:** sys/src/libmemdraw/mkfile
- **Verification:** `grep -c '\$O\.drawtime'` = 0; `$O.drawtest`/`$O.arctest` present; rebuild green.
- **Committed in:** 9ecac570b

**3. [Scope boundary - recorded, not fixed] the rgba word-lane fast path still double-rounds**
- **Found during:** Task 1 (class sweep of the two-rounding sites)
- **Issue:** alphacalc3679/alphacalc11's `q` arms (`MUL0123(fs,...)+MUL0123(fd,...)`, taken when src AND dst have 4-byte pixels, e.g. r8g8b8a8) sum two lane roundings — the same imprecision family the audit found on the byte path. No reproducer exists: the default r8g8b8 test never takes these arms, and redesigning the 16-bit-lane macros (plan9port needed new CALC22/CALC42 machinery for it) without an executing reproducer would be unverifiable.
- **Fix:** recorded here + in the class commit message; a future class commit owns it if a 4-byte-channel consumer appears (plan9port's CALC42 is the reference shape).
- **Files modified:** none (deliberate)
- **Verification:** the default-channel test path is fully covered by the committed fix (proven bit-exact); the lane path is documented out of the measured class.
- **Committed in:** noted in 506ca9605

---

**Total deviations:** 2 auto-fixed (1 verify-environment, 1 class-completeness) + 1 recorded scope boundary. **Impact on plan:** all inside the plan's own disciplines (the documented devshell venue, the stale-target class, the root-cause-only rule — the lane-path restraint IS that rule); no scope creep.

## Authentication Gates

None.

## Known Stubs

None — no guards, no skips, no weakened assertions: the test's comparison is
byte-exact against the library by proof, and the wild-pointer fix removes the
fault rather than containing it. The runtime re-join (DRAWTEST-OK/ARCTEST-OK)
is not a stub either: it is the plan's explicit contract allocation — the
source-level fixes are this plan's deliverable, the venue proof is 15-06
Task 3's stage (wave 4).

## Issues Encountered

None beyond the deviations — the fault address arithmetic (0x1c001374bc =
0x1b001374bc + 2^32) was derivable from the preserved Phase-6 evidence, and
the plan9port diff served as the upstream-fix map for both classes (its
drawtest +127 and its addr.c `(int)` cast are exactly the shapes landed here;
its u32int re-keying was NOT needed — Harvey's Plan 9 compilers keep
long/ulong 32-bit even on 64-bit targets, 6c/7c gc.h `SZ_LONG 4`).

## User Setup Required

None.

## Next Plan Readiness

READY for 15-06 (the trace rig): the pixel paths are fixed at the source, the
test binaries link green against the fresh tier-0 archive, the mkfile targets
the rig needs are intact, and the re-join contract above names the markers
(DRAWTEST-OK / ARCTEST-OK), the invocation shape, and the regression rule
(root-cause, never skip). NOTE for 15-08: the evidence bundle can cite the two
class commits and this SUMMARY's fault-address arithmetic as the audit's
provenance.

## Self-Check: PASSED

- files exist: draw.c, drawtest.c, addr.c (modified), libmemdraw/mkfile, liboventi/mkfile ✓
- commits are ancestors of HEAD: 506ca9605, e8df41c7d, 9ecac570b ✓
- plan-level verification green at HEAD: final full tier rebuild exit 0, TIER-BUILD-OK, 24 T0-ART / 12 T3-ART; sys/src tree clean; vttest=0 / $O.test=0; real targets present ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-10*
