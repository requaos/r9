//! devip/medium.rs — the medium registry (contract §10; the donor's
//! `media[]` + `ipfindmedium`).  Registered: **loopback**, **null**, and
//! **ether** (08-06 — the locked loopback+null-FIRST sequencing completed
//! in 08-04; the ether medium consumes Phase 7's devether).
//!
//! - `bind loopback` — creates the loopback ipifc over the seam's
//!   `LoopbackDev` (the fixed-pool `Medium::Ip` Device, MTU 16384 —
//!   NEVER smoltcp's per-packet-allocating `phy::Loopback`): installs
//!   the seam's loopback core and prints the bound marker.
//! - `bind null` — errors EXACTLY `"cannot bind null device"`
//!   (nullmedium.c:9-13).  That error IS the null-medium contract: the
//!   name resolves, the bind fails.
//! - `bind ether <path>` — validates the path names the one ether (the
//!   r9 era's two spellings: `#l0` / `/net/ether0` — the adapter binds
//!   the single published virtio-net device; an unknown path errors
//!   "unknown interface type"), then installs the ether core over
//!   `EtherDev` and prints `devip: medium ether bound #l0`.

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::Result;

use super::stack;

/// The registered medium names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MediumKind {
    Loopback,
    Ether,
}

/// Is the name a REGISTERED medium (the donor's ipfindmedium-resolves
/// predicate — arp.rs's `add` medium check; note this is name resolution
/// only: `null` resolves here even though its BIND always fails)?
pub(crate) fn is_known(name: &[u8]) -> bool {
    matches!(name, b"loopback" | b"null" | b"ether")
}

/// Resolve a medium name (ipfindmedium): "loopback"/"ether" → the kind;
/// "null" → the null-medium error (the name RESOLVES — the bind is what
/// fails); anything else → "unknown interface type".
pub(crate) fn find(name: &[u8]) -> Result<MediumKind> {
    match name {
        b"loopback" => Ok(MediumKind::Loopback),
        b"ether" => Ok(MediumKind::Ether),
        b"null" => Err("cannot bind null device"),
        _ => Err("unknown interface type"),
    }
}

/// The ether path spellings the era admits (the single published
/// virtio-net device): `#l0` (the device's own path) and `/net/ether0`
/// (its /net alias — ipconfig's default dev).  Anything else errors
/// "unknown interface type" (08-06's plan: with exactly one ether
/// existing, a path that doesn't name it is the unknown-interface arm).
fn ether_path_ok(path: &[u8]) -> bool {
    matches!(path, b"#l0" | b"/net/ether0" | b"ether0")
}

/// The medium's bind action: install the medium's seam core and name
/// the interface (the donor's `snprint(ifc->dev, "%s%d", medium->name,
/// c->x)`).  Returns the devname for the ipifc record.
pub(crate) fn bind_medium(kind: MediumKind, conv_id: usize, path: Option<&[u8]>) -> Result<Vec<u8>> {
    match kind {
        MediumKind::Loopback => {
            {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                if g.core.is_some() {
                    return Err("interface already bound");
                }
                g.core = Some(stack::StackCore::new_loopback(super::phyadapt::LoopbackDev::new()));
            }
            // D-13's serial marker (the gate asserts it).
            crate::println!("devip: medium loopback bound");
            let mut name = Vec::new();
            name.extend_from_slice(b"loopback");
            name.push(b'0' + (conv_id % 10) as u8);
            Ok(name)
        }
        MediumKind::Ether => {
            // The path validation BEFORE any allocation (an unknown
            // path never allocates conversations).
            if let Some(p) = path
                && !ether_path_ok(p)
            {
                return Err("unknown interface type");
            }
            // The double-bind check BEFORE the conv allocation (never
            // leak fresh convs on the "interface already bound" arm).
            // The IFC slot check in ipifc.rs's bind fires first (it
            // holds the IFC lock across this call); this is the
            // defensive twin.
            {
                let g = stack::STACK.enter().expect("devip: stack contended");
                if g.ether.is_some() {
                    return Err("interface already bound");
                }
            }
            // The adapter's construction (the netif conv machinery +
            // the published driver ops) OUTSIDE the stack cell (netif's
            // own locks; the seam never holds STACK across them).
            let dev = super::phyadapt::EtherDev::bind()?;
            {
                let mut g = stack::STACK.enter().expect("devip: stack contended");
                g.ether = Some(stack::StackCore::new_ether(dev));
            }
            // D-13's serial marker (the gate asserts it): the bound
            // device is #l0's single published NIC ("/net/ether0" is its
            // /net alias).
            crate::println!("devip: medium ether bound #l0");
            Ok(Vec::from(&b"ether0"[..]))
        }
    }
}

/// The medium's maxtu (contract §10: loopback 16 KiB —
/// loopbackmedium.c:11-13; ether 1514 — INCLUDES the 14-byte header).
pub(crate) fn maxtu(kind: MediumKind) -> u32 {
    match kind {
        MediumKind::Loopback => 16384,
        MediumKind::Ether => 1514,
    }
}

/// The medium's mintu (loopbackmedium's 0; ether's 60).
pub(crate) fn mintu(kind: MediumKind) -> u32 {
    match kind {
        MediumKind::Loopback => 0,
        MediumKind::Ether => 60,
    }
}
