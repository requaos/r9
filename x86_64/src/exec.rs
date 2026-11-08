//! Plan 9 a.out loader (amd64) + exec image policy: parse and validate
//! the big-endian header, compute the segment map (the 6l-derived
//! constants), and seed the forkret-shaped 184-byte Ureg on the proc's
//! kernel stack for the touser/SYSRETQ handoff.
//!
//! Eager-vs-demand mapping policy (the aarch64 module-doc terms,
//! re-keyed): data+bss pages are mapped EAGERLY at install — 6c places
//! string literals and statics in data, and a real binary (boot(8))
//! passes them to syscalls without any prior ring-3 touch;
//! `validate_user_range` is a membership check and nested demand-paging
//! from syscall context is forbidden, so an unread literal page would
//! fault the kernel mid-syscall.  Text, stack and Tos pages fault in on
//! demand instead: text pages arrive from the paqfs byte source;
//! stack/tos faults map zero pages.
//!
//! exec_into is the first-exec path (the exec bootstrap, 14-05);
//! exec_replace is the EXEC syscall's replace-the-running-image path
//! (parse first, free second, seed third, then rewrite the live Ureg in
//! place — a validation failure returns -1/errstr with the old image
//! intact).  The mapping-carrying bodies of that split (install_image /
//! map_data_bss_eager / seed_user_pages, and read_aout's
//! backing-length check) land with 14-04's proc port — this module
//! lands the loader itself host-verified: parse/validate, the segment
//! math, and the Ureg seed (the v1.0 04-04 compile-and-host-test
//! precedent).
//!
//! The segment constants are the 6l-derived set (sys/src/cmd/6l/obj.c
//! HEADTYPE 2 defaults: HEADR 40, INITTEXT 0x200000+HEADR, INITRND
//! 0x200000) — published as amd64/ABI.md's stack-top policy section.

use port::mem::{PAGE_SIZE_4K, VirtRange};
use port::Result;

use crate::dat::Ureg;

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
    let data_va = (text_va + text as usize).next_multiple_of(DATA_ALIGN);
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
    let text_end = (aout.text_va + aout.text as usize).next_multiple_of(PAGE_SIZE_4K);
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
