# Phase 8: r9 TCP/IP Path — devip-Compatible /net - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-05
**Phase:** 8-r9 TCP/IP Path — devip-Compatible /net
**Areas discussed:** TCP implementation choice (NETIP-01), Protocol scope + IPv6 depth, DHCP strategy + addressing, Gate machinery + aan + exit-gate shape, smoltcp drive model, Conv-semantics fidelity depth, CI/venue shape for TCG soak

---

## TCP implementation choice (NETIP-01)

### Q1: Which TCP implementation path does r9 take?

| Option | Description | Selected |
|--------|-------------|----------|
| Staged Rust port of 9k/ip | Hand-port the IPv4 subset (devip/ip/ipifc/iproute/ipaux/chandial/netlog/ptclbsum + tcp/udp/icmp/arp + loopback/null/ether); devip-exact by construction, zero deps, consistent with Phase 7 D-01; you own Reno/timers/reassembly | |
| smoltcp 0.14 behind devip ABI | medium-ethernet + proto-ipv4 + socket-tcp/udp/icmp + alloc behind a hand-built file ABI; upstream-tested machinery; first external crate in r9 port/; fidelity risk at the devip↔smoltcp seam | ✓ |
| Hybrid: hand-built shell, smoltcp engines | Port devip/ipifc/mediums + icmp/arp by hand, smoltcp sockets as TCP/UDP engine; unvalidated by research, likely fights smoltcp's IP-layer ownership | |

**User's choice:** smoltcp 0.14 behind devip ABI
**Notes:** The research synthesizer's recommendation. Diverges deliberately from Phase 7's from-scratch virtio call — different axis: correctness-sensitive protocol engine vs transport machinery.

### Q2: How does smoltcp enter r9's dependency tree?

| Option | Description | Selected |
|--------|-------------|----------|
| crates.io pin + committed lockfile | Cargo.toml version req + committed Cargo.lock; flake fixed-output derivation pins the vendor tree | ✓ |
| Vendor in-repo + provenance README | cargo vendor into r9, Phase 2 9front-donor pattern; byte-for-byte in-repo control | |
| Git-SHA pinned dependency | rev-locked git dep; content-addressed but complicates nix sandboxed builds | |

**User's choice:** crates.io pin + committed lockfile

### Q3: How is 'devip-exact' execution-proven?

| Option | Description | Selected |
|--------|-------------|----------|
| Contract doc + donor A/B gates (Recommended) | Committed semantics doc + identical in-guest sweeps against donor devip and r9 devip, diffed | |
| Contract doc + targeted gates only | Doc is the spec; gates assert documented behaviors directly; doc fidelity is reviewed, not execution-proven | ✓ |
| Donor A/B diff only | The diff harness IS the spec; least doc debt, worst failure legibility | |

**User's choice:** Contract doc + targeted gates only
**Notes:** Recorded as agent discretion (citing Phase 7 D-01 precedent): NETIP-01's "recorded together with virtio provenance" lands r9-side + in CONTEXT.md.

---

## Protocol scope + IPv6 depth

### Q1: NETIP-06 scope — how deep does IPv6 go?

| Option | Description | Selected |
|--------|-------------|----------|
| ip6 tree + static config + v6 loopback ping (Recommended) | Smallest honest reading; no ether-leg IPv6, no ND over the NIC | |
| + ether-leg IPv6 with ND | Loopback proof PLUS smoltcp proto-ipv6 ICMPv6/Neighbor Discovery against the host; guest v6-reachable | ✓ |
| Tree + config only, no v6 traffic | Correct clone/ctl/status shape, no packets flow; risks under-delivering "demonstrated" | |

**User's choice:** + ether-leg IPv6 with ND
**Notes:** Full-fidelity lean also recorded for /net/arp's presentation over smoltcp's neighbor cache (agent discretion).

### Q2: What debug/observability surface lands with the stack?

| Option | Description | Selected |
|--------|-------------|----------|
| netlog + debug flags from day one (Recommended) | /net/log + flag-guarded serial prints per repo convention; Pitfall 15 posture | ✓ |
| Serial debug prints only | /net/log deferred; in-guest post-mortem inspection weakened | |
| pcap + smoltcp tracing only | Lightest; weakest against Pitfall 15 | |

**User's choice:** netlog + debug flags from day one

---

## DHCP strategy + addressing

### Q1: NETIP-05 — which DHCP path does this phase gate?

| Option | Description | Selected |
|--------|-------------|----------|
| ipconfig dhcpclient is the gate (Recommended) | Harvey's real ipconfig over /net/udp broadcast fidelity; smoltcp dhcpv4 never wired unless this fails (recorded per NETIP-05) | ✓ |
| smoltcp dhcpv4 socket as primary | Kernel-side DHCP, ipconfig bypassed; cpurc's real path unproven until later | |
| Both paths gated | Belt-and-suspenders; double the DHCP surface in one phase | |

**User's choice:** ipconfig dhcpclient is the gate

### Q2: Where does the DHCP gate run?

| Option | Description | Selected |
|--------|-------------|----------|
| slirp now, re-prove on tap in Phase 9 (Recommended) | slirp's built-in DHCP (10.0.2.x) hermetic in CI; tap re-proof when Phase 9 lands the venue; slirp-DHCP documented as not-L2-proof | ✓ |
| DHCP waits for real tap L2 | Phase 8 carries provisional tap+dnsmasq; pulls Phase 9 scope | |
| slirp only, no tap re-proof | Pitfall 9 risk accepted and recorded | |

**User's choice:** slirp now, re-prove on tap in Phase 9

### Q3: How is per-leg addressing sourced for the gates?

| Option | Description | Selected |
|--------|-------------|----------|
| Single config seam now, TOML in Phase 9 (Recommended) | One committed harness-side data file per leg; baked ndb/cpurc + gate assertions derive from it; Phase 9 lifts to HNET-02 TOML | ✓ |
| Hardcode slirp addressing for now | 10.0.2.x in gate scripts, documented; Pitfall 9's exact failure baked in | |
| Full TOML single-sourcing now | Pulls HNET-02 machinery forward; couples Phase 8 to Phase 9's design | |

**User's choice:** Single config seam now, TOML in Phase 9

---

## Gate machinery, aan + exit-gate shape

### Q1: How do the gate binaries get into the guest?

| Option | Description | Selected |
|--------|-------------|----------|
| Phase-8-owned bootfs payloads (Recommended) | ipconfig/ping/echo built by phase-8 harness rules per the Phase 7 netblast/arp.req precedent; tier allowlists stay pure for Phase 10 | ✓ |
| Early tier-allowlist appends | Phase-8-owned lines in the tier data files; full tier gate treatment immediately | |
| Minimal ctl-writer only | Raw ipifc ctl writes, no ipconfig; likely under-delivers NETIP-04/05 | |

**User's choice:** Phase-8-owned bootfs payloads

### Q2: Where do Phase 8's gates live?

| Option | Description | Selected |
|--------|-------------|----------|
| Layered: r9 profiles + Harvey expect smoke (Recommended) | r9 xtask net profiles for kernel/stack markers; Harvey composed expect smoke for user-visible flows (NETIP-03's "committed expect gates") | ✓ |
| All r9-side xtask | Consolidates gate code; blurs v1.0 D-12's repo boundary | |
| All Harvey-side smoke | Loses kernel-level marker granularity (Phase 7 RX-offset class) | |

**User's choice:** Layered: r9 profiles + Harvey expect smoke

### Q3: NETIP-07 — what does 'aan available' demonstrably mean?

| Option | Description | Selected |
|--------|-------------|----------|
| Compile + scripted reconnect gate on loopback (Recommended) | aan client+server in-guest, scripted hangup of the underlying conn, gate asserts resumption; value proposition proven NIC-free | ✓ |
| Compile + establishment smoke only | No forced-drop test; reconnect first exercised in Phase 12 | |
| Compile-only this phase | Runtime proof waits for Phase 12's cpu service | |

**User's choice:** Compile + scripted reconnect gate on loopback

### Q4: NETIP-08 — where does the sustained-bidirectional-TCP exit gate execute?

| Option | Description | Selected |
|--------|-------------|----------|
| Machinery + slirp soak in P8; tap verdict in P9 (Recommended) | Phase 8 builds tcpblast machinery, proves on slirp (labeled not-L2-truth); Phase 9 runs the truth verdict on tap as joint acceptance | ✓ |
| Provisional tap in Phase 8 | Pulls HNET-03 plumbing forward; duplicates Phase 9 | |
| All soak work waits for Phase 9 | TCP bulk-transfer correctness unexercised until tap exists | |

**User's choice:** Machinery + slirp soak in P8; tap verdict in P9

---

## smoltcp drive model (follow-up area)

### Q1/Q2: How does r9 drive smoltcp's stack?

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated poller proc owns the stack (initial recommendation) | Kernel poller woken by RX IRQ + timer; file ops block on Rendez | |
| Global stack lock, poll-on-entry | Least machinery; lock convoys; convoluted blocked-reader wakeup | |
| Researcher/planner decides | Constraint-free delegation | |
| Event-driven, locked (Recommended after verification) | No periodic polling: poll() on RX interrupt + one-shot poll_at()/poll_delay() timer + local socket activity; socket wakers (register_recv_waker/send_waker) wake blocked readers/writers | ✓ |

**User's choice (freeform, then confirmed):** "smoltcp advertises that it is an event-driven TCP/IP stack and as such we should investigate how properly hook into the event surface on our side to be event-driven as well. Do we absolutely need a polling mechanism here? let's question that assumption here, just because it has Interface::poll() doesn't mean polling is the only way to interact."
**Notes:** User-challenged assumption verified against smoltcp 0.14 (current 0.14.0, 2026-08-17; MSRV 1.91): no internal threads, no tick requirement; poll() is the event-processing entry point invoked exactly on events; WakerRegistration + poll_at()/poll_delay() provide the event surface. Confirmed "Event-driven, locked" — single-ownership seam mechanics go to phase research with the constraint locked.

---

## Conv-semantics fidelity depth (follow-up area)

### Q1: How much of devip's conversation semantics must be exact?

| Option | Description | Selected |
|--------|-------------|----------|
| Measure-first: traced + named consumers only (Recommended) | v1.0 D-14 discipline applied to /net; loud-unimplement the rest | |
| Full documented devip surface | Everything sys/man documents — all ctl verbs, file formats, error strings — exact, consumer-tracing or not | ✓ |
| Milestone-path minimum | Only Phase 8 gates + Phase 12 cpu needs; risks Pitfall 1 whack-a-mole in Phase 10/11 | |

**User's choice:** Full documented devip surface
**Notes:** Deliberate override of the measure-first recommendation, consistent with the full-fidelity IPv6 answer. The contract doc enumerates the complete surface and lands FIRST.

---

## CI/venue shape for TCG soak (follow-up area)

### Q1: Where does the TCG TCP soak run?

| Option | Description | Selected |
|--------|-------------|----------|
| TCG-scaled soak in CI, heavy soak manual (Recommended) | Full-pipeline-in-CI per Phase 6 D-16; TCG-scaled counts, presence-based assertions | |
| Short gates in CI; soak manual-only | CI: loopback ping/echo, ipconfig static+dhcp, brief TCP; sustained soak documented-manual on dev hosts | ✓ |
| New dedicated net workflow | Separate workflow owns all Phase 8 gates; more duplication | |

**User's choice:** Short gates in CI; soak manual-only
**Notes:** Deliberate override of Phase 6 D-16's default for the soak specifically — aarch64-guest soak is TCG on every automated venue (x86_64 CI; darwin HVF rejects GICv2). Recorded with the tension noted; reversible.

---

## the agent's Discretion

- smoltcp feature set within the locked shape (dhcpv4 stays disabled per D-09) and exact 0.14.x pin — researcher verifies
- Event-driven seam mechanics: &mut Interface+SocketSet ownership context, waker→Rendez bridge, one-shot timer wiring (event-driven constraint locked)
- devip module layout in ../r9/port/src/ and Fs/Proto-style dispatch mapping
- /net/arp ctl-verb depth (full-fidelity lean per D-08)
- Medium registration order within loopback+null-first; ethermedium Device adapter shape
- tcpblast payload shape and TCG-scaled counts (netblast precedent)
- Addressing seam file format/location
- CI job placement within r9-shell.yml
- Contract doc home (r9-side docs/ recommended) and enumeration format
- Payload build mechanics for ipconfig/ping/echo/tcpblast
- NETIP-01 r9-side record placement (Phase 7 D-01 precedent)
- Name-resolution: gates numeric-only; cs/dns deferred to Phase 10/11 (area not selected for discussion)

## Deferred Ideas

- Host netdev modes + TOML single-sourcing + tap/bridge + socket_vmnet + doctor-checker — Phase 9 (which also executes the NETIP-08 tap verdict)
- cs/dns/ndb service integration + name-resolution gates — Phase 10/11
- cpu service + drawterm + rio — Phase 12 (aan's real venue)
- auth/devcap/#! exec//dev/random — Phase 11; virtio-rng later
- smoltcp dhcpv4 wiring — recorded fallback only if the ipconfig path fails
- gre/esp/rudp/igmp/ipmux/pktmedium/netdevmedium — absent from the file tree (no milestone consumer)
- Donor A/B diff harness — rejected; contract doc + targeted gates instead
- TCG-scaled CI soak — rejected for now; revisit if the manual discipline rots
- virtio-blk, virtio-9p, SMP/PSCI, RTC — future milestones
- EVENT_IDX, MSI/GICv2m — v2.0 non-goals (carried from Phase 7)
