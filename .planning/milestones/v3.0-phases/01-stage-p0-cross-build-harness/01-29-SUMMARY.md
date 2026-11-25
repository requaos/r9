---
phase: 01-stage-p0-cross-build-harness
plan: 29
subsystem: cross-build-harness
tags: [xtask, boot-profiles, image-assembler, q35exec, q35churn, q35shell, q35churnshell, q35auth, d-16, d-29, harn-06]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-11's `image:` scheme and manifest engine; 01-15's marker/chain/substrate manifests and the reference bake (R9_HARVEY_REFERENCE_AMD64); 01-16's cpu/auth images and q35cpu on image:cpu"
provides:
  - "x86_64/lib/config_q35exec.toml and config_q35churn.toml: module = \"image:marker\""
  - "x86_64/lib/config_q35shell.toml and config_q35churnshell.toml: module = \"image:chain\""
  - "x86_64/lib/config_q35auth.toml: module = \"image:substrate\""
  - "the whole x86_64 config set has no harvey: module (HARN-06, D-16)"
affects: [01-22, HARN-06, D-16, D-29]

actuals:
  tokens: 1117
  tasks: 1
  commits: 1

plan_head_before: f454f05556b82a90c7403eddff8ff77bb00a5770
plan_head_after: 61387b8444438c429e8ee37693bb2f7fe5eef18d

tech-stack:
  added: []
  patterns:
    - "A boot profile's [dist] module names an r9 image with `image:<name>`; resolve_boot_module materialises it on demand through ensure_named_image and stages target/images/x86_64/<name>.paq"

key-files:
  created: []
  modified:
    - x86_64/lib/config_q35exec.toml
    - x86_64/lib/config_q35churn.toml
    - x86_64/lib/config_q35shell.toml
    - x86_64/lib/config_q35churnshell.toml
    - x86_64/lib/config_q35auth.toml

key-decisions:
  - "The three images were re-checked with --compare-reference in this plan's own run BEFORE any config edit (member lists 30/49/27, all matching), satisfying D-29's rule that a profile is re-pointed only at an image verified in the same run."
  - "The [dist] module comments name the r9 manifest (xtask/manifests/amd64/<name>.toml) and the on-demand target/images path; no copy of a Harvey-baked path survives in any comment."
  - "No other config key changed: boot parameters, append lines, netdev tables and hostfwd pairs are byte-identical (the patch is exactly the module line plus its adjacent comment per file)."

requirements-completed: [HARN-06]

coverage:
  - id: D1
    description: "The five remaining x86_64 boot-test profiles (q35exec, q35churn, q35shell, q35churnshell, q35auth) name r9-assembled, reference-matched images, and no x86_64 profile reads a Harvey-baked image"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "for n in marker chain substrate: nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image $n --compare-reference -> matches the reference member list (30 / 49 / 27 members), THREE-STILL-MATCH (run before the edits and re-run after)"
        status: pass
      - kind: other
        ref: "FIVE-ON-IMAGE loop over q35exec:marker q35churn:marker q35shell:chain q35churnshell:chain q35auth:substrate; cat x86_64/lib/*.toml | grep -vE '^[[:space:]]*#' | grep -c 'module = \"harvey:' = 0; grep 'harvey:' x86_64/lib/config_q35*.toml = no match"
        status: pass
    human_judgment: false
  - id: D2
    description: "All five profiles still build (config parse and kernel build) and the kernel bytes are unchanged"
    requirement: HARN-06
    verification:
      - kind: other
        ref: "for c in q35exec q35churn q35shell q35churnshell q35auth: nix develop -c cargo xtask build --arch x86-64 --config $c -> FIVE-BUILD"
        status: pass
      - kind: other
        ref: "target/x86_64-unknown-none-elf/debug/x86_64 sha256 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f after a q35churnshell build = the 01-08 baseline"
        status: pass
    human_judgment: false
  - id: D3
    description: "The five profiles boot green under KVM from the r9-assembled images on the owner's x86_64 host"
    verification: []
    human_judgment: true
    rationale: "x86_64 ISO composition and KVM boots are Linux-host-only witness steps; the Mac cannot claim them. They are queued for stop B (plan 01-22, step_profiles) and must be judged from that run's log."

duration: 3min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 29: The five x86_64 marker, chain and substrate profiles on the assembled images Summary

**q35exec and q35churn now boot `image:marker`, q35shell and q35churnshell boot `image:chain`, and q35auth boots `image:substrate` — after all three images re-passed `--compare-reference` in this run — so no x86_64 profile reads a Harvey-baked image and every profile still builds with the kernel bytes at the 01-08 baseline.**

## Performance

- **Duration:** 3 min (2026-11-23T11:09:44Z → 2026-11-23T11:12:42Z)
- **Started:** 2026-11-23T11:09:44Z
- **Completed:** 2026-11-23T11:12:42Z
- **Tasks:** 1 of 1
- **Files modified:** 5 (30 insertions, 23 deletions)
- **Plan estimate:** 30,000 tokens; **actual:** 1,117 tokens (chars/4 over the realized diff). The estimate was low-confidence; the work was five two-line config changes once the reference gate had passed.

## Accomplishments

- **The D-29 gate ran first, on all three images.** `nix develop -c env -u R9_HARVEY_DIR cargo xtask image --arch x86-64 --image <marker|chain|substrate> --compare-reference` printed "matches the reference member list" for 30 / 49 / 27 members, and `THREE-STILL-MATCH`, **before any config edit**. No profile was re-pointed at an unverified image (T-01-93).
- **Five profiles moved to `image:`.** q35exec and q35churn: `module = "image:marker"`; q35shell and q35churnshell: `module = "image:chain"`; q35auth: `module = "image:substrate"`. Each adjacent comment now names `xtask/manifests/amd64/<name>.toml` and the on-demand `target/images/x86_64/<name>.paq` assembly; no copy of a Harvey-baked path remains in a comment.
- **The whole x86_64 config set is Harvey-free.** With 01-16's q35cpu on `image:cpu`, `cat x86_64/lib/*.toml | grep -vE '^[[:space:]]*#' | grep -c 'module = "harvey:'` prints **0**, and no `harvey:` token remains in any `config_q35*.toml` (HARN-06).
- **All five profiles build and the kernel bytes did not move.** `FIVE-BUILD` for q35exec/q35churn/q35shell/q35churnshell/q35auth; the q35churnshell kernel ELF sha256 is `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f`, equal to the 01-08 baseline.
- **D-12 checks green.** `cargo xtask test` 87 tests passed (0 failed), `cargo xtask clippy` exit 0, `cargo xtask check` exit 0.
- **Harvey untouched.** HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b`, status hash `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`; zero `port/src` changes.

## Task Commits

Each task was committed atomically:

1. **Task 1: Re-check the three images against the reference, then point the five profiles at them** — `61387b8` (feat)

**Plan metadata:** the `docs(01-29): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `x86_64/lib/config_q35exec.toml` — `[dist] module = "image:marker"`; comment names `xtask/manifests/amd64/marker.toml`
- `x86_64/lib/config_q35churn.toml` — `[dist] module = "image:marker"` (the 17-02 marker bake comment kept, now naming the r9 manifest)
- `x86_64/lib/config_q35shell.toml` — `[dist] module = "image:chain"`; comment names `xtask/manifests/amd64/chain.toml`
- `x86_64/lib/config_q35churnshell.toml` — `[dist] module = "image:chain"`; same comment wording
- `x86_64/lib/config_q35auth.toml` — `[dist] module = "image:substrate"`; comment names `xtask/manifests/amd64/substrate.toml` and randread as `/amd64/init`

No `xtask/`, `flake.nix`, manifest or tree file was touched; the `image:` scheme and manifests were already in place from 01-11/01-15/01-16.

## The commit's diff (`git show --stat --patch 61387b8 -- x86_64/lib`)

```text
 x86_64/lib/config_q35auth.toml       | 13 +++++++------
 x86_64/lib/config_q35churn.toml      |  7 +++++--
 x86_64/lib/config_q35churnshell.toml | 11 ++++++-----
 x86_64/lib/config_q35exec.toml       | 11 ++++++-----
 x86_64/lib/config_q35shell.toml      | 11 ++++++-----
 5 files changed, 30 insertions(+), 23 deletions(-)

diff --git a/x86_64/lib/config_q35auth.toml b/x86_64/lib/config_q35auth.toml
@@ -81,9 +81,10 @@ pcap = false
 [dist]
 format = "elf"
-# The staged bootfs (the 18-01 substrate bake): Harvey's mkimage §13
-# bakes out/bootfs-substrate.paq (randread as /amd64/init); the ISO
-# builder stages it at /boot/bootfs.paq and emits the grub.cfg module2
-# line IN.  Absence is a loud pre-ISO error naming the Harvey-side
-# producers (stage-build.sh + mkimage.sh).
-module = "harvey:build/x86_64/out/bootfs-substrate.paq"
+# The staged bootfs (the 18-01 substrate bake): the substrate image, now
+# r9-assembled.  `image:substrate` names
+# xtask/manifests/amd64/substrate.toml; the assembler bakes
+# target/images/x86_64/substrate.paq on demand (its member list matches
+# the reference bake's), with randread as /amd64/init.  The ISO builder
+# stages it at /boot/bootfs.paq and emits the grub.cfg module2 line IN.
+module = "image:substrate"
diff --git a/x86_64/lib/config_q35churn.toml b/x86_64/lib/config_q35churn.toml
@@ -66,5 +66,8 @@ pcap = false
 [dist]
 format = "elf"
 # The staged bootfs (the 17-02 marker bake: init/smokesleep PLUS
-# churnmark/churnsem — the churn driver walks the pair by name).
-module = "harvey:build/x86_64/out/bootfs.paq"
+# churnmark/churnsem — the churn driver walks the pair by name), now
+# r9-assembled: `image:marker` names xtask/manifests/amd64/marker.toml
+# and the assembler bakes target/images/x86_64/marker.paq on demand
+# (its member list matches the reference bake's).
+module = "image:marker"
diff --git a/x86_64/lib/config_q35churnshell.toml b/x86_64/lib/config_q35churnshell.toml
@@ -115,8 +115,9 @@ pcap = false
 [dist]
 format = "elf"
-# The CHAIN bootfs (D-03): Harvey's 14-06 bake produces this under the
-# checkout R9_HARVEY_DIR names; the ISO builder stages it at
-# /boot/bootfs.paq and emits the grub.cfg module2 line IN.  Absence is
-# a loud pre-ISO error naming the Harvey-side producers.
-module = "harvey:build/x86_64/out/bootfs-chain.paq"
+# The CHAIN bootfs (D-03), now r9-assembled: `image:chain` names
+# xtask/manifests/amd64/chain.toml and the assembler bakes
+# target/images/x86_64/chain.paq on demand (its member list matches the
+# reference bake's).  The ISO builder stages it at /boot/bootfs.paq and
+# emits the grub.cfg module2 line IN.
+module = "image:chain"
diff --git a/x86_64/lib/config_q35exec.toml b/x86_64/lib/config_q35exec.toml
@@ -84,8 +84,9 @@ pcap = false
 [dist]
 format = "elf"
-# The staged bootfs (D-03): Harvey's 14-05 bake produces this under the
-# checkout R9_HARVEY_DIR names; the ISO builder stages it at
-# /boot/bootfs.paq and emits the grub.cfg module2 line IN.  Absence is
-# a loud pre-ISO error naming the Harvey-side producers.
-module = "harvey:build/x86_64/out/bootfs.paq"
+# The staged bootfs (D-03): the marker image, now r9-assembled.
+# `image:marker` names xtask/manifests/amd64/marker.toml; the assembler
+# bakes target/images/x86_64/marker.paq on demand (its member list
+# matches the reference bake's).  The ISO builder stages it at
+# /boot/bootfs.paq and emits the grub.cfg module2 line IN.
+module = "image:marker"
diff --git a/x86_64/lib/config_q35shell.toml b/x86_64/lib/config_q35shell.toml
@@ -90,8 +90,9 @@ pcap = false
 [dist]
 format = "elf"
-# The CHAIN bootfs (D-03): Harvey's 14-06 bake produces this under the
-# checkout R9_HARVEY_DIR names; the ISO builder stages it at
-# /boot/bootfs.paq and emits the grub.cfg module2 line IN.  Absence is
-# a loud pre-ISO error naming the Harvey-side producers.
-module = "harvey:build/x86_64/out/bootfs-chain.paq"
+# The CHAIN bootfs (D-03), now r9-assembled: `image:chain` names
+# xtask/manifests/amd64/chain.toml and the assembler bakes
+# target/images/x86_64/chain.paq on demand (its member list matches the
+# reference bake's).  The ISO builder stages it at /boot/bootfs.paq and
+# emits the grub.cfg module2 line IN.
+module = "image:chain"
```

Every changed line is a `module =` line or its adjacent comment. No key other than `[dist] module` changed; `format`, `append`, netdev tables, hostfwd pairs and every other line are untouched.

## Queued for stop B (plan 01-22, `step_profiles`)

The five boots are **not claimed here**. They run on the owner's x86_64 KVM host through `step_profiles` in `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh`, each as `nix develop -c cargo xtask boot-test --arch x86-64 --config <c> --accel kvm`:

| Profile | Image | Queued verdict |
|---|---|---|
| q35exec | `image:marker` | stop B, `step_profiles` |
| q35churn | `image:marker` | stop B, `step_profiles` |
| q35shell | `image:chain` | stop B, `step_profiles` |
| q35churnshell | `image:chain` | stop B, `step_profiles` |
| q35auth | `image:substrate` | stop B, `step_profiles` |

`step_cpuauth_ref` still applies for the cpu/auth compare runs (01-16 recorded authdomain-gen as Linux-only); the marker/chain/substrate compares are already proven on the Mac by this plan.

## Decisions Made

- **Reference gate before edits (D-29).** The three compare-reference runs were executed first and re-run after the edits; both runs printed THREE-STILL-MATCH with 30/49/27 members. The five profiles were only then re-pointed.
- **Comment scope.** Each `[dist]` comment now states the image is r9-assembled, names the manifest, and says the member list matches the reference; the old Harvey path is gone from every comment (the plan's "leave no copy" rule).
- **No other config key touched.** The patch is exactly the module line plus its adjacent comment per file.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **The image `.paq` sha256 differs between two independent assemblies while the member lists match.** `marker.paq` was `4acdc42b…` before the edits and `de652576…` on the post-edit re-run (chain/substrate likewise); `xtask/src/image.rs:248` documents that `mkpaqfs` embeds a time field, so the image bytes are per-bake. D-29's comparison is the member list, which matched in both runs (30/49/27). No drift; byte-differential comparisons remain deferred as planned.
- **`requirements.mark-complete` was not run.** `requirements.ready-ids` reports HARN-06 `blocked` — sibling plans that also declare it (01-30 and others in this wave) have no SUMMARY yet. `REQUIREMENTS.md` is a shared file the orchestrator updates after the wave merges; it was not modified here.

## Known Stubs

None. No placeholder config value remains; every `[dist] module` names a real, reference-verified r9 image.

## Threat Flags

None beyond the plan's threat model. T-01-93 is implemented by running `--compare-reference` for marker, chain and substrate in this plan's own run before any config edit; T-01-SC by no package installs.

## User Setup Required

None — no external service configuration. The KVM boots are the owner's stop-B action, tracked by plan 01-22.

## Next Phase Readiness

- **HARN-06's x86_64 profile move is complete on the Mac.** q35cpu (01-16) plus the five profiles here leave no `harvey:` module in any x86_64 config.
- **Plan 01-22's `step_profiles` now boots the r9-assembled images for all five.** The profiles assemble their images on demand through `resolve_boot_module` → `ensure_named_image`; the images are member-list-equal to the reference bake.
- **Harvey remains untouched** and the kernel baseline is intact (`50b01c9a…`).

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- The five modified configs exist on disk — all FOUND.
- Task commit `61387b8` is an ancestor of HEAD; `git rev-list --count f454f05..HEAD` = 1 (measured against the plan ledger).
- Task 1 verification re-run: THREE-STILL-MATCH (marker 30 / chain 49 / substrate 27 members); FIVE-ON-IMAGE; `module = "harvey:` count 0; FIVE-BUILD.
- Kernel ELF sha256 after a q35churnshell build `50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f` = the 01-08 baseline.
- `cargo xtask test/clippy/check` exit 0/0/0 (87 tests); Harvey HEAD `25d97dc4d…`, status hash `041d2929…` (untouched); zero `port/src` changes.
- `STATE.md`/`ROADMAP.md` were not modified (worktree mode; the orchestrator owns them).
- `REQUIREMENTS.md` was not modified: HARN-06 is blocked by sibling plans without SUMMARYs; the orchestrator owns shared-file updates.
