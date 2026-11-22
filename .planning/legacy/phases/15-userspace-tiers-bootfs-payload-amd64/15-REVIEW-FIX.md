---
phase: 15-userspace-tiers-bootfs-payload-amd64
fixed_at: 2026-11-10T20:57:56Z
review_path: .planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 15: Code Review Fix Report

**Fixed at:** 2026-11-10T20:57:56Z
**Source review:** `.planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (CR-01, WR-01..WR-07; the seven IN-* findings stay open per the fix scope — triage-level, recorded in 15-REVIEW-DISPOSITION.md)
- Fixed: 8
- Skipped: 0

**Verification discipline** (this project's law — fixed live, not just compiled):

- Gates ran in the MAIN CHECKOUT `/home/req/code/harvey` (`workflow.use_worktrees=false`; no worktree was created), so every number below is reproducible from this tree.
- `nix develop -c sh build/mm/build-9kdraw.sh` — kernel REBUILD (CR-01, WR-01, WR-02 in the linked kernel) + payload bake + draw venue boot: **rc=0**, all four bring-up markers + DRAWTEST-OK + ARCTEST-OK, zero panic/trap (run log + capture retained: `evidence/15-review-fix-draw-venue.log`).
- `nix develop -c sh build/mm/boot-9kdraw-trace.sh` at HEAD — **rc=0, TRACE-GATE-PASS** (sessions rio/acme/sam/rejoin, egress green, four markers, zero panic/trap; capture retained: `evidence/15-review-fix-trace-venue.log`). Thousands of live `syscallfmt` fragment formats flowed through the fixed guard on this run.
- `nix develop -c sh build/mm/boot-9kgs-smoke.sh` — **green rc=0** (`GS-EXIT-OK + GS-RENDER-OK + GS-BITS 43269`, capture retained: `evidence/15-review-fix-gs-smoke.log`) and a **negative run rc=1** on a temporarily-erroring fixture (restored after; capture retained: `evidence/15-review-fix-gs-smoke-negative.log`).
- Trace-harness red path driven live (dead kernel → rc=1): nothing promoted, no scratch left, all nine committed `15-06-*` evidence files byte-identical (sha1-checked).
- CI step (WR-06) verified host-static: the workflow YAML parses, `bash -n` clean, and the extracted step was driven through four stubbed outcomes (driver exit 7 → step exits 7 with the summary written, incl. the run-log fallback when the driver died before creating its log; exit 0 + markers → 0; exit 0 without markers → gate still reds).
- Artifacts provenance: the booted kernel/payload carry CR-01+WR-01+WR-02+WR-03 exactly as committed; WR-04/05/06/07 touch only host harnesses, YAML, comments, and planning docs (no boot artifact).

## Fixed Issues

### CR-01: Unvalidated syscall number indexes `systab` out of bounds in the newly-wired trace path (kernel OOB read → panic / kernel-memory leak)

**Files modified:** `sys/src/9k/port/syscallfmt.c`, `sys/src/9k/k10/syscall.c`
**Commit:** 06c884034
**Applied fix:** `syscallfmt`'s guard corrected to `if(syscallno < 0 || syscallno >= nsyscall)` — both the off-by-one (`syscallno == nsyscall` passed and read one element past `systab`) and the entirely-unguarded negative index. The bogus-number arm still prints the number (the designed behavior), so a traced bad syscall leaves a fragment instead of a nil read. At the syscall.c call site the reviewer's suggested skip-fragment guard was deliberately NOT taken: with the sink bounded, suppressing the fragment would only lose trace information; instead a comment now pins the invariant ("nothing in this block may index systab with it") so the constraint survives future edits. `sysretfmt` never indexes `systab`, so the return side needs no change.
**Verification:** kernel rebuilt and booted by the build-9kdraw gate (rc=0); the trace gate at HEAD exercised the formatter live across the rio/acme/sam sessions and the drawtest/arctest rejoin (TRACE-GATE-PASS). Note: no committed gate issues a bogus syscall number, so the out-of-range arm itself is verified by reading + compile, not by a live bogus-number trace.
**Follow-up flagged (out of CR-01's scope, per the review's own "follow-up kernel commit" note):** the pre-existing main-path sinks remain — `syscall.c` line ~267 `systab[scallnr].f` is still read (and called) for negative `scallnr`, and the `else` branch's `ar0 = systab[scallnr].r` (plus the DBGFLG `.n` print) reads out of bounds for ANY out-of-range number — reachable by any process, traced or not (a large positive number reads ~48 GB past the table and would likely panic the kernel). The fixer recommends an immediate follow-up commit clamping the number once at the top of the main body and guarding the else-branch reads.

### WR-01: `startns` read uninitialized when the return-side trace arm happens mid-syscall

**Files modified:** `sys/src/9k/k10/syscall.c`
**Commit:** 07d4557f5
**Applied fix:** `startns = 0;` on its own line before the entry-side trace block (style(6): no initialization at declaration), with a comment naming the startsyscall-while-stopped sequence and the "entry not seen" zero sentinel. The normal path still overwrites it with `todget(nil)` inside the entry block.
**Verification:** same rebuilt-kernel boots and green trace gate as CR-01; the normal (entry-armed) timestamp path is visible in the retained capture's RETURN fragments. The mid-syscall arm sequence itself is not produced by any gate rig (as the review noted for the committed tracesup discipline).

### WR-02: `-initrd` module basename overflows the fixed-size `Dirtab.name` in `addbootfile`

**Files modified:** `sys/src/9k/port/devroot.c`
**Commit:** e93f02772
**Applied fix:** `addlist` now copies with `strecpy(d->name, d->name+sizeof d->name, name)` — the tree's bounded end-pointer idiom, always NUL-terminated within `Dirtab.name[KNAMELEN]` — instead of `strcpy`. No boot-time print on truncation: `multiboot()` runs at `main.c:170`, before `cgainit`/`i8250console` (181-182), so console output is not yet safe there; a too-long name now answers to its truncated name (a visible "file not found" at boot) rather than corrupting the neighbouring `bootdir[]` entries.
**Verification:** every gate boot in this session registered its `-initrd` module through the changed code path (`mod` boot print asserted by the build-9kdraw gate; the venue then mounted the paq through /boot). The >27-character truncation branch itself is reasoned, not booted (no gate feeds a long module name; booting one would break the venue's `bootfs-draw.paq` channel contract by design).

### WR-03: drawsmoke's pixel-readback assert does not accept "either canonical byte order" and false-passes palindromic garbage

**Files modified:** `build/x86_64/drawsmoke.c`
**Commit:** bb18d73c3
**Applied fix:** The comparison now checks the fill color in either canonical byte order (`got`/`rev`) against the pixel value — never "the two readings agree" — so a palindromic wrong-color pattern cannot pass; the x byte (CIgnore) is masked and mismatches print the actual bytes. Applying the reviewer's snippet alone immediately REDDED the live venue, and the diagnostic line exposed the real, worse situation: the readback was **all zero bytes**. Root cause (verified against `devdraw.c`'s 'b' handler and `rgbatoimg`): the probe's constants were RGBA-vs-pixel confused — `Fillval 0x000000ff` is `DBlack` in 'b'-message RGBA (not "blue in x8r8g8b8"), and the GREY1 mask colored `1` converts to mask value 0, fully transparent — so the probe painted nothing on the black-initialized screen and read back zeros; the old palindromic arm is exactly what had been passing (the review's arm-1-matches explanation of the green venue was itself wrong — arm 1 never matched). Fixed the probe too: fill `0x112233FF` (opaque, all channels distinct, non-palindromic — no byte-order alias can satisfy the check), mask `DOpaque`, and the 'd' message's src/mask points zeroed (they were stale bytes of the previous message). Red/green proof retained: the red capture (`.build/review-fix/wr03-red-old-constants-serial.log`, uncommitted working log) vs the green venue boot (`evidence/15-review-fix-draw-venue.log` with DRAW-SMOKE-OK on a real verified pixel readback).

### WR-04: GS-RENDER-OK ignores gs's exit status — a partial render passes the D-11 smoke

**Files modified:** `build/mm/boot-9kgs-smoke.sh`
**Commit:** 9752be29d
**Applied fix:** The baked session captures `$status` on the line after gs (rc idiom with ample tree precedent, e.g. `rc/bin/lp`), prints `GS-EXIT-OK` / `GS-EXIT-FAIL <status>`, still prints `GS-BITS` whenever the bitmap is non-empty (partial renders stay diagnosable), and gates `GS-RENDER-OK` on `~ $gsstatus '' && test -s /tmp/render.bit`. The host classifier asserts `GS-EXIT-OK` by name with its own failure message. The optional exact-byte-count pin (43269) was NOT added: the status gate closes the described hole and a pinned constant would couple the harness to the fixture's page geometry. Verified live in combination with WR-05 (below): the negative run reproduced the exact WR-04 false-pass shape — gs errored AFTER the full 43269-byte bitmap was written — and the harness now reds with `GS-EXIT-FAIL gs 33: 1` + `GS-RENDER-FAIL` + named GS-GATE-FAIL reasons; the green run passes with `GS-EXIT-OK`.

### WR-05: Venue harnesses overwrite the committed phase evidence on failing runs

**Files modified:** `build/mm/boot-9kgs-smoke.sh`, `build/mm/boot-9kdraw-trace.sh`
**Commit:** c60e6448c
**Applied fix:** gs harness: the capture is retained in the evidence dir ONLY on RC=0; a failing run never writes there (its capture stays at `$GS_LOG`, catted to stdout), and the harness gains the evidence-dir/basename override the trace harness had (`GS_EVIDENCE`/`GS_EVIDENCE_NAME`, defaults unchanged). Trace harness: raw extraction and distillation now go to a `mktemp -d` scratch dir and are promoted into `$EVIDENCE` (plus the capture under `$EVNAME.log`) only when the WHOLE run is green — so a proven venue with a failed session no longer overwrites `15-06-*`; a failing run keeps its partial scratch dir, named on stdout, for diagnosis. Verified live in both directions: gs negative run (default evidence dir) left `evidence/15-07-gs-smoke.log` byte-identical (sha1 + mtime checked); trace red run (dead kernel) left all nine `15-06-*` files byte-identical and no scratch residue; both green runs promoted/retained correctly (the trace gate's own green run in this session used `TRACE_EVIDENCE`/`TRACE_EVIDENCE_NAME` overrides pointing at a scratch dir so the committed 15-06 evidence of record was never rewritten by this fix pass).

### WR-06: tier-2 CI leg's failure path is dead — `cmd | tee || handler` without pipefail

**Files modified:** `.github/workflows/phase6.yml`
**Commit:** 63b4bda93
**Applied fix:** `set -o pipefail` + `rc=0; cmd | tee log || rc=$?` + an `if [ "$rc" -ne 0 ]` handler that writes the step summary and `exit "$rc"`, with a `tail` fallback to `.build/tier2-run.log` when the driver died before creating its own log. IMPORTANT deviation from the review's suggested snippet: `if ! cmd | tee; then rc=$?` was measured to capture the `!` operator's status (always 0), so `exit $rc` would have exited the step successfully and SKIPPED the `grep -q TIER-BUILD-OK` gate — the literal suggestion traded lost diagnosability for a false green. The corrected form was driven host-statically through four stubbed outcomes (see verification section); GitHub's default `bash -e {0}` has no pipefail, which `set -o pipefail` supplies.

### WR-07: The MUL2 round-to-nearest identity only holds for `s <= 65025`; the "byte path equally covered" claim overstates

**Files modified:** `sys/src/libmemdraw/draw.c`, `sys/src/libmemdraw/drawtest.c`, `.planning/phases/15-userspace-tiers-bootfs-payload-amd64/deferred-items.md`
**Commit:** 38bcb9854
**Applied fix:** Documentation scoping chosen over an arithmetic change (the reviewer's first option; a macro reshaping could not be proven green — the MUL0123 lane-path deferral already blocks alpha-channel drawtest, and a shape identical on [0,65025] changes no proven behavior while one that isn't risks the re-join). The reviewer's numbers were independently re-verified numerically before writing anything: 253 diverging values, ALL in (65025, 130050], first at s=65663 (257 vs 258), none in [0,65025]. The live texts now carry the bounded claim: draw.c's MUL2 comment scopes the identity to the convex (no-alpha / premultiplied) regime and names the non-convex residue; drawtest.c's reference comment (which uses exact `(s+127)/255` division — the reference, not the macro, is the ideal) notes the same bound; and the residue is recorded as a deferred item next to the lane-path deferral, with the pointer for the eventual clamped two-term fix. The overstated sentence itself lives only in 15-09-SUMMARY.md (immutable history) — no live comment or contract repeated it.
**Verification:** both files compile clean under `6c -FTVw` (comment-only changes; no object-code delta), and the venue's DRAWTEST-OK/ARCTEST-OK re-join stayed green in both live boots.

---

**Out of scope (left open deliberately):** IN-01..IN-07 (info tier) — recorded in the disposition ledger; the ledger itself was not touched by this pass.

**Follow-ups surfaced by this pass (not fixed here):**
1. The pre-existing `systab` sinks in `syscall.c`'s main body (see CR-01 above) — negative-index `.f` call and the else-branch `.r`/`.n` OOB reads for any out-of-range number, reachable untraced by any process. One clamp at the top of the main body plus guarded else-branch reads closes the class.
2. drawsmoke's pixel readback had never actually verified pixels before this pass (transparent-mask/opaque-black constants + the palindromic arm); any evidence produced by the 15-05/15-06/15-08 boots' DRAW-SMOKE-OK should be read as "draw loop ran", not "pixels verified". The venue is now honest (red/green pair retained).

_Fixed: 2026-11-10T20:57:56Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
