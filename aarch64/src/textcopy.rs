//! rfork's user-page copy and the I-cache coherence it owes for text.
//!
//! rfork_copy (proc.rs) writes every child page into a fresh physical
//! page through the destination's KZERO DATA alias.  Since frees became
//! real (8958c9e) that page is often one another process executed
//! moments ago: the allocator hands a fork child pages the command that
//! just exited freed.  With CTR_EL0.DIC == 0 the instruction side is not
//! coherent with those data writes, and on the HVF hosts (Apple
//! M-series, 4 KiB granule) it keeps the OLD instructions for a
//! (VA, PA) pair — through unmap, tlbi and remap of the VA — while other
//! VAs aliasing the page fetch the new bytes.  So when the child's copy
//! of VA v lands on the page the previous process mapped at the same v,
//! the child runs the old owner's code: WINDOWS #49's residual (rc's
//! simple.c page landing on bind's just-executed page at 0x18000 in
//! Harvey's net-smoke fixture, HVF only — TCG models no caches).
//!
//! [`TextCopy`] is the ONE copy path for rfork: every page goes through
//! [`TextCopy::copy_page`], a text page is cleaned to PoU there, and the
//! whole I-cache is invalidated once (IC IALLUIS — it also covers any
//! VA-indexed instruction state that by-VA maintenance through the
//! kernel alias would miss) at [`TextCopy::finish`] or, on an early
//! return, at drop.
//!
//! [`selftest`] is the regression guard: at every exec-profile boot it
//! warms a code page at an executable VA, overwrites the page through
//! [`TextCopy`] exactly as rfork does, and requires the NEW code at the
//! same VA.  A control arm repeats the overwrite WITHOUT maintenance and
//! counts stale fetches, so the boot line says whether the venue can see
//! the hazard at all (HVF: 64 of 64 stale; TCG: 0 — no I-cache model).

use crate::vm;
use port::mem::PAGE_SIZE_4K;

/// The I-cache sync a batch of page copies owes: armed by the first text
/// page, discharged once by `finish` (or drop).
pub(crate) struct TextCopy {
    pending: bool,
}

impl TextCopy {
    pub(crate) const fn new() -> Self {
        TextCopy { pending: false }
    }

    /// Copy one 4 KiB page `src` → `dst`; a text page is also cleaned to
    /// the point of unification through `dst` and arms the I-cache sync.
    ///
    /// # Safety
    /// `src` and `dst` name distinct, mapped, readable/writable 4 KiB pages
    /// (rfork passes both pages' KZERO aliases).
    pub(crate) unsafe fn copy_page(&mut self, src: *const u8, dst: *mut u8, text: bool) {
        // SAFETY: the caller's contract — two distinct mapped pages.
        unsafe { core::ptr::copy_nonoverlapping(src, dst, PAGE_SIZE_4K) };
        if text {
            vm::clean_dcache_to_pou(dst as usize, PAGE_SIZE_4K);
            self.pending = true;
        }
    }

    /// Discharge the sync now (before the copied text can run).
    pub(crate) fn finish(mut self) {
        self.sync();
    }

    fn sync(&mut self) {
        if self.pending {
            vm::invalidate_icache_all();
            #[cfg(test)]
            tests::count_sync();
            self.pending = false;
        }
    }
}

impl Drop for TextCopy {
    fn drop(&mut self) {
        self.sync();
    }
}

/// The boot-time guard (see the module doc).  Returns
/// `(fresh, control_stale, rounds)`: `fresh` must equal `rounds`;
/// `control_stale` is the venue's sensitivity (informational).
#[cfg(all(not(test), dev_exec, any(not(dev_churn), dev_bootchain)))]
pub(crate) fn selftest() -> (u32, u32, u32) {
    use crate::kmem::physaddr_as_ptr_mut_offset_from_kzero;
    use crate::pagealloc;
    use crate::param::KZERO;
    use crate::vm::{
        Entry, PageSize, PhysPageAllocator, RootPageTableType, VaMapping, VmTraitImpl,
    };
    use port::mem::{PhysAddr, PhysRange};
    use port::println;

    const ROUNDS: u32 = 64;
    fn map_at(pa: PhysAddr, va: usize, entry: Entry) {
        let mut physalloc = PhysPageAllocator {};
        let mut vmt = VmTraitImpl {};
        vm::kernel_pagetable()
            .map_phys_range(
                &mut physalloc,
                &mut vmt,
                "icache-selftest",
                &PhysRange::with_pa_len(pa, PAGE_SIZE_4K),
                VaMapping::Addr(va),
                entry,
                PageSize::Page4K,
                RootPageTableType::Kernel,
            )
            .expect("icache selftest: kernel map failed");
    }
    /// `movz w0, #imm; ret` at `p` (imm < 4096).
    unsafe fn emit(p: *mut u8, imm: u32) {
        let w = p as *mut u32;
        // SAFETY: p is inside a mapped writable page (the caller).
        unsafe {
            core::ptr::write_volatile(w, 0x5280_0000 | (imm << 5));
            core::ptr::write_volatile(w.add(1), 0xd65f_03c0);
        }
    }
    fn call(va: usize) -> u32 {
        // SAFETY: va names a `movz w0; ret` pair in a page mapped
        // kernel-executable (ro_kernel_text) by selftest itself.
        let f: extern "C" fn() -> u32 = unsafe { core::mem::transmute(va) };
        f()
    }

    // `code` is the page under test (mapped executable at `x` and r/w at
    // its KZERO alias); `src` plays the parent page rfork copies from.
    let code = pagealloc::allocate_physpage().expect("icache selftest: page");
    let src = pagealloc::allocate_physpage().expect("icache selftest: page");
    for pa in [code, src] {
        map_at(pa, pa.addr() as usize + KZERO, Entry::rw_kernel_data());
    }
    let x = match vm::next_free_device_page4k() {
        VaMapping::Addr(a) => a,
        VaMapping::Offset(_) => unreachable!("device VA arena hands out addresses"),
    };
    map_at(code, x, Entry::ro_kernel_text());
    let kcode = physaddr_as_ptr_mut_offset_from_kzero::<u8>(code);
    let ksrc = physaddr_as_ptr_mut_offset_from_kzero::<u8>(src);

    let (mut fresh, mut stale) = (0u32, 0u32);
    for arm in 0..2 {
        for r in 0..ROUNDS {
            let off = (r as usize % 32) * 128;
            let (old, new) = (0x100 + r, 0x800 + r);
            // The page's previous life: `old` code, executed hot at x.
            // SAFETY: kcode/ksrc are the pages' KZERO aliases (mapped
            // r/w above); off + 8 <= 4096.
            unsafe { emit(kcode.add(off), old) };
            vm::clean_dcache_to_pou(kcode as usize + off, 8);
            vm::invalidate_icache_all();
            for _ in 0..16 {
                call(x + off);
            }
            // The parent page carries `new` at the same offset.
            // SAFETY: ksrc is the src page's KZERO alias (mapped r/w
            // above); off + 8 <= 4096, as for the kcode emit above.
            unsafe { emit(ksrc.add(off), new) };
            if arm == 0 {
                // rfork's path: the shared helper, then finish.
                let mut tc = TextCopy::new();
                // SAFETY: two distinct pages, both KZERO-mapped r/w.
                unsafe { tc.copy_page(ksrc, kcode, true) };
                tc.finish();
                if call(x + off) == new {
                    fresh += 1;
                }
            } else {
                // Control: the same copy with NO maintenance.
                // SAFETY: as above.
                unsafe { core::ptr::copy_nonoverlapping(ksrc, kcode, PAGE_SIZE_4K) };
                // SAFETY: `isb` is a context-synchronization event with
                // no memory operands and no clobbers; executing it at
                // EL1 is always architecturally valid.
                unsafe { core::arch::asm!("isb") };
                if call(x + off) == old {
                    stale += 1;
                }
                vm::invalidate_icache_all();
            }
        }
    }
    // Tear down: unmap the executable alias (map_to's per-write tlbi
    // covers it), drop any lines it left, then free both pages.  The
    // KZERO aliases stay, per the kernel-alias convention.
    map_at(code, x, Entry::empty());
    vm::invalidate_icache_all();
    for pa in [code, src] {
        if let Err(err) = pagealloc::deallocate_physpage(pa) {
            println!("icache selftest: deallocate {:#x}: {err:?}", pa.addr());
        }
    }
    (fresh, stale, ROUNDS)
}

#[cfg(test)]
mod tests {
    use super::TextCopy;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static SYNCS: AtomicUsize = AtomicUsize::new(0);

    pub(super) fn count_sync() {
        SYNCS.fetch_add(1, Ordering::SeqCst);
    }

    /// The copy helper owes exactly one I-cache sync per batch that copied
    /// text — discharged by finish, or by drop on an early return — and
    /// none for a data-only batch; the bytes are copied either way.  (The
    /// hardware half — that the sync makes the new code visible — is the
    /// boot-time selftest's job; a host cannot observe an I-cache.)
    /// One test function: SYNCS is process-global.
    #[test]
    fn text_copies_owe_exactly_one_icache_sync() {
        let src = [0xa5u8; 4096];
        let mut dst = [0u8; 4096];

        let before = SYNCS.load(Ordering::SeqCst);
        let mut tc = TextCopy::new();
        unsafe { tc.copy_page(src.as_ptr(), dst.as_mut_ptr(), false) };
        tc.finish();
        assert_eq!(dst, src, "data page copied");
        assert_eq!(SYNCS.load(Ordering::SeqCst), before, "a data-only batch owes no sync");

        dst = [0u8; 4096];
        let mut tc = TextCopy::new();
        unsafe { tc.copy_page(src.as_ptr(), dst.as_mut_ptr(), false) };
        unsafe { tc.copy_page(src.as_ptr(), dst.as_mut_ptr(), true) };
        unsafe { tc.copy_page(src.as_ptr(), dst.as_mut_ptr(), true) };
        tc.finish();
        assert_eq!(dst, src, "text page copied");
        assert_eq!(SYNCS.load(Ordering::SeqCst), before + 1, "one sync per text batch, at finish");

        {
            let mut tc = TextCopy::new();
            unsafe { tc.copy_page(src.as_ptr(), dst.as_mut_ptr(), true) };
            // early return: no finish
        }
        assert_eq!(SYNCS.load(Ordering::SeqCst), before + 2, "drop discharges a pending sync");
    }
}
