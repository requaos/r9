//! devsrv — the `#s` service registry (semantics ported from 9k's
//! devsrv.c).  boot's
//! `srvcreate("boot", fd)` (sys/src/9k/boot/aux.c — FATAL on failure)
//! creates `#s/boot` and writes the root pipe's fd number into it;
//! settime's later `open("#s/boot")` (via devmnt) resolves the
//! SAME fd.
//!
//! The registry maps a NAME to a posted chan: create makes the entry
//! (Eexist on a duplicate, OWRITE only — the donor's srvcreate), a write
//! of a decimal fd number binds the entry to a REFERENCE CLONE of the
//! calling proc's fd's chan (the Arc-shared pipe state keeps
//! the queue alive after the registrant closes/exits — the srv-held
//! clone is exactly the donor's fdtochan incref), and the dir listing
//! shows the registered names.  Opening a posted file returns the posted
//! chan's clone (the donor's srvopen: `cclose(c); incref(sp->chan)`),
//! so the opener reads/writes the real object (the pipe end).  The
//! donor's ownership/removal/wstat surface and its CCEXEC/CRCLOSE/QTAUTH
//! posted-fd guards are later-era (no measured caller; r9 carries no
//! such flags).
//!
//! The fd resolution is dependency-injected (the devproc.rs
//! set_post_hook idiom — port cannot see the arch's proc table): the
//! arch registers a resolver closure once at boot.
//!
//! The dat.rs `Dev` trait is NOT implemented here (its skeleton is
//! non-constructible — see devroot.rs's header); attach/walk/open/
//! create/read/write/stat are free functions, dispatched by chan.dev via
//! namec.rs's legs (registry rune 's').
//!
//! Qid scheme: the root is 0 (QTDIR); a service's qid path is its
//! registration sequence from 1 (QTFILE).

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_SRV, OREAD, ORDWR, OTRUNC, OWRITE};
use crate::mcslock::{Lock, LockNode};
use crate::Result;

/// The device-registry rune (namec.rs's '#s' arm — how `#s/boot`
/// resolves).
pub const REGISTRY_RUNE: u8 = b's';

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// One registered service: its name, qid path, perm bits (donor
/// sp->perm), and the posted chan (None until the fd write lands).
pub struct SrvEntry {
    pub name: Vec<u8>,
    pub qid_path: u64,
    pub perm: u32,
    pub chan: Option<Chan>,
}

/// The registry + the qid-path sequence (9k's srv/qidpath statics).
static SRV: Lock<(Vec<SrvEntry>, u64)> = Lock::new("devsrv", (Vec::new(), 1));

/// The arch-side resolver hook: the CALLING proc's fd N as a reference
/// clone (None for closed/out-of-range).
pub type ResolveHook = fn(u64) -> Option<Chan>;

static RESOLVE_HOOK: Lock<Option<ResolveHook>> = Lock::new("devsrvhook", None);

/// Register the arch-side fd resolver.  Called once during boot before
/// any user process exists; a second registration is a boot bug.
pub fn set_resolve_hook(hook: ResolveHook) {
    let node = LockNode::new();
    let mut guard = RESOLVE_HOOK.lock(&node);
    assert!(guard.is_none(), "devsrv: resolve hook registered twice");
    *guard = Some(hook);
}

fn resolve_fd(fd: u64) -> Result<Chan> {
    let hook = {
        let node = LockNode::new();
        *RESOLVE_HOOK.lock(&node)
    };
    let Some(hook) = hook else {
        return Err("devsrv: fd resolve hook not registered");
    };
    hook(fd).ok_or("devsrv: bad fd")
}

/// attach: the `#s` root dir chan (empty spec this era — the devcons
/// shape).
pub fn attach() -> Chan {
    let mut c = Chan::new(DEV_SRV, Qid { path: 0, vers: 0, typ: QTDIR }, 0);
    c.path = b"#s".to_vec();
    c
}

/// Walk a single name component from the `#s` root to a registered
/// service (donor srvwalk/srvlookup-by-name): unknown names are walk
/// errors (never silent shadows).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR || c.qid.path != 0 {
        return Err("devsrv: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devsrv: walk takes a single name component");
    }
    let node = LockNode::new();
    let guard = SRV.lock(&node);
    let Some(e) = guard.0.iter().find(|e| e.name == name) else {
        return Err("devsrv: name not found");
    };
    let mut nc = Chan::new(DEV_SRV, Qid { path: e.qid_path, vers: 0, typ: QTFILE }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Create a service entry under the `#s` root (donor srvcreate): OWRITE
/// only; Eexist on a duplicate name; the entry starts unposted (the fd
/// write binds it).  The returned chan is OPEN OWRITE (namec's
/// create_chan funnel marks it copen).
pub fn create(c: &Chan, name: &[u8], mode: u32, perms: u32) -> Result<Chan> {
    if c.qid.typ != QTDIR || c.qid.path != 0 {
        return Err("devsrv: create under a non-directory");
    }
    if mode & crate::dat::OMASK != OWRITE {
        return Err("devsrv: services create OWRITE only");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devsrv: bad service name");
    }
    let node = LockNode::new();
    let mut guard = SRV.lock(&node);
    if guard.0.iter().any(|e| e.name == name) {
        return Err("devsrv: name exists");
    }
    let qid_path = guard.1;
    guard.1 += 1;
    guard.0.push(SrvEntry {
        name: name.to_vec(),
        qid_path,
        perm: perms & 0o777,
        chan: None,
    });
    let mut nc = Chan::new(DEV_SRV, Qid { path: qid_path, vers: 0, typ: QTFILE }, OWRITE);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open a devsrv chan.  The root dir opens OREAD only.  A service file:
/// the entry must exist and have a POSTED chan (else the donor's
/// Eshutdown); OTRUNC is the donor's "srv file already exists"; the
/// requested access must match the posted chan's mode (or it is ORDWR).
/// The open returns the POSTED CHAN's reference clone — the opener
/// reads/writes the real object (the pipe end), exactly the donor's
/// srvopen.  (settime's path rides this: open("#s/boot") → the
/// boot pipe's chan.)
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let access = mode & crate::dat::OMASK;
    if c.qid.typ == QTDIR {
        if access != OREAD {
            return Err("devsrv: directory opens OREAD only");
        }
        let mut nc = Chan::new(c.dev, c.qid, access);
        nc.path = c.path.clone();
        return Ok(nc);
    }
    if mode & OTRUNC != 0 {
        return Err("devsrv: srv file already exists");
    }
    let posted = {
        let node = LockNode::new();
        let guard = SRV.lock(&node);
        let Some(e) = guard.0.iter().find(|e| e.qid_path == c.qid.path) else {
            return Err("devsrv: nonexistent service");
        };
        let Some(chan) = &e.chan else {
            return Err("devsrv: service has no posted fd");
        };
        chan.dup()
    };
    let posted_access = posted.mode & crate::dat::OMASK;
    if posted_access != access && posted_access != ORDWR {
        return Err("devsrv: mode mismatch with the posted fd");
    }
    Ok(posted)
}

/// Write an fd number to a service file (donor srvwrite): a decimal fd
/// under 32 bytes; the fd resolves through the injected hook (the CALLING
/// proc's table) and is REFERENCE-CLONED into the entry (the
/// clone's Arc keeps a pipe's queue alive after the registrant exits).
/// An already-posted service errors (donor Ebadusefd).
pub fn write(c: &mut Chan, buf: &[u8], _offset: u64) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return Err("devsrv: write of a directory");
    }
    if c.mode & crate::dat::OMASK != OWRITE {
        return Err("devsrv: service not open for writing");
    }
    if buf.len() >= 32 {
        return Err("devsrv: fd string too long");
    }
    if buf.is_empty() || !buf.iter().all(|b| b.is_ascii_digit()) {
        return Err("devsrv: not an fd number");
    }
    let mut fd: u64 = 0;
    for &b in buf {
        fd = fd * 10 + u64::from(b - b'0');
    }
    let chan = resolve_fd(fd)?;
    let node = LockNode::new();
    let mut guard = SRV.lock(&node);
    let Some(e) = guard.0.iter_mut().find(|e| e.qid_path == c.qid.path) else {
        return Err("devsrv: nonexistent service");
    };
    if e.chan.is_some() {
        return Err("devsrv: service already posted");
    }
    e.chan = Some(chan);
    Ok(buf.len() as u64)
}

/// Read a devsrv chan.  The root dir serves the stat-wire listing of the
/// registered services (donor devdirread over srvgen — the names and
/// their perms); a service FILE chan never reads here (open returned the
/// posted chan) — a defensive loud error.
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    if c.qid.typ != QTDIR {
        return Err("devsrv: service files are read through their posted chan");
    }
    let off = if offset == u64::MAX { c.offset } else { offset };
    let mut listing = Vec::new();
    {
        let node = LockNode::new();
        let guard = SRV.lock(&node);
        for e in &guard.0 {
            let start = listing.len();
            listing.resize(start + 41 + (2 + e.name.len()) + 3 * (2 + 6), 0);
            write_stat_record(&mut listing[start..], &e.name, e.qid_path, QTFILE, e.perm, 0)?;
        }
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

/// stat of a devsrv chan (sys_stat/sys_fstat's dispatch leg): the root
/// serves "#s" DMDIR|0555; a service serves its name, perm, and qid
/// (donor srvgen — length 0).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return write_stat_record(sb, b"#s", 0, QTDIR, 0x8000_0000 | 0o555, 0);
    }
    let node = LockNode::new();
    let guard = SRV.lock(&node);
    let Some(e) = guard.0.iter().find(|e| e.qid_path == c.qid.path) else {
        return Err("devsrv: nonexistent service");
    };
    write_stat_record(sb, &e.name, e.qid_path, QTFILE, e.perm, 0)
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
        return Err("devsrv: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(DEV_SRV as u32);
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
