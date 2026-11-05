//! devip — the IP protocol device `#I` (NETIP-02): the devip-exact `/net`
//! file tree over smoltcp 0.14 sockets (D-01) on the event-driven seam
//! (D-05).  Semantics donors: `sys/src/9k/ip/devip.c` + `sys/man/3/ip`
//! (read-only references — the C is never copied); the D-03/D-08 contract
//! doc `docs/devip-contract.md` is the design authority — every divergence
//! from the donor is enumerated and recorded there.
//!
//! This file is the DEVICE surface: attach/walk/open/stat/read/write/close
//! as free functions (the devether.rs idiom — the dat.rs Dev trait is
//! non-constructible), dispatched by chan.dev via namec.rs's legs +
//! syscall.rs's pread/pwrite arms.  The model halves live in the
//! submodules: `conv` (the conversation tables), `proto` (icmp/udp — the
//! ctl verbs + data paths), `ipifc` (the interface/address model),
//! `medium` (the medium registry), `stack` (THE SEAM), `phyadapt` (the
//! Device adapters), and `crate::devip_waker` (the RawWaker bridge — the
//! phase's ONE named unsafe site, mounted at the crate root because this
//! file's forbid propagates to submodules).
//!
//! THE TREE (contract §1-§3): three levels — the top directory (files
//! arp/iproute/ipselftab/ndb/log + proto dirs), one directory per protocol
//! (clone/stats + a numbered dir per allocated conv), one directory per
//! conversation (ctl/data/err/listen/local/remote/status, + snoop for
//! ipifc).  Qids pack as `QID(p,c,y) = (p<<17)|(c<<5)|y` with the donor's
//! field widths (Logtype=5, Logconv=12, Logproto=8) — devip's OWN scheme,
//! never netif's NETQID.
//!
//! The top-level files this plan are EMPTY-OR-MINIMAL REAL files (the
//! tracer's scope rule): arp/iproute read their (initially empty) table
//! formats and take the contract's write verbs over real devip-owned
//! tables; ipselftab reads the live self-table (the ipifc `add` path
//! populates it); ndb is the real 1024-byte buffer (append-at-offset,
//! vers bump, OWRITE|OTRUNC zeroes, Eio past end); log EXISTS with its
//! 0666 mode — its read BLOCKS until netlog output (the contract's
//! semantic; the netlog feed lands in 08-05 — a functionality gap, never
//! an architectural stub), its set/clear/only writes parse and record
//! the flags now.  tcp/icmpv6 proto dirs EXIST (0555) with clone opens
//! erroring Enodev until their protos land (08-05/08-06); the D-08
//! dropped protocols (gre/esp/rudp/igmp/ipmux) are simply ABSENT.
//!
//! DELTAS FROM THE DONOR (the devether header discipline):
//!
//! - Single-user ownership collapse: the donor's iseve/owner perm checks
//!   collapse (every proc runs as glenda); the stat surface keeps the
//!   byte shape (conv files 0660, clone 0666/network→glenda).
//! - r9 implements ONE stack this era (contract §1): attach spec 0 (or
//!   empty) attaches; any other spec errors "bad specification".  The qid
//!   packing keeps the donor's field widths so a multi-stack era changes
//!   only the stack-slot count.
//! - The stack-debt drain (contract §12.1's idle corner): the syscall
//!   arms' first move is `stack::drain_debt_if_set()` (wired in
//!   syscall.rs's DEV_IP pread/pwrite arms).

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicBool, Ordering};

use crate::dat::{Chan, Qid, DEV_IP, OREAD};
use crate::mcslock::{Lock, LockNode};
use crate::Result;

pub mod conv;
pub mod ipifc;
pub mod medium;
pub mod phyadapt;
pub mod proto;
pub mod stack;

use conv::Proto;

// ---------------------------------------------------------------------------
// The qid scheme (contract §1; devip.c:35-49 verbatim field widths).
// ---------------------------------------------------------------------------

pub(crate) const MASKTYPE: u64 = 31;
pub(crate) const MASKCONV: u64 = 4095;
pub(crate) const MASKPROTO: u64 = 255;
const SHIFTCONV: u64 = 5;
const SHIFTPROTO: u64 = 17;

/// QID(p,c,y) = (p<<17)|(c<<5)|y.
pub(crate) const fn qid(p: u64, c: u64, y: u64) -> u64 {
    (p << SHIFTPROTO) | (c << SHIFTCONV) | y
}

/// TYPE(q) = q & Masktype.
pub(crate) const fn qtype(q: u64) -> u64 {
    q & MASKTYPE
}

/// CONV(q) = (q >> 5) & Maskconv.
pub(crate) const fn qconv(q: u64) -> u64 {
    (q >> SHIFTCONV) & MASKCONV
}

/// PROTO(q) = (q >> 17) & Maskproto.
pub(crate) const fn qproto(q: u64) -> u64 {
    (q >> SHIFTPROTO) & MASKPROTO
}

// The type tags (devip.c:9-33).
pub(crate) const QTOPDIR: u64 = 1;
pub(crate) const QARP: u64 = 2;
pub(crate) const QNDB: u64 = 3;
pub(crate) const QIPROUTE: u64 = 4;
pub(crate) const QIPSELFTAB: u64 = 5;
pub(crate) const QLOG: u64 = 6;
pub(crate) const QPROTODIR: u64 = 7;
pub(crate) const QCLONE: u64 = 8;
pub(crate) const QSTATS: u64 = 9;
pub(crate) const QCONVDIR: u64 = 10;
pub(crate) const QCTL: u64 = 11;
pub(crate) const QDATA: u64 = 12;
pub(crate) const QERR: u64 = 13;
pub(crate) const QLISTEN: u64 = 14;
pub(crate) const QLOCAL: u64 = 15;
pub(crate) const QREMOTE: u64 = 16;
pub(crate) const QSTATUS: u64 = 17;
pub(crate) const QSNOOP: u64 = 18;

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// The top-level fixed files (ip1gen's set): (name, tag, mode).
const TOP_FILES: [(&[u8], u64, u32); 5] = [
    (b"arp", QARP, 0o664),
    (b"ndb", QNDB, 0o666),
    (b"iproute", QIPROUTE, 0o664),
    (b"ipselftab", QIPSELFTAB, 0o444),
    (b"log", QLOG, 0o666),
];

/// The conv-dir files (ip3gen's set): (name, tag).  ctl/data/err/listen
/// carry the conv's perm (0660); local/remote/status are 0444; snoop is
/// ipifc-only, 0400.
const CONV_FILES: [(&[u8], u64); 8] = [
    (b"ctl", QCTL),
    (b"data", QDATA),
    (b"err", QERR),
    (b"listen", QLISTEN),
    (b"local", QLOCAL),
    (b"remote", QREMOTE),
    (b"status", QSTATUS),
    (b"snoop", QSNOOP),
];

/// The proto-dir fixed files (ip2gen's set): (name, tag, mode).
const PROTO_FILES: [(&[u8], u64, u32); 2] = [(b"clone", QCLONE, 0o666), (b"stats", QSTATS, 0o444)];

// ---------------------------------------------------------------------------
// The Fs-level state (the donor's `Fs` struct): ndb buffer + version, the
// route table, the self-table, the arp view, the netlog flags.  Syscall-
// context mcslock; a LEAF (never held across block(); never nested with
// TABLES/STACK — its readers/writers take it alone).
// ---------------------------------------------------------------------------

/// One self-table entry (the donor's Ipself): the address + its flags
/// (Runi / Rbcast per ipifcadd's family).
struct SelfEntry {
    addr: [u8; 16],
    bcast: bool,
}

/// One route (the donor's route entry — the file-visible fields; the
/// ifc name is the bound interface's devname when the route is local).
struct Route {
    target: [u8; 16],
    mask: [u8; 16],
    nexthop: [u8; 16],
    /// v4 route (the `4` flag) vs v6 (`6`).
    v4: bool,
    /// local interface route (the `i` flag; set for ipifc add's routes).
    ifc: bool,
    /// local unicast (the `u` flag).
    uni: bool,
    /// The originating tag (the `tag` verb's binding).
    tag: [u8; 4],
}

/// One arp view entry (contract §2/§12.5 — the snoop-fed VIEW over
/// smoltcp's accessor-less neighbor cache; 90-byte lines, no timeout).
struct ArpEntry {
    ip: [u8; 16],
    mac: [u8; 6],
    /// state: true = OK, false = WAIT (the donor's arpstate[]).
    ok: bool,
}

struct FsState {
    /// The 1024-byte ndb buffer (Fs.ndb): free-form text, append at
    /// offset, NUL-terminated; `vers` bumps on every write.
    ndb: Vec<u8>,
    ndbvers: u32,
    /// The route table (devip-owned; smoltcp's Routes mirror the
    /// default route when a gateway exists — none this era).
    routes: Vec<Route>,
    /// The current route tag (the `tag` verb's record; the donor binds
    /// it per-fd via IPaux — r9's Chan has no aux slot this era, so the
    /// tag is per-stack: behaviorally identical for every phase
    /// consumer — ipconfig's route adds never tag).
    route_tag: [u8; 4],
    /// The self-table (devip-owned; ipselftab's content).
    selftab: Vec<SelfEntry>,
    /// ipforme's accept-all flag (set by adding the null address —
    /// addselfcache's side effect; the DHCP leg's preconfig state).
    acceptall: bool,
    /// The arp VIEW table (§12.5: snoop-fed on the ether leg; static
    /// add writes the view only — the recorded divergence).
    arp: Vec<ArpEntry>,
    /// The netlog flags (set/clear/only record them; the feed lands in
    /// 08-05).  `only` is the optional address restriction.
    log_flags: u32,
    log_only: Option<[u8; 16]>,
}

static FS: Lock<FsState> = Lock::new(
    "devipfs",
    FsState {
        ndb: Vec::new(),
        ndbvers: 0,
        routes: Vec::new(),
        route_tag: *b"    ",
        selftab: Vec::new(),
        acceptall: false,
        arp: Vec::new(),
        log_flags: 0,
        log_only: None,
    },
);


// The self-table mutators/accessors (ipifc.rs's add/remove call these).

pub(crate) fn selftab_add(addr: [u8; 16], bcast: bool) {
    let node = LockNode::new();
    let mut g = FS.lock(&node);
    if g.selftab.iter().any(|e| e.addr == addr) {
        return; // the donor's exists check (addselfcache)
    }
    if addr == IP_NOADDR || addr == v4prefix_of(&[0, 0, 0, 0]) {
        g.acceptall = true; // addselfcache's null-address side effect
    }
    g.selftab.push(SelfEntry { addr, bcast });
}

pub(crate) fn selftab_remove(addr: [u8; 16]) {
    let node = LockNode::new();
    let mut g = FS.lock(&node);
    g.selftab.retain(|e| e.addr != addr);
    if addr == IP_NOADDR || addr == v4prefix_of(&[0, 0, 0, 0]) {
        g.acceptall = false;
    }
}

pub(crate) fn selftab_contains(addr: &[u8; 16]) -> bool {
    { let node = LockNode::new(); FS.lock(&node).selftab.iter().any(|e| &e.addr == addr) }
}

pub(crate) fn selftab_acceptall() -> bool {
    { let node = LockNode::new(); FS.lock(&node).acceptall }
}

/// The route add (devip-side table of record; the donor's
/// v4addroute/v6addroute shape — replace on same target+mask).
pub(crate) fn route_add(target: [u8; 16], mask: [u8; 16], nexthop: [u8; 16], ifc: bool) {
    let node = LockNode::new();
    let mut g = FS.lock(&node);
    let tag = g.route_tag;
    let v4 = is_v4(&target);
    // Replace on same target+mask (routewrite's add arm).
    g.routes.retain(|r| !(r.target == target && r.mask == mask));
    g.routes.push(Route { target, mask, nexthop, v4, ifc, uni: true, tag });
}

pub(crate) fn route_remove(target: [u8; 16], mask: [u8; 16]) {
    let node = LockNode::new();
    let mut g = FS.lock(&node);
    g.routes.retain(|r| !(r.target == target && r.mask == mask));
}

// ---------------------------------------------------------------------------
// The IP address vocabulary (16-byte IPv6 form; v4 mapped under the
// 12-byte v4prefix — contract §1).  parse/format follow the donor's
// libip parseip/eipfmt shapes.
// ---------------------------------------------------------------------------

/// The v4-mapped prefix (00..00 ff ff).
const V4PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

/// IPnoaddr — all zeros.
pub(crate) const IP_NOADDR: [u8; 16] = [0; 16];

/// v4tov6: map a v4 dotted quad into the 16-byte form.
pub(crate) fn v4prefix_of(v4: &[u8; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..12].copy_from_slice(&V4PREFIX);
    out[12..16].copy_from_slice(v4);
    out
}

/// Is the 16-byte address v4-mapped?
pub(crate) fn is_v4(a: &[u8; 16]) -> bool {
    a[..12] == V4PREFIX
}

/// The v4 tail of a v4-mapped address (None for a real v6).
pub(crate) fn v4_of(a: &[u8; 16]) -> Option<[u8; 4]> {
    if is_v4(a) { Some([a[12], a[13], a[14], a[15]]) } else { None }
}

/// a & mask bytewise.
pub(crate) fn mask_ip(a: &[u8; 16], mask: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = a[i] & mask[i];
    }
    out
}

/// The donor's atoi (leading decimal digits; garbage/empty → 0).
pub(crate) fn atoi(s: &[u8]) -> u32 {
    let mut v: u32 = 0;
    for &b in s {
        if !b.is_ascii_digit() {
            break;
        }
        v = v.saturating_mul(10).saturating_add(u32::from(b - b'0'));
    }
    v
}

/// parseip (libip): v4 dotted quad (1-4 components; a short form's last
/// component spans the remaining bytes — the classful shorthand) or v6
/// hex groups with one `::` compression.  None on malformed input (the
/// caller maps it to Ebadip).
pub(crate) fn parse_ip(s: &[u8]) -> Option<[u8; 16]> {
    if s.contains(&b':') {
        return parse_v6(s);
    }
    // v4: up to 4 dot-separated decimal components; <4 → the LAST
    // component's value spans the remaining bytes (inet_aton's shape).
    let mut parts: Vec<u32> = Vec::new();
    for comp in s.split(|&b| b == b'.') {
        if comp.is_empty() || comp.len() > 10 || !comp.iter().all(|b| b.is_ascii_digit()) {
            return None;
        }
        parts.push(atoi(comp));
    }
    if parts.is_empty() || parts.len() > 4 {
        return None;
    }
    let mut v4 = [0u8; 4];
    match parts.len() {
        4 => {
            for (i, &p) in parts.iter().enumerate() {
                if p > 255 {
                    return None;
                }
                v4[i] = p as u8;
            }
        }
        1 => {
            // a → the whole 32-bit number.
            v4 = parts[0].to_be_bytes();
        }
        n => {
            // a.b → a + (b as the remaining tail); a.b.c → a.b + c-tail.
            for (i, &p) in parts[..n - 1].iter().enumerate() {
                if p > 255 {
                    return None;
                }
                v4[i] = p as u8;
            }
            let tail_bytes = 4 - (n - 1);
            let tail = parts[n - 1];
            if tail_bytes < 4 && tail >= (1u32 << (tail_bytes * 8)) {
                return None;
            }
            let tb = tail.to_be_bytes();
            v4[4 - tail_bytes..4].copy_from_slice(&tb[4 - tail_bytes..4]);
        }
    }
    Some(v4prefix_of(&v4))
}

/// v6 parse: 8 hex groups with at most one `::` zero-compression.
fn parse_v6(s: &[u8]) -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    let mut groups: Vec<u16> = Vec::new();
    let mut tail: Option<Vec<u16>> = None;
    let mut cur: Option<u16> = None;
    let mut i = 0;
    while i < s.len() {
        match s[i] {
            b':' => {
                if i + 1 < s.len() && s[i + 1] == b':' {
                    // The compression point (one allowed).
                    if tail.is_some() {
                        return None;
                    }
                    if let Some(g) = cur.take() {
                        groups.push(g);
                    }
                    tail = Some(Vec::new());
                    i += 2;
                    continue;
                }
                if let Some(g) = cur.take() {
                    groups.push(g);
                } else {
                    // A lone ':' with no group before it (leading ':',
                    // doubled lone colons) is malformed.
                    return None;
                }
                i += 1;
            }
            b => {
                let d = (b as char).to_digit(16)?;
                cur = Some(cur.unwrap_or(0).checked_mul(16)?.checked_add(d as u16)?);
                i += 1;
            }
        }
    }
    if let Some(g) = cur.take() {
        groups.push(g);
    }
    match tail {
        None => {
            if groups.len() != 8 {
                return None;
            }
            for (i, &g) in groups.iter().enumerate() {
                out[2 * i] = (g >> 8) as u8;
                out[2 * i + 1] = (g & 0xff) as u8;
            }
        }
        Some(_) => {
            // Split the groups at the compression point: the parse above
            // pushed head groups before "::" and needs the tail separated.
            // We recorded the split at the FIRST "::" — but groups after
            // it landed in `groups` too.  Recompute: the head count is
            // unknown here, so parse again with an explicit split.
            return parse_v6_split(s);
        }
    }
    let _ = &mut out;
    Some(out)
}

/// The two-pass v6 parse for the compression form: split at the first
/// "::", parse head and tail group lists, zero-fill the middle.
fn parse_v6_split(s: &[u8]) -> Option<[u8; 16]> {
    let pos = {
        let mut found = None;
        let mut i = 0;
        while i + 1 < s.len() {
            if s[i] == b':' && s[i + 1] == b':' {
                found = Some(i);
                break;
            }
            i += 1;
        }
        found?
    };
    let head_s = &s[..pos];
    let tail_s = &s[pos + 2..];
    let parse_groups = |t: &[u8]| -> Option<Vec<u16>> {
        if t.is_empty() {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        for g in t.split(|&b| b == b':') {
            if g.is_empty() || g.len() > 4 || !g.iter().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let mut v: u16 = 0;
            for &b in g {
                v = v.checked_mul(16)?.checked_add((b as char).to_digit(16)? as u16)?;
            }
            out.push(v);
        }
        Some(out)
    };
    let head = parse_groups(head_s)?;
    let tail = parse_groups(tail_s)?;
    if head.len() + tail.len() > 8 {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, &g) in head.iter().enumerate() {
        out[2 * i] = (g >> 8) as u8;
        out[2 * i + 1] = (g & 0xff) as u8;
    }
    let skip = 8 - tail.len();
    for (j, &g) in tail.iter().enumerate() {
        let idx = skip + j;
        out[2 * idx] = (g >> 8) as u8;
        out[2 * idx + 1] = (g & 0xff) as u8;
    }
    Some(out)
}

/// parseipmask (libip): a v4 dotted mask, a v6 mask, or a numeric
/// prefix length ("/120" forms) → the 16-byte mask.
pub(crate) fn parse_ipmask(s: &[u8]) -> Option<[u8; 16]> {
    // A leading '/' → a prefix length (v6 form); the donor's
    // parseipmask also accepts the dotted form.
    if s.first() == Some(&b'/') {
        let n = atoi(&s[1..]) as usize;
        if n > 128 {
            return None;
        }
        let mut out = [0u8; 16];
        for i in 0..n {
            out[i / 8] |= 0x80 >> (i % 8);
        }
        return Some(out);
    }
    if s.contains(&b':') {
        return parse_v6(s);
    }
    // The v4 dotted mask → v4-mapped form.
    parse_ip(s)
}

/// eipfmt's %I: v4-mapped prints dotted quad; IPnoaddr prints "::";
/// other v6 prints compressed hex groups.
pub(crate) fn format_ip(a: &[u8; 16]) -> Vec<u8> {
    if let Some(v) = v4_of(a) {
        let mut out = Vec::new();
        for (i, &b) in v.iter().enumerate() {
            if i > 0 {
                out.push(b'.');
            }
            out.extend_from_slice(&proto::dec(u64::from(b)));
        }
        return out;
    }
    if a == &IP_NOADDR {
        return b"::".to_vec();
    }
    // v6: 8 groups, longest zero run ≥ 2 compresses to "::".
    let mut groups = [0u16; 8];
    for i in 0..8 {
        groups[i] = (u16::from(a[2 * i]) << 8) | u16::from(a[2 * i + 1]);
    }
    let mut best_start = None;
    let mut best_len = 0;
    let mut i = 0;
    while i < 8 {
        if groups[i] == 0 {
            let mut j = i;
            while j < 8 && groups[j] == 0 {
                j += 1;
            }
            if j - i >= 2 && j - i > best_len {
                best_len = j - i;
                best_start = Some(i);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    let mut out = Vec::new();
    let mut first = true;
    let mut i = 0;
    while i < 8 {
        if Some(i) == best_start {
            out.extend_from_slice(b"::");
            i += best_len;
            first = false; // the "::" carries the separator
            continue;
        }
        if !first {
            out.push(b':');
        }
        // lowercase hex, no leading zeros
        let mut hex = Vec::new();
        let mut g = groups[i];
        if g == 0 {
            hex.push(b'0');
        }
        while g > 0 {
            let d = (g & 0xf) as u8;
            hex.insert(0, if d < 10 { b'0' + d } else { b'a' + d - 10 });
            g >>= 4;
        }
        out.extend_from_slice(&hex);
        first = false;
        i += 1;
    }
    out
}

/// eipfmt's %M (the mask prints in the same dotted/hex form).
pub(crate) fn format_mask(m: &[u8; 16]) -> Vec<u8> {
    format_ip(m)
}

// ---------------------------------------------------------------------------
// attach + walk + open + close
// ---------------------------------------------------------------------------

/// The first-attach marker (D-13; printed once).
static ATTACH_PRINTED: AtomicBool = AtomicBool::new(false);

/// attach(spec): the stack-number parse (devip.c's ipattach): empty =
/// stack 0; a decimal number otherwise (atoi shape — leading digits);
/// r9 implements ONE stack this era (contract §1), so any nonzero spec
/// errors "bad specification" (the donor's `devno >= Nfs` arm with
/// r9's Nfs = 1).
pub fn attach(spec: &[u8]) -> Result<Chan> {
    let mut v: u64 = 0;
    for &b in spec {
        if !b.is_ascii_digit() {
            break; // atoi stops at the first non-digit
        }
        v = v.saturating_mul(10).saturating_add(u64::from(b - b'0'));
    }
    if v != 0 {
        return Err("bad specification");
    }
    if !ATTACH_PRINTED.swap(true, Ordering::Relaxed) {
        crate::println!("#I0: devip attached");
    }
    let mut c = Chan::new(DEV_IP, Qid { path: qid(0, 0, QTOPDIR), vers: 0, typ: QTDIR }, 0);
    c.path = b"#I0".to_vec();
    Ok(c)
}

/// A decimal name (the conv dirs), None on any non-digit.
fn parse_decimal(name: &[u8]) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let mut v: usize = 0;
    for &b in name {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add(usize::from(b - b'0'))?;
    }
    Some(v)
}

/// Walk a single name component (ipgen's child lookup).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR {
        return Err("devip: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devip: walk takes a single name component");
    }
    let path = c.qid.path;
    let (qid_path, typ) = match qtype(path) {
        QTOPDIR => {
            // A proto dir by name, or a top-level file.
            if let Some(p) = Proto::ALL.iter().find(|p| p.name() == name) {
                (qid(p.index(), 0, QPROTODIR), QTDIR)
            } else if let Some((_, tag, _)) = TOP_FILES.iter().find(|(n, _, _)| *n == name) {
                (qid(0, 0, *tag), QTFILE)
            } else {
                return Err("devip: name not found");
            }
        }
        QPROTODIR => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: name not found")?;
            if let Some((_, tag, _)) = PROTO_FILES.iter().find(|(n, _, _)| *n == name) {
                (qid(p.index(), 0, *tag), QTFILE)
            } else if let Some(id) = parse_decimal(name) {
                let alloc = conv::conv_allocated(p);
                if id >= conv::NCONV || !alloc[id] {
                    return Err("devip: name not found");
                }
                (qid(p.index(), id as u64, QCONVDIR), QTDIR)
            } else {
                return Err("devip: name not found");
            }
        }
        QCONVDIR => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: name not found")?;
            let id = qconv(path);
            let Some((_, tag)) = CONV_FILES.iter().find(|(n, _)| *n == name) else {
                return Err("devip: name not found");
            };
            // snoop is ipifc-only (devip.c:95-98's ip3gen arm).
            if *tag == QSNOOP && p != Proto::Ipifc {
                return Err("devip: name not found");
            }
            (qid(p.index(), id, *tag), QTFILE)
        }
        _ => return Err("devip: name not found"),
    };
    let mut nc = Chan::new(DEV_IP, Qid { path: qid_path, vers: 0, typ }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open a walked chan (ipopen): dirs OREAD-only; clone allocates a conv
/// and rewrites the qid to its ctl (tcp/icmpv6 clones error Enodev until
/// their protos land — the honest clone-exhaustion error); ctl/data/err
/// re-open the conv (the owner-collapse bump); listen requires the
/// Announced state and parks until an incoming call; the info files are
/// OREAD-only.  ndb's OWRITE|OTRUNC open zeroes the buffer first
/// (contract §2; the donor's hostowner gate collapses single-user).
/// `mode` is the FULL open mode (the namec DEV_IP arm passes it through
/// for the OTRUNC bit); access checks mask it themselves.
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let path = c.qid.path;
    let access = mode & crate::dat::OMASK;
    if c.qid.typ == QTDIR {
        if access != OREAD {
            return Err("permission denied");
        }
        let mut nc = Chan::new(c.dev, c.qid, mode);
        nc.path = c.path.clone();
        return Ok(nc);
    }
    let mut new_qid = c.qid;
    match qtype(path) {
        QCLONE => {
            let p = Proto::from_index(qproto(path)).ok_or("no free devices")?;
            // tcp/icmpv6 clone error Enodev until their protos land
            // (08-05/08-06) — clone exhaustion is the honest devip
            // error (contract §3; the dirs EXIST).
            if !matches!(p, Proto::Icmp | Proto::Ipifc) {
                return Err("no free devices");
            }
            let id = conv::conv_alloc(p)?;
            new_qid = Qid { path: qid(p.index(), id as u64, QCTL), vers: 0, typ: QTFILE };
        }
        QCTL | QDATA | QERR => {
            let p = Proto::from_index(qproto(path)).ok_or("no free devices")?;
            conv::conv_open(p, qconv(path) as usize)?;
        }
        QLISTEN => {
            let p = Proto::from_index(qproto(path)).ok_or("no free devices")?;
            let id = qconv(path) as usize;
            return listen_open(c, p, id, mode);
        }
        QLOCAL | QREMOTE | QSTATUS | QSTATS | QIPSELFTAB | QSNOOP => {
            if access != OREAD {
                return Err("permission denied");
            }
        }
        QNDB => {
            if access == crate::dat::OWRITE && mode & crate::dat::OTRUNC != 0 {
                let node = LockNode::new();
                let mut g = FS.lock(&node);
                g.ndb.clear();
                g.ndbvers += 1;
            }
        }
        _ => {
            // arp/iproute/log: the perms allow read+write (0664/0666).
        }
    }
    let mut nc = Chan::new(c.dev, new_qid, mode);
    nc.path = c.path.clone();
    Ok(nc)
}

/// The listen-file open (devip.c:445-495): requires state == Announced
/// ("not announced"); blocks until an incoming call lands in the conv's
/// incall queue; the returned chan points at the NEW conv's ctl.  A
/// hangup of the waiting conv errors "listen hungup".
fn listen_open(c: &Chan, p: Proto, id: usize, mode: u32) -> Result<Chan> {
    {
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        if t.table(p)[id].state != conv::ConvState::Announced {
            return Err("not announced");
        }
    }
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            let table = t.table_mut(p);
            let cv = &mut table[id];
            if let Some(nid) = cv.incall.pop_front() {
                // The returned chan names the NEW conv's ctl (the donor
                // re-affirms the owner on it; single-user collapse).
                let mut nc = Chan::new(
                    c.dev,
                    Qid { path: qid(p.index(), nid as u64, QCTL), vers: 0, typ: QTFILE },
                    mode,
                );
                nc.path = c.path.clone();
                return Ok(nc);
            }
            if cv.state != conv::ConvState::Announced {
                return Err("listen hungup");
            }
            // Register, drop, block (devpipe's wakeup-loss-free shape);
            // Fsnewcall (udp Task 2 / tcp 08-05) wakes via wake_conv.
            if hooks.is_none() {
                hooks = Some(stack::hooks()?);
            }
            let pid = (hooks.unwrap().current)();
            cv.waiters.register(pid);
            stack::register_net_waiter(pid);
        }
        (hooks.unwrap().block)();
    }
}

/// dat.rs Chan::dup's DEV_IP arm (the devether precedent): an open
/// conv-file chan's value copy bumps the conv's open count.
pub(crate) fn ref_open(qid_path: u64) {
    if matches!(qtype(qid_path), QCTL | QDATA | QERR) {
        let p = match Proto::from_index(qproto(qid_path)) {
            Some(p) => p,
            None => return,
        };
        conv::conv_ref(p, qconv(qid_path) as usize);
    }
}

/// The close dispatch's DEV_IP arm (namec::close_chan, COPEN-gated by
/// the caller): ipclose's shape — conv files run closeconv.
pub fn close(c: &Chan) {
    if c.qid.typ == QTDIR {
        return;
    }
    if matches!(qtype(c.qid.path), QCTL | QDATA | QERR) {
        let Some(p) = Proto::from_index(qproto(c.qid.path)) else { return };
        conv::conv_close(p, qconv(c.qid.path) as usize);
    }
}

// ---------------------------------------------------------------------------
// stat + directory listings (the stat(5) wire, FOUR strings per record —
// libc's statcheck walks four; convM2D.c)
// ---------------------------------------------------------------------------

/// One stat(5) wire record (the devcons/devether shape; uid/gid/muid
/// glenda).  Returns the byte count (the convD2M contract).
fn write_stat_record(sb: &mut [u8], name: &[u8], qid_path: u64, vers: u32, typ: u8, mode: u32, length: u64) -> Result<u64> {
    let need = 41 + (2 + name.len()) + 3 * (2 + 6);
    if sb.len() < need {
        return Err("devip: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(DEV_IP as u32);
    w.u8(typ);
    w.u32(vers);
    w.u64(qid_path);
    w.u32(mode);
    w.u32(0); // atime
    w.u32(0); // mtime
    w.u64(length);
    w.string(name);
    w.string(b"glenda");
    w.string(b"glenda");
    w.string(b"glenda"); // muid — the fourth string statcheck walks
    Ok(w.finish())
}

/// The mode/length for one conv file (the owner-collapse surface: the
/// conv's perm (0660) on ctl/data/err/listen, 0444 on local/remote/
/// status; live queue lengths on data/err per ip3gen).
fn conv_file_mode(p: Proto, id: usize, tag: u64) -> (u32, u64) {
    let node = LockNode::new();
    let t = conv::TABLES.lock(&node);
    let c = &t.table(p)[id];
    match tag {
        QCTL | QLISTEN => (c.perm, 0),
        QDATA => {
            // ip3gen's live qlen: the socket's queued RX bytes (0 for a
            // socket-less conv).  TABLES → STACK nesting (conv.rs's
            // documented order).
            let n = match c.sock {
                Some(h) => {
                    let g = stack::STACK.enter().expect("devip: stack contended");
                    match g.core.as_ref() {
                        Some(core) => match p {
                            Proto::Icmp => core.sockets.get::<smoltcp::socket::icmp::Socket>(h).recv_queue() as u64,
                            _ => 0,
                        },
                        None => 0,
                    }
                }
                None => 0,
            };
            (c.perm, n)
        }
        QERR => (c.perm, c.errq.len() as u64),
        QSNOOP => (0o400, 0),
        _ => (0o444, 0),
    }
}

/// The record for one qid (the listing builder's and stat's shared
/// core).  `name` is the leaf name.
fn record_for(sb: &mut [u8], name: &[u8], qid_path: u64, typ: u8) -> Result<u64> {
    let (mode, length, vers) = if typ == QTDIR {
        (0x8000_0000 | 0o555, 0, 0)
    } else {
        match qtype(qid_path) {
            QCLONE => (0o666, 0, 0),
            QSTATS => (0o444, 0, 0),
            QARP => (0o664, arp_content_len() as u64, 0),
            QNDB => {
                let node = LockNode::new();
                let g = FS.lock(&node);
                (0o666, g.ndb.len() as u64, g.ndbvers)
            }
            QIPROUTE => (0o664, 0, 0),
            QIPSELFTAB => (0o444, 0, 0),
            QLOG => (0o666, 0, 0),
            QCTL | QDATA | QERR | QLISTEN | QLOCAL | QREMOTE | QSTATUS | QSNOOP => {
                let p = Proto::from_index(qproto(qid_path)).ok_or("devip: stat of unknown file")?;
                let (m, l) = conv_file_mode(p, qconv(qid_path) as usize, qtype(qid_path));
                (m, l, 0)
            }
            _ => return Err("devip: stat of unknown file"),
        }
    };
    write_stat_record(sb, name, qid_path, vers, typ, mode, length)
}

/// stat of a devip chan (sys_stat/sys_fstat's dispatch leg).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let path = c.qid.path;
    if qtype(path) == QTOPDIR {
        return write_stat_record(sb, b"#I0", path, 0, QTDIR, 0x8000_0000 | 0o555, 0);
    }
    match qtype(path) {
        QPROTODIR => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: stat of unknown file")?;
            record_for(sb, p.name(), path, QTDIR)
        }
        QCONVDIR => {
            let id = qconv(path);
            let mut name = Vec::new();
            let mut v = id;
            if v == 0 {
                name.push(b'0');
            }
            while v > 0 {
                name.insert(0, b'0' + (v % 10) as u8);
                v /= 10;
            }
            record_for(sb, &name, path, QTDIR)
        }
        _ => {
            let name = c.path.rsplit(|&b| b == b'/').next().unwrap_or(b"").to_vec();
            record_for(sb, &name, path, QTFILE)
        }
    }
}

/// The directory listing (devdirread's shape): rebuilt per call from
/// the live tables; the streaming-offset convention (read_str windows
/// it).  Order per ipgen: proto dirs first, then the top files; within
/// a proto dir, the allocated conv dirs then clone/stats.
fn dir_listing(c: &Chan) -> Result<Vec<u8>> {
    let mut listing = Vec::new();
    macro_rules! emit {
        ($name:expr, $qid:expr, $typ:expr) => {{
            let name: &[u8] = $name;
            let start = listing.len();
            listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
            record_for(&mut listing[start..], name, $qid, $typ)?;
        }};
    }
    match qtype(c.qid.path) {
        QTOPDIR => {
            for p in Proto::ALL {
                emit!(p.name(), qid(p.index(), 0, QPROTODIR), QTDIR);
            }
            for (name, tag, _) in TOP_FILES {
                emit!(name, qid(0, 0, tag), QTFILE);
            }
        }
        QPROTODIR => {
            let p = Proto::from_index(qproto(c.qid.path)).ok_or("devip: read of a non-directory")?;
            let alloc = conv::conv_allocated(p);
            for (id, present) in alloc.iter().enumerate() {
                if !present {
                    continue;
                }
                let mut name = Vec::new();
                let mut v = id;
                if v == 0 {
                    name.push(b'0');
                }
                while v > 0 {
                    name.insert(0, b'0' + (v % 10) as u8);
                    v /= 10;
                }
                emit!(&name, qid(p.index(), id as u64, QCONVDIR), QTDIR);
            }
            for (name, tag, _) in PROTO_FILES {
                emit!(name, qid(p.index(), 0, tag), QTFILE);
            }
        }
        QCONVDIR => {
            let p = Proto::from_index(qproto(c.qid.path)).ok_or("devip: read of a non-directory")?;
            let id = qconv(c.qid.path);
            for (name, tag) in CONV_FILES {
                if tag == QSNOOP && p != Proto::Ipifc {
                    continue; // ipifc-only (devip.c:95-98)
                }
                emit!(name, qid(p.index(), id, tag), QTFILE);
            }
        }
        _ => return Err("devip: read of a non-directory"),
    }
    Ok(listing)
}

// ---------------------------------------------------------------------------
// read
// ---------------------------------------------------------------------------

/// readstr's window onto a content byte string (9k dev.c): offset
/// semantics, u64::MAX = the streaming chan offset.
fn read_str(c: &mut Chan, buf: &mut [u8], offset: u64, content: &[u8]) -> u64 {
    let off = if offset == u64::MAX { c.offset } else { offset };
    let start = (off as usize).min(content.len());
    let avail = &content[start..];
    let n = avail.len().min(buf.len());
    buf[..n].copy_from_slice(&avail[..n]);
    if offset == u64::MAX {
        c.offset += n as u64;
    }
    n as u64
}

/// Read from a devip chan (ipread): dirs serve the stat-wire listing;
/// the top files serve their table content; ctl returns the conv number
/// (plain decimal, the donor's "%lud" — NOT readnum's padded shape);
/// local/remote/status serve the proto's content; data/err block per
/// the proto's Qmsg semantics.
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let path = c.qid.path;
    if c.qid.typ == QTDIR {
        let listing = dir_listing(c)?;
        return Ok(read_str(c, buf, offset, &listing));
    }
    match qtype(path) {
        QARP => {
            let content = arp_content();
            Ok(read_str(c, buf, offset, &content))
        }
        QNDB => {
            let content = { let node = LockNode::new(); FS.lock(&node).ndb.clone() };
            Ok(read_str(c, buf, offset, &content))
        }
        QIPROUTE => {
            let content = route_content();
            Ok(read_str(c, buf, offset, &content))
        }
        QIPSELFTAB => {
            let content = selftab_content();
            Ok(read_str(c, buf, offset, &content))
        }
        QLOG => log_read(c, buf),
        QCTL => {
            let content = conv::ctl_number_content(qconv(path) as usize);
            Ok(read_str(c, buf, offset, &content))
        }
        QLOCAL => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: read of unknown file")?;
            let id = qconv(path) as usize;
            let content = if p == Proto::Ipifc { ipifc::local(id) } else { proto::local_remote(p, id, true) };
            Ok(read_str(c, buf, offset, &content))
        }
        QREMOTE => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: read of unknown file")?;
            let content = proto::local_remote(p, qconv(path) as usize, false);
            Ok(read_str(c, buf, offset, &content))
        }
        QSTATUS => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: read of unknown file")?;
            let id = qconv(path) as usize;
            let content = if p == Proto::Ipifc { ipifc::status(id) } else { proto::status(p, id) };
            Ok(read_str(c, buf, offset, &content))
        }
        QDATA => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: read of unknown file")?;
            match p {
                Proto::Icmp => proto::icmp_read_data(qconv(path) as usize, buf),
                // udp's read joins in Task 2; tcp/icmpv6 in 08-05/08-06.
                // A conv of another proto can never be open here (its
                // clone errored Enodev), so this arm is defensive.
                _ => Err("devip: no data on this proto this era"),
            }
        }
        QERR => err_read(c, buf, path),
        QSNOOP => {
            // ipifc snoop is ungated this era (RESEARCH A4): the queue
            // is never fed; the read blocks like the donor's qread.
            err_read(c, buf, path)
        }
        QSTATS => {
            let p = Proto::from_index(qproto(path)).ok_or("devip: read of unknown file")?;
            let content = match p {
                Proto::Icmp => proto::icmp_stats(),
                Proto::Ipifc => ipifc_stats(),
                _ => return Err("stats not implemented"),
            };
            Ok(read_str(c, buf, offset, &content))
        }
        _ => Err("devip: read of unknown file"),
    }
}

/// The err-file read (the conv's error queue, Qmsg): one message per
/// read; blocks when empty (the donor's qread(c->eq)); EOF when the
/// conv was closed mid-read (the qclosed arm — closeconv wakes us).
fn err_read(_c: &mut Chan, buf: &mut [u8], path: u64) -> Result<u64> {
    let p = Proto::from_index(qproto(path)).ok_or("devip: read of unknown file")?;
    let id = qconv(path) as usize;
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            let cv = &mut t.table_mut(p)[id];
            if let Some(msg) = cv.errq.pop_front() {
                let n = msg.len().min(buf.len());
                buf[..n].copy_from_slice(&msg[..n]);
                return Ok(n as u64);
            }
            if hooks.is_none() {
                hooks = Some(stack::hooks()?);
            }
            let pid = (hooks.unwrap().current)();
            cv.waiters.register(pid);
            stack::register_net_waiter(pid);
        }
        (hooks.unwrap().block)();
        // A wake re-checks (spurious discipline); closeconv's reset
        // empties the queue AND the last close wakes us — a woken
        // reader with an empty queue on a closed conv reads EOF.
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        let cv = &t.table(p)[id];
        if cv.errq.is_empty() && cv.inuse == 0 {
            return Ok(0);
        }
    }
}

/// The log read (netlogread's contract §2): BLOCKS until netlog output
/// is available.  The netlog feed lands in 08-05 (a functionality gap,
/// never an architectural stub — the file EXISTS with its 0666 mode and
/// the blocking semantic now).  Same block shape as err_read on an
/// empty, never-fed queue.
fn log_read(c: &mut Chan, buf: &mut [u8]) -> Result<u64> {
    // The Qlog qid's proto/conv fields are 0 — the block rides the
    // global net waiter set (no conv to hang the wait on).
    let _ = c;
    let _ = buf;
    let hooks = stack::hooks()?;
    loop {
        stack::register_net_waiter((hooks.current)());
        (hooks.block)();
        // The netlog feed lands in 08-05; a wake re-checks (nothing to
        // read yet) and re-blocks.
    }
}

// ---------------------------------------------------------------------------
// write
// ---------------------------------------------------------------------------

/// Write to a devip chan (ipwrite): ctl takes the verb dispatch (the
/// 64-byte cmdbuf bound lives in proto::ctl_write); data is the proto's
/// write path; ndb appends at offset; arp/iproute/log take their
/// contract verbs; everything else is read-only (Eperm).
pub fn write(c: &mut Chan, buf: &[u8], offset: u64) -> Result<u64> {
    let path = c.qid.path;
    match qtype(path) {
        QCTL => {
            let p = Proto::from_index(qproto(path)).ok_or("permission denied")?;
            proto::ctl_write(p, qconv(path) as usize, buf)
        }
        QDATA => {
            let p = Proto::from_index(qproto(path)).ok_or("permission denied")?;
            match p {
                Proto::Icmp => proto::icmp_write_data(qconv(path) as usize, buf),
                _ => Err("devip: no data on this proto this era"),
            }
        }
        QNDB => ndb_write(c, buf, offset),
        QARP => arp_write(buf),
        QIPROUTE => route_write(buf),
        QLOG => log_write(buf),
        _ => Err("permission denied"),
    }
}

// ---------------------------------------------------------------------------
// The top-level files' content + verbs.
// ---------------------------------------------------------------------------

fn arp_content_len() -> usize {
    { let node = LockNode::new(); FS.lock(&node).arp.len() * 90 }
}

/// The arp read (contract §2): fixed 90-byte lines,
/// `"%-6.6s %-8.8s %-40.40I %-32.32s\n"` (medium, state, ip, mac);
/// `offset % 90 != 0` → 0 (enforced by the read's window onto the
/// 90-byte-aligned content).
fn arp_content() -> Vec<u8> {
    let node = LockNode::new();
    let g = FS.lock(&node);
    let mut out = Vec::new();
    for e in &g.arp {
        let mut line = Vec::new();
        line.extend_from_slice(&pad_right(b"loopback", 6));
        line.push(b' ');
        line.extend_from_slice(&pad_right(if e.ok { b"OK" } else { b"WAIT" }, 8));
        line.push(b' ');
        line.extend_from_slice(&pad_right(&format_ip(&e.ip), 40));
        line.push(b' ');
        let mut mac = Vec::new();
        for (i, &b) in e.mac.iter().enumerate() {
            if i > 0 {
                mac.push(b':');
            }
            mac.extend_from_slice(&hex2(b));
        }
        line.extend_from_slice(&pad_right(&mac, 32));
        line.push(b'\n');
        // The line is 90 bytes by construction (6+1+8+1+40+1+32+1).
        line.resize(90, b' ');
        out.extend_from_slice(&line[..90]);
    }
    out
}

/// The arp write (contract §2): `flush` / `add ether <ip> <mac>` /
/// `del <ip>`.  §12.5's divergence: the table is a VIEW — `add` writes
/// the view only (no resolution steering).
fn arp_write(buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    let text = &buf[..n.min(63)];
    let fields: Vec<&[u8]> = text
        .split(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
        .filter(|w| !w.is_empty())
        .collect();
    if fields.is_empty() {
        return Err("bad arp");
    }
    let node = LockNode::new();
    let mut g = FS.lock(&node);
    match fields[0] {
        b"flush" => {
            g.arp.clear();
            Ok(n as u64)
        }
        b"add" => {
            // The documented 4-field form: add ether <ip> <mac>.
            if fields.len() != 4 {
                return Err("bad arp");
            }
            if fields[1] != b"ether" {
                return Err("bad arp"); // the medium name must resolve
            }
            let ip = parse_ip(fields[2]).ok_or("bad ip address syntax")?;
            let mac = parse_mac(fields[3]).ok_or("bad arp")?;
            g.arp.retain(|e| e.ip != ip);
            g.arp.push(ArpEntry { ip, mac, ok: true });
            Ok(n as u64)
        }
        b"del" => {
            if fields.len() != 2 {
                return Err("bad arp");
            }
            let ip = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            g.arp.retain(|e| e.ip != ip);
            Ok(n as u64)
        }
        _ => Err("bad arp"),
    }
}

/// The iproute read (contract §2): one line per route,
/// `"%-15I %-4M %-15I %4.4s %4.4s %3s\n"` (target mask nexthop flags
/// tag ifc).
fn route_content() -> Vec<u8> {
    let node = LockNode::new();
    let g = FS.lock(&node);
    let mut out = Vec::new();
    for r in &g.routes {
        out.extend_from_slice(&pad_right(&format_ip(&r.target), 15));
        out.push(b' ');
        out.extend_from_slice(&pad_right(&format_mask(&r.mask), 4));
        out.push(b' ');
        out.extend_from_slice(&pad_right(&format_ip(&r.nexthop), 15));
        out.push(b' ');
        // The flags (routetype): version char + i + u/b/m/p.
        let mut flags = Vec::new();
        flags.push(if r.v4 { b'4' } else { b'6' });
        if r.ifc {
            flags.push(b'i');
        }
        if r.uni {
            flags.push(b'u');
        }
        out.extend_from_slice(&pad_right(&flags, 4));
        out.push(b' ');
        out.extend_from_slice(&pad_right(&r.tag, 4));
        out.push(b' ');
        let ifc = if r.ifc { b"loopback0".as_ref() } else { b"".as_ref() };
        out.extend_from_slice(&pad_right(ifc, 3));
        out.push(b'\n');
    }
    out
}

/// The iproute write (contract §2): `flush` / `tag <string>` /
/// `add <target> <mask> <nexthop>` / `remove <target> <mask>` /
/// `route <target>` (console print, the debug aid).
fn route_write(buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    let text = &buf[..n.min(63)];
    let fields: Vec<&[u8]> = text
        .split(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
        .filter(|w| !w.is_empty())
        .collect();
    if fields.is_empty() {
        return Err("bad arg in system call");
    }
    match fields[0] {
        b"flush" => {
            { let node = LockNode::new(); FS.lock(&node).routes.clear() };
            Ok(n as u64)
        }
        b"tag" => {
            if fields.len() != 2 {
                return Err("bad arg in system call");
            }
            let mut tag = *b"    ";
            let src = &fields[1][..fields[1].len().min(4)];
            tag[..src.len()].copy_from_slice(src);
            { let node = LockNode::new(); FS.lock(&node).route_tag = tag };
            Ok(n as u64)
        }
        b"add" => {
            if fields.len() != 4 {
                return Err("bad arg in system call");
            }
            let target = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            let mask = parse_ipmask(fields[2]).ok_or("bad ip address syntax")?;
            let nexthop = parse_ip(fields[3]).ok_or("bad ip address syntax")?;
            route_add(target, mask, nexthop, false);
            Ok(n as u64)
        }
        b"remove" => {
            if fields.len() != 3 {
                return Err("bad arg in system call");
            }
            let target = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            let mask = parse_ipmask(fields[2]).ok_or("bad ip address syntax")?;
            route_remove(target, mask);
            Ok(n as u64)
        }
        b"route" => {
            if fields.len() != 2 {
                return Err("bad arg in system call");
            }
            let target = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            let found = {
                let node = LockNode::new();
                let g = FS.lock(&node);
                g.routes.iter().find(|r| mask_ip(&target, &r.mask) == r.target).map(|r| {
                    (format_ip(&r.target), format_mask(&r.mask), format_ip(&r.nexthop))
                })
            };
            match found {
                Some((t, m, h)) => {
                    crate::println!("devip: route {:?} {:?} {:?}", String::from_utf8_lossy(&t), String::from_utf8_lossy(&m), String::from_utf8_lossy(&h));
                }
                None => crate::println!("devip: no route"),
            }
            Ok(n as u64)
        }
        _ => Err("bad arg in system call"),
    }
}

/// The ipselftab read (contract §2; ipifc.c:1028-1052):
/// `"%-44.44I %2.2d %4.4s\n"` per local address (addr, ifc use-count,
/// routetype flags — self entries carry no version bit, so the first
/// flag char is `6` per the source's else arm; contract §2 records the
/// `4` flag is never set).
fn selftab_content() -> Vec<u8> {
    let node = LockNode::new();
    let g = FS.lock(&node);
    let mut out = Vec::new();
    for e in &g.selftab {
        out.extend_from_slice(&pad_right(&format_ip(&e.addr), 44));
        out.push(b' ');
        out.extend_from_slice(&pad_left(&proto::dec(1), 2));
        out.push(b' ');
        // routetype(Runi|Rbcast): no Rv4 bit → '6'; then 'u' or 'b'.
        let mut flags = Vec::new();
        flags.push(b'6');
        flags.push(if e.bcast { b'b' } else { b'u' });
        out.extend_from_slice(&pad_right(&flags, 4));
        out.push(b'\n');
    }
    out
}

/// The ndb write (ndbwrite, devip.c:1429-1441): append at offset,
/// NUL-terminated, Eio past end; OWRITE|OTRUNC zeroes first (handled at
/// open).  vers bumps on every write.  The streaming offset
/// (u64::MAX = the chan's own offset, advanced) resolves like
/// read_str's.
fn ndb_write(c: &mut Chan, buf: &[u8], offset: u64) -> Result<u64> {
    let n = buf.len();
    let node = LockNode::new();
    let mut g = FS.lock(&node);
    let off = if offset == u64::MAX { c.offset } else { offset };
    let off = off as usize;
    if off > g.ndb.len() {
        return Err("i/o error");
    }
    if off + n >= 1024 {
        return Err("i/o error");
    }
    // Grow-or-write at the offset (append-at-offset semantics; the
    // buffer is length-tracked, not NUL-padded, with the content cap
    // at 1024).
    if g.ndb.len() < off + n {
        g.ndb.resize(off + n, 0);
    }
    g.ndb[off..off + n].copy_from_slice(buf);
    g.ndbvers += 1;
    if offset == u64::MAX {
        c.offset += n as u64;
    }
    Ok(n as u64)
}

/// The log write (netlogctl, contract §2): `set <flags…>` /
/// `clear <flags…>` / `only <addr>`; too few arguments → Ebadnetctl.
/// The flags are recorded now; the feed lands in 08-05.
fn log_write(buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    let text = &buf[..n.min(63)];
    let fields: Vec<&[u8]> = text
        .split(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
        .filter(|w| !w.is_empty())
        .collect();
    if fields.len() < 2 {
        return Err("too few arguments for netlog control message");
    }
    let node = LockNode::new();
    let mut g = FS.lock(&node);
    match fields[0] {
        b"set" => {
            for f in &fields[1..] {
                g.log_flags |= log_flag(f);
            }
            Ok(n as u64)
        }
        b"clear" => {
            for f in &fields[1..] {
                g.log_flags &= !log_flag(f);
            }
            Ok(n as u64)
        }
        b"only" => {
            g.log_only = parse_ip(fields[1]);
            Ok(n as u64)
        }
        _ => Err("too few arguments for netlog control message"),
    }
}

/// The netlog flag names → bits (netlog.c's flags[]).
fn log_flag(name: &[u8]) -> u32 {
    match name {
        b"ppp" => 1 << 0,
        b"ip" => 1 << 1,
        b"fs" => 1 << 2,
        b"tcp" => 1 << 3,
        b"icmp" => 1 << 4,
        b"udp" => 1 << 5,
        b"compress" => 1 << 6,
        b"gre" => 1 << 7,
        b"tcpwin" => (1 << 3) | (1 << 8),
        b"tcprxmt" => (1 << 3) | (1 << 9),
        b"udpmsg" => (1 << 5) | (1 << 10),
        b"ipmsg" => (1 << 1) | (1 << 11),
        b"esp" => 1 << 12,
        _ => 0,
    }
}

/// The ipifc stats (contract §9.4 — the MIB II 19 tagged lines;
/// bridge-maintained per P-M: the counters increment at the bridge).
fn ipifc_stats() -> Vec<u8> {
    const NAMES: [&[u8]; 19] = [
        b"Forwarding",
        b"DefaultTTL",
        b"InReceives",
        b"InHdrErrors",
        b"InAddrErrors",
        b"ForwDatagrams",
        b"InUnknownProtos",
        b"InDiscards",
        b"InDelivers",
        b"OutRequests",
        b"OutDiscards",
        b"OutNoRoutes",
        b"ReasmTimeout",
        b"ReasmReqds",
        b"ReasmOKs",
        b"ReasmFails",
        b"FragOKs",
        b"FragFails",
        b"FragCreates",
    ];
    let mut out = Vec::new();
    for name in NAMES {
        out.extend_from_slice(name);
        out.extend_from_slice(b": ");
        out.extend_from_slice(&proto::dec(ipstat_count(name)));
        out.push(b'\n');
    }
    out
}

/// The ipifc counters: Forwarding 0 (no routing this era), DefaultTTL
/// 255 (§12.8), the rest bridge-observed (0 until the ether leg's
/// packet paths land — the loopback counters live in the Ifc record's
/// status line).
fn ipstat_count(name: &[u8]) -> u64 {
    match name {
        b"Forwarding" => 0,
        b"DefaultTTL" => conv::MAX_TTL as u64,
        _ => 0,
    }
}

/// The per-conv wake-slot arrays for the icmp data paths (the waker
/// bridge's never-outlives-the-conv invariant made mechanical — the
/// slots are statics beside the conv table).
pub(crate) static ICMP_RX_SLOTS: [crate::devip_waker::WakeSlot; conv::NCONV] =
    [const { crate::devip_waker::WakeSlot::new() }; conv::NCONV];
pub(crate) static ICMP_TX_SLOTS: [crate::devip_waker::WakeSlot; conv::NCONV] =
    [const { crate::devip_waker::WakeSlot::new() }; conv::NCONV];

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn pad_right(s: &[u8], width: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&s[..s.len().min(width)]);
    while out.len() < width {
        out.push(b' ');
    }
    out
}

fn pad_left(s: &[u8], width: usize) -> Vec<u8> {
    let mut out = Vec::new();
    while out.len() + s.len() < width {
        out.push(b' ');
    }
    out.extend_from_slice(s);
    out
}

/// Lowercase hex byte pair.
fn hex2(b: u8) -> [u8; 2] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    [HEX[(b >> 4) as usize], HEX[(b & 0xf) as usize]]
}

/// Parse a colon-separated MAC (aa:bb:cc:dd:ee:ff).
fn parse_mac(s: &[u8]) -> Option<[u8; 6]> {
    let mut out = [0u8; 6];
    let parts: Vec<&[u8]> = s.split(|&b| b == b':').collect();
    if parts.len() != 6 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        if p.len() != 2 || !p.iter().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let hi = (p[0] as char).to_digit(16)?;
        let lo = (p[1] as char).to_digit(16)?;
        out[i] = ((hi << 4) | lo) as u8;
    }
    Some(out)
}
