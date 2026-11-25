//! fid — the `9P2000.aead` hybrid file-handle layouts: the 8-byte
//! `StatefulHandle`, the 24-byte `StatelessCap`, the 64-byte `FidEntry`, the
//! `FidHandle` discriminator, the volatility class and the qid codec
//! (WIRE-07, WIRE-10; D-05, D-11 to D-14, D-22, D-27).
//!
//! # The layouts (WIRE-07, D-14)
//!
//! - [`StatefulHandle`], [`STATEFUL_HANDLE_LEN`] = 8 bytes:
//!   `fid_key(4) ptr(1) shard_id(1) epoch(2)`, all multi-byte fields little
//!   endian.  `ptr` is a [`ds::tpht::TinyPtr`], which is a `NonZeroU8`, so a
//!   stateful handle can never be all zero (D-12).
//! - [`StatelessCap`], [`STATELESS_CAP_LEN`] = 24 bytes:
//!   `qid_path(8) qid_vers(4) qid_type(1) mode(1) epoch(2) mac_tag(8)`.  The
//!   tag is an 8-byte truncation of keyed BLAKE3 (D-14).
//! - [`FidEntry`], 64 bytes with 64-byte alignment — exactly one cache line,
//!   the unit `deref_tiny` reads.
//!
//! # Where the capability travels (D-11, D-12)
//!
//! The header `FidHandle` field is 8 bytes in every packet.  A
//! [`StatelessCap`] travels in its own TLV under the sixth `ExtMask` bit and
//! **not** in that field: when the capability is present the header field is
//! all zero, and [`FidHandle::from_wire`] rejects anything else.  That is the
//! discriminator between the two kinds (D-12), and it is safe precisely because
//! a `StatefulHandle` can never be all zero.
//!
//! # The volatility class is its own byte (WIRE-10, D-27)
//!
//! [`VolatilityClass`] has exactly three values — 0x20, 0x40 and 0x80 — and it
//! is carried in [`FidEntry::volatility`] and in the `EXT_QID_VOLATILITY`
//! record, never in a `dat::Qid` type byte.  Those three numbers are Plan 9's
//! `QTEXCL`, `QTAPPEND` and `QTDIR` (Harvey `sys/include/libc.h:591-593`), so
//! writing a class into the type byte would turn a volatile file into a
//! directory or the reverse (T-04-30).  A test pins the collision.
//!
//! # A read through `deref_tiny` is not a consistent snapshot
//!
//! [`FidEntry::from_words`] rebuilds an entry from the eight words
//! `ds::tpht::FlattenedTphtShard::deref_tiny` returned.  Those eight words are
//! eight independent atomic loads with no seqlock, so a read racing an insert
//! into the same slot can mix two entries (RESEARCH Pitfall 4).  That is
//! tolerable and is not made better here: the server hands a handle out only
//! after the insert returns, the race needs a client to reuse a stale handle
//! for the same fid number, and both entries in the mix belong to that client.
//! Keeping `fid_key` and `client_id` in word 0 (as this layout does) is what
//! makes the identity check a single atomic word.  Do not claim a consistent
//! snapshot anywhere in this module.
//!
//! # What is not here yet
//!
//! The table, the shards, capability issue and verify, the sweep and the
//! single-read resolve are WIRE-08's and WIRE-09's, in later plans of this
//! phase (D-22).  This module is the layout half only.

use ds::tpht::TinyPtr;

/// The `StatefulHandle` wire length in bytes (WIRE-07).
pub const STATEFUL_HANDLE_LEN: usize = 8;

/// The `StatelessCap` wire length in bytes (WIRE-07, D-14).
pub const STATELESS_CAP_LEN: usize = 24;

/// The `FidEntry::flags` bit that marks a live entry.
///
/// A zero arena slot — a clunked entry whose words `ds` has cleared — has
/// `flags == 0`, so it never validates however the rest of the words are read
/// (T-04-29).
pub const FID_LIVE: u16 = 1;

/// The stateful half of a [`FidHandle`]: an 8-byte, client-scoped pointer into
/// one shard's TPHT arena (WIRE-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatefulHandle {
    /// The client-scoped FID key.
    pub fid_key: u32,
    /// The arena displacement; `1..=255`, never zero (D-12).
    pub ptr: TinyPtr,
    /// The shard the entry lives in (`0..=255`).
    pub shard_id: u8,
    /// The server generation epoch.
    pub epoch: u16,
}

impl StatefulHandle {
    /// The 8-byte wire form: `fid_key(4) ptr(1) shard_id(1) epoch(2)`.
    pub fn to_wire(&self) -> [u8; STATEFUL_HANDLE_LEN] {
        [0u8; STATEFUL_HANDLE_LEN]
    }

    /// Decode the 8-byte wire form; a zero `ptr` byte is an error because a
    /// stateful handle is never all zero (D-12).
    pub fn from_wire(_bytes: &[u8; STATEFUL_HANDLE_LEN]) -> crate::Result<StatefulHandle> {
        Err("fid: RED stub")
    }
}

/// The stateless half: a 24-byte capability that a replica verifies with the
/// domain secret and no FID memory (WIRE-07, D-13, D-14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatelessCap {
    /// The content-addressed qid path.
    pub qid_path: u64,
    /// The qid version.
    pub qid_vers: u32,
    /// The Plan 9 type bits, unchanged (the volatility class is separate).
    pub qid_type: u8,
    /// The open access mode.
    pub mode: u8,
    /// The server generation epoch.
    pub epoch: u16,
    /// The 8-byte truncated keyed BLAKE3 tag (D-14).
    pub mac_tag: [u8; 8],
}

impl StatelessCap {
    /// The 24-byte wire form:
    /// `qid_path(8) qid_vers(4) qid_type(1) mode(1) epoch(2) mac_tag(8)`.
    pub fn to_wire(&self) -> [u8; STATELESS_CAP_LEN] {
        [0u8; STATELESS_CAP_LEN]
    }

    /// Decode the 24-byte wire form.
    pub fn from_wire(_bytes: &[u8; STATELESS_CAP_LEN]) -> StatelessCap {
        StatelessCap { qid_path: 0, qid_vers: 0, qid_type: 0, mode: 0, epoch: 0, mac_tag: [0; 8] }
    }

    /// The first 16 wire bytes: the part the `mac_tag` covers.
    pub fn body(&self) -> [u8; 16] {
        [0u8; 16]
    }
}

/// Which half a packet's handle is (D-11, D-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidHandle {
    /// A stateful entry in a shard.
    Stateful(StatefulHandle),
    /// A capability that needs no server memory.
    Stateless(StatelessCap),
}

impl FidHandle {
    /// Rebuild the handle from the packet: the 8-byte header field always, and
    /// the capability TLV when the `EXT_STATELESS_CAP` bit is set.
    ///
    /// A capability with a non-zero header field is rejected (D-12).
    pub fn from_wire(
        _header: &[u8; STATEFUL_HANDLE_LEN],
        _cap: Option<&[u8; STATELESS_CAP_LEN]>,
    ) -> crate::Result<FidHandle> {
        Err("fid: RED stub")
    }
}

/// One 64-byte, cache-line-aligned FID table entry (WIRE-07, D-05).
///
/// Field order is the wire order of the word form: word 0 is `fid_key` then
/// `client_id`, which is what makes the identity check one atomic word
/// (RESEARCH Pitfall 4).  `volatility` is the class byte taken from the RFC's
/// reserved area (D-27); `_reserved` is the remaining padding.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FidEntry {
    /// The client-scoped FID key.
    pub fid_key: u32,
    /// The authenticated client node id.
    pub client_id: u32,
    /// The authoritative resource path.
    pub qid_path: u64,
    /// The current file cursor.
    pub offset: u64,
    /// The resource version.
    pub qid_vers: u32,
    /// The preferred I/O chunk size.
    pub iounit: u32,
    /// The associated distributed lease generation.
    pub lease_gen: u32,
    /// The zero-copy DMA bypass ring slot.
    pub ring_slot: u32,
    /// The kernel synthetic device index.
    pub dev_handler: u16,
    /// The allocation epoch.
    pub epoch: u16,
    /// The Plan 9 type bits, unchanged (the class is separate, D-27).
    pub qid_type: u8,
    /// The read/write/exec mode bits.
    pub open_mode: u8,
    /// Status flags; [`FID_LIVE`] marks a live entry.
    pub flags: u16,
    /// The [`VolatilityClass`] byte (D-27).
    pub volatility: u8,
    /// Cache-line padding.
    pub _reserved: [u8; 15],
}

const _: () = {
    assert!(core::mem::size_of::<FidEntry>() == 64);
    assert!(core::mem::align_of::<FidEntry>() == 64);
};

impl FidEntry {
    /// Pack the entry into its eight-word arena form, field by field.
    pub fn to_words(&self) -> [u64; 8] {
        [0u64; 8]
    }

    /// Unpack the eight-word arena form into an entry.
    pub fn from_words(_words: &[u64; 8]) -> FidEntry {
        FidEntry {
            fid_key: 0,
            client_id: 0,
            qid_path: 0,
            offset: 0,
            qid_vers: 0,
            iounit: 0,
            lease_gen: 0,
            ring_slot: 0,
            dev_handler: 0,
            epoch: 0,
            qid_type: 0,
            open_mode: 0,
            flags: 0,
            volatility: 0,
            _reserved: [0; 15],
        }
    }
}

/// The three volatility classes (WIRE-10, D-27).
///
/// The values collide numerically with Plan 9's `QTEXCL`, `QTAPPEND` and
/// `QTDIR`, which is exactly why the class has its own byte (T-04-30).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilityClass {
    /// `QTIMMUTABLE`.
    Immutable = 0x20,
    /// `QTLEASED`.
    Leased = 0x40,
    /// `QTVOLATILE`.
    Volatile = 0x80,
}

impl VolatilityClass {
    /// The class a byte names, or an error for any other value.
    pub fn from_byte(_byte: u8) -> crate::Result<VolatilityClass> {
        Err("fid: RED stub")
    }

    /// The class as its one byte.
    pub fn as_byte(self) -> u8 {
        0
    }
}

/// Encode a [`crate::dat::Qid`] into its 13-byte wire form:
/// `type[1] vers[4] path[8]` (the `convM2S` order `port::devmnt` reads).
pub fn qid_encode(_qid: &crate::dat::Qid) -> [u8; 13] {
    [0u8; 13]
}

/// Decode the 13-byte wire form into a [`crate::dat::Qid`].
pub fn qid_decode(_bytes: &[u8; 13]) -> crate::dat::Qid {
    crate::dat::Qid { path: 0, vers: 0, typ: 0 }
}

/// The `#G` qid path: `((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)`
/// (WIRE-10).
pub fn gpu_qid_path(_ctx_id: u32, _node_type: u8) -> u64 {
    0
}
