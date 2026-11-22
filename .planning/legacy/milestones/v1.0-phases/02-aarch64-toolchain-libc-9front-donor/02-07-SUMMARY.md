---
phase: 02-aarch64-toolchain-libc-9front-donor
plan: "07"
subsystem: infra
tags: [aarch64, tc-05, d-08, d-09, subset, shell-boot, execution-gate, nmount-audit, libmp, libthread, libbio, 9p-writable, dual-arch, regression]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "06"
    provides: committed objtype=aarch64 wiring (aarch64/mkfile, include headers, CPUS, OS+=7, pcc row) that the subset build flows through with zero scratch injection
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "05"
    provides: run-gate.sh bake+boot+execute+assert mechanics and the human-approved TC-03 execution evidence
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "03"
    provides: host nm (proven on 7l executables) for the D-09 audit; libc/aarch64
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "04"
    provides: donor kernel + boot-9qemu.sh harness (TCG boot to rc shell), srv9.py
provides:
  - sys/src/libmp/aarch64/mkfile + sys/src/libsec/aarch64/mkfile — empty-OFILES arch stubs (riscv64 pattern, byte-identical modulo the objtype line) so port+$objtype recursion resolves for aarch64
  - sys/src/libthread/aarch64.c — donor arm64.c byte-verbatim (pin 35026610); libthread's $objtype.$O member (JMPBUF macros identical in the committed aarch64/include/u.h)
  - build/aarch64/subset-build.sh — the aarch64 shell-subset namespace build driver (libs in dependency order = the full #pragma-lib closure, then the 16 commands; rsync-back to the gitignored host aarch64/bin; idempotent, names the failing dir on abort)
  - build/aarch64/subset-exec.sh — the D-08 execution gate (bake + boot + 20-step exercise + assert; marker table in header; SUBSET-OK sent only after every assertion passes); evidence .build/9front-dist/subset-exec.log (three consecutive green runs)
  - build/aarch64/srv9.py --writable-prefix — opt-in scoped writable 9P2000 (qid.vers bumps, dir-cache invalidation, honest modes); default byte-identical read-only
  - build/scripts/linux-build — arch=(amd64 aarch64) + subset-scoped build_aarch64 (the default build is now dual-arch)
  - nix/vm-build.sh — aarch64 artifact rsync-back; nix/build.sh — nm-shadow shim for host-tool builds
  - sys/src/libmp/port/mpvecdigmulsub.c — the split that retires the 2021 PORTmpvecdigmulsub hack (member names now align 1:1 with text symbols)
affects: [phase-03 (r9 — GICv3/HVF forward notes unchanged), phase-04 (r9 svc dispatch consumes ABI.md + ureg.h; hello.7.out remains the ABI bisect point; the subset binaries are the frozen-ABI userland), phase-05 (dist/ISO inherits a dual-arch default build), hygiene phase (deferred items 10-12)]

actuals:
  tokens: 15588   # chars/4 over the realized diff (62353 chars, 16 files)
  tasks: 3
  commits: 3      # measured: git rev-list --count c40ef68e7..HEAD (ledger gsd-plan-head-before-02-07)
  plan_head_before: c40ef68e7bd9d4e8d6d2ba4efa714486eb8e61d7
  plan_head_after: 280460eee

tech-stack:
  added: []
  patterns:
    - "pragma-closure lib scoping: the subset's lib set is discovered by link-time #pragma lib resolution (headers carry the pragmas), not by mkfile LIB= declarations — subset-build.sh's ordered lib list IS the closure"
    - "per-target install scoping (mkmany %.install) lets a dir ship exactly the named subset member (factotum without fgui's out-of-scope graphics deps)"
    - "opt-in writable 9P scratch for guest round-trip gates (srv9 --writable-prefix + lib/namespace's MCREATE bind)"
    - "execution gate calibration discipline: every marker regex anchored (?m)^...$ with \\r? and proven against the real capture (ls prints full paths; the donor RTC reports GMT)"

key-files:
  created:
    - sys/src/libmp/aarch64/mkfile
    - sys/src/libsec/aarch64/mkfile
    - sys/src/libthread/aarch64.c
    - sys/src/libmp/port/mpvecdigmulsub.c
    - build/aarch64/subset-build.sh
    - build/aarch64/subset-exec.sh
  modified:
    - sys/src/libmp/port/mpvecdigmuladd.c (sub function split out; 2021 PORT rename retired)
    - sys/src/libmp/port/mkfile (FILES += mpvecdigmulsub)
    - build/aarch64/srv9.py (--writable-prefix)
    - build/scripts/linux-build (build_aarch64 subset scope + arch activation)
    - nix/vm-build.sh (aarch64 rsync-back)
    - nix/build.sh (nm-shadow shim)
    - sys/src/BUILD_SETUP_NAMESPACE (/aarch64 bind)
    - sys/src/ape/lib/ap/mkfile, ape/lib/mp/mkfile, ape/lib/sec/mkfile (test -d clean guards)

key-decisions:
  - "factotum installs via mkmany's per-target rule (factotum.install): the dir's second TARG fgui needs libdraw/libcontrol (out-of-scope graphics stack) and is not in the named subset — the plan's own single-file idiom applied at dir level"
  - "build_aarch64 uses the PROVEN lib closure (libbio libregexp libip libndb libmp libsec libString libthread lib9p libauth libauthsrv), not the plan's shorter named list — the plan's flagged assumptions anticipated exactly this link-time discovery; libbio precedes the 7? loop (7a/a.h includes bio.h; the amd64 6? tools never use bio, so Phase 1's order never taught this)"
  - "libthread/aarch64.c is the donor's arm64.c byte-verbatim (JMPBUF macros identical in the committed aarch64/include/u.h) — factotum's dat.h pulls thread.h → libthread.a must exist for aarch64"
  - "mpvecdigmulsub lives in its own port file: empty-arch dirs (aarch64) get the real symbol from portable C; asm arches cull both port files via reduce; ape (whose reduce bind fails on Linux — pre-existing) can never duplicate-symbol because archive member names now align 1:1 with text symbols. The 2021 PORTmpvecdigmulsub rename (85f87b7b1) is retired"
  - "srv9.py --writable-prefix is opt-in and scoped (/tmp + /usr/glenda/tmp); the kernel permits creates via lib/namespace's MCREATE bind (createdir has no mode check), so only the server had to agree; boot-9qemu.sh/run-gate.sh never pass the flag"
  - "nix run .#build's rc=0 is never trusted as a pass signal (sys/src/BUILD's break-glass /bin/bash tail + BUILD_SETUP_NAMESPACE's cleanup swallow all failure statuses — deferred item 12); the gate is artifact checks + a zero-mk-failure log scan"

patterns-established:
  - "Execution evidence currency: the D-08 gate re-ran after the Task-3 libmp split changed factotum's link — the SUMMARY's execution claim covers the exact final binaries"
  - "Full-build failure scan vocabulary: zero hits for 'exit status=exit|Permission denied|don't know how|duplicate|skipping missing include|Can't read header' plus artifact existence = green (rc is meaningless)"

requirements-completed: [TC-05]

coverage:
  - id: D1
    description: "libmp/libsec aarch64 stubs (riscv64-pattern) + subset-build.sh: all sixteen subset binaries (rc grep ip/ipconfig auth/factotum cat cp date echo ls mkdir mv ps rm sed test wc) compile for aarch64 with Harvey's 7c in the lima namespace and install into /aarch64/bin; driver idempotent (second run green) and names the failing dir on abort"
    requirement: TC-05
    verification:
      - kind: integration
        ref: "sh build/aarch64/subset-build.sh — two consecutive green runs; .build/subset-build.log carries SUBSET-BUILD-OK; stub diff gates byte-exact modulo objtype"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-08 execution gate: every subset binary EXECUTES on the 9front donor kernel — build/aarch64/subset-exec.log carries all 20 per-command markers (echo/ls/cat/grep/sed/wc/date/test/rc-eval/ps/mkdir/cp/mv/rm/ipconfig-usage/factotum-resident) and SUBSET-OK, zero panic/trap/sys: note lines, three consecutive green runs (idempotence), final run against the post-split binaries"
    requirement: TC-05
    verification:
      - kind: e2e
        ref: "sh build/aarch64/subset-exec.sh ×3 (rc=0 each); grep -q SUBSET-OK + fault-scan '^0$' on .build/9front-dist/subset-exec.log"
        status: pass
    human_judgment: true
    rationale: "The plan carries an explicit <human-check>: a human should open .build/9front-dist/subset-exec.log and see the Harvey aarch64 subset visibly execute on the 9front donor kernel, command by command, ending in SUBSET-OK."
  - id: D3
    description: "D-09 NMOUNT audit: host nm (plan 02-03) over every installed subset binary finds zero nmount stub references — re-verified against the final full-build binaries after the libmp split changed links"
    requirement: TC-05
    verification:
      - kind: other
        ref: "plan Task 3 verify block 1 (hostbin nm loop over the 16 binaries, grep -w nmount empty) — run twice (post-subset-build and post-full-build), clean both times"
        status: pass
    human_judgment: false
  - id: D4
    description: "Dual-arch default build: linux-build carries arch=(amd64 aarch64) + subset-scoped build_aarch64 (no full-tree objtype=aarch64 install); nix/vm-build.sh rsyncs aarch64 back; nix run .#build green end-to-end — all three amd64 kernels + .elf variants (Phase 1 regression gate) AND the aarch64 subset + self-hosted 7a/7c/7l in the same run"
    requirement: TC-05
    verification:
      - kind: integration
        ref: "nix run .#build rc=0 + zero failure markers in log + artifact checks (amd64/9k10cpu{,.elf} 9k10cpuf{,.elf} 9k10f{,.elf}, aarch64/bin/{rc,auth/factotum,...}) + wiring greps (plan Task 3 verify block 2)"
        status: pass
    human_judgment: false

duration: 47 min
completed: 2026-11-01
status: complete
---

# Phase 02 Plan 07: Shell-Boot Subset Build + Execution + Dual-Arch Activation Summary

**The aarch64 shell-boot userland subset (rc, grep, ip/ipconfig, factotum + 12 core commands) compiles with Harvey's vendored 7c in the lima namespace, EXECUTES command-by-command on the 9front donor kernel under TCG with per-command markers and a closing SUBSET-OK (zero trap/panic lines, three consecutive green runs), passes the D-09 NMOUNT nm audit — and the default `nix run .#build` is now dual-arch: the Phase 1 amd64 kernel set (3 kernels + .elf) stays green while the same run produces the aarch64 subset and self-hosted 7a/7c/7l. TC-05 closed with execution evidence; the phase goal (Harvey compiles, links, and RUNS its own aarch64 Plan 9 binaries) is demonstrated.**

## Performance

- **Duration:** 47 min (plus harness compute: ~6 TCG boots/rebakes inside the runs)
- **Started:** 2026-11-01T15:17:29Z
- **Completed:** 2026-11-01T16:04:34Z
- **Tasks:** 3/3
- **Files created/modified:** 16 (6 created, 10 modified)

## Accomplishments

- **TC-05 build half (Task 1):** the 16-binary subset built green in the guest namespace through the committed 02-06 wiring (zero scratch injection) and rsynced back to the host's gitignored `aarch64/bin/`. The plan's flagged assumption about link-time library discovery proved out exactly: the real `#pragma lib` closure is larger than the named set (libbio for ls/ps/sed/wc/grep/7a; libregexp for sed; libndb for ipconfig; libString/libthread/lib9p for factotum) — all portable C, all built.
- **D-08 execution half (Task 2):** `subset-exec.sh` generalizes run-gate.sh — 16 binaries + a known-content `sentinel.txt` staged into the guest donor tree, CONF bootdir embed → devroot `/boot/h*`, targeted mkrootall rebake, TCG boot, and a 20-step paced/echo-verified exercise: echo sentinel, `ls /`, cat of the known file, grep and sed pipelines transforming the sentinel, `wc -l` counts (2 and 0), parseable `date`, `test -f` true-branch, rc command substitution (rc itself demonstrably evaluating), `ps` (shell resident), the full mkdir/cp/mv/rm round-trip in /tmp (via the new writable 9P scratch), ipconfig at usage level, factotum started `-n -s subexec` and observed resident in a later `ps` (pid 248, Pread), then SUBSET-OK. Three consecutive green runs; zero `panic|trap:|sys: ` lines.
- **D-09 audit (Task 3):** host nm over all sixteen binaries finds zero `nmount` references — twice (after the subset build and again after the full build replaced the binaries).
- **Dual-arch activation (Task 3):** `arch=(amd64 aarch64)` in linux-build with the subset-scoped `build_aarch64`; revert comments updated; `nix run .#build` green end-to-end with all six amd64 kernel artifacts plus the aarch64 subset.
- **Two real bugs found and fixed in-tree** (the phase's "boot-test, not build-test" lesson paying off again): the libmp `mpvecdigmulsub` split (below) and the nm-shadow autolib regression (02-03's hostbin nm broke plan9port 9l autolib for host-tool builds — first full build since 02-03 surfaced it; shimmed in nix/build.sh at toolchain.sh parity).

## Task Commits

1. **Task 1: libmp/libsec aarch64 stubs + namespace build of the shell subset** — `fb24ea695` (feat)
2. **Task 2: Execute the subset on the donor kernel (D-08 acceptance gate)** — `990b9c9e0` (feat)
3. **Task 3: D-09 NMOUNT audit + build_aarch64 subset-scoping + arch activation + full dual-arch regression** — `280460eee` (feat)

**Plan metadata:** see the final docs commit (SUMMARY/STATE/ROADMAP/REQUIREMENTS + deferred-items).

## Files Created/Modified

- `sys/src/libmp/aarch64/mkfile`, `sys/src/libsec/aarch64/mkfile` — empty-OFILES arch stubs, byte-identical to their riscv64 siblings modulo the objtype line (plan verify gate 1)
- `sys/src/libthread/aarch64.c` — donor arm64.c byte-verbatim (libthread's `$objtype.$O` member; factotum's pragma closure needs libthread.a to exist)
- `sys/src/libmp/port/mpvecdigmulsub.c` + `mpvecdigmuladd.c` (trimmed) + `port/mkfile` (FILES entry) — the split that retires the 2021 PORT rename (Deviation 3)
- `build/aarch64/subset-build.sh` — TC-05 build driver (vm-run.sh + generated guest script; libs in closure order; per-target installs; rsync-back; 16-binary assertion)
- `build/aarch64/subset-exec.sh` — D-08 gate (marker table in header; staging + CONF patch + rebake + boot + 20-step exercise + host assertions)
- `build/aarch64/srv9.py` — opt-in `--writable-prefix` (Tcreate/Twrite/Tremove/Twstat under scoped prefixes; qid.vers bumps + dir-cache invalidation; default read-only byte-identical)
- `build/scripts/linux-build` — subset-scoped `build_aarch64` + `arch=(amd64 aarch64)` + updated revert comments
- `nix/vm-build.sh` — rsync-back includes aarch64; `nix/build.sh` — nm-shadow shim
- `sys/src/BUILD_SETUP_NAMESPACE` — the /aarch64 bind (the nix/build.sh namespace path predated the arch dir)
- `sys/src/ape/lib/{ap,mp,sec}/mkfile` — `test -d` + `if not` guards on CPUS clean loops

## Decisions Made

- **factotum per-target install** (`mk factotum.install`): the dir's second TARG (fgui) needs libdraw/libcontrol — out-of-scope graphics. The plan's own single-file per-target idiom applied at the dir level; the named subset member ships.
- **build_aarch64 carries the PROVEN closure**, not the plan's pre-execution named subset of it — the plan's flagged assumptions explicitly anticipated link-time discovery; libbio precedes the 7? loop (7a/a.h includes bio.h — the amd64 6? tools never use bio, so Phase 1's ordering never taught this).
- **srv9 writable scratch is opt-in scoped** (`--writable-prefix`, two prefixes) rather than a second server or a CONF/kernel change: `lib/namespace` already binds `/usr/glenda/tmp` onto `/tmp` with MCREATE (createdir has no mode check — verified in donor chan.c), so only the server had to agree.
- **Execution evidence re-proven after link-affecting fixes:** the D-08 gate's third run executed the exact binaries the full build installs (the libmp split changed factotum's link layout).
- **rc=0 is not a pass signal** on this build path (sys/src/BUILD's `/bin/bash` break-glass tail + the namespace cleanup swallow all statuses — deferred item 12): the regression gate is artifact checks + a zero-mk-failure log scan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Driver lib list expanded to the full #pragma-lib closure**
- **Found during:** Task 1 (first run: `grep` link failed — `cannot open file: /aarch64/lib/libbio.a`)
- **Issue:** the plan's named lib set (libip libmp libsec libauth libauthsrv) is the declared-LIB surface; the subset's real link needs are carried by `#pragma lib` in headers (bio.h, regexp.h, ndb.h, String.h, thread.h, 9p.h) — the plan's flagged assumptions anticipated exactly this discovery
- **Fix:** subset-build.sh builds libc, libbio, libregexp, libip, libndb, libmp, libsec, libString, libthread, lib9p, libauth, libauthsrv in dependency order
- **Files modified:** build/aarch64/subset-build.sh
- **Verification:** all 16 binaries link and install; Task 2 executes them
- **Committed in:** fb24ea695 (Task 1)

**2. [Rule 2 - Genuine port bug, plan-sanctioned] libthread/aarch64.c imported (donor arm64.c, byte-verbatim)**
- **Found during:** Task 1 analysis (libthread's mkfile requires `$objtype.$O`; no aarch64.c existed)
- **Issue:** factotum's dat.h → thread.h → `#pragma lib "libthread.a"`; the archive must exist for aarch64
- **Fix:** imported the donor's arm64.c unchanged (JMPBUF macros identical in the committed aarch64/include/u.h — sha256-gated donor-verbatim since 02-06)
- **Files modified:** sys/src/libthread/aarch64.c (created)
- **Verification:** libthread.a builds; factotum links, starts, and stays resident on the donor kernel
- **Committed in:** fb24ea695 (Task 1)

**3. [Rule 1 - Bug] libmp mpvecdigmulsub: retired the 2021 PORT rename via a clean file split**
- **Found during:** Task 1 (factotum link: `mpdiv: undefined: mpvecdigmulsub`), then Task 3 (ape/libmp.a: `duplicate text symbol: mpvecdigmulsub`)
- **Issue:** 85f87b7b1 (2021) renamed the portable-C `mpvecdigmulsub` to `PORTmpvecdigmulsub` inside mpvecdigmuladd.c to dodge an amd64 duplicate — but that left empty-arch dirs (riscv64, aarch64) with NO mpvecdigmulsub at all (mpdiv calls it), while the rename's real masking target was ape's broken reduce bind. A plain revert fixed aarch64 but re-broke ape
- **Fix:** split the sub function into its own `port/mpvecdigmulsub.c` under its real name (+ FILES entry). Member names now align 1:1 with text symbols: empty-arch dirs get the real symbol from portable C; asm arches cull both port files via reduce; ape's same-name member replacement can never duplicate-symbol
- **Files modified:** sys/src/libmp/port/{mpvecdigmuladd.c, mpvecdigmulsub.c (new), mkfile}
- **Verification:** 7c compiles both files, one T symbol each (nm-checked); subset build green; full dual-arch build green (ape/libmp.a archives clean)
- **Committed in:** fb24ea695 (Task 1, rename) + 280460eee (Task 3, split)

**4. [Rule 3 - Blocking] factotum dir uses per-target install (fgui excluded)**
- **Found during:** Task 1 (factotum installed fine; the dir's second TARG fgui failed on missing libdraw.a)
- **Issue:** fgui needs the out-of-scope graphics stack (libdraw/libcontrol); building it would violate the subset scoping (REQUIREMENTS: prune rather than fix)
- **Fix:** `cd auth/factotum && objtype=aarch64 mk factotum.install` — the plan's single-file idiom at dir level
- **Files modified:** build/aarch64/subset-build.sh (and build_aarch64 in linux-build)
- **Verification:** factotum installs and executes; fgui simply not built
- **Committed in:** fb24ea695 (Task 1)

**5. [Rule 3 - Blocking] BUILD_SETUP_NAMESPACE gained the /aarch64 bind**
- **Found during:** Task 3 (first full `nix run .#build`: `skipping missing include file /aarch64/mkfile`, `7.c:10 6c: No such file or directory: /aarch64/include/ureg.h`, `don't know how to make '.cat'`)
- **Issue:** nsrun.sh (vm-run.sh path) conditionally binds /aarch64; the older nix/build.sh namespace path predates the arch dir — libc's CPUS recursion, libmach's 7.c, and all of build_aarch64 failed without it
- **Fix:** unconditional bind alongside the other arch dirs
- **Files modified:** sys/src/BUILD_SETUP_NAMESPACE
- **Verification:** full build green; zero missing-include warnings
- **Committed in:** 280460eee (Task 3)

**6. [Rule 3 - Blocking] ape/lib/{ap,mp,sec} mkfile clean loops guarded**
- **Found during:** Task 3 (full build: `Can't cd aarch64` aborting ape/lib clean)
- **Issue:** CPUS+=aarch64 (02-06) meets ape's missing per-arch dirs; and the rc guard needs an `if not` arm — a trailing failed `if(test -d)` test makes mk's `@{}` iteration abort (verified against plan9port rc on the host)
- **Fix:** `if(test -d $i) { ... } if not { echo skipping }` in the three CPUS-iterating clean/nuke loops
- **Files modified:** sys/src/ape/lib/{ap,mp,sec}/mkfile
- **Verification:** full build clean sweep (log shows `skipping aarch64 '(no dir)'`)
- **Committed in:** 280460eee (Task 3)

**7. [Rule 1 - Bug] nix/build.sh nm-shadow shim (phase-02 regression in the amd64 path)**
- **Found during:** Task 3 (full build: `nm: Can't read header for maketab.o` + undefined `Bopen/Brdline/Blinelen` linking awk's maketab)
- **Issue:** plan9port's 9l autolib calls bare `nm` on HOST ELF objects; 02-03 added Harvey's ken-format nm to hostbin (first on PATH in the build env), shadowing binutils nm and silently breaking autolib. toolchain.sh already shims this for the toolchain build; the full-build path lacked it. First full build since 02-03, so it surfaced here — exactly what this plan's regression gate exists for
- **Fix:** nix/build.sh installs the same nm-shadow shim (binutils nm from the wrapper PATH symlinked into a shimdir prepended for the duration)
- **Files modified:** nix/build.sh
- **Verification:** maketab links; zero `Can't read header` lines in the final build log
- **Committed in:** 280460eee (Task 3)

**8. [Rule 3 - Blocking] build_aarch64 lib order: libbio before the 7? loop**
- **Found during:** Task 3 (isolated pre-test: `7a` self-host link failed — `cannot open file: /aarch64/lib/libbio.a`)
- **Issue:** 7a/a.h includes <bio.h>; the amd64 6? tools never use bio so Phase 1's "libc then compilers" order never taught this
- **Fix:** libbio built right after libc in build_aarch64
- **Files modified:** build/scripts/linux-build
- **Verification:** 7a/7c/7l self-host and install for aarch64 in the full build
- **Committed in:** 280460eee (Task 3)

**9. [Rule 3 - Blocking, uncommitted local state] stale alphaobj.6 member pruned from amd64/lib/libmach.a**
- **Found during:** Task 3 (full build: `duplicate text symbol: _is7/_read7` archiving 7obj.6)
- **Issue:** `ar vu` never prunes members of removed sources — the pre-excision (02-02) alphaobj.6 still defined `_is7`/`_read7` (the letter-7 collision itself). Stale local artifact, not a source defect
- **Fix:** deleted the archive for regeneration (guest inherits via rsync --delete); deferred item 10 records the hygiene class
- **Files modified:** none (untracked build artifact)
- **Verification:** libmach.a rebuilds with exactly the current members; no duplicates
- **Committed in:** n/a (artifact removal; noted in deferred items)

**10. [Rule 1 - Bug] Exercise-driver regex calibration (three fixes, plan-sanctioned)**
- **Found during:** Task 2 bring-up runs
- **Issue:** (a) `ls /` prints full paths (`/boot`, not `boot`); (b) the donor RTC reports GMT, not UTC; (c) a `(?m)` omission made `$` anchor at buffer end; (d) ipconfig's usage line is `usage: /boot/hipconfig ...` (argv0 full path)
- **Fix:** regexes corrected to the observed reality (`^/boot$`, `(UTC|GMT)`, `(?m)` restored, `usage: .*hipconfig`); budget raised 900→1500 (TCG boot variance + 20 paced steps exceeded 900 mid-exercise — log proof: step 9b was never sent)
- **Files modified:** build/aarch64/subset-exec.sh
- **Verification:** three consecutive fully-green runs including the plan's exact verify chain
- **Committed in:** 990b9c9e0 (Task 2)

---

**Total deviations:** 10 auto-fixed (4 Rule 1 bug-class, 5 Rule 3 blocking-class, 1 Rule 2 missing-critical — plus the plan-sanctioned port-file import counted in Rule 2)
**Impact on plan:** Every deviation preserves or strengthens the plan's gates: the subset is exactly the named sixteen; D-08 execution evidence covers the final binaries; D-09 is clean twice; the amd64 regression gate caught two real bugs (mp symbol hygiene, nm shadowing) that are now fixed in-tree rather than worked around. No scope creep — the out-of-scope full userland stays pruned.

## Issues Encountered

- The plan's `<automated>` block for Task 3 could not know that `nix run .#build`'s rc is always 0 (BUILD's break-glass `/bin/bash` tail + namespace cleanup swallow statuses). The gate was therefore run as: rc (for the record) + zero-failure-marker log scan + the plan's artifact checks. Deferred item 12 proposes status plumbing in a hygiene phase.
- Several harness iterations of subset-exec.sh were needed for regex calibration (Deviation 10) — each costing a TCG boot; the final form is stable across three consecutive runs.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Phase goal demonstrated:** Harvey compiles and links its own aarch64 Plan 9 binaries on darwin-aarch64 (host tooling) and linux-aarch64 (guest builds), proven by real binaries that execute on the reference kernel.
- **ROADMAP SC#2** closed under the nm-only interpretation recorded in the plan's flagged assumptions (host nm built and proven in 02-03, exercised here for D-09; host acid deferred as research-scoped stretch with 7db.c landed in libmach).
- **ROADMAP SC#5** satisfied: subset builds for aarch64; generated SVC stubs freeze the user-side ABI (02-03); aout2uimage resolved as not needed (D-12).
- **D-08/D-09** closed with execution + audit evidence.
- **Phase 4 lead-ins:** `aarch64/ABI.md` + `aarch64/include/ureg.h` (02-06) are the frozen contract; `build/aarch64/hello.7.out` is the ABI bisect point; the sixteen subset binaries are the frozen-ABI userland proof set.
- **New deferred items:** 10 (archive member staleness), 11 (ape reduce bind failure), 12 (`nix run .#build` rc always 0).
- No blockers.

---

*Phase: 02-aarch64-toolchain-libc-9front-donor*
*Completed: 2026-11-01*

## Self-Check: PASSED

- All 7 key files verified on disk: `sys/src/libmp/aarch64/mkfile`, `sys/src/libsec/aarch64/mkfile`, `sys/src/libthread/aarch64.c`, `sys/src/libmp/port/mpvecdigmulsub.c`, `build/aarch64/subset-build.sh`, `build/aarch64/subset-exec.sh`, and this SUMMARY
- All 3 task commits found in git history: `fb24ea695`, `990b9c9e0`, `280460eee`
- Plan-level verification re-run at plan end: Task 1 stub-diff gates byte-exact; Task 1 idempotent second run green; Task 2 verify chain (bash -n + full run + SUBSET-OK grep + zero-fault scan) PASS on the final binaries; Task 3 D-09 nm audit clean on the final binaries; Task 3 wiring greps green; `nix run .#build` end-to-end green with all six amd64 kernel artifacts + the aarch64 subset present
- Measured commits (`git rev-list --count c40ef68e7..HEAD`): 3 — matches Task Commits
- No stubs: subset-build.sh and subset-exec.sh are wired end-to-end (driver → guest namespace build → rsync-back → bake → boot → exercise → assert); the subset-exec.log evidence is real captured serial, not scaffolding
- The 16 binaries in `aarch64/bin/` are the ones the execution gate ran (gate re-run after the Task-3 libmp split — evidence currency)
- Pre-existing unrelated working-tree modifications (rc/bin/Kill, postscript/devutf files) were never staged — every commit was explicit-path
