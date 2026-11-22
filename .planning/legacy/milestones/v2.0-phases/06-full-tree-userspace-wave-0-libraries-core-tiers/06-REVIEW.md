---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
reviewed: 2026-11-05T22:34:43Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - build/aarch64/coverage-sweep.sh
  - build/aarch64/dir-coverage.txt
  - build/aarch64/gethunk-repro-6l.sh
  - build/aarch64/mkimage.sh
  - build/aarch64/netaddr.rc
  - build/aarch64/root-tree/lib/netip-loopback.rc
  - build/aarch64/subset-build.sh
  - build/aarch64/subset-exec.sh
  - build/aarch64/tcpblast.c
  - build/aarch64/tcpecho.c
  - build/aarch64/tier1-sweep.rc
  - build/mm/boot-9k10t1-sweep.sh
  - build/mm/build-9k10t1.sh
  - flake.lock
  - .github/workflows/phase6.yml
  - .gitignore
  - sys/src/9k/k10/k10t1
  - sys/src/9k/k10/l32p.s
  - sys/src/9k/k10/root/k10t1.rc
  - sys/src/9k/port/devroot.c
  - sys/src/BUILD_SETUP_NAMESPACE
  - sys/src/cmd/6l/asm.c
  - sys/src/cmd/cpp/BUILD
  - sys/src/cmd/go/mkfile
findings:
  critical: 0
  warning: 2
  info: 6
  total: 8
status: issues_found
---

# Phase 06: Code Review Report

**Reviewed:** 2026-11-05T22:34:43Z
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Summary

Standard-depth review of the phase-6 wave-0 changeset (host build/gate harnesses, the 06-09 k10t1 tier venue, the 6l LP64 float-constant fix, and the phase6.yml CI pipeline), read against diff_base 9795e2d1c. The prior CR-01 (9front tier-donor CI leg) stays resolved-by-retirement per 06-REVIEW-DISPOSITION.md — the venue-pin step in phase6.yml now CI-asserts that retirement, and nothing in this review resurrects it.

The high-risk changes were traced to ground and held up:

- **sys/src/cmd/6l/asm.c (LP64 ieee fix)** — verified correct: `struct Ieee {long l; long h;}` (sys/src/cmd/ld/ld.h:15, struct def in ld.h) is 16 bytes on LP64 hosts vs 8 on Plan 9; the old struct-address emit read `l`'s zero high half instead of `h`. The two-int replacement reproduces the Plan 9 packed layout on little-endian hosts, and `fnuxi8` (6l/obj.c:1527) is the identity map there. The `case 4` float path (`ieeedtof` → address of a single `long`) is unaffected because only the low 4 bytes are indexed. The narrowing `long`→`int` assignments preserve the low 32 bits regardless of sign/zero extension. Not a regression on big-endian hosts relative to the old code (both were broken there; unsupported host class).
- **sys/src/9k/k10/l32p.s (0–4MiB → 0–8MiB entry map)** — slot math verified against k10/mem.h: `PDO(KZERO)` = slot 384 (KZERO=0xfffffffff0000000, PTLX shift 21), so the four KZERO PDEs occupy slots 384–387, consecutive and free; PMAPADDR (0xffffffffffe00000) lives in a different PDP entry entirely (0x1F8 vs KZERO's 0x1FC), no aliasing. The identity map (slot 0 only) and its `_zap0*` teardown are untouched and still consistent with the header comment's KZERO-bit conditions. Physical addresses 0/2/4/6MiB + PtePS|PteRW|PteP sequence is correct.
- **sys/src/9k/port/devroot.c (Nbootfiles 32→64)** — arrays/counters scale together; qid ranges (root 1..10, boot 0x1001..0x1040) don't overlap; 43 embed + 1 static entry ≤ 64. No other tree code keys on the old 32 (only sys/src/9/port/devroot.c, the legacy tree, keeps 32 — and subset-exec.sh's runtime patch of the *donor's* copy to 128 is separate scratch).
- **k10t1 config ↔ build-9k10t1.sh ↔ boot-9k10t1-sweep.sh ↔ tier1-sweep.rc parity** — rootdir carries exactly 43 entries (35 h* + staged rc/echo + bind + rcmain + boot script + tier1-sweep.rc + sentinel.txt + nm-input.7), matching the build's `addbootfile(` count assert. The harness's computed expected set (tier1-cmds.txt minus 7 driver-only exclusions, plus float-d01/selfhost×2/libtest×2 = 60 markers) exactly matches what the 9k-venue sweep emits (36 read-only T1-OK candidates + 22 literal `echo T1-SKIP` + 2 helper-emitted libtest SKIPs), and all 24 SKIP names are in the harness whitelist; the t1skip-helper trick correctly keeps the 9k-only libtest SKIP names out of the r9 gate's literal-grep want-set. The 9k venue's 35 embedded h* binaries cover every read-only sweep check.
- **k10t1.rc shebang `#!/boot/rc -m /boot/rcmain`** — verified against 9k/port/sysproc.c:296 (`tokenize(line+2, …)` splits multi-word shebangs) and rc's `-m` flag (rc/exec.c:188); the embedded boot script's `/dev/time` write rides devrtc's 12-byte `readnum` format under devcons `writetime`'s 13-byte cap, and happens *before* the `#c/hostowner` write while the process is still eve (writetime is eve-gated) — ordering is correct.
- **netip-loopback.rc's `ip/ping`** — verified not a bug: Harvey rc's `searchpath()` (rc/simple.c:100-110) only bypasses `$path` for leading `/`, `#`, `./`, `../`; a slash-containing name like `ip/ping` is still path-searched, so `/bin/ip/ping` resolves via `path=(. /bin)` from rcmain. init/termrc/rcmain never chdir, so cwd is `/` and the `.` leg harmlessly misses first.
- **phase6.yml venue-pin self-match safety** — verified: the ERE source text `subset-(build|exec)\.sh` does not match itself, and no other line in the workflow contains the literal `subset-build.sh`/`subset-exec.sh` strings; the `^  tier-donor:` anchor can't match the step's own deeper-indented lines.
- **Sentinel digest single-sourcing** — the pinned md5/sha1 constants in build-9k10t1.sh match tier1-sweep.rc's asserts, and the byte recipe (`printf 'alpha bravo charlie\n__SUBSET_GREP_LINE__\n'`) is identical at all three generation sites (subset-exec.sh:316, mkimage.sh:179, build-9k10t1.sh:90).
- **flake.lock / .gitignore** — r9 pin bump and the k10t1.c/systab.c/.tier1-stage ignore rules are routine; nothing anomalous.

Two warnings and six info items below. No critical defects found.

## Warnings

### WR-01: gethunk libc-provisioning block's error handling is dead under `set -e`

**File:** `build/aarch64/gethunk-repro-6l.sh:276-279`
**Issue:** The new (this-phase) amd64 libc provisioning block runs the namespace harness as a bare command and then captures its status:

```sh
(cd "$HARVEY_ROOT/sys/src" && bash RUN_BUILD_IN_PRIVATE_NAMESPACE)
RC=$?
rm -f "$NSLIBC"
[ $RC -eq 0 ] || { echo "error: libc namespace provisioning failed (rc=$RC)" >&2; exit 2; }
```

The script runs under `set -eu` (line 67). A non-zero exit from the subshell aborts the script *before* `RC=$?` executes, so the named error message and the `rm -f "$NSLIBC"` cleanup are unreachable — the gate still reds (set -e propagates the status), but the failure is silent except for whatever the harness printed into the void, and the temp script litters. Every other harness invocation in these scripts gets the `set +e … RC=$? … set -e` wrapper (subset-build.sh:354/369, coverage-sweep.sh:387/415, build-9k10t1.sh:163-166, subset-exec.sh:618-621).
**Fix:**

```sh
set +e
(cd "$HARVEY_ROOT/sys/src" && bash RUN_BUILD_IN_PRIVATE_NAMESPACE)
RC=$?
set -e
rm -f "$NSLIBC"
[ $RC -eq 0 ] || { echo "error: libc namespace provisioning failed (rc=$RC)" >&2; exit 2; }
```

### WR-02: coverage-sweep.sh unconditionally overwrites the host's /bin/cpp outside the namespace

**File:** `build/aarch64/coverage-sweep.sh:399-403`
**Issue:** The new native-linux leg does `sudo cp "$HARVEY_LINUX_BIN/cpp" /bin/cpp` before entering the namespace — a permanent, unguarded mutation of the host: on usrmerge systems `/bin/cpp` is the distro's real cpp binary (or a symlink cp follows and overwrites the target of), and the copy is re-done on every sweep run with no backup and no restore. This breaks the namespace-isolation discipline the rest of the harness keeps (nsrun.sh:105-107 demonstrates the non-destructive pattern — it *bind-mounts* `$PLAN9/bin/rc` over `/bin/rc` inside the private mount namespace, leaving the host untouched). The sibling mutations elsewhere in this phase are only-if-missing (`[ -x /bin/rc ] || sudo cp …`), which is meaningfully safer than this unconditional overwrite. pcc execs `/bin/cpp` by absolute path *inside* the namespace build, so an in-namespace bind mount satisfies the same need.
**Fix:** Move the provisioning inside the namespace (extend nsrun.sh's root phase or the guest script with `[ -e /bin/cpp ] || touch /bin/cpp; mount --bind "$HARVEY_LINUX_BIN/cpp" /bin/cpp`), mirroring nsrun.sh's /bin/rc bind. If the host-side cp is kept intentionally, at minimum make it conditional (`[ -x /bin/cpp ] || …`) and note the host mutation in the script header's usage block.

## Info

### IN-01: mkimage.sh keeps a dead dd trailer extraction

**File:** `build/aarch64/mkimage.sh:270-275`
**Issue:** Line 270 computes `trailer` with `dd … | od …` and line 275 immediately recomputes it with the `od -j` form; the dd result is never used. The comment (271-274) explains *why* dd was abandoned (Harvey's dash-pair dd shadows POSIX dd on the devshell PATH), so the surviving dd line is pure dead code that also execs a known-incompatible binary on every bake (harmless — `2>/dev/null`, pipeline status is od's, and the value is overwritten — but confusing to readers).
**Fix:** Delete line 270 (`trailer=$(dd if="$IMAGE" bs=1 skip=$((sz - 28)) count=4 2>/dev/null | od -An -tx1 | tr -d ' \n')`).

### IN-02: subset-exec.sh defines the D-02 GREP idiom but barely uses it

**File:** `build/aarch64/subset-exec.sh:659, 699, 705, 775, 791, 796-798, 824-833`
**Issue:** The script resolves `GREP=$(command -v /usr/bin/grep || command -v grep)` at line 177 with the standard D-02 comment ("the greps below need -q/-x, which plan9 grep lacks"), then invokes bare `grep -qF/-q/-oE/-qiE` at a dozen sites — including the safety-critical final marker assertions (824-833). Practical risk is low: the devshell hostbin ships no grep (verified: .build/hostbin-*/bin contains awk/sed/dd/date/ls/rm/basename but not grep) and `$PLAN9/bin` is last per BUILD_ON_UNIX, so bare grep resolves to the distro/BSD grep on supported hosts. But on a host with plan9port's grep earlier in PATH (the exact class D-02 documents for sed/date/dd), Plan 9 grep's missing `-q` would turn the final gate assertions into spurious reds, and the srv9 startup poll (699) into a false "srv9 did not start" timeout.
**Fix:** Mechanical substitution of the bare `grep` invocations with `"$GREP"` (or drop the resolution line and the comment if the project judges grep unshadowable — but the inconsistent middle state is the worst option).

### IN-03: subset-exec.sh sentinel comment says "40 bytes"; the fixture is 41

**File:** `build/aarch64/subset-exec.sh:315`
**Issue:** The comment claims "(deterministic wc counts: 2 lines, 4 words, 40 bytes)". The actual bytes (`alpha bravo charlie\n` = 20, `__SUBSET_GREP_LINE__\n` = 21) total 41 — as tier1-sweep.rc:241-243's own comment correctly states ("its exact 41 bytes"). No assert keys on the byte count (the wc check asserts `-l` = 2 only), so this is comment-only drift, but it's the kind of detail a future fixture edit will trust.
**Fix:** s/40 bytes/41 bytes/.

### IN-04: subset-exec.sh rebake comment undercounts the tier-1 bootdir (66 vs 70)

**File:** `build/aarch64/subset-exec.sh:582-585`
**Issue:** The devroot-cap comment says "the full tier-1 bootdir now embeds 66 subset files (37 new h* + 4 fixed inputs joined the 25)". The actual bootdir block (subset-bootdir.txt, lines 379-448) carries 70 entries, and the literal assert list at line 575 also checks 70 (25 + 37 + 4 + 3 canary h7* + tiny.s) — the comment predates the D-11 canary additions. Harmless (Nbootfiles=128 covers either count; the donor pin gates on HEAD, not this file) but stale.
**Fix:** Update the comment to 70 (or better, stop counting in prose and cite the assert list as the source of truth).

### IN-05: gethunk fixed-bin staleness key watches only obj.c

**File:** `build/aarch64/gethunk-repro-6l.sh:229`
**Issue:** The fixed-pair rebuild test is `[ "$HARVEY_ROOT/sys/src/cmd/6l/obj.c" -nt "$SCRATCH/fixed-bin/6l" ]`. This phase also changed `sys/src/cmd/6l/asm.c` (the LP64 float fix) — an asm.c-only edit leaves a stale fixed-bin in place despite the header's "the gate never trusts hostbin staleness" claim. Impact on this gate's verdict is nil (the spike input has no float constants), but the next 6l change outside obj.c silently reuses yesterday's linker.
**Fix:** Key the rebuild on the newest source in the 6l tree, e.g. `[ -n "$(find "$HARVEY_ROOT/sys/src/cmd/6l" -name '*.[ch]' -newer "$SCRATCH/fixed-bin/6l" 2>/dev/null)" ]`, or just always rebuild (the pair build is seconds).

### IN-06: devroot.c carries a pre-existing debug `panic("whoops")` remnant

**File:** `sys/src/9k/port/devroot.c:154-157`
**Issue:** The `t < 0` guard in `rootgen` is an upstream debug leftover — `print("rootgen %#llux %d %d\n", …); panic("whoops")` — at flush-left, space-indented formatting that violates the file's tab style, in a file this phase otherwise touched. The condition itself is unreachable for well-formed qids (rootgen's Qdir/Qboot cases return early, and valid entries start at path ≥ 1 / ≥ Qboot+1), so it's dead defensive code with an unserious panic message in committed kernel source.
**Fix:** Either delete the block (the invariant holds by construction) or promote it to a real guard: tab-indent it and give the panic a diagnosable message, e.g. `panic("rootgen: negative index %d (qid %#llux)", t, c->qid.path);`.

---

_Reviewed: 2026-11-05T22:34:43Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
