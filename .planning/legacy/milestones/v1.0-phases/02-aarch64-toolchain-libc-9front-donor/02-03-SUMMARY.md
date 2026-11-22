---
phase: 02-aarch64-toolchain-libc-9front-donor
plan: "03"
subsystem: infra
tags: [aarch64, arm64, libc, syscall-abi, svc-stubs, 9syscall, 9front, vendoring, nm, libmach, iar, pre-gate, namespace-build, lp64]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "01"
    provides: vendored 7a/7c/7l host binaries (thestring=aarch64 shims), build/aarch64/vm-run.sh + nsrun.sh namespace runners, pin-verified 9front donor checkout
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "02"
    provides: libmach arm64 wiring (7.c/7db.c/7obj.c, R_MAGIC exectab, ObjArm64 row) in Harvey's in-tree libmach; the host-dialect warning about executable.c
provides:
  - sys/src/libc/9syscall/mkfile `case aarch64` — the D-15 user-side ABI freeze generator (R0 spill to 0(FP), syscall number into R0, SVC, RETURN, local _seek rename; 53 stubs generated from Harvey's full sys.h incl. NMOUNT=54)
  - sys/src/libc/aarch64 — the donor's per-arch libc (16 asm + notejmp.c + _seek.c + mkfile) byte-identical to pin 35026610 modulo the mkfile objtype line and the lock.c drop (D-14 reconcile) + provenance README
  - sys/src/libc/9sys/callmain.c (+ callmain.$O in 9sys/mkfile) — _callmain/_privates/_nprivates for arm64 main9.s, additive and inert on amd64
  - build/aarch64/hello.7.out — 7c-compiled, 7l-linked aarch64 Plan 9 binary (R_MAGIC 0x8C47), built in the lima namespace by the COMMITTED libc mkfiles with guest-scratch aarch64/ scaffolding injected only into the disposable guest copy — committed gate evidence (ROADMAP SC#3 link half; execution is plan 02-05)
  - build/aarch64/pre-gate-build.sh + hello.c — the repeatable pre-gate driver and gate program
  - sys/src/cmd/unix/9cc libmach arm64 (7-format) support — 7obj.c/7.c/ureg7.h/cmd/7c/7.out.h + [ObjArm64] row + R_MAGIC exectab + FARM64 boot case — so iar writes __.SYMDEF for .7 members (libc.a linkable by 7l) and the host nm can parse 7-format objects and executables
  - sys/src/cmd/nm.BUILD + BUILDTOOLCHAIN wiring — host nm proven functional on hello.7.out (main, _main, print, exits listed)
affects: [02-04 donor kernel build (uses the same 7a/7c/7l + guest iar), 02-05 execution harness (hello.7.out is the gate binary), 02-06 objtype wiring (commits the same aarch64/mkfile + include headers used here as guest scratch; sha256-equality assertion), 02-07 shell subset + D-09 NMOUNT nm gate, Phase 4 r9 svc dispatch (consumes the frozen ABI)]

actuals:
  tokens: 20967
  tasks: 3
  commits: 3
  plan_head_before: 8605b715f06cc98773916d78713413539aad10ef
  plan_head_after: 78ee29f616ba0e6f6c9a798542e903e6d1dd21d1

tech-stack:
  added: [9front libc/arm64 @ 350266104f1fed81c78c3d05fa44f523bec0ae4d (vendored, MIT-compatible), 9front arm64 libmach subset in the vendored 9-cc tree (same pin), host nm binary]
  patterns: [guest-scratch pre-gate build (inject scaffolding into the disposable guest copy only; committed tree gains the binary, not the scaffolding), vendored-tree format extension (teach the 9cc host tools a new object format with donor-verbatim parser + wiring rows), LP64 width-explicit a.out header decode (u32 view) in host-built libmach]

key-files:
  created:
    - sys/src/libc/9sys/callmain.c
    - sys/src/libc/aarch64/ (20 files: 16 asm + notejmp.c + _seek.c + mkfile + README)
    - sys/src/cmd/unix/9cc/cmd/7c/7.out.h
    - sys/src/cmd/unix/9cc/libmach/7obj.c
    - sys/src/cmd/unix/9cc/libmach/7.c
    - sys/src/cmd/unix/9cc/libmach/ureg7.h
    - sys/src/cmd/nm.BUILD
    - build/aarch64/hello.c
    - build/aarch64/pre-gate-build.sh
    - build/aarch64/hello.7.out
  modified:
    - sys/src/libc/9syscall/mkfile (case aarch64)
    - sys/src/libc/9sys/mkfile (callmain.$O)
    - sys/src/cmd/unix/9cc/libmach/obj.c (_is7/_read7 + ObjArm64 row)
    - sys/src/cmd/unix/9cc/libmach/executable.c (marm64 extern + R_MAGIC exectab + FARM64 boot case + LP64 commonllp64 fix)
    - sys/src/cmd/unix/9cc/BUILD (-Wno-error=format-security)
    - sys/src/cmd/unix/9cc/README (provenance)
    - sys/src/BUILDTOOLCHAIN (nm.BUILD line)
    - .gitignore (nm binary + pre-gate log)
  deleted:
    - sys/src/libc/aarch64/lock.c (D-14 reconcile — see Deviations)

key-decisions:
  - "libc/aarch64 drops the donor's lock.c: 9front modernized Lock to a single-word tas spinlock ({ int val; }); Harvey keeps the classic { long key; long sem; } with the ainc/adec + semacquire protocol in portable sys/src/libc/port/lock.c — the same file amd64 uses (amd64's arch dir likewise carries no lock.c). ainc/adec come from the vendored atom.s; semacquire/semrelease stubs from the new generator"
  - "Host nm and iar's 7-format support landed in the vendored 9-cc tree: its unix-ported libmach is the only host-compilable libmach lineage (Harvey's in-tree executable.c embeds Ehdr/E64hdr typedef-name members whose promoted field names collide — Plan 9-compiler-only construct, pre-existing per 02-02's A/B probe). The parser body, marm64, exectab entry are the same pinned-donor sources Harvey's in-tree libmach carries"
  - "9cc executable.c commonllp64 decodes the a.out Exec header through an explicit u32 view and the R_MAGIC exectab hsize is the on-disk (8*4)+8 literal: on LP64 hosts ulong is 8 bytes (Plan 9: 4), so hswal's stride and sizeof(Exec) both lied — nm exited 0 with empty output until fixed"

patterns-established:
  - "Guest-scratch pre-gate build: pin-verify donor, rsync to lima guest, inject aarch64/mkfile + include/{u.h,ureg.h} + lib/ into the guest copy ONLY, run committed mkfiles under objtype=aarch64 in the namespace, pull the linked binary back, gate on `test ! -e aarch64` + git-status cleanliness (T-02-05 mitigation; the riscv64 vaporware rule made executable)"
  - "The 9cc vendored tree is the host-tool libmach: extend it (donor parser + table rows) rather than host-compiling Harvey's in-tree libmach; in-tree libmach remains authoritative for in-namespace/target builds"

requirements-completed: [TC-03, TC-01]

coverage:
  - id: D1
    description: "9syscall mkfile gains the donor-verbatim `case aarch64` generator (R0 spill to 0(FP), number into R0, SVC, RETURN, local _seek rename, no stub-side buffer store) — the D-15 user-side ABI freeze"
    requirement: TC-03
    verification:
      - kind: other
        ref: "sed -n '/case aarch64/,/RETURN/p' sys/src/libc/9syscall/mkfile — SVC + 0(FP) + _seek present, SYSCALL count 0 (plan gate chain 1)"
        status: pass
      - kind: integration
        ref: "pre-gate guest build log: 53 generated stub members (incl. _exits.7, _seek.7, nsec.7, nmount.7) 7a-assembled and ar-archived into /aarch64/lib/libc.a"
        status: pass
    human_judgment: false
  - id: D2
    description: "sys/src/libc/aarch64 vendored from the pinned donor: 16 asm + notejmp.c + _seek.c byte-identical; mkfile differs only in objtype=aarch64 (and the lock.c CFILES drop); README records provenance"
    requirement: TC-03
    verification:
      - kind: other
        ref: "diff -r --exclude=mkfile --exclude=README .build/9front-donor/sys/src/libc/arm64 sys/src/libc/aarch64 → exactly one line (Only in donor: lock.c — the documented D-14 reconcile); head -1 mkfile == objtype=aarch64"
        status: pass
    human_judgment: false
  - id: D3
    description: "callmain.c imported verbatim + callmain.$O in 9sys OFILES (additive, inert on amd64); compiled into /aarch64/lib/libc.a by the guest build (7c -FTVw callmain.c → a - callmain.7)"
    requirement: TC-03
    verification:
      - kind: other
        ref: "diff -q vs donor callmain.c (empty); grep callmain.$O sys/src/libc/9sys/mkfile; guest log lines 49/97/102"
        status: pass
    human_judgment: false
  - id: D4
    description: "Pre-gate namespace build compiles libc for aarch64 with the committed mkfiles (guest-scratch aarch64/ injected only into the disposable guest copy) and links hello-world: build/aarch64/hello.7.out committed as gate evidence; committed tree holds zero arch scaffolding"
    requirement: TC-03
    verification:
      - kind: integration
        ref: "sh build/aarch64/pre-gate-build.sh (rc 0; log captured at build/aarch64/pre-gate-build.log, gitignored)"
        status: pass
      - kind: other
        ref: "od -An -tx1 -N4 hello.7.out == 00008c47 (R_MAGIC); test ! -e aarch64; git-status grep for aarch64/ == 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "Host nm (nm.BUILD + BUILDTOOLCHAIN wiring) built against the now-7-capable 9cc libmach reads hello.7.out: exit 0, lists main (T 10028), _main (L 10044, the entry), print/exits SVC stubs — TC-01's functional proof and the D-09 tool for plan 02-07"
    requirement: TC-01
    verification:
      - kind: other
        ref: "sh -n nm.BUILD; (cd sys/src/cmd && sh nm.BUILD); hostbin nm build/aarch64/hello.7.out | grep -w main/_main"
        status: pass
    human_judgment: false

duration: 19 min
completed: 2026-10-31
status: complete
---

# Phase 02 Plan 03: User-Side ABI Freeze + libc/aarch64 + Pre-Gate Link + Host nm Summary

**Froze the user-side aarch64 syscall ABI (donor-verbatim `case aarch64` SVC-stub generator over Harvey's full sys.h 0–54), vendored 9front's libc/arm64 as sys/src/libc/aarch64, built `/aarch64/lib/libc.a` and linked `hello.7.out` (R_MAGIC 0x8C47) in the lima namespace using the committed mkfiles with guest-only scratch scaffolding — and landed a host nm (9cc libmach, now 7-format-capable) that decodes the binary's symbol table (main/_main/print/exits), closing TC-01 functionally and TC-03's build half.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-10-31T17:04:23Z
- **Completed:** 2026-10-31T17:24:09Z
- **Tasks:** 3
- **Files modified:** 28 (18 created, 9 modified, 1 deleted — incl. the committed gate binary)

## Accomplishments

- **D-15 ABI freeze landed:** `case aarch64` in `sys/src/libc/9syscall/mkfile` is the donor's arm64 block verbatim (only the case label renamed): per syscall it emits `TEXT $j(SB), 1, $0`; `MOV R0, 0(FP)` (arg0 materialized into the first slot the kernel reads — kernel-visible args at user SP+8, SP+0 reserved LR slot, research-corrected wording); `MOV $n, R0`; `SVC`; `RETURN`; local `j=$i`/`_seek` rename. Harvey's line-9 `_exits` rule is untouched; NSEC=53 gets a plain wire-correct stub; NMOUNT=54 is generated (in sys.h) and must stay uncalled on the donor kernel (D-09, plan 02-07). No buffer-store epilogue anywhere (Pitfall G).
- **libc/aarch64 vendored:** the donor's 20-file set minus `lock.c` (see Deviations) — 16 asm files (argv0/atom/atomic/atomic64/cycles/fabs/floor/getcallerpc/getfcr/main9/main9p/memmove/memset/setjmp/sqrt/tas) + notejmp.c + _seek.c + mkfile (line 1 `objtype=aarch64`, the only content edit) + README provenance. Diff vs pin 35026610 is exactly one line (`Only in donor: lock.c`).
- **`_callmain` startup pair imported:** 9sys/callmain.c verbatim from the donor + `callmain.$O` in 9sys/mkfile OFILES (alphabetical position). Guest log proves it compiled and archived (`7c -FTVw callmain.c` → `a - callmain.7`). Inert on amd64 (archive member never pulled).
- **Pre-gate gate passed (ROADMAP SC#3 link half):** `build/aarch64/pre-gate-build.sh` pin-verifies the donor, rsyncs to the lima guest, injects `aarch64/mkfile` (the known 5-line file) + `aarch64/include/{u.h,ureg.h}` from the donor + `aarch64/lib/` into the **guest copy only**, runs `cd /sys/src/libc && objtype=aarch64 mk nuke && mk install` in the namespace (committed libc mkfiles; 53 SVC stubs generated and 7a-assembled; fmt/port/aarch64 all green — port's `lock.c` supplies Harvey-semantics locks), then `7c -FVw hello.c && 7l -o /aarch64/hello.7.out hello.7` (7l resolves `libc.a` via /aarch64/lib, the 02-01 thestring shim). The binary came back with correct `00 00 8c 47` magic; `test ! -e aarch64` + git-status gate prove zero committed scaffolding.
- **Host nm works on 7l executables:** `sys/src/cmd/nm.BUILD` (host cc against the vendored 9cc tree — plan9port-free; shim maps nm.c's `<u.h>/<libc.h>` onto 9cc's lib9.h umbrella), wired into BUILDTOOLCHAIN. `nm build/aarch64/hello.7.out` exits 0 and lists `main` (T 0x10028), `_main` (L 0x10044 — the entry point), `print`/`exits` (the generated SVC stubs), `_callmain`, `_barrier`, `_exits`.

## Task Commits

1. **Task 1: Freeze the user-side ABI — case aarch64 generator + callmain.c + libc/aarch64 vendored** — `25ddb3d6c` (feat)
2. **Task 2: Pre-gate namespace build — libc/aarch64 compiled, hello-world linked via committed mkfiles (guest-scratch scaffolding only)** — `e60e168dc` (feat)
3. **Task 3: Host nm BUILD + BUILDTOOLCHAIN wiring + nm reads hello.7.out** — `78ee29f61` (feat)

**Plan metadata:** see final docs commit below.

## Files Created/Modified

- `sys/src/libc/9syscall/mkfile` — the `case aarch64` freeze block (between amd64 and sparc cases)
- `sys/src/libc/9sys/callmain.c` — donor-verbatim `_callmain` (defines `_privates`/`_nprivates` consumed by privalloc.c:8-9)
- `sys/src/libc/9sys/mkfile` — `callmain.$O` in OFILES
- `sys/src/libc/aarch64/` — 16 asm + notejmp.c + _seek.c + mkfile (objtype=aarch64) + README; **not** lock.c (Deviation 1)
- `sys/src/cmd/unix/9cc/cmd/7c/7.out.h`, `libmach/7obj.c`, `libmach/7.c`, `libmach/ureg7.h` — 9front arm64 support for the host tools (parser body/Mach/reglist/trap-frame donor-verbatim; include blocks adapted to the tree's lib9.h idiom)
- `sys/src/cmd/unix/9cc/libmach/obj.c` — `_is7`/`_read7` decls + `[ObjArm64] "arm64 .7"` parser row (the enum already existed; the row was missing upstream)
- `sys/src/cmd/unix/9cc/libmach/executable.c` — `extern Mach marm64` + donor-verbatim R_MAGIC exectab entry + FARM64 commonboot case + the LP64 commonllp64 fix (Deviation 4)
- `sys/src/cmd/unix/9cc/BUILD` — `-Wno-error=format-security` (nix devshell cc-wrapper hardening vs the tree's `-Wno-format`)
- `sys/src/cmd/unix/9cc/README` — provenance for the arm64 import
- `build/aarch64/hello.c` — the gate program (`print("hello aarch64 harvey\n")` + `exits(nil)`)
- `build/aarch64/pre-gate-build.sh` — the pre-gate driver (documents its guest-scratch nature in the header)
- `build/aarch64/hello.7.out` — committed gate evidence binary
- `sys/src/cmd/nm.BUILD`, `sys/src/BUILDTOOLCHAIN`, `.gitignore` — host nm build + wiring + hygiene

## Decisions Made

- **lock.c dropped from libc/aarch64 (D-14 reconcile clause).** 9front's `lock.c` uses `lk->val` — 9front modernized `Lock` to a single-word tas spinlock (`{ int val; }`), while Harvey's `Lock` stays `{ long key; long sem; }` with the ainc/adec + semacquire/semrelease protocol. Harvey's portable `sys/src/libc/port/lock.c` (in every arch's libc build — amd64's arch dir likewise has no lock.c) supplies `lock`/`unlock`/`canlock` for aarch64 with Harvey semantics; `ainc`/`adec` come from the vendored `atom.s`, and the `semacquire`/`semrelease` stubs from the new generator (both in the wire-identical 0–52 range, so this runs on the donor kernel too).
- **Host 7-format support went into the vendored 9-cc tree, not Harvey's in-tree libmach.** The plan's primary nm route (host cc + Harvey's `sys/src/libmach`) is defeated by a pre-existing source-dialect wall: `executable.c` embeds `Ehdr;`/`E64hdr;` typedef-name members whose promoted field names collide — legal only to the Plan 9 compilers (host clang rejects at declaration, with or without `-fms-extensions`/`-std=gnu89`; A/B-proven pre-existing in 02-02 and re-confirmed this session). nm needs `crackhdr` for executables, so there is no libmach subset route around executable.c. The 9cc tree's libmach is the same lineage ported for unix hosts; the arm64 additions are the same pinned-donor sources (7obj.c parser body and the exectab entry are byte-identical to Harvey's in-tree copies). Harvey's in-tree libmach stays authoritative for in-namespace/target builds.
- **LP64 fix in 9cc's commonllp64.** On Plan 9 `long` is 4 bytes; on darwin/linux LP64 hosts `ulong` is 8 — so `hswal(&hp->e, sizeof(Exec)/sizeof(long), beswal)` swapped 8-byte units with a 4-byte swapper (scrambling field pairing: `symsz` read 0) and the exectab's `sizeof(Exec)+8` hsize computed 72 instead of the on-disk 40. commonllp64 now decodes through an explicit `u32int` view and the R_MAGIC entry uses the literal `(8*4)+8`. Upstream never hit this: iar (the only 9cc consumer exercised to date) parses objects, not executables.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking / D-14 reconcile] Donor `lock.c` cannot compile against Harvey's libc.h**
- **Found during:** Task 2 (first guest libc build: `lock.c:12 not a member of struct/union: val`)
- **Issue:** the plan's "20-file verbatim set" must-have assumed the donor's lock.c compiled; 9front's Lock (`{int val}` tas spinlock) and Harvey's Lock (`{key, sem}` + semacquire) are incompatible ABIs, and every Harvey Lock consumer (qlock, privalloc, malloc) compiles against Harvey's struct.
- **Fix:** removed `sys/src/libc/aarch64/lock.c` + its CFILES entry; portable `port/lock.c` supplies lock/unlock/canlock for aarch64 exactly as for amd64 (verified in the guest build: `a - lock.7` from the port step). README updated.
- **Files modified:** sys/src/libc/aarch64/ (lock.c deleted, mkfile, README)
- **Verification:** guest libc build green; Task 1's diff gate now intentionally reports exactly this one line
- **Committed in:** e60e168dc (Task 2 commit)

**2. [Rule 3 - Blocking] iar wrote no `__.SYMDEF` for .7 members — 7l could not link libc.a**
- **Found during:** Task 2 (guest link: `7l ... /aarch64/lib/libc.a: first entry not symbol header: abort.7`)
- **Issue:** research's assumption "7-format archives are fine (ar doesn't parse objects)" is wrong — iar's `scanobj`/`objtraverse` builds the SYMDEF from *parsed* objects, and the vendored 9-cc libmach had no 7-format support (enums present, parser row missing), so `ar vu` of .7 members silently produced a symdef-less archive.
- **Fix:** vendored 9front arm64 support into the 9cc tree: `cmd/7c/7.out.h` + `libmach/ureg7.h` donor-verbatim; `libmach/7obj.c` + `libmach/7.c` donor bodies byte-identical with include blocks adapted to the tree's `<lib9.h>`/"mach.h"/local-ureg idiom; `obj.c` gained `_is7`/`_read7` decls and the `[ObjArm64]` row; `executable.c` gained the marm64 extern, the donor-verbatim R_MAGIC exectab entry and FARM64 boot case. Provenance recorded in 9cc/README.
- **Files modified:** sys/src/cmd/unix/9cc/ (4 created, 3 modified)
- **Verification:** `ar vu test.a memset.7` now writes `__.SYMDEF`; guest libc.a links under 7l
- **Committed in:** e60e168dc (Task 2 commit)

**3. [Rule 1 - Bug] 9cc/BUILD fails under the nix devshell cc wrapper**
- **Found during:** Task 2 (guest iar rebuild): `cc1: error: '-Wformat-security' ignored without '-Wformat' [-Werror=format-security]`
- **Issue:** the devshell's cc wrapper injects `-Werror=format-security`, which errors against the tree's `-Wno-format`; the flake-app path (`nix run .#toolchain`) doesn't inject it, which is why 02-01 never hit it.
- **Fix:** added `-Wno-error=format-security` to 9cc/BUILD's CFLAGS (documented in README).
- **Files modified:** sys/src/cmd/unix/9cc/BUILD, README
- **Verification:** 9cc build green in the guest devshell
- **Committed in:** e60e168dc (Task 2 commit)

**4. [Rule 1 - Bug] LP64 header-decode scramble in 9cc libmach commonllp64**
- **Found during:** Task 3 (nm exited 0 with EMPTY output on hello.7.out)
- **Issue:** `ulong` is 8 bytes on LP64 hosts (4 on Plan 9): `hswal`'s stride and `sizeof(Exec)` both doubled, scrambling the a.out Exec header fields (probe: symsz=0, txtsz=data size, hdrsz=72) — syminit returned an empty table silently. Pre-existing upstream; unexercised because iar never cracks executables.
- **Fix:** commonllp64 decodes the eight on-disk BE u32 words through an explicit `u32int` view; the R_MAGIC exectab hsize is the literal on-disk `(8*4)+8`. Scope note: the same stride idiom in `adotout`/crackhdr's whole-union swap affects other arches' host-side parsing and is NOT touched (out of scope; deferred-items #2).
- **Files modified:** sys/src/cmd/unix/9cc/libmach/executable.c
- **Verification:** nm lists main/_main/print/exits with correct values (entry == hdrmagic == 0x10044 == _main)
- **Committed in:** 78ee29f61 (Task 3 commit)

**5. [Rule 3 - Blocking] nm route substitution (plan's flagged assumption, stronger trigger)**
- **Found during:** Task 3 planning
- **Issue:** the plan's primary route compiles nm.c + every sys/src/libmach/*.c with host cc — impossible: executable.c is Plan 9-compiler-only (embedded-typedef promotion collisions; pre-existing, 02-02 A/B probe). The plan anticipated "header drift" as the trigger; the actual blocker is the stronger source-dialect wall. The plan's own fallback ("9cc tree route") was taken, with the adjustment that the 9cc tree's own (now 7-capable) libmach substitutes for "Harvey's libmach sources".
- **Fix:** nm.BUILD documents the route + rationale in its header; compiles nm.c unmodified via a per-build u.h/libc.h→lib9.h shim; bootstraps the 9cc archives when absent (BUILDTOOLCHAIN ordering).
- **Files modified:** sys/src/cmd/nm.BUILD (created), sys/src/BUILDTOOLCHAIN
- **Verification:** plan's Task 3 verify chain passes end-to-end
- **Committed in:** 78ee29f61 (Task 3 commit)

**6. [Rule 3 - Blocking] Minor plan-text mismatches in Task 2's command shape**
- **Issue:** the plan's step (5) says `cd /build/aarch64 && 7c ...` but nsrun.sh binds no /build; the compile runs from `$HARVEY/build/aarch64` (the real guest path — /sys and /aarch64 are the Plan 9 absolutes). Also, the toolchain-ensure sentinel would skip the stale guest `ar`, so pre-gate-build.sh rebuilds iar in the guest on every run (cheap, idempotent) after rsync.
- **Fix:** documented in the script header; no nsrun.sh change (shared machinery for 02-04..02-07).
- **Committed in:** e60e168dc (Task 2 commit)

---

**Total deviations:** 6 auto-fixed (3 blocking, 2 bugs, 1 reconcile/route)
**Impact on plan:** Every deviation preserves the plan's gates: the ABI freeze is donor-verbatim; the import set diverges from the donor only where Harvey's libc.h ABI forces it (lock.c); the host tools prove the same pinned-donor parsing code. No scope creep; the riscv64 vaporware gate held (zero committed arch scaffolding). The one verify-gate line that changed shape is Task 1's `diff -r` against the donor — it now intentionally reports exactly `Only in donor: lock.c`.

## Issues Encountered

- The research claim "ar doesn't parse objects" (02-RESEARCH Supporting table) was wrong and surfaced as Deviation 2 — the phase's iar dependency was under-analyzed. Resolved by the 9cc vendoring; no remaining risk.
- The guest toolchain ensure re-ran the full `nix run .#setup && nix run .#toolchain` despite the 7c/ar sentinels existing — idempotent but slow; not investigated (vm-run.sh shares the behavior). Logged as deferred-items #4.
- `sys/src/mkfile.proto` `OS=568ijqv` lacks `7`, so `9syscall`'s nuke/clean won't sweep `.7` files (install cleans up after itself; harmless here). PATTERNS already flagged this for 02-06's CPUS evaluation — deferred-items #3.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 02-04 (donor kernel build):** the guest's rebuilt iar (7-format SYMDEF) and the proven 7a/7c/7l are in place; uses guest dir `harvey-build-donor` per the declared partitioning (this plan mutated only `harvey-build`, and the next rsync --delete wipes the scratch).
- **Plan 02-05 (execution harness):** `build/aarch64/hello.7.out` is the gate binary (print + exits via the frozen SVC stubs); nm decode confirms its symbol table is well-formed for the donor kernel.
- **Plan 02-06 (objtype wiring):** commits `aarch64/mkfile` + `aarch64/include/{u.h,ureg.h}` — must assert sha256 equality with the scratch copies used here (donor pin 35026610, `arm64/mkfile` content per 02-RESEARCH Code Examples).
- **Plan 02-07 (shell subset + D-09):** the host nm (`$HARVEY_LINUX_BIN/nm`) exists and is proven on 7l executables — the D-09 NMOUNT gate is unblocked.
- **TC-01 note:** Harvey's *in-tree* libmach arm64 files (02-02) are wired but their compile proofs ride the namespace/target builds (02-06 headers land `aarch64/include/ureg.h` for 7.c); the host-side functional proof for TC-01's chain was delivered via the 9cc tree (same donor sources). See Deviation 5.
- No blockers.

## Self-Check: PASSED

- All 14 key files verified present on disk (9syscall/mkfile, callmain.c, 9sys/mkfile, libc/aarch64 set, 9cc arm64 files, nm.BUILD, hello.c, pre-gate-build.sh, hello.7.out)
- All 3 task commits found in git history: 25ddb3d6c, e60e168dc, 78ee29f61
- All plan `<verification>` gates re-run green at plan end (generator region-clean, donor diff == lock.c-only, R_MAGIC, no aarch64/ in tree or git status, nm functional)
