---
phase: 01-stage-p0-cross-build-harness
plan: 15
subsystem: cross-build-harness
tags: [nix, harvey-reference, mkimage, member-list, manifests, amd64, marker, chain, substrate, d-15, d-16, d-29, harn-03, harn-05]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-14's flake-built amd64 tree (harvey-tree-amd64), the hostbin/proot venue and the 27-row tier-0 library table; 01-13's mkpaqfs host package and the lazy devshell-export pattern; 01-11's image engine (parse_reference_members, compare_members, emit_manifest, the `cargo xtask image` CLI)"

provides:
  - "nix/harvey/reference.nix: harvey-reference-amd64 — one fresh run of Harvey's own x86_64 mkimage.sh on a writable copy of the flake-built amd64 tree, recording all nine image contracts as <image>.members lists (path/mode/size/sha256, opaque for p9sk1 material) plus a password-redacted mkimage.log"
  - "flake.nix: packages.<linux>.harvey-reference-amd64 and the lazy, guarded R9_HARVEY_REFERENCE_AMD64 devshell export"
  - "nix/harvey/tree-amd64.nix: the three source-side inputs the bake reads — sys/include, build/x86_64/mkimage.sh and sys/src/cmd/auth/secstore/mkfile"
  - "xtask/manifests/amd64/{marker,chain,substrate}.toml: per-file manifests whose member lists equal the reference bake (30/49/27 members), with /amd64/bin/ps a per-file entry"

affects: [01-16, 01-19, 01-29, HARN-03, HARN-05, D-15, D-16, D-29]

actuals:
  tokens: 7403
  tasks: 2
  commits: 2

plan_head_before: c6b27cd6d64d53069fdc639c08733fbc04a9dffb
plan_head_after: b59e6ed37c61c4606edb853982a8eb1683f8ee77

tech-stack:
  added:
    - "harvey-reference-amd64 flake package (Linux)"
  patterns:
    - "A reference bake runs in a plain stdenvNoCC derivation: mkimage is a host shell bake and its two host-side 6c/6l builds need no Plan 9 namespace, so the proot venue is not required"
    - "The mkpaqfs shim records the staging tree as a member list BEFORE execing the real mkpaqfs, so the D-29 reference is exactly the bytes the bake fed the format writer"
    - "Auth paths are listed by path, size and mode only (`opaque`); the store output carries no image, no auth artifact and no password, and the build refuses if one appears"
    - "The tree's HARVEY-TREE stamp rev is asserted against the flake pin, so a reference can never be produced from a different tree than the manifests read (T-01-51)"

key-files:
  created:
    - nix/harvey/reference.nix
    - xtask/manifests/amd64/marker.toml
    - xtask/manifests/amd64/chain.toml
    - xtask/manifests/amd64/substrate.toml
  modified:
    - nix/harvey/tree-amd64.nix
    - flake.nix

key-decisions:
  - "The reference bake is a plain stdenvNoCC derivation, not mkTreeDerivation's proot venue: mkimage's services/auth host-side builds (§11k/§14i2/§14i3) ran with 6c/6l directly, so no Plan 9 namespace was needed (recorded as the plan requires)."
  - "harvey-hostbin is appended to PATH (never prepended: its plan9port sed/ls/date must not shadow the GNU tools the bake checks use) and exported as HARVEY_LINUX_BIN, because mkimage §11d reads $HARVEY_LINUX_BIN (or $HARVEY/.build/hostbin-linux-x86_64/bin) rather than PATH for its host builds."
  - "The member-recording shim writes the engine's reference format (`<path> file <mode> <size> <sha256>` / `<path> dir <mode> - -`, sorted, paths with no leading slash) — the format parse_reference_members reads — not the plan's `f ...` shorthand, which is the [[tree]] members-file format."
  - "The opaque set is image-scoped: adm/nvram, adm/keys, adm/domain.manifest, lib/ndb/auth, boot/authfeed.txt (auth) and boot/authid-creds.txt (cpu) always; lib/ndb/local only for auth and cpu (only they carry the @AUTHDOM@ row; services' local is generated but secret-free and keeps a real hash)."
  - "KERNDATE is harvey.lastModified (the pin's commit time, 1795382101); a rev-less path override falls back to that same value so a session build stays deterministic."
  - "The reference output stores only the nine member lists and the redacted log — never a baked .paq or auth artifact, because the store is world-readable."
  - "No generator registry entry was needed for marker, chain or substrate: every reference member maps to a tree file by sha256, so all entries are source = \"harvey\" and no file is generated at bake time for these three contracts."
  - "The three manifests are engine-emitted, not hand-written; the only edits are the per-manifest header comments naming the mkimage section and pin (the emitter's generic header contains the literal UNRESOLVED, which the plan's acceptance greps as zero)."

requirements-completed: [HARN-03, HARN-05]

coverage:
  - id: D1
    description: "harvey-reference-amd64 runs Harvey's own mkimage.sh once on a writable copy of the flake-built amd64 tree and records nine member lists; auth material is path/size/mode only and no credential leaves the derivation"
    requirement: HARN-03
    verification:
      - kind: other
        ref: "nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-reference-amd64 -> /nix/store/zyzy704k433qbldq02345mzfc7gir50k-harvey-reference-amd64-0; ls $p/*.members | wc -l = 9 (marker 30, chain 49, tier 91, net 64, services 4517, churn 52, substrate 27, auth 95, cpu 4442 lines)"
        status: pass
      - kind: other
        ref: "find $p -iname '*auth-creds*' | wc -l = 0; grep -rci 'password=' $p | sum = 0; auth.members/cpu.members each carry 6 opaque entries (adm/nvram, adm/keys, adm/domain.manifest, lib/ndb/auth, lib/ndb/local, boot/auth{feed|id-creds}.txt)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The devshell exports R9_HARVEY_REFERENCE_AMD64 lazily and never clobbers a user value"
    requirement: HARN-03
    verification:
      - kind: other
        ref: "nix develop -c sh -c 'printf %s \"$R9_HARVEY_REFERENCE_AMD64\"' == nix eval --raw .#packages.aarch64-linux.harvey-reference-amd64.outPath -> LAZY-PATH-MATCH; R9_HARVEY_REFERENCE_AMD64=/tmp/keep-me survives; nix-store -q --references devShell.drv | grep -c harvey-reference = 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Marker, chain and substrate are r9-assembled from per-file manifests whose member lists equal the reference bake's"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "for n in marker chain substrate: nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image $n --compare-reference -> matches the reference member list (30 / 49 / 27 members), THREE-MATCH"
        status: pass
      - kind: unit
        ref: "nix develop -c cargo test -p xtask image:: -> lib target 30 passed, 0 failed (the two non-lib targets run 0 tests; see the verification note)"
        status: pass
    human_judgment: false
  - id: D4
    description: "No kernel-byte change and Harvey untouched"
    requirement: HARN-03
    verification:
      - kind: other
        ref: "target/x86_64-unknown-none-elf/debug/x86_64 sha256 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f and target/aarch64-unknown-none-elf/debug/aarch64 sha256 76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2, both equal to the 01-08 baseline; Harvey HEAD 25d97dc4d..., status hash 041d2929...; zero port/src changes; cargo xtask test/clippy/check = 0/0/0"
        status: pass
    human_judgment: false

duration: 44min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 15: The amd64 reference bake and the marker, chain and substrate images Summary

**`harvey-reference-amd64` runs Harvey's own x86_64 `mkimage.sh` once on a writable copy of the flake-built amd64 tree and records all nine image contracts as member lists — auth material as path/size/mode only, no credential in the store — and r9 now assembles the marker, chain and substrate images from per-file manifests whose member lists are byte-for-byte equal to that reference (30/49/27 members).**

## Performance

- **Duration:** ~44 min (worktree spawn 2026-11-23T07:55:01Z to close 2026-11-23T08:40Z), dominated by three tree rebuilds (each ~5 min) while the reference bake's missing source-side inputs surfaced, and the three ~4,500-file member-list recordings
- **Started:** 2026-11-23T07:55:01Z
- **Completed:** 2026-11-23T08:40:00Z
- **Tasks:** 2 of 2
- **Files modified:** 6 (4 created, 2 modified; 861 insertions)
- **Plan estimate:** 70,000 tokens; **actual:** 7,403 tokens (chars/4 over the realized diff). The estimate sized the reference-bake investigation; the wall time went to builder runs, which the token measure does not see.

## Accomplishments

- **The reference is one fresh bake, and it completed all nine contracts.** `nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-reference-amd64` prints `/nix/store/zyzy704k433qbldq02345mzfc7gir50k-harvey-reference-amd64-0`; the log shows every `== PASS: baked …` line through §15 (cpu) with KERNDATE 1795382101 and the pin's own header/trailer magics. Member-list sizes: marker 30, chain 49, tier 91, net 64, services 4517, churn 52, substrate 27, auth 95, cpu 4442.
- **No credential or key material reaches the store.** The derivation copies no baked image and no auth artifact into `$out`; the member lists mark the six p9sk1-carrying paths per auth/cpu image `opaque` (nvram, keys, domain.manifest, ndb/auth, ndb/local, boot/authfeed.txt or boot/authid-creds.txt); the log is filtered of every `password` line; the build runs `find $out -iname '*auth-creds*'` and scans `$out` for the generator's own password value, failing on either. The plan's verify prints 0 and 0.
- **The venue is recorded: plain stdenvNoCC, not proot.** mkimage's two host-side builds (§11k hsvctcpecho/hmkdir, §14i2/§14i3 hsvctcpecho/hauthprobe) compile with the hostbin 6c/6l and link against the tree's `amd64/lib` with no Plan 9 namespace, so `reference.nix` is a plain derivation. `harvey-hostbin` is exported as `HARVEY_LINUX_BIN` (mkimage §11d's own variable) and appended to PATH.
- **The reference is pinned to the tree it describes.** `reference.nix` asserts `rev=<flake pin>` and `arch=amd64` from the tree's HARVEY-TREE stamp and refuses otherwise (T-01-51); the tree is the FlakeHub/locked fetch, not the read-only checkout.
- **The devshell supplies the reference lazily.** `R9_HARVEY_REFERENCE_AMD64` equals the evaluated outPath (LAZY-PATH-MATCH), a pre-set value survives shell entry, and the devShell derivation has zero references to the reference derivation.
- **Three x86_64 images are now r9-assembled and D-29-verified.** The engine emitted `marker.toml` (30), `chain.toml` (49) and `substrate.toml` (27) from the reference lists; every member is a per-file `[[file]]` (D-15) with a `[[dir]]` per image directory, no `[[tree]]` and no generator entry. `cargo xtask image --arch x86-64 --image <n> --compare-reference` prints "matches the reference member list" for all three. `/amd64/bin/ps` is a per-file entry in chain.toml (the D-27 swap target).
- **Kernel bytes and Harvey are untouched.** q35churnshell `50b01c9a…` and virtshell `76e6e153…` equal the 01-08 baseline after the flake/tree changes; Harvey HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`; zero `port/src` changes; `cargo xtask test/clippy/check` 87 tests and 0/0/0.

## Task Commits

Each task was committed atomically:

1. **Task 1: harvey-reference-amd64 — one fresh mkimage.sh bake as nine member lists** — `a5fc417` (feat)
2. **Task 2: amd64 marker, chain and substrate manifests verified against the reference** — `b59e6ed` (feat)

**Plan metadata:** the `docs(01-15): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `nix/harvey/reference.nix` (created) — the reference bake: stamp check, writable tree copy, the member-recording mkpaqfs shim (`opaque` set, engine-format sorted list), `HARVEY_LINUX_BIN`/PATH wiring, the redacted-log and secret gates, the nine-list count
- `nix/harvey/tree-amd64.nix` (modified) — the D-19 source-side list gains `sys/include` (host 6c headers), `build/x86_64/mkimage.sh` (the reference bake) and `sys/src/cmd/auth/secstore/mkfile` (the auth-suite multi-TARG resolver), each with its read site
- `flake.nix` (modified) — `harveyReferenceAmd64`; package `harvey-reference-amd64` (Linux); the guarded lazy `R9_HARVEY_REFERENCE_AMD64` export and banner line
- `xtask/manifests/amd64/marker.toml` (created) — 30 members, §1-6, pin `bd7a698…`
- `xtask/manifests/amd64/chain.toml` (created) — 49 members, §8, pin `bd7a698…`, `/amd64/bin/ps` per-file
- `xtask/manifests/amd64/substrate.toml` (created) — 27 members, §13, pin `bd7a698…`
- `xtask/src/image.rs` — **not modified**: no generator entry was needed for these three images

## Decisions Made

- **Plain stdenvNoCC, not the proot venue.** The bake's host-side compiles need only 6c/6l and the tree's headers/libs; recorded in the nix file and here as the plan asks.
- **hostbin last on PATH plus HARVEY_LINUX_BIN.** mkimage §11d invokes `$HARVEY_LINUX_BIN/6c` (default `$HARVEY/.build/hostbin-linux-x86_64/bin`), so the explicit variable is the load-bearing wire; the PATH append only serves bare-name helpers, and last position keeps plan9port's sed/ls/date from shadowing GNU.
- **The member list is the engine's reference format.** `<path> file <mode> <size> <sha256>` / `<path> dir <mode> - -`, sorted, path relative; `parse_reference_members` is the only consumer and the plan's `f …` shorthand belongs to the `[[tree]]` members file.
- **Image-scoped opaque set.** `lib/ndb/local` is opaque only for auth/cpu (the @AUTHDOM@ row); the services image's generated local carries no domain and keeps a real hash.
- **KERNDATE = 1795382101** (`harvey.lastModified`, the bd7a698 commit time); the rev-less override fallback uses the same value.
- **No generator registry change.** Marker/chain/substrate contain only copies of tree files; every reference sha256 matched a tree file, so `--emit-manifest` produced no UNRESOLVED entries.
- **The emitted header is replaced, not amended.** The emitter's generic header contains the literal `UNRESOLVED`, which the plan's zero-count grep would read; each manifest's header now names its mkimage section and the pin instead.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The tree did not carry `build/x86_64/mkimage.sh`**

- **Found during:** Task 1 (the first reference build: `HARVEY-REFERENCE-FAIL the tree lacks build/x86_64/mkimage.sh`)
- **Issue:** the reference bake runs the pin's own script, but 01-14's D-19 install list carried only the data the *assembler* read; the script itself was never installed.
- **Fix:** `build/x86_64/mkimage.sh` joins `tree-amd64.nix`'s source-side file list with its read site ("the reference bake itself").
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** the rebuilt tree contains the script; the reference bake proceeds.
- **Committed in:** `a5fc417`

**2. [Rule 3 - Blocking] The tree did not carry `sys/include`**

- **Found during:** Task 1, before the second build (the reference's own preflight asserts the directory, because mkimage §11k/§14i2/§14i3 compile with `-Isys/include`).
- **Issue:** the host-side 6c builds read the source headers; the tree only carried `amd64/include`.
- **Fix:** `sys/include` joins the source-side directory list with the three section read sites.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** the reference bake's host builds compile and link; the images pass their magic checks.
- **Committed in:** `a5fc417`

**3. [Rule 1 - Bug] The auth-suite resolver lost `hsecstored` (the S3/A12 failure class)**

- **Found during:** Task 1, the third reference build: `error: auth bake: sweep subject hsecstored missing from the computed auth-suite set`
- **Issue:** mkimage §14b reads `sys/src/cmd/auth/secstore/mkfile` to learn the multi-word `TARG = secstore secstored secuser aescbc`; the tree carried no mkfiles, so the resolver staged only `secstore` and the bake refused. This is the exact error S3 recorded as the bake-2 stop (A12), here caused by the tree's source-side list rather than by a re-bake.
- **Fix:** install `sys/src/cmd/auth/secstore/mkfile` in the tree with its §14b read site.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** the rebuilt reference completes §14 and §15 (`auth.members` 95, `cpu.members` 4442) with the six opaque entries each.
- **Committed in:** `a5fc417`

**4. [Rule 1 - Plan defect] Task 1's stated member-list format is the `[[tree]]` members-file format**

- **Found during:** Task 1 implementation (reading `parse_reference_members`)
- **Issue:** the plan says `f <path> <size> <sha256> <mode>` / `d <path>`, but `--compare-reference` (the plan's own key link) parses `<path> file <mode> <size> <sha256>` / `<path> dir <mode> - -`, which is what `write_members` emits and `parse_reference_members` accepts.
- **Fix:** the shim records the engine's reference format; the plan-text shorthand is corrected here.
- **Files modified:** `nix/harvey/reference.nix`
- **Verification:** all three `--compare-reference` runs parse and match.
- **Committed in:** `a5fc417`

**5. [Rule 1 - Plan defect] Task 1's opaque acceptance grep expects a trailing space**

- **Found during:** Task 1's acceptance re-run
- **Issue:** `grep -c ' opaque '` cannot match the canonical member-list line, whose last field is `opaque` with no trailing space (the format is what `parse_reference_members` reads).
- **Fix:** verified the criterion's intent instead: `grep -c ' opaque$' "$p/cpu.members"` = 6 (and 6 in auth.members). No format change — a trailing space would desync the reference from the engine's own writer for no gain.
- **Files modified:** none (verification scope)
- **Verification:** the six paths are listed above and each is asserted by the derivation itself.
- **Committed in:** `a5fc417` (the enforcement)

**6. [Rule 1 - Plan defect] `cargo test -p xtask image::` prints "running 0 tests" for the non-lib targets**

- **Found during:** Task 2's verify
- **Issue:** the image tests live in the **library** crate (`xtask/src/lib.rs` -> `pub mod image`), so the lib target runs 30 tests, while the two extra test binaries (the `xtask` binary and `r9-netdev-args`) report `running 0 tests`, which the plan's `fails_when` text literally forbids.
- **Fix:** verified the intent: the lib target runs and passes all 30 `image::tests::*`; the two zero-test targets contain no image tests. `cargo test -p xtask --lib image::` shows 30 passed, 0 failed.
- **Files modified:** none (verification scope)
- **Verification:** `cargo test -p xtask image::` -> 30 passed in the lib; `cargo test -p xtask --lib image::` -> 30 passed, 50 filtered out.
- **Committed in:** `b59e6ed` (Task 2 commit)

---

**Total deviations:** 6 (3 blocking/bug fixes in the tree's source-side list, 3 plan-text verifications implemented by intent). **Impact:** the first three were required for the reference bake to complete at all and explain the S3/A12 `hsecstored` failure; the last three change no code and are recorded so the verifier can re-run the intended checks.

### Verification note (plan convention)

The x86_64-linux build of `harvey-tree-amd64`/`harvey-reference-amd64` is the reference venue, not an x86_64 boot; no x86_64 boot is claimed here (the plan's stop-B convention). The `packages.x86_64-linux.harvey-reference-amd64` attribute evaluates but was not built; only the aarch64-linux builder leg is measured.

## Issues Encountered

- **Three tree rebuilds were needed.** Each missing source-side input (mkimage.sh, sys/include, the secstore mkfile) surfaced only after the tier build re-ran; the D-19 install list is now complete for the reference bake, and each input names its read site so the next reader can audit it.
- **The S3/A12 `hsecstored` root cause is now explained.** §14b's computed auth-suite set depends on the source mkfile's multi-word `TARG`; a bake from a tree without it fails exactly as S3's second bake did. The fresh derivation here succeeded once the mkfile was installed.
- **`requirements-completed` is copied verbatim from the plan, but REQUIREMENTS.md was not ticked.** HARN-03/HARN-05 are declared by sibling plans without SUMMARYs; the shared-ID gate blocks the mark. The orchestrator owns shared-file updates after the wave merges.

## Known Stubs

None. No image or auth artifact is stored; the reference output is exactly the nine member lists and the redacted log. The three manifests carry every member; no generator/UNRESOLVED entry exists for these images.

## Threat Flags

None beyond the plan's threat model. T-01-50 is implemented by the opaque set, the password-line filter, and the two build-failing scans; T-01-51 by the stamp-rev assertion plus the fresh derivation and the passing compare-reference runs; T-01-SC by the locked nixpkgs with no package installs.

## Next Phase Readiness

- **Plan 01-16 gets the reference for its generator work**: all nine member lists are available through `R9_HARVEY_REFERENCE_AMD64`, so `--emit-manifest` for cpu/auth can map everything except the two generated `lib/ndb/local` files (which need a registered generator) and the opaque paths (path/size/mode only).
- **Plan 01-19 gets the remaining contracts' reference** (tier, net, services, churn) from this one bake; the emitter and compare gates are unchanged.
- **Plan 01-29 can point the five profiles at `image:marker`, `image:chain` and `image:substrate`** once 01-16 lands the cpu/auth images; these three are already r9-assembled and reference-equal.
- **Harvey remains untouched** and the kernel baseline is intact after the flake/tree changes.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `nix/harvey/reference.nix`, `xtask/manifests/amd64/{marker,chain,substrate}.toml`, `01-15-SUMMARY.md` — all FOUND; `nix/harvey/tree-amd64.nix` and `flake.nix` modified.
- Task commits `a5fc417` (Task 1) and `b59e6ed` (Task 2) are ancestors of HEAD; `git rev-list --count c6b27cd..HEAD` = 2 (measured against the plan ledger).
- Task 1 verification re-run: `harvey-reference-amd64` prints `/nix/store/zyzy704…`; `ls $p/*.members | wc -l` = 9; `find $p -iname '*auth-creds*' | wc -l` = 0; `grep -rci 'password=' $p` sums to 0; `grep -c ' opaque$' $p/cpu.members` = 6 (the plan's literal `' opaque '` pattern is a trailing-space typo — deviation 5).
- Task 2 verification re-run: all three `--compare-reference` runs print "matches the reference member list" (30/49/27) and `THREE-MATCH`; `grep -c 'UNRESOLVED'` across the three manifests = 0; `grep -c '"/amd64/bin/ps"' chain.toml` = 1; both headers name `mkimage.sh §…` and `bd7a698`; `cargo test -p xtask image::` -> 30 passed in the lib target (the two non-lib targets run 0 — deviation 6).
- Plan verification re-run: kernel ELF sha256 `50b01c9a…` (x86_64 q35churnshell) and `76e6e153…` (aarch64 virtshell) equal the 01-08 baseline; Harvey HEAD `25d97dc4d…` with status hash `041d2929…` (HARVEY-UNTOUCHED); zero `port/src` changes; `cargo xtask test/clippy/check` exit 0/0/0 (87 tests).
- Devshell: `R9_HARVEY_REFERENCE_AMD64` == the evaluated outPath (LAZY-PATH-MATCH); a user value survives; the devShell derivation references the reference derivation 0 times.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
