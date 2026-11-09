//! Process scaffolding on the x86_64 per-process address-space model
//! (the OQ-2 resolution, PROJECT.md's locked per-process
//! live-user-memory model): per-process PML4/CR3 with the kernel
//! higher half SHARED (PML4[511] = the boot KZERO table — all RAM; the
//! SYSCALL entry's gs-relative Mach access needs PML4[510] too), a
//! small fixed proc table behind the MCS-lock idiom, the
//! current-proc pointer, and the scheduler — tick-driven preemptive
//! round-robin (preempt/reschedule) plus the swtch-based blocking path
//! (sched/sched_loop) that AWAIT rides.  The kernel stack switch goes
//! through the TSS rsp0 mechanism (the ONE kernel-stack mechanism both
//! ring-3 entry paths share; re-pointed as one locked step on every
//! switch), never through an aarch64-style CURKSTACK side channel.
//!
//! Locking discipline (carried from the aarch64 port verbatim):
//! - PROC_TABLE is taken only briefly in process/boot/trap context —
//!   NEVER across the fault/page-in paths or a pagealloc call that
//!   could recurse, and never across a swtch (the resumed proc may lock
//!   it).
//! - Trap-context code reads the current proc through `current_proc()`'s
//!   raw pointer, set at exec/switch time.  Invariant: single CPU, and
//!   trap handlers run IRQ-masked, so the pointer is stable for the
//!   handler's lifetime.
//! - CURFRAME (the live-frame record) is written only under the
//!   PROC_TABLE lock or in IRQ-masked trap context; read by the
//!   scheduler's resume arms and the note-delivery boundary.
//!
//! Physical-memory access model: every "PA" the process machinery
//! handles is reached through the kernel-half alias at `KZERO+pa` (the
//! boot tables map ALL of the q35 guest's RAM there — the 1 GiB
//! KZERO_MAP_LIMIT guard below documents the window).  There is no
//! Linux-style copy layer anywhere: user VAs are directly
//! dereferenceable in a proc's syscall context because its CR3 is
//! live, and its pages are writable by the kernel through the same
//! alias when its table is NOT the active one.

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
use port::mem::{PAGE_SIZE_4K, VirtRange};
#[cfg(not(test))]
use port::mem::PhysAddr;
use port::paqfs::PaqDir;
use port::pgrp::Pgrp;
use port::println;
use port::rendez::RendezGroup;

use crate::dat::Ureg;
#[cfg(not(test))]
use crate::dat::Stack;
use crate::swtch;

/// The higher-half base (kernel.ld) — the same value main.rs links by.
/// Every physical page the process machinery touches is aliased at
/// `KZERO+pa` in the SHARED kernel half (PML4[511] → the boot KZERO
/// table), so kernel-side content writes work identically under the
/// boot PML4 and under any per-process PML4.
pub(crate) const KZERO: u64 = 0xffff_ffff_8000_0000;

/// The device-MMIO alias window's base (l.S's bootmachpt3 entries 2..5,
/// inside PML4[510] — the slot every proc table SHARES): the four low
/// GiB of PA re-mapped at a fixed higher-half VA.  Every device
/// volatile deref MUST go through `kaddr` — the raw identity VA
/// (PA==VA) is covered only by the boot table's PML4[0], which
/// per-proc tables do not carry ([0] is the user half), so a post-fork
/// device access (the tick's xAPIC EOI, an ether IRQ's virtio queue
/// work) faults under the running proc's CR3 (the 14-06 chain gate
/// surfaced it as a kernel write fault at 0xFEE00380 in timer::tick
/// under a proc table).  The KZERO alias cannot host these: KZERO
/// sits 2 GiB below the top of the canonical space, so KZERO+pa
/// overflows u64 for every pa >= 2 GiB (measured: checked_add fails
/// at the ECAM base 0xB0000000).
pub(crate) const MMIO_ALIAS: usize = 0xFFFF_FF00_8000_0000;

#[track_caller]
pub(crate) fn kaddr(pa: u64) -> usize {
    if pa >> 32 >= 4 {
        panic!("kaddr: PA {pa:#x} outside the 4 GiB device window");
    }
    MMIO_ALIAS + pa as usize
}

/// The KZERO window's ceiling: the boot tables map exactly the first
/// 1 GiB of physical memory at KZERO (l.S's bootkernpt3 single 1 GiB
/// leaf), and the q35 gate guest is 1024 MiB — an exact fit.  A page
/// allocated beyond it would be invisible at the alias; the guard
/// below makes that loud instead of a wild write.
const KZERO_MAP_LIMIT: u64 = 1 << 30;

/// Plan 9 KNAMELEN (27 + NUL).
pub const KNAMELEN: usize = 28;
/// Plan 9 ERRMAX (sys/include/libc.h) — errstr and note-message buffers.
pub const ERRMAX: usize = 128;
/// Proc table size.  Slots are never REAPED this era (a Moribund husk
/// pins its slot — the documented exit-sweep deferral), so the table is
/// a boot-session fork BUDGET, not a concurrency bound: the 07-07 net
/// sweep alone forks ~20 times, and the 06-07 tier-1 sweep MEASURES 201
/// born-traced pids for the full sweep.  512 covers the measured 201 +
/// the boot chain with >2x headroom; the exit-sweep deferral stays the
/// tracked follow-up (a long-lived shell still eventually fills ANY
/// fixed table).
pub const MAX_PROCS: usize = 512;
/// Open fd slots per proc (init's closefds closes 3..29 — the table
/// must hold fds through 29 without making that fatal; pipe+dup flows
/// hold several at once).
pub const MAX_FDS: usize = 32;
/// Per-process kernel stack: 16 KiB (4 pages) — the 9k donor's KSTACK
/// (mem.h: `#define KSTACK (16*1024)`) and the aarch64 port's value.
/// The stack is NOT physically contiguous: it maps at the fixed
/// kernel-half window below, so four independently-allocated pages
/// form one contiguous VA range (the per-process PML4's own mapping).
const KSTACK_PAGES: usize = 4;
/// The per-process kstack's PML4 slot (index 400 of the 256..509
/// kernel-half span the boot tables leave free — 510 is the KCPUZERO
/// Mach window, 511 is KZERO).  Every proc's PML4 maps ITS OWN kstack
/// at this SAME VA, kernel-only (no PTE_US): the TSS rsp0 and the
/// SYSCALL/IDT entries find it under whichever per-process table is
/// active, and ring 3 can never reach it.
const KSTACK_PML4_INDEX: usize = 400;
/// The kstack window's base VA (KSTACK_PML4_INDEX sign-extended:
/// 400 << 39 = 0x0000_c800_0000_0000, bit 47 set, so the canonical
/// form carries 0xffff in the top half).  The bring-up value
/// 0xffff_3200_0000_0000 was NON-canonical (0x3200_0000_0000 is index
/// 100's low-half offset — bits 63:47 disagreed), and any access
/// through it raised #GP(0): invisible while nothing dereferenced the
/// window (host tests check PTEs and write via the PA alias), loud the
/// moment 14-05's bootstrap read the seeded frame through it (PF/#GP
/// at the exec print, live).
const KSTACK_VA: usize = 0xffff_c800_0000_0000;
/// Queued-notes capacity (the note delivery path fills it).
pub const NOTE_QUEUE_CAP: usize = 8;
/// The exec-recorded argv's byte bound (the /proc/N/args content —
/// ps reads 256 bytes max, ps.c:63).  NUL-joined per arg.
pub const ARGV_CAP: usize = 256;

// --- x86_64 PTE bits (the aarch64 vm::Entry kinds re-keyed) ---
const PTE_P: u64 = 1 << 0;
const PTE_RW: u64 = 1 << 1;
const PTE_US: u64 = 1 << 2;
const PTE_NX: u64 = 1 << 63;
/// A user leaf entry: present, r/w, user-accessible, EXECUTABLE (the
/// aarch64 `rw_user_text`: UXN-clear — user code fetches through it;
/// the kernel-half is unaffected).
pub(crate) const PTE_USER_TEXT: u64 = PTE_P | PTE_RW | PTE_US;
/// A user leaf entry: present, r/w, user-accessible, NO-EXECUTE (the
/// aarch64 `rw_user_data` — data/bss/stack/tos).
pub(crate) const PTE_USER_DATA: u64 = PTE_P | PTE_RW | PTE_US | PTE_NX;

// rfork flags (Harvey sys/include/libc.h — frozen values).  Homed here
// (the aarch64 port's home is syscall.rs) so the process machinery —
// rfork_clone/rfork_copy — compiles standalone; syscall.rs consumes
// them from this module.
pub(crate) const RFNAMEG: u64 = 1 << 0;
pub(crate) const RFENVG: u64 = 1 << 1;
pub(crate) const RFFDG: u64 = 1 << 2;
pub(crate) const RFNOTEG: u64 = 1 << 3;
pub(crate) const RFPROC: u64 = 1 << 4;
pub(crate) const RFMEM: u64 = 1 << 5;
pub(crate) const RFNOWAIT: u64 = 1 << 6;
pub(crate) const RFCNAMEG: u64 = 1 << 10;
pub(crate) const RFCENVG: u64 = 1 << 11;
pub(crate) const RFCFDG: u64 = 1 << 12;
pub(crate) const RFREND: u64 = 1 << 13;

/// The note-group id allocator (the donor's `Ref noteidalloc` — 9k
/// proc.c:13; fresh id per newproc, inherited or split per the rfork
/// RFNOTEG rules).
static NEXT_NOTEID: AtomicU64 = AtomicU64::new(1);

/// User segment kinds — the mapping log's tag (rfork-copy/exit-free
/// walk the log by kind; exec/ensure-resident classify by segment).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegKind {
    Text,
    Data,
    Bss,
    Stack,
    Tos,
}

impl SegKind {
    /// The leaf PTE kind this segment maps as (text is the only
    /// executable user segment).
    pub(crate) fn pte(self) -> u64 {
        match self {
            SegKind::Text => PTE_USER_TEXT,
            _ => PTE_USER_DATA,
        }
    }
}

/// One user mapping — appended by exec (seed pages) and the
/// residency paths (demand zero-page-in).
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
    /// Classify a user VA (exec.rs's eager map records byte-exact
    /// data/bss ends; the classifier below mirrors the aarch64 port's
    /// shared-page rule).  Empty segments (zero-size data/bss) classify
    /// nothing; the byte-exact ranges stay authoritative for
    /// validate_user_range.  The stack range IS the 16 MiB growth
    /// bound, so a Stack hit is always in-bounds.
    pub fn classify(&self, va: usize) -> Option<SegKind> {
        // Data's classification end, page-rounded into the bss head
        // — but never past bss_end (an empty/tiny bss extends
        // nothing), and never below data's own end.  The shared
        // last-data/first-bss page must classify as Data: page-in
        // order then yields the right bytes either way (the aarch64
        // silent-corruption rule, carried verbatim).
        let data_classify_end =
            round_up_page(self.data.0.end).min(self.bss.0.end.max(self.data.0.end));
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
    /// devmnt binaries (the /bin union crossing): the WHOLE file, read
    /// eagerly at exec through the 9P client — syscall context can
    /// block on RPCs; a fault-context pager NEVER can, so the bytes are
    /// kernel-owned from exec on.  Arc-shared across rfork (the
    /// source-handle clone).
    Mnt { file: Arc<Vec<u8>> },
}

impl ByteBacking {
    /// Read the file's bytes at `file_off` into `dst`, clamped to the
    /// file's length; returns the byte count (0 at/past EOF).
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

    /// The backing file's total length (the truncated-binary check).
    pub fn file_len(&self) -> u64 {
        match self {
            ByteBacking::Paq { dir, .. } => u64::from(dir.length),
            ByteBacking::Mnt { file } => file.len() as u64,
        }
    }
}

/// The demand-page byte source for text/data: the backing plus the
/// data-segment file base (40 + textsize — data file offsets are NOT
/// page-aligned).  Clone for rfork (the child mirrors the parent's
/// source handles; the Mnt variant Arc-shares the staged bytes).
#[derive(Clone)]
pub struct ByteSource {
    pub backing: ByteBacking,
    pub data_file_base: u64,
}

impl ByteSource {
    /// The pager's read path — delegates to the backing.
    pub fn read_at(&self, file_off: u64, dst: &mut [u8]) -> Result<usize> {
        self.backing.read_at(file_off, dst)
    }
}

/// A queued note message (delivered at the return-to-ring-3 boundary).
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

// ---------------------------------------------------------------------------
// The physical-memory seam: the "PA" plumbing every mapping body uses.
// Kernel side: pagealloc + the KZERO alias.  Host-test side: real host
// pages whose ADDRESSES play the PA role, with phys_ptr_mut the
// identity — the 14-03 mock-proc doctrine (the mock's memory IS the
// address range), extended to the mapping machinery so the exec/rfork
// bodies are genuinely host-exercised, not deferred to metal.
// ---------------------------------------------------------------------------

/// The kernel alias of a physical page (the boot tables' shared KZERO
/// window).  Panics loudly on a PA outside the window — the 1 GiB
/// guest is an exact fit, and silence here would be a wild write.
#[cfg(not(test))]
pub(crate) fn phys_ptr_mut(pa: u64) -> *mut u8 {
    assert!(pa < KZERO_MAP_LIMIT, "PA {pa:#x} outside the KZERO window");
    (KZERO + pa) as *mut u8
}

/// The host-test identity alias: test "PAs" ARE host pointers.
#[cfg(test)]
pub(crate) fn phys_ptr_mut(pa: u64) -> *mut u8 {
    pa as *mut u8
}

/// Allocate one fresh ZEROED page, returning its "PA" (the aarch64
/// map_one_user_page zeroing discipline, folded into the seam so no
/// caller can forget it).
#[cfg(not(test))]
pub(crate) fn alloc_phys_page() -> Result<u64> {
    let pa = crate::pagealloc::allocate_physpage()
        .map(|p| p.addr())
        .map_err(|_| "page alloc failed")?;
    zero_page(pa);
    Ok(pa)
}

/// Free a page the seam (or a table walk) allocated.
#[cfg(not(test))]
pub(crate) fn free_phys_page(pa: u64) {
    if let Err(err) = crate::pagealloc::deallocate_physpage(PhysAddr::new(pa)) {
        println!("proc: deallocate page {pa:#x}: {err:?}");
    }
}

/// Zero one full page through the alias.
fn zero_page(pa: u64) {
    // SAFETY: pa names a freshly-allocated owned page (the seam's
    // contract); exactly PAGE_SIZE_4K bytes are written.
    unsafe { ptr::write_bytes(phys_ptr_mut(pa), 0, PAGE_SIZE_4K) };
}

/// The host-test page arena: leaked-on-free pages would accumulate
/// across the module's tests — track and actually free so test counts
/// stay honest.  Layout is the constant 4096/4096.
#[cfg(test)]
pub(crate) fn alloc_phys_page() -> Result<u64> {
    use alloc::alloc::{Layout, alloc};
    let layout = Layout::from_size_align(PAGE_SIZE_4K, PAGE_SIZE_4K).unwrap();
    // SAFETY: the layout is valid and nonzero-sized.
    let p = unsafe { alloc(layout) } as u64;
    if p == 0 {
        return Err("page alloc failed");
    }
    zero_page(p);
    Ok(p)
}

#[cfg(test)]
pub(crate) fn free_phys_page(pa: u64) {
    use alloc::alloc::{Layout, dealloc};
    let layout = Layout::from_size_align(PAGE_SIZE_4K, PAGE_SIZE_4K).unwrap();
    // SAFETY: pa was produced by this seam's alloc with this layout.
    unsafe { dealloc(pa as *mut u8, layout) };
}

// ---------------------------------------------------------------------------
// The per-process PML4 walk (the aarch64 vm::map_to re-key): map/unmap
// 4 KiB leaves in a proc's OWN table, allocating intermediate pages on
// the way down.  The kernel half (510/511) is never walked — those
// entries are shared at construction.
// ---------------------------------------------------------------------------

/// One entry of the table at `table_pa`.

/// Power-of-two page round-up — `usize::next_multiple_of` lowers
/// through u128 arithmetic (`__multi3`, soft-float here); the page
/// size is a power of two, so the mask form is exact.
fn round_up_page(x: usize) -> usize {
    (x + PAGE_SIZE_4K - 1) & !(PAGE_SIZE_4K - 1)
}

fn table_entry(table_pa: u64, idx: usize) -> u64 {
    // SAFETY: table_pa names a page this module allocated (the seam)
    // and 0-initialized; idx < 512 by the callers' masks.
    unsafe { phys_ptr_mut(table_pa).cast::<u64>().add(idx).read() }
}

/// Write one entry of the table at `table_pa`.
fn set_table_entry(table_pa: u64, idx: usize, val: u64) {
    // SAFETY: table_entry's contract; the write lands inside the page.
    unsafe { phys_ptr_mut(table_pa).cast::<u64>().add(idx).write(val) };
}

/// Invalidate one VA's translation (needed only when a PRESENT entry
/// is replaced or cleared in the ACTIVE table; harmless otherwise).
#[cfg(not(test))]
fn invlpg(va: usize) {
    // SAFETY: invlpg is a pure TLB-management instruction on the
    // caller-named VA.
    unsafe { core::arch::asm!("invlpg ({0})", in(reg) va, options(att_syntax, nostack, preserves_flags)) };
}

#[cfg(test)]
fn invlpg(_va: usize) {}

/// Map the 4 KiB page `pa` at `va` in the table rooted at `pml4_pa`
/// with leaf flags `leaf` (PTE_USER_*).  Allocates missing
/// intermediate tables (user-half: P|RW|US so user leaves are
/// reachable); returns the pages the walk CREATED (intermediates the
/// CALLER must record for the owning proc's exit free — a walk failure
/// frees them itself, leaving no leaks).  A present-leaf replacement
/// invalidates the VA.
fn pml4_map_user(
    pml4_pa: u64,
    va: usize,
    pa: u64,
    leaf: u64,
) -> Result<[Option<u64>; 3]> {
    let mut created: [Option<u64>; 3] = [None, None, None];
    let inter = PTE_P | PTE_RW | PTE_US;
    let mut table_pa = pml4_pa;
    for (level, slot) in created.iter_mut().enumerate() {
        let shift = 39 - 9 * level;
        let idx = (va >> shift) & 0x1ff;
        let next = {
            let e = table_entry(table_pa, idx);
            if e & PTE_P != 0 {
                e & 0x000f_ffff_ffff_f000
            } else {
                let np = alloc_phys_page()?;
                *slot = Some(np);
                set_table_entry(table_pa, idx, np | inter);
                np
            }
        };
        table_pa = next;
    }
    let idx = (va >> 12) & 0x1ff;
    let was_present = table_entry(table_pa, idx) & PTE_P != 0;
    set_table_entry(table_pa, idx, (pa & 0x000f_ffff_ffff_f000) | leaf);
    if was_present {
        invlpg(va);
    }
    Ok(created)
}

/// Clear the 4 KiB leaf at `va` (if present).  Returns true when a
/// mapping existed (the caller's free-side bookkeeping).
fn pml4_unmap_user(pml4_pa: u64, va: usize) -> bool {
    let mut table_pa = pml4_pa;
    for level in 0..3 {
        let shift = 39 - 9 * level;
        let idx = (va >> shift) & 0x1ff;
        let e = table_entry(table_pa, idx);
        if e & PTE_P == 0 {
            return false;
        }
        table_pa = e & 0x000f_ffff_ffff_f000;
    }
    let idx = (va >> 12) & 0x1ff;
    let e = table_entry(table_pa, idx);
    if e & PTE_P == 0 {
        return false;
    }
    set_table_entry(table_pa, idx, 0);
    invlpg(va);
    true
}

/// The boot PML4's kernel-half entries (510: the KCPUZERO Mach window
/// the gs-relative entry asm needs under EVERY table; 511: KZERO, all
/// RAM) — copied into every per-process PML4 at construction.  Read
/// from the linked boot table itself (l.S's bootpml4), never restated.
#[cfg(not(test))]
fn kernel_half_entries() -> [u64; 2] {
    unsafe extern "C" {
        static bootpml4: u8;
    }
    let base = ptr::addr_of!(bootpml4) as usize;
    // SAFETY: the boot PML4 is linked, mapped (its own entry 511 maps
    // it — KZERO covers the whole image) and read-only to us; exactly
    // two qwords are read, at slots 510/511 of the 512-entry page.
    unsafe {
        let e510 = (base as *const u64).add(510).read();
        let e511 = (base as *const u64).add(511).read();
        [e510, e511]
    }
}

/// The active CR3 (the current table's physical root).
#[cfg(not(test))]
pub(crate) fn read_cr3() -> u64 {
    let v: u64;
    // SAFETY: a pure control-register read.
    unsafe {
        core::arch::asm!("mov %cr3, %rax", out("rax") v, options(att_syntax, nostack, preserves_flags))
    };
    v
}

/// Switch the active address space (the aarch64 vm::switch re-key).
#[cfg(not(test))]
pub(crate) fn switch_cr3(pml4_pa: u64) {
    // SAFETY: a pure control-register write; the caller (the
    // scheduler's locked step) guarantees the table is fully built.
    unsafe {
        core::arch::asm!("mov %rax, %cr3", in("rax") pml4_pa, options(att_syntax, nostack, preserves_flags))
    };
}

#[cfg(test)]
pub(crate) fn read_cr3() -> u64 {
    0
}

#[cfg(test)]
pub(crate) fn switch_cr3(_pml4_pa: u64) {}

/// Re-point the TSS rsp0 (the kstack switch mechanism) — the Mach at
/// its fixed KCPUZERO VA.
#[cfg(not(test))]
pub(crate) fn set_tss_rsp0(top: u64) {
    // SAFETY: the Mach lives at the fixed VA l.S maps (dat::KMACH);
    // single CPU, IRQ-masked caller contexts (the scheduler's locked
    // step discipline).
    unsafe {
        (*(crate::dat::KMACH as *mut crate::dat::Mach)).tss.set_rsp0_raw(top);
    }
}

#[cfg(test)]
pub(crate) fn set_tss_rsp0(_top: u64) {}

// ---------------------------------------------------------------------------
// RFMEM shared-page refcounts (the donor dupseg `sameseg` accounting,
// verbatim from the aarch64 port — pure registry logic, no arch
// touchpoints).
// ---------------------------------------------------------------------------

/// RFMEM shared-page refcounts — translated to the per-proc mapping_log
/// model: each proc's PML4 maps the shared PHYS page directly, so the
/// sharer count lives here keyed by physical address.  A page enters
/// the table at its first RFMEM share (count = 2: parent + child) and
/// leaves at the release that drops the count to 0.  Pages never
/// shared are absent from the table and free unconditionally.
///
/// Locking: syscall context only, briefly — never IRQ context, the
/// same leaf discipline as PROC_TABLE.
static SHARED_PAGES: Lock<SharedPages> = Lock::new("rfmem", SharedPages::new());

/// The PA → sharer-count table behind SHARED_PAGES.
struct SharedPages {
    map: Vec<(u64, usize)>,
}

impl SharedPages {
    const fn new() -> Self {
        Self { map: Vec::new() }
    }

    /// Record one more sharer of `pa` (rfork's RFMEM arm): the first
    /// share inserts at 2 (parent + child); a later share increments.
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
    /// and the brk_ family-growth arm).  The page must be absent.
    fn share_new(&mut self, pa: u64, count: usize) {
        debug_assert!(count >= 2, "share_new with fewer than two sharers");
        self.map.push((pa, count));
    }

    /// Drop one sharer of `pa`.  Returns true when the caller must
    /// FREE the page — either it was never shared (absent from the
    /// table) or this release was the last sharer.  False = sibling
    /// sharers still map the page: the caller unmaps it from the
    /// exiting proc's table but must NOT return it to the allocator.
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
/// true = deallocate the page; false = siblings still map it.
fn release_user_page(pa: u64) -> bool {
    let node = LockNode::new();
    SHARED_PAGES.lock(&node).release(pa)
}

/// An RFMEM family — the donor's shared-segment GROUP.  rfork under
/// RFMEM makes the parent and child share their Data/Bss pages; the
/// family registry records the members so sys_brk_'s family arm keeps
/// the members' Data/Bss mappings identical by construction.  The
/// invariant: at any instant, all members of a family map the same PAs
/// across [bss start, break) and carry the same break.  rfork
/// establishes it, brk_ maintains it, exec/exit dissolve membership.
/// Stack/Tos pages are NEVER family-shared (SG_STACK is always a fresh
/// copy — note delivery writes the proc's own stack).
static RFMEM_FAMILIES: Lock<Vec<(u64, u64)>> = Lock::new("rfmemfam", Vec::new());

/// The family-id allocator (fresh id per first RFMEM fork in a family).
static NEXT_RFMEM_FAMILY: AtomicU64 = AtomicU64::new(1);

/// Join the caller's family (or found one when the parent forks its
/// first RFMEM child).  Runs ONLY after a successful rfork_copy.
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

/// Dissolve the proc's membership (exec/exit via free_user_mappings).
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
/// member's table and refcounted at the member count.
///
/// Failure hygiene: a mid-arm failure (OOM) unmaps everything this call
/// mapped and releases the published refcounts — the family is left
/// exactly as at entry.
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
    let mut va = round_up_page(old_end);
    let limit = round_up_page(new_end);
    while va < limit {
        let pa = alloc_phys_page().map_err(|_| "brk: out of memory")?;
        // (The page is pre-zeroed by the seam.)
        let mut mapped: Vec<*mut Proc> = Vec::new();
        let mut failed = false;
        for &m in &members {
            // SAFETY: table-resident pointer; single CPU, IRQ-masked
            // syscall context.  pml4_map_user edits the member's table
            // (ACTIVE for the caller, INACTIVE for its siblings — the
            // kernel-half alias makes both identical operations); the
            // VA was unmapped, so no stale translation exists.
            unsafe {
                let p = &mut *m;
                match pml4_map_user(p.pml4_pa, va, pa, SegKind::Bss.pte()) {
                    Ok(created) => {
                        for c in created.into_iter().flatten() {
                            p.mapping_tables.push(c);
                        }
                    }
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
                p.mapping_log.push(MappingRecord {
                    va,
                    phys: pa as usize,
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
                    pml4_unmap_user(p.pml4_pa, va);
                    if let Some(pos) = p.mapping_log.iter().position(|r| r.va == va) {
                        p.mapping_log.swap_remove(pos);
                    }
                }
            }
            free_phys_page(pa);
            break;
        }
        share_new_user_page(pa, members.len());
        done.push((va, pa));
        va += PAGE_SIZE_4K;
    }
    let completed_all = va >= limit;
    if !completed_all {
        // Unwind the completed pages: unmap from every member and
        // release per member — the last release frees.
        for &(done_va, done_pa) in &done {
            for &m in &members {
                unsafe {
                    let p = &mut *m;
                    pml4_unmap_user(p.pml4_pa, done_va);
                    if let Some(pos) = p.mapping_log.iter().position(|r| r.va == done_va) {
                        p.mapping_log.swap_remove(pos);
                    }
                }
                if release_user_page(done_pa) {
                    free_phys_page(done_pa);
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
/// arm edits every member).  PROC_TABLE-brief; the caller mutates
/// through them AFTER the lock drops.
pub fn rfmem_family_members(family: u64) -> Vec<*mut Proc> {
    let node = LockNode::new();
    let pids: Vec<u64> = {
        let families = RFMEM_FAMILIES.lock(&node);
        families.iter().filter(|&&(f, _)| f == family).map(|&(_, pid)| pid).collect()
    };
    let table = PROC_TABLE.lock(&node);
    let mut out = Vec::new();
    for p in table.procs.iter().flatten() {
        if pids.contains(&p.pid) && p.state != ProcState::Moribund {
            out.push(ptr::addr_of!(*p) as *mut Proc);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SEMACQUIRE/SEMRELEASE (37/38) — the semaphore wait registry (verbatim
// from the aarch64 port: the word lives in user memory, the waiters
// are keyed by the word's PHYSICAL address so an RFMEM family contends
// on ONE queue while unrelated procs reusing the same VA never meet).
// ---------------------------------------------------------------------------

/// (word PA, waiter pid) pairs.
static SEM_WAITERS: Lock<Vec<(u64, u64)>> = Lock::new("sem", Vec::new());

/// sys_semacquire's contended arm: record the caller as a waiter on the
/// word's PA.  Idempotent — the acquire loop RE-registers before every
/// block (sem_wake consumes registrations as it picks its wakers).
pub fn sem_register_waiter(pa: u64, pid: u64) {
    let node = LockNode::new();
    let mut waiters = SEM_WAITERS.lock(&node);
    if !waiters.contains(&(pa, pid)) {
        waiters.push((pa, pid));
    }
}

/// Acquired (or erroring out): the caller leaves the wait set.
pub fn sem_deregister_waiter(pa: u64, pid: u64) {
    let node = LockNode::new();
    let mut waiters = SEM_WAITERS.lock(&node);
    if let Some(pos) = waiters.iter().position(|&e| e == (pa, pid)) {
        waiters.swap_remove(pos);
    }
}

/// sys_semrelease's wake half: ready up to `delta` waiters registered
/// on the word's PA (registration order).  Entries are consumed as
/// they are picked — the register→check→block ordering is
/// wakeup-loss-free by construction (single CPU, IRQ-masked syscall
/// context).
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

/// A user process.  Raw-pointer fields (`frame`) name memory this proc
/// owns; the address space is its own PML4 (`pml4_pa` — the CR3 value),
/// with the kernel higher half shared by construction.
pub struct Proc {
    pub pid: u64,
    pub name: [u8; KNAMELEN],
    pub state: ProcState,
    /// Top of this proc's 16 KiB kernel stack — the fixed kernel-half
    /// window VA (KSTACK_VA + 16 KiB) inside this proc's OWN PML4; the
    /// TSS rsp0 names it while this proc is current (the one
    /// kernel-stack mechanism both ring-3 entry paths share).
    pub kstack_top: usize,
    /// Physical pages backing the kernel stack (the EXITS path frees
    /// them — the mapping is per-proc, so construction records the
    /// backing pages here).
    pub kstack_pas: [u64; KSTACK_PAGES],
    /// The kstack's three intermediate table pages (PML4[400] → PDPT →
    /// PD → PT): per-proc, freed with the kstack.
    pub kstack_table_pas: [u64; 3],
    /// The per-process PML4's physical address — the CR3 value (the
    /// aarch64 `root_table` re-key: per-process user half, kernel
    /// higher half shared).  NEVER the boot PML4.
    pub pml4_pa: u64,
    /// Intermediate table pages the user-half walk allocated in THIS
    /// proc's table (freed at exit alongside the PML4 — the walk
    /// returns them per call, this Vec records them for the free path).
    pub mapping_tables: Vec<u64>,
    /// fd group: the slots live in a SHAREABLE group object behind the
    /// mcslock idiom, so rfork's fd semantics are real per
    /// fork(2)/sysproc.c (RFNEITHER shares the Arc; RFFDG copies
    /// slot-by-slot; RFCFDG starts empty).  fd 0/1/2 are devcons chans,
    /// preopened at construction (Plan 9 convention).
    pub fd_group: Arc<Lock<FdGroup<MAX_FDS>>>,
    /// User mappings, exec/residency-appended — rfork-copy/exit-free
    /// walk this.
    pub mapping_log: Vec<MappingRecord>,
    /// The a.out-derived VA map; None until exec.
    pub segments: Option<Segments>,
    /// paqfs byte source for demand page-in; None until exec.
    pub text_source: Option<ByteSource>,
    /// The stashed Ureg on this proc's kernel stack (the forkret seed
    /// at exec; trap save/restore rides it from then on).  Read by the
    /// CURFRAME record at the resume boundary.
    pub frame: *mut Ureg,
    /// Kernel-context switch state — valid only while
    /// blocked_in_kernel (sched() saved it mid-syscall); the sched
    /// loop swtch'es it back in.
    pub context: swtch::Context,
    /// True while `context` holds a live saved kernel context (the
    /// proc blocked mid-syscall).  The sched loop resumes such procs
    /// via swtch; every other Ready proc resumes via trap_return on
    /// `frame`.
    pub blocked_in_kernel: bool,
    /// The rfork parent — EXITS posts the wait record here.  Null for
    /// the staged init.
    pub parent: *mut Proc,
    /// rfork'd children not yet reaped by AWAIT.  RFNOWAIT children
    /// are excluded: they post no record and are invisible to AWAIT.
    pub children_live: usize,
    /// rfork(RFNOWAIT) child: EXITS posts no wait record.
    pub nowait: bool,
    pub note_queue: VecDeque<Note>,
    pub notify_handler: Option<u64>,
    /// The saved-Ureg user VA of the most recently delivered note
    /// (donor `up->noteureg`): set by the note-delivery boundary, read
    /// and cleared by NOTED.  0 = not inside a note handler.
    pub note_ureg: u64,
    /// The last delivered note's bytes (donor `up->lastnote`) — NOTED's
    /// NDFLT arm exits with this message.
    pub last_note: [u8; ERRMAX],
    pub last_note_len: usize,
    /// Posted child-exit records (one per exited child, consumed by
    /// AWAIT in post order).
    pub wait_records: VecDeque<WaitRecord>,
    pub errstr: [u8; ERRMAX],
    /// The namespace group handle (the mount table; pgrp.rs).
    pub pgrp: Arc<Lock<Pgrp>>,
    /// The environment group handle (devenv.rs).
    pub env_group: Arc<Lock<EnvGroup>>,
    /// The process root and current-directory chans (donor up->slash /
    /// up->dot).  exec_bootstrap sets both before the first ring-3
    /// entry; rfork inherits (value-dup); exec preserves them.
    pub slash: Option<Chan>,
    pub dot: Option<Chan>,
    /// The note-group id — the donor's `noteid`.  A FRESH group id per
    /// construction; rfork(RFPROC) WITHOUT RFNOTEG has the child
    /// INHERIT the parent's group, and non-RFPROC rfork(RFNOTEG)
    /// re-groups the CALLER.
    pub noteid: u64,
    /// True while the proc is parked in a cons read.
    pub cons_waiting: bool,
    /// sys_sleep's wake deadline in TICKS (0 = not sleeping).
    pub sleep_until: u64,
    /// sys_alarm's expiry deadline in TICKS (0 = disarmed).
    pub alarm_at: u64,
    /// The exec-recorded argv — NUL-joined, bounded at ARGV_CAP.
    pub argv: Vec<u8>,
    /// The recorded scheduling priority (devproc ctl's "pri N" — the
    /// honest degradation: nothing READS it but the status report).
    pub pri: u32,
    /// The rendezvous group handle (port::rendez; 9k's Rgrp).
    pub rgrp: Arc<Lock<RendezGroup>>,
    /// The value a rendezvous peer left for us.
    pub rendval: u64,
    /// The RFMEM family id (None at construction, after exec/exit, and
    /// for every non-RFMEM fork child).
    pub rfmem_family: Option<u64>,
}

impl Proc {
    fn new(
        pid: u64,
        name: &str,
        kstack_top: usize,
        kstack_pas: [u64; KSTACK_PAGES],
        kstack_table_pas: [u64; 3],
        pml4_pa: u64,
    ) -> Proc {
        let mut proc = Proc {
            pid,
            name: [0; KNAMELEN],
            state: ProcState::Ready,
            kstack_top,
            kstack_pas,
            kstack_table_pas,
            pml4_pa,
            mapping_tables: Vec::new(),
            fd_group: Arc::new(Lock::new("fdgrp", FdGroup::new())),
            mapping_log: Vec::new(),
            segments: None,
            text_source: None,
            frame: ptr::null_mut(),
            context: swtch::Context::new(),
            note_queue: VecDeque::with_capacity(NOTE_QUEUE_CAP),
            notify_handler: None,
            note_ureg: 0,
            last_note: [0; ERRMAX],
            last_note_len: 0,
            blocked_in_kernel: false,
            parent: ptr::null_mut(),
            children_live: 0,
            nowait: false,
            wait_records: VecDeque::new(),
            errstr: [0; ERRMAX],
            pgrp: Arc::new(Lock::new("pgrp", Pgrp::new())),
            // Fresh EMPTY env group at construction; the first proc's
            // SEEDED root group is assigned by exec_bootstrap, and
            // rfork derives every later group from its parent.
            env_group: Arc::new(Lock::new("env", EnvGroup::new())),
            slash: None,
            dot: None,
            noteid: NEXT_NOTEID.fetch_add(1, Ordering::Relaxed),
            cons_waiting: false,
            sleep_until: 0,
            alarm_at: 0,
            argv: Vec::new(),
            pri: 0,
            rgrp: Arc::new(Lock::new("rendez", RendezGroup::new())),
            rendval: 0,
            rfmem_family: None,
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

    /// Record the exec argv (NUL-joined, the donor's `p->args` shape)
    /// for devproc's /proc/N/args.  Truncation rule: args append
    /// NUL-terminated while the result stays <= ARGV_CAP; the first arg
    /// that would overflow ends the record.
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
    /// OREAD, 1/2 OWRITE).  Called at the end of construction.
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
    // fd ops — the checked-out-chan pattern (fdgrp.rs's header): the
    // aarch64 port's set, verbatim.
    // -------------------------------------------------------------------

    /// Take the chan OUT of fd's slot.  None for out-of-range/closed.
    pub fn fd_take(&mut self, fd: u64) -> Option<Chan> {
        let node = LockNode::new();
        self.fd_group.lock(&node).take(fd)
    }

    /// Put a checked-out chan BACK after its op.  An occupied slot is
    /// close-dispatched defensively and the anomaly logged, loudly.
    pub fn fd_store(&mut self, fd: u64, c: Chan) {
        let node = LockNode::new();
        if let Some(evicted) = self.fd_group.lock(&node).store(fd, c) {
            println!("proc: fd {fd} occupied at write-back — closing the evicted chan");
            port::namec::close_chan(&evicted);
        }
    }

    /// A REFERENCE clone of the chan at fd (Chan::dup).
    pub fn fd_ref_clone(&mut self, fd: u64) -> Option<Chan> {
        let node = LockNode::new();
        self.fd_group.lock(&node).ref_clone(fd)
    }

    /// The transient op-reference pair for blocking-capable syscalls.
    pub fn fd_op_ref(&mut self, fd: u64) -> Option<Chan> {
        let node = LockNode::new();
        self.fd_group.lock(&node).op_ref(fd)
    }

    /// The op-ref's return half.
    pub fn fd_op_done(&mut self, fd: u64, c: Chan) {
        let node = LockNode::new();
        self.fd_group.lock(&node).op_done(fd, c)
    }

    /// The chan's walked path at fd (sys_fd2path's source).
    pub fn fd_path(&mut self, fd: u64) -> Option<Vec<u8>> {
        let node = LockNode::new();
        self.fd_group.lock(&node).get(fd).map(|c| c.path.clone())
    }

    /// Install `c` at the lowest free slot.  On a full table the chan
    /// is close-dispatched HERE and None returns.
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
    /// chan for the caller to close.  An out-of-range target
    /// close-dispatches `c` HERE and returns None.
    pub fn fd_install_at(&mut self, fd: u64, c: Chan) -> Option<Option<Chan>> {
        if fd as usize >= MAX_FDS {
            port::namec::close_chan(&c);
            return None;
        }
        let node = LockNode::new();
        Some(self.fd_group.lock(&node).store(fd, c))
    }

    /// sys_pipe's pair allocation (two free slots found BEFORE
    /// anything installs).
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

    /// Set the errstr string (Plan 9 ERRMAX-bounded,
    /// NUL-terminated) — the syscall error side channel.
    pub fn set_errstr(&mut self, msg: &str) {
        let bytes = msg.as_bytes();
        let n = bytes.len().min(ERRMAX - 1);
        self.errstr = [0; ERRMAX];
        self.errstr[..n].copy_from_slice(&bytes[..n]);
    }

    /// Validate a user-VA range against the proc's segment map BEFORE
    /// the kernel reads or writes it.  The range must lie wholly inside
    /// ONE of the proc's segments (text/data/bss/stack/tos).  A
    /// zero-length range is trivially valid.  `writable` is accepted
    /// for the signature and the future read-only-text enforcement —
    /// membership is the enforceable check this era.
    ///
    /// Caveat (single-process era, carried): membership is a SEGMENT
    /// check — an in-segment VA whose page has not been paged in yet
    /// would take a kernel-mode #PF if the kernel dereferenced it here
    /// (loud panic, never silent corruption).  The measured binaries'
    /// syscall buffers are made resident by the usermem constructors
    /// (ensure_user_resident) or are resident by construction.
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

    /// The 14-03 usermem predicate plugs in here: Proc implements
    /// `UserAddressSpace` (validate_user_range + ensure_user_resident)
    /// — the module compiled ahead of this port against the trait, so
    /// the choke point's surface is unchanged.
    fn ensure_user_resident_inherent(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let mut page = start & !(PAGE_SIZE_4K - 1);
        while page < end {
            if !self.mapping_log.iter().any(|m| m.va == page) {
                // An unmapped in-segment page is bss-growth or
                // stack-growth by construction (text/data/tos are
                // exec-mapped); classify for the log's record.
                let kind = match self.segments.as_ref().and_then(|s| s.classify(page)) {
                    Some(SegKind::Stack) => SegKind::Stack,
                    _ => SegKind::Bss,
                };
                self.page_in_zeroed(page, kind);
            }
            page += PAGE_SIZE_4K;
        }
    }

    /// The note-delivery path's stack window residency (kind is always
    /// Stack — the aarch64 fault::ensure_stack_resident shape).
    pub(crate) fn ensure_stack_resident(&mut self, start: usize, end: usize) {
        let mut page = start & !(PAGE_SIZE_4K - 1);
        while page < end {
            if !self.mapping_log.iter().any(|m| m.va == page) {
                self.page_in_zeroed(page, SegKind::Stack);
            }
            page += PAGE_SIZE_4K;
        }
    }

    /// Map a zeroed page (bss/stack/tos growth) into this proc's table
    /// and record it.  The zero fill rides the kernel alias (works for
    /// the active table and any other alike).
    fn page_in_zeroed(&mut self, page_va: usize, kind: SegKind) {
        let pa = alloc_phys_page().unwrap_or_else(|_| panic!("proc: phys page alloc failed"));
        // (Pre-zeroed by the seam; only a PRESENT-leaf replacement
        // would need the invalidation, and the VA was unmapped.)
        let created = pml4_map_user(self.pml4_pa, page_va, pa, kind.pte())
            .unwrap_or_else(|_| panic!("proc: user page map failed at {page_va:#x}"));
        for c in created.into_iter().flatten() {
            self.mapping_tables.push(c);
        }
        self.mapping_log.push(MappingRecord {
            va: page_va,
            phys: pa as usize,
            size: PAGE_SIZE_4K,
            kind,
        });
    }

    /// Map one 4 KiB user page (fresh, pre-zeroed) at `va` in this
    /// proc's table with the segment's leaf kind, record it, and
    /// return the page's PA (exec's install body consumer).
    pub(crate) fn map_user_page(&mut self, va: usize, kind: SegKind) -> Result<u64> {
        let pa = alloc_phys_page()?;
        let created = pml4_map_user(self.pml4_pa, va, pa, kind.pte())?;
        for c in created.into_iter().flatten() {
            self.mapping_tables.push(c);
        }
        self.mapping_log.push(MappingRecord { va, phys: pa as usize, size: PAGE_SIZE_4K, kind });
        Ok(pa)
    }

    /// EXEC/EXITS: unmap and free every user mapping (the mapping_log
    /// walk), then drop the segment map and the demand-page byte
    /// source.  Each leaf is cleared BEFORE its phys page goes back to
    /// the allocator — a stale translation pointing at a recycled page
    /// would be silent corruption (pml4_unmap_user invalidates).
    pub fn free_user_mappings(&mut self) {
        let records = core::mem::take(&mut self.mapping_log);
        for rec in records {
            pml4_unmap_user(self.pml4_pa, rec.va);
            // RFMEM: an RFMEM-shared page's leaf is cleared above, but
            // the PHYS page goes back only when this release is the
            // last sharer's; siblings keep mapping it until their own
            // exit/exec.
            if release_user_page(rec.phys as u64) {
                free_phys_page(rec.phys as u64);
            }
        }
        // The user-half intermediate tables die with the address space.
        for pa in core::mem::take(&mut self.mapping_tables) {
            free_phys_page(pa);
        }
        self.segments = None;
        self.text_source = None;
        // The address space the RFMEM family co-owns is gone — dissolve
        // the membership (the shared pages were released by the
        // refcount pivot above).
        rfmem_leave(self);
    }

    /// EXITS: free the PML4 page, the kernel-stack pages, and their
    /// intermediate tables.  Runs LAST in the exit path: the handler is
    /// still executing ON the kstack and CR3 still names the PML4 when
    /// this runs — safe because nothing allocates until the hand-off
    /// (single CPU, IRQ-masked syscall context) and the pages' contents
    /// stay intact until reuse (the bitmap free marks, never zeroes).
    pub fn free_root_table_and_kstack(&mut self) {
        free_phys_page(self.pml4_pa);
        for pa in self.kstack_pas {
            free_phys_page(pa);
        }
        for pa in self.kstack_table_pas {
            free_phys_page(pa);
        }
        self.kstack_pas = [0; KSTACK_PAGES];
        self.kstack_table_pas = [0; 3];
    }

    /// Seed a forkret-shaped 184-byte Ureg at the top of this proc's
    /// kstack, through the top page's kernel alias (the frame's VA is
    /// inside THIS proc's PML4 — an alias write is the correct edit
    /// path from any context, exec's install_image consumer).  Returns
    /// the frame's VA under this proc's table.
    pub(crate) fn seed_frame(
        &mut self,
        entry: u64,
        user_sp: u64,
        tos_va: u64,
    ) -> *mut Ureg {
        let top_pa = self.kstack_pas[KSTACK_PAGES - 1];
        let off = PAGE_SIZE_4K - core::mem::size_of::<Ureg>();
        // SAFETY: top_pa names this proc's top kstack page (recorded at
        // construction); off..off+184 lies inside it; nothing else
        // aliases it until the first ring-3 entry.
        let dst = unsafe { phys_ptr_mut(top_pa).add(off) as *mut Ureg };
        // SAFETY: dst is 8-aligned (184 bytes at a page tail) and
        // writeable for one Ureg.
        unsafe {
            ptr::write_volatile(dst, Ureg::zeroed());
            (*dst).seed_exec_entry(tos_va, user_sp, entry);
        }
        (self.kstack_top - core::mem::size_of::<Ureg>()) as *mut Ureg
    }
}

impl crate::usermem::UserAddressSpace for Proc {
    fn validate_user_range(
        &self,
        va: u64,
        len: u64,
        writable: bool,
    ) -> core::result::Result<(), &'static str> {
        Proc::validate_user_range(self, va, len, writable)
    }

    fn ensure_user_resident(&mut self, start: usize, end: usize) {
        Proc::ensure_user_resident_inherent(self, start, end)
    }
}

/// The process table — a small fixed array behind the mcslock idiom.
pub struct ProcTable {
    procs: [Option<Proc>; MAX_PROCS],
    next_pid: u64,
}

pub static PROC_TABLE: Lock<ProcTable> =
    Lock::new("proc", ProcTable { procs: [const { None }; MAX_PROCS], next_pid: 1 });

/// The current-proc pointer — written in process/boot context, read in
/// IRQ-masked trap context (single CPU: stable for a handler's
/// lifetime), atomic so every access is data-race free.
static CURRENT_PROC: AtomicPtr<Proc> = AtomicPtr::new(ptr::null_mut());

/// The current process's raw pointer.  Returns null before the first
/// exec.
pub fn current_proc() -> *mut Proc {
    CURRENT_PROC.load(Ordering::Relaxed)
}

/// Install the current-process pointer (the exec/switch path only).
pub(crate) fn set_current_proc(p: *mut Proc) {
    CURRENT_PROC.store(p, Ordering::Relaxed)
}

/// The ONE guarded entry point for dereferencing the current process.
/// None before the first exec.
pub fn current() -> Option<ProcGuard> {
    let p = CURRENT_PROC.load(Ordering::Relaxed);
    if p.is_null() { None } else { Some(ProcGuard(p)) }
}

/// True while any non-Moribund proc exists (the EXITS path: when none
/// remain, the all-exited marker and the clean halt follow).
pub fn any_active_procs() -> bool {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table.procs.iter().flatten().any(|p| p.state != ProcState::Moribund)
}

/// The shared mid-construction unwind for new_user_proc — free the
/// PML4 page, the kstack pages, and the kstack's intermediate tables
/// (the caller passes exactly the live prefixes).  Never called with a
/// lock held.
fn unwind_new_proc(
    pml4_pa: u64,
    kstack_pas: &[u64],
    kstack_table_pas: &[u64],
    context: &'static str,
) {
    free_phys_page(pml4_pa);
    for &pa in kstack_table_pas {
        free_phys_page(pa);
    }
    for &pa in kstack_pas {
        free_phys_page(pa);
    }
    let _ = context;
}

/// Allocate and register a fresh user process: pid, 16 KiB kernel
/// stack at the fixed kernel-half window, and a per-process PML4 with
/// the kernel higher half shared (the OQ-2 resolution).  The proc
/// starts with no segments, no byte source, and an empty mapping log;
/// fd 0/1/2 are the devcons preopen.
///
/// Returns a ProcGuard, never a `&'static mut Proc`.
pub fn new_user_proc(name: &str) -> Result<ProcGuard> {
    // The per-process PML4 first (BEFORE taking the table lock — the
    // lock is never held across an allocation): one fresh zeroed page;
    // the kernel half (510/511) is copied from the boot PML4 so the
    // Mach window and KZERO alias are live under EVERY table.
    let pml4_pa = alloc_phys_page().map_err(|_| "proc: PML4 page alloc failed")?;
    #[cfg(not(test))]
    {
        let [e510, e511] = kernel_half_entries();
        set_table_entry(pml4_pa, 510, e510);
        set_table_entry(pml4_pa, 511, e511);
    }
    // The kstack window: PML4[400] -> PDPT -> PD -> PT, kernel-only
    // (no PTE_US — ring 3 can never reach the window even though its
    // VA is fixed and known).  Explicit (not lazily walked) so the
    // mid-construction unwind knows exactly what to free.
    let tables = match (alloc_phys_page(), alloc_phys_page(), alloc_phys_page()) {
        (Ok(a), Ok(b), Ok(c)) => [a, b, c],
        _ => {
            unwind_new_proc(pml4_pa, &[], &[], "kstack-tables");
            return Err("proc: kstack table alloc failed");
        }
    };
    let [pdpt_pa, pd_pa, pt_pa] = tables;
    set_table_entry(pml4_pa, KSTACK_PML4_INDEX, pdpt_pa | PTE_P | PTE_RW);
    set_table_entry(pdpt_pa, 0, pd_pa | PTE_P | PTE_RW);
    set_table_entry(pd_pa, 0, pt_pa | PTE_P | PTE_RW);
    let kstack_table_pas = tables;
    let mut kstack_pas = [0u64; KSTACK_PAGES];
    for i in 0..KSTACK_PAGES {
        let pa = match alloc_phys_page() {
            Ok(pa) => pa,
            Err(_) => {
                unwind_new_proc(pml4_pa, &kstack_pas[..i], &kstack_table_pas, "kstack-pages");
                return Err("proc: kernel stack page alloc failed");
            }
        };
        kstack_pas[i] = pa;
        set_table_entry(pt_pa, i, pa | PTE_P | PTE_RW);
    }
    let kstack_top = KSTACK_VA + KSTACK_PAGES * PAGE_SIZE_4K;

    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let pid = table.next_pid;
    table.next_pid += 1;
    let Some(slot) = table.procs.iter_mut().find(|s| s.is_none()) else {
        // The table-full path must free the already-allocated pages.
        // Drop the PROC_TABLE guard BEFORE the frees (the module
        // locking discipline); the burned pid is harmless.
        drop(table);
        unwind_new_proc(pml4_pa, &kstack_pas, &kstack_table_pas, "table-full");
        return Err("proc: table full");
    };
    let mut proc = Proc::new(pid, name, kstack_top, kstack_pas, kstack_table_pas, pml4_pa);
    proc.preopen_cons_fds();
    *slot = Some(proc);
    // The table is a fixed static and slots are never freed this era,
    // so the handle stays valid after the guard drops.
    Ok(ProcGuard(ptr::from_mut(slot.as_mut().unwrap())))
}

/// A guard handle to a PROC_TABLE-resident Proc.  Deref/DerefMut reach
/// the proc; the table slots are never freed this era and handles live
/// only in boot/syscall context (single CPU, IRQ-masked), so the target
/// is stable for the guard's lifetime.
pub struct ProcGuard(*mut Proc);

impl Deref for ProcGuard {
    type Target = Proc;

    fn deref(&self) -> &Proc {
        // SAFETY: new_user_proc's invariant — the Proc lives in a
        // static PROC_TABLE slot that is never freed/moved this era.
        unsafe { &*self.0 }
    }
}

impl DerefMut for ProcGuard {
    fn deref_mut(&mut self) -> &mut Proc {
        // SAFETY: Deref's invariant; the caller holds no other live
        // handle to this proc.
        unsafe { &mut *self.0 }
    }
}

// ---------------------------------------------------------------------------
// The scheduler: tick-driven preemptive round-robin over PROC_TABLE
// plus the swtch-based kernel-context path for procs blocked
// mid-syscall.
//
// Two resume paths, never mixed:
// - Ready procs with NO kernel context (fresh from rfork, or preempted
//   at ring 3) resume through trap_return on their `frame` — the ONLY
//   way ring 3 is entered.  The locked switch step (state, current
//   pointer, TSS rsp0, CURFRAME, CR3) installs everything the resume
//   path reads BEFORE the resume happens.
// - Procs blocked mid-syscall (AWAIT) hold a saved kernel `context`
//   and resume via swtch from the sched loop; their syscall handler
//   then returns through the SYSCALL return tail.
//
// Time slice: the LAPIC timer tick is the preemption source (100 Hz =
// 10 ms this era).
//
// Ring-3 frame placement invariant: interrupts fire only at ring 3
// (kernel context runs IF=0), and the IDT delivery loads rsp0 — so a
// preempted proc's hardware frame lands at exactly kstack_top-184
// every time; CURFRAME records it (reschedule's stash).
// ---------------------------------------------------------------------------

/// The live-frame record: written by reschedule's locked step and read
/// by the note-delivery boundary (the frame the return path actually
/// restores).  SyncUnsafeCell, never `static mut`; written only under
/// the PROC_TABLE lock or in IRQ-masked trap context.
static CURFRAME: SyncUnsafeCell<u64> = SyncUnsafeCell::new(0);

fn read_curframe() -> *mut Ureg {
    // SAFETY: single CPU; CURFRAME is written only under the
    // PROC_TABLE lock or in IRQ-masked trap context (the module
    // header) — no concurrent access.
    unsafe { CURFRAME.get().read() as *mut Ureg }
}

/// The live Ureg the resume boundary will restore (the note-delivery
/// path reads it).
pub(crate) fn current_frame() -> *mut Ureg {
    read_curframe()
}

fn write_curframe(frame: *mut Ureg) {
    // SAFETY: read_curframe's discipline.
    unsafe { CURFRAME.get().write(frame as u64) }
}

/// The Mach's own scheduler stack top (the swtch-based sched loop's
/// context runs here — free while the idle loop owns the entry).
#[cfg(not(test))]
fn scheduler_stack_top() -> usize {
    // SAFETY: the Mach at the fixed KCPUZERO VA (l.S's mapping); a
    // read of its embedded stack's top address.
    unsafe { (*(crate::dat::KMACH as *const crate::dat::Mach)).stack.top().addr() }
}

/// First activation (exec_bootstrap's only call): publish the proc as
/// current + Running, point the TSS rsp0 at its kstack — the mechanism
/// every ring-3 entry (SYSCALL and IDT alike) then shares — and record
/// the seeded frame in CURFRAME (the live-frame record the scheduler's
/// preempt path stashes from; the aarch64 activate_first published the
/// CURKSTACK analog).
pub fn activate_first(p: &mut Proc) {
    p.state = ProcState::Running;
    set_current_proc(p);
    set_tss_rsp0(p.kstack_top as u64);
    write_curframe(p.frame);
}

/// The timer-tick preemption entry.  Acts only when the current proc
/// is Running — i.e. the IRQ interrupted ring 3 (kernel syscall/trap
/// context runs IRQ-masked; the sched loop's idle hlt finds current
/// Blocked/Moribund and its own post-hlt re-scan dispatches instead).
pub fn preempt() {
    let Some(cur) = current() else {
        return;
    };
    if cur.state != ProcState::Running {
        return;
    }
    reschedule();
}

/// Round-robin reschedule from the tick (IRQ-from-ring-3: the live
/// frame sits at the current proc's kstack top, recorded in CURFRAME).
/// When another runnable proc exists: PARK — stash the live frame,
/// mark Ready + blocked_in_kernel, drop the lock, and swtch into the
/// scheduler context (the Mach's own stack, which no CR3 switch can
/// alias).  The sched loop then performs the real switch (pick_next's
/// locked step) and resumes procs per their kind; when THIS proc is
/// eventually picked, the swtch returns here and the ordinary
/// interrupt-return path iretq's into the stashed frame — the whole
/// round trip runs on this proc's OWN kstack.
///
/// 14-06 root cause this shape replaces: the old reschedule performed
/// the CR3 switch ITSELF, inside the lock scope, while executing on
/// the CURRENT proc's kstack at the FIXED window VA (PML4[400]) — the
/// moment CR3 flipped, that VA aliased the NEXT proc's kstack pages,
/// destroying every spilled local, the MCS LockNode, and the return
/// chain mid-flight (the post-switch println read clobbered locals —
/// the observed `sched: switch 0->0` — and the lock drop then walked
/// a node of the wrong proc's bytes, the wild-write that corrupted
/// kernel statics).  Park-and-swtch never switches CR3 in the
/// preempted proc's context at all — the same discipline sched() has
/// proven since 14-05's await path.
pub fn reschedule() {
    let Some(mut cur) = current() else { return };
    let cur_pid = cur.pid;
    let node = LockNode::new();
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
        // No other runnable proc: return through OUR OWN stack — no
        // switch ever happens and the interrupt-return is ordinary.
        let Some(_ni) = pick else { return };
        let curp = table.procs[cur_idx].as_mut().unwrap();
        curp.frame = read_curframe();
        curp.state = ProcState::Ready;
        // The parked proc resumes via the sched loop's Switch arm (the
        // swtch below lands mid-interrupt on this kstack — trap_return
        // would be wrong), so mark it like sched()'s blocked procs.
        curp.blocked_in_kernel = true;
    }
    println!("sched: park {cur_pid}");
    // Park into the scheduler context.  The swtch saves this proc's
    // callee-saved state into its PROC_TABLE-resident context — never
    // this kstack — so nothing under this frame is invalidated by any
    // later switch; the Mach stack is mapped identically under every
    // table (the kernel half is shared).
    // SAFETY: sched_context()'s seed-once discipline; cur.context is
    // PROC_TABLE-resident (a stable address across the switch).
    unsafe {
        swtch::swtch(&mut cur.context, &*sched_context());
    }
    // Resumed: pick_next re-installed this proc (current + rsp0 +
    // CURFRAME + CR3, one locked step) before swtch'ing back — our
    // kstack pages own this VA again.  Return up through preempt and
    // the interrupt path; the iretq pops the frame at our kstack top.
}

/// How the sched loop resumes the picked proc: swtch into its saved
/// kernel context (blocked mid-syscall) or trap_return into its frame
/// (everything else).
enum Resume {
    Switch(*mut swtch::Context),
    Return(*mut Ureg),
}

/// The scheduler context — a kernel-context switch target running
/// sched_loop on the Mach's own scheduler stack.  Seeded on first use;
/// accessed only from IRQ-masked single-CPU context.
static SCHED_CONTEXT: SyncUnsafeCell<swtch::Context> =
    SyncUnsafeCell::new(swtch::Context::new());
static SCHED_SEEDED: SyncUnsafeCell<bool> = SyncUnsafeCell::new(false);

fn sched_context() -> *mut swtch::Context {
    // SAFETY: single CPU, IRQ-masked contexts only: the seed-once
    // write and every later swtch access are serialized by the caller
    // contexts — no concurrent access is possible.
    unsafe {
        if !SCHED_SEEDED.get().read() {
            let ctx = SCHED_CONTEXT.get();
            (*ctx).set_return(sched_loop as *const () as u64);
            // SysV wants %rsp ≡ 8 (mod 16) at a function entry (as
            // after a call); the Mach stack top is 64 KiB-aligned, so
            // seed one qword down.
            #[cfg(not(test))]
            (*ctx).set_stack_pointer(scheduler_stack_top() as u64 - 8);
            SCHED_SEEDED.get().write(true);
        }
        SCHED_CONTEXT.get()
    }
}

/// Block the CURRENT proc mid-syscall and switch to the scheduler
/// context (the AWAIT path).  Returns when the sched loop swtch'es back
/// — after the proc was woken and picked.  Ring-3 entry/exit never
/// rides this path.
pub fn sched() {
    let mut cur = current().expect("sched with no current process");
    cur.state = ProcState::Blocked;
    cur.blocked_in_kernel = true;
    // SAFETY: sched_context()'s seed-once discipline; cur's context
    // field is PROC_TABLE-resident (a stable address across the
    // switch).
    unsafe {
        swtch::swtch(&mut cur.context, &*sched_context());
    }
    // Resumed inside the syscall handler; the caller re-checks its
    // wait records and returns up the return tail.
}

/// The scheduler loop (entered via swtch on the Mach's scheduler
/// stack): pick the next Ready proc — pick_next applies the TSS rsp0 +
/// CURFRAME + CR3 + current-proc swap under the PROC_TABLE lock
/// BEFORE the resume — then resume it per its kind.  Nothing Ready →
/// hlt with IRQs briefly unmasked: the tick fires, re-arms, and the
/// loop re-scans.
fn sched_loop() -> ! {
    let mut all_blocked = 0u32;
    loop {
        match pick_next() {
            // SAFETY: sched_context() is seeded; ctx names the picked
            // proc's PROC_TABLE-resident saved context.
            Some(Resume::Switch(ctx)) => unsafe { swtch::swtch(&mut *sched_context(), &*ctx) }
            // SAFETY: frame names the picked proc's kstack-resident
            // Ureg (pick_next installed the switch under the lock just
            // above — CR3 included); trap_return iretqs into it and
            // diverges.
            Some(Resume::Return(frame)) => unsafe { crate::trap::trap_return(frame) },
            None => {
                all_blocked += 1;
                if all_blocked == 200_000 {
                    println!("wedgedbg: fully idle — dumping (pipe-RPC wedge shape)");
                    dump_proc_states();
                }
                // The consolidated idle-wait primitive (briefly
                // unmasked hlt — the tick fires and re-arms).
                crate::trap::idle_wait();
            }
        }
    }
}

/// gatedbg: one wire line per live proc — the wedge autopsy.  Holds
/// PROC_TABLE while printing (PROC_TABLE -> Console is the documented
/// nesting order; reschedule's switch print is the precedent).
pub(crate) fn dump_proc_states() {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    for p in table.procs.iter().flatten() {
        if p.state == ProcState::Moribund {
            continue;
        }
        let n = p.name.iter().position(|&b| b == 0).unwrap_or(KNAMELEN);
        let name = core::str::from_utf8(&p.name[..n]).unwrap_or("?");
        // 14-06 cycle 3: the dump must SURVIVE a corrupt parent —
        // it exists to diagnose wedges, and the first live run of the
        // chain wedged with one proc holding a garbage parent pointer
        // (the dump crashed before naming the blocked procs).  Print
        // the raw pointer when it is outside the table; the audit of
        // WHO set it follows from the printed value.
        let lo = ptr::from_ref(&PROC_TABLE) as usize;
        let hi = lo + core::mem::size_of::<ProcTable>();
        let pp = p.parent as usize;
        let parent_pid = if pp == 0 {
            0
        } else if (lo..=hi).contains(&pp) {
            // SAFETY: bounds-proven table-resident; the lock is held.
            unsafe { (*(pp as *const Proc)).pid }
        } else {
            0xffff
        };
        let argv_str = core::str::from_utf8(&p.argv).unwrap_or("?");
        println!(
            "wedgedbg: pid={} state={} cons_wait={} blocked_in_kernel={} sleep_until={} name={} parent={} (raw {:#x}) argv={}",
            p.pid,
            state_name(p.state),
            p.cons_waiting,
            p.blocked_in_kernel,
            p.sleep_until,
            name,
            parent_pid,
            pp,
            argv_str
        );
    }
}

/// Round-robin pick of the next Ready proc after the current one,
/// applying the full switch as one PROC_TABLE-locked step (state,
/// current-proc pointer, TSS rsp0, CURFRAME, CR3) so the resumed
/// proc's return path reads its OWN frame, never the previous proc's.
/// None when no other Ready proc exists.
///
/// The current proc is NOT skipped — only its scan PRIORITY wraps
/// (the round-robin start is cur+1, so cur is picked last; the
/// cons-read deadlock the aarch64 port documented lives here too).
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
        set_tss_rsp0(p.kstack_top as u64);
        write_curframe(p.frame);
        // CR3 last (reschedule's ordering note): the frame pointer is
        // dereferenced only after the lock drops, under this table.
        switch_cr3(p.pml4_pa);
        return Some(if p.blocked_in_kernel {
            p.blocked_in_kernel = false;
            Resume::Switch(&mut p.context as *mut swtch::Context)
        } else {
            Resume::Return(p.frame)
        });
    }
    None
}

/// EXITS's tail: the Moribund current proc never runs again — switch
/// into the scheduler context with no wake path.  The swtch save
/// writes only the PROC_TABLE-resident context struct (never the freed
/// kstack), so EXITS can free the kstack before calling here.
/// Diverges.
pub fn sched_from_moribund() -> ! {
    let cur = current();
    // SAFETY: sched_context()'s seed-once discipline; the moribund
    // proc's context is PROC_TABLE-resident and its kstack is already
    // freed (the swtch save writes only the context struct).
    unsafe {
        let ctx = sched_context();
        if let Some(mut cur) = cur {
            swtch::swtch(&mut cur.context, &*ctx);
        }
    }
    unreachable!("sched: moribund process resumed");
}

/// Non-RFPROC rfork's namespace-group half (9k sysproc.c's non-RFPROC
/// branch): the CALLER's group becomes a fresh deep COPY when `copy`
/// is set (RFNAMEG) or a fresh EMPTY group otherwise (RFCNAMEG).
pub fn rfork_namespace_split(proc: &mut Proc, copy: bool) {
    let next = if copy {
        let node = LockNode::new();
        proc.pgrp.lock(&node).deep_copy()
    } else {
        Pgrp::new()
    };
    proc.pgrp = Arc::new(Lock::new("pgrp", next));
}

/// Non-RFPROC rfork's env-group half: the CALLER's group becomes a
/// fresh deep COPY when `copy` is set (RFENVG) or fresh EMPTY
/// otherwise (RFCENVG).
pub fn rfork_env_split(proc: &mut Proc, copy: bool) {
    let next = if copy {
        let node = LockNode::new();
        proc.env_group.lock(&node).deep_copy()
    } else {
        EnvGroup::new()
    };
    proc.env_group = Arc::new(Lock::new("env", next));
}

/// devenv's current-group hook: the CURRENT proc's env-group handle.
pub fn current_env_group() -> Option<Arc<Lock<EnvGroup>>> {
    Some(current()?.env_group.clone())
}

/// devpipe's block/wake hooks (the port crate cannot see PROC_TABLE,
/// so these are injected at boot).
///
/// The calling proc's pid (waiter registration).
pub fn pipe_current_pid() -> u64 {
    current().expect("devpipe op with no current process").pid
}

/// Park the calling proc mid-syscall until a pipe write/close wakes it.
pub fn pipe_block_current() {
    sched();
}

/// Wake the procs blocked on a pipe queue: Blocked -> Ready.
pub fn pipe_wake(pids: &[u64]) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.state == ProcState::Blocked && pids.contains(&p.pid) {
            p.state = ProcState::Ready;
        }
    }
}

/// devmnt's RPC-contention wait: park the caller ~1 tick and let its
/// enter() retry — qlock(9)'s block-and-retry at this era's
/// granularity.  dev_exec-only (the tick wiring is the exec profiles').
#[cfg(dev_exec)]
pub fn mnt_contention_wait() {
    let mut cur = current().expect("mnt contention wait with no current process");
    cur.sleep_until = crate::platform::q35::timer::ticks() + 1;
    sched();
}

/// The exit-time shared empty fd group: exit_tail detaches the dying
/// proc's group by REPLACING it, and the husk keeps the replacement
/// forever (slots are never reaped this era) — every husk shares ONE
/// empty group instead (a dead proc never touches its group again).
pub fn empty_fdgroup() -> Arc<Lock<FdGroup<MAX_FDS>>> {
    let node = LockNode::new();
    let mut guard = EMPTY_FDGROUP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("fdgrp", FdGroup::<MAX_FDS>::new())));
    }
    guard.as_ref().unwrap().clone()
}

/// The one shared empty fd group (lazily built at the first exit).
static EMPTY_FDGROUP: Lock<Option<Arc<Lock<FdGroup<MAX_FDS>>>>> = Lock::new("emptyfdgrp", None);

/// devproc-lite's write hook: append the note to the ADDRESSED pid's
/// queue — no broadcast, no group semantics.  Delivery happens at the
/// target's next return-to-ring-3 boundary.
pub fn post_note(pid: u64, msg: &[u8]) -> Result<()> {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let Some(p) = table.procs.iter_mut().flatten().find(|p| p.pid == pid) else {
        return Err("devproc: no such process");
    };
    push_note(p, msg)
}

/// The donor statename string for a ProcState.
fn state_name(s: ProcState) -> &'static str {
    match s {
        ProcState::Ready => "Ready",
        ProcState::Running => "Running",
        ProcState::Blocked => "Blocked",
        ProcState::Moribund => "Moribund",
    }
}

/// devproc's root-listing hook: the LIVE (non-Moribund) pids.
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

/// devproc's per-pid info hook: the status/args data.
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

/// devproc's ctl hook: record the "pri N" write on the ADDRESSED pid.
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

/// The note-queue append shared by post_note and the note-group post:
/// Moribund targets and full queues are skipped per the donor's
/// postnote failure shape.
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
    Ok(())
}

/// devcons's DEL path — post a note to the note GROUPS of the given
/// reader pids.
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

/// The calling proc's pid (devcons's `current` hook).
pub fn current_pid() -> u64 {
    current().expect("device op with no current process").pid
}

/// devcons's block hook: park the calling proc in its cons read until
/// the tick's cons_input_wake readies it.
pub fn cons_block_current() {
    let mut cur = current().expect("cons read with no current process");
    cur.cons_waiting = true;
    sched();
    cur.cons_waiting = false;
}

/// The tick's cons wake: ready every cons-waiting Blocked proc.
pub fn cons_input_wake() {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.state == ProcState::Blocked && p.cons_waiting {
            p.state = ProcState::Ready;
        }
    }
}

/// Non-RFPROC rfork(RFNOTEG)'s note-group half: the CALLER joins a
/// fresh note group.
pub fn rfork_noteg_split(proc: &mut Proc) {
    proc.noteid = NEXT_NOTEID.fetch_add(1, Ordering::Relaxed);
}

/// Non-RFPROC rfork(RFREND)'s rendezvous-group half: the CALLER joins
/// a fresh empty group.
pub fn rfork_rendez_split(proc: &mut Proc) {
    proc.rgrp = Arc::new(Lock::new("rendez", RendezGroup::new()));
}

/// The rendezvous wake half: the sleeper named by `pid` is handed the
/// waker's value and readied.
pub fn rendez_wake(pid: u64, value: u64) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let Some(p) = table.procs.iter_mut().flatten().find(|p| p.pid == pid) else {
        return; // died between the park and the wake
    };
    p.rendval = value;
    if p.state == ProcState::Blocked {
        p.state = ProcState::Ready;
    }
}

/// Exit-time rendezvous sweep: a killed/exited sleeper must never
/// linger as a wake target in its group.
pub fn rendez_remove(proc: &mut Proc) {
    let rgrp = proc.rgrp.clone();
    let node = LockNode::new();
    rgrp.lock(&node).remove_pid(proc.pid);
}

/// The tick's timeout sweep: per proc, the ALARM expiry arm runs FIRST
/// (posting the "alarm" note and disarming), THEN the sleep wake arm
/// readies a Blocked sleeper whose deadline passed OR whose note_queue
/// is non-empty.
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

/// devdup/devsrv's fd resolver hook: the CALLING proc's fd N as a
/// reference clone.
pub fn current_fd_resolve(fd: u64) -> Option<Chan> {
    current()?.fd_ref_clone(fd)
}

/// devdup's listing hook: the CALLING proc's open fds as (fd, mode)
/// pairs.
pub fn current_fd_list() -> Vec<(u64, u32)> {
    let Some(p) = current() else {
        return Vec::new();
    };
    let node = LockNode::new();
    p.fd_group.lock(&node).list_open()
}

/// EXITS: post the child's wait record to the parent and wake it if it
/// sits Blocked in AWAIT.  The record is kernel memory in the parent's
/// table slot (no cross-process pointers).
pub fn post_wait_record(parent: *mut Proc, pid: u64, msg: &[u8]) {
    if parent.is_null() {
        return;
    }
    let node = LockNode::new();
    // Held for the posting + wake (the parent pointer is
    // table-resident).
    let _guard = PROC_TABLE.lock(&node);
    // SAFETY: parent names a PROC_TABLE-resident Proc and the table
    // lock is held — the deref cannot race.
    let parent = unsafe { &mut *parent };
    let mut rec = WaitRecord { pid, msg: [0; ERRMAX], msg_len: msg.len().min(ERRMAX) };
    rec.msg[..rec.msg_len].copy_from_slice(&msg[..rec.msg_len]);
    parent.wait_records.push_back(rec);
    if parent.state == ProcState::Blocked {
        parent.state = ProcState::Ready;
    }
}

/// RFORK: clone `parent` — a fresh proc (new kernel stack + PML4 via
/// new_user_proc), the address space per the donor dupseg shape (an
/// EAGER page-for-page copy without RFMEM; with RFMEM the Data/Bss
/// pages SHARE the parent's physical pages, refcounted in
/// SHARED_PAGES — Stack/Tos/Text stay fresh copies), a dup'd fd table,
/// mirrored segment records + byte source, and a copy of the parent's
/// live Ureg on the child's kstack with ax = 0 (donor forkchild; the
/// amd64 wire-result slot).  The child is queued Ready; its first run
/// resumes through the CURFRAME/trap_return path.  Returns the child
/// pid (the parent's wire result).
pub fn rfork_clone(parent: &mut Proc, frame: &Ureg, flag: u64) -> Result<u64> {
    let nowait = flag & RFNOWAIT != 0;
    let name = {
        let n = parent.name.iter().position(|&b| b == 0).unwrap_or(KNAMELEN);
        core::str::from_utf8(&parent.name[..n]).unwrap_or("fork")
    };
    let mut child = new_user_proc(name)?;

    // Everything from here is fallible, and the child is already
    // PROC_TABLE-resident.  A mid-copy Err that just returned would
    // leave the half-built child Runnable in the table — unwind
    // instead: mark Moribund, free the pages already copied, then the
    // PML4 and kstack by their recorded PAs.
    if let Err(msg) = rfork_copy(parent, &mut child, frame, flag) {
        child.state = ProcState::Moribund;
        for rec in core::mem::take(&mut child.mapping_log) {
            if release_user_page(rec.phys as u64) {
                free_phys_page(rec.phys as u64);
            }
        }
        for pa in core::mem::take(&mut child.mapping_tables) {
            free_phys_page(pa);
        }
        child.free_root_table_and_kstack();
        return Err(msg);
    }
    // The copy succeeded — under RFMEM the pair now co-owns the shared
    // Data/Bss pages: record the family membership.
    if flag & RFMEM != 0 {
        rfmem_join(parent, &mut child);
    }
    // An RFNOWAIT child posts no wait record at EXITS and is never
    // reaped by AWAIT, so it must not count toward the parent's AWAIT
    // block-vs-"no children" pivot.
    if !nowait {
        parent.children_live += 1;
    }
    Ok(child.pid)
}

/// The fallible body of rfork_clone after the child exists: an EAGER
/// page-for-page copy of every user mapping, a dup'd fd table, mirrored
/// segment records + byte source, and a copy of the parent's live Ureg
/// on the child's kstack with ax = 0.
fn rfork_copy(parent: &mut Proc, child: &mut Proc, frame: &Ureg, flag: u64) -> Result<()> {
    let nowait = flag & RFNOWAIT != 0;
    let rfmem = flag & RFMEM != 0;
    // The eager copy, with the donor dupseg shape under RFMEM:
    // SG_STACK is ALWAYS a fresh copy (note delivery and the libc Tos
    // machinery write the proc's OWN stack pages); SG_BSS/SG_DATA
    // SHARE under RFMEM — the same PA mapped in both procs' tables,
    // refcounted in SHARED_PAGES against the exit/exec frees.
    let records: Vec<MappingRecord> = parent.mapping_log.clone();
    for rec in &records {
        if rfmem && matches!(rec.kind, SegKind::Data | SegKind::Bss) {
            // The RFMEM share arm: no copy — the parent's PA maps
            // straight into the child's table at the same VA, then the
            // refcount records the second sharer.
            let created =
                pml4_map_user(child.pml4_pa, rec.va, rec.phys as u64, rec.kind.pte())
                    .map_err(|_| "rfork: child shared map failed")?;
            for c in created.into_iter().flatten() {
                child.mapping_tables.push(c);
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
        // RFMEM): the parent's bytes copy through the two pages' kernel
        // aliases into the child's fresh page, which maps into the
        // child's table at the same VA with the same segment kind.
        let pa = alloc_phys_page().map_err(|_| "rfork: page alloc failed")?;
        // (The fresh page is pre-zeroed by the seam; the full-page copy
        // overwrites every byte.)
        // SAFETY: both "PAs" name owned pages; exactly one
        // non-overlapping page is copied.
        unsafe {
            let src = phys_ptr_mut(rec.phys as u64);
            let dst = phys_ptr_mut(pa);
            ptr::copy_nonoverlapping(src, dst, PAGE_SIZE_4K);
        }
        let created = pml4_map_user(child.pml4_pa, rec.va, pa, rec.kind.pte())
            .map_err(|_| "rfork: child user map failed")?;
        for c in created.into_iter().flatten() {
            child.mapping_tables.push(c);
        }
        child.mapping_log.push(MappingRecord {
            va: rec.va,
            phys: pa as usize,
            size: rec.size,
            kind: rec.kind,
        });
    }
    // No icache maintenance on the copy: x86_64 has coherent
    // instruction caches — the aarch64 port's per-page CTR_EL0 flush
    // step is deliberately absent here (the re-key the plan records).

    // The bss sweep (RFMEM only): the log walk above shares only
    // PRESENT pages, but the parent's bss range can cover pages
    // brk_-grown and never touched.  Materialize each absent in-range
    // page now as ONE zeroed page mapped into BOTH tables, closing the
    // hole by construction.
    if rfmem {
        let sweep = parent.segments.as_ref().map(|segs| {
            (segs.bss.0.start & !(PAGE_SIZE_4K - 1), round_up_page(segs.bss.0.end))
        });
        if let Some((start, end)) = sweep {
            let mut va = start;
            while va < end {
                if parent.mapping_log.iter().any(|m| m.va == va) {
                    va += PAGE_SIZE_4K;
                    continue;
                }
                let pa = alloc_phys_page().map_err(|_| "rfork: sweep page alloc failed")?;
                // (Pre-zeroed by the seam.)
                let created = match pml4_map_user(parent.pml4_pa, va, pa, SegKind::Bss.pte()) {
                    Ok(c) => c,
                    Err(_) => {
                        // The page never published a refcount nor a
                        // record — free it or it leaks.
                        free_phys_page(pa);
                        return Err("rfork: sweep parent map failed");
                    }
                };
                for c in created.into_iter().flatten() {
                    parent.mapping_tables.push(c);
                }
                let created = match pml4_map_user(child.pml4_pa, va, pa, SegKind::Bss.pte()) {
                    Ok(c) => c,
                    Err(_) => {
                        // The parent's mapping is UNTRACKED (the record
                        // push below never ran) — unmap it and free the
                        // page, mirroring the brk arm's failure hygiene.
                        pml4_unmap_user(parent.pml4_pa, va);
                        free_phys_page(pa);
                        return Err("rfork: sweep child map failed");
                    }
                };
                for c in created.into_iter().flatten() {
                    child.mapping_tables.push(c);
                }
                let rec = MappingRecord { va, phys: pa as usize, size: PAGE_SIZE_4K, kind: SegKind::Bss };
                parent.mapping_log.push(rec);
                child.mapping_log.push(rec);
                share_new_user_page(pa, 2);
                va += PAGE_SIZE_4K;
            }
        }
    }

    // Mirror the segment map + demand-page byte source so the child's
    // future faults resolve in the child.
    child.segments = parent.segments.clone();
    child.text_source = parent.text_source.clone();

    // The fd GROUP semantics per fork(2): RFCFDG = fresh EMPTY;
    // RFFDG = fresh COPIED slot-by-slot; NEITHER = SHARE the parent's
    // handle (Arc clone).
    child.fd_group = if flag & RFCFDG != 0 {
        Arc::new(Lock::new("fdgrp", FdGroup::new()))
    } else if flag & RFFDG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("fdgrp", parent.fd_group.lock(&node).deep_copy()))
    } else {
        parent.fd_group.clone()
    };

    // The namespace group: RFNAMEG = fresh COPY; RFCNAMEG = fresh
    // EMPTY; NEITHER = share.
    child.pgrp = if flag & RFCNAMEG != 0 {
        Arc::new(Lock::new("pgrp", Pgrp::new()))
    } else if flag & RFNAMEG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("pgrp", parent.pgrp.lock(&node).deep_copy()))
    } else {
        parent.pgrp.clone()
    };
    // The env group: the SAME shape.
    child.env_group = if flag & RFCENVG != 0 {
        Arc::new(Lock::new("env", EnvGroup::new()))
    } else if flag & RFENVG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("env", parent.env_group.lock(&node).deep_copy()))
    } else {
        parent.env_group.clone()
    };
    child.slash = parent.slash.as_ref().map(|c| c.dup());
    child.dot = parent.dot.as_ref().map(|c| c.dup());

    // The forkchild frame: a copy of the parent's live Ureg on the
    // CHILD's kernel stack, with ax = 0 (the child's wire result — the
    // aarch64 x0 re-key).  The write rides the child's top kstack page
    // through the kernel alias (the frame VA lives in the CHILD's
    // table — an alias write is the correct edit path from the
    // parent's context).
    let top_pa = child.kstack_pas[KSTACK_PAGES - 1];
    let off = PAGE_SIZE_4K - core::mem::size_of::<Ureg>();
    // SAFETY: top_pa names the child's top kstack page (recorded at
    // construction); off..off+184 lies inside it; the parent's live
    // frame is the source — disjoint pages.
    unsafe {
        let dst = phys_ptr_mut(top_pa).add(off) as *mut Ureg;
        ptr::copy_nonoverlapping(frame as *const Ureg, dst, 1);
        (*dst).ax = 0;
    }
    child.frame = (child.kstack_top - core::mem::size_of::<Ureg>()) as *mut Ureg;
    child.parent = ptr::addr_of!(*parent) as *mut Proc;
    child.nowait = nowait;
    // Donor newproc: the child INHERITS the notify handler; the note
    // queue/last-note/note_ureg start empty (Proc::new).
    child.notify_handler = parent.notify_handler;
    // The note group: WITHOUT RFNOTEG the child INHERITS the parent's
    // noteid; WITH it the child keeps the fresh group id its
    // construction assigned.
    if flag & RFNOTEG == 0 {
        child.noteid = parent.noteid;
    }
    // The rendezvous group: WITH RFREND the child gets a FRESH empty
    // group; WITHOUT it the child SHARES the parent's handle.
    child.rgrp = if flag & RFREND != 0 {
        Arc::new(Lock::new("rendez", RendezGroup::new()))
    } else {
        parent.rgrp.clone()
    };
    // The recorded ctl priority is inherited; the argv record is NOT —
    // a fork child's args are empty until its own exec.
    child.pri = parent.pri;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A segments map over REAL host memory — the 14-03 mock doctrine:
    /// the segment IS the buffer's address range, so kernel-side
    /// accesses through the "VA" genuinely hit the mock's bytes.
    fn mock_segments() -> Segments {
        let base = 0x1000_0000usize;
        Segments {
            text: VirtRange::with_end(base, base + 0x1000),
            data: VirtRange::with_end(base + 0x2000, base + 0x2100),
            // bss runs past the data page's 4 KiB end so the
            // classifier has a real Bss-only region (the shared
            // last-data/first-bss page classifies as Data).
            bss: VirtRange::with_end(base + 0x2100, base + 0x5000),
            stack: VirtRange::with_end(0x7f00_0000_0000, 0x7f00_0010_0000),
            tos: VirtRange::with_end(0x7f00_0000_0000 - 0x2000, 0x7f00_0000_0000 - 0x1000),
        }
    }

    /// A host-test Proc: pure-field construction, no PML4/kstack —
    /// validate_user_range and the bookkeeping methods never touch
    /// them (the mapping bodies get their own tests below).
    fn mock_proc() -> Proc {
        Proc::new(1, "mock", 0, [0; KSTACK_PAGES], [0; 3], 0)
    }

    /// Membership is per-segment and whole-range: the contained shapes
    /// pass, the straddling/overflowing/outside shapes fail loud (the
    /// shape-verbatim port's pin).
    #[test]
    fn validate_user_range_membership() {
        let mut p = mock_proc();
        // No segments: every range errors.
        assert_eq!(p.validate_user_range(0x1000, 8, false), Err("proc has no segments"));
        p.segments = Some(mock_segments());
        let s = p.segments.clone().unwrap();
        // Wholly inside each of the five segments.
        assert!(p.validate_user_range(s.text.0.start as u64, 8, false).is_ok());
        assert!(p.validate_user_range(s.data.0.start as u64, 0x100, false).is_ok());
        assert!(p.validate_user_range(s.bss.0.start as u64, 0x300, false).is_ok());
        assert!(p.validate_user_range(s.stack.0.start as u64 + 0x1000, 0x1000, false).is_ok());
        assert!(p.validate_user_range(s.tos.0.start as u64, 0x1000, false).is_ok());
        // Zero-length is trivially valid anywhere.
        assert!(p.validate_user_range(s.text.0.start as u64, 0, false).is_ok());
        // Straddling a segment boundary fails (not wholly inside ONE).
        assert_eq!(
            p.validate_user_range(s.data.0.end as u64 - 4, 8, false),
            Err("bad user address")
        );
        // Outside every segment.
        assert_eq!(p.validate_user_range(0x5000_0000, 8, false), Err("bad user address"));
        // Overflow wraps loudly.
        assert_eq!(
            p.validate_user_range(u64::MAX - 4, 8, false),
            Err("user range overflows")
        );
        // A range whose END crosses the segment end fails even when
        // the start is inside.
        assert_eq!(
            p.validate_user_range(s.text.0.end as u64 - 8, 16, false),
            Err("bad user address")
        );
    }

    /// The classifier's shared-page rule: the last data page and the
    /// bss head classify as Data (page-rounded boundary), bss proper as
    /// Bss, and the empty segment classifies nothing.
    #[test]
    fn classify_data_bss_shared_page() {
        let s = mock_segments();
        assert_eq!(s.classify(s.data.0.start), Some(SegKind::Data));
        // data ends at +0x2100 — byte 0x2104 is bss-head, but shares
        // data's last page (0x2000..0x3000): Data by the rule.
        assert_eq!(s.classify(0x1000_0000 + 0x2104), Some(SegKind::Data));
        // Page 0x3000 is bss-only.
        assert_eq!(s.classify(0x1000_0000 + 0x3000), Some(SegKind::Bss));
        assert_eq!(s.classify(s.text.0.start), Some(SegKind::Text));
        assert_eq!(s.classify(s.stack.0.start), Some(SegKind::Stack));
        assert_eq!(s.classify(s.tos.0.start), Some(SegKind::Tos));
        // The gap between bss end and stack classifies nothing.
        assert_eq!(s.classify(0x2000_0000_0000), None);
    }

    /// Wait-record bookkeeping: the record lands in post order, wakes a
    /// Blocked parent, and AWAIT's pivot fields behave (the pure-logic
    /// pin the plan asks for).
    #[test]
    fn wait_record_bookkeeping() {
        let mut parent = mock_proc();
        parent.state = ProcState::Blocked;
        parent.children_live = 2;
        let pp: *mut Proc = &mut parent;
        post_wait_record(pp, 7, b"done");
        assert_eq!(parent.wait_records.len(), 1);
        assert_eq!(parent.wait_records[0].pid, 7);
        assert_eq!(&parent.wait_records[0].msg[..parent.wait_records[0].msg_len], b"done");
        // The Blocked parent was woken.
        assert_eq!(parent.state, ProcState::Ready);
        // A null parent (the staged init) posts nothing, never panics.
        post_wait_record(ptr::null_mut(), 7, b"done");
        // The consumer side: pop decrements children_live.
        let rec = parent.wait_records.pop_front().unwrap();
        assert_eq!(rec.pid, 7);
        parent.children_live = parent.children_live.saturating_sub(1);
        assert_eq!(parent.children_live, 1);
    }

    /// record_argv's truncation rule: args append NUL-joined while they
    /// fit; the first overflow ends the record; an oversize FIRST arg
    /// is byte-truncated with its terminator.
    #[test]
    fn record_argv_truncation() {
        let mut p = mock_proc();
        p.record_argv(&[b"one", b"two", b"three"]);
        assert_eq!(&p.argv, b"one\0two\0three\0");
        // A cap-overflowing tail is dropped whole, never half-written.
        let long: Vec<u8> = vec![b'x'; ARGV_CAP - 10];
        let mut p2 = mock_proc();
        p2.record_argv(&[b"first", &long, b"dropped"]);
        assert_eq!(&p2.argv[..6], b"first\0");
        assert!(p2.argv.len() <= ARGV_CAP);
        // A single oversize first arg keeps ARGV_CAP-1 bytes + NUL.
        let huge: Vec<u8> = vec![b'y'; ARGV_CAP];
        let mut p3 = mock_proc();
        p3.record_argv(&[&huge]);
        assert_eq!(p3.argv.len(), ARGV_CAP);
        assert_eq!(p3.argv[ARGV_CAP - 1], 0);
        assert_eq!(p3.argv[ARGV_CAP - 2], b'y');
    }

    /// RFMEM shared-page refcount decisions (the aarch64 pins,
    /// verbatim).
    #[test]
    fn rfmem_shared_pages_lifecycle() {
        let mut t = SharedPages::new();
        assert!(t.release(0x1000)); // never shared: frees
        t.share(0x2000); // parent + child = 2
        assert!(!t.release(0x2000)); // first sharer exits — sibling holds
        assert!(t.release(0x2000)); // last sharer frees
        t.share(0x3000);
        t.share(0x3000); // + grandchild = 3
        assert!(!t.release(0x3000));
        assert!(!t.release(0x3000));
        assert!(t.release(0x3000));
        t.share_new(0x7000, 3); // born shared by 3
        assert!(!t.release(0x7000));
        assert!(!t.release(0x7000));
        assert!(t.release(0x7000));
    }

    /// The semaphore registry's consume-on-wake discipline (CR-03's
    /// pin: a woken waiter that loses the unit race re-registers
    /// before re-blocking — the consume is what makes that safe).
    #[test]
    fn sem_registry_consumes_and_reregisters() {
        let pa = 0x9100_0000;
        sem_register_waiter(pa, 11);
        sem_register_waiter(pa, 11); // idempotent
        sem_register_waiter(pa, 22);
        sem_wake(pa, 1);
        {
            let node = LockNode::new();
            let w = SEM_WAITERS.lock(&node);
            let got: Vec<u64> = w.iter().filter(|e| e.0 == pa).map(|e| e.1).collect();
            assert_eq!(got, vec![22], "the first-registered waiter is consumed");
        }
        sem_wake(pa, 5); // drains
        sem_deregister_waiter(pa, 11); // no-op (already consumed)
        let other = 0x9500_0000;
        sem_register_waiter(other, 42);
        sem_wake(pa, 8); // never touches other words
        {
            let node = LockNode::new();
            let w = SEM_WAITERS.lock(&node);
            assert!(w.contains(&(other, 42)));
        }
        sem_deregister_waiter(other, 42);
    }

    /// new_user_proc through the host seam: the PML4, the kstack
    /// window mapping, and the cons preopen all construct (the
    /// mapping machinery's host-exercised proof — the "PAs" are host
    /// pages and the walk genuinely writes PTEs).
    #[test]
    fn new_user_proc_constructs_the_address_space() {
        let before_pid = {
            let node = LockNode::new();
            PROC_TABLE.lock(&node).next_pid
        };
        let guard = new_user_proc("hosttest").expect("construction succeeds");
        assert!(guard.pid >= before_pid);
        assert_eq!(guard.kstack_top, KSTACK_VA + KSTACK_PAGES * PAGE_SIZE_4K);
        assert!(guard.pml4_pa != 0);
        assert!(guard.kstack_pas.iter().all(|&pa| pa != 0));
        // The kstack window PML4 walk: PML4[400] present, and the leaf
        // for the TOP kstack page present (kernel-only).
        let idx400 = table_entry(guard.pml4_pa, KSTACK_PML4_INDEX);
        assert_eq!(idx400 & PTE_P, PTE_P);
        assert_eq!(idx400 & PTE_US, 0, "the kstack window is kernel-only");
        // fd 0/1/2 preopened as devcons chans.
        let node = LockNode::new();
        let g = guard.fd_group.lock(&node);
        assert!(g.fds[0].is_some() && g.fds[1].is_some() && g.fds[2].is_some());
        assert!(g.fds[3].is_none());
    }

    /// map_user_page + pml4_unmap_user round-trip through the seam: a
    /// mapped user leaf reads back with the segment's flags, the
    /// walk's created intermediates are recorded, and the unmap clears
    /// (returning the was-present verdict).
    #[test]
    fn user_page_map_unmap_round_trip() {
        let mut guard = new_user_proc("maptest").expect("construction succeeds");
        let va = 0x0040_0000usize; // user half, PML4[0] region
        let pa = guard.map_user_page(va, SegKind::Data).expect("map succeeds");
        assert_eq!(guard.mapping_log.len(), 1);
        // Walk the four levels by the book (PML4 -> PDPT -> PD -> PT)
        // and read the leaf back: present, user, NX (the Data kind),
        // pointing at the mapped page.
        let pml4e = table_entry(guard.pml4_pa, (va >> 39) & 0x1ff);
        assert_eq!(pml4e & PTE_P, PTE_P);
        let pdpte = table_entry(pml4e & 0x000f_ffff_ffff_f000, (va >> 30) & 0x1ff);
        assert_eq!(pdpte & PTE_P, PTE_P);
        let pde = table_entry(pdpte & 0x000f_ffff_ffff_f000, (va >> 21) & 0x1ff);
        assert_eq!(pde & PTE_P, PTE_P);
        let pte = table_entry(pde & 0x000f_ffff_ffff_f000, (va >> 12) & 0x1ff);
        assert_eq!(pte & PTE_P, PTE_P);
        assert_eq!(pte & PTE_US, PTE_US);
        assert_eq!(pte & PTE_NX, PTE_NX);
        assert_eq!(pte & 0x000f_ffff_ffff_f000, pa & 0x000f_ffff_ffff_f000);
        // The walk created intermediates and recorded them.
        assert!(!guard.mapping_tables.is_empty());
        // Content writes through the alias hit the mapped page (the
        // alias IS the page).
        // SAFETY: pa is an owned seam page; 8 bytes inside it.
        unsafe { phys_ptr_mut(pa).cast::<u64>().write(0x1234) };
        // Unmap: was-present true; the leaf clears; a second unmap
        // reports false.
        assert!(pml4_unmap_user(guard.pml4_pa, va));
        assert!(!pml4_unmap_user(guard.pml4_pa, va));
        // Free the mapping like free_user_mappings would.
        guard.mapping_log.clear();
        free_phys_page(pa);
        for t in core::mem::take(&mut guard.mapping_tables) {
            free_phys_page(t);
        }
    }
}
