//! User demand-fault handler (D-09 — the phase's core VM deliverable):
//! instruction/data aborts from EL0 (EC 0x20/0x24) decode their fault class,
//! classify the faulting VA against the current proc's segment map, and
//! either page in (text/data from the paqfs byte source, zero pages for
//! bss/stack/tos) or kill the process loudly.
//!
//! Trap-context invariants (T-04-09, 04-RESEARCH.md Pitfall 3):
//! - IRQs arrive masked and are NEVER unmasked here.
//! - The handler touches only pre-mapped kernel memory and the user VA being
//!   installed.  trap.rs routes only lower-EL aborts here, so a
//!   kernel-context FAR is a userland wild pointer: the process dies via
//!   the unresolvable kill (WR-08).  A handler-SIDE kernel-context touch
//!   arrives as a current-EL abort (EC 0x21/0x25) and hits trap()'s
//!   dump-and-halt — the real T-04-09 guard.
//! - No locks are held on entry and PROC_TABLE is never taken; the Console
//!   MCS lock (println!) is safe pre-syscall-era — nothing else holds it in
//!   trap context (04-PATTERNS.md "Locking discipline"; the tick print is
//!   gone before EL0 exists).
//! - The paqfs reader is stateless-per-call (04-02) — reentrant here.
//! - pagealloc's MCS lock is taken only briefly (single page allocations)
//!   and never while holding another lock.

use port::mem::PAGE_SIZE_4K;
use port::println;

use crate::param::KZERO;
use crate::proc::{self, MappingRecord, Proc, SegKind};
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
    // WR-08 (T-04-09 corrected): trap.rs routes ONLY lower-EL aborts here
    // (EC 0x20/0x24), so the FAR is always a USER address — a top-half
    // value is an ordinary userland wild pointer, not a kernel bug, and
    // panicking turned any future binary's bad pointer into a kernel halt
    // (user-reachable DoS).  Fall through to the unresolvable kill, which
    // since WR-06 routes through exit_tail: one process dies, the kernel
    // and its siblings live.  A handler-side touch of kernel-context
    // memory would arrive as a CURRENT-EL abort (EC 0x21/0x25) and hits
    // trap()'s dump-and-halt — that path is the real T-04-09 guard.
    if va >= KZERO {
        unresolvable(frame, class, far, access);
    }
    let proc_ptr = proc::current_proc();
    if proc_ptr.is_null() {
        panic!("user fault with no current process class={class} addr={va:#x}");
    }
    let proc = unsafe { &mut *proc_ptr };
    let page_va = va & !(PAGE_SIZE_4K - 1);

    let Some(segments) = proc.segments.clone() else {
        unresolvable(frame, class, far, access);
    };

    // A fault on an already-mapped page is a genuine protection/access
    // violation (e.g. fetching from a data page) — not demand paging; remap
    // would silently lose live page contents.  Kill loudly instead.
    let already_mapped = proc.mapping_log.iter().any(|m| m.va == page_va);

    match segments.classify(va) {
        Some(SegKind::Text) if !already_mapped => page_in_from_image(proc, page_va, SegKind::Text),
        Some(SegKind::Data) if !already_mapped => page_in_from_image(proc, page_va, SegKind::Data),
        Some(SegKind::Bss) if !already_mapped => page_in_zeroed(proc, page_va, SegKind::Bss),
        Some(SegKind::Tos) if !already_mapped => page_in_zeroed(proc, page_va, SegKind::Tos),
        // The stack segment range IS the 16 MiB growth bound — a Stack
        // classification is always in-bounds; faults below the bound
        // classify None and fall through to unresolvable.
        Some(SegKind::Stack) if !already_mapped => page_in_zeroed(proc, page_va, SegKind::Stack),
        _ => unresolvable(frame, class, far, access),
    }
}

/// Demand-page text/data from the proc's paqfs byte source into the ACTIVE
/// user table, writing the bytes THROUGH THE USER VA (the faulting process's
/// TTBR0 is live — no aliasing, no PAN on cortex-a72; 04-RESEARCH.md
/// Pitfall 4).  Text pages get the D-09 icache sequence after the write.
fn page_in_from_image(proc: &mut Proc, page_va: usize, kind: SegKind) {
    let pa = pagealloc::allocate_physpage()
        .unwrap_or_else(|_| panic!("fault: phys page alloc failed"));
    // Text is UXN-clear/PXN-set (user-executable); data is both-set.
    let entry = match kind {
        SegKind::Text => Entry::rw_user_text(),
        _ => Entry::rw_user_data(),
    }
    .with_phys_addr(pa);
    map_active_user(page_va, entry);

    // Zero first (fresh phys pages are garbage and the file window may be
    // short at a segment tail), then read the file window in at byte
    // granularity (Pitfall 5: data file offsets are not page-aligned).
    //
    // CLAMP the read to the segment's exact content end (04-06 Rule-1 fix,
    // exposed by the exec-era runtime): the paqfs entry's length covers the
    // WHOLE file (syms included), so an unclamped full-page read bleeds
    // trailing file bytes (the syms section) into the page tail — bytes that
    // must stay ZERO (the bss window sharing the last data page; the
    // padding past text end).  Poisoned bss globals are read as
    // file-string garbage (observed: exits()'s onex[i].f became "tas.s" and
    // the loop made an indirect call into ASCII).
    let full = unsafe { core::slice::from_raw_parts_mut(page_va as *mut u8, PAGE_SIZE_4K) };
    full.fill(0);
    let Some(source) = proc.text_source.as_ref() else {
        panic!("fault: {kind:?} page-in with no byte source");
    };
    let file_off = match kind {
        // The 40-byte header is part of the text mapping: a text page's file
        // offset is its VA offset from 0x10000.
        SegKind::Text => (page_va - exec::TEXT_MAP_BASE) as u64,
        // data file offset = (pageVA - data_va) + (40 + textsize)
        _ => {
            (page_va - proc.segments.as_ref().unwrap().data.start()) as u64 + source.data_file_base
        }
    };
    // Exact content end (VA): text content ends where the data FILE base
    // begins (data_file_base = 40 + textsize, i.e. TEXT_MAP_BASE + that is
    // the text-content end VA); data ends at the exact data_end.
    let content_end = match kind {
        SegKind::Text => exec::TEXT_MAP_BASE + source.data_file_base as usize,
        _ => proc.segments.as_ref().unwrap().data.end(),
    };
    let clamp = content_end.saturating_sub(page_va).min(PAGE_SIZE_4K);
    let dst = &mut full[..clamp];
    // 05-07: the byte source abstracts the backing (devroot's paqfs image
    // window, or a devmnt binary's staged whole-file buffer — the pager
    // NEVER RPCs; trap context can't block on the server).
    if let Err(err) = source.read_at(file_off, dst) {
        panic!("fault: byte-source read_at failed: {err}");
    }
    if kind == SegKind::Text {
        flush_icache_user_range(page_va, page_va + PAGE_SIZE_4K);
    }
    // Evidence markers (exec-profile proof of demand paging).
    match kind {
        SegKind::Text => println!("fault: text page-in va={page_va:#x}"),
        _ => println!("fault: data page-in va={page_va:#x}"),
    }
    proc.mapping_log.push(MappingRecord {
        va: page_va,
        phys: pa.addr() as usize,
        size: PAGE_SIZE_4K,
        kind,
    });
}

/// 04-08 (T-04-16's residency half): page-in any unmapped pages of
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
            page_in_zeroed(proc, page, SegKind::Stack);
        }
        page += PAGE_SIZE_4K;
    }
}

/// 05-06 (the cold-syscall-buffer class — surfaced by the 05-06 gate's
/// paqfs, which emalloc's a heap buffer and lets the KERNEL's pread make
/// its first touch): generalize ensure_stack_resident to ANY writable
/// user range — an unmapped page is classified (Bss for brk_-grown heap,
/// Stack for growth pages) and paged in zeroed.  The syscall layer calls
/// this after validate_user_range and BEFORE any deref of a user buffer
/// (pread's read target, pwrite's read source, errstr/await/fd2path's
/// outputs, pipe's fd array): membership proves the segment, but a
/// demand-paged page that no EL0 code has touched yet aborts at EL1h on
/// a kernel access (the documented T-04-11 caveat — phase-04 deferred
/// #3's class, closed at exec for data+bss in 05-04 and now closed for
/// post-exec growth pages here).  This is EXPLICIT page-in at a
/// controlled point with no locks held — never a nested fault from
/// syscall context, so T-05-06's rule holds.
pub(crate) fn ensure_user_resident(proc: &mut Proc, start: usize, end: usize) {
    if start >= end {
        return;
    }
    let mut page = start & !(PAGE_SIZE_4K - 1);
    while page < end {
        if !proc.mapping_log.iter().any(|m| m.va == page) {
            // An unmapped in-segment page is bss-growth or stack-growth
            // by construction (text/data/tos are exec-mapped); classify
            // for the mapping log's record (rfork-copy/exit-free walk it).
            let kind = match proc.segments.as_ref().and_then(|s| s.classify(page)) {
                Some(SegKind::Stack) => SegKind::Stack,
                _ => SegKind::Bss,
            };
            page_in_zeroed(proc, page, kind);
        }
        page += PAGE_SIZE_4K;
    }
}

/// Map a zeroed page (bss/stack/tos).  Stack faults arrive only inside the
/// 16 MiB growth bound — the segment classifier bounds them.
fn page_in_zeroed(proc: &mut Proc, page_va: usize, kind: SegKind) {    let pa = pagealloc::allocate_physpage()
        .unwrap_or_else(|_| panic!("fault: phys page alloc failed"));
    map_active_user(page_va, Entry::rw_user_data().with_phys_addr(pa));
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
}

/// Map `entry` at `va` in the ACTIVE user root table — the faulting
/// process's TTBR0 is live, and map_to's temp-recursive dance edits it
/// directly (its per-write full TLB invalidate is heavy but correct;
/// optimization is Phase 5+).
fn map_active_user(va: usize, entry: Entry) {
    let mut physalloc = PhysPageAllocator {};
    let mut vmt = VmTraitImpl {};
    vm::root_page_table(RootPageTableType::User)
        .map_to(&mut physalloc, &mut vmt, entry, va, PageSize::Page4K, RootPageTableType::User)
        .unwrap_or_else(|err| panic!("fault: user map failed at {va:#x}: {err:?}"));
}

/// D-09 icache/dcache maintenance after a text page-in, on the USER VA
/// (04-RESEARCH.md "D-09 Deliverable" verbatim): clean D to the Point of
/// Unification per CTR_EL0.DminLine, DSB ISH, invalidate I to PoU per
/// IminLine, DSB ISH, ISB.  Geometry is ALWAYS discovered from CTR_EL0 —
/// never hardcode 64 (Phase-2 D-03's architectural-discovery rule).
/// aarch64-cpu 11.2 has no CTR_EL0 wrapper, hence the inline asm (the same
/// pattern as vm.rs's ttbr reads).
fn flush_icache_user_range(start: usize, end: usize) {
    #[cfg(not(test))]
    unsafe {
        let ctr: u64;
        core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr);
        let dline = 4usize << ((ctr >> 16) & 0xf); // DminLine
        let iline = 4usize << (ctr & 0xf); // IminLine
        let mut p = start & !(dline - 1);
        while p < end {
            // clean dcache to Point of Unification
            core::arch::asm!("dc cvau, {}", in(reg) p);
            p += dline;
        }
        core::arch::asm!("dsb ish"); // drains the cleans (Inner-Shareable)
        let mut p = start & !(iline - 1);
        while p < end {
            // invalidate icache to Point of Unification
            core::arch::asm!("ic ivau, {}", in(reg) p);
            p += iline;
        }
        core::arch::asm!("dsb ish");
        core::arch::asm!("isb"); // discard any stale prefetched instructions
    }
    #[cfg(test)]
    {
        let _ = (start, end);
    }
}

/// Unresolvable fault: print the marker and kill the process loudly.
fn unresolvable(frame: &TrapFrame, class: u8, far: u64, access: &str) -> ! {
    println!(
        "user fault: unresolvable class={class} addr={far:#x} pc={:#x} {access}",
        frame.pc()
    );
    kill_current_process();
}

/// Kill the current process from trap/fault context (WR-06): route through
/// the 04-07 exit tail — free the user mappings, mark Moribund, print the
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
    let proc_ptr = proc::current_proc();
    if proc_ptr.is_null() {
        // No current process (pre-userspace trap): nothing to route
        // through the lifecycle — park.
        println!("fault kill with no current process — parking");
        loop {
            unsafe { core::arch::asm!("wfi") };
        }
    }
    let proc = unsafe { &mut *proc_ptr };
    crate::syscall::exit_tail(proc, b"fault");
}
