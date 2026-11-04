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

use crate::dat::{Chan, DEV_CONS, DEV_DUP, DEV_ENV, DEV_MNT, DEV_PIPE, DEV_PROC, DEV_ROOT, DEV_SRV};
use crate::mcslock::{Lock, LockNode};
use crate::pgrp::Pgrp;
use crate::Result;

/// Plan 9 Qid type byte (sys/include/libc.h).
const QTDIR: u8 = 0x80;

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

/// The device registry (05-04, D-03; RESEARCH OQ-3's hand-built shape):
/// '#' + rune → the device's attach fn.  The spec (the bytes between the
/// rune and the next '/') is accepted and ignored by this era's devices
/// (devcons/devproc serve one tree each — devroot's attach precedent);
/// devenv (05-05) is the exception: it VALIDATES the spec (empty only —
/// a "#ec" attach names the donor's config group, which the seeded root
/// group replaces; the loud error is boot's designed fail-soft path).
/// devroot is NOT a registry member: absolute paths reach it through the
/// proc's slash chan (the donor's slash is the boot root attach).
fn registry_attach(rune: char, spec: &[u8]) -> Result<Chan> {
    match rune {
        'c' => Ok(crate::devcons::attach()),
        'e' => crate::devenv::attach(spec),
        'p' => Ok(crate::devproc::attach()),
        // 05-06: a FRESH pipe pair per attach (9k's pipeattach) — the
        // sys_pipe handler's entry point.
        '|' => Ok(crate::devpipe::attach()),
        // 05-06: '#d' per-fd files (rcmain's interactive `. -i '#d/0'`)
        // and the '#s' service registry (boot's srvcreate).
        'd' => Ok(crate::devdup::attach()),
        's' => Ok(crate::devsrv::attach()),
        _ => Err("namec: unknown # device"),
    }
}

/// UTF-8 decode of the first rune (9k chartorune) — device chars include
/// multibyte runes ('æ' in "#æ" is two bytes; boot's fail-soft binds need
/// a clean unknown-device error, never a mis-parse).
fn first_rune(b: &[u8]) -> Result<(char, usize)> {
    let Some(&b0) = b.first() else { return Err("namec: empty # device") };
    let width = if b0 < 0x80 {
        1
    } else if b0 >= 0xF0 {
        4
    } else if b0 >= 0xE0 {
        3
    } else if b0 >= 0xC0 {
        2
    } else {
        return Err("namec: bad # device encoding");
    };
    if b.len() < width {
        return Err("namec: bad # device encoding");
    }
    let s = core::str::from_utf8(&b[..width]).map_err(|_| "namec: bad # device encoding")?;
    Ok((s.chars().next().unwrap(), width))
}

/// The per-device walk dispatch (the registry's twin): a resolved chan
/// knows its device by index.  devroot's walk needs the mounted-image
/// handle (stateless-per-call over the published image).
fn dev_walk(c: &Chan, comp: &[u8]) -> Result<Chan> {
    match c.dev {
        DEV_ROOT => crate::devroot::root().ok_or("namec: devroot not mounted")?.walk(c, comp),
        DEV_CONS => crate::devcons::walk(c, comp),
        DEV_PROC => crate::devproc::walk(c, comp),
        DEV_ENV => crate::devenv::walk(c, comp),
        DEV_PIPE => crate::devpipe::walk(c, comp),
        DEV_DUP => crate::devdup::walk(c, comp),
        DEV_SRV => crate::devsrv::walk(c, comp),
        // 05-07: a walk into a mounted tree is a Twalk RPC (devmnt — the
        // mount entry's `to` chan carries the session + the attach fid).
        DEV_MNT => crate::devmnt::walk(c, comp),
        _ => Err("namec: chan with unknown device index"),
    }
}

/// Resolve `path` to a Chan.
///
/// LOCKING (WR-04): the mount-table consults take the pgrp lock PER
/// CONSULT (snapshot the union chain via chain_snapshot, drop the lock,
/// then walk) — the lock is NEVER held across a device walk.  A walk into
/// a mounted tree is a devmnt Twalk RPC that BLOCKS mid-syscall (the pipe
/// read schedules the server proc); holding the pgrp lock across it
/// deadlocks the moment any 9P server proc sharing this pgrp (a fork()
/// child without RFNAMEG) takes the lock while serving.  This is
/// sys_mount's documented discipline ("NO pgrp lock is held across the
/// RPC") applied to resolution itself.
///
/// FID SWEEP (261104-b4p, item 5): every DEV_MNT chan this walk MINTS and
/// walks PAST (walk-step results + redirect clones) collects in `swept`
/// and is devmnt::clunk_if_last'd on EVERY exit — Ok and Err alike.  The
/// Arc count is the second line of the discipline: the entry dup
/// (Arc-shares the caller's slash/dot fid), chain-snapshot clones (the
/// mount table's fid), and the RETURNED chan are structurally never
/// clunked here (the count is never 1 for the shared cases; the returned
/// chan is not in the sweep).
pub fn namec(path: &[u8], slash: &Chan, dot: &Chan, pg: &Lock<Pgrp>, mode: NamecMode) -> Result<Chan> {
    if path.is_empty() {
        return Err("namec: empty file name");
    }
    let mut nomount = false;
    let (mut c, rest): (Chan, &[u8]) = match path[0] {
        b'#' => {
            nomount = true;
            let (rune, w) = first_rune(&path[1..])?;
            let after = &path[1 + w..];
            let (spec, rest) = match after.iter().position(|&b| b == b'/') {
                Some(i) => (&after[..i], &after[i + 1..]),
                None => (after, &b""[..]),
            };
            (registry_attach(rune, spec)?, rest)
        }
        b'/' => (slash.dup(), &path[1..]),
        _ => (dot.dup(), path),
    };

    let mut swept: alloc::vec::Vec<Chan> = alloc::vec::Vec::new();
    let result = namec_walk(&mut c, rest, nomount, pg, mode, &mut swept);
    for s in swept.drain(..) {
        crate::devmnt::clunk_if_last(&s);
    }
    match result {
        Ok(()) => Ok(c),
        Err(msg) => {
            // The in-flight chan never reaches the caller — sweep it too
            // (clunk_if_last's count check protects the entry dup).
            crate::devmnt::clunk_if_last(&c);
            Err(msg)
        }
    }
}

/// The walk loop + final-component redirect (namec's body, factored so
/// the fid sweep has ONE exit tail).  `c` is mutated in place; every
/// walked-past DEV_MNT chan is pushed to `swept` (clunked by the caller
/// on any exit).
fn namec_walk(
    c: &mut Chan,
    rest: &[u8],
    nomount: bool,
    pg: &Lock<Pgrp>,
    mode: NamecMode,
    swept: &mut alloc::vec::Vec<Chan>,
) -> Result<()> {
    // Walk the components (9k walk's loop): before each device walk,
    // cross a mount point if the current chan sits on one — then try the
    // component in the union chain's order (the crossing's head first, the
    // underlying dir in chain position, MAFTER tails last).  WR-04: the
    // chain is SNAPSHOTED under a per-consult lock; the walks below run
    // lock-free.
    for comp in rest.split(|&b| b == b'/').filter(|c| !c.is_empty()) {
        if c.qid.typ & QTDIR == 0 {
            return Err("namec: walk of a non-directory");
        }
        let chain = if nomount {
            None
        } else {
            let node = LockNode::new();
            let guard = pg.lock(&node);
            guard.chain_snapshot(&c.path)
        };
        // 05-09: "." is the devwalk Accept arm (9k dev.c devwalk:
        // `strcmp(n, ".") == 0` keeps the current qid) — AFTER the mount
        // consultation, so "x/." (and ps's chdir("/proc") + open("."))
        // resolves to the UNION HEAD (the devproc attach), not the
        // underlying dir.  ".." stays unhandled (no measured caller this
        // era — the device walk errors loudly).
        if comp == b"." {
            if let Some(chain) = chain {
                let Some(head) = chain.first() else {
                    return Err("namec: empty union chain");
                };
                // 05-09: DEV_MNT heads are fid-CLONED, never bare-dup'd —
                // the redirect's open/close lifecycle must not touch the
                // mount table's server-side fid (devmnt::clone_chan).
                let mut nc = redirect_clone(&head.to)?;
                nc.path = c.path.clone();
                let old = core::mem::replace(c, nc);
                if old.dev == DEV_MNT {
                    swept.push(old);
                }
            }
            continue;
        }
        if let Some(chain) = chain {
            let mut walked = None;
            for entry in &chain {
                // Re-root the chain entry's path at the mount point so the
                // walked chan carries the caller-visible path (fd2path's
                // future input; deeper mount points match on it).
                let mut base = entry.to.dup();
                base.path = c.path.clone();
                if let Ok(nc) = dev_walk(&base, comp) {
                    walked = Some(nc);
                    break;
                }
            }
            let Some(nc) = walked else {
                return Err("namec: does not exist");
            };
            let old = core::mem::replace(c, nc);
            if old.dev == DEV_MNT {
                swept.push(old);
            }
            continue;
        }
        match dev_walk(c, comp) {
            Ok(nc) => {
                let old = core::mem::replace(c, nc);
                if old.dev == DEV_MNT {
                    swept.push(old);
                }
            }
            Err(_) => return Err("namec: does not exist"),
        }
    }

    // The final component's mount crossing (chan.c's namec tail):
    // open/stat-style resolution lands on the union head; bind targets and
    // chdir stay before the mount point.  05-09: a DEV_MNT head is
    // fid-CLONED (never bare-dup'd) — the redirect's open/close lifecycle
    // must not touch the mount table's server-side fid (the gate-surfaced
    // aliasing fix: ls /bin's open+close Tclunked the table's fid under a
    // bare dup, and every later walk from it Rerror'd).  WR-04: the
    // consult snapshots under a per-consult lock, then drops it.
    if !nomount
        && mode != NamecMode::MountTarget
        && mode != NamecMode::ToDir
    {
        let chain = {
            let node = LockNode::new();
            let guard = pg.lock(&node);
            guard.chain_snapshot(&c.path)
        };
        if let Some(chain) = chain {
            let Some(head) = chain.first() else {
                return Err("namec: empty union chain");
            };
            let mut nc = redirect_clone(&head.to)?;
            nc.path = c.path.clone();
            let old = core::mem::replace(c, nc);
            if old.dev == DEV_MNT {
                swept.push(old);
            }
        }
    }
    Ok(())
}

/// The mount-redirect chan (05-09): a DEV_MNT head is fid-cloned (its own
/// server-side fid — devmnt::clone_chan, the zero-name Twalk idiom); every
/// other device value-dups as before.  pub for sys_create's dot redirect.
pub fn redirect_clone(c: &Chan) -> Result<Chan> {
    if c.dev == DEV_MNT {
        crate::devmnt::clone_chan(c)
    } else {
        Ok(c.dup())
    }
}

/// A single-component walk from an already-resolved chan through the
/// device dispatch (05-06: sys_create's final-component probe).  No
/// mount-point consultation — the caller's parent already resolved with
/// the union semantics it needed.
pub fn walk_one(c: &Chan, name: &[u8]) -> Result<Chan> {
    dev_walk(c, name)
}

/// The open dispatch for a resolved chan (per-device open by dev index —
/// the registry's twin).  Each device's open enforces its own mode
/// discipline (devroot rejects write-side modes and OTRUNC; the flag bits
/// beyond the access mode are the device's business per 9k convention).
/// devenv gets the FULL mode (its open honors OTRUNC — the setenv flow's
/// create-on-existing → OTRUNC-open fallback per 9k chan.c Acreate).
pub fn open_chan(c: &Chan, mode: u32) -> Result<Chan> {
    let access = mode & crate::dat::OMASK;
    let mut opened = match c.dev {
        DEV_ROOT => {
            if mode & crate::dat::OTRUNC != 0 {
                return Err("devroot: read-only filesystem");
            }
            crate::devroot::root().ok_or("namec: devroot not mounted")?.open(c, access)?
        }
        DEV_CONS => crate::devcons::open(c, access)?,
        DEV_PROC => crate::devproc::open(c, access)?,
        DEV_ENV => crate::devenv::open(c, mode)?,
        DEV_PIPE => crate::devpipe::open(c, access)?,
        DEV_DUP => crate::devdup::open(c, access)?,
        DEV_SRV => crate::devsrv::open(c, mode)?,
        // 05-07: Topen over the session (devmnt gets the FULL mode — it
        // masks the wire byte itself, OTRUNC included for the server to
        // refuse).
        DEV_MNT => crate::devmnt::open(c, mode)?,
        _ => return Err("namec: chan with unknown device index"),
    };
    // 05-06: the donor's COPEN — every funnel-opened chan is marked;
    // Chan::dup's pipe-reference rule and close_chan's dispatch read it.
    opened.copen = true;
    Ok(opened)
}

/// The stat dispatch for a resolved chan (05-04 Task 3's sys_stat/sys_fstat
/// ride this): per-device stat by dev index.  Returns the stat(5) wire
/// byte count (the size prefix included — convD2M's contract).
pub fn stat_chan(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    match c.dev {
        DEV_ROOT => crate::devroot::root().ok_or("namec: devroot not mounted")?.stat(c, sb),
        DEV_CONS => crate::devcons::stat(c, sb),
        DEV_PROC => crate::devproc::stat(c, sb),
        DEV_ENV => crate::devenv::stat(c, sb),
        DEV_PIPE => crate::devpipe::stat(c, sb),
        DEV_DUP => crate::devdup::stat(c, sb),
        DEV_SRV => crate::devsrv::stat(c, sb),
        // 05-07: Tstat over the session (settime's stat of the /tmp mount).
        DEV_MNT => crate::devmnt::stat(c, sb),
        _ => Err("namec: chan with unknown device index"),
    }
}

/// The create dispatch (05-06's sys_create — the donor chan.c Acreate's
/// device half): per-device create by dev index, returning the new chan
/// OPEN (copen marked here — the open funnel's twin).  Devices without a
/// create op error loudly (never a silent no-op); devenv (env keys) and
/// devsrv (service names) are this phase's two creators.
pub fn create_chan(c: &Chan, name: &[u8], mode: u32, perm: u32) -> Result<Chan> {
    let mut created = match c.dev {
        DEV_ENV => crate::devenv::create(c, name, mode, perm)?,
        DEV_SRV => crate::devsrv::create(c, name, mode, perm)?,
        // 05-07: the mounted tree is a read-only server this era (paqfs's
        // rcreate would refuse regardless) — loud, never a silent no-op.
        DEV_MNT => return Err("devmnt: create unsupported (read-only server this era)"),
        _ => return Err("namec: device has no create"),
    };
    created.copen = true;
    Ok(created)
}

/// The close dispatch (05-06 — the donor's devclose): called when an
/// open chan LEAVES an fd slot (sys_close's pop, sys_dup's overwrite
/// eviction, sys_pipe/sys_open's error paths) — and, 261104-b4p (item
/// 5), whenever a WALKED-but-never-installed DEV_MNT chan leaves scope
/// (namec's walk sweep, the syscall callers' walked-past chans).  Only
/// devices with close semantics act: devpipe decrements the end's open
/// count — COPEN-gated (the last close hangs the peer's read queue up —
/// the EOF path the connectembed teardown rides); devmnt Tclunks the fid
/// via clunk_if_last REGARDLESS of copen (05-07 — best-effort, never
/// fails; walked-unopened chans hold fids too, and the Arc count makes
/// non-last refs a no-op).  Non-open chans of close-less devices are
/// no-ops.
pub fn close_chan(c: &Chan) {
    match c.dev {
        DEV_PIPE => {
            if c.copen {
                crate::devpipe::close(c);
            }
        }
        DEV_MNT => crate::devmnt::clunk_if_last(c),
        _ => {}
    }
}
