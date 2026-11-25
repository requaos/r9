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
//! # The table on TPHT (WIRE-08, WIRE-09; D-05, D-13)
//!
//! [`FidShard`] wraps one home-only
//! `ds::tpht::flattened::FlattenedTphtShard<8, _>` and the two bitmaps that
//! name its live and reclaimed arena slots; [`FidTable`] boxes at most 256 of
//! them.  A resolve makes exactly one arena read and zero header reads
//! (`deref_tiny`), then compares the identity pair in registers.  A capability
//! is issued and verified without touching a shard or a bitmap at all, which is
//! what "zero FID memory" means.  The failover sweep is one bitmap
//! `difference` per shard.
//!
//! # The capability is a bearer token within the domain (WIRE-07, D-13, P8)
//!
//! A [`StatelessCap`] is a **bearer token** within the domain: it is not bound
//! to a client or a connection, by design, so any replica holding the domain
//! secret verifies it in O(1) with no session memory.  With an 8-byte tag a
//! forgery succeeds with probability 2^-64 per attempt, and an attempt needs an
//! authenticated connection.  A caller treats a failed capability as
//! connection-fatal (the transport does), so guesses are not free.  The domain
//! secret is a constructor parameter ([`FidTable::new`]); this module holds no
//! default secret, no `static` and no constant key material, and provisioning
//! the secret to nodes is Phase 6 (D-13).  Bumping the table's epoch revokes
//! every handle and capability issued before it.
//!
//! # Client id 0 is reserved
//!
//! Client id 0 is the identity a cleared arena slot reads as:
//! [`FidEntry::from_words`] of an all-zero image has `fid_key == 0` and
//! `client_id == 0`.  `ds` clears a removed entry's eight words one at a time,
//! so a [`FidTable::resolve`] racing a `clunk` can read word 0 already zero
//! while the epoch word and [`FID_LIVE`] are not yet cleared.  Reserving client
//! id 0 rules that image out: [`FidTable::insert`] and [`FidTable::resolve`]
//! refuse client id 0, so every live entry's identity word is non-zero whatever
//! its fid number, and [`FidEntry::matches`] rejects a zero identity word
//! outright.  Fid number 0 stays usable by every other client.  This is not a
//! consistent-snapshot claim — the read is still eight independent loads (see
//! above) — and it is narrower than "no torn read ever resolves": it says only
//! that a read whose identity word is already zero matches no client and no
//! handle.
//!
//! # `client_id` has no authenticated source yet
//!
//! [`FidTable::insert`] takes the `client_id` the caller's connection assigned.
//! Ids start at 1; 0 is reserved for the cleared arena slot (`# Client id 0 is
//! reserved`).  This phase has no authentication, so the test server assigns one
//! per connection; Phase 6 defines the real source.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crypto::key::Key;
use ds::roaring::RoaringBitmap;
use ds::tpht::flattened::FlattenedTphtShard;
use ds::tpht::{HOME_SLOTS, NoProbe, Probe, TinyPtr, TphtError};

use crate::mcslock::{Lock, LockNode};

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

    /// Whether these words are the live entry `handle` names for `client_id`
    /// at `table_epoch`: the register comparisons [`FidTable::resolve`] makes
    /// on the eight words it read.
    pub fn matches(&self, client_id: u32, handle: &StatefulHandle, table_epoch: u16) -> bool {
        // Client id 0 is the cleared-slot identity and an all-zero identity word
        // is a cleared (or torn) slot, so neither can name a live entry.  With
        // those two rules every live entry's word 0 is non-zero whatever its fid
        // number, and a read that sees word 0 already cleared matches nobody
        // (WIRE-08, WR-04).
        client_id != 0
            && (self.fid_key != 0 || self.client_id != 0)
            && self.fid_key == handle.fid_key
            && self.client_id == client_id
            && self.epoch == handle.epoch
            && handle.epoch == table_epoch
            && self.flags & FID_LIVE != 0
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

// ---------------------------------------------------------------------------
// The table on TPHT (WIRE-08, WIRE-09)
// ---------------------------------------------------------------------------

/// The number of 64-bit words a [`FidEntry`] occupies in a shard's arena.
const FID_ENTRY_WORDS: usize = 8;

/// The most shards a table may hold: a handle's `shard_id` is one byte (D-12).
const MAX_SHARDS: usize = 256;

/// The TPHT key for one client's fid: `client_id` in the high half and
/// `fid_key` in the low half, so two clients' identical fid numbers never
/// collide (RESEARCH Pattern 7, A10).
fn tpht_key(client_id: u32, fid_key: u32) -> u64 {
    ((client_id as u64) << 32) | fid_key as u64
}

/// A 64-bit avalanche mix of the TPHT key, used only to choose a key's home
/// shard.  It is the SplitMix64 finalizer: deterministic, cheap and spread
/// across every shard count.
fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One shard of a [`FidTable`]: a home-only TPHT shard and the two bitmaps that
/// name its live and reclaimed arena slots (WIRE-08, WIRE-09).
///
/// The TPHT shard is built with zero banks, so every entry has a
/// [`TinyPtr`] and can become a [`StatefulHandle`]; a full home block reports
/// `TphtError::HomeFull` and the table falls over to the next shard
/// (RESEARCH Pitfall 3).  The bitmaps hold dense arena slot ids from
/// `FlattenedTphtShard::slot_id`.
pub struct FidShard<P: Probe = NoProbe> {
    tpht: FlattenedTphtShard<FID_ENTRY_WORDS, P>,
    active: Lock<RoaringBitmap>,
    reclaimed: Lock<RoaringBitmap>,
}

/// The server's FID table: at most 256 [`FidShard`]s, the domain secret a
/// capability is signed under (D-13), and the server generation epoch.
pub struct FidTable<P: Probe = NoProbe> {
    shards: Box<[FidShard<P>]>,
    domain_secret: Key,
    epoch: u16,
}

impl FidTable<NoProbe> {
    /// Build a table with `shards` home-only shards of `blocks_per_shard`
    /// home blocks each, the caller-supplied capability domain secret (D-13)
    /// and the server epoch.
    ///
    /// `shards` is `1..=256` (the handle's `shard_id` is one byte);
    /// `blocks_per_shard` must be a power of two, as `ds` requires.
    pub fn new(
        shards: usize,
        blocks_per_shard: usize,
        domain_secret: Key,
        epoch: u16,
    ) -> crate::Result<FidTable> {
        FidTable::<NoProbe>::with_probe(shards, blocks_per_shard, domain_secret, epoch, NoProbe)
    }
}

impl<P: Probe + Clone> FidTable<P> {
    /// Build a table sharing one cloneable [`Probe`] with every shard, so a
    /// test can count the memory accesses a resolve makes (DS-03, WIRE-08).
    pub fn with_probe(
        shards: usize,
        blocks_per_shard: usize,
        domain_secret: Key,
        epoch: u16,
        probe: P,
    ) -> crate::Result<FidTable<P>> {
        if shards == 0 || shards > MAX_SHARDS {
            return Err(
                "fid: a table holds 1 to 256 shards (the handle's shard_id is one byte, D-12)",
            );
        }
        let mut built = Vec::with_capacity(shards);
        for _ in 0..shards {
            let tpht =
                FlattenedTphtShard::<FID_ENTRY_WORDS, P>::with_banks_and_probe(
                    blocks_per_shard,
                    0,
                    probe.clone(),
                )
                .map_err(|_| {
                    "fid: the shard's block count is zero or not a power of two of at most 1 << 32"
                })?;
            built.push(FidShard {
                tpht,
                active: Lock::new("fid-active", RoaringBitmap::new()),
                reclaimed: Lock::new("fid-reclaimed", RoaringBitmap::new()),
            });
        }
        Ok(FidTable { shards: built.into_boxed_slice(), domain_secret, epoch })
    }

    /// The shard a key's insertion starts at: a mix of the TPHT key modulo the
    /// shard count.  On a full home shard the insert walks forward from here.
    pub fn home_shard(&self, client_id: u32, fid_key: u32) -> u8 {
        (mix64(tpht_key(client_id, fid_key)) as usize % self.shards.len()) as u8
    }

    /// The number of live and reclaimed slot ids `shard_id`'s two bitmaps
    /// hold, or `None` when the shard id is out of range.
    ///
    /// This is an observability accessor for tests and metrics; the capability
    /// issue and verify path never calls it and touches no bitmap.
    pub fn bitmap_lens(&self, shard_id: u8) -> Option<(u64, u64)> {
        let shard = self.shards.get(shard_id as usize)?;
        let node = LockNode::new();
        let active = shard.active.lock(&node).len();
        let node = LockNode::new();
        let reclaimed = shard.reclaimed.lock(&node).len();
        Some((active, reclaimed))
    }

    /// Insert `entry` for `client_id` and return the handle that resolves it
    /// (WIRE-08).
    ///
    /// The entry's `client_id`, `epoch` and [`FID_LIVE`] flag are set here; the
    /// key is `(client_id, fid_key)`.  Client id 0 is reserved for the cleared
    /// arena slot and is refused before any shard is touched (WR-04).  The
    /// insert starts at the key's home shard ([`Self::home_shard`]) and walks
    /// forward on `HomeFull`, so a key whose home block is full lands in the
    /// next shard and its handle records that shard's index.  Every shard full
    /// is an error.
    pub fn insert(&self, client_id: u32, mut entry: FidEntry) -> crate::Result<StatefulHandle> {
        if client_id == 0 {
            return Err(
                "fid: client id 0 is reserved for the cleared arena slot, so it cannot own an \
                 entry (WIRE-08, WR-04)",
            );
        }
        entry.client_id = client_id;
        entry.epoch = self.epoch;
        entry.flags |= FID_LIVE;
        let fid_key = entry.fid_key;
        let key = tpht_key(client_id, fid_key);
        let words = entry.to_words();
        let shard_count = self.shards.len();
        let start = self.home_shard(client_id, fid_key) as usize;
        for offset in 0..shard_count {
            let index = (start + offset) % shard_count;
            let shard = &self.shards[index];
            let same = |value: &[u64; FID_ENTRY_WORDS]| {
                let existing = FidEntry::from_words(value);
                existing.fid_key == fid_key && existing.client_id == client_id
            };
            match shard.tpht.insert(key, words, same) {
                Ok(Some(ptr)) => {
                    let slot = shard.tpht.slot_id(key, ptr);
                    let node = LockNode::new();
                    shard.active.lock(&node).insert(slot);
                    return Ok(StatefulHandle {
                        fid_key,
                        ptr,
                        shard_id: index as u8,
                        epoch: self.epoch,
                    });
                }
                Ok(None) => {
                    return Err(
                        "fid: a home-only shard returned a spilled pointer (it has no banks)",
                    );
                }
                Err(TphtError::HomeFull) => continue,
                Err(TphtError::Exists) => {
                    return Err("fid: an entry for this client and fid is already present");
                }
                Err(_) => return Err("fid: the shard rejected the insert"),
            }
        }
        Err("fid: every shard is full")
    }

    /// Resolve `handle` for `client_id` in one arena read and zero header
    /// reads (WIRE-08, DS-03).
    ///
    /// Client id 0 is refused, and so is an out-of-range `shard_id`, before any
    /// read (T-04-35, WR-04).  A `ptr` above [`HOME_SLOTS`] is refused too: the
    /// table's shards are home-only, so no handle this table issued has such a
    /// pointer, and `ds` folds a pointer that wide onto another slot (WR-06).
    /// The one `deref_tiny` call is the only memory access; the identity pair,
    /// epoch and live flag are then compared in registers by
    /// [`FidEntry::matches`].  A handle from another client or another epoch,
    /// and a clunked handle whose slot was zeroed, all fail here (T-04-32,
    /// T-04-33).
    pub fn resolve(&self, client_id: u32, handle: &StatefulHandle) -> crate::Result<FidEntry> {
        if client_id == 0 {
            return Err(
                "fid: client id 0 is reserved for the cleared arena slot and never resolves an \
                 entry (WIRE-08, WR-04)",
            );
        }
        let Some(shard) = self.shards.get(handle.shard_id as usize) else {
            return Err("fid: the handle's shard id is outside the table");
        };
        if handle.ptr.get() as usize > HOME_SLOTS {
            return Err(
                "fid: the handle's ptr is outside the home window this table's shards can have \
                 issued (WIRE-08, WR-06)",
            );
        }
        let key = tpht_key(client_id, handle.fid_key);
        // Exactly one arena read, zero header reads (WIRE-08, DS-03).  Every
        // comparison below is register arithmetic on the words just read and
        // the handle.
        let words = shard.tpht.deref_tiny(key, handle.ptr);
        let entry = FidEntry::from_words(&words);
        if !entry.matches(client_id, handle, self.epoch) {
            return Err(
                "fid: the handle does not name a live entry for this client and epoch \
                 (T-04-32, T-04-33)",
            );
        }
        Ok(entry)
    }

    /// Clunk `handle` for `client_id`: resolve it, remove it from its shard
    /// (which zeroes the arena slot, plan 04-09) and drop its slot id from both
    /// bitmaps (WIRE-08).
    ///
    /// The handle is resolved first, so a `ptr` outside the home window is
    /// refused there and no slot id is ever computed from an unvalidated
    /// pointer (WR-06).
    pub fn clunk(&self, client_id: u32, handle: &StatefulHandle) -> crate::Result<()> {
        let _entry = self.resolve(client_id, handle)?;
        let shard = &self.shards[handle.shard_id as usize];
        let key = tpht_key(client_id, handle.fid_key);
        let same = |value: &[u64; FID_ENTRY_WORDS]| {
            let existing = FidEntry::from_words(value);
            existing.fid_key == handle.fid_key && existing.client_id == client_id
        };
        if !shard.tpht.remove(key, same) {
            return Err("fid: the entry vanished before it could be clunked");
        }
        let slot = shard.tpht.slot_id(key, handle.ptr);
        let node = LockNode::new();
        shard.active.lock(&node).remove(slot);
        let node = LockNode::new();
        shard.reclaimed.lock(&node).remove(slot);
        Ok(())
    }

    /// Set the server epoch.  Bumping it revokes every handle and capability
    /// issued before by making their epoch check fail (RESEARCH Pattern 7).
    pub fn set_epoch(&mut self, epoch: u16) {
        self.epoch = epoch;
    }

    /// Issue a stateless capability for `qid` opened with `mode`, signed under
    /// the table's domain secret at the table's epoch (WIRE-07, P8).
    pub fn issue_cap(&self, qid: &crate::dat::Qid, mode: u8) -> StatelessCap {
        let mut cap = StatelessCap {
            qid_path: qid.path,
            qid_vers: qid.vers,
            qid_type: qid.typ,
            mode,
            epoch: self.epoch,
            mac_tag: [0u8; 8],
        };
        // The tag computation and the key bytes stay in `crypto/`: `port` never
        // sees either (T-04-03, P8).
        cap.mac_tag = crypto::session::cap_tag(&self.domain_secret, &cap.body());
        cap
    }

    /// Verify a capability with the domain secret and **zero FID memory**: no
    /// shard and no bitmap is touched (WIRE-07, WIRE-08, P8).
    ///
    /// The epoch is checked against the table's current epoch, so bumping the
    /// epoch revokes every capability issued before it.  The tag comparison is
    /// constant-time inside `crypto::session::cap_verify`.  A failure here is
    /// connection-fatal for the caller (P8): the capability is a bearer token,
    /// so a guess must not be free.
    pub fn verify_cap(&self, cap: &StatelessCap) -> crate::Result<()> {
        if cap.epoch != self.epoch {
            return Err("fid: the capability's epoch is not the table's current epoch");
        }
        crypto::session::cap_verify(&self.domain_secret, &cap.body(), &cap.mac_tag).map_err(
            |_| "fid: the capability tag does not verify under this table's domain secret",
        )
    }

    /// Mark `handle`'s arena slot reclaimed, so the next [`Self::sweep`] keeps
    /// it and collects every other live slot instead (WIRE-09).
    ///
    /// The handle is resolved first, so a stale, foreign or clunked handle, and
    /// a `ptr` outside the home window, are refused and mark nothing (WR-06).
    pub fn mark_reclaimed(&self, client_id: u32, handle: &StatefulHandle) -> crate::Result<()> {
        let _entry = self.resolve(client_id, handle)?;
        let shard = &self.shards[handle.shard_id as usize];
        let key = tpht_key(client_id, handle.fid_key);
        let slot = shard.tpht.slot_id(key, handle.ptr);
        let node = LockNode::new();
        shard.reclaimed.lock(&node).insert(slot);
        Ok(())
    }

    /// The failover sweep: remove every active slot that was not marked
    /// reclaimed, returning the swept `(shard index, slot id)` pairs (WIRE-09).
    ///
    /// Membership of the result is one bitmap `difference` per shard
    /// (`active \ reclaimed`) and nothing else; reclaimed entries keep
    /// resolving afterwards and swept entries do not.  After a sweep the
    /// reclaim window is empty and `active` holds exactly the survivors.
    pub fn sweep(&self) -> Vec<(u8, u32)> {
        let mut swept = Vec::new();
        for (index, shard) in self.shards.iter().enumerate() {
            let node = LockNode::new();
            let mut active = shard.active.lock(&node);
            let node = LockNode::new();
            let mut reclaimed = shard.reclaimed.lock(&node);

            // The one and-not WIRE-09 names.  Membership of the result is this
            // bitmap difference and nothing else: there is no per-id
            // membership test below.
            let orphans = active.difference(&reclaimed);
            for slot in orphans.iter() {
                // `read_slot` is the sweep's bounds-checked read; a corrupt
                // bitmap id can name no slot and is still reported as swept so
                // the caller can account for it.
                if let Some(words) = shard.tpht.read_slot(slot) {
                    let entry = FidEntry::from_words(&words);
                    let key = tpht_key(entry.client_id, entry.fid_key);
                    let same = |value: &[u64; FID_ENTRY_WORDS]| {
                        let candidate = FidEntry::from_words(value);
                        candidate.fid_key == entry.fid_key
                            && candidate.client_id == entry.client_id
                    };
                    shard.tpht.remove(key, same);
                }
                // Drop the swept id from the live set as it is collected, so
                // the survivors need no second bitmap operation.
                active.remove(slot);
                swept.push((index as u8, slot));
            }
            // The reclaim window resets for the next failover round.
            *reclaimed = RoaringBitmap::new();
        }
        swept
    }
}
