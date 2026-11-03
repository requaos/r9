//! devdup — the per-fd files device `#d` (05-06; semantics ported from
//! 9k's devdup.c — the RESEARCH Don't-Hand-Roll mandate).  rcmain's
//! interactive path `. -i '#d/0'` depends on it (05-RESEARCH Pitfall 9):
//! rc opens the device path `#d/0` — the calling process's fd 0 as a
//! FILE — and sources it interactively.
//!
//! The tree: the `#d` root dir plus one file per OPEN fd of the CALLING
//! process (`#d/0`, `#d/1`, … — the donor's dupgen).  The ctl-file half
//! (`#d/Nctl`) is not implemented this era (no measured caller).
//!
//! The load-bearing semantic (donor dupopen): opening `#d/N` resolves the
//! OPERATING process's fd N AT OPEN TIME and returns THAT chan — reads
//! and writes then flow to the real file (a console, a pipe end, …).  The
//! resolution is dependency-injected (the devproc.rs set_post_hook idiom
//! — port cannot see the arch's proc table): the arch registers a
//! resolver closure (`the calling proc's fd N, ref-cloned`) and a listing
//! closure (`its open fds as (fd, mode) pairs`) once at boot.
//!
//! Mode discipline (donor fdtochan): the requested access must equal the
//! referenced chan's access mode, or the chan is ORDWR — opening `#d/0`
//! OWRITE against a read-only fd 0 errors loudly.
//!
//! The dat.rs `Dev` trait is NOT implemented here (its skeleton is
//! non-constructible — see devroot.rs's header); attach/walk/open/read/
//! write/stat are free functions, dispatched by chan.dev via namec.rs's
//! legs (registry rune 'd').
//!
//! Qid scheme (donor devdup.c: "(2*fd + (file is ctl))+1" — the ctl arm
//! reserved): the root is 0 (QTDIR); `#d/N` is `2*N+1` (QTFILE).

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_DUP, OREAD, ORDWR};
use crate::mcslock::{Lock, LockNode};
use crate::Result;

/// The device-registry rune (namec.rs's '#d' arm — how `#d/0` resolves).
pub const REGISTRY_RUNE: u8 = b'd';

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// The arch-side resolver shape: the CALLING proc's fd N as a reference
/// clone (None for closed/out-of-range), and its open fds as (fd, mode)
/// pairs for the directory listing.
pub type ResolveHook = fn(u64) -> Option<Chan>;
pub type ListHook = fn() -> Vec<(u64, u32)>;

/// The injected hooks (registered once, pre-EL0 — the set-once idiom).
#[derive(Clone, Copy)]
pub struct DupHooks {
    pub resolve: ResolveHook,
    pub list: ListHook,
}

static HOOKS: Lock<Option<DupHooks>> = Lock::new("devdup", None);

/// Register the arch-side hooks.  Called once during boot before any
/// user process exists; a second registration is a boot bug.
pub fn set_hooks(hooks: DupHooks) {
    let node = LockNode::new();
    let mut guard = HOOKS.lock(&node);
    assert!(guard.is_none(), "devdup: hooks registered twice");
    *guard = Some(hooks);
}

fn hooks() -> Result<DupHooks> {
    let node = LockNode::new();
    HOOKS.lock(&node).ok_or("devdup: hooks not registered")
}

/// attach: the `#d` root dir chan (the attach spec is empty this era —
/// devcons/devproc's shape).
pub fn attach() -> Chan {
    let mut c = Chan::new(DEV_DUP, Qid { path: 0, vers: 0, typ: QTDIR }, 0);
    c.path = b"#d".to_vec();
    c
}

/// Parse a numeric fd component (devdup's whole grammar: ASCII digits).
fn parse_fd(name: &[u8]) -> Result<u64> {
    if name.is_empty() || name.contains(&b'/') {
        return Err("devdup: walk takes a single name component");
    }
    if !name.iter().all(|b| b.is_ascii_digit()) {
        return Err("devdup: not an fd number");
    }
    let mut fd: u64 = 0;
    for &b in name {
        fd = fd
            .checked_mul(10)
            .and_then(|f| f.checked_add(u64::from(b - b'0')))
            .ok_or("devdup: fd number overflows")?;
    }
    Ok(fd)
}

/// Walk a single name component from the `#d` root: a decimal fd number.
/// The fd's EXISTENCE is checked at open/stat time (the walk is the
/// syntactic half — donor devwalk/dupgen's shape).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR || c.qid.path != 0 {
        return Err("devdup: walk of a non-directory");
    }
    let fd = parse_fd(name)?;
    let mut nc = Chan::new(DEV_DUP, Qid { path: 2 * fd + 1, vers: 0, typ: QTFILE }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open a devdup chan.  The root dir opens OREAD only.  An fd file
/// RESOLVES the calling proc's fd N through the injected hook and returns
/// THAT chan (a reference clone — a pipe end's open count bumps, so the
/// `#d/N` fd is a real second reference the close dispatch balances).
/// The donor fdtochan mode check: the requested access must match the
/// chan's access mode (or the chan is ORDWR).
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let access = mode & crate::dat::OMASK;
    if c.qid.typ == QTDIR {
        if access != OREAD {
            return Err("devdup: directory opens OREAD only");
        }
        let mut nc = Chan::new(c.dev, c.qid, access);
        nc.path = c.path.clone();
        return Ok(nc);
    }
    let fd = (c.qid.path - 1) / 2;
    let Some(target) = (hooks()?.resolve)(fd) else {
        return Err("devdup: no such fd");
    };
    let target_access = target.mode & crate::dat::OMASK;
    if target_access != access && target_access != ORDWR {
        return Err("devdup: mode mismatch with the referenced fd");
    }
    Ok(target)
}

/// Read a devdup chan.  The root dir serves the CALLING proc's open-fd
/// listing (stat-wire records named by fd number, perms from each chan's
/// mode — donor dupgen's 0400/0200/0600); the streaming-offset
/// convention applies.  An fd FILE chan never reads here (open returned
/// the referenced chan) — a defensive loud error.
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    if c.qid.typ != QTDIR {
        return Err("devdup: fd files are read through their referenced chan");
    }
    let off = if offset == u64::MAX { c.offset } else { offset };
    let h = hooks()?;
    let mut listing = Vec::new();
    for (fd, mode) in (h.list)() {
        let perm = match mode & crate::dat::OMASK {
            OREAD => 0o400,
            crate::dat::OWRITE => 0o200,
            _ => 0o600,
        };
        let mut name = Vec::new();
        let mut v = fd;
        if v == 0 {
            name.push(b'0');
        }
        while v > 0 {
            name.insert(0, b'0' + (v % 10) as u8);
            v /= 10;
        }
        let start = listing.len();
        listing.resize(start + 41 + (2 + name.len()) + 2 * (2 + 6), 0);
        write_stat_record(&mut listing[start..], &name, 2 * fd + 1, QTFILE, perm, 0)?;
    }
    let start = (off as usize).min(listing.len());
    let avail = &listing[start..];
    let n = avail.len().min(buf.len());
    buf[..n].copy_from_slice(&avail[..n]);
    if offset == u64::MAX {
        c.offset += n as u64;
    }
    Ok(n as u64)
}

/// Write: the donor's dupwrite is always Eperm (fd I/O flows through the
/// referenced chan that open returned — never the `#d/N` file itself).
pub fn write(_c: &mut Chan, _buf: &[u8], _offset: u64) -> Result<u64> {
    Err("devdup: not writable (open #d/N to reach the fd)")
}

/// stat of a devdup chan (sys_stat/sys_fstat's dispatch leg): the root
/// serves "#d" DMDIR|0555; an fd file serves its number name and the
/// referenced chan's mode-derived perms (resolving through the hook —
/// donor dupstat's dupgen).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return write_stat_record(sb, b"#d", 0, QTDIR, 0x8000_0000 | 0o555, 0);
    }
    let fd = (c.qid.path - 1) / 2;
    let Some(target) = (hooks()?.resolve)(fd) else {
        return Err("devdup: no such fd");
    };
    let perm = match target.mode & crate::dat::OMASK {
        OREAD => 0o400,
        crate::dat::OWRITE => 0o200,
        _ => 0o600,
    };
    let mut name = Vec::new();
    let mut v = fd;
    if v == 0 {
        name.push(b'0');
    }
    while v > 0 {
        name.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    write_stat_record(sb, &name, c.qid.path, QTFILE, perm, 0)
}

/// One stat(5) wire record (the devcons/devproc/devenv shape; uid/gid
/// glenda per D-02).  Returns the byte count (the convD2M contract).
fn write_stat_record(
    sb: &mut [u8],
    name: &[u8],
    qid_path: u64,
    typ: u8,
    mode: u32,
    length: u64,
) -> Result<u64> {
    let need = 41 + (2 + name.len()) + (2 + 6) + (2 + 6);
    if sb.len() < need {
        return Err("devdup: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(DEV_DUP as u32);
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
    Ok(w.finish())
}
