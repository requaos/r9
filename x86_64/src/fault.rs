//! Ring-3 `#PF` demand pager (MEM-14, MEM-15, MEM-16; D-15, D-16, D-18).
//!
//! The x86_64 analog of `aarch64/src/fault.rs`, re-keyed to the page-fault
//! exception (vector 14) and its error code:
//!
//! - bit 0 `P`   — the fault was a protection violation (present page),
//! - bit 1 `W/R` — the access was a write,
//! - bit 2 `U/S` — the access came from ring 3,
//! - bit 4 `I/D` — the access was an instruction fetch.
//!
//! `trap.rs` routes ONLY ring-3 vector 14 here.  A kernel-origin fault
//! (any vector, or ring 0) keeps the dump-and-halt path: a nested page-in
//! from syscall context could self-deadlock on the page-cache/page-alloc
//! ranks, so a handler-side bad kernel pointer is a kernel bug, never a
//! demand-map request.
//!
//! Trap-context invariants (the aarch64 sibling's list, re-keyed):
//! - IRQs arrive masked and are NEVER unmasked here.
//! - No locks are held on entry; `PROC_TABLE` is never taken (the calling
//!   proc's CR3 is live on this CPU, and `current()` is the per-CPU
//!   `Mach.current_proc` pointer).
//! - The byte source is stateless-per-call — reentrant here.
//! - `PAGE_ALLOC` is taken only inside the allocator call and never while
//!   holding `PAGE_CACHE`.
//!
//! Text is SHARED through `palloc.hash` (MEM-16) and mapped strictly
//! read-only (`PTE_P | PTE_U`, no `PTE_W`, no `PTE_NX`).  Initialised data
//! is mapped writable with hardware `PTE_D` explicitly clear so the live
//! evictor can tell an untouched (droppable) page from a written one
//! (MEM-15).  bss/stack/tos map zeroed pages (D-14: dirty anonymous memory
//! is never evicted).


use port::mem::{PhysAddr, PAGE_SIZE_4K};
use port::println;

use crate::dat::Ureg;
use crate::proc::{
    self, alloc_phys_page, data_page_key, free_phys_page, phys_ptr_mut, text_page_key, MappingRecord,
    Proc, SegKind,
};

// Test-only: kernel_address_rejection pins the bound against KZERO.
#[cfg(test)]
use crate::proc::KZERO;

/// `#PF` error-code bits (SDM Vol. 3A §4.7).
pub(crate) const PF_P: u64 = 1 << 0;
pub(crate) const PF_WRITE: u64 = 1 << 1;
pub(crate) const PF_USER: u64 = 1 << 2;
pub(crate) const PF_INSTRUCTION: u64 = 1 << 4;

/// Leaf PTE bits the pager installs.
const PTE_P: u64 = 1 << 0;
const PTE_RW: u64 = 1 << 1;
const PTE_US: u64 = 1 << 2;
/// Hardware dirty bit (MEM-15): set by the CPU on the first store.
pub(crate) const PTE_D: u64 = 1 << 6;
const PTE_NX: u64 = 1 << 63;
/// Physical-address mask inside a leaf entry.
const PTE_PA_MASK: u64 = 0x000f_ffff_ffff_f000;

/// User text: present, user, READ-ONLY, executable (W^X; MEM-15/D-15).
const USER_TEXT_RO: u64 = PTE_P | PTE_US;
/// User data/bss/stack/tos: present, r/w, user, no-execute.
const USER_DATA_RW: u64 = PTE_P | PTE_RW | PTE_US | PTE_NX;

/// The decoded ring-3 `#PF` error code (pure, host-tested).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PfError {
    pub present: bool,
    pub write: bool,
    pub user: bool,
    pub instruction: bool,
}

impl PfError {
    pub(crate) fn decode(ecode: u64) -> PfError {
        PfError {
            present: ecode & PF_P != 0,
            write: ecode & PF_WRITE != 0,
            user: ecode & PF_USER != 0,
            instruction: ecode & PF_INSTRUCTION != 0,
        }
    }
}

/// True when `cr2` names a kernel (top-half) address — the demand pager
/// must never satisfy such a fault from a ring-3 trap (T-05-35).  The
/// kernel half begins where the user's 128 TiB end: at the direct map's
/// base, which is below KZERO — the Mach window and the device alias
/// live there too (until 2026-11-27 the bound was KZERO, and a ring-3
/// fault at those addresses reached the segment classifier instead).
pub(crate) fn is_kernel_address(cr2: u64) -> bool {
    cr2 >= crate::directmap::DIRECT_BASE
}

/// The ring-3 `#PF` entry from `trap()`'s vector-14 split.  Returns with
/// the fault resolved (the iretq tail retries the instruction) or kills
/// the offending process and parks.
pub fn user_fault(frame: &mut Ureg, cr2: u64) {
    let err = PfError::decode(frame.ecode);
    let va = cr2 as usize;

    // Reject kernel addresses and anything not from ring 3: one user
    // process's wild pointer kills one process, never the kernel.
    if is_kernel_address(cr2) || !err.user {
        unresolvable(frame, cr2, "address");
    }

    let Some(mut proc) = proc::current() else {
        panic!("user fault with no current process cr2={cr2:#x}");
    };
    let Some(segments) = proc.segments.clone() else {
        unresolvable(frame, cr2, "segments");
    };
    let page_va = va & !(PAGE_SIZE_4K - 1);

    // A fault on an already-mapped page is a protection violation, except
    // the one sanctioned case: a kernel write to an untouched Data page
    // through `UserSliceMut` promotes it to dirty (MEM-15).  A write to
    // text (or a fetch from data) kills the offending process (T-05-35).
    let already_mapped = proc.mapping_log.iter().any(|m| m.va == page_va);
    if already_mapped {
        if err.write && segments.classify(va) == Some(SegKind::Data) {
            if let Err(reason) = promote_data_page(&mut proc, page_va) {
                println!("user fault: {reason}");
                unresolvable(frame, cr2, "write");
            }
            return;
        }
        unresolvable(frame, cr2, if err.write { "write" } else { "access" });
    }

    // A present-leaf write to text is never a demand fault.
    if err.write && segments.classify(va) == Some(SegKind::Text) {
        unresolvable(frame, cr2, "text write");
    }

    let resolved = match segments.classify(va) {
        Some(SegKind::Text) => page_in_from_image(&mut proc, page_va, SegKind::Text),
        Some(SegKind::Data) => page_in_from_image(&mut proc, page_va, SegKind::Data),
        Some(SegKind::Bss) => page_in_zeroed(&mut proc, page_va, SegKind::Bss),
        Some(SegKind::Tos) => page_in_zeroed(&mut proc, page_va, SegKind::Tos),
        // The stack range IS the 16 MiB growth bound — a Stack
        // classification is always in-bounds.
        Some(SegKind::Stack) => page_in_zeroed(&mut proc, page_va, SegKind::Stack),
        None => unresolvable(frame, cr2, "unmapped"),
    };
    if let Err(reason) = resolved {
        println!("user fault: {reason}");
        unresolvable(frame, cr2, "page-in");
    }
}

/// Map `pa` (or a cached shared frame) at `page_va` with `leaf`.
fn map_user_leaf(proc: &mut Proc, page_va: usize, pa: u64, leaf: u64) -> Result<(), &'static str> {
    let created = super::pml4_map_user(proc.pml4_pa, page_va, pa, leaf)
        .map_err(|_| "fault: user map failed")?;
    for c in created.into_iter().flatten() {
        proc.mapping_tables.push(c);
    }
    Ok(())
}

/// The cache key for an image page (text is image-shared; data is
/// proc+exec-private, salted by pid and exec generation).
fn image_key(image_id: u32, proc: &Proc, page_va: usize, kind: SegKind) -> port::pagecache::PageKey {
    match kind {
        SegKind::Text => text_page_key(image_id, page_va),
        _ => data_page_key(image_id, proc.pid, proc.exec_gen, page_va),
    }
}

/// Demand-page text/data from the proc's byte source (MEM-15/MEM-16).
///
/// On a cache miss the page is allocated and filled OUTSIDE `PAGE_CACHE`
/// (the allocator may run the evictor, which re-acquires Rank 20); the
/// cache is re-checked afterwards so a concurrent insert on the other CPU
/// is deduplicated (the losing frame is freed back to `PAGE_ALLOC`).
fn page_in_from_image(proc: &mut Proc, page_va: usize, kind: SegKind) -> Result<(), &'static str> {
    let image_id = proc
        .text_source
        .as_ref()
        .ok_or("fault: page-in with no byte source")?
        .image_id;
    let key = image_key(image_id, proc, page_va, kind);

    if let Some(cache) = crate::pagealloc::page_cache()
        && let Some(page) = cache.lookup(key)
    {
        map_user_leaf(proc, page_va, page.pa.addr(), leaf_for(kind))?;
        cache.add_rmap(
            key,
            port::pagecache::RmapEntry { proc_slot: proc.rmap_slot(), page_va },
        );
        record_mapping(proc, page_va, page.pa.addr(), kind);
        return Ok(());
    }

    let mut pa = alloc_phys_page().map_err(|_| "fault: phys page alloc failed")?;
    if let Err(reason) = fill_page_from_source(proc, page_va, kind, pa) {
        free_phys_page(pa);
        return Err(reason);
    }

    // Re-check + deduplicate a concurrent insert.
    if let Some(cache) = crate::pagealloc::page_cache() {
        if let Some(page) = cache.lookup(key) {
            free_phys_page(pa);
            pa = page.pa.addr();
            cache.add_rmap(
                key,
                port::pagecache::RmapEntry { proc_slot: proc.rmap_slot(), page_va },
            );
        } else {
            let inserted = match kind {
                SegKind::Text => cache.insert_text(key, PhysAddr::new(pa)),
                _ => cache.register_untouched_data(key, PhysAddr::new(pa)),
            };
            if inserted.is_ok() {
                cache.add_rmap(
                    key,
                    port::pagecache::RmapEntry { proc_slot: proc.rmap_slot(), page_va },
                );
            }
        }
    }

    map_user_leaf(proc, page_va, pa, leaf_for(kind))?;
    match kind {
        SegKind::Text => println!("fault: text page-in va={page_va:#x}"),
        SegKind::Data => println!("fault: data page-in va={page_va:#x}"),
        _ => {}
    }
    record_mapping(proc, page_va, pa, kind);
    Ok(())
}

/// The leaf for a demand image page: text RO+exec, data RW+NX with the
/// hardware dirty bit explicitly clear.
fn leaf_for(kind: SegKind) -> u64 {
    match kind {
        SegKind::Text => USER_TEXT_RO,
        _ => USER_DATA_RW & !PTE_D,
    }
}

/// Zero + fill one page through the KZERO alias.  The read is CLAMPED to
/// the segment's exact content end: the file length covers syms too, so a
/// full-page read would bleed trailing file bytes into a page tail that
/// must stay zero.
fn fill_page_from_source(
    proc: &Proc,
    page_va: usize,
    kind: SegKind,
    pa: u64,
) -> Result<(), &'static str> {
    let segments = proc.segments.as_ref().ok_or("fault: page-in with no segments")?;
    let source = proc.text_source.as_ref().ok_or("fault: page-in with no byte source")?;
    let file_off = match kind {
        SegKind::Text => (page_va - crate::exec::TEXT_MAP_BASE) as u64,
        _ => (page_va - segments.data.start()) as u64 + source.data_file_base,
    };
    let content_end = match kind {
        SegKind::Text => crate::exec::TEXT_MAP_BASE + source.data_file_base as usize,
        _ => segments.data.end(),
    };
    let clamp = content_end.saturating_sub(page_va).min(PAGE_SIZE_4K);
    // SAFETY: `pa` is a freshly-allocated owned frame; exactly one page is
    // written through the boot tables' shared KZERO alias.
    let full = unsafe { core::slice::from_raw_parts_mut(phys_ptr_mut(pa), PAGE_SIZE_4K) };
    full.fill(0);
    if clamp > 0 {
        source
            .read_at(file_off, &mut full[..clamp])
            .map_err(|_| "fault: byte-source read_at failed")?;
    }
    Ok(())
}

/// Record a user mapping (rfork's copy and exit's free walk this log).
fn record_mapping(proc: &mut Proc, page_va: usize, pa: u64, kind: SegKind) {
    proc.mapping_log.push(MappingRecord {
        va: page_va,
        phys: pa as usize,
        size: PAGE_SIZE_4K,
        kind,
    });
}

/// Page-in a zeroed bss/stack/tos page.  `alloc_phys_page` already zeroes
/// the frame through the seam; the mapping is writable, no-execute.
fn page_in_zeroed(proc: &mut Proc, page_va: usize, kind: SegKind) -> Result<(), &'static str> {
    let pa = alloc_phys_page().map_err(|_| "fault: phys page alloc failed")?;
    map_user_leaf(proc, page_va, pa, USER_DATA_RW)?;
    match kind {
        SegKind::Stack => println!("fault: stack growth va={page_va:#x}"),
        SegKind::Bss => println!("fault: bss page-in va={page_va:#x}"),
        _ => println!("fault: zero page-in va={page_va:#x}"),
    }
    record_mapping(proc, page_va, pa, kind);
    Ok(())
}

/// Promote an already-mapped Data page to dirty (MEM-15, user-write or
/// `UserSliceMut` write).  On x86_64 the CPU sets `PTE_D` on the first
/// store, so a hardware-dirty page needs no PTE change; this path is the
/// kernel-write case (the page may still be droppable behind the cache).
fn promote_data_page(proc: &mut Proc, page_va: usize) -> Result<(), &'static str> {
    let record = proc
        .mapping_log
        .iter()
        .find(|rec| rec.va == page_va && rec.kind == SegKind::Data)
        .ok_or("fault: data page is not resident")?;
    if let Some(cache) = crate::pagealloc::page_cache() {
        cache.mark_data_dirty(PhysAddr::new(record.phys as u64));
    }
    Ok(())
}

/// The note-delivery path's stack residency (the aarch64
/// `ensure_stack_resident` shape).
pub(crate) fn ensure_stack_resident(proc: &mut Proc, start: usize, end: usize) {
    let mut page = start & !(PAGE_SIZE_4K - 1);
    while page < end {
        if !proc.mapping_log.iter().any(|m| m.va == page)
            && page_in_zeroed(proc, page, SegKind::Stack).is_err()
        {
            println!("fault: stack page-in failed during note delivery");
            kill_current_process();
        }
        page += PAGE_SIZE_4K;
    }
}

/// Force every page in `[start, end)` resident.  A writable range rejects
/// read-only text (T-05-35) and marks touched Data pages dirty so a kernel
/// write into an untouched buffer is never dropped (MEM-15).
pub(crate) fn ensure_user_resident(
    proc: &mut Proc,
    start: usize,
    end: usize,
    writable: bool,
) -> Result<(), &'static str> {
    if start >= end {
        return Ok(());
    }
    let mut page = start & !(PAGE_SIZE_4K - 1);
    while page < end {
        let mapped = proc.mapping_log.iter().any(|m| m.va == page);
        let kind = proc.segments.as_ref().and_then(|segments| segments.classify(page));
        match kind {
            Some(SegKind::Text) => {
                if writable {
                    return Err("usermem: write to read-only text");
                }
                if !mapped {
                    page_in_from_image(proc, page, SegKind::Text)?;
                }
            }
            Some(SegKind::Data) => {
                if !mapped {
                    page_in_from_image(proc, page, SegKind::Data)?;
                }
                if writable {
                    promote_data_page(proc, page)?;
                }
            }
            Some(growth @ (SegKind::Bss | SegKind::Stack | SegKind::Tos)) => {
                if !mapped {
                    page_in_zeroed(proc, page, growth)?;
                }
            }
            None => {
                if !mapped {
                    // An unmapped in-segment page is bss/stack growth.
                    page_in_zeroed(proc, page, SegKind::Bss)?;
                }
            }
        }
        page += PAGE_SIZE_4K;
    }
    Ok(())
}

/// Kill the current process from trap context through the exit tail.
pub(crate) fn kill_current_process() -> ! {
    let Some(mut proc) = proc::current() else {
        println!("fault kill with no current process — parking");
        crate::trap::park_forever();
    };
    crate::syscall::exit_tail(&mut proc, b"fault")
}

/// Unresolvable fault: print the identity and kill the process loudly.
fn unresolvable(frame: &Ureg, cr2: u64, access: &str) -> ! {
    let who = proc::current()
        .map(|p| {
            let n = p.name.iter().position(|&b| b == 0).unwrap_or(p.name.len());
            let name = core::str::from_utf8(&p.name[..n]).unwrap_or("?");
            alloc::format!("pid={} name={}", p.pid, name)
        })
        .unwrap_or_else(|| alloc::format!("pid=? name=?"));
    println!(
        "user fault: unresolvable addr={cr2:#x} pc={:#x} {access} {who}",
        frame.pc
    );
    println!("{frame:#x?}");
    kill_current_process();
}

/// The leaf PTE physical-address mask (host-tested, Task 3).
#[cfg(test)]
pub(crate) fn leaf_masks_pa(entry: u64) -> u64 {
    entry & PTE_PA_MASK
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MEM-14 encoding: every `#PF` error-code bit decodes independently.
    #[test]
    fn pf_error_code_bits_decode() {
        assert_eq!(
            PfError::decode(0),
            PfError { present: false, write: false, user: false, instruction: false }
        );
        assert_eq!(
            PfError::decode(PF_P | PF_WRITE | PF_USER | PF_INSTRUCTION),
            PfError { present: true, write: true, user: true, instruction: true }
        );
        let e = PfError::decode(PF_WRITE | PF_USER);
        assert!(!e.present && e.write && e.user && !e.instruction);
        let e = PfError::decode(PF_INSTRUCTION);
        assert!(!e.present && !e.write && !e.user && e.instruction);
        // Unrelated high bits never leak into the decode.
        let e = PfError::decode(0xffff_ffff_ffff_ffe0);
        assert!(!e.present && !e.write && !e.user && !e.instruction);
    }

    /// MEM-14 encoding: a top-half address (from the direct map's base
    /// up) is a kernel address and must be rejected (T-05-35); every
    /// user-half address passes.
    #[test]
    fn kernel_address_rejection() {
        assert!(is_kernel_address(KZERO));
        assert!(is_kernel_address(KZERO + PAGE_SIZE_4K as u64));
        assert!(is_kernel_address(u64::MAX));
        // The kernel half begins below KZERO: the direct map, the Mach
        // window and the device alias are kernel addresses as well.
        assert!(is_kernel_address(KZERO - PAGE_SIZE_4K as u64));
        assert!(is_kernel_address(crate::directmap::DIRECT_BASE + 0x1_2345_6000));
        assert!(is_kernel_address(crate::proc::MMIO_ALIAS as u64));
        assert!(is_kernel_address(crate::directmap::DIRECT_BASE), "the first kernel address");
        assert!(!is_kernel_address(crate::directmap::DIRECT_BASE - 1), "the last user address");
        assert!(!is_kernel_address(0));
        assert!(!is_kernel_address(0x2000));
        assert!(!is_kernel_address(0x0000_7fff_ffff_f000));
    }

    /// MEM-15 encoding: `PTE_D` is bit 6, RO text carries neither `PTE_RW`
    /// nor `PTE_NX`, data is writable+NX, and the leaf masks its PA.
    #[test]
    fn pte_d_and_leaf_kinds() {
        assert_eq!(PTE_D, 1 << 6);
        assert_eq!(USER_TEXT_RO & (PTE_RW | PTE_NX), 0);
        assert_eq!(USER_TEXT_RO & (PTE_P | PTE_US), PTE_P | PTE_US);
        assert_ne!(USER_DATA_RW & PTE_RW, 0);
        assert_ne!(USER_DATA_RW & PTE_NX, 0);
        assert_eq!(USER_DATA_RW & PTE_D, 0, "data leaf starts dirty-clear");
        assert_eq!(leaf_masks_pa(0x1234_5000 | PTE_P | PTE_RW), 0x1234_5000);
        assert_eq!(leaf_masks_pa(0x000f_ffff_ffff_f000 | PTE_NX), 0x000f_ffff_ffff_f000);
    }
}
