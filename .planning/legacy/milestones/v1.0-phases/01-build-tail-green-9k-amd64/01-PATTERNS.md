# Phase 1: Build-Tail Green (9k amd64) - Pattern Map

**Mapped:** 2026-10-30
**Files analyzed:** 18 (14 modified, 4 created)
**Analogs found:** 16 / 18

This phase is almost entirely **edit-in-place against existing build machinery** — most "analogs" are the target files themselves plus one proven sibling idiom (awk's host-generator mkfile pattern). The dominant pattern work is: (1) normalize broken generator mkfiles onto the awk idiom, (2) extend existing preflight/export blocks in `nix/*.sh`, (3) mechanical renames in rc/sh scripts, (4) two net-new artifacts (policy doc, minimega harness) with weak or no in-tree analogs.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `sys/src/cmd/jpg/mkfile` (mod) | build config (mkfile) | compile-and-run codegen | `sys/src/cmd/awk/mkfile` | exact (same idiom, working) |
| `sys/src/cmd/jtagfs/mkfile` (mod) | build config (mkfile) | compile-and-run codegen | `sys/src/cmd/awk/mkfile` + `sys/src/cmd/spell/mkfile` | exact |
| `sys/src/cmd/mkfile` (mod — add `checkall`) | build config (mkfile) | batch recursion driver | itself (`dirs:V:` loop) + `sys/src/mkfile` | exact |
| Whack-a-mole tail: `sys/src/cmd/*/mkfile`, vendored C (`gs/**`, `games/mahjongg/**`, …), toolchain C (`sys/src/cmd/cpp/*`) | build config / C source | batch compile | per-file; first known: `sys/src/cmd/4a/mkfile` (itself) | n/a — D-04 tail, fix per-file using shared patterns below |
| `nix/build.sh` (mod — KERNDATE export + preflight) | host orchestration (bash) | sequential pipeline | itself (existing export + preflight blocks) | exact |
| `nix/vm-build.sh` (mod — KERNDATE through ssh) | host orchestration (bash) | sequential pipeline (rsync/ssh) | itself | exact |
| `sys/src/9k/k10/mkfile` (mod — KERNDATE line 20) | build config (kernel mkfile) | batch compile/link | itself + `sys/src/9k/mk/portmkfile` | exact |
| `sys/src/9k/mk/portmkfile` (mod — KERNDATE line 103) | build config (mk include) | batch compile | itself | exact |
| `build/scripts/build` (mod — rename `build_arm64`, lines 76-80) | rc orchestration script | sequential arch loop | itself (`fn build_amd64` sibling) | exact |
| `build/scripts/linux-build` (mod — rename lines 63-67; maybe narrow `arch=(386 amd64)` line 131) | rc orchestration script | sequential arch loop | itself | exact |
| `build/mkdirs` (mod — drop `./arm64/bbin` line 264) | host setup (bash) | batch mkdir | itself (array-entry removal) | exact |
| `sys/src/BUILD_SETUP_NAMESPACE` (mod — drop/rename arm64 stanza lines 23-24) | namespace setup (sh) | sequential bind-mounts | itself (per-arch stanza shape) | exact |
| `dist/replica/plan9.proto` (mod — arm64→aarch64 lines 38-39) | replica config | declarative tree spec | itself (sibling arch stanzas) | exact |
| `flake.nix` (mod — optional smoke app registration) | nix config | declarative packaging | itself (`mkScriptApp`/`mkApp` blocks, lines 37-72) | exact |
| `build/REPRODUCIBILITY.md` (new — BUILD-04 policy doc) | documentation | — | `BUILD_ON_UNIX` / `build/README` (weak — repo docs are plain text) | partial |
| `build/mm/boot-9k10cpu.mm` (new — minimega script) | VM orchestration DSL | VM lifecycle | none (only in-tree `.mm` is troff `sys/lib/tmac/strings.mm` — unrelated) | none → use RESEARCH.md Pattern 5 |
| `build/mm/` or `nix/` capture+assert wrapper (new bash) | host orchestration (bash) | stream capture + assert | `nix/setup.sh`, `nix/build.sh` | role-match |

## Pattern Assignments

### `sys/src/cmd/jpg/mkfile` + `sys/src/cmd/jtagfs/mkfile` (mkfile, compile-and-run codegen)

**Analog:** `sys/src/cmd/awk/mkfile` — the in-tree blessed host-generator idiom. Secondary: `sys/src/cmd/spell/mkfile`.

The invariant (RESEARCH Pattern 1): **never derive a tool name from `/$cputype/mkfile`** — instead recurse with `objtype=$cputype`, which resolves `CC=9c LD=9l O=o` via `/unix/mkfile` (`unix/mkfile:3-6`: `CC=9c`, `LD=9l`, `O=o`, `AS=9a`), producing a host-runnable binary with plan9port's compiler.

**Working generator idiom** (`sys/src/cmd/awk/mkfile:42-50`):
```mk
proctab.c:	$cputype.maketab
	./$cputype.maketab >proctab.c

$cputype.maketab:	y.tab.h maketab.c
	objtype=$cputype mk maketab.$cputype

maketab.$objtype:V:
	$CC $CFLAGS maketab.c
	$LD $LDFLAGS -o $objtype.maketab maketab.$O
```

**spell variant** (`sys/src/cmd/spell/mkfile:11,24-29`) — derives only the *output-object suffix* (`cpuobjtype`), never a tool name:
```mk
cpuobjtype=`{sed -n 's/^O=//p' /$cputype/mkfile}

$cpuobjtype.pcode:	pcode.c $HFILES
	objtype=$cputype
	mk pcode.$cputype

pcode.$cputype:	pcode.$O $HFILES
	$LD -o $O.pcode pcode.$O
```

**Broken idiom being replaced** (`sys/src/cmd/jpg/mkfile:45,61-69` — do not propagate):
```mk
c=`{sed -n 's/^O=//p' /$cputype/mkfile}		# → o under cputype=unix
ycbcr.h:	rgbycc.c
	$c^c rgbycc.c					# → oc (does not exist)
	$c^l -o $c.rgbycc rgbycc.$c
	$c.rgbycc > ycbcr.h
```
Same breakage at `sys/src/cmd/jtagfs/mkfile:5,47-49` (`Oo=...` → `$Oo^c` → `oc`).

**Cleanup-idiom requirement:** generated files must land in `nuke`/`clean` targets — copy jpg's existing `nuke-headers` pattern (`sys/src/cmd/jpg/mkfile:71-74`) and awk's `rm -f ... *.maketab` (`awk/mkfile:37,40`).

**Standard mkfile skeleton to preserve** (every cmd mkfile — `jpg/mkfile:1-44` shape):
```mk
</$objtype/mkfile

TARG=...
OFILES=...$O\
HFILES=...
BIN=/$objtype/bin
UPDATE=\
	mkfile\
	...

</sys/src/cmd/mkmany		# or mkone for single-target dirs
```

---

### `sys/src/cmd/mkfile` (mod — add `checkall` report-all target, D-03)

**Analog:** the file's own `dirs:V:` loop + `sys/src/mkfile:53-59` recursion pattern.

**Existing strict loop to leave untouched** (`sys/src/cmd/mkfile:33-38`):
```mk
dirs:V:
	for(i in cc $DIRS) @{
		cd $i
		echo mk $i
		mk $MKFLAGS all
	}
```

**rc-loop idiom for the new sibling target** — rc blocks inside mk use `@{...}`; per-iteration status capture without aborting follows the rc pattern already used elsewhere in this file (`compilers:V:` target, lines 125-139, uses `if(! ~ $i cc rc) @{...}` guards). New target shape per RESEARCH Pattern 3:
```mk
checkall:V:
	fail=()
	for(i in cc $DIRS) @{
		cd $i
		if(! mk $MKFLAGS all)
			fail=($fail $i)
		cd ..
	}
	if(! ~ $#fail 0) {
		echo 'checkall failures: '^$"fail
		exit checkall
	}
```
(Exact rc syntax tuned at execution; keep `:V:` virtual-target marking and tab indentation consistent with the file.)

---

### `nix/build.sh` (mod — KERNDATE export + userland preflight)

**Analog:** the file's own env-export block and preflight loop.

**Env export pattern** (`nix/build.sh:4-14`) — new `KERNDATE` export lands here:
```bash
HARVEY="${HARVEY:-$PWD}"
export HARVEY
_hb_os=$(uname -s); _hb_os=${_hb_os,,}
_hb_arch=$(uname -m); [ "$_hb_arch" = "arm64" ] && _hb_arch=aarch64
export HARVEY_LINUX_BIN="${HARVEY_LINUX_BIN:-$HARVEY/.build/hostbin-$_hb_os-$_hb_arch/bin}"
export PATH="$HARVEY_LINUX_BIN:$PATH:/rc/bin:."
export objtype=amd64
export cputype=unix
export FORCERCFORMK=1
```
Note: this file already contains the `arm64`→`aarch64` host-mapping idiom (line 7) — the legitimate-keeper use of the string `arm64`.

**Preflight pattern** (`nix/build.sh:16-21`) — the kernel-input assertion (RESEARCH Pattern 2) follows this exact shape:
```bash
for t in pcc cpp ar; do
  if [ ! -x "$HARVEY_LINUX_BIN/$t" ]; then
    echo "error: $HARVEY_LINUX_BIN/$t missing — run 'nix run .#setup' then 'nix run .#toolchain' first" >&2
    exit 1
  fi
done
```
New assertion goes after `bash "$HARVEY/build/mkdirs"` (line 33) / before the namespace entry, checking `$HARVEY/amd64/bin/{ip/ipconfig,auth/factotum,fossil/fossil,venti/venti}` per RESEARCH.

**Header-comment convention** (`nix/build.sh:1-3`): leading `#` comment stating purpose + "Wrapper provides:" — new wrapper scripts must carry this.

---

### `nix/vm-build.sh` (mod — KERNDATE through the ssh line)

**Analog:** itself. KERNDATE must be computed **host-side** (rsync excludes `.git`, line 37) and exported through the ssh command line (`nix/vm-build.sh:43`):
```bash
ssh -F "$SSH_CFG" "$GUEST" "bash -lc 'set -e; cd \"$GUEST_DIR\" && nix run .#setup && nix run .#toolchain && nix run .#build'"
```
Pattern to copy for error handling (`nix/vm-build.sh:14-17`):
```bash
if [ ! -d "$HARVEY/sys/src" ]; then
  echo "error: $HARVEY does not look like the harvey checkout (missing sys/src)" >&2
  exit 1
fi
```

---

### `sys/src/9k/k10/mkfile` + `sys/src/9k/mk/portmkfile` (mod — KERNDATE env honoring)

**Analog:** the two existing `-DKERNDATE` sites themselves.

**Current sites:**
- `sys/src/9k/k10/mkfile:20`:
```mk
$p$CONF:	$CONF.c $OBJ $LIB
	$CC $CFLAGS '-DKERNDATE='`{date -n} $CONF.c
```
- `sys/src/9k/mk/portmkfile:102-103`:
```mk
$CONF.$O:	$CONF.c
	$CC $CFLAGS '-DKERNDATE='`{date -n} $CONF.c
```

**Replacement idiom** (rc substitution inside mk recipe, per RESEARCH Pattern 4):
```mk
	$CC $CFLAGS '-DKERNDATE='`{if(~ $#KERNDATE 0) KERNDATE=`{date -n}; echo $KERNDATE} $CONF.c
```
Both sites get the identical change. Backtick-rc substitution `` `{...} `` is the established mkfile idiom (see `sys/src/cmd/mkfile:3,16-17`, `jpg/mkfile:45`).

---

### `build/scripts/build` + `build/scripts/linux-build` (mod — `build_arm64`→`build_aarch64` rename; maybe narrow `arch` loop)

**Analog:** sibling `fn build_*` functions in the same files — the rename must match the established function shape exactly.

**Function idiom** (`build/scripts/linux-build:63-67`, rename target):
```rc
fn build_arm64{
	for(i in /sys/src/libc) {cd $i && mk nuke && mk install && mk clean}
	for(i in /sys/src/cmd/7?) {cd $i && mk nuke && mk install && mk clean}
	cd /sys/src && objtype=arm64 {mk nuke && mk install && mk clean}
}
```
→ becomes `fn build_aarch64{ ... objtype=aarch64 ... }` (correct-but-dormant until Phase 2 per D-05). Same edit at `build/scripts/build:76-80`.

**Arch loop + status-check idiom** (`build/scripts/linux-build:131-155`) — site of the possible `arch=(amd64)` narrowing (Open Question 2):
```rc
arch=(386 amd64)
for(i in $arch){
	echo build $i
	mkdir -p /$i/bin/auth
	...
	build_$i
	if(! ~ $status '')
		exit build
}
```
rc conventions to preserve: `fn name{` (no space before `{`), `if(! ~ $status '')` failure test, `mk CONF'='$i` quoting.

---

### `build/mkdirs`, `sys/src/BUILD_SETUP_NAMESPACE`, `dist/replica/plan9.proto` (mod — arm64 quarantine, D-05)

**Coupling rule (Pitfall 3):** all three edits **plus** root `arm64/` removal land in **one commit** — `mount --bind` fails on a missing source.

**`build/mkdirs`** — bash array-entry removal (`build/mkdirs:261-272`):
```bash
	./bbin \
	./amd64/bbin \
	./386/bbin \
	./arm64/bbin \		# ← drop this line
	./arm/bbin \
	...
)
for i in "${lst[@]}"; do
	mkdir -p $i
done
```

**`sys/src/BUILD_SETUP_NAMESPACE`** — per-arch stanza shape (`sys/src/BUILD_SETUP_NAMESPACE:20-30`); drop lines 23-24 (recommended per RESEARCH: Phase 2 adds `/aarch64` wiring when the arch is real):
```sh
mkdir -p /arm
mount --bind $HARVEY/arm /arm

mkdir -p /arm64			# ← drop stanza
mount --bind $HARVEY/arm64 /arm64	# ← drop stanza

mkdir -p /amd64
mount --bind $HARVEY/amd64 /amd64
```

**`dist/replica/plan9.proto`** — arch stanza shape (`dist/replica/plan9.proto:32-39`); rename to `aarch64` or drop per planner's call:
```
amd64	d775 sys sys
	+	d775 sys sys
arm64	d775 sys sys		# ← rename/drop
	+	d775 sys sys		# ← rename/drop
```

---

### Whack-a-mole tail: cmd mkfiles + vendored/toolchain C (D-04)

No upfront file list exists (locked decision D-04: fix current blocker, rerun). Apply these per-file patterns as each blocker surfaces:

**For mkfile blockers:** apply the awk generator idiom above. Grep watchlist for the whole class: `grep -ln 'cputype' sys/src/cmd/*/mkfile` (currently: awk, gs, jpg, jtagfs, spell — awk/spell/gs survivable, jpg/jtagfs broken).

**For C source fixes (vendored or toolchain):** follow project conventions (AGENTS.md / `sys/man/6/style`):
- Tabs only; return type on its own line; `nil` not `NULL`; `exits("msg")` not `exit()`; block comments `/* */` only — no `//`
- Error idiom: return `-1` + `werrstr`/`%r`; fatal via `sysfatal("...: %r", ...)`; diagnostics `fprint(2, "cmd: ...: %r\n")`
- No `#ifndef` header guards in Plan 9 headers; no new POSIX includes in native code
- Match the file being edited where historical code diverges from style(6)

**D-02 toolchain-first diagnostic:** before patching vendored C (gs, antiword, apeg, mahjongg), preprocess with a reference cpp and compare — Harvey's own cpp is a known bug source (`build/Dockerfile:13` TODO). Fix toolchain bugs in `sys/src/cmd/cpp`, vendored bugs in the vendored source.

**Known first blocker:** `sys/src/cmd/4a/mkfile` (full file read — 19 lines, standard `mkone` shape; `lex.$O` depends on `../cc/macbody ../cc/lexbody ../cc/compat`). Its `rfork bind lex.c` recipe failure is a generator-adjacent failure inside the cc/4a shared-lexer machinery, not a `$cputype` derivation.

---

### `flake.nix` (optional mod — register smoke app)

**Analog:** the existing app-registration blocks (`flake.nix:37-72`).

**Script-app pattern** (`flake.nix:12-25, 56-65`):
```nix
mkScriptApp = pkgs: name: file: extraHeader: runtimeInputs:
  pkgs.writeShellApplication {
    inherit name runtimeInputs;
    text = ''
      set -euo pipefail
      export PLAN9="${pkgs.plan9port}/plan9"
      # plan9 bin LAST per BUILD_ON_UNIX
      export PATH="$PATH:$PLAN9/bin"
      ${extraHeader}
      ${builtins.readFile file}
    '';
  };
...
buildPkg = if isLinux
  then mkScriptApp pkgs "harvey-build" ./nix/build.sh ''
    export GNU_SED="${pkgs.gnused}/bin/sed"
  '' [ pkgs.plan9port pkgs.bash pkgs.util-linux pkgs.gnused pkgs.mount pkgs.binutils ]
```
A smoke app = one more `mkScriptApp` binding + one `mkApp` entry in the `in { ... }` attrset (lines 66-71). Linux-only gating follows the `isLinux` conditional pattern.

---

### `build/REPRODUCIBILITY.md` (new — BUILD-04 policy doc)

**Analog (weak):** `BUILD_ON_UNIX` and `build/README` — repo docs are terse plain text, not structured markdown. Markdown is acceptable (`.planning/` uses it; the doc is new-style tooling documentation), but keep it terse and factual like the existing docs. Must contain (per RESEARCH Pattern 4): scope (9k kernels), derivation (`git log -1 --format=%ct`, host-side), override (`KERNDATE=<epoch>` env), exception (`conffile` embeds absolute checkout path — same-path reproducibility only), verification (two clean builds, identical `sha256sum` of `amd64/9k10cpu*`).

---

### `build/mm/boot-9k10cpu.mm` + capture wrapper (new — BUILD-05)

**No in-tree analog.** Use RESEARCH.md §Pattern 5 directly (all mechanisms verified against minimega 3.3.0 source):
```
vm config memory 512
vm config kernel /path/to/amd64/9k10cpu.elf
vm config serial-ports 1
vm config backchannel false
vm launch kvm k10cpu
vm start k10cpu
```
Capture wrapper follows the `nix/*.sh` bash conventions above (header comment, `HARVEY` default, stderr+`exit 1` errors). Banner assertion: `grep -q 'Plan 9' boot.log` — grounded at `sys/src/9k/k10/main.c:182-184` (`i8250console("0");` unconditional, `print("\nPlan 9\n")`).

## Shared Patterns

### mkfile conventions (apply to ALL mkfile edits)
**Source:** `sys/src/cmd/awk/mkfile`, `sys/src/cmd/mkfile`, `sys/src/9k/k10/mkfile`
- First line always `</$objtype/mkfile`; includes use `<` / `<| awk ...` (`k10/mkfile:11-13`)
- Virtual targets marked `:V:` (or `:VQ:` when quiet); recipes are **rc**, tab-indented
- rc blocks: `@{...}` for subshell grouping; `` `{...} `` for command substitution; `$"var` to flatten lists
- Tool variables from the arch include: `$CC $CFLAGS`, `$LD $LDFLAGS`, `$O`, `$AS` — never hardcode `6c`/`6l` in cmd mkfiles (arch mkfiles like k10's may)
- Generated files must be removable via `clean`/`nuke` targets

### Host-generator rule (apply to any compile-and-run-during-build step)
**Source:** `sys/src/cmd/awk/mkfile:42-50`, `unix/mkfile:3-6`
Generators build via `objtype=$cputype mk <tool>.$cputype` → plan9port `9c`/`9l` → host-runnable binary. Never derive tool names from `sed`/`grep` of `/$cputype/mkfile` (`$c^c`, `$Oo^c` are the banned idioms).

### bash host-script conventions (apply to `nix/*.sh` edits + new capture wrapper)
**Source:** `nix/build.sh:1-21`, `nix/setup.sh:1-8, 20-24`, `nix/vm-build.sh:14-17`
- Leading `#` comment: purpose + "Wrapper provides:" (env injected by `flake.nix` `mkScriptApp`, which supplies `set -euo pipefail` — don't re-add)
- `HARVEY="${HARVEY:-$PWD}"` default; os/arch detection + `arm64`→`aarch64` host mapping where needed
- Errors: `echo "error: ..." >&2; exit 1`
- `$PLAN9/bin` stays LAST in PATH (AGENTS.md constraint; flake wrapper already enforces)

### rc build-script conventions (apply to `build/scripts/*` edits)
**Source:** `build/scripts/linux-build`, `build/scripts/build`, `build/scripts/funcs`
- Shebang `#!/bin/rc -e` where present; `fn name{` (no space); `{cd $i && mk nuke && mk install && mk clean}` chained with `&&`
- Failure propagation: `if(! ~ $status '') exit build`
- Deferred-arch pattern: keep dormant functions correct-but-unreferenced (matches commented `arch=(...)` variants in `funcs:34-41`)

### Preflight-over-exit-status (apply to build-gate additions)
**Source:** RESEARCH Pitfall "Looks Done But Isn't"; `nix/build.sh:16-21`
Strict mk loops skip the tail silently on abort — assert **files exist** (`[ -x path ]`), never trust exit status alone.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `build/mm/boot-9k10cpu.mm` | VM orchestration DSL | VM lifecycle | No minimega/qemu config scripts in-tree; the only `.mm` file (`sys/lib/tmac/strings.mm`) is troff macros. Use RESEARCH.md Pattern 5 verbatim. |
| `build/REPRODUCIBILITY.md` | policy doc | — | Repo docs are plain text (`BUILD_ON_UNIX`, `build/README`); no markdown policy-doc precedent in-tree. Content specified by RESEARCH Pattern 4. |

## Metadata

**Analog search scope:** `sys/src/cmd/{awk,spell,jpg,jtagfs,4a}/mkfile`, `sys/src/cmd/mkfile`, `sys/src/mkfile`, `nix/*.sh`, `flake.nix`, `build/{mkdirs,scripts/*}`, `sys/src/{BUILD,BUILD_SETUP_NAMESPACE}`, `sys/src/9k/{k10/mkfile,mk/portmkfile}`, `unix/mkfile`, `dist/replica/plan9.proto`, `BUILD_ON_UNIX`
**Files scanned:** 18 read in full or at cited ranges; all analog paths verified git-tracked via `git ls-files`
**Pattern extraction date:** 2026-10-30
