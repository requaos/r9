//! DTB-driven PCI/ECAM discovery for qemu's virt machine (NETDRV-01)
//! plus the virtio-net device bring-up and IRQ handler (07-06,
//! NETDRV-02's boot half).
//!
//! Everything here is data, never constants: the ECAM window's base/size
//! come from the pci node's translated `reg`, the device is found by a
//! bus-0 config-space walk, the interrupt route is decoded from the
//! pci node's `interrupt-map` via the port::fdt helpers, and the BAR4
//! assignment window comes from the pci node's `ranges`.  Absence or
//! malformation of any piece is a loud named print and no bind — there is
//! deliberately NO hardcoded-window fallback (timer.rs's documented-
//! fallback idiom does not apply here: the moved-window perturbation
//! gate's teeth depend on a hardcoded driver failing LOUDLY).

use port::fdt::DeviceTree;
use port::mem::{PhysAddr, PhysRange};

use crate::deviceutil::map_device_register;
use crate::io::read_reg;
use crate::vm;

#[cfg(dev_ether)]
use core::cell::SyncUnsafeCell;
#[cfg(dev_ether)]
use port::mem::{VirtRange, PAGE_SIZE_4K};
#[cfg(dev_ether)]
use port::virtio::{self, CommonCfg, PciCfg, Ring, RingMem};
#[cfg(dev_ether)]
use crate::io;
#[cfg(dev_ether)]
use crate::pagealloc;
#[cfg(dev_ether)]
use crate::param::KZERO;
#[cfg(dev_ether)]
use crate::vm::{Entry, RootPageTableType, VaMapping};

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
/// facts the driver bind consumes.
#[derive(Debug, Clone, Copy)]
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

    let discovery =
        VirtioNetDiscovery { ecam_base: base, ecam_size: size, bus: 0, dev, fun: 0, rev, spi, flags };
    // Under dev_ether the bind path consumes this same probe (same ECAM
    // mapping, same facts — no second bus walk) via the publish-once cell.
    // SAFETY: publish-once — discovery runs once from print_machine_info
    // (boot context, long before trap::unmask_irq), and the bind path
    // reads it later in the same boot context; the cell is never written
    // again (the pl011 discipline).
    #[cfg(dev_ether)]
    unsafe {
        *PCI_DISCOVERY.get() = Some((discovery, ecam))
    };
    Some(discovery)
}

// ---------------------------------------------------------------------------
// virtio-net device bring-up (07-06, NETDRV-02's boot half) — BAR
// sizing/assignment, PCI_COMMAND programming, the capability walk over the
// mapped BAR4, the 07-03 negotiation + queue machinery driven against the
// live device, and the ISR read-to-ack IRQ handler.  Compiled only for
// dev_ether configs (D-10's always-on is the end state; discovery above
// stays compiled everywhere).
// ---------------------------------------------------------------------------

/// The 07-02 discovery result + its ECAM mapping, kept for the bind path.
#[cfg(dev_ether)]
static PCI_DISCOVERY: SyncUnsafeCell<Option<(VirtioNetDiscovery, VirtRange)>> =
    SyncUnsafeCell::new(None);

/// The bound device's GIC INTID (0 = unbound).  trap.rs's dispatch guard
/// keys on this — zero hardcoded interrupt numbers (NETDRV-01).
#[cfg(dev_ether)]
static VIRTIO_NET_INTID: SyncUnsafeCell<u32> = SyncUnsafeCell::new(0);

/// The published device state (rings, ISR/notify regions, buffer pages,
/// counters).  Written once at bind; the IRQ handler is its only writer
/// thereafter (single-CPU, non-nesting EL1 IRQ context).
#[cfg(dev_ether)]
static VIRTIO_NET: SyncUnsafeCell<Option<VirtioNetState>> = SyncUnsafeCell::new(None);

// PCI config-space offsets beyond the discovery set.
#[cfg(dev_ether)]
const PCI_OFF_COMMAND: usize = 0x04;
#[cfg(dev_ether)]
const PCI_OFF_BAR4: usize = 0x20;
/// PCI_COMMAND bits: Memory Space (1) and Bus Master (2) — both explicit:
/// qemu's modern virtio path never auto-enables bus-master (the legacy
/// I/O-port status-write hack does not exist on this path), and without
/// it DMA silently does nothing (Pitfall N3's silent dead zone).
#[cfg(dev_ether)]
const PCI_COMMAND_MEMORY: u16 = 1 << 1;
#[cfg(dev_ether)]
const PCI_COMMAND_MASTER: u16 = 1 << 2;

/// One 4 KiB page per RX buffer (12-byte header + 1514 max frame fits
/// trivially; the slab/packing optimization is deferred by the research).
#[cfg(dev_ether)]
const RX_PAYLOAD_CAP: u32 = PAGE_SIZE_4K as u32;
/// qsize/2 two-descriptor chains — qemu's 256-entry queue posts 128.
#[cfg(dev_ether)]
const MAX_RX_SLOTS: usize = (virtio::MAX_QSIZE / 2) as usize;

/// The PciCfg seam over the mapped bus-0 ECAM window: the device's 4 KiB
/// config space sits at base = dev<<15.
#[cfg(dev_ether)]
struct EcamCfg {
    range: VirtRange,
    base: usize,
}

#[cfg(dev_ether)]
impl PciCfg for EcamCfg {
    fn len(&self) -> usize {
        4096
    }
    fn read_u8(&self, off: usize) -> u8 {
        io::read_reg8(&self.range, self.base + off)
    }
    fn read_u32(&self, off: usize) -> u32 {
        io::read_reg(&self.range, self.base + off)
    }
}

/// The CommonCfg seam over a mapped BAR region — the RegBlock idiom
/// (typed-offset volatile access through the io.rs widths) kept local so
/// regblock.rs stays untouched this plan.
#[cfg(dev_ether)]
struct Regs {
    range: VirtRange,
}

#[cfg(dev_ether)]
impl CommonCfg for Regs {
    fn read8(&self, off: usize) -> u8 {
        io::read_reg8(&self.range, off)
    }
    fn read16(&self, off: usize) -> u16 {
        io::read_reg16(&self.range, off)
    }
    fn read32(&self, off: usize) -> u32 {
        io::read_reg(&self.range, off)
    }
    fn write8(&self, off: usize, v: u8) {
        io::write_reg8(&self.range, off, v)
    }
    fn write16(&self, off: usize, v: u16) {
        io::write_reg16(&self.range, off, v)
    }
    fn write32(&self, off: usize, v: u32) {
        io::write_reg(&self.range, off, v)
    }
}

/// The RingMem seam over a mapped region — the Normal ring pages
/// (KZERO+PA) or the BAR's notify window.  fence() keeps the trait's
/// default (core's fence: dmb ish / dmb ishld on aarch64, the
/// RESEARCH-verified codegen).
#[cfg(dev_ether)]
struct VolatileMem {
    range: VirtRange,
}

#[cfg(dev_ether)]
impl RingMem for VolatileMem {
    fn read_u16(&self, off: usize) -> u16 {
        io::read_reg16(&self.range, off)
    }
    fn read_u32(&self, off: usize) -> u32 {
        io::read_reg(&self.range, off)
    }
    fn read_u64(&self, off: usize) -> u64 {
        io::read_reg64(&self.range, off)
    }
    fn write_u16(&self, off: usize, v: u16) {
        io::write_reg16(&self.range, off, v)
    }
    fn write_u32(&self, off: usize, v: u32) {
        io::write_reg(&self.range, off, v)
    }
    fn write_u64(&self, off: usize, v: u64) {
        io::write_reg64(&self.range, off, v)
    }
}

/// The bound device's live state (see VIRTIO_NET's publish-once cell).
#[cfg(dev_ether)]
struct VirtioNetState {
    /// The ISR byte region (the isr cap's BAR4 window).  The handler's
    /// FIRST read is this byte — read-to-ack (§4.1.4.5).
    isr: VirtRange,
    /// The device-config region (the device cap's window): the MAC at
    /// offset 0, the link status le16 at offset 6 (§5.1.4).
    devcfg: VirtRange,
    /// The notify window (the notify cap's BAR4 region).
    notify: VolatileMem,
    rx: Ring<VolatileMem>,
    tx: Ring<VolatileMem>,
    rx_notify_off: usize,
    /// TX's notify offset — consumed by 07-07's data-file TX path
    /// (tx_post); the retire half never notifies.
    #[allow(dead_code)]
    tx_notify_off: usize,
    /// Slot → its RX buffer page's physical address (one page per chain).
    rx_buf_pa: [u64; MAX_RX_SLOTS],
    /// The free stack of RX buffer pages (popped by replenish, pushed by
    /// retire).  Ring/buffer pages are exclusively driver-owned — never
    /// aliased into user mappings (T-07-08's page hygiene).
    rx_free: [u64; MAX_RX_SLOTS],
    rx_free_len: usize,
    counters: virtio::VirtioNetCounters,
    /// The negotiated MAC (the bind line + 07-07's addr file).
    #[allow(dead_code)]
    mac: [u8; 6],
    /// One-time latch for the config-change print (the one-time-print
    /// IRQ-context carve-out; slirp's link never actually changes).
    link_reported: bool,
    /// Device-fault count (T-07-12 surfaces) — the first is printed.
    faults: u32,
}

/// The bound device's GIC INTID, or 0 when unbound.  trap.rs's dispatch
/// guard reads this; the publish-once discipline makes the read lock-free.
#[cfg(dev_ether)]
pub fn virtio_irq_intid() -> u32 {
    // SAFETY: write-once at bind (boot context, before gic::enable and
    // before IRQ unmask); read lock-free thereafter (pl011 discipline).
    unsafe { *VIRTIO_NET_INTID.get() }
}

/// Size a 64-bit BAR: write all-ones to both halves, read the mask back,
/// restore the original contents.  Returns (size_bytes, original_value).
/// The low nibble of the mask is flags (bit 3 prefetchable, bits 2:1
/// type); size = !(mask & !0xF) + 1 (T-07-09: the sizing write is always
/// restored).
#[cfg(dev_ether)]
fn size_bar64(ecam: &VirtRange, cfg: usize, bar_off: usize) -> (u64, u64) {
    let orig_lo = read_reg(ecam, cfg + bar_off);
    let orig_hi = read_reg(ecam, cfg + bar_off + 4);
    io::write_reg(ecam, cfg + bar_off, !0);
    io::write_reg(ecam, cfg + bar_off + 4, !0);
    let mask_lo = read_reg(ecam, cfg + bar_off);
    let mask_hi = read_reg(ecam, cfg + bar_off + 4);
    io::write_reg(ecam, cfg + bar_off, orig_lo);
    io::write_reg(ecam, cfg + bar_off + 4, orig_hi);
    let mask = (u64::from(mask_hi) << 32) | u64::from(mask_lo);
    let orig = (u64::from(orig_hi) << 32) | u64::from(orig_lo);
    let size = (!(mask & !0xF)).wrapping_add(1);
    (size, orig)
}

/// The DTB pci node's 64-bit-MMIO `ranges` window: (parent base, size).
/// Parsed here rather than via fdt::property_range_iter — that helper
/// caps child addresses at 2 cells and the pci node declares 3, so the
/// PCI tuple (3 child + 2 parent + 2 size cells on the committed DTBs)
/// is walked directly from the property bytes.  The 64-bit space is
/// selected by the child phys.hi space bits (bits 25:24 == 0b11).
#[cfg(dev_ether)]
fn mmio64_window(dt: &DeviceTree, node: port::fdt::Node) -> Option<(u64, u64)> {
    let cells_of = |n: &port::fdt::Node, name: &str, default: usize| {
        dt.property(n, name)
            .and_then(|p| dt.property_value_as_u32(&p))
            .map_or(default, |v| v as usize)
    };
    let child_cells = cells_of(&node, "#address-cells", 3);
    let size_cells = cells_of(&node, "#size-cells", 2);
    let parent = dt.parent(&node)?;
    let parent_cells = cells_of(&parent, "#address-cells", 2);
    if child_cells != 3 || !(1..=2).contains(&parent_cells) || !(1..=2).contains(&size_cells) {
        return None;
    }
    let stride = child_cells + parent_cells + size_cells;
    let prop = dt.property(&node, "ranges")?;
    let pack = |cells: &[u32]| cells.iter().fold(0u64, |a, c| (a << 32) | u64::from(*c));
    let mut tuple = [0u32; 7];
    let mut i = 0usize;
    for cell in dt.property_value_as_u32_iter(&prop) {
        if i < tuple.len() {
            tuple[i] = cell;
        }
        i += 1;
        if i == stride {
            i = 0;
            if (tuple[0] >> 24) & 0x3 == 0x3 {
                let base = pack(&tuple[child_cells..child_cells + parent_cells]);
                let size = pack(&tuple[child_cells + parent_cells..stride]);
                return Some((base, size));
            }
        }
    }
    None
}

/// One zeroed 4 KiB pagealloc page mapped Normal at KZERO+PA — the
/// main.rs initrd idiom (Entry::rw_kernel_data, NEVER deviceutil's
/// rw_device: ring/frame memory is Normal cacheable; the DTB's
/// dma-coherent on the pci node makes that correct on qemu virt).
/// Returns (physical address, mapped range).  The mapping is permanent
/// this era (no unmap path), so dropping the returned range handle is
/// safe — the page stays mapped and driver-owned.
#[cfg(dev_ether)]
fn alloc_ring_page(id: &str) -> port::Result<(u64, VirtRange)> {
    let pa = pagealloc::allocate_physpage().map_err(|_| "virtio: pagealloc exhausted")?;
    let physrange = PhysRange::with_pa_len(pa, PAGE_SIZE_4K);
    let mut physpage_allocator = vm::PhysPageAllocator {};
    let mut vmtrait_impl = vm::VmTraitImpl {};
    let vr = vm::kernel_pagetable()
        .map_phys_range(
            &mut physpage_allocator,
            &mut vmtrait_impl,
            id,
            &physrange,
            VaMapping::Offset(KZERO),
            Entry::rw_kernel_data(),
            vm::PageSize::Page4K,
            RootPageTableType::Kernel,
        )
        .map_err(|_| "virtio: ring page map failed")?;
    // SAFETY: the page is freshly allocated (exclusively ours) and was
    // just mapped r/w at KZERO+PA above; the zero fill writes exactly the
    // one mapped page (T-07-08 hygiene: no stale data in DMA buffers).
    unsafe { core::ptr::write_bytes(vr.start() as *mut u8, 0, PAGE_SIZE_4K) };
    Ok((pa.addr(), vr))
}

/// One queue's bring-up (§4.1.5.1.3 order): select → read queue_size
/// (accepted as-offered, never downsized; the one-page-per-part bound is
/// checked honestly) → allocate + zero the ring pages → program the three
/// area physical addresses → read the notify offset → queue_enable LAST.
#[cfg(dev_ether)]
fn bring_up_queue(
    common: &Regs,
    queue: u16,
    dir: virtio::Dir,
    notify_mult: u32,
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
    if desc_bytes > PAGE_SIZE_4K || avail_bytes > PAGE_SIZE_4K || used_bytes > PAGE_SIZE_4K {
        return Err("virtio: queue size spills a page (unsupported)");
    }
    let (desc_pa, desc_vr) = alloc_ring_page("vnetdesc")?;
    let (avail_pa, avail_vr) = alloc_ring_page("vnetavail")?;
    let (used_pa, used_vr) = alloc_ring_page("vnetused")?;
    let (hdr_pa, _hdr_vr) = alloc_ring_page("vnethdr")?;
    // The RX used-len bound covers the 12-byte header + one payload page.
    let rx_cap = virtio::VIRTIO_NET_HDR_LEN as u32 + RX_PAYLOAD_CAP;
    let ring = Ring::new(
        VolatileMem { range: desc_vr },
        VolatileMem { range: avail_vr },
        VolatileMem { range: used_vr },
        qsize,
        dir,
        hdr_pa,
        rx_cap,
    )?;
    virtio::set_queue_addresses(common, queue, desc_pa, avail_pa, used_pa);
    let notify_off = usize::from(virtio::queue_notify_off(common, queue)) * notify_mult as usize;
    virtio::enable_queue(common, queue);
    Ok((ring, notify_off))
}

/// Bind the discovered virtio-net NIC.  Called from main9's virt
/// IRQ-arming window under cfg(dev_ether) — after the GIC/timer/PL011
/// are armed, strictly before the unmask paths, so the whole ring setup
/// runs with IRQs still masked and races nothing.  Tolerant-kernel
/// posture: every failure is a loud named print and a skipped bind — the
/// boot continues (the gates are what assert presence).
#[cfg(dev_ether)]
pub fn bind_virtio_net(dt: &DeviceTree) {
    if let Err(msg) = bind_virtio_net_inner(dt) {
        println!("virtio-net: bind failed: {msg}");
    }
}

#[cfg(dev_ether)]
fn bind_virtio_net_inner(dt: &DeviceTree) -> port::Result<()> {
    // SAFETY: the discovery cell's publish-once discipline (written once
    // by discover_and_print, earlier in this same boot context).
    let found = unsafe { &*PCI_DISCOVERY.get() };
    let Some((disc, ecam)) = found.as_ref() else {
        println!("virtio-net: no NIC discovered — bind skipped");
        return Ok(());
    };
    let cfg = (disc.dev as usize) << 15;

    // BAR4: size it (all-ones mask/restore, T-07-09), assign it from the
    // DTB's 64-bit MMIO window (read from the DTB, never a constant),
    // then enable Memory Space + Bus Master explicitly (Pitfall N3:
    // qemu's modern path never auto-enables bus-master).
    let (bar_size, _bar_orig) = size_bar64(ecam, cfg, PCI_OFF_BAR4);
    if bar_size == 0 || bar_size > (16 << 20) {
        return Err("bar4 size implausible (device fault)");
    }
    let node = dt
        .find_device_type("pci")
        .next()
        .or_else(|| dt.find_compatible("pci-host-ecam-generic").next())
        .ok_or("no pci node in dtb")?;
    let (win_base, _win_size) =
        mmio64_window(dt, node).ok_or("no 64-bit mmio window in dtb ranges")?;
    let bar_pa = win_base;
    io::write_reg(ecam, cfg + PCI_OFF_BAR4, bar_pa as u32);
    io::write_reg(ecam, cfg + PCI_OFF_BAR4 + 4, (bar_pa >> 32) as u32);
    let cmd = io::read_reg16(ecam, cfg + PCI_OFF_COMMAND);
    io::write_reg16(ecam, cfg + PCI_OFF_COMMAND, cmd | PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER);
    // Pitfall N3 sanity: the BAR readback — a silent mismatch here is the
    // DMA dead zone's first symptom.
    let rb = (u64::from(read_reg(ecam, cfg + PCI_OFF_BAR4 + 4)) << 32)
        | (u64::from(read_reg(ecam, cfg + PCI_OFF_BAR4)) & !0xF);
    println!("pci: bar4 pa {bar_pa:#x} size {bar_size:#x} readback {rb:#x}");
    if rb != bar_pa {
        return Err("bar4 readback mismatch");
    }

    // Map the assigned BAR as device memory; the capability walk then
    // runs over the config space and each cap's (bar, offset, length) is
    // bounds-checked against the BAR size BEFORE the region is trusted
    // (T-07-11 — a lying cap aborts the bind loudly).
    let bar_phys = PhysRange::with_pa_len(PhysAddr::new(bar_pa), bar_size as usize);
    let bar4 = match map_device_register("virtnetbar4", bar_phys, vm::PageSize::Page4K) {
        Ok(vr) => vr,
        Err(msg) => {
            println!("virtio-net: can't map bar4 {msg:?}");
            return Err("bar4 map failed");
        }
    };
    let ecam_cfg = EcamCfg { range: ecam.clone(), base: cfg };
    let caps = virtio::Caps::parse(&ecam_cfg)?;
    for t in [caps.common, caps.notify, caps.isr] {
        if t.bar != 4 {
            return Err("virtio: capability outside bar4 (unsupported)");
        }
        if u64::from(t.offset) + u64::from(t.length) > bar_size {
            return Err("virtio: capability extent beyond bar4 (device fault)");
        }
    }
    let device = match caps.device {
        Some(d)
            if d.bar == 4
                && d.length >= 6
                && u64::from(d.offset) + u64::from(d.length) <= bar_size =>
        {
            d
        }
        _ => return Err("virtio: no usable device config capability"),
    };
    println!(
        "pci: caps common@{:#x}+{:#x} notify@{:#x} isr@{:#x} device@{:#x} mult {}",
        caps.common.offset,
        caps.common.length,
        caps.notify.offset,
        caps.isr.offset,
        device.offset,
        caps.notify_off_multiplier
    );

    let at = |off: u32, len: u32| VirtRange::with_len(bar4.start() + off as usize, len as usize);
    let common = Regs { range: at(caps.common.offset, caps.common.length) };
    let isr = at(caps.isr.offset, caps.isr.length);
    let devcfg = at(device.offset, device.length);
    let notify = VolatileMem { range: at(caps.notify.offset, caps.notify.length) };

    // The §3.1.1 machine (07-03): reset+read-back wait, ACKNOWLEDGE,
    // DRIVER, two-window feature read, exactly MAC|STATUS|VERSION_1
    // accepted (D-03), FEATURES_OK with the re-read — a rejecting device
    // is left FAILED and the Err propagates to the loud bind-fail print.
    let negotiated = virtio::negotiate(&common)?;
    // The audit pair (Pitfall N6): offered AND negotiated.  The
    // device_feature registers are RO — the post-negotiation re-read
    // returns the offered windows.
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
        *b = io::read_reg8(&devcfg, i);
    }

    // Queues: 0 = RX, 1 = TX (§5.1.2; queue 2 needs CTRL_VQ, never
    // negotiated).  queue_enable is written LAST per queue; DRIVER_OK
    // comes after both, before any notify.
    let (mut rx, rx_notify_off) = bring_up_queue(
        &common,
        virtio::VIRTIO_NET_QUEUE_RX,
        virtio::Dir::Rx,
        caps.notify_off_multiplier,
    )?;
    let (tx, tx_notify_off) = bring_up_queue(
        &common,
        virtio::VIRTIO_NET_QUEUE_TX,
        virtio::Dir::Tx,
        caps.notify_off_multiplier,
    )?;
    virtio::set_driver_ok(&common);

    // RX pre-post: every chain gets one zeroed 4 KiB buffer page, then
    // ONE notify — the first notify, after DRIVER_OK (§3.1.1).
    let nslots = (rx.qsize() / 2) as usize;
    let mut rx_free = [0u64; MAX_RX_SLOTS];
    let mut rx_free_len = 0usize;
    for _ in 0..nslots {
        let (pa, _vr) = alloc_ring_page("vnetrxbuf")?;
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

    let state = VirtioNetState {
        isr,
        devcfg,
        notify,
        rx,
        tx,
        rx_notify_off,
        tx_notify_off,
        rx_buf_pa,
        rx_free,
        rx_free_len,
        counters: virtio::VirtioNetCounters::default(),
        mac,
        link_reported: false,
        faults: 0,
    };
    let intid = 32 + disc.spi;
    // SAFETY: publish-once — these writes run at bind (boot context,
    // before gic::enable and before the IRQ unmask paths), so no IRQ
    // reader can observe a torn write; the cells are never written again
    // (the pl011 PL011_RANGE discipline).
    unsafe {
        *VIRTIO_NET.get() = Some(state);
        *VIRTIO_NET_INTID.get() = intid;
    }
    super::gic::enable(intid, 0xA0);
    println!(
        "#l0: virtio: 1000Mbps: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    Ok(())
}

/// Bounded used-ring retire per queue per invocation (T-07-13): at most
/// qsize/2 chains are outstanding, so a quiet queue always drains inside
/// the cap; under a storm the completions arriving mid-drain re-assert
/// the level line after EOI, so the leftover is picked up by the next
/// invocation — a wedge manifests as a gate timeout, never silence.
#[cfg(dev_ether)]
const DRAIN_CAP: u32 = virtio::MAX_QSIZE as u32;

/// The trap.rs dispatch entry for the published INTID.  IRQ-context
/// discipline (the pl011 CONS_RX rules): lock-free, no console on the hot
/// path (the one-time prints are the plan's explicit carve-out), bounded
/// work per invocation (DRAIN_CAP above).
///
/// The ISR byte is read FIRST (§4.1.4.5): the read resets the ISR to 0
/// and deasserts the level line at the source (qemu: qatomic_xchg(&isr,0)
/// + pci_irq_deassert) — dispatch_irq's EOI then completes the GIC
/// handshake, exactly the existing ack → handler → eoi flow.
#[cfg(dev_ether)]
pub fn virtio_net_irq() {
    // SAFETY: the state was published once at bind (before gic::enable and
    // before IRQ unmask); IRQ handlers run at EL1 with same-priority
    // nesting masked, so this is the only writer — the pl011 publish-once
    // discipline.
    let state = unsafe { &mut *VIRTIO_NET.get() };
    let Some(state) = state.as_mut() else { return };

    // FIRST device touch: the ISR byte (read-to-ack).
    let isr = io::read_reg8(&state.isr, 0);

    if isr & virtio::VIRTIO_ISR_QUEUE != 0 {
        let mut retired = [0u32; 2];
        for _ in 0..DRAIN_CAP {
            match state.tx.next_used() {
                None => break,
                Some(Ok(_used)) => {
                    // The slot freed itself inside next_used; the posted
                    // buffer's accounting is 07-07's TX path.
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
        for _ in 0..DRAIN_CAP {
            match state.rx.next_used() {
                None => break,
                Some(Ok(used)) => {
                    retired[virtio::VIRTIO_NET_QUEUE_RX as usize] += 1;
                    // The frame (used.len - 12 bytes at the slot's buffer
                    // page + 12) is counted-but-dropped until 07-07's
                    // devether demux; the page returns to the free stack.
                    let _frame_len = virtio::virtio_net_frame_len(used.len);
                    let pa = state.rx_buf_pa[used.slot as usize];
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
        // is LINK_UP (§5.1.4).  qemu/slirp's link never actually changes,
        // so this line exists for the audit trail, not the hot path.
        state.link_reported = true;
        let status = io::read_reg16(&state.devcfg, 6);
        println!("virtio-net: link {}", if status & 1 != 0 { "up" } else { "down" });
    }
}
