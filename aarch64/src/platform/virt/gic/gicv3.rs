// GICv3 driver for qemu's virt machine (gic-version=3) — the only GIC shape
// qemu 11.1.2's hvf accel accepts on darwin (-accel hvf -cpu host), and
// equally runnable under TCG.
//
// GICv3 reshapes the v2 driver in two ways: SGIs/PPIs move out of the
// distributor into the PER-CPU redistributor (GICR: an RD frame + an SGI
// frame per CPU, found via GICR_TYPER's affinity fields), and the CPU
// interface stops being GICC MMIO and becomes system registers (ICC_*_EL1
// via mrs/msr).  The bring-up sequence:
//
//   1. wake the redistributor — qemu resets GICR_WAKER.ProcessorSleep=1, and
//      a sleeping redistributor silently drops every PPI/SGI (dead timer,
//      dead console): clear ProcessorSleep, poll ChildrenAsleep=0;
//   2. GICD: all SPIs Group-1 (IGROUPRn), then enable with
//      ARE_NS|EnableGrp1NS (RWP-polled);
//   3. GICR SGI frame: all SGIs/PPIs Group-1 (IGROUPR0);
//   4. CPU interface: ICC_SRE_EL1.SRE=1, ICC_PMR_EL1=0xff (accept all),
//      ICC_IGRPEN1_EL1=1.
//
// Handlers ack via ICC_IAR1_EL1 and finish with exactly one ICC_EOIR1_EL1
// write per interrupt (ICC_CTLR_EL1.EOImode resets to 0, so the EOIR write
// both priority-drops and deactivates; never write it for the spurious
// INTID 1023).

use core::arch::asm;

use aarch64_cpu::registers::{MPIDR_EL1, Readable};

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
pub const GICD_TYPER: usize = 0x004; // Interrupt controller type register
pub const GICD_IGROUPR0: usize = 0x080; // Interrupt group, 32 irqs per word
pub const GICD_ISENABLER0: usize = 0x100; // Interrupt set-enable, 32 irqs per word
pub const GICD_IPRIORITYR0: usize = 0x400; // Interrupt priority, 4 irqs per word
pub const GICD_ICFGR0: usize = 0xC00; // Interrupt trigger config (SPIs), 16 irqs per word
pub const GICD_IROUTER0: usize = 0x6000; // Interrupt routing, one 64-bit reg per irq

// GICD_CTLR bits
const GICD_CTLR_ENABLE_GRP1NS: u32 = 1 << 1;
const GICD_CTLR_ARE_NS: u32 = 1 << 5;
const GICD_CTLR_RWP: u32 = 1 << 31; // register write pending

// Redistributor RD-frame register offsets
pub const GICR_TYPER: usize = 0x008; // Redistributor type (64-bit RO)
pub const GICR_WAKER: usize = 0x014; // Power management
const GICR_TYPER_LAST: u64 = 1 << 4;
const GICR_WAKER_PROCESSOR_SLEEP: u32 = 1 << 1;
const GICR_WAKER_CHILDREN_ASLEEP: u32 = 1 << 2;

// Redistributor SGI-frame register offsets
pub const GICR_IGROUPR0: usize = 0x080; // Interrupt group (SGIs/PPIs)
pub const GICR_ISENABLER0: usize = 0x100; // Interrupt set-enable
pub const GICR_IPRIORITYR0: usize = 0x400; // Interrupt priority
pub const GICR_ICFGR0: usize = 0xC00; // Interrupt trigger config (SGIs/PPIs)

/// Size of one redistributor's RD frame (the SGI frame follows at +64 KiB).
const GICR_RD_FRAME_SIZE: usize = 0x10000;
/// Size of one redistributor's RD+SGI frame pair.
const GICR_REDIST_STRIDE: usize = 2 * GICR_RD_FRAME_SIZE;
/// Bound on the redistributor scan; GICR_TYPER.Last terminates it first in
/// practice (the DTB window covers 123 frames — scanning them all is waste).
const GICR_SCAN_MAX: usize = 16;

pub struct Gicv3 {
    gicd: RegBlock,
    /// This CPU's redistributor SGI frame (SGI/PPI bank).  The RD frame is
    /// only touched at init (wake) and stays mapped without being stored.
    sgi: RegBlock,
    /// The boot CPU's affinity in GICD_IROUTER form (Aff0 at [7:0] ..
    /// Aff3 at [39:32], Interrupt_Routing_Mode clear) — the SPI target.
    routing: u64,
}

impl Gicv3 {
    pub fn new(dt: &DeviceTree) -> Result<Gicv3> {
        let node = dt.find_compatible("arm,gic-v3").next().ok_or("can't find gic-v3")?;

        // DTB reg order is GICD first, GICR second (verified from the dumped
        // virt-gicv3.dtb); assert both are present.  The ITS entry that would
        // follow on some machines is an MSI doorbell — unused here.
        let mut regs = dt.property_translated_reg_iter(node);
        let gicd_physrange = regs
            .next()
            .and_then(|reg| reg.regblock())
            .map(|reg| PhysRange::from(&reg))
            .ok_or("can't find gicd reg")?;
        let gicr_physrange = regs
            .next()
            .and_then(|reg| reg.regblock())
            .map(|reg| PhysRange::from(&reg))
            .ok_or("can't find gicr reg")?;

        let gicd = match map_device_register("gicd", gicd_physrange, vm::PageSize::Page4K) {
            Ok(gicd) => gicd,
            Err(msg) => {
                println!("can't map gicd {:?}", msg);
                return Err("can't create gicv3");
            }
        };
        let gicd = RegBlock::new(gicd);

        // This CPU's MPIDR_EL1 affinities select its redistributor frame and
        // become the IROUTER SPI target (Aff0 -> [7:0], Aff1 -> [15:8],
        // Aff2 -> [23:16], Aff3 -> [39:32]).
        let mpidr = MPIDR_EL1.get();
        let routing = (mpidr & 0xff)
            | (((mpidr >> 8) & 0xff) << 8)
            | (((mpidr >> 16) & 0xff) << 16)
            | (((mpidr >> 32) & 0xff) << 32);

        // Find THIS CPU's redistributor: RD frames are GICR_REDIST_STRIDE
        // apart, and GICR_TYPER's affinity fields (Aff0 at [39:32] .. Aff3
        // at [63:56]) match MPIDR_EL1's.  qemu numbers redistributors in cpu
        // order, so the boot CPU is frame 0 in practice.
        let frames = (gicr_physrange.size() / GICR_REDIST_STRIDE).min(GICR_SCAN_MAX);
        let mut rd = None;
        for i in 0..frames {
            let frame_phys = PhysRange::with_pa_len(
                gicr_physrange.start() + (i * GICR_REDIST_STRIDE) as u64,
                GICR_RD_FRAME_SIZE,
            );
            let Ok(vr) = map_device_register("gicr-rd", frame_phys.clone(), vm::PageSize::Page4K) else {
                break;
            };
            let block = RegBlock::new(vr);
            // GICR_TYPER is 64-bit RO; the two 32-bit halves read
            // independently (low word: Last + Processor_Number, high word:
            // the four affinity bytes).
            let typer =
                (block.read(GICR_TYPER) as u64) | ((block.read(GICR_TYPER + 4) as u64) << 32);
            let affinity_matches = (typer >> 32) & 0xff == mpidr & 0xff
                && (typer >> 40) & 0xff == (mpidr >> 8) & 0xff
                && (typer >> 48) & 0xff == (mpidr >> 16) & 0xff
                && (typer >> 56) & 0xff == (mpidr >> 32) & 0xff;
            if affinity_matches {
                rd = Some((block, frame_phys));
                break;
            }
            if typer & GICR_TYPER_LAST != 0 {
                break;
            }
        }
        let Some((rd, rd_phys)) = rd else {
            println!("gicr: no redistributor matches mpidr {:#x}", mpidr);
            return Err("can't find redistributor");
        };

        // The SGI frame follows the matched RD frame at +64 KiB.
        let sgi_phys =
            PhysRange::with_pa_len(rd_phys.start() + GICR_RD_FRAME_SIZE as u64, GICR_RD_FRAME_SIZE);
        let sgi = match map_device_register("gicr-sgi", sgi_phys, vm::PageSize::Page4K) {
            Ok(sgi) => sgi,
            Err(msg) => {
                println!("can't map gicr sgi {:?}", msg);
                return Err("can't create gicv3");
            }
        };
        let sgi = RegBlock::new(sgi);

        // 1. Wake the redistributor BEFORE touching its SGI frame (a sleeping
        //    redistributor's registers are inaccessible).  qemu resets
        //    ProcessorSleep=1; clear it and wait for ChildrenAsleep=0.
        let waker = rd.read(GICR_WAKER);
        rd.write(GICR_WAKER, waker & !GICR_WAKER_PROCESSOR_SLEEP);
        let mut awake = false;
        for _ in 0..1_000_000 {
            if rd.read(GICR_WAKER) & GICR_WAKER_CHILDREN_ASLEEP == 0 {
                awake = true;
                break;
            }
            core::hint::spin_loop();
        }
        if !awake {
            println!("gicr waker stuck: ChildrenAsleep never cleared");
            return Err("gicr stays asleep");
        }

        // 2. GICD: disable while configuring (every CTLR write is RWP-polled
        //    — the GIC applies config writes asynchronously), mark all SPIs
        //    Group-1, then enable with affinity routing.  Bank 0
        //    (SGIs/PPIs) is the redistributor's, configured below.
        gicd.write(GICD_CTLR, 0);
        wait_rwp(&gicd);
        let itlines = ((gicd.read(GICD_TYPER) & 0x1f) + 1) as usize;
        for n in 1..itlines {
            gicd.write(GICD_IGROUPR0 + 4 * n, !0);
        }
        gicd.write(GICD_CTLR, GICD_CTLR_ARE_NS | GICD_CTLR_ENABLE_GRP1NS);
        wait_rwp(&gicd);

        // 3. SGIs/PPIs Group-1 via this CPU's SGI frame.
        sgi.write(GICR_IGROUPR0, !0);

        // 4. CPU interface: system-register access on (qemu's reset state
        //    already sets SRE; write anyway — the bit is the whole CPU
        //    interface on firmware-first boots), accept-all priority mask,
        //    Group-1 enable.
        icc_sre_el1_write(icc_sre_el1_read() | 1);
        isb();
        icc_pmr_el1_write(0xff);
        icc_igrpen1_el1_write(1);
        isb();

        Ok(Gicv3 { gicd, sgi, routing })
    }

    /// Enable interrupt intid at the given priority.  SGIs/PPIs (intid < 32)
    /// live in this CPU's redistributor SGI frame; SPIs in the distributor,
    /// which also routes them to this CPU via GICD_IROUTERn (the v3
    /// successor of ITARGETSR — RES0 with ARE on).  The ICFGR trigger bits
    /// (SGI frame for intid < 32, GICD otherwise) are programmed LEVEL
    /// first — every source on this machine is level-triggered (the DTB's
    /// flags cells say 0x04), and real-silicon ICFGR reset is
    /// IMPLEMENTATION DEFINED (Pitfall N4); qemu defaults level, but we
    /// never rely on it.
    pub fn enable(&self, intid: u32, prio: u8) {
        let intid = intid as usize;
        let byte_shift = (intid % 4) * 8;
        // ICFGRn: 2 bits per interrupt, 16 per word; the trigger bit is the
        // high bit of the pair (0 = level).  Written BEFORE ISENABLER so
        // the interrupt is never enabled with the wrong trigger.
        let icfgr_bit = 0b10 << (2 * (intid % 16));

        if intid < 32 {
            // SGI/PPI: one bank total in the SGI frame (IGROUPR0 was set
            // all-Group-1 at init).
            let icfgr_off = GICR_ICFGR0 + 4 * (intid / 16);
            let icfgr = self.sgi.read(icfgr_off);
            self.sgi.write(icfgr_off, icfgr & !icfgr_bit);
            let isenabler = self.sgi.read(GICR_ISENABLER0);
            self.sgi.write(GICR_ISENABLER0, isenabler | (1 << (intid % 32)));
            let iprio_off = GICR_IPRIORITYR0 + 4 * (intid / 4);
            let iprio = self.sgi.read(iprio_off);
            self.sgi.write(iprio_off, (iprio & !(0xff << byte_shift)) | ((prio as u32) << byte_shift));
        } else {
            let icfgr_off = GICD_ICFGR0 + 4 * (intid / 16);
            let icfgr = self.gicd.read(icfgr_off);
            self.gicd.write(icfgr_off, icfgr & !icfgr_bit);
            let isenabler_off = GICD_ISENABLER0 + 4 * (intid / 32);
            let isenabler = self.gicd.read(isenabler_off);
            self.gicd.write(isenabler_off, isenabler | (1 << (intid % 32)));
            let iprio_off = GICD_IPRIORITYR0 + 4 * (intid / 4);
            let iprio = self.gicd.read(iprio_off);
            self.gicd
                .write(iprio_off, (iprio & !(0xff << byte_shift)) | ((prio as u32) << byte_shift));
            // Route to this CPU (Interrupt_Routing_Mode clear = affinity
            // routing).  Written as two 32-bit halves; qemu's reset value
            // already targets affinity 0, so this documents rather than
            // changes the routing on the single-CPU profiles.
            let irouter_off = GICD_IROUTER0 + 8 * intid;
            self.gicd.write(irouter_off, self.routing as u32);
            self.gicd.write(irouter_off + 4, (self.routing >> 32) as u32);
        }
    }

    /// Acknowledge the pending interrupt; the ICC_IAR1_EL1 read yields the
    /// INTID in bits 23:0.
    pub fn ack(&self) -> u32 {
        icc_iar1_el1_read() & 0x00ff_ffff
    }

    /// Complete the interrupt with one ICC_EOIR1_EL1 write (EOImode 0: this
    /// also deactivates).  Never call this for the spurious INTID 1023.
    pub fn eoi(&self, intid: u32) {
        icc_eoir1_el1_write(intid);
    }
}

/// Poll GICD_CTLR.RWP clear — the distributor applies CTLR/config writes
/// asynchronously and later writes can be lost while a write is pending.
fn wait_rwp(gicd: &RegBlock) {
    for _ in 0..1_000_000 {
        if gicd.read(GICD_CTLR) & GICD_CTLR_RWP == 0 {
            return;
        }
        core::hint::spin_loop();
    }
    println!("gicd rwp stuck: register write never completed");
}

fn isb() {
    // SAFETY: instruction barrier — no side effects beyond serialization.
    unsafe { asm!("isb") };
}

/// MPIDR is read via the aarch64-cpu crate, but that crate ships only two of
/// the ICC registers (ICC_CTLR_EL1, ICC_SRE_EL2), so the CPU interface is
/// driven by these asm helpers — one mechanism for all of them.
fn icc_sre_el1_read() -> u64 {
    let val: u64;
    // SAFETY: ICC_SRE_EL1 is EL1-accessible; the read has no side effects.
    unsafe { asm!("mrs {val}, ICC_SRE_EL1", val = out(reg) val) };
    val
}

fn icc_sre_el1_write(val: u64) {
    // SAFETY: enables system-register access to the CPU interface (SRE);
    // EL1-writable, followed by isb at the call site.
    unsafe { asm!("msr ICC_SRE_EL1, {val}", val = in(reg) val) };
}

fn icc_pmr_el1_write(val: u64) {
    // SAFETY: ICC_PMR_EL1 is EL1-writable; 0xff accepts all priorities.
    unsafe { asm!("msr ICC_PMR_EL1, {val}", val = in(reg) val) };
}

fn icc_igrpen1_el1_write(val: u64) {
    // SAFETY: ICC_IGRPEN1_EL1 is EL1-writable; enables Group-1 signaling,
    // followed by isb at the call site.
    unsafe { asm!("msr ICC_IGRPEN1_EL1, {val}", val = in(reg) val) };
}

fn icc_iar1_el1_read() -> u32 {
    let val: u64;
    // SAFETY: the interrupt acknowledge read — valid from EL1 in IRQ context
    // once init enabled system-register access and Group-1.
    unsafe { asm!("mrs {val}, ICC_IAR1_EL1", val = out(reg) val) };
    val as u32
}

fn icc_eoir1_el1_write(val: u32) {
    // SAFETY: the end-of-interrupt write — exactly one per acked interrupt
    // (never for the spurious INTID), from EL1 IRQ context.
    unsafe { asm!("msr ICC_EOIR1_EL1, {val}", val = in(reg) val as u64) };
}
