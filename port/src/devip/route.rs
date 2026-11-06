//! devip/route.rs — the `/net/iproute` route forest (contract §2's
//! iproute chapter; the donor's `sys/src/9k/ip/iproute.c` — read-only
//! semantics reference, never copied).
//!
//! THE TABLE: devip-owned tagged entries (the donor's route structs carry
//! the file-visible fields: target/mask/nexthop, the version arm `4`/`6`,
//! the `i`/`u`/`b`/`m`/`p` flags, the ≤4-char tag bound by the `tag`
//! verb).  Lookup is LONGEST-MASK match (`dst AND mask == target`; the
//! longest mask wins — routetype's walk shape).
//!
//! THE SMOLTCP MIRROR (contract §2's `add` verb mapping): a DEFAULT route
//! (target and mask both all-zero) mirrors into the seam's smoltcp
//! `Routes` (`add_default_ipv4_route` / `add_default_ipv6_route`;
//! `remove` of a default mirrors out).  Non-default routes are the
//! devip-owned record only (smoltcp's socket dispatch consults its own
//! Routes for the gateway; on-link prefixes ride the interface's
//! address's prefix — the loopback era's single-subnet shape).  The
//! mirror never panics across the file boundary: a full smoltcp route
//! storage maps to a devip error (the panic-boundary rule).
//!
//! The verbs (iproute.c:802-880): `flush` / `tag <string>` /
//! `add <target> <mask> <nexthop>` (replace on same target+mask) /
//! `remove <target> <mask>` / `route <target>` (console-print the chosen
//! route — the debugging aid).  The read emits the contract's line
//! format: `"%-15I %-4M %-15I %4.4s %4.4s %3s\n"` (target mask nexthop
//! flags tag ifc).

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use smoltcp::wire::{Ipv4Address, Ipv6Address};

use crate::mcslock::{Lock, LockNode};
use crate::Result;

use super::{format_ip, format_mask, mask_ip, parse_ip, parse_ipmask, stack, v4_of};

/// One route (the file-visible fields; `ifc` marks a local-interface
/// route — ipifc's add path carries the bound interface's devname).
#[derive(Clone)]
pub(crate) struct Route {
    pub(crate) target: [u8; 16],
    pub(crate) mask: [u8; 16],
    pub(crate) nexthop: [u8; 16],
    /// v4 route (the `4` flag) vs v6 (`6`).
    pub(crate) v4: bool,
    /// local interface route (the `i` flag; set for ipifc add's routes).
    pub(crate) ifc: bool,
    /// local unicast (the `u` flag).
    pub(crate) uni: bool,
    /// The originating tag (the `tag` verb's binding).
    pub(crate) tag: [u8; 4],
}

struct RouteTable {
    routes: Vec<Route>,
    /// The current route tag (the `tag` verb's record; the donor binds it
    /// per-fd via IPaux — r9's Chan has no aux slot this era, so the tag
    /// is per-stack: behaviorally identical for every phase consumer —
    /// ipconfig's route adds never tag).
    tag: [u8; 4],
}

static ROUTES: Lock<RouteTable> = Lock::new("deviproute", RouteTable { routes: Vec::new(), tag: *b"    " });

/// The mask's prefix length (the longest-match key): count leading 1
/// bits; a non-contiguous mask counts its leading run (the donor's
/// formider shape — longest match is by prefix length).
fn mask_len(mask: &[u8; 16]) -> usize {
    // A v4-MAPPED mask counts its 4-byte tail's prefix (the v4prefix
    // marker's 0xff 0xff bytes are NOT mask bits); a v6 mask counts the
    // whole 16 bytes.  Non-contiguous masks count their leading run.
    let bytes: &[u8] = if super::is_v4(mask) { &mask[12..16] } else { mask };
    let mut n = 0;
    'outer: for &b in bytes {
        for i in 0..8 {
            if b & (0x80 >> i) != 0 {
                n += 1;
            } else {
                break 'outer;
            }
        }
    }
    n
}

/// The longest-mask match (`dst AND mask == target`; the donor's route
/// lookup).  Returns the winning route.
pub(crate) fn lookup(target: &[u8; 16]) -> Option<Route> {
    let node = LockNode::new();
    let g = ROUTES.lock(&node);
    let mut best: Option<&Route> = None;
    for r in &g.routes {
        if mask_ip(target, &r.mask) == r.target {
            let better = match best {
                None => true,
                Some(b) => mask_len(&r.mask) > mask_len(&b.mask),
            };
            if better {
                best = Some(r);
            }
        }
    }
    best.cloned()
}

/// Is the route the DEFAULT?  The all-zero mask in EITHER spelling
/// (16-byte all-zero or the v4-mapped zero — parse_ip("0.0.0.0") yields
/// the v4-mapped form whose marker bytes are 0xff 0xff, so a naive
/// all-zero compare never fires for the written form).
fn is_default(target: &[u8; 16], mask: &[u8; 16]) -> bool {
    let zero_v4 = super::v4prefix_of(&[0, 0, 0, 0]);
    let zeroish = |a: &[u8; 16]| a.iter().all(|&b| b == 0) || a == &zero_v4;
    zeroish(target) && zeroish(mask)
}

/// Mirror a default route into the seam's smoltcp Routes (the contract's
/// `add` mapping).  An unspecified nexthop skips the mirror (the donor's
/// `add 0.0.0.0 0.0.0.0 0.0.0.0` is a no-op-ish record — and smoltcp must
/// never see a non-unicast gateway).  RouteTableFull maps to a devip
/// error — a smoltcp panic never crosses the file boundary (the
/// panic-boundary rule).
fn mirror_default_add(nexthop: &[u8; 16]) -> Result<()> {
    if nexthop.iter().all(|&b| b == 0) {
        return Ok(());
    }
    let mut g = stack::STACK.enter().expect("devip: stack contended");
    let Some(core) = g.core.as_mut() else {
        return Ok(()); // no bound medium yet — the record stands; the mirror lands at bind
    };
    if let Some(gw4) = v4_of(nexthop) {
        core.iface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::from(gw4))
            .map_err(|_| "no free devices")?;
    } else {
        let mut gw6 = [0u8; 16];
        gw6.copy_from_slice(nexthop);
        core.iface
            .routes_mut()
            .add_default_ipv6_route(Ipv6Address::from(gw6))
            .map_err(|_| "no free devices")?;
    }
    Ok(())
}

/// Mirror a default-route removal out of smoltcp's Routes.
fn mirror_default_remove(v4: bool) {
    let mut g = stack::STACK.enter().expect("devip: stack contended");
    let Some(core) = g.core.as_mut() else { return };
    if v4 {
        core.iface.routes_mut().remove_default_ipv4_route();
    } else {
        core.iface.routes_mut().remove_default_ipv6_route();
    }
}

/// The route add (the devip-side table of record; the donor's
/// v4addroute/v6addroute shape — replace on same target+mask; the current
/// tag binds).  A default route mirrors into smoltcp's Routes.
pub(crate) fn add(target: [u8; 16], mask: [u8; 16], nexthop: [u8; 16], ifc: bool) -> Result<()> {
    let v4 = v4_of(&target).is_some();
    {
        let node = LockNode::new();
        let mut g = ROUTES.lock(&node);
        let tag = g.tag;
        g.routes.retain(|r| !(r.target == target && r.mask == mask));
        g.routes.push(Route { target, mask, nexthop, v4, ifc, uni: true, tag });
    }
    if is_default(&target, &mask) {
        mirror_default_add(&nexthop)?;
    }
    Ok(())
}

/// The route remove (routewrite's remove arm).
pub(crate) fn remove(target: [u8; 16], mask: [u8; 16]) {
    let was_default_v4 = is_default(&target, &mask) && v4_of(&target).is_some();
    {
        let node = LockNode::new();
        let mut g = ROUTES.lock(&node);
        g.routes.retain(|r| !(r.target == target && r.mask == mask));
    }
    if is_default(&target, &mask) {
        mirror_default_remove(was_default_v4);
    }
}

/// The read content (contract §2): one line per route,
/// `"%-15I %-4M %-15I %4.4s %4.4s %3s\n"` (target mask nexthop flags tag
/// ifc).
pub(crate) fn content() -> Vec<u8> {
    let node = LockNode::new();
    let g = ROUTES.lock(&node);
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

/// The write verbs (contract §2): `flush` / `tag <string>` /
/// `add <target> <mask> <nexthop>` / `remove <target> <mask>` /
/// `route <target>` (the console-print debug aid — longest-match now,
/// matching the donor's lookup).
pub(crate) fn ctl(buf: &[u8]) -> Result<u64> {
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
            let had_v4 = {
                let node = LockNode::new();
                let g = ROUTES.lock(&node);
                g.routes.iter().any(|r| is_default(&r.target, &r.mask) && r.v4)
            };
            let had_v6 = {
                let node = LockNode::new();
                let g = ROUTES.lock(&node);
                g.routes.iter().any(|r| is_default(&r.target, &r.mask) && !r.v4)
            };
            {
                let node = LockNode::new();
                ROUTES.lock(&node).routes.clear();
            }
            // Flush mirrors the defaults out too.
            if had_v4 {
                mirror_default_remove(true);
            }
            if had_v6 {
                mirror_default_remove(false);
            }
            Ok(n as u64)
        }
        b"tag" => {
            if fields.len() != 2 {
                return Err("bad arg in system call");
            }
            let mut tag = *b"    ";
            let src = &fields[1][..fields[1].len().min(4)];
            tag[..src.len()].copy_from_slice(src);
            let node = LockNode::new();
            ROUTES.lock(&node).tag = tag;
            Ok(n as u64)
        }
        b"add" => {
            if fields.len() != 4 {
                return Err("bad arg in system call");
            }
            let target = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            let mask = parse_ipmask(fields[2]).ok_or("bad ip address syntax")?;
            let nexthop = parse_ip(fields[3]).ok_or("bad ip address syntax")?;
            add(target, mask, nexthop, false)?;
            Ok(n as u64)
        }
        b"remove" => {
            if fields.len() != 3 {
                return Err("bad arg in system call");
            }
            let target = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            let mask = parse_ipmask(fields[2]).ok_or("bad ip address syntax")?;
            remove(target, mask);
            Ok(n as u64)
        }
        b"route" => {
            if fields.len() != 2 {
                return Err("bad arg in system call");
            }
            let target = parse_ip(fields[1]).ok_or("bad ip address syntax")?;
            match lookup(&target) {
                Some(r) => {
                    crate::println!(
                        "devip: route {} {} {}",
                        alloc::string::String::from_utf8_lossy(&format_ip(&r.target)),
                        alloc::string::String::from_utf8_lossy(&format_mask(&r.mask)),
                        alloc::string::String::from_utf8_lossy(&format_ip(&r.nexthop))
                    );
                }
                None => crate::println!("devip: no route"),
            }
            Ok(n as u64)
        }
        _ => Err("bad arg in system call"),
    }
}

fn pad_right(s: &[u8], width: usize) -> Vec<u8> {
    // C's `%-<width>M`/`%-<width>I` semantics: PAD to the width, never
    // truncate — a longer value overflows the column (the donor's print;
    // e.g. the mask "255.255.255.255" at %-4M prints full).  The flags
    // (≤4) and tag (≤4 by construction) columns never overflow.
    let mut out = Vec::new();
    out.extend_from_slice(s);
    while out.len() < width {
        out.push(b' ');
    }
    out
}
