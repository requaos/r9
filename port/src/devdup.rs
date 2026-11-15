//! devdup — the per-fd files device `#d` (semantics ported from
//! 9k's devdup.c).  rcmain's
//! interactive path `. -i '#d/0'` depends on it:
//! rc opens the device path `#d/0` — the calling process's fd 0 as a
//! FILE — and sources it interactively.
//!
//! The tree: the `#d` root dir plus one file per OPEN fd of the CALLING
//! process (`#d/0`, `#d/1`, … — the donor's dupgen) and each fd's ctl
//! file (`#d/Nctl`).  The ctl half (19-10, the desktop leg): the read
//! serves the donor procfdprint fd line —
//! `"%3d %.2s %C %4ud (%.16llux %*lud %.2ux) %5ld %8lld %s\n"`
//! (devproc.c:553) — whose 8th whitespace token is the chan's IOUNIT;
//! libc iounit(2) parses exactly that column (sys/src/libc/9sys/
//! iounit.c).  libdraw's initdisplay sizes its draw-message buffer by
//! iounit(/dev/draw/data): a 0 answer makes it fall back to 8000, over
//! this era's 4072 mount-transport cap, and the resulting mid-message
//! write split kills cpu-session rio at `can't open display` — the
//! ctl file is the fd half of that ABI (devmnt's Ropen iounit is the
//! kernel half).
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
//! Qid scheme (donor devdup.c: "(2*fd + (file is ctl))+1"): the root is
//! 0 (QTDIR); `#d/N` is `2*N+1`, `#d/Nctl` is `2*N+2` (QTFILE).

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

/// Walk a single name component from the `#d` root: a decimal fd number
/// or `Nctl` (the donor's two-file-per-fd tree).  The fd's EXISTENCE is
/// checked at open/stat time (the walk is the syntactic half — donor
/// devwalk/dupgen's shape).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR || c.qid.path != 0 {
        return Err("devdup: walk of a non-directory");
    }
    // the ctl suffix splits the namespace: `N` is the fd file, `Nctl`
    // its control file (donor dupgen's `s & 1` parity)
    let (digits, ctl) = match name.strip_suffix(b"ctl") {
        Some(d) => (d, true),
        None => (name, false),
    };
    let fd = parse_fd(digits)?;
    let qid_path = if ctl { 2 * fd + 2 } else { 2 * fd + 1 };
    let mut nc = Chan::new(DEV_DUP, Qid { path: qid_path, vers: 0, typ: QTFILE }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open a devdup chan.  The root dir opens OREAD only.  An fd file
/// RESOLVES the calling proc's fd N through the injected hook and returns
/// THAT chan (a reference clone — a pipe end's open count bumps, so the
/// `#d/N` fd is a real second reference the close dispatch balances).
/// A CTL file opens ITSELF (the donor dupopen's `f = c` arm — reads
/// format the fd line at read time, so the fd is resolved fresh then).
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
    let twicefd = c.qid.path - 1;
    let fd = twicefd / 2;
    if twicefd & 1 != 0 {
        // the ctl arm: the ctl chan itself opens (donor `f = c`)
        let mut nc = Chan::new(c.dev, c.qid, access);
        nc.path = c.path.clone();
        return Ok(nc);
    }
    let Some(target) = (hooks()?.resolve)(fd) else {
        return Err("devdup: no such fd");
    };
    let target_access = target.mode & crate::dat::OMASK;
    if target_access != access && target_access != ORDWR {
        return Err("devdup: mode mismatch with the referenced fd");
    }
    Ok(target)
}

/// The donor's per-dev dc character (procfdprint's `%C` column — the
/// devtab rune for the chan's device).  Mapped from the DEV_* index;
/// unknown devices print '?' (the token only needs to be single and
/// space-free for libc iounit's tokenizer).
fn dev_dc(dev: usize) -> u8 {
    match dev {
        crate::dat::DEV_ROOT => b'/',
        crate::dat::DEV_CONS => b'c',
        crate::dat::DEV_PROC => b'p',
        crate::dat::DEV_ENV => b'e',
        crate::dat::DEV_PIPE => b'|',
        crate::dat::DEV_DUP => b'd',
        crate::dat::DEV_SRV => b's',
        crate::dat::DEV_MNT => b'M',
        crate::dat::DEV_ETHER => b'l',
        crate::dat::DEV_IP => b'I',
        crate::dat::DEV_CAP => 0xc2, // '¤' (the UTF-8 lead byte — a token, not a rune)
        crate::dat::DEV_SSL => b'D',
        _ => b'?',
    }
}

/// The donor procfdprint fd line (devproc.c:553):
/// `"%3d %.2s %C %4ud (%.16llux %*lud %.2ux) %5ld %8lld %s\n"` — fd,
/// mode, dev dc, devno, (qid path vers type), IOUNIT, offset, path.
/// libc iounit(2) tokenizes and takes column 8 (the iounit).
fn fd_line(fd: u64, c: &Chan, out: &mut Vec<u8>) {
    let mode: &[u8] = match c.mode & crate::dat::OMASK {
        OREAD => b"r ",
        crate::dat::OWRITE => b" w",
        _ => b"rw",
    };
    fn dec(v: u64, width: usize, out: &mut Vec<u8>) {
        let mut num = [0u8; 20];
        let mut i = 0;
        let mut v = v;
        if v == 0 {
            num[i] = b'0';
            i = 1;
        }
        while v > 0 {
            num[i] = b'0' + (v % 10) as u8;
            i += 1;
            v /= 10;
        }
        for _ in i..width {
            out.push(b' ');
        }
        while i > 0 {
            i -= 1;
            out.push(num[i]);
        }
    }
    fn hex(v: u64, width: usize, out: &mut Vec<u8>) {
        let mut num = [0u8; 20];
        let mut i = 0;
        let mut v = v;
        if v == 0 {
            num[i] = b'0';
            i = 1;
        }
        while v > 0 {
            let d = (v % 16) as u8;
            num[i] = if d < 10 { b'0' + d } else { b'a' + d - 10 };
            i += 1;
            v /= 16;
        }
        for _ in i..width {
            out.push(b'0');
        }
        while i > 0 {
            i -= 1;
            out.push(num[i]);
        }
    }
    dec(fd, 3, out);
    out.push(b' ');
    out.extend_from_slice(mode);
    out.push(b' ');
    out.push(dev_dc(c.dev));
    out.push(b' ');
    dec(0, 4, out); // devno — 0 this era (no device instances)
    out.extend_from_slice(b" (");
    hex(c.qid.path, 16, out);
    out.push(b' ');
    dec(c.qid.vers as u64, 1, out);
    out.push(b' ');
    hex(c.qid.typ as u64, 2, out);
    out.extend_from_slice(b") ");
    dec(c.iounit as u64, 5, out);
    out.push(b' ');
    dec(c.offset, 8, out);
    out.push(b' ');
    out.extend_from_slice(&c.path);
    out.push(b'\n');
}

/// Read a devdup chan.  The root dir serves the CALLING proc's open-fd
/// listing (stat-wire records named by fd number AND each fd's ctl file
/// — donor dupgen's two-entries-per-fd shape, perms 0400/0200/0600 for
/// the fd file and 0400 for its ctl); a CTL file serves the donor
/// procfdprint fd line of the referenced fd (fresh resolution at read
/// time); the streaming-offset convention applies.  An fd FILE chan
/// never reads here (open returned the referenced chan) — a defensive
/// loud error.
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    if c.qid.typ != QTDIR {
        let twicefd = c.qid.path - 1;
        if twicefd & 1 == 0 {
            return Err("devdup: fd files are read through their referenced chan");
        }
        let fd = twicefd / 2;
        let target = (hooks()?.resolve)(fd).ok_or("devdup: no such fd")?;
        let mut line = Vec::new();
        fd_line(fd, &target, &mut line);
        let off = if offset == u64::MAX { c.offset } else { offset };
        let start = (off as usize).min(line.len());
        let avail = &line[start..];
        let n = avail.len().min(buf.len());
        buf[..n].copy_from_slice(&avail[..n]);
        if offset == u64::MAX {
            c.offset += n as u64;
        }
        return Ok(n as u64);
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
        // the fd file (donor dupgen's even s)
        let start = listing.len();
        listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
        write_stat_record(&mut listing[start..], &name, 2 * fd + 1, QTFILE, perm, 0)?;
        // its ctl file (donor dupgen's odd s — 0400, name Nctl)
        let mut ctlname = name.clone();
        ctlname.extend_from_slice(b"ctl");
        let start = listing.len();
        listing.resize(start + 41 + (2 + ctlname.len()) + 3 * (2 + 6), 0);
        write_stat_record(&mut listing[start..], &ctlname, 2 * fd + 2, QTFILE, 0o400, 0)?;
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
    let twicefd = c.qid.path - 1;
    let fd = twicefd / 2;
    let mut name = Vec::new();
    let mut v = fd;
    if v == 0 {
        name.push(b'0');
    }
    while v > 0 {
        name.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    if twicefd & 1 != 0 {
        // the ctl file: 0400, name Nctl (donor dupgen's odd arm)
        name.extend_from_slice(b"ctl");
        return write_stat_record(sb, &name, c.qid.path, QTFILE, 0o400, 0);
    }
    let Some(target) = (hooks()?.resolve)(fd) else {
        return Err("devdup: no such fd");
    };
    let perm = match target.mode & crate::dat::OMASK {
        OREAD => 0o400,
        crate::dat::OWRITE => 0o200,
        _ => 0o600,
    };
    write_stat_record(sb, &name, c.qid.path, QTFILE, perm, 0)
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
    w.string(b"glenda"); // muid — the fourth string statcheck walks
    Ok(w.finish())
}
