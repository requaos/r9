// ARM generic timer (EL1 physical timer, CNTP) for qemu's virt machine.
//
// The sequence was verified live on
// qemu 11.1.2: CNTFRQ_EL0 is read at RUNTIME (62.5 MHz under TCG, 24 MHz
// under HVF -- both measured; never hardcoded), CNTP_TVAL_EL0 arms the
// countdown, CNTP_CTL_EL0 ENABLE=1 with IMASK clear raises the PPI.  CNTP is
// PPI 14, i.e. INTID 30 (16+14); the DTB timer node carries it as the
// <0x01 0x0e 0x104> triple (type PPI, num 14, flags level).

use core::sync::atomic::{AtomicU64, Ordering};

use aarch64_cpu::registers::{CNTFRQ_EL0, CNTP_CTL_EL0, CNTP_TVAL_EL0, Readable, Writeable};
use port::fdt::DeviceTree;

#[cfg(not(test))]
use port::println;

/// CNTP's PPI mapping (PPI 14 -> INTID 16+14), matching the committed
/// virt-gicv2.dtb timer node.  init() re-discovers it from the DTB and warns
/// if it ever disagrees; this constant is the documented fallback.
pub const CNTP_INTID: u32 = 30;

/// Tick rate: 1000 Hz = a 1 ms round-robin time slice.
/// The userland era runs only a few
/// ms of wall time under TCG, and kernel trap/syscall context is
/// IRQ-masked — a slower slice can expire only while masked and fire at
/// most once per eret-to-EL0; measured runs showed a 10 ms slice never
/// expiring mid-userland at all (zero preemptions).
const TICKS_PER_SEC: u64 = 1000;
/// Print divider for the not(dev_exec) profile: one console line per
/// second of ticks (the readable 1 Hz print-per-
/// tick pacing is unchanged by the faster slice).
const PRINT_EVERY: u64 = TICKS_PER_SEC;

static TICKS: AtomicU64 = AtomicU64::new(0);

/// The current tick count (sys_sleep/sys_alarm's deadlines).  1 tick
/// = 1 ms this era (TICKS_PER_SEC = 1000), so millisecond args are tick
/// counts verbatim.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Re-discover CNTP's INTID from the DTB timer node: interrupts is a list of
/// <type, num, flags> triples; CNTP is the PPI triple (type 0x01) with num
/// 0x0e.  PPIs map to INTID 16+num.  Fall back to CNTP_INTID if parsing finds
/// nothing (the committed virt-gicv2.dtb always carries it).
fn discover_intid(dt: &DeviceTree) -> u32 {
    let Some(node) = dt.find_compatible("arm,armv8-timer").next() else {
        return CNTP_INTID;
    };
    let Some(interrupts) = dt.property(&node, "interrupts") else {
        return CNTP_INTID;
    };
    let mut cells = dt.property_value_as_u32_iter(&interrupts);
    while let (Some(irq_type), Some(num), Some(_flags)) =
        (cells.next(), cells.next(), cells.next())
    {
        // PPI triples have type 0x01; CNTP is PPI 14.
        if irq_type == 0x01 && num == 14 {
            return 16 + num;
        }
    }
    CNTP_INTID
}

/// Arm the generic timer.  Called by main9 after the GIC is up and before
/// trap::unmask_irq() (research P5 ordering).
pub fn init(dt: &DeviceTree) {
    let intid = discover_intid(dt);
    if intid != CNTP_INTID {
        println!("timer: DTB reports CNTP intid {intid}, expected {CNTP_INTID}");
    }

    arm();

    // ENABLE with IMASK clear: assert the interrupt when the countdown ends.
    CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
}

/// (Re-)arm the countdown from the runtime frequency.  CNTFRQ_EL0 is read
/// every time -- 62.5 MHz under TCG vs 24 MHz under HVF are both measured
/// values, so nothing is hardcoded.
fn arm() {
    let freq = CNTFRQ_EL0.get();
    CNTP_TVAL_EL0.set(freq / TICKS_PER_SEC);
}

/// Timer interrupt handler.  TVAL is a countdown register, so every tick
/// re-arms it or the timer fires exactly once.
///
/// Forward note: printing from IRQ context is safe ONLY because post-boot
/// nothing else holds the Console lock (main9's idle loop is wfi and prints
/// nothing).  Under dev_exec that invariant ends — main9's exec orchestration
/// prints after unmask_irq, and userland-era syscalls print from trap
/// context — so the tick print is gated off there.
/// The tick itself keeps re-arming in every config.
///
/// The tick is the preemption source.  proc::preempt() acts
/// only when a process is Running (i.e. the IRQ interrupted EL0 — kernel
/// context runs IRQ-masked), so the call is a cheap no-op in the pre-exec
/// and no-exec configs; when it does act, the trap tail's CURFRAME reload
/// returns to the NEW current process.  preempt() never prints to the
/// console from IRQ context in a config where syscalls can print... its
/// `sched: switch` marker is the deliberate exception: syscalls run
/// IRQ-masked, so no Console-lock holder can be interrupted (single CPU).
pub fn tick() {
    arm();
    let n = TICKS.fetch_add(1, Ordering::Relaxed);
    #[cfg(not(dev_exec))]
    if n % PRINT_EVERY == 0 {
        println!("tick {}", n / PRINT_EVERY);
    }
    #[cfg(dev_exec)]
    {
        // The sleep/alarm sweep (the donor's tsleep wakeup +
        // alarmkproc expiry — init's restart-loop sleep(1000) rides it)
        // and the cons-read waker — a blocked console reader readies when
        // the PL011 RX ring holds bytes.  The peek is lock-free
        // (pl011.rs); both sweeps take PROC_TABLE from trap context
        // exactly as preempt()/reschedule() do (the proc.rs module-header
        // discipline: the lock is only ever held in IRQ-masked context).
        crate::proc::tick_timeouts(n + 1);
        // The net-deadline sweep (08-04, D-05's timer leg): the
        // poll_at-armed one-shot deadline rides the existing 1 ms tick —
        // on expiry the devip waiters wake and drive the stack from
        // their WOKEN SYSCALL context (poll NEVER runs in IRQ/tick
        // context — the no-alloc-in-IRQ and mcslock-LEAF invariants).
        #[cfg(dev_ip)]
        port::devip::stack::tick_check(n + 1);
        // Poll the UART FIFO directly before the wake check
        // — a byte that misses its RX/RT interrupt is collected here
        // within one tick, never lost until the next byte arrives.
        crate::platform::pl011::cons_rx_poll();
        if crate::platform::pl011::cons_rx_pending() {
            crate::proc::cons_input_wake();
        }
    }
    crate::proc::preempt();
}
