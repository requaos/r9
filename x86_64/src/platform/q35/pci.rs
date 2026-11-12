//! MCFG-driven PCI/ECAM discovery for qemu's q35 machine (X64-03) — the
//! aarch64 platform/virt/pci.rs doctrine re-keyed: everything is data,
//! never constants.  The ECAM window's base and bus range come from the
//! checksummed ACPI MCFG table (acpi.rs), the kernel PROGRAMS the MCH
//! PCIEXBAR register from that base before the first ECAM access, and
//! the recursive bridge walk (the 9k pcilscan shape) finds the virtio-net
//! device wherever q35's root ports put it.  Absence or malformation of
//! any piece is a loud named print and no discovery — there is
//! deliberately NO hardcoded-window fallback: a compiled-in default base
//! (the SeaBIOS 0xb0000000 class) would silently pass the default boot
//! and lie under the moved-window perturbation gate (13-04 Task 3).
//!
//! The D-07 carve-out, recorded here and in the r9 commit message: the
//! ONE CF8/CFC port access in the kernel is the single helper below,
//! used solely to program the host bridge's PCIEXBAR register (bus 0/
//! dev 0/fn 0, config offset 0x60) before the ECAM window is live at
//! the MCFG base.  Port I/O is never a device-discovery or general
//! config-space path — after this one write, all config access rides
//! ECAM.  Why not write the register through the currently-live ECAM
//! window instead?  Because reaching the MCH config space through the
//! live window requires KNOWING the live window's base — SeaBIOS's
//! hardcoded 0xb0000000 — which is exactly the constant this parser
//! exists to avoid; the architectural CF8/CFC ports address the MCH
//! deterministically.  Removal path once firmware always leaves
//! PCIEXBAR where MCFG says: write config 0x60 through the live window
//! and delete the helper.

use core::cell::SyncUnsafeCell;
use core::ptr::read_volatile;
use crate::proc::kaddr;

#[cfg(dev_ether)]
use core::ptr::write_volatile;
#[cfg(dev_ether)]
use port::virtio::{self, CommonCfg, PciCfg, Ring, RingMem};

use super::acpi::{Acpi, McfgEntry};

#[cfg(not(test))]
use port::println;

use crate::pio;

/// virtio's PCI vendor ID (1af4).
const VIRTIO_VENDOR: u32 = 0x1AF4;
/// The modern (non-transitional) virtio-net device ID (0x1040 + 1,
/// 1af4:1041).
const VIRTIO_NET_MODERN: u32 = 0x1041;
/// The legacy transitional virtio-net device ID — rejected loudly (D-03).
const VIRTIO_NET_TRANSITIONAL: u32 = 0x1000;

/// MCH PCIEXBAR (the pciexbar register) — config offset 0x60, 64-bit:
/// bits 35:28 the 256 MiB
/// window base, bits 3:1 the length (00 = 256 MiB), bit 0 the enable
/// (qemu q35.h's MCH_HOST_BRIDGE_* vocabulary).  Written as
/// `base | LENGTH_256M | PCIEXBAREN`; under the default boot this
/// rewrites SeaBIOS's own value harmlessly, under the moved-MCFG
/// perturbation it follows the table (RESEARCH Pitfall 1).
const MCH_PCIEXBAR: u8 = 0x60;
/// PCIEXBAR length field: 256 MiB (the q35 default window).
const PCIEXBAR_LENGTH_256M: u64 = 0 << 1;
/// PCIEXBAR enable bit.
const PCIEXBAR_ENABLE: u64 = 1;

/// The architectural PCI config mechanism #1 ports — the D-07 carve-out
/// lives between exactly these two.
const PORT_CF8: u16 = 0xCF8;
const PORT_CFC: u16 = 0xCFC;

/// The boot identity window's coverage (lapic.rs's bound): every ECAM
/// address is checked against it before the volatile read.
const BOOT_IDENTITY_LIMIT: u64 = 1 << 32;

/// PCI config-space offsets the walk reads.
const CFG_ID: u8 = 0x00; // vendor lo16 / device hi16
const CFG_REV_CLASS: u8 = 0x08; // rev lo8 / prog-if / subclass / class
const CFG_SECONDARY: u8 = 0x19; // bridge secondary bus
const CFG_SUBORDINATE: u8 = 0x1A; // bridge subordinate bus
const CFG_BAR0: u8 = 0x10; // six BAR dwords through 0x24
const CFG_INTERRUPT_PIN: u8 = 0x3D; // 0 / 1-4 = INTA-D

/// Class 06:04 — a PCI-to-PCI bridge (pcilscan's ccrb/ccru pair).
const CLASS_BRIDGE: u32 = 0x0604;

/// One walked bridge: where it sits and what it claims (the pcilscan
/// facts, printed per bridge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeFact {
    pub bus: u8,
    pub dev: u8,
    pub secondary: u8,
    pub subordinate: u8,
}

/// Headroom over the root-port count a q35 boot carries.
pub const MAX_BRIDGES: usize = 16;

/// Recursion guard: deeper trees than this are malformed, loudly.
const MAX_WALK_DEPTH: usize = 8;

/// What MCFG-driven discovery learned about the virtio-net NIC — the
/// facts 13-05's bind consumes (the aarch64 VirtioNetDiscovery with the
/// x86 route derivation in place of the DTB decode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtioNetDiscovery {
    /// The ECAM window MCFG named (the only source; no fallback).
    pub ecam_base: u64,
    pub bus_start: u8,
    pub bus_end: u8,
    /// Coordinates of the found device (the walk probes fn 0 — the
    /// q35 devices are single-function, the aarch64 arc's shape).
    pub bus: u8,
    pub dev: u8,
    pub fun: u8,
    /// PCI revision ID (config 0x08) — >= 1 is modern.
    pub rev: u8,
    /// The INTx pin (config 0x3D): 1-4 = INTA-D.
    pub pin: u8,
    /// The derived q35 route: PIRQ letter's GSI (16 + PIRQ), from the
    /// swizzle chain up to the root-bus slot.
    pub gsi: u32,
}

/// The published discovery: MCFG facts, the walked bridge topology, and
/// the device facts — written once for 13-05's bind (no second walk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Discovery {
    pub mcfg: McfgEntry,
    pub bridges: [BridgeFact; MAX_BRIDGES],
    pub nbridges: usize,
    pub nic: Option<VirtioNetDiscovery>,
}

/// The publish-once cell: discovery runs once from print_machine_info
/// (boot context, long before sti — discovery is NOT IRQ-driven), and
/// 13-05's bind reads it later in the same boot context.
static DISCOVERY: SyncUnsafeCell<Option<Discovery>> = SyncUnsafeCell::new(None);

/// The IDT vector the bound virtio-net line was armed at — the trap
/// dispatch's guard (zero hardcoded vectors: the discovery cell carries
/// the GSI; the bind derives the vector, arms the IOAPIC RTE at it, and
/// publishes it here).  Written once by the bind inside the pre-sti
/// window, read lock-free from IRQ context thereafter (the aarch64
/// virtio_irq_intid analog).
#[cfg(dev_ether)]
static VIRTIO_VECTOR: SyncUnsafeCell<Option<u8>> = SyncUnsafeCell::new(None);

/// The bound virtio-net device's IDT vector, if a bind published one
/// (None until the bind ran — the dispatch arm never matches then).
#[cfg(dev_ether)]
pub fn virtio_irq_vector() -> Option<u8> {
    // SAFETY: publish-once — written by the bind in boot context (IF=0,
    // before sti), read lock-free from the trap dispatch thereafter.
    *unsafe { &*VIRTIO_VECTOR.get() }
}

// ---------------------------------------------------------------------------
// The D-07 carve-out: the kernel's ONE CF8/CFC access (see the module
// header).  Everything else in this file reaches config space via ECAM.
// ---------------------------------------------------------------------------

/// Read — and optionally write — one 32-bit config register of the MCH
/// host bridge (bus 0, device 0, function 0 — a q35 machine fact, like
/// COM1 for the console) through the architectural CF8/CFC ports,
/// returning the register's value after the operation.  This is the
/// only port-I/O config path in the kernel (host-bridge register
/// programming, never device access).
fn mch_cfg32(offset: u8, write: Option<u32>) -> u32 {
    // Mechanism #1 address: enable bit 31, the MCH's bus/dev/fn are all
    // zero, so only the register offset rides the low byte.
    let addr = 0x8000_0000u32 | ((offset & 0xFC) as u32);
    // SAFETY: ports 0xCF8/0xCFC are the architectural config-mechanism
    // ports, always mapped; pio's asm is a cfg(not(test)) no-op on the
    // host, so the helper is host-compilable without side effects.
    unsafe {
        pio::outl(PORT_CF8, addr);
        if let Some(v) = write {
            pio::outl(PORT_CFC, v);
        }
        pio::inl(PORT_CFC)
    }
}

// ---------------------------------------------------------------------------
// ECAM access: identity-mapped volatile reads over the window MCFG named
// (the lapic.rs read family re-keyed over the ECAM span).
// ---------------------------------------------------------------------------

/// The function's config-space byte address inside the ECAM window:
/// base + (bus<<20 | dev<<15 | fn<<12) + off.  ECAM is memory-mapped
/// config space — byte granularity, NO alignment mask (the & 0xFC that
/// CF8's mechanism #1 genuinely requires lives only in mch_cfg32; an
/// ECAM-side mask silently redirects every sub-dword field read — the
/// device ID at 0x02 re-read the vendor, the pin at 0x3D read the
/// interrupt LINE — and the walk skips the device as unidentified).
fn ecam_addr(base: u64, bus: u8, dev: u8, fun: u8, off: u8) -> u64 {
    base
        + ((bus as u64) << 20)
        + ((dev as u64) << 15)
        + ((fun as u64) << 12)
        + off as u64
}

fn ecam_read32(base: u64, bus: u8, dev: u8, fun: u8, off: u8) -> u32 {
    let addr = ecam_addr(base, bus, dev, fun, off);
    debug_assert!(addr + 4 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: the walk only runs with bus inside the MCFG range and the
    // window inside the boot identity map (both checked in
    // discover_and_print before the walk); a 32-bit read at the computed
    // address stays inside the function's 4 KiB config page.
    unsafe { read_volatile(kaddr(addr) as *const u32) }
}

fn ecam_read16(base: u64, bus: u8, dev: u8, fun: u8, off: u8) -> u16 {
    let addr = ecam_addr(base, bus, dev, fun, off);
    debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: as ecam_read32; the 16-bit read is width-matched to the
    // vendor/device field.
    unsafe { read_volatile(kaddr(addr) as *const u16) }
}

fn ecam_read8(base: u64, bus: u8, dev: u8, fun: u8, off: u8) -> u8 {
    let addr = ecam_addr(base, bus, dev, fun, off);
    debug_assert!(addr < BOOT_IDENTITY_LIMIT);
    // SAFETY: as ecam_read32; the byte read is width-matched to the
    // revision/pin fields.
    unsafe { read_volatile(kaddr(addr) as *const u8) }
}

/// A plain identity-window MMIO read outside ECAM (the RCBA IRR
/// registers live in the ICH9's fixed window, not config space).
fn mmio_read16(addr: u64) -> u16 {
    debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: callers pass addresses inside the boot identity map (the
    // RCBA base is read from the LPC's config space and masked); the
    // volatile read is the MMIO floor.
    unsafe { read_volatile(kaddr(addr) as *const u16) }
}

// ---------------------------------------------------------------------------
// The q35 INTx -> GSI derivation (RESEARCH Pitfall 7, source-verified):
// qemu wires PIRQ[A-H] -> GSI 16+ unconditionally (ich9_pirq_to_gsi),
// and the root bus's default route table maps slot S raising intx i to
// PIRQ (S + i) % 4 + 4 (PIRQ E-H).  A device behind bridges swizzles
// intx = (slot + intx) % 4 per crossing (the PCI standard rotation,
// qemu's pci_swizzle_map_irq_fn), folding up to the root-bus slot.  The
// chipset wiring convention is platform DATA with this printed
// provenance — not a per-device hardcode (the NETDRV-01 doctrine's x86
// enforcement); the RCBA IRR registers cross-check it for the D25-D31
// slots where they exist.
// ---------------------------------------------------------------------------

/// The PIRQ letters, index 0-7 = PIRQ A-H.
const PIRQ_LETTERS: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];

/// The PIRQ->GSI bias (qemu's ich9_pirq_to_gsi: pirq + 16).
const PIRQ_GSI_BASE: u32 = 16;

/// The intx name for a 1-based pin (1-4 -> a-d).
fn intx_name(pin: u8) -> &'static str {
    match pin {
        1 => "inta",
        2 => "intb",
        3 => "intc",
        4 => "intd",
        _ => "intx",
    }
}

/// Fold the swizzle chain: `chain` lists the ancestor bridge slots from
/// the ROOT side down to the device's immediate parent; `dev_slot` is
/// the device's own slot; `pin` its 1-based INTx pin.  Returns
/// (root_slot, pirq) — the q35 root-bus formula applied at the top.
fn root_route(dev_slot: u8, pin: u8, chain: &[u8]) -> (u8, u8) {
    // 0-based intx at the device, swizzled up one crossing per bridge.
    let mut intx = (pin.saturating_sub(1).min(3)) as u32;
    let mut slot = dev_slot;
    // Innermost crossing first: the chain is root-first, so fold in
    // reverse.
    for &ancestor in chain.iter().rev() {
        intx = (slot as u32 + intx) % 4;
        slot = ancestor;
    }
    // The root-bus default table: slot S raising intx i -> PIRQ
    // (S + i) % 4 + 4 (PIRQ E-H), GSI = 16 + PIRQ.
    let pirq = ((slot as u32 + intx) % 4 + 4) as u8;
    (slot, pirq)
}

/// Cross-check the derived route against the ICH9 RCBA IRR registers,
/// which exist for root-bus slots 25-31 (0x3140-family, one 16-bit
/// register per slot): nibbles 3:0 INTA, 7:4 INTB, 11:8 INTC, 15:12
/// INTD, each naming PIRQ A-H (0-7).  Any disagreement prints LOUD
/// (this is the provenance cross-check, not a gate the kernel enforces).
fn rcba_irr_cross_check(ecam_base: u64, root_slot: u8, root_intx: u32, derived_pirq: u8) {
    let slot = root_slot as u32;
    if !(25..=31).contains(&slot) {
        return; // no IRR register exists for this slot
    }
    // The RCBA base lives in the LPC's config 0xF0 (bus 0/dev 31/fn 0),
    // bit 0 enable, bits 31:14 the base — read via ECAM (config-space
    // access through the window; only port I/O is confined by D-07).
    let rcba_reg = ecam_read32(ecam_base, 0, 31, 0, 0xF0);
    if rcba_reg & 1 == 0 {
        println!("pci: rcba disabled — irr cross-check skipped");
        return;
    }
    let irr_addr = ((rcba_reg & 0xFFFF_C000) as u64) + 0x3140 + 2 * (slot as u64 - 25);
    let irr = mmio_read16(irr_addr);
    let nib = ((irr >> (root_intx * 4)) & 0xF) as u8;
    if nib == derived_pirq {
        println!("pci: irr cross-check d{slot}ir {irr:#06x} pirq {} (agrees)", PIRQ_LETTERS[nib as usize % 8]);
    } else {
        println!(
            "pci: irr MISMATCH: derived pirq {} but rcba d{slot}ir {irr:#06x} names pirq {nib}",
            PIRQ_LETTERS[derived_pirq as usize % 8]
        );
    }
}

// ---------------------------------------------------------------------------
// The recursive bus walk (D-08, the pcilscan shape): scan a bus, print
// and record each bridge, recurse into secondary buses; find the
// virtio-net function with the aarch64 reject logic verbatim.
// ---------------------------------------------------------------------------

/// Scan one bus (fn 0 of each device), recursing through bridges.
/// `chain` carries the ancestor bridge slots from the root side (for
/// the INTx swizzle fold).
fn scan_bus(d: &mut Discovery, bus: u8, chain: &[u8], depth: usize) {
    let base = d.mcfg.base;
    for dev in 0..32u8 {
        let vendor = ecam_read16(base, bus, dev, 0, CFG_ID);
        if vendor as u32 == 0xFFFF {
            continue; // absent function
        }
        let device = ecam_read16(base, bus, dev, 0, CFG_ID + 2) as u32;
        let rev_class = ecam_read32(base, bus, dev, 0, CFG_REV_CLASS);
        let class = rev_class >> 16; // prog-if lo8, subclass, class hi8
        if class == CLASS_BRIDGE {
            // The bridge header: secondary at 0x19, subordinate at 0x1A.
            let secondary = ecam_read8(base, bus, dev, 0, CFG_SECONDARY);
            let subordinate = ecam_read8(base, bus, dev, 0, CFG_SUBORDINATE);
            println!(
                "pci: bridge {bus:02x}:{dev:02x}.0 secondary {secondary:#04x} subordinate {subordinate:#04x}"
            );
            if d.nbridges < MAX_BRIDGES {
                d.bridges[d.nbridges] =
                    BridgeFact { bus, dev, secondary, subordinate };
                d.nbridges += 1;
            }
            let in_range = secondary >= d.mcfg.start_bus
                && secondary <= d.mcfg.end_bus
                && secondary != bus;
            if in_range && depth < MAX_WALK_DEPTH {
                let mut deeper = [0u8; MAX_BRIDGES];
                deeper[..chain.len()].copy_from_slice(chain);
                deeper[chain.len()] = dev;
                scan_bus(d, secondary, &deeper[..chain.len() + 1], depth + 1);
            }
        } else if vendor as u32 == VIRTIO_VENDOR {
            let rev = (rev_class & 0xFF) as u8;
            if device == VIRTIO_NET_TRANSITIONAL {
                println!(
                    "pci: virtio-net {:04x}:{device:04x} at {bus:02x}:{dev:02x}.0 rejected (transitional, modern-only)",
                    vendor
                );
                continue;
            }
            if device == VIRTIO_NET_MODERN && rev == 0 {
                println!(
                    "pci: virtio-net {:04x}:{device:04x} at {bus:02x}:{dev:02x}.0 rev {rev:02x} rejected (pre-1.0)",
                    vendor
                );
                continue;
            }
            if device == VIRTIO_NET_MODERN {
                let pin = ecam_read8(base, bus, dev, 0, CFG_INTERRUPT_PIN);
                let mut nbars = 0;
                for bar in 0..6u8 {
                    if ecam_read32(base, bus, dev, 0, CFG_BAR0 + bar * 4) != 0 {
                        nbars += 1;
                    }
                }
                println!(
                    "pci: virtio-net {vendor:04x}:{device:04x} at {bus:02x}:{dev:02x}.0 rev {rev:02x} pin {pin} ({}) bars {nbars}",
                    intx_name(pin)
                );
                // The route derivation: swizzle up the chain, then the
                // q35 root-bus formula; the provenance print names every
                // hop (the aarch64 `inta -> spi N` analog).
                let (root_slot, pirq) = root_route(dev, pin, chain);
                let gsi = PIRQ_GSI_BASE + pirq as u32;
                println!(
                    "pci: {} -> gsi {gsi} (q35 root slot {root_slot}, pirq {}: 16+{pirq})",
                    intx_name(pin),
                    PIRQ_LETTERS[pirq as usize % 8]
                );
                rcba_irr_cross_check(base, root_slot, (pin.saturating_sub(1).min(3)) as u32, pirq);
                d.nic = Some(VirtioNetDiscovery {
                    ecam_base: base,
                    bus_start: d.mcfg.start_bus,
                    bus_end: d.mcfg.end_bus,
                    bus,
                    dev,
                    fun: 0,
                    rev,
                    pin,
                    gsi,
                });
            }
        }
    }
}

/// Discover the ECAM window, the bridge topology, and the virtio-net
/// device from the ACPI MCFG facts, printing one `pci: ` line per fact
/// (the aarch64 discover_and_print arc): program PCIEXBAR from the
/// parsed base with the old->new audit print, walk recursively from bus
/// 0, derive the INTx->GSI route with its provenance print, and publish
/// the discovery once for 13-05's bind.  Absence is loud and means no
/// discovery (the gates assert presence).
pub fn discover_and_print(acpi: &Acpi) {
    let Some(mcfg) = acpi.mcfg else {
        println!("pci: no MCFG table — no ecam window (no fallback)");
        return;
    };
    if mcfg.end_bus < mcfg.start_bus {
        println!("pci: MCFG bus range {:#04x}-{:#04x} inverted", mcfg.start_bus, mcfg.end_bus);
        return;
    }
    let span = ((mcfg.end_bus - mcfg.start_bus) as u64 + 1) << 20;
    if mcfg.base.checked_add(span).is_none_or(|end| end > BOOT_IDENTITY_LIMIT) {
        println!(
            "pci: MCFG window {:#x}+{span:#x} outside the boot identity map",
            mcfg.base
        );
        return;
    }

    // Program the MCH PCIEXBAR from the parsed base — the SeaBIOS
    // hazard step (RESEARCH Pitfall 1): under the default boot this
    // rewrites SeaBIOS's own value harmlessly; under the moved-MCFG
    // perturbation it MOVES the window to where the table says.  The
    // pciexbar old->new print is the audit line every gate log carries.
    let old_lo = mch_cfg32(MCH_PCIEXBAR, None);
    let old_hi = mch_cfg32(MCH_PCIEXBAR + 4, None);
    let old = (old_hi as u64) << 32 | old_lo as u64;
    let new = mcfg.base | PCIEXBAR_LENGTH_256M | PCIEXBAR_ENABLE;
    mch_cfg32(MCH_PCIEXBAR, Some(new as u32));
    mch_cfg32(MCH_PCIEXBAR + 4, Some((new >> 32) as u32));
    println!("pciexbar: {old:#x} -> {new:#x}");

    let mut d = Discovery {
        mcfg,
        bridges: [BridgeFact { bus: 0, dev: 0, secondary: 0, subordinate: 0 }; MAX_BRIDGES],
        nbridges: 0,
        nic: None,
    };
    scan_bus(&mut d, mcfg.start_bus, &[], 0);
    if d.nic.is_none() {
        println!("pci: no virtio-net device found");
    }
    // SAFETY: publish-once — discovery runs once from
    // print_machine_info (boot context, long before sti — discovery is
    // not IRQ-driven), and 13-05's bind reads it later in the same boot
    // context; the cell is never written again (the gic registry
    // discipline).
    unsafe {
        *DISCOVERY.get() = Some(d);
    }
}

// ---------------------------------------------------------------------------
// The virtio-net bind (13-05 Task 2, X64-04) — the aarch64 platform/virt
// bind arc re-keyed over the identity map: the four port seams (PciCfg
// over ECAM, CommonCfg over the assigned BAR, RingMem over the ring
// pages, Uart already done at devcons), BAR sizing + assignment from the
// q35 MMIO window derived from live config, the modern-only negotiation
// with its audit print, the §4.1.5.1.3 queue bring-up, RX pre-post +
// DRIVER_OK, the ISR-read-first IRQ handler, and the driver-ops surface
// devether publishes as /net/ether0.  cfg(dev_ether) per D-01a's
// idle-unbound-then-bound staging; everything the idle build needs stays
// ungated above.
//
// Memory-model note (RESEARCH Pitfall 6's verdict): VolatileMem KEEPS
// the RingMem trait's core-fence default — NO x86 fence specialization.
// core's fence(Release)/fence(Acquire) lower to compiler-only barriers
// on x86 TSO (no mfence), which is correct here because EVENT_IDX (bit
// 29) is never negotiated and the avail no-notify flag is never set, so
// no notify-skip decision depends on the sandwich's one store→load
// adjacency.  The whole file runs through l.S's 1 GiB WB identity window
// (13-03's do-not-resplit finding: a UC split of device pages broke MMIO
// under BOTH accels on this channel; real-hardware hygiene would map the
// notify BAR UC — revisit only if EVENT_IDX ever enters a negotiation
// mask, and then with fence(SeqCst) at the ordering points).
// ---------------------------------------------------------------------------

/// The IDT vector the NIC's line is armed at — the bind's own arming
/// decision, the first spare past the timer in the 0x20..=0x3F IRQ span
/// trap.rs documents.  The dispatch guards on the PUBLISHED value (the
/// VIRTIO_VECTOR cell), never on this constant — zero hardcoded vectors
/// at the dispatch; moving the arming choice is a one-line change with
/// no dispatch edit.
#[cfg(dev_ether)]
const VIRTIO_NET_VECTOR: u8 = 0x21;

/// PCI config-space offsets the bind touches beyond the discovery set.
#[cfg(dev_ether)]
const PCI_OFF_COMMAND: u8 = 0x04;
/// PCI_COMMAND bits: Memory Space (1) and Bus Master (2) — both
/// explicit: qemu's modern virtio path never auto-enables bus-master,
/// and without it DMA silently does nothing.
#[cfg(dev_ether)]
const PCI_COMMAND_MEMORY: u16 = 1 << 1;
#[cfg(dev_ether)]
const PCI_COMMAND_MASTER: u16 = 1 << 2;
/// The first BAR register's config offset (BAR i at 0x10 + 4i — the
/// caps' own bar field picks WHICH one; no hardcoded BAR index).
#[cfg(dev_ether)]
const PCI_OFF_BAR0: u8 = 0x10;

/// One 4 KiB page per RX buffer (12-byte header + 1514 max frame fits
/// trivially).
#[cfg(dev_ether)]
const RX_PAYLOAD_CAP: u32 = 4096;
/// qsize/2 two-descriptor chains — qemu's 256-entry queue posts 128.
#[cfg(dev_ether)]
const MAX_RX_SLOTS: usize = (virtio::MAX_QSIZE / 2) as usize;
/// TX slots mirror RX; each carries its own 4 KiB frame page for the
/// ring's lifetime (devether's write path copies the frame in; the page
/// frees when the slot's used entry retires — no allocation mid-IRQ).
#[cfg(dev_ether)]
const MAX_TX_SLOTS: usize = (virtio::MAX_QSIZE / 2) as usize;

/// Bounded used-ring retire per queue per invocation (T-07-13): at most
/// qsize/2 chains are outstanding, so a quiet queue always drains
/// inside the cap; under a storm the completions arriving mid-drain
/// re-assert the level line after EOI, so the leftover is picked up by
/// the next invocation — a wedge manifests as a gate timeout, never
/// silence.
#[cfg(dev_ether)]
const DRAIN_CAP: u32 = virtio::MAX_QSIZE as u32;

/// The published device state (rings, ISR/notify regions, buffer pages,
/// counters).  Written once at bind; the IRQ handler is its only writer
/// thereafter — single CPU with the whole kernel booting IRQ-masked
/// until sti, and the netgate/tx_post callers run from boot context
/// post-sti where an interrupt CAN interleave... which is exactly why
/// the state's two writer classes stay safe: the IRQ handler retires
/// rings and bumps counters while tx_post only touches the TX ring's
/// avail side through slot ownership (posted flags) that the handler
/// clears under the same single-CPU invariant — mutual exclusion in
/// TIME, no lock (the aarch64 VirtioNetState contract, whose syscall
/// masking becomes the idle-loop shape here until exec arrives).
#[cfg(dev_ether)]
struct VirtioNetState {
    /// The ISR byte's identity address (the isr cap's BAR window) — the
    /// handler's FIRST read (read-to-ack, §4.1.4.5).
    isr: u64,
    /// The device-config region's identity address: the MAC at offset 0,
    /// the link status le16 at offset 6 (§5.1.4).
    devcfg: u64,
    /// The notify window.
    notify: VolatileMem,
    rx: Ring<VolatileMem>,
    tx: Ring<VolatileMem>,
    rx_notify_off: usize,
    /// TX's notify offset — the netgate/tx_post consumers use it; the
    /// retire half never notifies.
    tx_notify_off: usize,
    /// Slot → its RX buffer page's physical address (one page per chain;
    /// PA doubles as VA under the identity map).
    rx_buf_pa: [u64; MAX_RX_SLOTS],
    /// The free stack of RX buffer pages (popped by replenish, pushed by
    /// retire).  Ring/buffer pages are exclusively driver-owned.
    rx_free: [u64; MAX_RX_SLOTS],
    rx_free_len: usize,
    /// Slot → its TX frame page's physical address (one page per chain,
    /// bound at bring-up; the slot's posted flag IS the page's in-use
    /// flag).
    tx_buf_pa: [u64; MAX_TX_SLOTS],
    counters: virtio::VirtioNetCounters,
    /// One-time latch for the config-change print (the one-time-print
    /// IRQ-context carve-out; slirp's link never actually changes).
    link_reported: bool,
    /// Device-fault count (T-07-12 surfaces) — the first is printed.
    faults: u32,
}

/// The published state cell: written once at bind (boot context, IF=0),
/// read lock-free from IRQ context thereafter.
#[cfg(dev_ether)]
static VIRTIO_NET: SyncUnsafeCell<Option<VirtioNetState>> = SyncUnsafeCell::new(None);

// ---------------------------------------------------------------------------
// The seams.  x86_64 runs on l.S's identity map of the first 4 GiB
// (PA==VA), so every "mapping" is an address inside the window that
// new()/discover_and_print already bounds-checked — the aarch64
// VirtRange plumbing collapses to raw addresses and the volatile floor.
// ---------------------------------------------------------------------------

/// Width-matched identity-window MMIO reads/writes (the lapic.rs family,
/// pci.rs edition).
#[cfg(dev_ether)]
fn mmio_read8(addr: u64) -> u8 {
    debug_assert!(addr < BOOT_IDENTITY_LIMIT);
    // SAFETY: callers pass BAR-derived addresses bounds-checked against
    // the BAR extent inside the identity window; volatile is the MMIO
    // floor.
    unsafe { read_volatile(kaddr(addr) as *const u8) }
}

#[cfg(dev_ether)]
fn mmio_read32(addr: u64) -> u32 {
    debug_assert!(addr + 4 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: as mmio_read8; width-matched.
    unsafe { read_volatile(kaddr(addr) as *const u32) }
}

#[cfg(dev_ether)]
fn mmio_write8(addr: u64, v: u8) {
    debug_assert!(addr < BOOT_IDENTITY_LIMIT);
    // SAFETY: as mmio_read8; the volatile write is the MMIO floor.
    unsafe { write_volatile(kaddr(addr) as *mut u8, v) }
}

#[cfg(dev_ether)]
fn mmio_write16(addr: u64, v: u16) {
    debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: as mmio_read8; width-matched.
    unsafe { write_volatile(kaddr(addr) as *mut u16, v) }
}

#[cfg(dev_ether)]
fn mmio_write32(addr: u64, v: u32) {
    debug_assert!(addr + 4 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: as mmio_read8; width-matched.
    unsafe { write_volatile(kaddr(addr) as *mut u32, v) }
}

/// ECAM config-space writes (BAR sizing/assignment, PCI_COMMAND) — the
/// write complements of the ecam_read* family.
#[cfg(dev_ether)]
fn ecam_write32(base: u64, bus: u8, dev: u8, fun: u8, off: u8, v: u32) {
    let addr = ecam_addr(base, bus, dev, fun, off);
    debug_assert!(addr + 4 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: as ecam_read32 — bus inside the MCFG range, window inside
    // the boot identity map (both checked in discover_and_print).
    unsafe { write_volatile(kaddr(addr) as *mut u32, v) }
}

#[cfg(dev_ether)]
fn ecam_write16(base: u64, bus: u8, dev: u8, fun: u8, off: u8, v: u16) {
    let addr = ecam_addr(base, bus, dev, fun, off);
    debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: as ecam_write32; width-matched to the COMMAND field.
    unsafe { write_volatile(kaddr(addr) as *mut u16, v) }
}

/// The PciCfg seam over the device's ECAM config page (the aarch64
/// EcamCfg re-keyed: the function's 4 KiB config space at the ecam
/// address arithmetic, byte-granular).
#[cfg(dev_ether)]
struct EcamDev {
    base: u64,
    bus: u8,
    dev: u8,
    fun: u8,
}

#[cfg(dev_ether)]
impl PciCfg for EcamDev {
    fn len(&self) -> usize {
        4096
    }
    fn read_u8(&self, off: usize) -> u8 {
        ecam_read8(self.base, self.bus, self.dev, self.fun, off as u8)
    }
    fn read_u32(&self, off: usize) -> u32 {
        ecam_read32(self.base, self.bus, self.dev, self.fun, off as u8)
    }
}

/// The CommonCfg seam over the assigned BAR's identity window — the
/// §4.1.3.1 width-matched access discipline (64-bit fields as two
/// 32-bit halves live inside port::virtio).
#[cfg(dev_ether)]
struct Regs {
    base: u64,
}

#[cfg(dev_ether)]
impl CommonCfg for Regs {
    fn read8(&self, off: usize) -> u8 {
        mmio_read8(self.base + off as u64)
    }
    fn read16(&self, off: usize) -> u16 {
        mmio_read16(self.base + off as u64)
    }
    fn read32(&self, off: usize) -> u32 {
        mmio_read32(self.base + off as u64)
    }
    fn write8(&self, off: usize, v: u8) {
        mmio_write8(self.base + off as u64, v)
    }
    fn write16(&self, off: usize, v: u16) {
        mmio_write16(self.base + off as u64, v)
    }
    fn write32(&self, off: usize, v: u32) {
        mmio_write32(self.base + off as u64, v)
    }
}

/// The RingMem seam over an identity-mapped page — the Normal ring pages
/// (pagealloc PA==VA) or the BAR's notify window.  fence() KEEPS the
/// trait's default: the Pitfall-6 verdict — core's fences lower to
/// compiler-only barriers on x86 TSO, correct because EVENT_IDX is never
/// negotiated and the notify-skip decision never rides the sandwich's
/// store→load adjacency (see the module header; the objdump check at
/// Task 2's verify records the lowering — no mfence at the notify site).
#[cfg(dev_ether)]
struct VolatileMem {
    base: usize,
}

#[cfg(dev_ether)]
impl RingMem for VolatileMem {
    fn read_u16(&self, off: usize) -> u16 {
        let addr = self.base + off;
        debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT as usize);
        // SAFETY: ring pages are pagealloc pages (PA==VA inside the
        // identity map) or the bounds-checked notify window; volatile is
        // the DMA-memory floor.
        unsafe { read_volatile(kaddr(addr as u64) as *const u16) }
    }
    fn read_u32(&self, off: usize) -> u32 {
        let addr = self.base + off;
        debug_assert!(addr + 4 <= BOOT_IDENTITY_LIMIT as usize);
        // SAFETY: as read_u16; width-matched.
        unsafe { read_volatile(kaddr(addr as u64) as *const u32) }
    }
    fn read_u64(&self, off: usize) -> u64 {
        let addr = self.base + off;
        debug_assert!(addr as u64 + 8 <= BOOT_IDENTITY_LIMIT);
        // SAFETY: as read_u16; width-matched.
        unsafe { read_volatile(kaddr(addr as u64) as *const u64) }
    }
    fn write_u16(&self, off: usize, v: u16) {
        let addr = self.base + off;
        debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT as usize);
        // SAFETY: as read_u16; the volatile write is the DMA floor.
        unsafe { write_volatile(kaddr(addr as u64) as *mut u16, v) }
    }
    fn write_u32(&self, off: usize, v: u32) {
        let addr = self.base + off;
        debug_assert!(addr + 4 <= BOOT_IDENTITY_LIMIT as usize);
        // SAFETY: as read_u16; width-matched.
        unsafe { write_volatile(kaddr(addr as u64) as *mut u32, v) }
    }
    fn write_u64(&self, off: usize, v: u64) {
        let addr = self.base + off;
        debug_assert!(addr as u64 + 8 <= BOOT_IDENTITY_LIMIT);
        // SAFETY: as read_u16; width-matched.
        unsafe { write_volatile(kaddr(addr as u64) as *mut u64, v) }
    }
}

/// One zeroed 4 KiB pagealloc page — the identity-map shape of the
/// aarch64 alloc_ring_page: no vm mapping needed, the boot PML4s already
/// cover the page (PA doubles as VA).  Returns the physical address.
#[cfg(dev_ether)]
fn alloc_ring_page() -> port::Result<u64> {
    let pa = crate::pagealloc::allocate_physpage().map_err(|_| "virtio: pagealloc exhausted")?;
    let va = pa.addr() as usize;
    // SAFETY: the page is freshly allocated (exclusively ours) inside
    // the identity-mapped first 4 GiB; the zero fill writes exactly the
    // one page (T-07-08 hygiene: no stale data in DMA buffers).
    unsafe { core::ptr::write_bytes(kaddr(va as u64) as *mut u8, 0, 4096) };
    Ok(pa.addr())
}

/// The q35 MMIO window for BAR assignment, derived from live config
/// (never a constant): [ECAM-window end, IOAPIC base) — both endpoints
/// from parsed firmware tables (MCFG's base+bus span, the MADT IOAPIC
/// entry).  This is the aarch64 DTB `ranges` window's x86 analog.
#[cfg(any(test, dev_ether))]
fn mmio_window(mcfg: &McfgEntry, ioapic_base: u64) -> Option<(u64, u64)> {
    let span = ((mcfg.end_bus - mcfg.start_bus) as u64 + 1) << 20;
    let start = mcfg.base.checked_add(span)?;
    if start >= ioapic_base {
        return None;
    }
    Some((start, ioapic_base))
}

/// Decide the BAR's assignment: a firmware-assigned (nonzero) address
/// inside the identity map is KEPT — SeaBIOS already ran PCI resource
/// allocation on this channel, and rewriting its choice could collide
/// with another device's; an unassigned BAR takes the derived window's
/// 4 KiB-aligned base.  Returns (address, firmware_assigned); None when
/// neither fits (a loud bind failure, never a silent dead zone).
/// WR-03 (17-04): the derived base must also be NATURALLY ALIGNED to the
/// BAR size — a base the device's decoded mask cannot address is a dead
/// assignment.  checked_rem: a zero size is a loud None, never a
/// divide-by-zero.
#[cfg(any(test, dev_ether))]
fn bar_assignment(orig: u64, size: u64, win: (u64, u64)) -> Option<(u64, bool)> {
    let fw = orig & !0xF;
    if fw != 0 && fw.checked_add(size).is_some_and(|end| end <= BOOT_IDENTITY_LIMIT) {
        return Some((fw, true));
    }
    let base = win.0 & !0xFFF;
    if base != 0
        && base.checked_rem(size) == Some(0)
        && base.checked_add(size).is_some_and(|end| end <= win.1)
    {
        return Some((base, false));
    }
    None
}

/// The BAR type nibble a 64-bit sizing probe may trust (WR-03, 17-04):
/// bit 0 clear (a memory BAR, not I/O) and bits 2:1 == 0b10 (64-bit,
/// relocatable anywhere) — otherwise the high-dword all-ones write lands
/// in the NEXT BAR's register (or meaningless I/O space) and the
/// computed size is garbage.  Pure: pinned beside the probe.
#[cfg(any(test, dev_ether))]
fn is_64bit_mem_bar_nibble(lo: u32) -> bool {
    lo & 0x1 == 0 && (lo >> 1) & 0b11 == 0b10
}

/// Size a 64-bit BAR: write all-ones to both halves, read the mask back,
/// restore the original contents (T-07-09: the sizing write is always
/// restored).  Returns (size_bytes, original_value); None when the BAR's
/// type nibble is not a 64-bit memory BAR — the probe is never trusted
/// on a BAR it cannot size (WR-03, 17-04).
#[cfg(dev_ether)]
fn size_bar64(base: u64, bus: u8, dev: u8, fun: u8, bar_off: u8) -> Option<(u64, u64)> {
    let orig_lo = ecam_read32(base, bus, dev, fun, bar_off);
    if !is_64bit_mem_bar_nibble(orig_lo) {
        return None;
    }
    let orig_hi = ecam_read32(base, bus, dev, fun, bar_off + 4);
    ecam_write32(base, bus, dev, fun, bar_off, !0);
    ecam_write32(base, bus, dev, fun, bar_off + 4, !0);
    let mask_lo = ecam_read32(base, bus, dev, fun, bar_off);
    let mask_hi = ecam_read32(base, bus, dev, fun, bar_off + 4);
    ecam_write32(base, bus, dev, fun, bar_off, orig_lo);
    ecam_write32(base, bus, dev, fun, bar_off + 4, orig_hi);
    let mask = (u64::from(mask_hi) << 32) | u64::from(mask_lo);
    let orig = (u64::from(orig_hi) << 32) | u64::from(orig_lo);
    let size = (!(mask & !0xF)).wrapping_add(1);
    Some((size, orig))
}

/// WR-01 (17-04): the queue's notify write (a u16 at notify_off within
/// the notify cap) must land inside the cap's declared length — a device
/// advertising a short notify region faults at the first notify.  Pure:
/// pinned beside the bring-up.
#[cfg(any(test, dev_ether))]
fn notify_reachable(notify_off: usize, notify_len: u32) -> bool {
    notify_off + 2 <= notify_len as usize
}

/// One queue's bring-up (§4.1.5.1.3 order): select → read queue_size
/// (accepted as-offered, never downsized; the one-page-per-part bound is
/// checked honestly) → allocate + zero the ring pages → program the
/// three area physical addresses → read the notify offset → queue_enable
/// LAST.
#[cfg(dev_ether)]
fn bring_up_queue(
    common: &Regs,
    queue: u16,
    dir: virtio::Dir,
    notify_mult: u32,
    notify_len: u32,
) -> port::Result<(Ring<VolatileMem>, usize)> {
    let qsize = virtio::queue_size(common, queue);
    if qsize == 0 {
        return Err("virtio: queue absent (queue_size 0)");
    }
    // One 4 KiB page per part: desc 16×qsize, avail 6+2×qsize, used
    // 6+8×qsize (qsize ≤ MAX_QSIZE=256 keeps each within a page — desc
    // fills it exactly at 256).
    let desc_bytes = 16 * qsize as usize;
    let avail_bytes = 6 + 2 * qsize as usize;
    let used_bytes = 6 + 8 * qsize as usize;
    if desc_bytes > 4096 || avail_bytes > 4096 || used_bytes > 4096 {
        return Err("virtio: queue size spills a page (unsupported)");
    }
    let desc_pa = alloc_ring_page()?;
    let avail_pa = alloc_ring_page()?;
    let used_pa = alloc_ring_page()?;
    let hdr_pa = alloc_ring_page()?;
    // The RX used-len bound covers the 12-byte header + one payload page.
    let rx_cap = virtio::VIRTIO_NET_HDR_LEN as u32 + RX_PAYLOAD_CAP;
    let ring = Ring::new(
        VolatileMem { base: desc_pa as usize },
        VolatileMem { base: avail_pa as usize },
        VolatileMem { base: used_pa as usize },
        qsize,
        dir,
        hdr_pa,
        rx_cap,
    )?;
    virtio::set_queue_addresses(common, queue, desc_pa, avail_pa, used_pa);
    let notify_off = usize::from(virtio::queue_notify_off(common, queue)) * notify_mult as usize;
    // WR-01 (17-04): queue_notify_off × multiplier must address a u16
    // INSIDE the notify cap's declared length (bind-time reject, the
    // '(device fault)' family — never a faulting notify write).
    if !notify_reachable(notify_off, notify_len) {
        return Err("virtio: notify offset beyond the notify capability (device fault)");
    }
    virtio::enable_queue(common, queue);
    Ok((ring, notify_off))
}

/// Bind the discovered virtio-net NIC.  Called from main9's pre-sti
/// window under cfg(dev_ether) — after ioapic::init, strictly before
/// sti, so the whole ring setup runs with IRQs still masked and races
/// nothing.  Tolerant-kernel posture: every failure is a loud named
/// print and a skipped bind — the boot continues (the gates are what
/// assert presence).  Zero arguments: everything consumed is published
/// (the discovery cell, the ACPI facts cell).
#[cfg(dev_ether)]
pub fn bind_virtio_net() {
    if let Err(msg) = bind_inner() {
        println!("virtio-net: bind failed: {msg}");
    }
}

/// Each typed cap's (bar, offset, length) bounds-checked against the BAR
/// size BEFORE the region is trusted (T-07-11 — a lying cap aborts the
/// bind loudly).  The 17-04 admission holes (WR-01/WR-02) closed:
/// zero-length extents rejected (a cap at offset == bar_size with
/// length 0 previously passed the `>` check — the empty region faults at
/// the first register touch), minimum extents enforced (common covers
/// the register block through CFG_QUEUE_DEVICE+8; the isr cap's one byte
/// is subsumed by the zero-length reject), and the device config must
/// cover the MAC AND the link-status le16 the IRQ handler reads at
/// devcfg+6 (the config-change arm) — 8, not 6.
///
/// End edge: an extent must fit inside [0, bar_size); offset+length >
/// bar_size is rejected.  An extent ending EXACTLY at the boundary is
/// legal and required: qemu sizes the modern BAR
/// pow2ceil(notify.offset + notify.size) with the notify cap length =
/// notify_off_multiplier * VIRTIO_QUEUE_MAX (4 * 1024 = 0x1000 at
/// offset 0x3000), so the live notify cap ends exactly at the 0x4000
/// boundary — a >=-style edge would reject the real device.  The
/// zero-length reject is what closes the offset == bar_size hole.  The
/// sum is u64-widened: no 32-bit wrap can admit an out-of-bounds cap.
/// Pure: host-testable beside the bind path (17-04).
#[cfg(any(test, dev_ether))]
fn check_cap_extents(caps: &port::virtio::Caps, bar_size: u64) -> port::Result<()> {
    let device = caps.device.ok_or("virtio: no device config capability")?;
    for t in [caps.common, caps.notify, caps.isr, device] {
        if t.length == 0 {
            return Err("virtio: zero-length capability extent (device fault)");
        }
        if u64::from(t.offset) + u64::from(t.length) > bar_size {
            return Err("virtio: capability extent beyond the bar (device fault)");
        }
    }
    if caps.common.length < 0x40 {
        // The register block runs through CFG_QUEUE_DEVICE (0x30) + its
        // u64 — 0x38 touched, 0x40 rounded past the highest offset the
        // transport ever uses (WR-01's prescribed minimum).
        return Err("virtio: common capability smaller than the register block (device fault)");
    }
    if device.length < 8 {
        // MAC (offsets 0..5) + the link-status le16 at +6..8 read by the
        // IRQ handler on the first config-change interrupt
        // (mmio_read16(state.devcfg + 6)) — the old `< 6` admitted a
        // config the handler then over-read (WR-02).
        return Err(
            "virtio: device config capability too small for MAC + link status (device fault)",
        );
    }
    Ok(())
}

#[cfg(dev_ether)]
fn bind_inner() -> port::Result<()> {
    // SAFETY: the discovery cell's publish-once discipline (written once
    // by discover_and_print, earlier in this same boot context).
    let discovery = unsafe { &*DISCOVERY.get() };
    let Some(disc) = discovery.as_ref() else {
        println!("virtio-net: no discovery — bind skipped");
        return Ok(());
    };
    let Some(nic) = disc.nic else {
        println!("virtio-net: no NIC in discovery — bind skipped");
        return Ok(());
    };
    let Some(acpi) = super::acpi_facts() else {
        return Err("no acpi facts for the bind");
    };
    let ioapic_base = acpi
        .madt
        .as_ref()
        .and_then(|m| m.ioapic)
        .map(|io| io.addr)
        .ok_or("no ioapic base for the mmio window")?;

    // The capabilities NAME the BAR they ride (data, never a constant
    // BAR index — the aarch64 code asserted bar==4; q35 takes whatever
    // bar the caps say, requiring only that all four agree).
    let ecam_cfg = EcamDev { base: nic.ecam_base, bus: nic.bus, dev: nic.dev, fun: nic.fun };
    let caps = virtio::Caps::parse(&ecam_cfg)?;
    let bar = caps.common.bar;
    for t in [caps.common, caps.notify, caps.isr] {
        if t.bar != bar {
            return Err("virtio: capabilities split across bars (unsupported)");
        }
    }
    let device = caps.device.ok_or("virtio: no device config capability")?;
    if device.bar != bar {
        return Err("virtio: device capability outside the caps' bar");
    }
    let bar_off = PCI_OFF_BAR0 + 4 * bar;

    // BAR sizing (all-ones mask/restore, T-07-09), then assignment: keep
    // SeaBIOS's assignment when it made one, else the derived window.
    // WR-03 (17-04): the type nibble is validated inside the probe — a
    // 32-bit or I/O BAR can never have its high-dword sizing write
    // trusted.
    let Some((bar_size, orig)) = size_bar64(nic.ecam_base, nic.bus, nic.dev, nic.fun, bar_off)
    else {
        return Err("bar not a 64-bit memory BAR (device fault)");
    };
    if bar_size == 0 || bar_size > (16 << 20) {
        return Err("bar size implausible (device fault)");
    }
    let win = mmio_window(&disc.mcfg, ioapic_base)
        .ok_or("no q35 mmio window between the ecam end and the ioapic")?;
    let Some((bar_pa, fw_assigned)) = bar_assignment(orig, bar_size, win) else {
        return Err("bar assignment outside the mmio window");
    };
    if !fw_assigned {
        ecam_write32(nic.ecam_base, nic.bus, nic.dev, nic.fun, bar_off, bar_pa as u32);
        ecam_write32(nic.ecam_base, nic.bus, nic.dev, nic.fun, bar_off + 4, (bar_pa >> 32) as u32);
    }
    // PCI_COMMAND Memory|Master explicit (qemu's modern path never
    // auto-enables bus-master), with the readback check.
    let cmd = ecam_read16(nic.ecam_base, nic.bus, nic.dev, nic.fun, PCI_OFF_COMMAND);
    ecam_write16(
        nic.ecam_base,
        nic.bus,
        nic.dev,
        nic.fun,
        PCI_OFF_COMMAND,
        cmd | PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER,
    );
    if ecam_read16(nic.ecam_base, nic.bus, nic.dev, nic.fun, PCI_OFF_COMMAND)
        & (PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER)
        != (PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER)
    {
        return Err("pci command readback mismatch");
    }
    let rb = (u64::from(ecam_read32(nic.ecam_base, nic.bus, nic.dev, nic.fun, bar_off + 4)) << 32)
        | (u64::from(ecam_read32(nic.ecam_base, nic.bus, nic.dev, nic.fun, bar_off)) & !0xF);
    println!(
        "pci: bar{bar} pa {bar_pa:#x} size {bar_size:#x}{} readback {rb:#x}",
        if fw_assigned { " (firmware)" } else { " (kernel-assigned)" }
    );
    if rb != bar_pa {
        return Err("bar readback mismatch");
    }
    // Each cap's (bar, offset, length) bounds-checked against the BAR
    // size BEFORE the region is trusted (T-07-11 — a lying cap aborts
    // the bind loudly).
    check_cap_extents(&caps, bar_size)?;
    println!(
        "pci: caps common@{:#x}+{:#x} notify@{:#x} isr@{:#x} device@{:#x} mult {}",
        caps.common.offset,
        caps.common.length,
        caps.notify.offset,
        caps.isr.offset,
        device.offset,
        caps.notify_off_multiplier
    );

    let common = Regs { base: bar_pa + u64::from(caps.common.offset) };
    let isr_pa = bar_pa + u64::from(caps.isr.offset);
    let devcfg_pa = bar_pa + u64::from(device.offset);
    let notify = VolatileMem { base: (bar_pa + u64::from(caps.notify.offset)) as usize };

    // The §3.1.1 machine: reset+read-back wait, ACKNOWLEDGE, DRIVER,
    // two-window feature read, exactly MAC|STATUS|VERSION_1 accepted
    // (the locked D-03 mask), FEATURES_OK with the re-read — a rejecting
    // device is left FAILED and the Err propagates to the loud bind-fail
    // print (a legacy/transitional fallback path is forbidden).
    let negotiated = virtio::negotiate(&common)?;
    // The audit pair: offered AND negotiated (the device_feature
    // registers are RO — the post-negotiation re-read returns the
    // offered windows).
    common.write32(virtio::CFG_DEVICE_FEATURE_SELECT, 0);
    let offered_lo = common.read32(virtio::CFG_DEVICE_FEATURE);
    common.write32(virtio::CFG_DEVICE_FEATURE_SELECT, 1);
    let offered_hi = common.read32(virtio::CFG_DEVICE_FEATURE);
    let offered = (u64::from(offered_hi) << 32) | u64::from(offered_lo);
    let neg = (u64::from(negotiated.hi) << 32) | u64::from(negotiated.lo);
    println!("virtio-net: features offered {offered:#x} negotiated {neg:#x}");
    if !negotiated.has_mac() {
        return Err("virtio: MAC feature not negotiated");
    }

    // The device config's MAC (§5.1.4: 6 bytes at the DEVICE cap's
    // bar+offset+0, valid under the negotiated MAC feature).
    let mut mac = [0u8; 6];
    for (i, b) in mac.iter_mut().enumerate() {
        *b = mmio_read8(devcfg_pa + i as u64);
    }

    // Queues: 0 = RX, 1 = TX (§5.1.2; queue 2 needs CTRL_VQ, never
    // negotiated).  queue_enable is written LAST per queue; DRIVER_OK
    // comes after both, before any notify.
    let (mut rx, rx_notify_off) = bring_up_queue(
        &common,
        virtio::VIRTIO_NET_QUEUE_RX,
        virtio::Dir::Rx,
        caps.notify_off_multiplier,
        caps.notify.length,
    )?;
    let (tx, tx_notify_off) = bring_up_queue(
        &common,
        virtio::VIRTIO_NET_QUEUE_TX,
        virtio::Dir::Tx,
        caps.notify_off_multiplier,
        caps.notify.length,
    )?;
    virtio::set_driver_ok(&common);

    // RX pre-post: every chain gets one zeroed 4 KiB buffer page, then
    // ONE notify — the first notify, after DRIVER_OK (§3.1.1).
    let nslots = (rx.qsize() / 2) as usize;
    let mut rx_free = [0u64; MAX_RX_SLOTS];
    let mut rx_free_len = 0usize;
    for _ in 0..nslots {
        let pa = alloc_ring_page()?;
        rx_free[rx_free_len] = pa;
        rx_free_len += 1;
    }
    let mut rx_buf_pa = [0u64; MAX_RX_SLOTS];
    let outcome = rx.rx_replenish(
        |slot| {
            if rx_free_len == 0 {
                return None;
            }
            rx_free_len -= 1;
            let pa = rx_free[rx_free_len];
            rx_buf_pa[slot as usize] = pa;
            Some((pa, RX_PAYLOAD_CAP))
        },
        &notify,
        rx_notify_off,
        virtio::VIRTIO_NET_QUEUE_RX,
    )?;
    if outcome.offered as usize != nslots {
        return Err("virtio: rx pre-post short");
    }
    println!("virtio-net: rx pre-posted {} buffers qsize {}", outcome.offered, rx.qsize());

    // TX frame pages: one zeroed page per slot, bound for the ring's
    // lifetime (the write path copies a frame into the slot's page and
    // posts; the page is reusable when the slot's used entry retires).
    let ntx = (tx.qsize() / 2) as usize;
    let mut tx_buf_pa = [0u64; MAX_TX_SLOTS];
    for slot in tx_buf_pa.iter_mut().take(ntx) {
        *slot = alloc_ring_page()?;
    }

    let state = VirtioNetState {
        isr: isr_pa,
        devcfg: devcfg_pa,
        notify,
        rx,
        tx,
        rx_notify_off,
        tx_notify_off,
        rx_buf_pa,
        rx_free,
        rx_free_len,
        tx_buf_pa,
        counters: virtio::VirtioNetCounters::default(),
        link_reported: false,
        faults: 0,
    };
    // SAFETY: publish-once — these writes run at bind (boot context,
    // IF=0, before ioapic::enable and before sti), so no IRQ reader can
    // observe a torn write; the cells are never written again (the gic
    // registry discipline).
    unsafe {
        *VIRTIO_NET.get() = Some(state);
        *VIRTIO_VECTOR.get() = Some(VIRTIO_NET_VECTOR);
    }
    // Publish devether's driver seam: #l attaches resolve from here on.
    // Before the IOAPIC arm so no interrupt can precede it.
    port::devether::publish_driver(port::devether::EtherDriverOps {
        mac,
        tx_post: virtnet_tx_post,
        ifstats: virtnet_ifstats,
    });
    super::ioapic::enable(nic.gsi, VIRTIO_NET_VECTOR, true);
    println!(
        "#l0: virtio: 1000Mbps: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// The driver-ops surface (the netgate's and devether's syscall/boot-
// context half) + the IRQ handler.  Registered via
// port::devether::publish_driver at bind.
// ---------------------------------------------------------------------------

/// The TX-post op: copy the frame into the next slot's page and run the
/// §2.7.13 publication sequence.  Ok(false) = the ring is full (the
/// caller waits for a retire — the netgate's bounded backpressure shape;
/// a stalled retire wedges the writer and the gate's deadline catches
/// it, never a silent drop).
#[cfg(dev_ether)]
fn virtnet_tx_post(frame: &[u8]) -> port::Result<bool> {
    // SAFETY: boot/syscall context with the single-CPU invariant — the
    // state's writers are mutually exclusive in TIME (see
    // VirtioNetState's contract); the cell was published once at bind.
    let state = unsafe { &mut *VIRTIO_NET.get() };
    let Some(state) = state.as_mut() else { return Err("devether: virtio-net not bound") };
    let Some(slot) = state.tx.tx_next_slot() else { return Ok(false) };
    let pa = state.tx_buf_pa[slot as usize];
    // SAFETY: the page was allocated (identity-mapped, PA==VA) at bind
    // and stays driver-owned; frame.len() ≤ 1514 < 4096 (the write path
    // bounds-checked it); the slot is free (tx_next_slot), so the page
    // carries no live frame.
    unsafe {
        core::ptr::copy_nonoverlapping(frame.as_ptr(), kaddr(pa) as *mut u8, frame.len())
    };
    let posted = state.tx.tx_post(
        pa,
        frame.len() as u32,
        &state.notify,
        state.tx_notify_off,
        virtio::VIRTIO_NET_QUEUE_TX,
    )?;
    if posted.notified {
        state.counters.nnote[virtio::VIRTIO_NET_QUEUE_TX as usize] += 1;
    }
    Ok(true)
}

/// The ifstats op: the per-queue accounting snapshot (avail, used,
/// lastused, nintr, nnote) for vq0 (RX) and vq1 (TX) — ethervirtio.c's
/// ifstat fields, the X64-02 reconciliation surface.
#[cfg(dev_ether)]
fn virtnet_ifstats() -> [(u16, u16, u16, u32, u32); 2] {
    // SAFETY: as virtnet_tx_post's contract; the used-idx reads are
    // volatile telemetry, not content trust.
    let state = unsafe { &*VIRTIO_NET.get() };
    let Some(state) = state.as_ref() else { return [(0, 0, 0, 0, 0); 2] };
    [
        (
            state.rx.avail_idx(),
            state.rx.used_idx(),
            state.rx.last_used(),
            state.counters.nintr[virtio::VIRTIO_NET_QUEUE_RX as usize],
            state.counters.nnote[virtio::VIRTIO_NET_QUEUE_RX as usize],
        ),
        (
            state.tx.avail_idx(),
            state.tx.used_idx(),
            state.tx.last_used(),
            state.counters.nintr[virtio::VIRTIO_NET_QUEUE_TX as usize],
            state.counters.nnote[virtio::VIRTIO_NET_QUEUE_TX as usize],
        ),
    ]
}

/// The trap.rs dispatch entry for the published vector.  IRQ-context
/// discipline (the aarch64 header, re-keyed for the IOAPIC/LAPIC chain):
///
/// - ISR byte read FIRST (§4.1.4.5): the read resets the ISR to 0 and
///   deasserts the level-triggered line at the source (qemu:
///   qatomic_xchg(&isr,0) + pci_irq_deassert) — the dispatcher's LAPIC
///   EOI then completes the handshake.  An EOI written before this read
///   re-delivers the still-asserted line immediately: the interrupt
///   storm class Pitfall 5 exists to catch (nintr racing past the frame
///   counts is its signature).
/// - Bounded drain with re-check: DRAIN_CAP entries per queue per
///   invocation, the ring's used-idx re-read each iteration (§2.7.14's
///   re-check shape); mid-drain completions re-assert the line after
///   EOI, so a storm spreads across invocations instead of pinning the
///   handler (T-07-13).
/// - No locks, no console on the hot path: the Console mcslock may be
///   held by the interrupted context — taking it here would
///   self-deadlock.  The one-time prints (first device fault, first
///   config change) are the explicit carve-out; slirp's link never
///   changes after DRIVER_OK, so the link line fires exactly once per
///   boot.
/// - Publish-once device state: written once at bind (before
///   ioapic::enable and sti); the handler and the boot-context driver
///   ops are mutually exclusive in TIME on this single-CPU kernel.
/// - A NIC interrupt arriving with no bind cannot match the dispatch
///   guard (the vector cell is empty) and prints `unhandled IRQ` — the
///   negative scan trips on that line, so a bind-ordering bug reds the
///   gate loudly.
#[cfg(dev_ether)]
pub fn virtio_net_irq() {
    // SAFETY: the state was published once at bind (before
    // ioapic::enable and sti); the IDT entry that lands here masks
    // further interrupts, so this is the only writer — the publish-once
    // discipline.
    let state = unsafe { &mut *VIRTIO_NET.get() };
    let Some(state) = state.as_mut() else { return };

    // FIRST device touch: the ISR byte (read-to-ack).
    let isr = mmio_read8(state.isr);

    if isr & virtio::VIRTIO_ISR_QUEUE != 0 {
        let mut retired = [0u32; 2];
        for _ in 0..DRAIN_CAP {
            match state.tx.next_used() {
                None => break,
                Some(Ok(_used)) => {
                    // The slot freed itself inside next_used — its frame
                    // page is bound to the slot and reusable from now.
                    retired[virtio::VIRTIO_NET_QUEUE_TX as usize] += 1;
                }
                Some(Err(msg)) => {
                    state.faults += 1;
                    if state.faults == 1 {
                        println!("virtio-net: device fault: {msg}");
                    }
                    break;
                }
            }
        }
        // A TX retire freed slots — wake any writer blocked on a full
        // ring (the netgate's bounded wait).  The wake rides the
        // port-side entry (never a port lock from IRQ context).
        if retired[virtio::VIRTIO_NET_QUEUE_TX as usize] > 0 {
            port::devether::tx_retired();
        }
        for _ in 0..DRAIN_CAP {
            match state.rx.next_used() {
                None => break,
                Some(Ok(used)) => {
                    retired[virtio::VIRTIO_NET_QUEUE_RX as usize] += 1;
                    // Demux the frame (devether's etheriq): the 12-byte
                    // header is not "stripped" from this buffer — it was
                    // never in it: the device wrote it into the shared
                    // header-page descriptor (§5.1.6.3.2) and the frame
                    // occupies the payload page from offset 0; only
                    // used.len includes the 12 bytes.  The frame lands in
                    // every matching conversation's queue, blocked
                    // readers wake; then the page returns to the free
                    // stack (the ingress copied the bytes out).
                    let frame_len = virtio::virtio_net_frame_len(used.len) as usize;
                    let pa = state.rx_buf_pa[used.slot as usize];
                    // SAFETY: the page is identity-mapped (PA==VA),
                    // driver-owned, and the device finished with it (the
                    // acquire fence inside next_used orders the read);
                    // the base is the page's offset 0 and frame_len
                    // ≤ 4096 (next_used's used-len validation:
                    // 12 ≤ len ≤ 12+4096), so the slice stays within the
                    // page by construction.
                    let frame = unsafe {
                        core::slice::from_raw_parts(
                            (kaddr(pa) + virtio::virtio_net_frame_offset()) as *const u8,
                            frame_len,
                        )
                    };
                    port::devether::rx_ingress(frame);
                    if state.rx_free_len < MAX_RX_SLOTS {
                        state.rx_free[state.rx_free_len] = pa;
                        state.rx_free_len += 1;
                    }
                }
                Some(Err(msg)) => {
                    state.faults += 1;
                    if state.faults == 1 {
                        println!("virtio-net: device fault: {msg}");
                    }
                    break;
                }
            }
        }
        // nintr: one per queue with observed work (ethervirtio.c's shape).
        if retired[virtio::VIRTIO_NET_QUEUE_RX as usize] > 0 {
            state.counters.nintr[virtio::VIRTIO_NET_QUEUE_RX as usize] += 1;
            // The devip seam's RX wake (dev_ip configs only): the waiters
            // drive the stack from their syscall context — a WAKE, never
            // a poll.  x86_64 never sets dev_ip this phase; the cfg keeps
            // the port-side surface identical across arches.
            #[cfg(dev_ip)]
            port::devip::note_rx_pending();
        }
        if retired[virtio::VIRTIO_NET_QUEUE_TX as usize] > 0 {
            state.counters.nintr[virtio::VIRTIO_NET_QUEUE_TX as usize] += 1;
        }
        // Replenish RX (reposting the retired pages), then ONE notify.
        match state.rx.rx_replenish(
            |slot| {
                if state.rx_free_len == 0 {
                    return None;
                }
                state.rx_free_len -= 1;
                let pa = state.rx_free[state.rx_free_len];
                state.rx_buf_pa[slot as usize] = pa;
                Some((pa, RX_PAYLOAD_CAP))
            },
            &state.notify,
            state.rx_notify_off,
            virtio::VIRTIO_NET_QUEUE_RX,
        ) {
            Ok(r) => {
                if r.notified {
                    state.counters.nnote[virtio::VIRTIO_NET_QUEUE_RX as usize] += 1;
                }
            }
            Err(_) => {} // the direction guard — unreachable for an RX ring
        }
    }

    if isr & virtio::VIRTIO_ISR_CONFIG != 0 && !state.link_reported {
        // One-time print (the plan's IRQ-context carve-out): STATUS bit 0
        // is LINK_UP (§5.1.4).  qemu/slirp's link never actually changes
        // after DRIVER_OK, so this line exists for the audit trail, not
        // the hot path.
        state.link_reported = true;
        let status = mmio_read16(state.devcfg + 6);
        println!("virtio-net: link {}", if status & 1 != 0 { "up" } else { "down" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The swizzle fold, pinned against the standard rotation tables:
    // a device directly on the root bus at slot S raising INTA routes
    // to PIRQ (S+0)%4+4; behind one bridge the device's slot joins the
    // rotation first (the qemu pci_swizzle_map_irq_fn shape).
    #[test]
    fn root_route_matches_the_rotation_tables() {
        // Direct root-bus devices: slot 0 INTA -> PIRQ E (gsi 20),
        // slot 1 INTA -> PIRQ F (21), slot 3 INTA -> PIRQ H (23),
        // slot 4 INTA wraps to PIRQ E again.
        assert_eq!(root_route(0, 1, &[]), (0, 4));
        assert_eq!(root_route(1, 1, &[]), (1, 5));
        assert_eq!(root_route(3, 1, &[]), (3, 7));
        assert_eq!(root_route(4, 1, &[]), (4, 4));
        // The pin rotates too: slot 2 INTB -> PIRQ (2+1)%4+4 = G.
        assert_eq!(root_route(2, 2, &[]), (2, 7));

        // Behind one bridge at root slot 6, a device at secondary slot 3
        // raising INTA: the crossing swizzles intx to (3+0)%4 = 3, then
        // the root-bus fold gives PIRQ (6+3)%4+4 = 1+4 = F.
        assert_eq!(root_route(3, 1, &[6]), (6, 5));
        // The classic identity: slot 0 INTA behind a slot-0 bridge stays
        // INTA at PIRQ E.
        assert_eq!(root_route(0, 1, &[0]), (0, 4));
        // Two bridges (chain root-first [5, 9]: root port at slot 5,
        // inner bridge at slot 9): dev slot 1 INTB (intx 1) crosses the
        // inner bridge to (1+1)%4 = 2, the root port to (9+2)%4 = 3,
        // and the root fold lands on PIRQ (5+3)%4+4 = E.
        assert_eq!(root_route(1, 2, &[5, 9]), (5, 4));
    }

    // An out-of-range pin clamps into the table rather than indexing
    // out (a device reporting pin 0 or 5 must not panic the walk).
    #[test]
    fn root_route_clamps_bogus_pins() {
        assert_eq!(root_route(2, 0, &[]), root_route(2, 1, &[]));
        assert_eq!(root_route(2, 9, &[]), root_route(2, 4, &[]));
    }

    // The q35 MMIO window, derived from live config only: the gap
    // between the MCFG ECAM window's end and the MADT IOAPIC base —
    // both endpoints from parsed firmware tables, never a constant
    // (the live values: 0xb0000000 + the 256-bus 256 MiB span =
    // 0xc0000000, up to the IOAPIC at 0xfec00000).
    #[test]
    fn mmio_window_spans_ecam_end_to_ioapic() {
        let mcfg = McfgEntry { base: 0xb000_0000, seg: 0, start_bus: 0, end_bus: 255 };
        assert_eq!(mmio_window(&mcfg, 0xFEC0_0000), Some((0xC000_0000, 0xFEC0_0000)));
        // A moved ECAM window moves the BAR window with it — the same
        // data-following the perturbation gate proves for discovery.
        let moved = McfgEntry { base: 0xe000_0000, seg: 0, start_bus: 0, end_bus: 255 };
        assert_eq!(mmio_window(&moved, 0xFEC0_0000), Some((0xF000_0000, 0xFEC0_0000)));
        // A narrow bus range shrinks the span to its actual size.
        let narrow = McfgEntry { base: 0xb000_0000, seg: 0, start_bus: 0, end_bus: 0 };
        assert_eq!(mmio_window(&narrow, 0xFEC0_0000), Some((0xB010_0000, 0xFEC0_0000)));
        // No room between the ECAM window's end and the IOAPIC: None —
        // a loud bind failure, never a silent overlap.
        let tight = McfgEntry { base: 0xFE00_0000, seg: 0, start_bus: 0, end_bus: 255 };
        assert_eq!(mmio_window(&tight, 0xFEC0_0000), None);
    }

    // The BAR assignment decision: a firmware-assigned (nonzero,
    // identity-mappable) address is kept verbatim (SeaBIOS already ran
    // PCI resource allocation on this channel — rewriting its choice
    // could collide with another device's); an unassigned or unmappable
    // BAR falls back to the derived window's base; a size that fits
    // nowhere is a loud None.
    #[test]
    fn bar_assignment_keeps_firmware_and_falls_back_to_the_window() {
        let win = (0xC000_0000, 0xFEC0_0000);
        // firmware assignments kept (the flags nibble masked off)
        assert_eq!(bar_assignment(0xFD00_0000 | 0x4, 0x4000, win), Some((0xFD00_0000, true)));
        assert_eq!(bar_assignment(0xC000_0000, 0x4000, win), Some((0xC000_0000, true)));
        // unassigned (zero BAR) -> the window base, kernel-assigned
        assert_eq!(bar_assignment(0x0, 0x4000, win), Some((0xC000_0000, false)));
        // a firmware address outside the identity map -> the window
        assert_eq!(
            bar_assignment(0xFFF0_0000_0000_0000 | 0x4, 0x4000, win),
            Some((0xC000_0000, false))
        );
        // a size that fits neither branch -> None (the loud bind error)
        assert_eq!(bar_assignment(0x0, 0x1000_0000, (0xF000_0000, 0xFEC0_0000)), None);
    }

    // ------------------------------------------------------------------
    // 17-04: the cap-walk admission holes (WR-01/WR-02/WR-03)
    // ------------------------------------------------------------------

    // The live q35 virtio-net cap shape, mirrored from caps_fixture and
    // the live bind print: a 0x4000 BAR4 carrying common@0x0+0x1000,
    // isr@0x1000, device@0x2000, notify@0x3000+0x1000 (qemu sizes the
    // modern BAR pow2ceil(notify.offset + notify.size) with
    // notify.size = notify_off_multiplier * VIRTIO_QUEUE_MAX =
    // 4 * 1024 = 0x1000 — the notify cap ends EXACTLY at the bar
    // boundary; see cap_extents_admit_the_live_q35_shape).
    fn live_caps() -> port::virtio::Caps {
        let t = |offset: u32, length: u32| port::virtio::CapTriple { bar: 4, offset, length };
        port::virtio::Caps {
            common: t(0x0000, 0x1000),
            notify: t(0x3000, 0x1000),
            isr: t(0x1000, 0x1000),
            device: Some(t(0x2000, 0x1000)),
            notify_off_multiplier: 4,
        }
    }

    // The live shape binds — the guard against over-tightening the end
    // edge: qemu's notify cap ends exactly AT the bar boundary
    // (0x3000 + 0x1000 == 0x4000), which is legal (its last byte,
    // 0x3FFF, is inside the BAR).  A >=-style end edge would reject the
    // real device and redden every net gate — the edge was re-derived
    // against the live caps per the plan's own over-rejection
    // fails_when.
    #[test]
    fn cap_extents_admit_the_live_q35_shape() {
        check_cap_extents(&live_caps(), 0x4000).expect("the live cap layout binds");
    }

    // WR-01: a zero-length extent is an empty region — the first
    // register touch faults instead of binding.  The exact hole shape
    // (offset == bar_size, length == 0) previously passed the `>` check.
    #[test]
    fn cap_extents_reject_zero_length_caps() {
        let mut caps = live_caps();
        caps.common.length = 0; // at offset 0, inside the bar
        assert_eq!(
            check_cap_extents(&caps, 0x4000).unwrap_err(),
            "virtio: zero-length capability extent (device fault)"
        );
        let mut caps = live_caps();
        caps.isr = port::virtio::CapTriple { bar: 4, offset: 0x4000, length: 0 };
        let err = check_cap_extents(&caps, 0x4000).unwrap_err();
        assert!(err.contains("zero-length"), "the error names the empty extent: {err}");
    }

    // The end edge: an extent hanging past the BAR's last byte is a
    // device fault (the pre-existing check, pinned), and the u64-widened
    // sum means a u32-wrapped offset+length pair can never be admitted.
    #[test]
    fn cap_extents_reject_extents_past_the_bar_end() {
        let mut caps = live_caps();
        caps.device = Some(port::virtio::CapTriple { bar: 4, offset: 0x3ff0, length: 0x1000 });
        let err = check_cap_extents(&caps, 0x4000).unwrap_err();
        assert!(err.contains("beyond the bar"), "the error names the overrun: {err}");
        // offset + length would wrap as u32 (0xffff_ffff + 0xffff_ffff):
        // widened, the sum is ~8.6e9 and must be rejected, never wrapped
        // into admission.
        let mut caps = live_caps();
        caps.notify = port::virtio::CapTriple { bar: 4, offset: u32::MAX, length: u32::MAX };
        let err = check_cap_extents(&caps, 0x4000).unwrap_err();
        assert!(err.contains("beyond the bar"), "no 32-bit wrap admits an OOB cap: {err}");
    }

    // WR-01's minimum extents: the common cap must cover the register
    // block (0x40 — CFG_QUEUE_DEVICE at 0x30 + its u64, rounded); WR-02:
    // the device config must cover the MAC (0..5) AND the link-status
    // le16 at +6..8 the IRQ handler reads on the config-change
    // interrupt — 6 and 7 used to pass the `< 6` check.
    #[test]
    fn cap_extents_reject_undersized_common_and_device_configs() {
        let mut caps = live_caps();
        caps.common.length = 0x3f; // just under the register block
        let err = check_cap_extents(&caps, 0x4000).unwrap_err();
        assert!(err.contains("register block"), "the error names the common minimum: {err}");
        caps.common.length = 0x40; // the minimum itself is legal
        check_cap_extents(&caps, 0x4000).expect("a 0x40 common cap binds");

        for len in [6u32, 7] {
            caps.device = Some(port::virtio::CapTriple { bar: 4, offset: 0x2000, length: len });
            let err = check_cap_extents(&caps, 0x4000).unwrap_err();
            assert!(
                err.contains("MAC + link status"),
                "length {len} admits a config the handler over-reads: {err}"
            );
        }
        caps.device = Some(port::virtio::CapTriple { bar: 4, offset: 0x2000, length: 8 });
        check_cap_extents(&caps, 0x4000).expect("an 8-byte device config binds");
    }

    // WR-03(b): the derived window base must be naturally aligned to the
    // BAR size — a base the device's decoded mask cannot address is a
    // loud None, never a silent dead zone.  The window END check stays
    // (a size that runs past win.1 is still None), and a zero size can
    // never divide (checked_rem, not %).
    #[test]
    fn bar_assignment_rejects_naturally_misaligned_window_bases() {
        // window base 0xC000_1000 against a 0x4000 BAR: fits the window
        // END but is not 0x4000-aligned — the device decodes only
        // 0x4000-aligned addresses inside its BAR.
        assert_eq!(bar_assignment(0x0, 0x4000, (0xC000_1000, 0xFEC0_0000)), None);
        // aligned base still assigned (kernel-assigned arm)
        assert_eq!(
            bar_assignment(0x0, 0x4000, (0xC000_0000, 0xFEC0_0000)),
            Some((0xC000_0000, false))
        );
        // size 0: no modulo, a loud None — never a divide-by-zero panic
        assert_eq!(bar_assignment(0x0, 0, (0xC000_0000, 0xFEC0_0000)), None);
    }

    // WR-03(a): only a 64-bit MEMORY BAR's type nibble may be trusted
    // for the all-ones sizing probe — the live device presents 0b10<<1
    // (bit 0 clear, 64-bit), everything else (I/O bit set, 32-bit, or
    // the reserved/expansable ROM types) makes the high-dword write
    // garbage.
    #[test]
    fn only_64bit_memory_nibbles_are_sizing_trusted() {
        // The type field is bits 2:1 (0b10 = 64-bit) with bit 0 the
        // memory/I/O flag — the live device presents nibble 0b100.
        assert!(is_64bit_mem_bar_nibble(0b100), "the live 64-bit memory nibble");
        assert!(!is_64bit_mem_bar_nibble(0b000), "32-bit memory");
        assert!(!is_64bit_mem_bar_nibble(0b110), "reserved below-ROM type");
        assert!(!is_64bit_mem_bar_nibble(0b101), "I/O BAR (bit 0 set)");
        assert!(!is_64bit_mem_bar_nibble(0), "unprogrammed BAR");
    }

    // WR-01's notify minimum: queue_notify_off × multiplier must address
    // a u16 inside the notify cap's declared length (the live shape:
    // mult 4, queue 0/1 notify_off 0/4 against a 0x1000 cap).
    #[test]
    fn notify_offsets_must_reach_inside_the_notify_cap() {
        assert!(notify_reachable(0, 0x1000), "the live queue-0 shape");
        assert!(notify_reachable(4, 0x1000), "the live queue-1 shape");
        assert!(notify_reachable(0xFFE, 0x1000), "the last u16 that fits");
        assert!(!notify_reachable(0x1000, 0x1000), "a u16 write past the cap end");
        assert!(!notify_reachable(0xFFF, 0x1000), "a straddling write");
    }
}
