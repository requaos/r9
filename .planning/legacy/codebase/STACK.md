---
last_mapped_commit: bdcf45a51dced761eec09538b5f0786ff81e0399
last_mapped_at: 2026-11-13
---
# Technology Stack

**Analysis Date:** 2026-11-13

## Project Identity

Two-repo system, composed by Harvey's Nix flake:

- **Harvey** (this repo) — Plan 9 4th Edition derivative OS: C userspace + libs (`sys/src/`), mk(1) build, dual kernel trees (`sys/src/9/` legacy, `sys/src/9k/` amd64-active C kernel), and all host-side build/bake/gate machinery (`build/`, `nix/`).
- **r9** (sibling repo `../r9`, consumed ONLY as the flake input `github:requaos/r9` pinned in `flake.lock`) — Rust no_std reimplementation of the Plan 9 kernel; the adopted kernel for v2.1 (x86_64/q35) and frozen aarch64 (qemu virt).

Milestone v2.1 (since 2026-11-06): x86_64 pivot; Harvey amd64 userspace boots on r9 under `qemu -M q35 -accel kvm` to an interactive `% ` rc prompt via `nix run .#boot-r9-x64`. aarch64 is frozen at the v2.0 pin — do not "fix" aarch64 paths in passing.

## Languages

**Primary:**
- **C (Plan 9 dialect)** — `sys/src/9k/` (active amd64 C kernel: `port/` + `k10/`), `sys/src/9/` (legacy multi-arch kernel), `sys/src/libc/`, `sys/src/lib9p/`, `libbio/`, `libthread/`, `libdraw/`+`libmemdraw/`+`libmemlayer/`, `libsec/`, `libmp/`, `libauth/`+`libauthsrv/`, `libndb/`, `libip/`, and ~274 entries under `sys/src/cmd/` (from `cat.c` to `acme/`, `rio/`, `auth/`, `ip/`, and the compiler toolchains `6c/6a/6l`, `7c/7a/7l`, …). Style per `sys/man/6/style` (tabs, lowercase, `nil`, `exits`/`sysfatal`).
- **Rust (no_std, edition 2024)** — the r9 kernel, `../r9`: workspace members `port/` (arch-neutral kernel: proc/exec/syscalls/namec, devcons/devenv/devpipe/devsrv/devmnt/devproc/devroot/devether/devip, virtio transport, paqfs reader), `x86_64/` (v2.1 target: l.S boot, LAPIC/IOAPIC/ECAM, TSC-deadline timer, SYSCALL entry), `aarch64/` (frozen), `riscv64/` (vestigial), `xtask/` (build/qemu/boot-test/ureg-check/initrd/iso driver). Toolchain pinned by `../r9/rust-toolchain.toml`: **nightly-2026-01-30**, targets `aarch64-unknown-none`, `riscv64gc-unknown-none-elf`, `x86_64-unknown-none`; custom bare-metal link configs in `../r9/lib/*.json`; `rustfmt.toml` (edition 2024, `use_small_heuristics = "Max"`).
- **Rust (std, edition 2021)** — Harvey host harness `build/harness/` (one zero-dependency binary; subcommands `srv9`, `trace-syscalls`, `gdbpoke9`, `mkfs9`, `drive9` — see `build/harness/src/main.rs`). Built with stable nixpkgs Rust (NO rust-overlay, NO nightly) via `flake.nix` `packages.harness`.

**Secondary:**
- **rc shell** — in-guest startup and gate fixtures: `build/x86_64/root-tree/rc/bin/termrc` (skeleton), `build/x86_64/services-termrc`, `/lib/netip-*.rc` fixtures, `tier1-sweep.rc`, `services-sweep.rc`; mk recipe execution (`FORCERCFORMK=1`).
- **Assembly** — kernel/runtime per-arch (`sys/src/9k/k10/*.s`, `sys/src/libc/<arch>/*.s`, `../r9/x86_64/src/l.S`).
- **Yacc / Lex** — `sys/src/cmd/bc.y`, `units.y`, `sys/src/cmd/cc/cc.y`, `upas/smtp/*.y`, etc.
- **Bash / POSIX sh** — all host orchestration: `nix/*.sh` (flake app bodies), `build/x86_64/stage-build.sh`, `mkimage.sh`, `mkpaqfs-host.sh`, `subset-build.sh`, `build/mm/*.sh`, legacy `build/build.bash`.
- **expect (Tcl)** — every automated boot gate driver: `build/x86_64/{smoke,tier1,net,soak,churn,stress,services}-r9*.expect`, `build/aarch64/*.expect`.
- **Nix** — `flake.nix` (devshell + 15+ apps + packages), `nix/minimega.nix`.
- **C (host cc)** — `nix/host-echo.c` (tap-soak echo peer, built by `flake.nix` `hostEchoX64Pkg`).
- **Go** — NOT used to build Harvey. Only `nix/minimega.nix` builds Go code (upstream minimega via `buildGoModule`). The legacy Go-1.16.6/`ufs`/in-ISO-Go-programs flow (`sys/src/cmd/go/`) is retired from CI (`.github/workflows/harvey.yml` rewritten 2026-11-05).

## Runtime

**Environment:**
- Host: **NixOS linux-x86_64** primary (since 2026-11-06, `.planning/HOSTS.md`); darwin-aarch64 supported as documented-manual venue (namespace build inside a lima VM — `nix/lima-harvey.yaml`, Ubuntu 24.04 cloud images + Determinate nix installer); linux aarch64 supported. `x86_64-darwin` deliberately out of scope (`flake.nix` systems list).
- **plan9port** (nixpkgs `pkgs.plan9port`) provides host `mk`, `rc`, `9c`, `sed`, `yacc`, `lex`, `awk`, `date`. `$PLAN9/bin` is LAST in PATH (plan9port's 9p `mount` must not shadow util-linux mount — `BUILD_ON_UNIX`, `flake.nix` `mkScriptApp`).
- **qemu 11.1.2** (project pin): `qemu-system-x86_64 -M q35 -accel kvm -cpu host -smp 1 -m 1024M -cdrom <ISO> -nographic` is the composed boot shape (`nix/boot-r9-x64.sh`); TCG (`-accel tcg -cpu qemu64`) is an explicit fallback only — no silent downgrade (`R9_ACCEL`).
- Guest: Harvey amd64 binaries (Plan 9 a.out, `S_MAGIC 0x8A97`) exec'd by the r9 kernel through the frozen 184-byte 9k-verbatim Ureg ABI (`amd64/ABI.md`; aarch64 counterpart `aarch64/ABI.md`).

**Package Manager:**
- **Nix (flakes)** for the host environment. Lockfile `flake.lock` present: nixpkgs `nixos-unstable` (rev `1d06008`), `r9` = `github:requaos/r9` rev `8f3ff11` (the 17-07 fix pin; r9's own inlined lock pins nixpkgs rev `1c89083` + `oxalica/rust-overlay` rev `c85ccd3`).
- **cargo** for r9 (`../r9/Cargo.lock` committed — supply-chain record) and the harness (`build/harness/Cargo.lock`, zero deps).
- **mk(1)** for everything in-tree Plan 9 (`sys/src/mkfile`, per-dir `mkfile`, `</$objtype/mkfile` includes).

## Frameworks

**Core (in-tree C libraries, `sys/src/`):**
- `sys/src/libc/` (`port/`, `9sys/`, `9syscall/sys.h` syscall table, per-arch `amd64/`, `aarch64/`, …)
- `sys/src/lib9p/` (user-space 9P server framework, `sys/include/9p.h`), `libthread/`, `libbio/`, `libdraw/` family, `libsec/`+`libmp/`, `libauth/`+`libauthsrv/` (p9sk1), `libndb/`, `libip/`, `libventi/`/`liboventi/`, `libmach/`, `libregexp/`, `libflate/`, `libaml/`, `libString/`, `libavl/`, etc.
- C kernels: `sys/src/9k/` (active; `port/` portable + `k10/` amd64 + `386/`; configs `k10cpu`, `k10cpuf`, `k10f`, plus venue kernels `k10t1`, `k10draw` built by `build/mm/build-9k10t1.sh`, `build/mm/build-9kdraw.sh`), `sys/src/9/` (legacy).

**Kernel (Rust, `../r9`):**
- `port/` — Plan 9 kernel semantics: `namec.rs`/`pgrp.rs` (namespaces), `dat.rs`, `devcons.rs`…`devroot.rs`, `devmnt.rs` (9P client), `devip.rs`+`devip/` (smoltcp-backed /net), `devether.rs`+`netif.rs`, `virtio.rs` (from-scratch unsafe-free transport), `paqfs.rs` (miniz-inflate bootfs reader), `mcslock.rs`/`rendez.rs`, `allocator.rs`/`pagealloc.rs`, `fdt.rs` (DTB/mb2 tag parsing).
- `x86_64/` — `src/l.S` (mb2 header + long-mode entry), `lib/kernel.ld`, `lib/grub.cfg` (TEMPLATE with `@R9-MODULE@` markers), `lib/config_q35*.toml` (7 committed machine configs: q35, q35lowmcfg, q35net, q35exec, q35shell, q35churn, q35churnshell — platform/dev/netdev data, not Rust edits), `lib/q35-mcfg-moved.bin`.
- `xtask/` — `cargo xtask {build, expand, kasm, dist, test, clippy, boot-test, ureg-check, initrd, …}`; config-TOML-driven (`xtask/src/config.rs`); second bin `xtask/src/bin/r9-netdev-args.rs` (exported emitter; requires `default-run = "xtask"`).

**Testing/Gates (not a unit-test framework — expect-driven boot gates):**
- expect drivers + marker assertions: `build/x86_64/smoke-r9.expect` (echo round-trip, ls/ps/cat, DEL safety, init restart), `tier1-r9.expect` (computed marker set + float canary), `smoke-r9-net.expect`, `netsoak-r9.expect`, `churn-r9.expect`, `stress-r9.expect`, `services-r9.expect`; aarch64 twins under `build/aarch64/`.
- Rust host tests: `cargo test` in r9 `port/` (152+ pin suites, `*_tests.rs`) and `build/harness/` (`nix develop -c cargo test`).

**Build/Dev:**
- **Nix flake apps** (`flake.nix`): `setup` → `toolchain` → `build` | `iso`; r9 composes (aarch64, frozen-at-pin): `boot-r9`, `boot-r9-smoke`, `boot-r9-net-smoke`, `boot-r9-tier1`; x64 (v2.1 active): `boot-r9-x64`, `boot-r9-smoke-x64`, `boot-r9-tier1-x64`, `boot-r9-net-smoke-x64`, `boot-r9-services-x64`, `boot-r9-net-soak-x64`, `boot-r9-churn-x64`, `boot-r9-stress-x64`, `boot-r9-auth-x64` (the p9sk1 auth-domain gate: mkimage §14's bootfs-auth.paq + authdomain-gen invoked at bake), `net-doctor` (linux-only), `drawterm` (linux-only; the vendored drawterm client wrapper — scrubs `$cpu`/`$auth`/`$PASS`/`$USER`, pins the loopback `-O` spec, prints the encrypted-default connect line, headless runs the `-G` stdio line). Packages: `harness`, `minimega`, `drawterm` (linux-only; hermetic build of the committed `sys/src/cmd/unix/drawterm` source — no fetch, no patchPhase). `devShells.default` exports `HARVEY`, `PLAN9`, `HARVEY_LINUX_BIN`, `objtype=amd64`, `HARNESS_BIN`, PATH ordering `$HARVEY_LINUX_BIN:$PATH:$PLAN9/bin`.
- **Namespace build**: `nix/build.sh` (Linux native `sudo unshare`-based; darwin leg `nix/vm-build.sh` via lima) wrapping `sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE` → `BUILD` → `build/scripts/linux-build`.
- **Userspace staging/baking (x64)**: `build/x86_64/stage-build.sh` (marker + boot-chain binaries, `objtype=amd64`, S_MAGIC-verified) → `build/x86_64/mkimage.sh` (bakes `out/bootfs.paq`, `bootfs-chain.paq`, `bootfs-tier.paq`, `bootfs-net.paq`, `bootfs-services.paq` from `root-tree/` skeletons + `bootfs-payload.txt` manifest) using host `mkpaqfs` (`build/x86_64/mkpaqfs-host.sh`).
- **Tier machinery**: append-only allowlists as data — `build/x86_64/tiers/*.txt` (+ `build/aarch64/tiers/`), driven by `build/x86_64/subset-build.sh`; artifact+execution marker ledgers are the only gates (mk exit status never trusted).

## Key Dependencies

**Critical (r9 kernel crates, `../r9/*/Cargo.toml`):**
- `smoltcp` 0.14.0 (default-features=false; `medium-ethernet`, `medium-ip`, `proto-ipv4/6`, fragmentation, `socket-tcp/udp/icmp`, `socket-tcp-reno`, `alloc`, `multicast`, `auto-icmp-echo-reply`, `async`) — TCP/IP engine behind devip
- `miniz_oxide` 0.9 (`with-alloc`) + `adler2` 2 — paqfs DeflateEnc inflate
- `bitflags` 2.5; x86_64 crate: `x86` 0.52, `bitstruct` 0.1, `bit_field` 0.10.3, `seq-macro` 0.3.6, `zerocopy` 0.8.27, `static_assertions` 1.1.0; aarch64: `aarch64-cpu` 11.2, `num_enum` 0.7; xtask: `clap` 4.5, `serde` 1.0, `toml` 0.9, `target-lexicon` 0.13.2
- Lint contract: `undocumented_unsafe_blocks = deny` (workspace clippy) — every `unsafe` in `port/`+`aarch64/` carries a SAFETY comment

**Infrastructure (host, via nixpkgs in `flake.nix`):**
- `plan9port` (mk/rc/9c host tools), `qemu`, `expect`, `git`, `curl`/`bzip2`/`gnutar` (iso app), `dtc` (aarch64 DTB patch only), `grub2` 2.14 + `xorriso` 1.5.8.pl02 (r9 iso-builder), `util-linux`/`mount`/`gnused`/`binutils` (namespace build), `lima`+`openssh`+`rsync` (darwin), `iproute2`/`iptables`/`procps` (net gates + doctor), `rustPlatform` (harness), stable `cargo`/`rustc` (devshell)

**Vendored in-tree (no external fetch):**
- 9front aarch64 toolchain donor at pin `350266104f`: `sys/src/cmd/{7a,7c,7l}/`, `sys/src/libc/aarch64/`, libmach letter-7 files (per-dir provenance READMEs)
- 9-cc `iar` subset: `sys/src/cmd/unix/9cc/` — installed as the host `ar` writing Plan 9-format archives
- Harvey's own Plan 9 toolchain built from `sys/src/cmd/{6a,6c,6l,8a,8c,8l,cpp,…}` by `nix/toolchain.sh` (via `sys/src/BUILDTOOLCHAIN` + `sys/src/cmd/BUILD`)

**External fetches by Nix (never submodules/relative paths):**
- `github:requaos/r9` (flake input; packages consumed: `kernel-x64`, `kernel-virt`, `iso-builder`, `netdev-args`, `boot-iso` FOD)
- `sandia-minimega/minimega` tag 3.3.0 (`nix/minimega.nix`, `fetchFromGitHub`, linux-only, amd64 test VMs)
- legacy only: 9legacy ISO (`build/build.bash`, GitHub releases URL) for the retired `nix run .#iso` bootstrap path

## Compiler Toolchain (Plan 9 native, in `sys/src/cmd/`)

| Arch | Tools | Status |
|------|-------|--------|
| amd64 | `6a`/`6c`/`6l` | **active** (`objtype=amd64`; all v2.1 userland + 9k kernels) |
| 386 | `8a`/`8c`/`8l` | builds (host toolchain set); legacy target |
| aarch64 | `7a`/`7c`/`7l` | vendored 9front; **frozen at v2.0 pin** |
| arm/mips/power/riscv/sparc | `5c`, `4c`, `9c`, `kc`, `vc`… | in-tree, dormant |

Host driver: `pcc` (plan9port `9c` bootstrap, replaced by Harvey's own `sys/src/cmd/BUILD` output); host `ar` = vendored `iar`; host `nm` must be binutils' (shimmed — see `nix/toolchain.sh`, `nix/build.sh`).

## Configuration

**Environment (devshell exports + per-app contract):**
- `HARVEY` (repo root), `HARVEY_LINUX_BIN` (`.build/hostbin-<os>-<arch>/bin`), `PLAN9`, `objtype=amd64`, `cputype=unix`, `FORCERCFORMK=1`, `CGO_ENABLED=0`
- `KERNDATE` — pinned build timestamp from git commit (`nix/build.sh`, `nix/boot-r9-x64.sh`); bit-reproducibility key (`build/REPRODUCIBILITY.md`)
- Boot-app contract (`nix/boot-r9-x64.sh`): `R9_KERNEL_X64_PKG`, `R9_ISO_BUILDER`, `R9_NETDEV_EMITTER` (all required, flake-exported store paths), `R9_NET_DOCTOR` (tap only), `R9_ACCEL` (kvm default|tcg), `R9_CPU`, `R9_NET` (slirp default|tap|vmnet), `R9_SKIP_BUILD`, `R9_NETDEV_ARGV`, `HOST_ECHO_X64`, `HARVEY_NETADDR_SEAM`
- Guest addressing seam: `build/x86_64/root-tree/lib/netaddr.rc` (single source of venue addresses; bake-time grep enforces no literals elsewhere)

**Build config files:**
- `flake.nix`, `flake.lock`, `nix/*.sh` (app bodies), `nix/lima-harvey.yaml` (darwin VM)
- `sys/src/mkfile` + `sys/src/mkfile.proto` + `sys/src/$objtype/mkfile`; per-dir mkfiles (`mkone`/`mkmany`/`mksyslib` in `sys/src/cmd/`)
- r9 machine configs: `../r9/x86_64/lib/config_q35*.toml` (accel/cpu/smp/memory/netdev tables), `../r9/aarch64/platform/*/` (frozen)
- Gate data: `build/x86_64/tiers/*.txt`, `bootfs-payload.txt`, `tier*.ledger`, `dir-coverage.txt`

**No `.env` files** — configuration is entirely env vars + Nix + committed data files. No secrets anywhere in-tree (Phase 18's auth keys are bake-time generated).

## Platform Requirements

**Development:**
- Linux x86_64 with nix (flakes) + `/dev/kvm` (world-rw on the primary host) for KVM-native gates; `/dev/net/tun` for tap venue
- darwin-aarch64: nix + lima (namespace build in VM; `boot-r9-x64`'s unshare leg is linux-only — darwin is documented-manual for x64 boots)
- plan9port via nix; ~8 GiB lima VM on darwin

**Production/Guest:**
- Target machine: qemu q35 (SeaBIOS → GRUB2 → mb2 → r9; `-cdrom` ISO, never `-kernel`), 1024M, 1 vCPU, virtio-net-pci-non-transitional NIC
- Harvey-standalone 9k kernels (k10cpu family, k10t1, k10draw) boot under plain qemu/minimega on KVM hosts (`build/mm/`)
- aarch64 `qemu -M virt,gic-version=2/3` frozen at the v2.0 r9 pin (`boot-r9*` aarch64 apps stay byte-untouched)

---

*Stack analysis: 2026-11-13*
