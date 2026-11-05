//! devip/medium.rs — the medium registry (contract §10; the donor's
//! `media[]` + `ipfindmedium`).  Registered this plan: **loopback** and
//! **null** (the locked loopback+null-FIRST sequencing); the ether
//! medium's name resolves in 08-6 when the ethermedium binding lands
//! (until then `bind ether …` errors "unknown interface type" like any
//! unregistered name — the same string `bind pkt`/`bind netdev` get).
//!
//! - `bind loopback` — creates the loopback ipifc over the seam's
//!   `LoopbackDev` (the fixed-pool `Medium::Ip` Device, MTU 16384 —
//!   NEVER smoltcp's per-packet-allocating `phy::Loopback`): installs
//!   the seam's `StackCore` and prints the bound marker.
//! - `bind null` — errors EXACTLY `"cannot bind null device"`
//!   (nullmedium.c:9-13).  That error IS the null-medium contract: the
//!   name resolves, the bind fails.

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::Result;

use super::stack;

/// The registered medium names.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MediumKind {
    Loopback,
}

/// Resolve a medium name (ipfindmedium): "loopback" → the kind;
/// "null" → the null-medium error (the name RESOLVES — the bind is what
/// fails); anything else → "unknown interface type".
pub(crate) fn find(name: &[u8]) -> Result<MediumKind> {
    match name {
        b"loopback" => Ok(MediumKind::Loopback),
        b"null" => Err("cannot bind null device"),
        _ => Err("unknown interface type"),
    }
}

/// The medium's bind action: install the seam's loopback core and name
/// the interface (the donor's `snprint(ifc->dev, "%s%d", medium->name,
/// c->x)`).  Returns the devname for the ipifc record.
pub(crate) fn bind_medium(kind: MediumKind, conv_id: usize) -> Result<Vec<u8>> {
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
    }
}

/// The medium's maxtu (contract §10: loopback 16 KiB —
/// loopbackmedium.c:11-13).
pub(crate) fn maxtu(kind: MediumKind) -> u32 {
    match kind {
        MediumKind::Loopback => 16384,
    }
}

/// The medium's mintu (loopbackmedium's 0).
pub(crate) fn mintu(kind: MediumKind) -> u32 {
    match kind {
        MediumKind::Loopback => 0,
    }
}
