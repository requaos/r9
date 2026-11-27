//! User demand-fault handler:
//! instruction/data aborts from EL0 (EC 0x20/0x24) decode their fault class,
//! classify the faulting VA against the current proc's segment map, and
//! either page in (text/data from the paqfs byte source, zero pages for
//! bss/stack/tos) or kill the process loudly.
//!
//! Trap-context invariants:
//! - IRQs arrive masked and are NEVER unmasked here.
//! - The handler touches only pre-mapped kernel memory and the user VA being
//!   installed.  trap.rs routes only lower-EL aborts here, so a
//!   kernel-context FAR is a userland wild pointer: the process dies via
//!   the unresolvable kill.  A handler-SIDE kernel-context touch
//!   arrives as a current-EL abort (EC 0x21/0x25) and hits trap()'s
//!   dump-and-halt — the real kernel-side guard.
//! - No locks are held on entry and PROC_TABLE is never taken; the Console
//!   MCS lock (println!) is safe pre-syscall-era — nothing else holds it in
//!   trap context (the tick print is
//!   gone before EL0 exists).
//! - The paqfs reader is stateless-per-call — reentrant here.
//! - pagealloc's MCS lock is taken only briefly (single page allocations)
//!   and never while holding another lock.

use port::mem::{PAGE_SIZE_4K, PhysAddr, PhysRange};
use port::println;

use crate::param::KZERO;
use crate::proc::{self, MappingRecord, Proc, SegKind, data_page_key, text_page_key};
use crate::registers::{EsrEl1, EsrEl1IssDataAbort, EsrEl1IssInstructionAbort, ExceptionClass};
use crate::trap::TrapFrame;
use crate::vm::{self, Entry, PageSize, PhysPageAllocator, RootPageTableType, VmTraitImpl};
use crate::{exec, pagealloc};

/// The EC 0x20/0x24 entry from trap()'s split.  Returns with the fault
/// resolved (the restore path erets back into EL0) or kills the process and
/// parks (unresolvable).
pub fn user_fault(frame: &mut TrapFrame, esr: EsrEl1, far: u64) {
    // Decode the fault class: IFSC for instruction aborts (a fetch), DFSC +
    // WnR (write-not-read) for data aborts.  trap() already matched the EC,
    // so construct the ISS decoders directly; unknown status encodings are
    // unresolvable.
    let (class, access) = match esr.exception_class_enum() {
        Ok(ExceptionClass::InstructionAbortLowerEl) => {
            let iss = EsrEl1IssInstructionAbort(esr.iss());
            match iss.instruction_fault() {
                Ok(fsc) => (fsc as u8, "fetch"),
                Err(n) => unresolvable(frame, n, far, "fetch"),
            }
        }
        Ok(ExceptionClass::DataAbortLowerEl) => {
            let iss = EsrEl1IssDataAbort(esr.iss());
            match iss.data_fault() {
                Ok(fsc) => (fsc as u8, if iss.wnr() { "write" } else { "read" }),
                Err(n) => unresolvable(frame, n, far, "access"),
            }
        }
        // Not a lower-EL abort — the trap.rs split only routes EC 0x20/0x24
        // here; anything else is a kernel bug.
        _ => panic!("user_fault: unexpected exception class {:?}", esr.exception_class_enum()),
    };

    // faultarm64's routing table (9front@350266104f:sys/src/9/arm64/trap.c:273-332):
    // translation (4-7), access-flag (8-11), permission (12-15), TLB-conflict
    // (48) → the demand path; everything else kills the process loudly.
    match class {
        4..=7 | 8..=11 | 12..=15 | 48 => {}
        other => unresolvable(frame, other, far, access),
    }

    let va = far as usize;
    // trap.rs routes ONLY lower-EL aborts here
    // (EC 0x20/0x24), so the FAR is always a USER address — a top-half
    // value is an ordinary userland wild pointer, not a kernel bug, and
    // panicking turned any future binary's bad pointer into a kernel halt
    // (user-reachable DoS).  Fall through to the unresolvable kill, which
    // routes through exit_tail: one process dies, the kernel
    // and its siblings live.  A handler-side touch of kernel-context
    // memory would arrive as a CURRENT-EL abort (EC 0x21/0x25) and hits
    // trap()'s dump-and-halt — that path is the real kernel-side guard.
    if va >= KZERO {
        unresolvable(frame, class, far, access);
    }
    let Some(mut proc) = proc::current() else {
        panic!("user fault with no current process class={class} addr={va:#x}");
    };
    let page_va = va & !(PAGE_SIZE_4K - 1);

    let Some(segments) = proc.segments.clone() else {
        unresolvable(frame, class, far, access);
    };

    // A fault on an already-mapped page is a protection/access violation —
    // except the ONE sanctioned case: MEM-15's write-protect dirty
    // promotion for an untouched initialised-data page.  A write to text
    // (or a fetch from data) kills the offending process.
    let already_mapped = proc.mapping_log.iter().any(|m| m.va == page_va);

    if already_mapped {
        if access == "write" && segments.classify(va) == Some(SegKind::Data) {
            if let Err(reason) = promote_data_page(&mut proc, page_va) {
                println!("user fault: {reason}");
                unresolvable(frame, class, far, access);
            }
            return;
        }
        unresolvable(frame, class, far, access);
    }

    let resolved = match segments.classify(va) {
        Some(SegKind::Text) => page_in_from_image(&mut proc, page_va, SegKind::Text),
        Some(SegKind::Data) => page_in_from_image(&mut proc, page_va, SegKind::Data),
        Some(SegKind::Bss) => page_in_zeroed(&mut proc, page_va, SegKind::Bss),
        Some(SegKind::Tos) => page_in_zeroed(&mut proc, page_va, SegKind::Tos),
        // The stack segment range IS the 16 MiB growth bound — a Stack
        // classification is always in-bounds; faults below the bound
        // classify None and fall through to unresolvable.
        Some(SegKind::Stack) => page_in_zeroed(&mut proc, page_va, SegKind::Stack),
        None => unresolvable(frame, class, far, access),
    };
    if let Err(reason) = resolved {
        println!("user fault: {reason}");
        unresolvable(frame, class, far, access);
    }
}

/// Demand-page text/data from the proc's image byte source (MEM-15/MEM-16).
///
/// Text is SHARED through `palloc.hash` and mapped read-only at EL0 and EL1;
/// the frame is filled through the kernel `KZERO + pa` alias before the RO
/// PTE is installed.  Untouched initialised data is mapped read-only and
/// registered in the cache as droppable until its first write.
/// Returns an error when the allocator cannot satisfy the fault (the caller
/// kills the process rather than panicking the kernel).
fn page_in_from_image(proc: &mut Proc, page_va: usize, kind: SegKind) -> Result<(), &'static str> {
    let image_id = proc
        .text_source
        .as_ref()
        .ok_or("fault: page-in with no byte source")?
        .image_id;
    let key = page_image_key(image_id, proc, page_va, kind);

    // Cache hit: map the shared/cached frame read-only and join the rmap.
    if let Some(cache) = pagealloc::page_cache() {
        if let Some(page) = cache.lookup(key) {
            map_active_user(page_va, page_entry(kind).with_phys_addr(page.pa))?;
            cache.add_rmap(
                key,
                port::pagecache::RmapEntry { proc_slot: proc.rmap_slot(), page_va },
            );
            record_mapping(proc, page_va, page.pa, kind);
            return Ok(());
        }
    }

    // Miss: allocate + fill OUTSIDE `PAGE_CACHE` (a miss must never hold the
    // Rank-20 lock across allocation, whose evictor re-acquires it).
    let mut pa = pagealloc::allocate_physpage().map_err(|_| "fault: phys page alloc failed")?;
    if let Err(reason) = fill_page_from_source(proc, page_va, kind, pa) {
        let _ = pagealloc::deallocate_physpage(pa);
        return Err(reason);
    }

    // Re-check + deduplicate a concurrent insert on the other CPU.
    if let Some(cache) = pagealloc::page_cache() {
        if let Some(page) = cache.lookup(key) {
            let _ = pagealloc::deallocate_physpage(pa);
            pa = page.pa;
        } else {
            let inserted = match kind {
                SegKind::Text => cache.insert_text(key, pa),
                _ => cache.register_untouched_data(key, pa),
            };
            if inserted.is_ok() {
                cache.add_rmap(
                    key,
                    port::pagecache::RmapEntry { proc_slot: proc.rmap_slot(), page_va },
                );
            }
        }
    }

    map_active_user(page_va, page_entry(kind).with_phys_addr(pa))?;
    if kind == SegKind::Text {
        flush_icache_user_range(page_va, page_va + PAGE_SIZE_4K);
    }
    // Evidence markers (exec-profile proof of demand paging).
    match kind {
        SegKind::Text => println!("fault: text page-in va={page_va:#x}"),
        SegKind::Data => println!("fault: data page-in va={page_va:#x}"),
        _ => {}
    }
    record_mapping(proc, page_va, pa, kind);
    Ok(())
}

/// The RO/write-protect PTE for a demand image page.
fn page_entry(kind: SegKind) -> Entry {
    match kind {
        SegKind::Text => Entry::ro_user_text(),
        _ => Entry::ro_user_data(),
    }
}

/// The `palloc.hash` key for an image page.
fn page_image_key(image_id: u32, proc: &Proc, page_va: usize, kind: SegKind) -> port::pagecache::PageKey {
    match kind {
        SegKind::Text => text_page_key(image_id, page_va),
        _ => data_page_key(image_id, proc.pid, proc.exec_gen, page_va),
    }
}

/// Zero + fill one page through the kernel `KZERO + pa` alias.
///
/// CLAMP the read to the segment's exact content end: the paqfs entry's
/// length covers the WHOLE file (syms included), so an unclamped full-page
/// read would bleed trailing file bytes into a page tail that must stay zero
/// (the bss window sharing the last data page; the padding past text end).
fn fill_page_from_source(
    proc: &mut Proc,
    page_va: usize,
    kind: SegKind,
    pa: PhysAddr,
) -> Result<(), &'static str> {
    let segments = proc.segments.as_ref().ok_or("fault: page-in with no segments")?;
    let source = proc.text_source.as_ref().ok_or("fault: page-in with no byte source")?;
    let file_off = match kind {
        // The 40-byte header is part of the text mapping: a text page's file
        // offset is its VA offset from 0x10000.
        SegKind::Text => (page_va - exec::TEXT_MAP_BASE) as u64,
        // data file offset = (pageVA - data_va) + (40 + textsize)
        _ => (page_va - segments.data.start()) as u64 + source.data_file_base,
    };
    let content_end = match kind {
        SegKind::Text => exec::TEXT_MAP_BASE + source.data_file_base as usize,
        _ => segments.data.end(),
    };
    let clamp = content_end.saturating_sub(page_va).min(PAGE_SIZE_4K);
    let alias = map_kernel_alias(pa)?;
    // SAFETY: the KZERO+PA alias was just mapped r/w in the kernel table; the
    // slice is exactly one page and is the fill target.
    let full = unsafe { core::slice::from_raw_parts_mut(alias, PAGE_SIZE_4K) };
    full.fill(0);
    if clamp > 0 {
        // The byte source abstracts the backing (devroot's paqfs image
        // window, or a devmnt binary's staged whole-file buffer — the pager
        // NEVER RPCs; trap context can't block on the server).
        source
            .read_at(file_off, &mut full[..clamp])
            .map_err(|_| "fault: byte-source read_at failed")?;
    }
    Ok(())
}

/// Map `pa` at the kernel's `KZERO + pa` alias and return the writable
/// pointer.  `map_to` REPLACES, so re-mapping an existing alias is safe.
fn map_kernel_alias(pa: PhysAddr) -> Result<*mut u8, &'static str> {
    let mut physalloc = PhysPageAllocator {};
    let mut vmt = VmTraitImpl {};
    vm::kernel_pagetable()
        .map_phys_range(
            &mut physalloc,
            &mut vmt,
            "faultpage",
            &PhysRange::with_pa_len(pa, PAGE_SIZE_4K),
            vm::VaMapping::Offset(KZERO),
            Entry::rw_kernel_data(),
            PageSize::Page4K,
            RootPageTableType::Kernel,
        )
        .map_err(|_| "fault: kernel alias map failed")?;
    Ok((KZERO + pa.addr() as usize) as *mut u8)
}

/// Append the mapping-log record (rfork copy + exit-free walk it by kind).
fn record_mapping(proc: &mut Proc, page_va: usize, pa: PhysAddr, kind: SegKind) {
    proc.mapping_log.push(MappingRecord {
        va: page_va,
        phys: pa.addr() as usize,
        size: PAGE_SIZE_4K,
        kind,
    });
}

/// Promote an already-mapped clean `SegKind::Data` page to `AllRw`, mark it
/// dirty, and invalidate its VA (MEM-15, Pitfall 2).  Used by the EL0 write
/// fault and by `ensure_user_resident(..., writable: true)` before a kernel
/// write through `UserSliceMut`.
fn promote_data_page(proc: &mut Proc, page_va: usize) -> Result<(), &'static str> {
    let record = proc
        .mapping_log
        .iter()
        .find(|rec| rec.va == page_va && rec.kind == SegKind::Data)
        .ok_or("usermem: data page is not resident")?;
    let pa = PhysAddr::new(record.phys as u64);
    if !vm::set_user_leaf(page_va, Entry::rw_user_data().with_phys_addr(pa)) {
        return Err("usermem: data page has no leaf PTE");
    }
    if let Some(cache) = pagealloc::page_cache() {
        cache.mark_data_dirty(pa);
    }
    Ok(())
}

/// Page-in any unmapped pages of
/// [start, end) as zeroed Stack pages.  The note-delivery path's stack
/// window must be RESIDENT before the kernel writes it — membership was
/// already proven by validate_user_range, but an in-segment page that was
/// never demand-faulted would take an EL1h abort mid-delivery (a kernel
/// panic, never acceptable on a user-triggerable path).  Same machinery
/// and context as a user stack fault (IRQ-masked, the proc's TTBR0 live).
pub(crate) fn ensure_stack_resident(proc: &mut Proc, start: usize, end: usize) {
    let mut page = start & !(PAGE_SIZE_4K - 1);
    while page < end {
        if !proc.mapping_log.iter().any(|m| m.va == page) {
            if page_in_zeroed(proc, page, SegKind::Stack).is_err() {
                println!("fault: stack page-in failed during note delivery");
                kill_current_process();
            }
        }
        page += PAGE_SIZE_4K;
    }
}

/// The cold-syscall-buffer class (a
/// paqfs server emalloc's a heap buffer and lets the KERNEL's pread make
/// its first touch): generalize ensure_stack_resident to ANY user range —
/// an unmapped page is classified and paged in, and `writable == true` also
/// promotes every clean `SegKind::Data` page to `AllRw` + dirty BEFORE the
/// caller writes it (Pitfall 2: AllRo denies EL1 writes).  The syscall layer
/// calls this after validate_user_range and BEFORE any deref of a user
/// buffer; explicit page-in at a controlled point with no locks held.
/// Returns an error for a write targeting read-only text, or on allocation
/// failure (the syscall reports a bad buffer rather than panicking).
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
                    // An unmapped in-segment page is bss-growth or
                    // stack-growth by construction; record it as Bss.
                    page_in_zeroed(proc, page, SegKind::Bss)?;
                }
            }
        }
        page += PAGE_SIZE_4K;
    }
    Ok(())
}

/// Map a zeroed page (bss/stack/tos).  Stack faults arrive only inside the
/// 16 MiB growth bound — the segment classifier bounds them.
fn page_in_zeroed(proc: &mut Proc, page_va: usize, kind: SegKind) -> Result<(), &'static str> {
    let pa = pagealloc::allocate_physpage().map_err(|_| "fault: phys page alloc failed")?;
    map_active_user(page_va, Entry::rw_user_data().with_phys_addr(pa))?;
    // SAFETY: page_va was just mapped r/w in the ACTIVE user table above
    // (the faulting proc's TTBR0 is live); the whole page is zeroed.
    let dst = unsafe { core::slice::from_raw_parts_mut(page_va as *mut u8, PAGE_SIZE_4K) };
    dst.fill(0);
    match kind {
        SegKind::Stack => println!("fault: stack growth va={page_va:#x}"),
        SegKind::Bss => println!("fault: bss page-in va={page_va:#x}"),
        _ => println!("fault: zero page-in va={page_va:#x}"),
    }
    proc.mapping_log.push(MappingRecord {
        va: page_va,
        phys: pa.addr() as usize,
        size: PAGE_SIZE_4K,
        kind,
    });
    Ok(())
}

/// Map `entry` at `va` in the ACTIVE user root table — the faulting
/// process's TTBR0 is live, and map_to's temp-recursive dance edits it
/// directly (its per-write full TLB invalidate is heavy but correct;
/// optimization is future work).
fn map_active_user(va: usize, entry: Entry) -> Result<(), &'static str> {
    let mut physalloc = PhysPageAllocator {};
    let mut vmt = VmTraitImpl {};
    vm::root_page_table(RootPageTableType::User)
        .map_to(&mut physalloc, &mut vmt, entry, va, PageSize::Page4K, RootPageTableType::User)
        .map_err(|_| "fault: user map failed")
}

/// Icache/dcache maintenance after a text page-in, on the USER VA:
/// clean D to the Point of
/// Unification per CTR_EL0.DminLine, DSB ISH, invalidate I to PoU per
/// IminLine, DSB ISH, ISB.  Geometry is ALWAYS discovered from CTR_EL0 —
/// never hardcode 64 (the architectural-discovery rule).
/// aarch64-cpu 11.2 has no CTR_EL0 wrapper, hence the inline asm (the same
/// pattern as vm.rs's ttbr reads).
/// The sequence lives in vm.rs (the
/// TLB/barrier/cache floor) — this wrapper keeps the fault.rs call sites.
fn flush_icache_user_range(start: usize, end: usize) {
    vm::flush_icache_user_range(start, end);
}

/// Live evictor: drop up to `target` unpinned re-readable pages (MEM-15,
/// MEM-14, D-14/D-17/D-21).  Runs with `PAGE_ALLOC` released (the caller is
/// `PageAlloc::alloc_clean`), so the lock order is
/// `PAGE_CACHE (20) -> PROC_TABLE (50)` -> release -> `request_shootdown`
/// (SGI 1) -> `PAGE_ALLOC (70)`.  Pinned frames are skipped and counted by
/// `PageCache::select_eviction_victim` (`pinned_skipped`).
pub(crate) fn evict_re_readable_pages(target: usize) -> usize {
    port::palloc::record_evict_call();
    let Some(cache) = pagealloc::page_cache() else {
        #[cfg(not(test))]
        {
            use core::sync::atomic::{AtomicU64, Ordering};
            static ABSENT_LOGGED: AtomicU64 = AtomicU64::new(0);
            if ABSENT_LOGGED.fetch_add(1, Ordering::Relaxed) < 4 {
                println!("evict: call target={target} page_cache=absent");
            }
        }
        return 0;
    };
    #[cfg(not(test))]
    {
        use core::sync::atomic::{AtomicU64, Ordering};
        // One line per boot at the exhaustion boundary: the cache state and
        // pool cardinalities the live evictor first saw.  This is the
        // pressure gate's evidence anchor (counters alone say how much was
        // reclaimed, not that the pool had truly reached zero free frames).
        static CALLS: AtomicU64 = AtomicU64::new(0);
        if CALLS.fetch_add(1, Ordering::Relaxed) == 0 {
            let s = port::palloc::cache_stats();
            let pool = pagealloc::pool_snapshot();
            println!(
                "evict: first reclaim target={target} entries={} droppable={} inserts={} hits={} misses={} fails={} pool=(clean={} dirty={} active={} pinned={})",
                s.entries,
                s.droppable,
                s.inserts,
                s.hits,
                s.misses,
                s.insert_failures,
                pool.clean,
                pool.dirty,
                pool.active,
                pool.pinned
            );
        }
    }
    let mut freed = 0;
    while freed < target {
        let Some(candidate) = cache.select_eviction_victim(pagealloc::is_pinned) else {
            break;
        };
        // Re-check after the candidate's key left the hash: a fault on the
        // other CPU may have pinned it in the window.  Abort that victim
        // (the frame stays mapped and uncached; nothing is freed).
        if pagealloc::is_pinned(candidate.pa) {
            continue;
        }
        let _unmapped = proc::evict_user_frame(candidate.pa);
        // The VAs are cleared on every CPU's tables; invalidate the TLB
        // before returning the frame to the allocator.
        port::smp::request_shootdown(Some(candidate.pa.addr() as usize));
        // A failed free cannot be retried usefully (the cache key is gone);
        // count the attempt so the loop terminates.
        match pagealloc::deallocate_physpage(candidate.pa) {
            Ok(()) => port::palloc::record_evict_reclaimed(),
            Err(err) => {
                port::palloc::record_evict_free_failure();
                #[cfg(not(test))]
                println!("evict: free {:#x} failed: {err:?}", candidate.pa.addr());
            }
        }
        freed += 1;
    }
    freed
}

/// Unresolvable fault: print the marker and kill the process loudly.
fn unresolvable(frame: &TrapFrame, class: u8, far: u64, access: &str) -> ! {
    // Name the faulting process in the print (the wedgedbg dump's
    // `name=` precedent) — a bare pc/addr pair cannot say WHICH binary
    // faulted when several share the text-base convention, and that
    // identity is the first fact a post-mortem needs.  The full table
    // dump follows (the wedge autopsy's shape): a wild read's family
    // tree is the second fact.
    let who = proc::current()
        .map(|p| {
            let n = p.name.iter().position(|&b| b == 0).unwrap_or(p.name.len());
            let name = core::str::from_utf8(&p.name[..n]).unwrap_or("?");
            alloc::format!("pid={} name={}", p.pid, name)
        })
        .unwrap_or_else(|| alloc::format!("pid=? name=?"));
    println!(
        "user fault: unresolvable class={class} addr={far:#x} pc={:#x} {access} {who}",
        frame.pc()
    );
    // The frame dump (the virtpanic autopsy shape): the faulter's
    // registers are the third fact — a nil/garbage pointer's origin
    // register is visible here.
    println!("{:#?}", frame);
    proc::dump_proc_states();
    kill_current_process();
}

/// Kill the current process from trap/fault context: route through
/// the exit tail — free the user mappings, mark Moribund, print the
/// exit line, post the wait record and wake an AWAITing parent, free the
/// root-table page and kstack, then hand off to the scheduler (the
/// all-exited clean halt remains exit_tail's tail).  Exactly the shape
/// deliver_notes's no-handler kill already uses (trap.rs): one process's
/// wild pointer now kills ONE process — never the IRQs-masked wfi wedge of
/// the single-process era, which took every sibling and the scheduler down
/// with it and left the parent's AWAIT blocked on children_live > 0.
/// trap()'s other-EL0-origin sync path (the donor faultnote shape) shares
/// this.
pub(crate) fn kill_current_process() -> ! {
    let Some(mut proc) = proc::current() else {
        // No current process (pre-userspace trap): nothing to route
        // through the lifecycle — park (the consolidated
        // halt primitive).
        println!("fault kill with no current process — parking");
        crate::trap::park_forever();
    };
    crate::syscall::exit_tail(&mut proc, b"fault");
}
