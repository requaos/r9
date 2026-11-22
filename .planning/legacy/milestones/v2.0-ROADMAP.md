# Roadmap: Harvey OS — Build Revival + aarch64 Port

## Overview

v2.0 turns the v1.0 shell-boot proof into a full Plan 9 system. Two parallel tracks run from the v1.0 foundation: the Harvey userspace track sweeps the whole tree to aarch64 behind Wave-0 landmine fixes and tiered, execution-gated allowlists (Phases 6, 10), while the r9/host networking track builds PCI/ECAM + virtio-net + devether (Phase 7), the devip-compatible `/net` TCP/IP path loopback-first (Phase 8), and the explicit host netdev tiers (Phase 9). The tracks hard-join at the in-guest p9sk1 auth domain (Phase 11) and again at the payoff — cpu service + drawterm + rio desktop (Phase 12) — where `nix run .#boot-r9` remains the ONE command and now brings up the complete system, and `nix run .#drawterm` lands the user in an interactive rio session.

## Milestones

- ✅ **v1.0 Boot Revival** — Phases 1-5 (shipped 2026-11-04)
- 🚧 **v2.0 Full System & Drawterm** — Phases 6-12 (in progress)

## Phases

**Phase Numbering:**
- Integer phases (6, 7, 8…): Planned milestone work — numbering CONTINUES from v1.0 (ended at Phase 5); v2.0's first phase is Phase 6
- Decimal phases (7.1, 7.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

<details>
<summary>✅ v1.0 Boot Revival (Phases 1-5) — SHIPPED 2026-11-04</summary>

- [x] Phase 1: Build Tail Green (9k amd64) (4/4 plans) — completed 2026-11-01
- [x] Phase 2: aarch64 Toolchain + libc + 9front Donor (7/7 plans) — completed 2026-11-01
- [x] Phase 3: r9 qemu-virt Platform Bring-up (6/6 plans) — completed 2026-11-01
- [x] Phase 4: r9 Userspace Exec + Harvey↔r9 ABI (8/8 plans) — completed 2026-11-02
- [x] Phase 5: Shell Boot + One-Command Integration (13/13 plans) — completed 2026-11-04

Full detail: `.planning/milestones/v1.0-ROADMAP.md`

</details>

**v2.0 Full System & Drawterm (Phases 6–12):**

- [x] **Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers** - Landmine fixes + generator audit + tiered allowlist machinery; tier-0 libs and core cmds/build chain execute green on the donor kernel first, then r9 (completed 2026-11-05)
- [x] **Phase 7: r9 PCI/ECAM + virtio-net + devether** - *(cross-repo: `../r9`)* DTB-driven PCI discovery, zero-offload virtio-net from Day 1, `/net/ether0`, guest ARP observable on the host (completed 2026-11-05)
- [x] **Phase 8: r9 TCP/IP Path — devip-Compatible /net** - *(cross-repo: `../r9`)* Loopback-first `/net` with devip-exact file semantics; exit gate is sustained bidirectional TCP on the tap leg, not ping on slirp (completed 2026-11-06; the NETIP-08 tap-leg truth verdict is Phase 9's joint acceptance gate per D-15)
- [ ] **Phase 9: Host Networking + boot-r9 Net Modes** - Hermetic slirp+hostfwd default (CI-able), explicit `--net=slirp|tap|vmnet`, documented linux tap / darwin socket_vmnet legs, doctor-checker
- [ ] **Phase 10: Userspace Services + GUI Tiers + Bootfs Payload** - Services/IP/auth suites donor-executed; GUI stack compiled with measure-first donor syscall traces; fonts/profiles/ndb staged in bootfs
- [ ] **Phase 11: In-Guest p9sk1 Auth Domain** - Hash-paired bake-time domain generator; keyfs + authsrv on tcp/567 + ndb; r9 devcap + `/dev/random`; loopback p9sk1 probe green before any drawterm
- [ ] **Phase 12: CPU Service + Drawterm + rio Desktop** - `service=cpu` boot + aux/listen tcp17010 + `cpu -R` end-to-end; flake drawterm; rio in-session; full-chain smoke + clean-checkout proof

## Phase Details

### Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers

**Goal**: The mk build factory sweeps the full tree for aarch64 without structural aborts or silent hangs, gated by tiered append-only allowlists whose only truth is artifact+execution markers — tier-0 libraries and the core command/build-chain tier run green on the 9front donor kernel first, then on r9.
**Depends on**: Nothing in v2.0 (v1.0 complete — the dual-arch build, donor-kernel harness, and r9 shell substrate are the entry equipment); runs parallel with Phases 7–9
**Requirements**: USPACE-01, USPACE-02, USPACE-03, USPACE-04, USPACE-05, USPACE-06, USPACE-10
**Success Criteria** (what must be TRUE):
  1. Wave-0 landmines are closed with regression proof: the full-tree aarch64 sweep runs without the cc/macbody + 6l/obj.c gethunk compile-hangs, and the v1.0-carryover 9k-amd64 embedded-root `bad magic` pool panic is re-tested against the pswt.c-fixed 6c with a recorded verdict (USPACE-01, USPACE-10)
  2. The mk DIRS recursion demonstrably reaches every cmd dir — the jpg/mkfile-class `oc` derivation, mahjongg cpp, and `4a` lex.c blockers are fixed, and a committed dir-coverage report proves complete reach (USPACE-02)
  3. The cross-compiled-generator audit artifact lists every build-then-run generator in swept mkfiles with host-toolchain proof (7c output never runs on the host), and CI reds on any `.7`-magic generator artifact (USPACE-03)
  4. Tiered allowlist machinery is live: per-dir append-only allowlist growth (never a flag-flip), per-tier execution gates run on the 9front donor kernel FIRST then r9 via measure-first traces, and artifact+execution markers are committed as the only gates — namespace-build mk exit status is never trusted (USPACE-04)
  5. Tier-0/1 are green with committed markers: all 22 outstanding libraries (libdraw/libmemdraw/libmemlayer/libframe, libcontrol, libhtml, libhttpd, libdisk, libflate, libgeometry, liblex, libplumb, libventi/liboventi, …) compile for aarch64 via the mksyslib portable-C fallback (no cargo-culted stale `.6`/`.8` objects), and shell/coreutils + the guest build chain (7a/7c/7l/cc/cpp/mk/yacc/lex as aarch64 binaries) execute on the donor kernel (USPACE-05, USPACE-06)

**Plans:** 10/10 plans complete (+ 2 gap-closure plans planned for G-06-1)
**Wave 1**
- [x] 06-01-PLAN.md — Wave-0 landmines: gethunk ×2 fixes + teeth-proven repro gates + FPdbleword uint fix (USPACE-01, D-01)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 06-02-PLAN.md — Append-only tier allowlist machinery: D-02 hygiene, A1 spike, D-13 checkpoint, objtype-gated mk filter, build_aarch64 filtered sweep (USPACE-04)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 06-03-PLAN.md — Tracer: libavl + awk + float canary end-to-end through the donor gate stack; ledgers started (USPACE-04, D-07/D-10/D-14)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 06-04-PLAN.md — Dir-coverage report + generator audit + .7-magic scan (USPACE-02, USPACE-03, D-15)

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 06-05-PLAN.md — Tier-0 wave: 22 libraries compile fresh via mksyslib portable-C; libdynld rename verdict; documented exclusions (USPACE-05, D-05..D-08)

**Wave 6** *(blocked on Wave 5 completion)*
- [x] 06-06-PLAN.md — Tier-1: build chain + ~32 coreutils + guest self-host canary on the donor (USPACE-06, D-09/D-11)

**Wave 7** *(blocked on Wave 6 completion)*
- [x] 06-07-PLAN.md — r9 venue: measure-first dispatch growth, push-before-pin, boot-r9-tier1 gate (USPACE-04, USPACE-06, D-12)

**Wave 8** *(blocked on Wave 7 completion)*
- [x] 06-08-PLAN.md — CI pipeline: phase6.yml four jobs incl. x86_64 pool-panic verdict leg (USPACE-10, D-04/D-16)

**Wave 9** *(gap closure G-06-1 — maintainer direction 2026-11-05: x86_64 CI donor evidence from Harvey's own kernel)*
- [x] 06-09-PLAN.md — Harvey-kernel tier venue: k10t1 embedded-root config + devroot raise + build driver + sweep harness, local green proof with teeth (USPACE-04, USPACE-06)

**Wave 10** *(gap closure G-06-1, blocked on Wave 9)*
- [x] 06-10-PLAN.md — phase6.yml: tier-9k job in, tier-donor leg retired + venue-pin assertion; dir-coverage venue wording (USPACE-02, USPACE-04)

### Phase 7: r9 PCI/ECAM + virtio-net + devether

**Goal**: r9 speaks Ethernet on qemu virt — PCI/ECAM discovery driven entirely by the committed DTB, a virtio-net driver with Day-1 zero-offload discipline over barrier-first virtqueues, and devether presenting `/net/ether0`, proven by a guest ARP request captured on the host. **Kernel work in sibling repo `../r9`.**
**Depends on**: v1.0 (Phase 5 — r9 virt platform + boot-test harness); parallel with Phase 6 (different repo) and with Phase 8's loopback staging
**Requirements**: NETDRV-01, NETDRV-02, NETDRV-03, NETDRV-04
**Success Criteria** (what must be TRUE):
  1. r9 finds the NIC from data, not constants: DTB-driven ECAM scan + `interrupt-map`→GICv2 SPI routing with zero hardcoded IRQs/addresses, demonstrated by a DTB-perturbation (moved-window) boot-test that still binds the device (NETDRV-01)
  2. The virtio-net driver negotiates the zero-offload feature mask (MAC+STATUS only) from Day 1 over split virtqueues with barrier-first aarch64 ring code, and the provenance decision (virtio-drivers crate vs from-scratch `port/src/virtio.rs`) is recorded in-repo (NETDRV-02)
  3. devether `#l` presents `/net/ether0` per netif.h with live RX/TX packet counters, and the guest's ARP request is observable in a host-side capture (NETDRV-03)
  4. Interrupt accounting is proven under load: INTx level-triggered plumbing with ISR read-to-ack, and a sustained packet blast shows guest and host counters reconciling with no silent virtqueue wedge (NETDRV-04)

**Plans:** 10/10 plans complete
- [x] 07-09-PLAN.md
- [x] 07-10-PLAN.md — gap closure round 2 (G-07-1): RX payload-offset fix (drop the +VIRTIO_NET_HDR_LEN slice base on two-descriptor chains) + chain-faithful host pin + virtnet gate re-proof + push-before-pin (NETDRV-03)

**Wave 1**
- [x] 07-01-PLAN.md — xtask netdev stanza (D-09/D-10 idle staging) + pcap parser + harvey: initrd channel + idle-NIC regression (NETDRV-03 machinery)
- [x] 07-03-PLAN.md — from-scratch virtio transport + unsafe-free ring index model + host unit tests + D-01 provenance record (NETDRV-02 transport half)
- [x] 07-04-PLAN.md — Harvey bootfs payload: 60-byte arp.req fixture, net-sweep.rc, netblast.c, dd into the image (NETDRV-03 payload half)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 07-02-PLAN.md — fdt interrupt-map extensions + pci.rs DTB discovery prints + lowecam moved-window perturbation gate (NETDRV-01)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 07-05-PLAN.md — virtnet boot-test profile: Net stage machine + typed sweep + pcap/stats assertion suite + red-path proof (NETDRV-03 gate machinery)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 07-06-PLAN.md — virtio-net driver bring-up: BAR/caps/negotiation/queues/ISR handler + GIC ICFGR + bind in the arming window, ether on all virt configs (NETDRV-02 boot half)

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 07-07-PLAN.md — devether #l + netif model + arp-mode gate green + blast-mode load gate + evidence (NETDRV-03, NETDRV-04)

**Wave 6** *(blocked on Wave 5 completion)*
- [x] 07-08-PLAN.md — closure: r9 push-before-pin, flake.lock pin advance (own commit), compose-path NIC + smoke bind-line gate + CI + 07-EVIDENCE.md (NETDRV-03 end state)

### Phase 8: r9 TCP/IP Path — devip-Compatible /net

**Goal**: `/net` exists with devip-exact file semantics — loopback + null mediums FIRST so the TCP state machine is debuggable without the NIC, then arp/ethermedium bound to devether — ending with a sustained bidirectional TCP exit gate on the tap leg. **Kernel work in sibling repo `../r9`; `sys/src/9k/ip/` is a read-only semantics reference.**
**Depends on**: v1.0 (Phase 5 — r9 syscall/device substrate); loopback/null staging runs parallel with Phase 7 by design (the loopback-before-NIC decoupling); the ether binding (NETIP-04 onward) needs Phase 7's devether, and the NETIP-08 exit gate needs Phase 9's tap leg
**Requirements**: NETIP-01, NETIP-02, NETIP-03, NETIP-04, NETIP-05, NETIP-06, NETIP-07, NETIP-08
**Success Criteria** (what must be TRUE):
  1. **DECISION GATE**: the TCP implementation choice (research recommends smoltcp 0.14 behind the devip ABI vs a staged Rust port of `sys/src/9k/ip`) is FINAL and recorded — together with the same-axis virtio provenance decision — before any driver-dependent plan lands (NETIP-01)
  2. libc `dial()`/`announce()`/`listen()` work unmodified against r9's `/net`: tcp/udp/icmp clone/ctl/data/status + ipifc with devip-exact semantics — no network syscalls added, the file tree IS the contract (NETIP-02)
  3. TCP is debuggable without the NIC: `ping 127.0.0.1`, udp echo, and in-guest tcp/7 echo over loopback all pass as committed expect gates (NETIP-03)
  4. The guest takes an address and is reachable: arp + ethermedium bound to devether, static ipconfig assigns IPv4, DHCP works via ipconfig `/net/udp` broadcast fidelity (or the recorded smoltcp dhcpv4 fallback), and the devip ip6 tree + ipconfig IPv6 configuration are demonstrated (NETIP-04, NETIP-05, NETIP-06)
  5. The networking exit gate is green: sustained bidirectional TCP flow + load soak **on the tap leg** (slirp evidence explicitly not accepted — it forgives L2 bugs), with aan available for cpu connections (NETIP-07, NETIP-08)

**Plans:** 9/9 plans complete
**Wave 1**
- [x] 08-01-PLAN.md — D-03 contract doc (full devip surface enumeration) + smoltcp 0.14.0 pin/lockfile + cfg(dev_ip) + NETIP-01 provenance record (NETIP-01, NETIP-02; D-01..D-04)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 08-02-PLAN.md — event-driven seam (stack/waker/phyadapt) + host pins incl. OQ-1 DHCP-shaped exchange FIRST + UNSAFE-AUDIT entry (NETIP-02, NETIP-05; D-05)
- [x] 08-03-PLAN.md — Harvey gate payloads: ipconfig/ping/udpecho/aan + tcpblast/tcpecho + D-11 addressing seam + bake asserts (NETIP-03..08; D-11, D-12)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 08-04-PLAN.md — TRACER: ping 127.0.0.1 over loopback end-to-end (#I device + conv + mediums + icmp + kernel wiring + virtip profile) + udp echo (NETIP-02, NETIP-03)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 08-05-PLAN.md — tcp proto (accept pool, blocking connect, listen/err, close/hangup mapping) + tcp/7 echo + netlog/stats/iproute/ipselftab/ndb (NETIP-02, NETIP-03; D-07)

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 08-06-PLAN.md — D-17 /dev/random + /dev/urandom, ethermedium + EtherDev + snoop-fed /net/arp + gratuitous ARP + static ipconfig gate on slirp (NETIP-02, NETIP-04; D-17)

**Wave 6** *(blocked on Wave 5 completion)*
- [x] 08-07-PLAN.md — DHCP via unmodified ipconfig over /net/udp broadcast fidelity + IPv6 (icmpv6 full-packet bridge, add6/ra6/try) (NETIP-05, NETIP-06; D-06, D-09, D-10)

**Wave 7** *(blocked on Wave 6 completion)*
- [x] 08-08-PLAN.md — aan scripted drop-and-resume on loopback + tcpblast bidirectional soak machinery on slirp + documented manual soak command (NETIP-07, NETIP-08; D-14, D-15, D-16)

**Wave 8** *(blocked on Wave 7 completion)*
- [x] 08-09-PLAN.md — composed net smoke + CI net-smoke job + 08-EVIDENCE.md + push-before-pin + flake.lock advance (D-13, D-16)

### Phase 9: Host Networking + boot-r9 Net Modes

**Goal**: Host networking is an explicit, documented choice — a hermetic slirp+hostfwd default that runs unprivileged and in CI, real-L2 tap/bridge on linux and opt-in socket_vmnet on darwin, and a doctor-checker that prints exact remediation for privileged setup.
**Depends on**: v1.0 (Phase 5 — the boot-r9 compose pipeline); the slirp/hostfwd leg can start alongside Phase 7; tap/vmnet acceptance needs Phase 8's ether-bound guest stack (this phase's tap leg is the venue for Phase 8's NETIP-08 exit gate)
**Requirements**: HNET-01, HNET-02, HNET-03, HNET-04, HNET-05
**Success Criteria** (what must be TRUE):
  1. The default boot is hermetic and CI-able: slirp with hostfwd bound to 127.0.0.1 for BOTH tcp/17010 and tcp/567 (drawterm dials the auth server directly), zero privilege required (HNET-01)
  2. `nix run .#boot-r9 -- --net=slirp|tap|vmnet` selects the mode explicitly with no silent auto-downgrade, and the netdev stanza is single-sourced as data in r9's platform TOML, consumed by Harvey's flake (HNET-02)
  3. The linux tap/bridge leg (qemu-bridge-helper + br0) is documented per-distro and tested — the guest is reachable at L2 from the host, providing the L2-truth venue for Phase 8's exit gate (HNET-03)
  4. The darwin real-L2 decision spike (socket_vmnet root daemon + fd handoff vs signed-qemu vmnet — builtin vmnet is entitlement-walled on nixpkgs qemu) is recorded, and the chosen leg is documented as opt-in with a volunteer-run validation log (HNET-04)
  5. A doctor-style checker diagnoses privileged host setup and prints exact sudo/bridge/firewall remediation commands (HNET-05)

**Plans**: TBD

### Phase 10: Userspace Services + GUI Tiers + Bootfs Payload

**Goal**: The services and GUI tiers ride Phase 6's machinery — session daemons, the IP suite, and the auth suite compile and donor-execute; the GUI stack compiles with donor-kernel syscall traces captured measure-first; fonts/profiles/ndb templates are staged into bootfs so Phases 11–12 never block on payload.
**Depends on**: Phase 6 (tier machinery + Wave-0 fixes are the entry equipment); parallel with Phases 7–9
**Requirements**: USPACE-07, USPACE-08, USPACE-09, DESK-03 (bootfs payload staged here per research — Phase 12 consumes it)
**Success Criteria** (what must be TRUE):
  1. The services tier is donor-executed green: aux/listen, mntgen, ndb/cs+dns, exportfs, ipconfig + ping, and the auth suite (~80 files) run on the 9front donor kernel with committed execution markers (USPACE-07)
  2. The GUI stack compiles for aarch64 (libdraw family + rio + acme + sam + plumb), and donor-kernel syscall traces for rio/acme/sam are captured and committed — the measure-first input for any r9 kernel growth the desktop needs (USPACE-08)
  3. The long tail is swept: upas + lp + the remaining cmd dirs compile; fossil + venti are compile-gated and documented as runtime-unverifiable this milestone (no block device in r9); gs (753 files) is included (USPACE-09)
  4. The bootfs payload is staged with manifest assertions: `/lib/font/bit/*`, a glenda-style lib/profile, and a `/lib/ndb/local` template are present in the baked image (DESK-03)

**Plans:** 10 plans (0/10 complete)
**Wave 1**
- [ ] 10-01-PLAN.md — Tier machinery extension to tiers 2/3/4 + the one-way grammar checkpoint + the cmd/aux:listen end-to-end tracer (USPACE-07; D-13/D-14)

**Wave 2** *(blocked on Wave 1)*
- [ ] 10-02-PLAN.md — DESK-03 bootfs payload: full 36MB fonts + EARLY 1024M headroom proof, trimmed glenda profile, ndb/local template substitution, manifest asserts (DESK-03; D-13/D-14/D-15)

**Wave 3** *(blocked on Wave 2)*
- [ ] 10-03-PLAN.md — D-01a vendored ape arm64 slice + per-arch ape headers + default.aarch64.h; libap.a wired; compress pcc-route tracer; gs wall-fall evidence (USPACE-09 enabler)

**Wave 4** *(blocked on Wave 3)*
- [ ] 10-04-PLAN.md — Tier-2 full roster (ip tree, ndb, aux/listen, mntgen, exportfs, auth suite) + driver completion + compile sweep + COMPILE-ONLY ledger (USPACE-07 compile half; D-06)

**Wave 5** *(blocked on Wave 4)*
- [ ] 10-05-PLAN.md — Tier-2 functional probes on the donor: cs/dns/exportfs/listen/mntgen/ipconfig+ping/auth-shallow (10 probes) + T2-OK:donor ledger supersedes (USPACE-07 execution half; D-03/D-05)

**Wave 6** *(blocked on Wave 5)*
- [ ] 10-06-PLAN.md — Phase-6 checklist: libmemdraw LP64 audit + drawtest/arctest rejoin donor-green; stale test targets; libdynld:128; rc argv-stack audit (USPACE-08 prerequisite)

**Wave 7** *(blocked on Wave 6)*
- [ ] 10-07-PLAN.md — Tier-3 GUI compile wave (rio/acme/sam/samterm/plumb/stats/winwatch/page/proof/screenlock) + the D-12 headless plumber probe (USPACE-08 compile half)

**Wave 8** *(blocked on Wave 7)*
- [ ] 10-08-PLAN.md — GUI trace rig: draw9 serve-only draw server + trace --gui + scripted-interaction traces of rio/acme/sam+samterm, darwin documented-manual (USPACE-08 trace half; D-08/D-09a/D-10a/D-11)

**Wave 9** *(blocked on Wave 8)*
- [ ] 10-09-PLAN.md — Tier-4 long tail: derived roster + games filter + mass compile + fix-or-exempt + coverage regen + gs donor render smoke (USPACE-09; D-01/D-02/D-07)

**Wave 10** *(blocked on Wave 9)*
- [ ] 10-10-PLAN.md — phase10.yml compile-gate CI sibling (D-11a) + 10-EVIDENCE.md consolidation + closeout declarations (all four requirements)
**UI hint**: yes

### Phase 11: In-Guest p9sk1 Auth Domain

**Goal**: The guest is a self-contained p9sk1 auth domain — one bake-time generator produces the machine key, `/adm/keys`, nvram-substitute, and ndb entries as a hash-paired unit so all four p9sk1 configs agree by construction; keyfs + authsrv serve it; and an in-guest loopback probe proves the handshake before any drawterm is ever attempted.
**Depends on**: Phase 8 (loopback `/net` — the AS dial) + Phase 10 (auth suite binaries) — the milestone's first hard cross-track join; this phase's loopback probe green is Phase 12's entry criterion
**Requirements**: AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05, AUTH-06, AUTH-07, AUTH-08, AUTH-09
**Success Criteria** (what must be TRUE):
  1. The bake-time domain generator exists and is the only way domains are made: machine key + `/adm/keys` + nvram-substitute + ndb entries produced as ONE hash-paired unit; the demo domain is loudly labeled DEMO and pairing verification fails loud (AUTH-01)
  2. r9's auth substrate is live with boot-test evidence: a DTB-seeded `/dev/random` CSPRNG for p9sk1 challenges and devcap `#¤` (caphash/capuse — the `auth_chuid` path the cpu server depends on) (AUTH-02, AUTH-03)
  3. keyfs starts non-interactively over the baked `/adm/keys`, authsrv listens on tcp/567, and the guest domain's ndb content (authdom, hostid, sys, ether) is staged in the image (AUTH-04, AUTH-05)
  4. The in-guest loopback p9sk1 probe is green BEFORE any drawterm attempt: auth/debug completes a full handshake over loopback, the non-hostowner speaksfor gate passes, and serial-visible auth logging has been present since day one (AUTH-06, AUTH-07)
  5. secstore is available for persistent key storage beyond the baked keys, and the writable-`/adm` + nvram-substitute scoping is decided and documented (in-phase decision spike: baked+paired for CI; host-9P `/adm` mount for real use) (AUTH-08, AUTH-09)

**Plans**: TBD

### Phase 12: CPU Service + Drawterm + rio Desktop

**Goal**: The milestone's payoff — `nix run .#boot-r9` remains the ONE command and now brings up the full system (net + auth + cpu listener); `nix run .#drawterm` connects from the host, authenticates, and lands the user in an interactive rio desktop session. **Work spans both repos plus the flake glue.**
**Depends on**: Phases 6–11 — strictly last (the dependency graph is a chain: desktop consumes cpu, cpu consumes auth, auth consumes net + userspace)
**Requirements**: CPU-01, CPU-02, CPU-03, CPU-04, DESK-01, DESK-02, DESK-04, DESK-05, DESK-06, DESK-07, INT-05, INT-06, INT-07 (DESK-03 staged in Phase 10)
**Success Criteria** (what must be TRUE):
  1. The cpu boot path is real: r9 execs `#!` scripts (tcp17010 is literally `#!/bin/cpu -R`, execl'd by listen), `service=cpu` arrives via plan9.ini in DTB `/chosen/bootargs` so cpurc runs instead of termrc, and aux/listen + tcp17010 are live at boot with exportfs served (r9 implements WSTAT/FWSTAT/REMOVE) (CPU-01, CPU-02, CPU-03)
  2. The full `cpu -R` remoteside flow is verified end-to-end with serial evidence: p9any→p9sk1 → auth_chuid → client exportfs mounted at /mnt/term → stdio from /mnt/term/dev/cons → `rc -li` (CPU-04)
  3. The flake drawterm app exists (`nix run .#drawterm`) with its provenance decision recorded (research recommends nixpkgs 9front drawterm + wrapper); the wrapper pins `-O`, scrubs `$cpu`/`$auth`/`$PASS`, and prints the exact connect line; the `-G` text-mode interop gate is green in CI via hostfwd (DESK-01, DESK-06)
  4. rio starts only in-session: profile `case cpu` binds /mnt/term/dev over /dev and execs rio (a serial marker proves rio did NOT start at boot); plumb runs in the drawterm session; and r9 devssl `#D` (rc4_256/sha1) is available for encrypted sessions via pushssl, with `-e clear` as the documented fallback (DESK-02, DESK-04, DESK-05)
  5. The graphical leg is human-accepted on darwin AND linux (window opens, mouse works, typed input lands); boot-r9-smoke is extended to the full chain; the r9 flake.lock pin advances past the networking commits as its OWN commit with a fresh hosted CI run (push-before-pin, never bundled); and the whole v2.0 chain reproduces from clean checkouts on darwin-aarch64 and linux (DESK-07, INT-05, INT-06, INT-07)

**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Two parallel tracks from the v1.0 foundation: userspace {6 → 10} ∥ networking {7, 8, 9} — Phase 8's loopback staging is decoupled from Phase 7's driver by design; Phase 8's exit gate and Phase 9's tap acceptance meet on the tap leg. Hard cross-track joins: Phase 11 (needs 8-loopback + 10) → Phase 12 (needs all). Decimal insertions slot numerically.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Build-Tail Green (9k amd64) | v1.0 | 4/4 | Complete | 2026-10-31 |
| 2. aarch64 Toolchain + libc | v1.0 | 7/7 | Complete | 2026-11-01 |
| 3. r9 qemu-virt Platform Bring-up | v1.0 | 6/6 | Complete | 2026-11-01 |
| 4. r9 Userspace-Exec + Harvey↔r9 ABI | v1.0 | 8/8 | Complete | 2026-11-02 |
| 5. Shell Boot + One-Command Integration | v1.0 | 13/13 | Complete | 2026-11-04 |
| 6. Full-Tree Userspace — Wave 0 + Libraries/Core | v2.0 | 10/10 | Complete    | 2026-11-05 |
| 7. r9 PCI/ECAM + virtio-net + devether | v2.0 | 10/10 | Complete    | 2026-11-05 |
| 8. r9 TCP/IP Path — devip-Compatible /net | v2.0 | 9/9 | Complete    | 2026-11-06 |
| 9. Host Networking + boot-r9 Net Modes | v2.0 | - | Not started | - |
| 10. Userspace Services + GUI Tiers + Bootfs | v2.0 | 0/10 | Planned | - |
| 11. In-Guest p9sk1 Auth Domain | v2.0 | - | Not started | - |
| 12. CPU Service + Drawterm + rio Desktop | v2.0 | - | Not started | - |

## Backlog

### Phase 999.1: Follow-up — Phase 06 deferred UAT follow-up: Test 1 (BACKLOG)

**Goal:** Resolve the UAT checkpoint deferred during Phase 06 verification
**Source phase:** 06
**Deferred at:** 2026-11-05 during /gsd-verify-work 06 session completion
**Follow-ups:**
- [x] Test 1: Fix the legacy harvey.yml workflow to build via nix so it is green moving forward (pre-dates this work; red streak on master recorded since 2026-11-04T12:01Z — pre-existing/unrelated to Phase 6) (deferred 2026-11-05) — workflow rewritten onto the nix pipeline 2026-11-05; hosted green run PENDING FIRST PUSH (261105-slx)
