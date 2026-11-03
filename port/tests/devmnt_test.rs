//! Host-side tests for the 05-07 devmnt kernel 9P client (D-03's mount
//! half; T-05-13's mntchk discipline; T-05-14's mount-crossing matrix).
//!
//! The rig: a SCRIPTED FAKE SERVER speaking real 9P2000 wire bytes over an
//! in-memory byte pipe (the paqfs_test.rs precedent — the bytes a real
//! server produces, built independently of the codec under test).  The
//! fake implements `devmnt::MntIo` over a canned tree:
//!
//!   root (qid 100, dir)
//!   ├── adir (qid 101, dir)
//!   │   └── leaf.txt (qid 102) = "hello 9p\n"
//!   └── top.txt (qid 103) = "top of tree\n"
//!
//! plus the malformation knobs T-05-13 needs: an inflated reply-size
//! field (the oversized-claim rejection), a corrupted reply tag (the
//! tag-mismatch rejection), and a server that tries to RAISE the msize
//! (devmnt.c's "server tries to increase msize" check).
//!
//! The fid table is shared with the test via Rc<RefCell> so Tclunk's
//! fid release is directly observable.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

use port::dat::{Chan, DEV_MNT, DEV_ROOT};
use port::devmnt::{self, Mnt, MntIo};
use port::devroot;
use port::namec::{self, NamecMode};
use port::pgrp::{self, Pgrp};
use port::Result;

// ---------------------------------------------------------------------------
// Little-endian wire helpers (fcall.h's PBIT* — built test-side so the
// codec under test is exercised through real bytes)
// ---------------------------------------------------------------------------

fn pu8(v: &mut Vec<u8>, x: u8) {
    v.push(x);
}
fn pu16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pu32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pu64(v: &mut Vec<u8>, x: u64) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pstr(v: &mut Vec<u8>, s: &[u8]) {
    pu16(v, s.len() as u16);
    v.extend_from_slice(s);
}
fn pqid(v: &mut Vec<u8>, typ: u8, vers: u32, path: u64) {
    pu8(v, typ);
    pu32(v, vers);
    pu64(v, path);
}
/// Wrap a body in the 9P header: size[4] type[1] tag[2] + body.
fn frame(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    pu32(&mut v, (7 + body.len()) as u32);
    pu8(&mut v, typ);
    pu16(&mut v, tag);
    v.extend_from_slice(body);
    v
}

/// A checked little-endian cursor for parsing the fake's inbound
/// T-messages (the test's own mini convM2S).
struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Cur<'a> {
        Cur { b, p: 0 }
    }
    fn u8(&mut self) -> u8 {
        let v = self.b[self.p];
        self.p += 1;
        v
    }
    fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.b[self.p..self.p + 2].try_into().unwrap());
        self.p += 2;
        v
    }
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.b[self.p..self.p + 4].try_into().unwrap());
        self.p += 4;
        v
    }
    fn u64(&mut self) -> u64 {
        let v = u64::from_le_bytes(self.b[self.p..self.p + 8].try_into().unwrap());
        self.p += 8;
        v
    }
    fn string(&mut self) -> Vec<u8> {
        let n = self.u16() as usize;
        let s = self.b[self.p..self.p + n].to_vec();
        self.p += n;
        s
    }
}

// ---------------------------------------------------------------------------
// The fake server
// ---------------------------------------------------------------------------

const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

// The wire message types (fcall.h — frozen values, restated test-side).
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

const QID_ROOT: u64 = 100;
const QID_ADIR: u64 = 101;
const QID_LEAF: u64 = 102;
const QID_TOP: u64 = 103;

const LEAF_CONTENT: &[u8] = b"hello 9p\n";
const TOP_CONTENT: &[u8] = b"top of tree\n";

/// One tree node (flat table keyed by qid path).
struct Node {
    name: &'static [u8],
    parent: u64, // 0 = root's own parent (unused)
    typ: u8,
    content: &'static [u8],
}

fn tree() -> HashMap<u64, Node> {
    let mut t = HashMap::new();
    t.insert(QID_ROOT, Node { name: b"/", parent: 0, typ: QTDIR, content: b"" });
    t.insert(QID_ADIR, Node { name: b"adir", parent: QID_ROOT, typ: QTDIR, content: b"" });
    t.insert(QID_LEAF, Node { name: b"leaf.txt", parent: QID_ADIR, typ: QTFILE, content: LEAF_CONTENT });
    t.insert(QID_TOP, Node { name: b"top.txt", parent: QID_ROOT, typ: QTFILE, content: TOP_CONTENT });
    t
}

/// A stat(5) image for a node (the packDir shape: the 2-byte prefix is
/// the count of the REMAINING bytes).
fn pack_stat(n: &Node, qid_path: u64) -> Vec<u8> {
    let mut body = Vec::new();
    pu16(&mut body, 0); // type: unused
    pu32(&mut body, 7); // dev: arbitrary
    pqid(&mut body, n.typ, 0, qid_path);
    pu32(&mut body, if n.typ == QTDIR { 0x8000_0000 | 0o555 } else { 0o444 }); // mode
    pu32(&mut body, 1_700_000_000); // atime
    pu32(&mut body, 1_700_000_000); // mtime
    pu64(&mut body, n.content.len() as u64);
    pstr(&mut body, n.name);
    pstr(&mut body, b"glenda");
    pstr(&mut body, b"glenda");
    let mut out = Vec::new();
    pu16(&mut out, body.len() as u16);
    out.extend_from_slice(&body);
    out
}

/// The scripted server: MntIo over a canned tree with malformation knobs.
/// `write_msg` processes one whole T-message synchronously (every T the
/// client sends is a single whole-message write — the pipe's contract);
/// the reply queues for `read_some`.
struct FakeServer {
    outq: VecDeque<u8>,
    fids: Rc<RefCell<HashMap<u32, u64>>>, // fid → node qid path
    /// The server's msize answer cap (rversion: min(offer, this)).
    server_msize: u32,
    /// Malformation knobs (T-05-13's rejection proofs):
    inflate_msize: bool, // answer msize HIGHER than the offer
    size_boost: u32,     // add to EVERY reply's size field (no bytes added)
    size_boost_after_first: u32, // add to every reply AFTER the first
    wrong_tag: bool,     // echo tag+1 on every reply
    walk_zero_qid: bool, // answer Rwalk(nwqid=0) instead of Rerror on a miss
    replies: usize,      // replies sent (the after-first knob's counter)
}

impl FakeServer {
    fn new() -> (FakeServer, Rc<RefCell<HashMap<u32, u64>>>) {
        let fids = Rc::new(RefCell::new(HashMap::new()));
        (
            FakeServer {
                outq: VecDeque::new(),
                fids: Rc::clone(&fids),
                server_msize: 8192,
                inflate_msize: false,
                size_boost: 0,
                size_boost_after_first: 0,
                wrong_tag: false,
                walk_zero_qid: false,
                replies: 0,
            },
            fids,
        )
    }

    fn rerror(&mut self, tag: u16, ename: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        pstr(&mut body, ename);
        self.finish(RERROR, tag, &body)
    }

    /// Frame + the malformation knobs.
    fn finish(&mut self, typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
        let tag = if self.wrong_tag { tag.wrapping_add(1) } else { tag };
        let mut msg = frame(typ, tag, body);
        let boost = if self.replies == 0 { self.size_boost } else { self.size_boost_after_first };
        if boost != 0 {
            let size = u32::from_le_bytes(msg[0..4].try_into().unwrap()) + boost;
            msg[0..4].copy_from_slice(&size.to_le_bytes());
        }
        self.replies += 1;
        msg
    }

    fn handle(&mut self, msg: &[u8]) -> Vec<u8> {
        let mut c = Cur::new(msg);
        let _size = c.u32();
        let typ = c.u8();
        let tag = c.u16();
        let t = tree();
        match typ {
            TVERSION => {
                let offer = c.u32();
                let _version = c.string();
                let mut body = Vec::new();
                let answer = if self.inflate_msize { offer + 4096 } else { offer.min(self.server_msize) };
                pu32(&mut body, answer);
                pstr(&mut body, b"9P2000");
                self.finish(RVERSION, tag, &body)
            }
            TATTACH => {
                let fid = c.u32();
                let _afid = c.u32();
                let _uname = c.string();
                let _aname = c.string();
                self.fids.borrow_mut().insert(fid, QID_ROOT);
                let mut body = Vec::new();
                pqid(&mut body, QTDIR, 0, QID_ROOT);
                self.finish(RATTACH, tag, &body)
            }
            TWALK => {
                let fid = c.u32();
                let newfid = c.u32();
                let nwname = c.u16();
                let names: Vec<Vec<u8>> = (0..nwname).map(|_| c.string()).collect();
                let found = { self.fids.borrow().get(&fid).copied() };
                let Some(start) = found else {
                    return self.rerror(tag, b"fid not found");
                };
                let mut cur = start;
                let mut qids = Vec::new();
                let mut err: Option<&[u8]> = None;
                for (i, name) in names.iter().enumerate() {
                    let node = &t[&cur];
                    if node.typ != QTDIR {
                        err = Some(b"not a directory");
                        break;
                    }
                    let child = t
                        .iter()
                        .find(|(_, n)| n.parent == cur && n.name == name.as_slice())
                        .map(|(q, _)| *q);
                    let Some(child) = child else {
                        if i == 0 && !self.walk_zero_qid {
                            err = Some(b"file does not exist");
                        }
                        break;
                    };
                    cur = child;
                    qids.push((t[&child].typ, child));
                }
                if let Some(ename) = err {
                    return self.rerror(tag, ename);
                }
                if qids.len() == names.len() {
                    self.fids.borrow_mut().insert(newfid, cur);
                }
                let mut body = Vec::new();
                pu16(&mut body, qids.len() as u16);
                for (typ, path) in qids {
                    pqid(&mut body, typ, 0, path);
                }
                self.finish(RWALK, tag, &body)
            }
            TOPEN => {
                let fid = c.u32();
                let mode = c.u8();
                let found = { self.fids.borrow().get(&fid).copied() };
                let Some(node) = found else {
                    return self.rerror(tag, b"fid not found");
                };
                let n = &t[&node];
                if n.typ == QTDIR && mode != 0 {
                    return self.rerror(tag, b"permission denied");
                }
                let mut body = Vec::new();
                pqid(&mut body, n.typ, 0, node);
                pu32(&mut body, 0); // iounit 0 (paqfs's answer)
                self.finish(ROPEN, tag, &body)
            }
            TREAD => {
                let fid = c.u32();
                let off = c.u64();
                let count = c.u32();
                let found = { self.fids.borrow().get(&fid).copied() };
                let Some(node) = found else {
                    return self.rerror(tag, b"fid not found");
                };
                let n = &t[&node];
                if n.typ == QTDIR {
                    return self.rerror(tag, b"dir reads unimplemented in the fake");
                }
                let content = n.content;
                let data = if off as usize >= content.len() {
                    &[][..]
                } else {
                    &content[off as usize..content.len().min(off as usize + count as usize)]
                };
                let mut body = Vec::new();
                pu32(&mut body, data.len() as u32);
                body.extend_from_slice(data);
                self.finish(RREAD, tag, &body)
            }
            TSTAT => {
                let fid = c.u32();
                let found = { self.fids.borrow().get(&fid).copied() };
                let Some(node) = found else {
                    return self.rerror(tag, b"fid not found");
                };
                let stat = pack_stat(&t[&node], node);
                let mut body = Vec::new();
                pu16(&mut body, stat.len() as u16);
                body.extend_from_slice(&stat);
                self.finish(RSTAT, tag, &body)
            }
            TCLUNK => {
                let fid = c.u32();
                self.fids.borrow_mut().remove(&fid);
                self.finish(RCLUNK, tag, &[])
            }
            _ => self.rerror(tag, b"bad fcall type"),
        }
    }
}

impl MntIo for FakeServer {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        let reply = self.handle(buf);
        self.outq.extend(reply);
        Ok(buf.len() as u64)
    }
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        let n = self.outq.len().min(buf.len());
        for b in buf.iter_mut().take(n) {
            *b = self.outq.pop_front().unwrap();
        }
        Ok(n as u64)
    }
}

/// Build a negotiated session over a fresh, well-behaved fake (and the
/// shared fid table for server-side assertions).
fn session() -> (Mnt, Rc<RefCell<HashMap<u32, u64>>>) {
    let (fake, fids) = FakeServer::new();
    let mnt = Mnt::negotiate(Box::new(fake), 0).expect("the version handshake negotiates");
    (mnt, fids)
}

// ---------------------------------------------------------------------------
// The behavior bullets (05-07 Task 1)
// ---------------------------------------------------------------------------

#[test]
fn devmnt_version_attach_walk_open_read_stat_clunk_round_trip() {
    let (mnt, fids) = session();

    // The negotiated msize is the offered default (the server accepts).
    assert_eq!(mnt.msize(), 8192, "offer 8192, server accepts 8192");

    // attach: a fresh fid on the tree root, afid = NOFID (D-02).
    let (fid, qid) = mnt.attach(b"glenda", b"").expect("attach over the pipe");
    assert!(fids.borrow().contains_key(&fid), "the server holds the attach fid");
    assert_eq!(qid.typ, QTDIR, "the tree root is a directory");
    assert_eq!(qid.path, QID_ROOT);

    // A multi-component path CHAINS: walk adir, then leaf.txt under it
    // (each walk allocates a fresh fid — the donor mntwalk shape).
    let (adir_fid, adir_qid) = mnt.walk(fid, b"adir").expect("walk adir");
    assert_eq!(adir_qid.path, QID_ADIR);
    let (leaf_fid, leaf_qid) = mnt.walk(adir_fid, b"leaf.txt").expect("walk adir/leaf.txt");
    assert_eq!(leaf_qid.path, QID_LEAF);
    assert_eq!(leaf_qid.typ, QTFILE);

    // open OREAD, then read the content in two offset-addressed RPCs.
    let (open_qid, _iounit) = mnt.open(leaf_fid, 0).expect("open OREAD");
    assert_eq!(open_qid.path, QID_LEAF);
    let first = mnt.read(leaf_fid, 0, 4).expect("read [0..4]");
    assert_eq!(first.as_slice(), b"hell");
    let rest = mnt.read(leaf_fid, 4, 100).expect("read [4..] clamps at EOF");
    assert_eq!(rest.as_slice(), b"o 9p\n");
    // Reading AT end is a zero-length reply, never an error.
    let eof = mnt.read(leaf_fid, 9, 100).expect("read at EOF");
    assert!(eof.is_empty());

    // stat: the payload is a stat(5) image (its own 2-byte prefix counts
    // the remaining bytes) naming leaf.txt with its length.
    let stat = mnt.stat(leaf_fid).expect("stat");
    let prefix = u16::from_le_bytes(stat[0..2].try_into().unwrap()) as usize;
    assert_eq!(prefix, stat.len() - 2, "the stat prefix counts the rest");
    let len_off = 2 + 2 + 4 + 13 + 4 + 4 + 4; // size+type+dev+qid+mode+atime+mtime
    let length = u64::from_le_bytes(stat[len_off..len_off + 8].try_into().unwrap());
    assert_eq!(length, LEAF_CONTENT.len() as u64);

    // Tclunk releases the fid server-side (observable via the shared table).
    let before = fids.borrow().len();
    mnt.clunk(leaf_fid).expect("clunk");
    assert_eq!(fids.borrow().len(), before - 1, "the fid is released");
    assert!(!fids.borrow().contains_key(&leaf_fid));
}

#[test]
fn devmnt_version_accepts_servers_lower_answer() {
    let (mut fake, _fids) = FakeServer::new();
    fake.server_msize = 2048;
    let mnt = Mnt::negotiate(Box::new(fake), 0).expect("the server answers lower");
    assert_eq!(mnt.msize(), 2048, "the negotiated msize is the server's lower answer");
}

#[test]
fn devmnt_rejects_server_msize_increase() {
    let (mut fake, _fids) = FakeServer::new();
    fake.inflate_msize = true; // the server answers HIGHER than the offer
    let err = Mnt::negotiate(Box::new(fake), 0).expect_err("a raised msize is a protocol error");
    assert!(err.contains("msize"), "the error names the msize violation: {err}");
}

#[test]
fn devmnt_rejects_oversized_reply() {
    // T-05-13's core: an R-message claiming more bytes than the negotiated
    // msize is rejected BEFORE any field is trusted.  The knob is armed
    // BEFORE the handshake, so the Rversion's size field (19 real bytes +
    // 8192 boost = 8211 claimed) exceeds the 8192 offer.
    let (mut fake, _fids) = FakeServer::new();
    fake.size_boost = 8192;
    let err = Mnt::negotiate(Box::new(fake), 0)
        .expect_err("an R-message claiming more than the msize is a protocol error");
    assert!(err.contains("msize"), "the error names the size violation: {err}");

    // And post-negotiation: a clean handshake, then the ATTACH reply's
    // size field is inflated (20 real bytes + 8192 = 8212 > the
    // negotiated 8192) — the session bound, not just the handshake's.
    let (mut fake2, _f) = FakeServer::new();
    fake2.size_boost_after_first = 8192;
    let mnt = Mnt::negotiate(Box::new(fake2), 0).expect("a clean handshake negotiates");
    let err = mnt
        .attach(b"glenda", b"")
        .expect_err("the inflated Rattach exceeds the negotiated msize");
    assert!(err.contains("msize"), "the error names the size violation: {err}");
}

#[test]
fn devmnt_rejects_tag_mismatch() {
    let (mut fake, _fids) = FakeServer::new();
    fake.wrong_tag = true; // every reply's tag is off by one
    let err = Mnt::negotiate(Box::new(fake), 0)
        .expect_err("a reply tag that is not the outstanding tag is a protocol error");
    assert!(err.contains("tag"), "the error names the tag violation: {err}");
}

#[test]
fn devmnt_walk_not_found_is_a_loud_error() {
    let (mnt, _fids) = session();
    let (fid, _qid) = mnt.attach(b"glenda", b"").expect("attach");
    // The paqfs shape: a first-component miss answers Rerror.
    let err = mnt.walk(fid, b"nosuch").expect_err("walk of a missing name errors");
    assert!(err.contains("devmnt"), "the error is a devmnt errstr: {err}");
    // The donor's other shape: Rwalk with nwqid < nwname (0 for a
    // single-name walk) is also the not-found.
    let (mut fake2, _f) = FakeServer::new();
    fake2.walk_zero_qid = true;
    let mnt2 = Mnt::negotiate(Box::new(fake2), 0).expect("negotiate");
    let (fid2, _) = mnt2.attach(b"glenda", b"").expect("attach");
    assert!(mnt2.walk(fid2, b"nosuch").is_err(), "Rwalk(0) on a single-name walk is not-found");
}

#[test]
fn devmnt_walk_of_a_non_directory_errors() {
    let (mnt, _fids) = session();
    let (fid, _) = mnt.attach(b"glenda", b"").expect("attach");
    let (top_fid, qid) = mnt.walk(fid, b"top.txt").expect("walk top.txt");
    assert_eq!(qid.typ, QTFILE);
    let err = mnt.walk(top_fid, b"child").expect_err("walking INTO a file errors (Enotdir)");
    assert!(err.contains("devmnt"), "the error is a devmnt errstr: {err}");
}

// ---------------------------------------------------------------------------
// T-05-14: the mount-crossing matrix (the 05-04 namespace host matrix
// extended to devmnt) — a bind of the fake's tree onto the devroot
// fixture's /dir, then namec resolution ACROSS the stitch.
// ---------------------------------------------------------------------------

static FIXTURE: &[u8] = include_bytes!("fixtures/probe-tree.paq");

fn fixture_slash() -> Chan {
    devroot::mount_initrd(FIXTURE).expect("mount_initrd");
    devroot::root().expect("root mounted").attach_root().expect("attach root")
}

/// Bind the fake's tree at `target` in a fresh pgrp (the sys_mount table
/// half, host-side).
fn mount_fake(pg: &mut Pgrp, slash: &Chan, target: &str, flag: u32) -> Chan {
    let (mnt, _fids) = session();
    let root = devmnt::attach(&Arc::new(mnt), b"").expect("attach the fake tree");
    let tgt = namec::namec(target.as_bytes(), slash, slash, pg, NamecMode::MountTarget)
        .expect("the mount target resolves");
    pg.bind(root.dup(), &tgt, flag).expect("the mount inserts");
    root
}

#[test]
fn devmnt_mount_crossing_walk_matrix() {
    let mut pg = Pgrp::new();
    let slash = fixture_slash();

    // MAFTER: the underlying fixture dir wins ties; misses fall THROUGH to
    // the mounted tree (the union fallback).
    mount_fake(&mut pg, &slash, "/dir", pgrp::MAFTER);

    let under = namec::namec(b"/dir/nested.txt", &slash, &slash, &pg, NamecMode::Open)
        .expect("the underlying fixture file resolves");
    assert_eq!(under.dev, DEV_ROOT, "MAFTER: the underlying wins the tie");

    let mut cross = namec::namec(b"/dir/adir/leaf.txt", &slash, &slash, &pg, NamecMode::Open)
        .expect("a miss in the underlying falls through to the mounted tree");
    assert_eq!(cross.dev, DEV_MNT, "the crossed chan is devmnt's");
    let mut opened = namec::open_chan(&cross, 0).expect("open the crossed file OREAD");
    let mut buf = [0u8; 64];
    let n = devmnt::read(&mut opened, &mut buf, 0).expect("read through the stitch");
    assert_eq!(&buf[..n as usize], LEAF_CONTENT, "the mounted file's bytes cross the stitch");
    // The streaming-offset convention: offset u64::MAX continues from the
    // chan's offset (advanced by the explicit-offset read above — no, an
    // explicit-offset read does NOT advance the chan; a streaming read
    // starts at the chan's 0).
    let mut buf2 = [0u8; 64];
    let n2 = devmnt::read(&mut opened, &mut buf2, u64::MAX).expect("a streaming read");
    assert_eq!(&buf2[..n2 as usize], LEAF_CONTENT);
    let _ = &mut cross;

    // MREPL: the mounted tree REPLACES the underlying (a fresh pgrp).
    let mut pg2 = Pgrp::new();
    mount_fake(&mut pg2, &slash, "/dir", pgrp::MREPL);
    assert!(
        namec::namec(b"/dir/nested.txt", &slash, &slash, &pg2, NamecMode::Open).is_err(),
        "MREPL shadows the underlying fixture file"
    );
    let top = namec::namec(b"/dir/top.txt", &slash, &slash, &pg2, NamecMode::Open)
        .expect("the mounted tree answers after MREPL");
    assert_eq!(top.dev, DEV_MNT);

    // The mounted ROOT itself resolves to the stitch (the final-component
    // redirect to the union head — MAFTER's head is the underlying; MREPL's
    // head is the mount).
    let rootdir = namec::namec(b"/dir", &slash, &slash, &pg2, NamecMode::Open)
        .expect("the mount point resolves");
    assert_eq!(rootdir.dev, DEV_MNT, "MREPL: the point resolves to the mounted root");
}
