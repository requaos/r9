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
use std::sync::{Arc, Condvar, Mutex};

use port::dat::{Chan, DEV_MNT, DEV_ROOT};
use port::devmnt::{self, Mnt, MntIo};
use port::devroot;
use port::mcslock::{Lock, LockNode};
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
const TWRITE: u8 = 118;
const TCLUNK: u8 = 120;
const RWRITE: u8 = 119;
const RCLUNK: u8 = 121;
const TSTAT: u8 = 124;
const RSTAT: u8 = 125;
const TREMOVE: u8 = 122;
const RREMOVE: u8 = 123;

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
    /// 261104-b4p (item 5): the Tclunk record — EVERY clunked fid, in
    /// order (the exactly-once assertions read this, not just the table).
    clunked: Rc<RefCell<Vec<u32>>>,
    /// 19-10 cont-5: EVERY observed Twrite's data, in order — the
    /// zero-count Twrite pin reads this (the server OBSERVES empty
    /// writes; a skip-when-empty client loop never appears here).
    writes: Rc<RefCell<Vec<Vec<u8>>>>,
    /// 20-14: EVERY Tremove the server accepted, as (fid, removed qid)
    /// — the remove pins read which fid carried the remove.
    removed: Rc<RefCell<Vec<(u32, u64)>>>,
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
    fn new() -> (FakeServer, Rc<RefCell<HashMap<u32, u64>>>, Rc<RefCell<Vec<u32>>>) {
        let (f, fids, clunked, _writes) = FakeServer::new_with_writes();
        (f, fids, clunked)
    }

    /// 19-10 cont-5: the writes-log variant (the zero-count Twrite pin).
    fn new_with_writes() -> (
        FakeServer,
        Rc<RefCell<HashMap<u32, u64>>>,
        Rc<RefCell<Vec<u32>>>,
        Rc<RefCell<Vec<Vec<u8>>>>,
    ) {
        let fids = Rc::new(RefCell::new(HashMap::new()));
        let clunked = Rc::new(RefCell::new(Vec::new()));
        let writes = Rc::new(RefCell::new(Vec::new()));
        (
            FakeServer {
                outq: VecDeque::new(),
                fids: Rc::clone(&fids),
                clunked: Rc::clone(&clunked),
                writes: Rc::clone(&writes),
                removed: Rc::new(RefCell::new(Vec::new())),
                server_msize: 8192,
                inflate_msize: false,
                size_boost: 0,
                size_boost_after_first: 0,
                wrong_tag: false,
                walk_zero_qid: false,
                replies: 0,
            },
            fids,
            clunked,
            writes,
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
            TWRITE => {
                let fid = c.u32();
                let _off = c.u64();
                let count = c.u32();
                let start = c.p;
                let data = msg[start..start + count as usize].to_vec();
                if !self.fids.borrow().contains_key(&fid) {
                    return self.rerror(tag, b"fid not found");
                }
                self.writes.borrow_mut().push(data);
                let mut body = Vec::new();
                pu32(&mut body, count);
                self.finish(RWRITE, tag, &body)
            }
            TREMOVE => {
                // remove(5): the fid is clunked whether or not the
                // remove succeeds.  The fake refuses directories (the
                // loud-refusal pin) and logs every accepted remove.
                let fid = c.u32();
                let found = { self.fids.borrow_mut().remove(&fid) };
                let Some(node) = found else {
                    return self.rerror(tag, b"fid not found");
                };
                if t[&node].typ == QTDIR {
                    return self.rerror(tag, b"permission denied");
                }
                self.removed.borrow_mut().push((fid, node));
                self.finish(RREMOVE, tag, &[])
            }
            TCLUNK => {
                let fid = c.u32();
                self.fids.borrow_mut().remove(&fid);
                self.clunked.borrow_mut().push(fid);
                self.finish(RCLUNK, tag, &[])
            }
            _ => self.rerror(tag, b"bad fcall type"),
        }
    }
}

impl MntIo for FakeHalf {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        let reply = self.0.borrow_mut().handle(buf);
        self.0.borrow_mut().outq.extend(reply);
        Ok(buf.len() as u64)
    }
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        let n = {
            let mut f = self.0.borrow_mut();
            let n = f.outq.len().min(buf.len());
            for b in buf.iter_mut().take(n) {
                *b = f.outq.pop_front().unwrap();
            }
            n
        };
        Ok(n as u64)
    }
}

/// The session's split-wire shape over one shared fake: BOTH halves
/// borrow the SAME server (the kernel's two ChanIo dups share one
/// wire — a parked reader in io_r and an active writer in io_w see
/// one reply queue).
struct FakeHalf(Rc<RefCell<FakeServer>>);

impl FakeServer {
    /// The (io_r, io_w) pair over this server, moved behind Rc.
    fn split(self) -> (Box<FakeHalf>, Box<FakeHalf>) {
        let rc = Rc::new(RefCell::new(self));
        (Box::new(FakeHalf(Rc::clone(&rc))), Box::new(FakeHalf(rc)))
    }
}

/// Build a negotiated session over a fresh, well-behaved fake (and the
/// shared fid table for server-side assertions).
fn session() -> (Mnt, Rc<RefCell<HashMap<u32, u64>>>) {
    let (fake, fids, _clunked) = FakeServer::new();
    let (r, w) = fake.split();
    let mnt = Mnt::negotiate(r, w, 0).expect("the version handshake negotiates");
    (mnt, fids)
}

/// 261104-b4p: a session with the Tclunk record exposed (the fid
/// lifecycle pins assert exact clunk counts, not just table presence).
fn session_with_clunk_log() -> (Mnt, Rc<RefCell<HashMap<u32, u64>>>, Rc<RefCell<Vec<u32>>>) {
    let (fake, fids, clunked) = FakeServer::new();
    let (r, w) = fake.split();
    let mnt = Mnt::negotiate(r, w, 0).expect("the version handshake negotiates");
    (mnt, fids, clunked)
}

// ---------------------------------------------------------------------------
// 19-10 Leg-1 cont 5: the zero-count Twrite (the cursor-arrow revert)
// ---------------------------------------------------------------------------

/// The donor mntrdwr is a do-while: even a ZERO-LENGTH write sends one
/// Twrite with count=0, and the SERVER observes it — drawterm's devmouse
/// Qcursor carries the arrow reset on exactly that event (libdraw
/// setcursor(nil) writes 0 bytes).  A skip-when-empty client loop
/// returns Ok(0) having sent NOTHING: the cursor sticks at the last
/// contextual shape (the owner's live report — no zero-length Qcursor
/// write ever observed host-side across every retained trace).
#[test]
fn zero_length_write_sends_the_zero_count_twrite() {
    let (fake, fids, _clunked, writes) = FakeServer::new_with_writes();
    let (r, w) = fake.split();
    let mnt = Mnt::negotiate(r, w, 0).expect("negotiate");
    let root = devmnt::attach(&Arc::new(mnt), b"").expect("attach");
    // open top.txt OWRITE (the fake's tree's plain file)
    let leaf = devmnt::walk(&root, b"top.txt").expect("walk top.txt");
    let mut chan = devmnt::open(&leaf, 2 /* OWRITE */).expect("open OWRITE");

    // The zero-length write: Ok(0) AND the server saw ONE empty Twrite.
    let n = devmnt::write(&mut chan, &[], u64::MAX).expect("zero-length write");
    assert_eq!(n, 0, "the byte count is 0");
    let log = writes.borrow();
    assert_eq!(
        log.len(),
        1,
        "the server must OBSERVE the zero-count Twrite (the donor do-while)"
    );
    assert!(log[0].is_empty(), "the observed Twrite carries count=0");

    // A follow-up non-empty write lands as its own Twrite (order kept).
    drop(log);
    devmnt::write(&mut chan, b"xy", u64::MAX).expect("2-byte write");
    assert_eq!(writes.borrow().len(), 2);
    assert_eq!(writes.borrow()[1].as_slice(), b"xy");
    let _ = fids; // session fid table lives as long as the assertions
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
    let (open_qid, iounit) = mnt.open(leaf_fid, 0).expect("open OREAD");
    assert_eq!(open_qid.path, QID_LEAF);
    // 19-10 (the desktop leg): the Ropen iounit — the fake answers 0
    // (paqfs's shape) and the clamp must land on the era's per-rpc DATA
    // cap (rpc_data_cap: min(msize, TRANSPORT_MSG_MAX)-IOHDRSZ = 4072
    // at the 8192 negotiation), NOT msize-24 — the iounit is the
    // caller's max-single-write contract, and a value above the
    // transport splits writes mid-message (libdraw's draw buffer sizes
    // by iounit; the cpu-session rio death this pin guards).
    assert_eq!(iounit, 4072, "iounit 0 clamps to rpc_data_cap, never msize-24");
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
    let (mut fake, _fids, _cl) = FakeServer::new();
    fake.server_msize = 2048;
    let (r, w) = fake.split();
    let mnt = Mnt::negotiate(r, w, 0).expect("the server answers lower");
    assert_eq!(mnt.msize(), 2048, "the negotiated msize is the server's lower answer");
}

#[test]
fn devmnt_rejects_server_msize_increase() {
    let (mut fake, _fids, _cl) = FakeServer::new();
    fake.inflate_msize = true; // the server answers HIGHER than the offer
    let (r, w) = fake.split();
    let err = Mnt::negotiate(r, w, 0).expect_err("a raised msize is a protocol error");
    assert!(err.contains("msize"), "the error names the msize violation: {err}");
}

#[test]
fn devmnt_rejects_oversized_reply() {
    // T-05-13's core: an R-message claiming more bytes than the negotiated
    // msize is rejected BEFORE any field is trusted.  The knob is armed
    // BEFORE the handshake, so the Rversion's size field (19 real bytes +
    // 8192 boost = 8211 claimed) exceeds the 8192 offer.
    let (mut fake, _fids, _cl) = FakeServer::new();
    fake.size_boost = 8192;
    let (r, w) = fake.split();
    let err = Mnt::negotiate(r, w, 0)
        .expect_err("an R-message claiming more than the msize is a protocol error");
    assert!(err.contains("msize"), "the error names the size violation: {err}");

    // And post-negotiation: a clean handshake, then the ATTACH reply's
    // size field is inflated (20 real bytes + 8192 = 8212 > the
    // negotiated 8192) — the session bound, not just the handshake's.
    let (mut fake2, _f, _c2) = FakeServer::new();
    fake2.size_boost_after_first = 8192;
    let (r2, w2) = fake2.split();
    let mnt = Mnt::negotiate(r2, w2, 0).expect("a clean handshake negotiates");
    let err = mnt
        .attach(b"glenda", b"")
        .expect_err("the inflated Rattach exceeds the negotiated msize");
    assert!(err.contains("msize"), "the error names the size violation: {err}");
}

#[test]
fn devmnt_rejects_tag_mismatch() {
    let (mut fake, _fids, _cl) = FakeServer::new();
    fake.wrong_tag = true; // every reply's tag is off by one
    let (r, w) = fake.split();
    // 19-10 cont 6 re-keyed this pin: the foreign tag is the donor
    // mountmux's DROP arm (logged + dropped, the reader keeps
    // serving), so the negotiating rpc no longer fails with the tag
    // error — it fails when the wire runs dry waiting for the reply
    // that never came (the foreign-tag print rides the log).  The
    // mismatch is still LOUD on the serial log; the SEMANTIC change
    // is that a late reply can no longer drain a healthy waiter (the
    // -G session-exit regression the cont-6 exit-drain exposed).
    let err = Mnt::negotiate(r, w, 0)
        .expect_err("a reply tag that is not the outstanding tag never satisfies the rpc");
    assert!(
        err.contains("eof") || err.contains("tag"),
        "the rpc fails on the missing reply (eof) or the tag violation: {err}"
    );
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
    let (mut fake2, _f, _c2) = FakeServer::new();
    fake2.walk_zero_qid = true;
    let (r2, w2) = fake2.split();
    let mnt2 = Mnt::negotiate(r2, w2, 0).expect("negotiate");
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
/// half, host-side).  WR-04: resolution runs unlocked (namec's per-consult
/// snapshots); the lock covers the mount-table insert only.
fn mount_fake(pg: &Lock<Pgrp>, slash: &Chan, target: &str, flag: u32) -> Chan {
    let (mnt, _fids) = session();
    let root = devmnt::attach(&Arc::new(mnt), b"").expect("attach the fake tree");
    let tgt = namec::namec(target.as_bytes(), slash, slash, pg, NamecMode::MountTarget)
        .expect("the mount target resolves");
    let node = LockNode::new();
    pg.lock(&node).bind(root.dup(), &tgt, flag).expect("the mount inserts");
    root
}

#[test]
fn devmnt_mount_crossing_walk_matrix() {
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let slash = fixture_slash();

    // MAFTER: the underlying fixture dir wins ties; misses fall THROUGH to
    // the mounted tree (the union fallback).
    mount_fake(&pg, &slash, "/dir", pgrp::MAFTER);

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
    let pg2 = Lock::new("pgrp-test", Pgrp::new());
    mount_fake(&pg2, &slash, "/dir", pgrp::MREPL);
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

/// 05-09: the mount-table fid-aliasing regression pin (the 05-09 gate's
/// surfacing: ls /bin's open+close of the redirect Tclunked the mount
/// TABLE's server-side fid under the bare dup, and every later walk from
/// it — ps's exec — Rerror'd).  The redirect must fid-CLONE (the
/// zero-name Twalk idiom), so the redirect's open/close lifecycle never
/// touches the table's fid.
#[test]
fn devmnt_mount_redirect_clones_the_table_fid() {
    let slash = fixture_slash();
    let (mnt, fids) = session();
    let mnt = Arc::new(mnt);
    let root = devmnt::attach(&mnt, b"").expect("attach the fake tree");
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let tgt = namec::namec(b"/dir", &slash, &slash, &pg, NamecMode::MountTarget)
        .expect("the mount target resolves");
    let node = LockNode::new();
    pg.lock(&node).bind(root, &tgt, pgrp::MREPL).expect("the mount inserts");
    let attach_fids: Vec<u32> = fids.borrow().keys().copied().collect();
    assert_eq!(attach_fids.len(), 1, "one attach fid server-side");

    // The redirect's lifecycle: resolve the mount point via Open (the
    // final-component redirect), open the union head, close it.
    let c = namec::namec(b"/dir", &slash, &slash, &pg, NamecMode::Open)
        .expect("the redirect resolves");
    assert_eq!(c.dev, DEV_MNT);
    let opened = namec::open_chan(&c, 0).expect("open the union head OREAD");
    namec::close_chan(&opened); // Tclunk — the CLONE's fid, not the table's

    // The table's attach fid survives server-side (the redirect rode a
    // clone): the fid→QID_ROOT entry is still present.
    assert!(
        fids.borrow().values().any(|q| *q == QID_ROOT),
        "the mount table's fid was never clunked (the redirect cloned it)"
    );

    // And a later walk from the mount entry still works — the gate's
    // failure shape (ps's exec after ls /bin's listing).
    let c2 = namec::namec(b"/dir/top.txt", &slash, &slash, &pg, NamecMode::Open)
        .expect("a walk from the table's fid works after the redirect's open+close");
    let mut o2 = namec::open_chan(&c2, 0).expect("open top.txt");
    let mut buf = [0u8; 64];
    let n = devmnt::read(&mut o2, &mut buf, 0).expect("read top.txt");
    assert_eq!(&buf[..n as usize], TOP_CONTENT);
}

/// WR-06(b): read_all's cap discipline — a file of EXACTLY cap bytes
/// stages cleanly (the final zero-length EOF read terminates the loop;
/// the old top-of-loop check errored it without ever attempting that
/// read), one byte under cap is the loud "too large", and a wide cap
/// stages the same bytes.
#[test]
fn devmnt_read_all_exact_cap_and_over_cap() {
    let (mnt, _fids) = session();
    let root = devmnt::attach(&Arc::new(mnt), b"").expect("attach the fake tree");
    let adir = devmnt::walk(&root, b"adir").expect("walk adir");
    let leaf = devmnt::walk(&adir, b"leaf.txt").expect("walk leaf.txt");
    let opened = namec::open_chan(&leaf, 0).expect("open leaf.txt OREAD");

    let staged = devmnt::read_all(&opened, LEAF_CONTENT.len()).expect("exact-cap stage succeeds");
    assert_eq!(staged, LEAF_CONTENT, "the exact-cap file stages whole");

    let err = devmnt::read_all(&opened, LEAF_CONTENT.len() - 1).expect_err("one byte over cap errors");
    assert!(err.contains("too large"), "the named errstr, got: {err}");

    let staged = devmnt::read_all(&opened, 4096).expect("wide-cap stage succeeds");
    assert_eq!(staged, LEAF_CONTENT);
}

// ---------------------------------------------------------------------------
// 261104-b4p (item 5 — the fid lifecycle): the Arc'd fid's exactly-once
// close dispatch + namec's walk-intermediate sweep.  The fid table maps
// fid → qid path, so fids are named test-side by the node they name.
// ---------------------------------------------------------------------------

/// The fid currently naming `qid` server-side (None when clunked/never
/// minted).
fn fid_for(fids: &Rc<RefCell<HashMap<u32, u64>>>, qid: u64) -> Option<u32> {
    fids.borrow().iter().find(|(_, q)| **q == qid).map(|(f, _)| *f)
}

/// An open-then-dup'd chan closed twice Tclunks EXACTLY ONCE (the last
/// ref only — the value-copy era's double-Tclunk divergence retired):
/// open SHARES the walked fid's Arc, dup clones it, and only the final
/// close dispatches.
#[test]
fn devmnt_dup_close_clunks_exactly_once() {
    let (mnt, fids, clunked) = session_with_clunk_log();
    let mnt = Arc::new(mnt);
    let root = devmnt::attach(&mnt, b"").expect("attach the fake tree");

    let adir = devmnt::walk(&root, b"adir").expect("walk adir");
    let adir_fid = fid_for(&fids, QID_ADIR).expect("the server holds the adir fid");

    // open shares the walked fid (open-doesn't-realloc); the walked-past
    // intermediate's close dispatch is NOT the last ref — no Tclunk.
    let opened = namec::open_chan(&adir, 0).expect("open adir OREAD");
    namec::close_chan(&adir);
    assert!(
        fids.borrow().contains_key(&adir_fid),
        "the intermediate's close never clunks a shared fid"
    );
    drop(adir); // the walked chan leaves scope (the Arc decrements)

    // The fd-table dup (sys_dup/rfork's ref_clone) is a SECOND reference
    // on the same fid, never a new fid: the first close must not Tclunk.
    // (The kernel pops the chan out of its fd slot and the VALUE drops
    // right after the dispatch — mirror that: drop each closed chan so
    // the next close sees the true remaining count.)
    let dup = opened.dup();
    namec::close_chan(&opened);
    assert!(
        fids.borrow().contains_key(&adir_fid),
        "the first of two closes is not the last ref — no Tclunk"
    );
    drop(opened);
    namec::close_chan(&dup);
    assert!(!fids.borrow().contains_key(&adir_fid), "the LAST ref's close Tclunks");
    assert_eq!(
        clunked.borrow().iter().filter(|f| **f == adir_fid).count(),
        1,
        "exactly one Tclunk for the shared fid (T-b4p-01's pin)"
    );

    // The attach (mount-root) fid is untouched throughout.
    assert!(fid_for(&fids, QID_ROOT).is_some(), "the attach fid survives");
}

/// A two-component namec walk through a mounted tree leaves NO
/// intermediate fids server-side while the final fid lives (the walk
/// intermediates' leak retired — swept on the SUCCESS exit).
#[test]
fn devmnt_namec_walk_sweeps_intermediate_fids() {
    let slash = fixture_slash();
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let (mnt, fids, clunked) = session_with_clunk_log();
    let root = devmnt::attach(&Arc::new(mnt), b"").expect("attach the fake tree");
    let tgt = namec::namec(b"/dir", &slash, &slash, &pg, NamecMode::MountTarget)
        .expect("the mount target resolves");
    let node = LockNode::new();
    pg.lock(&node).bind(root, &tgt, pgrp::MREPL).expect("the mount inserts");
    let attach_fid = fid_for(&fids, QID_ROOT).expect("the table holds the attach fid");

    let leaf = namec::namec(b"/dir/adir/leaf.txt", &slash, &slash, &pg, NamecMode::Open)
        .expect("the crossed file resolves");
    assert_eq!(leaf.dev, DEV_MNT);
    let leaf_fid = fid_for(&fids, QID_LEAF).expect("the final fid lives server-side");
    assert!(
        !fids.borrow().values().any(|q| *q == QID_ADIR),
        "the walk intermediate's fid was clunked on the success exit (no server-side leak)"
    );
    assert_eq!(
        fids.borrow().len(),
        2,
        "only the mount table's attach fid + the returned chan's fid remain"
    );
    assert!(fids.borrow().contains_key(&attach_fid), "the mount table's fid is never touched");
    assert!(fids.borrow().contains_key(&leaf_fid));
    // The intermediate's clunk is recorded (the sweep really Tclunk'd —
    // not merely dropped).
    let adir_clunks = {
        let log = clunked.borrow();
        let live: Vec<u32> = fids.borrow().keys().copied().collect();
        log.iter().filter(|f| !live.contains(f)).count()
    };
    assert!(adir_clunks >= 1, "the sweep dispatched a real Tclunk for the intermediate");
}

/// A mid-walk not-found Err clunks the intermediates minted so far (the
/// sweep fires on EVERY exit — Err alike).
#[test]
fn devmnt_namec_walk_error_clunks_intermediates() {
    let slash = fixture_slash();
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let (mnt, fids, _clunked) = session_with_clunk_log();
    let root = devmnt::attach(&Arc::new(mnt), b"").expect("attach the fake tree");
    let tgt = namec::namec(b"/dir", &slash, &slash, &pg, NamecMode::MountTarget)
        .expect("the mount target resolves");
    let node = LockNode::new();
    pg.lock(&node).bind(root, &tgt, pgrp::MREPL).expect("the mount inserts");
    let attach_fid = fid_for(&fids, QID_ROOT).expect("the table holds the attach fid");

    // adir resolves, then "nosuch" fails — the adir intermediate was
    // minted before the error and must be clunked on the Err exit.
    assert!(
        namec::namec(b"/dir/adir/nosuch", &slash, &slash, &pg, NamecMode::Open).is_err(),
        "a missing final component errors"
    );
    assert!(
        !fids.borrow().values().any(|q| *q == QID_ADIR),
        "the mid-walk intermediate was clunked on the error exit"
    );
    assert!(fids.borrow().contains_key(&attach_fid), "the mount table's fid is never touched");
    assert_eq!(fids.borrow().len(), 1, "only the table's attach fid remains");
}

// ---------------------------------------------------------------------------
// 19-08: the tag-mux pins — concurrent rpcs demux by tag, the exact
// class the -G session needs (a parked indefinite reader must keep
// serving a concurrent rpc's reply).
// ---------------------------------------------------------------------------

/// A thread-safe scripted server whose READ half blocks until bytes
/// arrive (the kernel pipe shape — the Condvar stands in for the
/// park/wake hooks; the 10s cap keeps a broken mux a loud failure,
/// never a hung suite).  `write_msg` handles the T synchronously.
///
/// The out-of-order knob (`stash_first`): the reply to the FIRST Tread
/// is stashed and released only when the SECOND Twrite arrives — the
/// wire then holds the second rpc's reply BEFORE the first's, so the
/// first rpc's reader (parked on the wire, holding the reader token)
/// must frame and ROUTE a reply it does not own before its own
/// arrives.
struct ThreadFakeState {
    outq: VecDeque<u8>,
    /// fid → content (the walk set).
    files: HashMap<u32, Vec<u8>>,
    /// T writes handled (the interleave observable).
    writes: usize,
    /// Contexts currently parked inside read_some.
    reads_waiting: usize,
    stash_first: bool,
    stashed: Option<Vec<u8>>,
    /// The tags queued onto the wire, in order (the reorder assert).
    queued_tags: Vec<u16>,
}

struct ThreadFake(Arc<(Mutex<ThreadFakeState>, Condvar)>);

impl ThreadFake {
    fn clone_fake(&self) -> ThreadFake {
        ThreadFake(self.0.clone())
    }

    fn new() -> (ThreadFake, Arc<(Mutex<ThreadFakeState>, Condvar)>) {
        let st = Arc::new((
            Mutex::new(ThreadFakeState {
                outq: VecDeque::new(),
                files: HashMap::new(),
                writes: 0,
                reads_waiting: 0,
                stash_first: false,
                stashed: None,
                queued_tags: Vec::new(),
            }),
            Condvar::new(),
        ));
        (ThreadFake(st.clone()), st)
    }

    /// One canned reply: Rversion/Rattach/Rwalk by type; Tread serves
    /// the walked fid's content.  fid mapping is fixed by the test.
    fn handle(state: &mut ThreadFakeState, buf: &[u8]) -> Vec<u8> {
        let size = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
        let body = &buf[7..size];
        let typ = buf[4];
        let tag = u16::from_le_bytes(buf[5..7].try_into().unwrap());
        match typ {
            100 => {
                // Tversion -> Rversion(offer, 9P2000)
                let mut b = Vec::new();
                pu32(&mut b, 8192);
                pstr(&mut b, b"9P2000");
                frame(101, tag, &b)
            }
            104 => {
                // Tattach -> Rattach(dir qid); fid 1 = root (the fid
                // word is positional — the mapping is fixed by the
                // test's walk order)
                state.files.insert(1, Vec::new());
                let mut b = Vec::new();
                pqid(&mut b, 0x80, 0, 100);
                frame(105, tag, &b)
            }
            110 => {
                // Twalk(one name) -> Rwalk(one qid); the CONTENT maps
                // by NAME onto the new fid (the source fid word is
                // positional — the test's walk order fixes it).
                let newfid = u32::from_le_bytes(body[4..8].try_into().unwrap());
                let nlen = u16::from_le_bytes(body[10..12].try_into().unwrap()) as usize;
                let name = &body[12..12 + nlen];
                let content = match name {
                    b"adir" => Vec::new(),
                    b"leaf.txt" => LEAF_CONTENT.to_vec(),
                    b"top.txt" => TOP_CONTENT.to_vec(),
                    _ => Vec::new(),
                };
                state.files.insert(newfid, content);
                let mut b = Vec::new();
                pu16(&mut b, 1);
                pqid(&mut b, 0x00, 0, 102);
                frame(111, tag, &b)
            }
            116 => {
                // Tread -> Rread(content)
                let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
                let off = u64::from_le_bytes(body[4..12].try_into().unwrap());
                let count = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;
                let content = state.files.get(&fid).cloned().unwrap_or_default();
                let data = &content[off.min(content.len() as u64) as usize..(off as usize + count).min(content.len())];
                let mut b = Vec::new();
                pu32(&mut b, data.len() as u32);
                b.extend_from_slice(data);
                frame(117, tag, &b)
            }
            _ => frame(107, tag, b"bad type"),
        }
    }

    /// Queue one reply onto the wire (raw — the stash decision is
    /// made by the caller, which knows whether the reply is FRESH or
    /// a RELEASE; the release must never re-stash).
    fn queue(state: &mut ThreadFakeState, reply: Vec<u8>) {
        let tag = u16::from_le_bytes(reply[5..7].try_into().unwrap());
        state.queued_tags.push(tag);
        state.outq.extend(reply);
    }
}

impl MntIo for ThreadFake {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        {
            let mut st = self.0.0.lock().unwrap();
            st.writes += 1;
            let reply = Self::handle(&mut st, buf);
            // The stash trigger fires on FRESH replies only (the
            // first Rread after arming) — and the release fires only
            // when the stash PREDATES this write (otherwise the
            // stashing write would immediately release its own
            // stash and A would never park).
            let mut just_stashed = false;
            if st.stash_first && st.stashed.is_none() && reply[4] == 117 {
                st.stashed = Some(reply);
                just_stashed = true;
            } else {
                Self::queue(&mut st, reply);
            }
            // A LATER write releases the stash AFTER this reply
            // queues (the wire then holds [this reply, the stashed
            // one]).
            if !just_stashed && let Some(stash) = st.stashed.take() {
                Self::queue(&mut st, stash);
            }
        }
        self.0.1.notify_all();
        Ok(buf.len() as u64)
    }

    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        let mut st = self.0.0.lock().unwrap();
        loop {
            if !st.outq.is_empty() {
                let n = st.outq.len().min(buf.len());
                for b in buf.iter_mut().take(n) {
                    *b = st.outq.pop_front().unwrap();
                }
                return Ok(n as u64);
            }
            st.reads_waiting += 1;
            let (guard, _timeout) = self.0.1
                .wait_timeout(st, std::time::Duration::from_secs(10))
                .unwrap();
            st = guard;
            st.reads_waiting -= 1;
            if st.outq.is_empty() {
                // The cap: an empty wire after a full timeout is the
                // server hanging up (the client's loud eof error).
                return Ok(0);
            }
        }
    }
}

/// The park hook for the threaded pin: sleep ~1ms (the kernel's
/// park-a-tick; without a hook, contention is the loud error).
fn test_park_one_tick() {
    std::thread::sleep(std::time::Duration::from_millis(1));
}

/// THE mux pin: rpc A parks on the wire (reader token held, reply
/// stashed — the cpunote/console-stdin shape); rpc B sends and its
/// reply arrives FIRST on the wire.  B must receive exactly its own
/// reply — routed by A — and A must then receive its own.  Out-of-
/// order delivery is proven from the server's own queue record.
#[test]
fn mux_parked_reader_routes_a_concurrent_rpcs_reply_before_its_own() {
    devmnt::set_contention_hook_for_test(test_park_one_tick);
    let (fake, st) = ThreadFake::new();
    let (r, w) = (Box::new(fake.clone_fake()), Box::new(fake));
    let mnt = Mnt::negotiate(r, w, 0).expect("negotiate over the threaded fake");

    let (_root_fid, _) = mnt.attach(b"glenda", b"").expect("attach");
    let (adir, _) = mnt.walk(1, b"adir").expect("walk adir");
    let (leaf_fid, _) = mnt.walk(adir, b"leaf.txt").expect("walk leaf");
    let (top_fid, _) = mnt.walk(adir, b"top.txt").expect("walk top");

    // Arm the stash: the NEXT Tread's reply (A's) is held until the
    // following write (B's Tread) — then the wire order is
    // [B's reply, A's reply].
    {
        let mut s = st.0.lock().unwrap();
        s.stash_first = true;
        s.queued_tags.clear();
    }

    let mnt = Arc::new(mnt);
    let mnt_a = mnt.clone();
    let a = std::thread::spawn(move || mnt_a.read(leaf_fid, 0, 4096).expect("A reads its leaf"));

    // Wait until A has SENT its Tread and is parked inside read_some
    // holding the reader token (the deterministic interleave).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        {
            let s = st.0.lock().unwrap();
            if s.writes >= 1 && s.reads_waiting >= 1 {
                break;
            }
        }
        assert!(std::time::Instant::now() < deadline, "A never parked on the wire");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    // B's rpc: sends while A holds the token; the wire answers B
    // FIRST.  B parks on the token (the 1ms hook) until A routes it.
    let b = mnt.read(top_fid, 0, 4096).expect("B reads its top");
    assert_eq!(b, TOP_CONTENT, "B receives exactly its own reply, never A's");

    let a_out = a.join().expect("A completes");
    assert_eq!(a_out, LEAF_CONTENT, "A receives its own reply after routing B's");

    // The wire really did answer B first (the reorder, from the
    // server's own queue record — the pin's premise, proven).
    {
        let s = st.0.lock().unwrap();
        assert_eq!(s.queued_tags.len(), 2, "two replies crossed the wire");
        assert!(s.queued_tags[0] != s.queued_tags[1], "the two replies are distinct rpcs");
    }
}

/// 19-10 cont 6: the donor mountmux's DROP arm — a late reply whose
/// owner is gone (an interrupted exit-time clunk's answer) is logged
/// and dropped, never fatal: the reader keeps serving the live
/// waiters.  RED against the loud-error route (the -G gate's measured
/// session-exit regression: the late reply drained a HEALTHY rpc).
#[test]
fn mux_foreign_reply_is_dropped_and_live_waiters_still_complete() {
    let (fake, st) = ThreadFake::new();
    let (r, w) = (Box::new(fake.clone_fake()), Box::new(fake));
    let mnt = Mnt::negotiate(r, w, 0).expect("negotiate");

    let (_root_fid, _) = mnt.attach(b"glenda", b"").expect("attach");
    let (leaf_fid, _) = mnt.walk(1, b"adir").and_then(|(f, _)| Mnt::walk(&mnt, f, b"leaf.txt")).expect("walk leaf");

    // Queue a FOREIGN reply (a tag nobody owns — the interrupted
    // clunk's late answer) directly onto the fake's wire, THEN run a
    // real read: the reader frames the foreign reply first, must DROP
    // it, and must still deliver the real read's answer.
    {
        let mut s = st.0.lock().unwrap();
        let foreign = frame(119, 0xBEEF, &[0u8; 4][..]); // Rwrite-shaped, unknown tag
        ThreadFake::queue(&mut s, foreign);
    }
    let out = mnt.read(leaf_fid, 0, 4096).expect("the live read completes past the dropped reply");
    assert_eq!(out, LEAF_CONTENT);
}

// ---------------------------------------------------------------------------
// 20-14: the DEV_MNT remove leg (devmnt::remove via namec::remove_chan)
// ---------------------------------------------------------------------------

/// A mounted fake with the clunk and remove logs exposed, bound at /dir.
fn mounted_with_remove_log() -> (
    Chan,
    Lock<Pgrp>,
    Rc<RefCell<HashMap<u32, u64>>>,
    Rc<RefCell<Vec<u32>>>,
    Rc<RefCell<Vec<(u32, u64)>>>,
) {
    let slash = fixture_slash();
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let (fake, fids, clunked) = FakeServer::new();
    let removed = Rc::clone(&fake.removed);
    let (r, w) = fake.split();
    let mnt = Mnt::negotiate(r, w, 0).expect("the version handshake negotiates");
    let root = devmnt::attach(&Arc::new(mnt), b"").expect("attach the fake tree");
    let tgt = namec::namec(b"/dir", &slash, &slash, &pg, NamecMode::MountTarget)
        .expect("the mount target resolves");
    let node = LockNode::new();
    pg.lock(&node).bind(root, &tgt, pgrp::MREPL).expect("the mount inserts");
    (slash, pg, fids, clunked, removed)
}

/// sysremove through a mount: the Tremove rides a CLONE of the walked
/// chan's fid (Tremove consumes the fid it is handed), so the chan's own
/// fid is still live after the remove and close_chan Tclunks it exactly
/// once; the clone is never Tclunk'd by the client (the server already
/// released it).  No fid leaks either side.
#[test]
fn devmnt_remove_rides_a_clone_and_close_clunks_the_walked_fid_once() {
    let (slash, pg, fids, clunked, removed) = mounted_with_remove_log();
    let attach_fid = fid_for(&fids, QID_ROOT).expect("the table holds the attach fid");
    let leaf = namec::namec(b"/dir/adir/leaf.txt", &slash, &slash, &pg, NamecMode::Open)
        .expect("the crossed file resolves");
    assert_eq!(leaf.dev, DEV_MNT);
    let leaf_fid = fid_for(&fids, QID_LEAF).expect("the walked fid lives server-side");

    namec::remove_chan(&leaf).expect("the DEV_MNT arm sends a Tremove the server accepts");
    let log = removed.borrow().clone();
    assert_eq!(log.len(), 1, "exactly one Tremove");
    assert_eq!(log[0].1, QID_LEAF, "the Tremove named the walked file");
    assert_ne!(log[0].0, leaf_fid, "the Tremove rode a clone, not the chan's own fid");
    assert!(fids.borrow().contains_key(&leaf_fid), "the chan's own fid survives the remove");

    namec::close_chan(&leaf);
    drop(leaf);
    assert_eq!(
        clunked.borrow().iter().filter(|f| **f == leaf_fid).count(),
        1,
        "the walked fid is Tclunk'd exactly once, at close"
    );
    assert!(!clunked.borrow().contains(&log[0].0), "the consumed clone is never Tclunk'd");
    assert_eq!(fids.borrow().len(), 1, "only the mount table's attach fid remains");
    assert!(fids.borrow().contains_key(&attach_fid));
}

/// A remove the server refuses is the loud error (devmnt's Rerror arm),
/// never a silent 0, and still leaks no fid: the server clunked the
/// clone, and close_chan clunks the walked fid.
#[test]
fn devmnt_remove_refused_is_loud_and_leaks_no_fid() {
    let (slash, pg, fids, _clunked, removed) = mounted_with_remove_log();
    let adir = namec::namec(b"/dir/adir", &slash, &slash, &pg, NamecMode::Open)
        .expect("the crossed directory resolves");
    let before = fids.borrow().len();
    assert_eq!(namec::remove_chan(&adir), Err("devmnt: server error"));
    assert!(removed.borrow().is_empty(), "the server accepted nothing");
    assert_eq!(fids.borrow().len(), before, "the refused Tremove's clone is gone server-side");
    namec::close_chan(&adir);
    drop(adir);
    assert_eq!(fids.borrow().len(), 1, "only the mount table's attach fid remains");
}
