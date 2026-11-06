//! devip/ipifc.rs — the interface protocol (contract §9; donor
//! `ipifc.c`).  This plan lands the LOOPBACK leg: clone, then ctl verbs
//! `bind loopback` / `add <local> [mask]` / `remove <local> <mask>` /
//! `unbind`; `bind ether` is 08-06 (the ethermedium binding consumes
//! Phase 7's devether); `bind pkt`/`bind netdev` are ABSENT per D-08
//! (they hit "unknown interface type" like any unknown medium).
//!
//! THE ADDRESS MODEL: the devip-visible tables (the self-table behind
//! /net/ipselftab, the route table behind /net/iproute) are devip-owned
//! and updated here at add/remove (the donor's addselfcache/v4addroute
//! shape, including the four v4 broadcast/network self-cache entries);
//! smoltcp learns only the real UNICAST address through
//! `update_ip_addrs` — the panic-boundary rule (P-F): a non-unicast
//! address (`0.0.0.0`/`::`) is ABSORBED devip-side as the
//! DHCP-in-progress marker (contract §12.6 — recorded on the lifc,
//! never passed through; smoltcp's update_ip_addrs panics on it), and
//! overflow past IFACE_MAX_ADDR_COUNT maps to a devip error string,
//! never a panic.
//!
//! BIND PERSISTENCE: the loopback medium's `unbindonclose` is 0 in the
//! donor (loopbackmedium.c's Medium struct leaves it unset), so the
//! binding SURVIVES a ctl-fd close — the gate's `bind`/`add` writes ride
//! separate open/close cycles and must find the interface still bound.
//! `close_conv` honors exactly that flag discipline.

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use smoltcp::wire::{IpCidr, Ipv4Cidr};

use crate::mcslock::{Lock, LockNode};
use crate::Result;

use super::conv::{self, Proto};
use super::medium;
use super::stack;
use super::{IP_NOADDR, atoi, format_ip, format_mask, is_v4, mask_ip, parse_ip, parse_ipmask, v4_of, v4prefix_of};

/// smoltcp's compile-time interface address cap (the crate's default
/// IFACE_MAX_ADDR_COUNT = 8 — research-verified).  Overflow maps to a
/// devip error (P-F), never an smoltcp panic.
const IFACE_MAX_ADDR_COUNT: usize = 8;

/// A logical interface address (the donor's Iplifc): local/mask/remote
/// + the lifetimes (validlt/preflt; ~0 = forever).
pub(crate) struct Lifc {
    pub(crate) local: [u8; 16],
    pub(crate) mask: [u8; 16],
    pub(crate) remote: [u8; 16],
    pub(crate) validlt: u64,
    pub(crate) preflt: u64,
    /// `try` adds a tentative v6 address for DAD (08-06's consumer).
    pub(crate) tentative: bool,
    /// The absorbed non-unicast marker (contract §12.6): the address is
    /// recorded here and in the self-table but NEVER passed to smoltcp's
    /// update_ip_addrs (it panics on non-unicast).
    pub(crate) absorbed: bool,
}

/// The one bound interface this era (contract §12.4's
/// one-Interface-per-bound-ipifc — the ether leg's second core joins in
/// 08-06).  The seam's `StackState.core` is the loopback core's home;
/// this record is the devip-side view (the conv that owns the binding,
/// the medium name, the lifc list, the RA parameters).
pub(crate) struct Ifc {
    /// The binding ipifc conv id.
    conv: usize,
    medium: medium::MediumKind,
    /// The bound device name (the donor's `ifc->dev`: medium name +
    /// conv id — `loopback0`).
    devname: Vec<u8>,
    maxtu: u32,
    lifcs: Vec<Lifc>,
    // The RA parameters (ipifcbind defaults, ipifc.c:143-153).  No
    // consumer this era — carried for the status line's fidelity.
    mflag: bool,
    oflag: bool,
    maxraint: u32,
    minraint: u32,
    linkmtu: u32,
    reachtime: u32,
    rxmitra: u32,
    ttl: u32,
    routerlt: u32,
    // Packet counters (the status line's pktin/pktout/errin/errout).
    pktin: u64,
    pktout: u64,
    errin: u64,
    errout: u64,
}

static IFC: Lock<Option<Ifc>> = Lock::new("devipifc", None);

/// Bind the conv to a medium (`bind <type> [path]` — ipifcbind,
/// ipifc.c:103-167).  The medium registry (medium.rs) resolves the name;
/// the seam core install is the medium's bind action.
pub(crate) fn bind(id: usize, args: &[&[u8]]) -> Result<()> {
    if args.is_empty() {
        return Err("bad arg in system call");
    }
    let kind = medium::find(args[0])?;
    let node = LockNode::new();
    let mut guard = IFC.lock(&node);
    if guard.is_some() {
        return Err("interface already bound");
    }
    // The conv must be unbound too (a re-bind of a bound conv errors
    // the same way — the donor's `ifc->medium != nil` check).
    {
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        if t.table(Proto::Ipifc)[id].inuse == 0 {
            return Err("no free devices");
        }
    }
    let devname = medium::bind_medium(kind, id)?;
    *guard = Some(Ifc {
        conv: id,
        medium: kind,
        devname,
        maxtu: medium::maxtu(kind),
        lifcs: Vec::new(),
        mflag: false,
        oflag: false,
        maxraint: 600000,
        minraint: 200000,
        linkmtu: 0,
        reachtime: 0,
        rxmitra: 0,
        ttl: conv::MAX_TTL,
        routerlt: 3 * 600000,
        pktin: 0,
        pktout: 0,
        errin: 0,
        errout: 0,
    });
    Ok(())
}

/// Unbind (`unbind` — ipifcunbind, ipifc.c:173-216): drop every lifc
/// (with its self-table + route entries), drop the seam core, drop the
/// interface record.
pub(crate) fn unbind(id: usize) -> Result<()> {
    let node = LockNode::new();
    let mut guard = IFC.lock(&node);
    let Some(ifc) = guard.as_ref() else { return Err("ipifc not yet bound to device") };
    if ifc.conv != id {
        return Err("ipifc not yet bound to device");
    }
    let lifcs = core::mem::take(&mut guard.as_mut().unwrap().lifcs);
    for lifc in &lifcs {
        remove_address(lifc);
    }
    *guard = None;
    // The seam core goes away (the loopback iface is dropped).
    let mut g = stack::STACK.enter().expect("devip: stack contended");
    g.core = None;
    Ok(())
}

/// The proto close hook (conv.rs's closeconv calls here): unbind ONLY
/// when the medium's unbindonclose flag is set — loopback's is 0, so
/// the binding PERSISTS across ctl close (the gate's bind → close →
/// add flow depends on it).
pub(crate) fn close_conv(_t: &mut conv::ConvTables, _id: usize) {
    // loopback: unbindonclose == 0 → no action (the donor's ipifcclose
    // checks the flag and does nothing).
}

/// `add <local> [mask]` (ipifcadd, ipifc.c:368-530): validate BEFORE
/// smoltcp (P-F), then the lifc + self-table + route + smoltcp updates.
/// Defaults: class mask for mask, `local AND mask` for remote.
/// A duplicate add of the same address is a no-op (the donor's
/// already-a-local-address arm, ipifc.c:423-437).
pub(crate) fn add(id: usize, args: &[&[u8]], tentative: bool) -> Result<()> {
    let node = LockNode::new();
    let mut guard = IFC.lock(&node);
    let Some(ifc) = guard.as_mut() else { return Err("ipifc not yet bound to device") };
    if ifc.conv != id {
        return Err("ipifc not yet bound to device");
    }
    let ip = parse_ip(args.first().ok_or("bad arg in system call")?).ok_or("bad ip address syntax")?;
    let mask = match args.len() {
        1 => defmask(&ip),
        _ => parse_ipmask(args[1]).ok_or("bad ip address syntax")?,
    };
    // [remote [mtu [proxy]]] — parse remote when present; the mtu arm
    // honors the medium's bounds (ipifc.c:391-395), proxy is a
    // recorded no-op flag this era.
    if args.len() >= 3 {
        let _remote = parse_ip(args[2]).ok_or("bad ip address syntax")?;
    }
    if args.len() >= 4 {
        let mtu = atoi(args[3]);
        if mtu >= medium::mintu(ifc.medium) && mtu <= medium::maxtu(ifc.medium) {
            ifc.maxtu = mtu;
        }
    }
    let remote = mask_ip(&ip, &mask);
    // The duplicate no-op.
    for l in ifc.lifcs.iter_mut() {
        if l.local == ip {
            l.tentative = tentative && !is_v4(&ip);
            return Ok(());
        }
    }
    // P-F: the non-unicast absorption (0.0.0.0 / ::) — record the lifc
    // (the DHCP-in-progress marker), update the devip-visible tables,
    // NEVER call smoltcp's update_ip_addrs with it.
    let unspecified = ip == IP_NOADDR || (is_v4(&ip) && v4_of(&ip) == Some([0, 0, 0, 0]));
    if unspecified {
        // A note-worthy event (D-07): the absorption is deliberate but
        // invisible on the wire — it shows up in netlog under `ip`.
        super::netlog::emit(
            super::netlog::LOG_IP,
            alloc::format!("ipifc: absorbed add of the unspecified address (the DHCP-in-progress marker)").into_bytes(),
        );
        super::netlog::netlog_dbg!(super::netlog::LOG_IP, "devip: absorbed 0.0.0.0 add");
    }
    // The cap check BEFORE any mutation: overflow maps to a devip
    // error — never an smoltcp panic (P-F).
    if !unspecified && ifc.lifcs.iter().filter(|l| !l.absorbed).count() >= IFACE_MAX_ADDR_COUNT {
        return Err("devip: too many addresses");
    }
    let lifc = Lifc {
        local: ip,
        mask,
        remote,
        validlt: u64::MAX,
        preflt: u64::MAX,
        tentative: tentative && !is_v4(&ip),
        absorbed: unspecified,
    };
    if !unspecified {
        // smoltcp learns the unicast address.
        let mut g = stack::STACK.enter().expect("devip: stack contended");
        let core = g.core.as_mut().ok_or("devip: no bound medium")?;
        if is_v4(&ip) {
            let v4 = v4_of(&ip).unwrap();
            let pfx = mask_prefix_v4(&mask);
            let cidr = Ipv4Cidr::new(v4.into(), pfx);
            core.iface.update_ip_addrs(|addrs| {
                let _ = addrs.push(IpCidr::Ipv4(cidr));
            });
        }
        // (v6 addresses join with the ether leg in 08-06.)
    }
    // The devip-visible tables (the donor's addselfcache/v4addroute
    // tails), then the lifc joins the interface record.
    add_address(&lifc)?;
    ifc.lifcs.push(lifc);
    Ok(())
}

/// `remove <local> <mask>` (ipifcrem): drop the lifc and its table
/// entries; not-found → the donor's "lifc not found" arm maps to a
/// plain error.
pub(crate) fn remove(id: usize, args: &[&[u8]]) -> Result<()> {
    let node = LockNode::new();
    let mut guard = IFC.lock(&node);
    let Some(ifc) = guard.as_mut() else { return Err("ipifc not yet bound to device") };
    if ifc.conv != id {
        return Err("ipifc not yet bound to device");
    }
    if args.len() != 2 {
        return Err("bad arg in system call");
    }
    let ip = parse_ip(args[0]).ok_or("bad ip address syntax")?;
    let mask = parse_ipmask(args[1]).ok_or("bad ip address syntax")?;
    let Some(pos) = ifc.lifcs.iter().position(|l| l.local == ip && l.mask == mask) else {
        return Err("devip: no such local address");
    };
    let lifc = ifc.lifcs.remove(pos);
    remove_address(&lifc);
    // smoltcp's address list drops the unicast address too.
    if !lifc.absorbed && is_v4(&lifc.local) {
        let v4 = v4_of(&lifc.local).unwrap();
        let mut g = stack::STACK.enter().expect("devip: stack contended");
        if let Some(core) = g.core.as_mut() {
            core.iface.update_ip_addrs(|addrs| {
                if let Some(i) = addrs.iter().position(|a| {
                    matches!(a, IpCidr::Ipv4(c) if c.address().octets() == v4)
                }) {
                    addrs.remove(i);
                }
            });
        }
    }
    Ok(())
}

/// The ipifc ctl dispatch (ipifcctl, ipifc.c:758-795): unknown verb →
/// "unsupported ctl" (incl. `bind pkt`/`bind netdev` — ABSENT per D-8).
pub(crate) fn ctl(id: usize, verb: &[u8], args: &[&[u8]]) -> Result<()> {
    match verb {
        b"bind" => bind(id, args),
        b"add" => add(id, args, false),
        b"try" => add(id, args, true),
        b"remove" => remove(id, args),
        b"unbind" => unbind(id),
        b"mtu" => {
            let node = LockNode::new();
            let mut guard = IFC.lock(&node);
            let Some(ifc) = guard.as_mut() else { return Err("ipifc not yet bound to device") };
            if args.is_empty() {
                return Err("bad arg in system call");
            }
            let mtu = atoi(args[0]);
            if mtu < medium::mintu(ifc.medium) || mtu > medium::maxtu(ifc.medium) {
                return Err("bad arg in system call");
            }
            ifc.maxtu = mtu;
            Ok(())
        }
        b"reassemble" => Ok(()), // recorded no-op this era (no forwarding)
        b"iprouting" => Ok(()),  // forwarding toggle — no routing this era
        _ => Err("unsupported ctl"),
    }
}

/// findlocalip (devip.c:723-726): the local address closest to the
/// remote.  Address-class demux (contract §12.4): 127/8 and ::1 → the
/// loopback iface's address; otherwise the first configured v4 address.
/// IPnoaddr when nothing is bound/configured.
pub(crate) fn find_local_ip(raddr: &[u8; 16]) -> [u8; 16] {
    let node = LockNode::new();
    let guard = IFC.lock(&node);
    let Some(ifc) = guard.as_ref() else { return IP_NOADDR };
    let loopback = is_loopback_addr(raddr);
    for l in &ifc.lifcs {
        if l.absorbed {
            continue;
        }
        if loopback && is_loopback_addr(&l.local) {
            return l.local;
        }
    }
    for l in &ifc.lifcs {
        if l.absorbed {
            continue;
        }
        if is_v4(&l.local) {
            return l.local;
        }
    }
    IP_NOADDR
}

/// Is the address the loopback class (127/8 v4-mapped or ::1)?
fn is_loopback_addr(a: &[u8; 16]) -> bool {
    if let Some(v) = v4_of(a) {
        return v[0] == 127;
    }
    a[..15].iter().all(|&b| b == 0) && a[15] == 1
}

/// ipforme (ipifc.c:1083-1098): is the address one of ours (the
/// self-table), Runi-typed?  Used by setladdrport's "not a local IP
/// address" check.
pub(crate) fn ip_for_me(addr: &[u8; 16]) -> bool {
    super::selftab_contains(addr) || super::selftab_acceptall()
}

/// The ipifc status read (ipifcstate, ipifc.c:218-249): the device
/// line, then one per-address line.  **[man-vs-src]**: the source
/// format is the contract.
pub(crate) fn status(id: usize) -> Vec<u8> {
    let node = LockNode::new();
    let guard = IFC.lock(&node);
    let mut out = Vec::new();
    let Some(ifc) = guard.as_ref() else {
        out.extend_from_slice(b"\n");
        return out;
    };
    if ifc.conv != id {
        out.extend_from_slice(b"\n");
        return out;
    }
    out.extend_from_slice(b"device ");
    out.extend_from_slice(&ifc.devname);
    out.extend_from_slice(b" maxtu ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.maxtu)));
    out.extend_from_slice(b" sendra 0 recvra 0");
    out.extend_from_slice(b" mflag ");
    out.extend_from_slice(if ifc.mflag { b"1" } else { b"0" });
    out.extend_from_slice(b" oflag ");
    out.extend_from_slice(if ifc.oflag { b"1" } else { b"0" });
    out.extend_from_slice(b" maxraint ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.maxraint)));
    out.extend_from_slice(b" minraint ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.minraint)));
    out.extend_from_slice(b" linkmtu ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.linkmtu)));
    out.extend_from_slice(b" reachtime ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.reachtime)));
    out.extend_from_slice(b" rxmitra ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.rxmitra)));
    out.extend_from_slice(b" ttl ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.ttl)));
    out.extend_from_slice(b" routerlt ");
    out.extend_from_slice(&super::proto::dec(u64::from(ifc.routerlt)));
    out.extend_from_slice(b" pktin ");
    out.extend_from_slice(&super::proto::dec(ifc.pktin));
    out.extend_from_slice(b" pktout ");
    out.extend_from_slice(&super::proto::dec(ifc.pktout));
    out.extend_from_slice(b" errin ");
    out.extend_from_slice(&super::proto::dec(ifc.errin));
    out.extend_from_slice(b" errout ");
    out.extend_from_slice(&super::proto::dec(ifc.errout));
    out.push(b'\n');
    for l in &ifc.lifcs {
        // "\t%-40I %-10M %-40I %-12lud %-12lud\n" (slineformat)
        out.push(b'\t');
        out.extend_from_slice(&pad_right(&format_ip(&l.local), 40));
        out.push(b' ');
        out.extend_from_slice(&pad_right(&format_mask(&l.mask), 10));
        out.push(b' ');
        out.extend_from_slice(&pad_right(&format_ip(&l.remote), 40));
        out.push(b' ');
        let lt = if l.validlt == u64::MAX { 4294967295 } else { l.validlt };
        out.extend_from_slice(&pad_left(&super::proto::dec(lt), 12));
        out.push(b' ');
        let pt = if l.preflt == u64::MAX { 4294967295 } else { l.preflt };
        out.extend_from_slice(&pad_left(&super::proto::dec(pt), 12));
        out.push(b'\n');
    }
    out
}

/// The ipifc `local` read (ipifclocal, ipifc.c:250-275): per-address
/// `"%-40.40I ->"` + the self-IPs linked to it (the loopback leg: the
/// address itself).
pub(crate) fn local(id: usize) -> Vec<u8> {
    let node = LockNode::new();
    let guard = IFC.lock(&node);
    let mut out = Vec::new();
    let Some(ifc) = guard.as_ref() else { return out };
    if ifc.conv != id {
        return out;
    }
    for l in &ifc.lifcs {
        out.extend_from_slice(&pad_right(&format_ip(&l.local), 40));
        out.extend_from_slice(b" ->");
        out.push(b'\n');
    }
    out
}

// ---------------------------------------------------------------------------
// The devip-visible address tables (the donor's addselfcache/v4addroute
// shape — devip.rs serves the /net/ipselftab and /net/iproute reads).
// ---------------------------------------------------------------------------

/// Add the lifc's entries to the self-table + route table (ipifcadd's
/// tails): the unicast entry (Runi), and for v4 the four
/// broadcast/network self-cache entries + the local route.  The local
/// route add can only fail on smoltcp's route-table overflow (never
/// mirrored — it is not a default route); propagate honestly.
fn add_address(lifc: &Lifc) -> Result<()> {
    // The unicast self entry.
    super::selftab_add(lifc.local, false);
    // The local route (ipifcadd's v4addroute: target = remote (the
    // net), mask, nexthop = remote, Rifc|Runi).
    super::route::add(lifc.remote, lifc.mask, lifc.remote, true)?;
    if is_v4(&lifc.local) || lifc.local == IP_NOADDR {
        // The v4 self-cache family (ipifc.c:481-504): subnet-directed
        // broadcast + network with the GIVEN mask, network-directed
        // with the class mask, and the all-ones broadcast.  Duplicate
        // adds collapse (the self-table's exists check).
        let mask = lifc.mask;
        let bcast1 = bor_not(&lifc.local, &mask);
        let net1 = mask_ip(&lifc.local, &mask);
        super::selftab_add(bcast1, true);
        super::selftab_add(net1, true);
        let dm = defmask(&lifc.local);
        let bcast2 = bor_not(&lifc.local, &dm);
        let net2 = mask_ip(&lifc.local, &dm);
        super::selftab_add(bcast2, true);
        super::selftab_add(net2, true);
        super::selftab_add(v4prefix_of(&[255, 255, 255, 255]), true);
    }
    Ok(())
}

/// Remove the lifc's entries (ipifcremlifc's shape).
fn remove_address(lifc: &Lifc) {
    super::selftab_remove(lifc.local);
    super::route::remove(lifc.remote, lifc.mask);
    if is_v4(&lifc.local) || lifc.local == IP_NOADDR {
        let mask = lifc.mask;
        super::selftab_remove(bor_not(&lifc.local, &mask));
        super::selftab_remove(mask_ip(&lifc.local, &mask));
        let dm = defmask(&lifc.local);
        super::selftab_remove(bor_not(&lifc.local, &dm));
        super::selftab_remove(mask_ip(&lifc.local, &dm));
        super::selftab_remove(v4prefix_of(&[255, 255, 255, 255]));
    }
}

/// (a & mask) | ~mask — the broadcast address for the mask.
fn bor_not(a: &[u8; 16], mask: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = (a[i] & mask[i]) | !mask[i];
    }
    out
}

/// The classful default mask (the donor's defmask): v4-mapped class
/// A/B/C by the first byte; v6 → all-bits.
fn defmask(ip: &[u8; 16]) -> [u8; 16] {
    let Some(v) = v4_of(ip) else { return [0xff; 16] };
    let m = match v[0] {
        0..=127 => [255, 0, 0, 0],
        128..=191 => [255, 255, 0, 0],
        _ => [255, 255, 255, 0],
    };
    v4prefix_of(&m)
}

/// The prefix length of a v4-mapped mask (count of 1 bits in the v4
/// tail).
fn mask_prefix_v4(mask: &[u8; 16]) -> u8 {
    let mut n = 0u8;
    for &b in &mask[12..16] {
        n += b.count_ones() as u8;
    }
    n
}

/// Left-padding helper for the status line's `%12lud` fields.
fn pad_left(s: &[u8], width: usize) -> Vec<u8> {
    let mut out = Vec::new();
    if s.len() < width {
        out.resize(width - s.len(), b' ');
    }
    out.extend_from_slice(s);
    out
}

/// Right-padding helper for the status line's `%-40I`/`%-10M` fields.
fn pad_right(s: &[u8], width: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(s);
    if s.len() < width {
        out.resize(width, b' ');
    }
    out
}
