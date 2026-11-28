// The LAPIC timer for qemu's q35 machine — one tick()/ticks() surface
// over two CPUID-gated modes (the aarch64 platform/virt/timer.rs shape,
// re-keyed): CPUID.1.ECX[24] set (the KVM -cpu host default) selects
// TSC-deadline — a one-shot armed by wrmsr(0x6E0) = rdtsc() + delta and
// re-armed inside tick(); clear (TCG never exposes the bit — RESEARCH
// Pitfall 2) selects LAPIC periodic — DCR divide + ICR initial count.
// The frequency is resolved at RUNTIME and printed with its source —
// never a compiled-in constant: the rate CPUID 0x15/0x16 reports stands
// only while a PIT measurement over a fixed one-shot window agrees with
// it.

use core::cell::SyncUnsafeCell;
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(not(test))]
use port::println;

use crate::cpu;
use crate::pio;
use crate::platform::q35::lapic;
use crate::trap;

/// IA32_TSC_DEADLINE — the TSC-deadline arm MSR, 0x6E0 (SDM Vol 3A
/// 10.5.4.2; Linux and qemu both name MSR_IA32_TSC_DEADLINE/DEADLINE at
/// 0x6e0).  NOT 0x832: that is the x2APIC-mode address of the LVT-timer
/// register ITSELF (0x800+(0x320>>4)), and KVM gates MSR-range
/// 0x800-0xBFF on the guest APIC actually being in x2APIC mode —
/// wrmsr(0x832) from xAPIC mode is what injected the #GP(0) that
/// blocked this gate (the 13-03 dispatch-3 root cause), and from x2APIC
/// mode it would clobber the LVT entry, not arm a deadline.  Written
/// ONLY inside the CPUID.1.ECX[24]-gated mode (init's arm and tick's
/// re-arm); an unconditional write would fault or hang under TCG,
/// silently making the gates KVM-only (the prohibition, RESEARCH
/// Pitfall 2).  pub(crate): lapic.rs's `init_ap` arms the AP's deadline
/// from the BSP's published calibration (SMP-01).
pub(crate) const MSR_IA32_TSC_DEADLINE: u32 = 0x6E0;

/// Tick rate: 100 Hz (the 10 ms slice — generous enough that TCG's
/// slower tick paths still reach the gate's marker counts inside the
/// overall budget; order, never timings, is what the gate asserts).
const TICKS_PER_SEC: u64 = 100;
/// The tick interval in milliseconds (the syscall layer's ms→tick
/// conversion consumes it: q35 ticks at 100 Hz, so ONE tick is 10 ms —
/// the aarch64 twin ticks at 1 kHz where the two units are the same
/// number, which is why the inherited `now + ms` was invisible there).
pub(crate) const TICK_MS: u64 = 1000 / TICKS_PER_SEC;
/// Print divider: one console line per second of ticks (the aarch64
/// pacing).
const PRINT_EVERY: u64 = TICKS_PER_SEC;

/// The CPUID-gated timer mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerMode {
    /// TSC-deadline (D-05): one-shot, armed by wrmsr(0x6E0) =
    /// rdtsc() + delta, re-armed inside tick().
    TscDeadline,
    /// LAPIC periodic (the TCG fallback): DCR divide + ICR initial
    /// count.
    Periodic,
}

/// The pure mode gate: leaf-1 ECX bit 24 set -> TSCDeadline, clear ->
/// Periodic — Periodic whenever the bit is absent, regardless of
/// anything else (the assumption-delta companion invariant; TCG's
/// qemu64 carries only SSE3|CX16).
pub fn timer_mode(cpuid1_ecx: u32) -> TimerMode {
    if cpu::has_tsc_deadline(cpuid1_ecx) {
        TimerMode::TscDeadline
    } else {
        TimerMode::Periodic
    }
}

/// The one-shot delta arithmetic: TSC ticks covering `duration_ms` at
/// `freq_hz` — (freq * ms) / 1000, checked so a hostile pair is a loud
/// None, never a wrapped deadline.
pub fn tsc_delta(freq_hz: u64, duration_ms: u64) -> Option<u64> {
    freq_hz.checked_mul(duration_ms).map(|ticks| ticks / 1000)
}

/// The armed timer's facts, published once at init and read lock-free
/// from tick context thereafter.
#[derive(Clone, Copy)]
pub struct TimerState {
    pub mode: TimerMode,
    /// TSC ticks per tick interval — recorded in both modes (smp's
    /// start_one_ap paces the AP bring-up with it); `tick_rearm` reads
    /// it in TscDeadline mode only.
    pub tsc_delta: u64,
    /// Periodic: LAPIC ICR count per tick interval.
    pub icr: u32,
}

/// What a tick must perform to re-arm (the re-arm-FIRST discipline: a
/// one-shot timer fires exactly once otherwise — the aarch64 arm()-in-
/// handler idiom).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rearm {
    /// TSC-deadline: write the absolute deadline to MSR 0x6E0.
    TscDeadline(u64),
    /// Periodic: rewrite the LAPIC initial count.
    Periodic(u32),
}

/// The pure tick core: compute the re-arm FIRST, then bump the counter
/// exactly once.  Returns the re-arm to perform and the pre-increment
/// count (the print-pacing input).  Split out so the host tests pin
/// the accounting contract without a CPU — the hardware tick() runs
/// this same core on CPU 0 (SMP-01: the global TICKS bumps on the BSP
/// only; a secondary's tick re-arms its own LAPIC timer without
/// touching the timebase).
pub fn tick_plan(state: &TimerState, now: u64, ticks: &AtomicU64) -> (Rearm, u64) {
    let rearm = tick_rearm(state, now);
    let prev = ticks.fetch_add(1, Ordering::Relaxed);
    (rearm, prev)
}

/// The re-arm half of the tick core (SMP-01): EVERY online CPU's LAPIC
/// timer re-arms itself per tick, but only CPU 0 bumps TICKS (two
/// increments per slice would double the 100 Hz timebase every
/// sleep/timeout deadline rides on — the aarch64 05-22 discipline).
pub fn tick_rearm(state: &TimerState, now: u64) -> Rearm {
    match state.mode {
        TimerMode::TscDeadline => Rearm::TscDeadline(now.wrapping_add(state.tsc_delta)),
        TimerMode::Periodic => Rearm::Periodic(state.icr),
    }
}

static TICKS: AtomicU64 = AtomicU64::new(0);

/// The current tick count (the deadlines consumer — the port-side
/// sleep/alarm sweep that reads it arrives with the exec era).
#[allow(dead_code)]
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// The armed state, published once at init (inside the arming window,
/// before sti), read lock-free from tick context thereafter — the gic/
/// publish-once discipline.
static STATE: SyncUnsafeCell<Option<TimerState>> = SyncUnsafeCell::new(None);

/// The published armed state (SMP-01): the AP LAPIC-timer arming input
/// — an AP never re-runs the PIT/TSC calibration; it reuses the BSP's
/// measured state verbatim.  None before init (an AP then arms untimed
/// and the BSP's init already printed the failure loudly).
#[allow(dead_code)] // consumed by lapic::init_ap (05-12's ap_main)
pub(crate) fn state() -> Option<TimerState> {
    // SAFETY: STATE's write-once discipline — published at init inside
    // the arming window (IF=0), read lock-free here; TimerState is Copy,
    // so the read returns a value, never a torn reference.
    (unsafe { &*STATE.get() }).as_ref().copied()
}

/// Bring the tick source up: probe CPUID once, resolve the TSC
/// frequency at runtime, program the timer LVT per mode, arm, publish.
/// Runs inside main9's arming window (IF=0); a failure prints loudly
/// and leaves the state unpublished — tick() then no-ops, so the boot
/// degrades (no ticks) instead of panicking.
pub fn init() {
    let (_, _, ecx1, _) = cpu::cpuid(1, 0);
    let mode = timer_mode(ecx1);

    let (tsc_hz, source) = match resolve_tsc_hz() {
        Ok(v) => v,
        Err(msg) => {
            println!("can't initialise timer: {msg}");
            return;
        }
    };

    let state = match mode {
        TimerMode::TscDeadline => {
            let Some(delta) = tsc_delta(tsc_hz, TICK_MS).filter(|d| *d > 0) else {
                println!("can't initialise timer: bad tsc delta at {tsc_hz} Hz");
                return;
            };
            // LVT: TSC-deadline mode, unmasked — the gate already proved
            // the mode, so the bit-18 write is legal.
            lapic::timer_lvt(trap::TIMER_VECTOR, lapic::LVT_TSC_DEADLINE);
            // SAFETY: CPUID.1.ECX[24] is set (mode == TscDeadline) — the
            // MSR write is gated exactly per the prohibition.
            unsafe { cpu::wrmsr(MSR_IA32_TSC_DEADLINE, cpu::rdtsc() + delta) };
            TimerState { mode, tsc_delta: delta, icr: 0 }
        }
        TimerMode::Periodic => {
            // The LAPIC countdown rate is NOT the TSC rate (the in-kernel
            // APIC's bus clock under KVM, qemu's model under TCG) — so it
            // is calibrated at runtime against the resolved TSC, the 9k
            // apiconline TSC-window shape.
            let icr = match calibrate_lapic_icr(tsc_hz) {
                Ok(c) if c > 0 => c,
                Ok(_) => {
                    println!("can't initialise timer: zero lapic calibration");
                    return;
                }
                Err(msg) => {
                    println!("can't initialise timer: {msg}");
                    return;
                }
            };
            lapic::timer_divide(lapic::TIMER_DIV1);
            lapic::timer_init_count(icr);
            // Unmask LAST: the LVT carried the mask through calibration.
            lapic::timer_lvt(trap::TIMER_VECTOR, lapic::LVT_PERIODIC);
            // tsc_delta is recorded in BOTH modes: start_one_ap paces the
            // AP bring-up (the INIT→SIPI 10 ms, the online timeout) with
            // it, and the TSC rate is resolved above regardless of how
            // the tick is armed.  Published as 0 here it collapsed both
            // delays on the periodic venue (TCG's qemu64 has no
            // TSC-deadline timer): the SIPI went out one cycle after
            // INIT and the online poll gave up ~0.4 ms later, before
            // the AP had run ap_main (measured live, TCG, 27-11).
            TimerState { mode, tsc_delta: tsc_delta(tsc_hz, TICK_MS).unwrap_or(0), icr }
        }
    };

    println!("timer: {tsc_hz} Hz ({source}), {:?} mode, armed", state.mode);
    // SAFETY: the publish-once discipline — this write runs at boot
    // inside the arming window (IF=0), before any tick reader can run;
    // the cell is never written again.
    unsafe { *STATE.get() = Some(state) }
}

/// The timer interrupt handler (IRQ context, from trap's dispatch —
/// EVERY online CPU's LAPIC timer lands here, SMP-01).  Re-arm FIRST
/// (both modes — a one-shot fires exactly once otherwise), then the
/// per-CPU discipline:
///
/// - The global TICKS bumps on CPU 0 ONLY (`dat::this_mach().cpu_id` —
///   two increments per slice would double the 100 Hz timebase every
///   sleep/timeout deadline rides on; the aarch64 05-22 discipline).
/// - Every CPU bumps its own watchdog heartbeat (SMP-12); CPU 0's tick
///   additionally runs the cross-CPU watchdog check and the shared-state
///   sweeps (tick_timeouts, the cons RX poll/wake), which stay on the
///   timebase-owning CPU so the single-CPU cadence is preserved exactly.
/// - preempt() is NOT here: trap's dispatch_irq calls it (gated on the
///   tick having interrupted ring 3 on this CPU — an idle CPU's stale
///   per-CPU current_proc can name a proc now Running on a sibling, the
///   05-22 double-run tear class) on BOTH CPUs.
///
/// Printing from IRQ context is safe only because post-boot nothing
/// else holds the Console lock (the aarch64 timer.rs doctrine: the
/// idle loop prints nothing).  Under dev_exec that invariant ENDS —
/// syscall context takes the Console MCS lock (devcons writes), so the
/// tick println is cfg-gated OFF there (Pitfall 8, the aarch64
/// exec-profile precedent): the tick's dev_exec arms are the timeout
/// sweep and the preemption instead, and the wire stays quiet.
pub fn tick() {
    // SAFETY: STATE's write-once discipline — published at init inside
    // the arming window, read lock-free here.
    let Some(state) = (unsafe { &*STATE.get() }).as_ref() else {
        return;
    };
    // The per-CPU re-arm: each CPU's OWN LAPIC timer (TSC-deadline MSR
    // is per-CPU; the periodic LAPIC count is per-LAPIC).
    match tick_rearm(state, cpu::rdtsc()) {
        Rearm::TscDeadline(deadline) => {
            // SAFETY: this MSR is only ever written in TscDeadline mode,
            // selected behind the CPUID.1.ECX[24] gate at init.
            unsafe { cpu::wrmsr(MSR_IA32_TSC_DEADLINE, deadline) }
        }
        Rearm::Periodic(count) => lapic::timer_init_count(count),
    }
    // SMP-12: the per-CPU progress heartbeat — every CPU's tick, every
    // config (a no-op in release builds and before register_port_smp).
    port::smp::watchdog_heartbeat();
    let cpu0 = crate::dat::this_mach().cpu_id == 0;
    // The global 100 Hz timebase: CPU 0 only (SMP-01).
    let prev = if cpu0 {
        TICKS.fetch_add(1, Ordering::Relaxed)
    } else {
        TICKS.load(Ordering::Relaxed)
    };
    #[cfg(not(dev_exec))]
    if cpu0 && prev % PRINT_EVERY == 0 {
        println!("tick {}", prev / PRINT_EVERY);
    }
    // The exec-profile arms (the aarch64 tick's dev_exec block, re-keyed):
    // the timeout sweep (alarm expiry + sleep wakes) — CPU 0 only, on the
    // timebase-owning CPU (the single-CPU cadence preserved exactly; the
    // sweep acts on PROC_TABLE under its ranked IrqSpinLock).
    #[cfg(dev_exec)]
    if cpu0 {
        crate::proc::tick_timeouts(prev + 1);
        // The net-deadline sweep (08-04, D-05's timer leg — D-10 wires it
        // on x86_64, the aarch64 platform/virt/timer.rs precedent
        // verbatim): the poll_at-armed one-shot deadline rides the
        // existing tick (the 100 Hz q35 slice here — 1 ms on aarch64; the
        // sweep is deadline-driven, not rate-driven) — on expiry the
        // devip waiters wake and drive the stack from their WOKEN SYSCALL
        // context (poll NEVER runs in IRQ/tick context — the
        // no-alloc-in-IRQ and mcslock-LEAF invariants).  Without this
        // call, retransmit/keepalive/connect-timeout timers never fire
        // under full park on this venue (the 18-07 record's adjacent
        // latent defect).
        #[cfg(dev_ip)]
        port::devip::stack::tick_check(prev + 1);
    }
    // The console-RX sweep, cfg-keyed (aarch64's rx_irq echo/buffer
    // split re-keyed to the tick-polled 16550): the bring-up profile
    // echoes typed bytes verbatim (the Phase-13 typed-probe gate — no
    // reader exists, the wire echo is the proof); the exec profile
    // feeds the RX ring (uart16550::cons_rx) and wakes cons readers —
    // the cooked echo moves to the devcons input engine (reader
    // context, the echo hook), never tick context (the Console-lock
    // race the aarch64 P8 rule forbids).  CPU 0 only (SMP-01): the
    // 16550's polled producer stays on the timebase-owning CPU; a reader
    // running on CPU 1 is found through the wake's enqueue + the
    // scheduler's work stealing.
    #[cfg(not(dev_exec))]
    if cpu0 {
        crate::uart16550::rx_echo(crate::uart16550::COM1);
    }
    #[cfg(dev_exec)]
    if cpu0 {
        crate::uart16550::rx_ring_buffer(crate::uart16550::COM1);
        if crate::uart16550::cons_rx_pending() {
            crate::proc::cons_input_wake();
        }
    }
    // SMP-12 (D-26): CPU 0's tick runs the cross-CPU deadlock watchdog —
    // a waiting CPU whose heartbeat stopped past the threshold panics
    // naming the CPU, the lock and its owner (a no-op in release builds).
    if cpu0 {
        port::smp::watchdog_check_all(watchdog_threshold_ticks());
    }
}

/// The SMP-12 (D-26) cross-CPU deadlock watchdog threshold, in ticks
/// (1 tick = 10 ms at the q35 100 Hz slice): 500 ticks (5 s) is hugely
/// generous against the microsecond-scale critical sections on KVM;
/// TCG stretches every guest critical section under emulation (the
/// 05-RESEARCH §3.2 x6 scaling), so the TCG leg gets 3000 (30 s).  The
/// venue read is the CPUID hypervisor signature ("TCGTCGTCGTCG" under
/// QEMU's software emulation).
fn watchdog_threshold_ticks() -> u64 {
    if is_tcg() { 3_000 } else { 500 }
}

/// True under QEMU's TCG software emulation (vs KVM hardware
/// virtualization): CPUID leaf 0x40000000's hypervisor signature
/// (ebx/ecx/edx, in that order) spells "TCGTCGTCGTCG".  Bare metal (no
/// hypervisor leaf) reads as not-TCG.
fn is_tcg() -> bool {
    let (_, ebx, ecx, edx) = cpu::cpuid(0x4000_0000, 0);
    let mut sig = [0u8; 12];
    sig[0..4].copy_from_slice(&ebx.to_le_bytes());
    sig[4..8].copy_from_slice(&ecx.to_le_bytes());
    sig[8..12].copy_from_slice(&edx.to_le_bytes());
    &sig == b"TCGTCGTCGTCG"
}

/// The plausible-TSC sanity floor/ceiling for CPUID-reported
/// frequencies: synthetic qemu CPU models return nonsense leaf-0x15/0x16
/// values (measured: Skylake-Client under TCG reports a ~68 kHz
/// "crystal" — 832*576/7); a frequency outside the band is treated as
/// garbage and the chain falls through to the next source (Linux's
/// sanity-check discipline).
const TSC_HZ_MIN: u64 = 1_000_000;
const TSC_HZ_MAX: u64 = 1_000_000_000_000;

/// How far a CPUID-reported rate may sit from the PIT's measurement and
/// still be the one used: one per cent, in parts per million.  The
/// measurement itself repeats to a few hundredths of a per cent (the
/// KVM and TCG legs' 2,496,10x,xxx..2,496,74x,xxx), so a real report
/// passes and a made-up one does not.
const TSC_REPORT_TOLERANCE_PPM: u64 = 10_000;

/// The CPUID-reported TSC rate: leaf 0x15 (the core crystal clock
/// ratio) when the leaf is real, else leaf 0x16 (the nominal base in
/// MHz) — None when neither reports a rate inside the band.
fn reported_tsc_hz(eax15: u32, ebx15: u32, ecx15: u32, eax16: u32) -> Option<(u64, &'static str)> {
    if ecx15 != 0 && eax15 != 0 {
        // TSC = crystal (ECX) * numerator (EBX) / denominator (EAX).
        if let Some(hz) = u64::from(ecx15).checked_mul(u64::from(ebx15)) {
            let hz = hz / u64::from(eax15);
            if (TSC_HZ_MIN..=TSC_HZ_MAX).contains(&hz) {
                return Some((hz, "cpuid 0x15"));
            }
        }
    }
    let hz = u64::from(eax16) * 1_000_000;
    (eax16 != 0 && (TSC_HZ_MIN..=TSC_HZ_MAX).contains(&hz)).then_some((hz, "cpuid 0x16"))
}

/// The rate to run on, where it came from, and the report it overruled
/// (None when nothing was overruled).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TscRate {
    hz: u64,
    source: &'static str,
    overruled: Option<u64>,
}

/// Pick the TSC rate: a CPUID report stands only while the PIT's
/// measurement agrees with it, and otherwise the measurement is used.
/// The band above cannot tell a made-up rate from a real one.
/// qemu64,+avx,+avx2,+xsave,+xsaveopt under TCG reports a 7 MHz base in
/// leaf 0x16 while its TSC counts the host's 2.5 GHz, and every
/// TSC-paced delay then collapsed: the AP bring-up's 10 ms wait after
/// INIT and 100 ms poll for the new CPU came to tens and hundreds of
/// microseconds, and the boot processor gave up on it a line before it
/// answered (measured 2026-11-28, the smoke-x64-avx2 CI leg).  A
/// measurement that fails outright leaves a report as the only source
/// there is.
fn choose_tsc_hz(
    reported: Option<(u64, &'static str)>,
    measured: Result<(u64, &'static str), &'static str>,
) -> Result<TscRate, &'static str> {
    let (measured_hz, measured_from) = match measured {
        Ok(measured) => measured,
        Err(msg) => {
            return match reported {
                Some((hz, source)) => Ok(TscRate { hz, source, overruled: None }),
                None => Err(msg),
            };
        }
    };
    match reported {
        Some((hz, source)) if within_ppm(hz, measured_hz, TSC_REPORT_TOLERANCE_PPM) => {
            Ok(TscRate { hz, source, overruled: None })
        }
        Some((hz, _)) => Ok(TscRate {
            hz: measured_hz,
            source: measured_from,
            overruled: Some(hz),
        }),
        None => Ok(TscRate { hz: measured_hz, source: measured_from, overruled: None }),
    }
}

/// True when `measured` lies within `tolerance_ppm` parts per million of
/// `reported`.
fn within_ppm(reported: u64, measured: u64, tolerance_ppm: u64) -> bool {
    let gap = reported.abs_diff(measured);
    // Neither side can overflow: both rates are below the ceiling.
    gap.checked_mul(1_000_000).is_some_and(|gap| gap <= reported.saturating_mul(tolerance_ppm))
}

/// Resolve the TSC frequency at runtime: the CPUID report (leaf 0x15,
/// else 0x16) checked against a PIT one-shot window — the choice is
/// choose_tsc_hz's.  The source is returned so init prints the
/// provenance, loud; an overruled report is named on its own line.
fn resolve_tsc_hz() -> Result<(u64, &'static str), &'static str> {
    let (eax15, ebx15, ecx15, _) = cpu::cpuid(0x15, 0);
    let (eax16, _, _, _) = cpu::cpuid(0x16, 0);
    let rate = choose_tsc_hz(reported_tsc_hz(eax15, ebx15, ecx15, eax16), pit_calibrate_tsc())?;
    if let Some(reported_hz) = rate.overruled {
        println!(
            "timer: cpuid reports {reported_hz} Hz, the PIT measures {} Hz — using the measurement",
            rate.hz
        );
    }
    Ok((rate.hz, rate.source))
}

/// The PIT ports (the 8254's fixed I/O block — an architectural fact
/// of the machine model, like COM1).
const PIT_CH2: u16 = 0x42;
const PIT_CMD: u16 = 0x43;
/// The NMI status/control port: bit 0 gates channel 2, bit 1 the
/// speaker, bit 5 reads channel 2's OUT.
const PIT_PORT61: u16 = 0x61;
/// The PIT's fixed input oscillator (14.31818 MHz / 12) — the
/// calibration REFERENCE, an architectural constant of the 8254 (9k
/// i8254.c's Osc; Linux's PIT_TICK_RATE), never the TSC frequency
/// itself (which is what gets measured).
const PIT_OSC_HZ: u64 = 1_193_182;
/// The calibration window: a ~50 ms channel-2 mode-0 one-shot.
const PIT_CAL_COUNT: u32 = (PIT_OSC_HZ / 20) as u32;

/// Calibrate the TSC against the PIT: program channel 2 as a mode-0
/// one-shot (gated through port 0x61), read the TSC across it, scale
/// the measured ticks to Hz.  pio port 0x4x accesses — the in* family
/// Task 1 landed.
fn pit_calibrate_tsc() -> Result<(u64, &'static str), &'static str> {
    // SAFETY: port I/O to the fixed 8254 block — the channel-2 gate is
    // raised and restored around the window; nothing else on q35 uses
    // channel 2 (the speaker stays off).
    unsafe {
        let p61 = pio::inb(PIT_PORT61);
        // Gate on (bit 0), speaker off (bit 1): OUT (bit 5) reads low
        // until terminal count.
        pio::outb(PIT_PORT61, (p61 & !0x02) | 0x01);
        // Channel 2, lo/hi access, mode 0 (interrupt on terminal
        // count), binary.
        pio::outb(PIT_CMD, 0xB0);
        let tsc0 = cpu::rdtsc();
        pio::outb(PIT_CH2, (PIT_CAL_COUNT & 0xFF) as u8);
        pio::outb(PIT_CH2, (PIT_CAL_COUNT >> 8) as u8);
        // Bounded wait for OUT to go high: an anti-hang bound generous
        // past any plausible TSC rate (a wedged PIT must not hang the
        // boot — loud error instead).
        let hang_at = tsc0.wrapping_add(1 << 42);
        while pio::inb(PIT_PORT61) & 0x20 == 0 {
            if cpu::rdtsc() >= hang_at {
                pio::outb(PIT_PORT61, p61);
                return Err("pit calibration window timed out");
            }
        }
        let tsc1 = cpu::rdtsc();
        pio::outb(PIT_PORT61, p61);
        let delta = tsc1.wrapping_sub(tsc0);
        if delta == 0 {
            return Err("pit calibration measured zero");
        }
        // delta ticks per 1/20 s -> Hz = delta * 20.
        let hz = delta.checked_mul(20).ok_or("pit calibration overflow")?;
        // The report's band, for the measurement: a wedged or absent PIT
        // must not publish a rate no TSC runs at.
        if !(TSC_HZ_MIN..=TSC_HZ_MAX).contains(&hz) {
            return Err("pit calibration measured a rate outside the band");
        }
        Ok((hz, "pit 1193182 Hz reference"))
    }
}

/// Calibrate the LAPIC timer's countdown rate against the resolved TSC:
/// arm a masked, divide-by-1 full-scale countdown, spin a fixed TSC
/// window (~20 ms), and read how far the CCR ran — the ratio yields the
/// ICR for TICKS_PER_SEC.  Runs with the LVT masked and IF=0, so the
/// armed-but-masked state can deliver nothing (the 9k apiconline
/// TSC-window calibration, re-keyed).
fn calibrate_lapic_icr(tsc_hz: u64) -> Result<u32, &'static str> {
    lapic::timer_lvt(trap::TIMER_VECTOR, lapic::LVT_MASKED | lapic::LVT_PERIODIC);
    lapic::timer_divide(lapic::TIMER_DIV1);
    lapic::timer_init_count(0xFFFF_FFFF);
    let window = tsc_hz / 50;
    let tsc0 = cpu::rdtsc();
    while cpu::rdtsc().wrapping_sub(tsc0) < window {
        core::hint::spin_loop();
    }
    let elapsed_lapic = 0xFFFF_FFFFu64 - u64::from(lapic::timer_current_count());
    let elapsed_tsc = cpu::rdtsc().wrapping_sub(tsc0);
    if elapsed_lapic == 0 || elapsed_tsc == 0 {
        return Err("lapic calibration measured zero drift");
    }
    // lapic ticks/s = elapsed_lapic * tsc_hz / elapsed_tsc; both
    // factors are ~1e6..1e7 and ~1e9, so the product fits u64 with
    // orders of magnitude to spare — checked anyway (hostile-input
    // discipline).
    let lapic_hz = elapsed_lapic
        .checked_mul(tsc_hz)
        .ok_or("lapic calibration overflow")?
        / elapsed_tsc;
    let icr = lapic_hz / TICKS_PER_SEC;
    if icr == 0 || icr > u32::MAX as u64 {
        return Err("lapic calibration out of icr range");
    }
    Ok(icr as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 1 (the mode gate — the assumption-delta companion invariant):
    /// the pure decision over probed features returns TSCDeadline when
    /// leaf-1 ECX bit 24 is set and Periodic when it is clear —
    /// including the qemu64 shape (only SSE3|CX16, bit clear), the CPU
    /// model every TCG fallback leg boots with.  Periodic whenever the
    /// bit is absent, regardless of anything else.
    #[test]
    fn mode_gate_keys_off_cpuid_leaf1_ecx_bit24() {
        assert_eq!(timer_mode(1 << 24), TimerMode::TscDeadline);
        // the bit among company
        assert_eq!(timer_mode((1 << 24) | (1 << 0) | (1 << 13)), TimerMode::TscDeadline);
        // the qemu64 TCG shape verbatim: SSE3 (bit 0) | CX16 (bit 13),
        // bit 24 clear — RESEARCH Pitfall 2's exact CPU model facts
        assert_eq!(timer_mode((1 << 0) | (1 << 13)), TimerMode::Periodic);
        assert_eq!(timer_mode(0), TimerMode::Periodic);
        // routed through the pure helper (cpu.rs)
        assert!(crate::cpu::has_tsc_deadline(1 << 24));
        assert!(!crate::cpu::has_tsc_deadline(0));
        assert!(!crate::cpu::has_tsc_deadline((1 << 0) | (1 << 13)));
    }

    /// Test 2 (deadline arithmetic): the one-shot delta computation
    /// converts a target interval to TSC units from a supplied
    /// frequency without overflow at the q35 scale — 1024M-class TSC
    /// frequencies, second-scale deltas.
    #[test]
    fn tsc_delta_converts_without_overflow_at_q35_scale() {
        // 1024M-class TSC, whole seconds
        assert_eq!(tsc_delta(1_024_000_000, 1000), Some(1_024_000_000));
        // 2.4 GHz, ten seconds
        assert_eq!(tsc_delta(2_400_000_000, 10_000), Some(24_000_000_000));
        // the 10 ms tick slice at 2.4 GHz
        assert_eq!(tsc_delta(2_400_000_000, 10), Some(24_000_000));
        // 1 ms at 1 GHz lands on an exact count
        assert_eq!(tsc_delta(1_000_000_000, 1), Some(1_000_000));
        // a hostile pair is a loud None, never a wrapped delta
        assert_eq!(tsc_delta(u64::MAX, u64::MAX), None);
    }

    /// Test 2b (the rate's provenance — the smoke-x64-avx2 leg's shape,
    /// measured 2026-11-28): qemu64,+avx,+avx2,+xsave,+xsaveopt under
    /// TCG reports a 7 MHz base in leaf 0x16 while its TSC counts the
    /// host's 2.496 GHz.  The report is inside the band, so only the
    /// PIT's cross-check rejects it; at the wrong rate the AP bring-up's
    /// 100 ms poll was 700,000 ticks.
    #[test]
    fn a_cpuid_rate_the_pit_contradicts_is_overruled() {
        let reported = reported_tsc_hz(0, 0, 0, 7);
        assert_eq!(reported, Some((7_000_000, "cpuid 0x16")));
        let rate = choose_tsc_hz(reported, Ok((2_496_741_520, "pit 1193182 Hz reference"))).unwrap();
        assert_eq!(
            rate,
            TscRate {
                hz: 2_496_741_520,
                source: "pit 1193182 Hz reference",
                overruled: Some(7_000_000),
            }
        );
        assert_eq!(tsc_delta(7_000_000, 100), Some(700_000));
        assert_eq!(tsc_delta(rate.hz, 100), Some(249_674_152));
    }

    /// Test 2c: a report the PIT confirms is used as reported, to the
    /// tolerance's edge — the shape of a real leaf 0x15 (a 2.4 GHz TSC
    /// off a 24 MHz crystal) on bare metal.
    #[test]
    fn a_cpuid_rate_the_pit_confirms_stands() {
        let reported = reported_tsc_hz(1, 100, 24_000_000, 2400);
        assert_eq!(reported, Some((2_400_000_000, "cpuid 0x15")));
        for measured_hz in [2_397_600_000u64, 2_400_000_000, 2_402_400_000] {
            let rate = choose_tsc_hz(reported, Ok((measured_hz, "pit 1193182 Hz reference"))).unwrap();
            assert_eq!((rate.hz, rate.source, rate.overruled), (2_400_000_000, "cpuid 0x15", None));
        }
        assert!(within_ppm(1_000_000_000, 990_000_000, 10_000));
        assert!(within_ppm(1_000_000_000, 1_010_000_000, 10_000));
        assert!(!within_ppm(1_000_000_000, 989_999_999, 10_000));
        assert!(!within_ppm(1_000_000_000, 1_010_000_001, 10_000));
        // the ceiling against the floor: no overflow, no agreement
        assert!(!within_ppm(1_000_000_000_000, 1_000_000, 10_000));
    }

    /// Test 2d: the default qemu64 TCG shape and this host's KVM shape —
    /// both leaves read zero — and an out-of-band crystal ratio that
    /// falls through to leaf 0x16.
    #[test]
    fn without_a_usable_report_the_pit_measurement_stands() {
        assert_eq!(reported_tsc_hz(0, 0, 0, 0), None);
        let rate = choose_tsc_hz(None, Ok((2_496_741_520, "pit 1193182 Hz reference"))).unwrap();
        assert_eq!((rate.hz, rate.source, rate.overruled), (2_496_741_520, "pit 1193182 Hz reference", None));
        // a ~68 kHz "crystal" at a 1:1 ratio is no rate; leaf 0x16 is
        assert_eq!(reported_tsc_hz(1, 1, 68_000, 2400), Some((2_400_000_000, "cpuid 0x16")));
        // and one past the ceiling is neither
        assert_eq!(reported_tsc_hz(0, 0, 0, 2_000_000), None);
    }

    /// Test 2e: a PIT that fails leaves a report as the only source
    /// there is, and fails the boot only when there is no report.
    #[test]
    fn a_failed_measurement_leaves_the_report_or_fails() {
        let rate = choose_tsc_hz(Some((2_400_000_000, "cpuid 0x16")), Err("pit calibration window timed out"))
            .unwrap();
        assert_eq!((rate.hz, rate.source, rate.overruled), (2_400_000_000, "cpuid 0x16", None));
        assert_eq!(choose_tsc_hz(None, Err("pit calibration window timed out")), Err("pit calibration window timed out"));
    }

    /// Test 3 (tick accounting): a tick handler invocation increments
    /// the ticks counter exactly once and re-arms — the counter surface
    /// ticks() reads.
    #[test]
    fn tick_accounts_once_and_rearms() {
        // TSC-deadline shape: deadline = now + delta, counter +1 exactly.
        let state = TimerState {
            mode: TimerMode::TscDeadline,
            tsc_delta: 102_400_000,
            icr: 0,
        };
        let ticks = AtomicU64::new(41);
        let (rearm, prev) = tick_plan(&state, 1_000_000, &ticks);
        assert_eq!(prev, 41);
        assert_eq!(ticks.load(Ordering::Relaxed), 42);
        assert_eq!(rearm, Rearm::TscDeadline(1_000_000 + 102_400_000));
        // Periodic shape: ICR rewrite, same single increment.
        let state = TimerState { mode: TimerMode::Periodic, tsc_delta: 0, icr: 625_000 };
        let ticks = AtomicU64::new(0);
        let (rearm, prev) = tick_plan(&state, 7_777_777, &ticks);
        assert_eq!(prev, 0);
        assert_eq!(ticks.load(Ordering::Relaxed), 1);
        assert_eq!(rearm, Rearm::Periodic(625_000));
    }
}
