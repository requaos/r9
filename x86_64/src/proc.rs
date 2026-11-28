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
//! Locking discipline (SMP-02/SMP-03, D-22 — the aarch64 05-10 shape
//! re-keyed):
//! - PROC_TABLE is an `IrqSpinLock` at `LockRank::ProcTable` (50), taken
//!   only briefly in process/boot/trap context — NEVER across the
//!   fault/page-in paths or a pagealloc call that could recurse, and
//!   never across a swtch (the resumed proc may lock it).
//! - SHARED_PAGES / RFMEM_FAMILIES / SEM_WAITERS are `IrqSpinLock`s at
//!   `LockRank::VmShared` (30) — IRQ-shared mutable state, never nested
//!   inside each other.
//! - Trap-context code reads the current proc through `current_proc()`'s
//!   raw pointer — PER CPU (the CPU's own `Mach.current_proc` slot,
//!   `%gs:0`-indexed): written only by the owning CPU's exec/switch path,
//!   so a load on this CPU is stable for the reader's lifetime.
//! - The live-frame record (`Mach.cur_frame`) is written only under the
//!   PROC_TABLE lock or in the owning CPU's IRQ-masked trap context; read
//!   by the scheduler's resume arms and the note-delivery boundary.
//! - The lost-wakeup contract (Pitfall 4, D-22/D-25): `wake_pending` on
//!   each Proc is published (Release) INSIDE the waker's PROC_TABLE
//!   critical section — together with the condition mutation and the
//!   Blocked→Ready+enqueue — and consumed (`swap(false, AcqRel)`) under
//!   the same lock inside `sched()` BEFORE the Blocked transition (a
//!   consumed flag returns WITHOUT sleeping).  Subsystem-owned wait
//!   conditions clear it via `port::smp::prepare_block()` (->
//!   `prepare_block_current`) before registration.
//! - Ready ⟺ enqueued (SMP-11, D-23): every Ready-producing transition
//!   routes through `enqueue_ready` exactly once (a RUN_QUEUES push +
//!   the reschedule-IPI report), and `pick_next` pops via
//!   `RUN_QUEUES.pop_or_steal` (per-CPU FIFO, victim steal via try_lock
//!   — never two rank-60 queue locks held).  A stale entry (a proc no
//!   longer Ready) is skipped, never fatal.
//! - Kernel-stack ownership (05-24, the aarch64 339036f/665c94e port —
//!   the HVF `-smp 2` fork-sweep corruption's fix re-keyed):
//!   `Proc::on_cpu` marks a proc whose kstack (and, mid-syscall, its
//!   saved context) a CPU owns.  `pick_next` never starts an `on_cpu`
//!   proc, and the token is cleared only by the owning CPU's sched-loop
//!   `release_left_proc` — which runs on the Mach scheduler stack, after
//!   the CPU has left the proc's kstack.  A preempted proc is therefore
//!   NOT published Ready by `reschedule()` itself (which parks with
//!   swtch): `release_left_proc` marks it Ready + enqueues it.  Two CPUs
//!   must never run on one kstack — overlapping frames corrupt every
//!   stack-resident critical section (the console MCS queue's LockNodes
//!   were the aarch64 deadlock's witness); `pick_next`'s debug tripwire
//!   panics if another CPU's `cur_kstack` still names the candidate.
//!
//! Physical-memory access model: every "PA" the process machinery
//! handles is reached through `phys_ptr_mut` — the first gibibyte
//! through the kernel-half alias at `KZERO+pa` (the boot tables map it
//! there, and the kernel image lives in it), the RAM above through the
//! direct map at `DIRECT_BASE+pa` (directmap.rs, built at boot into
//! the same shared kernel half).  Both are in every page table.  There
//! is no Linux-style copy layer anywhere: user VAs are directly
//! dereferenceable in a proc's syscall context because its CR3 is
//! live, and its pages are writable by the kernel through the same
//! alias when its table is NOT the active one.

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use port::Result;
use port::dat::Chan;
use port::devenv::EnvGroup;
use port::fdgrp::FdGroup;
use port::mcslock::{IrqSpinLock, Lock, LockNode};
use port::mem::{PAGE_SIZE_4K, VirtRange};
// Not cfg(test)-gated: evict_user_frame names it in every build.
use port::mem::PhysAddr;
use port::paqfs::PaqDir;
use port::pagecache::PageKey;
use port::pgrp::Pgrp;
use port::println;
use port::rendez::RendezGroup;
use port::rfork::{RFCENVG, RFCFDG, RFCNAMEG, RFENVG, RFFDG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFREND};
use port::smp::LockRank;

use crate::dat::Ureg;
#[cfg(not(test))]
use crate::dat::Stack;
use crate::maplog::MappingLog;
use crate::{directmap, kstack};
use crate::swtch;

/// The ring-3 `#PF` demand pager (MEM-14/MEM-15/MEM-16).  Mounted here so
/// `crate::proc::fault::*` is reachable by `trap.rs`, `exec.rs` and
/// `usermem.rs`; it lives beside the process machinery it pages for.
#[path = "fault.rs"]
pub mod fault;

/// The higher-half base (kernel.ld) — the same value main.rs links by.
/// The first gibibyte of RAM is aliased at `KZERO+pa` in the SHARED
/// kernel half (PML4[511] → the boot KZERO table), so kernel-side
/// content writes work identically under the boot PML4 and under any
/// per-process PML4.
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
/// leaf).  The kernel image, the page tables and the kernel stacks are
/// all placed inside it; the RAM above is reached through the direct
/// map (directmap.rs).  `phys_ptr_mut` below is the one place the two
/// are told apart.
pub(crate) const KZERO_MAP_LIMIT: u64 = 1 << 30;

/// The longest process name kept: the last element of the executed
/// file, which is a path element, so it is bounded like one
/// (`port::namec::PATHMAX`).  Until 2026-11-28 the name was Plan 9's
/// fixed KNAMELEN field, 27 characters, and `ps`, `#p/N/status` and the
/// kernel's own reports cut a longer file name short.
pub const KNAMELEN: usize = port::namec::PATHMAX;
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
#[cfg(not(dev_churn))]
pub const MAX_PROCS: usize = 512;
/// The churn profile's fork budget (17-02, cfg-gated so every other
/// build keeps the measured-201 shape above): WINDOWS #44's repro
/// shape is ~1400 fork+exec iterations plus CR-03's RFMEM contenders
/// (~40 generations x 5 procs) — a boot-session budget above the
/// default.  The static table grows ~1 MiB of .bss under the gate (a
/// Proc is ~600 bytes); the 1024M guest carries it trivially.
#[cfg(dev_churn)]
pub const MAX_PROCS: usize = 2048;
/// Open fd slots per proc (init's closefds closes 3..29 — the table
/// must hold fds through 29 without making that fatal; pipe+dup flows
/// hold several at once).
pub const MAX_FDS: usize = 32;
/// Per-process kernel stack: 32 KiB (8 pages), the lowest of them the
/// unmapped guard page below the stack (kstack.rs).  The 9k donor's
/// KSTACK is 16 KiB (mem.h: `#define KSTACK (16*1024)`) and so was this,
/// but the in-kernel 9P2000.aead loopback (`#M`) is a coroutine server:
/// a proc's syscall frames stay live while the server end runs on the
/// same stack, and that path runs deeper than 16 KiB.  With 16 KiB
/// slots `ls '#M'` ran off the bottom of its slot into the neighbouring
/// one, and the root file server resumed onto the overflow (measured
/// 2026-11-28 at two CPUs); the aarch64 port took the same path to the
/// same answer (e5ae868).  One contiguous slot of the boot-reserved
/// arena (kstack.rs), reached at the KZERO alias that every page table
/// carries — so every CPU can write every other CPU's stack, which the
/// MCS locks' stack-resident nodes require.
pub(crate) const KSTACK_PAGES: usize = 8;
/// The deferred moribund reap record (05-24, the aarch64 05-22
/// moribund_reap port): an exiting proc's PML4 PA and the base PA of
/// its kernel-stack slot, parked on the exiting CPU's
/// `Mach::moribund_reap` slot from `exit_tail` until that CPU's sched
/// loop frees them after the moribund hand-off swtch (reap_moribund).
pub(crate) type MoribundReap = (u64, u64);
/// Queued-notes capacity (the note delivery path fills it).
pub const NOTE_QUEUE_CAP: usize = 8;
/// The exec-recorded argv's byte bound (the /proc/N/args content —
/// ps reads 256 bytes max, ps.c:63).  NUL-joined per arg.
pub const ARGV_CAP: usize = 256;

// --- x86_64 PTE bits (the aarch64 vm::Entry kinds re-keyed) ---
const PTE_P: u64 = 1 << 0;
const PTE_RW: u64 = 1 << 1;
const PTE_US: u64 = 1 << 2;
/// Hardware dirty bit: the CPU sets it on the first store to a present,
/// writable user page (MEM-15, D-15).
pub(crate) const PTE_D: u64 = 1 << 6;
const PTE_NX: u64 = 1 << 63;
/// A user leaf entry: present, user-accessible, READ-ONLY, EXECUTABLE
/// (MEM-15/D-15's W^X text: no `PTE_RW`, so a ring-3 store faults and the
/// pager kills the process).
pub(crate) const PTE_USER_TEXT: u64 = PTE_P | PTE_US;
/// A user leaf entry: present, r/w, user-accessible, NO-EXECUTE (the
/// aarch64 `rw_user_data` — data/bss/stack/tos).
pub(crate) const PTE_USER_DATA: u64 = PTE_P | PTE_RW | PTE_US | PTE_NX;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// The `palloc.hash` key for a shared user-text page (MEM-16).  Every
/// process maps a binary at the same `TEXT_MAP_BASE`, so the VA-derived
/// index is stable across them and the image id makes the frame shared.
pub(crate) fn text_page_key(image_id: u32, va: usize) -> PageKey {
    PageKey::new(image_id, (va >> 12) as u32)
}

/// The `palloc.hash` key for a proc-private untouched initialised-data
/// page.  Salted with the pid and exec generation so two processes (or two
/// execs of one process) never alias each other's cached frames.
pub(crate) fn data_page_key(image_id: u32, pid: u64, exec_gen: u32, va: usize) -> PageKey {
    let salt = (pid as u32).rotate_left(7) ^ exec_gen.wrapping_mul(0x9E37_79B1);
    PageKey::new(image_id ^ salt, (va >> 12) as u32)
}

/// The cache key for a mapping-log record, or `None` for anonymous kinds
/// (bss/stack/tos are never cached or evicted).
pub(crate) fn mapping_page_key(
    image_id: u32,
    pid: u64,
    exec_gen: u32,
    rec: &MappingRecord,
) -> Option<PageKey> {
    match rec.kind {
        SegKind::Text => Some(text_page_key(image_id, rec.va)),
        SegKind::Data => Some(data_page_key(image_id, pid, exec_gen, rec.va)),
        SegKind::Bss | SegKind::Stack | SegKind::Tos => None,
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
    /// A stable identity for the executable image (MEM-16): the paqfs qid
    /// mixed with the image base on devroot, a content hash on devmnt.  Two
    /// processes running the same binary share text frames under it.
    pub image_id: u32,
}

impl ByteSource {
    /// Build a byte source and derive its image identity.
    pub fn new(backing: ByteBacking, data_file_base: u64) -> Self {
        let image_id = derive_image_id(&backing);
        Self { backing, data_file_base, image_id }
    }

    /// The pager's read path — delegates to the backing.
    pub fn read_at(&self, file_off: u64, dst: &mut [u8]) -> Result<usize> {
        self.backing.read_at(file_off, dst)
    }
}

/// FNV-1a over the whole file (devmnt binaries: their staged buffer is the
/// only stable identity available).
fn content_image_id(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &byte in bytes {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// A non-zero image identity for a backing.
fn derive_image_id(backing: &ByteBacking) -> u32 {
    let id = match backing {
        ByteBacking::Paq { dir, image } => dir.qid ^ (image.as_ptr() as usize as u32).rotate_left(16),
        ByteBacking::Mnt { file } => content_image_id(file),
    };
    if id == 0 { 1 } else { id }
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
// Kernel side: pagealloc + the kernel's map of RAM (the KZERO alias
// below the first gibibyte, the direct map above it).  Host-test side:
// real host pages whose ADDRESSES play the PA role, with phys_ptr_mut
// the identity — the 14-03 mock-proc doctrine (the mock's memory IS the
// address range), extended to the mapping machinery so the exec/rfork
// bodies are genuinely host-exercised, not deferred to metal.
// ---------------------------------------------------------------------------

/// The kernel alias of a physical page: the boot tables' shared KZERO
/// window below the first gibibyte, the direct map above it
/// (directmap.rs).  Panics loudly on a PA neither maps — silence here
/// would be a wild write.
#[cfg(not(test))]
#[track_caller]
pub(crate) fn phys_ptr_mut(pa: u64) -> *mut u8 {
    if pa < KZERO_MAP_LIMIT {
        return (KZERO + pa) as *mut u8;
    }
    assert!(directmap::covers(pa), "PA {pa:#x} is in neither the KZERO window nor the direct map");
    directmap::va(pa) as *mut u8
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
// the way down.  The kernel half (slots 256..=511) is never walked —
// those entries are shared at construction.
// ---------------------------------------------------------------------------

/// Power-of-two page round-up — `usize::next_multiple_of` lowers
/// through u128 arithmetic (`__multi3`, soft-float here); the page
/// size is a power of two, so the mask form is exact.
fn round_up_page(x: usize) -> usize {
    (x + PAGE_SIZE_4K - 1) & !(PAGE_SIZE_4K - 1)
}

/// Read one entry of the page table at `table_pa`.
pub(crate) fn table_entry(table_pa: u64, idx: usize) -> u64 {
    // SAFETY: table_pa names a page this module allocated (the seam)
    // and 0-initialized; idx < 512 by the callers' masks.
    unsafe { phys_ptr_mut(table_pa).cast::<u64>().add(idx).read() }
}

/// Write one entry of the page table at `table_pa`.
pub(crate) fn set_table_entry(table_pa: u64, idx: usize, val: u64) {
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
    for level in 0..3 {
        let shift = 39 - 9 * level;
        let idx = (va >> shift) & 0x1ff;
        let next = {
            let e = table_entry(table_pa, idx);
            if e & PTE_P != 0 {
                e & 0x000f_ffff_ffff_f000
            } else {
                let np = match alloc_phys_page() {
                    Ok(np) => np,
                    Err(msg) => {
                        // A walk failure frees its own intermediates
                        // (17-01, the WR-03 OOM-release family — this
                        // doc's "leaving no leaks" contract, now true):
                        // each created page is unlinked from the entry
                        // this call wrote into its parent and freed,
                        // innermost first.  No invlpg: the VA was
                        // unmapped before the call, so no CPU's TLB can
                        // hold a translation through these just-written
                        // entries (a translation for an invalid entry is
                        // never installed, on any CPU).
                        for l in (0..level).rev() {
                            let pidx = (va >> (39 - 9 * l)) & 0x1ff;
                            let parent = if l == 0 { pml4_pa } else { created[l - 1].unwrap() };
                            set_table_entry(parent, pidx, 0);
                            free_phys_page(created[l].unwrap());
                        }
                        return Err(msg);
                    }
                };
                created[level] = Some(np);
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

/// The PML4 slots of the user half (the canonical low half, VAs below
/// 0x0000_8000_0000_0000).  Every table under them was created by
/// pml4_map_user and recorded in the owning proc's mapping_tables.
const USER_PML4_SLOTS: usize = 256;

/// Clear every user-half entry of the root at `pml4_pa` (the
/// kernel-half slots stay: the exit tail still runs on them), then
/// flush what may still name the tables below.  The per-leaf invlpg's
/// came first, and a walk after them can re-cache an upper-level entry
/// in the paging-structure caches; a CR3 reload drops every non-global
/// translation, and user mappings are never global.  Only the ACTIVE
/// root can hold such entries (no PCIDs: every CR3 switch flushes).
fn unlink_user_half(pml4_pa: u64) {
    for idx in 0..USER_PML4_SLOTS {
        set_table_entry(pml4_pa, idx, 0);
    }
    let cr3 = read_cr3();
    if cr3 & 0x000f_ffff_ffff_f000 == pml4_pa {
        switch_cr3(cr3);
    }
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

/// The live evictor's result for one candidate frame.
pub(crate) enum EvictOutcome {
    /// Every mapping was cleared; the caller shoots the TLB down and frees.
    Dropped(Option<usize>),
    /// A concurrent write set `PTE_D`; the PTE was restored, the frame
    /// stays mapped and is marked dirty (no shootdown, never freed).
    DirtyRestored,
    /// The frame could not be evicted safely (multiple owners / no leaf).
    Skipped,
}

/// The 8-byte leaf PTE slot for `va`, or `None` when a level is absent.
fn pml4_leaf_slot(pml4_pa: u64, va: usize) -> Option<*mut u64> {
    let mut table_pa = pml4_pa;
    for level in 0..3 {
        let shift = 39 - 9 * level;
        let idx = (va >> shift) & 0x1ff;
        let e = table_entry(table_pa, idx);
        if e & PTE_P == 0 {
            return None;
        }
        table_pa = e & 0x000f_ffff_ffff_f000;
    }
    let idx = (va >> 12) & 0x1ff;
    // SAFETY: `table_pa` names a page-allocated page table (the pml4 walk
    // allocated every level) and `idx < 512`; the returned slot is the
    // 8-byte-aligned leaf entry inside that page.
    Some(unsafe { phys_ptr_mut(table_pa).cast::<u64>().add(idx) })
}

/// Clear the leaf PTE at `slot` atomically (`swap(0, AcqRel)`) and return
/// the old entry (MEM-15, T-05-34).  The caller inspects `PTE_D` before
/// releasing `PROC_TABLE`.
fn pte_swap_zero(slot: *mut u64) -> u64 {
    // SAFETY: `slot` is an 8-byte-aligned leaf entry in a live page table;
    // AtomicU64's representation is the PTE word and the table is only
    // edited under PROC_TABLE (this path) or by the owning CPU.
    unsafe { (&*(slot as *const AtomicU64)).swap(0, Ordering::AcqRel) }
}

/// Restore a previously swapped leaf entry (the dirty-race abort).
fn pte_restore(slot: *mut u64, old: u64) {
    // SAFETY: pte_swap_zero's contract; the restore publishes the exact
    // entry the hardware had before the swap.
    unsafe { (&*(slot as *const AtomicU64)).store(old, Ordering::Release) };
}

/// Unmap every mapping of `pa` from every proc's user table (MEM-15/D-14).
///
/// The caller holds `PAGE_CACHE` (Rank 20) through `with_lock`, so a
/// concurrent page-in on the other CPU serializes behind this swap; the
/// candidate's key was already removed from the hash by
/// `select_eviction_victim`.
///
/// For `PageKind::UntouchedData` the leaf is cleared with
/// `AtomicU64::swap(0, AcqRel)` and the old entry inspected for `PTE_D`
/// while `PROC_TABLE` (Rank 50) is held: a concurrent store that the CPU
/// recorded as dirty is RESTORED immediately and the frame is never freed
/// (Pitfall 3 / T-05-34).  `PageKind::SharedText` pages are read-only, so
/// they can never be dirty.  The caller shoots the TLB down before the
/// frame returns to `PAGE_ALLOC`.
pub(crate) fn evict_user_frame(pa: PhysAddr, kind: port::pagecache::PageKind) -> EvictOutcome {
    let page = pa.addr();
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);

    // A private untouched-data frame must have exactly one owner; a shared
    // RFMEM page must have been marked dirty and is never a candidate.
    if kind == port::pagecache::PageKind::UntouchedData {
        let owners: usize = table
            .procs
            .iter()
            .flatten()
            .map(|p| p.mapping_log.iter().filter(|r| r.phys as u64 == page).count())
            .sum();
        if owners != 1 {
            return EvictOutcome::Skipped;
        }
    }

    // Phase 1: atomically clear every matching leaf, collecting the old
    // entries so a dirty abort can restore them all.
    let mut cleared: Vec<(*mut u64, u64, usize)> = Vec::new();
    for proc in table.procs.iter().flatten() {
        for rec in proc.mapping_log.iter().filter(|r| r.phys as u64 == page) {
            if let Some(slot) = pml4_leaf_slot(proc.pml4_pa, rec.va) {
                let old = pte_swap_zero(slot);
                cleared.push((slot, old, rec.va));
            }
        }
    }
    if cleared.is_empty() {
        return EvictOutcome::Skipped;
    }
    if cleared.iter().any(|&(_, old, _)| old & PTE_D != 0) {
        for &(slot, old, _) in &cleared {
            pte_restore(slot, old);
        }
        return EvictOutcome::DirtyRestored;
    }

    // Phase 2: no CPU could have dirtied the frame — drop the mapping
    // records and invalidate this CPU's TLB for each cleared VA.
    for proc in table.procs.iter_mut().flatten() {
        proc.mapping_log.retain(|rec| rec.phys as u64 != page);
    }
    for &(_, _, va) in &cleared {
        invlpg(va);
    }
    EvictOutcome::Dropped(cleared.first().map(|&(_, _, va)| va))
}

/// Live evictor: drop up to `target` unpinned re-readable pages (MEM-15,
/// MEM-14, D-14/D-17/D-21).  Runs with `PAGE_ALLOC` released (the caller is
/// `PageAlloc::alloc_clean`), so the lock order is
/// `PAGE_CACHE (20) -> PROC_TABLE (50)` -> release -> `request_shootdown`
/// (LAPIC 0xF0) -> `PAGE_ALLOC (70)`.  Pinned frames are skipped and counted
/// by `PageCache::select_eviction_victim` (`pinned_skipped`).
pub(crate) fn evict_re_readable_pages(target: usize) -> usize {
    port::palloc::record_evict_call();
    let Some(cache) = crate::pagealloc::page_cache() else { return 0 };
    let mut freed = 0;
    while freed < target {
        let Some(candidate) = cache.select_eviction_victim(crate::pagealloc::is_pinned) else {
            break;
        };
        // Re-check after the candidate's key left the hash: a fault on the
        // other CPU may have pinned it in the window.
        if crate::pagealloc::is_pinned(candidate.pa) {
            port::palloc::record_pinned_skipped();
            continue;
        }
        match cache.with_lock(|| evict_user_frame(candidate.pa, candidate.kind)) {
            EvictOutcome::Dropped(va) => {
                // Text is RO and never dirty; data passes the cleared VA so
                // the remote flush targets it.  Text flushes everything.
                let target_va = match candidate.kind {
                    port::pagecache::PageKind::SharedText => None,
                    port::pagecache::PageKind::UntouchedData => va,
                };
                port::smp::request_shootdown(target_va);
                match crate::pagealloc::deallocate_physpage(candidate.pa) {
                    Ok(()) => port::palloc::record_evict_reclaimed(),
                    Err(_) => port::palloc::record_evict_free_failure(),
                }
                freed += 1;
            }
            EvictOutcome::DirtyRestored => {
                // The page was written concurrently: keep it mapped and
                // mark it permanently non-droppable (no shootdown).
                let _ = cache.mark_data_dirty(candidate.pa);
            }
            EvictOutcome::Skipped => {}
        }
    }
    freed
}

/// Copy the boot PML4's kernel half — every entry from slot
/// [`USER_PML4_SLOTS`] up: the direct map of RAM (slots 256..=509,
/// directmap.rs), 510 the KCPUZERO Mach window the gs-relative entry
/// asm needs under EVERY table, 511 KZERO — into a new per-process
/// PML4 at `pml4_pa`.  Read from the linked boot table itself (l.S's
/// bootpml4), never restated: whatever the kernel maps at boot is live
/// under every CR3.
#[cfg(not(test))]
fn copy_kernel_half(pml4_pa: u64) {
    unsafe extern "C" {
        static bootpml4: u8;
    }
    let base = ptr::addr_of!(bootpml4) as *const u64;
    // SAFETY: the boot PML4 is linked, mapped (its own entry 511 maps
    // it — KZERO covers the whole image) and read-only to us; the
    // kernel half is 256 qwords inside the 512-entry page.
    unsafe {
        for slot in USER_PML4_SLOTS..512 {
            let entry = base.add(slot).read();
            if entry != 0 {
                set_table_entry(pml4_pa, slot, entry);
            }
        }
    }
}

/// The boot PML4's PHYSICAL address (l.S's `bootpml4` link symbol, VA −
/// KZERO) — the AP trampoline's CR3 (SMP-01): the boot tables
/// identity-map the first 4 GiB, so the SIPI trampoline keeps executing
/// at PA == VA through the mode switches.  NEVER the current CR3 at the
/// call site (a proc's table carries no identity map).
#[cfg(not(test))]
pub(crate) fn boot_pml4_pa() -> u64 {
    unsafe extern "C" {
        static bootpml4: u8;
    }
    // SAFETY: kernel.ld links the symbol inside the image (KZERO-based
    // VA); only its ADDRESS is read (the link-sym class, kmem.rs's
    // convention), never its contents.
    ptr::addr_of!(bootpml4) as u64 - KZERO
}

/// Leave the current proc's page table before the swtch into the
/// scheduler context: the scheduler (the Mach stack, the proc table, the
/// run queues, the reaper, the idle loop) runs on the boot table and on
/// nothing else.  A proc's table is in CR3 only while that proc is
/// current — from pick_next's install to this call.
///
/// Until 2026-11-27 a CPU that had nothing to pick kept the last proc's
/// table in CR3, and the sched loop ran each iteration on the table of
/// the proc it had just left.  On one CPU that held by luck: nothing
/// zeroed a freed page before the next pick, and the TLB carried what
/// the loop touched.  With two it is a triple fault.  The page goes back
/// to the allocator when the proc exits — through this CPU's reap, or
/// through a sibling that stole the preempted proc — and is zeroed for
/// reuse at once; a CPU still translating through it then faults on the
/// first address its TLB does not hold, and with the kernel half gone
/// the fault handler faults too (measured: the -smp 2 churn gate reset
/// the machine 600 cats in, in 14 runs of 25).
#[cfg(not(test))]
fn leave_proc_table() {
    let boot = boot_pml4_pa();
    if read_cr3() != boot {
        switch_cr3(boot);
    }
}

/// The host-test stand-in: no page tables on the host.
#[cfg(test)]
fn leave_proc_table() {}

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

/// Re-point the TSS rsp0 (the kstack switch mechanism) — on the CALLING
/// CPU's own Mach (`%gs:0`, SMP-02): each CPU's TSS names its own Mach,
/// so a context switch on CPU 1 re-points CPU 1's rsp0, never CPU 0's.
#[cfg(not(test))]
pub(crate) fn set_tss_rsp0(top: u64) {
    // SAFETY: mach_ptr() names the calling CPU's live Mach (dat.rs's
    // %gs:0 contract); tss.rsp0 is written only by the owning CPU's
    // scheduler/boot context (the locked-step discipline), so the &mut
    // through the per-CPU pointer is exclusive by construction.
    unsafe {
        (&mut *(crate::dat::mach_ptr() as *mut crate::dat::Mach)).tss.set_rsp0_raw(top);
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
/// Locking: `IrqSpinLock` at `LockRank::VmShared` (30) — SMP-03 (D-22):
/// the refcount table is mutated from syscall context on ANY CPU and
/// read at exit/exec on ANY CPU; IRQ-safe because a timer tick can
/// preempt into PROC_TABLE work on the same CPU.
static SHARED_PAGES: IrqSpinLock<SharedPages> =
    IrqSpinLock::new("rfmem", LockRank::VmShared, SharedPages::new());

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
/// SMP-03 (D-22): `IrqSpinLock` at `LockRank::VmShared` (30) — the
/// family registry is mutated/read from syscall context on any CPU.
static RFMEM_FAMILIES: IrqSpinLock<Vec<(u64, u64)>> =
    IrqSpinLock::new("rfmemfam", LockRank::VmShared, Vec::new());

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
            // 17-02 (the churn gate's contention-leg finding): a family
            // reduced to its last member still brks — the grown pages
            // map into it exactly like the private arm.  The previous
            // shape moved only segs.bss.end, leaving the growth
            // UNMAPPED: the last RFMEM sibling's exit leaves the
            // survivor a lone member (rfmem_leave dissolves only the
            // leaver's entry), and the survivor's next brk — its
            // malloc pool's first growth, precisely the churn sem
            // parent's post-reap print — fault-killed it at the pool
            // header (33/39 generations died at exactly this page).
            let page = PAGE_SIZE_4K;
            let mut va = round_up_page(old_end);
            let limit = round_up_page(new_end);
            while va < limit {
                let pa = alloc_phys_page().map_err(|_| "brk: out of memory")?;
                // SAFETY: table-resident pointer (rfmem_family_members;
                // the PROC_TABLE slots are never freed this era, so the
                // target is always a live Proc).  The freshly-mapped VA
                // was invalid in the member's table before the call, so
                // no CPU's TLB can hold a stale translation for it (a
                // translation for an invalid entry is never installed);
                // a member running on a sibling CPU faults the grown
                // range in on demand or reads the old break — both valid
                // break states.
                unsafe {
                    let p = &mut *m;
                    match pml4_map_user(p.pml4_pa, va, pa, SegKind::Bss.pte()) {
                        Ok(created) => {
                            for c in created.into_iter().flatten() {
                                p.mapping_tables.push(c);
                            }
                        }
                        Err(_) => {
                            // The walk freed its own intermediates; the
                            // fresh leaf was never recorded — free it
                            // (the record-or-unwind hygiene).
                            free_phys_page(pa);
                            return Err("brk: growth page map failed");
                        }
                    }
                    if let Err(msg) = p.mapping_log.insert(va, pa as usize, SegKind::Bss) {
                        // Mapped but unrecorded: undo the map and free
                        // the page (the record-or-unwind hygiene).
                        pml4_unmap_user(p.pml4_pa, va);
                        free_phys_page(pa);
                        return Err(msg);
                    }
                }
                va += page;
            }
            // SAFETY: as above.
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
            // SAFETY: table-resident pointer (the PROC_TABLE slots are
            // never freed this era).  pml4_map_user edits the member's
            // table (ACTIVE for the caller, INACTIVE for its siblings —
            // the kernel-half alias makes both identical operations);
            // the VA was unmapped, so no CPU's TLB can hold a stale
            // translation for it (a translation for an invalid entry is
            // never installed, on any CPU).
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
                if p.mapping_log.insert(va, pa as usize, SegKind::Bss).is_err() {
                    // Mapped but unrecorded: undo this member's map;
                    // the unwind below handles the members before it.
                    pml4_unmap_user(p.pml4_pa, va);
                    failed = true;
                    break;
                }
                mapped.push(m);
            }
        }
        if failed {
            // This page never published a refcount: unmap it from the
            // members that took it, drop their records, free the page.
            for &m in &mapped {
                // SAFETY: `m` is a PROC_TABLE-resident member pointer
                // from the `mapped` list this growth just built (slots
                // are never freed this era), and the unmap targets the
                // VA this loop's own pml4_map_user installed (with its
                // per-write TLB invalidate); the swap_remove drops
                // exactly that mapping's log record.  The member is not
                // running on this CPU (the caller's syscall holds it);
                // a sibling-CPU member's TLB cannot hold the VA (it was
                // unmapped at entry).
                unsafe {
                    let p = &mut *m;
                    pml4_unmap_user(p.pml4_pa, va);
                    p.mapping_log.remove(va);
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
                // SAFETY: `m` is a PROC_TABLE-resident member pointer
                // from `members` (rfmem_family_members; slots are never
                // freed this era).  `done_va` was mapped into this
                // member by this function's own pml4_map_user
                // (per-write TLB invalidate inside the unmap), so the
                // unmap and the swap_remove target exactly that record.
                unsafe {
                    let p = &mut *m;
                    pml4_unmap_user(p.pml4_pa, done_va);
                    p.mapping_log.remove(done_va);
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

/// (word PA, waiter pid) pairs.  SMP-03 (D-22): `IrqSpinLock` at
/// `LockRank::VmShared` (30) — the registry is mutated/read from syscall
/// context on any CPU (the `sem_wake` waker and the `sys_semacquire`
/// waiter can be on different CPUs).
static SEM_WAITERS: IrqSpinLock<Vec<(u64, u64)>> =
    IrqSpinLock::new("sem", LockRank::VmShared, Vec::new());

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

/// sys_semrelease's wake half: ready up to `delta` waiters registered
/// on the word's PA (registration order).  Entries are consumed as
/// they are picked — the register→check→block ordering is
/// wakeup-loss-free (the prepare_block/wake_pending contract, SMP-03:
/// a releaser on another CPU either publishes wake_pending — consumed
/// by the waiter's sched() under PROC_TABLE — or sees the waiter
/// Blocked and readies + enqueues it).
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
            wake_locked(p);
        }
    }
}

/// A user process.  Raw-pointer fields (`frame`) name memory this proc
/// owns; the address space is its own PML4 (`pml4_pa` — the CR3 value),
/// with the kernel higher half shared by construction.
pub struct Proc {
    pub pid: u64,
    pub name: Vec<u8>,
    pub state: ProcState,
    /// Top of this proc's kernel stack — the KZERO alias of its
    /// arena slot's end (kstack.rs), the same address under every page
    /// table; the TSS rsp0 names it while this proc is current (the one
    /// kernel-stack mechanism both ring-3 entry paths share).
    pub kstack_top: usize,
    /// The stack's four pages, lowest first: one contiguous slot of the
    /// kernel-stack arena (kstack.rs).  The slot returns to the arena
    /// at the deferred reap; `kstack_pas[0]` names it.
    pub kstack_pas: [u64; KSTACK_PAGES],
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
    pub mapping_log: MappingLog,
    /// The a.out-derived VA map; None until exec.
    pub segments: Option<Segments>,
    /// paqfs byte source for demand page-in; None until exec.
    pub text_source: Option<ByteSource>,
    /// Exec generation (MEM-16): bumped on every exec, mixed into the
    /// proc-private untouched-data cache key so a new image never aliases
    /// its predecessor's cached frames.
    pub exec_gen: u32,
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
    /// The user FPU/SSE/AVX state (the 832-byte image in the boot's
    /// selected format — MEM-18, D-20: XSAVE/XSAVEOPT with ymm where
    /// CPUID reports it, the FXSAVE legacy region elsewhere) while the
    /// proc does not own the registers; stale while it does (fpu.rs's
    /// LIVE_OWNER invariant).  Installed at every switch-in
    /// (activate_first, pick_next), parked at every switch-out (sched,
    /// reschedule's park) and by a kernel SIMD section; rfork seeds a
    /// child's from the parent's current state, exec loads the default.
    pub fpsave: crate::fpu::FpState,
    /// The interrupted code's FPU/SSE/AVX state while a note handler
    /// runs (the donor's notify-time fpsave): syscall.rs's
    /// deliver_notes records it (all 832 bytes), NOTED's NCONT/NRSTR put
    /// it back.  Meaningful only while note_ureg != 0.
    pub note_fpsave: crate::fpu::FpState,
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
    /// (donor `up->noteureg`): set by the note-delivery boundary, which
    /// also writes the previous value into the word just below the new
    /// saved Ureg (the donor's chain); NOTED NCONT/NRSTR pop back through
    /// that word, NSAVE keeps it (a later NRSTR returns through it).
    /// 0 = no saved Ureg.
    pub note_ureg: u64,
    /// The donor's `up->notified`: a note handler is running, so further
    /// notes stay queued.  Set at delivery; NOTED's NCONT/NRSTR/NSAVE
    /// clear it (NSAVE re-enables notes inside its trampoline).
    pub notified: bool,
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
    /// The lost-wakeup flag (SMP-03, D-22, Pitfall 4): a waker
    /// publishes `true` (Release) INSIDE its PROC_TABLE critical section
    /// (together with the condition mutation and any Blocked→Ready
    /// transition); `sched()` consumes it (`swap(false, AcqRel)`) under
    /// the same lock BEFORE marking Blocked, returning without sleeping
    /// when a wake landed in the register→drop-lock→block window.
    /// Subsystem-owned wait conditions clear it BEFORE registration via
    /// `port::smp::prepare_block()` → `prepare_block_current()`.
    pub wake_pending: AtomicBool,
    /// True while a CPU owns this proc's kernel stack (and, for a proc
    /// parked mid-syscall, its saved kernel context): set by the CPU
    /// that starts the proc (pick_next / activate_first), cleared ONLY
    /// by that CPU's sched-loop release step — which runs on the Mach
    /// scheduler stack, i.e. after the CPU has left the proc's kstack.
    ///
    /// The invariant `pick_next` enforces (05-24, the aarch64 339036f
    /// port): a proc is never started on a CPU while `on_cpu` is set, so
    /// two CPUs can never share one kernel stack.  The pre-port shape
    /// published a preempted proc Ready + enqueued while the preempting
    /// CPU still ran on its kstack (the publish sat between the
    /// PROC_TABLE-locked mark and the park's swtch), so a sibling CPU
    /// could pop the pid and swtch into a context the owning CPU had not
    /// even saved yet — overlapping kernel frames on one kstack, the
    /// class the aarch64 HVF `-smp 2` boot corruption proved live.  The
    /// sched() window (a wake readying a proc whose CPU has not yet
    /// reached its swtch) is covered by the same token.
    pub on_cpu: AtomicBool,
    /// sys_sleep's wake deadline in TICKS (0 = not sleeping).
    pub sleep_until: u64,
    /// sys_alarm's expiry deadline in TICKS (0 = disarmed).
    pub alarm_at: u64,
    /// The exec-recorded argv — NUL-joined, bounded at ARGV_CAP.
    pub argv: Vec<u8>,
    /// The recorded scheduling priority (devproc ctl's "pri N" — the
    /// honest degradation: nothing READS it but the status report).
    pub pri: u32,
    /// The donor's privatemem (CMprivate — factotum's anti-debug write,
    /// recorded, never yet enforced: the /proc mem files are a later
    /// era).
    pub privatemem: bool,
    /// The rendezvous group handle (port::rendez; 9k's Rgrp).
    pub rgrp: Arc<Lock<RendezGroup>>,
    /// The value a rendezvous peer left for us.
    pub rendval: u64,
    /// The RFMEM family id (None at construction, after exec/exit, and
    /// for every non-RFMEM fork child).
    pub rfmem_family: Option<u64>,
    /// The proc's user (donor `up->user`): seeded `glenda` at
    /// construction, fork-copied, served by #c/user through the devcons
    /// hooks, and MUTATED by devcap's capuse success path (the donor's
    /// `kstrdup(&up->user, to)` — 18-02's AUTH-03, the first r9 kernel
    /// uid mutation).  No trailing newline (the readstr serve rule).
    pub user: Vec<u8>,
}

impl Proc {
    fn new(pid: u64, name: &str, kstack_top: usize, kstack_pas: [u64; KSTACK_PAGES], pml4_pa: u64) -> Proc {
        let mut proc = Proc {
            pid,
            name: Vec::new(),
            state: ProcState::Ready,
            kstack_top,
            kstack_pas,
            pml4_pa,
            mapping_tables: Vec::new(),
            fd_group: Arc::new(Lock::new("fdgrp", FdGroup::new())),
            mapping_log: MappingLog::new(),
            segments: None,
            text_source: None,
            exec_gen: 0,
            frame: ptr::null_mut(),
            context: swtch::Context::new(),
            note_queue: VecDeque::with_capacity(NOTE_QUEUE_CAP),
            notify_handler: None,
            note_ureg: 0,
            notified: false,
            last_note: [0; ERRMAX],
            last_note_len: 0,
            blocked_in_kernel: false,
            fpsave: crate::fpu::FpState::DEFAULT,
            note_fpsave: crate::fpu::FpState::DEFAULT,
            parent: ptr::null_mut(),
            children_live: 0,
            nowait: false,
            wake_pending: AtomicBool::new(false),
            on_cpu: AtomicBool::new(false),
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
        let n = bytes.len().min(KNAMELEN);
        self.name.clear();
        self.name.extend_from_slice(&bytes[..n]);
    }

    /// The process's name as a string (a non-UTF-8 byte reads as "?").
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name).unwrap_or("?")
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

    /// The page-cache reverse-map slot for this proc (the low 16 bits of
    /// the pid — pids are monotonic and the proc table is bounded).
    pub fn rmap_slot(&self) -> u16 {
        self.pid as u16
    }

    /// The note-delivery path's stack window must be RESIDENT before the
    /// kernel writes it (the aarch64 `fault::ensure_stack_resident` shape).
    pub(crate) fn ensure_stack_resident(&mut self, start: usize, end: usize) {
        fault::ensure_stack_resident(self, start, end);
    }

    /// Map one 4 KiB user page (fresh, pre-zeroed) at `va` in this
    /// proc's table with the segment's leaf kind, record it, and
    /// return the page's PA (exec's install body consumer).
    pub(crate) fn map_user_page(&mut self, va: usize, kind: SegKind) -> Result<u64> {
        let pa = alloc_phys_page()?;
        let created = match pml4_map_user(self.pml4_pa, va, pa, kind.pte()) {
            Ok(created) => created,
            Err(msg) => {
                // The walk already freed its own intermediates; the
                // fresh leaf page never mapped — free it too or it
                // leaks (17-01, the WR-03 OOM-release family).
                free_phys_page(pa);
                return Err(msg);
            }
        };
        for c in created.into_iter().flatten() {
            self.mapping_tables.push(c);
        }
        if let Err(msg) = self.mapping_log.insert(va, pa as usize, kind) {
            // The page is mapped but unrecorded — undo both, or it leaks
            // (17-01, the WR-03 OOM-release family).
            pml4_unmap_user(self.pml4_pa, va);
            free_phys_page(pa);
            return Err(msg);
        }
        Ok(pa)
    }

    /// EXEC/EXITS: unmap and free every user mapping (the mapping_log
    /// walk), then drop the segment map and the demand-page byte
    /// source.  Each leaf is cleared BEFORE its phys page goes back to
    /// the allocator — a stale translation pointing at a recycled page
    /// would be silent corruption (pml4_unmap_user invalidates).
    ///
    /// MEM-16: a frame still owned by `palloc.hash` (shared text or a
    /// cached untouched data page) is NOT returned to the allocator — the
    /// cache may hand the same frame to another process, and the live
    /// evictor is the only path that frees it.  The proc's reverse-map
    /// entries are removed.
    pub fn free_user_mappings(&mut self) {
        // The log leaves the proc first (the old Vec was taken the same
        // way): the evictor on another CPU walks procs' logs under
        // PROC_TABLE, and it must find this one empty while its index
        // pages go back.  The taken log drops, and frees them, at the
        // end of the walk.
        let records = core::mem::take(&mut self.mapping_log);
        let slot = self.rmap_slot();
        let cache = crate::pagealloc::page_cache();
        let image_id = self.text_source.as_ref().map(|source| source.image_id);
        for rec in records {
            pml4_unmap_user(self.pml4_pa, rec.va);
            let cached = match (cache, image_id) {
                (Some(cache), Some(image_id)) => {
                    mapping_page_key(image_id, self.pid, self.exec_gen, &rec)
                        .and_then(|key| cache.lookup(key))
                        .is_some_and(|page| page.pa.addr() == rec.phys as u64)
                }
                _ => false,
            };
            // RFMEM: an RFMEM-shared page's leaf is cleared above, but
            // the PHYS page goes back only when this release is the
            // last sharer's; siblings keep mapping it until their own
            // exit/exec.
            if !cached && release_user_page(rec.phys as u64) {
                free_phys_page(rec.phys as u64);
            }
        }
        if let Some(cache) = cache {
            cache.remove_proc_rmaps(slot);
        }
        // The user-half intermediate tables die with the address space,
        // but the PML4 does not: EXEC maps the new image into this same
        // root.  Unlink them BEFORE they go back to the allocator, or
        // the new image's walk descends into pages the allocator hands
        // out again as text/data leaves (the image bytes then land in
        // live tables — r9 8958c9e made frees real; until then these
        // pages simply leaked).
        unlink_user_half(self.pml4_pa);
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

    /// Free the PML4 page and return the kernel-stack slot IN PLACE.
    /// Only the rfork mid-construction unwind calls this now: a
    /// never-run child's pages are nobody's live state, so the direct
    /// free is correct there.  The EXITS path uses
    /// `defer_root_kstack_reap` instead (05-24): the exit handler is
    /// still executing ON the kstack and CR3 still names the PML4 when
    /// it runs, and under a sibling CPU the old single-CPU invariant
    /// ("nothing allocates until the hand-off") is gone — a sibling's
    /// new proc could take the returned slot and run on it while this
    /// CPU still does (the aarch64 05-22 exit-mid-free corruption class).
    pub fn free_root_table_and_kstack(&mut self) {
        free_phys_page(self.pml4_pa);
        let slot = core::mem::take(&mut self.kstack_pas[0]);
        self.kstack_pas = [0; KSTACK_PAGES];
        // A zero PA is the host-test mock's "no stack" (nothing to return).
        if slot != 0 {
            kstack::free(slot);
        }
    }

    /// Record the PML4 PA and the kernel-stack slot on this CPU's
    /// deferred reap slot instead of freeing in place (05-24, the
    /// aarch64 05-22 `defer_root_kstack_reap` port): the exit path's
    /// "nothing allocates until the hand-off" invariant was single-CPU —
    /// with a sibling CPU allocating concurrently, a kstack slot
    /// returned at exit is handed to a new proc and written while THIS
    /// CPU still runs on it up to the hand-off swtch (and CR3 still
    /// names the PML4 until the next pick's switch_cr3).  The sched
    /// loop's `reap_moribund` runs after the hand-off, on the Mach
    /// scheduler stack.
    pub fn defer_root_kstack_reap(&mut self) {
        let slot = self.kstack_pas[0];
        self.kstack_pas = [0; KSTACK_PAGES];
        let entry: MoribundReap = (self.pml4_pa, slot);
        let slot = crate::dat::this_mach().moribund_reap.get();
        // SAFETY: per-CPU slot — written only on the owning CPU in
        // IRQ-masked syscall context (here) and taken in this CPU's
        // sched loop (reap_moribund), so no concurrent access is
        // possible.
        unsafe {
            debug_assert!(
                (*slot).is_none(),
                "proc: moribund reap slot occupied — the sched loop drains it between exits"
            );
            *slot = Some(entry);
        }
    }

    /// Seed a forkret-shaped 184-byte Ureg at the top of this proc's
    /// kstack, through the top page's kernel alias — which, for an arena
    /// slot, IS the frame's VA under every table (kstack.rs).  exec's
    /// install_image consumer reads it back at the returned VA.
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

/// Free this CPU's deferred moribund pages (05-24): runs from the sched
/// loop's iteration top, after the exiting CPU's hand-off swtch — this
/// CPU no longer runs on the dead kstack, and CR3 is the boot table
/// (`leave_proc_table` put it there before the swtch), so the root page
/// can go back at once.  (Until 2026-11-27 the loop ran on the dead PML4
/// until the next pick replaced CR3, on the single-CPU assumption that
/// nothing would recycle and zero the page first; see leave_proc_table.)
/// The stack slot
/// returns to the arena here and only here, for the same reason: a
/// sibling's new proc may take it the moment it is free.  No lock is
/// held here; free_phys_page takes the page allocator's own lock.
fn reap_moribund() {
    let slot = crate::dat::this_mach().moribund_reap.get();
    // SAFETY: per-CPU slot — taken only on the owning CPU's sched loop
    // (IRQ-masked); exit_tail's write is the owning CPU's too.
    let entry = unsafe { (*slot).take() };
    let Some((pml4_pa, kstack_pa)) = entry else { return };
    free_phys_page(pml4_pa);
    // A zero PA is the host-test mock's "no stack" (nothing to return).
    if kstack_pa != 0 {
        kstack::free(kstack_pa);
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

    fn ensure_user_resident(
        &mut self,
        start: usize,
        end: usize,
        writable: bool,
    ) -> core::result::Result<(), &'static str> {
        fault::ensure_user_resident(self, start, end, writable)
    }

    fn resident_phys(&self, page_va: usize) -> Option<u64> {
        self.mapping_log.get(page_va).map(|m| m.phys as u64)
    }
}

/// The process table — a small fixed array behind the mcslock idiom.
pub struct ProcTable {
    procs: [Option<Proc>; MAX_PROCS],
    next_pid: u64,
}

/// SMP-03 (D-22): `IrqSpinLock` at `LockRank::ProcTable` (50) — the
/// table is read/written from process, boot AND trap context (the tick's
/// `reschedule`/`tick_timeouts` run in IRQ context on any CPU), so the
/// lock is IRQ-safe and ranked.
pub static PROC_TABLE: IrqSpinLock<ProcTable> = IrqSpinLock::new(
    "proc",
    LockRank::ProcTable,
    ProcTable { procs: [const { None }; MAX_PROCS], next_pid: 1 },
);

/// The per-CPU work-stealing run queues (SMP-11, D-23, D-25):
/// `port::sched::RunQueues` holds one IrqSpinLock<VecDeque>-per-CPU at
/// `LockRank::RunQueue` (60 — taken only while PROC_TABLE (50) is held
/// or no lock is held, never the reverse nest), local FIFO pop, neighbor
/// steal via try_lock, and the idle-mask re-check that pairs with
/// `enqueue`'s idle-CPU report (which the wake paths turn into the
/// LAPIC 0xF1 reschedule IPI).  Ready ⟺ enqueued: every Ready-producing
/// transition enqueues exactly once (the module header).
static RUN_QUEUES: port::sched::RunQueues<64> = port::sched::RunQueues::new();

/// Enqueue `pid` onto the calling CPU's run queue (the waker's CPU —
/// no affinity tracking this era; idle CPUs steal, D-23) and send the
/// reschedule IPI when an idle CPU was found.  Safe with or without
/// PROC_TABLE held (RunQueue at 60 nests above ProcTable at 50); on the
/// single-CPU default hooks (before `smp::register_port_smp`) the IPI
/// is a no-op and `num_online` is 1.
pub(crate) fn enqueue_ready(pid: u64) {
    let num = port::smp::num_online_cpus();
    let target = port::smp::current_cpu_id();
    if let Some(idle) = RUN_QUEUES.enqueue(target, pid, num) {
        port::smp::send_resched_ipi(idle);
    }
}

/// The wake critical section under PROC_TABLE (SMP-03, D-22, Pitfall 4)
/// — shared by every pid-keyed wake path (sem_wake, pipe_wake,
/// rendez_wake, cons_input_wake, tick_timeouts, push_note,
/// post_wait_record): publish `wake_pending` (Release) FIRST, so a
/// sched() that already passed its registration sees the flag under
/// this same lock; then the Blocked → Ready transition enqueues the
/// pid (Ready ⟺ enqueued).  A wake of a not-yet-Blocked proc leaves
/// the flag for its sched() to consume — the 2-CPU lost-wakeup fix.
/// The wake_pending store and the state transition share one critical
/// section with sched()'s flag-consume and Blocked-set, so no
/// interleave can lose the wake (an unlocked store BEFORE the lock
/// would re-open the consumed-flag/queued-record race this pairing
/// closes — the 05-10 lock-paired placement, not the plan's shorthand).
fn wake_locked(p: &mut Proc) {
    p.wake_pending.store(true, Ordering::Release);
    if p.state == ProcState::Blocked {
        p.state = ProcState::Ready;
        enqueue_ready(p.pid);
    }
}

/// The arch half of `port::smp::prepare_block()` (SMP-03, D-22): clear
/// the current proc's `wake_pending` (Release) BEFORE the caller's pid
/// becomes wake-targetable (the port-side register-under-subsystem-lock
/// contract).  No-op when no proc is current (early boot).  Registered
/// on `SmpArch::prepare_block` by `x86_64/src/smp.rs`; the port-side
/// callers already invoke it through `port::smp::prepare_block()`.
pub(crate) fn prepare_block_current() {
    if let Some(cur) = current() {
        cur.wake_pending.store(false, Ordering::Release);
    }
}

/// The current-proc pointer lives in the CPU's own `Mach.current_proc`
/// slot (`%gs:0`-indexed, SMP-02) — written only by the owning CPU's
/// process/boot/trap context, so a load on this CPU is stable for the
/// reader's lifetime (the module header's invariant).  Returns null
/// before the first exec.
pub fn current_proc() -> *mut Proc {
    crate::dat::this_mach().current_proc.load(Ordering::Relaxed)
}

/// Install the current-process pointer for THIS CPU (the exec/switch
/// path only).
pub(crate) fn set_current_proc(p: *mut Proc) {
    crate::dat::this_mach().current_proc.store(p, Ordering::Relaxed)
}

/// The ONE guarded entry point for dereferencing the current process.
/// None before the first exec.
pub fn current() -> Option<ProcGuard> {
    let p = current_proc();
    if p.is_null() { None } else { Some(ProcGuard(p)) }
}

/// True while any non-Moribund proc exists (the EXITS path: when none
/// remain, the all-exited marker and the clean halt follow).
pub fn any_active_procs() -> bool {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table.procs.iter().flatten().any(|p| p.state != ProcState::Moribund)
}

/// 17-02 (dev_churn): the drain probe — true while any proc OTHER than
/// `my_pid` is non-Moribund (the churn driver's final wait: every
/// nowait contention generation's contenders gone before the end
/// sample).  any_active_procs minus self, the table- private-field
/// read living here beside its sibling.
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

/// The mid-construction unwind for new_user_proc: free the PML4 page
/// and return the kernel-stack slot, for whichever is already held
/// (`slot` is 0 when none was taken).  Never called with a lock held.
fn unwind_new_proc(pml4_pa: u64, slot: u64) {
    free_phys_page(pml4_pa);
    if slot != 0 {
        kstack::free(slot);
    }
}

/// Allocate and register a fresh user process: pid, a kernel stack from
/// the boot-reserved arena (kstack.rs — reachable from every
/// CPU through the KZERO alias), and a per-process PML4 with the kernel
/// higher half shared (the OQ-2 resolution).  The proc starts with no
/// segments, no byte source, and an empty mapping log; fd 0/1/2 are the
/// devcons preopen.
///
/// Returns a ProcGuard, never a `&'static mut Proc`.
pub fn new_user_proc(name: &str) -> Result<ProcGuard> {
    // The per-process PML4 first (BEFORE taking the table lock — the
    // lock is never held across an allocation): one fresh zeroed page;
    // the kernel half is copied from the boot PML4 so the direct map,
    // the Mach window and the KZERO alias are live under EVERY table.
    let pml4_pa = alloc_phys_page().map_err(|_| "proc: PML4 page alloc failed")?;
    #[cfg(not(test))]
    copy_kernel_half(pml4_pa);
    // The kernel stack: one contiguous arena slot, no page-table work.
    // It is taken before the table lock, like the PML4 page, so the
    // lock is never held across it; a failure frees the page taken so
    // far (17-01, the WR-03 OOM-release family).
    let slot = match kstack::alloc() {
        Some(slot) => slot,
        None => {
            unwind_new_proc(pml4_pa, 0);
            return Err("proc: kernel stack alloc failed (arena full)");
        }
    };
    let kstack_top = kstack::top_va(slot);
    let mut kstack_pas = [0u64; KSTACK_PAGES];
    for (i, pa) in kstack_pas.iter_mut().enumerate() {
        *pa = slot + (i * PAGE_SIZE_4K) as u64;
    }

    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let pid = table.next_pid;
    table.next_pid += 1;
    let Some(slot) = table.procs.iter_mut().find(|s| s.is_none()) else {
        // The table-full path must free the already-allocated page and
        // return the stack slot.  Drop the PROC_TABLE guard BEFORE the
        // frees (the module locking discipline); the burned pid is
        // harmless.
        drop(table);
        unwind_new_proc(pml4_pa, kstack_pas[0]);
        return Err("proc: table full");
    };
    let mut proc = Proc::new(pid, name, kstack_top, kstack_pas, pml4_pa);
    proc.preopen_cons_fds();
    *slot = Some(proc);
    // The table is a fixed static and slots are never freed this era,
    // so the handle stays valid after the guard drops.
    Ok(ProcGuard(ptr::from_mut(slot.as_mut().unwrap())))
}

/// A guard handle to a PROC_TABLE-resident Proc.  Deref/DerefMut reach
/// the proc; the table slots are never freed this era and handles live
/// only in the OWNING CPU's boot/syscall/trap context (the per-CPU
/// `Mach.current_proc` slot is written only by that CPU), so the target
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

/// The live-frame record (PER CPU, SMP-02): the CPU's own
/// `Mach.cur_frame` slot (`%gs:0`-indexed) — written by the owning CPU
/// under the PROC_TABLE lock or in its IRQ-masked trap context; read by
/// the note-delivery boundary (the frame the return path actually
/// restores).
fn read_curframe() -> *mut Ureg {
    crate::dat::this_mach().cur_frame.load(Ordering::Relaxed) as *mut Ureg
}

/// The live Ureg the resume boundary will restore (the note-delivery
/// path reads it).
pub(crate) fn current_frame() -> *mut Ureg {
    read_curframe()
}

fn write_curframe(frame: *mut Ureg) {
    crate::dat::this_mach().cur_frame.store(frame as u64, Ordering::Relaxed)
}

/// The kstack-ownership record (05-24, the aarch64 `write_curkstack`
/// port): the identity of the proc kstack this CPU currently owns,
/// written beside every TSS rsp0 re-point in the switch paths
/// (activate_first/pick_next) and cleared by the sched loop's
/// `release_left_proc` when the CPU leaves the proc's kstack.
/// pick_next's debug tripwire reads it cross-CPU — the TSS rsp0 itself
/// can never serve: it is not retracted when the CPU parks into the
/// scheduler, so it would stale-name a released proc's stack.
///
/// The identity is the kstack top page's PHYSICAL address ([`kstack_id`])
/// — unique among live stacks, and independent of how stacks are
/// mapped.  It was ported as a VA compare against a per-process window
/// in which every proc's kstack sat at the same VA; the tripwire then
/// fired for every proc a second CPU picked while the first ran any
/// process (observed live under KVM and TCG: "pid 2 picked while cpu 0
/// still names its kernel stack 0xffffc80000004000" — pid 1's stack VA
/// as well).  The arena (kstack.rs) gives each stack its own VA too;
/// the PA stays as the identity so that cannot regress by a mapping
/// change.
fn write_curkstack(top_page_pa: u64) {
    crate::dat::this_mach().cur_kstack.store(top_page_pa, Ordering::Relaxed)
}

/// The kstack identity `write_curkstack` records for `p`: its top page
/// PA — unique among live kstacks, and read-only for the proc's life.
fn kstack_id(p: &Proc) -> u64 {
    p.kstack_pas[KSTACK_PAGES - 1]
}

/// The calling CPU's Mach scheduler stack top (the swtch-based sched
/// loop's context runs here — free while the idle loop owns the entry).
/// Per CPU (SMP-02): each CPU's loop runs on its own Mach's stack.
#[cfg(not(test))]
fn scheduler_stack_top() -> usize {
    crate::dat::this_mach().stack.top().addr()
}

/// First activation (exec_bootstrap's only call): publish the proc as
/// current + Running, point the TSS rsp0 at its kstack — the mechanism
/// every ring-3 entry (SYSCALL and IDT alike) then shares — and record
/// the seeded frame in this CPU's `Mach.cur_frame` (the live-frame
/// record the scheduler's preempt path stashes from).  The proc was
/// enqueued Ready by exec_into (Ready ⟺ enqueued); the claim here makes
/// it Running directly, and pick_next skips the now-stale queue entry.
/// Takes the kstack-ownership token (`Proc::on_cpu`, 05-24): this CPU
/// owns the stack until it swtch'es off it into its scheduler context.
pub fn activate_first(p: &mut Proc) {
    p.state = ProcState::Running;
    p.on_cpu.store(true, Ordering::Release);
    set_current_proc(p);
    set_tss_rsp0(p.kstack_top as u64);
    write_curkstack(kstack_id(p));
    write_curframe(p.frame);
    // Install: the proc's user FPU/SSE state (exec loaded the default)
    // goes live.
    crate::fpu::install(&mut p.fpsave);
}

/// 17-02 (dev_churn): seed a freshly built proc's KERNEL entry — the
/// per-CPU sched_context seeding shape re-keyed onto a proc.  The
/// context resume jumps into `entry` on the proc's own kstack (rsp one
/// qword under the 64-KiB-aligned top, SysV entry-aligned); no Ureg, no
/// ring 3 — the churn driver's orchestrator context.  SMP-11: the seed
/// is the moment the proc becomes schedulable, so it enters the run
/// queue here (Ready ⟺ enqueued); pick_next installs it and the
/// scheduler's ONE swtch arm enters it.
#[cfg(dev_churn)]
pub(crate) fn seed_kernel_context(p: &mut Proc, entry: fn() -> !) {
    p.context.set_return(entry as *const () as u64);
    #[cfg(not(test))]
    p.context.set_stack_pointer(p.kstack_top as u64 - 8);
    enqueue_ready(p.pid);
}

/// 17-02 (dev_churn): the first-resume seeding for a proc built by
/// exec_into (the bootstrap's direct-trap_return path never runs for
/// it): the rfork_copy tail verbatim — the context enters the
/// first_entry trampoline with the planted Ureg in rbx, so the
/// scheduler's ONE swtch arm performs the ring-3 entry like any
/// fork's.
#[cfg(dev_churn)]
pub(crate) fn first_resume_seed(p: &mut Proc) {
    p.context.set_return(first_entry as *const () as u64);
    #[cfg(not(test))]
    p.context.set_stack_pointer(p.kstack_top as u64 - 0x400);
    p.context.rbx = p.frame as u64;
}

/// 17-02 (dev_churn): park the boot context into the scheduler loop —
/// the one-way handoff for the profile whose first "process" is a
/// kernel-context orchestrator (the churn driver): the boot context
/// never resumes (its save lands in this CPU's `Mach.park_context`
/// scratch slot — nothing reads it, SMP-02), and sched_loop runs on
/// this CPU's Mach scheduler stack from here on, dispatching the
/// already-Ready driver.
#[cfg(dev_churn)]
pub(crate) fn enter_sched_loop_from_boot() -> ! {
    // SAFETY: runs once from this CPU's boot context (IRQ-masked,
    // driver already Ready); this CPU's park_context is never read
    // again, and sched_context()'s per-CPU seed-once discipline covers
    // the target.
    unsafe { swtch::swtch(crate::dat::this_mach().park_context.get(), sched_context()) }
    unreachable!("the boot context parked into the scheduler loop")
}

/// The timer-tick preemption entry.  Acts only when the current proc
/// is Running — i.e. the IRQ interrupted ring 3 (kernel syscall/trap
/// context runs IRQ-masked; the sched loop's idle hlt finds current
/// Blocked/Moribund and its own post-hlt re-scan dispatches instead).
///
/// SMP-03 (the 05-22 proven gate): this CPU's `Mach.current_proc` is
/// STALE while the CPU is idle — it still names the last proc this CPU
/// ran, and under SMP that proc can have been woken and be Running on
/// ANOTHER CPU.  A state check alone cannot tell "Running here" from
/// "Running elsewhere": rescheduling then would stash this CPU's idle
/// cur_frame over the proc's LIVE frame and re-enqueue it mid-run (the
/// double-run tear).  The `interrupted_user` gate (the trap frame's cs
/// ring bits, threaded in from trap's dispatch) closes it: preempt acts
/// only when the IRQ interrupted a user proc on THIS CPU.  An idle
/// CPU's tick/resched IPI (kernel-context hlt) passes false and its
/// loop simply re-pops after the iret — the wake is delivered by the
/// enqueue, not by this IRQ.
pub fn preempt(interrupted_user: bool) {
    if !interrupted_user {
        return;
    }
    let Some(cur) = current() else {
        return;
    };
    if cur.state != ProcState::Running {
        return;
    }
    reschedule();
}

/// Round-robin reschedule from the tick (IRQ-from-ring-3: the live
/// frame sits at the current proc's kstack top, recorded in this CPU's
/// `Mach.cur_frame`) or from a yield (sys_sleep).
///
/// THE HANDOFF RULE (05-24, the aarch64 339036f port): the preempted
/// proc must NOT be published Ready/enqueued while this CPU is still
/// running on its kernel stack.  This function therefore only PARKS the
/// CPU's kernel context on the current proc's kstack (`swtch` to this
/// CPU's scheduler context) and lets the sched loop perform the actual
/// publish: the loop's `release_left_proc` runs on the Mach scheduler
/// stack — after this CPU has left the proc's kstack — and only there
/// is the proc marked Ready and enqueued (see `Proc::on_cpu`).  The
/// pre-port shape published it HERE (state + enqueue under PROC_TABLE)
/// and only then swtch'ed off the stack: a sibling CPU could pop the
/// pid in that window and swtch into a kernel context this CPU had not
/// even saved yet (and later take ring-3 entries onto the shared
/// kstack) — the overlapping-frames class the aarch64 HVF `-smp 2`
/// fork-sweep corruption proved live.
///
/// The caller's kernel frames (tick / syscall) stay on the proc's
/// kstack and are resumed by the sched loop's swtch when the proc is
/// picked again — the same mid-kernel resume shape `sched()`'s blocked
/// procs use, which is why `blocked_in_kernel` is set: `pick_next`
/// resumes this context via its ONE swtch arm, never by trap-returning
/// the user frame over it.
///
/// 14-06 root-cause note (kept): the pre-14-06 reschedule performed the
/// CR3 switch ITSELF, inside the lock scope, while executing on the
/// CURRENT proc's kstack at the FIXED window VA (PML4[400]) — the
/// moment CR3 flipped, that VA aliased the NEXT proc's kstack pages,
/// destroying every spilled local, the MCS LockNode, and the return
/// chain mid-flight.  Park-and-swtch never switches CR3 in the
/// preempted proc's context at all — the same discipline sched() has
/// proven since 14-05's await path.
pub fn reschedule() {
    let Some(mut cur) = current() else { return };
    // Fast path: nothing is waiting on the run queues, so there is no
    // switch to make — keep this proc on the CPU without a scheduler
    // round trip.  A stale queue entry can make this probe say yes
    // needlessly; the sched loop then discards it (pick_next's Ready
    // check) and re-picks this proc.
    if !RUN_QUEUES.any_enqueued(port::smp::num_online_cpus()) {
        return;
    }
    // Stash the live frame (this trap's frame on the proc's kstack) —
    // pick_next hands it back through this CPU's cur_frame on the
    // resume.  No PROC_TABLE hold: the proc is Running and unenqueued
    // (no picker can see it), and `on_cpu` pins its kstack to this CPU
    // until the sched loop's release.
    cur.frame = read_curframe();
    // The resume form: the sched loop swtches back into this context
    // (mid-reschedule, on this kstack) and it unwinds normally.
    cur.blocked_in_kernel = true;
    // Leave: the live FPU/SSE registers are cur's user state (the kernel
    // never touches them) — park them here; the next proc's install
    // happens in pick_next, on the sched loop.
    crate::fpu::leave(&mut cur.fpsave);
    // The boot table goes into CR3 before the switch: everything from
    // here on is scheduler context, which never runs on a proc's table.
    leave_proc_table();
    // Park into the scheduler context.  The swtch saves this proc's
    // callee-saved state into its PROC_TABLE-resident context — never
    // this kstack — so nothing under this frame is invalidated by any
    // later switch; and the scheduler context it lands on runs on the
    // boot table (leave_proc_table, just above).
    // SAFETY: sched_context()'s per-CPU seed-once discipline (this CPU's
    // IRQ-masked trap context); cur.context is PROC_TABLE-resident (a
    // stable address across the switch).  No lock is held; the resumed
    // path returns to the caller (tick / sys_sleep) and continues its
    // normal unwind.
    unsafe {
        swtch::swtch(&mut cur.context, &*sched_context());
    }
    // Resumed: pick_next re-installed this proc (current + rsp0 +
    // cur_frame + CR3, one locked step) before swtch'ing back — our
    // kstack pages own this VA again.  Return up through preempt and
    // the interrupt path; the iretq pops the frame at our kstack top.
}

/// The sched loop's publish step (05-24, the aarch64
/// `release_left_proc` port): this CPU has just swtched off the proc
/// named by this CPU's `Mach.current_proc` onto its Mach scheduler
/// stack, so the proc's kernel stack is no longer in use and it may
/// become runnable elsewhere.  Clears `on_cpu` (the kstack-ownership
/// token `pick_next` enforces) and, for a PREEMPTED proc (state still
/// Running — `reschedule` deliberately left the state alone), marks it
/// Ready and enqueues it (SMP-11: Ready ⟺ enqueued) so the loop's pick
/// can run it again.  A Blocked proc stays Blocked (a waker owns its
/// readiness; one that woke inside the sched() window already sits
/// Ready+enqueued, so nothing is repeated), and a Moribund proc is left
/// alone (the deferred reap frees its pages).
///
/// Runs on the owning CPU only (the Mach slots read here are this
/// CPU's), under PROC_TABLE — so the state write, the on_cpu clear and
/// the enqueue are one atomic step for every picker — and before
/// `reap_moribund` (which frees an exiting proc's kstack).  Consuming
/// `Mach.current_proc` makes it a no-op on every loop iteration that
/// did not follow a swtch-off (an idle period, a resumed-switch
/// return): only the CPU that just left a proc can publish it.
fn release_left_proc() {
    let cur = current_proc();
    if cur.is_null() {
        return;
    }
    // SAFETY: the per-CPU current pointer names a PROC_TABLE-resident
    // Proc (slots are never freed this era) and is read on the owning
    // CPU; the pid field is a plain u64 write-once at construction.
    let cur_pid = unsafe { (*cur).pid };
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    // The release is strictly OWNER-scoped: consume this CPU's current
    // pointer in the same critical section.  It is the token that says
    // "this CPU just left that proc", and clearing it is what keeps a
    // later loop iteration — after the CPU went idle with a stale
    // `current_proc` naming a proc that has since migrated to another
    // CPU — from releasing (and re-enqueueing!) a proc that CPU is
    // actively running.
    set_current_proc(ptr::null_mut());
    let Some(p) = table.procs.iter_mut().flatten().find(|q| q.pid == cur_pid) else {
        return;
    };
    if !p.on_cpu.swap(false, Ordering::AcqRel) {
        return; // never started, or already released: nothing to publish
    }
    // This CPU no longer names the proc's kstack: retract the
    // live-frame/kstack records pick_next's ownership tripwire reads.
    // The next pick republishes both for the proc it runs.  (The TSS
    // rsp0 is deliberately NOT re-pointed here: no ring-3 entry can
    // happen while this CPU sits in its sched loop, and the next pick
    // re-arms it before any resume.)
    write_curkstack(0);
    write_curframe(ptr::null_mut());
    if p.state == ProcState::Running {
        // Preempted (reschedule parks a Running proc).  Publish it now
        // — the CPU is off its stack — and record the switch's source
        // for pick_next's `sched: switch` marker (the preemption
        // evidence the boot gates count).
        p.state = ProcState::Ready;
        crate::dat::this_mach().sched_from.store(p.pid, Ordering::Relaxed);
        enqueue_ready(p.pid);
    }
}

/// A fresh (never-entered) proc's first ring-3 entry: EVERY resume —
/// first entry included — rides the sched loop's swtch arm (see
/// sched_loop for why the loop must never diverge via trap_return).
/// The fork child's context is seeded to enter here with its planted
/// Ureg pointer carried in rbx (one of swtch's saved registers); this
/// shim hands it to trap_return, which iretq's into ring 3 and
/// diverges ON THE CHILD'S OWN KSTACK — the scheduler's context was
/// re-saved by the swtch that landed here, so it stays resumable.
#[cfg(not(test))]
#[unsafe(naked)]
unsafe extern "C" fn first_entry() -> ! {
    use core::arch::naked_asm;
    naked_asm!(
        "movq %rbx, %rdi",
        "jmp {target}",
        target = sym crate::trap::trap_return,
        options(att_syntax),
    )
}

/// The host-test stand-in (the swtch.rs doctrine): no kernel stacks
/// exist on the host — loud, never a silent fallthrough.
#[cfg(test)]
unsafe extern "C" fn first_entry() -> ! {
    unreachable!("first_entry is unavailable on the host (no kernel stacks)")
}

/// The scheduler context — a PER-CPU kernel-context switch target
/// running sched_loop on the CPU's own Mach scheduler stack (SMP-02).
/// Lives in the CPU's `Mach.sched_context` slot (`%gs:0`-indexed);
/// seeded once per CPU on first use; accessed only from the owning
/// CPU's IRQ-masked contexts.
fn sched_context() -> *mut swtch::Context {
    let cpu = crate::dat::this_mach();
    if !cpu.sched_seeded.load(Ordering::Acquire) {
        let ctx = cpu.sched_context.get();
        // SAFETY: per-CPU seed-once — only this CPU writes its own
        // sched_context, here, before sched_seeded publishes (Release);
        // every later access is this CPU's swtch source/target
        // (sched/sched_from_moribund/sched_loop itself), so no
        // concurrent access is possible.
        unsafe {
            (*ctx).set_return(sched_loop as *const () as u64);
            // SysV wants %rsp ≡ 8 (mod 16) at a function entry (as
            // after a call); the Mach stack top is 64 KiB-aligned, so
            // seed one qword down.
            #[cfg(not(test))]
            (*ctx).set_stack_pointer(scheduler_stack_top() as u64 - 8);
        }
        cpu.sched_seeded.store(true, Ordering::Release);
    }
    cpu.sched_context.get()
}

/// Block the CURRENT proc mid-syscall and switch to this CPU's
/// scheduler context (the AWAIT path).  Returns when the sched loop
/// swtch'es back — i.e. after the proc was woken (Blocked → Ready by a
/// waker) and picked off this CPU's run queue (or stolen onto it).
/// Ring-3 entry/exit never rides this path.
///
/// SMP-03 (D-22, Pitfall 4): the wake_pending consume and the Blocked
/// transition run in ONE PROC_TABLE critical section, so a concurrent
/// wake on another CPU either (a) landed before this section — its
/// wake_pending store is consumed here and we return WITHOUT sleeping —
/// or (b) lands after — its waker sees Blocked and readies + enqueues
/// us.  No interleave can park the proc past its wake.
pub fn sched() {
    let mut cur = current().expect("sched with no current process");
    {
        let node = LockNode::new();
        let _table = PROC_TABLE.lock(&node);
        if cur.wake_pending.swap(false, Ordering::AcqRel) {
            // A wake landed between this proc's prepare_block() /
            // registration and here — return without sleeping; the
            // caller's loop re-checks its condition.
            return;
        }
        cur.state = ProcState::Blocked;
        cur.blocked_in_kernel = true;
        // Leave: park the user FPU/SSE state — the sched loop installs
        // the next proc's (possibly this one again, from this save).
        crate::fpu::leave(&mut cur.fpsave);
    }
    // The boot table goes into CR3 before the switch: everything from
    // here on is scheduler context, which never runs on a proc's table.
    leave_proc_table();
    // SAFETY: sched_context()'s per-CPU seed-once discipline (this CPU's
    // IRQ-masked syscall context); cur's context field is
    // PROC_TABLE-resident (a stable address across the switch).  The
    // Blocked transition ran under PROC_TABLE above (the lock is never
    // held across a swtch — the resumed proc may lock it).
    unsafe {
        swtch::swtch(&mut cur.context, &*sched_context());
    }
    // Resumed inside the syscall handler; the caller re-checks its
    // wait records and returns up the return tail.
}

/// The scheduler loop (entered via swtch on this CPU's Mach scheduler
/// stack — or by `ap_main`'s direct call on a secondary, SMP-01): pop
/// the next pid off this CPU's run queue (stealing a neighbor's when
/// empty — pick_next wraps `RUN_QUEUES.pop_or_steal`, SMP-11), apply
/// the TSS rsp0 + cur_frame + CR3 + current-proc swap under the
/// PROC_TABLE lock BEFORE the resume, then resume it through THE ONE
/// swtch arm.  Nothing queued → hlt with IRQs briefly unmasked (the
/// consolidated `idle_wait`: sti;hlt;cli — the tick fires and re-arms,
/// a wake's enqueue IPIs the idle CPU with 0xF1, and the loop re-pops).
///
/// 05-24 (the aarch64 339036f port): the loop runs with IRQs MASKED
/// from entry (the `mask_irq` below), and each iteration's top
/// publishes the proc this CPU just left (`release_left_proc` — the
/// kstack-ownership handoff) and frees any proc this CPU exited
/// (`reap_moribund` — the deferred moribund reap) BEFORE the next
/// pick, so no publish or free ever runs while the CPU still stands on
/// the proc's kstack.
///
/// 14-06 root cause this shape closes: the loop ONCE had a second arm
/// — `Resume::Return` — that called trap_return(frame) directly for a
/// fresh fork's first entry.  trap_return diverges (it never
/// returns), so that arm left SCHED_CONTEXT holding the LAST
/// Switch-arm save — a save whose saved sp/return-address slot the
/// scheduler's own continuation had already popped and reused (the
/// next pick_next call lands its return address on exactly that
/// slot).  The next blocked proc's swtch-to-SCHED_CONTEXT then re-
/// entered the loop at the STALE save: the ret popped a reused slot
/// and the match read its discriminant from whatever RAX carried —
/// execution went wild with corrupted stack locals, producing the
/// chain's deterministic "lying panic" right after init's first block
/// (different panic message per build layout, self-contradictory
/// values — cold-block-merged panic entries under a garbage register
/// state; the truncated prints were the wild path landing inside the
/// console lock's usage).  Now EVERY resume rides the swtch arm —
/// fresh forks enter via the first_entry trampoline seeded into their
/// context at fork time — so each resume re-saves this CPU's
/// sched_context and every sched() re-entry lands on a CURRENT save.
pub(crate) fn sched_loop() -> ! {
    // The scheduler contract: this loop runs with IRQs MASKED.  Most
    // callers are already masked (a trap entry, sched()'s swtch from
    // IRQ-masked syscall context, the AP trampoline's cli), but the
    // dev_churn boot context reaches `enter_sched_loop_from_boot` with
    // IF=1 (main's sti precedes the churn bootstrap) — and an IRQ
    // taken inside the pick_next→swtch handoff would run the tick on
    // the Mach scheduler stack with a stale current_proc (the aarch64
    // secondary_main class: an IRQ in the handoff window rewrote the
    // CPU's live-frame record from the scheduler stack).  Mask
    // defensively so the contract holds however this CPU entered; the
    // idle arm's `idle_wait` is the one deliberate unmask window.
    crate::trap::mask_irq();
    let mut all_blocked = 0u32;
    loop {
        // Publish the proc this CPU just left (sched/reschedule/
        // moribund swtch): clear its kstack-ownership token and, for a
        // preempted proc, re-enqueue it.  Runs on the Mach scheduler
        // stack — after the CPU has left the proc's kstack — which is
        // the whole point of the handoff rule (see reschedule's doc).
        release_left_proc();
        // Free any proc this CPU exited since the last iteration (the
        // deferred moribund reap, 05-24) — the hand-off swtch already
        // moved this CPU off the dead kstack onto the boot table
        // (leave_proc_table).
        reap_moribund();
        match pick_next() {
            // SAFETY: sched_context() is seeded (its own per-CPU
            // discipline); ctx names the picked proc's
            // PROC_TABLE-resident saved context (a mid-syscall block's
            // sched() save, a preempt park's save, or the fork-seeded
            // first_entry trampoline) — the pointer escaped the lock
            // under pick_next's discipline (the Ready check under
            // PROC_TABLE filtered stale entries; slots are never freed
            // this era).
            Some(ctx) => unsafe { swtch::swtch(&mut *sched_context(), &*ctx) }
            None => {
                all_blocked += 1;
                if all_blocked == 200_000 {
                    println!("wedgedbg: fully idle — dumping (pipe-RPC wedge shape)");
                    dump_proc_states();
                }
                // The consolidated idle-wait primitive (briefly
                // unmasked hlt — the tick fires and re-arms, and a
                // wake's enqueue IPI (0xF1) wakes an idle CPU out of
                // the halt without waiting for the next tick).
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
        let name = p.name_str();
        // 14-06 cycle 3: the dump must SURVIVE a corrupt parent —
        // it exists to diagnose wedges, and the first live run of the
        // chain wedged with one proc holding a garbage parent pointer
        // (the dump crashed before naming the blocked procs).  Print
        // the raw pointer when it is outside the table; the audit of
        // WHO set it follows from the printed value.  The range is
        // measured from the LOCKED payload (the IrqSpinLock wrapper's
        // header fields sit before it), never the static wrapper.
        let lo = ptr::from_ref(&*table) as usize;
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

/// Pop the next pid off this CPU's run queue (stealing a neighbor's
/// when empty — `RUN_QUEUES.pop_or_steal`, SMP-11), resolve it under
/// PROC_TABLE, and apply the full switch as one PROC_TABLE-locked step
/// (state, on_cpu, current-proc pointer, TSS rsp0, cur_kstack,
/// cur_frame, CR3) so the resumed proc's return path reads its OWN
/// frame, never the previous proc's.  Returns the proc's saved kernel
/// context — every resume (mid-syscall block, preempt park, or a fresh
/// fork's first_entry trampoline) rides sched_loop's ONE swtch arm.
/// None when every queue is empty (or every candidate is kstack-owned
/// by a sibling — the cycle guard) — the caller's idle arm then hlt's;
/// a wake's enqueue IPIs an idle CPU with 0xF1.
///
/// Stale entries are skipped, never fatal: a pid whose proc is no longer
/// Ready (already Running on a CPU — e.g. pid 1 claimed by
/// activate_first, which leaves its exec_into queue entry behind by
/// design; or unwound to Moribund mid-rfork) carries a stale entry that
/// the state check filters.  An `on_cpu` entry (05-24: a proc whose
/// kstack a sibling CPU has not released yet — the sched()-window wake)
/// is requeued, never started, and the debug tripwire panics if another
/// CPU's `cur_kstack` still names the candidate's stack.  The converse
/// never happens: Ready ⟺ enqueued (every Ready transition routes
/// through enqueue_ready), so a genuinely Ready proc always has an
/// entry to be found by.
///
/// The FIFO-per-CPU pop preserves the local ordering the old table scan
/// had (a proc that blocks via sched() REMAINS this CPU's current proc
/// while the sched loop runs, and its wake enqueues it — the pop finds
/// it; the old skip-current scan bug of the cons read path has no
/// analogue here).
fn pick_next() -> Option<*mut swtch::Context> {
    let my_cpu = port::smp::current_cpu_id();
    let num = port::smp::num_online_cpus();
    let mut first_skipped: Option<u64> = None;
    loop {
        let pid = RUN_QUEUES.pop_or_steal(my_cpu, num)?;
        let node = LockNode::new();
        let mut table = PROC_TABLE.lock(&node);
        let Some(p) = table.procs.iter_mut().flatten().find(|q| q.pid == pid) else {
            continue; // stale entry: the proc unwound to Moribund mid-rfork
        };
        if p.state != ProcState::Ready {
            continue; // stale entry: already Running/picked elsewhere
        }
        if p.on_cpu.load(Ordering::Acquire) {
            // The kstack-ownership token (05-24): a CPU still owns this
            // proc's kernel stack (it has not reached its sched-loop
            // release yet — the sched() window's waker raced its
            // swtch).  NEVER start it here; requeue and try another.
            // Two racing picks of the same proc would be caught by the
            // tripwire below failing, but the token makes it impossible
            // by construction (the aarch64 HVF -smp 2 fork/exec-sweep
            // fix, ported).
            if owned_pick_give_up(&mut first_skipped, pid) {
                // Cycled every queue; this proc's owner will release it
                // (and its release's enqueue may IPI this cpu) — idle
                // instead of spinning on the same entry.
                enqueue_ready(pid);
                return None;
            }
            enqueue_ready(pid);
            continue;
        }
        // Tripwire: no OTHER cpu may still name this proc's kernel
        // stack.  `on_cpu` is the authoritative token (checked above);
        // this cross-check catches a publish path that forgot the token
        // — the exact pre-port shape, where `reschedule` enqueued a
        // proc while its cpu was still on the stack.  The per-CPU
        // records are written under PROC_TABLE (activate_first/
        // pick_next) and cleared under it (release_left_proc), so a
        // read under the lock is a consistent snapshot.  Compiled out
        // in release, like aarch64's.
        #[cfg(debug_assertions)]
        {
            let n = num.min(crate::smp::MAX_CPUS);
            let mut c = 0;
            while c < n {
                if c != my_cpu && crate::smp::cur_kstack_of(c) == Some(kstack_id(p)) {
                    panic!(
                        "sched: pid {} picked while cpu {c} still names its kernel stack (top page {:#x}) — two cpus on one kstack (a preempt/block publish raced the stack handoff)",
                        p.pid,
                        kstack_id(p)
                    );
                }
                c += 1;
            }
        }
        p.state = ProcState::Running;
        p.on_cpu.store(true, Ordering::Release);
        set_current_proc(p);
        set_tss_rsp0(p.kstack_top as u64);
        write_curkstack(kstack_id(p));
        write_curframe(p.frame);
        p.blocked_in_kernel = false;
        // Install: the proc's saved user FPU/SSE state goes live (every
        // path into the sched loop parked or dropped the previous one).
        crate::fpu::install(&mut p.fpsave);
        // CR3 last (reschedule's ordering note): the frame pointer is
        // dereferenced only after the lock drops, under this table.
        switch_cr3(p.pml4_pa);
        // The preemption evidence: release_left_proc stored the
        // preempted proc's pid when it published this switch's source.
        // PROC_TABLE → Console is the documented nesting order.
        let from = crate::dat::this_mach().sched_from.swap(0, Ordering::Relaxed);
        if from != 0 {
            println!("sched: switch {from}->{}", p.pid);
        }
        return Some(&mut p.context as *mut swtch::Context);
    }
}

/// pick_next's on_cpu refusal cycle guard (05-24 — factored pure for
/// the host pin): an `on_cpu` proc is requeued, never started; the
/// guard records the FIRST refused pid, and a SECOND refusal of that
/// same pid (this CPU cycled every queue and the owner still has not
/// released the kstack token) ends the pick — the caller idles (the
/// owning CPU's release re-enqueues, and that enqueue may IPI this CPU)
/// instead of spinning on the same entry.  Returns true when the pick
/// must stop.
fn owned_pick_give_up(first_skipped: &mut Option<u64>, pid: u64) -> bool {
    if *first_skipped == Some(pid) {
        return true;
    }
    *first_skipped = first_skipped.or(Some(pid));
    false
}

/// EXITS's tail: the Moribund current proc never runs again — switch
/// into the scheduler context with no wake path.  The swtch save
/// writes only the PROC_TABLE-resident context struct (never the
/// kstack).  05-24: the kstack/PML4 pages are NOT freed before this
/// call anymore — exit_tail recorded them on this CPU's
/// `Mach::moribund_reap` slot (`defer_root_kstack_reap`), and the sched
/// loop's `reap_moribund` frees them after this swtch, once the CPU
/// stands on its Mach scheduler stack (the aarch64 deferred-reap port:
/// a sibling CPU's allocator could otherwise recycle a kstack page
/// while this CPU still runs on it up to the swtch).  Diverges.
pub fn sched_from_moribund() -> ! {
    let cur = current();
    // SAFETY: sched_context()'s seed-once discipline; the moribund
    // proc's context is PROC_TABLE-resident and its kstack stays mapped
    // until the sched loop's deferred reap (the swtch save writes only
    // the context struct regardless).
    unsafe {
        let ctx = sched_context();
        if let Some(mut cur) = cur {
            // The dead proc's live FPU/SSE state is dropped, unsaved.
            crate::fpu::discard(&mut cur.fpsave);
            // The dead proc's table leaves CR3 first: its root page is
            // freed by the reap this swtch hands over to.
            leave_proc_table();
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

/// Wake the procs blocked on a pipe queue (devpipe's write/close calls
/// with the queue's waiter list): the wake critical section per pid —
/// wake_pending published (Release) and Blocked → Ready + run-queue
/// enqueue under PROC_TABLE (blocked_in_kernel stays set — the sched
/// loop resumes them via swtch, the post_wait_record pattern).  The
/// port-side caller ran `port::smp::prepare_block()` and registered the
/// pid under the Pipe lock before dropping it (the devpipe SMP-03
/// contract).  PROC_TABLE-brief.
pub fn pipe_wake(pids: &[u64]) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if pids.contains(&p.pid) {
            wake_locked(p);
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

/// The fd-group release (the donor's closefgrp), shared by the exit
/// sweep and the bare rfork split: swap the group in `slot` for `fresh`
/// and, only when this holder was the old group's LAST sharer, drain
/// every occupied slot and hand each chan to `close` (namec::close_chan
/// — devpipe's open-count decrement, devmnt's clunk_if_last).  The close
/// dispatch runs AFTER the group lock drops (the fdgrp leaf-lock rule; a
/// close can block).  A non-last sharer only gives up its reference.
///
/// 19-10 cont 6: the drained chans are consumed ONE at a time — a close
/// under a BORROW would pin every duplicate of the same server fid
/// inside the batch, so clunk_if_last would never see the last ref (the
/// donor closefgrp's nil-the-slot-then-cclose cascade).  A RETURNING
/// function on purpose (17-02 #44): exit_tail diverges, so a local it
/// held — the detached Arc above all — would never drop.
pub(crate) fn release_fd_group(
    slot: &mut Arc<Lock<FdGroup<MAX_FDS>>>,
    fresh: Arc<Lock<FdGroup<MAX_FDS>>>,
    mut close: impl FnMut(&Chan),
) {
    let detached = core::mem::replace(slot, fresh);
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
/// release the old handle exactly as an exit does (release_fd_group).
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

/// 17-02 (the #44 fix, WINDOWS row closure): the shared EMPTY pgrp —
/// the exit sweep's singleton (empty_fdgroup's shape).  Slots are never
/// reaped this era, so every group handle a Moribund husk pins is a
/// permanent allocation; the sweep swaps each for a singleton clone
/// (refcount bump only) and lets the old Arc drop — a last sharer's
/// group (and its mount table) dies with it, the donor pexit
/// closepgrp's refcount semantics.
pub fn empty_pgrp() -> Arc<Lock<Pgrp>> {
    let node = LockNode::new();
    let mut guard = EMPTY_PGRP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("pgrp", Pgrp::new())));
    }
    guard.as_ref().unwrap().clone()
}

static EMPTY_PGRP: Lock<Option<Arc<Lock<Pgrp>>>> = Lock::new("emptypgrp", None);

/// The exit sweep's shared EMPTY env group (the RFCENVG-fresh shape).
pub fn empty_env_group() -> Arc<Lock<EnvGroup>> {
    let node = LockNode::new();
    let mut guard = EMPTY_ENVGROUP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("env", EnvGroup::new())));
    }
    guard.as_ref().unwrap().clone()
}

static EMPTY_ENVGROUP: Lock<Option<Arc<Lock<EnvGroup>>>> = Lock::new("emptyenvgrp", None);

/// The exit sweep's shared EMPTY rendezvous group (the RFREND-fresh
/// shape).
pub fn empty_rendez_group() -> Arc<Lock<RendezGroup>> {
    let node = LockNode::new();
    let mut guard = EMPTY_RENDEZGROUP.lock(&node);
    if guard.is_none() {
        *guard = Some(Arc::new(Lock::new("rendez", RendezGroup::new())));
    }
    guard.as_ref().unwrap().clone()
}

static EMPTY_RENDEZGROUP: Lock<Option<Arc<Lock<RendezGroup>>>> = Lock::new("emptyrgrp", None);

/// 17-02 (the #44 fix): the exit sweep — a Moribund husk pins its slot
/// forever, so every heap payload it retains is a permanent allocation
/// (the churn gate measured the whole set: one fresh fd group + the
/// pgrp/env/rgrp Arc shells + the slash/dot chans per exit).  The donor
/// pexit's closefgrp/closepgrp/closergrp re-keyed: the group handles
/// swap for shared singletons (the old Arcs drop here — a last sharer's
/// group dies with it).  The slash/dot chans DROP without the
/// close-dispatch: r9's exit tail is not 9P-safe (a devmnt clunk is a
/// BLOCKING RPC, and the mounted venue measured userland pool
/// corruption when exits talked 9P mid-exit — 17-02's composed-leg
/// catch), so the chan values (their Vecs, the mnt fid Arcs) release
/// by plain drop; the server-side fid of a husk's chan clone leaks
/// until the server dies — a bounded, documented era shape (the last
/// Arc's drop cannot Tclunk without the RPC; the real fix rides the
/// exit-path-9P-safety work).
pub fn sweep_husk_payloads(proc: &mut Proc) {
    // 19-10 cont 6: the donor closepgrp's mount half — when THIS proc
    // holds the LAST reference to its namespace group, the mount
    // table's chans (every mount's ROOT/attach fid) close-dispatch
    // before the handle drops.  A plain Arc drop never Tclunks them:
    // the attach fid leaks server-side (rio's wsys attach carries a
    // window ref — xfidattach's `if(!newlymade) incref(w)`), the
    // shell-exit clunk chain lands one short, and `exit` in a window
    // leaves the window on screen (the owner's live report).
    if Arc::strong_count(&proc.pgrp) == 1 {
        let chans = {
            let node = LockNode::new();
            proc.pgrp.lock(&node).take_mount_chans()
        };
        for c in chans {
            port::namec::close_chan(&c);
        }
    }
    proc.pgrp = empty_pgrp();
    proc.env_group = empty_env_group();
    proc.rgrp = empty_rendez_group();
    let _ = proc.slash.take();
    let _ = proc.dot.take();
}

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
    Some(port::devproc::ProcInfo {
        name: p.name.clone(),
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

/// devproc's ctl hook: record the "private" write on the ADDRESSED pid
/// (the donor CMprivate's p->privatemem = 1).  Accepting + storing is
/// the honest degradation — the /proc mem-read exclusions are a later
/// era.  Missing/Moribund -> the loud error.
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

/// The note-queue append shared by post_note (per-pid) and the note-group post:
/// Moribund targets and full queues are skipped per the donor's
/// postnote failure shape.  A Blocked target is READIED (the donor
/// postnote's ready(p)): the note must reach a proc parked in an
/// indefinite device read — the read's loop re-runs on wake, sees the
/// pending note, and errors "interrupted" (the note-interruptible
/// blocking-IO contract; without the wake, a kill note to a parked
/// reader sits queued forever and the reader's session never tears
/// down — the 19-08 session-exit variance).
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
        wake_locked(p);
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

/// devcons's block hook: park the calling proc in its cons read until
/// the tick's cons_input_wake readies it (input pending).  The
/// register-then-block order cannot lose a wake (SMP-03): cons_waiting
/// is set before sched(), and cons_input_wake's PROC_TABLE critical
/// section publishes wake_pending + the Ready transition against it —
/// sched()'s flag consume under the same lock either sees a wake that
/// landed mid-window (returns without sleeping) or blocks first and is
/// readied.  The flag clears on resume — the reader's post-wake drain
/// re-checks the ring regardless (a spurious wake re-blocks).
pub fn cons_block_current() {
    let mut cur = current().expect("cons read with no current process");
    cur.cons_waiting = true;
    sched();
    // Resumed inside the syscall handler by the sched loop's swtch; this
    // proc is current again (the blocked_in_kernel resume path).
    cur.cons_waiting = false;
}

/// The tick's cons wake: ready every cons-waiting Blocked proc — the
/// caller (timer.rs) has already seen the 16550 RX ring non-empty via
/// the locked peek.  PROC_TABLE from tick (trap) context is the
/// reschedule() precedent: the lock is IRQ-safe (IrqSpinLock, rank 50)
/// and only ever held briefly.  wake_locked publishes wake_pending even
/// for a cons-waiting proc caught mid-block-window (still Running), so
/// its sched() returns without parking (SMP-03, Pitfall 4).
pub fn cons_input_wake() {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.cons_waiting {
            wake_locked(p);
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
/// waker's value and readied.  Caller MUST already have taken the
/// sleeper entry off its group.  The PROC_TABLE idiom
/// (post_wait_record's shape): table-resident procs, briefly locked,
/// never from IRQ context (the syscall handler is the only caller).
/// rendval is written in the SAME critical section as the wake_pending
/// publish + Blocked→Ready transition, so the woken (or
/// about-to-block, flag-consuming) sleeper reads a fully-written value
/// after its sched()'s lock acquire pairs with this section's release.
pub fn rendez_wake(pid: u64, value: u64) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let Some(p) = table.procs.iter_mut().flatten().find(|p| p.pid == pid) else {
        return; // died between the park and the wake
    };
    p.rendval = value;
    wake_locked(p);
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
///
/// SMP-03 (Pitfall 4): the expiry condition is evaluated REGARDLESS of
/// the proc's state — a sleeper caught mid-block-window (still Running,
/// its sched() not yet under this lock) gets `wake_pending` published
/// (Release) without the Ready transition, so its sched() consumes the
/// flag and returns without parking past its expiry.
pub fn tick_timeouts(now: u64) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.alarm_at != 0 && now >= p.alarm_at {
            p.alarm_at = 0;
            let _ = push_note(p, b"alarm");
        }
        if p.sleep_until != 0 && (now >= p.sleep_until || !p.note_queue.is_empty()) {
            p.sleep_until = 0;
            wake_locked(p);
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
/// sits Blocked in AWAIT (Blocked → Ready + run-queue enqueue;
/// blocked_in_kernel stays set — the parent resumes through the sched
/// loop's swtch).  The record is kernel memory in the parent's table
/// slot (no cross-process pointers).  PROC_TABLE-brief.
///
/// SMP-03 (D-22, Pitfall 4): the record push, the wake_pending publish
/// (Release) and the Blocked→Ready transition share ONE PROC_TABLE
/// critical section, paired with the parent's sched() flag-consume +
/// Blocked-set under the same lock — so an AWAITing parent either
/// consumes the flag and re-checks (finding the record) or is readied
/// here.
pub fn post_wait_record(parent: *mut Proc, pid: u64, msg: &[u8]) {
    if parent.is_null() {
        return;
    }
    let node = LockNode::new();
    // Held for the posting + wake (the parent pointer is
    // table-resident).
    let _guard = PROC_TABLE.lock(&node);
    // SAFETY: parent names a PROC_TABLE-resident Proc (recorded at
    // rfork; slots are never freed this era) and the table lock is held
    // — the deref cannot race (IRQ-safe ranked lock, brief section).
    let parent = unsafe { &mut *parent };
    let mut rec = WaitRecord { pid, msg: [0; ERRMAX], msg_len: msg.len().min(ERRMAX) };
    rec.msg[..rec.msg_len].copy_from_slice(&msg[..rec.msg_len]);
    parent.wait_records.push_back(rec);
    wake_locked(parent);
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
        core::str::from_utf8(&parent.name).unwrap_or("fork")
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
    // The child starts with the parent's user FPU/SSE state — the live
    // registers when the parent owns them (it is on the CPU in its own
    // rfork syscall), its fpsave otherwise; the child is off the CPU, so
    // the copy lands in its fpsave, which its first install restores.
    child.fpsave = crate::fpu::snapshot(&parent.fpsave);
    // An RFNOWAIT child posts no wait record at EXITS and is never
    // reaped by AWAIT, so it must not count toward the parent's AWAIT
    // block-vs-"no children" pivot.
    if !nowait {
        parent.children_live += 1;
    }
    // SMP-11: the copy succeeded and the child's frame is seeded — enter
    // it into the run queues (Ready ⟺ enqueued; an idle CPU is IPI'd).
    // Before this point the construction-Ready child has a null frame
    // only while no other CPU can pick it (it was never enqueued).
    let child_pid = child.pid;
    enqueue_ready(child_pid);
    Ok(child_pid)
}

/// RFMEM's share arm, for one parent mapping: no copy — the parent's PA
/// maps straight into the child's table at the same VA, the sharer count
/// records the second user, and the child's log gets the record.
fn rfork_share_mapping(child: &mut Proc, rec: &MappingRecord) -> Result<()> {
    let created = pml4_map_user(child.pml4_pa, rec.va, rec.phys as u64, rec.kind.pte())
        .map_err(|_| "rfork: child shared map failed")?;
    for c in created.into_iter().flatten() {
        child.mapping_tables.push(c);
    }
    share_user_page(rec.phys as u64);
    // MEM-15/D-14: an RFMEM-shared Data/Bss page is writable by two
    // procs and must never be dropped by the evictor — mark any cached
    // copy dirty (aarch64's RFMEM dirty-marking rule).
    if let Some(cache) = crate::pagealloc::page_cache() {
        cache.mark_data_dirty(PhysAddr::new(rec.phys as u64));
    }
    child
        .mapping_log
        .insert(rec.va, rec.phys, rec.kind)
        .map_err(|_| "rfork: child mapping record failed")
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
    for rec in parent.mapping_log.iter() {
        if rfmem && matches!(rec.kind, SegKind::Data | SegKind::Bss) {
            rfork_share_mapping(child, &rec)?;
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
        let created = match pml4_map_user(child.pml4_pa, rec.va, pa, rec.kind.pte()) {
            Ok(created) => created,
            Err(_) => {
                // The copied page was never recorded — free it (the
                // sweep arms' record-or-unwind hygiene, 17-01/WR-03
                // family; the walk freed its own intermediates).
                free_phys_page(pa);
                return Err("rfork: child user map failed");
            }
        };
        for c in created.into_iter().flatten() {
            child.mapping_tables.push(c);
        }
        child
            .mapping_log
            .insert(rec.va, pa as usize, rec.kind)
            .map_err(|_| "rfork: child mapping record failed")?;
    }
    // No icache maintenance on the copy: x86_64 has coherent
    // instruction caches — the aarch64 port's per-page CTR_EL0 flush
    // step is deliberately absent here (the re-key the plan records).

    // The RFMEM sweep: the log walk above shares only PRESENT pages, but
    // under RFMEM the whole data segment and bss must end up shared —
    // a page the parent has not touched yet included, since the bss range
    // covers pages brk_-grown and never touched, and the parent may still
    // write a data page it has never read.  Every page from the data
    // segment's start to the end of bss that the parent lacks is
    // materialized now, by kind:
    //
    // - a Data page (the classifier's word for the whole data segment and
    //   for the page the segment ends in, which carries the tail of the
    //   initialised data beside the head of bss) is read from the image for
    //   the parent, and the child shares it as the log walk shares a
    //   present Data page;
    // - a Bss page is born ONE zero page mapped into BOTH tables.
    //
    // Until 2026-11-28 the sweep ran from the page the data segment ends in
    // and zero-filled every page it found absent, that one too, in the
    // parent as in the child: a program's statically initialised data read
    // back as zeros after its first shared-memory fork.  Measured: the C
    // library's heap pool lost its lock hook, so every 9P server the boot
    // chain forks that way died at pc=0 — what the keyfs, auth and services
    // gates were stopping on (the owner's defect (4)).  Its trigger was
    // narrow and easy to miss: a server that posts a service name writes
    // that page first (its printf mallocs) and survives; one that does not
    // dies.
    if rfmem {
        let sweep = parent
            .segments
            .as_ref()
            .map(|segs| (segs.data.0.start, round_up_page(segs.bss.0.end)));
        if let Some((start, end)) = sweep {
            let mut va = start;
            while va < end {
                if parent.mapping_log.get(va).is_some() {
                    // Shared by the log walk above.
                    va += PAGE_SIZE_4K;
                    continue;
                }
                let kind = parent.segments.as_ref().and_then(|segs| segs.classify(va));
                match kind {
                    Some(SegKind::Data) if parent.text_source.is_some() => {
                        fault::ensure_user_resident(parent, va, va + PAGE_SIZE_4K, false)?;
                        let rec = parent.mapping_log.get(va).expect("the data page is now mapped");
                        rfork_share_mapping(child, &rec)?;
                    }
                    Some(SegKind::Bss) => {
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
                        let recorded = parent.mapping_log.insert(va, pa as usize, SegKind::Bss).and_then(
                            |()| child.mapping_log.insert(va, pa as usize, SegKind::Bss),
                        );
                        if let Err(msg) = recorded {
                            // Drop whichever half was recorded, unmap both,
                            // free the page (the sweep's failure hygiene), and
                            // report.
                            parent.mapping_log.remove(va);
                            child.mapping_log.remove(va);
                            pml4_unmap_user(parent.pml4_pa, va);
                            pml4_unmap_user(child.pml4_pa, va);
                            free_phys_page(pa);
                            return Err(msg);
                        }
                        share_new_user_page(pa, 2);
                    }
                    // Nothing the sweep can hold: no segment covers the page,
                    // or data with no byte source to read.  The page stays
                    // absent, as before the sweep existed.
                    _ => {}
                }
                va += PAGE_SIZE_4K;
            }
        }
    }

    // Mirror the segment map + demand-page byte source so the child's
    // future faults resolve in the child.
    child.segments = parent.segments.clone();
    child.text_source = parent.text_source.clone();
    // The child shares the parent's exec generation: its copied data
    // pages are private PAs, and cache keys must not alias a later
    // exec's fresh image.
    child.exec_gen = parent.exec_gen;

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
    // The user copies at fork (donor proc.c newproc:
    // `kstrdup(p->user, up->user)`) — no rfork flag governs it; a cap
    // the parent presents does NOT carry to the child's user.
    child.user = parent.user.clone();

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
    // The child's FIRST resume rides the swtch arm like every other
    // (sched_loop's 14-06 root-cause note): seed its context to the
    // first_entry trampoline — pc = the shim, sp = a kstack scratch
    // (never dereferenced: trap_return replaces rsp before any push),
    // rbx = the planted Ureg (the shim's carrier into rdi).
    child.context.set_return(first_entry as *const () as u64);
    #[cfg(not(test))]
    child.context.set_stack_pointer(child.kstack_top as u64 - 0x400);
    child.context.rbx = child.frame as u64;
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
    use port::rfork::RFPROC;

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
        Proc::new(1, "mock", 0, [0; KSTACK_PAGES], 0)
    }

    /// The kstack-ownership token (05-24, the aarch64 339036f port):
    /// construction starts unowned (pick_next may start the proc); the
    /// claim/release round trip is owner-scoped — the owner's release
    /// observes the claim, and a SECOND release observes nothing
    /// (release_left_proc's `swap(false)` guard, so a stale sched-loop
    /// iteration can never re-publish a proc another CPU now runs).
    #[test]
    fn on_cpu_token_round_trip_is_owner_scoped() {
        let p = mock_proc();
        assert!(!p.on_cpu.load(Ordering::Acquire), "construction: no CPU owns the kstack");
        // pick_next/activate_first's claim.
        p.on_cpu.store(true, Ordering::Release);
        assert!(
            p.on_cpu.swap(false, Ordering::AcqRel),
            "the owning CPU's sched-loop release observes the claim"
        );
        assert!(
            !p.on_cpu.swap(false, Ordering::AcqRel),
            "a second release is a no-op — release_left_proc returns without publishing"
        );
    }

    /// pick_next's on_cpu refusal cycle guard: the FIRST refusal of a
    /// pid requeues it and records the guard; a DIFFERENT owned pid
    /// keeps the first guard (still requeued); the SAME pid a second
    /// time — every queue cycled, the owner still holding the kstack
    /// token — gives up to the idle arm instead of spinning.
    #[test]
    fn owned_pick_give_up_cycles_once_then_idles() {
        let mut first = None;
        assert!(!owned_pick_give_up(&mut first, 7), "first refusal requeues and records");
        assert_eq!(first, Some(7));
        assert!(!owned_pick_give_up(&mut first, 9), "a different owned pid requeues too");
        assert_eq!(first, Some(7), "the guard keeps the FIRST refused pid");
        assert!(
            owned_pick_give_up(&mut first, 7),
            "the same pid again: cycled every queue — the pick stops"
        );
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

    /// The pids registered on a word PA, in registration order (the
    /// aarch64 sem_tests idiom — 17-01's CR-03 pins re-keyed to x86_64).
    fn sem_waiters_for(pa: u64) -> Vec<u64> {
        let node = LockNode::new();
        SEM_WAITERS.lock(&node).iter().filter(|e| e.0 == pa).map(|e| e.1).collect()
    }

    /// CR-03 pin (aarch64's register_is_idempotent, re-keyed): the loop
    /// re-registers before EVERY block — a spuriously-woken waiter's
    /// re-register must not stack duplicates.  Distinct word PAs from
    /// the sibling sem pin above: SEM_WAITERS is one shared static and
    /// cargo runs this bin's tests in parallel.
    #[test]
    fn sem_register_is_idempotent() {
        let pa = 0x9600_0000;
        sem_register_waiter(pa, 11);
        sem_register_waiter(pa, 11);
        assert_eq!(sem_waiters_for(pa), vec![11]);
        sem_deregister_waiter(pa, 11);
        assert_eq!(sem_waiters_for(pa), Vec::<u64>::new());
    }

    /// CR-03 pin (aarch64's wake_consumes_picked_registrations_in_order):
    /// the wake consumes up to delta registrations in registration
    /// order — a picked waiter's re-registration on the next block is
    /// what makes the consume safe.
    #[test]
    fn sem_wake_consumes_picked_registrations_in_order() {
        let pa = 0x9700_0000;
        sem_register_waiter(pa, 21);
        sem_register_waiter(pa, 22);
        sem_wake(pa, 1);
        assert_eq!(sem_waiters_for(pa), vec![22], "the first-registered waiter is consumed");
        sem_wake(pa, 0); // a zero-delta release wakes nobody
        assert_eq!(sem_waiters_for(pa), vec![22]);
        sem_wake(pa, 5); // delta beyond the set drains what's there
        assert_eq!(sem_waiters_for(pa), Vec::<u64>::new());
    }

    /// CR-03 pin (aarch64's reregistered_waiter_is_found_by_the_next_release):
    /// a woken (consumed) waiter that loses the unit race RE-REGISTERS
    /// before re-blocking; the next release finds it — the exact
    /// syscall.rs loop contract (register -> check -> block, per
    /// iteration) the Phase-8 fix landed.
    #[test]
    fn sem_reregistered_waiter_is_found_by_the_next_release() {
        let pa = 0x9800_0000;
        sem_register_waiter(pa, 31);
        sem_wake(pa, 1); // the release wakes (consumes) the waiter
        assert_eq!(sem_waiters_for(pa), Vec::<u64>::new());
        // The waiter runs, loses the unit race, and re-blocks — the
        // loop's register-before-block restores the registration.
        sem_register_waiter(pa, 31);
        assert_eq!(sem_waiters_for(pa), vec![31]);
        sem_wake(pa, 1); // the next release finds it
        assert_eq!(sem_waiters_for(pa), Vec::<u64>::new());
    }

    /// CR-03 pin (aarch64's wake_never_touches_other_words): waiters on
    /// OTHER words are never consumed (the PA keying — RFMEM family
    /// members share a PA; unrelated procs never meet).
    #[test]
    fn sem_wake_never_touches_other_words() {
        let pa = 0x9900_0000;
        let other = 0x9a00_0000;
        sem_register_waiter(pa, 41);
        sem_register_waiter(other, 42);
        sem_wake(pa, 8);
        assert_eq!(sem_waiters_for(pa), Vec::<u64>::new());
        assert_eq!(sem_waiters_for(other), vec![42]);
        sem_deregister_waiter(other, 42);
    }

    /// A deregister on ONE word keeps every OTHER word's waiters in
    /// registration order (the wake's FIFO).  swap_remove moved the
    /// shared Vec's last entry into the freed slot, so this exact
    /// sequence flipped word b's [71, 72] to [72, 71] and woke 72 first
    /// (aarch64 carries the same pin).
    #[test]
    fn sem_deregister_on_one_word_keeps_another_words_fifo() {
        let a = 0x9c00_0000;
        let b = 0x9d00_0000;
        sem_register_waiter(a, 61);
        sem_register_waiter(b, 71);
        sem_register_waiter(b, 72);
        sem_deregister_waiter(a, 61);
        assert_eq!(sem_waiters_for(b), vec![71, 72], "b's registration order survives a's deregister");
        sem_wake(b, 1);
        assert_eq!(sem_waiters_for(b), vec![72], "the first-registered waiter is woken first");
        sem_wake(b, 5);
        assert_eq!(sem_waiters_for(b), Vec::<u64>::new());
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
        assert!(guard.pml4_pa != 0);
        // The kstack is one contiguous arena slot: KSTACK_PAGES pages
        // lowest first, slot-aligned, and no private PML4 window any
        // more (the old slot 400 must stay EMPTY — the stack lives in
        // the kernel half every table shares).  Its first page is the
        // guard on metal (kstack.rs); the records name every page.
        assert!(guard.kstack_pas.iter().all(|&pa| pa != 0));
        let base = guard.kstack_pas[0];
        assert_eq!(guard.kstack_pas.len(), 8, "32 KiB of stack above the guard");
        assert!(base.is_multiple_of(crate::kstack::SLOT_BYTES as u64), "the slot is slot-aligned");
        for (i, &pa) in guard.kstack_pas.iter().enumerate() {
            assert_eq!(pa, base + (i * PAGE_SIZE_4K) as u64, "the slot's pages are contiguous");
        }
        assert_eq!(guard.kstack_top as u64, crate::kstack::top_va(base) as u64, "the top is the slot's end");
        assert_eq!(table_entry(guard.pml4_pa, 400), 0, "no private kstack window");
        assert_eq!(guard.kstack_top, crate::kstack::top_va(base));
        // fd 0/1/2 preopened as devcons chans.
        let node = LockNode::new();
        let g = guard.fd_group.lock(&node);
        assert!(g.fds[0].is_some() && g.fds[1].is_some() && g.fds[2].is_some());
        assert!(g.fds[3].is_none());
        // 18-02 (AUTH-03): the user seeds glenda at construction (the
        // donor's first-proc `kstrdup(&p->user, eve)` re-keyed to the
        // sane-default era) — the #c/user serve stays byte-identical to
        // the const era for every default boot.
        assert_eq!(guard.user, b"glenda".to_vec(), "the user seeds glenda at construction");
    }

    /// 18-02 (AUTH-03): the fork copy — the child's user is the
    /// parent's user at rfork time (donor newproc's
    /// `kstrdup(p->user, up->user)`), with no rfork flag governing it.
    /// A mapping-free parent exercises the copy site through the same
    /// host seam the address-space pin rides.
    #[test]
    fn rfork_copies_the_parent_user() {
        let mut parent = new_user_proc("forkuser").expect("construction succeeds");
        parent.user = b"norbert".to_vec();
        let frame = crate::dat::Ureg::zeroed();
        let child_pid =
            rfork_clone(&mut parent, &frame, RFPROC).expect("rfork_clone copies the user");
        let node = LockNode::new();
        let mut table = PROC_TABLE.lock(&node);
        let child = table
            .procs
            .iter_mut()
            .flatten()
            .find(|p| p.pid == child_pid)
            .expect("the child is table-resident");
        assert_eq!(child.user, b"norbert".to_vec(), "the child inherits the parent's user");
        // The parent's user is untouched by the copy (no aliasing).
        assert_eq!(parent.user, b"norbert".to_vec());
        // Retire the test child so it never counts as a live proc for
        // any later pin in this binary.
        child.state = ProcState::Moribund;
    }

    /// An image for the RFMEM sweep tests: mock_segments' data segment
    /// (0x100 bytes, ending mid-page) as a pattern at the data file base,
    /// zeros elsewhere.
    fn sweep_image() -> ByteSource {
        let data_file_base = 0x40u64;
        let mut file = alloc::vec![0u8; 0x400];
        for i in 0..0x100usize {
            file[data_file_base as usize + i] = (0xA0 ^ i) as u8;
        }
        ByteSource::new(ByteBacking::Mnt { file: Arc::new(file) }, data_file_base)
    }

    /// RFMEM leaves the whole data segment and bss shared after the fork:
    /// a data page the parent never touched is read from the image and
    /// shared, its bytes intact; a bss page is born a shared zero page; a
    /// page the parent has is shared as it is.  Until 2026-11-28 the sweep
    /// zero-filled the first of those (the page the data segment ends in):
    /// a program's statically initialised data came back as zeros after
    /// its first shared-memory fork.
    #[test]
    fn rfork_mem_shares_the_data_and_bss_pages_the_parent_never_touched() {
        let base = 0x1000_0000usize;
        let mut parent = new_user_proc("sweeptest").expect("construction succeeds");
        parent.segments = Some(mock_segments());
        parent.text_source = Some(sweep_image());
        // One bss page present before the fork, as a program that had used
        // its bss would have it.
        let had = parent.map_user_page(base + 0x3000, SegKind::Bss).expect("a bss page");
        let frame = crate::dat::Ureg::zeroed();
        let child_pid =
            rfork_clone(&mut parent, &frame, RFPROC | RFMEM | RFNOWAIT).expect("rfork_clone");
        let node = LockNode::new();
        let mut table = PROC_TABLE.lock(&node);
        let child = table
            .procs
            .iter_mut()
            .flatten()
            .find(|p| p.pid == child_pid)
            .expect("the child is table-resident");
        // One frame in both tables for every page of the data segment and
        // bss: the data page (its end mid-page), the bss page the parent
        // had, and the bss page it lacked.
        for (va, kind) in
            [(base + 0x2000, SegKind::Data), (base + 0x3000, SegKind::Bss), (base + 0x4000, SegKind::Bss)]
        {
            let prec = parent.mapping_log.get(va).unwrap_or_else(|| panic!("the parent has {va:#x}"));
            let crec = child.mapping_log.get(va).unwrap_or_else(|| panic!("the child has {va:#x}"));
            assert_eq!(prec.kind, kind, "{va:#x}");
            assert_eq!((crec.phys, crec.kind), (prec.phys, kind), "one frame, two tables: {va:#x}");
        }
        assert_eq!(parent.mapping_log.get(base + 0x3000).map(|r| r.phys), Some(had as usize));
        assert!(parent.mapping_log.get(base + 0x5000).is_none(), "past bss, nothing");
        // SAFETY: the record names the seam page the sweep mapped for this
        // test's parent; exactly one page is read.
        let bytes = unsafe {
            core::slice::from_raw_parts(
                phys_ptr_mut(parent.mapping_log.get(base + 0x2000).expect("the data page").phys as u64),
                PAGE_SIZE_4K,
            )
        };
        for i in 0..0x100usize {
            assert_eq!(bytes[i], (0xA0 ^ i) as u8, "data byte {i} survived the fork");
        }
        assert!(bytes[0x100..].iter().all(|&b| b == 0), "the bss head of that page is zero");
        // Retire the test child so it never counts as a live proc.
        child.state = ProcState::Moribund;
    }

    /// Without RFMEM nothing is materialised: the fork copies what the
    /// parent has, and a page the parent never touched stays absent in
    /// both.
    #[test]
    fn a_plain_fork_materialises_nothing() {
        let base = 0x1000_0000usize;
        let mut parent = new_user_proc("plainfork").expect("construction succeeds");
        parent.segments = Some(mock_segments());
        parent.text_source = Some(sweep_image());
        parent.map_user_page(base + 0x3000, SegKind::Bss).expect("a bss page");
        let frame = crate::dat::Ureg::zeroed();
        let child_pid = rfork_clone(&mut parent, &frame, RFPROC | RFNOWAIT).expect("rfork_clone");
        assert!(parent.mapping_log.get(base + 0x2000).is_none());
        assert!(parent.mapping_log.get(base + 0x4000).is_none());
        let node = LockNode::new();
        let mut table = PROC_TABLE.lock(&node);
        let child = table
            .procs
            .iter_mut()
            .flatten()
            .find(|p| p.pid == child_pid)
            .expect("the child is table-resident");
        assert_eq!(child.mapping_log.len(), 1, "the one page the parent had, copied");
        assert!(child.mapping_log.get(base + 0x2000).is_none());
        child.state = ProcState::Moribund;
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

    /// The user-half table pages reachable from `pml4_pa` (PDPT, PD and
    /// PT pages under PML4[0..256)), in walk order.
    fn reachable_user_tables(pml4_pa: u64) -> Vec<u64> {
        let mask = 0x000f_ffff_ffff_f000;
        let mut out = Vec::new();
        for i in 0..256 {
            let e4 = table_entry(pml4_pa, i);
            if e4 & PTE_P == 0 {
                continue;
            }
            let pdpt = e4 & mask;
            out.push(pdpt);
            for j in 0..512 {
                let e3 = table_entry(pdpt, j);
                if e3 & PTE_P == 0 {
                    continue;
                }
                let pd = e3 & mask;
                out.push(pd);
                for k in 0..512 {
                    let e2 = table_entry(pd, k);
                    if e2 & PTE_P != 0 {
                        out.push(e2 & mask);
                    }
                }
            }
        }
        out
    }

    /// The exec-reuse pin: EXEC frees the old image's mappings and then
    /// maps the new image into the SAME PML4.  free_user_mappings
    /// returns the user-half intermediate tables to the allocator, so it
    /// must also unlink them from the surviving PML4 first.  With the
    /// PML4 entries left behind, the new image's walk descends into the
    /// freed pages while the allocator hands those same pages out again
    /// as leaf pages: text and data bytes then land in live page tables.
    /// On the q35 venue that was a garbage PTE (the KZERO-window panic
    /// at phys_ptr_mut) or a PTE written over a text page (#UD at a
    /// 0xff8 page offset) once 8958c9e made frees real.
    #[test]
    fn free_user_mappings_unlinks_the_freed_tables_for_exec_reuse() {
        let mut guard = new_user_proc("execreuse").expect("construction succeeds");
        // The text base and the top stack page: PML4[0] and PML4[255],
        // the two user-half subtrees every exec'd image builds.
        let text_va = 0x0020_0000usize;
        let stack_va = 0x0000_7fff_ffff_e000usize;
        guard.map_user_page(text_va, SegKind::Text).expect("text map succeeds");
        guard.map_user_page(stack_va, SegKind::Stack).expect("stack map succeeds");
        assert_eq!(table_entry(guard.pml4_pa, 0) & PTE_P, PTE_P);
        assert_eq!(table_entry(guard.pml4_pa, 255) & PTE_P, PTE_P);
        let kstack_top = guard.kstack_top;

        guard.free_user_mappings();

        // Every user-half PML4 entry is gone: the freed tables are no
        // longer reachable from the root the next image reuses.  (Read
        // before any further walk: under the defect a walk here would
        // read freed pages.)
        for i in 0..256 {
            assert_eq!(
                table_entry(guard.pml4_pa, i),
                0,
                "PML4[{i}] still links a freed user-half table after free_user_mappings"
            );
        }
        assert!(guard.mapping_tables.is_empty());
        // The kernel side survives: the exit tail still runs on the
        // kstack — an arena slot, untouched by the user-half teardown.
        assert_eq!(guard.kstack_top, kstack_top);
        assert!(guard.kstack_pas.iter().all(|&pa| pa != 0));

        // The exec re-map builds fresh tables, and every table page the
        // root reaches is one this proc owns (recorded for its next
        // free), so no walk can land in a page the allocator handed out.
        guard.map_user_page(text_va, SegKind::Text).expect("text re-map succeeds");
        guard.map_user_page(stack_va, SegKind::Stack).expect("stack re-map succeeds");
        let reachable = reachable_user_tables(guard.pml4_pa);
        assert_eq!(reachable.len(), 6, "two subtrees of PDPT+PD+PT each");
        for t in &reachable {
            assert!(
                guard.mapping_tables.contains(t),
                "table page {t:#x} is reachable but not owned by the proc"
            );
        }
        guard.free_user_mappings();
        guard.state = ProcState::Moribund;
    }
}

/// The fd-group release and the bare-rfork split (release_fd_group,
/// split_fd_group_in_place) with real devpipe pipes; aarch64 carries the
/// same pins (Harvey 20-07's shared-table leak).  rfdprobe -s shape: a
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
}
