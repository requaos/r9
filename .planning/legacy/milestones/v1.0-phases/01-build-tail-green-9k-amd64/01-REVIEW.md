---
phase: 01-build-tail-green-9k-amd64
reviewed: 2026-10-31T13:09:44Z
depth: standard
files_reviewed: 23
files_reviewed_list:
  - build/mkdirs
  - build/mm/boot-9k10cpu.mm
  - build/mm/boot-9k10cpu.sh
  - build/REPRODUCIBILITY.md
  - build/scripts/build
  - build/scripts/linux-build
  - dist/replica/plan9.proto
  - .gitignore
  - nix/build.sh
  - nix/toolchain.sh
  - nix/vm-build.sh
  - sys/src/9k/k10/mkfile
  - sys/src/9k/mk/portmkfile
  - sys/src/BUILD_SETUP_NAMESPACE
  - sys/src/cmd/6c/cgen.c
  - sys/src/cmd/6c/swt.c
  - sys/src/cmd/6l/span.c
  - sys/src/cmd/cc/cc.h
  - sys/src/cmd/jpg/mkfile
  - sys/src/cmd/jtagfs/mkfile
  - sys/src/cmd/mkfile
  - sys/src/cmd/spell/pcode.c
  - sys/src/nboot/efi/mkfile
findings:
  critical: 1
  warning: 4
  info: 5
  total: 10
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-10-31T13:09:44Z
**Depth:** standard
**Files Reviewed:** 23
**Status:** issues_found

## Summary

Reviewed the phase's diff (`589868a^..HEAD`) per file, in context, against Plan 9 C dialect rules, mk semantics (verified against the in-tree `sys/src/cmd/mk` and `sys/src/cmd/rc` sources where behavior mattered), and the KERNDATE/reproducibility design.

The headline C changes (LP64 host fixes in `6c/cgen.c`, `6c/swt.c`, `6l/span.c`, `cc/cc.h`) are **correct**: explicit `[-2^31, 2^31)` range checks restore exact Plan-9-host semantics on LP64 hosts, boundary values verified, and the `NVAR=BITS*32` fix matches `bits.c`'s `b[n/32]` / `1<<(n%32)` 32-bit-word indexing (verified). The mkfile aggregation question raised by jpg's `clean:V: nuke-headers` was traced through `mk`'s `addrule`/`dorecipe` and is safe (mkmany's recipe is retained; `nuke-headers` becomes a prerequisite). `checkall`'s `exit 0` is valid rc (`truestatus()` treats all-'0' status as true; verified in `rc/exec.c:1035`, `rc/plan9.c:595`, `rc/unix.c:453`).

The main defect is the new minimega smoke harness: an executable script with no shebang, bash-only features, and a documented direct-execution usage that fails on dash-based Linux. Around it: a silent-fallback flaw in the KERNDATE override, a self-blocking preflight gate, an unsynchronized serial-capture race, and unchecked `cd`s in the new `checkall` target.

## Critical Issues

### CR-01: boot-9k10cpu.sh is executable with no shebang and requires bash — documented direct invocation breaks on dash-sh systems

**File:** `build/mm/boot-9k10cpu.sh:1` (git mode 100755)
**Issue:** Line 1 is a plain comment (`# boot-9k10cpu.sh — ...`), not a shebang. The header documents `sudo build/mm/boot-9k10cpu.sh` as the primary KVM-host invocation. Direct `execve` of a shebang-less file returns ENOEXEC; sudo's fallback (where present) re-runs it via `/bin/sh`. On Ubuntu/Debian `/bin/sh` is dash, which fails immediately at line 46 (`set -euo pipefail` — dash has no `pipefail`) and would also fail on `$SECONDS` (line 230; not POSIX, unset under `set -u`). On systems whose sudo does not emulate the shell fallback, the invocation fails outright with "Exec format error". Either way the documented smoke test cannot start. The companion scripts (`nix/*.sh`) get away with no shebang because they are non-executable (100644) and flake-wrapped via `writeShellApplication`; this script is explicitly standalone (its own comment says so) and executable.
**Fix:**
```sh
#!/usr/bin/env bash
# boot-9k10cpu.sh — BUILD-05 smoke harness: ...
```
(also acceptable: change the usage line to `sudo bash build/mm/boot-9k10cpu.sh`, but the shebang is the robust fix since the file is already mode +x).

## Warnings

### WR-01: User-supplied invalid KERNDATE is silently replaced by wall-clock time

**File:** `nix/build.sh:17-28`, `nix/vm-build.sh:23-32`
**Issue:** When `KERNDATE` is set but non-numeric (typo, stray whitespace, `KERNDATE=abc`), the regex-validation blocks silently overwrite it with `date -n`/`date +%s` — the non-reproducible wall-clock path. The user believes they pinned a timestamp (the documented override in `build/REPRODUCIBILITY.md:44-48`) and gets a silently non-reproducible kernel. The fallback chain is appropriate only for the *unset* case (no git metadata). Additionally, the ~10-line derivation/validation block is now duplicated verbatim in both files; a future fix must be applied twice.
**Fix:**
```bash
if [ -n "${KERNDATE:-}" ] && ! [[ "$KERNDATE" =~ ^[0-9]+$ ]]; then
	echo "error: KERNDATE='$KERNDATE' is not an epoch (digits only)" >&2
	exit 1
fi
```
then keep the existing unset-case derivation. Consider factoring the block into a shared `nix/kerndate.sh` sourced by both.

### WR-02: Userland preflight sentinel can deadlock the only documented repair path

**File:** `nix/build.sh:58-66` (gate) and `nix/build.sh:85-89` (arming)
**Issue:** Once `amd64/.userland-ok` is armed, a missing `amd64/bin/{ip/ipconfig,auth/factotum,fossil/fossil,venti/venti}` makes `nix run .#build` exit 1 — but the build itself is what regenerates those binaries, and the error message ("preflight: $f missing — cmd chain incomplete") names no escape hatch. After a partial manual cleanup, a corrupted checkout, or an interrupted earlier run, the standard command is permanently blocked until the user discovers and removes the undocumented sentinel (or runs `RUN_BUILD_IN_PRIVATE_NAMESPACE` by hand). The header comment claims a repairing run is "never blocked", but that only holds while the sentinel is absent — the armed state is exactly the state a repair run starts from.
**Fix:** Print the remedy in the failure path, e.g.
```bash
echo "preflight: $f missing — cmd chain incomplete" >&2
echo "  to force a full rebuild (regenerates these binaries): rm $HARVEY/amd64/.userland-ok" >&2
```
Better: add an explicit bypass (`MM=`, e.g. `SKIP_PREFLIGHT=1`) or flip the logic so the gate is skipped when the sentinel's own timestamp predates the last failed build.

### WR-03: Serial capture attach is unsynchronized — lost race silently drops the boot banner

**File:** `build/mm/boot-9k10cpu.sh:221-227`
**Issue:** The script backgrounds socat (`UNIX-CONNECT` → `CREATE`) and immediately issues `mm read start.mm`. Nothing confirms socat has actually connected to `serial0` before qemu resumes; the script's own header (lines 26-31) explains that chardev output with no client connected is discarded and that the banner prints within the first milliseconds of `vm start`. Normally connect(µs) beats the minimega/QMP round-trip(ms), but under load, on TCG hosts, or with a cold page cache the fork/exec/connect can lose — and the failure mode is exactly the one the script says must not happen silently: a full-budget wait followed by "triage manually". The script already treats this ordering as correctness-critical; it just doesn't enforce the second half of it.
**Fix:** After backgrounding socat, verify it is alive and has connected before starting the VM, e.g. poll briefly for the peer:
```bash
"$SOCAT" -u "UNIX-CONNECT:$SERIAL_SOCK" "CREATE:$MM_LOG" &
SOCAT_PID=$!
kill -0 "$SOCAT_PID" 2>/dev/null || err "socat died attaching to $SERIAL_SOCK"
# brief settle so the unix connect completes before qemu resumes
sleep 0.2
```
(a `kill -0` liveness check plus a short settle is cheap; a stricter check would parse socat's `-d -d` diagnostics for a successful connect).

### WR-04: `checkall` does not check its `cd`s — a failed cd mis-attributes or mis-builds everything after it

**File:** `sys/src/cmd/mkfile:46-60`
**Issue:** In the new `checkall` recipe, `cd $i` and `cd ..` are unchecked. If one `cd $i` fails (dir deleted mid-run, permissions, stale `$DIRS` entry from the `ls -l` glob), `mk $MKFLAGS all` then runs in the *parent* directory (recursively re-entering `sys/src/cmd`'s own `all`/`dirs`), `cd ..` walks a second level up, and every subsequent iteration builds and reports against the wrong subtree. For a diagnostic whose entire purpose (D-03) is accurate per-directory failure attribution, a single `cd` failure silently corrupts the whole report. The same fragility pre-exists in `dirs`, but `dirs` aborts loudly on the first failure while `checkall` is specifically designed to keep going.
**Fix:**
```rc
for(i in cc $DIRS) {
	if(! cd $i) {
		echo 'checkall: cannot cd '^$i
		fail=($fail $i)
		cd $build_dir   # or: continue-equivalent via next iteration from a fixed base
	}
	...
}
```
At minimum, record the cd failure in `fail` and re-establish the base directory each iteration (`cd /sys/src/cmd` absolute, rather than relative `cd ..`).

## Info

### IN-01: .gitignore misses artifacts newly produced by the host-generator mkfile pattern

**File:** `.gitignore:57-68`, interacting with `sys/src/cmd/jtagfs/mkfile:44-52` and `sys/src/cmd/jpg/mkfile:60-78`
**Issue:** The phase's own gitignore pass (fef19a8a5) covers kernels, `.efi`, `.userland-ok`, and generated `$CONF.c`, but the green build also leaves behind: `sys/src/cmd/jtagfs/rtab.c` (generated, untracked, unignored), `sys/src/cmd/jtagfs/*.genrtab`, `sys/src/cmd/jpg/*.rgbycc`, `sys/src/cmd/jpg/*.rgbrgbv` (host binaries named e.g. `unix.rgbycc` — no gitignore pattern matches them), plus their `*.o` objects (those *are* covered by line 43). The amd64 cmd pass ends without a clean, so all of these persist and show up as untracked noise in `git status`. Verified: `git check-ignore` matches none of them; `git ls-files` shows `rtab.c` untracked.
**Fix:** Add `*.rgbycc`, `*.rgbrgbv`, `*.genrtab`, and `/sys/src/cmd/jtagfs/rtab.c` to .gitignore (or add a trailing `mk clean` to the cmd pass — but ignoring matches the phase's existing approach).

### IN-02: build/mkdirs declares `#!/bin/sh` but is bash-only

**File:** `build/mkdirs:1`
**Issue:** The body uses bash arrays (`lst=( \ ... )`, `"${lst[@]}"`), which dash/posix-sh reject with a syntax error. The in-tree invocation is explicit `bash "$HARVEY/build/mkdirs"` (nix/build.sh:48) and the file is mode 0644, so this is latent — but the shebang lies to anyone who chmods and runs it directly. Same class of defect as CR-01, minus the executable bit.
**Fix:** Change the shebang to `#!/usr/bin/env bash` (or rewrite as a POSIX here-doc loop).

### IN-03: KERNDATE policy is duplicated across four sites; mkfile fragment mishandles set-but-empty KERNDATE

**File:** `nix/build.sh:17-28`, `nix/vm-build.sh:23-32`, `sys/src/9k/k10/mkfile:20`, `sys/src/9k/mk/portmkfile:103`
**Issue:** (a) The same policy is expressed in four places (two host scripts, two mkfile rc fragments) — `REPRODUCIBILITY.md` documents all four, so any change needs five synchronized edits. (b) The mkfile fragment `if(~ $#KERNDATE 0) KERNDATE=`{date -n}` tests only *unset*; a set-but-empty `KERNDATE=` yields `-DKERNDATE=` and the generated `$CONF.c` then fails with an opaque C syntax error at `ulong kerndate = ;` (sys/src/9k/mk/parse:461). The nix wrappers regex-validate so they never emit this, but interactive `mk` users can hit it.
**Fix:** For (b), extend the mkfile guard: `if(~ $#KERNDATE 0 || ~ $KERNDATE '') KERNDATE=`{date -n}`. For (a), at minimum cross-reference the sites in a comment (REPRODUCIBILITY.md already lists them; the files themselves don't).

### IN-04: Unescaped path interpolation in the .mm sed render

**File:** `build/mm/boot-9k10cpu.sh:142-144`
**Issue:** `sed -e "s|^vm config kernel KERNEL_ELF\$|vm config kernel $KERNEL_ELF|"` interpolates a checkout path into a sed replacement without escaping `&`, `\`, or the `|` delimiter. A checkout under a path containing those characters silently produces a corrupt `vm config kernel` line; the sanity greps (lines 152-153) check only the launch/start lines, so the corruption surfaces much later as a 60-second serial-socket timeout. Pathological for typical paths, but the hardening is one line.
**Fix:** Validate early: `case "$KERNEL_ELF" in *[|'\'\&]* ) err "HARVEY path contains sed metacharacters";; esac` or escape with `printf '%s' "$KERNEL_ELF" | sed 's/[&|\]/\\&/g'`.

### IN-05: `build_aarch64` is dead scaffolding that fails confusingly if ever reached

**File:** `build/scripts/build:76-80`, `build/scripts/linux-build:63-67`
**Issue:** The renamed `build_aarch64` references `objtype=aarch64`, but no `aarch64/mkfile` exists at the repo root and there are no `sys/src/cmd/7?` tools, so `for(i in /sys/src/cmd/7?)` iterates the literal unmatched glob and `cd /sys/src/cmd/7?` fails under `rc -e` with a misleading path error. Unreachable in the Phase 1 green path (`arch=(amd64)`), so this is deferred-scope debt — but one guard line would make the failure honest until the aarch64 port lands.
**Fix:** Add at the top of each function: `if(! test -d /sys/src/cmd/7c) { echo 'aarch64 toolchain not in tree yet'; exit }` (or leave a comment saying so — currently nothing in the function hints that it cannot work today).

---

## Verified-sound items (traced, not findings)

- `sys/src/cmd/6c/cgen.c:1816` `immconst` — range check `[-(vlong)0x80000000, (vlong)0x7fffffff]` is exactly the sign-extended imm32 domain; all five call sites (cgen.c:113, 417, 527, 1333; txt.c:581) use it only for imm32-encodable operands. Restores Plan-9-host semantics; `typechlpv` = `typechlp`+TVLONG+TUVLONG (txt.c:32-34), so the check applies precisely to 64-bit constants.
- `sys/src/cmd/6c/swt.c:381` — T_64 test and the subsequent low-32/high-32 `Bputc` emission are correct on LP64 hosts including negative offsets (arithmetic `>>32` sign-extension then byte truncation yields the correct high word).
- `sys/src/cmd/6l/span.c:542` — Ys32/Yi32/Yi64 classification now matches Plan 9 host behavior; `(v>>32)==0` Yi32 path and Yi64 fallthrough unchanged and correct.
- `sys/src/cmd/cc/cc.h:43` — `NVAR=BITS*32` matches `bits.c` indexing (`c.b[n/32] = 1L<<(n%32)`); old LP64 value (512) would have indexed past `b[8]` — the fix removes a genuine stack smash.
- `sys/src/cmd/spell/pcode.c` — the added `int` return type fixes the real modern-compiler error (implicit int); the file is only host-compiled via `9c` against plan9port headers in this path (9c forces `-I$PLAN9/include`, per sys/src/cmd/unix/9cc/BUILD comment), so Harvey libc.h's non-const `qsort` prototype is not in play here.
- `sys/src/cmd/mkfile` `checkall` — `exit 0` is a *success* status in rc semantics (`truestatus()`: all-'0'/`|` chars = true; rc/exec.c:1035, Exit at rc/plan9.c:595 / rc/unix.c:453). Brace-group vs `@{}` accumulator reasoning is correct (rc forks for `@{}`).
- `sys/src/cmd/jpg/mkfile` — adding `clean:V: nuke-headers` after `</sys/src/cmd/mkmany` does NOT clobber mkmany's clean recipe: mk aggregates rules with different tails (rule.c `addrule`), and `dorecipe` (recipe.c:16-21) picks the last arc *with a recipe* — jpg's rule has none, so mkmany's `rm` recipe runs with `nuke-headers` as an added prerequisite. Correct.
- `nix/toolchain.sh` `_WRAP_PATH` — captured before both the hostbin prepend and the SHIMDIR prepend; the vendored 9cc build correctly sees GNU `ar` (plan9port ships no `ar`), fixing the repeat-run iar-shadowing bug.
- `nix/vm-build.sh:58` — `KERNDATE` is digits-validated (lines 27-32) before being interpolated into the ssh command string; no injection path. Host-side `$GUEST_DIR` expansion inside the double-quoted remote command is correct.
- `sys/src/BUILD_SETUP_NAMESPACE:75` — `chown 0:0 $NSLIB` correctly satisfies `fs.protected_symlinks` (dir-owner == symlink-owner for the root-owned loader symlink preserved by `cp -a`); KERNDATE survives the namespace via `export > exports.$$` + `. $2` + non-login `su` env preservation.
- arm64→aarch64 quarantine — no dangling references remain (`grep` confirms no `build_arm64`/`arm64` mount left; surviving `arm64` strings are uname normalization and the Ubuntu image URL, both correct). Neither `arm64/` nor `aarch64/` existed at repo root before or after, so removing the mount/mkdirs entries is a strict fix.
- `build/mm/boot-9k10cpu.*` — main.c line references cited in the comments verified (`i8250console("0")` at main.c:182, `print("\nPlan 9\n")` at main.c:191; multiboot header at l32p.s:24-28).
- Reproducibility design — traced `9k/mk/mkrootall`/`mkroot`: the embedded root is `strip` + `data2s` of file *bytes* (no mtimes), and devroot synthesizes Dir metadata from `kerndate` (port/dev.c:37), so the doc's kernel-only reproducibility claim is coherent even with non-reproducible userland archive metadata.

---

_Reviewed: 2026-10-31T13:09:44Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
