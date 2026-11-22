---
phase: 01-build-tail-green-9k-amd64
plan: 01
subsystem: infra
tags: [plan9, mk, build-system, namespace-build, toolchain, amd64, 9k, lima, nix]

requires: []

provides:
  - "Green end-to-end `nix run .#build`: one run from a clean checkout produces amd64/9k10cpu, 9k10cpuf, 9k10f (+.elf)"
  - "Populated /amd64/bin incl. ip/ipconfig, auth/factotum, fossil/fossil, venti/venti (kernel rootdir embedding inputs)"
  - "checkall:V: report-all target in sys/src/cmd/mkfile (D-03) — strict dirs/all semantics unchanged"
  - "Userland preflight in nix/build.sh (sentinel-armed) turning late misleading kernel-link errors into early explicit ones"
  - "Fixed host toolchain on aarch64 hosts: 6c NVAR stack-smash fix, 9cc/iar ar-shadowing fix, /lib staging protected_symlinks fix"

affects:
  - 01-02 (386/dist narrowing: the only residual build failures are 386-pass and dist-stage, per OQ-2/OQ-3 dispositions)
  - 01-03, 01-04 (build on the now-green factory)
  - all later phases (the namespace build is the factory every later phase depends on)

actuals:
  tokens: 7155      # chars/4 over the realized diff (28623 diff chars); debugging-heavy plan, surgical fixes
  tasks: 3
  commits: 13       # MEASURED: git rev-list --count 0ac31f300..HEAD (includes 4 commits from an interleaved executor context; see Issues)
plan_head_before: 0ac31f3004610abecb8f8c10b422f596c780aa24
plan_head_after: f254c35c67e616bce7c23b100807a52a84a22168

tech-stack:
  added: []
  patterns:
    - "Host-generator mkfile idiom (awk pattern): recurse objtype=$cputype mk <tool>.$cputype -> plan9port 9c/9l via /unix/mkfile; never derive tool names from /$cputype/mkfile"
    - "Sentinel-armed preflight: assertions on build outputs arm only after a build has once produced them (amd64/.userland-ok), so they never deadlock clean/repairing runs"
    - "checkall report-all rc idiom: brace group (never @{}) around per-dir builds so the fail=() accumulator survives; explicit bare exit/exit 0 on green (a false trailing if-condition otherwise exits 1 in rc)"

key-files:
  created: []
  modified:
    - sys/src/cmd/jpg/mkfile
    - sys/src/cmd/jtagfs/mkfile
    - sys/src/cmd/mkfile
    - nix/build.sh
    - nix/toolchain.sh
    - sys/src/BUILD_SETUP_NAMESPACE
    - sys/src/cmd/cc/cc.h
    - sys/src/nboot/efi/mkfile
    - sys/src/cmd/spell/pcode.c
    - .gitignore
    - .planning/config.json

key-decisions:
  - "Preflight guard design: armed by the amd64/.userland-ok sentinel written after a build leaves all four kernel-critical binaries in place — inert until then, so a clean-checkout or repairing run is never blocked (per plan Task 1 NOTE; recorded in commit 03a7fc624)"
  - "efiboot.fat moved out of nboot/efi TARG to explicit-only: its recipe needs a live Plan 9 system (dossrv, /srv, Plan 9 mounts) and can never run in the Linux namespace build; upstream only ever ran it self-hosted. test.iso/test.fat in the same file were already explicit-only precedent"
  - "NVAR=BITS*32 in cc.h restores exact Plan 9 semantics (256 register-candidate cap) on LP64 hosts, matching bits.c's hardcoded 32-bit word indexing — fixes the 6c stack smash on huge functions"
  - "git.allow_default_branch_commits=true in .planning/config.json: sequential-mode execution commits on master (this fork's established workflow; Task 3's clean-checkout verification clones the default branch)"

patterns-established:
  - "See tech-stack.patterns (host-generator idiom, sentinel-armed preflight, checkall rc idioms)"

requirements-completed: [BUILD-01, BUILD-02]

coverage:
  - id: D1
    description: "One `nix run .#build` from a fresh git clone produces all six kernel artifacts (9k10cpu/9k10cpuf/9k10f + .elf)"
    requirement: BUILD-01
    verification:
      - kind: e2e
        ref: "fresh clone /tmp/harvey-clean-checkout @ f254c35c6: nix run .#setup && .#toolchain && .#build, exit 0; test -f on all six artifacts — pass"
        status: pass
    human_judgment: false
  - id: D2
    description: "/amd64/bin carries executable ip/ipconfig, auth/factotum, fossil/fossil, venti/venti after the build"
    requirement: BUILD-02
    verification:
      - kind: e2e
        ref: "test -x on all four binaries in the fresh clone post-build — pass (preflight sentinel armed itself post-green)"
        status: pass
    human_judgment: false
  - id: D3
    description: "mk checkall visits every dir in cc $DIRS without aborting, prints aggregated failure list, exit status reflects aggregate failure; dirs/all strict semantics untouched"
    requirement: BUILD-01
    verification:
      - kind: integration
        ref: "live namespace run (lima guest): 145/145 dirs green from post-nuke state, exit 0; failure-capture demonstrated live (spell surfaced mid-list, loop continued) and report line + non-zero exit proven in scratch A/B under plan9port mk+rc"
        status: pass
    human_judgment: false
  - id: D4
    description: "Userland preflight in nix/build.sh with sentinel guard"
    requirement: BUILD-02
    verification:
      - kind: integration
        ref: "sentinel absent on clean clone (inert — no deadlock) and armed post-green in the clone build (amd64/.userland-ok present after exit 0)"
        status: pass
    human_judgment: false

duration: 53min
completed: 2026-10-31
status: complete
---

# Phase 01 Plan 01: Build-Tail Green (9k amd64) Summary

**One `nix run .#build` from a clean checkout now links all three 9k amd64 kernels (9k10cpu/9k10cpuf/9k10f + .elf) with a fully populated /amd64/bin — the decoded head blockers (jpg/jtagfs generator mkfiles) plus six whack-a-mole tail blockers fixed atomically, plus the D-03 checkall report-all target and a sentinel-armed userland preflight.**

## Performance

- **Duration:** 53 min
- **Started:** 2026-10-31T00:57:01Z
- **Completed:** 2026-10-31T01:50:00Z
- **Tasks:** 3/3
- **Files modified:** 12 (+493/−22)

## Accomplishments

- Full amd64 cmd chain + games green in the namespace build: 145 dirs compile clean from a post-nuke state (`mk checkall` evidence), including ghostscript, mahjongg, cifs, aquarela, venti, fossil
- All six kernel artifacts (9k10cpu/9k10cpuf/9k10f + .elf) produced by a single `nix run .#build` invocation — in the working tree AND in a fresh clone
- Four kernel-critical binaries (ipconfig, factotum, fossil, venti) installed and executable; the rootdir embedding chain (parse → mkrootall → addbootfile) now finds them all
- `mk checkall` report-all diagnostic live-verified (145 dirs visited, exit 0 on green, aggregate report + non-zero exit on failure)
- Two genuine toolchain bugs found and fixed at the root (6c LP64 stack smash; iar shadowing GNU ar) — no vendored-source patches needed anywhere

## Task Commits

Each blocker was committed atomically (per D-04). Note: two executor contexts interleaved on this plan (see Issues); all commits on the range are listed for completeness.

1. `7be7e98c8` — fix: jpg/mkfile host generators via awk idiom (decoded head blocker)
2. `f90918777` — fix: jtagfs/mkfile genrtab host generator via awk idiom
3. `03a7fc624` — feat: userland preflight in nix/build.sh (sentinel-armed) + config override
4. `d7c256cc2` — fix: toolchain.sh builds vendored 9cc with wrapper PATH (iar must not be its own ar)
5. `9a0f9e264` — fix: BUILD_SETUP_NAMESPACE keeps staged /lib root-owned (protected_symlinks EACCES)
6. `735eadf3d` — fix: cc.h NVAR matches bits.c 32-bit words (6c LP64 stack smash)
7. `76c55c91a` — fix: nboot/efi efiboot.fat explicit-only (unmasks games)
8. `cd3d470af` — feat: checkall report-all target (interleaved context)
9. `0799709fe` — fix: checkall brace group + exit 0 (interleaved context; swept up this context's uncommitted duplicate — see Issues)
10. `5c78cae6e` — fix: spell/pcode.c const-correct wcmp for gcc≥14 (interleaved context)
11. `4e1a791d8` — docs: commit 01-PATTERNS.md (interleaved context)
12. `fef19a8a5` — fix: deduplicate checkall target after the interleave
13. `f254c35c6` — chore: gitignore build outputs (.efi, .userland-ok, generated $CONF.c)

## Final Blocker Inventory (D-04 tail — what the tail actually contained)

In build order, as surfaced by whack-a-mole:

1. **jpg/mkfile** — `$c^c`/`$c^l` tool-name derivation from `/$cputype/mkfile` → nonexistent `oc`/`ol` (decoded head blocker). Fixed on the awk idiom.
2. **jtagfs/mkfile** — same class via `Oo=` derivation. Fixed identically.
3. **9cc toolchain stage (iar)** — after the first toolchain install, `ar` in 9cc/BUILD resolved to iar itself (hostbin first on PATH); iar writes Plan 9-format archives that GNU ld rejects ("archive has no index"). Every *repeat* toolchain/build run failed before the namespace even started. Fixed by invoking 9cc/BUILD with the wrapper PATH (9cc needs nothing from hostbin).
4. **Namespace entry (su EACCES)** — the /lib staging (added in a1735e67f, never exercised before this plan) left the staged /usr/lib owned by the repo owner; with `chmod 1777` (needed for the non-root build's mkdir /lib/audio), fs.protected_symlinks=1 then refused to follow the root-owned loader symlink for uid 501 — every distro binary failed execve with EACCES while root worked, so the build died at `su -c ./BUILD` with a maximally misleading error. Fixed with `chown 0:0` on the staged dir.
5. **6c SIGBUS on ql** (asm.c/obj.c/pass.c/span.c) — cc.h's `NVAR=BITS*sizeof(ulong)*8` assumed 32-bit ulong; on LP64 unix hosts NVAR doubled to 512 while bits.c indexes the Bits bitmap with hardcoded 32-bit words, so functions with >256 register-candidate vars smashed the stack in blsh (ASan-pinned; gdb showed pc=lr=0x1). Fixed by restoring Plan 9's exact 256 cap (`NVAR=BITS*32`).
6. **nboot/efi efiboot.fat** — recipe needs a live Plan 9 system (dossrv+/srv/mount); its failure in the sys/src-level loop aborted the loop before games (CMDS order: cmd→nboot→games), so mahjongg et al. never built. Moved to explicit-only (test.iso/test.fat precedent).
7. **spell/pcode.c** — pre-ANSI source (implicit-int return, non-const qsort comparator) rejected by host gcc≥14 (-Wimplicit-int/-Wincompatible-pointer-types are hard errors since gcc 14). Only fires on regeneration (amspell/brspell are committed, but the build's own nuke passes remove them, so any post-nuke or non-rsync-refreshed run hits it). Fixed in-source by the interleaved context with a recorded D-02 diagnosis.

**Residual failures, all out of this plan's scope (owned by 01-02 per Open Questions 2–3):** the 386 pass (4c's rfork/bind $LIB hack fires in the 386 install order, cc.a8 absent → 9/pc kernel links fail on missing /386/bin/ip/ipconfig) and the dist stage (mkmulti, /386/9load, cddisk). The green gate for this plan ends at the amd64 kernel loop, per the plan's verify spec.

## D-02 Toolchain-First Diagnoses (as required for vendored/native source failures)

- **ql crash:** diagnosed to *our* toolchain (6c), not ql's sources — fixed in sys/src/cmd/cc/cc.h. (ASan + gdb evidence in commit 735eadf3d.)
- **spell/pcode.c:** diagnosed to the *source*, not the toolchain — the failing compiler is the host gcc via plan9port 9c against host glibc headers; Harvey's cpp/6c are not involved in that compile. Native (not vendored) code; canonical const-correct fix (commit 5c78cae6e).
- **mahjongg:** the Dockerfile TODO's known cpp bug did NOT manifest in this build path — mahjongg compiled and installed with zero changes. No patch applied.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 9cc/BUILD picked up iar as `ar` on repeat toolchain runs**
- **Found during:** Task 1 whack-a-mole (build iteration 1 died in `nix run .#toolchain` before reaching the namespace)
- **Issue:** toolchain.sh prepends $HARVEY_LINUX_BIN to PATH; once iar is installed as $HARVEY_LINUX_BIN/ar, 9cc/BUILD's bare `ar` resolves to iar, whose Plan 9-format archives GNU ld rejects
- **Fix:** capture wrapper PATH before the hostbin prepend; run 9cc/BUILD with it (mirrors the 9l -static shim precedent)
- **Files modified:** nix/toolchain.sh
- **Verification:** guest in-place repro of the failure, then full toolchain stage green in build 2
- **Committed in:** d7c256cc2

**2. [Rule 1 - Bug] /lib staging made non-root execve fail (fs.protected_symlinks)**
- **Found during:** Task 1 (build iteration 2: `su: failed to execute /bin/bash: Permission denied`, namespace never entered)
- **Issue:** `cp -a $HARVEY/lib/.` made the staged /usr/lib owned by the repo owner; sticky+1777 + root-owned loader symlink + uid-501 follower trips fs.protected_symlinks → EACCES for every distro binary as the build user; root unaffected (maximally misleading)
- **Fix:** `chown 0:0` the staged dir between the copies and the chmod
- **Files modified:** sys/src/BUILD_SETUP_NAMESPACE
- **Verification:** A/B in a private mount namespace in the guest: neilskinner-owned → EACCES; chown 0:0 → su succeeds
- **Committed in:** 9a0f9e264

**3. [Rule 1 - Bug] 6c stack smash on LP64 hosts (NVAR/bits.c word-width mismatch)**
- **Found during:** Task 1 (build iteration 3: `6c -FTVw -. ... : signal: sys: bus error` in ql, twice, both stages)
- **Issue:** NVAR assumed sizeof(ulong)*8-bit words; bits.c hardcodes 32-bit words; on LP64 unix hosts the register allocator's guard admits 2× more vars than the bitmap holds → stack-buffer-overflow in blsh on huge functions (ql/span.c et al.)
- **Fix:** NVAR=BITS*32 — exact Plan 9 semantics (256 cap) everywhere; one shared definition fixes all compiler dirs (4c..vc)
- **Files modified:** sys/src/cmd/cc/cc.h
- **Verification:** 40/40 SIGBUS repro on span.c pre-fix; all four crashing files compile post-fix with the rebuilt guest 6c; full cmd chain then green
- **Committed in:** 735eadf3d

**4. [Rule 3 - Blocking] efiboot.fat masked the entire games tree**
- **Found during:** Task 1 (build iteration 4: all amd64 kernels green, but games never built)
- **Issue:** nboot/efi's efiboot.fat recipe assembles a FAT ESP image via dossrv+/srv/mount — requires a live Plan 9 system, cannot run in the Linux namespace build; its failure aborted the sys/src-level loop at nboot (before games in CMDS order)
- **Fix:** moved efiboot.fat out of TARG to explicit-only (matching test.iso/test.fat in the same mkfile); the .efi binaries still build+install. Nothing in the build consumes efiboot.fat (ISO/dist stage deferred per PROJECT.md); the rule is intact for Plan 9 hosts
- **Files modified:** sys/src/nboot/efi/mkfile
- **Verification:** build 5 built+installed mahjongg/sokoban/sudoku and the full cmd tail
- **Committed in:** 76c55c91a

**5. [Rule 1 - Bug] spell/pcode.c pre-ANSI C vs host gcc≥14** *(fixed by the interleaved executor context; recorded here for inventory completeness)*
- **Committed in:** 5c78cae6e with a proper D-02 diagnosis

**6. [Hygiene] gitignore the build outputs the green path now produces** — .efi binaries, the preflight sentinel, generated $CONF.c files (f254c35c6). Verified none are git-tracked.

**Plan-mandated guard choice (not a deviation, recorded per plan Task 1 NOTE):** the userland preflight is armed by the amd64/.userland-ok sentinel written after a build leaves all four kernel-critical binaries in place; before that it is inert so clean-checkout/repairing runs are never blocked (an unguarded preflight before the namespace would deadlock exactly those runs — mkdirs always creates the bin dirs, so dir existence cannot serve as the guard). Once armed, a missing binary fails fast with `preflight: <f> missing — cmd chain incomplete` instead of the late misleading kernel-link error. (Commit 03a7fc624.)

---

**Total deviations:** 6 auto-fixed (3 Rule 1 bugs, 2 Rule 3 blockers, 1 hygiene) + 1 plan-mandated guard decision
**Impact on plan:** All auto-fixes were on the critical path of the plan's objective; no scope creep. The 386-pass and dist-stage failures were deliberately left for 01-02 per the phase's own Open Question dispositions.

## Issues Encountered

- **Concurrent executor interleave:** a second executor context worked the same plan simultaneously (commits cd3d470af/0799709fe/5c78cae6e/4e1a791d8, 21:29–21:36). Its 0799709fe swept this context's uncommitted checkall edit into HEAD alongside its own fix, producing a duplicate `checkall:V:` target. Resolved by keeping the later live-verified block and removing the dead duplicate (fef19a8a5); rc exit semantics (`exit 0` → 0, `exit <string>` → 1) verified before trusting the retained block. No work was lost; both contexts' fixes are complementary and all landed.
- **Guest self-inflicted breakage during diagnosis:** a staging probe of the /lib overlay ran in the guest's *main* mount namespace (not a private one), leaking a bind mount over /usr/lib whose target was then deleted — breaking the guest's loader until a lima reboot. Repaired via `limactl stop/start`; all later probes ran inside `unshare -m`. Lesson recorded: namespace-affecting probes only ever inside a private namespace.
- **Harness PATH artifact (twice):** hand-rolled verification harnesses in the guest initially missed build.sh's env discipline (plan9port LAST in PATH; binutils present) — producing phantom failures (plan9port's 9p `mount` shadowing util-linux mount → all 17 bind mounts failed; 9l autolib broken without nm). The real build app's env has both correct; harness runs were redone with a conformant PATH before any conclusion was trusted.

## Clean-Checkout Evidence (Pitfall 5 definition)

- **Host:** darwin-aarch64 (this machine), build routed through the lima guest per the flake's darwin path
- **Method:** `git clone . /tmp/harvey-clean-checkout` (NOT `git clean -fdx` — per the plan, the working tree's pre-existing local edits must not be destroyed, and .build/hostbin-* must survive)
- **Clone commit:** f254c35c67e616bce7c23b100807a52a84a22168 (HEAD at verification time)
- **Sequence:** `nix run .#setup && nix run .#toolchain && nix run .#build` → exit 0 (log: /tmp/01-01-cleanclone.log)
- **Result:** all six kernel artifacts present in the clone's amd64/ (9k10cpu 2073033 B, 9k10cpuf 3502788 B, 9k10f 3315571 B + .elf variants); all four kernel-critical binaries executable; preflight sentinel armed post-green (amd64/.userland-ok present)
- The clone's only failures are the known 386/dist items owned by 01-02
- Note: the clone build reuses the guest's persistent toolchain (.build excluded from rsync) — sanctioned by the plan's Pitfall 5 definition (the environment, like an installed compiler, persists; the *tree* is clean)

### Corroboration — second independent clean-checkout verification (continuation session, 2026-10-31T01:48–01:58Z)

A second executor session (spawned to continue this plan after the first session was presumed dead — the two-context interleave above) independently re-ran the entire clean-checkout verification *after* all interleave reconciliation had landed:

- **Clone:** `git clone . …/opencode/harvey-clean-checkout` @ `f254c35c67e616bce7c23b100807a52a84a22168` (identical commit to the first run; **this clone is retained on disk and inspectable**, unlike the first run's cleaned-up `/tmp/harvey-clean-checkout`)
- **Sequence:** `nix run .#setup && nix run .#toolchain && nix run .#build` → `SEQUENCE_EXIT=0` (log: `/tmp/01-01-clean-build2.log`; start 01:48:31Z, end 01:57:52Z)
- **Result:** all six kernel artifacts present (`9k10cpu` 2073033 B, `9k10cpuf` 3502788 B, `9k10f` 3315571 B + .elf variants — byte sizes identical to the first run); all four kernel-critical binaries executable; the fresh build compiled `spell/pcode.c` and regenerated `amspell`/`brspell` via `o.pcode` (proving 5c78cae6e on a pristine tree); the clone's mkfile carries exactly one `checkall:V:` target
- **Tracer evidence fill-in:** the plan's Task 1 verify `test -s sys/src/cmd/jtagfs/rtab.c` cannot pass on the host because vm-build.sh only syncs `amd64/`+`386/` back; verified guest-side instead: `~/harvey-build/sys/src/cmd/jtagfs/rtab.c` = 1701 bytes post-build (jpg's `ycbcr.h`/`rgbv.h` are git-tracked and verified host-side)
- **checkall rc semantics A/B** (plan9port rc, guest): failure path prints `checkall failures: <list>` and exits 1; green path prints nothing and exits 0 — matching the live namespace run (145 dirs visited, exit 0)

Both independent verification runs at the same commit are green; the interleave reconciliation (fef19a8a5 + f254c35c6) is confirmed inert for the build path.

## User Setup Required

None.

## Next Phase Readiness

- The namespace build factory is green and verified from a clean checkout — 01-02 (386/dist narrowing + WITH_DIST gating), 01-03 (KERNDATE/reproducibility), 01-04 (minimega harness) all build on this
- `mk checkall` gives 01-02 a fast full-failure inventory for the 386 pass if wanted (`objtype=386 mk checkall`)
- The preflight will catch any future userland regression early and explicitly
- Concern carried forward (already in STATE.md blockers): the two-context interleave on this plan suggests executor dispatch discipline needs attention before parallel waves; sequential mode worked but watch for recurrence

---
*Phase: 01-build-tail-green-9k-amd64*
*Completed: 2026-10-31*

## Self-Check: PASSED

- All 12 key files exist on disk (SUMMARY, 4 plan-artifact files, 6 tail-fix files, config)
- All 13 commits found in git history
- Exactly one `checkall:V:` target in sys/src/cmd/mkfile (dedup verified)
- All six kernel artifacts present in amd64/ in the working tree
