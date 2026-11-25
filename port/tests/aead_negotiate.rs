//! The `devmnt` version selection point, pinned against the real `devmnt`
//! code (WIRE-12, D-23), and one whole sealed session driven from it
//! (WIRE-05, D-16).
//!
//! D-16 limit: the session secret is injected by a test seam.  No
//! authenticated connection between two real machines exists in this phase;
//! nothing below is a live mount.

mod common;

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use port::Result;
use port::chan::VERSION_AEAD;
use port::dat::Qid;
use port::devmnt::{self, Dialect, Mnt, MntIo, Negotiated};

// ---------------------------------------------------------------------------
// Little-endian wire helpers (built test-side, independent of the codec)
// ---------------------------------------------------------------------------

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
    v.push(typ);
    pu32(v, vers);
    pu64(v, path);
}
/// Wrap a body in the 9P header: size[4] type[1] tag[2] + body.
fn frame(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    pu32(&mut v, (7 + body.len()) as u32);
    v.push(typ);
    pu16(&mut v, tag);
    v.extend_from_slice(body);
    v
}

/// A checked little-endian cursor for the fake's inbound T-messages.
struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Cur<'a> {
        Cur { b, p: 0 }
    }    fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.b[self.p..self.p + 2].try_into().unwrap());
        self.p += 2;
        v
    }
    fn u8(&mut self) -> u8 {
        let v = self.b[self.p];
        self.p += 1;
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
// The fake server: the bytes a real 9P server produces, over the MntIo seam
// ---------------------------------------------------------------------------

const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

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

const QID_ROOT: u64 = 100;
const QID_FILE: u64 = 101;
const FILE_CONTENT: &[u8] = b"aead negotiation file\n";

/// What the fake answers a `Tversion` with.
#[derive(Clone, Copy)]
enum VersionAnswer {
    /// Answer this literal version string whatever was offered.
    Literal(&'static [u8]),
    /// Answer the offered dialect when it is the aead one, plain otherwise
    /// (a dual-stack server).
    DualStack,
}

struct Fake {
    outq: VecDeque<u8>,
    /// fid -> node qid path.
    fids: HashMap<u32, u64>,
    version_answer: VersionAnswer,
    /// Override the answer's msize (the raised and nonsense cases).
    answer_msize: Option<u32>,
    /// Every T-message received, whole, in order.
    requests: Vec<Vec<u8>>,
    /// Every R-message sent, whole, in order.
    sent: Vec<Vec<u8>>,
}

impl Fake {
    fn new() -> Fake {
        Fake {
            outq: VecDeque::new(),
            fids: HashMap::new(),
            version_answer: VersionAnswer::Literal(VERSION_AEAD),
            answer_msize: None,
            requests: Vec::new(),
            sent: Vec::new(),
        }
    }

    fn qid_for(&self, path: u64) -> Qid {
        Qid {
            path,
            vers: 0,
            typ: if path == QID_ROOT { QTDIR } else { QTFILE },
        }
    }

    fn finish(&mut self, typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
        let msg = frame(typ, tag, body);
        self.sent.push(msg.clone());
        msg
    }

    fn rerror(&mut self, tag: u16, ename: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        pstr(&mut body, ename);
        self.finish(RERROR, tag, &body)
    }

    fn handle(&mut self, msg: &[u8]) -> Vec<u8> {
        self.requests.push(msg.to_vec());
        let mut c = Cur::new(msg);
        let _size = c.u32();
        let typ = c.u8();
        let tag = c.u16();
        match typ {
            TVERSION => {
                let offer = c.u32();
                let offered = c.string();
                let answer_msize = self.answer_msize.unwrap_or(offer);
                let version = match self.version_answer {
                    VersionAnswer::Literal(v) => v.to_vec(),
                    VersionAnswer::DualStack => {
                        if offered == VERSION_AEAD { VERSION_AEAD.to_vec() } else { b"9P2000".to_vec() }
                    }
                };
                let mut body = Vec::new();
                pu32(&mut body, answer_msize);
                pstr(&mut body, &version);
                self.finish(RVERSION, tag, &body)
            }
            TATTACH => {
                let fid = c.u32();
                let _afid = c.u32();
                let _uname = c.string();
                let _aname = c.string();
                self.fids.insert(fid, QID_ROOT);
                let mut body = Vec::new();
                pqid(&mut body, QTDIR, 0, QID_ROOT);
                self.finish(RATTACH, tag, &body)
            }
            TWALK => {
                let fid = c.u32();
                let newfid = c.u32();
                let nwname = c.u16();
                let names: Vec<Vec<u8>> = (0..nwname).map(|_| c.string()).collect();
                let Some(start) = self.fids.get(&fid).copied() else {
                    return self.rerror(tag, b"fid not found");
                };
                let mut node = start;
                let mut qids = Vec::new();
                for name in &names {
                    if node != QID_ROOT || name.as_slice() != b"file" {
                        return self.rerror(tag, b"file not found");
                    }
                    node = QID_FILE;
                    qids.push(self.qid_for(node));
                }
                self.fids.insert(newfid, node);
                let mut body = Vec::new();
                pu16(&mut body, qids.len() as u16);
                for qid in &qids {
                    pqid(&mut body, qid.typ, qid.vers, qid.path);
                }
                self.finish(RWALK, tag, &body)
            }
            TOPEN => {
                let fid = c.u32();
                let _mode = c.u8();
                let Some(node) = self.fids.get(&fid).copied() else {
                    return self.rerror(tag, b"fid not found");
                };
                let mut body = Vec::new();
                let qid = self.qid_for(node);
                pqid(&mut body, qid.typ, qid.vers, qid.path);
                pu32(&mut body, 0);
                self.finish(ROPEN, tag, &body)
            }
            TREAD => {
                let fid = c.u32();
                let offset = c.u64();
                let count = c.u32() as usize;
                let Some(node) = self.fids.get(&fid).copied() else {
                    return self.rerror(tag, b"fid not found");
                };
                let source: &[u8] = if node == QID_FILE { FILE_CONTENT } else { b"" };
                let start = (offset as usize).min(source.len());
                let end = (start + count).min(source.len());
                let mut body = Vec::new();
                pu32(&mut body, (end - start) as u32);
                body.extend_from_slice(&source[start..end]);
                self.finish(RREAD, tag, &body)
            }
            TCLUNK => {
                let fid = c.u32();
                self.fids.remove(&fid);
                self.finish(RCLUNK, tag, &[])
            }
            _ => self.rerror(tag, b"bad fcall type"),
        }
    }
}

/// One `MntIo` half over the one shared fake.
struct FakeHalf(Rc<RefCell<Fake>>);

impl MntIo for FakeHalf {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        let reply = self.0.borrow_mut().handle(buf);
        self.0.borrow_mut().outq.extend(reply);
        Ok(buf.len() as u64)
    }
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        let mut f = self.0.borrow_mut();
        let n = f.outq.len().min(buf.len());
        for b in buf.iter_mut().take(n) {
            *b = f.outq.pop_front().unwrap();
        }
        Ok(n as u64)
    }
}

/// The (io_r, io_w) pair over one fake.
fn split(fake: Fake) -> (Box<FakeHalf>, Box<FakeHalf>, Rc<RefCell<Fake>>) {
    let rc = Rc::new(RefCell::new(fake));
    (Box::new(FakeHalf(Rc::clone(&rc))), Box::new(FakeHalf(Rc::clone(&rc))), rc)
}

/// Attach, walk the one file, open it and read it back.
fn exercise(mnt: &Mnt) -> Vec<u8> {
    let (root, qid) = mnt.attach(b"glenda", b"").expect("the plain session attaches");
    assert_eq!(qid.typ & QTDIR, QTDIR, "the attach reply's qid is the root directory");
    let (file_fid, file_qid) = mnt.walk(root, b"file").expect("the plain session walks `file`");
    assert_eq!(file_qid.path, QID_FILE, "the walked qid is the canned file");
    mnt.open(file_fid, 0).expect("the plain session opens the file");
    let data = mnt.read(file_fid, 0, 4096).expect("the plain session reads the file");
    mnt.clunk(file_fid).expect("the plain session clunks the file");
    data
}

// ---------------------------------------------------------------------------
// The plain version string the pre-plan client sent
// ---------------------------------------------------------------------------

#[test]
fn aead_negotiate_aead_offer_reaches_the_engine() {
    let fake = Fake::new();
    let (r, w, fake) = split(fake);
    let negotiated =
        Mnt::negotiate_dialect(r, w, 0, Dialect::Aead).expect("the aead offer reaches the engine");
    let Negotiated::Aead(handoff) = negotiated else {
        panic!("an aead answer must hand the session to the engine");
    };
    assert_eq!(handoff.msize, 8192, "the handoff carries the negotiated msize");
    let sent = fake.borrow();
    assert_eq!(
        handoff.tversion,
        sent.requests[0],
        "the handoff carries the exact Tversion bytes sent"
    );
    assert_eq!(
        handoff.rversion,
        sent.sent[0],
        "the handoff carries the exact Rversion bytes received"
    );
}

#[test]
fn aead_negotiate_plain_answer_to_aead_offer_is_an_error() {
    let mut fake = Fake::new();
    fake.version_answer = VersionAnswer::Literal(b"9P2000");
    let (r, w, _fake) = split(fake);
    let err = match Mnt::negotiate_dialect(r, w, 0, Dialect::Aead) {
        Ok(_) => panic!("a plain answer to an aead offer must fail closed"),
        Err(e) => e,
    };
    assert!(
        err.contains("plain 9P2000"),
        "the downgrade error names the plain answer: {err}"
    );
}

#[test]
fn aead_negotiate_plain_offer_yields_a_working_mnt() {
    for answer in [VersionAnswer::Literal(b"9P2000"), VersionAnswer::DualStack] {
        let mut fake = Fake::new();
        fake.version_answer = answer;
        let (r, w, _fake) = split(fake);
        let negotiated =
            Mnt::negotiate_dialect(r, w, 0, Dialect::Plain).expect("a plain offer negotiates");
        let Negotiated::Plain(mnt) = negotiated else {
            panic!("a plain dialect request must yield a real Mnt");
        };
        assert_eq!(exercise(&mnt), FILE_CONTENT, "the plain-dialect Mnt reads the canned tree");

        let mut fake = Fake::new();
        fake.version_answer = answer;
        let (r, w, _fake) = split(fake);
        let mnt = Mnt::negotiate(r, w, 0).expect("Mnt::negotiate still negotiates plain 9P2000");
        assert_eq!(exercise(&mnt), FILE_CONTENT, "Mnt::negotiate reads the canned tree too");
    }
}

#[test]
fn aead_negotiate_plain_offer_sends_the_plain_version_string() {
    let mut fake = Fake::new();
    fake.version_answer = VersionAnswer::Literal(b"9P2000");
    let (r, w, fake) = split(fake);
    Mnt::negotiate(r, w, 0).expect("the plain offer negotiates");
    let mut body = Vec::new();
    pu32(&mut body, devmnt::DEFAULT_MSIZE);
    pstr(&mut body, devmnt::VERSION9P);
    assert_eq!(
        fake.borrow().requests[0],
        frame(TVERSION, devmnt::NOTAG, &body),
        "a plain offer is byte-for-byte the Tversion the client sent before this plan"
    );
}

#[test]
fn aead_negotiate_unknown_version_is_an_error() {
    for dialect in [Dialect::Plain, Dialect::Aead] {
        let mut fake = Fake::new();
        fake.version_answer = VersionAnswer::Literal(b"unknown");
        let (r, w, _fake) = split(fake);
        let err = match Mnt::negotiate_dialect(r, w, 0, dialect) {
            Ok(_) => panic!("an unknown version must be an error"),
            Err(e) => e,
        };
        assert!(
            err.contains("bad 9P version"),
            "an unknown version is the existing bad-version error: {err}"
        );
    }
}

#[test]
fn aead_negotiate_msize_rules_hold_on_the_aead_branch() {
    for dialect in [Dialect::Plain, Dialect::Aead] {
        // A server that raises the msize above the offer.
        let mut fake = Fake::new();
        fake.answer_msize = Some(devmnt::DEFAULT_MSIZE + 4096);
        let (r, w, _fake) = split(fake);
        let err = match Mnt::negotiate_dialect(r, w, 0, dialect) {
            Ok(_) => panic!("a raised msize must be an error"),
            Err(e) => e,
        };
        assert!(err.contains("msize"), "a raised msize is rejected: {err}");

        // A nonsense msize below the floor.
        let mut fake = Fake::new();
        fake.answer_msize = Some(128);
        let (r, w, _fake) = split(fake);
        let err = match Mnt::negotiate_dialect(r, w, 0, dialect) {
            Ok(_) => panic!("a nonsense msize must be an error"),
            Err(e) => e,
        };
        assert!(err.contains("msize"), "a nonsense msize is rejected: {err}");
    }
}
