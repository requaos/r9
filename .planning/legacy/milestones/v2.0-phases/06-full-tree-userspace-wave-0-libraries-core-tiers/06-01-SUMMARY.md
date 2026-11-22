---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 01
subsystem: infra
tags: [plan9, toolchain, gethunk, cc, 6l, linker, compiler, aarch64, regression-gate, tdd]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: vendored 7a/7c/7l toolchain, the proven 7l/unixcompat.c gethunk pattern, deferred item 02-6 (gethunk ×2), hostbin layout
  - phase: 01-9k-amd64-build-green
    provides: the amd64-safety gate shape (nix run .#build + artifact asserts + zero-failure-marker scan)
provides:
  - cc/macbody gethunk accumulate-when-contiguous fix + teeth-proven repro gate (build/aarch64/gethunk-repro-cc.sh)
  - 6l/obj.c gethunk accumulate-when-contiguous fix + teeth-proven repro gate (build/aarch64/gethunk-repro-6l.sh)
  - aarch64/include/u.h FPdbleword uint lo/hi (D-01) with libc/fmt rebuilt green
  - amd64 parity evidence after both shared-frontend edits
  - TEETH-OK/TEETH-FAIL/GREEN/RED marker vocabulary for the 06-08 wave0-host CI job
affects: [06-03 (tier-1 float canary consumes the FPdbleword fix), 06-04 (coverage sweep depends on the gethunk fixes not wedging), 06-08 (wave0-host CI job consumes the repro gates), 10 (tiers 2-4 inherit the fixed toolchain)]

# Actuals (#2632) — chars/4 over the realized diff of the plan's five commits
actuals:
  tokens: 8539
  tasks: 3
  commits: 5   # the plan's own commits; the ledger range below also contains 2 concurrent phase-07 docs commits

# Plan-commit ledger (#3968) — measured, includes concurrent-session commits in range
commits: 7
plan_head_before: df4b1cb4ab5d7bc9f97bb7b4882933428336acda
plan_head_after: 83fde179f1a0b61c02b60a34e1e4c1e6de0e2f62

tech-stack:
  added: []
  patterns:
    - "gethunk accumulate-when-contiguous tail (if(h == hunk + nhunk) nhunk += nh; else reset) — now uniform across cc/macbody, 6l/obj.c, 7l/unixcompat.c"
    - "timeout-gated repro harness shape: snapshot|archive|verify, git-archive pristine teeth + working-tree fixed build, TAP report for the TDD RED gate"
    - "darwin→lima platform split for gates that need >4MB of sbrk (macOS arena measured at exactly 4096KB)"

key-files:
  created:
    - build/aarch64/gethunk-repro-cc.sh
    - build/aarch64/gethunk-repro-6l.sh
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-01-RED-task1.json
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-01-RED-task2.json
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/deferred-items.md
  modified:
    - sys/src/cmd/cc/macbody
    - sys/src/cmd/6l/obj.c
    - aarch64/include/u.h

key-decisions:
  - "06-01: gethunk ×2 fixed with the proven accumulate-when-contiguous tail; each fix teeth-proven by a committed timeout-gated repro (cc: >600KB macro define; 6l: >2.5MB archive __.SYMDEF bang-malloc) — deferred item 02-6 closed"
  - "06-01: the unbounded gethunk loop's terminus is host-dependent (timeout 124 on deep-sbrk linux, out-of-memory on bounded arenas) — both signatures count as TEETH-OK; only a clean compile/link inside the timeout is TEETH-FAIL"
  - "06-01: A4 verdict — the 6l output data image streams through datblk (array shape toothless) and cc caps string constants (string-literal shape impossible); the working 6l trigger is a >2.5MB archive symbol table (obj.c:520 bang-malloc)"
  - "06-01: macOS sbrk caps at exactly 4096KB/process — >2.5MB-malloc gates cannot run natively on darwin; the 6l repro runs linux legs (native, or lima guest on darwin) without the mount namespace (committed amd64/lib via -L suffices)"
  - "06-01: FPdbleword uint lo/hi landed per D-01; libc/fmt rebuilt green (frexp.c/nan.c/fltfmt.c recompiled into libc.a); float-print execution canary queued to 06-03 tier-1 sweep; riscv64 latent defect noted, deliberately untouched"

requirements-completed: [USPACE-01]

coverage:
  - id: D1
    description: "cc/macbody gethunk accumulate-when-contiguous fix with teeth-proven timeout gate (6c+7c compile a >600KB macro define inside 60s; archive-built pre-fix pair cannot)"
    requirement: USPACE-01
    verification:
      - kind: integration
        ref: "sh build/aarch64/gethunk-repro-cc.sh verify — TEETH-OK 6c/7c (archive df4b1cb4a, ENOMEM terminus) + GREEN 6c/7c, exit 0"
        status: pass
      - kind: other
        ref: "sed -n '/^gethunk(void)/,/^}/p' sys/src/cmd/cc/macbody | grep -c 'nhunk += nh' → 1"
        status: pass
    human_judgment: false
  - id: D2
    description: "6l/obj.c gethunk accumulate-when-contiguous fix with teeth-proven timeout gate (fixed 6l links a 250K-function ~4.5MB-symtab archive inside 120s; archive-built pre-fix 6l aborts ENOMEM)"
    requirement: USPACE-01
    verification:
      - kind: integration
        ref: "sh build/aarch64/gethunk-repro-6l.sh verify (lima linux leg) — TEETH-OK 6l + GREEN 6l, exit 0"
        status: pass
      - kind: other
        ref: "sed -n '/^gethunk(void)/,/^}/p' sys/src/cmd/6l/obj.c | grep -c 'nhunk += nh' → 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "amd64 leg unchanged after both shared-frontend fixes: host toolchain rebuild + amd64 9k10cpu kernel build + zero-failure-marker log scan"
    requirement: USPACE-01
    verification:
      - kind: integration
        ref: "nix run .#toolchain && nix run .#build → amd64/9k10cpu + amd64/9k10cpu.elf present; 0 hits on exit status=exit|Permission denied|don't know how|duplicate|skipping missing include|Can't read header"
        status: pass
    human_judgment: false
  - id: D4
    description: "aarch64/include/u.h FPdbleword uint lo/hi (D-01) with libc (port + fmt) rebuilt green for aarch64"
    requirement: USPACE-01
    verification:
      - kind: other
        ref: "sed -n '/^union FPdbleword/,/^};/p' aarch64/include/u.h | grep -cE 'uint (lo|hi);' → 2, zero ulong members"
        status: pass
      - kind: integration
        ref: "sh build/aarch64/subset-build.sh → SUBSET-BUILD-OK (fltfmt.c/frexp.c/nan.c recompiled into /aarch64/lib/libc.a against the fixed header)"
        status: pass
    human_judgment: false

duration: 35 min
completed: 2026-11-04
status: complete
---

# Phase 6 Plan 01: Wave-0 gethunk ×2 + FPdbleword Summary

**Both silent compile/link-hang landmines (cc/macbody, 6l/obj.c gethunk reset-not-accumulate) closed with teeth-proven timeout-gated repro harnesses, and the aarch64 FPdbleword union fixed to uint lo/hi with libc/fmt rebuilt green — amd64 leg re-proven green after the shared-frontend edits.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-11-04T22:41:59Z
- **Completed:** 2026-11-04T23:16:44Z
- **Tasks:** 3
- **Files modified:** 3 source files (macbody, 6l/obj.c, aarch64/include/u.h) + 2 new harness scripts + 3 planning artifacts

## Accomplishments

- `cc/macbody` gethunk now accumulates contiguous hunks (10L*NHUNK ladder, yyerror reporter, and `(char*)mysbrk` cast untouched); 6c+7c compile a >600KB single-macro input inside 60s while the archive-built pre-fix pair from `df4b1cb4a` cannot (teeth proven in CI-durable form via `git archive`)
- `6l/obj.c` gethunk same tail fix (5L/25L ladder, diag reporter, bare mysbrk untouched); fixed 6l links a 250K-function archive whose `__.SYMDEF` is ~4.5MB inside 120s while the pre-fix 6l aborts with the unbounded-loop ENOMEM terminus
- A4 input-shape spike settled empirically and recorded in the 6l script header: array shape toothless (datblk streams), string-literal shape impossible (cc string caps), archive-symtab bang-malloc is the working trigger
- `aarch64/include/u.h` FPdbleword `ulong lo/hi` → `uint lo/hi` matching amd64 exactly; `frexp.c`/`nan.c`/`fltfmt.c` recompiled into `/aarch64/lib/libc.a`; subset relink green (SUBSET-BUILD-OK)
- amd64 parity: `nix run .#toolchain` + `nix run .#build` green after both gethunk edits; `amd64/9k10cpu{,.elf}` present; zero failure-marker hits in the build log
- Measured platform fact now on record: macOS sbrk arena is exactly 4096KB/process — the 6l gate's venue is linux (native or lima guest), and the phase's deferred-items.md carries the Io.b[16KB] macro-expansion cap discovery

## Task Commits

TDD tasks 1 and 2 each produced RED (test) + GREEN (fix) commits:

1. **Task 1 RED:** `75d534b12` — `test(06-01): gethunk timeout-gate repro for cc/macbody (RED)` (build/aarch64/gethunk-repro-cc.sh)
2. **Task 1 GREEN:** `2e3b37027` — `fix(06-01): gethunk accumulate-when-contiguous in cc/macbody (USPACE-01)` (sys/src/cmd/cc/macbody)
3. **Task 2 RED:** `99946a78a` — `test(06-01): gethunk timeout-gate repro for 6l/obj.c (RED)` (build/aarch64/gethunk-repro-6l.sh)
4. **Task 2 GREEN:** `f9bf852da` — `fix(06-01): gethunk accumulate-when-contiguous in 6l/obj.c (USPACE-01)` (sys/src/cmd/6l/obj.c)
5. **Task 3:** `83fde179f` — `fix(06-01): FPdbleword uint lo/hi on aarch64 (D-01, USPACE-06 gate prep)` (aarch64/include/u.h)

**Amend-collision repair (concurrent-session history):** `bb6ae2c53` — `docs(07): create phase plan` re-creates, content-identical (verified: tree diff empty), the phase-07 planning commit `c25f04220` that a concurrent session committed while this executor was working and that an executor `--amend` briefly absorbed. No content lost; attribution restored.

**Plan metadata:** recorded below (final docs commit).

## TDD Gate Compliance

| Task | RED commit | GREEN commit | RED evidence | REFACTOR |
|------|-----------|--------------|--------------|----------|
| 1 (cc/macbody) | `75d534b12` ✓ | `2e3b37027` ✓ | RED_EVIDENCE_OK (06-01-RED-task1.json) | not needed (tail-only edit) |
| 2 (6l/obj.c) | `99946a78a` ✓ | `f9bf852da` ✓ | RED_EVIDENCE_OK (06-01-RED-task2.json) | not needed (tail-only edit) |

- **GREEN commit type:** the plan's action steps mandate `fix(06-01): ...` messages; tdd.md's gate table names `feat(...)`. These are bug fixes, so the plan's explicit `fix(...)` type was used — RED commits use the canonical `test(06-01)` type. `workflow.tdd_mode` is false in config (gate advisory).
- **Atomic-commit reconciliation:** the plan text said "ONE atomic commit (fix + repro script + gate)"; the dispatch-level TDD contract (commit-scope: test→feat) splits each into RED test commit + GREEN fix commit. Per RED/GREEN pair, the union of files is exactly {script, fixed source} — the acceptance criterion's "git show --stat HEAD names exactly macbody + gethunk-repro-cc.sh" holds across the pair (each individual commit names exactly one).
- Each RED run failed on the target assertion ("fixed <tool> compiles/links within Ns") with the pristine-direction teeth assertions already passing — intentional RED, machine-validated via `gsd_run check tdd-red-evidence`.

## Files Created/Modified

- `sys/src/cmd/cc/macbody` — gethunk tail: reset → accumulate-when-contiguous (+16-line why-comment)
- `sys/src/cmd/6l/obj.c` — same tail fix (+18-line why-comment naming the bootfs/symtab incident class)
- `aarch64/include/u.h` — FPdbleword `ulong lo/hi` → `uint lo/hi` (two tokens)
- `build/aarch64/gethunk-repro-cc.sh` — new; snapshot|archive|verify; TEETH-OK/TEETH-FAIL/GREEN/RED markers + TAP; >600KB macro-define input
- `build/aarch64/gethunk-repro-6l.sh` — new; same modes/vocabulary; 250K-function archive-symtab input; darwin→lima venue split
- `.planning/phases/06-*/06-01-RED-task{1,2}.json` — persisted TDD RED evidence records
- `.planning/phases/06-*/deferred-items.md` — Io.b[16KB] macexpand cap + darwin 4096KB sbrk arena discoveries

## Decisions Made

- Both gethunk fixes use the verbatim 7l/unixcompat.c accumulate-when-contiguous tail, adapted per site (10L ladder + yyerror + cast for cc; 5L/25L + diag + bare mysbrk for 6l) — carried-forward amd64-safety rule honored.
- ENOMEM ("out of memory", rc=1, no output) is accepted as a TEETH-OK terminus alongside timeout-124: the unbounded gethunk loop ends wherever the host's sbrk arena ends, and darwin's 4096KB arena reaches it fast. Only a clean compile/link inside the timeout is TEETH-FAIL.
- 6l repro input shape per the A4 spike: archive symbol table >2.5MB (the only proven >2.5MB single-malloc site in 6l — obj.c:520). The plan's array and string-literal shapes are both empirically impossible; the spike clause in the plan sanctioned the switch, and the verdict is recorded in the script header.
- The 6l repro's fixed pair is built fresh from the working tree with the in-tree BUILD scripts (not hostbin) — zero staleness in the gate; real-hostbin freshness remains gated by the amd64-safety leg (`nix run .#toolchain` + `.#build`).
- FPdbleword commit message names the riscv64 latent defect, deliberately untouched (skeletal port, D-01 scope).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] cc repro input shape: string literal → digit/comma macro define**
- **Found during:** Task 1 RED spike
- **Issue:** The plan's sketched input (a single >600KB string literal as the macro body) errors before reaching gethunk: macdef routes alphabetic runs in strings through nextsym (NSYMB=1500 → "symbol too large"), and the lexer caps string constants at ~STRINGSZ ("string constant too long").
- **Fix:** Input shape changed to `#define BIG (1,1,...,1)` (>600KB of digit/comma replacement text) + `#ifdef BIG` presence block; every byte flows through macdef's allocn growth (the gethunk caller under test).
- **Files modified:** build/aarch64/gethunk-repro-cc.sh (header documents the spike)
- **Verification:** pristine archive pair hits the unbounded-loop terminus; fixed hostbin exits 0 with object files written
- **Committed in:** 75d534b12 (test), 2e3b37027 (fix)

**2. [Rule 3 - Blocking] cc repro use-line dropped: macexpand cannot carry >16KB expansions**
- **Found during:** Task 1 GREEN spike
- **Issue:** The plan's `char *p = BIG;` use fails even on the FIXED compiler — macexpand writes into Io.b[BUFSIZ=16KB] unbounded and the io length field is a `short` ("saw $end" syntax error at the use line). Pre-existing upstream limitation, orthogonal to the define-time hang under test.
- **Fix:** The use line was replaced by `#ifdef BIG` (a real macro-table consumer that never expands); the define path alone is the gate. Discovery logged to the phase deferred-items.md.
- **Files modified:** build/aarch64/gethunk-repro-cc.sh
- **Verification:** GREEN 6c/7c with object assertion; TEETH intact on the archive pair
- **Committed in:** 75d534b12 (test), 2e3b37027 (fix)

**3. [Rule 3 - Blocking] Archive build needs 6l+ld dirs beyond the plan's listed set**
- **Found during:** Task 1 archive teeth-proof
- **Issue:** The plan's archive set (cc, 6c, 7c) doesn't host-build: both compiler BUILDs compile `../6l/unixcompat.c`, and 6l/l.h pulls `../ld/elf.h`.
- **Fix:** Archive set extended to {cc, 6c, 6l, 7c} + ld; also had to strip hostbin from PATH with a pure-sh loop (hostbin's ken-format nm silently breaks 9l autolib; hostbin's Plan 9-dialect sed rejected the first filter).
- **Files modified:** build/aarch64/gethunk-repro-cc.sh
- **Verification:** archive build produces working pristine 6c/7c; TEETH-OK ×2 from the archive pair
- **Committed in:** 75d534b12 (test)

**4. [Rule 3 - Blocking] 6l repro input shape: archive symbol table (A4 spike settled)**
- **Found during:** Task 2 spike (lima linux guest, pristine 6l)
- **Issue:** Plan's primary ≥4MB array shape is toothless (6l streams its data image through datblk's rolling buffer — pristine link completed); plan's >2.5MB string-literal fallback cannot be produced by cc (string caps). Additionally the dense-array compile itself OOMs on darwin's 4MB sbrk arena even post-fix.
- **Fix:** Trigger switched to a 250K-function archive (~4.5MB `__.SYMDEF`), exercising objfile's whole-symtab bang-malloc (6l/obj.c:520) — the only proven >2.5MB single-malloc site in 6l. Verdict recorded in the script header per the plan's A4 clause.
- **Files modified:** build/aarch64/gethunk-repro-6l.sh
- **Verification:** pristine 6l aborts ENOMEM with no output; fixed 6l links within 120s and writes the binary
- **Committed in:** 99946a78a (test), f9bf852da (fix)

**5. [Rule 3 - Blocking] 6l repro venue: darwin sbrk arena (4096KB, measured) cannot host a >2.5MB-malloc gate**
- **Found during:** Task 2 spike
- **Issue:** Pre-fix, the loop ENOMEMs at 4MB (teeth fine); but the FIXED linker needs ~5MB to accumulate past the 2.5MB ceiling — GREEN is impossible natively on darwin.
- **Fix:** The 6l repro gate runs on linux: natively when uname=Linux, in the lima guest on darwin (platform split copied from vm-run.sh, minus the mount namespace — the link needs only committed amd64/lib via -L). On linux CI legs the gate runs unchanged.
- **Files modified:** build/aarch64/gethunk-repro-6l.sh
- **Verification:** full verify green on the lima linux leg; same script path runs native-linux (CI wave0-host job)
- **Committed in:** 99946a78a (test), f9bf852da (fix)

**6. [Rule 3 - Blocking] 6l repro fixed candidate built from the working tree, not hostbin**
- **Found during:** Task 2 design
- **Issue:** The plan's "current hostbin pair" is unverifiable in the lima venue (guest hostbin is guest-local and sentinel-skipped once built — it would stay pre-fix forever).
- **Fix:** verify builds the fixed 6c+6l fresh from the (pushed) working tree with the in-tree BUILD scripts — a stronger, staleness-free reading of "host-built from the working tree"; real hostbin freshness stays gated by the amd64-safety leg.
- **Files modified:** build/aarch64/gethunk-repro-6l.sh
- **Committed in:** 99946a78a (test)

---

**Total deviations:** 6 auto-fixed (all Rule 3 - blocking input-shape/venue corrections)
**Impact on plan:** Every deviation is gate-machinery plumbing to make the planned teeth provable on the real toolchain and the real hosts; the three C edits are byte-exactly what the plan specified (two tail-only gethunk blocks + the two-token FPdbleword fix). No scope creep; no amd64-leg behavior change (parity gate green).

## Issues Encountered

- **Concurrent-session amend collision:** a phase-07 planning session committed `docs(07): create phase plan` (c25f04220) to master while this executor was working; an executor-side `git commit --amend` absorbed it. Repaired by soft-reset split and content-identical re-creation (`bb6ae2c53`; tree diff vs the collision commit is empty). No content lost. Lesson recorded: no more `--amend` mid-plan on this shared-checkout workflow; later RED-test refinements were committed as separate amends only after reflog checks, and the final scripts stand.
- **vm-build rsync-back clobbered the in-flight u.h edit:** `nix run .#build` pulls `amd64/ 386/ aarch64/` back from the guest wholesale at completion; the guest tree predated the FPdbleword edit, so the header reverted on disk and the FIRST subset-build run validated the *unfixed* header (SUBSET-BUILD-OK, invalid as Task-3 evidence). Caught by re-checking the file before commit; re-applied the edit and re-ran subset-build — the second log shows frexp.c/nan.c/fltfmt.c recompiling into libc.a against the fixed header. (Pre-existing CONCERNS.md-class build-mutates-tree hazard; not caused by this plan.)
- **Guest-side compile provenance note:** the 6l repro's fixed/pristine pairs build with the in-tree BUILD scripts under plan9port 9c/9l in the lima guest (linux-aarch64) — no mount namespace required; `-L <checkout>/amd64/lib` suffices for libc.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 06-02..06-08 unblocked: the coverage sweep (06-04) can no longer wedge on the gethunk pair; the tier-1 float canary (06-03, `T1-OK float-d01`) now gates on a correct FPdbleword; the wave0-host CI job (06-08) consumes both repro scripts' TEETH-OK/GREEN marker vocabulary.
- ROADMAP SC#1's first half (gethunk regression proof) is satisfiable from the committed archive-mode teeth (`git archive $PREREF` works from any later checkout); the 9k-amd64 pool-panic re-test half lands in 06-08 (D-04).
- Deferred items filed: macexpand Io.b[16KB] expansion cap; darwin 4096KB sbrk arena venue constraint.

## Self-Check: PASSED

- [x] `build/aarch64/gethunk-repro-cc.sh` exists on disk (259 lines, committed 75d534b12)
- [x] `build/aarch64/gethunk-repro-6l.sh` exists on disk (356 lines, committed 99946a78a)
- [x] `sys/src/cmd/cc/macbody` gethunk contains `if(h == hunk + nhunk)` + `nhunk += nh`, retains `10L*NHUNK` + `yyerror("out of memory")`
- [x] `sys/src/cmd/6l/obj.c` gethunk contains `if(h == hunk + nhunk)` + `nhunk += nh`, retains `25L*NHUNK` + `diag("out of memory")`
- [x] `aarch64/include/u.h` union region: 2 `uint` members, 0 `ulong` members
- [x] Commits found: 75d534b12, 2e3b37027, 99946a78a, f9bf852da, 83fde179f, bb6ae2c53 (all on master)
- [x] `sh build/aarch64/gethunk-repro-cc.sh verify` exit 0 (TEETH-OK ×2 + GREEN ×2) — post-rebuild re-run
- [x] `sh build/aarch64/gethunk-repro-6l.sh verify` exit 0 (TEETH-OK + GREEN, lima linux leg)
- [x] `nix run .#build` exit 0; amd64/9k10cpu + 9k10cpu.elf present; 0 failure-marker hits
- [x] `sh build/aarch64/subset-build.sh` exit 0 with SUBSET-BUILD-OK (against the fixed header)
- [x] No pre-existing dirty-tree files staged (rc/bin/Kill, sys/lib/postscript/**, sys/lib/troff/**, sys/src/cmd/postscript/**, .planning/phases/07-*/**, graphs, PATTERNS files all untouched by every commit)

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-04*
