---
phase: 02-aarch64-toolchain-libc-9front-donor
plan: "02"
subsystem: infra
tags: [aarch64, arm64, libmach, mach.h, nm, acid, 9front, vendoring, plan9, alpha-excision, object-format]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "01"
    provides: pin-verified 9front donor checkout at .build/9front-donor (SHA 350266104f1fed81c78c3d05fa44f523bec0ae4d); vendored sys/src/cmd/7c/7.out.h (7obj.c's mk prerequisite)
provides:
  - letter 7 freed from the retired DEC Alpha port — alpha.c/alphadb.c/alphaobj.c/alpha/ deleted, five mach.h enums (MALPHA/FALPHA/FALPHAB/AALPHA/ObjAlpha) excised, zero residue
  - sys/src/libmach/7.c + 7db.c + 7obj.c — vendored 9front arm64 machine support (Mach/reglist, disassembler, 7-format object parser) with the single D-11 include rewrite to /aarch64/include/ureg.h
  - setmach.c "arm64" entry + marm64/arm64mach externs; executable.c R_MAGIC exectab entry + case FARM64 boot-image; obj.c [ObjArm64] parser row; mkfile 7/7db/7obj + absolute 7obj.$O dep
  - sys/src/libmach/README — provenance (pin SHA, import set, local-modification list)
affects: [02-03 host nm build + libc, 02-07 D-09 NMOUNT gate (nm), phase-exit dual-arch build]

actuals:
  tokens: 16447
  tasks: 2
  commits: 2
  plan_head_before: 7ca35b47b8c7f32879b919466c43cd42a3af1609
  plan_head_after: ef0aa4678826840409a70f374259c753a7061dc1

tech-stack:
  added: [9front libmach arm64 @ 350266104f1fed81c78c3d05fa44f523bec0ae4d (7.c/7db.c/7obj.c vendored source, MIT-compatible)]
  patterns: [collision-only excision (D-05: remove exactly what blocks, leave other retired arches for hygiene phase), donor-verbatim table wiring (setmach/exectab/commonboot/objtable entries diff-proven against the pin), host-probe A/B dialect baseline (pre-existing Plan 9 anonymous-typedef embedding + USED idiom proven identical pre/post edit)]

key-files:
  created:
    - sys/src/libmach/7.c
    - sys/src/libmach/7db.c
    - sys/src/libmach/7obj.c
    - sys/src/libmach/README
  modified:
    - sys/include/mach.h
    - sys/src/libmach/mkfile
    - sys/src/libmach/setmach.c
    - sys/src/libmach/executable.c
    - sys/src/libmach/obj.c
  deleted:
    - sys/src/libmach/alpha.c
    - sys/src/libmach/alphadb.c
    - sys/src/libmach/alphaobj.c
    - sys/src/libmach/alpha/ (7.out.h, ureg.h)

key-decisions:
  - "executable.c common() comment's alpha mention removed with the code sites — the plan's zero-residue grep gate (grep -ri alpha over *.c/*.h/mkfile) and D-04's no-tombstone rule cover comments too; the line list in the plan was pre-excision and the gate is authoritative"
  - "Plan's 9c host probe substituted with direct host-clang probe (Harvey headers only: amd64/include + sys/include) — 9c hard-codes -I$PLAN9/include before user -I flags, so plan9port's u.h/mach.h shadow Harvey's and its String.h hits Harvey's string.h via case-insensitive APFS; the substitution is the plan's own flagged Assumption-A3 fallback path (record drift, do not shim Harvey headers)"
  - "mach.h ObjArm64 comment corrected .4? → .7 — plan-allowed cosmetic; stale in both trees per research"

patterns-established:
  - "Zero-residue excision gate: the verify grep covers comments as well as code — tombstone cleanup is part of the excision, not optional"
  - "Host-probe dialect baseline: pre-existing host-clang failures (anonymous typedef embedding, USED) are A/B-proven against git HEAD before attributing any error to an edit"

requirements-completed: [TC-01]

coverage:
  - id: D1
    description: "Retired DEC Alpha port fully excised from libmach + mach.h (D-04/D-05): five files git-rm'd, five enum lines dropped, setmach/exectab/bootcase/objtable/mkfile wiring removed, stale pre-excision .6/.8 objects deleted; grep finds zero residue; vendored 9cc tree untouched"
    requirement: TC-01
    verification:
      - kind: other
        ref: "test ! -e alpha.c && test ! -d alpha && grep -ri alpha sys/src/libmach sys/include/mach.h --include='*.c' --include='*.h' --include='mkfile' | wc=0; git ls-files sys/src/libmach | grep -ci alpha = 0 (plan's automated gate 1)"
        status: pass
      - kind: other
        ref: "host-clang probe: setmach.c + obj.c compile 0 errors; executable.c 20 pre-existing dialect errors A/B-identical to git HEAD (anonymous typedef embedding + USED), zero referencing removed identifiers"
        status: pass
    human_judgment: false
  - id: D2
    description: "libmach learns the 9front arm64 machine: vendored 7.c/7db.c/7obj.c byte-identical to the pin except 7.c's single /aarch64/include/ureg.h rewrite; setmach arm64 entry + both externs; R_MAGIC exectab entry + case FARM64; [ObjArm64] parser row; mkfile FILES + absolute 7obj.$O dep; README provenance with pin SHA"
    requirement: TC-01
    verification:
      - kind: other
        ref: "plan's automated gate 1: file existence, no /arm64/ residue in vendored trio, four wiring greps (\"arm64\", R_MAGIC, ObjArm64, 7obj.$O dep) all present"
        status: pass
      - kind: other
        ref: "diff vs .build/9front-donor: 7db.c and 7obj.c byte-identical; 7.c differs by exactly the one rewritten include line; setmach/exectab/commonboot/obj entries diff-verbatim against donor"
        status: pass
      - kind: other
        ref: "host-clang probe: setmach.c + obj.c + 7db.c compile (7db.c's 2 USED-only errors are the same pre-existing host-dialect gap as in-tree 5db.c); executable.c errors A/B-identical to HEAD"
        status: pass
    human_judgment: false
  - id: D3
    description: "7.c and 7obj.c full host-compile proof deferred by plan design: 7.c includes /aarch64/include/ureg.h (lands in 02-06 post-gate) and 7obj.c's nm-build proof is 02-03 Task 3's verify"
    requirement: TC-01
    verification: []
    human_judgment: true
    rationale: "The plan deliberately defers these two compile proofs to later plans (02-03 nm BUILD compiles 7obj.c against 02-01's 7c/7.out.h; 02-06 lands the aarch64/include headers 7.c names). No test asserts them in this plan — verify-work must confirm the downstream proofs happened."

duration: 5 min
completed: 2026-10-31
status: complete
---

# Phase 02 Plan 02: libmach arm64 (9front donor) + Alpha Excision Summary

**Excised the retired DEC Alpha port from libmach/mach.h (five files, five enums, all wiring — zero residue) and vendored 9front's arm64 machine support (7.c/7db.c/7obj.c byte-identical to pin 35026610 except 7.c's single `/aarch64/include/ureg.h` rewrite) with donor-verbatim setmach/exectab/commonboot/objtable/mkfile wiring, so nm and acid can operate on 7l aarch64 objects.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-10-31T16:56:07Z
- **Completed:** 2026-10-31T17:01:37Z
- **Tasks:** 2
- **Files modified:** 13 (4 created, 4 modified wiring + 1 header, 5 deleted incl. alpha/ dir)

## Accomplishments

- **Collision resolved (D-04/D-05):** letter 7 is free for arm64 — `alpha.c`, `alphadb.c`, `alphaobj.c`, `alpha/7.out.h`, `alpha/ureg.h` git-rm'd; `MALPHA`/`FALPHA`/`FALPHAB`/`AALPHA`/`ObjAlpha` enums excised from mach.h (exactly 5 deletions; `MARM64`/`FARM64`/`FARM64B`/`AARM64`/`ObjArm64` retained); setmach entry, both exectab entries (`L_MAGIC` + `0x0700e0c3`), the `case FALPHA` boot case, the `[ObjAlpha]` parser row, and all four mkfile wiring lines removed; stale on-disk `alpha{,db,obj}.{6,8}` + pre-excision `setmach/executable/obj.{6,8}` objects deleted (Pitfall C)
- **arm64 vendored (TC-01):** `7.c` (152 lines), `7db.c` (800), `7obj.c` (153) copied from the pinned 9front donor — diff-proven byte-identical except 7.c's one planned rewrite (`/arm64/include/ureg.h` → `/aarch64/include/ureg.h`, the only `/arm64/` path in the import set)
- **All six wiring sites landed, donor-verbatim:** setmach.c `marm64`/`arm64mach` externs + `"arm64"` entry (9front setmach.c:109-114); executable.c `extern Mach marm64` + `R_MAGIC` exectab entry (9front executable.c:239-247) + `case FARM64:` in commonboot (372-377); obj.c `[ObjArm64] "arm64 .7", _is7, _read7,` row (donor obj.c:59; decls already present at lines 27/39); mkfile `7`/`7db`/`7obj` in FILES + `7obj.$O: /sys/src/cmd/7c/7.out.h` in absolute 6obj-form (planner correction of PATTERNS' relative suggestion — no libmach-local 7c/ copy exists)
- **Provenance:** `sys/src/libmach/README` records the donor URL, pin SHA (350266104f1fed81c78c3d05fa44f523bec0ae4d, 2026-10-31), the three vendored files, the single path rewrite, and the D-01/D-11 note that libmach-internal names keep `arm64` (D-10, 9cc-README shape)
- **Plan-allowed cosmetic:** mach.h `ObjArm64` comment corrected `/* .4? */` → `/* .7 */` (stale in both trees per research)
- **Vendored 9cc tree untouched** (D-04 boundary held — verified in the residue grep's scope and by git status)

## Task Commits

1. **Task 1: Excise the retired DEC RISC port from libmach (D-04/D-05)** — `d8793d15f` (refactor)
2. **Task 2: Vendor libmach arm64 (7.c/7db.c/7obj.c) + wire setmach/exectab/objtable/mkfile** — `ef0aa4678` (feat)

**Plan metadata:** see final docs commit below.

## Files Created/Modified/Deleted

- `sys/src/libmach/7.c` — vendored arm64 Mach/reglist (marm64); single rewritten include per D-11
- `sys/src/libmach/7db.c` — vendored arm64 disassembler (arm64mach) for acid/nm
- `sys/src/libmach/7obj.c` — vendored `_is7`/`_read7` 7-format object parser
- `sys/src/libmach/README` — provenance + local-modification list (D-10)
- `sys/include/mach.h` — five retired enum lines removed; ObjArm64 comment corrected
- `sys/src/libmach/setmach.c` — alpha externs/entry out; arm64 externs/entry in
- `sys/src/libmach/executable.c` — alpha extern/exectab-pair/boot-case out (+ stale common() comment line); marm64 extern + R_MAGIC entry + case FARM64 in
- `sys/src/libmach/obj.c` — `[ObjAlpha]` row out; `[ObjArm64]` row in
- `sys/src/libmach/mkfile` — alpha FILES/dep-rule out; 7/7db/7obj FILES + absolute 7obj.$O dep in
- Deleted: `sys/src/libmach/alpha.c`, `alphadb.c`, `alphaobj.c`, `alpha/7.out.h`, `alpha/ureg.h`

## Decisions Made

- The zero-residue grep gate covers comments: `executable.c`'s `common()` header comment named the removed port ("alpha plan9-style bootable images for axp") — trimmed to `_MAGIC() style headers` as part of the excision (D-04: no tombstones; the plan's line list was pre-excision, the gate is authoritative)
- Host probe ran as direct host-clang against Harvey's headers only (`amd64/include` + `sys/include`), substituting for the plan's literal `9c` invocation — 9c hard-codes `-I"$PLAN9/include"` ahead of user `-I` flags, so plan9port's `u.h`/`mach.h` shadow Harvey's (and its `String.h` collides with Harvey's `string.h` on case-insensitive APFS). This is the plan's flagged Assumption-A3 fallback: record the drift, do not shim Harvey headers
- `executable.c`'s 20 host-clang errors are pre-existing Plan 9 dialect (anonymous typedef embedding `Exec;`/`Ehdr;` at lines 14-24 + the `USED` idiom) — A/B-proven identical against git HEAD; zero errors reference removed or added identifiers. Same for 7db.c's 2 `USED`-only errors (identical idiom to in-tree 5db.c, which compiles in the namespace build today)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `common()` comment named the removed port (zero-residue gate failure)**
- **Found during:** Task 1 verification prep (grep enumeration of all alpha sites)
- **Issue:** the plan's enumerated executable.c line list (72, 240-257, 418-423) missed line 456's comment `alpha plan9-style bootable images for axp "headerless" boot` — the task's own verify gate (`grep -ri 'alpha' ... --include='*.c'` → zero) and the action's no-tombstone rule both require its removal
- **Fix:** trimmed the comment to `_MAGIC() style headers` (9front keeps the line because 9front keeps the alpha entries; Harvey's D-05 excision goes further by design)
- **Files modified:** sys/src/libmach/executable.c
- **Verification:** plan's automated gate 1 prints zero residue
- **Committed in:** d8793d15f (Task 1 commit)

**2. [Rule 3 - Blocking] Plan's literal `9c` probe command cannot work on this host**
- **Found during:** Task 1 verification
- **Issue:** `9c` invokes `$cc -DPLAN9PORT -I"$PLAN9/include" $cflags "$@"` — its own include dir precedes user `-I` flags, so `<u.h>`/`<mach.h>` resolve to plan9port's copies and plan9port u.h's `#include <String.h>` resolves to Harvey's `sys/include/string.h` on case-insensitive APFS; 60 errors, all header-resolution, none from the edit
- **Fix:** substituted direct host-clang with Harvey's headers only (`-I amd64/include -I sys/include -I sys/src/cmd`) — the same sources compiled the same way, per the plan's flagged Assumption-A3 fallback ("record the drift and move on; do not shim Harvey headers"); no Harvey header or source was shimmed
- **Files modified:** none (probe invocation only)
- **Verification:** setmach.c/obj.c compile 0 errors both tasks; executable.c A/B-identical 20 pre-existing dialect errors; 7db.c 2 `USED`-only errors matching the in-tree idiom
- **Committed in:** n/a (verification-path substitution)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both preserve the plan's intent and acceptance criteria; the A3 fallback is the plan's own documented path. No scope creep; no shims to Harvey headers (D-13 and the fallback clause both held).

## Issues Encountered

- **Host-probe environment drift (Assumption A3 realized, LOW-MEDIUM as rated):** plan9port header shadowing under `9c`, plus pre-existing host-clang dialect gaps in executable.c (`Exec;` anonymous typedef embedding) and the `USED` idiom across libmach. All pre-existing and A/B-proven unrelated to this plan's edits. Consequence for downstream: plan 02-03's nm BUILD will hit the same host-dialect wall if it compiles Harvey's in-tree `executable.c` with a host compiler directly — the vendored 9cc tree's own libmach (0intro's unix-ported copy) or a namespace/native 6c compile are the proven routes. Recorded here so 02-03 plans its probe accordingly.
- **7.c and 7obj.c compile proofs are deferred by plan design:** 7.c names `/aarch64/include/ureg.h` (absolute namespace path; the headers land in 02-06 post-gate) and 7obj.c's compile is 02-03 Task 3's nm-BUILD proof. Neither is a gap in this plan; coverage entry D3 routes the downstream confirmation to verify-work.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 02-03 (libc/aarch64 + host nm):** libmach now answers arm64 objects on every table (setmach/exectab/bootimage/objtable); `sys/src/cmd/7c/7.out.h` (02-01) + this plan's `7obj.$O` dep rule make the nm BUILD's libmach compile orderable. Watch the host-dialect note above when choosing the nm probe path.
- **Plan 02-06 (objtype wiring):** lands `aarch64/include/ureg.h`, which resolves 7.c's rewritten include for in-namespace and host builds.
- **Plan 02-07:** the D-09 NMOUNT gate's host `nm` depends on this plan's libmach.
- No blockers.

## Self-Check: PASSED
