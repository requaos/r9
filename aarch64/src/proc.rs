//! Process scaffolding (R9-02/R9-03 groundwork; D-13's derived requirement):
//! per-process kernel stacks and root page tables, a small fixed proc table
//! behind the MCS-lock idiom, and the current-proc pointer trap-context code
//! reads.
//!
//! Nothing here *runs* a process yet — 04-04's exec.rs seeds the forkret
//! frame onto this scaffolding, fault.rs pages into its root table, 04-05
//! wires the boot_init call path, and 04-07 lands the scheduler (Blocked and
//! await arrive there).
//!
//! Locking discipline (04-PATTERNS.md "Locking discipline"):
//! - PROC_TABLE is taken only briefly in process/boot context — NEVER across
//!   the fault handler or a pagealloc call that could recurse.
//! - Trap-context code reads the current proc through `current_proc()`'s raw
//!   pointer, set at exec/switch time.  Invariant: single CPU, and trap
//!   handlers run IRQ-masked, so the pointer is stable for the handler's
//!   lifetime.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::ptr;

use port::Result;
use port::dat::Chan;
use port::mcslock::{Lock, LockNode};
use port::mem::{PAGE_SIZE_4K, PhysAddr, PhysRange, VirtRange};
use port::paqfs::PaqDir;
use port::println;

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
/// Proc table size (D-13 groundwork: a small fixed array; the 04-07
/// scheduler iterates it round-robin).
pub const MAX_PROCS: usize = 8;
/// Open fd slots per proc.
pub const MAX_FDS: usize = 16;
/// Per-process kernel stack: 16 KiB (4 pages) — the INTERRUPTSTACKSZ
/// discipline (a dedicated fixed-size kernel stack per trap-taking context).
const KSTACK_PAGES: usize = 4;
/// Queued-notes capacity (04-08 fills the delivery path).
pub const NOTE_QUEUE_CAP: usize = 8;

/// User segment kinds — the fault handler's classification and the mapping
/// log's tag (04-07's rfork-copy/exit-free walk the log by kind).
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

/// A proc's user VA map (D-11: r9 owns stack-top/growth policy; text/data
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
    /// Classify a faulting user VA.  Ranges are page-rounded at exec time;
    /// empty segments (zero-size data/bss) classify nothing.  The stack
    /// range IS the 16 MiB growth bound, so a Stack hit is always in-bounds.
    pub fn classify(&self, va: usize) -> Option<SegKind> {
        if self.text.0.contains(&va) {
            Some(SegKind::Text)
        } else if self.data.0.contains(&va) {
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

/// The demand-page byte source for text/data: the exec'd binary's paqfs
/// directory entry plus the image it reads from, and the data-segment file
/// base (40 + textsize — Pitfall 5: data file offsets are NOT page-aligned).
pub struct ByteSource {
    pub dir: PaqDir,
    pub image: &'static [u8],
    pub data_file_base: u64,
}

/// A queued note message (04-08 delivers at the return-to-EL0 boundary).
pub struct Note {
    pub buf: [u8; ERRMAX],
    pub len: usize,
}

/// Exit/wait record slot (04-07's exits/await path fills and consumes).
pub struct WaitRecord {
    pub pid: u64,
    pub msg: [u8; ERRMAX],
    pub msg_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcState {
    Ready,
    Running,
    Moribund,
    // Blocked arrives with 04-07's await.
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
    /// Physical pages backing the kernel stack (04-06's EXITS path frees
    /// them — the VAs come from the kernel-VA bump allocator and say nothing
    /// about their PAs, so construction records them here).
    pub kstack_pas: [u64; KSTACK_PAGES],
    /// Per-process root page table (one pagealloc'd page, recursive slot-511
    /// self-entry written at construction).  NEVER the vm.rs user-table
    /// static singleton — one shared table would give every process the same
    /// address space (04-RESEARCH.md anti-patterns).
    pub root_table: *mut RootPageTable,
    /// fd slots.  fd 0/1/2 are devcons chans, preopened at construction
    /// (D-15, Plan 9 convention) — 04-06 fills the seam 04-04 left.
    pub fd_table: [Option<Chan>; MAX_FDS],
    /// User mappings, exec/fault-appended — rfork-copy/exit-free walk this.
    pub mapping_log: Vec<MappingRecord>,
    /// The a.out-derived VA map; None until exec.
    pub segments: Option<Segments>,
    /// paqfs byte source for demand page-in; None until exec.
    pub text_source: Option<ByteSource>,
    /// The stashed trap frame on this proc's kernel stack (the forkret seed
    /// at exec; trap save/restore rides it from then on).
    pub frame: *mut TrapFrame,
    /// Kernel-context switch state (04-07's scheduler uses it).
    pub context: swtch::Context,
    pub note_queue: VecDeque<Note>,
    pub notify_handler: Option<u64>,
    pub wait_record: Option<WaitRecord>,
    pub errstr: [u8; ERRMAX],
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
            fd_table: core::array::from_fn(|_| None),
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
            wait_record: None,
            errstr: [0; ERRMAX],
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

    /// D-15 preopen: fd 0/1/2 are devcons chans (Plan 9 convention — 0
    /// OREAD, 1/2 OWRITE).  Called at the end of construction; the cons
    /// device itself needs no per-chan state (one console).
    fn preopen_cons_fds(&mut self) {
        use port::dat::{DEV_CONS, OREAD, OWRITE};
        // Qid path 1 = the cons file (one file this era; QTFILE).
        let cons_qid = port::dat::Qid { path: 1, vers: 0, typ: 0 };
        self.fd_table[0] = Some(Chan::new(DEV_CONS, cons_qid, OREAD));
        self.fd_table[1] = Some(Chan::new(DEV_CONS, cons_qid, OWRITE));
        self.fd_table[2] = Some(Chan::new(DEV_CONS, cons_qid, OWRITE));
    }

    /// The chan behind `fd`, or None for an out-of-range/closed slot (the
    /// syscall handlers map that to errstr + -1).
    pub fn fd(&mut self, fd: u64) -> Option<&mut Chan> {
        self.fd_table.get_mut(fd as usize)?.as_mut()
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

    /// T-04-11: validate a user-VA range against the proc's segment map
    /// BEFORE the kernel reads or writes it.  The range must lie wholly
    /// inside ONE of the proc's segments (text/data/bss/stack/tos).  A
    /// zero-length range is trivially valid (a null buffer with n=0 is not
    /// an error).  `writable` is accepted for the plan's signature and the
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

    /// 04-06 EXEC/EXITS: unmap and free every user mapping (the
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
            if let Err(err) =
                pagealloc::deallocate_physpage(PhysAddr::new(rec.phys as u64))
            {
                println!("proc: deallocate user page {:#x}: {err:?}", rec.phys);
            }
        }
        self.segments = None;
        self.text_source = None;
    }

    /// EXITS (04-06): free the root-table page and the kernel-stack pages
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

/// The process table — a small fixed array behind the mcslock idiom
/// (D-13 groundwork; the scheduler lands in 04-07).
pub struct ProcTable {
    procs: [Option<Proc>; MAX_PROCS],
    next_pid: u64,
}

pub static PROC_TABLE: Lock<ProcTable> =
    Lock::new("proc", ProcTable { procs: [const { None }; MAX_PROCS], next_pid: 1 });

static mut CURRENT_PROC: *mut Proc = ptr::null_mut();

/// The current process's raw pointer — set at exec/switch time, read in trap
/// context.  INVARIANT: single CPU; the pointer is written only in
/// process/boot context and read with IRQs masked, so it cannot change under
/// a reader.  Returns null before the first exec.
pub fn current_proc() -> *mut Proc {
    unsafe { ptr::addr_of!(CURRENT_PROC).read() }
}

/// Install the current-process pointer (the exec/switch path only).
pub(crate) fn set_current_proc(p: *mut Proc) {
    unsafe { ptr::addr_of_mut!(CURRENT_PROC).write(p) }
}

/// True while any non-Moribund proc exists (04-06's EXITS path: when none
/// remain, the `all user processes exited` marker and the clean halt
/// follow — the single-process era's end-of-userspace).
pub fn any_active_procs() -> bool {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table.procs.iter().flatten().any(|p| p.state != ProcState::Moribund)
}

/// Allocate and register a fresh user process: pid, 16 KiB kernel stack, and
/// a per-process root page table with the recursive slot-511 self-entry.
/// The proc starts with no segments, no byte source, and an empty mapping
/// log; fd 0/1/2 are the D-15 devcons preopen (filled at construction).
pub fn new_user_proc(name: &str) -> Result<&'static mut Proc> {
    // Allocate the per-process root page table first (BEFORE taking the
    // table lock — the lock is never held across a pagealloc call): one
    // pagealloc'd page, mapped at KZERO+PA in the kernel table (the 04-02
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
    unsafe {
        ptr::write_bytes(root_ptr, 0, 1);
        vm::init_empty_root_page_table(&mut *root_ptr);
    }

    // Kernel stack: KSTACK_PAGES contiguous kernel VAs from the kernel-VA
    // bump allocator, each backed by a fresh pagealloc page.  Nothing
    // interleaves between the bump calls in the single-threaded boot/exec
    // context; a gap would silently break the stack, so assert contiguity.
    // The PAs are recorded (kstack_pas) — the bump-allocator VAs say nothing
    // about backing pages, and 04-06's EXITS path frees by PA.
    let mut kstack_base = 0usize;
    let mut kstack_pas = [0u64; KSTACK_PAGES];
    for (i, pa_slot) in kstack_pas.iter_mut().enumerate() {
        let va_mapping = vm::next_free_device_page4k();
        let va = va_mapping.map(PhysAddr::new(0));
        if i == 0 {
            kstack_base = va;
        } else if va != kstack_base + i * PAGE_SIZE_4K {
            return Err("proc: kernel stack VAs not contiguous");
        }
        let pa = pagealloc::allocate_physpage()
            .map_err(|_| "proc: kernel stack page alloc failed")?;
        let mut physalloc = PhysPageAllocator {};
        let mut vmt = VmTraitImpl {};
        vm::kernel_pagetable()
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
            .map_err(|_| "proc: kernel stack map failed")?;
        *pa_slot = pa.addr();
    }
    let kstack_top = kstack_base + KSTACK_PAGES * PAGE_SIZE_4K;

    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let pid = table.next_pid;
    table.next_pid += 1;
    let Some(slot) = table.procs.iter_mut().find(|s| s.is_none()) else {
        return Err("proc: table full");
    };
    let mut proc = Proc::new(pid, name, kstack_top, kstack_pas, root_ptr);
    proc.preopen_cons_fds();
    *slot = Some(proc);
    // The table is a fixed static and slots are never freed this era, so the
    // reference stays valid after the guard drops (single CPU — see the
    // module header's locking discipline).
    Ok(unsafe { &mut *(slot.as_mut().unwrap() as *mut Proc) })
}
