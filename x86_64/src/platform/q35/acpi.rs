// The ACPI static-table mini-parser for qemu's q35 machine (X64-03) —
// the x86 analog of port/fdt.rs: heap-less, bounds-checked, error-typed,
// host-testable over synthetic byte fixtures.
//
// Scope (D-07): ONE parser serving both consumers — MCFG (the ECAM
// window pci.rs programs PCIEXBAR from) and MADT (the IOAPIC base+gsib
// and every IntSrcOverride 13-05's ioapic consumes) — plus the optional
// HPET base (best-effort, loud-by-absence only; timer.rs's calibration
// already works without it).  NO AML: static tables only.
//
// Discovery order (D-03's loader-mediated theme): the mb2 ACPI tags
// (14/15) embed the RSDP and 13-02's parser already signature-validated
// it and extracted the root-table address — that hint is preferred and
// this walk fully gates the root TABLE it names (signature + length +
// checksum before ANY entry pointer is derived, the 9k devacpi.c
// sdtchecksum idiom).  The EBDA/ROM scan (rsdsearch shape: the BDA
// 0x40:0x0E word to the EBDA's first KiB, then 0xE0000-0xFFFFF, both
// 16-byte-stepped, for "RSD PTR ") is the fallback — it validates the
// RSDP itself (rev>=2: 36-byte checksum + XSDT PA at offset 24; rev<2:
// 20-byte checksum + RSDT PA).
//
// Firmware tables are untrusted memory-resident data driving ECAM-window
// and interrupt-routing decisions (T-13-07): every table is verified
// signature + length + checksum BEFORE any pointer is derived from it;
// a failing table is a loud named error and NO partial facts — never a
// walk off the end, never a panic.
//
// Task 1 lands the parser ahead of its Task 2 consumer (mod.rs's
// print_machine_info + pci.rs); the allow dies with the wiring commit.
#![allow(dead_code)]

use super::mb2::AcpiRoot;

/// The RSDP signature every revision starts with.
const RSDP_SIG: &[u8; 8] = b"RSD PTR ";

/// An ACPI table header's fixed size (sig 4 + length 4 + checksum 1 +
/// revision 1 + oemid 6 + oemtableid 8 + oemrevision 4 + creatorid 4 +
/// creatorrevision 4).
pub const SDT_HEADER_SIZE: usize = 36;

/// The byte-offset of the checksum inside an SDT header, and the RSDP's
/// own first checksum right after its 8-byte signature.
const SDT_CSUM_OFF: usize = 9;
const RSDP_CSUM_OFF: usize = 8;

/// Upper bound on any table this parser will read: MCFG/MADT/HPET/XSDT
/// are all far below; a length beyond this is a loud BadLength, so a
/// lying table can never steer a huge read (T-13-07).
pub const TABLE_CAP: usize = 512;

/// Headroom for MADT IntSrcOverride entries (live q35 reports a
/// handful; more is a loud TooManyEntries).
pub const MAX_INTSRC_OVERRIDES: usize = 16;

/// How many named walk errors the facts struct carries; further ones are
/// dropped (the first failures are the loud ones).
pub const MAX_WALK_ERRORS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// A slice shorter than the fixed header it needs.
    BufferTooSmall,
    /// Signature mismatch (RSDP or table).
    BadSignature,
    /// The checksum does not sum to zero.
    BadChecksum,
    /// An implausible length (undercuts the header, exceeds TABLE_CAP).
    BadLength,
    /// A malformed or truncated entry (MCFG stride, MADT entry len).
    BadEntry,
    /// More entries of a kind than the facts struct carries.
    TooManyEntries,
    /// The RSDP scan found nothing.
    NotFound,
}

/// Every extraction goes through checked slice gets — never raw indexing
/// (the mb2.rs/fdt.rs discipline).
fn le_u16(bytes: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(off..off + 2)?.try_into().ok()?))
}

fn le_u32(bytes: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(off..off + 4)?.try_into().ok()?))
}

fn le_u64(bytes: &[u8], off: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(off..off + 8)?.try_into().ok()?))
}

/// The sdtchecksum gate (devacpi.c idiom): every byte of the region must
/// sum to zero before anything inside it is trusted.
fn checksum_zero(bytes: &[u8]) -> bool {
    bytes.iter().fold(0u8, |sum, b| sum.wrapping_add(*b)) == 0
}

/// One MCFG allocation entry: {u64 base, u16 seg, u8 start_bus, u8
/// end_bus, u32 reserved} — the ECAM window's only source (the
/// no-hardcoded-window doctrine; pci.rs has NO constant fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McfgEntry {
    pub base: u64,
    pub seg: u16,
    pub start_bus: u8,
    pub end_bus: u8,
}

/// One MADT type-1 IOAPIC entry: {id, reserved, u32 addr, u32 gsib} —
/// the physical IOAPIC and its GSI base, the facts 13-05 programs RTEs
/// against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoApicEntry {
    pub id: u8,
    pub addr: u64,
    pub gsib: u32,
}

/// One MADT type-2 Interrupt Source Override: {bus, source, u32 gsi,
/// u16 flags} — an ISA-level input remapped to a different GSI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntSrcOverride {
    pub bus: u8,
    pub source: u8,
    pub gsi: u32,
    pub flags: u16,
}

/// The MADT's parsed facts (header-adjacent fields plus the walked
/// entries).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Madt {
    /// The local interrupt controller address at table offset 36 (the
    /// LAPIC's architectural base; printed as the lapic.rs cross-check).
    pub local_intc: u64,
    /// The FIRST type-1 entry (multiple IOAPICs are a multiprocessor
    /// shape this kernel does not grow yet; the walk still parses past
    /// them).
    pub ioapic: Option<IoApicEntry>,
    /// Every type-2 override, in table order.
    pub intsrc: [IntSrcOverride; MAX_INTSRC_OVERRIDES],
    pub nintsrc: usize,
}

/// Provenance of the root-table walk — printed by the caller so every
/// gate log names where the facts came from (loader-mediated vs scan).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootInfo {
    /// The root table's physical address.
    pub table: u64,
    /// True when the table's own signature said XSDT (u64 entries);
    /// false when RSDT (u32 entries) — decided from the table, never
    /// from the hint.
    pub xsdt: bool,
    /// How many entry pointers the table carries.
    pub nentries: usize,
    /// "mb2 acpi tag" or "ebda/rom scan".
    pub via: &'static str,
}

/// The parsed ACPI facts, published by value.  Every Option absent is a
/// fact the firmware did not provide or a table that failed its gate —
/// the caller prints each absence loudly (loud-by-absence; the gates,
/// not the kernel, assert presence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acpi {
    pub root: Option<RootInfo>,
    pub mcfg: Option<McfgEntry>,
    /// MADT facts (present only when the whole MADT passed its gates —
    /// no partial table ever).
    pub madt: Option<Madt>,
    /// The HPET base (best-effort: any malformation is a plain absence,
    /// not a walk error — the timer calibrates without it).
    pub hpet: Option<u64>,
    /// The named loud errors the walk recorded — the caller prints each
    /// ("q35: acpi <label>: <err>" lines).
    pub errors: [(&'static str, ParseError); MAX_WALK_ERRORS],
    pub nerrors: usize,
}

impl Acpi {
    const fn empty() -> Acpi {
        Acpi {
            root: None,
            mcfg: None,
            madt: None,
            hpet: None,
            errors: [( "", ParseError::NotFound); MAX_WALK_ERRORS],
            nerrors: 0,
        }
    }

    fn push_error(&mut self, label: &'static str, err: ParseError) {
        if self.nerrors < MAX_WALK_ERRORS {
            self.errors[self.nerrors] = (label, err);
            self.nerrors += 1;
        }
    }
}

/// Physical-memory reads the walk runs over.  The kernel implements this
/// over l.S's identity map of the first 4 GiB; host tests implement it
/// over a synthetic memory image — the walk itself stays pure and
/// unsafe-free.
pub trait PhysMem {
    /// Copy `buf.len()` bytes at physical address `pa` into `buf`; false
    /// when the range is unreadable (outside the map).
    fn read_phys(&self, pa: u64, buf: &mut [u8]) -> bool;
}

/// Validate an RSDP byte slice and extract the root-table address:
/// rev>=2 — signature, 36-byte checksum, XSDT PA at offset 24; rev<2 —
/// signature, 20-byte checksum, RSDT PA at offset 16 (the devacpi.c
/// acpirsdptr flow).
pub fn rsdp_root(bytes: &[u8]) -> Result<AcpiRoot, ParseError> {
    if bytes.len() < 20 {
        return Err(ParseError::BufferTooSmall);
    }
    if bytes[..8] != *RSDP_SIG {
        return Err(ParseError::BadSignature);
    }
    let rev = bytes[15];
    if rev >= 2 {
        if bytes.len() < 36 {
            return Err(ParseError::BufferTooSmall);
        }
        if !checksum_zero(&bytes[..36]) {
            return Err(ParseError::BadChecksum);
        }
        let len = le_u32(bytes, 20).ok_or(ParseError::BadLength)? as usize;
        if len < 36 {
            return Err(ParseError::BadLength);
        }
        let xsdt = le_u64(bytes, 24).ok_or(ParseError::BadLength)?;
        Ok(AcpiRoot { table: xsdt, xsdt: true })
    } else {
        if !checksum_zero(&bytes[..20]) {
            return Err(ParseError::BadChecksum);
        }
        let rsdt = le_u32(bytes, 16).ok_or(ParseError::BadLength)? as u64;
        Ok(AcpiRoot { table: rsdt, xsdt: false })
    }
}

/// Parse a fully-validated MCFG table (header included): the first
/// allocation entry.  Entries region must hold at least one 16-byte
/// entry and divide evenly — anything else is BadEntry, and no partial
/// entry is ever returned.
pub fn parse_mcfg(bytes: &[u8]) -> Result<McfgEntry, ParseError> {
    const MCFG_BODY_FIXED: usize = 8; // u64 reserved before the entries
    const MCFG_ENTRY_SIZE: usize = 16; // {u64 base, u16 seg, u8 s, u8 e, u32 rsvd}
    let len = bytes.len();
    if len < SDT_HEADER_SIZE + MCFG_BODY_FIXED + MCFG_ENTRY_SIZE {
        return Err(ParseError::BadLength);
    }
    let entries = len - SDT_HEADER_SIZE - MCFG_BODY_FIXED;
    if !entries.is_multiple_of(MCFG_ENTRY_SIZE) {
        return Err(ParseError::BadEntry);
    }
    Ok(McfgEntry {
        base: le_u64(bytes, 44).ok_or(ParseError::BadLength)?,
        seg: le_u16(bytes, 52).ok_or(ParseError::BadLength)?,
        start_bus: bytes[54],
        end_bus: bytes[55],
    })
}

/// Parse a fully-validated MADT (header included): the local-intc
/// address, the first IOAPIC entry, and every IntSrcOverride.  An entry
/// whose length undercuts its type's fixed body or overruns the table is
/// BadEntry — never a panic, never a partial Madt.
pub fn parse_madt(bytes: &[u8]) -> Result<Madt, ParseError> {
    const MADT_LAPIC_ADDR: usize = 36; // u32 local intc address
    const MADT_FIRST_ENTRY: usize = 40;
    const IOAPIC_ENTRY_LEN: usize = 12; // {ty, len, id, rsvd, u32 addr, u32 gsib}
    const INTSRC_ENTRY_LEN: usize = 10; // {ty, len, bus, src, u32 gsi, u16 flags}
    let len = bytes.len();
    if len < MADT_FIRST_ENTRY {
        return Err(ParseError::BadLength);
    }
    let mut madt = Madt {
        local_intc: le_u32(bytes, MADT_LAPIC_ADDR).ok_or(ParseError::BadLength)? as u64,
        ioapic: None,
        intsrc: [IntSrcOverride { bus: 0, source: 0, gsi: 0, flags: 0 }; MAX_INTSRC_OVERRIDES],
        nintsrc: 0,
    };
    let mut off = MADT_FIRST_ENTRY;
    while off < len {
        let ty = bytes.get(off).copied().ok_or(ParseError::BadEntry)?;
        let elen = bytes.get(off + 1).copied().ok_or(ParseError::BadEntry)? as usize;
        if elen < 2 || off.checked_add(elen).is_none_or(|end| end > len) {
            return Err(ParseError::BadEntry);
        }
        match ty {
            1 => {
                if elen < IOAPIC_ENTRY_LEN {
                    return Err(ParseError::BadEntry);
                }
                let e = IoApicEntry {
                    id: bytes[off + 2],
                    addr: le_u32(bytes, off + 4).ok_or(ParseError::BadEntry)? as u64,
                    gsib: le_u32(bytes, off + 8).ok_or(ParseError::BadEntry)?,
                };
                if madt.ioapic.is_none() {
                    madt.ioapic = Some(e);
                }
            }
            2 => {
                if elen < INTSRC_ENTRY_LEN {
                    return Err(ParseError::BadEntry);
                }
                if madt.nintsrc == MAX_INTSRC_OVERRIDES {
                    return Err(ParseError::TooManyEntries);
                }
                madt.intsrc[madt.nintsrc] = IntSrcOverride {
                    bus: bytes[off + 2],
                    source: bytes[off + 3],
                    gsi: le_u32(bytes, off + 4).ok_or(ParseError::BadEntry)?,
                    flags: le_u16(bytes, off + 8).ok_or(ParseError::BadEntry)?,
                };
                madt.nintsrc += 1;
            }
            // Type 0 (LAPIC) and every other entry kind: walked past on
            // their own declared length — validated above, so this is
            // forward compatibility, not a silent skip.
            _ => {}
        }
        off += elen;
    }
    Ok(madt)
}

/// The HPET base (best-effort): the GAS at table offset 40 must name
/// system memory, with the u64 address at offset 44; any malformation or
/// absence yields None — the loud-by-absence case the caller prints.
pub fn parse_hpet(bytes: &[u8]) -> Option<u64> {
    // 36 header + 4 block id + the 12-byte GAS covers the address at
    // offset 44..52; the trailing HPET-number/min-tick fields are not
    // needed for the base.
    const HPET_GAS_OFF: usize = 40;
    const HPET_ADDR_OFF: usize = 44;
    if bytes.len() < HPET_ADDR_OFF + 8 {
        return None;
    }
    // GAS address-space id 0 = system memory (anything else is not the
    // MMIO HPET this platform would use).
    if bytes[HPET_GAS_OFF] != 0 {
        return None;
    }
    le_u64(bytes, HPET_ADDR_OFF)
}

/// Read and fully validate a table at `pa`.  `want` names the expected
/// 4-byte signature; `None` accepts XSDT or RSDT (the root table — its
/// own signature selects the entry width, never the hint).  Returns:
/// `Ok(None)` — signature mismatch (not the wanted table; not an error),
/// `Ok(Some(len))` — validated, `buf[..len]` is the table,
/// `Err(e)` — a loud named gate failure.
fn fetch_table(
    mem: &impl PhysMem,
    pa: u64,
    want: Option<&[u8; 4]>,
    buf: &mut [u8; TABLE_CAP],
) -> Result<Option<usize>, ParseError> {
    let mut hdr = [0u8; SDT_HEADER_SIZE];
    if !mem.read_phys(pa, &mut hdr) {
        return Err(ParseError::BufferTooSmall);
    }
    let sig: [u8; 4] = hdr[..4].try_into().unwrap();
    let matches = match want {
        Some(w) => sig == *w,
        None => sig == *b"XSDT" || sig == *b"RSDT",
    };
    if !matches {
        return Ok(None);
    }
    let len = le_u32(&hdr, 4).ok_or(ParseError::BadLength)? as usize;
    if !(SDT_HEADER_SIZE..=TABLE_CAP).contains(&len) {
        return Err(ParseError::BadLength);
    }
    if !mem.read_phys(pa, &mut buf[..len]) {
        return Err(ParseError::BufferTooSmall);
    }
    // The sdtchecksum gate: no pointer inside the table is read before
    // the whole-table sum passes (T-13-07).
    if !checksum_zero(&buf[..len]) {
        return Err(ParseError::BadChecksum);
    }
    Ok(Some(len))
}

/// Walk the root table's entries, parsing MCFG/MADT/HPET when their
/// signatures appear.  Each sub-table is fully gated before parsing; a
/// failure is recorded and the walk continues (no partial facts).
fn walk_root_table(mem: &impl PhysMem, root: AcpiRoot, via: &'static str, acpi: &mut Acpi) {
    let mut root_buf = [0u8; TABLE_CAP];
    let mut tbl = [0u8; TABLE_CAP];
    let len = match fetch_table(mem, root.table, None, &mut root_buf) {
        Ok(Some(len)) => len,
        // The signature check inside fetch_table passed for the root
        // kinds only; Ok(None) is unreachable here — treat it as a bad
        // signature anyway so the arm is total.
        Ok(None) | Err(ParseError::BadSignature) => {
            acpi.push_error("root table", ParseError::BadSignature);
            return;
        }
        Err(e) => {
            acpi.push_error("root table", e);
            return;
        }
    };
    let xsdt = root_buf[..4] == *b"XSDT";
    let (stride, nentries) = if xsdt {
        (8usize, (len - SDT_HEADER_SIZE) / 8)
    } else {
        (4usize, (len - SDT_HEADER_SIZE) / 4)
    };
    acpi.root = Some(RootInfo { table: root.table, xsdt, nentries, via });
    for i in 0..nentries {
        let off = SDT_HEADER_SIZE + i * stride;
        let pa = if xsdt {
            le_u64(&root_buf[..len], off).unwrap_or(0)
        } else {
            u64::from(le_u32(&root_buf[..len], off).unwrap_or(0))
        };
        let mut sig = [0u8; 4];
        if !mem.read_phys(pa, &mut sig) {
            acpi.push_error("root entry", ParseError::BufferTooSmall);
            continue;
        }
        if sig == *b"MCFG" {
            match fetch_table(mem, pa, Some(b"MCFG"), &mut tbl) {
                Ok(Some(tlen)) => match parse_mcfg(&tbl[..tlen]) {
                    Ok(entry) => acpi.mcfg = Some(entry),
                    Err(e) => acpi.push_error("mcfg table", e),
                },
                Ok(None) => {}
                Err(e) => acpi.push_error("mcfg table", e),
            }
        } else if sig == *b"APIC" {
            match fetch_table(mem, pa, Some(b"APIC"), &mut tbl) {
                Ok(Some(tlen)) => match parse_madt(&tbl[..tlen]) {
                    Ok(madt) => acpi.madt = Some(madt),
                    Err(e) => acpi.push_error("madt table", e),
                },
                Ok(None) => {}
                Err(e) => acpi.push_error("madt table", e),
            }
        } else if sig == *b"HPET" {
            // Best-effort: gate failures here are plain absences, not
            // walk errors (the timer calibrates without the HPET).
            if let Ok(Some(tlen)) = fetch_table(mem, pa, Some(b"HPET"), &mut tbl) {
                acpi.hpet = parse_hpet(&tbl[..tlen]);
            }
        }
    }
}

/// Scan one 16-byte-stepped region for a checksum-valid RSDP (the
/// rsdscan half of devacpi.c's rsdsearch).
fn rsdp_scan_region(mem: &impl PhysMem, base: u64, len: u64) -> Option<AcpiRoot> {
    let mut cand = [0u8; 36];
    let mut sig = [0u8; 8];
    let mut off = 0u64;
    while off < len {
        let pa = base + off;
        if mem.read_phys(pa, &mut sig)
            && sig == *RSDP_SIG
            && mem.read_phys(pa, &mut cand)
            && let Ok(root) = rsdp_root(&cand)
        {
            return Some(root);
        }
        off += 16;
    }
    None
}

/// The EBDA/ROM RSDP scan (devacpi.c rsdsearch): the BDA 0x40:0x0E word
/// names the EBDA segment — its first KiB first — then the BIOS ROM
/// window 0xE0000-0xFFFFF.
fn scan_for_rsdp(mem: &impl PhysMem) -> Option<AcpiRoot> {
    let mut bda = [0u8; 2];
    if mem.read_phys(0x40E, &mut bda) {
        let ebda = (u16::from_le_bytes(bda) as u64) << 4;
        if ebda != 0
            && let Some(root) = rsdp_scan_region(mem, ebda, 1024)
        {
            return Some(root);
        }
    }
    rsdp_scan_region(mem, 0xE_0000, 0x2_0000)
}

/// The full walk: resolve the root table (mb2 hint first, EBDA/ROM scan
/// as fallback), gate it, then find MCFG/MADT/HPET by signature over the
/// entry pointers — each fully gated before parsing.  Returns the facts
/// with every failure recorded as a named error for the caller to print.
pub fn discover(mem: &impl PhysMem, mb2_root: Option<AcpiRoot>) -> Acpi {
    let mut acpi = Acpi::empty();
    // Loader-mediated root first (D-03's theme): the mb2 ACPI tags
    // embedded the RSDP and 13-02's parser signature-validated it and
    // extracted this address; the TABLE it names is fully gated below.
    match mb2_root {
        Some(root) if root.table != 0 => {
            walk_root_table(mem, root, "mb2 acpi tag", &mut acpi)
        }
        Some(_) => acpi.push_error("rsdp", ParseError::BadLength), // zero table address
        None => {}
    }
    if acpi.root.is_none() {
        // The fallback arm: the EBDA/ROM scan (cross-check when the
        // loader's hint proved unusable).
        match scan_for_rsdp(mem) {
            Some(root) => walk_root_table(mem, root, "ebda/rom scan", &mut acpi),
            None if mb2_root.is_none() => acpi.push_error("rsdp", ParseError::NotFound),
            None => {}
        }
    }
    acpi
}

/// The kernel's PhysMem: l.S's bootidentitypt3 identity-maps the first
/// 4 GiB PA==VA, and every table this walk touches (EBDA, the RSDP/XSDT
/// in the ACPI-reclaimable low window, and later the MCFG-named ECAM
/// base) lives inside it.  Host tests supply their own image.
#[cfg(not(test))]
pub struct IdentityMem;

#[cfg(not(test))]
impl PhysMem for IdentityMem {
    fn read_phys(&self, pa: u64, buf: &mut [u8]) -> bool {
        let Some(end) = pa.checked_add(buf.len() as u64) else {
            return false;
        };
        if end > 0x1_0000_0000 {
            return false;
        }
        for (k, b) in buf.iter_mut().enumerate() {
            // SAFETY: pa+k is inside the identity-mapped first 4 GiB
            // (the bound checked above); the volatile byte read keeps
            // firmware RAM reads unfused and unelided — the mb2-stash
            // discipline (tables may live in memory other firmware
            // could touch; volatile is the conservative floor).
            *b = unsafe { core::ptr::read_volatile((pa + k as u64) as usize as *const u8) };
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Host fixtures: a synthetic physical-memory image plus builders for
    // every table, checksums patched to spec (ACPI on x86 is
    // little-endian).
    // ------------------------------------------------------------------
    struct Image {
        mem: Vec<u8>,
    }

    impl Image {
        fn new(size: usize) -> Image {
            Image { mem: vec![0; size] }
        }

        fn put(&mut self, pa: u64, bytes: &[u8]) {
            let p = pa as usize;
            self.mem[p..p + bytes.len()].copy_from_slice(bytes);
        }
    }

    impl PhysMem for Image {
        fn read_phys(&self, pa: u64, buf: &mut [u8]) -> bool {
            let start = pa as usize;
            match start.checked_add(buf.len()) {
                Some(end) if end <= self.mem.len() => {
                    buf.copy_from_slice(&self.mem[start..end]);
                    true
                }
                _ => false,
            }
        }
    }

    /// Patch an SDT header's checksum (offset 9) so the whole table sums
    /// to zero.
    fn sdt_csum(t: &mut [u8]) {
        let sum = t
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != SDT_CSUM_OFF)
            .fold(0u8, |a, (_, b)| a.wrapping_add(*b));
        t[SDT_CSUM_OFF] = 0u8.wrapping_sub(sum);
    }

    /// One ACPI table: a 36-byte header (sig, length, patched checksum)
    /// plus the body.
    fn sdt(sig: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut t = Vec::with_capacity(SDT_HEADER_SIZE + body.len());
        t.extend_from_slice(sig);
        t.extend_from_slice(&((SDT_HEADER_SIZE + body.len()) as u32).to_le_bytes());
        t.push(0); // checksum — patched below
        t.push(1); // revision
        t.extend_from_slice(b"R9R9R9"); // OEMID
        t.extend_from_slice(b"R9TBLID1"); // OEM table id
        t.extend_from_slice(&42u32.to_le_bytes()); // OEM revision
        t.extend_from_slice(b"R9CR"); // creator id
        t.extend_from_slice(&1u32.to_le_bytes()); // creator revision
        assert_eq!(t.len(), SDT_HEADER_SIZE);
        t.extend_from_slice(body);
        sdt_csum(&mut t);
        t
    }

    /// A revision-2 RSDP (36 bytes, 36-byte checksum patched, XSDT PA at
    /// offset 24).
    fn rsdp_v2(xsdt_pa: u64) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(b"RSD PTR ");
        r.push(0); // checksum — patched below
        r.extend_from_slice(b"BOCHS "); // OEMID
        r.push(2); // revision
        r.extend_from_slice(&0u32.to_le_bytes()); // rsdt address (unused)
        r.extend_from_slice(&36u32.to_le_bytes()); // length
        r.extend_from_slice(&xsdt_pa.to_le_bytes());
        r.push(0); // extended checksum
        r.extend_from_slice(&[0u8; 3]); // reserved
        assert_eq!(r.len(), 36);
        let sum = r
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != RSDP_CSUM_OFF)
            .fold(0u8, |a, (_, b)| a.wrapping_add(*b));
        r[RSDP_CSUM_OFF] = 0u8.wrapping_sub(sum);
        r
    }

    /// A revision-0 RSDP (20 bytes, 20-byte checksum patched, RSDT PA at
    /// offset 16).
    fn rsdp_v1(rsdt_pa: u32) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(b"RSD PTR ");
        r.push(0);
        r.extend_from_slice(b"BOCHS ");
        r.push(0); // revision
        r.extend_from_slice(&rsdt_pa.to_le_bytes());
        assert_eq!(r.len(), 20);
        let sum = r
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != RSDP_CSUM_OFF)
            .fold(0u8, |a, (_, b)| a.wrapping_add(*b));
        r[RSDP_CSUM_OFF] = 0u8.wrapping_sub(sum);
        r
    }

    fn xsdt_table(entries: &[u64]) -> Vec<u8> {
        let mut body = Vec::new();
        for e in entries {
            body.extend_from_slice(&e.to_le_bytes());
        }
        sdt(b"XSDT", &body)
    }

    fn rsdt_table(entries: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        for e in entries {
            body.extend_from_slice(&e.to_le_bytes());
        }
        sdt(b"RSDT", &body)
    }

    fn mcfg_table(entry: &McfgEntry) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0u64.to_le_bytes()); // reserved
        body.extend_from_slice(&entry.base.to_le_bytes());
        body.extend_from_slice(&entry.seg.to_le_bytes());
        body.push(entry.start_bus);
        body.push(entry.end_bus);
        body.extend_from_slice(&0u32.to_le_bytes()); // reserved
        sdt(b"MCFG", &body)
    }

    fn ioapic_entry(id: u8, addr: u64, gsib: u32) -> Vec<u8> {
        let mut e = vec![1u8, 12, id, 0];
        e.extend_from_slice(&(addr as u32).to_le_bytes());
        e.extend_from_slice(&gsib.to_le_bytes());
        e
    }

    fn intsrc_entry(bus: u8, source: u8, gsi: u32, flags: u16) -> Vec<u8> {
        let mut e = vec![2u8, 10, bus, source];
        e.extend_from_slice(&gsi.to_le_bytes());
        e.extend_from_slice(&flags.to_le_bytes());
        e
    }

    /// A type-0 LAPIC entry — {type, len, uid, apic id, u32 flags},
    /// exactly the declared 8 bytes — walked past untouched.
    fn lapic_entry() -> Vec<u8> {
        let mut e = vec![0u8, 8, 0, 2];
        e.extend_from_slice(&1u32.to_le_bytes());
        e
    }

    fn madt_table(local_intc: u32, entries: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&local_intc.to_le_bytes());
        body.extend_from_slice(entries);
        sdt(b"APIC", &body)
    }

    fn hpet_table(base: u64) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0u32.to_le_bytes()); // block id (table offset 36)
        // GAS at table offset 40: space 0 (system memory), width, bit
        // offset, access size (4 bytes), then the u64 address at table
        // offset 44 — the GAS is 12 bytes total.
        body.extend_from_slice(&[0u8, 64, 0, 0]);
        body.extend_from_slice(&base.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes()); // HPET number
        body.extend_from_slice(&0u32.to_le_bytes()); // min tick
        body.extend_from_slice(&0u32.to_le_bytes()); // page protection
        sdt(b"HPET", &body)
    }

    const XSDT_PA: u64 = 0x100_0000;
    const MCFG_PA: u64 = 0x100_0100;
    const MADT_PA: u64 = 0x100_0200;
    const HPET_PA: u64 = 0x100_0300;

    /// The golden MCFG entry every positive test expects: q35's default
    /// 256 MiB window over the whole bus range.
    const GOLDEN_MCFG: McfgEntry =
        McfgEntry { base: 0xb000_0000, seg: 0, start_bus: 0, end_bus: 255 };

    // Test 1: the golden MCFG chain — a synthetic rev-2 RSDP (correct
    // 36-byte checksum, XSDT PA at offset 24) -> XSDT (valid header, one
    // entry) -> MCFG (header checksummed, one entry) parses to exactly
    // that (base, bus range), via all three discovery arms: the EBDA/ROM
    // scan, the loader-mediated mb2 hint (the live channel's shape), and
    // a rev-1 RSDP over an RSDT's u32 entries.
    #[test]
    fn golden_mcfg_chain_parses_base_and_bus_range() {
        let mut img = Image::new(0x100_1000);
        img.put(MCFG_PA, &mcfg_table(&GOLDEN_MCFG));
        img.put(XSDT_PA, &xsdt_table(&[MCFG_PA]));
        img.put(0xE_0000, &rsdp_v2(XSDT_PA));

        // (a) the scan arm: no hint, RSDP in the ROM region.
        let acpi = discover(&img, None);
        assert_eq!(acpi.mcfg, Some(GOLDEN_MCFG), "scan arm must parse the golden MCFG");
        assert_eq!(acpi.nerrors, 0);
        let root = acpi.root.expect("root provenance present");
        assert_eq!(root.table, XSDT_PA);
        assert!(root.xsdt);
        assert_eq!(root.nentries, 1);
        assert_eq!(root.via, "ebda/rom scan");

        // (b) the mb2-hint arm (the live q35 channel's shape: tag 14
        // names the RSDT, here the XSDT equivalent).
        let acpi = discover(&img, Some(AcpiRoot { table: XSDT_PA, xsdt: true }));
        assert_eq!(acpi.mcfg, Some(GOLDEN_MCFG), "mb2 hint arm must parse the golden MCFG");
        assert_eq!(acpi.root.expect("root").via, "mb2 acpi tag");
        assert_eq!(acpi.nerrors, 0);

        // (c) rev-1 RSDP over an RSDT (u32 entries).
        let mut img = Image::new(0x100_1000);
        img.put(MCFG_PA, &mcfg_table(&GOLDEN_MCFG));
        img.put(XSDT_PA, &rsdt_table(&[MCFG_PA as u32]));
        img.put(0xE_0000, &rsdp_v1(XSDT_PA as u32));
        let acpi = discover(&img, None);
        assert_eq!(acpi.mcfg, Some(GOLDEN_MCFG), "rev-1/rsdt arm must parse");
        let root = acpi.root.expect("root");
        assert!(!root.xsdt, "the table's own signature decides the entry width");
        assert_eq!(acpi.nerrors, 0);
    }

    // Test 2: the golden MADT — one IOAPIC entry (type 1) and two
    // IntSrcOverride entries (type 2) parse to the IOAPIC base+gsib and
    // both overrides preserved in table order, with the type-0 LAPIC
    // entry walked past untouched.
    #[test]
    fn golden_madt_parses_ioapic_and_overrides() {
        let mut entries = Vec::new();
        entries.extend_from_slice(&lapic_entry());
        entries.extend_from_slice(&ioapic_entry(8, 0xFEC0_0000, 0));
        entries.extend_from_slice(&intsrc_entry(0, 0, 2, 0));
        entries.extend_from_slice(&intsrc_entry(0, 5, 5, 0xf));
        let mut img = Image::new(0x100_1000);
        img.put(MADT_PA, &madt_table(0xFEE0_0000, &entries));
        img.put(XSDT_PA, &xsdt_table(&[MADT_PA]));
        img.put(0xE_0000, &rsdp_v2(XSDT_PA));

        let acpi = discover(&img, None);
        assert_eq!(acpi.nerrors, 0);
        let madt = acpi.madt.expect("golden MADT must parse");
        assert_eq!(madt.local_intc, 0xFEE0_0000);
        assert_eq!(madt.ioapic, Some(IoApicEntry { id: 8, addr: 0xFEC0_0000, gsib: 0 }));
        assert_eq!(madt.nintsrc, 2);
        assert_eq!(madt.intsrc[0], IntSrcOverride { bus: 0, source: 0, gsi: 2, flags: 0 });
        assert_eq!(madt.intsrc[1], IntSrcOverride { bus: 0, source: 5, gsi: 5, flags: 0xf });
    }

    // Test 3: a bad checksum is a loud named error — the MCFG's checksum
    // byte does not sum the table to zero, so no pointer is derived from
    // it and no partial entry exists; the (valid) root walk itself is
    // unaffected.
    #[test]
    fn bad_table_checksum_is_a_loud_named_error() {
        let mut img = Image::new(0x100_1000);
        let mut mcfg = mcfg_table(&GOLDEN_MCFG);
        mcfg[SDT_CSUM_OFF] ^= 1; // break the whole-table sum
        img.put(MCFG_PA, &mcfg);
        img.put(XSDT_PA, &xsdt_table(&[MCFG_PA]));
        img.put(0xE_0000, &rsdp_v2(XSDT_PA));

        let acpi = discover(&img, None);
        assert_eq!(acpi.mcfg, None, "no pointer derived from an unchecksummed table");
        assert!(acpi.root.is_some(), "the valid root walk is unaffected");
        assert!(
            acpi.errors[..acpi.nerrors].contains(&("mcfg table", ParseError::BadChecksum)),
            "the failure is a loud named error, got {:?}",
            &acpi.errors[..acpi.nerrors]
        );
    }

    // Test 4: bad signature/length — (a) a root table whose signature is
    // neither XSDT nor RSDT; (b) an MCFG whose length field overruns the
    // memory behind it.  Both are loud named errors with no facts.
    #[test]
    fn bad_signature_and_overrunning_length_are_loud() {
        // (a) wrong root signature.
        let mut img = Image::new(0x100_1000);
        img.put(MCFG_PA, &mcfg_table(&GOLDEN_MCFG));
        img.put(XSDT_PA, &xsdt_table(&[MCFG_PA]));
        img.put(0xE_0000, &rsdp_v2(XSDT_PA));
        img.mem[XSDT_PA as usize + 2] = b'X'; // "XSXT"

        let acpi = discover(&img, Some(AcpiRoot { table: XSDT_PA, xsdt: true }));
        assert_eq!(acpi.root, None);
        assert_eq!(acpi.mcfg, None);
        assert!(
            acpi.errors[..acpi.nerrors].contains(&("root table", ParseError::BadSignature)),
            "root signature failure is loud, got {:?}",
            &acpi.errors[..acpi.nerrors]
        );

        // (b) an MCFG length that overruns the buffer behind it: the
        // header claims 0x200 bytes but the image ends 0x80 into the
        // table — the full read is refused loudly, no partial parse.
        let mut img = Image::new(MCFG_PA as usize + 0x80);
        let mut mcfg = mcfg_table(&GOLDEN_MCFG);
        mcfg[4..8].copy_from_slice(&0x200u32.to_le_bytes());
        sdt_csum(&mut mcfg); // keep the sum honest; the LENGTH is the defect
        img.put(MCFG_PA, &mcfg);
        img.put(XSDT_PA, &xsdt_table(&[MCFG_PA]));
        img.put(0xE_0000, &rsdp_v2(XSDT_PA));

        let acpi = discover(&img, None);
        assert_eq!(acpi.mcfg, None);
        assert!(
            acpi.errors[..acpi.nerrors].contains(&("mcfg table", ParseError::BufferTooSmall)),
            "overrunning length is loud, got {:?}",
            &acpi.errors[..acpi.nerrors]
        );
    }

    // Test 5: truncation — a MADT entry cut mid-structure (its declared
    // length overruns the table) is a loud named error; never a panic,
    // never a partial Madt.
    #[test]
    fn truncated_madt_entry_is_loud_and_never_panics() {
        let mut entries = Vec::new();
        entries.extend_from_slice(&ioapic_entry(8, 0xFEC0_0000, 0));
        // An IntSrcOverride whose declared length (10) runs past the
        // table's end: only its first 4 bytes are inside.
        entries.extend_from_slice(&[2u8, 10, 0, 9]);
        let mut img = Image::new(0x100_1000);
        img.put(MADT_PA, &madt_table(0xFEE0_0000, &entries));
        img.put(XSDT_PA, &xsdt_table(&[MADT_PA]));
        img.put(0xE_0000, &rsdp_v2(XSDT_PA));

        let acpi = discover(&img, None);
        assert_eq!(acpi.madt, None, "no partial MADT");
        assert!(
            acpi.errors[..acpi.nerrors].contains(&("madt table", ParseError::BadEntry)),
            "truncated entry is loud, got {:?}",
            &acpi.errors[..acpi.nerrors]
        );

        // The HPET best-effort arm rides the same walk: a present HPET
        // parses, and its absence (as below) is simply None.
        let mut img = Image::new(0x100_1000);
        img.put(HPET_PA, &hpet_table(0xFED0_0000));
        img.put(XSDT_PA, &xsdt_table(&[HPET_PA]));
        img.put(0xE_0000, &rsdp_v2(XSDT_PA));
        let acpi = discover(&img, None);
        assert_eq!(acpi.hpet, Some(0xFED0_0000));
        assert_eq!(acpi.nerrors, 0);
    }
}
