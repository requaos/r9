//! x86_64/src/directmap.rs — the kernel's map of the RAM above the
//! first GiB.
//!
//! The boot assembly maps exactly one gibibyte of RAM, at KZERO (l.S's
//! bootkernpt3: a single 1 GiB page), and the kernel's only way to
//! touch physical memory was `phys_ptr_mut`, which refused any PA above
//! that window.  A machine with more RAM was therefore unusable: the
//! frame allocator pre-zeroes frames it could not reach, and measured
//! on the KVM host an 8 GiB guest panicked before the console existed
//! (.planning/SCALE-AUDIT.md, M2).  KZERO cannot simply be widened — it
//! sits 2 GiB below the top of the 64-bit space.
//!
//! This module adds a linear map of the REST of RAM, `DIRECT_BASE + pa`,
//! into PML4 slots 256..=509, which the boot tables leave free: 127 TiB
//! of virtual space.  It needs no frame allocation.  Its table pages
//! come from a reservation in low RAM, which the first GiB's KZERO page
//! already reaches, and the entries go straight into the boot PML4 — so
//! the map is live on the BSP at once (that table is in CR3), the APs
//! inherit it (the trampoline loads that table), and every per-process
//! PML4 copies the kernel slots from it (proc.rs's `copy_kernel_half`).
//! Nothing changes it afterwards: no shootdown, no per-process pages.
//!
//! Granularity: an aligned gibibyte is one 1 GiB page; the head and
//! tail of a RAM span are 2 MiB pages; whatever is left at a span's
//! edges below 2 MiB is NOT mapped and is reported back, so the frame
//! allocator never manages a frame the kernel cannot reach.
//!
//! Nothing executes through this map: its leaves are no-execute.
//!
//! [`covers`] is the bound `phys_ptr_mut` checks.  The kernel image and
//! the first GiB keep their KZERO addresses unchanged, so a 1 GiB guest
//! builds no map at all and runs exactly as before.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use alloc::vec::Vec;

use port::mem::{PAGE_SIZE_4K, PhysRange};

use crate::proc::KZERO_MAP_LIMIT;

/// The first kernel-half PML4 slot the boot tables leave free (510 is
/// the Mach window, 511 is KZERO; 0..=255 are the user half).
const FIRST_SLOT: usize = 256;
/// The free slots: 256..=509.
const SLOTS: usize = 254;

/// The RAM spans the map can hold: firmware's available ranges above the
/// first GiB, merged, so never more of them than the map keeps
/// (mb2::MAX_RANGES).
const MAX_SPANS: usize = crate::platform::q35::mb2::MAX_RANGES;

const GIB: u64 = 1 << 30;
const MIB2: u64 = 1 << 21;

/// Present, writable, no-execute — the leaf bits this map uses (the
/// SDM's names; the same bits as proc.rs's PTE_P, PTE_RW and PTE_NX).
const PTE_P: u64 = 1 << 0;
const PTE_RW: u64 = 1 << 1;
const PTE_NX: u64 = 1 << 63;

/// The page-size bit: 1 GiB in a PDPT entry, 2 MiB in a PD entry.
const PTE_PS: u64 = 1 << 7;
/// A table-pointer entry.
const INTER: u64 = PTE_P | PTE_RW;
/// A RAM leaf: supervisor, writable, no-execute.
const LEAF: u64 = PTE_P | PTE_RW | PTE_PS | PTE_NX;
/// The address mask of a table-pointer entry.
const PA_MASK: u64 = 0x000f_ffff_ffff_f000;

/// The map's virtual base — slot 256, sign-extended: the lowest
/// canonical kernel address.  A physical address above the first GiB
/// is reached at `DIRECT_BASE + pa`.
pub const DIRECT_BASE: u64 = 0xffff_8000_0000_0000;

/// The mapped spans' starts and ends, ascending and disjoint.  Written
/// once by [`publish`] (boot, before any AP starts) and read-only
/// afterwards; atomics are the unsafe-free way to share a static.
static SPAN_START: [AtomicU64; MAX_SPANS] = [const { AtomicU64::new(0) }; MAX_SPANS];
static SPAN_END: [AtomicU64; MAX_SPANS] = [const { AtomicU64::new(0) }; MAX_SPANS];
static SPANS: AtomicUsize = AtomicUsize::new(0);

/// What the boot built, for the `directmap:` boot line.
static MAPPED_BYTES: AtomicU64 = AtomicU64::new(0);
static TABLE_PAGES: AtomicUsize = AtomicUsize::new(0);
static BEYOND_BYTES: AtomicU64 = AtomicU64::new(0);

/// The map as boot built it: `(spans, mapped bytes, table pages, the RAM
/// withheld because the map cannot reach it)`.  All zero on a machine
/// with no RAM above the first gibibyte.
pub fn info() -> (usize, u64, usize, u64) {
    (
        SPANS.load(Ordering::Acquire),
        MAPPED_BYTES.load(Ordering::Acquire),
        TABLE_PAGES.load(Ordering::Acquire),
        BEYOND_BYTES.load(Ordering::Acquire),
    )
}

/// The virtual address of physical address `pa` above the first GiB.
pub fn va(pa: u64) -> usize {
    debug_assert!(covers(pa), "directmap: PA {pa:#x} is not mapped");
    (DIRECT_BASE + pa) as usize
}

/// Whether `pa` is reached through this map: RAM above KZERO's window,
/// inside one of the mapped spans.
pub fn covers(pa: u64) -> bool {
    if pa < KZERO_MAP_LIMIT {
        return false;
    }
    // Binary search for the last span that starts at or below `pa`.
    let mut lo = 0;
    let mut hi = SPANS.load(Ordering::Acquire);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if SPAN_START[mid].load(Ordering::Relaxed) <= pa {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo > 0 && pa < SPAN_END[lo - 1].load(Ordering::Relaxed)
}

/// Publish the mapped spans: ascending, disjoint, at most [`MAX_SPANS`].
/// A refusal leaves the spans published so far in place.
fn publish(spans: &[(u64, u64)]) -> Result<(), &'static str> {
    if spans.len() > MAX_SPANS {
        return Err("directmap: too many RAM spans for the kernel's map");
    }
    for (i, &(start, end)) in spans.iter().enumerate() {
        SPAN_START[i].store(start, Ordering::Relaxed);
        SPAN_END[i].store(end, Ordering::Relaxed);
    }
    SPANS.store(spans.len(), Ordering::Release);
    Ok(())
}

/// One page-table page's access.  The kernel reaches table pages
/// through the KZERO alias (they live in the first GiB); host tests
/// use real memory.
trait Tables {
    fn get(&self, table_pa: u64, idx: usize) -> u64;
    fn set(&mut self, table_pa: u64, idx: usize, val: u64);
}

/// What [`build`] produced.
#[derive(Debug, Default, PartialEq)]
pub struct Built {
    /// The 2 MiB-granular spans that are mapped, `[start, end)`,
    /// ascending and disjoint.
    pub spans: Vec<(u64, u64)>,
    /// RAM the map does not reach: the sub-2 MiB edges of the RAM
    /// spans above the first GiB.
    pub unmapped: Vec<PhysRange>,
    /// Bytes of RAM mapped.
    pub mapped_bytes: u64,
    /// Page-table pages taken.
    pub table_pages: usize,
}

/// An upper bound on the table pages [`build`] can need for `nranges`
/// RAM ranges ending at or below `top_pa`: one PDPT per 512 GiB of
/// physical space, and at most one PD per range edge.
pub fn table_pages_bound(nranges: usize, top_pa: u64) -> usize {
    let pdpts = (top_pa / GIB / 512) as usize + 1;
    2 * nranges + pdpts
}

/// Map the RAM above the first GiB into the PML4 at `pml4_pa`.  `ram`
/// holds `[start, end)` spans in any order, possibly overlapping or
/// adjacent (firmware's memory map).  `new_page` hands out zeroed
/// page-table pages by PA; None ends the build with an error.
fn build<T: Tables>(
    pml4_pa: u64,
    ram: &[(u64, u64)],
    tables: &mut T,
    mut new_page: impl FnMut() -> Option<u64>,
) -> Result<Built, &'static str> {
    // The parts above KZERO's window, merged into disjoint spans.
    let mut parts: Vec<(u64, u64)> = ram
        .iter()
        .filter_map(|&(start, end)| {
            let lo = start.max(KZERO_MAP_LIMIT);
            (end > lo).then_some((lo, end))
        })
        .collect();
    parts.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for (start, end) in parts {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }

    let mut built = Built::default();
    for &(lo, end) in &merged {
        // The 2 MiB-aligned middle is mapped; whatever the edges leave
        // is reported, so no frame is ever managed but unreachable (a
        // span with no aligned middle reports whole).
        let lo2 = lo.div_ceil(MIB2) * MIB2;
        let hi2 = end / MIB2 * MIB2;
        let (map_lo, map_hi) = (lo2.min(end), hi2.max(lo2.min(end)));
        if lo < map_lo {
            built.unmapped.push(PhysRange::with_end(lo, map_lo));
        }
        if map_hi < end {
            built.unmapped.push(PhysRange::with_end(map_hi, end));
        }
        if map_lo >= map_hi {
            continue; // nothing 2 MiB-aligned to map
        }
        let mut pa = map_lo;
        while pa < map_hi {
            let pdpt = pdpt_for(pml4_pa, pa, tables, &mut new_page, &mut built)?;
            if pa.is_multiple_of(GIB) && pa + GIB <= map_hi {
                tables.set(pdpt, ((pa >> 30) & 0x1ff) as usize, pa | LEAF);
                pa += GIB;
            } else {
                let pd = pd_for(pdpt, pa, tables, &mut new_page, &mut built)?;
                tables.set(pd, ((pa >> 21) & 0x1ff) as usize, pa | LEAF);
                pa += MIB2;
            }
        }
        built.spans.push((map_lo, map_hi));
        built.mapped_bytes += map_hi - map_lo;
    }
    Ok(built)
}

/// The PDPT page of `pa`'s PML4 slot, created on first use.
fn pdpt_for<T: Tables>(
    pml4_pa: u64,
    pa: u64,
    tables: &mut T,
    new_page: &mut impl FnMut() -> Option<u64>,
    built: &mut Built,
) -> Result<u64, &'static str> {
    let slot = FIRST_SLOT + (pa / GIB / 512) as usize;
    if slot >= FIRST_SLOT + SLOTS {
        return Err("directmap: RAM lies beyond the kernel's map slots");
    }
    let e = tables.get(pml4_pa, slot);
    if e & PTE_P != 0 {
        return Ok(e & PA_MASK);
    }
    let page = new_page().ok_or("directmap: out of page-table pages")?;
    built.table_pages += 1;
    tables.set(pml4_pa, slot, page | INTER);
    Ok(page)
}

/// The PD page of `pa`'s gibibyte inside `pdpt_pa`, created on first
/// use (only a gibibyte that is not one 1 GiB page needs one).
fn pd_for<T: Tables>(
    pdpt_pa: u64,
    pa: u64,
    tables: &mut T,
    new_page: &mut impl FnMut() -> Option<u64>,
    built: &mut Built,
) -> Result<u64, &'static str> {
    let idx = ((pa >> 30) & 0x1ff) as usize;
    let e = tables.get(pdpt_pa, idx);
    if e & PTE_P != 0 {
        debug_assert!(e & PTE_PS == 0, "directmap: a partial gibibyte was already a 1 GiB page");
        return Ok(e & PA_MASK);
    }
    let page = new_page().ok_or("directmap: out of page-table pages")?;
    built.table_pages += 1;
    tables.set(pdpt_pa, idx, page | INTER);
    Ok(page)
}

/// The kernel alias's place in the boot tables (l.S): PML4 slot 511 names
/// a PDPT whose entry 510 is ONE 1 GiB page — the first GiB of RAM at
/// KZERO.
const KZERO_SLOT: usize = 511;
const KZERO_PDPT_ENTRY: usize = 510;

/// What [`split_with_guards`] produced.
#[derive(Debug, Default, PartialEq)]
pub struct Split {
    /// Page-table pages taken: one PD for the gibibyte and one PT per
    /// 2 MiB of the guarded range.
    pub table_pages: usize,
    /// Guard pages left unmapped: one per `slot_bytes` of the range.
    pub guards: usize,
}

/// The table pages [`split_with_guards`] takes for `len` bytes of guarded
/// range: one PD for the gibibyte and one PT per 2 MiB of the range.
pub fn guard_table_pages(len: u64) -> usize {
    1 + (len / MIB2) as usize
}

/// Split the kernel alias's 1 GiB page so that every `slot_bytes`-spaced
/// stack in `[base, base + len)` has an unmapped page below it.  The
/// gibibyte becomes 512 entries of 2 MiB, and each 2 MiB that holds part
/// of the range becomes 512 entries of 4 KiB with the first page of each
/// slot absent.  Every other address maps exactly as before, with the
/// flags the 1 GiB page carried.
fn split_with_guards<T: Tables>(
    pml4_pa: u64,
    base: u64,
    len: u64,
    slot_bytes: u64,
    tables: &mut T,
    mut new_page: impl FnMut() -> Option<u64>,
) -> Result<Split, &'static str> {
    if !base.is_multiple_of(MIB2) || !len.is_multiple_of(MIB2) || !slot_bytes.is_multiple_of(PAGE_SIZE_4K as u64) {
        return Err("directmap: the guarded range is not 2 MiB pages of 4 KiB-multiple slots");
    }
    let pdpt = tables.get(pml4_pa, KZERO_SLOT);
    if pdpt & PTE_P == 0 || pdpt & PTE_PS != 0 {
        return Err("directmap: PML4 slot 511 does not name the kernel alias's PDPT");
    }
    let leaf = tables.get(pdpt & PA_MASK, KZERO_PDPT_ENTRY);
    if leaf & PTE_P == 0 || leaf & PTE_PS == 0 {
        return Err("directmap: the kernel alias is not one 1 GiB page");
    }
    // A large page carries its PAT bit at bit 12 and a 4 KiB page at bit
    // 7: the split cannot carry one across.
    if leaf & (1 << 12) != 0 {
        return Err("directmap: the kernel alias sets PAT; the split does not carry it");
    }
    // Everything but the page-size bit: present, writable, the cache
    // bits, the accessed and dirty bits, and no-execute.
    let flags = leaf & 0x8000_0000_0000_0fff & !PTE_PS;
    let mut split = Split::default();
    let pd = new_page().ok_or("directmap: out of page-table pages")?;
    split.table_pages += 1;
    for i in 0..512 {
        let pa2 = i as u64 * MIB2;
        if pa2 + MIB2 <= base || pa2 >= base + len {
            tables.set(pd, i, pa2 | flags | PTE_PS);
            continue;
        }
        let pt = new_page().ok_or("directmap: out of page-table pages")?;
        split.table_pages += 1;
        for j in 0..512u64 {
            let pa = pa2 + j * PAGE_SIZE_4K as u64;
            if pa >= base && pa < base + len && (pa - base).is_multiple_of(slot_bytes) {
                split.guards += 1; // left absent: the guard page
            } else {
                tables.set(pt, j as usize, pa | flags);
            }
        }
        tables.set(pd, i, pt | INTER);
    }
    // One store publishes the whole gibibyte's new shape.
    tables.set(pdpt & PA_MASK, KZERO_PDPT_ENTRY, pd | INTER);
    Ok(split)
}

#[cfg(not(test))]
mod live {
    use core::ptr;

    use super::*;
    use crate::proc::{phys_ptr_mut, set_table_entry, table_entry};

    /// The kernel's tables: proc.rs's KZERO-alias entry accessors.
    struct AliasTables;

    impl Tables for AliasTables {
        fn get(&self, table_pa: u64, idx: usize) -> u64 {
            table_entry(table_pa, idx)
        }
        fn set(&mut self, table_pa: u64, idx: usize, val: u64) {
            set_table_entry(table_pa, idx, val)
        }
    }

    /// Build the map into the boot PML4 at `pml4_pa` and prove it.
    /// `pages` is `[start, end)`: a run of RAM in the first GiB for the
    /// table pages, at least [`super::table_pages_bound`] pages long.
    /// Returns the RAM the map does not reach (for the page allocator to
    /// withhold) and the pages actually taken.
    pub fn init(
        pml4_pa: u64,
        ram: &[(u64, u64)],
        pages: (u64, u64),
    ) -> Result<(Vec<PhysRange>, usize), &'static str> {
        let (mut next, limit) = pages;
        let mut tables = AliasTables;
        let built = build(pml4_pa, ram, &mut tables, || {
            if next >= limit {
                return None;
            }
            let pa = next;
            next += PAGE_SIZE_4K as u64;
            // SAFETY: `pa` lies in the caller's table-page reservation,
            // inside the first GiB; a whole page is written.
            unsafe { ptr::write_bytes(phys_ptr_mut(pa), 0, PAGE_SIZE_4K) };
            Some(pa)
        })?;
        publish(&built.spans)?;
        MAPPED_BYTES.store(built.mapped_bytes, Ordering::Release);
        TABLE_PAGES.store(built.table_pages, Ordering::Release);
        let beyond: u64 = built.unmapped.iter().map(|r| r.end().addr() - r.start().addr()).sum();
        BEYOND_BYTES.store(beyond, Ordering::Release);
        // The proof: the first and the last frame of every span, through
        // the map itself (the boot CPU runs on the table just edited).
        for &(start, end) in &built.spans {
            for pa in [start, end - PAGE_SIZE_4K as u64] {
                // SAFETY: `pa` is a frame of a mapped span; an aligned
                // qword is written and read back.
                unsafe {
                    let p = phys_ptr_mut(pa).cast::<u64>();
                    p.write_volatile(0x5A5A_2026_1101);
                    assert_eq!(p.read_volatile(), 0x5A5A_2026_1101, "PA {pa:#x} is not reachable through the map");
                }
            }
        }
        Ok((built.unmapped, built.table_pages))
    }

    /// Split the kernel alias around the kernel-stack arena
    /// `[base, base + len)`, leaving a guard page below every
    /// `slot_bytes` stack ([`super::split_with_guards`]), with the table
    /// pages taken from `pages` `[start, end)` — a run in the first GiB,
    /// at least [`super::guard_table_pages`] pages long.
    pub fn arm_guards(
        base: u64,
        len: u64,
        slot_bytes: u64,
        pages: (u64, u64),
    ) -> Result<Split, &'static str> {
        let (mut next, limit) = pages;
        let mut tables = AliasTables;
        let split = split_with_guards(crate::proc::boot_pml4_pa(), base, len, slot_bytes, &mut tables, || {
            if next >= limit {
                return None;
            }
            let pa = next;
            next += PAGE_SIZE_4K as u64;
            // SAFETY: `pa` lies in the caller's table-page reservation,
            // inside the first GiB; a whole page is written.
            unsafe { ptr::write_bytes(phys_ptr_mut(pa), 0, PAGE_SIZE_4K) };
            Some(pa)
        })?;
        // The boot CPU may still translate through the gibibyte page.  A
        // CR3 reload drops every translation (this alias sets no global
        // bit), and each guard is then invalidated by name.
        crate::proc::switch_cr3(crate::proc::read_cr3());
        for i in 0..split.guards as u64 {
            // SAFETY: invlpg reads no memory and names no register.
            unsafe {
                core::arch::asm!(
                    "invlpg ({})",
                    in(reg) (crate::proc::KZERO + base + i * slot_bytes) as usize,
                    options(att_syntax, nostack, preserves_flags)
                );
            }
        }
        Ok(split)
    }
}

#[cfg(not(test))]
pub(crate) use live::{arm_guards, init};

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    const G: u64 = 1 << 30;
    const M: u64 = 1 << 20;
    const PML4: u64 = 0x1000;

    /// Page tables in host memory, keyed by a made-up "PA".
    type Pages = Rc<RefCell<BTreeMap<u64, Vec<u64>>>>;

    /// The tables a build writes.
    struct Fake {
        pages: Pages,
    }

    /// The host's supply of zeroed table pages, on a budget — a
    /// separate object so the build can hold the tables while its page
    /// closure holds this.
    struct Source {
        pages: Pages,
        next_pa: u64,
        budget: usize,
    }

    impl Fake {
        /// The tables (a zeroed PML4) and a supply of `budget` pages.
        fn with(budget: usize) -> (Fake, Source) {
            let pages: Pages = Rc::new(RefCell::new(BTreeMap::new()));
            pages.borrow_mut().insert(PML4, vec![0; 512]);
            (Fake { pages: pages.clone() }, Source { pages, next_pa: 0x1000_0000, budget })
        }
        fn entry(&self, table: u64, idx: usize) -> u64 {
            self.pages.borrow()[&table][idx]
        }
    }

    impl Source {
        fn page(&mut self) -> Option<u64> {
            if self.budget == 0 {
                return None;
            }
            self.budget -= 1;
            let pa = self.next_pa;
            self.next_pa += 0x1000;
            self.pages.borrow_mut().insert(pa, vec![0; 512]);
            Some(pa)
        }
    }

    impl Tables for Fake {
        fn get(&self, table_pa: u64, idx: usize) -> u64 {
            self.entry(table_pa, idx)
        }
        fn set(&mut self, table_pa: u64, idx: usize, val: u64) {
            self.pages.borrow_mut().get_mut(&table_pa).unwrap()[idx] = val;
        }
    }

    fn run(ram: &[(u64, u64)]) -> (Fake, Built) {
        let (mut f, mut src) = Fake::with(1 << 20);
        let built = build(PML4, ram, &mut f, || src.page()).expect("build");
        (f, built)
    }

    /// The PDPT page the map's PML4 slot 256 names.
    fn pdpt(f: &Fake) -> u64 {
        let e = f.entry(PML4, 256);
        assert_eq!(e & INTER, INTER, "slot 256 names a PDPT");
        e & PA_MASK
    }

    /// l.S's kernel alias in the fake tables: PML4 slot 511 names a
    /// PDPT whose entry 510 is one 1 GiB page for PA 0.
    fn kzero_alias(f: &mut Fake, src: &mut Source) -> u64 {
        let pdpt = src.page().expect("the alias's PDPT");
        f.set(PML4, 511, pdpt | INTER);
        f.set(pdpt, 510, PTE_PS | PTE_RW | PTE_P);
        pdpt
    }

    const K: u64 = 1 << 12;
    /// One kernel stack slot: eight pages (kstack.rs's SLOT_BYTES).
    const SLOT32K: u64 = 8 * K;

    #[test]
    fn the_split_leaves_a_guard_below_every_slot_and_maps_the_rest_as_before() {
        let (mut f, mut src) = Fake::with(1 << 20);
        let pdpt = kzero_alias(&mut f, &mut src);
        // a 16 MiB arena at 128 MiB
        let (base, len) = (128 * M, 16 * M);
        let split = split_with_guards(PML4, base, len, SLOT32K, &mut f, || src.page()).expect("split");
        assert_eq!(split, Split { table_pages: 1 + 8, guards: 512 });
        assert!(guard_table_pages(len) >= split.table_pages, "the bound covers the split");
        // slot 511 still names the same PDPT; its entry is a PD now
        let e = f.entry(pdpt, 510);
        assert_eq!(e & INTER, INTER, "the gibibyte leaf became a PD");
        assert_eq!(e & PTE_PS, 0);
        let pd = e & PA_MASK;
        for i in 0..512u64 {
            let e = f.entry(pd, i as usize);
            let pa2 = i * M * 2;
            if pa2 + 2 * M <= base || pa2 >= base + len {
                // outside the arena: a 2 MiB page with the leaf's flags
                assert_eq!(e, pa2 | PTE_PS | PTE_RW | PTE_P, "PD[{i}]");
            } else {
                assert_eq!(e & INTER, INTER, "PD[{i}] names a PT");
                assert_eq!(e & PTE_PS, 0);
                let pt = e & PA_MASK;
                for j in 0..512u64 {
                    let pa = pa2 + j * K;
                    let e = f.entry(pt, j as usize);
                    if pa >= base && pa < base + len && (pa - base) % SLOT32K == 0 {
                        assert_eq!(e, 0, "the guard page at PA {pa:#x} is absent");
                    } else {
                        assert_eq!(e, pa | PTE_RW | PTE_P, "PT[{i}][{j}]");
                    }
                }
            }
        }
    }

    #[test]
    fn the_split_refuses_what_it_cannot_carry() {
        // misaligned range
        let (mut f, mut src) = Fake::with(64);
        kzero_alias(&mut f, &mut src);
        assert!(split_with_guards(PML4, 128 * M + M, 16 * M, SLOT32K, &mut f, || src.page()).is_err());
        // a slot that is not a page multiple
        let (mut f, mut src) = Fake::with(64);
        kzero_alias(&mut f, &mut src);
        assert!(split_with_guards(PML4, 128 * M, 16 * M, SLOT32K - 1, &mut f, || src.page()).is_err());
        // slot 511 not a PDPT
        let (mut f, mut src) = Fake::with(64);
        f.set(PML4, 511, PTE_PS | PTE_RW | PTE_P);
        assert!(split_with_guards(PML4, 128 * M, 16 * M, SLOT32K, &mut f, || src.page()).is_err());
        // the alias is not one 1 GiB page
        let (mut f, mut src) = Fake::with(64);
        let pdpt = src.page().unwrap();
        f.set(PML4, 511, pdpt | INTER);
        assert!(split_with_guards(PML4, 128 * M, 16 * M, SLOT32K, &mut f, || src.page()).is_err());
        // out of table pages: nothing is published
        let (mut f, mut src) = Fake::with(1);
        let pdpt = kzero_alias(&mut f, &mut src);
        assert!(split_with_guards(PML4, 128 * M, 16 * M, SLOT32K, &mut f, || src.page()).is_err());
        assert_eq!(f.entry(pdpt, 510), PTE_PS | PTE_RW | PTE_P, "the gibibyte leaf still stands");
    }

    #[test]
    fn the_bound_is_one_pd_and_one_pt_per_2mib() {
        assert_eq!(guard_table_pages(16 * M), 9);
        assert_eq!(guard_table_pages(64 * M), 33);
    }

    #[test]
    fn aligned_gibibytes_are_one_gib_pages() {
        // 4 GiB .. 6 GiB, aligned.
        let (f, b) = run(&[(4 * G, 6 * G)]);
        assert_eq!(b.spans, vec![(4 * G, 6 * G)]);
        assert!(b.unmapped.is_empty());
        assert_eq!(b.mapped_bytes, 2 * G);
        assert_eq!(b.table_pages, 1, "one PDPT for the whole span");
        let t = pdpt(&f);
        assert_eq!(f.entry(t, 4), 4 * G | LEAF);
        assert_eq!(f.entry(t, 5), 5 * G | LEAF);
        assert_eq!(f.entry(t, 3), 0);
        assert_eq!(f.entry(t, 6), 0);
        assert_eq!(f.entry(PML4, 257), 0, "all of it lies in one slot");
        assert_eq!(f.entry(t, 4) & PTE_NX, PTE_NX, "nothing executes through the map");
        assert_eq!(f.entry(PML4, 256) & PTE_NX, 0, "the pointer entry carries no NX");
    }

    #[test]
    fn an_8g_guest_takes_two_table_pages() {
        // QEMU's split at 8 GiB: 2 GiB of RAM below 4 GiB, then 4 GiB ..
        // 8 GiB.  The low range ends short of a whole gibibyte.
        let (f, b) = run(&[(0x100000, 2 * G - 0x21000), (4 * G, 8 * G)]);
        assert_eq!(
            b.spans,
            vec![(G, 2 * G - 2 * M), (4 * G, 8 * G)],
            "the low range is trimmed to its 2 MiB-aligned middle"
        );
        assert_eq!(b.unmapped, vec![PhysRange::with_end(2 * G - 2 * M, 2 * G - 0x21000)]);
        assert_eq!(b.mapped_bytes, G - 2 * M + 4 * G);
        let t = pdpt(&f);
        // The only partial gibibyte: 511 pages of 2 MiB under one PD.
        let e = f.entry(t, 1);
        assert_eq!(e & (INTER | PTE_PS), INTER, "giB 1 is a PD pointer");
        let pd = e & PA_MASK;
        assert_eq!(f.entry(pd, 0), G | LEAF);
        // 511 pages of 2 MiB: the last starts 4 MiB below the range's
        // 2 MiB-aligned end, and index 511 is past it.
        assert_eq!(f.entry(pd, 510), (2 * G - 4 * M) | LEAF);
        assert_eq!(f.entry(pd, 511), 0);
        // The 2 GiB hole is not RAM: nothing there.
        assert_eq!(f.entry(t, 2), 0);
        assert_eq!(f.entry(t, 3), 0);
        for g in 4..8 {
            assert_eq!(f.entry(t, g as usize), g * G | LEAF, "giB {g}");
        }
        assert_eq!(f.entry(t, 8), 0);
        assert_eq!(b.table_pages, 2, "one PDPT and one PD");
    }

    #[test]
    fn range_edges_are_2mib_pages_and_the_rest_is_reported() {
        // 1 MiB .. 4 GiB - 132 KiB, then 4 GiB .. 10 GiB.
        let (f, b) = run(&[(0x100000, 4 * G - 0x21000), (4 * G, 10 * G)]);
        assert_eq!(
            b.spans,
            vec![(G, 4 * G - 2 * M), (4 * G, 10 * G)],
            "the top range is trimmed to its 2 MiB-aligned middle"
        );
        assert_eq!(b.unmapped, vec![PhysRange::with_end(4 * G - 2 * M, 4 * G - 0x21000)]);
        assert_eq!(b.mapped_bytes, 3 * G - 2 * M + 6 * G);
        let t = pdpt(&f);
        // GiB 1 and 2: whole gibibytes, 1 GiB pages.
        for g in 1..3 {
            assert_eq!(f.entry(t, g as usize), g * G | LEAF);
        }
        // GiB 3 falls 2 MiB short of its end: a PD of 511 pages of 2 MiB.
        let e = f.entry(t, 3);
        assert_eq!(e & (INTER | PTE_PS), INTER, "giB 3 is a PD pointer");
        let pd = e & PA_MASK;
        assert_eq!(f.entry(pd, 0), 3 * G | LEAF);
        assert_eq!(
            f.entry(pd, 510),
            (4 * G - 4 * M) | LEAF,
            "the last whole 2 MiB of the range starts 4 MiB below its end"
        );
        assert_eq!(f.entry(pd, 511), 0, "the last 2 MiB is not mapped");
        // 4 GiB .. 10 GiB again whole gibibytes, in the same PDPT.
        for g in 4..10 {
            assert_eq!(f.entry(t, g as usize), g * G | LEAF);
        }
        assert_eq!(f.entry(t, 10), 0);
        assert_eq!(b.table_pages, 2, "one PDPT and one PD");
    }

    #[test]
    fn overlapping_and_adjacent_ranges_merge() {
        let (_, b) = run(&[(4 * G, 5 * G), (9 * G, 10 * G), (0x1_2000_0000, 0x1_6000_0000), (0x1_2000_0000, 0x1_2400_0000)]);
        // 4 GiB..5 GiB and 4.5 GiB..5.5 GiB merge; 9 GiB..10 GiB stands.
        assert_eq!(b.spans, vec![(4 * G, 0x1_6000_0000), (9 * G, 10 * G)]);
        assert!(b.unmapped.is_empty());
    }

    #[test]
    fn a_part_under_2mib_is_left_unmapped_whole() {
        // 24 KiB just above 4 GiB.
        let (f, b) = run(&[(4 * G + 0x10000, 4 * G + 0x28000)]);
        assert!(b.spans.is_empty());
        assert_eq!(b.unmapped, vec![PhysRange::with_end(4 * G + 0x10000, 4 * G + 0x28000)]);
        assert_eq!(b.table_pages, 0);
        assert_eq!(f.entry(PML4, 256), 0, "nothing was touched");
    }

    #[test]
    fn a_tiny_range_at_an_aligned_start_reports_whole() {
        // 4 KiB at an aligned start: below the map's granularity, so
        // it must come back as unreachable - never silently managed.
        let (f, b) = run(&[(G, G + 0x1000)]);
        assert!(b.spans.is_empty());
        assert_eq!(b.unmapped, vec![PhysRange::with_end(G, G + 0x1000)]);
        assert_eq!(b.table_pages, 0);
        assert_eq!(f.entry(PML4, 256), 0);
    }

    #[test]
    fn a_1g_guest_builds_no_map_at_all() {
        // The gate guest's map.
        let (f, b) = run(&[(0, 0x9fc00), (0x100000, 0x3ffdf000)]);
        assert!(b.spans.is_empty());
        assert!(b.unmapped.is_empty());
        assert_eq!(b.table_pages, 0);
        assert_eq!(f.entry(PML4, 256), 0);
    }

    #[test]
    fn the_second_slot_starts_at_512_gib() {
        let (f, b) = run(&[(512 * G, 512 * G + G)]);
        assert_eq!(b.spans, vec![(512 * G, 512 * G + G)]);
        assert_eq!(b.table_pages, 1);
        assert_eq!(f.entry(PML4, 256), 0);
        assert_eq!(f.entry(PML4, 257) & INTER, INTER);
    }

    #[test]
    fn ram_past_the_free_slots_is_refused() {
        // 254 slots of 512 GiB: the map ends at 127 TiB.
        let top = SLOTS as u64 * 512 * G;
        let (mut f, mut src) = Fake::with(64);
        assert!(build(PML4, &[(top, top + G)], &mut f, || src.page()).is_err());
        assert!(build(PML4, &[(top - G, top)], &mut f, || src.page()).is_ok(), "the last gibibyte inside them is fine");
        assert_eq!(f.entry(PML4, 509) & INTER, INTER);
        assert_eq!(f.entry(PML4, 510), 0, "the Mach window's slot is never taken");
    }

    #[test]
    fn running_out_of_table_pages_is_an_error() {
        let (mut f, mut src) = Fake::with(0);
        assert!(build(PML4, &[(4 * G, 6 * G)], &mut f, || src.page()).is_err());
    }

    #[test]
    fn the_bound_covers_what_a_build_takes() {
        let ram = [(0x100000, 4 * G - 0x21000), (4 * G, 10 * G)];
        let (_, b) = run(&ram);
        assert!(b.table_pages <= table_pages_bound(ram.len(), 10 * G));
        // Three disjoint partial gibibytes at a 512 GiB step each.
        let ram = [
            (G + 2 * M, G + 4 * M),
            (512 * G + 2 * M, 512 * G + 4 * M),
            (1024 * G + 2 * M, 1024 * G + 4 * M),
        ];
        let (_, b) = run(&ram);
        assert_eq!(b.spans.len(), 3);
        assert_eq!(b.table_pages, 6, "three PDPTs and three PDs");
        assert!(b.table_pages <= table_pages_bound(ram.len(), 1024 * G + 4 * M));
        // A 128 GiB machine (QEMU: 2 GiB below 4 GiB, 126 above): whole
        // gibibytes, one PDPT.
        let ram = [(0x100000, 2 * G), (4 * G, 130 * G)];
        let (_, b) = run(&ram);
        assert_eq!(b.spans, vec![(G, 2 * G), (4 * G, 130 * G)]);
        assert_eq!(b.table_pages, 1);
        assert_eq!(b.mapped_bytes, 127 * G);
        assert!(b.table_pages <= table_pages_bound(ram.len(), 130 * G));
    }

    /// The only test that touches the published spans (they are a
    /// static, shared by every test in this binary).
    #[test]
    fn covers_exactly_the_published_spans() {
        let spans = [(G, 2 * G), (4 * G, 0x11_2345_6000), (600 * G, 601 * G)];
        publish(&spans).expect("three spans fit");
        // Below the first GiB is KZERO's, never the map's.
        assert!(!covers(0));
        assert!(!covers(KZERO_MAP_LIMIT - 1));
        // Inside each span, past each end, and in each gap.
        assert!(covers(G));
        assert!(covers(2 * G - 1));
        assert!(!covers(2 * G));
        assert!(!covers(4 * G - 1));
        assert!(covers(4 * G));
        assert!(covers(0x11_2345_6000 - 1));
        assert!(!covers(0x11_2345_6000));
        assert!(!covers(600 * G - 1));
        assert!(covers(601 * G - 1));
        assert!(!covers(601 * G));
        // A very high address is refused, not wrapped.
        assert!(!covers(0xffff_ffff_ffff_f000));
        // The virtual address is the base plus the physical one.
        assert_eq!(DIRECT_BASE, 0xffff_8000_0000_0000);
        assert_eq!(DIRECT_BASE >> 39 & 0x1ff, 256, "the map starts at PML4 slot 256");
        assert_eq!(va(4 * G), (DIRECT_BASE + 4 * G) as usize);
        assert!(va(4 * G) > 0xffff_8000_0000_0000usize, "and stays in the kernel half");
        // Too many spans is refused, and the published ones stand.
        assert!(publish(&[(0u64, 0u64); MAX_SPANS]).is_ok());
        assert!(publish(&[(0u64, 0u64); MAX_SPANS + 1]).is_err());
        assert_eq!(SPANS.load(Ordering::Acquire), MAX_SPANS);
        assert!(!covers(4 * G), "the empty spans replaced the old table");
    }
}
