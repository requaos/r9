---
phase: 01-stage-p0-cross-build-harness
plan: 13
subsystem: cross-build-harness
tags: [flake, harvey-tree, arm64, proot, mkpaqfs, hostbin, devshell, lazy-supply, nix, harn-02, harn-06, harn-07, d-01, d-03, d-05, d-21, d-24, d-25]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-08's one resolver (harvey_dir, HARVEY-TREE stamp, realise-on-first-use) and the arm64 smoke manifest; 01-10's harvey flake input (bd7a698) with the nixpkgs follow and docs/harvey-supply.md; 01-12's stop-A proof that the registered ssh-ng lima builder imports aarch64-linux results into the Mac store; 01-32's stop-A2 rio PASS (a13 fallback not needed)"

provides:
  - "nix/harvey/build-env.nix: Harvey's host toolchain (hostbin, S1's four adjustments) and mkTreeDerivation — the proot-only venue (D-25) with the SANDBOX-EVIDENCE refusal, S4's two venue ports, the empty-tree guard and the case-clean assertions"
  - "nix/harvey/tree-arm64.nix: the arm64 smoke tree — the arm64 libc, the seven STAGE_SRCS binaries (list sha256-pinned from the pinned stage-build.sh) plus hello.7.out, arm64/include/, build/aarch64/netaddr.rc and the HARVEY-TREE stamp"
  - "nix/harvey/mkpaqfs.nix: the host mkpaqfs from the pinned paqfs source with plan9port, on all three systems"
  - "flake.nix: packages harvey-hostbin/harvey-tree-arm64 (Linux) and harvey-mkpaqfs (all); the devshell exports R9_HARVEY_DIR_ARM64 (lazy), R9_HARVEY_TREE_SYSTEM and R9_MKPAQFS, never R9_HARVEY_DIR, each guarded"
  - "The HARN-07 arm64 smoke supply end to end: virtexec boots from the flake-built read-only tree with nothing set by hand"

affects: [01-14, 01-17, HARN-02, HARN-06, HARN-07, D-01, D-03, D-05, D-21, D-24, D-25]

actuals:
  tokens: 9963
  tasks: 2
  commits: 3

plan_head_before: cda27d92dfc2438cbcf86c612fed24b3b239a52b
plan_head_after: 1d66e2bc78f3c30080d866f24874ad34aee6ecc8

tech-stack:
  added:
    - "harvey-hostbin, harvey-tree-arm64 and harvey-mkpaqfs flake packages"
  patterns:
    - "One build environment per supply: a host toolchain derivation plus a proot-only venue function every tree derivation calls with { name, arch, guestScript, installScript }"
    - "The tree's own build list is the pinned data, read at build time with its line sha256 asserted in the nix file — a Harvey change fails the next pin bump loudly (Pitfall 5)"
    - "A lazy devshell export via builtins.unsafeDiscardStringContext of a Linux derivation's outPath; only the small host mkpaqfs is eager"
    - "The devshell banner writes to stderr so `nix develop -c '<cmd>'` stdout is exactly the command's output"

key-files:
  created:
    - nix/harvey/build-env.nix
    - nix/harvey/tree-arm64.nix
    - nix/harvey/mkpaqfs.nix
  modified:
    - flake.nix
    - HowItWorks.md
    - docs/harvey-supply.md

key-decisions:
  - "The venue has no unshare branch at all (D-25): the proot path is the only one, so no future caller can accidentally reintroduce the refused user-namespace route; `grep -v '^[[:space:]]*#' nix/harvey/build-env.nix | grep -c unshare` is 0."
  - "The STAGE_SRCS line is hashed as the literal `STAGE_SRCS=\"...\"` line (sha256 311086077628ded705ccd5f10ebfca2676f3d718fd08f3f18e6f80b12c3f9e2d) rather than the parsed list, so a reordering/whitespace change is caught too."
  - "harveySrc for Linux trees is a builder-side fetchFromGitHub keyed on harvey.rev + harvey.narHash (D-24); a rev-less session override uses harvey.outPath. harvey-mkpaqfs always uses the flake input path because it is a host build, not a tree."
  - "The tree variables are guarded (`[ -n \"${VAR:-}\" ] || export ...`) so a user's value wins; R9_HARVEY_DIR is never exported. The shellHook banner goes to stderr so the plan's exact LAZY-PATH-MATCH comparison sees only the command's stdout."
  - "The smoke binaries differ from the checkout's by 48-70 bytes while paths and modes match exactly; hello.7.out is byte-identical. Recorded as informational (the checkout's binaries have unrecorded provenance — the same class S4 saw with echo)."
  - "Install runs inside the same guest as the build (one shell), writing the final layout under /out; the derivation copies $work/out to $out and applies the case + non-empty assertions on the final store layout."

patterns-established:
  - "Tree derivations: hostbin + proot venue + a build-time-extracted work list with a recorded sha256 + R_MAGIC checks + the HARVEY-TREE stamp + case-fold/case-hack/non-empty assertions"
  - "Every plan-following build check inspects the store path contents, not only the build exit: an empty output can satisfy case assertions"
  - "Per-arch shell exports are guarded string writes of context-discarded outPaths; the realise-on-first-use in xtask is the only build trigger"

requirements-completed: [HARN-07, HARN-02, HARN-06]

coverage:
  - id: D1
    description: "The arm64 smoke tree is a flake-built, read-only, case-clean store path carrying the libc, the eight smoke members, arm64/include and netaddr.rc, with the HARVEY-TREE stamp"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-tree-arm64 -> /nix/store/fnmh3z9sryf01qcc5w1fr9z9hkkwyxi7-harvey-tree-arm64-0; nix store verify --no-trust -> clean; grep -qx 'arch=arm64' HARVEY-TREE; 8 smoke members R_MAGIC 00008c47"
        status: pass
      - kind: other
        ref: "find $p | tr A-Z a-z | sort | uniq -d | wc -l = 0; find $p -name '*~nix~case~hack~*' | wc -l = 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "The devshell supplies the tree lazily and never clobbers a user value: R9_HARVEY_DIR_ARM64 equals the evaluated outPath, R9_HARVEY_TREE_SYSTEM/R9_MKPAQFS are exported, R9_HARVEY_DIR is not, and the devShell derivation does not reference the tree derivation"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "test \"$(nix develop -c sh -c 'printf %s \"$R9_HARVEY_DIR_ARM64\"')\" = \"$(nix eval --raw .#packages.aarch64-linux.harvey-tree-arm64.outPath)\" -> LAZY-PATH-MATCH; R9_HARVEY_DIR_ARM64=/tmp/keep-me nix develop ... -> /tmp/keep-me; nix-store -q --references devShell drv | grep tree = 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Inside nix develop with nothing set by hand, cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf boots the flake-tree-assembled smoke initrd and passes"
    requirement: HARN-07
    verification:
      - kind: integration
        ref: "nix develop -c env -u R9_HARVEY_DIR -u HARVEY_SRC cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf -> 'boot-test: PASS -- banner, exec entry, init print, hello print and all-exited markers observed in order, stacksmoke depth-64 marker present' (initrd sha256 8bf31ff661aca223185b54de1e28a28268ad283a25b4458e724682a6410de58b)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The smoke member list matches the 01-08 legacy list in paths and modes; hello is byte-identical and the seven rebuilt binaries differ only in size/hash (informational)"
    requirement: HARN-06
    verification:
      - kind: other
        ref: "diff <(awk '{print $1,$2,$3}' /tmp/r9-0108-legacy.members) <(awk '{print $1,$2,$3}' target/images/aarch64/smoke.members) -> empty (PATHS-MODES-IDENTICAL)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The host mkpaqfs builds from the pinned paqfs source with plan9port on aarch64-darwin and aarch64-linux and bakes a paq; the x86_64-linux attribute evaluates"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "nix build .#packages.aarch64-darwin.harvey-mkpaqfs -> /nix/store/mjh7r217id3s5lrbrh02zq6a3k8anw22-harvey-mkpaqfs-0; .#packages.aarch64-linux.harvey-mkpaqfs -> /nix/store/wndp129dz81lb2ci2fgmcvksr5dqsr3n-harvey-mkpaqfs-0; both bake an empty-root probe paq"
        status: pass
    human_judgment: true
    rationale: "The x86_64-linux build of harvey-mkpaqfs is not run here (the Mac cannot realise it without the stop-B x86_64 witness); only its evaluation is proven, and 01-14's stop B is where that leg belongs."
  - id: D6
    description: "The supply and resolver contract is documented: HowItWorks.md names every variable, the precedence, realise-on-first-use, the stamp and the read-only check; docs/harvey-supply.md §1 names the attributes and the per-host build commands"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "CONTRACT-DOC-OK (all seven literal keys present); grep -c harvey-tree-arm64 docs/harvey-supply.md = 6; grep -c R9_HARVEY_DIR_ARM64 HowItWorks.md = 2"
        status: pass
    human_judgment: true
    rationale: "The greps prove the keys are present; whether the prose is clear enough for a developer without the plan in front of them is a reading judgment."
  - id: D7
    description: "No kernel bytes changed and Harvey is untouched"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "sha256 x86_64 q35churnshell 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f and aarch64 virtshell 76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2, identical at plan start and after the last commit; Harvey HEAD 25d97dc4d, status hash 041d2929…; zero port/src changes"
        status: pass
    human_judgment: false

duration: 11min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 13: The arm64 Harvey tree supply — venue, host toolchain, mkpaqfs and the smoke slice Summary

**The tracer's manual arm64 supply is now a flake-built, read-only store path: `harvey-tree-arm64` builds the arm64 libc and the seven smoke binaries (plus `hello`) from the pinned source in a proot venue on the registered lima builder, the devshell exports it lazily with `R9_HARVEY_TREE_SYSTEM` and `R9_MKPAQFS`, and `cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf` passes under HVF with nothing set by hand.**

## Performance

- **Duration:** ~11 min of executor time (2026-11-23T06:55:30Z to 2026-11-23T07:06:20Z), dominated by the first hostbin + plan9port builds on the aarch64-linux builder and the two kernel baseline builds
- **Started:** 2026-11-23T06:55:30Z
- **Completed:** 2026-11-23T07:06:20Z
- **Tasks:** 2 of 2 (plus one hardening fix commit found by the output inspection)
- **Files modified:** 6 (3 created, 3 modified; 653 insertions, 13 deletions)

## Accomplishments

- **`harvey-tree-arm64` is real and verified.** `nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-tree-arm64` prints `/nix/store/fnmh3z9sryf01qcc5w1fr9z9hkkwyxi7-harvey-tree-arm64-0`; `nix store verify --no-trust` is clean; the stamp reads `rev=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`, `narHash=sha256-N6VjnVI69pcxSCpQ9BUxrlpTqnp9LgjKfGcpO3F6Ma0=`, `arch=arm64`, `system=aarch64-linux`; the eight smoke members all carry R_MAGIC `00 00 8c 47`; there are 0 case-fold duplicate names and 0 `~nix~case~hack~` names. The tree is 360 KiB.
- **The venue is the phase's build environment (D-03/D-24/D-25).** `nix/harvey/build-env.nix` carries Harvey's host toolchain (`hostbin`, with S1's four sandbox adjustments: store-bash `9l` shim, `pkgs.ed`, plan9port last on PATH, GNU sed shadowing) and `mkTreeDerivation` — proot only (no unshare branch exists), with the SANDBOX-EVIDENCE block, the `/usr/bin/env` refusal, S4's two venue ports (`/rc/bin` on PATH, `INCLUDE="/<objtype>/include /sys/include"`), a hard empty-tree guard and the case assertions. The arm64 tree build: 10 s measured inside the guest for the libc plus all seven compiles; the whole first derivation including the hostbin toolchain finished in under 3 minutes on the builder, and later builds reuse the hostbin.
- **The build list is the pinned data, pinned twice.** The guest reads `STAGE_SRCS=` from `build/aarch64/stage-build.sh` at build time and refuses to proceed unless the line's sha256 equals `311086077628ded705ccd5f10ebfca2676f3d718fd08f3f18e6f80b12c3f9e2d` (recorded in `tree-arm64.nix`), so a Harvey change to the list fails the next pin bump loudly (Pitfall 5).
- **mkpaqfs is a host tool on all three systems.** `nix/harvey/mkpaqfs.nix` builds Harvey's in-tree `sys/src/cmd/paqfs/mkpaqfs.c` with plan9port's `9c`/`9l` (`-lflate -lsec -lmp -lbio -l9`), runs an empty-root bake probe, and installs `bin/mkpaqfs`. Built: `/nix/store/mjh7r217id3s5lrbrh02zq6a3k8anw22-harvey-mkpaqfs-0` (aarch64-darwin) and `/nix/store/wndp129dz81lb2ci2fgmcvksr5dqsr3n-harvey-mkpaqfs-0` (aarch64-linux); `.#packages.x86_64-linux.harvey-mkpaqfs` evaluates.
- **The devshell supply is lazy and non-clobbering.** `nix develop -c sh -c 'printf %s "$R9_HARVEY_DIR_ARM64"'` prints the same path `nix eval --raw .#packages.aarch64-linux.harvey-tree-arm64.outPath` does (LAZY-PATH-MATCH); `R9_HARVEY_TREE_SYSTEM=aarch64-linux`; `R9_MKPAQFS=/nix/store/mjh7…/bin/mkpaqfs`; `R9_HARVEY_DIR` is empty; a pre-set `R9_HARVEY_DIR_ARM64=/tmp/keep-me` survives shell entry; the devShell derivation has zero references to the tree derivation.
- **The tracer boots from the supply with nothing set.** After `rm -f target/bootfs.paq target/images/aarch64/smoke.members`, `nix develop -c env -u R9_HARVEY_DIR -u HARVEY_SRC cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf` assembled the initrd from the tree (sha256 `8bf31ff6…`) and printed the full PASS line (banner, exec entry, init print, hello print, all-exited, stacksmoke depth-64).
- **The smoke member list matches 01-08.** Paths and modes are identical (`churnmark`, `churnsem`, `init`, `smokefork`, `smokenote`, `smokesleep`, `stacksmoke` at 775; `hello` at 755; `/proc` dir 755). `hello` is byte-identical (`1c89e099…`); the seven rebuilt binaries differ only in size (48-70 bytes smaller) and hash — recorded as informational below.
- **The docs carry the contract.** `HowItWorks.md` now has "The Harvey tree supply and the resolver contract" (per-arch trees, lazy exports, precedence, realise-on-first-use, the stamp, the no-default rule, the unchanged kernel package); `docs/harvey-supply.md` §1 names the attributes, `R9_HARVEY_TREE_SYSTEM`, `R9_MKPAQFS`, and a per-host build-command table with the `nix store verify` recipe.

## The informational smoke byte differences

The tree is built from the pinned source with the pinned toolchain; the checkout's `build/aarch64/smoke-out/` binaries have unrecorded provenance (S4 saw the same class with `echo`). Paths and modes match; sizes and hashes are informational:

| Member | 01-08 legacy size / sha256 prefix | This tree size / sha256 prefix | Δ |
|---|---|---|---|
| churnmark | 37184 / 69cbe744 | 37115 / 35c13e16 | −69 B |
| churnsem | 64269 / 90cb4775 | 64200 / 86e413c4 | −69 B |
| init | 39563 / c674f6ae | 39494 / c410608f | −69 B |
| smokefork | 38003 / 3d77ea00 | 37934 / ee62bf60 | −69 B |
| smokenote | 37778 / e14fd69c | 37709 / 41040248 | −69 B |
| smokesleep | 38075 / ce97b1e8 | 38006 / a1f44f4e | −69 B |
| stacksmoke | 37442 / f17a6469 | 37373 / b6801c5a | −69 B |
| hello | 37165 / 1c89e099 | 37165 / 1c89e099 | identical (committed binary) |

## Task Commits

Each task was committed atomically:

1. **Task 1: Venue, hostbin, mkpaqfs and the arm64 smoke tree; devshell exports; boot virtexec with nothing set** - `1fa084a` (feat)
2. **Task 2: Document the supply contract (HowItWorks.md, docs/harvey-supply.md)** - `8a778f3` (docs)
3. **Hardening fix found while verifying Task 1's output** - `1d66e2b` (fix)

**Plan metadata:** the `docs(01-13): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/harvey/build-env.nix` (created) — `hostbin` (Harvey's `nix/setup.sh` + `nix/toolchain.sh` with S1's four adjustments) and `mkTreeDerivation` (proot-only venue, SANDBOX-EVIDENCE, S4's two ports, empty-tree guard, case assertions)
- `nix/harvey/tree-arm64.nix` (created) — the arm64 smoke tree: libc + the seven STAGE_SRCS binaries (line sha256-pinned) + `hello.7.out` as `hello`, `arm64/include/`, `build/aarch64/netaddr.rc`, the HARVEY-TREE stamp
- `nix/harvey/mkpaqfs.nix` (created) — the host mkpaqfs with plan9port on every system, plus the empty-root bake probe
- `flake.nix` (modified) — `harveyTreeSrc` (builder-side FOD or the session override), `harveyEnv`, `harveyTreeArm64`, `harveyMkpaqfs`; packages `harvey-hostbin`, `harvey-tree-arm64` (Linux), `harvey-mkpaqfs` (all); the guarded lazy devshell exports and the stderr banner
- `HowItWorks.md` (modified) — the section rewritten as "The Harvey tree supply and the resolver contract"; the stale `../harvey`-default sentence corrected
- `docs/harvey-supply.md` (modified) — §1 attribute names, `R9_HARVEY_TREE_SYSTEM`/`R9_MKPAQFS`, and the per-host build-command table

## Decisions Made

- **No unshare branch at all.** The plan said "delete the unshare branch"; the deletion means the venue function has no mode switch, so the refused path cannot be reintroduced by a future caller.
- **Hash the literal STAGE_SRCS line, not the parsed list.** A reorder or whitespace change is caught too; the recorded sha256 is `311086…` (the pinned `stage-build.sh` line).
- **Install inside the guest, assert outside.** The install script writes the final layout under `/out` in the same shell as the build; the derivation copies `$work/out` to `$out` and runs the case + non-empty assertions on the final store layout (where they can see the real names).
- **The devshell banner goes to stderr.** The plan's literal LAZY-PATH-MATCH command compares `nix develop -c` stdout with `nix eval`; hook output on stdout would make that impossible for any shell that echoes. The banner keeps its information on stderr.
- **`harvey-mkpaqfs` uses the flake input path, trees use the FOD.** mkpaqfs is a host build (never dispatched), so the Mac-fetched source is the right input; Linux trees use `fetchFromGitHub` keyed on rev + narHash so the source never round-trips through APFS (D-24).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] plan9port's `ls` shadowed coreutils and the copy guard silently exported an empty tree**
- **Found during:** Task 1, inspecting the first build's output path (the build exited 0 and the case assertions passed on an empty directory)
- **Issue:** `hostbin` puts plan9port's `ls` first on PATH (it is in setup.sh's tool list, and `$HOSTBIN` precedes the wrapper PATH), and plan9port's `ls` has no `-A`; `if [ -n "$(ls -A "$work/out" 2>/dev/null)" ]` therefore evaluated false and `cp -r` never ran. The first store path was a 96-byte empty directory.
- **Fix:** the guard is `find "$work/out" -mindepth 1 -print -quit`, with a comment; and the venue now fails loudly when the install step produced an empty tree.
- **Files modified:** `nix/harvey/build-env.nix`
- **Verification:** the rebuilt tree contains the full layout (8 smoke members, headers, netaddr.rc, stamp), verifies clean, and boots.
- **Committed in:** `1d66e2b` (the non-empty guard) — the copy-guard fix itself is in `1fa084a`

**2. [Rule 1 - Bug] The mkpaqfs usage probe archived `.` into its own redirect file**
- **Found during:** Task 1, the first `harvey-mkpaqfs` build
- **Issue:** `"$out/bin/mkpaqfs" > usage.txt 2>&1` with no arguments makes mkpaqfs archive `.` to stdout (there is no usage-on-no-args; `-o` is optional and the root defaults to `.`), so it read the growing `usage.txt` and failed with "file too big for blocksize".
- **Fix:** the probe bakes an empty root to a scratch file (`mkpaqfs -o probe.paq probe-root`) and checks the paq is non-empty.
- **Files modified:** `nix/harvey/mkpaqfs.nix`
- **Verification:** both host builds print `mkpaqfs: bake probe ok`.
- **Committed in:** `1fa084a`

**3. [Rule 1 - Bug] plan9port's `9c` needs `PLAN9` exported**
- **Found during:** Task 1, the first `harvey-mkpaqfs` build (`9c: line 60: -v: command not found`)
- **Issue:** `9c` sources `$PLAN9/config` for its `egrep`/`uniq` variables; with `PLAN9` unset the warning-filter pipeline ran with empty commands.
- **Fix:** `export PLAN9="${plan9}"` in the mkpaqfs build (the venue already exports it).
- **Files modified:** `nix/harvey/mkpaqfs.nix`
- **Verification:** the 9c compile and 9l link run clean.
- **Committed in:** `1fa084a`

**4. [Rule 2 - Missing Critical] `binutils` on mkpaqfs's PATH for 9l's autolib**
- **Found during:** Task 1, the first successful mkpaqfs build (two `nm: command not found` warnings from 9l)
- **Issue:** plan9port's `9l` calls bare `nm` on host ELF objects; the derivation's PATH had no `nm`, so autolib resolution was degraded (the link happened to succeed through explicit `-l` flags).
- **Fix:** `pkgs.binutils` in `nativeBuildInputs` (the same PATH discipline `mkpaqfs-host.sh` inherits from the devshell).
- **Files modified:** `nix/harvey/mkpaqfs.nix`
- **Verification:** rebuild prints no `nm: command not found`; the output path is unchanged (identical bytes).
- **Committed in:** `1fa084a`

**5. [Rule 1 - Bug] The devshell banner contaminated `nix develop -c` stdout**
- **Found during:** Task 1's LAZY-PATH-MATCH verification
- **Issue:** the shellHook's four `echo` lines go to stdout, so `$(nix develop -c sh -c 'printf %s "$R9_HARVEY_DIR_ARM64"')` returned the banner plus the path and the plan's literal equality check could not pass.
- **Fix:** the banner writes to stderr (`>&2`), so the command's stdout is exactly its output; the information is unchanged for a human entering the shell.
- **Files modified:** `flake.nix`
- **Verification:** the exact plan command prints LAZY-PATH-MATCH; the banner still appears (on stderr).
- **Committed in:** `1fa084a`

**6. [Rule 1 - Documentation bug] HowItWorks.md still claimed a `../harvey` default**
- **Found during:** Task 2 (the trap-frame section, above the rewritten section)
- **Issue:** the sentence said `cargo xtask ureg-check` parses `$R9_HARVEY_DIR, default ../harvey` — a default 01-08 deleted (D-01).
- **Fix:** the sentence now names the one resolver and the no-default rule, linking to the rewritten section.
- **Files modified:** `HowItWorks.md`
- **Verification:** `grep -c 'default \`../harvey\`' HowItWorks.md` = 0.
- **Committed in:** `8a778f3`

---

**Total deviations:** 6 auto-fixed (5 bugs, 1 missing-critical). **Impact:** four of the six were needed to make the new derivations work at all (PLAN9, binutils, the copy guard, the empty-tree guard); the mkpaqfs probe and the devshell stream fix protect the verification itself; none changes the plan's architecture or scope.

## Issues Encountered

- **The first tree build exported an empty store path and exited 0.** The case assertions cannot see emptiness, so the discovery came from inspecting the path, not from the build. The empty-tree guard and the `find` copy guard close it; recorded as deviations 1 and the fix commit.
- **The standalone `cargo xtask ureg-check` was not run.** It checks both architectures; the amd64 tree is plan 01-14's deliverable and does not exist yet, so the amd64 arm would fail with the named error by design. The arm64 arm ran through `boot-test`'s precondition (it parses `arm64/include/ureg.h` from the flake tree) and the boot passed. 01-14's task 2 runs the standalone both-arch check.
- **`nix build` warnings about the Git tree having uncommitted changes** are expected mid-task noise; they do not affect the evaluated or built paths (the tree path was identical from `nix eval` and after `nix build`).

## Known Stubs

None. The arm64 tree carries exactly the smoke slice the tracer needs; the full amd64 and arm64 trees are 01-14/01-17's planned expansion of the same `mkTreeDerivation`, not stubs in this plan.

## Threat Flags

None beyond the plan's threat model. The implemented surface matches its mitigations: T-01-43 (builder-side fixed-output fetch keyed on rev + narHash; no APFS round-trip), T-01-44 (case-fold duplicate count 0, case-hack count 0, only the listed outputs installed), T-01-45 (store path read-only; `nix store verify --no-trust` clean; the build runs in a `$TMPDIR` copy), T-01-46 (SANDBOX-EVIDENCE block and the `/usr/bin/env` refusal), T-01-SC (no npm/pip/cargo installs; proot/plan9port/binutils from the locked nixpkgs).

## Next Phase Readiness

- **Plan 01-14 can build `harvey-tree-amd64` on the same machinery:** `mkTreeDerivation { name, arch = "amd64", guestScript, installScript }` is in place, the hostbin is cached on the builder, the S4 dispatch kinds have their venue ports, and the case/empty assertions are generic.
- **The devshell already exports `R9_HARVEY_TREE_SYSTEM` and the lazy `R9_HARVEY_DIR_ARM64`**; 01-14 adds `R9_HARVEY_DIR_AMD64` and `R9_AUTHDOMAIN_GEN` beside them. `harvey-mkpaqfs` is on all three systems and `R9_MKPAQFS` names it.
- **The `STAGE_SRCS` sha256 is the pin-bump tripwire.** A Harvey bump that changes the smoke list fails the tree build with the recorded value and the new line printed; update the constant at the bump.
- **Kernel bytes and Harvey are untouched:** both ELF sha256 values equal the 01-08 baseline after the flake change; Harvey HEAD `25d97dc4d`, status hash `041d2929…`; zero `port/src` changes; no push.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/harvey/build-env.nix`, `nix/harvey/tree-arm64.nix`, `nix/harvey/mkpaqfs.nix`, `01-13-SUMMARY.md` — all FOUND; `flake.nix`, `HowItWorks.md`, `docs/harvey-supply.md` FOUND.
- Task commits `1fa084a`, `8a778f3`, `1d66e2b` are ancestors of HEAD; `git rev-list --count cda27d9..HEAD` = 3 (measured against the plan ledger).
- Task 1 acceptance re-run: `grep -v '^[[:space:]]*#' nix/harvey/build-env.nix | grep -c unshare` = 0; `fetchFromGitHub` total in build-env.nix + flake.nix = 1; tree duplicate-name count 0 and case-hack count 0; `grep -c 'export R9_HARVEY_DIR=' flake.nix` = 0; virtexec boot PASS with nothing set.
- Task 2 acceptance re-run: CONTRACT-DOC-OK; `harvey-tree-arm64` count in docs/harvey-supply.md = 6; `R9_HARVEY_DIR_ARM64` count in HowItWorks.md = 2.
- Plan verification re-run: kernel ELF sha256 `76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2` (aarch64 virtshell) and `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f` (x86_64 q35churnshell), identical to the 01-08 baseline after the flake change. Note: the boot-test rebuilds `target/aarch64-unknown-none-elf/debug/aarch64` with the `virtexec` config at the same path (Pitfall 7's shared artifact path), so the final check rebuilds `virshell` first; both hashes then reproduce exactly.
- `cargo xtask test` (87 passed), `clippy` and `check` green after Task 2; `nix flake show --all-systems` exit 0.
- Harvey untouched: HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`; zero `port/src` changes; no push.
