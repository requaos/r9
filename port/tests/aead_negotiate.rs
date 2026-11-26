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

// ---------------------------------------------------------------------------
// One whole sealed session, from the real negotiation to teardown (WIRE-05)
// ---------------------------------------------------------------------------

use crypto::key::Key;
use common::aead_link::{
    CLIENT_EXCHANGE, E2E_FILE_BYTES, E2E_FILE_NAME, E2eClient, E2eOpened, E2eServer, SERVER_EXCHANGE,
    Stage, TestExchange, body_of as fixture_body, config_with_rekey, connect_aead,
    connect_aead_shared, connect_aead_with,
};
use port::chan::PAGE_LEN;
use port::chan::msg::Msg;
use port::chan::tlv::Exts;
use port::chan::transport::{RekeyIncoming, is_rekey_traffic};
use port::chan::wire;
use port::devmnt::NOFID;
use port::fid::{FidTable, StatefulHandle};

/// The fixed 32-byte session secret both ends are handed by the test seam
/// (D-16: no real exchange exists in this phase).
fn secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = i as u8;
    }
    Key::from_bytes(bytes)
}

/// The re-key's second secret (a fresh exchange derives generation 1).
fn second_secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = 0x40 + i as u8;
    }
    Key::from_bytes(bytes)
}

/// Seal one request, let the server handle it and open the reply.
fn request(
    client: &mut E2eClient,
    server: &Rc<RefCell<E2eServer>>,
    typ: u8,
    handle: [u8; 8],
    exts: &Exts<'_>,
    message: &Msg,
) -> E2eOpened {
    let tag = 1;
    client
        .send(typ, tag, handle, exts, &fixture_body(message))
        .expect("the client seals the request");
    let mut exchange = TestExchange::new(Vec::new());
    server
        .borrow_mut()
        .handle_one(&mut exchange, 0)
        .expect("the server handles the request");
    client.recv().expect("the client opens the reply")
}

/// Attach, walk the one file and open it; returns the file's handle.
fn open_the_file(client: &mut E2eClient, server: &Rc<RefCell<E2eServer>>) -> StatefulHandle {
    let root = request(
        client,
        server,
        wire::TATTACH,
        [0u8; 8],
        &Exts::default(),
        &Msg::Tattach {
            fid: 1,
            afid: NOFID,
            uname: b"glenda".to_vec(),
            aname: Vec::new(),
        },
    );
    assert!(matches!(root.msg, Msg::Rattach { .. }), "the attach reply is Rattach");
    let root_handle =
        StatefulHandle::from_wire(&root.handle).expect("the attach reply carries a stateful handle");

    let walked = request(
        client,
        server,
        wire::TWALK,
        root_handle.to_wire(),
        &Exts::default(),
        &Msg::Twalk {
            fid: 1,
            newfid: 2,
            wname: vec![E2E_FILE_NAME.to_vec()],
        },
    );
    assert!(matches!(walked.msg, Msg::Rwalk { .. }), "the walk reply is Rwalk");
    let file_handle =
        StatefulHandle::from_wire(&walked.handle).expect("the walk reply carries the new handle");
    request(
        client,
        server,
        wire::TOPEN,
        file_handle.to_wire(),
        &Exts::default(),
        &Msg::Topen { fid: 2, mode: 0 },
    );
    file_handle
}

#[test]
fn aead_negotiate_end_to_end_session() {
    let (mut client, server, _link) = connect_aead(secret(), 1);
    let file_handle = open_the_file(&mut client, &server);

    let read = request(
        &mut client,
        &server,
        wire::TREAD,
        file_handle.to_wire(),
        &Exts::default(),
        &Msg::Tread {
            fid: 2,
            offset: 0,
            count: 4096,
        },
    );
    let Msg::Rread { data } = read.msg else {
        panic!("the read reply is Rread");
    };
    assert_eq!(data, E2E_FILE_BYTES, "the sealed read returns the canned file's bytes");
    assert_eq!(
        read.body_len,
        4 + PAGE_LEN,
        "Rread is one padded page with the true count in its parameters (D-09, P5)"
    );

    let clunked = request(
        &mut client,
        &server,
        wire::TCLUNK,
        file_handle.to_wire(),
        &Exts::default(),
        &Msg::Tclunk { fid: 2 },
    );
    assert!(matches!(clunked.msg, Msg::Rclunk), "the clunk reply is Rclunk");

    client.session.teardown();
    server.borrow_mut().teardown();
    assert!(client.session.is_closed(), "the client end closes on teardown");
    assert!(server.borrow().is_closed(), "the server end closes on teardown");
    assert!(client.session.keys_zeroized(), "the orderly teardown zeroizes the client's keys");
}

#[test]
fn aead_negotiate_end_to_end_stateless_capability() {
    let (mut client, server, _link) = connect_aead(secret(), 1);
    let file_handle = open_the_file(&mut client, &server);

    // The open reply carried a capability in EXT_STATELESS_CAP with an all-zero
    // header handle (D-11, D-12).
    let opened = request(
        &mut client,
        &server,
        wire::TOPEN,
        file_handle.to_wire(),
        &Exts::default(),
        &Msg::Topen { fid: 2, mode: 0 },
    );
    assert!(matches!(opened.msg, Msg::Ropen { .. }), "the open reply is Ropen");
    let cap = opened.cap.expect("the open reply carries a capability");
    assert_eq!(opened.handle, [0u8; 8], "a capability packet's header handle is all zero (D-12)");
    let before = server.borrow().fid_memory();

    let exts = Exts {
        stateless_cap: Some(&cap),
        ..Exts::default()
    };
    let read = request(
        &mut client,
        &server,
        wire::TREAD,
        [0u8; 8],
        &exts,
        &Msg::Tread {
            fid: 0,
            offset: 0,
            count: 4096,
        },
    );
    let Msg::Rread { data } = read.msg else {
        panic!("the stateless read reply is Rread");
    };
    assert_eq!(data, E2E_FILE_BYTES, "the capability names the canned file");
    assert_eq!(
        server.borrow().fid_memory(),
        before,
        "a capability is verified with zero FID memory (WIRE-08, P8)"
    );

    // One flipped byte fails verification, and P8 closes the connection: no
    // reply is sealed for the bad packet.
    let mut bad = cap;
    bad[4] ^= 0xFF;
    let exts = Exts {
        stateless_cap: Some(&bad),
        ..Exts::default()
    };
    client
        .send(
            wire::TREAD,
            2,
            [0u8; 8],
            &exts,
            &fixture_body(&Msg::Tread {
                fid: 0,
                offset: 0,
                count: 4096,
            }),
        )
        .expect("the bad capability packet is sealed");
    let mut exchange = TestExchange::new(Vec::new());
    server
        .borrow_mut()
        .handle_one(&mut exchange, 0)
        .expect("the server consumes the bad packet");
    assert!(server.borrow().is_closed(), "a capability that fails verification closes (P8)");
    assert!(
        matches!(client.recv(), Err(Stage::Empty)),
        "no reply is sealed for the failed capability"
    );
}

#[test]
fn aead_negotiate_end_to_end_survives_a_rekey() {
    let (mut client, server, _link) = connect_aead_with(secret(), 1, config_with_rekey(6, 5000));
    let file_handle = open_the_file(&mut client, &server);
    let before = request(
        &mut client,
        &server,
        wire::TREAD,
        file_handle.to_wire(),
        &Exts::default(),
        &Msg::Tread {
            fid: 2,
            offset: 0,
            count: 4096,
        },
    );
    let Msg::Rread { data: before_bytes } = before.msg else {
        panic!("the read reply is Rread");
    };
    assert_eq!(before_bytes, E2E_FILE_BYTES);

    // Four packets so far (attach, walk, open, read); two more reach the
    // threshold of 6 on both sides.
    for _ in 0..2 {
        request(
            &mut client,
            &server,
            wire::TREAD,
            file_handle.to_wire(),
            &Exts::default(),
            &Msg::Tread {
                fid: 2,
                offset: 0,
                count: 4096,
            },
        );
    }
    assert!(client.session.rekey_due(), "the client's send counter reached the threshold");
    assert!(server.borrow().rekey_due(), "the server's send counter reached the threshold");

    let mut client_exchange =
        TestExchange::with_payload(vec![second_secret()], CLIENT_EXCHANGE.to_vec());
    let mut server_exchange =
        TestExchange::with_payload(vec![second_secret()], SERVER_EXCHANGE.to_vec());
    let now = 0;

    // The in-band exchange: Tauth/Rauth, then the afid write/read round.
    let started = client
        .rekey(&mut client_exchange, None, now)
        .expect("the client starts the exchange");
    assert_eq!(started.len(), 1, "the client sends Tauth sealed");
    let msg = server.borrow_mut().recv_sealed().expect("the server opens Tauth");
    assert!(matches!(msg.msg, Msg::Tauth { .. }));
    server
        .borrow_mut()
        .rekey(&mut server_exchange, Some(&msg.msg), now)
        .expect("the server answers Rauth");

    let msg = client.recv().expect("the client opens Rauth");
    assert!(matches!(msg.msg, Msg::Rauth { .. }));
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client sends Twrite");

    let msg = server.borrow_mut().recv_sealed().expect("the server opens Twrite");
    assert!(matches!(msg.msg, Msg::Twrite { .. }));
    server
        .borrow_mut()
        .rekey(&mut server_exchange, Some(&msg.msg), now)
        .expect("the server answers Rwrite");

    let msg = client.recv().expect("the client opens Rwrite");
    assert!(matches!(msg.msg, Msg::Rwrite { .. }));
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client sends Tread");

    let msg = server.borrow_mut().recv_sealed().expect("the server opens Tread");
    assert!(matches!(msg.msg, Msg::Tread { .. }));
    server
        .borrow_mut()
        .rekey(&mut server_exchange, Some(&msg.msg), now)
        .expect("the server answers Rread");

    let msg = client.recv().expect("the client opens Rread");
    assert!(matches!(msg.msg, Msg::Rread { .. }));
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client derives generation 1");

    // The client cuts its send direction over: its marker, the server's ACK.
    let marker = client
        .rekey(&mut client_exchange, None, now)
        .expect("the client sends its marker");
    assert_eq!(marker.len(), 1, "the c2s marker is the last old-key packet");
    let msg = server.borrow_mut().recv_sealed().expect("the server opens the marker");
    assert!(matches!(msg.msg, Msg::Trekey { .. }));
    server
        .borrow_mut()
        .rekey(&mut server_exchange, Some(&msg.msg), now)
        .expect("the server cuts its receive direction over");
    let ack = client.recv().expect("the client opens the c2s acknowledgement");
    assert!(matches!(ack.msg, Msg::Rrekey { .. }));
    client
        .rekey(&mut client_exchange, Some(&ack.msg), now)
        .expect("the client verifies the acknowledgement");

    // The server cuts its own send direction over.
    let marker = server
        .borrow_mut()
        .rekey(&mut server_exchange, None, now)
        .expect("the server sends its marker");
    assert_eq!(marker.len(), 1, "the s2c marker is the last old-key packet");
    let msg = client.recv().expect("the client opens the s2c marker");
    assert!(matches!(msg.msg, Msg::Rrekey { .. }), "the server's own marker is Rrekey");
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client cuts its receive direction over");
    let ack = server.borrow_mut().recv_sealed().expect("the server opens the c2s ACK");
    assert!(matches!(ack.msg, Msg::Trekey { .. }), "the client's acknowledgement is Trekey");
    server
        .borrow_mut()
        .rekey(&mut server_exchange, Some(&ack.msg), now)
        .expect("the server verifies the acknowledgement");

    // The same session reads the same bytes under the new keys.
    let after = request(
        &mut client,
        &server,
        wire::TREAD,
        file_handle.to_wire(),
        &Exts::default(),
        &Msg::Tread {
            fid: 2,
            offset: 0,
            count: 4096,
        },
    );
    let Msg::Rread { data: after_bytes } = after.msg else {
        panic!("the post-re-key read reply is Rread");
    };
    assert_eq!(after_bytes, before_bytes, "the reads before and after the re-key agree");
}

#[test]
fn aead_negotiate_end_to_end_handle_from_another_connection_is_refused() {
    // One server, ONE slab FidTable, two connections (the slab sizes the table
    // per server, D-12): connection A has client_id 1, connection B has
    // client_id 2.
    let mut domain = [0u8; 32];
    for (i, byte) in domain.iter_mut().enumerate() {
        *byte = 0xE0u8.wrapping_add(i as u8);
    }
    let table = Rc::new(RefCell::new(FidTable::synthetic(Key::from_bytes(domain), 1)));

    // Connection A: one client_id, one entry for fid 1.
    let (mut client_a, server_a, _link_a) = connect_aead_shared(secret(), 1, Rc::clone(&table));
    let attach_a = request(
        &mut client_a,
        &server_a,
        wire::TATTACH,
        [0u8; 8],
        &Exts::default(),
        &Msg::Tattach {
            fid: 1,
            afid: NOFID,
            uname: b"glenda".to_vec(),
            aname: Vec::new(),
        },
    );
    assert!(matches!(attach_a.msg, Msg::Rattach { .. }));
    let foreign =
        StatefulHandle::from_wire(&attach_a.handle).expect("the attach reply carries a handle");

    // Connection B: another client_id on the SAME table, with its own attach.
    let (mut client_b, server_b, _link_b) = connect_aead_shared(secret(), 2, Rc::clone(&table));
    let attach_b = request(
        &mut client_b,
        &server_b,
        wire::TATTACH,
        [0u8; 8],
        &Exts::default(),
        &Msg::Tattach {
            fid: 7,
            afid: NOFID,
            uname: b"glenda".to_vec(),
            aname: Vec::new(),
        },
    );
    assert!(matches!(attach_b.msg, Msg::Rattach { .. }));

    // A's handle, in a validly sealed packet on B: the slot it names is owned
    // by client_id 1, so B's resolve fails the owning-client check (D-08) and
    // the server answers Rerror.
    let reply = request(
        &mut client_b,
        &server_b,
        wire::TREAD,
        foreign.to_wire(),
        &Exts::default(),
        &Msg::Tread {
            fid: 1,
            offset: 0,
            count: 64,
        },
    );
    assert!(
        matches!(reply.msg, Msg::Rerror { .. }),
        "a handle issued to another connection does not resolve (D-08, T-05-09)"
    );

    // B's own handle on the same table still resolves and is served.
    let own = StatefulHandle::from_wire(&attach_b.handle).expect("the attach reply carries a handle");
    let reply = request(
        &mut client_b,
        &server_b,
        wire::TREAD,
        own.to_wire(),
        &Exts::default(),
        &Msg::Tread {
            fid: 7,
            offset: 0,
            count: 64,
        },
    );
    assert!(
        matches!(reply.msg, Msg::Rread { .. }),
        "B's own handle still resolves on the shared table"
    );
}

#[test]
fn aead_negotiate_end_to_end_read_during_rekey_is_served() {
    let (mut client, server, _link) = connect_aead_with(secret(), 1, config_with_rekey(6, 5000));
    let file_handle = open_the_file(&mut client, &server);

    // Three requests so far (attach, walk, open); three more reach the
    // threshold of 6 on both sides.
    for _ in 0..3 {
        let reply = request(
            &mut client,
            &server,
            wire::TREAD,
            file_handle.to_wire(),
            &Exts::default(),
            &Msg::Tread {
                fid: 2,
                offset: 0,
                count: 4096,
            },
        );
        assert!(matches!(reply.msg, Msg::Rread { .. }));
    }
    assert!(client.session.rekey_due(), "the client's send counter reached the threshold");
    assert!(server.borrow().rekey_due(), "the server's send counter reached the threshold");

    let mut client_exchange =
        TestExchange::with_payload(vec![second_secret()], CLIENT_EXCHANGE.to_vec());
    let mut server_exchange =
        TestExchange::with_payload(vec![second_secret()], SERVER_EXCHANGE.to_vec());
    let now = 0;

    // Tauth, routed by the server fixture through `is_rekey_traffic` (P9).
    let started = client
        .rekey(&mut client_exchange, None, now)
        .expect("the client starts the exchange");
    assert_eq!(started.len(), 1, "the client sends Tauth sealed");
    server
        .borrow_mut()
        .handle_one(&mut server_exchange, now)
        .expect("the server routes the Tauth to its re-key state");

    let msg = client.recv().expect("the client opens Rauth");
    assert!(matches!(msg.msg, Msg::Rauth { .. }));
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client sends Twrite");

    // An ordinary file read crosses the exchange in the same direction.
    client
        .send(
            wire::TREAD,
            1,
            file_handle.to_wire(),
            &Exts::default(),
            &fixture_body(&Msg::Tread {
                fid: 2,
                offset: 0,
                count: 4096,
            }),
        )
        .expect("the client seals the ordinary read");

    // The server routes the exchange's Twrite, then serves the ordinary read
    // instead of consuming it.
    server
        .borrow_mut()
        .handle_one(&mut server_exchange, now)
        .expect("the server routes the exchange Twrite");
    server
        .borrow_mut()
        .handle_one(&mut server_exchange, now)
        .expect("the server serves the ordinary read");

    let msg = client.recv().expect("the client opens Rwrite");
    assert!(matches!(msg.msg, Msg::Rwrite { .. }));
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client sends Tread");

    // The ordinary reply reaches the client while it waits for the exchange's
    // Rread, and the router leaves it alone.
    let ordinary = client.recv().expect("the client opens the ordinary Rread");
    assert_eq!(ordinary.tag, 1, "the ordinary reply keeps its own tag");
    assert!(
        !is_rekey_traffic(ordinary.tag, &ordinary.msg),
        "an ordinary reply is never exchange traffic"
    );
    let packets = client
        .session
        .rekey_step(
            &mut client_exchange,
            Some(RekeyIncoming {
                tag: ordinary.tag,
                message: &ordinary.msg,
            }),
            now,
        )
        .expect("the router ignores ordinary traffic");
    assert!(packets.is_empty(), "the router sends nothing for an ordinary reply");
    match ordinary.msg {
        Msg::Rread { data } => assert_eq!(
            data, E2E_FILE_BYTES,
            "the file read is served while the in-band exchange is in flight"
        ),
        other => panic!("expected the ordinary Rread, got {other:?}"),
    }

    // The exchange finishes and both directions cut over.
    server
        .borrow_mut()
        .handle_one(&mut server_exchange, now)
        .expect("the server routes the exchange Tread");
    let msg = client.recv().expect("the client opens the exchange Rread");
    assert!(matches!(msg.msg, Msg::Rread { .. }));
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client derives generation 1");

    let marker = client
        .rekey(&mut client_exchange, None, now)
        .expect("the client sends its marker");
    assert_eq!(marker.len(), 1);
    server
        .borrow_mut()
        .handle_one(&mut server_exchange, now)
        .expect("the server routes the c2s marker");
    let ack = client.recv().expect("the client opens the c2s acknowledgement");
    client
        .rekey(&mut client_exchange, Some(&ack.msg), now)
        .expect("the client verifies the acknowledgement");

    let marker = server
        .borrow_mut()
        .rekey(&mut server_exchange, None, now)
        .expect("the server sends its own marker");
    assert_eq!(marker.len(), 1);
    let msg = client.recv().expect("the client opens the s2c marker");
    client
        .rekey(&mut client_exchange, Some(&msg.msg), now)
        .expect("the client cuts its receive direction over");
    server
        .borrow_mut()
        .handle_one(&mut server_exchange, now)
        .expect("the server routes the s2c acknowledgement");

    // The session survives the re-key and answers under generation 1.
    let after = request(
        &mut client,
        &server,
        wire::TREAD,
        file_handle.to_wire(),
        &Exts::default(),
        &Msg::Tread {
            fid: 2,
            offset: 0,
            count: 4096,
        },
    );
    match after.msg {
        Msg::Rread { data } => assert_eq!(data, E2E_FILE_BYTES, "the read after the re-key succeeds"),
        other => panic!("expected the post-re-key Rread, got {other:?}"),
    }
}
