---
phase: 02-aarch64-toolchain-libc-9front-donor
plan: "01"
subsystem: infra
tags: [aarch64, arm64, toolchain, 7a, 7c, 7l, 9front, vendoring, plan9, cross-compiler, lima, namespace-build]

requires:
  - phase: 01-build-tail-green-9k-amd64
    provides: host Plan 9 toolchain (6a/6c/6l, cpp, iar) in .build/hostbin-<os>-aarch64, lima VM 'harvey', BUILD-script + BUILDTOOLCHAIN pattern, NVAR=BITS*32 LP64-fixed shared cc frontend
provides:
  - sys/src/cmd/7c — vendored 9front arm64 C compiler, host-builds on darwin-aarch64 + linux-aarch64, compiles natively under 6c
  - sys/src/cmd/7a — vendored 9front arm64 assembler (assembles 7c -S output)
  - sys/src/cmd/7l — vendored 9front arm64 ken linker (parses .7 objects, correct diagnostics; full link awaits 02-03 libc)
  - sys/src/cmd/cc/compat.h — additive shared-frontend header (systemtype, my* family incl. mysbrk(ulong), alloc/allocn)
  - sys/src/BUILDTOOLCHAIN wiring for 7a/7c/7l (both host platforms)
  - build/aarch64/nsrun.sh + vm-run.sh — lima namespace runners reused by plans 02-03..02-07
affects: [02-02 libmach, 02-03 libc, 02-04 donor kernel, 02-05 harness, 02-06 objtype wiring, 02-07 shell subset]

actuals:
  tokens: 143745
  tasks: 3
  commits: 3
  plan_head_before: 16ec921da22baefca2ea7a2b74385ddc8054e07e
  plan_head_after: 9e6d92c66c2fdaf1f7f0ee96af1a7dcbcfdc0ac4

tech-stack:
  added: [9front 7a/7c/7l @ 350266104f1fed81c78c3d05fa44f523bec0ae4d (vendored source, MIT-compatible)]
  patterns: [vendor-and-own with per-dir provenance README, shim-in-vendored-files-only (D-13), host-tool BUILD via plan9port 9c/9l without -static, hunk-allocator gethunk→mysbrk discipline (Pitfall A), execution-gated ordering (no aarch64/ dirs or CPUS before the gate)]

key-files:
  created:
    - sys/src/cmd/cc/compat.h
    - sys/src/cmd/7c/ (vendored + math.h, BUILD, mkfile, README, committed enam.c)
    - sys/src/cmd/7a/ (vendored + BUILD, mkfile, README)
    - sys/src/cmd/7l/ (vendored + unixcompat.c, dynstub.c, BUILD, mkfile, README)
    - build/aarch64/nsrun.sh
    - build/aarch64/vm-run.sh
  modified:
    - sys/src/BUILDTOOLCHAIN (three BUILD wiring lines after 8l)
    - .gitignore (7a/7c/7l host-build binaries, 7l/cnam.c generated file)

key-decisions:
  - "thestring=aarch64 in all three vendored tools (planner-found shim): host-built tool include/library resolution targets /{thestring}/{include,lib}, so the donor's arm64 value would never find Harvey's renamed tree"
  - "Rotate-fold (OROL/rolor) excised from vendored 7c at 3 sites — Harvey's shared frontend never produces OROL nodes; zero functional loss, frontend untouched (D-13)"
  - "7c/enam.c generated once via donor mkenam ed script and committed (6c precedent); 7l/cnam.c generated per-build by sh mkcname (must track l.h's C_* enum)"
  - "dyn.c excluded from 7l builds (Harvey links statically); new dynstub.c supplies dynreloc no-op (output-equivalent on static links) + fail-loud asmdyn"
  - "7l/unixcompat.c is self-contained (hunk storage local): vendored 7l has no gethunk and l.h no hunk EXTERNs — donor keeps that allocator static in its own cc/compat"

patterns-established:
  - "thestring rename-on-import: every vendored tool's thestring assignment changes arm64→aarch64 (7c/txt.c, 7a/lex.c, 7l/obj.c)"
  - "Host BUILD scripts omit -static (the nix/toolchain.sh 9l shim exists for legacy 6c/8c lines only)"
  - "Namespace runners: nsrun.sh --wrap captures devshell exports and re-enters as root under unshare -m; su tail re-sources exports (Phase 1 su-proof env lesson); conditional /aarch64 bind serves pre- and post-gate trees"

requirements-completed: [TC-02]

coverage:
  - id: D1
    description: "7c vendored + shimmed + host-built on darwin-aarch64; -S smoke emits arm64 TEXT/MOV for recursion/shift/FP; only additive compat.h under sys/src/cmd/cc/"
    requirement: TC-02
    verification:
      - kind: integration
        ref: "cd sys/src/cmd/7c && sh BUILD; 7c -S fib.c → TEXT/MOV grep"
        status: pass
      - kind: other
        ref: "git -C .build/9front-donor rev-parse HEAD == 350266104f1fed81c78c3d05fa44f523bec0ae4d (hard pin gate)"
        status: pass
      - kind: other
        ref: "grep OROL/rolor residue = 0; git status sys/src/cmd/cc/ shows only compat.h"
        status: pass
    human_judgment: false
  - id: D2
    description: "7a + 7l vendored + shimmed + host-built; 7c→7a→7l pipeline runs and 7l emits correct ken-linker diagnostics (entry not text: _main / _main: not defined) with no Pitfall-A segfault"
    requirement: TC-02
    verification:
      - kind: integration
        ref: "7c fib.c && 7a fib.s && 7l fib.7 → _main/entry diagnostics grep (exit not 139)"
        status: pass
    human_judgment: false
  - id: D3
    description: "BUILDTOOLCHAIN builds 7a/7c/7l on darwin-aarch64 (nix run .#toolchain) and linux-aarch64 (guest toolchain ensure); native 6c compile of all three tools green in the amd64 namespace (vm-run sweep)"
    requirement: TC-02
    verification:
      - kind: integration
        ref: "sh build/aarch64/vm-run.sh 'for i in 7a 7c 7l; mk nuke && mk && mk clean' → SWEEP-EXIT-OK"
        status: pass
      - kind: integration
        ref: "nix run .#toolchain → 'harvey toolchain built'; hostbin 7a/7c/7l rebuilt"
        status: pass
    human_judgment: false

duration: 13 min
completed: 2026-10-31
status: complete
---

# Phase 02 Plan 01: Vendor 9front 7a/7c/7l Toolchain Summary

**Vendored 9front's arm64 toolchain (7a/7c/7l @ 350266104f1fed81c78c3d05fa44f523bec0ae4d) into sys/src/cmd/ with ten Harvey shims, host-built all three on darwin-aarch64 and linux-aarch64 via BUILDTOOLCHAIN, and proved native 6c-compilability in the amd64 namespace — with zero edits to Harvey's shared cc frontend (D-13).**

## Performance

- **Duration:** 13 min
- **Started:** 2026-10-31T16:39:30Z
- **Completed:** 2026-10-31T16:52:37Z
- **Tasks:** 3
- **Files modified:** 46 (44 created incl. vendored trees, 2 modified: BUILDTOOLCHAIN + .gitignore)

## Accomplishments

- Pin-verified 9front donor clone at `.build/9front-donor` (git init + fetch --depth 1 by SHA + checkout FETCH_HEAD + rev-parse hard gate) — the phase's single supply-chain input (T-02-01 mitigation)
- **7c**: vendored wholesale incl. 7.out.h; shims: math.h unix-SDK shadow (6c verbatim), gc.h const-qualified swcmp, rotate-fold excision (sgen.c rolor call, cgen.c OROL label, txt.c OROL block — no residue), thestring=aarch64; enam.c generated via donor mkenam ed script and committed; host-builds clean and emits correct arm64 assembly (recursion/shift/FP smoke)
- **7a**: vendored; shims: BUFSIZ=(16*1024), 2-arg macexpand decl, hunk-allocator EXTERN block (storage via lex.c `#define EXTERN` empty, 6a pattern), thestring=aarch64; assembles 7c's -S output
- **7l**: vendored; shims: l.h MAXIO=(16*1024), thestring=aarch64 (library search path), new self-contained unixcompat.c (hunk-backed malloc family; gethunk→mysbrk, never malloc — Pitfall A), new dynstub.c replacing dyn.c in builds; cnam.c generated per-build via sh mkcname; pipeline proof: 7c→7a→7l emits exact ken-linker diagnostics `_main: not defined` / `entry not text: _main` (libc link awaits plan 02-03)
- **Additive `sys/src/cmd/cc/compat.h`** — the only shared-frontend-side change; git status confirms zero modifications under sys/src/cmd/cc/
- **BUILDTOOLCHAIN wiring** proven on both host platforms: `nix run .#toolchain` on darwin-aarch64 and the lima guest's toolchain ensure on linux-aarch64 both built 7a/7c/7l
- **build/aarch64/nsrun.sh + vm-run.sh** — command-parameterized namespace runners (BUILD_SETUP_NAMESPACE bind set, merged root-owned /lib staging with trap unwind, conditional /aarch64 bind, su-proof env re-source); native-compile sweep green: all three tools `mk nuke && mk && mk clean` under objtype=amd64, so cmd/mkfile's DIRS auto-discovery cannot silently break the Phase 1 green gate

## Task Commits

1. **Task 1: Vendor 7c + additive cc/compat.h + shims; host-build and codegen-smoke 7c** — `3d9c7d51b` (feat)
2. **Task 2: Vendor 7a + 7l with shims; host-build both; pipeline diagnostics proof** — `263af822f` (feat)
3. **Task 3: BUILDTOOLCHAIN wiring + lima namespace runners + native-compile sweep check** — `9e6d92c66` (feat)

**Plan metadata:** see final docs commit below.

## Files Created/Modified

- `sys/src/cmd/cc/compat.h` — additive host-compat declarations (EXTERN idiom, Plan9/Unix/Windows enum, systemtype/pathchar, my* family with mysbrk(ulong), alloc/allocn); matches Harvey's cc/compat definitions
- `sys/src/cmd/7c/` — vendored donor tree (7.out.h, cgen.c, gc.h, list.c, machcap.c, mul.c, peep.c, reg.c, sgen.c, swt.c, txt.c) + math.h + enam.c (generated, committed) + BUILD + mkfile + README
- `sys/src/cmd/7a/` — vendored donor tree (a.h, a.y, lex.c) + BUILD + mkfile + README
- `sys/src/cmd/7l/` — vendored donor tree (asm.c, asmout.c, bits.c, compat.c, dyn.c, l.h, list.c, mkcname, mod.c, noop.c, obj.c, optab.c, pass.c, span.c) + unixcompat.c + dynstub.c + BUILD + mkfile + README
- `sys/src/BUILDTOOLCHAIN` — three `(cd $HARVEY/sys/src/cmd/7{a,c,l} && sh BUILD)` lines after the 8l entry
- `build/aarch64/nsrun.sh` — guest-side namespace runner (bind set mirror, command-parameterized su tail, /lib unwind trap, conditional /aarch64 bind)
- `build/aarch64/vm-run.sh` — host-side lima driver (rsync with vm-build.sh exclusions, guest toolchain ensure via 7c/ar sentinels, exit-status propagation)
- `.gitignore` — 7a/7c/7l host-build binaries + 7l/cnam.c (generated per-build)

## Decisions Made

- thestring=aarch64 across all three vendored tools (planner-found shim beyond research's nine): Harvey's host tools resolve `/{thestring}/include` and `/{thestring}/lib`; the donor's arm64 value would search paths that never exist in the renamed tree (D-11)
- enam.c committed / cnam.c per-build (research Open Question 4 resolution; 6c precedent)
- dynstub.c shape: dynreloc is a no-op (the donor's version only records entries consumed by asmdyn under dlm mode, so a no-op is output-equivalent for static links); asmdyn fails loud if dlm mode is ever triggered
- BUILDTOOLCHAIN's supported driver on darwin is `nix run .#toolchain` (provides the 9l -static shim); bare `sh BUILDTOOLCHAIN` in a devshell fails at 6c's legacy `-static` link — pre-existing, logged to deferred-items.md #1

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Build artifact swept into initial Task 1 staging**
- **Found during:** Task 1 commit
- **Issue:** `git add sys/src/cmd/7c` swept the locally-built `7c` Mach-O binary (472KB) into the commit — the dir's BUILD leaves the binary in-tree and no gitignore rule covered it (9cc's iar has one; the tools did not)
- **Fix:** `git rm --cached` the binary, added gitignore entries `/sys/src/cmd/7a/7a`, `/sys/src/cmd/7c/7c`, `/sys/src/cmd/7l/7l` (9cc/iar precedent), amended before any push; also added `/sys/src/cmd/7l/cnam.c` (generated per-build)
- **Files modified:** .gitignore, sys/src/cmd/7c/7c (untracked)
- **Verification:** `git ls-files` shows no 7c binary; `git check-ignore` passes for all three tool binaries and cnam.c
- **Committed in:** 3d9c7d51b (amended Task 1 commit)

**2. [Rule 1 - Bug] 7c/README wording tripped the plan's own thestring verify**
- **Found during:** Task 1 verification
- **Issue:** the README documented the shim as `thestring = "aarch64" (donor ships "arm64")` — one line containing both `thestring` and a quoted `"arm64"`, matching the verify's `grep -rn 'thestring.*"arm64"' sys/src/cmd/7c` (no --include filter), falsifying the residue check
- **Fix:** reworded the README to name the donor value without quoting it on a thestring line; applied the same regex-safe wording discipline to the 7a/7l READMEs
- **Files modified:** sys/src/cmd/7c/README
- **Verification:** plan's verify chain prints VERIFY1 PASS
- **Committed in:** 3d9c7d51b (Task 1 commit)

**3. [Rule 2 - Missing Critical] dyn.c exclusion left asmdyn/dynreloc undefined**
- **Found during:** Task 2 (7l host link)
- **Issue:** the plan (and its acceptance criteria) exclude vendored dyn.c from both builds — Harvey links statically — but asm.c/asmout.c still reference `asmdyn`/`dynreloc` (`ld: Undefined symbols: _asmdyn, _dynreloc`); dynreloc is reached even on static links (C_ADDR operands, brdist)
- **Fix:** new `sys/src/cmd/7l/dynstub.c`: dynreloc no-op (output-equivalent — the donor only writes the recorded relocations under dlm mode via asmdyn) + fail-loud asmdyn (`diag` + `errorexit`); wired into BUILD and mkfile in place of dyn.c; dyn.c itself kept in the tree and documented in 7l/README. The acceptance criterion "dyn.c is absent from BUILD and mkfile object lists" remains satisfied
- **Files modified:** sys/src/cmd/7l/dynstub.c (new), sys/src/cmd/7l/BUILD, sys/src/cmd/7l/mkfile, sys/src/cmd/7l/README
- **Verification:** 7l links and the 7c→7a→7l pipeline emits correct ken-linker diagnostics (no segfault)
- **Committed in:** 263af822f (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 missing critical)
**Impact on plan:** All three preserve the plan's intent and acceptance criteria; no scope creep, no shared-frontend edits (D-13 held).

## Issues Encountered

- Bare `sh sys/src/BUILDTOOLCHAIN` in a plain devshell fails at 6c's `-static` link (Apple ld has no crt0.o) — the 9l shim that drops `-static` exists only inside `nix run .#toolchain`. Pre-existing Phase 1 design, not caused by this plan's changes; the wiring was verified through the supported driver. Logged as deferred-items.md #1.
- The vendored 7l contains **no gethunk at all** (the donor keeps its allocator static inside its own cc/compat) — the plan's CRITICAL inspection ("find it by grep; confirm it calls mysbrk") was performed and resolved by making 7l/unixcompat.c self-contained (hunk storage + gethunk→mysbrk in-file). No vendored-file patch was needed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02-02 (libmach arm64): vendored `7c/7.out.h` is in place for `7obj.c`'s include; Alpha excision + setmach/executable.c/obj.c wiring can proceed
- Plan 02-03 (libc/aarch64): 7a/7c/7l host binaries in `.build/hostbin-darwin-aarch64/bin` and the guest's `hostbin-linux-aarch64/bin`; 7l's library search targets `/aarch64/lib` once libc lands
- Plans 02-04..02-07: `build/aarch64/vm-run.sh` + `nsrun.sh` are the reuse point for every guest-side namespace run (donor kernel build, libc compile, shell subset)
- No blockers.

## Self-Check: PASSED
