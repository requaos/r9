---
phase: 19-cpu-service-drawterm-rio-desktop
fixed_at: 2026-11-17T12:52:10Z
review_path: .planning/phases/19-cpu-service-drawterm-rio-desktop/19-REVIEW.md
iteration: 1
findings_in_scope: 2
findings_fixed: 2
fixed: 2
skipped: 0
status: all_fixed
fix_commit: 13091fe41
---

# Phase 19: Code Review Fix Report

**Fixed at:** 2026-11-17T12:52:10Z
**Source review:** .planning/phases/19-cpu-service-drawterm-rio-desktop/19-REVIEW.md
**Iteration:** 1
**Fix commit:** 13091fe41 (fix + self-test, one atomic commit)

**Summary:**
- Findings in scope for this fix pass: 2 (CR-01, WR-01 — both in
  `build/x86_64/authdomain-gen.c`, per the fix directive)
- Fixed: 2
- Skipped: 0
- Out of this pass by directive (dispositions recorded in
  19-REVIEW-DISPOSITION.md): WR-02 and IN-01 deferred (r9 kernel
  bytes / hardening milestone — reopeners appended to
  deferred-items.md); IN-02 skipped (donor-carried
  defense-in-depth kept by donor-fidelity judgment)

## Fixed Issues

### CR-01: One-byte stack out-of-bounds write in `verify()`/`info()` when a staged file is exactly Filecap bytes

**Files modified:** `build/x86_64/authdomain-gen.c`,
`build/x86_64/authdomain-selftest.sh`
**Commit:** 13091fe41
**Applied fix:** Chose the "read with room for the NUL" shape at BOTH
sites: `verify()` and `info()` now declare `char
manifest[Filecap+1]` over the unchanged `Filecap` read cap, so the
`manifest[n] = 0` terminator lands in-bounds when `readfile` accepts a
manifest of exactly 65536 bytes (readfile's exactly-cap acceptance is
kept by design — only a cap+1'th byte refuses). Anything that big is
still refused loud by verify's strict shape checks (trailing-rows /
label-length / unknown-row classes); nothing is silently truncated.
Why-comments at both sites name the boundary and CR-01.

**Verification** (all in the main checkout — `workflow.use_worktrees`
is false for this repo, so no isolated worktree was involved):

- Tier 1/2: `cc -fsyntax-only` over the vendored-donor include path
  clean; the self-test's own host-cc build line (the same shape
  mkimage §14d uses) compiles the tool clean.
- RED proof of the pin's teeth: the PRE-fix source (`git show
  HEAD:build/x86_64/authdomain-gen.c`), built `-fsanitize=address`
  and run `-V` over a staged exactly-65536-byte manifest, fires
  `ERROR: AddressSanitizer: stack-buffer-overflow` — the bug is real
  and the new leg would have caught it.
- New self-test legs (green): the boundary leg stages a manifest of
  exactly Filecap bytes — `-V` refuses loud with the trailing-rows
  marker (`FILECAP-BOUNDARY-REFUSED`), `-i` parses it clean — and an
  ASan build of the tool re-runs both with no sanitizer report
  (`FILECAP-BOUNDARY-ASAN-CLEAN`; leak detection off — the leg pins
  the overwrite, not donor allocation hygiene; a cc without ASan
  degrades to the behavioral pin with a printed note).
- Full self-test end to end: `GEN-SELFTEST-OK`, exit 0, twice (flake
  check); every pre-existing leg stayed green.

### WR-01: Modulo bias in generated passwords (`alph[r[i] % 62]`)

**Files modified:** `build/x86_64/authdomain-gen.c`,
`build/x86_64/authdomain-selftest.sh`
**Commit:** 13091fe41
**Applied fix:** `genpassword()` now draws each character by
rejection sampling in the file's donor-Plan-9 style: one urandom byte
at a time, redrawing while `c >= 256/(nelem(alph)-1)*(nelem(alph)-1)`
(= 248 = 4*62, the largest alphabet multiple under 256), then indexing
`alph[c % (nelem(alph)-1)]`. The surviving bytes hit each of the 62
residues exactly 4 times per 248, killing the ~25% relative
'0'-'7' favoritism in the strings that derive the nvram machkey (the
/adm/keys master) and p9sk1 user keys. The batch `uchar r[Maxpw]`
draw is gone; the function comment documents the arithmetic and why
it matters.

**Verification:**

- Tier 1/2: as CR-01 (syntax-only + the self-test build line).
- New self-test leg (green): 80 DEMO generates harvest the 12-char
  feed passwords (960 chars); the leg asserts every char is in
  `[0-9a-zA-Z]` and every one of the 62 alphabet symbols is reachable
  (`GENPASSWORD-ALPHABET-OK`; a missed symbol over 960 uniform draws
  is a <1e-5 flake). Termination of the redraw loop is exercised by
  all 80 runs. Honest limit, stated in the leg's comments: the
  uniformity of the distribution itself is not pinnable with a cheap
  shell leg — the pin is charset + reachability + termination over
  the fix's code shape.
- Full self-test green as above (twice).

## §14/§15 bake consumption — what actually ran

The tool's §14/§15 consumers (mkimage's 14d/§15 authdomain-gen
invocations) run only inside the flake boot apps (`nix run
.#boot-r9-x64 -- --system=cpu` etc.), whose step 3/3 boots a qemu
guest — and this pass was constrained to boot no guests, so no bake
run was made. The self-test was therefore the verification bar, per
the process constraint. It exercises the exact consumption surface
short of the guest: the same host-cc vendored-donor build line as
mkimage 14d, plus `-o`/`-V`/`-i` round-trips over the produced unit.
(The only guest-free mkimage path, `tier1-r9-x64.sh`, bakes the tier
section, not §14/§15.)

## Skipped Issues

None among the in-scope findings (the out-of-pass dispositions for
WR-02 / IN-01 / IN-02 live in 19-REVIEW-DISPOSITION.md and
deferred-items.md, updated in the docs commit).

---

_Fixed: 2026-11-17T12:52:10Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
