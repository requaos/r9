// GICv2 (GIC-400) driver for qemu's virt machine (gic-version=2).
//
// The register sequence is the phase-03 research Pattern 1 probe, verified
// live on qemu 11.1.2 (-M virt,gic-version=2 -cpu cortex-a72): distributor
// enable, accept-all priority mask, CPU-interface enable; per-interrupt
// ISENABLER/IPRIORITYR (+ ITARGETSR for SPIs only -- research P6: PPIs are
// hardwired per-CPU, SPIs are undelivered without a target write).  Handlers
// ack via GICC_IAR and finish with exactly one GICC_EOIR write per interrupt
// (never for the spurious INTID 1023).

use core::cell::SyncUnsafeCell;

use crate::deviceutil::map_device_register;
use crate::io::{read_reg, write_reg};
use crate::vm;
use port::Result;
use port::fdt::DeviceTree;
use port::mem::{PhysRange, VirtRange};

#[cfg(not(test))]
use port::println;

// Distributor (GICD) register offsets
pub const GICD_CTLR: usize = 0x000; // Distributor control register
pub const GICD_ISENABLER0: usize = 0x100; // Interrupt set-enable, 32 irqs per word
pub const GICD_IPRIORITYR0: usize = 0x400; // Interrupt priority, 4 irqs per word
pub const GICD_ITARGETSR0: usize = 0x800; // Interrupt cpu targets, 4 irqs per word

// CPU interface (GICC) register offsets
pub const GICC_CTLR: usize = 0x000; // CPU interface control register
pub const GICC_PMR: usize = 0x004; // Interrupt priority mask register
pub const GICC_IAR: usize = 0x00c; // Interrupt acknowledge register
pub const GICC_EOIR: usize = 0x010; // End of interrupt register

/// Spurious interrupt ID: acked but NEVER given an EOI write.
pub const SPURIOUS_INTID: u32 = 1023;

pub struct Gic {
    gicd: VirtRange,
    gicc: VirtRange,
}

/// Interrupt-context access to the GIC.  Written once at init (before
/// trap::unmask_irq), read lock-free thereafter -- taking an mcslock in IRQ
/// context could deadlock against an interrupted holder (research P8).
static GIC: SyncUnsafeCell<Option<Gic>> = SyncUnsafeCell::new(None);

impl Gic {
    fn new(dt: &DeviceTree) -> Result<Gic> {
        // qemu 11.1.2's gicv2 machine reports the GIC as "arm,cortex-a15-gic"
        // (the research correction to D-04); "arm,gic-400" is the fallback.
        let node = dt
            .find_compatible("arm,cortex-a15-gic")
            .next()
            .or_else(|| dt.find_compatible("arm,gic-400").next())
            .ok_or("can't find gic")?;

        // DTB reg order is GICD first, GICC second (verified from the dumped
        // virt-gicv2.dtb); assert both are present.
        let mut regs = dt.property_translated_reg_iter(node);
        let gicd_physrange = regs
            .next()
            .and_then(|reg| reg.regblock())
            .map(|reg| PhysRange::from(&reg))
            .ok_or("can't find gicd reg")?;
        let gicc_physrange = regs
            .next()
            .and_then(|reg| reg.regblock())
            .map(|reg| PhysRange::from(&reg))
            .ok_or("can't find gicc reg")?;

        let gicd = match map_device_register("gicd", gicd_physrange, vm::PageSize::Page4K) {
            Ok(gicd) => gicd,
            Err(msg) => {
                println!("can't map gicd {:?}", msg);
                return Err("can't create gic");
            }
        };
        let gicc = match map_device_register("gicc", gicc_physrange, vm::PageSize::Page4K) {
            Ok(gicc) => gicc,
            Err(msg) => {
                println!("can't map gicc {:?}", msg);
                return Err("can't create gic");
            }
        };

        let gic = Gic { gicd, gicc };

        // Verified probe sequence (research Pattern 1).
        write_reg(&gic.gicd, GICD_CTLR, 1); // enable distributor
        write_reg(&gic.gicc, GICC_PMR, 0xff); // accept all priorities
        write_reg(&gic.gicc, GICC_CTLR, 1); // enable cpu interface

        Ok(gic)
    }

    /// Enable interrupt intid at the given priority.  SPIs (intid >= 32)
    /// additionally get an ITARGETSR write targeting cpu0 (research P6);
    /// PPIs are hardwired per-CPU and must not be written.
    fn enable(&self, intid: u32, prio: u8) {
        let intid = intid as usize;

        // ISENABLERn: one bit per interrupt, 32 per word (read-modify-write).
        let isenabler_off = GICD_ISENABLER0 + 4 * (intid / 32);
        let isenabler = read_reg(&self.gicd, isenabler_off);
        write_reg(&self.gicd, isenabler_off, isenabler | (1 << (intid % 32)));

        // IPRIORITYRn: one byte per interrupt, 4 per word (probe used 0xA0).
        let byte_shift = (intid % 4) * 8;
        let ipriorityr_off = GICD_IPRIORITYR0 + 4 * (intid / 4);
        let ipriorityr = read_reg(&self.gicd, ipriorityr_off);
        write_reg(
            &self.gicd,
            ipriorityr_off,
            (ipriorityr & !(0xff << byte_shift)) | ((prio as u32) << byte_shift),
        );

        // ITARGETSRn: SPIs need an explicit cpu target or they are never
        // delivered; PPIs (intid < 32) are hardwired and skipped (P6).
        if intid >= 32 {
            let itargetsr_off = GICD_ITARGETSR0 + 4 * (intid / 4);
            let itargetsr = read_reg(&self.gicd, itargetsr_off);
            write_reg(
                &self.gicd,
                itargetsr_off,
                (itargetsr & !(0xff << byte_shift)) | (0x01 << byte_shift), // cpu0
            );
        }
    }

    /// Acknowledge the pending interrupt; the GICC_IAR read yields the INTID.
    fn ack(&self) -> u32 {
        read_reg(&self.gicc, GICC_IAR)
    }

    /// Complete the interrupt with a GICC_EOIR write.  Never call this for
    /// the spurious INTID 1023.
    fn eoi(&self, intid: u32) {
        write_reg(&self.gicc, GICC_EOIR, intid);
    }
}

/// Discover, map and enable the GIC.  Follows r9's tolerant device-init idiom:
/// a discovery failure prints and leaves the static empty (ack() then reports
/// the spurious INTID, so the IRQ path degrades instead of panicking).
pub fn init(dt: &DeviceTree) {
    match Gic::new(dt) {
        // SAFETY: the publish-once discipline — this write runs at boot
        // before trap::unmask_irq, so no IRQ reader can observe a torn
        // write; the cell is never written again.
        Ok(gic) => unsafe { *GIC.get() = Some(gic) },
        Err(msg) => {
            println!("can't initialise gic: {msg:?}");
        }
    }
}

pub fn enable(intid: u32, prio: u8) {
    // SAFETY: GIC's write-once discipline — published at init before
    // unmask; enable() runs in boot context, lock-free thereafter.
    if let Some(gic) = unsafe { &*GIC.get() }.as_ref() {
        gic.enable(intid, prio);
    }
}

/// Acknowledge from IRQ context.  If the GIC was never initialised, report
/// the spurious INTID so the dispatcher acks-without-EOI and returns.
pub fn ack() -> u32 {
    // SAFETY: GIC's write-once discipline (IRQ context reads lock-free —
    // the research-P8 no-lock rule).
    unsafe { &*GIC.get() }.as_ref().map_or(SPURIOUS_INTID, |gic| gic.ack())
}

pub fn eoi(intid: u32) {
    debug_assert!(intid != SPURIOUS_INTID);
    // SAFETY: GIC's write-once discipline (IRQ context, lock-free).
    if let Some(gic) = unsafe { &*GIC.get() }.as_ref() {
        gic.eoi(intid);
    }
}
