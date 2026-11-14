// The mb2 info-structure parser — the /chosen-and-memory analog of
// port/fdt.rs: heap-less, bounds-checked, error-typed.
//
// Structure (multiboot2 spec 3.2; x86 native little-endian):
//   { u32 total_size; u32 reserved; tags... }
// Tags are 8-byte aligned, each { u32 type; u32 size } with size counting
// the 8-byte header; the walk advances 8-aligned and terminates at the
// type-0 end tag.  The memory map rides tag 6; the boot modules ride tag 3
// (one tag per module, may repeat — D-03's bootfs delivery); the ACPI
// tags 14/15 embed the RSDP structure itself — the root-table address
// inside it is the loader-mediated platform-data source per D-03's theme
// (13-04 walks the table from there; a boot whose map carries neither tag
// is the loud-absence case the EBDA scan covers).
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
//
// 19-01 (CPU-02): the cmdline tag (type 1) joins the consumed set — the
// x86_64 /chosen/bootargs analog.  GRUB's `-append` (and the grub.cfg
// append seam the ISO builders emit) lands its string there, the kernel
// parses it into key=value bootconf pairs (parse_bootconf_pairs — the
// donor plan9.ini shape, one line), and main's exec prologue merges
// those pairs into the seeded ROOT env group BEFORE boot(8) runs: the
// ONE mechanism replacing 9k's #ec config env, image-scoped per the
// ISO's cmdline (an absent tag = zero pairs = the pre-19-01 boot
// byte-for-byte).  The live empty-cmdline shape is the observed size-9
// tag (header + one NUL) — a real boot always carries the tag, empty.

/// A memory-map entry's fixed size: {u64 base, u64 len, u32 type, u32 res}.
const MMAP_ENTRY_SIZE: usize = 24;

/// The tag types this parser consumes (spec 3.2.2).
const TAG_END: u32 = 0;
const TAG_CMDLINE: u32 = 1;
const TAG_MODULE: u32 = 3;
const TAG_MMAP: u32 = 6;
const TAG_ACPI_OLD_RSDP: u32 = 14;
const TAG_ACPI_NEW_RSDP: u32 = 15;

/// The cmdline string's hard cap — a bootconf longer than 256 bytes is
/// rejected LOUDLY at parse, never silently truncated (the cmdline is
/// host-controlled compose-time input; an overlong line means a
/// mis-staged ISO, and first boot is the cheapest place to say so).
pub const MAX_CMDLINE: usize = 256;

/// The bootconf pair cap — defensive headroom over the one- or two-pair
/// lines the images actually carry (init= alone today); exceeded is a
/// loud named error like every other malformed-input class here.
// Consumed only under dev_exec (main's exec prologue); the dead-code
// allowance is the `mod exec` precedent — the host tests exercise this
// from every config.
#[allow(dead_code)]
pub const MAX_BOOTCONF_PAIRS: usize = 32;

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
    /// A tag's body undercuts the fixed fields its type requires or fails
    /// their validation (an RSDP tag smaller than the structure it embeds
    /// or with a bad "RSD PTR " signature, an end tag not exactly 8
    /// bytes).
    InvalidTagBody,
    /// More memory-map entries than Facts carries (live GRUB/q35 reports
    /// 9; MAX_RANGES is the headroom).
    TooManyRanges,
    /// Module tag (type 3): the body undercuts the fixed {mod_start,
    /// mod_end} physical-address pair (a truncated tag — never a
    /// partial-read module).
    InvalidModuleBody,
    /// Module tag: mod_end does not name a non-empty range above
    /// mod_start (inverted or empty).
    InvalidModuleRange,
    /// Module tag: the physical range is not contained in one AVAILABLE
    /// memory-map range — checked at parse completion (the mmap tag may
    /// walk after the module tag), BEFORE any consumer maps the range
    /// (T-14-08: GRUB allocates modules from its own available-RAM heap,
    /// so anything else is malformed firmware input).
    ModuleNotInMemoryMap,
    /// More module tags than Facts carries (the composed ISO stages one
    /// bootfs.paq; MAX_MODULES is the headroom).
    TooManyModules,
    /// Cmdline tag (type 1): the declared tag body carries no NUL inside
    /// it — the spec's string is NUL-terminated within the tag, so
    /// anything else is a truncated tag, never a partial string.
    InvalidCmdlineTag,
    /// Cmdline tag: the string is longer than MAX_CMDLINE.  A loud
    /// reject, never a silent truncate — the bootconf is host-controlled
    /// and an overlong line means a mis-staged ISO (T-19-01).
    CmdlineTooLong,
    /// Bootconf pair parse: a whitespace token with no '=' arrives
    /// before any pair it could continue, or carries an empty key —
    /// malformed bootconf input, loud at first boot.
    #[allow(dead_code)] // dev_exec-only consumers (main's prologue); tests pin it
    InvalidBootconfToken,
    /// More key=value pairs than the bootconf carries room for
    /// (MAX_BOOTCONF_PAIRS).
    #[allow(dead_code)] // dev_exec-only consumers (main's prologue); tests pin it
    TooManyBootconfPairs,
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

/// The ACPI root-table fact the mb2 ACPI tags carry: GRUB embeds the
/// RSDP structure ITSELF in the tag (spec 3.9.6/3.9.7 — "a copy of the
/// RSDP", not a pointer to it), so the extractable fact is the root
/// table address inside the structure: tag 14 embeds the v1 shape
/// (rsdt_address at structure offset 16), tag 15 the v2 shape
/// (xsdt_address at offset 24).  13-04's RSDP→XSDT walk prefers the
/// xsdt arm; the "RSD PTR " signature is validated before any address
/// is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiRoot {
    /// The RSDT (tag 14, v1) or XSDT (tag 15, v2) physical address.
    pub table: u64,
    /// True when `table` is an XSDT (64-bit tables — the preferred
    /// D-07 walk); false when it is an RSDT.
    pub xsdt: bool,
}

/// A boot module the loader delivered (mb2 tag 3, D-03): the physical
/// byte range [start, end) GRUB placed the file at — u32 PHYSICAL
/// addresses the loader chose (no fixed load address; the aarch64
/// fixed-addr `-device loader` delivery is deliberately not carried).
/// The kernel consumes the range through its KZERO alias of all RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    pub start: u64,
    pub end: u64,
}

/// Headroom over the one module the composed ISO stages (bootfs.paq).
pub const MAX_MODULES: usize = 4;

/// The parsed facts main boots from: the memory map, the boot modules,
/// the cmdline string (19-01), and the optional ACPI root-table address.
/// Only type-1 (available) ranges ever seed the page allocator;
/// everything else stays reserved (T-13-04).
#[derive(Debug, PartialEq)]
pub struct Facts {
    ranges: [MemRange; MAX_RANGES],
    nranges: usize,
    modules: [Module; MAX_MODULES],
    nmodules: usize,
    cmdline: [u8; MAX_CMDLINE],
    ncmdline: usize,
    pub acpi_root: Option<AcpiRoot>,
}

impl Facts {
    const fn empty() -> Facts {
        Facts {
            ranges: [MemRange { base: 0, len: 0, kind: MemKind::Available }; MAX_RANGES],
            nranges: 0,
            modules: [Module { start: 0, end: 0 }; MAX_MODULES],
            nmodules: 0,
            cmdline: [0; MAX_CMDLINE],
            ncmdline: 0,
            acpi_root: None,
        }
    }

    /// The cmdline tag's string, NUL stripped — empty when the boot
    /// carried no tag or the live empty-tag shape (size 9).  The bytes
    /// borrow Facts (copied at parse), so the seed merge in main can
    /// hold the slices through the prologue.
    #[allow(dead_code)] // dev_exec-only consumers (main's prologue); tests pin it
    pub fn cmdline(&self) -> &[u8] {
        &self.cmdline[..self.ncmdline]
    }

    fn push(&mut self, range: MemRange) -> Result<(), ParseError> {
        if self.nranges == MAX_RANGES {
            return Err(ParseError::TooManyRanges);
        }
        self.ranges[self.nranges] = range;
        self.nranges += 1;
        Ok(())
    }

    fn push_module(&mut self, module: Module) -> Result<(), ParseError> {
        if self.nmodules == MAX_MODULES {
            return Err(ParseError::TooManyModules);
        }
        self.modules[self.nmodules] = module;
        self.nmodules += 1;
        Ok(())
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

    /// The boot modules in firmware order (tags may repeat — one tag
    /// per module; the composed ISO stages bootfs.paq alone).
    pub fn modules(&self) -> impl Iterator<Item = Module> {
        self.modules[..self.nmodules].iter().copied()
    }

    /// D-03/T-14-08: every collected module range is bounds-checked
    /// against the parsed memory map at parse completion — BEFORE any
    /// consumer maps it.  GRUB allocates modules from its own
    /// available-RAM heap, so a range outside one AVAILABLE map entry
    /// is malformed firmware input: a loud named error, never a wild
    /// map.  (checked_add: an overflowing map entry contains nothing.)
    fn validate_modules(&self) -> Result<(), ParseError> {
        for m in self.modules() {
            let contained = self.available_ranges().any(|r| match r.base.checked_add(r.len) {
                Some(top) => r.base <= m.start && m.end <= top,
                None => false,
            });
            if !contained {
                return Err(ParseError::ModuleNotInMemoryMap);
            }
        }
        Ok(())
    }
}

/// The fixed prefix size ({total_size, reserved}) — pass 1 of the
/// two-pass boot construction reads exactly this much.
pub const MB2_PREFIX_SIZE: usize = 8;

fn validate_total_size(total: usize) -> Result<(), ParseError> {
    if total < MB2_PREFIX_SIZE + 8 || !total.is_multiple_of(8) {
        return Err(ParseError::InvalidTotalSize);
    }
    Ok(())
}

/// Pass 1 of the two-pass construction (the port/fdt.rs discipline): read
/// total_size from the prefix so the boot caller can form the full slice.
/// `prefix` must hold at least MB2_PREFIX_SIZE bytes.
pub fn header_total_size(prefix: &[u8]) -> Result<usize, ParseError> {
    let total = le_u32(prefix, 0).ok_or(ParseError::BufferTooSmall)? as usize;
    validate_total_size(total)?;
    Ok(total)
}

/// Pass 2: parse the full info structure.  `data` must be exactly
/// total_size long — the boot caller forms it via header_total_size, and
/// a disagreement is a named error, never a silent truncation.
pub fn parse(data: &[u8]) -> Result<Facts, ParseError> {
    let total = header_total_size(data)?;
    if total != data.len() {
        return Err(ParseError::InvalidTotalSize);
    }
    let mut facts = Facts::empty();
    let mut off = MB2_PREFIX_SIZE;
    loop {
        let ty = le_u32(data, off).ok_or(ParseError::MissingEndTag)?;
        let size = le_u32(data, off + 4).ok_or(ParseError::MissingEndTag)? as usize;
        if size < 8 {
            return Err(ParseError::TagHeaderTooSmall);
        }
        if off + size > total {
            return Err(ParseError::TagOverrunsTotal);
        }
        match ty {
            TAG_END => {
                // The spec fixes the end tag at exactly 8 bytes.
                if size != 8 {
                    return Err(ParseError::InvalidTagBody);
                }
                // D-03's trust boundary (T-14-08): every collected module
                // range is bounds-checked against the (now fully parsed)
                // memory map before the walk hands Facts to any consumer.
                facts.validate_modules()?;
                return Ok(facts);
            }
            TAG_MODULE => parse_module(data, off, size, &mut facts)?,
            TAG_CMDLINE => parse_cmdline(data, off, size, &mut facts)?,
            TAG_MMAP => parse_mmap(data, off, size, &mut facts)?,
            TAG_ACPI_OLD_RSDP | TAG_ACPI_NEW_RSDP => {
                let root = parse_acpi_root(data, off, size, ty)?;
                // Tag 15 (the v2/XSDT shape) is the preferred source; a
                // tag 14 never displaces it.
                if ty == TAG_ACPI_NEW_RSDP || facts.acpi_root.is_none() {
                    facts.acpi_root = Some(root);
                }
            }
            // Unknown tag types walk past untouched — size and bounds
            // were validated above, so this is the spec's forward
            // compatibility, not a silent skip of a malformed tag.
            _ => {}
        }
        // Next tag at the 8-aligned advance (spec 3.2: tags are padded to
        // alignment; the live dump proved sizes themselves are not
        // 8-multiples).
        off = (off + size + 7) & !7;
        if off + 8 > total {
            // No room for another tag header: the end tag never came.
            return Err(ParseError::MissingEndTag);
        }
    }
}

/// The ACPI RSDP tags: the tag body IS the RSDP structure ("a copy of
/// the RSDP", spec 3.9.6/3.9.7 — not a pointer to it).  Signature
/// validated, then the root-table address taken: v1 (tag 14) carries
/// rsdt_address (u32) at structure offset 16; v2 (tag 15) carries
/// xsdt_address (u64) at offset 24.
fn parse_acpi_root(data: &[u8], off: usize, size: usize, ty: u32) -> Result<AcpiRoot, ParseError> {
    // Signature "RSD PTR " — the 8 bytes every RSDP revision starts with.
    for (k, want) in [b'R', b'S', b'D', b' ', b'P', b'T', b'R', b' '].into_iter().enumerate() {
        let got = data.get(off + 8 + k).copied().ok_or(ParseError::InvalidTagBody)?;
        if got != want {
            return Err(ParseError::InvalidTagBody);
        }
    }
    if ty == TAG_ACPI_NEW_RSDP {
        if size < 8 + 36 {
            return Err(ParseError::InvalidTagBody);
        }
        let xsdt = le_u64(data, off + 8 + 24).ok_or(ParseError::InvalidTagBody)?;
        Ok(AcpiRoot { table: xsdt, xsdt: true })
    } else {
        if size < 8 + 20 {
            return Err(ParseError::InvalidTagBody);
        }
        let rsdt = le_u32(data, off + 8 + 16).ok_or(ParseError::InvalidTagBody)? as u64;
        Ok(AcpiRoot { table: rsdt, xsdt: false })
    }
}

/// The module tag (type 3, D-03): {u32 mod_start, u32 mod_end, u8
/// cmdline[]} — mod_start/mod_end are 32-bit PHYSICAL addresses of the
/// range GRUB placed the module file at ([start, end)), the cmdline a
/// NUL-terminated string the loader appends (unused here).  One tag
/// per module and tags may repeat: each is collected; the range check
/// against the memory map runs at parse completion (validate_modules).
fn parse_module(data: &[u8], off: usize, size: usize, facts: &mut Facts) -> Result<(), ParseError> {
    if size < 16 {
        return Err(ParseError::InvalidModuleBody);
    }
    let start = le_u32(data, off + 8).ok_or(ParseError::InvalidModuleBody)? as u64;
    let end = le_u32(data, off + 12).ok_or(ParseError::InvalidModuleBody)? as u64;
    if end <= start {
        return Err(ParseError::InvalidModuleRange);
    }
    facts.push_module(Module { start, end })
}

/// The cmdline tag (type 1, 19-01/CPU-02): {u8 string[], NUL} — the
/// NUL-terminated kernel command line the loader appended (GRUB
/// `-append` / the grub.cfg append seam).  The string is the bytes up
/// to the FIRST NUL, which must land inside the declared tag (the live
/// empty shape is size 9 — header + NUL alone); past the NUL the tag
/// may carry padding the spec's alignment rules on.  Copied into
/// Facts's fixed buffer (capped by MAX_CMDLINE — overlong is loud,
/// T-19-01).  A repeated cmdline tag keeps the FIRST copy (the spec
/// fixes exactly one; a repeat is loader weirdness, and deterministic
/// first-wins beats last-wins on input this early).
fn parse_cmdline(data: &[u8], off: usize, size: usize, facts: &mut Facts) -> Result<(), ParseError> {
    // The NUL must sit inside the declared body [off+8, off+size).
    let body = data.get(off + 8..off + size).ok_or(ParseError::InvalidCmdlineTag)?;
    let nul = body.iter().position(|&b| b == 0).ok_or(ParseError::InvalidCmdlineTag)?;
    if facts.ncmdline != 0 {
        return Ok(()); // first tag wins on repeats
    }
    let string = &body[..nul];
    let Some(buf) = facts.cmdline.get_mut(..string.len()) else {
        return Err(ParseError::CmdlineTooLong);
    };
    buf.copy_from_slice(string);
    facts.ncmdline = string.len();
    Ok(())
}

/// A parsed bootconf: the cmdline's key=value pairs, key and value
/// slices BORROWED from the cmdline string (no allocation — the same
/// heap-less discipline as Facts).  Consumed by main's exec prologue to
/// merge into the seeded ROOT env group.
// dev_exec-only consumer (main's prologue); the dead-code allowance is
// the `mod exec` precedent — the host tests exercise it from every
// config.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Bootconf<'a> {
    pairs: [(&'a [u8], &'a [u8]); MAX_BOOTCONF_PAIRS],
    npairs: usize,
}

#[allow(dead_code)]
impl<'a> Bootconf<'a> {
    fn empty() -> Bootconf<'a> {
        Bootconf { pairs: [(&[], &[]); MAX_BOOTCONF_PAIRS], npairs: 0 }
    }

    /// The parsed pairs in cmdline order.
    pub fn pairs(&self) -> impl Iterator<Item = (&'a [u8], &'a [u8])> {
        self.pairs[..self.npairs].iter().copied()
    }

    /// True when the cmdline carried no pairs (the absent/empty case —
    /// the pre-19-01 boot, zero seed changes).
    pub fn is_empty(&self) -> bool {
        self.npairs == 0
    }
}

/// Parse a cmdline string into key=value bootconf pairs — the donor
/// plan9.ini shape, one line (19-01/CPU-02):
///
/// - tokens are whitespace-separated; a token containing '=' opens a
///   pair, split at the FIRST '=' — further '=' inside the value are
///   literal and the value runs to the token's end (the next
///   whitespace);
/// - a token WITHOUT '=' continues the previous pair's value, the
///   value's span extending to the token's end: the one-line analog of
///   plan9.ini's value-runs-to-end-of-line (the donor carries values
///   with spaces — `init=/amd64/init -c` is exactly this shape, and
///   boot(8)'s execinit tokenizes the multi-word value verbatim,
///   sys/src/9k/boot/boot.c:208-214);
/// - a continuation with no preceding pair, or an empty key, is a loud
///   named error — malformed bootconf surfaces at first boot, never a
///   silent drop;
/// - the empty string parses to zero pairs (the no-op).
// dev_exec-only consumer (main's prologue); the dead-code allowance is
// the `mod exec` precedent — the host tests exercise it from every
// config.
#[allow(dead_code)]
pub fn parse_bootconf_pairs<'a>(cmdline: &'a [u8]) -> Result<Bootconf<'a>, ParseError> {
    // Pair spans as cmdline INDEX ranges; the borrowed slices materialize
    // only when the walk finishes (a continuation extends the previous
    // value's end index — no slice surgery mid-walk).
    let mut keys = [(0usize, 0usize); MAX_BOOTCONF_PAIRS];
    let mut vals = [(0usize, 0usize); MAX_BOOTCONF_PAIRS];
    let mut npairs = 0usize;
    let mut i = 0usize;
    while i < cmdline.len() {
        while i < cmdline.len() && cmdline[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < cmdline.len() && !cmdline[i].is_ascii_whitespace() {
            i += 1;
        }
        if i == start {
            break; // trailing whitespace only
        }
        match cmdline[start..i].iter().position(|&b| b == b'=') {
            Some(0) => return Err(ParseError::InvalidBootconfToken), // empty key ("=v")
            Some(eq) => {
                if npairs == MAX_BOOTCONF_PAIRS {
                    return Err(ParseError::TooManyBootconfPairs);
                }
                keys[npairs] = (start, start + eq);
                vals[npairs] = (start + eq + 1, i);
                npairs += 1;
            }
            None => {
                // Continuation: extend the previous value's span to this
                // token's end (the whitespace between joins into the
                // value, exactly the donor's runs-to-end-of-line).
                if npairs == 0 {
                    return Err(ParseError::InvalidBootconfToken);
                }
                vals[npairs - 1].1 = i;
            }
        }
    }
    let mut out = Bootconf::empty();
    out.npairs = npairs;
    for k in 0..npairs {
        out.pairs[k] = (&cmdline[keys[k].0..keys[k].1], &cmdline[vals[k].0..vals[k].1]);
    }
    Ok(out)
}

/// The memory-map tag (type 6): {u32 entry_size, u32 entry_version,
/// entries[]} — every field validated before any entry slice is formed.
fn parse_mmap(data: &[u8], off: usize, size: usize, facts: &mut Facts) -> Result<(), ParseError> {
    let entry_size = le_u32(data, off + 8).ok_or(ParseError::InvalidTagBody)? as usize;
    let entry_version = le_u32(data, off + 12).ok_or(ParseError::InvalidTagBody)?;
    if entry_size < MMAP_ENTRY_SIZE {
        return Err(ParseError::InvalidEntrySize);
    }
    if entry_version != 0 {
        return Err(ParseError::InvalidEntryVersion);
    }
    let body = size - 16;
    if !body.is_multiple_of(entry_size) {
        return Err(ParseError::InvalidEntrySize);
    }
    for k in 0..body / entry_size {
        let e = off + 16 + k * entry_size;
        let base = le_u64(data, e).ok_or(ParseError::InvalidEntrySize)?;
        let len = le_u64(data, e + 8).ok_or(ParseError::InvalidEntrySize)?;
        let kind = le_u32(data, e + 16).ok_or(ParseError::InvalidEntrySize)?;
        facts.push(MemRange { base, len, kind: MemKind::from_raw(kind) })?;
    }
    Ok(())
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

    /// A module-tag body: {u32 mod_start, u32 mod_end, cmdline} — the
    /// physical placement GRUB chose (the composed ISO stages bootfs.paq).
    fn module_body(start: u32, end: u32, cmdline: &str) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&start.to_le_bytes());
        body.extend_from_slice(&end.to_le_bytes());
        body.extend_from_slice(cmdline.as_bytes());
        body.push(0); // NUL
        body
    }

    // Test 4a: the module tag round-trip (D-03) — a module inside an
    // available map range parses to exactly its physical [start, end)
    // range, repeated tags collect in order (one tag per module), and a
    // boot with no module tag yields none (kernel-only ISOs).
    #[test]
    fn module_tags_collect_ranges() {
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1)]));
        b.tag(TAG_MODULE, &module_body(0x4000000, 0x4001000, "/boot/bootfs.paq"));
        b.tag(TAG_MODULE, &module_body(0x5000000, 0x5000800, "other"));
        let facts = parse(&b.finish()).expect("module tags must parse");
        let mods: Vec<Module> = facts.modules().collect();
        assert_eq!(
            mods,
            vec![
                Module { start: 0x4000000, end: 0x4001000 },
                Module { start: 0x5000000, end: 0x5000800 },
            ]
        );

        // The module tag may also walk BEFORE the mmap tag — the range
        // check runs at parse completion, not collection time.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MODULE, &module_body(0x4000000, 0x4001000, "/boot/bootfs.paq"));
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1)]));
        assert_eq!(
            parse(&b.finish()).unwrap().modules().collect::<Vec<_>>(),
            vec![Module { start: 0x4000000, end: 0x4001000 }]
        );

        // Kernel-only boot: no module tag, no modules.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1)]));
        assert_eq!(parse(&b.finish()).unwrap().modules().count(), 0);
    }

    // Test 4b: the module tag's malformed cases are loud named errors,
    // never a partial module — a body under the fixed {mod_start,
    // mod_end} pair, an inverted/empty range, and (T-14-08) a range
    // outside every AVAILABLE memory-map entry.
    #[test]
    fn module_tag_violations_are_loud() {
        // Truncated body: size 12 undercuts the 16-byte fixed minimum.
        // (Hand-built: the builder would pad the body to spec shape.)
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1)]));
        b.tag(TAG_MODULE, &[0u8; 4]);
        assert_eq!(parse(&b.finish()), Err(ParseError::InvalidModuleBody));

        // Inverted range (end below start) and the empty range.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1)]));
        b.tag(TAG_MODULE, &module_body(0x4001000, 0x4000000, "x"));
        assert_eq!(parse(&b.finish()), Err(ParseError::InvalidModuleRange));
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1)]));
        b.tag(TAG_MODULE, &module_body(0x4000000, 0x4000000, "x"));
        assert_eq!(parse(&b.finish()), Err(ParseError::InvalidModuleRange));

        // Out-of-mmap range: inside a RESERVED entry, above every entry,
        // and straddling the available entry's end (top = 0x40f00000) —
        // none is contained in one AVAILABLE range, so all three refuse
        // before any mapping.
        for (start, end) in [(0x1000u32, 0x2000u32), (0x60000000, 0x60001000), (0x40f00000 - 0x800, 0x40f00800)] {
            let mut b = Mb2Builder::new();
            b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1), (0x0, 0x8000, 2)]));
            b.tag(TAG_MODULE, &module_body(start, end, "x"));
            assert_eq!(parse(&b.finish()), Err(ParseError::ModuleNotInMemoryMap));
        }

        // More module tags than Facts carries.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1)]));
        for _ in 0..(MAX_MODULES + 1) {
            b.tag(TAG_MODULE, &module_body(0x4000000, 0x4001000, "x"));
        }
        assert_eq!(parse(&b.finish()), Err(ParseError::TooManyModules));
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
        assert_eq!(facts.ranges().count(), 2);
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
        assert_eq!(facts.ranges().count(), 0);

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
        // advance, and the mmap tag behind them still parses.  The
        // cmdline tag carries its NUL (size 9 — observed live; 19-01
        // made the tag a consumed fact).
        let mut b = Mb2Builder::new();
        b.tag(1, b"\0"); // size 9 — observed live
        b.tag(2, b"GRUB 2.14\0"); // size 18 — observed live
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x100000, 0x3fedf000, 1)]));
        let facts = parse(&b.finish()).expect("live GRUB shape must parse");
        assert_eq!(facts.ranges().count(), 1);

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

    // Test 4: the ACPI tags — the tag body IS the RSDP structure (spec
    // 3.9.6/3.9.7, confirmed live: a naive pointer read yields the
    // "RSD " signature bytes).  A type-15 tag yields the embedded v2
    // structure's XSDT address, a type-14 tag the v1 RSDT address, tag
    // 15 wins when both ride, a bad signature is a loud named error,
    // and a boot carrying neither tag yields None (the absence 13-04
    // handles loudly via the EBDA scan).
    fn rsdp_v1_body(rsdt: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(b"RSD PTR "); // signature
        body.push(0); // checksum
        body.extend_from_slice(b"BOCHS "); // OEMID (6)
        body.push(0); // revision (0 = v1)
        body.extend_from_slice(&rsdt.to_le_bytes());
        body
    }

    fn rsdp_v2_body(xsdt: u64) -> Vec<u8> {
        let mut body = rsdp_v1_body(0);
        body.extend_from_slice(&36u32.to_le_bytes()); // length
        body.extend_from_slice(&xsdt.to_le_bytes());
        body.extend_from_slice(&[0u8; 4]); // ext checksum + reserved
        body
    }

    #[test]
    fn acpi_tags_yield_root_table_and_absence_yields_none() {
        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_NEW_RSDP, &rsdp_v2_body(0xdead_beef_0000_1000));
        let facts = parse(&b.finish()).unwrap();
        assert_eq!(
            facts.acpi_root,
            Some(AcpiRoot { table: 0xdead_beef_0000_1000, xsdt: true })
        );

        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_OLD_RSDP, &rsdp_v1_body(0xbfbf_f000));
        let facts = parse(&b.finish()).unwrap();
        assert_eq!(facts.acpi_root, Some(AcpiRoot { table: 0xbfbf_f000, xsdt: false }));

        // 15 is preferred when both ride, in either order.
        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_OLD_RSDP, &rsdp_v1_body(0xbfbf_f000));
        b.tag(TAG_ACPI_NEW_RSDP, &rsdp_v2_body(0xdead_beef_0000_1000));
        assert_eq!(
            parse(&b.finish()).unwrap().acpi_root,
            Some(AcpiRoot { table: 0xdead_beef_0000_1000, xsdt: true })
        );
        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_NEW_RSDP, &rsdp_v2_body(0xdead_beef_0000_1000));
        b.tag(TAG_ACPI_OLD_RSDP, &rsdp_v1_body(0xbfbf_f000));
        assert_eq!(
            parse(&b.finish()).unwrap().acpi_root,
            Some(AcpiRoot { table: 0xdead_beef_0000_1000, xsdt: true })
        );

        // A tag whose embedded structure does not carry the RSDP
        // signature is a loud named error, never a bogus address.
        let mut b = Mb2Builder::new();
        b.tag(TAG_ACPI_OLD_RSDP, &[0u8; 20]);
        assert_eq!(parse(&b.finish()), Err(ParseError::InvalidTagBody));

        // Neither tag: None.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x100000, 0x3fedf000, 1)]));
        assert_eq!(parse(&b.finish()).unwrap().acpi_root, None);
    }

    // Test 6 (19-01, CPU-02): the cmdline tag round-trips into Facts and
    // the bootconf pair parse carries the donor plan9.ini shape — the
    // empty tag is a no-op, two pairs parse, an embedded '=' rides the
    // value literally, and the multi-word init= shape (the cpu image's
    // bootconf line) keeps its flags inside the value so boot(8)'s
    // execinit tokenizes them verbatim.
    #[test]
    fn cmdline_tag_parses_and_bootconf_pairs_merge() {
        // The live empty-cmdline shape (size 9): empty string, zero
        // pairs — the pre-19-01 boot byte-for-byte.
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x100000, 0x3fedf000, 1)]));
        b.tag(1, b"\0");
        let facts = parse(&b.finish()).expect("empty cmdline tag must parse");
        assert_eq!(facts.cmdline(), b"");
        assert!(parse_bootconf_pairs(facts.cmdline()).unwrap().is_empty());

        // No cmdline tag at all: same no-op (kernel-only ISOs today).
        let mut b = Mb2Builder::new();
        b.tag(TAG_MMAP, &mmap_body(24, 0, &[(0x100000, 0x3fedf000, 1)]));
        let facts = parse(&b.finish()).unwrap();
        assert_eq!(facts.cmdline(), b"");

        // The real bootconf line, unpadded size (8 + 20 = 28).
        let mut b = Mb2Builder::new();
        b.tag(1, b"init=/amd64/init -c\0");
        let facts = parse(&b.finish()).expect("cmdline tag must parse");
        assert_eq!(facts.cmdline(), b"init=/amd64/init -c");
        let bootconf = parse_bootconf_pairs(facts.cmdline()).unwrap();
        assert_eq!(bootconf.pairs().collect::<Vec<_>>(), vec![(b"init".as_slice(), b"/amd64/init -c".as_slice())]);

        // Two pairs, the plain plan9.ini shape.
        let bootconf = parse_bootconf_pairs(b"cputype=amd64 nobootprompt=embed").unwrap();
        assert_eq!(
            bootconf.pairs().collect::<Vec<_>>(),
            vec![(b"cputype".as_slice(), b"amd64".as_slice()), (b"nobootprompt".as_slice(), b"embed".as_slice())]
        );

        // '=' inside a value is literal — the FIRST '=' splits, the
        // value runs to the next whitespace.
        let bootconf = parse_bootconf_pairs(b"key=a=b c=d=e").unwrap();
        assert_eq!(
            bootconf.pairs().collect::<Vec<_>>(),
            vec![(b"key".as_slice(), b"a=b".as_slice()), (b"c".as_slice(), b"d=e".as_slice())]
        );

        // Leading/trailing whitespace and the empty string: zero pairs
        // (the walk skips the runs).
        assert!(parse_bootconf_pairs(b"").unwrap().is_empty());
        assert!(parse_bootconf_pairs(b"   ").unwrap().is_empty());
        let bootconf = parse_bootconf_pairs(b"  a=1  ").unwrap();
        assert_eq!(bootconf.pairs().collect::<Vec<_>>(), vec![(b"a".as_slice(), b"1".as_slice())]);

        // A continuation after a fresh pair extends only the LAST value.
        let bootconf = parse_bootconf_pairs(b"a=1 b=2 tail").unwrap();
        assert_eq!(
            bootconf.pairs().collect::<Vec<_>>(),
            vec![(b"a".as_slice(), b"1".as_slice()), (b"b".as_slice(), b"2 tail".as_slice())]
        );
    }

    // Test 6b (19-01, T-19-01): the loud classes — an overlong cmdline
    // is rejected AT PARSE (never silently truncated), a tag without its
    // NUL is a named error, and the bootconf token/pair shapes that
    // carry no unambiguous meaning refuse loudly.
    #[test]
    fn cmdline_and_bootconf_violations_are_loud() {
        // Overlong string: 257 'a' bytes + NUL — one past MAX_CMDLINE.
        let mut over = vec![b'a'; MAX_CMDLINE + 1];
        over.push(0);
        let mut b = Mb2Builder::new();
        b.tag(1, &over);
        assert_eq!(parse(&b.finish()), Err(ParseError::CmdlineTooLong));

        // Exactly at the cap parses (256 is the boundary, not over it).
        let mut cap = vec![b'a'; MAX_CMDLINE];
        cap.push(0);
        let mut b = Mb2Builder::new();
        b.tag(1, &cap);
        assert!(parse(&b.finish()).is_ok());

        // A tag whose declared body carries no NUL at all: the builder
        // always terminates, so hand-build — tag at 8 {type 1, size
        // 0x10}, body [0x10, 0x18) all non-NUL, end tag at 0x18.
        let mut cut = vec![0u8; 0x20];
        cut[0..4].copy_from_slice(&0x20u32.to_le_bytes());
        cut[8..12].copy_from_slice(&1u32.to_le_bytes());
        cut[12..16].copy_from_slice(&0x10u32.to_le_bytes());
        for k in 0x10..0x18 {
            cut[k] = b'x';
        }
        cut[0x18..0x20].copy_from_slice(&[0, 0, 0, 0, 8, 0, 0, 0]);
        assert_eq!(parse(&cut), Err(ParseError::InvalidCmdlineTag));

        // A continuation with no preceding pair, and an empty key.
        assert_eq!(
            parse_bootconf_pairs(b"-c"),
            Err(ParseError::InvalidBootconfToken)
        );
        assert_eq!(
            parse_bootconf_pairs(b"=value"),
            Err(ParseError::InvalidBootconfToken)
        );

        // More pairs than Bootconf carries.
        let mut many = String::new();
        for k in 0..=MAX_BOOTCONF_PAIRS {
            many.push_str(&format!("k{k}=v{k} "));
        }
        assert_eq!(
            parse_bootconf_pairs(many.as_bytes()),
            Err(ParseError::TooManyBootconfPairs)
        );
    }

    // Test 5: truncation safety — every buffer cut mid-structure errors
    // loudly by name; never a panic, never a partial Facts.
    #[test]
    fn every_truncation_errors_loudly() {
        let mut b = Mb2Builder::new();
        b.tag(1, b"init=/amd64/init -c\0"); // the 19-01 bootconf carrier
        b.tag(
            TAG_MMAP,
            &mmap_body(24, 0, &[(0x1000000, 0x3ff00000, 1), (0x0, 0x8000, 2)]),
        );
        // A module inside the available range (14-05, D-03): the walk
        // must parse it AND the truncation cuts must still all be loud.
        b.tag(TAG_MODULE, &module_body(0x4000000, 0x4001000, "/boot/bootfs.paq"));
        b.tag(TAG_ACPI_NEW_RSDP, &rsdp_v2_body(0xdead_beef_0000_1000));
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
