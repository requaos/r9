//! devmnt — the kernel 9P client `#M` (05-07, D-03's mount half; the
//! virtio-9p prerequisite for v1.x).  Semantics ported from 9k's
//! devmnt.c (the RESEARCH Don't-Hand-Roll mandate — mntversion/mntattach/
//! mntwalk/mntopen/mntread/mntstat/mntclunk over the pipe fd, sized to
//! the paqfs message set) with the wire codec from fcall.h + convS2M.c /
//! convM2S.c.
//!
//! A mounted tree is a SESSION: sys_fversion negotiates the msize over
//! the fd's chan (Tversion/Rversion — "9P2000", offer 8192, accept the
//! server's lower answer) and parks the resulting `Arc<Mnt>` ON the chan
//! (dat.rs's `mnt` slot — the donor's `c->mux`); sys_mount attaches the
//! tree (Tattach, afid = NOFID — the D-02 no-auth path, nsinit's
//! fall-through) and inserts the mount-table entry at the resolved
//! target.  Chans in the mounted tree carry the session Arc plus their
//! server-side fid; namec's DEV_MNT legs dispatch walk/open/read/stat to
//! Twalk/Topen/Tread/Tstat RPCs, and namec::close_chan's DEV_MNT leg
//! sends Tclunk when an OPENED mnt chan leaves an fd slot.
//!
//! T-05-13 (the mntchk discipline, ported): EVERY R-message off the pipe
//! is untrusted input from a userspace server.  The receive path
//! bounds-checks the message size against the negotiated msize BEFORE
//! reading the body, tag-matches the reply against the outstanding
//! T-message, requires the reply type to be request+1 (Rerror maps to a
//! loud errstr), and parses every field through a checked cursor —
//! protocol failures are "devmnt: …" errstrs, never panics, never partial
//! reads, never trusted bytes.  Host tests prove rejection of oversized
//! and mis-tagged replies (port/tests/devmnt_test.rs).
//!
//! Concurrency this era (documented era limit): exactly ONE RPC is
//! outstanding per session — syscalls are serialized and the boot chain's
//! client side is strictly sequential (init parks in await while rc runs;
//! fork() copies the fd table, and no measured flow shares a session
//! across concurrently-running procs).  A second concurrent RPC gets a
//! LOUD error ("devmnt: concurrent rpc this era"), never a wedge — the
//! tag-multiplexing reader (the donor's mountio/mountmux) arrives with
//! the virtio-9p/SMP era.  The busy flag is a plain AtomicBool because
//! the RPC's recv BLOCKS (the pipe read schedules the server proc): an
//! MCS lock held across that block would spin-deadlock a contender
//! (IRQ-masked syscall context), so ownership is the flag, not the lock.
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
//! Fid lifecycle this era: fids are allocated at walk/attach from a
//! per-session counter (never reused — paqfs's Fid is ~50 bytes and the
//! boot chain walks dozens, not millions); Tclunk fires on the close
//! dispatch (opened chans) and on exec's explicit close.  Walk
//! intermediates that never reach an fd leak their fid — bounded and
//! documented (the donor clunks on cclose; r9's value-chan drops are
//! invisible to devices).

// The RED skeleton leaves most of the codec/constants unconstructed —
// silence the dead-code noise until GREEN fills the bodies.
#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::AtomicBool;

use crate::dat::{Chan, Qid, DEV_PIPE};
use crate::devpipe;
use crate::Result;

// ---------------------------------------------------------------------------
// The wire codec (fcall.h + convS2M.c/convM2S.c, ported for the client set)
// ---------------------------------------------------------------------------

/// 9P2000 message types (fcall.h — frozen values).
pub(crate) const TVERSION: u8 = 100;
pub(crate) const RVERSION: u8 = 101;
pub(crate) const TATTACH: u8 = 104;
pub(crate) const RATTACH: u8 = 105;
pub(crate) const RERROR: u8 = 107;
pub(crate) const TWALK: u8 = 110;
pub(crate) const RWALK: u8 = 111;
pub(crate) const TOPEN: u8 = 112;
pub(crate) const ROPEN: u8 = 113;
pub(crate) const TREAD: u8 = 116;
pub(crate) const RREAD: u8 = 117;
pub(crate) const TCLUNK: u8 = 120;
pub(crate) const RCLUNK: u8 = 121;
pub(crate) const TSTAT: u8 = 124;
pub(crate) const RSTAT: u8 = 125;

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

// ---------------------------------------------------------------------------
// The transport (the pipe fd's chan in the kernel; a scripted fake server
// in host tests)
// ---------------------------------------------------------------------------

/// The 9P byte transport: one whole message per write; reads return up to
/// the buffer's length and BLOCK until at least one byte is available
/// (the kernel transport's devpipe read schedules the server proc).
pub trait MntIo {
    /// Write one whole message, atomically (devpipe's contract: the full
    /// buffer lands or the write fails — never a partial message).
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64>;
    /// Read available bytes (blocks on empty); 0 = EOF (the peer hung up).
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64>;
}

/// The kernel transport: a 9P-capable chan (the pipe end).  Reads/writes
/// ride devpipe's ops directly (streaming — offsets don't apply to pipe
/// ends); anything else is a loud contract violation (sys_fversion/sys_mount
/// validate the fd before constructing this).
pub struct ChanIo {
    pub chan: Chan,
}

impl MntIo for ChanIo {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        match self.chan.dev {
            DEV_PIPE => devpipe::write(&mut self.chan, buf, u64::MAX),
            _ => Err("devmnt: fd is not a 9P-capable connection"),
        }
    }
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        match self.chan.dev {
            DEV_PIPE => devpipe::read(&mut self.chan, buf, u64::MAX),
            _ => Err("devmnt: fd is not a 9P-capable connection"),
        }
    }
}

// ---------------------------------------------------------------------------
// The session
// ---------------------------------------------------------------------------

/// The RPC-critical-section state: only the busy-flag OWNER touches it
/// (the flag IS the mutex — see the module header's concurrency note).
struct MntCore {
    io: Box<dyn MntIo>,
    /// The negotiated msize (the reply's answer, ≤ the offer).
    msize: u32,
    /// The negotiated version string (answered on re-version, the donor's
    /// c->mux replay in mntversion).
    version: Vec<u8>,
    /// Fid/tag sequences (donor mntchan/alloctag): fids from 1 up (never
    /// NOFID, never reused this era); tags from 1 up (never 0, never
    /// NOTAG — devmnt.c's mntreset exclusions).
    next_fid: u32,
    next_tag: u16,
    /// The msize-sized receive scratch (HEAP — an 8 KiB stack buffer is
    /// the 05-06 kstack-overflow lesson).
    rx: Vec<u8>,
}

/// A 9P client session (the donor's `Mnt`): one connection's negotiated
/// state plus the fid/tag allocators.  Shared by every chan in the
/// mounted tree (and by the pipe chan that fversion created it over) as
/// `Arc<Mnt>`; the busy flag serializes RPCs (one outstanding — the era's
/// documented limit).
pub struct Mnt {
    busy: AtomicBool,
    core: UnsafeCell<MntCore>,
}

// SAFETY: the busy flag is the ownership discipline (exactly one RPC owns
// the core at a time; every other caller gets a loud error).  Syscall
// context is IRQ-masked and single-CPU, so the swap is the whole protocol.
unsafe impl Send for Mnt {}
unsafe impl Sync for Mnt {}

impl core::fmt::Debug for Mnt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Mnt(9p session)")
    }
}

impl Mnt {
    /// The Tversion exchange (donor mntversion): offer `offer` (0 →
    /// DEFAULT_MSIZE), send Tversion(NOTAG, msize, "9P2000"), validate the
    /// Rversion (the server never increases the msize — devmnt.c's "server
    /// tries to increase msize"; nonsense bounds 256..=1MiB; the version
    /// string answers "9P2000"), and park the negotiated session.
    pub fn negotiate(io: Box<dyn MntIo>, offer: u32) -> Result<Mnt> {
        let _ = (io, offer);
        Err("devmnt: RED skeleton (05-07)")
    }

    /// The negotiated msize.
    pub fn msize(&self) -> u32 {
        0
    }

    /// Tattach (donor mntattach): a fresh fid attached to the tree root
    /// with afid = NOFID (D-02 — no auth this phase; nsinit's fall-through
    /// is the designed path).  Returns (fid, the root qid).
    pub fn attach(&self, uname: &[u8], aname: &[u8]) -> Result<(u32, Qid)> {
        let _ = (uname, aname);
        Err("devmnt: RED skeleton (05-07)")
    }

    /// Twalk one component from `fid` to a fresh fid (donor mntwalk,
    /// single-name shape — namec walks one component per call).  Returns
    /// (newfid, the walked qid); a zero-qid Rwalk or the server's Rerror
    /// is the loud not-found.
    pub fn walk(&self, fid: u32, name: &[u8]) -> Result<(u32, Qid)> {
        let _ = (fid, name);
        Err("devmnt: RED skeleton (05-07)")
    }

    /// Topen (donor mntopen): the mode byte is Plan 9's open-mode byte
    /// (the low 8 bits — OEXCL doesn't fit and is create-only, which this
    /// era never sends).  Returns (the opened qid, iounit).
    pub fn open(&self, fid: u32, mode: u8) -> Result<(Qid, u32)> {
        let _ = (fid, mode);
        Err("devmnt: RED skeleton (05-07)")
    }

    /// One Tread RPC (donor mntrdwr's inner loop): count is clamped to
    /// the era's per-RPC bound by the CHAN-level read (min(msize-IOHDRSZ,
    /// TRANSPORT_MSG_MAX-IOHDRSZ)); the reply's count never exceeds the
    /// request (the donor's silent clamp) nor the message's actual body
    /// (a short body is malformed — T-05-13).
    pub fn read(&self, fid: u32, off: u64, count: u32) -> Result<Vec<u8>> {
        let _ = (fid, off, count);
        Err("devmnt: RED skeleton (05-07)")
    }

    /// Tstat (donor mntstat): the reply's stat payload (nstat bytes — the
    /// stat(5) image WITH its own 2-byte prefix, whose value must be
    /// nstat-2 — the malformation check).
    pub fn stat(&self, fid: u32) -> Result<Vec<u8>> {
        let _ = fid;
        Err("devmnt: RED skeleton (05-07)")
    }

    /// Tclunk (donor mntclunk): release the fid server-side.
    pub fn clunk(&self, fid: u32) -> Result<()> {
        let _ = fid;
        Err("devmnt: RED skeleton (05-07)")
    }
}

// ---------------------------------------------------------------------------
// The Chan-wrapping device ops (namec.rs's DEV_MNT legs + the syscalls)
// ---------------------------------------------------------------------------

/// sys_fversion's helper (donor sysfversion): negotiate over the fd's chan
/// and park the session on it.  Idempotent on an established session (the
/// donor's c->mux replay: answers from the stored negotiation).  `out` is
/// the caller's version buffer (EMPTY = no write-back, the donor's
/// returnlen==0); returns the version string's length.
pub fn fversion(chan: &mut Chan, offer: u32, out: &mut [u8]) -> Result<u64> {
    let _ = (chan, offer, out);
    Err("devmnt: RED skeleton (05-07)")
}

/// sys_mount's attach (donor mntattach via bindmount): attach the session's
/// tree root and return the mount-root chan (DEV_MNT, fresh fid, the
/// server's root qid).  The caller sets the path (the mount target's) and
/// inserts it into the pgrp mount table.
pub fn attach(mnt: &Arc<Mnt>, aname: &[u8]) -> Result<Chan> {
    let _ = (mnt, aname);
    Err("devmnt: RED skeleton (05-07)")
}

/// The DEV_MNT walk leg (namec's dev_walk): Twalk to a fresh fid.
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    let _ = (c, name);
    Err("devmnt: RED skeleton (05-07)")
}

/// The DEV_MNT open leg (namec's open_chan): Topen; directory opens are
/// OREAD-only (the donor namec discipline); OTRUNC/ORCLOSE ride the wire
/// byte for the server to answer (paqfs refuses both — Erdonly).
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let _ = (c, mode);
    Err("devmnt: RED skeleton (05-07)")
}

/// The DEV_MNT read leg (sys_pread): chunked Treads (donor mntrdwr — stop
/// on a short reply or a full buffer); u64::MAX offset uses + advances the
/// chan's streaming offset (the pread convention).
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let _ = (c, buf, offset);
    Err("devmnt: RED skeleton (05-07)")
}

/// The DEV_MNT stat leg (sys_stat/sys_fstat): Tstat, the donor mntstat's
/// truncation shape (a tight buffer gets just the count prefix).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let _ = (c, sb);
    Err("devmnt: RED skeleton (05-07)")
}

/// The DEV_MNT close dispatch (namec::close_chan): Tclunk, best-effort —
/// a failure logs and swallows (the donor's cclose never fails).
pub fn clunk(c: &Chan) {
    let _ = c;
}

/// sys_exec's byte source for a mounted binary (05-07 Task 2's wiring):
/// read the WHOLE file eagerly (syscall context — RPCs block by design;
/// the trap-context pager must never RPC).  Bounded by `cap` (loud error,
/// never an unbounded read off an untrusted server).
pub fn read_all(c: &Chan, cap: usize) -> Result<Vec<u8>> {
    let _ = (c, cap);
    Err("devmnt: RED skeleton (05-07)")
}
