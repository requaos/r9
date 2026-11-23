---
phase: 01-stage-p0-cross-build-harness
plan: 14
subsystem: cross-build-harness
tags: [nix, harvey-tree, amd64, proot, tiers, stage-build, net-section, authdomain-gen, libraries, devshell, harn-03, harn-07, d-03, d-16, d-19, d-25]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-13's build-env venue (hostbin + proot mkTreeDerivation), the flake Harvey input pinned at bd7a698, the arm64 smoke-tree precedent and the lazy/guarded devshell export pattern"
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-11's image engine (Manifest, check_libraries, the `cargo xtask image` subcommand) and 01-08's per-arch resolver (HARVEY-TREE stamp, realise-on-first-use)"

provides:
  - "nix/harvey/tree-amd64.nix: the full amd64 tree — tier0/1/2/3/4 (27/27, 40/40, 68/68, 11/11, 118/118), the ape/lib substrate, the net section (9/9), the stage-build markers/chain/fixtures (17/17), and the D-19 source-side install list with the HARVEY-TREE stamp"
  - "nix/harvey/authdomain-gen.nix: the host p9sk1 domain generator from the vendored donor sources (tool only; Linux-only)"
  - "flake.nix: packages harvey-tree-amd64 (Linux) and harvey-authdomain-gen (Linux); the guarded lazy devshell export R9_HARVEY_DIR_AMD64 and the R9_AUTHDOMAIN_GEN export where the tool builds"
  - "xtask/manifests/amd64/libraries.toml: the 27 tier-0 [[library]] rows (in_image = false, source = harvey), checked by `cargo xtask image --arch x86-64 --libraries`"
  - "xtask: a check-only `--libraries` mode defaulting to the libraries manifest, and the [[library]] schema's source row field"

affects: [01-15, 01-16, 01-17, 01-19, HARN-03, HARN-07, D-16, D-19]

actuals:
  tokens: 13405
  tasks: 2
  commits: 2

plan_head_before: 6d86834e60eaf9d406e81d8259e883db5d6e442f
plan_head_after: fab8b361217b4f0c35c90abd8d1d12eb27bef6e3

tech-stack:
  added:
    - "harvey-tree-amd64 and harvey-authdomain-gen flake packages"
    - "the amd64 tier dispatcher and the file-wise store-copy helper in the proot venue"
  patterns:
    - "One work-list extractor per pinned list with a recorded sha256: all five tier files, the STAGE_SRCS/CHAIN_LIBS/chain/sweep/fpprobe lines, the net and desktop section labels, and the net-helper line (T-01-47)"
    - "Per-tier dispatch mirroring subset-build.sh: tier-0 fresh archive; tier-1 driver-only/leaf/dir/link-only forms; tiers 2-4 per-entry clean then install with a before/after /amd64/bin snapshot harvest"
    - "Two pinned no-op entries are counted as built but produce no bin artifact (cmd/ld's `#' mkfile, cmd/go's /bin/go guard)"
    - "Source-side data is an explicit install list with read-site comments, never a whole-subtree sweep (D-19, HARN-02)"
    - "Store trees are copied file-wise (mkdir + cp files) because the determinate-nixd builder refuses directory-mode changes with ENOTSUP"

key-files:
  created:
    - nix/harvey/tree-amd64.nix
    - nix/harvey/authdomain-gen.nix
    - xtask/manifests/amd64/libraries.toml
  modified:
    - flake.nix
    - xtask/src/main.rs
    - xtask/src/image.rs

key-decisions:
  - "The tree copies store trees file-wise (mkdir -p + per-file cp) instead of cp -R: the registered builder's filesystem returns ENOTSUP on the directory chmod cp -R performs ('setting permissions ...: Operation not supported'), and the final tree is read-only anyway."
  - "PATH ends in `.` for the guest (Plan 9's default path): rc's `.` builtin and mk's bare command lookup resolve cwd-local generator scripts (mkenam/mkcname) through $path, never the cwd — the coverage venue's measured rule."
  - "The guest creates /var/tmp (plan9port ed's hardcoded scratch path) and /mail/lib (upas/misc's install target that build/mkdirs omits)."
  - "The ape/lib substrate is built before the mass tiers: the pcc/APE class (compress, gs, postscript, lp) links /amd64/lib/ape/libap.a, exactly as the coverage sweep's PASS B establishes."
  - "cmd/ld and cmd/go are the only tolerated zero-artifact entries: cmd/ld's mkfile recurses its rules to `#`, and cmd/go's install is guarded on an absolute /bin/go and prints 'No go, skipping'. Every other empty install fails the derivation."
  - "authdomain-gen is Linux-only: the vendored donor crypt.c declares encrypt(void*,void*,int), which collides with macOS <unistd.h> encrypt(char*,int) under clang. The plan's documented contingency applies; R9_AUTHDOMAIN_GEN is exported only where the tool builds."
  - "`--libraries` is a check-only mode defaulting to the `libraries` manifest when --image is absent, and the [[library]] schema accepts the plan's `source = \"harvey\"` row field (deny_unknown_fields had rejected it)."

requirements-completed: [HARN-07, HARN-03]

coverage:
  - id: D1
    description: "harvey-tree-amd64 builds every pinned tier, the ape substrate, the net section and the stage-build programs from the pinned source in the proot venue, with failures fatal"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-tree-amd64 -> /nix/store/4g1idpis9vlqbqss9483yxqbymvigzrk-harvey-tree-amd64-0; nix log lines: TREE-AMD64 tier0 built=27/27, tier1 built=40/40, tier2 built=68/68, tier3 built=11/11, tier4 built=118/118, ape-libs built=ok, net built=9/9, stage built=17/17"
        status: pass
    human_judgment: false
  - id: D2
    description: "The tree is a read-only, case-clean store path carrying the built userland plus the D-19 source-side fixtures, the chain/marker trees and the HARVEY-TREE stamp (arch=amd64)"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "nix store verify --no-trust -> clean; grep -qx 'arch=amd64' HARVEY-TREE -> AMD64-TREE-OK; find | tr A-Z a-z | sort | uniq -d | wc -l = 0; TREE-AMD64-BIN-SCAN binaries=402 scripts=28 empty=1 total=431; 4931 files; font tree 4303 files; build/x86_64/root-tree and lib/font/bit present"
        status: pass
    human_judgment: false
  - id: D3
    description: "The devshell exports R9_HARVEY_DIR_AMD64 lazily and never clobbers a user value; ureg-check parses both architectures' headers with nothing set"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "devshell export == nix eval --raw outPath -> LAZY-PATH-MATCH (/nix/store/4g1idpis…-harvey-tree-amd64-0); R9_HARVEY_DIR_AMD64=/tmp/keep-me survives; devShell drv has 0 harvey-tree references; nix develop -c env -u R9_HARVEY_DIR -u HARVEY_SRC cargo xtask ureg-check -> 35 arm64 fields, 26 amd64 fields, negative control fired"
        status: pass
    human_judgment: false
  - id: D4
    description: "The 27-row tier-0 library table is committed and checked against the tree"
    requirement: HARN-03
    verification:
      - kind: other
        ref: "grep -c '^\\[\\[library\\]\\]' xtask/manifests/amd64/libraries.toml = 27; nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --libraries -> 'libraries: image x86_64/libraries tier-0 list OK'"
        status: pass
    human_judgment: false
  - id: D5
    description: "The host authdomain-gen builds from the pinned donor sources as a tool only, with no domain or key material in the store"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "nix build .#packages.aarch64-linux.harvey-authdomain-gen -> /nix/store/g2706qh5…-harvey-authdomain-gen-0; nix store verify --no-trust clean; the output carries only bin/authdomain-gen; no generate run happens in the derivation (T-01-48)"
        status: pass
    human_judgment: true
    rationale: "The Mac cannot build the tool (macOS unistd.h encrypt clash; the plan's Linux-only contingency), so R9_AUTHDOMAIN_GEN is verified on Linux only; the generator's runtime correctness is 18-03's committed self-test and 01-16's bake, not this plan."
  - id: D6
    description: "No kernel bytes changed and Harvey is untouched"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "target/x86_64-unknown-none-elf/debug/x86_64 sha256 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f and target/aarch64-unknown-none-elf/debug/aarch64 sha256 76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2, both equal to the 01-08 baseline; Harvey HEAD 25d97dc4d…, status hash 041d2929…"
        status: pass
    human_judgment: false
  - id: D7
    description: "Every installed amd64 binary carries S_MAGIC 00 00 8a 97; the long-tail tier's rc scripts and disk's ksync are the only non-binary outputs"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "scoped scan of $p/amd64/bin: 402 files with magic 00008a97, 28 files starting '#!', 1 empty file = disk/ksync (the pin's touch rule), 0 unexplained files"
        status: pass
    human_judgment: false

duration: 49min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 14: harvey-tree-amd64 and the host authdomain-gen Summary

**`harvey-tree-amd64` is a flake-built, read-only store path carrying Harvey's whole amd64 userland — every pinned tier (27/27 libraries, 40/40 tier-1 commands, 68/68 services, 11/11 GUI, 118/118 long tail), the ape/lib substrate, the net section, the stage-build marker/chain/fixture programs and the source-side fixtures the assembler and moved gates read — and the host `authdomain-gen` plus the 27-row tier-0 library table are wired into the flake and the devshell.**

## Performance

- **Duration:** ~49 min of executor time (base commit 2026-11-23T07:05:21Z to 2026-11-23T07:54Z), dominated by six full builder runs as each venue/pin port surfaced (each run rebuilds tiers 0-3 before reaching the failing tier-4 entry)
- **Started:** 2026-11-23T07:05:21Z (the 01-13 tracking commit the worktree forked from)
- **Completed:** 2026-11-23T07:54:00Z
- **Tasks:** 2 of 2
- **Files modified:** 6 (3 created, 3 modified; 1143 insertions, 8 deletions)
- **Plan estimate:** 90,000 tokens; **actual:** 13,405 tokens (chars/4 over the realized diff). The estimate was low-confidence and sized the reading/iteration, not the code; the wall time went to builder runs, which the token measure does not see.

## Build times and scale (the plan's requested record)

Measured inside the guest on the registered aarch64-linux builder (4 cores):

| Step | Wall time | Result |
|---|---|---|
| tier0 (27 libs, fresh archive) | 26 s | `TREE-AMD64 tier0 built=27/27` |
| tier1 (40 commands) | 74 s | `TREE-AMD64 tier1 built=40/40` |
| ape/lib substrate | 23 s | `TREE-AMD64 ape-libs built=ok` (12 archives) |
| tier2 (68 services) | 55 s | `TREE-AMD64 tier2 built=68/68` |
| tier3 (11 GUI) | 10 s | `TREE-AMD64 tier3 built=11/11` |
| tier4 (118 long-tail dirs) | 136 s | `TREE-AMD64 tier4 built=118/118` |
| net section | 8 s | `TREE-AMD64 net built=9/9` |
| stage-build programs | 16 s | `TREE-AMD64 stage built=17/17` |

Whole derivation: ~6 min including the (cached) hostbin, the store→proot copies and the install/scan. Final tree: **4931 files**; `amd64/bin` 431 files (402 a.out binaries, 28 `#!` scripts, 1 pinned empty `disk/ksync`); `amd64/lib` 27 tier-0 archives + 12 ape archives; font tree 4303 files.

## Accomplishments

- **Every tier builds in the plan 01-13 venue.** `nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-tree-amd64` prints `/nix/store/4g1idpis9vlqbqss9483yxqbymvigzrk-harvey-tree-amd64-0`; the log carries all five `TREE-AMD64 tier<N> built=<k>/<n>` lines with `k == n` (27/27, 40/40, 68/68, 11/11, 118/118), plus the ape-libs, net (9/9) and stage (17/17) lines. `nix store verify --no-trust` is clean; the case-fold duplicate count is 0 and the case-hack count is 0 (the venue's assertions).
- **The dispatch is the pin's, not a copy.** Tier 0 applies the fresh-archive rule (`rm -f` before `mk install`). Tier 1 follows subset-build's forms: `9k/boot` via `mkfile.amd64`, `stage/init` via `init.install` harvested from `/amd64/init`, `cmd/<dir>:<target>` leaves (factotum, paqfs — the latter also harvested to `out/`), dir installs, and link-only singles with the driver's `cp` harvest. Tiers 2-4 use the per-entry fresh-build discipline (`mk clean` → `mk install` → before/after `/amd64/bin` snapshot); `cmd/cc`'s evidence is `cc.a6`; `cmd/ld` and `cmd/go` are the pin's two deliberate no-ops.
- **The work lists are pinned twice.** Every extracted list's sha256 is recorded in the nix file and asserted in the guest: tier0 `c0ad974b…`, tier1 `104e2243…`, tier2 `ec951531…`, tier3 `446a2658…`, tier4 `f9e446d4…`; the `STAGE_SRCS` line `449ace85…`, `CHAIN_LIBS` `890f0020…`, the chain list `3d5dea60…`, fpprobe `5c1de745…`, the sweep list `a64beae9…`, the net labels `409b60ba…`, desktop labels `5f07b9fa…`, net-helper line `e5e9eadc…`. A Harvey change at a bump fails the build loudly (T-01-47).
- **The stage-build programs land at the paths mkimage reads.** `build/x86_64/out/` carries `boot init rc paqfs ls cat echo ps rm test netblast tcpblast tcpecho`; `build/x86_64/smoke-out/` carries `smokesleep churnmark churnsem churncats randread init fpprobe`; `amd64/init` is the real init. Every one is magic-checked `00008a97`.
- **The ape/lib substrate is present.** `amd64/lib/ape/` carries 12 archives including `libap.a`; the pcc/APE tier-4 commands (compress, gs, postscript, lp, …) link against them.
- **The source-side install list is explicit (D-19).** Directories: `build/x86_64/root-tree`, `build/x86_64/tiers`, `build/x86_64/authdom-donors`, `lib/font/bit` (4303 files), `lib/ndb`, `rc/lib`, `adm/timezone`. Files: `build/x86_64/{tier1-sweep.rc,tier-termrc,services-sweep.rc,services-termrc,auth-sweep.rc,auth-termrc,bootfs-payload.txt,nm-input.6,secstore-obj.txt,authdomain-gen.c,authdomain.example,svctcpecho.c,authprobe.c}`, `sys/src/cmd/mkdir.c`, `usr/glenda/lib/plumbing`, `sys/lib/plumb/{basic,fileaddr}`. Each entry's mkimage read site is named in the nix file. `rc/bin` is never installed wholesale (the case-colliding set); no 9k kernel or r9 artifact rides.
- **The devshell supply is lazy and guarded.** `R9_HARVEY_DIR_AMD64` equals the evaluated outPath (LAZY-PATH-MATCH), a pre-set value survives shell entry, the devShell derivation has zero references to the tree, and `cargo xtask ureg-check` parses both architectures (35 arm64 fields, 26 amd64 fields, negative control fired).
- **The library table holds.** `xtask/manifests/amd64/libraries.toml` has 27 `[[library]]` rows (`source = "harvey"`, `in_image = false`) and `cargo xtask image --arch x86-64 --libraries` prints `libraries: image x86_64/libraries tier-0 list OK`.
- **The host generator is packageable.** `harvey-authdomain-gen` builds on aarch64-linux from the vendored donor sources (`/nix/store/g2706qh5…`), verifies clean, and installs only `bin/authdomain-gen` — no domain is generated in the derivation (T-01-48). Its Linux-only limitation is the plan's documented contingency (macOS `unistd.h` `encrypt` clash).
- **Kernel bytes and Harvey are untouched.** q35churnshell `50b01c9a…` and virtshell `76e6e153…` equal the 01-08 baseline after the flake change; Harvey HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d2929…`; zero `port/src` changes.

## Task Commits

Each task was committed atomically:

1. **Task 1: The full amd64 tree — tiers, net section, stage programs, ape substrate, source-side list and stamp** — `238121b` (feat)
2. **Task 2: host authdomain-gen, the devshell export, the 27-row library table and the `--libraries` mode** — `fab8b36` (feat)

**Plan metadata:** the `docs(01-14): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/harvey/tree-amd64.nix` (created) — the tier/net/stage dispatcher with recorded list hashes, the file-wise store copy, the venue ports (`PATH` ends `.`, `/var/tmp`, `/mail/lib`), the ape substrate, the source-side install list and the HARVEY-TREE stamp
- `nix/harvey/authdomain-gen.nix` (created) — the host generator from the vendored donors; tool only, compile-only proof, no domain generation
- `xtask/manifests/amd64/libraries.toml` (created) — 27 tier-0 `[[library]]` rows
- `flake.nix` (modified) — `harveyTreeAmd64`, `harveyAuthdomainGen`; packages `harvey-tree-amd64`/`harvey-authdomain-gen` (Linux); guarded lazy `R9_HARVEY_DIR_AMD64`; `R9_AUTHDOMAIN_GEN` where available
- `xtask/src/main.rs` (modified) — `--libraries` is check-only and defaults to the `libraries` manifest; `--image` required otherwise
- `xtask/src/image.rs` (modified) — `LibraryEntry.source` (optional, must be `harvey`) and its check

## Decisions Made

- **The tree copies store trees file-wise.** The registered builder's filesystem returns `ENOTSUP` on the directory chmod `cp -R` performs; `mkdir -p` + per-file `cp` preserves the executable bits and the final tree is read-only anyway.
- **The guest PATH ends in `.`.** Plan 9's default path is what rc's `.` builtin and mk's bare lookup use for cwd-local generators (the tier-4 `mkenam`/`mkcname` scripts); last position means a cwd file can never shadow a toolchain command.
- **`/var/tmp` and `/mail/lib` are created in the guest.** plan9port ed hardcodes `/var/tmp/eXXXXX`; upas/misc's install writes into `/mail/lib`, which `build/mkdirs` does not create.
- **The ape substrate is a first-class build step.** The pcc/APE class links `/amd64/lib/ape/libap.a`; the coverage sweep's PASS B is the precedent.
- **Only `cmd/ld` and `cmd/go` may install zero artifacts.** Both are pinned no-ops; everything else fails loudly.
- **authdomain-gen stays Linux-only.** The donor `crypt.c`'s `encrypt(void*,void*,int)` collides with macOS's `encrypt(char*,int)`; the plan's contingency says restrict it to Linux and record that the cpu/auth images assemble on Linux, so 01-16's Mac checks are limited to parse and drift on fixtures.
- **`--libraries` is an audit mode.** It reads `xtask/manifests/<objtype>/libraries.toml` by default, returns after the check, and the schema now carries the plan's `source` row field.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Store-directory copies fail on the registered builder**
- **Found during:** Task 1, the first venue runs
- **Issue:** `cp -R` of a store tree inside proot exits 1 with `cp: setting permissions for '/386': Operation not supported` (the determinate-nixd builder's filesystem refuses the directory chmod; `--no-preserve=mode` does not help).
- **Fix:** a `copy_tree_files` helper (mkdir -p + per-file cp) used for the per-arch venue dirs, the built userland and every source-side tree.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** the build passes the copy stage; the final tree carries all files (4931).
- **Committed in:** `238121b`

**2. [Rule 3 - Blocking] rc's `.`/bare lookup could not find the cwd generator scripts**
- **Found during:** Task 1, tier4 `cmd/4c` (`mkenam: rc: .: can't open`)
- **Issue:** Plan 9 rc resolves `. file` and bare commands through `$path`; the venue PATH had no `.`, so the tier-4 `mkenam` generators were invisible.
- **Fix:** `export PATH="$SHIM:$PATH:."` (last — the coverage venue's measured rule).
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** `cmd/4c`, `cmd/7l` and every later compiler generator builds.
- **Committed in:** `238121b`

**3. [Rule 3 - Blocking] plan9port ed needs `/var/tmp`**
- **Found during:** Task 1, tier4 `cmd/7l` (`?TMP` then `cnam.c` missing)
- **Issue:** plan9port ed writes its scratch file at the absolute `/var/tmp/eXXXXX`; the proot root has no `/var`.
- **Fix:** `mkdir -p /var/tmp` in the guest (and `/mail/lib`, see 7).
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** `cmd/7l`'s `sh mkcname` generates `cnam.c`.
- **Committed in:** `238121b`

**4. [Rule 1 - Bug] The `cmd/cc` special case checked the wrong archive name**
- **Found during:** Task 1, tier4 `cmd/cc` (my `cc.a$objtype` = `cc.aamd64`)
- **Issue:** the mklib archive is `cc.a6` (the compiler suffix), exactly as subset-build pins.
- **Fix:** the check and the pre-clean use `cc.a6`.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** `cmd/cc` builds and `cc.a6` exists.
- **Committed in:** `238121b`

**5. [Rule 2 - Missing Critical] The pcc/APE class needs the ape/lib substrate**
- **Found during:** Task 1, tier4 `cmd/compress` (`cannot open /amd64/lib/ape/libap.a`)
- **Issue:** pcc links the APE libs; nothing built them.
- **Fix:** a `build_ape_libs` step (`sys/src/ape/lib`, `mk install`, asserting `libap.a`) before the mass tiers — the coverage sweep's PASS B.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** `ape-libs built=ok`; compress/gs/postscript/lp build; `amd64/lib/ape` carries 12 archives.
- **Committed in:** `238121b`

**6. [Rule 1 - Bug] `cmd/go` is a pinned no-op install**
- **Found during:** Task 1, tier4 `cmd/go` (`No go, skipping`)
- **Issue:** its install is guarded on an absolute `/bin/go`; the sandbox has none.
- **Fix:** tolerate zero artifacts for `cmd/go` alongside `cmd/ld`; every other empty install still fails.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** tier4 reaches 118/118.
- **Committed in:** `238121b`

**7. [Rule 3 - Blocking] upas/misc installs into a missing `/mail/lib`**
- **Found during:** Task 1, tier4 `cmd/upas` (`cp: target '/mail/lib': No such file or directory`)
- **Issue:** `build/mkdirs` creates the other mail spools but not `mail/lib` (it exists in the source tree, which the venue does not copy).
- **Fix:** `mkdir -p /mail/lib` in the guest.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** upas builds its whole DIRS set.
- **Committed in:** `238121b`

**8. [Rule 1 - Plan defect] Task 1's literal bin-magic acceptance contradicts the tier-list must-have**
- **Found during:** Task 1's acceptance re-run
- **Issue:** the criterion says every file under `$p/amd64/bin` starts `00 00 8a 97`, but the pinned tier-4 entries install 28 `#!` rc/cpu scripts (git, replica, venti/conf, auth/enable, …) and `cmd/disk`'s `ksync` is `touch`-created empty. Installing every tier entry (the plan's must-have) makes the literal command print `00008a97`, `23212f62` and an empty line.
- **Fix:** implemented the must-have; verified the intent instead: every non-script file is `00008a97` (402), every script starts `#!` (28), the one empty file is exactly `amd64/bin/disk/ksync`, and there are 0 unexplained files. The submitted `find … | sort -u` output is recorded above.
- **Files modified:** none (verification scope only)
- **Verification:** scoped scan BAD-COUNT=0; the derivation itself enforces this classification per installed file.
- **Committed in:** `238121b` (the enforcement)

**9. [Rule 3 - Blocking] `cargo xtask image --arch x86-64 --libraries` could not run as written**
- **Found during:** Task 2's verification
- **Issue:** clap required `--image`, and the handler assembled an image after the library check; the plan's command has neither an image nor an amd64 manifest to assemble. Separately, `LibraryEntry`'s `deny_unknown_fields` rejected the plan's `source = "harvey"` row.
- **Fix:** `--libraries` is now a check-only mode that defaults to `libraries.toml` when `--image` is absent (`--image` stays required otherwise); `LibraryEntry` carries an optional `source` and `check_libraries` rejects a non-harvey source.
- **Files modified:** `xtask/src/main.rs`, `xtask/src/image.rs`
- **Verification:** the exact plan command prints `libraries: image x86_64/libraries tier-0 list OK`; `cargo test -p xtask` 79 tests + 3 doctests green.
- **Committed in:** `fab8b36`

**10. [Rule 3 - Platform] authdomain-gen cannot build on aarch64-darwin**
- **Found during:** Task 2's first host build
- **Issue:** the vendored donor `crypt.c` declares `extern int encrypt(void*, void*, int)`, which conflicts with macOS `<unistd.h>`'s `encrypt(char *, int)` under clang.
- **Fix:** the plan's documented contingency — restrict the package to Linux and export `R9_AUTHDOMAIN_GEN` only where it builds. The cpu and auth images therefore assemble on Linux; 01-16's Mac checks are limited to parse and drift on fixtures.
- **Files modified:** `flake.nix`, `nix/harvey/authdomain-gen.nix`
- **Verification:** aarch64-linux builds and verifies clean; the Mac devshell leaves `R9_AUTHDOMAIN_GEN` unset.
- **Committed in:** `fab8b36`

### Commit organization

The tree derivation is one file, so Task 2's source-side install list and stamp landed in the same file and were committed with Task 1 (the alternative — a second full builder run after splitting the file — would have recompiled every tier to re-prove an unchanged build). Task 2's commit carries the host tool, its flake wiring, the library table and the CLI/schema change. Both tasks' acceptance criteria are verified in this SUMMARY.

### Not run (plan convention)

The x86_64-linux build of `harvey-tree-amd64` is the plan's stop-B witness step, not a Mac leg; it was not run here.

---

**Total deviations:** 10 (6 blocking venue/platform fixes, 2 bugs, 1 missing-critical, 1 plan-defect verification scope). **Impact:** all but the verification-scope note were required to make the pinned build run on this builder; none changes the plan's architecture, the tree's membership, or the contract the later plans consume.

## Issues Encountered

- **Six builder runs were needed.** Each venue/pin gap (store-copy chmod, rc path, ed temp dir, cc archive name, ape libs, the /bin/go no-op, /mail/lib) surfaced only after tiers 0-3 rebuilt, so the loop cost ~5 minutes per discovery. The fixes are all local to the tree derivation and documented above.
- **The tree's bin count (431) is the tier membership, not the checkout's 591.** The larger checkout set includes binaries built by other plans/coverage; the pinned five lists are the plan's work list.
- **`requirements.ready-ids` blocks HARN-07/HARN-03** because sibling plans (01-17 and others) also declare them and have no SUMMARY yet; the shared-ID gate (#2388) correctly leaves them Pending in REQUIREMENTS.md. The last declaring plan will flip them.

## Known Stubs

None. The two zero-artifact tier entries (`cmd/ld`, `cmd/go`) are pinned upstream no-ops, not stubs; every other entry installs its artifact.

## Threat Flags

None beyond the plan's threat model. The implemented surface matches its mitigations: T-01-47 (each extracted list's sha256 asserted before use), T-01-48 (authdomain-gen is a compile-only tool derivation; no domain is generated in a derivation, and no key material is installed), T-01-49 (explicit install list, case-fold count 0, case-hack count 0), T-01-SC (locked nixpkgs; no npm/pip/cargo installs).

## Next Phase Readiness

- **Plan 01-15 consumes the tree as the reference bake's input.** `harvey-tree-amd64` carries every read `mkimage.sh` makes, including `sys/src/cmd/mkdir.c` and `build/x86_64/{svctcpecho,authprobe}.c` for the host-side builds. The reference derivation must stage the hostbin at `$HARVEY/.build/hostbin-linux-x86_64/bin` (mkimage's 11d default) or set `HARVEY_LINUX_BIN`.
- **Plan 01-16 gets `R9_AUTHDOMAIN_GEN` on Linux** and the generator's own committed self-test as the runtime proof; on the Mac the cpu/auth checks are parse/drift only (the Linux-only contingency).
- **Plan 01-17 mirrors this file's patterns:** list hashes, per-entry dispatch, the file-wise copy, the `PATH`-ends-`.` port, and the same two pinned no-ops.
- **The devshell now resolves both trees lazily;** `R9_HARVEY_DIR` remains unset and a manual override still wins.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/harvey/tree-amd64.nix`, `nix/harvey/authdomain-gen.nix`, `xtask/manifests/amd64/libraries.toml`, `01-14-SUMMARY.md` — FOUND; `flake.nix`, `xtask/src/main.rs`, `xtask/src/image.rs` modified.
- Task commits `238121b` (Task 1) and `fab8b36` (Task 2) are ancestors of HEAD; `git rev-list --count 6d86834..HEAD` = 2 (measured against the plan ledger).
- Task 1 verification re-run: the build prints `/nix/store/4g1idpis9vlqbqss9483yxqbymvigzrk-harvey-tree-amd64-0`; `nix log` shows `tier0 27/27`, `tier1 40/40`, `tier2 68/68`, `tier3 11/11`, `tier4 118/118`; the scoped bin scan has 0 unexplained files (402 a.out, 28 scripts, 1 pinned empty ksync).
- Task 2 verification re-run: `ureg-check` 35/26 + negative control; `image --arch x86-64 --libraries` OK; `nix store verify --no-trust` clean; `arch=amd64` stamp; duplicate count 0; 27 library rows; `lib/font/bit` and `build/x86_64/root-tree` present; authdomain-gen store verify clean; 79 xtask tests + 3 doctests green.
- Plan verification re-run: kernel ELF sha256 `50b01c9a…` (x86_64 q35churnshell) and `76e6e153…` (aarch64 virtshell) equal the 01-08 baseline; Harvey HEAD `25d97dc4d…`, status hash `041d2929…`; zero `port/src` changes; no push.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
