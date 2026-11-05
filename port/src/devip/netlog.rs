//! devip/netlog.rs — `/net/log` + the netlog flag word + the flag-guarded
//! debug-print plumbing (D-07; contract §2's `log` chapter; the donor's
//! `sys/src/9k/ip/netlog.c` — read-only reference, never copied).
//!
//! THE SHAPE: the donor's Netlog is a 16 KiB ring whose read BLOCKS until
//! netlog output exists (bad-packet and error messages; the content is
//! implementation-defined per the man).  Ours is a bounded line queue
//! (64 lines, drop-oldest + counted) fed by `emit()`; the read pops one
//! line at a time (Qmsg-ish), blocking on empty via the seam's
//! register → drop → block shape.  The flag word is an AtomicU32 (the
//! devcons static-flag convention — print guards read it lock-free
//! mid-drive; the queue/waiters sit behind the mcslock).
//!
//! THE FLAG VOCABULARY (netlog.c's flags[] over ip.h:485-500's Log* bits
//! — the DONOR's bit values, not an ad-hoc set): `tcpwin`/`tcprxmt`/
//! `udpmsg`/`ipmsg` OR their message flag into the protocol bit.  The
//! ctl verbs are `set <flags…>` / `clear <flags…>` / `only <addr>`;
//! too few arguments → Ebadnetctl ("too few arguments for netlog control
//! message").  `only` restricts PER-PACKET debug output to one address
//! (the ether leg's frame logs); the note-worthy-event lines (the OQ-4
//! accepted-and-recorded verbs, absorbed 0.0.0.0 adds, parse rejects)
//! carry no packet address and emit whenever their flag is set.
//!
//! 08-05 Task 1 lands the module + the OQ-4 note sites; Task 2 adds the
//! `netlog: flags set <mask>` ctl-acceptance serial marker (D-13), the
//! remaining note feeds, and the host pins.

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU32, Ordering};

use crate::mcslock::{Lock, LockNode};
use crate::Result;

use super::conv::WaitPids;
use super::{parse_ip, stack};

// ---------------------------------------------------------------------------
// The flag vocabulary (ip.h:485-500 — the donor's bit VALUES verbatim).
// ---------------------------------------------------------------------------

pub(crate) const LOG_IP: u32 = 1 << 1;
pub(crate) const LOG_TCP: u32 = 1 << 2;
#[allow(dead_code)] // the fs flag is vocabulary-complete; nothing logs under it this era
pub(crate) const LOG_FS: u32 = 1 << 3;
pub(crate) const LOG_ICMP: u32 = 1 << 5;
pub(crate) const LOG_UDP: u32 = 1 << 6;
#[allow(dead_code)] // compress/gre/esp/ppp: the donor's vocabulary, unwired protos this era
pub(crate) const LOG_COMPRESS: u32 = 1 << 7;
#[allow(dead_code)]
pub(crate) const LOG_GRE: u32 = 1 << 9;
#[allow(dead_code)]
pub(crate) const LOG_PPP: u32 = 1 << 10;
#[allow(dead_code)] // tcpwin/tcprxmt are packet-window/rxmt traces — the smoltcp era never emits them
pub(crate) const LOG_TCP_RXMT: u32 = 1 << 11;
#[allow(dead_code)]
pub(crate) const LOG_UDP_MSG: u32 = 1 << 13;
#[allow(dead_code)]
pub(crate) const LOG_IP_MSG: u32 = 1 << 14;
#[allow(dead_code)]
pub(crate) const LOG_ESP: u32 = 1 << 17;
#[allow(dead_code)]
pub(crate) const LOG_TCP_WIN: u32 = 1 << 18;

/// The flag name → bits map (netlog.c's flags[]: the message flags OR
/// into their protocol bit).
fn flag_bits(name: &[u8]) -> u32 {
    match name {
        b"ppp" => LOG_PPP,
        b"ip" => LOG_IP,
        b"fs" => LOG_FS,
        b"tcp" => LOG_TCP,
        b"icmp" => LOG_ICMP,
        b"udp" => LOG_UDP,
        b"compress" => LOG_COMPRESS,
        b"gre" => LOG_GRE,
        b"tcpwin" => LOG_TCP | LOG_TCP_WIN,
        b"tcprxmt" => LOG_TCP | LOG_TCP_RXMT,
        b"udpmsg" => LOG_UDP | LOG_UDP_MSG,
        b"ipmsg" => LOG_IP | LOG_IP_MSG,
        b"esp" => LOG_ESP,
        _ => 0,
    }
}

/// THE flag word.  AtomicU32 so flag-guarded print sites read lock-free
/// (the devcons static-flag convention — a mid-drive print never takes
/// the netlog lock to learn whether it's on).
static FLAGS: AtomicU32 = AtomicU32::new(0);

/// Is the flag set (the print-guard read)?
pub(crate) fn flag_on(flag: u32) -> bool {
    FLAGS.load(Ordering::Relaxed) & flag != 0
}

/// The current mask (the Task-2 ctl-acceptance marker prints it).
#[allow(dead_code)] // Task 2's marker read
pub(crate) fn flags() -> u32 {
    FLAGS.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// The queue + blocked readers.
// ---------------------------------------------------------------------------

/// The queue bound (the donor's Nlog = 16 KiB char ring; ours is 64 LINES,
/// drop-oldest — the dropped-lines count is the honesty record).
const LOG_Q_CAP: usize = 64;

struct Netlog {
    /// The `only <addr>` restriction (zero address clears it).  Applies
    /// to per-packet logs (08-06's ether era); event notes bypass it.
    only: Option<[u8; 16]>,
    /// The line queue (one formatted line per entry).
    q: VecDeque<Vec<u8>>,
    /// Lines dropped on a full queue (the donor's ring-overwrite arm —
    /// counted, never silent).
    dropped: u64,
    /// Blocked readers (the conv WaitPids shape).
    waiters: WaitPids,
}

static NETLOG: Lock<Netlog> = Lock::new(
    "devipnetlog",
    Netlog { only: None, q: VecDeque::new(), dropped: 0, waiters: WaitPids::new() },
);

/// Emit a netlog line under a flag (gated).  The wake rides the seam's
/// hook AFTER the lock drops (the wake-after-drop discipline).
pub(crate) fn emit(flag: u32, line: Vec<u8>) {
    if !flag_on(flag) {
        return;
    }
    let mut pids = [0u64; 16];
    let n = {
        let node = LockNode::new();
        let mut g = NETLOG.lock(&node);
        if g.q.len() >= LOG_Q_CAP {
            g.q.pop_front();
            g.dropped += 1;
        }
        g.q.push_back(line);
        g.waiters.take(&mut pids)
    };
    if n > 0
        && let Ok(h) = stack::hooks()
    {
        (h.wake)(&pids[..n]);
    }
}

/// The /net/log write (netlogctl): `set <flags…>` / `clear <flags…>` /
/// `only <addr>`; too few arguments → Ebadnetctl.  Unknown flag names are
/// silently ignored (the donor's cmdtab-match shape: an unmatched flag
/// contributes nothing).
pub(crate) fn ctl(buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    let text = &buf[..n.min(63)];
    let fields: Vec<&[u8]> = text
        .split(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
        .filter(|w| !w.is_empty())
        .collect();
    if fields.len() < 2 {
        return Err("too few arguments for netlog control message");
    }
    match fields[0] {
        b"set" => {
            let mut mask = FLAGS.load(Ordering::Relaxed);
            for f in &fields[1..] {
                mask |= flag_bits(f);
            }
            FLAGS.store(mask, Ordering::Relaxed);
            Ok(n as u64)
        }
        b"clear" => {
            let mut mask = FLAGS.load(Ordering::Relaxed);
            for f in &fields[1..] {
                mask &= !flag_bits(f);
            }
            FLAGS.store(mask, Ordering::Relaxed);
            Ok(n as u64)
        }
        b"only" => {
            let addr = parse_ip(fields[1]);
            let node = LockNode::new();
            NETLOG.lock(&node).only = addr;
            Ok(n as u64)
        }
        _ => Err("too few arguments for netlog control message"),
    }
}

/// The /net/log read (netlogread's contract): pop one line; BLOCK when
/// the queue is empty (register → drop → block, the seam's wakeup-loss-
/// free shape; the emit path wakes).
pub(crate) fn read(buf: &mut [u8]) -> Result<u64> {
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        {
            let node = LockNode::new();
            let mut g = NETLOG.lock(&node);
            if let Some(line) = g.q.pop_front() {
                let n = line.len().min(buf.len());
                buf[..n].copy_from_slice(&line[..n]);
                return Ok(n as u64);
            }
            if hooks.is_none() {
                hooks = Some(stack::hooks()?);
            }
            let pid = (hooks.unwrap().current)();
            g.waiters.register(pid);
        }
        (hooks.unwrap().block)();
    }
}

/// The drop counter (observability).
#[allow(dead_code)] // read by the Task-2 pins' assert surface
pub(crate) fn dropped() -> u64 {
    let node = LockNode::new();
    NETLOG.lock(&node).dropped
}

/// The queue depth (pin assertions).
#[allow(dead_code)] // the Task-2 netlog pin
pub(crate) fn queued() -> usize {
    let node = LockNode::new();
    NETLOG.lock(&node).q.len()
}

/// Drain every queued line (the host pins' readback — the production read
/// path is the blocking one above).
#[cfg(test)]
pub(crate) fn drain_lines() -> Vec<Vec<u8>> {
    let node = LockNode::new();
    let mut g = NETLOG.lock(&node);
    g.q.drain(..).collect()
}
