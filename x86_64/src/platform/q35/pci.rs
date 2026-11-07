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
static VIRTIO_VECTOR: SyncUnsafeCell<Option<u8>> = SyncUnsafeCell::new(None);

/// The bound virtio-net device's IDT vector, if a bind published one
/// (None until the bind ran — the dispatch arm never matches then).
pub fn virtio_irq_vector() -> Option<u8> {
    // SAFETY: publish-once — written by the bind in boot context (IF=0,
    // before sti), read lock-free from the trap dispatch thereafter.
    *unsafe { &*VIRTIO_VECTOR.get() }
}

/// The virtio-net IRQ handler: the read-to-ack-first drain (the ISR
/// byte read deasserts the level-triggered line BEFORE the dispatcher's
/// LAPIC EOI — Pitfall 5's storm class), bounded with re-check, no
/// locks/no console on the hot path.  Unreachable until a bind
/// publishes the vector (the dispatch arm guards on it) — the drain
/// body lands with the bind (13-05 Task 2, same plan).
pub fn virtio_net_irq() {}

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
    unsafe { read_volatile(addr as usize as *const u32) }
}

fn ecam_read16(base: u64, bus: u8, dev: u8, fun: u8, off: u8) -> u16 {
    let addr = ecam_addr(base, bus, dev, fun, off);
    debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: as ecam_read32; the 16-bit read is width-matched to the
    // vendor/device field.
    unsafe { read_volatile(addr as usize as *const u16) }
}

fn ecam_read8(base: u64, bus: u8, dev: u8, fun: u8, off: u8) -> u8 {
    let addr = ecam_addr(base, bus, dev, fun, off);
    debug_assert!(addr < BOOT_IDENTITY_LIMIT);
    // SAFETY: as ecam_read32; the byte read is width-matched to the
    // revision/pin fields.
    unsafe { read_volatile(addr as usize as *const u8) }
}

/// A plain identity-window MMIO read outside ECAM (the RCBA IRR
/// registers live in the ICH9's fixed window, not config space).
fn mmio_read16(addr: u64) -> u16 {
    debug_assert!(addr + 2 <= BOOT_IDENTITY_LIMIT);
    // SAFETY: callers pass addresses inside the boot identity map (the
    // RCBA base is read from the LPC's config space and masked); the
    // volatile read is the MMIO floor.
    unsafe { read_volatile(addr as usize as *const u16) }
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
}
