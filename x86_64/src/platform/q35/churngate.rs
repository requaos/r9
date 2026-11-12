// The kernel-side churn driver (17-02, FLUSH-02's D-06 tier (a) +
// CR-03's in-kernel contention leg): the netgate.rs whole-file
// precedent re-keyed from frame posts to fork+exec churn — WINDOWS
// #44's repro shape (~1400 exec iterations of the baked 14-09
// marker-binary lineage: churnmark, staged beside /amd64/init by
// Harvey's mkimage) under a heap_stats cursor-slope CEILING derived
// from a first measured-green baseline.
//
// THE SLOPE DOCTRINE (port/src/allocator.rs:574-583): the bump cursor
// only ever grows (freed blocks recycle through the quick/misc lists),
// so absolute free bytes prove nothing — the PER-EXEC cursor delta is
// the leak signal.  The gate samples the cursor before the loop, after
// ONE exec, after the baseline leg (BASELINE_N execs — the derivation
// input), and at the end, then asserts the steady-leg slope against
// CEILING_BYTES_PER_EXEC below.  Moribund-husk accumulation is NOT
// chased as a leak: slots are never reaped this era (proc.rs:103-110),
// the husks pin PROC_TABLE slots (static bytes, not heap), and their
// heap payloads are dropped at exit (exit_tail) — a known bounded era
// shape excluded from the ceiling derivation (OQ-2's resolution).
//
// THE CEILING (OQ-2): derived from the FIRST measured-green baseline
// as  max(3 x steady_per_exec, 1024) + 512  — a 3x headroom over the
// measured steady residue plus a fixed floor so a near-zero baseline
// still leaves jitter room.  The number and its derivation are pinned
// in the comment at the const; the driver asserts against the CONST
// (fail-closed kernel-side, the netgate discipline) and prints the
// slope line the xtask post-pass re-derives from — the bound is one
// number, owned by the kernel, parsed by the gate.
//
// THE CONTENTION LEG (CR-03's D-07 in-kernel half): every SEM_EVERY-th
// spawn is a churnsem generation (RFMEM children contending
// semacquire/semrelease on a shared Data word — the Phase-8
// register-before-every-block loop under real scheduler timing)
// spawned WITHOUT waiting, so a generation's contenders overlap the
// execs that follow it.  Presence-marked: the generation's own
// `churngate: semgen` line (printed by the churnsem parent from ring
// 3) and the driver's semgens= count in the terminal marker.
//
// Marker vocabulary (the consts defined once in xtask beside the
// q35net family, emitted here and by the staged binaries):
//   churngate: heap <tag> <used> <total>     per-sample cursor line
//   churngate: slope steady=<s> measured=<m> ceiling=<c>
//   churngate: DONE iterations=<n> semgens=<k>   the terminal marker
//   churngate: FAIL <reason>                 fail-closed; DONE withheld
//
// Concurrency: the driver runs as a REAL kernel-context proc (never
// ring 3) — bootstrapped by bootstrap_and_run from the dev_exec boot
// window, seeded into the scheduler like the fork child's first_entry
// trampoline, and always IRQ-masked (the syscall-context discipline:
// its prints need no cli..sti window; ticks fire at ring 3 and in the
// sched loop's idle_wait, never under the driver).  It blocks via
// proc::sched() — woken by the child's exit-path wait record — and its
// bounded waits arm sleep_until as a watchdog (the sys_sleep shape)
// so a wedge fails the gate loudly inside the xtask budget, never a
// bare hang.

#[cfg(any(test, dev_churn))]
use alloc::string::String;

/// Total exec iterations (the #44 repro shape: ~1400 fork+exec).
pub const CHURN_N: u32 = 1400;
/// The baseline leg's exec count: samples ride before/after this leg,
/// and the per-exec steady delta derived from it is the ceiling's
/// input (the empty/one-exec boundary edge is MEASURED, not assumed —
/// the first exec pays one-time warmup the steady slope must exclude).
pub const BASELINE_N: u32 = 40;
/// Every SEM_EVERY-th spawn is a churnsem contention generation (the
/// driver does not wait for it — the contenders overlap the execs
/// that follow).  1400 / 35 = 40 generations.
pub const SEM_EVERY: u32 = 35;
/// The per-exec cursor-delta ceiling (bytes of bump-cursor growth per
/// exec, steady leg), derived from the first measured-green baseline:
///
///   first measured-green run (KVM, q35churn, 17-02): steady = <s>
///   bytes/exec over the 2..=BASELINE_N leg -> ceiling =
///   max(3 x <s>, 1024) + 512 = <CEILING> bytes/exec.
///
/// [17-02] filled after the first measured-green run; the negative
/// control (this const doctored below the measured baseline) must
/// demonstrably fire churngate: FAIL and red the xtask assert before
/// this gate is trusted (T-17-04: the bound has teeth).
pub const CEILING_BYTES_PER_EXEC: u64 = 0;

/// The per-exec cursor delta over `n` execs, saturating (a shrinking
/// cursor is impossible by the doctrine; 0 means flat — no leak).
#[cfg(any(test, dev_churn))]
fn per_exec_delta(before: usize, after: usize, n: u32) -> u64 {
    let _ = (before, after, n);
    0
}

/// The gate verdict: the measured steady-leg per-exec delta against
/// the ceiling.  Err names both numbers — the FAIL line is the whole
/// diagnosis (a leak this gate exists to catch, not a gate bug).
#[cfg(any(test, dev_churn))]
fn slope_ok(measured: u64, ceiling: u64) -> Result<(), &'static str> {
    let _ = (measured, ceiling);
    Ok(())
}

/// The ceiling derivation (documented at CEILING_BYTES_PER_EXEC):
/// max(3 x steady, 1024) + 512.  Pinned as a pure fn so the committed
/// const can be audited against the derivation in one place.
#[cfg(any(test, dev_churn))]
fn derive_ceiling(steady: u64) -> u64 {
    let _ = steady;
    0
}

/// One per-sample cursor line.  `tag` names the sample point
/// (start/one/base/end) — the xtask post-pass pairs the base and end
/// samples with the slope line's numbers.
#[cfg(any(test, dev_churn))]
fn heap_marker(tag: &str, used: usize, total: usize) -> String {
    let _ = (tag, used, total);
    String::new()
}

/// The slope line: the steady baseline, the measured steady-leg delta,
/// and the ceiling — every number the post-pass re-derives the verdict
/// from, on the wire.
#[cfg(any(test, dev_churn))]
fn slope_marker(steady: u64, measured: u64, ceiling: u64) -> String {
    let _ = (steady, measured, ceiling);
    String::new()
}

/// The terminal marker (presence: the gate's DONE).
#[cfg(any(test, dev_churn))]
fn done_marker(iterations: u32, semgens: u32) -> String {
    let _ = semgens;
    alloc::format!("churngate: DONE iterations={iterations}")
}

/// Presence-based terminal emission: DONE lands ONLY on a green gate —
/// any failed stage has already printed its churngate: FAIL line and
/// the terminal marker is WITHHELD (fail-closed, the netgate
/// discipline: the gate reds on the missing DONE, never a silent
/// pass).
#[cfg(any(test, dev_churn))]
fn terminal_marker(gate_ok: bool, iterations: u32, semgens: u32) -> String {
    let _ = gate_ok;
    done_marker(iterations, semgens)
}

/// One fail-closed failure line.
#[cfg(any(test, dev_churn))]
fn fail_marker(reason: &str) -> String {
    alloc::format!("churngate: FAIL {reason}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // The planned behavior, from the task's <behavior>: a sample
    // series with a FLAT live-allocation delta passes the ceiling.
    #[test]
    fn flat_delta_passes_ceiling() {
        let s0 = 0x10_0000;
        let s1 = s0 + 512; // one exec's warmup
        let s2 = s1; // the baseline leg adds nothing steady
        assert_eq!(per_exec_delta(s1, s2, BASELINE_N), 0, "flat leg has zero slope");
        assert_eq!(per_exec_delta(s0, s0, CHURN_N), 0);
        assert_eq!(slope_ok(0, CEILING_BYTES_PER_EXEC), Ok(()));
    }

    // ...and a series whose per-exec delta exceeds the ceiling fails
    // it, naming both numbers (the FAIL line is the diagnosis).
    #[test]
    fn over_ceiling_delta_fails() {
        let ceiling = 4096u64;
        assert_eq!(slope_ok(ceiling, ceiling), Ok(()), "the boundary itself passes");
        let verdict = slope_ok(ceiling + 1, ceiling);
        assert!(verdict.is_err(), "one byte over the ceiling must fail");
        let msg = verdict.unwrap_err();
        assert!(msg.contains("4097"), "the FAIL line names the measured delta: {msg}");
        assert!(msg.contains("4096"), "the FAIL line names the ceiling: {msg}");
    }

    // The derivation formula the committed ceiling documents — pinned
    // so the const can be audited against it mechanically.
    #[test]
    fn derive_ceiling_covers_steady_and_jitter() {
        assert_eq!(derive_ceiling(0), 1024 + 512, "a zero baseline still leaves the jitter floor");
        assert_eq!(derive_ceiling(100), 3 * 100 + 512);
        assert_eq!(derive_ceiling(5000), 3 * 5000 + 512);
        assert!(derive_ceiling(10_000) > 3 * 10_000 - 512);
        // monotone in the baseline
        assert!(derive_ceiling(7) < derive_ceiling(7000));
    }

    // The marker shapes the xtask constants grep and the post-pass
    // parses — pinned exactly, both sides key off them.
    #[test]
    fn marker_shapes_are_exact() {
        assert_eq!(done_marker(1400, 40), "churngate: DONE iterations=1400 semgens=40");
        assert_eq!(heap_marker("start", 100, 200), "churngate: heap start 100 200");
        assert_eq!(slope_marker(1, 2, 3), "churngate: slope steady=1 measured=2 ceiling=3");
        assert_eq!(fail_marker("exec child did not exit"), "churngate: FAIL exec child did not exit");
    }

    // Presence-based terminal emission (the contention/marker
    // discipline): the terminal marker lands ONLY on a green gate —
    // on failure it is withheld (the FAIL line already fired) and the
    // gate reds on the missing DONE.
    #[test]
    fn terminal_marker_withheld_on_failure() {
        assert_eq!(terminal_marker(true, 1400, 40), "churngate: DONE iterations=1400 semgens=40");
        assert_eq!(terminal_marker(false, 1400, 40), "", "DONE is withheld on a failed gate");
    }
}
