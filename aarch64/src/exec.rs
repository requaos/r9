//! Plan 9 a.out loader + exec image management (R9-03, D-09/D-11): parse
//! and validate the big-endian header, record the segment map, map the
//! data+bss pages EAGERLY (05-04 — see install_image), and seed exactly
//! TWO eager stack-side pages (the top-of-stack seed page and the zeroed
//! Tos page) + the forkret-shaped TrapFrame on the proc's kernel stack.
//! Text/stack/tos fault in on demand (D-09): text pages arrive via
//! fault.rs from the paqfs byte source; stack/tos faults map zero pages.
//!
//! 04-05 wired the first-exec runtime path (main9's exec_bootstrap →
//! exec_into).  04-06 adds exec_replace — the EXEC syscall's
//! replace-the-running-image path (parse first, free second, seed third,
//! then rewrite the live trap frame in place; T-04-13).

use core::ptr;

use port::Result;
use port::mem::{PAGE_SIZE_4K, PhysAddr, PhysRange, VirtRange};
use port::paqfs::PaqDir;
use port::println;

use crate::kmem::physaddr_as_ptr_mut_offset_from_kzero;
use crate::pagealloc;
use crate::param::KZERO;
use crate::proc::{ByteBacking, ByteSource, MappingRecord, Proc, ProcState, SegKind, Segments};
use crate::trap::TrapFrame;
use crate::vm::{self, Entry, PageSize, PhysPageAllocator, RootPageTableType, VmTraitImpl};

// --- r9 user address-space policy (D-11: r9 owns stack-top placement,
// segment/growth policy, and the initial stack frame; text/data VAs are
// fixed by 7l in the a.out header) ---
/// Top of the user stack.
pub const USTACKTOP: usize = 0x0000_8000_0000_0000;
/// Stack-growth bound (the donor's USTKSIZE reference value).
pub const USTKSIZE: usize = 16 * 1024 * 1024;
/// Ceiling for text/data/bss VAs.
pub const USTOP: usize = USTACKTOP - USTKSIZE;

// --- a.out format (Harvey sys/include/a.out.h; verified against
// build/aarch64/hello.7.out's bytes in 04-RESEARCH.md) ---
/// R_MAGIC arm64: _MAGIC(HDR_MAGIC, 28).
const R_MAGIC_AARCH64: u32 = 0x8C47;
/// HDR_MAGIC header size: 8×BE u32 + the BE u64 entry at bytes 32..39;
/// text follows at file offset 40.
const HDR_SIZE: usize = 40;
/// 7l's default text VA (INITTEXT = 0x10000 + HEADR, no -T flag).
pub const INITTEXT: usize = 0x10000 + HDR_SIZE;
/// The text mapping base: 7l includes the 40-byte header in the text
/// mapping, so text pages map from 0x10000 and the fault handler's text
/// file offset is pageVA - 0x10000.
pub const TEXT_MAP_BASE: usize = 0x10000;
/// Harvey sizeof(Tos) on aarch64 (sys/include/tos.h: 88 bytes) — the Tos
/// page is mapped zeroed; r9 never writes Tos fields this phase (Harvey
/// getpid reads #c/pid, not Tos — cheap insurance only).
const TOS_SIZE: usize = 88;
/// Cap on exec args — the initial stack frame must fit the seed page.
const MAX_ARGS: usize = 16;

/// A validated a.out header (all on-disk fields big-endian).
#[derive(Debug)]
pub struct AoutHeader {
    pub text: u64,
    pub data: u64,
    pub bss: u64,
    pub syms: u64,
    pub entry: u64,
    /// Text VA (INITTEXT); the mapping base is TEXT_MAP_BASE (header
    /// included).
    pub text_va: usize,
    /// Data VA — the 64K-aligned VA above text (7l's R_MAGIC alignment).
    pub data_va: usize,
}

fn be32(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], off: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(off..off + 8)?.try_into().ok()?))
}

/// Parse and validate an a.out header (T-04-08: the donor sysexec check set,
/// ported — every violation is a loud Err before any mapping exists).
pub fn parse_aout(bytes: &[u8]) -> Result<AoutHeader> {
    if bytes.len() < HDR_SIZE {
        return Err("exec: short a.out header");
    }
    let magic = be32(bytes, 0).ok_or("exec: short a.out header")?;
    if magic != R_MAGIC_AARCH64 {
        return Err("exec: bad a.out magic (not R_MAGIC arm64)");
    }
    // Bounds proven above: the header is 40 bytes.
    let text = be32(bytes, 4).unwrap() as u64;
    let data = be32(bytes, 8).unwrap() as u64;
    let bss = be32(bytes, 12).unwrap() as u64;
    let syms = be32(bytes, 16).unwrap() as u64;
    // The u32 entry field at 24 is vestigial on HDR_MAGIC arches; the real
    // entry point is the BE u64 at bytes 32..39.
    let entry = be64(bytes, 32).ok_or("exec: short a.out header")?;

    if text == 0 {
        return Err("exec: empty text segment");
    }
    let text_va = INITTEXT;
    if entry < text_va as u64 || entry >= text_va as u64 + text {
        return Err("exec: entry point outside text segment");
    }
    // Data VA: the 64K-aligned VA above text (donor align=0xffff for
    // R_MAGIC).  File offset is 40+textsize — NOT page-aligned (Pitfall 5).
    let data_va = (text_va + text as usize).next_multiple_of(0x10000);
    if data_va as u64 + data + bss >= USTOP as u64 {
        return Err("exec: text+data+bss exceeds USTOP");
    }
    Ok(AoutHeader { text, data, bss, syms, entry, text_va, data_va })
}

/// Read + parse + validate the 40-byte a.out header through the byte
/// backing's window — byte-granular, Pitfall-5 discipline (no
/// page-alignment assumptions on file offsets).  T-04-13: this runs BEFORE
/// any mapping or freeing in both exec paths — an invalid image is a loud
/// Err with everything intact.  05-07: the backing abstracts the byte
/// source (devroot's paqfs image window, or a devmnt binary's staged
/// whole-file buffer).
fn read_aout(backing: &ByteBacking) -> Result<AoutHeader> {
    let mut hdr = [0u8; HDR_SIZE];
    let n = backing.read_at(0, &mut hdr)?;
    if n < HDR_SIZE {
        return Err("exec: short read of a.out header");
    }
    let h = parse_aout(&hdr)?;
    // WR-06: the header's declared segment sizes must FIT the backing's
    // length — a truncated binary otherwise execs "successfully": the
    // pager's read_at clamps at EOF and page_in_from_image zero-fills
    // first, so the missing tail executes as zeros and the process dies
    // in a misleading place.  The donor fails the exec on a short
    // segment read; r9 fails it up front.  syms ride the image's length
    // semantics (a file claiming a symbol table it does not contain IS
    // truncated) even though exec never pages syms in.  (The sum cannot
    // overflow: four u32-derived u64s + 40.)
    let declared = HDR_SIZE as u64 + h.text + h.data + h.syms;
    if declared > backing.file_len() {
        return Err("exec: truncated binary");
    }
    Ok(h)
}

/// Record the segment map + the demand-page byte source + the proc name,
/// eagerly map data+bss, then seed the two eager stack-side pages.
/// Shared by first-exec (exec_into) and image replacement (exec_replace);
/// returns (user_sp, tos_va).
fn install_image(
    proc: &mut Proc,
    source: ByteSource,
    name: &str,
    args: &[&[u8]],
    aout: &AoutHeader,
) -> Result<(usize, usize)> {
    // Record the segment map (page-rounded for the fault classifier) and the
    // demand-page byte source.
    let text_end = (aout.text_va + aout.text as usize).next_multiple_of(PAGE_SIZE_4K);
    let data_end = aout.data_va + aout.data as usize;
    let bss_end = data_end + aout.bss as usize;
    proc.segments = Some(Segments {
        text: VirtRange::with_end(TEXT_MAP_BASE, text_end),
        data: VirtRange::with_end(aout.data_va, data_end),
        bss: VirtRange::with_end(data_end, bss_end),
        stack: VirtRange::with_end(USTACKTOP - USTKSIZE, USTACKTOP),
        tos: VirtRange::with_end(USTACKTOP - 2 * PAGE_SIZE_4K, USTACKTOP - PAGE_SIZE_4K),
    });
    proc.set_name(name);
    // 05-09 (D-09): record the argv for devproc's /proc/N/args (the
    // donor's p->args) — install_image is shared by exec_into (the
    // bootstrap path) and exec_replace (the EXEC syscall), so EVERY exec
    // records; the record replaces the prior image's.
    proc.record_argv(args);

    // 05-04 (the 05-04 tracer surfaced phase-04 deferred-items #3): map the
    // data+bss pages EAGERLY.  7c places string literals and statics in
    // data, and a real binary (boot(8)) passes them to syscalls without any
    // prior EL0 touch — Phase-4's measured binaries never showed this
    // because libc fmt read their buffers to format them ("always resident
    // by call time").  validate_user_range is a membership check and
    // T-05-06 forbids nested demand-paging from syscall context, so an
    // unread literal page aborted at EL1h (the kernel panic the 05-04
    // tracer's zero-panic gate forbids).  Eager data+bss residency is the
    // resolution that keeps BOTH rules: no nested paging anywhere, and
    // D-09's pinned wording ("faulting in user text and growing the user
    // stack") is untouched — text, stack and Tos stay demand-paged.
    map_data_bss_eager(proc, &source, aout)?;
    proc.text_source = Some(source);

    // SEED: the two eager stack-side pages (D-09/D-11) — the top-of-stack
    // seed page and the zeroed Tos page just below it.
    seed_user_pages(proc, args)
}

/// Eager data+bss mapping (05-04): zero-fill each page through its KZERO+PA
/// kernel alias (the proc's table is INACTIVE at exec — the alias is the
/// correct edit path, 04-02's convention, mirroring map_one_user_page),
/// then clamp-write the data segment's file content (byte-exact end — the
/// 04-06 Rule-1 clamp: the file's trailing syms bytes must never bleed
/// into the bss-sharing tail).  The pages are logged (SegKind::Data/Bss)
/// so rfork's copy and exit's free cover them like any other mapping.
/// 05-07: reads ride the proc's byte source (the backing abstraction —
/// paqfs image window or the staged devmnt buffer).
fn map_data_bss_eager(proc: &mut Proc, source: &ByteSource, aout: &AoutHeader) -> Result<()> {
    let data_end = aout.data_va + aout.data as usize;
    let bss_end = data_end + aout.bss as usize;
    let data_file_base = HDR_SIZE as u64 + aout.text;
    let mut page_va = aout.data_va;
    let data_pages_end = data_end.next_multiple_of(PAGE_SIZE_4K);
    while page_va < data_pages_end {
        let pa = map_one_user_page(proc, page_va, Entry::rw_user_data(), SegKind::Data)?;
        let clamp = data_end.saturating_sub(page_va).min(PAGE_SIZE_4K);
        if clamp > 0 {
            let file_off = data_file_base + (page_va - aout.data_va) as u64;
            let dst = unsafe {
                core::slice::from_raw_parts_mut(physaddr_as_ptr_mut_offset_from_kzero::<u8>(pa), clamp)
            };
            source.read_at(file_off, dst).map_err(|_| "exec: data page-in read failed")?;
        }
        page_va += PAGE_SIZE_4K;
    }
    let bss_pages_end = bss_end.next_multiple_of(PAGE_SIZE_4K);
    let mut page_va = data_pages_end;
    while page_va < bss_pages_end {
        map_one_user_page(proc, page_va, Entry::rw_user_data(), SegKind::Bss)?;
        page_va += PAGE_SIZE_4K;
    }
    Ok(())
}

/// First exec (the boot_init path; 04-05 wires the call): parse the binary's
/// header through the paqfs window, record segments + the byte source, seed
/// the two eager pages + the forkret frame, and leave the proc Ready.
///
/// `image` is the mapped paqfs initrd; `bin` is the binary's PaqDir inside
/// it.  D-09/D-11: text, stack and Tos pages fault in on demand; data+bss
/// are mapped eagerly (05-04 — the cold-syscall-argument fix).
pub fn exec_into(
    proc: &mut Proc,
    image: &'static [u8],
    bin: &PaqDir,
    name: &str,
    args: &[&[u8]],
) -> Result<()> {
    let backing = ByteBacking::Paq { dir: bin.clone(), image };
    let aout = read_aout(&backing)?;
    let source = ByteSource { backing, data_file_base: HDR_SIZE as u64 + aout.text };
    let (user_sp, tos_va) = install_image(proc, source, name, args, &aout)?;

    // The forkret-shaped frame at kstack_top - 288 (the published TrapFrame
    // stride): R0 = tos, SP = seeded user stack, PC = entry, PSR = 0 (EL0t),
    // LR = 0 — 04-05's boot path erets into EL0 from exactly this frame, and
    // the first instruction fetch faults into fault.rs's text page-in.
    let frame = seed_exec_frame(proc.kstack_top, aout.entry, user_sp as u64, tos_va as u64);
    proc.frame = frame;
    proc.state = ProcState::Ready;
    Ok(())
}

/// EXEC behind a running process (04-06's syscall path; Plan 9 sysexec
/// semantics): REPLACE the caller's image with the binary carried by
/// `backing` (05-07: the byte-source abstraction — devroot's paqfs image
/// entry, or a devmnt binary's staged whole-file buffer).
///
/// Ordering is the T-04-13 mitigation, verbatim from the plan: PARSE the new
/// image's a.out first (plus every other fallible pre-check), FREE the old
/// mappings second, SEED third — so a validation failure returns -1/errstr
/// with the old image intact, and the live-frame rewrite happens only after
/// a fully seeded new image.  On success the caller's trap frame is
/// rewritten in place (R0 = tos, SP → the new seed stack, PC = the new
/// entry, PSR = 0 / EL0t, LR = 0) and dispatch's return erets into the new
/// image — success never returns to the old image (donor exec semantics).
/// The proc keeps its pid, kstack, root table, and fd table (fds survive
/// exec, Plan 9 convention).
///
/// WR-07: seed_user_pages's pagealloc/map_to calls remain fallible AFTER
/// the free (the earlier "seed can no longer fail" invariant was false).
/// A post-free failure cannot return -1 — the trap frame's pc/sp name
/// freed text pages — so it kills the process loudly via the exit tail
/// (the fault-kill shape), not a limping return into a half-installed
/// segment map.
///
/// Returns the new tos_va: the handler passes it up so dispatch's
/// `frame.x0 = result` writeback AGREES with the seeded x0 by construction.
pub fn exec_replace(
    proc: &mut Proc,
    backing: ByteBacking,
    name: &str,
    args: &[&[u8]],
    frame: &mut TrapFrame,
) -> Result<u64> {
    // PARSE first.
    let aout = read_aout(&backing)?;
    // Every remaining VALIDATION check runs BEFORE the free: the seed-fit
    // arithmetic.  (WR-07: seed_user_pages's pagealloc/map_to allocations
    // remain fallible AFTER the free — the "can no longer fail" invariant
    // this comment previously claimed was false — so a post-free failure
    // kills via the exit tail below rather than returning -1 to a trap
    // frame whose pc/sp name freed text pages.)
    check_seed_fit(args)?;

    // FREE second: unmap every current user mapping from the (active) root
    // table and deallocate the phys pages; drop the old segment records and
    // byte source.  (proc.free_user_mappings clears each leaf BEFORE
    // freeing its page — no stale translation can point at recycled pages.)
    proc.free_user_mappings();
    // Donor sysexec clears the note machinery (9front sysproc.c:
    // freenotes + notify=nil + noteureg=nil): a stale handler VA would
    // redirect note delivery into the NEW image (T-04-17), and queued
    // notes named the old image's context.
    proc.notify_handler = None;
    proc.note_queue.clear();
    proc.note_ureg = 0;
    proc.last_note_len = 0;

    // SEED third: fresh seed-stack + Tos pages, new segment records, new
    // byte source.  A failure HERE is post-free (WR-07): returning
    // -1 would eret into a trap frame whose pc/sp name freed text pages,
    // so the process dies loudly via the exit tail (the WR-06 fault-kill
    // shape — the parent reaps the wait record like any other death).
    let source = ByteSource { backing, data_file_base: HDR_SIZE as u64 + aout.text };
    let (user_sp, tos_va) = match install_image(proc, source, name, args, &aout) {
        Ok(v) => v,
        Err(msg) => {
            println!("exec: post-free seed failed ({msg}) — killing pid {}", proc.pid);
            crate::syscall::exit_tail(proc, msg.as_bytes());
        }
    };

    // REWRITE THE LIVE TRAP FRAME in place — the dispatch return path's
    // restore + eret enters the new image.
    frame.seed_el0_entry(tos_va as u64, user_sp as u64, aout.entry);
    Ok(tos_va as u64)
}

/// The pre-free seed-fit check (T-04-13): mirrors seed_user_pages's
/// arithmetic so exec_replace never fails for ARG-SHAPE reasons after the
/// old image is freed (post-free ALLOCATION failures kill via the exit
/// tail — WR-07).
/// argc <= MAX_ARGS and the strings + pointer array + alignment slack must
/// fit the one seed page.
fn check_seed_fit(args: &[&[u8]]) -> Result<()> {
    if args.len() > MAX_ARGS {
        return Err("exec: too many args");
    }
    let strings: usize = args.iter().map(|a| a.len() + 1).sum();
    let slots = 8 * (args.len() + 2) + 16; // argc slot + argv + nil + align slack
    if strings + slots > PAGE_SIZE_4K {
        return Err("exec: arg strings overflow the seed page");
    }
    Ok(())
}

/// Map the two eager exec pages into the proc's (not-yet-active) root table
/// and write the initial stack frame ([argc][argv0]…[nil] + arg strings) via
/// the seed page's KZERO-offset physical alias — direct physical writes are
/// the correct edit path for an inactive table (04-PATTERNS.md vm.rs note).
/// Returns (user_sp, tos_va).
fn seed_user_pages(proc: &mut Proc, args: &[&[u8]]) -> Result<(usize, usize)> {
    if args.len() > MAX_ARGS {
        return Err("exec: too many args");
    }
    let stack_va = USTACKTOP - PAGE_SIZE_4K;
    let stack_pa = map_one_user_page(proc, stack_va, Entry::rw_user_data(), SegKind::Stack)?;
    let tos_page_va = USTACKTOP - 2 * PAGE_SIZE_4K;
    map_one_user_page(proc, tos_page_va, Entry::rw_user_data(), SegKind::Tos)?;

    // Tos sits 16-aligned at the top of its page, just below the seed stack
    // page (9front shape; the page is zeroed — r9 writes no Tos fields).
    let tos_va = (USTACKTOP - PAGE_SIZE_4K - TOS_SIZE) & !0xf;

    // Write [argc][argv0]…[nil] + strings into the seed page via its kernel
    // alias.  Layout ascending from user_sp (the 9front execregs contract —
    // libc main9.s/_callmain consumes exactly this):
    //   user_sp+0: argc; then the argv pointers; then nil; strings above.
    let kbase = physaddr_as_ptr_mut_offset_from_kzero::<u8>(stack_pa) as usize;
    let mut cursor = USTACKTOP; // one past the seed page's top
    let mut arg_ptrs = [0usize; MAX_ARGS];
    for (i, arg) in args.iter().enumerate() {
        let room = cursor - stack_va;
        if arg.len() + 1 > room {
            return Err("exec: arg strings overflow the seed page");
        }
        cursor -= arg.len() + 1;
        let off = cursor - stack_va;
        unsafe {
            ptr::copy_nonoverlapping(arg.as_ptr(), (kbase + off) as *mut u8, arg.len());
            *((kbase + off + arg.len()) as *mut u8) = 0;
        }
        arg_ptrs[i] = cursor;
    }
    cursor &= !0xf; // align the pointer array
    let argc = args.len();
    let slots = argc + 2; // argc slot + argv pointers + nil terminator
    if 8 * slots > cursor - stack_va {
        return Err("exec: stack seed overflow");
    }
    cursor -= 8 * slots;
    let off = cursor - stack_va;
    unsafe {
        let base = (kbase + off) as *mut u64;
        *base = argc as u64;
        for (i, p) in arg_ptrs.iter().take(argc).enumerate() {
            *base.add(1 + i) = *p as u64;
        }
        *base.add(1 + argc) = 0;
    }
    Ok((cursor, tos_va))
}

/// Allocate one phys page, map it into the proc's (inactive) root table at
/// `va` — map_to's temp-recursive dance edits inactive tables — and map the
/// same page at KZERO+PA in the kernel table for content writes (04-02's
/// KZERO+PA convention).  Zero the page and append to the mapping log.
fn map_one_user_page(
    proc: &mut Proc,
    va: usize,
    entry: Entry,
    kind: SegKind,
) -> Result<PhysAddr> {
    let pa = pagealloc::allocate_physpage().map_err(|_| "exec: page alloc failed")?;
    unsafe {
        let mut physalloc = PhysPageAllocator {};
        let mut vmt = VmTraitImpl {};
        (*proc.root_table)
            .map_to(
                &mut physalloc,
                &mut vmt,
                entry.with_phys_addr(pa),
                va,
                PageSize::Page4K,
                RootPageTableType::User,
            )
            .map_err(|_| "exec: user page map failed")?;
    }
    {
        let mut physalloc = PhysPageAllocator {};
        let mut vmt = VmTraitImpl {};
        vm::kernel_pagetable()
            .map_phys_range(
                &mut physalloc,
                &mut vmt,
                "execpage",
                &PhysRange::with_pa_len(pa, PAGE_SIZE_4K),
                vm::VaMapping::Offset(KZERO),
                Entry::rw_kernel_data(),
                PageSize::Page4K,
                RootPageTableType::Kernel,
            )
            .map_err(|_| "exec: kernel alias map failed")?;
    }
    unsafe { ptr::write_bytes(physaddr_as_ptr_mut_offset_from_kzero::<u8>(pa), 0, PAGE_SIZE_4K) };
    proc.mapping_log.push(MappingRecord {
        va,
        phys: pa.addr() as usize,
        size: PAGE_SIZE_4K,
        kind,
    });
    Ok(pa)
}

/// Seed the forkret-shaped TrapFrame at the top of the proc's kernel stack
/// (9front execregs + touser shape; 04-RESEARCH.md "Code Examples").  The
/// frame IS Harvey's Ureg (04-03's published layout — the const asserts and
/// xtask ureg-check pin it); kstack_top - 288 keeps it 16-aligned.
pub fn seed_exec_frame(
    kstack_top: usize,
    entry: u64,
    user_sp: u64,
    tos_va: u64,
) -> *mut TrapFrame {
    let frame_ptr = (kstack_top - core::mem::size_of::<TrapFrame>()) as *mut TrapFrame;
    let frame = unsafe { &mut *frame_ptr };
    *frame = TrapFrame::zeroed();
    frame.seed_el0_entry(tos_va, user_sp, entry);
    frame_ptr
}
