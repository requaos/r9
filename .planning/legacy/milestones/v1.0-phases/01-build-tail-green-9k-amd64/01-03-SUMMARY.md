---
phase: 01-build-tail-green-9k-amd64
plan: 03
subsystem: infra
tags: [plan9, mk, build-system, reproducibility, kerndate, source-date-epoch, amd64, 9k, lima, nix]

requires:
  - phase: 01-build-tail-green-9k-amd64
    provides: "01-01: green namespace build (preflight block in nix/build.sh — this plan's precondition) + conformant-PATH lesson that shaped the demo harness; 01-02: amd64-only green gate the demo's kernel relink rides on"

provides:
  - "KERNDATE env var honored at both -DKERNDATE compile sites (sys/src/9k/k10/mkfile $p$CONF rule, sys/src/9k/mk/portmkfile $CONF.$O rule) with date -n as in-idiom fallback only (BUILD-04)"
  - "Host-side KERNDATE derivation from git commit timestamp: exported by nix/build.sh before namespace entry; computed by nix/vm-build.sh before rsync and passed through the ssh line (guest has no .git)"
  - "build/REPRODUCIBILITY.md: kernel-scope policy doc — scope, derivation, override, exceptions (conffile same-path; no-git wall-clock fallback), verification procedure"
  - "Demonstrated bit-identical pinned-timestamp rebuild pair of 9k10cpu (+.elf) in the lima guest namespace, with a differing-KERNDATE negative control"

affects:
  - 01-04 (minimega harness boots kernels built under this policy)
  - Phase 5 (clean-checkout reproducibility claims rest on this policy — the STATE.md blocker is resolved)

actuals:
  tokens: 1651      # chars/4 over the realized diff (6604 diff chars, base..HEAD at SUMMARY write)
  tasks: 2
  commits: 2        # MEASURED: git rev-list --count d88e9572a..HEAD at SUMMARY write (final docs commit lands after)
plan_head_before: d88e9572a38651b0ad84a179f06af25123934428
plan_head_after: a1efaf948e491c66e3bf95e8b2a3269a99eb50b8

tech-stack:
  added: []
  patterns:
    - "SOURCE_DATE_EPOCH-style pin in Plan 9 mk recipes: '-DKERNDATE='`{if(~ $#KERNDATE 0) KERNDATE=`{date -n}; echo $KERNDATE} — env honored, build clock is in-idiom fallback only"
    - "Host-side derivation for guest builds: vm-build.sh's rsync excludes .git, so any commit-derived value must be computed before the ssh line and injected as an env prefix on the remote command"
    - "Guest namespace harness discipline (extends 01-01's PATH lesson): su resets PATH — re-source the captured exports (mirrors BUILD's `. buildnamespace`) AND append /rc/bin:. (repo's rc/bin/membername is required by bootmkfile's libboot archive rule)"

key-files:
  created:
    - build/REPRODUCIBILITY.md
  modified:
    - sys/src/9k/k10/mkfile
    - sys/src/9k/mk/portmkfile
    - nix/build.sh
    - nix/vm-build.sh

key-decisions:
  - "KERNDATE derivation is anchored to the checkout: `(cd \"$HARVEY\" && git log -1 --format=%ct)` in both nix entry points — both scripts support HARVEY != \$PWD, so a bare git log could silently pin the caller's cwd repo instead (Rule 2 deviation; gate literal preserved verbatim)"
  - "Negative control is part of the evidence standard for reproducibility claims: KERNDATE=1700000001 must produce a DIFFERENT binary (else 'deterministic anyway' is unfalsified), and restoring 1700000000 must reproduce the original hash"
  - "Policy scope is kernels-only by locked decision; every known non-determinism source (conffile absolute-path embedding, no-git wall-clock fallback) is written down in build/REPRODUCIBILITY.md rather than chased (Assumption A4 held — no new variance surfaced)"

patterns-established:
  - "See tech-stack.patterns"

requirements-completed: [BUILD-04]

coverage:
  - id: D1
    description: "Both -DKERNDATE sites honor KERNDATE env with date -n only as in-idiom fallback; nix/build.sh exports git-derived KERNDATE; nix/vm-build.sh computes it host-side and passes it through ssh"
    requirement: BUILD-04
    verification:
      - kind: static
        ref: "grep gates: 'if(~ $#KERNDATE 0)' present in both mkfiles; 'git log -1 --format=%ct' in nix/build.sh; KERNDATE in nix/vm-build.sh; per-file 'date -n' count == 1 (in-idiom) in both mkfiles; bash -n clean on both scripts"
        status: pass
      - kind: e2e
        ref: "lima guest namespace run: recipe echo shows the idiom live (`6c -FVw '-DKERNDATE='`{if(~ $#KERNDATE 0) ...} k10cpu.c`); rc idiom A/B on host plan9port rc: set → echoed, unset → date -n fallback invoked"
        status: pass
    human_judgment: false
  - id: D2
    description: "build/REPRODUCIBILITY.md carries scope/derivation/override/exceptions/verification and names the conffile same-path exception; no userland bit-reproducibility claim"
    requirement: BUILD-04
    verification:
      - kind: static
        ref: "grep gates: KERNDATE, conffile, scope (ci), override (ci) all present; doc read-through confirms no full-tree claim"
        status: pass
    human_judgment: false
  - id: D3
    description: "Two same-path pinned-KERNDATE rebuilds of 9k10cpu produce bit-identical sha256 (+ negative control)"
    requirement: BUILD-04
    verification:
      - kind: e2e
        ref: "lima guest namespace, KERNDATE=1700000000, rm k10cpu.6 9k10cpu 9k10cpu.elf between runs: run1 == run2 (diff empty); KERNDATE=1700000001 differs; restore 1700000000 == run1"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-10-31
status: complete
---

# Phase 01 Plan 03: KERNDATE Pin + Reproducibility Policy Summary

**Both `-DKERNDATE` compile sites now honor a SOURCE_DATE_EPOCH-style `KERNDATE` env var (falling back to `date -n` only when unset), both nix entry points derive the pin host-side from the checkout's git commit timestamp and plumb it into the namespace/lima-guest build, `build/REPRODUCIBILITY.md` documents the kernel-scope policy with an honest exception list — and a pinned-timestamp rebuild pair of `9k10cpu` in the lima guest produced bit-identical sha256, with a differing-KERNDATE negative control proving the pin is what makes the bytes move.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-10-31T02:06:39Z
- **Completed:** 2026-10-31T02:14:26Z
- **Tasks:** 2/2
- **Files modified:** 4 modified + 1 created (+97/−3 across both task commits)

## Accomplishments

- `sys/src/9k/k10/mkfile` (`$p$CONF` rule) and `sys/src/9k/mk/portmkfile` (`$CONF.$O` rule) both carry the identical env-honoring rc idiom `` `{if(~ $#KERNDATE 0) KERNDATE=`{date -n}; echo $KERNDATE} `` — the guest build log shows it live in the recipe echo
- `nix/build.sh` exports `KERNDATE` (git commit timestamp, env-override default) before namespace entry; `nix/vm-build.sh` computes it before the rsync and injects it as an env prefix on the ssh command line — the guest checkout's lack of `.git` is fully bypassed
- `build/REPRODUCIBILITY.md` (84 lines) documents scope (three 9k amd64 kernels only — no userland claim), derivation, override (`KERNDATE=<epoch> nix run .#build`), exceptions (conffile absolute-path embedding → same-path reproducibility only; no-git wall-clock fallback is non-reproducible by design), and the verification procedure
- Demonstrated, not assumed: two pinned rebuilds bit-identical; a KERNDATE+1 control build differs; a restore build matches the original hashes again (Assumption A4 — no variance beyond KERNDATE/conffile — held on same-path builds)

## Task Commits

Each task was committed atomically:

1. **Task 1: Honor KERNDATE env at both -DKERNDATE sites + plumb host-side derivation through nix/build.sh and nix/vm-build.sh** — `8ed379f42` (feat; exactly the four declared files)
2. **Task 2: Write build/REPRODUCIBILITY.md and demonstrate bit-identical kernel rebuilds** — `a1efaf948` (docs; exactly the one declared file)

**Plan metadata:** committed after this summary (docs: complete KERNDATE/reproducibility plan)

## Demonstrated Evidence (BUILD-04)

Environment: lima guest (`lima-harvey`, linux-aarch64), private mount namespace entered via the same machinery as the real build (`unshare -m` + BUILD_SETUP_NAMESPACE-equivalent bind mounts), conformant env via `nix develop`. Guest checkout: `~/harvey-build` with the two updated mkfiles rsynced in. Exact commands (run as the build user inside the namespace):

```
cd /sys/src/9k/k10
rm -f k10cpu.6 9k10cpu 9k10cpu.elf
KERNDATE=1700000000 mk CONF=k10cpu 9k10cpu
sha256sum 9k10cpu 9k10cpu.elf > /tmp/kerndate-run1
rm -f k10cpu.6 9k10cpu 9k10cpu.elf
KERNDATE=1700000000 mk CONF=k10cpu 9k10cpu
sha256sum 9k10cpu 9k10cpu.elf > /tmp/kerndate-run2
diff /tmp/kerndate-run1 /tmp/kerndate-run2     # EMPTY — identical
```

Captured hashes (identical in run 1 and run 2):

```
dea0cd7a0ca15c3f75262e0b65e41dc5bb9b1fa6d3e8d24830a541265e7abb73  9k10cpu
93c25582ee54a15f8338819a48fdaf7d586f12bed3a27afc073e606593e1a1f7  9k10cpu.elf
```

Negative control (beyond the plan's verify — falsifies the "deterministic anyway" null hypothesis):

```
KERNDATE=1700000001 → fcba8b7a2553dd9a98459f53f8e4adab38ca8d8da7353195fb3e0b3a9bf3b073  9k10cpu
                      ac6033f018b715511bab1abec0ed641c07c2995c61df325294aa45a21ce86ba7  9k10cpu.elf
                      (DIFFERS — the pin is effective)
KERNDATE=1700000000 → dea0cd7a… / 93c25582… (MATCHES run 1/run 2 again)
```

Host-side logs: `/tmp/01-03-kerndate-demo.log` (pair) and `/tmp/01-03-kerndate-control.log` (control + restore); the demo log's recipe echo (`6c -FVw '-DKERNDATE='`{if(~ $#KERNDATE 0) ...} k10cpu.c`) proves the new idiom is what ran.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Bug/correctness] KERNDATE derivation anchored to `$HARVEY`, not caller cwd**
- **Found during:** Task 1 (nix script edits)
- **Issue:** the plan's prescribed text is `$(git log -1 --format=%ct 2>/dev/null || date +%s)`, but both scripts explicitly support `HARVEY != $PWD` (`HARVEY="${HARVEY:-$PWD}"`); invoked from another directory, a bare `git log` would silently pin the *caller's* repo HEAD — or fall back to `date +%s` even though the checkout has git metadata — producing a wrong/non-reproducible timestamp with no error
- **Fix:** wrapped the derivation as `$(cd "$HARVEY" && git log -1 --format=%ct 2>/dev/null || date +%s)` in both scripts; the verify gate's literal substring `git log -1 --format=%ct` is preserved verbatim, and fallback semantics are unchanged (cd failure short-circuits to `date +%s`)
- **Files modified:** nix/build.sh, nix/vm-build.sh
- **Verification:** both grep gates pass; derivation from the checkout returns HEAD's commit timestamp (1793412325 = d88e9572a) when run from the repo root
- **Committed in:** 8ed379f42 (part of task commit)

### Demo harness adaptations (no repo files involved)

**2. Guest namespace harness needed the real build's env discipline (two iterations)**
- **Found during:** Task 2 demonstration setup
- **Issue:** (a) `su -c` resets PATH, so the in-namespace step couldn't find `mk`; (b) after re-sourcing the captured devshell exports, `bootmkfile`'s archive rule failed with `membername: No such file or directory` → empty `libboot.a6` → undefined-symbol link errors (`main: undefined: boot`, `pread: connecttcp: not defined`)
- **Root cause:** `membername` is a repo script (`rc/bin/membername`, `#!/bin/rc`); `nix/build.sh`'s PATH tail (`/rc/bin:.`) supplies it inside the namespace — the devshell env lacks that tail. This is 01-01's conformant-PATH lesson applying to my harness
- **Fix (guest `/tmp` scripts only):** re-source the captured exports after `su` (mirrors `BUILD`'s `. buildnamespace`), append `/rc/bin:.` to PATH, and `rm -f ../boot/libboot.a6` in run 1 to repair the empty archive left by the broken-PATH iteration
- **Verification:** the paired rebuild then ran clean and produced the bit-identical evidence above

---

**Total deviations:** 1 repo-code (Rule 2) + 1 harness-only
**Impact on plan:** No scope creep; the Rule 2 fix strengthens the plan's own must-have ("derived from the git commit") for the HARVEY≠PWD invocation the scripts advertise.

## Issues Encountered

None beyond the harness iterations above. The rc idiom's control flow was A/B-verified against plan9port rc on the host (set → echoed without invoking `date`; unset → `date -n` fallback invoked) before the guest run.

## User Setup Required

None.

## Next Phase Readiness

- **STATE.md blocker resolved:** the KERNDATE bit-reproducibility policy is decided (pinned, git-derived), documented in-repo, and demonstrably honored — Phase 5's clean-checkout reproducibility claims now have their foundation
- 01-04 (minimega smoke harness) boots kernels built under this policy; the two scripts' KERNDATE plumbing means any `nix run .#build` from either host path is already pinned
- The negative-control pattern (different pin must differ, restore must match) is reusable for any future reproducibility claim beyond kernels
- Guest store now has the full devShell realized (including minimega 3.3.0) — 01-04's harness work can use `nix develop` in the guest without a fetch/build wait

---
*Phase: 01-build-tail-green-9k-amd64*
*Completed: 2026-10-31*

## Self-Check: PASSED

- SUMMARY.md exists at .planning/phases/01-build-tail-green-9k-amd64/01-03-SUMMARY.md
- All five plan files exist on disk (4 modified + build/REPRODUCIBILITY.md created)
- Both task commits found in git history (8ed379f42, a1efaf948)
- Task 1 commit touches exactly the four declared files; Task 2 commit touches exactly build/REPRODUCIBILITY.md
- Rebuild-pair evidence captured in /tmp/01-03-kerndate-demo.log + /tmp/01-03-kerndate-control.log (hashes quoted above)
