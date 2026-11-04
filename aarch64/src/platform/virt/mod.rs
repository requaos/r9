// qemu `-M virt` platform (gic-version=2).

pub mod devcons;
pub mod gic;
pub mod pl011;
pub mod timer;

use port::fdt::DeviceTree;
#[cfg(not(test))]
use port::println;

/// Print DTB-derived machine information: the "what machine am I
/// on" slot virt occupies instead of raspi's mailbox-driven print_board_info.
/// Discovery flows through port::fdt -- no hardcoded virt address tables.
pub fn print_machine_info(dt: &DeviceTree) {
    println!("Machine information:");
    if let Some(mem) = dt
        .find_device_type("memory")
        .flat_map(|m| dt.property_translated_reg_iter(m).flat_map(|r| r.regblock()))
        .next()
    {
        println!("  Memory:\t{:#x} .. +{:#x}", mem.addr, mem.len.unwrap_or(0));
    }
    // qemu 11.1.2's gicv2 machine reports the GIC as "arm,cortex-a15-gic"
    // (the "arm,gic-400" entry legitimately finds nothing there -- both are
    // probed).
    for compat in ["arm,pl011", "arm,cortex-a15-gic", "arm,gic-400", "arm,armv8-timer"] {
        if dt.find_compatible(compat).next().is_some() {
            println!("  found: {compat}");
        }
    }
}
