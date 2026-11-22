---
phase: 261104-cms
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - "../r9/aarch64/src/ (all .rs/.S incl. platform/**)"
  - "../r9/port/src/ (all .rs)"
  - "../r9/xtask/src/ (config.rs, main.rs, uregcheck.rs)"
  - "../r9/Cargo.toml (5 citation lines — planner-found scope addition, same class as the listed member Cargo.tomls)"
  - "../r9/aarch64/Cargo.toml"
  - "../r9/port/Cargo.toml"
  - "../r9/aarch64/lib/config_*.toml"
  - "build/aarch64/ (swept .sh/.c/.expect per the mapping grep; never *.md, *.log, out/, smoke-out/, __pycache__/, *.paq)"
  - "build/harness/src/ (the 6 of 8 .rs files carrying citations)"
  - "nix/ (swept .sh/.yaml per the mapping grep)"
  - "flake.nix"
  - ".github/workflows/r9-shell.yml"
  - "build/mm/boot-9k10cpu.sh"
  - "sys/src/9k/boot/bootr9.c, sys/src/9k/boot/mkfile.aarch64"
  - "sys/src/cmd/7a/lex.c, sys/src/cmd/7c/swt.c, sys/src/cmd/7l/unixcompat.c, sys/src/cmd/mk/lex.c"
  - "sys/src/cmd/mkfile"
autonomous: true
requirements: ["261104-cms"]
estimate:
  tokens: 60000
  raw_tokens: 60000
  tasks: 2
  confidence: low
must_haves:
  truths:
    - "Every swept file's diff is comment-only: strip-comments pre/post equality holds per repo (code byte-identical modulo comments and blank-line residue)"
    - "Zero planning-artifact citations remain in the swept path sets, or each residual hit is recorded in the SUMMARY as coincidental/non-citation"
    - "r9 gate battery green post-sweep: port host tests 80/80, xtask check/clippy/test, five config builds, five boot-test profiles"
    - "harvey gates green post-sweep: host toolchain rebuild (7a/7c/7l), mk namespace build, harness cargo check, shellcheck-equivalence on touched .sh, flake metadata parse, workflow YAML parse"
    - "Exactly one new commit per repo, scope docs(261104-cms), comment-only, nothing pushed"
  artifacts:
    - "one commit on r9 main containing the full r9 sweep"
    - "one commit on harvey master containing the full harvey sweep (path-scoped; the pending staged .planning archive deletions are NOT swept in)"
    - ".planning/quick/261104-cms-comment-hygiene-sweep-across-harvey-r9-r/261104-cms-01-SUMMARY.md"
  key_links:
    - "SAFETY comments preserved on every unsafe block (r9 workspace lints deny undocumented unsafe — a dropped justification is a build failure)"
    - "shellcheck directive comments preserved verbatim in every touched .sh (warning sets unchanged vs HEAD)"
    - "commented-out table rows in sys/src/cmd/7a/lex.c (the //-prefixed LSPREG entries) never uncommented, never deleted"
    - "restyled comments carry the technical reason with zero citation residue (no '(was X)' tails, no plan-ID parentheticals)"
---

<objective>
Sweep planning-artifact citations out of source comments across both repos and compress long narrative header blocks to their discipline caps — the technical WHY stays, the citation dies. Comment-only: zero code edits. r9 (~962 citation lines across ~46 files: aarch64/src, port/src, xtask/src, both member Cargo.tomls, the workspace Cargo.toml, aarch64/lib/config_*.toml) then harvey (~230 lines across the mapped set under build/, nix/, flake.nix, the r9-shell workflow, and four C sources). Each repo lands one comment-only commit.

Purpose: the source tree should explain itself without referencing planning artifacts that are invisible to a reader of the code (and that churn); citations rot, reasons don't.
Output: one commit on r9 main, one commit on harvey master, plus this task's SUMMARY.
</objective>

<execution_context>
@/Users/neilskinner/.config/opencode/gsd-core/workflows/execute-plan.md
@/Users/neilskinner/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@/Users/neilskinner/code/harvey/AGENTS.md

Representative heavy files already read during planning (do not re-read in full; work from grep hits + header windows):
- r9: aarch64/src/usermem.rs (module header carrying quick-ID + threat-tag citations), aarch64/src/proc.rs (~34 citation lines), aarch64/src/trap.rs + trap.S (inline citation tails, `//` comment style in .S), port/src/devmnt.rs (~68-line narrative module header — compress to the cap), xtask/src/uregcheck.rs (contains `/*` INSIDE string literals — the diff-invariant stripper must protect strings)
- harvey: build/aarch64/subset-build.sh + mkimage.sh + nix/boot-r9.sh (heavy `#` headers), sys/src/cmd/mk/lex.c (the bquote shim comment — project-added comment in vendored-file lineage; keep the WHY: brace-nesting depth counter ported from the 9front donor, the naive first-close-brace scan ended early and corrupted the mkfile parse)

Hard exclusions: vendored/donor/upstream trees (sys/src/cmd/gs/, spin/, aux/antiword/, games/, sys/src/9/pc*, sys/src/9/pcboot/), any file whose only hits are coincidental upstream text, .planning/, .build/, target/, result, graphify-out/, and ALL *.md documents (they are documents, not comments).
</context>

<tasks>

<task type="auto">
  <name>Task 1: r9 comment sweep + full r9 gate battery</name>
  <files>../r9/aarch64/src/**, ../r9/port/src/**, ../r9/xtask/src/**, ../r9/Cargo.toml, ../r9/aarch64/Cargo.toml, ../r9/port/Cargo.toml, ../r9/aarch64/lib/config_*.toml</files>
  <precondition>r9 checkout is clean on main at fdeba94 or later; qemu-system-aarch64, rustup aarch64 target, and the devshell toolchain are present (the milestone's green gates used them); R9_HARVEY_DIR unset resolves to ../harvey, whose build/aarch64/out staged binaries exist from the milestone's green runs (boot-test's auto-bake regenerates if stale).</precondition>
  <action>
  Work in /Users/neilskinner/code/r9 (cargo/xtask commands run with that cwd; git via git -C). Derive the exact file list by grepping the declared roots (aarch64/src, port/src, xtask/src, aarch64/lib, both member Cargo.tomls, the workspace Cargo.toml) with the citation pattern defined in this task's <verify> block — expect ~46 files. Do not re-scan outside these roots.

  Per file, read only the top-of-file header block (~first 40 lines) and tight windows (+/-4 lines) around each grep hit; whole-file reads only when a header rewrite needs more context. Apply three transforms:

  1. Module headers (//! and // header blocks): rewrite to the ~8-line cap — one purpose sentence, the invariants that matter, and at most ONE contract pointer to a real artifact (e.g. the frozen ureg.h ABI path, a donor-source lineage note like "semantics ported from 9k's devmnt.c", a test-file path pointer). Delete narrative history, gate tallies, and landed-in-wave framing. devmnt.rs's ~68-line header is the size outlier; usermem.rs/proc.rs/trap.S carry the citation-density outliers.
  2. Inline comments: keep any comment that explains a non-obvious WHY; restyle citation tails into the plain technical reason (the citation dies, the reason stays — e.g. a comment citing the cold-buffer plan class becomes "cold user buffers are forced resident before dereference"; a harness-hygiene decision tag becomes "fresh serial log per run; qemu killed by process group"). Delete comments whose ENTIRE content is a citation with no technical content. Restyled text must not retain the citation in any form — no "(was X)" tails.
  3. TOML config/Cargo comments (# style): same restyle-or-delete discipline.

  HARD rules (violations = revert the file and redo):
  - NEVER remove or gut a SAFETY comment attached to an unsafe block — the workspace denies undocumented unsafe blocks, so the justification must remain complete; only its citation tail dies.
  - Keep donor/upstream provenance that is technical content: 9front pin SHAs, "ported from 9k <file>" lineage, frozen-ABI file paths, test-file paths. Kill planning-artifact references only: plan/phase/quick IDs, decision/threat/review/requirement tags, pitfall/open-question labels, and references to planning-tree documents or docs that live outside the code.
  - Style: Rust keeps the file's existing // vs //! idiom and lowercase-start convention; .S files keep // comments; tabs preserved everywhere.
  - Comment-only. If a gate fails, a code edit crept in: git checkout -- the implicated file(s) and redo that file. NEVER fix code to make a gate pass.

  Then run the gate battery in <verify> order. Commit once on main: git -C /Users/neilskinner/code/r9 add <the swept paths> && git -C /Users/neilskinner/code/r9 commit -m "docs(261104-cms): comment hygiene sweep — drop planning-artifact citations, compress narrative headers". No push.
  </action>
  <verify>
    <automated>
    cd /Users/neilskinner/code/r9

    # gate 1 — zero citations in swept paths (residuals must be zero, or each recorded in the SUMMARY as coincidental):
    grep -rnI -E '2611[0-9]{2}-[a-z0-9]{3}|0[1-5]-[0-9]{2}|[A-Z]{1,5}-[0-9]{2}|T-[a-z0-9]+-[0-9]{2}|SC#[0-9]|OQ-[0-9]|Pitfall [0-9]|\.planning/|[0-9]{2}-(SUMMARY|RESEARCH|CONTEXT|PATTERNS|VERIFICATION|EVIDENCE|REVIEW)|(SUMMARY|RESEARCH|CONTEXT)\.md|deferred-items|UNSAFE-AUDIT|HowItWorks\.md|Phase [0-9]|phase-0[0-9]|plan 0[0-9]-[0-9]{2}|wave [0-9]' aarch64/src aarch64/lib/config_*.toml aarch64/Cargo.toml port/src port/Cargo.toml xtask/src Cargo.toml

    # gate 2 — diff-invariant (comment-only proof). Strippers verified against the swept corpus:
    #   RSTRIP protects double-quoted strings first (uregcheck.rs has /* inside string literals);
    #   HASHSTRIP strips whole-line # comments only (shell/nix/toml have no block comments here).
    TMPD=$(mktemp -d)
    RSTRIP='s{("(?:\\.|[^"\\])*")|//[^\n]*|/\*.*?\*/}{defined $1 ? $1 : ""}gse'
    HASHSTRIP='s{^[ \t]*#[^\n]*$}{}gm'
    SQ='s{[ \t]+\n}{\n}g; 1 while s{\n[ \t]*\n}{\n}g; s{\A[ \t\n]+}{}; s{[ \t\n]+\z}{}'
    for f in $(git diff --name-only); do
      case "$f" in
        *.rs|*.S) STRIP="$RSTRIP" ;;
        *.toml)   STRIP="$HASHSTRIP" ;;
        *)        echo "UNEXPECTED FILE TYPE: $f — extend the case or revert"; continue ;;
      esac
      git show HEAD:"$f" | perl -0777 -pe "$STRIP; $SQ" > "$TMPD/pre"
      perl -0777 -pe "$STRIP; $SQ" "$f" > "$TMPD/post"
      cmp -s "$TMPD/pre" "$TMPD/post" || {
        echo "RESIDUAL DELTA: $f — audit: git diff --word-diff=porcelain -- $f"
        echo "  any code-token delta => git checkout -- $f and redo; comment-only delta => record in SUMMARY and proceed"
      }
    done; rm -rf "$TMPD"

    # gate 3 — host + build battery (any failure: revert the implicated file, redo):
    cargo test -p port            # 80/80
    cargo xtask check --verbose
    cargo xtask clippy --verbose
    cargo xtask test --verbose
    for c in virtshell virtexec virtpanic default raspi4b; do cargo xtask build --arch aarch64 --config $c; done

    # gate 4 — five boot profiles (arch flag required: boot-test defaults to x86-64 and errors otherwise):
    for c in virtpanic virtshell virtexec virtmulti virtnotes; do cargo xtask boot-test --arch aarch64 --config $c; done
    </automated>
  </verify>
  <done>Gates 1/3/4 green; gate 2 shows no unaudited residuals (any flagged file word-diff-audited: code-token deltas reverted-and-redone, comment-only residuals recorded in the SUMMARY); git show --stat HEAD in r9 lists only swept comment files; one commit on main, nothing pushed.</done>
</task>

<task type="auto">
  <name>Task 2: harvey comment sweep + host gates</name>
  <files>build/aarch64/* (swept sources), build/harness/src/*.rs, nix/*, flake.nix, .github/workflows/r9-shell.yml, build/mm/boot-9k10cpu.sh, sys/src/9k/boot/{bootr9.c,mkfile.aarch64}, sys/src/cmd/{7a/lex.c,7c/swt.c,7l/unixcompat.c,mk/lex.c}, sys/src/cmd/mkfile</files>
  <precondition>nix devshell functional on this host (flakes enabled; the milestone's toolchain/build apps ran green); on darwin the lima "harvey" VM is available for the mk namespace build (on linux, build/aarch64/nsrun.sh --wrap runs natively instead).</precondition>
  <action>
  Work in /Users/neilskinner/code/harvey. Derive the exact file list by grepping the mapped roots (build/aarch64, build/harness/src, nix, flake.nix, .github/workflows/r9-shell.yml, build/mm, sys/src/9k/boot, the four named C sources, sys/src/cmd/mkfile) with the citation pattern defined in this task's <verify> block, applying the hard exclusions (no *.md, no logs/binaries/out trees, no vendored/donor files, no coincidental-upstream-text files). Expect ~25 files, ~230 lines.

  Same three transforms as Task 1, tuned per file class:
  1. C/sh header blocks: compress to the ~10-line cap — purpose + usage + the one non-obvious constraint. subset-build.sh, mkimage.sh, and nix/boot-r9.sh are the header outliers: keep the platform-split constraint (darwin drives lima, linux runs natively under unshare), the out/-ownership rule, and the kernel-via-store-path contract; drop step-numbered narrative that restates the pipeline and all citation framing.
  2. Inline citations: restyle-or-delete per Task 1's rule (reason stays, citation dies, no "(was X)" residue).
  3. The mk/lex.c bquote comment keeps its full technical WHY (brace-nesting depth counter ported from the 9front donor; the naive terminator scan ended early and corrupted the parse; rc-style nesting increments on the open brace) minus the quick-ID/phase/deferred-item citations — donor provenance stays.

  HARD rules (violations = revert the file and redo):
  - Plan 9 C style: /* */ block comments only, tabs, lowercase-start per file convention; never introduce // line comments into C (the pre-existing //-prefixed commented-out LSPREG table rows in 7a/lex.c are commented-out CODE — leave them exactly as-is).
  - Shell/nix/yml/mkfile edits touch whole-line # comments only; if a code line carries a trailing citation tail, the code prefix must remain byte-identical (the word-diff audit in gate 2 proves it).
  - Preserve shellcheck directive comments verbatim (they are functional, not narrative).
  - Comment-only. Gate failure = revert the implicated file and redo; never fix code.

  Gates per <verify>, then commit: git commit -m "docs(261104-cms): comment hygiene sweep — drop planning-artifact citations, compress narrative headers" -- <exact swept paths>. The commit MUST be path-scoped: the milestone-archive staged deletions under .planning/ are pending in the index — never git add -A, never a bare git commit -a. Post-commit, git show --stat HEAD must list only swept files. No push.
  </action>
  <verify>
    <automated>
    cd /Users/neilskinner/code/harvey

    # gate 1 — zero citations in swept paths (residuals zero, or each recorded in the SUMMARY as coincidental):
    PAT='2611[0-9]{2}-[a-z0-9]{3}|0[1-5]-[0-9]{2}|[A-Z]{1,5}-[0-9]{2}|T-[a-z0-9]+-[0-9]{2}|SC#[0-9]|OQ-[0-9]|Pitfall [0-9]|\.planning/|[0-9]{2}-(SUMMARY|RESEARCH|CONTEXT|PATTERNS|VERIFICATION|EVIDENCE|REVIEW)|(SUMMARY|RESEARCH|CONTEXT)\.md|deferred-items|UNSAFE-AUDIT|HowItWorks\.md|Phase [0-9]|phase-0[0-9]|plan 0[0-9]-[0-9]{2}|wave [0-9]'
    grep -rnI -E "$PAT" --exclude='*.md' --exclude='*.log' --exclude-dir=out --exclude-dir=smoke-out --exclude-dir=__pycache__ build/aarch64
    grep -rnI -E "$PAT" build/harness/src nix flake.nix .github/workflows/r9-shell.yml build/mm/boot-9k10cpu.sh sys/src/9k/boot/bootr9.c sys/src/9k/boot/mkfile.aarch64 sys/src/cmd/7a/lex.c sys/src/cmd/7c/swt.c sys/src/cmd/7l/unixcompat.c sys/src/cmd/mk/lex.c sys/src/cmd/mkfile

    # gate 2 — diff-invariant. Strippers verified against the swept corpus:
    #   CSTRIP is deliberately naive (no string protection — safe ONLY because the safety grep below
    #     proves no string literal in these C files contains a comment opener; apostrophes in prose
    #     comments and '"' char literals break protected strippers, not this one)
    #   RSTRIP (harness .rs) protects double-quoted strings first
    #   HASHSTRIP strips whole-line # comments only (sh/nix/yml/mkfile/expect)
    TMPD=$(mktemp -d)
    CSTRIP='s{/\*.*?\*/}{}gs; s{//[^\n]*}{}g'
    RSTRIP='s{("(?:\\.|[^"\\])*")|//[^\n]*|/\*.*?\*/}{defined $1 ? $1 : ""}gse'
    HASHSTRIP='s{^[ \t]*#[^\n]*$}{}gm'
    SQ='s{[ \t]+\n}{\n}g; 1 while s{\n[ \t]*\n}{\n}g; s{\A[ \t\n]+}{}; s{[ \t\n]+\z}{}'
    for f in $(git diff --name-only -- build nix flake.nix .github sys); do
      case "$f" in
        *.c|*.h)
          grep -q '"[^"]*/[*]\|"[^"]*//' "$f" && { echo "SAFETY-GREP HIT: $f — audit by hand: git diff --word-diff=porcelain -- $f"; continue; }
          STRIP="$CSTRIP" ;;
        *.rs)     STRIP="$RSTRIP" ;;
        *)        STRIP="$HASHSTRIP" ;;
      esac
      git show HEAD:"$f" | perl -0777 -pe "$STRIP; $SQ" > "$TMPD/pre"
      perl -0777 -pe "$STRIP; $SQ" "$f" > "$TMPD/post"
      cmp -s "$TMPD/pre" "$TMPD/post" || {
        echo "RESIDUAL DELTA: $f — audit: git diff --word-diff=porcelain -- $f"
        echo "  any code-token delta => git checkout -- $f and redo; comment-only delta (e.g. a trailing # tail) => record in SUMMARY and proceed"
      }
    done; rm -rf "$TMPD"

    # gate 3 — host build sanity: toolchain app rebuilds 7a/7c/7l (BUILDTOOLCHAIN legs) covering lex.c/swt.c/unixcompat.c:
    nix run .#toolchain
    # mk has no BUILD script — build it in the Plan 9 namespace (darwin: lima; linux: nsrun.sh --wrap natively):
    sh build/aarch64/vm-run.sh 'cd sys/src/cmd/mk && mk nuke && mk'
    # harness crate still compiles:
    cargo check --manifest-path build/harness/Cargo.toml

    # gate 4 — lint/parse equivalence:
    for f in $(git diff --name-only -- '*.sh'); do
      diff <(git show HEAD:"$f" | shellcheck -f gcc - 2>&1) <(shellcheck -f gcc "$f" 2>&1) && echo "shellcheck-equivalent: $f"
    done
    nix flake metadata >/dev/null
    python3 -c 'import yaml,sys; yaml.safe_load(open(".github/workflows/r9-shell.yml"))'
    </automated>
  </verify>
  <done>Gates 1/3/4 green; gate 2 shows no unaudited residuals (same audit discipline as Task 1); the commit lists only swept files (git show --stat HEAD); the staged .planning archive deletions remain staged and uncommitted; one commit on master, nothing pushed.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| editor → source tree | a mechanical sweep across ~70 files — a non-comment edit smuggled in with the comment edits |
| comments ↔ build lints | some comments are load-bearing: r9 SAFETY justifications (denied-lint backed), shellcheck directives, commented-out code rows |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-cms-01 | Tampering | every swept file | high | mitigate | diff-invariant gate per repo (string-protecting strip + blank-squeeze + cmp) plus full build/test batteries; any residual code delta = revert the file and redo |
| T-cms-02 | Tampering | r9 unsafe blocks | medium | mitigate | SAFETY comments are restyle-never-delete; workspace undocumented_unsafe_blocks=deny makes a dropped justification a loud build failure (gates 3) |
| T-cms-03 | Tampering | harvey .sh + 7a/lex.c | medium | mitigate | shellcheck directives preserved verbatim + shellcheck-equivalence gate; commented-out LSPREG rows left untouched; whole-line-# rule for script files with word-diff audit for residuals |
| T-cms-SC | Tampering | npm/pip/cargo installs | low | accept | no package installs occur in this plan; the package-legitimacy gate is inert — cargo commands build/test the existing locked dep sets only (r9 Cargo.lock, harness Cargo.lock untouched) |
</threat_model>

<verification>
- r9: gates 1-4 of Task 1 green; `git -C /Users/neilskinner/code/r9 log --oneline -1` shows the docs(261104-cms) commit on main; `git -C /Users/neilskinner/code/r9 show --stat HEAD` lists only comment-swept files.
- harvey: gates 1-4 of Task 2 green; `git log --oneline -1` shows the docs(261104-cms) commit on master; `git show --stat HEAD` lists only swept files; `git status --short` still shows the pre-existing staged .planning archive deletions, uncommitted.
- Neither repo pushed (no `git push` anywhere in this plan).
</verification>

<success_criteria>
- Both repos carry exactly one new comment-only commit each; every swept file's stripped-pre/stripped-post comparison is byte-identical.
- Citation grep over both swept path sets returns zero hits (or each residual recorded as coincidental in the SUMMARY).
- r9: port tests 80/80, xtask check/clippy/test clean, five config builds, five boot-test profiles green.
- harvey: toolchain app rebuild green (7a/7c/7l), mk namespace build green, harness cargo check green, shellcheck warning sets identical to HEAD, flake metadata + workflow YAML parse clean.
</success_criteria>

<output>
Create `.planning/quick/261104-cms-comment-hygiene-sweep-across-harvey-r9-r/261104-cms-01-SUMMARY.md` when done — record: per-repo commit SHAs, swept-file counts, any residual coincidental grep hits (file + line + why it stays), any files reverted-and-redone and why, and gate runtimes (esp. the five boot-tests).
</output>
