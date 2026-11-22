# Phase 8: r9 TCP/IP Path — devip-Compatible /net - Research

**Researched:** 2026-11-05
**Domain:** Plan 9 devip `/net` file-interface reimplementation over smoltcp 0.14 in the r9 kernel (no_std Rust, aarch64 qemu `-M virt`), plus the Harvey-side gate payloads/harness
**Confidence:** HIGH — smoltcp 0.14.0 API verified against docs.rs pages AND the downloaded 0.14.0 crate source AND a no_std compile probe on r9's exact pinned toolchain (aarch64-unknown-none green); Harvey semantics read directly from `sys/src/9k/ip/` + `sys/man` + the consumer sources; r9 integration points read from `../r9` source.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**NETIP-01 — TCP implementation (the decision gate)**
- **D-01:** **smoltcp 0.14 behind a hand-built devip ABI** — over a staged Rust port of `sys/src/9k/ip` and over a hybrid. The `/net` file ABI is hand-built either way (the research's decisive finding: the file tree IS the contract — libc `dial()`/`announce()`/`listen()` are pure file I/O); smoltcp replaces the TCP/UDP/ICMP/ARP/ND machinery (Reno, timers, reassembly) with maintained, upstream-tested, `no_std` safe-Rust code. Verified current at discussion time: **0.14.0 (2026-08-17), MSRV 1.91** — cleared by r9's nightly-2026-01-30 pin. Diverges deliberately from Phase 7 D-01's from-scratch virtio choice: the virtio axis was transport machinery (small, auditable); the TCP axis is a correctness-sensitive protocol engine where upstream test coverage is the point. — **Reversibility:** costly — the devip↔smoltcp seam (event surface, socket-waker wakeups, neighbor-cache-as-`/net/arp`, Device adapter over devether) underpins every protocol gate; switching to the port later means re-homing all protocol engines behind the same file ABI.
- **D-02:** **Dependency entry = crates.io pin + committed Cargo.lock** — r9 `port/`'s first external crate. Not vendored in-repo, not git-SHA pinned. The lockfile hashes + the flake's fixed-output cargo vendor derivation are the supply-chain record. — **Reversibility:** reversible.
- **D-03:** **"devip-exact" proof = committed contract doc + targeted gates; NO donor A/B diff harness.** The contract doc derives from `sys/man` (`sys/man/3/ip`, `sys/man/2/ip`, `sys/man/8/ipconfig`) + `sys/src/9k/ip/` source and enumerates the full documented surface (per D-08); gates assert documented behaviors directly. — **Reversibility:** reversible — an A/B harness can be added later without invalidating anything.
- **D-04:** **NETIP-01 record placement follows the Phase 7 D-01 precedent**: an r9-side doc/commit plus this CONTEXT.md, recorded *together with* the virtio from-scratch provenance decision (NETIP-01 requires the same-axis decisions recorded together).

**smoltcp integration — event-driven (user-challenged, verified, locked)**
- **D-05:** **Event-driven integration — NO periodic polling.** The user challenged the polling assumption ("just because it has `Interface::poll()` doesn't mean polling is the only way"); verification against smoltcp 0.14 confirms it: no internal threads, no tick requirement. `Interface::poll()` is invoked **exactly on**: (1) devether RX interrupt (frame ingress), (2) a **one-shot timer armed from `poll_at()`/`poll_delay()`** after each poll (drives smoltcp's retransmit/keepalive timers), (3) local socket activity from devip file ops. **Socket wakers** (`WakerRegistration::register_recv_waker`/`register_send_waker`) wake blocked devip readers/writers directly — no blocked-process polling. The single-ownership context for `&mut Interface`+`SocketSet` is designed event-first; exact seam mechanics go to phase research with this constraint locked. — **Reversibility:** costly — the wakeup/timer seam is woven into devip's blocking file semantics; a periodic-cadence fallback would be a seam rewrite (the protocol machinery above it is unaffected).

**Protocol scope + IPv6**
- **D-06:** **Full-fidelity IPv6 — ether-leg with ICMPv6/Neighbor Discovery** via smoltcp `proto-ipv6`; the guest is v6-reachable from the host. The strongest reading of NETIP-06's "demonstrated"; overrides the research's port-path deferral of `ipv6.c`/`icmp6.c` (that deferral was written for the port's cost model, not smoltcp's).
- **D-07:** **netlog (`/net/log`) + flag-guarded serial debug prints land from day one** — Pitfall 15 posture: once workflows ride TCP, the stack bug wedges the channel used to observe it; observability is a deliverable, not an afterthought. Debug prints follow the repo convention (devcons-style static flag variables).
- **D-08:** **FULL documented devip surface is exact this phase** — all ctl verbs, all file formats, all error strings per `sys/man`, whether or not a milestone consumer touches them. Deliberate full-fidelity stance (consistent with D-06), overriding the measure-first subset recommendation (v1.0 D-14's discipline does not apply to the file-semantics surface here). The D-03 contract doc enumerates the complete surface from `sys/man` × `sys/src/9k/ip/` and lands FIRST; gates assert it. Dropped protocols (`gre/esp/rudp/igmp/ipmux/pktmedium/netdevmedium`) are simply absent from the file tree — no stub dirs.

**DHCP + addressing**
- **D-09:** **DHCP gate = Harvey's real `ip/ipconfig` dhcpclient over `/net/udp` broadcast fidelity** — the path `cpurc` will actually run in Phase 12. smoltcp's dhcpv4 socket is **never wired** unless the ipconfig path fails; if the fallback is ever taken it is recorded per NETIP-05's text. — **Reversibility:** reversible.
- **D-10:** **DHCP + static gates run on slirp now** (10.0.2.x, hermetic, unprivileged, CI-able); both are **re-demonstrated on the tap leg** when Phase 9 lands the venue. Every slirp-green gate is explicitly labeled not-L2-proof in the evidence (Pitfall 9).
- **D-11:** **Single committed addressing config seam** — one harness-side data file defines guest/host addresses per leg (loopback/slirp, later tap); baked ndb/cpurc fragments and gate assertions derive from it. Phase 9 lifts the seam into HNET-02's TOML single-sourcing — the Phase 7 D-09 pattern (minimal wiring now, flag shape Phase 9 will adopt). **No scattered `10.0.2.x` hardcodes** — Pitfall 9's named failure mode.

**Gate machinery + venue**
- **D-12:** **Gate binaries are Phase-8-owned bootfs payloads** — `ip/ipconfig`, `ip/ping`, and the echo/tcpblast binaries are built by phase-8 harness rules and baked into bootfs per the Phase 7 `arp.req`/`netblast.c` precedent. **NOT tier-allowlist appends** — Phase 6's tier data files stay pure; Phase 10's tier-2 services sweep remains the tier entry point for `sys/src/cmd/ip/`.
- **D-13:** **Layered gates** (v1.0 D-12 + Phase 6 D-12 hold): r9 xtask boot-test gains net profiles asserting kernel/stack serial markers (devip bind, medium registration, smoltcp poll health, netlog activity); Harvey gains a composed **expect-gated** smoke sibling driving the user-visible flows (`ping 127.0.0.1`, udp echo, in-guest tcp/7 echo, ipconfig static + dhcp runs) — NETIP-03's "committed expect gates" wording.
- **D-14:** **aan (NETIP-07) = compile for aarch64 + scripted reconnect gate on loopback**: aan client+server pair in-guest over tcp loopback, scripted hangup of the underlying `/net/tcp/N` conn, gate asserts the tunneled session resumes. The value proposition proven NIC-free; cpu-service integration stays Phase 12.
- **D-15:** **NETIP-08 sequencing: machinery + slirp soak in Phase 8; tap verdict in Phase 9.** Phase 8 builds the bidirectional TCP soak machinery (tcpblast-style payload, host-asserted via pcap + guest counters, TCG-scaled counts — the Phase 7 netblast precedent) and proves it on slirp, explicitly labeled not-L2-truth. The tap-leg truth verdict executes in Phase 9 as a **joint Phase-8/9 acceptance gate** using this phase's machinery — the roadmap's own cross-phase framing ("this phase's tap leg is the venue for Phase 8's NETIP-08 exit gate").
- **D-16:** **CI shape: short gates in hosted CI; the sustained soak is documented-manual on dev hosts.** Short gates = loopback ping/echo, ipconfig static+dhcp, brief TCP exchange — extending `r9-shell.yml` per the Phase 6/7 precedent. Deliberate override of Phase 6 D-16's "everything in CI unless measured over-budget" **for the soak specifically**: the aarch64-guest soak is TCG on every automated venue (x86_64 CI; darwin HVF rejects GICv2), and CI wall-clock honesty beats a flaky mega-gate. The exact dev-host soak command must be documented in the phase evidence or the soak silently never runs. — **Reversibility:** reversible — a TCG-scaled CI soak can be added if the manual discipline rots.

**Carried forward (locked in roadmap/prior phases — do not re-litigate)**
- Loopback + null mediums FIRST, before any ether binding; the ether binding needs Phase 7's devether (roadmap goal + depends-on).
- libc `dial()`/`announce()`/`listen()` unmodified — no network syscalls added; the file tree IS the contract (NETIP-02).
- Exit-gate truth venue is tap; slirp evidence never accepted as L2 proof (NETIP-08, Pitfall 9).
- DHCPv6/SLAAC out of scope; IPv6 = devip ip6 + static/ipconfig configuration (NETIP-06 + REQUIREMENTS Out-of-Scope table) — D-06 sets the depth within that boundary.
- virtio provenance: from-scratch `port/src/virtio.rs` (Phase 7 D-01) — recorded together with D-01 per NETIP-01.
- Phase 7 conventions: DTB-driven discovery, zero hardcoded addresses/IRQs; zero-offload mask (MAC+STATUS only); barrier-first aarch64 ring code; INTx level-triggered SPIs + ISR read-to-ack; **12-byte `virtio_net_hdr` under VERSION_1** (07-RESEARCH errata); NIC always-on for aarch64 virt boots (Phase 7 D-10).
- r9-side gates stay r9-side; r9 commits land on `main` of `requaos/r9` under push-before-pin; Harvey `flake.lock` r9 pin advances as its OWN commit with a fresh hosted CI run, never bundled.
- Evidence consolidated in `.planning/phases/08-*/` with the r9 commit SHA recorded; markers timing-tolerant (presence, never interleaving order).
- Bake-into-bootfs for gates (deterministic), ufs-over-tcp for iteration; the `mkimage.sh` real-init rodata guard applies to all payloads.
- qemu shape for r9 gates: `-M virt,gic-version=2`; TCG on darwin; `-m ≤ 1G`.
- raspi3b/raspi4b/x86_64/riscv64 targets stay green — the `port` crate is shared; virt-only additions are cfg-gated.
- `aarch64` naming everywhere; r9 follows its `rustfmt.toml`; unsafe posture per `../r9/UNSAFE-AUDIT.md` — smoltcp is safe Rust (no new unsafe from the dependency); the devether DMA boundary is unchanged.

### the agent's Discretion
- smoltcp feature set within the locked shape (`medium-ethernet` + `proto-ipv4` + `proto-ipv6` + `socket-tcp`/`udp`/`icmp` + `alloc`; `socket-dhcpv4` stays disabled/unwired per D-09) and the exact 0.14.x pin — researcher verifies against docs.rs.
- The event-driven seam mechanics (D-05's constraint is locked, the design is open): ownership context for `&mut Interface`+`SocketSet`, the waker→Rendez bridge, one-shot timer wiring off `poll_at()`.
- devip module layout in `../r9/port/src/` (`devip.rs` + protocol modules) and how the `Fs`/`Proto`-style dispatch maps onto the file tree.
- `/net/arp` ctl-verb depth as a view over smoltcp's neighbor cache — D-08's full-fidelity stance governs whatever `sys/man` documents.
- Medium registration/bind order within the locked loopback+null-first sequence; ethermedium's devether↔smoltcp `Device` adapter shape.
- tcpblast payload shape (name, loop counts, marker vocabulary, TCG-scaled counts) — netblast precedent.
- Addressing seam file format/location (D-11's constraint locked).
- CI job placement within `r9-shell.yml` (D-16's constraint locked).
- The contract doc's home (r9-side `docs/` recommended) and enumeration format.
- Payload build mechanics for ipconfig/ping/echo/tcpblast (Phase 7 payload precedent).

### Deferred Ideas (OUT OF SCOPE)
- Host netdev modes `--net=slirp|tap|vmnet`, netdev stanza TOML single-sourcing (HNET-02), linux tap/bridge docs, darwin socket_vmnet, doctor-checker — Phase 9 (HNET-01..05); Phase 9 also executes this phase's NETIP-08 tap verdict (D-15)
- cs/dns/ndb service integration + name-resolution gates — Phase 10 (services tier) / Phase 11 (auth domain); Phase 8 gates are numeric-only
- cpu service + drawterm + rio — Phase 12 (aan's real venue; D-14's loopback gate is the Phase-8 proof)
- auth/devcap/`#!` exec/`/dev/random` — Phase 11; virtio-rng-pci — later (carried from Phase 7)
- smoltcp dhcpv4 socket wiring — only if D-09's ipconfig path fails; the recorded fallback per NETIP-05
- `gre/esp/rudp/igmp/ipmux/pktmedium/netdevmedium` protocols — dropped per research; no milestone consumer; absent from the file tree (D-08)
- Donor A/B diff harness for devip fidelity — rejected (D-03); contract doc + targeted gates instead
- TCG-scaled CI soak leg — rejected for now (D-16 override); revisit if the manual discipline rots
- virtio-blk, virtio-9p host mount, SMP/PSCI, RTC — future milestones (carried)
- EVENT_IDX notification suppression; MSI/GICv2m — carried from Phase 7 deferred (non-goals for v2.0)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NETIP-01 | TCP implementation decision FINAL and recorded | D-01 resolved it (smoltcp 0.14); this research supplies the verification record: 0.14.0 published 2026-08-17, MSRV 1.91, no_std compile probe GREEN on r9's pinned nightly-2026-01-30 for `aarch64-unknown-none` with the locked feature set (see Standard Stack). The r9-side record lands per D-04 with the Phase 7 D-01 virtio provenance note. |
| NETIP-02 | `/net` devip-exact: tcp/udp/icmp clone/ctl/data/status + ipifc; loopback+null FIRST | The complete enumerated surface is in §"The devip Contract Surface" (the D-03 contract doc's table of contents): qid scheme, per-file semantics, ctl verbs, error strings, status/stats formats — read from `devip.c`, `tcp.c`, `udp.c`, `icmp.c`, `icmp6.c`, `ipifc.c`, `iproute.c`, `arp.c`, `netlog.c`, the mediums, and `sys/man/3/ip`. libc announce/listen/dial exact call sequences verified (`sys/src/libc/9sys/announce.c`). |
| NETIP-03 | TCP debuggable without the NIC: ping 127.0.0.1, udp echo, tcp/7 echo over loopback | Loopback design: smoltcp `phy::Loopback` exists but allocates per packet — ship a small fixed-pool `Medium::Ip` Device instead (§Architecture Patterns). ping's loopback path proven against smoltcp source: iface auto-echo-reply + icmp socket `Endpoint::Ident` accepts both EchoRequest and EchoReply by ident (`socket/icmp.rs` accepts_v4/v6). Event seam carries it (no NIC IRQ needed — loopback "RX" is synchronous TX-loop; the poll still runs from the file-op context). |
| NETIP-04 | arp + ethermedium bound to devether; static ipconfig assigns IPv4; guest reachable | The ethermedium→netif-conversation binding (0x800/0x806/0x86dd) maps to a smoltcp `Device` adapter over Phase 7's `netif` machinery; `/net/arp` is a devip-side snoop-fed table (smoltcp's neighbor cache has NO public accessor — verified). ipconfig's exact ctl surface enumerated (bind/add/remove/unbind/iproute/ndb). |
| NETIP-05 | DHCP via ipconfig `/net/udp` broadcast fidelity (or recorded fallback) | ipconfig's DHCP path fully traced (announce `udp!*!68` → `headers` → full-size Bootp rd/wr with alarm timeouts; `add 0.0.0.0 0.0.0.0` preconfig). smoltcp UDP RX broadcast acceptance verified (`socket/udp.rs` accepts()); the src-0.0.0.0 TX question and the `update_ip_addrs` non-unicast panic are the named fidelity risks with an early host-side pin prescribed (§Pitfalls P-F/P-I, §Open Questions OQ-1). |
| NETIP-06 | IPv6: devip ip6 tree + ipconfig IPv6 configuration | smoltcp `proto-ipv6` + ND on `medium-ethernet`; SLAAC stays OFF (Config.slaac=false; Out-of-Scope table). The icmpv6 full-IPv6-packet data semantics + `headers` mode + ipconfig's `try`/`add6`/`ra6`/DAD-arp-scan surface enumerated from `icmp6.c`/`ipv6.c` — a named mapping corner (Pitfall P-E). |
| NETIP-07 | aan available for cpu connections | aan mechanics fully mapped from `sys/src/cmd/aan.c` (12-byte LE hdr, 8s sync, reconnect-on-failure, listen/accept/local/remote reads, "connection refused"→exit client behavior). The D-14 gate's scripted drop = `hangup` write to `/net/tcp/N/ctl` (tcp.c tcpctl: hangup→RST). Dependencies: libthread (proccreate/alt/Channel), alarm, sleep, atnotify, syslog (non-fatal if /sys/log absent). |
| NETIP-08 | sustained bidirectional TCP + load soak; tap = truth (Phase 9 executes), slirp = machinery proof | tcpblast payload pattern per the Phase 7 netblast precedent; host-asserted via the already-landed qemu filter-dump pcap machinery (r9 xtask `mod pcap`) + guest counters; smoltcp RTO floor (1s min/initial, 60s max) documented for TCG-scaled counts. |
</phase_requirements>

## Summary

Phase 8 puts a Plan 9 `/net` in the r9 kernel: a hand-built devip-exact file tree (`#I`) whose protocol engines are smoltcp 0.14.0 sockets behind an event-driven seam (D-01, D-05). The phase's work splits cleanly into four independently testable layers: **(1)** the D-03 contract doc — the enumerated devip surface from `sys/man` × `sys/src/9k/ip/`, landing FIRST (D-08 makes it the phase's cost center); **(2)** the smoltcp seam — a single-ownership context for `Interface`+`SocketSet`+device-adapter, driven exactly by RX-IRQ wakes, a one-shot deadline armed from `poll_at()`, and local file-op activity, with socket wakers waking blocked readers; **(3)** the devip file tree itself — conv tables, perms/ownership collapse, clone/listen/err semantics — all hand-built, all host-testable; **(4)** the mediums in locked order: loopback (TCP debuggable NIC-free, NETIP-03) → null → ether (devether bound, NETIP-04..06) → the aan and tcpblast gates (NETIP-07/08 machinery).

The decisive verified facts: smoltcp 0.14.0's `Interface` is `Send+Sync`, has no threads and no tick requirement, and exposes `poll()` (returns `PollResult`, not `bool` — 0.14 changed this), `poll_at()`, `poll_delay()`, plus bounded-work variants. Socket wakers (`register_recv_waker`/`register_send_waker`) exist directly on tcp/udp/icmp sockets with one-shot, single-waker, spurious-allowed semantics — a precise fit for r9's register-then-`block()` hook idiom. r9 already has every integration primitive the seam needs: the `EtherHooks {current, block, wake}` registration pattern (devpipe/devether), the IRQ↔syscall BusyCell handoff (netif's INGRESS), a 1 ms generic-timer tick with a deadline sweep (`proc::tick_timeouts`), and IRQ-masked syscall context (the Phase-4 invariant) that makes register-then-block wakeup-loss-free by construction.

The load-bearing surprises the planner must design around: **(a)** smoltcp's neighbor cache has no public accessor — `/net/arp` is a devip-side table fed by ctl writes and adapter snooping of ARP/NDP frames (a documented divergence from 9k, where static `add` steers real resolution); **(b)** smoltcp TCP listen has NO backlog queue — excess SYNs are RSTed — so devip's 128-deep incall queue maps onto an accept-pool of pre-listening sockets; **(c)** ipconfig calls `truerand()` unconditionally at startup and libc's `truerand` sysfatals without `/dev/random` — a Phase-11 (AUTH-02) adjacency that Phase 8 must resolve early (minimal devcons `random` file, or an explicit phase-boundary decision); **(d)** devip's icmpv6 `data` carries full IPv6 packets (userspace builds the IP6 header) while smoltcp's icmp socket is payload-level — the bridge parses/synthesizes IP6 headers; **(e)** smoltcp `update_ip_addrs` panics on non-unicast addresses, so ipconfig's DHCP preconfig `add 0.0.0.0 0.0.0.0` must be absorbed devip-side, never passed through; **(f)** smoltcp 0.14's TCP RTO floor is 1 s (min == initial), which sizes TCG-scaled gate timeouts.

**Primary recommendation:** Land the contract doc first (D-03/D-08), then build the seam host-side with unit pins (`allocator_tests.rs` precedent — smoltcp compiles for the host, so the ENTIRE devip↔smoltcp mapping is host-testable before the first guest boot), then loopback/null mediums with the NETIP-03 expect gates, then the ether binding.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `/net` file tree semantics (clone/ctl/data/status/err/listen/local/remote, perms, conv lifecycle) | Kernel (r9 `port/src/devip*`) | — | The file tree IS the contract (NETIP-02); r9 owns all device trees |
| TCP/UDP/ICMP/ICMPv6 protocol engines | smoltcp (crates.io dependency, `port/` guest) | — | D-01: maintained upstream-tested safe Rust replaces Reno/timers/reassembly |
| ARP/ND protocol machinery (request/reply on the wire) | smoltcp (`medium-ethernet` internal) | — | `DeviceCapabilities.medium = Medium::Ethernet` triggers ARP/NDISC in smoltcp |
| `/net/arp` file (read format, add/del/flush verbs) | Kernel devip (snoop-fed view table) | smoltcp (invisible internal cache) | smoltcp 0.14 exports NO neighbor-cache accessor — verified against the 0.14.0 `iface` module item list |
| Routing table (`/net/iproute`) | Kernel devip (the file + the table of record) | smoltcp `Routes` (default v4/v6 route only) | devip's route forest (tags, flags, per-ifc) is richer than smoltcp's `heapless::Vec<Route, 4>`; devip owns the file semantics and mirrors the default route into smoltcp |
| Address assignment (`/net/ipifc` add/remove/add6/ra6) | Kernel devip (file semantics, lifc lists, DAD-visible `try`) | smoltcp (`update_ip_addrs`, ND) | devip owns the interface/address model; smoltcp learns addresses through the interface |
| DHCP client | Harvey `ip/ipconfig` UNMODIFIED (userspace payload) | — | D-09: the gate is the real binary over `/net/udp` broadcast fidelity; smoltcp dhcpv4 socket stays unwired |
| Frame RX/TX on the wire | Phase 7 devether/virtio (IRQ retire, ring) | smoltcp Device adapter (the seam) | The DMA boundary is unchanged; the adapter bridges netif conversations ↔ smoltcp tokens |
| Stack driving (poll scheduling) | Kernel event seam (IRQ wake + one-shot deadline + file-op activity) | — | D-05: no periodic polling; no kprocs (9k's kproc model does NOT carry) |
| Gate payloads (ping/ipconfig/echo/tcpblast/aan) | Harvey userspace, baked into bootfs | — | D-12: Phase-8-owned payload rules, NOT tier allowlists |
| Host netdev (slirp now; tap verdict) | qemu `-netdev user` (Phase 7 stanza, unchanged) | Phase 9 tap | D-10/D-15: slirp is hermetic CI; tap truth verdict is Phase 9's joint gate |
| Observability (`/net/log`, debug prints, pcap) | Kernel netlog + flag-guarded prints (D-07) | qemu filter-dump (already landed) | Pitfall 15: the debugging channel must not ride the stack being debugged |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `smoltcp` | **0.14.0** (published 2026-08-17; crates.io checksum `b6f8b28a…01980f`) [VERIFIED: crates.io API + docs.rs + compile probe] | TCP/UDP/ICMP/ICMPv6/IPv4/IPv6/ARP/ND protocol engines behind the devip ABI | D-01's locked choice; `no_std`, heap-optional, `#![deny(unsafe_code)]` in the crate itself, MSRV 1.91 cleared by r9's nightly-2026-01-30 (1.95.0-nightly), 0BSD license |
| `managed` | 0.8.0 (transitive) [VERIFIED: compile probe Cargo.lock] | smoltcp's owned/borrowed storage abstraction | smoltcp's `alloc` feature is `managed/alloc` |
| `heapless` | 0.9.3 (transitive) [VERIFIED: compile probe Cargo.lock] | Compile-time-sized collections (iface address list, route table, neighbor cache) | smoltcp 0.13 bumped heapless to 0.9; sizing feature flags (`iface-max-addr-count-*` etc.) ride it |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `bitflags` | 1.3.2 (transitive; already r9's dep at 2.5 for port) [VERIFIED: compile probe] | smoltcp wire flags | — (transitive; note the version split with port's bitflags 2.5 — cargo unifies per major) |
| `byteorder` | 1.5.0 (transitive) [VERIFIED: compile probe Cargo.lock] | Wire endianness helpers | — (transitive) |
| `cfg-if` | 1.0.5 (transitive) | cfg ergonomics | — (transitive) |
| `defmt` | 1.2.0 (optional transitive — in the lockfile, NOT compiled) [VERIFIED: compile probe] | Embedded logging | Never — keep the `defmt` feature OFF (r9 uses `println` to serial; D-07's debug prints follow devcons convention) |
| smoltcp `phy::Loopback` | (in-crate) | FIFO loopback Device | **Do NOT use as-is**: its `TxToken::consume` allocates (`vec![0; len]`) per packet [VERIFIED: crate source `src/phy/loopback.rs`]. Write a small fixed-pool `Medium::Ip` Device instead (Architecture Patterns) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| smoltcp 0.14 | Staged Rust port of `sys/src/9k/ip` | Rejected by D-01 (costly to reverse; the seam underpins every gate) |
| smoltcp 0.14 | `socket-dhcpv4` for NETIP-05 | Rejected by D-09 unless the ipconfig path fails; `socket-dhcpv4` = `[socket, medium-ethernet, proto-dhcpv4]` [VERIFIED: crates.io feature list] stays OUT of the feature set |
| `socket-tcp-reno` | `socket-tcp-cubic` | Rejected for the kernel: 0.14's Cubic needs `f64` (smoltcp's own docs warn against FPU in kernel/IRQ contexts) [VERIFIED: docs.rs tcp::Socket::set_congestion_control]. Reno is also devip's heritage (`tcp.c` is Reno). Enable `socket-tcp-reno`, never cubic |
| smoltcp `auto-icmp-echo-reply` (default-on) | `no-auto-icmp-echo-reply`-style off | Keep ON: devip's kernel answers echo requests itself (`icmp.c`), and ping-over-loopback was verified to work with it on (iface auto-replies; the reply dispatches to the Ident-bound socket) [VERIFIED: crate source `src/iface/interface/ipv4.rs:346-360` + `src/socket/icmp.rs` accepts_v4] |

**Installation (D-02 shape):**
```toml
# ../r9/port/Cargo.toml
[dependencies]
smoltcp = { version = "0.14.0", default-features = false, features = [
    "medium-ethernet", "medium-ip",
    "proto-ipv4", "proto-ipv6",
    "proto-ipv4-fragmentation", "proto-ipv6-fragmentation",
    "socket-tcp", "socket-udp", "socket-icmp", "socket-tcp-reno",
    "alloc", "multicast", "auto-icmp-echo-reply",
] }
```
then commit `../r9/Cargo.lock`. `medium-ip` is needed for the loopback medium's Device; `multicast` keeps IPv6 ND (multicast-group joins) working on the ether leg [VERIFIED: crates.io 0.14.0 feature graph: `medium-ethernet = ["socket"]`; `medium-ip = ["socket"]`; `proto-ipv6-slaac = ["proto-ipv6"]` stays OFF per Out-of-Scope].

**Version verification (done, not delegated):** the exact manifest above was compile-probed GREEN on 2026-11-05 with r9's pinned toolchain (`cargo 1.95.0-nightly-2026-01-30`) for BOTH the host (`aarch64-apple-darwin`) and the kernel target (`aarch64-unknown-none`), `#![no_std]`, exercising `Config::new`, `Interface::new(config, &mut dev, Instant::ZERO)`, `poll`/`poll_at`/`poll_delay`, `SocketSet::new(Vec::new())`, and the `Device` GAT surface. Lockfile closure: smoltcp 0.14.0, managed 0.8.0, heapless 0.9.3 (+hash32 0.3.1, +stable_deref_trait 1.2.1), bitflags 1.3.2, byteorder 1.5.0, cfg-if 1.0.5, defmt 1.2.0 (optional, uncompiled).

## Package Legitimacy Audit

Gate run via `gsd-tools query package-legitimacy check --ecosystem crates` on 2026-11-05; registry existence cross-checked by the compile probe (real download + build).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| smoltcp | crates.io | ~10 yrs (2016-12) | 207k/wk | github.com/smoltcp-rs/smoltcp | OK | Approved |
| managed | crates.io | ~10 yrs (2016-12) | 330k/wk | github.com/m-labs/rust-managed | OK | Approved |
| heapless | crates.io | ~9 yrs (2017-04) | 4.4M/wk | github.com/rust-embedded/heapless | OK | Approved |
| hash32 | crates.io | ~8 yrs (2018-04) | 4.35M/wk | github.com/rust-embedded-community/hash32 | OK | Approved |
| bitflags | crates.io | ~11 yrs (2015-01) | 43.4M/wk | github.com/bitflags/bitflags | OK | Approved |
| byteorder | crates.io | ~11 yrs (2015-02) | 15.5M/wk | github.com/BurntSushi/byteorder | OK | Approved |
| cfg-if | crates.io | ~11 yrs (2015-07) | 32.9M/wk | github.com/rust-lang/cfg-if | OK | Approved |
| stable_deref_trait | crates.io | ~9 yrs (2017-03) | 19.0M/wk | github.com/storyyeller/stable_deref_trait | OK | Approved |
| defmt | crates.io | ~6 yrs (2020-08) | 5.8M/wk | github.com/knurling-rs/defmt | OK | Approved (optional dep — feature OFF, never compiled) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

No postinstall/build-script network concerns: smoltcp ships a `build.rs` (feature-constant generation only — the compile probe ran it; no network or out-of-tree filesystem access observed). crates.io packages cannot carry postinstall hooks.

## smoltcp 0.14 — the Verified Integration Surface

All claims below verified 2026-11-05 against docs.rs/0.14.0 pages AND the downloaded `smoltcp-0.14.0` crate source (compile-probed). Where 0.14 differs from older-version memory, the difference is flagged — stale memory is a named trap.

### Interface driving (D-05's locked surface)

```rust
// VERIFIED: docs.rs/smoltcp/0.14.0/smoltcp/iface/struct.Interface.html
impl Interface {
    pub fn new(config: Config, device: &mut impl Device + ?Sized, now: Instant) -> Self;
    pub fn poll(&mut self, timestamp: Instant, device: &mut impl Device + ?Sized,
                sockets: &mut SocketSet<'_>) -> PollResult;
    pub fn poll_egress(&mut self, timestamp: Instant, device, sockets) -> PollResult;       // bounded
    pub fn poll_ingress_single(&mut self, timestamp: Instant, device, sockets) -> PollIngressSingleResult; // ≤1 packet
    pub fn poll_maintenance(&mut self, timestamp: Instant);                                  // bounded
    pub fn poll_at(&mut self, timestamp: Instant, sockets: &SocketSet<'_>) -> Option<Instant>;
    pub fn poll_delay(&mut self, timestamp: Instant, sockets: &SocketSet<'_>) -> Option<Duration>;
    pub fn context(&mut self) -> &mut iface::Context;   // tcp connect() needs this
}
```

- `PollResult { None, SocketStateChanged }` — **0.14 changed `poll()`'s return from `bool` to `PollResult`** [VERIFIED: docs.rs + crate source `iface/interface/mod.rs:77-82`]. `None` = socket state guaranteed unchanged; `SocketStateChanged` = re-check sockets.
- `Interface::new` takes the `device` reference at construction (medium/capability check; panics if `Config::hardware_addr` mismatches the medium) — since 0.11, not the 0.10-and-earlier `new(config, now)` shape.
- `poll()` **DoS warning** (docs.rs, verbatim): it "processes all packets in the device's queue. This can be an unbounded amount of work" — the bounded alternatives exist for exactly our IRQ-adjacent posture. With the adapter's queue bounded by the virtio ring (MAX_QSIZE 256, DRAIN_CAP precedent) a full `poll()` is bounded in practice; prefer `poll_ingress_single` loops with an explicit cap anyway (the gate-visible bound).
- `poll_at` = *soft deadline* for the next poll: "harmless (but wastes energy) to call before… potentially harmful (impacting quality of service) to call after" [VERIFIED: docs.rs]. This is the one-shot timer's arming source (D-05 leg 2). 0.13.0 fixed a `poll_at` stale-timestamp bug (#1127, `silent_until` expiry) — another reason the pin is 0.14, not older.
- 0.13.0 "Exhaust egress socket state on `poll`" (#1059): one poll after local activity fully flushes egress — the file-op-driven leg needs exactly one poll per op, not a drain loop.
- No internal threads, no tick requirement: poll is entirely caller-driven (D-05's premise, verified).
- `Interface` is `Send + Sync` (auto-traits listed on docs.rs) — it *may* live behind r9's lock types; the locking choice is ours (seam section).

### Time

`Instant::ZERO`, `Instant::from_millis(i64)`, `Instant::from_micros(i64)`; `Duration::from_micros/millis` [VERIFIED: crate source `src/time.rs:30-44,189`]. r9's `platform::timer::ticks()` is 1 ms/tick (TICKS_PER_SEC=1000, `timer.rs:23-42`), so `Instant::from_millis(ticks())` is the whole clock bridge. CNTFRQ is read at runtime (62.5 MHz TCG / 24 MHz HVF both measured — never hardcode; `timer.rs:81-87`).

### The `Device` trait (the devether adapter's surface)

```rust
// VERIFIED: docs.rs/smoltcp/0.14.0/smoltcp/phy/trait.Device.html + crate source phy/mod.rs:396-423
pub trait Device {
    type RxToken<'a>: RxToken where Self: 'a;
    type TxToken<'a>: TxToken where Self: 'a;
    fn receive(&mut self, timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)>;
    fn transmit(&mut self, timestamp: Instant) -> Option<Self::TxToken<'_>>;
    fn capabilities(&self) -> DeviceCapabilities;
}
pub trait RxToken { fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R; }      // infallible since 0.9
pub trait TxToken { fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R; }
```

- `receive` returns a PAIR (Rx+Tx) so a reply can be built from RX content without heap (the ICMP echo case).
- `DeviceCapabilities { medium, max_transmission_unit, max_burst_size, checksum }` (`#[non_exhaustive]`, `Default` exists). **MTU convention trap**: for Ethernet devices the MTU INCLUDES the 14-byte Ethernet header and excludes FCS — so 1514, matching devether's `ETHERMAXTU`/`mtu=1514` exactly [VERIFIED: docs.rs DeviceCapabilities + r9 `netif.rs:158-160`]. `medium: Medium::Ethernet` is what switches on ARP/NDISC (verified: "ARP/NDISC address resolution is only done for Ethernet mediums").
- `checksum: ChecksumCapabilities` — leave at the default (software checksums); Phase 7's zero-offload mask (MAC+STATUS only) means the hardware computes nothing, and 0.14's new `segmentation-offload` feature/TSO support stays OFF (Pitfall 6 closed by construction).
- `Device` is NOT dyn-compatible (GATs) — the adapter is a concrete type; multiple mediums = an enum or generic sites, not `dyn Device`.

### Socket layer

- `SocketSet::new(Vec::new())` (owned storage under `alloc`), `add() -> SocketHandle`, `get`/`get_mut` (panic on wrong type/handle), `remove`, `iter` [VERIFIED: docs.rs SocketSet]. The conv→SocketHandle map lives in the devip conv struct.
- `socket::Socket` enum variants (feature-gated): Raw/Icmp/Udp/Tcp/Dhcpv4/Dns [VERIFIED: crate source `socket/mod.rs`].
- **tcp::Socket** [VERIFIED: docs.rs tcp Socket page]: `new(SocketBuffer::new(vec![0; rx]), SocketBuffer::new(vec![0; tx]))`; `listen(endpoint) -> Result<(), ListenError>`; `connect(iface.context(), remote, local)`; `close()` = graceful FIN (devip's `close`); `abort()` = instant close + RST (devip's `hangup`); `state() -> tcp::State`; `is_open/is_active/is_listening/may_send/may_recv/can_send/can_recv`; `send_slice/recv_slice/peek_slice`; `set_keep_alive(Option<Duration>)` (devip `keepalive [n]`, default 30000 ms); `set_timeout`; `set_hop_limit` (default 64 = MAXTTL); `set_nagle_enabled` (default on); `set_ack_delay` (default 10 ms); `set_congestion_control(CongestionControl::Reno)` with `socket-tcp-reno`.
- **TCP `State` enum** [VERIFIED: crate source `socket/tcp.rs`]: `Closed, Listen, SynSent, SynReceived, Established, FinWait1, FinWait2, CloseWait, Closing, LastAck, TimeWait` — 11 states, a 1:1 mapping to devip's `tcpstates[] = {"Closed","Listen","Syn_sent","Syn_received","Established","Finwait1","Finwait2","Close_wait","Closing","Last_ack","Time_wait"}` [VERIFIED: `tcp.c:94-98`]. The status file's first field is a pure rename table.
- **The listen-backlog finding** (docs.rs tcp::Socket, verbatim): "for listening sockets, there is no 'backlog'; to be able to simultaneously accept several connections, as many sockets must be allocated, or any new connection attempts will be reset." devip queues up to `Maxincall = 128` pending incoming calls per announced conv [VERIFIED: `ip.h:40`]. The bridge = an **accept pool**: N smoltcp sockets all `listen(port)`; smoltcp's `accepts()` lets the first still-`Listen` socket absorb each SYN (tuple unset), then it transitions `SynReceived` with the 4-tuple bound [VERIFIED: `socket/tcp.rs` accepts()]. devip's `listen`-file open pops the oldest connected pool member and the pool refills. Pool sizing is a contract-doc decision (128 is devip-exact; a smaller committed pool is a documented deviation — D-08 pushes toward 128).
- **TCP RTO bounds** [VERIFIED: crate source `socket/tcp.rs:144,155,159`]: `RTTE_INITIAL_RTO = 1000ms`, `RTTE_MIN_RTO = 1000ms`, `RTTE_MAX_RTO = 60_000ms`. Compare devip `tcpsettimer`: bounded 0.3–64 s [VERIFIED: `tcp.c:3407-3422`]. The 1 s floor sizes TCG-scaled gate timeouts (a single dropped segment costs ≥1 s). `set_min_rto`/`set_initial_rto` exist only POST-0.14 (Unreleased) — not available; do not design against them.
- **udp::Socket**: `new(PacketBuffer::new(meta_vec, payload_vec), …)`; `bind(IpListenEndpoint)`; `send_slice(data, UdpMetadata)`; `recv_slice() -> (usize, UdpMetadata)` where UdpMetadata carries remote endpoint + local addr + (0.13) stored local address — everything needed to synthesize devip's 52-byte `Udphdr` headers mode (`raddr[16] laddr[16] ifcaddr[16] rport[2] lport[2]`, man/3/ip UDP section) [VERIFIED: docs.rs udp Socket + crate source].
- **UDP broadcast RX** (D-09's fidelity crux): `accepts()` matches on `dst_port`, and when bound to a specific addr still accepts broadcast/multicast destinations [VERIFIED: crate source `socket/udp.rs` accepts()]. A port-68-anyaddr socket receives DHCP OFFERs on slirp. The iface IP layer accepts broadcast-destined packets ("Ignore IP packets not directed at us, or broadcast, or any of the multicast groups" — `ipv4.rs:187-189`).
- **icmp::Socket**: `bind(icmp::Endpoint::Ident(u16))` — accepts EchoRequest AND EchoReply with matching ident [VERIFIED: `socket/icmp.rs` accepts_v4/v6]; also `Endpoint::Udp(port)`/`Endpoint::Tcp(port)` receive associated ICMP errors (0.13 added TCP) — the hook for devip's UDP `err`-file advice (`udpadvise` qhangup) if wired; the contract doc decides whether err-file ICMP advice is in the D-08 surface for the gates or listed-but-ungated.
- **Wakers** [VERIFIED: docs.rs, identical text on tcp/udp/icmp socket pages]: `register_recv_waker(&mut self, waker: &core::task::Waker)` and `register_send_waker` exist **directly on each socket type** — there is no standalone public `WakerRegistration` type in 0.14's API surface (the D-05 text names it; the methods are per-socket). Semantics: **one waker per direction at a time** (re-register overwrites); **woken ONCE** (must re-register after every wake); **spurious wakes allowed** (a wake doesn't guarantee progress — always re-check `can_recv`/`can_send`).

### Configuration + sizing (the numbers the planner needs)

- `Config::new(HardwareAddress)`; fields `random_seed: u64` ("strongly recommended… different on each boot"; needs no crypto), `hardware_addr`, `pan_id` (802.15.4 — None), `slaac: bool` (**false** — Out-of-Scope) [VERIFIED: docs.rs Config].
- Compile-time sizing defaults [VERIFIED: crate source `src/lib.rs:157-162`]: `IFACE_MAX_ADDR_COUNT = 8`, `IFACE_NEIGHBOR_CACHE_COUNT = 3`, `IFACE_MAX_ROUTE_COUNT = 4` — each overridable by feature flag (`iface-max-addr-count-16`, `iface-neighbor-cache-count-16`, `iface-max-route-count-16`, … up to 1024). Neighbor-cache 3 is small but self-managing (smoltcp does its own ARP/ND); the devip-visible `/net/arp` view table is ours and separately sized (devip NCACHE=256, LRU, no timeout [VERIFIED: `arp.c:18`]). `update_ip_addrs` **panics on non-unicast addresses and on overflow** [VERIFIED: docs.rs] — devip-side validation must absorb both before the call (Pitfall P-F/P-G).
- `Routes`: `routes()`/`routes_mut()`, `add_default_ipv4_route`/`add_default_ipv6_route`/`get_*`/`remove_*`, `update(FnOnce(&mut heapless::Vec<Route>))` [VERIFIED: docs.rs Routes]. devip's route forest is richer (tags, flags, longest-match v4/v6 trees — `iproute.c`); devip owns the file, mirroring the default route into smoltcp.

## Architecture Patterns

### System Architecture Diagram

```
userspace (Harvey, unmodified):  ping  ip/ipconfig  aan  echo/tcpblast  libc dial/announce/listen
        │  pure file I/O on /net (open/read/write — ZERO net syscalls)
        ▼
┌─ r9 kernel ─────────────────────────────────────────────────────────────────┐
│ #I devip (port/src/devip*.rs) — hand-built devip-exact file tree:            │
│   conv tables, qid scheme QID(p,c,y), clone/ctl/data/err/listen/local/       │
│   remote/status, ipifc, iproute, ipselftab, arp (view), ndb, log            │
│        │ conv ↔ SocketHandle map; ctl verbs translate to socket calls        │
│        ▼                                                                    │
│   THE SEAM (single-ownership context):  Interface + SocketSet + adapter      │
│        ▲ driven EXACTLY by:                                                 │
│        │ (1) RX-IRQ wake → (2) one-shot deadline from poll_at() →           │
│        │ (3) local file-op activity; socket wakers wake blocked readers      │
│        ▼                                                                    │
│   Device adapters:  loopback (fixed-pool, Medium::Ip) ─ null (bind errors) ─ │
│                     ether (Medium::Ethernet over netif convs 0x800/0x806/    │
│                     0x86dd; snoops ARP/NDP for the /net/arp view)            │
│        ▼                                                                    │
│   Phase-7 landing (UNCHANGED): netif.rs/devether.rs `#l`, virtio.rs rings,   │
│   pci.rs IRQ handler (ISR read-to-ack, DRAIN_CAP retire, rx_ingress demux,   │
│   tx_retired wake)                                                          │
└─────────────────────────────────────────────────────────────────────────────┘
        │ virtio-net (MAC+STATUS only, 12-byte hdr, barrier-first)
        ▼
qemu -netdev user (slirp, hermetic) → host; tap leg = Phase 9 verdict venue
```

### Recommended Project Structure

```
../r9/port/src/
├── devip.rs          # #I device surface: attach/walk/open/stat/read/write/close,
│                     #   qid scheme, ipgen-style dir generation, Fs/Proto dispatch
├── devip/
│   ├── conv.rs       # Conv struct (state, perms, owner collapse, SocketHandle, waiters)
│   ├── proto.rs      # tcp/udp/icmp/icmpv6 ctl-verb handlers + status/stats formatters
│   ├── ipifc.rs      # interfaces, lifc lists, bind/add/remove/add6/ra6 verbs
│   ├── route.rs      # /net/iproute forest (tags, flags, longest-match)
│   ├── arp.rs        # /net/arp view table (snoop-fed + ctl writes; NCACHE/LRU semantics)
│   ├── netlog.rs     # /net/log + flag-guarded debug print plumbing (D-07)
│   ├── medium.rs     # Medium registry: loopback/null/ether names + bind dispatch
│   ├── stack.rs      # THE SEAM: Interface+SocketSet ownership, drive(), waker bridge,
│   │                 #   poll_at deadline, stack-debt flag
│   └── phyadapt.rs   # Device impls: LoopbackDev (fixed pool), EtherDev (netif bridge)
├── devip_tests.rs    # host-side pins (allocator_tests.rs precedent; see Validation)
└── lib.rs            # + pub mod devip; cfg(dev_ip) gating per the workspace check-cfg rule
```

r9 integration points (all verified against the live tree):
- `port/src/lib.rs` — `pub mod devip;` (module list at `lib.rs:19-65`).
- `port/src/dat.rs` — add `DEV_IP: usize = 9` (next free; DEV_ETHER=8 at `dat.rs:38`). The dev-boundary error convention is `Result<T, &'static str>` (`dat.rs:11-13`).
- `port/src/namec.rs` — `registry_attach` gains `'I' => crate::devip::attach(spec)` (the hand-built rune table, `namec.rs:60-79`); `dev_walk`/`dev_open`/read/write/stat/close dispatches gain DEV_IP arms (the devether precedent in the same file + `aarch64/src/syscall.rs:292-296,383-388` for the pread/pwrite arms).
- `../r9/Cargo.toml` `[workspace.lints.rust] unexpected_cfgs` — **any new `cfg(dev_ip)` must be predeclared there** (the comment block at workspace Cargo.toml lines 15-30 documents the rule; dev_ether's entry is the precedent).
- `aarch64/src/main.rs` — hook registration next to `devether::set_hooks` (line 502-508): devip registers its own `{current, block, wake}` (reusing `proc::pipe_current_pid`/`pipe_block_current`/`pipe_wake` or devip-named equivalents).
- `aarch64/src/platform/virt/timer.rs` — the tick sweep (`tick()` → `proc::tick_timeouts`) is where the net deadline check rides (see seam).
- `aarch64/src/platform/virt/pci.rs` `virtio_net_irq()` (line 838) — the RX retire already calls `port::devether::rx_ingress(frame)`; the ether adapter's "frames pending" interest hooks in there (a wake, not a poll — see seam).

### Pattern 1: The event-driven seam (D-05's design space, researched)

**What:** single ownership of `Interface + SocketSet + Device-adapter` state, driven by events only.
**The r9 primitives it composes** (all verified in-tree):
- **Block/wake hooks**: `port::devpipe::PipeHooks { current: fn() -> u64, block: fn(), wake: fn(&[u64]) }` registered once pre-exec via `set_hooks` (`devpipe.rs:98-122`; arch impls `proc::pipe_current_pid`/`pipe_block_current`/`pipe_wake` wired in `main.rs:485-489`). devether's `EtherHooks` is the same shape (`netif.rs` header + `main.rs:502-508`). devip registers its own triple.
- **Wakeup-loss-freedom**: syscalls run IRQ-masked (the Phase-4 invariant: `PSTATE.I=0` only at EL0 or the idle `wfi`), so the check-condition → register-waiter → `block()` sequence is IRQ-atomic on a single CPU — "the waker can only run after block() swtch'es" (`devpipe.rs:16-22`). smoltcp's one-shot wakers slot in exactly: register → re-check (`can_recv` — spurious-wake discipline) → block; the wake fires from IRQ or another syscall, never between register and re-check.
- **The IRQ side**: IRQ handlers run at EL1 with same-priority nesting masked (`pci.rs:838-847` SAFETY comment); they never take port mcslocks (LEAF discipline — `netif.rs:42-69`); the IRQ↔syscall handoff is a `BusyCell` (non-spinning, fail-fast on contention — provably uncontended here).

**Where `poll()` runs — the recommendation (the agent's-discretion answer):**

Never in IRQ context. Three reasons, all verified in-tree: (1) r9's allocator locks are syscall-context locks ("No allocation happens in IRQ context" — `netif.rs:65-67`); smoltcp poll is allocation-free *given our config* (socket buffers preallocated at conv creation, `heapless` compile-time pools for reassembly/assembly — verified no `Vec` growth in the poll path; but `phy::Loopback` allocates per packet, one reason we don't use it) — the no-IRQ-alloc invariant is r9's to keep, not a fact to re-prove per smoltcp version; (2) port's mcslock `Lock` is never taken from IRQ context (LEAF discipline); (3) the BusyCell makes violations fail-fast, turning a design error into an accounted drop instead of a wedge.

The three D-05 legs then land as:
1. **RX leg**: `virtio_net_irq` retires RX → `rx_ingress` demuxes frames into the ether adapter's conv queues (0x800/0x806/0x86dd) → the adapter's "RX pending" interest wakes the registered devip waiter(s) → the **woken context** runs `drive_stack()` (poll → re-arm deadline) before re-testing its own file-op condition. On loopback there is no IRQ: the TX-write itself loops the packet, and the poll runs synchronously in the same file-op context (leg 3 subsumes).
2. **Timer leg**: after each poll, `poll_at(now, sockets)` → `Option<Instant>` → recorded as an `AtomicU64` deadline in tick units. `timer.rs tick()` already sweeps deadlines every 1 ms (`proc::tick_timeouts`); the net arm checks `now >= net_deadline` and wakes the devip waiters (same wake list as leg 1). This is one-shot *arming* over the existing periodic tick — no new interrupt plumbing, 1 ms granularity vs smoltcp's 1 s RTO floor (three orders of headroom), and the tick runs regardless (preemption source). A true dynamic-CNTP-rearm one-shot is possible but reworks the preemption timer for zero measurable benefit — **rejected**; note it in the contract doc as a considered alternative.
3. **Local-activity leg**: every devip file op that mutates socket state (ctl writes, data writes, connect/announce) ends with `drive_stack()` in the same syscall context (0.13's egress-exhaustion means one poll flushes the egress queue).

**The idle-stack-debt corner** (named so the plan can't forget it): a guest with unacked TCP data and NO devip-blocked procs and no local activity would miss a retransmit deadline — TCP timers are soft state (QoS, never correctness; smoltcp's own poll_at docs), but the phase gates never sit there (aan syncs at 8 s, ping at 1 s, tcpblast continuous, ipconfig alarm(1000) polls). Mechanism: when the timer leg fires with no registered waiters, set a "stack debt" flag; the next DEV_IP-dispatched syscall (or any syscall entry — planner's call) drives the stack first. One line in the contract doc, one host pin.

**The waker bridge** (smoltcp `Waker` → r9 wake): smoltcp wants `&core::task::Waker`. Constructing one needs `Waker::from_raw(RawWaker)` — **unsafe** (the only new unsafe the seam needs; smoltcp itself is `#![deny(unsafe_code)]`). Shape: a tiny RawWaker vtable whose `wake` reads a per-socket "wakee pid" slot and calls the arch `wake(&[pid])` hook; `clone` bumps nothing (the devip conv owns the registration; waker is recreated per registration). Location: port crate keeps `#![forbid(unsafe_code)]` on existing modules; the bridge lands either arch-side (like the hooks) or in a new `port/src/devip/waker.rs` carrying the SAFETY comment + an UNSAFE-AUDIT.md entry (the audit's "one SAFETY per site, lint-enforced" contract at `../r9/UNSAFE-AUDIT.md`). Because smoltcp's wakers are one-per-direction and woken-once, the devip conv records the blocked reader's pid and re-registers after every wake — the conv struct, not the waker, owns the waiter set.

**Host-side unit pins** (the `allocator_tests.rs`/`virtio_tests.rs` precedent — `cargo test -p port` on the host): smoltcp compiles for the host (probe-proven), so the ENTIRE seam is pinnable host-side before the first guest boot: (a) waker bridge fires on RX/close/buffer-space; (b) lost-wakeup shape — register→wake→re-register cycles under a scripted socket; (c) deadline arming from poll_at across a retransmit schedule; (d) the accept-pool demux (two SYNs → two pool members); (e) udp headers-mode synthesis from UdpMetadata; (f) tcp state-name mapping table vs devip's `tcpstates[]`; (g) the stack-debt flag drain. These pins are the phase's cheapest insurance and land BEFORE any boot gate depends on the seam (CONTEXT watch-out honored).

### Pattern 2: devip file tree over the seam

- **Qid scheme** (devip.c:35-49, verbatim): `QID(p,c,y) = (p<<17)|(c<<5)|y` — Logtype=5 (32 file types), Logconv=12 (4096 convs), Logproto=8 (256 protos). Top-level type tags: Qtopdir=1, then Qtopbase=2: Qarp=2, Qndb=3, Qiproute=4, Qipselftab=5, Qlog=6; Qprotodir=7, Qclone=8 (protobase), Qstats=9; Qconvdir=10, then Qctl=11, Qdata=12, Qerr=13, Qlisten=14, Qlocal=15, Qremote=16, Qstatus=17, Qsnoop=18. (Note: devip's own tag packing differs from netif's NETQID — devip.rs keeps its own scheme; do not reuse netif's.)
- **Fs/Proto dispatch → Rust**: `Proto` vtable (connect/announce/bind/state/create/close/rcv/ctl/advise/stats/local/remote/inuse/gc — `ip.h:383-417`) maps to a per-protocol module with free functions + a static dispatch table; `Conv` (ip.h:170-221) maps to a conv struct holding `{ state, perm, owner-collapse, laddr/raddr/lport/rport, ttl/tos, SocketHandle, waiters, headers flag, err queue }`.
- **Clone/open/close lifecycle** (devip.c:401-444, 546-577): clone allocates the first free conv (source scans `p->conv[]` and calls `p->gc` under pressure — the smoltcp era's gc = drop fully-Closed convs' sockets), first data/ctl open collapses owner/perms 0666→0660, last close resets (`closeconv`: state=Idle, owner="network", queues reopened).
- **Blocking data reads**: devip `qread(c->rq)` blocks per-conv; in the seam, the conv's smoltcp socket + registered recv waker + the block hook reproduce it. UDP reads are Qmsg datagram-at-once (discard-what-doesn't-fit per man/3/ip); TCP reads are byte-stream.
- **The `listen` file**: open requires state==Announced ("not announced"), parks until the accept pool yields a connected conv ("listen hungup" if the rq is closed), returns the NEW conv's ctl as the chan qid (devip.c:445-495).
- **err file**: per-conv error queue (devip `eq`, Qmsg). UDP ICMP advice hangups land here (udpadvise qhangup's rq+wq with the message unless `ignoreadvice`).

### Pattern 3: Mediums as (Interface, Device) pairs

One smoltcp `Interface` per bound ipifc, each with its own Device:
- **loopback** (`bind loopback`): a small hand-written fixed-pool `Medium::Ip` Device (transmit pushes into a bounded VecDeque; receive pops; capabilities MTU 65535 like smoltcp's Loopback — devip's loopback maxtu is 16 KiB [VERIFIED: `loopbackmedium.c:11-13`]; checksum ignored is fine on loopback per smoltcp's own #919). **Do NOT use `phy::Loopback`** (per-packet `vec!` allocation). The 9k loopback's kproc reader (`loopbackread`) has NO analog — the seam's synchronous drive replaces it (no kprocs in r9, by design).
- **null** (`bind null`): devip-exact behavior is that binding FAILS: `nullbind` errors `"cannot bind null device"` [VERIFIED: `nullmedium.c:9-13`]. The medium name is registered (so `ipfindmedium("null")` resolves and produces the right error), which is the entire null-medium deliverable. The CONTEXT's "loopback + null mediums FIRST" = register both names with correct bind behavior before ether.
- **ether** (`bind ether <path>`): the adapter opens netif conversations 0x800/0x806/0x86dd against `#l0` — in 9k via `chandial` of `path!0x800` etc. (`ethermedium.c:161-213`); in r9 via direct `netif::` calls (conv_alloc + conv_connect per type — the same machinery devether's file surface exposes, no file walk needed kernel-side). The adapter's `receive()` merges the three conv queues in arrival order; `transmit()` posts through the driver's `tx_post` (devether's registered `EtherDriverOps` — `devether.rs:111-124`). The adapter is also where the `/net/arp` snoop lives (next pattern).
- **TX loopback/broadcast feedback**: 9k's `etheroq → etheriq` TX self-delivery was deliberately NOT implemented in Phase 7 (`netif.rs:21-24`, "Phase 8's arp/ethermedium EXPECTS it"). For the smoltcp design this is mostly moot (smoltcp doesn't need to see its own TX frames), EXCEPT the /net/arp snoop wants TX-side ARP/NDP too — implement the feedback at the ADAPTER level (snoop on transmit() frames) rather than re-queuing into netif.

### Pattern 4: `/net/arp` as a snoop-fed view (the neighbor-cache invisibility answer)

smoltcp 0.14 has NO public neighbor-cache accessor (the `iface` module exports only Config/Context/Interface/Route/RouteTableFull/Routes/Slaac/SocketHandle/SocketSet/Storage + result enums [VERIFIED: docs.rs iface module item list]). So `/net/arp` is a **devip-owned table** with devip's exact surface:
- read: fixed 90-byte lines `"%-6.6s %-8.8s %-40.40I %-32.32s\n"` (medium, state[OK|WAIT], ip, mac), `offset % 90` gate, LRU over NCACHE=256, no timeout [VERIFIED: `arp.c:18,520-556`];
- write: `flush` / `add ether <ip> <mac>` / `del <ip>` [VERIFIED: `arp.c:425-475`; exercised for real by ipconfig's IPv6 RA path — `ipv6.c:604-615` writes `add ether %I %E` for the RA's source-lladdr];
- feed: the ether adapter parses ARP (ethertype 0x806) and NDP (ICMPv6 NS/NA inside 0x86dd) frames passing through `receive()`/`transmit()` with smoltcp's own `wire::ArpPacket`/ndisc parsers (reused, not hand-rolled), recording (ip→mac, OK) on replies/announcements and (ip, WAIT) when we observe our own solicitations.

**The deliberate divergence to document in the contract doc**: in 9k, `arp add` injects into the REAL resolution path; here it writes the view table only — smoltcp's internal cache self-manages (and flushes on IP change, 0.8.0 #564). Every Phase-8 consumer (static/DHCP ipconfig, IPv6 RA via ipconfig, ping, aan, tcpblast) works through smoltcp's own ARP/ND, so the divergence is observational, not functional — but a script that `arp add`s expecting resolution behavior gets the file-level success and no steering. D-08's gates assert the file semantics; the divergence note is the contract doc's honesty section.

### Anti-Patterns to Avoid

- **Polling the stack on a timer cadence "for simplicity"**: D-05 locked event-driven; a periodic poller is a seam rewrite later (the wakeup/timer seam is woven into the blocking file semantics).
- **Running `Interface::poll` in IRQ context**: breaks r9's no-alloc-in-IRQ and mcslock-LEAF invariants; the BusyCell would (correctly) turn it into accounted drops.
- **`phy::Loopback`**: allocates per packet (`vec![0;len]` in its TxToken) — wrong posture for the kernel; write the fixed-pool loopback Device.
- **Letting smoltcp errors/panics reach the file surface**: smoltcp panics on non-unicast `update_ip_addrs`, hop_limit=0, SocketSet type mismatch, full fixed storage — every smoltcp call site maps onto a devip error string (the contract doc's error table); a kernel panic from a userspace ctl write is a gate red.
- **Reaching into smoltcp internals for /net/arp**: there is no accessor; don't fork-patch smoltcp (D-02's crates.io pin is the supply-chain record — a patched fork breaks it).
- **Growing the syscall table for networking**: zero new syscalls (Pitfall M1 — the measured-set rule generalizes to devices; announce/listen/dial are file ops).
- **Wiring `socket-dhcpv4`** as a shortcut: D-09's gate is ipconfig-over-/net/udp; the dhcpv4 socket is the recorded fallback only.
- **`socket-tcp-cubic`**: needs f64 (smoltcp's docs explicitly warn against kernel/IRQ FP); `socket-tcp-reno` only.

## The devip Contract Surface (the D-03/D-08 enumeration)

This is the contract doc's table of contents — every entry verified against `sys/src/9k/ip/` source AND `sys/man/3/ip` this session. Source beats man where they disagree; each divergence is flagged **[man-vs-src]**. Depth: completeness over prose — the contract doc carries the full format strings.

### Stack topology
- `bind -a #I<spec> /net`; spec is an integer 0..15 per man (source: `Nfs = 128` stacks, `devip.c:44`; attach: `devno = atoi(spec)`, `error("bad specification")` on overflow) [VERIFIED: devip.c:298-316].
- Each stack implements IPv4 AND IPv6; stacks are independent.
- All addresses are 16-byte IPv6 form; v4 mapped as `00..00 ff ff` prefix (`v4prefix`).

### Top-level files (per stack)
| File | Mode | Read | Write |
|------|------|------|-------|
| `arp` | 0664 | one 90-byte line per entry (format below); `offset % 90 != 0` → 0 | `flush` / `add ether <ip> <mac>` / `del <ip>` |
| `iproute` | 0664 | one line per route: `target mask nexthop flags tag ifc` (flags: `4` `6` `i` `b` `u` `m` `p`) | `flush` / `tag <string>` / `add <target> <mask> <nexthop>` / `remove <target> <mask>` / `route <target>` (console print, debug) |
| `ipselftab` | 0444 | read-only; `"%-44.44I %2.2d %4.4s\n"` per local address (addr, ifc-use-count, flags as iproute) [VERIFIED: ipifc.c:1028-1052] | — |
| `ndb` | 0666 (write: hostowner-only in donor — single-user collapse this era) | ≤1024-byte free-form text (`Fs.ndb[1024]`), qid.vers = ndbvers | OWRITE\|OTRUNC zeroes first; writes append at offset; Eio past end [VERIFIED: devip.c:367-371,1429-1441] |
| `log` | 0666 | blocks until netlog output (bad-packet/error messages, implementation-defined content) | `set <flags…>` / `clear <flags…>` / `only <addr>`; flags: `ppp ip fs tcp icmp udp compress gre tcpwin tcprxmt udpmsg ipmsg esp` [VERIFIED: netlog.c flags[] + netlogctl] |
| `bootp` | — | **[man-vs-src]** in the man SYNOPSIS but ABSENT from the source (devip.c has no bootp qid) — the contract doc records it absent | — |

### Protocol directories
`tcp`, `udp`, `icmp`, `icmpv6`, `ipifc` present this phase. Dropped per D-08 (simply absent, no stubs): `gre esp rudp igmp ipmux` (+ `pktmedium`/`netdevmedium` mediums; the man documents `bind pkt`/`bind netdev` — the contract doc lists them as absent).

Each protocol dir contains: `clone` (0666, owner `network`), `stats` (0444), and one numbered dir per allocated conversation (`Fsprotoclone` allocates on clone-open; exhaustion → Enodev, or panic when `cpuserver` — the r9 era has no cpuserver flag: map to Enodev, note the divergence).

Each conversation dir contains: `ctl`, `data`, `err`, `listen`, `local`, `remote`, `status`, and — **`snoop` only for ipifc convs** (0400; devip.c:95-98 rejects snoop for other protos).

### Generic conversation semantics
- **clone**: `open(.../clone)` allocates a conv; the returned fd IS the new conv's `ctl`; reading ctl back returns the conv number (devip.c:642-647 prints `"%lud"`; the man says "a text string representing the number").
- **perms/ownership**: conv files 0666 until first open, then 0660 with the first opener as owner (`devip.c:436-440`; the donor's `eve` checks collapse in r9's single-user era — same collapse as devether, `netif.rs:13-15`). `ipwstat` on ctl/data may change owner/mode (devip.c:515-544).
- **ctl read** → conv number. **local/remote read** → `"%I!%d\n"` (or proto override). **status read** → proto-specific line(s). **err read** → per-conv error queue (Qmsg).
- **listen**: `open(N/listen)` requires Announced else `error("not announced")`; blocks until an incoming call; on success the returned chan points at the NEW conv's ctl; hangup during the wait → `error("listen hungup")` [VERIFIED: devip.c:445-495]. Incoming-call queue depth `Maxincall = 128` per announced conv [VERIFIED: ip.h:40] — see the smoltcp no-backlog finding (Architecture Pattern 1 / Pitfall P-A).
- **Conv states** (ip.h:71-75): `Idle=0, Announcing=1, Announced=2, Connecting=3, Connected=4`.
- **Generic ctl verbs** (devip.c:1128-1176):
  - `connect <ip>!<port>[!r] [<local>]` — active open; BLOCKS until Connected (devip.c connectctlmsg sleeps on `c->cr`; error via `c->cerr`: Econrefused/Etimedout/…). `!r` → restricted local port 600..1024 cycling; default local port: **random in [32768, 65536)** via `nrand` [VERIFIED: devip.c:786-811] — **[man-vs-src]** the man says "a free port number starting at 5000 is chosen"; the source randomizes in the high range. Quad-uniqueness violation → `"address in use"`.
  - `announce <X>` — X = decimal port or `*` (wildcard: any unannounced port; donor gates `*` on iseve — single-user collapse). Sets laddr=IPnoaddr; "Announce fails if the connection is already announced or connected" (Econinuse via state check).
  - `bind <X>` — set local port (APE sockets emulation; unused natively).
  - `ttl <n>` / `tos <n>` — default MAXTTL (64) when arg missing.
  - `ignoreadvice` — don't break UDP conns on ICMP errors.
  - `addmulti <ifc-ip> [mcast-ip]` / `remmulti <ip>` — multicast binds (error strings: "addmulti needs interface address", "addmulti for a non multicast address", Ebadip).
  - `maxfragsize <n>` — **[man-vs-src]** source-only verb, not in the man page [VERIFIED: devip.c:1165-1169].
  - Unknown verb → `error("unknown control request")`; empty write → `"short control request"`.
- **Port range**: man says 1..32767; source's allocator disagrees (above) — contract doc records source.

### TCP (tcp.c)
- Extra ctl verbs [VERIFIED: tcp.c:3330-3345]: `close` (graceful FIN → smoltcp `close()`), `hangup` (RST → smoltcp `abort()`), `hangupxmit` (**[man-vs-src]** source-only; `tcpxmitclose` — close after TX drain), `keepalive [n]` (default 30000 ms → smoltcp `set_keep_alive`), `checksum <n>` (n==0 emits zero checksums — smoltcp has NO such knob; the contract doc maps it as accepted-but-ignored-with-netlog-note or an explicit deviation — decide there), `tcpporthogdefense on|off` (a global toggle in the donor; smoltcp has no equivalent — random-seq keepalives are the mechanism; contract doc decides: accept-and-record vs loud-unimplemented).
- status format [VERIFIED: tcp.c:493-509]: `"%s qin %d qout %d rq %d.%d srtt %d mdev %d sst %lud cwin %lud swin %lud>>%d rwin %lud>>%d qscale %d timer.start %d timer.count %d rerecv %d katimer.start %d katimer.count %d\n"` with the state from `tcpstates[]`. Many fields have no smoltcp analog (qscale/rerecv/katimer) — the contract doc defines the mapping (real where smoltcp exposes it: state, send/recv queue bytes; fixed/zero where not).
- stats (`/net/tcp/stats`): man documents 11 fields; the source emits 28 tagged lines [VERIFIED: tcp.c:348-375 statnames: MaxConn, MaxSegment, ActiveOpens, PassiveOpens, EstabResets, CurrEstab, InSegs, OutSegs, RetransSegs, RetransSegsSent, RetransTimeouts, InErrs, OutRsts, CsumErrs, HlenErrs, LenErrs, OutOfOrder, Resequenced, ReseqBytelim, ReseqPktlim, Delayack, Wopenack, Recovery, RecoveryDone, RecoveryRTO, RecoveryNoSeq, RecoveryCwind, RecoveryPA] — **[man-vs-src]** contract doc carries the source's list; smoltcp exposes no stats counters, so these are devip-maintained counters incremented at the bridge.
- TCP gc (donor): Syn_received >5 s and Finwait2 >5 min reaped under conv pressure (tcp.c:3362-3405) — smoltcp has its own timeouts; the devip gc maps to dropping Closed conv sockets.

### UDP (udp.c)
- ctl: only `headers` (sets the 52-byte `Udphdr` prefix mode; source sets `headers = 7` "new headers format" [VERIFIED: udp.c:513-526]). Default = point-to-point.
- Semantics: after `announce`, each datagram from a NEW remote quad spawns a new incoming conv (Fsnewcall, udp.c:445) — the listen/accept path; in `headers` mode all datagrams land on the announced conv with the Udphdr prefix (no listen needed) — man/3/ip UDP section verbatim contract.
- status: `"%s qin %d qout %d\n"`, `"Open"`/`"Closed"` [VERIFIED: udp.c:124-132].
- stats: 4 tagged fields (datagrams received / bad ports / malformed / sent — man; verify source names in udp.c during contract writing).
- advise: ICMP errors qhangup the conv's rq+wq with the message unless `ignoreadvice` [VERIFIED: udp.c udpadvise].

### ICMP (icmp.c)
- ctl: **none** (`icmp->ctl = nil` [VERIFIED: icmp.c:479-ish]) — only the generic verbs. NO headers mode for v4 (man documents headers only for icmpv6/udp).
- The conv's `lport` IS the ICMP ident: on write the kernel OVERWRITES the packet's ident field with `c->lport` (`icmpkick`: `hnputs(p->icmpid, c->lport)` [VERIFIED: icmp.c:186]); RX dispatch matches reply-ident to `lport` (icmp.c:280-284). smoltcp mapping: `icmp::Endpoint::Ident(lport)`.
- ping(8) exercises exactly this: `netmkaddr(host, "icmp", "1")` → `connect <host>!1` → raw Icmphdr writes/reads on data [VERIFIED: ping.c:562-565]; ping filters `type != echoreply` itself (tolerates the looped-back request).
- status: `"Datagram qin %d qout %d\n"` [VERIFIED: icmp.c:128-137].

### ICMPv6 (icmp6.c) — the mapping corner
- ctl: `headers` only (sets `headers = 6` [VERIFIED: icmp6.c:286-291]).
- **`data` carries FULL IPv6 packets** — userspace builds/parses the IP6 header itself (write path: `padblock(bp, IP6HDR)`, the kernel fills src/dst/ident unless headers mode; read path delivers the packet with the IP header — ping.c's v6 proto skips `IPV6HDR_LEN = 40` on both sides [VERIFIED: icmp6.c:236-282, ping.c:131,143]). smoltcp's icmp socket is payload-level → the bridge PARSES the user's IPv6 packet on write (extract src/dst/payload; honor the user src in headers mode) and SYNTHESIZES an IP6 header on read. Pitfall P-E.
- `headers` mode prefix: 40-byte `{unused[8], laddr[16], raddr[16]}` (man/3/ip ICMPV6 section; `ICMP_USEAD6`).
- ipconfig's RA path uses it: `connect <addr>!<port>!r <port>` on icmpv6/clone ctl, then `headers`, then RS/RA I/O on data [VERIFIED: ipv6.c:346-384 dialicmp].

### ipifc (ipifc.c)
- clone/stats/N/{ctl,data,status,…,snoop}; `data` is unused for I/O this era (packet interfaces per `bind pkt` are dropped).
- ctl verbs [VERIFIED: ipifc.c:765-792 + devip.c bind path]:
  - `bind ether <path>` / `bind loopback` / (`bind pkt`, `bind netdev` — absent per D-08). Donor ether bind dials `path!0x800/0x806/0x86dd` [VERIFIED: ethermedium.c:161-213]; r9 maps to netif conv_alloc+conv_connect (Architecture Pattern 3). Errors: `"unknown interface type"`, `"interface already bound"`, Ebadarg.
  - `unbind`
  - `add <local> [mask [remote [mtu [proxy]]]]` / `try <…>` (try = tentative v6 address for DAD; used by ipconfig ipv6.c:404) — defaults: class mask, `local AND mask` remote, mtu 1514 ether/4096 packet.
  - `remove <local> <mask>`
  - `mtu <n>`
  - `reassemble` — reassemble IP fragments before forwarding
  - `iprouting [n]` — forwarding toggle
  - `bridge` / `promiscuous` / `connect <type>` / `addmulti` / `remmulti` / `scanbs` / `headersonly` — these arrive at the IPIFC ctl from the man page's ether-side list (devether's own conv ctl already covers the devether file surface — Phase 7 locked depth; the contract doc assigns each verb its exact home).
  - `add6 <v6addr> <pfx-len> [onlink auto validlt preflt]` — RFC 2461 §6.2.1 params.
  - `ra6 <keyword> <value> ...` — keywords: `recvra sendra mflag oflag maxraint minraint linkmtu reachtime rxmitra ttl routerlt` (man/3/ip + ipv6.c issuerara6/issuebasera6).
- status read [VERIFIED: ipifc.c:218-246]: first line `"device %s maxtu %d sendra %d recvra %d mflag %d oflag %d maxraint %d minraint %d linkmtu %d reachtime %d rxmitra %d ttl %d routerlt %d pktin %lud pktout %lud errin %lud errout %lud\n"`, then per-address `"\t%-40I %-10M %-40I %-12lud %-12lud\n"` (local, mask, remote, validlt, preflt); the man page's 9-field description is the older/simplified form — **[man-vs-src]** contract doc carries the source format.
- ipifc `local` read: per-address `"%-40.40I ->"` + linked self-IPs (ipifc.c:250-275).
- Bind defaults set at ipifcbind: `rp.maxraint=600000 minraint=200000 linkmtu=0 reachtime=0 rxmitra=0 ttl=MAXTTL routerlt=3*maxraint` [VERIFIED: ipifc.c:143-152].

### Mediums
- `ether`: hsize=14, mintu=60, maxtu=1514, maclen=6; ares=arpenter (resolution records), areg=sendgarp — **gratuitous ARP on every new configured address** [VERIFIED: ethermedium.c Medium struct + man "whenever a new address is configured onto an Ethernet, an ARP request is sent"]. smoltcp does NOT send gratuitous ARP on `update_ip_addrs` — the ether adapter emits it devip-side (craft via smoltcp `wire::ArpPacket` — an allowed use: wire codec, not a protocol engine) — contract-doc decision, gateable via pcap.
- `loopback`: maxtu 16 KiB, 1 MB Qmsg queue, kproc reader in the donor [VERIFIED: loopbackmedium.c:11-38] — kproc replaced by the seam's synchronous drive.
- `null`: registered name whose bind always errors `"cannot bind null device"` [VERIFIED: nullmedium.c:9-13]. That error IS the null-medium contract.

### Error vocabulary (the file surface's exact strings)
`Eperm` (Eperm on write-to-read-only etc.), `Enodev` (clone exhaustion / no interface), `Econinuse`, `Econrefused`, `Etimedout` (error.h:35-37), `Ebadip` ("bad ip address syntax"), `Ebadnetctl` (netlog), `Eio` (ndb past-end), plus the literals scattered above: "bad specification", "short control request", "unknown control request", "connect not supported", "announce not supported", "bad args to connect/announce/bind", "address in use", "not a local IP address", "malformed address", "not announced", "listen hungup", "stats not implemented", "unknown interface type", "interface already bound", "cannot bind null device". The contract doc's error table maps EVERY one to a trigger and a gate.

## Consumer Usage Surfaces (the gate binaries, read from source)

### libc (the unmodified consumer contract) [VERIFIED: sys/src/libc/9sys/announce.c, dial.c]
- `announce("tcp!*!7", dir)`: nettrans → open `/net/tcp/clone` ORDWR → read ctl → conv number → write `announce <naddr>` → return ctl fd + dir.
- `listen(dir, newdir)`: open `<dir>/listen` ORDWR (BLOCKS) → read → new conv number.
- `accept(lcfd, ldir)` (in listen path): uses the returned ctl fd.
- `dial("tcp!127.0.0.1!7")`: nettrans → clone → `connect <ip>!<port>` → open `<dir>/<n>/data`.
- No syscalls beyond open/read/write — the file tree is the whole ABI (NETIP-02's premise, re-verified).

### ip/ipconfig (main.c 2051 lines + ipv6.c 995 + ppp.c 60; DHCP lives INLINE in main.c — the CONTEXT's "dhcpclient.c" reference is the STANDALONE `sys/src/cmd/ip/dhcpclient.c`, a different binary used by boot paths; both enumerated here)

**Every ipconfig invocation** [VERIFIED: main.c:367-375]: `init()` calls `srand(truerand())` UNCONDITIONALLY, and libc's `truerand()` **sysfatals** if `/dev/random` can't be opened/read [VERIFIED: libc/9sys/truerand.c — `sysfatal("can't open /dev/random")`]. **r9 has no `/dev/random` today** (devcons has `#c/time` only; ARCHITECTURE.md's gap note + grep-verified empty). This is a hard Phase-8 blocker discovered in research — see Open Questions OQ-2 and Pitfall P-I. (The standalone dhcpclient.c also reads `/dev/random` directly for the xid, line 208.)

**Static IPv4 path** (NETIP-04): `-c <ctl>` writes to the ether device's ctl (ether-side, Phase 7 surface) → open `<mpoint>/ipifc/clone` → write `bind ether <dev>` (default `/net/ether0`) → write `add <ip> <mask> [<remote>]` → if `-g`: open `/net/iproute`, write `add 0 0 <gw>` (v4) or `add :: /0 <gw>` (v6) [VERIFIED: main.c:722-744,817-881, addroute]. First interface → mkndb → write `/net/ndb`.

**DHCP path** (NETIP-05, D-09's fidelity target) [VERIFIED: main.c:923-963 openlisten/dhcpsend/dhcprecv]:
1. `add 0.0.0.0 0.0.0.0` on the ipifc ctl (the unnumbered-preconfig that smoltcp's `update_ip_addrs` would PANIC on — absorb devip-side; Pitfall P-F).
2. `announce("<mpoint>/udp!*!68")` (renewing/rebinding: `udp!<laddr>!68`) → `fprint(cfd, "headers")` (UDP headers mode) → open `<dir>/data` ORDWR → close the ctl.
3. TX: full `sizeof(Bootp)` datagrams (the "All_Aboard NAT" comment — max-size packets survive broken NATs) with the Udphdr prefix in headers mode; broadcast `255.255.255.255` dest for Discover/Request, server-dest for renew.
4. RX: `alarm(1000)`-wrapped `read` (interrupt → timeout retry; the alarm/note machinery exists in r9 since Phase 5).
5. Lease: `dhcpwatch` fork keeps the lease (renew/rebind timers; `-r` backgrounds forever; `-X` no watchdog). On lease expiry: `remove <ip> <mask>`.
6. `-o` options always request `ipmask ipgw dns sys ntp`; vendor class `plan9_<cputype>` unless `-G`; options 128/129 = Plan 9 fs/auth servers. → `/net/ndb` write on first interface.
- smoltcp side confirmed compatible on RX (broadcast acceptance by port) — the TX src-0.0.0.0 question is OQ-1 with an early host pin prescribed.

**IPv6 path** (NETIP-06) [VERIFIED: ipv6.c]: `add6`/`ra6` ctl writes (formats above); DAD: `try <addr> <mask>` → sleep(3000) → **read `/net/arp` and tokenize lines, dup iff fields[2] == the new address** (ipv6.c:425-455) → dup ? `remove` : `add`. RA recv: `ra6 recvra 1` + the icmpv6 dialicmp path (RS send, RA recv); on RA with source-lladdr option: write `add ether <ra-src> <lladdr>` to `/net/arp` (ipv6.c:604-615). Link-local derivation via `ip/linklocal` (EUI-64 from MAC; reads `<dev>/addr` — devether's addr file, Phase 7).

**Standalone dhcpclient.c** (boot-time DHCP, not the phase gate): same `announce udp!*!68` + `headers` + Bootp data pattern; `/dev/random` direct read for xid; rfork-based `thread()` helper (not libthread).

### ip/ping (611 lines) [VERIFIED: ping.c]
- `ds = netmkaddr(argv[0], "icmp"|"icmpv6", "1")` → `dial(ds)` → data rd/wr. v4: raw Icmphdr in the data payload (kernel builds the IP header). v6: FULL IPv6 packets (`IPV6HDR_LEN` skip both directions; `prreply6` reads `Ip6hdr` from the read buffer).
- `notify(catch)` (note handling — exists), `rfork(RFPROC|RFMEM|RFFDG)` reader fork (exists), `nsec()` warmup, prints `rtt` from its own timestamps. No `headers` write for v4.
- The smoltcp-side loopback proof: iface auto-icmp-echo-reply answers the request addressed to us; the reply dispatches to the Ident(lport)-bound socket; ping reads it. ping ALSO receives its own looped-back EchoRequest (accepts_v4 matches requests too) and filters it by type — verified compatible.

### aan (524 lines) [VERIFIED: aan.c — full read]
- **Server**: `aan [-d] [-m maxto] <netdir>` (netdir = the ANNOUNCED conv dir, e.g. from `aux/listen`; strips a `/local` suffix) → `listen(devdir, ldir)` (blocks; `alarm(maxto*1000)`, default 24 h) → `accept` → reads `<ldir>/local` + `/remote` for logging → serves the tunneled 9P-ish stream.
- **Client**: `aan -c [-d] <dialstring>` → `dial()` in a 1 s retry loop; on errstr containing "connection refused" → `threadexitsall("server died...")` — the gate must keep the listener alive across the drop or accept that exit as the scripted signal.
- **Wire**: 12-byte LE header `{nb, msg, acked}` + payload; messages numbered, cumulatively acked; `Synctime = 8 s` sync beacons; on netfd rd/wr failure: close, redial/relisten, `synchronize()` retransmits unacked. Bufsize 8 KiB, Nbuf=10.
- **Deps**: libthread (`proccreate`, `alt`, `Channel` — the userspace tier builds it; aarch64 libthread is Phase 6/10 payload territory — verify the tier status when planning the payload), `alarm`, `sleep`, `atnotify`, `syslog(0, "aan", …)` — syslog falls back to console when /sys/log is unavailable (libc/9sys/syslog.c reopens and sets cons=1 on failure — non-fatal; still, gate evidence should prefer `-d` to #c/cons).
- **The D-14 drop**: `echo hangup > /net/tcp/N/ctl` on the underlying conn (tcp hangup → RST via smoltcp `abort()`) → both ends see read failure → reconnect → session resumes. The harness discovers N by walking `/net/tcp/*/status` or reading `local`/`remote` (all file ops).

### The tcpblast/echo payloads (D-12, Phase 7 precedent)
- Precedent: `build/aarch64/netblast.c` — a standalone C file compiled host-side with `7c -FTVw` + `7l` into `build/aarch64/out/`, baked via mkimage; fixtures under `build/aarch64/root-tree/lib/` (arp.req, net-sweep.rc); subset-build.sh carries the build+manifest asserts (lines 272-277, 405-406). tcpblast follows this shape (name/counts/markers are the agent's discretion; TCG-scaled).
- ipconfig/ping are full-tree binaries (sys/src/cmd/ip/…) — D-12 builds them via phase-8 payload rules (targeted mk/7c invocations) rather than tier allowlist appends; note they link libip (eipfmt/parseip — already aarch64-green from earlier tiers).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TCP state machine / Reno / retransmit / RTT estimation | any Rust TCP | smoltcp `socket-tcp` + `socket-tcp-reno` | D-01; 0.14's Reno is RFC-compliant (#1156), retransmit backoff RFC 6298-aligned (#1023) — upstream-tested is the point |
| UDP/ICMP/ICMPv6 engines | hand-rolled | smoltcp socket-udp/icmp | D-01 |
| ARP + IPv6 Neighbor Discovery on the wire | hand-rolled ARP/ND | smoltcp `medium-ethernet` internal machinery | Address resolution is correctness-subtle (0.8.0's ARP hardening series: fill-only-from-ARP, directed-only, flush-on-IP-change) |
| IPv4/IPv6 fragmentation+reassembly | hand-rolled | `proto-ipv4-fragmentation` / `proto-ipv6-fragmentation` features (fixed pools) | Reassembly is an attack surface; upstream's pool is compile-time-bounded |
| Wire packet parsing (for the /net/arp snoop + icmpv6 bridge) | byte-offset hand parsing | smoltcp `wire::{ArpPacket, Ipv6Packet, Icmpv6Repr, Ndisc…}` | The wire codec layer is public and documented ("every layer is fully exposed") — reuse it |
| A periodic stack tick | kproc/periodic poll thread | `poll_at`-armed deadline on the existing 1 ms tick sweep | D-05; r9 has no kprocs by design |
| DHCP client | smoltcp `socket-dhcpv4` wiring | Harvey ipconfig over /net/udp | D-09 — the gate is the real binary's fidelity |
| Address string formatting/parsing in gates | host-side parsers | the guest's own `%I/%M` fmt (eipfmt) via libip | Already aarch64-green; eipfmt is what the file formats are written in |

**Key insight:** the only hand-built things are the FILE SEMANTICS (the contract) and the SEAM (ownership/wake/timer) — everything that is a *protocol* is smoltcp's. The phase's bugs will live in the hand-built layer; that's where the host-side pins and the contract doc concentrate.

## Common Pitfalls

### Cross-check: milestone PITFALLS.md 6/7/9/15/16 applied to this design

- **Pitfall 6 (offload landmines, "ping works, TCP hangs"):** Closed by construction here — Phase 7's virtio mask negotiates MAC+STATUS only (no CSUM/TSO/MRG_RXBUF), smoltcp emits fully-software-checksummed frames, the adapter's `DeviceCapabilities.checksum` stays default (software), and 0.14's new `segmentation-offload` phy feature stays OFF. The *gate* still stands: NETIP-08's sustained bidirectional TCP is the phase's exit, not ping. smoltcp-specific addendum: none of 0.14's TSO support activates without the feature.
- **Pitfall 7 (virtqueue memory ordering):** No new ring code this phase — the adapter's TX consume calls the existing `tx_post` op (devether's registered `EtherDriverOps`); the 07-03 barrier-first machinery is untouched. The soak gate re-exercises it under TCP load (a new traffic shape — bulk bidirectional — the Phase 7 blast was TX-only ARP).
- **Pitfall 9 (slirp masks L2):** D-10/D-11/D-15 already encode the discipline; this research adds the smoltcp-specific note: slirp answers ARP for the gateway itself and accepts marginal frames — the snoop-fed `/net/arp` view will look thin on slirp (little real ARP traffic) and rich on tap; evidence labels must not over-read the slirp arp view. Every slirp-green gate is labeled not-L2-truth; tap re-runs in Phase 9 (joint acceptance).
- **Pitfall 15 (debugging TCP through the session that rides it):** D-07 lands netlog + flag-guarded prints from day one. This research adds the event-seam row to the triage runbook: **"ping-green / TCP-stalls-mid-flow ⇒ check the seam first: missed waker re-registration (one-shot wakers), a mis-armed `poll_at` deadline, or a wedged stack-debt drain — NOT offload"** (offload is closed; the seam is the new silent-wedge surface). netlog's `set tcp tcprxmt tcpwin` gives the retransmit visibility.
- **Pitfall 16 (cross-repo drift):** smoltcp enters via crates.io pin + committed Cargo.lock (D-02); the flake's fixed-output cargo vendor derivation must be re-riven when the lockfile changes (the r9 `packages.kernel-virt` vendor hash changes — the pin-advance commit will carry it; the 05-10 two-lockfile vendoring pattern already handles `-Z build-std`, the smoltcp dep joins the SAME Cargo.lock). Push-before-pin unchanged: r9 commits → r9 gates → push → Harvey flake.lock commit alone + fresh hosted CI. qemu netdev stanza single-source already landed (Phase 7 07-08).

### Phase-specific pitfalls (new, this design's own)

- **P-A — Listen has no backlog in smoltcp.** Excess SYNs to a listening socket are RSTed (docs.rs verbatim). devip queues 128 (`Maxincall`). The accept-pool bridge is the fidelity mechanism; a shallow pool deviates under SYN burst. *Avoid:* size the pool deliberately (128 is devip-exact; each pool member costs two socket buffers — memory math belongs in the plan), refill on accept, pin host-side.
- **P-B — `/net/arp` passthrough drift.** smoltcp's cache is invisible; a "view" that doesn't snoop is an empty file, and one that writes-through to nothing deviates silently. *Avoid:* the snoop-fed devip table (Pattern 4) + the documented divergence (static add doesn't steer smoltcp).
- **P-C — Lost-wakeup wedge in the waker bridge.** smoltcp wakers are one-per-direction, woken-ONCE, spurious-allowed. A bridge that registers once and assumes repeated wakes wedges blocked readers with zero polling cadence to save it (D-05's named watch-out). *Avoid:* re-register after EVERY wake; treat every wake as "re-check `can_recv`/`can_send`"; host-pin the register→wake→re-register cycle before any boot gate depends on it.
- **P-D — `poll()`'s unbounded RX drain.** docs.rs's own DoS warning: a full device queue is unbounded work. *Avoid:* the adapter's RX queue is bounded by construction (virtio ring 256, DRAIN_CAP), but prefer `poll_ingress_single` loops with an explicit per-drive cap + `poll_egress` + `poll_maintenance` — the cap is gate-visible and keeps syscall latency bounded.
- **P-E — icmpv6's full-packet data semantics.** Userspace builds/parses the IPv6 header on `/net/icmpv6/N/data` (unlike v4). A payload-level bridge (smoltcp's natural shape) silently breaks `ping -6` and ipconfig's RS/RA. *Avoid:* the bridge parses/synthesizes IP6 headers; gate with ping -6 over loopback + ipconfig's `add6`/`ra6` run.
- **P-F — smoltcp panics on non-unicast `update_ip_addrs`.** ipconfig's DHCP preconfig writes `add 0.0.0.0 0.0.0.0` (main.c:926). Passed through, this is a KERNEL PANIC from a userspace ctl write. *Avoid:* devip-side absorbs the unspecified-address add (recorded, not pushed); same guard for `::` and overflow past `IFACE_MAX_ADDR_COUNT`.
- **P-G — DHCP src-0.0.0.0 TX fidelity.** smoltcp's udp socket TX sources from the interface address; with no configured v4 address the source-selection path is unproven for our use (smoltcp's own dhcpv4 socket crafts src-0.0.0.0 packets by hand at the wire layer — `socket/dhcpv4.rs:610` — proof the machinery exists, but the udp-socket path may not expose it). *Avoid:* OQ-1's host-side pin FIRST (build the exact ipconfig-shaped exchange host-side: bind 68, broadcast TX with no iface addr); if the udp path can't source 0.0.0.0, the bridge special-cases the DHCP-shaped datagram through the wire layer (the same construction smoltcp's dhcpv4 socket uses) — a boundary shim, not a protocol engine. The recorded fallback is D-09's dhcpv4 socket.
- **P-H — Stale-API traps from older smoltcp.** 0.14: `poll()` returns `PollResult` not bool; `Interface::new` takes the device; heapless 0.9; congestion control is an enum with feature-gated variants; waker methods live on the sockets (no public WakerRegistration type). *Avoid:* this document's code examples are the verified shapes; when in doubt, the downloaded crate source at 0.14.0 is the authority.
- **P-I — ipconfig needs /dev/random.** `truerand()` sysfatals otherwise; called unconditionally at ipconfig init. r9 has no random file. *Avoid:* resolve OQ-2 before the first ipconfig gate (minimal devcons `random`/`urandom` this phase vs reordering AUTH-02 — a user-visible phase-boundary question).
- **P-J — Clock assumptions.** `Instant::from_millis(ticks())` only; CNTFRQ is runtime-read (62.5 MHz TCG / 24 MHz HVF); smoltcp RTO floor is 1 s (min == initial); gate timeouts TCG-scaled per the smoke precedent (600 s stages).
- **P-K — devip `connect` blocks; smoltcp `connect` doesn't.** devip's connectctlmsg sleeps until Connected/failed. The bridge: initiate via `socket.connect(iface.context(), …)`, then block on the waker until `state()==Established` (→ Connected), socket Closed mid-SYN (→ Econrefused), or the deadline (→ Etimedout). Same shape for the blocking `listen` open (pool member leaves Listen → wake).
- **P-L — close/hangup/hangupxmit mapping.** `close`→FIN (`close()`), `hangup`→RST (`abort()`), `hangupxmit`→source-only verb (tcpxmitclose; map to close-after-drain — the contract doc decides the exact smoltcp shape; `shutdown`-of-TX-then-await-drain is close() semantics adjacent).
- **P-M — Stats/counters honesty.** smoltcp exposes no per-proto stats; devip's stats files are devip-maintained counters incremented at the bridge. A stats file that reads all zeros forever is a contract violation (D-08) — the bridge increments at every packet transition it observes.

## Code Examples

Verified patterns (the probe compiled the Device/GAT/poll skeleton against 0.14.0 for both host and aarch64-unknown-none):

### The Device adapter skeleton (compiled green in the probe)
```rust
// Source: probe built against smoltcp-0.14.0 crate source + docs.rs phy pages
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;

struct Null0;                    // the null-medium-shaped Device (rx always empty)
struct NullRx;                   // (devip's null medium itself errors on bind —
struct NullTx;                   //  this shape is the starting point for loopback/ether)
impl RxToken for NullRx { fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R { f(&[]) } }
impl TxToken for NullTx { fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, _len: usize, f: F) -> R { f(&mut []) } }
impl Device for Null0 {
    type RxToken<'a> = NullRx;
    type TxToken<'a> = NullTx;
    fn receive(&mut self, _t: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> { None }
    fn transmit(&mut self, _t: Instant) -> Option<Self::TxToken<'_>> { None }
    fn capabilities(&self) -> DeviceCapabilities {
        let mut c = DeviceCapabilities::default();
        c.medium = Medium::Ethernet;      // Medium::Ip for the loopback device
        c.max_transmission_unit = 1514;   // ethernet MTU INCLUDES the 14-byte header
        c
    }
}
```

### Interface + socket set construction (compiled green in the probe)
```rust
// Source: probe; docs.rs iface pages
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress};
use alloc::vec::Vec;

let mut dev = Null0;
let mut cfg = Config::new(HardwareAddress::Ethernet(EthernetAddress([2,0,0,0,0,1])));
cfg.random_seed = /* per-boot seed: DTB/ticks-derived (docs: "strongly recommended…different on each boot"; needs no crypto) */;
cfg.slaac = false;               // Out-of-Scope (REQUIREMENTS table)
let mut iface = Interface::new(cfg, &mut dev, Instant::ZERO);
let mut sockets = SocketSet::new(Vec::new());   // owned storage under `alloc`
```

### The drive skeleton (D-05's three legs)
```rust
// Source: docs.rs poll/poll_at contract + r9 timer.rs tick sweep shape
fn drive_stack() {
    let now = Instant::from_millis(timer_ticks() as i64);   // 1 tick = 1 ms
    {
        let mut stack = STACK.enter().expect("stack cell contended");  // BusyCell, fail-fast
        // bounded ingress: adapter queue is ring-bounded; still cap explicitly
        for _ in 0..RX_DRIVE_CAP {
            match stack.iface.poll_ingress_single(now, &mut stack.dev, &mut stack.sockets) {
                PollIngressSingleResult::PacketProcessed => continue,
                _ => break,
            }
        }
        stack.iface.poll_egress(now, &mut stack.dev, &mut stack.sockets);
        stack.iface.poll_maintenance(now);
        match stack.iface.poll_at(now, &stack.sockets) {
            Some(at) => arm_net_deadline(at),   // AtomicU64 tick deadline; tick sweep wakes us
            None => disarm_net_deadline(),
        }
    }
    // after the cell drops: wake any blocked readers whose sockets changed
    // (the PollResult::SocketStateChanged leg), via the registered wake hook.
}
```
(`PollIngressSingleResult`'s exact variant names to be read from the 0.14.0 crate source at implementation time — the enum exists with "packet processed / socket state changed" information per docs.rs; treat the snippet's arm names as illustrative, tag [ASSUMED] for the variant identifiers.)

### TCP listener with an accept pool (devip announce/listen mapping)
```rust
// Source: docs.rs tcp::Socket (listen/no-backlog note) + crate source accepts()
// devip:  announce 7  → smoltcp: pool of N sockets all listen(7)
// devip:  open listen → park until a pool member leaves State::Listen → pop it as the new conv
let mut s = tcp::Socket::new(tcp::SocketBuffer::new(vec![0; RX]), tcp::SocketBuffer::new(vec![0; TX]));
s.listen(7)?;                       // IpListenEndpoint { addr: None, port: 7 }
let h = sockets.add(s);             // conv records h; conv.announced = true
// … in drive_stack: a pool member with state() != State::Listen && is_open() has an incall;
// the listen-file open's waker rides that member's recv waker (SYN completion wakes it).
```

### UDP headers-mode synthesis (devip `headers` ctl)
```rust
// Source: docs.rs udp::Socket recv_slice → (usize, UdpMetadata); man/3/ip Udphdr layout
// read path: devip prefixes the 52-byte Udphdr {raddr[16], laddr[16], ifcaddr[16], rport[2], lport[2]}
// from UdpMetadata { endpoint (remote), local_addr, meta } — v4 addresses mapped into v6 form
// (v4prefix) per the devip address model.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| smoltcp `poll() -> bool` | `poll() -> PollResult` (None / SocketStateChanged) | 0.14.0 (2026-08) | The seam's wake-discipline keys off the enum; stale bool memory breaks the build |
| smoltcp ≤0.13 `Interface::new(config, …)` shapes | `new(config, device, now)` (device ref at construction) | 0.11 | Construction needs `&mut dev` alive before the iface |
| heapless 0.8 | heapless 0.9 (+ MSRV 1.91, edition 2024) | 0.13.0 (2026-03) | r9's port is already edition 2024 / nightly-2026-01-30 — aligned |
| fixed TCP retransmit floor 100 ms (0.6-era) | RTO initial 1 s, min 1 s, max 60 s | 0.7.0+ (RTT-estimated); 0.14 bounds verified | TCG gates: one lost segment ≥ 1 s; size timeouts accordingly |
| Reno-only TCP | Reno (`socket-tcp-reno`) vs Cubic (`socket-tcp-cubic`, f64) split features; both RFC-compliant in 0.14 (#1156/#1157) | 0.12 (CC API) → 0.14 (redesign) | Kernel choice: Reno only (Cubic needs f64 — smoltcp's docs warn against kernel/IRQ FP) |
| no TSO | `segmentation-offload` phy feature + TSO (#1148) | 0.14.0 | Stays OFF (Pitfall 6 discipline) |
| ICMP auto-reply always on | `auto-icmp-echo-reply` feature-gated (default on) | 0.13.0 | Keep ON — matches devip kernel-answered echo |
| `WakerRegistration` standalone type (older docs memory) | waker methods directly on each socket | (current 0.14 reality) | D-05's name maps to per-socket `register_recv_waker`/`register_send_waker` |

**Deprecated/outdated for this phase:**
- `smoltcp::phy::Loopback` (per-packet allocation — wrong kernel posture; hand-write the fixed-pool loopback Device)
- `socket-dhcpv4` (D-09: unwired fallback only)
- `proto-ipv6-slaac` / `Config.slaac = true` (REQUIREMENTS Out-of-Scope: DHCPv6/SLAAC excluded; IPv6 = ipconfig-driven)
- smoltcp `async` feature (r9 is not an async kernel; the wakers bridge onto the block/wake hooks, not an executor)

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `PollIngressSingleResult`'s variant identifiers (used illustratively in the drive skeleton) | Code Examples | Trivial — compile error names them; semantics (packet-processed + state-changed bits) are documented on docs.rs |
| A2 | smoltcp `poll()` is allocation-free given preallocated socket buffers + compile-time pools + our fixed-pool adapters | Pattern 1 (seam) | Low for the seam design (we don't poll in IRQ regardless); matters only if a future plan revisits IRQ-context poll |
| A3 | The udp-socket path cannot source 0.0.0.0 without a configured interface address (P-G frames it as a risk, not a verified fact) | Pitfall P-G / OQ-1 | If wrong (it CAN), the DHCP gate is simpler than feared — the host pin resolves it either way before the guest gate |
| A4 | devip `snoop` file and `Qsnoop` queue semantics for ipifc are ungated this phase (documented in the tree, no milestone consumer exercises snoop) | Contract Surface | Low — D-08 enumerates it; the gates may legitimately leave snoop reads unexercised if the contract doc marks it so |
| A5 | The tick-sweep net arm (deadline check in `timer.rs tick()` → wake devip waiters) satisfies D-05's "one-shot timer" wording — logically one-shot arming over the existing 1 ms periodic tick | Pattern 1 | If the user reads D-05 as requiring a literal CNTP re-arm, the timer leg needs rework (rejected here with rationale — worth a plan-time confirmation note) |
| A6 | `cargo xtask check/test` host venue can compile smoltcp for the host target inside r9's flake (probe used the bare toolchain; the flake devshell parity is unverified this session) | Validation / env | Low — r9 already host-tests port; the dep is a normal crates.io entry |
| A7 | libthread for aarch64 is available for the aan payload (tier status not re-verified this session; Phase 6 swept libraries core tiers) | Consumer Surfaces (aan) | If missing, the aan gate blocks on a Phase 6/10 payload question — check at plan time |

**Everything else in this document is [VERIFIED] against in-session tool reads (docs.rs, crates.io API, the downloaded smoltcp-0.14.0 crate source, the compile probe, and direct reads of `sys/src/9k/ip/`, `sys/man/`, `sys/src/cmd/`, `sys/src/libc/`, and `../r9` source) or tagged [ASSUMED] inline.**

## Open Questions (RESOLVED — per-question dispositions below)

_All five questions were resolved at plan time (2026-11-05); each entry below carries its disposition and venue. The section is kept for the reasoning trail._

1. **OQ-1 — Can smoltcp's udp socket TX source 0.0.0.0 + broadcast with NO configured interface address?** (D-09's DHCP fidelity crux)
   - What we know: RX side verified (broadcast accepted by port-bound sockets; iface accepts broadcast-destined packets). smoltcp's own dhcpv4 socket hand-crafts src-0.0.0.0 packets at the wire layer (`socket/dhcpv4.rs:610`). The udp socket's source-selection path with zero configured addresses is unproven for this shape.
   - What's unclear: whether `update_ip_addrs` must be skipped for `0.0.0.0` (yes — it panics on non-unicast, verified) AND how the TX path then sources the packet.
   - Recommendation: the FIRST host-side pin of the phase (before any guest gate): build ipconfig's exact DHCP-shaped exchange against a host-side smoltcp instance — bind port 68, send a broadcast Discover with no iface address, observe. If it fails, the bridge's wire-layer shim (same construction as smoltcp's dhcpv4 socket internals) is the designed answer; the recorded fallback stays D-09's dhcpv4 socket.
   - **RESOLVED (plan venue):** Plan 08-02 Task 2 pin (h) implements exactly this as the phase's FIRST host pin (ScriptedDev captures the TX frame; src 0.0.0.0 present-or-absent is the verdict); the PASS-path/SHIM-path verdict is recorded as a dated line in `docs/devip-contract.md`'s mapping chapter.

2. **OQ-2 — `/dev/random` for ipconfig (the Phase-11 adjacency pulled forward).**
   - What we know: ipconfig's `init()` calls `srand(truerand())` unconditionally; libc's truerand sysfatals without `/dev/random`; r9 devcons has no random files; CONTEXT's deferred list places `/dev/random` in Phase 11 (AUTH-02's DTB-seeded CSPRNG).
   - What's unclear: whether Phase 8 lands a minimal devcons `random`/`urandom` (small, non-CSPRNG, loudly labeled) or the milestone re-scopes (pull AUTH-02's device forward / patch nothing — D-09/D-12 forbid patching ipconfig).
   - Recommendation: planner surfaces this to the user at plan check — it crosses a locked phase boundary (CONTEXT's deferred list). The minimal device is ~small and devcons-adjacent; AUTH-02's full CSPRNG stays Phase 11 either way.
   - **RESOLVED (user decision, CONTEXT.md D-17, 2026-11-05):** the minimal DTB-seeded CSPRNG lands THIS phase — devcons `random`/`urandom` files only, no devcap/auth adjacency; Phase 11 hardens/expands. Plan 08-06 Task 1 implements it BEFORE the first ipconfig gate; ipconfig stays unmodified (D-09).

3. **OQ-3 — Accept-pool depth for TCP listen.**
   - What we know: devip `Maxincall = 128`; smoltcp resets excess SYNs; each pool member costs rx+tx socket buffers.
   - What's unclear: the memory budget per conv at the phase's buffer sizes; whether 128-deep pools are affordable or a documented smaller deviation.
   - Recommendation: plan computes the byte math (pool depth × (rx+tx) per proto) and records the choice in the contract doc; D-08 pushes toward 128 unless the memory math forbids.
   - **RESOLVED (plan venue):** Plan 08-01 Task 1 — the contract doc's mapping chapter computes the byte math (pool depth × (rx+tx) socket-buffer bytes per announced conv vs r9's recorded kernel-heap budget from `../r9/port/src/allocator.rs`) and records the depth decision (devip-exact 128 per D-08, or the documented smaller deviation with the math shown).

4. **OQ-4 — `checksum n` and `tcpporthogdefense` ctl verbs have no smoltcp mechanism.**
   - What we know: verified absent from smoltcp's socket API (checksums are always computed; keepalives are interval-based, not random-seq probes).
   - Recommendation: contract doc maps them as accepted-and-recorded (netlog note) or loud-unimplemented — D-08's full-fidelity stance wants the former with the deviation documented; decide at contract-doc time.
   - **RESOLVED (plan venue):** Plan 08-01 Task 1 — the contract doc's TCP chapter maps both verbs as accepted-and-recorded with a netlog note (D-08's full-fidelity stance: acceptance + documented deviation, not loud-unimplemented).

5. **OQ-5 — Multiple smoltcp Interfaces vs one.**
   - What we know: one Interface owns one Device; devip allows multiple bound ipifcs (loopback + ether). A socket's egress rides the interface whose Device the socket's adapter belongs to; address-based demux (127/8, ::1 → loopback iface) is the seam's routing.
   - Recommendation: two Interfaces (loopback + ether) created at medium bind, socket-to-interface binding at conv connect/announce by address class; the contract doc's routing section records it. This is the agent's discretion; the plan should validate the demux against the loopback+ether gates early.
   - **RESOLVED (plan venue):** Plan 08-01 Task 1 records one smoltcp Interface per bound ipifc with address-class demux (127/8 and ::1 → loopback iface; everything else → ether iface) in the contract doc's routing chapter; plans 08-04 (loopback leg) and 08-06 (ether leg) implement it and their gates validate the demux.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-aarch64 | All guest gates | ✓ (nix store) | 11.1.2 (`/nix/store/dzdki5sj…/bin/qemu-system-aarch64`) | — |
| expect | composed expect smoke (D-13) | ✓ | /usr/bin/expect (macOS system expect) | — |
| nix | flake composition, vendor derivations | ✓ | `/nix/var/nix/profiles/default/bin/nix` | — |
| r9 Rust toolchain (nightly-2026-01-30) | all r9 work | ✓ (nix store, built) | `rust-default-1.95.0-nightly-2026-01-30` incl. `aarch64-unknown-none` rust-std | — |
| crates.io reachability | D-02 pin entry + vendor hash | ✓ (probe downloaded smoltcp 0.14.0 + closure) | — | flake fixed-output vendor derivation caches it thereafter |
| smoltcp 0.14.0 no_std aarch64 build | the whole phase | ✓ **probe-verified green** (both host + aarch64-unknown-none) | 0.14.0 | — |
| tap/bridge host plumbing | NETIP-08 truth verdict | Phase 9 venue (not Phase 8 scope) | — | D-15: slirp machinery-proof now, tap verdict in Phase 9 |
| plan9port host tools | Harvey payload builds (7c/7l host legs ride the existing subset-build.sh machinery) | ✓ (project harness) | — | — |

**Missing dependencies with no fallback:** none for Phase 8's own gates.
**Missing dependencies with fallback:** `/dev/random` in the guest (OQ-2 — not an environment item but a phase-scope item; listed here for visibility since ipconfig sysfatals without it). **Resolved by CONTEXT.md D-17:** the minimal DTB-seeded device lands in plan 08-06 Task 1 before the first ipconfig gate.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (auth is Phase 11; Phase 8 gates are numeric-only per CONTEXT) |
| V3 Session Management | no | — |
| V4 Access Control | yes (file-permission shape) | devip's owner-collapse + perm bits (0666→0660) are the conv access control — the contract doc's perm table + gates assert it; r9's single-user collapse is a documented era limit, not a hole |
| V5 Input Validation | yes | Every ctl write parses untrusted text: the devip ctl parsers must bound-check (donor's `parsecmd` 64-byte cmdbuf bound precedent at devether), reject malformed verbs with the exact error strings, and NEVER let a smoltcp panic cross the file boundary (P-F is the canonical case). All wire input arrives via smoltcp (upstream fuzz-tested) |
| V6 Cryptography | no | — (smoltcp does no crypto; the demo auth domain is Phase 11) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed network input → parser panic | Tampering/DoS | smoltcp's wire parsers are upstream fuzz-tested (rstest + fuzz harnesses in-crate); the devip bridge validates BEFORE smoltcp calls (non-unicast addrs, port 0, overflow) and maps smoltcp errors to devip error strings — a kernel panic from a network packet or a ctl write is a gate red |
| SYN flood on announced ports | DoS | smoltcp resets excess SYNs (no backlog) — the accept pool bounds memory by construction; devip's Maxincall=128 semantics documented in the contract doc |
| The guest's own DHCP/broadcast fidelity | Information disclosure (none) / DoS | ipconfig unmodified; slirp is hermetic (hostfwd none in Phase 8's r9-side gates; the composed smoke's netdev stanza is Phase 7's — no hostfwd added this phase) |
| Supply chain (first external crate in r9 port/) | Tampering | D-02: crates.io pin + committed Cargo.lock + flake fixed-output vendor; this document's legitimacy audit (all OK); push-before-pin discipline |
| Waker/bridge wedges (availability) | DoS (self) | P-C discipline + host pins; netlog observability from day one (D-07) |

## Sources

### Primary (HIGH confidence)
- docs.rs smoltcp 0.14.0 pages fetched this session: `iface/struct.Interface.html` (poll family, new), `phy/trait.Device.html` + `trait.RxToken.html` + `trait.TxToken.html`, `phy/struct.DeviceCapabilities.html`, `iface/struct.Config.html`, `iface/struct.SocketSet.html`, `iface/enum.PollResult.html`, `iface/struct.Routes.html`, `socket/tcp/struct.Socket.html`, `socket/udp/struct.Socket.html`, `socket/icmp/struct.Socket.html`, `socket/index.html`, `phy/struct.Loopback.html`
- crates.io API `/api/v1/crates/smoltcp/0.14.0` (feature graph, checksum, rust_version 1.91, publish date 2026-08-17)
- Downloaded crate source `smoltcp-0.14.0` (crates.io tarball): `src/iface/interface/mod.rs` (PollResult def, poll_at), `ipv4.rs` (icmp ingress + auto-reply + broadcast acceptance), `socket/icmp.rs` (accepts_v4/v6 Ident semantics), `socket/udp.rs` (accepts broadcast), `socket/tcp.rs` (accepts/State/RTTE bounds), `socket/dhcpv4.rs` (0.0.0.0 construction), `phy/loopback.rs` (per-packet alloc), `lib.rs` (sizing constants, no_std gating), `time.rs` (Instant/Duration), `storage/packet_buffer.rs`, `CHANGELOG.md` (0.13/0.14 deltas)
- **Compile probe** (this session, scratch crate): smoltcp 0.14.0 + the locked feature set, `#![no_std]`, green on `aarch64-apple-darwin` AND `aarch64-unknown-none` under r9's pinned `1.95.0-nightly-2026-01-30`; probe Cargo.lock captured the dependency closure
- Harvey tree (read directly): `sys/man/3/ip` (full), `sys/man/2/ip` (full), `sys/man/8/ipconfig` (full), `sys/src/9k/ip/` — `devip.c` (full), `tcp.c` (ctl/stats/state/tcpstates/tcpsettimer/tcpgc), `udp.c` (ctl/announce/connect/advise/Fsnewcall), `icmp.c` (connect/announce/kick/dispatch), `icmp6.c` (headers/full-packet semantics), `ipifc.c` (ctl verbs/status formats/ipselftab/ipifcbind), `iproute.c` (verbs/sprintroute), `arp.c` (arpread/arpwrite/aformat/states/NCACHE/Alinelen), `netlog.c` (flags/netlogctl), `ip.h` (Conv/Proto/Fs/Medium/Maxincall/states), `loopbackmedium.c`, `nullmedium.c`, `ethermedium.c` (bind dial triple, ares/areg), `sys/src/9k/port/error.h` (net error strings)
- Harvey consumers (read directly): `sys/src/libc/9sys/announce.c` (announce/listen), `sys/src/libc/9sys/truerand.c` (the /dev/random sysfatal), `sys/src/cmd/ip/ipconfig/main.c` (init/binddevice/doadd/dhcp paths), `sys/src/cmd/ip/ipconfig/ipv6.c` (dialicmp/ip6cfg/ra6 issuers/recvrahost arp write), `sys/src/cmd/ip/dhcpclient.c` (standalone surface), `sys/src/cmd/ip/ping.c` (protos/dial/geticmp), `sys/src/cmd/aan.c` (full)
- r9 tree (read directly): `port/src/devether.rs` (full), `port/src/netif.rs` (qid scheme, locking invariants, conv model), `port/src/devpipe.rs` (PipeHooks idiom), `port/src/rendez.rs` (it is the rendezvous(2) group — NOT a kernel sleep/wake primitive; the block/wake idiom lives in the hooks), `port/src/dat.rs` (DEV_* indices, Result convention), `port/src/namec.rs` (registry_attach/dev_walk), `port/src/lib.rs` (module list), `aarch64/src/main.rs` (hook registrations), `aarch64/src/platform/virt/timer.rs` (the 1 ms tick + tick_timeouts), `aarch64/src/platform/virt/pci.rs` (virtio_net_irq retire/demux), `aarch64/src/syscall.rs` (DEV_ETHER read/write arms), `xtask/src/main.rs` (boot-test profiles, negative scans, pcap module, net sweep modes), `Cargo.toml` (workspace lints, check-cfg predeclare rule), `rust-toolchain.toml`, `UNSAFE-AUDIT.md` (unsafe contract)
- Harness (read directly): `build/aarch64/smoke-r9.expect` (full — the staged-gate shape D-13 extends), `build/aarch64/netblast.c` (payload precedent), `build/aarch64/subset-build.sh` (payload build/bake asserts), `build/aarch64/root-tree/lib/{test/arp.req,net-sweep.rc}` (fixtures)

### Secondary (MEDIUM confidence)
- gsd-tools `package-legitimacy check` verdicts (registry metadata: age/downloads/repo) — corroborated by the compile probe's real download
- smoltcp GitHub PR numbers cited in CHANGELOG entries (referenced, not individually fetched)

### Tertiary (LOW confidence)
- None load-bearing; A1-A7 in the Assumptions Log carry the LOW-tier residue.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every smoltcp claim verified against 0.14.0 docs.rs + crate source + a live compile probe on r9's exact toolchain and target
- Architecture: HIGH for the seam's r9-side primitives (read from source) and the devip enumeration (read from source); MEDIUM-HIGH for the seam's *design* (the recommendation is reasoned from verified constraints, not from an existing implementation)
- Pitfalls: HIGH — the milestone pitfalls cross-checked against this design; the phase-specific list (P-A…P-M) is derived from verified API/source facts
- Consumer surfaces: HIGH (ipconfig/ping/aan/dhcpclient read directly; the /dev/random blocker is a source-verified finding, not a guess)

**Research date:** 2026-11-05
**Valid until:** 2026-12-05 (stable target: smoltcp 0.14.x line is current; re-verify only if the pin moves or smoltcp 0.15 ships before execution)







