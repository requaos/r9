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
    Closed, ExchangeStep, Handshake, Role, Session, SessionExchange, TransportConfig,
};
use port::chan::tlv::Exts;
use port::chan::wire;
use port::chan::{HEADER_LEN, MAC_LEN, VERSION_AEAD};
use port::dat::Qid;

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
fn body_of(message: &Msg) -> Vec<u8> {
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

    /// Receive and decode one sealed message.
    pub fn recv(&mut self) -> core::result::Result<Msg, Stage> {
        let (now, count) = {
            let link = self.link.borrow();
            (link.now, Rc::clone(&link.decode_count))
        };
        let mut link = self.link.borrow_mut();
        recv_one(&mut self.session, &mut link.s2c, now, &count)
    }

    /// Drive one re-key step and put the returned sealed packets on the wire.
    pub fn rekey(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<&Msg>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
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

    /// Receive and decode one sealed message.
    pub fn recv(&mut self) -> core::result::Result<Msg, Stage> {
        let (now, count) = {
            let link = self.link.borrow();
            (link.now, Rc::clone(&link.decode_count))
        };
        let mut link = self.link.borrow_mut();
        recv_one(&mut self.session, &mut link.c2s, now, &count)
    }

    /// Drive one re-key step and put the returned sealed packets on the wire.
    pub fn rekey(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<&Msg>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
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
) -> core::result::Result<Msg, Stage> {
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
    let message = msg::decode_body(typ, verified.body()).map_err(|_| Stage::Decode)?;
    *decode_count.borrow_mut() += 1;
    Ok(message)
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
