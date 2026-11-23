---
phase: 01-stage-p0-cross-build-harness
reviewed: 2026-11-23T22:01:36Z
depth: standard
files_reviewed: 95
files_reviewed_list:
  - .github/workflows/gates.yml
  - .github/workflows/xtask.yml
  - aarch64/lib/config_virtip.toml
  - aarch64/lib/config_virtnet.toml
  - flake.lock
  - flake.nix
  - gates/aarch64/cpu-r9.expect
  - gates/aarch64/drive-r9.expect
  - gates/aarch64/smoke-r9-net.expect
  - gates/aarch64/smoke-r9.expect
  - gates/aarch64/tier1-r9.expect
  - gates/x86_64/churn-r9.expect
  - gates/x86_64/cpu-bringup.expect
  - gates/x86_64/drawterm-gate.expect
  - gates/x86_64/netsoak-r9.expect
  - gates/x86_64/services-r9.expect
  - gates/x86_64/smoke-r9-auth.expect
  - gates/x86_64/smoke-r9-net.expect
  - gates/x86_64/smoke-r9.expect
  - gates/x86_64/stress-r9.expect
  - gates/x86_64/tier1-r9.expect
  - nix/gates/boot-r9-x64.sh
  - nix/gates/boot-r9.sh
  - nix/gates/churn-r9-x64.sh
  - nix/gates/cpu-r9.sh
  - nix/gates/drawterm-gate-x64.sh
  - nix/gates/drawterm-x64.sh
  - nix/gates/host-echo.c
  - nix/gates/net-doctor.sh
  - nix/gates/services-r9-x64.sh
  - nix/gates/smoke-r9-auth-x64.sh
  - nix/gates/smoke-r9-net-x64.sh
  - nix/gates/smoke-r9-net.sh
  - nix/gates/smoke-r9-x64.sh
  - nix/gates/smoke-r9.sh
  - nix/gates/soak-r9-net-x64.sh
  - nix/gates/stress-r9-net-x64.sh
  - nix/gates/tier1-r9-x64.sh
  - nix/gates/tier1-r9.sh
  - nix/harvey/authdomain-gen.nix
  - nix/harvey/build-env.nix
  - nix/harvey/mkpaqfs.nix
  - nix/harvey/reference.nix
  - nix/harvey/tree-amd64.nix
  - nix/harvey/tree-arm64.nix
  - x86_64/lib/config_q35auth.toml
  - x86_64/lib/config_q35churn.toml
  - x86_64/lib/config_q35churnshell.toml
  - x86_64/lib/config_q35cpu.toml
  - x86_64/lib/config_q35exec.toml
  - x86_64/lib/config_q35shell.toml
  - xtask/manifests/amd64/auth.toml
  - xtask/manifests/amd64/chain.toml
  - xtask/manifests/amd64/churn.toml
  - xtask/manifests/amd64/cpu.toml
  - xtask/manifests/amd64/fonts-bit.members
  - xtask/manifests/amd64/libraries.toml
  - xtask/manifests/amd64/marker.toml
  - xtask/manifests/amd64/net.toml
  - xtask/manifests/amd64/services.toml
  - xtask/manifests/amd64/substrate.toml
  - xtask/manifests/amd64/tier.toml
  - xtask/manifests/arm64/bootfs.toml
  - xtask/manifests/arm64/libraries.toml
  - xtask/src/config.rs
  - xtask/src/gate.rs
  - xtask/src/harvey.rs
  - xtask/src/image.rs
  - xtask/src/lib.rs
  - xtask/src/main.rs
  - xtask/src/pcap.rs
  - xtask/src/uregcheck.rs
  - x86_64/src/platform/q35/timer.rs
  - x86_64/src/syscall.rs
  - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
  - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/flake.nix
  - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/rio-gate.expect
  - .planning/phases/01-stage-p0-cross-build-harness/rio-derisk/rio-gate.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/venue.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/determinism.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/tier.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/boot-q35.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/aout.py
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/build-probe.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/linux-x86_64.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-amd64.ld
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-amd64.rs
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-arm64.ld
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-arm64.rs
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/q35shell-probe.expect
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/run-arm64-hvf.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/virt-probe.expect
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/make-bundle.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/run-all.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/run-from-repo.sh
findings:
  critical: 1
  warning: 12
  info: 13
  total: 26
  necessity: 0
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-11-23T22:01:36Z
**Depth:** standard
**Files Reviewed:** 96
**Status:** issues_found

## Summary

Scope was the phase's own source: the `xtask` resolver / image engine / gate /
probe code, the x86_64 `sleep`/`alarm` tick conversion and the
pid-tagged-unimplemented change, the 17 `nix/gates` scripts and the moved
`expect` drivers, the six `nix/harvey` derivations, the two flake files and the
CI workflows, the image manifests/configs, and the witness driver plus the
rio-derisk/linux-witness/spike sources named in the phase SUMMARYs. Generated
store paths, `evidence/` logs, `spikes/*/work`, `findings` documents and other
legacy evidence were excluded as requested; the exploratory `s1`/`s2`/`s3`/`s4`
scaffolding not listed in `files_reviewed_list` was sampled only through its
production successors.

The headline fixes are sound: the q35 `ms_to_ticks` ceiling conversion is
correct for the 100 Hz slice (the `TICK_MS` compile-time pin is a good
guard), the `(pid <name>)` suffix preserves the negative-scan prefix match,
the per-architecture resolver's precedence/stamp/realise logic is coherent,
and the engine's two-way drift, provenance and path-containment checks are real
enforcement rather than documentation.

The material risks are in the unvalidated **image name** (a destructive path
escape out of `target/`), a quick-gate HARN-02 proof that is skipped on early
error paths, credential-file cleanup/permission paths that are not uniform, a
self-matching identity sweep in the drawterm driver, and several
environment/evidence-integrity gaps in the witness and app scripts. Findings
below are advisory; no source files were modified.

## Critical Issues

### CR-01: Unvalidated image name escapes the staging root (path traversal / directory deletion)

**File:** `xtask/src/image.rs:1918` (also `:766`, `:1688`, `:1925`, `:1936`; `xtask/src/main.rs:3976`; `xtask/src/config.rs:263`)
**Issue:** `Manifest.image` is used to build filesystem paths but is never validated. `validate()` (`xtask/src/image.rs:289-374`) checks `arch`, member paths, modes, generators and duplicates — not `image`. A manifest with `image = "../../../../tmp/x"` (or an `--image`/`image:` name with `/` or `..`) makes
`out_dir.join(format!("{}.stage", manifest.image))` resolve outside
`target/images/<arch>/`, and line 766 then calls `fs::remove_dir_all(stage_dir)` on it. The same unvalidated value names the `.paq`, `.members`, `.creds`, `.authdom` and `.authdom-verify` outputs, and flows into `image_manifest_path()` / `image_paq()` for the `image:` config and `--image` surface. This defeats the engine's own HARN-05/T-01-27 claim that "a manifest cannot escape the staging root", and it is a data-loss risk (the first thing a malicious/typo'd name does is recursively delete a directory).
The input is normally repo- or operator-controlled, so exploitability depends on trusting the manifest; the deletion primitive is still unsafe.
**Fix:** require `image` to be one `Component::Normal` with no `/`, `.` or `..`, and require the manifest's `image` to equal the requested name before any path is joined. Validate `--image` and the `image:` scheme the same way, and add a test with `../../escape` expecting a named error rather than a filesystem touch.
```rust
fn validate_image_name(name: &str) -> Result<()> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) if name != "." && name != ".." => Ok(()),
        _ => Err(format!("image name {name:?} is not a single normal path component").into()),
    }
}
```

## Warnings

### WR-01: `ureg-check` exits 0 when one architecture's tree is unresolvable

**File:** `xtask/src/uregcheck.rs:142`
**Issue:** the match arm `(CheckStatus::Match, _) | (_, CheckStatus::Match) => Ok(CheckStatus::Match)` runs before the `errors` arm. If one per-arch variable is unset (for example only `R9_HARVEY_DIR_ARM64` is exported) and the other arm resolves and matches, `cargo xtask ureg-check` prints the resolver error to stderr and still returns `Match` (exit 0). A half-configured environment therefore passes the cross-repo drift gate for one architecture.
**Fix:** return the collected error whenever `!errors.is_empty()` (keeping the historical "headerless tree SKIP" as the only tolerated non-match), or require both arms to be `Match` before returning success.

### WR-02: `cargo xtask gate --full` silently ignores `--accel`

**File:** `xtask/src/gate.rs:537` (and the `accel` resolution at `:306`)
**Issue:** `--accel`/`R9_ACCEL` is resolved and passed only in `run_quick`. `run_full` spawns `nix run .#<app>` with no `R9_ACCEL` in the child environment, so `cargo xtask gate --full --accel tcg` runs the apps with their KVM default and fails on a KVM-less host; the witness works around this by exporting `R9_ACCEL=kvm` around the gate (`.planning/.../witness/run-witness.sh:697`). A documented flag that does nothing for half the command's modes is a trap.
**Fix:** resolve accel once in `GateStep::run` and set `.env("R9_ACCEL", accel)` on every `nix run` in `run_full`; optionally apply `quick_venue` to the full tier as well.

### WR-03: Quick gate's tree-unmodified proof is skipped on every early error path

**File:** `xtask/src/gate.rs:345-426`
**Issue:** the module doc says step 5 proves the Harvey tree unmodified "on pass AND fail", but every `?` before step 5 (`harvey::harvey_dir`, `assemble_named_image`, `BuildStep`, `build_x86_64_iso`, `read_creds`, `write_private`) returns before the `store_verify`/`tree_digest` comparison. The image engine's authdomain generator runs with `current_dir(tree)`, and a failed build can have written into an override (non-store) tree; on exactly those failures the HARN-02 witness is not evaluated.
**Fix:** accumulate the outcome of steps 1–4 into a value (or use a scope guard) so the store-verify/digest step runs on every exit, then return the first failure after recording the tree verdict.

### WR-04: Gate password file mode is only applied at creation

**File:** `xtask/src/gate.rs:630-641`
**Issue:** `OpenOptionsExt::mode(0o600)` only takes effect when `dt-password` is created. If the path already exists (the gate directory is `<pid>-<seconds>` under `target/gate`, predictable enough for a local pre-creation), `.truncate(true)` preserves the old permissions and the secret is written into a file that may be 0644/0666. The shell wrappers explicitly do `umask 077` **and** `chmod 600`; this Rust path does neither after the write.
**Fix:** after writing, `fs::set_permissions(path, fs::Permissions::from_mode(0o600))`; ideally unlink/create-new or use a `create_new` temp file plus rename.

### WR-05: Per-bake authdomain (with the DEMO password) is left under `target/` on error paths

**File:** `xtask/src/image.rs:1917-1930` (also `:1729-1737`, `:1809`)
**Issue:** the generated domain directory is removed only after pairing, creds-copy and the successful path at line 1930. If `parse_creds` fails (`:1737`), staging/drift/provenance fails, pairing refuses, or `mkpaqfs` fails, `target/images/<arch>/<image>.authdom/auth-creds` (the per-bake password) remains on disk in a directory created with the default mode. That contradicts the file header's "only the staged copies … and the 0600 creds file remain under target/".
**Fix:** wrap the domain directory in a scope guard (or explicit cleanup on every `Err` return) so `remove_dir_all(domain_dir)` runs on failure too; create the directory with mode 0700.

### WR-06: Drawterm driver identity sweep can match and kill its own helper

**File:** `gates/x86_64/drawterm-gate.expect:150-172`
**Issue:** `sweep_identity` runs `exec bash -c {…} sweep "$run_tag" "$gate_dir" [pid]`. The helper's own `/proc/<pid>/cmdline` contains the literal `drawterm` (inside the `pgrep -f 'qemu-system|Xvfb|drawterm'` pattern) and the run tag/gate dir are its positional arguments; its inherited `environ` also carries `R9_RUN_TAG`. `excl` only excludes the expect pid (`$self`) and its ancestors, not the helper's own pid (`$$`), so the helper matches itself and sends itself SIGTERM. Today it usually survives because pgrep returns PIDs in ascending order and the helper is the newest match, but the sweep's guarantee then depends on PID ordering rather than on the exclusion list; a survivor with a higher PID can outlive the sweep.
**Fix:** seed `excl` with `$$` as well (`excl=" $$ "`) and/or pass the pattern/tag via environment so the helper's argv cannot match; add a static test that the helper excludes `$$`.

### WR-07: `eval` of the netaddr seam in the doctor and both tap gates

**File:** `nix/gates/net-doctor.sh:266`; `nix/gates/soak-r9-net-x64.sh:292`; `nix/gates/stress-r9-net-x64.sh:269`
**Issue:** all three scripts do `eval "$(grep -E '^(tap_gwv4|blastport)=' "$SEAM_PATH" || true)"`. The seam is committed repo data in the intended flow, but `HARVEY`/`HARVEY_NETADDR_SEAM` are environment-selectable and the soak/stress scripts accept a writable scratch tree. A seam line such as `tap_gwv4=10.0.2.1; <command>` executes arbitrary shell as the owner in the doctor and in both gates. The doctor's "read-only" contract is only about its own commands, not about the seam it evals.
**Fix:** parse the two keys without `eval` (`sed -n 's/^tap_gwv4=//p'`), then validate `tap_gwv4` as a dotted-quad and `blastport` as 1–65535 before use.

### WR-08: `host-echo` accepts out-of-range ports (unsigned-short wrap)

**File:** `nix/gates/host-echo.c:126-130`
**Issue:** `port = (unsigned short)strtoul(argv[2], &end, 10);` is range-checked only against 0 afterwards. `70000` truncates to `4464` and `-1` becomes `65535`, both accepted and bound, despite the "want 1-65535" error text. A typo in the seam's `blastport` would make the peer listen on a different port while the guest dials the intended one, turning a venue error into a confusing soak failure.
**Fix:** parse into `unsigned long`, check `errno == 0`, `*end == '\0'`, and `1 <= value && value <= 65535`, then cast.

### WR-09: Gate scripts and the image engine can resolve different Harvey trees

**File:** `nix/gates/boot-r9-x64.sh:129` (same shape in the other 14 gate scripts, e.g. `nix/gates/drawterm-gate-x64.sh:68`, `nix/gates/smoke-r9-net-x64.sh:102`); resolver precedence `xtask/src/harvey.rs:178-180`
**Issue:** every app script reads only `R9_HARVEY_DIR_AMD64`/`_ARM64` into `$HARVEY`, checks that tree's `HARVEY-TREE` stamp, and uses it for the seam/doctor/fixture reads. The `cargo xtask image` child, however, resolves `R9_HARVEY_DIR` **first** (documented manual override). With both variables set, the image is assembled from the override tree while the app validates and reads the per-arch tree — the stamp check passes on a tree the engine is not using, and the seam/doctor see different files than the bake. The quick gate does not have this split because it calls the resolver directly.
**Fix:** resolve once at the top of each script with the resolver's precedence (`HARVEY="${R9_HARVEY_DIR:-${R9_HARVEY_DIR_AMD64:-}}"`, plus an appropriate stamp/checkout check), or refuse to run when `R9_HARVEY_DIR` and the exported per-arch var name different paths.

### WR-10: Witness reuses the Harvey scratch tree and equivalence baselines without pin provenance

**File:** `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh:259-288`, `:803-817`
**Issue:** `ensure_harvey_scratch` reuses any `$WORK/harvey` that has `sys/src` and no `.git`; it never records or checks `$HARVEY_PIN`. After a pin bump (this phase already moved the flake pin to `c697666`; the driver's fetch pin is `bd7a698`), the rio/control steps silently run the old tree. `ensure_equiv_baseline` similarly reuses any baseline directory containing a `*.log`, so changed marker expectations or a new pin keep the old fresh baseline.
**Fix:** write `$WORK/harvey/.r9-harvey-pin` and `$dir/.r9-pin` when creating each, compare on reuse, and `rm -rf`/re-prefetch on mismatch; record the pin in `out/env.txt` per step.

### WR-11: Full tier includes an interactive qemu boot with no timeout or driver

**File:** `xtask/src/gate.rs:53-62`, `:537`; `nix/gates/boot-r9-x64.sh:356-380`
**Issue:** `FULL_APPS_X86_64` includes `boot-r9-x64`, whose app boots qemu with `-nographic` on the terminal and waits for the user to quit (`ctrl-a x to quit`). `run_full` runs `nix run .#boot-r9-x64` via `.status()` with no timeout and no serial driver, so an unattended full run blocks indefinitely; the stop-C2 capture ends with `% QEMU: Terminated`, i.e. the run was ended by a human/terminal. This makes the D-14 full tier non-reproducible unattended and is a CI hang risk if it is ever wired.
**Fix:** drive a `quit`/`ctrl-a x` through a serial expect after the rc prompt, or exclude the terminal app from the automated full list (recording it as an explicitly interactive leg), or run it under a bounded `timeout` with a named `INTERACTIVE` verdict.

### WR-12: Credential scans delete leaks but never fail the witness

**File:** `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh:947-993`, `:1048-1058`; `spikes/linux-witness/run-all.sh:305-352`, `:372-382`
**Issue:** `finish` sets `found=1` when it deletes an `auth-creds`-named artifact or any file containing the DEMO password, prints a `REMINDER`, and returns. The run's exit status is computed only from `summary.tsv`, so a redaction failure is a green witness whose evidence has just been deleted. A credential leak in the retained evidence should fail the run, not be silently remediated.
**Fix:** append a synthetic `secret-scan\t1\t0` row (or an explicit `FAIL`) when `found=1`, and include the deletion list in `WITNESS-RETURN.md` so the return cannot be read as clean.

## Info

### IN-01: `tree_digest` is a non-cryptographic hash and omits modes and symlink targets

**File:** `xtask/src/gate.rs:195-219`
**Issue:** the override-tree HARN-02 witness is FNV-1a 64 over relative paths plus file bytes. It ignores permission/ownership changes and a symlink's target path (it hashes the target's bytes for a file link and treats a directory/dangling link as `d`), and 64-bit FNV is not collision-resistant. That is adequate for accidental drift, not for an integrity claim.
**Fix:** use SHA-256 (or `sha256sum` over a canonical listing) and include the mode bits and `readlink` target in the digest; say explicitly in the error text that it is an accidental-change witness.

### IN-02: `[[tree]]` members-file modes are not bounded like manifest modes

**File:** `xtask/src/image.rs:706-714` vs `:654-663`
**Issue:** `parse_mode` rejects values above `0o7777`, but `parse_tree_members` accepts any octal `u32` from the members file and passes it to `set_permissions`. The shipped `fonts-bit.members` is all 644, but the parser asymmetry weakens the "malformed input is a named error" rule.
**Fix:** apply the same `<= 0o7777` bound (and reject empty values) in `parse_tree_members`.

### IN-03: Generated `ndb-template` inputs bypass the symlink containment checks

**File:** `xtask/src/image.rs:881-897`
**Issue:** `harvey`, `rust` and `[[tree]]` sources are canonicalized and required to stay inside the workspace/tree, but the `ndb-template` template (`tree.join(from)`) and seam (`tree.join(seam)`) are read directly. A symlink in a writable override tree can make the bake read an arbitrary host file into the image, while the same path would be rejected as a `harvey` source.
**Fix:** canonicalize both paths and assert `starts_with(canonical_tree)` before `read_to_string`, matching the `harvey` branch.

### IN-04: `Dist.module` documentation still claims only the `harvey:` scheme

**File:** `xtask/src/config.rs:318-323`
**Issue:** the doc comment says a `harvey:` prefix resolves under `R9_HARVEY_DIR` and does not mention the new `image:` scheme implemented by `resolve_boot_module` (`xtask/src/main.rs:4676`). The configs now use `image:` exclusively, so the documentation points readers at the retired channel.
**Fix:** document `image:<name>` (assembled on demand into `target/images/<arch>/`), `harvey:<path>` and ordinary workspace-relative paths.

### IN-05: `run_full` checks one flake system but runs another

**File:** `xtask/src/gate.rs:512-518` and `:537-542`
**Issue:** the pre-check evaluates `apps.<flake_system(arch)>.<app>.program`, but `nix run .#<app>` resolves the **host** system's app. On a host whose system differs from the requested arch (e.g. Darwin with `--arch x86_64`), the pre-check passes and the run dies with a misleading "app not found"; on an x86_64-linux host with `--arch aarch64` the pre-check and the run are different attributes. There is also no `quick_venue`/Linux check on the full tier.
**Fix:** run `nix run .#apps.<flake_system(arch)>.<app>` (or `--system`-select explicitly), and call `quick_venue`/`flake_system` before the loop so a wrong-venue full run is a `GATE-VENUE` error, not a per-app FAIL.

### IN-06: Unused `control_leg` variable in the drawterm driver

**File:** `gates/x86_64/drawterm-gate.expect:84`
**Issue:** `control_leg` is computed (`control-noG || force-no-rio`) and never referenced; only `rio_leg` and `x11_leg` drive the branches. It is dead state in a 777-line gate driver and suggests a missing control-mode branch.
**Fix:** remove it or use it in the branch predicates if the intent was to distinguish the control assertion.

### IN-07: `drawterm_bin()` does not validate `DRAWTERM_BIN` as an executable file

**File:** `xtask/src/gate.rs:584-594`
**Issue:** an explicit `DRAWTERM_BIN` is returned unchecked, unlike the `-f`/`-x` contract the shell wrappers apply. A directory or non-executable path becomes a spawn failure at the first leg rather than a named contract error before any build/boot spend.
**Fix:** check `is_file()` and the execute bit (or defer to `command_in_path`'s predicate) and return `GATE-VENUE drawterm-missing` with the path.

### IN-08: Several `kvm` selections only check existence of `/dev/kvm`

**File:** `nix/gates/boot-r9-x64.sh:175`, `smoke-r9-x64.sh:156`, `tier1-r9-x64.sh:126`, `churn-r9-x64.sh:118`, `smoke-r9-net-x64.sh:161`, `smoke-r9-auth-x64.sh:157`, `services-r9-x64.sh:131`, `soak-r9-net-x64.sh:219`, `stress-r9-net-x64.sh:201`, `.planning/phases/01-stage-p0-cross-build-harness/rio-derisk/rio-gate.sh:307`
**Issue:** these use `[ -e /dev/kvm ]` while `drawterm-gate-x64.sh:122` and `gate.rs:kvm_writable()` correctly test writability. On a host where `/dev/kvm` exists but is not writable (missing group/ACL), the script proceeds and qemu fails with a lower-level error instead of the intended "kvm-unavailable" named refusal.
**Fix:** use `[ -w /dev/kvm ]` consistently (or open it read/write) and name the permission case in the error.

### IN-09: S6 spike teardown patterns never match, and one final check is host-wide

**File:** `.planning/.../spikes/s6/run-arm64-hvf.sh:68-75`, `:222-226`; `spikes/s6/linux-x86_64.sh:87-94`
**Issue:** `cleanup_qemu` matches `qemu-system-*.*$scratch` / `qemu-system-x86_64.*$S6_R9`, but the qemu argv contains neither the scratch path nor the r9 clone path (the `R9_HARVEY_DIR`/cwd are not on qemu's command line), so the trap is a no-op and only the expect driver actually reaps qemu. The final `pgrep -f qemu-system-aarch64` in `run-arm64-hvf.sh` matches *any* aarch64 qemu on the host and fails the spike even when this run cleaned up.
**Fix:** key the teardown on a per-run `-name` tag exported to the driver (as the production gates do), and make the final check match only that tag.

### IN-10: `boot-q35.sh` only scans for failure signatures after the banner

**File:** `.planning/.../spikes/s5/boot-q35.sh:156-197`
**Issue:** the `SIGS` scan lives inside `if [ "$banner" = yes ]`. A kernel that dies before printing `Plan 9` (bad magic, early panic, trap) is correctly a FAIL but reports `SIGS=none`, losing the evidence that names the failure class.
**Fix:** run the signature scan unconditionally over the whole capture (the final scan at 186-196 already runs whole-file; move it outside the banner branch).

### IN-11: `--serial-check` markers are injected as Tcl regular expressions

**File:** `gates/x86_64/drawterm-gate.expect:724-725`; `xtask/src/gate.rs:474`
**Issue:** `GATE_SERIAL_CHECKS` values are split on `=` and the marker is passed to `expect -re $sc_marker`. A marker containing regex metacharacters changes the assertion: `.*` passes trivially, `[` aborts the driver, and a marker with `(`/`)` may match the wrong text. The CLI documents `<cmd>=<marker>` as a literal marker.
**Fix:** quote the marker for expect (e.g. `-re [string map {...}]` / use `-ex` or an escaped literal) or validate that the marker contains only `[A-Za-z0-9 ._:/-]`.

### IN-12: `run_full` silently ignores `--force-no-rio`/`--serial-check`, and capture freshness is not checked

**File:** `xtask/src/gate.rs:290-312`, `:493-559`; `.planning/.../witness/run-witness.sh:685-731`
**Issue:** `GateStep::run` parses `--force-no-rio` and `--serial-check` then dispatches `run_full`, which never reads either field; the flags are accepted and dropped. In `step_full`, the capture copy loop only checks that `$R9/target/gates/<dir>` exists, not that this run wrote it, so a stale directory from an earlier run can be copied (the gate's own `full.tsv` still governs the verdict, but the evidence tree can contain stale captures).
**Fix:** reject the flags with `--full` (or implement them), and have `step_full` remove/record the pre-run capture dirs or compare mtimes against the `start.stamp` (the `run_dir` lookup already does this).

### IN-13: `aout.py` computes an unused `declared` length

**File:** `.planning/.../spikes/s6/aout.py:99`
**Issue:** `declared = HDR_SIZE + text + data + syms` is never used; the following check recomputes `HDR_SIZE + text + data`. Dead local in the probe's a.out checker (harmless, but it obscures whether `syms` was meant to be part of the length check).
**Fix:** delete it or use it in the length check with an explicit "syms must be 0" ordering.

---

_Reviewed: 2026-11-23T22:01:36Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
