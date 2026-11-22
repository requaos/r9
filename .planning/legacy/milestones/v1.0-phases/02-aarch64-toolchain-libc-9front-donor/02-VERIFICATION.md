---
phase: 02-aarch64-toolchain-libc-9front-donor
verified: 2026-11-01T19:37:35Z
status: passed
score: 5/5 must-haves verified
covered_files:
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
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-01-PLAN.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-01-SUMMARY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-02-PLAN.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-02-SUMMARY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-03-PLAN.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-03-SUMMARY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-04-PLAN.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-04-SUMMARY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-05-PLAN.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-05-SUMMARY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-06-PLAN.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-06-SUMMARY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-07-PLAN.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-07-SUMMARY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-CONTEXT.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-REVIEW.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-REVIEW-DISPOSITION.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-REVIEW-FIX.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-SECURITY.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-UAT.md
  - .planning/phases/02-aarch64-toolchain-libc-9front-donor/deferred-items.md

covered_digest: "v2:sha256:6b3c43b439b9e554eeff4753398980bcdcc7dd75247cbc6f5af852edda5520dc"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 5/5
  gaps_closed:
    - "Human item 1 (D-08 subset execution log eyeball): human approved the captured subset-exec.log in-conversation 2026-11-01 — 02-UAT.md test 1 passed"
    - "Human item 2 (CR-01 + WR-01..10 disposition): human chose fix-now 2026-11-01; gsd-code-fixer applied all 11 in-scope fixes (02-REVIEW-FIX.md status all_fixed, commits 064c6a185..b4e652801) — 02-UAT.md test 2 passed"
  gaps_remaining: []
  regressions: []
---

# Phase 02: aarch64 Toolchain + libc (9front donor) Verification Report

**Phase Goal:** Harvey can compile and link its own aarch64 Plan 9 binaries on darwin-aarch64 and linux-aarch64 hosts — the toolchain, libc, and objtype wiring exist and are proven by real binaries, not scaffolding.
**Verified:** 2026-11-01T19:37:35Z
**Status:** passed
**Re-verification:** Yes — after human-item closure and code-review fix commits (064c6a185..b4e652801). The prior run (2026-11-01T16:31:54Z, `human_needed`, 5/5) went stale when the fix commits touched covered harness files. Both outstanding human items are now resolved per 02-UAT.md (status complete, 2/2 passed); this run confirms the fixes landed as recorded and regressed no must-have truth.

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `7a`/`7c`/`7l` vendored from 9front into `sys/src/cmd/` with BUILD scripts; build natively via the toolchain app on darwin-aarch64 and linux-aarch64 | ✓ VERIFIED | **Regression check (this run):** hostbin binaries (Oct 31 12:51 — untouched by fix commits; `git log` on 7a/7c/7l/cc/libmach since prior verify is EMPTY). Live re-run: `7c -FVw hello.c` (the committed gate source) exit 0 → 711-byte `hello.7`; `7a` assembled in-tree `sys/src/libc/aarch64/setjmp.s` (→ 492-byte .7) and a generated-style SVC stub (TEXT/MOV R0,0(FP)/MOV $19,R0/SVC/RETURN → 282-byte .7), both exit 0; `7l` parsed the object and resolved the `libc.a` pragma to the namespace path `/aarch64/lib/libc.a` (correct by design — the full link is a namespace-build operation; the committed `hello.7.out` is the linked product, sha256-verified). Vendoring fidelity spot-checked: vendored `7a/lex.c` vs pinned donor `350266104f` differs ONLY in the thestring shim + the documented LP64 `int l` fix; donor pin live-verified via `rev-parse`. linux-aarch64 half unchanged: guest donor-kernel build provenance in donor-build.log stands |
| 2 | `nm`/`acid` operate on 7l aarch64 objects — libmach arm64 support landed, letter-7 collision resolved | ✓ VERIFIED | **Regression check (this run):** hostbin `nm` re-run live — decoded committed `hello.7.out` exit 0 (`_callmain` T 10058 et al.); decoded `aarch64/bin/rc` (530 symbols) with 0 `nmount` references (D-09 audit re-run). libmach sources untouched since prior verify. nm-only interpretation recorded at initial verification stands (7db.c tables landed; no host acid built) |
| 3 | 7c-compiled, 7l-linked hello-world aarch64 binary produced and runs, committed BEFORE any `aarch64/` arch dir, libc dir, or CPUS entry lands | ✓ VERIFIED | **Regression check (this run):** `hello.7.out` sha256 `1c89e0990e386ae5…` re-computed — matches all 3 PASS blocks in committed `gate-evidence.log`. Git ancestry re-verified live: execution-evidence commit `62351f81f` IS ancestor of first `aarch64/` commit `a7945d8b5` (`merge-base --is-ancestor`); binary commit `e60e168dc` precedes CPUS/pcc commit `0f8ea566f`. `gate-evidence.log` untouched by fix commits (last commit remains `62351f81f` — CR-01 fix was script-only, confirmed). Human-approved at tracer gate; 3 `gate: PASS` blocks, 0 fault lines |
| 4 | `objtype=aarch64` build wiring works end-to-end: `aarch64/mkfile` (CC=7c O=7), `pcc.c` entry, `mkfile.proto` CPUS, config section named `aarch64` | ✓ VERIFIED | **Regression check (this run):** `aarch64/mkfile` re-read: `CC=7c LD=7l AS=7a O=7` + mkfile.proto include. `pcc.c:21` row `{"aarch64", "7c", "7l", "7", "7.out"}` present. `mkfile.proto`: `CPUS=amd64 arm 386 riscv riscv64 power mips aarch64`, `OS=568ijqv7`. `linux-build:73` `fn build_aarch64` + `arch=(amd64 aarch64)` dispatch (Assumption A1 realization of "config section"). Committed aarch64 libc.a present (779,312 bytes). WR-10's mkdirs shebang fix (bash) touches this wiring's helper — verified: shebang `#!/usr/bin/env bash`, `bash -n` clean, functional run recorded in 02-REVIEW-FIX.md |
| 5 | Shell-boot userland subset (rc, ipconfig, factotum, core cmds) builds for aarch64 with `sys/src/libc/aarch64/` + generated `9syscall` SVC stubs freezing the Harvey↔r9 user ABI | ✓ VERIFIED | **Regression check (this run):** 19 executable files under `aarch64/bin/` = the 16 subset (rc grep cat cp date echo ls mkdir mv ps rm sed test wc + ip/ipconfig + auth/factotum) + the three aarch64 toolchain binaries (7a/7c/7l, per plan 02-07's `cmd/7?` scope); R_MAGIC `00 00 8c 47` re-verified live on rc, factotum, ipconfig. `9syscall/mkfile` `case aarch64` block re-read line-by-line (TEXT / `MOV R0, 0(FP)` / `MOV $n, R0` / SVC / RETURN + `_seek` rename). `sys/src/libc/aarch64/` intact (16 asm + notejmp.c + _seek.c + mkfile + README). subset-exec.log content re-verified: SUBSET-OK present, 0 panic/trap/`sys: ` fault lines across the 169-line capture — **and the human eyeball is now recorded** (02-UAT.md test 1 passed 2026-11-01) |

**Score:** 5/5 truths verified (0 behavior-unverified)

### Re-Verification: Fix-Commit Verification (the stale-digest trigger)

All 11 fix commits (`064c6a185..b4e652801`) confirmed in git history; the touched-file union matches 02-REVIEW-FIX.md exactly (13 files); working tree clean for all 13. Every fix spot-checked against the live tree — not the fix report's claims:

| Fix | Commit | Live-tree verification | Status |
|-----|--------|------------------------|--------|
| CR-01 run-gate.sh stale assertion | 064c6a185 | `test ! -e $HARVEY/aarch64` REMOVED; NOTE comment at lines 739-743 records why (post-02-06 arch dir is legitimate); `gate-evidence.log` NOT modified (last commit `62351f81f`); `sh -n` clean. Gate idempotent per README contract again | ✓ |
| WR-01 pre-gate-build.sh obsolete guard | 2b30d900e | Guard at lines 68-71; **live run: exits 2 with the documented message** ("obsolete post-02-06 … use objtype=aarch64 mk in the namespace"); shebang `#!/usr/bin/env bash`; README row updated | ✓ |
| WR-02 nsrun.sh set -e | 872d84824 | `set -e` at lines 28 and 49 (post-arg-parse root mode); `bash -n` clean | ✓ |
| WR-03 gdbpoke9 deadline scales | f494b7727 | `--deadline "$BOOT_BUDGET"` at all three call sites (boot-9qemu.sh:209, run-gate.sh:667, subset-exec.sh:732) | ✓ |
| WR-04 echo-proof prompt/shell proofs | 795fd9de6 | `PROMPT_RE = (?m)^term% $` (anchored bare-prompt), `search_from` order-dependent proof, `echo __GSD_SHELL_^OK__` rc-caret-split probe — present in drive9.py AND both embedded forks (run-gate.sh:341/371/524/533, subset-exec.sh:369/449/595/603); `py_compile` clean | ✓ |
| WR-05 9cc/BUILD set -e | fd95a0336 | `set -e` at line 19; `sh -n` clean | ✓ |
| WR-06 9cc README provenance | d448617ed | executable.c bullet now records the `(8*4)+8` hsize literal + commonllp64 LP64 decode fix | ✓ |
| WR-07 awk fixture exclusion | d41a23220 | `find … -name '*.7' ! -path '*/cmd/awk/test/*' -delete` at donor-vm-build.sh:132; comment records both exclusion sets | ✓ |
| WR-08 vm-build.sh failure propagation | d663bab7b | Guest-failure branch prints `error: guest build failed (rc=$RC) — refusing to rsync artifacts back` and exits with guest status (lines 60-61), before the artifact pull | ✓ |
| WR-09 nix/build.sh set -e + status guard | 566677c94 | `set -e` at line 9; `bash -n` clean | ✓ |
| WR-10 mkdirs bash shebang | b4e652801 | Line 1 `#!/usr/bin/env bash`; `bash -n` clean | ✓ |

**Truth-impact assessment:** all 13 changed files are harness/host-side scripts or docs; NONE intersect the toolchain sources, libc, libmach, the committed gate binary, or the committed evidence logs. The 5 truths' supporting artifacts are byte-unchanged since the initial verification except where a fix intentionally hardened a harness script. No truth regression found.

### Deferred Items

None. The prior run's two human-verification items are closed (see `re_verification.gaps_closed`), and no new gap was identified that a later milestone phase addresses. (The phase's standing hygiene backlog lives in `deferred-items.md` — 9 open items, all recorded during execution as non-blocking with their own future-work framing; none is a must-have gap.)

### Advisory (New Scope, Unevidenced)

None. The one new observation from this re-run (the `7c -S` listing-dialect note below) carries deterministic live reproduction and is recorded as an Info anti-pattern, not an advisory.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `sys/src/cmd/7a/`, `7c/`, `7l/` | Vendored 9front toolchain + BUILD/mkfile/README | ✓ VERIFIED | Unchanged since initial verify; hostbin binaries run live (7c exit 0 on hello.c; 7a exit 0 on in-tree asm) |
| `sys/src/cmd/cc/compat.h` | Host-compat declarations | ✓ VERIFIED | Unchanged; `mysbrk(ulong)` present |
| `sys/src/libmach/7.c`, `7db.c`, `7obj.c` | arm64 machine support | ✓ VERIFIED | Unchanged; nm functional live |
| `sys/src/libc/aarch64/` | Per-arch libc | ✓ VERIFIED | 19 files + README intact |
| `sys/src/libc/9syscall/mkfile` | `case aarch64` generator | ✓ VERIFIED | Block re-verified line-by-line |
| `sys/src/libc/9sys/callmain.c` | _callmain startup glue | ✓ VERIFIED | Unchanged |
| `aarch64/mkfile`, `include/u.h`, `include/ureg.h`, `ABI.md` | objtype wiring + frozen headers + ABI contract | ✓ VERIFIED | Contents re-verified |
| `build/aarch64/hello.7.out` | Committed gate binary | ✓ VERIFIED | sha256 re-computed, matches evidence log |
| `build/aarch64/gate-evidence.log` | Execution proof | ✓ VERIFIED | Untouched by fixes (last commit `62351f81f`); 3 PASS blocks, 0 fault lines |
| `build/scripts/linux-build` | Dual-arch activation | ✓ VERIFIED | `build_aarch64` + `arch=(amd64 aarch64)` intact |
| `sys/src/libmp/aarch64/mkfile`, `sys/src/libsec/aarch64/mkfile` | riscv64-pattern arch stubs | ✓ VERIFIED | Present |
| `sys/src/libthread/aarch64.c` | libthread arch member | ✓ VERIFIED | Present |
| `build/aarch64/run-gate.sh` (+ 12 fix-touched files) | Hardened harness post-review | ✓ VERIFIED | See fix-commit table above — all 11 fixes live-verified, all scripts syntax-clean |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| BUILDTOOLCHAIN | 7a/7c/7l BUILD scripts | lines 10-12 | ✓ WIRED | Unchanged |
| BUILDTOOLCHAIN | nm.BUILD | line 15 | ✓ WIRED | nm runs live |
| 7c → 7l direct-object pipeline | hostbin tools | live smoke test | ✓ WIRED | Re-run this session: 7c exit 0 → hello.7; 7l parses + resolves pragma per namespace design |
| 7a | in-tree aarch64 asm | live assembly | ✓ WIRED | setjmp.s + generated-style SVC stub both assemble exit 0 |
| nm (host) | 7l executables | 9cc libmach ObjArm64 + R_MAGIC | ✓ WIRED | Live: hello.7.out + aarch64/bin/rc decoded |
| libmach obj.c | 7obj.c | `[ObjArm64]` row | ✓ WIRED | Unchanged |
| 9syscall generator | libc.a stubs | `case aarch64` | ✓ WIRED | Block intact; libc.a 779,312 bytes on disk |
| linux-build | aarch64 subset | `build_aarch64` | ✓ WIRED | Function + dispatch intact |
| aarch64/mkfile | mkfile.proto | `</$objtype/mkfile` | ✓ WIRED | Intact |
| run-gate.sh | gate-evidence.log | TC-03 gate | ✓ WIRED | **CR-01 RESOLVED** — stale assertion removed; the initial run's ⚠️ STALE SELF-CHECK flag is cleared. Gate is idempotent per its README contract; committed evidence untouched |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `aarch64/bin/*` (19 executables) | n/a (build products) | 7c/7l via committed namespace wiring | Yes — R_MAGIC executables; executed on donor kernel per human-approved subset-exec.log | ✓ FLOWING |
| `gate-evidence.log` | serial capture | live qemu TCG boot of donor kernel | Yes — sha256 pair re-verified against committed binary this run | ✓ FLOWING |
| `subset-exec.log` | serial capture | live qemu TCG boot + exercise | Yes — SUBSET-OK, 0 fault lines; human-approved 2026-11-01 (02-UAT.md test 1) | ✓ FLOWING |
| `donor-build.log` | build transcript | guest in-donor-tree kernel build | Yes — provenance line resolves 7c to Harvey guest hostbin | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 7c compiles the committed gate source | `$HB/7c -FVw -I… hello.c` | exit 0, hello.7 (711 bytes) | ✓ PASS |
| 7a assembles in-tree aarch64 asm | `$HB/7a setjmp.s` | exit 0, setjmp.7 (492 bytes) | ✓ PASS |
| 7a assembles a generated-style SVC stub | `$HB/7a svc-stub.s` (TEXT/MOV/SVC/RETURN) | exit 0, svc-stub.7 (282 bytes) | ✓ PASS |
| 7l parses .7 + resolves libc pragma | `$HB/7l hello.7` | correct diagnostic `cannot open /aarch64/lib/libc.a` (namespace path — by design) | ✓ PASS |
| nm decodes committed 7l executable | `$HB/nm build/aarch64/hello.7.out` | exit 0; symbols listed | ✓ PASS |
| D-09 audit capability | `$HB/nm aarch64/bin/rc` | 530 symbols, 0 nmount refs | ✓ PASS |
| Gate binary integrity | `shasum -a 256 hello.7.out` vs evidence log | `1c89e099…` matches all 3 PASS blocks | ✓ PASS |
| Git ordering (vaporware rule) | `git merge-base --is-ancestor` | 62351f81f ≺ a7945d8b5; e60e168dc ≺ 0f8ea566f | ✓ PASS |
| Donor pin integrity | `git -C .build/9front-donor rev-parse HEAD` | `350266104f1fed81c78c3d05fa44f523bec0ae4d` | ✓ PASS |
| WR-01 guard behavior | `bash pre-gate-build.sh` | exit 2 + documented message | ✓ PASS |
| Fix-commit syntax | `bash -n` × 10 scripts + `py_compile` drive9.py | all clean | ✓ PASS |
| Phase 1 regression artifacts | `ls amd64/9k10cpu{,.elf} 9k10cpuf{,.elf} 9k10f{,.elf}` | all 6 present | ✓ PASS |
| Vendoring fidelity spot-check | diff vendored `7a/lex.c` vs pinned donor | only thestring shim + documented LP64 fix | ✓ PASS |

Boot/execution behaviors were not re-run (10+-minute TCG boots; the project's proof mechanism is committed, human-approved serial-capture evidence — both logs re-content-verified this run). The WR-04-strengthened serial driver's first live TCG gate rerun remains the scheduled end-to-end confirmation of that hardening; the human triaged this as a non-blocking follow-up when closing UAT item 2 (the hardened regexes/probe were validated against simulated console streams including the 5-byte-drop attack before commit).

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| TC-01 | 02-02, 02-03 | libmach arm64 (7.c/7db.c/7obj.c/setmach); letter-7 collision resolved | ✓ SATISFIED | Sources untouched; nm operates live on 7l objects (this run) |
| TC-02 | 02-01 | 7a/7c/7l vendored + BUILD scripts; darwin-aarch64 + linux-aarch64; compat.h reconciled | ✓ SATISFIED | Tools run live on this darwin-aarch64 host; linux half via guest provenance; vendoring fidelity diff-verified |
| TC-03 | 02-03, 02-04, 02-05 | libc/aarch64 + `case aarch64`; hello-world proves toolchain before scaffolding | ✓ SATISFIED | Binary + evidence intact; ordering re-proven via git ancestry; human approval stands |
| TC-04 | 02-06 | aarch64/mkfile, pcc.c entry, mkfile.proto CPUS, config section named aarch64 | ✓ SATISFIED | All wiring re-grepped live |
| TC-05 | 02-07 | Shell-boot subset builds for aarch64 | ✓ SATISFIED | 19 executables on disk with R_MAGIC; SUBSET-OK evidence now human-approved |

All five requirement IDs are mapped to Phase 2 in REQUIREMENTS.md (marked Complete) and claimed by plan frontmatter. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (debt-marker scan, 13 fix-touched files) | — | `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER` | — | **Zero real markers.** Only hits: `mktemp …XXXXXX` templates (false positives, same as initial run) |
| (stub scan, fix-touched files) | — | placeholder/coming-soon/not-implemented language | — | None |
| `sys/src/cmd/7c/list.c` + `sys/src/cmd/7a/lex.c` | list.c:100 `"%N(R%d)(NONE)"` | ℹ️ Info (upstream-faithful quirk) | `7c -S` listing output prints SP-based address bases as raw `R31` (e.g. `16(R31)` from an arg store), which the vendored 7a rejects — register 31 is named `ZR`/`RSP` in its table (R31 lexes as `R`+31 and fails the `LR '(' expr ')'` production). **Verified upstream-faithful this run:** the pinned donor's 7c/list.c is byte-identical in these printers and its 7a table likewise lacks R31 — a ken-suite arm64 listing-dialect quirk, not a Harvey vendoring defect. Zero phase impact: the build uses the direct-object path (`7c file.c` → .7), never a `-S` round-trip; all in-tree aarch64 .s assembles cleanly. Recorded for honesty against plan 02-01's smoke-test wording ("7a assembles 7c -S output" holds only for sources without SP-based arg stores); candidates for deferred-items.md if a future phase wants `-S` reassembly |

### Human Verification Required

None. Both items from the initial verification are resolved and recorded:

1. **D-08 subset execution log eyeball** — human approved the captured log in-conversation 2026-11-01 (02-UAT.md test 1, passed). This verifier's independent content check concurs: SUBSET-OK present, 0 panic/trap/`sys: ` lines across the 169-line capture.
2. **CR-01 + WR-01..10 disposition** — human chose fix-now (02-UAT.md test 2, passed); all 11 in-scope fixes applied and live-verified by this run (see fix-commit table). The 13 Info findings remain open by explicit scope choice (fix_scope=critical_warning); the WR-04 live-rerun follow-up was triaged non-blocking by the same human decision.

### Gaps Summary

No gaps. The phase goal — Harvey compiles and links its own aarch64 Plan 9 binaries on darwin-aarch64 and linux-aarch64, proven by real binaries, not scaffolding — remains **verified achieved** after the code-review fix round. This re-verification independently confirmed: (a) all 11 fix commits landed exactly as recorded, in the exact 13 files claimed, with syntax-clean and (where executable) behavior-verified results; (b) the fixes touch harness scripts only and regressed no must-have truth — the toolchain binaries, libc, libmach, committed gate binary, and both evidence logs are byte-unchanged since the initial verification; (c) both outstanding human items are formally closed in 02-UAT.md (status complete, 2/2 passed); (d) 02-SECURITY.md records threats_open: 0 across all 12 register entries.

Interpretation records carried forward unchanged from the initial verification: SC#2 delivered nm-only (7db.c tables landed; no host acid); SC#4's "config section" realized as `arch=(amd64 aarch64)` + `build_aarch64` per Assumption A1; SC#3 letter-vs-intent note (libc/aarch64 sources committed one commit before the binary — construction necessity; all *scaffolding* landed strictly after execution evidence).

---

_Verified: 2026-11-01T19:37:35Z_
_Verifier: the agent (gsd-verifier)_
