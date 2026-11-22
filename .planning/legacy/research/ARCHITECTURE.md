# Architecture Patterns — v2.0 Full System & Drawterm

**Domain:** OS milestone — full aarch64 userspace, guest networking, auth domain, cpu service, drawterm desktop
**Researched:** 2026-11-04
**Scope:** NEW-feature integration with the existing two-repo (Harvey + r9) architecture. v1.0 architecture is assumed, not re-derived.
**Overall confidence:** HIGH (all structural claims read directly from the two trees; only qemu-host-networking and effort estimates are MEDIUM)

---

## Recommended Architecture

```
host (darwin/linux)                     guest (qemu -M virt, r9 kernel)
┌─────────────────────────────┐        ┌────────────────────────────────────────┐
│ drawterm (nix-supplied)     │  tcp   │ listen (aux/listen) scans /net/tcp     │
│  serves /dev/draw /mouse    │─17010─▶│  └─▶ /rc/bin/service/tcp17010          │
│  /snarf over the cpu conn   │ 9P+    │       #!/bin/cpu -R                    │
│  mounts guest exportfs      │ p9sk1  │            ├─ auth_proxy (factotum)    │
└─────────────┬───────────────┘        │            ├─ auth_chuid → #¤ (devcap) │
              │ qemu netdev            │            ├─ newns(user)              │
   ┌──────────┴───────────┐            │            ├─ exportfs ⇄ drawterm      │
   │ slirp (default,       │  virtio-  │            └─ mount drawterm devfs →   │
   │  hostfwd 17010)       │  net-pci  │              /dev/draw,/dev/mouse…     │
   │ vmnet (darwin, sudo)  │◀─────────▶│              rio from $home/profile    │
   │ tap/bridge (linux)    │  virtqueue│                                          │
   └──────────────────────┘  +GICv2    │ kernel: devether(#l) ⇄ ethermedium      │
                             SPI/INTx  │           devip(#I): arp ip ipifc      │
┌─────────────────────────┐            │             iproute icmp udp tcp        │
│ srv9-style host 9P      │  9P/tcp    │             + loopback/null medium      │
│ server serves /adm      │◀──────────▶│ devmnt mounts /adm, /lib/ndb (real-use) │
│ + /lib/ndb (real use)   │            │ auth: keyfs authsrv factotum cs dns     │
└─────────────────────────┘            └────────────────────────────────────────┘
```

Everything above the `kernel:` line is **existing Harvey userspace, ported not invented**. Everything on the kernel line is **new Rust in r9, ported from Harvey's C**. The host's only novelties are qemu netdev wiring and the drawterm package.

---

## The TCP/IP Path Decision (the milestone's pivotal call)

### What Harvey actually ships (verified in-tree)

| Layer | Content | Location |
|-------|---------|----------|
| Kernel IP stack | Full IPv4+IPv6: `devip.c` (#I device), `ip.c`, `ipifc.c`, `iproute.c`, `arp.c`, `tcp.c` (3,486 lines), `udp.c`, `icmp.c`, `icmp6.c`, `ipv6.c`, mediums (`ethermedium`, `loopbackmedium`, `nullmedium`, `pktmedium`, `netdevmedium`, `gre`, `esp`, `rudp`, `igmp`, `ipmux`), `ipaux.c`, `chandial.c`, `netlog.c`, `ptclbsum.c` | `sys/src/9k/ip/` (**9k carries its own copy**, 22 files ≈ 15.4k lines C; it drifts from `sys/src/9/ip/`) |
| Kernel ether | `netif.c`/`netif.h` (portable ether framework) + per-driver `devether.c`/`ethervirtio.c` | `sys/src/9k/port/netif.c`, `sys/src/9k/386/ethervirtio.c` (legacy-transitional PCI only) |
| Userspace | `libip` = **address parsing/dial helpers only** (not a stack). `sys/src/cmd/ip/` = *services on top of the kernel stack*: `ipconfig` (2.1k-line main — configures the **kernel** stack via `/net/ether0`/`/net/ipifc` ctl writes + `dhcpclient.c`), `ping`, `traceroute`, `ftpd`, `telnetd`, `dhcpd`, `tftpd`, `snoopy`, `gping`, `udpecho`… | `sys/src/libip/`, `sys/src/cmd/ip/` |
| Connection service | `cs`/`dns` are userspace 9P servers mounted at `/net/cs`, `/net/dns` | `sys/src/cmd/ndb/cs.c`, `dns.c` |

**The decisive fact:** `announce()`/`listen()`/`dial()` in libc (`sys/src/libc/9sys/announce.c`) are **pure file I/O on `/net`** — open `/net/tcp/clone`, write `announce tcp!*!17010`, read back the line number. There are **no ANNOUNCE/LISTEN syscalls** in `sys/src/libc/9syscall/sys.h` (verified: table runs BIND(2)…NMOUNT(54) with no net syscalls). The entire TCP/IP interface is the `/net` file tree served by kernel devip. Whoever provides TCP must present devip's exact file interface (`clone`, `N/{ctl,data,local,remote,status}`, `ipifc`, `arp`) or none of libc/`ipconfig`/`listen` works.

### Options compared

| Option | What it means | Verdict |
|--------|---------------|---------|
| **(a)+(c) Rust `devether`+`devip` in r9, virtio-net under it** (the same option — (c) is (a) with the driver) | Port the IPv4 subset of `sys/src/9k/ip` to Rust in `r9/port/src/`; virtio-net feeds a new `devether` (#l) | **RECOMMENDED.** The only path that satisfies libc's `/net` contract without inventing a parallel one. r9 is the kernel; a Plan 9 kernel owns its IP stack. Bounded: ~11–12k lines C (IPv4 subset) → est. 6–8k lines Rust. |
| (b) Userspace TCP/IP over raw ether | Write a brand-new userspace TCP in C | **REJECTED.** Harvey ships no userspace stack to port — this is pure invention at the *same* scale as (a), and it must still reimplement devip's file interface for `announce()` to work. Worst of both worlds, non-idiomatic for Plan 9. |
| Kernel ip stack as foreign C linked into r9 | Compile `9k/ip` C into the Rust kernel | **REJECTED** (implicit): r9 is `#![forbid(unsafe_code)]` no_std Rust with a locked per-process usermem model; injecting a C stack violates the project's defining constraint (Key Decision: maximal unsafe elimination). |
| Dodge the stack entirely (9P over serial/virtio-console, slirp socket tricks) | — | **REJECTED.** The requirement is a real `tcp/17010` listener on a real L2 NIC that a stock drawterm dials. No dodge satisfies it. |

### Recommended shape: staged IPv4 port of `9k/ip`

Stage by protocol file, each with a gate (details in Build Order):

1. **Core + loopback**: `devip` device shell, `ip.c`, `ipifc.c`, `iproute.c`, `ipaux.c`, `chandial.c`, `netlog.c`, `ptclbsum.c`, `nullmedium`, `loopbackmedium` → `/net` exists; loopback dial works **before any NIC driver lands**. This decoupling is sequencing gold: auth/cpu pieces are testable over loopback.
2. **ICMP + UDP**: `icmp.c`, `udp.c` → `ping 127.0.0.1`, `udpecho` gates in-guest.
3. **TCP**: `tcp.c` (the 3.5k-line core: Reno CC, timers, listen/announce state machine) → in-guest `listen` on tcp/7 echo over loopback; then over ether.
4. **Ether binding**: `arp.c` + `ethermedium.c` bound to `devether` → real wire.
5. **Deferred**: `ipv6.c`/`icmp6.c` (defer to a later milestone), `gre/esp/rudp/igmp/inferno/ipmux/pktmedium/netdevmedium` (drop — not needed for cpu/auth/drawterm).

**Semantic dependencies the port must re-home** (the real cost, name them in the phase plan):
- **`Block`/`allocb` buffer chains** (`sys/src/9k/port/allocb.c`) — the stack's packet currency. r9 has no equivalent; add a `port/src/block.rs`.
- **`kproc` kernel processes** — tcp timers, arp expiry, iproute aging run as kernel procs in C. r9 has per-process kernel stacks + a 1 ms tick; restructure ip timers onto a tick-driven timer wheel instead of growing a kproc concept. Design decision for the phase.
- **`Rendez` sleep/wakeup + `qlock`** — r9 has `mcslock`/`Lock` and wait/wake machinery inside devpipe; extract a reusable rendez.
- **`Fs`/`Proto` table dispatch** (`devip.c` protocol table, `ipprotoinit[]`) — maps naturally to Rust match/statics.
- **Medium abstraction** — `Medium` vtable → Rust trait. This is what lets loopback land first.

---

## virtio-net Driver Shape in r9

**Transport choice: virtio 1.x modern PCI, non-transitional only.** Phase 2's donor note (`build/aarch64/boot-9qemu.sh:64-66`) established that qemu 11.1.2's default `virtio-net-pci` is transitional (device 0x1000) and already forced `virtio-net-pci-non-transitional` (modern, 0x1041) for the 9front donor. Since r9's xtask owns the qemu command line via TOML data (D-05/D-06/D-07), r9 dictates the modern device and implements **only** the modern path — PCI capabilities list (common cfg / notify / ISR / device cfg), no legacy I/O-port state machine. Harvey's own `9k/386/ethervirtio.c` is legacy-era and x86-port-I/O-flavored; **9front's `ethervirtio10.c` (in the vendored donor tree) is the correct modern reference**.

**New components (r9 repo):**

| Component | File (proposed) | Content |
|-----------|-----------------|---------|
| PCI/ECAM discovery | `aarch64/src/platform/virt/pci.rs` | ECAM window at PA `0x4010000000` (qemu virt map, confirmed in DTB `pci` node); bus scan; BAR sizing; INTx→SPI routing read from the DTB's PCI `interrupt-map` (r9's DTB-discovered-drivers convention — **never hardcoded IRQ numbers**) |
| virtio transport | `port/src/virtio.rs` | Virtio 1.x over PCI: caps parsing, split virtqueues (descriptor table/avail/used), queue 0=RX, queue 1=TX, ISR read + notify |
| devether | `port/src/devether.rs` + `port/src/netif.rs` | #l device presenting `/net/ether0/{clone,0/{addr,ctl,data,ifstats,stats,type}}` per `netif.h`; RX IRQ → wakeup → ethermedium; TX from ip stack → virtqueue |
| qemu wiring | `aarch64/lib/config_virt.toml` | `-device virtio-net-pci-non-transitional,netdev=n0` + `-netdev …` become TOML `[qemu]` data — platforms-as-data decision already locked |

**Interrupts:** PCI INTx → GICv2 SPI per DTB interrupt-map → existing `trap()` dispatch gains an IRQ-handler registry slot (today: timer INTID 30, PL011 INTID 33; virtio SPI joins the same table). MSI is possible on gicv2 only via a GICv2m frame — **skip for v2.0**, INTx is one line and DTB-declared.

---

## What auth/cpu/rio Need From the r9 Kernel

All three are **Harvey userspace** (`sys/src/cmd/auth/`, `cpu.c`, `exportfs/`, `aux/listen.c`, `rio/`). Verified kernel requirements beyond the measured 26-syscall set (r9 `aarch64/src/syscall.rs` dispatch table, currently 26 `Some` entries):

| Need | Why (evidence) | Size |
|------|----------------|------|
| **`#!` exec in r9** | `/rc/bin/service/tcp17010` is literally `#!/bin/cpu -R`, and `aux/listen.c:541` `execl()`s it directly — kernel-side `#!` parsing (Harvey: `sys/src/9k/port/sysproc.c:282-301`). r9 `exec.rs` has **no** shebang path today. Without it the cpu service cannot start. | Small, load-bearing |
| **devcap (`#¤`)** | cpu server set-user: `cpu.c:srvp9auth` → `auth_chuid(ai)` → libauth writes `/dev/capwrite` after the auth server seeds `/dev/caphash`. Standard namespace already does `bind -a #¤ /dev` (`lib/namespace`). | Small device (~200 lines C in `9k/port/devcap.c`) |
| **`/dev/random` + `/dev/urandom`** | p9sk1 challenge generation (factotum, authsrv, cpu key exchange — `cpu.c:p9auth` calls `truerand()`). r9 devcons has **no** random files today. Entropy on qemu-virt: recommend **DTB-seeded CSPRNG** — the boot-r9 app already fdtput-patches `/chosen` (initrd delivery, Phase 4); add an `rng-seed` property per launch, expand via chacha (libsec lineage). `virtio-rng-pci` is the cleaner follow-up once virtio exists — name both, take DTB-seed for v2.0. Dev-grade crypto is honest for this milestone (TLS hardening is explicitly out of scope). | Small |
| **WSTAT(44) / FWSTAT(45) / REMOVE(25)** | exportfs maps incoming Twstat/Tremove to these syscalls; a drawterm session that edits files (rio snarf buffers, acme) needs them. Today loud-`None`. | Small |
| **Measure-first additions** | RENDEZVOUS(34), SEMACQUIRE(37)/SEMRELEASE(38)/TSEMACQUIRE(52), SEGATTACH(30): predicted-unused but unverified. **Apply the Phase 5 D-14 discipline**: tracer-gate the real auth/cpu boot chain, extend only what the trace names. FAUTH(10) stays out (cpu-layer auth doesn't fauth; the designed no-auth fall-through already exists). | Method, not code |
| **devproc completeness** | `listen` procsetname writes `#p/pid/args`; cpurc/rc job control pokes `/proc` ctl. Phase 5's "full /proc" likely covers; verify against the traced chain. | Verify only |
| **Time** | r9 devcons already has `#c/time` (fixed epoch + settime writes) — sufficient for p9sk1 (tickets carry no wall-clock expiry). Optional nicety: PL031 RTC at `0x09010000` (DTB `arm,pl031`) for real wall time; ipconfig/DHCP lease bookkeeping likes it. **Not blocking.** | Optional |
| **devssl (`#S`)** | `cpu.c` defaults `ealgs = "rc4_256 sha1"` via `pushssl`. **Recommend negotiate `clear` for v2.0** (both cpu and drawterm accept `-e clear`) and defer devssl to the security-hardening milestone (already named Out of Scope). p9sk1 still authenticates; the session is unencrypted. Document loudly. | Deferred by decision |
| **Writable storage for auth state** | **Gap:** r9 has no writable fs — paqfs/devroot is read-only by design. `/adm/keys` (keyfs's format) must live somewhere. **Answer: host-served 9P.** The Phase 2 `harness srv9` lineage already serves 9P2000/tcp to a guest; r9's devmnt (kernel 9P client) mounts it as `/adm` + `/lib/ndb`. Chicken-egg is clean: the mount needs no auth on the host-local tap. **CI/smoke variant:** bake a fixed dev-password `/adm/keys` into the paqfs image (host-side generation via a one-shot `auth-setup` boot that writes through the 9P mount, then re-bake). Baked = deterministic CI; host-9P = real use. Both documented. | Integration, no new fs |

---

## Boot Flow Changes (terminal → cpu server)

Current r9 boot (`sys/src/9k/boot/bootr9.c`) hardcodes the terminal profile: `cpuflag=0` → `init -t` → `termrc`, method table = embed `/rootfs.img` only, `nobootprompt=embed` seeded by the kernel.

CPU-server profile needs:

1. **`service=cpu` delivery**: `boot.c` reads plan9.ini keys (`#e` env) and runs `init -c` → `/rc/bin/cpurc`. plan9.ini keys arrive via **DTB `/chosen/bootargs` → `-append`** — the Phase 2/4-established channel (`bootargs=tcp fs=… user=glenda` precedent). The boot-r9 app adds `service=cpu auth=<authdom-ip>` to the append line; `bootr9.c` (or a `bootr9cpu.c` sibling) sets `cpuflag=1` from it instead of hardcoding 0.
2. **A real `cpurc`** (extend `cfg/example/cpurc`, currently a 13-line stub): `ip/ipconfig` (static or `dhcpclient` vs slirp's 10.0.2.x DHCP), mount host-9P `/adm`+`/lib/ndb` (real-use path), `auth/keyfs -w /adm/keys`, `auth/authsrv`, `ndb/cs` + `ndb/dns` (guest-local — `lib/namespace` already mounts `/srv/cs`,`/srv/dns` into `/net`), `aux/listen -q tcp`.
3. **ndb content**: `/lib/ndb/local` with the authdom, guest IP, `auth=` entry. Baked into the image or host-9P-mounted (same duality as `/adm/keys`).
4. **init exists**: `sys/src/cmd/init.c` — the `-c` path is stock; no kernel change beyond the bootconf cpuflag.

**Namespace for a drawterm session** (all existing machinery): `cpu -R` post-auth runs `newns(user, nil)` → `/lib/namespace` (verified present, binds `#I`/`#l` into `/net`, `/srv/factotum` into `/mnt`), then serves `exportfs` toward drawterm **and mounts drawterm's device fs back over the same connection** (the cpu protocol is bidirectional 9P on one fd — how cpu/import have always worked), binding drawterm's `/dev/draw`, `/dev/mouse`, `/dev/cursor`, `/dev/snarf` into the session's `/dev`. rio starts from the user's `$home/lib/profile`, opens `/dev/draw` — which is now the **host's** window. r9 needs **no** devdraw/devmouse/gpu: the draw device arrives over 9P. This is the canonical Plan 9 terminal/cpu arrangement with drawterm playing the terminal kernel's role. (`rio.c` verified: needs `/dev/snarf`, `/dev/draw` via libdraw, `/dev/mouse` via mouse fd — all drawterm-served.)

---

## Host Side

| Piece | darwin | linux | Notes |
|-------|--------|-------|-------|
| Default net (unprivileged) | `-netdev user,id=n0,hostfwd=tcp:127.0.0.1:17010-:17010` (slirp) | same | Works with guest DHCP (10.0.2.15, gw 10.0.2.2); drawterm dials localhost. **CI-able** — the smoke gate uses this. Not true L2 (no real ARP visibility), which is fine for v2.0's gate. |
| Real L2 (documented, privileged) | `-netdev vmnet-shared` / `vmnet-bridged` — **requires root or the restricted `com.apple.vm.networking` entitlement** (confirmed: vmnet.framework is root-gated) | `-netdev tap,ifname=tap0,script=no` + bridge, or `-netdev bridge` via qemu-bridge-helper (setuid) | Per-host setup doc in the repo; the milestone's "real L2" story lands here. |
| Flake wrap | `boot-r9` app gains a net-mode switch: `slirp` default; `vmnet`/`tap` via env/flag; r9 TOML `[qemu]` gains netdev data | same | Keeps platforms/hosts-as-data discipline |
| drawterm | nixpkgs `drawterm` (builds darwin+linux; X11 on linux, cocoa-capable on darwin) | nixpkgs `drawterm` | **Recommend nixpkgs pin for v2.0.** In-tree `sys/src/cmd/unix/drawterm/` (with `Make.osx`/`gui-osx`, `Make.unix`/`gui-x11`) is a vendored-build follow-up consistent with the vendor-and-own discipline — not gate-blocking. |

---

## Suggested Build Order (dependency-explicit)

```
[1] Full aarch64 userspace ──────────────────────────────┐ (parallel with everything;
     all sys/src/cmd + GUI libs compile; compile-gated     │  GUI validated headless until [8])
     (ipconfig, listen, cpu, exportfs, auth/*, ndb/*, rio) │
                                                          ▼
[2] r9 PCI/ECAM + virtio-1.x + virtqueues + virtio-net + devether(#l)
     gate: RX/TX packet counters; ARP request visible on host tap/slirp
     │
     ▼ (parallel with [2]! loopback needs no NIC)
[3] r9 devip core: block.rs + devip/ip/ipifc/iproute/ipaux/chandial/netlog
     + null/loopback mediums; then icmp→udp→tcp
     gates: ping 127.0.0.1 → udpecho → in-guest tcp/7 echo (all loopback)
     │
     ├──────────────────────────────┐
     ▼                              ▼
[4] arp + ethermedium bound to    [6] r9 kernel service additions (parallel):
     devether; ipconfig vs slirp      #! exec, devcap(#¤), /dev/random (DTB-seed),
     gates: ping 10.0.2.2; udp        WSTAT/FWSTAT/REMOVE, tracer-gated syscall
     echo to host                     extensions, devproc verification
     │                              │
     ▼                              │
[5] Host net flake: slirp default + hostfwd, documented vmnet/tap,
     r9 TOML netdev data (needs [2]; e2e gate needs [4])
     │                              │
     ▼                              ▼
[7] Auth domain: cpurc, service=cpu bootconf, /adm/keys + ndb content
     (baked + host-9P paths), keyfs/authsrv/factotum/cs/dns composition
     gate: factotum p9sk1 challenge vs local authsrv **over loopback** [needs 3,6,1]
     │
     ▼
[8] cpu service + drawterm: listen/tcp17010, exportfs, drawterm pkg, rio session
     gates: drawterm → p9sk1 → rio window (interactive UAT);
            CI smoke via hostfwd 127.0.0.1:17010 [needs 4,5,7]
```

**Ordering rationale:**
- [2]∥[3] is the critical decoupling: loopback devip gives a `/net` before the NIC exists, so the TCP state machine (hardest part) is debuggable without driver heisenbugs in the loop.
- [7] before [8]: the cpu service authenticates; testing cpu without auth first is the debuggable intermediate (`srvnoauth` path exists in `cpu.c` — a legit stepping stone behind a debug flag, not the shipped gate).
- [1] is genuinely parallel: it shares no code with the r9 work and is compile-gated until [8] gives the GUI stack somewhere to run.
- [5]'s slirp leg is pure qemu-args work (can start at [2]); only its acceptance needs [4].

---

## Component Boundaries (new vs modified)

| Component | Repo | Status | Responsibility |
|-----------|------|--------|----------------|
| `port/src/virtio.rs` | r9 | **NEW** | virtio 1.x PCI transport, split virtqueues |
| `aarch64/src/platform/virt/pci.rs` | r9 | **NEW** | ECAM scan, BARs, DTB interrupt-map → GIC SPI |
| `port/src/devether.rs` + `netif.rs` | r9 | **NEW** | #l device, `/net/ether0` file tree |
| `port/src/block.rs` | r9 | **NEW** | allocb/Block packet buffers |
| `port/src/devip.rs` + `ip/` module | r9 | **NEW** (port of `9k/ip` IPv4 subset) | #I device, TCP/UDP/ICMP/ARP, mediums |
| `port/src/devcap.rs` | r9 | **NEW** (port of `9k/port/devcap.c`) | #¤ caphash/capwrite |
| `port/src/devcons.rs` | r9 | **MODIFIED** | add `random`/`urandom` files (DTB-seeded CSPRNG); optional PL031 rtc |
| `aarch64/src/exec.rs` | r9 | **MODIFIED** | `#!` interpreter exec |
| `aarch64/src/syscall.rs` | r9 | **MODIFIED** | WSTAT/FWSTAT/REMOVE + tracer-named additions |
| `aarch64/lib/config_virt.toml` | r9 | **MODIFIED** | virtio-net device + netdev qemu args as data |
| `sys/src/9k/boot/bootr9.c` (+`bootr9cpu.c`) | Harvey | **MODIFIED/NEW** | cpuflag from plan9.ini `service` key |
| `cfg/example/cpurc` | Harvey | **MODIFIED** | real cpu-server startup (ipconfig, keyfs, authsrv, cs/dns, listen) |
| `lib/ndb/local` + `/adm/keys` bake | Harvey | **NEW content** | authdom data; mkpaqfs bake stage injects |
| `flake.nix` / `nix/*.sh` | Harvey | **MODIFIED** | net-mode switch on boot-r9; `drawterm` app; `auth-setup` app |
| `sys/src/cmd/*` (all) + GUI libs | Harvey | **MODIFIED** (port fixes only) | aarch64 compile — no architectural change |
| `sys/src/cmd/ip/*`, `auth/*`, `cpu.c`, `exportfs/`, `aux/listen*`, `rio/`, `ndb/*` | Harvey | **UNCHANGED architecturally** | they are the payload; compile + run |

## Anti-Patterns to Avoid

### Inventing a userspace TCP/IP to dodge the kernel port
**What:** raw ether to a C/Rust userspace stack. **Why bad:** must still reimplement devip's `/net` file contract for libc `announce()` — same work, worse shape, breaks ipconfig/cs/dns assumptions. **Instead:** port `9k/ip` IPv4 subset into r9.

### Hardcoding virtio IRQs/addresses
**Why bad:** violates r9's locked DTB-discovered-driver convention (Phase 3 decision, raspi3b/qemu drift is the standing cautionary tale). **Instead:** PCI node + interrupt-map from the DTB; platform differences stay TOML/DTB data.

### Letting the C stack's kproc/timer model leak into r9 unchanged
**Why bad:** r9 deliberately has no kproc concept; bolting one on for ip timers grows kernel surface the unsafe-elimination milestone just shrank. **Instead:** tick-driven timer wheel on the existing 1 ms generic-timer tick.

### Shipping devssl/rc4 because cpu.c defaults to it
**Why bad:** ports weak crypto (rc4_256/sha1) into the new kernel — the opposite of the TLS-modernization direction. **Instead:** `-e clear` for v2.0 (documented), devssl/TLS in the hardening milestone.

### Baking real user keys into committed images
**Why bad:** secrets in git. **Instead:** baked **dev-only** keys for CI smoke (clearly marked), host-9P `/adm` mount for real use, `auth-setup` one-shot boot to generate.

## Scalability Considerations

| Concern | v2.0 (single drawterm) | Honest limit |
|---------|------------------------|--------------|
| devmnt outstanding RPCs | one per session (existing discipline) — fine for /adm mount | multi-mount concurrency unchanged |
| virtio queues | RX+TX pair, no multiqueue | multiqueue/VQ sizing if throughput matters |
| ip stack connections | devip's conv table — scales by design | IPv6/mediums deferred, not closed off |

## Sources

All HIGH confidence (read directly from source) unless noted:

- Harvey: `sys/src/9k/ip/` (22 files, 15.4k lines), `sys/src/9k/k10/k10cpu` (kernel config: `ip`+`ether netif`+`ethervirtio`+embedded `/amd64/bin/ip/ipconfig`), `sys/src/9k/mk/parse:319-327` (ip table codegen), `sys/src/libc/9syscall/sys.h` (no net syscalls; NMOUNT=54), `sys/src/libc/9sys/announce.c` (announce/listen = /net file I/O), `sys/src/cmd/cpu.c` (srvp9auth→auth_proxy→auth_chuid; ealgs default; newns), `sys/src/cmd/aux/listen.c:541` (direct execl of #! service scripts), `rc/bin/service/tcp17010` (`#!/bin/cpu -R`), `lib/namespace` (#I/#l/#¤/#S binds, /srv/cs+dns mounts), `cfg/example/cpurc` (stub), `sys/src/9k/boot/bootr9.c` (hardcoded terminal cpuflag), `sys/src/9k/port/sysproc.c:282-301` (kernel #! handling), `sys/src/9k/port/devcap.c`, `sys/src/9k/port/random.c`, `sys/src/cmd/unix/drawterm/` (in-tree drawterm, Make.osx/gui-osx), `build/aarch64/boot-9qemu.sh:55-90` (virtio-non-transitional donor note; srv9 9P/tcp root precedent), `build/aarch64/README.md` (bake-vs-ufs resolution).
- r9: `port/src/` device inventory (no net devices), `aarch64/src/syscall.rs` (26-entry dispatch, loud-None policy), `aarch64/src/exec.rs` (no #!), `port/src/devcons.rs` (time file present, no random), `aarch64/src/platform/virt/` (devcons/gic/pl011/timer; DTB-discovered convention), `HowItWorks.md` (boot contract, TOML-as-data, fdtput /chosen patching), `xtask/` (qemu args from TOML).
- MEDIUM: qemu vmnet root/entitlement requirement (Apple forums/lima docs via web search, 2026); slirp hostfwd behavior (qemu docs, long-stable).
- Effort estimates (Rust port sizes) are inference from C line counts — MEDIUM confidence, treat as planning inputs not commitments.
