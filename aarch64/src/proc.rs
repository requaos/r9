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
//!   context; CURFRAME is read by trap.S at entry/return — same
//!   single-CPU invariant.  CURKSTACK is proc.rs's record of the
//!   per-process entry-stack invariant (the value SP_EL1 holds at every
//!   EL0 trap entry); the trap_return tail has reconstructed it as
//!   frame+288 since 05-01 (every frame sits at entry-stack top - 288),
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
/// Proc table size (05-06, Pitfall 3 + the 05-03 trace: the steady-state
/// set is boot→init + the persistent paqfs server + rc + a transient
/// command child, and rc pipelines multiply both procs and fds — sized
/// for the traced chain with headroom; exhaustion is LOUD, never silent).
pub const MAX_PROCS: usize = 16;
/// Open fd slots per proc (05-06, Pitfall 3: init's closefds closes
/// 3..29 — the table must hold fds through 29 without making that fatal;
/// pipe+dup flows hold several at once).
pub const MAX_FDS: usize = 32;
/// Per-process kernel stack: 16 KiB (4 pages) — the INTERRUPTSTACKSZ
/// discipline (a dedicated fixed-size kernel stack per trap-taking context).
const KSTACK_PAGES: usize = 4;
/// Queued-notes capacity (04-08 fills the delivery path).
pub const NOTE_QUEUE_CAP: usize = 8;
/// 05-09 (D-09): the exec-recorded argv's byte bound (the /proc/N/args
/// content — ps reads 256 bytes max, ps.c:63).  NUL-joined per arg.
pub const ARGV_CAP: usize = 256;

/// 05-08 (D-06): the note-group id allocator (the donor's
/// `Ref noteidalloc` — 9k proc.c:13; fresh id per newproc, inherited or
/// split per the rfork RFNOTEG rules).
static NEXT_NOTEID: AtomicU64 = AtomicU64::new(1);

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
    /// Classify a faulting user VA.  Only the text range is page-rounded at
    /// exec time; the data/bss ranges are byte-exact, so when
    /// `data % 4096 != 0 && bss > 0` the last data page and the head of bss
    /// share one 4K page.  That shared page must classify as Data (CR-01):
    /// classify by PAGE at the data/bss boundary, not by address — a
    /// bss-first fault would otherwise map the page zeroed and the data
    /// file bytes at [page_va, data_end) would never load (the page is then
    /// mapped, so no later fault can repair it — silent userland data
    /// corruption).  The Data page-in's zero-fill + byte-exact content_end
    /// clamp (04-06 Rule 1) yields exactly the right bytes either order:
    /// file bytes below data_end, zeros above.  Empty segments (zero-size
    /// data/bss) classify nothing; the byte-exact ranges stay authoritative
    /// for validate_user_range.  The stack range IS the 16 MiB growth
    /// bound, so a Stack hit is always in-bounds.
    pub fn classify(&self, va: usize) -> Option<SegKind> {
        // Data's classification end, page-rounded into the bss head (CR-01)
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

/// The demand-page byte source's backing (05-07 adds the devmnt variant).
#[derive(Clone)]
pub enum ByteBacking {
    /// devroot binaries: the initrd image + the file's paqfs entry
    /// (demand-paged reads index the image by file offset — 04-04).
    Paq { dir: PaqDir, image: &'static [u8] },
    /// devmnt binaries (05-07, the /bin union crossing): the WHOLE file,
    /// read eagerly at exec through the 9P client — syscall context can
    /// block on RPCs; the trap-context pager NEVER can (a fault can't
    /// block on the server), so the bytes are kernel-owned from exec on.
    /// Arc-shared across rfork (04-07's source-handle clone).
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

    /// The backing file's total length (WR-06's truncated-binary check):
    /// the paqfs entry's recorded length / the staged buffer's length.
    pub fn file_len(&self) -> u64 {
        match self {
            ByteBacking::Paq { dir, .. } => u64::from(dir.length),
            ByteBacking::Mnt { file } => file.len() as u64,
        }
    }
}

/// The demand-page byte source for text/data: the backing (the paqfs image
/// entry for devroot binaries; the staged whole file for devmnt ones) plus
/// the data-segment file base (40 + textsize — Pitfall 5: data file offsets are NOT page-aligned).
/// Clone for 04-07's rfork (the child mirrors the parent's source handles;
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
    /// fd group — 05-06 (D-03's fd half): the slots live in a SHAREABLE
    /// group object (the donor's Fgrp; fdgrp.rs) behind the mcslock
    /// idiom, so rfork's fd semantics are real per fork(2)/sysproc.c:
    /// RFPROC with NEITHER RFFDG nor RFCFDG SHARES this handle (Arc
    /// clone); RFFDG gives the child a fresh table COPIED slot-by-slot
    /// (pipe chans clone their Arc — the same queue); RFCFDG a fresh
    /// EMPTY table.  (The 05-06 plan's bullet inverts RFFDG vs default;
    /// the donor governs — the 05-04/05-05 inversion precedent.  libc
    /// fork() = rfork(RFPROC|RFFDG|RFREND) = COPY, which connectembed's
    /// dup/close choreography requires.)  fd 0/1/2 are devcons chans,
    /// preopened at construction (D-15, Plan 9 convention).  The group
    /// Lock is a LEAF (syscall context only, briefly; never from IRQ
    /// context, never held across a blocking pipe op — the checked-out
    /// chan pattern, fdgrp.rs).  261104-b4p (item 4 — the 05-06 exit-sweep
    /// deferral RESOLVED): exit_tail detaches the dying proc's group and,
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
    /// wait record) — AWAIT's block-vs-"no children" pivot.  RFNOWAIT
    /// children are excluded (WR-03): they post no record and are
    /// invisible to AWAIT, donor-style.
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
    /// 05-04 (D-03): the namespace group handle (the mount table; pgrp.rs).
    /// rfork flag semantics per fork(2): RFPROC with NEITHER RFNAMEG nor
    /// RFCNAMEG shares this handle (Arc clone); RFNAMEG deep-copies
    /// (pgrpcpy); RFCNAMEG starts clean (the 05-03 finding's newns wipe).
    /// The Lock serializes bind/unmount against namec walks; taken only
    /// from syscall context, briefly, never from IRQ context.
    pub pgrp: Arc<Lock<Pgrp>>,
    /// 05-05 (D-07): the environment group handle (devenv.rs).  rfork
    /// flag semantics per fork(2) + 9k sysproc.c: RFPROC with NEITHER
    /// RFENVG nor RFCENVG SHARES this handle (Arc clone); RFENVG
    /// deep-copies (envcpy); RFCENVG starts EMPTY.  (The plan's truth
    /// bullet inverted the two flags; fork(2)/sysproc.c govern — the
    /// 05-04 RFNAMEG/RFCNAMEG precedent; recorded as a 05-05 deviation.)
    /// The Lock serializes setenv/create against reads; a LEAF lock
    /// (syscall context only, never IRQ context).
    pub env_group: Arc<Lock<EnvGroup>>,
    /// The process root and current-directory chans (donor up->slash /
    /// up->dot).  exec_bootstrap sets both to the devroot attach before the
    /// first EL0 entry; rfork inherits (value-dup); exec preserves them
    /// (Plan 9 convention).  None only pre-exec.
    pub slash: Option<Chan>,
    pub dot: Option<Chan>,
    /// 05-08 (D-06): the note-group id — the donor's `noteid` (9k
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
    /// 05-08 (D-06): true while the proc is parked in a devcons read
    /// (set/cleared by the cons_block_current hook around sched()).  The
    /// tick's cons_input_wake readies cons-waiting Blocked procs when the
    /// PL011 RX ring holds bytes; the reader's post-wake drain re-checks
    /// (a spurious wake re-blocks).
    pub cons_waiting: bool,
    /// 05-08 (trace-gated: sleep ×42, alarm ×1 in the 05-03 real-chain
    /// trace): sys_sleep's wake deadline in TICKS (0 = not sleeping) —
    /// the donor tsleep's up->twhen shape; the tick's tick_timeouts
    /// readies expired sleepers.  1 tick = 1 ms this era (timer.rs
    /// TICKS_PER_SEC = 1000).
    pub sleep_until: u64,
    /// 05-08: sys_alarm's expiry deadline in TICKS (0 = disarmed — the
    /// donor's up->alarm; alarm(0) clears, a fresh arm replaces).  On
    /// expiry tick_timeouts posts the "alarm" note (donor alarmkproc's
    /// postnote arm) and disarms.
    pub alarm_at: u64,
    /// 05-09 (D-09): the exec-recorded argv — NUL-joined, bounded at
    /// ARGV_CAP (the donor's `p->args`; the truncation rule is
    /// documented on record_argv).  exec.rs's install_image fills it at
    /// EVERY exec (the bootstrap exec_into and exec_replace paths share
    /// install_image), replacing the prior image's record; a proc that
    /// never exec'd has an EMPTY record, so devproc's /proc/N/args reads
    /// EOF, never garbage (ps.c's Noargs arm).  rfork does NOT inherit
    /// it (a fork child's args are empty until its own exec — donor
    /// newproc shape).
    pub argv: Vec<u8>,
    /// 05-09 (D-09): the recorded scheduling priority (devproc ctl's
    /// "pri N" — accepting + storing is the honest degradation the plan
    /// names: no scheduler priority exists this era, so nothing READS
    /// it but the /proc/N/status report).  rfork INHERITS it (donor
    /// forkchild); exec preserves it (Plan 9 convention).  0 until set.
    pub pri: u32,
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
            parent: ptr::null_mut(),
            children_live: 0,
            nowait: false,
            wait_records: VecDeque::new(),
            errstr: [0; ERRMAX],
            pgrp: Arc::new(Lock::new("pgrp", Pgrp::new())),
            // Fresh EMPTY env group at construction (05-05); the first
            // proc's SEEDED root group is assigned by exec_bootstrap
            // (the slash/dot seeding's twin), and rfork derives every
            // later group from its parent.
            env_group: Arc::new(Lock::new("env", EnvGroup::new())),
            slash: None,
            dot: None,
            // 05-08: a fresh note group per proc (donor newproc's
            // incref(&noteidalloc)); rfork may overwrite with the parent's.
            noteid: NEXT_NOTEID.fetch_add(1, Ordering::Relaxed),
            cons_waiting: false,
            sleep_until: 0,
            alarm_at: 0,
            argv: Vec::new(),
            pri: 0,
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

    /// 05-09 (D-09): record the exec argv (NUL-joined, the donor's
    /// `p->args` shape) for devproc's /proc/N/args.  Called by exec.rs's
    /// install_image at EVERY exec (both paths share it), replacing the
    /// prior image's record.  Truncation rule (documented per the plan):
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

    /// D-15 preopen: fd 0/1/2 are devcons chans (Plan 9 convention — 0
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
    // fd ops (05-06) — the checked-out-chan pattern (fdgrp.rs's header):
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

/// 261103-p64: the current-proc pointer is an AtomicPtr — the
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

/// 261103-p64 (the current-proc accessor): the ONE guarded entry point for
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

/// True while any non-Moribund proc exists (04-06's EXITS path: when none
/// remain, the `all user processes exited` marker and the clean halt
/// follow — the single-process era's end-of-userspace).
pub fn any_active_procs() -> bool {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    table.procs.iter().flatten().any(|p| p.state != ProcState::Moribund)
}

/// WR-05: the shared mid-construction unwind for new_user_proc — free the
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
/// log; fd 0/1/2 are the D-15 devcons preopen (filled at construction).
///
/// 261103-p64: returns a ProcGuard, never a `&'static mut Proc`.
pub fn new_user_proc(name: &str) -> Result<ProcGuard> {
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
    // about backing pages, and 04-06's EXITS path frees by PA.  WR-05:
    // every mid-construction failure funnels through unwind_new_proc
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
        // WR-02: the table-full path must free the already-allocated
        // root-table page and kstack pages — they are otherwise leaked
        // (reachable with MAX_PROCS = 8 once real init spawns
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

/// A guard handle to a PROC_TABLE-resident Proc (261103-p64 —
/// new_user_proc no longer hands out a `&'static mut`).  Deref/DerefMut
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

/// The trap-entry stack top / live frame pointer.  CURFRAME is written by
/// trap.S at every entry and read at every return (the reload guard
/// against a mid-trap preemption swap).  CURKSTACK records the invariant
/// SP_EL1 must hold at the next EL0 trap entry (the current proc's
/// kernel-stack top); the return tail reproduces it as frame+288 (05-01 —
/// the old explicit CURKSTACK load in trap.S needed a post-pops scratch
/// and clobbered the restored userland x9 on every EL0t return: the D-14
/// note-timeout/wild-write flake).  CURKSTACK initialises to the legacy
/// interrupt stack so pre-process traps (early boot, virtpanic) behave
/// exactly as pre-04-07.  Write discipline per the module header.
///
/// 261103-p64 (T-p64-05): SyncUnsafeCell<u64> is repr(transparent) over
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

/// The live trap frame the trap_return tail will restore (04-08's
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
/// entry-stack record; 05-01: proc.rs's invariant bookkeeping — the
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
}

/// The timer-tick preemption entry (D-13).  Acts only when the current
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
/// evidence per D-13.  The trap tail's CURFRAME reload then returns to the
/// NEW process.  Ready procs holding a saved kernel context
/// (blocked_in_kernel) are skipped here — only the sched loop can swtch
/// them back in.
pub fn reschedule() {
    // 261103-p64: the guard's None arm replaces a bare deref (a null
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
        let next = table.procs[ni].as_mut().unwrap();
        next.state = ProcState::Running;
        next_pid = next.pid;
        set_current_proc(next);
        write_curkstack(next.kstack_top);
        write_curframe(next.frame);
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
/// sched_loop on the legacy interrupt stack (free this era: 04-05's
/// no-switch entry left it reserved for reporting only).  Seeded on first
/// use; accessed only from IRQ-masked single-CPU context.  261103-p64:
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
/// (04-07; the AWAIT path).  Returns when the sched loop swtch'es back —
/// i.e. after the proc was woken (Blocked → Ready by an EXITS posting a
/// wait record) and picked.  EL0 entry/exit never rides this path.
pub fn sched() {
    let mut cur = current().expect("sched with no current process");
    cur.state = ProcState::Blocked;
    cur.blocked_in_kernel = true;
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
    // gatedbg (05-REVIEW-FIX wedge hunt): consecutive idle iterations with
    // a blocked cons reader while RX bytes pend, and with ALL live procs
    // blocked.  The idle arm runs between proc contexts (no syscall holds
    // Console across a sched boundary), so dumping from here follows the
    // PROC_TABLE → Console nesting order.
    // 261103-p64 (Rule 1 — pre-existing break): the cons-rx dump arms are
    // cfg(dev_exec) like the pl011 cons_rx_* fns they call (d8496d3 left
    // them unconditional, breaking every non-dev_exec profile build).
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
                // 261103-p64: the consolidated idle-wait primitive
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
fn dump_proc_states() {
    let node = LockNode::new();
    let table = PROC_TABLE.lock(&node);
    for p in table.procs.iter().flatten() {
        if p.state == ProcState::Moribund {
            continue;
        }
        let n = p.name.iter().position(|&b| b == 0).unwrap_or(KNAMELEN);
        let name = core::str::from_utf8(&p.name[..n]).unwrap_or("?");
        println!(
            "wedgedbg: pid={} state={} cons_wait={} blocked_in_kernel={} sleep_until={} name={}",
            p.pid,
            state_name(p.state),
            p.cons_waiting,
            p.blocked_in_kernel,
            p.sleep_until,
            name
        );
    }
}

/// Round-robin pick of the next Ready proc after the current one, applying
/// the full switch as one PROC_TABLE-locked step (the plan's ordering
/// assignment: state, current-proc pointer, CURKSTACK, CURFRAME, TTBR0) so
/// the resumed proc's trap-return tail reloads its OWN frame, never the
/// previous proc's.  None when no other Ready proc exists.
///
/// 05-08: the current proc is NOT skipped — only its scan PRIORITY wraps
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

/// EXITS's tail (04-07): the Moribund current proc never runs again —
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
            swtch::swtch(&mut cur.context, &*ctx);
        }
    }
    unreachable!("sched: moribund process resumed");
}

/// Non-RFPROC rfork's namespace-group half (05-04, D-03; 9k sysproc.c's
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

/// Non-RFPROC rfork's env-group half (05-05, D-07; 9k sysproc.c's
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

/// devenv's current-group hook (05-05, D-07): the CURRENT proc's
/// env-group handle — the donor devenv.c envgrp()'s `up->egrp` shape
/// (every #e op resolves against the OPERATING process's group, never a
/// group captured at attach).  None before the first proc exists (no #e
/// op can happen then).  Reads the current-proc raw pointer under the
/// module header's single-CPU/IRQ-masked invariant (syscall context).
pub fn current_env_group() -> Option<Arc<Lock<EnvGroup>>> {
    Some(current()?.env_group.clone())
}

/// devpipe's block/wake hooks (05-06, D-04's connectembed substrate —
/// the port crate cannot see PROC_TABLE, so these are injected at boot:
/// the devproc.rs set_post_hook idiom).
///
/// The calling proc's pid (waiter registration).  Syscall context only —
/// a pipe op always has a current proc.
pub fn pipe_current_pid() -> u64 {
    current().expect("devpipe op with no current process").pid
}

/// Park the calling proc mid-syscall until a pipe write/close wakes it —
/// the 04-07 AWAIT shape (mark Blocked + swtch to the scheduler context;
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
    push_note(p, msg)
}

// ---------------------------------------------------------------------------
// 05-09 (D-09): devproc's proc-table hooks — the ps(1) data source
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

/// devproc's root-listing hook (05-09): the LIVE (non-Moribund) pids,
/// ascending by table slot (ps's dirreadall content).  Moribund slots
/// stay pinned this era (the 05-06 exit-sweep deferral) but are dead —
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

/// devproc's per-pid info hook (05-09): the status/args data — name,
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

/// devproc's ctl hook (05-09): record the "pri N" write on the ADDRESSED
/// pid (the write-side per-pid resolution, T-05-17 — no broadcast).
/// Accepting + storing is the plan's honest degradation: no scheduler
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

/// The note-queue append shared by post_note (per-pid) and the 05-08
/// note-group post: Moribund targets and full queues are skipped per the
/// donor's postnote failure shape (the caller's group post ignores
/// individual failures — pgrpnote's waserror skip).
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

/// 05-08 (D-06): devcons's DEL path — post a note to the note GROUPS of
/// the given reader pids (the console's foreground group this era — the
/// plan's sanctioned scoping for the donor's notepg machinery: "the note
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

/// The calling proc's pid (devcons's `current` hook, 05-08 — the generic
/// twin of pipe_current_pid, which stays named for devpipe's registration
/// site).  Syscall context only — a device op always has a current proc.
pub fn current_pid() -> u64 {
    current().expect("device op with no current process").pid
}

/// devcons's block hook (05-08, D-06): park the calling proc in its cons
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

/// The tick's cons wake (05-08): ready every cons-waiting Blocked proc —
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

/// Non-RFPROC rfork(RFNOTEG)'s note-group half (05-08; 9k sysproc.c:82 —
/// `if(flag & RFNOTEG) up->noteid = incref(&noteidalloc)`): the CALLER
/// joins a fresh note group.  init's fexec child does this before exec'ing
/// rc, so a console DEL interrupts the shell's group and never init.
pub fn rfork_noteg_split(proc: &mut Proc) {
    proc.noteid = NEXT_NOTEID.fetch_add(1, Ordering::Relaxed);
}

/// The tick's timeout sweep (05-08; the donor's tsleep wakeup + alarm.c's
/// alarmkproc expiry arm; 261104-b4p item 6's note-interruptible sleep):
/// per proc, the ALARM expiry arm runs FIRST (posting the "alarm" note
/// and disarming — the donor's `rp->alarm = 0`), THEN the sleep wake arm
/// readies a Blocked sleeper whose deadline passed OR whose note_queue is
/// non-empty (a posted note interrupts the sleep — the donor tsleep
/// shape; the sleeper's sys_sleep discriminates early-wake-vs-expiry
/// against its local deadline copy, so this arm's alarm-first ordering
/// makes an alarm fired at the same tick interrupt a sleep that would
/// otherwise expire).  sleep_until clears either way.  Notes append via
/// push_note (Moribund/full-queue skips are the donor pgrpnote's
/// waserror shape); delivery rides the 04-08 return-to-EL0 boundary
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

/// devdup/devsrv's fd resolver hook (05-06): the CALLING proc's fd N as
/// a reference clone (the donor fdtochan incref shape — a pipe end's
/// open count bumps, so the resolved chan is a real second reference the
/// close dispatch balances).  None before the first proc exists.
pub fn current_fd_resolve(fd: u64) -> Option<Chan> {
    current()?.fd_ref_clone(fd)
}

/// devdup's listing hook (05-06): the CALLING proc's open fds as
/// (fd, mode) pairs (the `#d` root directory read).
pub fn current_fd_list() -> Vec<(u64, u32)> {
    let Some(p) = current() else {
        return Vec::new();
    };
    let node = LockNode::new();
    p.fd_group.lock(&node).list_open()
}

/// EXITS (04-07): post the child's wait record to the parent and wake it
/// if it sits Blocked in AWAIT (Blocked → Ready; blocked_in_kernel stays
/// set — the parent resumes through the sched loop's swtch).  The record
/// is kernel memory in the parent's table slot (T-04-15: no cross-process
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

/// RFORK (04-07): clone `parent` — a fresh proc (new kernel stack + root
/// table via new_user_proc), an EAGER page-for-page copy of every user
/// mapping (no COW, no shared writable pages — T-04-15), a dup'd fd table
/// (fresh Chan values naming the same files/devices), mirrored segment
/// records + paqfs byte source, and a copy of the parent's live trap frame
/// on the child's kstack with x0 = 0 (donor forkchild).  The child is
/// queued Ready; its first run erets through the CURFRAME/trap_return
/// path.  Returns the child pid (the parent's wire result).
///
/// 05-04 (D-03): the namespace group + slash/dot inheritance per fork(2)
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

    // WR-02: everything from here is fallible, and the child is already
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
        // the free_root_table_and_kstack convention.
        for rec in core::mem::take(&mut child.mapping_log) {
            if let Err(err) = pagealloc::deallocate_physpage(PhysAddr::new(rec.phys as u64)) {
                println!("rfork: unwind deallocate user page {:#x}: {err:?}", rec.phys);
            }
        }
        child.free_root_table_and_kstack();
        return Err(msg);
    }
    // WR-03: an RFNOWAIT child posts no wait record at EXITS and is never
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

/// The fallible body of rfork_clone after the child exists (the WR-02
/// split — rfork_clone unwinds the half-built child on any Err): an EAGER
/// page-for-page copy of every user mapping, a dup'd fd table, mirrored
/// segment records + paqfs byte source, and a copy of the parent's live
/// trap frame on the child's kstack with x0 = 0 (donor forkchild).
fn rfork_copy(parent: &Proc, child: &mut Proc, frame: &TrapFrame, flag: u64) -> Result<()> {
    let nowait = flag & crate::syscall::RFNOWAIT != 0;
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
        // SAFETY: both pages' KZERO+PA aliases were just (re)mapped r/w in
        // the kernel table above; src names the parent's live page, dst
        // the child's fresh page — one full page copied, non-overlapping.
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
    // No icache maintenance on the copy: identical bytes land at identical
    // VAs (a stale VIPT line would return the same bytes), and any later
    // exec's text page-in flushes per page (fault.rs).  Documented limit.

    // Mirror the segment map + demand-page byte source so the child's
    // future faults resolve in the child.
    child.segments = parent.segments.clone();
    child.text_source = parent.text_source.clone();

    // 05-06 (D-03's fd half): the fd GROUP semantics per fork(2) +
    // 9k sysproc.c sysrfork — RFCFDG = fresh EMPTY table (dupfgrp(nil));
    // RFFDG = a fresh table COPIED slot-by-slot (dupfgrp — pipe chans
    // clone their Arc, so both tables' pipe fds reference the SAME
    // queue); NEITHER = SHARE the parent's handle (Arc clone).  (The
    // 05-06 plan's bullet inverts RFFDG vs default; fork(2) + sysrfork +
    // libc fork() = rfork(RFPROC|RFFDG|RFREND) = COPY govern — the
    // phase's third inversion, the 05-04/05-05 precedent.  The pre-05-06
    // code always dup'd — the RFFDG arm by another name; SHARE is the
    // new default.)
    child.fd_group = if flag & crate::syscall::RFCFDG != 0 {
        Arc::new(Lock::new("fdgrp", FdGroup::new()))
    } else if flag & crate::syscall::RFFDG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("fdgrp", parent.fd_group.lock(&node).deep_copy()))
    } else {
        parent.fd_group.clone()
    };

    // 05-04 (D-03): the namespace group + slash/dot, fork(2)/9k sysrfork
    // semantics.  RFNAMEG = fresh COPY of the parent's group (pgrpcpy);
    // RFCNAMEG = fresh EMPTY group (the newns wipe); NEITHER = share the
    // parent's handle (Arc clone).  (The plan's behavior bullet inverted
    // the two flags; fork(2) + sysrfork + the 05-03 trace's RFCNAMEG-wipe
    // finding govern — recorded as a 05-04 deviation.)
    child.pgrp = if flag & crate::syscall::RFCNAMEG != 0 {
        Arc::new(Lock::new("pgrp", Pgrp::new()))
    } else if flag & crate::syscall::RFNAMEG != 0 {
        let node = LockNode::new();
        Arc::new(Lock::new("pgrp", parent.pgrp.lock(&node).deep_copy()))
    } else {
        parent.pgrp.clone()
    };
    // 05-05 (D-07): the env group, fork(2)/9k sysrfork semantics — the
    // SAME shape as the namespace group above: RFENVG = fresh deep COPY
    // of the parent's group (envcpy); RFCENVG = fresh EMPTY group;
    // NEITHER = share the parent's handle (Arc clone).  (The plan's
    // truth bullet inverted RFENVG/RFCENVG; fork(2) + sysproc.c +
    // newns's rfork(RFENVG|RFCNAMEG) govern — the 05-04 RFNAMEG
    // precedent, recorded as a 05-05 deviation.)
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
    // 05-08 (D-06): the note group — 9k sysproc.c:183: WITHOUT RFNOTEG the
    // child INHERITS the parent's noteid; WITH it the child keeps the fresh
    // group id its construction assigned (the donor's newproc incref).
    if flag & crate::syscall::RFNOTEG == 0 {
        child.noteid = parent.noteid;
    }
    // 05-09 (D-09): the recorded ctl priority is inherited (donor
    // forkchild); the argv record is NOT — a fork child's args are empty
    // until its own exec (Proc::new's construction), so /proc/N/args for
    // a pre-exec child reads EMPTY rather than the parent's (the plan's
    // truthful-args contract).
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
