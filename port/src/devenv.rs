//! devenv — the per-pgrp environment device `#e` (05-05, D-07; semantics
//! ported from 9k's devenv.c — the RESEARCH Don't-Hand-Roll mandate).
//!
//! The environment is a per-process-GROUP key/value store: every `#e`
//! operation resolves against the OPERATING process's env group (the
//! donor's `envgrp(c)` → `up->egrp` shape — dynamic, never captured at
//! attach, so a chan walked by one proc resolves in the group of the
//! proc using it).  rfork semantics follow fork(2) and 9k's sysproc.c
//! (the 05-04 RFNAMEG/RFCNAMEG precedent — the donor governs): RFPROC
//! with NEITHER RFENVG nor RFCENVG SHARES the group (Arc clone);
//! RFENVG gives the child a fresh deep COPY (envcpy); RFCENVG a fresh
//! EMPTY group.  The kernel seeds the ROOT group before the first exec
//! (cputype/nobootprompt/readparts — RESEARCH Pattern 3 / Pitfall 1);
//! main.rs assigns it to pid 1 and prints the `devenv: seeded …` gate
//! line.
//!
//! The group lives behind an mcslock (`Arc<Lock<EnvGroup>>` — the pgrp
//! field's exact shape); the lock is a LEAF: taken only from syscall
//! context, briefly, never from IRQ context, and never held while
//! taking another kernel lock.  The port crate cannot see the arch
//! side's proc table, so the current group arrives through a
//! dependency-injected hook (`set_group_hook` — the devproc.rs
//! `set_post_hook` idiom; PATTERNS names the same one-time-publish
//! shape for the seeded root group).
//!
//! The dat.rs `Dev` trait is NOT implemented here (its skeleton is
//! non-constructible — see devroot.rs's header); the device operations
//! (attach/walk/open/read/write/create/stat) are free functions, and
//! trait conformance lands with the devtab dispatch era.
//!
//! Qid scheme (collision-free): the root dir is qid path 0 (QTDIR); a
//! key's qid path is its per-group creation sequence from 1 (the
//! donor's `++eg->path`), QTFILE.  A write/trunc bumps the entry's
//! qid.vers (donor `e->qid.vers++`); the root stat serves the group's
//! vers.  deep_copy RENUMBERS (donor envcpy: `ne->qid.path = ++to->path`
//! — the copy's entries get fresh 1..n paths in insertion order, vers
//! 0), so a chan carried across an RFENVG fork resolves the copy's
//! k-th entry — donor-exact.
//!
//! Attach spec: empty only.  A non-empty spec (e.g. "c" — the donor's
//! `#ec` config group) errors loudly: r9's seeded ROOT group replaces
//! the config-env mechanism this era (Pattern 3), and boot's
//! `bind("#ec", …)` is the designed fail-soft path (05-04 proved it).
//!
//! The `#e` tree is FLAT: root dir + key files.  create rides the
//! sys_create handler (05-06 wires the syscall; this plan's gate needs
//! only seeded keys + write/replace — the ops are structured so create
//! slots in without rework: the strict Eexist-on-dup device op here is
//! the donor's envcreate, and namec's Acreate existing-file → OTRUNC
//! open fallback (9k chan.c) is the syscall side's business, not this
//! module's).

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_ENV, OREAD, OTRUNC};
use crate::mcslock::{Lock, LockNode};
use crate::Result;

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// Donor Maxenvsize (9k devenv.c): a value's offset+write extent never
/// crosses this bound (Etoobig).
pub const MAXENVSIZE: u64 = 16300;

/// The donor's name ceiling (9k envcreate's `sizeof up->genbuf` = 128):
/// names are shorter than 128 bytes.
const MAX_ENV_NAME: usize = 127;

/// One env variable (the donor's Evalue): name, value bytes, the
/// per-group qid path, and the entry version (bumped per write/trunc).
pub struct EnvEntry {
    pub name: Vec<u8>,
    pub value: Vec<u8>,
    pub qid_path: u64,
    pub vers: u32,
}

/// The per-process-group environment (the donor's Egrp): insertion-ordered
/// entries, the qid-path sequence, and the group version (bumped per
/// create/write/trunc — served as the root dir's qid.vers).
pub struct EnvGroup {
    entries: Vec<EnvEntry>,
    next_qid: u64,
    vers: u32,
}

/// The arch-side current-group hook signature: the OPERATING process's
/// env-group handle (None before the first proc exists — no `#e` op can
/// happen then).
type GroupHook = fn() -> Option<Arc<Lock<EnvGroup>>>;

/// The current-group hook.  Registered once by the arch side before the
/// first exec (the devproc.rs `set_post_hook` idiom — single writer,
/// pre-EL0; read from syscall context thereafter).
static GROUP_HOOK: Lock<Option<GroupHook>> = Lock::new("devenv", None);

/// The seeded root group, published once at boot (the one-time-publish
/// half of the DI idiom — PATTERNS).  pid 1 runs with this group; every
/// later proc derives its group via rfork.
static ROOT_GROUP: Lock<Option<Arc<Lock<EnvGroup>>>> = Lock::new("devenvroot", None);

/// Register the arch-side current-group hook.  Called once during boot
/// before any user process exists; a second registration is a boot bug.
pub fn set_group_hook(hook: GroupHook) {
    let node = LockNode::new();
    let mut guard = GROUP_HOOK.lock(&node);
    assert!(guard.is_none(), "devenv: group hook registered twice");
    *guard = Some(hook);
}

/// The current process's env group via the registered hook (the donor's
/// `up->egrp` at every op).  Err when no hook is registered or no proc
/// exists (a pre-exec `#e` op is impossible — a caller is a boot bug).
fn current_group() -> Result<Arc<Lock<EnvGroup>>> {
    let hook = {
        let node = LockNode::new();
        *GROUP_HOOK.lock(&node)
    };
    let Some(hook) = hook else {
        return Err("devenv: group hook not registered");
    };
    hook().ok_or("devenv: no current env group")
}

/// The published root group (None before main.rs seeds it).
pub fn root_group() -> Option<Arc<Lock<EnvGroup>>> {
    let node = LockNode::new();
    ROOT_GROUP.lock(&node).clone()
}

impl EnvGroup {
    /// A fresh, empty group (the RFCENVG arm).
    pub fn new() -> EnvGroup {
        EnvGroup { entries: Vec::new(), next_qid: 0, vers: 0 }
    }

    /// A group pre-populated with `pairs` (create + write per pair — the
    /// seeded-root constructor; 05-05's kernel seed and the host tests).
    pub fn seeded(pairs: &[(&[u8], &[u8])]) -> Result<EnvGroup> {
        let mut g = EnvGroup::new();
        for (name, value) in pairs {
            let qid = g.create(name)?;
            let wrote = g.write(qid, value, 0)?;
            assert_eq!(wrote as usize, value.len(), "devenv: short seed write");
        }
        Ok(g)
    }

    /// The donor envlookup by NAME.
    pub fn get(&self, name: &[u8]) -> Option<&EnvEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The donor envlookup by QID PATH (the chan-facing identity).
    pub fn get_qid(&self, qid_path: u64) -> Option<&EnvEntry> {
        self.entries.iter().find(|e| e.qid_path == qid_path)
    }

    /// The group version (the root dir's qid.vers — donor eg->vers).
    pub fn vers(&self) -> u32 {
        self.vers
    }

    /// The entries in insertion order (the directory listing's source).
    pub fn entries(&self) -> &[EnvEntry] {
        &self.entries
    }

    /// envcreate: a new EMPTY key.  Eexist on a duplicate name (the
    /// create-on-existing → OTRUNC-open fallback is namec's Acreate
    /// contract, 05-06's sys_create — not here).  Names are a single
    /// component: non-empty, no '/', under the donor's genbuf ceiling.
    /// Returns the new entry's qid path.
    pub fn create(&mut self, name: &[u8]) -> Result<u64> {
        if name.is_empty() || name.contains(&b'/') {
            return Err("devenv: bad name (single component)");
        }
        if name.len() > MAX_ENV_NAME {
            return Err("devenv: name too long");
        }
        if self.get(name).is_some() {
            return Err("devenv: name exists");
        }
        self.next_qid += 1;
        let qid = self.next_qid;
        self.vers += 1;
        self.entries
            .push(EnvEntry { name: name.to_vec(), value: Vec::new(), qid_path: qid, vers: 0 });
        Ok(qid)
    }

    /// envwrite: overwrite `buf` at `offset`, growing the value (zero-gap)
    /// when the extent passes the current length; bounded by MAXENVSIZE
    /// (Etoobig).  A zero-length write is a 0 no-op.
    pub fn write(&mut self, qid_path: u64, buf: &[u8], offset: u64) -> Result<u64> {
        if buf.is_empty() {
            return Ok(0);
        }
        if offset > MAXENVSIZE || buf.len() as u64 > MAXENVSIZE - offset {
            return Err("devenv: value too big");
        }
        let Some(e) = self.entries.iter_mut().find(|e| e.qid_path == qid_path) else {
            return Err("devenv: nonexistent entry");
        };
        let end = offset as usize + buf.len();
        if end > e.value.len() {
            // Donor smalloc zero-fills the gap between the old end and
            // the write offset.
            e.value.resize(end, 0);
        }
        e.value[offset as usize..end].copy_from_slice(buf);
        e.vers += 1;
        self.vers += 1;
        Ok(buf.len() as u64)
    }

    /// envread's byte half: clamp at the value length; a read at or past
    /// the end is EOF = 0 (never an error).
    pub fn read(&self, qid_path: u64, buf: &mut [u8], offset: u64) -> Result<u64> {
        let Some(e) = self.get_qid(qid_path) else {
            return Err("devenv: nonexistent entry");
        };
        let start = (offset as usize).min(e.value.len());
        let avail = &e.value[start..];
        let n = avail.len().min(buf.len());
        buf[..n].copy_from_slice(&avail[..n]);
        Ok(n as u64)
    }

    /// envopen's OTRUNC arm: empty the value (the entry vers bump is the
    /// donor's; the group vers is untouched — envopen's trunc arm bumps
    /// only e->qid.vers).
    pub fn trunc(&mut self, qid_path: u64) -> Result<()> {
        let Some(e) = self.entries.iter_mut().find(|e| e.qid_path == qid_path) else {
            return Err("devenv: nonexistent entry");
        };
        e.value = Vec::new();
        e.vers += 1;
        Ok(())
    }

    /// envcpy: a deep copy — same names/values, FRESH qid paths (the
    /// donor renumbers: `ne->qid.path = ++to->path`), entry vers 0.
    pub fn deep_copy(&self) -> EnvGroup {
        let mut to = EnvGroup::new();
        for e in &self.entries {
            to.next_qid += 1;
            to.entries.push(EnvEntry {
                name: e.name.clone(),
                value: e.value.clone(),
                qid_path: to.next_qid,
                vers: 0,
            });
        }
        to
    }
}

impl Default for EnvGroup {
    fn default() -> EnvGroup {
        EnvGroup::new()
    }
}

/// Seed the ROOT env group and publish it (one-time; a double seed is a
/// boot bug).  Returns the published handle for pid 1's assignment.
pub fn seed_root_group(pairs: &[(&[u8], &[u8])]) -> Arc<Lock<EnvGroup>> {
    let seeded =
        EnvGroup::seeded(pairs).unwrap_or_else(|err| panic!("devenv: root seed failed: {err}"));
    let arc = Arc::new(Lock::new("env", seeded));
    let node = LockNode::new();
    let mut guard = ROOT_GROUP.lock(&node);
    assert!(guard.is_none(), "devenv: root group seeded twice");
    *guard = Some(arc.clone());
    arc
}

/// attach: the env root dir chan.  The group is NOT captured — every op
/// resolves the OPERATING process's group (the donor's envgrp(c)).  The
/// spec must be empty (the `#ec` config-group mechanism is replaced by
/// the seeded root group this era — a non-empty spec errors loudly and
/// boot's `bind("#ec", …)` fails soft by design).
pub fn attach(spec: &[u8]) -> Result<Chan> {
    if !spec.is_empty() {
        return Err("devenv: unknown attach spec (no #ec this era)");
    }
    Ok(Chan::new(DEV_ENV, Qid { path: 0, vers: 0, typ: QTDIR }, 0))
}

/// Walk a single name component from the env root to a key (the donor's
/// envwalk/envgen: unknown names are walk errors — boot's fail-soft
/// readfile contract hangs on this).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR || c.qid.path != 0 {
        return Err("devenv: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devenv: walk takes a single name component");
    }
    let group = current_group()?;
    let node = LockNode::new();
    let guard = group.lock(&node);
    let Some(e) = guard.get(name) else {
        return Err("devenv: name not found");
    };
    let mut nc = Chan::new(DEV_ENV, Qid { path: e.qid_path, vers: e.vers, typ: QTFILE }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open an env chan (donor envopen): the root dir opens OREAD only; a
/// key opens in any access mode after an existence check (the donor's
/// Enonexist arm — a walked chan whose key vanished errors here); OTRUNC
/// empties the value first (the setenv flow — create-on-existing rides
/// namec's Acreate → OTRUNC open fallback, 05-06).  r9 has no eve
/// privilege split this era: every group is writable (the donor's
/// envwriteable `iseve() || c->aux == nil` — aux is always nil here).
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let access = mode & crate::dat::OMASK;
    if c.qid.typ == QTDIR {
        if access != OREAD {
            return Err("devenv: directory opens OREAD only");
        }
    } else {
        let group = current_group()?;
        let node = LockNode::new();
        let mut guard = group.lock(&node);
        if guard.get_qid(c.qid.path).is_none() {
            return Err("devenv: nonexistent entry");
        }
        if mode & OTRUNC != 0 {
            guard.trunc(c.qid.path)?;
        }
    }
    let mut nc = Chan::new(c.dev, c.qid, access);
    nc.path = c.path.clone();
    Ok(nc)
}

/// Read an env chan.  The root dir serves the stat(5)-wire listing of the
/// current group's entries (insertion order — the devcons root-listing
/// shape); a key serves its value bytes (donor envread: clamped at the
/// value length, EOF = 0).  `offset` u64::MAX uses + advances the chan's
/// streaming offset (the pread convention).
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    if c.mode == crate::dat::OWRITE {
        return Err("devenv: file not open for reading");
    }
    let group = current_group()?;
    let node = LockNode::new();
    let guard = group.lock(&node);
    let off = if offset == u64::MAX { c.offset } else { offset };
    let n = if c.qid.typ == QTDIR {
        // The directory listing (devdirread via envgen): one stat-wire
        // record per entry, insertion order, rebuilt per call (values'
        // lengths may shift after writes — devcons's hostowner note).
        let mut listing = Vec::new();
        for e in guard.entries() {
            let start = listing.len();
            listing.resize(start + 41 + (2 + e.name.len()) + 3 * (2 + 6), 0);
            write_stat_record(
                &mut listing[start..],
                &e.name,
                e.qid_path,
                e.vers,
                QTFILE,
                0o666,
                e.value.len() as u64,
            )?;
        }
        let start = (off as usize).min(listing.len());
        let avail = &listing[start..];
        let n = avail.len().min(buf.len());
        buf[..n].copy_from_slice(&avail[..n]);
        n as u64
    } else {
        guard.read(c.qid.path, buf, off)?
    };
    drop(guard);
    if offset == u64::MAX {
        c.offset += n;
    }
    Ok(n)
}

/// Write an env chan (donor envwrite over the current group): the chan
/// must be a key open for writing; the write overwrites at the offset,
/// growing the value (zero-gap), bounded by MAXENVSIZE.  `offset`
/// u64::MAX uses + advances the streaming offset.
pub fn write(c: &mut Chan, buf: &[u8], offset: u64) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return Err("devenv: write of a directory");
    }
    if c.mode == OREAD {
        return Err("devenv: file not open for writing");
    }
    let group = current_group()?;
    let node = LockNode::new();
    let mut guard = group.lock(&node);
    let off = if offset == u64::MAX { c.offset } else { offset };
    let n = guard.write(c.qid.path, buf, off)?;
    drop(guard);
    if offset == u64::MAX {
        c.offset += n;
    }
    Ok(n)
}

/// Create a new key under the env root (donor envcreate) and return it
/// OPEN in `mode` — the op 05-06's sys_create dispatches to.  Eexist on
/// a duplicate (create-on-existing is namec's OTRUNC-open fallback, the
/// syscall side's business).  `perms` is accepted and ignored this era
/// (every env file serves 0666 per the donor's envgen).
pub fn create(c: &Chan, name: &[u8], mode: u32, perms: u32) -> Result<Chan> {
    let _ = perms;
    if c.qid.typ != QTDIR || c.qid.path != 0 {
        return Err("devenv: create under a non-directory");
    }
    let group = current_group()?;
    let qid_path = {
        let node = LockNode::new();
        let mut guard = group.lock(&node);
        guard.create(name)?
    };
    let mut nc = Chan::new(DEV_ENV, Qid { path: qid_path, vers: 0, typ: QTFILE }, mode & crate::dat::OMASK);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// stat of an env chan (sys_stat/sys_fstat's dispatch leg): the root
/// serves name "#e", DMDIR|0775, the group's vers (donor envgen/envstat);
/// a key serves name, 0666, the value's length, the entry's qid.
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    let group = current_group()?;
    let node = LockNode::new();
    let guard = group.lock(&node);
    if c.qid.typ == QTDIR {
        return write_stat_record(sb, b"#e", 0, guard.vers(), QTDIR, 0x8000_0000 | 0o775, 0);
    }
    let Some(e) = guard.get_qid(c.qid.path) else {
        return Err("devenv: nonexistent entry");
    };
    write_stat_record(sb, &e.name, e.qid_path, e.vers, QTFILE, 0o666, e.value.len() as u64)
}

/// One stat(5) wire record (the devcons/devproc shape; uid/gid/muid glenda
/// per D-02).  Returns the byte count (the convD2M contract).  FOUR
/// strings — libc's statcheck walks four and rejects a record that ends
/// early (convM2D.c).
fn write_stat_record(
    sb: &mut [u8],
    name: &[u8],
    qid_path: u64,
    vers: u32,
    typ: u8,
    mode: u32,
    length: u64,
) -> Result<u64> {
    let need = 41 + (2 + name.len()) + 3 * (2 + 6);
    if sb.len() < need {
        return Err("devenv: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(DEV_ENV as u32);
    w.u8(typ);
    w.u32(vers);
    w.u64(qid_path);
    w.u32(mode);
    w.u32(0); // atime
    w.u32(0); // mtime
    w.u64(length);
    w.string(name);
    w.string(b"glenda");
    w.string(b"glenda");
    w.string(b"glenda"); // muid — the fourth string statcheck walks
    Ok(w.finish())
}
