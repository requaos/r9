# Phase 7: r9 PCI/ECAM + virtio-net + devether - Context

**Gathered:** 2026-11-04
**Status:** Ready for planning

<domain>
## Phase Boundary

r9's first DMA-capable device: DTB-driven PCI/ECAM discovery on qemu virt, a **from-scratch** virtio transport + virtio-net driver with Day-1 zero-offload discipline over barrier-first split virtqueues, and devether `#l` presenting `/net/ether0` per `netif.h` with live RX/TX counters — proven by a guest ARP request captured in a host-side pcap and interrupt accounting reconciled under load. **Kernel work in sibling repo `../r9`.** Harvey-side touch: the bootfs test payload (baked ARP frame + blaster binary), evidence consolidation, and the `flake.lock` r9 pin advance.

Addresses NETDRV-01, NETDRV-02, NETDRV-03, NETDRV-04.

NOT in this phase: devip/TCP/UDP/ICMP and the arp/ethermedium/ipifc binding to devether (Phase 8); host netdev modes `--net=slirp|tap|vmnet`, tap/bridge and socket_vmnet plumbing, doctor-checker (Phase 9); services/GUI userspace tiers (Phase 10); auth/devcap/`#!` exec/`/dev/random` (Phase 11); cpu/drawterm/rio (Phase 12). smoltcp vs port-of-9k/ip is Phase 8's NETIP-01 decision gate — unaffected by anything here.

</domain>

<decisions>
## Implementation Decisions

### Virtio provenance + driver shape
- **D-01:** **From-scratch `port/src/virtio.rs` — no external crate.** Deliberate user override of the milestone-research recommendation (virtio-drivers 0.13). Rationale: full control, zero external unsafe, matches r9's maximal-unsafe-elimination ethos (`UNSAFE-AUDIT.md`; `undocumented_unsafe_blocks = deny` across the aarch64+port workspace). This satisfies NETDRV-02's "provenance decision recorded in-repo" — record it in an r9-side doc/commit as well as here. Stale-caveat note for the record: the research's VERSION_1 concern was resolved upstream (accept-if-offered merged 2025-09, in 0.12.0+), so the crate was viable — the override stands on control/audit grounds, not on the caveat. — **Reversibility:** costly — the ring/transport machinery underpins devether and later virtio-blk/rng; switching to the crate later means re-homing all ring code behind the same internal API.
- **D-02:** **Generic transport, net first.** `virtio.rs` carries reusable machinery — PCI capability parsing, split-virtqueue descriptor/avail/used ring ops, barrier discipline, ISR-read-to-ack helper — and virtio-net is its first consumer. virtio-blk/virtio-rng (deferred Phase 6 items; Phase 11's `/dev/random` follow-up named virtio-rng-pci "the cleaner follow-up once virtio exists") plug in later at near-zero transport cost.
- **D-03:** **Modern-only negotiation.** VIRTIO_F_VERSION_1 required; MAC+STATUS are the only net features accepted (zero-offload per Pitfall 6 / roadmap criterion 2); legacy/transitional devices are rejected loudly at bind. Pairs with the `virtio-net-pci-non-transitional` qemu variant (v1.0 donor-trap lesson: transitional 0x1000 vs modern 0x1041).
- **D-04:** **Ring correctness proven twice.** Host-side `cargo test` unit tests for the virtqueue state machine (descriptor alloc, avail/used index advance, wrap, barrier placement — `port/src/allocator_tests.rs` is the in-tree precedent) PLUS xtask boot-test gates on qemu for integration and real memory-ordering behavior (TCG load gate, NETDRV-04).

### Host capture + net leg (gates)
- **D-05:** **Capture = qemu `-object filter-dump` on the default user/slirp netdev.** qemu itself writes a pcap of every frame on the netdev — hermetic, unprivileged, identical under linux CI and darwin TCG. Slirp's built-in ARP responder answers guest ARPs for its own addresses (gateway `10.0.2.2`), so RX is exercisable with zero host tooling. Tap/vmnet stay Phase 9 scope (HNET-03/04); Phase 8's tap-leg exit gate remains the L2-truth venue (Pitfall 9 acknowledged: slirp forgives L2 bugs — Phase 7's gates prove driver/frame-level correctness only).
- **D-06:** **Evidence = boot-test asserts the pcap; commit both.** The xtask boot-test parses the pcap (minimal pcap+ARP parser in xtask — the format is trivial) and asserts the guest's ARP request frame with the expected source MAC. Committed evidence in `.planning/phases/07-*/`: assertion output + raw pcap (a one-ARP pcap is ~100 bytes). Timing-tolerant presence assertions per the v1.0 smoke discipline.

### Traffic generation without devip
- **D-07:** **The guest ARP goes through the real file contract.** A 42-byte hand-crafted ARP request frame baked into bootfs (`/lib/test/arp.req`), written to `/net/ether0/0/data` by the in-guest rc sweep (`dd` — already a tier-1 binary). No kernel self-test hooks, no stub medium — the userspace→devether data-file path IS the deliverable Phase 8 consumes, so it is the thing under test. Sender protocol address = slirp's guest address (`10.0.2.15`), target = gateway (`10.0.2.2`), so slirp replies.
- **D-08:** **NETDRV-04 blast = ARP ping-pong + TX-bulk.** Ping-pong: N ARP requests to the gateway elicit exactly N replies — guest TX==N, RX==N, pcap shows 2N frames, IRQ accounting reconciles exactly. TX-bulk: a small in-guest blaster binary loops data-file writes as fast as the ring drains (wedge-hunting). Reconciliation asserted host-side from the pcap + serial-emitted counters; the exact ISR-count/frame-count invariant set is planner/researcher discretion.

### NIC wiring + boot integration
- **D-09:** **Minimal xtask wiring now.** `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` + `-object filter-dump,id=f0,netdev=n0,file=…` added directly to the aarch64 virt qemu path in `../r9/xtask/src/main.rs`. The TOML-data single-sourcing and `--net=slirp|tap|vmnet` modes are Phase 9 (HNET-02) — no speculative machinery now. — **Reversibility:** reversible — Phase 9 lifts the stanza into platform TOML; the flag shape is deliberately the same one HNET-02 will single-source.
- **D-10:** **NIC always-on for all aarch64 virt boots** (deliberate user override of the dedicated-net-profile recommendation): the standard boot speaks Ethernet by default; existing v1.0 profiles (virt/virtexec/virtmulti/virtnotes/virtpanic) keep their assertions and gain the IRQ source once the driver binds. Planner sequencing note: land the netdev stanza early as an *idle unbound* device — the DTB gains the pci node + interrupt-map, enabling a DTB-discovery serial-print gate before any driver exists; driver binding follows in dependency order so mid-phase instability doesn't red unrelated gates.

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- DTB-driven everything: ECAM scan + `interrupt-map`→GICv2 SPI routing, zero hardcoded IRQs/addresses; the moved-window DTB-perturbation boot-test proves it (NETDRV-01). This is r9's existing convention: "Discovery flows through port::fdt — no hardcoded virt address tables" (`virt/mod.rs`). The ECAM window (`0x4010000000` on qemu 11.x) is *read from the DTB pci node*, never a constant — qemu moved it once already.
- Zero-offload mask: MAC+STATUS only; no CSUM/TSO/GSO/MRG_RXBUF/CTRL_VQ/EVENT_IDX (Pitfall 6). EVENT_IDX reconsideration is a later optimization behind its own gate.
- Split virtqueues; barrier-first aarch64 ring code: descriptor write → store-store barrier → avail index (Pitfall 7).
- INTx level-triggered SPIs from the DTB interrupt-map + ISR read-to-ack; MSI/GICv2m a documented non-goal (Pitfall 8, NETDRV-04).
- PCI transport, not MMIO (the phase goal itself; the riscv64 leg's `virtio-net-device` is not a precedent).
- qemu shape for r9 gates: `-M virt,gic-version=2`; TCG on darwin; `-m ≤ 1G` (Phase 6 carry-forward).
- Gates stay r9-side: xtask boot-test profiles + serial markers (v1.0 D-12 / Phase 6 D-12). Harvey `flake.lock` r9 pin advances as its OWN commit with a fresh hosted CI run, never bundled; r9 commits land on `main` of `requaos/r9` under push-before-pin.
- Evidence consolidated in `.planning/phases/07-*/` with the r9 commit SHA recorded where r9 is involved; markers timing-tolerant (presence, never interleaving order).
- devether presents `/net/ether0` per `netif.h` with live RX/TX packet counters (NETDRV-03); binding `#l` at `/net` is ordinary namespace wiring.
- `aarch64` naming everywhere; r9 follows its `rustfmt.toml`; unsafe concentrated at the MMIO/DMA boundary with SAFETY comments per `UNSAFE-AUDIT.md` posture.
- "amd64 green is sacred" translates r9-side for this phase: raspi3b/raspi4b/x86_64/riscv64 targets stay green — the `port` crate is shared, so virt-only additions are cfg-gated.
- Bake-into-bootfs for gates (deterministic), ufs-over-tcp for iteration (Phase 2 bake-vs-ufs resolution).

### the agent's Discretion
- PCI/ECAM glue placement and file layout (research proposes `aarch64/src/platform/virt/pci.rs`; `platform/virt/mod.rs` is the DTB-discovery entry) and the internal `virtio.rs` / `devether.rs` / `netif.rs` module split in `port/src/`.
- Exact IRQ-accounting invariants asserted in the blast gate (ISR invocations vs frame counts, coalescing bounds) — exact-reconciling for ping-pong, bound-based for TX-bulk.
- devether ctl-command depth within the `netif.h` file set (clone/addr/ctl/data/ifstats/stats/type all present; which ctl writes are functional vs loud-unimplemented).
- The in-guest blaster binary's shape (name, loop counts, marker vocabulary) and bootfs payload mechanics (bake for gates; the `mkimage.sh` real-init rodata guard applies).
- DTB-perturbation mechanism for the moved-window test (qemu `highmem-ecam` vs fdtput patching) — the requirement is locked, the mechanism is the planner's.
- Whether existing v1.0 profiles gain the NIC in the same commit as driver binding or a follow-up (D-10 locks always-on as the end state).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 7 — goal, success criteria 1–4, requirement mapping
- `.planning/REQUIREMENTS.md` — NETDRV-01..04 full text; Out of Scope table; traceability
- `.planning/PROJECT.md` — Key Decisions (cross-repo r9 adoption), Constraints
- `.planning/STATE.md` — Accumulated Context (all v1.0 + Phase 6 decisions), Deferred Items

### v2.0 milestone research (the phase's design source)
- `.planning/research/SUMMARY.md` — stack recommendations; the virtio-drivers caveat (VERSION_1 — now stale per D-01's note); "r9 PCI/ECAM platform glue" component description
- `.planning/research/PITFALLS.md` — Pitfall 6 (virtio feature-negotiation landmines), Pitfall 7 (virtqueue memory-ordering on aarch64), Pitfall 8 (GICv2 INTx plumbing), Pitfall 9 (slirp masks L2 bugs), Pitfall 16 (cross-repo drift)
- `.planning/research/ARCHITECTURE.md` — "virtio-net Driver Shape in r9" table (proposed pci.rs / virtio.rs / devether.rs split, ECAM window, interrupt plumbing: INTID 30/33 precedent, MSI skip); the `/net` file-contract finding
- `.planning/research/FEATURES.md` §2 — Guest Networking table stakes (virtio-net is r9's first DMA device)

### Prior-phase decisions and debts
- `.planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-CONTEXT.md` — carried-forward conventions (r9-side gates, push-before-pin, evidence standard, qemu shape, bake-vs-ufs, in-guest rc sweep discipline)
- `.planning/milestones/v1.0-phases/05-shell-boot-one-command-integration/05-CONTEXT.md` — D-12 (r9-side gates stay r9-side), D-13 (CI = linux leg, full pipeline), sourced-not-typed sweep lesson
- `.planning/milestones/v1.0-phases/04-r9-userspace-exec-harvey-r9-abi/04-CONTEXT.md` — proof standard for r9 kernel changes

### Semantics references (Harvey tree — the contract being reimplemented)
- `sys/src/9k/port/netif.h` + `sys/src/9k/port/netif.c` — the devether file-tree contract (qid set, Netfile/Netif, stats machinery)
- `sys/src/9/pc/ethervirtio.c` — 686-line legacy C virtio-net driver: ring-shape semantics reference ONLY (legacy-transitional PCI; we are modern-only per D-03)
- `sys/src/9k/386/ethervirtio.c` — 9k's variant, same caveat
- `sys/src/9/port/netif.c` — the legacy portable ether framework

### r9 tree (where the work lands)
- `../r9/port/src/fdt.rs` — heap-less DeviceTree parser (the discovery substrate; verify `interrupt-map` traversal coverage)
- `../r9/port/src/dat.rs` — `Dev` trait (~line 232) + the `Result<T, &'static str>` dev-boundary convention
- `../r9/aarch64/src/platform/virt/mod.rs` — DTB-discovery convention + `print_machine_info` pattern
- `../r9/aarch64/src/platform/virt/gic/gicv2.rs` — SPI routing target (timer INTID 30, PL011 INTID 33 are the existing registry entries)
- `../r9/UNSAFE-AUDIT.md` — the unsafe posture D-01 answers to
- `../r9/xtask/src/main.rs` — qemu invocation (aarch64 virt currently has NO netdev; riscv64's `virtio-net-device` MMIO is not a precedent) + boot-test profile/marker machinery + negative panic scan
- `../r9/port/src/allocator_tests.rs` — host-side unit-test precedent for D-04
- `../r9/port/Cargo.toml`, `../r9/aarch64/Cargo.toml`, `../r9/rust-toolchain.toml` — dependency/lint/toolchain layout
- `../r9/HowItWorks.md` — r9 architecture overview

### External specs (researcher pulls exact sections)
- Virtio 1.x specification — split virtqueues, feature bits (MAC=5, STATUS=16, VERSION_1=32), PCI capability layout, ISR semantics
- Devicetree specification — `interrupt-map` for PCI INTx→SPI routing; qemu `virt` DTB pci node binding

### Harness + evidence (Harvey repo)
- `build/aarch64/` — bake harness (`mkimage.sh` real-init rodata guard) for the bootfs test payload; `build/aarch64/README.md` qemu drift notes
- `flake.nix` — the r9 input contract (`r9.packages.<system>.kernel-virt`); pin advance mechanics
- `.github/workflows/r9-shell.yml` — the CI shape net gates extend

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../r9/port/src/fdt.rs` — the heap-less DTB parser is the whole discovery substrate: `find_compatible`, `property_translated_reg_iter` already serve `print_machine_info`; ECAM window + interrupt-map traversal extend it
- `../r9/aarch64/src/platform/virt/gic/gicv2.rs` — GICv2 dispatch with two registered IRQs (timer 30, PL011 33); the virtio SPI joins the same table
- `../r9/port/src/dat.rs` Dev trait + the eight existing `dev*.rs` files — the registration pattern devether follows
- `../r9/xtask/src/main.rs` — boot-test profiles with serial-marker assertions and the always-on negative panic/Unhandled-interrupt scan; the net gates extend this machinery (plus the new pcap assertion per D-06)
- `../r9/port/src/allocator_tests.rs` — in-tree host-side `cargo test` precedent for the ring state machine tests (D-04)
- Harvey `build/aarch64/` bake harness + tier-1 in-guest rc sweep pattern (Phase 6 D-10, sourced-not-typed) — the dd-of-baked-frame gate rides it
- Harvey `sys/src/9/pc/ethervirtio.c` — 686-line working C virtio-net driver: virtqueue bring-up order, descriptor chaining, ISR handling semantics reference

### Established Patterns
- DTB-driven discovery, zero hardcoded addresses/IRQs — r9's defining platform convention (NETDRV-01 is this convention applied to PCI)
- Platforms-as-data (r9 TOML) — D-09 deliberately defers netdev single-sourcing to Phase 9 but keeps the flag shape HNET-02 will adopt
- `cfg(dev_*)` device gating via the config TOML `dev` list — devether joins it
- Execution-gated ordering (riscv64 vaporware rule): no scaffolding without a running gate — the idle-NIC-then-driver sequencing (D-10 note) keeps every commit bootable
- Evidence = serial markers + committed artifacts, timing-tolerant, presence-based
- r9-side gates stay r9-side; Harvey composes (flake pin advance as own commit)

### Integration Points
- `../r9/aarch64/src/platform/virt/` — new `pci.rs` + `mod.rs` wiring (DTB pci node → ECAM scan → SPI route)
- `../r9/port/src/` — new `virtio.rs` (transport/rings), `devether.rs` + `netif.rs` (`#l` device per netif.h)
- `../r9/xtask/src/main.rs` — aarch64 virt qemu args (netdev + device + filter-dump) and boot-test profile/pcap assertions
- Harvey `build/aarch64/` — bootfs payload (arp.req + blaster) and the smoke composition
- Harvey `flake.lock` — r9 pin advance (own commit, fresh hosted CI)

</code_context>

<specifics>
## Specific Ideas

- The acceptance story, end-to-end: the netdev stanza lands first (idle unbound NIC; the DTB gains the pci node + interrupt-map) → `pci.rs` discovery prints the DTB-derived ECAM window, the found virtio-net device, and its routed SPI over serial; a moved-window DTB-perturbation boot still binds (NETDRV-01) → `virtio.rs` transport grows host-side unit tests for ring state → the driver negotiates VERSION_1+MAC+STATUS and nothing else, ring code barrier-first → devether `#l` presents `/net/ether0/{clone,0/{addr,ctl,data,ifstats,stats,type}}` with live counters → the in-guest sweep `dd`s the baked `arp.req` to the data file; the host-side filter-dump pcap is asserted for that exact ARP frame and the guest's MAC (NETDRV-03) → ping-pong ×N plus a TX-bulk run: guest RX==N/TX==N, pcap 2N frames, IRQ accounting reconciles, no wedge, no panic, no Unhandled-interrupt line (NETDRV-04) → evidence + r9 commit SHA committed under `.planning/phases/07-*/`; `flake.lock` pin advances as its own commit.
- Watch-out: slirp's ARP responder only answers for its OWN addresses — the ARP request must target the gateway (`10.0.2.2`) or there is no reply and the ping-pong gate hangs.
- Watch-out: always-on NIC (D-10) means a mid-phase driver wedge reds every aarch64 profile — land gates in dependency order (idle NIC → discovery → transport → driver → devether) so each commit keeps the tree bootable.
- Watch-out: qemu moved the ECAM window once already (NETDRV-01's origin) — read the pci node's `reg` from the DTB every boot; `0x4010000000` appears in docs, never in code.
- Watch-out: Harvey's `ethervirtio.c` files are legacy-transitional PCI — a ring-semantics reference, NOT a negotiation reference (modern-only per D-03).
- Watch-out: TCG in CI — assertions presence-based with generous timeouts (v1.0 smoke discipline); the ping-pong count N should be modest on TCG and can scale up on KVM hosts.
- Watch-out: `virtio_net_hdr` size — ERRATA (07-RESEARCH.md, verified against qemu 11.1.2 + virtio-1.3 §5.1.6.1): the header is **12 bytes** whenever VIRTIO_F_VERSION_1 is negotiated, regardless of MRG_RXBUF (the 10-byte form is legacy-only; Harvey's ethervirtio.c VheaderSize=10 is the legacy shape D-03 forbids copying). Assert the negotiated mask in the boot log.
- ERRATA (07-RESEARCH.md): D-07's baked ARP frame is **60 bytes, not 42** — a netif-faithful devether write path enforces ETHERMINTU=60 (Etoosmall otherwise); pad 42+18 zero bytes, matching ethermedium's adjustblock precedent.
- User overrides to honor downstream (deliberate, don't second-guess): from-scratch virtio over the recommended crate (D-01 — control/audit grounds); always-on NIC over a dedicated net profile (D-10 — Ethernet is part of the standard boot).

</specifics>

<deferred>
## Deferred Ideas

- devip-compatible `/net` (TCP/UDP/ICMP, ipifc, arp + ethermedium bound to devether, loopback/null mediums, DHCP, IPv6) — Phase 8 (NETIP-01..08); its loopback-first staging runs parallel with this phase by design
- Host netdev modes: `--net=slirp|tap|vmnet`, netdev stanza single-sourced as TOML data, linux tap/bridge docs, darwin socket_vmnet, doctor-checker — Phase 9 (HNET-01..05)
- virtio-blk, virtio-9p host mount, SMP/PSCI, RTC — future milestones (carried from Phase 6 deferred list)
- virtio-rng-pci — "the cleaner follow-up once virtio exists" (Phase 11's `/dev/random`; v2.0 takes DTB-seeded CSPRNG)
- EVENT_IDX notification suppression — optimization behind its own gate, only after the plain path is proven (Pitfall 6)
- MSI / GICv2m — documented non-goal for v2.0 (Pitfall 8)
- smoltcp vs 9k/ip port — Phase 8's NETIP-01 decision gate; D-01's from-scratch choice does not pre-decide it (same-axis decision, recorded together per NETIP-01)

</deferred>

---

*Phase: 7-r9 PCI/ECAM + virtio-net + devether*
*Context gathered: 2026-11-04*
