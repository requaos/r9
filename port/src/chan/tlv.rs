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
//! bytes and reads fields with `from_le_bytes` — no alignment cast, no
//! `unsafe`, no allocation.
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
        0
    }

    /// The total padded length of this value's records.
    pub fn encoded_len(&self) -> usize {
        0
    }
}

/// Parse the `ExtTLVs` area named by `mask` (WIRE-02, P1).
pub fn parse(_mask: u16, _area: &[u8]) -> crate::Result<Exts<'_>> {
    Ok(Exts::default())
}

/// Append the encoded records for `exts` to `out`, in ascending type order.
pub fn encode(_exts: &Exts<'_>, _out: &mut alloc::vec::Vec<u8>) {}

/// Split the `EXT_STREAM_PRIO` value into its 2-bit tier and 30-bit credit.
pub fn stream_prio_parts(_value: u32) -> (u8, u32) {
    (0, 0)
}
