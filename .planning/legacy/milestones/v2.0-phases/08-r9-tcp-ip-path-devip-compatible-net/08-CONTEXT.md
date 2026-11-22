# Phase 8: r9 TCP/IP Path — devip-Compatible /net - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning

<domain>
## Phase Boundary

r9 grows its kernel TCP/IP path: a **devip-exact `/net` file tree** in the r9 kernel over **smoltcp 0.14** — loopback + null mediums FIRST (the TCP state machine is debuggable without the NIC), then ethermedium binding Phase 7's devether as smoltcp's Device with `/net/arp` presented over smoltcp's neighbor cache, static + DHCP IPv4 via **unmodified** Harvey `ip/ipconfig`, full-fidelity IPv6 on the ether leg (ICMPv6/Neighbor Discovery), `/net/log` (netlog) + flag-guarded debug prints from day one, and aan proven with a scripted drop-and-resume gate on loopback. The exit story: a bidirectional TCP soak machinery proven on slirp in this phase, with the **tap-leg truth verdict executing in Phase 9** as a joint Phase-8/9 acceptance gate. **Kernel work in sibling repo `../r9`; `sys/src/9k/ip/` (22 files, ~15.4k lines C) is a read-only semantics reference.** Harvey-side touch: the bootfs gate payloads (ipconfig/ping/echo/tcpblast), the composed expect smoke, evidence consolidation, and the `flake.lock` r9 pin advance.

Addresses NETIP-01, NETIP-02, NETIP-03, NETIP-04, NETIP-05, NETIP-06, NETIP-07, NETIP-08.

NOT in this phase: host netdev modes `--net=slirp|tap|vmnet`, tap/bridge plumbing, doctor-checker (Phase 9 — but Phase 9 executes this phase's NETIP-08 tap verdict); cs/dns/ndb services and name-resolution gates (Phases 10/11 — Phase 8 gates are numeric-only); cpu/drawterm/rio (Phase 12 — aan's real venue); auth/devcap (Phase 11); virtio-blk/rng (future milestones).

</domain>

<decisions>
## Implementation Decisions

### NETIP-01 — TCP implementation (the decision gate)
- **D-01:** **smoltcp 0.14 behind a hand-built devip ABI** — over a staged Rust port of `sys/src/9k/ip` and over a hybrid. The `/net` file ABI is hand-built either way (the research's decisive finding: the file tree IS the contract — libc `dial()`/`announce()`/`listen()` are pure file I/O); smoltcp replaces the TCP/UDP/ICMP/ARP/ND machinery (Reno, timers, reassembly) with maintained, upstream-tested, `no_std` safe-Rust code. Verified current at discussion time: **0.14.0 (2026-08-17), MSRV 1.91** — cleared by r9's nightly-2026-01-30 pin. Diverges deliberately from Phase 7 D-01's from-scratch virtio choice: the virtio axis was transport machinery (small, auditable); the TCP axis is a correctness-sensitive protocol engine where upstream test coverage is the point. — **Reversibility:** costly — the devip↔smoltcp seam (event surface, socket-waker wakeups, neighbor-cache-as-`/net/arp`, Device adapter over devether) underpins every protocol gate; switching to the port later means re-homing all protocol engines behind the same file ABI.
- **D-02:** **Dependency entry = crates.io pin + committed Cargo.lock** — r9 `port/`'s first external crate. Not vendored in-repo, not git-SHA pinned. The lockfile hashes + the flake's fixed-output cargo vendor derivation are the supply-chain record. — **Reversibility:** reversible.
- **D-03:** **"devip-exact" proof = committed contract doc + targeted gates; NO donor A/B diff harness.** The contract doc derives from `sys/man` (`sys/man/3/ip`, `sys/man/2/ip`, `sys/man/8/ipconfig`) + `sys/src/9k/ip/` source and enumerates the full documented surface (per D-08); gates assert documented behaviors directly. — **Reversibility:** reversible — an A/B harness can be added later without invalidating anything.
- **D-04:** **NETIP-01 record placement follows the Phase 7 D-01 precedent**: an r9-side doc/commit plus this CONTEXT.md, recorded *together with* the virtio from-scratch provenance decision (NETIP-01 requires the same-axis decisions recorded together).

### smoltcp integration — event-driven (user-challenged, verified, locked)
- **D-05:** **Event-driven integration — NO periodic polling.** The user challenged the polling assumption ("just because it has `Interface::poll()` doesn't mean polling is the only way"); verification against smoltcp 0.14 confirms it: no internal threads, no tick requirement. `Interface::poll()` is invoked **exactly on**: (1) devether RX interrupt (frame ingress), (2) a **one-shot timer armed from `poll_at()`/`poll_delay()`** after each poll (drives smoltcp's retransmit/keepalive timers), (3) local socket activity from devip file ops. **Socket wakers** (`WakerRegistration::register_recv_waker`/`register_send_waker`) wake blocked devip readers/writers directly — no blocked-process polling. The single-ownership context for `&mut Interface`+`SocketSet` is designed event-first; exact seam mechanics go to phase research with this constraint locked. — **Reversibility:** costly — the wakeup/timer seam is woven into devip's blocking file semantics; a periodic-cadence fallback would be a seam rewrite (the protocol machinery above it is unaffected).

### Protocol scope + IPv6
- **D-06:** **Full-fidelity IPv6 — ether-leg with ICMPv6/Neighbor Discovery** via smoltcp `proto-ipv6`; the guest is v6-reachable from the host. The strongest reading of NETIP-06's "demonstrated"; overrides the research's port-path deferral of `ipv6.c`/`icmp6.c` (that deferral was written for the port's cost model, not smoltcp's).
- **D-07:** **netlog (`/net/log`) + flag-guarded serial debug prints land from day one** — Pitfall 15 posture: once workflows ride TCP, the stack bug wedges the channel used to observe it; observability is a deliverable, not an afterthought. Debug prints follow the repo convention (devcons-style static flag variables).
- **D-08:** **FULL documented devip surface is exact this phase** — all ctl verbs, all file formats, all error strings per `sys/man`, whether or not a milestone consumer touches them. Deliberate full-fidelity stance (consistent with D-06), overriding the measure-first subset recommendation (v1.0 D-14's discipline does not apply to the file-semantics surface here). The D-03 contract doc enumerates the complete surface from `sys/man` × `sys/src/9k/ip/` and lands FIRST; gates assert it. Dropped protocols (`gre/esp/rudp/igmp/ipmux/pktmedium/netdevmedium`) are simply absent from the file tree — no stub dirs.

### DHCP + addressing
- **D-09:** **DHCP gate = Harvey's real `ip/ipconfig` dhcpclient over `/net/udp` broadcast fidelity** — the path `cpurc` will actually run in Phase 12. smoltcp's dhcpv4 socket is **never wired** unless the ipconfig path fails; if the fallback is ever taken it is recorded per NETIP-05's text. — **Reversibility:** reversible.
- **D-10:** **DHCP + static gates run on slirp now** (10.0.2.x, hermetic, unprivileged, CI-able); both are **re-demonstrated on the tap leg** when Phase 9 lands the venue. Every slirp-green gate is explicitly labeled not-L2-proof in the evidence (Pitfall 9).
- **D-11:** **Single committed addressing config seam** — one harness-side data file defines guest/host addresses per leg (loopback/slirp, later tap); baked ndb/cpurc fragments and gate assertions derive from it. Phase 9 lifts the seam into HNET-02's TOML single-sourcing — the Phase 7 D-09 pattern (minimal wiring now, flag shape Phase 9 will adopt). **No scattered `10.0.2.x` hardcodes** — Pitfall 9's named failure mode.

### Gate machinery + venue
- **D-12:** **Gate binaries are Phase-8-owned bootfs payloads** — `ip/ipconfig`, `ip/ping`, and the echo/tcpblast binaries are built by phase-8 harness rules and baked into bootfs per the Phase 7 `arp.req`/`netblast.c` precedent. **NOT tier-allowlist appends** — Phase 6's tier data files stay pure; Phase 10's tier-2 services sweep remains the tier entry point for `sys/src/cmd/ip/`.
- **D-13:** **Layered gates** (v1.0 D-12 + Phase 6 D-12 hold): r9 xtask boot-test gains net profiles asserting kernel/stack serial markers (devip bind, medium registration, smoltcp poll health, netlog activity); Harvey gains a composed **expect-gated** smoke sibling driving the user-visible flows (`ping 127.0.0.1`, udp echo, in-guest tcp/7 echo, ipconfig static + dhcp runs) — NETIP-03's "committed expect gates" wording.
- **D-14:** **aan (NETIP-07) = compile for aarch64 + scripted reconnect gate on loopback**: aan client+server pair in-guest over tcp loopback, scripted hangup of the underlying `/net/tcp/N` conn, gate asserts the tunneled session resumes. The value proposition proven NIC-free; cpu-service integration stays Phase 12.
- **D-15:** **NETIP-08 sequencing: machinery + slirp soak in Phase 8; tap verdict in Phase 9.** Phase 8 builds the bidirectional TCP soak machinery (tcpblast-style payload, host-asserted via pcap + guest counters, TCG-scaled counts — the Phase 7 netblast precedent) and proves it on slirp, explicitly labeled not-L2-truth. The tap-leg truth verdict executes in Phase 9 as a **joint Phase-8/9 acceptance gate** using this phase's machinery — the roadmap's own cross-phase framing ("this phase's tap leg is the venue for Phase 8's NETIP-08 exit gate").
- **D-16:** **CI shape: short gates in hosted CI; the sustained soak is documented-manual on dev hosts.** Short gates = loopback ping/echo, ipconfig static+dhcp, brief TCP exchange — extending `r9-shell.yml` per the Phase 6/7 precedent. Deliberate override of Phase 6 D-16's "everything in CI unless measured over-budget" **for the soak specifically**: the aarch64-guest soak is TCG on every automated venue (x86_64 CI; darwin HVF rejects GICv2), and CI wall-clock honesty beats a flaky mega-gate. The exact dev-host soak command must be documented in the phase evidence or the soak silently never runs. — **Reversibility:** reversible — a TCG-scaled CI soak can be added if the manual discipline rots.
- **D-17:** **Minimal DTB-seeded CSPRNG lands in r9 THIS phase** (`/dev/random` + `/dev/urandom`), resolving research OQ-2/Pitfall P-I (user decision at plan-phase kickoff 2026-11-05): every `ipconfig` invocation calls `srand(truerand())` unconditionally and libc's `truerand()` sysfatals without `/dev/random`; r9 has none today. Scope stays minimal — the random device only, no devcap/auth adjacency. Phase 11 hardens/expands it (the deferred `/dev/random` entry below is amended accordingly). Keeps ipconfig unmodified (D-09) and the NETIP-04/05/06 gates on schedule. — **Reversibility:** reversible — a small additive device; Phase 11 supersedes the implementation.

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 8 — goal, success criteria 1–5, requirement mapping; §Phase 9 depends-on for the tap-leg venue sharing
- `.planning/REQUIREMENTS.md` — NETIP-01..08 full text; Out of Scope table (slirp-as-correctness-target excluded, DHCPv6/SLAAC excluded); traceability
- `.planning/PROJECT.md` — Key Decisions (r9 adoption, cross-repo flake composition, unsafe-elimination ethos), Constraints
- `.planning/STATE.md` — current focus, accumulated context

### v2.0 milestone research (the phase's design source)
- `.planning/research/SUMMARY.md` — the smoltcp-vs-port open decision + synthesizer recommendation (resolved by D-01); "Plan 9 networking is a file-interface contract, not a syscall API" finding
- `.planning/research/PITFALLS.md` — Pitfall 6 (feature-negotiation/offload landmines), Pitfall 7 (virtqueue memory ordering), Pitfall 9 (slirp masks L2 bugs; hardcoded 10.0.2.x), Pitfall 15 (debugging TCP through the session that rides it — D-07's motivation), Pitfall 16 (cross-repo drift)
- `.planning/research/ARCHITECTURE.md` — devip component inventory + the `/net` file-contract finding; the staged-port plan (implementation superseded by D-01, but the file-interface enumeration and deferral lists stand); the (a)+(c) option analysis
- `.planning/research/FEATURES.md` §2 — Guest Networking table stakes

### Prior-phase decisions and debts
- `.planning/phases/07-r9-pci-ecam-virtio-net-devether/07-CONTEXT.md` — virtio/devether conventions (D-01 from-scratch, D-03 modern-only, D-05/06 filter-dump capture, D-09 minimal wiring pattern, D-10 always-on NIC), payload precedent, 12-byte hdr errata; `07-EVIDENCE.md` — the evidence standard in action
- `.planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-CONTEXT.md` — D-12 (layered r9/Harvey gates), D-13/D-14 (allowlist/markers purity — D-12 here honors), D-16 (CI default — D-16 here's scoped override)
- `.planning/milestones/v1.0-phases/05-shell-boot-one-command-integration/05-CONTEXT.md` — D-12 (r9-side gates stay r9-side), D-13 (CI = linux leg), sourced-not-typed sweep lesson
- `.planning/milestones/v1.0-phases/04-r9-userspace-exec-harvey-r9-abi/04-CONTEXT.md` — proof standard for r9 kernel changes
- `.planning/milestones/v1.0-phases/02-aarch64-toolchain-libc-9front-donor/02-CONTEXT.md` — D-14 measure-first discipline (D-08 here's deliberate scope exception), vendor-and-own supply-chain pattern

### Semantics references (Harvey tree — the contract being reimplemented)
- `sys/man/3/ip` + `sys/man/2/ip` + `sys/man/8/ipconfig` — the documented devip surface D-08 makes exact (the D-03 contract doc's primary source)
- `sys/src/9k/ip/` — 22 files ~15.4k lines C, read-only: `devip.c` (device shell/file dispatch), `ip.c`, `ipifc.c`, `iproute.c`, `tcp.c` (3,486-line state machine), `udp.c`, `icmp.c`, `icmp6.c`, `ipv6.c`, `arp.c`, `ethermedium.c`, `loopbackmedium.c`, `nullmedium.c`, `netlog.c`, `chandial.c`, `ipaux.c`, `ptclbsum.c`, `ip.h`/`ipv6.h`
- `sys/src/libc/9sys/announce.c` + `dial.c` — the unmodified consumer contract (pure `/net` file I/O)
- `sys/src/cmd/ip/ipconfig/` (`main.c` 2.1k lines, `dhcpclient.c`, `ipv6.c`) — the D-09 DHCP gate binary, unmodified
- `sys/src/cmd/ip/ping.c` — the NETIP-03 gate binary
- `sys/src/cmd/aan.c` — the NETIP-07 payload
- `sys/src/9k/port/netif.h` + `netif.c` — the ether framework devether already implements (the ethermedium binding target's semantics)

### r9 tree (where the work lands)
- `../r9/port/src/devether.rs` + `netif.rs` + `virtio.rs` — Phase 7's landing: `#l` device, `/net/ether0`, the DMA boundary smoltcp's Device adapter sits on
- `../r9/port/src/dat.rs` — `Dev` trait + dev-boundary `Result<T, &'static str>` convention devip follows
- `../r9/port/src/devcons.rs` — the flag-guarded debug-print convention D-07 follows
- `../r9/port/src/allocator_tests.rs` + `virtio_tests.rs` + `rx_chain_tests.rs` — host-side unit-pin precedents for the seam's waker/timer pins
- `../r9/xtask/src/main.rs` — boot-test profiles + the always-on netdev stanza (Phase 7 D-09) + pcap assertion machinery
- `../r9/UNSAFE-AUDIT.md` — the unsafe posture; `../r9/HowItWorks.md` — architecture overview; `../r9/Cargo.lock` + `rust-toolchain.toml` — where D-02 lands

### External specs (researcher pulls exact sections)
- smoltcp 0.14 (docs.rs) — `Interface::poll`/`poll_at`/`poll_delay`, `WakerRegistration` (`register_recv_waker`/`register_send_waker`), `medium-ethernet`/`proto-ipv4`/`proto-ipv6` features, socket-tcp/udp/icmp API, dhcpv4 socket (unwired fallback per D-09), the `Device` trait the devether adapter implements
- Virtio 1.x specification — as carried from Phase 7 (zero-offload mask, 12-byte hdr)

### Harness + evidence (Harvey repo)
- `build/aarch64/` — bake harness (`mkimage.sh` real-init rodata guard) for the gate payloads; `build/aarch64/README.md` qemu drift notes
- `build/aarch64/smoke-r9.expect` + `nix/boot-r9.sh` + `nix/smoke-r9.sh` — the composed smoke D-13 extends
- `flake.nix` — the r9 input contract (`r9.packages.<system>.kernel-virt`); pin advance mechanics
- `.github/workflows/r9-shell.yml` — the CI shape the short net gates extend (D-16)
- `.planning/phases/07-r9-pci-ecam-virtio-net-devether/07-EVIDENCE.md` — evidence consolidation example to mirror in `08-EVIDENCE.md`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../r9/port/src/{devether.rs, netif.rs, virtio.rs}` + `../r9/aarch64/src/platform/virt/pci.rs` — Phase 7's complete landing: the NIC, the `#l` file tree, and the discovery path the ethermedium binding consumes
- `../r9/xtask` boot-test machinery — profiles, serial-marker assertions, negative panic/Unhandled-interrupt scan, and the Phase 7 filter-dump pcap assertion the TCP/ARP gates extend
- smoltcp's waker API + `poll_at()`/`poll_delay()` — the event surface D-05 locks onto (no periodic tick needed)
- Harvey `build/aarch64/` bake harness + the sourced-not-typed in-guest rc sweep (Phase 6 D-10) + Phase 7's payload precedent (`arp.req`, `netblast.c`)
- `sys/src/9k/ip/` — the semantics gold mine: `devip.c`'s file dispatch and `tcp.c`'s state machine are the behavioral reference for the D-03 contract doc
- libc `announce()`/`dial()`/`listen()` — the unmodified consumer contract AND the natural test clients for the gates
- `../r9/port/src/{allocator_tests.rs, virtio_tests.rs, rx_chain_tests.rs}` — host-side `cargo test` pin precedent for the event seam's waker/timer unit tests

### Established Patterns
- **File-interface-is-the-contract** (the research's decisive finding): no net syscalls exist; whoever presents devip's file tree owns TCP/IP
- **Event-driven everything in r9** (IRQ/waker/one-shot timer) — D-05 makes the net stack a citizen, not a polling exception
- Execution-gated ordering: land gates in dependency order so every commit stays bootable (Phase 7 D-10's mid-phase-wedge watch-out)
- Evidence = serial markers + committed artifacts, timing-tolerant presence assertions, TCG-generous timeouts
- Phase-owned bootfs payloads keep tier allowlists pure (Phase 7 precedent; D-12)
- Single-source-as-data (D-11 addressing seam now; Phase 9 TOML later)
- **Contract-doc-as-spec** — new pattern this phase: the written devip-semantics doc lands first, gates assert it (D-03/D-08)
- r9-side gates stay r9-side; Harvey composes; pin advance as own commit

### Integration Points
- `../r9/port/src/` — new devip device (`#I`) + protocol modules; devether becomes smoltcp's `Device`
- `../r9/xtask/src/main.rs` — net boot-test profiles; the netdev stanza is already always-on (Phase 7)
- Harvey `build/aarch64/` — gate payloads (ipconfig/ping/echo/tcpblast) + sweep scripts
- Harvey composed smoke (`boot-r9-smoke` sibling) — the expect-gated NETIP-03 flows
- `flake.lock` — r9 pin advance (own commit, fresh hosted CI)
- Phase 9 seam: the tap leg executes NETIP-08's truth verdict with this phase's soak machinery (joint acceptance)

</code_context>

<specifics>
## Specific Ideas

- **The acceptance story, end-to-end:** the D-03 contract doc lands first (full documented devip surface enumerated from `sys/man` × `sys/src/9k/ip/`, reviewed) → smoltcp enters via crates.io pin, `Cargo.lock` committed, NETIP-01 recorded r9-side together with the virtio provenance note (D-04) → devip `#I` shell + loopback/null mediums on the event-driven seam (RX-IRQ/poll_at one-shot/wakers; seam unit pins host-side first) → icmp+udp+tcp over loopback: `ping 127.0.0.1`, udp echo, tcp/7 echo as committed expect gates (NETIP-03) → netlog + debug flags live (D-07) → ethermedium binds devether as smoltcp's `Device`; `/net/arp` views the neighbor cache → static ipconfig on slirp (NETIP-04) → `ipconfig` dhcpclient broadcast fidelity (NETIP-05; the dhcpv4 socket stays unwired) → ether-leg IPv6 with ND, guest v6-reachable from host (NETIP-06) → aan loopback drop-and-resume gate (NETIP-07) → tcpblast bidirectional soak on slirp, labeled not-L2-truth (NETIP-08 machinery proof) → evidence + r9 SHA consolidated in `08-EVIDENCE.md`; `flake.lock` pin advances as its own commit with fresh hosted CI → Phase 9 re-runs DHCP + static + the same soak on tap for the truth verdict.
- **Watch-out:** smoltcp owns ARP/ND internally — `/net/arp` must be a faithful *view* (reads reflect the neighbor cache; ctl verbs per the contract doc); a naive passthrough drifts from devip semantics at exactly the boundary scripts touch.
- **Watch-out:** the event-driven seam has no polling cadence to save you — a missed waker or a mis-armed `poll_at` timer wedges blocked readers silently. The seam's unit pins (host-side, the `allocator_tests.rs` precedent) land before any boot gate depends on it.
- **Watch-out:** ipconfig runs unmodified (D-09/D-12) — its `/net/udp` broadcast path exercises exactly the fidelity corners (broadcast Rx, udp headers mode) a minimal stack would skip; expect the first DHCP gate run to be the contract doc's real stress test.
- **Watch-out:** TCG everywhere for the soak (x86_64 CI, darwin HVF-rejected) — D-16's manual-only discipline needs the exact dev-host command documented in the phase evidence or the soak silently never runs.
- **Watch-out:** Pitfall 9 — slirp's DHCP/ARP forgive L2 bugs; every slirp-green gate is labeled not-L2-truth in the evidence, and Phase 9's tap acceptance re-runs DHCP + static + soak.
- **Watch-out:** the full documented devip surface (D-08) is the phase's cost center — the contract doc lands FIRST and is reviewed, or gate-writing chases an unbounded target.
- **User overrides to honor downstream (deliberate, don't second-guess):** smoltcp over the port on this axis despite Phase 7's opposite virtio call (D-01 — correctness-sensitive protocol engine vs transport machinery); full documented devip surface over measure-first subset (D-08); full-fidelity ether-leg IPv6 over loopback-only (D-06); ipconfig-dhcpclient-as-gate over smoltcp dhcpv4 primary (D-09); soak manual-only over TCG-scaled CI soak (D-16); event-driven locked over a periodic poller (D-05 — user-challenged assumption, verified against the smoltcp 0.14 API: no tick requirement; wakers + `poll_at` suffice).
- **Verified at discussion time:** smoltcp current = 0.14.0 (2026-08-17, crates.io max_version); MSRV 1.91; `no_std`, heap-optional — the research's 0.14 pin and MSRV clearance stand.

</specifics>

<deferred>
## Deferred Ideas

- Host netdev modes `--net=slirp|tap|vmnet`, netdev stanza TOML single-sourcing (HNET-02), linux tap/bridge docs, darwin socket_vmnet, doctor-checker — Phase 9 (HNET-01..05); Phase 9 also executes this phase's NETIP-08 tap verdict (D-15)
- cs/dns/ndb service integration + name-resolution gates — Phase 10 (services tier) / Phase 11 (auth domain); Phase 8 gates are numeric-only
- cpu service + drawterm + rio — Phase 12 (aan's real venue; D-14's loopback gate is the Phase-8 proof)
- auth/devcap/`#!` exec — Phase 11; `/dev/random` — **amended by D-17**: a minimal DTB-seeded CSPRNG device lands in THIS phase (ipconfig hard-blocker); Phase 11 hardens/expands it. virtio-rng-pci — later (carried from Phase 7)
- smoltcp dhcpv4 socket wiring — only if D-09's ipconfig path fails; the recorded fallback per NETIP-05
- `gre/esp/rudp/igmp/ipmux/pktmedium/netdevmedium` protocols — dropped per research; no milestone consumer; absent from the file tree (D-08)
- Donor A/B diff harness for devip fidelity — rejected (D-03); contract doc + targeted gates instead
- TCG-scaled CI soak leg — rejected for now (D-16 override); revisit if the manual discipline rots
- virtio-blk, virtio-9p host mount, SMP/PSCI, RTC — future milestones (carried)
- EVENT_IDX notification suppression; MSI/GICv2m — carried from Phase 7 deferred (non-goals for v2.0)

</deferred>

---

*Phase: 8-r9 TCP/IP Path — devip-Compatible /net*
*Context gathered: 2026-11-05*
