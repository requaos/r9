---
phase: "07"
slug: "r9-pci-ecam-virtio-net-devether"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-05"
---

# Phase 07 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| qemu device → r9 kernel | The virtio-net device writes the used ring and RX buffers; qemu is emulated hardware but treated as untrusted input (device-written id/len validated before any indexing) | Frame bytes, ring indices/lengths |
| qemu process → evidence files | filter-dump writes the pcap; boot-test reads serial + pcap; malformed capture content crosses into the parser | pcap bytes, serial text |
| workspace TOML → qemu argv | config values become argv tokens (std::process::Command arg vector — no shell) | Config strings |
| User ctl writes → devether | `#l` ctl verbs parsed from user-written commands | ctl strings |
| GitHub → flake.lock pin | the r9 pin advance consumes pushed commits | Supply chain (commit SHAs) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-07-01 | Tampering | xtask pcap parser (pcap.rs) | medium | mitigate | Bounds-checked byte-slice parsing; truncation/bad-magic return loud Err; unit tests pin (07-01) | closed |
| T-07-02 | Tampering | qemu argv emission from TOML | low | accept | argv via Command::arg vector (no shell interpolation); static stanza tokens — see Accepted Risks | closed |
| T-07-03 | Tampering | fdt.rs interrupt-map iterator | medium | mitigate | Stride from declared cell counts; cursor bounds-checked; non-multiple tail terminates without panic; tests pin (07-02) | closed |
| T-07-04 | Tampering | pci.rs ECAM window from DTB reg | medium | mitigate | Bounded VirtRange mapping; bus walk bounds-checks ECAM offsets; vendor 0xFFFFFFFF = absent; only 1AF4:1041 rev≥1 bound (07-02) | closed |
| T-07-05 | Tampering | virtio ring index model | high | mitigate | Used-ring id/len surfaced as CHECKED results (id < qsize, len ≥ 12, len ≤ buffer); host tests pin wrap/alias/retire (07-03) | closed |
| T-07-06 | Tampering | sweep serial-marker contract | low | accept | Markers consumed by presence assertions; malformed stream fails loudly, never silent-pass — see Accepted Risks | closed |
| T-07-07 | Tampering | serial-capture stats/ifstats parsing | low | mitigate | Presence-and-value assertions with got/want messages; length-checked byte scanning (07-05) | closed |
| T-07-08 | Tampering | DMA buffer bounds | high | mitigate | No IOMMU — page hygiene: ring/buffer pages exclusively driver-owned, never user-aliased; used len clamped before copy (07-03/06) | closed |
| T-07-09 | Tampering | BAR sizing readback | medium | mitigate | All-ones sizing write always restored (pci.rs:367-370, :530); window from DTB ranges (host truth); readback sanity print (07-06) | closed |
| T-07-10 | Elevation of privilege | feature-negotiation downgrade | medium | mitigate | ACCEPT_LO = MAC\|STATUS, ACCEPT_HI = VERSION_1 allowlist; offered & mask (virtio.rs:220-292); loud reject on missing VERSION_1 (07-03) | closed |
| T-07-11 | Tampering | capability offset/length outside BAR | medium | mitigate | Every cap offset+length bounds-checked against BAR size before mapping/reading; lying cap aborts bind loudly (07-06). Hardening gaps recorded in 07-REVIEW.md WR-01/02/04 (open in disposition ledger — non-blocking follow-up) | closed |
| T-07-12 | Tampering | used-ring id/len indexing | high | mitigate | id/qsize/len validated before any indexing or copy; a failed validation is a loud device fault, never an alias (virtio.rs:726-743; 07-06) | closed |
| T-07-13 | Denial of Service | interrupt storm (level line stuck) | high | mitigate | ISR byte read FIRST (read-to-ack §4.1.4.5, pci.rs:324-326); bounded retire per invocation; EOI exactly once; wedge = gate timeout, never silence (07-06) | closed |
| T-07-14 | Info disclosure | stale RX buffer contents | medium | mitigate | RX pages zeroed at allocation; only validated `len − 12` bytes exposed; short/oversized used len rejected before copy (07-06; offset fix 07-10) | closed |
| T-07-15 | Tampering | ctl verb input | medium | mitigate | Bounded integer parse, type range-checked; unknown verbs/out-of-range error loudly; no ctl write mutates driver state this era (07-07) | closed |
| T-07-16 | Denial of Service | always-on NIC expands boot surface | medium | accept | Deliberate user decision (D-10); device is qemu's, not physical; mitigations are the register itself — see Accepted Risks | closed |
| T-07-17 | Tampering (supply chain) | r9 pin advance | high | mitigate | Push-before-pin; locked-rev==live-HEAD gate; owner==requaos; isolated pin commit (07-08 executed) | closed |
| T-07-18 | Tampering | compose argv construction | low | accept | Static tokens via existing `set --`/spawn-list shapes; no variable interpolation — see Accepted Risks | closed |
| T-07-19 | Denial of Service | readnum_content usize underflow | high | mitigate | u32 truncation makes digits ≤ 10 < NUMSIZE−1 structurally; byte-exact pins in devether_tests.rs (07-09 IS the mitigation) | closed |
| T-07-20 | Tampering (supply chain) | pin advance carrying G-07-2 fix | high | mitigate | T-07-17's gate re-applied at fix HEAD 1c16193 (07-09 Task 4 executed, CI 42043430592 green) | closed |
| T-07-21 | Information Disclosure | RX drain slice overread (+12 base overreads up to 108 B past the 4 KiB payload page) | medium | mitigate | Offset-0 fix bounds the slice to the page by construction (frame_len ≤ 4096 via next_used validation); max-size arm in rx_chain_tests pins the cure (07-10 IS the mitigation) | closed |
| T-07-22 | Tampering (supply chain) | pin advance carrying G-07-1 fix | high | mitigate | T-07-17/20's gate re-applied at fix HEAD a5b1b42 (07-10 Task 4 executed, CI 42058021884 green; flake.lock r9 node rev == a5b1b42, owner requaos) | closed |
| T-07-SC | Tampering | npm/pip/cargo installs | high | accept | Zero new dependencies across all 10 plans (D-01 forbids the virtio-drivers crate; empty-Cargo-diff acceptance criteria; devshell-pinned qemu/dtc; in-tree toolchain) — see Accepted Risks | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-07-1 | T-07-02 | qemu argv is built via Command::arg vectors (no shell interpolation); the added netdev/filter stanzas are static strings; free-form config strings (machine/accel) are developer-trusted inputs consumed the same way | plan 07-01 (accept disposition) | 2026-11-05 |
| AR-07-2 | T-07-06 | Sweep serial markers are consumed by presence assertions in the gate harness; malformed serial fails the gate loudly by absence — a silent pass is not reachable | plan 07-04 (accept disposition) | 2026-11-05 |
| AR-07-3 | T-07-16 | Always-on NIC on every virt boot is a deliberate user decision (D-10); the device is qemu's emulated NIC, not physical hardware, and the register's mitigations (zero-offload mask, bounds-checked rings, ISR read-to-ack, bounded drain, negative scan) all landed | plan 07-08 (accept disposition) | 2026-11-05 |
| AR-07-4 | T-07-18 | Compose argv additions are static strings appended via the scripts' existing set --/spawn-list shapes; no shell interpolation of variable content | plan 07-08 (accept disposition) | 2026-11-05 |
| AR-07-5 | T-07-SC | Zero new package installs or crate dependencies across the entire phase (D-01/D-06 enforced by empty-Cargo-diff acceptance criteria in every plan); the package-legitimacy gate is vacuous by construction | all plans (accept disposition) | 2026-11-05 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-05 | 23 | 23 | 0 | gsd-secure-phase (L1 grep-depth; register authored at plan time; ASVS 1 short-circuit) |

Mitigation evidence spot-verified against live code at r9 HEAD a5b1b42: ACCEPT mask (virtio.rs:220-292), used-ring validation (:726-743), ISR read-first (pci.rs:324-326), BAR sizing restore (:367-370, :530), readnum_content u32 truncation (devether.rs:496), virtio_net_frame_offset()==0 (virtio.rs:846-848), pin chain (flake.lock r9 node rev == a5b1b42, owner requaos; CI runs 42043430592 + 42058021884 green at the pinned HEADs). UAT guest re-run passed 2026-11-05 (07-UAT.md: 2/2).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-05
