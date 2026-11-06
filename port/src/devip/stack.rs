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

use smoltcp::iface::{Config, Interface, PollIngressSingleResult, SocketHandle, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::HardwareAddress;

use crate::mcslock::{BusyCell, Lock, LockNode};
use crate::Result;

use super::phyadapt::{EtherDev, LoopbackDev, StackDev};

// ---------------------------------------------------------------------------
// The two-core shape (contract §12.4 — one smoltcp Interface per bound
// ipifc): the loopback core (`core`, 08-04's field — the host pins' home)
// and the ether core (`ether`, 08-06).  A conv's core is picked at socket
// creation by the address-class demux and recorded on the conv (CoreSel).
// ---------------------------------------------------------------------------

/// Which bound medium's core a conv lives on.  Recorded at connect/
/// announce/bind time via the demux; every later conv op reads it back
/// (a conv's socket handle is valid only in its OWN core's SocketSet).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CoreSel {
    Loopback,
    Ether,
}

/// The address class (contract §12.4): 127/8 (v4-mapped) and ::1 are the
/// loopback class.
pub(crate) fn is_loopback_class(a: &[u8; 16]) -> bool {
    if let Some(v) = super::v4_of(a) {
        return v[0] == 127;
    }
    a[..15].iter().all(|&b| b == 0) && a[15] == 1
}

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

/// The raw tick count for seeding conv.rs's local-port PRNG (the
/// donor's nrand role — the port crate cannot see the arch timer, so
/// the seam exposes its registered clock).
pub(crate) fn ticks_for_seed() -> u64 {
    ticks_now()
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

/// D-05 leg 1's IRQ-side entry (08-06): the virtio-net RX retire's wake,
/// called from pci.rs's virtio_net_irq right after the rx_ingress demux
/// (cfg(dev_ip)-gated there).  Snapshots (drains) the devip waiter set and
/// wakes it AFTER the guard drop — the woken SYSCALL contexts run
/// drive_stack() (poll NEVER runs in IRQ context — the no-alloc-in-IRQ and
/// mcslock-LEAF invariants).  A WAKE, never a poll.  With no registered
/// waiters, sets the stack-debt flag (the unsolicited-frame corner — the
/// next devip-dispatched op drains it via drain_debt_if_set).
///
/// IRQ-context discipline (the tick_check sibling shape): NET_WAITERS is
/// the BusyCell, provably uncontended (syscalls run IRQ-masked — the IRQ
/// can only fire while no syscall holds the cell); no port mcslock is
/// taken; the wake hook (PROC_TABLE) runs after the guard drops.
pub fn note_rx_pending() {
    let mut pids = [0u64; 16];
    let n = NET_WAITERS.enter().expect("devip: waiters contended (IRQ invariant violation)").take(&mut pids);
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
/// contract §12.4), its SocketSet, and its Device adapter.
pub struct StackCore {
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
    pub dev: StackDev,
    /// The close-linger list (the donor's closeconv → tcpclose shape: a
    /// tcp conv's last close() SEQUENCES the FIN but the tcb — here, the
    /// socket — must live until the close handshake completes/ages out,
    /// or the FIN and any queued TX tail never reach the wire).  A closed
    /// conv's socket parks here (still IN `sockets`, so poll_at's timers
    /// and the poll legs keep driving it); drive_stack reaps the entry
    /// once the socket reaches State::Closed.  The list dies WITH the
    /// core (an unbind drops both), so an entry's handle can never name
    /// a successor core's SocketSet.
    pub(crate) linger: Vec<SocketHandle>,
}

impl StackCore {
    /// Construct the loopback core (the `bind loopback` medium — 08-04's
    /// ipifc path installs it into the seam).
    pub fn new_loopback(dev: LoopbackDev) -> StackCore {
        Self::new(StackDev::Loopback(dev))
    }

    /// Construct the ether core (the `bind ether` medium — 08-06): Config
    /// from the negotiated MAC (devether's published addr — the Interface's
    /// hardware_addr), slaac=false (REQUIREMENTS Out-of-Scope; contract
    /// §12), random_seed from devcons's D-17 DRBG (the kernel-internal
    /// draw — seeding it if this is the first read).
    pub fn new_ether(dev: EtherDev) -> StackCore {
        let mac = dev.mac();
        let mut cfg = Config::new(HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress(mac)));
        // "strongly recommended… different on each boot" (docs.rs Config)
        // — the D-17 DRBG's draw (a real CSPRNG seed, vs the loopback
        // era's tick mix).
        cfg.random_seed = crate::devcons::random_u64();
        cfg.slaac = false;
        let mut dev = dev;
        let iface = Interface::new(cfg, &mut dev, instant_now());
        StackCore { iface, sockets: SocketSet::new(Vec::new()), dev: StackDev::Ether(dev), linger: Vec::new() }
    }

    /// Construct the scripted-test core (host pins only).
    #[cfg(test)]
    pub(crate) fn new_scripted(dev: super::phyadapt::ScriptedDev) -> StackCore {
        Self::new(StackDev::Scripted(dev))
    }

    /// The scripted ETHER core (the 08-06 ether pins' device —
    /// Medium::Ethernet, the snoop live, TX captured).
    #[cfg(test)]
    pub(crate) fn new_scripted_ether(dev: super::phyadapt::ScriptedEtherDev) -> StackCore {
        let mac = dev.mac;
        let mut cfg = Config::new(HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress(mac)));
        cfg.random_seed = 0x9e37_79b9_7f4a_7c15;
        cfg.slaac = false;
        let mut dev = dev;
        let iface = Interface::new(cfg, &mut dev, instant_now());
        StackCore { iface, sockets: SocketSet::new(Vec::new()), dev: StackDev::ScriptedEther(dev), linger: Vec::new() }
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
        StackCore { iface, sockets: SocketSet::new(Vec::new()), dev, linger: Vec::new() }
    }
}

/// The seam's owned state: one core per bound medium (contract §12.4).
/// Both are `None` at boot; `bind loopback` / `bind ether` install them.
pub struct StackState {
    /// The loopback core (08-04's single `core` field — the host pins'
    /// `g.core` home; the name stays for the pin surface).
    pub core: Option<StackCore>,
    /// The ether core (08-06).
    pub ether: Option<StackCore>,
}

impl StackState {
    /// The address-class demux (contract §12.4/OQ-5): the loopback class
    /// (127/8, ::1) rides the loopback core; EVERYTHING ELSE rides the
    /// ether core WHEN BOUND — else the loopback core serves it (the
    /// single-core fallback: the 08-04/08-05 loopback gates never bind
    /// ether, so their wildcard announces and any-address connects stay
    /// on the loopback core — byte-identical behavior there).
    pub(crate) fn demux(&self, addr: &[u8; 16]) -> CoreSel {
        if is_loopback_class(addr) {
            CoreSel::Loopback
        } else if self.ether.is_some() {
            CoreSel::Ether
        } else {
            CoreSel::Loopback
        }
    }

    /// The selected core, mutably (None when that medium isn't bound).
    pub(crate) fn core_mut(&mut self, sel: CoreSel) -> Option<&mut StackCore> {
        match sel {
            CoreSel::Loopback => self.core.as_mut(),
            CoreSel::Ether => self.ether.as_mut(),
        }
    }

    /// The selected core, read-only.
    pub(crate) fn core_ref(&self, sel: CoreSel) -> Option<&StackCore> {
        match sel {
            CoreSel::Loopback => self.core.as_ref(),
            CoreSel::Ether => self.ether.as_ref(),
        }
    }
}

/// THE single-ownership cell.  See the module header for the locking
/// invariants (uncontended by the IRQ-masked-syscall construction;
/// fail-fast on violation).
pub(crate) static STACK: BusyCell<StackState> = BusyCell::new(StackState { core: None, ether: None });

/// The explicit per-drive ingress bound (P-D — never an unbounded full
/// `poll()`; docs.rs's own DoS warning).  The adapter queues are bounded by
/// construction (LOOPBACK_Q_CAP / the virtio ring), and the cap keeps the
/// syscall-latency envelope gate-visible.
pub const RX_DRIVE_CAP: usize = 64;

/// The flag-guarded drive counter print (D-13 — devcons-style static
/// flag): when set, each drive_stack completion prints `devip: drive ok
/// <n>`.  Default off; the debug surface for the seam's event flow.
pub static DRIVE_PRINT: AtomicBool = AtomicBool::new(false);
static DRIVE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Set the drive-print flag (debug wiring — the gates never set it).
pub fn set_drive_print(on: bool) {
    DRIVE_PRINT.store(on, Ordering::Relaxed);
}

/// Drive the stack: bounded ingress, egress, maintenance on EACH bound
/// core (08-06's ether core joined the loopback core), then re-arm the
/// one-shot deadline from the EARLIEST of the cores' `poll_at()` — and
/// AFTER the cell drops, wake every pid whose socket waker fired mid-poll.
///
/// Called from exactly the three D-05 legs: the RX-woken context, the
/// deadline-woken context (or the debt drain), and every mutating devip
/// file op.  Never from IRQ context.
pub fn drive_stack() {
    let now = instant_now();
    {
        let mut guard = STACK.enter().expect("devip: stack contended");
        let mut next: Option<u64> = None; // the earliest armed deadline across the cores
        // Destructure the guard to borrow the two slots disjointly (an
        // array of &mut to both fields would double-borrow `guard`).
        let StackState { core: lb, ether: eth } = &mut *guard;
        for core in [lb, eth].into_iter().flatten() {
            // The ether adapter's staged-TX retry FIRST (a ring-full
            // consume staged one frame; it retries before new work — the
            // gate's traffic never fills the 256-slot ring, so the retry
            // lands on the next drive's timer/file-op/RX-wake leg).
            core.dev.flush_tx();
            // THE LOOPBACK-FEEDBACK LOOP (and the ether queues' drain): a
            // TX on the loopback Device loops the packet INTO the device's
            // RX queue — egress produces ingress work.  Drive until the
            // queue drains (bounded: each round's ingress is capped at
            // RX_DRIVE_CAP and consumes the queue; the queue's own cap
            // bounds the total; the ether leg's queues are ring-bounded
            // the same way — ether TX never loops back, so no feedback
            // cycle exists there).  Without the loop a write's drive would
            // leave the looped packet un-ingested until the NEXT drive —
            // with all procs blocked, that drive never comes (the wedge
            // the 08-04 tracer exists to catch).
            loop {
                // Bounded ingress (P-D): at most RX_DRIVE_CAP packets
                // per round.
                for _ in 0..RX_DRIVE_CAP {
                    match core.iface.poll_ingress_single(now, &mut core.dev, &mut core.sockets) {
                        PollIngressSingleResult::None => break,
                        PollIngressSingleResult::PacketProcessed
                        | PollIngressSingleResult::SocketStateChanged => {}
                    }
                }
                core.iface.poll_egress(now, &mut core.dev, &mut core.sockets);
                if !core.dev.rx_pending() {
                    break;
                }
            }
            core.iface.poll_maintenance(now);
            // The close-linger reap (the donor's tcb-free at the close
            // handshake's end): a conv-closed tcp socket parked in the
            // linger list was driven by the polls above — once it reaches
            // State::Closed (the FIN acked / the close timer aged out —
            // poll_at below keeps the deadline armed while it lingers)
            // removal is safe.  BEFORE the poll_at read so a reaped
            // socket's timers stop counting now.
            {
                let mut i = 0;
                while i < core.linger.len() {
                    let h = core.linger[i];
                    if core.sockets.get::<tcp::Socket>(h).state() == tcp::State::Closed {
                        core.sockets.remove(h);
                        core.linger.swap_remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
            if let Some(at) = core.iface.poll_at(now, &core.sockets) {
                let t = instant_to_ticks(at);
                next = Some(next.map_or(t, |n| n.min(t)));
            }
        }
        match next {
            Some(t) => NET_DEADLINE.store(t, Ordering::Release),
            None => NET_DEADLINE.store(DEADLINE_DISARMED, Ordering::Release),
        }
    }
    // Wake AFTER the guard drops — never the wake hook while the cell is
    // held (the hook takes PROC_TABLE).
    let mut pids = [0u64; 16];
    let n = PENDING_WAKES.enter().expect("devip: pending wakes contended").take(&mut pids);
    if n > 0 && let Ok(h) = hooks() {
        (h.wake)(&pids[..n]);
    }
    if DRIVE_PRINT.load(Ordering::Relaxed) {
        let n = DRIVE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        crate::println!("devip: drive ok {n}");
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
/// serialize through a test lock and each gets a clean seam).  The ether
/// slot clears too (a pin never inherits a sibling pin's ether core).
#[cfg(test)]
pub(crate) fn install_test_stack(core: StackCore) {
    {
        let mut guard = STACK.enter().expect("devip: stack contended");
        guard.core = Some(core);
        guard.ether = None;
    }
    NET_DEADLINE.store(DEADLINE_DISARMED, Ordering::Release);
    STACK_DEBT.store(false, Ordering::Release);
    let mut scratch = [0u64; 16];
    PENDING_WAKES.enter().expect("devip: pending wakes contended").take(&mut scratch);
    NET_WAITERS.enter().expect("devip: waiters contended").take(&mut scratch);
}

/// Install the ETHER core for a pin (08-6 — the two-core pins: the demux
/// table + the scripted ether device's drives).  The loopback core from
/// install_test_stack stays.
#[cfg(test)]
pub(crate) fn install_test_ether(core: StackCore) {
    let mut guard = STACK.enter().expect("devip: stack contended");
    guard.ether = Some(core);
}

/// Enter the stack cell (pin surgery on the core under the real ownership
/// discipline — the pins drive the seam's actual cell, never a side copy).
/// The cell is a static, so the guard's lifetime is 'static.
#[cfg(test)]
pub(crate) fn enter_stack() -> crate::mcslock::BusyGuard<'static, StackState> {
    STACK.enter().expect("devip: stack contended")
}

// ---------------------------------------------------------------------------
// The accept pool (contract §12.3; Pitfall P-A).
// ---------------------------------------------------------------------------

/// The devip-exact accept-pool depth (devip `Maxincall`, contract §12.3):
/// 128 members × (2048 + 2048)-byte socket buffers = 512 KiB per announced
/// conv, allocated AT announce.  The pool manager's depth is a parameter so
/// the host pins run test-scaled pools — the demux shape is
/// depth-independent.
pub const ACCEPT_POOL_DEPTH: usize = 128;

/// The TCP socket buffer size per pool member (contract §12.3).
pub const POOL_SOCK_BUF: usize = 2048;

/// The accept pool: smoltcp has NO listen backlog — excess SYNs to a
/// listening socket are RSTed (docs.rs verbatim) — so devip's Maxincall
/// incall queue maps onto N smoltcp sockets all `listen(port)`.  smoltcp's
/// `accepts()` lets the FIRST still-Listen member absorb each SYN (its
/// tuple is unset); the tuple binds on the SynReceived transition, so the
/// next SYN falls through to the next member.  The listen-file open pops
/// the oldest connected member and the pool refills.
///
/// The pool's ops enter the STACK cell themselves (short acquisitions;
/// never hold the cell across a pool op).
pub struct AcceptPool {
    members: Vec<SocketHandle>,
    port: u16,
    depth: usize,
    /// The core the pool's sockets live in (the announcing conv's demux
    /// — 08-06's two-core seam: a pool is valid only in its own core's
    /// SocketSet).
    sel: CoreSel,
}

impl AcceptPool {
    /// devip's `announce <port>`: allocate `depth` listening sockets
    /// (production callers pass ACCEPT_POOL_DEPTH) in the demuxed core.
    /// On a mid-allocation failure the partial pool is torn down — no
    /// half-announced conv.
    pub(crate) fn announce(depth: usize, port: u16, sel: CoreSel) -> Result<AcceptPool> {
        let mut guard = STACK.enter().expect("devip: stack contended");
        let core = guard.core_mut(sel).ok_or("devip: no bound medium")?;
        let mut members = Vec::with_capacity(depth);
        for _ in 0..depth {
            match Self::spawn_listener(core, port) {
                Ok(h) => members.push(h),
                Err(e) => {
                    for h in members {
                        core.sockets.remove(h);
                    }
                    return Err(e);
                }
            }
        }
        Ok(AcceptPool { members, port, depth, sel })
    }

    /// Pop the OLDEST pool member that has left Listen (an incoming call
    /// landed — SynReceived and beyond).  devip's listen-file open consumes
    /// the returned handle's conv; the caller refills the pool after.
    /// Members whose handshake died mid-way (RST → Closed) are PRUNED here
    /// (they can never materialize; the slot returns to the pool via the
    /// caller's refill).
    pub fn take_connected(&mut self) -> Option<SocketHandle> {
        let mut guard = STACK.enter().expect("devip: stack contended");
        let core = guard.core_mut(self.sel)?;
        self.prune_closed(core);
        let pos = self.members.iter().position(|&h| {
            let s = core.sockets.get::<tcp::Socket>(h);
            s.state() != tcp::State::Listen && s.is_open()
        })?;
        Some(self.members.remove(pos))
    }

    /// Top the pool back up to its depth with fresh Listen sockets (after
    /// pruning dead members — a SYN RSTed mid-handshake leaves a Closed
    /// socket that must not count against the depth).
    pub fn refill(&mut self) -> Result<()> {
        let mut guard = STACK.enter().expect("devip: stack contended");
        let core = guard.core_mut(self.sel).ok_or("devip: no bound medium")?;
        self.prune_closed(core);
        while self.members.len() < self.depth {
            let h = Self::spawn_listener(core, self.port)?;
            self.members.push(h);
        }
        Ok(())
    }

    /// Remove fully-Closed members from the pool and the SocketSet (the
    /// T-08-09 hygiene: an RSTed half-open never occupies a pool slot
    /// forever).
    fn prune_closed(&mut self, core: &mut StackCore) {
        let mut i = 0;
        while i < self.members.len() {
            let h = self.members[i];
            if core.sockets.get::<tcp::Socket>(h).state() == tcp::State::Closed {
                core.sockets.remove(h);
                self.members.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Tear the pool down: abort every member (RST where a handshake is in
    /// flight; Listen members close silently) and remove the sockets from
    /// the seam's SocketSet.  A dropped pool NEVER leaves live sockets
    /// behind.  (The donor's listener close FINs its in-progress incall
    /// convs; the abort-instead is a recorded mapping corner — an unpopped
    /// member has no conv to carry a graceful close.)
    pub fn teardown(mut self) {
        let mut guard = STACK.enter().expect("devip: stack contended");
        if let Some(core) = guard.core_mut(self.sel) {
            for h in self.members.drain(..) {
                core.sockets.get_mut::<tcp::Socket>(h).abort();
                core.sockets.remove(h);
            }
        }
    }

    /// Allocate one listening pool member: 2048/2048 buffers (contract
    /// §12.3) and hop limit 255 (contract §12.8 — devip's MAXTTL; smoltcp's
    /// 64 default must never leak).
    fn spawn_listener(core: &mut StackCore, port: u16) -> Result<SocketHandle> {
        let mut s = tcp::Socket::new(
            tcp::SocketBuffer::new(alloc::vec![0; POOL_SOCK_BUF]),
            tcp::SocketBuffer::new(alloc::vec![0; POOL_SOCK_BUF]),
        );
        s.set_hop_limit(Some(255));
        s.listen(port).map_err(|_| "devip: listen failed")?;
        Ok(core.sockets.add(s))
    }

    /// The pool's current members (the listen open arms their recv wakers
    /// on the parked pid; the pins assert on the set).
    pub(crate) fn members(&self) -> &[SocketHandle] {
        &self.members
    }

    /// Count of members still in Listen (pin assertions).
    #[cfg(test)]
    pub(crate) fn listening_count(&self) -> usize {
        let mut guard = STACK.enter().expect("devip: stack contended");
        let Some(core) = guard.core_mut(self.sel) else { return 0 };
        self.members
            .iter()
            .filter(|&&h| core.sockets.get::<tcp::Socket>(h).state() == tcp::State::Listen)
            .count()
    }

    /// All members in Listen (pin assertions).
    #[cfg(test)]
    pub(crate) fn all_listening(&self) -> bool {
        !self.members.is_empty() && self.listening_count() == self.members.len()
    }
}
