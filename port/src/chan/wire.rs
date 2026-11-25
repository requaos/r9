//! wire — the canonical 28-byte header, the framing bounds and (from plan
//! 04-04 Task 2) the opcode registry and the `Unverified`/`Verified` packet
//! types (WIRE-01, WIRE-03, WIRE-06; D-06, D-07, D-28).
//!
//! Every multi-byte field is little-endian and sits at the offset the parent
//! module pins (D-06).  `Size` is the only field that may be read before the
//! packet MAC verifies, and only to bound the packet (D-07); plan 04-04 Task
//! 2 makes that ordering a type discipline.
//!
//! # Why no `repr(C, packed)` (Pitfall 1, D-04)
//!
//! The header is written and read field by field with `to_le_bytes` and
//! `from_le_bytes`.  There is no `unsafe`, no `transmute` and no packed
//! struct: the wire bytes are the type's contract, not its memory layout.

use super::HEADER_LEN;

/// The fixed 28-byte envelope header (D-06).
///
/// `size` is the whole packet length including the 32-byte MAC; `typ` is a
/// registry opcode (`opcode_name`, Task 2); `pad` must be zero; `fid_handle`
/// is the message's primary handle (or all zero with a capability TLV);
/// `seq` is the per-direction sequence number; `ext_mask` is the `ExtMask`
/// bitmask and `ext_len` the length of the `ExtTLVs` area that follows the
/// header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// The whole packet length, including the MAC.
    pub size: u32,
    /// The 9P message type.
    pub typ: u8,
    /// The 9P tag, little-endian.
    pub tag: u16,
    /// Must be zero (D-06).
    pub pad: u8,
    /// The 8-byte handle field.
    pub fid_handle: [u8; 8],
    /// The per-direction sequence number.
    pub seq: u64,
    /// The extension bitmask.
    pub ext_mask: u16,
    /// The TLV area length that follows the header.
    pub ext_len: u16,
}

impl Header {
    /// Write every field at its pinned offset, little-endian.
    pub fn encode(&self, _out: &mut [u8; HEADER_LEN]) {}

    /// Read every field from its pinned offset.  This checks nothing: the
    /// caller runs `check_header` after the MAC verifies.
    pub fn decode(_bytes: &[u8; HEADER_LEN]) -> Header {
        Header {
            size: 0,
            typ: 0,
            tag: 0,
            pad: 0,
            fid_handle: [0u8; 8],
            seq: 0,
            ext_mask: 0,
            ext_len: 0,
        }
    }
}

/// Bound a claimed `Size` before any body byte is read (T-04-11).
pub fn frame_size(_prefix: &[u8; 4], _msize: u32) -> crate::Result<usize> {
    Ok(0)
}

/// The checked whole-packet length the encoder writes into `Size`.
pub fn packet_len(_tlv_len: usize, _body_len: usize, _msize: u32) -> crate::Result<u32> {
    Ok(0)
}
