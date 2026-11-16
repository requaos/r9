//! pgrp — the per-process namespace group: the mount table.
//!
//! Semantics ported literally from 9k's pgrp.c/chan.c (cmount's list
//! insertion, cunmount, pgrpcpy's deep copy): a mount point carries an
//! ORDERED union chain — MBEFORE binds (latest first), then the underlying
//! directory (present iff the first bind at the point was not MREPL), then
//! MAFTER binds (bind order); MREPL replaces the whole chain.  The frozen
//! M* flag values are re-exported from dat.rs.
//!
//! The mount point is keyed by the mount-point CHAN's IDENTITY — the
//! donor findmount/eqchanddq shape (chan.c: `eqchanddq(mh->from, dc,
//! devno, qid, 1)` over the MOUNTH(qid) hash): (dev, devno, qid.path).
//! A file reached THROUGH a union member is a DIFFERENT chan (its own
//! devno for a mounted tree, the server's qid) and never re-crosses a
//! mount bound over the underlying file at the same path.  Path-string
//! keying — the pre-19-10 shape — hijacked exactly that: rio's winshell
//! child resolved /dev/cons through the wsys union head, and the final
//! redirect re-consulted "/dev/cons" BY PATH, finding the cpu profile's
//! REPL bind of the terminal cons — the window's rc ran on the session
//! cons (the dual-cons-reader defect, live-probed 2026-11-16).
//! `Chan::devno` is the instance discriminator (devmnt stamps each 9P
//! session — the donor's mntalloc.id++).
//!
//! The group is shared state: Proc holds it as `Arc<Lock<Pgrp>>` (the arch
//! side's wiring), and the rfork flag semantics follow fork(2) — neither
//! RFNAMEG nor RFCNAMEG (with RFPROC) shares the handle, RFNAMEG
//! deep-copies (pgrpcpy), RFCNAMEG starts clean (the
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

/// A mount point (the donor's Mhead): the mount-point chan (identity
/// key + the chain's underlying member) plus its ordered union chain.
struct MountPoint {
    /// The mount-point chan — the donor's `mh->from`.  IDENTITY of the
    /// point (findmount's eqchanddq key: dev, devno, qid.path) and the
    /// underlying-directory member in one value.
    from: Chan,
    /// The union chain in 9k order (see the module header).
    chain: Vec<MountEntry>,
}

/// The per-process namespace group (the donor's Pgrp mount half — the env
/// half arrives with devenv in a later plan).
pub struct Pgrp {
    mounts: Vec<MountPoint>,
    next_mountid: u64,
}

/// Chan identity for mount-table matching (9k eqchanddq with skipvers:
/// dev, devno, qid.path — vers and the qid type byte are not identity).
fn same_point(a: &Chan, b: &Chan) -> bool {
    a.dev == b.dev && a.devno == b.devno && a.qid.path == b.qid.path
}

impl Pgrp {
    pub fn new() -> Pgrp {
        Pgrp { mounts: Vec::new(), next_mountid: 1 }
    }

    /// The donor closepgrp's mount half (the 19-10 cont-6 fix): take
    /// every mount-table chan OUT for close-dispatch.  The donor's
    /// closepgrp -> Mhead/Mount free -> cclose(m->to) chain ran at
    /// pgrp death; without it the mount ROOT fid (the ATTACH fid of
    /// every mounted tree — rio's wsys attach carries a window ref in
    /// xfidattach's `if(!newlymade) incref(w)`) never Tclunks: the
    /// ref leaks, the window's shell-exit clunk chain lands one short,
    /// and the window never deletes.  The CALLER close-chans the
    /// returned values outside the group lock (the leaf-lock rule) —
    /// closing can park (a Tclunk round trip), which the group lock
    /// must never span.
    pub fn take_mount_chans(&mut self) -> alloc::vec::Vec<Chan> {
        let mut out = alloc::vec::Vec::new();
        for m in self.mounts.drain(..) {
            out.push(m.from);
            for e in m.chain {
                out.push(e.to);
            }
        }
        out
    }

    /// bind/mount's table half (9k cmount, chan.c): validate the
    /// dir/file agreement, then insert per the order bits — MREPL
    /// replaces the chain; the first non-MREPL bind at a point adds the
    /// underlying directory to the chain; MBEFORE head-inserts; MAFTER
    /// appends.  Returns the new mount id.
    ///
    /// The point is found by the TARGET chan's identity (cmount's
    /// eqchan(mhead->from, old, 1) — never by path: two binds at the
    /// same path whose target walks land on different chans are two
    /// mount points, the donor's semantics).
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
        let id = self.next_mountid;
        self.next_mountid += 1;
        let entry = MountEntry { to: from, mflag: flag & MCREATE, mountid: id };
        match order {
            MREPL => {
                // cmount: mountfree the whole chain; the underlying
                // directory is NOT carried (a replaced point shadows
                // everything, including the dir it was bound over).
                let replaced = MountPoint { from: target.dup(), chain: alloc::vec![entry] };
                if let Some(i) = self.mounts.iter().position(|m| same_point(&m.from, target)) {
                    self.mounts[i] = replaced;
                } else {
                    self.mounts.push(replaced);
                }
            }
            MBEFORE => {
                if let Some(mp) = self.mounts.iter_mut().find(|m| same_point(&m.from, target)) {
                    // Head insertion (latest MBEFORE resolves first).
                    mp.chain.insert(0, entry);
                } else {
                    // First bind here: the underlying directory joins the
                    // union after the mounted chan (cmount's newmount(mhead,
                    // old, 0, 0)).
                    let underlying = MountEntry { to: target.dup(), mflag: 0, mountid: 0 };
                    self.mounts.push(MountPoint {
                        from: target.dup(),
                        chain: alloc::vec![entry, underlying],
                    });
                }
            }
            MAFTER => {
                if let Some(mp) = self.mounts.iter_mut().find(|m| same_point(&m.from, target)) {
                    mp.chain.push(entry);
                } else {
                    let underlying = MountEntry { to: target.dup(), mflag: 0, mountid: 0 };
                    self.mounts.push(MountPoint {
                        from: target.dup(),
                        chain: alloc::vec![underlying, entry],
                    });
                }
            }
            _ => return Err("bind: bad order bits"),
        }
        Ok(id)
    }

    /// unmount(name, old) (9k cunmount): `from` None removes the whole
    /// mount point; Some(chan) removes only the chain entry whose mounted
    /// chan matches that identity.  Errors when nothing is mounted at
    /// the target or the named entry is not in the union.
    pub fn unmount(&mut self, target: &Chan, from: Option<&Chan>) -> Result<()> {
        let Some(i) = self.mounts.iter().position(|m| same_point(&m.from, target)) else {
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
                let Some(j) = mp.chain.iter().position(|e| e.mountid != 0 && same_point(&e.to, c))
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
                    from: mp.from.dup(),
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

    /// namec's mount consult: the ordered union chain at a walked chan,
    /// if that chan is a mount point (findmount by identity — never by
    /// path; see the module header).
    pub fn chain_at(&self, c: &Chan) -> Option<&[MountEntry]> {
        self.mounts.iter().find(|m| same_point(&m.from, c)).map(|m| m.chain.as_slice())
    }

    /// A COPY of the union chain at `c` (the snapshot discipline):
    /// namec consults the mount table under the pgrp lock, drops the lock,
    /// then walks — the lock is never held across a device walk that can
    /// RPC (a devmnt Twalk blocks mid-syscall; a 9P server proc sharing
    /// this pgrp that takes the lock while serving would deadlock the
    /// system).  Chan copies ride dup() (the pipe open-end reference
    /// rule), deep_copy's shape.
    pub fn chain_snapshot(&self, c: &Chan) -> Option<Vec<MountEntry>> {
        self.chain_at(c).map(|chain| {
            chain
                .iter()
                .map(|e| MountEntry { to: e.to.dup(), mflag: e.mflag, mountid: e.mountid })
                .collect()
        })
    }
}

impl Default for Pgrp {
    fn default() -> Pgrp {
        Pgrp::new()
    }
}
