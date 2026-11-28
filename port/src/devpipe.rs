//! devpipe — the pipe device `#|` (the connectembed substrate;
//! semantics ported from 9k's devpipe.c + qio.c).
//!
//! A pipe is ONE shared Pipe object both ends reference
//! (`Arc<Lock<Pipe>>` on the chan — dat.rs's value-copy dup
//! would give each end its own bytes and the connectembed 9P exchange
//! would deadlock silently).  The Pipe holds TWO bounded queues,
//! cross-connected per the donor: end 0 (`data`) reads q[0] / writes
//! q[1]; end 1 (`data1`) reads q[1] / writes q[0].  (The shared PIPE
//! STATE both ends reference is the point;
//! the paqfs stdio flow — io(1) reads AND writes fd 1 — is only coherent
//! with the donor's direction pair: a single FIFO would let a server
//! consume its own replies.)
//!
//! Read on an empty open queue BLOCKS the calling proc (the
//! block/wakeup machinery: the arch-side hook marks Blocked + swtch'es;
//! a write or close wakes the registered waiters).  The block/wake is
//! dependency-injected (the devproc set_post_hook idiom) because port
//! cannot see the arch's proc table.  Invariant (load-bearing, SMP-03,
//! D-22): while holding the Pipe lock the reader/writer calls
//! `crate::smp::prepare_block()` (clearing the current proc's
//! `wake_pending` with Release) BEFORE inserting its pid into the waiter
//! list, then drops the lock and calls `block()` — any concurrent
//! `wake(pid)` from the other CPU that runs after the insertion sets
//! `wake_pending`, and the arch `block()` (plans 05-10/05-12) consumes
//! that flag and returns immediately instead of parking.  Register-under-
//! lock alone could lose a wake the moment a second CPU scheduled (the
//! waker no longer waits for our block() swtch); prepare_block closes
//! the window.
//!
//! Close semantics (donor pipeclose): the last close of end i hangs up
//! q[1-i] (its readers see EOF after draining) and closes q[i] (writers
//! get a named error); both ends closed → the queues reopen (donor
//! qreopen).  Open-end counts (qref) track OPEN data chans: open bumps,
//! Chan::dup of an open pipe chan bumps (dat.rs documents the rule),
//! namec::close_chan decrements.
//!
//! Overflow: a write the 4 KiB queue has no room for BLOCKS until a
//! reader drains (19-08) and then lands whole; a write larger than the
//! queue itself lands in pieces as room appears (`write`, 2026-11-28).
//! Nothing is ever dropped or refused for its size.
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

/// The per-direction queue bound (bounded — e.g. 4 KiB;
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
    /// "write on closed pipe"; the note-posting half is future work).
    closed: bool,
    /// Pids blocked reading this queue (woken by write/close).
    waiters: Vec<u64>,
    /// Pids blocked WRITING this queue (the queue is full — 19-08's
    /// blocking-write fix; woken by reads that drain and by the close
    /// paths).  The donor's qwrite BLOCKS on a full queue; the
    /// error-on-full shape killed the root paqfs under the tag-mux
    /// (concurrent client rpcs let server replies pile up while their
    /// owners park between polls — measured: `paqfs: mount write' ->
    /// the whole root died).
    wwaiters: Vec<u64>,
}

impl PipeQueue {
    fn new() -> PipeQueue {
        PipeQueue {
            buf: VecDeque::new(),
            hungup: false,
            closed: false,
            waiters: Vec::new(),
            wwaiters: Vec::new(),
        }
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
    /// The 9P session negotiated on each END's wire (19-08: the donor's
    /// srvopen hands out the SAME chan object, so the session its mount
    /// negotiated (c->mux) survives on the registry chan and every later
    /// open+mount REUSES it.  r9's devsrv::open returns a value dup, so
    /// the session must live on the SHARED connection instead — one slot
    /// per end (a session per wire direction), consulted by
    /// devmnt::fversion before negotiating.  Without it every mount of
    /// an /srv file negotiates a fresh session whose fid numbering
    /// restarts at 1 while the server (lib9p's fid space is
    /// per-CONNECTION) still holds the earlier session's fids — the
    /// measured `duplicate fid' Rerror at cpu -R's auth_chuid factotum
    /// re-mount (the first /srv-open-based double mount the tree ever
    /// ran).
    mnt_session: [Option<alloc::sync::Arc<crate::devmnt::Mnt>>; 2],
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
    /// The blocking loops' note probe (the donor's notepending check
    /// around sleep): true = a queued note interrupts this read.
    pub note_pending: fn() -> bool,
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
        Pipe {
            q: [PipeQueue::new(), PipeQueue::new()],
            qref: [0, 0],
            id,
            mnt_session: [None, None],
        },
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

/// dat.rs Chan::dup's bump: every value copy of an OPEN pipe end
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

/// The end-tag consult (19-08): the session a previous mount negotiated
/// on THIS pipe end's wire, if any — devmnt::fversion's reuse probe.
pub(crate) fn shared_mnt_session(pipe: &Arc<Lock<Pipe>>, qid_path: u64) -> Option<alloc::sync::Arc<crate::devmnt::Mnt>> {
    let side = match qid_path & 0x1f {
        QDATA0 => 0,
        QDATA1 => 1,
        _ => return None,
    };
    let node = LockNode::new();
    pipe.lock(&node).mnt_session[side].clone()
}

/// The end-tag store (19-08): remember the session so every later dup
/// of this end (a fresh devsrv::open) reuses it instead of negotiating
/// a colliding second session over the same wire.
pub(crate) fn set_shared_mnt_session(
    pipe: &Arc<Lock<Pipe>>,
    qid_path: u64,
    mnt: alloc::sync::Arc<crate::devmnt::Mnt>,
) {
    let side = match qid_path & 0x1f {
        QDATA0 => 0,
        QDATA1 => 1,
        _ => return,
    };
    let node = LockNode::new();
    let mut guard = pipe.lock(&node);
    if guard.mnt_session[side].is_none() {
        guard.mnt_session[side] = Some(mnt);
    }
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
/// block/wake machinery); queue-empty + hungup is EOF (0, never an error).
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
    // before block() (a writer must take it to wake us).  The 2-CPU
    // lost-wakeup discipline (SMP-03, D-22; the module header):
    // `crate::smp::prepare_block()` runs under the lock BEFORE the pid
    // enters `waiters`, so a wake from the other CPU landing any time
    // after the insertion sets `wake_pending` and the arch `block()`
    // consumes it instead of parking.  The hooks are fetched lazily: a
    // read that never blocks never requires registration.
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
                // The drain made room: wake parked writers (19-08's
                // blocking write — the donor's writer wakeup).
                let wwriters = core::mem::take(&mut q.wwaiters);
                drop(g);
                if !wwriters.is_empty() {
                    let hh = h.unwrap_or_else(|| hooks().expect("devpipe: hooks"));
                    (hh.wake)(&wwriters);
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
            // The donor's notepending pre-check (qread's sleep): a
            // queued note interrupts the read — post_note READIED us
            // (proc.rs push_note), so reaching the block arm again
            // with a pending note means nothing else will.
            if (h.as_ref().unwrap().note_pending)() {
                return Err("interrupted");
            }
            let pid = (h.unwrap().current)();
            // SMP-03 (D-22): clear wake_pending BEFORE the pid becomes
            // wake-targetable; a concurrent wake after the push sets it
            // and block() consumes it instead of sleeping.
            crate::smp::prepare_block();
            if !q.waiters.contains(&pid) {
                q.waiters.push(pid);
            }
        }
        (h.unwrap().block)(); // park mid-syscall; a write/close wakes us
    }
}

/// Write to a pipe end (donor pipewrite: end i writes q[1-i]).
/// BLOCKS while the queue lacks room (the donor's qwrite; 19-08: the
/// old error-on-full killed the root paqfs under the tag-mux's
/// concurrent rpcs).  A write no larger than the queue lands whole: it
/// either fits at once or parks until a reader has drained room for
/// all of it.  A write LARGER than the queue lands in pieces, each as
/// big as the room a reader leaves, and the call returns once the last
/// piece is in — until 2026-11-28 such a write parked for good: the
/// whole-fit test could never pass for it, the reader waited for bytes
/// that never landed, and the pair was dead (`hsort` flushes 8 KiB
/// stdio blocks into a 4 KiB pipe, so `hseq | hsort | cat` stopped for
/// good past 4 KiB of output; measured on x86_64).  Errors when the
/// read side is fully closed; a write cut short after pieces landed
/// reports what landed, never an error that hides it.  Wakes blocked
/// readers after the bytes land; the read path wakes parked writers as
/// it drains.
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
    let mut done = 0usize;
    let mut h: Option<PipeHooks> = None;
    loop {
        if done == n {
            return Ok(done as u64);
        }
        let node = LockNode::new();
        let mut readers: Vec<u64> = Vec::new();
        let mut landed = false;
        {
            let mut g = pipe.lock(&node);
            let q = &mut g.q[1 - side];
            if q.closed {
                return if done > 0 {
                    Ok(done as u64)
                } else {
                    Err("devpipe: write on closed pipe")
                };
            }
            let room = PIPE_Q_CAP - q.buf.len();
            if room > 0 {
                let piece = room.min(n - done);
                q.buf.extend(buf[done..done + piece].iter().copied());
                done += piece;
                readers = core::mem::take(&mut q.waiters);
                landed = true;
            } else {
                // Full: register as a write-waiter and park (the
                // donor's blocking qwrite; a draining read or the
                // close paths wake us).
                if h.is_none() {
                    h = Some(hooks()?);
                }
                // A pending note interrupts the parked write (the
                // note-interruptible blocking-IO contract).
                if (h.as_ref().unwrap().note_pending)() {
                    return if done > 0 {
                        Ok(done as u64)
                    } else {
                        Err("interrupted")
                    };
                }
                let pid = (h.as_ref().unwrap().current)();
                // SMP-03 (D-22): prepare_block before the pid enters
                // wwaiters — the same 2-CPU lost-wakeup invariant as the
                // read loop (see the module header).
                crate::smp::prepare_block();
                if !q.wwaiters.contains(&pid) {
                    q.wwaiters.push(pid);
                }
            }
        }
        if landed {
            if !readers.is_empty() {
                let hh = h.unwrap_or_else(|| hooks().expect("devpipe: hooks"));
                (hh.wake)(&readers);
            }
            continue;
        }
        // Park until a reader drains (prepare_block + register-under-lock
        // + drop + block — the 2-CPU wakeup-loss-free discipline, SMP-03).
        (h.as_ref().unwrap().block)();
    }
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
            // qhangup(q[1-side]): the peer's readers drain then EOF;
            // its parked WRITERS (full-queue parks) must also wake —
            // the draining EOF frees room and the close paths below
            // settle their fate (19-08's blocking write).
            let peer = &mut g.q[1 - side];
            peer.hungup = true;
            wake.append(&mut peer.waiters);
            wake.append(&mut peer.wwaiters);
            // qclose(q[side]): writers into it error from now on —
            // wake any parked ones so they take the error.
            let mine = &mut g.q[side];
            mine.closed = true;
            wake.append(&mut mine.wwaiters);
        }
        if g.qref == [0, 0] {
            // qreopen: both ends closed — the queues reset.
            for q in g.q.iter_mut() {
                q.buf.clear();
                q.hungup = false;
                q.closed = false;
                q.waiters.clear();
                q.wwaiters.clear();
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
/// glenda).  Returns the byte count (the convD2M contract).
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
