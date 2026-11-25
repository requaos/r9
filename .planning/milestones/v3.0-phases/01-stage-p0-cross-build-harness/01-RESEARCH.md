# Phase 1: Stage P0 - Cross-Build Harness - Research

**Researched:** 2026-11-22
**Domain:** nix flake composition (two repos), sandboxed Plan 9 cross-build, QEMU q35 gate harness (Rust xtask + expect), root-image assembly
**Confidence:** MEDIUM (repo facts HIGH; the three load-bearing environment questions are untested spikes, see Wave 0)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried in from the roadmap session (owner, 2026-11-22)**
- **D-01:** r9's flake pins Harvey, and the default devshell (`nix develop`) exports the read-only tree location (`R9_HARVEY_DIR`) so `cargo xtask` finds it with no manual setup. xtask keeps no built-in default path; outside the devshell an unset variable stays a named error. (HARN-07)
- **D-02:** Flake ownership flips: r9 imports Harvey and produces the composed outputs Harvey's flake owns today; Harvey's flake drops its `r9` input. (HARN-08) — **Reversibility:** costly — both flakes, both lock files, CI and every gate entry point change together.

**Pre-built tree supply**
- **D-03:** The built Harvey artifacts are produced by a nix derivation in r9's flake, built from the pinned Harvey source. The devshell points at the store path, so the tree is read-only by construction. Harvey's in-tree build scripts (unshare namespaces, writes into the checkout) must be made to run inside the nix sandbox.
- **D-04:** On aarch64-darwin the same derivation is built by a nix Linux builder (the nix-darwin linux-builder, or the existing lima VM registered as a remote builder). One build path on every host; no separate lima script path for r9.
- **D-05:** Outputs are per guest architecture (amd64, arm64). The devshell exposes both; each is built only when a command for that architecture first needs it.
- **D-06:** The Harvey pin moves by deliberate bumps: one r9 commit per bump, which must pass the golden gate. For local Harvey work, an input override points at a checkout for that session only; it is never the default.

**Flake flip shape**
- **D-07:** The gate machinery moves into r9: the `nix/*.sh` gate scripts and the `build/<arch>/*.expect` drivers are copied into r9, which owns them from then on. Harvey's copies are deleted in the Harvey commit. This widens the approved Harvey diff beyond `flake.nix` to those deletions. — **Reversibility:** costly — about twenty scripts and fifteen expect drivers change home.
- **D-08:** Harvey's flake stays self-sufficient without r9. The owner's words: "harvey should still be able to build itself and developer should be able to pull harvey and run one nix command to build and launch harvey in q35 qemu and another to build and launch drawterm to connect to harvey, all without r9." Harvey therefore keeps its build, setup, toolchain and iso apps, its devshell, its `drawterm` package, and a one-command q35 launch of Harvey's own kernel. If that launch command does not exist today it is added in Harvey as part of this phase.
- **D-09:** r9 takes Harvey as a normal flake input (not `flake = false`) and may reuse Harvey's packages (for example `drawterm`, `harness`, `minimega`). The built-tree derivation of D-03 is still defined in r9.
- **D-10:** Order of the flip: r9 first. r9 gains the Harvey input and every composed output and proves each gate green from r9. Only then does a Harvey commit remove the `r9` input, the composed outputs and the moved scripts, followed by an r9 pin bump to that Harvey commit. No commit may leave a lock cycle or a gate with no home.

**Boot-to-rio gate**
- **D-11:** "Reached rio" is proven by the existing drawterm gate (boot the cpu image on q35, connect drawterm over the encrypted session) extended to start rio and assert a marker that rio is serving its window files. A display-less way to confirm rio is up is needed.
- **D-12:** Enforcement is by practice, not by hook: one `cargo xtask` command is the gate, it is run and green before each commit lands on main during planned work, and the result is recorded in the phase summary. CI is a second witness.
- **D-13:** The required green witness is a local x86_64 Linux run under KVM. Hosted CI runs the gate best-effort. On the Mac the gate may run under TCG as a convenience but is not the witness.
- **D-14:** Two tiers. The quick gate (reuse the already-built Harvey tree from the store, boot, reach rio) runs for every commit. The full set of moved gates (smoke, net, auth, services and the rest) runs at phase boundaries and at Harvey pin bumps.

**Fallback manifest and swap**
- **D-15:** The manifest has one entry per file in the root image (each binary and library object), per guest architecture, marked Harvey or Rust. The harness fails when manifest and image disagree in either direction.
- **D-16:** r9 assembles the root image. It draws each file from the built Harvey tree or from the r9 Rust build according to the manifest and bakes the `.paq` itself. Harvey supplies files, not finished images. — **Reversibility:** costly — r9 becomes the owner of image contents and of the bake that Harvey's `mkimage` scripts perform today.
- **D-17:** The swap is proven with a trivial `no_std` Rust program built as a Plan 9 `a.out` that replaces one harmless tool (for example `echo`) and prints a recognisable marker; the gate checks the marker, then the entry is flipped back. This pulls the minimum Rust-to-`a.out` path (`elf2aout`) forward from Phase 9, only as far as this probe needs.
- **D-18:** The probe is removed once it has proven the mechanism; the result is recorded. The default image stays all-Harvey and no throwaway code remains in the tree.

### Claude's Discretion
- The exact marker used to show rio is serving (D-11) and how it is observed without a display.
- Manifest file format and location inside `xtask/`.
- How existing `R9_HARVEY_DIR` consumers (boot-test, ureg-check, initrd, pcap `netaddr.rc`, `harvey:` module paths) are routed through one resolver.
- Which Linux builder mechanism to document for the Mac (D-04), provided the build path stays single.

### Deferred Ideas (OUT OF SCOPE)
- Re-expressing the shell and expect gates as `cargo xtask` commands in Rust was considered and not chosen for this phase; it remains an option before Harvey retirement (Phase 16).
- A binary cache for the built Harvey tree was considered for the Mac and not chosen.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HARN-01 | One documented `cargo xtask` command boots r9 on q35 to a rio session, non-zero exit if rio not reached | Existing q35cpu cpurc ladder + drawterm expect driver exist; rio is NOT asserted anywhere today (the gate runs `-G`, which has no draw device so the profile never starts rio). Recommended marker: `/srv/rio.<user>.<pid>` + `/srv/riowctl.<user>.<pid>` observed on the serial console, with a non-`-G` drawterm under Xvfb. See "Boot-to-rio gate". |
| HARN-02 | One resolver for `R9_HARVEY_DIR`/`HARVEY_SRC`/`vendor/harvey/`; every consumer goes through it; tree unmodified | Seven consumer sites enumerated below; store immutability + `nix store verify` gives the byte-for-byte proof. Moved gate scripts WRITE into the Harvey checkout today and must be rewired, not just copied. |
| HARN-03 | Manifest of every C library object and userland binary per architecture | Image contents enumerated from `mkimage.sh` (x86_64: 9 contracts; aarch64: 1). Manifest entry needs `image_path` distinct from `source` (the `h*` helper renames). Whether libraries (`.a`) are in scope is ambiguous (Open Question 3). |
| HARN-04 | Flip one entry to Rust, single artifact swaps in, gate passes, flip back restores | Probe path researched: lld `--oformat=binary` + a 40-byte BE header wrapper (header layout read from `x86_64/src/exec.rs`). |
| HARN-05 | Named error on image/manifest drift in either direction | Check at the staging-tree level (the exact input to `mkpaqfs`) plus provenance-path check for "marked Rust but resolves to Harvey". |
| HARN-06 | Existing q35 profiles and aarch64 virt quick pass still pass through the reconciled mechanism | CI today runs only 3 profiles that need no baked image; the profiles that do need images (q35exec/shell/churn/auth/cpu, virtip/virtnet) name `harvey:build/...` paths that must become manifest-assembled images. |
| HARN-07 | r9 flake supplies the read-only tree; devshell exports `R9_HARVEY_DIR` | Tension with D-05 (per-arch, lazy) and with devshell semantics (a `${drv}` in shellHook builds eagerly). Resolution: per-arch trees, path exported with context discarded, realized on first use. Needs owner confirmation of variable naming (Flag F7). |
| HARN-08 | Flake ownership flipped; all composed gates pass from r9; no lock cycle | Full output inventory, script/env map and a verified-acyclic commit order below. |
</phase_requirements>

## Summary

The phase is mostly a *relocation and rewiring* job around three genuinely new capabilities: (1) a sandboxed nix derivation that builds Harvey's amd64 and arm64 userland, (2) an r9-owned per-file root-image assembler with a Harvey/Rust manifest, and (3) a display-less rio assertion. Everything else (gate scripts, expect drivers, netdev emitter, ISO builder, kernel packages) already exists and works; it moves and is repointed at a read-only tree.

Three findings change the cost picture and need owner attention before planning locks. **First**, Harvey's userland pipeline is not a single build: it is host-toolchain bootstrap (`setup`/`toolchain`, plan9port-based), then `stage-build.sh` + `subset-build.sh` (tiered, amd64 Linux-native only), then `mkimage.sh` (2096 lines for x86_64, nine image contracts, with a bake-time p9sk1 auth domain generated by a host C program). D-16 asks r9 to own that bake; that is the single largest work item and is not "about twenty scripts moving". **Second**, the Mac venue is harder than D-04 assumes: this machine runs Determinate Nix managed by the nix-darwin *determinate module*, which disables nix-darwin's `nix.linux-builder`; Determinate's native Linux builder needs a FlakeHub login (currently logged out); and Harvey's source contains 23 case-colliding paths which I confirmed are stored mangled (`kill~nix~case~hack~1`) on this Mac's case-insensitive store. **Third**, Harvey today has no one-command launch of its own kernel on q35 and no evidence that its 9k `k10cpu` kernel serves a drawterm-usable cpu session; D-08's "if it does not exist it is added" is real new, unproven Harvey-side work.

**Primary recommendation:** Run five short Wave 0 spikes (listed under Open Questions / Validation) before committing to plan shape, then build in this order: resolver refactor (Mac-testable, no flake change) -> flake supply (mkpaqfs + per-arch tree derivations + devshell export) -> manifest/assembler engine with byte-differential tests against Harvey's own `mkimage.sh` -> quick gate with rio -> gate script move with rewiring -> owner-approved Harvey commit -> pin bump.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Build Harvey userland (cross toolchain, tier build) | nix derivation on a Linux builder | — | Needs mount namespaces; hermetic and read-only by construction (D-03) |
| Supply tree location to xtask | nix devshell env + xtask resolver | CI workflow env | D-01: devshell exports, xtask holds no default |
| Root-image assembly + manifest enforcement | xtask (Rust) | `mkpaqfs` host tool (nix package) | D-16: r9 owns image contents; paq byte format owned by Harvey's `mkpaqfs.c` |
| ISO composition (kernel + paq + append) | existing `r9-iso-builder` flake package | — | One reproducibility implementation, already shared |
| Boot driving + marker assertions | `expect` drivers (moved) orchestrated by xtask | xtask `boot-test` serial stages | Deferred idea: keep shell/expect, not Rust |
| Rio-serving proof | guest kernel `/srv` observed over serial | host Xvfb + drawterm (X11 backend) on a pty | rio only starts when the client serves `/dev/draw` |
| Gate app composition | r9 flake `apps` | Harvey flake (own-kernel launch only, after flip) | D-02/D-10 |
| Rust probe -> a.out | xtask throwaway wrapper | rustc + rust-lld | D-17/D-18, removed afterwards |

## Standard Stack

### Core
| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| Nix flakes (Determinate Nix on the Mac) | 2.34.6 / Determinate 3.21.0 [VERIFIED: `nix --version` on this host] | Both repos' composition and the tree derivation | Already the project's build system |
| nixpkgs `plan9port` | `plan9port-0-unstable-2025-11-10`, platforms include x86_64-linux and aarch64-linux [VERIFIED: `nix eval`] | Provides 9c/9l/mk/rc for Harvey's toolchain bootstrap and `mkpaqfs` host build | Harvey's own `setup.sh`/`toolchain.sh` use it |
| Harvey `mkpaqfs.c` built against plan9port | Harvey source at the pin | Owns the paqfs byte format (magic 529ab12b, trailer 6b46e688) | Do not reimplement; format carried verbatim on both archs |
| `expect` (Tcl) | nixpkgs | Existing gate drivers need a pty (drawterm's password prompt, no `PASS` env) | 15 drivers already written; deferred idea says do not rewrite |
| `xvfb` + `xvfb-run` | `xvfb-21.1.24`, `xvfb-run-1+g87f6705` [VERIFIED: `nix eval nixpkgs#...xvfb.name`] | Headless X server so drawterm's X11 backend serves `/dev/draw` | Only way to make rio start without a human display |
| Harvey `drawterm` package (`harvey.packages.<linux>.drawterm`) | Harvey pin | Client for the gate and for the developer wrapper | Linux-only package (gui-x11 backend); D-09 allows reuse |
| Existing r9 flake packages `kernel-x64`, `kernel-virt`, `kernel-virt-cpu`, `iso-builder`, `netdev-args` [VERIFIED: flake.nix:376-497] | at r9 HEAD | Kernel/ISO/netdev for every gate | Already exist; after the flip referenced as `self.packages` |
| Rust `toml` + `serde` (already in `xtask/Cargo.toml`) [VERIFIED: xtask/Cargo.toml] | toml 0.9, serde 1.0 | Manifest parsing | No new dependency needed |

### Supporting
| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| `lib.fileset` (nixpkgs) | in nixpkgs | Filter Harvey source to the files the tree derivation reads, so a pin bump that only touches `flake.nix`/`nix/` does not rebuild the tree | Tree derivation `src` [ASSUMED standard technique] |
| `util-linux` (`unshare`, `flock`) | nixpkgs | The userns+chroot-on-tmpfs venue inside the sandbox | Tree derivation build |
| `rust-lld` `--oformat=binary` | in the pinned nightly toolchain | Produces flat text bytes for the probe | Probe only (D-17); [ASSUMED] works, spike S6 |
| `nix store verify --no-trust` | nix | Proves the tree is unmodified after a gate run | Validation of HARN-02 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Determinate native Linux builder (recommended for the Mac) | lima VM `harvey` as an ssh-ng remote builder | Lima works without FlakeHub login but needs root-readable ssh key, trusted-users on the VM, a pinned `ssh.localPort`, and is aarch64-linux only. Both keep one build path (D-04). |
| Determinate native builder | nix-darwin `nix.linux-builder` | Not available on this Mac: the determinate module requires nix-darwin's Nix management off [CITED: docs.determinate.systems/guides/nix-darwin/ and github.com/alurm/determinate via search]. Do not document it as the path. |
| Per-arch derivation pair | One derivation, two outputs | Multi-output derivations build all outputs together; violates D-05 laziness. |
| `expect` driving drawterm | Rust pty crate in xtask | New dependency plus rewrite of working drivers; deferred by owner. |

**Installation:** no new Rust crates are recommended. If xtask needs content hashing for HARN-05, shell out to `sha256sum`/`shasum`; if a crate is unavoidable, `sha2` returned `OK` from the legitimacy seam (32M downloads/week, RustCrypto/hashes) but is not in `Cargo.lock` today, so adding it touches the lock that `importCargoLock` vendors.

**Version verification:** nixpkgs attrs above were evaluated against `nixpkgs` as resolved on this machine on 2026-11-22. No npm/PyPI packages are involved.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| sha2 (optional, only if a hash crate is needed) | crates.io | 10 yrs (2016-05) | ~32M/wk | github.com/RustCrypto/hashes | OK | Approved if needed; prefer no new crate |
| plan9port, xvfb, xvfb-run, expect, util-linux | nixpkgs | n/a (distro packages) | n/a | nixpkgs | n/a (not a registry fetch) | Approved; existing project practice, pinned by flake.lock |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                       r9 flake (owns composition after the flip)
  inputs: nixpkgs, rust-overlay, harvey (pinned rev; D-06)
  ----------------------------------------------------------------------
   harvey source (flake input, read-only, narHash pinned)
        |  lib.fileset filter (only dirs the build reads)
        v
  [harvey-hostbin  <linux system>]  plan9port + host cc -> 6a/6c/6l/7*/mk shims (setup.sh+toolchain.sh logic)
        |
        +--> [harvey-tree-amd64]  userns+tmpfs-chroot venue, objtype=amd64:
        |        stage-build + subset-build tiers 0-4  --> $out/{amd64/bin,amd64/lib,amd64/include,
        |                                                        build/x86_64/{root-tree,tiers,...},rc,lib,adm}
        +--> [harvey-tree-arm64]  same venue, objtype=arm64, subset-build (aarch64)
        |
        +--> [mkpaqfs]  Harvey mkpaqfs.c + plan9port libs   (per HOST system: darwin and linux)
        v
  nix develop shell: exports R9_HARVEY_DIR_{AMD64,ARM64}  (path string, context discarded -> lazy)
                     R9_MKPAQFS, DRAWTERM_BIN, ...
  ----------------------------------------------------------------------
  cargo xtask gate  (quick tier, D-14)
     1 resolve tree for arch (resolver; realize via `nix build` if the store path is absent)
     2 load manifest (xtask/manifest/x86_64.toml) -> stage dir (target/, NEVER the tree)
         entry: image_path <- harvey|rust|generated|skeleton
     3 drift checks (HARN-05)  -> mkpaqfs -> bootfs-cpu.paq
     4 iso-builder(kernel-x64 built from THIS commit, paq, --append="init=/amd64/init -c")
     5 expect driver: qemu q35 (slirp hostfwd 17010/5670) + Xvfb + drawterm (X11) + serial console
          boot ladder -> cpurc: LISTEN-17010 -> p9sk1 session -> profile `exec rio`
          serial: `ls /srv`  must show rio.<user>.<pid> and riowctl.<user>.<pid>   => exit 0
```

### Recommended Project Structure
```
r9/
├── flake.nix                     # + inputs.harvey, harvey-hostbin, harvey-tree-{amd64,arm64}, mkpaqfs, composed apps, devshell exports
├── nix/                          # moved gate scripts (D-07), rewired to read-only tree + scratch dir
├── gates/{x86_64,aarch64}/*.expect   # moved expect drivers (location is discretion; keep out of x86_64/ and aarch64/ crate dirs)
├── xtask/
│   ├── src/harvey.rs             # the ONE resolver (arch-aware), replaces uregcheck::harvey_dir body
│   ├── src/image.rs              # manifest load, staging, drift checks, mkpaqfs invocation
│   ├── src/gate.rs               # `cargo xtask gate`
│   └── manifest/{x86_64,aarch64}/<image>.toml
└── .github/workflows/            # + the legs relocated from Harvey's r9-shell.yml / phase6.yml
```
Do not place moved scripts under `x86_64/` or `aarch64/`: those directories are Rust crate roots and kernel inputs.

### Pattern 1: Lazy per-arch tree path in the devshell
**What:** export the store path of an unbuilt derivation without making the shell depend on building it.
**When to use:** HARN-07 + D-05 together. A bare `${drv}` in `shellHook` makes `nix develop` build both architectures (through the Linux builder on the Mac) every time.
**Example:**
```nix
# Source: nix manual builtins.unsafeDiscardStringContext [ASSUMED technique, spike S2 proves it end to end]
let treeAmd64Path = builtins.unsafeDiscardStringContext (treeFor system "amd64").outPath;
in ''export R9_HARVEY_DIR_AMD64="${treeAmd64Path}"''
```
xtask's resolver then does: variable set -> path exists? use it : run `nix build <flake>#harvey-tree-amd64 --no-link` (printing the command), re-check, else named error. An explicitly set plain `R9_HARVEY_DIR` (a local checkout override, D-06) wins over the arch variables.

### Pattern 2: Sandbox venue = the existing userns chroot scaffold, minus NixOS assumptions
**What:** Harvey already has an unprivileged venue in `build/x86_64/stage-build.sh` (`unshare -Ur -m`, tmpfs scaffold, chroot, leaf binds) [CITED: harvey/build/x86_64/stage-build.sh:196-262]. The derivation reuses the idea but must replace `ln -sfn "$(readlink /run/current-system)"` (NixOS-only) with binds of the store paths the build actually needs (`/nix/store`), and must not call `sudo`, `nix run .#setup`, or write into the source.
**Why this and not the sudo venue:** `nix/build.sh` does `sudo cp ... /bin/cpp`, `/bin/rc`, `/bin/6c...`; there is no sudo and no writable `/bin` in a nix sandbox.
**Hard rules from Harvey's own measurements (keep them):** never `rm -rf` a scaffold that binds source trees (the tmpfs must own it); bind leaf mounts only (`/nix/store`, individual `/dev` nodes); make `/bin` a real dir of symlinks plus the plan9port `rc` bind.

### Pattern 3: Image assembly from a manifest, validated by differential test
**What:** a TOML manifest per (arch, image) whose entries are `{ image_path, source = "harvey"|"rust"|"skeleton"|"generated", artifact = <tree-relative or r9-relative path> }`. xtask stages exactly these into `target/…/stage/`, then runs `mkpaqfs`.
**Verification technique:** Harvey's `mkimage.sh` pins staged mtimes to `KERNDATE` and mkpaqfs's header time rides it, so two bakes of one tree are byte-identical (stated in the mkimage header [CITED: harvey/build/x86_64/mkimage.sh header, "Determinism"]). The xtask assembler can therefore be proven correct for the deterministic images (marker, chain, tier, net, services, churn) by byte-comparing against a reference bake of the same tree. The auth and cpu images embed bake-time random p9sk1 keys, so compare their file lists and sizes instead, not bytes. [ASSUMED that the determinism claim still holds at the pin; spike S3 checks it.]

### Anti-Patterns to Avoid
- **Copying gate scripts verbatim.** Every gate sets `HARVEY="${HARVEY:-$PWD}"` and writes `build/x86_64/out/`, `.build/`, `smoke-out/` inside the checkout [CITED: harvey/nix/drawterm-gate-x64.sh `OUTD`; harvey/build/x86_64/stage-build.sh `STAGE`]. On a store path that fails or, worse, would be "fixed" by pointing at a writable copy and defeating HARN-02. D-07 says "copied"; the real task is copy-and-rewire.
- **Reading the tree on the Mac by file name for the 23 colliding paths** (see Pitfall 3).
- **Binding the lima mount.** virtiofs subdirectory binds silently yield empty trees [CITED: harvey/nix/vm-build.sh header]; any lima-based builder must build on guest-local disk, which the nix remote-builder model does by construction.
- **Widening a frozen gate list ahead of a pin** (Harvey flake comments, "research Pitfall 6"): in r9's own flake the attrs are local, so this concern disappears after the flip, but do not reintroduce `hasAttr` gating against `harvey.packages`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| paqfs image writer | A Rust paq writer | Harvey `mkpaqfs.c` built with plan9port (`9c`/`9l ... -lflate -lsec -lmp -lbio -l9`, recipe in `build/x86_64/mkpaqfs-host.sh`) packaged per host system | Byte format owner; kernel reader is already matched to it; aarch64 and x86_64 share it |
| ISO composition | Another grub/xorriso wrapper | `r9-iso-builder` (flake.nix:81) | Reproducibility pins (dates, volid, strip-all, store-hash zap) are hard won |
| Netdev argv | Literal `-netdev` strings in scripts | `r9-netdev-args` emitter (D-05 of the old phase) | Pinned TOML is the single source of the hostfwd pair |
| Host toolchain bootstrap for the tree | New bootstrap logic | Port `setup.sh` + `toolchain.sh` steps into a `harvey-hostbin` derivation | They already encode the 9l `-static` shim, the `nm` shadowing, the `iar`-as-`ar` ordering trap |
| Bake-time auth domain | New key generator | `build/x86_64/authdomain-gen.c` compiled with `stdenv.cc` | The cpu image's pairing assert and the gate's creds parse both depend on its outputs |
| pty driving of drawterm | Hand-rolled pty in Rust | existing `drawterm-gate.expect` | Password prompt is tty-only; the driver encodes the `-s ''` and teardown lessons |
| Flake source filtering | Manual copying | `lib.fileset.toSource` | Content-addressed filtered source prevents needless rebuilds on pin bumps |

**Key insight:** every hard part of this phase already exists as a *measured* shell artifact in Harvey. The risk is not inventing them, it is relocating them out of an assumption set (sudo, writable checkout, NixOS paths) they were written under.

## Runtime State Inventory

> Included because the phase moves gate ownership between repos and replaces a build venue. No rename of a string is involved.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None. No database keys carry repo names. The gate's p9sk1 domain is generated at bake time into per-run scratch (`out/auth-creds.txt`), not stored. | None. After the flip creds must land under r9 scratch, never the tree. |
| Live service config | GitHub Actions in Harvey: `r9-shell.yml` runs `nix run .#boot-r9-smoke`, `boot-r9-net-smoke`, `boot-r9-cpu`, `boot-r9-net-smoke-x64`, `boot-r9-auth-x64`; `phase6.yml` runs `nix run .#boot-r9-tier1` [VERIFIED: grep of the workflows]. r9's `xtask.yml` checks Harvey out and sets `R9_HARVEY_DIR` (lines 139-146 of the workflow). | Relocate those legs into r9 workflows; delete them from Harvey in the same Harvey commit or they go red; replace the Harvey checkout step in `xtask.yml` with the flake supply. |
| OS-registered state | Lima VM `harvey` exists on disk at `~/.lima/harvey` (Ubuntu 24.04 aarch64, vz, 60 GiB disk, 4 cpu, 8 GiB; `lima.yaml` mounts `~` writable). `limactl` is not on PATH (lima 2.2.0 is in the store). No `/etc/nix/machines`, `external-builders = []`. | Only relevant if the owner picks lima as the builder; then configure `builders` (see Environment Availability). |
| Secrets / env vars | `R9_HARVEY_DIR` (consumed), `R9_KERNEL_X64_PKG`, `R9_ISO_BUILDER`, `R9_NETDEV_EMITTER`, `R9_NET_DOCTOR`, `HOST_ECHO_X64`, `HARVEY_NETADDR_SEAM`, `DRAWTERM_BIN`, `R9_DEMO_USER/PW`, `KERNDATE`, `R9_ACCEL`. Credentials are redacted by the existing drivers. | Gate scripts keep the same names inside r9 so drivers need no semantic change; `HARVEY_NETADDR_SEAM` changes from `${./build/x86_64/root-tree/lib/netaddr.rc}` (a path in Harvey's own flake source) to `${harvey}/build/x86_64/root-tree/lib/netaddr.rc` or the tree. |
| Build artifacts | Harvey checkout on this Mac already holds gitignored outputs (`amd64/` 143 MB, `arm64/` 14 MB, `.build/hostbin-*`, `result`); r9 has `result` and `target/`. | None for correctness; note they are not inputs after the flip. `.build/bin/mkpaqfs` (the path `assemble_initrd` hardcodes) disappears as a contract. |

## Common Pitfalls

### Pitfall 1: `nix develop` builds everything
**What goes wrong:** exporting `${harvey-tree-amd64}` in `shellHook` realizes both trees (via the Linux builder on the Mac) on every shell entry, contradicting D-05.
**How to avoid:** Pattern 1 (context-discarded path + build-on-first-use in the resolver).
**Warning signs:** `nix develop` taking tens of minutes the first time; builder VM starting when you only wanted to run `cargo test`.

### Pitfall 2: Namespaces inside the nix sandbox are unproven
**What goes wrong:** Harvey's build needs mount namespaces (`mount --bind` of source onto `/sys`, `/amd64`, `/rc`, `/lib`...). In a sandbox the builder has no sudo; the only route is a nested `unshare -Ur -m`. This works on the owner's NixOS host outside the sandbox (Harvey measured it) but I have not seen it work *inside* a nix build. Known blockers: `kernel.unprivileged_userns_clone`/`user.max_user_namespaces`, AppArmor userns restriction on Ubuntu 24.04 (hosted CI, and the lima VM image), and the builder VM's kernel config.
**Why it happens:** the nix sandbox itself decides whether it already put the build in a user namespace; behaviour depends on `build-users-group`/`auto-allocate-uids`.
**How to avoid:** Spike S1 first. Fallback ladder if it fails: (a) `bubblewrap`/`proot` (proot needs ptrace, 2-10x slower) in place of raw `unshare`; (b) `vmTools.runInLinuxVM` (needs KVM in the builder, unlikely on the Mac native builder); (c) patch absolute paths (`/bin/cpp`, `/bin/rc`, `/sys`, `/amd64`) at the mkfile level inside a *copy* of the source, which is invasive and fragile. Option (c) is the last resort because Harvey's mkfiles use `</$objtype/mkfile` and `/sys/include` pervasively.
**Warning signs:** `unshare: unshare failed: Operation not permitted` or `mount: ... permission denied` in the first build log.

### Pitfall 3: Case collisions on the Mac store
**What goes wrong:** Harvey's tree has 23 paths that differ only by case [VERIFIED: ran `git ls-files` case-fold check on the Harvey checkout]: `rc/bin/{kill,Kill}`, `sys/lib/postscript/troff/{Hx,HX}`, `sys/lib/troff/font/devutf/{Hb,HB,Hi,HI,Hx,HX}`, `.../charlib/{lh,lH,LH,rc,RC,rh,rH}`, `sys/src/cmd/postscript/devpost.add/{c1,C1,c2,C2,c3,C3}`. I fetched Harvey `bd7a698` into this Mac's store and the colliding names are stored with the case hack (`kill~nix~case~hack~1`, `Hb~nix~case~hack~1`) [VERIFIED: `nix flake prefetch` then `ls`]. This Mac's `/nix` is an APFS volume and `use-case-hack = true` [VERIFIED: `nix show-config`]. Determinate's own troubleshooting page lists case-insensitive store collisions as a known failure of its native builder ("files differing only in case can overwrite each other, causing hash mismatches", fix = case-sensitive store volume) [CITED: docs.determinate.systems/troubleshooting/native-linux-builder/].
**Impact:** (1) the tree derivation's *source input* is case-mangled on the Mac and may mismatch when shipped to the builder; (2) any output copied back that contains colliding names is mangled; (3) xtask on the Mac cannot open those 23 names by their real names.
**How to avoid:** the tree derivation's outputs must not contain the colliding paths (copy only what images and ureg checks need: none of the 23 is in an image tier list, `rc/bin/kill`/`Kill` are not in `tiers/*.txt` for either arch — aarch64 tier1 even notes kill is missing [CITED: harvey/build/aarch64/tiers/tier1-cmds.txt:98]). For the source input, Spike S2 must build a real derivation on the Mac's builder from the real Harvey input and check the build sees `rc/bin/Kill` and `rc/bin/kill` as two files. Fallback: have the builder fetch Harvey itself (a fixed-output `fetchFromGitHub` on the Linux builder) so the source never round-trips through APFS.
**Warning signs:** hash mismatch on the Mac when copying the source to the builder; a `kill` file containing `Kill`'s bytes.

### Pitfall 4: Determinate native builder and nix-darwin interplay
**What goes wrong:** D-04 names "the nix-darwin linux-builder". On this Mac `/etc/nix/nix.custom.conf` is "generated by the determinate module for nix-darwin" and sets `sandbox = false` [VERIFIED: file read]; nix-darwin's `nix.*` options (including `nix.linux-builder`) are not usable under that arrangement [CITED: search result summarizing docs.determinate.systems/guides/nix-darwin and the Determinate 3.8.4 announcement]. The native builder works only when logged in to FlakeHub; `determinate-nixd status` reports `Authentication: logged-out` [VERIFIED: ran it]. Default VM memory 8 GiB [CITED: Determinate troubleshooting page].
**How to avoid:** document, in this order: Determinate native builder after `determinate-nixd auth login` (one command, no Nix config), or lima as `ssh-ng` remote builder. Do not plan on nix-darwin's linux-builder. Both satisfy D-04's "one build path".
**Choice of builder system on the Mac:** the tree content is guest-architecture-specific, not host-specific, so build with the native `aarch64-linux` builder (no emulation). Spike S4 checks that the amd64 tier build runs on an aarch64-linux host (Harvey's x64 scripts say "native-linux only" but check only `uname -s`, and the full `linux-build` already produced `amd64/9k10cpu.elf` through the aarch64 lima guest on this Mac).

### Pitfall 5: Harvey's x64 build scripts assume a NixOS host and a writable checkout
**What goes wrong:** `stage-build.sh` venue does `readlink /run/current-system`; `subset-build.sh` calls `flock` on `$HARVEY/.build/amd64-tree.lock`, runs `nix run .#setup && nix run .#toolchain` if hostbin is missing, and writes the build log and artifacts into the checkout. The mkimage-bound programs live in heredocs (`GUEST_EOF`) inside the scripts, so they cannot be invoked separately without extracting them.
**How to avoid:** the derivation owns a small r9 venue wrapper and drives Harvey's `mk`-level steps itself, using Harvey's tier data (`build/x86_64/tiers/*.txt`) as the work list. If the planner prefers to reuse the heredoc bodies, extract them from the pinned source at build time (`awk '/<<.GUEST_EOF./,/^GUEST_EOF/'`) rather than copying them into r9 (copying duplicates a moving target). Recommended: extract, with a hash assertion on the extracted text so a Harvey change fails loudly at pin-bump time.

### Pitfall 6: `-G` drawterm never starts rio
**What goes wrong:** the existing gate asserts a shell prompt over a `-G` session. The profile's cpu case only does `plumber; exec rio` when `/mnt/term/dev/draw` exists, which a `-G` session lacks [CITED: harvey/build/x86_64/root-tree/usr/glenda/lib/profile]. And Harvey's `gui-none` backend deliberately dies if any screen entry point is called [CITED: harvey/sys/src/cmd/unix/drawterm/gui-none/screen.c], so there is no headless draw backend to switch on.
**How to avoid:** use the X11 backend that the Harvey `drawterm` package already links, against Xvfb (see "Boot-to-rio gate").

### Pitfall 7: Kernel derivations take `src = ./.`
**What goes wrong:** adding files or editing `flake.nix`/`xtask` changes the `kernel-x64` derivation hash (every file in the flake source is an input), triggering rebuilds, though not byte changes. The `boot-iso` FOD hash (flake.nix:850) is of the ISO bytes, so it only needs re-derivation if kernel bytes change.
**How to avoid:** prove "no kernel bytes changed" per commit by comparing the kernel ELF sha256 before and after (the owner's stated method), not by comparing store paths. On the Mac: `cargo xtask build --arch x86-64 --config q35churnshell` and hash `target/x86_64-unknown-none-elf/debug/x86_64` before and after [path from flake.nix:391]. On Linux: `nix build .#kernel-x64` and hash `kernel-x64.elf`.

### Pitfall 8: Lock-file duplication, not cycles
**What goes wrong:** r9 importing Harvey while Harvey still has `inputs.r9` produces nested lock nodes. This is not a cycle.
**Evidence:** [VERIFIED: ran `nix flake lock` in a scratch copy of r9 with `inputs.harvey.url = "github:requaos/harvey/bd7a6982a"`]: it succeeded and produced nodes `harvey`, `harvey/r9` (r9 `116bcc2`), `nixpkgs_2`, `rust-overlay_2`. Both repos are public and fetchable for pushed commits. **Harvey `25d97dc4d` (the freeze note) is unpushed and returned HTTP 404** from GitHub, so r9 can only pin pushed Harvey revisions; pushes need owner approval and must precede any r9 commit that locks to them (push-before-pin).
**How to avoid:** add `inputs.harvey.inputs.nixpkgs.follows = "nixpkgs"` to keep one nixpkgs [ASSUMED fine for `drawterm`/`harness` builds]; do not try `inputs.harvey.inputs.r9.follows = ""` before the flip because Harvey's `outputs = { self, nixpkgs, r9 }` function would then fail to evaluate at all.

### Pitfall 9: Gate cannot run on this Mac
See "Owner constraints" below. Plans must label each verification command with its venue.

## Code Examples

### The seven consumer sites that go through the new resolver [VERIFIED: grep of r9 + Read of listed files]
1. `xtask/src/uregcheck.rs:76-85` `harvey_dir()` — the only reader of `R9_HARVEY_DIR`; error text: `"R9_HARVEY_DIR is not set — point it at the Harvey checkout (it must contain arm64/include/ureg.h and amd64/include/ureg.h, and for the initrd bake build/aarch64/smoke-out/ plus .build/bin/mkpaqfs); r9's xtask no longer defaults to a sibling ../harvey checkout (05-CONTEXT D-10)"`
2. `uregcheck.rs` `run_check` / `check_for_boot_test` — read `arm64/include/ureg.h`, `amd64/include/ureg.h`
3. `xtask/src/config.rs:241-263, 750-753` — `harvey:`-prefixed `initrd.file` resolution
4. `xtask/src/main.rs:3729-3760` `assemble_initrd` — reads `build/aarch64/smoke-out/{INITRD_FILES}` and `$R9_HARVEY_DIR/.build/bin/mkpaqfs` (an existing per-file assembly precedent for aarch64)
5. `main.rs:3950-3990` `ensure_initrd_image` — `harvey:` image passthrough
6. `main.rs:4317-4345` `resolve_boot_module` — `[dist] module = "harvey:build/x86_64/out/bootfs-*.paq"`
7. `xtask/src/pcap.rs:126, 189, 848` — `build/aarch64/netaddr.rc` (a *source* file, so the tree must carry it)

TOML references that become manifest-assembled images [VERIFIED: grep of the config files]: `config_q35exec.toml:91` and `config_q35churn.toml:70` -> `bootfs.paq`; `config_q35shell.toml:97` and `config_q35churnshell.toml:122` -> `bootfs-chain.paq`; `config_q35cpu.toml:94` -> `bootfs-cpu.paq`; `config_q35auth.toml:89` -> `bootfs-substrate.paq`; `config_virtip.toml:42`, `config_virtnet.toml:44` -> `build/aarch64/bootfs.paq`.

Recommended replacement scheme: `module = "image:chain"` style values resolved by the assembler (cached under `target/images/<arch>/`), keeping `harvey:` only for plain source files such as headers.

### Resolver contract (HARN-02)
```rust
// xtask/src/harvey.rs  (sketch; names are the planner's call)
pub enum HarveyArch { Amd64, Arm64 }
/// Order: R9_HARVEY_DIR (explicit single-tree override, local checkout or CI)
///        > R9_HARVEY_DIR_AMD64 / R9_HARVEY_DIR_ARM64 (devshell, lazy store paths)
///        > HARVEY_SRC (explicit env only; never a probed path)
///        > Err naming every variable tried.
/// No `../harvey` and no implicit `vendor/harvey/` probe (D-01).
pub fn harvey_tree(arch: HarveyArch) -> Result<PathBuf>;
```
`vendor/harvey/` is accepted by setting `R9_HARVEY_DIR=$PWD/vendor/harvey`; the directory does not exist at HEAD (RETIRE-05 removes it later). This reading of HARN-02's three names against D-01's "no built-in default" is an interpretation (Assumption A14).

### Gate script -> env contract that moves unchanged [CITED: harvey/flake.nix:114-472]
| App (Harvey today) | Script | r9 attrs consumed | Extra |
|---|---|---|---|
| boot-r9 | boot-r9.sh | `R9_KERNEL_PKG`=kernel-virt | qemu, dtc, git (+lima on darwin) |
| boot-r9-smoke | smoke-r9.sh | kernel-virt | expect |
| boot-r9-net-smoke | smoke-r9-net.sh | kernel-virt | expect |
| boot-r9-tier1 | tier1-r9.sh | kernel-virt | expect |
| boot-r9-cpu | cpu-r9.sh | **kernel-virt-cpu** | expect |
| boot-r9-x64 | boot-r9-x64.sh | `R9_KERNEL_X64_PKG`, `R9_ISO_BUILDER`, `R9_NETDEV_EMITTER`, `R9_NET_DOCTOR` (net-doctor) | |
| boot-r9-smoke-x64 | smoke-r9-x64.sh | kernel-x64, iso-builder, netdev-args | expect, iproute2, procps |
| boot-r9-tier1-x64 | tier1-r9-x64.sh | kernel-x64, iso-builder | expect |
| boot-r9-net-smoke-x64 | smoke-r9-net-x64.sh | + netdev-args | expect, iproute2, procps |
| boot-r9-services-x64 | services-r9-x64.sh | kernel-x64, iso-builder | expect |
| boot-r9-net-soak-x64 | soak-r9-net-x64.sh | + netdev-args, `R9_NET_DOCTOR`, `HOST_ECHO_X64`, `HARVEY_NETADDR_SEAM` | tap venue |
| boot-r9-churn-x64 | churn-r9-x64.sh | kernel-x64, iso-builder | expect, procps |
| boot-r9-stress-x64 | stress-r9-net-x64.sh | as soak | |
| boot-r9-auth-x64 | smoke-r9-auth-x64.sh | kernel-x64, iso-builder, netdev-args | stdenv.cc (authdomain-gen) |
| drawterm-gate-x64 | drawterm-gate-x64.sh | as auth + `DRAWTERM_BIN` | linux only |
| net-doctor (linux) | net-doctor.sh | none | |
| drawterm (linux) | drawterm-x64.sh | `DRAWTERM_BIN` | |

Inventory totals: scripts that move = 16 `nix/*.sh` (boot-r9, boot-r9-x64, churn-r9-x64, cpu-r9, drawterm-gate-x64, net-doctor, services-r9-x64, smoke-r9-auth-x64, smoke-r9-net-x64, smoke-r9-net, smoke-r9-x64, smoke-r9, soak-r9-net-x64, stress-r9-net-x64, tier1-r9-x64, tier1-r9) plus `host-echo.c`; expect drivers = 10 in `build/x86_64` (churn-r9, cpu-bringup, drawterm-gate, netsoak-r9, services-r9, smoke-r9-auth, smoke-r9-net, smoke-r9, stress-r9, tier1-r9) + 5 in `build/aarch64` (cpu-r9, drive-r9, smoke-r9, smoke-r9-net, tier1-r9) = 15, matching D-07's counts [VERIFIED: directory listings].
**Stays in Harvey:** `setup.sh`, `toolchain.sh`, `build.sh`, `vm-build.sh`, `iso.sh`, `lima-harvey.yaml`, `drawterm.nix`, `minimega.nix`, plus (D-08) its devshell and the `drawterm` package. `drawterm-x64.sh` (the developer wrapper) is both: HARN-08 lists `drawterm` as a moved helper, D-08 says Harvey keeps a drawterm launch. Plan a copy in r9 (pointing at r9's cpu image creds/hostfwd) and a Harvey-owned variant for Harvey's own kernel (see D-08 finding).
**Gate data that is not a script or expect driver and is still read by them:** `build/*/root-tree/`, `tiers/*.txt`, `*-sweep.rc`, `*-termrc`, `trace-*`, `authdomain-gen.c`, the fixture `.c` programs, `mkimage.sh`/`stage-build.sh`/`subset-build.sh`. Recommendation: these stay in Harvey as source files the tree supplies (D-07 does not list them); r9 stops calling `mkimage.sh` once the assembler lands but Harvey need not delete it (no scope creep in the owner-approved Harvey diff). Planner decision: confirm with the owner.

### Probe: minimal a.out facts (D-17) [VERIFIED: x86_64/src/exec.rs:598-615, 656-696 read this session]
```rust
/// S_MAGIC amd64: _MAGIC(HDR_MAGIC, 26) = 0x8A97.
pub const S_MAGIC_AMD64: u32 = 0x8A97;
/// HDR_MAGIC header size: 8×BE u32 + the BE u64 entry at bytes 32..39;
pub const HDR_SIZE: usize = 40;
pub const INITTEXT: usize = 0x200000 + HDR_SIZE;
pub const TEXT_MAP_BASE: usize = 0x200000;
pub const DATA_ALIGN: usize = 0x200000;
// parse_aout: text@4, data@8, bss@12, syms@16 (BE u32); "The u32 entry field at 20 is vestigial";
// entry = BE u64 at 32..39; requires text != 0 and INITTEXT <= entry < INITTEXT + text.
```
Syscall ABI (from `amd64/ABI.md`, "Wire contract") [CITED: harvey/amd64/ABI.md:29-62]: number in `BP`, arguments in 8-byte slots at `SP+8` upward, `SP+0` reserved, trap is `SYSCALL`, result in `AX`. Numbers [VERIFIED: sys/src/libc/9syscall/sys.h:9,50]: `#define	EXITS		8`, `#define	PWRITE		51`.
A throwaway probe therefore needs: `#![no_std] #![no_main]`, a panic handler, a `global_asm!` `_start` (rbp cannot be an `asm!` operand) that stores fd/buf/len/-1 at `rsp+8..`, `mov ebp, 51; syscall`, then `mov ebp, 8; syscall`. Text placed at `0x200028` with `lld --oformat=binary` (flat output starting at the first section), prepend the 40-byte BE header with `text` = payload length, data/bss/syms = 0, entry u64 = `0x200028`. No data or bss segment is needed if the marker string lives in `.rodata` folded into the text output. This is a *flat single-segment* wrapper, deliberately not the Phase 9 `elf2aout` (which must handle data/bss and real ELF segments), which is why D-18's removal is clean. Exact asm and the lld flags are [ASSUMED] until spike S6.
First instruction contract: the kernel puts the `Tos` pointer in `AX` and argc/argv on the stack; a probe that ignores both is fine (ABI.md "Exec return / `_tos`").
Replacement target: the chain, tier, net and cpu images stage `ls cat echo ps rm test` as `/amd64/bin/*` [CITED: harvey/build/x86_64/stage-build.sh:196-218 and mkimage cpu section header]. rc's `echo` is a builtin, so the gate must run `/bin/echo` by path to execute the replacement binary.

## Findings on the Locked Decisions

### D-03: what Harvey's userland build actually does
1. **Host toolchain** (`nix/setup.sh`, `nix/toolchain.sh`): copies plan9port's `9c` as `pcc` and `sed yacc lex awk date dd rm basename ls` into `$HARVEY/.build/hostbin-<os>-<arch>/bin`; stubs `size`; builds `6a 6c 6l 8a 8c 8l 7a 7c 7l cpp aux nm` and the vendored `9cc` `iar` (installed as `ar`) with host cc. Shims: a `9l` wrapper drops `-static`; `nm` is shadowed by binutils `nm`; `ar` must come from the wrapper PATH, not hostbin, while building 9cc [CITED: harvey/nix/toolchain.sh]. No network is used after the nix inputs resolve. -> derivation `harvey-hostbin`.
2. **Full namespace build** (`nix/build.sh` -> `sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE` -> `sudo unshare -m bash BUILD_SETUP_NAMESPACE`): binds `$HARVEY/{sys,arm,amd64,arm64,386,mips,...,acme,rc,mail,lib(merged),unix,usr/glenda}` onto absolute `/…` paths, then `su` runs `BUILD` which runs `mk clean; mk libs; mk all` and `rc build/scripts/linux-build`; that script builds `arch=(amd64 arm64)` (libc, compiler dirs, `mk install`) and, for amd64, the 9k kernels `k10cpu k10cpuf k10f`. It does `sudo cp` of `cpp`, `bind`, `rc`, compilers, `ls`, `cp` into the host `/bin`, and edits mkfiles in place with `sed -i` [CITED: harvey/nix/build.sh, sys/src/BUILD_SETUP_NAMESPACE, build/scripts/linux-build]. Not usable in a sandbox as written.
3. **What the images actually consume** is not the full build. For amd64 the mkfiles are objtype-gated to tier data, and the producers are `stage-build.sh` (marker and chain binaries harvested to `build/x86_64/out`, compiled fixtures to `smoke-out/`) and `subset-build.sh` (tiers 0-4 with `T0..T3-ART` markers) [CITED: their headers]. arm64 has its own `stage-build.sh`/`subset-build.sh` and a lima leg. The derivation should run these (or their `mk` equivalents), not `linux-build`, and should skip the 9k kernels (Harvey-side concern, relevant only to D-08).
4. **Writes into the checkout:** everything above. In the derivation: copy the filtered source into `$TMPDIR/work`, run there, `install` only the needed paths into `$out`.
5. **Time:** Harvey's hosted CI gives the amd64 userspace-plus-kernels build a 60 minute job timeout [CITED: harvey/.github/workflows/harvey.yml `timeout-minutes: 60`]. No measured tier-only duration found; treat as tens of minutes per architecture [ASSUMED]. `lib.fileset` filtering keeps pin bumps from re-paying it.
6. **Reproducibility:** Harvey explicitly does not claim bit-reproducible userland [CITED: harvey/build/REPRODUCIBILITY.md "Scope"] and the 9k kernel embeds the absolute build path. In nix the build path is `/build` and stable, so rebuilds are consistent in practice; do not rely on identical bytes across builder systems.
7. **Unknowns that decide feasibility:** spikes S1, S2, S4.

### D-04: Linux builder options
| Option | Needs | Systems | Verdict for this Mac |
|---|---|---|---|
| Determinate native Linux builder | Determinate Nix >= 3.8.4 (have 3.21.0, update available 3.24.3), FlakeHub login (`determinate-nixd auth login`; currently logged out), Apple Silicon | aarch64-linux and x86_64-linux | Recommended primary. VM default 8 GiB, configurable via `builder.memoryBytes` [CITED: docs.determinate.systems/determinate-nix/linux-builder/ and troubleshooting page]. Known case-collision caveat. |
| lima VM `harvey` as remote builder | `builders = ssh-ng://<user>@127.0.0.1:<port> aarch64-linux <root-readable key> 4 1 big-parallel` in nix.conf/machines; Nix + trusted-users on the VM; `ssh.localPort` pinned in `lima.yaml`; `limactl` on PATH (lima 2.2.0 in store). The daemon runs as root so it cannot use `~/.lima` keys. | aarch64-linux only | Works offline of FlakeHub; more moving parts. NAR transport reverses the case hack on the way to a case-sensitive store [ASSUMED]. |
| nix-darwin `nix.linux-builder` | nix-darwin managing Nix | aarch64-linux | Not available: determinate module in use [CITED]. |
Guest architectures do not select the builder system: `amd64` and `arm64` Plan 9 binaries are produced by the cross toolchain on whichever linux system builds. Define the tree derivations once per *linux* system and let darwin devshells reference the aarch64-linux ones (`pkgsFor "aarch64-linux"`).
**Single build path:** satisfied on every host, because on Linux the same derivation builds locally (needs userns enabled, see Pitfall 2).

### D-05 / HARN-07: per-arch trees, one variable
HARN-07 literally says the devshell exports `R9_HARVEY_DIR`; D-05 says two architectures, built lazily. A single variable cannot name two trees. Recommended reconciliation: the resolver accepts `R9_HARVEY_DIR` as the unqualified override and the devshell exports `R9_HARVEY_DIR_AMD64` and `R9_HARVEY_DIR_ARM64`, each tree output being a complete view for its arch (built artifacts + the source-side files that arch's consumers read: headers, `root-tree`, `tiers`, `rc`, `lib`, `adm`, `netaddr.rc`). **Flag F7: needs owner confirmation that this satisfies HARN-07's wording.**

### D-07 / D-09 / D-10: output inventory and commit order
**Harvey flake outputs today** [VERIFIED: Read of harvey/flake.nix]: packages `harness` (all systems), `minimega`, `drawterm` (linux only); apps `setup`, `toolchain`, `build`, `iso`, `default`(=setup), then composed apps in four optionalAttrs blocks (aarch64 kernel-virt: `boot-r9`, `-smoke`, `-net-smoke`, `-tier1`; kernel-virt-cpu: `boot-r9-cpu`; x64 block: `boot-r9-x64`, `-smoke-x64`, `-tier1-x64`, `-net-smoke-x64`, `-services-x64`, `-net-soak-x64`, `-churn-x64`, `-stress-x64`, `-auth-x64`, `drawterm-gate-x64`; linux-only: `net-doctor`, `drawterm`); devshell `default`. Internal helper `hostEchoX64Pkg` (host cc build of `nix/host-echo.c`) must also move.

**Harvey own-kernel launch (D-08) — does it exist?** No. `nix run .#boot-r9-x64` launches *r9's* kernel. Harvey's `drawterm` app is wired to the r9 cpu image's hostfwd pair (cpu 127.0.0.1, auth `tcp!127.0.0.1!5670`, user glenda) and the credentials of the r9 bake. What Harvey has for its own 9k kernel are test harnesses, not developer launches: `build/mm/boot-9k10cpu.sh` (minimega, needs root and KVM or TCG), `boot-9k10cpu-qemu.sh` and `boot-9k10t1-sweep.sh` (`qemu-system-x86_64 -kernel amd64/9k10cpu.elf -m 512 -display none -serial file:...`, no `-M q35`, no network) [CITED: harvey/build/mm]. `k10cpu` does include `ethervirtio`, the IP stack and `cpuserver = 1` with embedded factotum/ipconfig/nvram [CITED: harvey/sys/src/9k/k10/k10cpu], but I found **no recorded evidence that it boots to a root and serves a cpu listener a drawterm can authenticate to**; `boot cpu` there prompts for a root. **Flag F2: D-08's own-kernel launch is new and unproven Harvey work; add a Wave 0 spike (S5) before promising it, and scope it as "app that builds and launches `amd64/9k10cpu.elf` under `-M q35` with virtio-net hostfwd" with the cpu-service-up claim explicitly optional.** Drawterm packages exist on Linux only; on darwin Harvey has `-G` text only.

**Commit order (verified acyclic at every step):**
1. r9: resolver refactor (xtask only). No flake or lock change.
2. r9: flake gains `inputs.harvey` pinned to a *pushed* Harvey rev (`bd7a698`, the `legacy-final` commit, verified fetchable; `25d97dc4d` is unpushed), `harvey-hostbin`, `harvey-tree-{amd64,arm64}`, `mkpaqfs`, devshell exports. `flake.lock` gains nested `harvey/r9` (116bcc2), `nixpkgs_2`, `rust-overlay_2` [VERIFIED by probe]. Harvey unchanged.
3. r9 (several commits): manifest/assembler, `gate`, composed apps in r9 referencing `self.packages.*` (not a pin), copied and rewired scripts and expect drivers, workflows relocated. Both repos can run every gate at this point (two homes, no gap). Run the full set at the last commit of this group.
4. **Harvey (owner approval, then push):** remove `inputs.r9`, composed apps, moved scripts/expects, host-echo, the r9-hosted workflow legs; add own-kernel launch; `flake.lock` drops the r9 nodes. Must not delete anything the r9 tree derivation still reads at the pin (build drivers, tiers, root-tree, fixtures stay).
5. r9: lock-only bump of `harvey` to the pushed Harvey commit from step 4; nested `harvey/r9` nodes disappear; run the golden gate (D-06) and the full set (D-14).
6. r9 pushes only after owner approval; push order Harvey before the r9 commit that locks to it.
The `legacy-final` tags (r9 `116bcc2`, Harvey `bd7a698`) keep the old arrangement reproducible. Harvey's lock pins r9 `116bcc2` today [VERIFIED: flake.lock read].

### D-11: how the gate works end to end today and the rio marker
Pipeline (`nix/drawterm-gate-x64.sh`) [CITED]: step 0 contract checks and accel selection (`R9_ACCEL` kvm default, tcg explicit, no silent downgrade) -> emitter slirp argv -> step 1 `stage-build.sh` + `subset-build.sh` -> step 2 `mkimage.sh` (all contracts, cpu image last so `out/auth-creds.txt` is the cpu pair) with post-bake greps (`cpu pairing verified`) -> step 3 creds parse (loud on empty) -> step 4 `iso-builder … --append="init=/amd64/init -c" bootfs-cpu.paq bootfs.paq` -> step 5 `drawterm-gate.expect`: qemu under the slirp argv (hostfwd 17010 and 5670), wait for the `cpurc: LISTEN-17010` ladder and both host listeners, spawn `drawterm -G -s '' -O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=$R9_DEMO_USER` on a pty, feed the password, assert the shell prompt, a fed marker, and the teardown line. D-14/D-15 of the old phase: the green witness is local KVM green twice.
**What r9 HEAD can do with rio:** the owner human-accepted a windowed rio session on x86_64 q35 under Linux KVM (legacy 19-10, r9 `0921a1bc`, screenshot recorded) [CITED: .planning/legacy/phases/19-cpu-service-drawterm-rio-desktop/19-10-SUMMARY.md]. So r9 *can* run rio via a drawterm that serves a display; nothing automated asserts it, and `legacy-final` is later than that evidence (the current pin is `116bcc2`), so re-prove, do not assume.
**What rio exposes (read this session):** rio posts two srv entries, `sprint(srvwctl, "/srv/riowctl.%s.%d", fs->user, pid); post(srvwctl, "wctl", p0);` and `sprint(srvpipe, "/srv/rio.%s.%d", fs->user, pid); post(srvpipe, "wsys", fs->cfd);` [VERIFIED: sys/src/cmd/rio/fsys.c:150-170]. Window dirs appear under `/mnt/wsys` once mounted.
**Recommended marker (Claude's discretion, D-11):**
- Host: `xvfb-run`/`Xvfb :N` and a non-`-G` drawterm (the package's X11 backend) on the existing pty, same feedstage, so the cpu profile sees `/mnt/term/dev/draw`, runs `plumber; exec rio`.
- Observation without a display, no second drawterm: after `cpurc: DONE`, init hands the *serial console* an interactive rc (`init: starting /bin/rc`, once, after DONE [CITED: xtask/src/main.rs:2493-2517 comment]). The expect driver already owns qemu's stdio, so it types `ls /srv` there and requires both `rio.<user>.<pid>` and `riowctl.<user>.<pid>`. Optional strengthening: also `echo RIO-SERVING > '#c/cons'` after `test -e /srv/rio.*`, mirroring the replay technique.
- Optional second witness: `xwd -root | …` dominant-colour check on Xvfb. Weaker and brittle; use only if `/srv` is not visible.
- Failure modes to make loud: drawterm exits (rio died), no `/srv/rio.*` within the stage timeout, `exec rio` skipped because `$wsysup` was set.
**Unverified premise (A5):** that `/srv` is kernel-global in r9's devsrv and visible from the serial console's namespace. The cpu image's `/lib/namespace` binds `#s /srv` [CITED: root-tree/lib/namespace via harvey chain image header], which supports it. Spike S3 proves it on the Linux witness.

### D-15/D-16: how the root image is baked today
- Two arches share one tool: Harvey's `mkpaqfs.c` as a host binary (`.build/bin/mkpaqfs`, built by `mkpaqfs-host.sh`: `9c -O0 -I… -c mkpaqfs.c` then `9l -o … -lflate -lsec -lmp -lbio -l9`), not the 9front donor's copy [CITED: harvey/build/x86_64/mkpaqfs-host.sh]. It is *not* a Harvey-built Plan 9 tool; it is a plan9port-built host tool, so it is per host system (darwin and linux), outside the tree.
- x86_64 `mkimage.sh` (2096 lines) bakes nine images from the committed skeleton `build/x86_64/root-tree/` plus built binaries: `bootfs.paq` (marker), `bootfs-chain.paq`, `bootfs-tier.paq`, `bootfs-net.paq`, `bootfs-services.paq`, `bootfs-substrate.paq`, `bootfs-auth.paq`, `bootfs-cpu.paq`, and the churn variant. Common content: `/boot/boot`, `/boot/paqfs`, `/amd64/init`, `/amd64/bin/rc`, verbatim `/rc/lib/rcmain`, `/rc/bin/termrc`, `/lib/namespace`, `/adm/timezone/local`, `/usr/glenda/lib/profile`, empty bind-target dirs; NO `/boot/factotum` (absence is the auth stub). Gate payloads add: tier1 command set (`tiers/tier1-cmds.txt`), net binaries and 6c fixtures (`tcpecho tcpblast netblast`), services set (`tiers/tier2-services.txt`), the font tree (`lib/font/bit`, manifest `bootfs-payload.txt`), `/lib/ndb/local` substituted from `local.template` via the netaddr seam, and for auth/cpu a bake-time p9sk1 domain from `authdomain-gen` with a pairing assert [CITED: mkimage.sh header and cpu section 15].
- aarch64 `mkimage.sh` (347 lines) bakes one `bootfs.paq`: `/boot/{boot,paqfs}`, `/arm64/init`, 16-binary subset + probes (`SUBSET="rc grep cat cp date echo ls mkdir mv ps rm sed test wc dd"` + stacksmoke, netblast, ipconfig, ping, udpecho, aan, tcpblast, tcpecho, fork/fd/brk/cexec/rclose/ssl/fp/cpu probes), same skeleton idea [CITED: harvey/build/aarch64/mkimage.sh:1-90].
- **Manifest entry shape needed:** `h*` helper names (`hcat`, `htest`, `hsleep`, `hsha1sum`) are renamed copies, so an entry must carry `image_path` and `source` separately. Entries also include fixtures compiled from Harvey's own `build/<arch>/*.c` with Harvey's compilers; those must be built into the tree (they are "Harvey artifacts") and listed in the manifest.
- **Flag F1 (largest cost item):** D-16 is a port of nine x86_64 image contracts plus the aarch64 one, including the auth-domain bake. Mitigations: (1) one generic stage-and-bake engine plus declarative per-image manifests, not nine hand-written bakes; (2) differential byte tests against a reference `mkimage.sh` bake for the deterministic images; (3) keep `authdomain-gen.c` as a tree-supplied host-compiled helper; (4) order the images by what the quick gate and HARN-06 need (cpu, chain, exec/marker, churn, auth/substrate for r9 boot-test profiles, aarch64 `bootfs.paq`), the rest (tier, net, services) at the "moved gates" step.
- **Drift check placement (HARN-05):** an "image" is a `.paq`; no Rust reader exists in xtask. Enforce at the staging tree (exactly mkpaqfs's input), plus the provenance rule for Rust entries (resolved path must not be under the Harvey tree and bytes must differ from the Harvey artifact). A `.paq` re-read is optional and costs a reader.

### Existing test/gate infrastructure (HARN-06)
- `cargo xtask` subcommands [VERIFIED: main.rs:90-245]: `build`, `expand`, `kasm`, `dist`, `test`, `clippy`, `check`, `qemu`, `boot-test`, `ureg-check`, `initrd`, `clean`.
- x86_64 boot-test profiles: `q35`, `q35lowmcfg`, `q35net` (need only `amd64/include/ureg.h` from Harvey), `q35exec`, `q35churn` (marker paq), `q35shell`, `q35churnshell` (chain paq), `q35cpu` (cpu paq + `init=/amd64/init -c` append; stage `Q35CpuRc` asserts `cpurc: DONE`/no `FAIL`), `q35auth` (substrate paq).
- aarch64: `virt`, `virtshell`, `virtip`, `virtnet` (Harvey `bootfs.paq` initrd), `virtcpu`, etc.; HARN-06's "virt quick pass" is tier-1 on HVF and TCG, smoke/net-smoke/cpu once on HVF, run through the moved Harvey-hosted-today gate scripts.
- Hosted CI `xtask.yml` runs `check`, `clippy`, `test`, builds, and three TCG profiles with a Harvey checkout for `ureg.h` [CITED: .github/workflows/xtask.yml:100-146]. Those three need no baked image, so CI is not exercising the assembler; the new gate and relocated legs must be added to CI as best-effort witnesses (D-13).
- Test counts for the Wave 0 baseline: `#[test]` in xtask: lib 2, config 22, pcap 53, main 26, uregcheck 11 [VERIFIED: grep -c].

## Owner constraints that shape tasks
- **Push approval** (both repos). Currently unpushed: r9 `5b8b70a`, `6ee45b5`, the GSD bootstrap commits; Harvey `25d97dc4d` [CITED: STATE.md Blockers]. An r9 commit that locks Harvey to an unpushed rev cannot be fetched by anyone else and breaks CI.
- **Any Harvey commit needs owner approval**; this research and execution must treat `/Users/neilskinner/code/harvey` as read-only until then. Never stage the 11 modified troff/postscript font files there (they are case-collision artifacts of this very Mac) [VERIFIED: `git status` shows them modified].
- **`port/src` edits need owner approval with the diff shown.** None are expected; if the assembler or gate needs a kernel-visible change (for example a boot-config key) it lands in `x86_64/` or `xtask/` instead.
- **x86_64 kernel bytes:** none should change. If one does: re-derive and commit the `boot-iso` `outputHash`, run x64 TCG gates in an amd64 Docker container (cleaned up afterwards; Docker 29.6.1 is present on this Mac), and get the owner's Linux `boot-iso` check [CITED: STATE.md Standing Owner Rules].
- **aarch64 kernel bytes:** quick pass now; full HVF series deferred to Phase 8.
- **Verification that cannot run on this aarch64 Mac:**
  - the required witness itself (x86_64 KVM; `/dev/kvm` does not exist on macOS);
  - `nix build .#boot-iso --rebuild` and `kernel-x64` (`x86_64-linux` only; `boot-iso` exists only for `system == "x86_64-linux"` [VERIFIED: flake.nix:499]);
  - the drawterm gate and the rio gate (`drawterm` package is Linux-only, Xvfb is Linux-side; grub2/xorriso are in the devshell on Linux only so the Mac cannot compose the ISO [VERIFIED: flake.nix:866]);
  - tap/bridge net-soak and stress (owner's NixOS host setup);
  - hosted-CI TCG gates (CI runs them, not the Mac).
  - What the Mac *can* do: resolver and manifest unit tests, `nix flake lock/eval/show`, aarch64 HVF and TCG gates (needs the Linux builder for the arm64 tree), x86_64 TCG boots only if the ISO can be composed (it cannot without Linux grub), kernel ELF byte comparison via `cargo xtask build`, differential `.paq` byte tests *if* the reference bake can run (needs a Linux builder).
- **`gh` is not installed**: `nix run nixpkgs#gh -- …`. macOS socket path limit 104 bytes: run boot tests from a short path. r9 exec caps argv entries at 128 bytes; PL011 buffer 256 bytes (keep typed serial lines short on aarch64 HVF).
- **Commit messages: no AI attribution** (user CLAUDE.md, permanent, overrides the harness trailer instruction).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Harvey flake composes r9 (`inputs.r9`) | r9 flake imports Harvey and owns composition | This phase (D-02) | Gate entry points, CI, locks move together |
| Harvey checkout via `R9_HARVEY_DIR`, produced by hand | Store-path tree supplied by devshell | This phase | Tree read-only by construction |
| Harvey `mkimage.sh` bakes finished `.paq` | r9 xtask assembles from manifest | This phase (D-16) | Per-file Harvey/Rust swap becomes possible |
| `-G` text-only drawterm gate | X11 drawterm under Xvfb + serial `/srv` check | This phase (D-11) | First automated rio assertion |

**Deprecated/outdated for this phase:** `../harvey` default path (removed earlier, D-10 of the old phase); `vendor/harvey/` as an implicit location (does not exist at HEAD).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `unshare -Ur -m` + tmpfs chroot works inside a nix build sandbox on the Linux witness and on the builder VM | Pitfall 2 | D-03 blocked; fall back to bwrap/proot/vmTools or patching mkfiles (large) |
| A2 | The Determinate native builder (or lima NAR transport) preserves Harvey's case-colliding names end to end | Pitfall 3 | Mac builds produce wrong or failing trees; need builder-side fetch or case-sensitive volume |
| A3 | aarch64-linux builder is the right system for Mac builds (tree content is host-independent) | D-04 | If the amd64 tier build only works on x86_64-linux, the Mac needs Rosetta/x86 builder |
| A4 | amd64 `subset-build`/`stage-build` run on an aarch64-linux host | Pitfall 4 | As A3 |
| A5 | `/srv` is global in r9 and visible from the serial console; rio's srv entries appear there | D-11 | Marker design changes (second drawterm session or Xvfb pixel check) |
| A6 | Harvey's drawterm X11 backend runs under Xvfb in the gate (the owner's accepted leg used a real display) | D-11 | No automated rio proof; weaker fallback |
| A7 | `lld --oformat=binary` yields flat text at 0x200028 acceptable to r9's loader | Probe | Need a small ELF-segment wrapper instead (still small) |
| A8 | Tier-only build time is tens of minutes per arch | D-03 | Gate/CI timeouts, builder sizing |
| A9 | `builtins.unsafeDiscardStringContext` on the output path gives lazy devshell supply and a correct path | Pattern 1 | Devshell builds eagerly or path mismatches; fall back to a wrapper command |
| A10 | `lib.fileset` filtering of the Harvey input prevents rebuilds on unrelated pin bumps | Standard Stack | Every pin bump pays a full tree rebuild |
| A11 | Harvey's own `k10cpu` can be launched under q35 and reach a usable cpu service | D-08 | D-08 deliverable shrinks to "launches and prints a banner" |
| A12 | `mkimage.sh` byte-determinism at the pin (stated in its header) holds, so differential tests can byte-compare | Pattern 3 | Compare file lists/sizes/hashes of members instead |
| A13 | `inputs.harvey.inputs.nixpkgs.follows = "nixpkgs"` builds `drawterm`/`harness` | Pitfall 8 | Keep Harvey's own nixpkgs (extra lock nodes only) |
| A14 | HARN-02's "reconcile `HARVEY_SRC` and `vendor/harvey/`" means explicit env only, no implicit probe, given D-01 | Resolver | Owner may want a documented probe order; small change |
| A15 | Owner accepts per-arch variables (`R9_HARVEY_DIR_AMD64/ARM64`) as satisfying HARN-07 | D-05 / HARN-07 | Requirement wording needs amendment or a combined layout |
| A16 | Hosted CI runners allow userns for the tree derivation (Ubuntu 24.04 AppArmor restriction) | CI | CI builds the tree via a different venue or skips it best-effort (D-13 allows best-effort) |

## Open Questions

1. **Does the namespace build run inside a nix sandbox (S1) and through the Mac builder (S2)?**
   - Known: the unprivileged venue works on the owner's NixOS host outside the sandbox; the sandbox case is untested; the Mac builder is unconfigured and logged out.
   - Recommendation: Wave 0 spike S1 = a `runCommand` that does the `unshare -Ur -m` + tmpfs + chroot + a trivial `mk` against a copy of the real Harvey source, run on the Linux witness, then on the Mac builder (S2 uses the real Harvey input to exercise the case collisions). Owner decides the Mac builder mechanism after S2.
2. **Own-kernel launch scope (S5).** What does "build and launch Harvey in q35 qemu" have to show: banner, root, or a drawterm-attachable cpu service? Recommendation: ask the owner; default to banner plus prompt reached, cpu service as stretch.
3. **Are libraries in the manifest?** HARN-03 says "C library object", D-15 says "each binary and library object" in the *image*; images contain no `.a` files. Recommendation: image manifest lists binaries and baked files; add a second table of tier-0 libs (`tiers/tier0-libs.txt`) marked `in_image = false` so the HARN-03 literal reading is met and Phase 9-12 linking work has the list.
4. **Where do gate fixtures live after the flip?** (`*-sweep.rc`, `*-termrc`, `root-tree`, tier data, fixture `.c`.) Recommendation: stay in Harvey, read via the tree; confirm with the owner because it determines the Harvey diff.
5. **Reference bake for differential tests** needs a Linux host with Harvey's `mkimage.sh` runnable on a writable copy of the tree. Acceptable to run it only on the Linux witness and record hashes as committed fixtures.
6. **Per-commit gate runtime.** KVM boot is seconds; the first tree build is tens of minutes. Recommendation: quick tier never builds the tree implicitly in CI timeouts; it fails with the exact `nix build` command printed.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| nix (Determinate) | everything | ✓ | nix 2.34.6, Determinate 3.21.0 (3.24.3 available) | — |
| Linux builder (native or lima) | tree derivations on the Mac | ✗ configured | `external-builders = []`, no `/etc/nix/machines`, FlakeHub logged out | `determinate-nixd auth login`, or lima remote builder |
| lima | lima-builder option | partial | lima 2.2.0 in store, `limactl` not on PATH, VM `harvey` on disk (state unknown) | native builder |
| nix-darwin `nix.linux-builder` | D-04 option 1 | ✗ | determinate module in use | n/a |
| cargo / rustc | xtask tests | ✗ outside devshell | — | `nix develop -c …` |
| qemu | gates | ✗ outside devshell | in devshell | `nix develop` |
| Docker | owner's kernel-byte TCG procedure | ✓ binary | 29.6.1 (daemon not probed) | — |
| `/dev/kvm` | required witness | ✗ (macOS) | — | local x86_64 Linux machine (owner) |
| expect, xvfb, drawterm in r9 devshell | gate | ✗ today | r9 devshell has only toolchain, qemu, dtc (+grub2, xorriso on Linux) [VERIFIED: flake.nix:865-866] | add to the Linux devshell |
| Hosted-CI userns | tree derivation in CI | unknown | — | best-effort per D-13 |
**Missing with no fallback:** a Linux x86_64 KVM host is the owner's, not provisionable from here.
**Missing with fallback:** Mac Linux builder (login or lima).

## Validation Architecture

> `.planning/config.json` does not exist in this repo, so `workflow.nyquist_validation` is absent and treated as enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in tests via `cargo xtask test` (host); expect/shell gates for boot behaviour |
| Config file | none (workspace `Cargo.toml`; xtask is a std crate) |
| Quick run command | `nix develop -c cargo test -p xtask` [ASSUMED crate-level invocation; `cargo xtask test` is the documented entry] |
| Full suite command | `nix develop -c cargo xtask test` then `nix develop -c cargo xtask clippy` and `check` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HARN-01 | quick gate exits 0 only when `/srv/rio.*` and `/srv/riowctl.*` seen; non-zero when rio absent (negative control: boot the cpu image with the profile's rio guard forced off, or `R9_GATE_NO_RIO=1` test hook) | integration (KVM, Linux only) | `nix develop -c cargo xtask gate --arch x86-64` | ❌ Wave 0 |
| HARN-02 | resolver precedence, named error on unset, arch variables; consumers use it (grep gate: no `env::var("R9_HARVEY_DIR")` outside `harvey.rs`) | unit | `cargo test -p xtask harvey::` and `! grep -rn 'R9_HARVEY_DIR' xtask/src --include=*.rs \| grep -v harvey.rs` | ❌ Wave 0 |
| HARN-02 | tree unmodified after a gate run | integration | `nix store verify --no-trust "$R9_HARVEY_DIR_AMD64"` before and after, exit 0 | ❌ Wave 0 |
| HARN-03 | manifest parses; every staged file classified; tier-0 libs table matches `tiers/tier0-libs.txt` | unit | `cargo test -p xtask image::manifest` | ❌ Wave 0 |
| HARN-04 | flip probe entry to `rust`, stage, assert only that member changes, gate green; flip back restores identical staging | unit + integration | `cargo test -p xtask image::swap` and `cargo xtask gate --arch x86-64 --manifest-override probe` | ❌ Wave 0 |
| HARN-05 | stray file, missing file, Rust entry pointing into the tree each fail with a named error | unit (negative controls) | `cargo test -p xtask image::drift` | ❌ Wave 0 |
| HARN-06 | existing profiles pass via resolver and assembler | integration | `cargo xtask boot-test --arch x86-64 --config q35 --accel tcg --cpu qemu64` (3 CI profiles, any host); `q35shell`, `q35cpu` etc. on Linux; aarch64 quick pass on the Mac per HANDOFF counts | ✅ profiles exist; images ❌ |
| HARN-07 | inside `nix develop` with nothing set, `cargo xtask ureg-check` and `gate` find the tree; outside, error names the variable | integration | `nix develop -c cargo xtask ureg-check` and `env -u R9_HARVEY_DIR_AMD64 -u R9_HARVEY_DIR cargo xtask ureg-check` expecting the named error | ❌ Wave 0 |
| HARN-08 | each moved app builds and passes from r9; Harvey flake has no `r9` input; no lock cycle | integration + static | `nix flake show` in both repos; `nix eval --raw .#apps.x86_64-linux.drawterm-gate-x64.program`; `jq '.nodes.root.inputs' flake.lock` in Harvey shows no `r9`; run each app on Linux/Mac per the venue table | ❌ Wave 0 |
| D-17/D-18 | probe replaces `echo`, marker appears, flipped back, probe code and manifest entry removed | integration | gate with probe manifest, then `git grep -n probe xtask` returns nothing after removal | ❌ Wave 0 |
| (invariant) | no kernel bytes changed by any commit | hash compare | `cargo xtask build --arch x86-64 --config q35churnshell` + `sha256` of the ELF before/after (Mac ok); Linux: `nix build .#kernel-x64` | ✅ tool exists |
| (equivalence) | moved gate results equal Harvey-hosted run | log diff | run Harvey-hosted app (pinned `bd7a698`) and r9 app; compare marker lines (`ALL STAGES PASS`, stage names, FAIL counts) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p xtask` (resolver/manifest units; Mac ok).
- **Per wave merge:** `cargo xtask test`, `clippy`, `check`; quick gate on the Linux witness once the gate exists.
- **Phase gate:** full set of moved gates on Linux KVM (green twice, captures retained) + aarch64 quick pass on the Mac + CI best-effort, then `/gsd-verify-work`.

### Wave 0 Gaps (spikes and fixtures)
- [ ] **S1** userns+chroot venue inside a nix sandbox (Linux witness, then builder). Pass = a derivation builds `libc` for amd64 from Harvey source with no sudo.
- [ ] **S2** Mac builder path with the real Harvey input and the 23 colliding paths; pass = derivation sees both `rc/bin/kill` and `rc/bin/Kill`, output copies back intact.
- [ ] **S3** headless rio proof on KVM: Xvfb + X11 drawterm + serial `ls /srv`; also checks `mkimage.sh` determinism claim (A12).
- [ ] **S4** amd64 tier build on an aarch64-linux host.
- [ ] **S5** Harvey own-kernel q35 launch feasibility (read-only against Harvey; produce a recipe, do not edit Harvey).
- [ ] **S6** probe: `lld --oformat=binary` + 40-byte header wrapper boots and prints under the r9 loader.
- [ ] `xtask/src/harvey.rs` tests, `xtask/src/image.rs` tests, `xtask/manifest/*` fixtures, a reference-bake hash fixture set.

## Security Domain

> `.planning/config.json` absent; `security_enforcement` treated as enabled.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | marginal | Gate creds come from the bake-time DEMO p9sk1 domain; never print the password, redact captures (existing driver does) |
| V3 Session Management | no | — |
| V4 Access Control | yes | Tree read-only by nix store immutability; xtask opens tree files read-only, writes only under `target/` |
| V5 Input Validation | yes | Manifest paths: reject absolute paths, `..`, and symlink escapes out of the tree/stage dir; validate manifest keys with serde `deny_unknown_fields` |
| V6 Cryptography | no | Not hand-rolled; hashing via tool or audited crate |
| V14 Config / supply chain | yes | Harvey and nixpkgs pinned by `flake.lock` narHash; filtered `src`; bump = deliberate commit (D-06) |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via manifest `artifact` | Tampering | Canonicalize and require prefix under tree or `target/` |
| Writing into the Harvey tree (pollution/tampering) | Tampering | Store-path read-only; `nix store verify` after gate; scripts take an explicit scratch dir |
| Shell injection when scripts are rewired | Tampering | Quote all expansions; keep `writeShellApplication` shellcheck gate |
| Stale/pre-seeded capture treated as evidence | Repudiation | Existing rm-before-launch rule retained |
| Survivor processes cross-killing sibling gates | DoS | Keep per-gate unique `R9_RUN_TAG`/`R9_DT_TAG` sweeps; add Xvfb to the sweep |
| Hostfwd ports exposed to LAN | Info disclosure | Keep loopback-only `127.0.0.1` binds (pinned TOML) |

## Sources

### Primary (HIGH confidence)
- Read this session: r9 `flake.nix`, Harvey `flake.nix`, `x86_64/src/exec.rs:590-740`, Harvey `sys/src/libc/9syscall/sys.h`, r9 `.planning` CONTEXT/REQUIREMENTS/STATE/HANDOFF.
- Executed probes: `nix flake lock` scratch probe (Harvey `bd7a698` imported by an r9 copy); `nix flake prefetch` of Harvey and listing of case-hacked names; `nix show-config`; `nix eval` of `xvfb`, `xvfb-run`, `plan9port` and platforms; `determinate-nixd status`; `git ls-files` case-fold scan; `du` of Harvey build outputs; directory listings.
- Harvey source files read via shell (verbatim): `nix/*.sh`, `build/{x86_64,aarch64}/{stage-build,subset-build,mkimage,mkpaqfs-host,nsrun}.sh`, `sys/src/BUILD*`, `RUN_BUILD_IN_PRIVATE_NAMESPACE`, `build/mkdirs`, `amd64/ABI.md`, `build/REPRODUCIBILITY.md`, `sys/src/cmd/rio/fsys.c`, drawterm `gui-none/screen.c` and `cpu-bl.c`, `sys/src/9k/k10/k10cpu`.

### Secondary (MEDIUM confidence)
- docs.determinate.systems linux-builder and native-linux-builder troubleshooting pages (fetched).
- Search results on nix-darwin determinate module interplay (docs.determinate.systems/guides/nix-darwin/, Determinate 3.8.4 announcement, changelog 3.15.2).

### Tertiary (LOW confidence)
- Nothing relied on from web search alone except the nix-darwin/linux-builder incompatibility statement, which is consistent with the `nix.custom.conf` header read locally.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for existing tools, MEDIUM for Xvfb-based rio proof (A5, A6).
- Architecture: MEDIUM. Commit order and lock behaviour verified; sandbox venue (A1) and Mac builder (A2) untested.
- Pitfalls: HIGH for case collision, builder/login, `src = ./.`, `-G` no-rio; MEDIUM for sandbox namespace risks.
- Cost: LOW-MEDIUM. D-16 breadth and D-08 launch are the dominant uncertainties.

**Flags raised against locked decisions (evidence above):** F1 D-16 is the largest item (ten image contracts, auth bake); F2 D-08 own-kernel launch is absent and unproven; F3 case-colliding source on the Mac store; F4 D-04's nix-darwin option is unavailable here and the native builder needs a FlakeHub login; F5 D-05 laziness conflicts with devshell semantics unless the path is exported context-free; F6 D-07's Harvey diff also includes `r9-shell.yml`/`phase6.yml` legs and `host-echo.c`, and "copy" really means rewire (scripts write into the checkout); F7 HARN-07's single `R9_HARVEY_DIR` versus D-05's two trees; F8 D-11 needs Xvfb plus an X11 drawterm because `-G` has no draw device and `gui-none` is deliberately screenless; F9 D-13/D-14: the required witness and most gates cannot run on the Mac; the aarch64 quick pass is the Mac-runnable leg.

**Research date:** 2026-11-22
**Valid until:** 2026-12-20 (Determinate and nixpkgs move fast; Harvey/r9 pins are fixed at `bd7a698` / `116bcc2` unless the owner pushes)
