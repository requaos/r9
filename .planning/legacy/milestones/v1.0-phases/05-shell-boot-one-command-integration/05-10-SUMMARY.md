---
phase: 05-shell-boot-one-command-integration
plan: 10
subsystem: infra
tags: [r9, nix, flake, packages.kernel-virt, importCargoLock, build-std, xtask, R9_HARVEY_DIR, D-10, D-11, flake-composition, INT-04]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-09)
    provides: the complete kernel surface the virtshell profile builds (the package's payload) and the proven staged-profile fan this plan re-ran as the D-12 regression gate
  - phase: 03-r9-qemu-virt-platform-bring-up
    provides: r9's flake (toolchainFor/fromRustupToolchainFile single-source rule, objcopyExport plumbing, apps block) and the committed aarch64/lib/virt-gicv2.dtb
provides:
  - r9 flake output packages.kernel-virt: a sandboxed derivation that builds the shell-capable aarch64 kernel ELF (cargo xtask build --arch aarch64 --config virtshell, debug profile) and installs it plus the committed virt-gicv2.dtb — the ONLY artifact Harvey's 05-11 compose app consumes (D-11)
  - the D-10 inversion r9-side: xtask's initrd subcommand, boot-test's initrd auto-bake (ensure_initrd_image), and ureg-check all REQUIRE R9_HARVEY_DIR — unset is a named error; no sibling-`../harvey` default remains anywhere in xtask
  - proven-green r9-side gates under the explicit contract: ureg-check 35/35 + all five staged profiles (virtexec/virtmulti/virtnotes/virt/virtpanic) with R9_HARVEY_DIR set
affects: [05-11 (Harvey's inputs.r9 → packages.kernel-virt consumption), 05-12/05-13 (the smoke/CI consume the composed boot), any future r9 flake consumer]

# Actuals (#2632) — pairs with the plan's estimate (60000 tokens, confidence: low).
actuals:
  tokens: 6200     # MEASURED chars/4: r9 diff 10775 chars (69f4843..9549cee) + this SUMMARY ~14000 = ~24775/4 ≈ 6200; the estimate overshot ~10x (confidence was low; the two-lockfile vendoring wrinkle was the only surprise)
  tasks: 2
  commits: 2       # MEASURED: r9 git rev-list --count 69f4843..9549cee = 2; harvey 0 (pre-metadata; the docs commit rides on top, per the 05-04..09 convention)
plan_head_before: {r9: "69f4843767883df7bdc4ec5bc0a4ea47a94294a0", harvey: "956caeba8d20fe27f4fe8898fbdfce685d6f8869"}
plan_head_after: {r9: "9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6", harvey: "956caeba8d20fe27f4fe8898fbdfce685d6f8869 (pre-metadata; the docs commit rides on top)"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies anywhere (T-05-SC held: no package installs; the derivation builds existing locked cargo deps only)
  patterns:
    - "sandboxed cargo-xtask kernel builds on nix: toolchainFor (rust-toolchain.toml single source) + cargoSetupHook + importCargoLock over TWO lockfiles — the workspace's Cargo.lock AND the toolchain rust-src library/Cargo.lock (the -Z build-std sysroot graph has its own crates.io members, e.g. rustc-literal-escaper, that the project lock never names)"
    - "symlinkJoin of importCargoLock outputs must be named 'cargo-vendor-dir': the vendor outputs ship a .cargo/config.toml whose vendored-sources directory is that RELATIVE name, and cargoSetupHook prefers the shipped config — the joined dir must keep the name so the hook's copy lands where the config points"
    - "env-var contract inversion shape: one pub(crate) resolver (uregcheck::harvey_dir) returns Result and every consumer propagates; the unset error names the variable AND the required checkout contents"

key-files:
  created: []
  modified:
    - ../r9/flake.nix
    - ../r9/.gitignore
    - ../r9/xtask/src/main.rs
    - ../r9/xtask/src/uregcheck.rs

key-decisions:
  - "the kernel package builds the DEBUG profile (cargo xtask build --arch aarch64 --config virtshell, no --release) — the Phase-4/5 gate default the apps and boot-test harness use; the plan sanctioned matching it ('match the debug default the Phase-4 gates used unless release is already canonical' — no release canonical exists)"
  - "store-path layout (the 05-11 contract): $out/kernel-virt.elf (the built shell-capable kernel) + $out/aarch64/lib/virt-gicv2.dtb (the committed DTB at its repo-relative path — the config TOML's dtb key names exactly that string)"
  - "xtask inversion keeps the two-tier ureg-check shape for a SET-but-headerless dir (standalone skips, boot-test fails) — 'behavior when set is unchanged'; only the UNSET case hardened to the named R9_HARVEY_DIR error (the plan's pinned case)"
  - "config.rs needed no edit although listed in files_modified — the plan's read_first marked prepare_initrd_dtb 'GENERIC, stays'; it carries no Harvey-path knowledge"

patterns-established:
  - "flake package for a no_std kernel via cargo xtask inside stdenv.mkDerivation: cargoSetupHook vendoring + the toolchain's own llvm-objcopy export; the build honors the rust-toolchain.toml pin without restating it"

requirements-completed: [INT-04]  # checkbox marking is shared-ID-gated (#2388): 05-11 also declares INT-04 — requirements.ready-ids returned 0/1; the checkbox flips when 05-11 lands its SUMMARY

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "r9's flake exposes packages.kernel-virt (D-11): nix build .#kernel-virt succeeds on aarch64-darwin and the store output carries the built shell-capable aarch64 kernel ELF plus the committed virt-gicv2.dtb; the toolchain comes from rust-toolchain.toml via toolchainFor/fromRustupToolchainFile (never a restated pin) and devShells/apps behavior is unchanged"
    requirement: INT-04
    verification:
      - kind: integration
        ref: "cd ../r9 && nix build .#kernel-virt (run twice: on the working tree and again on the final committed tree — both green; result/kernel-virt.elf = ELF 64-bit LSB ARM aarch64 statically linked, result/aarch64/lib/virt-gicv2.dtb present)"
        status: pass
      - kind: other
        ref: "nix develop -c cargo xtask build --arch aarch64 --config virtshell green in the devshell; nix flake show lists packages.aarch64-darwin.kernel-virt; grep confirms fromRustupToolchainFile in flake.nix"
        status: pass
    human_judgment: false
  - id: D2
    description: "the D-10 r9-side inversion: ureg-check, the initrd subcommand, and boot-test's initrd auto-bake all REQUIRE R9_HARVEY_DIR — unset, each fails with an error naming the variable and the required checkout contents; no sibling-../harvey default remains anywhere in xtask; with the var set the r9-side gates stay fully green (D-12)"
    requirement: INT-04
    verification:
      - kind: integration
        ref: "unset legs: cargo run -p xtask -- ureg-check and cargo xtask initrd --arch aarch64 --config virtexec both FAIL, each log greps R9_HARVEY_DIR (/tmp/ureg-noenv.log, /tmp/initrd-noenv.log)"
        status: pass
      - kind: e2e
        ref: "set leg: ureg-check 35/35 + boot-test virtexec/virtmulti/virtnotes/virt/virtpanic ALL PASS (target/boot-test-*.log fresh captures; zero qemu orphans after)"
        status: pass
      - kind: unit
        ref: "cargo test -p xtask (19 tests) and cargo test -p port (all suites) green; cargo clippy -p xtask clean; r9 CI (.github/workflows/xtask.yml) untouched — it invokes neither ureg-check nor boot-test (verified by reading it)"
        status: pass
    human_judgment: false

# Metrics
duration: 10 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 10: r9 flake kernel-virt package + the xtask inversion Summary

**r9 is now a clean flake input: `packages.kernel-virt` builds the shell-capable aarch64 kernel ELF in a sandboxed derivation (cargo xtask build --arch aarch64 --config virtshell, debug profile, toolchain single-sourced from rust-toolchain.toml, deps vendored from both Cargo.lock and the rust-src sysroot lockfile so `-Z build-std` resolves offline) and ships `kernel-virt.elf` + `aarch64/lib/virt-gicv2.dtb` as store outputs — and xtask's implicit Harvey-path glue is gone per D-10: ureg-check, `initrd`, and boot-test's auto-bake all require R9_HARVEY_DIR explicitly, failing with a named error when unset, with every r9-side regression gate (ureg-check 35/35, all five staged profiles, both host test suites, clippy) green when the variable is set.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-11-03T15:15:29Z
- **Completed:** 2026-11-03T15:25:26Z
- **Tasks:** 2
- **Files modified:** 4 (r9: flake.nix, .gitignore, xtask/src/main.rs, xtask/src/uregcheck.rs; harvey: planning artifacts only)

## Accomplishments

- **Task 1: `packages.kernel-virt` in r9's flake** (r9 `b7ba00f`) — the D-11 kernel-only artifact. A `stdenv.mkDerivation` per system runs `cargo xtask build --arch aarch64 --config virtshell` (debug — the gate default) under the `toolchainFor`-pinned toolchain with `objcopyExport` plumbing, and installs the two D-11 outputs. The one wrinkle was `-Z build-std`: the sysroot graph (rust's library workspace) has its own crates.io members (`rustc-literal-escaper` et al.) that r9's Cargo.lock never names, so the vendored set covers BOTH lockfiles via a `symlinkJoin` of two `importCargoLock` outputs — named `cargo-vendor-dir` because the vendor outputs' shipped `.cargo/config.toml` references that relative name and `cargoSetupHook` prefers the shipped config. `nix build .#kernel-virt` is green on this host (twice, including once on the final committed tree); `nix flake show` exposes the package; the devshell build and apps are byte-identical in behavior.
- **Task 2: the xtask inversion** (r9 `9549cee`) — D-10 made real. `uregcheck::harvey_dir()` now returns `Result<PathBuf>`: an unset or empty R9_HARVEY_DIR is a named error (it names the variable AND what the checkout must contain — `aarch64/include/ureg.h`, plus `build/aarch64/out/` + `.build/bin/mkpaqfs` for the bake); the `../harvey` fallback is deleted. `assemble_initrd` and `ensure_initrd_image` propagate the requirement, so boot-test profiles consuming `target/bootfs.paq` inherit it. Help texts and comments carry the 05-CONTEXT D-10 provenance where the default lived. Behavior with the variable set is unchanged — the staged-gate bake (RESEARCH OQ-2's lowest-risk answer) keeps working against the untouched `build/aarch64/stage-build.sh` supply chain.
- **The D-12 gates hold under the explicit contract:** with R9_HARVEY_DIR set, ureg-check reports 35/35 and all five staged profiles (virtexec, virtmulti, virtnotes, virt, virtpanic) PASS with fresh captures and zero qemu orphans; host suites (xtask 19 tests, port all suites) and clippy are green; r9's CI needs no change (it runs neither ureg-check nor boot-test — verified by reading xtask.yml).

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9` via `git -C` from the pinned harvey root; no harvey code changes this plan):

1. **Task 1: flake packages.kernel-virt — the D-11 kernel-only artifact** — r9 `b7ba00f` (feat; flake.nix + .gitignore)
2. **Task 2: xtask inversion — R9_HARVEY_DIR required, no sibling defaults (D-10)** — r9 `9549cee` (feat; xtask/src/main.rs + xtask/src/uregcheck.rs)

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP)

## Files Created/Modified

- `../r9/flake.nix` — the new `packages = forAllSystems` block with `kernel-virt` (devShells/apps untouched)
- `../r9/.gitignore` — `/result` (the nix build symlink the package flow produces)
- `../r9/xtask/src/uregcheck.rs` — `harvey_dir()` returns Result with the named R9_HARVEY_DIR error; module header + run_check/check_for_boot_test docs updated; unused `workspace` import dropped
- `../r9/xtask/src/main.rs` — both `harvey_dir()` call sites propagate (`?`) with a D-10 provenance comment; the `ureg-check` and `initrd` help texts now say "required"

## The inversion, before/after (D-10)

| xtask path | Before (Phase 4 glue) | After (05-10) |
|---|---|---|
| `cargo xtask ureg-check`, var unset | resolved `../harvey` silently; skip+exit-0 if the header was absent | hard error naming R9_HARVEY_DIR and the required contents |
| `cargo xtask initrd`, var unset | baked from `../harvey/build/aarch64/out/` | hard error naming R9_HARVEY_DIR |
| boot-test auto-bake (`ensure_initrd_image`), var unset | baked from `../harvey/...` | hard error (via `harvey_dir`); `check_for_boot_test` fails first with the updated message |
| any of the above, var set | used the variable | **unchanged** |
| `cargo xtask build` / r9 CI | never touched Harvey paths | unchanged |

## The package's store-path layout (the 05-11 consumption contract)

```
/nix/store/<hash>-r9-kernel-virt-<shortRev>/
├── kernel-virt.elf                  # the built shell-capable aarch64 kernel ELF (debug profile)
└── aarch64/lib/virt-gicv2.dtb       # the committed DTB, at its repo-relative path
```

Harvey's `inputs.r9.packages.<system>.kernel-virt` supplies exactly these two paths; Harvey bakes its own initrd (its own mkpaqfs) and launches qemu itself — r9's xtask is never invoked Harvey-side (D-11).

## Decisions Made

- **Debug profile for the package** — no release canonical exists in the gates; the plan sanctioned matching the debug default. Recorded per the plan's action text.
- **Store layout** (`kernel-virt.elf` + repo-relative DTB path) — chosen so the config TOML's `dtb` string stays directly meaningful to Harvey's launcher documentation.
- **Set-but-headerless keeps the two-tier shape** (standalone skip / boot-test fail) — the plan pinned only the unset case ("behavior when set is unchanged").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] the sandboxed `nix build` died offline resolving the `-Z build-std` sysroot graph**
- **Found during:** Task 1 verify (`nix build .#kernel-virt` → "no matching package named `rustc-literal-escaper` found … required by package `proc_macro` (rust-src library)")
- **Issue:** rust's library workspace (built by `-Z build-std=core,alloc`) has crates.io members pinned by the toolchain's OWN `library/Cargo.lock`, which r9's workspace lockfile never names — vendoring only r9's lock leaves the sysroot graph unresolvable offline
- **Fix:** two-lockfile vendoring — `symlinkJoin` of `importCargoLock` over `./Cargo.lock` AND `${toolchain}/lib/rustlib/src/rust/library/Cargo.lock` (the plan sanctioned exactly this latitude: "if sandbox cargo offline is an issue, pkgs.rustPlatform.importCargoLock or equivalent — the executor picks the idiomatic nix-rust shape and documents it"). A second failure surfaced the load-bearing name: the vendor outputs ship a `.cargo/config.toml` pointing at the RELATIVE `cargo-vendor-dir`, and `cargoSetupHook` prefers the shipped config — the joined output must be named `cargo-vendor-dir` so the hook's copy lands where the config points (documented in the flake comment)
- **Files modified:** `../r9/flake.nix`
- **Verification:** `nix build .#kernel-virt` green (twice, incl. the final committed tree); both D-11 artifacts in `result/`
- **Committed in:** `b7ba00f`

**2. [Rule 2 - Hygiene] `/result` not gitignored in r9**
- **Found during:** Task 1 commit staging (`git status` showed the nix build symlink untracked)
- **Issue:** the package build flow this task introduces produces a `result` symlink at the repo root; r9's .gitignore covered only `/target`
- **Fix:** added `/result` to `../r9/.gitignore` (generated/runtime output handling per the commit protocol)
- **Files modified:** `../r9/.gitignore`
- **Verification:** post-commit `git status` clean with the symlink present
- **Committed in:** `b7ba00f`

**3. [Process note] config.rs listed in files_modified but untouched**
- The plan's `files_modified` and Task 2 `<files>` name `../r9/xtask/src/config.rs`, but its own read_first marks `prepare_initrd_dtb` "GENERIC, stays" — the file carries no Harvey-path knowledge, so the inversion required no edit. Recorded to explain the delta between the plan's file list and the realized diff.

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 hygiene) + 1 process note
**Impact on plan:** Both auto-fixes are inside the plan's sanctioned executor latitude (the vendoring shape) or standard commit hygiene (the gitignore). No scope creep — r9's xtask surface is otherwise exactly the plan's.

## Issues Encountered

None beyond the deviations above. (The five-profile fan ran clean on the first pass — the 05-09 input-wedge flake did not reproduce; these staged profiles boot to all-exited rather than driving the interactive console, so they don't exercise that path.)

## User Setup Required

None - no external service configuration required. (Consumers set `R9_HARVEY_DIR` explicitly; the package itself consumes nothing external.)

## Known Stubs

None. This plan ships build tooling only — no stubs, placeholders, or unwired data paths. (`../r9/xtask/src/config.rs` untouched per the plan's "GENERIC, stays" note — see Deviations process note.)

## Threat Flags

None beyond the plan's register. (T-05-19 held: the derivation builds from the locked flake source with the rust-toolchain.toml pin and Cargo.lock-vendored deps — the second vendored lockfile is the pinned toolchain's own rust-src component, not a new external input; the committed DTB is copied from the tracked tree. T-05-SC held: zero package installs.)

## Next Phase Readiness

- **05-11 (Harvey-side composition)** is unblocked: `inputs.r9.packages.<system>.kernel-virt` now exists with the documented two-path layout; r9's xtask contract is explicit (R9_HARVEY_DIR), so Harvey's compose app consumes store paths only, per D-11. INT-04's checkbox stays pending until 05-11 lands (shared-ID gate #2388).
- **05-12/05-13** inherit a clean flake input and unchanged r9-side gates (the five-profile fan + ureg-check green with the explicit variable).
- r9 HEAD: `9549cee` (main). Harvey HEAD: `956caeba8` + the metadata commit.

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: `../r9/flake.nix` (packages block present — proven by `nix build .#kernel-virt` + `nix flake show`), `../r9/.gitignore` (`/result`), `../r9/xtask/src/main.rs` + `../r9/xtask/src/uregcheck.rs` (the inversion — proven by the unset/set verify legs) — all FOUND on disk in the committed state.
- Commits: r9 `b7ba00f` (Task 1) and `9549cee` (Task 2) — both FOUND in `git -C ../r9 log`; no harvey code commits this plan (metadata commit follows).
- Plan-level <verification> re-run at final state: (1) `nix build .#kernel-virt` on the committed tree → `result/kernel-virt.elf` (ELF aarch64) + `result/aarch64/lib/virt-gicv2.dtb` present; (2) unset-env legs fail naming R9_HARVEY_DIR (ureg-check + initrd); (3) set-env legs fully green — ureg-check 35/35 + virtexec/virtmulti/virtnotes/virt/virtpanic all PASS, no qemu orphans; (4) host suites (xtask, port) + clippy green.
- Acceptance criteria re-run at final state: Task 1 (package produces ELF + DTB on aarch64-darwin; toolchain via toolchainFor/fromRustupToolchainFile; devShells/apps behavior unchanged) — PASS. Task 2 (every former `../harvey` default now requires R9_HARVEY_DIR with a named error; gates green with it set; r9 CI untouched and valid) — PASS.
