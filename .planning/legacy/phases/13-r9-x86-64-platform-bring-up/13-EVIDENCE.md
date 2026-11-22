# Phase 13 Evidence Index — r9 x86_64 Platform Bring-Up

**Phase:** 13-r9-x86-64-platform-bring-up
**Consolidated:** 2026-11-07 (plan 13-06, task 3)
**Status:** complete — every requirement (X64-01..05) mapped to the gate runs that prove it, at the pinned r9 SHA, with the retained artifacts under `evidence/`.

## Pinned commits

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`cc29c7a35cb9a74c6ee6bc02607337b5428936c3`** | the phase-complete HEAD at consolidation — all final gate runs of this index executed on this exact tree (21 commits ahead of origin/main: the whole phase; the push + any Harvey flake-lock pin advance is Phase 14's push-before-pin step, never bundled). |
| harvey | `6ab8f91b5` | 13-06 Task 2: `build/x86_64/README.md` — the x86_64 companion (boot chain contract, accel contract, timer chain, qemu drift notes). |
| harvey | this docs commit | 13-EVIDENCE.md + `evidence/` (the consolidation you are reading). |

**Per-plan commit trail (r9, all on `main`, all ancestors of the pinned HEAD):**

| Plan | Commits | Closed |
|------|---------|--------|
| 13-01 (boot channel) | `3200816`, `b3fe837` | X64-01 channel half |
| 13-02 (main9-x64 + mb2 parser) | `e9d3f24` (RED), `d8d14db`, `b964a62` | X64-01 kernel half |
| 13-03 (LAPIC/timer/dispatch) | `c39a210`, `0877f3a` (RED), `333b940` (GREEN), `607a7c5`, `351287f` (x2APIC + 0x6E0 fix) | X64-01 complete; X64-02 LAPIC/timer half |
| 13-04 (ACPI/ECAM discovery) | `0c568de` (RED), `7d5b850` (GREEN), `772db29` (REFACTOR), `cca67bd`, `f607126` | X64-03 |
| 13-05 (IOAPIC + bind + netgate) | `81d6f92`, `b05fda3`, `0cf9c3a` | X64-04; X64-02 load half |
| 13-06 (flake + CI + evidence) | `edd3d97` (kernel-x64 + boot-iso), `cc29c7a` (CI leg) | X64-05 |

## Requirement → gate-run mapping

### X64-01 — the DTB-free boot protocol + the Phase-3-style platform gate

- **Protocol (recorded decisions, mirrored in `build/x86_64/README.md`):** qemu machine = **q35** (PCIe root-complex topology makes the D-08 recursive bridge walk load-bearing; the modern-only theme; `pc` rejected as legacy); boot protocol = **multiboot2 via the GRUB2 chain, recorded AGAINST 9k's multiboot(1)** (9k stays mb1; r9 x86_64 takes mb2 — tagged protocol, loader-mediated lineage matching Harvey's EFI/ISO boot lineage, qemu-independence; qemu's `-kernel` loader is mb1-only, hence SeaBIOS → GRUB2 → mb2 → kernel from a `grub-mkrescue` ISO under `-cdrom`). GRUB 2.14 honors the AT() `p_paddr` bias for the higher-half ELF64 (entry `0xffffffff80200000` at physical `0x200000`); the mb2 info structure is stashed to `.bss` before the Mach-zeroing mebibyte (GRUB 2.14 lands it at 0x16dc08 — inside the zeroed region).
- **Gate runs:** `cargo xtask boot-test --arch x86-64 --config q35` — banner → ≥ 1 `q35: memory` fact → ≥ 3 tick lines → the typed r9echo7 echo, order-anchored, timing-tolerant, negative panic scan. Green **twice consecutively under the KVM-native default** (cpu=host: `lapic: x2apic live`, `TscDeadline mode`) and **green under the TCG fallback** (`--accel tcg --cpu qemu64`: `lapic: xapic 0xfee00000 live`, `Periodic mode`) at 13-03 close (`351287f`); the final-phase runs at the pinned HEAD in `evidence/boot-test-q35.log`.
- **Provenance:** 13-01 (channel), 13-02 (parser + main9), 13-03 (ticks/echo + arming window).

### X64-02 — APIC interrupt plumbing + timers proven under load

- **LAPIC/timer substrate (13-03):** CPUID-gated dual-mode LAPIC (x2APIC MSR map behind leaf-1 ECX bit 21, xAPIC MMIO fallback), CPUID-gated TSC-deadline (the true `IA32_TSC_DEADLINE` MSR **0x6E0** — 0x832 is the x2APIC-mode LVT-timer address and KVM #GP-gates the 0x800-0xBFF range on x2APIC mode; root-caused live, fixed at `351287f`) / periodic fallback, runtime-resolved frequency with printed provenance (CPUID 0x15 → 0x16 sanity-banded → PIT; live 2496 MHz via PIT reference).
- **IOAPIC + load proof (13-05):** MADT-driven IOAPIC (82093AA IOREGSEL 0x00 / IOWIN 0x10 layout, live-verified: id 0 version 0x11 rtes 23 gsib 0; IntSrcOverride fold, zero hardcoded GSIs), ISR-read-first IRQ handler (the level-line deassert BEFORE the LAPIC EOI — Pitfall 5), proven under load by the q35net gate: **ping-pong x256 reconciled exactly (rx == tx == 256, nintr 256/256 — one IRQ per frame, never a re-delivery) and TX-bulk 4096 frames in 288 IRQs** (bounded-drain batching), zero storms, rings drained, fail-closed `netgate: FAIL` markers.
- **Gate runs:** `cargo xtask boot-test --arch x86-64 --config q35net` green ×3 KVM + ×1 TCG at 13-05 close (`0cf9c3a`), re-proven at the pinned HEAD (final run in `evidence/boot-test-q35net.log`; 4352 ARP requests / 4352 gateway replies asserted host-side in `evidence/boot-test-q35net.pcap`, every frame exactly 60 bytes from the guest MAC).

### X64-03 — PCI/ECAM discovery without DTB + the moved-window perturbation

- **Discovery (13-04):** checksum+signature+bounds-gated RSDP→XSDT→MCFG/MADT/HPET mini-parser (mb2 ACPI-tag hint first, EBDA/ROM scan fallback; host-tested over synthetic fixtures incl. corrupt paths); the kernel PROGRAMS the MCH PCIEXBAR (config 0x60) from the parsed MCFG base before the first ECAM access — the SeaBIOS 0xb0000000-hardcode hazard — through the ONE CF8/CFC helper (the recorded D-07 carve-out), printing `pciexbar: <old> -> <new>`; recursive bridge walk (secondary/subordinate at 0x19/0x1A) with the q35 INTx→GSI derivation (swizzle → root slot → PIRQ → GSI 16+pirq, provenance printed).
- **Moved-window proof:** `config_q35lowmcfg.toml` boots with the MCFG table naming **0xe0000000** while SeaBIOS's register says 0xb0000001 — the kernel follows the table (`pciexbar: 0xb0000001 -> 0xe0000001`), walks the moved ECAM window, and finds + routes the device identically (`pci: virtio-net 1af4:1041 at 00:02.0 …`, `pci: inta -> gsi 22`). The perturbation mechanism is `-acpitable file=x86_64/lib/q35-mcfg-moved.bin` — the committed moved-MCFG artifact (qemu 11.1.2 crashes on `-global q35-pcihost.MCFG` at machine init; see the drift notes in `build/x86_64/README.md`).
- **Gate runs:** both gates green in one invocation at 13-04 close (`f607126`: q35lowmcfg then q35); the final-phase KVM runs at the pinned HEAD in `evidence/boot-test-q35lowmcfg.log` (asserting the exact moved bases) and `evidence/boot-test-q35.log` (base window).

### X64-04 — virtio-net through the arch-neutral port stack + host-observable ARP

- **Bind (13-05):** the four port seams (PciCfg/CommonCfg/RingMem/Uart — `RingMem` keeps the trait's core-fence default per Pitfall 6's verdict, 0 mfence in the whole 2M-line disassembly), caps-name-the-BAR, firmware-kept BAR assignment with the derived-window fallback, modern-only negotiation (live: offered 0x10130bf8024 → exactly VERSION_1|MAC|STATUS), queue bring-up with enable LAST, `publish_driver` → `#l0:` + MAC, `ioapic::enable(gsi 22 → vector 0x21, level)`. `driver_ops()` widened pub(crate)→pub (the OQ-1 one-line seam); the kernel-side netgate (cfg(dev_ether)) posts the baked 60-byte gateway-targeted ARP from boot context — the Phase-14 exec swap-in seam.
- **Gate runs:** the q35net gate's pcap assertion is the acceptance: the guest's ARP is host-observable with the exact frame shape — 4352 requests / 4352 replies, 60-byte frames from the negotiated MAC. Green at 13-05 close and at the pinned HEAD (`evidence/boot-test-q35net.pcap`).

### X64-05 — the flake ships the x86_64 package; gates KVM-native by default; TCG documented + CI-able

- **kernel-x64 (`nix build .#kernel-x64`):** the kernel-virt derivation shape re-keyed (`cargo xtask build --arch x86-64 --config q35` — the SAME command the gates prove; the ELF at `kernel-x64.elf`). `nix eval .#packages.x86_64-linux.kernel-x64.pname` → `r9-kernel-x64`. The Phase-14 compose contract exists.
- **boot-iso (`nix build .#boot-iso`):** BIOS-only `grub-mkrescue` derivation, **fixed-output with the hash wired ONLY after the double-build byte-identity check passed**: two independent derivations (fresh kernel builds under both) produced the identical ISO `sha256 87a96c23a421ce886411ef103723b36c025fbb46f7e94ced726685369ca82003`; `nix build .#boot-iso --rebuild` re-verifies against freshly produced bytes (green; also green on a post-commit fresh build). Reproducibility mechanics (all measured on grub 2.14 + xorriso 1.5.8.pl02): the `--xorriso` wrapper pins grub-mkrescue's injected current-time `--modification-date`, adds `--set_all_file_dates` + pinned `-volid`, with `SOURCE_DATE_EPOCH` exported; the staged kernel is strip-all (rustc's per-invocation-random `.llvm.<u64>` anon symbols break symtab-level identity) with a length-preserving zero-zap of build-std's toolchain store-hash panic-path strings (a FOD must not reference store paths and nix's scanner keys on the hash); the grub module tree is grafted from a copy with modinfo.sh's store-path shebang scrubbed. The ISO boots the full chain (banner + x2apic + TSC-deadline under KVM, smoke-verified at the derivation output). Boundary decision: **the flake owns the distributable ISO; the xtask gate keeps building its own dev copy each invocation for evidence freshness** (OQ-3, recorded in the flake comment).
- **KVM-native default loop (the final runs, pinned HEAD):** `for c in q35 q35lowmcfg q35net: cargo xtask boot-test --arch x86-64 --config $c` — **all three green in one loop, no overrides** (the TOML default: accel="kvm", cpu="host"). Logs retained: `evidence/boot-test-{q35,q35lowmcfg,q35net}.log`.
- **TCG fallback (runnable, documented):** `--accel tcg --cpu qemu64` legs green for q35 and q35net (13-03/13-05 closes + this plan's re-runs), all three profiles green under TCG in the CI-sequence local validation below. Documented in `build/x86_64/README.md` (the accel contract + the no-silent-auto-downgrade rule).
- **`nix flake check .`** — all checks passed (no regression; pre-existing warnings only: apps lack `meta`, foreign systems omitted).

## The CI leg — local-validation matrix + PENDING FIRST PUSH

`../r9/.github/workflows/xtask.yml` gains the `boot-test-x86-64` job (commit `cc29c7a`): ubuntu-latest, self-provisioned (`apt-get install -y grub-pc-bin xorriso qemu-system-x86`), the three q35 profiles with EXPLICIT `--accel tcg --cpu qemu64` (hosted runners are KVM-less — RESEARCH A7; explicit selection, not a downgrade).

| Validation step (the no-push CI discipline) | Result |
|---|---|
| YAML parse (`python3 -c "yaml.safe_load(...)"`) | **YAML-OK** (one iteration: a `:` in a step name broke the first parse — fixed pre-commit) |
| Encoded command 1: `cargo xtask boot-test --arch x86-64 --config q35 --accel tcg --cpu qemu64 --verbose` (local: under `nix develop`) | **PASS** |
| Encoded command 2: `cargo xtask boot-test --arch x86-64 --config q35lowmcfg --accel tcg --cpu qemu64 --verbose` | **PASS** |
| Encoded command 3: `cargo xtask boot-test --arch x86-64 --config q35net --accel tcg --cpu qemu64 --verbose` | **PASS** |
| apt provisioning on the runner | **PENDING FIRST PUSH** (unobservable on this NixOS host — no apt; the first hosted run observes it; `grub-mkrescue` arrives via grub-pc-bin's grub-common dependency) |
| hosted run of the job | **PENDING FIRST PUSH** (r9 `main` is 21 commits ahead of origin — the whole phase rides the next push; the hosted leg's observation is recorded when it runs) |

## Recorded decisions for the record

- **X64-01 machine:** q35 over pc — PCIe root-complex topology (the recursive walk is load-bearing), the modern-only theme; pc rejected as legacy. **X64-01 protocol:** mb2 via GRUB2, recorded AGAINST 9k's existing multiboot(1) path — 9k stays mb1; r9 x86_64 takes mb2 (tagged protocol, loader-mediated lineage, qemu-independence; `-kernel` is mb1-only so the channel is SeaBIOS → GRUB2 → ISO → `-cdrom`). Mirrored with rationale in `build/x86_64/README.md`.
- **The CF8/CFC PCIEXBAR carve-out (13-04, OQ-4):** ONE isolated CF8/CFC helper exists for the single MCH config-0x60 host-bridge write (programming PCIEXBAR from the parsed MCFG base before ECAM is live) — framed as host-bridge register programming, not device discovery; all other config access is ECAM. Recorded in the pci.rs module header, the 13-04 commit message, and here.
- **The driver_ops widen + kernel-side netgate seam (13-05, OQ-1):** `port/src/devether.rs`'s `driver_ops()` widened pub(crate)→pub — the one-line seam; the kernel-side netgate consumes it after `sti` as the exec-free ARP trigger. Phase 14 swaps the exec'd `dd`-through-the-file-contract path in at the same seam.
- **The flake-owns-ISO boundary (13-06 Task 1, OQ-3):** the r9 flake owns the DISTRIBUTABLE reproducible boot-iso fixed-output derivation; the xtask gate keeps building its own dev copy per invocation so gate evidence is always fresh against the tree it just built. Recorded in the flake.nix comment.
- **Accelerator honesty (13-06):** the gates never silently fall back to TCG — accel comes only from the explicit chain (flag > `R9_ACCEL`/`R9_CPU` env > TOML kvm default); the CI leg's TCG is explicit selection for a KVM-less venue (the HNET-02 doctrine arriving in Phase 16).

## Regression state at the pinned HEAD (r9 `cc29c7a`)

| Leg | Command | Result |
|-----|---------|--------|
| three q35 profiles, KVM-native default | `for c in q35 q35lowmcfg q35net: cargo xtask boot-test --arch x86-64 --config $c` | PASS ×3 (one loop; `evidence/`) |
| TCG fallback legs | `… --config q35 --accel tcg --cpu qemu64` + `… --config q35net --accel tcg --cpu qemu64` | PASS ×2 (plus q35lowmcfg in the CI-sequence validation) |
| aarch64 sibling leg | `cargo xtask boot-test --arch aarch64 --config virt` | PASS (13-05 close; re-run wherever `R9_HARVEY_DIR` initrd inputs apply) |
| flake | `nix build .#kernel-x64`, `nix build .#boot-iso --rebuild`, `nix flake check .` | green / green (hash re-verified) / all checks passed |
| host suites | `cargo xtask test` / `check` / `clippy` | green at 13-05 close (26 x86_64 + 91 xtask + 101 port); no source changes to port/xtask in 13-06 |

## Retained artifacts (`evidence/`)

| File | What it is |
|------|-----------|
| `evidence/boot-test-q35.log` | final KVM-default q35 gate serial log (banner → facts → discovery → ticks → echo; x2apic + TscDeadline) |
| `evidence/boot-test-q35lowmcfg.log` | final KVM-default moved-window gate serial log (mcfg 0xe0000000, pciexbar 0xb0000001 → 0xe0000001, same device + route) |
| `evidence/boot-test-q35net.log` | final KVM-default net-gate serial log (bind audit, `#l0:` + MAC, ping-pong 256/256, tx-bulk 4096, X64-NET-DONE) |
| `evidence/boot-test-q35net.pcap` | the same run's filter-dump capture (678936 bytes; 4352 ARP requests + 4352 gateway replies, 60-byte frames from the guest MAC — asserted by the gate at finish()) |

---
*Phase: 13-r9-x86-64-platform-bring-up*
*Consolidated: 2026-11-07*
