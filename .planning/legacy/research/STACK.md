# Technology Stack

**Project:** Harvey OS v2.0 "Full System & Drawterm" — stack additions for new capabilities only
**Researched:** 2026-11-04
**Confidence:** HIGH for all in-tree facts (verified by direct read of Harvey + r9 sources); MEDIUM for web-sourced package/crate facts (websearch seam tier, cross-verified against docs.rs / raw nixpkgs + 9front sources). No LOW-tier claim is load-bearing.

## Executive Answer

Almost everything server-side **already exists in-tree**; the genuinely new code is confined to r9 (PCI/ECAM + virtio-net + a devip-compatible `/net`) and host-side plumbing (qemu netdev tiers + nixpkgs drawterm). The one real stack *decision*: **Harvey ships a kernel TCP/IP stack (`sys/src/9k/ip/`, ~15.4k lines C) plus userspace *clients* of it (`sys/src/cmd/ip/`) — there is no Harvey userspace TCP/IP stack to reuse**, so r9 implements `/net` semantics over **smoltcp** rather than porting devip C to Rust.

## Recommended Stack

### 1. Full aarch64 userspace — no new imports, build-scope change only

Everything compiles from what's already vendored (9front 7a/7c/7l pin `350266104f`, libc/aarch64, v1.0 green libs). The GUI stack needs **zero assembly work**: `libdraw`/`libmemdraw` ship portable `.c` next to stale committed `.6`/`.8` **binary objects** (verified: `alloc.6` is a compiled 6c object, not source), and `mksyslib`'s `%.$O: %.c` rule builds the portable C on aarch64 (`$O=7`, no `.7` files exist).

| Component | Status | Action |
|-----------|--------|--------|
| libc, libbio, libregexp, libndb, libString, libthread, lib9p, libip, libmp, libsec, libauth, libauthsrv | aarch64-green (v1.0) | none |
| libdraw, libmemdraw, libmemlayer, libframe, libcontrol, libplumb, libhtml, libhttpd, libflate, libdisk, libstdio, libaml, libscribble, libgeometry, libavl, libbin, libcomplete, libdynld, liboventi, libventi, libsunrpc, liblex, libmach | in-tree, **zero per-arch `.s`** (verified full sweep) | flip `build_aarch64` subset → full `LIBS`/`CMDS` in `build/scripts/build` + `linux-build`; rio needs libdraw+libframe+libthread+libplumb+libc (`cmd/rio/mkfile`) |
| ~262 cmd targets (132 top-level `.c`/`.y` + 130 dirs) | in-tree | build via existing `cmd/mkfile` DIRS loop; exclude `NOTSYS`/`BUGGERED` (`unix`, `old`) as today |
| **APE (`APEDIRS`: awk bzip2 compress cvs eqn grap gs lp pic postscript spin tex troff)** | **no aarch64 support** — `ape/lib/ap/` has 386/amd64/arm/mips*/power*/riscv*/sparc* but no arm64/aarch64 (verified) | **exclude from aarch64 scope** (see What NOT to Add) |
| Known upstream breakage | `cmd/jpg/mkfile` derives nonexistent `oc` and aborts the DIRS loop before `ip/`+`venti/` (PROJECT.md); `4a` rfork lex.c; mahjongg cpp | in-tree fixes (upstream dormant; this repo is the maintained fork) |

Runtime payload additions for the r9 initrd/paqfs: rio + fonts (`lib/font/`), `cmd/auth/*`, `cmd/ip/ipconfig`, `cmd/aux/{listen,listen1}`, `cmd/exportfs`, `cmd/ndb/{cs,dns}`, `lib/ndb/{local,common,auth}`, `rc/bin/service{,.auth}`, cpurc/profile templates.

### 2. r9 guest networking (the milestone's only greenfield kernel code)

| Technology | Version/Pin | Purpose | Why |
|------------|-------------|---------|-----|
| **virtio-drivers** (rcore-os) | crates.io `0.13.0` (docs.rs 2026-09-15) | virtio-net device driver + PCI/ECAM transport, no_std | Only maintained Rust no_std virtio stack. `VirtIONet<H,T,QUEUE_SIZE>` (alloc feature) pre-allocates rx buffers, exposes `mac_address`, IRQ ack/enable. PCI transport is **memory-mapped CAM/ECAM only — exactly qemu `-M virt` aarch64**. `transport::pci::bus` ships full enumeration: `PciRoot`, `MmioCam` (16MiB CAM/256MiB ECAM), `BusDeviceIterator`, `CapabilityIterator`, `BarInfo` — r9 does not write a PCI enumerator. Deps (bitflags, safe-mmio, thiserror, zerocopy, log) all no_std. Caveat: `VIRTIO_F_VERSION_1` negotiation marked TODO upstream — works against qemu modern devices in practice (rCore boots it on `-M virt`); keep the donor's `virtio-non-transitional` qemu habit and gate on a boot-test. |
| **smoltcp** | crates.io `0.14.0` (docs.rs 2026-09/10) | TCP/UDP/ICMP/IPv4 behind a devip-compatible `/net` | The standard bare-metal Rust TCP/IP. no_std, heap-free capable, MSRV stable 1.91 (r9's nightly-2026-01-30 clears it). `default-features=false`, enable `medium-ethernet`, `proto-ipv4`, `socket-tcp`, `socket-udp`, `socket-icmp`, `alloc` (r9 has a kernel heap: `port/globalalloc.rs`); `proto-dhcpv4`/`socket-dhcpv4` later for DHCP. Feature `compile_error!` guards force ≥1 socket-* + ≥1 medium-*. |
| Reference C (read-only) | in-tree `sys/src/9k/386/ethervirtio.c` (686 lines, legacy PCI); 9front `sys/src/9/port/ethervirtio10.c` (donor pin `350266104f`) | register-layout/ring-protocol references | Harvey's own is virtio-1.0-spec **legacy** register layout; 9front's `ethervirtio10` is the modern-capability model matching virtio-drivers' PCI transport. Read both; import neither. |
| Semantics reference | in-tree `sys/src/9k/ip/` (15,395 lines; `tcp.c` 3,486) | the file-interface contract for `/net` | Defines what Harvey userland expects: `/net/ipifc`, `/net/tcp|udp|icmp/{clone,N/{ctl,data,status,local,remote}}`, `/net/arp`, `/net/ndb`, `ethermedium` binding to `/net/ether0`. **Do not port this C** (see What NOT to Add) — implement its interface over smoltcp. |

**What r9 must implement** (none exists today — verified: no PCI/ECAM/virtio code in r9 kernel, only xtask qemu args):

1. **PCI/ECAM platform glue** (aarch64 virt): ECAM window at `0x4010000000` from the committed DTB, map device-nGnRE, `PciRoot::new(MmioCam::new(.., Cam::Ecam))`, enumerate vendor `0x1AF4`. GIC SPI routing per DTB `interrupt-map` (legacy SPIs; MSI optional).
2. **`Hal` impl** over r9 `pagealloc`/`kmem`: `dma_alloc/dealloc`, phys↔virt. qemu `-M virt` without SMMU is cache-coherent in practice; Hal's `flush`/`invalidate` hooks default no-op — document the assumption, add DSB/ISB barriers per `aarch64-cpu` where the spec requires.
3. **devether** (port crate, `devether.rs` after the `devpipe.rs` pattern): Plan 9 ether layout `/net/ether0/{addr,clone,stats,type,ifstats,N/{ctl,data,listen,local,status,type}}` backed by `VirtIONet`. Required because `ipconfig` opens `/net/ether0` by default (`cmd/ip/ipconfig/main.c:406`).
4. **devip** (port crate): devip-compatible device over smoltcp — `tcp/udp/icmp` clone/ctl/data files, `ipifc` ctl (`add`/`remove` static config first), `arp` thin shim (smoltcp's medium-ethernet does ARP internally), `/net/ndb` in-memory file. libc contract (verified in-tree): `dial.c` opens `/net/cs` if present else falls back to `/net/<proto>/clone` + `connect` ctl (numeric addrs need no cs); `announce.c`/`listen.c` drive `announce` ctl. **cs/dns stay userspace** (`cmd/ndb/{cs,dns}` over 9P via existing devmnt/devsrv) — r9 needs no kernel resolver.
5. xtask/flake: qemu already has the pattern — riscv64 path passes `-netdev type=user,id=net0 -device virtio-net-device`; aarch64 virt profile gains `-device virtio-net-pci` (PCI path per milestone) + tier-selected netdev (§3). Harvey `flake.lock` r9 pin must advance past the networking commits (pin currently `9549cee`; carried-over v1.0 item).

**Static-first IP config** (recommended for the milestone core): boot rc writes `add <ip> <mask> <gw>` to `/net/ipifc/0/ctl`; guest IP fixed per netdev tier (slirp `10.0.2.15`, socket_vmnet DHCP range, tap per-bridge). DHCP-via-ipconfig needs `/net/udp` broadcast + ether binding fidelity — **flag for phase research**, smoltcp's dhcpv4 socket is the fallback.

### 3. Host tap/bridge networking — three tiers, one privileged-setup doc per host OS

| Tier | Host | qemu args | Privilege | Use for |
|------|------|-----------|-----------|---------|
| 0 (default, CI) | any | `-netdev user,id=n0,hostfwd=tcp::17010-:17010` (+ `hostfwd=tcp::567-:567` if auth server enabled) | none (slirp) | smoke gates, hosted CI, first drawterm connect — p9sk1/9P ride TCP so L2 is not required for the core value |
| 1 (real L2) | darwin | `socket_vmnet_client /var/run/socket_vmnet qemu... -netdev socket,id=n0,fd=3 -device virtio-net-pci,netdev=n0` | one-time: root daemon (launchd) | macOS dev machines |
| 1 (real L2) | linux | `-netdev bridge,id=n0,br=br0,helper=<qemu-bridge-helper> -device virtio-net-pci,netdev=n0,mac=52:54:00:…` (unique MAC mandatory) | one-time: setuid helper + `/etc/qemu/bridge.conf` `allow br0` + `br0` exists | linux dev machines |
| 2 (optional) | darwin | `-netdev vmnet-bridged,id=n0[,ifname=en0]` | **sudo qemu** (nix qemu is unsigned; `com.apple.vm.networking` is a restricted Apple entitlement) | cross-machine drawterm |

Key facts (verified): qemu's builtin `vmnet-host/shared/bridged` netdevs (≥7.1, macOS 11+) **require root or the restricted entitlement** — nixpkgs qemu will never have it, so the unprivileged darwin path is **lima-vm/socket_vmnet v1.2.2** (Apache-2.0; daemon runs as root via launchd, qemu stays unprivileged, guest gets macOS bootpd DHCP and is host-reachable). **socket_vmnet is in nixpkgs** (`pkgs/by-name/so/socket-vmnet`, name `socket-vmnet`) and homebrew (keg-only). Linux: `qemu-bridge-helper` ships setuid-able in distro qemu packages (on NixOS wire via `security.wrappers`); the ACL file defaults deny-all. Lima's `lima-privileged-net` pattern (privileged `ip tuntap add user=$UID`, tap fd handed to unprivileged qemu) is the alternative if a setuid helper is unacceptable.

### 4. In-guest p9sk1 auth domain — 100% in-tree binaries, configuration work

All components exist in `sys/src/cmd/auth/` (verified mkfile TARG): `authsrv`, `keyfs`, `login`, `passwd`, `changeuser`, `newns`, `factotum/` (apop/chap/httpdigest/p9any/p9sk1…), `secstore/`. Deps (libauth, libauthsrv, libsec, libmp, libndb, libbio) are all aarch64-green. `keyfs.c` includes only libc/ctype/authsrv/fcall/bio/mp/libsec — no new lib work.

Wiring (all templates already in-tree):

| Piece | In-tree location | v2.0 action |
|-------|------------------|-------------|
| auth service scripts | `rc/bin/service.auth/{authsrv.tcp567,authsrv.il566}` | rename to enable (per 9p.io standalone-cpu-server guide) |
| cpurc template | `rc/bin/cpurc:39-51` commented `auth/keyfs -wp -m /mnt/keys /adm/keys`, `auth/cron`, `aux/listen -q -t /rc/bin/service.auth -d /rc/bin/service tcp` | enable for the guest cpu/auth profile |
| port names | `lib/ndb/common:255,291` — `tcp=ticket port=567`, `tcp=ncpu port=17010` | nothing |
| ndb entries | `lib/ndb/local` | add `authdom=<dom> auth=<guestip> sys=<name> ip=<guestip>` |
| speakfor db | `lib/ndb/auth` (commented template) | add `hostid=<hostowner>` + `uid=*` |
| key database | `/adm/keys` via `auth/keyfs` + `auth/changeuser` | seed script in initrd build (hostowner + user with p9sk1 password) |

**Sequencing shortcut (verified, 9p.io wiki "instant cpu server"):** the milestone can land drawterm login **before** the full auth server: `ip config; auth/factotum; echo 'key proto=p9sk1 dom=D user=U !password=P' >/mnt/factotum/ctl; aux/listen1 -t tcp!*!ncpu /bin/cpu -R &` — drawterm's `mkservertickets()` fallback (direct auth, `authid==hostid`) authenticates without any AS. Full keyfs+authsrv then lands as its own phase.

### 5. cpu server + drawterm protocol path — server in-tree, client from nixpkgs

Server side (verified in-tree): `rc/bin/service/tcp17010` is literally `#!/bin/cpu -R`; `aux/listen` scans `/bin/service`; `cmd/cpu.c` server (`-R`) does p9any→p9sk1 (`p9authproto="p9any"`, `ealgs="rc4_256 sha1"`), then `fversion` 64k, `mount(fd,-1,"/mnt/term",MCREATE|MREPL,"")`, opens `/mnt/term/dev/cons` as fd 0/1/2, execs `rc -li` with `service=cpu`. `sys/src/9k/root/profile:9-11` binds `/mnt/term/dev/{cons,consctl}` and `bind -a /mnt/term/dev /dev` — the imported `/dev/draw`, `/dev/mouse`, `/dev/kbd`, `/dev/snarf` appear automatically; rio opens `/dev/snarf` + draw/mouse from there. `cmd/exportfs/` exists (libc/fcall/auth/libsec only). **r9 implication: devmnt must mount 9P over a stream fd** (Tversion/Tattach on the socket) — same machinery as Phase 5's srv mounts, new transport; `pushssl`-equivalent lives in libsec userspace (`cpu.c` uses it), not the kernel.

Client side — **use the 9front fork via nixpkgs**, not 9fans and not Harvey's stale in-tree copy (`sys/src/cmd/unix/drawterm`, rsc vintage):

| Fact | Value (verified against raw nixpkgs `package.nix` + 9front drawterm `cpu.c`) |
|------|------|
| nixpkgs package | `pkgs.drawterm` = 9front fork, `fetchFrom9Front` rev `3aa7ce0c084089def7e721d7eb8ab091ab95318e` (`0-unstable-2026-10-12`), MIT, maintainer moody (9front dev), platforms incl. **aarch64-darwin, aarch64-linux, x86_64-linux** |
| darwin backend | `CONF=osx-cocoa`, `CC=clang` + lld → native Cocoa `drawterm.app` + `bin/drawterm` symlink. **No X11/XQuartz** |
| linux backends | default `CONF=unix` = X11 (libx11+libxt, padsp-wrapped); `pkgs.drawterm-wayland` = `CONF=linux` + wayland/libxkbcommon/libdecor/pipewire |
| p9sk1 retained? | **Yes** — `p9any()` accepts `p9sk1@dom` from the server's offer list; `gettickets()`→AS with `mkservertickets()` direct fallback; `ncpu()` dials service `ncpu`, speaks `p9 rc4_256 sha1` + pushssl — exactly Harvey's server |
| rcpu-first behavior | without `-O`, drawterm tries rcpu (dp9ik/TLS) first, **falls back to ncpu on dial failure**; recommend `-O` to skip the dead attempt |
| invocation | `drawterm -O -h <guest> -a <guest> -u <user>` (or `$cpu`/`$auth` env) |

Flake integration: `apps.drawterm` wraps `pkgs.drawterm` (per-system: cocoa on darwin, X11 on linux; expose `drawterm-wayland` as a second app), so `nix run .#drawterm -- -O -h 10.0.2.15 -a 10.0.2.15` works from a clean checkout. No custom derivation needed.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| r9 TCP/IP | smoltcp 0.14 behind devip-compatible `/net` | Port `sys/src/9k/ip` (15.4k lines C) to Rust | Deep coupling to C-kernel internals (Queue/Block/Fs/Rendez/ipifc tables); a rewrite, not a port — months, and discards a maintained stack |
| r9 TCP/IP | smoltcp | Port lwip/embedded C stack as Harvey userspace 9P server | No Harvey userspace TCP stack exists to start from; C port + new server framework > smoltcp in-kernel; splits the stack across the syscall boundary |
| virtio transport | PCI/ECAM via virtio-net-pci (milestone text) | virtio-mmio (`virtio-net-device`, DTB nodes at 0xa000000+) | MMIO is simpler (no PCI enum) and supported by the same crate — keep as **fallback** if ECAM bring-up slips; PCI matches the milestone, the DTB's existing ECAM window, and Harvey's PCI-based C reference |
| drawterm source | nixpkgs `pkgs.drawterm` (9front fork) | 9fans/drawterm (rsc) | Unmaintained by comparison (164 vs 607 commits), no wayland backend, gui-osx instead of maintained gui-cocoa; nixpkgs doesn't package it |
| drawterm source | nixpkgs | Harvey in-tree `sys/src/cmd/unix/drawterm` | Stale ("sometimes out of date" per its own README); would need bespoke flake build for cocoa; zero upside |
| darwin L2 | socket_vmnet daemon + `-netdev socket,fd=3` | qemu builtin `-netdev vmnet-shared/host/bridged` | Requires sudo qemu or a restricted Apple entitlement nix builds can't carry — fine as optional tier 2, unacceptable as the default |
| darwin L2 | socket_vmnet | tuntaposx / vde_vmnet | Dead kext approach / superseded by socket_vmnet (its own successor) |
| guest address | static ipifc ctl (milestone core) | DHCP via `ip/ipconfig` | Needs `/net/udp` broadcast + ether-medium fidelity; defer to a follow-up with smoltcp dhcpv4 as fallback |
| auth | p9sk1 (Harvey native) + 9front drawterm `-O`/fallback | dp9ik/rcpu/TLS server-side | Harvey's authsrv is p9sk1; TLS modernization is explicitly Out of Scope |

## What NOT to Add

| Avoid | Why |
|-------|-----|
| APE aarch64 port (`ape/lib/ap` has no arm64 dir) | POSIX shim layer; nothing in the rio/drawterm path needs it. Gate `APEDIRS` out of aarch64 builds and say so in the phase plan |
| Framebuffer/virtio-gpu/virtio-input in r9 | Milestone design: `/dev/draw` + `/dev/mouse` arrive via drawterm over 9P. Adding GPU drivers violates the core constraint and doubles r9 scope |
| dp9ik/TLS/rcpu server-side | Out of Scope (TLS hardening is a separate milestone) |
| Go userland | Unsupported on plan9/arm64 (PROJECT.md) |
| 9k C-kernel aarch64 port | Superseded by r9 adoption (PROJECT.md) |
| e1000/other NIC models | virtio-net only; one device class, one driver |
| Homebrew/MacPorts drawterm or socket_vmnet as *required* deps | nixpkgs covers both; brew path may be documented as convenience only |
| `.7` binary objects committed for libdraw/libmemdraw | The committed `.6`/`.8` are stale binary artifacts; aarch64 must compile the portable `.c` — don't cargo-cult the artifact pattern |

## Installation / Wiring

```bash
# r9 (sibling repo) — aarch64 or port Cargo.toml:
virtio-drivers = { version = "0.13", default-features = false, features = ["alloc"] }
smoltcp = { version = "0.14", default-features = false,
            features = ["medium-ethernet", "proto-ipv4", "socket-tcp",
                        "socket-udp", "socket-icmp", "alloc"] }
# r9 flake: cargo vendor hash refresh; then Harvey flake.lock inputs.r9 pin bump (push-before-pin)

# Harvey flake.nix:
#   apps.drawterm         = pkgs.drawterm            # cocoa on darwin, X11 on linux
#   apps.drawterm-wayland = pkgs.drawterm-wayland    # linux wayland hosts
#   devshell += [ pkgs.socket-vmnet ]  (darwin only)
#   nix/boot-r9.sh: R9_NET=user|vmnet|tap selects netdev tier
#     user: -netdev user,id=n0,hostfwd=tcp::17010-:17010 -device virtio-net-pci,netdev=n0
#     vmnet: exec socket_vmnet_client /var/run/socket_vmnet … -netdev socket,id=n0,fd=3 …
#     tap:  -netdev bridge,id=n0,br=br0,helper="$QEMU_BRIDGE_HELPER" …
# Documented one-time setup (per host, in docs + nix/net-setup-*.sh):
#   darwin: sudo launchd daemon for socket_vmnet (plist ships with the package)
#   linux:  br0 + setuid qemu-bridge-helper + /etc/qemu/bridge.conf "allow br0"
```

## Version Compatibility

| A | B | Notes |
|---|---|-------|
| smoltcp 0.14 MSRV stable 1.91 | r9 nightly-2026-01-30 | clears it |
| virtio-drivers 0.13 PCI transport | qemu `-M virt` ECAM | "memory-mapped CAM only" per crate docs — exact match; `VIRTIO_F_VERSION_1` TODO upstream → gate with boot-test, keep `virtio-non-transitional` qemu habit |
| 9front drawterm (nixpkgs rev `3aa7ce0c`) | Harvey p9sk1 `cpu -R` | verified protocol match (ncpu + p9any/p9sk1 + rc4_256/sha1); use `-O` |
| nixpkgs drawterm | Harvey flake nixpkgs pin | requires a nixpkgs pin new enough to carry `0-unstable-2026-*` drawterm + `socket-vmnet` — check pin age in the phase |
| qemu host ≥7.1 | vmnet netdevs | any nixpkgs-current qemu qualifies; vmnet needs macOS 11+ |

## Sources

- **In-tree (HIGH, direct reads):** Harvey `sys/src/cmd/` (274 entries; 132 `.c`/`.y` + 130 dirs), `cmd/mkfile` (NOMK/APEDIRS), `libmemdraw`/`libdraw` mkfiles + `alloc.6` (binary artifact check), `cmd/mksyslib` (`%.$O: %.c` fallback), `cmd/rio/mkfile`, `cmd/auth/mkfile` + `keyfs.c` includes, `cmd/cpu.c` (authmethods, /mnt/term flow), `cmd/exportfs/`, `cmd/aux/{listen,listen1}.c`, `rc/bin/service/tcp17010`, `rc/bin/service.auth/`, `rc/bin/cpurc:39-51`, `lib/ndb/{common:255,291,local,auth}`, `sys/src/9k/root/profile:9-11`, `sys/src/9k/ip/` (15,395 lines), `sys/src/9k/386/ethervirtio.c`, `sys/src/9k/k10/k10cpu` (ethervirtio + ipconfig embedding), `sys/src/ape/lib/ap` (no arm64), `libc/9sys/{dial,announce}.c`, `cmd/ip/ipconfig/main.c:406`, `cmd/ndb/{cs,dns}.c`, `nix/boot-r9.sh`, `flake.nix`. r9: `Cargo.toml` (workspace lints), `aarch64/Cargo.toml`, `port/src/` device modules, `rust-toolchain.toml` (nightly-2026-01-30), `xtask/src/main.rs:671-672` (riscv64 netdev pattern).
- **docs.rs (MEDIUM, cross-verified):** virtio-drivers 0.13.0 README/crate/transport::pci/bus pages; smoltcp 0.14.0 features/lib.rs (MSRV 1.91, feature guards).
- **Raw primary (MEDIUM):** nixpkgs `pkgs/by-name/dr/drawterm/package.nix`; 9front drawterm `cpu.c` (p9any p9sk1 acceptance, mkservertickets, ncpu); nixpkgs `socket-vmnet` existence (pkgs/by-name + brew formula 1.2.2).
- **qemu-devel patches + Apple vmnet docs (MEDIUM):** vmnet netdevs ≥qemu 7.1, macOS 11+, root-or-restricted-entitlement; socket_vmnet README (root daemon + unprivileged qemu via `socket_vmnet_client` + `-netdev socket,fd=3`); lima managed-networking docs.
- **Guix cookbook / OpenWrt qemustart (MEDIUM):** qemu-bridge-helper setuid + `/etc/qemu/bridge.conf` allow-list + unique-MAC requirement.
- **9p.io/plan9.io wikis + authsrv(6) (MEDIUM, corroborated by in-tree files):** standalone cpu server setup, instant-cpu-server listen1 trick, p9sk1 ticket exchange.

---
*Stack research for: Harvey OS v2.0 — full aarch64 userspace, r9 networking, auth domain, cpu/drawterm, host tap/bridge*
*Researched: 2026-11-04*
