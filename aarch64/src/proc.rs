//! Process scaffolding (R9-02/R9-03; D-13): per-process kernel stacks and
//! root page tables, a small fixed proc table behind the MCS-lock idiom,
//! the current-proc pointer trap-context code reads, and the 04-07
//! scheduler — tick-driven preemptive round-robin (preempt/reschedule)
//! plus the swtch-based blocking path (sched/sched_loop) that AWAIT rides.
//!
//! 04-04's exec.rs seeds the forkret frame onto this scaffolding, fault.rs
//! pages into its root table, 04-05 wired the boot_init call path, and
//! 04-06's syscall dispatch lands RFORK/AWAIT/EXITS-wait-records here in
//! 04-07.
//!
//! Locking discipline (04-PATTERNS.md "Locking discipline"):
//! - PROC_TABLE is taken only briefly in process/boot/trap context —
//!   NEVER across the fault handler or a pagealloc call that could
//!   recurse, and never across a swtch (the resumed proc may lock it).
//! - Trap-context code reads the current proc through `current_proc()`'s
//!   raw pointer, set at exec/switch time.  Invariant: single CPU, and
//!   trap handlers run IRQ-masked, so the pointer is stable for the
//!   handler's lifetime.
//! - CURKSTACK/CURFRAME (the trap.S entry-stack/live-frame symbols) are
//!   written only under the PROC_TABLE lock or in IRQ-masked trap
//!   context, and read by trap.S at entry/return — same single-CPU
//!   invariant.

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
/// Clone for 04-07's rfork (the child mirrors the parent's source handles).
#[derive(Clone)]
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
    /// Blocked mid-syscall (04-07's AWAIT) — holds a saved kernel context
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
    /// at exec; trap save/restore rides it from then on).  Read by the
    /// CURFRAME reload at trap return — written by the scheduler's locked
    /// swap and by reschedule's stash.
    pub frame: *mut TrapFrame,
    /// Kernel-context switch state — valid only while blocked_in_kernel
    /// (04-07's sched() saved it mid-syscall); the sched loop swtch'es it
    /// back in.  Preempted-at-EL0 procs carry no kernel context — they
    /// resume through the CURFRAME/trap_return path only.
    pub context: swtch::Context,
    /// True while `context` holds a live saved kernel context (the proc
    /// blocked mid-syscall).  The sched loop resumes such procs via swtch;
    /// every other Ready proc resumes via trap_return on `frame`.
    pub blocked_in_kernel: bool,
    /// The rfork parent (04-07) — EXITS posts the wait record here.  Null
    /// for the staged init (its exits posts nothing).
    pub parent: *mut Proc,
    /// rfork'd children not yet reaped by AWAIT (live or with a pending
    /// wait record) — AWAIT's block-vs-"no children" pivot.
    pub children_live: usize,
    /// rfork(RFNOWAIT) child: EXITS posts no wait record and wakes nobody.
    pub nowait: bool,
    pub note_queue: VecDeque<Note>,
    pub notify_handler: Option<u64>,
    /// The saved-Ureg user VA of the most recently delivered note (04-08;
    /// donor `up->noteureg`): set by trap.rs's deliver_notes, read and
    /// cleared by NOTED.  0 = not inside a note handler.
    pub note_ureg: u64,
    /// The last delivered note's bytes (04-08; donor `up->lastnote`) —
    /// NOTED's NDFLT arm exits with this message.
    pub last_note: [u8; ERRMAX],
    pub last_note_len: usize,
    /// Posted child-exit records (04-07; one per exited child, consumed by
    /// AWAIT in post order).
    pub wait_records: VecDeque<WaitRecord>,
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
            note_ureg: 0,
            last_note: [0; ERRMAX],
            last_note_len: 0,
            blocked_in_kernel: false,
            parent: ptr::null_mut(),
            children_live: 0,
            nowait: false,
            wait_records: VecDeque::new(),
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

// ---------------------------------------------------------------------------
// 04-07 scheduler (D-13): tick-driven preemptive round-robin over PROC_TABLE
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
// TICKS_PER_SEC — 100 Hz = 10 ms this era; CONTEXT.md agent's discretion).
// ---------------------------------------------------------------------------

/// The trap-entry stack top / live frame pointer, read by trap.S at every
/// entry (CURFRAME record) and return (CURFRAME reload; CURKSTACK load on
/// EL0t-bound returns).  CURKSTACK initialises to the legacy interrupt
/// stack so pre-process traps (early boot, virtpanic) behave exactly as
/// pre-04-07.  Write discipline per the module header.
#[unsafe(no_mangle)]
pub static mut CURKSTACK: u64 = 0;
#[unsafe(no_mangle)]
pub static mut CURFRAME: u64 = 0;

fn read_curframe() -> *mut TrapFrame {
    unsafe { ptr::addr_of!(CURFRAME).read() as *mut TrapFrame }
}

/// The live trap frame the trap_return tail will restore (04-08's
/// note-delivery boundary reads it): after a mid-trap preemption swap this
/// is the NEW current proc's frame, not the one the entry built — the
/// return-to-EL0 boundary must operate on the frame actually restored.
pub(crate) fn current_frame() -> *mut TrapFrame {
    read_curframe()
}

fn write_curframe(frame: *mut TrapFrame) {
    unsafe { ptr::addr_of_mut!(CURFRAME).write(frame as u64) }
}

fn write_curkstack(top: usize) {
    unsafe { ptr::addr_of_mut!(CURKSTACK).write(top as u64) }
}

/// The legacy interrupt stack's top (trap.S's reservation): the pre-process
/// CURKSTACK default AND the scheduler context's stack.  `interruptstacksz`
/// is an absolute (.equ) symbol — its "address" IS the size value.
fn interrupt_stack_top() -> usize {
    unsafe extern "C" {
        static interruptstackbase: u8;
        static interruptstacksz: u8;
    }
    unsafe { &interruptstackbase as *const u8 as usize + &interruptstacksz as *const u8 as usize }
}

/// Initialise CURKSTACK to the legacy interrupt stack.  main9 calls this
/// right after trap::init — before IRQs can unmask, so the first tick can
/// never read a null stack target.
pub fn init_trap_stacks() {
    write_curkstack(interrupt_stack_top());
}

/// First activation (exec_bootstrap's only call): publish the proc as
/// current + Running and point the trap-entry stack at its kstack — the
/// EL0t-bound trap_return tail loads SP_EL1 from CURKSTACK.
pub fn activate_first(p: &mut Proc) {
    p.state = ProcState::Running;
    set_current_proc(p);
    write_curkstack(p.kstack_top);
}

/// The timer-tick preemption entry (D-13).  Acts only when the current
/// proc is Running — i.e. the IRQ interrupted EL0 (kernel syscall/trap
/// context runs IRQ-masked; the sched loop's idle wfi finds current
/// Blocked/Moribund and its own post-wfi re-scan dispatches instead).
pub fn preempt() {
    let cur = current_proc();
    if cur.is_null() {
        return;
    }
    if unsafe { (*cur).state } != ProcState::Running {
        return;
    }
    reschedule();
}

/// Round-robin reschedule from the tick (IRQ-from-EL0: the live frame sits
/// on the current proc's kstack, recorded in CURFRAME by the trap entry).
/// On a switch: stash the live frame into the current proc, mark it Ready,
/// install the next (current + CURKSTACK + CURFRAME + TTBR0, one locked
/// step), and print the marker `sched: switch old->new` — marker-not-order
/// evidence per D-13.  The trap tail's CURFRAME reload then returns to the
/// NEW process.  Ready procs holding a saved kernel context
/// (blocked_in_kernel) are skipped here — only the sched loop can swtch
/// them back in.
pub fn reschedule() {
    let cur = current_proc();
    let cur_pid = unsafe { (*cur).pid };
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
        let next = table.procs[ni].as_mut().unwrap();
        next.state = ProcState::Running;
        next_pid = next.pid;
        set_current_proc(next);
        write_curkstack(next.kstack_top);
        write_curframe(next.frame);
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
/// sched_loop on the legacy interrupt stack (free this era: 04-05's
/// no-switch entry left it reserved for reporting only).  Seeded on first
/// use; accessed only from IRQ-masked single-CPU context.
static mut SCHED_CONTEXT: swtch::Context = swtch::Context {
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
};
static mut SCHED_SEEDED: bool = false;

fn sched_context() -> *mut swtch::Context {
    unsafe {
        if !ptr::addr_of!(SCHED_SEEDED).read() {
            let ctx = ptr::addr_of_mut!(SCHED_CONTEXT);
            (*ctx).set_return(sched_loop as *const () as u64);
            (*ctx).set_stack_pointer(interrupt_stack_top() as u64);
            ptr::addr_of_mut!(SCHED_SEEDED).write(true);
        }
        ptr::addr_of_mut!(SCHED_CONTEXT)
    }
}

/// Block the CURRENT proc mid-syscall and switch to the scheduler context
/// (04-07; the AWAIT path).  Returns when the sched loop swtch'es back —
/// i.e. after the proc was woken (Blocked → Ready by an EXITS posting a
/// wait record) and picked.  EL0 entry/exit never rides this path.
pub fn sched() {
    let cur = current_proc();
    assert!(!cur.is_null(), "sched with no current process");
    unsafe {
        (*cur).state = ProcState::Blocked;
        (*cur).blocked_in_kernel = true;
        swtch::swtch(&mut (*cur).context, &*sched_context());
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
    loop {
        match pick_next() {
            Some(Resume::Switch(ctx)) => unsafe { swtch::swtch(&mut *sched_context(), &*ctx) },
            Some(Resume::Return(frame)) => unsafe { crate::trap::trap_return(frame) },
            None => unsafe {
                core::arch::asm!("msr DAIFClr, #2", "wfi", "msr DAIFSet, #2");
            },
        }
    }
}

/// Round-robin pick of the next Ready proc after the current one, applying
/// the full switch as one PROC_TABLE-locked step (the plan's ordering
/// assignment: state, current-proc pointer, CURKSTACK, CURFRAME, TTBR0) so
/// the resumed proc's trap-return tail reloads its OWN frame, never the
/// previous proc's.  None when no other Ready proc exists.
fn pick_next() -> Option<Resume> {
    let cur = current_proc();
    let cur_pid = unsafe { cur.as_ref().map(|p| p.pid) };
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    let cur_idx = cur_pid
        .and_then(|pid| table.procs.iter().position(|s| s.as_ref().is_some_and(|p| p.pid == pid)));
    let start = cur_idx.map(|i| i + 1).unwrap_or(0);
    for off in 0..MAX_PROCS {
        let i = (start + off) % MAX_PROCS;
        if Some(i) == cur_idx {
            continue;
        }
        let Some(p) = table.procs[i].as_mut() else { continue };
        if p.state != ProcState::Ready {
            continue;
        }
        p.state = ProcState::Running;
        set_current_proc(p);
        write_curkstack(p.kstack_top);
        write_curframe(p.frame);
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

/// EXITS's tail (04-07): the Moribund current proc never runs again —
/// switch into the scheduler context with no wake path.  The swtch save
/// writes only the PROC_TABLE-resident context struct (never the freed
/// kstack), so EXITS can free the kstack before calling here.  Diverges.
pub fn sched_from_moribund() -> ! {
    let cur = current_proc();
    unsafe {
        let ctx = sched_context();
        if !cur.is_null() {
            swtch::swtch(&mut (*cur).context, &*ctx);
        }
    }
    unreachable!("sched: moribund process resumed");
}

/// devproc-lite's write hook (04-08, D-16/T-04-17): append the note to the
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

/// EXITS (04-07): post the child's wait record to the parent and wake it
/// if it sits Blocked in AWAIT (Blocked → Ready; blocked_in_kernel stays
/// set — the parent resumes through the sched loop's swtch).  The record
/// is kernel memory in the parent's table slot (T-04-15: no cross-process
/// pointers).  PROC_TABLE-brief.
pub fn post_wait_record(parent: *mut Proc, pid: u64, msg: &[u8]) {    if parent.is_null() {
        return;
    }
    let node = LockNode::new();
    // Held for the posting + wake (the parent pointer is table-resident).
    let _guard = PROC_TABLE.lock(&node);
    let parent = unsafe { &mut *parent };
    let mut rec = WaitRecord { pid, msg: [0; ERRMAX], msg_len: msg.len().min(ERRMAX) };
    rec.msg[..rec.msg_len].copy_from_slice(&msg[..rec.msg_len]);
    parent.wait_records.push_back(rec);
    if parent.state == ProcState::Blocked {
        parent.state = ProcState::Ready;
    }
}

/// RFORK (04-07): clone `parent` — a fresh proc (new kernel stack + root
/// table via new_user_proc), an EAGER page-for-page copy of every user
/// mapping (no COW, no shared writable pages — T-04-15), a dup'd fd table
/// (fresh Chan values naming the same files/devices), mirrored segment
/// records + paqfs byte source, and a copy of the parent's live trap frame
/// on the child's kstack with x0 = 0 (donor forkchild).  The child is
/// queued Ready; its first run erets through the CURFRAME/trap_return
/// path.  Returns the child pid (the parent's wire result).
pub fn rfork_clone(parent: &mut Proc, frame: &TrapFrame, nowait: bool) -> Result<u64> {
    let parent_ptr = ptr::addr_of_mut!(*parent);
    let name = {
        let n = parent.name.iter().position(|&b| b == 0).unwrap_or(KNAMELEN);
        core::str::from_utf8(&parent.name[..n]).unwrap_or("fork")
    };
    let child = new_user_proc(name)?;

    // The eager copy.  Each parent page's bytes go through the pages'
    // KZERO+PA kernel aliases (map_to REPLACES existing entries, so
    // re-mapping the exec/fault-side alias is idempotent); the child's
    // fresh page maps into its INACTIVE root table at the same VA with the
    // same entry kind (map_to's temp-recursive dance edits inactive
    // tables).  The parent's mapping_log is stable: single CPU, the parent
    // is current, IRQs are masked in syscall context.
    let records: Vec<MappingRecord> = parent.mapping_log.clone();
    for rec in &records {
        let pa = pagealloc::allocate_physpage().map_err(|_| "rfork: page alloc failed")?;
        map_kernel_alias(rec.phys as u64)?;
        map_kernel_alias(pa.addr())?;
        unsafe {
            let src = physaddr_as_ptr_mut_offset_from_kzero::<u8>(PhysAddr::new(rec.phys as u64));
            let dst = physaddr_as_ptr_mut_offset_from_kzero::<u8>(pa);
            ptr::copy_nonoverlapping(src, dst, PAGE_SIZE_4K);
        }
        // Text is UXN-clear/PXN-set; everything else rw_user_data — the
        // same kind→entry mapping fault.rs's page-in uses.
        let entry = match rec.kind {
            SegKind::Text => Entry::rw_user_text(),
            _ => Entry::rw_user_data(),
        }
        .with_phys_addr(pa);
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
    // No icache maintenance on the copy: identical bytes land at identical
    // VAs (a stale VIPT line would return the same bytes), and any later
    // exec's text page-in flushes per page (fault.rs).  Documented limit.

    // Mirror the segment map + demand-page byte source so the child's
    // future faults resolve in the child.
    child.segments = parent.segments.clone();
    child.text_source = parent.text_source.clone();

    // Dup the fd table (value copies — the streaming offsets diverge after
    // the fork, a documented approximation of Plan 9's shared-Chan
    // semantics; the smoke's fds are the devcons preopens).
    for (i, slot) in parent.fd_table.iter().enumerate() {
        if let Some(c) = slot {
            child.fd_table[i] = Some(c.dup());
        }
    }

    // The forkchild frame: a copy of the parent's live frame on the
    // CHILD's kernel stack, with x0 = 0 (the child's wire result).
    let child_frame = (child.kstack_top - core::mem::size_of::<TrapFrame>()) as *mut TrapFrame;
    unsafe {
        ptr::copy_nonoverlapping(frame, child_frame, 1);
        (*child_frame).x0 = 0;
    }
    child.frame = child_frame;
    child.parent = parent_ptr;
    child.nowait = nowait;
    // Donor newproc (9front@350266104f sys/src/9/port/sysproc.c:142-144):
    // the child INHERITS the notify handler; the note queue/last-note/
    // note_ureg start empty (Proc::new's construction).
    child.notify_handler = parent.notify_handler;
    parent.children_live += 1;
    Ok(child.pid)
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
