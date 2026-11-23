---
phase: 01-stage-p0-cross-build-harness
plan: 11
subsystem: cross-build-harness
tags: [xtask, image-engine, image-drift, provenance, manifest-emitter, member-list, tier-0, d-15, d-16, d-29, harn-03, harn-05]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "the 01-08 stage-and-bake engine (Manifest, stage, member_list, bake, sha256_file, the aarch64 smoke manifest and its byte-identical D-29 member list) and the one per-architecture Harvey resolver (harvey_dir)"

provides:
  - "xtask/src/image.rs: the hardened engine — two-way IMAGE-DRIFT (unlisted/missing/changed), IMAGE-PROVENANCE (Harvey-root prefix + byte identity), IMAGE-PATH (canonicalised escapes), IMAGE-GENERATOR (registry + reserved UNRESOLVED), [[tree]] members-file expansion, [[library]] tier-0 audit table, assemble_checked, compare_members, emit_manifest, check_libraries, parse_reference_members, 30 inline tests"
  - "xtask/src/main.rs: `cargo xtask image --arch <a> --image <n> [--compare-reference] [--emit-manifest <path>] [--libraries] [--verbose]` (ImageStep); assemble_initrd routed through assemble_checked; image:<name> assembled on demand in ensure_initrd_image and resolve_boot_module"
  - "xtask/src/config.rs: the `image:<name>` scheme for [qemu] initrd.file and [dist] module (target/images/<x86_64|aarch64>/<name>.paq) with a unit test"
  - "the D-29 evidence that the checked engine's smoke member list is byte-identical to 01-08's"

affects: [01-14, 01-15, 01-16, 01-17, 01-18, 01-20, HARN-03, HARN-05, D-16, D-29]

actuals:
  tokens: 28168
  tasks: 2
  commits: 2

plan_head_before: 1610013947d8da9d5f9749b53c81bb1ae134312e
plan_head_after: 88d1e8575a7962552bc95918b19532c1b7dc1333

tech-stack:
  added: []
  patterns:
    - "Manifest and image are checked against each other on the staging tree (mkpaqfs's exact input), both directions, before the bake — unlisted, missing and changed each carry a named IMAGE-DRIFT error"
    - "Every manifest source is canonicalised before use: harvey under the tree, rust under the workspace, tree members under the tree — a symlink escape is IMAGE-PATH, never a copied host file"
    - "A rust entry proves provenance twice: its canonical path must not start with any Harvey tree root, and its sha256 must differ from the artifact named by `replaces`"
    - "The generator registry is a named allow-list; this plan ships an empty registry plus the reserved UNRESOLVED marker, so an unresolved emitted manifest is refused by parse_manifest, never silently staged"
    - "The [[tree]] members file is the contract: every source file is staged, then its size/sha256/mode is compared to the record; the manifest emitter writes that record from the reference bake"
    - "Config schemes stay pure path transforms (harvey:, image:); the materialisation is a separate on-demand hook (ensure_named_image) shared by the initrd and dist paths"

key-files:
  created: []
  modified:
    - xtask/src/image.rs
    - xtask/src/main.rs
    - xtask/src/config.rs

key-decisions:
  - "01-08's `stage(manifest, tree, stage_dir)` is kept as a thin wrapper around the full `stage_full(manifest, manifest_dir, tree, workspace, stage_dir)`, so the published engine API does not break while rust/tree staging gains the workspace and members-file inputs."
  - "The schema change is additive and non-breaking: `from`/`mode` became optional struct fields but stay required per source kind in validate(), so the 01-08 smoke manifest parses unchanged and `deny_unknown_fields` still rejects stale keys."
  - "Drift runs on the staging tree against a manifest-derived expected set that includes the directories implied by every declared path, so an implicitly created ancestor directory is never reported as unlisted."
  - "[[tree]] staging copies every file under the source directory (using the members file for modes when a file is listed), and the drift pass then reports unlisted/missing/changed — the members list is the contract, not a staging filter."
  - "R9_HARVEY_REFERENCE_AMD64/_ARM64 are mandatory for --compare-reference and --emit-manifest: unset is a named error naming the variable and plans 01-15/01-17, never a skipped comparison."
  - "The tier-0 list is read from the tree's own build-directory names (build/x86_64 or build/aarch64), which differ from the Plan 9 objtype used for the manifest directories (amd64/arm64); the first --libraries run against the real tree caught the mix-up."
  - "`image:` resolves to a workspace-relative target/images/<x86_64|aarch64>/<name>.paq (matching the other target/ config paths) and both ensure_initrd_image and resolve_boot_module materialise it through the same assemble_named_image path ImageStep runs."
  - "The `image` subcommand requires both --arch and --image: a default arch on an image assembler would be a silent wrong-architecture bake."

patterns-established:
  - "Named-error vocabulary for the image engine: IMAGE-DRIFT unlisted|missing|changed, IMAGE-PROVENANCE, IMAGE-PATH, IMAGE-GENERATOR — every message names the image, the image path and the source"
  - "Reference bake comparison by member list (D-29), with `opaque` hashes compared by path, size and mode only (auth material), never by byte differential"
  - "Manifest emission is lossless-by-refusal: an unmatched reference file becomes generator = \"UNRESOLVED\" with a why-comment and the command exits non-zero, so no file can be quietly omitted"

requirements-completed: [HARN-03, HARN-05]

coverage:
  - id: D1
    description: "The image engine enforces manifest/image agreement in both directions, Rust provenance, source-path containment, generator registration and the tier-0 library table, all with named errors and unit tests on fixture trees"
    requirement: HARN-05
    verification:
      - kind: unit
        ref: "xtask/src/image.rs#image::tests (30 tests: drift unlisted/missing/changed, provenance tree/identical/ok, IMAGE-PATH symlink escape, IMAGE-GENERATOR unknown/UNRESOLVED, tree expansion/members, libraries, schema/path validation)"
        status: pass
    human_judgment: false
  - id: D2
    description: "cargo xtask image assembles the aarch64 smoke image through the checked engine and writes target/images/aarch64/smoke.members — 9 lines, byte-identical to 01-08's D-29 list"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "R9_HARVEY_DIR=/Users/neilskinner/code/harvey R9_MKPAQFS=... nix develop -c cargo xtask image --arch aarch64 --image smoke -> `image: aarch64/smoke ... members 9`; diff against 01-08-SUMMARY.md's member block -> empty"
        status: pass
      - kind: integration
        ref: "R9_HARVEY_DIR=... cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf -> boot-test: PASS (the initrd path now runs assemble_checked)"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-29 reference comparison, the reference-derived manifest emitter and the tier-0 library check exist and are wired into the CLI"
    requirement: HARN-03
    verification:
      - kind: unit
        ref: "xtask/src/image.rs#compare_members (identical/added/removed/changed/opaque), #emit_manifest (path->basename->lexical preference, UNRESOLVED, dirs, opaque), #check_libraries"
        status: pass
      - kind: integration
        ref: "fixture reference dir: --compare-reference passes then fails naming `hello` after a size edit (exit 1); --emit-manifest writes a parseable manifest and exits 1 listing `ghost` when unmatched"
        status: pass
    human_judgment: true
    rationale: "The real reference bakes for every image land with plans 01-14/01-15/01-17, and the full tier-0 success path needs an amd64 manifest carrying its [[library]] table (also 01-14); today's CLI evidence is fixture-scale even though the unit contract is complete."
  - id: D4
    description: "A boot config names `image:<name>` in [qemu] initrd.file and [dist] module; xtask assembles the missing/stale image on demand into target/images/<arch>/<name>.paq"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "temporary config with initrd = { file = \"image:smoke\", ... }, paq deleted first: `xtask: image aarch64/smoke missing or stale — assembling` then boot-test: PASS"
      - kind: unit
        ref: "xtask/src/config.rs#initrd_image_scheme_resolves_under_target_images (both arches + bare prefix error)"
        status: pass
    human_judgment: true
    rationale: "The [dist] module leg shares ensure_named_image but cannot be executed on this Mac: ISO composition needs grub2/xorriso, which the devshell supplies on Linux only (01-RESEARCH 'Verification that cannot run on this aarch64 Mac')."
  - id: D5
    description: "No regression: full workspace test/clippy/check green, the 01-08 virtexec smoke remains green, Harvey is untouched and port/src is untouched"
    verification:
      - kind: integration
        ref: "cargo xtask test/clippy/check -> 0/0/0; Harvey HEAD 25d97dc4d with status hash 041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e (identical to 01-08); git diff -- port/src empty"
        status: pass
    human_judgment: false

duration: 7min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 11: Image engine hardening — drift both ways, Rust provenance, reference comparison Summary

**The manifest is now a contract the harness enforces: the staging tree is checked against the manifest in both directions (IMAGE-DRIFT unlisted/missing/changed), a rust entry cannot resolve under a Harvey root or be byte-identical to the artifact it replaces (IMAGE-PROVENANCE), sources cannot escape their roots (IMAGE-PATH), the `cargo xtask image` subcommand assembles, compares to a reference member list and emits manifests, and `image:<name>` boots an on-demand assembled image.**

## Performance

- **Duration:** ~7 min (worktree branch created 05:06:57Z; last task commit 05:13:17Z)
- **Started:** 2026-11-23T05:06:57Z
- **Completed:** 2026-11-23T05:15:00Z
- **Tasks:** 2 of 2
- **Files modified:** 3 (2,105 insertions, 169 deletions across the plan; image.rs 30 inline tests)

## Accomplishments

- `xtask/src/image.rs` is the enforcement layer HARN-05 requires. `assemble_checked` runs parse → stage → generators → two-way drift → provenance → bake → members. The drift pass compares the staging tree (mkpaqfs's exact input) with a manifest-derived expected set — including the directories every declared path implies — and reports `IMAGE-DRIFT unlisted` (staged, undeclared), `IMAGE-DRIFT missing` (declared, unstaged, or a source absent) and `IMAGE-DRIFT changed` (a `[[tree]]` member whose size/sha256/mode disagrees with its members record), each naming the image, image path and source.
- Provenance is structural. A `rust` entry's source is canonicalised and refused if it resolves under any Harvey tree root (`IMAGE-PROVENANCE … resolves into the Harvey tree`) or is byte-identical to the tree file named by `replaces` (`IMAGE-PROVENANCE … identical to the Harvey artifact`). A symlink escaping the tree or workspace is `IMAGE-PATH`; every path is validated for absoluteness and `.`/`..`, including the new `[[tree]]` and `[[library]]` tables.
- The schema grew without breaking 01-08's smoke manifest: `source = "rust"` (`from` workspace-relative, `replaces` tree-relative), `source = "generated"` (`generator`, registry-backed), `[[tree]]` (`path`, `from`, `members`), `[[library]]` (`name`, `from`, `in_image = false`). The generator registry is empty by design — plan 01-16 registers the real generators — and `UNRESOLVED` (the emitter's marker) is rejected, so a generated entry can never silently no-op.
- `cargo xtask image --arch <a> --image <n>` assembles through the checked engine and prints `image: <arch>/<image> <paq> sha256 <hex> members <n>`; `--compare-reference` reads `$R9_HARVEY_REFERENCE_AMD64`/`_ARM64/<image>.members` and fails on every added, removed or changed path (an `opaque` reference hash compares path, size and mode only); `--emit-manifest <path>` maps every reference file to a tree file with the same sha256 (identical relative path → identical basename → lexically first), maps dirs to `[[dir]]`, and exits non-zero listing every `UNRESOLVED` entry; `--libraries` checks the `[[library]]` table against the tree's `build/<x86_64|aarch64>/tiers/tier0-libs.txt`.
- `assemble_initrd` now routes through `assemble_checked`, so the 01-08 smoke image gets the drift and provenance checks. Its member list is byte-identical to the D-29 list recorded in 01-08 (same 9 lines), and `boot-test --arch aarch64 --config virtexec --accel hvf` is green.
- The `image:` scheme (D-16) resolves `[qemu] initrd.file` and `[dist] module` to `target/images/<x86_64|aarch64>/<name>.paq`, with a `config.rs` unit test. `ensure_initrd_image` and `resolve_boot_module` assemble a missing/stale image through the same path as `ImageStep`, printing `xtask: image <arch>/<name> missing or stale — assembling`. Proven end to end: a temporary `initrd = { file = "image:smoke" }` config, with the paq deleted first, assembled and booted green.

## Task Commits

Each task was committed atomically:

1. **Task 1: Drift, provenance, path validation and tree entries in image.rs** — `b4f1090` (feat)
2. **Task 2: The `cargo xtask image` subcommand, the `image:` config scheme, and assemble_initrd routing** — `88d1e85` (feat)

**Plan metadata:** the `docs(01-11): complete …` commit that carries this SUMMARY.

_Note: both tasks are `tdd="true"`; see the TDD Gate Compliance section — tests and implementation landed together per the one-commit-per-task protocol._

## Files Created/Modified

- `xtask/src/image.rs` — the hardened engine: the additive schema (`rust`/`generated` file kinds, `[[tree]]`, `[[library]]`), `stage_full` (+ the 01-08-compatible `stage` wrapper), `expected_members`, `check_drift`, `check_provenance`, `assemble_checked`, `parse_reference_members`, `compare_members`, `emit_manifest`, `parse_tier_list`, `check_libraries`, `image_paq`/`arch_dir`; 18 new inline tests (30 total)
- `xtask/src/main.rs` — `ImageStep` + clap registration/dispatch; `assemble_initrd` through `assemble_checked`; `assemble_named_image`, `image_stale`, `ensure_named_image`, `reference_member_list`, `image_manifest_path`, `image_paq_path`, `objtype`, `build_dir_name`; `ensure_initrd_image` and `resolve_boot_module` `image:` branches
- `xtask/src/config.rs` — `Initrd::resolved_file` handles `image:<name>` (workspace-relative `target/images/<arch>/<name>.paq`, bare-prefix error); unit test for both arches

## Decisions Made

- **Keep the 01-08 `stage` API.** `stage(manifest, tree, stage_dir)` delegates to `stage_full(manifest, manifest_dir, tree, workspace, stage_dir)`; the checked path uses the full form. No caller breaks.
- **Additive, non-breaking schema.** `from`/`mode` are `Option` fields internally but required per source kind in `validate()`, so the 01-08 manifest is unchanged and unknown keys stay rejected.
- **Drift on the staging tree, both directions, including implicit ancestor directories.** The expected set is derived from the manifest (files, dirs, tree members, ancestors), so an implicitly created directory is never "unlisted".
- **Tree staging copies every source file; the members file supplies modes and is the drift contract.** Unlisted source files are caught by drift rather than skipped at staging.
- **The tier-0 list uses the tree's build-directory names (`build/x86_64`, `build/aarch64`), not the Plan 9 objtype (`amd64`, `arm64`).** The first `--libraries` run against the real Harvey tree caught the mix-up.
- **`R9_HARVEY_REFERENCE_*` unset is a named error.** A comparison that cannot find its reference never silently passes.
- **`--arch` and `--image` are required on `cargo xtask image`.** No default arch on an image assembler.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `smoke_initrd_stale` had to follow `image::input_paths`'s new signature**

- **Found during:** Task 1 (the first `cargo test -p xtask image::` build)
- **Issue:** The rust/tree kinds need the manifest directory and workspace to enumerate their inputs, so `input_paths` gained two parameters; the old two-argument call in `main.rs` no longer compiled, blocking Task 1's verify.
- **Fix:** The call site passes the smoke manifest's directory (`xtask/manifests/arm64`) and the workspace root. Task 2 then replaced the whole staleness helper with `image_stale`.
- **Files modified:** `xtask/src/main.rs`
- **Verification:** `cargo test -p xtask image::` compiles and passes; the virtexec boot is green.
- **Committed in:** `b4f1090` (Task 1 commit)

**2. [Rule 1 - Bug] `--libraries` read `build/arm64/…` instead of the tree's `build/aarch64/…`**

- **Found during:** Task 2 (`cargo xtask image --arch aarch64 --image smoke --libraries` against the real Harvey tree)
- **Issue:** The code reused the Plan 9 objtype (`arm64`) for the tree's build directory; the Harvey tree names it `aarch64` (and `x86_64` for amd64), so the tier list was never found.
- **Fix:** A separate `build_dir_name(arch)` maps to `aarch64`/`x86_64`; objtype stays for `xtask/manifests/<objtype>/`. Verified: the arm64 list now loads and reports its 35 tier-0 gaps against the smoke manifest (which has no `[[library]]` table yet), exit 1.
- **Files modified:** `xtask/src/main.rs`
- **Verification:** `--libraries` reads the real list; the unit success path is green.
- **Committed in:** `88d1e85` (Task 2 commit)

**3. [Rule 2 - Missing Critical] Image-path validation now rejects the image root and normalises trailing slashes**

- **Found during:** Task 2 (path-validation review before the commit)
- **Issue:** A `[[dir]]`/`[[tree]]` path of `/` passed validation but produced an empty staging key that no member-list line can match (a confusing drift "missing"); a trailing slash produced the same key mismatch.
- **Fix:** `validate_image_path` requires at least one normal component and `image_rel` trims both leading and trailing slashes. Threat register T-01-27/T-01-38 stays complete.
- **Files modified:** `xtask/src/image.rs`
- **Verification:** all 30 image tests and the full workspace suite pass; the smoke member list is unchanged.
- **Committed in:** `88d1e85` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 bug, 1 blocking, 1 missing critical)
**Impact on plan:** All three protect a stated acceptance criterion or a threat mitigation; no scope creep, no architecture change.

### Plan verification drift

- The `--libraries` success leg cannot run against the smoke manifest: it has no `[[library]]` table, and the amd64 manifest that carries the tier-0 table lands with plan 01-14. The success path is unit-tested with a small tier list; the CLI correctly reports every missing name today.
- The `[dist] module` `image:` leg is wired through the same `ensure_named_image` path as the verified initrd leg, but it cannot be executed on this Mac: ISO composition needs `grub2`/`xorriso`, which the devshell provides on Linux only (01-RESEARCH, "Verification that cannot run on this aarch64 Mac").

## TDD Gate Compliance

Both tasks carry `tdd="true"` in PLAN.md, but `workflow.tdd_mode` is not enabled (there is no `.planning/config.json` in this project), and the orchestrator's parallel-executor protocol requires one atomic commit per task. The behaviour blocks drove the 18 new image tests (30 total), yet the tests landed in the same commits as their implementations (`b4f1090`, `88d1e85`), so there is **no separate `test(01-11)` RED commit and no RED-evidence record** for either task — the TDD gate sequence is not satisfied by history, only by the test suite. This matches 01-08's precedent in this phase and is recorded here per the reference's missing-RED rule.

## Issues Encountered

- **The arm64 tier-0 list has 35 names** (the amd64 list has 27). When an aarch64 library table lands, its manifest must carry all 35 `[[library]]` rows; today's smoke manifest is a boot fixture, not a full image, so `--libraries` fails by design.
- **`requirements-completed` is copied verbatim from the plan, but REQUIREMENTS.md was not ticked.** `HARN-03` and `HARN-05` are declared by sibling plans that have not produced summaries yet; the shared-ID gate would block them. The orchestrator owns the shared-file updates after the wave merges.

## Known Stubs

None. The generator registry is deliberately empty and a generated entry fails loudly (`IMAGE-GENERATOR`) rather than staging nothing; plan 01-16 registers the real generators. The `image` command's `--emit-manifest` writes `UNRESOLVED` entries only as an explicit, named failure state, never as a silently accepted manifest.

## Threat Flags

None beyond the plan's threat model. T-01-37 (Rust provenance) is mitigated by the canonicalised Harvey-root prefix check plus the `replaces` byte-identity check; T-01-38 (symlink escape) by canonicalising every source (harvey/rust/tree member) and refusing anything outside its root (`IMAGE-PATH`); T-01-39 (silent omission) by the emitter's `UNRESOLVED` marker, which `parse_manifest` rejects and the CLI exits non-zero on. No new crate was added (sha256 still shells out to `sha256sum`/`shasum`).

## Next Phase Readiness

- The enforcement contract is settled for the image plans: 01-14/01-15/01-17 add manifests and export the `R9_HARVEY_REFERENCE_*` bakes; 01-16 registers the real generators. None needs to change the engine's shape.
- `cargo xtask image` is the developer tool for keeping ~eleven manifests honest across Harvey pin bumps: `--emit-manifest` derives one from a reference bake, `--compare-reference` checks it, `--libraries` keeps the tier-0 table honest.
- `cargo xtask test`, `clippy` and `check` are green; Harvey is untouched (HEAD `25d97dc4d`, status hash `041d2929…` identical to 01-08's); zero `port/src` changes.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `01-11-SUMMARY.md`; the plan creates no source files (it hardens existing ones).
- Task commits `b4f1090` and `88d1e85` are ancestors of HEAD.
- Task 1 acceptance: `grep -c '#\[test\]' xtask/src/image.rs` = 30 (>= 14); all six error tokens present (`ERRORS-OK`); virtexec HVF boot printed `boot-test: PASS`.
- Task 2 acceptance: `Command::new("image")` count = 1; `target/images/aarch64/smoke.members` has 9 lines and diffs empty against 01-08's list; `UNRESOLVED` count = 21 (>= 2); `"image:"` count in config.rs = 3 and the unit test is present/passing; `image: aarch64/smoke … members 9` printed; unset `R9_HARVEY_REFERENCE_ARM64` printed `NAMED-ERROR-OK`; `cargo xtask test`/`clippy`/`check` all exit 0.
- Plan verification: Harvey untouched (status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`), zero `port/src` changes.
