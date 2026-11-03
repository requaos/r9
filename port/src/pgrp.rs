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

/// Chan identity for unmount matching (9k eqchan, skipvers): the device
/// index plus qid path and type.  Path strings are NOT identity (two names
/// can reach one file through a union).
fn same_chan(a: &Chan, b: &Chan) -> bool {
    a.dev == b.dev && a.qid.path == b.qid.path && a.qid.typ == b.qid.typ
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
    ///
    /// The target must already exist (the caller resolves it via namec
    /// first — Pitfall-8: MCREATE permits later creation IN the mounted
    /// directory; it never conjures the mount target).
    pub fn bind(&mut self, from: Chan, target: &Chan, flag: u32) -> Result<u64> {
        let order = flag & MORDER;
        let from_dir = from.qid.typ & QTDIR != 0;
        let target_dir = target.qid.typ & QTDIR != 0;
        if from_dir != target_dir {
            return Err("bind: mount dir/file mismatch");
        }
        if !target_dir && order != MREPL {
            return Err("bind: union mount of a non-directory");
        }
        let key = normalize(&target.path);
        let id = self.next_mountid;
        self.next_mountid += 1;
        let entry = MountEntry { to: from, mflag: flag & MCREATE, mountid: id };
        match order {
            MREPL => {
                // cmount: mountfree the whole chain; the underlying
                // directory is NOT carried (a replaced point shadows
                // everything, including the dir it was bound over).
                let replaced = MountPoint { target: key, chain: alloc::vec![entry] };
                if let Some(i) = self.mounts.iter().position(|m| m.target == replaced.target) {
                    self.mounts[i] = replaced;
                } else {
                    self.mounts.push(replaced);
                }
            }
            MBEFORE => {
                if let Some(mp) = self.mounts.iter_mut().find(|m| m.target == key) {
                    // Head insertion (latest MBEFORE resolves first).
                    mp.chain.insert(0, entry);
                } else {
                    // First bind here: the underlying directory joins the
                    // union after the mounted chan (cmount's newmount(mhead,
                    // old, 0, 0)).
                    let underlying =
                        MountEntry { to: target.dup(), mflag: 0, mountid: 0 };
                    self.mounts.push(MountPoint { target: key, chain: alloc::vec![entry, underlying] });
                }
            }
            MAFTER => {
                if let Some(mp) = self.mounts.iter_mut().find(|m| m.target == key) {
                    mp.chain.push(entry);
                } else {
                    let underlying =
                        MountEntry { to: target.dup(), mflag: 0, mountid: 0 };
                    self.mounts.push(MountPoint { target: key, chain: alloc::vec![underlying, entry] });
                }
            }
            _ => return Err("bind: bad order bits"),
        }
        Ok(id)
    }

    /// unmount(name, old) (9k cunmount): `from` None removes the whole
    /// mount point; Some(chan) removes only the chain entry whose mounted
    /// chan matches that identity.  Errors when nothing is mounted there
    /// or the named entry is not in the union.
    pub fn unmount(&mut self, target_path: &[u8], from: Option<&Chan>) -> Result<()> {
        let key = normalize(target_path);
        let Some(i) = self.mounts.iter().position(|m| m.target == key) else {
            return Err("unmount: not a mount point");
        };
        match from {
            None => {
                self.mounts.remove(i);
                Ok(())
            }
            Some(c) => {
                let mp = &mut self.mounts[i];
                // The underlying-directory chain entry (mountid 0) is not
                // removable by name — cunmount removes mounted chans.
                let Some(j) = mp.chain.iter().position(|e| e.mountid != 0 && same_chan(&e.to, c))
                else {
                    return Err("unmount: name not in union");
                };
                mp.chain.remove(j);
                if mp.chain.is_empty() {
                    self.mounts.remove(i);
                }
                Ok(())
            }
        }
    }

    /// pgrpcpy (9k pgrp.c): a deep copy of every mount point and chain
    /// entry — the RFNAMEG child starts with the parent's mounts and
    /// diverges from there.  (The donor preserves mountid allocation order
    /// across the copy; chain ORDER here is positional, so a field-wise
    /// copy preserves the semantics — ids carry forward and the counter
    /// continues from the parent's next.)
    pub fn deep_copy(&self) -> Pgrp {
        Pgrp {
            mounts: self
                .mounts
                .iter()
                .map(|mp| MountPoint {
                    target: mp.target.clone(),
                    chain: mp
                        .chain
                        .iter()
                        .map(|e| MountEntry { to: e.to.dup(), mflag: e.mflag, mountid: e.mountid })
                        .collect(),
                })
                .collect(),
            next_mountid: self.next_mountid,
        }
    }

    /// namec's mount consult: the ordered union chain at a walked path,
    /// if the path is a mount point.
    pub(crate) fn chain_at(&self, path: &[u8]) -> Option<&[MountEntry]> {
        let key = normalize(path);
        self.mounts.iter().find(|m| m.target == key).map(|m| m.chain.as_slice())
    }
}

impl Default for Pgrp {
    fn default() -> Pgrp {
        Pgrp::new()
    }
}
