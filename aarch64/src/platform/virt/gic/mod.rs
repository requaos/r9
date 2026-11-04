// GIC dispatch for qemu's virt machine: v2 (GIC-400, gic-version=2) or v3
// (gic-version=3 — the only shape qemu 11.1.2's hvf accel accepts), selected
// by the DTB's compatible string at init.  Both backends satisfy the same
// contract: init discovers/maps/enables, enable arms an INTID, ack/eoi run
// from IRQ context lock-free, and the spurious INTID 1023 is acked but never
// EOI'd.

mod gicv2;
mod gicv3;

use core::cell::SyncUnsafeCell;

use port::Result;
use port::fdt::DeviceTree;

#[cfg(not(test))]
use port::println;

/// Spurious interrupt ID: acked but NEVER given an EOI write.
pub const SPURIOUS_INTID: u32 = 1023;

enum Gic {
    V2(gicv2::Gicv2),
    V3(gicv3::Gicv3),
}

/// Interrupt-context access to the GIC.  Written once at init (before
/// trap::unmask_irq), read lock-free thereafter -- taking an mcslock in IRQ
/// context could deadlock against an interrupted holder.
static GIC: SyncUnsafeCell<Option<Gic>> = SyncUnsafeCell::new(None);

/// Discover, map and enable the GIC.  Follows r9's tolerant device-init idiom:
/// a discovery failure prints and leaves the static empty (ack() then reports
/// the spurious INTID, so the IRQ path degrades instead of panicking).
pub fn init(dt: &DeviceTree) {
    let probed: Result<Gic> = if dt.find_compatible("arm,gic-v3").next().is_some() {
        gicv3::Gicv3::new(dt).map(Gic::V3)
    } else {
        gicv2::Gicv2::new(dt).map(Gic::V2)
    };
    match probed {
        // SAFETY: the publish-once discipline — this write runs at boot
        // before trap::unmask_irq, so no IRQ reader can observe a torn
        // write; the cell is never written again.
        Ok(gic) => unsafe { *GIC.get() = Some(gic) },
        Err(msg) => {
            println!("can't initialise gic: {msg:?}");
        }
    }
}

/// Enable interrupt intid at the given priority.
pub fn enable(intid: u32, prio: u8) {
    // SAFETY: GIC's write-once discipline — published at init before
    // unmask; enable() runs in boot context, lock-free thereafter.
    match unsafe { &*GIC.get() }.as_ref() {
        Some(Gic::V2(g)) => g.enable(intid, prio),
        Some(Gic::V3(g)) => g.enable(intid, prio),
        None => {}
    }
}

/// Acknowledge from IRQ context.  If the GIC was never initialised, report
/// the spurious INTID so the dispatcher acks-without-EOI and returns.
pub fn ack() -> u32 {
    // SAFETY: GIC's write-once discipline (IRQ context reads lock-free).
    unsafe { &*GIC.get() }.as_ref().map_or(SPURIOUS_INTID, |gic| match gic {
        Gic::V2(g) => g.ack(),
        Gic::V3(g) => g.ack(),
    })
}

pub fn eoi(intid: u32) {
    debug_assert!(intid != SPURIOUS_INTID);
    // SAFETY: GIC's write-once discipline (IRQ context, lock-free).
    match unsafe { &*GIC.get() }.as_ref() {
        Some(Gic::V2(g)) => g.eoi(intid),
        Some(Gic::V3(g)) => g.eoi(intid),
        None => {}
    }
}
