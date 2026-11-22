# Phase 7: r9 PCI/ECAM + virtio-net + devether - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-04
**Phase:** 7-r9 PCI/ECAM + virtio-net + devether
**Areas discussed:** Virtio provenance, Host capture + net leg, Traffic w/o devip, NIC wiring home

---

## Virtio provenance

| Option | Description | Selected |
|--------|-------------|----------|
| Import virtio-drivers 0.13 | Pin the crate, implement its Hal trait over r9 pagealloc; mandatory audit that its net feature set constrains to MAC+STATUS; vendor/patch or fall back if not | |
| From-scratch virtio.rs | In-house caps parsing, split virtqueues, barrier-first ring code; full control, zero external unsafe, slower path | ✓ |
| Hybrid: crate transport, own net driver | Crate for PCI transport + queues, own net driver so the feature mask is ours | |

**User's choice:** From-scratch virtio.rs
**Notes:** Deliberate override of the milestone-research recommendation (virtio-drivers 0.13). Grounds: control + r9's maximal-unsafe-elimination ethos (UNSAFE-AUDIT.md, undocumented_unsafe_blocks=deny). Fact-check before the question: the research's VERSION_1 caveat is stale — upstream merged accept-if-offered (PR #196, 2025-09) and 0.13.0 (2026-03-03) includes it; PCI transport remains memory-mapped CAM/ECAM only, matching qemu virt. Recorded in CONTEXT.md D-01 so the record doesn't imply the crate was non-viable.

| Option | Description | Selected |
|--------|-------------|----------|
| Generic transport, net first | virtio.rs as reusable machinery (caps, rings, barriers, ISR-ack helper); virtio-net first consumer; blk/rng later near-free | ✓ |
| Net-only, minimal | Exactly what virtio-net needs, hardcoded; fastest to ether0 | |

**User's choice:** Generic transport, net first

| Option | Description | Selected |
|--------|-------------|----------|
| Host unit tests + boot gates | Ring state machine unit-tested host-side (allocator_tests.rs precedent); TCG load gate still proves real ordering | ✓ |
| Boot-test gates only | Correctness only via xtask boot profiles; every ring bug costs a boot cycle | |

**User's choice:** Host unit tests + boot gates

---

## Host capture + net leg

| Option | Description | Selected |
|--------|-------------|----------|
| filter-dump on slirp | qemu filter-dump object on the user netdev writes a pcap host-side; hermetic, unprivileged, CI-able; slirp ARP-responds so RX is exercisable | ✓ |
| Pull tap forward | Real L2 truth now, but privileged setup and Phase 9 (HNET-03) scope | |
| socket netdev + host peer | Hermetic real-L2-ish via a new host tool; Phase 9 replaces it anyway | |

**User's choice:** filter-dump on slirp
**Notes:** Pitfall 9 (slirp masks L2 bugs) acknowledged in the decision text — Phase 7 gates prove driver/frame correctness; Phase 8's tap-leg exit gate remains the L2-truth venue.

| Option | Description | Selected |
|--------|-------------|----------|
| Boot-test asserts pcap, commit both | xtask parses the pcap, asserts the guest ARP frame + src MAC; commit assertion output + raw pcap (~100 bytes) | ✓ |
| Assertion output only | Lighter repo; frame-level proof not inspectable later | |

**User's choice:** Boot-test asserts pcap, commit both

---

## Traffic w/o devip

| Option | Description | Selected |
|--------|-------------|----------|
| Raw frames via ether0 data file | 42-byte ARP baked into bootfs, `dd` in the in-guest rc sweep; exercises the real userspace→devether contract; zero test-only kernel code | ✓ |
| Kernel self-test hook | ctl-triggered kernel-crafted ARP; deterministic but test-only kernel code forever, bypasses the file path | |
| Stub ARP-only medium | Minimal devip skeleton — Phase 8 scope arriving early | |

**User's choice:** Raw frames via ether0 data file

| Option | Description | Selected |
|--------|-------------|----------|
| ARP ping-pong + TX bulk | N requests to the gateway ⇒ TX==N, RX==N, pcap 2N, IRQ accounting exact; plus TX-bulk blaster for wedge-hunting | ✓ |
| TX-bulk only | Simpler; 'no silent wedge under load' proven on one direction only | |

**User's choice:** ARP ping-pong + TX bulk

---

## NIC wiring home

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal xtask wiring now | -netdev user + virtio-net-pci-non-transitional + filter-dump on aarch64 virt; Phase 9 lifts to TOML-data | ✓ |
| TOML-data shape now | Build HNET-02's single-sourcing before its requirements exist | |

**User's choice:** Minimal xtask wiring now

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated net profile | NIC only on a new virtnet profile; v1.0 profiles stay byte-identical | |
| Always-on NIC for all virt boots | Standard boot speaks Ethernet by default; existing profiles gain the IRQ source | ✓ |

**User's choice:** Always-on NIC for all virt boots
**Notes:** Deliberate override of the recommendation. CONTEXT.md D-10 records the sequencing consequence: netdev stanza lands early as an idle unbound device (enabling the DTB-discovery gate pre-driver); driver binding follows in dependency order so mid-phase instability doesn't red unrelated gates.

---

## the agent's Discretion

- PCI/ECAM glue placement + `virtio.rs`/`devether.rs`/`netif.rs` module split (research's `aarch64/src/platform/virt/pci.rs` proposal is the default)
- Exact IRQ-accounting invariants in the blast gate (exact for ping-pong, bounds for TX-bulk)
- devether ctl-command depth within the netif.h file set
- Blaster binary shape, loop counts, marker vocabulary; bootfs payload mechanics
- Moved-window DTB-perturbation mechanism (qemu highmem-ecam vs fdtput)
- Same-commit vs follow-up commit for attaching the NIC to existing v1.0 profiles

## Deferred Ideas

- devip `/net` stack, arp/ethermedium binding, DHCP, IPv6 → Phase 8
- --net modes, TOML-data netdev stanza, tap/vmnet plumbing, doctor-checker → Phase 9
- virtio-blk / virtio-9p / SMP / RTC → future milestones
- virtio-rng-pci → Phase 11 /dev/random follow-up
- EVENT_IDX optimization → own gate after the plain path is proven
- MSI/GICv2m → documented v2.0 non-goal
