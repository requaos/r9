---
phase: 261104-cms
plan: 01
subsystem: infra
tags: [comment-hygiene, citations, r9, aarch64, mk, expect, nix, devshell]

requires:
  - phase: v1.0 milestone (all archived phases + quick tasks)
    provides: the green gate batteries this sweep re-ran unchanged
provides:
  - r9 source tree free of planning-artifact citations (one docs commit on main)
  - harvey source tree free of planning-artifact citations (one docs commit on master)
  - proof that both gate batteries are citation-independent
affects: []

actuals:
  tokens: 126000    # chars/4 over both realized diffs (378411 + 126435 chars)
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Comments carry the technical WHY; planning IDs die. Provenance that stays: donor SHAs, 9k file lineage, frozen-ABI paths, test-file paths."
    - "Diff-invariant proof: string-protecting comment stripper + blank-squeeze + cmp per file; residuals word-diff-audited."

key-files:
  created: []
  modified:
    - "../r9/aarch64/src/syscall.rs, ../r9/aarch64/src/proc.rs, ../r9/port/src/devmnt.rs, ../r9/xtask/src/main.rs (the r9 density outliers)"
    - "build/aarch64/subset-build.sh, build/aarch64/mkimage.sh, nix/boot-r9.sh (the harvey header outliers, compressed to the cap)"
    - "sys/src/cmd/mk/lex.c (the bquote comment: full WHY kept, quick-ID/phase citations dropped, 9front donor pin kept)"
    - "sys/src/9k/boot/bootr9.c, sys/src/9k/boot/mkfile.aarch64, sys/src/cmd/mkfile, build/harness/src/*.rs (6), flake.nix, .github/workflows/r9-shell.yml"

key-decisions:
  - "String-literal citations (errstr/println/clap-help/echo/push_str artifact bytes) are CODE under the comment-only invariant — they stay, recorded as residuals below. The alternative (editing user-visible strings to satisfy gate 1) would trip gate 2's code-token delta, the worse violation."
  - "Gate-4 substitution: `virt` boot-tested in place of `virtshell` (see Deviations) — the milestone's proven battery shape."

patterns-established: []

requirements-completed: ["261104-cms"]

coverage:
  - id: D1
    description: "r9 sweep: zero comment citations in swept roots; diff comment-only; port tests 80/80; xtask check/clippy/test green; five config builds green; five boot profiles green (virt substituted for virtshell — see Deviations)"
    requirement: "261104-cms"
    verification:
      - kind: integration
        ref: "cargo test -p port (80/80) + cargo xtask check/clippy/test + xtask build ×5 + xtask boot-test ×5 (in-repo gates)"
        status: pass
    human_judgment: false
  - id: D2
    description: "harvey sweep: zero comment citations in swept set; diff comment-only; toolchain app rebuild (7a/7c/7l) green; mk namespace build green; harness cargo check green; 19/19 touched .sh shellcheck-equivalent to HEAD; flake metadata + workflow YAML parse green"
    requirement: "261104-cms"
    verification:
      - kind: integration
        ref: "nix run .#toolchain + vm-run.sh mk build + cargo check (harness) + shellcheck -f gcc diffs + nix flake metadata + yaml.safe_load"
        status: pass
    human_judgment: false
  - id: D3
    description: "Exactly one comment-only commit per repo, nothing pushed; harvey commit path-scoped so the pending .planning archive deletions never rode in"
    requirement: "261104-cms"
    verification:
      - kind: other
        ref: "git show --name-only HEAD (both repos) — exact swept-set match; git status — .planning deletions + troff/postscript mods uncommitted"
        status: pass
    human_judgment: false

duration: 2h 15m
completed: 2026-11-04
status: complete
---

# Phase 261104-cms Plan 01: Comment hygiene sweep across harvey + r9 Summary

**103 source files swept of planning-artifact citations across both repos (r9: 56 files, ~830 hits; harvey: 47 files, ~345 hits) — every diff proven comment-only by the stripper invariant, both gate batteries green, one docs commit per repo, nothing pushed.**

## Performance

- **Duration:** ~2h 15m
- **Completed:** 2026-11-04 (15:49Z)
- **Tasks:** 2/2
- **Files modified:** 56 (r9) + 47 (harvey)

## Accomplishments

- r9: all 56 citation-carrying files restyled (`aarch64/src/**`, `port/src/**`, `xtask/src/**`, workspace + member Cargo.tomls, `aarch64/lib/config_*.toml`): narrative module headers compressed (devmnt.rs's 68-line header → the cap; syscall.rs/proc.rs/main.rs/trap.rs/exec.rs headers carry purpose + invariants + at most one real contract pointer); inline citation tails restyled to the plain technical reason; SAFETY justifications preserved verbatim minus citation tails (the workspace `undocumented_unsafe_blocks=deny` lint stays green).
- harvey: all 47 mapped files restyled (build/aarch64 ×29, build/harness/src ×6, nix ×5, flake.nix, the r9-shell workflow, build/mm/boot-9k10cpu.sh, sys/src/9k/boot/{bootr9.c,mkfile.aarch64}, sys/src/cmd/{mk/lex.c,mkfile}): subset-build.sh/mkimage.sh/boot-r9.sh headers compressed to the cap (platform-split constraint + out/-ownership rule + the kernel-via-store-path contract kept); mk/lex.c's bquote comment keeps the full WHY (brace-depth counter ported from the 9front donor 350266104f; the naive first-close-brace scan corrupted the parse) minus quick-ID/phase/deferred-item citations; the 7a/lex.c //-prefixed commented-out LSPREG table rows never touched; shellcheck directives preserved verbatim.
- Both diffs are comment-only BY PROOF: the per-file strip-pre/strip-post comparison is byte-identical everywhere (two harvey .expect files flagged by the whole-line-# stripper for restyled TRAILING `#` tails — word-diff-audited: every code prefix byte-identical, zero code-token deltas).

## Task Commits

1. **Task 1: r9 comment sweep + full r9 gate battery** — `aa2ef2c` (docs, r9 main) — 56 files, +1101/−1186
2. **Task 2: harvey comment sweep + host gates** — `2ce5c5adb` (docs, harvey master) — 47 files, +371/−431

## Gate evidence

r9 (all via the r9 devshell, R9_HARVEY_DIR=/Users/neilskinner/code/harvey):
- gate 1 (citation grep over swept roots): zero comment hits; residuals listed below.
- gate 2 (diff-invariant, RSTRIP/HASHSTRIP + squeeze + cmp per file): zero residual deltas.
- gate 3: `cargo test -p port` 80/80; `cargo xtask check`/`clippy`/`test` green; builds green for virtshell, virtexec, virtpanic, default, raspi4b (~2s each, warm).
- gate 4 (boot-test, `--arch aarch64`): virtpanic PASS, virtexec PASS, virtmulti PASS, virtnotes PASS, virt PASS (~1–3 min each under TCG). virtshell — see Deviations.

harvey:
- gate 1: zero comment hits in the swept set; residuals listed below.
- gate 2: comment-only everywhere; the two .expect residuals audited (above).
- gate 3: `nix run .#toolchain` green (7a/7c/7l + aux rebuilt into .build/hostbin-darwin-aarch64); mk namespace build green (`vm-run.sh 'cd cmd/mk && mk nuke && mk'` — see Deviations for the cwd adjustment; lex.6 compiled and 6l linked clean); harness `cargo check` green.
- gate 4: 19/19 touched .sh files shellcheck-equivalent to HEAD (identical warning sets); `nix flake metadata` OK; r9-shell.yml YAML-parses.

## Residual grep hits (all recorded per the plan's coincidental/non-citation allowance)

Every residual is either a STRING LITERAL (code under the comment-only invariant — editing it would trip gate 2) or a coincidental technical term that is not a citation:

r9:
- `aarch64/src/syscall.rs:755` — errstr string `"mount: auth fd unsupported this era (D-02)"` (user-visible error text; code).
- `aarch64/src/main.rs:377` — println string `"(D-12 debug config)"` (serial-log output text; code).
- `aarch64/src/platform/virt/gic.rs:1` — `GIC-400` (ARM hardware name; coincidental substring match, not a citation).
- `xtask/src/main.rs:763` — `INTID-33` (ARM GIC interrupt-ID term in a doc comment; coincidental, not a citation).
- `xtask/src/main.rs` 219, 221, 227, 237, 239, 246–247, 253–254 — clap `--help`/`.about()` strings (CLI surface text; code).
- `xtask/src/main.rs` 1367, 1372, 1377 — boot-test PASS println strings (harness output text; code).
- `xtask/src/main.rs` 1745, 1765 — format! error strings (code).
- `xtask/src/uregcheck.rs` 59, 94 — Err() strings (code).

harvey:
- `build/aarch64/subset-build.sh` 30–32, 144, 159; `mkimage.sh` 45, 139; `run-gate.sh` 94, 446–447; `donor-vm-build.sh` 150–151; `donor-ns-build.sh` 132, 410; `pre-gate-build.sh` 68; `build/mm/boot-9k10cpu.sh` 56 — err()/echo string literals (script error/log output; code).
- `build/harness/src/trace.rs` 97, 119, 338–342, 349, 352, 354, 436–438, 1261, 1278, 1281, 1417, 1441, 1876, 1915, 2213 — push_str/format!/eprintln!/assert strings, incl. the trace-artifact header bytes whose byte-shape the harness's own tests pin (code); `trace.rs:489,491` — `.planning/phases/...` artifact filesystem paths (real paths the harness reads/writes; code).

## Deviations from Plan

**1. [Rule 4-shaped, surfaced not fixed] gate-4 virtshell → virt substitution**
- **Found during:** Task 1 gate 4.
- **Issue:** the plan's boot-test loop names `virtshell`, but `cargo xtask boot-test --config virtshell` panics deterministically at exec_bootstrap (`dev_exec: config carries no initrd image`): config_virtshell.toml carries NO [qemu] initrd key BY DESIGN (the shell image is baked and delivered Harvey-side via run-r9.sh, never by xtask). This fails identically at HEAD — the sweep is comment-only (gate 2), so it cannot be the cause. The milestone's own green battery (`.planning/milestones/v1.0-quick/261104-b4p-*/261104-b4p-PLAN.md:193`) never boot-tested virtshell: it ran `boot-test` over `virt virtexec virtmulti virtnotes virtpanic` and gave virtshell `build` + `clippy` only.
- **Fix:** ran the milestone's proven shape: `boot-test --config virt` (PASS) in the virtshell slot; virtshell covered by the gate-3 build + clippy legs (both green). No code edited to make a gate pass (the plan's hard rule).
- **Files modified:** none.
- **Verification:** all five boot-testable profiles PASS; virtshell build/clippy green.

**2. [Rule 3 - Blocking] vm-run.sh invocation cwd**
- **Found during:** Task 2 gate 3.
- **Issue:** the plan's literal `vm-run.sh 'cd sys/src/cmd/mk && ...'` fails: nsrun.sh lands the wrapped command at `$HARVEY/sys/src`, so `sys/src/cmd/mk` does not exist from there.
- **Fix:** invoked as `vm-run.sh 'cd cmd/mk && mk nuke && mk'` (the namespace's landing cwd).
- **Files modified:** none.
- **Verification:** mk nuked + rebuilt clean in the guest namespace (lex.6 compiled; 6.out linked).

**3. [Rule 1 - process] harvey commit re-homed path-scoped**
- **Found during:** Task 2 commit.
- **Issue:** the first commit attempt (`git add --pathspec-from-file` + bare `git commit -m`) swept the ONE pre-staged `.planning/REQUIREMENTS.md` archive deletion into the commit (48 files) — the plan's explicit prohibition.
- **Fix:** `git reset --soft HEAD~1` + `git commit --pathspec-from-file=<the 47 swept paths>`; verified `git show --name-only HEAD` == the swept set exactly (47 files, zero .planning entries, zero deletions); the archive deletions (162 working-tree + 1 staged) remain uncommitted exactly as pre-session.
- **Committed as:** `2ce5c5adb` (the final, correct commit).

**4. [Scope boundary] pre-existing unrelated working-tree state, never swept**
- `sys/lib/postscript/troff/HX`, `sys/lib/troff/font/devutf/{HB,HI,HX}`, `sys/lib/troff/font/devutf/charlib/{LH,RC,lH,rH}`, `sys/src/cmd/postscript/devpost.add/C{1,2,3}` carry pre-existing working-tree modifications (font-metric content — e.g. HX: `Helvetica-BoldOblique` → `Helvetica-Narrow-BoldOblique`). They predate this session, are outside the plan's file set, and were never edited by me; the path-scoped commit excludes them.
- The toolchain gate build left untracked binaries at `sys/src/cmd/{6a,6c,6l,8a,8c,8l,cpp}/…`, `sys/src/cmd/aux/*` etc. (BUILDTOOLCHAIN intermediates; not gitignored). Generated gate output — left untracked, excluded from the commit. Candidate for a future .gitignore pass; not this plan's scope.

**Impact on plan:** No scope creep; every deviation is either a documented substitution against the milestone's proven gate shape, an invocation correction, or a commit-hygiene repair verified exact.

## Issues Encountered

- One self-caught mistake mid-sweep: two shell `echo`/`err` string literals in donor-ns-build.sh were briefly edited before being reverted in-place (strings are code; gate 2 would have flagged them). Both verified reverted by the clean gate-2 run.
- The concurrent `nix run .#toolchain` + `vm-run.sh` runs raced (rsync walked the tree while the toolchain's temp objects churned `sys/src/cmd/unix/9cc/lib9/`): the first mk build died on a vanished `.tmp` file; re-ran serially, green. Lesson (recorded): never run the toolchain build concurrently with vm-run.sh's rsync.

## Known Stubs

None — no empty-value stubs, placeholders, or skipped tests introduced; every `<verify>` gate ran (with the one documented substitution).

## Threat Flags

None — no new network endpoints, auth paths, file-access patterns, or schema changes; the sweep is comment-only by construction.

## Next Phase Readiness

- Both trees explain themselves without planning-artifact references; the gates prove the citations were never load-bearing.
- The virtshell boot-test gap (Harvey-side initrd delivery vs `xtask boot-test`) is a real, pre-existing structural limitation worth a future quick task if virtshell should ever boot under xtask directly.

---
*Phase: 261104-cms*
*Completed: 2026-11-04*

## Self-Check: PASSED

- `aa2ef2c` found in r9 (`git log --oneline --all`).
- `2ce5c5adb` found in harvey (`git log --oneline --all`).
- `261104-cms-SUMMARY.md` exists at the plan's output path.
- Commit contents verified: r9 = exactly the 56 swept files; harvey = exactly the 47 swept files (`git show --name-only HEAD` == the swept set; zero `.planning` entries, zero deletions).
