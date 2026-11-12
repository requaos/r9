// The qemu q35 platform module (Phase 13): the mb2 firmware-data parser
// and the ACPI static-table mini-parser (both host-testable, the
// port/fdt.rs discipline), the console glue, the LAPIC registry, and the
// MCFG-driven PCI/ECAM discovery; the later bring-up layers (ioapic,
// the virtio bind) grow here in execution-gated order.

pub mod acpi;
pub mod devcons;
pub mod ioapic;
pub mod lapic;
pub mod mb2;
// netgate's pure halves (the fixture frame, the reconcile shapes) are
// host-testable; run() itself is dev_ether-only.
#[cfg(any(test, dev_ether))]
pub mod netgate;
// churngate's pure halves (the slope logic, the marker shapes) are
// host-testable; the driver itself is dev_churn-only.
#[cfg(any(test, dev_churn))]
pub mod churngate;
pub mod pci;
pub mod timer;

use core::cell::SyncUnsafeCell;

#[cfg(not(test))]
use port::println;

/// The walked ACPI facts, published once for the later bring-up layers
/// (13-05's ioapic and virtio bind consume the MADT/MCFG facts without
/// a second walk).  Written in print_machine_info — boot context, long
/// before sti — and never written again.
static ACPI: SyncUnsafeCell<Option<acpi::Acpi>> = SyncUnsafeCell::new(None);

/// The published ACPI facts (the publish-once read): None only before
/// print_machine_info ran — every caller (ioapic init, the 13-05 bind)
/// runs later in boot, inside the arming window.
pub fn acpi_facts() -> Option<&'static acpi::Acpi> {
    // SAFETY: publish-once — written once in print_machine_info (boot
    // context, before the arming window and long before sti) and never
    // written again; this lock-free read runs after, from boot context.
    (unsafe { &*ACPI.get() }).as_ref()
}

/// Print mb2- and ACPI-derived machine information: the "what machine
/// am I on" slot q35 occupies (the aarch64 virt/mod.rs print_machine_info
/// doctrine).  One `q35: `-prefixed line per fact, loud-by-absence, no
/// hardcoded tables — everything below decodes from the firmware data
/// GRUB handed us or the ACPI tables it points at; the gates (not the
/// kernel) assert presence.
pub fn print_machine_info(facts: &mb2::Facts) {
    println!("Machine information:");
    for r in facts.ranges() {
        let kind = r.kind.name();
        println!(
            "q35: memory {:#x}..{:#x} ({:#x}) {kind}",
            r.base,
            r.base + r.len,
            r.len
        );
    }
    match facts.acpi_root {
        Some(root) => {
            let kind = if root.xsdt { "xsdt" } else { "rsdt" };
            println!("q35: acpi {kind} {:#x} (mb2 RSDP tag — 13-04 walks it)", root.table);
        }
        // No ACPI tag in the map: the EBDA/ROM scan inside the 13-04
        // walk is the fallback source.
        None => println!("q35: no ACPI RSDP tag in mb2 — acpi walk falls back to the EBDA scan"),
    }

    // The 13-04 ACPI walk: RSDP (mb2 hint first, EBDA/ROM scan as
    // fallback) -> root table -> MCFG/MADT/HPET, every table gated
    // signature + length + checksum before any fact is taken.  The
    // facts below are what 13-05's ioapic consumes; every absence and
    // every walk error prints loudly here.
    let acpi_facts = acpi::discover(&acpi::IdentityMem, facts.acpi_root);
    // SAFETY: publish-once — this write runs in print_machine_info
    // (boot context, before lapic/timer init and long before sti), and
    // the cell is never written again (the gic registry discipline).
    unsafe {
        *ACPI.get() = Some(acpi_facts);
    }
    match acpi_facts.root {
        Some(root) => {
            let kind = if root.xsdt { "xsdt" } else { "rsdt" };
            println!(
                "q35: acpi {kind} {:#x} via {} ({} entries)",
                root.table, root.via, root.nentries
            );
        }
        None => println!("q35: no acpi root walked (see the error lines below)"),
    }
    if let Some(m) = acpi_facts.mcfg {
        println!("q35: mcfg {:#x} buses {:#x}-{:#x} (seg {})", m.base, m.start_bus, m.end_bus, m.seg);
    }
    match &acpi_facts.madt {
        Some(madt) => {
            println!("q35: lapic {:#x} (madt local intc)", madt.local_intc);
            match madt.ioapic {
                Some(io) => println!("q35: ioapic {:#x} gsib {} (id {})", io.addr, io.gsib, io.id),
                None => println!("q35: no ioapic entry in madt"),
            }
            for s in &madt.intsrc[..madt.nintsrc] {
                println!("q35: intsrc bus {} source {} -> gsi {} flags {:#x}", s.bus, s.source, s.gsi, s.flags);
            }
        }
        None => println!("q35: no madt table (ioapic facts absent)"),
    }
    match acpi_facts.hpet {
        Some(h) => println!("q35: hpet {:#x}", h),
        None => println!("q35: no hpet table (timer calibrates without it)"),
    }
    for (label, err) in &acpi_facts.errors[..acpi_facts.nerrors] {
        println!("q35: acpi {label} error: {err:?}");
    }

    // The PCI/ECAM discovery slot: PCIEXBAR programming + the recursive
    // bridge walk + the virtio-net find, all from the MCFG facts above.
    pci::discover_and_print(&acpi_facts);
}
