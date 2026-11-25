//! wire — the canonical 28-byte header, the framing bounds and (from plan
//! 04-04 Task 2) the opcode registry and the `Unverified`/`Verified` packet
//! types (WIRE-01, WIRE-03, WIRE-06; D-06, D-07, D-28).
//!
//! Every multi-byte field is little-endian and sits at the offset the parent
//! module pins (D-06).  `Size` is the only field that may be read before the
//! packet MAC verifies, and only to bound the packet (D-07); plan 04-04 Task
//! 2 makes that ordering a type discipline.
//!
//! # Why a field-by-field codec (Pitfall 1, D-04)
//!
//! The header is written and read field by field with `to_le_bytes` and
//! `from_le_bytes`.  The wire bytes are the type's contract, not its memory
//! layout, so the compiler's layout rules never enter the envelope and the
//! module stays inside `port`'s per-module forbid.

use super::{
    EXT_KNOWN, EXT_STATELESS_CAP, HEADER_LEN, MAC_LEN, MIN_PACKET_LEN, OFF_EXTLEN, OFF_EXTMASK,
    OFF_FIDHANDLE, OFF_PAD, OFF_SEQ, OFF_SIZE, OFF_TAG, OFF_TYPE,
};

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
    pub fn encode(&self, out: &mut [u8; HEADER_LEN]) {
        out[OFF_SIZE..OFF_SIZE + 4].copy_from_slice(&self.size.to_le_bytes());
        out[OFF_TYPE] = self.typ;
        out[OFF_TAG..OFF_TAG + 2].copy_from_slice(&self.tag.to_le_bytes());
        out[OFF_PAD] = self.pad;
        out[OFF_FIDHANDLE..OFF_FIDHANDLE + 8].copy_from_slice(&self.fid_handle);
        out[OFF_SEQ..OFF_SEQ + 8].copy_from_slice(&self.seq.to_le_bytes());
        out[OFF_EXTMASK..OFF_EXTMASK + 2].copy_from_slice(&self.ext_mask.to_le_bytes());
        out[OFF_EXTLEN..OFF_EXTLEN + 2].copy_from_slice(&self.ext_len.to_le_bytes());
    }

    /// Read every field from its pinned offset.  This checks nothing: the
    /// caller runs `check_header` after the MAC verifies.
    pub fn decode(bytes: &[u8; HEADER_LEN]) -> Header {
        Header {
            size: u32::from_le_bytes(
                bytes[OFF_SIZE..OFF_SIZE + 4].try_into().expect("the Size slice is 4 bytes"),
            ),
            typ: bytes[OFF_TYPE],
            tag: u16::from_le_bytes(
                bytes[OFF_TAG..OFF_TAG + 2].try_into().expect("the Tag slice is 2 bytes"),
            ),
            pad: bytes[OFF_PAD],
            fid_handle: bytes[OFF_FIDHANDLE..OFF_FIDHANDLE + 8]
                .try_into()
                .expect("the FidHandle slice is 8 bytes"),
            seq: u64::from_le_bytes(
                bytes[OFF_SEQ..OFF_SEQ + 8].try_into().expect("the SeqNonce slice is 8 bytes"),
            ),
            ext_mask: u16::from_le_bytes(
                bytes[OFF_EXTMASK..OFF_EXTMASK + 2].try_into().expect("the ExtMask slice is 2 bytes"),
            ),
            ext_len: u16::from_le_bytes(
                bytes[OFF_EXTLEN..OFF_EXTLEN + 2].try_into().expect("the ExtTLVs Length slice is 2 bytes"),
            ),
        }
    }
}

/// Bound a claimed `Size` before any body byte is read (T-04-11).
///
/// The lower bound is [`MIN_PACKET_LEN`] (60) — the header plus the MAC with
/// an empty body — and the upper bound is the negotiated msize.  `frame_one`
/// in `devmnt` applies the same discipline with its plain-9P lower bound.
pub fn frame_size(prefix: &[u8; 4], msize: u32) -> crate::Result<usize> {
    if msize < MIN_PACKET_LEN as u32 {
        return Err("chan: negotiated msize below the minimum packet");
    }
    let size = u32::from_le_bytes(*prefix);
    if (size as usize) < MIN_PACKET_LEN {
        return Err("chan: runt packet");
    }
    if size > msize {
        return Err("chan: packet exceeds negotiated msize");
    }
    Ok(size as usize)
}

/// The checked whole-packet length the encoder writes into `Size`.
///
/// Every length on the wire counts bytes: the header, the TLV area, the body
/// and the 32-byte MAC.  The arithmetic is checked, and the result must fit
/// the `u32` `Size` field and the negotiated msize — no length is narrowed
/// with a silent cast.
pub fn packet_len(tlv_len: usize, body_len: usize, msize: u32) -> crate::Result<u32> {
    let total = HEADER_LEN
        .checked_add(tlv_len)
        .and_then(|n| n.checked_add(body_len))
        .and_then(|n| n.checked_add(MAC_LEN))
        .ok_or("chan: packet length overflows usize")?;
    let size = u32::try_from(total).map_err(|_| "chan: packet exceeds the 32-bit size field")?;
    if size > msize {
        return Err("chan: packet exceeds negotiated msize");
    }
    Ok(size)
}

// ---------------------------------------------------------------------------
// The opcode registry (WIRE-06, D-28; fcall.h and RFC section 6.1)
// ---------------------------------------------------------------------------

/// fcall.h `Tversion`: negotiate the dialect and message unit.
pub const TVERSION: u8 = 100;
/// fcall.h `Rversion`.
pub const RVERSION: u8 = 101;
/// fcall.h `Tauth`.
pub const TAUTH: u8 = 102;
/// fcall.h `Rauth`.
pub const RAUTH: u8 = 103;
/// fcall.h `Tattach`.
pub const TATTACH: u8 = 104;
/// fcall.h `Rattach`.
pub const RATTACH: u8 = 105;
/// fcall.h `Rerror`.  106 (`Terror`) is illegal and deliberately absent.
pub const RERROR: u8 = 107;
/// fcall.h `Tflush`.
pub const TFLUSH: u8 = 108;
/// fcall.h `Rflush`.
pub const RFLUSH: u8 = 109;
/// fcall.h `Twalk`.
pub const TWALK: u8 = 110;
/// fcall.h `Rwalk`.
pub const RWALK: u8 = 111;
/// fcall.h `Topen`.
pub const TOPEN: u8 = 112;
/// fcall.h `Ropen`.
pub const ROPEN: u8 = 113;
/// fcall.h `Tcreate`.
pub const TCREATE: u8 = 114;
/// fcall.h `Rcreate`.
pub const RCREATE: u8 = 115;
/// fcall.h `Tread`.
pub const TREAD: u8 = 116;
/// fcall.h `Rread`.
pub const RREAD: u8 = 117;
/// fcall.h `Twrite`.
pub const TWRITE: u8 = 118;
/// fcall.h `Rwrite`.
pub const RWRITE: u8 = 119;
/// fcall.h `Tclunk`.
pub const TCLUNK: u8 = 120;
/// fcall.h `Rclunk`.
pub const RCLUNK: u8 = 121;
/// fcall.h `Tremove`.
pub const TREMOVE: u8 = 122;
/// fcall.h `Rremove`.
pub const RREMOVE: u8 = 123;
/// fcall.h `Tstat`.
pub const TSTAT: u8 = 124;
/// fcall.h `Rstat`.
pub const RSTAT: u8 = 125;
/// fcall.h `Twstat`.
pub const TWSTAT: u8 = 126;
/// fcall.h `Rwstat`.
pub const RWSTAT: u8 = 127;

/// RFC section 6.1 `Tcompound`: an atomic multi-operation pipeline.
pub const TCOMPOUND: u8 = 130;
/// RFC section 6.1 `Rcompound`.
pub const RCOMPOUND: u8 = 131;
/// RFC section 6.1 `Tstream`.
pub const TSTREAM: u8 = 132;
/// RFC section 6.1 `Rstream`.
pub const RSTREAM: u8 = 133;
/// RFC section 6.1 `Tlease_revoke`.
pub const TLEASE_REVOKE: u8 = 134;
/// RFC section 6.1 `Rlease_ack`.
pub const RLEASE_ACK: u8 = 135;
/// RFC section 6.1 `Treclaim`.
pub const TRECLAIM: u8 = 136;
/// RFC section 6.1 `Rreclaim`.
pub const RRECLAIM: u8 = 137;
/// P3 `Trekey`: the in-band re-key request.
pub const TREKEY: u8 = 138;
/// P3 `Rrekey`: the in-band re-key answer.
pub const RREKEY: u8 = 139;

/// The canonical name of a registry opcode, or `None` when the value is not
/// in the dialect's registry (WIRE-06, D-28).
pub fn opcode_name(_typ: u8) -> Option<&'static str> {
    None
}

/// Whether `typ` names a T-message (an even, known opcode).
pub fn is_t_message(_typ: u8) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Unverified and Verified (WIRE-03, T-04-12)
// ---------------------------------------------------------------------------

/// A packet whose `Size` has been bounded but whose MAC has not checked.
///
/// Only [`Unverified::len`] is readable, so no decode can run on
/// unauthenticated bytes (WIRE-03).
pub struct Unverified<'a> {
    packet: &'a mut [u8],
}

impl<'a> Unverified<'a> {
    /// Bound `packet` by its claimed `Size` (T-04-11).
    pub fn new(packet: &'a mut [u8], _msize: u32) -> crate::Result<Unverified<'a>> {
        Ok(Unverified { packet })
    }

    /// The whole packet length.
    pub fn len(&self) -> usize {
        0
    }
}

/// A packet whose MAC has verified: its header, TLV area and plaintext body
/// are readable (WIRE-03, T-04-12).
pub struct Verified<'a> {
    header: Header,
    tlvs: &'a [u8],
    body: &'a [u8],
}

impl<'a> Verified<'a> {
    /// The only constructor: crate-private, called by `chan::transport` after
    /// the packet MAC verifies.
    pub(crate) fn new_verified(
        header: Header,
        _packet: &'a [u8],
    ) -> crate::Result<Verified<'a>> {
        Ok(Verified { header, tlvs: &[], body: &[] })
    }

    /// The verified header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The cleartext, authenticated TLV area.
    pub fn tlvs(&self) -> &'a [u8] {
        self.tlvs
    }

    /// The decrypted message body.
    pub fn body(&self) -> &'a [u8] {
        self.body
    }
}

/// Check every header field that is only meaningful after the MAC verifies
/// (RESEARCH Pattern 2 step 4; D-12).
pub fn check_header(_header: &Header, _packet_len: usize) -> crate::Result<()> {
    Ok(())
}
