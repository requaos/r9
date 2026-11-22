---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 03
subsystem: infra
tags: [plan9, tiers, allowlist, donor-gate, aarch64, awk, 7l, linker, lp64, float, tracer, ledger, d-01, d-07, d-10, d-14]

requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-01)
    provides: FPdbleword uint fix (the float canary's header dependency) + gethunk fixes (the sweep can't wedge)
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-02)
    provides: the committed tier data files + header-only ledgers (D-13/D-14 grammar) + the objtype-gated mk filter
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: the vendored 7a/7c/7l toolchain + the subset venue (subset-build/subset-exec) this plan evolves in place
provides:
  - the tier gate stack proven end-to-end on real additions (libavl tier-0, cmd/awk tier-1): allowlist append → D-07 fresh-archive compile → T0-ART/T1-ART sha256 markers → bake → donor sourced sweep → per-binary serial markers → append-only ledger lines
  - D-01's float gate execution-asserted on the donor: awk prints 0.333 for the fixed input (T1-OK float-d01)
  - the sourced-sweep venue (D-10): committed tier1-sweep.rc + staged termrc override (trace-termrc precedent), drive9's typed per-binary steps retired
  - subset-exec.sh NATIVE_LINUX leg (Pitfall P6-4): limactl required only on darwin; --print-platform dispatch proof
  - 7l LP64 double-constant pool emission fix (2bbca81ed) — the tracer's catch, proven by donor execution
  - both D-14 ledgers seeded and classified (14 tier-0 lines; 25 tier-1 lines + 4 boot-chain supersede lines)
  - the phase evidence bundle: 06-EVIDENCE.md + evidence/06-03-tracer-donor.log
affects: [06-04 (coverage sweep rides the proven stack), 06-05 (22-lib tier-0 wave appends), 06-06 (tier-1 coreutils + self-host canary appends; PENDING-CANARY lines resolve), 06-07 (r9 venue; PENDING-CHAIN resolves; T1-SKIP write-flow lines engage), 06-08 (CI consumes the NATIVE_LINUX leg + ledger diff asserts), 10 (tiers 2-4)]

# Actuals (#2632) — chars/4 over the realized diff of the plan's seven commits
actuals:
  tokens: 23063
  tasks: 2
  commits: 7   # the plan's own commits; the ledger range below also contains 2 concurrent phase-07 docs commits

# Plan-commit ledger (#3968) — measured, includes concurrent-session commits in range
commits: 9
plan_head_before: e32ca76156487485637613253fd9a025a599ae93
plan_head_after: a8e93784cc339c67667f26107d1f69910f8d7872

tech-stack:
  added: []
  patterns:
    - "tier-file-driven build driver: while-read loops over the D-13 data files with first-whitespace-cut field extraction; entry-form dispatch (single/dir/nested/dir:target/9k-boot/stage-*) via shell case + the -d dir test"
    - "D-07 fresh-archive + T0-ART/T1-ART sha256 artifact markers (mk exit status never trusted — 02-12); cmd/cc's cc.a7 gets the same fresh-archive hygiene (mklib's ar vu is the same never-prune rule)"
    - "sourced in-guest sweep via staged termrc override: the override swaps the served rootfs's rc/bin/termrc for the run (cleanup-trap restore + stranded-override self-heal), its tail execs `rc -c '. /boot/tier1-sweep.rc donor'`; init's pass-2 manual rc gives drive9 its shell proof AFTER the sweep — zero console interaction during the sweep (D-10)"
    - "CONF bootdir awk patch made self-healing: stale subset lines dropped before the block insert (converges from any prior state), marker-guarded on tier1-sweep.rc"
    - "sweep check shapes: fn check for status-only, inline backquote + rc ~ pattern for output checks; T1-SKIP named lines for write-flow entries on read-only-root venues; failure never aborts the sweep (USPACE-04 empty edge)"
    - "nm shim for host-toolchain generator links (9l autolib execs bare nm on host ELF objects; Harvey's ken nm breaks it) — nix/build.sh:16-32's idiom mirrored into subset-build.sh's guest script"

key-files:
  created:
    - build/aarch64/tier1-sweep.rc
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/evidence/06-03-tracer-donor.log
  modified:
    - build/aarch64/subset-build.sh
    - build/aarch64/subset-exec.sh
    - build/aarch64/tiers/tier0-libs.txt
    - build/aarch64/tiers/tier1-cmds.txt
    - build/aarch64/markers/tier0.ledger
    - build/aarch64/markers/tier1.ledger
    - sys/src/cmd/7l/asm.c
    - .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/deferred-items.md

key-decisions:
  - "06-03: the staged termrc override swaps the SERVED rootfs's rc/bin/termrc (guest scratch, trap-restored, self-healing on a stranded override) — the donor's own init/pass structure (pass-1 rc -c sources termrc; pass-2 manual rc) means the sweep runs once at boot and drive9's shell proof lands on the pass-2 shell; no /lib/namespace edit needed"
  - "06-03: boot-chain ledger honesty (USPACE-04 transparency): stage/stacksmoke carries BOOT:donor with the 06-03 gate log (the staged termrc runs it); 9k/boot, stage/init, cmd/paqfs:paqfs carry BOOT:donor with the 05-03 real-chain trace as evidence — the subset-exec venue boots the DONOR's own chain, so the 06-03 log cannot evidence our boot(8)/init/paqfs; the plan text's 'evidence = the gate log path' was written on a wrong premise and the prohibition outranks it"
  - "06-03: transient exec vocabulary extension PENDING-CHAIN:06-07 for the boot-chain three in Task 1's seed lines (superseded by Task 2's BOOT:donor lines per last-line-wins) — an honest not-yet-classified state beats an unproven OK"
  - "06-03: the tracer caught a real toolchain bug class the phase exists for — 7l's LP64 double-constant pool emission wrote only the low 32 bits (struct Ieee is 16 bytes on LP64 hosts); fixed and proven by donor execution (float-print 0.333); 6l's embedded-Ieee variant is the same latent class, logged to deferred-items (amd64 leg byte-untouched)"
  - "06-03: drive9's panic fast-fail aborts the boot drive on a swept binary's userspace fault (its panic|trap: regex matches the kernel's per-proc 'sys: trap:' print) — the sweep still runs to T1-DONE guest-side, so the gate's failure path now reports the full marker state for triage"

patterns-established:
  - "tracer feedback gate workflow: prove one lib + one cmd through the whole stack, then re-run the gate (two consecutive green runs) before any expansion"
  - "stage-instrumented debug probes via TIER1_TERMCMD + the served root's writable /tmp (zero rebake cycles per iteration) — the diagnosis pattern for future in-guest faults"
  - "D-14 ledger lines name the machinery commit the gate RAN (self-reference: ledger lines land as a follow-up commit carrying the machinery commit's SHA)"

requirements-completed: [USPACE-04]

coverage:
  - id: D1
    description: "tier-file-driven build machinery: subset-build.sh tier-0 fresh-archive loop with T0-ART sha256 markers (14 libs incl. libavl) + tier-1 entry-form dispatch with T1-ART markers (24 entries incl. awk via the full G1 host-safe maketab generator path); hardcoded v1.0 loops gone; host-side artifact asserts + od R_MAGIC checks"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "sh build/aarch64/subset-build.sh (two green runs; run-3 log: 14 T0-ART + 24 T1-ART + TIER-BUILD-OK; T0-ART libavl + T1-ART awk present; awk's proctab.c generated by host-built unix.maketab per the mkfile's own rules)"
        status: pass
    human_judgment: false
  - id: D2
    description: "donor execution gate end-to-end: staged termrc override sources tier1-sweep.rc at boot; 19 T1-OK markers (incl. T1-OK awk, T1-OK float-d01 '0.333' — D-01 execution-asserted) + T1-DONE; zero T1-FAIL; zero panic|trap:; drive9 shell proof on the pass-2 rc; two consecutive green runs (gate + tracer-feedback re-verify)"
    requirement: USPACE-04
    verification:
      - kind: e2e
        ref: "sh build/aarch64/subset-exec.sh — PASS twice consecutively; evidence .planning/phases/06-*/evidence/06-03-tracer-donor.log (the re-verify run)"
        status: pass
    human_judgment: false
  - id: D3
    description: "7l LP64 double-constant pool emission fix (the tracer's catch): binary literal pools carry correct 64-bit double constants; donor-proven via awkprobe stage bisection (P7/P8/P12 correct bits; P14 float-print 0.333; PROBE-OK)"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "linked-binary constant bytes correct (2^32 at 0xaa60, 1/3 at 0xa872) + donor probe run P14 'float-print 0.333' + the full gate green with float-d01"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-14 ledgers seeded + classified: 14 tier-0 lines (13 CARRIED:tier1-consumer with grep-evidence carriers + libavl EXCLUDED:no-tier1-consumer), 25 tier-1 lines + 4 boot-chain BOOT:donor supersede lines with honest evidence paths; 8-field lint + per-dir coverage green"
    requirement: USPACE-04
    verification:
      - kind: other
        ref: "awk NF!=8 lint → LEDGER-FORMAT-OK; per-dir coverage over both tier files → PER-DIR-COVERAGE-OK; last-line-per-dir awk audit (all tier-0 latest lines CARRIED/EXCLUDED)"
        status: pass
    human_judgment: false
  - id: D5
    description: "subset-exec.sh NATIVE_LINUX leg (Pitfall P6-4): limactl required only on the darwin leg; donor fetch + pin-verify host-side on both; self-re-entrant rebake runs in the host's own namespace on linux"
    requirement: USPACE-04
    verification:
      - kind: other
        ref: "sh build/aarch64/subset-exec.sh --print-platform → platform=darwin limactl=required; FORCE_NATIVE_LINUX=1 ... --print-platform → platform=native-linux limactl=not-required"
        status: pass
    human_judgment: true
    rationale: "dispatch-proven only on the dev host; the leg's first real end-to-end native-linux run is 06-08's CI tier-donor job (the donor tree build on native linux is wired there) — a human/CI run must confirm"
  - id: D6
    description: "phase evidence bundle started: 06-EVIDENCE.md (gate stack description, tracer run record with commit SHA, carrier mapping table, boot-chain classification, append-format note) + evidence/06-03-tracer-donor.log retained"
    requirement: USPACE-04
    verification:
      - kind: other
        ref: "both files committed; the log contains T1-DONE and the full marker set"
        status: pass
    human_judgment: false

duration: 71 min
completed: 2026-11-05
status: complete
---

# Phase 6 Plan 03: Tier Gate Stack Tracer — libavl + awk End-to-End Summary

**The entire tier gate stack is proven end-to-end on real allowlist additions — libavl (tier-0) and cmd/awk (tier-1) ride append → D-07 fresh-archive compile → sha256 artifact markers → bake → donor-kernel sourced sweep → per-binary serial markers → append-only ledger lines — and the tracer did its job: it caught a latent LP64 double-constant emission bug in 7l (awk's float prints aborted with a null read), which is now fixed and execution-proven with the D-01 canary printing 0.333 on the donor.**

## Performance

- **Duration:** 71 min
- **Started:** 2026-11-05T00:13:45Z
- **Completed:** 2026-11-05T01:24:53Z
- **Tasks:** 2
- **Files modified:** 11 (8 build/tier files + 7l/asm.c + 2 planning artifacts) + 3 created

## Accomplishments

- **The gate stack is live and proven twice consecutively** (the initial gate + the tracer-feedback re-verify): 14 tier-0 libs with D-07 fresh-archive + T0-ART sha256 markers; 24 tier-1 entries via entry-form dispatch with T1-ART markers; the donor gate's 19-marker sweep (incl. `T1-OK awk`, `T1-OK float-d01`, `T1-DONE`), zero T1-FAIL, zero panic|trap:, drive9's shell proof on the pass-2 rc.
- **The tracer caught a real bug** (its entire purpose): hawk faulted with `suicide: sys: trap: fault read addr=0x0` on any float print. Stage-bisection via a probe binary (TIER1_TERMCMD hook + the served root's writable /tmp, zero rebake cycles) pinned it: double CONSTANTS materialized with hi=0 while FPU-computed doubles were correct; object files held correct bytes; the linked binary's pool entries did not. Root cause: 7l's datfill emitted D_FCONST case-8 via a cast over `struct Ieee {long l; long h;}` — 16 bytes on LP64 hosts, so h never reached the binary. Fixed (`2bbca81ed`) and execution-proven.
- **D-10's sourced sweep retired the typed step table**: committed `tier1-sweep.rc` (fn check wrapper, fixed inputs re-expressing the v1.0 table, venue parameter with named T1-SKIP write-flow lines for r9's read-only-root era, T1-DONE + exit tail) + a staged termrc override swapped into the served rootfs for the run (trap-restored, self-healing).
- **Pitfall P6-4 closed**: subset-exec.sh's NATIVE_LINUX leg runs the donor rebake in the host's own namespace; limactl is required only on darwin; `--print-platform` proves dispatch (darwin→lima vs FORCE_NATIVE_LINUX=1→nsrun-sudo-unshare).
- **D-14 ledgers seeded honestly**: 14 tier-0 lines (13 CARRIED:tier1-consumer with grep-evidence carriers; libavl EXCLUDED:no-tier1-consumer — avl.h's only tree-wide consumer is venti/copy.c), 25 tier-1 lines (18 swept T1-OK:donor; 4 build-chain PENDING-CANARY:06-06; 3 boot-chain PENDING-CHAIN:06-07 → Task-2 BOOT:donor supersedes), all 8-field clean, per-dir coverage green.
- **Phase evidence bundle exists**: `06-EVIDENCE.md` (gate stack, run record, carrier mapping, boot-chain classification, append-format note) + `evidence/06-03-tracer-donor.log`.

## Carrier-mapping grep evidence (06-EVIDENCE.md carries the table)

`grep -lE '<lib\.h>'` over the swept tier-1 sources (2026-11-05):

```
libbio:     awk/*.c(all), grep/grep.h, ip/ipconfig/{ipv6,main,ppp}.c, ls.c, ps.c, sed.c, wc.c (+paqfs)
libregexp:  sed.c, awk/re.c
libip:      ip/ipconfig/{ipv6,main,ppp}.c (+factotum/secstore.c)
libndb:     ip/ipconfig/{main,ppp}.c
libmp:      auth/factotum/dat.h (+paqfs)
libsec:     auth/factotum/dat.h (+paqfs)
libString:  auth/factotum/dat.h
libthread:  auth/factotum/dat.h
lib9p:      auth/factotum/dat.h
libauth:    auth/factotum/dat.h (+paqfs/paqfs.c)
libauthsrv: auth/factotum/dat.h
libflate:   paqfs/{mkpaqfs,paqfs}.c — no swept carrier; paqfs's donor evidence is the 05-03 trace (BOOT:donor)
libavl:     (none — tree-wide only venti/copy.c) → EXCLUDED:no-tier1-consumer
libc:       universal (u.h/libc.h in every swept binary)
```

## Task Commits

1. **Task 1 machinery:** `32e1ce685` — `feat(06-03): tier gate stack tracer — libavl + awk end-to-end on the donor (D-07/D-10/D-14, USPACE-04)`
2. **Task 1 deviation fix:** `1835813b7` — `fix(06-03): nm shim for host-toolchain generator links (9l autolib) in the tier build`
3. **Task 1 deviation fixes:** `26267b5bb` — `fix(06-03): sweep cat check matches the sentinel's first line; report sweep markers on drive failure`
4. **Task 1 deviation fix (the tracer's catch):** `2bbca81ed` — `fix(06-03): 7l LP64 double-constant pool emission — write the two 32-bit Ieee halves explicitly`
5. **Task 1 diagnostic tooling:** `7123bce9f` — `feat(06-03): TIER1_TERMCMD diagnostic override hook in subset-exec.sh`
6. **Task 1 ledgers:** `b025a38f3` — `feat(06-03): seed tier ledgers from the tracer gate run (D-14)`
7. **Task 2:** `a8e93784c` — `docs(06-03): tracer evidence bundle + ledger exec classification (D-6)`

**Plan metadata:** the final docs commit rides on top (SUMMARY + STATE/ROADMAP/REQUIREMENTS).

## Files Created/Modified

- `build/aarch64/subset-build.sh` — hardcoded loops → tier-file loops (tier0 fresh-archive + T0-ART; tier-1 entry-form dispatch + T1-ART; cc.a7 fresh-archive hygiene; nm shim; TIER-BUILD-OK); the 07-04 dd/netblast additive blocks preserved untouched
- `build/aarch64/subset-exec.sh` — NATIVE_LINUX leg + --print-platform; staging gains hawk/hstacksmoke/tier1-sweep.rc/hello.c; self-healing CONF bootdir awk patch; staged termrc override (trap-restored, self-healing); drive9 without typed steps; marker assertion block (19 T1-OK set + T1-DONE + zero-T1-FAIL + zero-panic|trap:); TIER1_TERMCMD diagnostic hook
- `build/aarch64/tier1-sweep.rc` — new committed sourced sweep (146 lines)
- `build/aarch64/tiers/tier0-libs.txt` — +libavl (06-03 tracer comment)
- `build/aarch64/tiers/tier1-cmds.txt` — +cmd/awk (06-03 tracer comment)
- `build/aarch64/markers/tier0.ledger` — 14 marker lines
- `build/aarch64/markers/tier1.ledger` — 25 seed lines + 4 BOOT:donor supersede lines
- `sys/src/cmd/7l/asm.c` — the LP64 double-constant emission fix (datfill D_FCONST case 8 → explicit int[2] halves)
- `.planning/phases/06-*/06-EVIDENCE.md` + `evidence/06-03-tracer-donor.log` — the phase evidence bundle
- `.planning/phases/06-*/deferred-items.md` — the 6l-class latent bug logged

## Decisions Made

- **Staged termrc override mechanics:** swap the served rootfs's `rc/bin/termrc` for the run's duration (backup in TMPD, cleanup-trap restore, stranded-override self-heal from the pristine donor checkout). The donor init's pass structure (pass-1 sources termrc; pass-2 is the manual interactive rc) makes the sweep run exactly once at boot and preserves drive9's unchanged shell proof — no /lib/namespace edit, no typed console interaction during the sweep.
- **Boot-chain honesty over plan-text literalism** (USPACE-04 prohibition outranks): the 06-03 venue boots the donor's own chain, so 9k/boot, stage/init, cmd/paqfs:paqfs carry BOOT:donor with the 05-03 real-chain trace as their evidence path (their true donor-kernel execution proof), and stage/stacksmoke carries BOOT:donor with the 06-03 gate log (the staged termrc runs it). PENDING-CHAIN:06-07 was added to the exec vocabulary as the transient Task-1 state for the boot-chain three.
- **The 7l fix's blast radius kept minimal:** 6l's embedded-Ieee variant shares the latent class but has no current consumer (the amd64 build carries no non-trivial double data constants) — logged to deferred-items.md; amd64 leg byte-untouched.
- **Ledger commit-field semantics:** lines name the machinery commit the gate RAN (7123bce9f595…) — self-reference discipline per the plan (ledger lines land as a follow-up commit).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] nm shim for the 9l-autolib generator link (G1 path)**
- **Found during:** Task 1 first build run — cmd/awk's maketab generator chain failed: `nm: Can't read header for maketab.o` → undefined Bopen/Brdline/Blinelen at link
- **Issue:** plan9port 9l's autolib execs bare `nm` on host ELF objects; the devshell PATH puts $HARVEY_LINUX_BIN first, so bare nm resolved to Harvey's ken-format nm — silently broken autolib
- **Fix:** mirrored nix/build.sh:16-32's proven shim into subset-build.sh's guest script (symlink the wrapper PATH's binutils nm into a shim dir, prepended for the script's duration) — the mkfile's own G1 rules run end-to-end (no plant), so the 06-04 audit's G1 row is proven live
- **Files modified:** build/aarch64/subset-build.sh
- **Verification:** full green rebuild; the log shows `9c -FTVw maketab.c` / `9l -o ...maketab` / `./$cputype.maketab >proctab.c`
- **Committed in:** 1835813b7

**2. [Rule 1 - Bug] sweep cat check matched the whole two-line output against the one-line expectation**
- **Found during:** Task 1 first gate run (T1-FAIL cat on a working hcat)
- **Issue:** the check compared `hcat /boot/sentinel.txt`'s FULL output (both sentinel lines) to 'alpha bravo charlie'; v1.0's step asserted the anchored first line
- **Fix:** match `'alpha bravo charlie '*` (first line + trailing second-line coverage)
- **Files modified:** build/aarch64/tier1-sweep.rc
- **Verification:** re-run gate → T1-OK cat
- **Committed in:** 26267b5bb

**3. [Rule 1 - Bug] drive9's panic fast-fail hides sweep marker state on a userspace fault**
- **Found during:** Task 1 first gate run (drive9 aborted at hawk's fault — its `panic|trap:` regex matches the kernel's per-process `sys: trap:` print)
- **Issue:** the gate exited without reporting the sweep's marker state (the sweep had still run to T1-DONE guest-side — USPACE-04's never-wedge edge held)
- **Fix:** the failure path now reports the full T1-OK/FAIL/SKIP marker set before exiting (the harness's fast-fail itself is deliberate and shared — not touched)
- **Files modified:** build/aarch64/subset-exec.sh
- **Verification:** the diagnostic runs' failure output carried the marker state; the fixed tree's gate is green without it
- **Committed in:** 26267b5bb

**4. [Rule 1 - Bug] 7l LP64 double-constant pool emission (the tracer's catch)**
- **Found during:** Task 1 first gate run — hawk faulted `suicide: sys: trap: fault read addr=0x0` on both float checks
- **Issue:** 7l's datfill emitted 8-byte double constants via a cast over `struct Ieee {long l; long h;}`; on LP64 hosts the struct is 16 bytes, so the emit wrote l's low half + l's zero high half — h never reached the binary; every double constant materialized with hi=0 (2^32→0.0), and strtod's umuldiv guard aborted the process. Latent since Phase 2 (no v1.0 gate ever printed a float on aarch64)
- **Fix:** emit the two 32-bit halves explicitly via `int fhw[2]` (same packed layout on every host) — sys/src/cmd/7l/asm.c
- **Diagnosis method (recorded for reuse):** awkprobe stage bisection on the donor via the TIER1_TERMCMD hook + the served root's writable /tmp (P0-P6 startup clean; P7/P8 constants hi=0; P9/P10/P11 FPU compute/compare correct; object bytes correct; linked binary pool bytes wrong → 7l), then 7l instrumentation (7LPOOL correct at creation → garbage at datfill → the struct-cast emission)
- **Verification:** relinked probe binary carries correct 2^32/1/3 constants (0xaa60/0xa872); donor probe run prints `P14 float-print 0.333` + PROBE-OK; full gate green with T1-OK float-d01 twice consecutively
- **Committed in:** 2bbca81ed

**5. [Rule 3 - Blocking] TIER1_TERMCMD diagnostic hook**
- **Found during:** the 7l fault diagnosis — bisecting in-guest behavior needed a probe run without a full rebake per cycle
- **Fix:** subset-exec.sh's staged termrc tail execs $TIER1_TERMCMD when set (unset in all gate flows; sweep path byte-identical when unset)
- **Files modified:** build/aarch64/subset-exec.sh
- **Verification:** three probe gate cycles drove the 7l diagnosis
- **Committed in:** 7123bce9f

### Plan-accuracy deviation (judgment, documented)

**6. Boot-chain evidence paths (Task 2 step 3's premise was wrong)**
- **Issue:** the plan asserted 9k/boot, stage/init, stage/stacksmoke, cmd/paqfs:paqfs "execute on every gated boot by construction" with "evidence = the gate log path". The 06-03 subset-exec venue boots the DONOR's own chain (donor kernel + donor init + served rootfs) — our boot(8)/init/paqfs never run in it
- **Resolution:** honoring the USPACE-04 transparency prohibition (never claim execution without serial-marker evidence): stage/stacksmoke carries BOOT:donor with the 06-03 gate log (the staged termrc runs it); the other three carry BOOT:donor with the retained 05-03 real-chain trace (their true donor-kernel execution evidence: our boot(8)/init/paqfs ran their real flows on the donor kernel there). The four entries carry exec=BOOT:donor per the plan; the evidence field names honest paths

---

**Total deviations:** 5 auto-fixed (3 Rule 1 bugs, 2 Rule 3 blocking) + 1 documented plan-accuracy resolution
**Impact on plan:** the tracer did exactly what it exists for — one real toolchain bug caught and fixed at the foundation instead of under 58 entries. No scope creep; every deviation is gate-machinery correctness or the bug the gate exists to catch.

## Issues Encountered

- **Toolchain-change freshness gap (02-10's generalized class):** mk's freshness graph never names the toolchain binaries, so after the 7l fix the tier-1 cmd binaries would have relinked nothing (stale objects → stale binaries). Resolved for this run by the guest tree's .7-clean state (vm-run's host→guest `--delete` rsync swept them; host tree carries no .7 litter) + the fixed 7l copied into the guest hostbin. **Watch item for 06-05/06-06:** the committed tier loops have no committed anti-staleness mechanism for cmd binaries after a toolchain change (D-07 covers only the .a archives) — consider a per-entry object sweep or a documented operator step when the toolchain changes.
- **Concurrent-session cadence:** the phase-07 session continued landing docs commits mid-plan (in the measured ledger range); the lima guest stayed usable throughout (guest-state partitioning held).
- **`init: can't open /adm/timezone/local`** on the donor guest — pre-existing srvroot content gap (the dist assembly mkdirs /adm but doesn't populate timezone); non-fatal (init continues). Not introduced here; noted for the donor-dist assembly's eventual owner.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **06-05/06-06 unblocked**: the 22-lib tier-0 wave and the ~32-coreutils + build-chain expansion extend a PROVEN slice — append a line, gate, append a ledger line. The PENDING-CANARY:06-06 lines (7a/7c/7l/cc) await D-11's self-host canary; the sweep's write-flow T1-SKIP lines await 06-07's r9 venue.
- **06-04 unblocked**: the coverage sweep rides the proven stack; the G1 generator row's host-safe proof is live in the build log (9c/9l maketab chain).
- **06-07/06-08**: the r9 venue resolves PENDING-CHAIN (the boot-chain three) via the composed image; the NATIVE_LINUX leg unblocks the CI tier-donor job.
- **Watch items:** the toolchain-freshness gap (above); the 6l-class latent bug (deferred-items.md); 06-02's recorded plan-gate-2 text repair (queued to 06-03 planning per its SUMMARY — surfaced here for the next planning pass).

## Self-Check: PASSED

- [x] `build/aarch64/tier1-sweep.rc` exists (146 lines, #!/bin/rc, venue param, fn check, 0.333 canary, T1-DONE+exit tail)
- [x] `build/aarch64/subset-build.sh` reads both tier files, emits T0-ART/T1-ART, hardcoded `for d in libc libbio` loop gone
- [x] `build/aarch64/subset-exec.sh` carries the NATIVE_LINUX branch (limactl darwin-only) + stages tier1-sweep.rc as bootdir data
- [x] tier0-libs.txt gained exactly libavl; tier1-cmds.txt gained exactly cmd/awk
- [x] tier0.ledger: 14 tier=0 lines; tier1.ledger: 25 seed + 4 supersede lines; 8-field lint + per-dir coverage green
- [x] `evidence/06-03-tracer-donor.log` exists and contains T1-DONE; `06-EVIDENCE.md` exists with the tracer run record
- [x] Commits found on master: 32e1ce685, 1835813b7, 26267b5bb, 2bbca81ed, 7123bce9f, b025a38f3, a8e93784c
- [x] Donor gate green TWICE consecutively (initial + tracer-feedback re-verify): full T1-OK set + float-d01 + T1-DONE, zero panic|trap:
- [x] No pre-existing dirty-tree files staged in any commit (explicit path lists throughout; rc/bin/Kill, sys/lib/postscript/**, sys/lib/troff/**, sys/src/cmd/postscript/**, .planning/config.json, graphs/tmp/state.json untouched)

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
