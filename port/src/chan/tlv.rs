//! tlv — the `ExtMask` bits and the `ExtTLVs` record parser (WIRE-02, D-11,
//! D-12, D-27, P1).
//!
//! # The record format (P1)
//!
//! Each extension record is `type[1] reserved[1] len[2] value[len] padding`, all
//! multi-byte fields little-endian:
//!
//! | bytes | field |
//! |-------|-------|
//! | 1 | `type`: the `ExtMask` bit index, 0 to 5 |
//! | 1 | `reserved`: zero |
//! | 2 | `len`: the value length in bytes, unpadded, little-endian |
//! | `len` | the value |
//! | 0 to 7 | zero padding so the record is a multiple of 8 bytes |
//!
//! # 64-bit alignment
//!
//! "64-bit aligned" means each record is a multiple of 8 bytes measured from
//! the start of the TLV area.  The area starts at packet offset 28, which is
//! not a multiple of 8; that does not matter, because [`parse`] borrows the
//! bytes and reads fields with `from_le_bytes` — no alignment cast and no
//! allocation.
//!
//! # The value layouts
//!
//! - `EXT_DEADLINE`: 8 bytes, a little-endian nanosecond deadline.
//! - `EXT_CAP_TOKEN`: 32 bytes, the per-user capability token (D-11; its only
//!   meaning).
//! - `EXT_QID_VOLATILITY`: `class[1] lease_gen[8] granted_mode[1]`.  `class` is
//!   exactly one of 0x20, 0x40, 0x80 (D-27) and is never related to
//!   `dat::Qid`'s type byte.
//! - `EXT_STREAM_PRIO`: 4 bytes, `tier << 30 | credit` ([`stream_prio_parts`]).
//! - `EXT_OBINARY`: 0 bytes; the record's presence is the marker.
//! - `EXT_STATELESS_CAP`: 24 bytes (D-11, D-12).

use alloc::vec::Vec;

use super::{
    EXT_CAP_TOKEN, EXT_DEADLINE, EXT_OBINARY, EXT_QID_VOLATILITY, EXT_STATELESS_CAP,
    EXT_STREAM_PRIO,
};

/// The fixed record header length: `type[1] reserved[1] len[2]`.
pub const TLV_HEADER_LEN: usize = 4;

/// `EXT_DEADLINE`'s fixed value length (P1).
pub const LEN_DEADLINE: usize = 8;
/// `EXT_CAP_TOKEN`'s fixed value length (P1, D-11).
pub const LEN_CAP_TOKEN: usize = 32;
/// `EXT_QID_VOLATILITY`'s fixed value length: `class[1] lease_gen[8] mode[1]`.
pub const LEN_QID_VOLATILITY: usize = 10;
/// `EXT_STREAM_PRIO`'s fixed value length (P1).
pub const LEN_STREAM_PRIO: usize = 4;
/// `EXT_OBINARY`'s fixed value length: the record carries no value.
pub const LEN_OBINARY: usize = 0;
/// `EXT_STATELESS_CAP`'s fixed value length (D-11, D-12).
pub const LEN_STATELESS_CAP: usize = 24;

/// The `EXT_QID_VOLATILITY` value: the volatility class, the lease generation
/// and the granted mode (D-27).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QidVolatility {
    /// `QTIMMUTABLE` 0x20, `QTLEASED` 0x40 or `QTVOLATILE` 0x80 (D-27).
    pub class: u8,
    /// The lease epoch.
    pub lease_gen: u64,
    /// 0 none, 1 shared read, 2 exclusive write (RFC section 4.4).
    pub granted_mode: u8,
}

/// The extension values a packet carries, borrowed from the parsed TLV area.
///
/// The 32-byte and 24-byte values are references into `area`, so a parse owns
/// no `Vec` and copies nothing (WIRE-02, zero-copy).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Exts<'a> {
    /// `EXT_DEADLINE`: an 8-byte nanosecond deadline.
    pub deadline: Option<u64>,
    /// `EXT_CAP_TOKEN`: the 32-byte per-user token.
    pub cap_token: Option<&'a [u8; LEN_CAP_TOKEN]>,
    /// `EXT_QID_VOLATILITY`: the volatility class record.
    pub qid_volatility: Option<QidVolatility>,
    /// `EXT_STREAM_PRIO`: `tier << 30 | credit`.
    pub stream_prio: Option<u32>,
    /// `EXT_OBINARY`: the binary-open marker.
    pub obinary: bool,
    /// `EXT_STATELESS_CAP`: the 24-byte capability (D-11, D-12).
    pub stateless_cap: Option<&'a [u8; LEN_STATELESS_CAP]>,
}

impl Exts<'_> {
    /// The `ExtMask` bits this value sets.
    pub fn mask(&self) -> u16 {
        let mut mask = 0;
        if self.deadline.is_some() {
            mask |= EXT_DEADLINE;
        }
        if self.cap_token.is_some() {
            mask |= EXT_CAP_TOKEN;
        }
        if self.qid_volatility.is_some() {
            mask |= EXT_QID_VOLATILITY;
        }
        if self.stream_prio.is_some() {
            mask |= EXT_STREAM_PRIO;
        }
        if self.obinary {
            mask |= EXT_OBINARY;
        }
        if self.stateless_cap.is_some() {
            mask |= EXT_STATELESS_CAP;
        }
        mask
    }

    /// The total padded length of this value's records.
    pub fn encoded_len(&self) -> usize {
        let mut len = 0;
        if self.deadline.is_some() {
            len += record_len(LEN_DEADLINE);
        }
        if self.cap_token.is_some() {
            len += record_len(LEN_CAP_TOKEN);
        }
        if self.qid_volatility.is_some() {
            len += record_len(LEN_QID_VOLATILITY);
        }
        if self.stream_prio.is_some() {
            len += record_len(LEN_STREAM_PRIO);
        }
        if self.obinary {
            len += record_len(LEN_OBINARY);
        }
        if self.stateless_cap.is_some() {
            len += record_len(LEN_STATELESS_CAP);
        }
        len
    }
}

/// The padded length of a record whose value is `value_len` bytes.
const fn record_len(value_len: usize) -> usize {
    (TLV_HEADER_LEN + value_len).next_multiple_of(8)
}

/// Parse the `ExtTLVs` area named by `mask` (WIRE-02, P1).
pub fn parse(mask: u16, area: &[u8]) -> crate::Result<Exts<'_>> {
    let mut exts = Exts::default();
    let mut pos = 0usize;
    for bit_index in 0..6u8 {
        let bit = 1u16 << bit_index;
        if mask & bit == 0 {
            continue;
        }
        let Some(header) = area.get(pos..pos + TLV_HEADER_LEN) else {
            return Err("chan: tlv set bit has no record");
        };
        let len = u16::from_le_bytes([header[2], header[3]]) as usize;
        let padded = record_len(len);
        let Some(rec_end) = pos.checked_add(padded) else {
            return Err("chan: tlv record length overflow");
        };
        let Some(record) = area.get(pos..rec_end) else {
            return Err("chan: tlv record value truncated");
        };
        let value = &record[TLV_HEADER_LEN..TLV_HEADER_LEN + len];
        match bit {
            EXT_DEADLINE => {
                let bytes: [u8; LEN_DEADLINE] = value
                    .try_into()
                    .map_err(|_| "chan: tlv record value truncated")?;
                exts.deadline = Some(u64::from_le_bytes(bytes));
            }
            EXT_CAP_TOKEN => {
                let bytes: &[u8; LEN_CAP_TOKEN] = value
                    .try_into()
                    .map_err(|_| "chan: tlv record value truncated")?;
                exts.cap_token = Some(bytes);
            }
            EXT_QID_VOLATILITY => {
                if value.len() != LEN_QID_VOLATILITY {
                    return Err("chan: tlv record value truncated");
                }
                exts.qid_volatility = Some(QidVolatility {
                    class: value[0],
                    lease_gen: u64::from_le_bytes(
                        value[1..9].try_into().expect("the lease generation is 8 bytes"),
                    ),
                    granted_mode: value[9],
                });
            }
            EXT_STREAM_PRIO => {
                let bytes: [u8; LEN_STREAM_PRIO] = value
                    .try_into()
                    .map_err(|_| "chan: tlv record value truncated")?;
                exts.stream_prio = Some(u32::from_le_bytes(bytes));
            }
            EXT_OBINARY => exts.obinary = true,
            EXT_STATELESS_CAP => {
                let bytes: &[u8; LEN_STATELESS_CAP] = value
                    .try_into()
                    .map_err(|_| "chan: tlv record value truncated")?;
                exts.stateless_cap = Some(bytes);
            }
            _ => return Err("chan: tlv unknown extension bit"),
        }
        pos = rec_end;
    }
    Ok(exts)
}

/// Append the encoded records for `exts` to `out`, in ascending type order.
///
/// Each record is padded to a multiple of 8 bytes, so appending to an 8-aligned
/// area keeps every record aligned from the start of the area.
pub fn encode(exts: &Exts<'_>, out: &mut Vec<u8>) {
    if let Some(value) = exts.deadline {
        push_record(out, EXT_DEADLINE.trailing_zeros() as u8, &value.to_le_bytes());
    }
    if let Some(value) = exts.cap_token {
        push_record(out, EXT_CAP_TOKEN.trailing_zeros() as u8, value);
    }
    if let Some(value) = exts.qid_volatility {
        let mut bytes = [0u8; LEN_QID_VOLATILITY];
        bytes[0] = value.class;
        bytes[1..9].copy_from_slice(&value.lease_gen.to_le_bytes());
        bytes[9] = value.granted_mode;
        push_record(out, EXT_QID_VOLATILITY.trailing_zeros() as u8, &bytes);
    }
    if let Some(value) = exts.stream_prio {
        push_record(out, EXT_STREAM_PRIO.trailing_zeros() as u8, &value.to_le_bytes());
    }
    if exts.obinary {
        push_record(out, EXT_OBINARY.trailing_zeros() as u8, &[]);
    }
    if let Some(value) = exts.stateless_cap {
        push_record(out, EXT_STATELESS_CAP.trailing_zeros() as u8, value);
    }
}

/// Append one record: `type[1] reserved[1] len[2] value padding`.
fn push_record(out: &mut Vec<u8>, type_index: u8, value: &[u8]) {
    let start = out.len();
    out.push(type_index);
    out.push(0);
    out.extend_from_slice(&(value.len() as u16).to_le_bytes());
    out.extend_from_slice(value);
    out.resize(start + record_len(value.len()), 0);
}

/// Split the `EXT_STREAM_PRIO` value into its 2-bit tier and 30-bit credit.
pub fn stream_prio_parts(value: u32) -> (u8, u32) {
    ((value >> 30) as u8, value & 0x3fff_ffff)
}
