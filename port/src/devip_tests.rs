//! The devip seam host pins: homed at the CRATE root beside
//! allocator_tests.rs/virtio_tests.rs/devether_tests.rs/rx_chain_tests.rs
//! (the crate-root placement precedent — lib.rs's registration block; the
//! scripted-device drives and the stub wake log need alloc/std while
//! port::devip keeps its unsafe-free forbid attribute and the waker bridge
//! keeps its lone documented exception).
//!
//! The seam under pin is D-05's event-driven machinery: `drive_stack`, the
//! waker bridge (one-shot, re-register after EVERY wake, spurious-allowed),
//! the `poll_at`-armed one-shot deadline (`NET_DEADLINE`, tick units), and
//! the stack-debt drain.  smoltcp compiles for the host (probe-proven), so
//! the entire devip↔smoltcp mapping is pinned here BEFORE any guest boot
//! gate depends on it (the CONTEXT watch-out: the seam has no polling
//! cadence to save it from a missed waker).
//!
//! Test-harness discipline (devether_tests/rx_chain_tests precedents):
//!
//! - STUB HOOKS over test-module statics, registered ONCE per process via a
//!   std::sync::Once (the seam's set_hooks/set_clock assert-once, and every
//!   unit test in the crate shares one test binary).
//! - The stub clock is ADVANCEABLE (`TEST_TICKS`) — the deadline pins move
//!   time explicitly instead of sleeping (TCG-honest, deterministic).
//! - The stateful pins SERIALIZE on `SEAM_TEST_LOCK` (the rust test harness
//!   is multi-threaded; the seam's statics are process-global) and each
//!   installs a FRESH scripted stack (`install_test_stack` resets the
//!   deadline/debt/wake sets).
//! - Wire packets are crafted/parsed with smoltcp's own `wire` codecs (the
//!   RESEARCH's allowed reuse — the wire layer is public and documented).

use std::sync::Mutex;
use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::devip::phyadapt::ScriptedDev;
use crate::devip::stack::{self, StackCore};

// ---------------------------------------------------------------------------
// The stub hooks (the devpipe/devether shape: {current, block, wake}) and
// the advanceable stub clock.
// ---------------------------------------------------------------------------

/// The fixed pid the pins register (WaitSet.register silently drops pid 0).
const STUB_PID: u64 = 42;

/// The wake hook's record: every pid slice the seam woke, in order.
static WAKE_LOG: Mutex<Vec<u64>> = Mutex::new(Vec::new());
/// The advanceable stub clock (1 tick = 1 ms).
static TEST_TICKS: AtomicU64 = AtomicU64::new(0);
/// Serializes the stateful pins (process-global seam statics).
static SEAM_TEST_LOCK: Mutex<()> = Mutex::new(());
static HOOKS_ONCE: Once = Once::new();

fn stub_current() -> u64 {
    STUB_PID
}

fn stub_block() {}

fn stub_wake(pids: &[u64]) {
    WAKE_LOG.lock().unwrap().extend_from_slice(pids);
}

fn stub_ticks() -> u64 {
    TEST_TICKS.load(Ordering::Relaxed)
}

fn set_ticks(t: u64) {
    TEST_TICKS.store(t, Ordering::Relaxed);
}

fn take_wake_log() -> Vec<u64> {
    std::mem::take(&mut *WAKE_LOG.lock().unwrap())
}

/// Register the stub hooks + clock exactly once per process.
fn ensure_hooks() {
    HOOKS_ONCE.call_once(|| {
        stack::set_clock(stub_ticks);
        stack::set_hooks(stack::DevipHooks {
            current: stub_current,
            block: stub_block,
            wake: stub_wake,
        });
    });
}

/// Install a FRESH scripted stack with the clock at 0 and a clean wake log.
/// Returns the guard serializing this pin against the others.
fn fresh_scripted_stack() -> std::sync::MutexGuard<'static, ()> {
    let guard = SEAM_TEST_LOCK.lock().unwrap();
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    stack::install_test_stack(StackCore::new_scripted(ScriptedDev::new()));
    guard
}

// ---------------------------------------------------------------------------
// Smoke: the seam constructs, installs, and drives clean (Task 1's
// compile-standalone proof; the behavioral pins are Task 2's).
// ---------------------------------------------------------------------------

#[test]
fn seam_installs_and_drives_clean() {
    let _serial = fresh_scripted_stack();

    // An idle stack drives to quiescence: no ingress, no egress, no pending
    // timers → the deadline disarms and nothing wakes.
    stack::drive_stack();
    assert_eq!(stack::net_deadline(), None);
    assert_eq!(take_wake_log(), Vec::<u64>::new());

    // No debt, and a drain on a clear flag is a no-op.
    assert!(!stack::drain_debt_if_set());

    // A deadline that has not passed wakes nobody and sets no debt.
    stack::NET_DEADLINE.store(1000, Ordering::Release);
    stack::tick_check(999);
    assert_eq!(take_wake_log(), Vec::<u64>::new());
    stack::tick_check(1000);
    assert!(stack::STACK_DEBT.load(Ordering::Acquire));
    // The drain clears it and drives (idles) the stack.
    assert!(stack::drain_debt_if_set());
    assert_eq!(stack::net_deadline(), None);
}
