---
phase: "13"
slug: "r9-x86-64-platform-bring-up"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-08"
---

# Phase 13 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| ISO build pipeline → guest boot | grub-mkrescue assembles the boot medium the emulator trusts (T-13-01, T-13-13) | kernel ELF + grub.cfg (integrity-critical) |
| workspace TOML → qemu argv | config values become argv tokens (std::process::Command arg vector, no shell) (T-13-02) | machine/accel/netdev/extra strings (developer-trusted) |
| firmware tables → kernel pointers | mb2 tags and ACPI tables are external input driving memory maps, RSDP/XSDT/MCFG/MADT parses (T-13-03, T-13-07, T-13-08) | table bytes (untrusted until signature+length+checksum) |
| memory map → page allocator | firmware-declared ranges seed the free pool (T-13-04) | type-1 available ranges only |
| emulated devices → kernel rings | virtio-net device writes used rings / config space the kernel validates (T-13-09, T-13-10, T-13-11) | device-written ring entries, ISR state (untrusted) |
| host supply chain → build inputs | nixpkgs flake inputs + runner distro archives (T-13-SC ×2) | grub2, xorriso, toolchain (registry-delegated legitimacy) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-13-01 | Tampering | ISO/channel supply chain (xtask ISO build + mb2 header) | medium | mitigate | Post-build mb2 header-arithmetic check (alignment/magic/checksum/end-tag) refuses an ISO GRUB would silently reject; harness builds the ISO from the same invocation's kernel build | closed |
| T-13-02 | Tampering | qemu argv emission from TOML config | low | accept | argv built via Command::arg vector (no shell interpolation); netdev/extra keys are developer-trusted data like the existing machine/accel strings | closed |
| T-13-03 | Tampering | mb2 info-structure parser | high | mitigate | fdt.rs discipline: typed ParseError, checked slice extraction, total_size/tag-size/alignment validation BEFORE any pointer; malformed tags fail loud; 5 host tests pin truncation/alignment/end-tag | closed |
| T-13-04 | DoS | Page allocator seeding | high | mitigate | Only type-1 (available) entries seed the pool; type-3 ACPI-reclaimable stays reserved; no range the memory map did not declare available | closed |
| T-13-05 | DoS | Interrupt handlers / LAPIC | medium | mitigate | Handlers registered + LVT armed BEFORE sti; EOI strictly after device work; stray vectors print + still EOI; spurious vector skips EOI per spec | closed |
| T-13-06 | DoS | Timer mode selection | medium | mitigate | CPUID gate mandatory (host-tested both ways: bit clear → Periodic); an unconditional TSC-deadline arm would fault-or-hang under TCG | closed |
| T-13-07 | Tampering | ACPI mini-parser (RSDP→XSDT→MCFG/MADT/HPET) | high | mitigate | Signature + length + checksum before ANY pointer; checked field extraction; MADT entry-len bounds; host tests pin bad-checksum/bad-signature/truncation as loud errors | closed |
| T-13-08 | Tampering | PCI ECAM window | high | mitigate | Window comes ONLY from the checksummed MCFG (no constant fallback, loud absence); pciexbar old→new audit print; ECAM reads bounds-checked against the MCFG bus range; perturbation gate reds any hardcoded window | closed |
| T-13-09 | Tampering | PCI device discovery | medium | mitigate | Allowlist-shaped reject (1af4:1041 rev≥1 required, transitional/rev-0 rejected loudly); absent functions read 0xFFFF and are skipped; no device-provided code executes | closed |
| T-13-10 | Tampering/Elevation | virtio-net rings through port/ | high | mitigate | V5 boundary re-proven on x86_64: used-ring id < qsize and even, 12 ≤ len ≤ capacity before any buffer trust; RingMem volatile discipline; host pins green; unsafe confined to the MMIO/DMA seam per UNSAFE-AUDIT.md | closed |
| T-13-11 | DoS | IRQ storm / interrupt wedge | high | mitigate | ISR byte read FIRST (deassert) THEN LAPIC EOI; bounded drain with re-check; unmask LAST; ping-pong exact-reconcile + TX-bulk gates catch the storm class (nintr 256/256, 288-batched, rings drained) | closed |
| T-13-12 | Tampering | Acceptance-gate reconciliation | medium | mitigate | Mismatch prints a named FAIL stage and the terminal marker never lands (fails closed); pcap assertion finish()-time on THIS run's capture, stale pcap removed pre-spawn | closed |
| T-13-13 | Tampering | Reproducible boot-iso derivation | medium | mitigate | Byte-reproducibility proven before the hash was wired (double-build); SOURCE_DATE_EPOCH + pinned volid scrub build-time state; wired hash reds on non-reproducible rebuild by construction | closed |
| T-13-14 | Tampering | Evidence freshness | medium | mitigate | Gates build-what-they-test each invocation; evidence bundle pins the r9 SHA (cc29c7a) and copies THIS phase's run artifacts; CI leg records local validation + PENDING FIRST PUSH rather than claiming an unobserved hosted green | closed |
| T-13-SC (13-01) | Tampering | Package installs (nixpkgs grub2/xorriso; zero crates.io) | high | mitigate | Package Legitimacy Audit in RESEARCH (both OK'd, none [ASSUMED]/[SUS]) via the locked nixpkgs flake input; zero new crates enforced by the empty-manifest-diff criterion | closed |
| T-13-SC (13-06) | Tampering | CI/host supply chain | high | mitigate | All host inputs from the locked nixpkgs flake input or the runner distro archive; zero crates.io additions across the phase (manifest diff = check-cfg predeclares only, verified 2026-11-08); no [ASSUMED] packages | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-13-01 | T-13-02 | qemu argv tokens assembled via Command::arg vector — no shell interpolation surface; the netdev/extra config keys are developer-authored TOML consumed identically to the pre-existing machine/accel strings (dev-machine tooling, not a production input path) | Plan 13-01 threat model (plan-time disposition) | 2026-11-06 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-08 | 16 | 16 | 0 | /gsd:secure-phase 13 (L1 grep-depth verification; all mitigations located in the r9 tree at cc29c7a + the 37/37 verification gates that exercise them; zero new crates confirmed by manifest-diff inspection) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-08
