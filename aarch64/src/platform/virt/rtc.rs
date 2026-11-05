// PL031 RTC on qemu's virt machine — the epoch source for NSEC(53)
// (Harvey phase 06-07's measured tier-1 delta: libc time() is
// nsec()/1e9, so the nanosecond clock must be EPOCH-based, not
// boot-relative; the sweep's date check keys on a 20xx year).
//
// The committed DTBs name the node pl031@9010000 (compatible
// "arm,pl031", reg <0x00 0x9010000 0x00 0x1000>) — qemu's virt machine
// model instantiates the PL031 there unconditionally and seeds it from
// the host clock, so RTCDR (offset 0) reads current Unix-epoch SECONDS
// on TCG and HVF alike.
//
// Composition: a one-second-granular RTC is too coarse for a nanosecond
// clock, so at first call the RTC read is ANCHORED against the generic
// timer's monotonic count — anchor_ns = rtcdr*1e9 - mono_ns() — and
// every read after that is anchor + mono_ns(): sub-second resolution
// from CNTPCT_EL0 scaled by the runtime CNTFRQ_EL0 (62.5 MHz under TCG,
// 24 MHz under HVF — both measured by timer.rs; never hardcoded), no
// cross-clock drift within an era run.  The lazy map + anchor run in
// IRQ-masked single-CPU syscall context, so the init needs no locking
// beyond the atomics.
//
// raspi carries no RTC driver (the boards have none); this module is
// virt-only exactly like timer.rs — a raspi+dev_exec config would fail
// to COMPILE (the honest failure: a fake clock is never silently served).

use core::sync::atomic::{AtomicU64, Ordering};

use aarch64_cpu::registers::{CNTFRQ_EL0, CNTPCT_EL0, Readable};
use port::mem::PhysRange;

use crate::{deviceutil, vm};

/// The PL031's physical base (the committed virt DTBs' pl031@9010000).
const PL031_PA: u64 = 0x0901_0000;
/// RTCDR — the current-count register (seconds since the Unix epoch
/// under qemu's seeding).
const RTCDR_OFFSET: usize = 0x00;

/// The mapped PL031 page's VA (0 = unmapped; mapped lazily at first
/// NSEC call — syscall context, IRQ-masked, single CPU: the init cannot
/// race).
static RTC_VA: AtomicU64 = AtomicU64::new(0);
/// The anchor: epoch_ns - mono_ns at first call (0 = unanchored).
static ANCHOR_NS: AtomicU64 = AtomicU64::new(0);

/// The generic timer's monotonic nanosecond count (CNTPCT scaled by the
/// runtime CNTFRQ; u128 math — cntpct*1e9 overflows u64 within minutes
/// at the measured frequencies).
fn mono_ns() -> u64 {
    let cntpct = CNTPCT_EL0.get();
    let cntfrq = CNTFRQ_EL0.get();
    if cntfrq == 0 {
        return 0;
    }
    (cntpct as u128 * 1_000_000_000u128 / cntfrq as u128) as u64
}

/// Map the PL031 page and read RTCDR (epoch seconds).  None on a map
/// failure (loud — the handler's error arm reports it).
fn rtc_read_secs() -> Option<u64> {
    let mut va = RTC_VA.load(Ordering::Relaxed);
    if va == 0 {
        let vr = deviceutil::map_device_register(
            "pl031",
            PhysRange::with_pa_len(port::mem::PhysAddr::new(PL031_PA), vm::PageSize::Page4K.size()),
            vm::PageSize::Page4K,
        )
        .ok()?;
        va = vr.start() as u64;
        RTC_VA.store(va, Ordering::Relaxed);
    }
    // A device register read: the mapped page is rw_device; the u32 read
    // is volatile so the compiler keeps it a real MMIO load.
    let ptr = (va as usize + RTCDR_OFFSET) as *const u32;
    Some(unsafe { core::ptr::read_volatile(ptr) } as u64)
}

/// Epoch nanoseconds for NSEC(53): the PL031's epoch-seconds read
/// anchored once against the generic timer's monotonic count, then
/// anchor + mono per call.
pub fn nsec_epoch() -> Option<u64> {
    let mono = mono_ns();
    let mut anchor = ANCHOR_NS.load(Ordering::Relaxed);
    if anchor == 0 {
        let secs = rtc_read_secs()?;
        // mono starts near 0 at boot and the RTC read is the epoch, so
        // the anchor is always positive; saturating_sub is the loud-free
        // guard, never the expected path.
        anchor = secs.saturating_mul(1_000_000_000).saturating_sub(mono);
        ANCHOR_NS.store(anchor, Ordering::Relaxed);
    }
    Some(anchor + mono)
}
