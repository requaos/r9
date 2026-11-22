---
phase: 03-r9-qemu-virt-platform-bring-up
plan: "01"
subsystem: infra
tags: [nix, flake, rust-overlay, qemu, dtc, aarch64, r9, devshell, toolchain-bootstrap]

requires: []
provides:
  - "../r9/flake.nix — devShells.default (pinned rust nightly via rust-overlay on rust-toolchain.toml + qemu + dtc) and apps.build/apps.default wrapping cargo xtask build --arch aarch64 with embedded runtimeInputs"
  - "../r9/flake.lock — pinned nixpkgs 1c89083 (2026-10-30) + rust-overlay c85ccd3 (2026-11-01), reproducible on any host"
  - "The phase's toolchain bootstrap: every later plan in phase 03 builds through this devshell"
affects: [03-02, 03-03, 03-04, 03-05, 03-06, phase-05-int-04]

# Actuals (#2632) — same scale as the plan's estimateTokens (chars/4 over realized diff).
# Cross-repo plan: commits = 1 r9 code commit + 1 harvey docs commit.
# r9 leg: 33a9273 -> ae4cd08 (flake.nix + flake.lock). Harvey leg measured from ledger below.
actuals:
  tokens: 1216
  tasks: 1
  commits: 2

# Commit ledger (#3968) — cross-repo plan: harvey base measured from the persisted
# ledger; the r9 leg is 33a9273..ae4cd08 (1 commit). commits: 2 = r9 ae4cd08 + this
# harvey docs commit. plan_head_after names the pre-amend docs hash; the amend only
# added these ledger fields (content otherwise identical).
plan_head_before: 9b9bc3602ccd8fbf00ba5f4bb7622ff2eb8034f0
plan_head_after: 9d7a1f2e0bc9557f096676663df506d4944b8a52

tech-stack:
  added: [oxalica/rust-overlay (flake input), nix flake in r9]
  patterns:
    - "fromRustupToolchainFile ./rust-toolchain.toml — the toml stays the single pin source; nix never restates channel/components/targets (D-09)"
    - "writeShellApplication with runtimeInputs for flake apps — apps run bare with zero host setup (Harvey flake convention, D-08)"
    - "nix devshell OBJCOPY defaulting: absolute-path values count as user overrides; nixpkgs' bintools placeholder (bare 'objcopy') does not"

key-files:
  created:
    - "../r9/flake.nix"
    - "../r9/flake.lock"
  modified: []

key-decisions:
  - "rust-overlay chosen over fenix as the D-09 overlay (research recommendation; fromRustupToolchainFile keeps rust-toolchain.toml authoritative)"
  - "Devshell OBJCOPY guard treats only absolute-path pre-set values as user overrides — nixpkgs' bintools wrapper unconditionally exports a bare-name OBJCOPY=objcopy before the shellHook, which is a placeholder, not an override"
  - "mkShellNoCC for the devshell (no C compiler needed: no build.rs, no cc deps) — partial mitigation only; bintools rides stdenv regardless, so the case-guard is the load-bearing fix"
  - "Toolchain-identity probe anchored to the nix store path (rust-default-1.95.0-nightly-2026-01-30) because rustc --version reports the rustc source-commit date (2026-01-29), not the manifest date"

requirements-completed: [R9-05]

coverage:
  - id: D1
    description: "r9 flake devshell + apps.build reproduce the raspi3b kernel on darwin-aarch64 with zero host setup (nix develop + cargo xtask build; nix run .#build bare)"
    requirement: R9-05
    verification:
      - kind: integration
        ref: "cd ../r9 && nix develop -c bash -c 'cargo xtask build --arch aarch64' → Finished dev profile; ELF at target/aarch64-unknown-none-elf/debug/aarch64 (10.5s cold build incl. build-std)"
        status: pass
      - kind: integration
        ref: "cd ../r9 && rm target/.../aarch64 && nix run .#build → exit 0, ELF rebuilt (runtimeInputs-embedded toolchain+qemu, no devshell)"
        status: pass
      - kind: other
        ref: "static gate: fromRustupToolchainFile/oxalica/system-tuples/devShells/apps greps, no Intel-Mac tuple, no restated pins, flake.lock non-empty with both inputs pinned"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-10 linux leg: nix develop + cargo xtask build --arch aarch64 and nix run .#build on a linux host (aarch64/x86_64) from r9 commit ae4cd08"
    requirement: R9-05
    verification: []
    human_judgment: true
    rationale: "D-10 is manual host verification by decision; harvested into end-of-phase UAT. Reference artifact: r9 commit ae4cd08a5cec69a0ba5a999bea6a00d6279e4eb2; expect rustc 1.95.0-nightly (842bd5be2 2026-01-29) from rust-default-1.95.0-nightly-2026-01-30, QEMU 11.1.2, DTC 1.8.1; flake.lock pins nixpkgs 1c89083 + rust-overlay c85ccd3."

duration: 13 min
completed: 2026-11-01
status: complete
---

# Phase 03 Plan 01: r9 flake (devshell + apps.build) Summary

**r9 now carries its own nix flake: a rust-overlay devshell honoring rust-toolchain.toml (nightly-2026-01-30 + llvm-tools, qemu, dtc, OBJCOPY wired to the toolchain's llvm-objcopy) plus a runtimeInputs-embedded `apps.build` — the raspi3b kernel ELF builds on darwin-aarch64 both inside `nix develop` and bare via `nix run .#build`, with zero host setup.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-01T22:18:54Z
- **Completed:** 2026-11-01T22:31:50Z
- **Tasks:** 1
- **Files modified:** 2 (both created, in sibling repo `../r9`)

## Accomplishments

- `../r9/flake.nix`: `devShells.default` (rust toolchain via `pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` + `pkgs.qemu` + `pkgs.dtc`, informative shellHook) and `apps.build`/`apps.default` (`writeShellApplication` `r9-build` with `runtimeInputs = [ toolchain qemu ]`, execs `cargo xtask build --arch aarch64 "$@"`). Systems: aarch64-darwin, aarch64-linux, x86_64-linux — no Intel-Mac tuple, no `packages.*` outputs, no `.github/workflows` changes.
- `../r9/flake.lock`: nixpkgs `1c89083e451e891152365b799595cf7790a43325` (2026-10-30), rust-overlay `c85ccd343087c57da6021cafa1647142ad03e32e` (2026-11-01), `rust-overlay/nixpkgs` follows the project nixpkgs (single lineage).
- **Assumption A1 CONFIRMED empirically:** rust-overlay resolves dated nightly-2026-01-30 with all components (rustfmt/rust-src/clippy/llvm-tools) and all three bare-metal targets on aarch64-darwin; `llvm-objcopy` lands at `lib/rustlib/<host>/bin/llvm-objcopy` as assumed. No fenix/explicit-list fallback needed.
- Darwin proof (this host): devshell `cargo xtask build --arch aarch64` cold build (incl. `-Z build-std=core,alloc` from the overlay's rust-src) finished in 10.5s → `ELF 64-bit LSB executable, ARM aarch64, statically linked, with debug_info` at `target/aarch64-unknown-none-elf/debug/aarch64`; bare `nix run .#build` (ELF deleted first) exit 0 and rebuilt the artifact.
- D-10 reference artifact for the linux leg: r9 commit `ae4cd08a5cec69a0ba5a999bea6a00d6279e4eb2`; toolchain versions `rustc 1.95.0-nightly (842bd5be2 2026-01-29)` (store derivation `rust-default-1.95.0-nightly-2026-01-30`), `QEMU emulator version 11.1.2`, `DTC 1.8.1`.

## Task Commits

1. **Task 1: r9 flake.nix + flake.lock** — `ae4cd08` (feat, repo: r9, branch main) — `feat(03-01): add flake.nix devshell and apps.build`

**Plan metadata:** harvey docs commit (see below; `docs(03-01): complete r9 flake plan`)

## Files Created/Modified

- `../r9/flake.nix` (created) — devShells.default + apps.build/default; rust-overlay honoring rust-toolchain.toml; OBJCOPY wiring
- `../r9/flake.lock` (created) — pinned nixpkgs + rust-overlay revisions

(Harvey repo: only this SUMMARY + state files, per D-13.)

## Decisions Made

- **Overlay: rust-overlay** (over fenix) — research recommendation; `fromRustupToolchainFile` is the D-09 mechanism and resolved cleanly (A1).
- **OBJCOPY override semantics in the devshell:** only absolute-path pre-set values are honored as user overrides. nixpkgs' bintools-wrapper setup hook *unconditionally* exports `OBJCOPY=objcopy` (bare name) before any shellHook runs — it is a stdenv placeholder (it even clobbers a user's pre-export), so a plain `${OBJCOPY:-…}` guard would ship a dangling OBJCOPY that xtask would exec and fail on. The case-guard distinguishes the two.
- **mkShellNoCC** for the devshell — no `build.rs`/`cc` deps anywhere in the workspace, so no C compiler is needed. (Noted: bintools still rides stdenv on darwin even with NoCC; the case-guard above is what actually fixes OBJCOPY.)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Comment named the forbidden Intel-Mac tuple**
- **Found during:** Task 1, verify gate 1 (static greps)
- **Issue:** the plan's gate requires `grep -c 'x86_64-darwin' flake.nix` to be exactly 0; my explanatory comment ("x86_64-darwin deliberately omitted") contained the literal string, failing the gate on a file that was semantically correct
- **Fix:** reworded the comment to "Intel-Mac tuple deliberately omitted" — the systems list itself was always correct
- **Files modified:** ../r9/flake.nix
- **Verification:** gate 1 passes
- **Committed in:** `ae4cd08`

**2. [Rule 1 - Bug] Devshell OBJCOPY defeated by nixpkgs bintools placeholder**
- **Found during:** Task 1, verify gate 2 (`test -n "$OBJCOPY"` passed but the value was bare `objcopy`, not the toolchain's llvm-objcopy)
- **Issue:** nixpkgs' bintools-wrapper setup hook (present in stdenv even under `mkShellNoCC` — stdenv keeps ar/strip for fixup) unconditionally `export OBJCOPY=objcopy` before the shellHook; the plan's "default only when unset" logic then preserved a placeholder that resolves to nothing useful for kernel objcopy work
- **Fix:** shellHook now treats only absolute-path OBJCOPY values as user overrides (`case "$OBJCOPY" in /*) keep ;; *) default to toolchain llvm-objcopy ;; esac`); also switched to `mkShellNoCC` since the workspace needs no C compiler
- **Files modified:** ../r9/flake.nix
- **Verification:** `nix develop` yields `OBJCOPY=/nix/store/…-rust-default-1.95.0-nightly-2026-01-30/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy`, executable, reports "llvm-objcopy, compatible with GNU objcopy"
- **Committed in:** `ae4cd08`

**3. [Rule 1 - Bug] writeShellApplication shellcheck rejected the ls-glob idiom**
- **Found during:** Task 1, verify gate 3 (`nix run .#build` — app derivation failed to build)
- **Issue:** `writeShellApplication`'s checkPhase runs shellcheck; SC2012 flags `ls <glob> | head -1`, aborting the `r9-build` derivation before the app could ever run
- **Fix:** app text uses a pure-bash glob loop (`for _r9_c in <toolchain>/lib/rustlib/*/bin/llvm-objcopy; do [ -e … ] && break; done`) — same rustlib glob, no external `ls`, no shellcheck trigger
- **Files modified:** ../r9/flake.nix
- **Verification:** `nix run .#build` exit 0; generated script inspected — PATH embeds toolchain + qemu-11.1.2 store paths, OBJCOPY default loop present
- **Committed in:** `ae4cd08`

**4. [Rule 1 - Bug] Plan gate's toolchain-identity probe unsatisfiable by construction**
- **Found during:** Task 1, verify gate 2 (`rustc --version | grep -q "nightly-2026-01-30"`)
- **Issue:** `rustc --version` prints the rustc *source-commit* date — `rustc 1.95.0-nightly (842bd5be2 2026-01-29)` — never the manifest date; the correct pinned toolchain can never pass that grep
- **Fix:** proved the intent with a stronger probe: `which rustc` resolves to `/nix/store/xdba9fkgy07iclisp1cdrf1bwqss81r3-rust-default-1.95.0-nightly-2026-01-30/bin/rustc` — the derivation rust-overlay built FROM `./rust-toolchain.toml` (gate 1 proves `fromRustupToolchainFile`), so the store path names the exact pinned nightly. All other gate-2 legs (qemu grep, dtc, OBJCOPY non-empty, cargo xtask build, ELF artifact) pass unchanged
- **Files modified:** none (verification-only correction; recorded here per the plan's "verify A1 empirically and record the result" instruction)
- **Verification:** store-path identity check passes; `rustc --version` recorded verbatim above for the D-10 linux leg
- **Committed in:** n/a (no code change)

---

**Total deviations:** 4 auto-fixed (all Rule 1 — three implementation bugs in my own flake, one unsatisfiable gate probe corrected to a stronger identity proof)
**Impact on plan:** No scope creep; all fixes were required for the gates to prove the plan's actual intent. The OBJCOPY findings are recorded as key decisions because later plans (and any future nix work in either repo) will hit the same bintools-placeholder behavior.

## Issues Encountered

None beyond the deviations above. First `nix flake lock` attempt failed with "Path 'flake.nix' … is not tracked by Git" — standard nix-flakes-in-git semantics (flakes only see tracked files); staged `flake.nix` and re-ran. Not a deviation, just nix mechanics.

## User Setup Required

None — the point of R9-05 is zero host setup beyond nix itself. `cd ../r9 && nix develop` (or `nix run .#build`) on any supported host.

## Next Phase Readiness

- **Plans 03-02 through 03-06** inherit a working devshell as their build environment (host cargo/rustup absent per RESEARCH.md — this was the phase bootstrap).
- **D-10 linux leg queued for end-of-phase UAT** (see coverage D2): run the SC#1 pair by hand on a linux host from r9 commit `ae4cd08a5cec69a0ba5a999bea6a00d6279e4eb2`.
- Ready for 03-02 (platform tree restructuring) — builds now run through `nix develop`.

---
*Phase: 03-r9-qemu-virt-platform-bring-up*
*Completed: 2026-11-01*

## Self-Check: PASSED

- FOUND: ../r9/flake.nix
- FOUND: ../r9/flake.lock
- FOUND: .planning/phases/03-r9-qemu-virt-platform-bring-up/03-01-SUMMARY.md
- FOUND: r9 commit ae4cd08 (`feat(03-01): add flake.nix devshell and apps.build`, branch main)
