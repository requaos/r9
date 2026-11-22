---
last_mapped_commit: bdcf45a51dced761eec09538b5f0786ff81e0399
last_mapped_at: 2026-11-13
---
# Coding Conventions

**Analysis Date:** 2026-11-13

Two convention families coexist in this repo:

1. **Plan 9 heritage code** (`sys/src/`, `sys/include/`, `rc/`, `cfg/`) — governed by `sys/man/6/style`. Style authority is that man page; "if the file you are editing already deviates, do what it does."
2. **Revival-era tooling** (`build/x86_64/`, `build/aarch64/`, `build/mm/`, `nix/`, `.github/workflows/`, `build/harness/`) — POSIX sh drivers, expect scripts, in-guest rc sweeps, and Rust host tools, each with their own strict in-house patterns (marker vocabulary, computed expectations, provenance comments).

## Naming Patterns

**Files (C tree):**
- All lowercase, no underscores: `bread.c`, `devcons.c`, `sysfatal.c`
- One function (or small tightly-related group) per library file: `sys/src/libbio/` has `binit.c`, `bflush.c`, `brdline.c`, each containing the function it is named after
- Commands are single files named after the command (`sys/src/cmd/cat.c`, `cp.c`, `tail.c`); multi-file commands get a directory (`sys/src/cmd/rio/`, `sys/src/cmd/fossil/`)
- Headers named after their library: `sys/include/bio.h`, `sys/include/thread.h`, `sys/include/9p.h`
- Kernel devices: one file per device named `dev<name>.c` (`sys/src/9k/port/devcons.c`, `devpipe.c`, `devmnt.c`)
- Objects never use `.o`: `$O` expands to the arch char (`6`=amd64, `8`=386, `5`=arm, `7`=aarch64), so `bread.$O` → `bread.6`

**Files (revival-era tooling):**
- Venue directories: `build/x86_64/`, `build/aarch64/` (the 64-bit ARM arch is always `aarch64`, never `arm64`)
- Gate drivers: `<name>.expect` (expect), `<name>-sweep.rc` (in-guest), `<phase>-r9-x64.sh` / drivers in `nix/`
- Data files: `build/x86_64/tiers/tier{0,1,2,3}*.txt` — append-only plain data
- Ledgers: `build/x86_64/tier{0,1,2,3}.ledger`, `build/aarch64/markers/*.ledger`

**Functions:**
- C: all lowercase, no underscores, often concatenated words: `copy1`, `dirstat`, `sysfatal`, `waserror`, `poperror`, `threadmain`
- Library prefix carried: libbio → `Bread`, `Binit`, `Bprint`; libdraw → `drawsetdebug` (see `sys/include/bio.h`)
- Static helpers same rule: `samefile` in `sys/src/cmd/cp.c`
- Rust (`build/harness/`, `../r9`): snake_case, `load_sysh`, `resolve` (`build/harness/src/trace.rs`)
- sh drivers: `err()` fatal helper, `cleanup()` EXIT-trap helper, `stage()` / `svc_stage()` / `payload_block()` per-script helpers (`build/x86_64/mkimage.sh`, `nix/tier1-r9-x64.sh`)

**Variables:**
- C locals: all lowercase, short (`dirb`, `todir`, `bout`, `rcount`); flag variables suffixed `flag` (`gflag`, `uflag`, `xflag` in `sys/src/cmd/cp.c`); globals plain (`failed`, `count`)
- Automatic variables are never initialized at declaration (style(6) rule; assign on a separate line — some old files violate, follow the file)
- rc sweeps: venue/prefix vars (`venue`, `B=/bin`, `D=/lib/tier1`); `checko_*`/`*out` result captures (`build/x86_64/tier1-sweep.rc`)
- sh drivers: `HARVEY`, `OUT`, `STAGE`, `TMPD`, `GREP` (uppercase env-adjacent); `_`-prefixed loop-private vars (`_wfail`, `_md5`) in `build/x86_64/mkimage.sh`
- expect: `spawn_pid`, `run_tag`, `QEMU`, `R9_MACHINE` (`build/x86_64/tier1-r9.expect`)

**Types:**
- `struct` tags and matching `typedef`s are Uppercase: `typedef struct Biobuf Biobuf;` (`sys/include/bio.h`), `QLock`, `Ref`, `Dir`, `Rune`, `Chan`, `Dev`
- `enum` / `#define` constants are Uppercase or UPPERCASE: `Binactive`, `Bsize`, `UTFmax`, `DMDIR`, `OREAD`, `ERRMAX`
- Rust: `TestStep`, `BootTestStep` (CamelCase types), `PINNED_FIELDS` const arrays (`../r9/xtask/src/uregcheck.rs`)

**Markers (test vocabulary — a first-class naming convention):**
- `T1-OK <name>` / `T1-FAIL <name>` / `T1-SKIP <name>` / `T1-DONE` — in-guest tier-1 sweep markers (`build/x86_64/tier1-sweep.rc`)
- `SVC-OK/SVC-FAIL/SVC-SKIP/SVC-DONE` — services-probe markers (`build/x86_64/services-sweep.rc`)
- `T0-ART <lib> <sha256>`, `T1-ART`, `T2-ART`, `T3-ART` — build-artifact markers; `TIER-BUILD-OK` (`build/x86_64/subset-build.sh`)
- `COVERAGE-VISIT` / `COVERAGE-EXIT <key> <status>` — coverage-shim protocol (`build/x86_64/coverage-sweep.sh`)
- `T1-XDOBS` — diagnostic echo variants are allowed but never gate

## Code Style

**C formatting (style(6), no automated formatter):**
- Tabs only, never expanded to spaces. No clang-format/indent config exists; `sys/src/cmd/cb/` is an in-tree beautifier but is never run automatically
- No whitespace before opening braces; none after `if`, `for`, `while`: `if(n < 0){`, `for(i=0; i<argc; i++){`
- Function return type on its own line, name at column 0:
```c
void
main(int argc, char *argv[])
{
```
- No braces around single-line bodies; opening `{` on the same line as `if`/`for`
- Surround binary operators with spaces; parenthesize arithmetic/bit-wise mixes, otherwise parenthesize lightly
- Standard idiom direction: `x < 0`, never `0 > x`
- Never `!strcmp(...)` or `if(memcmp(a,b,c))` — always compare explicitly with a relational operator (`if(strcmp(a,b) == 0)`)
- Block comments `/* ... */` only; `//` is acceptable only to comment out a few lines of code; never introduce new `//` comments
- No `#ifndef` header guards in Plan 9 headers — the compilers tolerate multiple inclusion; do not add guards

**sh drivers (`set -eu` POSIX sh):**
- Open with `#!/bin/sh` then a header comment block stating identity, phase/decision provenance (`D-xx`, `Pitfall N` references), contract, and usage
- `err() { echo "error: ..." >&2; exit 1; }` fatal helper; loud named failures with remediation text ("run sh build/x86_64/subset-build.sh first")
- `trap cleanup EXIT` with a temp-dir `mktemp -d` + `rm -rf`
- Resolve a host grep once: `GREP=$(command -v /usr/bin/grep || command -v grep)` — plan9port grep lacks `-q/-x`/`-E`, but a hardcoded path breaks NixOS hosts (`build/x86_64/mkimage.sh:120`, `nix/tier1-r9-x64.sh:91`)
- plan9port `$PLAN9/bin` goes LAST on PATH (project rule from `BUILD_ON_UNIX`; re-asserted in `build/x86_64/mkimage.sh:128-132`)
- Use `od`, not host `dd`, for byte inspection — Harvey's host dd shadows POSIX dd on the devshell PATH (`build/x86_64/mkimage.sh:250-253`)

**expect gates (`build/x86_64/*.expect`):**
- `#!/usr/bin/expect -f`; header comment: env contract (REQUIRED vars), usage, exit-code meaning (0 pass / 1 named-stage fail / 2 contract violation)
- `set timeout 600` (TCG-sized), `match_max 1048576`; keep `log_user` ON (expect 5.45 writes nothing to `log_file` when it is 0)
- Fresh capture per run: caller `rm -f`s the log, driver removes + reopens `log_file` itself (`build/x86_64/tier1-r9.expect:79-98`)
- `spawn` qemu with `-name $run_tag` (unique per run); `proc cleanup` kills qemu's process group (`kill -KILL -- -$spawn_pid`) on every exit path; caller's EXIT trap sweeps survivors via bracket-fixed `pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}"` — never a kernel-name class (`nix/tier1-r9-x64.sh:199-213`)
- `stage <name> <regex>` proc asserts ordered boot phases, each naming its failure on stderr; `expect_before { -re {panic|Unhandled interrupt} }` fast-fails on kernel panic (`build/x86_64/tier1-r9.expect:115-132`)
- Assertions are presence-only, never interleaving order (preemption/TCG make order meaningless); patterns are CRLF-tolerant substrings, never end-of-line anchored — except whole-file post-pass `\n` anchors which prevent prefix false-matches (`tier1-r9.expect:167-177`)

**rc sweeps (in-guest, `build/x86_64/tier1-sweep.rc`, `services-sweep.rc`):**
- `#!/bin/rc` with a header documenting markers, venue usage, and the rc-lexer discipline
- `check` fn: run fixed input under `if()`; failure emits `T1-FAIL` and the sweep CONTINUES — a failing entry never wedges or shortens the sweep
- No bare `=` in match patterns (rc lexer discipline); venue-mapped paths are caret-concatenated (`'digest '^$D^/sentinel.txt`), never embedded literals
- Backquote bodies stay simple-command pipelines (the rc lexer rejects subshells with separators inside backquotes — use a `fn` producer instead, see `uniqin` at `tier1-sweep.rc:171`)
- `if not` takes a SIMPLE command only — braced `if not {` bodies are a parse error that kills a sourced file mid-stream (`build/x86_64/services-sweep.rc:49-53`); use flat status-variable ladders
- Every command the sweep runs carries the `$B/h*` prefix (h-prefixed staged copies) except the boot chain's unprefixed embeds (rc, echo, bind)
- Bounded spin waits, never unguarded sleeps: `for(i in 1..10){ if(test -e X) ok=1; if not sleep 1 }` (`tier1-sweep.rc:319-322`)

**Rust (`build/harness/`, sibling `../r9`):**
- `build/harness`: plain `cargo` bin crate; modules per concern (`trace.rs`, `console.rs`, `drive9.rs`); pure parsing cores are separated from live drivers specifically so they can be unit-tested
- `../r9`: `rustfmt.toml` sets `edition = "2024"`, `use_small_heuristics = "Max"`, `newline_style = "Unix"`; rustfmt is authoritative there
- `no_std` kernel code; host-side tests live in `port/src/*_tests.rs` wired via `#[cfg(test)] mod x_tests;` in `port/src/lib.rs:86-114`

**Comment conventions (all files):**
- C: one or two lines above a function describing what it does; comment anything taking >2 seconds to understand, explaining WHY; no JSDoc/Doxygen machinery anywhere — documentation lives in man pages under `sys/man/`
- Revival-era files carry long header comment blocks that record provenance: which phase/task/decision (`D-xx`, `Pitfall N`), which precedent file a shape was "re-keyed" from, and the measured facts that motivated a rule. This is deliberate institutional memory — new drivers follow it (see `build/x86_64/mkimage.sh:1-104`, `nix/tier1-r9-x64.sh:1-79`)
- "Re-key, don't reinvent": when porting a discipline from the aarch64 venue to x86_64 (or vice versa), copy the proven script's shape and re-key paths — cite the source file in the header

## Import Organization

**C (user space):**
```c
#include <u.h>
#include <libc.h>
```
**C (kernel):**
```c
#include	"u.h"
#include	"../port/lib.h"
#include	"mem.h"
#include	"dat.h"
#include	"fns.h"
#include	"../port/error.h"
```
- No path aliases; includes resolve via `-I` into `/sys/include` and `/$objtype/include`; source-relative paths (`../port/lib.h`) used directly

**Rust:** normal crate imports; `build/harness` uses `use std::collections::HashMap` style; no path aliases configured

**sh:** drivers source nothing except their own helpers; `build/scripts/*` rc scripts use `. funcs` (`build/scripts/build:6`)

## Error Handling

**User-space C:**
- Integer-valued functions return `-1` on error, 0 or positive on success (style(6))
- Failing functions set the error string; callers print it with the `%r` verb: `fprint(2, "cp: can't stat %s: %r\n", from)` (`sys/src/cmd/cp.c:95`)
- Fatal: `sysfatal("can't open %s: %r", argv[i]);` — prints and exits (`sys/src/libc/9sys/sysfatal.c`)
- Non-fatal: `fprint(2, ...)` diagnostics, set a flag (`failed`), end with `exits("errors")` (`sys/src/cmd/cp.c:46-48`)
- Exit via `exits(0)` / `exits("message")`, NEVER `exit()`; nil/0 arg = success, string = failure
- Bad invocation: print `usage: ...` to stderr, `exits("usage")`, conventionally via `goto usage;` at the bottom of `main` (`sys/src/cmd/cp.c:50-54`)
- Do not invent return-code enums; `-1`/non-negative plus errstr only

**Kernel C:**
- `error(Esomething)` longjmp-style raise; `waserror(){...} poperror()` guards cleanup; centralized strings in `sys/src/9k/port/error.h`; `panic()` for unrecoverable state

**sh drivers:**
- `set -eu` + `err()` for fatal; every missing input names its producer and remediation (`err "$STAGE_SRC/$f missing — run sh build/x86_64/stage-build.sh first"`)
- EXIT-trap cleanup always runs; exit status preserved through the trap (`status=$?` ... `exit "$status"` in `nix/tier1-r9-x64.sh:203-212`)
- `set +e` around the expect gate invocation, capture RC, re-`set -e`, then `err` on nonzero — the gate names its stage, the driver adds the capture path

**expect gates:** every failure path calls `cleanup` then exits with a stage-named stderr line — never a bare hang-to-timeout without localization

**Anti-silent-failure rules (revival era):**
- mk exit status is NEVER trusted — artifact asserts + markers are the only gates (`build/x86_64/subset-build.sh:45-46`)
- `grep` in pipelines that may match zero lines gets `|| true` under `pipefail` wrappers (`nix/tier1-r9-x64.sh:162-167`)
- No silent accel downgrade: missing `/dev/kvm` with `R9_ACCEL=kvm` is an error; TCG must be chosen explicitly (`nix/tier1-r9-x64.sh:110-120`)

## Logging

**C:** `fprint(2, "prog: ...")` to stderr with program-name prefix; `%r` verb for errstr; kernel uses `print`/`ipprint` to console with static debug-flag guards. No syslog/severity-level concept — do not add one.

**sh/expect drivers:** progress via `echo "== stage N/M: ..."` lines to stdout; stage results to stderr (`puts stderr "tier1-r9-x64: stage ok — $name"`); serial captures are the evidence artifact (retained under `.planning/phases/*/evidence/`). Bake logs PRINT the staging manifest — the log IS the gate's evidence (`build/x86_64/mkimage.sh:307-315`).

## Comments

**When to comment:** non-obvious invariants, magic numbers, protocol quirks, measured hazards (e.g. the mkpaqfs 4194304-byte per-file silent-drop cap at `build/x86_64/mkimage.sh:1041-1049`); provenance references (`D-xx`, `Pitfall N`, phase/task IDs) in revival-era files.

**Format:** C block comments `/* */` only. Everything else uses the language's native line comments.

**JSDoc/TSDoc:** Not applicable — none anywhere. Man pages under `sys/man/` are the documentation.

## Function Design

**C:**
- Few parameters; pass struct pointers (`Dir *`, `Biobufhdr *bp`) over long scalar lists
- Output via caller-allocated buffers with explicit sizes; end-pointer idiom (`strecpy(dst, dstend, src)`)
- `-1` + errstr for ints; `nil` for pointers on failure; always `nil`, never `NULL`
- Variadic printf-family functions must be registered: `#pragma varargck argpos fprint 2` (`sys/include/libc.h`)
- `ARGBEGIN { ... } ARGEND` flag parsing with `EARGF(usage())` for valued flags (`sys/src/cmd/cp.c:19`, `sys/include/libc.h`)
- Concurrency: `threadmain` + `Channel*` via libthread when linking `-lthread`; never pthreads in native code

**Revival-era drivers:**
- One concern per script; pipeline steps numbered in comments and echoed (`step 1/4: ...`)
- Shared state serialized via `flock` on `.build/amd64-tree.lock` — every `/amd64` writer takes it (`build/x86_64/subset-build.sh:70-77`)
- Freshness is enforced, never assumed: per-entry `mk <target>.clean` / in-tree object `rm` before every build; `rm -f` target archives before `mk install` (`ar vu` never prunes) — a stale object can never earn a marker

## Module Design

**C tree:**
- Public API in one header per library under `sys/include/<name>.h`, beginning with `#pragma src "/sys/src/lib<name>"` and `#pragma lib "lib<name>.a"` (`sys/include/bio.h:1-2`)
- Declarations use explicit `extern` with tabbed alignment
- Every directory has a `mkfile`; first line `</$objtype/mkfile` pulls in `sys/src/mkfile.proto` + the arch toolchain (`amd64/mkfile` sets `CC=6c LD=6l O=6 AS=6a`)
- Libraries: `OFILES` with `$O` suffix, `LIB=/$objtype/lib/libX.a`, include `</sys/src/cmd/mksyslib` (`sys/src/libbio/mkfile`)
- Single-program commands: `</sys/src/cmd/mkone`; multi-program dirs: `TARG` list + `</sys/src/cmd/mkmany`
- Standard targets everywhere: `all`, `install`, `clean`, `nuke`, `update`, `installall`, `safeinstall`
- Recursion: `for(i in $LIBS $SUBSYS $CMDS)@{ cd $i; mk $target }` (`sys/src/mkfile`)
- Tier gating (new): under `objtype=amd64`/`aarch64` the `LIBS`/`CMDS`/`SUBSYS` sets are filtered through the tier data files via a single-line awk keep-rule — first path component after the mkfile's prefix, lines starting `[a-z0-9]`, cut at first whitespace (`sys/src/mkfile:69-113`)

**Data-file conventions (revival era):**
- Tier allowlists (`build/x86_64/tiers/tier1-cmds.txt`, `tier2-services.txt`) are plain data, APPEND-ONLY: growth is appending a line after its gate passes; never reorder/edit/delete — "a quiet trim to reach T1-DONE is gate fraud, not a pass"
- Entry grammar: `cmd/<single>`, `cmd/<dir>`, `cmd/<container>/<dir>`, `cmd/<dir>:<target>`, plus driver-only `9k/boot`, `stage/<name>`
- Exclusions are NAMED in comments ("nothing silently dropped"), never silent
- Exactly ONE list per concern — every consumer (mk filters, drivers, gates' computed expectations) reads the same committed file; a second hardcoded list is a defect class

**Barrel files:** Not used in C; header-per-library is the aggregation mechanism.

---

*Convention analysis: 2026-11-13*
