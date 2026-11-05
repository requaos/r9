//! netif — the Netif/Netfile model under devether `#l` (NETDRV-03):
//! the qid scheme, the conversation table, the counter set, the
//! per-conversation inbound frame queues, and the IRQ↔syscall handoff
//! machinery.  Semantics ported from 9k's `sys/src/9k/port/netif.h` +
//! `netif.c` and 9/pc's `devether.c` (the donors); this module is the
//! MODEL half — the `#l` device's file surface (attach/walk/open/stat/
//! read/write/close) lives in `devether.rs`.
//!
//! Deltas from the donors (documented per the devpipe header discipline):
//!
//! - Single-user ownership collapse (the plan's locked shape): the
//!   donor's `netown` per-user permission check collapses to
//!   first-opener-owns — every proc runs as glenda this era, so the
//!   check can never fail; the stat output keeps the donor's byte shape
//!   (0666/glenda before the first open, 0660/glenda after).
//! - No multicast table (the donor's Netaddr hash): `addmulti`/
//!   `remmulti` are loud-unimplemented in devether, so `activemulti`
//!   is always false and non-broadcast multicasts are never delivered
//!   (qemu's RX filter without CTRL_VQ delivers only our MAC + broadcast
//!   anyway — the filter question is moot on the gates).
//! - No kprocs and no `etheroq` → `etheriq` TX loopback/broadcast
//!   feedback this phase (nothing in the ARP gate needs it) — Phase 8's
//!   arp/ethermedium binding EXPECTS it and must add it then; the
//!   omission is deliberate and loud (07-07 must_haves).
//! - The blocking data read (netif's `qread(f->iq, …)`, Qmsg mode) is
//!   complete and correct here, but NO Phase-7 gate ever blocks on it
//!   (RESEARCH OQ-1's disposition: the sweep writes, never reads; the
//!   ARP gate's RX proof is the stats counter + the host pcap).  It is
//!   exercised only by smoke-level use; the gates' omission of it is
//!   recorded here per the plan's loud-omission rule.
//!
//! Qid scheme (netif.h:5-28, VERBATIM): the low 5 bits are the tag
//! (Ncloneqid=1, Naddrqid, N2ndqid, N3rdqid, Ndataqid, Nctlqid,
//! Nstatqid, Ntypeqid, Nifstatqid), `NETQID(i,t) = (i<<5)|t` — the same
//! low-5-bits-tag shape devpipe uses.  NOTE the donor's structural
//! aliasing: NETQID(0,t) == t, so conversation 0's `stats`/`ifstats`
//! share qid paths with the ether0-level files of the same tag — the
//! donor resolves this by making Nstatqid/Nifstatqid interface-level
//! WHEREVER they appear (netifread serves `nif->` counters regardless
//! of NETID); we reproduce that exactly.
//!
//! LOCKING (the load-bearing invariants):
//!
//! - `NETIF` is an mcslock serializing all SYSCALL-context devether
//!   model ops (open/close/ctl/listing).  It is a LEAF lock: never held
//!   across block()/sched(), never taken from IRQ context.  The ONE
//!   permitted nesting is NETIF → INGRESS (the lifecycle ops write the
//!   canonical record and its IRQ-side mirror in one critical section);
//!   INGRESS → NETIF never happens, and the IRQ side never touches
//!   NETIF, so the nesting cannot deadlock.
//! - `INGRESS` is a `BusyCell` (mcslock.rs's non-spinning sibling) —
//!   the plan's sanctioned "small lock-free handoff": the IRQ retire
//!   (the only producer) and the data-file read (the only consumer)
//!   both `enter()` it.  On this kernel the two contexts are mutually
//!   exclusive in TIME (syscalls run IRQ-masked; the handler runs only
//!   from EL0 or the sched idle wfi — the Phase-4 invariant), so
//!   `enter()` is provably uncontended; the BusyCell makes a violation
//!   of that invariant a fail-fast None (accounted as a soverflow drop)
//!   instead of a spin-deadlock.  INGRESS is never held across
//!   block()/sched() either (the read path enters, checks, registers a
//!   waiter, DROPS, then blocks — devpipe's register-under-lock →
//!   drop-lock → block() shape, with the BusyCell in the lock's place).
//!   The IRQ-side wake call happens AFTER the guard drops (INGRESS is
//!   never held while the wake hook takes PROC_TABLE).
//! - No allocation happens in IRQ context: each conversation's frame
//!   store is preallocated at clone-open (syscall context) — the
//!   allocator's lock is a syscall-context lock too.
//!
//! smp=1 this era — there is no cross-CPU sharing to model.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::mcslock::{BusyCell, Lock, LockNode};
use crate::Result;

// ---------------------------------------------------------------------------
// Qid scheme (netif.h:5-28 verbatim).  The tags are the low 5 bits of the
// qid path; the conversation id sits above them.
// ---------------------------------------------------------------------------

pub(crate) const NCLONEQID: u64 = 1;
pub(crate) const NADDRQID: u64 = 2;
pub(crate) const N2NDQID: u64 = 3;
pub(crate) const N3RDQID: u64 = 4;
pub(crate) const NDATAQID: u64 = 5;
pub(crate) const NCTLQID: u64 = 6;
pub(crate) const NSTATQID: u64 = 7;
pub(crate) const NTYPEQID: u64 = 8;
pub(crate) const NIFSTATQID: u64 = 9;

/// NETQID(i,t) = (i<<5)|t.
pub(crate) fn netqid(i: u64, t: u64) -> u64 {
    (i << 5) | t
}

/// NETTYPE(x) = x & 0x1f.
pub(crate) fn nettype(p: u64) -> u64 {
    p & 0x1f
}

/// NETID(x) = x >> 5.
pub(crate) fn netid(p: u64) -> u64 {
    p >> 5
}

/// The conversation-table size (the donor etherif.h's `Ntypes`).
pub(crate) const NFILE: usize = 8;

/// The donor's NUMSIZE (portdat.h:780): a number file's fixed width —
/// the value right-justified in 11 columns plus a trailing space.
pub(crate) const NUMSIZE: usize = 12;

// ---------------------------------------------------------------------------
// The syscall-context model state (NETIF — the LEAF mcslock above).
// ---------------------------------------------------------------------------

/// Per-conversation bookkeeping (the donor's `Netfile`): the open
/// reference count (`inuse`), the ownership collapse record, and the
/// canonical demux fields (`typ`/`prom` — mirrored into INGRESS for the
/// IRQ retire; the two are written together in one syscall critical
/// section, so they can never disagree).
pub(crate) struct ConvBook {
    pub(crate) inuse: usize,
    /// First-opener-owns (the single-user netown collapse): set by the
    /// first data/ctl open; the stat/listing modes read 0660 once set,
    /// 0666 before (the donor's byte-identical surface).
    pub(crate) owner_set: bool,
    /// The ether-type filter (`connect N`; -1 = all).  0 is the unset
    /// state and matches no real ethertype.
    pub(crate) typ: i32,
    pub(crate) prom: bool,
}

impl ConvBook {
    const fn new() -> ConvBook {
        ConvBook { inuse: 0, owner_set: false, typ: 0, prom: false }
    }
}

/// The interface record (the donor's `Netif`): one per bound NIC —
/// a single `ether0` this era (attach's controller-number parse admits
/// unit 0 only, matching the single virtio-net device qemu presents).
/// The IRQ-side mirrors (addr/bcast) live in INGRESS — the model's
/// copies are read by the file surface only.
pub(crate) struct Netif {
    /// The level-2 directory name ("ether0").
    pub(crate) name: &'static [u8],
    /// Address length (Eaddrlen = 6).
    pub(crate) alen: usize,
    /// The negotiated MAC (the addr file's content + the write path's
    /// source-MAC overwrite).
    pub(crate) addr: [u8; 6],
    /// devether.c:387-389: minmtu=ETHERMINTU(60), mtu=maxmtu=ETHERMAXTU(1514).
    pub(crate) mtu: u32,
    pub(crate) minmtu: u32,
    /// The stats file's `mbps:`/`link:` fields (a bound virtio-net is
    /// 1000 Mbps, link up — slirp's link never changes).
    pub(crate) mbps: u32,
    pub(crate) link: u32,
    /// Count of conversations in promiscuous mode (the stats `prom:`
    /// field + the donor's multicast-filter input).
    pub(crate) prom: u32,
    pub(crate) convs: [ConvBook; NFILE],
}

/// The interface state: None until the driver bind publishes (the
/// devcons HOSTOWNER static-publish idiom; attach's Enodev path reads
/// absence here).
pub(crate) static NETIF: Lock<Option<Netif>> = Lock::new("netif", None);

/// Publish the interface record at driver bind (devether::publish_driver
/// is the only caller; boot context, before the first attach).
pub(crate) fn publish(mac: [u8; 6]) {
    let node = LockNode::new();
    let mut guard = NETIF.lock(&node);
    assert!(guard.is_none(), "netif: interface published twice");
    *guard = Some(Netif {
        name: b"ether0",
        alen: 6,
        addr: mac,
        mtu: ETHERMAXTU as u32,
        minmtu: ETHERMINTU as u32,
        mbps: 1000,
        link: 1,
        prom: 0,
        convs: [const { ConvBook::new() }; NFILE],
    });
    // The IRQ-side demux reads the addresses from INGRESS (never the
    // NETIF lock) — mirror them now, before any frame can arrive.
    let mut ig = INGRESS.enter().expect("netif: publish with INGRESS contended (boot bug)");
    ig.addr = mac;
    ig.bcast = [0xff; 6];
}

// ---------------------------------------------------------------------------
// The IRQ↔syscall handoff (INGRESS — the BusyCell discipline above).
// ---------------------------------------------------------------------------

/// ETHERMINTU / ETHERMAXTU (9k's etherif.h:4-5) — the write path's
/// bounds and the frame-store slot size.
pub(crate) const ETHERMINTU: usize = 60;
pub(crate) const ETHERMAXTU: usize = 1514;

/// The per-conversation inbound queue bound (RESEARCH OQ-3): a fixed
/// 64 KiB frame store per open conversation.  Whole frames only (Qmsg);
/// a full store drops the inbound frame and counts a soverflow (the
/// donor's qpass-failure arm).
const CONV_STORE_CAP: usize = 64 * 1024;
const FRAME_SLOTS: usize = CONV_STORE_CAP / ETHERMAXTU; // 43

/// One conversation's preallocated frame store: `slots × ETHERMAXTU`
/// bytes of slot storage plus the free stack and the (slot, len) FIFO.
/// The FIFO is the donor's `Queue*` in Qmsg mode: one WHOLE frame per
/// pop (a partial-frame read is a defect, never a behavior — the plan's
/// NETDRV-03 prohibition).
struct FrameStore {
    bytes: Vec<u8>,
    /// Free slot indices (a stack; every slot starts free).
    free: Vec<u16>,
    /// Queued frames as (slot, len) pairs — FIFO, whole frames.
    queue: VecDeque<(u16, u16)>,
}

impl FrameStore {
    fn new() -> FrameStore {
        FrameStore {
            bytes: alloc::vec![0u8; FRAME_SLOTS * ETHERMAXTU],
            free: (0..FRAME_SLOTS as u16).rev().collect(),
            queue: VecDeque::new(),
        }
    }

    /// Push one whole frame: false when the store is absent (a closed
    /// conversation — impossible for a demux match, counted defensively
    /// by the caller) or full (the soverflow arm).
    fn push(&mut self, frame: &[u8]) -> bool {
        let Some(slot) = self.free.pop() else { return false };
        let n = frame.len().min(ETHERMAXTU);
        let base = slot as usize * ETHERMAXTU;
        self.bytes[base..base + n].copy_from_slice(&frame[..n]);
        self.queue.push_back((slot, n as u16));
        true
    }

    /// Pop one whole frame (Qmsg): the frame is CONSUMED; when the
    /// caller's buffer is shorter, the excess is discarded (the donor
    /// qread's Qmsg arm: `freeb(b)` on the leftover, never a putback).
    fn pop(&mut self, buf: &mut [u8]) -> Option<u64> {
        let (slot, len) = self.queue.pop_front()?;
        let base = slot as usize * ETHERMAXTU;
        let n = (len as usize).min(buf.len());
        buf[..n].copy_from_slice(&self.bytes[base..base + n]);
        self.free.push(slot);
        Some(n as u64)
    }
}

/// The IRQ-facing per-conversation slot: the demux snapshot (mirrored
/// from the ConvBook at ctl-connect/close — see its header) plus the
/// frame store.  `open` mirrors "the conversation has a live data/ctl
/// open" — the read path's EOF condition (the donor's qclose: a reader
/// blocked when the last open closed wakes and reads EOF).
struct ConvSlot {
    typ: i32,
    prom: bool,
    open: bool,
    store: Option<FrameStore>,
}

impl ConvSlot {
    const fn new() -> ConvSlot {
        ConvSlot { typ: 0, prom: false, open: false, store: None }
    }
}

/// The handoff state the IRQ retire touches (plus the counters — the
/// stats file reads them here, live).  All plain data under the
/// BusyCell: no atomics needed because the cell's enter() IS the
/// synchronization, and the kernel invariant makes it uncontended.
pub(crate) struct IngressState {
    convs: [ConvSlot; NFILE],
    /// Pids blocked on a data read (any conversation) — a GLOBAL wake
    /// set: any ingress frame wakes them all; a spuriously-woken reader
    /// re-checks its queue and re-blocks (devpipe's re-check loop
    /// shape).  At most one waiter per proc; the gates never block a
    /// reader (OQ-1).  FIXED capacity (one slot per possible proc): the
    /// IRQ-side drain must never allocate or free (the allocator's lock
    /// is a syscall-context lock).
    rx_waiters: WaitSet,
    /// Pids blocked on a TX-full write (woken by the retire).
    tx_waiters: WaitSet,
    /// The live counters (netif.c's stats-file fields):
    /// inpackets++ on EVERY inbound frame before demux; outpackets++
    /// per frame posted; soverflows++ per dropped delivery.
    inpackets: u64,
    outpackets: u64,
    soverflows: u64,
    /// The donor's `misses` — never incremented this era (the donor's
    /// etheriq doesn't either; the field exists for the file shape).
    #[allow(dead_code)]
    misses: u64,
    /// The demux's address snapshot (mirrored from the Netif at publish).
    addr: [u8; 6],
    bcast: [u8; 6],
    /// Count of promiscuous conversations (the donor's `nif->prom` —
    /// the multicast filter's global arm).
    prom: u32,
}

/// A fixed-capacity pid set (one waiter per proc — the proc table is
/// MAX_PROCS=16, so 16 slots can never fill).  pid 0 marks an empty
/// slot (pids start at 1).  No allocation ANYWHERE — the IRQ-side drain
/// moves the pids into a caller's stack buffer.
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
    /// Drain the set into a stack buffer (the wake hook's input) —
    /// returns the count; every drained slot clears.
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

pub(crate) static INGRESS: BusyCell<IngressState> = BusyCell::new(IngressState {
    convs: [const { ConvSlot::new() }; NFILE],
    rx_waiters: WaitSet::new(),
    tx_waiters: WaitSet::new(),
    inpackets: 0,
    outpackets: 0,
    soverflows: 0,
    misses: 0,
    addr: [0; 6],
    bcast: [0xff; 6],
    prom: 0,
});

// ---------------------------------------------------------------------------
// The block/wake hooks (the devpipe idiom — dependency-injected because
// port cannot see the arch's proc table).  Registered once from
// main9/exec_bootstrap alongside devpipe/devcons; a lazy hooks() accessor
// keeps a never-blocking op registration-free.
// ---------------------------------------------------------------------------

/// The arch-side block/wake hooks (devpipe's PipeHooks shape):
/// `current` names the calling proc (pid), `block` parks the caller
/// mid-syscall, `wake` readies the listed pids.
#[derive(Clone, Copy)]
pub struct EtherHooks {
    pub current: fn() -> u64,
    pub block: fn(),
    pub wake: fn(&[u64]),
}

static HOOKS: Lock<Option<EtherHooks>> = Lock::new("devether", None);

/// Register the arch-side hooks.  Called once during boot before any
/// user process exists; a second registration is a boot bug.
pub fn set_hooks(hooks: EtherHooks) {
    let node = LockNode::new();
    let mut guard = HOOKS.lock(&node);
    assert!(guard.is_none(), "devether: hooks registered twice");
    *guard = Some(hooks);
}

pub(crate) fn hooks() -> Result<EtherHooks> {
    let node = LockNode::new();
    HOOKS.lock(&node).ok_or("devether: block/wake hooks not registered")
}

// ---------------------------------------------------------------------------
// Conversation lifecycle (called by devether's open/close under no lock
// of its own — these take NETIF/INGRESS internally, sequentially).
// ---------------------------------------------------------------------------

/// The clone-open allocation (netif.c's `openfile(nif, -1)`): the first
/// slot with `inuse == 0` (never-allocated or last-closed); a fresh
/// slot's store is allocated here (syscall context — the IRQ path never
/// allocates).  Returns the conversation id; the caller rewrites the
/// chan's qid to NETQID(id, Nctlqid).
pub(crate) fn conv_alloc() -> Result<usize> {
    let node = LockNode::new();
    let mut ng = NETIF.lock(&node);
    let Some(nif) = ng.as_mut() else { return Err("devether: no ether0") };
    // The INGRESS mirror is taken UNDER the Netif lock (the documented
    // NETIF → INGRESS order) — the canonical record and the mirror move
    // in one syscall critical section.
    let mut ig = INGRESS.enter().ok_or("devether: ingress contended (invariant violation)")?;
    for id in 0..NFILE {
        if nif.convs[id].inuse == 0 {
            if ig.convs[id].store.is_none() {
                ig.convs[id].store = Some(FrameStore::new());
            }
            nif.convs[id].inuse = 1;
            nif.convs[id].owner_set = true;
            nif.convs[id].typ = 0;
            nif.convs[id].prom = false;
            ig.convs[id].typ = 0;
            ig.convs[id].prom = false;
            ig.convs[id].open = true;
            return Ok(id);
        }
    }
    Err("devether: no free conversations")
}

/// Re-open conversation `id` (netif.c's `openfile(nif, id)` — the
/// qreopen + inuse++ arm).  Errors Enodev-shaped when the slot was never
/// clone-allocated.
pub(crate) fn conv_open(id: usize) -> Result<()> {
    let node = LockNode::new();
    let mut ng = NETIF.lock(&node);
    let Some(nif) = ng.as_mut() else { return Err("devether: no ether0") };
    if id >= NFILE {
        return Err("devether: no such conversation");
    }
    let mut ig = INGRESS.enter().ok_or("devether: ingress contended (invariant violation)")?;
    if ig.convs[id].store.is_none() {
        return Err("devether: no such conversation");
    }
    nif.convs[id].inuse += 1;
    if !nif.convs[id].owner_set {
        // First-opener-owns (the netown collapse).
        nif.convs[id].owner_set = true;
    }
    ig.convs[id].open = true;
    Ok(())
}

/// dat.rs Chan::dup's bump for an open data/ctl chan (the devpipe
/// ref_open_end rule): every value copy of an OPEN conversation file is
/// a new open reference, or rfork/dup'd fds would double-decrement the
/// conversation at close.
pub(crate) fn conv_ref(qid_path: u64) {
    let id = netid(qid_path) as usize;
    if id >= NFILE {
        return;
    }
    let node = LockNode::new();
    let mut ng = NETIF.lock(&node);
    if let Some(nif) = ng.as_mut() {
        nif.convs[id].inuse += 1;
    }
}

/// The last-close reset (netif.c's netifclose): inuse--; at zero the
/// conversation resets (owner cleared, type 0, prom cleared, the queue
/// drained — the donor's qclose) and any blocked readers wake to EOF.
pub(crate) fn conv_close(id: usize) {
    if id >= NFILE {
        return;
    }
    let mut pids = [0u64; 16];
    let mut nwake = 0;
    {
        let node = LockNode::new();
        let mut ng = NETIF.lock(&node);
        let mut ig = INGRESS.enter().expect("netif: close with INGRESS contended (boot bug)");
        let Some(nif) = ng.as_mut() else { return };
        let book = &mut nif.convs[id];
        if book.inuse > 0 {
            book.inuse -= 1;
        }
        if book.inuse == 0 {
            if book.prom {
                book.prom = false;
                nif.prom = nif.prom.saturating_sub(1);
                ig.prom = ig.prom.saturating_sub(1);
            }
            book.owner_set = false;
            book.typ = 0;
            let slot = &mut ig.convs[id];
            slot.typ = 0;
            slot.prom = false;
            slot.open = false;
            if let Some(store) = slot.store.as_mut() {
                // qclose: the queue drains; the slots return free.
                store.queue.clear();
                store.free = (0..FRAME_SLOTS as u16).rev().collect();
            }
            nwake = ig.rx_waiters.take(&mut pids);
        }
    }
    // A blocked reader of the closed conversation wakes and reads EOF
    // (the donor's qclose → qwait-returns-0 arm).  The wake hook call is
    // OUTSIDE the locks (never hold either across PROC_TABLE).
    if nwake > 0 && let Ok(h) = hooks() {
        (h.wake)(&pids[..nwake]);
    }
}

/// `connect N` (netifwrite): set the conversation's ether-type filter.
/// N is range-checked to the ether-type width plus -1 (connect -1 =
/// all); the donor's typeinuse check rejects a positive type already in
/// use on another conversation (Einuse shape).
pub(crate) fn conv_connect(id: usize, typ: i32) -> Result<()> {
    let node = LockNode::new();
    let mut ng = NETIF.lock(&node);
    let Some(nif) = ng.as_mut() else { return Err("devether: no ether0") };
    if id >= NFILE {
        return Err("devether: no such conversation");
    }
    if typ > 0 {
        for (other, book) in nif.convs.iter().enumerate() {
            if other != id && book.inuse > 0 && book.typ == typ {
                return Err("devether: ether-type already in use");
            }
        }
    }
    nif.convs[id].typ = typ;
    let mut ig = INGRESS.enter().ok_or("devether: ingress contended (invariant violation)")?;
    ig.convs[id].typ = typ;
    Ok(())
}

/// `promiscuous` (netifwrite's arm): accepted as a no-op toward the
/// hardware (qemu's RX filter without CTRL_VQ delivers our MAC +
/// broadcast regardless — the plan's locked disposition); the per-
/// conversation flag + count still move so the demux and the stats
/// file's `prom:` field stay netif-faithful.
pub(crate) fn conv_promiscuous(id: usize) -> Result<()> {
    let node = LockNode::new();
    let mut ng = NETIF.lock(&node);
    let Some(nif) = ng.as_mut() else { return Err("devether: no ether0") };
    if id >= NFILE {
        return Err("devether: no such conversation");
    }
    if !nif.convs[id].prom {
        nif.convs[id].prom = true;
        nif.prom += 1;
        let mut ig = INGRESS.enter().ok_or("devether: ingress contended (invariant violation)")?;
        ig.convs[id].prom = true;
        ig.prom += 1;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The IRQ retire's entries (IRQ context — the handler calls these; the
// BusyCell discipline above is what makes them safe there).
// ---------------------------------------------------------------------------

/// RX ingress (devether.c's etheriq): count EVERY inbound frame, demux
/// to every conversation whose filter matches (typ == ethertype or -1)
/// and whose delivery gate passes (to-me || multicast || promiscuous —
/// the donor's `tome || multi || f->prom` with this era's always-empty
/// activemulti), queue-full → soverflows++.  Then wake blocked readers.
///
/// Called from the virtio-net IRQ handler with the 12-byte header
/// already stripped (the raw 14-byte-header Ethernet frame).
pub fn rx_ingress(frame: &[u8]) {
    let Some(mut ig) = INGRESS.enter() else {
        // Provably unreachable (the BusyCell discipline + the kernel's
        // IRQ-masked-syscall invariant); a violation drops the frame and
        // would surface as a stats/pcap mismatch, never a silent wedge.
        return;
    };
    ig.inpackets += 1;
    if frame.len() < 14 {
        return; // runt — counted as inbound, undeliverable
    }
    let tome = frame[0..6] == ig.addr;
    let multi = frame[0] & 1 != 0;
    // The donor's multicast gate: a non-broadcast multicast with no
    // promiscuous open and no activemulti (always empty this era) is
    // dropped for everyone.
    if multi && frame[0..6] != ig.bcast && ig.prom == 0 {
        return;
    }
    let etype = u16::from_be_bytes([frame[12], frame[13]]) as i32;
    let mut delivered = false;
    let mut drops: u64 = 0;
    for slot in ig.convs.iter_mut() {
        if !slot.open {
            continue;
        }
        if slot.typ != etype && slot.typ >= 0 {
            continue;
        }
        if !(tome || multi || slot.prom) {
            continue;
        }
        match slot.store.as_mut() {
            Some(store) => {
                if store.push(frame) {
                    delivered = true;
                } else {
                    drops += 1;
                }
            }
            None => drops += 1, // open-without-store: impossible; counted loudly
        }
    }
    ig.soverflows += drops;
    if !delivered {
        return;
    }
    let mut pids = [0u64; 16];
    let nwake = ig.rx_waiters.take(&mut pids);
    drop(ig);
    // The wake AFTER the guard drops (INGRESS is never held across the
    // hook's PROC_TABLE take).  A missing hook registration with live
    // waiters is a boot-order bug (hooks register before the first exec);
    // the reader wedges loudly at a gate timeout if so.
    if nwake > 0 && let Ok(h) = hooks() {
        (h.wake)(&pids[..nwake]);
    }
}

/// The TX retire's wake: a write blocked on a full ring (D-08's
/// wedge-hunt mechanism — a stalled retire wedges the WRITER, so the
/// gate's timeout catches it) readies here.  Called by the IRQ handler
/// when its TX drain retired ≥1 entry.
pub fn tx_retired() {
    let mut pids = [0u64; 16];
    let nwake = {
        let mut ig = match INGRESS.enter() {
            Some(g) => g,
            None => return, // the rx_ingress None arm's discipline
        };
        ig.tx_waiters.take(&mut pids)
    };
    if nwake > 0 && let Ok(h) = hooks() {
        (h.wake)(&pids[..nwake]);
    }
}

/// outpackets++ — the write path's per-frame-posted count (the donor's
/// etheroq arm: counted at post time, not at retire).
pub(crate) fn tx_posted() {
    let mut ig = INGRESS.enter().expect("netif: tx_posted with INGRESS contended (invariant violation)");
    ig.outpackets += 1;
}

/// Register the calling pid as blocked on a TX-full write (the write
/// path's full-ring arm).  Registration is IRQ-atomic with respect to
/// the retire: syscalls run IRQ-masked, so no retire can interleave
/// between the caller's full-ring observation and this registration.
pub(crate) fn register_tx_waiter(pid: u64) {
    let mut ig = INGRESS.enter().expect("netif: register with INGRESS contended (invariant violation)");
    ig.tx_waiters.register(pid);
}

/// The data-read population check: one whole frame popped into `buf`
/// (Qmsg — excess discarded), Empty (the caller blocks), or Closed (the
/// conversation's last open closed — the donor's qclosed-queue EOF).
pub(crate) enum Pop {
    Frame(u64),
    Empty,
    Closed,
}

pub(crate) fn pop_frame(id: usize, buf: &mut [u8]) -> Pop {
    let mut ig = INGRESS.enter().expect("netif: read with INGRESS contended (invariant violation)");
    if id >= NFILE {
        return Pop::Closed;
    }
    let slot = &mut ig.convs[id];
    if let Some(store) = slot.store.as_mut() {
        if let Some(n) = store.pop(buf) {
            return Pop::Frame(n);
        }
    }
    if slot.open { Pop::Empty } else { Pop::Closed }
}

/// Register the calling pid as blocked on a data read (the read path's
/// register → drop → block shape — see the module header).
pub(crate) fn register_rx_waiter(pid: u64) {
    let mut ig = INGRESS.enter().expect("netif: register with INGRESS contended (invariant violation)");
    ig.rx_waiters.register(pid);
}

// ---------------------------------------------------------------------------
// The stats/addr content (netif.c:221-248's byte formats — the stats
// file's exact field order, the addr file's bare lowercase hex with NO
// trailing newline).
// ---------------------------------------------------------------------------

/// Decimal ASCII, no padding (the devproc `dec` idiom).
fn dec(mut v: u64) -> Vec<u8> {
    let mut out = Vec::new();
    if v == 0 {
        out.push(b'0');
    }
    while v > 0 {
        out.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    out
}

/// Lowercase hex bytes, no separators (`%2.2ux` × alen).
fn hexaddr(addr: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::with_capacity(addr.len() * 2);
    for &b in addr {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 0xf) as usize]);
    }
    out
}

/// The stats file's content (netif.c:221-238 verbatim field order).
/// The counters are read live from INGRESS; the interface fields from
/// NETIF (the two locks are taken SEQUENTIALLY, never nested).
pub(crate) fn stats_content() -> Result<Vec<u8>> {
    let (link, mbps, prom, addr, alen) = {
        let node = LockNode::new();
        let ng = NETIF.lock(&node);
        let Some(nif) = ng.as_ref() else { return Err("devether: no ether0") };
        (nif.link, nif.mbps, nif.prom, nif.addr, nif.alen)
    };
    let (inp, outp, sov) = {
        let ig = INGRESS.enter().ok_or("devether: ingress contended (invariant violation)")?;
        (ig.inpackets, ig.outpackets, ig.soverflows)
    };
    let mut out = Vec::new();
    let mut line = |key: &[u8], val: Vec<u8>| {
        out.extend_from_slice(key);
        out.extend_from_slice(&val);
        out.push(b'\n');
    };
    line(b"in: ", dec(inp));
    line(b"link: ", dec(u64::from(link)));
    line(b"out: ", dec(outp));
    line(b"crc errs: ", dec(0));
    line(b"overflows: ", dec(0));
    line(b"soft overflows: ", dec(sov));
    line(b"framing errs: ", dec(0));
    line(b"buffer errs: ", dec(0));
    line(b"output errs: ", dec(0));
    line(b"prom: ", dec(u64::from(prom)));
    line(b"mbps: ", dec(u64::from(mbps)));
    out.extend_from_slice(b"addr: ");
    out.extend_from_slice(&hexaddr(&addr[..alen]));
    out.push(b'\n');
    Ok(out)
}

/// The addr file's content: the MAC as bare lowercase hex, NO trailing
/// newline (netif.c:239-248 — the digits only).
pub(crate) fn addr_content() -> Result<Vec<u8>> {
    let node = LockNode::new();
    let ng = NETIF.lock(&node);
    let Some(nif) = ng.as_ref() else { return Err("devether: no ether0") };
    Ok(hexaddr(&nif.addr[..nif.alen]))
}

/// The type file's value (the conversation's filter) — the canonical
/// copy under the Netif lock.
pub(crate) fn type_of(id: usize) -> Result<i32> {
    let node = LockNode::new();
    let ng = NETIF.lock(&node);
    let Some(nif) = ng.as_ref() else { return Err("devether: no ether0") };
    if id >= NFILE {
        return Err("devether: no such conversation");
    }
    Ok(nif.convs[id].typ)
}

/// The write path's MTU bounds, read from the model (the donor's
/// `ether->minmtu`/`ether->mtu`): (minmtu, mtu) = (60, 1514).
pub(crate) fn mtu_bounds() -> Result<(u32, u32)> {
    let node = LockNode::new();
    let ng = NETIF.lock(&node);
    let Some(nif) = ng.as_ref() else { return Err("devether: no ether0") };
    Ok((nif.minmtu, nif.mtu))
}

/// The interface's level-2 directory name ("ether0") — the walk and
/// listing read the model's copy, never a device-side literal.
pub(crate) fn ifname() -> Result<&'static [u8]> {
    let node = LockNode::new();
    let ng = NETIF.lock(&node);
    let Some(nif) = ng.as_ref() else { return Err("devether: no ether0") };
    Ok(nif.name)
}

/// The directory-listing support: which conversation slots are
/// allocated (the donor lists allocated slots regardless of inuse —
/// netifgen's `nif->f[i] != 0`) plus their owner-set flags (the level-3
/// listing's perms).
pub(crate) fn conv_allocated() -> [bool; NFILE] {
    let ig = match INGRESS.enter() {
        Some(g) => g,
        None => return [false; NFILE],
    };
    let mut out = [false; NFILE];
    for (i, s) in ig.convs.iter().enumerate() {
        out[i] = s.store.is_some();
    }
    out
}

/// The level-3 listing's owner-collapse state for one conversation
/// (0660 once first-opened, 0666 before — the donor's `*f->owner`
/// check).
pub(crate) fn conv_owned(id: usize) -> bool {
    let node = LockNode::new();
    let ng = NETIF.lock(&node);
    match ng.as_ref() {
        Some(nif) if id < NFILE => nif.convs[id].owner_set,
        _ => false,
    }
}
