// ARM generic timer (EL1 physical timer, CNTP) for qemu's virt machine.
//
// The sequence was verified live on
// qemu 11.1.2: CNTFRQ_EL0 is read at RUNTIME (62.5 MHz under TCG, 24 MHz
// under HVF -- both measured; never hardcoded), CNTP_TVAL_EL0 arms the
// countdown, CNTP_CTL_EL0 ENABLE=1 with IMASK clear raises the PPI.  CNTP is
// PPI 14, i.e. INTID 30 (16+14); the DTB timer node carries it as the
// <0x01 0x0e 0x104> triple (type PPI, num 14, flags level).

use core::sync::atomic::{AtomicU64, Ordering};

use aarch64_cpu::registers::{CNTFRQ_EL0, CNTP_CTL_EL0, CNTP_TVAL_EL0, CNTPCT_EL0, Readable, Writeable};
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

/// Arm a SECONDARY CPU's own generic timer (SMP-04).  The CNTP registers
/// are per-CPU architectural state (each core's EL1 physical timer), so
/// the secondary's bring-up tail re-runs just the arm + enable: the DTB
/// re-discovery is the BSP's (one INTID, already warned on), and the
/// GIC-side PPI 30 enable for this CPU already ran in
/// gic::init_secondary_cpu (the banked ISENABLER0 write).
pub fn init_secondary() {
    arm();
    CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
}

/// The raw physical counter (CNTPCT_EL0) — the wall-clock source the SMP
/// bring-up's bounded online-wait measures with (the BSP spins there with
/// IRQs masked, so the 1 kHz TICKS cannot advance).
pub fn raw_counter() -> u64 {
    CNTPCT_EL0.get()
}

/// The physical counter's frequency (CNTFRQ_EL0) — pairs with
/// [`raw_counter`] to turn a tick budget into a wall-clock budget.
pub fn raw_counter_freq() -> u64 {
    CNTFRQ_EL0.get()
}

/// (Re-)arm the countdown from the runtime frequency.  CNTFRQ_EL0 is read
/// every time -- 62.5 MHz under TCG vs 24 MHz under HVF are both measured
/// values, so nothing is hardcoded.
fn arm() {
    let freq = CNTFRQ_EL0.get();
    CNTP_TVAL_EL0.set(freq / TICKS_PER_SEC);
}

/// The SMP-12 (D-26) cross-CPU deadlock watchdog threshold, in ticks
/// (1 tick = 1 ms), keyed off the runtime counter frequency rather than a
/// hardcoded venue flag: 24 MHz is HVF's measured CNTFRQ (5 s of guest
/// time is hugely generous against the microsecond-scale critical
/// sections), 62.5 MHz is TCG's — where the virtual counter tracks host
/// time while every guest critical section stretches under emulation, so
/// the same guest-time wait covers far less progress (05-RESEARCH 3.2's
/// x6 TCG scaling).
fn watchdog_threshold_ticks() -> u64 {
    if CNTFRQ_EL0.get() >= 50_000_000 { 30_000 } else { 5_000 }
}

/// Timer interrupt handler.  TVAL is a countdown register, so every tick
/// re-arms it or the timer fires exactly once.
///
/// SMP-04 (T-05-18): BOTH CPUs' CNTP timers fire this handler (PPI 30 is
/// per-CPU), but the global TICKS advances on CPU 0 ONLY — two increments
/// per millisecond would double the 1 kHz timebase every sleep/timeout
/// deadline rides on.  Every CPU bumps its own watchdog heartbeat
/// (SMP-12); CPU 0's tick additionally runs the cross-CPU watchdog check
/// and (under dev_exec) the shared-state sweeps, which stay on the
/// timebase-owning CPU so the single-CPU cadence is preserved exactly
/// (they act on PROC_TABLE/devip/cons state under their ranked locks).
/// preempt() runs on BOTH CPUs — a proc Running at EL0 on CPU 1 is
/// preempted by CPU 1's own tick.
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
/// and no-exec configs; when it does act, the trap tail's cur_frame reload
/// returns to the NEW current process.  preempt() never prints to the
/// console from IRQ context in a config where syscalls can print... its
/// `sched: switch` marker is the deliberate exception: syscalls run
/// IRQ-masked, so no Console-lock holder can be interrupted by a print
/// on the SAME CPU (and the Console IrqSpinLock serializes cross-CPU).
pub fn tick() {
    arm();
    // SMP-12: the per-CPU progress heartbeat — every CPU's tick, every
    // config (a no-op in release builds and before register_port_smp).
    port::smp::watchdog_heartbeat();
    let cpu0 = crate::smp::this_cpu().cpu_id == 0;
    // The global 1 kHz timebase: CPU 0 only (T-05-18).
    let n = if cpu0 { TICKS.fetch_add(1, Ordering::Relaxed) } else { TICKS.load(Ordering::Relaxed) };
    #[cfg(not(dev_exec))]
    if cpu0 && n % PRINT_EVERY == 0 {
        println!("tick {}", n / PRINT_EVERY);
    }
    #[cfg(dev_exec)]
    if cpu0 {
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
    // SMP-12 (D-26): CPU 0's tick runs the cross-CPU deadlock watchdog —
    // a waiting CPU whose heartbeat stopped past the threshold panics
    // naming the CPU, the lock and its owner (a no-op in release builds).
    if cpu0 {
        port::smp::watchdog_check_all(watchdog_threshold_ticks());
    }
    crate::proc::preempt();
}
