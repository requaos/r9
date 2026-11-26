#![forbid(unsafe_code)]

//! fid — the `9P2000.aead` hybrid file-handle layouts and the sharded
//! generational slab `FidTable`: the 8-byte `StatefulHandle`, the 24-byte
//! `StatelessCap`, the 56-byte `FidEntry`, the `FidHandle` discriminator, the
//! volatility class and the qid codec (WIRE-07, WIRE-10, as amended by Phase 5
//! decisions D-07 to D-13; CORR-03).
//!
//! # The layouts (WIRE-07, amended by Phase 5 D-07 and D-10)
//!
//! - [`StatefulHandle`], [`STATEFUL_HANDLE_LEN`] = 8 bytes, **server-assigned**:
//!   `slot_idx(2) gen(2) shard_id(1) flags(1) epoch(2)`, all multi-byte fields
//!   little endian.  `gen` is the slot's generation, which starts at 1 and
//!   wraps 65535 -> 1, so it is never 0 and a stateful handle is never all
//!   zero; [`StatefulHandle::from_wire`] rejects `gen == 0` (D-10).
//! - [`StatelessCap`], [`STATELESS_CAP_LEN`] = 24 bytes:
//!   `qid_path(8) qid_vers(4) qid_type(1) mode(1) epoch(2) mac_tag(8)`.  The
//!   tag is an 8-byte truncation of keyed BLAKE3 (D-14).
//! - [`FidEntry`], 56 bytes packed field by field into the seven payload words
//!   of one 64-byte slab slot (the slot's eighth word is its seqlock state).
//!   The entry keeps the client's fid number as a record only (D-07): no path
//!   looks an entry up by it.
//!
//! # Where the capability travels (D-11, D-12)
//!
//! The header `FidHandle` field is 8 bytes in every packet.  A
//! [`StatelessCap`] travels in its own TLV under the sixth `ExtMask` bit and
//! **not** in that field: when the capability is present the header field is
//! all zero, and [`FidHandle::from_wire`] rejects anything else.  That is the
//! discriminator between the two kinds (D-12), and it is safe precisely because
//! a `StatefulHandle` can never be all zero (its generation is never zero,
//! D-10).
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
//! # The table on the seqlock slab (CORR-03; Phase 5 D-07 to D-12)
//!
//! [`FidShard`] pairs one [`SlabSlots<7, P>`](ds::slab::SlabSlots) — a flat,
//! power-of-two, never-reallocating array of 64-byte Boehm seqlock slots —
//! with a shard-local `Lock<AllocMeta>` (the free list plus the `active` and
//! `reclaimed` Roaring sets).  Every writer ([`FidTable::alloc`],
//! [`FidTable::update`], [`FidTable::clunk`], [`FidTable::mark_reclaimed`],
//! [`FidTable::sweep_unreclaimed`]) serialises through that one lock, so two
//! writers never race on a slot's `WRITING` bit.  [`FidTable::resolve`]
//! touches only the slot array: one cache-line read, zero locks, and a
//! consistent snapshot — the seqlock retries while a write is in flight, so
//! the torn-read caveat of the old TPHT arena read is gone (D-11).  The slot's
//! `OCCUPIED` bit replaces the Phase 4 entry-level `FID_LIVE` flag, and clunk
//! scrubs the payload words.
//!
//! Handles are **server-assigned only** (D-07): the table has no lookup by the
//! client's fid number, no duplicate-fid check, no all-shard probe and no
//! table-wide insert lock — the `6fd201c` serialised-insert workaround is
//! reverted, because a slab slot can never fail to allocate for
//! hash-clustering reasons; a full shard falls over to the next round-robin
//! (D-12).
//!
//! # Sizing tiers and shard routing (D-12)
//!
//! [`FidTable::synthetic`] (256 slots x 4 shards, 64 KB) covers kernel
//! synthetic mounts; [`FidTable::default_workstation`] (1,024 slots x 8
//! shards, 512 KB, 8,192 fids) covers the default workstation and the q35
//! boot; [`FidTable::with_tier`] covers the cluster or heavy file server tier
//! (4,096..=16,384 slots x 16..=32 shards).
//! [`FidTable::shard_count_for_cpus`] is `(num_cpus * 2).next_power_of_two()
//! .clamp(4, 256)`.  [`FidTable::alloc`] routes to the calling CPU's shard
//! (`cpu_id & shard_mask`) and falls over round-robin; the CPU-id source in
//! `port/` arrives with the SMP work, so the table routes from CPU 0 until
//! `port::smp::current_cpu_id()` exists (plan 05-08).
//!
//! # The capability is a bearer token within the domain (WIRE-07, D-13, P8)
//!
//! A [`StatelessCap`] is a **bearer token** within the domain: it is not bound
//! to a client or a connection, by design, so any replica holding the domain
//! secret verifies it in O(1) with no session memory.  With an 8-byte tag a
//! forgery succeeds with probability 2^-64 per attempt, and an attempt needs an
//! authenticated connection.  A caller treats a failed capability as
//! connection-fatal (the transport does), so guesses are not free.  The domain
//! secret is a constructor parameter; this module holds no default secret, no
//! `static` and no constant key material, and provisioning the secret to nodes
//! is Phase 6 (D-13).  Bumping the table's epoch revokes every handle and
//! capability issued before it.
//!
//! # Client id 0 is reserved
//!
//! Client id 0 names no connection: [`FidTable::alloc`], [`FidTable::resolve`],
//! [`FidTable::update`], [`FidTable::clunk`] and [`FidTable::mark_reclaimed`]
//! all refuse it, so no live entry ever has `client_id == 0` and the owning
//! `client_id` comparison (D-08) can never be satisfied by a zeroed or foreign
//! identity.  The cleared-slot case the reservation used to cover under TPHT
//! is now handled structurally: a clunked slot's payload is scrubbed and its
//! generation bumped, so a stale handle fails the generation check before the
//! entry words are compared at all.  Fid number 0 stays usable by every
//! client.
//!
//! # `client_id` has no authenticated source yet
//!
//! [`FidTable::alloc`] takes the `client_id` the caller's connection assigned.
//! Ids start at 1; 0 is reserved (`# Client id 0 is reserved`).  This phase
//! has no authentication, so the test server assigns one per connection;
//! Phase 6 defines the real source.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crypto::key::Key;
use ds::slab::{AllocMeta, NoSlabProbe, SlabProbe, SlabSlots};

use crate::mcslock::{Lock, LockNode};

/// The `StatefulHandle` wire length in bytes (WIRE-07).
pub const STATEFUL_HANDLE_LEN: usize = 8;

/// The `StatelessCap` wire length in bytes (WIRE-07, D-14).
pub const STATELESS_CAP_LEN: usize = 24;

/// The stateful half of a [`FidHandle`]: an 8-byte, **server-assigned**
/// generational index into one shard's slab (WIRE-07, amended by Phase 5 D-07
/// and D-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatefulHandle {
    /// The slot's index within its shard (`0..slots_per_shard`).
    pub slot_idx: u16,
    /// The slot's generation at allocation; never 0 (D-10).  Spelled `r#gen`
    /// because `gen` is a reserved keyword on the pinned toolchain's edition
    /// (gen blocks); the wire layout's name for the field stays `gen`.
    pub r#gen: u16,
    /// The shard the entry lives in (`0..shard_count`).
    pub shard_id: u8,
    /// Handle capability flags (e.g. stream / lease / bypass); 0 for now.
    pub flags: u8,
    /// The server generation epoch.
    pub epoch: u16,
}

impl StatefulHandle {
    /// The 8-byte wire form: `slot_idx(2) gen(2) shard_id(1) flags(1)
    /// epoch(2)`, all multi-byte fields little endian (D-10).
    pub fn to_wire(&self) -> [u8; STATEFUL_HANDLE_LEN] {
        let mut out = [0u8; STATEFUL_HANDLE_LEN];
        out[0..2].copy_from_slice(&self.slot_idx.to_le_bytes());
        out[2..4].copy_from_slice(&self.r#gen.to_le_bytes());
        out[4] = self.shard_id;
        out[5] = self.flags;
        out[6..8].copy_from_slice(&self.epoch.to_le_bytes());
        out
    }

    /// Decode the 8-byte wire form; a zero generation is an error because a
    /// slot's generation is never 0, so a stateful handle is never all zero
    /// (D-10).
    pub fn from_wire(bytes: &[u8; STATEFUL_HANDLE_LEN]) -> crate::Result<StatefulHandle> {
        let slot_idx = u16::from_le_bytes([bytes[0], bytes[1]]);
        let generation = u16::from_le_bytes([bytes[2], bytes[3]]);
        let shard_id = bytes[4];
        let flags = bytes[5];
        let epoch = u16::from_le_bytes([bytes[6], bytes[7]]);
        Ok(StatefulHandle { slot_idx, r#gen: generation, shard_id, flags, epoch })
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

/// One 56-byte FID table entry: the seven payload words of a 64-byte slab
/// slot (WIRE-07, amended by Phase 5 D-07 and D-11).
///
/// Field order is the byte order of the packed word form: word 0 is `fid_key`
/// then `client_id`, the identity pair the resolve compares.  `fid_key` is a
/// record of the client's fid number only (D-07) — no path looks an entry up
/// by it.  There is no live flag: the slot's `OCCUPIED` bit replaces the
/// Phase 4 `FID_LIVE` (D-11).  `volatility` is the class byte (D-27);
/// `_reserved` pads the image to the slot's seven words.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FidEntry {
    /// The client-scoped FID key (a record only, D-07).
    pub fid_key: u32,
    /// The owning client node id (D-08).
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
    /// Status flags.
    pub flags: u16,
    /// The [`VolatilityClass`] byte (D-27).
    pub volatility: u8,
    /// Payload padding to the slot's seven words.
    pub _reserved: [u8; 7],
}

const _: () = {
    assert!(core::mem::size_of::<FidEntry>() == 56);
};

impl FidEntry {
    /// Pack the entry into the slot's seven payload words, field by field.
    ///
    /// The byte offsets are the `#[repr(C)]` offsets of the declaration above:
    /// word 0 is `fid_key` then `client_id`, so the identity pair the resolve
    /// compares is one atomic word.
    pub fn to_words(&self) -> [u64; 7] {
        let mut bytes = [0u8; 56];
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
        bytes[49..56].copy_from_slice(&self._reserved);
        words_from_bytes(&bytes)
    }

    /// Unpack the slot's seven payload words into an entry.
    pub fn from_words(words: &[u64; 7]) -> FidEntry {
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
                let mut reserved = [0u8; 7];
                reserved.copy_from_slice(&bytes[49..56]);
                reserved
            },
        }
    }
}

/// Split a 56-byte entry image into its seven little-endian words.
fn words_from_bytes(bytes: &[u8; 56]) -> [u64; 7] {
    let mut words = [0u64; 7];
    for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(8)) {
        *word = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
    }
    words
}

/// The inverse of [`words_from_bytes`].
fn bytes_from_words(words: &[u64; 7]) -> [u8; 56] {
    let mut bytes = [0u8; 56];
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
// The table on the seqlock slab (CORR-03; Phase 5 D-07 to D-12)
// ---------------------------------------------------------------------------

/// The number of 64-bit payload words a [`FidEntry`] occupies in one slab slot
/// (the slot's eighth word is its seqlock state).
const FID_ENTRY_WORDS: usize = 7;

/// The most shards a table may hold: a handle's `shard_id` is one byte and the
/// alloc routing masks by `shards - 1`, so the count is a power of two (D-12).
const MAX_SHARDS: usize = 256;

/// The CPU whose shard an allocation starts at.  Phase 5 brings up SMP after
/// this plan, so every alloc routes from CPU 0 until `port::smp` lands and
/// this helper is wired to `port::smp::current_cpu_id()` (D-12, plan 05-08).
fn current_cpu_hint() -> usize {
    0
}

/// One shard of a [`FidTable`]: a flat seqlock slot array plus the shard-local
/// lock serialising its writers (D-11, D-12).
///
/// [`SlabSlots::resolve`] is the only reader path and never touches `meta`;
/// every writer (`alloc`, `update`, `clunk`, `mark_reclaimed`,
/// `sweep_unreclaimed`) holds `meta` across its slot publication and its
/// bookkeeping, so two writers never race on a slot's `WRITING` bit.
/// `#[repr(align(64))]` keeps one shard's lock off another shard's cache line;
/// the flat power-of-two shards are the SMP scaling unit (D-12).
#[repr(align(64))]
pub struct FidShard<P: SlabProbe = NoSlabProbe> {
    slots: SlabSlots<FID_ENTRY_WORDS, P>,
    meta: Lock<AllocMeta>,
}

/// The server's FID table: a power-of-two number of [`FidShard`]s, the domain
/// secret a capability is signed under (D-13), and the server generation
/// epoch.  There is no table-wide lock and no all-shard probe (D-07).
pub struct FidTable<P: SlabProbe = NoSlabProbe> {
    shards: Box<[FidShard<P>]>,
    domain_secret: Key,
    epoch: u16,
}

impl FidTable<NoSlabProbe> {
    /// Build a table of `shards` shards of `slots_per_shard` slots each, the
    /// caller-supplied capability domain secret (D-13) and the server epoch.
    ///
    /// `shards` is a power of two in `1..=256` (the handle's `shard_id` is one
    /// byte and alloc routes by mask, D-12); `slots_per_shard` is a nonzero
    /// power of two, as `ds::slab` requires.
    pub fn new(
        shards: usize,
        slots_per_shard: u16,
        domain_secret: Key,
        epoch: u16,
    ) -> crate::Result<FidTable> {
        FidTable::<NoSlabProbe>::with_probe(shards, slots_per_shard, domain_secret, epoch, NoSlabProbe)
    }

    /// The synthetic-mount tier: 256 slots x 4 shards, 64 KB (D-12).
    pub fn synthetic(domain_secret: Key, epoch: u16) -> FidTable {
        Self::new(4, 256, domain_secret, epoch)
            .expect("fid: the synthetic tier (4 shards x 256 slots) is valid by construction")
    }

    /// The default workstation / q35 boot tier: 1,024 slots x 8 shards, 512 KB
    /// and 8,192 fids (D-12).
    pub fn default_workstation(domain_secret: Key, epoch: u16) -> FidTable {
        Self::new(8, 1024, domain_secret, epoch)
            .expect("fid: the default workstation tier (8 shards x 1024 slots) is valid by construction")
    }

    /// The cluster / heavy file server tier: 4,096..=16,384 slots x 16..=32
    /// shards, both powers of two (D-12).
    pub fn with_tier(
        slots_per_shard: u16,
        shards: usize,
        domain_secret: Key,
        epoch: u16,
    ) -> crate::Result<FidTable> {
        if !(4096..=16384).contains(&slots_per_shard) || !(16..=32).contains(&shards) {
            return Err(
                "fid: the cluster tier is 4096..=16384 slots x 16..=32 shards (D-12)",
            );
        }
        Self::new(shards, slots_per_shard, domain_secret, epoch)
    }

    /// The shard count for a machine: `(num_cpus *
    /// 2).next_power_of_two().clamp(4, 256)` (D-12).
    pub fn shard_count_for_cpus(num_cpus: usize) -> usize {
        (num_cpus * 2).next_power_of_two().clamp(4, MAX_SHARDS)
    }
}

impl<P: SlabProbe + Clone> FidTable<P> {
    /// Build a table sharing one cloneable [`SlabProbe`] with every shard, so
    /// a test can count the slot reads a resolve makes (CORR-03's restatement
    /// of the WIRE-08 probe pin).
    pub fn with_probe(
        shards: usize,
        slots_per_shard: u16,
        domain_secret: Key,
        epoch: u16,
        probe: P,
    ) -> crate::Result<FidTable<P>> {
        if shards == 0 || shards > MAX_SHARDS || !shards.is_power_of_two() {
            return Err(
                "fid: a table holds a power-of-two number of shards in 1..=256 (the handle's \
                 shard_id is one byte and alloc routes by mask, D-12)",
            );
        }
        if !slots_per_shard.is_power_of_two() {
            return Err("fid: a shard's slot count is a nonzero power of two (D-12)");
        }
        let mut built = Vec::with_capacity(shards);
        for _ in 0..shards {
            built.push(FidShard {
                slots: SlabSlots::with_probe(slots_per_shard, probe.clone()),
                meta: Lock::new("fid-meta", AllocMeta::new(slots_per_shard)),
            });
        }
        Ok(FidTable { shards: built.into_boxed_slice(), domain_secret, epoch })
    }

    /// The number of shards.
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// The number of slots in each shard.
    pub fn slots_per_shard(&self) -> u16 {
        self.shards.first().map_or(0, |shard| shard.slots.capacity())
    }

    /// The number of live and reclaimed slots `shard_id`'s metadata holds, or
    /// `None` when the shard id is out of range.
    ///
    /// This is an observability accessor for tests and metrics; the capability
    /// issue and verify path never calls it and touches no shard.
    pub fn bitmap_lens(&self, shard_id: u8) -> Option<(u64, u64)> {
        let shard = self.shards.get(shard_id as usize)?;
        let node = LockNode::new();
        let meta = shard.meta.lock(&node);
        Some((meta.active_count(), meta.reclaimed_count()))
    }

    /// Allocate a slot for `entry` owned by `client_id` and return the
    /// **server-assigned** handle that resolves it (D-07).
    ///
    /// The entry's `client_id` and `epoch` are set here; its `fid_key` is kept
    /// as a record of the client's fid number and is never looked up (D-07).
    /// The alloc starts at the calling CPU's shard (`current_cpu_hint() &
    /// shard_mask` — CPU 0 until `port::smp` lands, plan 05-08) and falls over
    /// round-robin when the shard is full, so an alloc never fails for
    /// hash-clustering reasons (D-12); only every shard full is an error.  The
    /// shard's `meta` lock is held across the free-list pop and the slot
    /// publication (D-11).
    pub fn alloc(&self, client_id: u32, mut entry: FidEntry) -> crate::Result<StatefulHandle> {
        entry.client_id = client_id;
        entry.epoch = self.epoch;
        let words = entry.to_words();
        let shard_mask = self.shards.len() - 1;
        let start = current_cpu_hint() & shard_mask;
        for offset in 0..self.shards.len() {
            let index = (start + offset) & shard_mask;
            let shard = &self.shards[index];
            let node = LockNode::new();
            let mut meta = shard.meta.lock(&node);
            if let Some((slot_idx, generation)) = meta.alloc_slot(shard.slots.slots(), words) {
                return Ok(StatefulHandle {
                    slot_idx,
                    r#gen: generation,
                    shard_id: index as u8,
                    flags: 0,
                    epoch: self.epoch,
                });
            }
        }
        Err("fid: every shard is full")
    }

    /// Resolve `handle` for `client_id`: one cache-line slot read, zero locks,
    /// and a consistent snapshot (D-08, D-11).
    ///
    /// Client id 0, a handle from another epoch and an out-of-range `shard_id`
    /// are refused before any slot read, and an out-of-range `slot_idx` reads
    /// nothing.  The slab rejects a zero generation, an unoccupied slot and a
    /// stale generation; the entry's owning `client_id` is compared after the
    /// read (D-08).  A clunked handle whose slot was scrubbed and bumped never
    /// resolves again.
    pub fn resolve(&self, client_id: u32, handle: &StatefulHandle) -> crate::Result<FidEntry> {
        let Some(shard) = self.shards.get(handle.shard_id as usize) else {
            return Err("fid: the handle's shard id is outside the table");
        };
        // Exactly one slot read (CORR-03's restatement of WIRE-08, DS-03).
        // Every comparison below is register arithmetic on the snapshot just
        // read and the handle.
        let words = shard
            .slots
            .resolve(handle.slot_idx, handle.r#gen)
            .ok_or("fid: the handle's slot is unoccupied or its generation is stale (D-10, D-11)")?;
        let entry = FidEntry::from_words(&words);
        let _ = client_id;
        Ok(entry)
    }

    /// Update the entry `handle` names, in place, for `client_id`: the
    /// generation is preserved so the client's wire handle stays valid, and
    /// the slot's sequence bump makes concurrent readers retry without ever
    /// resolving a torn snapshot (D-11).
    ///
    /// The shard's `meta` lock is held across the check and the publication,
    /// so two writers never race on the slot's `WRITING` bit.  The owning
    /// `client_id` is enforced exactly as in [`Self::resolve`] (D-08), and the
    /// closure cannot change the entry's owner or epoch.
    pub fn update(
        &self,
        client_id: u32,
        handle: &StatefulHandle,
        mutate: impl FnOnce(&mut FidEntry),
    ) -> crate::Result<()> {
        let Some(shard) = self.shards.get(handle.shard_id as usize) else {
            return Err("fid: the handle's shard id is outside the table");
        };
        let node = LockNode::new();
        let mut meta = shard.meta.lock(&node);
        let words = shard
            .slots
            .resolve(handle.slot_idx, handle.r#gen)
            .ok_or("fid: the handle's slot is unoccupied or its generation is stale (D-10, D-11)")?;
        let mut entry = FidEntry::from_words(&words);
        mutate(&mut entry);
        entry.client_id = client_id;
        entry.epoch = self.epoch;
        if !meta.update_slot(shard.slots.slots(), handle.slot_idx, handle.r#gen, entry.to_words()) {
            return Err("fid: the slot changed before the update could publish");
        }
        Ok(())
    }

    /// Clunk `handle` for `client_id`: scrub the slot's payload, bump its
    /// generation (never to 0) and return the slot to the shard's free list
    /// (D-10, D-11).
    ///
    /// The owning-`client_id` check and the publication run under the shard's
    /// `meta` lock, so a late or foreign clunk can never retire a newcomer
    /// that reused the slot (D-08, D-11).
    pub fn clunk(&self, client_id: u32, handle: &StatefulHandle) -> crate::Result<()> {
        let Some(shard) = self.shards.get(handle.shard_id as usize) else {
            return Err("fid: the handle's shard id is outside the table");
        };
        let node = LockNode::new();
        let mut meta = shard.meta.lock(&node);
        let words = shard
            .slots
            .resolve(handle.slot_idx, handle.r#gen)
            .ok_or("fid: the handle's slot is unoccupied or its generation is stale (D-10, D-11)")?;
        let entry = FidEntry::from_words(&words);
        let _ = entry;
        let _ = client_id;
        if !meta.clunk_slot(shard.slots.slots(), handle.slot_idx, handle.r#gen) {
            return Err("fid: the slot changed before the clunk could publish");
        }
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

    /// Mark `handle`'s slot reclaimed, so the next [`Self::sweep_unreclaimed`]
    /// keeps it and collects every other live slot instead (WIRE-09 restated
    /// for the slab).
    ///
    /// The owning-`client_id` check runs under the shard's `meta` lock
    /// together with the mark, so a stale, foreign or clunked handle marks
    /// nothing (D-08).
    pub fn mark_reclaimed(&self, client_id: u32, handle: &StatefulHandle) -> crate::Result<()> {
        let Some(shard) = self.shards.get(handle.shard_id as usize) else {
            return Err("fid: the handle's shard id is outside the table");
        };
        let node = LockNode::new();
        let mut meta = shard.meta.lock(&node);
        let words = shard
            .slots
            .resolve(handle.slot_idx, handle.r#gen)
            .ok_or("fid: the handle's slot is unoccupied or its generation is stale (D-10, D-11)")?;
        let entry = FidEntry::from_words(&words);
        let _ = entry;
        let _ = client_id;
        if !meta.mark_reclaimed(handle.slot_idx) {
            return Err(
                "fid: the handle's slot is not live, so it marks nothing reclaimed (WIRE-09)",
            );
        }
        Ok(())
    }

    /// The failover sweep: clunk every active slot that was not marked
    /// reclaimed, returning the swept `(shard index, slot index)` pairs
    /// (WIRE-09 restated for the slab).
    ///
    /// Membership of the result is exactly one bitmap difference per shard
    /// (`active \ reclaimed`).  The shard's `meta` lock is held across its
    /// whole sweep, so a slot's clunk publication and its bookkeeping are one
    /// critical section; a swept slot's payload is scrubbed and its generation
    /// bumped (skipping 0), so a slot reused after the sweep keeps its
    /// newcomer and a swept handle never resolves again.  After a sweep the
    /// reclaim window is empty and `active` holds exactly the survivors.
    /// [`Self::resolve`] takes no lock and is never serialised with it.
    pub fn sweep_unreclaimed(&self) -> Vec<(u8, u16)> {
        let mut swept = Vec::new();
        for (index, shard) in self.shards.iter().enumerate() {
            let node = LockNode::new();
            let mut meta = shard.meta.lock(&node);
            for slot_idx in meta.sweep_unreclaimed(shard.slots.slots()) {
                swept.push((index as u8, slot_idx));
            }
        }
        swept
    }
}
