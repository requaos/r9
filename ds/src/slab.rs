#![forbid(unsafe_code)]

//! slab — the generic lock-free seqlock slab (CORR-03; Phase 5 CONTEXT D-09
//! to D-12).
//!
//! A slab is a flat array of 64-byte seqlock slots plus allocation metadata.
//! It backs the sharded generational `FidTable` of `port/src/fid.rs`
//! (plan 05-06); the compendium section 4 slab listing is amended by D-07,
//! D-08, D-11 and D-12.  This module is `#![forbid(unsafe_code)]` (D-09):
//! every field a reader can touch is an atomic, so the concurrent
//! reader/writer protocol below is zero data-race by construction and Miri
//! can prove it.
//!
//! **Slot layout (D-11).**  Each [`AtomicSlabSlot`] is exactly one cache
//! line — `#[repr(C, align(64))]`, 64 bytes at the default `WORDS = 7`: one
//! `AtomicU64` state word and one `[AtomicU64; 7]` payload.  The state word
//! packs bit 0 `WRITING`, bit 1 `OCCUPIED`, bits 2..=17 the 16-bit
//! generation and bits 18..=63 a 46-bit sequence counter.
//!
//! **Generation (D-10).**  A slot's generation starts at 1 and never equals
//! 0: [`next_generation`] wraps `65535 -> 1`, skipping 0, so a wire handle
//! that carries a generation is never all zero.  `resolve` rejects
//! `expected_gen == 0` outright.
//!
//! **Boehm fence-paired seqlock (D-11).**  The publication protocol is the
//! Boehm ordering (Boehm, "Can Seqlocks Get Along With Programming Language
//! Memory Models?", MSPC 2012), the same ordering `ds/src/tpht/flattened.rs`
//! documents and Miri verifies:
//!
//! - writer: store `state | WRITING` (`Relaxed`); `fence(Release)`; store
//!   the payload words (`Relaxed`; a clunk stores zeros, scrubbing the
//!   payload); store the published state (`Release`);
//! - reader (`resolve`): load state (`Acquire`); spin while `WRITING`;
//!   return `None` when unoccupied or the generation differs; load the words
//!   (`Relaxed`); `fence(Acquire)`; reload state (`Relaxed`); accept the
//!   snapshot only when the two state loads agree, else retry.
//!
//! The generation bumps only on clunk; the sequence bumps on alloc, in-place
//! update and clunk.  An in-place update therefore forces concurrent readers
//! to retry (the sequence changed) without invalidating the client's wire
//! handle (the generation is unchanged).
//!
//! **Serialisation contract.**  `ds/` has no mutex — the MCS `Lock` lives in
//! `port/` — so every writer (`alloc_slot`, `update_slot`, `clunk_slot`,
//! `sweep_unreclaimed`) takes its [`AllocMeta`] by `&mut` and the caller
//! serialises those calls with a shard-local lock (`port/src/fid.rs` wraps
//! `AllocMeta` in `Lock`).  `resolve` never touches that lock: it is a
//! lock-free single-cache-line read.  The slot array is a flat
//! `Box<[AtomicSlabSlot]>` sized as a power of two at construction and never
//! reallocates (D-12).
//!
//! **Failover bookkeeping (WIRE-09 restated for the slab).**  [`AllocMeta`]
//! tracks the free list and the `active` and `reclaimed` slot sets in
//! Roaring bitmaps.  `mark_reclaimed` keeps a slot through the next sweep;
//! `sweep_unreclaimed` clunks every active slot that was not marked
//! reclaimed and resets the reclaim window.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering, fence};

use crate::roaring::RoaringBitmap;

/// State bit 0: a writer holds the slot (D-11).
const WRITING: u64 = 1 << 0;
/// State bit 1: the slot holds a live entry (D-11; replaces the Phase 4
/// `FID_LIVE` entry flag).
const OCCUPIED: u64 = 1 << 1;
/// State bits 2..=17: the 16-bit generation.
const GEN_SHIFT: u32 = 2;
/// The generation's mask within the state word.
const GEN_MASK: u64 = 0xFFFF << GEN_SHIFT;
/// State bits 18..=63: the 46-bit sequence counter.
const SEQ_SHIFT: u32 = 18;
/// One sequence step within the state word.
const SEQ_INC: u64 = 1 << SEQ_SHIFT;

/// The next slot generation: `g + 1`, wrapping `65535 -> 1` so a generation
/// is never 0 (D-10).
pub fn next_generation(g: u16) -> u16 {
    g.wrapping_add(1).max(1)
}

/// The generation packed into `state`.
fn decode_gen(state: u64) -> u16 {
    ((state & GEN_MASK) >> GEN_SHIFT) as u16
}

/// A test seam for counting slot reads (CORR-03's restatement of the
/// Phase 4 WIRE-08 probe pin): production uses [`NoSlabProbe`]'s zero-sized
/// no-ops; a test probe counts `resolve` attempts so an uncontended resolve
/// can be shown to touch exactly one cache line and zero metadata.
pub trait SlabProbe {
    /// Called once per `resolve` attempt, before the sequence load.
    fn slot_read(&self) {}
}

/// The zero-sized production [`SlabProbe`]: every call compiles to nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSlabProbe;

impl SlabProbe for NoSlabProbe {}

/// One 64-byte seqlock slot: one atomic state word and `WORDS` atomic
/// payload words (D-11).
#[repr(C, align(64))]
pub struct AtomicSlabSlot<const WORDS: usize = 7> {
    state: AtomicU64,
    words: [AtomicU64; WORDS],
}

const _: () = {
    assert!(core::mem::size_of::<AtomicSlabSlot<7>>() == 64);
    assert!(core::mem::align_of::<AtomicSlabSlot<7>>() == 64);
};

impl<const WORDS: usize> AtomicSlabSlot<WORDS> {
    /// A fresh slot: generation 1 (never 0, D-10), unoccupied, not being
    /// written, sequence 0, payload zeroed.
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(1u64 << GEN_SHIFT),
            words: core::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    /// Lock-free consistent snapshot of the slot's payload for a handle
    /// carrying `expected_gen`: one cache-line read per attempt, invoking
    /// `on_read` once per attempt (the [`SlabProbe`] seam).
    ///
    /// Returns `None` when `expected_gen` is 0 (D-10), when the slot is
    /// unoccupied, or when its generation differs; retries while a writer is
    /// in flight or the state changed mid-read (Boehm seqlock, D-11).
    #[inline]
    pub fn resolve(&self, expected_gen: u16, on_read: impl Fn()) -> Option<[u64; WORDS]> {
        if expected_gen == 0 {
            return None; // D-10: a generation is never zero
        }
        loop {
            on_read();
            let s1 = self.state.load(Ordering::Acquire);
            if (s1 & WRITING) != 0 {
                core::hint::spin_loop();
                continue;
            }
            if (s1 & OCCUPIED) == 0 || decode_gen(s1) != expected_gen {
                return None;
            }
            let mut raw = [0u64; WORDS];
            for (dst, src) in raw.iter_mut().zip(self.words.iter()) {
                *dst = src.load(Ordering::Relaxed);
            }
            fence(Ordering::Acquire);
            let s2 = self.state.load(Ordering::Relaxed);
            if s1 == s2 {
                return Some(raw);
            }
        }
    }

    /// The Boehm write sequence: raise `WRITING` (`Relaxed`), `fence`(
    /// `Release`), store the payload (`Relaxed`), then publish the new state
    /// (`Release`).  Writers are serialised by the caller's `AllocMeta`
    /// lock, so the load-then-store of `state` cannot race another writer.
    fn publish(&self, cur: u64, words: &[u64; WORDS], published: u64) {
        debug_assert_eq!(cur & WRITING, 0, "writers are serialised by the AllocMeta lock");
        self.state.store(cur | WRITING, Ordering::Relaxed);
        fence(Ordering::Release);
        for (dst, &word) in self.words.iter().zip(words.iter()) {
            dst.store(word, Ordering::Relaxed);
        }
        self.state.store(published, Ordering::Release);
    }

    /// The next published state for an alloc or an in-place update: same
    /// generation, sequence bumped, `OCCUPIED` set, `WRITING` cleared.
    fn next_live_state(cur: u64) -> u64 {
        (cur & GEN_MASK) | ((cur >> SEQ_SHIFT).wrapping_add(1) << SEQ_SHIFT) | OCCUPIED
    }

    /// Publish `words` as a fresh allocation: keeps the generation, sets
    /// `OCCUPIED` and bumps the sequence.  Returns the slot's generation.
    ///
    /// The caller serialises writers (the shard's `AllocMeta` lock).
    pub fn publish_alloc(&self, words: [u64; WORDS]) -> u16 {
        let cur = self.state.load(Ordering::Relaxed);
        debug_assert_eq!(cur & OCCUPIED, 0, "alloc publishes on a free slot");
        self.publish(cur, &words, Self::next_live_state(cur));
        decode_gen(cur)
    }

    /// Publish an in-place update: keeps the generation, keeps `OCCUPIED`,
    /// bumps the sequence so concurrent readers retry (D-11).  Returns
    /// `false` — publishing nothing — when the slot is unoccupied or its
    /// generation differs from `expected_gen`.
    ///
    /// The caller serialises writers (the shard's `AllocMeta` lock).
    pub fn publish_update(&self, expected_gen: u16, words: [u64; WORDS]) -> bool {
        let cur = self.state.load(Ordering::Relaxed);
        if (cur & OCCUPIED) == 0 || decode_gen(cur) != expected_gen {
            return false;
        }
        self.publish(cur, &words, Self::next_live_state(cur));
        true
    }

    /// Publish a clunk: scrubs every payload word to zero, clears
    /// `OCCUPIED`, bumps the generation via [`next_generation`] (skipping 0)
    /// and bumps the sequence (D-10, D-11).  Returns `false` — clunking
    /// nothing — when the slot is unoccupied or its generation differs from
    /// `expected_gen`, so a duplicate clunk of an already-clunked slot is
    /// rejected cleanly.
    ///
    /// The caller serialises writers (the shard's `AllocMeta` lock).
    pub fn publish_clunk(&self, expected_gen: u16) -> bool {
        let cur = self.state.load(Ordering::Relaxed);
        if (cur & OCCUPIED) == 0 || decode_gen(cur) != expected_gen {
            return false;
        }
        self.clunk(cur);
        true
    }

    /// The unconditional clunk the failover sweep uses: it collects a slot
    /// by index, with no handle and so no generation to check.
    pub(crate) fn publish_clunk_any(&self) {
        let cur = self.state.load(Ordering::Relaxed);
        debug_assert_ne!(cur & OCCUPIED, 0, "the sweep only collects live slots");
        self.clunk(cur);
    }

    /// The shared clunk publication: zeroed payload, bumped generation,
    /// bumped sequence, `OCCUPIED` and `WRITING` cleared.
    fn clunk(&self, cur: u64) {
        let generation = next_generation(decode_gen(cur));
        let published =
            ((generation as u64) << GEN_SHIFT) | ((cur >> SEQ_SHIFT).wrapping_add(1) << SEQ_SHIFT);
        self.publish(cur, &[0u64; WORDS], published);
    }
}

impl<const WORDS: usize> Default for AtomicSlabSlot<WORDS> {
    fn default() -> Self {
        Self::new()
    }
}

/// The immutable slot array of one slab: a flat `Box<[AtomicSlabSlot]>`
/// sized as a power of two at construction and never reallocated (D-11,
/// D-12), plus the [`SlabProbe`] seam.
pub struct SlabSlots<const WORDS: usize = 7, P: SlabProbe = NoSlabProbe> {
    slots: Box<[AtomicSlabSlot<WORDS>]>,
    probe: P,
}

impl SlabSlots<7, NoSlabProbe> {
    /// Build a slab of `capacity` slots (a nonzero power of two, D-12) with
    /// the zero-sized [`NoSlabProbe`].
    pub fn new(capacity: u16) -> Self {
        Self::with_probe(capacity, NoSlabProbe)
    }
}

impl<const WORDS: usize, P: SlabProbe> SlabSlots<WORDS, P> {
    /// Build a slab of `capacity` slots with an injected [`SlabProbe`] (the
    /// test seam).
    pub fn with_probe(capacity: u16, probe: P) -> Self {
        assert!(
            capacity.is_power_of_two(),
            "slab capacity is a nonzero power of two (D-12), got {capacity}"
        );
        let slots = (0..capacity as usize)
            .map(|_| AtomicSlabSlot::<WORDS>::new())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self { slots, probe }
    }

    /// The number of slots.
    pub fn capacity(&self) -> u16 {
        self.slots.len() as u16
    }

    /// The slot array, for the serialised writers in [`AllocMeta`].
    pub fn slots(&self) -> &[AtomicSlabSlot<WORDS>] {
        &self.slots
    }

    /// Lock-free consistent snapshot of slot `idx`'s payload for a handle
    /// carrying `expected_gen`; `None` for an out-of-range index, a zero
    /// generation, an unoccupied slot or a generation mismatch.
    #[inline]
    pub fn resolve(&self, idx: u16, expected_gen: u16) -> Option<[u64; WORDS]> {
        self.slots
            .get(idx as usize)?
            .resolve(expected_gen, || self.probe.slot_read())
    }
}

/// The slab's allocation metadata: the free list and the `active` and
/// `reclaimed` slot sets (D-11).  `ds/` has no mutex, so the caller
/// serialises every `&mut self` method with a shard-local lock;
/// [`SlabSlots::resolve`] never touches this structure.
pub struct AllocMeta {
    free_list: Vec<u16>,
    active: RoaringBitmap,
    reclaimed: RoaringBitmap,
    capacity: u16,
}

impl AllocMeta {
    /// Metadata for a slab of `capacity` slots: every slot free.
    pub fn new(capacity: u16) -> Self {
        assert!(
            capacity.is_power_of_two(),
            "slab capacity is a nonzero power of two (D-12), got {capacity}"
        );
        Self {
            // Popping yields the lowest free index first.
            free_list: (0..capacity).rev().collect(),
            active: RoaringBitmap::new(),
            reclaimed: RoaringBitmap::new(),
            capacity,
        }
    }

    /// The slab's slot count.
    pub fn capacity(&self) -> u16 {
        self.capacity
    }

    /// The number of live (allocated, not clunked) slots.
    pub fn active_count(&self) -> u64 {
        self.active.len()
    }

    /// The number of live slots marked reclaimed (kept through the next
    /// sweep).
    pub fn reclaimed_count(&self) -> u64 {
        self.reclaimed.len()
    }

    /// The number of slots available to [`Self::alloc_slot`].
    pub fn free_count(&self) -> usize {
        self.free_list.len()
    }

    /// Whether `idx` is live (allocated and not clunked).
    pub fn is_active(&self, idx: u16) -> bool {
        self.active.contains(idx as u32)
    }

    /// Whether `idx` is marked reclaimed.
    pub fn is_reclaimed(&self, idx: u16) -> bool {
        self.reclaimed.contains(idx as u32)
    }

    fn check_slots<T>(&self, slots: &[T]) {
        assert_eq!(
            slots.len(),
            self.capacity as usize,
            "the slot array must match the metadata's capacity"
        );
    }

    /// Allocate the lowest free slot, publishing `words` in it, and return
    /// its `(index, generation)`; `None` when the slab is full.
    pub fn alloc_slot<const WORDS: usize>(
        &mut self,
        slots: &[AtomicSlabSlot<WORDS>],
        words: [u64; WORDS],
    ) -> Option<(u16, u16)> {
        self.check_slots(slots);
        let idx = self.free_list.pop()?;
        let generation = slots[idx as usize].publish_alloc(words);
        self.active.insert(idx as u32);
        Some((idx, generation))
    }

    /// Publish an in-place update of live slot `idx` (which must carry
    /// `expected_gen`): the generation is preserved so the client's wire
    /// handle stays valid, and the sequence bump makes concurrent readers
    /// retry (D-11).  Returns `false` for an out-of-range, stale or not
    /// live slot.
    pub fn update_slot<const WORDS: usize>(
        &mut self,
        slots: &[AtomicSlabSlot<WORDS>],
        idx: u16,
        expected_gen: u16,
        words: [u64; WORDS],
    ) -> bool {
        self.check_slots(slots);
        if idx >= self.capacity || !self.active.contains(idx as u32) {
            return false;
        }
        slots[idx as usize].publish_update(expected_gen, words)
    }

    /// Clunk live slot `idx` (which must carry `expected_gen`): scrubs its
    /// payload, clears `OCCUPIED`, bumps the generation (skipping 0) and
    /// returns the slot to the free list, dropping it from the `active` and
    /// `reclaimed` sets.  Returns `false` for an out-of-range, stale or
    /// already-clunked slot, so a duplicate clunk is rejected cleanly.
    pub fn clunk_slot<const WORDS: usize>(
        &mut self,
        slots: &[AtomicSlabSlot<WORDS>],
        idx: u16,
        expected_gen: u16,
    ) -> bool {
        self.check_slots(slots);
        if idx >= self.capacity || !self.active.contains(idx as u32) {
            return false;
        }
        if !slots[idx as usize].publish_clunk(expected_gen) {
            return false;
        }
        self.active.remove(idx as u32);
        self.reclaimed.remove(idx as u32);
        self.free_list.push(idx);
        true
    }

    /// Mark live slot `idx` reclaimed so the next [`Self::sweep_unreclaimed`]
    /// keeps it (WIRE-09).  Returns `false` for an out-of-range or not live
    /// slot.
    pub fn mark_reclaimed(&mut self, idx: u16) -> bool {
        if idx >= self.capacity || !self.active.contains(idx as u32) {
            return false;
        }
        self.reclaimed.insert(idx as u32)
    }

    /// The failover sweep: clunk every active slot that was not marked
    /// reclaimed, returning the swept slot indices (WIRE-09).  Membership of
    /// the result is exactly the bitmap difference `active \ reclaimed`.
    /// Afterwards the reclaim window is empty and `active` holds exactly the
    /// survivors.
    pub fn sweep_unreclaimed<const WORDS: usize>(
        &mut self,
        slots: &[AtomicSlabSlot<WORDS>],
    ) -> Vec<u16> {
        self.check_slots(slots);
        let orphans = self.active.difference(&self.reclaimed);
        let mut swept = Vec::new();
        for value in orphans.iter() {
            let idx = value as u16;
            slots[idx as usize].publish_clunk_any();
            self.active.remove(value);
            self.free_list.push(idx);
            swept.push(idx);
        }
        // The reclaim window resets for the next failover round.
        self.reclaimed = RoaringBitmap::new();
        swept
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use super::*;

    /// A slot's raw state word (the test module is the layout's verifier).
    fn state_of(slot: &AtomicSlabSlot<7>) -> u64 {
        slot.state.load(Ordering::Relaxed)
    }

    /// A slot's raw payload words.
    fn words_of(slot: &AtomicSlabSlot<7>) -> [u64; 7] {
        core::array::from_fn(|i| slot.words[i].load(Ordering::Relaxed))
    }

    /// A distinct, nonzero payload.
    fn payload(seed: u64) -> [u64; 7] {
        core::array::from_fn(|i| {
            seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(i as u64)
        })
    }

    #[test]
    fn slot_is_64_bytes_and_64_aligned() {
        assert_eq!(
            core::mem::size_of::<AtomicSlabSlot<7>>(),
            64,
            "AtomicSlabSlot<7> is exactly one cache line (D-11)"
        );
        assert_eq!(
            core::mem::align_of::<AtomicSlabSlot<7>>(),
            64,
            "AtomicSlabSlot<7> is cache-line aligned (D-11)"
        );
    }

    #[test]
    fn fresh_slot_alloc_publishes_occupied_and_resolves() {
        let slab = SlabSlots::new(8);
        let slot = &slab.slots()[3];
        let s = state_of(slot);
        assert_eq!(decode_gen(s), 1, "a fresh slot's generation is 1 (D-10)");
        assert_eq!(s & OCCUPIED, 0, "a fresh slot is unoccupied");
        assert_eq!(s & WRITING, 0, "a fresh slot is not being written");
        assert_eq!(s >> SEQ_SHIFT, 0, "a fresh slot's sequence is 0");
        assert_eq!(words_of(slot), [0u64; 7], "a fresh slot's payload is zeroed");
        assert_eq!(slab.resolve(3, 1), None, "an unoccupied slot resolves nothing");

        let mut meta = AllocMeta::new(8);
        assert_eq!(meta.free_count(), 8);
        assert_eq!(meta.active_count(), 0);
        let words = payload(0xA11C);
        let (idx, generation) = meta
            .alloc_slot(slab.slots(), words)
            .expect("a fresh slab has a free slot");
        assert_eq!(idx, 0, "the lowest free index allocates first");
        assert_eq!(generation, 1, "alloc preserves the slot's generation (D-10)");
        let s = state_of(&slab.slots()[idx as usize]);
        assert_ne!(s & OCCUPIED, 0, "alloc publishes OCCUPIED");
        assert_eq!(s >> SEQ_SHIFT, 1, "alloc increments the sequence");
        assert_eq!(decode_gen(s), 1, "alloc preserves the generation");
        assert_eq!(s & WRITING, 0, "the writer cleared WRITING on publication");
        assert_eq!(slab.resolve(idx, generation), Some(words), "resolve returns the exact payload");
        assert_eq!(slab.resolve(idx, 0), None, "generation zero never resolves (D-10)");
        assert_eq!(slab.resolve(idx, 2), None, "a mismatched generation resolves nothing");
        assert_eq!(meta.active_count(), 1);
        assert_eq!(meta.free_count(), 7);
        assert!(meta.is_active(idx));
    }

    #[test]
    fn update_keeps_generation_and_replaces_payload() {
        let slab = SlabSlots::new(8);
        let mut meta = AllocMeta::new(8);
        let (idx, generation) = meta
            .alloc_slot(slab.slots(), payload(0xB4))
            .expect("a fresh slab has a free slot");
        let before = state_of(&slab.slots()[idx as usize]);

        let new_words = payload(0xD1);
        assert!(
            meta.update_slot(slab.slots(), idx, generation, new_words),
            "an in-place update of a live slot succeeds"
        );
        let after = state_of(&slab.slots()[idx as usize]);
        assert_eq!(decode_gen(after), generation, "an update never changes the generation (D-11)");
        assert_eq!(
            after >> SEQ_SHIFT,
            (before >> SEQ_SHIFT) + 1,
            "an update increments the sequence so concurrent readers retry"
        );
        assert_ne!(after & OCCUPIED, 0, "an update keeps the slot occupied");
        assert_eq!(slab.resolve(idx, generation), Some(new_words), "resolve sees the new payload");

        // A second update with a stale generation is rejected, and a slot
        // that was never allocated rejects updates too.
        assert!(
            !meta.update_slot(slab.slots(), idx, generation + 1, payload(0xE5)),
            "a stale generation rejects the update"
        );
        assert!(
            !meta.update_slot(slab.slots(), 7, 1, payload(0xE6)),
            "an unallocated slot rejects the update"
        );
    }

    #[test]
    fn clunk_scrubs_payload_bumps_generation_and_recycles() {
        let slab = SlabSlots::new(8);
        let mut meta = AllocMeta::new(8);
        let (idx, generation) = meta
            .alloc_slot(slab.slots(), payload(0xC11C))
            .expect("a fresh slab has a free slot");
        let before = state_of(&slab.slots()[idx as usize]);

        assert!(meta.clunk_slot(slab.slots(), idx, generation), "the first clunk succeeds");
        let slot = &slab.slots()[idx as usize];
        let after = state_of(slot);
        assert_eq!(after & OCCUPIED, 0, "clunk clears OCCUPIED");
        assert_eq!(decode_gen(after), 2, "clunk bumps the generation (D-10)");
        assert_eq!(
            after >> SEQ_SHIFT,
            (before >> SEQ_SHIFT) + 1,
            "clunk increments the sequence"
        );
        assert_eq!(
            words_of(slot),
            [0u64; 7],
            "clunk scrubs all 7 payload words to zero (D-11)"
        );
        assert_eq!(slab.resolve(idx, generation), None, "the old generation no longer resolves");
        assert_eq!(slab.resolve(idx, 0), None, "generation zero never resolves (D-10)");
        assert_eq!(slab.resolve(idx, 2), None, "an unoccupied slot resolves nothing");
        assert!(!meta.is_active(idx), "clunk drops the slot from active");
        assert_eq!(meta.active_count(), 0);
        assert_eq!(meta.free_count(), 8, "clunk returns the slot to the free list");

        assert!(
            !meta.clunk_slot(slab.slots(), idx, generation),
            "a duplicate clunk of an already-clunked slot is rejected cleanly"
        );

        // The recycled slot allocates again at the bumped generation.
        meta.mark_reclaimed(idx);
        let (idx2, gen2) = meta
            .alloc_slot(slab.slots(), payload(0xD0))
            .expect("the clunked slot is back on the free list");
        assert_eq!(idx2, idx, "the freed slot is the one reused");
        assert_eq!(gen2, 2, "the recycled slot keeps its bumped generation (D-10)");
        assert_eq!(
            slab.resolve(idx2, gen2),
            Some(payload(0xD0)),
            "the recycled slot resolves its new payload"
        );

        // The generation arithmetic itself: 1 -> 2 and 65535 -> 1, never 0.
        assert_eq!(next_generation(1), 2);
        assert_eq!(next_generation(u16::MAX), 1, "generation wraps 65535 -> 1 (D-10)");
        assert_eq!(next_generation(0), 1, "generation zero maps to 1");
    }

    #[test]
    fn mark_reclaimed_and_sweep_unreclaimed() {
        let slab = SlabSlots::new(8);
        let mut meta = AllocMeta::new(8);
        let mut handles = Vec::new();
        for i in 0..4u16 {
            handles.push(
                meta.alloc_slot(slab.slots(), payload(i as u64 + 1))
                    .expect("four allocations fit in eight slots"),
            );
        }
        assert_eq!(meta.active_count(), 4);
        assert_eq!(meta.reclaimed_count(), 0);

        // Marking a live slot reclaimed succeeds; marking an unknown slot
        // does not.
        let keep = handles[1];
        assert!(meta.mark_reclaimed(keep.0), "a live slot marks reclaimed");
        assert!(!meta.mark_reclaimed(7), "a slot that is not live marks nothing");
        assert_eq!(meta.reclaimed_count(), 1);
        assert!(meta.is_reclaimed(keep.0));

        // The sweep clunks every active slot not marked reclaimed and keeps
        // the marked one (WIRE-09).
        let swept = meta.sweep_unreclaimed(slab.slots());
        let mut expected: Vec<u16> = handles.iter().map(|h| h.0).filter(|&i| i != keep.0).collect();
        let mut swept_sorted = swept.clone();
        swept_sorted.sort_unstable();
        expected.sort_unstable();
        assert_eq!(swept_sorted, expected, "the sweep collects exactly active \\ reclaimed");
        assert_eq!(meta.active_count(), 1, "only the reclaimed slot survives");
        assert!(meta.is_active(keep.0));
        assert_eq!(meta.reclaimed_count(), 0, "the reclaim window resets for the next round");
        assert_eq!(meta.free_count(), 8 - 1, "the swept slots return to the free list");

        // Swept slots were clunked: unoccupied, scrubbed, generation bumped.
        for &idx in &expected {
            let s = state_of(&slab.slots()[idx as usize]);
            assert_eq!(s & OCCUPIED, 0, "a swept slot is unoccupied (slot {idx})");
            assert_eq!(decode_gen(s), 2, "a swept slot's generation bumped (slot {idx})");
            assert_eq!(
                words_of(&slab.slots()[idx as usize]),
                [0u64; 7],
                "a swept slot's payload is scrubbed (slot {idx})"
            );
        }
        // The kept slot still resolves with its original generation.
        assert_eq!(
            slab.resolve(keep.0, keep.1),
            Some(payload(2)),
            "a reclaimed entry keeps resolving after the sweep"
        );

        // A swept slot allocates again at its bumped generation.
        let (idx, generation) = meta
            .alloc_slot(slab.slots(), payload(0x5E))
            .expect("a swept slot is back on the free list");
        assert_eq!(generation, 2, "the re-allocated slot carries the bumped generation");
        assert_eq!(slab.resolve(idx, generation), Some(payload(0x5E)));
    }
}
