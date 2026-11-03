//! devproc-lite — the per-process note channel (04-08, D-16's hard scope,
//! 04-RESEARCH.md Pitfall 12).  It serves per-process note files and
//! nothing else: `/proc/<pid>/note` is the whole tree, so libc's
//! `postnote(PNPROC, pid, msg)` works unmodified (there is no postnote
//! syscall — the note arrives by a device write).  Walk accepts a numeric
//! pid component under the root and then the literal name "note"; every
//! other name is a walk error.  Open is write-only on a note chan; every
//! other open errors loudly.  Write appends the message bytes (bounded to
//! ERRMAX with a NUL-terminated copy) to the addressed process's note
//! queue — the write resolves the addressed pid and no other target
//! (T-04-17: no broadcast, no group semantics).
//!
//! Like devroot, the dat.rs `Dev` trait is NOT implemented here (its
//! skeleton is non-constructible as written — see devroot.rs's header);
//! the device operations the plan names (attach/walk/open/write) are free
//! functions, and trait conformance lands with the devtab dispatch era.
//!
//! The target-process bookkeeping (the proc table) lives in the arch crate,
//! so the note-post itself is an injected hook — the same dependency
//! inversion as devcons's `Console::set_uart`: the arch side registers a
//! `fn(pid, msg) -> Result<()>` once at boot, and `write` calls it.

use crate::dat::{Chan, Qid, DEV_PROC, OWRITE};
use crate::mcslock::{Lock, LockNode};
use crate::Result;

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// The arch-side note-post hook signature: validate the pid, append to
/// that process's note queue.
type PostHook = fn(u64, &[u8]) -> Result<()>;

/// The arch-side note-post hook.  Registered once (single writer, pre-EL0;
/// read from syscall context thereafter) — the mcslock idiom guards the
/// one-time publish, mirroring devroot's ROOT_IMG.
static POST_HOOK: Lock<Option<PostHook>> = Lock::new("devproc", None);

/// Register the arch-side note-post hook.  Called once during boot before
/// any user process exists; a second registration is a boot bug.
pub fn set_post_hook(hook: PostHook) {
    let node = LockNode::new();
    let mut guard = POST_HOOK.lock(&node);
    assert!(guard.is_none(), "devproc: post hook registered twice");
    *guard = Some(hook);
}

/// The Qid path encoding: the root is 0; a pid's per-process directory is
/// `pid*2`; that process's note file is `pid*2+1` (pids start at 1, so the
/// three kinds never collide).
fn dir_qid(pid: u64) -> Qid {
    Qid { path: pid * 2, vers: 0, typ: QTDIR }
}
fn note_qid(pid: u64) -> Qid {
    Qid { path: pid * 2 + 1, vers: 0, typ: QTFILE }
}

/// attach: the `/proc` root chan (r9 serves one proc tree this era, so the
/// attach spec is empty — the same shape as devroot's attach_root).
pub fn attach() -> Chan {
    Chan::new(DEV_PROC, Qid { path: 0, vers: 0, typ: QTDIR }, 0)
}

/// Walk a single name component from `c`.  Under the root the component
/// must be all ASCII digits (a pid); under a pid directory the component
/// must be the literal name "note".  Anything else is a loud walk error —
/// the tree has no other names.  Pid EXISTENCE is checked at write time
/// (the walk is the syntactic half; the write is the semantic half).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR {
        return Err("devproc: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devproc: walk takes a single name component");
    }
    if c.qid.path == 0 {
        // Root → per-process directory: the component must parse as a pid.
        if !name.iter().all(|b| b.is_ascii_digit()) {
            return Err("devproc: not a numeric process name");
        }
        let mut pid: u64 = 0;
        for &b in name {
            pid = pid.checked_mul(10).and_then(|p| p.checked_add(u64::from(b - b'0')))
                .ok_or("devproc: process name overflows")?;
        }
        if pid == 0 {
            return Err("devproc: not a numeric process name");
        }
        let mut nc = Chan::new(DEV_PROC, dir_qid(pid), 0);
        nc.path = c.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(name);
        Ok(nc)
    } else {
        // Pid directory → the note file, and nothing else.
        if name != b"note" {
            return Err("devproc: unknown per-process name");
        }
        let pid = c.qid.path / 2;
        let mut nc = Chan::new(DEV_PROC, note_qid(pid), 0);
        nc.path = c.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(name);
        Ok(nc)
    }
}

/// Open `c`: a note chan opens write-only (OWRITE access); the directories
/// and every other mode error loudly — the scope guard is the open path.
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    if c.qid.typ == QTDIR {
        return Err("devproc: only per-process note files open");
    }
    if mode != OWRITE {
        return Err("devproc: note files are write-only");
    }
    let mut nc = Chan::new(c.dev, c.qid, mode);
    nc.path = c.path.clone();
    Ok(nc)
}

/// stat of a devproc chan (05-04 Task 3 — the namec stat dispatch's
/// devproc leg; the wire shape is the shared StatWriter).  The gate never
/// stats #p; this is the dispatch's total coverage, not a new surface.
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let (name, mode): (&[u8], u32) = if c.qid.path == 0 {
        (b"proc", 0x8000_0000 | 0o555) // DMDIR
    } else if c.qid.typ == QTDIR {
        // The per-pid directory's name is the pid (decimal).
        let pid = c.qid.path / 2;
        let mut nbuf = alloc::vec::Vec::new();
        let mut v = pid;
        if v == 0 {
            nbuf.push(b'0');
        }
        while v > 0 {
            nbuf.insert(0, b'0' + (v % 10) as u8);
            v /= 10;
        }
        return write_stat_record(sb, &nbuf, c.qid, 0x8000_0000 | 0o555);
    } else {
        (b"note", 0o666)
    };
    write_stat_record(sb, name, c.qid, mode)
}

/// One stat(5) wire record (devcons's shape; uid/gid glenda per D-02).
fn write_stat_record(sb: &mut [u8], name: &[u8], qid: crate::dat::Qid, mode: u32) -> Result<u64> {
    let need = 41 + (2 + name.len()) + (2 + 6) + (2 + 6);
    if sb.len() < need {
        return Err("devproc: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(crate::dat::DEV_PROC as u32);
    w.u8(qid.typ);
    w.u32(qid.vers);
    w.u64(qid.path);
    w.u32(mode);
    w.u32(0); // atime
    w.u32(0); // mtime
    w.u64(0); // length
    w.string(name);
    w.string(b"glenda");
    w.string(b"glenda");
    Ok(w.finish())
}

/// Write `buf` to the note chan `c` names: decode the pid from the chan's
/// qid, bound the message to ERRMAX (the note buffer's size, NUL-padded),
/// and hand it to the arch-side post hook.  Returns the byte count taken.
pub fn write(c: &mut Chan, buf: &[u8]) -> Result<u64> {
    if c.qid.typ == QTDIR || c.qid.path.is_multiple_of(2) {
        return Err("devproc: write of a non-note file");
    }
    if c.mode != OWRITE {
        return Err("devproc: note file not open for writing");
    }
    let pid = (c.qid.path - 1) / 2;
    let hook = {
        let node = LockNode::new();
        *POST_HOOK.lock(&node)
    };
    let Some(hook) = hook else {
        return Err("devproc: note post hook not registered");
    };
    hook(pid, buf)?;
    Ok(buf.len() as u64)
}
