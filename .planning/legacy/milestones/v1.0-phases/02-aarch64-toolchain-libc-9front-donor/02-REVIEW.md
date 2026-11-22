---
phase: 02-aarch64-toolchain-libc-9front-donor
reviewed: 2026-11-01T16:25:12Z
depth: standard
files_reviewed: 125
files_reviewed_list:
  - .gitignore
  - aarch64/ABI.md
  - aarch64/include/u.h
  - aarch64/include/ureg.h
  - aarch64/mkfile
  - build/aarch64/README.md
  - build/aarch64/boot-9qemu.sh
  - build/aarch64/donor-ns-build.sh
  - build/aarch64/donor-vm-build.sh
  - build/aarch64/drive9.py
  - build/aarch64/gate-evidence.log
  - build/aarch64/gdbpoke9.py
  - build/aarch64/hello.7.out
  - build/aarch64/hello.c
  - build/aarch64/mkfs9.py
  - build/aarch64/nsrun.sh
  - build/aarch64/pre-gate-build.sh
  - build/aarch64/run-gate.sh
  - build/aarch64/srv9.py
  - build/aarch64/subset-build.sh
  - build/aarch64/subset-exec.sh
  - build/aarch64/vm-run.sh
  - build/mkdirs
  - build/scripts/linux-build
  - nix/build.sh
  - nix/toolchain.sh
  - nix/vm-build.sh
  - sys/include/mach.h
  - sys/src/BUILDTOOLCHAIN
  - sys/src/BUILD_SETUP_NAMESPACE
  - sys/src/ape/lib/ap/mkfile
  - sys/src/ape/lib/mp/mkfile
  - sys/src/ape/lib/sec/mkfile
  - sys/src/cmd/7a/BUILD
  - sys/src/cmd/7a/README
  - sys/src/cmd/7a/a.h
  - sys/src/cmd/7a/a.y
  - sys/src/cmd/7a/lex.c
  - sys/src/cmd/7a/mkfile
  - sys/src/cmd/7c/7.out.h
  - sys/src/cmd/7c/BUILD
  - sys/src/cmd/7c/README
  - sys/src/cmd/7c/cgen.c
  - sys/src/cmd/7c/enam.c
  - sys/src/cmd/7c/gc.h
  - sys/src/cmd/7c/list.c
  - sys/src/cmd/7c/machcap.c
  - sys/src/cmd/7c/math.h
  - sys/src/cmd/7c/mkfile
  - sys/src/cmd/7c/mul.c
  - sys/src/cmd/7c/peep.c
  - sys/src/cmd/7c/reg.c
  - sys/src/cmd/7c/sgen.c
  - sys/src/cmd/7c/swt.c
  - sys/src/cmd/7c/txt.c
  - sys/src/cmd/7l/BUILD
  - sys/src/cmd/7l/README
  - sys/src/cmd/7l/asm.c
  - sys/src/cmd/7l/asmout.c
  - sys/src/cmd/7l/bits.c
  - sys/src/cmd/7l/compat.c
  - sys/src/cmd/7l/dyn.c
  - sys/src/cmd/7l/dynstub.c
  - sys/src/cmd/7l/l.h
  - sys/src/cmd/7l/list.c
  - sys/src/cmd/7l/mkcname
  - sys/src/cmd/7l/mkfile
  - sys/src/cmd/7l/mod.c
  - sys/src/cmd/7l/noop.c
  - sys/src/cmd/7l/obj.c
  - sys/src/cmd/7l/optab.c
  - sys/src/cmd/7l/pass.c
  - sys/src/cmd/7l/span.c
  - sys/src/cmd/7l/unixcompat.c
  - sys/src/cmd/cc/compat.h
  - sys/src/cmd/cc/mkfile
  - sys/src/cmd/cc/pswt.c
  - sys/src/cmd/nm.BUILD
  - sys/src/cmd/pcc.c
  - sys/src/cmd/unix/9cc/BUILD
  - sys/src/cmd/unix/9cc/README
  - sys/src/cmd/unix/9cc/cmd/7c/7.out.h
  - sys/src/cmd/unix/9cc/libmach/7.c
  - sys/src/cmd/unix/9cc/libmach/7obj.c
  - sys/src/cmd/unix/9cc/libmach/executable.c
  - sys/src/cmd/unix/9cc/libmach/obj.c
  - sys/src/cmd/unix/9cc/libmach/ureg7.h
  - sys/src/libc/9sys/callmain.c
  - sys/src/libc/9sys/mkfile
  - sys/src/libc/9syscall/mkfile
  - sys/src/libc/aarch64/README
  - sys/src/libc/aarch64/_seek.c
  - sys/src/libc/aarch64/argv0.s
  - sys/src/libc/aarch64/atom.s
  - sys/src/libc/aarch64/atomic.s
  - sys/src/libc/aarch64/atomic64.s
  - sys/src/libc/aarch64/cycles.s
  - sys/src/libc/aarch64/fabs.s
  - sys/src/libc/aarch64/floor.s
  - sys/src/libc/aarch64/getcallerpc.s
  - sys/src/libc/aarch64/getfcr.s
  - sys/src/libc/aarch64/main9.s
  - sys/src/libc/aarch64/main9p.s
  - sys/src/libc/aarch64/memmove.s
  - sys/src/libc/aarch64/memset.s
  - sys/src/libc/aarch64/mkfile
  - sys/src/libc/aarch64/notejmp.c
  - sys/src/libc/aarch64/setjmp.s
  - sys/src/libc/aarch64/sqrt.s
  - sys/src/libc/aarch64/tas.s
  - sys/src/libmach/7.c
  - sys/src/libmach/7db.c
  - sys/src/libmach/7obj.c
  - sys/src/libmach/README
  - sys/src/libmach/executable.c
  - sys/src/libmach/mkfile
  - sys/src/libmach/obj.c
  - sys/src/libmach/setmach.c
  - sys/src/libmp/aarch64/mkfile
  - sys/src/libmp/port/mkfile
  - sys/src/libmp/port/mpvecdigmuladd.c
  - sys/src/libmp/port/mpvecdigmulsub.c
  - sys/src/libsec/aarch64/mkfile
  - sys/src/libthread/aarch64.c
  - sys/src/mkfile.proto
findings:
  critical: 1
  warning: 10
  info: 13
  total: 24
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-11-01T16:25:12Z
**Depth:** standard
**Files Reviewed:** 125
**Status:** issues_found

## Summary

Reviewed all 125 phase files with donor-verbatim code (9front 7a/7c/7l, libc/aarch64 asm, libmach 7-format, unix/9cc arm64 additions) checked at the Harvey-modification seams and Harvey-authored files (build scripts, harness Python, mkfiles, ABI.md, shims) read in full.

The Harvey-local work is largely sound and verified in detail:

- **Donor fidelity confirmed by direct diff against the pinned checkout** (`350266104f1fed81c78c3d05fa44f523bec0ae4d`): 7a differs only in `a.h` (BUFSIZ, hunk EXTERNs, 2-arg macexpand decl), `lex.c` (thestring, zaddr `int l`), `mkfile` (UPDATE block); 7c only in the OROL excision (3 sites), `gc.h` const swcmp, `swt.c` zaddr `int l`, `txt.c` thestring, `math.h` added, `mkfile` rewrite in 6c idiom; 7l only in `l.h` MAXIO, `obj.c` thestring, `mkfile`; libc/aarch64 only in mkfile's objtype line, the memset/memmove MOVWU reconcile, dropped lock.c, README. `callmain.c`, `libthread/aarch64.c`, `aarch64/include/{u.h,ureg.h}` (sha256-verified), `aarch64/mkfile`, in-tree libmach `7db.c`/`7obj.c` all byte-identical to the pin.
- **The LP64 fixes are correct**: `pswt.c` `(vlong)(int)c->val` sign-extends 32-bit case constants host-independently at both switch paths (the donor-kernel pool panic root cause); the 7a/7c zaddr `int l` restores the `(vlong)l != a->offset` truncation check; the 9cc `commonllp64` u32-view decode matches the 7l on-disk layout (verified against `7l/asm.c:162-177`: 8×u32 BE + 8-byte hdrmagic = 40 bytes, matching the `(8*4)+8` hsize literal).
- **`unixcompat.c`'s gethunk merge fix** is correct: conditional accumulate on contiguous sbrk, replace otherwise; `malloc`'s `while(nhunk < n)` terminates. `dynstub.c` matches `l.h` signatures and `asmdyn` is confirmed reachable only under `if(dlm)` (`asm.c:143-149`).
- **The ABI freeze block is donor-verbatim** (`case aarch64` == donor `case arm64` modulo label), ABI.md's SP+8/R0/SVC contract reconciles with the generated stubs and was execution-proven; NSEC=53 zero-arg register-return verified consistent with 9front's `sys_nsec` 64-bit path.
- The committed `hello.7.out` matches the evidence-log sha256 (`1c89e099…`) and carries R_MAGIC `00008c47`; .gitignore covers all new build outputs.

The findings below concentrate on the harness scripts: one stale post-gate assertion that now breaks the documented TC-03 gate, silent-failure paths in the namespace/build drivers, a deadline mismatch that can false-fail slow TCG boots, and an echo-match weakness in the serial driver's shell proof.

## Critical Issues

### CR-01: run-gate.sh's "scaffolding-free" assertion is stale — the TC-03 gate now always fails (after appending PASS evidence)

**File:** `build/aarch64/run-gate.sh:727-729`
**Issue:** The final check `if [ -e "$HARVEY/aarch64" ]; then err "aarch64/ leaked into the committed tree"` was correct when written (02-05, pre-gate), but plan 02-06 legitimately committed `aarch64/` (`aarch64/mkfile`, `include/{u.h,ureg.h}`, `ABI.md`; commit a7945d8b5) — and run-gate.sh was never revisited (last touched in 02-05's 62351f81f). Every run today executes the full gate, **appends a "gate: PASS" block to the committed `gate-evidence.log` (line 724), and then exits 1** with a misleading "leaked" error. The documented contract ("Idempotent: consecutive runs append consecutive PASS blocks" — build/aarch64/README.md:36) is broken, and the evidence log accumulates PASS blocks from runs the script reports as failed — contradictory state in a load-bearing proof artifact.
**Fix:** Delete the assertion (its protective purpose ended when 02-06 committed the arch dir). If ordering evidence is still wanted, replace it with a git-log ordering check (gate-evidence commit precedes the first aarch64/ commit), not a filesystem-existence test:

```sh
# delete lines 726-729 entirely, or:
if git -C "$HARVEY" log --oneline --diff-filter=A -- aarch64/mkfile | grep -q .; then
	: # aarch64/ is committed scaffolding (plan 02-06) — expected post-gate
fi
```

## Warnings

### WR-01: pre-gate-build.sh carries the same stale assertion and is obsolete-but-undocumented

**File:** `build/aarch64/pre-gate-build.sh:138-141`
**Issue:** Same class as CR-01: post-02-06 the committed tree always has `aarch64/`, so the script always fails at its final step with "aarch64/ leaked into the committed tree" — a false error for what is now the committed wiring. The script's function (guest-scratch injection) was deliberately superseded by 02-06's committed wiring, yet it remains committed and is documented as a live script in `build/aarch64/README.md:32` with no obsolescence note. Additionally the script declares `#!/bin/sh` but uses the bashism `printf '%q'` (line 120) — works on macOS `/bin/sh` (bash POSIX mode), breaks under dash on the Linux hosts AGENTS.md lists as supported.
**Fix:** Either delete the script (and its README table row), or guard it explicitly as historical:

```sh
if [ -e "$HARVEY/aarch64/mkfile" ]; then
	echo "pre-gate-build.sh is obsolete post-02-06 (aarch64/ is committed); use objtype=aarch64 mk in the namespace" >&2
	exit 2
fi
```

### WR-02: nsrun.sh root mode runs the entire bind/staging sequence without `set -e`

**File:** `build/aarch64/nsrun.sh:39-125`
**Issue:** `set -u` is global (line 23) but `set -e` is only set inside the `--wrap` branch (line 28). In root mode (the namespace setup), every `mount --bind` (lines 50-125) and the merged-/lib staging (lines 98-111: `cp -a`, `chown`, `chmod`, `mount`) is unchecked — a missing `$HARVEY/arm` or a failed loader-staging copy silently yields a partial namespace, and the build then fails later with misleading missing-include errors. The command's own exit status *is* propagated (lines 138-141), so only setup is fragile. BUILD_SETUP_NAMESPACE shares the style, but nsrun.sh is the phase's reused runner for plans 02-03..02-07.
**Fix:** Add `set -e` after the root-mode argument parsing (line 42), and re-`set +e` only around the deliberate `su` capture (as is already done at line 138).

### WR-03: gdbpoke9.py's 120s default deadline is far below the 600–1500s boot budgets — false gate failures on slow TCG boots

**File:** `build/aarch64/gdbpoke9.py:69` (with callers `boot-9qemu.sh:207`, `run-gate.sh:653`, `subset-exec.sh:720`)
**Issue:** The wake poke watches for "bootargs is" with `deadline = time.time() + 120`; no caller passes `--deadline`, while the drivers' BOOT_BUDGET is 600s (1500s for subset-exec). On a loaded host where the TCG boot reaches the bootargs prompt after >120s, the poke exits 1 ("watch string never appeared"), the GIC redistributor stays asleep, console input stays dead, and the boot parks until the driver budget expires — a spurious gate failure. The failure *is* loud (the callers' `grep 'poked'` guard names the right culprit), but the gate still fails for purely harness-timing reasons, and the header explicitly says healthy-but-slow boots should just need a bigger budget.
**Fix:** Pass a deadline proportional to the boot budget from each caller, e.g. in boot-9qemu.sh:

```sh
"$PYTHON" "$HARVEY/build/aarch64/gdbpoke9.py" --sock "$TMPD/gdb.sock" \
	--watch-file "$QEMU_LOG" --watch-str "bootargs is" \
	--deadline "$BOOT_BUDGET" ...
```

### WR-04: drive9.py's "stronger" prompt proof is satisfied by console echo alone; SHELL_RE is one 5-byte drop away from a false positive

**File:** `build/aarch64/drive9.py:56-57, 206-215` (also the embedded forks `run-gate.sh:338,504-515` and `subset-exec.sh:366,577-588`)
**Issue:** Two compounding weaknesses in the shell-liveness proof:
1. `PROMPT_RE = re.compile(rb"term% ")` is an unanchored substring: the echoed assignment line `prompt='term% '` contains `term% `, so step 5's search succeeds even if rc never evaluated anything — the header's claim (lines 34-39, "a stronger proof … rc only prints the new prompt because it evaluated our assignment") is not delivered by the regex.
2. `SHELL_RE = (?m)^__GSD_SHELL_OK__\r?$` is echo-safe only because the echoed command line starts with `echo ` and fails the `^` anchor. But this harness family itself documents (02-05 deviation) that the console input path drops burst-leading bytes while kbdfs is busy; a probe that loses exactly the 5 bytes `echo ` and echoes the rest at a line boundary produces a line that matches SHELL_RE with no shell running. The gatedrive9/subsetdrive9 forks correctly made the *command* steps echo-safe (their output markers like `hello aarch64 harvey` cannot be echo-produced), but the liveness probe shares the hole in all three copies.
The boot assertion greps in boot-9qemu.sh (lines 244-248) inherit this: they grep the same capture for `term% `.
**Fix:** Make the prompt proof order-dependent and anchor it: search for `(?m)^term% $` only in the buffer region *after* the probe-output marker line (`search_from` with the recorded base), and/or make the shell probe produce transformed output (e.g. send `echo __GSD_SHELL_OK__ | tr O Q`… or simply require two distinct markers). At minimum, correct the header comment so reviewers don't over-trust the proof.

### WR-05: 9cc/BUILD has no `set -e` — mid-build failures can exit 0 against stale archives

**File:** `sys/src/cmd/unix/9cc/BUILD:39-47`
**Issue:** The per-library lines are `&&`-chained subshells, but without `set -e` a failing subshell doesn't abort the script; the exit status is the last command's (the iar build). If libmach's compile fails while a stale `$OBJDIR/libmach.a` from a previous run exists, the iar link succeeds against the stale archive and BUILD exits 0 — exactly the silent-staleness class this phase fought elsewhere (02-04 donor-object sweep). Every other BUILD added in this phase (`7a/7c/7l/BUILD`, `nm.BUILD`) sets `-e`.
**Fix:** Add `set -e` after the OSTYPE case (line 13). The `&&` chains already make each step atomic; `set -e` makes the script abort on the first failure.

### WR-06: 9cc README's executable.c provenance omits the LP64 commonllp64 rewrite

**File:** `sys/src/cmd/unix/9cc/README` (libmach arm64 bullet)
**Issue:** The README records "libmach/executable.c — extern marm64 + donor-verbatim R_MAGIC exectab entry + FARM64 commonboot case". But the R_MAGIC exectab entry is *not* donor-verbatim (hsize is the literal `(8*4)+8` with an explanatory comment, replacing `sizeof(Exec)+8`), and the file additionally carries the Harvey `commonllp64` u32-view LP64 rewrite (executable.c:401-446) — the only functional Harvey changes in the file, and the very thing this README exists to record (per-dir local-modification lists are the D-10 provenance mechanism). A future importer diffing against upstream will find an undocumented divergence.
**Fix:** Extend the bullet: "...R_MAGIC exectab entry with on-disk `(8*4)+8` hsize literal + FARM64 commonboot case + commonllp64 LP64 decode fix (u32 view; `long` is 8 bytes on LP64 hosts, so the donor's `hswal(sizeof(Exec)/sizeof(long))` scrambles the header — 02-03)".

### WR-07: donor-vm-build.sh's stale-object sweep deletes two tracked donor files

**File:** `build/aarch64/donor-vm-build.sh:130`
**Issue:** `find "$GUEST_DONOR/sys/src" -name '*.7' -delete` also matches the donor's tracked awk test fixtures `sys/src/cmd/awk/test/p.7` and `expected.p.7` (verified against the pinned checkout). The guest donor copy silently diverges from the pin between the sweep and step 5's rsync; only the rsync's no-`--delete` restore (line 135) masks it — and a future reordering (sweep after rsync) would leave them deleted, breaking awk's `mk test` in the guest. The `7.*` arm of the sweep correctly excludes tracked `7c/7.out.h` and `libmach/7.c`; the `*.7` arm has no such exclusions.
**Fix:** Exclude the fixtures explicitly (or scope to build outputs):

```sh
find "$GUEST_DONOR/sys/src" -name '*.7' \
	! -path '*/cmd/awk/test/*' -delete
```

### WR-08: vm-build.sh reports success when the guest build fails

**File:** `nix/vm-build.sh:56-65`
**Issue:** No `set -e` and no status check on the guest-build ssh (line 58): if the guest build fails, the script still rsyncs artifacts back and prints "harvey VM build complete", exiting 0. The phase documented "rc=0 is never trusted" for `nix run .#build` (deferred item 12); vm-build.sh's own tail is the same defect one level up, and it now also masks aarch64 subset failures in the dual-arch build. (Pre-existing script, but the 02-07 edit extended its artifact scope without fixing the propagation.)
**Fix:**

```sh
ssh -F "$SSH_CFG" "$GUEST" "KERNDATE=$KERNDATE bash -lc 'set -e; cd \"$GUEST_DIR\" && ...'" || {
	echo "error: guest build failed (rc=$?)" >&2; exit 1;
}
```

### WR-09: nix/build.sh lacks `set -e`; sudo-cp installs and the namespace build both fail softly

**File:** `nix/build.sh:86-105`
**Issue:** The script never sets `-e`. The `sudo cp` installs into /bin (lines 88-95) fail silently on a host without sudo rights, surfacing later as a confusing `pcc: exec failed`. Worse, the namespace build's failure (line 97) is masked by the preflight-arm tail (lines 101-105): when the four sentinel binaries are absent (i.e., the build failed), the loop hits `|| exit 0` — the script exits 0 on failure. This is the deferred-item-12 class, but this script's own status plumbing is part of that item and the 02-07 nm-shim edit touched this file without addressing it.
**Fix:** Add `set -e` near the top (the KERNDATE cascade already guards its own fallbacks with explicit `|| KERNDATE=` and regex checks, so it is `-e`-safe), and capture the namespace build's status explicitly before the preflight-arm tail:

```sh
(cd "$HARVEY/sys/src" && bash RUN_BUILD_IN_PRIVATE_NAMESPACE); RC=$?
[ $RC -eq 0 ] || { echo "error: namespace build failed (rc=$RC)" >&2; exit $RC; }
```

### WR-10: build/mkdirs declares `#!/bin/sh` but is bash (arrays)

**File:** `build/mkdirs:1,6,288-290`
**Issue:** The body uses `lst=( ... )` and `"${lst[@]}"` — bash arrays in a file whose shebang is `#!/bin/sh`. Under dash (Debian/Ubuntu `/bin/sh`), `sh build/mkdirs` fails with a syntax error. All in-tree callers happen to invoke `bash build/mkdirs` explicitly (nix/build.sh:64, subset-build.sh's guest script line 109), so the defect is latent — but the shebang invites the broken invocation, and 02-06 extended this file's aarch64 block without noticing.
**Fix:** Either change the shebang to `#!/usr/bin/env bash`, or de-array:

```sh
printf '%s\n' ./386/bin ./386/bin/ape ... | while read -r d; do mkdir -p "$d"; done
```

## Info

### IN-01: 7a/BUILD misses 7c/BUILD's stale-object sweep

**File:** `sys/src/cmd/7a/BUILD` (vs `sys/src/cmd/7c/BUILD:15`)
**Issue:** 7a compiles `9c ... *.c` and links `9l -o 7a *.o` with no preceding `rm -f *.o`; a stale object from an aborted experiment (e.g. a since-removed .c) would be linked into the binary. 7c/BUILD has the sweep; 7l/BUILD names its objects explicitly and is immune. Harmless in CI (clean trees), surprising locally.
**Fix:** Add `rm -f *.o` after the nm-shim block, matching 7c/BUILD.

### IN-02: 7l/mkfile defines CFILES that nothing consumes

**File:** `sys/src/cmd/7l/mkfile:27-28`
**Issue:** `CFILES=${OFILES:%.$O=%.c}` / the enam.c substitution are copied from 6l/mkfile, where they feed the `UPDATE=` list; 7l/mkfile has no UPDATE block, so both lines are dead. Harmless; confusing.
**Fix:** Either add the UPDATE block (6l parity) or drop the two lines.

### IN-03: "term%%" printed literally in all three serial drivers

**File:** `build/aarch64/drive9.py:212`, `run-gate.sh:526` (gatedrive9), `subset-exec.sh:598` (subsetdrive9)
**Issue:** `sys.stderr.write("... term%% prompt ...\n")` — `write()` does no `%` interpolation, so the log line reads `term%%`. Cosmetic only.
**Fix:** Single `%`.

### IN-04: srv9.py's fid table is server-global, not per-connection

**File:** `build/aarch64/srv9.py:112,221-223,464-465`
**Issue:** `self.fids` is shared across all connections (the comment claims "per-connection fid table"); `Tversion` clears it globally, and `serveconn`'s disconnect path clears it too — a second concurrent mount would collide on fid numbers and either connection's re-negotiation/hangup nukes the other's fids. Correct today only because the guest mounts exactly once. The header should say so, or fids should move into a per-connection object.
**Fix:** Move `fids` into a per-connection state object created in `serveconn`, or document the single-connection invariant in the header.

### IN-05: srv9.py Twstat rename silently deletes an existing target; no Topen-before-I/O enforcement

**File:** `build/aarch64/srv9.py:420-426` (rename), `286-316, 364-379` (unopened-fid I/O)
**Issue:** (a) Rename-over-existing does `os.remove/os.rmdir` on the target first — 9P wstat rename should fail ("already exists"); divergence is invisible to the guest's `mv` semantics tests. (b) `Tread`/`Twrite` never check `f.open`, so I/O on a walked-but-never-opened fid succeeds — protocol laxity. Both are scoped to the trusted kernel client + opt-in writable prefixes, hence info.
**Fix:** (a) Return `P9Error("file exists")` when `os.path.exists(npath)`; (b) add `if not f.open: raise P9Error("not open")` to Tread/Twrite.

### IN-06: nsrun.sh exports file is world-readable in /tmp and sourced as root

**File:** `build/aarch64/nsrun.sh:30-35, 45, 139`
**Issue:** `/tmp/nsrun-exports.$$` is created with the default umask (world-readable) and holds the full devshell environment (which can include tokens); it is later sourced as root. In the single-user lima guest the privilege boundary is illusory (the user already has sudo), but the exposure window is unnecessary. Also leaks on SIGINT (no trap in --wrap mode).
**Fix:** `umask 077` before `export > "$EXPORTS"` (or `mktemp` it), and `trap 'rm -f "$EXPORTS"' EXIT` in the wrapper branch.

### IN-07: unixcompat.c calloc multiplication overflow / malloc(0) corner

**File:** `sys/src/cmd/7l/unixcompat.c:88-96, 67-80`
**Issue:** `calloc(m, n)` computes `n *= m` with no overflow check, and `malloc(0)` on a cold hunk returns NULL (hunk still nil) — both consistent with the donor's throwaway-allocator idiom (no in-tree caller hits either), but worth a comment since this file is Harvey-authored and will be read as the reference for future host shims.
**Fix:** Comment the intentional omissions, or `if (m && n > (ulong)-1/m) { diag(...); errorexit(); }`.

### IN-08: mkfs9.py crashes on a bare `d` proto mode (latent; unreachable with pinned protos)

**File:** `build/aarch64/mkfs9.py:150,162`
**Issue:** `int(mode.replace("d","").replace("a","").replace("l",""), 8)` on a mode of exactly `d` (legal proto) becomes `int("", 8)` → ValueError traceback. Verified unreachable for the pinned CONF=qemu set (bootfs/disk/devusb.proto contain only 1- and 5-field lines, no bare `d`), and the header disclaims generality — noting for the next consumer.
**Fix:** `os.chmod(dd, int(mode.strip("dal") or "755", 8))` (dirs) — or assert a digit group with a clear error.

### IN-09: BOOTFSPROTOS splice hardcodes line numbers while the guard checks only presence

**File:** `build/aarch64/donor-ns-build.sh:500-515`
**Issue:** The `grep -qF` guard (line 504) verifies the nested-backquote line exists *somewhere* in bootmkfile; the splice then hardcodes `sed -n '1,15p'` / `'20,$p'` (verified correct for the pin: the block is lines 16-19). Safe only because donor-vm-build.sh's pin gate freezes the content — a pin bump without re-verification would silently corrupt the file.
**Fix:** Replace the line-number splice with an anchor-based one (awk range from `/^BOOTFSPROTOS/` through the first non-continuation line), so a moved block still splices correctly or fails the guard.

### IN-10: cc/compat.h has no double-include guard though the vendored tools are also host-compiled

**File:** `sys/src/cmd/cc/compat.h`
**Issue:** Plan 9 convention (no guards; 6c tolerates re-inclusion) — correct in-namespace — but the same header is compiled by host clang/gcc via the BUILD scripts, where a duplicate enum definition is a hard error. Currently included exactly once per TU (`7a/a.h:8`, `7l/l.h:5`), so latent.
**Fix:** None required today; if a future shim creates a second inclusion path, wrap the enum in `#ifndef __CC_COMPAT_H__`-style guard for host builds only (or document the single-include rule in the header comment).

### IN-11: three ~250-line forks of the serial driver diverge already

**File:** `build/aarch64/drive9.py`, `run-gate.sh:319-590` (embedded gatedrive9.py), `subset-exec.sh:347-658` (embedded subsetdrive9.py)
**Issue:** The "duplicate the discipline" allowance produced three copies of the Log/reader/qmp_cont/probe core; fixes have already landed unevenly (paced sends + echo verification exist only in the forks; the `term%%` cosmetic bug is in all three; the WR-04 probe weakness is in all three). The next fix will be applied ≤2 of 3 places.
**Fix:** Factor the shared core into a committed `build/aarch64/drv9lib.py` imported by generated drivers, or accept the forks but add a comment cross-referencing the copies.

### IN-12: dumpdtb relies on a fixed 2-second sleep

**File:** `build/aarch64/boot-9qemu.sh:171` (also run-gate.sh:622, subset-exec.sh:691)
**Issue:** `sleep 2; kill $DUMPPID` assumes qemu finishes the DTB dump within 2s. The `[ -s "$DTB" ]` check makes the failure loud rather than silent, so this is cosmetic-robustness only.
**Fix:** Poll for the file: `for i in $(seq 50); do [ -s "$DTB" ] && break; sleep 0.2; done`.

### IN-13: pcc.c `allos` still lacks `4` and `x` though the table has mips64/spim64 rows (pre-existing)

**File:** `sys/src/cmd/pcc.c:31`
**Issue:** Noted because this phase edited the line (added `7`, correct): `allos = "056789kqvij"` covers neither `4` (mips64) nor `x` (spim64), so `.4`/`.x` objects passed to pcc are silently dropped by the suffix filter (lines 151-158) instead of rejected as wrong-architecture. Pre-existing (upstream-shaped), not caused by the aarch64 row.
**Fix:** `allos = "0456789kqvijx"` if mips64/spim64 objects should be diagnosed; or leave and note upstream parity.

---

_Reviewed: 2026-11-01T16:25:12Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
