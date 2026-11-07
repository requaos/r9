// The mb2 info-structure parser — the /chosen-and-memory analog of
// port/fdt.rs: heap-less, bounds-checked, error-typed.
//
// Structure (multiboot2 spec 3.2; x86 native little-endian):
//   { u32 total_size; u32 reserved; tags... }
// Tags are 8-byte aligned, each { u32 type; u32 size } with size counting
// the 8-byte header; the walk advances 8-aligned and terminates at the
// type-0 end tag.  The memory map rides tag 6; ACPI RSDP pointers ride
// tags 14 (old, u32) / 15 (new, u64) — the preferred RSDP source per the
// D-03 loader-mediated theme (13-04 consumes it; on BIOS boots GRUB emits
// neither, and the loud-absence print is the expected case).
//
// Ground truth (live hex dump of GRUB 2.14's structure on the q35 channel,
// 13-02): the boot-loader-name tag carries size 18, an empty cmdline tag
// size 9, an APM tag size 28 — tag SIZES are legitimately NOT 8-multiples;
// only tag POSITIONS are aligned (the spec's tag padding).  The walk must
// align-up its advance; the alignment errors below therefore fire on
// positions/total_size/fixed-body sizes, never on size%8.
//
// The pointer GRUB hands the kernel is untrusted firmware input (T-13-03):
// every bounds or alignment violation is a loud named ParseError — never a
// silent skip, never a walk off the end.

/// A memory-map entry's fixed size: {u64 base, u64 len, u32 type, u32 res}.
const MMAP_ENTRY_SIZE: usize = 24;

/// The tag types this parser consumes (spec 3.2.2).
const TAG_END: u32 = 0;
const TAG_MMAP: u32 = 6;
const TAG_ACPI_OLD_RSDP: u32 = 14;
const TAG_ACPI_NEW_RSDP: u32 = 15;

/// Every extraction goes through checked slice gets — never raw indexing.
fn le_u32(bytes: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(off..off + 4)?.try_into().ok()?))
}

fn le_u64(bytes: &[u8], off: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(off..off + 8)?.try_into().ok()?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Slice too short for the fixed {total_size, reserved} prefix.
    BufferTooSmall,
    /// total_size disagrees with the slice the caller formed (two-pass
    /// mismatch), undercuts prefix+end-tag, or is not 8-aligned (the whole
    /// structure is 8-aligned; every tag boundary then is too).
    InvalidTotalSize,
    /// A tag's size field is smaller than its own {type, size} header.
    TagHeaderTooSmall,
    /// A tag's declared end passes total_size.
    TagOverrunsTotal,
    /// Walked to total_size without the type-0 end tag.
    MissingEndTag,
    /// Memory-map tag: entry_size undercuts {base, len, type, reserved} or
    /// does not divide the entries region.
    InvalidEntrySize,
    /// Memory-map tag: entry_version is not 0 (an unknown future format).
    InvalidEntryVersion,
    /// A tag's body undercuts the fixed fields its type requires (an RSDP
    /// tag smaller than its pointer, an end tag not exactly 8 bytes).
    InvalidTagBody,
    /// More memory-map entries than Facts carries (live GRUB/q35 reports
    /// 9; MAX_RANGES is the headroom).
    TooManyRanges,
}

/// Memory-range kinds (the mb2 tag mirrors EFI_MEMORY_DESCRIPTOR types;
/// spec 3.9.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemKind {
    Available,
    Reserved,
    /// Type 3: ACPI reclaimable — stays OUT of the free pool; 13-04 walks
    /// the tables that live there.
    AcpiReclaimable,
    AcpiNvs,
    Bad,
    Unknown(u32),
}

impl MemKind {
    fn from_raw(raw: u32) -> MemKind {
        match raw {
            1 => MemKind::Available,
            2 => MemKind::Reserved,
            3 => MemKind::AcpiReclaimable,
            4 => MemKind::AcpiNvs,
            5 => MemKind::Bad,
            other => MemKind::Unknown(other),
        }
    }

    /// The print name for the q35 fact lines.
    pub fn name(self) -> &'static str {
        match self {
            MemKind::Available => "available",
            MemKind::Reserved => "reserved",
            MemKind::AcpiReclaimable => "acpi-reclaimable",
            MemKind::AcpiNvs => "acpi-nvs",
            MemKind::Bad => "bad",
            MemKind::Unknown(_) => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemRange {
    pub base: u64,
    pub len: u64,
    pub kind: MemKind,
}

/// Headroom over the 9 entries live GRUB/q35 reports.
pub const MAX_RANGES: usize = 32;

/// The parsed facts main boots from: the memory map and the optional RSDP
/// physical pointer.  Only type-1 (available) ranges ever seed the page
/// allocator; everything else stays reserved (T-13-04).
#[derive(Debug, PartialEq)]
pub struct Facts {
    ranges: [MemRange; MAX_RANGES],
    nranges: usize,
    pub rsdp: Option<u64>,
}

impl Facts {
    const fn empty() -> Facts {
        Facts {
            ranges: [MemRange { base: 0, len: 0, kind: MemKind::Available }; MAX_RANGES],
            nranges: 0,
            rsdp: None,
        }
    }

    fn push(&mut self, range: MemRange) -> Result<(), ParseError> {
        if self.nranges == MAX_RANGES {
            return Err(ParseError::TooManyRanges);
        }
        self.ranges[self.nranges] = range;
        self.nranges += 1;
        Ok(())
    }

    pub fn nranges(&self) -> usize {
        self.nranges
    }

    /// The ranges in firmware order (GRUB passes the BIOS e820 order
    /// through; the map is not sorted by address).
    pub fn ranges(&self) -> impl Iterator<Item = MemRange> {
        self.ranges[..self.nranges].iter().copied()
    }

    /// The type-1 (available) ranges only — the page-allocator seed set.
    pub fn available_ranges(&self) -> impl Iterator<Item = MemRange> {
        self.ranges().filter(|r| r.kind == MemKind::Available)
    }
}

/// The fixed prefix size ({total_size, reserved}) — pass 1 of the
/// two-pass boot construction reads exactly this much.
pub const MB2_PREFIX_SIZE: usize = 8;

fn validate_total_size(total: usize) -> Result<(), ParseError> {
    if total < MB2_PREFIX_SIZE + 8 || total % 8 != 0 {
        return Err(ParseError::InvalidTotalSize);
    }
    Ok(())
}

/// Pass 1 of the two-pass construction (the port/fdt.rs discipline): read
/// total_size from the prefix so the boot caller can form the full slice.
/// `prefix` must hold at least MB2_PREFIX_SIZE bytes.
pub fn header_total_size(prefix: &[u8]) -> Result<usize, ParseError> {
    // RED stub (TDD, 13-02 Task 2): fails loudly so the golden test fails
    // on its planned assertion; the fdt.rs-discipline walk lands with the
    // feat commit (Task 1's main9-x64 arc).
    let _ = (prefix, &validate_total_size);
    Err(ParseError::BufferTooSmall)
}

/// Pass 2: parse the full info structure.  `data` must be exactly
/// total_size long — the boot caller forms it via header_total_size, and
/// a disagreement is a named error, never a silent truncation.
pub fn parse(data: &[u8]) -> Result<Facts, ParseError> {
    // RED stub (TDD, 13-02 Task 2): every parse fails loudly so the five
    // behavior tests fail on their planned assertions.
    let _ = data;
    Err(ParseError::BufferTooSmall)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic-structure builder: tags land 8-aligned with spec padding
    /// after unpadded sizes — the live-GRUB shape.  total_size is patched
    /// in finish() once the end tag is in.
    struct Mb2Builder {
        buf: Vec<u8>,
    }

    impl Mb2Builder {
        fn new() -> Mb2Builder {
            Mb2Builder { buf: vec![0; MB2_PREFIX_SIZE] }
        }

        fn tag(&mut self, ty: u32, body: &[u8]) {
            let size = 8 + body.len();
            self.buf.extend_from_slice(&ty.to_le_bytes());
            self.buf.extend_from_slice(&(size as u32).to_le_bytes());
            self.buf.extend_from_slice(body);
            while self.buf.len() % 8 != 0 {
                self.buf.push(0);
            }
        }

        fn finish(mut self) -> Vec<u8> {
            self.tag(TAG_END, &[]);
            let total = self.buf.len();
            self.buf[0..4].copy_from_slice(&(total as u32).to_le_bytes());
            self.buf
        }
    }

    fn mmap_body(entry_size: u32, version: u32, entries: &[(u64, u64, u32)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&entry_size.to_le_bytes());
        body.extend_from_slice(&version.to_le_bytes());
        for (base, len, kind) in entries {
            body.extend_from_slice(&base.to_le_bytes());
            body.extend_from_slice(&len.to_le_bytes());
            body.extend_from_slice(&kind.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes());
        }
        body
    }

    // Test 1: the golden memory map — a well-formed mmap tag parses to
    // exactly its entries with types preserved.
    #[test]
    fn golden_memory_map_parses_ranges_with_types() {
        let mut b = Mb2Builder::new();
        b.tag(
            TAG_MMAP,
            &mmap_body(
                24,
                0,
                &[(0x1000000, 0x3ff00000, 1), (0x0, 0x8000, 2)],
            ),
        );
        let buf = b.finish();

        // Pass 1 of the boot construction on the same buffer.
        assert_eq!(header_total_size(&buf).unwrap(), buf.len());

        let facts = parse(&buf).expect("golden buffer must parse");
        assert_eq!(facts.nranges(), 2);
        let ranges: Vec<MemRange> = facts.ranges().collect();
        assert_eq!(
            ranges[0],
            MemRange { base: 0x1000000, len: 0x3ff00000, kind: MemKind::Available }
        );
        assert_eq!(
            ranges[1],
            MemRange { base: 0x0, len: 0x8000, kind: MemKind::Reserved }
        );
        // available_ranges() yields only the type-1 seed set.
        assert_eq!(facts.available_ranges().count(), 1);
    }

    // Test 2: the end-tag walk — iteration stops exactly at a well-formed
    // type-0 end tag, and a total_size that cuts a tag short is a loud
    // named error, not a partial parse.
    #[test]
    fn end_tag_stops_walk_and_cut_total_size_is_loud() {
        // (a) padding bytes after the end tag (inside total_size) are
        // never interpreted: the walk stops at type 0 exactly.
        let mut b = Mb2Builder::new();
        b.tag(2, b"GRUB 2.14\0");
        let mut buf = b.finish();
        buf.extend_from_slice(&[0u8; 8]);
        let total = buf.len() as u32;
        buf[0..4].copy_from_slice(&total.to_le_bytes());
        let facts = parse(&buf).expect("walk must stop at the end tag");
        assert_eq!(facts.nranges(), 0);

        // (b) a tag whose declared end passes total_size.
        let mut cut = vec![0u8; 0x20];
        cut[0..4].copy_from_slice(&0x20u32.to_le_bytes());
        // tag at 8: {type 1, size 9} — the live empty-cmdline shape.
        cut[8..12].copy_from_slice(&1u32.to_le_bytes());
        cut[12..16].copy_from_slice(&9u32.to_le_bytes());
        // tag at 0x18 (align_up(8+9)): header fits, body does not.
        cut[0x18..0x1c].copy_from_slice(&6u32.to_le_bytes());
        cut[0x1c..0x20].copy_from_slice(&0x10u32.to_le_bytes());
        assert_eq!(parse(&cut), Err(ParseError::TagOverrunsTotal));
    }

    // Test 3: alignment and header loudness — a tag header smaller than 8
    // bytes and a misaligned total_size are loud named errors, while
    // unpadded (non-8-multiple) tag SIZES are the spec's tag padding and
    // walk correctly: GRUB 2.14's live structure on this channel carries
    // cmdline size 9 and boot-loader-name size 18 (ground-truth hex dump,
    // 13-02).  A size%8 check would reject the loader's own output.
    #[test]
    fn alignment_and_header_violations_are_loud() {
        // (a) tag header smaller than 8 bytes.
        let mut tiny = vec![0u8; 0x10];
        tiny[0..4].copy_from_slice(&0x10u32.to_le_bytes());
        tiny[8..12].copy_from_slice(&6u32.to_le_bytes());
        tiny[12..16].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(parse(&tiny), Err(ParseError::TagHeaderTooSmall));

        // (b) the live-GRUB shape: unpadded sizes walk via 8-aligned
        // advance, and the mmap tag behind them still parses.
        let mut b = Mb2Builder::new();
        b.tag(1, b""); // size 9 — observed live
        b.tag(2, b"GRUB 2.14\0"); // size 18 — observed live
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x100000, 0x3fedf000, 1)]));
        let facts = parse(&b.finish()).expect("live GRUB shape must parse");
        assert_eq!(facts.nranges(), 1);

        // (c) total_size not 8-aligned.
        let mut mis = vec![0u8; 0x1c];
        mis[0..4].copy_from_slice(&0x1cu32.to_le_bytes());
        assert_eq!(parse(&mis), Err(ParseError::InvalidTotalSize));

        // (d) mmap fixed fields: entry_size under 24, entry_version not
        // 0, and an entries region the entry_size does not divide.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(16, 0, &[(0, 0x1000, 1)]));
        assert_eq!(parse(&b.finish()), Err(ParseError::InvalidEntrySize));
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 1, &[(0, 0x1000, 1)]));
        assert_eq!(parse(&b.finish()), Err(ParseError::InvalidEntryVersion));
        // entry_size 25 does not divide the (padded) entries region.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(25, 0, &[(0, 0x1000, 1)]));
        assert_eq!(parse(&b.finish()), Err(ParseError::InvalidEntrySize));
    }

    // Test 4: the RSDP tags — a type-15 (EFI64) tag yields the pointer,
    // type 14 (EFI32) the old one, tag 15 wins when both ride, and a
    // BIOS boot carrying neither yields None (the absence 13-04 handles
    // loudly at boot).
    #[test]
    fn rsdp_tags_yield_pointer_and_absence_yields_none() {
        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_NEW_RSDP, &0xdead_beef_0000_1000u64.to_le_bytes());
        let facts = parse(&b.finish()).unwrap();
        assert_eq!(facts.rsdp, Some(0xdead_beef_0000_1000));

        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_OLD_RSDP, &0xbfbf_f000u32.to_le_bytes());
        let facts = parse(&b.finish()).unwrap();
        assert_eq!(facts.rsdp, Some(0xbfbf_f000));

        // 15 is preferred when both ride, in either order.
        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_OLD_RSDP, &0xbfbf_f000u32.to_le_bytes());
        b.tag(TAG_ACPI_NEW_RSDP, &0xdead_beef_0000_1000u64.to_le_bytes());
        assert_eq!(parse(&b.finish()).unwrap().rsdp, Some(0xdead_beef_0000_1000));
        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_NEW_RSDP, &0xdead_beef_0000_1000u64.to_le_bytes());
        b.tag(TAG_ACPI_OLD_RSDP, &0xbfbf_f000u32.to_le_bytes());
        assert_eq!(parse(&b.finish()).unwrap().rsdp, Some(0xdead_beef_0000_1000));

        // Neither tag (the BIOS-boot case): None.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x100000, 0x3fedf000, 1)]));
        assert_eq!(parse(&b.finish()).unwrap().rsdp, None);
    }

    // Test 5: truncation safety — every buffer cut mid-structure errors
    // loudly by name; never a panic, never a partial Facts.
    #[test]
    fn every_truncation_errors_loudly() {
        let mut b = Mb2Builder::new();
        b.tag(1, b""); // size 9 — the unpadded live shape
        b.tag(
            TAG_MMAP,
            &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1), (0x0, 0x8000, 2)]),
        );
        b.tag(TAG_ACPI_NEW_RSDP, &0xdead_beef_0000_1000u64.to_le_bytes());
        let buf = b.finish();
        assert!(parse(&buf).is_ok());
        for cut in 0..buf.len() {
            assert!(
                parse(&buf[..cut]).is_err(),
                "cut at {cut} must be a loud named error"
            );
        }
    }
}
