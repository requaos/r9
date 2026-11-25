//! transport — the connection engine: the plain-framed handshake, the
//! session, sealing and opening, and the single fail-closed path
//! (WIRE-03, WIRE-05, WIRE-11; D-07, D-15 to D-21, D-26).
//!
//! # The handshake (D-16, D-17)
//!
//! `Tversion` / `Rversion`, `Tauth` / `Rauth` and the auth-fid `Twrite` /
//! `Rwrite` and `Tread` / `Rread` exchange round travel in plain 9P2000
//! framing.  Every one of those messages, whole, sent or received, in wire
//! order, is absorbed by [`Handshake::absorb_plain`] into the unkeyed BLAKE3
//! transcript; [`Handshake::finish`] mixes that transcript into the key
//! derivation.  The handshake is strictly serial — one request outstanding at
//! a time — so both ends absorb the same bytes in the same order, and an
//! altered version string or msize gives the two ends different keys.
//!
//! The session secret is injected behind [`SessionExchange`]: the transport is
//! handed 32 bytes and never learns where they came from (D-16).  The real
//! authenticated exchange is later phases' work; no live connection between
//! two real machines comes out of this phase.
//!
//! # The receive order (D-07, D-26)
//!
//! A received packet is bounded by `Size` alone (`Session::frame_len`), the
//! packet MAC is verified over `[0, size - MAC_LEN)` (`Session::receive` uses
//! `Unverified::new` and then `Session::open`), and only after the MAC
//! verifies is any other header field read, the TLV area parsed and the body
//! decrypted.  Any rejection at any step goes through one private `fail()`:
//! both directions' keys are zeroized, both counters are zeroed, the session
//! is closed, and every later `seal` and `open` returns `Err(Closed)`.  Because
//! `fail` takes `&mut self`, the closed state is one change no other caller
//! can observe half-made (WIRE-03/concurrency).
//!
//! # The strict sequence check (WIRE-04)
//!
//! After the MAC verifies and before any other header field is used,
//! `Session::open` compares the packet's `SeqNonce` with the one value that
//! direction expects next.  A repeat, a skip or an earlier value is a
//! protocol error: under the owner's P7 decision the connection is closed
//! through the one `fail()` path and every key is zeroized, and the packet is
//! never handed to the handler.
//!
//! # What is not here yet
//!
//! Re-keying is plan 04-08; its timing needs `now`, and this plan threads that
//! argument through without reading it.  A connection owner must call a
//! periodic `poll(now)` once one exists; nothing in the kernel drives a
//! session this phase (D-22).

use alloc::vec::Vec;

use crypto::key::Key;
use crypto::session::{self, DirectionKeys, SessionKeys, Transcript};

use crate::mcslock::{Lock, LockNode};

use super::msg;
use super::tlv::{self, Exts};
use super::wire::{self, Header, Unverified, Verified, check_header, frame_size, packet_len};
use super::{HEADER_LEN, MAC_LEN, PAGE_LEN};

/// One end of a connection (D-15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The client: seals with the `c2s` keys, opens with the `s2c` keys.
    Client,
    /// The server: seals with the `s2c` keys, opens with the `c2s` keys.
    Server,
}

/// The default packet threshold that triggers an in-band re-key (P3, D-19).
pub const REKEY_AFTER_PACKETS: u64 = 1 << 48;

/// The default milliseconds to wait for a re-key acknowledgement (P3, D-19).
pub const REKEY_ACK_TIMEOUT_MS: u64 = 5_000;

/// The session parameters both ends agree on before the first sealed packet.
#[derive(Clone, Copy, Debug)]
pub struct TransportConfig {
    /// The negotiated maximum message unit (msize).
    pub msize: u32,
    /// Send this many packets before starting an in-band re-key (plan 04-08).
    pub rekey_after_packets: u64,
    /// How long to wait for a re-key acknowledgement (plan 04-08).
    pub rekey_ack_timeout_ms: u64,
}

impl Default for TransportConfig {
    fn default() -> TransportConfig {
        TransportConfig {
            msize: 8192,
            rekey_after_packets: REKEY_AFTER_PACKETS,
            rekey_ack_timeout_ms: REKEY_ACK_TIMEOUT_MS,
        }
    }
}

/// One step of the session exchange seam (D-16).
pub enum ExchangeStep {
    /// The exchange is not finished; the peer must supply more bytes.
    Continue,
    /// The exchange finished and produced the 32-byte session secret.
    Done(Key),
}

/// The seam that hands the transport its 32-byte session secret (D-16).
///
/// The transport never learns where the secret came from.  Tests implement
/// this with a fixed value; the real authenticated exchange is later phases'
/// work.  No implementation that returns a fixed or derivable secret may live
/// under `port/src` (WIRE-05 prohibition).
pub trait SessionExchange {
    /// Consume the peer's opaque exchange bytes, append this end's reply to
    /// `out`, and report whether the exchange finished.
    fn step(&mut self, incoming: &[u8], out: &mut Vec<u8>) -> crate::Result<ExchangeStep>;
}

/// The connection closed: a failed receive, or an orderly teardown.
///
/// The type carries no packet bytes, no plaintext and no key-derived value
/// (WIRE-03 prohibition).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Closed;

/// The mutable sending side one direction owns: the direction's keys and the
/// next sequence number (D-21).
struct SendState {
    keys: DirectionKeys,
    seq: u64,
    closed: bool,
}

impl SendState {
    /// Zeroize this direction's keys, reset the counter and close.
    fn fail(&mut self) {
        self.keys.zeroize();
        self.seq = 0;
        self.closed = true;
    }

    /// Whether this direction's keys are zero.
    fn is_zeroized(&self) -> bool {
        self.keys.is_zeroized()
    }
}

/// The sending half of a connection: sealing and writing happen under one
/// per-direction lock, so packets leave in counter order (D-21, WIRE-04).
///
/// The lock is held across the seal, the write and the counter advance, so two
/// callers sharing one `SendHalf` cannot put packets on the wire out of
/// sequence.  [`Session::seal`] takes the same lock for single-owner use.
pub struct SendHalf {
    msize: u32,
    authenticated: bool,
    state: Lock<SendState>,
}

impl SendHalf {
    /// Seal one message and write the whole packet through `io`, all under the
    /// one per-direction lock (D-21).
    ///
    /// A closed session writes nothing and returns [`Closed`].  A write that
    /// does not land whole closes the direction: a sequence number consumed by
    /// a packet the peer never saw would desynchronise the stream.
    #[allow(clippy::too_many_arguments)]
    pub fn send(
        &self,
        io: &mut dyn crate::devmnt::MntIo,
        typ: u8,
        tag: u16,
        fid_handle: [u8; 8],
        exts: &Exts<'_>,
        body: &[u8],
        now: u64,
    ) -> core::result::Result<(), Closed> {
        // Re-key timing (D-19) is plan 04-08's; this plan threads `now` only.
        let _ = now;
        if !self.authenticated {
            return Err(Closed);
        }
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let seq = state.seq;
        let packet = seal_into(&mut state, self.msize, seq, typ, tag, fid_handle, exts, body)?;
        // The seal, the write and the advance all sit inside this one lock
        // scope, so two senders cannot put packets on the wire out of order
        // (D-21).  A write that does not land whole closes the direction.
        match io.write_msg(&packet) {
            Ok(written) if written as usize == packet.len() => {
                state.seq = seq.wrapping_add(1);
                Ok(())
            }
            _ => {
                state.fail();
                Err(Closed)
            }
        }
    }

    /// Seal one message into a `Vec`, used by [`Session::seal`] for
    /// single-owner sending through the same lock and counter.
    fn seal_single(
        &self,
        typ: u8,
        tag: u16,
        fid_handle: [u8; 8],
        exts: &Exts<'_>,
        body: &[u8],
    ) -> core::result::Result<Vec<u8>, Closed> {
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let seq = state.seq;
        let packet = seal_into(&mut state, self.msize, seq, typ, tag, fid_handle, exts, body)?;
        state.seq = seq.wrapping_add(1);
        Ok(packet)
    }

    /// Zeroize the sending keys and close this direction.
    fn fail(&self) {
        let node = LockNode::new();
        self.state.lock(&node).fail();
    }

    /// Whether this direction has closed.
    fn is_closed(&self) -> bool {
        let node = LockNode::new();
        self.state.lock(&node).closed
    }

    /// Whether this direction's keys are zero.
    fn keys_zeroized(&self) -> bool {
        let node = LockNode::new();
        self.state.lock(&node).is_zeroized()
    }
}

/// Seal one packet into a fresh `Vec`, writing the fields at their pinned
/// offsets (D-06, D-07, P5).
///
/// `seq` is the sequence number the caller chose; the caller advances its own
/// counter under the same lock.  Every failure zeroizes the direction's keys
/// and closes it.
#[allow(clippy::too_many_arguments)]
fn seal_into(
    state: &mut SendState,
    msize: u32,
    seq: u64,
    typ: u8,
    tag: u16,
    fid_handle: [u8; 8],
    exts: &Exts<'_>,
    body: &[u8],
) -> core::result::Result<Vec<u8>, Closed> {
    if state.closed {
        return Err(Closed);
    }
    let tlv_len = exts.encoded_len();
    let Ok(ext_len) = u16::try_from(tlv_len) else {
        state.fail();
        return Err(Closed);
    };
    let Ok(size) = packet_len(tlv_len, body.len(), msize) else {
        state.fail();
        return Err(Closed);
    };
    let size = size as usize;
    let header = Header {
        size: size as u32,
        typ,
        tag,
        pad: 0,
        fid_handle,
        seq,
        ext_mask: exts.mask(),
        ext_len,
    };
    let mut packet = Vec::with_capacity(size);
    let mut encoded = [0u8; HEADER_LEN];
    header.encode(&mut encoded);
    packet.extend_from_slice(&encoded);
    tlv::encode(exts, &mut packet);
    packet.extend_from_slice(body);
    packet.resize(size, 0);

    let body_start = HEADER_LEN + tlv_len;
    let body_end = size - MAC_LEN;
    if crypt_body(&state.keys, seq, typ, &mut packet[body_start..body_end]).is_err() {
        state.fail();
        return Err(Closed);
    }

    let mut mac = state.keys.mac();
    mac.update(&packet[..size - MAC_LEN]);
    packet[size - MAC_LEN..].copy_from_slice(&mac.finalize());
    Ok(packet)
}

/// Collects the plain-framed handshake messages and finishes into a [`Session`]
/// (D-17).
pub struct Handshake {
    role: Role,
    config: TransportConfig,
    transcript: Transcript,
    seen_auth: bool,
    seen_rauth: bool,
}

impl Handshake {
    /// Start a handshake for one end of a connection.
    pub fn new(role: Role, config: TransportConfig) -> Handshake {
        Handshake {
            role,
            config,
            transcript: Transcript::new(),
            seen_auth: false,
            seen_rauth: false,
        }
    }

    /// Absorb one whole plain-framed message, sent or received, in wire order.
    ///
    /// The type byte sits at offset 4 of the 9P2000 frame; `Tauth` and `Rauth`
    /// are what mark the exchange as authenticated, so a session that never
    /// saw them refuses to seal or open (D-16, D-17).
    pub fn absorb_plain(&mut self, message: &[u8]) {
        self.transcript.absorb(message);
        if let Some(&typ) = message.get(4) {
            match typ {
                wire::TAUTH => self.seen_auth = true,
                wire::RAUTH => self.seen_rauth = true,
                _ => {}
            }
        }
    }

    /// Derive the session keys and hand back the ready [`Session`].
    ///
    /// The transcript hash is the HKDF salt, the secret is the input keying
    /// material, and generation 0 is the initial generation (P2).
    pub fn finish(self, secret: Key) -> Session {
        let transcript = self.transcript.hash();
        let keys = session::derive(&secret, &transcript, 0);
        let authenticated = self.seen_auth && self.seen_rauth;
        let SessionKeys { c2s, s2c } = keys;
        let (send_keys, recv_keys) = match self.role {
            Role::Client => (c2s, s2c),
            Role::Server => (s2c, c2s),
        };
        let send = SendHalf {
            msize: self.config.msize,
            authenticated,
            state: Lock::new(
                "chan-send",
                SendState {
                    keys: send_keys,
                    seq: 0,
                    closed: false,
                },
            ),
        };
        Session {
            config: self.config,
            generation: 0,
            transcript,
            send,
            recv_keys,
            recv_seq: 0,
            authenticated,
        }
    }
}

/// One end of an established connection (D-15).
pub struct Session {
    config: TransportConfig,
    /// The derivation generation; plan 04-08's re-key reads it.
    #[allow(dead_code)]
    generation: u32,
    /// The handshake transcript hash; plan 04-08 chains the re-key from it.
    #[allow(dead_code)]
    transcript: [u8; 32],
    /// The sending direction: its keys, sequence counter and closed state,
    /// behind one lock so concurrent senders stay in counter order (D-21).
    send: SendHalf,
    /// The receiving direction's keys; the receive side is single-reader.
    recv_keys: DirectionKeys,
    recv_seq: u64,
    /// Whether `Tauth` and `Rauth` were both absorbed (D-16, D-17).
    authenticated: bool,
}

impl Session {
    /// Seal one message and return the whole packet.
    ///
    /// The header is written field by field, the TLV area appended cleartext,
    /// the body encrypted — parameters from block counter 0 and data page `i`
    /// from `1 + 64 * i` (P5) — and the MAC over `[0, size - MAC_LEN)` appended
    /// last (D-07).  Every failure closes the connection: the only error this
    /// signature can return is [`Closed`].
    pub fn seal(
        &mut self,
        typ: u8,
        tag: u16,
        fid_handle: [u8; 8],
        exts: &Exts<'_>,
        body: &[u8],
        now: u64,
    ) -> core::result::Result<Vec<u8>, Closed> {
        // Re-key timing (D-19) is plan 04-08's; this plan threads `now` only.
        let _ = now;
        if !self.authenticated || self.send.is_closed() {
            return Err(Closed);
        }
        // Single-owner sending takes the same per-direction lock and advances
        // the same counter a shared `SendHalf` would (D-21).
        match self.send.seal_single(typ, tag, fid_handle, exts, body) {
            Ok(packet) => Ok(packet),
            Err(closed) => {
                self.fail();
                Err(closed)
            }
        }
    }

    /// The sending half, shareable across threads: every `send` seals, writes
    /// and advances under the one per-direction lock (D-21).
    pub fn send_half(&self) -> &SendHalf {
        &self.send
    }

    /// Open one packet: MAC first, then the sequence, the header fields, TLVs
    /// and body (D-07, D-26, WIRE-04).
    ///
    /// `Size` is the only field read before the MAC verifies.  Immediately
    /// after it does, the sequence number must be the value this direction
    /// expects next; a repeat, a skip or an earlier value closes the
    /// connection through `fail()` (P7, WIRE-04).
    pub fn open<'a>(
        &mut self,
        packet: Unverified<'a>,
        now: u64,
    ) -> core::result::Result<Verified<'a>, Closed> {
        let _ = now;
        if !self.authenticated || self.send.is_closed() {
            return Err(Closed);
        }
        let buf = packet.take();
        let size = buf.len();
        if size < HEADER_LEN + MAC_LEN {
            self.fail();
            return Err(Closed);
        }

        // Verify before acting on any field but Size (D-07).
        let mut mac = self.recv_keys.mac();
        mac.update(&buf[..size - MAC_LEN]);
        let Ok(tag) = <[u8; MAC_LEN]>::try_from(&buf[size - MAC_LEN..]) else {
            self.fail();
            return Err(Closed);
        };
        if mac.verify(&tag).is_err() {
            self.fail();
            return Err(Closed);
        }

        // Only now read the header, and check the sequence number before any
        // other field is used: a repeat, a skip or an earlier value is a
        // protocol error, and P7 says it closes the connection (WIRE-04).
        let Ok(header_bytes) = <&[u8; HEADER_LEN]>::try_from(&buf[..HEADER_LEN]) else {
            self.fail();
            return Err(Closed);
        };
        let header = Header::decode(header_bytes);
        let seq = header.seq;
        if seq != self.recv_seq {
            self.fail();
            return Err(Closed);
        }
        if check_header(&header, size).is_err() {
            self.fail();
            return Err(Closed);
        }

        let Some(tlv_end) = HEADER_LEN.checked_add(header.ext_len as usize) else {
            self.fail();
            return Err(Closed);
        };
        let Some(tlv_area) = buf.get(HEADER_LEN..tlv_end) else {
            self.fail();
            return Err(Closed);
        };
        if tlv::parse(header.ext_mask, tlv_area).is_err() {
            self.fail();
            return Err(Closed);
        }

        let body_end = size - MAC_LEN;
        if tlv_end > body_end {
            self.fail();
            return Err(Closed);
        }
        if crypt_body(
            &self.recv_keys,
            seq,
            header.typ,
            &mut buf[tlv_end..body_end],
        )
        .is_err()
        {
            self.fail();
            return Err(Closed);
        }

        let verified = match Verified::new_verified(header, buf) {
            Ok(verified) => verified,
            Err(_) => {
                self.fail();
                return Err(Closed);
            }
        };
        self.recv_seq = seq.wrapping_add(1);
        Ok(verified)
    }

    /// Frame a received packet from its 4-byte `Size` prefix (WIRE-03).
    ///
    /// Applies `wire::frame_size` with this session's negotiated msize and
    /// returns the whole packet length.  Only `Size` is read.
    pub fn frame_len(&mut self, prefix: &[u8; 4]) -> core::result::Result<usize, Closed> {
        if !self.authenticated || self.send.is_closed() {
            return Err(Closed);
        }
        match frame_size(prefix, self.config.msize) {
            Ok(size) => Ok(size),
            Err(_) => {
                self.fail();
                Err(Closed)
            }
        }
    }

    /// Open a received packet from its raw bytes (WIRE-03).
    ///
    /// Builds the bounded `Unverified` with this session's msize and hands it
    /// to [`Session::open`].
    pub fn receive<'a>(
        &mut self,
        packet: &'a mut [u8],
        now: u64,
    ) -> core::result::Result<Verified<'a>, Closed> {
        if !self.authenticated {
            return Err(Closed);
        }
        match Unverified::new(packet, self.config.msize) {
            Ok(unverified) => self.open(unverified, now),
            Err(_) => {
                self.fail();
                Err(Closed)
            }
        }
    }

    /// The single failure path: zeroize and close.
    ///
    /// Both directions' keys (the send and receive slots) are overwritten with
    /// zeros, both counters are zeroed, and the closed flag is set.  The
    /// zeroized values stay in their slots, so `keys_zeroized` has something
    /// to read.  Plan 04-08's pending and superseded key slots join this call;
    /// they do not exist yet.
    fn fail(&mut self) {
        // The send keys sit behind the per-direction lock, so this reaches
        // them through it; the receive keys are single-reader and are
        // zeroized directly (D-15, D-21).
        self.send.fail();
        self.recv_keys.zeroize();
        self.recv_seq = 0;
    }

    /// The orderly close: the same zeroize-and-close path as a failure (D-15).
    pub fn teardown(&mut self) {
        self.fail();
    }

    /// Whether this session has closed.
    pub fn is_closed(&self) -> bool {
        self.send.is_closed()
    }

    /// Whether every key this session holds is zero.
    ///
    /// Computed from the key storage, never from the closed flag, so a
    /// `fail()` or `teardown()` that set the flag without zeroizing answers
    /// `false`.  It reveals no key material, only `true` or `false`, and
    /// exists so an orderly teardown can be checked (D-15).  The send keys are
    /// read through the per-direction lock (D-21).
    pub fn keys_zeroized(&self) -> bool {
        self.send.keys_zeroized() && self.recv_keys.is_zeroized()
    }
}

/// One data page is `PAGE_LEN` bytes, which is exactly this many ChaCha20
/// 64-byte blocks (P5).
const PAGE_BLOCKS: usize = PAGE_LEN / 64;

/// Encrypt (or decrypt) one body under `keys` at sequence `seq` (P5).
///
/// Parameters encrypt from block counter 0; data page `i` from `1 + 64 * i`,
/// so one page decrypts on its own.  Decryption is the same XOR.
fn crypt_body(keys: &DirectionKeys, seq: u64, typ: u8, body: &mut [u8]) -> crate::Result<()> {
    let Some(layout) = msg::data_layout(typ, body) else {
        return keys
            .keystream(seq, 0, body)
            .map_err(|_| "chan: keystream counter overflow");
    };
    let params_len = layout.params_len.min(body.len());
    if params_len > 0 {
        keys.keystream(seq, 0, &mut body[..params_len])
            .map_err(|_| "chan: keystream counter overflow")?;
    }
    let mut offset = params_len;
    let mut page = 0usize;
    while offset < body.len() {
        let end = offset.saturating_add(PAGE_LEN).min(body.len());
        let counter = 1 + (PAGE_BLOCKS * page) as u32;
        keys.keystream(seq, counter, &mut body[offset..end])
            .map_err(|_| "chan: keystream counter overflow")?;
        offset = end;
        page += 1;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The plan 04-02 tracer functions, kept as thin wrappers so the tracer tests
// compile and pass unchanged (the session engine above is their successor).
// ---------------------------------------------------------------------------

/// Seal one packet: header, encrypted body, MAC (D-06, D-07).
///
/// The header is written field by field at the pinned offsets, the body is
/// encrypted in place from block counter 0, and the MAC covers the one
/// contiguous slice `[0, size - MAC_LEN)` and is appended last.
pub fn seal_packet(
    keys: &DirectionKeys,
    seq: u64,
    typ: u8,
    tag: u16,
    fid_handle: [u8; 8],
    body: &[u8],
) -> crate::Result<Vec<u8>> {
    // No TLV area travels in this plan, and no negotiated msize is known at
    // this layer, so only the 32-bit `Size` bound applies here.
    let size = packet_len(0, body.len(), u32::MAX)? as usize;
    let header = Header {
        size: size as u32,
        typ,
        tag,
        pad: 0,
        fid_handle,
        seq,
        ext_mask: 0,
        ext_len: 0,
    };
    let mut packet = alloc::vec![0u8; size];
    header.encode((&mut packet[..HEADER_LEN]).try_into().expect("the header slice is 28 bytes"));
    packet[HEADER_LEN..HEADER_LEN + body.len()].copy_from_slice(body);

    keys.keystream(seq, 0, &mut packet[HEADER_LEN..HEADER_LEN + body.len()])
        .map_err(|_| "chan: keystream counter overflow")?;

    let mut mac = keys.mac();
    mac.update(&packet[..size - MAC_LEN]);
    let tag_bytes = mac.finalize();
    packet[size - MAC_LEN..].copy_from_slice(&tag_bytes);
    Ok(packet)
}

/// Open one packet: bounds, MAC, sequence, header rules, then decrypt
/// (D-07, WIRE-03).
///
/// `Size` is the only field read before the MAC verifies, and only to bound
/// the packet; every other field is acted on only after verification
/// (T-04-11, T-04-12).  The returned body is the decrypted plaintext.
pub fn open_packet(
    keys: &DirectionKeys,
    expected_seq: u64,
    msize: u32,
    packet: &[u8],
) -> crate::Result<(u8, u16, [u8; 8], Vec<u8>)> {
    // The receive path decrypts in place and the public signature is
    // read-only (the tracer passes a shared slice), so one copy is made
    // here.  It is not a second parse: `Unverified::new` applies the bounds
    // to the copy first, exactly as `frame_one` does for plain 9P.
    let mut buf = packet.to_vec();
    let unverified = Unverified::new(&mut buf, msize)?;
    let size = unverified.len();

    // Verify before acting on any field but Size (D-07).
    let mut mac = keys.mac();
    mac.update(&buf[..size - MAC_LEN]);
    let tag: [u8; MAC_LEN] =
        buf[size - MAC_LEN..].try_into().expect("the tag slice is exactly MAC_LEN bytes");
    mac.verify(&tag).map_err(|_| "chan: packet MAC does not verify")?;

    // Only now read the header, check every field, and compare the sequence
    // number (RESEARCH Pattern 2 steps 3 and 4; WIRE-04).
    let header = Header::decode(
        (&buf[..HEADER_LEN]).try_into().expect("the header slice is exactly HEADER_LEN bytes"),
    );
    if header.seq != expected_seq {
        return Err("chan: unexpected sequence number");
    }
    check_header(&header, size)?;

    // Decrypt the body only: the TLV area travels cleartext and is covered by
    // the MAC.  Then hand the split packet to the decoder (WIRE-03).
    let body_start = HEADER_LEN + header.ext_len as usize;
    keys.keystream(header.seq, 0, &mut buf[body_start..size - MAC_LEN])
        .map_err(|_| "chan: keystream counter overflow")?;
    let verified = Verified::new_verified(header, &buf)?;
    Ok((header.typ, header.tag, header.fid_handle, verified.body().to_vec()))
}
