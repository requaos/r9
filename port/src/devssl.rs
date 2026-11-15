//! devssl — the `#D` secure-sockets-layer device (DESK-04, 19-04).  A
//! semantics-level Rust port of 9k's `sys/src/9k/port/devssl.c` (1520
//! lines, the donor of record).  Donor line cites below name the
//! semantics being ported; this is not a literal translation (the
//! project's D-06: port the donor, never the phrase).
//!
//! WHY this device: `pushssl` (libc/9sys/pushssl.c) HARD-CODES `#D` —
//! the cpu session's encrypted arm (cpu.c p9auth client-side,
//! srvp9auth server-side, D-15's encrypted default) speaks exactly
//! this protocol: open `#D/ssl/clone`, read the decimal conv id, open
//! `#D/ssl/N/data`, then ctl writes `fd N` / `secretin %s` /
//! `secretout %s` / `alg %s` — IN THAT ORDER (the donor enforces
//! fd-before-alg at devssl.c:1153).
//!
//! The secrets are dec64 (NEVER hex — u64.c's codec, port/src/rc4.rs)
//! of mksecret-shaped 20-char uppercase-hex ASCII; the wire is the
//! donor's "SSL format" framing (2-byte |0x80 header counting
//! digest+payload, the digest prepended INSIDE the encrypted body,
//! sha1 over secret‖message‖msgid) — ported as-is because drawterm's
//! vendored kern/devssl.c runs the identical framing and a cleaner
//! port breaks interop SILENTLY (auth succeeds, then every read
//! garbles — 19-RESEARCH Pitfall 11; the framing is pinned by
//! round-trip host pins, not restyled).
//!
//! Scoping (the pushssl subset, recorded per the plan): the NOSPOOKS
//! table reduced to `rc4_256` + `sha1` (what `cpu`'s ealgs default
//! `rc4_256 sha1` exercises — rc4_40/rc4_128/DES and md4/md5 are out
//! of measure); encalgs reads "rc4_256", hashalgs reads "sha1".
//! rc4_256/sha1 is the era shape, PROTOCOL-blocked from stronger
//! crypto per DESK-04's requirement text (T-19-08: accepted,
//! documented; the hardening milestone owns the upgrade).
//!
//! Structure (the devcap small-device skeleton): a `Lock`-held
//! conv table (donor `dstate[Maxdstate]`, Maxdstate=512 — capacity
//! only; drawterm's own 128 does not constrain the kernel side) +
//! free-function attach/walk/open/read/write/stat/close.  The
//! per-conv state rides `Arc<Lock<Dstate>>` so a blocking wire read
//! on one conv never holds the TABLE lock (the donor's per-conv in.q
//! /out.q discipline, folded to one lock per conv — a tightening,
//! never a loosening: the donor's ctl path takes BOTH in.ctlq and
//! out.q).  The underlying fd chan (`fd N` -> donor buftochan's
//! fdtochan incref) arrives through the arch-registered fd-resolver
//! hook — the devsrv set_resolve_hook idiom; the x86_64 boot wires
//! `proc::current_fd_resolve` (ref_clone = Chan::dup, the open-ref
//! shape whose conv-free close_chan balances).
//!
//! Qid scheme (donor devssl.c:86-101, verbatim): the enum
//! Qtopdir=1..Qhashalgs=10, `QID(c,y) = (c<<5)|y`, `TYPE(x) =
//! path&0xf`, `CONV(x) = (path>>5)&(Maxdstate-1)`.
//!
//! Error strings carry the donor's literals VERBATIM where the
//! donor's are literals ("must set fd before algorithm", "bad
//! algorithm", "algorithm but no secret", "arg too long", "bad
//! digest", "bad pad in ssl message", "cannot ssl encrypt devssl
//! files", "ssl message too short (dig+enc)") plus the error.h
//! classes (Ebadarg "bad arg in system call", Eperm "permission
//! denied", Ebadusefd "inappropriate use of fd", Ehungup "i/o on
//! hungup channel", Enodev "no free devices").  No formatted-error
//! transport is needed — every donor devssl error is a literal (the
//! devcap CapErr pattern exists for the one donor device whose
//! errors embed user bytes; devssl is not it).

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_SSL, OREAD, QTDIR, QTFILE};
use crate::mcslock::{Lock, LockNode};
use crate::{devcons, rc4, Result};

/// The device-registry rune (namec.rs's '#D' arm — pushssl hard-codes
/// `#D`; 'D' is one ASCII byte).
pub const REGISTRY_RUNE: char = 'D';

/// Donor `Maxdstate` = 512 (devssl.c:76): the conv-table capacity, a
/// power of two so CONV's mask stays a mask.  Allocation is the
/// donor's first-empty-slot scan; exhaustion refuses loudly
/// (Enodev-class), never wraps.
const MAXDSTATE: usize = 512;

/// Donor `Maxdmsg` = 1<<16 (devssl.c:75): the wire read size — one
/// whole record always fits.
const MAXDMSG: usize = 1 << 16;

/// The donor's qid enum (devssl.c:86-97).
const QTOPDIR: u64 = 1;
const QPROTODIR: u64 = 2;
const QCLONUS: u64 = 3;
const QCONVDIR: u64 = 4;
const QDATA: u64 = 5;
const QCTL: u64 = 6;
const QSECRETIN: u64 = 7;
const QSECRETOUT: u64 = 8;
const QENCALGS: u64 = 9;
const QHASHALGS: u64 = 10;

/// The donor's QID layout macros (devssl.c:99-101).
const fn qid(conv: usize, typ: u64) -> u64 {
    ((conv as u64) << 5) | typ
}
const fn typ_of(path: u64) -> u64 {
    path & 0xf
}
const fn conv_of(path: u64) -> usize {
    ((path >> 5) as usize) & (MAXDSTATE - 1)
}

/// Connection states (donor devssl.c:30-34).
const SINCOMPLETE: u8 = 0;
const SCLEAR: u8 = 1;
const SENCRYPTING: u8 = 2;
const SDIGESTING: u8 = 4;
const SDIGENC: u8 = SENCRYPTING | SDIGESTING;

/// Encryption algorithm ids (donor devssl.c:37-40) — this port
/// carries RC4 only (the scoping above); Noencryption is the
/// alg-parse reset value.
const NOENCRYPTION: u8 = 0;
const RC4ALG: u8 = 3;

/// The one hash this port serves (donor hashtab's sha1 row):
/// SHA1dlen via hmacsha1's block — the digest half composes
/// `sha1(secret || message || msgid)` (the donor's streaming calls
/// over the same three parts).
const SHA1DIGLEN: u16 = crate::hmacsha1::SHA1DLEN as u16;

/// The encalgs/hashalgs strings (donor sslinit builds them from the
/// tables; the scoped tables carry exactly these members).
const ENCALGS: &[u8] = b"rc4_256";
const HASHALGS: &[u8] = b"sha1";

/// The conv-dir file table (donor sslgen Qconvdir order: s 0..5).
const CONVFILES: [(&[u8], u64); 6] = [
    (b"ctl", QCTL),
    (b"data", QDATA),
    (b"secretin", QSECRETIN),
    (b"secretout", QSECRETOUT),
    (b"encalgs", QENCALGS),
    (b"hashalgs", QHASHALGS),
];

/// One direction of the session (donor OneWay minus its locks — the
/// conv lock stands in for q/ctlq).
struct OneWay {
    /// Donor `w->secret`/`w->slen`: the dec64'd (or raw file-written)
    /// secret bytes.
    secret: Vec<u8>,
    /// Donor `w->mid`: the per-record message id hashed into every
    /// digest — the replay/ordering ordinal both ends count in lockstep.
    mid: u32,
    /// Donor `w->state` for the RC4 key class (initRC4key's
    /// setupRC4state product, armed at alg-parse time from the secret).
    rc4: Option<rc4::Rc4state>,
}

impl OneWay {
    fn new() -> OneWay {
        OneWay { secret: Vec::new(), mid: 0, rc4: None }
    }
}

/// One ssl conversation (donor Dstate).
struct Dstate {
    /// Donor `s->c`: the underlying fd's chan (buftochan's incref'd
    /// reference — a dup-shape clone whose conv-free close balances).
    io: Option<Chan>,
    state: u8,
    refc: u32,
    encryptalg: u8,
    blocklen: u16,
    diglen: u16,
    /// Donor `s->hf != nil` — sha1 is the only hash this port serves.
    hashing: bool,
    max: u32,
    maxpad: u32,
    inway: OneWay,
    outway: OneWay,
    /// Decoded plaintext not yet handed to the reader (donor
    /// s->processed).
    processed: Vec<u8>,
    /// Wire bytes read but not yet parsed into a record (donor
    /// s->unprocessed).
    unprocessed: Vec<u8>,
    user: Vec<u8>,
    perm: u32,
}

impl Dstate {
    fn new(user: Vec<u8>) -> Dstate {
        Dstate {
            io: None,
            state: SINCOMPLETE,
            refc: 1,
            encryptalg: NOENCRYPTION,
            blocklen: 1,
            diglen: 0,
            hashing: false,
            max: 0,
            maxpad: 0,
            inway: OneWay::new(),
            outway: OneWay::new(),
            processed: Vec::new(),
            unprocessed: Vec::new(),
            user,
            perm: 0o660,
        }
    }
}

/// The device state (donor `dstate[Maxdstate]` + `dshiwat`): conv
/// slots + the high-water mark of ever-allocated slots (the donor's
/// walkable-set bound — it only grows).
struct SslState {
    convs: [Option<Arc<Lock<Dstate>>>; MAXDSTATE],
    hiwat: usize,
}

static DSSTATE: Lock<SslState> = Lock::new(
    "dstate",
    SslState { convs: [const { None }; MAXDSTATE], hiwat: 0 },
);

/// The arch-side fd resolver (donor buftochan's fdtochan): the
/// CALLING proc's fd N as an open-reference chan.  Registered once at
/// boot (the devsrv set_resolve_hook idiom); the x86_64 boot wires
/// proc::current_fd_resolve.
static FD_RESOLVE: Lock<Option<fn(u64) -> Option<Chan>>> = Lock::new("devsslfd", None);

/// Register the arch-side fd resolver.  A second registration is a
/// boot bug.
pub fn set_fd_resolve_hook(hook: fn(u64) -> Option<Chan>) {
    let node = LockNode::new();
    let mut guard = FD_RESOLVE.lock(&node);
    assert!(guard.is_none(), "devssl: fd resolver registered twice");
    *guard = Some(hook);
}

fn resolve_fd(fd: u64) -> Result<Chan> {
    let hook = {
        let node = LockNode::new();
        *FD_RESOLVE.lock(&node)
    };
    let Some(hook) = hook else {
        return Err("devssl: fd resolve hook not registered");
    };
    hook(fd).ok_or("devssl: bad fd")
}

// The test fd table (cfg(test) only): the host pins wire pipe-end
// chans "fd-to-fd" through the same resolver the kernel path uses.
#[cfg(test)]
static TEST_FDS: Lock<Vec<(u64, Chan)>> = Lock::new("devssltestfds", Vec::new());

#[cfg(test)]
fn test_resolve_fd(fd: u64) -> Option<Chan> {
    let node = LockNode::new();
    TEST_FDS
        .lock(&node)
        .iter()
        .find(|(f, _)| *f == fd)
        .map(|(_, c)| c.dup())
}

/// Install a clean device state for a pin (the devcap
/// install_test_state discipline): fresh conv table + the test fd
/// resolver, each pin starting from an empty #D.
#[cfg(test)]
pub(crate) fn install_test_state() {
    {
        let node = LockNode::new();
        let mut guard = DSSTATE.lock(&node);
        for slot in guard.convs.iter_mut() {
            *slot = None;
        }
        guard.hiwat = 0;
    }
    let node = LockNode::new();
    TEST_FDS.lock(&node).clear();
    let mut hook = FD_RESOLVE.lock(&node);
    if hook.is_none() {
        *hook = Some(test_resolve_fd);
    }
}

/// Put a chan in the test fd table (the pin-side stand-in for an open
/// fd — dup'd on resolve, so conv-free's close_chan balances).
#[cfg(test)]
pub(crate) fn test_fd_put(fd: u64, c: Chan) {
    let node = LockNode::new();
    TEST_FDS.lock(&node).push((fd, c));
}

/// attach: the `#D` root dir chan (donor sslattach — qid Qtopdir).
pub fn attach() -> Chan {
    let mut c = Chan::new(DEV_SSL, Qid { path: qid(0, QTOPDIR), vers: 0, typ: QTDIR }, 0);
    c.path = b"#D".to_vec();
    c
}

/// Walk a single name component (donor sslwalk/devwalk over sslgen):
/// root -> "ssl"; protodir -> "clone" or a decimal conv dir below the
/// hiwat mark (the walkable set — donor gen emits convs [0, dshiwat)
/// then clone, string-matched, so "007" never matches "7").
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR {
        return Err("devssl: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devssl: walk takes a single name component");
    }
    let mk = |path: u64, path_name: &[u8]| -> Chan {
        let typ = if typ_of(path) == QTOPDIR || typ_of(path) == QPROTODIR || typ_of(path) == QCONVDIR {
            QTDIR
        } else {
            QTFILE
        };
        let mut nc = Chan::new(DEV_SSL, Qid { path, vers: 0, typ }, 0);
        nc.path = c.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(path_name);
        nc
    };
    let hiwat = {
        let node = LockNode::new();
        DSSTATE.lock(&node).hiwat
    };
    match typ_of(c.qid.path) {
        QTOPDIR => {
            if name == b"ssl" {
                Ok(mk(qid(0, QPROTODIR), b"ssl"))
            } else {
                Err("devssl: name not found")
            }
        }
        QPROTODIR => {
            if name == b"clone" {
                Ok(mk(qid(0, QCLONUS), b"clone"))
            } else {
                // Donor gen: the conv dirs for [0, hiwat), matched by
                // their rendered decimal names.
                for s in 0..hiwat {
                    let rendered: Vec<u8> = s.to_string().into_bytes();
                    if name == rendered.as_slice() {
                        return Ok(mk(qid(s, QCONVDIR), &rendered));
                    }
                }
                Err("devssl: name not found")
            }
        }
        QCONVDIR => {
            let conv = conv_of(c.qid.path);
            if let Some((fname, typ)) = CONVFILES.iter().find(|(n, _)| *n == name) {
                Ok(mk(qid(conv, *typ), fname))
            } else {
                Err("devssl: name not found")
            }
        }
        _ => Err("devssl: walk of a non-directory"),
    }
}

/// Open (donor sslopen): dirs/encalgs/hashalgs OREAD-only (Eperm
/// otherwise); clone ALLOCATES a conv (first empty slot, dsclone) and
/// the returned chan's qid becomes the conv's ctl (donor dsnew's
/// Qclonus->Qctl rewrite); ctl/data/secretin/secretout create the conv
/// when the slot is empty (donor dsnew) or perm-check + ref-bump.
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let _ = (c, mode);
    Err("devssl: not implemented this pass")
}

/// Read (donor sslread): dirs serve the listing (sslgen); ctl serves
/// the decimal conv id; data serves sslbread's decoded stream;
/// encalgs/hashalgs serve the algorithm tables.
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let _ = (buf, offset);
    match typ_of(c.qid.path) {
        QTOPDIR | QPROTODIR | QCONVDIR => Err("devssl: not implemented this pass"),
        _ => Err("devssl: not implemented this pass"),
    }
}

/// Write (donor sslwrite): data rides sslput (the SSL-format record
/// builder); secretin/secretout set the RAW bytes as the direction's
/// secret (donor setsecret — the dec64 path is the CTL verbs'); ctl
/// runs the verb machine.
pub fn write(c: &mut Chan, buf: &[u8]) -> Result<u64> {
    let _ = buf;
    match typ_of(c.qid.path) {
        QTOPDIR | QPROTODIR | QCONVDIR | QENCALGS | QHASHALGS | QCLONUS => {
            Err("devssl: not implemented this pass")
        }
        _ => Err("devssl: not implemented this pass"),
    }
}

/// stat (donor sslstat over sslgen): "#D"/"ssl"/decimal conv dirs at
/// 0555; clone at 0555; conv files at 0660.
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let _ = sb;
    match typ_of(c.qid.path) {
        QTOPDIR | QPROTODIR | QCONVDIR | QCLONUS => Err("devssl: not implemented this pass"),
        _ => Err("devssl: not implemented this pass"),
    }
}

/// close (donor sslclose): an OPEN ctl/data/secretin/secretout chan
/// drops a conv reference; the last close frees the slot, hangs the
/// conv up, and releases the underlying fd chan (the donor's cclose —
/// namec::close_chan on the held open-ref clone).
pub fn close(c: &Chan) {
    let _ = c;
}

/// The donor's io on the underlying chan (s->c->dev->bread/bwrite):
/// the connection-bearing devices dispatch here — DEV_PIPE (the host
/// pins' loopback substrate) and DEV_IP (the live TCP session, with
/// the syscall layer's drain-debt discipline).  Other devices refuse
/// loudly naming the gap (nobody ssl-wraps them this era).
fn io_read_some(c: &mut Chan, buf: &mut [u8]) -> Result<u64> {
    match c.dev {
        crate::dat::DEV_PIPE => crate::devpipe::read(c, buf, u64::MAX),
        crate::dat::DEV_IP => {
            crate::devip::stack::drain_debt_if_set();
            crate::devip::read(c, buf, u64::MAX)
        }
        _ => Err("devssl: fd is not a connection"),
    }
}

fn io_write(c: &mut Chan, buf: &[u8]) -> Result<u64> {
    match c.dev {
        crate::dat::DEV_PIPE => crate::devpipe::write(c, buf, u64::MAX),
        crate::dat::DEV_IP => {
            crate::devip::stack::drain_debt_if_set();
            crate::devip::write(c, buf, u64::MAX)
        }
        _ => Err("devssl: fd is not a connection"),
    }
}

/// One stat(5) wire record (the devcons/devcap shape; uid/gid/muid
/// glenda — stat content is not on the pushssl path, which never
/// stats).
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
        return Err("devssl: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type
    w.u32(DEV_SSL as u32);
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
