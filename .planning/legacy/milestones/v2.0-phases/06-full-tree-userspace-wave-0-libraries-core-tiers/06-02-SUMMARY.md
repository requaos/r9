---
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
plan: 02
subsystem: infra
tags: [plan9, mk, allowlist, tiers, aarch64, build-system, grep-hygiene, d-02, d-13, d-14]

requires:
  - phase: 06-full-tree-userspace-wave-0-libraries-core-tiers (06-01)
    provides: gethunk×2 + FPdbleword fixes so the sweep cannot wedge; the amd64-safety gate shape
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: v1.0 subset machinery (subset-build.sh loops promoted to data here), hostbin layout, namespace harness
provides:
  - build/aarch64/tiers/tier0-libs.txt + tier1-cmds.txt — committed append-only allowlist data (13 libs / 24 cmds, v1.0-inherited baselines)
  - objtype-gated mk filter (sys/src/mkfile ALLLIBS/ALLCMDS/ALLSUBSYS; cmd/mkfile ALLTARG/ALLDIRS + checkall on ALLDIRS; container filters in ip/auth/auth/factotum) with COVERAGE_SWEEP=1 disengagement at every level
  - build_aarch64 grown to the filtered sweep (explicit libc bootstrap unchanged)
  - header-only ledgers build/aarch64/markers/tier{0,1}.ledger pinning the D-14 field-order grammar before Phase 10 appends
  - plan9port mk backquote-lexer constraints + the empty-TARG phantom guard (mkmany, cmd/mkfile)
affects: [06-03 (subset scripts evolve to tier-file loops), 06-04 (COVERAGE_SWEEP consumer + checkall foundation), 06-05 (tier-0 22-lib appends), 06-06/06-07 (tier-1 execution gates + appends), 06-08 (CI diff asserts against these files), 10 (tiers 2-4 append here)]

# Actuals (#2632) — chars/4 over the realized diff of the plan's four commits
actuals:
  tokens: 8781
  tasks: 3
  commits: 4   # the plan's own commits; the ledger range below also contains 4 concurrent phase-07 commits

# Plan-commit ledger (#3968) — measured, includes concurrent-session commits in range
commits: 8
plan_head_before: 840eacee3bb7b41b79672b0feb0d36314b281faf
plan_head_after: 8ec9c050e5fd3711ce14b6fa1f1079271b94f192

tech-stack:
  added: []
  patterns:
    - "tier-file-as-data consumed by mk backquote filters: single-line brace-free rc (`if(~ $objtype aarch64 && ! ~ $\"COVERAGE_SWEEP 1) for(...) if(~ $i $KEEP) echo $i; if not echo $ALL...`) with single-quoted awk first-component keep-extraction"
    - "uniform keep-rule: candidate survives iff its name is the first path component after the mkfile's prefix in some tier entry (member / container / dir:target collapse to one extraction; :target consumed by filter-equipped leaves + drivers)"
    - "empty-TARG phantom guard: `PROGS=\`{if(~ $#TARG 0) echo; if not echo $PROGS}` after ${TARG:%=$O.%} (plan9port mk expands an empty list to a phantom `$O.' target)"
    - "COVERAGE_SWEEP=1 disengagement branch in every filter (only 06-04's sweep sets it) — the sweep certifies the whole tree, never the allowlist"

key-files:
  created:
    - build/aarch64/tiers/tier0-libs.txt
    - build/aarch64/tiers/tier1-cmds.txt
    - build/aarch64/markers/tier0.ledger
    - build/aarch64/markers/tier1.ledger
  modified:
    - sys/src/mkfile
    - sys/src/cmd/mkfile
    - sys/src/cmd/mkmany
    - sys/src/cmd/ip/mkfile
    - sys/src/cmd/auth/mkfile
    - sys/src/cmd/auth/factotum/mkfile
    - build/scripts/linux-build
    - build/aarch64/subset-build.sh        # Task 1 (D-02) — prior segment
    - build/aarch64/subset-exec.sh         # Task 1 (D-02)
    - build/aarch64/mkimage.sh             # Task 1 (D-02)
    - build/aarch64/stage-build.sh         # Task 1 (D-02)
    - build/aarch64/vm-run.sh              # Task 1 (D-02)
    - build/aarch64/run-gate.sh            # Task 1 (D-02)
    - build/aarch64/pre-gate-build.sh      # Task 1 (D-02)
    - build/aarch64/donor-vm-build.sh      # Task 1 (D-02)
    - build/aarch64/gethunk-repro-6l.sh    # Task 1 deviation (D-02 class)

key-decisions:
  - "06-02: Task-2 checkpoint resolved planned-layout (user, 2026-11-04) — tier data files, entry forms, filter hooks, COVERAGE_SWEEP disengagement, and ledger grammar (fixed field order tier dir artifact|bin sha256 exec evidence commit date; append-only; last-line-per-dir-wins) are committed exactly as presented"
  - "06-02: uniform keep-rule = first-path-component awk extraction (colon-stripped); dir:target keeps its dir in the mk sweep (mkpaqfs compiles green under 7c) while host-only semantics live at the driver/staging layer; leaf target filtering only where the leaf mkfile is equipped (auth/factotum drops fgui)"
  - "06-02: plan9port mk backquote lexer spiked: literal newlines rejected, first UNQUOTED `}' terminates, `$1`/`$$` are lex errors — the filters are single-line brace-free rc with single-quoted awk programs; mkmany + cmd/mkfile carry empty-TARG phantom guards"
  - "06-02: Task-3 verify gate 2 is defective as written (mk -n never recurses, so ip/ipconfig|auth/factotum can never appear in cmd's own output — count is 0 filtered or not); intent proven by the per-level chain probes; gate text repair queued for 06-03 planning"

patterns-established:
  - "Append-only committed allowlist data under build/aarch64/tiers/ — one entry per line, `#` comments, entries at column 0, growth = append after the entry's gate"
  - "Header-only ledger files pin the D-14 grammar as reviewable data before the first append"
  - "Exclusion transparency: tier-file comment blocks name every intentional sweep exclusion with its reason (USPACE-04 prohibition)"

requirements-completed: [USPACE-04]

coverage:
  - id: D1
    description: "tier data files seeded with the exact v1.0-inherited baselines (13 libs / 24 entries) + comment banners + documented exclusions"
    requirement: USPACE-04
    verification:
      - kind: other
        ref: "grep -cv '^#' build/aarch64/tiers/tier0-libs.txt → 13; tier1-cmds.txt → 24 (blank-line exact)"
        status: pass
      - kind: other
        ref: "host awk keep-extractions: cmd-level 21 names, ip→ipconfig, auth→factotum, factotum-target→factotum"
        status: pass
    human_judgment: false
  - id: D2
    description: "objtype-gated mk filter live with amd64 byte-parity (5 mk -na before/after diffs empty: libs, cmd, ip, auth, factotum) and aarch64 filtered expansion equal to the independent rc keep-rule computation over ls output"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "in-guest probe suite (nsrun/lima): PARITY-OK ×5; libs=13 in ALLLIBS order; cmd dirs=7a 7c 7l auth cc grep ip paqfs rc (on-disk order); singles=12; install loop=13 libs+cmd"
        status: pass
    human_judgment: false
  - id: D3
    description: "COVERAGE_SWEEP=1 disengages every filter branch at container depth; checkall iterates $ALLDIRS unfiltered on every objtype (anti-tautology, Pitfall P6-1)"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "objtype=aarch64 COVERAGE_SWEEP=1 mk -na all: ip expands all 7 DIRS (ftpfs dhcpd httpd ipconfig ppp imap4d snoopy), auth expands lib/factotum/secstore, cmd expands the full 129-dir set == amd64; without the var the filtered sets return"
        status: pass
    human_judgment: false
  - id: D4
    description: "build_aarch64 grown to the filtered sweep; nix run .#build dual-arch green with /aarch64/lib/libc.a, /aarch64/bin/rc, /aarch64/bin/ip/ipconfig, /aarch64/bin/auth/factotum present and zero-failure-marker log scan"
    requirement: USPACE-04
    verification:
      - kind: e2e
        ref: "nix run .#build (lima namespace build): 0 hits on don't-know-how|duplicate|Can't read header|Permission denied|exit status=exit; four artifacts fresh with R_MAGIC 00008c47; amd64/9k10cpu{,.elf} rebuilt"
        status: pass
    human_judgment: false
  - id: D5
    description: "empty/ordering/adjacency edge behavior (USPACE-02): nonexistent-dir tier entry inert without erroring; filter emits candidates in ALLDIRS/ALLTARG on-disk order; prefix adjacency keeps container and member exactly once"
    requirement: USPACE-04
    verification:
      - kind: integration
        ref: "live in-guest mutation probe: tier1 minus cmd/grep plus cmd/nonexistent-dir → dirs loop loses grep, no error, bogus entry inert; order-sensitive equality vs independent computation; cpp not kept by cmd/cp"
        status: pass
    human_judgment: false

duration: 30 min (continuation segment; Task-1 segment ~25 min preceded, plus human-decision checkpoint latency)
completed: 2026-11-05
status: complete
---

# Phase 6 Plan 02: Tier Allowlist Machinery (D-13) + Wave-0 Grep Hygiene Summary

**The append-only tier allowlist machinery is live: two committed data files (13 libs / 24 cmds, v1.0-inherited baselines) drive an objtype-gated mk filter with byte-identical amd64 parity, container-depth COVERAGE_SWEEP disengagement, and a dual-arch `nix run .#build` green through the filtered aarch64 sweep — plus the D-02 host-grep resolution across the harness scripts and the empirical A1=both-run / A6=ok mk-semantics verdicts.**

## Performance

- **Duration:** 30 min (this continuation segment; Task 1 landed in a prior segment, checkpoint latency between)
- **Started:** 2026-11-04T23:39:04Z (continuation); Task 1 committed 2026-11-04T23:24Z
- **Completed:** 2026-11-05T00:09:20Z
- **Tasks:** 3
- **Files modified:** 16 (9 D-02 scripts in Task 1 + 6 build files + 2 tier data files + 2 ledger headers in Task 3; mkmany added by deviation)

## Accomplishments

- **D-02 hygiene (Task 1, prior segment):** all 8 `build/aarch64/*.sh` scripts resolve a host grep once via `command -v` fallback (`GREP=$(command -v /usr/bin/grep || command -v grep)`), 11 call sites rewritten, mkimage.sh's plan9-grep-lacks-`-q` comment preserved and repointed; the A1/A6 spike verdicts recorded in the commit body: **A1=both-run** (recipe-less double definition of a virtual target MERGES — container mkfiles therefore carry their own TARG/DIRS filters) and **A6=ok** (plan9port rc `~` list-membership works). A deviation commit extended the same fix to `gethunk-repro-6l.sh`.
- **Task-2 checkpoint decision: `planned-layout`** (user, 2026-11-04) — implemented exactly: paths, entry forms (`libname` | `cmd/single` | `cmd/dir` | `cmd/container/dir` | `cmd/dir:target` | driver-only `9k/boot`/`stage/name`), seeding (13/24 v1.0 baselines), filter hooks, COVERAGE_SWEEP disengagement, `$HARVEY`-absolute addressing, and the ledger grammar (fixed field order `tier dir artifact|bin sha256 exec evidence commit date`, `#` format header, last-line-per-dir-wins) — the ledgers are committed header-only so the grammar is reviewable data before 06-03's first append.
- **The objtype-gated filter is live and proven:** under `objtype=aarch64`, sys/src sweeps exactly the 13 tier0 libs + `cmd`; cmd sweeps 12 singles + 9 dirs (`7a 7c 7l auth cc grep ip paqfs rc`, on-disk order); ip keeps `ipconfig` only (zero TARG compile lines); auth keeps `factotum` only; factotum's TARG filters to `factotum` (fgui out). Every other objtype is byte-identical to pre-edit — five `mk -na` parity diffs (libs/cmd/ip/auth/factotum) empty.
- **Anti-tautology holds at depth:** `checkall` iterates `$ALLDIRS` unfiltered on every objtype; `COVERAGE_SWEEP=1` under aarch64 restores the full sets (ip → all 7 DIRS, auth → lib/factotum/secstore, cmd → the full 129-dir set == amd64).
- **Edge truths proven live:** a bogus `cmd/nonexistent-dir` entry is inert without erroring (guest mutation probe; removing `cmd/grep` in the same probe proved the live file was being read); emission order is on-disk, independent of tier-file line order; prefix adjacency keeps container and member exactly once (`cmd/cp` does not leak `cmd/cpp`).
- **Dual-arch gate green:** `nix run .#build` (lima namespace build) — zero failure-marker hits (`don't know how|duplicate|Can't read header|Permission denied|exit status=exit`), fresh `/aarch64/lib/libc.a`, `/aarch64/bin/rc`, `/aarch64/bin/ip/ipconfig`, `/aarch64/bin/auth/factotum` (R_MAGIC `00008c47` each), `amd64/9k10cpu{,.elf}` rebuilt; the log shows the filtered dir recursion exactly (`mk 7a 7c 7l auth cc grep ip paqfs rc`, once each).
- **USPACE-04 transparency:** the tier files name every intentional sweep exclusion with its reason (ape, gs, games, nboot, other-arch toolchain letters, the un-gated D-09 build-chain members pending 06-06, stage/init routing) — nothing drops silently.

## Task Commits

1. **Task 1 (deviation):** `d0df2b6f9` — `fix(06-02): resolve host grep in gethunk-repro-6l.sh (D-02 class)`
2. **Task 1:** `877cdae11` — `fix(06-02): resolve host grep via command -v fallback in build/aarch64 scripts (D-02)` (records `A1=both-run A6=ok` in the body)
3. **Task 3:** `e41b64dd5` — `feat(06-02): append-only tier allowlist machinery — data files + objtype-gated mk filter (D-13, USPACE-04)` (records the planned-layout decision, evidence summary, and the gate-2 defect note)
4. **Task 3 follow-through:** `8ec9c050e` — `docs(06-02): document sweep exclusions in tier data (USPACE-04 transparency)` (comment-only; filter output byte-identical before/after)

**Plan metadata:** recorded below (final docs commit).

## Files Created/Modified

- `build/aarch64/tiers/tier0-libs.txt` — new committed data: 13 v1.0-inherited libs with per-line gate provenance
- `build/aarch64/tiers/tier1-cmds.txt` — new committed data: 24-entry baseline (16-subset + boot chain + 7a/7c/7l/cc) with group banners + documented exclusions
- `build/aarch64/markers/tier{0,1}.ledger` — new header-only ledgers pinning the D-14 line grammar
- `sys/src/mkfile` — `ALLLIBS`/`ALLCMDS`/`ALLSUBSYS` renames (35-entry list verbatim) + `LIBS`/`CMDS`/`SUBSYS` objtype-gated backquote filters
- `sys/src/cmd/mkfile` — `ALLTARG`/`ALLDIRS` renames + filters + `checkall` iterates `$ALLDIRS` + empty-TARG PROGS guard
- `sys/src/cmd/mkmany` — empty-TARG phantom guard for `PROGS`/`MANFILES` (shared fix; identity for non-empty TARG)
- `sys/src/cmd/ip/mkfile`, `cmd/auth/mkfile` — container TARG/DIRS filters (per A1=both-run)
- `sys/src/cmd/auth/factotum/mkfile` — `:target`-qualified TARG filter (factotum kept, fgui out)
- `build/scripts/linux-build` — `build_aarch64` = explicit libc bootstrap + `objtype=aarch64 mk libs` + `objtype=aarch64 mk install`; the v1.0 hardcoded loops (named libs, 7? toolchain, cmd subset, factotum per-target) removed with the reason recorded in the comment

## Decisions Made

- **planned-layout confirmed at the Task-2 checkpoint** and implemented exactly (see commit `e41b64dd5` body for the full record).
- **The keep-rule collapsed to one uniform extraction:** candidate survives iff its name is the first path component after the mkfile's prefix in some tier entry — member, container, and colon-stripped dir:target all reduce to one awk program per level. `cmd/paqfs:paqfs` therefore keeps the paqfs dir in the sweep (mkpaqfs compiles green under 7c — probed); the `:target` qualifier's per-target semantics live at the driver/staging layer (06-03's per-target install) and at filter-equipped leaves (factotum). Nothing stages, executes, or marker-records mkpaqfs for aarch64.
- **Filter spelling forced by the plan9port mk backquote lexer (spiked live):** no literal newlines, first unquoted `}` terminates, `$1`/`$$` are lex errors — hence single-line brace-free rc with single-quoted awk extractions. `~ $i $KEEP` is exact-match membership, so `#` comments can never match candidates.
- **Empty-TARG phantom guard** in mkmany and cmd/mkfile: plan9port mk expands `${TARG:%=$O.%}` on an empty TARG to a phantom `$O.` target (empty stem), which lands in `all:V:` prereqs at parse time and has no recipe — the guard recomputes the lists empty-safe and is identity for every existing consumer.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] mkmany + cmd/mkfile empty-TARG phantom guard**
- **Found during:** Task 3 (first in-guest aarch64 probe: `mk: no recipe to make '.7'` in ip and auth — their TARG filters correctly produce empty lists, and plan9port mk's `${TARG:%=$O.%}` turns an empty list into a phantom `7.` target that enters `all:V:` prereqs at parse time)
- **Issue:** the plan's container filters cannot work without the guard; the phantom is a plan9port mk expansion quirk, not plan text
- **Fix:** `PROGS`/`MANFILES` recomputed empty-safe immediately after assignment (`{if(~ $#TARG 0) echo; if not echo $PROGS}`) — identical value for non-empty TARG, so amd64 parity and all existing mkmany consumers are untouched (5/5 parity diffs re-proven after the guard landed)
- **Files modified:** sys/src/cmd/mkmany, sys/src/cmd/mkfile (both absent from the plan's files_modified; required by the planned container filters)
- **Verification:** toy-mk reproduction (empty vs non-empty), then the in-guest probes: ip/auth recurse correctly with empty TARG
- **Committed in:** e41b64dd5

**2. [Rule 1 - Bug] filter spelling redesigned around the plan9port mk backquote lexer**
- **Found during:** Task 3 implementation spikes
- **Issue:** the PATTERNS/RESEARCH skeletons use multi-line braced rc inside backquotes; plan9port mk rejects literal newlines in backquotes and terminates at the first unquoted `}` (the v1.0 261104-b4p fix landed only in the in-tree mk; the namespace build runs plan9port mk). `$1`/`$$` are lex errors, so awk field references are impossible inline
- **Fix:** keep-extraction as a separate single-quoted awk program (first-component rule, no `$N` fields — regex/sub on `$0`), filter as a single-line brace-free rc command with the plan's exact engagement condition (`if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) ... ; if not echo $ALL...` — `; if not` proven valid rc). RESEARCH explicitly left the final rc spelling to executor discretion ("the constraints are verified mechanics"); all constraints (backquote timing, `$HARVEY` paths, Plan 9 grep dialect avoided entirely, amd64 passthrough, comment safety) hold
- **Files modified:** the five filter mkfiles
- **Verification:** five toy-mk spike rounds + the full in-guest probe suite (PROBE-ALL-OK)
- **Committed in:** e41b64dd5

**3. [Rule 2 - Missing Critical] header-only ledger files committed**
- **Found during:** Task 3 (the checkpoint-locked layout includes the ledgers' field order + header + supersede semantics, but the plan's files_modified omitted the files)
- **Issue:** D-14's grammar becomes a consumed contract (Phase 10 appends; CI diffs) — pinning it as committed data now is the one-way-door's whole point
- **Fix:** created `build/aarch64/markers/tier{0,1}.ledger` with exactly the locked `#` header (fixed field order, append-only, last-line-per-dir-wins); no marker lines (evidence begins with 06-03's gates)
- **Verification:** content matches the checkpoint text verbatim; comment-only files are inert to all consumers
- **Committed in:** e41b64dd5

---

**Total deviations:** 3 auto-fixed (2 Rule 1 bugs, 1 Rule 2 missing-critical)
**Impact on plan:** the deviations are the machinery making the planned design actually work under the real toolchain + the layout contract pinned as data. No scope creep; amd64 leg byte-identical (parity-proven after every change).

## Plan-Gate Defect Recorded (not a machinery failure)

Task 3's second verify gate greps `ip/ipconfig|auth/factotum` in `sys/src/cmd`'s own `mk -n all` output — but `mk -n` prints recipes without recursing into subdir mks, so those strings can never appear there (measured: count 0 in both filtered and unfiltered runs; the unfiltered run proves the count carries no filter signal). The gate's **intent** — nested entries survive the filter — is proven by the per-level chain probes: cmd keeps `ip`+`auth`, ip keeps `ipconfig`, auth keeps `factotum`, factotum keeps the `factotum` target. Repair of the gate text is queued for 06-03 planning (a correct form: run the per-level probes, as this SUMMARY's evidence does).

## Issues Encountered

- **Concurrent-session VM churn:** a phase-07 session actively used the same lima guest during this segment — the guest checkout's `.build` (including probe scratch and the toolchain) was wiped/rebuilt twice mid-segment. Resolution: probe state moved to the guest's `/tmp` (namespace-visible, outside the rsync'd tree), before-baselines kept host-side, and all probes made state-independent (`mk -na`, so stale `.7` objects in the guest tree cannot suppress compile lines). No content lost; every proof re-ran green afterward.
- **Probe harness lessons (recorded for 06-03+):** the namespace's toolchain prints as the caret form (`7^c`), not `7c`; plan9port awk rejects regex literals containing `[`+paren classes (`[(]`) — use `index`/`substr` for extraction; a scp filename mismatch (`expected.rc` vs `06-02-expected.rc`) masqueraded as rc source failures ("`.: can't open"`) — the error names the failing builtin, not the missing file.
- **`nix run .#build` exit status** remained 0 throughout per deferred item 02-12 — the gate is the artifact set + zero-marker scan, as designed.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **06-03 unblocked:** the tier-file loops replace subset-build.sh/subset-exec.sh's hardcoded lists in place; the empty-edge driver behavior ("fails loudly naming the entry") lands with that driver evolution (06-03 scope — the mk-filter half is proven inert-here).
- **06-04 unblocked:** `COVERAGE_SWEEP=1` disengagement is live at every level (ip/auth/cmd proven); `checkall` iterates `$ALLDIRS`; the sweep certifies the whole tree.
- **06-05/06-06/06-07:** append lines to the committed tier files as their per-dir gates pass; the ledger grammar awaits their first marker lines.
- **Watch item for 06-03 planning:** repair the Task-3 gate-2 text (see Plan-Gate Defect above); consider carrying the probe suite (`/tmp/06-02-probe.sh` shape) as a committed harness if the per-level probes are wanted in CI (06-08).

## Self-Check: PASSED

- [x] `build/aarch64/tiers/tier0-libs.txt` exists (13 entries, `grep -cv '^#'` == 13)
- [x] `build/aarch64/tiers/tier1-cmds.txt` exists (24 entries, `grep -cv '^#'` == 24)
- [x] `build/aarch64/markers/tier{0,1}.ledger` exist with the locked `#` header grammar
- [x] `sys/src/mkfile` contains `ALLLIBS=` (35-entry verbatim ✓ diff-checked), `ALLCMDS`, `ALLSUBSYS`, three filter branches
- [x] `sys/src/cmd/mkfile` contains `ALLTARG=`/`ALLDIRS=` renames, filter branches, `for(i in cc $ALLDIRS)` in checkall
- [x] Container mkfiles (ip, auth, auth/factotum) carry the filters; mkmany + cmd/mkfile carry the empty-TARG guard
- [x] `build/scripts/linux-build` `build_aarch64` = libc bootstrap + `objtype=aarch64 mk libs` + `objtype=aarch64 mk install`
- [x] No root-absolute `/build/` paths in the five mkfiles (gate-4 grep clean)
- [x] Commits found on master: d0df2b6f9, 877cdae11, e41b64dd5, 8ec9c050e
- [x] amd64 parity diffs empty ×5 (libs, cmd, ip, auth, factotum); evidence bundle host-side at `.build/06-probes/06-02-probes/`
- [x] Filtered-listing == independent expected set; COVERAGE_SWEEP probes green; empty/ordering/adjacency edges proven
- [x] `nix run .#build` green: 4 artifacts fresh, R_MAGIC 00008c47, zero failure markers, amd64 kernel rebuilt
- [x] No pre-existing dirty-tree files staged in any commit (staged sets were explicit path lists throughout)

---
*Phase: 06-full-tree-userspace-wave-0-libraries-core-tiers*
*Completed: 2026-11-05*
