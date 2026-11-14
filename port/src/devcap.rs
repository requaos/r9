//! devcap — the `#¤` capability device (AUTH-03, 18-02).  A
//! semantics-level Rust port of 9k's `sys/src/9k/port/devcap.c` (287
//! lines, the donor of record; the legacy twin `sys/src/9/port/devcap.c`
//! differs only in capstat's signature).  Donor line cites below name
//! the semantics being ported; this is not a literal translation (the
//! project's D-06: port the donor, never the phrase).
//!
//! The donor's contract: a process that can present
//! `<from>@<to>@<base64-random>` — knowing the random part of a cap the
//! HOSTOWNER posted to `caphash` — may change its user to `to`
//! (devcap.c's header comment: "if a process knows cap->cap, it can
//! change user to capabilty->user").  Phase 19's cpu server rides
//! auth_chuid on exactly these semantics.
//!
//! Structure (the devsrv small-device skeleton): REGISTRY_RUNE +
//! `Lock<CapState>` module state + free-function attach/walk/open/read/
//! write/stat/remove dispatched via namec's legs and the x86_64 syscall
//! read/write arms.  The per-proc user and the eve gate live in devcons
//! (UserHooks + HOSTOWNER) — devcap consumes them through the injected
//! hooks, never an arch proc table.
//!
//! The cap format (the userspace contract, sys/src/cmd/auth/as.c
//! mkcap:121-153 and factotum util.c's twin): cap string =
//! `<from>@<to>@<base64 20 rand bytes>`; the posted hash is
//! HMAC-SHA1(data=`from@to`, key=the base64-random text) — the kernel
//! splits at the LAST '@' for the key and the FIRST '@' of the remainder
//! for from/to (devcap.c capwrite).
//!
//! Qid scheme: the donor's enum — Qdir=0 (root), Qhash=1, Quse=2.  The
//! dirtab serves caphash LAST: eve's remove of caphash permanently
//! shrinks the served table (donor `ncapdir = nelem(capdir)-1`), the
//! anti-scraping rule — once hidden, the hash surface never comes back.
//!
//! Error strings: `invalid capability %s@%s` (formatted, donor
//! capwrite's snprint) and `capability must match user` (donor's
//! literal) carry VERBATIM; the Eperm/Eshort/Ebadarg classes ride the
//! device-prefixed r9 forms.

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_CAP, OREAD, QTDIR, QTFILE};
use crate::mcslock::{Lock, LockNode};
use crate::{hmacsha1, Result};

/// The device-registry rune (namec.rs's '#¤' arm — cfg-gated on the
/// dev_cap config key; '¤' is U+00A4, two UTF-8 bytes, decoded by
/// first_rune today).
pub const REGISTRY_RUNE: char = '¤';

/// The donor's `Hashlen` = SHA1dlen: the width of a posted/consumed
/// hash.
const HASHLEN: usize = hmacsha1::SHA1DLEN;

/// The donor's `Maxhash` = 256: the FIFO trim bound — while the table
/// holds >= 256 entries, addcap drops the OLDEST (head) before the
/// append.  THIS IS THE ONLY CAPACITY RULE: the donor records a ticks
/// value per entry and never checks it (no time-based capability
/// expiry exists upstream, and none is invented here — D-06's
/// prohibition).
const MAXHASH: usize = 256;

/// The donor's qid enum (devcap.c).
const QDIR: u64 = 0;
const QHASH: u64 = 1;
const QUSE: u64 = 2;

/// The dirtab (donor capdir — caphash MUST be last for capremove's
/// shrink): "." DMDIR|0500, capuse 0222, caphash 0200.
const DIRTAB: [(&[u8], u64, u32); 3] = [
    (b".", QDIR, 0x8000_0000 | 0o500),
    (b"capuse", QUSE, 0o222),
    (b"caphash", QHASH, 0o200),
];

/// The attach/stat root name — '#¤' is not a byte string (U+00A4 is
/// two UTF-8 bytes), so it rides a str literal's as_bytes.
const ROOT_NAME: &[u8] = "#¤".as_bytes();

/// One posted capability hash (donor struct Caphash).
struct Caphash {
    hash: [u8; HASHLEN],
    /// Donor `p->ticks = m->ticks` — recorded, NEVER read: the donor
    /// has no time-based expiry (devcap.c addcap writes it, nothing
    /// consults it).  The value comes from the arch tick hook when one
    /// is registered (0 otherwise — no hook, no device on that config).
    #[allow(dead_code)] // the donor's dead field, ported on purpose
    ticks: u64,
}

/// The device state (donor `capalloc`: QLock + the FIFO list + nhash,
/// plus the served-table length `ncapdir`).  FIFO order: OLDEST at the
/// head, appends at the tail.
struct CapState {
    entries: Vec<Caphash>,
    /// The served dirtab length: DIRTAB.len(), or DIRTAB.len()-1
    /// FOREVER after eve's caphash remove (donor ncapdir).
    ncapdir: usize,
}

static CAPALLOC: Lock<CapState> =
    Lock::new("capalloc", CapState { entries: Vec::new(), ncapdir: DIRTAB.len() });

/// The arch-side tick source for the recorded-but-never-read ticks
/// field (the devip set_clock idiom; registered under cfg(dev_cap) in
/// main — the value is dead state either way, kept honest rather than
/// invented).
static TICK_HOOK: Lock<Option<fn() -> u64>> = Lock::new("captick", None);

/// Register the arch-side tick source.  A second registration is a boot
/// bug.
pub fn set_tick_hook(hook: fn() -> u64) {
    let node = LockNode::new();
    let mut guard = TICK_HOOK.lock(&node);
    assert!(guard.is_none(), "devcap: tick hook registered twice");
    *guard = Some(hook);
}

fn ticks_now() -> u64 {
    let node = LockNode::new();
    TICK_HOOK.lock(&node).map_or(0, |f| f())
}

/// Install a clean device state for a pin (the devip
/// install_test_stack discipline: the pins serialize through a test
/// lock and each gets a fresh FIFO).
#[cfg(test)]
pub(crate) fn install_test_state() {
    let node = LockNode::new();
    let mut guard = CAPALLOC.lock(&node);
    guard.entries.clear();
    guard.ncapdir = DIRTAB.len();
}

/// devcap's write error: a static class (Eperm/Eshort shapes) or the
/// donor's FORMATTED `invalid capability <from>@<to>@<key>` — the port
/// Result's `&'static str` cannot carry formatted bytes, so the dynamic
/// variant transports them to the syscall arm, which lands them in the
/// caller's errstr verbatim (the x86_64 sys_pwrite DEV_CAP arm).
#[derive(Debug)]
pub enum CapErr {
    Static(&'static str),
    /// The formatted message bytes ("invalid capability " + from-to +
    /// "@" + key) — donor capwrite's `snprint(err, "invalid capability
    /// %s@%s", from, key)`.
    InvalidCapability(Vec<u8>),
}

/// attach: the `#¤` root dir chan (empty spec this era — the devcons
/// shape; the donor's capattach passes the spec to devattach and
/// ignores it beyond the rune).
pub fn attach() -> Chan {
    let mut c = Chan::new(DEV_CAP, Qid { path: QDIR, vers: 0, typ: QTDIR }, 0);
    c.path = ROOT_NAME.to_vec();
    c
}

/// Walk a single name component from the `#¤` root (donor capwalk over
/// the served dirtab prefix).  "." keeps the root (the donor devwalk's
/// Accept arm — namec's nomount path defers "." to the device).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR || c.qid.path != QDIR {
        return Err("devcap: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devcap: walk takes a single name component");
    }
    if name == b"." {
        let mut nc = c.dup();
        nc.path.push(b'/');
        nc.path.extend_from_slice(name);
        return Ok(nc);
    }
    let ncapdir = {
        let node = LockNode::new();
        CAPALLOC.lock(&node).ncapdir
    };
    let Some((_, qid, _)) = DIRTAB[..ncapdir].iter().find(|(n, _, _)| *n == name) else {
        return Err("devcap: name not found");
    };
    let mut nc = Chan::new(DEV_CAP, Qid { path: *qid, vers: 0, typ: QTFILE }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open a walked devcap chan (donor capopen): the dir opens OREAD only
/// (Ebadarg otherwise); caphash requires the hostowner (Eperm — the
/// donor's iseve gate, the one trust boundary on the posting side);
/// capuse opens for writing without an eve gate (presenting is for
/// everyone; the CAP is the secret).
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    if c.qid.typ == QTDIR {
        if mode != OREAD {
            return Err("devcap: directory opens OREAD only");
        }
    } else if c.qid.path == QHASH && !crate::devcons::iseve()? {
        return Err("devcap: caphash requires the hostowner");
    }
    let mut nc = Chan::new(c.dev, c.qid, mode);
    nc.path = c.path.clone();
    Ok(nc)
}

/// addcap (donor addcap:146-175): append at the TAIL under the device
/// lock, first trimming the OLDEST head entries while the table holds
/// >= Maxhash — the FIFO capacity rule, the only aging the donor knows.
fn addcap(hash: [u8; HASHLEN]) {
    let node = LockNode::new();
    let mut guard = CAPALLOC.lock(&node);
    while guard.entries.len() >= MAXHASH {
        guard.entries.remove(0);
    }
    guard.entries.push(Caphash { hash, ticks: ticks_now() });
}

/// remcap (donor remcap:121-143): find and UNLINK the matching entry —
/// consume-on-match, the one-shot rule.  Returns false when no posted
/// hash matches.
fn remcap(hash: &[u8; HASHLEN]) -> bool {
    let node = LockNode::new();
    let mut guard = CAPALLOC.lock(&node);
    let Some(i) = guard.entries.iter().position(|e| &e.hash == hash) else {
        return false;
    };
    guard.entries.remove(i);
    true
}

/// Write (donor capwrite:196-265).  caphash: eve-gated, >= Hashlen
/// bytes (Eshort), the first Hashlen bytes post via addcap.  capuse:
/// the capability-string parse — key at the LAST '@' (Eshort without
/// one), HMAC-SHA1 over the from-to part keyed by the base64 tail,
/// remcap consumes on match (no match -> the FORMATTED invalid
/// capability error), from/to split at the FIRST '@' of the remainder
/// (a single-user cap has to = from), the from@to form must match the
/// CURRENT user ("capability must match user"), and success sets the
/// current proc's user to `to` — the donor's `kstrdup(&up->user, to)`,
/// the first r9 kernel uid mutation.  The donor's consume-then-match
/// ORDER is kept: a cap whose from does not match burns on the failed
/// present (donor remcap runs before the user check).
pub fn write(c: &mut Chan, buf: &[u8]) -> core::result::Result<u64, CapErr> {
    match c.qid.path {
        QHASH => {
            if !crate::devcons::iseve().map_err(CapErr::Static)? {
                return Err(CapErr::Static("devcap: caphash requires the hostowner"));
            }
            if buf.len() < HASHLEN {
                return Err(CapErr::Static("devcap: i/o count too small"));
            }
            let mut hash = [0u8; HASHLEN];
            hash.copy_from_slice(&buf[..HASHLEN]);
            addcap(hash);
            Ok(buf.len() as u64)
        }
        QUSE => {
            // The donor copies n bytes + NUL then splits; byte slices
            // need no NUL — split on the LAST '@' for the key.
            let Some(at) = buf.iter().rposition(|&b| b == b'@') else {
                return Err(CapErr::Static("devcap: i/o count too small"));
            };
            let from_to = &buf[..at];
            let key = &buf[at + 1..];
            let hash = hmacsha1::hmac_sha1(from_to, key);
            if !remcap(&hash) {
                let mut msg = b"invalid capability ".to_vec();
                msg.extend_from_slice(from_to);
                msg.push(b'@');
                msg.extend_from_slice(key);
                return Err(CapErr::InvalidCapability(msg));
            }
            // from/to at the FIRST '@'; the single-user form (no '@')
            // speaks for the same user (donor `to = from`).
            let to: &[u8] = match from_to.iter().position(|&b| b == b'@') {
                None => from_to,
                Some(j) => {
                    let (from, to) = (&from_to[..j], &from_to[j + 1..]);
                    let current = crate::devcons::current_user().map_err(CapErr::Static)?;
                    if current != from {
                        return Err(CapErr::Static("capability must match user"));
                    }
                    to
                }
            };
            crate::devcons::set_current_user(to.to_vec()).map_err(CapErr::Static)?;
            Ok(buf.len() as u64)
        }
        QDIR => Err(CapErr::Static("devcap: directory is not writable")),
        _ => Err(CapErr::Static("devcap: write of unknown file")),
    }
}

/// Read (donor capread:182-194): the dir serves the dirtab listing;
/// every file read errors Eperm (capuse is write-only 0222, caphash
/// 0200 — the hashes are never readable back).
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    match c.qid.path {
        QDIR => {
            let off = if offset == u64::MAX { c.offset } else { offset };
            let mut listing = Vec::new();
            let ncapdir = {
                let node = LockNode::new();
                CAPALLOC.lock(&node).ncapdir
            };
            for (name, qid, mode) in &DIRTAB[..ncapdir] {
                if *name == b"." {
                    continue; // the root's own record is the dir read's shape, not an entry
                }
                let start = listing.len();
                listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
                write_stat_record(
                    &mut listing[start..],
                    name,
                    *qid,
                    QTFILE,
                    *mode,
                    0,
                )?;
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
        _ => Err("devcap: permission denied"),
    }
}

/// stat (donor capstat over the served dirtab prefix): the root serves
/// "#¤" DMDIR|0500; a file serves its dirtab row; caphash after the
/// hide errors (it is no longer in the served table — the donor's
/// devstat consults ncapdir).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return write_stat_record(sb, ROOT_NAME, QDIR, QTDIR, 0x8000_0000 | 0o500, 0);
    }
    let ncapdir = {
        let node = LockNode::new();
        CAPALLOC.lock(&node).ncapdir
    };
    let Some((name, qid, mode)) = DIRTAB[..ncapdir].iter().find(|(_, q, _)| *q == c.qid.path) else {
        return Err("devcap: nonexistent file");
    };
    write_stat_record(sb, name, *qid, QTFILE, *mode, 0)
}

/// remove (donor capremove:63-70): the hostowner removing caphash
/// PERMANENTLY shrinks the served dirtab (`ncapdir = nelem-1` — the
/// entry list itself is untouched); anything else errors Eperm.
/// NOTE: the REMOVE syscall is loud-unimplemented in r9's table this
/// era (syscall.rs's named list) — this op is the ported half, host-
/// pinned now, wired when sys_remove lands.
pub fn remove(c: &Chan) -> Result<()> {
    if crate::devcons::iseve()? && c.qid.path == QHASH {
        let node = LockNode::new();
        CAPALLOC.lock(&node).ncapdir = DIRTAB.len() - 1;
        Ok(())
    } else {
        Err("devcap: permission denied")
    }
}

/// One stat(5) wire record (the devcons/devsrv shape; uid/gid/muid
/// glenda; FOUR strings — statcheck walks four).
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
        return Err("devcap: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type
    w.u32(DEV_CAP as u32);
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
    w.string(b"glenda");
    Ok(w.finish())
}
