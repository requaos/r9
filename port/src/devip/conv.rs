//! devip/conv.rs — the conversation model (contract §3; donor `ip.h`'s
//! `Conv` + `devip.c`'s lifecycle).  One fixed table per protocol
//! (const-new pattern — no allocation in the table itself), clone/open/
//! ref/close with the donor's semantics:
//!
//! - `conv_alloc` (Fsprotoclone): first free slot, full field reset,
//!   gc under pressure = drop fully-Closed convs' sockets (the donor's
//!   gc hook maps to that in the smoltcp era); exhaustion → Enodev
//!   ("no free devices" — the donor's cpuserver panic arm is absent:
//!   r9 has no cpuserver flag, the error is always Enodev).
//! - `conv_open` (devip.c:417-444): inuse++; the first ctl/data/err
//!   open re-affirms the owner collapse (single-user era: the donor's
//!   iseve/owner perm checks collapse — same shape as devether).
//! - `conv_close` (closeconv, devip.c:546-577): last close resets the
//!   conv — pending incall convs closed, state = Idle, queues drained,
//!   socket dropped from the seam's SocketSet, and any blocked readers
//!   woken (they re-check and read EOF — the donor's qclose arm).  The
//!   wake hook call is AFTER the table guard drops (netif's
//!   wake-after-drop discipline).
//! - The `Chan::dup` ref-bump (dat.rs's DEV_IP arm → `conv_ref`):
//!   rfork/dup'd fds must not double-decrement the conv at close.
//!
//! LOCKING: `TABLES` is an mcslock taken in SYSCALL context only, a LEAF
//! with ONE documented nesting — TABLES → STACK (a conv op may enter the
//! seam's cell while holding the table; the seam NEVER takes TABLES:
//! drive_stack/waker bridge touch only seam statics).  Neither lock is
//! ever held across `block()`: the read path's register → drop → block
//! shape is the devpipe invariant (wakeup-loss-free because syscalls run
//! IRQ-masked — the waker can only run after block() swtch'es).

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use smoltcp::iface::SocketHandle;

use crate::mcslock::{Lock, LockNode};
use crate::Result;

/// The five conv states (`ip.h:71-75` verbatim).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ConvState {
    Idle,
    Announcing,
    Announced,
    Connecting,
    Connected,
}

/// devip's MAXTTL (contract §4/§12.8 — `ip.h:44` is 255, NOT the
/// research's "64" parenthetical; smoltcp's hop-limit default 64 must
/// never leak: sockets get 255 at creation).
pub(crate) const MAX_TTL: u32 = 255;

/// The per-proto conv-table size (fixed static tables — the donor
/// malloc's convs on demand up to Nchans=1024; 16 per proto is the
/// era's generous bound: the gates hold ≤ a handful open at once).
/// Clone exhaustion past the cap is Enodev (contract §3).
pub(crate) const NCONV: usize = 16;

/// Protocol indices — the donor's registration order (9k k10cpu's
/// `ip +dev` section: tcp, udp, ipifc, icmp, icmp6), which fixes both
/// the QID packing's proto field and the /net listing order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Proto {
    Tcp,
    Udp,
    Ipifc,
    Icmp,
    Icmp6,
}

impl Proto {
    pub(crate) const ALL: [Proto; 5] =
        [Proto::Tcp, Proto::Udp, Proto::Ipifc, Proto::Icmp, Proto::Icmp6];

    pub(crate) fn index(self) -> u64 {
        match self {
            Proto::Tcp => 0,
            Proto::Udp => 1,
            Proto::Ipifc => 2,
            Proto::Icmp => 3,
            Proto::Icmp6 => 4,
        }
    }

    pub(crate) fn from_index(i: u64) -> Option<Proto> {
        match i {
            0 => Some(Proto::Tcp),
            1 => Some(Proto::Udp),
            2 => Some(Proto::Ipifc),
            3 => Some(Proto::Icmp),
            4 => Some(Proto::Icmp6),
            _ => None,
        }
    }

    /// The directory name in /net.
    pub(crate) fn name(self) -> &'static [u8] {
        match self {
            Proto::Tcp => b"tcp",
            Proto::Udp => b"udp",
            Proto::Ipifc => b"ipifc",
            Proto::Icmp => b"icmp",
            Proto::Icmp6 => b"icmpv6",
        }
    }
}

/// A per-conv blocked-reader pid set (fixed capacity: one waiter per
/// proc, MAX_PROCS=16 slots can never fill; pid 0 marks an empty slot;
/// NO allocation — the wake drain moves pids through a caller's stack
/// buffer).  The CONV — not the waker — owns the waiter set (contract
/// §12.2, P-C).
pub(crate) struct WaitPids {
    pids: [u64; 16],
}

impl WaitPids {
    const fn new() -> WaitPids {
        WaitPids { pids: [0; 16] }
    }

    pub(crate) fn register(&mut self, pid: u64) {
        if pid == 0 || self.pids.contains(&pid) {
            return;
        }
        if let Some(slot) = self.pids.iter_mut().find(|p| **p == 0) {
            *slot = pid;
        }
        // The full case is impossible (one waiter per proc, ≤ 16 procs).
    }

    /// Drain the set into a stack buffer (the wake hook's input) —
    /// returns the count; every drained slot clears.
    pub(crate) fn take(&mut self, out: &mut [u64; 16]) -> usize {
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

/// One conversation (the donor's `Conv`, `ip.h:170-221`).  The smoltcp
/// socket handle indexes the seam's SocketSet; it exists only once the
/// conv is connected/announced (the endpoint is known then) — an Idle
/// conv has no socket.
pub(crate) struct Conv {
    /// Open reference count (the donor's inuse; clone sets 1,
    /// ctl/data/err open and Chan::dup bump, close decrements).
    pub(crate) inuse: usize,
    pub(crate) state: ConvState,
    /// The conv files' mode (0660 from clone per Fsprotoclone; the
    /// donor's ipwstat could change it — wstat is loud-None this era,
    /// so the field is a constant surface).
    pub(crate) perm: u32,
    /// 16-byte IPv6-form addresses (v4 mapped under v4prefix).
    pub(crate) laddr: [u8; 16],
    pub(crate) raddr: [u8; 16],
    pub(crate) lport: u16,
    pub(crate) rport: u16,
    pub(crate) ttl: u32,
    pub(crate) tos: u32,
    /// `connect …!r` — restricted local port (600..1024 cycling).
    pub(crate) restricted: bool,
    /// udp: don't break the conv on ICMP errors (the advise gate).
    pub(crate) ignoreadvice: bool,
    /// 0 = off; 7 = udp's 52-byte Udphdr prefix mode; 6 = icmpv6's
    /// 40-byte prefix mode.  Written by udp's `headers` ctl (08-04
    /// Task 2) / icmpv6's (08-06); read by their data paths then.
    pub(crate) headers: u8,
    pub(crate) maxfragsize: u32,
    /// The conv's smoltcp socket (the seam's SocketSet), once connected
    /// or announced.
    pub(crate) sock: Option<SocketHandle>,
    /// The per-conv error queue (Qmsg — one message per read).  UDP
    /// ICMP advice hangups land here (§6); read blocks when empty.
    pub(crate) errq: VecDeque<Vec<u8>>,
    /// The incoming-call queue (Fsnewcall-spawned conv ids) for an
    /// announced conv — udp (08-04 Task 2) and tcp (08-05) fill it;
    /// the listen-file open pops it.
    pub(crate) incall: VecDeque<usize>,
    /// Blocked readers/writers of THIS conv (the closeconv wake set;
    /// a wedged reader's pid here never touches another conv — the
    /// NETIP-02 isolation shape).
    pub(crate) waiters: WaitPids,
    /// Multicast memberships recorded by addmulti (the donor's
    /// c->multi list): (mcast addr, ifc addr) pairs.  The loopback era
    /// has no multicast delivery machinery — the record is the honest
    /// file-level state (delivery arrives with the ether leg, 08-06).
    pub(crate) mcast: Vec<([u8; 16], [u8; 16])>,
    /// Ever allocated (the donor's permanent-slot shape: a cloned conv
    /// keeps its slot — and its listing entry — after close; conv_alloc
    /// reuses the slot but the mark persists).
    pub(crate) allocated: bool,
    /// udp's Fsnewcall: the ANNOUNCED conv that spawned this one (None
    /// for clone-allocated convs).  The spawned conv's datagrams arrive
    /// on the announcer's socket — its read drains that socket first.
    pub(crate) announcer: Option<usize>,
    /// udp's devip-side datagram queue (the donor's per-conv rq):
    /// datagrams drained off the announcer's socket land here for
    /// spawned convs.  (Headers-mode and connected convs read their
    /// smoltcp socket directly — no queue.)
    pub(crate) dataq: VecDeque<Vec<u8>>,
    /// udp's advise hangup (udpadvise's qhangup): set by
    /// proto::udp_advise unless ignoreadvice; data reads/writes fail
    /// while set (sticky until close — the donor's qhangup shape).
    pub(crate) hangup: Option<Vec<u8>>,
}

impl Conv {
    const fn new() -> Conv {
        Conv {
            inuse: 0,
            state: ConvState::Idle,
            perm: 0o660,
            laddr: [0; 16],
            raddr: [0; 16],
            lport: 0,
            rport: 0,
            ttl: MAX_TTL,
            tos: 0,
            restricted: false,
            ignoreadvice: false,
            headers: 0,
            maxfragsize: 0,
            sock: None,
            errq: VecDeque::new(),
            incall: VecDeque::new(),
            waiters: WaitPids::new(),
            mcast: Vec::new(),
            allocated: false,
            announcer: None,
            dataq: VecDeque::new(),
            hangup: None,
        }
    }

    /// The clone-time reset (Fsprotoclone, devip.c:1324-1339): every
    /// field back to its freshly-allocated value, queues reopened
    /// (drained) for the next cloner.  The `allocated` mark survives
    /// (the donor's conv slots are permanent once allocated).
    fn reset(&mut self) {
        let was = self.allocated;
        *self = Conv::new();
        self.allocated = was;
        self.inuse = 1;
    }

    /// The close-time reset (closeconv): same surface, inuse stays 0,
    /// the allocated mark survives.
    fn close_reset(&mut self) {
        let was = self.allocated;
        *self = Conv::new();
        self.allocated = was;
    }
}

/// The per-proto tables + the restricted-port cyclers (donor's
/// `p->nextrport`, one per proto).
pub(crate) struct ConvTables {
    pub tcp: [Conv; NCONV],
    pub udp: [Conv; NCONV],
    pub ipifc: [Conv; NCONV],
    pub icmp: [Conv; NCONV],
    pub icmp6: [Conv; NCONV],
    nextrport: [u16; 5],
}

impl ConvTables {
    const fn new() -> ConvTables {
        ConvTables {
            tcp: [const { Conv::new() }; NCONV],
            udp: [const { Conv::new() }; NCONV],
            ipifc: [const { Conv::new() }; NCONV],
            icmp: [const { Conv::new() }; NCONV],
            icmp6: [const { Conv::new() }; NCONV],
            nextrport: [600; 5],
        }
    }

    pub(crate) fn table_mut(&mut self, p: Proto) -> &mut [Conv; NCONV] {
        match p {
            Proto::Tcp => &mut self.tcp,
            Proto::Udp => &mut self.udp,
            Proto::Ipifc => &mut self.ipifc,
            Proto::Icmp => &mut self.icmp,
            Proto::Icmp6 => &mut self.icmp6,
        }
    }

    pub(crate) fn table(&self, p: Proto) -> &[Conv; NCONV] {
        match p {
            Proto::Tcp => &self.tcp,
            Proto::Udp => &self.udp,
            Proto::Ipifc => &self.ipifc,
            Proto::Icmp => &self.icmp,
            Proto::Icmp6 => &self.icmp6,
        }
    }
}

/// THE conv tables.  Syscall-context mcslock, LEAF with the single
/// TABLES → STACK nesting (see the module header).  Lock it with the
/// in-tree two-line idiom (`let node = LockNode::new(); let mut t =
/// TABLES.lock(&node);`) — the guard borrows the node, so no helper
/// fn can return it.
pub(crate) static TABLES: Lock<ConvTables> = Lock::new("devipconv", ConvTables::new());

/// The clone-open allocation (Fsprotoclone): first slot with
/// inuse == 0; a gc pass under pressure (drop fully-Closed convs'
/// sockets — the smoltcp-era mapping of the donor's p->gc) and retry;
/// exhaustion → Enodev.  Returns the conv id; the caller (devip::open)
/// rewrites the chan's qid to QID(p, id, Qctl).
pub(crate) fn conv_alloc(p: Proto) -> Result<usize> {
    let node = LockNode::new();
    let mut t = TABLES.lock(&node);
    let table = t.table_mut(p);
    for (id, c) in table.iter_mut().enumerate() {
        if c.inuse == 0 {
            c.reset();
            c.allocated = true;
            return Ok(id);
        }
    }
    // The gc pass under pressure is tcp's (the donor's tcpgc reaps
    // timed-out convs; icmp/udp have no gc — p->gc is nil and the
    // donor falls straight to the Enodev arm).  08-05 lands it.
    Err("no free devices")
}

/// The allocated-slot map for the walk/listing (the donor lists conv
/// dirs for every allocated slot regardless of inuse).
pub(crate) fn conv_allocated(p: Proto) -> [bool; NCONV] {
    let node = LockNode::new();
    let t = TABLES.lock(&node);
    let mut out = [false; NCONV];
    for (i, c) in t.table(p).iter().enumerate() {
        out[i] = c.allocated;
    }
    out
}

/// Fsnewcall (devip.c:1385-1426): clone a conv off the announced one to
/// carry an incoming datagram's quad — raddr/rport from the sender,
/// laddr/lport from the announcer, state Connected, appended to the
/// announcer's incall queue.  Returns the new conv's id; None on table
/// exhaustion (the donor drops the datagram on a full incall — the
/// caller counts/logs it).  The caller holds the table lock (the udp
/// RX drain's TABLES → STACK nesting discipline: spawn+dispatch happen
/// under TABLES; the wake runs after it drops).  The donor's
/// Maxincall=128 cap never binds before this era's 16-slot table does.
pub(crate) fn fs_new_call(
    t: &mut ConvTables,
    p: Proto,
    announcer: usize,
    raddr: [u8; 16],
    rport: u16,
    laddr: [u8; 16],
    lport: u16,
) -> Option<usize> {
    let id = {
        let table = t.table(p);
        let mut free = None;
        for (id, c) in table.iter().enumerate() {
            if c.inuse == 0 {
                free = Some(id);
                break;
            }
        }
        free?
    };
    {
        let c = &mut t.table_mut(p)[id];
        c.reset();
        c.allocated = true;
        c.raddr = raddr;
        c.rport = rport;
        c.laddr = laddr;
        c.lport = lport;
        c.state = ConvState::Connected;
        c.announcer = Some(announcer);
    }
    t.table_mut(p)[announcer].incall.push_back(id);
    Some(id)
}

/// Re-open a conv's file (devip.c:417-444): inuse++, re-affirming the
/// owner collapse; a closed-but-allocated conv is REOPENED in its
/// closeconv-reset state (the donor's ipopen Qctl/Qdata arm has no
/// allocated check — the slot persists and inuse 0→1 revives it).
/// Enodev-shaped when the slot was never clone-allocated (unreachable
/// via walk; defensive).
pub(crate) fn conv_open(p: Proto, id: usize) -> Result<()> {
    let node = LockNode::new();
    let mut t = TABLES.lock(&node);
    let table = t.table_mut(p);
    if id >= NCONV || !table[id].allocated {
        return Err("no free devices");
    }
    if table[id].inuse == 0 {
        table[id].reset();
        table[id].allocated = true;
    } else {
        table[id].inuse += 1;
    }
    Ok(())
}

/// dat.rs Chan::dup's bump for an open conv file (the devether
/// ref_open precedent): every value copy of an OPEN conv file is a new
/// open reference, or rfork/dup'd fds would double-decrement the conv
/// at close.
pub(crate) fn conv_ref(p: Proto, id: usize) {
    if id >= NCONV {
        return;
    }
    let node = LockNode::new();
    let mut t = TABLES.lock(&node);
    let table = t.table_mut(p);
    if table[id].inuse > 0 {
        table[id].inuse += 1;
    }
}

/// The last-close reset (closeconv, devip.c:546-577): inuse--; at zero
/// the conv resets — pending incall convs closed, socket dropped from
/// the seam's SocketSet, queues drained, state = Idle — and blocked
/// readers wake to EOF (the donor's qclose → qwait-returns-0 arm).
/// The wake hook runs AFTER the table guard drops (never across
/// PROC_TABLE).
pub(crate) fn conv_close(p: Proto, id: usize) {
    if id >= NCONV {
        return;
    }
    let mut pids = [0u64; 16];
    let nwake;
    {
        let node = LockNode::new();
        let mut t = TABLES.lock(&node);
        // The decrement + last-close test.
        {
            let c = &mut t.table_mut(p)[id];
            if c.inuse > 0 {
                c.inuse -= 1;
            }
            if c.inuse != 0 {
                return;
            }
        }
        // TABLES → STACK (the documented nesting): drop the conv's
        // socket while the table guard is held.
        let sock = {
            let table = t.table_mut(p);
            let (incall, sock) = {
                let c = &mut table[id];
                nwake = c.waiters.take(&mut pids);
                (core::mem::take(&mut c.incall), c.sock.take())
            };
            // Close pending incall convs (no listen will ever happen —
            // closeconv's incall sweep): their last-close shape runs
            // inline (idle tables, no fds reference them).
            for nid in incall {
                table[nid].close_reset();
            }
            sock
        };
        // The proto close hook BEFORE the reset (the donor's
        // closeconv order: `cv->p->close(cv); cv->state = Idle`).
        // ipifc's unbind-on-close discipline is the medium's flag —
        // loopback has unbindonclose = 0, so the binding PERSISTS
        // across ctl close; see ipifc.rs.
        if p == Proto::Ipifc {
            super::ipifc::close_conv(&mut t, id);
        }
        // Drop the conv's socket from the seam's SocketSet (the
        // TABLES → STACK nesting — the cell is entered, the socket
        // removed, and the guard dropped, all under the table guard).
        if let Some(h) = sock {
            let mut g = super::stack::STACK.enter().expect("devip: stack contended");
            if let Some(core) = g.core.as_mut() {
                core.sockets.remove(h);
            }
        }
        // Reset to the clone-time surface (closeconv: state = Idle,
        // owner = "network" — collapsed this era — perm 0660, queues
        // drained/reopened for the next cloner; the allocated mark
        // survives).
        t.table_mut(p)[id].close_reset();
    }
    if nwake > 0
        && let Ok(h) = super::stack::hooks()
    {
        (h.wake)(&pids[..nwake]);
    }
}

/// Wake the conv's blocked readers (connect/announce completion and the
/// proto's hangup paths — a reader that blocked before the socket
/// existed re-checks and registers on the now-real socket).
pub(crate) fn wake_conv(p: Proto, id: usize) {
    let mut pids = [0u64; 16];
    let n = {
        let node = LockNode::new();
        let mut t = TABLES.lock(&node);
        t.table_mut(p)[id].waiters.take(&mut pids)
    };
    if n > 0
        && let Ok(h) = super::stack::hooks()
    {
        (h.wake)(&pids[..n]);
    }
}

/// Is the lport in use by another conv of this proto (donor's
/// lportinuse)?
fn lport_in_use(table: &[Conv; NCONV], lport: u16) -> bool {
    table.iter().any(|c| c.inuse > 0 && c.lport == lport)
}

/// Pick a local port (setlport, devip.c:778-825): restricted convs
/// cycle 600..1024; unrestricted pick a RANDOM port in [32768, 65536)
/// ([man-vs-src]: the man's "starting at 5000" is not the source).
/// Retries are bounded like the donor's 64 probes; the table's 16 slots
/// can never exhaust the range.
pub(crate) fn setlport(t: &mut ConvTables, p: Proto, id: usize) -> Result<()> {
    let table_idx = p.index() as usize;
    if t.table(p)[id].restricted {
        // Restricted ports cycle 600..1024 (the donor's nextrport).
        for _ in 0..(1024 - 600) {
            let cand = if !(600..1024).contains(&t.nextrport[table_idx]) { 600 } else { t.nextrport[table_idx] };
            t.nextrport[table_idx] = cand + 1;
            if !lport_in_use(t.table(p), cand) {
                t.table_mut(p)[id].lport = cand;
                return Ok(());
            }
        }
        return Err("no free devices");
    }
    for _ in 0..64 {
        let cand = 32768 + (prng_next() % 32768) as u16;
        if !lport_in_use(t.table(p), cand) {
            t.table_mut(p)[id].lport = cand;
            return Ok(());
        }
    }
    Err("no free devices")
}

/// The quad-uniqueness check (setluniqueport, devip.c:731-759): another
/// Connected/Announced conv with the same quad → "address in use".
pub(crate) fn setluniqueport(t: &mut ConvTables, p: Proto, id: usize, lport: u16) -> Result<()> {
    let (raddr, rport, laddr) = {
        let c = &t.table(p)[id];
        (c.raddr, c.rport, c.laddr)
    };
    for (x, xp) in t.table(p).iter().enumerate() {
        if x == id {
            continue;
        }
        if xp.inuse > 0
            && matches!(xp.state, ConvState::Connected | ConvState::Announced)
            && xp.lport == lport
            && xp.rport == rport
            && xp.raddr == raddr
            && xp.laddr == laddr
        {
            return Err("address in use");
        }
    }
    t.table_mut(p)[id].lport = lport;
    Ok(())
}

/// The local-port PRNG (the donor's nrand): xorshift64* seeded from the
/// seam's tick clock — crypto strength is irrelevant (port pick only),
/// and D-17's real CSPRNG lands in 08-06 for the gate that needs it.
fn prng_next() -> u64 {
    use core::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut x = STATE.load(Ordering::Relaxed);
    if x == 0 {
        x = super::stack::ticks_for_seed() ^ 0x9e37_79b9_7f4a_7c15;
        if x == 0 {
            x = 0x2545_f491_4f6c_dd1d;
        }
    }
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    STATE.store(x, Ordering::Relaxed);
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

/// The conv-number read content (devip.c:642-647's `"%lud"` — PLAIN
/// decimal, no padding; NOT devether's NUMSIZE-padded readnum).
pub(crate) fn ctl_number_content(id: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut v = id;
    if v == 0 {
        out.push(b'0');
    }
    while v > 0 {
        out.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    out
}
