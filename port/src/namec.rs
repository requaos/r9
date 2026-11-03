//! namec — path resolution for r9 (05-04, D-03): the namespace core the
//! syscall layer routes through (the sys_open prefix routing is RETIRED
//! this plan — resolution lives here, once).
//!
//! Semantics ported from 9k's chan.c namec/walk (the RESEARCH
//! Don't-Hand-Roll mandate): a leading "#x" selects the device-registry
//! entry (a hand-built rune → attach-fn table — RESEARCH OQ-3, no
//! generated devtab this era) with mount crossing suppressed (nomount);
//! absolute paths walk from the proc's slash; the rest from its dot.  At
//! each directory the proc's mount table is consulted: a mount-point
//! crossing redirects the walk into the mounted chan's tree, and a failed
//! component walk falls through the union in chain order (9k's
//! domount + union fallback).  The final component redirects for
//! open/stat-style resolution (Aopen) but NOT for bind targets (Amount —
//! "subsequent mounts attach to the original directory") or chdir
//! (Atodir).
//!
//! Errors are "namec: …"-prefixed static strings; device-layer errors
//! propagate with their own "devX: …" prefixes.  The dat.rs `Dev` trait
//! is NOT implemented here (its skeleton is non-constructible — see
//! devroot.rs's header); the registry is a plain match over attach fns,
//! the established free-function device idiom.

use crate::dat::{Chan, DEV_CONS, DEV_PROC, DEV_ROOT};
use crate::pgrp::Pgrp;
use crate::Result;

/// What the caller intends — the donor namec's amode decides whether the
/// FINAL component crosses a mount point (chan.c: Aopen/Abind domount,
/// Amount/Atodir do not).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NamecMode {
    /// open/stat-style resolution (donor Aopen/Aaccess): redirect the
    /// final component to the union head.
    Open,
    /// bind's SOURCE (donor Abind): a source that is itself covered by a
    /// mount binds the union head.
    BindSource,
    /// bind's TARGET (donor Amount): never redirect — a stacked bind
    /// attaches to the original directory, not the replacement.
    MountTarget,
    /// chdir (donor Atodir): directories stay before the mount point.
    ToDir,
}

/// Resolve `path` to a Chan.  RED-phase skeleton: the GREEN phase lands
/// the registry, the component walk, and the union machinery.
pub fn namec(
    _path: &[u8],
    _slash: &Chan,
    _dot: &Chan,
    _pg: &Pgrp,
    _mode: NamecMode,
) -> Result<Chan> {
    Err("namec: unimplemented (RED skeleton)")
}

/// The open dispatch for a resolved chan (per-device open by dev index —
/// the registry's twin; sys_open's mode checks live inside each device's
/// open).  RED-phase skeleton.
pub fn open_chan(_c: &Chan, _mode: u32) -> Result<Chan> {
    Err("namec: unimplemented (RED skeleton)")
}

/// Silence unused-import warnings in the RED skeleton (the GREEN phase's
/// dispatch uses all four).
#[allow(unused)]
fn _red_skeleton_touch(c: &Chan) {
    let _ = (DEV_ROOT, DEV_CONS, DEV_PROC);
    let _ = c;
}
