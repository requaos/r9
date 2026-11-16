//! devmnt — the kernel 9P client `#M`.  Semantics ported from 9k's
//! devmnt.c (mntversion/mntattach/mntwalk/mntopen/mntread/mntstat/
//! mntclunk over the pipe fd, sized to the paqfs message set) with the
//! wire codec from fcall.h + convS2M.c / convM2S.c.
//!
//! A mounted tree is a SESSION: sys_fversion negotiates the msize over
//! the fd's chan (Tversion/Rversion — "9P2000", offer 8192, accept the
//! server's lower answer) and parks the resulting `Arc<Mnt>` ON the chan
//! (dat.rs's `mnt` slot — the donor's `c->mux`); sys_mount attaches the
//! tree (Tattach, afid = NOFID — the no-auth path, nsinit's
//! fall-through) and inserts the mount-table entry at the resolved
//! target.  Chans in the mounted tree carry the session Arc plus their
//! server-side fid; namec's DEV_MNT legs dispatch walk/open/read/stat to
//! Twalk/Topen/Tread/Tstat RPCs, and namec::close_chan's DEV_MNT leg
//! sends Tclunk when an OPENED mnt chan leaves an fd slot.
//!
//! EVERY R-message off the pipe is untrusted input from a userspace
//! server (the mntchk discipline, ported): the receive path bounds-checks
//! the message size against the negotiated msize BEFORE reading the body,
//! tag-matches the reply against the outstanding T-message, requires the
//! reply type to be request+1 (Rerror maps to a loud errstr with the
//! server's ename on the serial log), and parses every field through a
//! checked cursor that must consume the message EXACTLY (the donor
//! convM2S's `l != n` check).  Protocol failures are "devmnt: …" errstrs,
//! never panics, never partial reads, never trusted bytes.  Host tests
//! prove rejection of oversized and mis-tagged replies
//! (port/tests/devmnt_test.rs).
//!
//! Concurrency (19-08's rework — the donor's mountio/mountmux model,
//! previously the era's documented single-outstanding limit): MANY
//! rpcs may be outstanding per session, multiplexed by TAG.  Everyone
//! SENDS (whole messages; the transport serializes at message
//! granularity); exactly ONE context READS the wire at a time — the
//! io_r BusyCell is the donor's `m->rip` reader token, and its guard
//! spans WHOLE-MESSAGE framing only, never a release between a
//! message's header and its body (the mid-frame window is the boot
//! race the first rework attempt died of).  The reader demuxes by
//! tag: its own reply ends its loop; a queued rpc's reply is ROUTED
//! to its slot (owners poll — the injected park hook bounds the
//! retry, no wakeup machinery to lose); a reply matching NOTHING
//! outstanding is a loud protocol error.  A reader parked on an
//! indefinite rpc (cpunote, console stdin over /mnt/term) keeps
//! serving everyone else's replies — the donor's designated-reader
//! shape, which is what un-deadlocks session exit.  BusyCells (not
//! MCS locks) guard everything whose ownership spans a blocking read:
//! an MCS lock held across a park would spin-deadlock an IRQ-masked
//! contender on this single-CPU era.
//!
//! Transport bound (the pipe's atomic-write ceiling): devpipe writes
//! whole messages or fail ("pipe full"), and the queue cap is 4096, so
//! per-RPC read counts clamp to `TRANSPORT_MSG_MAX - IOHDRSZ` (4072) even
//! though the negotiated msize is 8192 — the donor's iounit clamp, with
//! the era's transport as the binding constraint.  The virtio-9p era
//! lifts this.
//!
//! The dat.rs `Dev` trait is NOT implemented here (its skeleton is
//! non-constructible — see devroot.rs's header); the session methods and
//! the Chan-wrapping device ops are free functions/methods, dispatched by
//! chan.dev via namec.rs's legs (DEV_MNT).
//!
//! Fid lifecycle: fids are allocated at
//! walk/attach from a per-session counter (never reused — paqfs's Fid is
//! ~50 bytes and the boot chain walks dozens, not millions) and carried
//! on the chan as `Option<Arc<u32>>`: dup CLONES the Arc, open SHARES the
//! walked fid's Arc (the donor's open-doesn't-realloc shape), and the
//! close dispatch (`clunk_if_last`) Tclunks only when the LAST ref
//! leaves — exactly-once by construction, against any server.  namec's
//! walk sweeps the DEV_MNT chans IT mints (walk-step results + redirect
//! clones once walked past) on every exit, Ok and Err alike, so walk
//! intermediates never leak server-side.  One deliberate
//! bound remains: mount-TABLE entries stay table-lifetime-pinned — an
//! unmount/MREPL-replaced entry's fid is never Tclunk'd (deferred;
//! bounded by the mount count, ~one fid per mount per boot).

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_MNT, DEV_PIPE, DEV_SSL, OMASK, ORDWR, OREAD, QTDIR};
use crate::devpipe;
use crate::mcslock::{BusyCell, Lock, LockNode};
use crate::println;
use crate::Result;

// ---------------------------------------------------------------------------
// The wire codec (fcall.h + convS2M.c/convM2S.c, ported for the client set)
// ---------------------------------------------------------------------------

/// 9P2000 message types (fcall.h — frozen values).
const TVERSION: u8 = 100;
const RVERSION: u8 = 101;
const TATTACH: u8 = 104;
const RATTACH: u8 = 105;
const RERROR: u8 = 107;
const TWALK: u8 = 110;
const RWALK: u8 = 111;
const TOPEN: u8 = 112;
const ROPEN: u8 = 113;
const TREAD: u8 = 116;
const RREAD: u8 = 117;
const TWRITE: u8 = 118;
const RWRITE: u8 = 119;
const TCREATE: u8 = 114;
const RCREATE: u8 = 115;
const TCLUNK: u8 = 120;
const RCLUNK: u8 = 121;
const TSTAT: u8 = 124;
const RSTAT: u8 = 125;

/// The version tag (fcall.h NOTAG) and the no-auth fid (NOFID).
pub const NOTAG: u16 = 0xFFFF;
pub const NOFID: u32 = 0xFFFF_FFFF;
/// fcall.h IOHDRSZ — the Twrite/Rread header room margin.
pub const IOHDRSZ: u32 = 24;
/// fcall.h MAXWELEM.
const MAXWELEM: usize = 16;

/// The version string (fcall.h VERSION9P).
pub const VERSION9P: &[u8] = b"9P2000";

/// The msize devmnt offers when the caller passes 0 (the plan's pick —
/// paqfs's default mesgsize is 8216, so 8192 negotiates through cleanly).
pub const DEFAULT_MSIZE: u32 = 8192;

/// The transport's whole-message ceiling: devpipe's queue cap (a pipe
/// write is whole-message-or-fail, so no 9P message may exceed it).
pub const TRANSPORT_MSG_MAX: u32 = devpipe::PIPE_Q_CAP as u32;

/// The donor's msize sanity bounds (devmnt.c mntversion's
/// "nonsense value of msize" check) — applied to the OFFER too (a user
/// syscall arg sizes the kernel's receive scratch; unbounded allocation
/// from an unchecked arg is a memory-exhaustion hole).
const MSIZE_MIN: u32 = 256;
const MSIZE_MAX: u32 = 1024 * 1024;

// Little-endian wire writers (fcall.h's PBIT*).
fn pw16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pw32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pw64(v: &mut Vec<u8>, x: u64) {
    v.extend_from_slice(&x.to_le_bytes());
}
/// convS2M's pstring: u16 length prefix + the bytes.
fn pwstr(v: &mut Vec<u8>, s: &[u8]) {
    pw16(v, s.len() as u16);
    v.extend_from_slice(s);
}
/// Wrap a body in the 9P header: size[4] type[1] tag[2] + body.
fn tmsg(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(7 + body.len());
    pw32(&mut v, (7 + body.len()) as u32);
    v.push(typ);
    pw16(&mut v, tag);
    v.extend_from_slice(body);
    v
}

/// A checked little-endian cursor for parsing R-messages (the mini
/// convM2S): every field read is bounds-checked, and `done` requires the
/// message to be consumed EXACTLY (the donor's `l != n` check) — trailing
/// garbage is a protocol error.
struct RepCur<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> RepCur<'a> {
    fn new(b: &'a [u8]) -> RepCur<'a> {
        RepCur { b, p: 0 }
    }
    fn u8(&mut self) -> Result<u8> {
        let Some(&v) = self.b.get(self.p) else { return Err("devmnt: malformed reply (truncated)"); };
        self.p += 1;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16> {
        let Some(s) = self.b.get(self.p..self.p + 2) else {
            return Err("devmnt: malformed reply (truncated)");
        };
        self.p += 2;
        Ok(u16::from_le_bytes(s.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        let Some(s) = self.b.get(self.p..self.p + 4) else {
            return Err("devmnt: malformed reply (truncated)");
        };
        self.p += 4;
        Ok(u32::from_le_bytes(s.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        let Some(s) = self.b.get(self.p..self.p + 8) else {
            return Err("devmnt: malformed reply (truncated)");
        };
        self.p += 8;
        Ok(u64::from_le_bytes(s.try_into().unwrap()))
    }
    /// convM2S's gqid: type[1] vers[4] path[8].
    fn qid(&mut self) -> Result<Qid> {
        let typ = self.u8()?;
        let vers = self.u32()?;
        let path = self.u64()?;
        Ok(Qid { path, vers, typ })
    }
    /// convM2S's gstring: u16 length prefix + the bytes (bounds-checked —
    /// a count that overruns the message is malformed, never a panic).
    fn string(&mut self) -> Result<Vec<u8>> {
        let n = self.u16()? as usize;
        Ok(self.bytes(n)?.to_vec())
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let Some(s) = self.b.get(self.p..self.p + n) else {
            return Err("devmnt: malformed reply (count overruns the message)");
        };
        self.p += n;
        Ok(s)
    }
    /// The whole message must be consumed (convM2S's exact-size check).
    fn done(self) -> Result<()> {
        if self.p != self.b.len() {
            return Err("devmnt: malformed reply (trailing garbage)");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The transport (the pipe fd's chan in the kernel; a scripted fake server
// in host tests)
// ---------------------------------------------------------------------------

/// The 9P byte transport: one whole message per write; reads return up to
/// the buffer's length and BLOCK until at least one byte is available
/// (the kernel transport's devpipe read schedules the server proc).
/// A Send supertrait attempt breaks the unmodified host tests
/// (FakeServer's Rc fid table), so
/// the session's shareability proof is the BusyCell discipline and the
/// `unsafe impl Send for MntCore` lives beside the primitive (mcslock.rs).
pub trait MntIo {
    /// Write one whole message, atomically (devpipe's contract: the full
    /// buffer lands or the write fails — never a partial message).
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64>;
    /// Read available bytes (blocks on empty); 0 = EOF (the peer hung up).
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64>;
    /// Release the transport's kernel reference (the session-teardown
    /// half, the donor mntdec's cclose of m->c): the SESSION's Drop
    /// calls this so a dying session's connection chans dispatch their
    /// close (a bare Box drop would leak the open counts — the ssl conv
    /// would never hang its TCP wire up, and the peer's exportfs would
    /// never see the session end).  Default no-op (the host fakes own
    /// nothing).
    fn close_chan(&mut self) {}
}

/// The kernel transport: a 9P-capable chan — a pipe end (the /srv and
/// boot mounts) or a DEVSSL data chan (19-08: cpu -R's remoteside
/// fversions/mounts the client's exportfs over the encrypted session).
/// Reads/writes ride the device's streaming ops directly (offsets don't
/// apply); anything else is a loud contract violation
/// (sys_fversion/sys_mount validate the fd before constructing this).
pub struct ChanIo {
    pub chan: Chan,
}

impl MntIo for ChanIo {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        match self.chan.dev {
            DEV_PIPE => devpipe::write(&mut self.chan, buf, u64::MAX),
            DEV_SSL => crate::devssl::write(&mut self.chan, buf),
            _ => Err("devmnt: fd is not a 9P-capable connection"),
        }
    }
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        match self.chan.dev {
            DEV_PIPE => devpipe::read(&mut self.chan, buf, u64::MAX),
            DEV_SSL => crate::devssl::read(&mut self.chan, buf, u64::MAX),
            _ => Err("devmnt: fd is not a 9P-capable connection"),
        }
    }
    fn close_chan(&mut self) {
        crate::namec::close_chan(&self.chan);
    }
}

// ---------------------------------------------------------------------------
// The session
// ---------------------------------------------------------------------------

/// One io half's wrapper (the Send proof lives beside the BusyCell
/// primitive in mcslock.rs — devmnt itself is forbid(unsafe_code)).
pub(crate) struct MntIoBox(pub(crate) Box<dyn MntIo>);

/// The outstanding-rpc registry entry (the donor's queued Mntrpc's
/// completed half): the tag awaited and the slot the routed reply
/// lands in.  `Some` in `reply` IS the done flag (the donor's
/// q->done): the OWNER polls `take`, the READER `route`s — the leaf
/// lock orders the handoff, and a routed reply is never re-read off
/// the wire.
struct MntSlot {
    tag: u16,
    reply: Lock<Option<Vec<u8>>>,
}

impl MntSlot {
    fn new(tag: u16) -> MntSlot {
        MntSlot { tag, reply: Lock::new("mntslot", None) }
    }
    /// Route a framed reply to its owner (reader side).  The whole
    /// reply message lands here; the owner's check_reply re-validates
    /// it end-to-end (untrusted input, same as ever).
    fn route(&self, msg: Vec<u8>) {
        let node = LockNode::new();
        *self.reply.lock(&node) = Some(msg);
    }
    /// The routed reply, if any (owner side).  Once only.
    fn take(&self) -> Option<Vec<u8>> {
        let node = LockNode::new();
        self.reply.lock(&node).take()
    }
}

/// The fid/tag allocator state (donor mntchan/alloctag): fids from 1
/// up (never NOFID, never reused this era); tags from 1 up (never 0,
/// never NOTAG — devmnt.c's mntreset exclusions).  Leaf-locked — far
/// briefer than the old whole-rpc busy flag.
struct MntAlloc {
    next_fid: u32,
    next_tag: u16,
}

/// A 9P client session (the donor's `Mnt`): one connection's
/// negotiated state plus the fid/tag allocators and the tag-mux
/// registry.  Shared by every chan in the mounted tree (and by the
/// pipe chan that fversion created it over) as `Arc<Mnt>` — see the
/// module header's concurrency note for the reader-token model.
pub struct Mnt {
    /// The READ half + the single-wire-reader token in one BusyCell
    /// (the donor's m->rip): enter() = become the reader.  The guard
    /// spans blocking reads by design (the parked reader serves
    /// everyone); contenders park-retry, never spin.
    io_r: BusyCell<MntIoBox>,
    /// The WRITE half: enter() = the write slot (concurrent rpcs send
    /// interleaved whole messages; the cell serializes senders).
    io_w: BusyCell<MntIoBox>,
    /// The fid/tag allocators.
    alloc: Lock<MntAlloc>,
    /// The outstanding-rpc registry (the donor's mnt->queue).
    queue: Lock<VecDeque<Arc<MntSlot>>>,
    /// The negotiated msize (the reply's answer, ≤ the offer) — set once
    /// at negotiation, frozen after (plain shared reads, no guard).
    msize: u32,
    /// The negotiated version string (answered on re-version, the donor's
    /// c->mux replay in mntversion) — set-once, frozen.
    version: Vec<u8>,
    /// The session's device-instance id (the donor devmnt.c
    /// `c->devno = mntalloc.id++`): every chan this session mints carries
    /// it, making the mount-consult identity (dev, devno, qid.path —
    /// eqchanddq) unique per mounted tree even when two servers hand out
    /// colliding qids (19-10 Leg-1's identity keying).
    id: u32,
}

/// The mount-session id sequence — 9k's mntalloc.id.
static NEXT_MNT_ID: Lock<u32> = Lock::new("mntid", 1);

impl core::fmt::Debug for Mnt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Mnt(9p session)")
    }
}

impl Drop for Mnt {
    /// The donor's session teardown (mntdec → mntdestroy's cclose of
    /// the connection chan): when the LAST chan of the mounted tree
    /// goes — the service tree exits, the pgrp dies, the mount-table
    /// entry's chan value drops the session Arc — the session's io
    /// halves dispatch their close.  Without this the ssl conv's refc
    /// never reaches zero, its TCP wire never hangs up, and the peer's
    /// exportfs session never ends (the -G session-exit stage's
    /// measured stall: the whole service tree had exited and the
    /// connection still lived).  The BusyCells cannot be held at drop
    /// time: any in-flight rpc holds the session Arc through its
    /// caller's chan, so refcount 0 proves no reader or writer exists.
    fn drop(&mut self) {
        if let Some(mut g) = self.io_r.enter() {
            g.0.close_chan();
        }
        if let Some(mut g) = self.io_w.enter() {
            g.0.close_chan();
        }
    }
}

/// The arch-side contention-wait hook (the devpipe set_hooks idiom —
/// port can't see PROC_TABLE): on RPC ownership contention the caller
/// parks ~1 tick and retries (06-07's measured first concurrent caller —
/// the tier-1 sweep's pipelines — needed the wait, not the loud error).
/// Registered at boot before the first exec; absent on no-userspace
/// profiles, where the loud error stays the (unreachable) fallback.
static CONTENTION_WAIT: Lock<Option<fn()>> = Lock::new("devmnt-ctw", None);

/// Register the arch-side contention-wait hook.  Once only.
pub fn set_contention_hook(f: fn()) {
    let node = LockNode::new();
    let mut guard = CONTENTION_WAIT.lock(&node);
    assert!(guard.is_none(), "devmnt: contention hook registered twice");
    *guard = Some(f);
}

/// Park the current caller briefly so a concurrent RPC contender retries
/// instead of failing (the hook fn is Copy; the leaf lock is never held
/// across the park).  `loud` is the errstr for the no-hook venue (bare
/// profiles never contend — the hook is dev_exec-only wiring).
pub(crate) fn contention_park(loud: &'static str) -> Result<()> {
    let f = {
        let node = LockNode::new();
        let guard = CONTENTION_WAIT.lock(&node);
        *guard
    };
    match f {
        Some(f) => {
            f();
            Ok(())
        }
        None => Err(loud),
    }
}

fn contention_wait() -> Result<()> {
    contention_park("devmnt: concurrent rpc this era")
}

/// Test seam: replace the contention hook.  The production setter is
/// once-only by assert (a second registration is a boot bug); the mux
/// host pin needs a yield-based park inside the test binary, where no
/// arch-side hook exists.  Test-only by convention — nothing in the
/// kernel calls this.
#[doc(hidden)]
pub fn set_contention_hook_for_test(f: fn()) {
    let node = LockNode::new();
    let mut guard = CONTENTION_WAIT.lock(&node);
    *guard = Some(f);
}

impl Mnt {
    fn alloc_fid(&self) -> Result<u32> {
        let node = LockNode::new();
        let mut a = self.alloc.lock(&node);
        let fid = a.next_fid;
        if fid == NOFID {
            return Err("devmnt: fid space exhausted");
        }
        a.next_fid += 1;
        Ok(fid)
    }

    fn alloc_tag(&self) -> u16 {
        let node = LockNode::new();
        let mut a = self.alloc.lock(&node);
        let tag = a.next_tag;
        // NOTAG stays reserved; 0 is never issued (devmnt.c's mntreset).
        a.next_tag = if tag == NOTAG - 1 { 1 } else { tag + 1 };
        tag
    }

    /// Send one whole T-message (the donor mountio's write half —
    /// everyone sends).  The write cell serializes concurrent senders
    /// at message granularity (the transport is whole-message atomic
    /// anyway; this keeps ssl/TCP writes from interleaving record
    /// bodies).
    fn send(&self, msg: &[u8]) -> Result<()> {
        loop {
            let Some(mut guard) = self.io_w.enter() else {
                contention_wait()?;
                continue;
            };
            let w = guard.0.write_msg(msg)? as usize;
            if w != msg.len() {
                return Err("devmnt: short write on the 9P connection");
            }
            return Ok(());
        }
    }

    /// Frame exactly ONE whole message off the wire (the donor
    /// mntrpcread): the 4-byte size prefix, mntchk against the
    /// negotiated msize BEFORE the body is read, then exactly `size`
    /// bytes.  Called ONLY by the reader-token holder — the token
    /// spans this whole call, parks included (a release between the
    /// header and the body would let a second reader mis-frame the
    /// stream: the first rework attempt's boot race, eliminated
    /// structurally).
    fn frame_one(io: &mut MntIoBox, msize: u32) -> Result<Vec<u8>> {
        let mut hdr = [0u8; 4];
        let mut got = 0usize;
        while got < 4 {
            let n = io.0.read_some(&mut hdr[got..4])? as usize;
            if n == 0 {
                return Err("devmnt: eof waiting for a reply");
            }
            got += n;
        }
        let size = u32::from_le_bytes(hdr) as usize;
        // mntchk: the claimed size is untrusted until it passes
        // BOTH bounds — never read (or trust) a body the claim doesn't
        // cover, never read more than the negotiated msize.
        if size < 7 {
            println!("devmnt: runt reply message ({size} bytes)");
            return Err("devmnt: runt reply message");
        }
        if size > msize as usize {
            println!("devmnt: reply claims {size} bytes over msize {msize}");
            return Err("devmnt: reply exceeds negotiated msize");
        }
        let mut msg = Vec::with_capacity(size);
        msg.extend_from_slice(&hdr);
        let mut body = alloc::vec![0u8; size - 4];
        let mut gotb = 0usize;
        while gotb < body.len() {
            let n = io.0.read_some(&mut body[gotb..])? as usize;
            if n == 0 {
                return Err("devmnt: eof mid-message");
            }
            gotb += n;
        }
        msg.extend_from_slice(&body);
        Ok(msg)
    }

    /// Route a framed reply by tag (the donor mountmux): a queued
    /// rpc's reply lands in its slot and the registry entry leaves
    /// the queue at route time.  False when NOTHING outstanding owns
    /// the tag — the donor prints "unexpected reply tag" and drops;
    /// this port makes it a loud protocol error instead (the donor's
    /// drop arm exists for Tflush races, and Tflush does not exist
    /// this era — a foreign tag here is wire garbage or a server
    /// bug, and erroring drains the waiters in turn as each takes
    /// the token).
    fn route(&self, msg: Vec<u8>, tag: u16) -> bool {
        let node = LockNode::new();
        let mut q = self.queue.lock(&node);
        for (i, s) in q.iter().enumerate() {
            if s.tag == tag {
                let s = q.remove(i).unwrap();
                drop(q);
                s.route(msg);
                return true;
            }
        }
        false
    }

    /// Remove `slot` from the registry (error exits — a LATE reply to
    /// an abandoned rpc must hit the unexpected-tag arm, never a
    /// phantom owner).  Tolerant of an already-routed (removed) slot.
    fn dequeue(&self, slot: &Arc<MntSlot>) {
        let node = LockNode::new();
        self.queue.lock(&node).retain(|s| !Arc::ptr_eq(s, slot));
    }

    /// The reader loop (the donor mountio's read half): frame one
    /// whole message at a time and demux, until MY reply is routed
    /// (by a prior reader or this one).  Holds the reader token for
    /// the whole loop — including across the parks inside frame_one;
    /// that IS the design (the parked reader serves everyone else's
    /// replies — the cpunote/console-stdin shape).
    fn read_loop(&self, io: &mut MntIoBox, mine: &Arc<MntSlot>) -> Result<Vec<u8>> {
        loop {
            if let Some(r) = mine.take() {
                return Ok(r);
            }
            let msg = Self::frame_one(io, self.msize)?;
            let rtag = u16::from_le_bytes(msg[5..7].try_into().unwrap());
            if !self.route(msg, rtag) {
                println!("devmnt: unexpected reply tag {rtag}");
                return Err("devmnt: unexpected reply tag");
            }
        }
    }

    /// The reply validation (the old transact's tail, run by the
    /// OWNER on its routed reply — every R-message is untrusted input
    /// whoever framed it): Rerror maps to a loud errstr with the
    /// server's ename on the serial log; the tag/type checks stay as
    /// defense in depth (routing already matched the tag).
    fn check_reply(reply: &[u8], expect_type: u8, expect_tag: u16) -> Result<()> {
        let rtype = reply[4];
        let rtag = u16::from_le_bytes(reply[5..7].try_into().unwrap());
        if rtype == RERROR {
            let mut cur = RepCur::new(&reply[7..]);
            let ename = cur.string().unwrap_or_else(|_| b"<malformed Rerror>".to_vec());
            println!("devmnt: server error: {}", alloc::string::String::from_utf8_lossy(&ename));
            return Err("devmnt: server error");
        }
        if rtag != expect_tag {
            println!("devmnt: reply tag {rtag} is not the outstanding tag {expect_tag}");
            return Err("devmnt: reply tag mismatch");
        }
        if rtype != expect_type {
            println!("devmnt: reply type {rtype} is not the expected {expect_type}");
            return Err("devmnt: unexpected reply type");
        }
        Ok(())
    }

    /// The whole RPC (the donor mountrpc + mountio): queue the slot,
    /// send the T-message, then gate onto the wire ONE reader at a
    /// time — take the reader token (frame + demux for everyone,
    /// donor mntgate on exit: the token frees at a message boundary)
    /// or poll this rpc's slot while another reader works.  The park
    /// hook bounds the poll; with no hook registered (bare profiles)
    /// contention stays the loud error, exactly as before.
    fn rpc(&self, msg: &[u8], expect_type: u8, expect_tag: u16) -> Result<Vec<u8>> {
        let slot = Arc::new(MntSlot::new(expect_tag));
        {
            let node = LockNode::new();
            self.queue.lock(&node).push_back(slot.clone());
        }
        if let Err(e) = self.send(msg) {
            self.dequeue(&slot);
            return Err(e);
        }
        let reply = loop {
            if let Some(r) = slot.take() {
                break r;
            }
            match self.io_r.enter() {
                Some(mut reader) => {
                    let out = self.read_loop(&mut reader, &slot);
                    drop(reader); // mntgate: the token frees at a message boundary
                    match out {
                        Ok(r) => break r,
                        Err(e) => {
                            self.dequeue(&slot);
                            return Err(e);
                        }
                    }
                }
                None => contention_wait()?,
            }
        };
        Self::check_reply(&reply, expect_type, expect_tag)?;
        Ok(reply)
    }

    /// The Tversion exchange (donor mntversion): offer `offer` (0 →
    /// DEFAULT_MSIZE), send Tversion(NOTAG, msize, "9P2000"), validate the
    /// Rversion (the server never increases the msize — devmnt.c's "server
    /// tries to increase msize"; nonsense bounds 256..=1MiB; the version
    /// string answers "9P2000"), and park the negotiated session.  The
    /// two io halves arrive separately (the read half IS the reader
    /// token's cell — a pipe/ssl dup each, so a parked reader and an
    /// active writer never share one Chan).
    pub fn negotiate(io_r: Box<dyn MntIo>, io_w: Box<dyn MntIo>, offer: u32) -> Result<Mnt> {
        let offer = if offer == 0 { DEFAULT_MSIZE } else { offer };
        if !(MSIZE_MIN..=MSIZE_MAX).contains(&offer) {
            return Err("devmnt: nonsense msize in fversion");
        }
        let mut mnt = Mnt {
            io_r: BusyCell::new(MntIoBox(io_r)),
            io_w: BusyCell::new(MntIoBox(io_w)),
            alloc: Lock::new("mntalloc", MntAlloc { next_fid: 1, next_tag: 1 }),
            queue: Lock::new("mntqueue", VecDeque::new()),
            msize: offer, // the handshake's bound is the offer itself
            version: Vec::new(),
            id: {
                let node = LockNode::new();
                let mut g = NEXT_MNT_ID.lock(&node);
                let id = *g;
                *g += 1;
                id
            },
        };
        let mut body = Vec::new();
        pw32(&mut body, offer);
        pwstr(&mut body, VERSION9P);
        let msg = tmsg(TVERSION, NOTAG, &body);
        // The Mnt is a not-yet-shared local: the token is free and the
        // queue holds only this rpc, so rpc() runs uncontended.
        let reply = mnt.rpc(&msg, RVERSION, NOTAG)?;
        let (msize, version) = {
            let mut cur = RepCur::new(&reply[7..]);
            let msize = cur.u32()?;
            let version = cur.string()?;
            cur.done()?;
            (msize, version)
        };
        if msize > offer {
            return Err("devmnt: server tried to increase msize in fversion");
        }
        if !(MSIZE_MIN..=MSIZE_MAX).contains(&msize) {
            return Err("devmnt: nonsense msize in fversion");
        }
        if version != VERSION9P {
            return Err("devmnt: bad 9P version returned from server");
        }
        mnt.msize = msize;
        mnt.version = version;
        Ok(mnt)
    }

    /// The negotiated msize (a frozen set-once field — no guard).
    pub fn msize(&self) -> u32 {
        self.msize
    }

    /// The negotiated version string (the re-version replay's answer) —
    /// a frozen set-once field (no guard).
    fn version(&self) -> Vec<u8> {
        self.version.clone()
    }

    /// Tattach (donor mntattach): a fresh fid attached to the tree root
    /// with afid = NOFID (no auth this era; nsinit's fall-through
    /// is the designed path).  Returns (fid, the root qid).
    pub fn attach(&self, uname: &[u8], aname: &[u8]) -> Result<(u32, Qid)> {
        let fid = self.alloc_fid()?;
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw32(&mut body, NOFID);
        pwstr(&mut body, uname);
        pwstr(&mut body, aname);
        let msg = tmsg(TATTACH, tag, &body);
        let reply = self.rpc(&msg, RATTACH, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let qid = cur.qid()?;
        cur.done()?;
        Ok((fid, qid))
    }

    /// Twalk one component from `fid` to a fresh fid (donor mntwalk,
    /// single-name shape — namec walks one component per call).  Returns
    /// (newfid, the walked qid); a zero-qid Rwalk or the server's Rerror
    /// is the loud not-found.
    pub fn walk(&self, fid: u32, name: &[u8]) -> Result<(u32, Qid)> {
        if name.is_empty() || name.contains(&b'/') {
            return Err("devmnt: walk takes a single name component");
        }
        let newfid = self.alloc_fid()?;
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw32(&mut body, newfid);
        pw16(&mut body, 1); // nwname
        pwstr(&mut body, name);
        let msg = tmsg(TWALK, tag, &body);
        let reply = self.rpc(&msg, RWALK, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let nwqid = cur.u16()? as usize;
        // The donor's bounds: nwqid never exceeds MAXWELEM nor nwname (1).
        if nwqid > MAXWELEM || nwqid > 1 {
            return Err("devmnt: too many qids returned by walk");
        }
        if nwqid == 0 {
            return Err("devmnt: name not found");
        }
        let qid = cur.qid()?;
        cur.done()?;
        Ok((newfid, qid))
    }

    /// A fid CLONE (the mount-table aliasing fix):
    /// Twalk with ZERO names (the 9P fid-clone idiom — donor mntwalk's
    /// nwname==0 arm) clones the fid's position onto a fresh fid.  The
    /// mount table's entry chans are SHARED with namec's redirects; a
    /// bare dup shares the server-side fid, so an open+close of the
    /// redirect (ls /bin's listing) Tclunked the TABLE's fid and every
    /// later walk from it Rerror'd (paqfs: a clunked fid's busy==0 →
    /// Enotexist).  The redirects now clone the fid, so the open/clunk
    /// lifecycle rides the clone and the table's fid stays pristine.
    pub fn clone_fid(&self, fid: u32) -> Result<u32> {
        let newfid = self.alloc_fid()?;
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw32(&mut body, newfid);
        pw16(&mut body, 0); // nwname = 0 — the fid-clone idiom
        let msg = tmsg(TWALK, tag, &body);
        let reply = self.rpc(&msg, RWALK, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let nwqid = cur.u16()? as usize;
        // A zero-name walk answers nwqid = 0; the clone carries the
        // source's qid (the position is unchanged by construction).
        if nwqid != 0 {
            return Err("devmnt: fid clone returned qids");
        }
        cur.done()?;
        Ok(newfid)
    }

    /// Topen (donor mntopen): the mode byte is Plan 9's open-mode byte
    /// (the low 8 bits — OEXCL doesn't fit and is create-only, which this
    /// era never sends).  Returns (the opened qid, the iounit after the
    /// donor's clamp).
    pub fn open(&self, fid: u32, mode: u8) -> Result<(Qid, u32)> {
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        body.push(mode);
        let msg = tmsg(TOPEN, tag, &body);
        let reply = self.rpc(&msg, ROPEN, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let qid = cur.qid()?;
        let iounit = cur.u32()?;
        cur.done()?;
        // The donor's clamp (mntopencreate): 0 or oversized → the
        // session's per-message data cap.  The donor's cap is
        // msize-IOHDRSZ; THIS era's transport (devpipe's whole-message
        // 4096 bound) caps per-rpc DATA at rpc_data_cap below that, and
        // the iounit is the caller's max-single-write contract — it must
        // never exceed what one rpc can carry (19-10: libdraw sizes its
        // draw buffer by iounit; a cap above the transport split draw
        // writes mid-message).
        let cap = rpc_data_cap(self.msize) as u32;
        let iounit = if iounit == 0 || iounit > cap { cap } else { iounit };
        Ok((qid, iounit))
    }

    /// One Tread RPC (donor mntrdwr's inner loop): the CHAN-level read
    /// clamps count to the era's per-RPC bound; the reply's count must
    /// exactly cover the message body (convM2S's exact-size check) and is
    /// clamped to the request (the donor's silent clamp).
    pub fn read(&self, fid: u32, off: u64, count: u32) -> Result<Vec<u8>> {
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw64(&mut body, off);
        pw32(&mut body, count);
        let msg = tmsg(TREAD, tag, &body);
        let reply = self.rpc(&msg, RREAD, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let rcount = cur.u32()? as usize;
        let remaining = cur.b.len() - cur.p;
        if rcount != remaining {
            return Err("devmnt: malformed read reply (count/body mismatch)");
        }
        let keep = rcount.min(count as usize);
        let data = cur.bytes(rcount)?.to_vec();
        cur.done()?;
        Ok(data[..keep].to_vec())
    }

    /// One Twrite RPC (donor mntrdwr's write inner loop): count is clamped
    /// by the caller to the era's per-RPC bound; the reply's count is
    /// clamped to the request (the donor's silent clamp).
    pub fn write(&self, fid: u32, off: u64, data: &[u8]) -> Result<u32> {
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw64(&mut body, off);
        pw32(&mut body, data.len() as u32);
        body.extend_from_slice(data);
        let msg = tmsg(TWRITE, tag, &body);
        let reply = self.rpc(&msg, RWRITE, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let count = cur.u32()?;
        cur.done()?;
        Ok(count.min(data.len() as u32))
    }

    /// One Tcreate RPC (donor mntcreate): the perm is the FULL 32-bit
    /// word (DMDIR and friends ride the high bits — keyfs's DMDIR
    /// creates are the measured caller, 15-03); the mode byte is the
    /// low 8 (OTRUNC/ORCLOSE ride the wire per fcall.h).
    pub fn create(&self, fid: u32, name: &[u8], perm: u32, mode: u8) -> Result<(Qid, u32)> {
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pwstr(&mut body, name);
        pw32(&mut body, perm);
        body.push(mode);
        let msg = tmsg(TCREATE, tag, &body);
        let reply = self.rpc(&msg, RCREATE, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let qid = cur.qid()?;
        let iounit = cur.u32()?;
        cur.done()?;
        // The donor's clamp (mntopencreate): 0 or oversized → the
        // session's per-message data cap.  The donor's cap is
        // msize-IOHDRSZ; THIS era's transport (devpipe's whole-message
        // 4096 bound) caps per-rpc DATA at rpc_data_cap below that, and
        // the iounit is the caller's max-single-write contract — it must
        // never exceed what one rpc can carry (19-10: libdraw sizes its
        // draw buffer by iounit; a cap above the transport split draw
        // writes mid-message).
        let cap = rpc_data_cap(self.msize) as u32;
        let iounit = if iounit == 0 || iounit > cap { cap } else { iounit };
        Ok((qid, iounit))
    }

    /// Tstat (donor mntstat): the reply's stat payload (nstat bytes — the
    /// stat(5) image WITH its own 2-byte prefix, whose value must be
    /// nstat-2 — the malformation check).
    pub fn stat(&self, fid: u32) -> Result<Vec<u8>> {
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        let msg = tmsg(TSTAT, tag, &body);
        let reply = self.rpc(&msg, RSTAT, tag)?;
        let mut cur = RepCur::new(&reply[7..]);
        let nstat = cur.u16()? as usize;
        if nstat < 2 {
            return Err("devmnt: malformed stat reply");
        }
        let payload = cur.bytes(nstat)?;
        // The payload's own prefix must count its remainder (stat(5)
        // self-consistency — a lying server never gets trusted bytes).
        let inner = u16::from_le_bytes(payload[0..2].try_into().unwrap()) as usize;
        if inner != nstat - 2 {
            return Err("devmnt: malformed stat reply (bad inner prefix)");
        }
        let out = payload.to_vec();
        cur.done()?;
        Ok(out)
    }

    /// Tclunk (donor mntclunk): release the fid server-side.
    pub fn clunk(&self, fid: u32) -> Result<()> {
        let tag = self.alloc_tag();
        let mut body = Vec::new();
        pw32(&mut body, fid);
        let msg = tmsg(TCLUNK, tag, &body);
        let reply = self.rpc(&msg, RCLUNK, tag)?;
        RepCur::new(&reply[7..]).done()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The Chan-wrapping device ops (namec.rs's DEV_MNT legs + the syscalls)
// ---------------------------------------------------------------------------

/// The era's per-RPC read bound: min(msize, the transport's whole-message
/// ceiling) minus the header margin.
fn rpc_data_cap(msize: u32) -> usize {
    (msize.min(TRANSPORT_MSG_MAX) - IOHDRSZ) as usize
}

/// sys_fversion's helper (donor sysfversion): negotiate over the fd's chan
/// and park the session on it.  Idempotent on an established session (the
/// donor's c->mux replay: answers from the stored negotiation).  `out` is
/// the caller's version buffer (EMPTY = no write-back, the donor's
/// returnlen==0); returns the version string's length.
pub fn fversion(chan: &mut Chan, offer: u32, out: &mut [u8]) -> Result<u64> {
    // 19-08: the shared-wire rule.  The donor's srvopen returns the
    // registry's chan OBJECT, so the session the first mount negotiated
    // survives on it and every later mount reuses it; r9's devsrv::open
    // hands out a value dup, so the session is hosted on the pipe
    // instead (Pipe::mnt_session, keyed by end).  Without this probe
    // every fresh open+mount of an /srv file negotiates a SECOND session
    // whose fid numbering restarts at 1 while the server (one lib9p Srv
    // per connection) still holds the first session's fids — the
    // `duplicate fid' Rerror (measured at cpu -R's auth_chuid factotum
    // re-mount).
    if chan.mnt.is_none() && chan.dev == DEV_PIPE && let Some(pipe) = &chan.pipe {
        if let Some(mnt) = crate::devpipe::shared_mnt_session(pipe, chan.qid.path) {
            chan.mnt = Some(mnt);
        }
    }
    if let Some(mnt) = &chan.mnt {
        // The donor's c->mux replay: an established session answers from
        // the stored negotiation (and refuses a version CHANGE — the
        // donor's "incompatible 9P versions" — which the era's sole
        // version makes unreachable).
        let version = mnt.version();
        if !out.is_empty() {
            if out.len() < version.len() {
                return Err("fversion: version buffer too small");
            }
            out[..version.len()].copy_from_slice(&version);
        }
        return Ok(version.len() as u64);
    }
    // 19-08: the connection chan may be a PIPE end (the /srv and boot
    // mounts) or a DEVSSL data chan (cpu -R's remoteside fversions the
    // CLIENT's exportfs over the encrypted session — pushssl's returned
    // fd).  The session io wraps either; only the shared-session probe
    // above is pipe-specific.
    if chan.dev != DEV_PIPE && chan.dev != DEV_SSL {
        return Err("fversion: fd is not a 9P-capable connection");
    }
    if chan.mode & OMASK != ORDWR {
        return Err("fversion: fd is not open for read+write");
    }
    // The session's io chans: TWO dups (one per direction — a parked
    // reader and an active writer must hold separate Chan handles).
    // Open-end counts bump per dup (the session holds live references
    // even after the fd closes; the mount must outlive them — boot's
    // fd closes at init's closefds).
    let io_r = ChanIo { chan: chan.dup() };
    let io_w = ChanIo { chan: chan.dup() };
    let mnt = Arc::new(Mnt::negotiate(Box::new(io_r), Box::new(io_w), offer)?);
    if let Some(pipe) = &chan.pipe {
        crate::devpipe::set_shared_mnt_session(pipe, chan.qid.path, mnt.clone());
    }
    let version = mnt.version();
    if !out.is_empty() {
        if out.len() < version.len() {
            return Err("fversion: version buffer too small");
        }
        out[..version.len()].copy_from_slice(&version);
    }
    let len = version.len() as u64;
    chan.mnt = Some(mnt);
    Ok(len)
}

/// sys_mount's attach (donor mntattach via bindmount): attach the session's
/// tree root and return the mount-root chan (DEV_MNT, fresh fid, the
/// server's root qid).  The caller sets the path (the mount target's) and
/// inserts it into the pgrp mount table.  The fid is Arc'd.
pub fn attach(mnt: &Arc<Mnt>, aname: &[u8]) -> Result<Chan> {
    let (fid, qid) = mnt.attach(b"glenda", aname)?;
    let mut c = Chan::new(DEV_MNT, qid, 0);
    c.mnt = Some(mnt.clone());
    c.mnt_fid = Some(Arc::new(fid));
    c.devno = mnt.id;
    Ok(c)
}

/// The DEV_MNT walk leg (namec's dev_walk): Twalk to a fresh fid.
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: walk of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: walk of an fid-less chan");
    };
    if c.qid.typ & QTDIR == 0 {
        return Err("devmnt: walk of a non-directory");
    }
    let (newfid, qid) = mnt.walk(**fid, name)?;
    let mut nc = Chan::new(DEV_MNT, qid, 0);
    nc.mnt = Some(mnt.clone());
    nc.mnt_fid = Some(Arc::new(newfid));
    nc.devno = mnt.id;
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// The DEV_MNT fid-clone leg: namec's mount-point redirects clone
/// the mount TABLE's chan onto a fresh fid (Mnt::clone_fid — the zero-name
/// Twalk) instead of bare-dup'ing it, so a redirect's open+close lifecycle
/// never touches the table's server-side fid (a bare dup let ls's close
/// Tclunk the table's fid, and every
/// later walk from it Rerror'd).
pub fn clone_chan(c: &Chan) -> Result<Chan> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: clone of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: clone of an fid-less chan");
    };
    let newfid = mnt.clone_fid(**fid)?;
    let mut nc = Chan::new(DEV_MNT, c.qid, c.mode);
    nc.mnt = Some(mnt.clone());
    nc.mnt_fid = Some(Arc::new(newfid));
    nc.devno = mnt.id;
    nc.path = c.path.clone();
    Ok(nc)
}

/// The DEV_MNT open leg (namec's open_chan): Topen; directory opens are
/// OREAD-only (the donor namec discipline); OTRUNC/ORCLOSE ride the wire
/// byte for the server to answer (paqfs refuses both — Erdonly).
/// The open SHARES the walked fid's Arc (the donor's
/// open-doesn't-realloc shape) — the walked chan and the opened chan are
/// two refs on ONE server-side fid, and the last ref's close dispatches
/// the Tclunk.
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: open of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: open of an fid-less chan");
    };
    let access = mode & OMASK;
    if c.qid.typ & QTDIR != 0 && access != OREAD {
        return Err("devmnt: directory opens OREAD only");
    }
    let (qid, iounit) = mnt.open(**fid, (mode & 0xff) as u8)?;
    let mut nc = Chan::new(DEV_MNT, qid, access);
    nc.mnt = Some(mnt.clone());
    nc.mnt_fid = c.mnt_fid.clone();
    nc.devno = mnt.id;
    nc.path = c.path.clone();
    // The donor's c->iounit = reply.iounit (clamped) — mnt.open already
    // clamped (0 or over-cap → the cap), so the opened chan carries the
    // per-rpc data cap devdup's #d/Nctl line reports (19-10: libdraw's
    // iounit(2) sizes its draw-message buffer; an iounit of 0 fell back
    // to 8000, over this era's 4072 transport cap, and the mid-message
    // write split killed cpu-session rio at `can't open display`).
    nc.iounit = iounit;
    Ok(nc)
}

/// The DEV_MNT read leg (sys_pread): chunked Treads (donor mntrdwr — stop
/// on a short reply or a full buffer); u64::MAX offset uses + advances the
/// chan's streaming offset (the pread convention).
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: read of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: read of an fid-less chan");
    };
    let cap = rpc_data_cap(mnt.msize());
    let mut off = if offset == u64::MAX { c.offset } else { offset };
    let mut done = 0usize;
    // Do-while, the write loop's donor-faithful shape (mntrdwr runs at
    // least one rpc — a zero-length request is a real protocol event).
    loop {
        let want = (buf.len() - done).min(cap);
        let data = mnt.read(**fid, off, want as u32)?;
        let n = data.len();
        buf[done..done + n].copy_from_slice(&data);
        off += n as u64;
        done += n;
        if n < want || done == buf.len() {
            // a short reply = EOF (the donor break); done == len covers
            // the zero-length first pass.
            break;
        }
    }
    if offset == u64::MAX {
        c.offset += done as u64;
    }
    Ok(done as u64)
}

/// The DEV_MNT write leg (sys_pwrite): chunked Twrites (donor mntrdwr —
/// the read leg's mirror); u64::MAX offset uses + advances the chan's
/// streaming offset (the pwrite convention).  The mounted tree stopped
/// being read-only the day the services venues needed ctl writes through
/// srv-mounted daemons (15-03: cs/dns query writes, factotum's ctl
/// round-trip) — the SERVER still enforces its own permissions (paqfs
/// keeps refusing writes; Erdonly is paqfs's answer, not devmnt's).
pub fn write(c: &mut Chan, buf: &[u8], offset: u64) -> Result<u64> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: write of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: write of an fid-less chan");
    };
    let cap = rpc_data_cap(mnt.msize());
    let mut off = if offset == u64::MAX { c.offset } else { offset };
    let mut done = 0usize;
    // The donor mntrdwr is a do-while: AT LEAST ONE rpc goes out even
    // for a ZERO-LENGTH write (request.count = 0) — the server OBSERVES
    // the empty write and some devices carry semantics on exactly that
    // event (drawterm's devmouse Qcursor: n < 72 is the ARROW RESET —
    // libdraw setcursor(nil) writes 0 bytes; a skip-when-empty loop
    // swallowed it and the cursor stuck at the last contextual shape:
    // the 19-10 Leg-1 cont-5 owner report, no zero-length Qcursor ever
    // observed host-side across every retained trace).
    loop {
        let take = (buf.len() - done).min(cap);
        let wrote = mnt.write(**fid, off, &buf[done..done + take])? as usize;
        off += wrote as u64;
        done += wrote;
        if wrote < take || done == buf.len() {
            // a short reply = the server's fill point; done == len
            // covers the zero-length case (0 == 0 on the first pass).
            break;
        }
    }
    if offset == u64::MAX {
        c.offset += done as u64;
    }
    Ok(done as u64)
}

/// The DEV_MNT create leg (namec's create_chan): Tcreate TRANSFORMS the
/// fid it is handed (fcall.h), so the create rides a clone of the
/// parent's fid and the created chan owns the clone; the parent's fid
/// stays a directory fid.
pub fn create(c: &Chan, name: &[u8], mode: u32, perm: u32) -> Result<Chan> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: create of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: create of an fid-less chan")
    };
    if c.qid.typ & QTDIR == 0 {
        return Err("devmnt: create under a non-directory");
    }
    // Tcreate TRANSFORMS the fid it is handed (fcall.h: on success the
    // fid represents the created file) — so the create rides a CLONE,
    // and the parent's fid (the Arc the walked chan shares) stays a
    // directory fid (the donor mntcreate's clone-then-create shape).
    let newfid = mnt.clone_fid(**fid)?;
    let (qid, iounit) = match mnt.create(newfid, name, perm, (mode & 0xff) as u8) {
        Ok(v) => v,
        Err(msg) => {
            // The clone never became a file — clunk it best-effort so a
            // create the server refuses does not leak the fid
            // server-side.  Load-bearing since 17-05: namec's
            // create_chan_union tries union members in chain order, so
            // a refusing member is on the normal resolution path, not a
            // rare error tail.
            if let Err(cmsg) = mnt.clunk(newfid) {
                println!("devmnt: clunk of failed-create fid {newfid} failed: {cmsg}");
            }
            return Err(msg);
        }
    };
    let mut nc = Chan::new(DEV_MNT, qid, mode & OMASK);
    nc.mnt = Some(mnt.clone());
    nc.mnt_fid = Some(Arc::new(newfid));
    nc.devno = mnt.id;
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    // The donor's iounit leg for creates too (mntopencreate serves both
    // open and create — see open()'s note): the created chan carries the
    // clamped reply iounit.
    nc.iounit = iounit;
    Ok(nc)
}

/// The DEV_MNT stat leg (sys_stat/sys_fstat): Tstat, the donor mntstat's
/// truncation shape (a tight buffer gets just the count prefix).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: stat of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: stat of an fid-less chan");
    };
    let payload = mnt.stat(**fid)?;
    if payload.len() > sb.len() {
        // The donor mntstat truncation: the count prefix alone.
        if sb.len() < 2 {
            return Err("devmnt: stat buffer too small");
        }
        let n = (payload.len() - 2) as u16;
        sb[0..2].copy_from_slice(&n.to_le_bytes());
        return Ok(2);
    }
    sb[..payload.len()].copy_from_slice(&payload);
    Ok(payload.len() as u64)
}

/// The DEV_MNT close dispatch (namec::close_chan): Tclunk, best-effort —
/// a failure logs and swallows (the donor's cclose never fails).
pub fn clunk(c: &Chan) {
    let Some(mnt) = &c.mnt else { return };
    let Some(fid) = &c.mnt_fid else { return };
    if let Err(msg) = mnt.clunk(**fid) {
        println!("devmnt: clunk of fid {} failed: {msg}", fid);
    }
}

/// Tclunk ONLY when
/// this chan is the fid's LAST live reference (Arc::strong_count == 1).
/// Open shares the walked fid's Arc and dup clones it, so every close
/// dispatch that is not the last ref skips — a value copy's
/// double-Tclunk divergence is impossible by construction (paqfs's rclunk
/// no-op semantics stop being load-bearing).  Walked-but-never-opened
/// chans hold fids too: namec's close arm and walk sweep call this
/// REGARDLESS of copen.  Best-effort like clunk: a failure logs and
/// swallows.
pub fn clunk_if_last(c: &Chan) {
    let Some(fid) = &c.mnt_fid else { return };
    if Arc::strong_count(fid) != 1 {
        return;
    }
    clunk(c);
}

/// sys_exec's byte source for a mounted binary:
/// read the WHOLE file eagerly (syscall context — RPCs block by design;
/// the trap-context pager must never RPC).  Bounded by `cap` (loud error,
/// never an unbounded read off an untrusted server).  Reads may
/// reach cap+1 bytes so an EXACTLY-cap file terminates via the empty-read
/// EOF arm (a top-of-loop `out.len() >= cap` check would error it
/// without ever attempting the final read); only a byte past the cap is
/// "too large".
pub fn read_all(c: &Chan, cap: usize) -> Result<Vec<u8>> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: read of a non-mount chan") };
    let Some(fid) = &c.mnt_fid else {
        return Err("devmnt: read of an fid-less chan");
    };
    let rpc_cap = rpc_data_cap(mnt.msize());
    let mut out = Vec::new();
    loop {
        // want is always ≥ 1 here: out.len() ≤ cap (a larger length
        // errored below), so cap + 1 - out.len() ≥ 1.
        let want = rpc_cap.min(cap + 1 - out.len());
        let data = mnt.read(**fid, out.len() as u64, want as u32)?;
        if data.is_empty() {
            break; // EOF — the file fits within the cap
        }
        out.extend_from_slice(&data);
        if out.len() > cap {
            return Err("devmnt: file too large to stage for exec");
        }
        if data.len() < want {
            break; // a short reply = EOF
        }
    }
    Ok(out)
}
