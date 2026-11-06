//! devether — the ethernet device `#l` (NETDRV-03): `/net/ether0` per
//! netif.h over the 07-06 virtio-net driver.  Semantics ported from
//! 9/pc's `devether.c` + 9k's `netif.c` (the donors); the Netif/Netfile
//! model, the IRQ↔syscall handoff, and the EtherHooks live in
//! `netif.rs` (this file is the DEVICE surface: attach/walk/open/stat/
//! read/write/close as free functions, dispatched by chan.dev via
//! namec.rs's legs + syscall.rs's pread/pwrite arms — the established
//! port idiom; the dat.rs Dev trait is non-constructible, see
//! devroot.rs's header).
//!
//! The three-level tree (netif.c's netifgen): `#l` root → `ether0/`
//! (clone 0666, addr 0666, stats 0444, ifstats 0444, plus one numbered
//! dir per allocated conversation) → `N/` (data + ctl at the owner
//! perms — 0666 until the first open, 0660 after, the single-user
//! netown collapse with byte-identical stat output — stats 0444, type
//! 0444, ifstats 0444).  stat(5) records carry FOUR strings
//! (name/uid/gid/muid — libc's statcheck rejects short records).
//!
//! Clone/open (netif.c:158-197): opening `clone` allocates the first
//! free conversation id and rewrites the chan's qid to NETQID(id,
//! Nctlqid) — the returned file IS the new conversation's ctl; reading
//! ctl returns the id in decimal (readnum's 12-byte fixed shape).
//! Opening N/data or N/ctl re-opens that id (inuse++); dirs open
//! OREAD-only; addr/stats/type/ifstats OREAD-only.  The last close of a
//! data/ctl chan resets the conversation (type 0, owner cleared, queue
//! drained — netifclose verbatim in intent).
//!
//! The data-file write path is netif-faithful (devether.c:253-299):
//! writes only on the data qid; n > 1514 → "write too big" (Etoobig);
//! n < 60 → "write too small" (Etoosmall — ETHERMINTU; the 07-04
//! fixture's 60 bytes is exactly this bound); the kernel overwrites the
//! frame's source MAC with the NIC's negotiated address; the two-
//! descriptor TX chain (the zeroed 12-byte header page + the payload)
//! posts through the 07-03 machinery.  When the ring has no free chain
//! the write BLOCKS on the wake hook until the IRQ retire frees one —
//! a stalled retire wedges the writer → the gate's timeout, never a
//! silent drop (D-08's wedge-hunt mechanism).
//!
//! THE CRITICAL-SECTION NOTE (the plan's "DAIF-masked window"): the
//! kernel's Phase-4 invariant (PSTATE.I=0 ⟺ EL0 or the sched idle wfi)
//! means syscall context — where the TX post runs — is ALWAYS
//! IRQ-masked on this kernel, so an RX IRQ can never interleave with
//! the post's pool-alloc + avail-index window.  The guarantee the plan
//! asks for is realized by the invariant itself; no new asm/DAIF
//! sequence is needed (and none could exist here — this module is
//! unsafe-forbidden by attribute).
//!
//! The data-file read blocks per netif's qread semantics (Qmsg — one
//! WHOLE frame per read, the 14-byte-header Ethernet frame, the 12-byte
//! virtio header already stripped by the IRQ retire) via the devpipe-
//! shaped EtherHooks { current, block, wake }.  RESEARCH OQ-1's
//! disposition: NO Phase-7 gate ever blocks on a data read — the ARP
//! gate's RX proof is the stats counter + the host pcap — so this read
//! path is complete-and-correct but gate-unexercised (smoke-level
//! only).  That omission is recorded here per the plan's loud rule.
//!
//! TX loopback/broadcast feedback into the input path (the donor's
//! etheroq → etheriq call) is NOT implemented this phase (nothing in
//! the ARP gate needs it) — Phase 8's arp/ethermedium EXPECTS it; the
//! omission is deliberate (07-07 must_haves, netif.rs's header).
//!
//! ctl depth (the CONTEXT's discretion, locked): `connect N` functional
//! (the ether-type filter; the sweep's `echo connect 2054` pins ARP);
//! `promiscuous` accepted as a no-op toward the hardware (qemu's RX
//! filter without CTRL_VQ delivers our MAC + broadcast regardless);
//! `nonblocking` LOUD-UNIMPLEMENTED (there is no output queue this era
//! — writes are synchronous posts, so the verb would be a lie);
//! addmulti/remmulti/scanbs/bridge/headersonly loud-unimplemented;
//! unknown verbs error loudly ("devether: bad ctl" — the repo posture).
//!
//! The `ifstats` file routes to the driver hook and prints the
//! per-queue accounting line in ethervirtio.c:439-468's shape —
//! `vq<q>: avail=<a> used=<u> lastused=<l> nintr=<x> nnote=<m>`
//! (comma-free for trivial host-side parsing; the devfeat/drvfeat
//! header lines of the donor are the legacy I/O-port register reads,
//! meaningless on the modern path — omitted, the vq lines are THE
//! NETDRV-04 reconciliation surface).

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_ETHER, OREAD};
use crate::mcslock::{Lock, LockNode};
use crate::netif::{self, NFILE, NUMSIZE};
use crate::Result;

// Re-exported so the boot registers the hooks through the device module
// like its siblings (port::devpipe::set_hooks et al.), and so the IRQ
// handler's port-side entries name the DEVICE (the plan's key-link:
// trap.rs → devether.rs — the IRQ retire's demux/wake entries live in
// netif.rs with the model and ride through here).
pub use crate::netif::{EtherHooks, rx_ingress, set_hooks, tx_retired};

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

// ---------------------------------------------------------------------------
// The driver seam: the arch-side bind (pci.rs, 07-06) publishes the
// negotiated device here; attach errors Enodev-shaped until it does.
// ---------------------------------------------------------------------------

/// The arch-side driver operations (registered once by the 07-06 bind).
/// Every op runs in SYSCALL context (IRQ-masked per the kernel
/// invariant — see the module header's critical-section note), touching
/// the arch's publish-once device state.
#[derive(Clone, Copy)]
pub struct EtherDriverOps {
    /// The negotiated MAC (the addr file, the stats `addr:` line, and
    /// the write path's source-MAC overwrite).
    pub mac: [u8; 6],
    /// Post one whole frame (60..=1514 bytes, source MAC already
    /// overwritten) to the TX ring: Ok(true) posted; Ok(false) ring
    /// full — the caller registers a TX waiter and blocks on the wake
    /// hook until the IRQ retire frees a slot.
    pub tx_post: fn(&[u8]) -> Result<bool>,
    /// The per-queue accounting snapshot for the ifstats file, one
    /// tuple per queue (vq0 = RX, vq1 = TX): (avail, used, lastused,
    /// nintr, nnote) — ethervirtio.c's ifstat fields.
    pub ifstats: fn() -> [(u16, u16, u16, u32, u32); 2],
}

static DRIVER: Lock<Option<EtherDriverOps>> = Lock::new("devetherdrv", None);

/// The 07-06 bind's publication call: the device is live (DRIVER_OK,
/// rings posted, IRQ armed) — create the Netif and accept attaches.
/// Boot context, once; a second publication is a boot bug.
pub fn publish_driver(ops: EtherDriverOps) {
    netif::publish(ops.mac);
    let node = LockNode::new();
    let mut guard = DRIVER.lock(&node);
    assert!(guard.is_none(), "devether: driver published twice");
    *guard = Some(ops);
}

fn driver() -> Result<EtherDriverOps> {
    let node = LockNode::new();
    DRIVER.lock(&node).ok_or("devether: no ether0")
}

/// The published driver ops for the kernel-internal ether consumer (the
/// devip ether adapter, 08-06): the negotiated MAC + tx_post.  Errors
/// "devether: no ether0" until the bind publishes.
pub(crate) fn driver_ops() -> Result<EtherDriverOps> {
    driver()
}

// ---------------------------------------------------------------------------
// attach + the file tree
// ---------------------------------------------------------------------------

/// attach(spec): the controller-number parse (devether.c's etherattach)
/// — empty spec defaults to unit 0; any other unit or a driver that
/// never published errors Enodev-shaped.
pub fn attach(spec: &[u8]) -> Result<Chan> {
    let unit = parse_spec(spec)?;
    if unit != 0 {
        return Err("devether: no such interface");
    }
    driver()?; // "devether: no ether0" when the bind never published
    let mut c = Chan::new(DEV_ETHER, Qid { path: 0, vers: 0, typ: QTDIR }, 0);
    c.path = b"#l".to_vec();
    Ok(c)
}

/// The controller-number parse (the donor's strtoul shape): empty = 0;
/// a decimal number otherwise; trailing garbage or a non-number is
/// Ebadarg-shaped.
fn parse_spec(spec: &[u8]) -> Result<u64> {
    if spec.is_empty() {
        return Ok(0);
    }
    let mut v: u64 = 0;
    for &b in spec {
        if !b.is_ascii_digit() {
            return Err("devether: bad attach spec");
        }
        v = v.checked_mul(10).and_then(|v| v.checked_add(u64::from(b - b'0'))).ok_or("devether: bad attach spec")?;
    }
    Ok(v)
}

/// The static per-file metadata for the fixed files (the devcons FILES
/// precedent): level 2 (under ether0/) and level 3 (under a
/// conversation dir) share it via the qid helpers.  The data/ctl modes
/// shown here are the PRE-OPEN surface (0666); the owner-collapse mode
/// (0660) is computed at listing/stat time.
const LEVEL2_FILES: [(&[u8], u64, u32); 4] = [
    (b"clone", netif::NCLONEQID, 0o666),
    (b"addr", netif::NADDRQID, 0o666),
    (b"stats", netif::NSTATQID, 0o444),
    (b"ifstats", netif::NIFSTATQID, 0o444),
];

const LEVEL3_FILES: [(&[u8], u64); 5] = [
    (b"data", netif::NDATAQID),
    (b"ctl", netif::NCTLQID),
    (b"stats", netif::NSTATQID),
    (b"type", netif::NTYPEQID),
    (b"ifstats", netif::NIFSTATQID),
];

/// Walk a single name component (netifgen's child lookup).  Numeric
/// names at level 2 resolve to allocated conversation dirs.
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR {
        return Err("devether: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devether: walk takes a single name component");
    }
    let (qid, typ) = match c.qid.path {
        0 => {
            // The root contains only the interface dir (the model's name).
            if name != netif::ifname()? {
                return Err("devether: name not found");
            }
            (netif::N2NDQID, QTDIR)
        }
        p if netif::nettype(p) == netif::N2NDQID => {
            if let Some((_, q, _)) = LEVEL2_FILES.iter().find(|(n, _, _)| *n == name) {
                (*q, QTFILE)
            } else if let Some(id) = parse_decimal(name) {
                let alloc = netif::conv_allocated();
                if id >= NFILE || !alloc[id] {
                    return Err("devether: name not found");
                }
                (netif::netqid(id as u64, netif::N3RDQID), QTDIR)
            } else {
                return Err("devether: name not found");
            }
        }
        p if netif::nettype(p) == netif::N3RDQID => {
            let id = netif::netid(p);
            let Some((_, tag)) = LEVEL3_FILES.iter().find(|(n, _)| *n == name) else {
                return Err("devether: name not found");
            };
            (netif::netqid(id, *tag), QTFILE)
        }
        _ => return Err("devether: name not found"),
    };
    let mut nc = Chan::new(DEV_ETHER, Qid { path: qid, vers: 0, typ }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// A decimal name (the conversation dirs), None on any non-digit.
fn parse_decimal(name: &[u8]) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let mut v: usize = 0;
    for &b in name {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add(usize::from(b - b'0'))?;
    }
    Some(v)
}

/// Open a walked chan (netifopen): dirs OREAD-only (Eperm shape);
/// data/ctl re-open the conversation (inuse++) with the first-opener
/// ownership collapse; clone allocates a fresh conversation and
/// rewrites the chan's qid to its ctl; the info files are OREAD-only
/// (Ebadarg shape).
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let path = c.qid.path;
    if c.qid.typ == QTDIR {
        if mode != OREAD {
            return Err("devether: directory opens OREAD only");
        }
        let mut nc = Chan::new(c.dev, c.qid, mode);
        nc.path = c.path.clone();
        return Ok(nc);
    }
    let mut qid = c.qid;
    match netif::nettype(path) {
        netif::NDATAQID | netif::NCTLQID => {
            netif::conv_open(netif::netid(path) as usize)?;
        }
        netif::NCLONEQID => {
            let id = netif::conv_alloc()?;
            qid = Qid { path: netif::netqid(id as u64, netif::NCTLQID), vers: 0, typ: QTFILE };
        }
        _ => {
            if mode != OREAD {
                return Err("devether: addr/stats/type/ifstats open OREAD only");
            }
        }
    }
    let mut nc = Chan::new(c.dev, qid, mode);
    nc.path = c.path.clone();
    Ok(nc)
}

/// dat.rs Chan::dup's bump for an open data/ctl chan (the devpipe
/// ref_open_end rule — see netif::conv_ref).
pub(crate) fn ref_open(qid_path: u64) {
    if matches!(netif::nettype(qid_path), netif::NDATAQID | netif::NCTLQID) {
        netif::conv_ref(qid_path);
    }
}

/// The close dispatch's DEV_ETHER arm (namec::close_chan, COPEN-gated
/// by the caller): the donor's netifclose — only data/ctl chans carry
/// the open-reference count.
pub fn close(c: &Chan) {
    if c.qid.typ == QTDIR {
        return;
    }
    if matches!(netif::nettype(c.qid.path), netif::NDATAQID | netif::NCTLQID) {
        netif::conv_close(netif::netid(c.qid.path) as usize);
    }
}

// ---------------------------------------------------------------------------
// stat + directory listings (the stat(5) wire, FOUR strings per record —
// libc's statcheck walks four; convM2D.c)
// ---------------------------------------------------------------------------

/// One stat(5) wire record (the devcons/devpipe shape; uid/gid/muid
/// glenda).  Returns the byte count (the convD2M contract).
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
        return Err("devether: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(DEV_ETHER as u32);
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

/// The record for one qid (the listing builder's and stat's shared
/// core).  `name` is the leaf name; lengths are computed live for the
/// counter files (the stats content shifts with the counter widths).
fn record_for(sb: &mut [u8], name: &[u8], qid_path: u64, typ: u8) -> Result<u64> {
    let tag = netif::nettype(qid_path);
    let id = netif::netid(qid_path) as usize;
    let (mode, length) = if typ == QTDIR {
        (0x8000_0000 | 0o555, 0)
    } else {
        match tag {
            netif::NCLONEQID => (0o666, 0),
            netif::NADDRQID => (0o666, netif::addr_content()?.len() as u64),
            netif::NDATAQID | netif::NCTLQID => {
                // The owner-collapse surface (0660 once first-opened).
                (if netif::conv_owned(id) { 0o660 } else { 0o666 }, 0)
            }
            netif::NSTATQID => (0o444, netif::stats_content()?.len() as u64),
            netif::NTYPEQID => (0o444, NUMSIZE as u64),
            netif::NIFSTATQID => (0o444, ifstats_content()?.len() as u64),
            _ => return Err("devether: stat of unknown file"),
        }
    };
    write_stat_record(sb, name, qid_path, typ, mode, length)
}

/// stat of a devether chan (sys_stat/sys_fstat's dispatch leg).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let path = c.qid.path;
    if path == 0 {
        return write_stat_record(sb, b"#l", 0, QTDIR, 0x8000_0000 | 0o555, 0);
    }
    match netif::nettype(path) {
        netif::N2NDQID => record_for(sb, netif::ifname()?, path, QTDIR),
        netif::N3RDQID => {
            // The conversation dir's name is its decimal id.
            let id = netif::netid(path);
            let mut name = Vec::new();
            let mut v = id;
            if v == 0 {
                name.push(b'0');
            }
            while v > 0 {
                name.insert(0, b'0' + (v % 10) as u8);
                v /= 10;
            }
            record_for(sb, &name, path, QTDIR)
        }
        netif::NCLONEQID => record_for(sb, b"clone", path, QTFILE),
        netif::NADDRQID => record_for(sb, b"addr", path, QTFILE),
        netif::NDATAQID => record_for(sb, b"data", path, QTFILE),
        netif::NCTLQID => record_for(sb, b"ctl", path, QTFILE),
        netif::NSTATQID => record_for(sb, b"stats", path, QTFILE),
        netif::NTYPEQID => record_for(sb, b"type", path, QTFILE),
        netif::NIFSTATQID => record_for(sb, b"ifstats", path, QTFILE),
        _ => Err("devether: stat of unknown file"),
    }
}

/// The directory listing (devdirread's shape): rebuilt per call from
/// the live conversation table; the streaming-offset convention.
fn dir_listing(c: &Chan) -> Result<Vec<u8>> {
    let mut listing = Vec::new();
    let mut emit = |name: &[u8], qid_path: u64, typ: u8| -> Result<()> {
        let start = listing.len();
        listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
        record_for(&mut listing[start..], name, qid_path, typ)?;
        Ok(())
    };
    match c.qid.path {
        0 => {
            emit(netif::ifname()?, netif::N2NDQID, QTDIR)?;
        }
        p if netif::nettype(p) == netif::N2NDQID => {
            for (name, qid, _) in LEVEL2_FILES {
                emit(name, qid, QTFILE)?;
            }
            // One numbered dir per ALLOCATED conversation (the donor
            // lists allocated slots regardless of inuse).
            let alloc = netif::conv_allocated();
            for (id, present) in alloc.iter().enumerate() {
                if !present {
                    continue;
                }
                let mut name = Vec::new();
                let mut v = id;
                if v == 0 {
                    name.push(b'0');
                }
                while v > 0 {
                    name.insert(0, b'0' + (v % 10) as u8);
                    v /= 10;
                }
                emit(&name, netif::netqid(id as u64, netif::N3RDQID), QTDIR)?;
            }
        }
        p if netif::nettype(p) == netif::N3RDQID => {
            let id = netif::netid(p);
            for (name, tag) in LEVEL3_FILES {
                emit(name, netif::netqid(id, tag), QTFILE)?;
            }
        }
        _ => return Err("devether: read of a non-directory"),
    }
    Ok(listing)
}

// ---------------------------------------------------------------------------
// read
// ---------------------------------------------------------------------------

/// readstr's window onto a content byte string (9k dev.c): offset
/// semantics, u64::MAX = the streaming chan offset.
fn read_str(c: &mut Chan, buf: &mut [u8], offset: u64, content: &[u8]) -> u64 {
    let off = if offset == u64::MAX { c.offset } else { offset };
    let start = (off as usize).min(content.len());
    let avail = &content[start..];
    let n = avail.len().min(buf.len());
    buf[..n].copy_from_slice(&avail[..n]);
    if offset == u64::MAX {
        c.offset += n as u64;
    }
    n as u64
}

/// readnum's fixed-width number (9k devcons.c): the value right-
/// justified in NUMSIZE-1 columns plus a trailing space (12 bytes
/// total).  The donor passes Netfile.type — a 32-bit int
/// (netif.h:40) — through readnum's 32-bit `ulong` value parameter
/// (devcons.c:712): truncating, never sign-extending, because Plan 9
/// long/ulong is 32-bit even on amd64 (amd64/include/u.h — the 64-bit
/// types are vlong/uvlong, hence VLNUMSIZE=22 for the 64-bit values
/// NUMSIZE=12 cannot hold).  The `as u32` below reproduces that C
/// conversion byte-for-byte: `connect -1` truncates to 4294967295
/// (10 digits < NUMSIZE-1=11), so the pad arithmetic can never
/// underflow — do NOT add a saturating clamp; a guard would mask a
/// future width regression.
/// Class sweep (the G-07-2 diagnosis's conclusion): this is the port's
/// ONLY NUMSIZE-digits arithmetic site; the NCTLQID caller passes
/// conversation ids 0..7 (cannot underflow; the donor's NETID macro
/// truncates to ulong first, netif.h:26), and netif.rs contains zero
/// NUMSIZE arithmetic.
/// pub(crate): the crate-root devether_tests pin module calls it.
pub(crate) fn readnum_content(val: i64) -> Vec<u8> {
    // Decimal digits of the donor's 32-bit ulong view (the int→ulong
    // truncation — not a clamp, not an underflow guard).
    let v = val as u32 as u64;
    let mut digits = Vec::new();
    if v == 0 {
        digits.push(b'0');
    }
    let mut v = v;
    while v > 0 {
        digits.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    let mut out = alloc::vec![b' '; NUMSIZE];
    let pad = NUMSIZE - 1 - digits.len();
    out[pad..NUMSIZE - 1].copy_from_slice(&digits);
    out
}

/// Read from a devether chan (netifread): dirs serve the stat-wire
/// listing; data serves one WHOLE frame (blocking, Qmsg — see below);
/// ctl returns the conversation id (readnum); stats/ifstats/addr/type
/// serve their live content.
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    if c.qid.typ == QTDIR {
        let listing = dir_listing(c)?;
        return Ok(read_str(c, buf, offset, &listing));
    }
    let path = c.qid.path;
    match netif::nettype(path) {
        netif::NDATAQID => read_data(c, buf),
        netif::NCTLQID => {
            let content = readnum_content(netif::netid(path) as i64);
            Ok(read_str(c, buf, offset, &content))
        }
        netif::NSTATQID => {
            let content = netif::stats_content()?;
            Ok(read_str(c, buf, offset, &content))
        }
        netif::NADDRQID => {
            let content = netif::addr_content()?;
            Ok(read_str(c, buf, offset, &content))
        }
        netif::NTYPEQID => {
            let t = netif::type_of(netif::netid(path) as usize)?;
            let content = readnum_content(i64::from(t));
            Ok(read_str(c, buf, offset, &content))
        }
        netif::NIFSTATQID => {
            let content = ifstats_content()?;
            Ok(read_str(c, buf, offset, &content))
        }
        _ => Err("devether: read of unknown file"),
    }
}

/// The data-file read (netif.c's `qread(f->iq, a, n)`): a blocking,
/// whole-frame read (Qmsg — one frame per read; a partial-frame read is
/// a defect, never a behavior).  The register → drop → block() shape
/// (devpipe's invariant, restated in netif.rs's header): on this kernel
/// syscalls run IRQ-masked, so no ingress can interleave between the
/// empty check and the waiter registration — a wakeup is un-losable.
fn read_data(c: &mut Chan, buf: &mut [u8]) -> Result<u64> {
    let id = netif::netid(c.qid.path) as usize;
    let mut h: Option<netif::EtherHooks> = None;
    loop {
        match netif::pop_frame(id, buf) {
            netif::Pop::Frame(n) => return Ok(n),
            netif::Pop::Closed => return Ok(0), // EOF — the donor's qclosed queue
            netif::Pop::Empty => {}
        }
        if h.is_none() {
            h = Some(netif::hooks()?);
        }
        let pid = (h.unwrap().current)();
        netif::register_rx_waiter(pid);
        (h.unwrap().block)(); // park mid-syscall; an ingress frame wakes us
    }
}

/// The ifstats file's content (the driver hook): one comma-free line
/// per queue in ethervirtio.c:439-468's accounting shape.
fn ifstats_content() -> Result<Vec<u8>> {
    let ops = driver()?;
    let queues = (ops.ifstats)();
    let mut out = Vec::new();
    for (q, (avail, used, lastused, nintr, nnote)) in queues.iter().enumerate() {
        out.extend_from_slice(b"vq");
        out.extend_from_slice(&decu(q as u64));
        out.extend_from_slice(b": avail=");
        out.extend_from_slice(&decu(u64::from(*avail)));
        out.extend_from_slice(b" used=");
        out.extend_from_slice(&decu(u64::from(*used)));
        out.extend_from_slice(b" lastused=");
        out.extend_from_slice(&decu(u64::from(*lastused)));
        out.extend_from_slice(b" nintr=");
        out.extend_from_slice(&decu(u64::from(*nintr)));
        out.extend_from_slice(b" nnote=");
        out.extend_from_slice(&decu(u64::from(*nnote)));
        out.push(b'\n');
    }
    Ok(out)
}

/// Decimal ASCII (the devproc dec idiom).
fn decu(mut v: u64) -> Vec<u8> {
    let mut out = Vec::new();
    if v == 0 {
        out.push(b'0');
    }
    while v > 0 {
        out.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    out
}

// ---------------------------------------------------------------------------
// write
// ---------------------------------------------------------------------------

/// Write to a devether chan (devether.c's etherwrite): only the data
/// and ctl qids accept writes; everything else is read-only (the
/// donor's Eperm arm).
pub fn write(c: &mut Chan, buf: &[u8], _offset: u64) -> Result<u64> {
    match netif::nettype(c.qid.path) {
        netif::NDATAQID => write_data(c, buf),
        netif::NCTLQID => write_ctl(c, buf),
        _ => Err("devether: file is read-only"),
    }
}

/// The data-file write (devether.c:283-299, netif-faithful): the bounds
/// gate (n > mtu=1514 → Etoobig; n < minmtu=60 → Etoosmall), the
/// source-MAC overwrite with the NIC's negotiated address, then the TX
/// post.  The 12-byte virtio header is the TX chain's zeroed header
/// page (num_buffers=0, §5.1.6.2.1) — prepended by the chain shape, not
/// by a copy.  A full ring BLOCKS on the wake hook until the IRQ
/// retire frees a slot (D-08's wedge-hunt: a stalled retire wedges the
/// writer and the gate's deadline catches it — never a silent drop).
fn write_data(_c: &mut Chan, buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    // The bounds gate (devether.c:283-286): n > mtu → Etoobig, n <
    // minmtu → Etoosmall — read from the model (the donor's
    // ether->mtu/minmtu: 1514/60, ETHERMINTU is the 07-04 fixture's
    // exact size).
    let (minmtu, mtu) = netif::mtu_bounds()?;
    if n > mtu as usize {
        return Err("devether: write too big");
    }
    if n < minmtu as usize {
        return Err("devether: write too small");
    }
    let ops = driver()?;
    // Copy into a kernel buffer and overwrite the frame's source MAC
    // with the negotiated address (devether.c:294's memmove — the
    // caller's buffer is a user-memory view, never written).
    let mut frame = Vec::with_capacity(n);
    frame.extend_from_slice(buf);
    frame[6..12].copy_from_slice(&ops.mac);
    let mut h: Option<netif::EtherHooks> = None;
    loop {
        match (ops.tx_post)(&frame)? {
            true => {
                netif::tx_posted(); // outpackets++ per frame posted
                return Ok(n as u64);
            }
            false => {
                // The TX ring has no free chain — block until the
                // retire wakes us (register → block is IRQ-atomic here:
                // syscalls run IRQ-masked, so no retire can interleave).
                if h.is_none() {
                    h = Some(netif::hooks()?);
                }
                let pid = (h.unwrap().current)();
                netif::register_tx_waiter(pid);
                (h.unwrap().block)();
            }
        }
    }
}

/// The ctl-file write (netif.c:293-362 + devether.c:262-281): the
/// verb set is the plan's locked depth — `connect N` functional (the
/// ether-type filter, range-checked to the ether-type width or -1 —
/// T-07-15), `promiscuous` accepted as a no-op toward the hardware,
/// `nonblocking`/`addmulti`/`remmulti`/`scanbs`/`bridge`/`headersonly`
/// loud-unimplemented, unknown verbs a loud bad-ctl error.
fn write_ctl(c: &mut Chan, buf: &[u8]) -> Result<u64> {
    let id = netif::netid(c.qid.path) as usize;
    let n = buf.len();
    // The donor's 64-byte cmdbuf bound.
    let text = &buf[..n.min(63)];
    // matchtoken's shape: the verb, then whitespace-separated args.
    let mut it = text.split(|&b| b == b' ' || b == b'\t' || b == b'\n').filter(|w| !w.is_empty());
    let Some(verb) = it.next() else { return Ok(n as u64) }; // an empty write is a no-op
    match verb {
        b"connect" => {
            let Some(arg) = it.next() else { return Err("devether: bad ctl") };
            let typ = parse_i32(arg).ok_or("devether: bad ctl")?;
            if typ < -1 || typ > 0xffff {
                return Err("devether: ether-type out of range");
            }
            netif::conv_connect(id, typ)?;
            Ok(n as u64)
        }
        b"promiscuous" => {
            netif::conv_promiscuous(id)?;
            Ok(n as u64)
        }
        b"nonblocking" => Err("devether: nonblocking unsupported (writes are synchronous)"),
        b"addmulti" | b"remmulti" => Err("devether: multicast unsupported (no CTRL_VQ)"),
        b"scanbs" | b"bridge" | b"headersonly" => Err("devether: ctl verb unsupported this era"),
        _ => Err("devether: bad ctl"),
    }
}

/// A bounded decimal integer, optionally negative (the connect verb's
/// argument; the donor's atoi with a range check — T-07-15).
fn parse_i32(arg: &[u8]) -> Option<i32> {
    if arg.is_empty() {
        return None;
    }
    let (neg, digits) = match arg[0] {
        b'-' => (true, &arg[1..]),
        _ => (false, arg),
    };
    if digits.is_empty() {
        return None;
    }
    let mut v: i64 = 0;
    for &b in digits {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add(i64::from(b - b'0'))?;
    }
    let v = if neg { -v } else { v };
    i32::try_from(v).ok()
}
