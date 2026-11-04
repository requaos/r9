//! devmnt — the kernel 9P client `#M` (05-07, D-03's mount half; the
//! virtio-9p prerequisite for v1.x).  Semantics ported from 9k's
//! devmnt.c (the RESEARCH Don't-Hand-Roll mandate — mntversion/mntattach/
//! mntwalk/mntopen/mntread/mntstat/mntclunk over the pipe fd, sized to
//! the paqfs message set) with the wire codec from fcall.h + convS2M.c /
//! convM2S.c.
//!
//! A mounted tree is a SESSION: sys_fversion negotiates the msize over
//! the fd's chan (Tversion/Rversion — "9P2000", offer 8192, accept the
//! server's lower answer) and parks the resulting `Arc<Mnt>` ON the chan
//! (dat.rs's `mnt` slot — the donor's `c->mux`); sys_mount attaches the
//! tree (Tattach, afid = NOFID — the D-02 no-auth path, nsinit's
//! fall-through) and inserts the mount-table entry at the resolved
//! target.  Chans in the mounted tree carry the session Arc plus their
//! server-side fid; namec's DEV_MNT legs dispatch walk/open/read/stat to
//! Twalk/Topen/Tread/Tstat RPCs, and namec::close_chan's DEV_MNT leg
//! sends Tclunk when an OPENED mnt chan leaves an fd slot.
//!
//! T-05-13 (the mntchk discipline, ported): EVERY R-message off the pipe
//! is untrusted input from a userspace server.  The receive path
//! bounds-checks the message size against the negotiated msize BEFORE
//! reading the body, tag-matches the reply against the outstanding
//! T-message, requires the reply type to be request+1 (Rerror maps to a
//! loud errstr with the server's ename on the serial log), and parses
//! every field through a checked cursor that must consume the message
//! EXACTLY (the donor convM2S's `l != n` check).  Protocol failures are
//! "devmnt: …" errstrs, never panics, never partial reads, never trusted
//! bytes.  Host tests prove rejection of oversized and mis-tagged replies
//! (port/tests/devmnt_test.rs).
//!
//! Concurrency this era (documented era limit): exactly ONE RPC is
//! outstanding per session — syscalls are serialized and the boot chain's
//! client side is strictly sequential (init parks in await while rc runs;
//! fork() copies the fd table, and no measured flow shares a session
//! across concurrently-running procs).  A second concurrent RPC gets a
//! LOUD error ("devmnt: concurrent rpc this era"), never a wedge — the
//! tag-multiplexing reader (the donor's mountio/mountmux) arrives with
//! the virtio-9p/SMP era.  The busy flag is a plain AtomicBool because
//! the RPC's recv BLOCKS (the pipe read schedules the server proc): an
//! MCS lock held across that block would spin-deadlock a contender
//! (IRQ-masked syscall context), so ownership is the flag, not the lock.
//!
//! Transport bound (the pipe's atomic-write ceiling): devpipe writes
//! whole messages or fail ("pipe full"), and the queue cap is 4096, so
//! per-RPC read counts clamp to `TRANSPORT_MSG_MAX - IOHDRSZ` (4072) even
//! though the negotiated msize is 8192 — the donor's iounit clamp, with
//! the era's transport as the binding constraint.  The virtio-9p era
//! lifts this.
//!
//! The dat.rs `Dev` trait is NOT implemented here (its skeleton is
//! non-constructible — see devroot.rs's header); the session methods and
//! the Chan-wrapping device ops are free functions/methods, dispatched by
//! chan.dev via namec.rs's legs (DEV_MNT).
//!
//! Fid lifecycle this era: fids are allocated at walk/attach from a
//! per-session counter (never reused — paqfs's Fid is ~50 bytes and the
//! boot chain walks dozens, not millions); Tclunk fires on the close
//! dispatch (opened chans) and on exec's explicit close.  Two r9
//! divergences from the donor, both bounded and deliberate: (1) walk
//! intermediates that never reach an fd leak their fid (the donor clunks
//! on cclose; r9's value-chan drops are invisible to devices); (2) a
//! dup'd open mnt chan SHARES the fid (the value-copy convention), so a
//! double close can Tclunk twice — benign against paqfs (its rclunk of
//! an idle fid is a no-op), wrong against a stricter server, revisited
//! with the fid-refcount era.

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_MNT, DEV_PIPE, OMASK, ORDWR, OREAD, QTDIR};
use crate::devpipe;
use crate::mcslock::BusyCell;
use crate::println;
use crate::Result;

// ---------------------------------------------------------------------------
// The wire codec (fcall.h + convS2M.c/convM2S.c, ported for the client set)
// ---------------------------------------------------------------------------

/// 9P2000 message types (fcall.h — frozen values).
const TVERSION: u8 = 100;
const RVERSION: u8 = 101;
const TATTACH: u8 = 104;
const RATTACH: u8 = 105;
const RERROR: u8 = 107;
const TWALK: u8 = 110;
const RWALK: u8 = 111;
const TOPEN: u8 = 112;
const ROPEN: u8 = 113;
const TREAD: u8 = 116;
const RREAD: u8 = 117;
const TCLUNK: u8 = 120;
const RCLUNK: u8 = 121;
const TSTAT: u8 = 124;
const RSTAT: u8 = 125;

/// The version tag (fcall.h NOTAG) and the no-auth fid (NOFID).
pub const NOTAG: u16 = 0xFFFF;
pub const NOFID: u32 = 0xFFFF_FFFF;
/// fcall.h IOHDRSZ — the Twrite/Rread header room margin.
pub const IOHDRSZ: u32 = 24;
/// fcall.h MAXWELEM.
const MAXWELEM: usize = 16;

/// The version string (fcall.h VERSION9P).
pub const VERSION9P: &[u8] = b"9P2000";

/// The msize devmnt offers when the caller passes 0 (the plan's pick —
/// paqfs's default mesgsize is 8216, so 8192 negotiates through cleanly).
pub const DEFAULT_MSIZE: u32 = 8192;

/// The transport's whole-message ceiling: devpipe's queue cap (a pipe
/// write is whole-message-or-fail, so no 9P message may exceed it).
pub const TRANSPORT_MSG_MAX: u32 = devpipe::PIPE_Q_CAP as u32;

/// The donor's msize sanity bounds (devmnt.c mntversion's
/// "nonsense value of msize" check) — applied to the OFFER too (a user
/// syscall arg sizes the kernel's receive scratch; unbounded allocation
/// from an unchecked arg is a memory-exhaustion hole).
const MSIZE_MIN: u32 = 256;
const MSIZE_MAX: u32 = 1024 * 1024;

// Little-endian wire writers (fcall.h's PBIT*).
fn pw16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pw32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pw64(v: &mut Vec<u8>, x: u64) {
    v.extend_from_slice(&x.to_le_bytes());
}
/// convS2M's pstring: u16 length prefix + the bytes.
fn pwstr(v: &mut Vec<u8>, s: &[u8]) {
    pw16(v, s.len() as u16);
    v.extend_from_slice(s);
}
/// Wrap a body in the 9P header: size[4] type[1] tag[2] + body.
fn tmsg(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(7 + body.len());
    pw32(&mut v, (7 + body.len()) as u32);
    v.push(typ);
    pw16(&mut v, tag);
    v.extend_from_slice(body);
    v
}

/// A checked little-endian cursor for parsing R-messages (the mini
/// convM2S): every field read is bounds-checked, and `done` requires the
/// message to be consumed EXACTLY (the donor's `l != n` check) — trailing
/// garbage is a protocol error.
struct RepCur<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> RepCur<'a> {
    fn new(b: &'a [u8]) -> RepCur<'a> {
        RepCur { b, p: 0 }
    }
    fn u8(&mut self) -> Result<u8> {
        let Some(&v) = self.b.get(self.p) else { return Err("devmnt: malformed reply (truncated)"); };
        self.p += 1;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16> {
        let Some(s) = self.b.get(self.p..self.p + 2) else {
            return Err("devmnt: malformed reply (truncated)");
        };
        self.p += 2;
        Ok(u16::from_le_bytes(s.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        let Some(s) = self.b.get(self.p..self.p + 4) else {
            return Err("devmnt: malformed reply (truncated)");
        };
        self.p += 4;
        Ok(u32::from_le_bytes(s.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        let Some(s) = self.b.get(self.p..self.p + 8) else {
            return Err("devmnt: malformed reply (truncated)");
        };
        self.p += 8;
        Ok(u64::from_le_bytes(s.try_into().unwrap()))
    }
    /// convM2S's gqid: type[1] vers[4] path[8].
    fn qid(&mut self) -> Result<Qid> {
        let typ = self.u8()?;
        let vers = self.u32()?;
        let path = self.u64()?;
        Ok(Qid { path, vers, typ })
    }
    /// convM2S's gstring: u16 length prefix + the bytes (bounds-checked —
    /// a count that overruns the message is malformed, never a panic).
    fn string(&mut self) -> Result<Vec<u8>> {
        let n = self.u16()? as usize;
        Ok(self.bytes(n)?.to_vec())
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let Some(s) = self.b.get(self.p..self.p + n) else {
            return Err("devmnt: malformed reply (count overruns the message)");
        };
        self.p += n;
        Ok(s)
    }
    /// The whole message must be consumed (convM2S's exact-size check).
    fn done(self) -> Result<()> {
        if self.p != self.b.len() {
            return Err("devmnt: malformed reply (trailing garbage)");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The transport (the pipe fd's chan in the kernel; a scripted fake server
// in host tests)
// ---------------------------------------------------------------------------

/// The 9P byte transport: one whole message per write; reads return up to
/// the buffer's length and BLOCK until at least one byte is available
/// (the kernel transport's devpipe read schedules the server proc).
/// 261103-p64: the Send supertrait attempt is documented in UNSAFE-AUDIT.md
/// — it breaks the unmodified host tests (FakeServer's Rc fid table), so
/// the session's shareability proof is the BusyCell discipline and the
/// `unsafe impl Send for MntCore` lives beside the primitive (mcslock.rs).
pub trait MntIo {
    /// Write one whole message, atomically (devpipe's contract: the full
    /// buffer lands or the write fails — never a partial message).
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64>;
    /// Read available bytes (blocks on empty); 0 = EOF (the peer hung up).
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64>;
}

/// The kernel transport: a 9P-capable chan (the pipe end).  Reads/writes
/// ride devpipe's ops directly (streaming — offsets don't apply to pipe
/// ends); anything else is a loud contract violation (sys_fversion/sys_mount
/// validate the fd before constructing this).
pub struct ChanIo {
    pub chan: Chan,
}

impl MntIo for ChanIo {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        match self.chan.dev {
            DEV_PIPE => devpipe::write(&mut self.chan, buf, u64::MAX),
            _ => Err("devmnt: fd is not a 9P-capable connection"),
        }
    }
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        match self.chan.dev {
            DEV_PIPE => devpipe::read(&mut self.chan, buf, u64::MAX),
            _ => Err("devmnt: fd is not a 9P-capable connection"),
        }
    }
}

// ---------------------------------------------------------------------------
// The session
// ---------------------------------------------------------------------------

/// The RPC-critical-section state: only the busy-flag OWNER touches it
/// (the flag IS the mutex — see the module header's concurrency note).
/// 261103-p64: pub(crate) so the Send proof can live beside BusyCell
/// (mcslock.rs) — devmnt.rs itself carries zero unsafe for the
/// #[forbid(unsafe_code)] regression lock.
pub(crate) struct MntCore {
    io: Box<dyn MntIo>,
    /// Fid/tag sequences (donor mntchan/alloctag): fids from 1 up (never
    /// NOFID, never reused this era); tags from 1 up (never 0, never
    /// NOTAG — devmnt.c's mntreset exclusions).
    next_fid: u32,
    next_tag: u16,
    /// The msize-sized receive scratch (HEAP — an 8 KiB stack buffer is
    /// the 05-06 kstack-overflow lesson).
    rx: Vec<u8>,
}

/// A 9P client session (the donor's `Mnt`): one connection's negotiated
/// state plus the fid/tag allocators.  Shared by every chan in the
/// mounted tree (and by the pipe chan that fversion created it over) as
/// `Arc<Mnt>`; the busy flag serializes RPCs (one outstanding — the era's
/// documented limit).
pub struct Mnt {
    /// The RPC-mutable session state (io, fid/tag sequences, rx scratch).
    core: BusyCell<MntCore>,
    /// The negotiated msize (the reply's answer, ≤ the offer) — set once
    /// at negotiation, frozen after (plain shared reads, no guard).
    msize: u32,
    /// The negotiated version string (answered on re-version, the donor's
    /// c->mux replay in mntversion) — set-once, frozen.
    version: Vec<u8>,
}

impl core::fmt::Debug for Mnt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Mnt(9p session)")
    }
}

impl Mnt {
    /// Take RPC ownership (the era's serialization): one outstanding RPC
    /// per session; a concurrent attempt is a LOUD error, never a wedge.
    /// 261103-p64: the BusyCell guard IS the token — its Drop releases
    /// the session on any exit path.
    fn enter(&self) -> Result<crate::mcslock::BusyGuard<'_, MntCore>> {
        self.core.enter().ok_or("devmnt: concurrent rpc this era")
    }

    /// The whole RPC round-trip (donor mountrpc + the receive half of
    /// mountio/mntrpcread): send the T-message whole, then frame the
    /// reply — read the 4-byte size prefix, mntchk it against the
    /// negotiated msize BEFORE reading the body (T-05-13), read exactly
    /// `size` bytes, then require the expected type (Rerror maps to a
    /// loud errstr with the server's ename on the serial log) and the
    /// outstanding tag.  Returns the reply's total size (the body sits in
    /// the session's rx scratch).
    /// The RPC round-trip.  `core` rides the live BusyCell guard — holding
    /// it IS the ownership proof (only enter() hands it out).
    fn transact(&self, core: &mut MntCore, msg: &[u8], expect_type: u8, expect_tag: u16) -> Result<usize> {
        let w = core.io.write_msg(msg)?;
        if w as usize != msg.len() {
            return Err("devmnt: short write on the 9P connection");
        }
        // Frame the reply: the 4-byte size prefix first.
        let mut got = 0usize;
        while got < 4 {
            let n = core.io.read_some(&mut core.rx[got..4])? as usize;
            if n == 0 {
                return Err("devmnt: eof waiting for a reply");
            }
            got += n;
        }
        let size = u32::from_le_bytes(core.rx[0..4].try_into().unwrap()) as usize;
        // T-05-13 (mntchk): the claimed size is untrusted until it passes
        // BOTH bounds — never read (or trust) a body the claim doesn't
        // cover, never read more than the negotiated msize.
        if size < 7 {
            println!("devmnt: runt reply message ({size} bytes)");
            return Err("devmnt: runt reply message");
        }
        if size > self.msize as usize {
            println!("devmnt: reply claims {size} bytes over msize {}", self.msize);
            return Err("devmnt: reply exceeds negotiated msize");
        }
        while got < size {
            let n = core.io.read_some(&mut core.rx[got..size])? as usize;
            if n == 0 {
                return Err("devmnt: eof mid-message");
            }
            got += n;
        }
        let rtype = core.rx[4];
        let rtag = u16::from_le_bytes(core.rx[5..7].try_into().unwrap());
        if rtype == RERROR {
            let mut cur = RepCur::new(&core.rx[7..size]);
            let ename = cur.string().unwrap_or_else(|_| b"<malformed Rerror>".to_vec());
            println!("devmnt: server error: {}", alloc::string::String::from_utf8_lossy(&ename));
            return Err("devmnt: server error");
        }
        if rtag != expect_tag {
            println!("devmnt: reply tag {rtag} is not the outstanding tag {expect_tag}");
            return Err("devmnt: reply tag mismatch");
        }
        if rtype != expect_type {
            println!("devmnt: reply type {rtype} is not the expected {expect_type}");
            return Err("devmnt: unexpected reply type");
        }
        Ok(size)
    }

    fn alloc_fid(core: &mut MntCore) -> Result<u32> {
        let fid = core.next_fid;
        if fid == NOFID {
            return Err("devmnt: fid space exhausted");
        }
        core.next_fid += 1;
        Ok(fid)
    }

    fn alloc_tag(core: &mut MntCore) -> u16 {
        let tag = core.next_tag;
        // NOTAG stays reserved; 0 is never issued (devmnt.c's mntreset).
        core.next_tag = if tag == NOTAG - 1 { 1 } else { tag + 1 };
        tag
    }

    /// The Tversion exchange (donor mntversion): offer `offer` (0 →
    /// DEFAULT_MSIZE), send Tversion(NOTAG, msize, "9P2000"), validate the
    /// Rversion (the server never increases the msize — devmnt.c's "server
    /// tries to increase msize"; nonsense bounds 256..=1MiB; the version
    /// string answers "9P2000"), and park the negotiated session.
    pub fn negotiate(io: Box<dyn MntIo>, offer: u32) -> Result<Mnt> {
        let offer = if offer == 0 { DEFAULT_MSIZE } else { offer };
        if !(MSIZE_MIN..=MSIZE_MAX).contains(&offer) {
            return Err("devmnt: nonsense msize in fversion");
        }
        let mut mnt = Mnt {
            core: BusyCell::new(MntCore {
                io,
                next_fid: 1,
                next_tag: 1,
                rx: alloc::vec![0u8; offer as usize],
            }),
            msize: offer, // the handshake's bound is the offer itself
            version: Vec::new(),
        };
        let mut body = Vec::new();
        pw32(&mut body, offer);
        pwstr(&mut body, VERSION9P);
        let msg = tmsg(TVERSION, NOTAG, &body);
        // The guard is held across the reply parse (the RPC's ownership
        // ends when the parsed values are committed, not when transact
        // returns).  The Mnt is a not-yet-shared local, so enter() can
        // never contend.
        let (msize, version) = {
            let mut core = mnt.enter()?;
            let size = mnt.transact(&mut core, &msg, RVERSION, NOTAG)?;
            let mut cur = RepCur::new(&core.rx[7..size]);
            let msize = cur.u32()?;
            let version = cur.string()?;
            cur.done()?;
            (msize, version)
        };
        if msize > offer {
            return Err("devmnt: server tried to increase msize in fversion");
        }
        if !(MSIZE_MIN..=MSIZE_MAX).contains(&msize) {
            return Err("devmnt: nonsense msize in fversion");
        }
        if version != VERSION9P {
            return Err("devmnt: bad 9P version returned from server");
        }
        mnt.msize = msize;
        mnt.version = version;
        Ok(mnt)
    }

    /// The negotiated msize (a frozen set-once field — no guard).
    pub fn msize(&self) -> u32 {
        self.msize
    }

    /// The negotiated version string (the re-version replay's answer) —
    /// a frozen set-once field (no guard).
    fn version(&self) -> Vec<u8> {
        self.version.clone()
    }

    /// Tattach (donor mntattach): a fresh fid attached to the tree root
    /// with afid = NOFID (D-02 — no auth this phase; nsinit's fall-through
    /// is the designed path).  Returns (fid, the root qid).
    pub fn attach(&self, uname: &[u8], aname: &[u8]) -> Result<(u32, Qid)> {
        let mut core = self.enter()?;
        let fid = Self::alloc_fid(&mut core)?;
        let tag = Self::alloc_tag(&mut core);
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw32(&mut body, NOFID);
        pwstr(&mut body, uname);
        pwstr(&mut body, aname);
        let msg = tmsg(TATTACH, tag, &body);
        let size = self.transact(&mut core, &msg, RATTACH, tag)?;
        let mut cur = RepCur::new(&core.rx[7..size]);
        let qid = cur.qid()?;
        cur.done()?;
        Ok((fid, qid))
    }

    /// Twalk one component from `fid` to a fresh fid (donor mntwalk,
    /// single-name shape — namec walks one component per call).  Returns
    /// (newfid, the walked qid); a zero-qid Rwalk or the server's Rerror
    /// is the loud not-found.
    pub fn walk(&self, fid: u32, name: &[u8]) -> Result<(u32, Qid)> {
        if name.is_empty() || name.contains(&b'/') {
            return Err("devmnt: walk takes a single name component");
        }
        let mut core = self.enter()?;
        let newfid = Self::alloc_fid(&mut core)?;
        let tag = Self::alloc_tag(&mut core);
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw32(&mut body, newfid);
        pw16(&mut body, 1); // nwname
        pwstr(&mut body, name);
        let msg = tmsg(TWALK, tag, &body);
        let size = self.transact(&mut core, &msg, RWALK, tag)?;
        let mut cur = RepCur::new(&core.rx[7..size]);
        let nwqid = cur.u16()? as usize;
        // The donor's bounds: nwqid never exceeds MAXWELEM nor nwname (1).
        if nwqid > MAXWELEM || nwqid > 1 {
            return Err("devmnt: too many qids returned by walk");
        }
        if nwqid == 0 {
            return Err("devmnt: name not found");
        }
        let qid = cur.qid()?;
        cur.done()?;
        Ok((newfid, qid))
    }

    /// A fid CLONE (05-09 — the gate-surfaced mount-table aliasing fix):
    /// Twalk with ZERO names (the 9P fid-clone idiom — donor mntwalk's
    /// nwname==0 arm) clones the fid's position onto a fresh fid.  The
    /// mount table's entry chans are SHARED with namec's redirects; a
    /// bare dup shares the server-side fid, so an open+close of the
    /// redirect (ls /bin's listing) Tclunked the TABLE's fid and every
    /// later walk from it Rerror'd (paqfs: a clunked fid's busy==0 →
    /// Enotexist).  The redirects now clone the fid, so the open/clunk
    /// lifecycle rides the clone and the table's fid stays pristine.
    pub fn clone_fid(&self, fid: u32) -> Result<u32> {
        let mut core = self.enter()?;
        let newfid = Self::alloc_fid(&mut core)?;
        let tag = Self::alloc_tag(&mut core);
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw32(&mut body, newfid);
        pw16(&mut body, 0); // nwname = 0 — the fid-clone idiom
        let msg = tmsg(TWALK, tag, &body);
        let size = self.transact(&mut core, &msg, RWALK, tag)?;
        let mut cur = RepCur::new(&core.rx[7..size]);
        let nwqid = cur.u16()? as usize;
        // A zero-name walk answers nwqid = 0; the clone carries the
        // source's qid (the position is unchanged by construction).
        if nwqid != 0 {
            return Err("devmnt: fid clone returned qids");
        }
        Ok(newfid)
    }

    /// Topen (donor mntopen): the mode byte is Plan 9's open-mode byte
    /// (the low 8 bits — OEXCL doesn't fit and is create-only, which this
    /// era never sends).  Returns (the opened qid, the iounit after the
    /// donor's clamp).
    pub fn open(&self, fid: u32, mode: u8) -> Result<(Qid, u32)> {
        let mut core = self.enter()?;
        let tag = Self::alloc_tag(&mut core);
        let mut body = Vec::new();
        pw32(&mut body, fid);
        body.push(mode);
        let msg = tmsg(TOPEN, tag, &body);
        let size = self.transact(&mut core, &msg, ROPEN, tag)?;
        let mut cur = RepCur::new(&core.rx[7..size]);
        let qid = cur.qid()?;
        let iounit = cur.u32()?;
        cur.done()?;
        // The donor's clamp (mntopencreate): 0 or oversized → msize-IOHDRSZ.
        let cap = self.msize - IOHDRSZ;
        let iounit = if iounit == 0 || iounit > cap { cap } else { iounit };
        Ok((qid, iounit))
    }

    /// One Tread RPC (donor mntrdwr's inner loop): the CHAN-level read
    /// clamps count to the era's per-RPC bound; the reply's count must
    /// exactly cover the message body (convM2S's exact-size check) and is
    /// clamped to the request (the donor's silent clamp).
    pub fn read(&self, fid: u32, off: u64, count: u32) -> Result<Vec<u8>> {
        let mut core = self.enter()?;
        let tag = Self::alloc_tag(&mut core);
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw64(&mut body, off);
        pw32(&mut body, count);
        let msg = tmsg(TREAD, tag, &body);
        let size = self.transact(&mut core, &msg, RREAD, tag)?;
        let mut cur = RepCur::new(&core.rx[7..size]);
        let rcount = cur.u32()? as usize;
        let remaining = cur.b.len() - cur.p;
        if rcount != remaining {
            return Err("devmnt: malformed read reply (count/body mismatch)");
        }
        let keep = rcount.min(count as usize);
        let data = cur.bytes(rcount)?.to_vec();
        cur.done()?;
        Ok(data[..keep].to_vec())
    }

    /// Tstat (donor mntstat): the reply's stat payload (nstat bytes — the
    /// stat(5) image WITH its own 2-byte prefix, whose value must be
    /// nstat-2 — the malformation check).
    pub fn stat(&self, fid: u32) -> Result<Vec<u8>> {
        let mut core = self.enter()?;
        let tag = Self::alloc_tag(&mut core);
        let mut body = Vec::new();
        pw32(&mut body, fid);
        let msg = tmsg(TSTAT, tag, &body);
        let size = self.transact(&mut core, &msg, RSTAT, tag)?;
        let mut cur = RepCur::new(&core.rx[7..size]);
        let nstat = cur.u16()? as usize;
        if nstat < 2 {
            return Err("devmnt: malformed stat reply");
        }
        let payload = cur.bytes(nstat)?;
        // The payload's own prefix must count its remainder (stat(5)
        // self-consistency — a lying server never gets trusted bytes).
        let inner = u16::from_le_bytes(payload[0..2].try_into().unwrap()) as usize;
        if inner != nstat - 2 {
            return Err("devmnt: malformed stat reply (bad inner prefix)");
        }
        let out = payload.to_vec();
        cur.done()?;
        Ok(out)
    }

    /// Tclunk (donor mntclunk): release the fid server-side.
    pub fn clunk(&self, fid: u32) -> Result<()> {
        let mut core = self.enter()?;
        let tag = Self::alloc_tag(&mut core);
        let mut body = Vec::new();
        pw32(&mut body, fid);
        let msg = tmsg(TCLUNK, tag, &body);
        let size = self.transact(&mut core, &msg, RCLUNK, tag)?;
        RepCur::new(&core.rx[7..size]).done()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The Chan-wrapping device ops (namec.rs's DEV_MNT legs + the syscalls)
// ---------------------------------------------------------------------------

/// The era's per-RPC read bound: min(msize, the transport's whole-message
/// ceiling) minus the header margin.
fn rpc_data_cap(msize: u32) -> usize {
    (msize.min(TRANSPORT_MSG_MAX) - IOHDRSZ) as usize
}

/// sys_fversion's helper (donor sysfversion): negotiate over the fd's chan
/// and park the session on it.  Idempotent on an established session (the
/// donor's c->mux replay: answers from the stored negotiation).  `out` is
/// the caller's version buffer (EMPTY = no write-back, the donor's
/// returnlen==0); returns the version string's length.
pub fn fversion(chan: &mut Chan, offer: u32, out: &mut [u8]) -> Result<u64> {
    if let Some(mnt) = &chan.mnt {
        // The donor's c->mux replay: an established session answers from
        // the stored negotiation (and refuses a version CHANGE — the
        // donor's "incompatible 9P versions" — which the era's sole
        // version makes unreachable).
        let version = mnt.version();
        if !out.is_empty() {
            if out.len() < version.len() {
                return Err("fversion: version buffer too small");
            }
            out[..version.len()].copy_from_slice(&version);
        }
        return Ok(version.len() as u64);
    }
    if chan.dev != DEV_PIPE {
        return Err("fversion: fd is not a 9P-capable connection");
    }
    if chan.mode & OMASK != ORDWR {
        return Err("fversion: fd is not open for read+write");
    }
    // The session's io chan: a dup (an open pipe end's count bumps — the
    // session holds a live reference even after the fd closes; the mount
    // must outlive it — boot's fd closes at init's closefds).
    let io = ChanIo { chan: chan.dup() };
    let mnt = Mnt::negotiate(Box::new(io), offer)?;
    let version = mnt.version();
    if !out.is_empty() {
        if out.len() < version.len() {
            return Err("fversion: version buffer too small");
        }
        out[..version.len()].copy_from_slice(&version);
    }
    let len = version.len() as u64;
    chan.mnt = Some(Arc::new(mnt));
    Ok(len)
}

/// sys_mount's attach (donor mntattach via bindmount): attach the session's
/// tree root and return the mount-root chan (DEV_MNT, fresh fid, the
/// server's root qid).  The caller sets the path (the mount target's) and
/// inserts it into the pgrp mount table.
pub fn attach(mnt: &Arc<Mnt>, aname: &[u8]) -> Result<Chan> {
    let (fid, qid) = mnt.attach(b"glenda", aname)?;
    let mut c = Chan::new(DEV_MNT, qid, 0);
    c.mnt = Some(mnt.clone());
    c.mnt_fid = fid;
    Ok(c)
}

/// The DEV_MNT walk leg (namec's dev_walk): Twalk to a fresh fid.
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: walk of a non-mount chan") };
    if c.mnt_fid == 0 {
        return Err("devmnt: walk of an fid-less chan");
    }
    if c.qid.typ & QTDIR == 0 {
        return Err("devmnt: walk of a non-directory");
    }
    let (newfid, qid) = mnt.walk(c.mnt_fid, name)?;
    let mut nc = Chan::new(DEV_MNT, qid, 0);
    nc.mnt = Some(mnt.clone());
    nc.mnt_fid = newfid;
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// The DEV_MNT fid-clone leg (05-09): namec's mount-point redirects clone
/// the mount TABLE's chan onto a fresh fid (Mnt::clone_fid — the zero-name
/// Twalk) instead of bare-dup'ing it, so a redirect's open+close lifecycle
/// never touches the table's server-side fid (the ls /bin → ps exec gate
/// failure: the bare dup let ls's close Tclunk the table's fid, and every
/// later walk from it Rerror'd).
pub fn clone_chan(c: &Chan) -> Result<Chan> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: clone of a non-mount chan") };
    if c.mnt_fid == 0 {
        return Err("devmnt: clone of an fid-less chan");
    }
    let newfid = mnt.clone_fid(c.mnt_fid)?;
    let mut nc = Chan::new(DEV_MNT, c.qid, c.mode);
    nc.mnt = Some(mnt.clone());
    nc.mnt_fid = newfid;
    nc.path = c.path.clone();
    Ok(nc)
}

/// The DEV_MNT open leg (namec's open_chan): Topen; directory opens are
/// OREAD-only (the donor namec discipline); OTRUNC/ORCLOSE ride the wire
/// byte for the server to answer (paqfs refuses both — Erdonly).
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: open of a non-mount chan") };
    if c.mnt_fid == 0 {
        return Err("devmnt: open of an fid-less chan");
    }
    let access = mode & OMASK;
    if c.qid.typ & QTDIR != 0 && access != OREAD {
        return Err("devmnt: directory opens OREAD only");
    }
    let (qid, _iounit) = mnt.open(c.mnt_fid, (mode & 0xff) as u8)?;
    let mut nc = Chan::new(DEV_MNT, qid, access);
    nc.mnt = Some(mnt.clone());
    nc.mnt_fid = c.mnt_fid;
    nc.path = c.path.clone();
    Ok(nc)
}

/// The DEV_MNT read leg (sys_pread): chunked Treads (donor mntrdwr — stop
/// on a short reply or a full buffer); u64::MAX offset uses + advances the
/// chan's streaming offset (the pread convention).
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: read of a non-mount chan") };
    if c.mnt_fid == 0 {
        return Err("devmnt: read of an fid-less chan");
    }
    let cap = rpc_data_cap(mnt.msize());
    let mut off = if offset == u64::MAX { c.offset } else { offset };
    let mut done = 0usize;
    while done < buf.len() {
        let want = (buf.len() - done).min(cap);
        let data = mnt.read(c.mnt_fid, off, want as u32)?;
        let n = data.len();
        buf[done..done + n].copy_from_slice(&data);
        off += n as u64;
        done += n;
        if n < want {
            break; // a short reply = EOF (the donor mntrdwr break)
        }
    }
    if offset == u64::MAX {
        c.offset += done as u64;
    }
    Ok(done as u64)
}

/// The DEV_MNT stat leg (sys_stat/sys_fstat): Tstat, the donor mntstat's
/// truncation shape (a tight buffer gets just the count prefix).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: stat of a non-mount chan") };
    if c.mnt_fid == 0 {
        return Err("devmnt: stat of an fid-less chan");
    }
    let payload = mnt.stat(c.mnt_fid)?;
    if payload.len() > sb.len() {
        // The donor mntstat truncation: the count prefix alone.
        if sb.len() < 2 {
            return Err("devmnt: stat buffer too small");
        }
        let n = (payload.len() - 2) as u16;
        sb[0..2].copy_from_slice(&n.to_le_bytes());
        return Ok(2);
    }
    sb[..payload.len()].copy_from_slice(&payload);
    Ok(payload.len() as u64)
}

/// The DEV_MNT close dispatch (namec::close_chan): Tclunk, best-effort —
/// a failure logs and swallows (the donor's cclose never fails).
pub fn clunk(c: &Chan) {
    let Some(mnt) = &c.mnt else { return };
    if c.mnt_fid == 0 {
        return;
    }
    if let Err(msg) = mnt.clunk(c.mnt_fid) {
        println!("devmnt: clunk of fid {} failed: {msg}", c.mnt_fid);
    }
}

/// sys_exec's byte source for a mounted binary (05-07 Task 2's wiring):
/// read the WHOLE file eagerly (syscall context — RPCs block by design;
/// the trap-context pager must never RPC).  Bounded by `cap` (loud error,
/// never an unbounded read off an untrusted server).  WR-06(b): reads may
/// reach cap+1 bytes so an EXACTLY-cap file terminates via the empty-read
/// EOF arm (the old top-of-loop `out.len() >= cap` check errored it
/// without ever attempting the final read); only a byte past the cap is
/// "too large".
pub fn read_all(c: &Chan, cap: usize) -> Result<Vec<u8>> {
    let Some(mnt) = &c.mnt else { return Err("devmnt: read of a non-mount chan") };
    if c.mnt_fid == 0 {
        return Err("devmnt: read of an fid-less chan");
    }
    let rpc_cap = rpc_data_cap(mnt.msize());
    let mut out = Vec::new();
    loop {
        // want is always ≥ 1 here: out.len() ≤ cap (a larger length
        // errored below), so cap + 1 - out.len() ≥ 1.
        let want = rpc_cap.min(cap + 1 - out.len());
        let data = mnt.read(c.mnt_fid, out.len() as u64, want as u32)?;
        if data.is_empty() {
            break; // EOF — the file fits within the cap
        }
        out.extend_from_slice(&data);
        if out.len() > cap {
            return Err("devmnt: file too large to stage for exec");
        }
        if data.len() < want {
            break; // a short reply = EOF
        }
    }
    Ok(out)
}
