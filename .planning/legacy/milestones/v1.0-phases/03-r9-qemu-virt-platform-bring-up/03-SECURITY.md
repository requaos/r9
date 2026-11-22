---
phase: "03"
slug: "r9-qemu-virt-platform-bring-up"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-02"
---

# Phase 03 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| flake inputs (github pins) → build environment | nixpkgs and rust-overlay are external sources defining the toolchain executing all later phase work | toolchain provenance |
| toolchain → kernel artifact | the pinned nightly compiles the kernel; silent substitution would poison every downstream gate | build integrity |
| cfg dispatch → compiled code | the platform TOML key selects which module tree compiles | build correctness |
| TOML config → qemu argv | argv construction decides what machine the acceptance gates run on | gate fidelity |
| qemu → guest RAM (DTB, kernel image) | the DTB is machine-generated input parsed by port::fdt; the committed virt-gicv2.dtb is a build artifact of record | untrusted-ish parse input (trusted channel this phase: qemu-generated or committed DTBs only) |
| MMIO windows ↔ kernel mappings | mismapped device ranges as Normal text are a speculative-access hazard (research P3) | device memory safety |
| serial RX bytes → kernel | host-side socket writes enter the kernel as interrupt-delivered bytes — the phase's one live untrusted input path | untrusted input (echo-only) |
| IRQ context ↔ main context | the handler runs preemptively against the Console's mcslock-protected state | kernel liveness |
| committed evidence → milestone record | the evidence/ bundle is the phase's proof artifact | milestone integrity |
| debug artifact (virtpanic) → config surface | the deliberate-panic config must stay clearly labeled | operational safety |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-03-01 | Tampering | flake inputs (nixpkgs, rust-overlay) | medium | mitigate | flake.lock committed pinning both input revisions; single nixpkgs lineage; toolchain identity anchored to in-repo rust-toolchain.toml — verified: `fromRustupToolchainFile` present in ../r9/flake.nix, flake.lock committed (r9 ae4cd08) | closed |
| T-03-02 | Tampering | silent raspi regression from the platform move | low | mitigate | both raspi configs build through the devshell at every plan; raspi behavior pinned by not(virt) gating + same-point unmask; a wrong cfg surfaces as a compile error — verified: raspi3b/raspi4b builds green in 03-02/03-04/03-05/03-06 regression sweeps | closed |
| T-03-03 | Tampering | argv drift between config intent and executed qemu command | low | mitigate | config.rs unit tests assert exact argv emission; QemuStep --verbose prints the constructed command — verified: 11/11 xtask tests green (03-06 sweep) | closed |
| T-03-04 | Tampering | DTB parse of malformed data at the fixed-PA fallback | low | mitigate | port::fdt DeviceTree::from_usize validates FDT magic + bounds-checks property walks (ParseError); verify gate asserts the exact DTB-found address — verified: "DTB found at: 0xffff800040000000" in evidence/virt-boot.log | closed |
| T-03-05 | Tampering | MMIO windows mapped as Normal-cacheable kernel text | medium | mitigate | virt kernel physranges start at boottext_pa (kmem.rs verified); device mappings via map_device_register with device-nGnRE attributes — verified: P3 fix confirmed in 03-VERIFICATION.md | closed |
| T-03-06 | Tampering | serial RX bytes triggering unintended kernel actions | low | mitigate | rx_irq_echo echoes bytes verbatim, zero interpretation; control-byte actions deferred — verified: harness probe exercised echo-only path (03-05) | closed |
| T-03-07 | Denial of Service | IRQ-context Console-lock reentrancy wedging the kernel | medium | mitigate | UART handler lock-free by construction (raw DR writes; region-scoped gate proves no println/Console in rx_irq_echo — re-verified this audit); spurious 1023 acked without EOI; unknown INTIDs log-once + EOI | closed |
| T-03-08 | Tampering | virtpanic debug artifact mistaken for a production config | low | mitigate | debug-artifact header in config_virtpanic.toml; absent from README entirely (grep-verified this audit); requires explicit --config virtpanic selection | closed |
| T-03-09 | Tampering | evidence certifying a stale or different tree | medium | mitigate | boot-test builds the image it tests in the same invocation; EVIDENCE.md records r9 HEAD 55ca7c4; verifier reproduced both logs byte-for-byte from that SHA | closed |
| T-03-SC (all six plans) | Tampering | npm/pip/cargo registry installs | high | mitigate | zero new crate dependencies across the phase — verified: only aarch64/Cargo.toml touched (+11 lines, the [lints.rust] unexpected_cfgs check-cfg block, no deps); Cargo.lock untouched; nix-side packages flake.lock-pinned; package-legitimacy gate vacuous | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-02 | 10 | 10 | 0 | gsd-secure-phase (L1, register authored at plan time; short-circuit per asvs_level 1) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-02
