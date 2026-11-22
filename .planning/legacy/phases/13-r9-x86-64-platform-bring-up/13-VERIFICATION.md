---
phase: 13-r9-x86-64-platform-bring-up
verified: 2026-11-07T17:40:32Z
status: passed
score: 37/37 must-haves verified (5/5 roadmap success criteria + 32/32 plan truths)
covered_files: [".planning/phases/13-r9-x86-64-platform-bring-up/13-01-PLAN.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-01-SUMMARY.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-02-PLAN.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-02-SUMMARY.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-03-PLAN.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-03-SUMMARY.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-04-PLAN.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-04-SUMMARY.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-05-PLAN.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-05-SUMMARY.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-06-PLAN.md", ".planning/phases/13-r9-x86-64-platform-bring-up/13-06-SUMMARY.md", "build/x86_64/README.md"]
covered_digest: "v4:sha256:befb90f086f2e2eef24d883d6014d296df314943b5f0632b455850e730ad0d2e"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 13: r9 x86_64 Platform Bring-Up Verification Report

**Phase Goal:** r9 boots and speaks Ethernet on x86_64 — a q35/pc platform layer with a recorded DTB-free boot protocol, live APIC/HPET interrupts, data-driven PCI/ECAM discovery, and virtio-net bound through the arch-neutral `port/` stack — with KVM-native acceptance gates as the default. **Kernel work in sibling repo `../r9`.**

**Verified:** 2026-11-07T17:40:32Z
**Status:** passed
**Re-verification:** No — initial verification
**Kernel tree pinned at:** r9 `cc29c7a` (phase range `581e3c3..HEAD`, 21 commits, working tree clean — matches 13-EVIDENCE.md's pinned HEAD)

**Verification method note:** every acceptance gate, host test suite, and nix build claimed by the phase was **re-executed live by the verifier** on the actual tree — no SUMMARY claim below rests on the executor's own runs. Cross-repo key links (`../r9/...`) were verified by direct source inspection because `verify.key-links` rejects paths outside the Harvey root (tool limitation, recorded per-link below).

## Goal Achievement

### Observable Truths (roadmap contract)

| # | Truth (Success Criterion) | Status | Evidence |
|---|---------------------------|--------|----------|
| 1 | **X64-01** — r9 boots on qemu x86_64 via a recorded DTB-free boot protocol (q35-vs-pc decision recorded with rationale, protocol decided against 9k's multiboot path); boot-test gate asserts banner → ticks → serial echo in order, green on repeated runs | ✓ VERIFIED | Decisions recorded with rationale in `build/x86_64/README.md` ("Recorded decision: q35, not pc (X64-01)"; "-kernel loader speaks multiboot 1 ONLY" → SeaBIOS→GRUB2→mb2 channel). mb2 header in `../r9/x86_64/src/l.S` (magic `0xE85250D6` at line 117) + xtask post-build check (`check_mb2_header`, `xtask/src/main.rs:2992` with fixture tests at 3254-3287); live boot printed "mb2: header at file offset 0x1008 validated". Gate **re-run green by verifier twice**: KVM-native default (exit 0) and TCG fallback (exit 0), both asserting banner, ≥1 memory fact, discovery markers, ≥3 ticks, typed echo (`r9echo7`) in order; 13-03/13-06 evidence additionally shows twice-consecutive KVM runs |
| 2 | **X64-02** — APIC (LAPIC + IOAPIC) and HPET/PIT timers proven under load; INTx level-triggered virtio ISR read-to-ack re-proven; sustained IRQ-load run with no lost or wedged interrupts | ✓ VERIFIED | `lapic.rs` (SyncUnsafeCell publish-once, x2APIC/xAPIC CPUID-gated), `ioapic.rs` (MADT-driven, 82093AA IOWIN 0x10 layout), timer frequency runtime-resolved (live log: "2496108300 Hz (pit 1193182 Hz reference)"; HPET table parsed: "hpet 0xfed00000"); EOI-after-device-work ordering confirmed in `trap.rs:458-477` (`dispatch_irq` runs handler then `lapic::eoi()`, with the read-to-ack rationale comment); **q35net gate re-run green by verifier**: ping-pong exact reconcile (256/256, nintr 256/256 — one IRQ per frame), TX-bulk 4096 frames in 291 IRQs (bounded drain), zero wedges |
| 3 | **X64-03** — PCI/ECAM discovery without DTB (ACPI MCFG), zero hardcoded addresses; moved-window perturbation still finds and binds the device | ✓ VERIFIED | `acpi.rs` checksum/signature/bounds-gated RSDP→XSDT→MCFG/MADT parser (5 host tests green, verifier-run); `pci.rs` programs MCH PCIEXBAR (config 0x60) via the single recorded CF8/CFC carve-out, prints audit line; `0xb0000000` appears only in comments (no constant fallback — grep-verified). **lowmcfg gate re-run green by verifier**: MCFG table at 0xe0000000, `pciexbar: 0xb0000001 -> 0xe0000001`, device still found (`1af4:1041 at 00:02.0`, `inta -> gsi 22`). Perturbation mechanism is data (`extra = ["-acpitable", "file=...q35-mcfg-moved.bin"]` in config_q35lowmcfg.toml; the qemu 11.1.2 `-global` crash is documented in README drift notes) |
| 4 | **X64-04** — virtio-net binds through arch-neutral `port/`, devether presents `/net/ether0` with live RX/TX counters; guest ARP observable in host-side capture | ✓ VERIFIED | Four port seams in `pci.rs` (RingMem keeps trait-default fence — verifier objdump of built kernel: **0 mfence** in the whole disassembly); modern-only negotiation (live: offered `0x10130bf8024` negotiated `0x100010020` = exactly VERSION_1\|STATUS\|MAC); `#l0: virtio: 1000Mbps: 52:54:00:12:34:56` bind print; live ifstats tuples in serial (avail/used/lastused/nintr/nnote per queue); `driver_ops()` widened `pub` (`port/src/devether.rs:149`), consumed by `netgate.rs:162`. **Verifier parsed the retained pcap independently**: 8704 frames = 4352 ARP requests + 4352 gateway replies, 60-byte frames, senders {guest MAC 52:54:00:12:34:56, slirp 52:55:0a:00:02:02} — and the q35net gate re-run (fresh pcap) asserted the same counts live |
| 5 | **X64-05** — flake ships x86_64 kernel package; gates KVM-native by default; TCG fallback documented | ✓ VERIFIED | `nix eval .#packages.x86_64-linux.kernel-x64.pname` → `r9-kernel-x64`; **verifier ran `nix build .#kernel-x64`** — succeeded, output carries `kernel-x64.elf` (18 MB ELF 64-bit x86-64). **Verifier ran a fresh `nix build .#boot-iso`** — fixed-output derivation validated against wired hash (recursive-path hash `74248f3d…`), and the produced `boot.iso` file sha256 is **exactly** `87a96c23a421ce886411ef103723b36c025fbb46f7e94ced726685369ca82003` — the double-build-proven hash recorded in 13-EVIDENCE.md (the two hash values are different hash scopes, both correct). All three q35 profiles **re-run green by verifier in one KVM-native loop** (no overrides). TCG documented (`build/x86_64/README.md`, 15 tcg mentions) and runnable (verifier re-ran the q35 TCG leg green). CI job `boot-test-x86-64` present in `../r9/.github/workflows/xtask.yml` (verifier parse: YAML-OK) with explicit tcg/qemu64 + apt provisioning; hosted run recorded PENDING FIRST PUSH per the no-push discipline |

### Plan Truth Coverage (32/32 VERIFIED)

| Plan | Truths | Result |
|------|--------|--------|
| 13-01 (5) | mb2 header valid + xtask check + fixture test; TOML-sourced `-cdrom` channel (D-01 keys, no `-kernel`, `elf32-i386`/`r9.elf32` retired — grep count 0; `-cdrom` at xtask main.rs:753); gate green repeated; idle netdev stanza as data + `[qemu] extra` passthrough (config.rs:106-118, 442-445); sibling arches unchanged (aarch64 virt gate **verifier-run green**; `apply_virt_netdev` body untouched per git diff — doc comments only) | 5/5 ✓ |
| 13-02 (5) | main9-x64 arc parses mb2 (`main.rs:80-99`, facts → pagealloc + print_machine_info); platform/mod.rs cfg dispatch exists; old-gen retired (proc/syscall/node0/vsvm/devcons.rs all gone, `vsvm` count in dat.rs = 0); mb2 parser bounds-checked with typed ParseError (5 host tests green, verifier-run); gate marker set asserts memory facts (gate PASS line names "≥ 1 q35 memory-map fact line") | 5/5 ✓ |
| 13-03 (6) | LAPIC publish-once (SyncUnsafeCell ×2, LINT mask policy); CPUID-gated dual-mode timer — true IA32_TSC_DEADLINE `0x6E0` (the plan's `0x832` value was root-caused as wrong mid-phase and fixed at `351287f`; timer.rs documents why; mode gate proven both ways live: KVM = x2apic+TscDeadline, TCG = xapic+Periodic); IDT dispatch + EOI-last + sti-last; polled serial echo (`uart16550::rx_echo` polling LSR, called from `timer::tick()` at timer.rs:214); gate green twice consecutively (evidence + verifier re-runs); TCG leg passes (verifier-run) | 6/6 ✓ |
| 13-04 (5) | ACPI mini-parser with checksum gates + 5 host tests; PCIEXBAR programming with audit print (live); recursive bridge walk finds virtio-net with reject logic; publish-once discovery (SyncUnsafeCell ×4 in pci.rs); moved-window gate green (verifier-run) | 5/5 ✓ |
| 13-05 (6) | IOAPIC MADT-driven 0x10-layout (host tests: override fold, RTE field map); four-seam bind modern-only (live negotiation audit); devether `#l0:` + MAC + live counters; ARP host-observable (verifier pcap parse); IRQ discipline under load — exact reconcile (256/256, nintr 256/256) + bounded TX-bulk (4096/291) in verifier-run gate; port host pins green (verifier-run: 101 passed) + objdump 0 mfence | 6/6 ✓ |
| 13-06 (5) | kernel-x64 builds with ELF (verifier-run); boot-iso reproducible fixed-output — **hash verified live at both scopes by verifier**; KVM-native loop green ×3 (verifier-run); TCG documented + runnable (verifier-run); evidence consolidated in 13-EVIDENCE.md with SHA pinning + retained artifacts (read and cross-checked by verifier) | 5/5 ✓ |

**Score:** 37/37 truths verified (0 present-but-behavior-unverified — every behavior-dependent truth was exercised by a verifier-executed gate, test suite, or build)

### Required Artifacts

gsd-tools `verify.artifacts`: **20/20 passed** across the six plans (4/4, 3/3, 3/3, 3/3, 4/4, 3/3). Notable: the 13-03 artifact pattern `contains: 0x832` passes because timer.rs documents the 0x832 erratum in its module header — the actual MSR constant is the corrected `0x6E0` (line 34), which is the right outcome (commit `351287f` root-caused 0x832 as the x2APIC LVT address, not IA32_TSC_DEADLINE).

| Artifact | Status | Evidence |
|----------|--------|----------|
| `../r9/x86_64/lib/config_q35.toml` | ✓ VERIFIED | D-01 keys + netdev table + `platform = "q35"`; drives live gate |
| `../r9/x86_64/src/l.S` | ✓ VERIFIED | mb2 header magic 0xE85250D6 (line 117); GRUB boots it live |
| `../r9/xtask/src/main.rs` | ✓ VERIFIED | `-cdrom` arm, check_mb2_header + fixture tests, marker machinery |
| `../r9/xtask/src/config.rs` | ✓ VERIFIED | netdev/extra schema + emitter + tests |
| `../r9/x86_64/src/platform/q35/mb2.rs` | ✓ VERIFIED | ParseError + 5 host tests green |
| `../r9/x86_64/src/platform/q35/mod.rs` | ✓ VERIFIED | print_machine_info slot → acpi/pci, facts published once |
| `../r9/x86_64/src/platform/q35/lapic.rs` | ✓ VERIFIED | SyncUnsafeCell registry, x2APIC/xAPIC dual mode |
| `../r9/x86_64/src/platform/q35/timer.rs` | ✓ VERIFIED | 0x6E0 constant + runtime freq, 3 host tests |
| `../r9/x86_64/src/trap.rs` | ✓ VERIFIED | IDT + dispatch_irq, device-arm + EOI-last |
| `../r9/x86_64/src/platform/q35/acpi.rs` | ✓ VERIFIED | checksum gates, 5 host tests |
| `../r9/x86_64/src/platform/q35/pci.rs` | ✓ VERIFIED | discovery + bind halves, 63 KB substantive |
| `../r9/x86_64/lib/config_q35lowmcfg.toml` | ✓ VERIFIED | `extra` acpitable tokens — data |
| `../r9/x86_64/src/platform/q35/ioapic.rs` | ✓ VERIFIED | 0x10 layout, 3 host tests |
| `../r9/x86_64/src/platform/q35/netgate.rs` | ✓ VERIFIED | ARP fixture + reconcile, 4 host tests |
| `../r9/x86_64/lib/config_q35net.toml` | ✓ VERIFIED | `dev = ["ether"]` |
| `../r9/flake.nix` | ✓ VERIFIED | kernel-x64 + fixed-output boot-iso, both built by verifier |
| `build/x86_64/README.md` | ✓ VERIFIED | recorded decisions, accel contract, timer chain, drift notes |
| `.planning/.../13-EVIDENCE.md` | ✓ VERIFIED | full requirement→gate map, SHA-pinned, artifacts retained |
| `../r9/.github/workflows/xtask.yml` | ✓ VERIFIED | boot-test-x86-64 job, YAML-OK |
| `../r9/x86_64/src/main.rs` | ✓ VERIFIED | main9-x64 arc (mb2 stash, parse, arming window) |

### Key Link Verification

`verify.key-links` rejects `../r9/...` paths (outside Harvey root) — all cross-repo links verified by direct source inspection:

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| config_q35.toml | xtask/config.rs | netdev/extra keys → emitter | ✓ WIRED | Qemu struct fields (106-118), emission (442-462); live pcap produced from the stanza |
| xtask/main.rs | x86_64/src/l.S | mb2 header check 0xE85250D6 | ✓ WIRED | check_mb2_header validated the built ELF live ("header at file offset 0x1008 validated") |
| main.rs | platform/q35/mb2.rs | mbdata → parse → facts | ✓ WIRED | main.rs:80-99; live memory-fact prints |
| platform/mod.rs | platform/q35/mod.rs | cfg(platform = "q35") | ✓ WIRED | both arms compile q35 (host-test rationale documented in module header) |
| trap.rs | lapic.rs | handler-then-EOI | ✓ WIRED | dispatch_irq:458-477 |
| timer.rs | uart16550.rs | LSR-polled RX sweep | ✓ WIRED | timer::tick → rx_echo(COM1) (timer.rs:214); r9echo7 observed live |
| q35/mod.rs | acpi.rs | print_machine_info → ACPI walk | ✓ WIRED | mod.rs:57-109; MCFG/MADT/HPET prints live |
| acpi.rs | pci.rs | MCFG drives PCIEXBAR + ECAM | ✓ WIRED | discover_and_print consumes Acpi; perturbation run proves data-driven |
| trap.rs | pci.rs | published-vector guard → ISR-first handler | ✓ WIRED | trap.rs:471-472 (`virtio_irq_vector()` guard, cfg(dev_ether)) |
| netgate.rs | port/devether.rs | pub driver_ops() seam | ✓ WIRED | devether.rs:149 `pub fn`, netgate.rs:162 consumer |
| flake.nix | xtask/main.rs | same build command as gates | ✓ WIRED | flake buildPhase runs `cargo xtask build --arch x86-64 --config q35` |
| 13-EVIDENCE.md | xtask.yml | PENDING FIRST PUSH record | ✓ WIRED | tool-verified (pattern in source); CI-leg matrix present in evidence |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| memory facts | mb2 ranges | mb2 info structure (GRUB) | live prints match qemu q35 RAM map | ✓ FLOWING |
| MCFG window | mcfg base/buses | ACPI table (last-MCFG override via -acpitable) | 0xb0000000 default / 0xe0000000 perturbed — both live | ✓ FLOWING |
| timer frequency | tsc hz | CPUID 0x15 → PIT calibration at runtime | 2496 MHz printed, differs per run (2496108300 / 2496105260 / 2496389180) — not a constant | ✓ FLOWING |
| ifstats counters | avail/used/nintr/nnote | live virtq rings + IRQ path | ping-pong 256/256 nintr 256; tx-bulk 4352 used / 291 intr | ✓ FLOWING |
| pcap evidence | ARP frames | filter-dump on the live netdev | verifier-parsed: 4352 req + 4352 replies, 60-byte, guest MAC | ✓ FLOWING |

### Behavioral Spot-Checks / Probe Execution (all verifier-executed, 2026-11-07)

| Behavior / Probe | Command | Result | Status |
|------------------|---------|--------|--------|
| X64-01 gate, KVM default | `cargo xtask boot-test --arch x86-64 --config q35` | exit 0 — banner, facts, discovery, ≥3 ticks, echo in order | ✓ PASS |
| X64-01 gate, TCG fallback | `… --config q35 --accel tcg --cpu qemu64` | exit 0 — xapic + Periodic mode (CPUID gating proven) | ✓ PASS |
| X64-03 perturbation gate | `cargo xtask boot-test --arch x86-64 --config q35lowmcfg` | exit 0 — `pciexbar: 0xb0000001 -> 0xe0000001`, device found | ✓ PASS |
| X64-04/X64-02 net gate | `cargo xtask boot-test --arch x86-64 --config q35net` | exit 0 — bind, #l0, 256/256 exact, 4096-bulk, pcap assertion, X64-NET-DONE | ✓ PASS |
| Host suites | `cargo xtask test` + `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` | port 101 passed; x86_64 26 passed (mb2 5, acpi 5, timer 3, lapic 2, ioapic 3, netgate 4, pci 4) | ✓ PASS |
| Sibling-arch regression | `R9_HARVEY_DIR=… cargo xtask boot-test --arch aarch64 --config virt` | exit 0 — banner, ticks, echo | ✓ PASS |
| X64-05 package | `nix build .#kernel-x64` | output carries kernel-x64.elf (ELF 64-bit x86-64) | ✓ PASS |
| X64-05 reproducibility | `nix build .#boot-iso` (fresh) | FOD validated vs wired hash; boot.iso sha256 == recorded 87a96c23… | ✓ PASS |
| CI workflow parse | python3 yaml.safe_load | YAML-OK | ✓ PASS |
| Fence lowering | `objdump -d target/…/x86_64 \| grep -c mfence` | 0 — trait-default fence compiled away per TSO analysis | ✓ PASS |
| pcap content | verifier-side python parse of retained pcap | 8704 frames = 4352 ARP req + 4352 replies, 60-byte, guest MAC 52:54:00:12:34:56 | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|-------------|--------|----------|
| X64-01 | 13-01, 13-02, 13-03 | ✓ SATISFIED | recorded protocol (README), gate green ×2 KVM + TCG (verifier-run) |
| X64-02 | 13-03, 13-05 | ✓ SATISFIED | LAPIC/IOAPIC live, PIT-calibrated dual-mode timer, exact ping-pong reconcile + bounded bulk (verifier-run) |
| X64-03 | 13-04 | ✓ SATISFIED | MCFG-driven ECAM, no hardcoded fallback, perturbation gate green (verifier-run) |
| X64-04 | 13-05 | ✓ SATISFIED | four-seam port bind, #l0 + MAC, live counters, ARP host-observable (verifier pcap parse + gate assertion) |
| X64-05 | 13-06 | ✓ SATISFIED | kernel-x64 + reproducible boot-iso (verifier-built), KVM-default loop ×3 (verifier-run), TCG documented + runnable, CI leg authored + parse-checked |

Orphan check: REQUIREMENTS.md maps exactly X64-01..05 to Phase 13; all five are claimed by plan frontmatter. No orphans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `../r9/x86_64/src/dat.rs` | 97 | `// XXX: remove soon.` | ℹ️ Info (pre-existing) | Byte-identical at phase base `581e3c3` (line 113 there) — inherited debt on the `Trap::Intr` variant, untouched by this phase; not phase-introduced, completion remains auditable |
| `../r9/xtask/src/main.rs` | 653, 693-694 | `TODO Choose UART`, 2× `FIXME` | ℹ️ Info (pre-existing) | All three present verbatim at base `581e3c3` (lines 649/689/690) — inherited, untouched |
| `../r9/x86_64/src/trap.rs` | 239-246 | CR-01 (open review finding): `intrspecial` SWAPPED-witness offset +16 should be +24 | ⚠️ Warning | See "Notable Findings" — dormant in every Phase-13 configuration; pre-phase introduction; tracked open in 13-REVIEW-DISPOSITION.md |

No stubs, no placeholders, no console.log-only paths, no hardcoded-empty data: all dynamic values trace to mb2/ACPI/config-space/live-ring sources (Level 4 table above).

### Human Verification Required

None. All behavior-dependent claims were re-proven by verifier-executed gates, host test suites, and nix builds; no visual/UX/external-service surface exists in this phase. No `<human-check>` blocks exist in any plan.

### Notable Findings (non-blocking, for the record)

1. **CR-01 (open critical review finding, `trap.rs:239-246`):** the `intrspecial` stub marks the SWAPPED witness bit in the saved RIP word (`+16`) instead of the saved CS word (`+24`). Per the reviewer's own dormancy analysis — no user processes until Phase 14 and `%GSBASE` always the Mach base at entry — the buggy branch is unreachable on every Phase-13 configuration (consistent with all gates green). Introduced pre-phase (`0710f69`, present at base `581e3c3`). It is recorded open in `13-REVIEW-DISPOSITION.md` awaiting triage; **Phase 14 (XEXEC, user mode) activates this path and must not start without triaging it.**
2. **Open review findings WR-01..04, IN-01..04** — all recorded `open` in 13-REVIEW-DISPOSITION.md; none falsify a Phase-13 success criterion.
3. **Hosted CI leg is PENDING FIRST PUSH** (r9 main is 21 commits ahead of origin — the whole phase rides the next push, deliberately unbundled per the evidence file). Local validation matrix (YAML parse + all three encoded TCG commands) is recorded and the verifier re-confirmed the parse.
4. **`.planning/WINDOWS.md` ledger corruption** (pre-existing planning-infra issue, `deferred-items.md` status: open) — blocks new ledger appends only; no effect on phase deliverables.
5. **Bare `cargo test -p x86_64` does not compile** (581 errors: the default target picks the no_std custom target without `--bins`). The canonical runner `cargo xtask test` and the documented shape `cargo test --package x86_64 --bins --target x86_64-unknown-linux-gnu` both work (26 tests green). Not a defect — the crate is no_std — but worth knowing before ad-hoc invocations.

### Gaps Summary

No gaps. All five roadmap success criteria and all 32 plan truths verified with live evidence re-executed by the verifier (not executor claims): every gate green under the KVM-native default, the TCG fallback proven, the moved-window and pcap proofs reproduced, host test suites green, and both nix packages built with the reproducibility hash reconciled at both scopes. Score 37/37, 0 behavior-unverified, 0 overrides. The open review findings (CR-01 et al.) are tracked in the review-disposition workflow and flagged above for Phase-14 attention.

---

_Verified: 2026-11-07T17:40:32Z_
_Verifier: Claude (gsd-verifier)_
