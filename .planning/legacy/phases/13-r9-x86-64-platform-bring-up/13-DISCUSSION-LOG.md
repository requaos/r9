# Phase 13: r9 x86_64 Platform Bring-Up - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-06
**Phase:** 13-r9 x86_64 Platform Bring-Up
**Areas discussed:** Machine + accel shape, Boot protocol, Interrupts + timers, PCI/ECAM discovery

---

## Machine + accel shape

| Option | Description | Selected |
|--------|-------------|----------|
| q35 (Recommended) | Modern chipset: PCIe-native ECAM/MMCONFIG via ACPI MCFG, natural home for modern-only virtio 1.0, honest analog of the DTB-driven ECAM discipline | ✓ |
| pc (i440FX) | Legacy port-CF8/CFC PCI; ancient; cuts against the modern-only ethos locked in Phase 7 | |
| You decide | Researcher compares against qemu 11.1.2; planner records | |

**User's choice:** q35

| Option | Description | Selected |
|--------|-------------|----------|
| KVM default (Recommended) | `-accel kvm -cpu host` default (X64-05), TCG + generic model fallback documented, smp 1, 1024M — mirrors Phase-3 D-06/D-07 chain shape | ✓ |
| TCG default | CI parity with the aarch64 era; ignores the pivot's strongest argument (native-speed gates) | |
| You decide | Planner/researcher picks exact -cpu models after probing the NixOS host | |

**User's choice:** KVM default
**Notes:** Both answers followed the recommendation.

---

## Boot protocol

| Option | Description | Selected |
|--------|-------------|----------|
| Multiboot1 (Recommended) | qemu -kernel loads ELF as mb1 — zero host machinery; crate entry already takes mbdata; 9k k10 in-tree precedent | |
| Multiboot2 | Tagged info structure, sturdier protocol, EFI/loader-independent future — but new protocol work this phase | ✓ |
| Linux protocol | Wrong shape for ELF kernels; real-mode trampolines nobody wants | |
| You decide | Researcher verifies qemu 11.1.2 ELF handling | |

**User's choice:** Multiboot2
**Notes:** **Deliberate user override of the mb1 recommendation** — the modern/loader-mediated pick.

Follow-up (mechanism consequence — qemu's -kernel loader speaks mb1 only):

| Option | Description | Selected |
|--------|-------------|----------|
| GRUB2 chain (Recommended) | nix-built grub-mkrescue ISO, SeaBIOS→GRUB2→kernel; xtask gains -cdrom; Harvey's loader-mediated boot lineage | ✓ |
| Back to mb1 | Zero-loader ELF path the aarch64 gates enjoy | |
| Verify first | Researcher verifies qemu 11.1.2 mb2 handling; planner picks | |

**User's choice:** GRUB2 chain

| Option | Description | Selected |
|--------|-------------|----------|
| mbdata + ACPI (Recommended) | mb2 tags for the memory map; everything else discovered from ACPI at runtime — mirrors aarch64's DTB handoff | ✓ |
| fw_cfg channel | qemu-specific side channel; second protocol to maintain | |
| You decide | Planner weighs after researcher scopes the ACPI parser | |

**User's choice:** mbdata + ACPI
**Notes:** fw_cfg noted as available-if-needed (deferred, not chosen).

---

## Interrupts + timers

| Option | Description | Selected |
|--------|-------------|----------|
| LAPIC+IOAPIC (Recommended) | Modern GICv2 analog; KVM in-kernel APIC is the accelerated path; virtio INTx rides IOAPIC | ✓ |
| PIC first | 8259 legacy then upgrade — two interrupt stacks, exactly what modern-only avoids | |
| You decide | Planner scopes under KVM vs TCG | |

**User's choice:** LAPIC+IOAPIC

| Option | Description | Selected |
|--------|-------------|----------|
| LAPIC periodic (Recommended) | 1 ms periodic mirrors the existing tick/scheduler shape; smallest carried-code diff | |
| TSC-deadline | Modern, KVM-efficient, one-shot arming — different arming model; better as later optimization | ✓ |
| HPET/PIT primary | System-wide timers with qemu/KVM quirks; heavier than needed | |
| You decide | Researcher compares under KVM on the NixOS host | |

**User's choice:** TSC-deadline
**Notes:** **Deliberate user override of the periodic recommendation.** Fits the port/ one-shot-deadline idiom (NET_DEADLINE/poll_at precedent).

| Option | Description | Selected |
|--------|-------------|----------|
| ACPI MADT (Recommended) | Interrupt-source overrides read at boot — zero hardcoded GSI mappings; shares the area-4 ACPI parser | ✓ |
| qemu defaults | Static ISA-to-GSI mapping with documented exception — breaks the no-hardcoded-IRQs discipline | |
| You decide | Planner decides after MADT parser scoping | |

**User's choice:** ACPI MADT

---

## PCI/ECAM discovery

| Option | Description | Selected |
|--------|-------------|----------|
| ACPI MCFG (Recommended) | ECAM window located from MCFG; one RSDP→XSDT mini-parser serves MCFG + MADT; moved-window proof re-derived (PCIEXBAR moves, kernel follows) | ✓ |
| CF8/CFC ports | Legacy port-I/O PCI; doesn't scale past bus 0; second access mechanism | |
| Fixed window | Fastest bring-up; explicitly breaks the no-hardcoded-addresses discipline (X64-03) | |
| You decide | Researcher scopes parser size against fdt.rs precedent | |

**User's choice:** ACPI MCFG

| Option | Description | Selected |
|--------|-------------|----------|
| Recursive (Recommended) | Recursive bus walk from bus 0 reading bridge headers — aarch64 scan shape carried; q35 root ports make it real | ✓ |
| Bus 0 only | Smaller scope; risks gate-red when device placement shifts | |
| You decide | Planner decides scan depth against aarch64 pci.rs precedent | |

**User's choice:** Recursive

---

## Claude's Discretion

Kernel VA/layout + linker shape; early console mechanics; old-crate (`proc/syscall/trap/vsvm/node0`) retirement sequencing; mb2 header placement; grub-ISO reproducibility mechanics; config/package naming; RSDP discovery mechanics; IRQ-accounting invariants; x86 fence specialization question.

## Deferred Ideas

fw_cfg channel (declined, revisit-if); x2APIC (future hardening); MSI-X (milestone Out of Scope); 8259 PIC (non-goal); SMP INIT-SIPI-SIPI, virtio-blk/rng/9p, RTC (Future Requirements); Harvey-side compose (Phase 14); TSC calibration details (researcher).
