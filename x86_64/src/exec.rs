//! Plan 9 a.out loader (amd64) + exec image policy: parse and validate
//! the big-endian header, compute the segment map (the 6l-derived
//! constants), and seed the forkret-shaped 184-byte Ureg on the proc's
//! kernel stack for the touser/SYSRETQ handoff.
//!
//! Eager-vs-demand mapping policy (the aarch64 module-doc terms,
//! re-keyed — with the x86_64 deviation): text, data and bss pages are
//! ALL mapped EAGERLY at install.  The data+bss half is the aarch64
//! rule verbatim (6c places string literals and statics in data, and a
//! real binary passes them to syscalls without any prior ring-3 touch;
//! `validate_user_range` is a membership check and nested
//! demand-paging from syscall context is forbidden).  TEXT is eager as
//! the x86_64 deviation: no trap-context pager exists on x86_64 this
//! era (the aarch64 fault.rs demand path has no sibling here), so the
//! first instruction fetch must find its page mapped — text demand
//! paging arrives with a pager (the recorded follow-up).  Beyond the
//! two seeded top pages, deeper stack/tos growth likewise waits on the
//! pager.
//!
//! exec_into is the first-exec path (the exec bootstrap, 14-05);
//! exec_replace is the EXEC syscall's replace-the-running-image path
//! (parse first, free second, seed third, then rewrite the live Ureg in
//! place — a validation failure returns -1/errstr with the old image
//! intact).  The mapping-carrying bodies of that split (install_image /
//! map_text_eager / map_data_bss_eager / seed_user_pages, and
//! read_aout's backing-length check) are 14-04's proc-port half,
//! landed here against proc's PML4 walk.
//!
//! The segment constants are the 6l-derived set (sys/src/cmd/6l/obj.c
//! HEADTYPE 2 defaults: HEADR 40, INITTEXT 0x200000+HEADR, INITRND
//! 0x200000) — published as amd64/ABI.md's stack-top policy section.

use port::mem::{PAGE_SIZE_4K, VirtRange};
use port::Result;

use crate::dat::Ureg;
use crate::proc::{self, ByteBacking, ByteSource, Proc, ProcState, SegKind, Segments};

/// Harvey sizeof(Tos) on amd64 (sys/include/tos.h: the same 88 bytes —
/// identical pointer/long sizes) — the Tos page is mapped zeroed; r9
/// never writes Tos fields this phase (cheap insurance only).
const TOS_SIZE: usize = 88;
/// Cap on exec args — the initial stack frame must fit the seed page
/// (the byte-fit check in check_seed_fit is the REAL constraint; this
/// pre-cap just keeps the slots array honest).  32 matches sys_exec's
/// entry cap (MAX_EXEC_ARGS).
const MAX_ARGS: usize = 32;

/// Read + parse + validate the 40-byte a.out header through the byte
/// backing's window — byte-granular (no page-alignment assumptions on
/// file offsets).  Runs BEFORE any mapping or freeing in both exec
/// paths — an invalid image is a loud Err with everything intact.
fn read_aout(backing: &ByteBacking) -> Result<AoutHeader> {
    let mut hdr = [0u8; HDR_SIZE];
    let n = backing.read_at(0, &mut hdr)?;
    if n < HDR_SIZE {
        return Err("exec: short read of a.out header");
    }
    let h = parse_aout(&hdr)?;
    // The header's declared segment sizes must FIT the backing's
    // length — a truncated binary otherwise execs "successfully" and
    // the missing tail executes as zeros (the donor fails the exec on
    // a short segment read; r9 fails it up front).  syms ride the
    // image's length semantics even though exec never pages them in.
    // (The sum cannot overflow: four u32-derived u64s + 40.)
    let declared = HDR_SIZE as u64 + h.text + h.data + h.syms;
    if declared > backing.file_len() {
        return Err("exec: truncated binary");
    }
    Ok(h)
}

/// Record the segment map + the demand-page byte source + the proc
/// name, map text/data/bss EAGERLY, then seed the two eager stack-side
/// pages.  Shared by first-exec (exec_into) and image replacement
/// (exec_replace); returns (user_sp, tos_va).
fn install_image(
    proc: &mut Proc,
    source: ByteSource,
    name: &str,
    args: &[&[u8]],
    aout: &AoutHeader,
) -> Result<(usize, usize)> {
    // Record the segment map (page-rounded text for the classifier;
    // byte-exact data/bss ends — the aarch64 shape) and the byte
    // source.
    let text_end = round_up_pow2(aout.text_va + aout.text as usize, PAGE_SIZE_4K);
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
    // Record the argv for devproc's /proc/N/args — install_image is
    // shared by both exec paths, so EVERY exec records.
    proc.record_argv(args);

    // Map text EAGERLY (the x86_64 deviation from the aarch64 policy,
    // documented in the module header: no trap-context pager exists on
    // x86_64 this era, so the first instruction fetch must find its
    // page mapped), then data+bss (the cold-syscall-argument fix).
    map_text_eager(proc, &source, aout)?;
    map_data_bss_eager(proc, &source, aout)?;
    proc.text_source = Some(source);

    // SEED: the two eager stack-side pages — the top-of-stack seed
    // page and the zeroed Tos page just below it.
    seed_user_pages(proc, args)
}

/// Eager text mapping (the x86_64 policy deviation): every text page
/// maps in at install with its file bytes (the header rides the
/// mapping — the file offset is pageVA - TEXT_MAP_BASE; the tail page
/// clamps at the byte-exact text end so the data bytes never bleed
/// into the text mapping).
fn map_text_eager(proc: &mut Proc, source: &ByteSource, aout: &AoutHeader) -> Result<()> {
    let text_end = aout.text_va + aout.text as usize;
    let mut page_va = TEXT_MAP_BASE;
    while page_va < text_end {
        let pa = proc.map_user_page(page_va, SegKind::Text)?;
        let clamp = (text_end - page_va).min(PAGE_SIZE_4K);
        let file_off = (page_va - TEXT_MAP_BASE) as u64;
        // SAFETY: pa names the just-mapped owned page; clamp bytes lie
        // inside it.
        let dst = unsafe {
            core::slice::from_raw_parts_mut(proc::phys_ptr_mut(pa), clamp)
        };
        source.read_at(file_off, dst).map_err(|_| "exec: text page-in read failed")?;
        page_va += PAGE_SIZE_4K;
    }
    Ok(())
}

/// Eager data+bss mapping: each page maps in zeroed (the seam), then
/// the data segment's file content clamp-writes through the kernel
/// alias (byte-exact end — the file's trailing syms bytes must never
/// bleed into the bss-sharing tail).  The pages are logged
/// (SegKind::Data/Bss) so rfork's copy and exit's free cover them like
/// any other mapping.
fn map_data_bss_eager(proc: &mut Proc, source: &ByteSource, aout: &AoutHeader) -> Result<()> {
    let data_end = aout.data_va + aout.data as usize;
    let bss_end = data_end + aout.bss as usize;
    let data_file_base = HDR_SIZE as u64 + aout.text;
    let mut page_va = aout.data_va;
    let data_pages_end = round_up_pow2(data_end, PAGE_SIZE_4K);
    while page_va < data_pages_end {
        let pa = proc.map_user_page(page_va, SegKind::Data)?;
        let clamp = data_end.saturating_sub(page_va).min(PAGE_SIZE_4K);
        if clamp > 0 {
            let file_off = data_file_base + (page_va - aout.data_va) as u64;
            // SAFETY: pa names the just-mapped owned page; clamp bytes
            // lie inside it.
            let dst = unsafe {
                core::slice::from_raw_parts_mut(proc::phys_ptr_mut(pa), clamp)
            };
            source.read_at(file_off, dst).map_err(|_| "exec: data page-in read failed")?;
        }
        page_va += PAGE_SIZE_4K;
    }
    let bss_pages_end = round_up_pow2(bss_end, PAGE_SIZE_4K);
    let mut page_va = data_pages_end;
    while page_va < bss_pages_end {
        proc.map_user_page(page_va, SegKind::Bss)?;
        page_va += PAGE_SIZE_4K;
    }
    Ok(())
}

/// The pre-free seed-fit check: mirrors seed_user_pages's arithmetic
/// so exec_replace never fails for ARG-SHAPE reasons after the old
/// image is freed (post-free ALLOCATION failures kill via the exit
/// tail).
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

/// Map the two eager exec pages and write the initial stack frame
/// ([argc][argv0]…[nil] + arg strings) through the seed page's kernel
/// alias (direct physical writes — the correct edit path for any table
/// state).  Returns (user_sp, tos_va).  The argv block is
/// ORDER-STABLE: the pointer array walks the args in argument order
/// and the strings copy byte-verbatim (the plan's ordering edge).
fn seed_user_pages(proc: &mut Proc, args: &[&[u8]]) -> Result<(usize, usize)> {
    if args.len() > MAX_ARGS {
        return Err("exec: too many args");
    }
    let stack_va = USTACKTOP - PAGE_SIZE_4K;
    let stack_pa = proc.map_user_page(stack_va, SegKind::Stack)?;
    let tos_page_va = USTACKTOP - 2 * PAGE_SIZE_4K;
    proc.map_user_page(tos_page_va, SegKind::Tos)?;

    // Tos sits 16-aligned at the top of its page, just below the seed
    // stack page (the donor shape; the page is zeroed — r9 writes no
    // Tos fields).
    let tos_va = (USTACKTOP - PAGE_SIZE_4K - TOS_SIZE) & !0xf;

    // Write [argc][argv0]…[nil] + strings into the seed page via its
    // kernel alias.  Layout ascending from user_sp (the execregs
    // contract — libc main9.s/_callmain consumes exactly this):
    //   user_sp+0: argc; then the argv pointers; then nil; strings
    //   packed top-down above, in argument order.
    let kbase = proc::phys_ptr_mut(stack_pa) as usize;
    let mut cursor = USTACKTOP; // one past the seed page's top
    let mut arg_ptrs = [0usize; MAX_ARGS];
    for (i, arg) in args.iter().enumerate() {
        let room = cursor - stack_va;
        if arg.len() + 1 > room {
            return Err("exec: arg strings overflow the seed page");
        }
        cursor -= arg.len() + 1;
        let off = cursor - stack_va;
        // SAFETY: kbase+off..+arg.len()+1 lies inside the seed page
        // (the room check above); kbase is the page's kernel alias;
        // the arg source is an owned kernel slice.
        unsafe {
            core::ptr::copy_nonoverlapping(arg.as_ptr(), (kbase + off) as *mut u8, arg.len());
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
    // SAFETY: the slots check above proves 8*(argc+2) bytes fit below
    // the arg strings inside the seed page; base walks exactly that
    // window.
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

/// First exec (the 14-05 exec_bootstrap path): parse the binary's
/// header through the paqfs window, record segments + the byte source,
/// map text/data/bss eagerly, seed the two eager pages + the
/// forkret-shaped Ureg on the proc's kernel stack, and leave the proc
/// Ready.
pub fn exec_into(
    proc: &mut Proc,
    image: &'static [u8],
    bin: &port::paqfs::PaqDir,
    name: &str,
    args: &[&[u8]],
) -> Result<()> {
    let backing = ByteBacking::Paq { dir: bin.clone(), image };
    let aout = read_aout(&backing)?;
    let source = ByteSource { backing, data_file_base: HDR_SIZE as u64 + aout.text };
    let (user_sp, tos_va) = install_image(proc, source, name, args, &aout)?;

    // The forkret-shaped 184-byte Ureg at kstack_top-184 (through the
    // top kstack page's kernel alias — the correct edit path from the
    // bootstrap's table context): bp = tos (RARG), sp = the seeded
    // user stack, pc = entry — the resume tail SYSRETQs into ring 3
    // from exactly this frame.
    let frame = proc.seed_frame(aout.entry, user_sp as u64, tos_va as u64);
    proc.frame = frame;
    proc.state = ProcState::Ready;
    Ok(())
}

/// EXEC behind a running process (Plan 9 sysexec semantics): REPLACE
/// the caller's image with the binary carried by `backing` (the
/// byte-source abstraction — devroot's paqfs image entry, or a devmnt
/// binary's staged whole-file buffer).
///
/// Ordering: PARSE the new image's a.out first (plus every other
/// fallible pre-check), FREE the old mappings second, SEED third — so
/// a validation failure returns -1/errstr with the old image intact,
/// and the live-frame rewrite happens only after a fully seeded new
/// image.  On success the caller's Ureg is rewritten in place (bp =
/// tos, sp = the new seed stack, pc = the new entry) and the dispatch
/// return SYSRETQs into the new image — success never returns to the
/// old image (donor exec semantics).  The proc keeps its pid, kstack,
/// PML4, and fd table (fds survive exec, Plan 9 convention).
///
/// seed_user_pages's page allocations remain fallible AFTER the free:
/// a post-free failure cannot return -1 — the frame's pc/sp name freed
/// text pages — so it kills the process loudly via the exit tail, not
/// a limping return into a half-installed segment map.
///
/// Returns the new tos_va: the handler passes it up so dispatch's
/// `frame.ax = result` writeback lands alongside the seeded bp.
pub fn exec_replace(
    proc: &mut Proc,
    backing: ByteBacking,
    name: &str,
    args: &[&[u8]],
    frame: &mut Ureg,
) -> Result<u64> {
    // PARSE first.
    let aout = read_aout(&backing)?;
    // Every remaining VALIDATION check runs BEFORE the free: the
    // seed-fit arithmetic.
    check_seed_fit(args)?;

    // FREE second: unmap every current user mapping from the (active)
    // PML4 and deallocate the phys pages; drop the old segment records
    // and byte source.  (free_user_mappings clears each leaf BEFORE
    // freeing its page — no stale translation can point at recycled
    // pages.)
    proc.free_user_mappings();
    // Donor sysexec clears the note machinery: a stale handler VA
    // would redirect note delivery into the NEW image, and queued
    // notes named the old image's context.
    proc.notify_handler = None;
    proc.note_queue.clear();
    proc.note_ureg = 0;
    proc.last_note_len = 0;

    // SEED third.  A failure HERE is post-free: the process dies
    // loudly via the exit tail (the parent reaps the wait record like
    // any other death).
    let source = ByteSource { backing, data_file_base: HDR_SIZE as u64 + aout.text };
    let (user_sp, tos_va) = match install_image(proc, source, name, args, &aout) {
        Ok(v) => v,
        Err(msg) => {
            port::println!("exec: post-free seed failed ({msg}) — killing pid {}", proc.pid);
            crate::syscall::exit_tail(proc, msg.as_bytes());
        }
    };

    // REWRITE THE LIVE UREG in place — the return path's restore +
    // SYSRETQ enters the new image.
    frame.seed_exec_entry(tos_va as u64, user_sp as u64, aout.entry);
    Ok(tos_va as u64)
}

// --- r9 user address-space policy (r9 owns stack-top placement,
// segment/growth policy, and the initial stack frame; text/data VAs are
// fixed by 6l in the a.out header) ---
/// Top of the user stack — the adopted discretion resolution (14-01,
/// ABI.md section 7): the 9k k10 mem.h donor values.
pub const USTACKTOP: usize = 0x0000_7fff_ffff_f000;
/// Stack-growth bound (the donor's USTKSIZE reference value).
pub const USTKSIZE: usize = 16 * 1024 * 1024;
/// Ceiling for text/data/bss VAs.
pub const USTOP: usize = USTACKTOP - USTKSIZE;

// --- a.out format (Harvey sys/include/a.out.h; the 6l-derived facts) ---
/// S_MAGIC amd64: _MAGIC(HDR_MAGIC, 26) = 0x8A97.
pub const S_MAGIC_AMD64: u32 = 0x8A97;
/// HDR_MAGIC header size: 8×BE u32 + the BE u64 entry at bytes 32..39;
/// text follows at file offset 40 (6l HEADR = 32+8).
pub const HDR_SIZE: usize = 40;
/// 6l's default text VA (INITTEXT = 0x200000 + HEADR, no -T flag) —
/// UTZERO+0x28, the seeded-entry VA touser's fixed ip mirrors.
pub const INITTEXT: usize = 0x200000 + HDR_SIZE;
/// The text mapping base: 6l includes the 40-byte header in the text
/// mapping, so text pages map from 0x200000 and the fault handler's
/// text file offset is pageVA - 0x200000.
pub const TEXT_MAP_BASE: usize = 0x200000;
/// 6l's INITRND (HEADTYPE 2 default): the data segment lands at the
/// NEXT 2 MiB boundary above text end — NOT aarch64's 64 KiB rule (the
/// A1 assumption, derived from obj.c; proven against a real 6l binary
/// at 14-05's stage-build magic assert).
pub const DATA_ALIGN: usize = 0x200000;

/// The 184-byte Ureg's planted user-entry flags: If — the user enters
/// with IF=1 (touser's R11 plant; the unmask-at-handoff discipline).
const IF: u64 = 0x200;

/// A validated a.out header (all on-disk fields big-endian).
#[derive(Debug, PartialEq)]
pub struct AoutHeader {
    pub text: u64,
    pub data: u64,
    pub bss: u64,
    pub syms: u64,
    pub entry: u64,
    /// Text VA (INITTEXT); the mapping base is TEXT_MAP_BASE (header
    /// included).
    pub text_va: usize,
    /// Data VA — the next 2 MiB-aligned VA above text end (6l's INITRND).
    pub data_va: usize,
}

/// The exec segment map (the install_image ranges, computed here so the
/// layout math is host-testable ahead of 14-04's mapping bodies).
/// Page-rounded where the fault classifier needs it (text); data/bss
/// keep byte-exact ends (the eager mapper clamps the tail).
pub struct SegmentLayout {
    pub text: VirtRange,
    pub data: VirtRange,
    pub bss: VirtRange,
}


/// Power-of-two round-up (the local `next_multiple_of` replacement):
/// `usize::next_multiple_of` lowers through u128 arithmetic
/// (`__multi3` — soft-float in this SSE-free kernel), a link error
/// once the syscall chain made this module's code live.  Both
/// alignments here are powers of two, so the mask form is exact.
fn round_up_pow2(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}

fn be32(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], off: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(off..off + 8)?.try_into().ok()?))
}

/// Parse and validate an a.out header (the donor sysexec check set,
/// ported — every violation is a loud Err before any mapping exists).
pub fn parse_aout(bytes: &[u8]) -> Result<AoutHeader> {
    if bytes.len() < HDR_SIZE {
        return Err("exec: short a.out header");
    }
    let magic = be32(bytes, 0).ok_or("exec: short a.out header")?;
    if magic != S_MAGIC_AMD64 {
        return Err("exec: bad a.out magic (not S_MAGIC amd64)");
    }
    // Bounds proven above: the header is 40 bytes.
    let text = be32(bytes, 4).unwrap() as u64;
    let data = be32(bytes, 8).unwrap() as u64;
    let bss = be32(bytes, 12).unwrap() as u64;
    let syms = be32(bytes, 16).unwrap() as u64;
    // The u32 entry field at 20 is vestigial on HDR_MAGIC arches; the
    // real entry point is the BE u64 at bytes 32..39.
    let entry = be64(bytes, 32).ok_or("exec: short a.out header")?;

    if text == 0 {
        return Err("exec: empty text segment");
    }
    let text_va = INITTEXT;
    if entry < text_va as u64 || entry >= text_va as u64 + text {
        return Err("exec: entry point outside text segment");
    }
    // Data VA: the next 2 MiB-aligned VA above text end (6l's INITRND
    // for HEADTYPE 2).  File offset is 40+textsize — NOT page-aligned.
    // The sum cannot overflow: u32-derived u64s on a VA below 2^47.
    let data_va = round_up_pow2(text_va + text as usize, DATA_ALIGN);
    check_data_bss_below_ustop(data_va, data, bss)?;
    Ok(AoutHeader { text, data, bss, syms, entry, text_va, data_va })
}

/// The USTOP ceiling check (the donor's, carried verbatim): data+bss
/// must leave room below the stack.  NOTE: with u32 a.out size fields
/// (~8 GiB max) against the 128 TiB user map this can never fire for a
/// parseable header — it is defense-in-depth (a future 64-bit format
/// or a moved stack policy), factored out so its boundary arithmetic
/// is host-pinned anyway.
fn check_data_bss_below_ustop(data_va: usize, data: u64, bss: u64) -> Result<()> {
    if data_va as u64 + data + bss >= USTOP as u64 {
        return Err("exec: text+data+bss exceeds USTOP");
    }
    Ok(())
}

/// The exec segment map (the install_image ranges, 14-04's consumer):
/// text page-rounded for the fault classifier, mapped from
/// TEXT_MAP_BASE (the 40-byte header rides the text mapping — the text
/// file offset is pageVA - TEXT_MAP_BASE); data/bss keep byte-exact
/// ends (the eager mapper clamps the data tail inside its last page).
pub fn segment_layout(aout: &AoutHeader) -> SegmentLayout {
    let text_end = round_up_pow2(aout.text_va + aout.text as usize, PAGE_SIZE_4K);
    let data_end = aout.data_va + aout.data as usize;
    let bss_end = data_end + aout.bss as usize;
    SegmentLayout {
        text: VirtRange::with_end(TEXT_MAP_BASE, text_end),
        data: VirtRange::with_end(aout.data_va, data_end),
        bss: VirtRange::with_end(data_end, bss_end),
    }
}

/// Seed the forkret-shaped 184-byte Ureg at the top of the proc's kernel
/// stack (9k sysexecregs + touser shape): zeroed, then the entry state
/// planted.  The frame-return tail (14-02's syscallreturn) enters ring 3
/// from exactly this frame — CX=ip, R11=flags, the quartet loads, the
/// user sp — and the first instruction fetch faults in the first text
/// page.
pub fn seed_exec_frame(kstack_top: usize, entry: u64, user_sp: u64, tos_va: u64) -> *mut Ureg {
    let frame_ptr = (kstack_top - core::mem::size_of::<Ureg>()) as *mut Ureg;
    // SAFETY: the frame sits at the top of the proc's freshly-mapped
    // kstack (184 bytes below kstack_top — mapped by construction; the
    // host tests pass a real buffer); nothing else aliases it until the
    // first ring-3 entry.
    let frame = unsafe { &mut *frame_ptr };
    *frame = Ureg::zeroed();
    frame.seed_exec_entry(tos_va, user_sp, entry);
    frame_ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic 40-byte a.out header: magic/text/data/bss/syms
    /// BE u32s, the vestigial entry u32 at 20, spsz/pcsz at 24/28, and
    /// the REAL entry BE u64 at bytes 32..39 (the HDR_MAGIC layout).
    fn aout_header(magic: u32, text: u32, data: u32, bss: u32, syms: u32, entry: u64) -> [u8; 40] {
        let mut h = [0u8; 40];
        h[0..4].copy_from_slice(&magic.to_be_bytes());
        h[4..8].copy_from_slice(&text.to_be_bytes());
        h[8..12].copy_from_slice(&data.to_be_bytes());
        h[12..16].copy_from_slice(&bss.to_be_bytes());
        h[16..20].copy_from_slice(&syms.to_be_bytes());
        h[32..40].copy_from_slice(&entry.to_be_bytes());
        h
    }

    #[test]
    fn parse_aout_accepts_well_formed_s_magic() {
        let h = aout_header(S_MAGIC_AMD64, 0x1000, 0x100, 0x80, 0x40, INITTEXT as u64);
        let a = parse_aout(&h).expect("well-formed S_MAGIC header parses");
        assert_eq!(a.text, 0x1000);
        assert_eq!(a.data, 0x100);
        assert_eq!(a.bss, 0x80);
        assert_eq!(a.syms, 0x40);
        assert_eq!(a.entry, INITTEXT as u64);
        assert_eq!(a.text_va, INITTEXT);
        // data_va: the NEXT 2 MiB multiple above text end (0x201028 ->
        // 0x400000) — 6l's INITRND, not aarch64's 64 KiB rule.
        assert_eq!(a.data_va, 0x400000);
        // The pinned segment layout: text maps from TEXT_MAP_BASE
        // (header included).
        let l = segment_layout(&a);
        assert_eq!(l.text.0.start, TEXT_MAP_BASE);
        assert_eq!(l.text.0.end, (INITTEXT + 0x1000).next_multiple_of(PAGE_SIZE_4K));
        assert_eq!(l.data.0.start, 0x400000);
        assert_eq!(l.data.0.end, 0x400000 + 0x100);
        assert_eq!(l.bss.0.start, 0x400000 + 0x100);
        assert_eq!(l.bss.0.end, 0x400000 + 0x100 + 0x80);
    }

    #[test]
    fn truncated_header_is_a_loud_named_err() {
        // Fewer bytes than HEADR — never a panic, never a partial read.
        let h = aout_header(S_MAGIC_AMD64, 0x1000, 0, 0, 0, INITTEXT as u64);
        assert_eq!(parse_aout(&h[..39]), Err("exec: short a.out header"));
        assert_eq!(parse_aout(&h[..4]), Err("exec: short a.out header"));
    }

    #[test]
    fn zero_length_input_is_a_loud_named_err() {
        assert_eq!(parse_aout(&[]), Err("exec: short a.out header"));
    }

    #[test]
    fn wrong_magic_is_a_named_err() {
        // The aarch64 R_MAGIC value must be rejected by the amd64
        // loader — a nameable mix-up class.
        let h = aout_header(0x8C47, 0x1000, 0, 0, 0, INITTEXT as u64);
        assert_eq!(parse_aout(&h), Err("exec: bad a.out magic (not S_MAGIC amd64)"));
    }

    #[test]
    fn entry_outside_text_is_a_named_err() {
        // One past the text end.
        let h = aout_header(S_MAGIC_AMD64, 0x1000, 0, 0, 0, (INITTEXT + 0x1000) as u64);
        assert_eq!(parse_aout(&h), Err("exec: entry point outside text segment"));
        // Below text_va: inside the mapped header, not inside text.
        let h = aout_header(S_MAGIC_AMD64, 0x1000, 0, 0, 0, (INITTEXT - 8) as u64);
        assert_eq!(parse_aout(&h), Err("exec: entry point outside text segment"));
    }

    #[test]
    fn empty_text_is_a_named_err() {
        let h = aout_header(S_MAGIC_AMD64, 0, 0x100, 0x80, 0, 0x200028);
        assert_eq!(parse_aout(&h), Err("exec: empty text segment"));
    }

    #[test]
    fn data_bss_overflowing_ustop_is_a_named_err() {
        // Driven at the check itself: u32 a.out fields (~8 GiB max)
        // cannot reach USTOP on the 128 TiB user map, so no synthetic
        // header can trip the parse-path call — the boundary arithmetic
        // is pinned here instead (the check is donor-carried
        // defense-in-depth; parse_aout calls this same function).
        let base = 0x400000usize;
        let exact = (USTOP - base) as u64;
        assert_eq!(
            check_data_bss_below_ustop(base, USTOP as u64, 0),
            Err("exec: text+data+bss exceeds USTOP")
        );
        // The exact boundary is >= (rejected)...
        assert_eq!(
            check_data_bss_below_ustop(base, exact, 0),
            Err("exec: text+data+bss exceeds USTOP")
        );
        // ...one byte below it is accepted.
        assert_eq!(check_data_bss_below_ustop(base, exact - 1, 0), Ok(()));
        assert_eq!(check_data_bss_below_ustop(base, 0, exact - 1), Ok(()));
        // A maximal u32-field header stays legal everywhere on the map.
        let h = aout_header(S_MAGIC_AMD64, 0x1000, 0xffff_ffff, 0xffff_ffff, 0, INITTEXT as u64);
        assert!(parse_aout(&h).is_ok());
    }

    #[test]
    fn empty_data_segment_maps_text_only() {
        // data+bss == 0: text only — no data segment (empty range), no
        // overlap with the text mapping (the empty edge).
        let h = aout_header(S_MAGIC_AMD64, 0x1000, 0, 0, 0, INITTEXT as u64);
        let a = parse_aout(&h).expect("empty data segment is legal");
        assert_eq!(a.data, 0);
        assert_eq!(a.bss, 0);
        let l = segment_layout(&a);
        assert_eq!(l.text.0.start, TEXT_MAP_BASE);
        assert_eq!(l.text.0.end, (INITTEXT + 0x1000).next_multiple_of(PAGE_SIZE_4K));
        assert_eq!(l.data.0.start, l.data.0.end);
        assert_eq!(l.bss.0.start, l.bss.0.end);
        assert!(l.data.0.start >= l.text.0.end);
    }

    #[test]
    fn adjacent_segments_at_the_2mib_boundary() {
        // Text ending EXACTLY at a 2 MiB boundary puts data_va at that
        // same boundary — two adjacent segments, zero gap, zero overlap,
        // no stray page (the adjacency edge; INITTEXT = 0x200028).
        let text = (0x400000 - INITTEXT) as u32;
        assert_eq!(text, 0x1FFFD8);
        let h = aout_header(S_MAGIC_AMD64, text, 0x1234, 0x100, 0, INITTEXT as u64);
        let a = parse_aout(&h).expect("boundary-adjacent binary parses");
        assert_eq!(a.data_va, 0x400000);
        let l = segment_layout(&a);
        assert_eq!(l.text.0.end, 0x400000);
        assert_eq!(l.data.0.start, 0x400000);
        assert_eq!(l.data.0.end, 0x400000 + 0x1234);
    }

    #[test]
    fn seed_exec_frame_plants_the_184_byte_ureg_entry_state() {
        // A real host buffer stands in for the proc kstack: the frame
        // lands at kstack_top - 184 and every planted field is read back
        // at the ABI.md byte offsets — the witness against the frozen
        // layout and the fields 14-02's touser/syscallreturn consume.
        let mut kstack = [0u8; 256];
        let kstack_top = kstack.as_ptr() as usize + kstack.len();
        let entry = (INITTEXT + 0x40) as u64;
        let user_sp = 0x0000_7fff_ff00_0000u64;
        let tos_va = 0x0000_7fff_feff_ff00u64;
        let frame = seed_exec_frame(kstack_top, entry, user_sp, tos_va);
        assert_eq!(frame as usize + core::mem::size_of::<Ureg>(), kstack_top);
        // SAFETY: frame names the seeded Ureg inside the test buffer;
        // every read below is at a frozen ABI offset inside 184 bytes.
        let read = |off: usize| -> u64 {
            unsafe { ((frame as usize + off) as *const u64).read_unaligned() }
        };
        let read16 = |off: usize| -> u16 {
            unsafe { ((frame as usize + off) as *const u16).read_unaligned() }
        };
        // bp carries the Tos pointer (RARG = BP — the 6c first-arg
        // register, the aarch64 x0 analog).
        assert_eq!(read(48), tos_va);
        // type: the donor's "fiction for acid" (sysexecregs).
        assert_eq!(read(128), 64);
        // ip: the a.out entry field.
        assert_eq!(read(144), entry);
        // cs/ss: the flat user selectors, the values syscallentry
        // constructs and userureg checks (witnessed 0x23/0x1b in
        // trap.rs's const asserts).
        assert_eq!(read(152), 0x23);
        assert_eq!(read(176), 0x1b);
        // flags = If — the user enters with IF=1.
        assert_eq!(read(160), IF);
        // sp: the seeded user stack.
        assert_eq!(read(168), user_sp);
        // The packed segment quartet: the flat user data selector in all
        // four (touser's DS/ES/FS/GS plant).
        assert_eq!(read16(120), 0x1b);
        assert_eq!(read16(122), 0x1b);
        assert_eq!(read16(124), 0x1b);
        assert_eq!(read16(126), 0x1b);
        // Everything the seed does not plant stays zero.
        assert_eq!(read(0), 0); // ax
        assert_eq!(read(16), 0); // cx
        assert_eq!(read(136), 0); // ecode
    }
}
