//! devpipe — the pipe device `#|` (05-06, D-04's connectembed substrate;
//! semantics ported from 9k's devpipe.c + qio.c — the RESEARCH
//! Don't-Hand-Roll mandate).
//!
//! A pipe is ONE shared Pipe object both ends reference
//! (`Arc<Lock<Pipe>>` on the chan — Pitfall 4: dat.rs's value-copy dup
//! would give each end its own bytes and the connectembed 9P exchange
//! would deadlock silently).  The Pipe holds TWO bounded queues,
//! cross-connected per the donor: end 0 (`data`) reads q[0] / writes
//! q[1]; end 1 (`data1`) reads q[1] / writes q[0].  (The plan's "single
//! shared queue" prose names the shared PIPE STATE both ends reference;
//! the paqfs stdio flow — io(1) reads AND writes fd 1 — is only coherent
//! with the donor's direction pair: a single FIFO would let a server
//! consume its own replies.)
//!
//! Read on an empty open queue BLOCKS the calling proc (the 04-07
//! block/wakeup machinery: the arch-side hook marks Blocked + swtch'es;
//! a write or close wakes the registered waiters).  The block/wake is
//! dependency-injected (the devproc set_post_hook idiom) because port
//! cannot see the arch's proc table.  Invariant (load-bearing): syscalls
//! are serialized (single CPU, IRQ-masked syscall context), so
//! register-waiter (under the Pipe lock) → drop-lock → block() cannot
//! lose a wakeup — the waker can only run after block() swtch'es.
//!
//! Close semantics (donor pipeclose): the last close of end i hangs up
//! q[1-i] (its readers see EOF after draining) and closes q[i] (writers
//! get a named error); both ends closed → the queues reopen (donor
//! qreopen).  Open-end counts (qref) track OPEN data chans: open bumps,
//! Chan::dup of an open pipe chan bumps (dat.rs documents the rule),
//! namec::close_chan decrements.
//!
//! Overflow: writes that would exceed the 4 KiB queue bound error with a
//! named errstr (never silent loss, never partial — 9P messages must stay
//! whole).  Blocking-write is a later-era concern (no measured caller).
//!
//! The dat.rs `Dev` trait is NOT implemented here (its skeleton is
//! non-constructible — see devroot.rs's header); attach/walk/open/read/
//! write/stat/close are free functions, dispatched by chan.dev via
//! namec.rs's legs (registry rune '|').
//!
//! Qid scheme (9k's PIPEQID): path = (pipe_id << 5) | tag where tag is 0
//! (the dir), 1 (`data`, end 0), 2 (`data1`, end 1).

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_PIPE, OREAD};
use crate::mcslock::{Lock, LockNode};
use crate::println;
use crate::Result;

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// Qid tags (the low 5 bits of the qid path — 9k's Qdir/Qdata0/Qdata1).
const QDIR: u64 = 0;
const QDATA0: u64 = 1;
const QDATA1: u64 = 2;

/// The per-direction queue bound (05-06 plan: "bounded — e.g. 4 KiB";
/// the 9P exchanges here are small).
pub const PIPE_Q_CAP: usize = 4096;

/// One direction's byte queue: the buffered bytes, the hangup/close
/// flags (donor qhangup/qclose), and the pids blocked reading it.
pub struct PipeQueue {
    buf: VecDeque<u8>,
    /// The write side is fully closed (last writer-end close) — readers
    /// drain the remaining bytes then see EOF (a 0 read), never a hang.
    hungup: bool,
    /// The READ side is fully closed — writes error (donor's
    /// "write on closed pipe"; the note-posting half is 05-08's era).
    closed: bool,
    /// Pids blocked reading this queue (woken by write/close).
    waiters: Vec<u64>,
}

impl PipeQueue {
    fn new() -> PipeQueue {
        PipeQueue { buf: VecDeque::new(), hungup: false, closed: false, waiters: Vec::new() }
    }
}

/// The shared pipe object (9k's `Pipe`): two cross-connected queues +
/// the open-end counts per side (donor qref).  Carried by the chans as
/// `Arc<Lock<Pipe>>`; the Lock serializes queue ops from syscall
/// context (a LEAF lock: never held across block()/sched(), never taken
/// from IRQ context, never held while taking PROC_TABLE).
pub struct Pipe {
    q: [PipeQueue; 2],
    qref: [usize; 2],
    id: u64,
}

/// The arch-side block/wake hooks (the devproc.rs set_post_hook idiom —
/// registered once before the first exec; single writer, pre-EL0).
/// `current` names the calling context (the proc pid); `block` parks the
/// caller mid-syscall (arch: proc::sched()); `wake` readies the pids.
#[derive(Clone, Copy)]
pub struct PipeHooks {
    pub current: fn() -> u64,
    pub block: fn(),
    pub wake: fn(&[u64]),
}

static HOOKS: Lock<Option<PipeHooks>> = Lock::new("devpipe", None);

/// Register the arch-side hooks.  Called once during boot before any
/// user process exists; a second registration is a boot bug.
pub fn set_hooks(hooks: PipeHooks) {
    let node = LockNode::new();
    let mut guard = HOOKS.lock(&node);
    assert!(guard.is_none(), "devpipe: hooks registered twice");
    *guard = Some(hooks);
}

fn hooks() -> Result<PipeHooks> {
    let node = LockNode::new();
    HOOKS.lock(&node).ok_or("devpipe: block/wake hooks not registered")
}

/// The pipe-id sequence (qid path bases) — 9k's pipealloc.path.
static NEXT_ID: Lock<u64> = Lock::new("devpipeid", 1);

/// attach: a FRESH pipe pair per attach (9k's pipeattach — two attaches
/// never share).  Returns the pipe root dir chan carrying the Arc (the
/// walk/open path clones it onto the end chans).
pub fn attach() -> Chan {
    let node = LockNode::new();
    let id = {
        let mut g = NEXT_ID.lock(&node);
        let id = *g;
        *g += 1;
        id
    };
    let pipe = Arc::new(Lock::new(
        "pipe",
        Pipe { q: [PipeQueue::new(), PipeQueue::new()], qref: [0, 0], id },
    ));
    let mut c = Chan::new(DEV_PIPE, Qid { path: (id << 5) | QDIR, vers: 0, typ: QTDIR }, 0);
    c.pipe = Some(pipe);
    c.path = b"#|".to_vec();
    c
}

/// The chan's end (0 for `data`, 1 for `data1`) from its qid tag, or
/// None for the directory.
fn end_of(c: &Chan) -> Option<usize> {
    match c.qid.path & 0x1f {
        QDATA0 => Some(0),
        QDATA1 => Some(1),
        _ => None,
    }
}

/// dat.rs Chan::dup's bump (05-06): every value copy of an OPEN pipe end
/// is a new open reference (the donor's cclone+incref shape) — sys_dup's
/// install, rfork's fd-table copy, devsrv's stored clone all ride it.
/// `qid_path` carries the end tag.  Internal to the port crate.
pub(crate) fn ref_open_end(pipe: &Arc<Lock<Pipe>>, qid_path: u64) {
    let side = match qid_path & 0x1f {
        QDATA0 => 0,
        QDATA1 => 1,
        _ => return,
    };
    let node = LockNode::new();
    pipe.lock(&node).qref[side] += 1;
}

/// Walk a single name component from the pipe root: "data" (end 0) or
/// "data1" (end 1).  The walked chan shares the pipe's Arc (no qref bump
/// — it is not open yet; open bumps).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR {
        return Err("devpipe: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devpipe: walk takes a single name component");
    }
    let tag = match name {
        b"data" => QDATA0,
        b"data1" => QDATA1,
        _ => return Err("devpipe: name not found"),
    };
    let mut nc = Chan::new(DEV_PIPE, Qid { path: (c.qid.path & !0x1f) | tag, vers: 0, typ: QTFILE }, 0);
    nc.pipe = c.pipe.clone();
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open a pipe chan.  The root dir opens OREAD only.  An end opens in any
/// access mode and bumps the open-end count (the donor's pipeopen qref).
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let access = mode & crate::dat::OMASK;
    if c.qid.typ == QTDIR {
        if access != OREAD {
            return Err("devpipe: directory opens OREAD only");
        }
        let mut nc = Chan::new(c.dev, c.qid, access);
        nc.pipe = c.pipe.clone();
        nc.path = c.path.clone();
        return Ok(nc);
    }
    let Some(side) = end_of(c) else {
        return Err("devpipe: open of unknown pipe file");
    };
    let Some(pipe) = &c.pipe else {
        return Err("devpipe: open of unattached pipe chan");
    };
    {
        let node = LockNode::new();
        pipe.lock(&node).qref[side] += 1;
    }
    let mut nc = Chan::new(c.dev, c.qid, access);
    nc.pipe = Some(pipe.clone());
    nc.path = c.path.clone();
    Ok(nc)
}

/// Read a pipe chan.  The root dir serves the two-end stat-wire listing
/// (donor devdirread over pipedir; the streaming-offset convention).  An
/// end read ignores the offset argument (donor qread — pipes are
/// streaming, there are no offsets) and NEVER advances c.offset:
/// queue-empty + write-side-open blocks via the injected hooks (the
/// 04-07 machinery); queue-empty + hungup is EOF (0, never an error).
/// The block loop re-checks after every wake (a spurious wake re-blocks;
/// the waiter set is per-queue and cleared on each wake).
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    if c.qid.typ == QTDIR {
        // The directory listing (devdirread shape): "data" and "data1"
        // records, rebuilt per call (queue lengths shift).
        let off = if offset == u64::MAX { c.offset } else { offset };
        let mut listing = Vec::new();
        let qlens = match &c.pipe {
            Some(pipe) => {
                let node = LockNode::new();
                let g = pipe.lock(&node);
                [g.q[0].buf.len() as u64, g.q[1].buf.len() as u64]
            }
            None => [0, 0],
        };
        for (name, tag, len) in [(b"data".as_slice(), QDATA0, qlens[0]), (b"data1", QDATA1, qlens[1])] {
            let start = listing.len();
            listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
            write_stat_record(&mut listing[start..], name, (c.qid.path & !0x1f) | tag, QTFILE, 0o600, len)?;
        }
        let start = (off as usize).min(listing.len());
        let avail = &listing[start..];
        let n = avail.len().min(buf.len());
        buf[..n].copy_from_slice(&avail[..n]);
        if offset == u64::MAX {
            c.offset += n as u64;
        }
        return Ok(n as u64);
    }
    let Some(side) = end_of(c) else {
        return Err("devpipe: read of unknown pipe file");
    };
    let Some(pipe) = &c.pipe else {
        return Err("devpipe: read of unattached pipe chan");
    };
    // The blocking loop (donor qread): drain if bytes; EOF if hungup;
    // otherwise register as a waiter and block — the lock is DROPPED
    // before block() (a writer must take it to wake us), and the
    // register-under-lock + syscall-serialization invariant (see the
    // module header) makes the wakeup un-losable.  The hooks are fetched
    // lazily: a read that never blocks never requires registration.
    let mut h: Option<PipeHooks> = None;
    loop {
        let node = LockNode::new();
        {
            let mut g = pipe.lock(&node);
            let q = &mut g.q[side];
            if !q.buf.is_empty() {
                let n = q.buf.len().min(buf.len());
                for (i, b) in q.buf.drain(..n).enumerate() {
                    buf[i] = b;
                }
                return Ok(n as u64);
            }
            if q.hungup {
                return Ok(0); // EOF — the write side is fully closed
            }
            if q.closed {
                // Unreachable while we hold an open end (closed sets only
                // at qref[side] == 0); loud rather than a silent EOF.
                return Err("devpipe: read on closed queue");
            }
            if h.is_none() {
                h = Some(hooks()?);
            }
            let pid = (h.unwrap().current)();
            if !q.waiters.contains(&pid) {
                q.waiters.push(pid);
            }
        }
        (h.unwrap().block)(); // park mid-syscall; a write/close wakes us
        crate::println!("pipedbg: read woke pid={}", (h.unwrap().current)()); // gatedbg
    }
}

/// Write to a pipe end (donor pipewrite: end i writes q[1-i]).  Errors
/// when the read side is fully closed ("write on closed pipe") or the
/// write would exceed the queue bound ("pipe full" — atomic, never
/// partial, never silent).  Wakes blocked readers after the bytes land.
pub fn write(c: &mut Chan, buf: &[u8], _offset: u64) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return Err("devpipe: write of a directory");
    }
    let Some(side) = end_of(c) else {
        return Err("devpipe: write of unknown pipe file");
    };
    let Some(pipe) = &c.pipe else {
        return Err("devpipe: write of unattached pipe chan");
    };
    let n = buf.len();
    let node = LockNode::new();
    let waiters = {
        let mut g = pipe.lock(&node);
        let q = &mut g.q[1 - side];
        if q.closed {
            return Err("devpipe: write on closed pipe");
        }
        if n > PIPE_Q_CAP - q.buf.len() {
            return Err("devpipe: pipe full");
        }
        q.buf.extend(buf.iter().copied());
        core::mem::take(&mut q.waiters)
    };
    // Hooks are needed only to WAKE (a write with no blocked readers is
    // hook-free — the dir/listing/queue paths never require registration).
    if !waiters.is_empty() {
        (hooks()?.wake)(&waiters);
    }
    Ok(n as u64)
}

/// Close an open pipe end (namec::close_chan's DEV_PIPE leg — donor
/// pipeclose): decrement the end's open count; the LAST close of end i
/// hangs up q[1-i] (readers at end 1-i drain then see EOF) and closes
/// q[i] (writers error); both ends at zero reopen the queues (donor
/// qreopen).  Wakes the affected waiters after the lock drops.
pub fn close(c: &Chan) {
    let Some(pipe) = &c.pipe else { return };
    let Some(side) = end_of(c) else { return };
    let node = LockNode::new();
    let wake: Vec<u64> = {
        let mut g = pipe.lock(&node);
        if g.qref[side] > 0 {
            g.qref[side] -= 1;
        } else {
            // A close with no recorded open end is a bookkeeping bug —
            // loud in debug builds, harmless in the flow.
            println!("devpipe: close of an unopened end (pipe {} side {})", g.id, side);
        }
        let mut wake = Vec::new();
        if g.qref[side] == 0 {
            // qhangup(q[1-side]): the peer's readers drain then EOF.
            let peer = &mut g.q[1 - side];
            peer.hungup = true;
            wake.append(&mut peer.waiters);
            // qclose(q[side]): writers into it error from now on.
            g.q[side].closed = true;
        }
        if g.qref == [0, 0] {
            // qreopen: both ends closed — the queues reset.
            for q in g.q.iter_mut() {
                q.buf.clear();
                q.hungup = false;
                q.closed = false;
                q.waiters.clear();
            }
        }
        wake
    };
    if !wake.is_empty() && let Ok(h) = hooks() {
        (h.wake)(&wake);
    }
}

/// stat of a pipe chan (sys_stat/sys_fstat's dispatch leg): the root
/// serves "#|" DMDIR|0555; an end serves its name, 0600, and its READ
/// queue's current length (donor pipestat's qlen).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return write_stat_record(sb, b"#|", c.qid.path, QTDIR, 0x8000_0000 | 0o555, 0);
    }
    let Some(side) = end_of(c) else {
        return Err("devpipe: stat of unknown pipe file");
    };
    let len = match &c.pipe {
        Some(pipe) => {
            let node = LockNode::new();
            pipe.lock(&node).q[side].buf.len() as u64
        }
        None => 0,
    };
    let name: &[u8] = if side == 0 { b"data" } else { b"data1" };
    write_stat_record(sb, name, c.qid.path, QTFILE, 0o600, len)
}

/// One stat(5) wire record (the devcons/devproc/devenv shape; uid/gid/muid
/// glenda per D-02).  Returns the byte count (the convD2M contract).
/// FOUR strings — libc's statcheck walks four and rejects a record that
/// ends early (convM2D.c).
fn write_stat_record(
    sb: &mut [u8],
    name: &[u8],
    qid_path: u64,
    typ: u8,
    mode: u32,
    length: u64,
) -> Result<u64> {
    let need = 41 + (2 + name.len()) + 3 * (2 + 6);
    if sb.len() < need {
        return Err("devpipe: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(DEV_PIPE as u32);
    w.u8(typ);
    w.u32(0); // vers
    w.u64(qid_path);
    w.u32(mode);
    w.u32(0); // atime
    w.u32(0); // mtime
    w.u64(length);
    w.string(name);
    w.string(b"glenda");
    w.string(b"glenda");
    w.string(b"glenda"); // muid — the fourth string statcheck walks
    Ok(w.finish())
}
