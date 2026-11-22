# Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers - Pattern Map

**Mapped:** 2026-11-04
**Files analyzed:** 24 (9 modified, 15 new)
**Analogs found:** 21 / 24 (3 new-convention files documented under "No Analog Found")

All analog paths verified git-tracked via `git ls-files` (no gitignored capability mirrors exist in this repo's build tree).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `sys/src/cmd/cc/macbody` (mod, lines 851-868) | C source (host compiler frontend) | batch (host-run) | `sys/src/cmd/7l/unixcompat.c:27-62` | **exact** (proven fix) |
| `sys/src/cmd/6l/obj.c` (mod, lines 1290-1310) | C source (host linker) | batch (host-run) | `sys/src/cmd/7l/unixcompat.c:27-62` | **exact** (proven fix) |
| `aarch64/include/u.h` (mod, lines 54-61) | arch header | compile-time contract | `amd64/include/u.h:49-56` | **exact** (working reference) |
| 8× `build/aarch64/*.sh` (mod, grep hygiene) | shell drivers | batch | in-file (per-script top resolution) | role-match |
| `sys/src/mkfile` (mod: LIBS filter) | mk build file | batch recursion | in-file backquote idioms + `sys/src/cmd/mkfile:3,16-17` | exact (in-file) |
| `sys/src/cmd/mkfile` (mod: ALLTARG/ALLDIRS + filter + checkall) | mk build file | batch recursion | in-file (lines 3, 16-17, 44-57) | exact (in-file) |
| `sys/src/cmd/ip/mkfile`, `cmd/auth/mkfile`, `games/mkfile` (mod) | container mk files | batch recursion | `sys/src/cmd/ip/mkfile:3-46` (current shape) | exact (in-file) |
| `sys/src/games/mkfile` (mod: add checkall) | mk build file | batch recursion | `sys/src/cmd/mkfile:44-57` (checkall) | exact |
| `build/scripts/linux-build` (mod: build_aarch64) | rc build script | batch | in-file `build_aarch64` fn (lines 73-92) | exact (in-file) |
| `build/aarch64/tiers/tier0-libs.txt` (new) | committed data | batch (consumed by mk + sh) | none in-tree (new convention) | **no analog** |
| `build/aarch64/tiers/tier1-cmds.txt` (new) | committed data | batch | none in-tree | **no analog** |
| `build/aarch64/markers/tier{0,1}.ledger` (new) | committed data (append-only) | batch | none tracked (`gate-evidence.log` is gitignored) | **no analog** |
| `build/aarch64/tier-build.sh` (new — or evolve subset-build.sh in place; RESEARCH recommends in-place) | shell driver | batch (namespace build) | `build/aarch64/subset-build.sh` (whole file) | **exact** |
| `build/aarch64/tier-exec.sh` (new — or evolve subset-exec.sh) | shell driver | request-response (boot+assert) | `build/aarch64/subset-exec.sh` (whole file) | **exact** |
| `build/aarch64/tier1-sweep.rc` (new) | rc script (in-guest) | batch (serial markers) | `build/aarch64/trace-sweep.rc` + `trace-termrc` | **exact** |
| `build/aarch64/coverage-sweep.sh` (new) | shell driver | batch | `build/aarch64/subset-build.sh` (shape) + `sys/src/cmd/mkfile:44-57` (mechanism) | role-match |
| `build/aarch64/generator-audit.txt` (new) | committed report | batch | none in-tree | **no analog** |
| `build/aarch64/dir-coverage.txt` (new) | committed report | batch | none in-tree | **no analog** |
| gethunk repro scripts ×2 (new, e.g. `build/aarch64/gethunk-repro-{cc,6l}.sh`) | shell evidence harness | batch (timeout-gated) | `build/aarch64/bquote-repro.sh` (whole file) | **exact** |
| `build/aarch64/tier1-r9.expect` (new) | expect driver | request-response (staged asserts) | `build/aarch64/smoke-r9.expect:63-159` | **exact** |
| `.github/workflows/phase6.yml` (new) | CI config | event-driven | `.github/workflows/r9-shell.yml` (whole file) | **exact** |
| `flake.nix` (mod: add `boot-r9-tier1` app) | nix flake | event-driven | `flake.nix:106-120` (bootR9Pkg/smokeR9Pkg) | exact (in-file) |
| `build/aarch64/mkimage.sh` (mod: tier-1 payload + manifest) | shell bake script | batch | in-file (lines 63-157) | exact (in-file) |
| `sys/src/libdynld/dynld-arm64.c` → `dynld-aarch64.c` (rename, D-08 edge case) | C source (lib) | batch | `sys/src/libdynld/mkfile:6` + mksyslib | role-match |

## Pattern Assignments

### `sys/src/cmd/cc/macbody` + `sys/src/cmd/6l/obj.c` — the gethunk ×2 fixes (USPACE-01)

**Analog:** `sys/src/cmd/7l/unixcompat.c` — Phase 2's PROVEN accumulate-when-contiguous fix.

**The proven fixed gethunk** (unixcompat.c lines 27-62 — copy the tail block verbatim):

```c
void
gethunk(void)
{
	char *h;
	long nh;

	nh = NHUNK;
	if(thunk >= 5L*NHUNK) {
		nh = 5L*NHUNK;
		if(thunk >= 25L*NHUNK)
			nh = 25L*NHUNK;
	}
	h = mysbrk(nh);
	if(h == (char*)-1) {
		diag("out of memory");
		errorexit();
	}
	/*
	 * Merge with the current hunk when contiguous (consecutive sbrk
	 * calls return contiguous addresses while nothing else interleaves
	 * the break) — REQUIRED for malloc()'s `while(nhunk < n)` loop to
	 * terminate for n larger than the largest single hunk. 9front's
	 * cc/compat accumulates unconditionally (nhunk += nh) because it
	 * grows one arena with brk; with sbrk the merge must be conditional.
	 * Without it, malloc(n > 25L*NHUNK) loops gethunk forever (observed:
	 * the arm64 donor-kernel link allocates 3.7MB for the embedded
	 * bootfs data → thunk reached 88TB before sbrk returned ENOMEM).
	 */
	if(h == hunk + nhunk)
		nhunk += nh;
	else {
		hunk = h;
		nhunk = nh;
	}
	thunk += nh;
}
```

**The edit rule (carried-forward amd64-safety):** edit ONLY the tail. At each buggy site, replace:

```c
	hunk = h;
	nhunk = nh;
	thunk += nh;
```

with the accumulate-when-contiguous `if(h == hunk + nhunk) … else …` block above. Each site's own escalation ladder and error reporter stay untouched:

- `cc/macbody:851-868` — keeps `if(thunk >= 10L*NHUNK) nh = 10L*NHUNK;` ladder, `yyerror` reporter, `(char*)mysbrk(nh)` cast. NHUNK = `50000L` (`sys/src/cmd/cc/cc.h:25`); ceiling 500000 bytes.
- `6l/obj.c:1290-1310` — keeps the `5L/25L` ladder, `diag` reporter. NHUNK = `100000` (`sys/src/cmd/6l/l.h:121`); ceiling 2.5MB.

Note the style delta to preserve: macbody's `h = (char*)mysbrk(nh);` cast vs 6l's bare `h = mysbrk(nh);` — match the file you're editing.

**Comment style for the fix block:** adapt unixcompat.c's 10-line `/* … */` block comment (why, not what; names the failure mode and the observed incident). Block comments only — never `//` (style(6)).

**Regression gates:** each fix = one atomic commit (fix + synthetic repro + timeout gate). Repro shapes from RESEARCH (plan-time spike confirms hang signature pre-fix):
- cc: generated C file with a single macro whose replacement text exceeds 600KB; compile with freshly built HOST 7c AND 6c under `timeout 60` — pre-fix hang (timeout kill = RED), post-fix compiles (GREEN).
- 6l: generated C file with ≥4MB initialized data array (fallback: >2.5MB string literal); compile+link with 6c/6l under `timeout 120`.

**Amd64-safety gate after both fixes:** rebuild host toolchain (`nix run .#toolchain`), then amd64 9k kernel build + `mk checkall` green (the Phase-1 gate).

---

### `aarch64/include/u.h` — the FPdbleword fix (D-01)

**Analog:** `amd64/include/u.h:49-56` — the working reference, verbatim:

```c
union FPdbleword
{
	double	x;
	struct {	/* little endian */
		uint lo;
		uint hi;
	};
};
```

**The defect** (aarch64/include/u.h:54-61): identical shape but `ulong lo; ulong hi;` — `ulong` is 64-bit on aarch64 (line 5: `typedef unsigned long ulong;`), so the union is 16 bytes and `hi` sits outside the 8-byte double.

**The edit:** exactly two tokens — `ulong lo;` → `uint lo;`, `ulong hi;` → `uint hi;`. Nothing else in the header changes. Commit message notes `riscv64/include/u.h` carries the same latent defect but is NOT fixed here (skeletal port, out of scope).

**Consumers unbroken:** `sys/src/libc/port/frexp.c` (`e = (x.hi >> SHIFT) & MASK`), `sys/src/libc/port/nan.c`, `sys/src/libc/fmt/fltfmt.c:94` (`frexp(f, &e)` — the `%e/%f/%g` path).

**Rebuild order:** libc (port + fmt) FIRST, then every aarch64 binary. In the sweep the fix commit lands before any tier build; tier-0's fresh libc build picks it up.

**Gate:** float-print marker in the tier-1 sweep — `awk 'BEGIN{printf "%.3f\n", 1.0/3.0}'` must emit `0.333` on donor AND r9. No v1.0 re-baseline needed (no prior gate printed floats).

---

### 8× `build/aarch64/*.sh` — `/usr/bin/grep` hygiene (D-02)

**Verified call-site inventory** (grep this session — 9 hits across 8 files; CONCERNS.md's "six scripts" undercounts):

| File | Lines |
|------|-------|
| `donor-vm-build.sh` | 85 |
| `mkimage.sh` | 127, 129 (126 is the explanatory comment) |
| `pre-gate-build.sh` | 82 |
| `run-gate.sh` | 99 |
| `stage-build.sh` | 52, 129 |
| `subset-build.sh` | 60, 201 |
| `subset-exec.sh` | 116 |
| `vm-run.sh` | 45 |

**Fix pattern (one commit, all 8 files):** resolve a host grep once per script at the top, then use `$GREP` at every site:

```sh
# host grep: plan9 grep lacks -q (mkimage.sh's comment discipline) —
# resolve once; absolute path breaks on NixOS-class hosts (D-02)
GREP=$(command -v /usr/bin/grep || command -v grep) || err "no host grep found"
```

Both call shapes in the inventory take the replacement identically:
- `limactl list -q 2>/dev/null | /usr/bin/grep -qx "$VM"` → `| "$GREP" -qx "$VM"`
- `/usr/bin/grep -q 'SUBSET-BUILD-OK' "$LOG"` → `"$GREP" -q 'SUBSET-BUILD-OK' "$LOG"`

Preserve mkimage.sh:126's comment (the *why*: plan9 grep lacks `-q`); update it to reference the resolution line.

---

### `sys/src/mkfile` + `sys/src/cmd/mkfile` + container mkfiles — the objtype-gated allowlist filter (D-13)

**Analog:** in-file idioms. The backquote-assignment vocabulary is already the file's own language:

`sys/src/cmd/mkfile:3,16-17` (existing backquote patterns to copy):

```mk
TARG=`{ls *.[cy] | sed '/\.tab\.c$/d;s/..$//'}
cpuobjtype=`{sed -n 's/^O=//p' /$cputype/mkfile}
DIRS=`{ls -l | sed '/^d/!d; s/.* //; /^('$NOMK')$/d'}
```

`sys/src/cmd/mkfile:44-57` (checkall — the report-all accumulator; note the brace-group, not `@{}`, so `fail=()` survives; explicit `exit 0`):

```mk
checkall:V:
	fail=()
	for(i in cc $DIRS) {
		cd $i
		echo mk $i
		if(! mk $MKFLAGS all)
			fail=($fail $i)
		cd ..
	}
	if(! ~ $#fail 0) {
		echo 'checkall failures: '^$"fail
		exit checkall
	}
	exit 0
```

**Filter idiom shape** (mechanics verified in RESEARCH; final rc spelling is planner's discretion):

```mk
# sys/src/cmd/mkfile — rename existing computed lists:
ALLTARG=`{ls *.[cy] | sed '/\.tab\.c$/d;s/..$//'}
ALLDIRS=`{ls -l | sed '/^d/!d; s/.* //; /^('$NOMK')$/d'}
TIERFILE=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
# D-13: the aarch64 leg sweeps the append-only allowlist; every other
# objtype sweeps the full tree exactly as before (byte-parity branch).
TARG=`{if(~ $objtype aarch64){ <keep-rule over $ALLTARG> } if not echo $ALLTARG}
DIRS=`{if(~ $objtype aarch64){ <keep-rule over $ALLDIRS> } if not echo $ALLDIRS}
```

**Uniform keep-rule (RESEARCH-verified, serves every level):** at a mkfile whose sys/src-relative path is P, candidate `i` is kept iff some tier entry E satisfies `E == P/i` (member) or `E` starts with `P/i/` (container of a member).

**Hard constraints (from verified mechanics):**
1. **`checkall` iterates `$ALLDIRS`, never filtered `$DIRS`** — cmd/mkfile line 46's `for(i in cc $DIRS)` becomes `for(i in cc $ALLDIRS)`. Filtering checkall makes the USPACE-02 coverage proof tautological (Pitfall P6-1).
2. **`$HARVEY/build/...` paths, never `/build/...`** — `/build` is NOT bound in either namespace setup (`BUILD_SETUP_NAMESPACE`/`nsrun.sh`); under `unshare -m` (not chroot) host-absolute paths stay live. (Pitfall P6-2.)
3. **Plan 9 grep dialect inside mk recipes** — no `-q`, no `-E` on the recipe shell's grep; use rc's `~` list-membership operator or `grep pattern file >/dev/null` with `if`. (Pitfall P6-3.)
4. **amd64 passthrough must be byte-identical** — `if not echo $ALLDIRS` reproduces today's expansion word-for-word.
5. **Comment lines in the data files are safe without stripping** — `~` exact-match can never match a `# …` line to a real dir name.

**`sys/src/mkfile` LIBS filter** — same shape over the 35-entry LIBS list (lines 5-40): rename to `ALLLIBS`, filter against `tier0-libs.txt` when `~ $objtype aarch64`. The recursion loop (lines 53-58) and `libs:` target (79-87) consume `$LIBS` unchanged. libc stays first-class: `linux-build`'s explicit libc bootstrap line is untouched.

**Container mkfiles** (`sys/src/cmd/ip/mkfile` shown; `cmd/aux`, `cmd/auth`, `sys/src/games` same treatment) — each holds its own explicit `TARG` + `DIRS` (ip/mkfile:3-26) and overrides `all:V:`/`install:V:` AFTER `</sys/src/cmd/mkmany` (ip/mkfile:37-55). The filter idiom applies per container mkfile that holds tier-listed members — keeping container `ip` in cmd's DIRS still builds ip's 22 TARGs unless ip/mkfile's own TARG is filtered too (RESEARCH §Allowlist Hook ¶2; mk double-rule semantics flagged as Open Question A1 — one plan-time spike before locking).

**`sys/src/games/mkfile` checkall addition** — copy cmd/mkfile:44-57's shape verbatim over games' `$DIRS` (games/mkfile:18-25); the file currently has only `dirall`/`dirinstall` (lines 47-57). Additive target; existing `all:V: dirall` untouched.

---

### `build/aarch64/tier-build.sh` — the tier sweep driver (D-05..D-08; extends subset-build.sh)

**Analog:** `build/aarch64/subset-build.sh` (245 lines, whole file). RESEARCH's primary recommendation: **evolve subset-build.sh in place** — its hardcoded lib/cmd loops become tier-file loops (the v1.0 16-subset ⊂ tier1-cmds.txt by construction), so mkimage.sh/smoke-r9.sh inherit tier-1 without call-site changes.

**Shell skeleton pattern** (subset-build.sh:20-77 — copy wholesale):

```sh
#!/bin/sh
set -eu

HARVEY="${HARVEY:-$PWD}"
VM="${HARVEY_VM:-harvey}"
LOG="${SUBSET_BUILD_LOG:-$HARVEY/.build/subset-build.log}"

err() { echo "error: $*" >&2; exit 1; }

[ -d "$HARVEY/sys/src" ] || err "$HARVEY does not look like the harvey checkout (missing sys/src)"
# … precondition block …

# Platform split: linux runs natively (sudo unshare via nsrun.sh);
# darwin drives the lima guest.  Guest script + host asserts shared.
NATIVE_LINUX=
if [ "$(uname -s)" = "Linux" ]; then
	NATIVE_LINUX=1
fi

if [ -n "$NATIVE_LINUX" ]; then
	export HARVEY                       # nsrun.sh --wrap re-sources `export` output in-namespace
	_hb_arch=$(uname -m); [ "$_hb_arch" = "arm64" ] && _hb_arch=aarch64
	HARVEY_LINUX_BIN="${HARVEY_LINUX_BIN:-$HARVEY/.build/hostbin-linux-$_hb_arch/bin}"
	export HARVEY_LINUX_BIN
	# … host toolchain bootstrap if missing …
	export FORCERCFORMK=1 CGO_ENABLED=0 objtype=amd64 cputype=unix
else
	command -v limactl >/dev/null 2>&1 || err "limactl not found — run inside the nix devshell"
	# … lima ensure-running …
fi

TMPD=$(mktemp -d "${TMPDIR:-/tmp}/subset-build.XXXXXX")
cleanup() {
	rm -rf "$TMPD"
}
trap cleanup EXIT
```

**Guest-script heredoc pattern** (subset-build.sh:83-103): `cat > "$TMPD/subset-build-guest.sh" <<'GUEST_EOF'` (quoted delimiter — no host expansion), guest script starts `#!/usr/bin/env bash` + `set -e` + a `fail()` helper emitting `SUBSET-BUILD-FAIL: <dir>` markers, provenance echo of the resolved toolchain (`command -v 7c` etc., lines 93-98).

**The lib loop to generalize** (subset-build.sh:113-116 — becomes the tier-0 loop):

```sh
for d in libc libbio libregexp libip libndb libmp libsec libString libthread lib9p libauth libauthsrv libflate; do
	echo "== subset lib: $d"
	(cd "/sys/src/$d" && objtype=aarch64 mk install) || fail "lib $d"
done
```

**D-07 fresh-archive + artifact-marker evolution** (the tier-0 loop's new body; RESEARCH-verified shape):

```sh
while read -r line; do
	case "$line" in ''|'#'*) continue;; esac
	d="$line"
	rm -f "/aarch64/lib/lib${d#lib}.a"          # D-07: kill stale members (02-10 class)
	(cd "/sys/src/$d" && objtype=aarch64 mk install) || fail "lib $d"
	[ -s "/aarch64/lib/lib${d#lib}.a" ] || fail "lib $d: no artifact"
	echo "T0-ART $d $(sha256sum "/aarch64/lib/lib${d#lib}.a" | cut -d' ' -f1)"
done < "$HARVEY/build/aarch64/tiers/tier0-libs.txt"
```

(The `ar vu $LIB $newprereq` rule in `sys/src/cmd/mksyslib:5-6` is append-only and never prunes — hence the `rm -f`. `$HARVEY/...` path, not `/build/...`.)

**The cmd loops to generalize** (subset-build.sh:123-135 — three proven shapes the tier-1 entry forms map onto):

```sh
# dir entries:                    (cd "/sys/src/cmd/$d" && objtype=aarch64 mk install)
# per-target entries (dir:target): (cd /sys/src/cmd/auth/factotum && objtype=aarch64 mk factotum.install)
# single-file cmd/ entries:       cd /sys/src/cmd && objtype=aarch64 mk cat.install cp.install …
```

The per-target `%.install:V: $BIN/%` rule is mkmany's (`sys/src/cmd/mkmany:28`) / cmd/mkfile:75's — the subset-proven way to build one member of a multi-TARG dir without its out-of-tier siblings (factotum-without-fgui precedent, subset-build.sh:118-128).

**Run + assert pattern** (subset-build.sh:179-201): `set +e` around the nsrun/vm-run invocation, capture RC, `cat "$LOG"` always, RC-check with a hint naming the log, then marker grep (`SUBSET-BUILD-OK` → tier equivalent), then platform-split rsync-back (darwin) vs in-place (linux). **Host-side artifact asserts** (lines 230-243): `[ -x … ]` per binary + the `od -An -tx1 -N4 … | tr -d ' \n'` = `00008c47` R_MAGIC check per boot-chain binary — the exact idiom the tier artifact markers and the USPACE-03 `.7`-magic CI scan reuse.

---

### `build/aarch64/tier-exec.sh` — the donor execution gate (D-10; extends subset-exec.sh)

**Analog:** `build/aarch64/subset-exec.sh` (494 lines). Same 4-phase shape: stage → CONF patch + rebake → boot → assert.

**Precondition + pin-verify pattern** (subset-exec.sh:103-136): donor pin assertion first —

```sh
actual=$(git -C "$DONOR" rev-parse HEAD 2>/dev/null || echo missing)
[ "$actual" = "$DONOR_SHA" ] || err "donor pin mismatch: want $DONOR_SHA, have $actual"
```

then artifact existence per staged binary, tool resolution via `command -v … || true` + explicit `-x` check (QEMU_BIN/HARNESS_BIN shape, lines 124-136).

**Cleanup-trap pattern** (subset-exec.sh:148-160) — copy verbatim (three PIDs + TMPD, `exit $status`):

```sh
TMPD=$(mktemp -d "${TMPDIR:-/tmp}/subset-exec.XXXXXX")
SRV9_PID=""
QEMU_PID=""
POKE_PID=""
cleanup() {
	status=$?
	[ -n "$QEMU_PID" ] && kill "$QEMU_PID" 2>/dev/null || true
	[ -n "$SRV9_PID" ] && kill "$SRV9_PID" 2>/dev/null || true
	[ -n "$POKE_PID" ] && kill "$POKE_PID" 2>/dev/null || true
	rm -rf "$TMPD"
	exit $status
}
trap cleanup EXIT
```

**Staging pattern** (subset-exec.sh:166-174): rsync each tier binary into the guest donor tree under `arm64/bin/<tier>/` with the `h`-prefix renaming (`h$f` — the guest's own binaries stay distinguishable from Harvey's under test).

**CONF bootdir-block patch** (subset-exec.sh:177-195 + 284-302): data-file of `/$objtype/bin/subset/hX hX` lines heredoc'd, rsynced, then the **marker-guarded idempotent awk patch**:

```sh
CONF="$DONOR/sys/src/9/arm64/qemu"
if grep -qF '/$objtype/bin/subset/hrc' "$CONF"; then
	echo "== CONF already carries the subset bootdir block (idempotent rerun)"
else
	awk '
		/^bootdir$/ { inbd=1; print; next }
		inbd && /^[^ \t]/ {
			while ((getline l < "/tmp/gsd-subset-bootdir.txt") > 0) print l
			inbd=0; print; next
		}
		{ print }
		END {
			if (inbd)
				while ((getline l < "/tmp/gsd-subset-bootdir.txt") > 0) print l
		}
	' "$CONF" > "$CONF.subset"
	mv "$CONF.subset" "$CONF"
fi
```

CONF bootdir entries carry text scripts as DATA — "mkrootall strips only file-identified executables; text rides through" (trace-boot.rc header). This is how `tier1-sweep.rc` + `hello.c` reach `/boot`.

**Self-re-entrant guest script shape** (subset-exec.sh:198-279, the run-gate.sh lineage): `wrap -> --ns -> --build` in ONE script — wrap phase does the CONF patch + `export > "$EXPORTS"` + `sudo unshare -m bash "$0" --ns "$USER" "$EXPORTS"`; `--ns` does the bind-mounts (`mount --bind "$DONOR/sys" /sys` etc., lines 252-267, with the merged-/lib staging dance lines 260-267) then `su -c ". '$ENVFILE'; bash '$0' --build"`; `--build` sets PATH/objtype/FORCERCFORMK, runs the targeted rebake. **This is the template for the D-16 native-linux CI leg** (Pitfall P6-4): on linux CI the "guest" phases run host-side.

**Targeted rebake pattern** (subset-exec.sh:225-237):

```sh
cd /sys/src/9/arm64
# force regeneration of everything derived from the CONF/bootdir
rm -f qemu.c qemu.7 qemu.root.s qemu.root.7 qemu.rootc.c qemu.rootc.7 9qemu 9qemu.u
mk 'CONF=qemu'
[ -s 9qemu ] && [ -s 9qemu.u ] || { echo "error: relink produced no kernel" >&2; exit 1; }
```

**qemu boot invocation** (subset-exec.sh:400-429 — the drift-accommodated donor shape; copy exactly, TCG only):

```sh
# dump the DTB with -append baked in, launch PAUSED with it pre-placed
"$QEMU_BIN" \
	-M virt,gic-version=3,highmem-ecam=off -cpu cortex-a72 -m 1G \
	-kernel "$GATE_U" \
	-append "$APPEND" \
	-machine "dumpdtb=$DTB" -display none -S &
# … then the real launch:
"$QEMU_BIN" \
	-M virt,gic-version=3,highmem-ecam=off -cpu cortex-a72 -accel tcg -smp 1 -m 1G \
	-kernel "$GATE_U" \
	-append "$APPEND" \
	-device "loader,file=$DTB,addr=0x40000000" \
	-netdev "user,id=n0,guestfwd=tcp:$SRV9_GUEST_IP:564-tcp:127.0.0.1:$SRV9_PORT" \
	-device virtio-net-pci-non-transitional,netdev=n0 \
	-chardev "socket,id=ser0,path=$TMPD/serial.sock,server=on,wait=off" \
	-serial chardev:ser0 \
	-qmp "unix:$TMPD/qmp.sock,server=on,wait=off" \
	-gdb "unix:$TMPD/gdb.sock,server=on,wait=off" \
	-S -display none &
```

plus the GICR wake poke (`harness gdbpoke9 --watch-str "bootargs is"`, lines 437-446) and srv9 serve with writable prefixes (lines 381-398 — `--writable-prefix /tmp --writable-prefix /usr/glenda/tmp`; the writable /tmp is what the D-11 self-host canary's in-guest compile needs).

**Marker assertion block** (subset-exec.sh:473-492 — the defense-in-depth tail; the tier version asserts per-binary `T1-OK` presence instead of the 22 named markers):

```sh
grep -qiE '9front|Plan 9' "$RUNLOG" || err "captured log lacks the 9front/Plan 9 banner"
grep -q 'term% ' "$RUNLOG" || err "captured log lacks the rc prompt signature"
# … one presence-grep per marker …
if grep -qiE 'panic|trap:' "$RUNLOG"; then
	err "captured log contains a kernel fault line"
fi
```

Presence-based, never interleaving order; generous TCG-sized budgets (`BOOT_BUDGET`).

---

### `build/aarch64/tier1-sweep.rc` — the sourced in-guest sweep (D-10/D-11/D-01 gates)

**Analogs:** `build/aarch64/trace-termrc` (9 lines) + `build/aarch64/trace-sweep.rc` (13 lines) — the 05-03 proven sourced-not-typed pair.

**trace-termrc, verbatim** (the staged termrc whose final exec sources the sweep — zero console interaction, TCG-timing tolerant):

```rc
#!/bin/rc
# trace termrc (guest scratch — staged as /boot/ourtermrc and
# bound over /rc/bin/termrc by the trace namespace).  Identical to the
# image's termrc except the final exec: the sweep is SOURCED from
# /boot/oursweep.rc instead of typed at the console, because the donor's
# devcons fails /dev/cons reads by design (Egreg — userland console
# input on the donor is kbdfs-served; r9's devcons serves reads).
stacksmoke
exec rc -c '. /boot/oursweep.rc'
```

**trace-sweep.rc, verbatim** (sweep body shape; ends with `exit` so init's restart-loop sleep is captured before teardown):

```rc
#!/bin/rc
echo __GSD_SWEEP_ECHO__
ls /bin
ps
cat /rc/lib/rcmain
exit
```

**Tier-1 evolution** (marker vocabulary is discretion; the proven idioms): per-binary fixed-input run wrapped so failure cannot abort the sweep —

```rc
fn check { # name cmd...
	if($2) echo T1-OK $1
	if not echo T1-FAIL $1
}
# float canary (D-01, MANDATORY):
out=`{/boot/hawk 'BEGIN{printf "%.3f\n", 1.0/3.0}'}
if(~ $out 0.333) echo T1-OK float-d01
if not echo T1-FAIL float-d01
# self-host canary (D-11): compile+run in the writable /tmp scratch
cd /tmp && /boot/h7c -FTVw /boot/hello.c && /boot/h7l -o /tmp/hello hello.7 && /tmp/hello
# … then match output against the known string for T1-OK selfhost …
echo T1-DONE
exit
```

Host asserts: every tier1-cmds.txt entry has `T1-OK`, zero `panic|trap:` lines, `T1-DONE` present. The writable-prefix exec (A3) is the one unproven mechanic — plan-time verification with documented fallback (compile-only marker + host-side R_MAGIC assert on a fetched copy).

---

### gethunk repro scripts ×2 — synthetic timeout-gated evidence harnesses (USPACE-01 gates)

**Analog:** `build/aarch64/bquote-repro.sh` (99 lines) — the Phase-1 nested-backquote repro harness. Copy its skeleton wholesale:

**Mode split + pristine-source capture** (bquote-repro.sh:24-45):

```sh
MODE="${1:?usage: bquote-repro.sh pristine|working}"

REPRO="$HARVEY_ROOT/.build/bquote-repro"     # gitignored scratch — never committed artifacts
mkdir -p "$REPRO"
SRC="$REPRO/src-$MODE"
rm -rf "$SRC"
mkdir -p "$SRC"

if [ "$MODE" = pristine ]; then
	# pre-fix tree straight from git — the working tree stays untouched
	git archive HEAD sys/src/cmd/mk | tar -x -C "$SRC"
else
	mkdir -p "$SRC/sys/src/cmd"
	cp -R sys/src/cmd/mk "$SRC/sys/src/cmd/mk"
fi
```

**$PLAN9/bin LAST discipline** (bquote-repro.sh:26-29 — copy verbatim):

```sh
P9LAST=$(echo "$PATH" | sed "s|:$PLAN9/bin||g; s|^$PLAN9/bin:||; s|^$PLAN9/bin$||")
PATH="$P9LAST:$PLAN9/bin"
export PATH
```

**Host-tool build idiom** (bquote-repro.sh:66-79): per-file `9c` compile loop + `9l` link with the plan9port libs; the gethunk repros instead use the already-built hostbin `cc`/`6c`/`6l` under `timeout` — the difference is the gate, not the harness:

```sh
set +e
timeout 60 "$HOSTBIN/6c" -FTVw "$REPRO/bigmacro.c" > "$LOG" 2>&1
RC=$?
set -e
# pre-fix: RC=124 (timeout kill) is the RED regression signature;
# post-fix: RC=0 GREEN
```

Generated inputs land in the gitignored `.build/` scratch via shell loops (`#define BIG "…"` ×600KB for cc; `char big[4194304] = {…};` for 6l). Each repro ships in the SAME atomic commit as its fix (Pitfall 4 discipline).

---

### `build/aarch64/coverage-sweep.sh` + `dir-coverage.txt` — the USPACE-02 report (D-15)

**Analogs:** subset-build.sh's driver skeleton (platform split, TMPD trap, log discipline) + `sys/src/cmd/mkfile:44-57`'s checkall (the mechanism) — quoted above.

**Key mechanics:**
- checkall runs UNFILTERED: `objtype=aarch64 mk checkall` iterates `$ALLDIRS` (Pattern: the `dirs`/`all` targets use filtered `$DIRS`; checkall uses `$ALLDIRS`).
- The `echo mk $i` per-visit lines in the log ARE the reach evidence — the script parses the log, cross-checks against `find sys/src/cmd sys/src/games -name mkfile | wc -l`.
- **Gate = reach, not green:** per-dir `build=ok|fail` is DATA (Phase-10 backlog), not a gate failure.
- **Hang watchdog:** watch log-tail staleness (kill at N minutes without a new `mk <dir>` line; N TCG-sized generous). Wave-0's gethunk fixes land FIRST by dependency order.
- Report format (RESEARCH proposal, diff-stable): header (commit, date, host triple, objtype) + one line per dir `sys/src/cmd/ip/ipconfig visited=1 build=ok` + explicit `excluded=NOMK|decision` lines + the find cross-check tail. CI regenerates and `git diff --exit-code`s.

---

### `build/aarch64/generator-audit.txt` + CI `.7`-magic scan — USPACE-03

**No in-tree analog for the committed report file** (new convention), but every MECHANISM is copied:

- The 9 swept generator sites are enumerated in RESEARCH §Generator Audit with file:line (G1 awk/mkfile:42-50, G2/G3 jpg:60-74, G4 jtagfs:44-52, G5 spell:18-25, G6 gs:~147-160 guarded, G7 usbd:33-36 rc script, G8 bayes:14-17 guarded, G9 7l:35-36 mkcname).
- Report line format (fixed field order, diff-stable): `G1 site=sys/src/cmd/awk/mkfile:43 generator=maketab artifact=unix.maketab built-by=objtype=$cputype-mk runs=always proof=host-magic`.
- The magic-scan idiom copies subset-build.sh:241-242's `od -An -tx1 -N4 … | tr -d ' \n'`; RESEARCH's CI fragment: red on `00008c47` (R_MAGIC .7), pass `7f454c46` (ELF) / `23212f*` (`#!`), whitelist exactly the audited guarded rows (G6's `7.genarch` — Pitfall P6-5).
- Build-log scan: `cannot execute|exec format error|Exec format error` → red.

---

### `build/aarch64/tier1-r9.expect` — the r9-side tier-1 gate driver (D-12)

**Analog:** `build/aarch64/smoke-r9.expect` — stays untouched (v1.0 gate); the new driver copies its machinery:

**Env contract + spawn** (smoke-r9.expect:63-96): loud unset-env check `foreach v {…} { if {![info exists env($v)]} { … exit 2 } }`, `exec rm -f $env(R9_SMOKE_LOG)` fresh-capture hygiene (line 79), spawn with caller-selected shape from env (R9_MACHINE/R9_CPU/R9_ACCEL, lines 85-94), `-name $run_tag` per-run unique tag, `log_file` capture.

**Teardown** (smoke-r9.expect:102-106 — kill the process GROUP, catch already-dead):

```tcl
proc cleanup {} {
    global spawn_pid
    catch { exec kill -KILL -- -$spawn_pid 2>/dev/null }
    catch { exec kill -KILL $spawn_pid 2>/dev/null }
}
```

**Staged assertion proc** (smoke-r9.expect:112-119 — each stage names its failure; `expect_out` must be global):

```tcl
proc stage {name re} {
    global expect_out
    expect {
        -re $re { puts stderr "smoke-r9: stage ok — $name" }
        timeout { puts stderr "smoke-r9: TIMEOUT at stage: $name"; cleanup; exit 1 }
        eof { puts stderr "smoke-r9: qemu exited (EOF) at stage: $name"; cleanup; exit 1 }
    }
}
```

**Panic fast-fail** (smoke-r9.expect:123-129): `expect_before { -re {panic|Unhandled interrupt} { … exit 1 } }`.

**The D-12 difference:** r9's devcons serves reads, so typed works r9-side — at the `% ` prompt the driver sends ONE line (`send "rc /lib/tier1-sweep.rc\r"`), then stages per-marker waits (presence-only, TCG-sized 600s/stage timeouts), `T1-DONE` last, then a whole-file zero-panic scan. The two-occurrence echo discipline (smoke-r9.expect:137-146 — typed echo first, output second, each stage consumes one occurrence) applies to that one typed line.

**Wrapper side** (`nix/smoke-r9.sh` pattern for a sibling `nix/tier1-r9.sh`): R9_RUN_TAG per-run keying + bracket-fixed pkill survivor sweep (`pkill -f "[q]emu-system-aarch64.*${R9_RUN_TAG}"`, smoke-r9.sh:172-186) — never kernel-name-class sweeps (CONCERNS WR-07).

---

### `.github/workflows/phase6.yml` — the full-pipeline CI (D-16 + D-04)

**Analog:** `.github/workflows/r9-shell.yml` (40 lines, whole file) — left untouched; the new workflow is a sibling.

**Shape to copy** (r9-shell.yml:23-40):

```yaml
name: r9-shell

on: [push, pull_request]

permissions:
  contents: read

jobs:
  boot-smoke:
    runs-on: ubuntu-latest
    timeout-minutes: 90 # TCG scale; per-stage 600s timeouts localize failures
    steps:
      - uses: actions/checkout@v4
      - name: install nix
        uses: DeterminateSystems/nix-installer-action@3138316df39ed29be04236d7ffc686fa525866aa
      - name: boot r9 + harvey aarch64 userland smoke
        run: nix run .#boot-r9-smoke
```

**Supply-chain upgrades the NEW workflow must make** (CONCERNS IN-06; r9-shell.yml's header names the rule but line 35 leaves checkout at `@v4` — do NOT repeat):
- Pin ALL actions by 40-char SHA, **including `actions/checkout`**.
- Add `concurrency: { group: phase6-${{ github.ref }}, cancel-in-progress: true }` (r9-shell.yml lacks one; 90-min serial runs amplify).
- `permissions: contents: read` as shown.

**Job split (RESEARCH recommendation; discretion):**
- `wave0-host` (fast): setup+toolchain → gethunk synthetic repros (timeout-gated) → generator audit + `.7`-magic scan → grep-hygiene grep (no `/usr/bin/grep` literal survives in build/aarch64/*.sh) → FPdbleword header assertion → coverage sweep + `git diff --exit-code`.
- `tier-donor` (slow, TCG aarch64): full build → tier-build → tier-exec via the NATIVE_LINUX split (subset-build.sh:37-57 + the self-re-entrant `--ns` shape — Pitfall P6-4; the donor fetch+pin-verify runs host-side per donor-vm-build.sh:105-106's logic).
- `tier-r9` (slow, TCG aarch64): `nix run .#boot-r9-tier1`.
- `x86_64-poolpanic` (D-04): below.

---

### x86_64 pool-panic CI leg (D-04, USPACE-10) — plain-qemu serial capture

**Analog:** `build/mm/boot-9k10cpu.sh` — the Phase-1 serial capture/assert discipline, minus minimega (deferred, D-03).

**Patterns to copy:**

*Acceleration probe + budget* (boot-9k10cpu.sh:76-96):

```sh
TCG=0
if [ -w /dev/kvm ]; then
	:
elif [ "${MM_TCG:-0}" = "1" ]; then
	TCG=1
	# … resolve qemu-system-x86_64 …
else
	err "/dev/kvm not writable and MM_TCG unset — …"
fi

BUDGET="${MM_BOOT_BUDGET:-}"
if [ -z "$BUDGET" ]; then
	if [ "$TCG" = 1 ]; then BUDGET=600; else BUDGET=60; fi
fi
```

*Capture-before-start + deadline-wait + banner-grep* (boot-9k10cpu.sh:217-244 — the banner grep, not the timer, is the source of truth):

```sh
: >"$MM_LOG"
"$SOCAT" -u "UNIX-CONNECT:$SERIAL_SOCK" "CREATE:$MM_LOG" &
SOCAT_PID=$!
# … start the VM …
echo "== waiting for 'Plan 9' banner (budget ${BUDGET}s)"
deadline=$((SECONDS + BUDGET))
found=1
while [ $SECONDS -lt $deadline ]; do
	if grep -q 'Plan 9' "$MM_LOG" 2>/dev/null; then
		found=0
		break
	fi
	sleep 5
done
```

**The plain-qemu replacement** (RESEARCH-verified; minimega's essence without the daemon): `qemu-system-x86_64 -kernel amd64/9k10cpu.elf -m 512 -accel kvm -cpu host -display none -no-reboot -serial file:"$LOG" &` (TCG fallback: `-accel tcg -cpu qemu64`, 600s budget). The kernel artifact is `sys/src/9k/k10/mkfile:22`'s `$target.elf` (`6l -H5`, multiboot header at `k10/l32p.s:25-28`); CI builds it from a clean checkout via `nix run .#build` (guaranteeing the pswt-fixed 6c — fix `7efff3aa3`).

**Assertion content (locked by D-04):** banner `Plan 9` present AND classify against the recorded bad-magic pool-panic signature set: `bad magic`, `panic`, `suicide: sys: trap` (note: "pool panic" does NOT exist verbatim in the 9k tree — use the robust set). Healthy continuation markers: boot.c's `argc=%d` / `version...`. **Verdict recorded either way** — the job asserts verdict PRODUCTION (banner seen + classification written to committed evidence), never the outcome; a FAIL verdict closes USPACE-10 with information + spawns a deferred item.

---

### `flake.nix` — the `boot-r9-tier1` app (D-12)

**Analog:** in-file — `flake.nix:106-120`, the bootR9Pkg/smokeR9Pkg pair. Copy smokeR9Pkg's shape exactly:

```nix
smokeR9Pkg = mkScriptApp pkgs "harvey-boot-r9-smoke" ./nix/smoke-r9.sh ''
  export R9_KERNEL_PKG="${r9.packages.${system}.kernel-virt}"
'' ([ pkgs.plan9port pkgs.bash pkgs.qemu pkgs.dtc pkgs.git pkgs.expect ]
  ++ pkgs.lib.optionals isLinux [ pkgs.util-linux pkgs.gnused pkgs.mount pkgs.binutils ]
  ++ pkgs.lib.optionals (!isLinux) [ pkgs.lima pkgs.openssh pkgs.rsync ]);
```

The tier-1 sibling: same `mkScriptApp` + `R9_KERNEL_PKG` export + `pkgs.expect` in runtimeInputs; new script `./nix/tier1-r9.sh`; registered in the `optionalAttrs (builtins.hasAttr system r9.packages)` attrset alongside `boot-r9-smoke` (line 133-136). mkScriptApp itself (lines 26-39) already enforces `$PLAN9/bin LAST`.

---

### `build/aarch64/mkimage.sh` — tier-1 payload + manifest extension

**Analog:** in-file. The patterns to extend:

**Input precondition pattern** (mkimage.sh:63-74 — one `[ -f ] || err "… run sh build/aarch64/subset-build.sh first"` per input; the tier-1 binaries + `tier1-sweep.rc` + `hello.c` join this block, naming their producer script).

**Staging pattern** (lines 95-109): `cp` per binary into `"$STAGE/aarch64/bin/…"`; the sweep script lands at e.g. `"$STAGE/lib/tier1-sweep.rc"` as data.

**Pre-bake manifest** (lines 111-139 — one `[ -f "$STAGE/$p" ] || err "manifest: $p missing"` per mandated path; extend the loop's path list).

**The real-init rodata guard** (lines 127-130) stays the tripwire — tier machinery must NOT add a third writer to `build/aarch64/out/` (Pitfall P6-7; tier-1 binaries live in `aarch64/bin`, the existing harvest target).

**Byte-format asserts** (lines 153-157 — paqfs header `529ab12b` / trailer `6b46e688` via od/dd) unchanged.

---

### `build/scripts/linux-build` — build_aarch64 evolution

**Analog:** in-file `build_aarch64` fn (lines 73-92) — the current hardcoded subset:

```rc
fn build_aarch64{
	for(i in /sys/src/libc) {cd $i && objtype=aarch64 mk nuke && objtype=aarch64 mk install && objtype=aarch64 mk clean}
	for(i in /sys/src/libbio) {cd $i && objtype=aarch64 mk install}
	for(i in /sys/src/cmd/7?) {cd $i && objtype=aarch64 mk nuke && objtype=aarch64 mk install && objtype=aarch64 mk clean}
	for(i in libregexp libip libndb libmp libsec libString libthread lib9p libauth libauthsrv){
		cd /sys/src/$i && objtype=aarch64 mk install
	}
	for(i in rc grep ip/ipconfig){
		cd /sys/src/cmd/$i && objtype=aarch64 mk install
	}
	# factotum installs per-target: the dir's second TARG (fgui) needs the
	# out-of-scope graphics stack (libdraw/libcontrol) and is not in the named subset
	cd /sys/src/cmd/auth/factotum && objtype=aarch64 mk factotum.install
	cd /sys/src/cmd && objtype=aarch64 mk cat.install cp.install date.install …
}
```

The phase grows this leg to the filtered full sweep (the mk-layer filter does the filtering; the explicit libc-first bootstrap line stays unchanged — RESEARCH §Allowlist Hook ¶7). rc function shape (`fn name{…}`, `for(i in …){cd … && …}`) is the file's own idiom.

---

### `sys/src/libdynld/dynld-arm64.c` → `dynld-aarch64.c` — D-08 edge case

**Analog:** `sys/src/libdynld/mkfile` (10 lines) + `sys/src/cmd/mksyslib`:

```mk
</$objtype/mkfile

LIB=/$objtype/lib/libdynld.a

OFILES=\
	dynld-$objtype.$O\
	dynloadfd.$O\
	dynld.$O\

</sys/src/cmd/mksyslib
```

The mkfile requires `dynld-aarch64.7` but only `dynld-arm64.c` exists (Inferno import `d6405e5fb`). D-08 recommendation: attempt the one-file rename (additive commit); if compile fails, the documented-exclusion comment in `tier0-libs.txt` records the attempt + failure. Zero dynld.h consumers exist outside libdynld, so execution-leg exclusion is already determined either way.

---

### Tier data files + ledgers + committed reports (new conventions — see No Analog Found)

No in-tree committed-data analogs exist; the conventions are fixed by decision text and verified mechanics:

- **`tiers/tier0-libs.txt` / `tier1-cmds.txt`** (D-13): one entry per line, `#` comments, sys/src-relative forms: `libdraw` (lib), `cmd/sort` (single), `cmd/awk` (dir), `cmd/ip/ipconfig` (nested), `cmd/auth/factotum:factotum` (dir:target — consumed by the driver's per-target install; the mk filter's prefix keep-rule keeps the container). Append-only: growth = append a line. Single-source-as-data convention (r9 TOML precedent, cross-repo). Two consumers: the mk filter AND the rc/sh drivers.
- **`markers/tier{0,1}.ledger`** (D-14): machine-readable append-only lines, fixed field order `tier dir artifact/bin sha256 exec evidence commit date`, space-separated `key=value`. sha256 via devshell `sha256sum` (harness is deliberately std-only Rust — no sha256 there). New lines land in their own commits with the gate evidence (r9 SHA in the evidence log when r9 is involved).
- **`generator-audit.txt` / `dir-coverage.txt`:** committed, diff-stable, CI-regenerated-and-diffed (red on drift, D-15).

## Shared Patterns

### Shell driver skeleton (`set -eu`, err(), HARVEY check, TMPD trap)
**Source:** `build/aarch64/subset-build.sh:20-32,73-77` + `subset-exec.sh:148-160`
**Apply to:** tier-build.sh, tier-exec.sh, coverage-sweep.sh, both gethunk repro scripts, the x86_64 CI leg script

```sh
#!/bin/sh
set -eu

HARVEY="${HARVEY:-$PWD}"
err() { echo "error: $*" >&2; exit 1; }
[ -d "$HARVEY/sys/src" ] || err "$HARVEY does not look like the harvey checkout (missing sys/src)"

TMPD=$(mktemp -d "${TMPDIR:-/tmp}/<name>.XXXXXX")
cleanup() {
	rm -rf "$TMPD"
}
trap cleanup EXIT
```

### Platform split (darwin lima / linux native-unshare)
**Source:** `build/aarch64/subset-build.sh:34-67`
**Apply to:** tier-build.sh, tier-exec.sh, coverage-sweep.sh (every script that enters the namespace build)

```sh
NATIVE_LINUX=
if [ "$(uname -s)" = "Linux" ]; then
	NATIVE_LINUX=1
fi
if [ -n "$NATIVE_LINUX" ]; then
	export HARVEY            # nsrun.sh --wrap re-sources `export` output in-namespace
	_hb_arch=$(uname -m); [ "$_hb_arch" = "arm64" ] && _hb_arch=aarch64
	HARVEY_LINUX_BIN="${HARVEY_LINUX_BIN:-$HARVEY/.build/hostbin-linux-$_hb_arch/bin}"
	export HARVEY_LINUX_BIN
	export FORCERCFORMK=1 CGO_ENABLED=0 objtype=amd64 cputype=unix
else
	command -v limactl >/dev/null 2>&1 || err "limactl not found — run inside the nix devshell"
	# … lima ensure-running …
fi
```

### Marker-based gating (never trust exit status)
**Source:** `subset-build.sh:193-201` + `subset-exec.sh:473-492`
**Apply to:** every gate this phase lands

```sh
set +e
<the run> > "$LOG" 2>&1
RC=$?
set -e
cat "$LOG"
if [ $RC -ne 0 ]; then
	echo "error: <run> failed (rc=$RC); log: $LOG" >&2
	exit $RC
fi
"$GREP" -q '<MARKER-OK>' "$LOG" || err "log lacks <MARKER-OK>"
# … one presence-grep per marker; presence never interleaving order …
if "$GREP" -qiE 'panic|trap:' "$LOG"; then
	err "captured log contains a kernel fault line"
fi
```

### a.out magic assertion (od idiom)
**Source:** `subset-build.sh:241-242`
**Apply to:** tier artifact markers, USPACE-03 `.7`-magic CI scan, self-host canary fallback

```sh
magic=$(od -An -tx1 -N4 "$f" | tr -d ' \n')
[ "$magic" = "00008c47" ] || err "$f: bad magic $magic (want 00008c47)"   # R_MAGIC .7 aarch64
```

### mk namespace addressing ($HARVEY, never /build)
**Source:** RESEARCH-verified namespace mechanics (`BUILD_SETUP_NAMESPACE`/`nsrun.sh` bind sets)
**Apply to:** all mk recipes + guest scripts touching tier data

```sh
# in mk backquotes / guest scripts:
$HARVEY/build/aarch64/tiers/tier0-libs.txt     # RIGHT — host-absolute, live under unshare -m
/build/aarch64/tiers/tier0-libs.txt            # WRONG — /build is never bound in the namespace
```

### Plan 9 C edit style (Wave-0 C fixes)
**Source:** `sys/man/6/style` conventions + the surrounding files
**Apply to:** macbody, 6l/obj.c, aarch64/include/u.h, libdynld rename

- Tabs only; `/* … */` block comments only; automatics never initialized at declaration.
- Match the file's own tighter historical style (macbody's `(char*)mysbrk` cast stays; 6l's bare `mysbrk` stays).
- Additive edits to shared frontend sources; amd64 green is sacred (gate: host toolchain rebuild + 9k build + `mk checkall`).

## No Analog Found

Files with no close match in the codebase — the planner should use RESEARCH.md's verified formats directly (all three are new committed-data conventions whose *consumers* have exact analogs above):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `build/aarch64/tiers/tier{0,1}.txt` | committed data | batch | First committed plain-data allowlist in the repo; convention fixed by D-13 (one entry per line, `#` comments, sys/src-relative forms). Closest in-spirit: the SUBSET_FLAT/SUBSET_NAMED lists at subset-exec.sh:98-99 (as code, not data). |
| `build/aarch64/markers/tier{0,1}.ledger` | committed data (append-only) | batch | `build/aarch64/gate-evidence.log` is the spiritual ancestor but is gitignored transient evidence; a committed machine-readable ledger is new. Format fixed by D-14 + RESEARCH §Ledger line format. |
| `build/aarch64/generator-audit.txt` + `dir-coverage.txt` | committed reports | batch | First CI-regenerated-and-diffed committed reports. Line formats specified in RESEARCH §Generator Audit + §Dir-Coverage Report (fixed field order, diff-stable). |

## Metadata

**Analog search scope:** `sys/src/cmd/{cc,6l,7l,ip,mkfile,mkone,mkmany,mksyslib}`, `sys/src/{mkfile,mkfile.proto,games/mkfile}`, `{aarch64,amd64}/include/u.h`, `sys/src/libdynld/`, `build/aarch64/` (full listing + 9 scripts read), `build/mm/`, `build/scripts/linux-build`, `nix/`, `flake.nix`, `.github/workflows/`
**Files scanned:** 30 (24 read in full or targeted sections; 6 grepped)
**Tracked-source gate:** all analog paths verified via `git ls-files` — no gitignored mirrors substituted
**Pattern extraction date:** 2026-11-04
