# Phase 1: Stage P0 Cross-Build Harness (pass 2) - Pattern Map

**Mapped:** 2026-11-22
**Files analyzed:** 22 new/modified (groups noted where one pattern covers many files)
**Analogs found:** 21 / 22 (one partial)

All analog paths below were checked as git-tracked source. Harvey paths are in the READ-ONLY sibling checkout `/Users/neilskinner/code/harvey` (originals to copy; never edit there in pass 2 before the D-10 owner-approved commit). Spike scratch under `.planning/phases/01-stage-p0-cross-build-harness/spikes/` is tracked and is the recipe source for the derivations. Gitignored `work/` dirs and `.gsd` mirrors are NOT analogs.

Abbreviation: `SP` = `/Users/neilskinner/code/r9/.planning/phases/01-stage-p0-cross-build-harness/spikes`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `xtask/src/harvey.rs` (new, resolver) | utility | request-response (env to path) | `xtask/src/uregcheck.rs::harvey_dir` (L70-85) | exact (it is the thing being generalised) |
| `xtask/src/lib.rs` (modify: `pub mod harvey;`) | config | n/a | `xtask/src/lib.rs` L19-20 | exact |
| `xtask/src/uregcheck.rs` (modify: `harvey_dir` delegates, `run_check`) | utility | file-I/O | itself L76-130 | exact |
| `xtask/src/config.rs` (modify `Initrd::resolved_file`, `harvey:` scheme, L247-259, L750) | model | transform | itself | exact |
| `xtask/src/main.rs` (modify L3729, L3972, L4317-4345 `harvey:`, new `gate` subcommand L174-250, step structs) | controller | request-response/batch | `UregCheckStep` L4349-4359 + clap block L174-199 | exact |
| `xtask/src/pcap.rs` (modify `netaddr_*` L125-146) | utility | file-I/O | itself L125-146 | exact |
| `xtask/src/image.rs` (new: manifest parse + assembler + drift check) | service | batch / file-I/O | `main.rs::assemble_initrd` L3720-3800 + `config.rs` toml/serde use | role-match |
| `xtask/manifests/{amd64,arm64}.toml` (new) | config | n/a | `x86_64/lib/config_q35shell.toml` / `xtask/doc/config_example.toml` (serde+toml) | role-match |
| `xtask/src/main.rs` gate step (`GateStep`) | controller | event-driven (spawn qemu, read serial) | `BootTestStep` (main.rs ~L1029-1400) | role-match |
| `flake.nix` `inputs.harvey` | config | n/a | `flake.nix` L4-6 inputs; Harvey `flake.nix` L5 `inputs.r9.url` (mirror image) | exact |
| `flake.nix` `harvey-hostbin`, `harvey-tree-{amd64,arm64}`, `mkpaqfs` | service (derivation) | batch / file-I/O | `SP/s4/venue.nix`, `SP/s4/hostbin.nix`, `SP/s4/tier.nix` | exact (spike-proven recipe) |
| `flake.nix` devshell exports | config | n/a | `flake.nix` L854-875 shellHook + L28-33 `objcopyHook` | exact |
| `flake.nix` composed apps (moved from Harvey) | config | request-response | Harvey `flake.nix` L26-39 `mkScriptApp`, L188-227 apps; r9 `flake.nix` L881-908 apps | exact |
| `nix/*.sh` gate scripts moved to r9 (about 20) | utility | batch/event-driven | Harvey `nix/drawterm-gate-x64.sh` and siblings | exact (copy, rewire) |
| `build/{x86_64,aarch64}/*.expect` moved (15) | test | event-driven | Harvey `build/x86_64/drawterm-gate.expect`, `SP/s3/rio-gate.expect` | exact (copy) |
| rio leg in drawterm gate (D-11/D-28) | test | event-driven | `SP/s3/rio-gate.sh` + `rio-gate.expect` | role-match |
| `cargo xtask gate` quick/full tiers | controller | batch | `main.rs` boot-test dispatch L235-241 | role-match |
| `xtask` tests (manifest drift, resolver) | test | transform | `xtask/src/uregcheck.rs` L392-510 `#[cfg(test)]` | exact |
| `.github/workflows/xtask.yml` | config | n/a | itself, `boot-test-x86-64` job | exact |
| temp Rust a.out probe (`ps` swap) | utility | transform | `SP/s6/probe-amd64.rs`, `probe-amd64.ld`, `aout.py`, `build-probe.sh` | exact (spike-proven; then deleted, D-18) |
| Harvey own-kernel q35 app (D-26) | config/app | event-driven | `SP/s5/boot-q35.sh` | exact |
| Harvey-side commit (flake.nix, deletions) | config | n/a | n/a (owner-approved, last, D-10) | no analog needed |

## Pattern Assignments

### `xtask/src/harvey.rs` (utility, request-response) and its consumers

**Analog:** `xtask/src/uregcheck.rs` L70-85, the single reader today.

```rust
pub fn harvey_dir() -> Result<PathBuf> {
    match env::var("R9_HARVEY_DIR") {
        Ok(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => Err("R9_HARVEY_DIR is not set — point it at the Harvey checkout ... r9's xtask no longer defaults \
             to a sibling ../harvey checkout (05-CONTEXT D-10)".into()),
    }
}
```

Imports convention (uregcheck.rs L29-32): `use std::path::PathBuf; use std::{env, fs}; use crate::{Arch, DynError, Result};` (`Arch` from lib.rs, `Result<T> = Result<T, Box<dyn Error>>`).

New resolver shape (D-01, D-21): `harvey_dir(arch: Arch)` reads `R9_HARVEY_DIR` first (override wins, non-empty), else `R9_HARVEY_DIR_AMD64` / `R9_HARVEY_DIR_ARM64` by arch, else the same named error, no built-in default path. Keep empty-is-unset. Keep the existing `pub fn harvey_dir()` as a thin wrapper or change all seven callers at once (they are listed below), so there is exactly one env reader. Because config.rs lives in the lib and pcap.rs reaches it as `xtask::uregcheck::harvey_dir()`, put the resolver in the lib (`pub mod harvey;` beside `pub mod uregcheck;` at lib.rs L19-20) so the `r9-netdev-args` bin keeps compiling.

Callers to route (grep verified):
- `xtask/src/config.rs:250` `Initrd::resolved_file` (`harvey:` prefix: `dir.join(rest)`); also L753 `starts_with("harvey:")`.
- `xtask/src/main.rs:3730` (`assemble_initrd`), `:3972` (staleness check), `:4326` (`resolve_boot_module`, `harvey:` dist module), and `uregcheck.rs:94` (`run_check`).
- `xtask/src/pcap.rs:126` `netaddr_slirp` (+ `netaddr_slirp_v6`, `netaddr_blastport`, same shape):

```rust
let path = xtask::uregcheck::harvey_dir()
    .map_err(|e| format!("{e}"))?
    .join("build/aarch64/netaddr.rc");
```

Arch selection note: `netaddr.rc` and `ureg.h` (both arches) are read from one tree. The resolver must pick which per-arch tree for a given file, or the arm64 tree must carry both arch dirs; decide in the plan and keep ureg-check reading `arm64/include` and `amd64/include` through the matching per-arch tree (or the single override).

Error-message pattern to keep: name the variable, name what the tree must contain, state no default (`uregcheck.rs` L79-82). HARN-02 "after-gate unmodified proof": add a check (hash or `nix store verify`-style, per `SP/S1-FINDINGS.md`) that the tree is unchanged after the gate.

Tests: copy the inline `#[cfg(test)] mod tests` style (`uregcheck.rs` L392+, `#[test]` fns with `assert_eq!` on `Result`). For env-reading tests, pass the lookup in as a closure/argument (resolver `fn resolve(get: impl Fn(&str)->Option<String>, arch)`) so tests do not mutate process env.

---

### `xtask/src/main.rs` `gate` subcommand (controller, batch)

**Analog:** clap block L174-199 and dispatch L217-249; step struct `UregCheckStep` L4349-4359.

clap registration:
```rust
.subcommand(
    clap::Command::new("boot-test")
        .about("Boot r9 under QEMU ... (D-11)")
        .args(&[
            clap::arg!(--arch <arch> "Target architecture")
                .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
            clap::arg!(--config <name> "Configuration")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .default_value("default"),
            clap::arg!(--verbose "Print commands"),
            clap::arg!(--accel <accel> "Accelerator override (tcg/kvm/hvf); beats R9_ACCEL env and [qemu] accel (D-06)")
                .value_parser(clap::builder::NonEmptyStringValueParser::new()),
        ]),
)
```
Dispatch (L235-241): build then test in the same invocation so evidence is never stale:
```rust
Some(("boot-test", m)) => {
    let s1 = BuildStep::new(m);
    let s2 = BootTestStep::new(m);
    s1.run().and_then(|_| s2.run())
}
Some(("ureg-check", _)) => UregCheckStep::new().run(),
```
Errors print via `eprintln!("{e}"); process::exit(1)` (L246-249): a failing gate must return `Err`, never exit 0. Add `Some(("gate", m)) => GateStep::new(m).run()` with `--full` flag (D-14: quick by default, full set at phase boundaries and pin bumps). Accel discipline: reuse flag > `R9_ACCEL` > TOML, no silent downgrade (see Harvey `drawterm-gate-x64.sh` step 0b below). The quick tier calls the per-arch resolver, assembles the manifest image, boots, and asserts rio (D-11/D-28) plus the negative control; the full tier shells out to the moved gate scripts.

The `main.rs` file is 4937 lines. For the new gate/image code prefer new modules (`image.rs`, a `gate.rs`) over extending `main.rs`; only the clap block, dispatch arm, and step struct touch it.

---

### `xtask/src/image.rs` (service, batch/file-I/O) and manifests

**Analog (assembler):** `assemble_initrd` at `xtask/src/main.rs` L3720-3800. Pattern: resolve Harvey dir, locate `mkpaqfs`, stage a tree under `workspace()`, recreate it from scratch, run `mkpaqfs`, loud named errors for each absent producer:

```rust
let harvey = uregcheck::harvey_dir()?;
let mkpaqfs = harvey.join(".build/bin/mkpaqfs");
if !mkpaqfs.is_file() {
    return Err(format!("mkpaqfs host binary missing at {} — build it ... ", mkpaqfs.display()).into());
}
let ws = workspace();
let tree = ws.join(INITRD_TREE);
if tree.exists() { std::fs::remove_dir_all(&tree)?; }
std::fs::create_dir_all(&tree)?;
```
After D-16, `mkpaqfs` comes from the flake `mkpaqfs` derivation (devshell-exported path) and each staged file comes from the Harvey tree or the r9 Rust build per the manifest. `workspace()` (main.rs L4361) is the root-finding helper.

Staleness pattern (main.rs L3971-3985): compare mtimes of inputs against output and rebuild with a printed `xtask: initrd ... missing or stale — assembling` line. Reuse for the assembled image cache.

**Analog (manifest format):** serde+toml. `xtask/Cargo.toml` already has `serde` (derive) and `toml = "0.9"`; configs such as `x86_64/lib/config_q35shell.toml` are parsed in `config.rs`. Use TOML, one table per arch, one entry per image file: `path = "/amd64/bin/ps"`, `source = "harvey" | "rust"`, and for Rust provenance the artifact path (HARN-05). Parse with `#[derive(Deserialize)]` structs like `config.rs`. Location: `xtask/manifests/<arch>.toml` (discretion).

Drift check (D-15, HARN-05): compute the set difference both ways between manifest paths and the staged image member list, and return a named error per direction (`in image but not in manifest: ...` / `in manifest but not in image: ...`). Verification (D-29): compare assembled member list (paths, sizes, hashes) to a reference Harvey bake, no byte differential. Do NOT rely on two-bake determinism (A12 refuted).

Tier-0 library table: source it from the pinned tree files `build/x86_64/tiers/{tier0-libs,tier1-cmds}.txt` (read via the resolver, D-19). Parsing recipe from `SP/s4/tier.nix`: `awk '!/^[ \t]*#/ && NF {print $1}'`; in Rust, skip `#` lines and blank lines and take the first whitespace field.

---

### `flake.nix`: Harvey input, hostbin/tree/mkpaqfs derivations

**Analog:** `SP/s4/hostbin.nix`, `SP/s4/venue.nix`, `SP/s4/tier.nix`, `SP/s1/hostbin.nix`, `SP/s1/venue.nix`, flake wiring `SP/s4/flake.nix`.

Input shape. r9 `flake.nix` L4-6 is the style; Harvey's mirror is `inputs.r9.url = "github:requaos/r9";` (Harvey flake.nix L12). D-09 says a normal input (not `flake = false`; the spikes used `flake = false`):
```nix
inputs.harvey.url = "github:requaos/harvey/bd7a6982a3e4f465f8ed3da5e0c7290521514ff0";  # D-23 pin
inputs.harvey.inputs.nixpkgs.follows = "nixpkgs";   # A13 untested: verify drawterm/harness still build
```
Caution: D-10 order means r9 can only add the input while Harvey still imports r9, so the lock graph must not cycle: r9 locks Harvey `bd7a698` (`legacy-final`) whose own lock pins r9 `116bcc2`. Nix allows this (no cycle in a lock DAG across different revs), but do not add `follows` that tie them. Dev override is `--override-input harvey path:...`, never the default (D-06).

Derivation pattern (copy verbatim, port to r9's `pkgsFor`/`forAllSystems`, `harveySrc = harvey` / `harvey.outPath`):
```nix
# SP/s4/flake.nix
hostbin = import ./hostbin.nix { inherit pkgs; harveySrc = harvey; };
s4-tier-amd64 = import ./tier.nix { inherit pkgs hostbin; harveySrc = harvey; };
```
`venue.nix` core (L47-95): `stdenvNoCC.mkDerivation { dontUnpack = true; nativeBuildInputs = [ bash coreutils util-linux plan9port gnused gawk gnugrep diffutils ] ++ optional (mode == "proot") proot ++ extraPath; buildCommand = ''set -euo pipefail ... cp -r "${harveySrc}/$d" "$work/$d"; chmod -R u+w "$work" ...'' }`. Use `mode = "proot"` (D-25). Keep the comments about `/usr/bin/env` sandbox-evidence check. Carry S1's four host-toolchain adjustments (store-bash shebang for `9l`, `pkgs.ed`, plan9port last on PATH, GNU sed shadow) and S4's two ports (`/rc/bin` on PATH, explicit `INCLUDE`) in the venue, not per step (SPIKE-RESULTS Decision 2). Output only what images and ureg checks consume, assert no output name contains `~nix~case~hack~`, and add `mkpaqfs` as a small separate derivation (its recipe is Harvey `build/aarch64/mkpaqfs-host.sh`; read it from the pinned tree).

Tree derivation outputs must be complete per arch for `R9_HARVEY_DIR_<ARCH>`: `amd64/include/ureg.h` (or arm64), `build/...` fixtures read by `netaddr.rc`/sweeps (D-19, read from the pinned tree), plus built `bin/` and `lib/`. For the Mac the Harvey source is a builder-side fixed-output fetch (D-24, `SP/s2/flake.nix` `s2-fod-lima` is the analog) rather than the Mac-fetched input.

Style rules from existing r9 flake: derivations live inside `packages = forAllSystems (system: let ... in { ... })` (flake.nix L335+); existing derivations use `pkgs.stdenv.mkDerivation` (L376 `kernel-x64`, L407 `netdev-args`, L428 `kernel-virt`). Pin nixpkgs the same as r9 (`SP/s4/flake.nix` used `1c89083e...`, r9's own rev; use `inputs.nixpkgs`).

---

### `flake.nix` devshell exports (config)

**Analog:** `flake.nix` L854-875:
```nix
devShells = forAllSystems (system:
  let pkgs = pkgsFor system; toolchain = toolchainFor system; in
  { default = pkgs.mkShellNoCC {
      packages = [ toolchain pkgs.qemu pkgs.dtc ]
        ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.grub2 pkgs.xorriso ];
      shellHook = ''
        ${objcopyHook toolchain}
        echo "r9 devshell: $(rustc --version)"
      ''; }; });
```
Add to `shellHook`: `export R9_HARVEY_DIR_AMD64="${treeAmd64}"` and `..._ARM64` using the lazy outPath pattern proven by A9 (context-discarded `builtins.unsafeDiscardStringContext ...outPath` of the aarch64-linux derivation on darwin; entering the shell builds nothing; see `SP/s2/flake.nix` s2-lazy-target and `SP/s2/evidence/lazy-devshell.log`). Do not export `R9_HARVEY_DIR` (manual override only, D-21). Mirror the "treat only absolute values as user overrides" caution from `objcopyHook` L28-33: do not clobber an existing manual `R9_HARVEY_DIR`.

---

### `flake.nix` composed apps and moved gate scripts

**Analog (app wrapper):** Harvey `flake.nix` L26-39, copy into r9:
```nix
mkScriptApp = pkgs: name: file: extraHeader: runtimeInputs:
  pkgs.writeShellApplication {
    inherit name runtimeInputs;
    text = ''
      set -euo pipefail
      export PLAN9="${pkgs.plan9port}/plan9"
      export PATH="$PATH:$PLAN9/bin"      # plan9 bin LAST per BUILD_ON_UNIX
      ${extraHeader}
      ${builtins.readFile file}
    '';
  };
```
**Analog (rewire):** Harvey L188-196. After the flip the `r9.packages.${system}.X` references become in-flake `self.packages.${system}.X`:
```nix
bootR9X64Pkg = mkScriptApp pkgs "harvey-boot-r9-x64" ./nix/boot-r9-x64.sh ''
  export R9_KERNEL_X64_PKG="${r9.packages.${system}.kernel-x64}"
  export R9_ISO_BUILDER="${r9.packages.${system}.iso-builder}/bin/r9-iso-builder"
  export R9_NETDEV_EMITTER="${r9.packages.${system}.netdev-args}/bin/r9-netdev-args"
''
```
Becomes `${self.packages.${system}.kernel-x64}` etc. Add `export DRAWTERM_BIN="${harvey.packages.${system}.drawterm}/bin/drawterm"` (Harvey L332 sets it from `self.packages...drawterm`; here use the Harvey input, D-09) and the Harvey tree path (`R9_HARVEY_DIR_AMD64`) replacing the checkout `HARVEY="${HARVEY:-$PWD}"` convention. App registration: r9 flake.nix L898-907 style, `name = { type = "app"; program = "${pkg}/bin/<name>"; }`; also Harvey's `mkApp`. Rename `harvey-*` app names to r9 names consistently (ex: `r9-` prefix) in one pass; flake `apps` entries Harvey exposes (e.g. `drawterm-gate-x64`, L540) keep their attr names so CI/doc commands read the same.

Gate scripts (D-07/D-19): copy `/Users/neilskinner/code/harvey/nix/{boot-r9,boot-r9-x64,smoke-r9,smoke-r9-x64,smoke-r9-net,smoke-r9-net-x64,smoke-r9-auth-x64,tier1-r9,tier1-r9-x64,services-r9-x64,cpu-r9,churn-r9-x64,soak-r9-net-x64,stress-r9-net-x64,drawterm-gate-x64,net-doctor}.sh` and `build/x86_64/*.expect`, `build/aarch64/*.expect` (15 drivers). Harvey-only scripts that STAY (D-08): `build.sh setup.sh toolchain.sh iso.sh vm-build.sh lima-harvey.yaml drawterm-x64.sh drawterm.nix minimega.nix host-echo.c`. Fixtures (sweep `.rc`, termrc, `root-tree`, tiers, `.c` sources, `mkimage.sh`) are NOT copied; scripts reach them under the Harvey tree path.

Env/contract header to preserve in each moved script (Harvey `drawterm-gate-x64.sh` L80-123):
```sh
set -e
HARVEY="${HARVEY:-$PWD}"
err() { echo "error: $*" >&2; exit 1; }
[ -d "$HARVEY/sys/src" ] || err "$HARVEY does not look like the harvey checkout (missing sys/src) ..."
[ -n "${R9_KERNEL_X64_PKG:-}" ] || err "R9_KERNEL_X64_PKG unset — run via the flake app ..."
[ -n "${R9_ISO_BUILDER:-}" ] ...; [ -n "${R9_NETDEV_EMITTER:-}" ] ...; [ -n "${DRAWTERM_BIN:-}" ] ...
R9_ACCEL="${R9_ACCEL:-kvm}"
case "$R9_ACCEL" in
	kvm) [ -e /dev/kvm ] || err "/dev/kvm absent and R9_ACCEL=$R9_ACCEL — no silent downgrade: ..."; R9_CPU="${R9_CPU:-host}" ;;
	tcg) R9_CPU="${R9_CPU:-qemu64}" ;;
	*) err "R9_ACCEL=$R9_ACCEL: want kvm|tcg" ;;
esac
```
Changes the move requires: `HARVEY` must point at the built tree (`R9_HARVEY_DIR_AMD64`) instead of a writable checkout, so any script step that writes into `$HARVEY/build/x86_64/out` (the stage-build/mkimage steps, `R9_SKIP_BUILD`) must be replaced by the r9 assembler (D-16: r9 bakes the paq) and write to a scratch dir (`${TMPDIR}`/`target/`). Tree is read-only by construction. Keep the unique-tag teardown discipline (qemu `-name R9_RUN_TAG`, drawterm argv0 `R9_DT_TAG`); per Decision 5, reap drawterm by scratch/dial identity or the pty child pid, not by argv0 tag alone. ShellCheck: scripts are embedded through `writeShellApplication` (checkPhase runs shellcheck), mirror Harvey's existing scripts which already pass.

---

### rio leg (D-11, D-28) in the moved drawterm gate

**Analog:** `SP/s3/rio-gate.sh`, `SP/s3/rio-gate.expect` (derived from Harvey `nix/drawterm-gate-x64.sh` + `build/x86_64/drawterm-gate.expect`). Marker: serial `ls /srv` shows both `/srv/rio.<user>.<pid>` and `/srv/riowctl.<user>.<pid>`; control `-G` leg (no rio) shows only `/srv/boot`, `/srv/factotum` and stays unchanged. Known unresolved step (must be first task): the headless drawterm password feed. Harvey's gate reads `out/auth-creds.txt` (`user=` then `password=` lines, CR/LF stripped, error naming the path when empty) and exports `R9_DEMO_USER`/`R9_DEMO_PW`; the spike's failure was the DEMO password unknown. Because the bake now happens in r9 (D-16), the creds file must be produced by the r9 assembled cpu/auth image path; investigate `mkimage.sh:1290-1305` and `:1848-1856` (bake-time p9sk1 domain/creds) before writing the leg. Do not treat the `tr-ok` line from the owner's graphical login as evidence.

---

### Harvey own-kernel q35 launch app (D-26) (Harvey-side, last)

**Analog:** `SP/s5/boot-q35.sh` (capture-before-start, signature classification, kill/reap trap) and `SP/s5/evidence/build-path-analysis.md`. Reached state: `root is from (tcp)[tcp]: ` prompt. Flake app registered with Harvey's `mkScriptApp` (L26-39). Build the kernel it boots and record its sha256 (the committed `SP/s5/9k10cpu.elf` source commit is unknown). This is a Harvey-repo change and rides the single owner-approved Harvey commit (D-10).

---

### Temporary Rust a.out swap probe (`ps`, D-17/D-27/D-18)

**Analog:** `SP/s6/probe-amd64.rs`, `probe-amd64.ld`, `probe-arm64.{rs,ld}`, `aout.py`, `build-probe.sh`, `linux-x86_64.sh`, `q35shell-probe.expect`. Core (probe-amd64.rs L12-40): `#![no_std] #![no_main]`, looping `#[panic_handler]`, `global_asm!` `_start` that does PWRITE(51)/EXITS(8) with BP as syscall number and args at SP+8.. (amd64) with the marker in the same flat text; linker script places text at `0x200028` (arm64: `0x10028`), `--oformat=binary`, then a 40-byte big-endian Plan 9 header (magic, text, data, bss, syms, entry, spsz, pcsz) wrapped by `aout.py` and validated by its `check` mode that mirrors r9's `parse_aout`. Build command (Decision 4): plain `rustc --target x86_64-unknown-none -C panic=abort -C opt-level=s -C relocation-model=static -C link-arg=-T<ld> -C link-arg=--oformat=binary`, from the devshell, never in the cargo workspace. In xtask, the wrapper must be Rust (not Python); keep it as a small function in the probe path only and delete with the probe. Flow: manifest entry `ps = rust` -> assembler stages the probe -> gate checks the marker -> flip back and assert the assembled image member list equals the control. Sites that use `ps`: `build/x86_64/smoke-r9.expect:265`, `tier1-sweep.rc:137` (read-only; confirm they tolerate the marker, or skip those legs in the probe run). Removal commit must delete the probe sources, the manifest `rust` entry and any `elf2aout` helper; record the result in the phase summary.

---

### `.github/workflows/xtask.yml` (config)

**Analog:** the `boot-test-x86-64` job (L110-150 of the file): steps are `actions/checkout@v3`, apt provision (`grub-pc-bin xorriso mtools qemu-system-x86`), a Harvey checkout step (`repository: requaos/harvey, path: harvey`) and `env: R9_HARVEY_DIR: ${{ github.workspace }}/harvey`, then `cargo xtask boot-test --arch x86-64 --config q35 --accel tcg --cpu qemu64 --verbose`. Change: replace the Harvey checkout with a Nix install step and run the boot-test/gate inside `nix develop -c` so `R9_HARVEY_DIR_AMD64` is exported from the flake supply; remove `R9_HARVEY_DIR`. Keep explicit `--accel tcg --cpu qemu64` (hosted runners have no `/dev/kvm`; best-effort gate per D-13) and the long `timeout-minutes: 120`. Other jobs (`lint`, `tests`, `build-*`) do not touch Harvey and stay as is. A16: AppArmor-restricted runners need the proot venue, which the derivation already uses. Add a `nix build .#harvey-tree-amd64` step, cached through the runner's store, ahead of the gate. Relocated CI legs for gates that moved from Harvey follow this same job shape.

---

## Shared Patterns

### Named-error, no-silent-default
**Source:** `xtask/src/uregcheck.rs` L70-85; `main.rs` L3727-3735; `main.rs` L4317-4345 (`resolve_boot_module`).
**Apply to:** `harvey.rs`, `image.rs`, `GateStep`, all moved scripts (`err()` helper).
Every missing producer/variable returns an error naming the variable/file and the command that produces it; never fall back to a guessed path.

### Result type and error plumbing
**Source:** `xtask/src/lib.rs` L21-22 (`DynError`, `Result`); top-level handler `main.rs` L246-249.
**Apply to:** all new Rust modules. Return `Result<T>`; format with `format!(...).into()`; map foreign errors with `.map_err(|e| format!("context: {e}"))?`.

### No silent accel downgrade
**Source:** Harvey `nix/drawterm-gate-x64.sh` step 0b; xtask `--accel`/`R9_ACCEL` chain (`main.rs` boot-test args L186-189).
**Apply to:** `GateStep` and every moved boot script. KVM is the witness venue (D-13); TCG only by explicit selection.

### Unique-tag process hygiene
**Source:** Harvey `nix/drawterm-gate-x64.sh` header (qemu `-name R9_RUN_TAG`, drawterm argv0 tag, EXIT trap bracket-pkills).
**Apply to:** moved gate scripts and the rio leg (with Decision 5's caveat: reap by pty child pid / scratch identity).

### Reproducibility: kernel bytes must not change
**Source:** CONTEXT code_context; `flake.nix` L376 `kernel-x64` (FOD hashes `outputHash` L850).
**Apply to:** every flake or xtask edit. Run the kernel byte check; if kernel bytes change, the x86_64 byte-change procedure in `.planning/STATE.md` applies.

### Tests next to code
**Source:** `xtask/src/uregcheck.rs` L392+ inline `#[cfg(test)] mod tests` (string-fixture parsers, `assert_eq!` on `Result`, error text contains named field).
**Apply to:** resolver, manifest parser, drift check (both directions), tier-list parser.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| Remote-builder docs/config for the Mac (lima `ssh-ng` primary, Determinate native alternative, D-24) | docs | n/a | No existing doc; evidence in `SP/S2-FINDINGS.md`; add to `HowItWorks.md` next to the "R9_HARVEY_DIR contract" section |
| Pin-bump practice (D-06) | docs | n/a | New practice; document alongside the builder docs |
| Drift test of the assembler against a reference bake (D-29) | test | transform | Only analog is the spike's `SP/s3/determinism.sh` (which failed at the auth bake); write fresh member-list comparison, do not copy the byte differential |
| Partial: `xtask` `elf2aout` helper for the probe | utility | transform | Only analog is the Python `aout.py` (spike); port its header layout and `check` rules to Rust |

## Metadata

**Analog search scope:** `/Users/neilskinner/code/r9` (`xtask/src`, `flake.nix`, `.github/workflows`, spikes under `.planning/phases/01-.../spikes`), `/Users/neilskinner/code/harvey` (`flake.nix`, `nix/`, `build/{x86_64,aarch64}`).
**Files scanned:** about 25 read or grepped. Large files (`main.rs` 4937, `flake.nix` 908, Harvey `flake.nix`) were read by targeted ranges only.
**Pattern extraction date:** 2026-11-22
