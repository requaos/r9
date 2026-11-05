//! DTB-driven PCI/ECAM discovery for qemu's virt machine (NETDRV-01).
//!
//! Everything here is data, never constants: the ECAM window's base/size
//! come from the pci node's translated `reg`, the device is found by a
//! bus-0 config-space walk, and the interrupt route is decoded from the
//! pci node's `interrupt-map` via the port::fdt helpers.  Absence or
//! malformation of any piece is a loud named print and no bind — there is
//! deliberately NO hardcoded-window fallback (timer.rs's documented-
//! fallback idiom does not apply here: the moved-window perturbation
//! gate's teeth depend on a hardcoded driver failing LOUDLY).

use port::fdt::DeviceTree;
use port::mem::{PhysAddr, PhysRange};

use crate::deviceutil::map_device_register;
use crate::io::read_reg;
use crate::vm;

#[cfg(not(test))]
use port::println;

/// virtio's PCI vendor ID.
const VIRTIO_VENDOR: u32 = 0x1AF4;
/// The modern (non-transitional) virtio-net device ID (0x1040 + 1).
const VIRTIO_NET_MODERN: u32 = 0x1041;
/// The legacy transitional virtio-net device ID — rejected loudly (D-03).
const VIRTIO_NET_TRANSITIONAL: u32 = 0x1000;
/// INTA, the pin the standard boot's NIC raises.
const INTA: u8 = 1;

/// One bus's ECAM span: (bus<<20) | (dev<<15) | (fn<<12) | reg.
const BUS_SPAN: u64 = 1 << 20;

/// What DTB-driven discovery learned about the virtio-net NIC — the
/// facts the driver bind (a later plan) consumes.
#[derive(Debug)]
pub struct VirtioNetDiscovery {
    /// ECAM window base/size, translated from the pci node's `reg`.
    pub ecam_base: u64,
    pub ecam_size: u64,
    /// Coordinates of the found device (the walk probes fn 0 only —
    /// qemu virt's devices are all single-function).
    pub bus: u32,
    pub dev: u32,
    pub fun: u32,
    /// PCI revision ID byte (config offset 0x08) — >= 1 is modern.
    pub rev: u8,
    /// The decoded interrupt route: GIC SPI number and its flags cell
    /// (0x04 = level-high on the committed DTBs).
    pub spi: u32,
    pub flags: u32,
}

/// Discover the ECAM window, the virtio-net device, and its interrupt
/// route from the DTB, printing one `pci: ` line per fact.  Returns the
/// discovery on success; on any absence prints a loud named line and
/// returns None (a NIC-less boot stays legal — the gates are what assert
/// presence).
pub fn discover_and_print(dt: &DeviceTree) -> Option<VirtioNetDiscovery> {
    let node = dt
        .find_device_type("pci")
        .next()
        .or_else(|| dt.find_compatible("pci-host-ecam-generic").next());
    let Some(node) = node else {
        println!("pci: no pci node in dtb");
        return None;
    };

    let window = dt
        .property_translated_reg_iter(node)
        .next()
        .and_then(|r| r.regblock())
        .and_then(|r| r.len.map(|len| (r.addr, len)));
    let Some((base, size)) = window else {
        println!("pci: no ecam window in pci node reg");
        return None;
    };
    println!("pci: ecam base {:#x} size {:#x}", base, size);

    // Map bus 0's span only (the research's no-full-window-map guidance:
    // the whole window is 256 MiB; qemu virt puts every device on bus 0).
    let span = size.min(BUS_SPAN) as usize;
    let physrange = PhysRange::with_pa_len(PhysAddr::new(base), span);
    let ecam = match map_device_register("pciecam", physrange, vm::PageSize::Page4K) {
        Ok(vr) => vr,
        Err(msg) => {
            println!("pci: can't map ecam window {:?}", msg);
            return None;
        }
    };

    // Walk bus 0, fn 0: vendor/device at config offset 0x00, revision at
    // 0x08.  Vendor 0xFFFF marks an absent device.  ECAM offsets are
    // bounded by the mapped span (T-07-04).
    let max_dev = ((span >> 15) as u32).min(32);
    let mut found: Option<(u32, u8)> = None;
    for dev in 0..max_dev {
        let cfg_off = (dev as usize) << 15;
        let id = read_reg(&ecam, cfg_off);
        let (vendor, device) = (id & 0xffff, id >> 16);
        if vendor == 0xffff {
            continue;
        }
        let rev = (read_reg(&ecam, cfg_off + 0x08) & 0xff) as u8;
        if vendor != VIRTIO_VENDOR {
            continue;
        }
        if device == VIRTIO_NET_TRANSITIONAL {
            println!("pci: virtio-net {:04x}:{:04x} at 00:{dev:02x}.0 rejected (transitional, modern-only)", vendor, device);
            continue;
        }
        if device == VIRTIO_NET_MODERN && rev == 0 {
            println!("pci: virtio-net {:04x}:{:04x} at 00:{dev:02x}.0 rev {rev:02x} rejected (pre-1.0)", vendor, device);
            continue;
        }
        if device == VIRTIO_NET_MODERN {
            println!("pci: virtio-net {:04x}:{:04x} at 00:{dev:02x}.0 rev {rev:02x}", vendor, device);
            found = Some((dev, rev));
            break;
        }
    }
    let Some((dev, rev)) = found else {
        println!("pci: no virtio-net device found");
        return None;
    };

    let Some(route) = dt.interrupt_map_lookup(node, 0, dev, 0, INTA) else {
        println!("pci: no interrupt route for 00:{dev:02x}.0 inta");
        return None;
    };
    let [itype, spi, flags] = route.parent_irq;
    if itype != 0 {
        println!("pci: irq type {itype} for 00:{dev:02x}.0 inta is not an SPI");
        return None;
    }
    let trigger = if flags & 0x04 != 0 {
        "level-high"
    } else if flags & 0x01 != 0 {
        "edge-rising"
    } else {
        "unknown-trigger"
    };
    // INTID = 32 + SPI (the GIC's SPI bias).
    println!("pci: inta -> spi {spi} intid {} flags {:#x} {}", 32 + spi, flags, trigger);

    Some(VirtioNetDiscovery {
        ecam_base: base,
        ecam_size: size,
        bus: 0,
        dev,
        fun: 0,
        rev,
        spi,
        flags,
    })
}
