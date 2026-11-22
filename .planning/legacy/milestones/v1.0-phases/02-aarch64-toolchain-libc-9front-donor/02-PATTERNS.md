# Phase 2: aarch64 Toolchain + libc (9front donor) - Pattern Map

**Mapped:** 2026-10-31
**Files analyzed:** 30 (15 new/vendored, 14 modified, 1 deletion set)
**Analogs found:** 30 / 30 (14 exact, 12 role-match/self, 4 partial — all partials have verbatim donor or research-verified content in 02-RESEARCH.md)

**Tracked-source gate:** every analog path below was verified git-tracked via `git ls-files` this session. No mirror/install paths used. All analogs are in-tree Harvey sources at repo root `/Users/neilskinner/code/harvey`.

**Naming reminder (D-01/D-06):** in-tree paths and docs use `aarch64`; donor content and libmach internals keep `arm64`. Tool letters stay `7a/7c/7l`. The rename mapping is: `libc/arm64` → `sys/src/libc/aarch64`, `arm64/include/` → `aarch64/include/`, and the single in-file path rewrite `/arm64/include/ureg.h` → `/aarch64/include/ureg.h` (in vendored `libmach/7.c`).

## File Classification

### New / vendored files

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `sys/src/cmd/7a/` (vendored sources + `a.h` shims) | assembler (toolchain) | batch (asm in → `.7` object out) | `sys/src/cmd/6a/` | exact |
| `sys/src/cmd/7a/BUILD` | host build script | batch | `sys/src/cmd/6a/BUILD` | exact |
| `sys/src/cmd/7a/mkfile` | target mk config | batch | `sys/src/cmd/6a/mkfile` | exact |
| `sys/src/cmd/7a/README` | provenance doc | n/a | `sys/src/cmd/unix/9cc/README` | role-match |
| `sys/src/cmd/7c/` (vendored sources + `math.h`, `gc.h`, OROL shims) | compiler (toolchain) | batch (C in → asm out) | `sys/src/cmd/6c/` | exact |
| `sys/src/cmd/7c/BUILD` | host build script | batch | `sys/src/cmd/6c/BUILD` | exact |
| `sys/src/cmd/7c/mkfile` | target mk config | batch | `sys/src/cmd/6c/mkfile` | exact |
| `sys/src/cmd/7c/README` | provenance doc | n/a | `sys/src/cmd/unix/9cc/README` | role-match |
| `sys/src/cmd/7l/` (vendored sources + `l.h` MAXIO shim) | linker (toolchain) | batch (objects → `7.out`) | `sys/src/cmd/6l/` | exact |
| `sys/src/cmd/7l/BUILD` | host build script | batch | `sys/src/cmd/6l/BUILD` | exact |
| `sys/src/cmd/7l/mkfile` | target mk config | batch | `sys/src/cmd/6l/mkfile` | exact |
| `sys/src/cmd/7l/unixcompat.c` | host-compat shim (new file) | batch | `sys/src/cmd/6l/unixcompat.c` | exact |
| `sys/src/cmd/7l/README` | provenance doc | n/a | `sys/src/cmd/unix/9cc/README` | role-match |
| `sys/src/cmd/cc/compat.h` | shared-frontend header (additive) | n/a | synthesized; content verified verbatim in 02-RESEARCH.md §Pattern 2 item 1 | partial (no in-tree analog — Harvey has `cc/compat` + `cc/compat.c` but no header) |
| `sys/src/libmach/7.c`, `7db.c`, `7obj.c` | library (machine support) | transform (object/executable introspection) | `sys/src/libmach/{6.c,6db.c,6obj.c}` | role-match (content verbatim from donor; wiring analogs in-tree) |
| `sys/src/libc/aarch64/` (20 donor files + mkfile + README) | per-arch libc library | batch | `sys/src/libc/amd64/` | exact |
| `sys/src/libc/9sys/callmain.c` | library (startup glue) | request-response | donor verbatim; coherence analog `sys/src/libc/9sys/privalloc.c` | partial |
| `aarch64/mkfile` | objtype config | n/a | `amd64/mkfile` | exact |
| `aarch64/include/u.h`, `ureg.h` | per-arch headers (ABI freeze) | n/a | `amd64/include/u.h`, `ureg.h` | role-match (content verbatim from donor; `ureg.h` quoted verbatim in 02-RESEARCH.md §Code Examples) |
| host `nm` BUILD script (`sys/src/cmd/nm` gains a `BUILD`) | host build script | batch | `sys/src/cmd/unix/9cc/BUILD` (libmach under host compiler) + `sys/src/cmd/6a/BUILD` (shape) | partial |
| donor-kernel qemu harness script(s) (e.g. `build/aarch64/boot-9qemu.sh` + serial assert) | test harness | event-driven (serial capture → grep assert) | `build/mm/boot-9k10cpu.sh` + `build/mm/boot-9k10cpu.mm` | role-match (port pattern to plain qemu; minimega is x86-only) |

### Modified files (all self-analog: the pattern to copy is already inside the file)

| Modified File | Role | Edit | In-file Pattern Source |
|---------------|------|------|------------------------|
| `sys/src/libmach/mkfile` | mk config | remove `alpha\`(22), `alphadb\`(33), `alphaobj\`(42), dep rule (69); add `7\`,`7db\`,`7obj\` + `7obj.$O: 7c/7.out.h` | lines 18/32/41 (`4`/`4db`/`4obj`) and dep rule line 66 (`4obj.$O: 4c/4.out.h`) |
| `sys/src/libmach/setmach.c` | machine table | delete `"alpha"` entry (121–126) + `malpha`/`alphamach` externs (20, 22); add `marm64`/`arm64mach` externs + entry | `"amd64"` entry lines 97–102 |
| `sys/src/libmach/executable.c` | exec-format table | add `R_MAGIC`/FARM64 exectab entry + `case FARM64:` in `commonboot` | amd64 exectab entry lines 186–194; `case FAMD64:` lines 430–435 |
| `sys/src/libmach/obj.c` | object-parser table | add `[ObjArm64] "arm64 .7", _is7, _read7,` | line 61 (`[ObjAmd64]`); `_is7`/`_read7` decls already at lines 27/39 |
| `sys/include/mach.h` | header enums | delete 5 Alpha enums: `MALPHA,`(35), `FALPHA,`(64), `FALPHAB,`(65), `AALPHA,`(91), `ObjAlpha,`(110); keep `MARM64`(40), `FARM64`/`FARM64B`(72–73), `AARM64`(95), `ObjArm64`(115) | surrounding entries; optional comment fix `/* .4? */` → `/* .7 */` at line 115 |
| `sys/src/libc/9syscall/mkfile` | codegen mkfile | add `case aarch64` block | existing `case amd64` lines 81–95 + donor `case arm64` quoted verbatim in 02-RESEARCH.md §Pattern 4 |
| `sys/src/libc/9sys/mkfile` | mk config | add `callmain.$O` to OFILES | OFILES list lines 4–54 (alphabetical, `$O`-suffixed) |
| `sys/src/cmd/pcc.c` | driver table | add `{"aarch64", "7c", "7l", "7", "7.out"},` + `'7'` in `allos` | table lines 14–27; `allos` line 28 |
| `sys/src/mkfile.proto` | mk config | `CPUS` += `aarch64` (line 6, **LAST, post-gate**); planner: also evaluate adding `7` to `OS=568ijqv` (line 5) — mkone/mkmany regex `[$OS]` must match `.7` objects | lines 5–6 |
| `build/mkdirs` | dir-list config | add `./aarch64/bbin` (and `./aarch64/bin*`, `./aarch64/lib` set as needed) | `bbin` block lines 261–266; `./amd64/*` block lines 32–49 |
| `build/scripts/funcs` | build config | optionally add `aarch64` to `arch=(386 amd64)` (line 37) | line 37 + commented variants 34–41 |
| `build/scripts/linux-build` | build config | add `aarch64` to `arch=(amd64)` (line 133) → activates existing `build_aarch64` (lines 63–67, already correct) | `fn build_arm` lines ~44–61; `fn build_aarch64` 63–67 |
| `sys/src/BUILDTOOLCHAIN` | host orchestration | add `(cd $HARVEY/sys/src/cmd/7a && sh BUILD)` etc. | lines 4–11 |
| `sys/src/cmd/cc/mkfile` | mk config | (discretion, post-build only) add `7c` to `CURCC` | line 26 |

### Deleted files (D-04 Alpha excision)

`sys/src/libmach/alpha/` (`7.out.h`, `ureg.h`), `alpha.c`, `alphadb.c`, `alphaobj.c`, committed stale artifacts `alpha.6 alpha.8 alphadb.6 alphadb.8 alphaobj.6 alphaobj.8`. `git rm`; no analog needed.

### Automatic integration points (NO edit needed — planner should not create tasks)

- `sys/src/cmd/mkfile` — `DIRS` (line 17) auto-discovers `7a/7c/7l` via `ls -l`; `compilers:V` (lines 144–158) globs `?c` so `7c` is picked up automatically.
- `sys/src/libc/mkfile` — `all install:` (line 8) iterates `$PORTDIRS $objtype`; `libc/aarch64` is picked up automatically once `objtype=aarch64` exists. `installall` (line 43) iterates `$CPUS`.

---

## Pattern Assignments

### `sys/src/cmd/7c/BUILD` + `7a/BUILD` + `7l/BUILD` (host build scripts, batch)

**Analogs:** `sys/src/cmd/6c/BUILD` (compiler), `sys/src/cmd/6a/BUILD` (assembler), `sys/src/cmd/6l/BUILD` (linker)

**Core pattern — compiler (6c/BUILD, entire file, lines 1–54):**
```bash
rm -f *.o

(cd ../cc; $PLAN9/bin/yacc -D1 -d cc.y)
9c -g -I /../../../../include -I . -I ../cc  \
		-O0 \
		-Wno-parentheses \
		-Wno-missing-braces \
		-Wno-switch \
		-Wno-comment \
		-Wno-sign-compare \
		-Wno-unknown-pragmas \
		-Wno-misleading-indentation \
		-Wno-stringop-truncation \
		-Wno-stringop-overflow \
		-Wno-format-truncation \
		-fno-omit-frame-pointer \
		-fsigned-char \
		-fcommon \
		-Wno-char-subscripts  \
		-Wno-builtin-declaration-mismatch \
		-Wno-implicit-function-declaration \
		-Wno-int-conversion \
		-Wno-maybe-uninitialized \
		-Wno-incompatible-pointer-types \
		-DRunemask=0x1FFFFF \
	../cc/acid.c \
	../cc/bits.c \
	../cc/com64.c \
	../cc/com.c \
	../6l/unixcompat.c \
	../cc/dcl.c \
	../cc/dpchk.c \
	../cc/funct.c \
	../cc/lex.c \
	../cc/mac.c \
	../cc/pgen.c \
	../cc/pickle.c \
	../cc/pswt.c \
	../cc/scon.c \
	../cc/sub.c \
	../cc/y.tab.c \
	cgen.c \
	div.c \
	enam.c \
	list.c \
	machcap.c \
	mul.c \
	peep.c \
	reg.c \
	sgen.c \
	swt.c \
	txt.c \
&& 9l -o 6c -static -g *.o
cp 6c $HARVEY_LINUX_BIN
```
For 7c: drop `div.c`, keep the rest of the shared `../cc/*.c` list; per-tool sources become the vendored 7c set (`cgen.c enam.c list.c machcap.c mul.c peep.c reg.c sgen.c swt.c txt.c`). **Do not pass `-static`** — it is neutralized by the `9l` shim in `nix/toolchain.sh` (lines 23–35 drop `-static` because modern Apple ld and nix glibc both lack static libc). New BUILDs may simply omit it.

**Assembler (6a/BUILD, entire file, lines 1–4):**
```bash
$PLAN9/bin/yacc -D1 -d a.y
9c -O0 -Wno-maybe-uninitialized -g -I . *.c
9l -o 6a *.o
cp 6a $HARVEY_LINUX_BIN
```

**Linker (6l/BUILD, entire file, lines 1–15):**
```bash
#!/bin/sh
9c -O0 -Wno-char-subscripts -g -I . asm.c list.c obj.c optab.c pass.c span.c unixcompat.c ../6c/enam.c ../ld/elf.c

9l -o 6l \
asm.o \
enam.o \
elf.o \
list.o \
obj.o \
optab.o \
pass.o \
span.o \
unixcompat.o 

cp 6l $HARVEY_LINUX_BIN
```
For 7l: vendored sources + `unixcompat.c` + generated `cnam.c` (run `sh mkcname` in BUILD — see generator note below). Harvey links statically; the donor's `dyn.c` is droppable (carried-forward decision).

**Generator note (from 02-RESEARCH.md §Pattern 2):** commit the generated `7c/enam.c` (6c precedent — `sys/src/cmd/6c/enam.c` is committed and 6c/mkfile has no enam.c rule); generate `7l/cnam.c` per-build via `sh mkcname` (host `ed` + sh proven this session). Use POSIX-safe invocations in BUILD scripts — hostbin prepends Plan 9 `ls`/`sed`/`date`, and macOS BSD sed treats `\t` as literal `t` (use `perl -pi -e` or literal tabs for shim edits).

---

### `sys/src/cmd/7l/unixcompat.c` (host-compat shim, new file)

**Analog:** `sys/src/cmd/6l/unixcompat.c` (entire file, 154 lines)

**Core pattern — hunk-backed malloc family (lines 1–57):**
```c
#include	"l.h"

/*
 * fake malloc
 */
void*
malloc(ulong n)
{
	void *p;

	while(n & 7)
		n++;
	while(nhunk < n)
		gethunk();
	p = hunk;
	nhunk -= n;
	hunk += n;
	return p;
}

void
free(void *p)
{
	USED(p);
}

void*
calloc(ulong m, ulong n)
{
	void *p;

	n *= m;
	p = malloc(n);
	memset(p, 0, n);
	return p;
}

void*
realloc(void*_, ulong __)
{
	fprint(2, "realloc called\n");
	abort();
	return 0;
}

void*
mysbrk(ulong size)
{
	return sbrk(size);
}

void
setmalloctag(void *v, ulong pc)
{
	USED(v);
	USED(pc);
}
```
Plus the `my*` host-OS wrappers (`myaccess`/`mycreat`/`mywait`/`mydup`/`mypipe`/`systemtype`/`pathchar`/`mygetwd`/`myexec`/`myfork`, lines 59–154) — copy the file shape wholesale.

**Critical (Pitfall A):** `gethunk` must call `mysbrk` (→ `sbrk`), NEVER `malloc`. Model in-tree: `sys/src/cmd/6l/obj.c:1291–1302` (`gethunk(void)` → `h = mysbrk(nh);`). The malloc→gethunk→malloc recursion was reproduced live in research (macOS stack-guard segfault).

---

### `sys/src/cmd/7a/` `a.h` shims + `7c/math.h` + `7c/gc.h` (vendored-file shim pattern)

**Analogs:** `sys/src/cmd/6a/a.h`, `sys/src/cmd/6c/math.h`, `sys/src/cmd/6c/gc.h`

**`math.h` SDK shadow — copy verbatim from `sys/src/cmd/6c/math.h` (entire file, 2 lines):**
```c
// This is ONLY here for building on unix.
double modf(double, double *);
```
(This existing `//` comment is grandfathered vendored-tool style; do not introduce new `//` comments elsewhere per style(6).)

**`7a/a.h` constant + decl shims — patterns from `sys/src/cmd/6a/a.h`:**
```c
/* line 21 — BUFSIZ: use this, not IOUNIT */
#define	BUFSIZ		(16*1024)

/* lines 112–131 — hunk-allocator EXTERN block (storage via lex.c:1 #define EXTERN empty) */
EXTERN	char*	hunk;
EXTERN	long	nhunk;
EXTERN	long	thunk;

/* lines 134, 163, 172 — matching function decls */
void*	allocn(void*, long, long);
void	macexpand(Sym*, char*);      /* 2-arg Harvey form, not 9front's 3-arg */
void	gethunk(void);
```

**`7c/gc.h` swcmp decl — match `sys/src/cmd/6c/gc.h:273`:**
```c
int	swcmp(const void*, const void*);
```
(Harvey's definition lives at `sys/src/cmd/cc/pswt.c:4`; 9front's 7c gc.h ships the stale non-const form.)

**OROL excision (vendored 7c only, D-13):** delete the `rolor(n);` call (donor `sgen.c:191`), the `case OROL:` label line (donor `cgen.c:202` — shares body with `case OAND:`/`case OOR:`), and the OROL case block (donor `txt.c:1054–1062`). Never touch shared `cc.h`/`sub.c` — the frontend carries Phase 1's `NVAR=BITS*32` LP64 fix (`sys/src/cmd/cc/cc.h:43`).

---

### `sys/src/cmd/7a/mkfile`, `7c/mkfile`, `7l/mkfile` (target mk configs)

**Analogs:** `sys/src/cmd/6a/mkfile`, `sys/src/cmd/6c/mkfile`, `sys/src/cmd/6l/mkfile`

**Assembler shape (6a/mkfile, entire file, lines 1–26):**
```
</$objtype/mkfile

TARG=6a
OFILES=\
	y.tab.$O\
	lex.$O\

HFILES=\
	../6c/6.out.h\
	y.tab.h\
	a.h\

YFILES=a.y\

BIN=/$objtype/bin
UPDATE=\
	mkfile\
	a.y\
	lex.c\
	a.h\
	${TARG:%=/386/bin/%}\

< /sys/src/cmd/mkone
YFLAGS=-D1 -d

lex.$O:	../cc/macbody ../cc/lexbody ../cc/compat
```

**Compiler shape (6c/mkfile, lines 1–36) — key idioms:** `TARG`, `OFILES` with `$O` suffix, `HFILES` including `../cc/cc.h`, `LIB=../cc/cc.a$O`, `</sys/src/cmd/mkone>`, and the two catch-all rules:
```
$LIB:
	cd ../cc
	mk install
	mk clean

%.$O: ../cc/%.c
	$CC $CFLAGS ../cc/$stem.c
```

**Linker shape (6l/mkfile, lines 1–35) — cross-directory source idiom:**
```
CFLAGS=$CFLAGS -. -I.
CFILES=${OFILES:%.$O=%.c}
CFILES=${CFILES:enam.c=../6c/enam.c}
...
enam.$O:	../6c/enam.c
	$CC $CFLAGS ../6c/enam.c
```

---

### `sys/src/libmach/` wiring (mkfile, setmach.c, executable.c, obj.c, mach.h)

**Analogs:** self-analogs — existing per-arch entries in the same files.

**`setmach.c` — table entry pattern (amd64 entry, lines 97–102):**
```c
	{	"amd64",			/*amd64*/
		FAMD64,
		FAMD64B,
		AAMD64,
		&mamd64,
		&i386mach,	},
```
New entry (donor-verbatim per 02-RESEARCH.md §Pattern 3 site 4): `{ "arm64", /*ARM64*/ FARM64, FARM64B, AARM64, &marm64, &arm64mach, },` — **the libmach name stays `arm64`** (D-01: rename-on-import applies to tree paths, not to libmach's internal arch names, which upstream nm/acid users type). Add `marm64` to the `extern Mach` list (lines 19–20) and `arm64mach` to the `extern Machdata` list (lines 21–22); delete `malpha`/`alphamach` and the `"alpha"` entry (lines 121–126).

**`executable.c` — exectab entry pattern (amd64, lines 186–194):**
```c
	{ S_MAGIC,			/* amd64 6.out & boot image */
		"amd64 plan 9 executable",
		"amd64 plan 9 dlm",
		FAMD64,
		1,
		&mamd64,
		sizeof(Exec)+8,
		nil,
		commonllp64 },
```
New entry (donor-verbatim, quoted in 02-RESEARCH.md §Pattern 3 site 6): same shape with `R_MAGIC` / `FARM64` / `&marm64`. `commonllp64` already exists (line 473). Also delete the two Alpha entries (lines 240–257: `L_MAGIC` + `0x0700e0c3`).

**`executable.c` — `commonboot` boot-image case pattern (lines 430–435):**
```c
	case FAMD64:
		fp->type = FAMD64B;
		fp->txtaddr = fp->entry;
		fp->name = "amd64 plan 9 boot image";
		fp->dataddr = _round(fp->txtaddr+fp->txtsz, 4096);
		break;
```
Add the donor's `case FARM64:` (same shape, `_round(...)` per donor executable.c:372–377); delete `case FALPHA:` (lines 418–423).

**`obj.c` — object-parser table pattern (line 61):**
```c
	[ObjAmd64]	"amd64 .6",	_is6, _read6,
```
Add `[ObjArm64] "arm64 .7", _is7, _read7,` (donor obj.c:59); `_is7`/`_read7` decls already present (lines 27/39) — they were Alpha's, now served by vendored `7obj.c`. Delete `[ObjAlpha] "alpha .7", _is7, _read7,` (line 63).

**`mach.h` enum edits:** delete exactly `MALPHA,			/* retired */` (35), `FALPHA,			/* retired */` (64), `FALPHAB,		/* retired DEC Alpha bootable */` (65), `AALPHA,			/* retired */` (91), `ObjAlpha,		/* retired */` (110). Enum-position shift is safe (in-memory selectors only, never persisted — Pitfall C; full rebuild after excision). Optional cosmetic: `ObjArm64,		/* .4? */` → `/* .7 */` (line 115).

**`libmach/mkfile` edits:** remove `alpha\` (22), `alphadb\` (33), `alphaobj\` (42), `alphaobj.$O: alpha/7.out.h` (69); add `7\`, `7db\`, `7obj\` to FILES and `7obj.$O: 7c/7.out.h` after line 66's `4obj.$O: 4c/4.out.h` pattern. `CFLAGS=$CFLAGS -I/sys/src/cmd` (line 63) already makes vendored `7obj.c`'s `#include "7c/7.out.h"` resolve.

**Vendored `7.c` path rewrite (the only one in the import set):** `7.c:9` `#include "/arm64/include/ureg.h"` → `#include "/aarch64/include/ureg.h"` (D-11).

---

### `sys/src/libc/aarch64/` (per-arch libc, vendored)

**Analog:** `sys/src/libc/amd64/` (mkfile shape + startup/lock asm conventions)

**mkfile shape (amd64/mkfile, entire file, lines 1–42):**
```
objtype=amd64
</$objtype/mkfile

LIB=/$objtype/lib/libc.a
SFILES=\
	argv0.s\
	atom.s\
	cycles.s\
	getfcr.s\
	main9.s\
	main9p.s\
	memccpy.s\
	...
	tas.s\

CFILES=\
	_seek.c\
	getcallerpc.c\
	notejmp.c\

HFILES=/sys/include/libc.h

OFILES=${CFILES:%.c=%.$O} ${SFILES:%.s=%.$O}

UPDATE=mkfile\
	$HFILES\
	$CFILES\
	$SFILES\

</sys/src/cmd/mksyslib
```
The imported aarch64 mkfile is the donor's verbatim (quoted in 02-RESEARCH.md §Pattern 5) with **one edit: `objtype=arm64` → `objtype=aarch64`** on line 1. Donor set: 16 `.s` files + `lock.c notejmp.c _seek.c`.

**Startup convention differs from amd64 — import the matched pair, don't hybridize (02-RESEARCH.md Don't-Hand-Roll row 5).** amd64's `main9.s` (entire file, lines 1–19) shows the contrast — amd64 uses `inargc/inargv` externs and inline `_privates`:
```asm
#define NPRIVATES	16

TEXT	_main(SB), 1, $(2*8+NPRIVATES*8)
	MOVQ	AX, _tos(SB)
	LEAQ	16(SP), AX
	MOVQ	AX, _privates(SB)
	MOVL	$NPRIVATES, _nprivates(SB)
	MOVL	inargc-8(FP), RARG
	LEAQ	inargv+0(FP), AX
	MOVQ	AX, 8(SP)
	CALL	main(SB)
```
9front's arm64 `main9.s` instead ends `B _callmain(SB)` — which is why `sys/src/libc/9sys/callmain.c` is a required additive import (defines `_privates`/`_nprivates` consumed by `sys/src/libc/9sys/privalloc.c:8–9`). Add `callmain.$O` to `9sys/mkfile` OFILES (alphabetical, after `announce.$O` line 7). Inert on amd64 (no references → archive member never pulled).

**`_seek.c` wrapper pattern** — identical shape to `sys/src/libc/amd64/_seek.c` (entire file, lines 1–14); the donor's version pairs with the `_seek` stub rename in the 9syscall generator.

---

### `sys/src/libc/9syscall/mkfile` — `case aarch64` (codegen, ABI freeze D-15)

**Analogs:** in-file `case amd64` (lines 81–95) for Harvey's switch idiom; donor `case arm64` (9front mkfile:110–117, quoted verbatim in 02-RESEARCH.md §Pattern 4) for the exact instruction sequence.

**Harvey switch idiom + shared quirks (lines 5–9, 81–95):**
```
	SYS=`{sed '/^#define._X[123]/d; s/#define.([A-Z0-9_]*).*/\1/' sys.h}
	for(I in $SYS) {
		i=`{echo $I|tr A-Z a-z}
		n=`{sed -n '/[ 	]'$I'[ 	]/s/.*	//p' sys.h}
		if(~ $i exits) i=_exits
		{switch($objtype){
		...
		case amd64
			if(~ $i seek)
				echo TEXT _seek'(SB)', 1, '$0'
			if not
				echo TEXT $i'(SB)', 1, '$0'
			if(! ~ $i nsec)
				echo MOVQ RARG, 'a0+0(FP)'
			echo MOVQ '$'$n, RARG
			echo SYSCALL
			echo RET
```

**New `case aarch64`** — the donor arm64 block with only the case-label rename:
```
	case aarch64
		j=$i
		if(~ $i seek) j=_seek
		echo TEXT $j'(SB)', 1, '$0'
		echo MOV R0, '0(FP)'
		echo MOV '$'$n, R0
		echo SVC
		echo RETURN
```
Freeze wording (02-RESEARCH.md §Code Examples, corrected): **number in R0; args in 8-byte slots at user SP+8 (SP+0 reserved LR slot); SVC; result in R0.** The stub's `MOV R0, 0(FP)` materializes arg0 into the first slot the kernel reads. `seek`→`_seek` rename pairs with `libc/aarch64/_seek.c`; no stub-side buffer store (kernel always writes the buffer — Pitfall G). NSEC=53 is zero-arg register-return on 64-bit; Harvey's generated `nsec` stub is wire-correct. NMOUNT=54 (sys.h:53) is generated but must stay uncalled on the donor kernel (D-09 nm gate).

---

### `aarch64/mkfile` + `pcc.c` + `mkfile.proto` + `mkdirs` (objtype wiring, TC-04 — post-gate)

**Analog:** `amd64/mkfile` (entire file, lines 1–6):
```
</sys/src/mkfile.proto

CC=6c
LD=6l
O=6
AS=6a
```
The aarch64 file is byte-identical with `CC=7c LD=7l AS=7a O=7` (donor's 5-line file with rename, quoted in 02-RESEARCH.md §Code Examples).

**`pcc.c` table pattern (lines 14–28):**
```c
Objtype objtype[] = {
	{"spim",	"0c", "0l", "0", "0.out"},
	...
	{"amd64",	"6c", "6l", "6", "6.out"},
	...
};
char	*allos = "05689kqvij";
```
Add `{"aarch64",	"7c", "7l", "7", "7.out"},` and add `7` to `allos` (else pcc rejects `.7` files, line ~152 check). Note the line-13 comment "sync with /sys/src/ape/cmd/cc.c" — APE is out of scope this phase; record, don't do.

**`mkfile.proto` (lines 5–6):** `CPUS=amd64 arm 386 riscv riscv64 power mips` — append `aarch64` **last, after the execution gate** (Pitfall D / riscv64 vaporware rule). Planner: also evaluate `OS=568ijqv` (line 5) — the `[$OS]` regexes in `sys/src/cmd/mkfile` (lines 61–67, 106) and `mkone`/`mkmany` must match `.7` objects for in-tree aarch64 builds; riscv64's `j` is present, so `7` likely needs adding.

**`mkdirs`:** add `./aarch64/bbin` (pattern: lines 261–266) and the `./aarch64/bin{,/auth,/aux,...}` + `./aarch64/lib` set following the `./amd64/*` block (lines 32–49).

**`build/scripts/linux-build`:** `build_aarch64` already exists and is correctly named (lines 63–67):
```rc
fn build_aarch64{
	for(i in /sys/src/libc) {cd $i && mk nuke && mk install && mk clean}
	for(i in /sys/src/cmd/7?) {cd $i && mk nuke && mk install && mk clean}
	cd /sys/src && objtype=aarch64 {mk nuke && mk install && mk clean}
}
```
Activation = adding `aarch64` to `arch=(amd64)` (line 133) per the TC-04 interpretation (02-RESEARCH.md Open Question 1 / Assumption A1).

---

### host `nm` BUILD (new — D-09 gate dependency)

**Analogs:** `sys/src/cmd/unix/9cc/BUILD` (compiling libmach for the host) + `sys/src/cmd/6a/BUILD` (install shape)

**9cc/BUILD core pattern (lines 7–39) — the libmach-under-host-compiler proof:**
```bash
case "`uname -s`" in
	Darwin)	OSTYPE=darwin ;;
	Linux)	OSTYPE=linux ;;
	*)	echo "9cc: unsupported host OS `uname -s`"; exit 1 ;;
esac

ROOT=`pwd`
IFLAGS="-I$ROOT/include -I$ROOT/lib9/$OSTYPE -I$ROOT -I$ROOT/cmd"
...
# libmach (needs cmd/*.out.h via $ROOT)
(cd libmach && $CC -c $CFLAGS *.c && ar rv $OBJDIR/libmach.a *.o && rm -f *.o)

# iar
(cd iar && $CC $CFLAGS -o iar ar.c posix.c $OBJDIR/libmach.a $OBJDIR/libbio.a $OBJDIR/lib9.a -lm && \
	cp iar $HARVEY_LINUX_BIN/ar)
```
Two routes for host nm (planner picks): (a) 9c/9l against Harvey's in-tree libmach (now including 7.c/7db.c/7obj.c) + plan9port bio — research Assumption A3, LOW-MEDIUM risk; (b) host `cc` against the vendored 9cc tree's include/lib9/libbio + Harvey's libmach sources (the 9cc precedent above). Either way install to `$HARVEY_LINUX_BIN/nm`. `sys/src/cmd/nm.c` needs only `<u.h> <libc.h> <ar.h> <bio.h> <mach.h>` (lines 4–8). Fallback if host libmach shims balloon: target-built nm run on the donor kernel (research Open Question 3).

---

### `sys/src/cmd/7{a,c,l}/README` (provenance docs, D-10)

**Analog:** `sys/src/cmd/unix/9cc/README` (entire file, lines 1–23) — the shape to copy:
```markdown
# Vendored subset of 9-cc (https://github.com/0intro/9-cc @ 634e7371,
# "master" as of 2022-04-04), the pieces Harvey needs to build `iar` ...
#
# Layout mirrors upstream, minus everything unrelated to iar:
#   include/    shared headers ...
#
# Local modifications vs upstream:
#   * lib9/getcallerpc.c — single portable version using
#     __builtin_return_address; replaces per-platform asm variants
#   ...
```
Each README records: upstream URL + pin SHA (`350266104f1fed81c78c3d05fa44f523bec0ae4d`, branch `front`, 2026-10-31), what was imported, and an enumerated local-modifications list (the shim set for that dir). License credits: Bell Labs/Lucent, Vita Nuova, C H Forsyth, 9front (MIT-compatible).

---

### donor-kernel qemu harness (test harness, event-driven)

**Analog:** `build/mm/boot-9k10cpu.sh` (247 lines) + `build/mm/boot-9k10cpu.mm` — port the *discipline*, not the minimega mechanics (minimega is x86/KVM-only; aarch64 uses plain qemu).

**Patterns to carry over (boot-9k10cpu.sh):**
- Header contract block (lines 1–44): usage, env overrides with defaults, boot-proof statement, mechanics notes, explicit scope boundary.
- `set -euo pipefail` + `err()` helper (lines 46–55).
- Precondition checks on artifacts before launching (lines 57–58).
- Acceleration probe pattern (lines 79–92) — becomes: TCG pinned (`-accel tcg -cpu cortex-a72 -M virt,gic-version=2`); HVF empirically rejected for the GICv2-only donor kernel (qemu 11.1.2: `HVF does not support GICv2 emulation`).
- `cleanup()` + `trap cleanup EXIT` killing the VM/capture and removing tmpdirs (lines 104–119).
- **Capture-before-start ordering** (lines 218–227): attach serial capture (`-serial mon:stdio` redirected, or a chardev socket + socat) BEFORE the guest runs; the banner prints within milliseconds.
- **Banner grep is the source of truth, not the timer** (lines 229–245): deadline loop `grep -q` on the capture log; on miss, `tail` the log and exit 1 with a "do not retry silently" hint.
- Boot budget env override (`MM_BOOT_BUDGET`-style; TCG default ~600s).

**qemu invocation (research-verified):** `qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -accel tcg -smp 1 -nographic -m 1G -kernel 9qemu.u -serial mon:stdio`. Donor kernel built **in the pinned donor tree** (never in Harvey's repo — D-07) inside the lima VM Linux mount namespace, with `$HARVEY_LINUX_BIN` (our 7a/7c/7l) on PATH and `objtype=arm64` *inside the donor tree*. Namespace-build precedent: `sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE` (unshare -m pattern, 5 lines). Binary delivery recommendation (research Open Question 2): bake into `$CONF.bootfs.paq` for the gate; document ufs-over-tcp as the iteration path.

---

## Shared Patterns

### Host-tool build (applies to: 7a/7c/7l BUILD, nm BUILD, BUILDTOOLCHAIN wiring)
**Sources:** `sys/src/cmd/6{c,a,l}/BUILD`, `nix/toolchain.sh:9,23–35`
- plan9port `9c`/`9l`, `$PLAN9/bin` LAST in PATH; install via `cp <tool> $HARVEY_LINUX_BIN` where `HARVEY_LINUX_BIN="${HARVEY}/.build/hostbin-<os>-<arch>/bin"` (toolchain.sh:9).
- Never `-static` (shimmed out anyway); `-fsigned-char -fcommon -O0 -g` + the long `-Wno-*` list is the proven flag set.
- Wire new BUILDs into `sys/src/BUILDTOOLCHAIN` following lines 4–11: `(cd $HARVEY/sys/src/cmd/7a && sh BUILD)`.
- BSD-sed `\t` trap: use `perl -pi -e` or literal tabs for any shim edits inside BUILD scripts.

### mk idiom (applies to: all new/edited mkfiles)
**Sources:** `amd64/mkfile`, `sys/src/libc/amd64/mkfile`, `sys/src/cmd/6{c,a,l}/mkfile`, `sys/src/mkfile.proto`
- First line `</$objtype/mkfile` (or `</sys/src/mkfile.proto` for arch-root mkfiles).
- Object suffix is `$O` never `.o`; lists use trailing-backslash continuation with tabs.
- Libraries end with `</sys/src/cmd/mksyslib`; single programs with `</sys/src/cmd/mkone`; multi-program dirs `</sys/src/cmd/mkmany`.
- Standard targets everywhere: `all install clean nuke update installall`.

### Rename-on-import (applies to: every vendored donor path)
**Source:** CONTEXT D-06/D-11, verified by donor grep in research
- Directory names: `libc/arm64` → `libc/aarch64`, `arm64/include` → `aarch64/include`.
- In-file absolute paths: `/arm64/include/ureg.h` → `/aarch64/include/ureg.h` (exactly one occurrence, `libmach/7.c:9`).
- NOT renamed: tool letters (`7a/7c/7l`), libmach internal names (`"arm64"` setmach entry, `marm64`, `arm64mach`, `ObjArm64`), and `objtype=arm64` **inside the donor tree** during the D-07 harness build.

### Vendor-and-own shim discipline (applies to: all vendored dirs)
**Source:** CONTEXT D-13, Phase 1 D-02
- All drift fixes land in vendored files or the one new additive header (`cc/compat.h`); Harvey's shared `cc.h`/`sub.c`/`com.c`/`dcl.c` are untouched (LP64 fix `NVAR=BITS*32` at cc.h:43 protects amd64 green).
- Toolchain-first diagnostic rule: vendored code that built for decades and now fails → suspect our cpp/compiler/host shims first.
- Each shim enumerated in the dir's README local-modifications list.

### Execution-gated ordering (applies to: plan wave structure)
**Source:** CONTEXT D-06, carried-forward riscv64 vaporware gate
- Strict order: tools build → libc compiles → hello links → **hello executes on donor kernel** → only then `aarch64/` dirs, `pcc.c`, `mkdirs`, CPUS.
- Phase-exit: shell subset executes on donor kernel + host `nm` asserts zero `NMOUNT` references (D-09).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `sys/src/cmd/cc/compat.h` | frontend header | n/a | Harvey has `cc/compat` (source) + `cc/compat.c` but no header; content fully synthesized and trial-verified in 02-RESEARCH.md §Pattern 2 item 1 (note `mysbrk(ulong)` — wrong signature is a hard error) |
| `sys/src/libc/9sys/callmain.c` | startup glue | n/a | No `_callmain` anywhere in Harvey (grep-verified); donor file imports verbatim; coherence target `9sys/privalloc.c:8–9` read this session |
| `aarch64/include/u.h`, `ureg.h` | arch headers | n/a | Content is donor-verbatim (ureg.h quoted in full in 02-RESEARCH.md §Code Examples); in-tree `amd64/include/` provides structural shape only |
| host `nm` BUILD | host script | batch | No host BUILD produces nm today; closest patterns are 9cc/BUILD (libmach host compile) and 6a/BUILD (shape) — combined above |

## Metadata

**Analog search scope:** `sys/src/cmd/{6a,6c,6l,8a,cc,unix/9cc}`, `sys/src/libmach`, `sys/src/libc/{amd64,9sys,9syscall}`, `sys/include`, `sys/src/{BUILDTOOLCHAIN,mkfile.proto,RUN_BUILD_IN_PRIVATE_NAMESPACE}`, `amd64/`, `build/{mkdirs,mm,scripts}`, `nix/toolchain.sh`
**Files scanned:** 28 (all read in full or targeted non-overlapping ranges; all `git ls-files`-verified tracked)
**Pattern extraction date:** 2026-10-31
