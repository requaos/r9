// The LAPIC timer for qemu's q35 machine — one tick()/ticks() surface
// over two CPUID-gated modes (aarch64 platform/virt/timer.rs re-keyed):
// CPUID.1.ECX[24] set (KVM -cpu host) selects TSC-deadline — a one-shot
// armed by wrmsr(0x832) = rdtsc() + delta and re-armed inside tick();
// clear (TCG never exposes the bit, RESEARCH Pitfall 2) selects LAPIC
// periodic — DCR divide + ICR initial count.  The frequency is resolved
// at RUNTIME (CPUID 0x15/0x16 when present, else PIT-calibrated over a
// fixed window) — never a compiled-in constant.
//
// LOUD STUB (RED, 13-03 Task 2): the three planned behavior tests pin
// the contract FIRST; these stubs compile and fail the assertions on
// purpose.  The GREEN implementation replaces them.

// TEMPORARY (13-03): the machinery lands ahead of Task 3's main.rs
// arming-window wiring (and ahead of Task 2's own GREEN consumers);
// until then the bin crate's dead_code lint would fire on the
// not-yet-referenced items.  Task 3's wiring commit removes this
// whole-file allow (together with lapic.rs's).
#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};

/// The CPUID-gated timer mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerMode {
    /// TSC-deadline (D-05): one-shot, armed by wrmsr(0x832) =
    /// rdtsc() + delta, re-armed inside tick().
    TscDeadline,
    /// LAPIC periodic (the TCG fallback): DCR divide + ICR initial
    /// count.
    Periodic,
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
    /// TSC-deadline: write the absolute deadline to MSR 0x832.
    TscDeadline(u64),
    /// Periodic: rewrite the LAPIC initial count.
    Periodic(u32),
}

/// STUB: always Periodic — the bit-24-set assertion fails (by design).
pub fn timer_mode(_cpuid1_ecx: u32) -> TimerMode {
    TimerMode::Periodic
}

/// STUB: a zero delta — the arithmetic assertions fail (by design).
pub fn tsc_delta(_freq_hz: u64, _duration_ms: u64) -> Option<u64> {
    Some(0)
}

/// STUB: no increment, a null re-arm — the accounting assertions fail
/// (by design).
pub fn tick_plan(_state: &TimerState, _now: u64, ticks: &AtomicU64) -> (Rearm, u64) {
    (Rearm::Periodic(0), ticks.load(Ordering::Relaxed))
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
