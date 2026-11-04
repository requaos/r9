// GICv2 (GIC-400) driver for qemu's virt machine (gic-version=2).
//
// The register sequence was verified
// live on qemu 11.1.2 (-M virt,gic-version=2 -cpu cortex-a72): distributor
// enable, accept-all priority mask, CPU-interface enable; per-interrupt
// ISENABLER/IPRIORITYR (+ ITARGETSR for SPIs only -- research P6: PPIs are
// hardwired per-CPU, SPIs are undelivered without a target write).  Handlers
// ack via GICC_IAR and finish with exactly one GICC_EOIR write per interrupt
// (never for the spurious INTID 1023).

use crate::deviceutil::map_device_register;
use crate::regblock::RegBlock;
use crate::vm;
use port::Result;
use port::fdt::DeviceTree;
use port::mem::PhysRange;

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

pub struct Gicv2 {
    gicd: RegBlock,
    gicc: RegBlock,
}

impl Gicv2 {
    pub fn new(dt: &DeviceTree) -> Result<Gicv2> {
        // qemu 11.1.2's gicv2 machine reports the GIC as "arm,cortex-a15-gic";
        // "arm,gic-400" is the fallback.
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

        let gic = Gicv2 { gicd: RegBlock::new(gicd), gicc: RegBlock::new(gicc) };

        // Verified probe sequence (research Pattern 1).
        gic.gicd.write(GICD_CTLR, 1); // enable distributor
        gic.gicc.write(GICC_PMR, 0xff); // accept all priorities
        gic.gicc.write(GICC_CTLR, 1); // enable cpu interface

        Ok(gic)
    }

    /// Enable interrupt intid at the given priority.  SPIs (intid >= 32)
    /// additionally get an ITARGETSR write targeting cpu0 (research P6);
    /// PPIs are hardwired per-CPU and must not be written.
    pub fn enable(&self, intid: u32, prio: u8) {
        let intid = intid as usize;

        // ISENABLERn: one bit per interrupt, 32 per word (read-modify-write).
        let isenabler_off = GICD_ISENABLER0 + 4 * (intid / 32);
        let isenabler = self.gicd.read(isenabler_off);
        self.gicd.write(isenabler_off, isenabler | (1 << (intid % 32)));

        // IPRIORITYRn: one byte per interrupt, 4 per word (probe used 0xA0).
        let byte_shift = (intid % 4) * 8;
        let ipriorityr_off = GICD_IPRIORITYR0 + 4 * (intid / 4);
        let ipriorityr = self.gicd.read(ipriorityr_off);
        self.gicd
            .write(ipriorityr_off, (ipriorityr & !(0xff << byte_shift)) | ((prio as u32) << byte_shift));

        // ITARGETSRn: SPIs need an explicit cpu target or they are never
        // delivered; PPIs (intid < 32) are hardwired and skipped (P6).
        if intid >= 32 {
            let itargetsr_off = GICD_ITARGETSR0 + 4 * (intid / 4);
            let itargetsr = self.gicd.read(itargetsr_off);
            self.gicd
                .write(itargetsr_off, (itargetsr & !(0xff << byte_shift)) | (0x01 << byte_shift)); // cpu0
        }
    }

    /// Acknowledge the pending interrupt; the GICC_IAR read yields the INTID.
    pub fn ack(&self) -> u32 {
        self.gicc.read(GICC_IAR)
    }

    /// Complete the interrupt with a GICC_EOIR write.  Never call this for
    /// the spurious INTID 1023.
    pub fn eoi(&self, intid: u32) {
        self.gicc.write(GICC_EOIR, intid);
    }
}
