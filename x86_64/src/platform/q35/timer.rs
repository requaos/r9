// The LAPIC timer for qemu's q35 machine — one tick()/ticks() surface
// over two CPUID-gated modes (the aarch64 platform/virt/timer.rs shape,
// re-keyed): CPUID.1.ECX[24] set (the KVM -cpu host default) selects
// TSC-deadline — a one-shot armed by wrmsr(0x6E0) = rdtsc() + delta and
// re-armed inside tick(); clear (TCG never exposes the bit — RESEARCH
// Pitfall 2) selects LAPIC periodic — DCR divide + ICR initial count.
// The frequency is resolved at RUNTIME (CPUID 0x15/0x16 when present,
// else PIT-calibrated over a fixed one-shot window) and printed with
// its source — never a compiled-in constant.

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
/// Pitfall 2).
const MSR_IA32_TSC_DEADLINE: u32 = 0x6E0;

/// Tick rate: 100 Hz (the 10 ms slice — generous enough that TCG's
/// slower tick paths still reach the gate's marker counts inside the
/// overall budget; order, never timings, is what the gate asserts).
const TICKS_PER_SEC: u64 = 100;
/// The tick interval in milliseconds.
const TICK_MS: u64 = 1000 / TICKS_PER_SEC;
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
pub struct TimerState {
    pub mode: TimerMode,
    /// TSC-deadline: TSC ticks per tick interval.
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
/// this same core.
pub fn tick_plan(state: &TimerState, now: u64, ticks: &AtomicU64) -> (Rearm, u64) {
    let rearm = match state.mode {
        TimerMode::TscDeadline => Rearm::TscDeadline(now.wrapping_add(state.tsc_delta)),
        TimerMode::Periodic => Rearm::Periodic(state.icr),
    };
    let prev = ticks.fetch_add(1, Ordering::Relaxed);
    (rearm, prev)
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
            TimerState { mode, tsc_delta: 0, icr }
        }
    };

    println!("timer: {tsc_hz} Hz ({source}), {:?} mode, armed", state.mode);
    // SAFETY: the publish-once discipline — this write runs at boot
    // inside the arming window (IF=0), before any tick reader can run;
    // the cell is never written again.
    unsafe { *STATE.get() = Some(state) }
}

/// The timer interrupt handler (IRQ context, from trap's dispatch).
/// Re-arm FIRST (both modes — a one-shot fires exactly once otherwise),
/// account exactly one tick, print at the 1 Hz pacing, then the polled
/// console-RX sweep.  Printing from IRQ context is safe only because
/// post-boot nothing else holds the Console lock (the aarch64 timer.rs
/// doctrine: the idle loop prints nothing).
pub fn tick() {
    // SAFETY: STATE's write-once discipline — published at init inside
    // the arming window, read lock-free here.
    let Some(state) = (unsafe { &*STATE.get() }).as_ref() else {
        return;
    };
    let (rearm, prev) = tick_plan(state, cpu::rdtsc(), &TICKS);
    match rearm {
        Rearm::TscDeadline(deadline) => {
            // SAFETY: this MSR is only ever written in TscDeadline mode,
            // selected behind the CPUID.1.ECX[24] gate at init.
            unsafe { cpu::wrmsr(MSR_IA32_TSC_DEADLINE, deadline) }
        }
        Rearm::Periodic(count) => lapic::timer_init_count(count),
    }
    if prev % PRINT_EVERY == 0 {
        println!("tick {}", prev / PRINT_EVERY);
    }
    // The polled console-RX sweep: drain + echo the 16550 (the
    // bring-up console's RX path rides the tick; IRQ-driven RX is not
    // this phase).
    crate::uart16550::rx_echo(crate::uart16550::COM1);
}

/// The plausible-TSC sanity floor/ceiling for CPUID-reported
/// frequencies: synthetic qemu CPU models return nonsense leaf-0x15/0x16
/// values (measured: Skylake-Client under TCG reports a ~68 kHz
/// "crystal" — 832*576/7); a frequency outside the band is treated as
/// garbage and the chain falls through to the next source (Linux's
/// sanity-check discipline).
const TSC_HZ_MIN: u64 = 1_000_000;
const TSC_HZ_MAX: u64 = 1_000_000_000_000;

/// Resolve the TSC frequency at runtime: CPUID 0x15 (the core crystal
/// clock ratio) when the leaf is real, else CPUID 0x16 (the nominal
/// base in MHz), else a PIT one-shot calibration.  The source is
/// returned so init prints the provenance, loud.
fn resolve_tsc_hz() -> Result<(u64, &'static str), &'static str> {
    let (eax15, ebx15, ecx15, _) = cpu::cpuid(0x15, 0);
    if ecx15 != 0 && eax15 != 0 {
        // TSC = crystal (ECX) * numerator (EBX) / denominator (EAX).
        if let Some(hz) = u64::from(ecx15).checked_mul(u64::from(ebx15)) {
            let hz = hz / u64::from(eax15);
            if (TSC_HZ_MIN..=TSC_HZ_MAX).contains(&hz) {
                return Ok((hz, "cpuid 0x15"));
            }
        }
    }
    let (eax16, _, _, _) = cpu::cpuid(0x16, 0);
    if eax16 != 0 {
        let hz = u64::from(eax16) * 1_000_000;
        if (TSC_HZ_MIN..=TSC_HZ_MAX).contains(&hz) {
            return Ok((hz, "cpuid 0x16"));
        }
    }
    pit_calibrate_tsc()
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
        match delta.checked_mul(20) {
            Some(hz) => Ok((hz, "pit 1193182 Hz reference")),
            None => Err("pit calibration overflow"),
        }
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
