//! x86_64/src/kstack.rs — the per-process kernel stacks.
//!
//! Every process's 16 KiB kernel stack is one slot of an arena reserved
//! from RAM at boot and reached at `KZERO + pa`.  The boot tables map
//! that alias in every page table, so every CPU can write every other
//! CPU's stack — which is what the portable MCS locks need: their queue
//! nodes live on the calling CPU's kernel stack, and an acquire writes
//! the PREDECESSOR's node (`predecessor.next.store`,
//! port/src/mcslock.rs).  Until 2026-11-27 each proc mapped its own
//! stack at one private VA window under its own PML4; a cross-CPU
//! hand-off then wrote the waiter's stack instead of the holder's node,
//! and both CPUs spun forever
//! (.planning/debug/x86_64-smp2-mcs-kstack.md).
//!
//! Reserving the arena at boot is what keeps this simple: the slots are
//! contiguous and 16 KiB-aligned with no page-table work at all — no
//! per-proc table pages (three per proc before), nothing to unmap at
//! exit, no TLB shootdown, and a returned slot can be handed out again
//! at once.  A slot is only returned by the sched loop's deferred reap,
//! after the exiting CPU has left the stack (proc.rs's
//! `reap_moribund`), so a sibling's new proc can never be started on a
//! stack another CPU still runs on.
//!
//! The arena sits below the 1 GiB alias ceiling today.  [`top_va`] is
//! the ONE place a stack's VA is derived: rebuilding the kernel's map of
//! RAM for large machines (.planning/SCALE-AUDIT.md, planning item 2)
//! moves that function and nothing here.
//!
//! Host tests reach the same seam with real host memory — a test "PA"
//! is a 16 KiB-aligned host pointer and the alias is the identity
//! (proc.rs's mock-proc doctrine) — so new_user_proc is exercised on
//! the host exactly as on metal.

use core::sync::atomic::{AtomicU64, Ordering};

use port::mem::PAGE_SIZE_4K;

use crate::proc::{KSTACK_PAGES, MAX_PROCS};

/// One kernel stack: four pages, contiguous.
pub const SLOT_BYTES: u64 = (KSTACK_PAGES * PAGE_SIZE_4K) as u64;

/// The arena holds one slot per process-table entry: a proc can never
/// fail for want of a stack while the table has room.
const WORDS: usize = MAX_PROCS.div_ceil(64);

/// The free slots, one bit each (set = taken).  Lock-free: a slot is
/// taken with a CAS on its word and returned with a bit clear.
struct Bitmap {
    used: [AtomicU64; WORDS],
}

impl Bitmap {
    const fn new() -> Bitmap {
        Bitmap { used: [const { AtomicU64::new(0) }; WORDS] }
    }

    /// The lowest free slot, or None when every slot is taken.
    fn alloc(&self) -> Option<usize> {
        for (w, word) in self.used.iter().enumerate() {
            let mut bits = word.load(Ordering::Relaxed);
            while bits != !0 {
                let bit = bits.trailing_ones() as usize;
                match word.compare_exchange_weak(
                    bits,
                    bits | 1 << bit,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Some(w * 64 + bit),
                    Err(now) => bits = now,
                }
            }
        }
        None
    }

    /// Return a slot.  False when it was already free — a double free,
    /// or a PA that never came from [`alloc`](Bitmap::alloc).
    fn free(&self, slot: usize) -> bool {
        let Some(word) = self.used.get(slot / 64) else { return false };
        word.fetch_and(!(1 << (slot % 64)), Ordering::AcqRel) & (1 << (slot % 64)) != 0
    }
}

/// The arena's size: one slot per process-table entry.
pub fn arena_bytes() -> u64 {
    MAX_PROCS as u64 * SLOT_BYTES
}

#[cfg(not(test))]
mod live {
    use core::sync::atomic::{AtomicU64, Ordering};

    use super::{Bitmap, SLOT_BYTES};
    use crate::proc::{KZERO, phys_ptr_mut};

    /// The arena's base PA — 0 until `init`.
    static BASE: AtomicU64 = AtomicU64::new(0);
    static FREE: Bitmap = Bitmap::new();

    /// Publish the arena at `base` (16 KiB-aligned, already withheld
    /// from the frame allocator) and zero it, so every stack starts from
    /// the same state the page allocator's zeroed pages used to give.
    pub fn init(base: u64) {
        assert!(base != 0, "kstack: arena base is zero");
        assert!(base.is_multiple_of(SLOT_BYTES), "kstack: arena base {base:#x} is not {SLOT_BYTES:#x}-aligned");
        let bytes = super::arena_bytes() as usize;
        // SAFETY: the boot reservation names `bytes` bytes of RAM at
        // `base`, inside the alias window (the page allocator's seeding
        // chose it there and holds it out of its pools); nothing else
        // references the span until the first alloc below.
        unsafe { core::ptr::write_bytes(phys_ptr_mut(base), 0, bytes) };
        BASE.store(base, Ordering::Release);
    }

    /// Take a slot; its base PA.  None once every slot is taken.
    pub fn alloc() -> Option<u64> {
        let base = BASE.load(Ordering::Acquire);
        assert!(base != 0, "kstack: alloc before init");
        FREE.alloc().map(|slot| base + slot as u64 * SLOT_BYTES)
    }

    /// Return the slot whose base PA is `base_pa`.
    pub fn free(base_pa: u64) {
        let base = BASE.load(Ordering::Acquire);
        debug_assert!(
            base != 0 && base_pa >= base && base_pa < base + super::arena_bytes(),
            "kstack: {base_pa:#x} is not an arena slot"
        );
        let slot = ((base_pa - base) / SLOT_BYTES) as usize;
        debug_assert!(FREE.free(slot), "kstack: slot {base_pa:#x} returned twice");
    }

    /// The stack's top VA — the alias of the byte after its last page.
    pub fn top_va(base_pa: u64) -> usize {
        (KZERO + base_pa + SLOT_BYTES) as usize
    }
}

#[cfg(not(test))]
pub use live::{alloc, free, init, top_va};

/// The host-test seam: a 16 KiB-aligned, zeroed host block stands in
/// for the slot, its address for the PA, and the alias is the identity.
#[cfg(test)]
mod host {
    use alloc::alloc::{Layout, alloc_zeroed, dealloc};

    use super::SLOT_BYTES;

    fn layout() -> Layout {
        Layout::from_size_align(SLOT_BYTES as usize, SLOT_BYTES as usize).unwrap()
    }

    pub fn alloc() -> Option<u64> {
        // SAFETY: the layout has a non-zero size.
        let p = unsafe { alloc_zeroed(layout()) } as u64;
        (p != 0).then_some(p)
    }

    pub fn free(base_pa: u64) {
        // SAFETY: `base_pa` is a live block this module's `alloc` returned
        // for exactly this layout.
        unsafe { dealloc(base_pa as *mut u8, layout()) };
    }

    pub fn top_va(base_pa: u64) -> usize {
        (base_pa + SLOT_BYTES) as usize
    }
}

#[cfg(test)]
pub use host::{alloc, free, top_va};

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{Bitmap, MAX_PROCS, SLOT_BYTES, arena_bytes};

    #[test]
    fn arena_is_one_slot_per_process_table_entry() {
        assert_eq!(arena_bytes(), MAX_PROCS as u64 * SLOT_BYTES);
    }

    #[test]
    fn slots_are_handed_out_lowest_first_and_run_out() {
        let b = Bitmap::new();
        let first: Vec<usize> = (0..200).map(|_| b.alloc().unwrap()).collect();
        assert_eq!(first, (0..200).collect::<Vec<_>>());
        for _ in 200..MAX_PROCS {
            assert!(b.alloc().is_some());
        }
        assert!(b.alloc().is_none(), "every process-table entry already has its slot");
    }

    #[test]
    fn a_returned_slot_is_reused_and_a_second_return_is_refused() {
        let b = Bitmap::new();
        assert_eq!(b.alloc(), Some(0));
        assert_eq!(b.alloc(), Some(1));
        assert!(b.free(0));
        assert!(!b.free(0), "a double return must be visible");
        assert_eq!(b.alloc(), Some(0), "the freed slot is the lowest free one again");
        assert!(!b.free(MAX_PROCS), "a slot index past the arena is refused");
    }

    #[test]
    fn words_are_independent() {
        // Slots straddling word boundaries (64, 128, 192): a set bit in
        // one word must not hide a free bit in the next.
        let b = Bitmap::new();
        for _ in 0..64 {
            b.alloc().unwrap();
        }
        assert_eq!(b.alloc(), Some(64));
        assert!(b.free(63));
        assert_eq!(b.alloc(), Some(63));
        assert_eq!(b.alloc(), Some(65));
    }
}
