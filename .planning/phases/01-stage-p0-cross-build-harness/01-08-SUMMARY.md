---
phase: 01-stage-p0-cross-build-harness
plan: 08
subsystem: cross-build-harness
tags: [xtask, harvey-resolver, image-manifest, mkpaqfs, tracer, d-01, d-15, d-16, d-21, d-29, harn-02, harn-03, harn-06, harn-07]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "the pass-2 planning artefacts (01-CONTEXT D-01..D-29) and the pass-1 spike results; the owner's pre-built Harvey checkout at /Users/neilskinner/code/harvey (smoke-out/ and .build/bin/mkpaqfs) is the manual-override supply (D-21)"

provides:
  - "xtask/src/harvey.rs: harvey_dir(arch) and resolve(); precedence R9_HARVEY_DIR > R9_HARVEY_DIR_ARM64/_AMD64 > HARVEY_SRC; one named error; HARVEY-TREE stamp check; realise-on-first-use; 14 inline tests"
  - "xtask/src/image.rs: schema-1 manifest (deny_unknown_fields), stage(), member_list(), write_members(), bake(), sha256_file(); 12 inline tests"
  - "xtask/manifests/arm64/smoke.toml: the 8-file + /proc aarch64 smoke manifest"
  - "the D-29 proof for the smoke image: the manifest-assembled member list is byte-identical to the pre-D-16 hard-coded assembler's"
  - "every Harvey-tree consumer routed through the one resolver (7 sites); Arch threaded through resolved_file / loader_device_arg / apply_to_qemu_step_with_overrides / prepare_initrd_dtb / ensure_initrd_image / resolve_boot_module"

affects: [01-09, 01-10, 01-11, 01-13, 01-14, 01-16, 01-20, HARN-02, HARN-03, HARN-06, HARN-07]

actuals:
  tokens: 22133
  tasks: 2
  commits: 2

plan_head_before: a643bc1cf28c6bc44d36b1e35d43dbbd302caf9f
plan_head_after: a297253006d0a754c5d4a12e1259c483efebf7f5

tech-stack:
  added: []
  patterns:
    - "One generic stage-and-bake engine plus declarative per-image TOML manifests; a manifest lists every image member with path, source, from and mode"
    - "One resolver per architecture with an injected lookup/workspace/realiser: precedence, the named error and realise-on-first-use are unit-testable without mutating the process environment"
    - "A source invariant test (include_str! over every sibling source file) enforces that only harvey.rs reads a Harvey-tree variable"
    - "A `source =` enum with forward-declared kinds (rust/generated/tree) so later plans add provenance without a manifest schema break; staging an unimplemented kind is a loud error, never a skip"

key-files:
  created:
    - xtask/src/harvey.rs
    - xtask/src/image.rs
    - xtask/manifests/arm64/smoke.toml
  modified:
    - xtask/src/main.rs
    - xtask/src/lib.rs
    - xtask/src/uregcheck.rs
    - xtask/src/config.rs
    - xtask/src/pcap.rs

key-decisions:
  - "Manifest modes are the staged binaries' own modes (0775, hello 0755), not a uniform 0755: the old assembler preserved the source mode via fs::copy, and D-29's acceptance criterion is an EMPTY member-list diff (the member list records mode)."
  - "R9_MKPAQFS is an explicit input with no derived default; the devshell exports it from plan 01-13 onward, and a checkout's .build/bin/mkpaqfs can be named now. mkpaqfs is not part of the Harvey TREE contract."
  - "ensure_initrd_image refuses a target/-resident initrd for any arch other than aarch64: the only manifest this plan carries is the arm64 smoke bake, and silently handing an x86_64 profile the aarch64 userland would be worse than a named error."
  - "The member-list format is whitespace-delimited `<path> file <mode> <size> <sha256>` / `<path> dir <mode> - -`, mode octal without zero padding (stat %a's shape), sorted by path relative to the image root."
  - "Task 1's resolver skeleton and Task 2's full contract are two separate commits behind the same harvey_dir(arch) signature; Task 2's rewrite adds resolve(), Via/Resolved, HARVEY_SRC, relativisation, the stamp check and the realiser."

requirements-completed: [HARN-02, HARN-03, HARN-06, HARN-07]

coverage:
  - id: D1
    description: "The tracer path boots: virtexec under HVF from an initrd assembled by the manifest engine through the resolver (resolver -> per-file manifest -> stage -> mkpaqfs -> boot)"
    verification:
      - kind: integration
        ref: "R9_HARVEY_DIR=/Users/neilskinner/code/harvey R9_MKPAQFS=/Users/neilskinner/code/harvey/.build/bin/mkpaqfs nix develop -c cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf -> boot-test: PASS"
      - kind: unit
        ref: "xtask/src/image.rs#image::tests (12 tests: parse/stage/member_list/bake inputs)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The manifest-assembled smoke image has the same member list (path, size, sha256, mode) as the pre-D-16 hard-coded assembler's (D-29)"
    verification:
      - kind: other
        ref: "diff $TMPDIR/r9-0108-legacy.members target/images/aarch64/smoke.members -> empty (commit 6a64287)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The one resolver serves every Harvey-tree consumer for both architectures, with override precedence, HARVEY_SRC, the named error, tree stamps and realise-on-first-use"
    verification:
      - kind: unit
        ref: "xtask/src/harvey.rs#harvey::tests (14 tests incl. the include_str! source invariant)"
        status: pass
      - kind: integration
        ref: "R9_HARVEY_DIR=... cargo xtask ureg-check -> 35 fields match arm64, 26 fields match amd64, negative control fired"
        status: pass
      - kind: integration
        ref: "env -u R9_HARVEY_DIR -u R9_HARVEY_DIR_AMD64 -u R9_HARVEY_DIR_ARM64 -u HARVEY_SRC cargo xtask ureg-check -> non-zero with R9_HARVEY_DIR_ARM64/HARVEY_SRC/vendor/harvey named"
        status: pass
    human_judgment: true
    rationale: "Whether the named-error prose is actually useful to a person without the plan in front of them is a judgment the greps cannot make; the string presence is automated, the wording is not."

duration: 4min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 08: Tracer — the per-architecture resolver and the manifest engine boot the aarch64 smoke image Summary

**The aarch64 smoke initrd is now assembled by r9 from a per-file manifest through one per-architecture resolver and one stage-and-bake engine, and virtexec boots it green under HVF; the manifest-assembled member list is byte-identical to the pre-D-16 assembler's (D-29).**

## Performance

- **Duration:** ~4 min of executor time (04:47:52Z to 04:51:44Z)
- **Started:** 2026-11-23T04:47:52Z
- **Completed:** 2026-11-23T04:51:44Z
- **Tasks:** 2 of 2
- **Files modified:** 8 (3 created, 5 modified; 1,498 insertions, 232 deletions)

## Accomplishments

- `xtask/src/harvey.rs` is the ONE Harvey-tree resolver (D-01, D-21, HARN-02): `harvey_dir(arch)` wraps `resolve(arch, lookup, workspace, realise)`, so precedence, the named error, the stamp check and realise-on-first-use are all unit-tested without touching the process environment. Precedence is `R9_HARVEY_DIR` (override, wins) > `R9_HARVEY_DIR_ARM64`/`_AMD64` > `HARVEY_SRC`; empty is unset; a relative value joins the workspace root; a per-arch tree must carry `HARVEY-TREE` with the matching `arch=` line; a missing `/nix/store` path is realised once with a printed `nix build <workspace>#packages.<system>.harvey-tree-<arch>` and a printed out path that differs from the exported one is a named error (D-05).
- `xtask/src/image.rs` is the generic stage-and-bake engine (D-15/D-16): `parse_manifest` (schema 1, `deny_unknown_fields` on every table, `..`/absolute rejection, duplicate-path rejection), `stage` (recreates the staging tree, copies from the read-only tree, applies the manifest mode, refuses a source that canonicalises outside the tree), `member_list` (sorted, path/size/sha256/octal mode for files plus dir lines), `write_members`, `bake`, and the moved `sha256_file`.
- `xtask/manifests/arm64/smoke.toml` declares the eight smoke binaries and `/proc`; `assemble_initrd` loads it, resolves the tree, takes `R9_MKPAQFS` explicitly (no derived default), stages under `target/images/aarch64/smoke.stage/`, bakes `target/bootfs.paq` (config_virtexec unchanged) and writes `target/images/aarch64/smoke.members`.
- All seven Harvey-tree consumer sites are routed through the one resolver: ureg-check's aarch64 and amd64 arms (each reporting its own resolution error instead of skipping), the boot-test precondition, the `harvey:` initrd scheme, the `harvey:` dist module, the initrd assembler and its staleness check, and the three pcap netaddr readers. `uregcheck`'s zero-argument resolver is deleted; `Arch` is threaded through `resolved_file`, `loader_device_arg`, `apply_to_qemu_step_with_overrides`, `prepare_initrd_dtb`, `ensure_initrd_image` and `resolve_boot_module`.
- The tracer is proven end to end: `boot-test --arch aarch64 --config virtexec --accel hvf` prints `boot-test: PASS -- banner, exec entry, init print, hello print and all-exited markers observed in order, stacksmoke depth-64 marker present`.

## Kernel-byte baseline (phase baseline for later plans)

Built at plan start and re-hashed after the last plan commit; both unchanged.

| Target/config | sha256 (baseline = final) |
|---|---|
| `target/x86_64-unknown-none-elf/debug/x86_64` (`--arch x86-64 --config q35churnshell`) | `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f` |
| `target/aarch64-unknown-none-elf/debug/aarch64` (`--arch aarch64 --config virtshell`) | `76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2` |

This plan changes no kernel or config byte, so both hashes are identical before and after; they are recorded here as the comparison baseline for the plans that follow.

## D-29 member-list evidence

The legacy assembler ran BEFORE the rewrite (`R9_HARVEY_DIR=<checkout> cargo xtask initrd --arch aarch64`, paq sha256 `c2f171f6a8de9ea568522a7c2989f670864ed86604c4aad59bbeb51e89d5832b`) and its `target/initrd-tree` member list was saved to `$TMPDIR/r9-0108-legacy.members`. After the rewrite the engine produced `target/images/aarch64/smoke.members`, and `diff` between the two is **empty**:

```
churnmark file 775 37184 69cbe744e9431c6aaedbe5ad89cbfa2328da2708b8def25cdb38a4e7bf9b615d
churnsem file 775 64269 90cb4775c84d23d7e379e46448b590fa631559cddeb013a5fd7af6ba350fd22e
hello file 755 37165 1c89e0990e386ae503d9f513d3a6c1985222f772d1edb40016c19aa3881abb04
init file 775 39563 c674f6ae5d5157dea6eef8955480d17a3b277a31285f67851568b06742ffe3eb
proc dir 755 - -
smokefork file 775 38003 3d77ea005438140caa6ed9c8f792ed53e9b03456d7e12b32a39baff6007d8363
smokenote file 775 37778 e14fd69cabeabf474777d7cda4343571cd75be06429fed53702b1df629a05655
smokesleep file 775 38075 ce97b1e802cc5d27cb0a3a1543cee0f983ffcca8f11b2a6d0887a95e52a5a75b
stacksmoke file 775 37442 f17a6469f907951ca6aaefa88cbe202ee231f538517431017adee578b63c27a1
```

The two `.paq` digests differ (`c2f171f6...` vs the engine bake `5af426d7...`) because mkpaqfs embeds a time field in every bake — exactly why D-29 compares member lists rather than bytes (A12 also refuted two-bake determinism).

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer): resolver -> smoke manifest -> image engine -> virtexec boot under HVF** - `6a64287` (feat)
2. **Task 2: complete the resolver (HARN-02) and route every consumer through it** - `a297253` (feat)

**Plan metadata:** the `docs(01-08): complete ...` commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `xtask/src/harvey.rs` (created) — the one resolver: `harvey_dir`, `resolve`, `Resolved`, `Via`, the named error, the stamp check, the realiser, and the source-invariant test
- `xtask/src/image.rs` (created) — manifest schema and validation, staging, member list, bake, sha256
- `xtask/manifests/arm64/smoke.toml` (created) — 8 `[[file]]` + 1 `[[dir]]`, modes matching the staged binaries
- `xtask/src/main.rs` (modified) — `assemble_initrd` on the engine, `mkpaqfs_path` (R9_MKPAQFS), `smoke_manifest`, `smoke_initrd_stale`, arch-threaded initrd/module paths, updated clap about texts; `INITRD_FILES`/`INITRD_TREE`/`sha256_of` deleted
- `xtask/src/lib.rs` (modified) — `pub mod harvey; pub mod image;` and the doc correction
- `xtask/src/uregcheck.rs` (modified) — zero-argument resolver deleted, two per-arch arms, arch-aware boot-test precondition
- `xtask/src/config.rs` (modified) — `resolved_file(arch)`, `loader_device_arg(arch)`, `apply_to_qemu_step_with_overrides(..., arch)`, `prepare_initrd_dtb(..., arch)`
- `xtask/src/pcap.rs` (modified) — the three netaddr readers resolve `Arch::Aarch64` through `xtask::harvey::harvey_dir`

## Decisions Made

- **Per-file modes, not a uniform 0755.** `build/aarch64/smoke-out/` carries 0775 on seven files and 0755 on `hello`; the old assembler preserved them through `fs::copy`, and the member list records mode, so a uniform 0755 would have broken the empty-diff acceptance criterion. The manifest carries the real modes and says why in its header comment.
- **`R9_MKPAQFS` has no derived default.** The plan's supply model is explicit-environment-only; mkpaqfs is a host tool, not a tree file, so it is its own named input with an error that names both the future devshell export and the current checkout path.
- **A non-aarch64 `target/` initrd is a named error.** `ensure_initrd_image` would otherwise bake the aarch64 smoke image for an x86_64 profile; no such profile exists today, and the loud refusal is the correct shape until the x86_64 manifests land.
- **Forward-declared `source` kinds stay loud.** `rust`/`generated`/`tree` parse (so later manifests do not break schema 1) but `stage` refuses them with a named error naming the plans that implement them.
- **TDD note.** Both tasks carry `tdd="true"`, but `workflow.tdd_mode` is false and the orchestrator requires one commit per task; the behaviour blocks drove the test suites (12 image + 14 harvey tests), yet tests and implementation landed in the same commit, so there is no separate RED observation to record honestly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Manifest modes follow the staged binaries, not the plan's uniform 0755**
- **Found during:** Task 1 step 5 (the D-29 legacy-versus-engine check)
- **Issue:** The plan says each `[[file]]` entry carries `mode = "0755"`, but the legacy `target/initrd-tree` has 0775 on seven members (`fs::copy` preserves the source mode) and 0755 only on `hello`. A uniform 0755 would make the required empty member-list diff impossible and would silently change seven members' mode bits in the image.
- **Fix:** The manifest carries `0775` for the seven and `0755` for `hello`; the manifest header records the reason.
- **Files modified:** `xtask/manifests/arm64/smoke.toml`
- **Commit:** `6a64287`

**2. [Rule 2 - Missing Critical] `ensure_initrd_image` refuses a non-aarch64 `target/` initrd**
- **Found during:** Task 2 (threading `Arch` through the initrd path)
- **Issue:** With the arch threaded in, an x86_64 profile naming a `target/` initrd would still be assembled from the arm64 smoke manifest — the wrong architecture's userland, silently.
- **Fix:** A named error explains that only the aarch64 manifest exists and that no `target/`-resident image exists for the requested arch yet. No committed config hits it.
- **Files modified:** `xtask/src/main.rs`
- **Commit:** `a297253`

**3. [Rule 2 - Missing Critical] Rejected a manifest source that canonicalises outside the tree**
- **Found during:** Task 1 (threat T-01-27)
- **Issue:** Lexical `..` rejection does not stop a symlink inside the Harvey tree from pointing outside it, which would let a manifest copy arbitrary host files into an image.
- **Fix:** `stage` canonicalises each source and refuses it unless it starts with the canonicalised tree root.
- **Files modified:** `xtask/src/image.rs`
- **Commit:** `6a64287`

---

**Total deviations:** 3 auto-fixed (1 bug, 2 missing critical)
**Impact on plan:** All three protect a stated acceptance criterion or a threat mitigation; no scope creep, no architecture change.

### Plan verification drift

- The plan's acceptance criterion `grep -c 'env::var("R9_HARVEY_DIR' … | grep -vc harvey.rs` passes, but the source-invariant test's own needle for the old resolver name had to be written as `concat!("uregcheck::", "harvey_dir")` so `harvey.rs` does not itself trip the plan's `grep -c 'uregcheck::harvey_dir' == 0` check.
- `HowItWorks.md` still describes the pre-plan `uregcheck::harvey_dir()` contract. The plan marks that section read-only for this plan and assigns the update to 01-13; it was not edited.

## Issues Encountered

- **`requirements-completed` is copied verbatim from the plan, but REQUIREMENTS.md was not ticked.** `requirements.ready-ids` reports all four IDs (`HARN-02`, `HARN-03`, `HARN-06`, `HARN-07`) blocked because sibling plans in this phase also declare them and have no SUMMARY yet. The shared-ID gate is behaving as designed; the later plans' completions will release the ticks.
- **The two `.paq` digests differ across bakes.** Expected: mkpaqfs embeds a time field, which is why D-29 uses the member list. Recorded above so a later plan does not mistake it for drift.

## Known Stubs

None. `Source::{Rust, Generated, Tree}` are declared enum variants with no staging implementation, but a manifest naming one fails loudly (not silently), and this plan's manifests use only `harvey`.

## Threat Flags

None beyond the plan's threat model. The new surface it predicted (manifest paths into the staging root, the resolver's `nix build`, the per-arch stamp) is implemented with its mitigations: `deny_unknown_fields`, `..`/absolute rejection, canonicalised-source containment, printed `Via`, stamp check, and a realiser limited to `packages.<system>.harvey-tree-<arch>` with an exported-path equality check.

## Next Phase Readiness

- The tracer contract is settled: later plans add manifests and supply (01-13's flake tree + devshell `R9_MKPAQFS`) without changing xtask's architecture.
- Override precedence and the named error are unit-tested for both architectures; the flake-supplied `R9_HARVEY_DIR_ARM64` path is exercised only by unit tests today — the real `nix build` realisation is plan 01-13's first live run.
- `cargo xtask test`, `clippy` and `check` are green; Harvey is untouched (HEAD `25d97dc4d` and status hash `041d2929…`); zero `port/src` changes; both kernel ELFs byte-identical to the baseline.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `xtask/src/harvey.rs`, `xtask/src/image.rs`, `xtask/manifests/arm64/smoke.toml`, `01-08-SUMMARY.md`.
- Task commits `6a64287` and `a297253` are ancestors of HEAD.
- Task 1 acceptance: `grep -c 'const INITRD_FILES' xtask/src/main.rs` = 0; smoke manifest has 8 `[[file]]` / 1 `[[dir]]`; `deny_unknown_fields` count = 4; virtexec HVF boot prints the PASS line; legacy-versus-engine diff empty; Harvey status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`.
- Task 2 acceptance: `pub fn harvey_dir()` in uregcheck.rs = 0; `HARVEY_SRC` in harvey.rs = 16; `R9_HARVEY_TREE_SYSTEM` = 5; direct env reads outside harvey.rs = 0; `uregcheck::harvey_dir` sites = 0; no-env ureg-check prints the named error; override ureg-check prints both field-match lines; `cargo xtask test`/`clippy`/`check` green.
- Plan verification: Harvey untouched; zero `port/src` changes; both kernel ELFs byte-identical to the recorded baseline.

