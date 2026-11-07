// IOAPIC for qemu's q35 machine — MADT-driven (D-06: zero hardcoded GSI
// mappings), the gic/ registry contract re-keyed (aarch64
// platform/virt/gic): init reads the IOAPIC base address and GSI base
// from the published MADT facts (13-04's walk; a MADT that carried none
// is a loud named error and NO ioapic — no hardcoded fallback), masks
// every redirection-table entry for a known state, and publishes the
// driver once; enable(gsi, vector, level) programs one RTE in boot
// context only, trigger bits configured while masked and the UNMASK
// write LAST (the Phase-3 ordering rule — an RTE armed before its
// handler exists is a wedge, not a print); the MADT IntSrcOverrides are
// folded so ISA-source GSIs route through the table
// (override_applies).
//
// Register layout: the 82093AA / ACPI-era indirect scheme — IOREGSEL at
// 0x00 selects the register, IOWIN at 0x10 is the 32-bit data window,
// and RTE i occupies register selects 0x10+2i (lo) / 0x11+2i (hi).
// 9k's ioapic.c uses Iowin = 0x04 — the DEPRECATED pre-82093AA variant;
// this driver uses the 0x10 layout, verified at bring-up against the
// MADT-reported address (the live q35 gate boots through it: a wrong
// data window reads all-zero IDs and never arms the NIC line).
//
// The IOAPIC is MMIO-only regardless of the LAPIC's x2APIC mode (it is
// a separate device on the other side of the chipset, not part of the
// LAPIC's register file), map-free over l.S's identity map of the first
// 4 GiB (the lapic.rs discipline).

use core::cell::SyncUnsafeCell;
use core::ptr::{read_volatile, write_volatile};

use super::acpi::{Acpi, MAX_INTSRC_OVERRIDES};

#[cfg(not(test))]
use port::println;

/// IOREGSEL — the indirect register-select window at offset 0x00
/// (82093AA §3.1: a u32 write selects the register the data window
/// addresses).
pub const IOREGSEL: usize = 0x00;

/// IOWIN — the indirect data window at offset 0x10, the 82093AA/ACPI
/// layout.  NOT 9k's deprecated 0x04 variant (see the module header);
/// every register access below goes select-then-window through exactly
/// these two offsets.
pub const IOWIN: usize = 0x10;

/// Register select: the IOAPIC ID (bits 27:24 carry the 4-bit ID).
pub const REG_ID: u8 = 0x00;
/// Register select: the version (bits 7:0 version, bits 23:16 the max
/// redirection entry index — the table's bounds).
pub const REG_VERSION: u8 = 0x01;
/// Register select base of the redirection table: RTE i's lo word at
/// 0x10+2i, its hi word at 0x11+2i.
pub const REG_RTE: u8 = 0x10;

/// RTE lo: delivery mode fixed (bits 10:8 = 0), destination mode
/// physical (bit 11 = 0), polarity active-high (bit 13 = 0 — the ICH9
/// presents the PIRQ lines active-high at the IOAPIC even though PCI
/// INTx# is electrically active-low; Linux's PCI IOAPIC entries use
/// level + active-high).
#[allow(dead_code)] // enable() ORs it — the bind is its first caller the 13-05 dev_ether bind (idle q35 leaves it unlinked)
pub const RTE_TRIGGER_LEVEL: u32 = 1 << 15;
/// RTE lo bit 16: the mask.  Set while the entry is configured, cleared
/// by the final write — the unmask LAST.
pub const RTE_MASKED: u32 = 1 << 16;

/// The register span actually occupied (the select + data windows; the
/// 82093AA reserves a 1 MiB frame but only the first page is
/// implemented) — the lapic.rs offset-bounds discipline.
const IOAPIC_SPAN: usize = 0x1000;

/// The boot identity window's coverage (lapic.rs's bound): init refuses
/// loudly if the MADT-reported base falls outside it.
const BOOT_IDENTITY_LIMIT: usize = 1 << 32;

/// The initialized IOAPIC.  Plain data: every access is a volatile u32
/// write to IOREGSEL then a volatile read/write at IOWIN, so boot
/// context takes no locks (enable is boot-only by contract; nothing
/// here is called from IRQ context).
struct Ioapic {
    base: usize,
    /// The GSI base from the MADT type-1 entry: RTE index = gsi - gsib.
    gsib: u32,
    /// The max redirection-entry index (version register bits 23:16):
    /// valid RTE indices are 0..=nrtes.
    nrtes: u8,
    /// The MADT IntSrcOverrides, folded by override_applies.
    #[allow(dead_code)] // read by override_applies — the bind's routing call (the 13-05 dev_ether bind)
    overrides: [super::acpi::IntSrcOverride; MAX_INTSRC_OVERRIDES],
    #[allow(dead_code)] // as overrides
    noverrides: usize,
}

/// The pure ISA-source fold: an IntSrcOverride with bus 0 and the given
/// source remaps it to its GSI; an unmatched source is identity.
/// Split out so the host tests pin the fold without a device.
#[allow(dead_code)] // kernel callers arrive with the bind (the 13-05 dev_ether bind); tests pin it now
fn overrides_fold(overrides: &[super::acpi::IntSrcOverride], source_irq: u32) -> u32 {
    for s in overrides {
        if s.bus == 0 && u32::from(s.source) == source_irq {
            return s.gsi;
        }
    }
    source_irq
}

/// The pure RTE-index arithmetic: gsi - gsib, inside the table's bounds
/// (None is loud at the caller — a GSI the table cannot reach is a
/// routing error, never an out-of-bounds select).
#[allow(dead_code)] // kernel callers arrive with the bind (the 13-05 dev_ether bind); tests pin it now
fn rte_index_for(gsib: u32, nrtes: u8, gsi: u32) -> Option<u8> {
    if gsi < gsib {
        return None;
    }
    let idx = gsi - gsib;
    if idx <= u32::from(nrtes) {
        Some(idx as u8)
    } else {
        None
    }
}

/// The pure RTE lo composition: vector | fixed delivery | physical
/// destination | active-high | (level|edge) | (masked|unmasked).  The
/// trigger mode lands in the same write as the vector, but the entry
/// only goes live on the final UNMASKED write.
#[allow(dead_code)] // kernel callers arrive with the bind (the 13-05 dev_ether bind); tests pin it now
fn rte_lo(vector: u8, level: bool, masked: bool) -> u32 {
    let mut lo = u32::from(vector);
    if level {
        lo |= RTE_TRIGGER_LEVEL;
    }
    if masked {
        lo |= RTE_MASKED;
    }
    lo
}

impl Ioapic {
    /// Build from the MADT facts: the type-1 entry's base address and
    /// GSI base, plus every IntSrcOverride.  A MADT that carried no
    /// IOAPIC entry is a loud named error — D-06's zero-hardcode
    /// enforcement (there is deliberately no default base).
    fn new(acpi: &Acpi) -> Result<Ioapic, &'static str> {
        let madt = acpi.madt.as_ref().ok_or("madt table absent")?;
        let io = madt.ioapic.ok_or("madt carried no ioapic entry")?;
        if io.addr as usize + IOAPIC_SPAN > BOOT_IDENTITY_LIMIT {
            return Err("ioapic base outside the boot identity window");
        }
        Ok(Ioapic {
            base: io.addr as usize,
            gsib: io.gsib,
            nrtes: 0, // read from the version register below
            overrides: madt.intsrc,
            noverrides: madt.nintsrc,
        })
    }

    fn select(&self, sel: u8) {
        // SAFETY: base was bounds-checked at new() inside the boot
        // identity map; the volatile write is the MMIO floor.
        unsafe { write_volatile((self.base + IOREGSEL) as *mut u32, u32::from(sel)) }
    }

    fn read_reg(&self, sel: u8) -> u32 {
        self.select(sel);
        // SAFETY: the data window is 4 bytes past the select window,
        // inside the same bounds-checked span; the volatile read is the
        // MMIO floor.
        unsafe { read_volatile((self.base + IOWIN) as *const u32) }
    }

    fn write_reg(&self, sel: u8, val: u32) {
        self.select(sel);
        // SAFETY: as read_reg; the volatile write is the MMIO floor.
        unsafe { write_volatile((self.base + IOWIN) as *mut u32, val) }
    }

    /// Fold the MADT IntSrcOverrides: the GSI an ISA-source IRQ routes
    /// to (identity when no override covers it).  The per-override
    /// provenance lines (q35: intsrc ...) are already printed and gated
    /// by 13-04's fact block.
    #[allow(dead_code)] // the bind's ISA-source routing helper (the 13-05 dev_ether bind)
    pub fn override_applies(&self, source_irq: u32) -> u32 {
        let _ = self.noverrides; // the count rides the array; see fold
        overrides_fold(&self.overrides, source_irq)
    }

    #[allow(dead_code)] // enable()'s table bounds (the 13-05 dev_ether bind)
    fn rte_index(&self, gsi: u32) -> Option<u8> {
        rte_index_for(self.gsib, self.nrtes, gsi)
    }
}

/// Boot-context access to the IOAPIC.  Written once at init (inside the
/// arming window, before sti — enable is a boot-context-only call by
/// contract), read lock-free thereafter — the gic/ publish-once
/// discipline (the cell is never written again).
static IOAPIC: SyncUnsafeCell<Option<Ioapic>> = SyncUnsafeCell::new(None);

/// Discover the IOAPIC from the published MADT facts, mask every RTE
/// for a known state, and publish the driver — the gic/ tolerant idiom:
/// any failure prints loudly and leaves the cell empty (enable then
/// no-ops, so a MADT-less boot degrades instead of panicking; the
/// device bind's own absence print is what makes it visible).
pub fn init() {
    let Some(acpi) = super::acpi_facts() else {
        println!("can't initialise ioapic: no acpi facts published");
        return;
    };
    match Ioapic::new(acpi) {
        Ok(mut io) => {
            // The bring-up cross-check print: ID + version + the table
            // bounds straight from the hardware (a wrong IOWIN offset
            // reads zeros here — the 82093AA-vs-9k divergence's trip
            // wire).
            let id = io.read_reg(REG_ID);
            let ver = io.read_reg(REG_VERSION);
            io.nrtes = ((ver >> 16) & 0xFF) as u8;
            println!(
                "ioapic: id {} version {:#x} rtes {} gsib {} (madt {:#x})",
                (id >> 24) & 0xF,
                ver & 0xFF,
                io.nrtes,
                io.gsib,
                io.base
            );
            // Every RTE starts MASKED with destination 0: the known
            // state the enable() unmask-last discipline arms against
            // (firmware may have left entries from its own PIC-era
            // setup; nothing routes until this kernel asks).
            for i in 0..=io.nrtes {
                io.write_reg(REG_RTE + 2 * i, RTE_MASKED);
                io.write_reg(REG_RTE + 2 * i + 1, 0);
            }
            // SAFETY: the publish-once discipline — this write runs at
            // boot inside the arming window (IF=0), before sti; the
            // cell is never written again.
            unsafe { *IOAPIC.get() = Some(io) }
        }
        Err(msg) => println!("can't initialise ioapic: {msg}"),
    }
}

/// Program the GSI's redirection-table entry: vector | fixed delivery |
/// level-or-edge trigger, destination cpu0 (physical, the BSP — the
/// gicv2 ITARGETSR-cpu0 analog).  Boot context ONLY, from the bind
/// inside the pre-sti window: the entry is configured masked, then the
/// UNMASK write lands LAST so the line never goes live half-programmed
/// (gicv2's trigger-before-enable discipline, and the Phase-3 rule that
/// no interrupt may be deliverable before its handler exists — the trap
/// dispatch arm guards on the published vector).
#[allow(dead_code)] // the bind inside the pre-sti window is the first caller the 13-05 dev_ether bind (idle q35 leaves it unlinked)
pub fn enable(gsi: u32, vector: u8, level: bool) {
    with(|io| {
        let Some(idx) = io.rte_index(gsi) else {
            println!(
                "ioapic: gsi {gsi} outside the table (gsib {}, rtes {}) — not armed",
                io.gsib, io.nrtes
            );
            return;
        };
        let sel = REG_RTE + 2 * idx;
        // Destination hi: physical mode, APIC ID 0 (cpu0, the BSP).
        io.write_reg(sel + 1, 0);
        // Config lo, MASKED first — vector + trigger live but the line
        // is down.
        io.write_reg(sel, rte_lo(vector, level, true));
        // The unmask write LAST.
        io.write_reg(sel, rte_lo(vector, level, false));
        println!(
            "ioapic: gsi {gsi} -> vector {vector:#x} ({})",
            if level { "level" } else { "edge" }
        );
    });
}

#[allow(dead_code)] // enable()'s publish-once reader (the 13-05 dev_ether bind)
fn with(f: impl FnOnce(&Ioapic)) {
    // SAFETY: IOAPIC's write-once discipline — published at init inside
    // the arming window (IF=0), read lock-free thereafter.
    if let Some(io) = (unsafe { &*IOAPIC.get() }).as_ref() {
        f(io);
    }
}

#[cfg(test)]
mod tests {
    use super::super::acpi::IntSrcOverride;
    use super::*;

    fn intsrc(source: u8, gsi: u32) -> IntSrcOverride {
        IntSrcOverride { bus: 0, source, gsi, flags: 0 }
    }

    // The MADT override fold, pinned against the live q35 table (13-04's
    // gate facts: 0->2, 5->5, 9->9, 10->10, 11->11): an overridden ISA
    // source routes to its GSI, a self-mapping override and an
    // uncovered source are identity.
    #[test]
    fn override_fold_matches_the_live_q35_table() {
        let table = [
            intsrc(0, 2),
            intsrc(5, 5),
            intsrc(9, 9),
            intsrc(10, 10),
            intsrc(11, 11),
        ];
        assert_eq!(overrides_fold(&table, 0), 2, "irq0 -> gsi 2 (the timer remap)");
        assert_eq!(overrides_fold(&table, 5), 5);
        assert_eq!(overrides_fold(&table, 9), 9);
        assert_eq!(overrides_fold(&table, 10), 10);
        assert_eq!(overrides_fold(&table, 11), 11);
        // uncovered ISA sources and PCI-derived GSIs pass through
        assert_eq!(overrides_fold(&table, 4), 4);
        assert_eq!(overrides_fold(&table, 22), 22);
        assert_eq!(overrides_fold(&table, 16), 16);
        // non-zero bus entries are not ISA overrides — never folded
        let pci_bus = [IntSrcOverride { bus: 1, source: 0, gsi: 9, flags: 0 }];
        assert_eq!(overrides_fold(&pci_bus, 0), 0);
    }

    // The RTE lo composition, byte-for-byte against the 82093AA field
    // map: vector in 7:0, fixed delivery and physical/active-high
    // defaults zero, level trigger bit 15, mask bit 16.
    #[test]
    fn rte_lo_composition_matches_the_82093aa_field_map() {
        assert_eq!(rte_lo(0x30, true, false), 0x0000_8030);
        assert_eq!(rte_lo(0x30, true, true), 0x0001_8030);
        assert_eq!(rte_lo(0x30, false, false), 0x0000_0030);
        assert_eq!(rte_lo(0x21, false, true), 0x0001_0021);
        assert_eq!(rte_lo(0xFF, true, false), 0x0000_80FF);
        // vector bits never leak past bit 7 into the delivery-mode field
        assert_eq!(rte_lo(0x37, true, false) & 0x1FF00, 0x08000);
    }

    // The GSI -> RTE index arithmetic and bounds: gsib 0 with the q35
    // 24-entry table (max index 23) covers the live GSI 22 route; a
    // non-zero gsib shifts the window; out-of-range GSIs (both sides)
    // are None — a loud refusal, never an out-of-bounds select.
    #[test]
    fn rte_index_covers_the_live_route_and_refuses_out_of_range() {
        assert_eq!(rte_index_for(0, 23, 0), Some(0));
        assert_eq!(rte_index_for(0, 23, 22), Some(22), "the live NIC route, gsi 22");
        assert_eq!(rte_index_for(0, 23, 23), Some(23), "max index is inclusive");
        assert_eq!(rte_index_for(0, 23, 24), None);
        assert_eq!(rte_index_for(16, 23, 16), Some(0));
        assert_eq!(rte_index_for(16, 23, 15), None, "below the GSI base");
    }
}
