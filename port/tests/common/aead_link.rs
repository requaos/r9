//! The in-memory duplex link and the two connection ends for the transport
//! pins (WIRE-03, WIRE-05, WIRE-11; D-15 to D-17).
//!
//! `AeadLink` owns two byte queues (client to server, server to client), a
//! fake clock and the public knobs a test sets before driving an exchange.
//! `connect` drives the plain-framed handshake on both ends and hands back a
//! `ClientEnd` and a `ServerEnd`; `fresh_server` builds one more server end on
//! the same canonical handshake, which is what the every-byte pin needs (one
//! pristine receiver per altered offset).
//!
//! The two ends frame every sealed packet they receive through
//! `Session::frame_len` and deliver it through `Session::receive`; the fixture
//! never turns a framing failure into `Closed` itself.  `Stage` records which
//! call rejected the bytes, and `decode_count` counts only bodies that decoded.

// A shared fixture: each test binary uses a different subset of these helpers.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crypto::key::Key;
use crypto::session::Direction;
use port::chan::msg::{self, Msg};
use port::chan::transport::{
    Closed, ExchangeStep, Handshake, RekeyIncoming, Role, Session, SessionExchange,
    TransportConfig, is_rekey_traffic,
};
use port::chan::tlv::{self, Exts};
use port::chan::wire;
use port::chan::{HEADER_LEN, MAC_LEN, VERSION_AEAD};
use port::dat::Qid;
use port::devmnt::{Dialect, Mnt, MntIo, Negotiated};
use port::fid::{FidEntry, FidHandle, FidTable, StatefulHandle};

/// The client's opaque exchange bytes for the default handshake.
pub const CLIENT_EXCHANGE: &[u8] = b"client exchange bytes";
/// The server's opaque exchange bytes for the default handshake.
pub const SERVER_EXCHANGE: &[u8] = b"server exchange bytes";

/// The auth fid the exchange rides (D-16).
pub const AFID: u32 = 7;

/// The default negotiated msize the fixture's handshakes agree on.
pub const MSIZE: u32 = 8192;

/// The session parameters the fixture's connections use.
pub fn config() -> TransportConfig {
    TransportConfig::default()
}

/// The session parameters with a chosen msize (the two-page wire pin needs a
/// packet larger than the default 8192-byte msize).
pub fn config_with_msize(msize: u32) -> TransportConfig {
    TransportConfig {
        msize,
        ..TransportConfig::default()
    }
}

/// The session parameters with a chosen re-key threshold and ack timeout.
pub fn config_with_rekey(rekey_after_packets: u64, rekey_ack_timeout_ms: u64) -> TransportConfig {
    TransportConfig {
        rekey_after_packets,
        rekey_ack_timeout_ms,
        ..TransportConfig::default()
    }
}

/// Which call rejected a received packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// `Session::frame_len` rejected the `Size` prefix.
    FrameLen,
    /// `Session::receive` (the MAC or a later check) rejected the frame.
    Receive,
    /// The frame opened but `msg::decode_body` rejected its body.
    Decode,
    /// Fewer bytes were queued than a whole frame needs.
    Empty,
}

/// The in-memory duplex link: two byte queues, a fake clock and the knobs.
pub struct AeadLink {
    /// Bytes the client wrote, waiting for the server.
    pub c2s: VecDeque<u8>,
    /// Bytes the server wrote, waiting for the client.
    pub s2c: VecDeque<u8>,
    /// Replace the msize in `Rversion` on the wire, after the server absorbed
    /// its own (correct) copy — models a second party altering the handshake.
    pub alter_rversion_msize: Option<u32>,
    /// Replace the version string in `Rversion` on the wire, likewise.
    pub alter_rversion_version: Option<Vec<u8>>,
    /// XOR the byte at this offset of every packet as it is put on the wire.
    pub flip_byte: Option<usize>,
    /// Replay the last packet instead of the next one (plan 04-16).
    pub replay_last: bool,
    /// Reflect the next packet back to its sender (plan 04-16).
    pub reflect: bool,
    /// Withhold the re-key acknowledgement (plan 04-08).
    pub withhold_rekey_ack: bool,
    /// The fake clock, in milliseconds (D-23: the transport never reads a clock).
    pub now: u64,
    /// Bodies the server end (and any other end) decoded.
    pub decode_count: Rc<RefCell<usize>>,
    /// Every plain-framed handshake message, whole, in wire order.
    pub plain_wire: Vec<Vec<u8>>,
    /// Every frame put on the link, with its direction, plain and sealed alike,
    /// in wire order (plan 04-08's independent wire pin).
    pub wire_log: Vec<(Direction, Vec<u8>)>,
}

impl AeadLink {
    /// A fresh link with empty queues, a zero clock and no knobs.
    pub fn new() -> Rc<RefCell<AeadLink>> {
        Rc::new(RefCell::new(AeadLink {
            c2s: VecDeque::new(),
            s2c: VecDeque::new(),
            alter_rversion_msize: None,
            alter_rversion_version: None,
            flip_byte: None,
            replay_last: false,
            reflect: false,
            withhold_rekey_ack: false,
            now: 0,
            decode_count: Rc::new(RefCell::new(0)),
            plain_wire: Vec::new(),
            wire_log: Vec::new(),
        }))
    }

    /// Advance the fake clock.
    pub fn advance(&mut self, ms: u64) {
        self.now += ms;
    }

    /// How many bodies have decoded.
    pub fn decoded(&self) -> usize {
        *self.decode_count.borrow()
    }
}

impl Default for AeadLink {
    fn default() -> AeadLink {
        AeadLink {
            c2s: VecDeque::new(),
            s2c: VecDeque::new(),
            alter_rversion_msize: None,
            alter_rversion_version: None,
            flip_byte: None,
            replay_last: false,
            reflect: false,
            withhold_rekey_ack: false,
            now: 0,
            decode_count: Rc::new(RefCell::new(0)),
            plain_wire: Vec::new(),
            wire_log: Vec::new(),
        }
    }
}

/// The seam for tests: a fixed list of secrets, one per exchange.
pub struct TestExchange {
    secrets: Vec<Key>,
    index: usize,
    payload: Vec<u8>,
}

impl TestExchange {
    /// An exchange with no opaque bytes of its own.
    pub fn new(secrets: Vec<Key>) -> TestExchange {
        TestExchange {
            secrets,
            index: 0,
            payload: Vec::new(),
        }
    }

    /// An exchange that also contributes `payload` to the wire transcript.
    pub fn with_payload(secrets: Vec<Key>, payload: Vec<u8>) -> TestExchange {
        TestExchange {
            secrets,
            index: 0,
            payload,
        }
    }

    /// How many times the seam was stepped (the handshake is run one; a
    /// re-key is the second).
    pub fn runs(&self) -> usize {
        self.index
    }
}

impl SessionExchange for TestExchange {
    fn step(&mut self, _incoming: &[u8], out: &mut Vec<u8>) -> port::Result<ExchangeStep> {
        let secret = self
            .secrets
            .get(self.index)
            .or_else(|| self.secrets.last())
            .expect("the exchange was given at least one secret")
            .clone();
        self.index += 1;
        out.extend_from_slice(&self.payload);
        Ok(ExchangeStep::Done(secret))
    }
}

/// Frame one plain 9P2000 message: `size[4] type[1] tag[2] body`.
pub fn plain_frame(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(7 + body.len());
    out.extend_from_slice(&((7 + body.len()) as u32).to_le_bytes());
    out.push(typ);
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Encode one message body with the crate's own encoder.
pub fn body_of(message: &Msg) -> Vec<u8> {
    let mut out = Vec::new();
    msg::encode_body(message, &mut out).expect("the fixture's body encodes");
    out
}

/// The `Tauth` frame with its standard 9P2000 body (D-16).
pub fn tauth_frame() -> Vec<u8> {
    plain_frame(
        wire::TAUTH,
        1,
        &body_of(&Msg::Tauth {
            afid: AFID,
            uname: b"none".to_vec(),
            aname: Vec::new(),
        }),
    )
}

/// The `Rauth` frame with its standard 9P2000 body (D-16).
pub fn rauth_frame() -> Vec<u8> {
    plain_frame(
        wire::RAUTH,
        1,
        &body_of(&Msg::Rauth {
            aqid: Qid {
                path: 0,
                vers: 0,
                typ: 0,
            },
        }),
    )
}

/// The canonical plain-framed handshake, in wire order.
///
/// `Tversion`, `Rversion`, `Tauth`, `Rauth`, one auth-fid `Twrite` / `Rwrite`
/// and `Tread` / `Rread` round.  Both ends of a connection absorb exactly these
/// bytes; `connect` records the same list in `AeadLink::plain_wire`.
pub fn handshake_frames() -> Vec<Vec<u8>> {
    let tversion = plain_frame(
        wire::TVERSION,
        0,
        &body_of(&Msg::Tversion {
            msize: MSIZE,
            version: VERSION_AEAD.to_vec(),
        }),
    );
    let rversion = plain_frame(
        wire::RVERSION,
        0,
        &body_of(&Msg::Rversion {
            msize: MSIZE,
            version: VERSION_AEAD.to_vec(),
        }),
    );
    let twrite = plain_frame(
        wire::TWRITE,
        2,
        &body_of(&Msg::Twrite {
            fid: AFID,
            offset: 0,
            data: CLIENT_EXCHANGE.to_vec(),
        }),
    );
    let rwrite = plain_frame(
        wire::RWRITE,
        2,
        &body_of(&Msg::Rwrite {
            count: CLIENT_EXCHANGE.len() as u32,
        }),
    );
    let tread = plain_frame(
        wire::TREAD,
        3,
        &body_of(&Msg::Tread {
            fid: AFID,
            offset: 0,
            count: 65536,
        }),
    );
    let rread = plain_frame(
        wire::RREAD,
        3,
        &body_of(&Msg::Rread {
            data: SERVER_EXCHANGE.to_vec(),
        }),
    );
    vec![
        tversion,
        rversion,
        tauth_frame(),
        rauth_frame(),
        twrite,
        rwrite,
        tread,
        rread,
    ]
}

/// A session built directly from the canonical handshake, without a link.
pub fn session(role: Role, secret: Key) -> Session {
    let mut handshake = Handshake::new(role, config());
    for frame in handshake_frames() {
        handshake.absorb_plain(&frame);
    }
    handshake.finish(secret)
}

/// A session that absorbed only `Tversion` / `Rversion`, so the exchange never
/// finished: it must refuse to seal or open (D-16).
pub fn unauthenticated_session(role: Role, secret: Key) -> Session {
    let mut handshake = Handshake::new(role, config());
    for frame in handshake_frames().into_iter().take(2) {
        handshake.absorb_plain(&frame);
    }
    handshake.finish(secret)
}

/// Apply the `Rversion` knobs to the bytes on their way to the client.
fn alter_rversion(link: &Rc<RefCell<AeadLink>>, mut frame: Vec<u8>) -> Vec<u8> {
    let guard = link.borrow();
    if let Some(msize) = guard.alter_rversion_msize {
        frame[7..11].copy_from_slice(&msize.to_le_bytes());
    }
    if let Some(version) = &guard.alter_rversion_version {
        let msize = u32::from_le_bytes(
            frame[7..11]
                .try_into()
                .expect("the Rversion body starts with four msize bytes"),
        );
        frame = plain_frame(
            wire::RVERSION,
            0,
            &body_of(&Msg::Rversion {
                msize,
                version: version.clone(),
            }),
        );
    }
    frame
}

/// Drive the plain-framed handshake on both ends and return the two ends.
///
/// The default exchange contributes [`CLIENT_EXCHANGE`] and
/// [`SERVER_EXCHANGE`] to the transcript; `connect_with` takes the seams.
pub fn connect(
    link: Rc<RefCell<AeadLink>>,
    client_secret: Key,
    server_secret: Key,
) -> (ClientEnd, ServerEnd) {
    let client = TestExchange::with_payload(vec![client_secret], CLIENT_EXCHANGE.to_vec());
    let server = TestExchange::with_payload(vec![server_secret], SERVER_EXCHANGE.to_vec());
    connect_with(link, client, server)
}

/// Drive the handshake with caller-supplied seams.
pub fn connect_with<C: SessionExchange, S: SessionExchange>(
    link: Rc<RefCell<AeadLink>>,
    mut client_exchange: C,
    mut server_exchange: S,
) -> (ClientEnd, ServerEnd) {
    connect_with_kept(link, &mut client_exchange, &mut server_exchange)
}

/// Drive the handshake with borrowed seams, so the caller keeps them for the
/// in-band re-key (the same seam object serves the second exchange).
pub fn connect_with_kept<C: SessionExchange, S: SessionExchange>(
    link: Rc<RefCell<AeadLink>>,
    client_exchange: &mut C,
    server_exchange: &mut S,
) -> (ClientEnd, ServerEnd) {
    connect_with_kept_config(link, client_exchange, server_exchange, config())
}

/// Drive the handshake with borrowed seams and a chosen transport config.
pub fn connect_with_kept_config<C: SessionExchange, S: SessionExchange>(
    link: Rc<RefCell<AeadLink>>,
    client_exchange: &mut C,
    server_exchange: &mut S,
    transport: TransportConfig,
) -> (ClientEnd, ServerEnd) {
    let mut client = Handshake::new(Role::Client, transport);
    let mut server = Handshake::new(Role::Server, transport);

    // Tversion: client to server.
    let tversion = plain_frame(
        wire::TVERSION,
        0,
        &body_of(&Msg::Tversion {
            msize: MSIZE,
            version: VERSION_AEAD.to_vec(),
        }),
    );
    absorb(&link, &mut client, &mut server, &tversion, Direction::ClientToServer);

    // Rversion: the server absorbs the correct bytes; the client absorbs what
    // arrives, which the knobs may have altered.
    let rversion = plain_frame(
        wire::RVERSION,
        0,
        &body_of(&Msg::Rversion {
            msize: MSIZE,
            version: VERSION_AEAD.to_vec(),
        }),
    );
    server.absorb_plain(&rversion);
    let delivered = alter_rversion(&link, rversion);
    client.absorb_plain(&delivered);
    link.borrow_mut().plain_wire.push(delivered.clone());
    link.borrow_mut().wire_log.push((Direction::ServerToClient, delivered));

    // Tauth / Rauth with their standard bodies.
    absorb(&link, &mut client, &mut server, &tauth_frame(), Direction::ClientToServer);
    let rauth = rauth_frame();
    server.absorb_plain(&rauth);
    client.absorb_plain(&rauth);
    link.borrow_mut().plain_wire.push(rauth.clone());
    link.borrow_mut().wire_log.push((Direction::ServerToClient, rauth));

    // Exchange round: the client's bytes ride Twrite, the server's ride Rread.
    let mut client_out = Vec::new();
    let client_step = client_exchange
        .step(&[], &mut client_out)
        .expect("the client exchange starts");
    let ExchangeStep::Done(client_secret) = client_step else {
        panic!("the fixture's client exchange finishes in one step");
    };
    let twrite = plain_frame(
        wire::TWRITE,
        2,
        &body_of(&Msg::Twrite {
            fid: AFID,
            offset: 0,
            data: client_out.clone(),
        }),
    );
    absorb(&link, &mut client, &mut server, &twrite, Direction::ClientToServer);
    let rwrite = plain_frame(
        wire::RWRITE,
        2,
        &body_of(&Msg::Rwrite {
            count: client_out.len() as u32,
        }),
    );
    server.absorb_plain(&rwrite);
    client.absorb_plain(&rwrite);
    link.borrow_mut().plain_wire.push(rwrite.clone());
    link.borrow_mut().wire_log.push((Direction::ServerToClient, rwrite));

    let mut server_out = Vec::new();
    let server_step = server_exchange
        .step(&client_out, &mut server_out)
        .expect("the server exchange answers");
    let ExchangeStep::Done(server_secret) = server_step else {
        panic!("the fixture's server exchange finishes in one step");
    };
    let tread = plain_frame(
        wire::TREAD,
        3,
        &body_of(&Msg::Tread {
            fid: AFID,
            offset: 0,
            count: 65536,
        }),
    );
    absorb(&link, &mut client, &mut server, &tread, Direction::ClientToServer);
    let rread = plain_frame(
        wire::RREAD,
        3,
        &body_of(&Msg::Rread { data: server_out }),
    );
    server.absorb_plain(&rread);
    client.absorb_plain(&rread);
    link.borrow_mut().plain_wire.push(rread.clone());
    link.borrow_mut().wire_log.push((Direction::ServerToClient, rread));

    let client_session = client.finish(client_secret);
    let server_session = server.finish(server_secret);
    (
        ClientEnd::new(client_session, Rc::clone(&link)),
        ServerEnd::new(server_session, link),
    )
}

/// Absorb one frame on both ends and record it in the plain wire log.
fn absorb(
    link: &Rc<RefCell<AeadLink>>,
    client: &mut Handshake,
    server: &mut Handshake,
    frame: &[u8],
    direction: Direction,
) {
    client.absorb_plain(frame);
    server.absorb_plain(frame);
    link.borrow_mut().plain_wire.push(frame.to_vec());
    link.borrow_mut().wire_log.push((direction, frame.to_vec()));
}

/// A pristine server end over the canonical handshake (one per altered byte).
pub fn fresh_server(link: Rc<RefCell<AeadLink>>, secret: Key) -> ServerEnd {
    ServerEnd::new(session(Role::Server, secret), link)
}

/// The client end of a connection.
pub struct ClientEnd {
    /// The client's session.
    pub session: Session,
    link: Rc<RefCell<AeadLink>>,
}

impl ClientEnd {
    /// Wrap a session and a link.
    pub fn new(session: Session, link: Rc<RefCell<AeadLink>>) -> ClientEnd {
        ClientEnd { session, link }
    }

    /// Seal one message and put it on the wire.
    pub fn send(
        &mut self,
        typ: u8,
        tag: u16,
        fid_handle: [u8; 8],
        exts: &Exts<'_>,
        body: &[u8],
    ) -> core::result::Result<(), Closed> {
        let now = self.link.borrow().now;
        let mut packet = self
            .session
            .seal(typ, tag, fid_handle, exts, body, now)?;
        let mut link = self.link.borrow_mut();
        if let Some(offset) = link.flip_byte {
            if offset < packet.len() {
                packet[offset] ^= 0xFF;
            }
        }
        link.wire_log.push((Direction::ClientToServer, packet.clone()));
        // A tap that duplicates the packet: the same sealed bytes go on the
        // wire twice, which a strict receiver must refuse the second time
        // (plan 04-16, WIRE-11).
        if link.replay_last {
            link.c2s.extend(packet.iter().copied());
        }
        // A tap that reflects the packet back to its sender, which holds the
        // other direction's keys and so fails the MAC (plan 04-16, WIRE-11).
        if link.reflect {
            link.s2c.extend(packet);
        } else {
            link.c2s.extend(packet);
        }
        Ok(())
    }

    /// Receive and decode one sealed message, dropping its header tag.
    pub fn recv(&mut self) -> core::result::Result<Msg, Stage> {
        self.recv_tagged().map(|(_, message)| message)
    }

    /// Receive and decode one sealed message, returning its header tag beside
    /// the message (DECISION P9: the tag is the exchange discriminator).
    pub fn recv_tagged(&mut self) -> core::result::Result<(u16, Msg), Stage> {
        let (now, count) = {
            let link = self.link.borrow();
            (link.now, Rc::clone(&link.decode_count))
        };
        let mut link = self.link.borrow_mut();
        recv_one(&mut self.session, &mut link.s2c, now, &count)
    }

    /// Drive one re-key step with the reserved exchange tag and put the
    /// returned sealed packets on the wire.
    ///
    /// Convenience for the many pins whose exchange and control messages all
    /// carry `NOTAG`; [`ClientEnd::rekey_tagged`] passes any other tag.
    pub fn rekey(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<&Msg>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
        let incoming = incoming.map(|message| (port::devmnt::NOTAG, message));
        self.rekey_tagged(exchange, incoming, now)
    }

    /// Drive one re-key step with an explicit incoming tag and put the returned
    /// sealed packets on the wire.
    pub fn rekey_tagged(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<(u16, &Msg)>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
        let incoming = incoming.map(|(tag, message)| RekeyIncoming { tag, message });
        let packets = self.session.rekey_step(exchange, incoming, now)?;
        let mut link = self.link.borrow_mut();
        for packet in &packets {
            link.wire_log.push((Direction::ClientToServer, packet.clone()));
            link.c2s.extend(packet.iter().copied());
        }
        Ok(packets)
    }
}

/// The server end of a connection.
pub struct ServerEnd {
    /// The server's session.
    pub session: Session,
    link: Rc<RefCell<AeadLink>>,
}

impl ServerEnd {
    /// Wrap a session and a link.
    pub fn new(session: Session, link: Rc<RefCell<AeadLink>>) -> ServerEnd {
        ServerEnd { session, link }
    }

    /// Seal one message and put it on the wire.
    pub fn send(
        &mut self,
        typ: u8,
        tag: u16,
        fid_handle: [u8; 8],
        exts: &Exts<'_>,
        body: &[u8],
    ) -> core::result::Result<(), Closed> {
        let now = self.link.borrow().now;
        let mut packet = self
            .session
            .seal(typ, tag, fid_handle, exts, body, now)?;
        let mut link = self.link.borrow_mut();
        if let Some(offset) = link.flip_byte {
            if offset < packet.len() {
                packet[offset] ^= 0xFF;
            }
        }
        link.wire_log.push((Direction::ServerToClient, packet.clone()));
        if link.replay_last {
            link.s2c.extend(packet.iter().copied());
        }
        if link.reflect {
            link.c2s.extend(packet);
        } else {
            link.s2c.extend(packet);
        }
        Ok(())
    }

    /// Receive and decode one sealed message, dropping its header tag.
    pub fn recv(&mut self) -> core::result::Result<Msg, Stage> {
        self.recv_tagged().map(|(_, message)| message)
    }

    /// Receive and decode one sealed message, returning its header tag beside
    /// the message (DECISION P9).
    pub fn recv_tagged(&mut self) -> core::result::Result<(u16, Msg), Stage> {
        let (now, count) = {
            let link = self.link.borrow();
            (link.now, Rc::clone(&link.decode_count))
        };
        let mut link = self.link.borrow_mut();
        recv_one(&mut self.session, &mut link.c2s, now, &count)
    }

    /// Drive one re-key step with the reserved exchange tag and put the
    /// returned sealed packets on the wire ([`ServerEnd::rekey_tagged`] passes
    /// any other tag).
    pub fn rekey(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<&Msg>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
        let incoming = incoming.map(|message| (port::devmnt::NOTAG, message));
        self.rekey_tagged(exchange, incoming, now)
    }

    /// Drive one re-key step with an explicit incoming tag and put the returned
    /// sealed packets on the wire.
    pub fn rekey_tagged(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<(u16, &Msg)>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
        let incoming = incoming.map(|(tag, message)| RekeyIncoming { tag, message });
        let packets = self.session.rekey_step(exchange, incoming, now)?;
        let mut link = self.link.borrow_mut();
        for packet in &packets {
            link.wire_log.push((Direction::ServerToClient, packet.clone()));
            if link.withhold_rekey_ack && packet.get(4) == Some(&wire::RREKEY) {
                continue;
            }
            link.s2c.extend(packet.iter().copied());
        }
        Ok(packets)
    }
}

/// Frame one packet with the session, open it, and decode its body.
///
/// `frame_len` reads only `Size`; `receive` bounds, verifies and decrypts.  A
/// framing rejection is reported as `Stage::FrameLen` — the fixture does not
/// manufacture `Closed` on its own.
fn recv_one(
    session: &mut Session,
    queue: &mut VecDeque<u8>,
    now: u64,
    decode_count: &Rc<RefCell<usize>>,
) -> core::result::Result<(u16, Msg), Stage> {
    if queue.len() < 4 {
        return Err(Stage::Empty);
    }
    let mut prefix = [0u8; 4];
    for (dst, src) in prefix.iter_mut().zip(queue.iter()) {
        *dst = *src;
    }
    let len = session.frame_len(&prefix).map_err(|_| Stage::FrameLen)?;
    if queue.len() < len {
        return Err(Stage::Empty);
    }
    let mut buf: Vec<u8> = queue.drain(..len).collect();
    let verified = session
        .receive(&mut buf, now)
        .map_err(|_| Stage::Receive)?;
    let typ = verified.header().typ;
    let tag = verified.header().tag;
    let message = msg::decode_body(typ, verified.body()).map_err(|_| Stage::Decode)?;
    *decode_count.borrow_mut() += 1;
    Ok((tag, message))
}

/// Build a packet with a valid MAC and arbitrary header and TLV bytes.
///
/// This bypasses every post-MAC check deliberately: the malformed-after-MAC pin
/// needs a packet the receiver's MAC accepts and its header or TLV rules
/// reject.  It uses only public `crypto::session` and `chan::wire` items.
pub fn seal_bypass(
    keys: &crypto::session::DirectionKeys,
    seq: u64,
    typ: u8,
    tag: u16,
    pad: u8,
    fid_handle: [u8; 8],
    ext_mask: u16,
    tlv_area: &[u8],
    body: &[u8],
) -> Vec<u8> {
    let size = HEADER_LEN + tlv_area.len() + body.len() + MAC_LEN;
    let header = wire::Header {
        size: size as u32,
        typ,
        tag,
        pad,
        fid_handle,
        seq,
        ext_mask,
        ext_len: tlv_area.len() as u16,
    };
    let mut packet = vec![0u8; size];
    let mut encoded = [0u8; HEADER_LEN];
    header.encode(&mut encoded);
    packet[..HEADER_LEN].copy_from_slice(&encoded);
    let tlv_end = HEADER_LEN + tlv_area.len();
    packet[HEADER_LEN..tlv_end].copy_from_slice(tlv_area);
    let body_end = size - MAC_LEN;
    packet[tlv_end..body_end].copy_from_slice(body);
    keys.keystream(seq, 0, &mut packet[tlv_end..body_end])
        .expect("a non-data body fits one keystream");
    let mut mac = keys.mac();
    mac.update(&packet[..size - MAC_LEN]);
    packet[size - MAC_LEN..].copy_from_slice(&mac.finalize());
    packet
}

// ---------------------------------------------------------------------------
// One whole sealed session from the real `devmnt` negotiation (plan 04-11)
// ---------------------------------------------------------------------------

/// The canned tree's root qid path.
pub const E2E_ROOT: u64 = 0x1000;
/// The canned tree's one file qid path.
pub const E2E_FILE: u64 = 0x1001;
/// The canned file's bytes.
pub const E2E_FILE_BYTES: &[u8] = b"sealed session file bytes\n";
/// Plan 9's directory type bit (`QTDIR`).
pub const QTDIR: u8 = 0x80;
/// The canned tree's one file name.
pub const E2E_FILE_NAME: &[u8] = b"file";

/// One decoded sealed packet, with the header fields the server dispatch needs.
pub struct E2eOpened {
    /// The decoded message.
    pub msg: Msg,
    /// The request or reply tag.
    pub tag: u16,
    /// The 8-byte header handle, always carried (D-11).
    pub handle: [u8; 8],
    /// The capability TLV, when the packet carried one.
    pub cap: Option<[u8; 24]>,
    /// The decrypted body length (the padded page for a data message).
    pub body_len: usize,
}

fn e2e_domain_key() -> Key {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = 0xD0u8.wrapping_add(i as u8);
    }
    Key::from_bytes(bytes)
}

fn e2e_entry(fid_key: u32, qid_path: u64, qid_type: u8) -> FidEntry {
    FidEntry {
        fid_key,
        client_id: 0,
        qid_path,
        offset: 0,
        qid_vers: 0,
        iounit: 0,
        lease_gen: 0,
        ring_slot: 0,
        dev_handler: 0,
        epoch: 0,
        qid_type,
        open_mode: 0,
        flags: 0,
        volatility: 0,
        _reserved: [0u8; 7],
    }
}

fn e2e_qid(path: u64) -> Qid {
    Qid {
        path,
        vers: 0,
        typ: if path == E2E_ROOT { QTDIR } else { 0 },
    }
}

/// One `MntIo` half of the client over the link.
///
/// The write half also drives the server's plain handshake, so
/// `Mnt::negotiate_dialect` sees a synchronous peer exactly as the kernel's
/// pipe transport does (one whole message per write).
struct ClientLinkIo {
    link: Rc<RefCell<AeadLink>>,
    server: Option<Rc<RefCell<E2eServer>>>,
}

impl MntIo for ClientLinkIo {
    fn write_msg(&mut self, buf: &[u8]) -> port::Result<u64> {
        if let Some(server) = &self.server {
            server.borrow_mut().receive_plain(buf)?;
        }
        Ok(buf.len() as u64)
    }

    fn read_some(&mut self, buf: &mut [u8]) -> port::Result<u64> {
        let mut link = self.link.borrow_mut();
        let n = link.s2c.len().min(buf.len());
        for byte in buf.iter_mut().take(n) {
            *byte = link.s2c.pop_front().expect("the counted byte is queued");
        }
        Ok(n as u64)
    }
}

fn read_exact(io: &mut dyn MntIo, buf: &mut [u8]) {
    let mut got = 0usize;
    while got < buf.len() {
        let n = io
            .read_some(&mut buf[got..])
            .expect("the fixture's plain reply is available") as usize;
        assert!(n > 0, "the plain reply ended early");
        got += n;
    }
}

fn read_plain_reply(io_r: &mut dyn MntIo) -> Vec<u8> {
    let mut hdr = [0u8; 4];
    read_exact(io_r, &mut hdr);
    let size = u32::from_le_bytes(hdr) as usize;
    let mut msg = hdr.to_vec();
    let mut body = vec![0u8; size - 4];
    read_exact(io_r, &mut body);
    msg.extend_from_slice(&body);
    msg
}

/// One plain-framed send and its whole reply, absorbed on both sides of the
/// client's handshake in wire order.
fn plain_step(io_w: &mut dyn MntIo, io_r: &mut dyn MntIo, handshake: &mut Handshake, frame: &[u8]) {
    handshake.absorb_plain(frame);
    let n = io_w.write_msg(frame).expect("the plain frame is written") as usize;
    assert_eq!(n, frame.len(), "the plain write lands whole");
    let reply = read_plain_reply(io_r);
    handshake.absorb_plain(&reply);
}

/// The server end of one whole sealed session.
///
/// It owns a [`FidTable`] (the synthetic slab tier, one test domain key, epoch
/// 1), assigns one `client_id` per connection, keeps a canned tree, answers
/// the plain handshake and then resolves every sealed request through the
/// table.
pub struct E2eServer {
    link: Rc<RefCell<AeadLink>>,
    handshake: Option<Handshake>,
    session: Option<Session>,
    secret: Key,
    config: TransportConfig,
    table: FidTable,
    client_id: u32,
}

impl E2eServer {
    /// Build a server end over `link` with `client_id` and `config`.
    pub fn new(
        link: Rc<RefCell<AeadLink>>,
        secret: Key,
        client_id: u32,
        config: TransportConfig,
    ) -> Rc<RefCell<E2eServer>> {
        let table = FidTable::synthetic(e2e_domain_key(), 1);
        Rc::new(RefCell::new(E2eServer {
            link,
            handshake: Some(Handshake::new(Role::Server, config)),
            session: None,
            secret,
            config,
            table,
            client_id,
        }))
    }

    fn absorb(&mut self, frame: &[u8]) {
        if let Some(handshake) = &mut self.handshake {
            handshake.absorb_plain(frame);
        }
    }

    /// Answer one plain-framed client message: `Tversion`, `Tauth`, the afid
    /// `Twrite` or the afid `Tread` (the last finishes the server's handshake).
    pub fn receive_plain(&mut self, frame: &[u8]) -> port::Result<()> {
        let typ = frame[4];
        let tag = u16::from_le_bytes([frame[5], frame[6]]);
        if typ == wire::TVERSION {
            let Msg::Tversion { msize: offer, .. } = msg::decode_body(typ, &frame[7..])? else {
                return Err("e2e server: malformed Tversion");
            };
            let answer = offer.min(MSIZE);
            self.config.msize = answer;
            self.handshake = Some(Handshake::new(Role::Server, self.config));
            self.absorb(frame);
            let reply = plain_frame(
                wire::RVERSION,
                tag,
                &body_of(&Msg::Rversion {
                    msize: answer,
                    version: VERSION_AEAD.to_vec(),
                }),
            );
            self.absorb(&reply);
            self.link.borrow_mut().s2c.extend(reply.iter().copied());
            return Ok(());
        }
        self.absorb(frame);
        let reply = match typ {
            wire::TAUTH => plain_frame(
                wire::RAUTH,
                tag,
                &body_of(&Msg::Rauth {
                    aqid: Qid { path: 0, vers: 0, typ: 0 },
                }),
            ),
            wire::TWRITE => {
                let Msg::Twrite { data, .. } = msg::decode_body(typ, &frame[7..])? else {
                    return Err("e2e server: malformed Twrite");
                };
                plain_frame(
                    wire::RWRITE,
                    tag,
                    &body_of(&Msg::Rwrite {
                        count: data.len() as u32,
                    }),
                )
            }
            wire::TREAD => plain_frame(
                wire::RREAD,
                tag,
                &body_of(&Msg::Rread {
                    data: SERVER_EXCHANGE.to_vec(),
                }),
            ),
            _ => return Err("e2e server: unexpected plain message"),
        };
        self.absorb(&reply);
        self.link.borrow_mut().s2c.extend(reply.iter().copied());
        if typ == wire::TREAD {
            let handshake = self.handshake.take().expect("the plain handshake is running");
            self.session = Some(handshake.finish(self.secret.clone()));
        }
        Ok(())
    }

    /// Open one sealed client packet without answering it (the re-key driver).
    pub fn recv_sealed(&mut self) -> core::result::Result<E2eOpened, Stage> {
        let Some(mut session) = self.session.take() else {
            return Err(Stage::Receive);
        };
        let result = recv_sealed_frame(&mut session, &self.link, true);
        self.session = Some(session);
        result
    }

    /// Open one sealed request, route it, and answer it.
    ///
    /// Every opened message is classified with [`is_rekey_traffic`] first
    /// (DECISION P9): exchange and control traffic goes to the session's re-key
    /// state machine, whose sealed packets this end puts on the wire; anything
    /// else is an ordinary request and is served from the file tree.  A message
    /// the router rejects never reaches the second path and never closes the
    /// connection because an exchange is in flight (CR-02).
    pub fn handle_one(
        &mut self,
        exchange: &mut dyn SessionExchange,
        now: u64,
    ) -> core::result::Result<(), Stage> {
        let opened = self.recv_sealed()?;
        if is_rekey_traffic(opened.tag, &opened.msg) {
            let incoming = RekeyIncoming {
                tag: opened.tag,
                message: &opened.msg,
            };
            let packets = {
                let Some(session) = self.session.as_mut() else {
                    return Err(Stage::Receive);
                };
                session
                    .rekey_step(exchange, Some(incoming), now)
                    .map_err(|_| Stage::Receive)?
            };
            let mut link = self.link.borrow_mut();
            for packet in &packets {
                link.wire_log.push((Direction::ServerToClient, packet.clone()));
                link.s2c.extend(packet.iter().copied());
            }
            return Ok(());
        }
        self.serve(opened)
    }

    /// Answer one already-opened ordinary request from the canned tree.
    fn serve(&mut self, opened: E2eOpened) -> core::result::Result<(), Stage> {
        let E2eOpened { msg, tag, handle, cap, .. } = opened;
        match msg {
            Msg::Tattach { fid, .. } => {
                let entry = e2e_entry(fid, E2E_ROOT, QTDIR);
                let new = self.table.alloc(self.client_id, entry).map_err(|_| Stage::Decode)?;
                self.reply(
                    tag,
                    new.to_wire(),
                    &Exts::default(),
                    &Msg::Rattach { qid: e2e_qid(E2E_ROOT) },
                )
            }
            Msg::Twalk { newfid, wname, .. } => {
                let entry = match self.stateful(&handle) {
                    Ok(entry) => entry,
                    Err(_) => return self.rerror(tag),
                };
                let next_path = if wname.is_empty() {
                    entry.qid_path
                } else if entry.qid_path == E2E_ROOT && wname[0].as_slice() == E2E_FILE_NAME {
                    E2E_FILE
                } else {
                    return self.rerror(tag);
                };
                let qid_type = if next_path == E2E_ROOT { QTDIR } else { 0 };
                let new = self
                    .table
                    .alloc(self.client_id, e2e_entry(newfid, next_path, qid_type))
                    .map_err(|_| Stage::Decode)?;
                self.reply(
                    tag,
                    new.to_wire(),
                    &Exts::default(),
                    &Msg::Rwalk { wqid: vec![e2e_qid(next_path)] },
                )
            }
            Msg::Topen { mode, .. } => {
                let entry = match self.stateful(&handle) {
                    Ok(entry) => entry,
                    Err(_) => return self.rerror(tag),
                };
                let qid = e2e_qid(entry.qid_path);
                let wire_cap = self.table.issue_cap(&qid, mode).to_wire();
                let exts = Exts {
                    stateless_cap: Some(&wire_cap),
                    ..Exts::default()
                };
                // D-12: a packet carrying a capability has an all-zero handle.
                self.reply(tag, [0u8; 8], &exts, &Msg::Ropen { qid, iounit: 0 })
            }
            Msg::Tread { offset, count, .. } => {
                let path = match (&cap, FidHandle::from_wire(&handle, cap.as_ref())) {
                    (Some(_), Ok(FidHandle::Stateless(capability))) => {
                        if self.table.verify_cap(&capability).is_err() {
                            // P8: a capability that fails verification closes
                            // the connection; nothing is answered.
                            self.teardown();
                            return Ok(());
                        }
                        capability.qid_path
                    }
                    (None, Ok(FidHandle::Stateful(_))) => match self.stateful(&handle) {
                        Ok(entry) => entry.qid_path,
                        Err(_) => return self.rerror(tag),
                    },
                    _ => return self.rerror(tag),
                };
                let source: &[u8] = if path == E2E_FILE { E2E_FILE_BYTES } else { b"" };
                let start = (offset as usize).min(source.len());
                let end = (start + count as usize).min(source.len());
                let data = source[start..end].to_vec();
                let reply_handle = if cap.is_some() { [0u8; 8] } else { handle };
                self.reply(tag, reply_handle, &Exts::default(), &Msg::Rread { data })
            }
            Msg::Tclunk { .. } => {
                let parsed = match StatefulHandle::from_wire(&handle) {
                    Ok(parsed) => parsed,
                    Err(_) => return self.rerror(tag),
                };
                if self.table.clunk(self.client_id, &parsed).is_err() {
                    return self.rerror(tag);
                }
                self.reply(tag, [0u8; 8], &Exts::default(), &Msg::Rclunk)
            }
            _ => self.rerror(tag),
        }
    }

    fn stateful(&self, handle: &[u8; 8]) -> port::Result<FidEntry> {
        let parsed = StatefulHandle::from_wire(handle)?;
        self.table.resolve(self.client_id, &parsed)
    }

    /// Drive one re-key step and put the returned sealed packets on the wire.
    ///
    /// The exchange's messages all carry the reserved tag (DECISION P9), so
    /// this convenience forwards `NOTAG`; a pin that passes another tag calls
    /// [`Session::rekey_step`] directly.
    pub fn rekey(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<&Msg>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
        let Some(session) = self.session.as_mut() else {
            return Err(Closed);
        };
        let incoming = incoming.map(|message| RekeyIncoming {
            tag: port::devmnt::NOTAG,
            message,
        });
        let packets = session.rekey_step(exchange, incoming, now)?;
        let mut link = self.link.borrow_mut();
        for packet in &packets {
            link.wire_log.push((Direction::ServerToClient, packet.clone()));
            link.s2c.extend(packet.iter().copied());
        }
        Ok(packets)
    }

    /// Whether this end's send counter has reached the re-key threshold.
    pub fn rekey_due(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.rekey_due())
    }

    /// The live and reclaimed slot counts of every shard, in shard order.
    pub fn fid_memory(&self) -> Vec<(u64, u64)> {
        (0..self.table.shard_count() as u8)
            .filter_map(|shard| self.table.bitmap_lens(shard))
            .collect()
    }

    /// Zeroize and close this end.
    pub fn teardown(&mut self) {
        if let Some(session) = self.session.as_mut() {
            session.teardown();
        }
    }

    /// Whether this end has closed.
    pub fn is_closed(&self) -> bool {
        self.session.as_ref().is_none_or(|session| session.is_closed())
    }

    fn rerror(&mut self, tag: u16) -> core::result::Result<(), Stage> {
        self.reply(
            tag,
            [0u8; 8],
            &Exts::default(),
            &Msg::Rerror {
                ename: b"e2e server: request refused".to_vec(),
            },
        )
    }

    fn reply(
        &mut self,
        tag: u16,
        handle: [u8; 8],
        exts: &Exts<'_>,
        message: &Msg,
    ) -> core::result::Result<(), Stage> {
        let mut body = Vec::new();
        if msg::encode_body(message, &mut body).is_err() {
            self.teardown();
            return Err(Stage::Decode);
        }
        let now = self.link.borrow().now;
        let typ = msg::opcode(message);
        let Some(session) = self.session.as_mut() else {
            return Err(Stage::Receive);
        };
        match session.seal(typ, tag, handle, exts, &body, now) {
            Err(_) => Err(Stage::Receive),
            Ok(packet) => {
                let mut link = self.link.borrow_mut();
                link.wire_log.push((Direction::ServerToClient, packet.clone()));
                link.s2c.extend(packet.iter().copied());
                Ok(())
            }
        }
    }
}

/// The client end of one whole sealed session.
pub struct E2eClient {
    /// The client's session (negotiated through the real `devmnt` code).
    pub session: Session,
    link: Rc<RefCell<AeadLink>>,
}

impl E2eClient {
    /// Wrap a finished session and the link it rides.
    pub fn new(session: Session, link: Rc<RefCell<AeadLink>>) -> E2eClient {
        E2eClient { session, link }
    }

    /// Seal one message and put it on the wire.
    pub fn send(
        &mut self,
        typ: u8,
        tag: u16,
        handle: [u8; 8],
        exts: &Exts<'_>,
        body: &[u8],
    ) -> core::result::Result<(), Closed> {
        let now = self.link.borrow().now;
        let packet = self.session.seal(typ, tag, handle, exts, body, now)?;
        let mut link = self.link.borrow_mut();
        link.wire_log.push((Direction::ClientToServer, packet.clone()));
        link.c2s.extend(packet.iter().copied());
        Ok(())
    }

    /// Open one sealed reply without answering it.
    pub fn recv(&mut self) -> core::result::Result<E2eOpened, Stage> {
        recv_sealed_frame(&mut self.session, &self.link, false)
    }

    /// Drive one re-key step and put the returned sealed packets on the wire.
    ///
    /// The exchange's messages all carry the reserved tag (DECISION P9).
    pub fn rekey(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<&Msg>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
        let incoming = incoming.map(|message| RekeyIncoming {
            tag: port::devmnt::NOTAG,
            message,
        });
        let packets = self.session.rekey_step(exchange, incoming, now)?;
        let mut link = self.link.borrow_mut();
        for packet in &packets {
            link.wire_log.push((Direction::ClientToServer, packet.clone()));
            link.c2s.extend(packet.iter().copied());
        }
        Ok(packets)
    }
}

/// Open one whole sealed frame from `link` (`from_client` chooses the queue).
fn recv_sealed_frame(
    session: &mut Session,
    link: &Rc<RefCell<AeadLink>>,
    from_client: bool,
) -> core::result::Result<E2eOpened, Stage> {
    let now = link.borrow().now;
    let mut guard = link.borrow_mut();
    let queue = if from_client { &mut guard.c2s } else { &mut guard.s2c };
    if queue.len() < 4 {
        return Err(Stage::Empty);
    }
    let mut prefix = [0u8; 4];
    for (dst, src) in prefix.iter_mut().zip(queue.iter()) {
        *dst = *src;
    }
    let len = session.frame_len(&prefix).map_err(|_| Stage::FrameLen)?;
    if queue.len() < len {
        return Err(Stage::Empty);
    }
    let mut buf: Vec<u8> = queue.drain(..len).collect();
    drop(guard);
    match session.receive(&mut buf, now) {
        Err(_) => Err(Stage::Receive),
        Ok(verified) => {
            let header = *verified.header();
            let exts = match tlv::parse(header.ext_mask, verified.tlvs()) {
                Ok(exts) => exts,
                Err(_) => return Err(Stage::Receive),
            };
            let cap = exts.stateless_cap.copied();
            let body_len = verified.body().len();
            match msg::decode_body(header.typ, verified.body()) {
                Err(_) => Err(Stage::Decode),
                Ok(message) => Ok(E2eOpened {
                    msg: message,
                    tag: header.tag,
                    handle: header.fid_handle,
                    cap,
                    body_len,
                }),
            }
        }
    }
}

/// Negotiate `9P2000.aead` through the real `devmnt` code and finish both
/// ends' handshakes, returning the ready client, server and link.
pub fn connect_aead(
    secret: Key,
    client_id: u32,
) -> (E2eClient, Rc<RefCell<E2eServer>>, Rc<RefCell<AeadLink>>) {
    connect_aead_with(secret, client_id, config())
}

/// `connect_aead` with chosen session parameters (the re-key pin's low
/// threshold).
pub fn connect_aead_with(
    secret: Key,
    client_id: u32,
    transport: TransportConfig,
) -> (E2eClient, Rc<RefCell<E2eServer>>, Rc<RefCell<AeadLink>>) {
    let link = AeadLink::new();
    let server = E2eServer::new(Rc::clone(&link), secret.clone(), client_id, transport);
    let io_r: Box<dyn MntIo> = Box::new(ClientLinkIo {
        link: Rc::clone(&link),
        server: None,
    });
    let io_w: Box<dyn MntIo> = Box::new(ClientLinkIo {
        link: Rc::clone(&link),
        server: Some(Rc::clone(&server)),
    });
    let negotiated =
        Mnt::negotiate_dialect(io_r, io_w, 0, Dialect::Aead).expect("the aead offer reaches the engine");
    let Negotiated::Aead(handoff) = negotiated else {
        panic!("an aead answer must hand the session to the engine");
    };
    let (mut handshake, mut io_r, mut io_w) = handoff.into_handshake(Role::Client, transport);

    // The rest of the plain handshake: Tauth/Rauth, then one afid write/read
    // round carrying the two ends' opaque exchange bytes (D-16, D-17).
    plain_step(&mut *io_w, &mut *io_r, &mut handshake, &tauth_frame());
    plain_step(
        &mut *io_w,
        &mut *io_r,
        &mut handshake,
        &plain_frame(
            wire::TWRITE,
            2,
            &body_of(&Msg::Twrite {
                fid: AFID,
                offset: 0,
                data: CLIENT_EXCHANGE.to_vec(),
            }),
        ),
    );
    plain_step(
        &mut *io_w,
        &mut *io_r,
        &mut handshake,
        &plain_frame(
            wire::TREAD,
            3,
            &body_of(&Msg::Tread {
                fid: AFID,
                offset: 0,
                count: 65536,
            }),
        ),
    );

    let session = handshake.finish(secret);
    (E2eClient::new(session, Rc::clone(&link)), server, link)
}
