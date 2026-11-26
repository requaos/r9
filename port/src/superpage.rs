//! superpage — the exact 64-byte `SuperpageResidency` (MEM-08).
//!
//! One 2 MiB superpage is eight 4 KiB sub-pages and each `u64` covers 64
//! sub-pages, so `[AtomicU64; 8]` holds 512 residency bits in exactly one
//! 64-byte cache line.  Demotion resets the whole word array atomically per
//! word with `Release` stores; observers use `Acquire` loads, so a demoted
//! superpage is never observed half-populated.
//!
//! Bit `i` lives in word `i / 64`, bit `i % 64`; indices `0`, `63`, `64`
//! and `511` cross word boundaries without shifting overflow, and an index
//! at or past 512 is rejected without touching memory.

use core::mem::{align_of, size_of};
use core::sync::atomic::{AtomicU64, Ordering};

/// Sub-pages in one 2 MiB superpage.
pub const SUBPAGES: usize = 512;
/// Words in the residency bitmap (one 64-byte cache line).
pub const WORDS: usize = 8;

/// Residency state of the 512 sub-pages of one superpage.
#[repr(C, align(64))]
pub struct SuperpageResidency {
    bits: [AtomicU64; WORDS],
}

const _: () = assert!(size_of::<SuperpageResidency>() == 64, "SuperpageResidency must be one cache line");
const _: () = assert!(align_of::<SuperpageResidency>() == 64, "SuperpageResidency must be 64-byte aligned");

impl Default for SuperpageResidency {
    fn default() -> Self {
        Self::new()
    }
}

impl SuperpageResidency {
    /// A fully non-resident superpage.
    pub const fn new() -> Self {
        Self { bits: [const { AtomicU64::new(0) }; WORDS] }
    }

    /// Mark sub-page `idx` resident.  Returns `false` for an out-of-range
    /// index (and leaves every word untouched).
    pub fn set_bit(&self, idx: usize) -> bool {
        let Some((word, mask)) = Self::locate(idx) else {
            return false;
        };
        self.bits[word].fetch_or(mask, Ordering::AcqRel);
        true
    }

    /// Clear sub-page `idx`.  Returns `false` for an out-of-range index.
    pub fn clear_bit(&self, idx: usize) -> bool {
        let Some((word, mask)) = Self::locate(idx) else {
            return false;
        };
        self.bits[word].fetch_and(!mask, Ordering::AcqRel);
        true
    }

    /// Whether every one of the 512 sub-pages is resident.
    pub fn is_full(&self) -> bool {
        self.count() == SUBPAGES
    }

    /// The number of resident sub-pages.
    pub fn count(&self) -> usize {
        self.bits.iter().map(|word| word.load(Ordering::Acquire).count_ones() as usize).sum()
    }

    /// Demotion: atomically clear every residency bit.  Each word is stored
    /// with `Release`, so a reader that observes the zero word also sees the
    /// demotion ordering.
    pub fn reset(&self) {
        for word in &self.bits {
            word.store(0, Ordering::Release);
        }
    }

    /// `(word index, bit mask)` for `idx`, or `None` when out of range.
    fn locate(idx: usize) -> Option<(usize, u64)> {
        if idx >= SUBPAGES {
            return None;
        }
        Some((idx / 64, 1u64 << (idx % 64)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 6 (MEM-08 size/align/CRUD): exact layout and demotion reset.
    #[test]
    fn superpage_residency_is_exactly_64_bytes_and_counts_bits() {
        assert_eq!(size_of::<SuperpageResidency>(), 64);
        assert_eq!(align_of::<SuperpageResidency>(), 64);

        let residency = SuperpageResidency::new();
        assert_eq!(residency.count(), 0);
        assert!(!residency.is_full());

        for idx in 0..SUBPAGES {
            assert!(residency.set_bit(idx), "bit {idx} is in range");
        }
        assert!(residency.is_full());
        assert_eq!(residency.count(), SUBPAGES);

        assert!(residency.clear_bit(64));
        assert!(!residency.is_full());
        assert_eq!(residency.count(), SUBPAGES - 1);

        residency.reset();
        assert_eq!(residency.count(), 0);
        assert!(!residency.is_full());
        assert_eq!(size_of::<SuperpageResidency>(), 64, "reset changes no layout");
    }

    /// MEM-08 adjacency: bit indices 0, 63, 64 and 511 map across adjacent
    /// `u64` words; out-of-range indices are rejected.
    #[test]
    fn superpage_bits_cross_word_boundaries_without_overflow() {
        let residency = SuperpageResidency::new();
        for idx in [0usize, 63, 64, 511] {
            assert!(residency.set_bit(idx));
        }
        assert_eq!(residency.count(), 4);
        assert_eq!(residency.bits[0].load(Ordering::Acquire), 1 | (1 << 63));
        assert_eq!(residency.bits[1].load(Ordering::Acquire), 1);
        assert_eq!(residency.bits[7].load(Ordering::Acquire), 1 << 63);

        assert!(!residency.set_bit(SUBPAGES));
        assert!(!residency.clear_bit(SUBPAGES));
        assert!(!residency.set_bit(usize::MAX));
        assert_eq!(residency.count(), 4);
    }
}
