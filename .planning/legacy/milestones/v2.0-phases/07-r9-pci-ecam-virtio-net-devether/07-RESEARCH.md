# Phase 7: r9 PCI/ECAM + virtio-net + devether - Research

**Researched:** 2026-11-04
**Domain:** virtio 1.x modern PCI network driver in a Rust no_std kernel (r9), aarch64 qemu `-M virt,gic-version=2`, Plan 9 devether file contract
**Confidence:** HIGH (all load-bearing claims verified this session against in-repo source, the pinned qemu 11.1.2 binary/source, the nix-store libslirp 4.9.5 source, the r9 toolchain disassembly, or cited to the virtio 1.3 spec)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Virtio provenance + driver shape**
- **D-01:** From-scratch `port/src/virtio.rs` — NO external crate. Recorded rationale: full control, zero external unsafe, maximal-unsafe-elimination ethos. Satisfies NETDRV-02's "provenance decision recorded in-repo". Reversibility: costly.
- **D-02:** Generic transport, virtio-net first consumer. `virtio.rs` carries reusable machinery (PCI capability parsing, split-virtqueue ring ops, barrier discipline, ISR-read-to-ack helper); blk/rng plug in later.
- **D-03:** Modern-only — VIRTIO_F_VERSION_1 required, MAC+STATUS the only net features, legacy/transitional rejected loudly at bind. Pairs with `virtio-net-pci-non-transitional`.
- **D-04:** Host `cargo test` unit tests for the ring state machine (allocator_tests.rs precedent) + xtask boot gates on qemu.

**Host capture + net leg (gates)**
- **D-05:** Capture = qemu `-object filter-dump` on the default user/slirp netdev; slirp's ARP responder answers for its own addresses (gateway 10.0.2.2).
- **D-06:** xtask boot-test parses the pcap (minimal pcap+ARP parser in xtask) and asserts the guest's ARP request frame with the expected source MAC; assertion output + raw pcap committed to `.planning/phases/07-*/`. Timing-tolerant presence assertions.

**Traffic generation without devip**
- **D-07:** Guest ARP emitted as a raw frame written to `/net/ether0/0/data` via dd from a baked bootfs file (`/lib/test/arp.req`); sender protocol address = 10.0.2.15, target = 10.0.2.2. No kernel test hooks.
- **D-08:** ARP ping-pong (N requests → N replies; TX==N, RX==N, pcap 2N frames, IRQ reconciles) + TX-bulk blaster; exact ISR/frame invariant set is planner/researcher discretion.

**NIC wiring + boot integration**
- **D-09:** Minimal xtask wiring: `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` + `-object filter-dump,id=f0,netdev=n0,file=…` directly in the aarch64 virt qemu path. No speculative TOML machinery.
- **D-10:** NIC always-on for all aarch64 virt boots; netdev stanza lands early as an idle unbound device; existing profiles keep assertions and gain the IRQ source once the driver binds.

**Carried forward (locked in roadmap/prior phases):**
- DTB-driven everything: ECAM scan + interrupt-map→GICv2 SPI routing, zero hardcoded IRQs/addresses; moved-window DTB-perturbation boot-test proves it (NETDRV-01). ECAM window read from the DTB pci node, never a constant.
- Zero-offload mask: MAC+STATUS only; no CSUM/TSO/GSO/MRG_RXBUF/CTRL_VQ/EVENT_IDX.
- Split virtqueues; barrier-first aarch64 ring code: descriptor write → store-store barrier → avail index.
- INTx level-triggered SPIs from the DTB interrupt-map + ISR read-to-ack; MSI/GICv2m a documented non-goal.
- PCI transport, not MMIO.
- qemu shape: `-M virt,gic-version=2`; TCG on darwin; `-m ≤ 1G`.
- Gates stay r9-side: xtask boot-test profiles + serial markers; Harvey flake.lock r9 pin advances as its OWN commit with a fresh hosted CI run; r9 commits land on `main` of `requaos/r9` under push-before-pin.
- Evidence consolidated in `.planning/phases/07-*/` with r9 commit SHA recorded; markers timing-tolerant (presence, never interleaving order).
- devether presents `/net/ether0` per netif.h with live RX/TX counters; binding `#l` at `/net` is ordinary namespace wiring.
- `aarch64` naming everywhere; r9 follows its rustfmt.toml; unsafe concentrated at the MMIO/DMA boundary with SAFETY comments per UNSAFE-AUDIT.md posture.
- raspi3b/raspi4b/x86_64/riscv64 targets stay green — port crate is shared, virt-only additions cfg-gated.
- Bake-into-bootfs for gates (deterministic), ufs-over-tcp for iteration.

### the agent's Discretion
- PCI/ECAM glue placement and file layout (research proposes `aarch64/src/platform/virt/pci.rs`; `platform/virt/mod.rs` is the DTB-discovery entry) and the internal `virtio.rs` / `devether.rs` / `netif.rs` module split in `port/src/`.
- Exact IRQ-accounting invariants asserted in the blast gate (ISR invocations vs frame counts, coalescing bounds) — exact-reconciling for ping-pong, bound-based for TX-bulk.
- devether ctl-command depth within the netif.h file set (clone/addr/ctl/data/ifstats/stats/type all present; which ctl writes are functional vs loud-unimplemented).
- The in-guest blaster binary's shape (name, loop counts, marker vocabulary) and bootfs payload mechanics (bake for gates; the mkimage.sh real-init rodata guard applies).
- DTB-perturbation mechanism for the moved-window test (qemu highmem-ecam vs fdtput patching) — requirement locked, mechanism planner's.
- Whether existing v1.0 profiles gain the NIC in the same commit as driver binding or a follow-up (D-10 locks always-on as the end state).

### Deferred Ideas (OUT OF SCOPE)
- devip-compatible /net (TCP/UDP/ICMP, ipifc, arp+ethermedium bound to devether, loopback/null mediums, DHCP, IPv6) — Phase 8.
- Host netdev modes --net=slirp|tap|vmnet, netdev stanza single-sourced as TOML data, tap/bridge docs, socket_vmnet, doctor-checker — Phase 9.
- virtio-blk, virtio-9p host mount, SMP/PSCI, RTC — future milestones.
- virtio-rng-pci — Phase 11 follow-up.
- EVENT_IDX notification suppression — later optimization behind its own gate.
- MSI / GICv2m — documented non-goal for v2.0.
- smoltcp vs 9k/ip port — Phase 8's NETIP-01 gate.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NETDRV-01 | DTB-driven ECAM scan + interrupt-map→GICv2 SPI routing, no hardcoded IRQs/addresses | §"PCI/ECAM Discovery" + §"r9 fdt.rs Coverage" (exact helpers to add) + §"DTB-Perturbation Mechanism" (verified highmem-ecam addresses) |
| NETDRV-02 | virtio-net driver, split virtqueues, MAC+STATUS zero-offload mask, barrier-first ring code, provenance recorded | §"Virtio 1.x Modern PCI Mechanics" (spec-pinned layouts/sequence/barriers) + §"aarch64 Barriers from Rust" (fence codegen proof) + Pattern 1 (unsafe-free ring design) — provenance decision is D-01 itself (from-scratch), recorded here and committed r9-side |
| NETDRV-03 | devether `#l` presenting `/net/ether0` per netif.h, RX/TX counters, guest ARP observable on host | §"devether/netif Semantics" (qid set + stats format verbatim) + §"slirp + filter-dump Facts" (pcap parse target) + the 60-byte minmtu finding (Pitfall N2) |
| NETDRV-04 | Interrupt accounting + INTx level plumbing (ISR read-to-ack) proven under load | §"GICv2 SPI Plumbing" (ICFGR gap + registry slot) + §"ISR semantics" (spec + qemu confirmation) + §"ethervirtio.c Ring Reference" (nintr/nnote counter shape) |
</phase_requirements>

## Summary

Phase 7 lands r9's first DMA-capable device. The research confirms the milestone design is sound and, more importantly, **corrects two stale assumptions in CONTEXT.md's specifics before they reach a plan**:

1. **The modern virtio-net header is 12 bytes, not 10.** CONTEXT.md's specifics describe "10-byte virtio_net_hdr without MRG_RXBUF" — that is the *legacy* rule. qemu 11.1.2's `virtio_net_set_mrg_rx_bufs` selects `guest_hdr_len = sizeof(struct virtio_net_hdr_mrg_rxbuf)` (12 bytes) **whenever `VIRTIO_F_VERSION_1` is negotiated**, regardless of MRG_RXBUF [VERIFIED: qemu v11.1.2 `hw/net/virtio-net.c` — read this session]. The spec's legacy note is the contrapositive witness: "the legacy driver only presented num_buffers ... when VIRTIO_NET_F_MRG_RXBUF was negotiated; without that feature the structure was 2 bytes shorter" [CITED: virtio-v1.3 §5.1.6.1]. Since D-03 mandates VERSION_1, every TX header prepend is 12 bytes with `num_buffers=0` (spec: "The driver MUST set num_buffers to zero" [CITED: virtio-v1.3 §5.1.6.2.1]) and every RX used length includes 12 header bytes. Harvey's `ethervirtio.c` uses 10 only because it is a legacy-transitional driver — exactly the legacy-only content D-03 forbids copying.
2. **A 42-byte ARP frame cannot cross a netif-faithful devether write path.** `ETHERMINTU = 60 /* minimum transmit size */` [VERIFIED: sys/src/9/port/netif.h:128, same in sys/src/9k/k10/etherif.h:4], and devether's write path rejects `n < ether->minmtu` with `Etoosmall` [VERIFIED: sys/src/9/pc/devether.c:283-286 — `if(n > ether->mtu) error(Etoobig); if(n < ether->minmtu) error(Etoosmall);`]. The in-stack precedent pads: ethermedium does `if(BLEN(bp) < ifc->mintu) bp = adjustblock(bp, ifc->mintu);` [VERIFIED: sys/src/9k/ip/ethermedium.c:306-307]. The D-07 baked payload should be **60 bytes** (42-byte ARP + 18 zero pad); the pcap assertion then matches a 60-byte frame on the wire (slirp tolerates both, the contract is what matters).

Everything else the phase needs is verified in place: the committed DTB already carries the full PCI `interrupt-map` (16 entries, dev0–3 × INTA–D, all level-high SPIs to the GIC); the standard xtask boot puts the NIC at 00:01.0 with INTA → SPI 4 → **GIC INTID 36** [VERIFIED: qemu 11.1.2 `info pci` probe + committed DTB decode]; `core::sync::atomic::fence(Release)` compiles to `dmb ish` on the exact r9 toolchain (no new `asm!` needed for ring ordering) [VERIFIED: local disassembly]; qemu's filter-dump writes classic pcap with **unbuffered raw `writev()` syscalls**, so the D-16 SIGKILL teardown loses nothing [VERIFIED: qemu v11.1.2 `net/dump.c`]; and the `highmem-ecam=off` machine moves the ECAM window from `0x4010000000` to `0x3f000000` with an *identical* interrupt-map — the cleanest NETDRV-01 perturbation vehicle [VERIFIED: local qemu dumpdtb probe].

**Primary recommendation:** land in dependency order per CONTEXT.md's acceptance story — (1) xtask netdev stanza (idle NIC, DTB gains nothing — the pci node is already committed) → (2) `port::fdt` interrupt-map/phandle extension + `aarch64/src/platform/virt/pci.rs` discovery with serial prints → (3) the highmem-ecam perturbation profile (NETDRV-01) → (4) `port/src/virtio.rs` as an **unsafe-free index-model ring state machine** (allocator.rs precedent) with host `cargo test` gates, MMIO/volatile edges pushed to arch-side floor wrappers → (5) virtio-net driver (VERSION_1+MAC+STATUS, 12-byte header, barrier-first) → (6) devether `#l` via a new `registry_attach` arm → (7) pcap-asserted ARP gate + ping-pong/blast reconciliation.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| PCI/ECAM discovery (bus walk, cap parse, BAR assignment) | `aarch64/src/platform/virt/pci.rs` (arch — MMIO mapping, KZERO) | `port/src/fdt.rs` (DTB parsing extension) | MMIO mapping + publish-once cells are arch-side floor; DTB traversal helpers stay portable |
| virtio transport (caps, features, queue setup) | `port/src/virtio.rs` (unsafe-free logic) | arch floor (`io.rs` volatile widths, `vm.rs` barriers if needed) | D-01/D-02: reusable transport in the shared crate; unsafe concentrated at the MMIO/DMA boundary |
| Split-virtqueue ring state machine | `port/src/virtio.rs` (pure index/arena model, host-testable) | — | allocator.rs precedent: descriptor indices/avail/used positions as safe Rust; raw memory at the edges |
| virtio-net device driver | `port/src/virtio.rs` (net consumer of the transport) | `aarch64/src/platform/virt/pci.rs` (probing result handoff) | D-02: net is the first consumer |
| devether `#l` file tree | `port/src/devether.rs` + `port/src/netif.rs` | — | port hosts all devices (devcons/devpipe/...); namec dispatch via new registry arm |
| Interrupt registration (virtio SPI → handler) | `aarch64/src/trap.rs` dispatch table + `platform/virt/gic` enable | `port/src/devether.rs` (RX retire/wake hooks) | trap.rs owns the INTID match; port gets hooks (the devpipe/devproc dependency-inversion idiom) |
| GICv2 SPI programming (ISENABLER/IPRIORITYR/ITARGETSR/ICFGR) | `aarch64/src/platform/virt/gic/gicv2.rs` | — | existing enable() gains ICFGR (level) programming |
| qemu netdev/filter-dump wiring | r9 `xtask/src/main.rs` + `config.rs` | config TOMLs | D-09: minimal, direct flags; Phase 9 lifts to TOML data |
| Bootfs test payload (arp.req, blaster) | Harvey `build/aarch64/` staging + r9 `xtask` initrd | — | existing INITRD_FILES/smoke-out pipeline |
| Evidence (pcap, assertion logs, commit SHAs) | Harvey `.planning/phases/07-*/` | — | v1.0 evidence standard |



- Plan 9 conventions for Harvey-side C (tabs, lowercase, `nil`, `exits`/`sysfatal`); r9 follows its own `rustfmt.toml`.
- The 64-bit ARM architecture is named `aarch64` (never `arm64`) in tooling and docs.
- GSD workflow enforcement: file-changing work flows through GSD commands (planner/executor pick this up).
- The r9 repo is read-only for research; kernel work lands there during execution under r9's own gates.
- r9 unsafe posture (`UNSAFE-AUDIT.md`): `clippy::undocumented_unsafe_blocks = "deny"` on gated crates; every port module except `globalalloc`/`mcslock` carries `#[forbid(unsafe_code)]`; `asm!` exists only in named floor modules (`trap.rs`, `swtch.rs`, `vm.rs`, `pre_mmu/*`, `main.rs` boot entry). **A new unsafe site in a forbidden module fails the build mechanically** — the ring code must be designed around this (see "Ring memory model" below).

## Standard Stack

**This phase installs NO external packages.** D-01 forbids the virtio-drivers crate; D-06's pcap parser is hand-rolled in xtask (the "zero new crate dependencies" precedent from the socket-serial boot-test, r9 `xtask/src/main.rs:742-743`). All work is in-repo Rust (no_std) + qemu flags + Harvey bootfs payload files.

### Core (all pre-existing in the two repos)
| Component | Where | Role |
|-----------|-------|------|
| qemu-system-aarch64 11.1.2 | r9 nix devshell (pinned in r9 flake.lock) | `-M virt,gic-version=2`, ECAM host, slirp, filter-dump |
| dtc/fdtput 1.8.1 | r9 nix devshell | DTB round-trip + /chosen patching (existing xtask initrd path) |
| rustc 1.95.0-nightly (2026-01-30) | r9 `rust-toolchain.toml` | `aarch64-unknown-none` target; fence codegen verified below |
| `port` crate (no_std, forbid(unsafe_code)) | `../r9/port/` | virtio.rs/devether.rs/netif.rs home |
| `port::fdt` heapless DTB parser | `../r9/port/src/fdt.rs` | discovery substrate — extended for interrupt-map (see gap list) |
| `port::paqfs` + xtask initrd | `../r9/xtask/src/main.rs:1694-1754` | bootfs payload delivery |

### Supporting (referenced, not modified)
| Component | Role |
|-----------|------|
| Harvey `sys/src/9/pc/ethervirtio.c` | ring-semantics reference ONLY (legacy-transitional — see "keep/discard" list) |
| Harvey `sys/src/9k/port/netif.{h,c}` + `sys/src/9/pc/devether.c` | the devether file-tree contract being reimplemented |
| Harvey `build/aarch64/` bake harness | bootfs payload (arp.req + blaster) staging |
| 9front `ethervirtio10.c` (vendored donor tree) | modern-era negotiation shape reference (cross-check only) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| from-scratch `port/src/virtio.rs` | `virtio-drivers` crate | **Rejected by D-01** (user override: control/audit grounds). Note for the record: the VERSION_1 caveat in the milestone research is stale upstream (accept-if-offered merged 2025-09, crate ≥0.12.0 viable) — the override stands on unsafe-elimination grounds, not the caveat. |
| INTx + ISR read-to-ack | MSI via GICv2m | **Locked out** (Pitfall 8, Out of Scope): the DTB carries a `v2m@8020000` node and the pci node carries `msi-map`, but INTx is one line, DTB-declared, and sufficient at qemu virtio rates. |
| second committed DTB for perturbation | fdtput-patching the pci reg | fdtput alone makes a **lying DTB** (qemu never rewrites the pci node on the `-dtb` path — verified below). The machine+DTB must agree, so a second dumped pair is the mechanism. |

## Package Legitimacy Audit

**Not applicable — this phase introduces zero external package dependencies** (D-01's from-scratch mandate + xtask's std-only precedent). No npm/PyPI/crates entries are added; the r9 workspace `Cargo.toml` gains no `[dependencies]`. The existing workspace pins (bitflags 2.5, miniz_oxide 0.9, adler2 2, toml) are untouched.

## Architecture Patterns

### System Architecture Diagram

```
guest (qemu -M virt,gic-version=2, TCG)                    host
┌────────────────────────────────────────────────────────────┐
│ DTB (committed virt-gicv2.dtb, -dtb at PA 0x40000000)      │
│   pcie@10000000: reg → ECAM window; interrupt-map → SPIs   │
│         │                                                  │
│         ▼   main9 (before trap::unmask_irq)                │
│  virt/pci.rs ── ECAM walk: bus0 dev1 fn0, 1af4:1041, INTA  │
│         │      BAR4 assign (64-bit MMIO window 0x80_0000_0000+)
│         │      map common/notify/ISR/device caps           │
│         ▼                                                  │
│  port/virtio.rs ── VERSION_1+MAC+STATUS negotiation        │
│         │      queues 0=RX 1=TX: desc/avail/used pages     │
│         │      (Normal memory, KZERO+PA, dma-coherent)     │
│         ▼                                                  │
│  port/devether.rs (#l) ── /net/ether0/{clone,0/{addr,ctl,  │
│         │                  data,ifstats,stats,type}}       │
│         ▼  write(data) → TX descriptors → fence → notify ──┼──► virtio-net device ──► slirp
│  IRQ INTID 36 → trap.rs match → ISR read-to-ack →          │    (qemu 11.1.2)        (10.0.2.0/24:
│  retire used → RX frames → per-conv queue → wake reader    │        │                 gw .2, dns .3,
│         │ devether counters (in/out) ++                    │        ▼                 guest .15)
└─────────┼──────────────────────────────────────────────────┘  -object filter-dump
          │  serial markers (pcap assertion evidence)          file=… (classic pcap,
          └──────────────────────────────────────────────────►  raw writev, both dirs)
```

### Recommended Project Structure (r9)

```
../r9/
├── aarch64/src/platform/virt/
│   ├── pci.rs              # NEW: ECAM walk, cap parse, BAR assign, SPI route (DTB-driven)
│   └── mod.rs              # MODIFIED: print_machine_info gains pci discovery lines
├── aarch64/src/platform/virt/gic/gicv2.rs  # MODIFIED: ICFGR (level-trigger) programming
├── aarch64/src/trap.rs     # MODIFIED: virtio SPI dispatch arm + accounting
├── aarch64/src/io.rs       # MODIFIED: u16/u64 volatile accessors (u8 for ISR byte)
├── port/src/
│   ├── fdt.rs              # MODIFIED: phandle lookup + interrupt-map iterator
│   ├── virtio.rs           # NEW: transport + split-ring index model (unsafe-free)
│   ├── virtio_tests.rs     # NEW: host-side ring state machine tests (crate-root, like allocator_tests)
│   ├── devether.rs         # NEW: #l device + virtio-net binding
│   └── netif.rs            # NEW: Netif/Netfile model, qid scheme, stats
├── port/src/lib.rs         # MODIFIED: module adds (keep forbid(unsafe_code) on all three)
├── aarch64/lib/
│   ├── config_virt*.toml   # MODIFIED: dev = [..., "ether"] where bound
│   └── virt-gicv2-lowecam.{dts,dtb}  # NEW: highmem-ecam=off pair (perturbation gate)
├── Cargo.toml              # MODIFIED: check-cfg gains cfg(dev_ether)
└── xtask/src/main.rs       # MODIFIED: netdev+filter-dump args, Net boot-test profile, pcap assert
```

Harvey side:
```
build/aarch64/
├── root-tree/lib/test/arp.req   # NEW: 60-byte padded ARP request (see minmtu finding)
├── <blaster>.c → staged binary  # NEW: TX-bulk generator (tier-1 style)
└── smoke-out/                   # staging consumed by r9 xtask initrd
.planning/phases/07-*/           # evidence: assertion output + raw pcap + r9 SHA
```

### Pattern 1: The unsafe-free ring state machine (allocator.rs precedent)

**What:** port modules carry `#[forbid(unsafe_code)]` [VERIFIED: ../r9/port/src/lib.rs:10-57 — every module except `globalalloc` and `mcslock`]. Wave-3 proved the pattern: `allocator.rs` went unsafe-free by converting the free-list to an **index/arena model** (side-table headers as `Option<Idx>` chains, arena-offset blocks) with the raw-pointer shim split into `globalalloc.rs` [VERIFIED: ../r9/UNSAFE-AUDIT.md:44-55].

**Apply it to virtqueues:** the ring logic (descriptor allocation from a fixed-size index pool, avail/used index advance, wrap at 2^16, chain assembly, used-retire matching `lastused != used.idx`) is a pure state machine over indices — host-testable with zero MMIO. The memory edge (writing 16-byte descriptors, reading used elements, the ISR byte) is served by narrow arch-side wrappers:
- extend `aarch64/src/io.rs` with u8/u16/u64 volatile read/write twins of the existing u32 `read_reg`/`write_reg`/`write_or_reg` [VERIFIED: ../r9/aarch64/src/io.rs:21-50 — u32-only today, each carrying a SAFETY comment];
- descriptor/ring memory accessed as a mapped `VirtRange` through those wrappers (regblock.rs's typed-block idiom, but Normal memory);
- barriers as `core::sync::atomic::fence` — NOT unsafe, NOT asm! (see "aarch64 barriers from Rust").

**When to use:** every `port/src/virtio.rs` ring operation. The unit tests (D-04) then exercise the real state machine over a fake backing arena — exactly `allocator_tests.rs`'s crate-root test module shape [VERIFIED: ../r9/port/src/lib.rs:59-65, ../r9/port/src/allocator_tests.rs:1-10].

### Pattern 2: Device registration (devtab-less era)

r9 has NO devtab yet: `namec.rs` dispatches `#`-devices through `registry_attach(rune, spec)` — verbatim current arms: `'c' => devcons::attach()`, `'e'`, `'p'`, `'|'`, `'d'`, `'s'`, else `Err("namec: unknown # device")` [VERIFIED: ../r9/port/src/namec.rs:58-72]. Per-op dispatch happens in sibling match arms keyed by `chan.dev` constants `DEV_ROOT=0` … `DEV_MNT=7` [VERIFIED: ../r9/port/src/dat.rs:20-33].

devether joins by: (a) `DEV_ETHER` constant in dat.rs; (b) `'l' => crate::devether::attach(spec)` arm; (c) devether arms in the walk/open/stat/read/write/close dispatch matches; (d) a static FILES-style metadata table per devcons's precedent [VERIFIED: ../r9/port/src/devcons.rs:190-200 — `(b"cons", QID_CONS, 0o666), …`]; (e) `cfg(dev_ether)` gating via the TOML `dev` list (`dev = ["exec"]` → `--cfg dev_exec` emission [VERIFIED: ../r9/xtask/src/config.rs:217-223]) plus a new `cfg(dev_ether)` entry in the workspace `[workspace.lints.rust] unexpected_cfgs` check-cfg list [VERIFIED: ../r9/Cargo.toml — the comment there says each family member "must be declared here when code first uses it"].

### Pattern 3: Interrupt handler slot

`dispatch_irq` in trap.rs acks, matches INTID, EOIs once (never for spurious 1023) [VERIFIED: ../r9/aarch64/src/trap.rs:372-394]:
```rust
let intid = platform::gic::ack();
match intid {
    30 => platform::timer::tick(),
    33 => platform::pl011::rx_irq(),
    platform::gic::SPURIOUS_INTID => {}
    _ => println!("unhandled IRQ {intid}"),
}
if intid != platform::gic::SPURIOUS_INTID { platform::gic::eoi(intid); }
```
The virtio SPI joins this match. Two gate-relevant subtleties: (1) the always-on negative scan matches `panic|unhandled interrupt|note-timeout|unimplemented syscall` [VERIFIED: ../r9/xtask/src/main.rs:838-851] — an idle NIC's surprise IRQ would print `unhandled IRQ 36` which does NOT trip the scan; nonetheless the D-10 staging (idle NIC → discovery print → driver bind) means the handler is armed before the NIC can raise interrupts; (2) the handler runs IRQ-context lock-free per the GIC cell discipline (pl011's CONS_RX pattern [VERIFIED: ../r9/aarch64/src/platform/virt/pl011.rs:100,156-157]) — RX retire must not take a lock that syscall context holds; use the BusyCell/mcslock try-enter idiom or a lock-free ring into a per-conversation queue, waking readers via the devpipe-style block/wake hooks [VERIFIED: ../r9/port/src/devpipe.rs:98-113 — `set_hooks(PipeHooks { current, block, wake })`].

### Anti-Patterns to Avoid

- **Hardcoding `0x4010000000` or INTID 36:** the whole point of NETDRV-01; both must derive from the DTB at runtime (the values appear in this document as expected-check values for serial prints, never as code constants).
- **Mapping ring memory as Device memory:** `deviceutil::alloc_device_page` maps with `Entry::rw_device()` [VERIFIED: ../r9/aarch64/src/deviceutil.rs:38-61] — correct for MMIO registers, wrong for descriptor rings/frames: the virtio barrier model assumes Normal memory, and Device-memory reads of ring indices are pointlessly slow. Use `pagealloc::allocate_physpage()` + `vm::kernel_pagetable().map_phys_range(..., VaMapping::Offset(KZERO), Entry::rw_kernel_data(), Page4K, Kernel)` — the exact convention main9 uses for the initrd [VERIFIED: ../r9/aarch64/src/main.rs:255-268]. The DTB carries `dma-coherent` on the root and pci nodes [VERIFIED: ../r9/aarch64/lib/virt-gicv2.dts:34,320] — no cache maintenance on qemu virt (see "DMA/cache reality").
- **Copying ethervirtio.c's negotiation:** it is a legacy driver (I/O-port register block, PFN queue programming, 10-byte header, accepts CTRL_VQ|CTRL_RX). D-03 rejects all of it; only the ring *semantics* carry (see keep/discard list).
- **Accepting the device's full feature set:** negotiate exactly `MAC(5) | STATUS(16) | VERSION_1(32)`; assert the negotiated mask in the boot log (the guardrail against Pitfall 6's off-by-2/header-drift class).

## Virtio 1.x Modern PCI Mechanics (Research Q1)

All section numbers cite the virtio 1.3 specification (`docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html`, fetched this session). These are the normative texts the planner's tasks must implement letter-for-letter.

### Split virtqueue layout [CITED: virtio-v1.3 §2.7, table at §2.7 opening; driver requirement §2.7.1]

| Virtqueue Part | Alignment (bytes) | Size (bytes) |
|----------------|-------------------|--------------|
| Descriptor Table | 16 | 16 × Queue Size |
| Available Ring | 2 | 6 + 2 × Queue Size |
| Used Ring | 4 | 6 + 8 × Queue Size |

"The driver MUST ensure that the physical address of the first byte of each virtqueue part is a multiple of the specified alignment value" (§2.7.1). Queue Size is always a power of 2, max 32768. Descriptor: `{ le64 addr; le32 len; le16 flags; le16 next; }` (16 bytes); flags: NEXT=1, WRITE=2, INDIRECT=4. Avail ring: `{ le16 flags; le16 idx; le16 ring[qsz]; /* le16 used_event if EVENT_IDX */ }`. Used ring: `{ le16 flags; le16 idx; struct { le32 id; le32 len; } ring[qsz]; /* le16 avail_event if EVENT_IDX */ }`. We do NOT negotiate EVENT_IDX, so the event fields are absent/ignored.

### Barrier requirements — the normative contract [CITED: virtio-v1.3 §2.7.13 "Supplying Buffers to The Device"]

The driver algorithm's barrier steps, verbatim:
- Step 4: "The driver performs a suitable memory barrier to ensure the device sees the updated descriptor table and available ring before the next step." (between ring-entry writes and the idx update)
- §2.7.13.3.1: "The driver MUST perform a suitable memory barrier before the idx update, to ensure the device sees the most up-to-date copy."
- Step 6 / §2.7.13.4.1: "The driver MUST perform a suitable memory barrier before reading flags or avail_event, to avoid missing a notification." (between idx update and the suppression check → notify)

RX side (used ring): §2.7.14's re-check pattern — after re-enabling notifications, `mb()` then re-read `used.idx` before concluding the ring is drained. Since we run without EVENT_IDX and without suppression toggling, the required receive-side discipline is: **acquire barrier after observing `used.idx` advance, before trusting descriptor/buffer contents** (the load-side twin).

Device-side note: §6 VIRTIO_F_ORDER_PLATFORM(36) — we do NOT negotiate it; "the device and driver are assumed to be implemented in software … a weaker form of memory barriers is sufficient." Our `dmb ish` fences satisfy even the stronger hardware-device ordering, so we are conservative-correct on both counts.

### PCI capability walk [CITED: virtio-v1.3 §4.1.4]

`struct virtio_pci_cap` (little-endian, read-only): `u8 cap_vndr (=0x09, PCI_CAP_ID_VNDR); u8 cap_next; u8 cap_len; u8 cfg_type; u8 bar; u8 id; u8 padding[2]; le32 offset; le32 length;` — then cfg_type-specific extra data.

cfg_type values: COMMON_CFG=1, NOTIFY_CFG=2, ISR_CFG=3, DEVICE_CFG=4, PCI_CFG=5, SHARED_MEMORY=8, VENDOR=9. Driver MUST ignore reserved cfg_type/bar values and MUST accept larger cap_len (§4.1.4.1).

The capability list is walked from the PCI config header: status bit 4 (capabilities list), pointer at config offset 0x34. qemu presents four virtio caps (common, notify, ISR, device) + the PCI_CFG cap [VERIFIED: qemu v11.1.2 `hw/virtio/virtio-pci.c` realize path — `virtio_pci_modern_mem_region_map(proxy, &proxy->common, &cap); … &proxy->isr … &proxy->device … &proxy->notify` and a `VIRTIO_PCI_CAP_PCI_CFG` cap; notify cap carries `notify_off_multiplier = virtio_pci_queue_mem_mult(proxy)` = 4 by default].

### Common configuration structure [CITED: virtio-v1.3 §4.1.4.3]

Offsets from the COMMON_CFG cap's (bar, offset):

| Offset | Field | Access |
|--------|-------|--------|
| 0x00 | device_feature_select (le32) | RW |
| 0x04 | device_feature (le32) | RO |
| 0x08 | driver_feature_select (le32) | RW |
| 0x0C | driver_feature (le32) | RW |
| 0x10 | config_msix_vector (le16) | RW — leave 0xffff (no MSI-X) |
| 0x12 | num_queues (le16) | RO |
| 0x14 | device_status (u8) | RW (write 0 = reset) |
| 0x15 | config_generation (u8) | RO |
| 0x16 | queue_select (le16) | RW |
| 0x18 | queue_size (le16) | RW (0 = queue unavailable; power of 2) |
| 0x1A | queue_msix_vector (le16) | RW — leave 0xffff |
| 0x1C | queue_enable (le16) | RW (1 = enable; MUST NOT write 0) |
| 0x1E | queue_notify_off (le16) | RO |
| 0x20 | queue_desc (le64) | RW — physical address of Descriptor Area |
| 0x28 | queue_driver (le64) | RW — physical address of Driver (avail) Area |
| 0x2C… | (queue_avail in 1.0 terms = queue_driver) | |
| 0x30 | queue_device (le64) | RW — physical address of Device (used) Area |

(1.3 adds queue_notif_config_data/queue_reset/admin fields beyond 0x34 — unused by us.) Access discipline: "8-bit wide accesses for 8-bit wide fields, 16-bit … for 16-bit … 32-bit wide and aligned accesses for 32-bit and 64-bit fields … MAY access each of the high and low 32-bit parts independently" (§4.1.3.1) — matches io.rs's u32 floor plus a u8/u16 extension.

Queue setup sequence (§4.1.5.1.3): write queue_select → read queue_size (0 = absent) → optionally downsize → allocate+zero descriptor/avail/used → write queue_desc/queue_driver/queue_device → (no MSI-X) → **write queue_enable=1 LAST** — "The driver MUST configure the other virtqueue fields before enabling the virtqueue with queue_enable" (§4.1.4.3.2). qemu's queue_notify_off: "Simply map queues in order" — returns queue_select [VERIFIED: qemu v11.1.2 virtio-pci.c `VIRTIO_PCI_COMMON_Q_NOFF: val = vdev->queue_sel;`].

### Notify address [CITED: virtio-v1.3 §4.1.4.4]

`notify_addr = BAR(notify_cap.bar) base + notify_cap.offset + queue_notify_off × notify_off_multiplier`. On qemu: BAR4, offset 0x3000, multiplier 4 → queue N notifies at `bar4 + 0x3000 + N×4` [VERIFIED: qemu v11.1.2 virtio-pci.c — `proxy->notify.offset = 0x3000; proxy->notify.size = virtio_pci_queue_mem_mult(proxy) * VIRTIO_QUEUE_MAX;` with mult=4]. The notification value written is the 16-bit virtqueue index (no VIRTIO_F_NOTIFICATION_DATA — §4.1.5.2.1).

### ISR semantics [CITED: virtio-v1.3 §4.1.4.5]

The ISR capability covers an 8-bit ISR status field: bit 0 = Queue Interrupt, bit 1 = Device Configuration Interrupt. "To avoid an extra access, simply reading this register resets it to 0 and causes the device to de-assert an interrupt." Device-side MUST: "The device MUST reset ISR status to 0 on driver read" (§4.1.4.5.1). qemu confirms the mechanism: the ISR read does `qatomic_xchg(&vdev->isr, 0)` + `pci_irq_deassert(...)`; INTx assertion is `pci_set_irq(..., qatomic_read(&vdev->isr) & 1)` [VERIFIED: qemu v11.1.2 virtio-pci.c `virtio_ioport_read` VIRTIO_PCI_ISR arm + `virtio_pci_notify`]. Handler discipline (NETDRV-04): read the ISR byte **first** in the IRQ handler; bit 0 → retire used-ring entries; bit 1 → config-change path (link status — log it).

### Feature negotiation protocol [CITED: virtio-v1.3 §3.1.1 + §2.1 + §6]

Init sequence (§3.1.1, MUST-follow): (1) reset (write 0 to device_status; wait for read-back 0 per §4.1.4.3.2); (2) ACKNOWLEDGE(1); (3) DRIVER(2); (4) read device features (device_feature_select 0 → bits 0..31, 1 → bits 32..63), write the accepted subset via driver_feature_select/driver_feature; (5) FEATURES_OK(8); (6) re-read device_status, FEATURES_OK still set else device unusable; (7) device-specific setup (queues); (8) DRIVER_OK(4) — "At this point the device is 'live'"; never notify before DRIVER_OK.

Reserved bits (§6): VERSION_1=32 — "A driver MUST accept VIRTIO_F_VERSION_1 if it is offered. A driver MAY fail to operate further if VIRTIO_F_VERSION_1 is not offered." We do exactly the latter: reject loudly when absent (D-3's transitional rejection). Feature read on qemu returns bits in 32-bit windows; VERSION_1 lives in window 1 bit 0.

**Zero-offload accept mask (D-03):** `MAC(5) | STATUS(16) | VERSION_1(32)`. Everything else offered by qemu (CSUM/TSO/GSO/MRG_RXBUF/CTRL_VQ/MQ/EVENT_IDX…) is explicitly NOT accepted.

### virtio-net device specifics [CITED: virtio-v1.3 §5.1]

- Queues (§5.1.2): 0=receiveq1, 1=transmitq1; controlq at 2 only exists if CTRL_VQ negotiated (we don't — ignore queue 2 even though qemu registers it).
- Feature bits (§5.1.3): MAC=5 ("Device has given MAC address"), STATUS=16 ("Configuration status field is available"), MRG_RXBUF=15 (rejected), CSUM=0 (rejected), etc.
- Device config layout (§5.1.4): `u8 mac[6]` at offset 0 (always present; valid when MAC negotiated); `le16 status` at offset 6 (present when STATUS negotiated; bits: LINK_UP=1, ANNOUNCE=2). All read-only for the driver. On qemu the config is feature-gated: `min_size = endof(mac)`, status appended when F_STATUS [VERIFIED: qemu v11.1.2 virtio-net.c `feature_sizes` table]. MAC is read-only for a modern driver (qemu ignores MAC writes when VERSION_1: `if (!virtio_vdev_has_feature(vdev, VIRTIO_NET_F_CTRL_MAC_ADDR) && !virtio_vdev_has_feature(vdev, VIRTIO_F_VERSION_1) && memcmp(...))` gates the set [VERIFIED: qemu v11.1.2 virtio-net.c `virtio_net_set_config`]). Default slirp NIC MAC is qemu's `52:54:00:12:34:56` unless `-device ...,mac=` overrides.
- Packet header (§5.1.6 + the legacy note §5.1.6.1): **12 bytes under VERSION_1** — `{ u8 flags; u8 gso_type; le16 hdr_len; le16 gso_size; le16 csum_start; le16 csum_offset; le16 num_buffers; }` with flags=0, gso_type=GSO_NONE(0), num_buffers=0 on TX (§5.1.6.2.1: "The driver MUST set num_buffers to zero"; "If VIRTIO_NET_F_CSUM is not negotiated, the driver MUST set flags to zero and SHOULD supply a fully checksummed packet"). RX frame length = used-elem `len` − 12.
- Queue sizes on qemu: RX/TX both default 256, and "with virtio-1, guest can downsize" [VERIFIED: qemu v11.1.2 virtio-net.c `VIRTIO_NET_RX_QUEUE_DEFAULT_SIZE 256`]. Planner discretion: accept 256 (2 KiB×256 descriptor memory is trivially satisfiable from pagealloc) or downsize via queue_size write.

## PCI/ECAM Discovery (Research Q2)

### ECAM address computation

`pci-host-ecam-generic` mapping: config-space byte address = `ecam_base + (bus << 20) | (dev << 15) | (fn << 12) | reg` (bus 8 bits, dev 5, fn 3, reg 12). The window base+size come from the DTB pci node's `reg`, translated to the root bus. On the committed DTB [VERIFIED: ../r9/aarch64/lib/virt-gicv2.dts:318]: `reg = <0x40 0x10000000 0x00 0x10000000>` → base **0x4010000000**, size **0x10000000** (256 MiB, 256 buses; matches `bus-range = <0x00 0xff>` at line 321).

### Discovery walk

1. Find the pci node: `dt.find_device_type("pci")` (device_type = "pci" at virt-gicv2.dts:325) or `find_compatible("pci-host-ecam-generic")` (line 326) — both helpers exist in fdt.rs today.
2. Read ECAM window via the existing `property_translated_reg_iter` [VERIFIED: ../r9/port/src/fdt.rs:301-342].
3. Map the ECAM range as device memory (`deviceutil::map_device_register` pattern) — note: 256 MiB of ECAM mapped as 4K pages = 65536 page-table entries; **map on demand per bus** (we only enumerate bus 0 in practice — bus-range 0..0xff but only dev 0..31 exist on bus 0 for qemu virt; a full-window map is wasteful. Recommend mapping bus 0's 1 MiB, or mapping per-device 4 KiB pages as found — planner's choice within the "no full-RAM map" r9 convention [Phase 4 finding: runtime physical access maps explicitly via map_phys_range]).
4. Enumerate: for dev in 0..32, fn 0: read vendor/device at ECAM(bus0, dev, fn, 0x00); 0xFFFFFFFF = absent. virtio vendor **0x1AF4**, modern net device **0x1041** (=0x1040+1; transitional net 0x1000 — D-03 rejects it loudly if seen). Verify revision ≥ 1 (modern) at config offset 0x08 byte.
5. On the standard xtask boot the NIC lands at **00:01.0** — [VERIFIED: qemu 11.1.2 `info pci` probe this session: "Bus 0, device 1, function 0: Ethernet controller: PCI device 1af4:1041 … IRQ 0, pin A; BAR1: 32 bit memory (not mapped); BAR4: 64 bit prefetchable memory (not mapped); BAR6: 32 bit memory (not mapped)"].

### BAR handling on qemu virt

qemu assigns **no** BAR addresses — there is no firmware ("not mapped" in the probe). r9 must do minimal PCI resource assignment:
1. Size BAR4: write 0xFFFFFFFF_FFFFFFFF, read back mask, restore… then assign from the pci node's `ranges` 64-bit MMIO window. From the committed DTB [VERIFIED: virt-gicv2.dts:317]: the `ranges` property's 64-bit region is `<0x3000000 0x80 0x00 0x80 0x00 0x80 0x00>` → child 0x03_000000 (64-bit MMIO space flag) maps to parent **0x80_00000000**, size 0x80_00000000 (512 GiB). Assign BAR4 = 0x80_00000000 (first free).
2. Write PCI_COMMAND (offset 0x04): set bit 1 (Memory Space) **and bit 2 (Bus Master)** — on the modern path qemu does NOT auto-enable bus-master (the auto-enable hack exists only in the legacy I/O-port status write: `if (val == (ACKNOWLEDGE|DRIVER)) pci_default_write_config(... PCI_COMMAND_MASTER …)` inside `virtio_ioport_write` [VERIFIED: qemu v11.1.2 virtio-pci.c] — the modern `virtio_pci_common_write` STATUS arm has no such side effect). Without bus-master, DMA silently does nothing.
3. Map the assigned BAR4 phys range (16 KiB: common 0x0-0x1000, ISR 0x1000-0x2000, device 0x2000-0x3000, notify 0x3000-0x3000+size) as device memory into the kernel table.
4. Walk the capability list (offset 0x34) for vendor 0x09 caps; collect common/notify/ISR/device.

### DTB interrupt-map decoding

From the committed DTB [VERIFIED: ../r9/aarch64/lib/virt-gicv2.dts:313-327]:
- `interrupt-map-mask = <0x1800 0x00 0x00 0x07>` — mask over (child-addr phys.hi, phys.mid, phys.lo, pin). 0x1800 keeps device-number bits 0–1 of phys.hi; 0x07 keeps the pin.
- `#interrupt-cells = <0x01>` on the pci node; the GIC node has `#interrupt-cells = <0x03>` and `#address-cells = <0x02>` [VERIFIED: lines 316, 356-358].
- Entry tuple layout (10 cells): child unit address (3 cells: phys.hi = bus<<16|dev<<11|fn<<8), child interrupt specifier (1 cell: pin 1-4), interrupt-parent phandle (1 cell = 0x8002 → intc@8000000), parent unit address (2 cells, zero), parent interrupt specifier (3 cells: type, number, flags).

The 16 entries decode (verified by hand against the committed bytes):

| dev | INTA→ | INTB→ | INTC→ | INTD→ |
|-----|-------|-------|-------|-------|
| 0 | SPI 3 (INTID 35) | SPI 4 (36) | SPI 5 (37) | SPI 6 (38) |
| 1 | SPI 4 (36) | SPI 5 (37) | SPI 6 (38) | SPI 3 (35) |
| 2 | SPI 5 (37) | SPI 6 (38) | SPI 3 (35) | SPI 4 (36) |
| 3 | SPI 6 (38) | SPI 3 (35) | SPI 4 (36) | SPI 5 (37) |

All with flags **0x04 = level-high** (GIC flag encoding: 1=edge-rising, 4=level-high). INTID = 32 + SPI. **The standard boot's NIC (dev 1, INTA) routes to SPI 4 = GIC INTID 36.** The swizzle is the classic (dev+pin) rotation. This decode table belongs in a host-side unit test for the parser (feed the committed DTB bytes as a fixture — the blob is in-repo).

## r9 fdt.rs Coverage — What Exists, What Must Be Added (Research Q3)

Read in full this session: ../r9/port/src/fdt.rs (768 lines, unsafe-free, `#[forbid(unsafe_code)]`).

**Existing helpers (sufficient for reg/ranges/compatible work):**
- `DeviceTree::new(&[u8])` / `header_totalsize` two-pass construction (lines 62-81)
- `find_compatible(comp) -> impl Iterator<Item = Node>` (lines 389-399)
- `find_device_type(device_type)` (lines 402-409)
- `find_by_path(path)` (lines 352-386)
- `property(node, name)`, `property_value_as_u32/u64`, `property_value_as_u32_iter`, `property_value_bytes` (lines 146-182)
- `property_reg_iter`, `property_range_iter`, `property_translated_reg_iter` with full parent-range translation (lines 206-342)
- `parent()`, `children()`, `nodes()`, `node_name()`

**Missing — must be ADDED for the pci interrupt-map:**
1. **Phandle resolution**: no helper maps a phandle value (e.g. 0x8002) back to the Node carrying `phandle = <0x8002>`. Add `find_phandle(phandle: u32) -> Option<Node>` — a `nodes()` scan comparing the `phandle` property's u32. (Needed because the interrupt-map's parent field is a phandle, and the parent's `#interrupt-cells`/`#address-cells` govern tuple stride.)
2. **`#interrupt-cells` / `#address-cells` readers on arbitrary nodes**: `node_address_size_cells` is private and covers address/size only (lines 184-195); expose an `#interrupt-cells` reader (pub or pub(crate) per the module's style).
3. **An `interrupt-map` iterator**: parse the packed tuple array into `struct PciIrqMap { child_hi: u32, pin: u8, parent_phandle: u32, parent_irq: [u32; 3] }` (stride derived from the two nodes' cell counts + `interrupt-map-mask` applied to the lookup key). Heapless, iterator-shaped, matching the file's existing style (`core::iter::from_fn` over a cursor — same shape as `property_range_iter` at lines 244-298).
4. **A lookup helper**: given (bus, dev, fn, pin) → masked key → matched parent SPI + flags. The mask semantics: `(child_hi & mask_hi) == entry.child_hi && (pin & mask_pin) == entry.pin`.

Nothing else needs adding: the ECAM window read reuses `property_translated_reg_iter` unchanged.

**How the DTB reaches the kernel** [VERIFIED: ../r9/aarch64/src/main.rs:140-160 + ../r9/aarch64/lib/config_virt.toml:25-27 + HowItWorks.md "Boot contract"]: virt boots the ELF `-kernel` with x0=0 (qemu 11.1.2 loads no DTB for ELF kernels); both consumers substitute the fixed PA **0x40000000** where qemu loads the committed `-dtb` blob (`aarch64/lib/virt-gicv2.dtb` per `[qemu].dtb`). When the config carries an initrd, xtask's `prepare_initrd_dtb` fdtput-patches a `target/` copy with `/chosen/linux,initrd-start/end` (two BE u32 cells per u64, hi first) and hands qemu the patched copy [VERIFIED: ../r9/xtask/src/config.rs:444-483]. The same patched-copy channel can carry an `rng-seed`/bootargs — Phase 11's affair, not ours.

## GICv2 SPI Plumbing in r9 (Research Q4)

**Existing machinery** (read in full): ../r9/aarch64/src/platform/virt/gic/gicv2.rs (125 lines) + gic/mod.rs facade.

- `Gicv2::new(dt)` finds `arm,cortex-a15-gic` (fallback `arm,gic-400`), reads the two translated regs (GICD then GICC), maps both via `map_device_register`, then the verified probe sequence: GICD_CTLR=1, GICC_PMR=0xff, GICC_CTLR=1 [VERIFIED: gicv2.rs:39-85].
- `enable(intid, prio)`: ISENABLER RMW (offset 0x100 + 4×(intid/32)), IPRIORITYR byte write (0x400 + 4×(intid/4), shift (intid%4)×8), and **ITARGETSR only for SPIs (intid ≥ 32)** targeting cpu0 [VERIFIED: gicv2.rs:90-113]. The module header documents the research-P6 finding: "PPIs are hardwired per-CPU, SPIs are undelivered without a target write."
- `ack()` = GICC_IAR read; `eoi(intid)` = GICC_EOIR write, never for spurious 1023 [VERIFIED: gicv2.rs:115-124, mod.rs:20].
- Registered handlers today: INTID 30 (timer PPI 14) and INTID 33 (PL011 SPI 1) — armed in main9's virt block: `gic::init(&dt); gic::enable(30, 0xA0); timer::init(&dt); pl011::init_rx(); gic::enable(33, 0xA0);` then unmask [VERIFIED: ../r9/aarch64/src/main.rs:357-367].

**What must be ADDED:**
1. **ICFGR (trigger) programming** — gicv2.rs has no ICFGR code (offset 0xC00, 2 bits per interrupt, bit (2×(intid%16)+1): 0=level, 1=edge). The DTB says flags=0x04 (level-high) for every PCI INTx entry. qemu's GICv2 model defaults SPIs to level at reset (irq_state zeroed; the ICFGR write path sets edge via `GIC_DIST_SET_EDGE_TRIGGER` only when bit-pattern `2 << (i*2)` is written [VERIFIED: qemu v11.1.2 hw/intc/arm_gic.c:1435-1452]) — which is why PL011 works today without it. Program it anyway: real GIC-400 silicon's ICFGR reset value is IMPLEMENTATION DEFINED, and Pitfall 8 names wrong-trigger as a wedge vector. Add `set_level_triggered(intid)` (or fold into `enable`), writing before ISENABLER.
2. **The virtio SPI registry slot**: the trap.rs match arm (Pattern 3 above) + `gic::enable(spi, 0xA0)` in the driver-bind path. Per the ordering rule (HowItWorks.md: "`trap::unmask_irq()` is called LAST, after the GIC, timer and UART RX paths are armed"), the NIC's GIC enable must happen before unmask — for dev_exec configs that means inside the exec-path boot orchestration before the first EL0 entry (the unmask rides exec_bootstrap's tail), for plain virt before `trap::unmask_irq()` in main9. The driver bind (pci probe → virtio init → devether attach → gic enable) belongs in that window; it runs in boot context with IRQs still masked, so the ring setup races nothing.
3. **EOI ordering**: the existing dispatch does ack → handler → eoi. For a level-triggered line the ISR-read inside the handler is what deasserts the source; EOI then completes the GIC handshake. This is exactly the existing flow — the handler just must read the virtio ISR byte before returning (NETDRV-04's ISR read-to-ack).
4. Optional: a `disable(intid)` (ICENABLER at 0x180) for driver teardown — not needed this phase (no unload path).

## aarch64 Barriers from Rust (Research Q5)

**Empirically answered with the phase's exact toolchain** (r9 nix devshell, rustc 1.95.0-nightly 2026-01-29, `--target aarch64-unknown-none -O`; disassembly this session):

```rust
fence(Ordering::Release)  →  dmb ish     // full inner-shareable store+load barrier
fence(Ordering::Acquire)  →  dmb ishld   // load-side barrier
fence(Ordering::SeqCst)   →  dmb ish
```

**Verdict: `core::sync::atomic::fence` suffices — no new `asm!` is needed for ring ordering.** Release lowers to the *stronger* `dmb ish` (not the minimal `ishst`), which satisfies §2.7.13's "suitable memory barrier" for descriptor-table/avail-ring store→idx-store ordering, and orders the subsequent notify MMIO write too (DMB orders all memory accesses in the domain regardless of Normal/Device type; the PCI root complex is an inner-shareable observer). Acquire (`dmb ishld`) is precisely the used-ring read-side twin.

**Compiler-side discipline (the part fences don't cover):** fences order *atomic* operations in Rust's model; the descriptor/ring accesses themselves must be volatile (or atomic) so the compiler never reorders/elides them around the fence — i.e., the io.rs volatile-wrapper edge from Pattern 1, with `fence(Release)` between descriptor writes and the avail-idx volatile store, and `fence(Acquire)` after reading used-idx before reading buffer contents. This matches UNSAFE-AUDIT's asm-consolidation posture: zero new asm! sites; the barrier is a core intrinsic, and volatile stays in the named io.rs floor. If a reviewer prefers an explicit `dmb ishst`, it would land in `vm.rs` (the named barrier floor [VERIFIED: ../r9/UNSAFE-AUDIT.md:59-62]) as a safe wrapper — but the fence evidence says don't bother.

**MMIO write ordering for notify:** the notify register write is a volatile store to Device memory. Program order + the Release fence before it give the required ordering (descriptor/ring Normal stores → fence → Device store). No `dsb` is required (dsb would only matter if we needed to *wait* for completion — we don't; the IRQ is the completion signal).

## DMA/Cache Reality on qemu virt (Research Q6)

- **No IOMMU, virt==phys:** no virtio-iommu/SMMU node on this machine; VIRTIO_F_ACCESS_PLATFORM is not offered by qemu here, so "the driver MUST pass only physical addresses to the device" (§6.1) — queue_desc/driver/device take raw PAs from `pagealloc::allocate_physpage()` [VERIFIED: ../r9/aarch64/src/pagealloc.rs:63-78 — "Try to allocate a physical page. Note that this is NOT mapped."].
- **Coherence:** the DTB carries `dma-coherent` on the root node and the pci node [VERIFIED: ../r9/aarch64/lib/virt-gicv2.dts:34,320] — qemu's DMA reads/writes guest RAM coherently (TCG and HVF alike on this machine). **No cache maintenance on descriptor rings or frame buffers.** (This is the virt-specific answer; the raspi targets would need maintenance, but this phase's driver is cfg-gated to virt.)
- **Buffer allocation:** rings and RX/TX buffers come from `pagealloc::allocate_physpage()` (4 KiB pages) mapped at KZERO+PA with `Entry::rw_kernel_data()` (Normal, cacheable) — NOT `deviceutil::alloc_device_page` (which maps `rw_device()` — wrong memory type for rings; see Anti-Patterns). Ring alignment (desc 16 / avail 2 / used 4) is trivially satisfiable inside a zeroed 4 KiB page per queue part; a single page per part per queue is the simplest layout (desc 16×256=4096 exactly fills one page for qsize 256; avail 6+2×256=518; used 6+8×256=2054).
- **RX buffer size:** 12-byte header + 1514 max frame → one 4 KiB page per RX buffer is grossly oversized but free of fragmentation questions; a slab of header+frame slots inside pages is an optimization the planner may defer. Pre-posting strategy: fill all avail slots at bind (ethervirtio.c's replenish-loop semantics, minus its kproc).

## devether/netif Semantics (Research Q7)

Sources read in full this session: sys/src/9k/port/netif.h (121 lines), sys/src/9k/port/netif.c (732), sys/src/9/pc/devether.c (571). The 9k and 9/port netif.c are identical (732 lines each).

### The file/qid set `#l` must present

Qid encoding [VERIFIED: sys/src/9k/port/netif.h:5-28]:
```c
enum { Ncloneqid= 1, Naddrqid, N2ndqid, N3rdqid, Ndataqid, Nctlqid,
       Nstatqid, Ntypeqid, Nifstatqid, Nmtuqid, };
#define NETTYPE(x) (((ulong)x)&0x1f)
#define NETID(x)   ((((ulong)x))>>5)
#define NETQID(i,t) ((((ulong)i)<<5)|(t))
```

Three-level tree (netifgen [VERIFIED: netif.c:36-150]):
- `#l` root (qid path 0): contains one dir per interface named `ether0` (nif->name), qid N2ndqid.
- `#l/ether0/` (level 2): `clone` (0666), `addr` (0666), `stats` (0444), `ifstats` (0444), plus one numbered dir per open conversation (`0`, `1`, …) with qid NETQID(i, N3rdqid).
- `#l/ether0/N/` (level 3): `data`, `ctl` (owner perms, default 0666 eve), `stats`, `type`, `ifstats` (0444).

NETDRV-03 names the presentation `/net/ether0/{clone,0/{addr,ctl,data,ifstats,stats,type}}` — with `#l` bound at /net via the conventional `bind -a #l /net` [VERIFIED: lib/namespace:40].

### Clone/open semantics [VERIFIED: netif.c:158-197 + 491-543]

- Opening `clone` allocates a fresh conversation id (first free Netfile slot; `openfile(nif, -1)`), sets its `inuse`, and **rewrites the chan's qid to NETQID(id, Nctlqid)** — the returned file IS the new conversation's ctl file. Reading ctl returns the id in decimal (`readnum(offset, a, n, NETID(...), NUMSIZE)` at netif.c:216).
- Opening `N/data` or `N/ctl` re-opens that id (`openfile(nif, id)`, qreopen + inuse++); dirs are OREAD-only (else Eperm); addr/stats/type/ifstats OREAD-only else Ebadarg (netif.c:169-191).
- Ownership: first opener becomes `owner` with mode 0660 (`netown`, netif.c:456-485); later opens by other users check perms. For r9's single-user era this can collapse to "first open owns" with the same surface behavior; keep the file perms identical for stat output.

### Data file read/write semantics

- **Read** (`netif.c:212-214`): `qread(f->iq, a, n)` — a **blocking** queue read of whole inbound Ethernet frames (Qmsg mode — message boundaries preserved, one frame per read). r9 answer: the read blocks via the devpipe-style block/wake hook until the RX path delivers a frame; each read returns exactly one frame (raw 14-byte-header Ethernet, no virtio header — the driver strips it).
- **Write** (`devether.c:253-299`): only valid on the data qid (else it falls to ctl handling). Frame size gate: `n > mtu (1514) → Etoobig; n < minmtu (60) → Etoosmall` — **this is the D-07 landmine**: the ARP payload must be baked at 60 bytes. Then: allocb, copy, and **the kernel overwrites the source MAC with the NIC's own address** (`memmove(bp->rp+Eaddrlen, ether->ea, Eaddrlen)` — devether.c:294) before queueing on `oq` and kicking `transmit`. Note the D-06 pcap assertion's "expected source MAC" is therefore the NIC's negotiated MAC (52:54:00:12:34:56 default), whatever the baked frame carried — bake it that way for clarity, but the driver rewrites it regardless.
- MTU bounds: minmtu=ETHERMINTU=60, mtu=ETHERMAXTU=1514 [VERIFIED: devether.c:387-389 — `ether->minmtu = ETHERMINTU; ether->maxmtu = ETHERMAXTU; ether->mtu = ETHERMAXTU;`].
- Loopback/broadcast/promiscuous TX feedback into the input path (`etheroq` → `etheriq(ether, bp, 0)`, devether.c:215-251) — needed for ethermedium in Phase 8; **this phase may ship TX without loopback feedback** (nothing in the ARP gate needs it) but the planner should note the omission loudly since Phase 8's arp expects it.

### RX demux (`etheriq`, devether.c:133-213)

`inpackets++` on every inbound block; ether-type demux to conversations: deliver to every Netfile with `f->type == type || f->type < 0` (connect -1 = all), filtered by `tome || multi || f->prom`; multicast filtered against `activemulti` unless promiscuous; queue-full or alloc failure → `soverflows++`. The counters NETDRV-03 wants map to: `inpackets` (RX), `outpackets` (TX, incremented in etheroq devether.c:221), plus `soverflows`/`misses` for wedge forensics.

### Stats file formats (must match byte-for-byte where feasible)

Interface-level `stats` (netif.c:217-238) — exact format:
```
in: %llud\nlink: %d\nout: %llud\ncrc errs: %d\noverflows: %d\nsoft overflows: %d\nframing errs: %d\nbuffer errs: %d\noutput errs: %d\nprom: %d\nmbps: %d\naddr: %2.2ux…\n
```
`addr` file: the MAC as lowercase hex bytes, no separators (`%2.2ux` × alen, netif.c:239-248). `type` file: conversation's type number (decimal). `ifstats` at the netif level returns empty; devether routes it to the driver's ifstat hook (devether.c:80-89) — ethervirtio.c's ifstat prints devfeat/drvfeat/devstatus/netstatus + per-queue `avail->idx used->idx lastused nintr nnote` (ethervirtio.c:439-468) — **that per-queue line is exactly the IRQ-accounting surface NETDRV-04 reconciles against**; keep its shape.

### ctl commands [VERIFIED: netif.c:293-362 + devether.c:262-281]

netifwrite recognizes: `connect N`, `promiscuous`, `scanbs N`, `bridge`, `headersonly`, `addmulti <addr>`, `remmulti <addr>`; unknown → error. devether additionally handles `nonblocking [0|1]` (qnoblock on oq) and forwards the rest to the driver's ctl hook, else `Ebadctl`.

**Recommended minimum for this phase** (within the planner's ctl-depth discretion): functional `connect N` (sets f->type; the in-guest sweep's `echo 'connect 0x0806' > /net/ether0/0/ctl` is how an ARP-only conversation pins ether-type 0x0806 — though for D-07's raw-frame gate the default type-0 catch is unnecessary: writing the data file works without any connect; connect matters for the RX demux when the ARP *reply* arrives — so either connect 2054 (0x0806) or connect -1 must precede the dd), functional `promiscuous` (can be a no-op accepted silently or loud-unimplemented — qemu's RX filter without CTRL_VQ delivers to our MAC + broadcast regardless), `addmulti/remmulti` loud-unimplemented (no CTRL_VQ), `scanbs/bridge/headersonly` loud-unimplemented (wifi-era). Unknown verbs error loudly per the repo's loud-unimplemented posture.

### What `attach(spec)` does

`etherattach`: parses the controller number from spec (default 0), errors Enodev if absent, devattach('l'), chan->dev = ctlrno, calls the driver's attach hook — which in ethervirtio.c sets DRIVER_OK and starts kprocs (see Q8 — r9 has no kprocs; see Q11 for the restructure).

## ethervirtio.c Ring Reference — Keep/Discard (Research Q8)

Read in full: sys/src/9/pc/ethervirtio.c (686 lines; the 9k/386 variant is byte-identical in the ring parts).

**Keep (semantics to re-express in Rust):**
- Two-descriptor chains: every buffer = (header desc, payload desc); RX both flagged WRITE (device-writable), TX neither (device-read) [ethervirtio.c:204-216 TX, 278-290 RX].
- Static avail pre-linking: `availent[i] = availent[i + qsize/2] = 2i` — each avail slot permanently names its descriptor head; posting = `avail->idx++` after filling payload addr/len [lines 211, 285].
- RX replenish-until-caught-up loop then one notify: `do { … allocb; fill desc; coherence(); avail->idx++; } while(avail->idx != used->idx); vqnotify(Vrxq)` [lines 299-312].
- Used retirement by `lastused != used->idx`, slot recovery via `(usedent[i & qmask].id & qmask) >> 1`, RX frame length `used.len - hdrsize` [lines 226-234, 319-330].
- The barrier point is explicit in C: `coherence()` immediately before `avail->idx++` [lines 251-252, 309-310] — our `fence(Release)` sites.
- Interrupt handler: ISR-byte read first, queue-bit check, wake [lines 393-412]; `nintr`/`nnote` counters (the IRQ accounting idea NETDRV-04 formalizes).
- Notification suppression the simple way: `avail->flags &= ~Rnointerrupt` (driver wants interrupts) and check `used->flags & Unonotify` before kicking [lines 179, 218, 292] — no EVENT_IDX.

**Discard (legacy-only, D-03 forbids):**
- The entire I/O-port register interface (Qdevfeat…Qmac via inb/outb/ins/outs/inl/outl on an I/O BAR) [lines 31-41, 588-607, 656-661].
- Transitional probing: accepts did 0x1000 OR 0x1041, requires revision 0, requires BAR0 I/O-space bit, subsystem-id match at 0x2E [lines 556-571]. We match ONLY 0x1041 with revision ≥ 1.
- PFN-based queue programming: one physical base ÷4096 written to Qaddr with the legacy contiguous VPGROUND layout [lines 498-545, 607]. Modern: three separate 64-bit queue_desc/queue_driver/queue_device addresses, independent alignment.
- Feature mask `Fmac|Fstatus|Fctrlvq|Fctrlrx` [line 593] — we accept MAC|STATUS only (plus VERSION_1), so no control queue exists; promiscuous/multicast via vctlcmd [lines 478-496] goes away (devether ctl gets loud-unimplemented instead).
- `VheaderSize = 10` [line 67] — **wrong for modern: 12** (see Summary finding #1). The Rust port's header struct carries num_buffers and zeroes it on TX.
- No FEATURES_OK step [legacy §3.1.2 omits it] — modern init must include it (§3.1.1 steps 5-6) with the re-read check.
- kproc-based rxproc/txproc [lines 186-257, 259-332] — r9 has no kprocs; restructure per Q11.
- `mallocalign` legacy ring allocation — replace with per-part pagealloc pages.

## slirp + filter-dump Facts (Research Q9)

**slirp (libslirp 4.9.5 — the exact nixpkgs-packaged version, source read from the local nix store this session):**

- Defaults (qemu 11.1.2 `net/slirp.c:446-450`): network 10.0.2.0/24, gateway/host 10.0.2.2, DNS 10.0.2.3, DHCP start (guest) 10.0.2.15.
- ARP responder (`src/slirp.c:1207-1271`, `arp_input`): answers ARP requests **only** for `vnameserver_addr` (10.0.2.3) and `vhost_addr` (10.0.2.2), plus guestfwd entries; all other targets get silence. **The D-07 ARP must target 10.0.2.2** (the CONTEXT.md watch-out confirmed in source).
- Reply MAC: `special_ethaddr` is `52:55:00:00:00:00` with bytes [2..6] replaced by the answered IP — gateway replies come from **52:55:0a:00:02:02** [slirp.c:83-84 comment "emulated hosts use the MAC addr 52:55:IP:IP:IP:IP", 1253-1255].
- Gratuitous ARP (tip==sip) is absorbed into the ARP table, no reply [slirp.c:1229-1232].
- slirp ARPs the GUEST only when it has an IP packet to deliver with an unknown guest MAC (`if_encap4`, slirp.c:1331-1358 — broadcast request, source 52:55:0a:00:02:02, 1-second retry expiry). **Idle slirp with no hostfwd connections emits nothing** — no free RX chatter; the guest must speak first (which D-07 does). Any hostfwd connection attempt generates a broadcast ARP to the guest — a Phase-8+ RX source, not needed here.
- The guest's ARP request gets a reply (RX frame) → the ping-pong gate's RX==N comes from slirp's arp_ok path, TCG-timing-tolerant.

**qemu `-object filter-dump,id=f0,netdev=n0,file=…`** [VERIFIED: qemu v11.1.2 `net/dump.c`, read this session]:
- Output is **classic libpcap** (not pcap-ng): global header magic 0xa1b2c3d4, version 2.4, snaplen = maxlen property (default 65536), **linktype 1 (Ethernet)**; per packet: 16-byte record header `{ le32 ts_sec; le32 ts_usec; le32 caplen; le32 len }` + frame bytes.
- Writes are **unbuffered `writev()` syscalls per packet**; the global header is written at object setup (before guest start). There is no stdio buffer to lose — the D-16 SIGKILL process-group teardown loses nothing already captured [net/dump.c:62-97 dump_receive_iov, 108-143 net_dump_state_init].
- Attached to the netdev it captures **both directions** (every frame to/from the peer); the vnet-hdr offset (`qemu_get_vnet_hdr_len(nf->netdev)`) is 0 for the user backend — the pcap contains pure Ethernet frames [net/dump.c:155-164].
- Timestamps: virtual clock + boot wall-clock base — presence assertions only, never timing (D-06 discipline).

**Minimal xtask pcap+ARP parse (D-06 target):** skip 24-byte global header; iterate 16-byte record headers; for each frame: dst[6] src[6] ethertype[2] == 0x0806; ARP body: htype=1, ptype=0x0800, hlen=6, plen=4, oper=1 (request), sha=sender MAC, **spa=10.0.2.15**, tha=00:00:00:00:00:00, **tpa=10.0.2.2**. Assert: ≥1 frame with sha == the guest's negotiated MAC and those addresses; for the ping-pong gate, exactly N requests from guest MAC + N replies from 52:55:0a:00:02:02 (oper=2). ~100 lines of std-only Rust; no new crate (per the xtask zero-dependency precedent).

## DTB-Perturbation Mechanism for NETDRV-01 (Research Q10)

**Verified empirically this session** (qemu 11.1.2, `-cpu cortex-a72 -m 1024M`, dumpdtb both ways):

| Machine flag | pci node reg | ECAM window | Size | bus-range |
|---|---|---|---|---|
| (default, highmem-ecam=on) | `<0x40 0x10000000 0x00 0x10000000>` | **0x4010000000** | 256 MiB | 0x00-0xff |
| `highmem-ecam=off` | `<0x00 0x3f000000 0x00 0x1000000>` | **0x3f000000** | 16 MiB | 0x00-0x0f |

The `interrupt-map`, `ranges`, and everything else in the pci node are **byte-identical** between the two — the perturbation isolates exactly the ECAM window (and bus-range shrink), which is what NETDRV-01 wants proven.

**Recommended mechanism: a second committed DTB pair + a second boot-test profile.**
1. Generate: `qemu-system-aarch64 -M virt,gic-version=2,highmem-ecam=off,dumpdtb=…` + dtc round-trip → commit `aarch64/lib/virt-gicv2-lowecam.{dts,dtb}` with a header comment naming the provenance (mirroring the existing regeneration recipe at virt-gicv2.dts:13-18).
2. New config `config_virtnetle.toml` (name planner's): `machine = "virt,gic-version=2,highmem-ecam=off"`, `dtb = "aarch64/lib/virt-gicv2-lowecam.dtb"` — `[qemu].machine` is a free-form string passed to `-M` [VERIFIED: ../r9/xtask/src/config.rs emission tests:541-556], so no xtask schema change is needed.
3. The gate: the net boot profile binds the NIC under BOTH DTBs with identical serial evidence. A driver hardcoding 0x4010000000 fails the low-ecam boot loudly: that address is outside RAM and outside the 16 MiB ECAM hole → config reads abort or return garbage ≠ 0x1AF4 → "no virtio-net found" print + bind failure.

**Why not fdtput-patching the committed blob:** qemu's `-dtb` path patches ONLY `/memory` (re-derived from -m) and `/chosen` (bootargs/initrd/rng-seed/kaslr) — the pci node is taken as-is [VERIFIED: qemu v11.1.2 `hw/arm/boot.c` arm_load_dtb: nop-/memory + fdt_add_memory_node; /chosen setprops; no pci fixup; and `hw/arm/virt.c:3030` create_fdt builds its own fdt only for the no-dtb path]. So an fdtput-moved `reg` creates a lying DTB (DTB says moved, machine didn't move) — that tests "the driver read the DTB" negatively (must fail to bind) rather than the positive proof (binds under a moved window) NETDRV-01's success criterion asks for. The low-ecam pair keeps DTB == machine-truth.

## r9 Integration Surface (Research Q11)

**Device registration:** devtab-less era — see Pattern 2. devether = new `DEV_ETHER` const + `registry_attach` arm `'l'` + dispatch arms in namec's dev_walk/dev_open/dev_stat/dev_read/dev_write/close tables. `#l` bound at `/net` by ordinary namespace wiring (lib/namespace:40 `bind -a #l /net` — Harvey-side, consumed when the boot chain's namespace runs).

**Driver bind point in boot sequence:** the virt IRQ-arming block in main9 (main.rs:357-401) is the insertion window — after `devcons::init` (console live for prints) and pagealloc/vm are up, and strictly before `trap::unmask_irq()` / `exec_bootstrap`'s unmask. The bind chain: `pci::probe(&dt)` → map ECAM/BAR4 → virtio init (reset→negotiate→queues) → `devether::attach` publishing the #l statics → `gic::enable(spi, 0xA0)` → serial print (`#l0: virtio: 1000Mbps: 52:54:00:12:34:56` echoing devether.c:434-448's print shape). Discovery prints land earlier, in `print_machine_info` (mod.rs:17-35 pattern) or a sibling called from it — enabling the D-10 staging gate (pci node + ECAM window + routed SPI over serial with an idle NIC).

**Interrupt integration:** trap.rs match arm → `devether`-bound handler. The handler shape (lock-free IRQ context per pl011 precedent): read ISR byte (ack at source) → if bit 0: retire TX used entries (free/counters) and pull RX used entries onto the conversation queues → wake blocked readers via the devpipe-style registered hooks → counters (nintr per queue). The proc-facing block/wake rides the existing hook-injection idiom (`port::devpipe::set_hooks` [VERIFIED: devpipe.rs:98-113]; `port::devcons::set_cons_hooks` [VERIFIED: main.rs:498-504]) — devether gets `set_hooks(EtherHooks { block, wake, current })` registered from exec_bootstrap/main9 alongside the others. **Everything is IRQ-driven; no kproc/tick polling fallback is needed** — the RX path is ISR-triggered retire + wake; the TX path is write-synchronous post + IRQ retire. (A tick-poll fallback would mask ISR bugs — Pitfall 8 wants the IRQ path proven, so do NOT add one.)

**RX delivery model without kprocs:** ethervirtio.c's rxproc sleeps on `vhasroom`; r9's equivalent: the IRQ handler does the retire (it's short: parse header, strip 12 bytes, demux into per-conversation queue, wake) — the same shape as pl011's `rx_irq` FIFO drain into CONS_RX. Replenish (posting fresh RX buffers) also runs in the handler after retire, or in the data-file read path when it drains — planner's choice; replenish-in-handler keeps latency flat and matches the C.

**cfg gating:** new `dev = ["ether"]`-style entries on the virt configs that bind; workspace Cargo.toml check-cfg gains `cfg(dev_ether)`; raspi/x86_64/riscv64 configs never gain the flag (port code compiles everywhere, virt-only glue is `#[cfg(platform = "virt")]`).

**Harvey-side coupling:** r9's xtask initrd assembles target/bootfs.paq from `R9_HARVEY_DIR`'s `build/aarch64/smoke-out/` staged binaries (`INITRD_FILES` list at main.rs:1705) — the blaster binary joins that list and `/lib/test/arp.req` joins the staged tree (mkpaqfs bakes files, not just binaries — the tree assembly at main.rs:1738-1754 creates dirs like `proc/`; a `lib/test/` dir + file is the same mechanism). The in-guest sweep that `dd`s the frame rides the virtshell boot chain (dev_bootchain → real Harvey boot(8) → termrc sweep — sourced-not-typed per Phase 5 D-10); the net profile's sweep gains the `bind -a #l /net` + `echo connect … > ctl` + `dd if=/lib/test/arp.req of=/net/ether0/0/data` lines.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| virtio transport | a crate dependency (forbidden by D-01) | in-repo `port/src/virtio.rs` — but DO study `virtio-drivers`'s ring invariants as a cross-check | user-locked; the spec sections above are the authority |
| DTB parsing | a new DTB crate (virtio-drivers pulls one) | extend `port::fdt` (phandle + interrupt-map iterators) | the heapless parser already does translated-reg; ~150 lines of additions |
| pcap parsing | pcap/pcap-parser crates | ~100 lines std-only Rust in xtask | format is 24-byte header + 16-byte records; zero-dependency precedent in xtask |
| IRQ accounting | perf counters / tracing infra | the devether ifstats line (nintr/nnote per queue, ethervirtio.c shape) + serial markers | the file contract IS the observability surface |
| Ring buffer allocator | a DMA-allocator abstraction | `pagealloc::allocate_physpage` + KZERO+PA maps | one page per ring part, one page per frame buffer; no allocator design this phase |
| ARP wire format | a packet-builder crate | a baked binary file (60 bytes) + dd | D-07's real-file-contract gate |

**Key insight:** the domain's complexity concentrates in *ordering and negotiation*, not algorithms — the spec's normative texts (copied into the plan's tasks) plus the from-scratch index-model ring are cheaper than auditing any crate's unsafe surface against UNSAFE-AUDIT.

## Common Pitfalls

### Pitfall N1: The 12-byte header (corrects CONTEXT.md)
**What goes wrong:** implementing a 10-byte virtio_net_hdr (per Harvey's legacy driver and the CONTEXT specifics) under VERSION_1. Every RX frame's payload starts 2 bytes early — frames parse as garbage that looks like "intermittent corruption"; TX headers confuse qemu's parser.
**Why:** qemu picks the header length from `(version_1 || mrg_rxbuf || hash/tunnel)` — VERSION_1 alone forces 12 bytes [VERIFIED: qemu v11.1.2 virtio-net.c virtio_net_set_mrg_rx_bufs].
**Avoid:** `VnetHdr` is 12 bytes, `num_buffers=0` on TX, RX `len-12`; a const-assert on the size; the boot log prints the negotiated feature mask + header size.
**Warning signs:** ARP replies that never demux; pcap shows correct TX frames but RX lengths are off by 2.

### Pitfall N2: minmtu rejects the 42-byte ARP (corrects D-07's framing)
**What goes wrong:** the sweep's `dd` of a 42-byte file fails `Etoosmall` at the devether write — gate red with a confusing errno.
**Why:** ETHERMINTU=60 (netif.h:128); the faithful write path bounds-checks.
**Avoid:** bake `/lib/test/arp.req` at **60 bytes** (42-byte ARP + 18 zero pad — exactly what ethermedium's `adjustblock(bp, mintu)` produces on the real stack). The pcap assertion expects a 60-byte frame; slirp replies either way.
**Warning signs:** `dd` exits nonzero with "too small"; zero frames in the pcap.

### Pitfall N3: BAR unassigned / bus-master off — silent DMA dead zone
**What goes wrong:** config-space caps read fine (ECAM works) but DMA never happens; rings sit empty with no error and no IRQs.
**Why:** qemu virt ships no firmware — BAR4 is "not mapped" until the guest assigns it, and the modern virtio status path never auto-enables bus-master (the legacy I/O path does).
**Avoid:** assign BAR4 from the DTB `ranges` 64-bit window (0x80_00000000), set PCI_COMMAND MEMORY|MASTER explicitly, and sanity-print the BAR readback + a negotiated-feature readback at bind.
**Warning signs:** ISR byte always 0; queue_notify writes do nothing; used.idx never advances.

### Pitfall N4 (Pitfall 8 in PITFALLS.md): INTx trigger/ack plumbing
Level-triggered SPIs: program GICD_ICFGR level (don't rely on qemu's level-at-reset default — impl-defined on real GIC-400); read the virtio ISR byte FIRST in the handler (read-to-ack deasserts the line); EOI exactly once. MSI is a mirage on gic-version=2 (v2m node exists but is a documented non-goal). Flat `info irq`-equivalent counters after the first packet = the ISR read was skipped.

### Pitfall N5 (Pitfall 7): memory-ordering under load
Barrier-first ring code from day one: descriptor writes → `fence(Release)` → avail idx store → suppression check → notify write; used side: read idx → `fence(Acquire)` → trust contents. TCG single-vCPU is gentle; the TX-bulk blaster (D-08) is the load gate. Moving corruption when logging is added = the tell.

### Pitfall N6 (Pitfall 6): feature-negotiation landmines
Accept MAC|STATUS|VERSION_1 and NOTHING else; print the offered and negotiated masks at bind; never "temporarily" accept CSUM to make something work. MRG_RXBUF would additionally flip qemu's RX buffer accounting (`virtio_net_has_buffers` mergeable path) — another reason it stays off.

### Pitfall N7 (Pitfall 9): slirp forgives — gate scope discipline
This phase's gates prove driver/frame-level correctness only; the L2-truth leg is Phase 8/9 (tap). Never hardcode 10.0.2.x outside the baked test fixture; the ARP fixture's addresses are the one sanctioned exception (they ARE the protocol payload).

### Pitfall N8 (Pitfall 16): cross-repo drift
r9 commits → r9 gates → push → Harvey flake.lock pin-bump commit → hosted CI → Harvey feature commit. The netdev stanza shape is shared verbatim between r9 xtask and (Phase 9) Harvey's flake — keep the flag string identical. Evidence commits record the r9 SHA (v1.0 D-13 ordering).

### Pitfall N9: mid-phase always-on NIC reds unrelated profiles (CONTEXT watch-out)
Land in dependency order: netdev stanza (idle, unbound — safe) → discovery prints → transport unit tests → driver bind → devether. Every commit keeps the tree bootable; the idle NIC prints nothing to the negative scan's patterns (an unexpected IRQ would print `unhandled IRQ 36`, which the scan doesn't match — but a bound-then-wedged driver would red everything, so driver binding lands after its gates exist).

### Pitfall N10: "unhandled IRQ" ≠ "Unhandled interrupt"
The negative scan keys on `panic|unhandled interrupt|note-timeout|unimplemented syscall` (lowercased compare). A stray NIC interrupt before binding prints `unhandled IRQ 36` — visible in the log but gate-green. Treat any such line as a bind-ordering bug to fix, not a cosmetic (and consider adding it to the scan in a follow-up — planner's call, flag it in the plan).

## Code Examples

### Capability walk + BAR4 map (sketch; offsets verified above)
```rust
// Source: layout per virtio-v1.3 §4.1.4; qemu offsets verified from v11.1.2 virtio-pci.c
// ECAM config read: ecam_base (from DTB pci reg) + (bus<<20)|(dev<<15)|(fn<<12)|reg
let mut cap_ptr = read_cfg_u8(ecam, 0x34) as usize;           // capability list pointer
while cap_ptr != 0 {
    let id = read_cfg_u8(ecam, cap_ptr);
    if id == 0x09 {                                            // PCI_CAP_ID_VNDR
        let cfg_type = read_cfg_u8(ecam, cap_ptr + 3);
        let bar      = read_cfg_u8(ecam, cap_ptr + 4);
        let offset   = read_cfg_u32(ecam, cap_ptr + 8);
        let length   = read_cfg_u32(ecam, cap_ptr + 12);
        match cfg_type {
            1 => common = Some((bar, offset, length)),
            2 => { notify_mult = read_cfg_u32(ecam, cap_ptr + 16); notify = Some((bar, offset)); }
            3 => isr    = Some((bar, offset)),
            4 => devcfg = Some((bar, offset)),
            _ => {}
        }
    }
    cap_ptr = read_cfg_u8(ecam, cap_ptr + 1) as usize;         // cap_next
}
// qemu values: all in BAR4; common@0x0/0x1000, isr@0x1000/0x1000, devcfg@0x2000/0x1000,
// notify@0x3000, notify_mult=4.
```

### Negotiation + queue bring-up (the §3.1.1 sequence)
```rust
// Source: virtio-v1.3 §3.1.1, §4.1.4.3, §4.1.5.1.3; fence discipline §2.7.13
common.write8(0x14, 0);                       // reset (device_status)
while common.read8(0x14) != 0 {}              // MUST wait for read-back 0
common.write8(0x14, 1);                       // ACKNOWLEDGE
common.write8(0x14, 1 | 2);                   // + DRIVER
common.write32(0x00, 0); let lo = common.read32(0x04);   // features 0..31
common.write32(0x00, 1); let hi = common.read32(0x04);   // features 32..63
if hi & 1 == 0 { /* no VERSION_1 → reject loudly, set FAILED(128) */ }
let accept_lo: u32 = (1 << 5) | (1 << 16);    // MAC | STATUS
let accept_hi: u32 = 1;                       // VERSION_1
common.write32(0x08, 0); common.write32(0x0C, accept_lo & lo);
common.write32(0x08, 1); common.write32(0x0C, accept_hi & hi);
common.write8(0x14, 1 | 2 | 8);               // + FEATURES_OK
if common.read8(0x14) & 8 == 0 { /* device rejected the subset → FAILED */ }
// per queue q in {0=rx, 1=tx}:
//   write16(0x16, q); read16(0x18) → size; write queue_desc/driver/device PAs;
//   write16(0x1C, 1) LAST
common.write8(0x14, 1 | 2 | 8 | 4);           // + DRIVER_OK — live
```

### The barrier sandwich (avail-side post)
```rust
// Source: virtio-v1.3 §2.7.13 steps 4-6; fence lowering verified on the r9 toolchain
ring.write_descriptor(head, buf_pa, len, flags);      // volatile writes (io.rs floor)
ring.write_avail_entry(slot, head);
core::sync::atomic::fence(Ordering::Release);          // dmb ish — device sees table+ring
ring.write_avail_idx(idx.wrapping_add(added));
core::sync::atomic::fence(Ordering::Release);          // idx visible before suppression read
if ring.read_used_flags() & UNONOTIFY == 0 {           // volatile
    notify_reg.write16(queue_index);                   // notify addr = bar4+0x3000+q*4
}
```

### IRQ handler (ISR read-to-ack first)
```rust
// Source: virtio-v1.3 §4.1.4.5 (read resets+deasserts); r9 trap.rs dispatch shape
fn virtio_net_irq() {                        // reached from dispatch_irq's match arm
    let isr = read_isr_byte();               // FIRST — deasserts the level line
    if isr & 0x1 != 0 {
        retire_tx_used();                    // free pages, outpackets accounting
        retire_rx_used();                    // strip 12-byte hdr, demux, wake readers
        replenish_rx();                      // pre-post fresh buffers
        counters.nintr += 1;
    }
    if isr & 0x2 != 0 { /* config change (link) — print once */ }
}
```

### xtask qemu stanza (D-09)
```
# appended to the aarch64 virt qemu argv (all profiles, NIC always-on per D-10):
-netdev user,id=n0
-device virtio-net-pci-non-transitional,netdev=n0
-object filter-dump,id=f0,netdev=n0,file=<target/boot-test-<profile>.pcap>
```

### In-guest sweep fragment (rides the virtshell termrc sweep)
```rc
# after devether is bound (serial marker proves it):
bind -a '#l' /net
echo 'connect 2054' > /net/ether0/0/ctl        # ether-type 0x0806 (ARP) on conv 0
dd if=/lib/test/arp.req of=/net/ether0/0/data  # 60-byte padded ARP request
cat /net/ether0/stats                          # in:/out: counters — serial evidence
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Legacy virtio PCI (I/O-port BAR, PFN queue programming) | Modern-only capabilities in MMIO BARs (VERSION_1) | virtio 1.0 (2015); qemu default-transitional, `-non-transitional` modern | D-03's rejection of legacy; 12-byte header |
| 10-byte net header without MRG_RXBUF | 12 bytes always when VERSION_1 | virtio 1.0 made num_buffers unconditional for non-legacy | Pitfall N1 — CONTEXT.md correction |
| MSI-first virtio bring-up | INTx + DTB interrupt-map on GICv2 | project decision (Pitfall 8) | simpler, DTB-declared, no v2m driver |
| x86 "no visible barriers" ring code | fence-based ordering on aarch64 | always true on ARM; empirically confirmed `dmb ish`/`ishld` | Pitfall N5's barrier-first rule |

**Deprecated/outdated:**
- Transitional device matching (0x1000/rid==0/I/O BAR): rejected loudly at bind per D-3.
- EVENT_IDX: deferred optimization behind its own gate (CONTEXT deferred list).
- MSI/GICv2m: documented non-goal for v2.0.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | qemu 11.1.2's `-M virt` keeps BAR4 "not mapped" at guest start, i.e. r9 must assign BARs — verified for the NIC via `info pci`; assumed to hold across the boot-test invocation path (same machine config) | PCI/ECAM | LOW — if qemu pre-assigns BARs in some path, the assignment code must tolerate an already-valid BAR (read-before-write makes this safe either way) |
| A2 | raspi targets would need cache maintenance for DMA; out of scope since the driver is cfg-gated to virt | DMA/cache reality | none this phase — flagged for the future hardware bring-up milestone |
| A3 | The in-guest sweep's exact profile wiring (whether the ARP gate rides virtshell or a new virtnet profile sharing its initrd) | r9 integration | LOW — planner decision; both shapes are precedented |
| A4 | ping-pong N modest on TCG (suggest N=4 default, env-scalable on KVM) | gates | LOW — D-08 leaves N to planner/researcher discretion; TCG pacing is the only variable |
| A5 | qemu's GICv2 ICFGR defaults SPIs to level-triggered at reset (irq_state zero-init observed; the zeroing itself is inferred from gic realize conventions, and PL011-level behavior is consistent) | GICv2 plumbing | LOW — the plan programs ICFGR explicitly regardless, making the default moot |

## Open Questions (RESOLVED)

All four were discharged at planning time (2026-11-04); each disposition names the plan that adopted it.

1. **RX read blocking vs. polling-read:** the data-file read must block until a frame arrives (netif's qread semantics). r9 has the block/wake hook machinery (devpipe). Confirm in planning that a read on `/net/ether0/0/data` from the sweep is expected to block (it isn't for the ARP gate — the sweep writes, never reads; the stats file carries RX proof). If no reader ever blocks this phase, the block hook can land minimal-but-correct.
   - What we know: the hook pattern exists and is precedented; the ARP gate never reads data.
   - What's unclear: whether any Phase-7 gate wants a blocking data read (recommendation: no — stats + pcap suffice).
   - Disposition (RESOLVED → 07-07): no Phase-7 gate blocks on a data read — the sweep writes, never reads; RX proof is the stats counter + pcap. The blocking read path lands complete-but-gate-unexercised, recorded loudly in devether.rs's module header (07-07 must_haves cites this as OQ-1's disposition).
2. **Queue sizing:** accept qemu's 256 or downsize to 64/128? (Memory: ~3 pages/queue part either way.) Recommendation: accept 256 — zero queue_size write, one less deviation from qemu's tested default.
   - Disposition (RESOLVED → 07-03): accept qemu's 256 with no downsize write — recorded as a flagged assumption in 07-03's frontmatter (EDGE-PROBE NETDRV-02).
3. **RX buffer ring capacity vs. conversation queue depth:** how many frames may queue per conversation before soverflows++? netifinit's `limit` (bsz computed from mbps) — recommend a fixed modest bound (e.g. 64 KiB) this phase.
   - Disposition (RESOLVED → 07-07): a fixed 64 KiB per-conversation inbound queue bound (the netif.rs VecDeque limit, cited as OQ-3 in 07-07 Task 1); queue-full increments soverflows.
4. **The TX-bulk blaster's exact invariant set** (CONTEXT D-08 leaves it to us): recommend ISR-count ≤ TX-frames ≤ ISR-count + coalescing bound; pcap frame count == TX count exactly; zero soverflows/misses; no wedge over the full drain.
   - Disposition (RESOLVED → 07-05 + 07-07): blast invariants adopted — pcap frame count == `netblast: sent` count == stats out, stats in == 0 on a fresh boot, nintr_tx ∈ [1, frames], used==lastused (drained), zero soverflows, NET-DONE inside the budget; arp mode is the exact-reconciling ping-pong (N requests ⇒ N replies). 07-05 unit-proves the assertions on synthetic fixtures; 07-07 runs them live.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-aarch64 | all gates | ✓ (r9 nix devshell) | 11.1.2 | — |
| dtc / fdtput | DTB round-trip, /chosen patching | ✓ (devshell) | 1.8.1 | — |
| rustc/cargo (nightly) | kernel build, host unit tests | ✓ (devshell/rust-toolchain.toml) | 1.95.0-nightly 2026-01-29 | — |
| Harvey `.build/bin/mkpaqfs` | xtask initrd assembly | ✓ via `nix develop -c sh build/aarch64/mkpaqfs-host.sh` | in-tree | — |
| Harvey staged smoke binaries (build/aarch64/smoke-out/) | exec-family initrds | ✓ via `sh build/aarch64/stage-build.sh` (lima leg on darwin) | in-tree | — |
| KVM | none — TCG by design on darwin | ✗ on darwin (HVF rejects GICv2 on 11.1.2 — v1.0 finding) | — | generous TCG timeouts (existing discipline) |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** KVM/HVF acceleration (TCG is the gate substrate — accepted since v1.0).

## Security Domain

This phase adds a DMA-capable device driver — the kernel's first untrusted-byte ingest path (a qemu-controlled device writes guest memory). The applicable posture is driver-boundary hygiene rather than the full ASVS set:

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | **yes** (the device is an input source) | validate used-ring `id`/`len` before indexing (`id < qsize`, `len ≤ buffer len`, len ≥ 12) — a malicious/buggy device must not index out of the arena; treat descriptor contents as untrusted |
| V6 Cryptography | no | — |
| V2/V3/V4 (auth/session/access) | no | single-user kernel era; devether ownership collapses to first-opener semantics (documented) |

**Threat patterns for this stack:**

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious device (firmware/qemu bug) writing outside ring/buffer pages | Tampering | no IOMMU available — mitigate by page hygiene: ring/buffer pages are exclusively owned by the driver (never shared with user mappings); used-len clamp before any copy |
| Interrupt storm (line stuck asserted) | DoS | ISR read-to-ack FIRST in handler; handler does bounded work per IRQ (drain-to-empty with a per-invocation cap + re-check pattern from §2.7.14) |
| Feature-negotiation downgrade (device "offering" trap features) | Elevation of privilege | the zero-offload mask is the allowlist; negotiated mask printed at bind (the audit line) |
| info leak via stale frame buffers | Info disclosure | zero RX buffer pages at allocation; write only `len` bytes to the consumer |

## Sources

### Primary (HIGH confidence — read/executed this session)
- **../r9/port/src/fdt.rs** (768 lines, full read) — helper inventory + gap list
- **../r9/aarch64/src/platform/virt/gic/gicv2.rs**, **gic/mod.rs**, **trap.rs**, **main.rs**, **virt/mod.rs**, **regblock.rs**, **io.rs**, **deviceutil.rs**, **pagealloc.rs**, **port/src/lib.rs**, **port/src/namec.rs**, **port/src/dat.rs**, **port/src/devcons.rs** (pattern), **port/src/allocator_tests.rs**, **xtask/src/main.rs**, **xtask/src/config.rs**, **UNSAFE-AUDIT.md**, **HowItWorks.md**, **Cargo.toml** (workspace) — r9 integration surface
- **../r9/aarch64/lib/virt-gicv2.dts** — the committed DTB: pci node, ECAM window, interrupt-map, dma-coherent
- **sys/src/9k/port/netif.h + netif.c**, **sys/src/9/pc/devether.c**, **sys/src/9/pc/ethervirtio.c** (full reads), **sys/src/9k/ip/ethermedium.c** (padding precedent), **lib/namespace** — Harvey file contract
- **qemu v11.1.2 source** (GitHub raw at the pinned version): hw/virtio/virtio-pci.c (BAR layout, ISR xchg, no modern bus-master auto-enable), hw/net/virtio-net.c (guest_hdr_len rule, queue sizes, feature-gated config), net/dump.c (pcap writer), net/slirp.c (defaults), hw/arm/boot.c + hw/arm/virt.c (-dtb fixup scope), hw/intc/arm_gic.c (ICFGR semantics)
- **libslirp 4.9.5 source** (local nix store `/nix/store/rxsb07j671bv29si6q0wx8ah3scf51pk-source` — the exact version the devshell's qemu links): src/slirp.c arp_input/if_encap4
- **Local probes this session:** `fence()` disassembly (dmb ish/ishld); dumpdtb highmem-ecam on/off; `info pci` device/slot/BAR probe
- **virtio 1.3 csd01 spec** (docs.oasis-open.org, fetched and section-extracted): §2.1, §2.7.x, §3.1.1, §4.1.3-4.1.5, §5.1.x, §6

### Secondary (MEDIUM)
- Devicetree interrupt-map tuple semantics — cross-checked between the committed DTB bytes and the GIC node's cell counts (self-consistent decode); the normative spec text (devicetree.org §2.4.3) was not re-fetched.

### Tertiary (LOW)
- None carried into recommendations.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; all components in-repo or devshell-pinned
- Architecture: HIGH — module split maps 1:1 onto verified r9 conventions (registry_attach, cfg(dev_*), hook injection, unsafe posture)
- Pitfalls: HIGH — six verified against primary sources this session; four newly discovered FROM those sources
- Spec citations: HIGH — fetched and quoted from the normative text
- qemu/slirp behavior: HIGH — exact-version source + live probes

**Research date:** 2026-11-04
**Valid until:** ~30 days (pinned qemu 11.1.2 + pinned toolchain make this stable; re-verify on any r9 flake.lock qemu bump)
