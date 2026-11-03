//! pgrp — the per-process namespace group: the mount table (05-04, D-03).
//!
//! Semantics ported literally from 9k's pgrp.c/chan.c (cmount's list
//! insertion, cunmount, pgrpcpy's deep copy): a mount point carries an
//! ORDERED union chain — MBEFORE binds (latest first), then the underlying
//! directory (present iff the first bind at the point was not MREPL), then
//! MAFTER binds (bind order); MREPL replaces the whole chain.  The frozen
//! M* flag values are re-exported from dat.rs.
//!
//! The group is shared state: Proc holds it as `Arc<Lock<Pgrp>>` (the arch
//! side's wiring), and the rfork flag semantics follow fork(2) — neither
//! RFNAMEG nor RFCNAMEG (with RFPROC) shares the handle, RFNAMEG
//! deep-copies (pgrpcpy), RFCNAMEG starts clean (the 05-03 finding's
//! newns wipe).
//!
//! The dat.rs `Dev` trait is not involved here (the established idiom —
//! see devroot.rs's header); this module is kernel state, not a device.

use alloc::vec::Vec;

pub use crate::dat::{MAFTER, MBEFORE, MCACHE, MCREATE, MMASK, MORDER, MREPL};
use crate::dat::{Chan, MountEntry};
use crate::Result;

/// Plan 9 Qid type byte (sys/include/libc.h) — the dir/file distinction
/// cmount's QTDIR-xor check needs.
const QTDIR: u8 = 0x80;

/// A mount point (the donor's Mhead): the normalized target path plus its
/// ordered union chain.
struct MountPoint {
    /// Normalized absolute target path ("" = the root; no trailing '/').
    target: Vec<u8>,
    /// The union chain in 9k order (see the module header).
    chain: Vec<MountEntry>,
}

/// The per-process namespace group (the donor's Pgrp mount half — the env
/// half arrives with devenv in a later plan).
pub struct Pgrp {
    mounts: Vec<MountPoint>,
    next_mountid: u64,
}

/// Normalize a target path to the table's key shape: strip trailing '/'
/// (the root normalizes to "", matching devroot's root-chan path
/// convention — components append "/"+name from there).
fn normalize(path: &[u8]) -> Vec<u8> {
    let mut p = path;
    while p.len() > 1 && p.last() == Some(&b'/') {
        p = &p[..p.len() - 1];
    }
    if p == b"/" {
        return Vec::new();
    }
    p.to_vec()
}

impl Pgrp {
    pub fn new() -> Pgrp {
        Pgrp { mounts: Vec::new(), next_mountid: 1 }
    }

    /// bind/mount's table half (9k cmount, chan.c): validate the
    /// dir/file agreement, then insert per the order bits — MREPL
    /// replaces the chain; the first non-MREPL bind at a point adds the
    /// underlying directory to the chain; MBEFORE head-inserts; MAFTER
    /// appends.  Returns the new mount id.
    pub fn bind(&mut self, _from: Chan, _target: &Chan, _flag: u32) -> Result<u64> {
        Err("pgrp: bind unimplemented (RED skeleton)")
    }

    /// unmount(name, old) (9k cunmount): `from` None removes the whole
    /// mount point; Some(chan) removes only the chain entry whose mounted
    /// chan matches that identity.  Errors when nothing is mounted there
    /// or the named entry is not in the union.
    pub fn unmount(&mut self, _target_path: &[u8], _from: Option<&Chan>) -> Result<()> {
        Err("pgrp: unmount unimplemented (RED skeleton)")
    }

    /// pgrpcpy (9k pgrp.c): a deep copy of every mount point and chain
    /// entry — the RFNAMEG child starts with the parent's mounts and
    /// diverges from there.
    pub fn deep_copy(&self) -> Pgrp {
        Pgrp::new()
    }

    /// namec's mount consult: the ordered union chain at a walked path,
    /// if the path is a mount point.
    pub(crate) fn chain_at(&self, _path: &[u8]) -> Option<&[MountEntry]> {
        None
    }
}

impl Default for Pgrp {
    fn default() -> Pgrp {
        Pgrp::new()
    }
}
