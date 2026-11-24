//! tpht — the succinct concurrent hash table: `FlattenedTphtShard`,
//! `ChainedTphtOverflow`, Feistel quotienting and `deref_tiny` (DS-01 to
//! DS-04).
//!
//! This is the one `ds/` module that may hold `unsafe` (Phase 3 CONTEXT
//! D-09); plan 03-10 audits every site.  The compendium's appendix listings
//! 9.3 and 9.4 are non-normative sketches and are not copied here: not their
//! memory orderings (which Miri rejects) and not a plain, non-atomic header
//! field read by a concurrent reader (CONTEXT D-03; 03-RESEARCH F7).  Every
//! field a reader can touch is an atomic, and the seqlock uses the Boehm
//! ordering (D-13).
//!
//! `feistel` is a safe submodule and carries its own
//! `#![forbid(unsafe_code)]`; `flattened` holds the 64-byte bucket header and
//! the shard that spills into the dereference banks; `chained` holds the
//! banks (plan 03-07); `deref_tiny` landed in plan 03-13.
//!
//! Every test under this module takes its sizes through [`miri_scaled`] so
//! the same test body runs at full size natively and at a small size under
//! Miri; no test is gated out of Miri (D-13).

pub mod chained;
pub mod feistel;
pub mod flattened;
#[cfg(test)]
mod measure;

/// The number of home slots in a bucket header: `occupied_mask` uses bits
/// `0..15` and a home tiny pointer is `1..=16`.
pub const HOME_SLOTS: usize = 16;

/// How many of a block's 16 home slots fill before further keys of that block
/// spill into the dereference banks (`ChainedTphtOverflow`).
///
/// At this commit the value is `4`, the only number the sources give (guide
/// section 5.3, "4-slot capacity"); it is **not yet measured**.  Task 2 of
/// plan 03-08 replaces it with the value chosen by the pre-committed rule from
/// the host measurement (D-15), and its doc comment then carries that result.
/// The constant must lie in `1..=HOME_SLOTS` and is not part of the IG-D09
/// lock (D-17): a change is a new measurement.
pub const SPILL_THRESHOLD: usize = 4;

/// A test seam for counting memory accesses (DS-03, plan 03-13): production
/// uses [`NoProbe`]'s zero-sized no-ops; a test probe counts header and arena
/// reads so `deref_tiny` can be shown to perform exactly one arena read.
pub trait Probe {
    /// Called once per reader attempt, before the sequence load.
    fn header_read(&self) {}

    /// Called once per arena slot read.
    fn arena_read(&self) {}
}

/// The zero-sized production [`Probe`]: every call compiles to nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProbe;

impl Probe for NoProbe {}

/// Errors a TPHT operation can return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TphtError {
    /// The block count is zero, not a power of two, or above `1 << 32`.
    BadBlockCount,
    /// An entry for this full key is already present.
    Exists,
    /// Returned only by a home-only shard (`with_banks(n, 0)`) whose block has
    /// all 16 header slots in use: it has nowhere to spill.  A shard with
    /// banks never returns this; a spilling insert whose candidate banks are
    /// full returns [`TphtError::BanksFull`] instead.
    HomeFull,
    /// The dereference banks have no free node (plan 03-07).
    BanksFull,
}

/// An 8-bit displacement into the flat value arena: `1..=255`, where 0 means
/// empty and is therefore not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TinyPtr(core::num::NonZeroU8);

impl TinyPtr {
    /// Build a tiny pointer from a raw byte; 0 is rejected.
    pub fn new(value: u8) -> Option<TinyPtr> {
        core::num::NonZeroU8::new(value).map(TinyPtr)
    }

    /// The raw byte (`1..=255`).
    pub fn get(self) -> u8 {
        self.0.get()
    }
}

/// `log2(num_blocks)` for a power of two from 1 to `1 << 32`; anything else
/// is [`TphtError::BadBlockCount`].
pub fn index_bits(num_blocks: usize) -> Result<u32, TphtError> {
    if num_blocks == 0 || !num_blocks.is_power_of_two() {
        return Err(TphtError::BadBlockCount);
    }
    let bits = num_blocks.trailing_zeros();
    if bits > 32 {
        return Err(TphtError::BadBlockCount);
    }
    Ok(bits)
}

/// The one sizing rule for every `tpht` test: return `under_miri` when the
/// test binary is running under Miri and `native` otherwise.  One test body,
/// two sizes; a size-dependent numeric bound is scaled with the size the test
/// ran at.  Nothing is gated out of Miri: there is no ignored test and no
/// compile-time exclusion from the Miri run (D-13).
#[cfg(test)]
pub(crate) fn miri_scaled(native: usize, under_miri: usize) -> usize {
    if cfg!(miri) { under_miri } else { native }
}
