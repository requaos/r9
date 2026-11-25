//! chan — the `9P2000.aead` envelope: the fixed header, the packet MAC and
//! the sealed body (WIRE-01, WIRE-03, WIRE-05; D-06, D-07, D-08, D-22).
//!
//! # The header (D-06)
//!
//! 28 bytes, in the RFC section 4.1 order and all multi-byte fields little
//! endian: `Size(4) Type(1) Tag(2) Pad(1) FidHandle(8) SeqNonce(8)
//! ExtMask(2) ExtTLVs Length(2)`.  The field list is the decision; the figure
//! "24" it was first written with was an arithmetic slip.  The offsets below
//! pin every field, and [`HEADER_LEN`] pins the total.  [`wire`] holds the
//! canonical codec for the same layout.
//!
//! # What the MAC covers (D-07)
//!
//! Every byte of the packet before the MAC, as one contiguous slice from
//! byte 0 of `Size` to the last ciphertext byte.  `Pad` and `ExtTLVs Length`
//! are therefore authenticated, and [`open_packet`] verifies before it acts
//! on any field but `Size` (which it reads only to bound the packet).
//!
//! # Module split (plan 04-04)
//!
//! [`wire`] carries the header codec, the framing bounds and the opcode
//! registry; [`transport`] carries sealing and opening; [`tlv`] and [`msg`]
//! are doc-only here and are filled in by later plans 04-06 and 04-05.  The
//! constants stay at this module root so every submodule shares one pin.
//!
//! # What is not here yet
//!
//! This is still a thin real path: one sealed packet, one open.  TLVs,
//! message bodies, the transport state machine and re-keying are later plans
//! in this phase (D-22); `port/src/fid.rs` is plan 04-09's.

pub mod msg;
pub mod tlv;
pub mod transport;
pub mod wire;

pub use transport::{
    Closed, ExchangeStep, Handshake, REKEY_ACK_TIMEOUT_MS, REKEY_AFTER_PACKETS, Role, SendHalf,
    Session, SessionExchange, TransportConfig, open_packet, seal_packet,
};

/// The fixed envelope header length (D-06).
pub const HEADER_LEN: usize = 28;

/// The packet MAC length in bytes (D-01, D-06).
pub const MAC_LEN: usize = 32;

/// The dialect version string (D-17).
pub const VERSION_AEAD: &[u8] = b"9P2000.aead";

/// `Size` offset: the whole packet length, little-endian (D-06).
pub const OFF_SIZE: usize = 0;
/// `Type` offset: the 9P message type (D-06).
pub const OFF_TYPE: usize = 4;
/// `Tag` offset: the 9P tag, little-endian (D-06).
pub const OFF_TAG: usize = 5;
/// `Pad` offset: must be zero (D-06).
pub const OFF_PAD: usize = 7;
/// `FidHandle` offset: the 8-byte handle (D-06).
pub const OFF_FIDHANDLE: usize = 8;
/// `SeqNonce` offset: the per-direction sequence number (D-06, D-08).
pub const OFF_SEQ: usize = 16;
/// `ExtMask` offset: the extension bitmask (D-06).
pub const OFF_EXTMASK: usize = 24;
/// `ExtTLVs Length` offset: the TLV area length (D-06).
pub const OFF_EXTLEN: usize = 26;

/// The 4 KiB page unit of every data body (D-09).
pub const PAGE_LEN: usize = 4096;

/// The smallest complete packet: the header and the MAC, with an empty body.
pub const MIN_PACKET_LEN: usize = HEADER_LEN + MAC_LEN;

/// `EXT_DEADLINE` (P1): the deadline extension bit; value 8 bytes.
pub const EXT_DEADLINE: u16 = 1 << 0;
/// `EXT_CAP_TOKEN` (P1): the per-user capability token; value 32 bytes.
pub const EXT_CAP_TOKEN: u16 = 1 << 1;
/// `EXT_QID_VOLATILITY` (P1): the volatility class record.
pub const EXT_QID_VOLATILITY: u16 = 1 << 2;
/// `EXT_STREAM_PRIO` (P1): the stream priority record; value 4 bytes.
pub const EXT_STREAM_PRIO: u16 = 1 << 3;
/// `EXT_OBINARY` (P1): the binary-open marker; value 0 bytes.
pub const EXT_OBINARY: u16 = 1 << 4;
/// `EXT_STATELESS_CAP` (P1, D-11): the capability TLV bit; value 24 bytes.
pub const EXT_STATELESS_CAP: u16 = 1 << 5;
/// Every `ExtMask` bit this dialect knows (P1): the OR of the six above.
pub const EXT_KNOWN: u16 = 0x003F;
