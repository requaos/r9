//! Process scaffolding: per-process kernel stacks and
//! root page tables, a small fixed proc table behind the MCS-lock idiom,
//! the current-proc pointer trap-context code reads, and the
//! scheduler — tick-driven preemptive round-robin (preempt/reschedule)
//! plus the swtch-based blocking path (sched/sched_loop) that AWAIT rides.
//! exec.rs seeds the forkret frame onto this scaffolding, fault.rs
//! pages into its root table, and the syscall dispatch lands
//! RFORK/AWAIT/EXITS-wait-records here.
//!
//! Locking discipline:
//! - PROC_TABLE is taken only briefly in process/boot/trap context —
//!   NEVER across the fault handler or a pagealloc call that could
//!   recurse, and never across a swtch (the resumed proc may lock it).
//! - Trap-context code reads the current proc through `current_proc()`'s
//!   raw pointer, set at exec/switch time.  Invariant: single CPU, and
//!   trap handlers run IRQ-masked, so the pointer is stable for the
//!   handler's lifetime.
//! - CURKSTACK/CURFRAME (the trap.S entry-stack/live-frame symbols) are
//!   written only under the PROC_TABLE lock or in IRQ-masked trap
//!   context; CURFRAME is read by trap.S at entry/return — same
//!   single-CPU invariant.  CURKSTACK is proc.rs's record of the
//!   per-process entry-stack invariant (the value SP_EL1 holds at every
//!   EL0 trap entry); the trap_return tail reconstructs it as
//!   frame+288 (every frame sits at entry-stack top - 288),
//!   so nothing reads it.

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::SyncUnsafeCell;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use port::Result;
use port::dat::Chan;
use port::devenv::EnvGroup;
use port::fdgrp::FdGroup;
use port::mcslock::{Lock, LockNode};
use port::mem::{PAGE_SIZE_4K, PhysAddr, PhysRange, VirtRange};
use port::paqfs::PaqDir;
use port::pgrp::Pgrp;
use port::println;
use port::rendez::RendezGroup;

use crate::kmem::{from_ptr_to_physaddr_offset_from_kzero, physaddr_as_ptr_mut_offset_from_kzero};
use crate::pagealloc;
use crate::param::KZERO;
use crate::swtch;
use crate::trap::TrapFrame;
use crate::vm::{
    self, Entry, PageSize, PhysPageAllocator, RootPageTable, RootPageTableType, VaMapping,
    VmTraitImpl,
};

/// Plan 9 KNAMELEN (27 + NUL).
pub const KNAMELEN: usize = 28;
/// Plan 9 ERRMAX (sys/include/libc.h) — errstr and note-message buffers.
pub const ERRMAX: usize = 128;
/// Proc table size.  Slots are never REAPED this era (a Moribund husk
/// pins its slot — the documented exit-sweep deferral), so the table is
/// a boot-session fork BUDGET, not a concurrency bound: the 07-07 net
/// sweep alone forks ~20 times (a dd + an echo per loop iteration, the
/// bind/clone/cat/test machinery, the blaster), and the 16-slot table
/// wedged mid-sweep ("proc: table full" at arp-send 3).  The 06-07
/// tier-1 sweep MEASURES its fork count on the donor: 201 born-traced
/// pids for the full sweep (each check's command forks + backquote
/// pipelines; hfactotum's "rfork: proc: table full" was the 64-slot
/// table's wall at the sweep's factotum check).  512 covers the
/// measured 201 + the boot chain with >2× headroom; the exit-sweep
/// deferral stays the tracked follow-up (a long-lived shell still
/// eventually fills ANY fixed table).
#[cfg(not(dev_churn))]
pub const MAX_PROCS: usize = 512;
/// The churn profile's fork budget (20-09, the x86_64 17-02 twin,
/// cfg-gated so every other build keeps the measured-201 shape above):
/// the churn driver's ~1400 exec iterations plus the churnsem RFMEM
/// contenders (~40 generations x 5 procs) each pin a husk slot — a
/// boot-session budget above the default.
#[cfg(dev_churn)]
pub const MAX_PROCS: usize = 2048;
/// Open fd slots per proc (init's closefds closes
/// 3..29 — the table must hold fds through 29 without making that fatal;
/// pipe+dup flows hold several at once).
pub const MAX_FDS: usize = 32;
/// Per-process kernel stack: 16 KiB (4 pages) — the INTERRUPTSTACKSZ
/// discipline (a dedicated fixed-size kernel stack per trap-taking context).
const KSTACK_PAGES: usize = 4;
/// Queued-notes capacity (the note delivery path fills it).
pub const NOTE_QUEUE_CAP: usize = 8;
/// The exec-recorded argv's byte bound (the /proc/N/args
/// content — ps reads 256 bytes max, ps.c:63).  NUL-joined per arg.
pub const ARGV_CAP: usize = 256;

/// The note-group id allocator (the donor's
/// `Ref noteidalloc` — 9k proc.c:13; fresh id per newproc, inherited or
/// split per the rfork RFNOTEG rules).
static NEXT_NOTEID: AtomicU64 = AtomicU64::new(1);

/// User segment kinds — the fault handler's classification and the mapping
/// log's tag (rfork-copy/exit-free walk the log by kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegKind {
    Text,
    Data,
    Bss,
    Stack,
    Tos,
}

/// One user mapping — appended by exec (seed pages) and the fault handler
/// (demand pages).
#[derive(Clone, Copy, Debug)]
pub struct MappingRecord {
    pub va: usize,
    pub phys: usize,
    pub size: usize,
    pub kind: SegKind,
}

/// A proc's user VA map (r9 owns stack-top/growth policy; text/data
/// VAs come from the a.out header — exec.rs records them here).
#[derive(Clone, Debug)]
pub struct Segments {
    pub text: VirtRange,
    pub data: VirtRange,
    pub bss: VirtRange,
    pub stack: VirtRange,
    pub tos: VirtRange,
}

impl Segments {
    /// Classify a faulting user VA.  Only the text range is page-rounded at
    /// exec time; the data/bss ranges are byte-exact, so when
    /// `data % 4096 != 0 && bss > 0` the last data page and the head of bss
    /// share one 4K page.  That shared page must classify as Data:
    /// classify by PAGE at the data/bss boundary, not by address — a
    /// bss-first fault would otherwise map the page zeroed and the data
    /// file bytes at [page_va, data_end) would never load (the page is then
    /// mapped, so no later fault can repair it — silent userland data
    /// corruption).  The Data page-in's zero-fill + byte-exact content_end
    /// clamp yields exactly the right bytes either order:
    /// file bytes below data_end, zeros above.  Empty segments (zero-size
    /// data/bss) classify nothing; the byte-exact ranges stay authoritative
    /// for validate_user_range.  The stack range IS the 16 MiB growth
    /// bound, so a Stack hit is always in-bounds.
    pub fn classify(&self, va: usize) -> Option<SegKind> {
        // Data's classification end, page-rounded into the bss head
        // — but never past bss_end (an empty/tiny bss extends nothing), and
        // never below data's own end.
        let data_classify_end =
            self.data.0.end.next_multiple_of(PAGE_SIZE_4K).min(self.bss.0.end.max(self.data.0.end));
        if self.text.0.contains(&va) {
            Some(SegKind::Text)
        } else if self.data.0.start <= va && va < data_classify_end {
            Some(SegKind::Data)
        } else if self.bss.0.contains(&va) {
            Some(SegKind::Bss)
        } else if self.tos.0.contains(&va) {
            Some(SegKind::Tos)
        } else if self.stack.0.contains(&va) {
            Some(SegKind::Stack)
        } else {
            None
        }
    }
}

/// The demand-page byte source's backing.
#[derive(Clone)]
pub enum ByteBacking {
    /// devroot binaries: the initrd image + the file's paqfs entry
    /// (demand-paged reads index the image by file offset).
    Paq { dir: PaqDir, image: &'static [u8] },
    /// devmnt binaries (the /bin union crossing): the WHOLE file,
    /// read eagerly at exec through the 9P client — syscall context can
    /// block on RPCs; the trap-context pager NEVER can (a fault can't
    /// block on the server), so the bytes are kernel-owned from exec on.
    /// Arc-shared across rfork (the source-handle clone).
    Mnt { file: Arc<Vec<u8>> },
}

impl ByteBacking {
    /// Read the file's bytes at `file_off` into `dst` (exec + the pager's
    /// shared byte window), clamped to the file's length; returns the byte
    /// count (0 at/past EOF).
    pub fn read_at(&self, file_off: u64, dst: &mut [u8]) -> Result<usize> {
        match self {
            ByteBacking::Paq { dir, image } => {
                let fs = port::paqfs::Paqfs::open_image(image)?;
                fs.read_at(dir, file_off, dst)
            }
            ByteBacking::Mnt { file } => {
                if file_off >= file.len() as u64 {
                    return Ok(0);
                }
                let start = file_off as usize;
                let n = (file.len() - start).min(dst.len());
                dst[..n].copy_from_slice(&file[start..start + n]);
                Ok(n)
            }
        }
    }

    /// The backing file's total length (the truncated-binary check):
    /// the paqfs entry's recorded length / the staged buffer's length.
    pub fn file_len(&self) -> u64 {
        match self {
            ByteBacking::Paq { dir, .. } => u64::from(dir.length),
            ByteBacking::Mnt { file } => file.len() as u64,
        }
    }
}

/// RFMEM shared-page refcounts — the donor dupseg `sameseg` accounting
/// (sys/src/9/port/segment.c) translated to r9's per-proc mapping_log
/// model.  The donor shares a Segment kernel object whose pte map both
/// procs reference; r9 has no such object — each proc's root table maps
/// the shared PHYS page directly, so the sharer count lives here keyed
/// by physical address.  A page enters the table at its first RFMEM
/// share (count = 2: parent + child) and leaves at the release that
/// drops the count to 0 (the last sharer's exit/exec frees it — the
/// donor's Segment `ref` hitting 0).  Pages never shared are absent
/// from the table and free unconditionally — exactly the pre-RFMEM
/// behavior.
///
/// Locking: syscall context only, briefly (rfork's share arm; the
/// exit/exec release walk; the rfork mid-copy unwind) — never IRQ
/// context, the same leaf discipline as PROC_TABLE.
static SHARED_PAGES: Lock<SharedPages> = Lock::new("rfmem", SharedPages::new());

/// The PA → sharer-count table behind SHARED_PAGES.  A Vec with a
/// linear scan is the right shape: the entry count is the shared
/// Data/Bss page count of the live RFMEM families (ping's is a
/// handful), and share/release run at fork/exit rates, never per-op.
struct SharedPages {
    map: Vec<(u64, usize)>,
}

impl SharedPages {
    const fn new() -> Self {
        Self { map: Vec::new() }
    }

    /// Record one more sharer of `pa` (rfork's RFMEM arm): the first
    /// share inserts at 2 (parent + child); a later share (a shared
    /// proc forking again under RFMEM) increments.
    fn share(&mut self, pa: u64) {
        for entry in &mut self.map {
            if entry.0 == pa {
                entry.1 += 1;
                return;
            }
        }
        self.map.push((pa, 2));
    }

    /// Insert a page BORN shared by `count` procs (the rfork bss-sweep
    /// and the brk_ family-growth arm: a fresh zero page mapped into
    /// every family member at once).  The page must be absent — a
    /// re-share of a live page goes through share().
    fn share_new(&mut self, pa: u64, count: usize) {
        debug_assert!(count >= 2, "share_new with fewer than two sharers");
        self.map.push((pa, count));
    }

    /// Drop one sharer of `pa`.  Returns true when the caller must FREE
    /// the page — either it was never shared (absent from the table:
    /// an ordinary per-proc page, the pre-RFMEM free behavior) or this
    /// release was the last sharer.  False = sibling sharers still map
    /// the page: the caller unmaps it from the exiting proc's table but
    /// must NOT return it to the allocator.
    fn release(&mut self, pa: u64) -> bool {
        if let Some(pos) = self.map.iter().position(|entry| entry.0 == pa) {
            let remaining = self.map[pos].1 - 1;
            if remaining == 0 {
                self.map.swap_remove(pos);
                return true;
            }
            self.map[pos].1 = remaining;
            return false;
        }
        true
    }
}

/// rfork's RFMEM arm: record the child sharing the parent's page.
fn share_user_page(pa: u64) {
    let node = LockNode::new();
    SHARED_PAGES.lock(&node).share(pa);
}

/// The rfork bss-sweep and sys_brk_ family arm: record a page BORN
/// shared by `count` members (never a re-share of a live page).
fn share_new_user_page(pa: u64, count: usize) {
    let node = LockNode::new();
    SHARED_PAGES.lock(&node).share_new(pa, count);
}

/// The free-side pivot (free_user_mappings + rfork's mid-copy unwind):
/// true = deallocate the page (never shared, or this was the last
/// sharer); false = siblings still map it (unmap only).
fn release_user_page(pa: u64) -> bool {
    let node = LockNode::new();
    SHARED_PAGES.lock(&node).release(pa)
}

/// An RFMEM family — the donor's shared-segment GROUP.  rfork under
/// RFMEM makes the parent and child share their Data/Bss pages (the
/// dupseg `sameseg` arm); the donor's shared SEGMENT then grows for
/// both procs together (sysbrk_ moves the shared segment's break, and a
/// page faulted into it is visible to both through the shared pte map).
/// r9 has no shared segment object, so the family registry records the
/// members: sys_brk_'s family arm maps the grown pages into EVERY
/// member's root table (zeroed, refcounted in SHARED_PAGES) and moves
/// every member's break, keeping the members' Data/Bss mappings
/// identical by construction.  The invariant: at any instant, all
/// members of a family map the same PAs across [bss start, break) and
/// carry the same break.  rfork establishes it (the share arm + the
/// bss sweep), brk_ maintains it, exec/exit dissolve membership.
/// Stack/Tos pages are NEVER family-shared (SG_STACK is always a fresh
/// copy — note delivery writes the proc's own stack).
static RFMEM_FAMILIES: Lock<Vec<(u64, u64)>> = Lock::new("rfmemfam", Vec::new());

/// The family-id allocator (fresh id per first RFMEM fork in a family).
static NEXT_RFMEM_FAMILY: AtomicU64 = AtomicU64::new(1);

/// The Proc field is `rfmem_family: Option<u64>`.
/// Join the caller's family (or found one when the parent forks its
/// first RFMEM child).  Runs ONLY after a successful rfork_copy (the
/// unwind path has no family state to tear down).
fn rfmem_join(parent: &mut Proc, child: &mut Proc) {
    let family = match parent.rfmem_family {
        Some(f) => f,
        None => {
            let f = NEXT_RFMEM_FAMILY.fetch_add(1, Ordering::Relaxed);
            parent.rfmem_family = Some(f);
            let node = LockNode::new();
            RFMEM_FAMILIES.lock(&node).push((f, parent.pid));
            f
        }
    };
    child.rfmem_family = Some(family);
    let node = LockNode::new();
    RFMEM_FAMILIES.lock(&node).push((family, child.pid));
}

/// Dissolve the proc's membership (exec/exit via free_user_mappings —
/// the address space the family co-owns is gone).  The pages themselves
/// are released by the mapping_log walk's refcount pivot; this drops
/// only the membership record.
fn rfmem_leave(proc: &mut Proc) {
    let Some(family) = proc.rfmem_family.take() else { return };
    let node = LockNode::new();
    let mut families = RFMEM_FAMILIES.lock(&node);
    if let Some(pos) = families.iter().position(|&(f, pid)| f == family && pid == proc.pid) {
        families.swap_remove(pos);
    }
}

/// sys_brk_'s family arm (the donor's shared-segment break move): every
/// member's break moves to `new_end`, and the GROWN range's pages are
/// born shared — one fresh zeroed page per VA mapped into every
/// member's table and refcounted at the member count.  This is what
/// keeps an RFMEM family's Data/Bss mappings identical by construction:
/// libc's malloc pool grows through brk_, and the grown pages must be
/// visible to every member (the 08-04 ping gate: the sender's malloc'd
/// request nodes must be readable by the receiver).  Shrink moves only
/// the ends (r9 never frees pages on brk_ shrink — the standing lazy
/// shape, now family-wide).  A one-live-member family (siblings
/// exited/exec'd) is an ordinary private proc: lazy end move only.
/// Harvey 20-07 measured that arm (brkprobe, 5 HVF + 5 TCG runs, owner
/// D-09 ramfs-venue-brk-holds): the lone member's 8 MiB growth took
/// exactly 2048 demand bss page-ins, so x86_64's eager one-member
/// mapping is deliberately NOT ported — aarch64 demand-faults Bss.
///
/// Failure hygiene: a mid-arm failure (OOM) unmaps everything this call
/// mapped and releases the published refcounts (the last release frees)
/// — the family is left exactly as at entry.
pub fn rfmem_brk_set(family: u64, old_end: usize, new_end: usize) -> Result<()> {
    let members = rfmem_family_members(family);
    if members.len() < 2 {
        if let Some(&m) = members.first() {
            // SAFETY: table-resident pointer (rfmem_family_members);
            // single CPU, IRQ-masked syscall context — the member is
            // not running while the caller's syscall executes.
            unsafe {
                if let Some(segs) = (*m).segments.as_mut() {
                    segs.bss.0.end = new_end;
                }
            }
        }
        return Ok(());
    }
    let mut done: Vec<(usize, u64)> = Vec::new();
    let mut va = old_end.next_multiple_of(PAGE_SIZE_4K);
    let limit = new_end.next_multiple_of(PAGE_SIZE_4K);
    while va < limit {
        let pa = pagealloc::allocate_physpage().map_err(|_| "brk: out of memory")?;
        map_kernel_alias(pa.addr())?;
        // SAFETY: the page's KZERO+PA alias was just (re)mapped r/w in
        // the kernel table above; one full fresh page, owned.
        unsafe {
            ptr::write_bytes(physaddr_as_ptr_mut_offset_from_kzero::<u8>(pa), 0, PAGE_SIZE_4K);
        }
        let entry = Entry::rw_user_data().with_phys_addr(pa);
        let mut mapped: Vec<*mut Proc> = Vec::new();
        let mut failed = false;
        for &m in &members {
            // SAFETY: table-resident pointer; single CPU, IRQ-masked
            // syscall context.  map_to edits the member's table (ACTIVE
            // for the caller, INACTIVE for its siblings — both handled)
            // with the per-write TLB invalidate; the VA was unmapped, so
            // no stale translation exists.
            unsafe {
                let p = &mut *m;
                let mut physalloc = PhysPageAllocator {};
                let mut vmt = VmTraitImpl {};
                if (*p.root_table)
                    .map_to(&mut physalloc, &mut vmt, entry, va, PageSize::Page4K, RootPageTableType::User)
                    .is_err()
                {
                    failed = true;
                    break;
                }
                p.mapping_log.push(MappingRecord {
                    va,
                    phys: pa.addr() as usize,
                    size: PAGE_SIZE_4K,
                    kind: SegKind::Bss,
                });
                mapped.push(m);
            }
        }
        if failed {
            // This page never published a refcount: unmap it from the
            // members that took it, drop their records, free the page.
            for &m in &mapped {
                unsafe {
                    let p = &mut *m;
                    let mut physalloc = PhysPageAllocator {};
                    let mut vmt = VmTraitImpl {};
                    let _ = (*p.root_table).map_to(
                        &mut physalloc,
                        &mut vmt,
                        Entry::empty(),
                        va,
                        PageSize::Page4K,
                        RootPageTableType::User,
                    );
                    if let Some(pos) = p.mapping_log.iter().position(|r| r.va == va) {
                        p.mapping_log.swap_remove(pos);
                    }
                }
            }
            let _ = pagealloc::deallocate_physpage(pa);
            break;
        }
        share_new_user_page(pa.addr(), members.len());
        done.push((va, pa.addr()));
        va += PAGE_SIZE_4K;
    }
    let completed_all = va >= limit;
    if !completed_all {
        // Unwind the completed pages: unmap from every member and release
        // per member — each page's count runs members.len() → 0, the
        // last release freeing it.
        for &(done_va, done_pa) in &done {
            for &m in &members {
                unsafe {
                    let p = &mut *m;
                    let mut physalloc = PhysPageAllocator {};
                    let mut vmt = VmTraitImpl {};
                    let _ = (*p.root_table).map_to(
                        &mut physalloc,
                        &mut vmt,
                        Entry::empty(),
                        done_va,
                        PageSize::Page4K,
                        RootPageTableType::User,
                    );
                    if let Some(pos) = p.mapping_log.iter().position(|r| r.va == done_va) {
                        p.mapping_log.swap_remove(pos);
                    }
                }
                // The LAST release (count → 0) frees: the return is the
                // free verdict — ignoring it leaked every completed page
                // of a failed growth (WR-03).
                if release_user_page(done_pa) {
                    let _ = pagealloc::deallocate_physpage(PhysAddr::new(done_pa));
                }
            }
        }
        return Err("brk: out of memory");
    }
    // Move every member's break (shrink included — ends only).
    for &m in &members {
        // SAFETY: as above.
        unsafe {
            if let Some(segs) = (*m).segments.as_mut() {
                segs.bss.0.end = new_end;
            }
        }
    }
    Ok(())
}

/// The proc's family siblings as raw Proc pointers (sys_brk_'s growth
/// arm edits every member).  PROC_TABLE-brief: the pointers are
/// table-resident and stable under the module header's single-CPU/
/// IRQ-masked invariant (the ProcGuard discipline); the caller mutates
/// through them AFTER the lock drops.
pub fn rfmem_family_members(family: u64) -> Vec<*mut Proc> {
    let node = LockNode::new();
    let pids: Vec<u64> = {
        let families = RFMEM_FAMILIES.lock(&node);
        families.iter().filter(|&&(f, _)| f == family).map(|&(_, pid)| pid).collect()
    };
    let table = PROC_TABLE.lock(&node);
    let mut out = Vec::new();
    for p in table.procs.iter() {
        if let Some(p) = p {
            if pids.contains(&p.pid) && p.state != ProcState::Moribund {
                out.push(ptr::addr_of!(*p) as *mut Proc);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SEMACQUIRE/SEMRELEASE (37/38) — the semaphore wait registry.
// The semaphore WORD lives in user memory (libc Lock.sem); the waiters
// are keyed by the word's PHYSICAL address so an RFMEM family — whose
// members map the same PA — contends on ONE queue, while unrelated
/// procs reusing the same VA name different PAs and never meet.
// ---------------------------------------------------------------------------

/// (word PA, waiter pid) pairs.  Small and scanned at fork/lock rates:
/// the contended-lock wait set of the whole system.
static SEM_WAITERS: Lock<Vec<(u64, u64)>> = Lock::new("sem", Vec::new());

/// sys_semacquire's contended arm: record the caller as a waiter on the
/// word's PA.  Idempotent (the contains check) — the acquire loop
/// RE-registers before every block: sem_wake consumes registrations as
/// it picks its wakers, so a woken waiter that loses the unit race
/// re-registers on its next block (the wakeup-loss-free shape).
pub fn sem_register_waiter(pa: u64, pid: u64) {
    let node = LockNode::new();
    let mut waiters = SEM_WAITERS.lock(&node);
    if !waiters.contains(&(pa, pid)) {
        waiters.push((pa, pid));
    }
}

/// Acquired (or erroring out): the caller leaves the wait set.  An
/// ORDER-PRESERVING remove: sem_wake picks each word's waiters in
/// registration order, and every word shares this one Vec, so
/// swap_remove (the Vec's last entry moved into the freed slot) reordered
/// OTHER words' waiters — a deregister on word A turned word B's
/// [21, 22] into [22, 21].  That broke B's FIFO wake order, and it was the
/// ~1/500 flake of the parallel sem tests (two tests' words in one Vec).
pub fn sem_deregister_waiter(pa: u64, pid: u64) {
    let node = LockNode::new();
    let mut waiters = SEM_WAITERS.lock(&node);
    if let Some(pos) = waiters.iter().position(|&e| e == (pa, pid)) {
        waiters.remove(pos);
    }
}

/// sys_semrelease's wake half: ready up to `delta` waiters registered on
/// the word's PA (registration order — the donor semwakeup's list walk).
/// Entries are consumed as they are picked: a woken waiter that loses
/// the unit race re-registers on its next block (the register→check→
/// block ordering is wakeup-loss-free by construction — single CPU,
/// IRQ-masked syscall context: no releaser can run between a waiter's
/// check and its sched()).  A stale entry (a pid that exited mid-wait —
/// possible only via a note kill of a blocked acquirer, which this era's
/// uninterruptible semacquire never takes; kept as hygiene) is pruned
/// without effect (a Moribund/absent pid is never Blocked).
pub fn sem_wake(pa: u64, delta: u64) {
    if delta == 0 {
        return;
    }
    let node = LockNode::new();
    let picked: Vec<u64> = {
        let mut waiters = SEM_WAITERS.lock(&node);
        let mut picked = Vec::new();
        let mut i = 0;
        while i < waiters.len() && (picked.len() as u64) < delta {
            if waiters[i].0 == pa {
                picked.push(waiters[i].1);
                waiters.remove(i);
            } else {
                i += 1;
            }
        }
        picked
    };
    if picked.is_empty() {
        return;
    }
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.state == ProcState::Blocked && picked.contains(&p.pid) {
            p.state = ProcState::Ready;
        }
    }
}

/// The demand-page byte source for text/data: the backing (the paqfs image
/// entry for devroot binaries; the staged whole file for devmnt ones) plus
/// the data-segment file base (40 + textsize — data file offsets are NOT page-aligned).
/// Clone for rfork (the child mirrors the parent's source handles;
/// the Mnt variant Arc-shares the staged bytes).
#[derive(Clone)]
pub struct ByteSource {
    pub backing: ByteBacking,
    pub data_file_base: u64,
}

impl ByteSource {
    /// The pager's read path (fault.rs) — delegates to the backing.
    pub fn read_at(&self, file_off: u64, dst: &mut [u8]) -> Result<usize> {
        self.backing.read_at(file_off, dst)
    }
}

/// A queued note message (delivered at the return-to-EL0 boundary).
pub struct Note {
    pub buf: [u8; ERRMAX],
    pub len: usize,
}

/// Exit/wait record slot (the exits/await path fills and consumes).
pub struct WaitRecord {
    pub pid: u64,
    pub msg: [u8; ERRMAX],
    pub msg_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcState {
    Ready,
    Running,
    /// Blocked mid-syscall (AWAIT) — holds a saved kernel context
    /// (blocked_in_kernel) and resumes via the sched loop's swtch.
    Blocked,
    Moribund,
}

/// A user process.  Raw-pointer fields (`root_table`, `frame`) name memory
/// this proc owns (pagealloc'd, never freed in the single-process era) —
/// they stay valid for the proc's lifetime.
pub struct Proc {
    pub pid: u64,
    pub name: [u8; KNAMELEN],
    pub state: ProcState,
    /// Top of this proc's 16 KiB kernel stack (pagealloc'd at construction).
    pub kstack_top: usize,
    /// Physical pages backing the kernel stack (the EXITS path frees
    /// them — the VAs come from the kernel-VA bump allocator and say nothing
    /// about their PAs, so construction records them here).
    pub kstack_pas: [u64; KSTACK_PAGES],
    /// Per-process root page table (one pagealloc'd page, recursive slot-511
    /// self-entry written at construction).  NEVER the vm.rs user-table
    /// static singleton — one shared table would give every process the same
    /// address space.
    pub root_table: *mut RootPageTable,
    /// fd group: the slots live in a SHAREABLE
    /// group object (the donor's Fgrp; fdgrp.rs) behind the mcslock
    /// idiom, so rfork's fd semantics are real per fork(2)/sysproc.c:
    /// RFPROC with NEITHER RFFDG nor RFCFDG SHARES this handle (Arc
    /// clone); RFFDG gives the child a fresh table COPIED slot-by-slot
    /// (pipe chans clone their Arc — the same queue); RFCFDG a fresh
    /// EMPTY table.  (libc
    /// fork() = rfork(RFPROC|RFFDG|RFREND) = COPY, which connectembed's
    /// dup/close choreography requires.)  fd 0/1/2 are devcons chans,
    /// preopened at construction (Plan 9 convention).  The group
    /// Lock is a LEAF (syscall context only, briefly; never from IRQ
    /// context, never held across a blocking pipe op — the checked-out
    /// chan pattern, fdgrp.rs).  At exit, exit_tail detaches the dying
    /// proc's group and,
    /// when it was the LAST sharer (Arc::strong_count == 1), drains every
    /// occupied slot and close-dispatches each chan AFTER the lock drops
    /// (the donor pexit closefgrp; syscall.rs's exit_tail comment names
    /// the structural exclusions — the devsrv registry clone and the
    /// parked paqfs io child are never swept BY DESIGN).
    pub fd_group: Arc<Lock<FdGroup<MAX_FDS>>>,
    /// User mappings, exec/fault-appended — rfork-copy/exit-free walk this.
    pub mapping_log: Vec<MappingRecord>,
    /// The a.out-derived VA map; None until exec.
    pub segments: Option<Segments>,
    /// paqfs byte source for demand page-in; None until exec.
    pub text_source: Option<ByteSource>,
    /// The stashed trap frame on this proc's kernel stack (the forkret seed
    /// at exec; trap save/restore rides it from then on).  Read by the
    /// CURFRAME reload at trap return — written by the scheduler's locked
    /// swap and by reschedule's stash.
    pub frame: *mut TrapFrame,
    /// Kernel-context switch state — valid only while blocked_in_kernel
    /// (sched() saved it mid-syscall); the sched loop swtch'es it
    /// back in.  Preempted-at-EL0 procs carry no kernel context — they
    /// resume through the CURFRAME/trap_return path only.
    pub context: swtch::Context,
    /// True while `context` holds a live saved kernel context (the proc
    /// blocked mid-syscall).  The sched loop resumes such procs via swtch;
    /// every other Ready proc resumes via trap_return on `frame`.
    pub blocked_in_kernel: bool,
    /// The user FP/SIMD state (q0-q31, FPCR, FPSR) while the proc does
    /// not own the registers; stale while it does (fpu.rs's LIVE_OWNER
    /// invariant).  Installed at every switch-in (activate_first,
    /// pick_next, reschedule), parked at every switch-out (sched,
    /// reschedule) and by a kernel SIMD section; rfork seeds a child's
    /// from the parent's current state, exec zeroes it.
    pub fpsave: crate::fpu::FpState,
    /// The interrupted code's FP/SIMD state while a note handler runs
    /// (the donor's notify-time fpsave): trap.rs's deliver_notes records
    /// it, NOTED's NCONT/NRSTR put it back, so a handler that uses FP
    /// cannot clobber the code it interrupted.  Meaningful only while
    /// note_ureg != 0.
    pub note_fpsave: crate::fpu::FpState,
    /// The rfork parent — EXITS posts the wait record here.  Null
    /// for the staged init (its exits posts nothing).
    pub parent: *mut Proc,
    /// rfork'd children not yet reaped by AWAIT (live or with a pending
    /// wait record) — AWAIT's block-vs-"no children" pivot.  RFNOWAIT
    /// children are excluded: they post no record and are
    /// invisible to AWAIT, donor-style.
    pub children_live: usize,
    /// rfork(RFNOWAIT) child: EXITS posts no wait record and wakes nobody.
    pub nowait: bool,
    pub note_queue: VecDeque<Note>,
    pub notify_handler: Option<u64>,
    /// The saved-Ureg user VA of the most recently delivered note (
    /// donor `up->noteureg`): set by trap.rs's deliver_notes, read and
    /// cleared by NOTED.  0 = not inside a note handler.
    pub note_ureg: u64,
    /// The last delivered note's bytes (donor `up->lastnote`) —
    /// NOTED's NDFLT arm exits with this message.
    pub last_note: [u8; ERRMAX],
    pub last_note_len: usize,
    /// Posted child-exit records (one per exited child, consumed by
    /// AWAIT in post order).
    pub wait_records: VecDeque<WaitRecord>,
    pub errstr: [u8; ERRMAX],
    /// The namespace group handle (the mount table; pgrp.rs).
    /// rfork flag semantics per fork(2): RFPROC with NEITHER RFNAMEG nor
    /// RFCNAMEG shares this handle (Arc clone); RFNAMEG deep-copies
    /// (pgrpcpy); RFCNAMEG starts clean (the newns wipe).
    /// The Lock serializes bind/unmount against namec walks; taken only
    /// from syscall context, briefly, never from IRQ context.
    pub pgrp: Arc<Lock<Pgrp>>,
    /// The environment group handle (devenv.rs).  rfork
    /// flag semantics per fork(2) + 9k sysproc.c: RFPROC with NEITHER
    /// RFENVG nor RFCENVG SHARES this handle (Arc clone); RFENVG
    /// deep-copies (envcpy); RFCENVG starts EMPTY.  (fork(2)/sysproc.c
    /// govern the two flags.)
    /// The Lock serializes setenv/create against reads; a LEAF lock
    /// (syscall context only, never IRQ context).
    pub env_group: Arc<Lock<EnvGroup>>,
    /// The process root and current-directory chans (donor up->slash /
    /// up->dot).  exec_bootstrap sets both to the devroot attach before the
    /// first EL0 entry; rfork inherits (value-dup); exec preserves them
    /// (Plan 9 convention).  None only pre-exec.
    pub slash: Option<Chan>,
    pub dot: Option<Chan>,
    /// The note-group id — the donor's `noteid` (9k
    /// portdat.h: "equivalent of note group").  A FRESH group id is
    /// assigned at construction (donor newproc's incref(&noteidalloc));
    /// rfork(RFPROC) WITHOUT RFNOTEG has the child INHERIT the parent's
    /// group (9k sysproc.c:183 `if((flag&RFNOTEG) == 0) p->noteid =
    /// up->noteid`), and non-RFPROC rfork(RFNOTEG) re-groups the CALLER
    /// (init's fexec child does this before exec'ing rc — the shell's
    /// console-DEL interrupts never reach init).  The console DEL posts
    /// "interrupt" to the console readers' groups via
    /// post_note_group_of_members (devcons.rs's foreground-group scoping).
    pub noteid: u64,
    /// True while the proc is parked in a devcons read
    /// (set/cleared by the cons_block_current hook around sched()).  The
    /// tick's cons_input_wake readies cons-waiting Blocked procs when the
    /// PL011 RX ring holds bytes; the reader's post-wake drain re-checks
    /// (a spurious wake re-blocks).
    pub cons_waiting: bool,
    /// sys_sleep's wake deadline in TICKS (0 = not sleeping) —
    /// the donor tsleep's up->twhen shape; the tick's tick_timeouts
    /// readies expired sleepers.  1 tick = 1 ms this era (timer.rs
    /// TICKS_PER_SEC = 1000).
    pub sleep_until: u64,
    /// sys_alarm's expiry deadline in TICKS (0 = disarmed — the
    /// donor's up->alarm; alarm(0) clears, a fresh arm replaces).  On
    /// expiry tick_timeouts posts the "alarm" note (donor alarmkproc's
    /// postnote arm) and disarms.
    pub alarm_at: u64,
    /// The exec-recorded argv — NUL-joined, bounded at
    /// ARGV_CAP (the donor's `p->args`; the truncation rule is
    /// documented on record_argv).  exec.rs's install_image fills it at
    /// EVERY exec (the bootstrap exec_into and exec_replace paths share
    /// install_image), replacing the prior image's record; a proc that
    /// never exec'd has an EMPTY record, so devproc's /proc/N/args reads
    /// EOF, never garbage (ps.c's Noargs arm).  rfork does NOT inherit
    /// it (a fork child's args are empty until its own exec — donor
    /// newproc shape).
    pub argv: Vec<u8>,
    /// The recorded scheduling priority (devproc ctl's
    /// "pri N" — accepting + storing is the honest degradation:
    /// no scheduler priority exists this era, so nothing READS
    /// it but the /proc/N/status report).  rfork INHERITS it (donor
    /// forkchild); exec preserves it (Plan 9 convention).  0 until set.
    pub pri: u32,
    /// The donor's privatemem (CMprivate — factotum's anti-debug write,
    /// recorded, never yet enforced: the /proc mem files are a later
    /// era).
    pub privatemem: bool,
    /// The rendezvous group handle (port::rendez; 9k's Rgrp).  rfork
    /// flag semantics per fork(2)/9k sysproc.c: RFPROC WITH RFREND gives
    /// the child a FRESH empty group; WITHOUT it the child SHARES this
    /// handle (Arc clone); the non-RFPROC RFREND arm re-groups the
    /// CALLER (rfork_rendez_split).  rendezvous(34) parks/searches the
    /// group's sleeper table — measured into the dispatch set by the
    /// 06-07 tier-1 trace (hfactotum's libthread start).  The Lock is a
    /// LEAF (syscall context only, briefly; never IRQ context, dropped
    /// before the block's sched()).
    pub rgrp: Arc<Lock<RendezGroup>>,
    /// The value a rendezvous peer left for us (9k's up->rendval): the
    /// second caller by tag overwrites it before readying us; our own
    /// stored offer rides the group's sleeper entry.
    pub rendval: u64,
    /// The RFMEM family id (the donor's shared-segment group — see
    /// RFMEM_FAMILIES).  Some only after an RFMEM rfork (the parent's
    /// existing family joined, or a fresh family founded); None at
    /// construction, after exec/exit (free_user_mappings dissolves the
    /// membership), and for every non-RFMEM fork child.
    pub rfmem_family: Option<u64>,
    /// The proc's user (donor `up->user`): seeded `glenda` at
    /// construction, fork-copied, served by #c/user through the devcons
    /// hooks, and MUTATED by devcap's capuse success path (the donor's
    /// `kstrdup(&up->user, to)`).  No trailing newline (the readstr
    /// serve rule).  An owned Vec per proc — fork copies, never shares.
    pub user: Vec<u8>,
}

impl Proc {
    fn new(
        pid: u64,
        name: &str,
        kstack_top: usize,
        kstack_pas: [u64; KSTACK_PAGES],
        root_table: *mut RootPageTable,
    ) -> Proc {
        let mut proc = Proc {
            pid,
            name: [0; KNAMELEN],
            state: ProcState::Ready,
            kstack_top,
            kstack_pas,
            root_table,
            fd_group: Arc::new(Lock::new("fdgrp", FdGroup::new())),
            mapping_log: Vec::new(),
            segments: None,
            text_source: None,
            frame: ptr::null_mut(),
            context: swtch::Context {
                x19: 0,
                x20: 0,
                x21: 0,
                x22: 0,
                x23: 0,
                x24: 0,
                x25: 0,
                x26: 0,
                x27: 0,
                x28: 0,
                x29: 0,
                x30: 0,
                sp: 0,
                spsr: 0,
            },
            note_queue: VecDeque::with_capacity(NOTE_QUEUE_CAP),
            notify_handler: None,
            note_ureg: 0,
            last_note: [0; ERRMAX],
            last_note_len: 0,
            blocked_in_kernel: false,
            fpsave: crate::fpu::FpState::ZERO,
            note_fpsave: crate::fpu::FpState::ZERO,
            parent: ptr::null_mut(),
            children_live: 0,
            nowait: false,
            wait_records: VecDeque::new(),
            errstr: [0; ERRMAX],
            pgrp: Arc::new(Lock::new("pgrp", Pgrp::new())),
            // Fresh EMPTY env group at construction; the first
            // proc's SEEDED root group is assigned by exec_bootstrap
            // (the slash/dot seeding's twin), and rfork derives every
            // later group from its parent.
            env_group: Arc::new(Lock::new("env", EnvGroup::new())),
            slash: None,
            dot: None,
            // A fresh note group per proc (donor newproc's
            // incref(&noteidalloc)); rfork may overwrite with the parent's.
            noteid: NEXT_NOTEID.fetch_add(1, Ordering::Relaxed),
            cons_waiting: false,
            sleep_until: 0,
            alarm_at: 0,
            argv: Vec::new(),
            pri: 0,
            privatemem: false,
            rgrp: Arc::new(Lock::new("rendez", RendezGroup::new())),
            rendval: 0,
            rfmem_family: None,
            // The donor seeds the first proc's user from eve (k10
            // main.c: `kstrdup(&p->user, eve)`); r9's sane-default era
            // seeds glenda directly — every #c/user consumer (init's
            // readenv, devcap's eve gate) sees the same bytes the
            // const-serve era produced.
            user: b"glenda".to_vec(),
        };
        proc.set_name(name);
        proc
    }

    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let n = bytes.len().min(KNAMELEN - 1);
        self.name[..n].copy_from_slice(&bytes[..n]);
        self.name[n] = 0;
    }

    /// Record the exec argv (NUL-joined, the donor's
    /// `p->args` shape) for devproc's /proc/N/args.  Called by exec.rs's
    /// install_image at EVERY exec (both paths share it), replacing the
    /// prior image's record.  Truncation rule:
    /// args append NUL-terminated while the result stays ≤ ARGV_CAP; the
    /// first arg that would overflow ends the record (the argv tail is
    /// dropped, never half-written — except a FIRST arg longer than
    /// ARGV_CAP-1, which is byte-truncated to fit with its terminator).
    pub fn record_argv(&mut self, args: &[&[u8]]) {
        self.argv.clear();
        for (i, a) in args.iter().enumerate() {
            if self.argv.len() + a.len() + 1 > ARGV_CAP {
                if i == 0 {
                    // A single oversize first arg: keep ARGV_CAP-1 bytes.
                    self.argv.extend_from_slice(&a[..ARGV_CAP - 1]);
                    self.argv.push(0);
                }
                break;
            }
            self.argv.extend_from_slice(a);
            self.argv.push(0);
        }
    }

    /// Preopen: fd 0/1/2 are devcons chans (Plan 9 convention — 0
    /// OREAD, 1/2 OWRITE).  Called at the end of construction; the cons
    /// device itself needs no per-chan state (one console).  The chans
    /// are marked copen (they ARE open fds) — the flag's only consumer
    /// this era is devpipe's open-end counting, which cons never touches.
    fn preopen_cons_fds(&mut self) {
        use port::dat::{DEV_CONS, OREAD, OWRITE};
        // Qid path 1 = the cons file (one file this era; QTFILE).
        let cons_qid = port::dat::Qid { path: 1, vers: 0, typ: 0 };
        let node = LockNode::new();
        let mut g = self.fd_group.lock(&node);
        for (slot, mode) in [(0, OREAD), (1, OWRITE), (2, OWRITE)] {
            let mut c = Chan::new(DEV_CONS, cons_qid, mode);
            c.mark_open();
            g.fds[slot] = Some(c);
        }
    }

    // -------------------------------------------------------------------
    // fd ops — the checked-out-chan pattern (fdgrp.rs's header):
    // read/write/stat TAKE the chan out of its slot and put it back, so a
    // blocking pipe op never holds the group lock.  Take/store are move
    // semantics (no open-count effects); ref_clone is the
    // reference-creating dup (Chan::dup's pipe rule); close dispatch
    // (namec::close_chan) fires when an open chan leaves a slot.
    // -------------------------------------------------------------------

    /// Take the chan OUT of fd's slot (the checkout's removal half).
    /// None for out-of-range/closed (callers map that to errstr + -1).
    pub fn fd_take(&mut self, fd: u64) -> Option<Chan> {
        let node = LockNode::new();
        self.fd_group.lock(&node).take(fd)
    }

    /// Put a checked-out chan BACK after its op.  An occupied slot means
    /// a shared-group sibling interleaved during a blocking op (no caller
    /// this era: the only blocking op — a pipe read — has no
    /// shared-table user); the evictee is close-dispatched defensively
    /// and the anomaly is logged, loudly.
    pub fn fd_store(&mut self, fd: u64, c: Chan) {
        let node = LockNode::new();
        if let Some(evicted) = self.fd_group.lock(&node).store(fd, c) {
            println!("proc: fd {fd} occupied at write-back — closing the evicted chan");
            port::namec::close_chan(&evicted);
        }
    }

    /// A REFERENCE clone of the chan at fd (Chan::dup — an open pipe
    /// end's count bumps).  sys_dup / devsrv's store / devdup's resolve
    /// ride this.
    pub fn fd_ref_clone(&mut self, fd: u64) -> Option<Chan> {
        let node = LockNode::new();
        self.fd_group.lock(&node).ref_clone(fd)
    }

    /// The transient op-reference pair for blocking-capable syscalls
    /// (pread/pwrite): the slot keeps its chan (a shared-group sibling's
    /// concurrent op on the same fd never sees an emptied slot) and the
    /// op rides a count-free clone.  fdgrp.rs's op_ref/op_done.
    pub fn fd_op_ref(&mut self, fd: u64) -> Option<Chan> {
        let node = LockNode::new();
        self.fd_group.lock(&node).op_ref(fd)
    }

    /// The op-ref's return half (offset merge + release; never
    /// close-dispatches an emptied slot here).
    pub fn fd_op_done(&mut self, fd: u64, c: Chan) {
        let node = LockNode::new();
        self.fd_group.lock(&node).op_done(fd, c);
    }

    /// The chan's walked path at fd (sys_fd2path's source) — a read, not
    /// a checkout.
    pub fn fd_path(&mut self, fd: u64) -> Option<Vec<u8>> {
        let node = LockNode::new();
        self.fd_group.lock(&node).get(fd).map(|c| c.path.clone())
    }

    /// Install `c` at the lowest free slot.  On a full table the chan is
    /// close-dispatched HERE (an opened-but-never-installed pipe end's
    /// count must not leak) and None returns — the caller names the
    /// error ("open: no free fd" etc.).
    pub fn fd_install_lowest(&mut self, c: Chan) -> Option<u64> {
        let node = LockNode::new();
        let mut g = self.fd_group.lock(&node);
        match g.fds.iter().position(|s| s.is_none()) {
            Some(slot) => {
                g.fds[slot] = Some(c);
                Some(slot as u64)
            }
            None => {
                drop(g);
                port::namec::close_chan(&c);
                None
            }
        }
    }

    /// sys_dup's onto-arm: install `c` at `fd`, returning the EVICTED
    /// chan for the caller to close (the donor's "close after dup'ing,
    /// so date > #d/1 works" — the swap happens first).  An out-of-range
    /// target close-dispatches `c` HERE (same leak rule) and returns
    /// None (the caller names the error).
    pub fn fd_install_at(&mut self, fd: u64, c: Chan) -> Option<Option<Chan>> {
        if fd as usize >= MAX_FDS {
            port::namec::close_chan(&c);
            return None;
        }
        let node = LockNode::new();
        Some(self.fd_group.lock(&node).store(fd, c))
    }

    /// sys_pipe's pair allocation (donor newfd2): two free slots found
    /// BEFORE anything installs (a one-slot table fails the pipe without
    /// consuming an fd).
    pub fn fd_free_pair(&mut self) -> Option<(usize, usize)> {
        let node = LockNode::new();
        self.fd_group.lock(&node).free_pair()
    }

    /// Install a pair at the slots fd_free_pair found.
    pub fn fd_install_pair(&mut self, pair: (usize, usize), c0: Chan, c1: Chan) {
        let node = LockNode::new();
        let mut g = self.fd_group.lock(&node);
        g.fds[pair.0] = Some(c0);
        g.fds[pair.1] = Some(c1);
    }

    /// Set the errstr string (Plan 9 ERRMAX-bounded, NUL-terminated) — the
    /// syscall error side channel (the wire result is -1; libc's %r reads
    /// errstr, though no measured binary does so this era).
    pub fn set_errstr(&mut self, msg: &str) {
        let bytes = msg.as_bytes();
        let n = bytes.len().min(ERRMAX - 1);
        self.errstr = [0; ERRMAX];
        self.errstr[..n].copy_from_slice(&bytes[..n]);
    }

    /// Validate a user-VA range against the proc's segment map
    /// BEFORE the kernel reads or writes it.  The range must lie wholly
    /// inside ONE of the proc's segments (text/data/bss/stack/tos).  A
    /// zero-length range is trivially valid (a null buffer with n=0 is not
    /// an error).  `writable` is accepted for the signature and the
    /// future read-only-text enforcement — r9's user entries are currently
    /// AllRW for both text and data, so membership is the enforceable check
    /// this era.
    ///
    /// Caveat (single-process era): membership is a SEGMENT check — an
    /// in-segment VA whose page has not been demand-faulted in yet would
    /// take an EL1h abort if the kernel dereferenced it here (loud panic,
    /// never silent corruption).  The measured binaries' syscall buffers are
    /// always resident by call time (libc fmt read them to format them);
    /// nested demand-paging from syscall context is out of scope this era.
    pub fn validate_user_range(&self, va: u64, len: u64, _writable: bool) -> Result<()> {
        let Some(end) = va.checked_add(len) else {
            return Err("user range overflows");
        };
        if len == 0 {
            return Ok(());
        }
        let Some(segs) = &self.segments else {
            return Err("proc has no segments");
        };
        let (start, end) = (va as usize, end as usize);
        let contained = [&segs.text, &segs.data, &segs.bss, &segs.stack, &segs.tos]
            .iter()
            .any(|r| r.0.start <= start && start < end && end <= r.0.end);
        if contained { Ok(()) } else { Err("bad user address") }
    }

    /// EXEC/EXITS: unmap and free every user mapping (the
    /// mapping_log walk), then drop the segment map and the demand-page
    /// byte source.  Each leaf entry is cleared BEFORE its phys page goes
    /// back to the allocator — a stale translation pointing at a recycled
    /// page would be silent corruption.  map_to's per-write TLB invalidate
    /// covers the removal.  The caller's TTBR0 is this table (EXEC/EXITS
    /// run in the proc's syscall context), and map_to's dance edits active
    /// and inactive tables alike.
    pub fn free_user_mappings(&mut self) {
        let records = core::mem::take(&mut self.mapping_log);
        for rec in records {
            // SAFETY: self.root_table names this proc's pagealloc'd root
            // table (the construction invariant) — written through its
            // KZERO+PA alias; single CPU, IRQ-masked syscall context.
            unsafe {
                let mut physalloc = PhysPageAllocator {};
                let mut vmt = VmTraitImpl {};
                (*self.root_table)
                    .map_to(
                        &mut physalloc,
                        &mut vmt,
                        Entry::empty(),
                        rec.va,
                        PageSize::Page4K,
                        RootPageTableType::User,
                    )
                    .unwrap_or_else(|err| {
                        panic!("proc: user unmap failed at {:#x}: {err:?}", rec.va)
                    });
            }
            // RFMEM: an RFMEM-shared page's leaf is cleared above, but the
            // PHYS page goes back to the allocator only when this release
            // is the last sharer's (the donor's Segment `ref` hitting 0);
            // siblings keep mapping it until their own exit/exec.
            if release_user_page(rec.phys as u64) {
                if let Err(err) =
                    pagealloc::deallocate_physpage(PhysAddr::new(rec.phys as u64))
                {
                    println!("proc: deallocate user page {:#x}: {err:?}", rec.phys);
                }
            }
        }
        self.segments = None;
        self.text_source = None;
        // The address space the RFMEM family co-owns is gone (EXEC's
        // replacement path and EXITS both land here) — dissolve the
        // membership.  The shared pages themselves were released by the
        // refcount pivot above; the donor's exec drops the shared
        // segments the same way (the proc's new image is private).
        rfmem_leave(self);
    }

    /// EXITS: free the root-table page and the kernel-stack pages
    /// (the PAs recorded at construction — the kstack's bump-allocator VAs
    /// say nothing about their backing pages).  Runs LAST in the exit
    /// path: the handler is still executing ON the kstack and TTBR0 still
    /// names the root table when this runs — safe in the single-process
    /// era because nothing allocates afterwards (the all-exited halt
    /// follows) and the pages' contents stay intact until reuse.  The
    /// kernel-side VA mappings (KZERO+PA root alias, kstack bump VAs) are
    /// deliberately NOT torn down: nobody resolves them again.
    pub fn free_root_table_and_kstack(&mut self) {
        let root_pa = from_ptr_to_physaddr_offset_from_kzero(self.root_table);
        if let Err(err) = pagealloc::deallocate_physpage(root_pa) {
            println!("proc: deallocate root table {:#x}: {err:?}", root_pa.addr());
        }
        for pa in self.kstack_pas {
            if let Err(err) = pagealloc::deallocate_physpage(PhysAddr::new(pa)) {
                println!("proc: deallocate kstack page {pa:#x}: {err:?}");
            }
        }
        self.kstack_pas = [0; KSTACK_PAGES];
    }
}

/// The process table — a small fixed array behind the mcslock idiom.
pub struct ProcTable {
    procs: [Option<Proc>; MAX_PROCS],
    next_pid: u64,
}

pub static PROC_TABLE: Lock<ProcTable> =
    Lock::new("proc", ProcTable { procs: [const { None }; MAX_PROCS], next_pid: 1 });

/// The current-proc pointer is an AtomicPtr — the
/// single-CPU/IRQ-masked invariant below keeps the loaded pointer stable
/// for a handler's lifetime, and the atomic makes every access data-race
/// free by construction (no `static mut`).
static CURRENT_PROC: AtomicPtr<Proc> = AtomicPtr::new(ptr::null_mut());

/// The current process's raw pointer — set at exec/switch time, read in trap
/// context.  INVARIANT: single CPU; the pointer is written only in
/// process/boot context and read with IRQs masked, so it cannot change under
/// a reader.  Returns null before the first exec.
pub fn current_proc() -> *mut Proc {
    CURRENT_PROC.load(Ordering::Relaxed)
}

/// Install the current-process pointer (the exec/switch path only).
pub(crate) fn set_current_proc(p: *mut Proc) {
    CURRENT_PROC.store(p, Ordering::Relaxed)
}

/// The ONE guarded entry point for
/// dereferencing the current process — every former `&mut *current_proc()`
/// site in proc.rs/trap.rs/fault.rs/syscall.rs/main.rs goes through here.
/// The raw-pointer deref lives ONLY in ProcGuard's Deref/DerefMut impls,
/// behind the module-header invariant (single CPU; the pointer is written
/// in process/boot context and read with IRQs masked, so it cannot change
/// under a reader).  None before the first exec.
pub fn current() -> Option<ProcGuard> {
    let p = CURRENT_PROC.load(Ordering::Relaxed);
    if p.is_null() { None } else { Some(ProcGuard(p)) }
}

/// True while any non-Moribund proc exists (the EXITS path: when none
/// remain, the `all user processes exited` marker and the clean halt
/// follow — the end-of-userspace).
pub fn any_active_procs() -> bool {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table.procs.iter().flatten().any(|p| p.state != ProcState::Moribund)
}

/// 20-09 (dev_churn, the x86_64 17-02 twin): the drain probe — true
/// while any proc OTHER than `my_pid` is non-Moribund (the churn
/// driver's final wait: every nowait contention generation's
/// contenders gone before the end sample).  any_active_procs minus
/// self, the table-private-field read living here beside its sibling.
#[cfg(dev_churn)]
pub fn any_other_active_procs(my_pid: u64) -> bool {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table
        .procs
        .iter()
        .flatten()
        .any(|p| p.pid != my_pid && p.state != ProcState::Moribund)
}

/// The shared mid-construction unwind for new_user_proc — free the
/// root-table page and the already-allocated kstack pages (the caller
/// passes exactly the live prefix, kstack_pas[..n]).  The kernel-side VA
/// mappings stay, per the free_root_table_and_kstack convention.  Never
/// called with a lock held (the module discipline: the table lock is never
/// held across a pagealloc call).
fn unwind_new_proc(root_pa: PhysAddr, live_kstack_pas: &[u64], context: &'static str) {
    if let Err(err) = pagealloc::deallocate_physpage(root_pa) {
        println!("proc: {context} deallocate root table {:#x}: {err:?}", root_pa.addr());
    }
    for &pa in live_kstack_pas {
        if let Err(err) = pagealloc::deallocate_physpage(PhysAddr::new(pa)) {
            println!("proc: {context} deallocate kstack page {pa:#x}: {err:?}");
        }
    }
}

/// Allocate and register a fresh user process: pid, 16 KiB kernel stack, and
/// a per-process root page table with the recursive slot-511 self-entry.
/// The proc starts with no segments, no byte source, and an empty mapping
/// log; fd 0/1/2 are the devcons preopen (filled at construction).
///
/// Returns a ProcGuard, never a `&'static mut Proc`.
pub fn new_user_proc(name: &str) -> Result<ProcGuard> {
    // Allocate the per-process root page table first (BEFORE taking the
    // table lock — the lock is never held across a pagealloc call): one
    // pagealloc'd page, mapped at KZERO+PA in the kernel table (the
    // initrd convention).  Construction writes go through that direct
    // physical alias — the table is not active yet, and
    // init_empty_root_page_table computes the self-entry's PA from exactly
    // this KZERO-offset pointer.
    let root_pa =
        pagealloc::allocate_physpage().map_err(|_| "proc: root table page alloc failed")?;
    {
        let mut physalloc = PhysPageAllocator {};
        let mut vmt = VmTraitImpl {};
        vm::kernel_pagetable()
            .map_phys_range(
                &mut physalloc,
                &mut vmt,
                "procroot",
                &PhysRange::with_pa_len(root_pa, PAGE_SIZE_4K),
                VaMapping::Offset(KZERO),
                Entry::rw_kernel_data(),
                PageSize::Page4K,
                RootPageTableType::Kernel,
            )
            .map_err(|_| "proc: root table map failed")?;
    }
    let root_ptr = physaddr_as_ptr_mut_offset_from_kzero::<RootPageTable>(root_pa);
    // SAFETY: root_pa's KZERO+PA alias was just mapped r/w in the kernel
    // table above; the page is owned (fresh pagealloc) and gets its
    // recursive slot-511 entry before any use.
    unsafe {
        ptr::write_bytes(root_ptr, 0, 1);
        vm::init_empty_root_page_table(&mut *root_ptr);
    }

    // Kernel stack: KSTACK_PAGES contiguous kernel VAs from the kernel-VA
    // bump allocator, each backed by a fresh pagealloc page.  Nothing
    // interleaves between the bump calls in the single-threaded boot/exec
    // context; a gap would silently break the stack, so assert contiguity.
    // The PAs are recorded (kstack_pas) — the bump-allocator VAs say nothing
    // about backing pages, and the EXITS path frees by PA.
    // Every mid-construction failure funnels through unwind_new_proc
    // (root-table page + the kstack pages allocated so far) — no kernel
    // pages leak per failed rfork/exec attempt.
    let mut kstack_base = 0usize;
    let mut kstack_pas = [0u64; KSTACK_PAGES];
    for i in 0..KSTACK_PAGES {
        let va_mapping = vm::next_free_device_page4k();
        let va = va_mapping.map(PhysAddr::new(0));
        if i == 0 {
            kstack_base = va;
        } else if va != kstack_base + i * PAGE_SIZE_4K {
            unwind_new_proc(root_pa, &kstack_pas[..i], "kstack-contiguity");
            return Err("proc: kernel stack VAs not contiguous");
        }
        let pa = match pagealloc::allocate_physpage() {
            Ok(pa) => pa,
            Err(_) => {
                unwind_new_proc(root_pa, &kstack_pas[..i], "kstack-alloc");
                return Err("proc: kernel stack page alloc failed");
            }
        };
        // Record BEFORE the map so the map-failure unwind covers this page.
        kstack_pas[i] = pa.addr();
        let mut physalloc = PhysPageAllocator {};
        let mut vmt = VmTraitImpl {};
        if vm::kernel_pagetable()
            .map_phys_range(
                &mut physalloc,
                &mut vmt,
                "prockstack",
                &PhysRange::with_pa_len(pa, PAGE_SIZE_4K),
                VaMapping::Addr(va),
                Entry::rw_kernel_data(),
                PageSize::Page4K,
                RootPageTableType::Kernel,
            )
            .is_err()
        {
            unwind_new_proc(root_pa, &kstack_pas[..=i], "kstack-map");
            return Err("proc: kernel stack map failed");
        }
    }
    let kstack_top = kstack_base + KSTACK_PAGES * PAGE_SIZE_4K;

    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let pid = table.next_pid;
    table.next_pid += 1;
    let Some(slot) = table.procs.iter_mut().find(|s| s.is_none()) else {
        // The table-full path must free the already-allocated
        // root-table page and kstack pages — they are otherwise leaked
        // (reachable once real init spawns
        // grandchildren).  Drop the PROC_TABLE guard BEFORE the frees: the
        // module locking discipline never holds it across a pagealloc
        // call.  The kernel-side VA mappings stay, per the
        // free_root_table_and_kstack convention; the burned pid is
        // harmless.
        drop(table);
        unwind_new_proc(root_pa, &kstack_pas, "table-full");
        return Err("proc: table full");
    };
    let mut proc = Proc::new(pid, name, kstack_top, kstack_pas, root_ptr);
    proc.preopen_cons_fds();
    *slot = Some(proc);
    // The table is a fixed static and slots are never freed this era, so the
    // handle stays valid after the guard drops (single CPU — see the
    // module header's locking discipline).
    Ok(ProcGuard(ptr::from_mut(slot.as_mut().unwrap())))
}

/// A guard handle to a PROC_TABLE-resident Proc
/// (new_user_proc never hands out a `&'static mut`).  Deref/DerefMut
/// reach the proc; the table slots are never freed this era and handles
/// live only in boot/syscall context (single CPU, IRQ-masked), so the
/// target is stable for the guard's lifetime.
pub struct ProcGuard(*mut Proc);

impl Deref for ProcGuard {
    type Target = Proc;

    fn deref(&self) -> &Proc {
        // SAFETY: new_user_proc's invariant — the Proc lives in a static
        // PROC_TABLE slot that is never freed/moved this era.
        unsafe { &*self.0 }
    }
}

impl DerefMut for ProcGuard {
    fn deref_mut(&mut self) -> &mut Proc {
        // SAFETY: Deref's invariant; the caller holds no other live handle
        // to this proc (construction/rfork/exec orchestration context).
        unsafe { &mut *self.0 }
    }
}

// ---------------------------------------------------------------------------
// The scheduler: tick-driven preemptive round-robin over PROC_TABLE
// plus the swtch-based kernel-context path for procs blocked mid-syscall.
//
// Two resume paths, never mixed (the experiment's documented limit):
// - Ready procs with NO kernel context (fresh from rfork, or preempted at
//   EL0) resume through trap_return on their `frame` — the ONLY way EL0 is
//   entered.  The trap.S tail reloads the frame base from CURFRAME and, on
//   EL0t-bound returns, loads SP_EL1 from CURKSTACK, so both are switched
//   under the PROC_TABLE lock BEFORE the resume (the checker ordering
//   assignment: one locked step covering current-proc, CURKSTACK, CURFRAME
//   and TTBR0).
// - Procs blocked mid-syscall (AWAIT) hold a saved kernel `context` and
//   resume via swtch from the sched loop; their syscall handler then
//   returns through the normal trap-return tail (which reads the same
//   CURFRAME the locked swap installed).
//
// Time slice: the generic-timer tick is the preemption source (timer.rs's
// TICKS_PER_SEC — 100 Hz = 10 ms this era).
// ---------------------------------------------------------------------------

/// The trap-entry stack top / live frame pointer.  CURFRAME is written by
/// trap.S at every entry and read at every return (the reload guard
/// against a mid-trap preemption swap).  CURKSTACK records the invariant
/// SP_EL1 must hold at the next EL0 trap entry (the current proc's
/// kernel-stack top); the return tail reproduces it as frame+288
/// (the old explicit CURKSTACK load in trap.S needed a post-pops scratch
/// and clobbered the restored userland x9 on every EL0t return — the
/// note-timeout/wild-write flake).  CURKSTACK initialises to the legacy
/// interrupt stack so pre-process traps (early boot, virtpanic) behave
/// exactly as before processes existed.  Write discipline per the module
/// header.
///
/// SyncUnsafeCell<u64> is repr(transparent) over
/// u64 — the no_mangle symbol names AND the memory layout trap.S's
/// absolute `ldr xN, =CURKSTACK` / str-into-CURFRAME accesses link and
/// operate on are unchanged; trap.S itself is byte-untouched.
#[unsafe(no_mangle)]
pub static CURKSTACK: SyncUnsafeCell<u64> = SyncUnsafeCell::new(0);
#[unsafe(no_mangle)]
pub static CURFRAME: SyncUnsafeCell<u64> = SyncUnsafeCell::new(0);

fn read_curframe() -> *mut TrapFrame {
    // SAFETY: single CPU; CURFRAME is written only under the PROC_TABLE
    // lock or by trap.S in IRQ-masked trap context, and read under the
    // same discipline (the module header) — no concurrent access.
    unsafe { CURFRAME.get().read() as *mut TrapFrame }
}

/// The live trap frame the trap_return tail will restore (the
/// note-delivery boundary reads it): after a mid-trap preemption swap this
/// is the NEW current proc's frame, not the one the entry built — the
/// return-to-EL0 boundary must operate on the frame actually restored.
pub(crate) fn current_frame() -> *mut TrapFrame {
    read_curframe()
}

fn write_curframe(frame: *mut TrapFrame) {
    // SAFETY: read_curframe's discipline (PROC_TABLE-held or IRQ-masked
    // single-CPU context; trap.S is the only other writer, at entry).
    unsafe { CURFRAME.get().write(frame as u64) }
}

fn write_curkstack(top: usize) {
    // SAFETY: write_curframe's discipline.
    unsafe { CURKSTACK.get().write(top as u64) }
}

/// The legacy interrupt stack's top (trap.S's reservation): the pre-process
/// CURKSTACK default AND the scheduler context's stack.  `interruptstacksz`
/// is an absolute (.equ) symbol — its "address" IS the size value.
fn interrupt_stack_top() -> usize {
    unsafe extern "C" {
        static interruptstackbase: u8;
        static interruptstacksz: u8;
    }
    // SAFETY: trap.S reserves the interrupt stack and defines
    // interruptstacksz as an absolute (.equ) symbol — its "address" IS the
    // size value; only the symbols' addresses are read (the link-sym
    // class), never their contents.
    unsafe { &interruptstackbase as *const u8 as usize + &interruptstacksz as *const u8 as usize }
}

/// Initialise CURKSTACK to the legacy interrupt stack (the pre-process
/// entry-stack record; proc.rs's invariant bookkeeping — the
/// trap_return tail no longer reads it).  main9 calls this right after
/// trap::init, before IRQs can unmask.
pub fn init_trap_stacks() {
    write_curkstack(interrupt_stack_top());
}

/// First activation (exec_bootstrap's only call): publish the proc as
/// current + Running and record its kstack as the trap-entry stack — the
/// trap_return tail leaves SP_EL1 at frame+288 (= this kstack top) on the
/// EL0t-bound return into the seeded frame.
pub fn activate_first(p: &mut Proc) {
    p.state = ProcState::Running;
    set_current_proc(p);
    write_curkstack(p.kstack_top);
    // Install: the proc's user FP/SIMD state (exec zeroed it) goes live.
    crate::fpu::install(&mut p.fpsave);
}

/// 20-09 (dev_churn, the x86_64 17-02 twin): seed a freshly built
/// proc's KERNEL entry — the SCHED_CONTEXT seeding shape re-keyed onto
/// a proc.  The context resume `ret`s into `entry` on the proc's own
/// kstack (sp 16 bytes under the top, AAPCS64-aligned); no TrapFrame,
/// never EL0 — the churn driver's orchestrator context.  aarch64's
/// pick_next resumes a Ready proc through swtch only when
/// blocked_in_kernel is set (everything else trap_returns into its
/// frame, and new_user_proc leaves that frame null), so the flag is
/// what routes the driver's first pick to `entry`.
#[cfg(dev_churn)]
pub(crate) fn seed_kernel_context(p: &mut Proc, entry: fn() -> !) {
    p.context.set_return(entry as *const () as u64);
    p.context.set_stack_pointer(p.kstack_top as u64 - 16);
    p.blocked_in_kernel = true;
}

/// 20-09 (dev_churn): park the boot context into the scheduler loop —
/// the one-way handoff for the profile whose first "process" is a
/// kernel-context orchestrator (the churn driver): the boot context
/// never resumes (its save lands in a scratch Context nothing reads),
/// and sched_loop runs on the interrupt stack from here on, dispatching
/// the already-Ready driver.  The x86_64 17-02 shape.
#[cfg(dev_churn)]
pub(crate) fn enter_sched_loop_from_boot() -> ! {
    static BOOT_CTX: SyncUnsafeCell<swtch::Context> = SyncUnsafeCell::new(swtch::Context {
        x19: 0,
        x20: 0,
        x21: 0,
        x22: 0,
        x23: 0,
        x24: 0,
        x25: 0,
        x26: 0,
        x27: 0,
        x28: 0,
        x29: 0,
        x30: 0,
        sp: 0,
        spsr: 0,
    });
    // SAFETY: single CPU; runs once from the boot context (IRQ-masked,
    // driver already Ready); BOOT_CTX is never read again, and
    // sched_context()'s seed-once discipline covers the target.
    unsafe { swtch::swtch(&mut *BOOT_CTX.get(), &*sched_context()) }
    unreachable!("the boot context parked into the scheduler loop")
}

/// The timer-tick preemption entry.  Acts only when the current
/// proc is Running — i.e. the IRQ interrupted EL0 (kernel syscall/trap
/// context runs IRQ-masked; the sched loop's idle wfi finds current
/// Blocked/Moribund and its own post-wfi re-scan dispatches instead).
pub fn preempt() {
    let Some(cur) = current() else {
        return;
    };
    if cur.state != ProcState::Running {
        return;
    }
    reschedule();
}

/// Round-robin reschedule from the tick (IRQ-from-EL0: the live frame sits
/// on the current proc's kstack, recorded in CURFRAME by the trap entry).
/// On a switch: stash the live frame into the current proc, mark it Ready,
/// install the next (current + CURKSTACK + CURFRAME + TTBR0, one locked
/// step), and print the marker `sched: switch old->new` — marker-not-order
/// evidence.  The trap tail's CURFRAME reload then returns to the
/// NEW process.  Ready procs holding a saved kernel context
/// (blocked_in_kernel) are skipped here — only the sched loop can swtch
/// them back in.
pub fn reschedule() {
    // The guard's None arm replaces a bare deref (a null
    // current here was previously silent UB; unreachable in practice —
    // preempt() gates on Running).
    let Some(cur) = current() else { return };
    let cur_pid = cur.pid;
    let node = LockNode::new();
    let next_pid;
    {
        let mut table = PROC_TABLE.lock(&node);
        let Some(cur_idx) =
            table.procs.iter().position(|s| s.as_ref().is_some_and(|p| p.pid == cur_pid))
        else {
            return;
        };
        let mut pick = None;
        for off in 1..MAX_PROCS {
            let i = (cur_idx + off) % MAX_PROCS;
            let Some(p) = table.procs[i].as_ref() else { continue };
            if p.state == ProcState::Ready && !p.blocked_in_kernel {
                pick = Some(i);
                break;
            }
        }
        let Some(ni) = pick else { return };
        let curp = table.procs[cur_idx].as_mut().unwrap();
        curp.frame = read_curframe();
        curp.state = ProcState::Ready;
        // Leave: the live FP/SIMD registers are cur's user state (the
        // kernel never touches them) — park them before next's go live.
        crate::fpu::leave(&mut curp.fpsave);
        let next = table.procs[ni].as_mut().unwrap();
        next.state = ProcState::Running;
        next_pid = next.pid;
        set_current_proc(next);
        write_curkstack(next.kstack_top);
        write_curframe(next.frame);
        crate::fpu::install(&mut next.fpsave);
        // SAFETY: next's root table is fully built (the construction
        // invariant — recursive self-entry + its mappings) and named by
        // the pointer the PROC_TABLE slot owns; the switch runs with the
        // table lock held in IRQ-masked trap context.
        unsafe { vm::switch(&*next.root_table, RootPageTableType::User) };
    }
    // Outside the lock (PROC_TABLE → Console is the only nesting order);
    // still IRQ-masked trap context, so the line precedes the return eret.
    println!("sched: switch {cur_pid}->{next_pid}");
}

/// How the sched loop resumes the picked proc: swtch into its saved kernel
/// context (blocked mid-syscall) or trap_return into its frame (everything
/// else).  Raw pointers escape the PROC_TABLE guard by design — the lock is
/// never held across a swtch (the resumed proc's syscall handler may lock
/// it) and the single-CPU/IRQ-masked invariant keeps them valid.
enum Resume {
    Switch(*mut swtch::Context),
    Return(*mut TrapFrame),
}

/// The scheduler context — a kernel-context switch target running
/// sched_loop on the legacy interrupt stack (free this era — the
/// no-switch entry left it reserved for reporting only).  Seeded on first
/// use; accessed only from IRQ-masked single-CPU context.
/// SyncUnsafeCell, never `static mut`.
static SCHED_CONTEXT: SyncUnsafeCell<swtch::Context> = SyncUnsafeCell::new(swtch::Context {
    x19: 0,
    x20: 0,
    x21: 0,
    x22: 0,
    x23: 0,
    x24: 0,
    x25: 0,
    x26: 0,
    x27: 0,
    x28: 0,
    x29: 0,
    x30: 0,
    sp: 0,
    spsr: 0,
});
static SCHED_SEEDED: SyncUnsafeCell<bool> = SyncUnsafeCell::new(false);

fn sched_context() -> *mut swtch::Context {
    // SAFETY: single CPU, IRQ-masked contexts only (the module header's
    // discipline): the seed-once write and every later swtch access are
    // serialized by the caller contexts (sched/sched_from_moribund/
    // sched_loop itself) — no concurrent access is possible.
    unsafe {
        if !SCHED_SEEDED.get().read() {
            let ctx = SCHED_CONTEXT.get();
            (*ctx).set_return(sched_loop as *const () as u64);
            (*ctx).set_stack_pointer(interrupt_stack_top() as u64);
            SCHED_SEEDED.get().write(true);
        }
        SCHED_CONTEXT.get()
    }
}

/// Block the CURRENT proc mid-syscall and switch to the scheduler context
/// (the AWAIT path).  Returns when the sched loop swtch'es back —
/// i.e. after the proc was woken (Blocked → Ready by an EXITS posting a
/// wait record) and picked.  EL0 entry/exit never rides this path.
pub fn sched() {
    let mut cur = current().expect("sched with no current process");
    cur.state = ProcState::Blocked;
    cur.blocked_in_kernel = true;
    // Leave: park the user FP/SIMD state — the sched loop installs the
    // next proc's (possibly this one again, from this save).
    crate::fpu::leave(&mut cur.fpsave);
    // SAFETY: sched_context()'s seed-once discipline (single CPU,
    // IRQ-masked syscall context); cur's context field is
    // PROC_TABLE-resident (a stable address across the switch).
    unsafe {
        swtch::swtch(&mut cur.context, &*sched_context());
    }
    // Resumed inside the syscall handler; the caller re-checks its wait
    // records and returns up the trap-return tail.
}

/// The scheduler loop (entered via swtch on the legacy interrupt stack):
/// pick the next Ready proc — pick_next applies the CURKSTACK + CURFRAME +
/// TTBR0 + current-proc swap under the PROC_TABLE lock BEFORE the resume —
/// then resume it per its kind.  Nothing Ready → wfi with IRQs briefly
/// unmasked: the tick fires, re-arms, and (current not Running) skips
/// preemption; the loop then re-scans.  Stateless between scans — re-entry
/// from the seed (after a Resume::Return abandoned the loop mid-iteration)
/// is as correct as resumption after a swtch-out.
fn sched_loop() -> ! {
    // gatedbg: consecutive idle iterations with
    // a blocked cons reader while RX bytes pend, and with ALL live procs
    // blocked.  The idle arm runs between proc contexts (no syscall holds
    // Console across a sched boundary), so dumping from here follows the
    // PROC_TABLE → Console nesting order.
    // The cons-rx dump arms are cfg(dev_exec) like the pl011 cons_rx_*
    // fns they call — an unconditional arm breaks every non-dev_exec
    // profile build.
    #[cfg(dev_exec)]
    let mut cons_wedge = 0u32;
    let mut all_blocked = 0u32;
    loop {
        match pick_next() {
            // SAFETY: sched_context() is seeded (its own discipline); ctx
            // names the picked proc's PROC_TABLE-resident saved context —
            // the pointer escaped the lock under the single-CPU/IRQ-masked
            // invariant (pick_next's doc).
            Some(Resume::Switch(ctx)) => unsafe { swtch::swtch(&mut *sched_context(), &*ctx) },
            // SAFETY: frame names the picked proc's kstack-resident trap
            // frame (pick_next installed CURFRAME/CURKSTACK under the lock
            // just above); trap_return erets into it and diverges.
            Some(Resume::Return(frame)) => unsafe { crate::trap::trap_return(frame) },
            None => {
                #[cfg(dev_exec)]
                {
                    let reader_blocked = cons_reader_blocked();
                    let rx_pending = crate::platform::pl011::cons_rx_pending()
                        || crate::platform::pl011::cons_rx_fifo_pending() == Some(true);
                    if reader_blocked && rx_pending {
                        cons_wedge += 1;
                        if cons_wedge == 2000 {
                            println!("wedgedbg: idle with a blocked cons reader + pending RX — dumping");
                            dump_proc_states();
                            port::devcons::debug_dump_input();
                            let (h, t) = crate::platform::pl011::cons_rx_ring_pos();
                            println!("wedgedbg: cons ring head={} tail={} fifo_pending={:?}", h, t, crate::platform::pl011::cons_rx_fifo_pending());
                        }
                    } else {
                        cons_wedge = 0;
                    }
                }
                all_blocked = all_blocked + 1;
                if all_blocked == 200_000 {
                    println!("wedgedbg: ~100s fully idle — dumping (pipe-RPC wedge shape)");
                    dump_proc_states();
                    port::devcons::debug_dump_input();
                }
                // The consolidated idle-wait primitive
                // (briefly unmasked wfi — the tick fires and re-arms).
                crate::trap::idle_wait();
            }
        }
    }
}

/// gatedbg: any live proc Blocked in a cons read (cons_waiting).
#[cfg(dev_exec)]
fn cons_reader_blocked() -> bool {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table
        .procs
        .iter()
        .flatten()
        .any(|p| p.state == ProcState::Blocked && p.cons_waiting && p.state != ProcState::Moribund)
}

/// gatedbg: one wire line per live proc (pid, state, cons_waiting,
/// blocked_in_kernel, sleep_until, name) — the wedge autopsy.  Holds
/// PROC_TABLE while printing: PROC_TABLE → Console is the documented
/// nesting order (reschedule's switch print is the precedent).
/// pub(crate): fault.rs's unresolvable kill dumps the table too (a wild
/// read's family tree is the first post-mortem fact after the name).
pub(crate) fn dump_proc_states() {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    for p in table.procs.iter().flatten() {
        if p.state == ProcState::Moribund {
            continue;
        }
        let n = p.name.iter().position(|&b| b == 0).unwrap_or(KNAMELEN);
        let name = core::str::from_utf8(&p.name[..n]).unwrap_or("?");
        // The parent pid + the exec-recorded argv (the spawn topology
        // distinguishes a libthread proccreate child from a
        // fork+exec'd command).  SAFETY: parent names a
        // PROC_TABLE-resident Proc (recorded at rfork; slots are never
        // freed this era) and the table lock is held — the deref cannot
        // race (post_wait_record's precedent).
        let parent_pid = if p.parent.is_null() {
            0
        } else {
            unsafe { (*p.parent).pid }
        };
        let argv_str = core::str::from_utf8(&p.argv).unwrap_or("?");
        println!(
            "wedgedbg: pid={} state={} cons_wait={} blocked_in_kernel={} sleep_until={} name={} parent={} argv={}",
            p.pid,
            state_name(p.state),
            p.cons_waiting,
            p.blocked_in_kernel,
            p.sleep_until,
            name,
            parent_pid,
            argv_str
        );
    }
}

/// Round-robin pick of the next Ready proc after the current one, applying
/// the full switch as one PROC_TABLE-locked step (the ordering
/// assignment: state, current-proc pointer, CURKSTACK, CURFRAME, TTBR0) so
/// the resumed proc's trap-return tail reloads its OWN frame, never the
/// previous proc's.  None when no other Ready proc exists.
///
/// The current proc is NOT skipped — only its scan PRIORITY wraps
/// (the round-robin start is cur+1, so cur is picked last).  The old
/// skip-current rule deadlocked the cons read path: a proc that blocks
/// via sched() REMAINS the current proc while the sched loop runs, so
/// when the tick's cons_input_wake readied it (Blocked → Ready) the scan
/// skipped right past it and it never resumed.  (The pipe/await wakes
/// never tripped this because their waker runs in a DIFFERENT proc's
/// context — the wakee is not current then.)  At pick_next entry the
/// current proc is normally Blocked/Moribund (excluded by the state
/// check); a readied current is exactly the proc that must be resumable.
fn pick_next() -> Option<Resume> {
    let cur_pid = current().map(|p| p.pid);
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let cur_idx = cur_pid
        .and_then(|pid| table.procs.iter().position(|s| s.as_ref().is_some_and(|p| p.pid == pid)));
    let start = cur_idx.map(|i| i + 1).unwrap_or(0);
    for off in 0..MAX_PROCS {
        let i = (start + off) % MAX_PROCS;
        let Some(p) = table.procs[i].as_mut() else { continue };
        if p.state != ProcState::Ready {
            continue;
        }
        p.state = ProcState::Running;
        set_current_proc(p);
        write_curkstack(p.kstack_top);
        write_curframe(p.frame);
        // Install: the proc's saved user FP/SIMD state goes live (every
        // path into the sched loop parked or dropped the previous one).
        crate::fpu::install(&mut p.fpsave);
        // SAFETY: p's root table is fully built (construction invariant)
        // and owned by the table slot; the switch runs under the
        // PROC_TABLE lock in IRQ-masked context.
        unsafe { vm::switch(&*p.root_table, RootPageTableType::User) };
        return Some(if p.blocked_in_kernel {
            p.blocked_in_kernel = false;
            Resume::Switch(&mut p.context as *mut swtch::Context)
        } else {
            Resume::Return(p.frame)
        });
    }
    None
}

/// EXITS's tail: the Moribund current proc never runs again —
/// switch into the scheduler context with no wake path.  The swtch save
/// writes only the PROC_TABLE-resident context struct (never the freed
/// kstack), so EXITS can free the kstack before calling here.  Diverges.
pub fn sched_from_moribund() -> ! {
    let cur = current();
    // SAFETY: sched_context()'s seed-once discipline (single CPU,
    // IRQ-masked context); the moribund proc's context is
    // PROC_TABLE-resident and its kstack is already freed (the swtch save
    // writes only the context struct — the exit_tail contract).
    unsafe {
        let ctx = sched_context();
        if let Some(mut cur) = cur {
            // The dead proc's live FP/SIMD state is dropped, unsaved.
            crate::fpu::discard(&mut cur.fpsave);
            swtch::swtch(&mut cur.context, &*ctx);
        }
    }
    unreachable!("sched: moribund process resumed");
}

/// Non-RFPROC rfork's namespace-group half (9k sysproc.c's
/// non-RFPROC branch): the CALLER's group becomes a fresh deep COPY when
/// `copy` is set (RFNAMEG — pgrpcpy; boot(8)'s `rfork(RFNAMEG)` "leave
/// existing subprocs in own namespace") or a fresh EMPTY group otherwise
/// (RFCNAMEG — the newns wipe).  Flag validation stays in sys_rfork.
pub fn rfork_namespace_split(proc: &mut Proc, copy: bool) {
    let next = if copy {
        let node = LockNode::new();
        proc.pgrp.lock(&node).deep_copy()
    } else {
        Pgrp::new()
    };
    proc.pgrp = Arc::new(Lock::new("pgrp", next));
}

/// Non-RFPROC rfork's env-group half (9k sysproc.c's
/// non-RFPROC branch — the newns rfork(RFENVG|RFCNAMEG) shape): the
/// CALLER's group becomes a fresh deep COPY when `copy` is set (RFENVG —
/// envcpy; per-login user=/home= overrides never leak back into init's
/// group) or a fresh EMPTY group otherwise (RFCENVG).  Flag validation
/// stays in sys_rfork.
pub fn rfork_env_split(proc: &mut Proc, copy: bool) {
    let next = if copy {
        let node = LockNode::new();
        proc.env_group.lock(&node).deep_copy()
    } else {
        EnvGroup::new()
    };
    proc.env_group = Arc::new(Lock::new("env", next));
}

/// devenv's current-group hook: the CURRENT proc's
/// env-group handle — the donor devenv.c envgrp()'s `up->egrp` shape
/// (every #e op resolves against the OPERATING process's group, never a
/// group captured at attach).  None before the first proc exists (no #e
/// op can happen then).  Reads the current-proc raw pointer under the
/// module header's single-CPU/IRQ-masked invariant (syscall context).
pub fn current_env_group() -> Option<Arc<Lock<EnvGroup>>> {
    Some(current()?.env_group.clone())
}

/// devpipe's block/wake hooks (the connectembed substrate —
/// the port crate cannot see PROC_TABLE, so these are injected at boot:
/// the devproc.rs set_post_hook idiom).
///
/// The calling proc's pid (waiter registration).  Syscall context only —
/// a pipe op always has a current proc.
pub fn pipe_current_pid() -> u64 {
    current().expect("devpipe op with no current process").pid
}

/// Park the calling proc mid-syscall until a pipe write/close wakes it —
/// the AWAIT shape (mark Blocked + swtch to the scheduler context;
/// resumes when pipe_wake readies us).
pub fn pipe_block_current() {
    sched();
}

/// Wake the procs blocked on a pipe queue (devpipe's write/close calls
/// with the queue's waiter list): Blocked → Ready (blocked_in_kernel
/// stays set — the sched loop resumes them via swtch, the post_wait_record
/// pattern).  PROC_TABLE-brief.
pub fn pipe_wake(pids: &[u64]) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.state == ProcState::Blocked && pids.contains(&p.pid) {
            p.state = ProcState::Ready;
        }
    }
}

/// devmnt's RPC-contention wait (registered with port::devmnt at boot —
/// port can't see PROC_TABLE, the devpipe set_hooks idiom): park the
/// caller ~1 tick and let its enter() retry.  The 1 kHz tick's
/// tick_timeouts always readies an expired sleeper — never a spin, never
/// a lost wake, never a wedge (qlock(9)'s block-and-retry at this era's
/// granularity).  sys_sleep's shape, minus the errstr bookkeeping.  The
/// first measured concurrent caller is the 06-07 tier-1 sweep's
/// pipelines (two children exec through the shared mount session
/// concurrently).  dev_exec-only: the platform timer module exists only
/// on the exec profiles (raspi has none — and no userspace to contend).
#[cfg(dev_exec)]
pub fn mnt_contention_wait() {
    let mut cur = current().expect("mnt contention wait with no current process");
    cur.sleep_until = crate::platform::timer::ticks() + 1;
    sched();
}

/// The exit-time shared empty fd group (06-07, measured at the tier-1
/// gate): exit_tail detaches the dying proc's group by REPLACING it, and
/// the husk keeps the replacement forever (slots are never reaped this
/// era) — a FRESH 32-slot FdGroup per exit is ~6 KB of heap retained per
/// husk, and the tier-1 sweep's ~200 fork+exit cycles measured the arena
/// wall.  Every husk shares ONE empty group instead (a dead proc never
/// touches its group again, so the sharing is never observed).  The
/// DRAIN decision reads the DETACHED (old) group's strong count — the
/// replacement's sharing is invisible to it.
pub fn empty_fdgroup() -> Arc<Lock<FdGroup<MAX_FDS>>> {
    let node = LockNode::new();
    let mut guard = EMPTY_FDGROUP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("fdgrp", FdGroup::<MAX_FDS>::new())));
    }
    guard.as_ref().unwrap().clone()
}

/// The one shared empty fd group (lazily built at the first exit — a
/// Vec allocation, so not const-constructible).
static EMPTY_FDGROUP: Lock<Option<Arc<Lock<FdGroup<MAX_FDS>>>>> = Lock::new("emptyfdgrp", None);

/// The exit sweep's fd-group half (the donor pexit's closefgrp): swap the
/// dying proc's group in `slot` for `empty`, and only when this proc was
/// the group's LAST sharer drain every occupied slot and hand each chan
/// to `close` (exit_tail passes namec::close_chan — devpipe's open-count
/// decrement, devmnt's clunk_if_last).  The close dispatch runs AFTER
/// the group lock drops (the fdgrp leaf-lock rule; a close can block).
/// A non-last sharer only gives up its reference.
///
/// A RETURNING function on purpose: exit_tail diverges (park_forever /
/// sched_from_moribund), so a local it holds is never dropped.  The
/// detached Arc used to live there, so every non-last sharer's exit
/// leaked one strong reference: a group shared through a plain
/// rfork(RFPROC) then still read count 2 at its LAST sharer's exit, the
/// drain never ran, and a pipe write end in it stayed open forever (the
/// Harvey 20-07 `{rfdprobe -s} reader never saw EOF).  Here every local
/// drops at the return, before the tail diverges.  (x86_64's exit_tail
/// drops it explicitly — r9 c4f2f2c, the #44 fix.)
///
/// The drained chans are consumed ONE at a time (x86_64 ab0a977, 19-10
/// cont 6): each drops before the next close, so a later duplicate of
/// the same server fid sees the earlier one gone.  A close under a
/// BORROW of the whole batch pinned every duplicate, so devmnt's
/// clunk_if_last (a strong-count test on the fid Arc) never saw the last
/// reference and the Tclunk never fired — the donor closefgrp nils each
/// slot before its cclose.
pub(crate) fn release_fd_group(
    slot: &mut Arc<Lock<FdGroup<MAX_FDS>>>,
    empty: Arc<Lock<FdGroup<MAX_FDS>>>,
    mut close: impl FnMut(&Chan),
) {
    let detached = core::mem::replace(slot, empty);
    if Arc::strong_count(&detached) == 1 {
        let mut drained = {
            let node = LockNode::new();
            detached.lock(&node).drain_all()
        };
        for c in drained.drain(..) {
            close(&c);
            drop(c);
        }
    }
}

/// The non-RFPROC rfork fd-group split (donor sysproc.c: `ofg =
/// up->fgrp; up->fgrp = dupfgrp(copy ? ofg : nil); closefgrp(ofg)`):
/// re-group the CALLER in place — `copy` (RFFDG) gives it a private
/// slot-by-slot copy, otherwise (RFCFDG) a FRESH empty table — then
/// release the old handle exactly as an exit does (release_fd_group:
/// the last holder drains and close-dispatches it; a shared table just
/// loses this reference).
///
/// The fresh table is never empty_fdgroup(): that singleton is the exit
/// husks' shared replacement, so a LIVE proc handed it would install
/// its later fds in a table every husk and every other RFCFDG caller
/// shares, and no exit would ever drain it (its count is never 1).
/// And the old handle is never just dropped: a private table's chans
/// would then skip close_chan — after RFFDG every pipe end keeps the
/// copy's extra open count (its peer never sees hangup), after RFCFDG
/// none of them closes at all.
pub(crate) fn split_fd_group_in_place(
    slot: &mut Arc<Lock<FdGroup<MAX_FDS>>>,
    copy: bool,
    close: impl FnMut(&Chan),
) {
    let fresh = if copy {
        let node = LockNode::new();
        let copied = slot.lock(&node).deep_copy();
        Arc::new(Lock::new("fdgrp", copied))
    } else {
        Arc::new(Lock::new("fdgrp", FdGroup::new()))
    };
    release_fd_group(slot, fresh, close);
}

/// The exit sweep's shared EMPTY pgrp (x86_64 c4f2f2c, the #44 fix —
/// empty_fdgroup's shape).  Slots are never reaped this era, so every
/// group handle a Moribund husk pins is a permanent allocation; the sweep
/// swaps each for a singleton clone (a refcount bump) and lets the old
/// Arc drop — a last sharer's group dies with it.  Husks only: a live
/// proc is never handed one (the fresh-table rule of
/// split_fd_group_in_place).
pub fn empty_pgrp() -> Arc<Lock<Pgrp>> {
    let node = LockNode::new();
    let mut guard = EMPTY_PGRP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("pgrp", Pgrp::new())));
    }
    guard.as_ref().unwrap().clone()
}

static EMPTY_PGRP: Lock<Option<Arc<Lock<Pgrp>>>> = Lock::new("emptypgrp", None);

/// The exit sweep's shared EMPTY env group (husks only; see empty_pgrp).
pub fn empty_env_group() -> Arc<Lock<EnvGroup>> {
    let node = LockNode::new();
    let mut guard = EMPTY_ENVGROUP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("env", EnvGroup::new())));
    }
    guard.as_ref().unwrap().clone()
}

static EMPTY_ENVGROUP: Lock<Option<Arc<Lock<EnvGroup>>>> = Lock::new("emptyenvgrp", None);

/// The exit sweep's shared EMPTY rendezvous group (husks only; see
/// empty_pgrp).
pub fn empty_rendez_group() -> Arc<Lock<RendezGroup>> {
    let node = LockNode::new();
    let mut guard = EMPTY_RENDEZGROUP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("rendez", RendezGroup::new())));
    }
    guard.as_ref().unwrap().clone()
}

static EMPTY_RENDEZGROUP: Lock<Option<Arc<Lock<RendezGroup>>>> = Lock::new("emptyrgrp", None);

/// The donor closepgrp, mount half included (x86_64 ab0a977, 19-10
/// cont 6): swap the namespace group in `slot` for `fresh` and, only
/// when this holder had the LAST reference, close-dispatch every
/// mount-table chan (each point's `from` and its whole chain) one at a
/// time, outside the group lock (a Tclunk can park).  A plain Arc drop
/// never Tclunks them: every mounted tree's root/attach fid leaks
/// server-side (rio's wsys attach carries a window reference, so a
/// window whose shell exits never deletes).
pub(crate) fn release_pgrp(slot: &mut Arc<Lock<Pgrp>>, fresh: Arc<Lock<Pgrp>>, mut close: impl FnMut(&Chan)) {
    let detached = core::mem::replace(slot, fresh);
    if Arc::strong_count(&detached) == 1 {
        let chans = {
            let node = LockNode::new();
            detached.lock(&node).take_mount_chans()
        };
        for c in chans {
            close(&c);
        }
    }
}

/// The #44 husk sweep (x86_64 c4f2f2c, 72edbba, ab0a977): a Moribund
/// husk pins its slot forever, so every heap payload it keeps is a
/// permanent allocation.  The donor pexit's closepgrp/closeegrp/
/// closergrp re-keyed as singleton swaps — the old Arcs drop here, a
/// last sharer's group dies with it, and a last namespace holder's mount
/// table close-dispatches first (release_pgrp).  slash/dot DROP without
/// the close dispatch (72edbba): a devmnt clunk of a husk's root/dot
/// clone mid-exit measured userland pool corruption on x86_64's mounted
/// venue, so they release by plain drop (the server-side fid of the
/// clone leaks until the server dies — the documented era shape).
pub fn sweep_husk_payloads(proc: &mut Proc) {
    release_pgrp(&mut proc.pgrp, empty_pgrp(), port::namec::close_chan);
    proc.env_group = empty_env_group();
    proc.rgrp = empty_rendez_group();
    let _ = proc.slash.take();
    let _ = proc.dot.take();
}


/// devproc-lite's write hook: append the note to the
/// ADDRESSED pid's queue — no broadcast, no group semantics.  The write is
/// the whole posting path (libc postnote(PNPROC) lands here via
/// /proc/N/note); delivery happens at the target's next return-to-EL0
/// boundary (trap.rs's deliver_notes).  PROC_TABLE-brief.  Registered with
/// port::devproc at boot (main.rs under dev_exec).
pub fn post_note(pid: u64, msg: &[u8]) -> Result<()> {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let Some(p) = table.procs.iter_mut().flatten().find(|p| p.pid == pid) else {
        return Err("devproc: no such process");
    };
    push_note(p, msg)
}

// ---------------------------------------------------------------------------
// devproc's proc-table hooks — the ps(1) data source
// (list/info) and init's ctl priority record (set_pri).  The note-post
// hook's siblings; registered with port::devproc at boot (main.rs under
// dev_exec).  All PROC_TABLE-brief.
// ---------------------------------------------------------------------------

/// The donor statename string for a ProcState (9k devproc.c's statename[]
/// subset — r9's ProcState has exactly these four variants).
fn state_name(s: ProcState) -> &'static str {
    match s {
        ProcState::Ready => "Ready",
        ProcState::Running => "Running",
        ProcState::Blocked => "Blocked",
        ProcState::Moribund => "Moribund",
    }
}

/// devproc's root-listing hook: the LIVE (non-Moribund) pids,
/// ascending by table slot (ps's dirreadall content).  Moribund slots
/// stay pinned this era (the exit-sweep deferral) but are dead —
/// the donor's dir listing skips Dead procs the same way.
pub fn proc_list_live() -> Vec<u64> {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table
        .procs
        .iter()
        .flatten()
        .filter(|p| p.state != ProcState::Moribund)
        .map(|p| p.pid)
        .collect()
}

/// devproc's per-pid info hook: the status/args data — name,
/// donor-statename state, the exec-recorded argv (NUL-joined; empty for
/// a never-exec'd proc), the recorded ctl priority.  None for a missing
/// or Moribund pid (the donor's Eprocdied shape — devproc maps it to
/// "no such process").
pub fn proc_info(pid: u64) -> Option<port::devproc::ProcInfo> {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    let p = table.procs.iter().flatten().find(|p| p.pid == pid)?;
    if p.state == ProcState::Moribund {
        return None;
    }
    let name_len = p.name.iter().position(|&b| b == 0).unwrap_or(KNAMELEN);
    Some(port::devproc::ProcInfo {
        name: p.name[..name_len].to_vec(),
        state: state_name(p.state),
        args: p.argv.clone(),
        pri: p.pri,
    })
}

/// devproc's ctl hook: record the "pri N" write on the ADDRESSED
/// pid (write-side per-pid resolution — no broadcast).
/// Accepting + storing is the honest degradation: no scheduler
/// priority exists this era.  Missing/Moribund → the loud error.
pub fn proc_set_pri(pid: u64, pri: u32) -> Result<()> {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let Some(p) = table.procs.iter_mut().flatten().find(|p| p.pid == pid) else {
        return Err("devproc: no such process");
    };
    if p.state == ProcState::Moribund {
        return Err("devproc: no such process");
    }
    p.pri = pri;
    Ok(())
}

/// devproc's ctl hook: record the "private" write on the ADDRESSED pid
/// (the donor CMprivate's p->privatemem = 1) — the x86_64 sibling's
/// shape, so the port hook surface stays arch-uniform.
pub fn proc_set_private(pid: u64) -> Result<()> {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let Some(p) = table.procs.iter_mut().flatten().find(|p| p.pid == pid) else {
        return Err("devproc: no such process");
    };
    if p.state == ProcState::Moribund {
        return Err("devproc: no such process");
    }
    p.privatemem = true;
    Ok(())
}

/// The note-queue append shared by post_note (per-pid) and the
/// note-group post: Moribund targets and full queues are skipped per the
/// donor's postnote failure shape (the caller's group post ignores
/// individual failures — pgrpnote's waserror skip).  A Blocked target is
/// READIED (the donor postnote's ready(p)): the note must reach a proc
/// parked in an indefinite device read — the read's loop re-runs on
/// wake, sees the pending note, and errors "interrupted" (the
/// note-interruptible blocking-IO contract; without the wake, a kill
/// note to a parked reader sits queued forever and the reader's session
/// never tears down — the 19-08 session-exit variance).
fn push_note(p: &mut Proc, msg: &[u8]) -> Result<()> {
    if p.state == ProcState::Moribund {
        return Err("devproc: no such process");
    }
    if p.note_queue.len() >= NOTE_QUEUE_CAP {
        return Err("devproc: note queue full");
    }
    let mut note = Note { buf: [0; ERRMAX], len: msg.len().min(ERRMAX - 1) };
    note.buf[..note.len].copy_from_slice(&msg[..note.len]);
    p.note_queue.push_back(note);
    if p.state == ProcState::Blocked {
        p.sleep_until = 0;
        p.state = ProcState::Ready;
    }
    Ok(())
}

/// The current proc has a queued note (the blocking-IO loops' interrupt
/// probe — the donor's `if(!notepending())` around every sleep).
pub fn note_pending() -> bool {
    match current() {
        Some(p) => !p.note_queue.is_empty(),
        None => false,
    }
}

/// devcons's DEL path — post a note to the note GROUPS of
/// the given reader pids (the console's foreground group this era — the
/// sanctioned scoping for the donor's notepg machinery: "the note
/// group of the proc(s) holding cons open").  Reader pids resolve to
/// their noteids; every live proc sharing one gets the note (the donor's
/// pgrpnote loop, 9k pgrp.c:16).  No caller exclusion: the poster is the
/// console, not a proc.  PROC_TABLE-brief; registered with port::devcons
/// at boot (main.rs under dev_exec).
pub fn post_note_group_of_members(readers: &[u64], msg: &[u8]) {
    let node = LockNode::new();
    let mut groups: Vec<u64> = Vec::new();
    {
        let table = PROC_TABLE.lock(&node);
        for p in table.procs.iter().flatten() {
            if readers.contains(&p.pid)
                && p.state != ProcState::Moribund
                && !groups.contains(&p.noteid)
            {
                groups.push(p.noteid);
            }
        }
    }
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if groups.contains(&p.noteid) {
            let _ = push_note(p, msg);
        }
    }
}

/// The calling proc's pid (devcons's `current` hook — the generic
/// twin of pipe_current_pid, which stays named for devpipe's registration
/// site).  Syscall context only — a device op always has a current proc.
pub fn current_pid() -> u64 {
    current().expect("device op with no current process").pid
}

/// The calling proc's user bytes (devcons's UserHooks.current — the
/// #c/user serve, devcap's eve gate and user-match read).
pub fn current_user() -> Vec<u8> {
    current().expect("user read with no current process").user.clone()
}

/// Mutate the calling proc's user (devcons's UserHooks.set — devcap
/// capuse's success path, the donor's `kstrdup(&up->user, to)`).
pub fn set_current_user(user: Vec<u8>) {
    current().expect("user set with no current process").user = user;
}

/// devcons's block hook: park the calling proc in its cons
/// read until the tick's cons_input_wake readies it (input pending).  The
/// register-then-block order cannot lose a wake: syscalls are serialized
/// (single CPU, IRQ-masked), so the tick fires only after sched() swtch'es
/// (devpipe's documented invariant).  The flag clears on resume — the
/// reader's post-wake drain re-checks the ring regardless (a spurious
/// wake re-blocks).
pub fn cons_block_current() {
    let mut cur = current().expect("cons read with no current process");
    cur.cons_waiting = true;
    sched();
    // Resumed inside the syscall handler by the sched loop's swtch; this
    // proc is current again (the blocked_in_kernel resume path).
    cur.cons_waiting = false;
}

/// The tick's cons wake: ready every cons-waiting Blocked proc —
/// the caller (timer.rs) has already seen the PL011 RX ring non-empty via
/// the lock-free peek.  PROC_TABLE from tick (trap) context is the
/// reschedule() precedent: the lock is only ever held in IRQ-masked
/// context (the module header's discipline).
pub fn cons_input_wake() {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.state == ProcState::Blocked && p.cons_waiting {
            p.state = ProcState::Ready;
        }
    }
}

/// Non-RFPROC rfork(RFNOTEG)'s note-group half (9k sysproc.c:82 —
/// `if(flag & RFNOTEG) up->noteid = incref(&noteidalloc)`): the CALLER
/// joins a fresh note group.  init's fexec child does this before exec'ing
/// rc, so a console DEL interrupts the shell's group and never init.
pub fn rfork_noteg_split(proc: &mut Proc) {
    proc.noteid = NEXT_NOTEID.fetch_add(1, Ordering::Relaxed);
}

/// Non-RFPROC rfork(RFREND)'s rendezvous-group half (9k sysproc.c:69-72
/// — `if(flag & RFREND) { org = up->rgrp; up->rgrp = newrgrp(); ... }`):
/// the CALLER joins a fresh empty group.
pub fn rfork_rendez_split(proc: &mut Proc) {
    proc.rgrp = Arc::new(Lock::new("rendez", RendezGroup::new()));
}

/// The rendezvous wake half (9k sysrendezvous's found arm): the sleeper
/// named by `pid` is handed the waker's value (its sched() returns it via
/// proc.rendval) and readied.  Caller MUST already have taken the sleeper
/// entry off its group.  The PROC_TABLE idiom (post_wait_record's shape):
/// table-resident procs, briefly locked, never from IRQ context (the
/// syscall handler is the only caller).
///
/// No 9k `while(p->mach != 0)` spin: the sleeper called proc::sched()
/// BEFORE the waker's syscall could run (syscalls are serialized —
/// single CPU, IRQ-masked, preemption only at EL0), so by the time any
/// waker executes the sleeper is fully switched out.
pub fn rendez_wake(pid: u64, value: u64) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let Some(p) = table.procs.iter_mut().flatten().find(|p| p.pid == pid) else {
        return; // died between the park and the wake (the exit sweep removes it)
    };
    p.rendval = value;
    if p.state == ProcState::Blocked {
        p.state = ProcState::Ready;
    }
}

/// Exit-time rendezvous sweep (9k's pexit removes the proc from the
/// rendezvous hash the same way): a killed/exited sleeper must never
/// linger as a wake target in its group.
pub fn rendez_remove(proc: &mut Proc) {
    let rgrp = proc.rgrp.clone();
    let node = LockNode::new();
    rgrp.lock(&node).remove_pid(proc.pid);
}

/// The tick's timeout sweep (the donor's tsleep wakeup + alarm.c's
/// alarmkproc expiry arm, plus the note-interruptible sleep):
/// per proc, the ALARM expiry arm runs FIRST (posting the "alarm" note
/// and disarming — the donor's `rp->alarm = 0`), THEN the sleep wake arm
/// readies a Blocked sleeper whose deadline passed OR whose note_queue is
/// non-empty (a posted note interrupts the sleep — the donor tsleep
/// shape; the sleeper's sys_sleep discriminates early-wake-vs-expiry
/// against its local deadline copy, so this arm's alarm-first ordering
/// makes an alarm fired at the same tick interrupt a sleep that would
/// otherwise expire).  sleep_until clears either way.  Notes append via
/// push_note (Moribund/full-queue skips are the donor pgrpnote's
/// waserror shape); delivery rides the return-to-EL0 boundary
/// unchanged.  PROC_TABLE from tick (trap) context — the reschedule()
/// precedent (the module header's locking discipline).
pub fn tick_timeouts(now: u64) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.alarm_at != 0 && now >= p.alarm_at {
            p.alarm_at = 0;
            let _ = push_note(p, b"alarm");
        }
        if p.state == ProcState::Blocked
            && p.sleep_until != 0
            && (now >= p.sleep_until || !p.note_queue.is_empty())
        {
            p.sleep_until = 0;
            p.state = ProcState::Ready;
        }
    }
}

/// devdup/devsrv's fd resolver hook: the CALLING proc's fd N as
/// a reference clone (the donor fdtochan incref shape — a pipe end's
/// open count bumps, so the resolved chan is a real second reference the
/// close dispatch balances).  None before the first proc exists.
pub fn current_fd_resolve(fd: u64) -> Option<Chan> {
    current()?.fd_ref_clone(fd)
}

/// devdup's listing hook: the CALLING proc's open fds as
/// (fd, mode) pairs (the `#d` root directory read).
pub fn current_fd_list() -> Vec<(u64, u32)> {
    let Some(p) = current() else {
        return Vec::new();
    };
    let node = LockNode::new();
    p.fd_group.lock(&node).list_open()
}

/// EXITS: post the child's wait record to the parent and wake it
/// if it sits Blocked in AWAIT (Blocked → Ready; blocked_in_kernel stays
/// set — the parent resumes through the sched loop's swtch).  The record
/// is kernel memory in the parent's table slot (no cross-process
/// pointers).  PROC_TABLE-brief.
pub fn post_wait_record(parent: *mut Proc, pid: u64, msg: &[u8]) {
    if parent.is_null() {
        return;
    }
    let node = LockNode::new();
    // Held for the posting + wake (the parent pointer is table-resident).
    let _guard = PROC_TABLE.lock(&node);
    // SAFETY: parent names a PROC_TABLE-resident Proc (recorded at rfork;
    // slots are never freed this era) and the table lock is held — the
    // deref cannot race (single CPU, IRQ-masked syscall context).
    let parent = unsafe { &mut *parent };
    let mut rec = WaitRecord { pid, msg: [0; ERRMAX], msg_len: msg.len().min(ERRMAX) };
    rec.msg[..rec.msg_len].copy_from_slice(&msg[..rec.msg_len]);
    parent.wait_records.push_back(rec);
    if parent.state == ProcState::Blocked {
        parent.state = ProcState::Ready;
    }
}

/// RFORK: clone `parent` — a fresh proc (new kernel stack + root
/// table via new_user_proc), the address space per the donor dupseg
/// shape (an EAGER page-for-page copy without RFMEM; with RFMEM the
/// Data/Bss pages SHARE the parent's physical pages, refcounted in
/// SHARED_PAGES — Stack/Tos/Text stay fresh copies), a dup'd fd table
/// (fresh Chan values naming the same files/devices), mirrored segment
/// records + paqfs byte source, and a copy of the parent's live trap frame
/// on the child's kstack with x0 = 0 (donor forkchild).  The child is
/// queued Ready; its first run erets through the CURFRAME/trap_return
/// path.  Returns the child pid (the parent's wire result).
///
/// The namespace group + slash/dot inheritance per fork(2)
/// and 9k sysproc.c sysrfork — RFPROC with NEITHER RFNAMEG nor RFCNAMEG
/// SHARES the parent's pgrp (Arc clone); RFNAMEG gives the child a fresh
/// COPY (pgrpcpy); RFCNAMEG gives the child a fresh EMPTY group (the
/// newns wipe).  slash/dot are value-dup'd (donor incref shape).
pub fn rfork_clone(parent: &mut Proc, frame: &TrapFrame, flag: u64) -> Result<u64> {
    let nowait = flag & crate::syscall::RFNOWAIT != 0;
    let name = {
        let n = parent.name.iter().position(|&b| b == 0).unwrap_or(KNAMELEN);
        core::str::from_utf8(&parent.name[..n]).unwrap_or("fork")
    };
    let mut child = new_user_proc(name)?;

    // Everything from here is fallible, and the child is already
    // PROC_TABLE-resident (new_user_proc inserts it Ready with a null
    // frame).  A mid-copy Err that just returned would leave the
    // half-built child Runnable in the table — the scheduler's pick_next
    // would eventually pick it and trap_return on CURFRAME = 0 (an EL1h
    // abort from an ordinary OOM) — and leak its pages.  Unwind instead:
    // mark Moribund, free the pages already copied, then the root-table
    // page and kstack by their recorded PAs.
    if let Err(msg) = rfork_copy(parent, &mut child, frame, flag) {
        child.state = ProcState::Moribund;
        // The copied pages' child-table entries die with the root-table
        // page below; the kernel-side KZERO+PA aliases stay mapped, per
        // the free_root_table_and_kstack convention.  The release pivot
        // keeps RFMEM-shared pages alive (the parent still holds its
        // share) while the fresh copies free as before.
        for rec in core::mem::take(&mut child.mapping_log) {
            if release_user_page(rec.phys as u64) {
                if let Err(err) = pagealloc::deallocate_physpage(PhysAddr::new(rec.phys as u64)) {
                    println!("rfork: unwind deallocate user page {:#x}: {err:?}", rec.phys);
                }
            }
        }
        child.free_root_table_and_kstack();
        return Err(msg);
    }
    // The copy succeeded — under RFMEM the pair now co-owns the shared
    // Data/Bss pages: record the family membership (the donor's shared
    // segment GROUP) so brk_'s family arm keeps the members' breaks and
    // mappings identical from here on.
    if flag & crate::syscall::RFMEM != 0 {
        rfmem_join(parent, &mut child);
    }
    // The child starts with the parent's user FP/SIMD state — the live
    // registers when the parent owns them (it is on the CPU in its own
    // rfork syscall), its fpsave otherwise; the child is off the CPU, so
    // the copy lands in its fpsave, which its first install restores.
    child.fpsave = crate::fpu::snapshot(&parent.fpsave);
    // An RFNOWAIT child posts no wait record at EXITS and is never
    // reaped by AWAIT, so it must not count toward the parent's AWAIT
    // block-vs-"no children" pivot (Plan 9: an RFNOWAIT child is invisible
    // to pwait).  Counting it left children_live elevated for the rest of
    // the parent's life; the parent's next AWAIT with no queued records
    // then blocked in sched() with nothing left to ever wake it.
    if !nowait {
        parent.children_live += 1;
    }
    Ok(child.pid)
}

/// The fallible body of rfork_clone after the child exists
/// (rfork_clone unwinds the half-built child on any Err): an EAGER
/// page-for-page copy of every user mapping, a dup'd fd table, mirrored
/// segment records + paqfs byte source, and a copy of the parent's live
/// trap frame on the child's kstack with x0 = 0 (donor forkchild).
fn rfork_copy(parent: &mut Proc, child: &mut Proc, frame: &TrapFrame, flag: u64) -> Result<()> {
    let nowait = flag & crate::syscall::RFNOWAIT != 0;
    let rfmem = flag & crate::syscall::RFMEM != 0;
    // The eager copy, with the donor dupseg shape (sys/src/9/port/
    // segment.c) under RFMEM: SG_STACK is ALWAYS a fresh copy (never
    // shared — note delivery and the libc Tos machinery write the
    // proc's OWN stack pages); SG_BSS/SG_DATA SHARE under RFMEM — the
    // same PA mapped in both procs' root tables, refcounted in
    // SHARED_PAGES against the exit/exec frees.  Text and Tos keep this
    // era's eager copy (the donor shares SG_TEXT always; r9's proven
    // copy behavior stands — ping's reader/sender pair shares only its
    // data/bss).  KNOWN LIMIT (documented in the 08-04 record): a page
    // brk_-grown or demand-faulted by one sharer AFTER the fork is not
    // visible to the other — ping's request list lives in the
    // fork-time-shared pages (its malloc arena grew during dial(),
    // before the rfork), so the gate never crosses the limit; full
    // demand-page sharing (a shared page-map structure, the donor's
    // ptecpy-into-a-shared-segment shape) is a later refinement.
    let records: Vec<MappingRecord> = parent.mapping_log.clone();
    // Every page copy below goes through the ONE copy path, which owes
    // the I-cache sync for copied text (textcopy.rs: WINDOWS #49's
    // residual — the boot-time selftest guards it on real hardware).
    let mut text_copy = crate::textcopy::TextCopy::new();
    for rec in &records {
        if rfmem && matches!(rec.kind, SegKind::Data | SegKind::Bss) {
            // The RFMEM share arm: no copy, no kernel alias — the
            // parent's PA maps straight into the child's INACTIVE root
            // table at the same VA (map_to's temp-recursive dance edits
            // inactive tables), then the refcount records the second
            // sharer.  The parent's mapping stays exactly as it was.
            let entry =
                Entry::rw_user_data().with_phys_addr(PhysAddr::new(rec.phys as u64));
            // SAFETY: child.root_table names the child's pagealloc'd root
            // table (the construction invariant) — an INACTIVE table edited
            // through its KZERO+PA alias via map_to's temp-recursive dance;
            // single CPU, IRQ-masked syscall context.  The shared page's PA
            // is the parent's live page — refcounted below before any free
            // path can see the child's record.
            unsafe {
                let mut physalloc = PhysPageAllocator {};
                let mut vmt = VmTraitImpl {};
                (*child.root_table)
                    .map_to(
                        &mut physalloc,
                        &mut vmt,
                        entry,
                        rec.va,
                        PageSize::Page4K,
                        RootPageTableType::User,
                    )
                    .map_err(|_| "rfork: child shared map failed")?;
            }
            share_user_page(rec.phys as u64);
            child.mapping_log.push(MappingRecord {
                va: rec.va,
                phys: rec.phys,
                size: rec.size,
                kind: rec.kind,
            });
            continue;
        }
        // The copy arm (always for Text/Stack/Tos; for Data/Bss without
        // RFMEM): each parent page's bytes go through the pages' KZERO+PA
        // kernel aliases (map_to REPLACES existing entries, so re-mapping
        // the exec/fault-side alias is idempotent); the child's fresh page
        // maps into its INACTIVE root table at the same VA with the same
        // entry kind.  The parent's mapping_log is stable through the loop:
        // single CPU, the parent is current, IRQs are masked in syscall
        // context.
        let pa = pagealloc::allocate_physpage().map_err(|_| "rfork: page alloc failed")?;
        map_kernel_alias(rec.phys as u64)?;
        map_kernel_alias(pa.addr())?;
        // SAFETY: both pages' KZERO+PA aliases were just (re)mapped r/w in
        // the kernel table above; src names the parent's live page, dst
        // the child's fresh page — one full page copied, non-overlapping.
        // A text page is new instructions written through a DATA alias:
        // copy_page cleans it to PoU and arms the one I-cache sync.
        unsafe {
            let src = physaddr_as_ptr_mut_offset_from_kzero::<u8>(PhysAddr::new(rec.phys as u64));
            let dst = physaddr_as_ptr_mut_offset_from_kzero::<u8>(pa);
            text_copy.copy_page(src, dst, rec.kind == SegKind::Text);
        }
        // Text is UXN-clear/PXN-set; everything else rw_user_data — the
        // same kind→entry mapping fault.rs's page-in uses.
        let entry = match rec.kind {
            SegKind::Text => Entry::rw_user_text(),
            _ => Entry::rw_user_data(),
        }
        .with_phys_addr(pa);
        // SAFETY: child.root_table names the child's pagealloc'd root
        // table (the construction invariant) — an INACTIVE table edited
        // through its KZERO+PA alias via map_to's temp-recursive dance;
        // single CPU, IRQ-masked syscall context.
        unsafe {
            let mut physalloc = PhysPageAllocator {};
            let mut vmt = VmTraitImpl {};
            (*child.root_table)
                .map_to(
                    &mut physalloc,
                    &mut vmt,
                    entry,
                    rec.va,
                    PageSize::Page4K,
                    RootPageTableType::User,
                )
                .map_err(|_| "rfork: child user map failed")?;
        }
        child.mapping_log.push(MappingRecord {
            va: rec.va,
            phys: pa.addr() as usize,
            size: rec.size,
            kind: rec.kind,
        });
    }
    // Discharge the I-cache sync before the child can run any copied
    // text (textcopy.rs carries the why: a recycled page keeps the old
    // owner's instructions for the same VA on the HVF hosts).  An early
    // return above discharges it at drop.
    text_copy.finish();

    // The bss sweep (RFMEM only): the log walk above shares only PRESENT
    // pages, but the parent's bss range can cover pages brk_-grown and
    // never touched (exec maps data+bss eagerly; brk_ extends lazily).
    // Left alone, each proc would fault such a page in PRIVATELY —
    // silent divergence where the donor's shared segment shows both
    // procs the same page.  Materialize each absent in-range page now as
    // ONE zeroed page mapped into BOTH tables (the parent's ACTIVE table
    // and the child's INACTIVE one — map_to handles both), closing the
    // hole by construction; from here on the family's bss grows only
    // through sys_brk_'s family arm.
    if rfmem {
        let sweep = parent.segments.as_ref().map(|segs| {
            (segs.bss.0.start & !(PAGE_SIZE_4K - 1), segs.bss.0.end.next_multiple_of(PAGE_SIZE_4K))
        });
        if let Some((start, end)) = sweep {
            let mut va = start;
            while va < end {
                if parent.mapping_log.iter().any(|m| m.va == va) {
                    va += PAGE_SIZE_4K;
                    continue;
                }
                let pa =
                    pagealloc::allocate_physpage().map_err(|_| "rfork: sweep page alloc failed")?;
                map_kernel_alias(pa.addr())?;
                // SAFETY: the page's KZERO+PA alias was just (re)mapped r/w
                // in the kernel table above; one full fresh page, owned.
                unsafe {
                    ptr::write_bytes(physaddr_as_ptr_mut_offset_from_kzero::<u8>(pa), 0, PAGE_SIZE_4K);
                }
                let entry = Entry::rw_user_data().with_phys_addr(pa);
                // SAFETY (both map_to calls): parent.root_table is the
                // parent's ACTIVE user table (map_to's per-write TLB
                // invalidate covers the addition — the VA was unmapped, so
                // no stale translation); child.root_table is INACTIVE and
                // edited via the temp-recursive dance.  Single CPU,
                // IRQ-masked syscall context throughout.
                unsafe {
                    let mut physalloc = PhysPageAllocator {};
                    let mut vmt = VmTraitImpl {};
                    if (*parent.root_table)
                        .map_to(
                            &mut physalloc,
                            &mut vmt,
                            entry,
                            va,
                            PageSize::Page4K,
                            RootPageTableType::User,
                        )
                        .is_err()
                    {
                        // The page never published a refcount nor a
                        // mapping record — free it (the brk arm's failure
                        // hygiene), or every failed growth leaks it.
                        let _ = pagealloc::deallocate_physpage(pa);
                        return Err("rfork: sweep parent map failed");
                    }
                    let mut physalloc = PhysPageAllocator {};
                    let mut vmt = VmTraitImpl {};
                    if (*child.root_table)
                        .map_to(
                            &mut physalloc,
                            &mut vmt,
                            entry,
                            va,
                            PageSize::Page4K,
                            RootPageTableType::User,
                        )
                        .is_err()
                    {
                        // The parent's mapping is UNTRACKED (the record
                        // push below never ran for this page) — unmap it
                        // and free the page, or the page leaks and the
                        // parent's exit walk (free_user_mappings) can't
                        // see it to free it (WR-03).  Mirrors the brk
                        // arm's per-member failure unwind.
                        let mut physalloc = PhysPageAllocator {};
                        let mut vmt = VmTraitImpl {};
                        let _ = (*parent.root_table).map_to(
                            &mut physalloc,
                            &mut vmt,
                            Entry::empty(),
                            va,
                            PageSize::Page4K,
                            RootPageTableType::User,
                        );
                        let _ = pagealloc::deallocate_physpage(pa);
                        return Err("rfork: sweep child map failed");
                    }
                }
                let rec = MappingRecord { va, phys: pa.addr() as usize, size: PAGE_SIZE_4K, kind: SegKind::Bss };
                parent.mapping_log.push(rec);
                child.mapping_log.push(rec);
                share_new_user_page(pa.addr(), 2);
                va += PAGE_SIZE_4K;
            }
        }
    }

    // Mirror the segment map + demand-page byte source so the child's
    // future faults resolve in the child.
    child.segments = parent.segments.clone();
    child.text_source = parent.text_source.clone();

    // The fd GROUP semantics per fork(2) +
    // 9k sysproc.c sysrfork — RFCFDG = fresh EMPTY table (dupfgrp(nil));
    // RFFDG = a fresh table COPIED slot-by-slot (dupfgrp — pipe chans
    // clone their Arc, so both tables' pipe fds reference the SAME
    // queue); NEITHER = SHARE the parent's handle (Arc clone).  (fork(2)
    // + sysrfork + libc fork() = rfork(RFPROC|RFFDG|RFREND) = COPY
    // govern.)
    child.fd_group = if flag & crate::syscall::RFCFDG != 0 {
        Arc::new(Lock::new("fdgrp", FdGroup::new()))
    } else if flag & crate::syscall::RFFDG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("fdgrp", parent.fd_group.lock(&node).deep_copy()))
    } else {
        parent.fd_group.clone()
    };

    // The namespace group + slash/dot, fork(2)/9k sysrfork
    // semantics.  RFNAMEG = fresh COPY of the parent's group (pgrpcpy);
    // RFCNAMEG = fresh EMPTY group (the newns wipe); NEITHER = share the
    // parent's handle (Arc clone).
    child.pgrp = if flag & crate::syscall::RFCNAMEG != 0 {
        Arc::new(Lock::new("pgrp", Pgrp::new()))
    } else if flag & crate::syscall::RFNAMEG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("pgrp", parent.pgrp.lock(&node).deep_copy()))
    } else {
        parent.pgrp.clone()
    };
    // The env group, fork(2)/9k sysrfork semantics — the
    // SAME shape as the namespace group above: RFENVG = fresh deep COPY
    // of the parent's group (envcpy); RFCENVG = fresh EMPTY group;
    // NEITHER = share the parent's handle (Arc clone).  (fork(2) +
    // sysproc.c + newns's rfork(RFENVG|RFCNAMEG) govern.)
    child.env_group = if flag & crate::syscall::RFCENVG != 0 {
        Arc::new(Lock::new("env", EnvGroup::new()))
    } else if flag & crate::syscall::RFENVG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("env", parent.env_group.lock(&node).deep_copy()))
    } else {
        parent.env_group.clone()
    };
    child.slash = parent.slash.as_ref().map(|c| c.dup());
    child.dot = parent.dot.as_ref().map(|c| c.dup());
    // The user copies at fork (donor proc.c newproc:
    // `kstrdup(p->user, up->user)`) — no rfork flag governs it; a cap
    // the parent presents does NOT carry to the child's user.
    child.user = parent.user.clone();

    // The forkchild frame: a copy of the parent's live frame on the
    // CHILD's kernel stack, with x0 = 0 (the child's wire result).
    let child_frame = (child.kstack_top - core::mem::size_of::<TrapFrame>()) as *mut TrapFrame;
    // SAFETY: the child's kstack is freshly mapped (4 pages, construction)
    // and the frame sits at its top (288 bytes below kstack_top); the
    // parent's live frame is the source — disjoint kernel memory.
    unsafe {
        ptr::copy_nonoverlapping(frame, child_frame, 1);
        (*child_frame).x0 = 0;
    }
    child.frame = child_frame;
    child.parent = ptr::addr_of!(*parent) as *mut Proc;
    child.nowait = nowait;
    // Donor newproc (9front@350266104f sys/src/9/port/sysproc.c:142-144):
    // the child INHERITS the notify handler; the note queue/last-note/
    // note_ureg start empty (Proc::new's construction).
    child.notify_handler = parent.notify_handler;
    // The note group — 9k sysproc.c:183: WITHOUT RFNOTEG the
    // child INHERITS the parent's noteid; WITH it the child keeps the fresh
    // group id its construction assigned (the donor's newproc incref).
    if flag & crate::syscall::RFNOTEG == 0 {
        child.noteid = parent.noteid;
    }
    // The rendezvous group — 9k sysproc.c's RFPROC arm: WITH RFREND the
    // child gets a FRESH empty group; WITHOUT it the child SHARES the
    // parent's handle (Arc clone).  (libc fork() = rfork(RFPROC|RFFDG|
    // RFREND) therefore gives its child a FRESH group; libthread's
    // procs share one — the rendezvous exchange's whole point.)
    child.rgrp = if flag & crate::syscall::RFREND != 0 {
        Arc::new(Lock::new("rendez", RendezGroup::new()))
    } else {
        parent.rgrp.clone()
    };
    // The recorded ctl priority is inherited (donor
    // forkchild); the argv record is NOT — a fork child's args are empty
    // until its own exec (Proc::new's construction), so /proc/N/args for
    // a pre-exec child reads EMPTY rather than the parent's.
    child.pri = parent.pri;
    Ok(())
}

/// Map (or idempotently re-map — map_to replaces) a phys page's KZERO+PA
/// alias in the kernel table for rfork's copy loop.
fn map_kernel_alias(pa: u64) -> Result<()> {
    let mut physalloc = PhysPageAllocator {};
    let mut vmt = VmTraitImpl {};
    vm::kernel_pagetable()
        .map_phys_range(
            &mut physalloc,
            &mut vmt,
            "forkcopy",
            &PhysRange::with_pa_len(PhysAddr::new(pa), PAGE_SIZE_4K),
            vm::VaMapping::Offset(KZERO),
            Entry::rw_kernel_data(),
            PageSize::Page4K,
            RootPageTableType::Kernel,
        )
        .map_err(|_| "rfork: kernel alias map failed")
        .map(|_| ())
}

/// Host pins for the RFMEM shared-page refcount table (the donor dupseg
/// `sameseg` accounting): the free-side pivot is the piece a regression
/// would silently break — a page freed while a sibling still maps it is
/// use-after-free corruption with no marker.  The full share semantics
/// (both procs reading each other's writes through the same PA) are
/// guest-verified by the 08-04 ping gate; these pins cover the table's
/// share/release/refcount-gated-free decisions.
#[cfg(test)]
mod rfmem_tests {
    use super::SharedPages;

    /// A never-shared page frees unconditionally (the pre-RFMEM behavior
    /// for every ordinary page — the absence from the table IS the
    /// verdict).
    #[test]
    fn unshared_page_frees_immediately() {
        let mut t = SharedPages::new();
        assert!(t.release(0x1000));
        assert!(t.release(0x1000)); // idempotent shape: still absent
    }

    /// The ping shape: parent + child share; the child's exit must NOT
    /// free (the parent's release later must).
    #[test]
    fn last_sharers_release_frees() {
        let mut t = SharedPages::new();
        t.share(0x2000); // parent + child = 2
        assert!(!t.release(0x2000)); // first sharer exits — sibling holds
        assert!(t.release(0x2000)); // last sharer exits — free now
        // The table entry is gone: a later page at the same PA (recycled
        // by the allocator) is an ordinary page again.
        assert!(t.release(0x2000));
    }

    /// A shared proc forking again under RFMEM (the three-way shape):
    /// two releases leave the page mapped; the third frees.
    #[test]
    fn double_fork_refcounts_three() {
        let mut t = SharedPages::new();
        t.share(0x3000); // parent + child = 2
        t.share(0x3000); // + grandchild = 3
        assert!(!t.release(0x3000));
        assert!(!t.release(0x3000));
        assert!(t.release(0x3000));
    }

    /// Pages are independent: one page's lifecycle never frees another's.
    #[test]
    fn pages_are_independent() {
        let mut t = SharedPages::new();
        t.share(0x4000);
        t.share(0x5000);
        assert!(!t.release(0x4000));
        assert!(t.release(0x4000)); // 0x4000 fully released
        assert!(!t.release(0x5000)); // 0x5000 still held by its sibling
        assert!(t.release(0x5000));
    }

    /// Re-share after a full release starts a fresh lifecycle at 2
    /// (no stale count leaks into the recycled PA's next family).
    #[test]
    fn reshare_after_full_release_starts_fresh() {
        let mut t = SharedPages::new();
        t.share(0x6000);
        assert!(!t.release(0x6000));
        assert!(t.release(0x6000)); // family gone
        t.share(0x6000); // a NEW family shares the recycled page
        assert!(!t.release(0x6000));
        assert!(t.release(0x6000));
    }

    /// share_new: a page BORN shared by N members (the rfork bss sweep
    /// and brk_ family-growth arm) frees at the Nth release.
    #[test]
    fn born_shared_frees_at_the_last_release() {
        let mut t = SharedPages::new();
        t.share_new(0x7000, 3); // a brk_ growth page in a 3-member family
        assert!(!t.release(0x7000));
        assert!(!t.release(0x7000));
        assert!(t.release(0x7000));
        assert!(t.release(0x7000)); // absent now — recycled PA is ordinary
    }
}

/// Host pins for the semaphore wait registry (CR-03): `sem_wake` CONSUMES
/// the registrations it picks, so the syscall loop's
/// register → check → block-PER-ITERATION shape is load-bearing — a woken
/// waiter that loses the unit race is only ever found by a future release
/// because it RE-registers before re-blocking.  (An earlier shape
/// registered once before the loop: the consumed registration was never
/// restored and the waiter wedged permanently.)  These pins cover the
/// registry's consume/idempotency/order decisions; the PROC_TABLE ready
/// half no-ops on an empty host table, and the full two-waiter race is
/// guest-covered by the RFMEM contention gates.
#[cfg(test)]
mod sem_tests {
    use super::{SEM_WAITERS, sem_deregister_waiter, sem_register_waiter, sem_wake};
    use port::mcslock::LockNode;

    /// The pids registered on a word PA, in registration order.
    fn waiters_for(pa: u64) -> Vec<u64> {
        let node = LockNode::new();
        SEM_WAITERS.lock(&node).iter().filter(|e| e.0 == pa).map(|e| e.1).collect()
    }

    /// Registration is idempotent (the loop re-registers before EVERY
    /// block — a spuriously-woken waiter's re-register must not stack
    /// duplicates).
    #[test]
    fn register_is_idempotent() {
        let pa = 0x9100_0000;
        sem_register_waiter(pa, 11);
        sem_register_waiter(pa, 11);
        assert_eq!(waiters_for(pa), vec![11]);
        sem_deregister_waiter(pa, 11);
        assert_eq!(waiters_for(pa), Vec::<u64>::new());
    }

    /// The wake CONSUMES up to delta registrations in registration order:
    /// a picked waiter leaves the set (its re-registration on the next
    /// block is what makes the consume safe).
    #[test]
    fn wake_consumes_picked_registrations_in_order() {
        let pa = 0x9200_0000;
        sem_register_waiter(pa, 21);
        sem_register_waiter(pa, 22);
        sem_wake(pa, 1);
        assert_eq!(waiters_for(pa), vec![22], "the first-registered waiter is consumed");
        sem_wake(pa, 0); // a zero-delta release wakes nobody
        assert_eq!(waiters_for(pa), vec![22]);
        sem_wake(pa, 5); // delta beyond the set drains what's there
        assert_eq!(waiters_for(pa), Vec::<u64>::new());
    }

    /// The CR-03 shape: a woken (consumed) waiter that loses the unit
    /// race RE-REGISTERS before re-blocking; the next release finds it.
    #[test]
    fn reregistered_waiter_is_found_by_the_next_release() {
        let pa = 0x9300_0000;
        sem_register_waiter(pa, 31);
        sem_wake(pa, 1); // the release wakes (consumes) the waiter
        assert_eq!(waiters_for(pa), Vec::<u64>::new());
        // The waiter runs, loses the unit race, and re-blocks — the
        // loop's register-before-block restores the registration.
        sem_register_waiter(pa, 31);
        assert_eq!(waiters_for(pa), vec![31]);
        sem_wake(pa, 1); // the next release finds it
        assert_eq!(waiters_for(pa), Vec::<u64>::new());
    }

    /// Waiters on OTHER words are never consumed (the PA keying — RFMEM
    /// family members share a PA; unrelated procs never meet).
    #[test]
    fn wake_never_touches_other_words() {
        let pa = 0x9400_0000;
        let other = 0x9500_0000;
        sem_register_waiter(pa, 41);
        sem_register_waiter(other, 42);
        sem_wake(pa, 8);
        assert_eq!(waiters_for(pa), Vec::<u64>::new());
        assert_eq!(waiters_for(other), vec![42]);
        sem_deregister_waiter(other, 42);
    }

    /// A deregister on ONE word keeps every OTHER word's waiters in
    /// registration order (the wake's FIFO).  swap_remove moved the
    /// shared Vec's last entry into the freed slot, so this exact
    /// sequence flipped word b's [71, 72] to [72, 71] and woke 72 first.
    #[test]
    fn deregister_on_one_word_keeps_another_words_fifo() {
        let a = 0x9c00_0000;
        let b = 0x9d00_0000;
        sem_register_waiter(a, 61);
        sem_register_waiter(b, 71);
        sem_register_waiter(b, 72);
        sem_deregister_waiter(a, 61);
        assert_eq!(waiters_for(b), vec![71, 72], "b's registration order survives a's deregister");
        sem_wake(b, 1);
        assert_eq!(waiters_for(b), vec![72], "the first-registered waiter is woken first");
        sem_wake(b, 5);
        assert_eq!(waiters_for(b), Vec::<u64>::new());
    }
}

/// exit_tail's fd-group sweep (release_fd_group) with real devpipe
/// pipes — the Harvey 20-07 shared-table leak.  rfdprobe -s shape: a
/// process whose fd 1 is a backquote pipe's write end forks a plain
/// rfork(RFPROC) child (the SHARED table — an Arc clone), both exit, and
/// the reader must then see EOF.  Each sharer's exit is one
/// release_fd_group call on its own handle.
#[cfg(test)]
mod fdgrp_release_tests {
    use super::{MAX_FDS, empty_fdgroup, release_fd_group, split_fd_group_in_place};
    use alloc::sync::Arc;
    use port::dat::{Chan, ORDWR};
    use port::fdgrp::FdGroup;
    use port::mcslock::{Lock, LockNode};
    use port::{devpipe, namec};

    type Group = Arc<Lock<FdGroup<MAX_FDS>>>;

    fn empty() -> Group {
        Arc::new(Lock::new("fdgrp", FdGroup::new()))
    }

    /// A fresh group holding a pipe's write end at fd 1 (rfdprobe's
    /// stdout), and the pipe's other end (the reader rc keeps).
    fn group_with_pipe_writer() -> (Group, Chan) {
        let dir = devpipe::attach();
        let c0 = devpipe::walk(&dir, b"data").expect("walk data");
        let c1 = devpipe::walk(&dir, b"data1").expect("walk data1");
        let reader = namec::open_chan(&c0, ORDWR).expect("open data");
        let writer = namec::open_chan(&c1, ORDWR).expect("open data1");
        let g = empty();
        {
            let node = LockNode::new();
            assert!(g.lock(&node).store(1, writer).is_none());
        }
        (g, reader)
    }

    fn holds_fd1(g: &Group) -> bool {
        let node = LockNode::new();
        g.lock(&node).get(1).is_some()
    }

    /// Only called once the write side should be fully closed: on the
    /// host an empty pipe that is still open returns an error (no
    /// block hooks), so this never parks.
    fn at_eof(reader: &mut Chan) -> bool {
        let mut buf = [0u8; 8];
        devpipe::read(reader, &mut buf, u64::MAX) == Ok(0)
    }

    /// The reproducer: the child exits first, then the parent.
    #[test]
    fn last_sharer_release_closes_the_shared_pipe_writer() {
        let (mut parent, mut reader) = group_with_pipe_writer();
        let mut child = parent.clone(); // rfork(RFPROC): the shared table
        let mut closes = 0;

        let husk = empty();
        release_fd_group(&mut child, husk.clone(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert!(Arc::ptr_eq(&child, &husk), "the exiting proc holds the empty group");
        assert_eq!(closes, 0, "a non-last sharer closes nothing");
        assert_eq!(Arc::strong_count(&parent), 1, "the child's exit released its reference");
        assert!(holds_fd1(&parent), "the parent keeps the shared writer");

        release_fd_group(&mut parent, empty(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 1, "the last sharer closes the writer exactly once");
        assert!(at_eof(&mut reader), "the reader sees EOF after the last sharer's exit");
    }

    /// Boundary: an unshared group (fork()'s RFFDG copy, count 1)
    /// drains at its only holder's exit.
    #[test]
    fn unshared_group_release_closes_at_once() {
        let (mut g, mut reader) = group_with_pipe_writer();
        let mut closes = 0;
        release_fd_group(&mut g, empty(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 1);
        assert!(at_eof(&mut reader));
    }

    /// Boundary: three sharers (two shared children) — each non-last
    /// exit releases exactly one reference; only the third drains.
    #[test]
    fn three_sharers_drain_only_at_the_third_release() {
        let (mut a, mut reader) = group_with_pipe_writer();
        let mut b = a.clone();
        let mut c = a.clone();
        let mut closes = 0;

        release_fd_group(&mut a, empty(), |ch| {
            closes += 1;
            namec::close_chan(ch);
        });
        assert_eq!(closes, 0);
        assert_eq!(Arc::strong_count(&b), 2);
        assert!(holds_fd1(&b));

        release_fd_group(&mut b, empty(), |ch| {
            closes += 1;
            namec::close_chan(ch);
        });
        assert_eq!(closes, 0);
        assert_eq!(Arc::strong_count(&c), 1);
        assert!(holds_fd1(&c));

        release_fd_group(&mut c, empty(), |ch| {
            closes += 1;
            namec::close_chan(ch);
        });
        assert_eq!(closes, 1);
        assert!(at_eof(&mut reader));
    }

    /// Bare rfork(RFCFDG) (split_fd_group_in_place, copy = false): the
    /// caller gets a PRIVATE fresh table — never the exit husks'
    /// singleton, never another RFCFDG caller's — and its old private
    /// table is released (the donor's closefgrp), so the pipe writer it
    /// held closes.
    #[test]
    fn bare_rfcfdg_gets_a_private_fresh_table_and_releases_the_old_one() {
        let (mut a, mut reader) = group_with_pipe_writer();
        let mut closes = 0;
        split_fd_group_in_place(&mut a, false, |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 1, "the old private table is released");
        assert!(at_eof(&mut reader), "its pipe writer closed");
        assert!(!holds_fd1(&a), "the fresh table is empty");
        assert!(!Arc::ptr_eq(&a, &empty_fdgroup()), "never the husk singleton");

        // A later fd in this table must stay private to it.
        let (mut other, _other_reader) = group_with_pipe_writer();
        {
            let node = LockNode::new();
            let fd = a.lock(&node).alloc_lowest(Chan::new(
                port::dat::DEV_CONS,
                port::dat::Qid { path: 1, vers: 0, typ: 0 },
                ORDWR,
            ));
            assert_eq!(fd, Some(0));
        }
        split_fd_group_in_place(&mut other, false, namec::close_chan);
        assert!(!Arc::ptr_eq(&a, &other), "two RFCFDG callers never share");
        {
            let node = LockNode::new();
            assert!(other.lock(&node).get(0).is_none(), "the other caller's table is empty");
        }
        {
            let node = LockNode::new();
            assert!(empty_fdgroup().lock(&node).get(0).is_none(), "the husk singleton is untouched");
        }
    }

    /// Bare rfork(RFFDG) on a PRIVATE table (rio's startup shape): the
    /// copy dup'd the writer (open count +1), so the old table must be
    /// released (-1) or the pipe never hangs up after the caller exits.
    #[test]
    fn bare_rffdg_on_a_private_table_keeps_the_pipe_balanced() {
        let (mut g, mut reader) = group_with_pipe_writer();
        let mut closes = 0;
        split_fd_group_in_place(&mut g, true, |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 1, "the old private table is released");
        assert!(holds_fd1(&g), "the copy keeps the writer");
        release_fd_group(&mut g, empty(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 2);
        assert!(at_eof(&mut reader), "the reader sees EOF after the caller's exit");
    }

    /// Bare rfork(RFFDG) on a SHARED table (lib9p _postmountsrv: the
    /// server child shares, the parent privatizes): nothing closes at
    /// the split; each table closes its own copy at its holder's exit.
    #[test]
    fn bare_rffdg_on_a_shared_table_closes_nothing_at_the_split() {
        let (mut child, mut reader) = group_with_pipe_writer();
        let mut parent = child.clone();
        let mut closes = 0;
        split_fd_group_in_place(&mut parent, true, |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 0, "the child still holds the shared table");
        assert_eq!(Arc::strong_count(&child), 1, "the parent's reference moved to its copy");
        assert!(holds_fd1(&parent) && holds_fd1(&child));

        release_fd_group(&mut child, empty(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        release_fd_group(&mut parent, empty(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 2);
        assert!(at_eof(&mut reader));
    }

    /// A table holding a pipe writer AND its dup (two slots, one open
    /// object — the dup'd-fd shape): the last sharer's drain must
    /// close-dispatch every slot, or the dup's open count keeps the
    /// pipe up.
    #[test]
    fn last_sharer_drain_closes_every_duplicate() {
        let (mut g, mut reader) = group_with_pipe_writer();
        {
            let node = LockNode::new();
            let mut t = g.lock(&node);
            let d = t.ref_clone(1).expect("dup fd 1");
            assert!(t.store(2, d).is_none());
        }
        let mut closes = 0;
        release_fd_group(&mut g, empty(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 2, "both duplicates close");
        assert!(at_eof(&mut reader));
    }
}

/// The #44 husk sweep's namespace half (release_pgrp — the donor
/// closepgrp with its mount half, x86_64 ab0a977): a pipe write end
/// mounted in a namespace stands in for a mounted tree's attach fid
/// (both are chans only the mount table holds), so the reader's EOF
/// proves the mount table was close-dispatched.
#[cfg(test)]
mod husk_sweep_tests {
    use super::{empty_env_group, empty_pgrp, empty_rendez_group, release_pgrp};
    use alloc::sync::Arc;
    use port::dat::{Chan, DEV_CONS, ORDWR, Qid};
    use port::mcslock::{Lock, LockNode};
    use port::pgrp::{MREPL, Pgrp};
    use port::{devpipe, namec};

    type Ns = Arc<Lock<Pgrp>>;

    fn fresh() -> Ns {
        Arc::new(Lock::new("pgrp", Pgrp::new()))
    }

    /// A namespace whose mount table holds a pipe's write end (MREPL
    /// over a plain file point), and the pipe's other end.
    fn ns_with_mounted_writer() -> (Ns, Chan) {
        let dir = devpipe::attach();
        let c0 = devpipe::walk(&dir, b"data").expect("walk data");
        let c1 = devpipe::walk(&dir, b"data1").expect("walk data1");
        let reader = namec::open_chan(&c0, ORDWR).expect("open data");
        let writer = namec::open_chan(&c1, ORDWR).expect("open data1");
        let point = Chan::new(DEV_CONS, Qid { path: 7, vers: 0, typ: 0 }, ORDWR);
        let ns = fresh();
        {
            let node = LockNode::new();
            ns.lock(&node).bind(writer, &point, MREPL).expect("bind the writer over the point");
        }
        (ns, reader)
    }

    fn at_eof(reader: &mut Chan) -> bool {
        let mut buf = [0u8; 8];
        devpipe::read(reader, &mut buf, u64::MAX) == Ok(0)
    }

    /// Two procs share a namespace (rfork without RFNAMEG): the first
    /// exit closes nothing and gives up its reference; the last closes
    /// the whole mount table (the point's `from` + the mounted chan).
    #[test]
    fn last_holder_release_closes_the_mount_table() {
        let (mut a, mut reader) = ns_with_mounted_writer();
        let mut b = a.clone();
        let mut closes = 0;
        release_pgrp(&mut a, fresh(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 0, "a non-last holder closes nothing");
        assert_eq!(Arc::strong_count(&b), 1, "its reference is released");

        release_pgrp(&mut b, fresh(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 2, "the point's from + the mounted chan");
        assert!(at_eof(&mut reader), "the mounted writer was closed");
    }

    /// Boundary: an unshared namespace (RFNAMEG's private copy) closes
    /// its mount table at its only holder's exit, and the slot then holds
    /// the replacement it was given.
    #[test]
    fn unshared_release_closes_at_once_and_installs_the_replacement() {
        let (mut ns, mut reader) = ns_with_mounted_writer();
        let husk = fresh();
        let mut closes = 0;
        release_pgrp(&mut ns, husk.clone(), |c| {
            closes += 1;
            namec::close_chan(c);
        });
        assert_eq!(closes, 2);
        assert!(at_eof(&mut reader));
        assert!(Arc::ptr_eq(&ns, &husk));
    }

    /// The husk singletons are one shared object each (a refcount bump
    /// per exit, never a fresh allocation) and start empty.
    #[test]
    fn husk_singletons_are_shared() {
        assert!(Arc::ptr_eq(&empty_pgrp(), &empty_pgrp()));
        assert!(Arc::ptr_eq(&empty_env_group(), &empty_env_group()));
        assert!(Arc::ptr_eq(&empty_rendez_group(), &empty_rendez_group()));
        let node = LockNode::new();
        assert!(empty_pgrp().lock(&node).take_mount_chans().is_empty());
    }
}
