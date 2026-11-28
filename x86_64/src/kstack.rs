//! x86_64/src/kstack.rs — the per-process kernel stacks.
//!
//! Every process's kernel stack is one slot of an arena reserved from
//! RAM at boot and reached at `KZERO + pa`.  The boot tables map that
//! alias in every page table, so every CPU can write every other CPU's
//! stack — which is what the portable MCS locks need: their queue nodes
//! live on the calling CPU's kernel stack, and an acquire writes the
//! PREDECESSOR's node (`predecessor.next.store`, port/src/mcslock.rs).
//! Until 2026-11-27 each proc mapped its own stack at one private VA
//! window under its own PML4; a cross-CPU hand-off then wrote the
//! waiter's stack instead of the holder's node, and both CPUs spun
//! forever (.planning/debug/x86_64-smp2-mcs-kstack.md).
//!
//! The lowest page of every slot is a GUARD: the kernel alias is split
//! around the arena at boot (directmap.rs's `arm_guards`) and that page
//! is left unmapped, so a kernel path that runs off the bottom of its
//! stack takes a page fault at a named address instead of writing into
//! the neighbouring slot.  Contiguous slots alone made that silent:
//! with 16 KiB slots `ls '#M'` ran the in-kernel loopback's crypto off
//! the bottom of its own slot and overwrote the root file server's
//! saved context in the next one (2026-11-28).  The stacks are 32 KiB
//! now (proc.rs's KSTACK_PAGES), and the guard is what keeps the next
//! over-deep path loud.
//!
//! Reserving the arena at boot is what keeps this simple: the slots are
//! contiguous and SLOT_BYTES-aligned with no page-table work per proc —
//! nothing to unmap at exit, no TLB shootdown, and a returned slot can
//! be handed out again at once.  A slot is only returned by the sched
//! loop's deferred reap, after the exiting CPU has left the stack
//! (proc.rs's `reap_moribund`), so a sibling's new proc can never be
//! started on a stack another CPU still runs on.
//!
//! The arena sits below the 1 GiB alias ceiling today.  [`top_va`] is
//! the ONE place a stack's VA is derived: rebuilding the kernel's map of
//! RAM for large machines (.planning/SCALE-AUDIT.md, planning item 2)
//! moves that function and nothing here.
//!
//! Host tests reach the same seam with real host memory — a test "PA"
//! is a SLOT_BYTES-aligned host pointer and the alias is the identity
//! (proc.rs's mock-proc doctrine) — so new_user_proc is exercised on
//! the host exactly as on metal.

use core::sync::atomic::{AtomicU64, Ordering};

use port::mem::PAGE_SIZE_4K;

use crate::proc::{KSTACK_PAGES, MAX_PROCS};

/// One kernel stack slot: its pages, contiguous; the lowest is the
/// unmapped guard page below the stack (the module header).
pub const SLOT_BYTES: u64 = (KSTACK_PAGES * PAGE_SIZE_4K) as u64;

/// Whether `va` lies on a stack's guard page: the first page of a slot
/// of the arena whose first byte is at `base_va`.  A fault names one
/// byte of the page, so the page is what is classified.
fn is_guard_page(base_va: u64, va: u64) -> bool {
    let page = va & !(PAGE_SIZE_4K as u64 - 1);
    let off = page.wrapping_sub(base_va);
    page >= base_va && off < arena_bytes() && off.is_multiple_of(SLOT_BYTES)
}

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

    use port::mem::PAGE_SIZE_4K;

    use super::{Bitmap, MAX_PROCS, SLOT_BYTES};
    use crate::proc::{KZERO, phys_ptr_mut};

    /// The arena's base PA — 0 until `init`.
    static BASE: AtomicU64 = AtomicU64::new(0);
    static FREE: Bitmap = Bitmap::new();
    static GUARDS: AtomicU64 = AtomicU64::new(0);

    /// Split the kernel alias around the arena at `base` and leave a
    /// guard page below every slot; the table pages come from `pages`
    /// `[start, end)`, a run in the first GiB.  Runs once at boot, before
    /// any AP starts and any proc exists, so every page table shares the
    /// result (each copies the boot PML4's kernel half).
    pub fn arm(base: u64, pages: (u64, u64)) -> Result<(), &'static str> {
        assert!(base.is_multiple_of(2 << 20), "kstack: arena base {base:#x} is not 2 MiB-aligned");
        let bytes = super::arena_bytes();
        let split = crate::directmap::arm_guards(base, bytes, SLOT_BYTES, pages)?;
        GUARDS.store(split.guards as u64, Ordering::Release);
        Ok(())
    }

    /// Publish the arena at `base` (SLOT_BYTES-aligned, already withheld
    /// from the frame allocator and armed) and zero its mapped pages, so
    /// every stack starts from the same state the page allocator's
    /// zeroed pages used to give.  The guard pages are not touched: they
    /// are unmapped, and zeroing them is what proves the split kept
    /// every other page reachable.
    pub fn init(base: u64) {
        assert!(base != 0, "kstack: arena base is zero");
        assert!(base.is_multiple_of(SLOT_BYTES), "kstack: arena base {base:#x} is not {SLOT_BYTES:#x}-aligned");
        for slot in 0..MAX_PROCS as u64 {
            let start = base + slot * SLOT_BYTES + PAGE_SIZE_4K as u64;
            // SAFETY: the boot reservation names the arena at `base`,
            // inside the alias window (the page allocator's seeding chose
            // it there and holds it out of its pools); `start` is the
            // first mapped page of one slot and the length is the rest
            // of it.  Nothing else references the span until the first
            // alloc below.
            unsafe {
                core::ptr::write_bytes(
                    phys_ptr_mut(start),
                    0,
                    (SLOT_BYTES - PAGE_SIZE_4K as u64) as usize,
                );
            }
        }
        BASE.store(base, Ordering::Release);
    }

    /// Whether `va` is one of the guard pages below a stack — the
    /// signature of a kernel path that ran off the bottom of its stack.
    pub fn is_guard(va: usize) -> bool {
        let base = BASE.load(Ordering::Acquire);
        base != 0 && super::is_guard_page(KZERO + base, va as u64)
    }

    /// `(slots, slot bytes, guard pages)` for the boot line.
    pub fn info() -> (usize, u64, usize) {
        (MAX_PROCS, SLOT_BYTES, GUARDS.load(Ordering::Acquire) as usize)
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
pub use live::{alloc, arm, free, info, init, is_guard, top_va};

/// The host-test seam: a SLOT_BYTES-aligned, zeroed host block stands in
/// for the slot, its address for the PA, and the alias is the identity.
/// No page tables exist on the host, so nothing is ever a guard there.
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
pub fn is_guard(_va: usize) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{Bitmap, MAX_PROCS, PAGE_SIZE_4K, SLOT_BYTES, arena_bytes, is_guard_page};

    #[test]
    fn arena_is_one_slot_per_process_table_entry() {
        assert_eq!(arena_bytes(), MAX_PROCS as u64 * SLOT_BYTES);
    }

    /// A slot is eight pages, and its first is the guard: 32 KiB stacks
    /// with 28 KiB to run on.
    #[test]
    fn a_slot_is_eight_pages_and_its_first_page_is_the_guard() {
        assert_eq!(SLOT_BYTES, 32 * 1024);
        let base = 0xffff_ff00_0000_0000u64;
        let top = base + arena_bytes();
        assert!(is_guard_page(base, base), "the first slot's guard");
        assert!(is_guard_page(base, base + 8), "the pushed frame's second qword");
        assert!(is_guard_page(base, base + PAGE_SIZE_4K as u64 - 8), "the page's last qword");
        assert!(is_guard_page(base, base + SLOT_BYTES), "the second slot's guard");
        assert!(is_guard_page(base, top - SLOT_BYTES + 0x1f0), "the last slot's guard");
        // the stack's own pages, the page above the arena and everything below it
        assert!(!is_guard_page(base, base + 4096));
        assert!(!is_guard_page(base, base + SLOT_BYTES - 4096));
        assert!(!is_guard_page(base, top));
        assert!(!is_guard_page(base, base - 4096));
        assert!(!is_guard_page(base, base + 4096 * 9), "the second page of the second slot");
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
