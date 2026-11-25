//! chan — the `9P2000.aead` envelope: the fixed header, the packet MAC and
//! the sealed body (WIRE-01, WIRE-03, WIRE-05; D-06, D-07, D-08, D-22).
//!
//! # The header (D-06)
//!
//! 28 bytes, in the RFC section 4.1 order and all multi-byte fields little
//! endian: `Size(4) Type(1) Tag(2) Pad(1) FidHandle(8) SeqNonce(8)
//! ExtMask(2) ExtTLVs Length(2)`.  The field list is the decision; the figure
//! "24" it was first written with was an arithmetic slip.  The offsets below
//! pin every field, and [`HEADER_LEN`] pins the total.
//!
//! # What the MAC covers (D-07)
//!
//! Every byte of the packet before the MAC, as one contiguous slice from
//! byte 0 of `Size` to the last ciphertext byte.  `Pad` and `ExtTLVs Length`
//! are therefore authenticated, and [`open_packet`] verifies before it acts
//! on any field but `Size` (which it reads only to bound the packet).
//!
//! # What is not here yet
//!
//! This is the thinnest real path: one sealed packet, one open.  TLVs,
//! message bodies, the transport state machine and re-keying are later plans
//! in this phase (D-22); `port/src/fid.rs` is plan 04-09's.

extern crate alloc;

use alloc::vec::Vec;

use crypto::session::DirectionKeys;

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

/// Seal one packet: header, encrypted body, MAC (D-06, D-07).
pub fn seal_packet(
    keys: &DirectionKeys,
    seq: u64,
    typ: u8,
    tag: u16,
    fid_handle: [u8; 8],
    body: &[u8],
) -> crate::Result<Vec<u8>> {
    // RED: the body compiles and is wrong — no header, no ciphertext, no MAC.
    let _ = (keys, seq, typ, tag, fid_handle, body);
    Ok(Vec::new())
}

/// Open one packet: bounds, MAC, sequence, then decrypt (D-07, WIRE-03).
pub fn open_packet(
    keys: &DirectionKeys,
    expected_seq: u64,
    msize: u32,
    packet: &[u8],
) -> crate::Result<(u8, u16, [u8; 8], Vec<u8>)> {
    // RED: the body compiles and is wrong — it accepts anything and returns
    // zeroed fields.
    let _ = (keys, expected_seq, msize, packet);
    Ok((0, 0, [0u8; 8], Vec::new()))
}
