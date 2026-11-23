---
phase: 01-stage-p0-cross-build-harness
plan: 16
subsystem: cross-build-harness
tags: [xtask, image-assembler, authdomain, p9sk1, ndb-template, manifests, fonts, q35cpu, d-16, d-29, harn-01, harn-03, harn-06]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-11's manifest engine and generator registry seat; 01-14's R9_AUTHDOMAIN_GEN (Linux-only) and the amd64 tree; 01-15's nine reference member lists (R9_HARVEY_REFERENCE_AMD64)"
provides:
  - "xtask/src/image.rs: the authdomain and ndb-template generators, the per-bake p9sk1 flow (generate, pair-check over staged copies, 0600 creds, fresh domain per bake), and the secret-hygienic log"
  - "xtask/manifests/amd64/{cpu,auth}.toml: per-file manifests whose member lists equal the reference bake's (4442 / 95 members)"
  - "xtask/manifests/amd64/fonts-bit.members: the 4303-file font payload, one [[tree]] entry"
  - "nix/harvey/tree-amd64.nix: the auth image's host-built clients (hsvctcpecho, hauthprobe) as tree artifacts, byte-identical to the reference"
  - "x86_64/lib/config_q35cpu.toml: module = \"image:cpu\" (HARN-06)"
affects: [01-19, 01-22, 01-29, HARN-01, HARN-03, HARN-06, D-16, D-29]

actuals:
  tokens: 123451
  tasks: 2
  commits: 4

plan_head_before: 237611e7b0b2b9bc3cfa110e09eec231a725bcef
plan_head_after: f69b072a9d511d6070812cf8a9a0eec1fb5c3f9c

tech-stack:
  added: []
  patterns:
    - "Generator registry entries stage either a produced artifact (authdomain, named by `from`) or a substituted tree template (ndb-template, with `tokens`/`seam`/`append`); the generated-file mode is mandatory"
    - "The bake-time secret flow mirrors mkimage §14/§15 step for step: -o into target/images/x86_64/<image>.authdom, -i for the authdom token, -V over a scratch twin of the STAGED copies, creds at 0600, then the domain directory deleted"
    - "The generator path and log sink are parameters of the assemble seam, so tests drive a fake generator and scan every emitted line for the password without mutating the process environment"
    - "Host-side bake programs are tree-built artifacts the assembler only copies (plan 01-19's rule); 6c/6l output depends on the build's path strings, so the tree builds them with the reference's exact cwd/staging shape"

key-files:
  created:
    - xtask/manifests/amd64/cpu.toml
    - xtask/manifests/amd64/auth.toml
    - xtask/manifests/amd64/fonts-bit.members
  modified:
    - xtask/src/image.rs
    - nix/harvey/tree-amd64.nix
    - x86_64/lib/config_q35cpu.toml

key-decisions:
  - "Generated entries reuse `from` for the per-generator source (authdomain: the produced artifact name; ndb-template: the tree-relative template) and add `tokens`, `seam` and `append`; `mode` is required so the reference's 0600/0644/0444 bits survive staging."
  - "ndb-template resolves @AUTHDOM@ from the same bake's authdomain -i output and @GUESTV4@/@MASKV4@/@GWV4@/@ETHER@ from the tree's netaddr seam, exactly mkimage §10b/§15e; the auth image appends the tcp=ticket service row via `append`."
  - "The two --compare-reference runs were realised on the registered aarch64-linux lima builder (the only platform where authdomain-gen builds): the Mac runs the assembler, tree reads and mkpaqfs; a session-only shim (target/r9-lima-authdomain-gen, gitignored) forwards -o/-i/-V into the shared worktree. Plan 01-22's stop-B step still runs them natively on the KVM host (its conditional is satisfied)."
  - "The auth image's hsvctcpecho/hauthprobe are built in the amd64 tree with the host 6c/6l, same source/flags as mkimage, in a /build/harvey copy with the reference's staging shape; this reproduces the reference shas a93df702 and 2965d9b9 byte for byte."

requirements-completed: [HARN-03, HARN-01, HARN-06]

coverage:
  - id: D1
    description: "The bake-time p9sk1 auth domain is an r9 generator: fresh -o per bake, -i authdom token, -V pairing assert over the staged copies, 0600 creds file, domain dir deleted, no password in any emitted line"
    requirement: HARN-01
    verification:
      - kind: unit
        ref: "xtask/src/image.rs#authdomain_stages_homes_writes_0600_creds_and_never_logs_the_password (+ a_second_bake_overwrites_the_creds_with_new_values, a_refused_pairing_is_an_image_generator_authdomain_pairing_error, an_info_run_without_an_authdom_token_fails_named)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The cpu image (4442 members) is r9-assembled from cpu.toml and equals the reference member list; the font payload is one [[tree]] entry with 4303 members"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image cpu --compare-reference -> matches the reference member list (4442 members)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The auth image (95 members) is r9-assembled from auth.toml and equals the reference member list, including the two host-built clients; auth material compares by path, size and mode"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image auth --compare-reference -> matches the reference member list (95 members)"
        status: pass
    human_judgment: false
  - id: D4
    description: "q35cpu names image:cpu and builds; the kernel bytes are unchanged"
    requirement: HARN-06
    verification:
      - kind: other
        ref: "grep -c 'module = \"image:cpu\"' x86_64/lib/config_q35cpu.toml = 1; harvey: modules = 0; cargo xtask build --arch x86-64 --config q35cpu exits 0; target/x86_64-unknown-none-elf/debug/x86_64 sha256 50b01c9a... (q35churnshell) = the 01-08 baseline"
        status: pass
    human_judgment: false
  - id: D5
    description: "Secret containment: creds files are 0600 under target/, no domain directory survives a bake, target/ never appears in git status, and the password never occurs in the captured image-command output"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "/usr/bin/stat -f '%Lp' target/images/x86_64/{cpu,auth}.creds | sort -u = 600; no *.authdom* dirs; git status --porcelain has no target entry; grep -F <password> of the captured CPU-AUTH-MATCH log = no match; unit test asserts the same over assemble_checked's log"
        status: pass
    human_judgment: false

duration: 2h 20m
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 16: The cpu and auth images, with the bake-time auth domain in r9's assembler Summary

**r9's assembler now owns the last piece of Harvey's bake: each `cpu`/`auth` assembly runs a fresh p9sk1 domain through `R9_AUTHDOMAIN_GEN`, pair-checks it over the staged copies, writes a 0600 creds file under `target/`, composes `/lib/ndb/local` from the tree's template and netaddr seam, and bakes images whose member lists equal the reference bake (4442 and 95 members); q35cpu names `image:cpu`.**

## Performance

- **Duration:** 2h 20m (2026-11-23T08:47:02Z → 2026-11-23T11:07:33Z), dominated by five amd64 tree builds and two reference bakes while the auth image's missing host-built inputs were diagnosed and reproduced
- **Started:** 2026-11-23T08:47:02Z
- **Completed:** 2026-11-23T11:07:33Z
- **Tasks:** 2 of 2
- **Files modified:** 6 (3 created, 3 modified; 6605 insertions, 33 deletions)
- **Plan estimate:** 75,000 tokens; **actual:** 123,451 tokens (chars/4 over the realized diff, which includes the 4303-line members file). The estimate was low-confidence; the overshoot is the generated members file plus the generator implementation and tests.

## Accomplishments

- **The bake-time auth domain is an r9 generator.** `assemble_checked` runs `R9_AUTHDOMAIN_GEN -o target/images/x86_64/<image>.authdom/` from the Harvey tree, stages each produced artifact at its manifest guest home, runs `-V` over a scratch twin of the **staged copies** (refusing on mismatch with `IMAGE-GENERATOR authdomain-pairing`), writes `target/images/x86_64/<image>.creds` at 0600 with exactly `user=`/`password=`, and deletes the domain directory. A second bake overwrites the creds with new values and a stale directory never survives.
- **Secret hygiene is enforced and tested.** The generator's output is filtered through the same run's password before it reaches the log; the tests scan the captured log for the fake password, assert the creds mode is `0o600`, and assert no creds file exists when the pairing assert refuses.
- **The ndb composition is declarative.** `ndb-template` substitutes only the tokens the manifest names (`@AUTHDOM@` from the same bake's `-i`; `@GUESTV4@/@MASKV4@/@GWV4@/@ETHER@` from the tree's `lib/netaddr.rc`), appends the auth image's `tcp=ticket port=567` row, and fails on any token left in the output.
- **The cpu image is reference-equal.** `cpu.toml` (4442 members) is engine-emitted then resolved: the six opaque p9sk1 paths are authdomain entries, `/lib/ndb/local` is the ndb-template composition, and the whole font payload is one `[[tree]]` entry with `fonts-bit.members` (4303 files). `--compare-reference` prints "matches the reference member list (4442 members)".
- **The auth image is reference-equal.** `auth.toml` (95 members) resolves the same way, with `boot/authfeed.txt` and the two host-side clients. `--compare-reference` prints "matches the reference member list (95 members)".
- **The auth image's host-built clients are tree artifacts.** `hsvctcpecho` and `hauthprobe` are built in `harvey-tree-amd64` with the host 6c/6l, same source and flags as mkimage §14i2/§14i3, in a `/build/harvey` copy with the reference's staging shape; the tree files are byte-identical to the reference (`a93df702…`, `2965d9b9…`). The rebuilt reference's member lists are byte-identical to 01-15's.
- **q35cpu moves onto the r9 image.** `[dist] module = "image:cpu"`, no `harvey:` module remains, `cargo xtask build --arch x86-64 --config q35cpu` exits 0, and the q35churnshell kernel sha256 `50b01c9a…` equals the 01-08 baseline. Harvey HEAD is `25d97dc4d…` with status hash `041d2929…` (untouched).

## TDD Gate Compliance (Task 1)

| Phase | Commit | Evidence |
|---|---|---|
| RED | `7ea3a8e` test(01-16): add failing tests for the bake-time generator seam | `cargo test -p xtask --lib image::` → 30 passed, 9 failed; each failure is an assertion on the planned behaviour (manifest parse rejects `seam`/`append`, the generator registry is empty), not a build error |
| GREEN | `87ab175` feat(01-16): implement the bake-time authdomain and ndb-template generators | `cargo test -p xtask image::` → 39 lib tests pass; `cargo test -p xtask` → 89 + 1 + 79 + 3 doctests pass; `cargo clippy -p xtask --all-targets` clean |
| REFACTOR | — | none needed |

The RED evidence is recorded by command output; the `tdd-red-evidence` classifier has no cargo/libtest adapter (the reference's documented Rust gap, #4379), and the plan frontmatter is `type: execute`, so the plan-level gate is not asserted.

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED): failing tests for the generator seam** — `7ea3a8e` (test)
2. **Task 1 (GREEN): the authdomain and ndb-template generators** — `87ab175` (feat)
3. **Task 2 (blocking fix): the auth image's host-side clients in the tree** — `b693f85` (fix)
4. **Task 2: cpu/auth manifests, fonts members, q35cpu on image:cpu** — `f69b072` (feat)

**Plan metadata:** the `docs(01-16): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `xtask/src/image.rs` — the `authdomain` and `ndb-template` registry entries, `GeneratedBake`, `parse_creds`/`write_creds`, seam reading, token substitution, the pairing assert, the filtered log, and nine new tests (39 image tests total; 89 xtask tests)
- `xtask/manifests/amd64/cpu.toml` (created) — 121 blocks: 81 per-file members, 39 dirs, one `[[tree]]` font entry; the six generated entries
- `xtask/manifests/amd64/auth.toml` (created) — 95 blocks: 70 per-file members, 25 dirs; the six generated entries (authfeed, ndb local with the append)
- `xtask/manifests/amd64/fonts-bit.members` (created) — 4303 `f <rel> <size> <sha256> <mode>` lines
- `nix/harvey/tree-amd64.nix` (modified) — `build_auth_helpers`: hsvctcpecho/hauthprobe built with the host 6c/6l at the reference's path shape
- `x86_64/lib/config_q35cpu.toml` (modified) — `module = "image:cpu"` and the comment

## Decisions Made

- **Schema:** generated entries reuse `from` (produced artifact / template) and add `tokens`/`seam`/`append`; `mode` is required for generated entries. `deny_unknown_fields` still rejects typos.
- **Venue for the compare runs:** the generator is Linux-only (01-14), so the two compare runs were realised on the registered aarch64-linux lima builder. The Mac runs the assembler, tree reads and mkpaqfs; the shim `target/r9-lima-authdomain-gen` (gitignored, session-only) forwards `-o/-i/-V` into the shared worktree. The runs are real (fresh DEMO domain, real member lists), and plan 01-22's conditional `step_cpuauth_ref` still runs them natively on the KVM witness.
- **Host-built clients:** the tree builds them rather than the assembler compiling (plan 01-19's rule, and the assembler has no host 6c/6l). 6c/6l output depends on the build's path strings, so the tree build reproduces the reference's `/build/harvey` + `/build/mkimage-x64.XXXXXX/root-auth` shape; both shas match.
- **Font payload:** one `[[tree]]` entry (`from = "lib/font/bit"`) with a members file built from the reference's file lines; the explicit `[[dir]]`/`[[file]]` font blocks are removed so the tree entry owns the whole subtree.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The reference auth image needs two host-built clients the tree did not carry**
- **Found during:** Task 2's `--emit-manifest` for auth (8 unresolved entries, not 6: `amd64/bin/hsvctcpecho`, `amd64/bin/hauthprobe`)
- **Issue:** mkimage §14i2/§14i3 builds them with the host 6c/6l at bake time; the reference records their bytes, but 01-14's tree carried only their `.c` sources. The plan's action allows resolving to "the tree source of a renamed copy" and Task 1 registers only authdomain/ndb-template, so neither applies.
- **Fix:** added `build_auth_helpers` to `nix/harvey/tree-amd64.nix` (outside the plan's `files_modified`) — the same host toolchain, flags and source as the bake, in a `/build/harvey` copy with the reference's staging-path shape. The tree files equal the reference shas exactly (`a93df702…`, `2965d9b9…`), and a rebuilt reference produced byte-identical member lists. This follows the tree-built-artifact pattern plan 01-19 explicitly applies to the services section's host builds.
- **Files modified:** `nix/harvey/tree-amd64.nix`
- **Verification:** `shasum` of the tree's `build/x86_64/out/{svctcpecho,authprobe}` equals the reference `auth.members` rows; `diff` of the rebuilt reference's cpu/auth member lists against 01-15's = identical; both `--compare-reference` runs pass.
- **Commit:** `b693f85`

### Verification-scope notes (plan text vs the environment)

1. **The plan's `stat -f '%Lp'` assumes BSD `stat`.** The shell's `stat` is uutils coreutils 0.12, which reads `-f` as a filesystem argument. `/usr/bin/stat -f '%Lp'` prints `600` for both creds files; the plan's intent is satisfied.
2. **`cargo test -p xtask image::` prints "running 0 tests" for the two non-lib targets.** The image tests live in the library crate (39 pass); the binary targets filter them out. The 01-15 precedent (its deviation 6) applies.
3. **The font count is 4303 files, not 4321 reference lines.** The reference lists 18 directories under `lib/font/bit`; `fonts-bit.members` carries one line per **file**, as the `[[tree]]` members format requires. `awk '$1 ~ /^lib\/font\/bit\// && $2=="file"'` = 4303 = `wc -l`.
4. **The compare runs ran on the lima builder, not the Mac**, per the plan's Linux-only convention. The exact loop was run with `R9_AUTHDOMAIN_GEN` pointing at the session shim and printed `CPU-AUTH-MATCH`; without the shim the Mac devshell leaves the variable unset by design.

---

**Total deviations:** 1 auto-fixed (Rule 3 blocking, with a scope expansion), plus four verification-scope notes. **Impact:** the tree addition is required for the plan's must-have (the auth image cannot be assembled without the two clients); it changes no existing tree artifact and follows the pattern the next plan already plans to use.

## Issues Encountered

- **The reference's host-built clients are path-sensitive.** A naive tree build (guest 6c/6l, or the host 6c/6l at `/` or a long virtiofs cwd) produced binaries 2–200 bytes off the reference. A controlled probe isolated the cause to the build's cwd/staging path shape; matching the reference's `/build/harvey` + `/build/mkimage-x64.XXXXXX/root-auth` shape reproduces the exact shas. The tree build now encodes that shape and a comment explains why.
- **Two extra tree/reference rebuilds** were spent on the comment wording and the path-shape diagnosis; the final tree (`h187vs0v…`) and reference (`x7my22kg…`) reproduce the auth-helper bytes and the 01-15 member lists exactly.
- **`nix build` timeouts:** one 30-minute invocation was killed during output copy-back; rerunning the same derivation copied the completed output. No partial state was consumed.

## Known Stubs

None. All generated paths are produced by registered generators; no `UNRESOLVED` marker remains in either manifest; the auth clients are real tree artifacts.

## Threat Flags

None beyond the plan's threat model. T-01-52 is implemented by the 0600 creds file, the deleted domain directory, the filtered log, and the tests; T-01-53 by a fresh `-o` per bake plus the `-V` assert over the staged copies; T-01-54 by the leftover-token error. The tree addition builds two known Harvey sources with the pinned host toolchain; no new network or auth surface.

## User Setup Required

None — no external service configuration. On the Mac, `R9_AUTHDOMAIN_GEN` stays unset by design; the Linux devshell exports it.

## Next Phase Readiness

- **Plan 01-19** gets the `ndb-template` generator with the seam/tokens schema it expected, and the `build_auth_helpers` pattern for the services section's host builds (`hsvctcpecho` is the same artifact; `hmkdir` still needs its own build). Its "if tree-amd64.nix changed, the 01-15 marker/chain/substrate and 01-16 cpu/auth compare runs still pass" check holds: the rebuilt reference's lists are identical and both 01-16 compares pass.
- **Plan 01-22** should run the conditional `step_cpuauth_ref` on the KVM host: 01-14 recorded authdomain-gen as Linux-only, so the two compare runs are native there.
- **Plan 01-29** can move the remaining five x86_64 profiles; q35cpu is already on `image:cpu`. The auth image is assembled but no profile consumes it yet (q35auth moves to `image:substrate`).
- **The quick gate's image and credentials are r9's:** `target/images/x86_64/cpu.paq` and `cpu.creds` are produced by r9 with a fresh domain per bake, and the q35cpu profile assembles them on demand.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `xtask/manifests/amd64/{cpu.toml,auth.toml,fonts-bit.members}`, `01-16-SUMMARY.md` — FOUND; `nix/harvey/tree-amd64.nix`, `x86_64/lib/config_q35cpu.toml`, `xtask/src/image.rs` modified.
- Task commits `7ea3a8e` (RED), `87ab175` (GREEN), `b693f85` (tree fix), `f69b072` (Task 2) are ancestors of HEAD; `git rev-list --count 237611e..HEAD` = 4 (measured against the plan ledger).
- Task 1 verification re-run: `cargo test -p xtask image::` → 39 passed in the lib target (the two non-lib targets run 0); `grep -c R9_AUTHDOMAIN_GEN` = 6; `grep -c 0o600` = 6.
- Task 2 verification re-run: `CPU-AUTH-MATCH` (cpu 4442/4442, auth 95/95 members); `/usr/bin/stat -f '%Lp'` on both creds = `600`; `module = "harvey:` count = 0, `module = "image:cpu"` count = 1; `fonts-bit.members` 4303 lines = the reference's font file count; `UNRESOLVED` count = 0 in both manifests.
- Secret checks re-run: no `*.authdom*` directory remains; `git status --porcelain` has no `target` entry; the captured compare log does not contain the creds password.
- Plan verification re-run: q35churnshell kernel sha256 `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f` (01-08 baseline); Harvey HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e` (untouched); zero `port/src` changes.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
