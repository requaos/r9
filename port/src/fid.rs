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
        let mut out = [0u8; STATEFUL_HANDLE_LEN];
        out[0..4].copy_from_slice(&self.fid_key.to_le_bytes());
        out[4] = self.ptr.get();
        out[5] = self.shard_id;
        out[6..8].copy_from_slice(&self.epoch.to_le_bytes());
        out
    }

    /// Decode the 8-byte wire form; a zero `ptr` byte is an error because a
    /// stateful handle is never all zero (D-12).
    pub fn from_wire(bytes: &[u8; STATEFUL_HANDLE_LEN]) -> crate::Result<StatefulHandle> {
        let fid_key = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let Some(ptr) = TinyPtr::new(bytes[4]) else {
            return Err(
                "fid: a StatefulHandle's ptr byte is zero (D-12: a stateful handle is never all \
                 zero, so eight zero bytes are not one)",
            );
        };
        let shard_id = bytes[5];
        let epoch = u16::from_le_bytes([bytes[6], bytes[7]]);
        Ok(StatefulHandle { fid_key, ptr, shard_id, epoch })
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
        let mut out = [0u8; STATELESS_CAP_LEN];
        out[0..8].copy_from_slice(&self.qid_path.to_le_bytes());
        out[8..12].copy_from_slice(&self.qid_vers.to_le_bytes());
        out[12] = self.qid_type;
        out[13] = self.mode;
        out[14..16].copy_from_slice(&self.epoch.to_le_bytes());
        out[16..24].copy_from_slice(&self.mac_tag);
        out
    }

    /// Decode the 24-byte wire form.
    pub fn from_wire(bytes: &[u8; STATELESS_CAP_LEN]) -> StatelessCap {
        StatelessCap {
            qid_path: u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            qid_vers: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            qid_type: bytes[12],
            mode: bytes[13],
            epoch: u16::from_le_bytes([bytes[14], bytes[15]]),
            mac_tag: [bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23]],
        }
    }

    /// The first 16 wire bytes: the part the `mac_tag` covers.
    pub fn body(&self) -> [u8; 16] {
        let wire = self.to_wire();
        let mut out = [0u8; 16];
        out.copy_from_slice(&wire[0..16]);
        out
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
        header: &[u8; STATEFUL_HANDLE_LEN],
        cap: Option<&[u8; STATELESS_CAP_LEN]>,
    ) -> crate::Result<FidHandle> {
        match cap {
            Some(cap_bytes) => {
                if header.iter().any(|&byte| byte != 0) {
                    return Err(
                        "fid: a capability packet's 8-byte FidHandle field must be all zero \
                         (D-12: the zero field is what makes the capability the discriminator, \
                         and a stateful handle is never all zero)",
                    );
                }
                Ok(FidHandle::Stateless(StatelessCap::from_wire(cap_bytes)))
            }
            None => Ok(FidHandle::Stateful(StatefulHandle::from_wire(header)?)),
        }
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
    ///
    /// The byte offsets are the `#[repr(C)]` offsets of the declaration above:
    /// word 0 is `fid_key` then `client_id`, so the identity pair the resolve
    /// compares is one atomic word (RESEARCH Pitfall 4).
    pub fn to_words(&self) -> [u64; 8] {
        let mut bytes = [0u8; 64];
        bytes[0..4].copy_from_slice(&self.fid_key.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.client_id.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.qid_path.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.offset.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.qid_vers.to_le_bytes());
        bytes[28..32].copy_from_slice(&self.iounit.to_le_bytes());
        bytes[32..36].copy_from_slice(&self.lease_gen.to_le_bytes());
        bytes[36..40].copy_from_slice(&self.ring_slot.to_le_bytes());
        bytes[40..42].copy_from_slice(&self.dev_handler.to_le_bytes());
        bytes[42..44].copy_from_slice(&self.epoch.to_le_bytes());
        bytes[44] = self.qid_type;
        bytes[45] = self.open_mode;
        bytes[46..48].copy_from_slice(&self.flags.to_le_bytes());
        bytes[48] = self.volatility;
        bytes[49..64].copy_from_slice(&self._reserved);
        words_from_bytes(&bytes)
    }

    /// Unpack the eight-word arena form into an entry.
    pub fn from_words(words: &[u64; 8]) -> FidEntry {
        let bytes = bytes_from_words(words);
        FidEntry {
            fid_key: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            client_id: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            qid_path: u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
            offset: u64::from_le_bytes([
                bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                bytes[23],
            ]),
            qid_vers: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            iounit: u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            lease_gen: u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]),
            ring_slot: u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]),
            dev_handler: u16::from_le_bytes([bytes[40], bytes[41]]),
            epoch: u16::from_le_bytes([bytes[42], bytes[43]]),
            qid_type: bytes[44],
            open_mode: bytes[45],
            flags: u16::from_le_bytes([bytes[46], bytes[47]]),
            volatility: bytes[48],
            _reserved: {
                let mut reserved = [0u8; 15];
                reserved.copy_from_slice(&bytes[49..64]);
                reserved
            },
        }
    }
}

/// Split a 64-byte entry image into its eight little-endian words.
fn words_from_bytes(bytes: &[u8; 64]) -> [u64; 8] {
    let mut words = [0u64; 8];
    for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(8)) {
        *word = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
    }
    words
}

/// The inverse of [`words_from_bytes`].
fn bytes_from_words(words: &[u64; 8]) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    for (chunk, word) in bytes.chunks_exact_mut(8).zip(words.iter()) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    bytes
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
    pub fn from_byte(byte: u8) -> crate::Result<VolatilityClass> {
        match byte {
            0x20 => Ok(VolatilityClass::Immutable),
            0x40 => Ok(VolatilityClass::Leased),
            0x80 => Ok(VolatilityClass::Volatile),
            _ => Err(
                "fid: not a volatility class byte (D-27: exactly 0x20 immutable, 0x40 leased or \
                 0x80 volatile)",
            ),
        }
    }

    /// The class as its one byte.
    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

/// Encode a [`crate::dat::Qid`] into its 13-byte wire form:
/// `type[1] vers[4] path[8]` (the `convM2S` order `port::devmnt` reads).
pub fn qid_encode(qid: &crate::dat::Qid) -> [u8; 13] {
    let mut out = [0u8; 13];
    out[0] = qid.typ;
    out[1..5].copy_from_slice(&qid.vers.to_le_bytes());
    out[5..13].copy_from_slice(&qid.path.to_le_bytes());
    out
}

/// Decode the 13-byte wire form into a [`crate::dat::Qid`].
pub fn qid_decode(bytes: &[u8; 13]) -> crate::dat::Qid {
    crate::dat::Qid {
        path: u64::from_le_bytes([
            bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
        ]),
        vers: u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
        typ: bytes[0],
    }
}

/// The `#G` qid path: `((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)`
/// (WIRE-10).
pub fn gpu_qid_path(ctx_id: u32, node_type: u8) -> u64 {
    // `node_type` is already a `u8`, so `node_type & 0xFF` from the published
    // formula is an identity operation and is left off to keep clippy quiet;
    // the wide mask on `ctx_id` is real and stays.
    (((ctx_id & 0x00FF_FFFF) as u64) << 8) | node_type as u64
}
