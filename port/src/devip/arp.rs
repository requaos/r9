//! devip/arp.rs — the `/net/arp` view (contract §2's arp chapter + §12.5's
//! r9 mapping).  The table is DEVIP-OWNED and SNOOP-FED: smoltcp 0.14 has
//! NO public neighbor-cache accessor, so the ether adapter
//! (phyadapt.rs) parses ARP (ethertype 0x806) and NDP (ICMPv6 NS/NA
//! inside 0x86dd) frames passing through BOTH `receive()` and
//! `transmit()` with smoltcp's own wire codecs (never hand-rolled
//! offsets) and feeds entries here: `(ip→mac, OK)` on replies/
//! announcements, `(ip, WAIT)` when our own solicitations are observed
//! (the TX-side feedback).
//!
//! THE STATIC-ADD DIVERGENCE (one line, per contract §12.5's honesty
//! rule): in 9k, `arp add` injects into the REAL resolution path; here it
//! writes the VIEW table only — smoltcp's internal cache self-manages, so
//! a script that `arp add`s gets file-level success and no resolution
//! steering.
//!
//! The surface (arp.c:425-556): reads emit fixed 90-byte lines
//! `"%-6.6s %-8.8s %-40.40I %-32.32s\n"` (medium, state[OK|WAIT], ip,
//! mac); a read with `offset % 90 != 0` returns 0.  The table is
//! NCACHE=256 entries, LRU, NO timeout (entries never expire); writes are
//! `flush` / `add ether <ip> <mac>` / `add <ip> <mac>` (the 3-field
//! form's route-mediated medium pick) / `del <ip>`.  The medium must name
//! one with ARP machinery — only ether has it this era (the donor's
//! `type->ares == nil` arm: `add loopback`/`add null` error "bad arp").

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::Chan;
use crate::mcslock::{Lock, LockNode};
use crate::Result;

use super::{format_ip, parse_ip, v4prefix_of};

/// NCACHE = 256 (arp.c:18) — the table bound; LRU eviction past it.
const NCACHE: usize = 256;

/// One view-table entry (the donor's Arpent's file-visible fields).
struct ArpEntry {
    ip: [u8; 16],
    mac: [u8; 6],
    /// state: true = OK, false = WAIT (the donor's arpstate[]).
    ok: bool,
    /// The medium name printed in the line's first field.
    medium: &'static [u8],
    /// The LRU stamp (the monotonic feed/write counter — never the wall
    /// clock; the table has no timeout semantics).
    last: u64,
}

struct ArpTable {
    entries: Vec<ArpEntry>,
    /// The monotonic stamp counter (the LRU time source).
    clock: u64,
}

static ARP: Lock<ArpTable> = Lock::new("deviparp", ArpTable { entries: Vec::new(), clock: 0 });

/// Insert-or-evict (LRU): at NCACHE the least-recently-stamped entry
/// dies (the donor's arpenter eviction shape).
fn insert(g: &mut ArpTable, e: ArpEntry) {
    if g.entries.len() >= NCACHE {
        let mut lru = 0;
        for (i, x) in g.entries.iter().enumerate() {
            if x.last < g.entries[lru].last {
                lru = i;
            }
        }
        g.entries.remove(lru);
    }
    g.entries.push(e);
}

/// Feed an entry: refresh the stamp on a hit (a reply completes a WAIT);
/// insert otherwise.  `mac` present → (ip→mac, OK); absent → (ip, WAIT).
fn feed(ip: [u8; 16], mac: Option<[u8; 6]>, medium: &'static [u8]) {
    let node = LockNode::new();
    let mut g = ARP.lock(&node);
    g.clock += 1;
    let stamp = g.clock;
    if let Some(e) = g.entries.iter_mut().find(|e| e.ip == ip) {
        e.last = stamp;
        if let Some(m) = mac {
            e.mac = m;
            e.ok = true;
        }
        return;
    }
    let ok = mac.is_some();
    let e = ArpEntry { ip, mac: mac.unwrap_or([0; 6]), ok, medium, last: stamp };
    insert(&mut g, e);
}

/// The snoop feed (phyadapt's ether adapter calls here from BOTH
/// receive() and transmit(); `tx` marks our own emissions — the TX-side
/// feedback that records our own solicitations as WAIT).  Parses the
/// frame with smoltcp's wire codecs (an allowed wire-codec use — never
/// a protocol engine, never a hand-rolled offset); an unparseable frame
/// is skipped silently (the frame itself still flows — the view is
/// observational, never a gate on delivery).
pub(crate) fn snoop_frame(frame: &[u8], tx: bool) {
    use smoltcp::wire::{
        ArpOperation, ArpPacket, ArpRepr, EthernetFrame, EthernetProtocol, Icmpv6Packet, Icmpv6Repr,
        IpProtocol, Ipv6Packet, NdiscRepr,
    };
    use smoltcp::phy::ChecksumCapabilities;
    let Ok(ef) = EthernetFrame::new_checked(frame) else { return };
    match ef.ethertype() {
        EthernetProtocol::Arp => {
            let Ok(pkt) = ArpPacket::new_checked(ef.payload()) else { return };
            let Ok(ArpRepr::EthernetIpv4 {
                operation,
                source_hardware_addr,
                source_protocol_addr,
                target_protocol_addr,
                ..
            }) = ArpRepr::parse(&pkt)
            else {
                return;
            };
            let spa = v4prefix_of(&source_protocol_addr.octets());
            match operation {
                // A reply/announcement: learn the sender (the donor's
                // arpenter learns from any arp packet's sender fields).
                ArpOperation::Reply => feed(spa, Some(source_hardware_addr.0), b"ether"),
                ArpOperation::Request => {
                    if source_protocol_addr == target_protocol_addr {
                        // The gratuitous announcement (spa == tpa —
                        // sendgarp's shape): record the announcer.
                        feed(spa, Some(source_hardware_addr.0), b"ether");
                    } else if tx {
                        // Our own solicitation: the TARGET is pending
                        // (WAIT) until a reply completes it.
                        feed(v4prefix_of(&target_protocol_addr.octets()), None, b"ether");
                    } else {
                        // Someone else's solicitation: learn the sender.
                        feed(spa, Some(source_hardware_addr.0), b"ether");
                    }
                }
                _ => {} // ArpOperation is #[non_exhaustive]
            }
        }
        EthernetProtocol::Ipv6 => {
            let Ok(v6) = Ipv6Packet::new_checked(ef.payload()) else { return };
            if v6.next_header() != IpProtocol::Icmpv6 {
                return;
            }
            let Ok(icmp) = Icmpv6Packet::new_checked(v6.payload()) else { return };
            // The wire frames carry valid checksums (smoltcp computes
            // ours; slirp's are computed too) — parse with checksum
            // validation on (the codec's shape).
            let Ok(repr) =
                Icmpv6Repr::parse(&v6.src_addr(), &v6.dst_addr(), &icmp, &ChecksumCapabilities::default())
            else {
                return;
            };
            match repr {
                Icmpv6Repr::Ndisc(NdiscRepr::NeighborSolicit { target_addr, lladdr }) => {
                    if tx {
                        // Our own NS: the target is pending.
                        feed(target_addr.octets(), None, b"ether");
                    } else if let Some(ll) = lladdr {
                        // Someone else's NS carrying a source-lladdr
                        // option: learn the solicitor (keyed by the v6
                        // SOURCE address, not the target).
                        if let Ok(mac) = <[u8; 6]>::try_from(ll.as_bytes()) {
                            feed(v6.src_addr().octets(), Some(mac), b"ether");
                        }
                    }
                }
                Icmpv6Repr::Ndisc(NdiscRepr::NeighborAdvert { target_addr, lladdr, .. }) => {
                    // An NA: the target is resolved at its target-lladdr.
                    if let Some(ll) = lladdr
                        && let Ok(mac) = <[u8; 6]>::try_from(ll.as_bytes())
                    {
                        feed(target_addr.octets(), Some(mac), b"ether");
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// The read (contract §2's arp chapter): fixed 90-byte lines; the
/// `offset % 90 != 0` gate returns 0 (arp.c:520-556).  `offset` u64::MAX
/// uses + advances the chan's streaming offset (the pread convention).
pub(crate) fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let off = if offset == u64::MAX { c.offset } else { offset };
    if off % 90 != 0 {
        return Ok(0);
    }
    let content = content();
    let start = (off as usize).min(content.len());
    let avail = &content[start..];
    let n = avail.len().min(buf.len());
    buf[..n].copy_from_slice(&avail[..n]);
    if offset == u64::MAX {
        c.offset += n as u64;
    }
    Ok(n as u64)
}

/// The listing's byte length (stat's length field): 90 per entry.
pub(crate) fn content_len() -> usize {
    let node = LockNode::new();
    ARP.lock(&node).entries.len() * 90
}

/// The read content: one 90-byte `"%-6.6s %-8.8s %-40.40I %-32.32s\n"`
/// line per entry (medium, state, ip, mac).
fn content() -> Vec<u8> {
    let node = LockNode::new();
    let g = ARP.lock(&node);
    let mut out = Vec::new();
    for e in &g.entries {
        let mut line = Vec::new();
        line.extend_from_slice(&super::pad_right(e.medium, 6));
        line.push(b' ');
        line.extend_from_slice(&super::pad_right(if e.ok { b"OK" } else { b"WAIT" }, 8));
        line.push(b' ');
        line.extend_from_slice(&super::pad_right(&format_ip(&e.ip), 40));
        line.push(b' ');
        let mut mac = Vec::new();
        for (i, &b) in e.mac.iter().enumerate() {
            if i > 0 {
                mac.push(b':');
            }
            mac.extend_from_slice(&super::hex2(b));
        }
        line.extend_from_slice(&super::pad_right(&mac, 32));
        line.push(b'\n');
        // The line is 90 bytes by construction (6+1+8+1+40+1+32+1).
        line.resize(90, b' ');
        out.extend_from_slice(&line[..90]);
    }
    out
}

/// The write (arp.c:425-516): `flush` / `add ether <ip> <mac>` /
/// `add <ip> <mac>` / `del <ip>`.
///
/// §12.5's divergence, honestly: `add` writes the VIEW only — no
/// resolution steering (smoltcp's cache self-manages).  The 3-field
/// form's medium pick is route-mediated (the donor's shape): no route →
/// "Destination unreachable"; a route picks the era's ether medium (the
/// only one with ARP machinery — `ares == nil` for loopback/null maps
/// to "bad arp", the donor's exact arm).
pub(crate) fn write(buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    // The donor's cmdbuf bound.
    let text = &buf[..n.min(63)];
    let fields: Vec<&[u8]> = text
        .split(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
        .filter(|w| !w.is_empty())
        .collect();
    if fields.is_empty() {
        return Err("bad arp");
    }
    match fields[0] {
        b"flush" => {
            let node = LockNode::new();
            let mut g = ARP.lock(&node);
            g.entries.clear();
            Ok(n as u64)
        }
        b"add" => {
            let (ip_s, mac_s) = match fields.len() {
                3 => {
                    // The 3-field form: the medium pick is route-mediated
                    // (the donor's v4lookup/v6lookup → r->ifc->medium).
                    let ip = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
                    let Some(_r) = super::route::lookup(&ip) else {
                        return Err("Destination unreachable");
                    };
                    // A found route to a real address rides the ether
                    // medium this era (the only one with ares); the
                    // ether ifc must be bound.
                    if !super::ipifc::ether_bound() {
                        return Err("bad arp"); // the ares == nil arm
                    }
                    (fields[1], fields[2])
                }
                4 => {
                    // The documented form: the medium name must resolve
                    // (ipfindmedium, nil → Ebadarp) AND carry ARP
                    // machinery (the donor's `type->ares == nil` arm —
                    // only ether has it this era).
                    if !super::medium::is_known(fields[1]) {
                        return Err("bad arp");
                    }
                    if fields[1] != b"ether" {
                        return Err("bad arp"); // ares == nil on loopback/null
                    }
                    (fields[2], fields[3])
                }
                _ => return Err("bad arg in system call"),
            };
            let ip = parse_ip(ip_s).ok_or("bad ip address syntax")?;
            let mac = parse_mac(mac_s).ok_or("bad arp")?;
            feed(ip, Some(mac), b"ether");
            Ok(n as u64)
        }
        b"del" => {
            if fields.len() != 2 {
                return Err("bad arg in system call");
            }
            let ip = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            let node = LockNode::new();
            let mut g = ARP.lock(&node);
            g.entries.retain(|e| e.ip != ip);
            Ok(n as u64)
        }
        _ => Err("bad arp"),
    }
}

/// Parse a colon-separated MAC (aa:bb:cc:dd:ee:ff) — the donor's
/// parsemac with maclen 6 (variable-width hex fields, the donor's
/// strtoul shape).
fn parse_mac(s: &[u8]) -> Option<[u8; 6]> {
    let mut out = [0u8; 6];
    let parts: Vec<&[u8]> = s.split(|&b| b == b':').collect();
    if parts.len() != 6 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() || p.len() > 2 {
            return None;
        }
        let mut v: u8 = 0;
        for &b in p.iter() {
            let d = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return None,
            };
            v = v.checked_mul(16)?.checked_add(d)?;
        }
        out[i] = v;
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Host-pin accessors (devip_tests.rs — 08-06's ether pins; the table is a
// process-global static the pins serialize against).
// ---------------------------------------------------------------------------

/// Flush the view table (pin setup — the static survives across pins).
#[cfg(test)]
pub(crate) fn test_flush() {
    let node = LockNode::new();
    let mut g = ARP.lock(&node);
    g.entries.clear();
}

/// The entry for an address: (mac, ok) — the pin's feed assertion.
#[cfg(test)]
pub(crate) fn test_lookup(ip: &[u8; 16]) -> Option<([u8; 6], bool)> {
    let node = LockNode::new();
    let g = ARP.lock(&node);
    g.entries.iter().find(|e| &e.ip == ip).map(|e| (e.mac, e.ok))
}

/// The entry count (the pin's table assertions).
#[cfg(test)]
pub(crate) fn test_len() -> usize {
    let node = LockNode::new();
    ARP.lock(&node).entries.len()
}
