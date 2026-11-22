---
phase: 15-userspace-tiers-bootfs-payload-amd64
reviewed: 2026-11-10T21:30:00Z
depth: deep
files_reviewed: 55
files_reviewed_list:
  - .github/workflows/phase6.yml
  - .gitignore
  - build/mm/boot-9k10t1-sweep.sh
  - build/mm/boot-9kdraw-trace.sh
  - build/mm/boot-9kgs-smoke.sh
  - build/mm/build-9k10t1.sh
  - build/mm/build-9kdraw.sh
  - build/x86_64/bootfs-payload.txt
  - build/x86_64/coverage-sweep.sh
  - build/x86_64/dir-coverage.txt
  - build/x86_64/drawsmoke.c
  - build/x86_64/gs-render.ps
  - build/x86_64/mkimage.sh
  - build/x86_64/replay-acme.rc
  - build/x86_64/replay-rio.rc
  - build/x86_64/replay-sam.rc
  - build/x86_64/root-tree/lib/ndb/local.template
  - build/x86_64/root-tree/usr/glenda/lib/profile
  - build/x86_64/services-r9.expect
  - build/x86_64/services-sweep.rc
  - build/x86_64/subset-build.sh
  - build/x86_64/svctcpecho.c
  - build/x86_64/tier1-r9.expect
  - build/x86_64/tier1-sweep.rc
  - build/x86_64/tier2.ledger
  - build/x86_64/tier3.ledger
  - build/x86_64/tiers/tier0-libs.txt
  - build/x86_64/tiers/tier2-services.txt
  - build/x86_64/tiers/tier3-gui.txt
  - build/x86_64/tiers/tier4-tail.txt
  - build/x86_64/trace-distill.sh
  - nix/services-r9-x64.sh
  - nix/tier1-r9-x64.sh
  - sys/src/9k/k10/asm.c
  - sys/src/9k/k10/k10draw
  - sys/src/9k/k10/k10t1
  - sys/src/9k/k10/mkfile
  - sys/src/9k/k10/multiboot.c
  - sys/src/9k/k10/root/k10draw.rc
  - sys/src/9k/k10/syscall.c
  - sys/src/9k/k10/vgadispi.c
  - sys/src/9k/port/devcons.c
  - sys/src/9k/port/devroot.c
  - sys/src/9k/port/syscallfmt.c
  - sys/src/cmd/auth/factotum/mkfile
  - sys/src/cmd/auth/mkfile
  - sys/src/cmd/aux/mkfile
  - sys/src/cmd/ip/mkfile
  - sys/src/cmd/mkfile
  - sys/src/libmemdraw/addr.c
  - sys/src/libmemdraw/draw.c
  - sys/src/libmemdraw/drawtest.c
  - sys/src/libmemdraw/mkfile
  - sys/src/liboventi/mkfile
  - sys/src/mkfile
findings:
  critical: 1
  warning: 7
  info: 7
  total: 15
status: issues_found
---

# Phase 15: Code Review Report

**Reviewed:** 2026-11-10T21:30:00Z
**Depth:** deep (per-file + cross-file: kernel call chains traced into devproc/proc/multiboot/asm, mkfile filter sites cross-checked against the tier data and drivers, harness classifiers traced against the sweep/expect marker producers)
**Files Reviewed:** 55 (the phase evaluation scope; `flake.nix` — modified by a fix commit outside the task-commit union — was also read and is clean)
**Status:** issues_found

## Summary

Phase 15 is a build/venue-hardening phase: tier-2/3/4 allowlist machinery wired into six mkfile awk filter sites, two boot venues (k10t1 services donor, k10draw draw donor), an r9 services gate, a syscall-trace rig, a full-tree coverage proof, a DESK-03 payload bake, and small targeted kernel C deltas (syscall trace fragments, DISPI VGAdev, synthetic input, module-span meminit fix, libmemdraw pixel-path fixes).

The overall engineering quality is unusually high: exit polarity and marker anchoring in the harnesses are almost everywhere correct (boundary-anchored classifier regexes, computed expectations with both-direction SKIP converses, fresh-capture hygiene, magic checks on every artifact, flock'd shared-tree discipline, per-job CI timeouts, SHA-pinned actions). The libmemdraw arithmetic fixes were independently re-verified numerically during this review: the new MUL2 macro is exactly `(s+127)/255` for all `s <= 65025`, and the old double-MUL diverges on ~50% of inputs — the 15-09 root-cause holds (see WR-07 for the range caveat). The meminit rewrite in asm.c is arithmetically sound (`vmstart`-based identity mapping, postconditions preserved).

However, the review found one critical kernel memory-safety defect newly activated by the trace substrate (unvalidated syscall number indexing `systab` before validation — reachable by any traced process), plus seven warnings: an uninitialized `startns` read, a boot-time `strcpy` overflow from the `-initrd` module name, a draw-probe assert that does not do what its comment claims, a gs render marker that ignores gs's exit status, harnesses that clobber committed evidence on failing runs, a dead CI failure path, and an overstated byte-path coverage claim in the compositing fix. The kernel-side items (CR-01, WR-01, WR-02) should be fixed before this code ships; the gate/harness items degrade verification trustworthiness and diagnosability.

## Structural Findings (fallow)

None provided — no structural pre-pass block was supplied for this phase.

## Narrative Findings (AI reviewer)

### Critical Issues

### CR-01: Unvalidated syscall number indexes `systab` out of bounds in the newly-wired trace path (kernel OOB read → panic / kernel-memory leak)

**File:** `sys/src/9k/port/syscallfmt.c:73-76` (the sink), `sys/src/9k/k10/syscall.c:228-249` (the newly-added caller), `sys/src/9k/k10/syscall.c:267` (pre-existing companion sink)
**Issue:** 15-06 wired `syscall.c`'s `Proc_tracesyscall` entry stop to call `syscallfmt(scallnr, ...)` (syscall.c:247) **before** the syscall number is validated (the `scallnr >= nsyscall || systab[scallnr].f == nil` check is at line 267, and it runs only in the main body). `syscallfmt` guards its table access with `if(syscallno > nsyscall)` — an off-by-one: `nsyscall == nelem(systab)` (port/systab.c:122), so `syscallno == nsyscall` passes the guard and reads `systab[nsyscall].n`, one element past the array. Worse, **negative numbers are entirely unguarded**: `scallnr` is the low 32 bits of the user-controlled syscall register (`PUSHQ RARG` in `l64syscall.s:47`, RARG = AX at `SYSCALL` entry), so a traced process issuing `syscall(-N)` reads `systab[-N].n` — an arbitrary-offset kernel data read whose garbage value is then printed with `%s` (`fmtprint(&fmt, "%s ", systab[syscallno].n)`), faulting the kernel if the garbage pointer is unmapped (panic) or leaking kernel memory into `/proc/N/syscall`. The same unvalidated index class exists pre-phase at syscall.c:267/291: for negative `scallnr`, `scallnr >= nsyscall` is false, so `systab[scallnr].f` is read out of bounds and, if non-nil, **called** — but that expression is unchanged upstream code; the phase's contribution is that (a) syscallfmt.c was dead code until now ("its first 9k compile", per the 15-06 summary) and (b) the new call site moved a formatter walk of the unvalidated number ahead of validation. A traced process can deterministically panic the kernel; this is the same defect class the phase itself fixed in `fmtuserstring` (a traced process corrupting/panicking the kernel via the trace path).

Reachability requires `up->procctl == Proc_tracesyscall` (armed via `/proc/N/ctl` `startsyscall`, devproc.c:1457-1460) plus a hand-crafted syscall number — no libc-using binary issues one, so the committed gates stay green; but the kernel code as committed accepts it.

**Fix:** Validate the number once, before any table access, in both places:

```c
/* syscall.c, entry trace block — reject before formatting */
if(up->procctl == Proc_tracesyscall){
	if(scallnr < 0 || scallnr >= nsyscall){
		/* no fragment for a bogus number; the main path reports it */
	} else if(!waserror()){
		...
	}
```

and in `syscallfmt.c:73`:

```c
	if(syscallno < 0 || syscallno >= nsyscall)
		fmtprint(&fmt, " %d ", syscallno);
	else
		fmtprint(&fmt, "%s ", systab[syscallno].n);
```

(The legacy tree `9/pc/trap.c` + `9/port/syscallfmt.c` carry the identical `>` bug; fixing 9k's copy is in-tree-fix-preferred per project constraints. Line 267's negative-index `systab[scallnr].f` deserves the same clamp in a follow-up kernel commit — it predates this phase but is one `if` away from arbitrary kernel function-pointer calls.)

## Warnings

### WR-01: `startns` read uninitialized when the return-side trace arm happens mid-syscall

**File:** `sys/src/9k/k10/syscall.c:213,255,308-314`
**Issue:** `startns` is assigned only inside the entry-side trace block (line 255). The return-side block (line 308) uses it (line 314) whenever `procctl == Proc_tracesyscall` at return. `CMstartsyscall` (devproc.c:1457-1460) sets `p->procctl = Proc_tracesyscall` on a **stopped** process — and a process blocked inside a syscall can be stopped and then released with `startsyscall`. In that sequence the syscall completes with the return-side trace armed but the entry block never having run: `sysretfmt` receives garbage stack data as the start timestamp and prints it into `/proc/N/syscall`. No crash (passed by value), but the trace evidence is silently wrong. The committed tracesup rig always arms at exec boundaries so its own tables are unaffected.
**Fix:** Initialize `vlong startns, stopns;` to 0 (or gate the return block's `sysretfmt` on an `armed` flag set by the entry block), e.g. `vlong startns = 0, stopns = 0;` — one line, removes the undefined behavior.

### WR-02: `-initrd` module basename overflows the fixed-size `Dirtab.name` in `addbootfile`

**File:** `sys/src/9k/k10/multiboot.c:106-113`, `sys/src/9k/port/devroot.c:57-74`
**Issue:** 15-05's multiboot delta exposes each mb1 module through `addbootfile(q, ...)` where `q` is the module string's basename — the host path given to `qemu -initrd`, i.e., externally controlled boot input. `addlist` does `strcpy(d->name, name)` into `char name[KNAMELEN]` (portdat.h:224, `KNAMELEN == 28` per port/lib.h:206) with no bound. A basename longer than 27 characters overflows the static `bootdir[]` array, corrupting the adjacent Dirtab entries' qid/length/perm (or `bootdata[]` past the end) — silent kernel memory corruption at boot. Current harness names (`bootfs-draw.paq`, `bootfs-gs.paq`) are short, so nothing fires today, but nothing enforces it.
**Fix:** Bound the copy in `addbootfile`/`addlist`:

```c
static void
addlist(Dirlist *l, char *name, uchar *contents, ulong len, int perm)
{
	...
	strncpy(d->name, name, KNAMELEN-1);
	d->name[KNAMELEN-1] = 0;
	...
}
```

(or reject names that don't fit with a boot-time print — the other `addbootfile` callers pass compile-time names that stay short).

### WR-03: drawsmoke's pixel-readback assert does not accept "either canonical byte order" and false-passes palindromic garbage

**File:** `build/x86_64/drawsmoke.c:240-249`
**Issue:** The comment claims the check "accept[s] the fill color in either canonical byte order", but the code fails unless `BE(p) == Fillval` **or** `BE(p) == LE(p)` — the second arm is satisfied by any palindromic byte pattern (`p[0]==p[3] && p[1]==p[2]`) regardless of color, while a genuinely little-endian wire (`ff 00 00 00` for `0x000000ff`) is *rejected*. So the assert is simultaneously weaker than documented (wrong-color palindromic pixels pass) and stricter than documented (the LE order it claims to accept would fail). The venue is green because the draw protocol's `r` unload is big-endian-per-value and arm 1 matches, making arm 2 dead weight that only opens the false-green hole.
**Fix:**

```c
	got = (p[0]<<24)|(p[1]<<16)|(p[2]<<8)|p[3];
	rev = (p[3]<<24)|(p[2]<<16)|(p[1]<<8)|p[0];
	if(got != Fillval && rev != Fillval)
		fail("pixel mismatch");
```

### WR-04: GS-RENDER-OK ignores gs's exit status — a partial render passes the D-11 smoke

**File:** `build/mm/boot-9kgs-smoke.sh:172-179` (baked session), `:327-339` (classifier)
**Issue:** The in-guest session runs `/bin/gs ...` with its status discarded and gates `GS-RENDER-OK` purely on `test -s /tmp/render.bit`. ppmraw writes the PPM header and rows incrementally, so a gs that fails mid-render (e.g., an `Unrecoverable error` after output started, `exits("error")` — exactly the error class this harness already met at init time on its fourth bring-up boot) leaves a non-empty partial file: `GS-RENDER-OK` + `GS-BITS n>0` print and the host classifier passes. The zero-panic/trap grep only catches kernel-reported faults, not a clean gs failure exit. The harness's own history shows gs failing in exactly this "Unrecoverable error" mode.
**Fix:** Gate the marker on gs's status in the baked session (`/bin/gs ... && echo gs-exited-ok` into a probe file checked alongside `test -s`), and/or assert the deterministic byte count for the fixed fixture (43269) instead of merely non-empty — the fixture is committed, so its rendered size is a constant.

### WR-05: Venue harnesses overwrite the committed phase evidence on failing runs

**File:** `build/mm/boot-9kgs-smoke.sh:341-348`, `build/mm/boot-9kdraw-trace.sh:228-256`
**Issue:** The gs harness **unconditionally** copies the serial capture to `.planning/phases/15-.../evidence/15-07-gs-smoke.log` — including when the gate fails (RC=1): any later red or bring-up run silently replaces the retained green evidence of record in the working tree. The trace harness is one step better (writes only when the venue is "proven") but still overwrites `15-06-trace-venue.log`, the raw/table files, and the capture whenever the four bring-up markers land even if every session failed (RC=1). Only a deliberate `TRACE_EVIDENCE` override (used by the executor for diagnostic runs, per the 15-06 summary) protects the committed copies — nothing in the scripts themselves distinguishes "evidence of record" from "diagnostic capture", and the gs harness has no override at all.
**Fix:** Retain into the evidence dir only on pass; on failure write to a `-FAIL` suffixed name (or `$BOOTLOG` only), e.g. `if [ "$RC" -eq 0 ]; then cp "$BOOTLOG" "$EVID/15-07-gs-smoke.log"; fi` — and give the gs harness the same evidence-dir override the trace harness has.

### WR-06: tier-2 CI leg's failure path is dead — `cmd | tee || handler` without pipefail

**File:** `.github/workflows/phase6.yml:285-290`
**Issue:** `nix develop -c sh build/x86_64/subset-build.sh 2>&1 | tee .build/tier2-run.log || { rc=$?; ... GITHUB_STEP_SUMMARY ... }` — the pipeline's status is `tee`'s (GitHub's default shell has no `pipefail`), so the `||` arm only fires if `tee` itself fails; and even then `rc=$?` would capture tee's status, not the build's. The documented behavior ("On failure the log tail lands in the step summary, D-16's convention") never happens for the actual failure mode (subset-build non-zero). The gate itself stays honest — the following `grep -q TIER-BUILD-OK` reds the step — so this is lost diagnosability, not a false green.
**Fix:** Enable pipefail for that block and capture rc before the pipe:

```yaml
        run: |
          set -o pipefail
          if ! nix develop -c sh build/x86_64/subset-build.sh 2>&1 | tee .build/tier2-run.log; then
            rc=$?
            { echo "## ❌ tier-2 compile failed (rc=$rc)"; echo '```'; tail -60 .build/subset-build-x64.log; echo '```'; } >> "$GITHUB_STEP_SUMMARY"
            exit $rc
          fi
```

### WR-07: The MUL2 round-to-nearest identity only holds for `s <= 65025`; the "byte path equally covered" claim overstates

**File:** `sys/src/libmemdraw/draw.c:28-36` (MUL2), `sys/src/libmemdraw/drawtest.c:974-1000` (reference)
**Issue:** Verified numerically during this review: `(t+(t>>8))>>8 == (s+127)/255` with `t=s+128` is exact for every `s` in `[0, 65025]` — which is the whole reachable range for the default no-alpha `r8g8b8` channels (`sa=255` forces `M = 255-m`, so `m+M == 255` and `s = sr*m + dr*M <= 255*255`), so the committed DRAWTEST-OK re-join and the fix's core claim are sound. **But** the call sites sum two products (`fs*src + fd*dst`), and with alpha-bearing sources the coefficient pair is non-convex (`M = 255-round(sa*m/255)` leaves `m+M > 255` whenever `sa*m <= 127`, e.g., a transparent source with non-premultiplied color channels), so `s` can reach `130050`. In `(65025, 130050]` the library macro and the (equally-updated) test reference diverge on 253 values by 1 LSB — surviving 8-bit truncation (first at `s=65663`: 257 vs 258 → 1 vs 2). The 15-06 re-join contract's claim that "any channel arguments the rig passes are equally covered on the byte path" is therefore not proven: `mk 6.drawtest` with alpha or mixed channels (e.g., `r8g8b8a8 r8g8b8`) on the byte path can still abort stochastically. This is distinct from the documented rgba-lane (`MUL0123`) deferral, which the summary records — this residue on the byte path is not recorded anywhere.
**Fix:** Either scope the claim in the contract/summary to the convex (no-alpha / premultiplied) regime and log the extended-range divergence as a deferred item next to the lane-path one, or adopt plan9port's clamped shape for the two-term case so the identity holds to 130050.

## Info

### IN-01: Lockless `/proc/N/syscall` read races the new per-syscall free sites

**File:** `sys/src/9k/port/devproc.c:752-755`, `sys/src/9k/k10/syscall.c:252-254,320-322`, `sys/src/9k/port/syscallfmt.c:78-79,326,341-342`
**Issue:** `Qsyscall` reads `p->syscalltrace` with no `qlock(&p->debug)` and no Stopped-state check, while the now-live trace code frees and reassigns that pointer on every traced syscall (the legacy tree has the same unlocked read, but on 9k the field was always nil until this phase). A reader of a *running* traced process can race the free — use-after-free in `readstr`. The tracesup discipline (read only while stopped) is the de facto guard, and the reader must already control the process, so this is a hardening item, not an exploit path on these venues.
**Fix:** Take `qlock(&p->debug)` (and/or require `p->state == Stopped`) around the `Qsyscall` copy, as neighboring cases in `devproc.c` already do.

### IN-02: Stale contradictory comment in the committed sam replay driver

**File:** `build/x86_64/replay-sam.rc:43-45` vs `:65`
**Issue:** Comment block "0." states the file is "argv-loaded inside the startup handshake, so no mid-session B/load exchange (see the session-driver note)" — but line 65 types `B /tmp/sam-replay.txt` into kbin (a mid-session load), and the actual session-driver note in build-9kdraw.sh:507-513 says the argv load was measured dead and bare-sam + mid-session B is what runs. These replay files are the committed measurement record that Phase 17+ plans cite; the comment describes a launch protocol that is not the one executed.
**Fix:** Replace the "0." comment with the driver's actual shape (bare sam; the B command below loads/creates the file).

### IN-03: Dead no-op loop in build-9kdraw.sh

**File:** `build/mm/build-9kdraw.sh:149`
**Issue:** `for t in drawtest arctest; do :; done` does nothing — a vestige of a removed generic loop (the two builds below it are spelled out individually).
**Fix:** Delete the line.

### IN-04: Off-by-one entry count in the k10t1 config comment

**File:** `sys/src/9k/k10/k10t1:21`
**Issue:** The comment says "57 entries at the full leg; still under the 63-addbootfile capacity" but the rootdir growth is 15 files over the original 43 = 58 — the value the config's own build assert and build-9k10t1.sh:235 both enforce (`embedded root files: 58`).
**Fix:** Correct the comment to 58.

### IN-05: coverage-sweep "worst status wins" is order-dependent between fail and hang

**File:** `build/x86_64/coverage-sweep.sh:567-586`
**Issue:** The row classifier overwrites `cls[d]` on each EXIT, so a fail-then-hang key labels `hang` while hang-then-fail labels `fail` — contradicting the "worst status wins" comment. Both classes count as not-ok (`BAD_N`), so the green gate is unaffected; only the diagnostic label varies with visitation order.
**Fix:** Make hang sticky (`if (cls[d] != "hang" && (st==124||st==137)) cls[d]="hang"; else if (...)`) or fix the comment.

### IN-06: The passwordless (userns) venue is NixOS-only via hardcoded `/run/current-system/sw/bin`

**File:** `build/x86_64/coverage-sweep.sh:103,471-475` (new this phase); same shape inherited in `build/x86_64/subset-build.sh:527-579` and `build/mm/build-9kdraw.sh:233-276`
**Issue:** The host-side "system tools FIRST" PATH pin and the chroot's `/usr/bin` symlink farm resolve through `/run/current-system/sw/bin`; on a non-NixOS Linux host that directory does not exist, the pin silently degrades (plan9 `ls`/`sed`/`comm` can garble the host-side universe computations — the exact failure the pin exists to prevent), and the userns scaffold creates dangling symlinks. The sudo venue and CI (ubuntu) are unaffected, and the grep/timeout resolution falls back gracefully, so this is a portability note for the new coverage driver, consistent with the repo's Nix-first devshell stance.
**Fix:** Probe the system bin dir once (as `GREP` already does) and fail loudly if the pinned directory is absent, instead of silently prepending a nonexistent path.

### IN-07: services-r9.expect degrades to a 900s stage timeout instead of the documented post-pass classification

**File:** `build/x86_64/services-r9.expect:148-171` (comment at 152-154)
**Issue:** The header says a failed `#I` bind "degrades to the named skip-why line and the post-pass/classify reds on the missing probes", but the ordered stage list stalls first: with the bind failed the sweep emits `SVC-SKIP cs` and no `SVC-CSOBS`/`SVC-FAIL cs`, so the `cs-verdict` stage waits its full 900s timeout and exits 1 as "TIMEOUT at stage: cs-verdict" — correct polarity, but the failure is slow and misattributed relative to the comment.
**Fix:** Add the skip arms to the verdict stages (`{SVC-(OK|FAIL) cs|SVC-SKIP cs}`) or lower the per-stage timeout when a `SVC-SKIP-WHY` line has been seen, so degraded runs classify at the post-pass as documented.

---

**Verified-clean spot checks worth recording:** the six mkfile filter sites read the four-file tier union in one awk call each with the `$ALLDIRS:V:` parse-error fix correctly applied (aux); `subset-build.sh`'s tier-2/3 dispatch, mtime-stamp harvest, magic checks and lock discipline are correct; `mkimage.sh` section 11/P computes the tier-2 payload from the tier data with duplicate-h* detection, digest-pinned sentinel, mkpaqfs per-file cap assert, and both-direction token substitution asserts; `services-sweep.rc`'s rc-dialect ladders, bounded spins and exactly-one-verdict-per-branch marker logic are correct, and the harnesses' `SVC-SKIP`-with-space extraction cannot match the `SVC-SKIP-WHY` lines; `boot-9k10t1-sweep.sh`'s computed expectations and converses are correctly anchored; the asm.c meminit rewrite maps `[vmstart, vmend)` consistently with `KZERO`-based identity mapping and preserves the fit-loop arithmetic; vgadispi.c's flush math matches the vgavesa soft-screen discipline; devcons's `Qkbin` is correctly eve-gated at open and overflow-safe through kbdputc's ring; syscall.c's `waserror` guards are balanced against the `nerrlab` reset; the tier data files, ledgers, and dir-coverage.txt are internally consistent with the counts the summaries claim (65/91, 11/12, 702/702).

_Reviewed: 2026-11-10T21:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
