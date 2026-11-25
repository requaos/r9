---
phase: 01-stage-p0-cross-build-harness
plan: 19
subsystem: cross-build-harness
tags: [xtask, image-assembler, manifests, amd64, net, services, tier, churn, ndb-template, netaddr-seam, hmkdir, sentinel, d-15, d-16, d-29, harn-03, harn-05]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-15's nine amd64 reference member lists (harvey-reference-amd64) and the emitter/compare workflow; 01-16's ndb-template generator, the authdomain seam and the build_auth_helpers tree pattern; 01-14's amd64 tree with the tier-0..4 userland, the net section and the D-19 source-side fixtures"
provides:
  - "xtask/manifests/amd64/net.toml: 64 members, all plain harvey copies (§10)"
  - "xtask/manifests/amd64/services.toml: 4517 members (§11) with one [[tree]] font entry, the ndb-template /lib/ndb/local composition and the tree hmkdir/sentinel artifacts"
  - "xtask/manifests/amd64/tier.toml: 91 members (§9), the 37 h* renamed copies mapped to their tree sources"
  - "xtask/manifests/amd64/churn.toml: 52 members (§12)"
  - "xtask/src/image.rs: ndb-template `strip` (line literals removed before substitution, asserted both ways) with three unit tests"
  - "nix/harvey/tree-amd64.nix: the §11k hmkdir host build (build/x86_64/out/mkdir, sha 02f45c07) and the tier/services sentinel fixture (build/x86_64/tier1-sentinel.txt, sha cd180d3f)"
affects: [01-23, 01-24, HARN-03, HARN-05, D-15, D-16, D-29]

actuals:
  tokens: 12868
  tasks: 2
  commits: 4

plan_head_before: f454f05556b82a90c7403eddff8ff77bb00a5770
plan_head_after: 78f5cdf7d5f3dd6243960615aaa5ab6d0ce092bd

tech-stack:
  added: []
  patterns:
    - "The ndb composition is a declarative `strip` list on the ndb-template entry: mkimage §11e2's sed deletes the whole line containing the named literal before substitution, and a literal that matches no line is an IMAGE-GENERATOR error (the strip is asserted both ways)"
    - "Generated bake-time files with no producer become tree fixtures when the bytes are static (the sentinel carries mkimage's exact printf text), so the assembler only ever copies; a file with a real producer stays a registered generator"
    - "A host-side bake build is a tree artifact built at the reference's exact cwd and staging-path shape (6c/6l output depends on the strings); the r9svc1 prefix is the same six-character length as the bake's mktemp suffix"
    - "A large static payload stays one [[tree]] entry with a committed members list (fonts-bit.members), keeping the manifest reviewable without weakening the per-file member comparison"

key-files:
  created:
    - xtask/manifests/amd64/net.toml
    - xtask/manifests/amd64/services.toml
    - xtask/manifests/amd64/tier.toml
    - xtask/manifests/amd64/churn.toml
  modified:
    - xtask/src/image.rs
    - nix/harvey/tree-amd64.nix

key-decisions:
  - "The net image carries no /lib/ndb/local: the reference net.members lists only lib/ndb/local.template, and mkimage's seam composition is §P2b/§11e2 (services) plus §14f3 (auth/cpu). The plan text attributes it to net; the composition was implemented and verified on the image that actually has it, and net stays all-harvey copies."
  - "The services ndb is the ndb-template generator with tokens [@GUESTV4@, @MASKV4@, @GWV4@, @ETHER@] and strip [authdom=@AUTHDOM@]; the strip deletes the whole ipnet line (the auth row shares it at the pin), exactly matching the reference local (sha 66353f6b, 2698 bytes)."
  - "hmkdir is built in the tree by build_services_helpers with the bake's own 6c/6l, flags and relative source path, in /build/harvey and the /build/mkimage-x64.r9svc1/root-svc staging shape; the harvested build/x86_64/out/mkdir equals the reference hmkdir byte for byte (02f45c07). hsvctcpecho was already a byte-equal tree artifact from 01-16."
  - "The tier and services sentinel is one tree printf fixture (build/x86_64/tier1-sentinel.txt, mkimage's exact bytes); the emitter maps both reference entries (lib/tier1/sentinel.txt, boot/sentinel.txt) to it."
  - "The 4303 font files are one [[tree]] entry (fonts-bit.members), the 01-16 cpu precedent; the explicit /lib/font/bit dir entry is dropped so the tree entry owns the root."
  - "The cpu/auth compare runs used the documented Linux-only generator shim (01-16's convention): the Mac devshell leaves R9_AUTHDOMAIN_GEN unset by design, so a session-only target/ shim ran authdomain-gen inside lima-harvey and tarred the domain directory across."
  - "The rebuilt reference (aqa9fpxi) emits all nine member lists byte-identical to 01-15/01-16's (x7my22kg), so the tree additions changed no baked image; the tree change is additive only."

requirements-completed: [HARN-03, HARN-05]

coverage:
  - id: D1
    description: "The net image (64 members) is r9-assembled from a per-file manifest whose member list equals the §10 reference bake"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image net --compare-reference -> matches the reference member list (64 members)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The services image (4517 members) is r9-assembled and reference-equal, including the ndb seam composition, the tree hmkdir and the font tree"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image services --compare-reference -> matches the reference member list (4517 members)"
        status: pass
      - kind: unit
        ref: "xtask/src/image.rs#ndb_template_strips_the_named_rows_and_substitutes_the_seam_values (+ strip-that-matches-no-line, missing-seam-key)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The tier image (91 members) is r9-assembled and reference-equal; the 37 h* entries are renamed copies mapped to their tree sources"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image tier --compare-reference -> matches the reference member list (91 members)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The churn image (52 members) is r9-assembled and reference-equal"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image churn --compare-reference -> matches the reference member list (52 members)"
        status: pass
    human_judgment: false
  - id: D5
    description: "All nine x86_64 image contracts are r9-assembled and match the reference bake in one loop; HARN-03's x86_64 enumeration is complete"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "for n in marker chain substrate cpu auth net services tier churn: cargo xtask image --arch x86-64 --image $n --compare-reference -> NINE-MATCH (30/49/27/4442/95/64/4517/91/52 members)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The assembler does not compile; the services host program and the generated sentinel are tree artifacts equal to the reference bytes"
    requirement: HARN-05
    verification:
      - kind: other
        ref: "shasum build/x86_64/out/mkdir = 02f45c07 (reference hmkdir 02f45c07); build/x86_64/tier1-sentinel.txt = cd180d3f (reference lib/tier1/sentinel.txt and boot/sentinel.txt); no xtask build step invokes 6c/6l"
        status: pass
    human_judgment: false
  - id: D7
    description: "No kernel bytes changed and Harvey stayed read-only"
    requirement: HARN-05
    verification:
      - kind: other
        ref: "target/x86_64-unknown-none-elf/debug/x86_64 sha256 50b01c9a... = the 01-08 baseline; target/aarch64-unknown-none-elf/debug/aarch64 sha256 76e6e153... = the baseline; Harvey HEAD 25d97dc4d..., status hash 041d2929...; zero port/src changes"
        status: pass
    human_judgment: false

duration: 36min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 19: The net, services, tier and churn images Summary

**All nine x86_64 image contracts are now r9-assembled: the net (64), services (4517), tier (91) and churn (52) manifests are D-29-equal to the reference bake, the services ndb is composed from the tree template through the netaddr seam with a tested row strip, and the section's two bake-time programs — hmkdir and the tier/services sentinel — are byte-equal tree artifacts the assembler only copies.**

## Performance

- **Duration:** 36 min (2026-11-23T11:19:32Z → 2026-11-23T11:56:01Z), dominated by the amd64 tree rebuild and the full nine-image reference bake
- **Started:** 2026-11-23T11:19:32Z
- **Completed:** 2026-11-23T11:56:01Z
- **Tasks:** 2 of 2
- **Files modified:** 6 (4 created, 2 modified; 2387 insertions, 3 deletions)
- **Plan estimate:** 65,000 tokens; **actual:** 12,868 tokens (chars/4 over the realized diff, which includes the 1,118-line services manifest). The estimate sized the reference investigation; the wall time went to the builder runs, which the token measure does not see.

## Accomplishments

- **The net image is reference-equal.** `net.toml` is the emitter's 64 entries, all `source = "harvey"`: the chain tree, the ip container's three binaries, tcpecho/tcpblast/netblast, the utility set, and the `lib/netaddr.rc` + five `netip-*.rc` fixtures riding the skeleton. `--compare-reference` prints "matches the reference member list (64 members)". The reference carries no `/lib/ndb/local` for net — only the template — so the plan's net-ndb attribution is a plan-text defect (deviation 1).
- **The services image is reference-equal and carries the composed ndb.** `services.toml` has 4517 members: the §11 chain set, the tier-1 h* helpers, the computed tier-2 h* set, the net/utility members, the §11k hmkdir, the sweep/termrc override, the /boot-side rcmain/sentinel/tcp9901/tcp9902, and the whole font tree as one `[[tree]]` entry (4303 files). `/lib/ndb/local` is the `ndb-template` composition with `strip = ["authdom=@AUTHDOM@"]`; the strip deletes the whole ipnet line (the auth row shares it), and the staged file equals the reference's 2698-byte local (sha `66353f6b`). `--compare-reference` prints "matches the reference member list (4517 members)".
- **The ndb strip is unit-tested.** Three new tests cover the services composition (`ndb_template_strips_the_named_rows_and_substitutes_the_seam_values`), a strip literal that matches no line (an `IMAGE-GENERATOR` error, so a template drift cannot silently keep the auth row), and a missing seam key naming the key. `cargo test -p xtask --lib image::` runs 42 tests, all passing.
- **The tier image is reference-equal.** `tier.toml` has 91 members: the chain set, the 37 `h*` renamed copies mapped to their tree sources (by the emitter's sha index, each one `amd64/bin/<name>`), the H* sweep, the tree sentinel, `nm-input.6`, `fpprobe`, and the tier termrc override. `--compare-reference` prints "matches the reference member list (91 members)".
- **The churn image is reference-equal.** `churn.toml` has 52 members: the chain tree plus `test`/`dc`, the `churncats` C driver and the committed `/lib/churn-cats.rc` fixture. `--compare-reference` prints "matches the reference member list (52 members)".
- **All nine contracts pass in one loop.** `NINE-MATCH` for marker/chain/substrate/cpu/auth/net/services/tier/churn with member counts 30/49/27/4442/95/64/4517/91/52. HARN-03's x86_64 enumeration is complete; HARN-05's drift/provenance checks run on every assembly.
- **The tree gained the two bake-time artifacts, byte-equal to the reference.** `build_services_helpers` builds `hmkdir` with the host 6c/6l at the reference's exact cwd/path shape — `build/x86_64/out/mkdir` has sha `02f45c07`, equal to the services reference's `amd64/bin/hmkdir`. `build/x86_64/tier1-sentinel.txt` carries mkimage's exact printf bytes (sha `cd180d3f`), mapping both the tier's `lib/tier1/sentinel.txt` and the services' `boot/sentinel.txt`. `hsvctcpecho` reuses 01-16's byte-equal `build/x86_64/out/svctcpecho`.
- **The rebuild changed no image.** The new tree (`8xjpnqdv…`) and reference (`aqa9fpxi…`) produce all nine member lists byte-identical to 01-15/01-16's (`x7my22kg…`); the tree additions are additive only. The 01-15 marker/chain/substrate and 01-16 cpu/auth compares still pass after the tree change (with the documented Linux-only generator shim for cpu/auth).
- **No kernel bytes changed; Harvey untouched.** q35churnshell `50b01c9a…` and virtshell `76e6e153…` equal the 01-08 baseline; Harvey HEAD `25d97dc4d…`, status hash `041d2929…`; zero `port/src` changes; `cargo xtask test/clippy/check` exit 0/0/0.

## TDD Gate Compliance (Task 1)

| Phase | Commit | Evidence |
|---|---|---|
| RED | `2dae9b4` test(01-19): add failing tests for the ndb services row strip | `cargo test -p xtask --lib image::tests::ndb_template_strips_the_named_rows_and_substitutes_the_seam_values` → exit 101, `test result: FAILED. 0 passed; 1 failed; 91 filtered out`; the target test failed on the planned schema gap (`unknown field 'strip'`), not a build error. The semantic assessment: the target executed and asserted the planned behaviour (the manifest must accept the §11e2 strip and compose the services ndb). |
| GREEN | `784ab95` feat(01-19): implement the ndb-template row strip | `cargo test -p xtask --lib image::tests::ndb_template` → 6 passed; `cargo test -p xtask --lib image::` → 42 passed, 0 failed; clippy exit 0 |
| REFACTOR | — | none needed |

The RED evidence is recorded by command output; the `tdd-red-evidence` classifier has no cargo/libtest adapter (the reference's documented Rust gap, #4379), and the plan frontmatter is `type: execute`, so the plan-level gate is not asserted.

## Task Commits

Each task was committed atomically (the TDD task carries its RED/GREEN commits plus the deliverable):

1. **Task 1 (RED): failing tests for the ndb services strip** — `2dae9b4` (test)
2. **Task 1 (GREEN): the ndb-template row strip** — `784ab95` (feat)
3. **Task 1: the net and services manifests + the tree artifacts** — `bd00879` (feat)
4. **Task 2: the tier and churn manifests** — `78f5cdf` (feat)

**Plan metadata:** the `docs(01-19): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `xtask/manifests/amd64/net.toml` (created) — 64 members, §10 at bd7a698, all harvey copies
- `xtask/manifests/amd64/services.toml` (created) — 4517 members, §11 at bd7a698: one `[[tree]]` font entry, the ndb-template local with the auth-row strip, the tree hmkdir/sentinel entries
- `xtask/manifests/amd64/tier.toml` (created) — 91 members, §9 at bd7a698, the h* renamed copies and the tree sentinel
- `xtask/manifests/amd64/churn.toml` (created) — 52 members, §12 at bd7a698
- `xtask/src/image.rs` (modified) — the `strip` field on `FileEntry`, `strip_lines`, its validation and three tests
- `nix/harvey/tree-amd64.nix` (modified) — `build_services_helpers` (hmkdir) and the tier/services sentinel fixture

## Decisions Made

- **The ndb seam composition lands on services, not net.** The reference proves net has only `lib/ndb/local.template`; mkimage composes `/lib/ndb/local` in §P2b/§11e2 (services) and §14f3 (auth/cpu). The services manifest uses the existing ndb-template generator with the four seam tokens and a new declarative `strip` list implementing §11e2's `sed "/authdom=@AUTHDOM@/d"`. Net stays all-harvey copies.
- **The strip is asserted both ways.** A literal matching no line fails the bake, so a template drift that drops the auth row cannot silently leave the row in an image with no domain; the existing leftover-token assert still guards the substituted output.
- **hmkdir is a tree artifact, not an assembler compile.** Built with the same host 6c/6l, flags and relative source path as the bake, in `/build/harvey` and the `/build/mkimage-x64.r9svc1/root-svc` staging shape; the harvest is byte-equal (`02f45c07`). The `r9svc1` prefix is the same six-character length as the bake's mktemp suffix, the 01-16 `r9auth` discipline.
- **The sentinel is a tree printf fixture.** mkimage generates the same 41 bytes in two images; the tree materialises them once as `build/x86_64/tier1-sentinel.txt` (the aarch64 precedent), and both reference entries map to it.
- **The font payload is one `[[tree]]` entry.** 4303 files in `fonts-bit.members` keep the services manifest reviewable; the member comparison is unchanged and still per-file.
- **The cpu/auth compares ran through the Linux-only generator shim.** The Mac devshell leaves `R9_AUTHDOMAIN_GEN` unset by design; a session-only `target/` shim ran authdomain-gen inside lima-harvey and tarred the domain directory across (01-16's documented convention).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan defect] The net image has no `/lib/ndb/local`; the seam composition is the services image's**

- **Found during:** Task 1 (`--emit-manifest` for net: 64/64 resolved, no generated entry; reading the reference's `net.members`)
- **Issue:** the plan says "the net image's `/lib/ndb/local` is composed from a template through the netaddr seam" and points at mkimage §10b, but §10b is the single-seam *assertion*; the compose is §P2b/§11e2 (services) and §14f3 (auth/cpu). The reference net image carries only `lib/ndb/local.template`.
- **Fix:** implemented the seam composition on the services image through a new declarative `strip` on the ndb-template generator (with three unit tests), and left net as the reference's all-harvey member set. Both images were then verified against their reference lists (64/64 and 4517/4517, including the composed local's bytes/sha).
- **Files modified:** `xtask/src/image.rs`, `xtask/manifests/amd64/services.toml`, `xtask/manifests/amd64/net.toml`
- **Verification:** services compare-reference passes with the local's 2698-byte sha `66353f6b`; the strip tests pass; net compare-reference passes 64/64.
- **Committed in:** `784ab95` (the generator) and `bd00879` (the manifests)

**2. [Rule 2 - Missing Critical] The amd64 tree had no file for mkimage's generated tier/services sentinel**

- **Found during:** Task 1 (tier `--emit-manifest`: 1 entry, `lib/tier1/sentinel.txt`, unresolved; services: `boot/sentinel.txt` in the same class)
- **Issue:** mkimage writes the 41-byte sentinel with `printf` at bake time; the assembler stages files and never generates, and the amd64 tree carried no fixture. 01-17 hit the same gap on arm64 and solved it with a tree fixture; the amd64 tree did not inherit it.
- **Fix:** the tree's install list materialises `build/x86_64/tier1-sentinel.txt` with mkimage's exact printf bytes; the emitter maps both reference entries to it (one `[[file]]` each in the tier and services manifests).
- **Files modified:** `nix/harvey/tree-amd64.nix`, `xtask/manifests/amd64/tier.toml`, `xtask/manifests/amd64/services.toml`
- **Verification:** the rebuilt tree's fixture sha is `cd180d3f`, equal to both reference entries; both compares pass.
- **Committed in:** `bd00879`

### Verification-scope notes (plan text vs the environment)

1. **`cargo test -p xtask image::` prints "running 0 tests" for the two non-lib targets.** The image tests live in the library crate (42 pass); the binary targets filter them out. The 01-15/01-16 precedent applies.
2. **The cpu/auth compare runs used the session-only Linux generator shim.** `R9_AUTHDOMAIN_GEN` is unset in the Mac devshell by design (01-14/01-16); the shim ran the store's authdomain-gen inside lima-harvey and tarred the fresh domain directory back to the Mac. The runs are real (fresh DEMO domain, pairing verified over the staged copies) and both print "matches the reference member list" (4442 / 95).

---

**Total deviations:** 2 auto-fixed (1 plan defect, 1 missing critical), plus two verification-scope notes. **Impact:** the plan's goal is met for all four images and all nine x86_64 contracts; the plan-defect fix puts the ndb composition on the image that actually carries it and makes the strip a tested, declarative rule; the sentinel fixture is the same additive tree pattern the aarch64 tree already uses.

## Issues Encountered

- **The rebuilt reference's outPath changed** (`x7my22kg…` → `aqa9fpxi…`) because the tree input changed, even though every member list is byte-identical and the bake's `== PASS` lines all print. The devshell export follows the evaluation, so the manifests and compares read the new reference without any path edits.
- **The tree rebuild was two derivations** (tree then reference) on the registered aarch64-linux external builder; both completed first-try, and the hmkdir/sentinel hashes were correct without a probe cycle.

## Known Stubs

None. Every one of the four manifests has zero unresolved entries; the services composition and the two tree artifacts are real, byte-equal files; no generated path is unimplemented.

## Threat Flags

None beyond the plan's threat model. T-01-63 (ndb composition tampering) is implemented by the single-seam read, the missing-key error, the strip-both-ways assert and the compare; T-01-64 (bake-time compilation drift) by building hmkdir with mkimage's own compiler, flags, source and path shape and proving byte equality in the compare; T-01-SC by no package installs.

## Next Phase Readiness

- **Plans 01-23 and 01-24 (the moved x86_64 gates) get all nine image contracts**: `cargo xtask image --arch x86-64 --image <name>` is D-29-equal for marker, chain, substrate, cpu, auth, net, services, tier and churn, so the moved gate scripts can point at `target/images/x86_64/*.paq` instead of a Harvey bake.
- **The reference is one derivation for all nine lists** (`aqa9fpxi…`), and the tree (`8xjpnqdv…`) is the single read-only supply; a later Harvey pin bump re-runs both and fails loudly on any list drift.
- **Harvey remains untouched** and both kernel baselines are intact after the tree and generator changes.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist on disk: `xtask/manifests/amd64/{net,services,tier,churn}.toml` and `01-19-SUMMARY.md` — all FOUND; `xtask/src/image.rs` and `nix/harvey/tree-amd64.nix` modified.
- Task commits `2dae9b4` (RED), `784ab95` (GREEN), `bd00879` (Task 1) and `78f5cdf` (Task 2) are ancestors of HEAD; `git rev-list --count f454f05..HEAD` = 4 (measured against the plan ledger).
- Task 1 verify re-run: net and services `--compare-reference` print "matches the reference member list" (64 / 4517) and `NET-SERVICES-MATCH`; `grep -c UNRESOLVED` across net and services = 0; the 01-15 marker/chain/substrate and 01-16 cpu/auth compares still pass after the tree change (`ALL-FIVE-MATCH`); `cargo test -p xtask --lib image::` = 42 passed (the two non-lib targets run 0 — 01-15/01-16 precedent).
- Task 2 verify re-run: tier and churn `--compare-reference` print (91 / 52) and `TIER-CHURN-MATCH`; the manifest count (non-libraries) = 9; `grep -c UNRESOLVED` across tier and churn = 0.
- Acceptance re-run: all nine x86_64 compares in one loop print `NINE-MATCH` (30/49/27/4442/95/64/4517/91/52); `UNRESOLVED` across the four manifests sums to 0; services has one `[[tree]]` font entry and one `strip` line; tier carries 37 h* and services 128 h* entries.
- Plan verification re-run: `cargo xtask test/clippy/check` exit 0/0/0; x86_64 q35churnshell kernel sha256 `50b01c9a…` and aarch64 virtshell `76e6e153…` equal the 01-08 baseline; Harvey HEAD `25d97dc4d…`, status hash `041d2929…`; zero `port/src` changes.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).

