//! devip/proto.rs — the per-protocol modules (the donor's `Proto`
//! vtable mapped to free functions per proto; contract §4's generic ctl
//! verbs + the per-proto chapters).  This file carries **icmp** (08-04's
//! NETIP-03 ping leg), **udp** (08-04's udp echo leg), and **tcp**
//! (08-05: the accept pool + the blocking connect bridge + close/hangup/
//! hangupxmit + the status/stats files); icmpv6 lands in 08-06.
//!
//! THE CTL DISPATCH (devip.c:1115-1176 verbatim shape): a 64-byte-bounded
//! tokenizer (T-08-06 — the donor's cmdbuf discipline per the plan's
//! threat register; parsecmd's field split on " \t\r\n"), the generic
//! verbs first, then the proto's own ctl, then the exact error strings
//! ("short control request" on an empty write, "unknown control request"
//! on an unrecognized verb).
//!
//! THE PANIC-BOUNDARY RULE (contract §11): every smoltcp call site
//! validates FIRST and maps failure onto a devip error string — a kernel
//! panic from a ctl write or a packet is a gate red (V5).  Concretely:
//! hop_limit 0 is never passed to smoltcp (it panics); socket-buffer
//! pressure maps to the blocking write shape, never a panic.
//!
//! ICMP (contract §7): the conv's lport IS the ICMP ident — on write the
//! kernel OVERWRITES the packet's ident field with c->lport (icmpkick,
//! icmp.c:186) and zeroes the checksum (smoltcp recomputes at emit); the
//! read delivers the packet WITH its 20-byte IPv4 header area
//! (ICMP_IPSIZE, icmp.c's full-block qpass + ping.c's
//! `geticmp(buf)=buf+20` — the phase research's "raw Icmphdr" reading is
//! corrected here against the source: BOTH directions carry the prefix).
//! smoltcp mapping: `icmp::Endpoint::Ident(lport)` accepts EchoRequest
//! AND EchoReply with the matching ident — ping receives its own
//! looped-back request and filters by type (recorded in §7).

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, Ordering};

use smoltcp::socket::icmp;
use smoltcp::socket::tcp;
use smoltcp::socket::udp;
use smoltcp::storage::{PacketBuffer, PacketMetadata};
use smoltcp::time::Duration;
use smoltcp::wire::{IpAddress, IpEndpoint, IpListenEndpoint, Ipv4Address};

use crate::mcslock::LockNode;
use crate::Result;

use super::conv::{self, ConvState, Proto};
use super::stack;
use super::{IP_NOADDR, atoi, format_ip, parse_ip, v4_of, v4prefix_of};

// ---------------------------------------------------------------------------
// The icmp stats counters (contract §7 — bridge-maintained per P-M;
// smoltcp exposes none; atomics so the read path needs no lock).
// ---------------------------------------------------------------------------

static IN_MSGS: AtomicU64 = AtomicU64::new(0);
static IN_ERRORS: AtomicU64 = AtomicU64::new(0);
static OUT_MSGS: AtomicU64 = AtomicU64::new(0);
static CSUM_ERRS: AtomicU64 = AtomicU64::new(0);
static LEN_ERRS: AtomicU64 = AtomicU64::new(0);
static HLEN_ERRS: AtomicU64 = AtomicU64::new(0);

/// Per-type in/out counters (icmp.c's Icmppriv in[Maxtype+1]/out[]).
static TYPE_IN: [AtomicU64; 19] = [const { AtomicU64::new(0) }; 19];
static TYPE_OUT: [AtomicU64; 19] = [const { AtomicU64::new(0) }; 19];

/// The per-type names (icmp.c's icmpnames[]; holes print numerically).
const ICMP_NAMES: [Option<&[u8]>; 19] = [
    Some(b"EchoReply"),       // 0
    None,
    None,
    Some(b"Unreachable"),     // 3
    Some(b"SrcQuench"),       // 4
    Some(b"Redirect"),        // 5
    None,
    None,
    Some(b"EchoRequest"),     // 8
    None,
    None,
    Some(b"TimeExceed"),      // 11
    Some(b"InParmProblem"),   // 12
    Some(b"Timestamp"),       // 13
    Some(b"TimestampReply"),  // 14
    Some(b"InfoRequest"),     // 15
    Some(b"InfoReply"),       // 16
    Some(b"AddrMaskRequest"), // 17
    Some(b"AddrMaskReply"),   // 18
];

// ---------------------------------------------------------------------------
// The ctl dispatch (devip.c's ipwrite Qctl arm).
// ---------------------------------------------------------------------------

/// A ctl-file write (parsecmd's tokenize on " \t\r\n" after the 64-byte
/// cmdbuf bound — T-08-06).  Returns the byte count on success (the
/// donor returns n).
pub(crate) fn ctl_write(p: Proto, id: usize, buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    // The donor's 64-byte cmdbuf bound (devether's netifwrite shape —
    // the plan's threat-register mitigation for the untrusted parse).
    let text = &buf[..n.min(63)];
    let mut fields: Vec<&[u8]> = Vec::new();
    for w in text.split(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n') {
        if !w.is_empty() {
            fields.push(w);
        }
    }
    if fields.is_empty() {
        return Err("short control request");
    }
    let verb = fields[0];
    let args = &fields[1..];
    match verb {
        b"connect" => ctl_connect(p, id, args)?,
        b"announce" => ctl_announce(p, id, args)?,
        // bindctlmsg (devip.c:1047-1058): the PROTO's own bind when it
        // has one (ipifc's ipifcbind — the medium bind), else the
        // generic Fsstdbind (set local port only).
        b"bind" => {
            if p == Proto::Ipifc {
                super::ipifc::ctl(id, verb, args)?;
            } else {
                ctl_bind(p, id, args)?;
            }
        }
        b"ttl" => {
            // ttlctlmsg (devip.c:1069-1076): no arg → MAXTTL.
            let v = match args.first() {
                None => conv::MAX_TTL,
                Some(a) => atoi(a),
            };
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            t.table_mut(p)[id].ttl = v;
            // Apply to a live socket — NEVER hop_limit 0 (smoltcp
            // panics; the panic-boundary rule).  The conv records the
            // file-visible value regardless (the donor allows ttl 0;
            // the wire value keeps the last valid one — a recorded
            // mapping corner).
            if v > 0 && v <= 255
                && let Some(h) = t.table(p)[id].sock
            {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                if let Some(core) = g.core.as_mut() {
                    apply_hop_limit(core, p, h, v as u8);
                }
            }
        }
        b"tos" => {
            // tosctlmsg (devip.c:1061-1067): no arg → 0.  smoltcp has no
            // per-packet tos knob on the socket emit path — the value is
            // recorded on the conv (the file-visible truth); the wire
            // tos stays 0 (a recorded mapping corner; no gate reads it).
            let v = match args.first() {
                None => 0,
                Some(a) => atoi(a),
            };
            {
                let node = LockNode::new();
                let mut t = conv::TABLES.lock(&node);
                t.table_mut(p)[id].tos = v;
            }
        }
        b"ignoreadvice" => {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            t.table_mut(p)[id].ignoreadvice = true;
        }
        b"addmulti" => ctl_addmulti(p, id, args)?,
        b"remmulti" => ctl_remmulti(p, id, args)?,
        b"maxfragsize" => {
            // [man-vs-src] source-only verb (devip.c:1165-1169).
            let Some(a) = args.first() else { return Err("maxfragsize needs size") };
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            t.table_mut(p)[id].maxfragsize = atoi(a);
        }
        _ => {
            // The proto fallthrough; a reject is a note-worthy parse
            // event (D-07) — netlog under the proto's flag.
            let r = proto_ctl(p, id, verb, args);
            if r.is_err() {
                let flag = match p {
                    Proto::Tcp => super::netlog::LOG_TCP,
                    Proto::Udp => super::netlog::LOG_UDP,
                    Proto::Icmp => super::netlog::LOG_ICMP,
                    _ => super::netlog::LOG_IP,
                };
                super::netlog::emit(
                    flag,
                    alloc::format!(
                        "devip: ctl parse reject on {} conv {}: {}",
                        alloc::string::String::from_utf8_lossy(p.name()),
                        id,
                        alloc::string::String::from_utf8_lossy(verb)
                    )
                    .into_bytes(),
                );
            }
            r?;
        }
    }
    // D-05 leg 3: every mutating ctl write ends with a drive (the
    // loopback's synchronous TX loop lands replies in the same file-op
    // context).
    stack::drive_stack();
    Ok(n as u64)
}

/// The proto's own ctl verbs (the `x->ctl` fallthrough).  icmp has NONE
/// (`icmp->ctl = nil`); ipifc's live in ipifc.rs; udp's `headers` is
/// udp_ctl; tcp's set (close/hangup/hangupxmit/keepalive/checksum/
/// tcpporthogdefense — contract §5.1) is tcp_ctl.
fn proto_ctl(p: Proto, id: usize, verb: &[u8], args: &[&[u8]]) -> Result<()> {
    match p {
        Proto::Ipifc => super::ipifc::ctl(id, verb, args),
        Proto::Udp => udp_ctl(id, verb, args),
        Proto::Tcp => tcp_ctl(id, verb, args),
        _ => Err("unknown control request"),
    }
}

// ---------------------------------------------------------------------------
// The generic verbs (contract §4).
// ---------------------------------------------------------------------------

/// `connect <ip>!<port>[!r] [<local>]` (connectctlmsg + Fsstdconnect,
/// devip.c:951-977 + 908-942).  For icmp the proto connect is
/// synchronous (no handshake — the donor's sleep returns immediately
/// via Fsconnected); the P-K blocking bridge (block on the waker until
/// Established/Closed/deadline) is tcp's shape in 08-05.
fn ctl_connect(p: Proto, id: usize, args: &[&[u8]]) -> Result<()> {
    if p == Proto::Ipifc {
        return Err("connect not supported");
    }
    match args.len() {
        1 | 2 => {}
        _ => return Err("bad args to connect"),
    }
    {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        let c = &mut t.table_mut(p)[id];
        if c.state != ConvState::Idle {
            return Err("connection in use");
        }
        // setraddrport (devip.c:885-903): addr!port[!r].
        let (raddr, rport, restricted) = parse_raddrport(args[0])?;
        c.raddr = raddr;
        c.rport = rport;
        if restricted {
            c.restricted = true;
        }
        c.state = ConvState::Connecting;
        match args.len() {
            1 => {
                // setladdr: the local address closest to the remote
                // (address-class demux — contract §12.4), then setlport.
                c.laddr = super::ipifc::find_local_ip(&c.raddr);
                conv::setlport(&mut t, p, id)?;
            }
            2 => {
                let (ra, rp) = {
                    let c2 = &t.table(p)[id];
                    (c2.raddr, c2.rport)
                };
                set_laddrport(&mut t, p, id, args[1], false, ra, rp)?;
            }
            _ => unreachable!(),
        }
    }
    // The proto connect (icmp: create + bind the socket, Connected now).
    connect_proto(p, id)?;
    {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        t.table_mut(p)[id].state = ConvState::Connected;
    }
    // A reader that blocked before the socket existed re-checks now.
    conv::wake_conv(p, id);
    Ok(())
}

/// `announce <X>` (announcectlmsg + Fsstdannounce, devip.c:1004-1030):
/// X is `[addr!]port` — a decimal port, `*` (the wildcard announce), or
/// the `addr!port` form libc's announce() actually writes (`announce
/// *!7` for tcpecho).  Fsstdannounce routes through setladdrport's
/// ANNOUNCING form — the hand-rolled atoi-only parse this replaced could
/// not express `*!7` (Rule 1: the tcp/7 gate's literal-port announce).
fn ctl_announce(p: Proto, id: usize, args: &[&[u8]]) -> Result<()> {
    if p == Proto::Ipifc {
        return Err("announce not supported");
    }
    if args.len() != 1 {
        return Err("bad args to announce");
    }
    {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        {
            let c = &mut t.table_mut(p)[id];
            if c.state != ConvState::Idle {
                return Err("connection in use");
            }
            // Fsstdannounce: raddr/rport cleared; laddr/lport are
            // set_laddrport's to write.
            c.raddr = IP_NOADDR;
            c.rport = 0;
            c.state = ConvState::Announcing;
        }
        set_laddrport(&mut t, p, id, args[0], true, IP_NOADDR, 0)?;
    }
    announce_proto(p, id)?;
    {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        t.table_mut(p)[id].state = ConvState::Announced;
    }
    conv::wake_conv(p, id);
    Ok(())
}

/// `bind <X>` (bindctlmsg → Fsstdbind, devip.c:1036-1058): set the
/// local port only (APE sockets emulation; unused natively).
fn ctl_bind(p: Proto, id: usize, args: &[&[u8]]) -> Result<()> {
    if args.len() != 1 {
        return Err("bad args to bind");
    }
    let node = LockNode::new();
    let mut t = conv::TABLES.lock(&node);
    let (raddr, rport) = {
        let c = &t.table(p)[id];
        (c.raddr, c.rport)
    };
    set_laddrport(&mut t, p, id, args[0], false, raddr, rport)
}

/// `addmulti <ifc-ip> [mcast-ip]` (devip.c:1140-1156): validate and
/// record the membership on the conv (the donor's c->multi list).  The
/// loopback era has no multicast delivery machinery — the record is the
/// honest file-level state (delivery arrives with the ether leg).
fn ctl_addmulti(p: Proto, id: usize, args: &[&[u8]]) -> Result<()> {
    let node = LockNode::new();
    let mut t = conv::TABLES.lock(&node);
    let c = &mut t.table_mut(p)[id];
    match args.len() {
        1 => {
            // 2 fields total: raddr must be multicast, f[1] is the ifc.
            if !is_multicast(&c.raddr) {
                return Err("addmulti for a non multicast address");
            }
            let ia = parse_ip(args[0]).ok_or("bad ip address syntax")?;
            c.mcast.push((c.raddr, ia));
            Ok(())
        }
        2 => {
            let ma = parse_ip(args[0]).ok_or("bad ip address syntax")?;
            let ia = parse_ip(args[1]).ok_or("bad ip address syntax")?;
            if !is_multicast(&ma) {
                return Err("addmulti for a non multicast address");
            }
            c.mcast.push((ma, ia));
            Ok(())
        }
        _ => Err("addmulti needs interface address"),
    }
}

/// `remmulti <ip>` (devip.c:1157-1164).
fn ctl_remmulti(p: Proto, id: usize, args: &[&[u8]]) -> Result<()> {
    if args.is_empty() {
        return Err("remmulti needs interface address");
    }
    let node = LockNode::new();
    let mut t = conv::TABLES.lock(&node);
    let c = &mut t.table_mut(p)[id];
    if !is_multicast(&c.raddr) {
        return Err("remmulti for a non multicast address");
    }
    let ia = parse_ip(args[0]).ok_or("bad ip address syntax")?;
    c.mcast.retain(|e| e.1 != ia);
    Ok(())
}

/// setladdrport (devip.c:831-883): `[address!]port[!r]` — the `!r`
/// suffix is meaningless on local ports (ignored by the source).
fn set_laddrport(
    t: &mut conv::ConvTables,
    p: Proto,
    id: usize,
    s: &[u8],
    announcing: bool,
    _raddr: [u8; 16],
    _rport: u16,
) -> Result<()> {
    // Split on the FIRST '!'; a trailing "r" flag is ignored (the
    // donor's `strcmp(p, "r") == 0 → p = nil`).
    let (head, port_str) = match s.iter().position(|&b| b == b'!') {
        Some(i) => {
            let rest = &s[i + 1..];
            // A second '!' introduces the flag section.
            let (port, _flag) = match rest.iter().position(|&b| b == b'!') {
                Some(j) => (&rest[..j], Some(&rest[j + 1..])),
                None => (rest, None),
            };
            (&s[..i], port)
        }
        None => (&s[..0], s),
    };
    t.table_mut(p)[id].lport = 0;
    if s.iter().position(|&b| b == b'!').is_none() {
        // Bare port: laddr = IPnoaddr when announcing, else the local
        // address for the conv's current raddr (setladdr's shape).
        if announcing {
            t.table_mut(p)[id].laddr = IP_NOADDR;
        } else {
            let raddr = t.table(p)[id].raddr;
            t.table_mut(p)[id].laddr = super::ipifc::find_local_ip(&raddr);
        }
    } else if head == b"*" {
        t.table_mut(p)[id].laddr = IP_NOADDR;
    } else {
        let addr = parse_ip(head).ok_or("bad ip address syntax")?;
        if !super::ipifc::ip_for_me(&addr) {
            return Err("not a local IP address");
        }
        t.table_mut(p)[id].laddr = addr;
    }
    // One process can get all connections (the wildcard announce).
    if announcing && port_str == b"*" {
        return conv::setluniqueport(t, p, id, 0);
    }
    // atoi semantics (donor): a non-numeric port reads as 0 (a random
    // port via setlport upstream).
    let port = atoi(port_str) as u16;
    if port == 0 {
        conv::setlport(t, p, id)
    } else {
        conv::setluniqueport(t, p, id, port)
    }
}

/// setraddrport (devip.c:885-903): `addr!port[!r]` — no '!' →
/// "malformed address"; bad address → Ebadip.
fn parse_raddrport(s: &[u8]) -> Result<([u8; 16], u16, bool)> {
    let Some(i) = s.iter().position(|&b| b == b'!') else {
        return Err("malformed address");
    };
    let addr = parse_ip(&s[..i]).ok_or("bad ip address syntax")?;
    let rest = &s[i + 1..];
    let (port_s, flag_s) = match rest.iter().position(|&b| b == b'!') {
        Some(j) => (&rest[..j], Some(&rest[j + 1..])),
        None => (rest, None),
    };
    let port = atoi(port_s) as u16;
    let restricted = flag_s.is_some_and(|f| f.contains(&b'r'));
    Ok((addr, port, restricted))
}

/// Is the address multicast (v4 224/4 or v6 ff00/8)?
fn is_multicast(a: &[u8; 16]) -> bool {
    if let Some(b) = v4_of(a) {
        return (b[0] & 0xf0) == 0xe0;
    }
    a[0] == 0xff
}

// ---------------------------------------------------------------------------
// icmp: socket creation (connect/announce), data read/write, status, stats.
// ---------------------------------------------------------------------------

/// The icmp socket buffer sizing: the donor's icmp conv rq is a 64 KiB
/// Qmsg queue (icmpcreate, icmp.c:110-114); 8 packets × 16 KiB (the
/// loopback maxtu) matches it.  Allocated AT connect/announce (only
/// active convs cost — the era's arena discipline).
const ICMP_RX_META: usize = 8;
const ICMP_BUF: usize = 64 * 1024;

/// The per-proto connect: create the proto's smoltcp socket bound to
/// the conv's local endpoint with hop limit = conv.ttl (255 default —
/// §12.8: smoltcp's 64 default must never leak).  tcp's connect is the
/// P-K BLOCKING bridge (contract §4): the socket connects, then the write
/// blocks on the waker until Established → Connected / Closed-mid-SYN →
/// Econrefused / the bridge deadline → Etimedout.
fn connect_proto(p: Proto, id: usize) -> Result<()> {
    match p {
        Proto::Icmp => {
            let (lport, ttl) = {
                let node = LockNode::new();
                let t = conv::TABLES.lock(&node);
                let c = &t.table(p)[id];
                (c.lport, c.ttl)
            };
            let mut s = icmp::Socket::new(
                PacketBuffer::new(alloc::vec![PacketMetadata::EMPTY; ICMP_RX_META], alloc::vec![0; ICMP_BUF]),
                PacketBuffer::new(alloc::vec![PacketMetadata::EMPTY; ICMP_RX_META], alloc::vec![0; ICMP_BUF]),
            );
            s.bind(icmp::Endpoint::Ident(lport)).map_err(|_| "connection in use")?;
            if ttl > 0 && ttl <= 255 {
                s.set_hop_limit(Some(ttl as u8));
            }
            let h = {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                let core = g.core.as_mut().ok_or("devip: no bound medium")?;
                core.sockets.add(s)
            };
            {
                let node = LockNode::new();
                let mut t = conv::TABLES.lock(&node);
                t.table_mut(p)[id].sock = Some(h);
            }
            Ok(())
        }
        Proto::Udp => {
            let (laddr, lport, ttl) = {
                let node = LockNode::new();
                let t = conv::TABLES.lock(&node);
                let c = &t.table(p)[id];
                (c.laddr, c.lport, c.ttl)
            };
            let s = udp_socket(laddr, lport, ttl)?;
            let h = {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                let core = g.core.as_mut().ok_or("devip: no bound medium")?;
                core.sockets.add(s)
            };
            {
                let node = LockNode::new();
                let mut t = conv::TABLES.lock(&node);
                t.table_mut(p)[id].sock = Some(h);
            }
            Ok(())
        }
        Proto::Tcp => {
            let (laddr, lport, raddr, rport, ttl) = {
                let node = LockNode::new();
                let t = conv::TABLES.lock(&node);
                let c = &t.table(p)[id];
                (c.laddr, c.lport, c.raddr, c.rport, c.ttl)
            };
            // The v4 leg (the donor's V4 arm); v6 destinations land with
            // 08-06's ether leg (udp's recorded corner shape).
            let Some(r4) = v4_of(&raddr) else {
                return Err("bad ip address syntax");
            };
            let mut s = tcp_socket(ttl);
            {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                let core = g.core.as_mut().ok_or("devip: no bound medium")?;
                // IpListenEndpoint.addr = None → smoltcp's source pick
                // (get_source_address); the conv's recorded laddr when set
                // (the guest gate's 127.0.0.1).  NEVER an unspecified
                // Some (smoltcp errors Unaddressable on it).
                let local = IpListenEndpoint {
                    addr: v4_of(&laddr).map(|a| IpAddress::Ipv4(Ipv4Address::from(a))),
                    port: lport,
                };
                s.connect(
                    core.iface.context(),
                    IpEndpoint { addr: IpAddress::Ipv4(Ipv4Address::from(r4)), port: rport },
                    local,
                )
                .map_err(|e| match e {
                    tcp::ConnectError::InvalidState => "connection in use",
                    // Remote port 0 / unspecified remote / no source
                    // address: the donor's observable outcome for those
                    // inputs is a SYN that never resolves → Etimedout.
                    tcp::ConnectError::Unaddressable => "connection timed out",
                })?;
                let h = core.sockets.add(s);
                drop(g);
                let node = LockNode::new();
                let mut t = conv::TABLES.lock(&node);
                t.table_mut(p)[id].sock = Some(h);
            }
            TCP_STATS[TCPSTAT_ACTIVE_OPENS].fetch_add(1, Ordering::Relaxed);
            // P-K: block until the handshake resolves (tcp.c's
            // connectctlmsg sleep on c->cr until Connected/failed).
            tcp_connect_wait(id)
        }
        _ => Err("connect not supported"),
    }
}

/// The per-proto announce.  tcp's announce builds the ACCEPT POOL
/// (contract §12.3: 128 listening sockets — smoltcp has no listen
/// backlog, so devip's Maxincall maps onto the pool).
fn announce_proto(p: Proto, id: usize) -> Result<()> {
    match p {
        Proto::Icmp => connect_proto(p, id), // icmp announce = the same socket bind (icmp.c's Fsstdannounce + Fsconnected)
        Proto::Udp => connect_proto(p, id),  // udp announce = the same socket bind (udp.c's Fsstdannounce shape)
        Proto::Tcp => {
            let lport = {
                let node = LockNode::new();
                let t = conv::TABLES.lock(&node);
                t.table(p)[id].lport
            };
            let pool = stack::AcceptPool::announce(stack::ACCEPT_POOL_DEPTH, lport)?;
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            t.table_mut(p)[id].pool = Some(pool);
            Ok(())
        }
        _ => Err("announce not supported"),
    }
}

/// Hop-limit application helper (the ttl ctl on a live socket).
fn apply_hop_limit(core: &mut stack::StackCore, p: Proto, h: smoltcp::iface::SocketHandle, v: u8) {
    match p {
        Proto::Icmp => {
            core.sockets.get_mut::<icmp::Socket>(h).set_hop_limit(Some(v));
        }
        Proto::Udp => {
            core.sockets.get_mut::<udp::Socket>(h).set_hop_limit(Some(v));
        }
        Proto::Tcp => {
            core.sockets.get_mut::<tcp::Socket>(h).set_hop_limit(Some(v));
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// The icmp data path
// ---------------------------------------------------------------------------

/// The data-file write (icmpkick, icmp.c:164-191): the user's buffer is
/// the full IPv4-packet-shaped block — a 20-byte header area the kernel
/// overwrites, then the ICMP message.  We extract the message, overwrite
/// its ident with the conv's lport, zero the checksum field (smoltcp
/// recomputes at emit), and send to the conv's recorded raddr.
///
/// The donor silently DROPS a short write (`blocklen < ICMP_IPSIZE +
/// ICMP_HDRSIZE` → freeblist, return) — the write itself succeeds.  We
/// replicate that (counted via HlenErrs, never silent).
pub(crate) fn icmp_write_data(id: usize, buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    if n < 20 + 8 {
        HLEN_ERRS.fetch_add(1, Ordering::Relaxed);
        return Ok(n as u64);
    }
    let (sock, raddr, lport) = {
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        let c = &t.table(Proto::Icmp)[id];
        (c.sock, c.raddr, c.lport)
    };
    let Some(h) = sock else {
        // Never-connected conv: the donor's qwrite lands in the queue
        // and ipoput4 drops the unroutable packet — fire-and-forget.
        // Counted, never an error (the donor's write returns n).
        LEN_ERRS.fetch_add(1, Ordering::Relaxed);
        return Ok(n as u64);
    };
    let Some(dst) = v4_of(&raddr).map(Ipv4Address::from) else {
        // v6 destinations ride icmpv6, not this file (08-06).
        LEN_ERRS.fetch_add(1, Ordering::Relaxed);
        return Ok(n as u64);
    };
    let mut msg = alloc::vec![0u8; n - 20];
    msg.copy_from_slice(&buf[20..]);
    // icmpkick: hnputs(p->icmpid, c->lport) — the kernel OVERWRITES the
    // packet's ident; memset cksum (smoltcp recomputes at emit).
    msg[4] = (lport >> 8) as u8;
    msg[5] = (lport & 0xff) as u8;
    msg[2] = 0;
    msg[3] = 0;
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        let send_res = {
            let mut g = stack::STACK.enter().expect("devip: stack contended");
            let Some(core) = g.core.as_mut() else { return Err("devip: no bound medium") };
            let s = core.sockets.get_mut::<icmp::Socket>(h);
            s.send_slice(&msg, IpAddress::Ipv4(dst))
        };
        match send_res {
            Ok(()) => break,
            Err(_) => {
                // TX buffer full: block until the drive frees space
                // (register → drop → block — wakeup-loss-free per the
                // seam's IRQ-masked-syscall invariant).
                if hooks.is_none() {
                    hooks = Some(stack::hooks()?);
                }
                let hks = hooks.unwrap();
                let pid = (hks.current)();
                {
                    let node = LockNode::new();
                    let mut t = conv::TABLES.lock(&node);
                    t.table_mut(Proto::Icmp)[id].waiters.register(pid);
                }
                stack::register_net_waiter(pid);
                {
                    let mut g = stack::STACK.enter().expect("devip: stack contended");
                    if let Some(core) = g.core.as_mut() {
                        super::ICMP_TX_SLOTS[id].arm(pid);
                        core.sockets
                            .get_mut::<icmp::Socket>(h)
                            .register_send_waker(&crate::devip_waker::waker_for(&super::ICMP_TX_SLOTS[id]));
                    }
                }
                stack::drive_stack();
                (hks.block)();
            }
        }
    }
    OUT_MSGS.fetch_add(1, Ordering::Relaxed);
    let ty = msg[0];
    if (ty as usize) < 19 {
        TYPE_OUT[ty as usize].fetch_add(1, Ordering::Relaxed);
    }
    // D-05 leg 3: the mutating write ends with a drive — the loopback
    // loops the packet synchronously and the reply lands now.
    stack::drive_stack();
    Ok(n as u64)
}

/// The data-file read (the donor's `qread(c->rq)` — Qmsg
/// datagram-at-once: a short buffer DISCARDS the excess, never splits a
/// datagram).  The delivered block is the full IPv4-packet shape: the
/// 20-byte header area (synthesized — smoltcp's icmp socket is
/// payload-level) + the ICMP message.  Blocks per the seam's
/// register → drop → block shape; a wake is always followed by a
/// `can_recv` re-check (spurious-wake discipline, P-C).
pub(crate) fn icmp_read_data(id: usize, buf: &mut [u8]) -> Result<u64> {
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            let c = &mut t.table_mut(Proto::Icmp)[id];
            if let Some(h) = c.sock {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                if let Some(core) = g.core.as_mut() {
                    let s = core.sockets.get_mut::<icmp::Socket>(h);
                    if s.can_recv() {
                        let Ok((pkt, remote)) = s.recv() else {
                            drop(g);
                            continue;
                        };
                        let n = synthesize_v4_packet(pkt, &remote, c.laddr, c.ttl, buf);
                        IN_MSGS.fetch_add(1, Ordering::Relaxed);
                        let ty = pkt.first().copied().unwrap_or(0) as usize;
                        if ty < 19 {
                            TYPE_IN[ty].fetch_add(1, Ordering::Relaxed);
                        }
                        return Ok(n as u64);
                    }
                }
            }
        }
        // Flush pending loopback work before blocking (the read-side of
        // D-05 leg 3 — a read IS local activity; the drive is idle-cheap
        // when nothing is pending).
        stack::drive_stack();
        {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            let c = &mut t.table_mut(Proto::Icmp)[id];
            let has_data = if let Some(h) = c.sock {
                let g = stack::STACK.enter().expect("devip: stack contended");
                match g.core.as_ref() {
                    Some(core) => core.sockets.get::<icmp::Socket>(h).can_recv(),
                    None => false,
                }
            } else {
                false
            };
            if has_data {
                continue;
            }
            // Register, drop, block (devpipe's wakeup-loss-free shape).
            if hooks.is_none() {
                hooks = Some(stack::hooks()?);
            }
            let hks = hooks.unwrap();
            let pid = (hks.current)();
            c.waiters.register(pid);
            stack::register_net_waiter(pid);
            if let Some(h) = c.sock {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                if let Some(core) = g.core.as_mut() {
                    super::ICMP_RX_SLOTS[id].arm(pid);
                    core.sockets
                        .get_mut::<icmp::Socket>(h)
                        .register_recv_waker(&crate::devip_waker::waker_for(&super::ICMP_RX_SLOTS[id]));
                }
            }
        }
        (hooks.unwrap().block)();
    }
}

/// Synthesize the 20-byte IPv4 header area + the ICMP message into the
/// reader's buffer (Qmsg truncation: a short buffer discards the
/// excess).  The ttl field carries the conv's configured ttl — smoltcp
/// exposes no received-hop-limit on the icmp recv metadata, a recorded
/// mapping corner (ping prints it cosmetically).
fn synthesize_v4_packet(pkt: &[u8], remote: &IpAddress, laddr: [u8; 16], ttl: u32, buf: &mut [u8]) -> usize {
    let total = 20 + pkt.len();
    let n = total.min(buf.len());
    if n == 0 {
        return 0;
    }
    let mut hdr = [0u8; 20];
    hdr[0] = 0x45; // vihl: v4, 20-byte header
    hdr[1] = 0; // tos
    let tl = (total as u16).to_be_bytes();
    hdr[2] = tl[0];
    hdr[3] = tl[1];
    // id/frag: 0
    hdr[8] = ttl.min(255) as u8;
    hdr[9] = 1; // proto: ICMP
    // header cksum: 0 (nothing in the guest validates the synthesized
    // area's checksum — ping reads ttl only).
    if let IpAddress::Ipv4(r) = remote {
        hdr[12..16].copy_from_slice(&r.octets());
    }
    if let Some(l) = v4_of(&laddr) {
        hdr[16..20].copy_from_slice(&l);
    }
    let hcpy = n.min(20);
    buf[..hcpy].copy_from_slice(&hdr[..hcpy]);
    if n > 20 {
        let rest = n - 20;
        buf[20..n].copy_from_slice(&pkt[..rest]);
    }
    n
}

// ---------------------------------------------------------------------------
// Per-file content: status / stats / local / remote.
// ---------------------------------------------------------------------------

/// The conv's status line (icmpstate, icmp.c:128-137):
/// `"Datagram qin %d qout %d\n"` — the queue byte counts read live from
/// the socket (0 for a socket-less conv).  tcp's line is the contract's
/// §5.2 format (tcpstate, tcp.c:493-509) — tcp_status builds it.
pub(crate) fn status(p: Proto, id: usize) -> Vec<u8> {
    if p == Proto::Tcp {
        return tcp_status(id);
    }
    let (qin, qout) = {
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        match t.table(p)[id].sock {
            Some(h) => {
                let g = stack::STACK.enter().expect("devip: stack contended");
                match g.core.as_ref() {
                    Some(core) => match p {
                        Proto::Icmp => {
                            let s = core.sockets.get::<icmp::Socket>(h);
                            (s.recv_queue(), s.send_queue())
                        }
                        Proto::Udp => {
                            let s = core.sockets.get::<udp::Socket>(h);
                            (s.recv_queue(), s.send_queue())
                        }
                        _ => (0, 0), // tcp/icmpv6 land in 08-05/08-06
                    },
                    None => (0, 0),
                }
            }
            None => {
                // A socket-less udp conv (a spawned incoming call) reports
                // its devip-side queue depth.
                if p == Proto::Udp {
                    let c = &t.table(p)[id];
                    (c.dataq.iter().map(|m| m.len()).sum(), 0)
                } else {
                    (0, 0)
                }
            }
        }
    };
    let head: &[u8] = match p {
        Proto::Icmp | Proto::Icmp6 => b"Datagram",
        Proto::Udp => {
            let node = LockNode::new();
            let t = conv::TABLES.lock(&node);
            let open = t.table(p)[id].sock.is_some();
            if open { b"Open" } else { b"Closed" }
        }
        _ => b"Datagram",
    };
    let mut out = Vec::new();
    out.extend_from_slice(head);
    out.extend_from_slice(b" qin ");
    out.extend_from_slice(&dec(qin as u64));
    out.extend_from_slice(b" qout ");
    out.extend_from_slice(&dec(qout as u64));
    out.push(b'\n');
    out
}

/// The `%I!%d\n` local/remote reads (devip.c:648-671's default arm).
pub(crate) fn local_remote(p: Proto, id: usize, local: bool) -> Vec<u8> {
    let node = LockNode::new();
    let t = conv::TABLES.lock(&node);
    let c = &t.table(p)[id];
    let (addr, port) = if local { (c.laddr, c.lport) } else { (c.raddr, c.rport) };
    let mut out = format_ip(&addr);
    out.push(b'!');
    out.extend_from_slice(&dec(u64::from(port)));
    out.push(b'\n');
    out
}

/// The icmp stats file (icmpstats, icmp.c:450-469): the six tagged
/// lines, then the per-type in/out lines (named where icmpnames has an
/// entry, numeric otherwise).
pub(crate) fn icmp_stats() -> Vec<u8> {
    let mut out = Vec::new();
    let mut line = |key: &[u8], val: u64| {
        out.extend_from_slice(key);
        out.extend_from_slice(b": ");
        out.extend_from_slice(&dec(val));
        out.push(b'\n');
    };
    line(b"InMsgs", IN_MSGS.load(Ordering::Relaxed));
    line(b"InErrors", IN_ERRORS.load(Ordering::Relaxed));
    line(b"OutMsgs", OUT_MSGS.load(Ordering::Relaxed));
    line(b"CsumErrs", CSUM_ERRS.load(Ordering::Relaxed));
    line(b"LenErrs", LEN_ERRS.load(Ordering::Relaxed));
    line(b"HlenErrs", HLEN_ERRS.load(Ordering::Relaxed));
    for (i, name) in ICMP_NAMES.iter().enumerate() {
        let i64v = |c: &AtomicU64| c.load(Ordering::Relaxed);
        match name {
            Some(n) => {
                out.extend_from_slice(n);
            }
            None => {
                out.extend_from_slice(&dec(i as u64));
            }
        }
        out.extend_from_slice(b": ");
        out.extend_from_slice(&dec(i64v(&TYPE_IN[i])));
        out.push(b' ');
        out.extend_from_slice(&dec(i64v(&TYPE_OUT[i])));
        out.push(b'\n');
    }
    out
}

/// Decimal ASCII, no padding (the devproc dec idiom).
pub(crate) fn dec(mut v: u64) -> Vec<u8> {
    let mut out = Vec::new();
    if v == 0 {
        out.push(b'0');
    }
    while v > 0 {
        out.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    out
}

// ---------------------------------------------------------------------------
// udp (contract §6): the file-exact udp module — headers mode (the
// 52-byte Udphdr prefix), the Fsnewcall spawn/demux machine, Qmsg
// datagram-at-once reads, the advise hangup, and the four-field stats.
//
// THE RX MODEL: an announced conv's smoltcp socket receives every
// datagram for its port (smoltcp demuxes by dst port/addr, first match —
// it has no per-REMOTE demux).  Headers-mode convs read the socket
// directly, prefix-synthesizing at delivery.  Non-headers announced
// convs never carry data themselves (the donor shape): udp_rx_drain
// moves their datagrams into per-conv queues — an existing spawned conv
// of the same quad, else a fresh Fsnewcall spawn.  Spawned convs read
// their own dataq (draining the announcer's socket first).  Connected
// (client) convs read their own socket directly — a reply to the
// client's unique local port never touches the announced socket.
// ---------------------------------------------------------------------------

/// The udp socket buffer sizing: the donor's udp conv rq is a 64 KiB
/// Qmsg queue (udpcreate, udp.c); 8 datagrams × 16 KiB (the loopback
/// maxtu) matches it.  Allocated AT connect/announce (only active convs
/// cost — the era's arena discipline).
const UDP_RX_META: usize = 8;
const UDP_BUF: usize = 64 * 1024;

/// The udp stats counters (contract §6.3's four tagged lines —
/// bridge-maintained per P-M; smoltcp exposes none).  NoPorts/InErrors
/// are structurally still this era: smoltcp's IP demux answers
/// no-socket datagrams with its own ICMP port-unreachable (never
/// surfaced to us) and drops malformed frames below the socket layer —
/// the counters exist and move where the bridge can observe the event
/// (InErrors counts dropped-at-the-boundary writes/reads).
static UDP_IN: AtomicU64 = AtomicU64::new(0);
static UDP_NO_PORTS: AtomicU64 = AtomicU64::new(0);
static UDP_IN_ERRS: AtomicU64 = AtomicU64::new(0);
static UDP_OUT: AtomicU64 = AtomicU64::new(0);

/// Create the conv's smoltcp udp socket bound to (laddr, lport) with
/// hop limit = conv.ttl (255 default — §12.8: smoltcp's 64 default must
/// never leak).  Quad-exact RX where the local address is known (the
/// donor's conv matching); IP_NOADDR binds any (the announce shape).
fn udp_socket(laddr: [u8; 16], lport: u16, ttl: u32) -> Result<udp::Socket<'static>> {
    let mut s = udp::Socket::new(
        PacketBuffer::new(alloc::vec![PacketMetadata::EMPTY; UDP_RX_META], alloc::vec![0; UDP_BUF]),
        PacketBuffer::new(alloc::vec![PacketMetadata::EMPTY; UDP_RX_META], alloc::vec![0; UDP_BUF]),
    );
    let bind_addr = v4_of(&laddr).map(|a| IpAddress::Ipv4(Ipv4Address::from(a)));
    s.bind(IpListenEndpoint { addr: bind_addr, port: lport })
        .map_err(|_| "connection in use")?;
    if ttl > 0 && ttl <= 255 {
        s.set_hop_limit(Some(ttl as u8));
    }
    Ok(s)
}

/// udp's own ctl verb (udpctl, udp.c:513-526): `headers` sets the
/// 52-byte prefix mode (headers = 7, "new headers format").  Anything
/// else is the generic dispatch's unknown-verb error.
fn udp_ctl(id: usize, verb: &[u8], args: &[&[u8]]) -> Result<()> {
    match verb {
        b"headers" if args.is_empty() => {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            t.table_mut(Proto::Udp)[id].headers = 7;
            Ok(())
        }
        _ => Err("unknown control request"),
    }
}

/// One probe of the conv's readable state: the spawn-drain first, then
/// the queue leg (spawned convs), then the direct-socket leg (connected
/// convs + headers-mode announced convs).  Ok(Some(n)) = delivered
/// (Qmsg datagram-at-once: a short buffer DISCARDS the excess);
/// Ok(None) = nothing readable; Err = the advise hangup.
fn udp_try_recv(id: usize, buf: &mut [u8]) -> Result<Option<u64>> {
    // Move socket-staged datagrams into the conv queues first (the
    // Fsnewcall spawn/demux lives in the drain; a no-op when nothing is
    // pending).
    udp_rx_drain();
    let node = LockNode::new();
    let mut t = conv::TABLES.lock(&node);
    let (direct, sock, headers, laddr, lport) = {
        let c = &mut t.table_mut(Proto::Udp)[id];
        if c.hangup.is_some() {
            return Err("udp: hangup on advise");
        }
        // The spawned conv's queue leg: datagrams the drain moved here
        // (payload only — the spawn's quad is the conv's recorded
        // raddr/rport).
        if let Some(msg) = c.dataq.pop_front() {
            let n = msg.len().min(buf.len());
            buf[..n].copy_from_slice(&msg[..n]);
            return Ok(Some(n as u64));
        }
        (c.announcer.is_none(), c.sock, c.headers == 7, c.laddr, c.lport)
    };
    // The direct-socket leg: recv from the conv's own socket.
    if direct {
        if let Some(h) = sock {
            let mut g = stack::STACK.enter().expect("devip: stack contended");
            if let Some(core) = g.core.as_mut() {
                let s = core.sockets.get_mut::<udp::Socket>(h);
                if s.can_recv() {
                    if let Ok((pkt, meta)) = s.recv() {
                        let n = if headers {
                            synthesize_udphdr(pkt, &meta, laddr, lport, buf)
                        } else {
                            let n = pkt.len().min(buf.len());
                            buf[..n].copy_from_slice(&pkt[..n]);
                            n
                        };
                        UDP_IN.fetch_add(1, Ordering::Relaxed);
                        return Ok(Some(n as u64));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// The udp data read (the donor's `qread(c->rq)`): probe → drive →
/// re-probe → register/arm → block, re-checking on every wake
/// (spurious-wake discipline, P-C).  The pre-block DRIVE is the D-05
/// leg-3 read side (icmp_read_data's shape): a write's egress only LOOPS
/// the packet into the device; this drive's ingress moves it into the
/// destination socket and fires its waker — without it a loopback
/// exchange deadlocks with both ends blocked.
pub(crate) fn udp_read_data(id: usize, buf: &mut [u8]) -> Result<u64> {
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        if let Some(n) = udp_try_recv(id, buf)? {
            return Ok(n);
        }
        // Flush pending loopback work before blocking, then re-probe.
        stack::drive_stack();
        if let Some(n) = udp_try_recv(id, buf)? {
            return Ok(n);
        }
        // Nothing to read: register, drop, block (devpipe's
        // wakeup-loss-free shape).  The recv waker arms the socket that
        // FEEDS this conv — a spawned conv's datagrams arrive on its
        // announcer's socket.
        {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            if hooks.is_none() {
                hooks = Some(stack::hooks()?);
            }
            let hks = hooks.unwrap();
            let pid = (hks.current)();
            let (sock, announcer) = {
                let c = &mut t.table_mut(Proto::Udp)[id];
                c.waiters.register(pid);
                (c.sock, c.announcer)
            };
            stack::register_net_waiter(pid);
            let feed = match announcer {
                Some(aid) => t.table(Proto::Udp)[aid].sock,
                None => sock,
            };
            if let Some(h) = feed {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                if let Some(core) = g.core.as_mut() {
                    super::UDP_RX_SLOTS[id].arm(pid);
                    core.sockets
                        .get_mut::<udp::Socket>(h)
                        .register_recv_waker(&crate::devip_waker::waker_for(&super::UDP_RX_SLOTS[id]));
                }
            }
        }
        (hooks.unwrap().block)();
    }
}

/// Synthesize the 52-byte Udphdr prefix + the payload into the reader's
/// buffer (contract §6.2: raddr[16] laddr[16] ifcaddr[16] rport[2]
/// lport[2]; v4 mapped into v6 form per the v4prefix model; ports
/// big-endian).  Qmsg truncation: a short buffer discards the excess.
/// laddr is the conv's recorded local when set, else the packet's
/// receiving address (the donor reads the packet's dst); ifcaddr is the
/// receiving interface's local — the same address on this era's
/// one-address interfaces (a recorded corner for the multi-address
/// ether era).
fn synthesize_udphdr(
    pkt: &[u8],
    meta: &udp::UdpMetadata,
    laddr: [u8; 16],
    lport: u16,
    buf: &mut [u8],
) -> usize {
    let total = 52 + pkt.len();
    let n = total.min(buf.len());
    if n == 0 {
        return 0;
    }
    let mut hdr = [0u8; 52];
    // raddr/rport: the sender.
    if let IpAddress::Ipv4(r) = meta.endpoint.addr {
        hdr[0..16].copy_from_slice(&v4prefix_of(&r.octets()));
    }
    // laddr: the packet's receiving address when the conv's is
    // unspecified (announce cleared it); the conv's recorded local
    // otherwise.
    let receiving = match meta.local_address {
        Some(IpAddress::Ipv4(l)) => v4prefix_of(&l.octets()),
        _ => IP_NOADDR,
    };
    let local = if laddr == IP_NOADDR { receiving } else { laddr };
    hdr[16..32].copy_from_slice(&local);
    // ifcaddr: the receiving interface's local (this era's one-address
    // interfaces make it the receiving address).
    hdr[32..48].copy_from_slice(&receiving);
    hdr[48..50].copy_from_slice(&meta.endpoint.port.to_be_bytes());
    hdr[50..52].copy_from_slice(&lport.to_be_bytes());
    let hcpy = n.min(52);
    buf[..hcpy].copy_from_slice(&hdr[..hcpy]);
    if n > 52 {
        let rest = n - 52;
        buf[52..n].copy_from_slice(&pkt[..rest]);
    }
    n
}

/// The udp data write.  Connected mode: the conv's recorded quad.
/// Headers mode: the user's buffer carries the 52-byte Udphdr prefix —
/// raddr/rport from the prefix, the lport OVERRIDDEN by the conv's
/// announced port and a non-local user laddr overridden by the conv's
/// (the man/3/ip rule).  The OQ-1 bridge rule (§12.6): the send's
/// local_address comes from the conv's recorded laddr (never the naive
/// silent-drop shape); an unspecified laddr resolves via the interface
/// for the destination.
pub(crate) fn udp_write_data(id: usize, buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    // (The conv's lport never appears here: the socket's bind pinned the
    // source port at connect/announce — the man's "the kernel overrides
    // the user-specified local port with the announced one" is thus
    // enforced by construction on every write.)
    let (sock, raddr, rport, laddr, headers) = {
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        let c = &t.table(Proto::Udp)[id];
        if c.hangup.is_some() {
            return Err("udp: hangup on advise");
        }
        (c.sock, c.raddr, c.rport, c.laddr, c.headers == 7)
    };
    let Some(h) = sock else {
        // Never-connected conv: the donor's qwrite queues the packet and
        // ipoput4 drops it unroutable — fire-and-forget (the write
        // succeeds).  Counted, never silent.
        UDP_IN_ERRS.fetch_add(1, Ordering::Relaxed);
        return Ok(n as u64);
    };
    let (daddr16, dport, laddr16, payload): ([u8; 16], u16, [u8; 16], &[u8]) = if headers {
        if n < 52 {
            // A short headers-mode write can't carry a prefix — the
            // donor's short-write drop shape (the write succeeds).
            UDP_IN_ERRS.fetch_add(1, Ordering::Relaxed);
            return Ok(n as u64);
        }
        let mut ra = [0u8; 16];
        ra.copy_from_slice(&buf[0..16]);
        let mut la = [0u8; 16];
        la.copy_from_slice(&buf[16..32]);
        let rp = u16::from_be_bytes([buf[48], buf[49]]);
        // The man's override rule: the announced lport wins; a non-local
        // user laddr is replaced by the conv's.
        let la = if la == IP_NOADDR || !super::ipifc::ip_for_me(&la) { laddr } else { la };
        (ra, rp, la, &buf[52..])
    } else {
        (raddr, rport, laddr, buf)
    };
    let Some(dst4) = v4_of(&daddr16) else {
        // A v6 destination on the v4 udp conv — the donor's udpwrite4/
        // udpwrite6 split; the v6 leg lands with 08-06.
        return Err("bad ip address syntax");
    };
    let local = v4_of(&laddr16)
        .map(|a| IpAddress::Ipv4(Ipv4Address::from(a)))
        .or_else(|| {
            let r = super::ipifc::find_local_ip(&daddr16);
            v4_of(&r).map(|a| IpAddress::Ipv4(Ipv4Address::from(a)))
        });
    let meta = udp::UdpMetadata {
        endpoint: IpEndpoint { addr: IpAddress::Ipv4(Ipv4Address::from(dst4)), port: dport },
        local_address: local,
        meta: smoltcp::phy::PacketMeta::default(),
    };
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        let send_res = {
            let mut g = stack::STACK.enter().expect("devip: stack contended");
            let Some(core) = g.core.as_mut() else { return Err("devip: no bound medium") };
            let s = core.sockets.get_mut::<udp::Socket>(h);
            s.send_slice(payload, meta)
        };
        match send_res {
            Ok(()) => break,
            Err(_) => {
                // TX buffer full: block until the drive frees space
                // (register → drop → block — icmp_write_data's shape).
                if hooks.is_none() {
                    hooks = Some(stack::hooks()?);
                }
                let hks = hooks.unwrap();
                let pid = (hks.current)();
                {
                    let node = LockNode::new();
                    let mut t = conv::TABLES.lock(&node);
                    t.table_mut(Proto::Udp)[id].waiters.register(pid);
                }
                stack::register_net_waiter(pid);
                {
                    let mut g = stack::STACK.enter().expect("devip: stack contended");
                    if let Some(core) = g.core.as_mut() {
                        super::UDP_TX_SLOTS[id].arm(pid);
                        core.sockets
                            .get_mut::<udp::Socket>(h)
                            .register_send_waker(&crate::devip_waker::waker_for(&super::UDP_TX_SLOTS[id]));
                    }
                }
                stack::drive_stack();
                (hks.block)();
            }
        }
    }
    UDP_OUT.fetch_add(1, Ordering::Relaxed);
    // D-05 leg 3: the mutating write ends with a drive — the loopback
    // loops the datagram synchronously.
    stack::drive_stack();
    Ok(n as u64)
}

/// The udp RX drain: move datagrams staged in ANNOUNCED, non-headers
/// convs' sockets into the devip-side conv queues — delivered to the
/// existing spawned conv of the same remote quad, else to a fresh
/// Fsnewcall spawn (contract §6.2).  Headers-mode convs are skipped
/// (their reads recv from the socket directly); connected convs are
/// skipped (their socket is their own).  Runs at the top of every udp
/// read and the listen open's re-check loop.  Locking: TABLES → STACK
/// (the documented nesting); the wakes run AFTER the table guard drops
/// (netif's wake-after-drop discipline).  Never drives the stack (the
/// caller's file op drives per D-05 leg 3).
pub(crate) fn udp_rx_drain() {
    let mut wakes: Vec<usize> = Vec::new();
    {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        for id in 0..conv::NCONV {
            let (state, headers, sock, laddr, lport) = {
                let c = &t.table(Proto::Udp)[id];
                (c.state, c.headers, c.sock, c.laddr, c.lport)
            };
            if state != conv::ConvState::Announced || headers != 0 {
                continue;
            }
            let Some(h) = sock else { continue };
            loop {
                // Pop one staged datagram (copy it out — the socket
                // borrow ends with the cell guard).
                let next = {
                    let mut g = stack::STACK.enter().expect("devip: stack contended");
                    match g.core.as_mut() {
                        Some(core) => {
                            let s = core.sockets.get_mut::<udp::Socket>(h);
                            if !s.can_recv() {
                                None
                            } else {
                                match s.recv() {
                                    Ok((pkt, meta)) => Some((pkt.to_vec(), meta)),
                                    Err(_) => None,
                                }
                            }
                        }
                        None => None,
                    }
                };
                let Some((pkt, meta)) = next else { break };
                UDP_IN.fetch_add(1, Ordering::Relaxed);
                // The sender's quad (v4-mapped into the devip 16-byte
                // form; v6 senders land with 08-6's udp6 leg).
                let (raddr, rport) = match meta.endpoint.addr {
                    IpAddress::Ipv4(a) => (v4prefix_of(&a.octets()), meta.endpoint.port),
                    _ => continue,
                };
                let local16 = match meta.local_address {
                    Some(IpAddress::Ipv4(a)) => v4prefix_of(&a.octets()),
                    _ => laddr,
                };
                // Per-quad demux: an existing spawned conv of this
                // announcer with this quad?
                let existing = t
                    .table(Proto::Udp)
                    .iter()
                    .enumerate()
                    .find(|(_, c)| {
                        c.inuse > 0
                            && c.announcer == Some(id)
                            && c.raddr == raddr
                            && c.rport == rport
                    })
                    .map(|(i, _)| i);
                let nid = match existing {
                    Some(nid) => nid,
                    None => {
                        let Some(nid) = conv::fs_new_call(
                            &mut t, Proto::Udp, id, raddr, rport, local16, lport,
                        ) else {
                            // Table full: the donor drops the datagram on
                            // a full incall (its netlog note is 08-05's).
                            break;
                        };
                        wakes.push(id); // the announcer's listenr wakes
                        nid
                    }
                };
                t.table_mut(Proto::Udp)[nid].dataq.push_back(pkt);
                wakes.push(nid); // the spawned conv's readers wake
            }
        }
    }
    for nid in wakes {
        conv::wake_conv(Proto::Udp, nid);
    }
}

/// udpadvise (udp.c:530-580): an ICMP error associated with the conv
/// qhangups it with the message — the err file carries it, and data
/// reads/writes fail while the hangup stands (sticky until close) —
/// unless ignoreadvice.  The conv-level wiring lands here; the smoltcp
/// icmp-error ingress (Endpoint::Udp) stays unwired per contract §12.7
/// (listed-but-ungated this phase).
#[allow(dead_code)] // exercised by the host pin today; the icmp-ingress caller is §12.7's recorded corner
pub(crate) fn udp_advise(id: usize, msg: &[u8]) {
    {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        let c = &mut t.table_mut(Proto::Udp)[id];
        if c.ignoreadvice {
            return;
        }
        let m = msg.to_vec();
        c.errq.push_back(m.clone());
        c.hangup = Some(m);
    }
    // The wake after the table guard drops (netif's discipline): blocked
    // readers/writers re-check and take the hangup error.
    conv::wake_conv(Proto::Udp, id);
}

/// The udp stats file (udpstats, udp.c:585-596): the four tagged lines
/// (source tag names govern — the man's prose names the same four).
pub(crate) fn udp_stats() -> Vec<u8> {
    let mut out = Vec::new();
    let mut line = |key: &[u8], val: u64| {
        out.extend_from_slice(key);
        out.extend_from_slice(b": ");
        out.extend_from_slice(&dec(val));
        out.push(b'\n');
    };
    line(b"InDatagrams", UDP_IN.load(Ordering::Relaxed));
    line(b"NoPorts", UDP_NO_PORTS.load(Ordering::Relaxed));
    line(b"InErrors", UDP_IN_ERRS.load(Ordering::Relaxed));
    line(b"OutDatagrams", UDP_OUT.load(Ordering::Relaxed));
    out
}

// ---------------------------------------------------------------------------
// tcp (contract §5): the accept pool (P-A/§12.3) + the blocking connect
// bridge (P-K), close/hangup/hangupxmit (P-L), keepalive, the OQ-4
// accepted-and-recorded verbs, the status/stats files (P-M), and the gc.
// smoltcp owns the state machine (D-01); this module is the devip
// semantics around it — exactly where the phase's bugs live, so the
// behavior pins in devip_tests.rs landed BEFORE this module (TDD RED).
//
// THE STATS HONESTY RULE (P-M): the bridge increments a counter at every
// packet transition it OBSERVES (connect initiation → ActiveOpens; accept
// pop → PassiveOpens; send/recv completions → OutSegs/InSegs; abort() →
// OutRsts).  smoltcp exposes NO packet counters and no retransmit/reseq/
// recovery internals, so those tags print 0 — real where observable, zero
// where not, never fabricated (the contract's field-mapping rule).  A
// stats file that reads all zeros forever is a contract violation (D-08);
// the gate asserts non-zero counters after the echo exchange.
// ---------------------------------------------------------------------------

/// The 28 stats tags (tcp.c:348-375's statnames[] — the SOURCE's list;
/// the man documents only 11, [man-vs-src] recorded in contract §5.3).
pub(crate) const TCPSTAT_MAX_CONN: usize = 0;
pub(crate) const TCPSTAT_MAX_SEGMENT: usize = 1;
pub(crate) const TCPSTAT_ACTIVE_OPENS: usize = 2;
pub(crate) const TCPSTAT_PASSIVE_OPENS: usize = 3;
#[allow(dead_code)] // EstabResets is zero-mapped this era (see TCP_STATS)
pub(crate) const TCPSTAT_ESTAB_RESETS: usize = 4;
pub(crate) const TCPSTAT_CURR_ESTAB: usize = 5;
pub(crate) const TCPSTAT_IN_SEGS: usize = 6;
pub(crate) const TCPSTAT_OUT_SEGS: usize = 7;
#[allow(dead_code)] // the retransmit/reseq/recovery block: smoltcp-internal, zero-mapped
pub(crate) const TCPSTAT_RETRANS_SEGS: usize = 8;
pub(crate) const TCPSTAT_IN_ERRS: usize = 12;
pub(crate) const TCPSTAT_OUT_RSTS: usize = 13;

const TCP_STAT_NAMES: [&[u8]; 28] = [
    b"MaxConn",
    b"MaxSegment",
    b"ActiveOpens",
    b"PassiveOpens",
    b"EstabResets",
    b"CurrEstab",
    b"InSegs",
    b"OutSegs",
    b"RetransSegs",
    b"RetransSegsSent",
    b"RetransTimeouts",
    b"InErrs",
    b"OutRsts",
    b"CsumErrs",
    b"HlenErrs",
    b"LenErrs",
    b"OutOfOrder",
    b"Resequenced",
    b"ReseqBytelim",
    b"ReseqPktlim",
    b"Delayack",
    b"Wopenack",
    b"Recovery",
    b"RecoveryDone",
    b"RecoveryRTO",
    b"RecoveryNoSeq",
    b"RecoveryCwind",
    b"RecoveryPA",
];

/// The bridge-maintained counters (pub(crate) for the P-M pin's
/// delta assertions).
pub(crate) static TCP_STATS: [AtomicU64; 28] = [const { AtomicU64::new(0) }; 28];

/// The CurrEstab decrement (conv_close's tcp arm — the conv left
/// Established).
pub(crate) fn tcp_curr_estab_dec() {
    let _ = TCP_STATS[TCPSTAT_CURR_ESTAB]
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_sub(1));
}

/// The accept-pop bookkeeping (devip.rs's listen path): the incoming call
/// materialized → PassiveOpens; an already-Established member counts into
/// CurrEstab and marks the new conv (its close unwinds).
pub(crate) fn tcp_note_passive_open(t: &mut conv::ConvTables, nid: usize, established: bool) {
    TCP_STATS[TCPSTAT_PASSIVE_OPENS].fetch_add(1, Ordering::Relaxed);
    if established {
        t.table_mut(Proto::Tcp)[nid].estab = true;
        TCP_STATS[TCPSTAT_CURR_ESTAB].fetch_add(1, Ordering::Relaxed);
    }
}

/// The donor's `tcpporthogdefense` GLOBAL toggle (random-sequence
/// keepalives shut hijacked connections) — no smoltcp mechanism; OQ-4's
/// accepted-and-recorded verb: the static is the honest record (contract
/// §5.1).  Never read: nothing steers on it this era.
#[allow(dead_code)]
static TCP_PORT_HOG_DEFENSE: AtomicU64 = AtomicU64::new(0);

/// The bridge's connect deadline (P-K's Etimedout arm): the donor's SYN
/// retransmit schedule gives up at MAXBACKOFF ≈ 2–3 min; smoltcp
/// retransmits FOREVER (no give-up), so the Etimedout mapping needs a
/// bridge-side deadline.  120 s sits in the donor's envelope, past
/// smoltcp's 60 s RTO max (at least one max-interval retransmit precedes
/// the give-up), and is never exercised by the happy-path gates.  The
/// host pin fast-forwards the stub clock past it.
const TCP_CONNECT_TIMEOUT_MS: i64 = 120_000;

/// The 11-state rename table (contract §5.2): smoltcp `tcp::State` → the
/// devip `tcpstates[]` strings — a pure 1:1 rename, pinned arm-by-arm.
pub(crate) fn tcp_state_name(s: tcp::State) -> &'static [u8] {
    match s {
        tcp::State::Closed => b"Closed",
        tcp::State::Listen => b"Listen",
        tcp::State::SynSent => b"Syn_sent",
        tcp::State::SynReceived => b"Syn_received",
        tcp::State::Established => b"Established",
        tcp::State::FinWait1 => b"Finwait1",
        tcp::State::FinWait2 => b"Finwait2",
        tcp::State::CloseWait => b"Close_wait",
        tcp::State::Closing => b"Closing",
        tcp::State::LastAck => b"Last_ack",
        tcp::State::TimeWait => b"Time_wait",
    }
}

/// Create a tcp socket (2048/2048 buffers per contract §12.3, hop limit
/// = conv.ttl — §12.8's no-leak rule).
fn tcp_socket(ttl: u32) -> tcp::Socket<'static> {
    let mut s = tcp::Socket::new(
        tcp::SocketBuffer::new(alloc::vec![0; stack::POOL_SOCK_BUF]),
        tcp::SocketBuffer::new(alloc::vec![0; stack::POOL_SOCK_BUF]),
    );
    if ttl > 0 && ttl <= 255 {
        s.set_hop_limit(Some(ttl as u8));
    }
    s
}


/// The P-K blocking connect wait (the donor's connectctlmsg sleep on
/// c->cr until Connected/failed): probe the socket state → Established
/// maps Connected; Closed mid-handshake maps Econrefused (the donor's
/// localclose(s, Econrefused) on RST-to-SYN); the bridge deadline maps
/// Etimedout (the donor's retransmit-exhaustion arm — smoltcp retransmits
/// forever, so the deadline is ours).  The wait is the seam's
/// wakeup-loss-free shape — probe → DRIVE → RE-PROBE → register/arm →
/// block: on loopback the drive completes the handshake SYNCHRONOUSLY
/// (the SYN loops in, the SYN-ACK loops back — the Established
/// transition and its waker fire DURING the drive), so the re-probe
/// after the drive is load-bearing (a wake fired mid-drive readies a
/// RUNNING proc — a no-op; without the re-probe the block wedges).
fn tcp_connect_wait(id: usize) -> Result<()> {
    let deadline = stack::instant_now() + Duration::from_millis(TCP_CONNECT_TIMEOUT_MS as u64);
    let mut hooks: Option<stack::DevipHooks> = None;
    enum Probe {
        Established,
        Closed,
        Progress,
    }
    // One probe of the socket state (TABLES → STACK nesting).
    let probe_state = |id: usize| {
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        match t.table(Proto::Tcp)[id].sock {
            Some(h) => {
                let g = stack::STACK.enter().expect("devip: stack contended");
                match g.core.as_ref() {
                    Some(core) => match core.sockets.get::<tcp::Socket>(h).state() {
                        tcp::State::Established => Probe::Established,
                        tcp::State::Closed => Probe::Closed,
                        _ => Probe::Progress,
                    },
                    None => Probe::Closed,
                }
            }
            None => Probe::Closed,
        }
    };
    loop {
        match probe_state(id) {
            Probe::Established => {
                let node = LockNode::new();
                let mut t = conv::TABLES.lock(&node);
                t.table_mut(Proto::Tcp)[id].estab = true;
                TCP_STATS[TCPSTAT_CURR_ESTAB].fetch_add(1, Ordering::Relaxed);
                super::netlog::netlog_dbg!(super::netlog::LOG_TCP, "devip: tcp {id} connect established");
                return Ok(());
            }
            Probe::Closed => {
                super::netlog::netlog_dbg!(super::netlog::LOG_TCP, "devip: tcp {id} connect refused");
                return Err(tcp_connect_fail(id, "connection refused"));
            }
            Probe::Progress => {}
        }
        if stack::instant_now() >= deadline {
            super::netlog::netlog_dbg!(super::netlog::LOG_TCP, "devip: tcp {id} connect timed out");
            return Err(tcp_connect_fail(id, "connection timed out"));
        }
        // Drive (the SYN leaves / the loopback handshake completes here),
        // then RE-PROBE before registering — a wake fired mid-drive is
        // consumed by the re-probe, never by a later block.
        stack::drive_stack();
        match probe_state(id) {
            Probe::Established => continue, // the top arm records + returns
            Probe::Closed => continue,      // the top arm maps Econrefused
            Probe::Progress => {}
        }
        if stack::instant_now() >= deadline {
            return Err(tcp_connect_fail(id, "connection timed out"));
        }
        // Register, drop, block (devpipe's wakeup-loss-free shape — the
        // waker can only fire after block() swtch'es: the IRQ-masked
        // single-CPU syscall invariant).
        if hooks.is_none() {
            hooks = Some(stack::hooks()?);
        }
        let hks = hooks.unwrap();
        let pid = (hks.current)();
        {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            let c = &mut t.table_mut(Proto::Tcp)[id];
            c.waiters.register(pid);
            if let Some(h) = c.sock {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                if let Some(core) = g.core.as_mut() {
                    super::TCP_RX_SLOTS[id].arm(pid);
                    core.sockets
                        .get_mut::<tcp::Socket>(h)
                        .register_recv_waker(&crate::devip_waker::waker_for(&super::TCP_RX_SLOTS[id]));
                }
            }
        }
        stack::register_net_waiter(pid);
        (hks.block)();
    }
}

/// The connect failure tail (the donor's localclose + Fsconnected(c,
/// reason)): the socket is dropped, the conv goes Connected at the FILE
/// level with the reason hung on it (reads/writes error Ehungup; the err
/// file carries the reason — contract §3), and any co-blocked readers
/// wake.  The ctl write then errors with the reason.
fn tcp_connect_fail(id: usize, reason: &'static str) -> &'static str {
    let sock = {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        let c = &mut t.table_mut(Proto::Tcp)[id];
        c.state = ConvState::Connected;
        c.hangup = Some(reason.as_bytes().to_vec());
        c.errq.push_back(reason.as_bytes().to_vec());
        c.sock.take()
    };
    if let Some(h) = sock {
        // No RST needed (the refused case: the peer already sent it; the
        // timeout case: nobody is there) — just remove the half-open
        // socket from the set.
        let mut g = stack::STACK.enter().expect("devip: stack contended");
        if let Some(core) = g.core.as_mut() {
            core.sockets.remove(h);
        }
    }
    conv::wake_conv(Proto::Tcp, id);
    reason
}

/// tcp's own ctl verbs (tcpctl, tcp.c:3330-3345).
fn tcp_ctl(id: usize, verb: &[u8], args: &[&[u8]]) -> Result<()> {
    match verb {
        b"close" if args.is_empty() => tcp_close_verb(id, CloseKind::Graceful),
        b"hangup" if args.is_empty() => tcp_close_verb(id, CloseKind::Abort),
        b"hangupxmit" if args.is_empty() => tcp_close_verb(id, CloseKind::XmitDrain),
        b"keepalive" => {
            // tcpstartka (tcp.c:2924-2941): requires Established — the
            // donor's typo is the string VERBATIM; the default is 30000 ms
            // (contract §5.1).  Values below MSPTICK (50 ms) leave the
            // donor's timer unset — the record still applies (the conv
            // carries the value; the era's smoltcp mapping sets it).
            let ms = match args.first() {
                None => 30_000,
                Some(a) => atoi(a),
            };
            let sock = {
                let node = LockNode::new();
                let t = conv::TABLES.lock(&node);
                t.table(Proto::Tcp)[id].sock
            };
            let Some(h) = sock else {
                return Err("connection must be in Establised state");
            };
            let mut g = stack::STACK.enter().expect("devip: stack contended");
            let Some(core) = g.core.as_mut() else {
                return Err("devip: no bound medium");
            };
            let s = core.sockets.get_mut::<tcp::Socket>(h);
            if s.state() != tcp::State::Established {
                return Err("connection must be in Establised state");
            }
            s.set_keep_alive(Some(Duration::from_millis(u64::from(ms))));
            Ok(())
        }
        b"checksum" => {
            // tcpsetchecksum: `nochecksum = !atoi(f[1])` — n==0 emits zero
            // checksums.  smoltcp has NO such mechanism (checksums are
            // always computed) — OQ-4's resolution (contract §5.1, D-08):
            // accepted-and-recorded with a netlog note, never
            // loud-unimplemented.  (The donor dereferences f[1]
            // unconditionally; a bare `checksum` maps to n=1 here —
            // checksums stay on.)
            let n = args.first().map(|a| atoi(a)).unwrap_or(1);
            {
                let node = LockNode::new();
                let mut t = conv::TABLES.lock(&node);
                t.table_mut(Proto::Tcp)[id].nochecksum = n == 0;
            }
            super::netlog::emit(
                super::netlog::LOG_TCP,
                alloc::format!("tcp: checksum {n} accepted-and-recorded (no smoltcp mechanism)").into_bytes(),
            );
            Ok(())
        }
        b"tcpporthogdefense" => {
            // tcpporthogdefensectl: a GLOBAL toggle, `on`|`off` only.
            // No smoltcp mechanism — OQ-4's accepted-and-recorded shape
            // with a netlog note.
            let Some(v) = args.first() else {
                return Err("unknown value for tcpporthogdefense");
            };
            let on = match *v {
                b"on" => 1u64,
                b"off" => 0u64,
                _ => return Err("unknown value for tcpporthogdefense"),
            };
            TCP_PORT_HOG_DEFENSE.store(on, Ordering::Relaxed);
            super::netlog::emit(
                super::netlog::LOG_TCP,
                alloc::format!("tcp: tcpporthogdefense {} accepted-and-recorded (no smoltcp mechanism)", if on == 1 { "on" } else { "off" })
                    .into_bytes(),
            );
            Ok(())
        }
        _ => Err("unknown control request"),
    }
}

/// The close-kind (P-L; tcp.c's tcpclose2/tcphangup/tcpxmitclose).
enum CloseKind {
    /// `close` → `socket.close()` (FIN).  smoltcp's egress emits the FIN
    /// only once the segment covers the TX buffer's tail — the FIN is
    /// sequenced AFTER queued data by construction.
    Graceful,
    /// `hangup` → `socket.abort()` (RST — "one reset packet will be
    /// sent", smoltcp socket/tcp.rs).
    Abort,
    /// `hangupxmit` (source-only verb) → close-after-TX-drain.  On the
    /// wire this IS close() (the FIN never precedes queued data); the
    /// donor's close-vs-hangupxmit distinction is the rq flush, which
    /// smoltcp has no mechanism for — a recorded mapping corner.
    XmitDrain,
}

/// The close/hangup/hangupxmit verbs.  On an ANNOUNCED conv (the
/// listener): the donor's tcpclose/tcpxmitclose Listen arms run
/// `Fsconnected(c, "Hangup")` — the parked listen open wakes and errors
/// "listen hungup"; the pool tears down (unpopped members are aborted —
/// a recorded corner: the donor FINs its incall convs, but r9's unpopped
/// members have no conv to carry a graceful close).  On a connected conv:
/// the socket close/abort per the kind.
fn tcp_close_verb(id: usize, kind: CloseKind) -> Result<()> {
    let (is_listener, sock, pool) = {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        let c = &mut t.table_mut(Proto::Tcp)[id];
        let is_listener = c.pool.is_some()
            || matches!(c.state, ConvState::Announced | ConvState::Announcing);
        if is_listener {
            c.hangup = Some(b"Hangup".to_vec());
            c.errq.push_back(b"Hangup".to_vec());
        }
        // The listener's own socket (it has none — defensive take) and the
        // pool leave the conv; a CONNECTED conv KEEPS its socket handle —
        // the donor's close/hangup doesn't free the conv (the status file
        // still reads the closing state machine; conv_close owns the drop).
        (is_listener, if is_listener { c.sock.take() } else { c.sock }, c.pool.take())
    };
    if is_listener {
        if let Some(pool) = pool {
            pool.teardown();
        }
        if let Some(h) = sock {
            // An announced conv carries no socket of its own; defensive.
            let mut g = stack::STACK.enter().expect("devip: stack contended");
            if let Some(core) = g.core.as_mut() {
                core.sockets.remove(h);
            }
        }
        // The parked listener's wake (after the table guard dropped).
        conv::wake_conv(Proto::Tcp, id);
        return Ok(());
    }
    let Some(h) = sock else {
        // An Idle conv: the donor's localclose on a Closed tcb — a
        // no-op at the file level.  Accept.
        return Ok(());
    };
    {
        let mut g = stack::STACK.enter().expect("devip: stack contended");
        if let Some(core) = g.core.as_mut() {
            let s = core.sockets.get_mut::<tcp::Socket>(h);
            match kind {
                CloseKind::Graceful | CloseKind::XmitDrain => s.close(),
                CloseKind::Abort => {
                    s.abort();
                    TCP_STATS[TCPSTAT_OUT_RSTS].fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    // CurrEstab unwinds (the conv left/was leaving Established).
    {
        let node = LockNode::new();
        let mut t = conv::TABLES.lock(&node);
        let c = &mut t.table_mut(Proto::Tcp)[id];
        if c.estab {
            c.estab = false;
            tcp_curr_estab_dec();
        }
    }
    // The FIN/RST leaves now (the generic ctl tail also drives).
    stack::drive_stack();
    Ok(())
}

/// The tcp data read (the donor's `qread(c->rq)` — a BYTE STREAM):
/// recv from the conv's socket; EOF(0) when the peer closed and the
/// buffer drained (RecvError::Finished — the donor's qread-returns-0
/// arm); Ehungup when the conv's queues are hung (the connect-failure /
/// hangup markers — qhangup's shape); otherwise the seam's probe →
/// drive → re-probe → register/arm → block.
pub(crate) fn tcp_read_data(id: usize, buf: &mut [u8]) -> Result<u64> {
    let mut hooks: Option<stack::DevipHooks> = None;
    loop {
        enum R {
            Done(u64),
            Eof,
            Block,
        }
        let r = {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            let c = &mut t.table_mut(Proto::Tcp)[id];
            if c.hangup.is_some() {
                return Err("i/o on hungup channel");
            }
            match c.sock {
                Some(h) => {
                    let mut g = stack::STACK.enter().expect("devip: stack contended");
                    let Some(core) = g.core.as_mut() else {
                        return Err("devip: no bound medium");
                    };
                    let s = core.sockets.get_mut::<tcp::Socket>(h);
                    if s.can_recv() {
                        match s.recv_slice(buf) {
                            Ok(n) => {
                                TCP_STATS[TCPSTAT_IN_SEGS].fetch_add(1, Ordering::Relaxed);
                                R::Done(n as u64)
                            }
                            // can_recv raced the state; the Finished arm
                            // is the graceful EOF.
                            Err(_) => R::Eof,
                        }
                    } else if !s.may_recv() {
                        // Peer closed and the buffer is drained (or the
                        // socket aborted) → the donor's EOF read.
                        R::Eof
                    } else {
                        R::Block
                    }
                }
                None => {
                    // Socket-less: an Idle conv EOFs (closeconv's
                    // qclosed arm — a gc-stolen conv's reader EOFs); an
                    // announced conv's data read BLOCKS (the listener
                    // never receives — the donor's qread on its rq).
                    if c.state == ConvState::Idle {
                        R::Eof
                    } else {
                        R::Block
                    }
                }
            }
        };
        match r {
            R::Done(n) => return Ok(n),
            R::Eof => return Ok(0),
            R::Block => {}
        }
        // Flush pending loopback work before blocking (the D-05 leg-3
        // read side), then re-probe via the loop.
        stack::drive_stack();
        {
            let node = LockNode::new();
            let mut t = conv::TABLES.lock(&node);
            let c = &mut t.table_mut(Proto::Tcp)[id];
            if c.hangup.is_some() {
                continue; // the top probe takes the error
            }
            let can = if let Some(h) = c.sock {
                let g = stack::STACK.enter().expect("devip: stack contended");
                match g.core.as_ref() {
                    Some(core) => {
                        let s = core.sockets.get::<tcp::Socket>(h);
                        s.can_recv() || !s.may_recv()
                    }
                    None => true,
                }
            } else {
                c.state == ConvState::Idle
            };
            if can {
                continue; // the top probe delivers/EOFs
            }
            // Register, drop, block (devpipe's wakeup-loss-free shape).
            if hooks.is_none() {
                hooks = Some(stack::hooks()?);
            }
            let hks = hooks.unwrap();
            let pid = (hks.current)();
            c.waiters.register(pid);
            stack::register_net_waiter(pid);
            if let Some(h) = c.sock {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                if let Some(core) = g.core.as_mut() {
                    super::TCP_RX_SLOTS[id].arm(pid);
                    core.sockets
                        .get_mut::<tcp::Socket>(h)
                        .register_recv_waker(&crate::devip_waker::waker_for(&super::TCP_RX_SLOTS[id]));
                }
            }
        }
        (hooks.unwrap().block)();
    }
}

/// The tcp data write (the donor's qwrite(c->wq)): queue the bytes into
/// the socket's TX buffer, looping while a full buffer blocks (register
/// the SEND waker, drive, block — the icmp write's shape).  Writes on a
/// hung conv error Ehungup; writes on a socket whose send half closed
/// (our close/peer's full close) error Ehungup (the donor's
/// qwrite-on-closed-queue arm).
pub(crate) fn tcp_write_data(id: usize, buf: &[u8]) -> Result<u64> {
    let n = buf.len();
    let mut hooks: Option<stack::DevipHooks> = None;
    let mut sent = 0usize;
    while sent < n {
        enum W {
            Sent(usize),
            Block,
        }
        let w = {
            let node = LockNode::new();
            let t = conv::TABLES.lock(&node);
            let c = &t.table(Proto::Tcp)[id];
            if c.hangup.is_some() {
                return Err("i/o on hungup channel");
            }
            match c.sock {
                Some(h) => {
                    let mut g = stack::STACK.enter().expect("devip: stack contended");
                    let Some(core) = g.core.as_mut() else {
                        return Err("devip: no bound medium");
                    };
                    let s = core.sockets.get_mut::<tcp::Socket>(h);
                    if !s.may_send() {
                        // Closed send half — the donor's qwrite on a hung
                        // wq.
                        return Err("i/o on hungup channel");
                    }
                    if !s.can_send() {
                        W::Block
                    } else {
                        match s.send_slice(&buf[sent..]) {
                            Ok(0) => W::Block, // a zero enqueue: full — block
                            Ok(m) => {
                                TCP_STATS[TCPSTAT_OUT_SEGS].fetch_add(1, Ordering::Relaxed);
                                W::Sent(m)
                            }
                            Err(_) => return Err("i/o on hungup channel"),
                        }
                    }
                }
                None => {
                    // A never-connected conv: the donor's qwrite queues
                    // the block and the state machine never drains it —
                    // the write SUCCEEDS (fire-and-forget, the icmp/udp
                    // never-connected shape).  Counted, never silent.
                    TCP_STATS[TCPSTAT_IN_ERRS].fetch_add(1, Ordering::Relaxed);
                    return Ok(n as u64);
                }
            }
        };
        match w {
            W::Sent(m) => sent += m,
            W::Block => {
                // Buffer full.  Drive FIRST (a peer ACK already queued
                // frees space now), then re-probe before parking — a
                // send waker fired mid-drive readies a RUNNING proc (a
                // no-op), so the register+block below must come after a
                // post-drive re-probe (the connect wait's shape).
                stack::drive_stack();
                let sendable = {
                    let node = LockNode::new();
                    let t = conv::TABLES.lock(&node);
                    let c = &t.table(Proto::Tcp)[id];
                    if c.hangup.is_some() {
                        return Err("i/o on hungup channel");
                    }
                    match c.sock {
                        Some(h) => {
                            let g = stack::STACK.enter().expect("devip: stack contended");
                            match g.core.as_ref() {
                                Some(core) => {
                                    let s = core.sockets.get::<tcp::Socket>(h);
                                    s.may_send() && s.can_send()
                                }
                                None => false,
                            }
                        }
                        None => false,
                    }
                };
                if sendable {
                    continue;
                }
                if hooks.is_none() {
                    hooks = Some(stack::hooks()?);
                }
                let hks = hooks.unwrap();
                let pid = (hks.current)();
                {
                    let node = LockNode::new();
                    let mut t = conv::TABLES.lock(&node);
                    let c = &mut t.table_mut(Proto::Tcp)[id];
                    c.waiters.register(pid);
                    if let Some(h) = c.sock {
                        let mut g = stack::STACK.enter().expect("devip: stack contended");
                        if let Some(core) = g.core.as_mut() {
                            super::TCP_TX_SLOTS[id].arm(pid);
                            core.sockets
                                .get_mut::<tcp::Socket>(h)
                                .register_send_waker(&crate::devip_waker::waker_for(&super::TCP_TX_SLOTS[id]));
                        }
                    }
                }
                stack::register_net_waiter(pid);
                (hks.block)();
            }
        }
    }
    // D-05 leg 3: the mutating write ends with a drive — the loopback
    // loops the segment synchronously.
    stack::drive_stack();
    Ok(n as u64)
}

/// The tcp status line (tcpstate, tcp.c:493-509 — contract §5.2):
/// `"State qin %d qout %d rq %d.%d srtt %d mdev %d sst %lud cwin %lud
/// swin %lud>>%d rwin %lud>>%d qscale %d timer.start %d timer.count %d
/// rerecv %d katimer.start %d katimer.count %d\n"`.  Real where smoltcp
/// exposes it (state via the rename table, qin/qout = the socket's
/// recv/send queue bytes), fixed/zero elsewhere (smoltcp 0.14 exposes
/// none of the timer/reseq/window-scale internals — the contract's
/// zero-mapped tail; gates assert the FORMAT and the state/qin/qout
/// fields, never the zeros).
pub(crate) fn tcp_status(id: usize) -> Vec<u8> {
    let (name, qin, qout) = {
        let node = LockNode::new();
        let t = conv::TABLES.lock(&node);
        let c = &t.table(Proto::Tcp)[id];
        match c.sock {
            Some(h) => {
                let g = stack::STACK.enter().expect("devip: stack contended");
                match g.core.as_ref() {
                    Some(core) => {
                        let s = core.sockets.get::<tcp::Socket>(h);
                        (tcp_state_name(s.state()), s.recv_queue() as u64, s.send_queue() as u64)
                    }
                    None => (b"Closed".as_ref(), 0, 0),
                }
            }
            // The ANNOUNCED conv (the pool owner) IS the listener; an
            // Idle conv reads Closed (the donor's tcpcreate state).
            None => {
                if c.pool.is_some() {
                    (b"Listen".as_ref(), 0, 0)
                } else {
                    (b"Closed".as_ref(), 0, 0)
                }
            }
        }
    };
    let mut out = Vec::new();
    out.extend_from_slice(name);
    out.extend_from_slice(b" qin ");
    out.extend_from_slice(&dec(qin));
    out.extend_from_slice(b" qout ");
    out.extend_from_slice(&dec(qout));
    out.extend_from_slice(
        b" rq 0.0 srtt 0 mdev 0 sst 0 cwin 0 swin 0>>0 rwin 0>>0 qscale 0 timer.start 0 timer.count 0 rerecv 0 katimer.start 0 katimer.count 0\n",
    );
    out
}

/// The tcp stats file (tcpstats, tcp.c:3348-3360): the 28 tagged lines
/// from the bridge-maintained counters (P-M).
pub(crate) fn tcp_stats() -> Vec<u8> {
    let mut out = Vec::new();
    for (i, name) in TCP_STAT_NAMES.iter().enumerate() {
        out.extend_from_slice(name);
        out.extend_from_slice(b": ");
        out.extend_from_slice(&dec(TCP_STATS[i].load(Ordering::Relaxed)));
        out.push(b'\n');
    }
    out
}

/// The tcp gc (contract §5.4 — the donor's tcpgc reaps timed-out convs
/// under clone pressure; the smoltcp-era mapping: steal a slot whose
/// socket is FULLY CLOSED).  The victim's socket leaves the SocketSet;
/// its blocked readers wake to EOF (the reset's Idle-state read arm).
/// Returns the stolen slot, reset, inuse = 1 (the clone-open shape).
/// The caller (conv_alloc) holds TABLES.
pub(crate) fn tcp_gc(t: &mut conv::ConvTables) -> Option<usize> {
    // Find a fully-Closed socket (TABLES → STACK, the documented nesting).
    let victim = {
        let mut g = stack::STACK.enter().expect("devip: stack contended");
        let core = g.core.as_mut()?;
        let mut found = None;
        for (id, c) in t.table(Proto::Tcp).iter().enumerate() {
            if let Some(h) = c.sock
                && core.sockets.get::<tcp::Socket>(h).state() == tcp::State::Closed
            {
                found = Some((id, h));
                break;
            }
        }
        if let Some((_, h)) = found {
            core.sockets.remove(h);
        }
        found.map(|(id, _)| id)
    }?;
    let mut pids = [0u64; 16];
    let n = {
        let c = &mut t.table_mut(Proto::Tcp)[victim];
        let n = c.waiters.take(&mut pids);
        c.reset();
        c.allocated = true;
        n
    };
    if n > 0
        && let Ok(h) = stack::hooks()
    {
        (h.wake)(&pids[..n]);
    }
    Some(victim)
}
