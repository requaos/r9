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
//! # In-band re-keying (WIRE-04, D-18 to D-21)
//!
//! Either end starts a fresh `Tauth` exchange in band when its send counter
//! reaches `rekey_after_packets`; the exchange derives generation `g + 1` and
//! each direction cuts over under its own marker and acknowledgement, with an
//! acknowledgement deadline the caller's `now` drives.  A sender that would
//! reach `SEQ_HARD_LIMIT` with no new keys closes instead of wrapping.  A
//! connection owner must call `poll(now)` periodically so a withheld
//! acknowledgement closes the session; nothing in the kernel drives a session
//! this phase (D-22).

use alloc::vec::Vec;

use crypto::key::Key;
use crypto::session::{self, Direction, DirectionKeys, SessionKeys, Transcript};

use crate::dat::Qid;
use crate::mcslock::{Lock, LockNode};

use super::msg;
use super::tlv::{self, Exts};
use super::wire::{self, Header, Unverified, Verified, check_header, frame_size, packet_len};
use super::{HEADER_LEN, MAC_LEN, PAGE_LEN};

/// The auth fid the in-band re-key exchange rides (P9, D-18).
///
/// `0xFFFFFFFE` is one below `devmnt::NOFID`, which the allocator never issues,
/// and it is the afid the in-band `Tauth`, `Twrite` and `Tread` name.
const REKEY_AFID: u32 = 0xFFFF_FFFE;

/// The reserved tag every in-band exchange message carries (P9).
///
/// `devmnt` issues tags 1 to `0xFFFE` and never `NOTAG`, so a sealed message
/// with this tag is exchange or control traffic and never an ordinary request.
/// The same constant rebuilds the frame both ends absorb into the re-key
/// transcript, so the two transcripts agree byte for byte.
const REKEY_EXCHANGE_TAG: u16 = crate::devmnt::NOTAG;

/// The tag the re-key control messages (`Trekey` / `Rrekey`) carry (P3, P9).
const REKEY_TAG: u16 = crate::devmnt::NOTAG;

/// The sequence number that is never sent (P3, D-19).
///
/// A sender whose counter reaches this value with no new keys installed closes
/// the connection rather than wrap.
pub const SEQ_HARD_LIMIT: u64 = u64::MAX;

/// One message handed to the re-key router: its wire tag beside its decoded
/// body (DECISION P9).
///
/// The tag is the discriminator: a sealed message is in-band exchange traffic
/// only when its tag is [`crate::devmnt::NOTAG`] and its type is one of the
/// exchange or control types.  The caller opens the packet, decodes the body
/// and passes both here; a message [`is_rekey_traffic`] rejects belongs to the
/// caller's 9P layer and must never reach the re-key state machine.
pub struct RekeyIncoming<'a> {
    /// The packet header `Tag`.
    pub tag: u16,
    /// The decoded message body.
    pub message: &'a msg::Msg,
}

/// Whether a sealed message is in-band re-key traffic (DECISION P9).
///
/// True only under the reserved tag `NOTAG` (P9): for the control messages
/// `Trekey`/`Rrekey` (P3), and for the six exchange messages, where `Tauth`,
/// `Twrite` and `Tread` must additionally name the reserved afid.  The owner's
/// P9 amendment extends the same rule to `Tstream` (reserved tag and the
/// exchange afid in its 8-byte `FidHandle`) and `Rstream` (reserved tag; it
/// carries no fid).  A message this function rejects belongs to the caller's 9P
/// layer: it is never absorbed, never advances the exchange and never closes
/// the connection because an exchange is in flight.
pub fn is_rekey_traffic(tag: u16, message: &msg::Msg) -> bool {
    if tag != REKEY_EXCHANGE_TAG {
        return false;
    }
    match message {
        msg::Msg::Trekey { .. } | msg::Msg::Rrekey { .. } => true,
        msg::Msg::Tauth { afid, .. } => *afid == REKEY_AFID,
        // `Rauth` carries only a qid; `Rwrite` and `Rread` carry no fid, so the
        // reserved tag identifies all three.
        msg::Msg::Rauth { .. } => true,
        msg::Msg::Twrite { fid, .. } | msg::Msg::Tread { fid, .. } => *fid == REKEY_AFID,
        msg::Msg::Rwrite { .. } | msg::Msg::Rread { .. } => true,
        msg::Msg::Tstream { fid_handle, .. } => *fid_handle == exchange_stream_handle(),
        msg::Msg::Rstream { .. } => true,
        _ => false,
    }
}

/// The P9 afid as the 8-byte `FidHandle` a `Tstream` body names: the afid
/// little-endian in the first four bytes, the remaining four zero (P9).
fn exchange_stream_handle() -> [u8; 8] {
    let mut handle = [0u8; 8];
    handle[..4].copy_from_slice(&REKEY_AFID.to_le_bytes());
    handle
}

/// The state of a direction's superseded (old-generation) key slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Superseded {
    /// The direction has not cut over; no old keys are held.
    None,
    /// The old keys are still live: the sender between its marker and the ack.
    Held,
    /// The old keys were zeroized: the receiver at the marker, or the sender
    /// after a valid acknowledgement.
    Zeroized,
}

/// The index of a direction in the pending-key array.
fn dir_index(direction: Direction) -> usize {
    match direction {
        Direction::ClientToServer => 0,
        Direction::ServerToClient => 1,
    }
}

/// The direction a `Role` sends in.
fn send_direction_of(role: Role) -> Direction {
    match role {
        Role::Client => Direction::ClientToServer,
        Role::Server => Direction::ServerToClient,
    }
}

/// Where the in-band re-key exchange stands at this end (D-18, RESEARCH Pattern 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExPhase {
    Idle,
    /// Client sent `Tauth`, awaiting `Rauth`.
    AwaitingRauth,
    /// Client sent `Twrite`, awaiting `Rwrite`.
    AwaitingRwrite,
    /// Client sent `Tread`, awaiting `Rread`.
    AwaitingRread,
    /// Server sent `Rauth`, awaiting `Twrite`.
    AwaitingTwrite,
    /// Server sent `Rwrite`, awaiting `Tread`.
    AwaitingTread,
    /// The exchange finished and the pending keys were derived.
    Done,
}

/// The re-key state: the running exchange, the pending keys, the superseded
/// receive slot and the flags that decide when the next marker goes out.
struct Rekey {
    /// The derived generation `g + 1` keys, one slot per direction.
    pending: [Option<DirectionKeys>; 2],
    /// The chained transcript of the exchange in progress.
    transcript: Option<Transcript>,
    /// Our secret from the seam, once it returned `Done`.
    secret: Option<Key>,
    /// The bytes this end's seam most recently produced (the server's `Rread`).
    out: Vec<u8>,
    phase: ExPhase,
    /// An exchange is running at this end.
    active: bool,
    /// This end asked the peer to start an exchange (server threshold).
    request_sent: bool,
    /// The exchange finished and the pending keys are set.
    ready: bool,
    /// This end has sent its own direction's marker for this generation.
    own_marker_sent: bool,
    /// The old receive keys of the direction that was cut over.
    recv_superseded: Option<DirectionKeys>,
}

impl Rekey {
    fn new() -> Rekey {
        Rekey {
            pending: [None, None],
            transcript: None,
            secret: None,
            out: Vec::new(),
            phase: ExPhase::Idle,
            active: false,
            request_sent: false,
            ready: false,
            own_marker_sent: false,
            recv_superseded: None,
        }
    }

    /// Zeroize every key the re-key state holds: the exchange secret, the
    /// seam's output buffer, the in-progress transcript and the superseded and
    /// pending key slots.
    ///
    /// The values that stay in their slots are zeroed in place so
    /// `keys_zeroized` can read them; the exchange secret is dropped (its
    /// `Key` is `Zeroizing`) and the seam's buffer is zero-filled and cleared
    /// (CR-01).
    fn fail(&mut self) {
        // Drop (not just overwrite) the exchange secret: `Key` is
        // `Zeroizing`, so assigning `None` zeroes it on drop.
        self.secret = None;
        for byte in self.out.iter_mut() {
            *byte = 0;
        }
        self.out.clear();
        self.transcript = None;
        if let Some(keys) = &mut self.recv_superseded {
            keys.zeroize();
        }
        for keys in self.pending.iter_mut().flatten() {
            keys.zeroize();
        }
    }

    /// Whether every key the re-key state holds is zero.
    ///
    /// Covers the exchange secret (absent, not merely overwritten) and every
    /// byte of the seam's output buffer, beside the superseded and pending key
    /// slots (CR-01).
    fn keys_zeroized(&self) -> bool {
        self.secret.is_none()
            && self.out.iter().all(|&byte| byte == 0)
            && self.recv_superseded.as_ref().is_none_or(DirectionKeys::is_zeroized)
            && self.pending.iter().all(|slot| slot.as_ref().is_none_or(DirectionKeys::is_zeroized))
    }

    /// Begin an exchange: chain the previous transcript and reset the flags.
    fn start(&mut self, previous: &[u8; 32]) {
        self.transcript = Some(Transcript::chain(previous));
        self.secret = None;
        self.out.clear();
        self.phase = ExPhase::Idle;
        self.active = true;
        self.request_sent = false;
        self.ready = false;
        self.own_marker_sent = false;
    }
}

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
    /// The old keys of this direction after a cutover; `Held` until the ack,
    /// then zeroized, or zeroized at once if this end is the receiver side.
    superseded: Option<DirectionKeys>,
    /// The re-key ack deadline, in the caller's `now` milliseconds (D-20, F4).
    deadline: Option<u64>,
}

impl SendState {
    /// Zeroize this direction's keys (and any superseded slot), reset the
    /// counter and close.
    fn fail(&mut self) {
        self.keys.zeroize();
        if let Some(keys) = &mut self.superseded {
            keys.zeroize();
        }
        self.seq = 0;
        self.deadline = None;
        self.closed = true;
    }

    /// Whether this direction's keys and any superseded slot are zero.
    fn is_zeroized(&self) -> bool {
        self.keys.is_zeroized() && self.superseded.as_ref().is_none_or(DirectionKeys::is_zeroized)
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
    /// a packet the peer never saw would desynchronise the stream.  When a
    /// re-key acknowledgement deadline is outstanding and `now` has reached
    /// it, this closes the direction without writing; like [`Session::poll`],
    /// a deadline only fires when something calls in (F4, WR-02).
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
        if !self.authenticated {
            return Err(Closed);
        }
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        // The deadline check sits inside the send lock, before sealing, so a
        // withheld acknowledgement writes nothing (D-20 step 4, WR-02).
        if state.closed {
            return Err(Closed);
        }
        if state.deadline.is_some_and(|deadline| now >= deadline) {
            state.fail();
            return Err(Closed);
        }
        let seq = state.seq;
        let packet = seal_into(&mut state, self.msize, seq, typ, tag, fid_handle, exts, body)?;
        // The seal, the write and the advance all sit inside this one lock
        // scope, so two senders cannot put packets on the wire out of order
        // (D-21).  A write that does not land whole closes the direction.
        match io.write_msg(&packet) {
            Ok(written) if written as usize == packet.len() => {
                if let Some(next) = seq.checked_add(1) {
                    state.seq = next;
                } else {
                    state.fail();
                    return Err(Closed);
                }
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
        state.seq = seq.checked_add(1).ok_or(Closed)?;
        Ok(packet)
    }

    /// The marker send, the key switch and the counter reset in one lock scope
    /// (D-20, D-21, acceptance criteria).
    ///
    /// Seals the marker as the last packet under the old keys, moves those
    /// keys into the superseded slot (`Held`), installs `new_keys`, resets the
    /// counter to 0 and stores the acknowledgement deadline.
    fn marker_and_switch(
        &self,
        typ: u8,
        tag: u16,
        body: &[u8],
        new_keys: DirectionKeys,
        deadline: u64,
    ) -> core::result::Result<Vec<u8>, Closed> {
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        let seq = state.seq;
        let packet =
            seal_into(&mut state, self.msize, seq, typ, tag, [0u8; 8], &Exts::default(), body)?;
        let old = core::mem::replace(&mut state.keys, new_keys);
        state.superseded = Some(old);
        state.seq = 0;
        state.deadline = Some(deadline);
        Ok(packet)
    }

    /// Whether the send counter has reached `threshold` (plan 04-08).
    fn seq_reached(&self, threshold: u64) -> bool {
        let node = LockNode::new();
        self.state.lock(&node).seq >= threshold
    }

    /// The state of this direction's superseded key slot.
    fn superseded_state(&self) -> Superseded {
        let node = LockNode::new();
        let state = self.state.lock(&node);
        match &state.superseded {
            None => Superseded::None,
            Some(keys) if keys.is_zeroized() => Superseded::Zeroized,
            Some(_) => Superseded::Held,
        }
    }

    /// Verify a re-key confirm under this direction's current (new) MAC key.
    fn verify_confirm(&self, generation: u32, confirm: &[u8; 32]) -> bool {
        let node = LockNode::new();
        let state = self.state.lock(&node);
        state.keys.verify_rekey_confirm(generation, confirm).is_ok()
    }

    /// Zeroize the superseded slot after a valid acknowledgement (D-20 step 3).
    fn zeroize_superseded(&self) {
        let node = LockNode::new();
        let mut state = self.state.lock(&node);
        if let Some(keys) = &mut state.superseded {
            keys.zeroize();
        }
    }

    /// The stored acknowledgement deadline, if a marker is outstanding.
    fn deadline(&self) -> Option<u64> {
        let node = LockNode::new();
        self.state.lock(&node).deadline
    }

    /// Clear the acknowledgement deadline after a valid acknowledgement.
    fn clear_deadline(&self) {
        let node = LockNode::new();
        self.state.lock(&node).deadline = None;
    }

    /// Whether `now` has reached the stored acknowledgement deadline (F4).
    fn deadline_passed(&self, now: u64) -> bool {
        self.deadline().is_some_and(|deadline| now >= deadline)
    }

    /// Set the send counter, for the hidden host counter-limit constructor only.
    #[allow(dead_code)]
    fn set_seq(&self, seq: u64) {
        let node = LockNode::new();
        self.state.lock(&node).seq = seq;
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
    // The sender never sends sequence `u64::MAX`; with no new keys installed
    // it closes rather than wrap (P3, D-19).
    if seq == SEQ_HARD_LIMIT {
        state.fail();
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
                    superseded: None,
                    deadline: None,
                },
            ),
        };
        Session {
            config: self.config,
            generation: 0,
            transcript,
            send_direction: send_direction_of(self.role),
            send,
            recv_keys,
            recv_seq: 0,
            authenticated,
            rekey: Rekey::new(),
        }
    }

    /// Finish the handshake with a chosen send counter.
    ///
    /// Exists for the host counter-limit test only (D-17): no production path
    /// may start a session at a non-zero sequence number, and
    /// [`TransportConfig`] has no field for one.  Do not call this from kernel
    /// code.  [`Handshake::finish`] is unchanged and always starts at 0.
    #[doc(hidden)]
    pub fn finish_with_send_seq(self, secret: Key, send_seq: u64) -> Session {
        let session = self.finish(secret);
        session.send.set_seq(send_seq);
        session
    }
}

/// The agreed `9P2000.aead` session `devmnt` hands to the engine (WIRE-12, D-23).
///
/// Carries the two io halves out of the plain negotiation, the negotiated
/// msize and the exact `Tversion` and `Rversion` bytes, so the engine's
/// handshake transcript starts from the same cleartext bytes both ends saw
/// (D-17).
pub struct Handoff {
    /// The read half of the connection.
    pub io_r: alloc::boxed::Box<dyn crate::devmnt::MntIo>,
    /// The write half of the connection.
    pub io_w: alloc::boxed::Box<dyn crate::devmnt::MntIo>,
    /// The msize the server agreed to.
    pub msize: u32,
    /// The exact `Tversion` message this end sent, whole.
    pub tversion: Vec<u8>,
    /// The exact `Rversion` reply this end received, whole.
    pub rversion: Vec<u8>,
}

impl Handoff {
    /// Start the engine's handshake from the agreed negotiation (D-17).
    ///
    /// The exact `Tversion` and `Rversion` bytes are absorbed first, in wire
    /// order, so the transcript covers the same cleartext prefix both ends
    /// saw; the negotiated msize becomes the session's packet bound.  The rest
    /// of the plain handshake (`Tauth` / `Rauth` and the afid exchange) is
    /// absorbed by the caller through [`Handshake::absorb_plain`] before
    /// [`Handshake::finish`].
    pub fn into_handshake(
        self,
        role: Role,
        mut config: TransportConfig,
    ) -> (
        Handshake,
        alloc::boxed::Box<dyn crate::devmnt::MntIo>,
        alloc::boxed::Box<dyn crate::devmnt::MntIo>,
    ) {
        config.msize = self.msize;
        let mut handshake = Handshake::new(role, config);
        handshake.absorb_plain(&self.tversion);
        handshake.absorb_plain(&self.rversion);
        (handshake, self.io_r, self.io_w)
    }
}

/// One end of an established connection (D-15).
pub struct Session {
    config: TransportConfig,
    /// The current derivation generation (P2, D-18).
    generation: u32,
    /// The current generation's transcript hash; a re-key chains from it.
    transcript: [u8; 32],
    /// The direction this end seals in.
    send_direction: Direction,
    /// The sending direction: its keys, sequence counter and closed state,
    /// behind one lock so concurrent senders stay in counter order (D-21).
    send: SendHalf,
    /// The receiving direction's keys; the receive side is single-reader.
    recv_keys: DirectionKeys,
    recv_seq: u64,
    /// Whether `Tauth` and `Rauth` were both absorbed (D-16, D-17).
    authenticated: bool,
    /// The re-key state: pending keys, the running exchange and the received
    /// direction's superseded slot (D-18 to D-20).
    rekey: Rekey,
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
        if !self.authenticated {
            return Err(Closed);
        }
        if self.send.is_closed() {
            // The send half can close on its own (a deadline, a failed write),
            // so the session finishes the zeroization it cannot do (WR-02).
            self.fail();
            return Err(Closed);
        }
        // A re-key acknowledgement that never arrived closes the connection on
        // the next send (D-20 step 4).  A deadline only fires when something
        // calls in (F4).
        if self.send.deadline_passed(now) {
            self.fail();
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
        if !self.authenticated {
            return Err(Closed);
        }
        if self.send.is_closed() {
            // The session finishes the zeroization of a self-closed send half
            // (WR-02).
            self.fail();
            return Err(Closed);
        }
        // A missing acknowledgement closes the connection on the next receive
        // too (D-20 step 4); the deadline uses only the caller's `now` (F4).
        if self.send.deadline_passed(now) {
            self.fail();
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
    /// re-key state's pending and superseded keys, the exchange secret and the
    /// seam's output buffer join the same call.  The zeroized values stay in
    /// their slots, so `keys_zeroized` has something to read.
    fn fail(&mut self) {
        // The send keys sit behind the per-direction lock, so this reaches
        // them through it; the receive keys are single-reader and are
        // zeroized directly (D-15, D-21).  The re-key state's pending and
        // superseded keys join the same path.
        self.send.fail();
        self.recv_keys.zeroize();
        self.recv_seq = 0;
        self.rekey.fail();
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
    /// Covers both directions' keys, the re-key state's pending and
    /// superseded slots, the exchange secret and the seam's output buffer.
    /// Computed from the key storage, never from the closed flag, so a
    /// `fail()` or `teardown()` that set the flag without zeroizing answers
    /// `false`.  It reveals no key material, only `true` or `false`, and
    /// exists so an orderly teardown can be checked (D-15).  The send keys are
    /// read through the per-direction lock (D-21).
    pub fn keys_zeroized(&self) -> bool {
        self.send.keys_zeroized() && self.recv_keys.is_zeroized() && self.rekey.keys_zeroized()
    }

    /// Whether the in-band exchange secret and the seam's output buffer are
    /// zero.
    ///
    /// True exactly when the re-key state holds no exchange secret and every
    /// byte of the seam's output buffer is zero. Computed from that storage,
    /// never from a flag, so a failure path that skipped the scrub answers
    /// `false` (CR-01, WIRE-03). It reveals no key material, only `true` or
    /// `false` (WIRE-05 prohibition).
    pub fn exchange_secret_zeroized(&self) -> bool {
        self.rekey.secret.is_none() && self.rekey.out.iter().all(|&byte| byte == 0)
    }

    /// Whether a re-key is due: the send counter has reached the configured
    /// threshold and none is running or waiting to be acknowledged (D-19).
    ///
    /// The state is idle exactly when no exchange is running, no derived keys
    /// are waiting for a marker, no request is outstanding, both pending slots
    /// are empty and no acknowledgement deadline is outstanding; a later
    /// threshold crossing then starts another exchange (WR-01).
    pub fn rekey_due(&self) -> bool {
        !self.send.is_closed()
            && !self.rekey.active
            && !self.rekey.ready
            && !self.rekey.request_sent
            && self.rekey.pending[0].is_none()
            && self.rekey.pending[1].is_none()
            && self.send.deadline().is_none()
            && self.send.seq_reached(self.config.rekey_after_packets)
    }

    /// The state of a direction's superseded (old-generation) key slot.
    ///
    /// Computed from `is_zeroized()` on the slot, never from a flag, so a
    /// cutover that skipped its `zeroize` call reports `Held` (D-20 steps 2,
    /// 3 and 4).
    pub fn superseded_keys(&self, direction: Direction) -> Superseded {
        if direction == self.send_direction {
            self.send.superseded_state()
        } else {
            match &self.rekey.recv_superseded {
                None => Superseded::None,
                Some(keys) if keys.is_zeroized() => Superseded::Zeroized,
                Some(_) => Superseded::Held,
            }
        }
    }

    /// The deadline check without the close: shared by `poll`, `seal` and `open`.
    fn deadline_expired(&self, now: u64) -> bool {
        self.send.deadline_passed(now)
    }

    /// Drive the in-band re-key state machine one step and return the sealed
    /// packets to send next (D-18 to D-20, RESEARCH Pattern 4).
    ///
    /// `incoming` is one decoded message just received, with its header tag
    /// ([`RekeyIncoming`]); `None` lets this end emit whatever it has to send
    /// next (a `Tauth`, a `REQUEST` or its own marker).  The caller routes every
    /// opened message with [`is_rekey_traffic`] and hands ordinary messages to
    /// its 9P layer; the message that arrives here is exchange or control
    /// traffic.  The exchange travels sealed: this method seals each message
    /// under the session's current keys and the caller puts the returned
    /// packets on the wire.  Every deadline comparison uses only `now` (F4).
    pub fn rekey_step(
        &mut self,
        exchange: &mut dyn SessionExchange,
        incoming: Option<RekeyIncoming<'_>>,
        now: u64,
    ) -> core::result::Result<Vec<Vec<u8>>, Closed> {
        if !self.authenticated {
            return Err(Closed);
        }
        if self.send.is_closed() {
            // The session finishes the zeroization of a self-closed send half
            // (WR-02).
            self.fail();
            return Err(Closed);
        }
        if self.deadline_expired(now) {
            self.fail();
            return Err(Closed);
        }
        let mut packets = Vec::new();
        let emit = incoming.is_none();


        if let Some(incoming) = incoming {
            self.rekey_handle(incoming, exchange, now, &mut packets)?;
        }

        // With nothing to answer, emit a marker once the exchange finished, or
        // start an exchange when this end's threshold (or the peer's request)
        // asked for one.
        if emit {
            if self.rekey.ready && !self.rekey.own_marker_sent {
                self.send_own_marker(now, &mut packets)?;
            } else if !self.rekey.active && !self.rekey.ready && self.rekey_due() {
                if self.send_direction == Direction::ClientToServer {
                    self.begin_client_exchange(&mut packets)?;
                } else {
                    self.send_request(&mut packets)?;
                }
            }
        }

        Ok(packets)
    }

    /// Dispatch one decoded message into the re-key state machine.
    ///
    /// The tag is the discriminator (P9): control traffic must carry `NOTAG`
    /// (P3), exchange traffic is what [`is_rekey_traffic`] accepts, and any
    /// other message is ordinary traffic - it leaves every field of the re-key
    /// state as it was and returns no packet.  A message carrying the reserved
    /// tag that does not fit the route is a protocol violation and closes.
    fn rekey_handle(
        &mut self,
        incoming: RekeyIncoming<'_>,
        exchange: &mut dyn SessionExchange,
        now: u64,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        let RekeyIncoming { tag, message } = incoming;
        match message {
            msg::Msg::Trekey {
                kind,
                generation,
                confirm,
            }
            | msg::Msg::Rrekey {
                kind,
                generation,
                confirm,
            } => {
                // P3 pins NOTAG for every control message; any other tag is a
                // violation, whatever the kind.
                if tag != REKEY_TAG {
                    self.fail();
                    return Err(Closed);
                }
                match *kind {
                    msg::REKEY_REQUEST => {
                        // Only the client drives the exchange; a server already
                        // waiting ignores a stray request.
                        if self.send_direction == Direction::ClientToServer
                            && !self.rekey.active
                            && !self.rekey.ready
                        {
                            self.begin_client_exchange(packets)?;
                        }
                        Ok(())
                    }
                    msg::REKEY_MARKER => self.handle_marker(*generation, packets),
                    msg::REKEY_ACK => self.handle_ack(*generation, confirm),
                    _ => {
                        self.fail();
                        Err(Closed)
                    }
                }
            }
            msg::Msg::Tauth { .. }
            | msg::Msg::Rauth { .. }
            | msg::Msg::Twrite { .. }
            | msg::Msg::Rwrite { .. }
            | msg::Msg::Tread { .. }
            | msg::Msg::Rread { .. }
            | msg::Msg::Tstream { .. }
            | msg::Msg::Rstream { .. } => {
                if !is_rekey_traffic(tag, message) {
                    // A reserved-tag message that does not fit the exchange
                    // route (a wrong fid, say) reopens CR-02; any other tag is
                    // ordinary traffic and is never consumed (P9).
                    if tag == crate::devmnt::NOTAG {
                        self.fail();
                        return Err(Closed);
                    }
                    return Ok(());
                }
                self.exchange_incoming(message, exchange, now, packets)
            }
            _ => {
                // No other message type may carry the reserved tag.
                if tag == crate::devmnt::NOTAG {
                    self.fail();
                    return Err(Closed);
                }
                Ok(())
            }
        }
    }

    /// Begin an exchange as the client: chain the transcript, send `Tauth`.
    fn begin_client_exchange(
        &mut self,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        self.rekey.start(&self.transcript);
        let tauth = msg::Msg::Tauth {
            afid: REKEY_AFID,
            uname: b"none".to_vec(),
            aname: Vec::new(),
        };
        self.send_exchange(wire::TAUTH, REKEY_EXCHANGE_TAG, &tauth, packets)?;
        self.rekey.phase = ExPhase::AwaitingRauth;
        Ok(())
    }

    /// Ask the peer to start an exchange (a server that reached the threshold
    /// first): send `Rrekey` kind `REQUEST` (D-18, RESEARCH Pattern 4 step 1).
    fn send_request(&mut self, packets: &mut Vec<Vec<u8>>) -> core::result::Result<(), Closed> {
        let request = msg::Msg::Rrekey {
            kind: msg::REKEY_REQUEST,
            generation: self.generation.saturating_add(1),
            confirm: [0u8; 32],
        };
        self.send_control(&request, packets)?;
        self.rekey.request_sent = true;
        Ok(())
    }

    /// Send this end's own direction's marker, then cut that direction over.
    fn send_own_marker(
        &mut self,
        now: u64,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        let index = dir_index(self.send_direction);
        let Some(new_keys) = self.rekey.pending[index].take() else {
            self.fail();
            return Err(Closed);
        };
        let generation = self.generation;
        let marker = match self.send_direction {
            Direction::ClientToServer => msg::Msg::Trekey {
                kind: msg::REKEY_MARKER,
                generation,
                confirm: [0u8; 32],
            },
            Direction::ServerToClient => msg::Msg::Rrekey {
                kind: msg::REKEY_MARKER,
                generation,
                confirm: [0u8; 32],
            },
        };
        let mut body = Vec::new();
        if msg::encode_body(&marker, &mut body).is_err() {
            self.fail();
            return Err(Closed);
        }
        let (typ, tag) = self.control_type_tag();
        let deadline = now.saturating_add(self.config.rekey_ack_timeout_ms);
        let packet = self.send.marker_and_switch(typ, tag, &body, new_keys, deadline)?;
        packets.push(packet);
        self.rekey.own_marker_sent = true;
        Ok(())
    }

    /// The receive side of a marker: switch keys, zeroize the old ones and (in
    /// Task 2) answer with an acknowledgement.
    fn handle_marker(
        &mut self,
        generation: u32,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        let index = dir_index(self.recv_direction());
        if generation != self.generation || self.rekey.pending[index].is_none() {
            self.fail();
            return Err(Closed);
        }
        let Some(new_keys) = self.rekey.pending[index].take() else {
            self.fail();
            return Err(Closed);
        };
        let old = core::mem::replace(&mut self.recv_keys, new_keys);
        self.rekey.recv_superseded = Some(old);
        if let Some(keys) = &mut self.rekey.recv_superseded {
            keys.zeroize();
        }
        self.recv_seq = 0;
        self.send_ack(generation, packets)
    }

    /// Answer a marker with the acknowledgement carrying the confirm value
    /// under the new receive-direction MAC key (D-20 step 2, P3).
    fn send_ack(
        &mut self,
        generation: u32,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        // The confirm is made under the new receive-direction MAC key: sealing
        // the ACK under the other direction's keys proves nothing about the
        // new ones (D-20 step 2, P3).
        let confirm = self.recv_keys.rekey_confirm(generation);
        let ack = match self.send_direction {
            Direction::ClientToServer => msg::Msg::Trekey {
                kind: msg::REKEY_ACK,
                generation,
                confirm,
            },
            Direction::ServerToClient => msg::Msg::Rrekey {
                kind: msg::REKEY_ACK,
                generation,
                confirm,
            },
        };
        self.send_control(&ack, packets)
    }

    /// Verify an acknowledgement: generation, then the confirm value under the
    /// new send keys; a valid one zeroizes the old keys and returns this end's
    /// own-direction state to idle (D-20 step 3, WR-01).
    fn handle_ack(
        &mut self,
        generation: u32,
        confirm: &[u8; 32],
    ) -> core::result::Result<(), Closed> {
        // A wrong generation, an ACK with no marker outstanding, or a failed
        // confirm closes the connection; a valid one zeroizes the old keys
        // (D-20 step 3, P3).
        if generation != self.generation
            || self.send.deadline().is_none()
            || !self.send.verify_confirm(generation, confirm)
        {
            self.fail();
            return Err(Closed);
        }
        self.send.zeroize_superseded();
        self.send.clear_deadline();
        // This direction's cutover is complete: return to idle so a later
        // threshold crossing starts another exchange (WR-01). A pending slot
        // of the other direction may still be held until the peer's marker
        // arrives; `rekey_due` and the `Tauth` arm guard against overlapping.
        self.rekey.ready = false;
        self.rekey.own_marker_sent = false;
        self.rekey.request_sent = false;
        self.rekey.active = false;
        self.rekey.phase = ExPhase::Idle;
        Ok(())
    }

    /// The direction this end receives in.
    fn recv_direction(&self) -> Direction {
        match self.send_direction {
            Direction::ClientToServer => Direction::ServerToClient,
            Direction::ServerToClient => Direction::ClientToServer,
        }
    }

    /// The type and tag a control message from this end carries.
    fn control_type_tag(&self) -> (u8, u16) {
        match self.send_direction {
            Direction::ClientToServer => (wire::TREKEY, REKEY_TAG),
            Direction::ServerToClient => (wire::RREKEY, REKEY_TAG),
        }
    }

    /// Encode, absorb and seal one exchange message, appending its packet.
    fn send_exchange(
        &mut self,
        typ: u8,
        tag: u16,
        message: &msg::Msg,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        let mut body = Vec::new();
        if msg::encode_body(message, &mut body).is_err() {
            self.fail();
            return Err(Closed);
        }
        self.absorb_exchange(typ, tag, &body);
        let packet = self.send.seal_single(typ, tag, [0u8; 8], &Exts::default(), &body)?;
        packets.push(packet);
        Ok(())
    }

    /// Encode and seal one control message (`REQUEST`, `MARKER` or `ACK`).
    fn send_control(
        &mut self,
        message: &msg::Msg,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        let mut body = Vec::new();
        if msg::encode_body(message, &mut body).is_err() {
            self.fail();
            return Err(Closed);
        }
        let (typ, tag) = self.control_type_tag();
        let packet = self.send.seal_single(typ, tag, [0u8; 8], &Exts::default(), &body)?;
        packets.push(packet);
        Ok(())
    }

    /// Absorb one exchange message in its standard 9P2000 form.
    fn absorb_exchange(&mut self, typ: u8, tag: u16, body: &[u8]) {
        let mut frame = Vec::with_capacity(7 + body.len());
        frame.extend_from_slice(&((7 + body.len()) as u32).to_le_bytes());
        frame.push(typ);
        frame.extend_from_slice(&tag.to_le_bytes());
        frame.extend_from_slice(body);
        if let Some(transcript) = &mut self.rekey.transcript {
            transcript.absorb(&frame);
        }
    }

    /// Absorb a received exchange message by re-encoding its body.
    fn absorb_exchange_incoming(
        &mut self,
        typ: u8,
        tag: u16,
        message: &msg::Msg,
    ) -> core::result::Result<(), Closed> {
        let mut body = Vec::new();
        if msg::encode_body(message, &mut body).is_err() {
            self.fail();
            return Err(Closed);
        }
        self.absorb_exchange(typ, tag, &body);
        Ok(())
    }

    /// Run one step of the in-band exchange on whichever side this end is.
    fn exchange_incoming(
        &mut self,
        message: &msg::Msg,
        exchange: &mut dyn SessionExchange,
        _now: u64,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        match self.send_direction {
            Direction::ClientToServer => self.client_exchange_incoming(message, exchange, packets),
            Direction::ServerToClient => self.server_exchange_incoming(message, exchange, packets),
        }
    }

    /// The client half of the in-band exchange (D-18, Pattern 4).
    fn client_exchange_incoming(
        &mut self,
        message: &msg::Msg,
        exchange: &mut dyn SessionExchange,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        match (self.rekey.phase, message) {
            (ExPhase::AwaitingRauth, msg::Msg::Rauth { .. }) => {
                self.absorb_exchange_incoming(wire::RAUTH, REKEY_EXCHANGE_TAG, message)?;
                let mut out = Vec::new();
                match exchange.step(&[], &mut out) {
                    Ok(ExchangeStep::Done(secret)) => self.rekey.secret = Some(secret),
                    Ok(ExchangeStep::Continue) => {}
                    Err(_) => {
                        self.fail();
                        return Err(Closed);
                    }
                }
                let twrite = msg::Msg::Twrite {
                    fid: REKEY_AFID,
                    offset: 0,
                    data: out,
                };
                self.send_exchange(wire::TWRITE, REKEY_EXCHANGE_TAG, &twrite, packets)?;
                self.rekey.phase = ExPhase::AwaitingRwrite;
                Ok(())
            }
            (ExPhase::AwaitingRwrite, msg::Msg::Rwrite { .. }) => {
                self.absorb_exchange_incoming(wire::RWRITE, REKEY_EXCHANGE_TAG, message)?;
                let tread = msg::Msg::Tread {
                    fid: REKEY_AFID,
                    offset: 0,
                    count: 65536,
                };
                self.send_exchange(wire::TREAD, REKEY_EXCHANGE_TAG, &tread, packets)?;
                self.rekey.phase = ExPhase::AwaitingRread;
                Ok(())
            }
            (ExPhase::AwaitingRread, msg::Msg::Rread { data }) => {
                self.absorb_exchange_incoming(wire::RREAD, REKEY_EXCHANGE_TAG, message)?;
                if self.rekey.secret.is_none() {
                    let mut out = Vec::new();
                    match exchange.step(data, &mut out) {
                        Ok(ExchangeStep::Done(secret)) => self.rekey.secret = Some(secret),
                        Ok(ExchangeStep::Continue) => {
                            let twrite = msg::Msg::Twrite {
                                fid: REKEY_AFID,
                                offset: 0,
                                data: out,
                            };
                            self.send_exchange(wire::TWRITE, REKEY_EXCHANGE_TAG, &twrite, packets)?;
                            self.rekey.phase = ExPhase::AwaitingRwrite;
                            return Ok(());
                        }
                        Err(_) => {
                            self.fail();
                            return Err(Closed);
                        }
                    }
                }
                self.finish_exchange()
            }
            _ => {
                self.fail();
                Err(Closed)
            }
        }
    }

    /// The server half of the in-band exchange (D-18, Pattern 4).
    fn server_exchange_incoming(
        &mut self,
        message: &msg::Msg,
        exchange: &mut dyn SessionExchange,
        packets: &mut Vec<Vec<u8>>,
    ) -> core::result::Result<(), Closed> {
        match (self.rekey.phase, message) {
            (ExPhase::Idle, msg::Msg::Tauth { afid, .. }) => {
                // P9: the in-band Tauth names the reserved afid.  The router
                // already checked it; the arm checks again so the exchange
                // cannot advance on a wrong fid whatever the caller passed.
                if *afid != REKEY_AFID {
                    self.fail();
                    return Err(Closed);
                }
                // A previous generation's pending keys must be fully taken
                // before another exchange starts: two generations in flight for
                // one direction is the overlap D-20 forbids (WR-01).
                if self.rekey.pending[0].is_some() || self.rekey.pending[1].is_some() {
                    self.fail();
                    return Err(Closed);
                }
                self.rekey.start(&self.transcript);
                self.absorb_exchange_incoming(wire::TAUTH, REKEY_EXCHANGE_TAG, message)?;
                let rauth = msg::Msg::Rauth {
                    aqid: Qid {
                        path: 0,
                        vers: 0,
                        typ: 0,
                    },
                };
                self.send_exchange(wire::RAUTH, REKEY_EXCHANGE_TAG, &rauth, packets)?;
                self.rekey.phase = ExPhase::AwaitingTwrite;
                Ok(())
            }
            (ExPhase::AwaitingTwrite, msg::Msg::Twrite { fid, data, .. }) => {
                if *fid != REKEY_AFID {
                    self.fail();
                    return Err(Closed);
                }
                self.absorb_exchange_incoming(wire::TWRITE, REKEY_EXCHANGE_TAG, message)?;
                let mut out = Vec::new();
                let step = match exchange.step(data, &mut out) {
                    Ok(step) => step,
                    Err(_) => {
                        self.fail();
                        return Err(Closed);
                    }
                };
                self.rekey.out = out;
                if let ExchangeStep::Done(secret) = step {
                    self.rekey.secret = Some(secret);
                }
                let rwrite = msg::Msg::Rwrite {
                    count: data.len() as u32,
                };
                self.send_exchange(wire::RWRITE, REKEY_EXCHANGE_TAG, &rwrite, packets)?;
                if self.rekey.secret.is_some() {
                    self.rekey.phase = ExPhase::AwaitingTread;
                }
                Ok(())
            }
            (ExPhase::AwaitingTread, msg::Msg::Tread { fid, .. }) => {
                if *fid != REKEY_AFID {
                    self.fail();
                    return Err(Closed);
                }
                self.absorb_exchange_incoming(wire::TREAD, REKEY_EXCHANGE_TAG, message)?;
                let data = core::mem::take(&mut self.rekey.out);
                let rread = msg::Msg::Rread { data };
                self.send_exchange(wire::RREAD, REKEY_EXCHANGE_TAG, &rread, packets)?;
                self.finish_exchange()
            }
            _ => {
                self.fail();
                Err(Closed)
            }
        }
    }

    /// Derive generation `g + 1` from the seam's secret and the chained
    /// transcript, and hold the keys as pending (D-18, Pattern 4 step 2).
    fn finish_exchange(&mut self) -> core::result::Result<(), Closed> {
        let Some(secret) = self.rekey.secret.take() else {
            self.fail();
            return Err(Closed);
        };
        let Some(transcript) = self.rekey.transcript.take() else {
            self.fail();
            return Err(Closed);
        };
        let hash = transcript.hash();
        let next = self.generation.saturating_add(1);
        let keys = session::derive(&secret, &hash, next);
        self.rekey.pending[0] = Some(keys.c2s);
        self.rekey.pending[1] = Some(keys.s2c);
        self.generation = next;
        self.transcript = hash;
        self.rekey.ready = true;
        self.rekey.own_marker_sent = false;
        self.rekey.active = false;
        self.rekey.phase = ExPhase::Done;
        Ok(())
    }

    /// Check a stored acknowledgement deadline and close if it passed.
    ///
    /// A deadline only fires when something calls in: whoever owns the
    /// connection must call [`Session::poll`] periodically (F4, D-20 step 4).
    pub fn poll(&mut self, now: u64) -> core::result::Result<(), Closed> {
        if !self.authenticated {
            return Err(Closed);
        }
        if self.send.is_closed() {
            // The session finishes the zeroization of a self-closed send half
            // (WR-02).
            self.fail();
            return Err(Closed);
        }
        if self.deadline_expired(now) {
            self.fail();
            return Err(Closed);
        }
        Ok(())
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

/// Seal one packet: header, encrypted body, MAC (D-06, D-07, P5).
///
/// The header is written field by field at the pinned offsets, the body is
/// encrypted in place through [`crypt_body`] — parameters from ChaCha20 block
/// counter 0 and data page `i` from `1 + 64 * i`, the same placement
/// [`Session::seal`] uses — and the MAC covers the one contiguous slice
/// `[0, size - MAC_LEN)` and is appended last.  It is the tracer's thin path
/// beside `Session` (WR-03).
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

    crypt_body(keys, seq, typ, &mut packet[HEADER_LEN..HEADER_LEN + body.len()])?;

    let mut mac = keys.mac();
    mac.update(&packet[..size - MAC_LEN]);
    let tag_bytes = mac.finalize();
    packet[size - MAC_LEN..].copy_from_slice(&tag_bytes);
    Ok(packet)
}

/// Open one packet: bounds, MAC, sequence, header rules, then decrypt
/// (D-07, WIRE-03, P5).
///
/// `Size` is the only field read before the MAC verifies, and only to bound
/// the packet; every other field is acted on only after verification
/// (T-04-11, T-04-12).  The body is decrypted through [`crypt_body`], the same
/// P5 placement [`Session::open`] uses, and the returned body is the decrypted
/// plaintext.  It is the tracer's thin path beside `Session` (WR-03).
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
    crypt_body(keys, header.seq, header.typ, &mut buf[body_start..size - MAC_LEN])?;
    let verified = Verified::new_verified(header, &buf)?;
    Ok((header.typ, header.tag, header.fid_handle, verified.body().to_vec()))
}
