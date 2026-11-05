//! devip/stack.rs — THE SEAM (D-05; contract §12.1/§12.2): single ownership
//! of the smoltcp `Interface` + `SocketSet` + Device-adapter state, driven
//! by EVENTS ONLY — no periodic polling anywhere (the phase prohibition:
//! a hidden timer poller is a seam rewrite, never a fallback).
//!
//! LOCKING (the load-bearing invariants — netif.rs's documentation
//! discipline):
//!
//! - `STACK` is a `BusyCell<StackState>` (mcslock's non-spinning
//!   try-enter cell) owning the stack core.  `drive_stack()` enters with
//!   `.expect("devip: stack contended")` — fail-fast turns a locking design
//!   error into an accounted panic instead of a wedge (the devmnt/netif
//!   INGRESS discipline).  On this kernel the cell is provably
//!   uncontended: syscalls run IRQ-masked on a single CPU (the Phase-4
//!   invariant), so two drive contexts can never overlap in time.
//! - `Interface::poll` NEVER runs in IRQ context — r9's no-alloc-in-IRQ and
//!   mcslock-LEAF rules hold (RESEARCH Pattern 1's three verified reasons).
//!   The RX-IRQ leg WAKES a devip waiter; the woken SYSCALL context drives.
//! - Wake ALWAYS after guard drop: the waker bridge fires mid-poll with the
//!   STACK cell held, and the arch wake hook takes PROC_TABLE — so the
//!   bridge only RECORDS into `PENDING_WAKES` (`note_wake`); `drive_stack`
//!   drains the set after the cell drops (netif.rs's "IRQ-side wake call
//!   happens AFTER the guard drops" rule, verbatim).
//! - No allocation in IRQ context: the tick sweep's `tick_check` touches
//!   only atomics and the fixed-capacity waiter set; socket/frame storage
//!   is allocated at conv/bind time in syscall context.
//!
//! THE THREE DRIVE LEGS (D-05's locked surface):
//!
//! 1. RX-IRQ wake — the ether adapter's RX-pending interest wakes the
//!    registered devip waiter(s); the WOKEN context runs `drive_stack()`
//!    before re-testing its own file-op condition.  On loopback there is no
//!    IRQ: the TX-write loops the packet synchronously and leg 3's poll in
//!    the same file-op context subsumes it (this replaces the donor's
//!    loopback kproc reader).
//! 2. One-shot deadline armed from `poll_at()` — after each drive the next
//!    soft deadline is recorded in `NET_DEADLINE` (TICK UNITS: 1 tick = 1 ms
//!    — the aarch64 generic-timer tick, TICKS_PER_SEC=1000).  The existing
//!    1 ms tick sweep calls `tick_check(now)` (wired in 08-04 beside
//!    `proc::tick_timeouts`): a passed deadline wakes the devip waiters
//!    (they drive from their woken syscall context); with NO registered
//!    waiters it sets `STACK_DEBT` (the idle-stack-debt corner — TCP timers
//!    are soft state, QoS never correctness) and the next devip-dispatched
//!    op drains it via `drain_debt_if_set()`.
//! 3. Local file-op activity — every devip file op that mutates socket
//!    state ends with `drive_stack()` in the same syscall context.
//!
//! THE CLOCK BRIDGE: smoltcp's `Instant::from_millis(ticks())` — the arch's
//! 1 ms tick counter, registered once via `set_clock` (port cannot see the
//! arch's `timer::ticks`; the DevipHooks triple is the devpipe/devether
//! shape VERBATIM, so the tick source gets its own registration point).
//! CNTFRQ is read at runtime arch-side (62.5 MHz TCG / 24 MHz HVF, both
//! measured) — nothing here is hardcoded.
//!
//! The seam's wake precision comes from the per-socket waker bridge
//! (`crate::devip_waker`): smoltcp's one-shot wakers fire mid-poll into
//! `PENDING_WAKES`; convs re-arm + re-register after EVERY wake and re-check
//! `can_recv`/`can_send` (spurious-wake discipline, P-C).

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use smoltcp::iface::{Config, Interface, PollIngressSingleResult, SocketSet};
use smoltcp::time::Instant;
use smoltcp::wire::HardwareAddress;

use crate::mcslock::{BusyCell, Lock, LockNode};
use crate::Result;

use super::phyadapt::{LoopbackDev, StackDev};

// ---------------------------------------------------------------------------
// The block/wake hooks (the devpipe/devether shape verbatim) + the tick
// source (its own registration point — port cannot see the arch timer).
// ---------------------------------------------------------------------------

/// The arch-side block/wake hooks (devpipe's PipeHooks / netif's
/// EtherHooks shape): `current` names the calling proc (pid), `block`
/// parks the caller mid-syscall, `wake` readies the listed pids.
#[derive(Clone, Copy)]
pub struct DevipHooks {
    pub current: fn() -> u64,
    pub block: fn(),
    pub wake: fn(&[u64]),
}

static HOOKS: Lock<Option<DevipHooks>> = Lock::new("devip", None);

/// Register the arch-side hooks.  Called once during boot before any user
/// process exists; a second registration is a boot bug.
pub fn set_hooks(hooks: DevipHooks) {
    let node = LockNode::new();
    let mut guard = HOOKS.lock(&node);
    assert!(guard.is_none(), "devip: hooks registered twice");
    *guard = Some(hooks);
}

pub(crate) fn hooks() -> Result<DevipHooks> {
    let node = LockNode::new();
    HOOKS.lock(&node).ok_or("devip: block/wake hooks not registered")
}

static CLOCK: Lock<Option<fn() -> u64>> = Lock::new("devipclock", None);

/// Register the tick source (`aarch64::platform::timer::ticks` — 1 tick =
/// 1 ms).  Called once during boot; a second registration is a boot bug.
pub fn set_clock(f: fn() -> u64) {
    let node = LockNode::new();
    let mut guard = CLOCK.lock(&node);
    assert!(guard.is_none(), "devip: clock registered twice");
    *guard = Some(f);
}

/// The current tick count (1 tick = 1 ms).
fn ticks_now() -> u64 {
    let node = LockNode::new();
    let guard = CLOCK.lock(&node);
    let Some(f) = *guard else { panic!("devip: clock not registered") };
    f()
}

/// The Instant bridge: `Instant::from_millis(ticks())` — 1 tick = 1 ms
/// (TICKS_PER_SEC=1000 arch-side), CNTFRQ runtime-read, never hardcoded.
pub(crate) fn instant_now() -> Instant {
    Instant::from_millis(ticks_now() as i64)
}

/// Instant → tick units (1 tick = 1 ms).
fn instant_to_ticks(at: Instant) -> u64 {
    at.total_millis() as u64
}

// ---------------------------------------------------------------------------
// The fixed-capacity pid sets (netif.rs's WaitSet shape): one waiter per
// proc (MAX_PROCS=16 slots can never fill; pid 0 marks an empty slot).  NO
// allocation ANYWHERE — the IRQ-side tick sweep and the waker bridge move
// pids through a caller's stack buffer.
// ---------------------------------------------------------------------------

struct WaitSet {
    pids: [u64; 16],
}

impl WaitSet {
    const fn new() -> WaitSet {
        WaitSet { pids: [0; 16] }
    }
    fn register(&mut self, pid: u64) {
        if pid == 0 || self.pids.contains(&pid) {
            return;
        }
        if let Some(slot) = self.pids.iter_mut().find(|p| **p == 0) {
            *slot = pid;
        }
        // The full case is impossible (one waiter per proc, ≤ 16 procs).
    }
    /// Drain the set into a stack buffer (the wake hook's input) — returns
    /// the count; every drained slot clears.
    fn take(&mut self, out: &mut [u64; 16]) -> usize {
        let mut n = 0;
        for p in self.pids.iter_mut() {
            if *p != 0 {
                out[n] = *p;
                *p = 0;
                n += 1;
            }
        }
        n
    }
}

/// Pids blocked in devip file ops — the timer leg's wake set (the tick
/// sweep wakes them so a SYSCALL context drives the stack; poll never runs
/// in IRQ context).
static NET_WAITERS: BusyCell<WaitSet> = BusyCell::new(WaitSet::new());

/// Pids whose per-socket wakers fired mid-poll.  The bridge records here
/// (never the wake hook directly); `drive_stack` drains after the STACK
/// guard drops.
static PENDING_WAKES: BusyCell<WaitSet> = BusyCell::new(WaitSet::new());

/// The waker bridge's record entry (called from `devip_waker`'s vtable,
/// mid-poll with the STACK cell held — recording only, never the hook).
pub(crate) fn note_wake(pid: u64) {
    PENDING_WAKES.enter().expect("devip: pending wakes contended").register(pid);
}

/// Register a blocked devip waiter (the conv's block path registers BEFORE
/// `block()` — register → drop → block is wakeup-loss-free because syscalls
/// run IRQ-masked: the waker can only run after block() swtch'es).
pub fn register_net_waiter(pid: u64) {
    NET_WAITERS.enter().expect("devip: waiters contended").register(pid);
}

// ---------------------------------------------------------------------------
// The one-shot deadline + the stack-debt flag (D-05 leg 2 + the idle
// corner).
// ---------------------------------------------------------------------------

/// The disarmed sentinel for `NET_DEADLINE` (a deadline of "never").
pub const DEADLINE_DISARMED: u64 = u64::MAX;

/// The armed one-shot deadline, in TICK units (1 tick = 1 ms).  Armed from
/// `poll_at()` at the end of every drive; disarmed when smoltcp has no
/// pending timers.  The aarch64 1 ms tick sweep reads it (08-04 wires the
/// call into `timer.rs::tick()`).
pub static NET_DEADLINE: AtomicU64 = AtomicU64::new(DEADLINE_DISARMED);

/// The currently armed deadline (None = disarmed).
pub fn net_deadline() -> Option<u64> {
    match NET_DEADLINE.load(Ordering::Acquire) {
        DEADLINE_DISARMED => None,
        t => Some(t),
    }
}

/// The stack-debt flag: set when the deadline fires with NO registered
/// waiters (the idle corner — nothing is blocked, so nothing would drive
/// the retransmit).  The next devip-dispatched op drains it.
pub static STACK_DEBT: AtomicBool = AtomicBool::new(false);

/// The timer leg's tick-sweep entry (08-04 wires it into the 1 ms tick):
/// when the armed deadline has passed, wake the devip waiters — the WOKEN
/// contexts drive the stack (poll NEVER runs in IRQ context).  With no
/// registered waiters, set the stack-debt flag instead.  Re-arms happen at
/// the end of the woken/draining drive.
pub fn tick_check(now_ticks: u64) {
    let Some(deadline) = net_deadline() else { return };
    if now_ticks < deadline {
        return;
    }
    let mut pids = [0u64; 16];
    let n = NET_WAITERS.enter().expect("devip: waiters contended (invariant violation)").take(&mut pids);
    if n == 0 {
        STACK_DEBT.store(true, Ordering::Release);
        return;
    }
    if let Ok(h) = hooks() {
        (h.wake)(&pids[..n]);
    }
}

/// Drain the stack-debt flag: if a deadline fired while nobody was
/// blocked, drive the stack now (the next devip-dispatched op's first
/// move — the call site lands with the syscall arms in 08-04).  Returns
/// whether a debt was drained.
pub fn drain_debt_if_set() -> bool {
    if STACK_DEBT.swap(false, Ordering::AcqRel) {
        drive_stack();
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// The single-ownership stack core.
// ---------------------------------------------------------------------------

/// One bound medium's stack: the smoltcp Interface (one per bound ipifc —
/// contract §12.4), its SocketSet, and its Device adapter.  This plan lands
/// the loopback core; 08-06's ether leg grows `StackState` to per-medium
/// cores with the address-class demux (127/8 + ::1 → loopback).
pub struct StackCore {
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
    pub dev: StackDev,
}

impl StackCore {
    /// Construct the loopback core (the `bind loopback` medium — 08-04's
    /// ipifc path installs it into the seam).
    pub fn new_loopback(dev: LoopbackDev) -> StackCore {
        Self::new(StackDev::Loopback(dev))
    }

    /// Construct the scripted-test core (host pins only).
    #[cfg(test)]
    pub(crate) fn new_scripted(dev: super::phyadapt::ScriptedDev) -> StackCore {
        Self::new(StackDev::Scripted(dev))
    }

    fn new(dev: StackDev) -> StackCore {
        let mut dev = dev;
        let mut cfg = Config::new(HardwareAddress::Ip);
        // "strongly recommended… different on each boot" (docs.rs Config);
        // needs no crypto.  The tick counter mixes the boot's wall time in;
        // tests override the clock to 0 and get deterministic ISNs (the
        // pins parse captured frames, never hardcoded sequences).
        cfg.random_seed = ticks_now() ^ 0x9e37_79b9_7f4a_7c15;
        cfg.slaac = false; // Out-of-Scope (REQUIREMENTS; contract §12)
        let iface = Interface::new(cfg, &mut dev, instant_now());
        StackCore { iface, sockets: SocketSet::new(Vec::new()), dev }
    }
}

/// The seam's owned state.  One core per bound medium this era; `None`
/// until the first `bind` installs one.
pub struct StackState {
    pub core: Option<StackCore>,
}

/// THE single-ownership cell.  See the module header for the locking
/// invariants (uncontended by the IRQ-masked-syscall construction;
/// fail-fast on violation).
pub(crate) static STACK: BusyCell<StackState> = BusyCell::new(StackState { core: None });

/// The explicit per-drive ingress bound (P-D — never an unbounded full
/// `poll()`; docs.rs's own DoS warning).  The adapter queues are bounded by
/// construction (LOOPBACK_Q_CAP / the virtio ring), and the cap keeps the
/// syscall-latency envelope gate-visible.
pub const RX_DRIVE_CAP: usize = 64;

/// Drive the stack: bounded ingress, egress, maintenance, then re-arm the
/// one-shot deadline — and AFTER the cell drops, wake every pid whose
/// socket waker fired mid-poll.
///
/// Called from exactly the three D-05 legs: the RX-woken context, the
/// deadline-woken context (or the debt drain), and every mutating devip
/// file op.  Never from IRQ context.
pub fn drive_stack() {
    let now = instant_now();
    {
        let mut guard = STACK.enter().expect("devip: stack contended");
        if let Some(core) = guard.core.as_mut() {
            // Bounded ingress (P-D): at most RX_DRIVE_CAP packets per drive.
            for _ in 0..RX_DRIVE_CAP {
                match core.iface.poll_ingress_single(now, &mut core.dev, &mut core.sockets) {
                    PollIngressSingleResult::None => break,
                    PollIngressSingleResult::PacketProcessed
                    | PollIngressSingleResult::SocketStateChanged => {}
                }
            }
            core.iface.poll_egress(now, &mut core.dev, &mut core.sockets);
            core.iface.poll_maintenance(now);
            match core.iface.poll_at(now, &core.sockets) {
                Some(at) => NET_DEADLINE.store(instant_to_ticks(at), Ordering::Release),
                None => NET_DEADLINE.store(DEADLINE_DISARMED, Ordering::Release),
            }
        }
    }
    // Wake AFTER the guard drops — never the wake hook while the cell is
    // held (the hook takes PROC_TABLE).
    let mut pids = [0u64; 16];
    let n = PENDING_WAKES.enter().expect("devip: pending wakes contended").take(&mut pids);
    if n > 0 && let Ok(h) = hooks() {
        (h.wake)(&pids[..n]);
    }
}

// ---------------------------------------------------------------------------
// Host-test seams (crate-internal; compiled only in test builds).
// ---------------------------------------------------------------------------

impl StackCore {
    /// The scripted device (pins only — asserts the core is the scripted one).
    #[cfg(test)]
    pub(crate) fn dev_scripted(&self) -> &super::phyadapt::ScriptedDev {
        match &self.dev {
            #[cfg(test)]
            StackDev::Scripted(d) => d,
            _ => panic!("pin: not a scripted core"),
        }
    }

    /// The scripted device, mutably (the pins push inbound frames).
    #[cfg(test)]
    pub(crate) fn dev_scripted_mut(&mut self) -> &mut super::phyadapt::ScriptedDev {
        match &mut self.dev {
            #[cfg(test)]
            StackDev::Scripted(d) => d,
            _ => panic!("pin: not a scripted core"),
        }
    }
}

/// Install a fresh stack core for a pin, resetting the seam's global flags
/// and wake sets (the devether_tests publish-once discipline: the pins
/// serialize through a test lock and each gets a clean seam).
#[cfg(test)]
pub(crate) fn install_test_stack(core: StackCore) {
    {
        let mut guard = STACK.enter().expect("devip: stack contended");
        guard.core = Some(core);
    }
    NET_DEADLINE.store(DEADLINE_DISARMED, Ordering::Release);
    STACK_DEBT.store(false, Ordering::Release);
    let mut scratch = [0u64; 16];
    PENDING_WAKES.enter().expect("devip: pending wakes contended").take(&mut scratch);
    NET_WAITERS.enter().expect("devip: waiters contended").take(&mut scratch);
}

/// Enter the stack cell (pin surgery on the core under the real ownership
/// discipline — the pins drive the seam's actual cell, never a side copy).
/// The cell is a static, so the guard's lifetime is 'static.
#[cfg(test)]
pub(crate) fn enter_stack() -> crate::mcslock::BusyGuard<'static, StackState> {
    STACK.enter().expect("devip: stack contended")
}
