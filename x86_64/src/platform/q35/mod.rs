// The qemu q35 platform module (Phase 13): the mb2 firmware-data parser
// lands first (host-testable, the port/fdt.rs discipline); devcons console
// glue, print_machine_info, and the later bring-up layers (acpi, lapic,
// ioapic, timer, pci) grow here in execution-gated order.

pub mod mb2;
