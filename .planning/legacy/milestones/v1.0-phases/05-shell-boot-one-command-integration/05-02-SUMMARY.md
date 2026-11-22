---
phase: 05-shell-boot-one-command-integration
plan: 02
subsystem: infra
tags: [harvey, aarch64, boot, paqfs, mk, plan9-c, qemu, r9, mkimage]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-01)
    provides: build/aarch64/stacksmoke.c (the D-15 binary Task 2 compiles and ships) + the harness hygiene conventions run-r9.sh documents
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: the vendored 7c/7l toolchain, the lima namespace-build harness (vm-run.sh/nsrun.sh), and the 16-binary subset build this plan extends
  - phase: 04-r9-userspace-exec-harvey-r9-abi
    provides: the paqfs byte-format contract (529ab12b/6b46e688 magics, adler32 seed-0) and the loader+fdtput-patched-DTB delivery recipe (D-01 amended)
provides:
  - out/{boot,init,paqfs,stacksmoke} aarch64 binaries (R_MAGIC-proven) — the real boot chain's userland
  - sys/src/9k/boot/{mkfile.aarch64,bootr9.c} — the additive, objtype-gated boot(8) build with the embed-method hand-written bootconf (/rootfs.img contract)
  - build/aarch64/root-tree/ committed image skeleton (scoped namespace D-08, scoped termrc D-05, profile) + build/aarch64/mkimage.sh bake → build/aarch64/bootfs.paq with /boot/factotum absent (D-02)
  - build/aarch64/run-r9.sh — the dev-time qemu launcher (required-env R9_KERNEL/R9_DTB, loader+fdtput argv, --log gate mode) the 05-04..05-09 gates drive
affects: [05-03 (donor trace consumes the built chain), 05-04 (tracer consumes the image + launcher), 05-05 (devroot raw node serves /rootfs.img), 05-06..05-09 (gates), 05-11 (composition app wraps both scripts)]

# Actuals (#2632) — pairs with the plan's `estimate` (90000 tokens, confidence: low).
actuals:
  tokens: 6347       # chars/4 over the realized diff (25391 chars); estimate overshot ~14x — the research/pattern maps did the heavy lifting
  tasks: 3
  commits: 3         # MEASURED: git rev-list --count 0e871ab2a..HEAD (task commits); the metadata commit rides on top
plan_head_before: "0e871ab2a2deeba803f33de87c3676b9578a45b6"
plan_head_after: "a783c938e37308c6a2984c7b0c5c795ca48da0d9"

# Tech tracking
tech-stack:
  added: []          # no new dependencies anywhere (T-05-SC held: zero package installs)
  patterns:
    - "hand-written bootconf: when no kernel config exists, the awk-generated boot$CONF.c equivalent is written by hand — method[] + cpuflag/rootdir/bootdisk AND the generated file's two further load-bearing symbols (main → boot, cfs = 0)"
    - "per-target installs only (mk init.install / paqfs.install / <cmd>.install) — never a dir's whole mkmany TARG (paqfs's would build mkpaqfs for aarch64, which only the host needs)"
    - "bake-time verbatim copies (rc/lib/rcmain, adm/timezone/local) — the image diverges from the stock tree by CONTENT selection, never by editing shared sources (amd64-boot safety keys on image contents)"
    - "required-env launcher contract: R9_KERNEL/R9_DTB have no defaults; R9_INITRD defaults to the baked image and self-bakes when absent"

key-files:
  created:
    - sys/src/9k/boot/bootr9.c
    - sys/src/9k/boot/mkfile.aarch64
    - build/aarch64/mkimage.sh
    - build/aarch64/run-r9.sh
    - build/aarch64/root-tree/lib/namespace
    - build/aarch64/root-tree/rc/bin/termrc
    - build/aarch64/root-tree/usr/glenda/lib/profile
  modified:
    - build/aarch64/subset-build.sh
    - .gitignore

key-decisions:
  - "bootr9.c supplies six symbols, not four: method[]/cpuflag/rootdir/bootdisk plus main() and cfs=0 — the awk generator emits all six (parse mkbootconf) and the link cannot resolve without them (Rule 3)"
  - "BOOTFILES trimmed to the real 7l closure by reference analysis: paq.7 is the lone trim (embed-only table never names it); bootcache stays for statbuf (settime/local extern it), bootip stays via local.c's configip, printstub linked explicitly so its fmt stubs beat libc's"
  - "subset-build rsyncs out/ per-file without --delete; stage-build.sh keeps its wholesale --delete — last writer wins on out/, and mkimage.sh's manifest assert names the fix (re-run subset-build.sh) when its inputs are absent"
  - "adm/timezone/local is NOT committed to root-tree/ — mkimage.sh copies the repo's adm/timezone/local at bake time (the action text's 'copy at bake time' over the files_modified listing)"

patterns-established:
  - "image-content divergence without shared-source edits: scoped files live in build/aarch64/root-tree/ and are staged; stock files (rcmain) are copied verbatim at bake time"
  - "bake-manifest assertion: every mandated path (incl. empty mount-point dirs) is asserted in the staged tree pre-bake, and /boot/factotum's ABSENCE is asserted (D-02 auth-stub-by-absence is a property of the image, enforced at bake)"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "boot(8) compiles and links for aarch64 via the additive objtype-gated mkfile.aarch64 with the hand-written embed-method bootconf (/rootfs.img, cpuflag=0, rootdir=/root); out/boot carries R_MAGIC 00008c47; zero diff in every pre-existing 9k source"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "sh build/aarch64/subset-build.sh (lima guest namespace) → SUBSET-BUILD-OK; od -An -tx1 -N4 build/aarch64/out/boot = 00008c47"
        status: pass
      - kind: other
        ref: "git diff --exit-code sys/src/9k/ + git status --porcelain limited to the two new files; bootr9.c greps for \"embed\"/rootfs.img/cpuflag/rootdir"
        status: pass
    human_judgment: false
  - id: D2
    description: "init, paqfs, stacksmoke join the aarch64 build set with the 16-binary subset unregressed; mkimage.sh bakes bootfs.paq carrying the full mandated skeleton (scoped termrc/namespace, verbatim rcmain, timezone, profile, empty mount-point dirs) with /boot/factotum absent and both paqfs magics correct"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "presence loop over out/{init,paqfs,stacksmoke} + aarch64/bin 16-binary regression list; sh build/aarch64/mkimage.sh → 1015975-byte image, header 529ab12b, trailer 6b46e688"
        status: pass
      - kind: other
        ref: "skeleton greps (bind -c #e /env, bind /root/$cputype/bin /bin, bind #d /fd, exec rc -i, stacksmoke present, no ipconfig/ndb/rio in termrc); bake-time manifest assert incl. factotum absence"
        status: pass
    human_judgment: false
  - id: D3
    description: "run-r9.sh assembles the config_virtexec.toml qemu argv (loader at 0x48000000) with the two-cell fdtput /chosen patch on a throwaway DTB copy; R9_KERNEL/R9_DTB are required-env with named errors and no sibling-checkout default; --log mode starts with a fresh log"
    requirement: INT-01
    verification:
      - kind: other
        ref: "sh -n + R9_KERNEL= rejection naming R9_KERNEL + /nonexistent kernel/dtb rejection pre-spawn"
        status: pass
      - kind: integration
        ref: "stub-qemu functional smoke: argv shape byte-compared against the TOML values; dtc dump of the patched DTB shows linux,initrd-start=<0x0 0x48000000>, linux,initrd-end=<0x0 0x480f80a7> (hi,lo cells)"
        status: pass
    human_judgment: false

# Metrics
duration: 15 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 02: Harvey-side real-chain supply chain — boot(8)/init/paqfs/stacksmoke aarch64 builds, paqfs image bake, dev-time qemu launcher Summary

**boot(8) now builds for aarch64 as an additive, objtype-gated target with a hand-written embed-method bootconf naming /rootfs.img; init, paqfs, and stacksmoke join the proven subset build; mkimage.sh bakes a manifest-asserted bootfs.paq (both magics correct, /boot/factotum absent per D-02); and run-r9.sh stands ready as the required-env qemu launcher the 05-04+ gates drive — with zero diff in every shared amd64-boot source.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-11-03T00:52:59Z
- **Completed:** 2026-11-03T01:07:57Z
- **Tasks:** 3
- **Files modified:** 9 (7 created, 2 modified — subset-build.sh, .gitignore)

## Accomplishments

- **The pid-1 binary exists.** `sys/src/9k/boot/mkfile.aarch64` + `bootr9.c` are the entire sys/ diff: a standalone aarch64 boot(8) build (there is no standalone boot mkfile — the amd64 path awk-generates the bootconf from a kernel config; RESEARCH Pattern 2). `bootr9.c` hand-writes the generated equivalent: `method[]` names the embed method first with arg `/rootfs.img` (the 05-05 devroot raw-node contract), `cpuflag=0` (terminal → `init -t`, D-05), `rootdir=/root`, `bootdisk=""`, plus the generated file's two other load-bearing symbols (`main` → `boot`, `cfs=0`). BOOTFILES is bootmkfile's list trimmed to the real 7l closure by reference analysis — `paq.7` is the lone trim; `bootcache.7` stays for `statbuf`, `bootip.7` via local.c's `configip`, `printstub.7` linked explicitly so its fmt stubs beat libc's. Verified in the lima guest namespace: libboot.a7 archives via the ar/membername idiom, 7l links `boot` (163116 bytes), R_MAGIC `00008c47`. `git diff --exit-code sys/src/9k/` is clean — amd64 boot behavior untouched (T-05-03).
- **The image supply chain is complete and byte-format-proven.** subset-build.sh gains init (`mk init.install` — cmd/mkfile's special `$BIN/init` rule lands it at `/aarch64/init`, exactly where execinit looks), paqfs (per-target — never the mkmany TARG), and a guest compile of 05-01's stacksmoke.c; all four harvest to `build/aarch64/out/` with R_MAGIC asserts, and all 16 prior subset binaries still install. mkimage.sh stages root-tree/ + the binaries + verbatim `rc/lib/rcmain` + the repo's `adm/timezone/local`, creates the Pitfall-8 empty mount-point dirs, asserts the full manifest **including /boot/factotum's ABSENCE** (D-02 auth-stub-by-absence is a property of the image, enforced at bake), bakes via the host mkpaqfs, and asserts both proven magics: header `529ab12b`, trailer `6b46e688` (1,015,975 bytes).
- **run-r9.sh enforces the composition contract.** R9_KERNEL and R9_DTB are required-env with named errors — no implicit `../r9` default anywhere (D-10/D-11: the 05-11 flake app passes both from the r9 package); R9_INITRD defaults to bootfs.paq and self-bakes via mkimage.sh when absent. The argv matches config_virtexec.toml's [qemu] values exactly (verified byte-for-byte against a stub qemu), and the two-cell fdtput /chosen patch lands correctly (dtc dump: `linux,initrd-start=<0x0 0x48000000>`, `linux,initrd-end=<0x0 0x480f80a7>` — hi,lo cells per the Phase-4 recipe). `--log FILE` is the fresh-log gate mode (Pitfall 10/D-16); teardown kills the spawned qemu's process group.

## Task Commits

Each task was committed atomically (harvey-only plan — no r9 files touched):

1. **Task 1: additive aarch64 boot(8) build** — `73bc47d66` (feat)
2. **Task 2: subset growth + root-tree skeleton + mkimage.sh bake** — `80e9ac1cd` (feat)
3. **Task 3: run-r9.sh dev-time qemu launcher** — `a783c938e` (feat)

**Plan metadata:** (the docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS)

## Files Created/Modified

- `sys/src/9k/boot/bootr9.c` — hand-written bootconf: embed method first, /rootfs.img arg, cpuflag=0, rootdir=/root, bootdisk="", main, cfs=0
- `sys/src/9k/boot/mkfile.aarch64` — the additive objtype-gated boot build (libboot.a7 + 7l link; `clean` keeps the guest tree pristine)
- `build/aarch64/subset-build.sh` — gains the boot-chain section (boot build/harvest/clean, init.install, paqfs.install, stacksmoke compile), the out/ rsync-back, and per-binary R_MAGIC asserts; libflate joins the libs loop
- `build/aarch64/mkimage.sh` — the bake: staging, manifest assert (factotum absent), mkpaqfs, magic checks
- `build/aarch64/root-tree/lib/namespace` — scoped per D-08 (k10root.namespace shape, /root/ prefix, no network binds, no factotum mount)
- `build/aarch64/root-tree/rc/bin/termrc` — scoped per D-05 (stacksmoke first, ends `exec rc -i`)
- `build/aarch64/root-tree/usr/glenda/lib/profile` — near-empty (rcmain's profile path has a file)
- `build/aarch64/run-r9.sh` — the dev-time launcher (required-env contract, argv assembly, fdtput patch, --log gate mode, group-kill hygiene)
- `.gitignore` — `/build/aarch64/bootfs.paq` (the baked image is gitignored, per must_haves)

## Decisions Made

- **bootr9.c carries six symbols, not four.** The plan's "exactly the four extern bootconf symbols" names the configuration surface (method/cpuflag/rootdir/bootdisk); the awk generator additionally emits `main()` and `cfs` (parse mkbootconf, verified), and the 7l link fails without them. The file's header comment records this provenance.
- **out/ ownership: per-file sync from subset-build, wholesale --delete stays stage-build's.** Both scripts remain individually coherent; mkimage.sh's manifest assert names subset-build.sh as the fix when the image inputs are absent. 05-10's "unchanged staged-init supply chain" constraint is honored — stage-build.sh is untouched.
- **`adm/timezone/local` is not committed to root-tree/;** the action text ("copy … at bake time") governs over the files_modified listing — mkimage.sh copies the repo's `adm/timezone/local` into the staged tree.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] paqfs link failed: libflate.a missing from the subset libs loop**
- **Found during:** Task 2 (first subset-build run with paqfs.install)
- **Issue:** paqfs.c includes `<flate.h>`, whose `#pragma lib "libflate.a"` (tab-separated — easy to miss) makes 7l demand `/aarch64/lib/libflate.a`; the Phase-2 libs loop never built it (no prior subset binary links flate)
- **Fix:** added libflate to the guest libs loop (after libauthsrv; it depends only on libc) and to the header comment's closure description
- **Files modified:** build/aarch64/subset-build.sh
- **Verification:** rerun green — SUBSET-BUILD-OK, out/paqfs R_MAGIC 00008c47
- **Committed in:** `80e9ac1cd` (part of the Task 2 commit)

**2. [Rule 1 - Acceptance gate] termrc's own header comment tripped the plan's negative grep**
- **Found during:** Task 2 verification
- **Issue:** the plan's acceptance runs `! grep -qiE 'ipconfig|ndb|rio'` over the committed termrc — my header comment named those programs ("no ipconfig/ndb and no rio"), failing the gate on prose, not content
- **Fix:** reworded the comment to "the embedded root has neither" without naming the programs
- **Files modified:** build/aarch64/root-tree/rc/bin/termrc
- **Verification:** full Task 2 verify battery re-run green; image rebaked (magics re-verified)
- **Committed in:** `80e9ac1cd` (part of the Task 2 commit)

**3. [Rule 3 - Blocking] bootr9.c needed main() and cfs beyond the four named symbols**
- **Found during:** Task 1 (link analysis before writing)
- **Issue:** boot.h declares `extern int (*cfs)(int)` (used by boot.c's connectroot) and nothing supplies `main` — both are emitted by the awk generator the hand-written file replaces; without them the link cannot resolve
- **Fix:** bootr9.c defines `int (*cfs)(int) = 0` (the generated no-cfs default) and `main` calling `boot(argc, argv)`, documented in the file header
- **Files modified:** sys/src/9k/boot/bootr9.c
- **Verification:** boot links, R_MAGIC confirmed; the plan's bootr9.c greps (embed/rootfs.img/cpuflag/rootdir) all pass
- **Committed in:** `73bc47d66` (part of the Task 1 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 3 blocking, 1 Rule 1 acceptance gate)
**Impact on plan:** All three are the plan's intent executed faithfully — the generated-equivalent symbol set, the pragma-lib closure completion, and the negative-grep-clean comment. No scope creep; no plan content changed.

## Issues Encountered

- None beyond the deviations above. The boot build linked green on its first guest run; the bake's magic asserts passed on the first invocation.

## User Setup Required

None - no external service configuration required. (Note for driving run-r9.sh by hand: it must run in an environment with qemu-system-aarch64 and dtc/fdtput on PATH — the r9 devshell provides both today; 05-11's compose app carries them in runtimeInputs.)

## Known Stubs

None. The near-empty `usr/glenda/lib/profile` is the plan-mandated content (D-05's rcmain profile path requires a file to exist; nothing is wired to its contents), not an unfinished stub.

## Threat Flags

None — no new network endpoints, auth paths, or trust-boundary surface beyond the plan's threat model. T-05-03 (zero diff in shared boot-chain C sources) verified by `git diff --exit-code sys/src/9k/` + explicit checks of rcmain/init.c/namespace/termrc. T-05-04 (auth-stub-by-absence) is enforced by mkimage.sh's manifest assert on every bake. T-05-SC: zero package installs.

## Next Phase Readiness

- 05-03's donor trace has its target binaries (the full real chain built for aarch64); 05-04's tracer consumes `bootfs.paq` + `run-r9.sh` with an explicit r9 kernel path.
- The cross-plan contract literals are in place: `/rootfs.img` (bootr9.c ↔ 05-05's devroot raw node), the two-cell /chosen patch (run-r9.sh ↔ the kernel's DTB initrd discovery), and the image skeleton (mount points pre-exist for 05-05+ namespace machinery).
- Harvey HEAD after this plan's task commits: `a783c938e` (metadata commit follows).

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: bootr9.c, mkfile.aarch64, mkimage.sh, run-r9.sh, root-tree/{lib/namespace,rc/bin/termrc,usr/glenda/lib/profile} — all FOUND on disk and committed; SUMMARY.md FOUND.
- Commits: 73bc47d66, 80e9ac1cd, a783c938e — all FOUND in git log.
- Plan-level <verification>: all four items re-verified at summary time (four R_MAGIC binaries + 16-subset unregressed; bootfs.paq header 529ab12b / trailer 6b46e688; run-r9.sh env contracts; zero diff in sys/src/9k/ and the shared rcmain/init.c/namespace/termrc sources).
