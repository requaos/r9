// The qemu q35 platform module (Phase 13): the mb2 firmware-data parser
// (host-testable, the port/fdt.rs discipline), the console glue, and
// the LAPIC registry; the later bring-up layers (acpi, ioapic, timer,
// pci) grow here in execution-gated order.

pub mod devcons;
pub mod lapic;
pub mod mb2;

#[cfg(not(test))]
use port::println;

/// Print mb2-derived machine information: the "what machine am I on"
/// slot q35 occupies (the aarch64 virt/mod.rs print_machine_info
/// doctrine).  One `q35: `-prefixed line per fact, loud-by-absence, no
/// hardcoded tables — everything below decodes from the firmware data
/// GRUB handed us; the gates (not the kernel) assert presence.
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
        // No ACPI tag in the map: the EBDA/ROM scan that finds the RSDP
        // is 13-04's acpi.rs — printed loudly now so the absence is never
        // mistaken for a silent parse failure.
        None => println!("q35: no ACPI RSDP tag in mb2 — EBDA scan lands in 13-04"),
    }
}
