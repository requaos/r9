---
last_mapped_commit: bdcf45a51dced761eec09538b5f0786ff81e0399
last_mapped_at: 2026-11-13
---
# External Integrations

**Analysis Date:** 2026-11-13

## APIs & External Services

**Sibling kernel repo (THE central integration):**
- **r9** (`github:requaos/r9`, writable sibling at `../r9`) — the Rust Plan 9 kernel Harvey boots on. Consumed exclusively as a Nix flake input pinned in `flake.lock` (rev `8f3ff11`, the 17-07 fix pin) — never a path/submodule dependency.
  - SDK/Client: `inputs.r9` in `flake.nix`; store-path contract `r9.packages.<system>.{kernel-x64, kernel-virt, iso-builder, netdev-args}`
  - Consumed by: `nix/boot-r9-x64.sh` (`R9_KERNEL_X64_PKG` → `kernel-x64.elf`, the q35shell-config kernel; `R9_ISO_BUILDER` → `bin/r9-iso-builder`), `nix/boot-r9.sh` (aarch64, frozen), all x64 gate apps in `flake.nix`
  - Dev-only override: `nix run .#boot-r9-x64 --override-input r9 git+file:///path/to/r9`
  - hasAttr gating in `flake.nix`: x64 apps exist only where r9 exports `kernel-x64`+`iso-builder`+`netdev-args`; aarch64 apps where `kernel-virt` exists — never widen these lists ahead of a pin advance
- **r9 netdev emitter** — `r9-netdev-args` bin (`R9_NETDEV_EMITTER`): the ONE source of qemu netdev/device argv for modes `slirp|tap|vmnet`; data comes from committed TOML (`../r9/x86_64/lib/config_q35shell.toml` `[qemu.netdev]`). No Harvey script assembles netdev text (`nix/boot-r9-x64.sh` word-splits its output).
- **r9 iso-builder** — `r9-iso-builder KERNEL-ELF OUTDIR [MODULE PATH NAME ...]`: GRUB 2.14 `grub-mkrescue` + xorriso 1.5.8.pl02 pipeline with pinned dates (`SOURCE_DATE_EPOCH`, `R9_ISO_DATE`, `R9_ISO_VOLID`); strip-ALL kernel staging; modules staged at `/boot/<NAME>` (bootfs.paq rides as an mb2 tag-3 module). One implementation, three doors (r9 FOD, r9 lib, Harvey compose).

**Virtualization:**
- **qemu 11.1.2** (`qemu-system-x86_64`) — q35 machine, KVM default (`-accel kvm -cpu host`), TCG explicit fallback; net modes: `-netdev user` slirp (hermetic default), `-netdev tap,br=br0,helper=/run/wrappers/bin/qemu-bridge-helper` (linux L2-truth venue), `-netdev vmnet-socket` (darwin manual venue). Drift notes + pins: `build/x86_64/README.md`.
- **minimega 3.3.0** (Sandia; `nix/minimega.nix`, fetched from `github.com/sandia-minimega/minimega` tag 3.3.0, built by nix as a Go module) — KVM VM orchestration for amd64 test VMs; `.mm` scripts under `build/mm/` (`boot-9k10cpu.mm`). Linux-only, x86/KVM-centric. Reference clone `../minimega` is read-only.
- **lima** (darwin only; `nix/lima-harvey.yaml`) — Ubuntu 24.04 VM hosting the namespace build on macOS; provisions itself via `install.determinate.systems` nix installer script.

**Host toolchain / donor sources (vendored or nix-fetched):**
- **plan9port** (`github.com/9fans/plan9port`, via nixpkgs) — `mk`, `rc`, `9c`, `sed`, `yacc`, `lex`, `awk` on the host
- **9front** (MIT) — aarch64 toolchain donor vendored in-tree at pin `350266104f` (`sys/src/cmd/{7a,7c,7l}/`, `sys/src/libc/aarch64/`); also the aarch64-local donor kernel venue (`build/aarch64/donor-*.sh`)
- **9-cc** (`0intro/9-cc`) — `iar` subset vendored at `sys/src/cmd/unix/9cc/` (host `ar` for Plan 9 archives)
- **9legacy ISO** (legacy, not CI): `build/build.bash` downloads `9legacy.iso.bz2` from `github.com/Harvey-OS/harvey/releases` to bootstrap the retired in-VM build (still reachable via `nix run .#iso`; no longer invoked by CI)

**Host-side network peers (gates):**
- `host-echo` (`nix/host-echo.c`, flake package `hostEchoX64Pkg`) — poll()-concurrent TCP echo peer for the tap soak; runs as the gate's own child, never a daemon
- in-guest peers: `tcpecho.c`, `tcpblast.c`, `netblast.c`, `svctcpecho.c` (`build/x86_64/`), ip-container binaries (`ipconfig`, `ping`, `udpecho` from the tier build)

## Data Storage

**Databases:**
- None. All persistent state is files: paqfs archives (`build/x86_64/out/bootfs*.paq` — magic `529ab12b`, trailer `6b46e688`), the embedded devroot of 9k kernels, and committed data files (tier allowlists, ledgers, manifests).

**File Storage:**
- Git-tracked source trees + Nix store (read-only store paths for kernels/builders). Build outputs land in gitignored `build/x86_64/{out,smoke-out}/`, `.build/`, `amd64/bin/`, `../r9/target/`.

**Caching:**
- Nix binary cache (implicit). No application-level cache. Fossil/Venti exist in-tree (`sys/src/cmd/fossil`, `venti`) but are compile-gated only — runtime-unverifiable until r9 gains a block device (future milestone).

## Authentication & Identity

**Auth Provider:**
- Custom — Plan 9 **p9sk1** (in-guest auth domain). No OAuth/LDAP/external IdP anywhere.
  - Libraries: `sys/src/libauth/`, `sys/src/libauthsrv/` (p9sk1, p9cr, chap, apop protocols); commands `sys/src/cmd/auth/` (authsrv, keyfs, factotum, changeuser…), `sys/src/cmd/auth/factotum/`
  - Current state (Phase 17 complete): bootfs images deliberately carry **NO `/boot/factotum`** — auth stub = absence; `authentication()` in `boot(8)` stays on the no-auth path (`build/x86_64/mkimage.sh` contracts)
  - **Phase 18 (next)**: in-guest p9sk1 domain — hash-paired bake-time key generator, `/dev/random` from x86_64 entropy + devcap `#¤`, keyfs/authsrv listening on **tcp/567**, loopback probe gate; **Phase 19**: cpu listener + drawterm + rio
  - slirp hostfwd already provisions the host entry points: `127.0.0.1:5670` → guest `567` (auth), `127.0.0.1:17010` → guest `17010` (cpu-listener drawterm target) — loopback-bound only, never `0.0.0.0` (the 5670 remap exists because unprivileged binds <1024 fail EPERM; `build/x86_64/README.md`)
  - Secrets: none committed — Phase 18's keys are generated at bake time; `auth/key`-class files never enter git

## Monitoring & Observability

**Error Tracking:**
- None (no Sentry-class service). Kernel panics print to the serial console; expect gates fast-fail on panic signatures mid-capture.

**Logs:**
- Serial-console captures per boot: `build/x86_64/*.log` (smoke/tier1/net/soak/churn/stress/services runs retained as evidence), `.build/*.log`; r9 boot-test logs + pcaps under `../r9/target/`
- CI failure evidence goes to `$GITHUB_STEP_SUMMARY` (log tails) + `actions/upload-artifact` (`.github/workflows/phase6.yml`)
- Kernel diagnostics: `print`/`iprint` to console with provenance lines (e.g. `timer: <Hz> (<source>), <mode>, armed`, `pciexbar: <old> -> <new>`); `-d int,guest_errors -D <file>` qemu logging for the stress instrument
- Provenance: every composed boot echoes its full qemu argv + store paths; `KERNDATE` embedded in paqfs headers; evidence files name their r9 SHA

**Debug tooling:**
- `harness gdbpoke9` (GDB poke/peek over the serial protocol, `build/harness/src/gdbpoke9.rs`), `harness trace-syscalls`, `srv9` (9P host server), `drive9`, `mkfs9`

## CI/CD & Deployment

**Hosting:**
- None — this is an OS; "deployment" = reproducible ISO composition + qemu boot from clean checkouts. GitHub repo runs CI only.

**CI Pipeline (GitHub Actions, ubuntu-latest, nix-based, linux leg only):**
- `.github/workflows/r9-shell.yml` — `boot-smoke` (`nix run .#boot-r9-smoke`, aarch64), `net-smoke` (`.#boot-r9-net-smoke`), `net-smoke-x64` (`R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64`). 90-min TCG timeouts.
- `.github/workflows/phase6.yml` — `wave0-host` (toolchain products, gethunk repro gates, coverage drift diff, generator scan, grep hygiene, venue pin, FPdbleword assert), `tier-r9` (`.#boot-r9-tier1`), `tier-2` (services-tier compile gate via `build/x86_64/subset-build.sh`), `tier-9k` (k10t1 venue kernel build + boot sweep), `x86_64-poolpanic` (9k10cpu boot verdict).
- `.github/workflows/harvey.yml` — base clean-checkout nix build gate (legacy 9legacy/ufs/expect bootstrap fully retired 2026-11-05).
- Supply-chain rules (all three): every action pinned to a full 40-char commit SHA (`actions/checkout` v4 = `11d5960a…`, `DeterminateSystems/nix-installer-action` v23 = `3138316d…`, `actions/upload-artifact` v4 = `ea165f8d…`); `permissions: contents: read`; no secrets/tokens; concurrency cancel groups.
- r9's own workflows live in `../r9/.github/workflows/` (boot-test-x86-64 runs the q35 profiles TCG).
- CI-green = linux only; darwin legs are documented-manual (`build/REPRODUCIBILITY.md`).

**Reproducibility contract:**
- Bit-reproducible kernels/ISOs: KERNDATE from git, pinned-date xorriso wrapper, strip-ALL + store-hash zero-zap of rustc anon symbols; FOD hashes re-wired via two-independent-derivation byte-identity proofs (`build/REPRODUCIBILITY.md`, r9 `boot-iso` FOD).

## Environment Configuration

**Required env vars (flakes/devshell set these; see STACK.md Configuration for the full list):**
- `HARVEY`, `PLAN9`, `HARVEY_LINUX_BIN`, `objtype`, `FORCERCFORMK`, `CGO_ENABLED`, `KERNDATE`
- x64 boot apps: `R9_KERNEL_X64_PKG`, `R9_ISO_BUILDER`, `R9_NETDEV_EMITTER`, (tap) `R9_NET_DOCTOR`; `R9_ACCEL`/`R9_CPU`/`R9_NET`; soak: `HOST_ECHO_X64`, `HARVEY_NETADDR_SEAM`

**Secrets location:**
- None exist. No `.env` files; auth material is bake-time generated (Phase 18). The only credentials-adjacent host config is the tap ACL `/etc/qemu/bridge.conf` (`allow br0` — exact per-bridge lines, `allow all` forbidden) and the setuid `qemu-bridge-helper` wrapper, owned by the host NixOS config (colmena-managed), not this repo.

## Webhooks & Callbacks

**Incoming:**
- None.

**Outgoing:**
- None. All service-style endpoints are in-guest or host-loopback:
  - guest `tcp/7` echo, `tcp/567` auth listener (Phase 18), `tcp/17010` cpu listener (Phase 19) — reachable from the host only via the slirp hostfwd pair on 127.0.0.1 (5670/17010)
  - tap venue: guest `192.168.122.15` ↔ host peer `192.168.122.1` (blastport 7777) on the NAT bridge `br0` — addresses live only in `build/x86_64/root-tree/lib/netaddr.rc` (the D-11 single seam)

---

*Integration audit: 2026-11-13*
