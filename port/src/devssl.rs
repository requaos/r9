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
//! per-conv state rides `Arc<Conv>`: a brief-op `Lock<Dstate>` for
//! the table/ctl-visible fields, plus TWO direction cells (donor
//! `s->in.q` / `s->out.q`, 19-08's lock split — the donor-faithful
//! direction separation that keeps a parked ssl READ from blocking
//! ssl WRITES on the same conv: the -G session's 9P reader parks
//! indefinitely in the in-direction while rc's prompt Twrites ride
//! the out-direction).  The directions are `BusyCell`s, not MCS
//! locks, because their guards SPAN the blocking wire IO (the
//! donor holds in.q across ensure()): an MCS lock held across a
//! park would spin-deadlock an IRQ-masked contender on this
//! single-CPU era (mcslock's BusyCell note) — a BusyCell contender
//! parks ~1 tick and retries via devmnt's contention hook instead.
//! The ctl path claims BOTH directions (the donor's in.ctlq+out.q),
//! so algorithm changes are excluded while either direction runs.
//! The underlying fd chan (`fd N` -> donor buftochan's fdtochan
//! incref) arrives through the arch-registered fd-resolver hook —
//! the devsrv set_resolve_hook idiom; the x86_64 boot wires
//! `proc::current_fd_resolve` (ref_clone = Chan::dup, the open-ref
//! shape whose conv-free close_chan balances).  The conv keeps TWO
//! dups of it — one per direction — so the parked reader and an
//! active writer hold SEPARATE chan handles (concurrent &mut on one
//! Chan is impossible; the pipe/TCP state itself lives behind the
//! device's own Arc locks).
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

use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_SSL, OREAD, ORDWR, OWRITE, QTDIR, QTFILE};
use crate::mcslock::{BusyCell, Lock, LockNode};
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

/// One direction of the session (donor OneWay minus its locks —
/// direction ownership stands in for q/ctlq).
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

/// The IN direction (donor `s->in` + its half of the wire): everything
/// the direction's owner mutates while it holds the BusyCell — the
/// guard IS the lock (the BusyCell discipline; an MCS guard would spin
/// a contender when this direction parks in ensure()).
struct InDir {
    /// This direction's dup of the underlying fd chan (donor `s->c`,
    /// split per direction so a parked read and an active write hold
    /// separate handles).
    io: Option<Chan>,
    way: OneWay,
    /// Decoded plaintext not yet handed to the reader (donor
    /// s->processed).
    processed: Vec<u8>,
    /// Wire bytes read but not yet parsed into a record (donor
    /// s->unprocessed) — kept here so an interrupted ensure resumes
    /// exactly where it parked (the donor's Eintr discipline).
    unprocessed: Vec<u8>,
}

impl InDir {
    fn new() -> InDir {
        InDir { io: None, way: OneWay::new(), processed: Vec::new(), unprocessed: Vec::new() }
    }
}

/// The OUT direction (donor `s->out` + its half of the wire).
struct OutDir {
    io: Option<Chan>,
    way: OneWay,
}

impl OutDir {
    fn new() -> OutDir {
        OutDir { io: None, way: OneWay::new() }
    }
}

/// The brief-op per-conv state (donor Dstate's table/ctl-visible
/// fields).  Guarded by a plain MCS `Lock` — every access under it is
/// short and never spans a park.  Mutations additionally require BOTH
/// direction claims (the donor's ctl path takes in.ctlq AND out.q), so
/// a direction owner's post-claim snapshot is stable for its whole
/// ownership.
struct Dstate {
    state: u8,
    refc: u32,
    encryptalg: u8,
    blocklen: u16,
    diglen: u16,
    /// Donor `s->hf != nil` — sha1 is the only hash this port serves.
    hashing: bool,
    max: u32,
    maxpad: u32,
    user: Vec<u8>,
    perm: u32,
}

impl Dstate {
    fn new(user: Vec<u8>) -> Dstate {
        Dstate {
            state: SINCOMPLETE,
            refc: 1,
            encryptalg: NOENCRYPTION,
            blocklen: 1,
            diglen: 0,
            hashing: false,
            max: 0,
            maxpad: 0,
            user,
            perm: 0o660,
        }
    }
}

/// One ssl conversation: the shared brief-op state + the two
/// direction cells (donor `s->in`/`s->out`).
struct Conv {
    shared: Lock<Dstate>,
    indir: BusyCell<InDir>,
    outdir: BusyCell<OutDir>,
}

impl Conv {
    fn new(user: Vec<u8>) -> Conv {
        Conv {
            shared: Lock::new("dstate", Dstate::new(user)),
            indir: BusyCell::new(InDir::new()),
            outdir: BusyCell::new(OutDir::new()),
        }
    }
}

/// Claim the in-direction, parking ~1 tick between attempts (the
/// BusyCell discipline — see the module header).  Returns with
/// NOTHING else held; snapshot the shared params only AFTER this
/// succeeds (mutations need both directions, so the claim freezes
/// them).
fn claim_in(conv: &Arc<Conv>) -> Result<crate::mcslock::BusyGuard<'_, InDir>> {
    loop {
        if let Some(g) = conv.indir.enter() {
            return Ok(g);
        }
        crate::devmnt::contention_park("devssl: concurrent ssl read this era")?;
    }
}

/// Claim the out-direction (the write sibling of `claim_in`).
fn claim_out(conv: &Arc<Conv>) -> Result<crate::mcslock::BusyGuard<'_, OutDir>> {
    loop {
        if let Some(g) = conv.outdir.enter() {
            return Ok(g);
        }
        crate::devmnt::contention_park("devssl: concurrent ssl write this era")?;
    }
}

/// The algorithm parameters a direction owner snapshots after its
/// claim (stable for the ownership's duration — mutations require
/// both directions).
struct AlgParams {
    state: u8,
    encryptalg: u8,
    diglen: u16,
    hashing: bool,
    max: u32,
}

fn alg_params(d: &Dstate) -> AlgParams {
    AlgParams {
        state: d.state,
        encryptalg: d.encryptalg,
        diglen: d.diglen,
        hashing: d.hashing,
        max: d.max,
    }
}

/// The device state (donor `dstate[Maxdstate]` + `dshiwat`): conv
/// slots + the high-water mark of ever-allocated slots (the donor's
/// walkable-set bound — it only grows).
struct SslState {
    convs: [Option<Arc<Conv>>; MAXDSTATE],
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
type FdResolve = fn(u64) -> Option<Chan>;
static FD_RESOLVE: Lock<Option<FdResolve>> = Lock::new("devsslfd", None);

/// Register the arch-side fd resolver.  A second registration is a
/// boot bug.
pub fn set_fd_resolve_hook(hook: FdResolve) {
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

/// The donor's open-perm mapping (sslopen): OREAD->4, OWRITE->2,
/// ORDWR->6 over the access-mode field.
fn open_perm(access: u32) -> u32 {
    match access {
        OREAD => 4,
        OWRITE => 2,
        ORDWR => 6,
        _ => 0,
    }
}

/// Open (donor sslopen): dirs/encalgs/hashalgs OREAD-only (Eperm
/// otherwise); clone ALLOCATES a conv (first empty slot, dsclone) and
/// the returned chan's qid becomes the conv's ctl (donor dsnew's
/// Qclonus->Qctl rewrite); ctl/data/secretin/secretout create the conv
/// when the slot is empty (donor dsnew) or perm-check + ref-bump.
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let access = mode & crate::dat::OMASK;
    let perm = open_perm(access);
    let opened = |qid: Qid| -> Chan {
        let mut nc = Chan::new(DEV_SSL, qid, mode);
        nc.path = c.path.clone();
        nc
    };
    match typ_of(c.qid.path) {
        QTOPDIR | QPROTODIR | QCONVDIR => {
            if access != OREAD {
                return Err("permission denied");
            }
            Ok(opened(c.qid))
        }
        QCLONUS => {
            // dsclone: the donor's first-empty-slot scan; exhaustion
            // refuses loudly (Enodev), never wraps.
            let user = devcons::current_user()?;
            let node = LockNode::new();
            let mut guard = DSSTATE.lock(&node);
            let Some(slot) = (0..MAXDSTATE).find(|&i| guard.convs[i].is_none()) else {
                return Err("no free devices");
            };
            guard.convs[slot] = Some(Arc::new(Conv::new(user)));
            if slot >= guard.hiwat {
                guard.hiwat = slot + 1;
            }
            drop(guard);
            // donor dsnew: the clone chan's qid becomes the conv CTL.
            Ok(opened(Qid { path: qid(slot, QCTL), vers: 0, typ: QTFILE }))
        }
        QENCALGS | QHASHALGS => {
            if access != OREAD {
                return Err("permission denied");
            }
            Ok(opened(c.qid))
        }
        QCTL | QDATA | QSECRETIN | QSECRETOUT => {
            let conv = conv_of(c.qid.path);
            let user = devcons::current_user()?;
            let node = LockNode::new();
            let mut guard = DSSTATE.lock(&node);
            if guard.convs[conv].is_none() {
                guard.convs[conv] = Some(Arc::new(Conv::new(user)));
                if conv >= guard.hiwat {
                    guard.hiwat = conv + 1;
                }
            } else {
                // The existing-conv arm: perm check then ref bump
                // (donor sslopen under dslock).
                let ds = guard.convs[conv].as_ref().unwrap().clone();
                drop(guard);
                let dnode = LockNode::new();
                let mut d = ds.shared.lock(&dnode);
                if (perm & (d.perm >> 6)) != perm && (user != d.user || (perm & d.perm) != perm) {
                    return Err("permission denied");
                }
                d.refc += 1;
            }
            Ok(opened(c.qid))
        }
        _ => Err("devssl: open of unknown file"),
    }
}

/// dat.rs Chan::dup's bump (19-08): every value copy of an OPEN ssl
/// data/ctl chan is a new open reference (devpipe's ref_open_end rule
/// extended to #D).  Without it the io dup a 9P session holds
/// (devmnt::fversion's ChanIo) does not keep the conversation alive —
/// cpu -R's remoteside closes the ssl fd after mounting /mnt/term, the
/// conv's refc hits zero, and the hung-up conversation kills every
/// later rpc of the mounted session (the measured post-mount walk
/// failures: /mnt/term/dev/cpunote + /mnt/term/dev/cons).
pub(crate) fn ref_open(qid_path: u64) {
    if !matches!(typ_of(qid_path), QCTL | QDATA) {
        return;
    }
    let conv = conv_of(qid_path);
    let node = LockNode::new();
    let Some(arc) = DSSTATE.lock(&node).convs.get(conv).and_then(|s| s.clone()) else {
        return;
    };
    let mut d = arc.shared.lock(&node);
    d.refc += 1;
}

/// The conv's Arc under a brief table lock (None when the slot is
/// empty — the donor's dstate[i]==0 shapes error Ebadusefd).
fn conv_arc(conv: usize) -> Result<Arc<Conv>> {
    let node = LockNode::new();
    DSSTATE
        .lock(&node)
        .convs
        .get(conv)
        .and_then(|s| s.clone())
        .ok_or("inappropriate use of fd")
}

/// Read (donor sslread): dirs serve the listing (sslgen); ctl serves
/// the decimal conv id; data serves sslbread's decoded stream;
/// encalgs/hashalgs serve the algorithm tables.
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let conv = conv_of(c.qid.path);
    match typ_of(c.qid.path) {
        QTOPDIR | QPROTODIR | QCONVDIR => {
            let off = if offset == u64::MAX { c.offset } else { offset };
            let hiwat = {
                let node = LockNode::new();
                DSSTATE.lock(&node).hiwat
            };
            let mut listing = Vec::new();
            let mut push = |name: &[u8], path: u64, typ: u8, mode: u32| -> Result<()> {
                let start = listing.len();
                listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
                write_stat_record(&mut listing[start..], name, path, typ, mode, 0)?;
                Ok(())
            };
            match typ_of(c.qid.path) {
                QTOPDIR => push(b"ssl", qid(0, QPROTODIR), QTDIR, 0o555)?,
                QPROTODIR => {
                    for s in 0..hiwat {
                        let name: Vec<u8> = s.to_string().into_bytes();
                        push(&name, qid(s, QCONVDIR), QTDIR, 0o555)?;
                    }
                    push(b"clone", qid(0, QCLONUS), QTFILE, 0o555)?;
                }
                _ => {
                    for (fname, typ) in &CONVFILES {
                        push(fname, qid(conv, *typ), QTFILE, 0o660)?;
                    }
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
        QCTL => {
            let s: Vec<u8> = conv.to_string().into_bytes();
            read_str_at(c, buf, offset, &s)
        }
        QENCALGS => read_str_at(c, buf, offset, ENCALGS),
        QHASHALGS => read_str_at(c, buf, offset, HASHALGS),
        QDATA => {
            let arc = conv_arc(conv)?;
            // The donor's in-direction (qlock(&s->in.q)): ownership
            // spans the blocking ensure() reads — a concurrent WRITE
            // on the same conv proceeds on ITS direction (19-08's
            // deadlock fix; the BusyCell never spins a contender).
            let mut dir = claim_in(&arc)?;
            let p = {
                let node = LockNode::new();
                alg_params(&arc.shared.lock(&node))
            };
            if p.state == SINCOMPLETE {
                return Err("inappropriate use of fd");
            }
            if dir.processed.is_empty() {
                sslbread(&mut dir, &p)?;
            }
            let n = buf.len().min(dir.processed.len());
            let taken: Vec<u8> = dir.processed.drain(..n).collect();
            buf[..n].copy_from_slice(&taken);
            Ok(n as u64)
        }
        _ => Err("inappropriate use of fd"),
    }
}

/// readstr (the donor's readstr(offset, a, n, buf) over the device's
/// string): offset-honoring, streaming when offset is the chan's.
fn read_str_at(c: &mut Chan, buf: &mut [u8], offset: u64, s: &[u8]) -> Result<u64> {
    let off = if offset == u64::MAX { c.offset } else { offset };
    let start = (off as usize).min(s.len());
    let avail = &s[start..];
    let n = avail.len().min(buf.len());
    buf[..n].copy_from_slice(&avail[..n]);
    if offset == u64::MAX {
        c.offset += n as u64;
    }
    Ok(n as u64)
}

/// ensure (donor ensure): read wire bytes until `unprocessed` holds at
/// least `n`; a zero-byte read is the hangup (Ehungup).  Called with
/// the in-direction owned (the park inside is the direction owner's to
/// make — no shared lock is held across it).
fn ensure(io: &mut Chan, unprocessed: &mut Vec<u8>, n: usize) -> Result<()> {
    let mut scratch = alloc::vec![0u8; MAXDMSG];
    while unprocessed.len() < n {
        let got = io_read_some(io, &mut scratch)?;
        if got == 0 {
            return Err("i/o on hungup channel");
        }
        unprocessed.extend_from_slice(&scratch[..got as usize]);
    }
    Ok(())
}

/// checkdigestb (donor): sha1(secret || message-after-digest ||
/// big-endian mid) against the record's leading diglen bytes.
fn check_digest(dir: &InDir, diglen: u16, body: &[u8]) -> Result<()> {
    let diglen = diglen as usize;
    let mut input = Vec::with_capacity(dir.way.secret.len() + body.len() + 4);
    input.extend_from_slice(&dir.way.secret);
    input.extend_from_slice(&body[diglen..]);
    input.extend_from_slice(&dir.way.mid.to_be_bytes());
    let digest = crate::hmacsha1::sha1(&input);
    if digest[..diglen] != body[..diglen] {
        return Err("bad digest");
    }
    Ok(())
}

/// sslbread (donor sslbread over Block lists, ported to Vec buffers):
/// parse the SSL-format header off the wire, decrypt/digest-check the
/// record, stage the plaintext into `processed`.  The 2-byte |0x80
/// form is the encrypted/digest shape this device writes; the 3-byte
/// padded form is still PARSED (donor-exact — a block-cipher peer's
/// frame must not wedge the reader).  Runs under IN-direction
/// ownership with the claim-time algorithm snapshot `p` (stable for
/// the ownership — ctl needs both directions).
fn sslbread(dir: &mut InDir, p: &AlgParams) -> Result<()> {
    if p.state == SINCOMPLETE {
        return Err("inappropriate use of fd");
    }
    let (len, pad, toconsume) = {
        let io = dir.io.as_mut().ok_or("inappropriate use of fd")?;
        ensure(io, &mut dir.unprocessed, 3)?;
        let h = &dir.unprocessed;
        if h[0] & 0x80 != 0 {
            ((((h[0] & 0x7f) as usize) << 8) | h[1] as usize, 0usize, 2usize)
        } else {
            let len = (((h[0] & 0x3f) as usize) << 8) | h[1] as usize;
            let pad = h[2] as usize;
            if pad > len {
                return Err("bad pad in ssl message");
            }
            (len, pad, 3)
        }
    };
    ensure(dir.io.as_mut().unwrap(), &mut dir.unprocessed, toconsume + len)?;
    dir.unprocessed.drain(..toconsume);
    let mut body: Vec<u8> = dir.unprocessed.drain(..len).collect();
    let mut plen = len;
    match p.state {
        SENCRYPTING => {
            if body.is_empty() {
                return Err("ssl message too short (encrypting)");
            }
            if let Some(st) = dir.way.rc4.as_mut() {
                st.rc4(&mut body);
            }
        }
        SDIGESTING | SDIGENC => {
            if p.state == SDIGENC
                && let Some(st) = dir.way.rc4.as_mut()
            {
                st.rc4(&mut body);
            }
            if body.len() < p.diglen as usize {
                return Err("ssl message too short (dig+enc)");
            }
            check_digest(dir, p.diglen, &body)?;
            body.drain(..p.diglen as usize);
            plen -= p.diglen as usize;
        }
        _ => {}
    }
    if pad > 0 && pad <= plen {
        body.truncate(plen - pad);
    }
    dir.processed = body;
    dir.way.mid += 1;
    Ok(())
}

/// One SSL-format record (donor sslput's loop body, the pad-free
/// shape — blocklen is only ever 1 this era, the rc4_256 class, so no
/// block padding is ever WRITTEN; the reader still parses padded
/// frames): [0x80|len_hi, len_lo] counting digest+payload, the digest
/// computed over secret||payload||mid and prepended INSIDE the
/// encrypted body (the header stays clear — drawterm's vendored arm
/// runs the identical bytes).  Runs under OUT-direction ownership
/// with the claim-time snapshot `p` — a parked ssl read on the same
/// conv never blocks this (19-08's direction split).
fn sslput_record(dir: &mut OutDir, p: &AlgParams, chunk: &[u8]) -> Result<()> {
    let diglen = p.diglen as usize;
    let l2 = chunk.len() as u32 + p.diglen as u32;
    let mut rec = alloc::vec![0u8; 2 + diglen + chunk.len()];
    rec[0] = ((l2 >> 8) as u8) | 0x80;
    rec[1] = l2 as u8;
    rec[2 + diglen..].copy_from_slice(chunk);
    if p.hashing {
        let mut input = Vec::with_capacity(dir.way.secret.len() + chunk.len() + 4);
        input.extend_from_slice(&dir.way.secret);
        input.extend_from_slice(chunk);
        input.extend_from_slice(&dir.way.mid.to_be_bytes());
        let digest = crate::hmacsha1::sha1(&input);
        rec[2..2 + diglen].copy_from_slice(&digest[..diglen]);
    }
    if p.encryptalg == RC4ALG
        && let Some(st) = dir.way.rc4.as_mut()
    {
        st.rc4(&mut rec[2..]);
    }
    let io = dir.io.as_mut().ok_or("inappropriate use of fd")?;
    io_write(io, &rec)?;
    dir.way.mid += 1;
    Ok(())
}

/// Write (donor sslwrite): data rides sslput (the SSL-format record
/// builder, chunked at max) under OUT-direction ownership;
/// secretin/secretout/ctl claim BOTH directions (the donor's
/// in.ctlq+out.q) and run the verb machine.
pub fn write(c: &mut Chan, buf: &[u8]) -> Result<u64> {
    let conv = conv_of(c.qid.path);
    match typ_of(c.qid.path) {
        QDATA => {
            let arc = conv_arc(conv)?;
            // The donor's out-direction (qlock(&s->out.q)): a parked
            // ssl READ on the same conv holds only ITS direction.
            let mut dir = claim_out(&arc)?;
            let p = {
                let node = LockNode::new();
                alg_params(&arc.shared.lock(&node))
            };
            if p.state == SINCOMPLETE {
                return Err("inappropriate use of fd");
            }
            let max = p.max.max(1) as usize;
            for chunk in buf.chunks(max) {
                sslput_record(&mut dir, &p, chunk)?;
            }
            Ok(buf.len() as u64)
        }
        QSECRETIN | QSECRETOUT | QCTL => {
            let arc = conv_arc(conv)?;
            // The donor's ctl-path locks (in.ctlq THEN out.q), taken
            // in that order; mutations hold BOTH, so a direction
            // owner's post-claim snapshot is stable.
            let mut indir = claim_in(&arc)?;
            let mut outdir = claim_out(&arc)?;
            let node = LockNode::new();
            let mut d = arc.shared.lock(&node);
            ctl_write(&mut d, &mut indir, &mut outdir, buf)
        }
        QCLONUS => Err("devssl: clone is not writable"),
        QENCALGS | QHASHALGS => {
            // The donor PANICS here (sslwrite's default arm) — a
            // write refusal is the port's loud-refusal discipline for
            // the donor's own bug class.
            Err("devssl: alg tables are not writable")
        }
        _ => Err("devssl: directory is not writable"),
    }
}

/// The ctl verb machine (donor sslwrite's Qctl parse, devssl.c:1139-1211
/// — verb at the first space, secrets dec64'd, fd-before-alg enforced).
/// Runs with BOTH directions claimed and the shared lock held (the
/// donor's in.ctlq+out.q).
fn ctl_write(d: &mut Dstate, indir: &mut InDir, outdir: &mut OutDir, buf: &[u8]) -> Result<u64> {
    if buf.len() >= 128 {
        return Err("arg too long");
    }
    let mut line = buf;
    if let Some(nl) = line.iter().position(|&b| b == b'\n') {
        line = &line[..nl];
    }
    let (verb, p) = match line.iter().position(|&b| b == b' ') {
        Some(sp) => (&line[..sp], Some(&line[sp + 1..])),
        None => (line, None),
    };

    if verb == b"fd" {
        // buftochan: fdtochan incref (the resolver's dup shape), the
        // device's own files refused (donor's "cannot ssl encrypt
        // devssl files" — the recursion guard).  The conv keeps TWO
        // dups — one per direction — so a parked read and an active
        // write hold separate Chan handles (19-08's split); any
        // previous pair is released first (the donor's incref/decref
        // balance, which the single-store replace used to skip).
        let Some(p) = p else { return Err("bad arg in system call") };
        let s = core::str::from_utf8(p).map_err(|_| "bad arg in system call")?;
        let fd: u64 = s.trim().parse().map_err(|_| "bad arg in system call")?;
        let chan = resolve_fd(fd)?;
        if chan.dev == DEV_SSL {
            crate::namec::close_chan(&chan);
            return Err("cannot ssl encrypt devssl files");
        }
        let wchan = chan.dup();
        if let Some(old) = indir.io.take() {
            crate::namec::close_chan(&old);
        }
        if let Some(old) = outdir.io.take() {
            crate::namec::close_chan(&old);
        }
        indir.io = Some(chan);
        outdir.io = Some(wchan);
        // Default is clear (msg delimiters only) — donor's fd arm.
        d.state = SCLEAR;
        d.blocklen = 1;
        d.diglen = 0;
        d.max = rc4::ssl_max(1, 0);
        d.maxpad = rc4::ssl_maxpad(1, 0);
        indir.way.mid = 0;
        outdir.way.mid = 0;
        return Ok(buf.len() as u64);
    }

    if verb == b"alg" {
        let Some(p) = p else { return Err("bad arg in system call") };
        d.blocklen = 1;
        d.diglen = 0;
        if indir.io.is_none() {
            return Err("must set fd before algorithm");
        }
        d.state = SCLEAR;
        d.max = rc4::ssl_max(1, 0);
        d.maxpad = rc4::ssl_maxpad(1, 0);
        if p == b"clear" {
            return Ok(buf.len() as u64); // the -e clear debug fallback
        }
        // The donor's single-secret auto-copy (alg with only one
        // direction armed mirrors it to the other).
        if !indir.way.secret.is_empty() && outdir.way.secret.is_empty() {
            outdir.way.secret = indir.way.secret.clone();
        }
        if !outdir.way.secret.is_empty() && indir.way.secret.is_empty() {
            indir.way.secret = outdir.way.secret.clone();
        }
        if indir.way.secret.is_empty() || outdir.way.secret.is_empty() {
            return Err("algorithm but no secret");
        }
        d.hashing = false;
        d.encryptalg = NOENCRYPTION;
        d.blocklen = 1;
        // Donor parsehashalg || parseencryptalg per space-separated
        // token (empty tokens fail — the donor's split shape).
        for tok in p.split(|&b| b == b' ') {
            if tok == b"sha1" {
                d.hashing = true;
                d.diglen = SHA1DIGLEN;
                d.state &= !SCLEAR;
                d.state |= SDIGESTING;
            } else if tok == b"rc4_256" {
                d.encryptalg = RC4ALG;
                d.blocklen = 1;
                indir.way.rc4 = Some(rc4::setup_rc4_state(&indir.way.secret));
                outdir.way.rc4 = Some(rc4::setup_rc4_state(&outdir.way.secret));
                d.state &= !SCLEAR;
                d.state |= SENCRYPTING;
            } else {
                return Err("bad algorithm");
            }
        }
        if !d.hashing && d.encryptalg == NOENCRYPTION {
            return Err("bad algorithm");
        }
        d.max = rc4::ssl_max(d.blocklen, d.diglen);
        d.maxpad = rc4::ssl_maxpad(d.blocklen, d.diglen);
        return Ok(buf.len() as u64);
    }

    if verb == b"secretin" || verb == b"secretout" {
        let Some(p) = p else { return Err("bad arg in system call") };
        // Donor buffer bounds: (strlen*3)/2 for secretin, +1 for
        // secretout (both over-provisioned for base64 — kept).
        let l = (p.len() * 3) / 2 + usize::from(verb == b"secretout");
        let mut x = alloc::vec![0u8; l];
        let t = rc4::dec64(&mut x, p);
        if t == 0 {
            return Err("bad arg in system call");
        }
        let secret = x[..t].to_vec();
        if verb == b"secretin" {
            indir.way.secret = secret;
        } else {
            outdir.way.secret = secret;
        }
        return Ok(buf.len() as u64);
    }

    Err("bad arg in system call")
}

/// stat (donor sslstat over sslgen): "#D"/"ssl"/decimal conv dirs at
/// 0555; clone at 0555; conv files at 0660.
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let conv = conv_of(c.qid.path);
    match typ_of(c.qid.path) {
        QTOPDIR => write_stat_record(sb, b"#D", c.qid.path, QTDIR, 0x8000_0000 | 0o555, 0),
        QPROTODIR => write_stat_record(sb, b"ssl", c.qid.path, QTDIR, 0x8000_0000 | 0o555, 0),
        QCONVDIR => {
            let name: Vec<u8> = conv.to_string().into_bytes();
            write_stat_record(sb, &name, c.qid.path, QTDIR, 0x8000_0000 | 0o555, 0)
        }
        QCLONUS => write_stat_record(sb, b"clone", c.qid.path, QTFILE, 0o555, 0),
        t => {
            let Some((name, _)) = CONVFILES.iter().find(|(_, ty)| *ty == t) else {
                return Err("devssl: stat of unknown file");
            };
            write_stat_record(sb, name, c.qid.path, QTFILE, 0o660, 0)
        }
    }
}

/// close (donor sslclose): an OPEN ctl/data/secretin/secretout chan
/// drops a conv reference; the last close frees the slot, hangs the
/// conv up, and releases BOTH direction io chans (the donor's cclose —
/// namec::close_chan on each held dup; refc==0 means no data read can
/// be parked mid-record — the reading syscall holds an open chan).
pub fn close(c: &Chan) {
    if !matches!(typ_of(c.qid.path), QCTL | QDATA | QSECRETIN | QSECRETOUT) || !c.copen {
        return;
    }
    let conv = conv_of(c.qid.path);
    let node = LockNode::new();
    let arc = DSSTATE.lock(&node).convs.get(conv).and_then(|s| s.clone());
    let Some(arc) = arc else { return };
    let last = {
        let mut d = arc.shared.lock(&node);
        if d.refc == 0 {
            return;
        }
        d.refc -= 1;
        d.refc == 0
    };
    if last {
        {
            let mut guard = DSSTATE.lock(&node);
            if let Some(slot) = guard.convs.get(conv)
                && slot.as_ref().is_some_and(|s| Arc::ptr_eq(s, &arc))
            {
                guard.convs[conv] = None;
            }
        }
        // sslhangup + the frees: the direction cells drop with the
        // Conv once the table slot and this Arc go; each direction's
        // io dup gets the donor's explicit cclose.  Claim both cells
        // first (uncontended at refc==0 — the parked-reader window
        // cannot exist without an open chan keeping refc>0).
        if let Ok(mut indir) = claim_in(&arc)
            && let Ok(mut outdir) = claim_out(&arc)
        {
            if let Some(io) = indir.io.take() {
                crate::namec::close_chan(&io);
            }
            if let Some(io) = outdir.io.take() {
                crate::namec::close_chan(&io);
            }
        }
    }
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
