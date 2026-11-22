# Phase 13: r9 x86_64 Platform Bring-Up - Context

**Gathered:** 2026-11-06
**Status:** Ready for planning

<domain>
## Phase Boundary

r9's x86_64 crate becomes a peer of the aarch64 crate on the current `port/`-based kernel generation: it boots on qemu q35 via Multiboot2 through a GRUB2 chain, brings up LAPIC + IOAPIC interrupts and a TSC-deadline timer, discovers PCI through an ACPI MCFG-located ECAM window, and binds virtio-net + devether through the arch-neutral `port/` stack — with KVM-native acceptance gates as the default and the r9 flake shipping an x86_64 kernel package. **All kernel work in sibling repo `../r9`** (fork `requaos/r9`, branch `main`). Harvey-side touch: evidence consolidation and (per X64-05) the flake/kernel-package contract Phase 14 composes.

Addresses X64-01, X64-02, X64-03, X64-04, X64-05.

NOT in this phase: amd64 userspace exec/ABI/boot-chain (Phase 14 — XEXEC-01..05), userspace tiers re-keying (Phase 15), host net modes / tap leg / NETIP-08 (Phase 16), kernel flush-out fixes (Phase 17), auth (Phase 18), cpu/drawterm (Phase 19). The old-generation local `proc.rs`/`syscall.rs`/`trap.rs`/`vsvm.rs` in `x86_64/src` are superseded by the `port/`-based system as the restructure proceeds — their retirement sequencing is planner discretion, not a deliverable.

</domain>

<decisions>
## Implementation Decisions

**Design theme (user's consistent through-line — honor downstream):** modern-only, KVM-native, no legacy compatibility. Every fork below resolved to the modern option, twice deliberately past the agent's recommendation (D-02 mb2, D-05 TSC-deadline).

### Machine + gate shape
- **D-01:** **qemu machine = q35**; gate shape = **KVM-native default** (`-accel kvm -cpu host`) with TCG + a generic CPU model as the documented fallback (X64-05), `-smp 1` (SMP stays a Future item), `1024M` memory. Mirrors Phase 3's D-06/D-07 accel-chain shape and the platforms-as-TOML-data doctrine (`[qemu]` section keys in the x86_64 config TOML).
- **D-01a:** qemu invocation stays single-sourced as TOML data in the x86_64 config; the netdev stanza lands in the same idle-unbound-then-bound order Phase 7's D-10 established (each commit stays bootable).

### Boot protocol + platform data
- **D-02:** **Multiboot2 via the GRUB2 chain** (deliberate user override of the mb1 recommendation): qemu's built-in `-kernel` loader speaks mb1 only, so the gate pipeline boots SeaBIOS → GRUB2 → kernel from a **nix-built `grub-mkrescue` ISO**; xtask gains `-cdrom` instead of `-kernel`. Rationale: mb2's tagged protocol, loader-mediated entry matching Harvey's EFI/ISO boot lineage, and qemu-independence (real hardware, other loaders). The protocol decision is recorded against 9k's existing multiboot(1) path — 9k stays mb1; r9 x86_64 takes mb2 as its own recorded contract (X64-01). — **Reversibility:** costly — the supply chain gains the ISO build step and the mb2 tag parser; reverting to mb1 drops the loader but rewrites the entry + info-structure code.
- **D-03:** **Platform data = mb2 tags + ACPI at runtime, no fw_cfg.** Memory map from the mb2 memory tags; PCI window from ACPI MCFG; interrupt routes from ACPI MADT (D-06/D-07). fw_cfg was considered and declined — a qemu-specific side channel is a second protocol to maintain; revisit only if ACPI parsing proves prohibitive (see Deferred).

### Interrupts + timers
- **D-04:** **LAPIC + IOAPIC only — the 8259 PIC is a documented non-goal** (mirrors the GICv2-decision shape; KVM's in-kernel APIC is the accelerated path). virtio INTx rides IOAPIC routes with ISR read-to-ack carried from Phase 7.
- **D-05:** **TSC-deadline LAPIC timer** (deliberate user override of the periodic recommendation): one-shot arming per deadline — maps directly onto the existing one-shot-deadline idiom in `port/` (the NET_DEADLINE/poll_at pattern from Phase 8). Invariant-TSC handling and TCG-fallback behavior are researcher items. — **Reversibility:** costly — the tick source's arming model threads through scheduler wakeup paths.
- **D-06:** **IOAPIC routing overrides read from ACPI MADT at boot** — zero hardcoded GSI mappings; the x86 enforcement of Phase 7's no-hardcoded-IRQs discipline.

### PCI/ECAM
- **D-07:** **PCI config space via the ACPI MCFG-located ECAM/MMCONFIG window — no CF8/CFC port path, no fixed base.** ONE ACPI mini-parser (RSDP → XSDT) serves both MCFG (D-07) and MADT (D-06) — the x86 analog of `port/fdt.rs`. The moved-window perturbation proof re-derived: qemu moves the q35 PCIEXBAR base, MCFG reports the new base, the kernel follows and still binds the device (X64-03).
- **D-08:** **Recursive bus walk from bus 0** reading bridge headers (carries over the aarch64 `pci.rs` ECAM scan shape; q35 root ports make recursion load-bearing, not theoretical).

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- From-scratch virtio transport in `port/` (no crate), modern-only negotiation (VERSION_1 required, legacy/transitional rejected loudly), zero-offload mask MAC+STATUS, split virtqueues, ISR read-to-ack (Phase 7 D-01..D-04).
- Data-driven discovery, zero hardcoded addresses/IRQs + perturbation proof (Phase 7 NETDRV-01; D-06/D-07 above apply it to x86).
- Platform-tree structure per the riscv64/aarch64 precedent (Phase 3 D-01): `x86_64/src/platform/<name>/` + `[config] platform` TOML key; platform-as-data for qemu args.
- Gates stay r9-side (`cargo xtask boot-test` profiles, serial markers, negative panic scan); canonical evidence consolidated in Harvey `.planning/phases/13-*/` with the r9 commit SHA recorded; timing-tolerant presence assertions.
- push-before-pin: r9 pushes to `main` of `requaos/r9` before any `flake.lock` advance; the pin advance is its OWN commit + fresh hosted CI, never bundled.
- The `port` crate is shared: aarch64 (virt + raspi), riscv64, and x86_64 targets all stay green — x86_64-only additions are cfg-gated; unsafe concentrated at the MMIO/DMA boundary per `UNSAFE-AUDIT.md`.
- Phase-7 gate payload machinery (baked ARP frame `dd`'d to `/net/ether0/0/data`, filter-dump pcap asserted host-side, ping-pong + TX-bulk reconciliation) is re-keyed, not reinvented — the 60-byte ETHERMINTU padding erratum and slirp-targets-gateway watch-out carry over verbatim.
- qemu-11.1.2 drift accommodations live harness-side, never in vendored/pinned trees (Phase 2 D-12 pattern); x86_64 drift notes land in a companion README alongside `build/aarch64/README.md` if any are found.
- r9 follows its own `rustfmt.toml`; Harvey's Plan 9 C style does not cross the repo boundary; `aarch64` naming discipline unaffected (this phase is `x86_64`).

### Claude's Discretion
- Kernel VA layout / KZERO-analog / linker script + load-address shape for the higher-half x86_64 kernel, and the early pre-MMU debug console mechanics (COM1 16550 already exists in-crate).
- Retirement sequencing of the old-generation `x86_64/src` locals (`proc.rs`, `syscall.rs`, `trap.rs`, `node0.rs`, `vsvm.rs`) as the `port/`-based system lands — keep every commit bootable (execution-gated ordering).
- The mb2 header placement mechanics (spec: 8-byte aligned, within the first 32 KiB of the image) in `l.S`/`kernel.ld`, and which mb2 tags are parsed beyond the memory map.
- The nix `grub-mkrescue` reproducibility mechanics (fixed-output derivation; scrubbed dates/UUIDs) and where the ISO build lives (r9 flake vs xtask).
- Config TOML names (`config_q35.toml`-style), the flake package name (`kernel-q35`/`kernel-x64`-style), and dev-list cfg keying for `dev_ether`/`dev_ip`.
- LAPIC/IOAPIC/TSC driver file layout inside `platform/`; RSDP discovery mechanics (EBDA scan); exact IRQ-accounting invariants for the X64-02 load proof (exact-reconcile for ping-pong, bounds for TX-bulk — Phase 7 D-08 shape).
- Whether the virtio ring memory-ordering override on x86 needs a `RingMem::fence` specialization beyond the core-fence default (Phase 7-03's trait) — researcher verifies x86 ordering semantics against the barrier sandwich.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 13 — goal, success criteria 1–5, requirement mapping
- `.planning/REQUIREMENTS.md` — X64-01..05 full text; Out of Scope table; traceability
- `.planning/PROJECT.md` — Key Decisions (pivot row, three-track north star, r9 adoption), Constraints (host platforms, x86_64 pivot)
- `.planning/STATE.md` — Accumulated Context (all prior decisions), Blockers
- `.planning/HOSTS.md` — the pivot analysis: what carries (port/ arch-neutral), what's re-derived (platform layer, ABI), KVM rationale

### Prior-phase decisions (Harvey repo)
- `.planning/milestones/v1.0-phases/03-r9-qemu-virt-platform-bring-up/03-CONTEXT.md` — THE template for this phase: platform-tree D-01, TOML-as-data D-05..D-07, boot-test harness D-11, panic config D-12, evidence placement D-13
- `.planning/milestones/v2.0-phases/07-r9-pci-ecam-virtio-net-devether/07-CONTEXT.md` — PCI/virtio discipline D-01..D-10, gate payload machinery D-05..D-08, the 60-byte ARP erratum + slirp-gateway watch-out
- `.planning/milestones/v1.0-phases/05-shell-boot-one-command-integration/05-CONTEXT.md` — D-12 (gates stay r9-side), D-13 (CI = linux leg)

### r9 repo (sibling, `../r9` — where all code work happens)
- `../r9/x86_64/src/main.rs` — existing entry `main(mach: &mut Mach, _mbdata: u64)`; the old-generation locals to be superseded
- `../r9/x86_64/src/l.S`, `../r9/x86_64/src/{uart16550.rs,pio.rs,cpu.rs,dat.rs,trap.rs}` — existing x86_64 substrate (16550 + port I/O are keepers)
- `../r9/x86_64/lib/config_default.toml` — the config-TOML entry point D-01 extends
- `../r9/aarch64/src/platform/virt/` — the structural template: `mod.rs` discovery prints, `pci.rs` ECAM scan, `gic/` IRQ registry (the LAPIC/IOAPID analog shape)
- `../r9/port/src/virtio.rs`, `../r9/port/src/devether.rs`, `../r9/port/src/netif.rs` — the arch-neutral transport + ether stack X64-04 re-proves (incl. `RingMem::fence`)
- `../r9/port/src/fdt.rs` — the parser whose ACPI mini-parser analog D-07 builds (heap-less precedent)
- `../r9/xtask/src/main.rs` + `../r9/xtask/src/config.rs` — qemu-as-data machinery, boot-test profiles/markers/pcap asserts, the x86_64 arm that gains `-cdrom` + netdev stanza
- `../r9/flake.nix` — the `packages.kernel-virt` sandboxed-derivation pattern X64-05's x86_64 package mirrors
- `../r9/UNSAFE-AUDIT.md`, `../r9/HowItWorks.md` — unsafe posture; update HowItWorks with material changes per its own header
- `../r9/rust-toolchain.toml` — pinned toolchain (targets already include x86_64?)

### Harvey tree — semantics references
- `sys/src/9k/k10/main.c` — `main(u32int ax, u32int bx)`: the in-tree multiboot(1) entry precedent D-02 records against
- `sys/src/9k/k10/{apic.c,ioapic.c,hpet.c,i8254.c,mp.c}` + `sys/src/9k/k10/l64*.s` — C semantics references for LAPIC/IOAPIC bring-up shapes
- `sys/src/9/pc/ethervirtio.c` + `sys/src/9k/386/ethervirtio.c` — ring-semantics reference ONLY (legacy-transitional; modern-only per Phase 7 D-03)
- `build/aarch64/README.md` — the qemu-drift-notes discipline this phase's x86_64 companion follows
- `flake.nix` — the r9 input contract (`r9.packages.<system>.kernel-virt`) Phase 14's compose extends to the x86_64 package
- `.github/workflows/r9-shell.yml` — the CI shape the KVM-native gates extend

### External specs (researcher pulls exact sections)
- Multiboot 2.0 specification — header layout/tagged info structure (memory tags, alignment requirements, entry state)
- ACPI 6.x — RSDP discovery (EBDA), XSDT walk, MCFG (ECAM base/bus range), MADT (IOAPIC + interrupt-source overrides)
- Intel SDM Vol 3 — LAPIC (SVR/EOI, TSC-deadline MSR 0x832, x2APIC decision NOT taken this phase), IOAPIC (82093AA redirection table)
- Virtio 1.x PCI — carried from 07-CONTEXT (transport already built; only x86 ordering/barrier semantics need verification)
- GRUB2 multiboot2 manual + reproducible `grub-mkrescue` practices under nix

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `port/virtio.rs` + `devether.rs`/`netif.rs` — the whole device stack X64-04 exercises; arch-neutral by construction (Phase 7/8 built it that way)
- `aarch64/src/platform/virt/pci.rs` — ECAM scan + moved-window proof shape to re-derive x86-side
- `xtask` boot-test profile/marker/pcap machinery — every gate in this phase extends it
- `x86_64/src/uart16550.rs` + `pio.rs` — existing keepers (console + port I/O substrate)
- Config-TOML system (`[config]`/`[qemu]`/`[link]` keys) — D-01's data path exists verbatim
- `flake.nix` kernel-virt sandboxed derivation — the X64-05 package template (two-lockfile vendoring precedent)

### Established Patterns
- Platforms-as-data: one config TOML per platform; `--config` selects; new platform = data + a platform module, not Rust edits elsewhere
- Execution-gated ordering: idle-unbound device first (Phase 7 D-10), driver binds in dependency order, every commit bootable
- `cfg(dev_*)` device gating via the config `dev` list — devether/devip join the x86_64 config
- Evidence = serial markers + committed artifacts, timing-tolerant, presence-based; r9 SHA recorded in the Harvey phase dir
- The `port` crate is shared — cfg-gate x86_64-only additions; all sibling targets stay green

### Integration Points
- `../r9/x86_64/src/platform/q35/` (new) — mb2 parse, ACPI mini-parser, LAPIC/IOAPIC, TSC-deadline, ECAM/pci glue
- `../r9/xtask/src/main.rs` — x86_64 QemuStep arm: `-cdrom` ISO, netdev + filter-dump stanza, KVM-default accel chain; boot-test x86_64 profiles
- `../r9/flake.nix` — new x86_64 kernel package + the grub-ISO derivation
- Harvey `flake.nix` — consumes the new package in Phase 14 (not this phase's deliverable beyond the package existing)

</code_context>

<specifics>
## Specific Ideas

- The acceptance story, end-to-end: the q35 config TOML lands (idle netdev stanza, KVM-default accel keys) → early console + mb2 tag parse prints (memory map on serial) → ACPI RSDP→XSDT walk prints the MCFG ECAM base + MADT GSI overrides → LAPIC + IOAPIC live, TSC-deadline tick prints periodic markers → recursive PCI scan finds the virtio-net device (serial: vendor/device/BAR/DID from config space) → the moved-window perturbation run (PCIEXBAR base moved qemu-side; MCFG reports it; the device still binds) → the `port/` virtio transport negotiates VERSION_1+MAC+STATUS (host pins re-run green on x86) → devether `#l` presents `/net/ether0` → the Phase-7 payload gate re-keyed: baked 60-byte ARP `dd`'d to the data file, host-side filter-dump pcap asserts the frame, ping-pong ×N reconciles RX==N/TX==N + IRQ accounting, TX-bulk hunts wedges → the r9 flake ships the x86_64 kernel package + grub ISO; gates run KVM-native on the NixOS host, TCG fallback documented → evidence + r9 SHA under `.planning/phases/13-*/`.
- Watch-out: the virtio barrier sandwich is aarch64-worded (dmb ish/ishld) — x86 ordering rules differ; verify whether `RingMem::fence`'s core-fence default suffices or an x86 override belongs in the trait (researcher item; a wrong guess here is a silent-wedge class, exactly what the Phase-7 host pins exist to catch).
- Watch-out: mb2 header must be 8-byte aligned within the first 32 KiB of the image — a linker/`l.S` detail that fails silently under GRUB if wrong (GRUB just won't recognize the kernel).
- Watch-out: reproducible `grub-mkrescue` — embedded timestamps/UUIDs and the grub module dir make naive ISOs non-deterministic; fixed-output nix derivation with scrubbed inputs (the KERNDATE reproducibility discipline from Phase 1 is the precedent).
- Watch-out: TSC-deadline under TCG — the fallback path must actually tick (researcher verifies; if TCG's TSC-deadline emulation is unreliable, the documented fallback uses a secondary mode, NOT a second architecture).
- Watch-out: SeaBIOS+GRUB adds seconds of wall-clock before the kernel banner — boot-test marker timeouts get tuned (the aarch64 gates' generous-timeout discipline applies).
- Watch-out: Phase 3's unmask-ordering hazard applies verbatim — no IRQ unmask before LAPIC/IOAPIC handlers are registered (any spurious IRQ before dispatch = wedge).
- Watch-out: `x86_64`'s existing `vsvm.rs` is old-generation; its fate (retire vs keep cfg-dead) is planner sequencing, not a deliverable.
- User overrides to honor downstream (deliberate, don't second-guess): Multiboot2 over mb1 (D-02 — tagged protocol, loader-mediated lineage, qemu-independence); TSC-deadline over LAPIC-periodic (D-05 — one-shot idiom fit, modern/KVM-native).

</specifics>

<deferred>
## Deferred Ideas

- fw_cfg platform-data channel — declined in D-03; revisit only if ACPI parsing proves prohibitive
- x2APIC — NOT this phase (classic LAPIC mode; x2APIC is future hardening)
- MSI-X — Out of Scope this milestone (INTx is the one truth)
- 8259 PIC — documented non-goal (D-04)
- SMP (INIT-SIPI-SIPI), virtio-blk/rng/9p, RTC — Future Requirements (REQUIREMENTS.md)
- Harvey-side compose/boot-r9 on x86_64 — Phase 14 (XEXEC) consumes the X64-05 package
- Invariant-TSC calibration strategy + TCG TSC-deadline verification — researcher scope, planner records
- Kernel VA layout + old-crate retirement sequencing — planner discretion per decisions section

</deferred>

---

*Phase: 13-r9 x86_64 Platform Bring-Up*
*Context gathered: 2026-11-06*
