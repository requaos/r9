//! devroot — the root file tree over the initrd paqfs image (04-05, D-15).
//! The one mechanism exec sources binaries through: attach → root Chan, walk
//! a single name component, open OREAD/OEXEC, read via the paqfs reader's
//! byte-granular read_at, stat minimal (name/qid/length from the PaqDir).
//!
//! Like the paqfs reader it wraps, the device is stateless-per-call over the
//! memory-resident image: no statics, no locks, no mutation — safe to call
//! from process/boot context this era (the trap-context fault handler reads
//! the image directly through its own Paqfs, never through here).
//!
//! The dat.rs `Dev` trait is NOT implemented here: its skeleton is
//! undispatched and partially non-constructible as written (`Walkqid` has an
//! unsized `_qids: [Qid]` field, and `attach` is a static that could not see
//! a mounted image).  This module implements the device OPERATIONS the plan
//! names (attach/walk/open/read/stat) as inherent methods; trait conformance
//! lands when the devtab dispatch era (04-06+) grows the skeleton into a
//! constructible shape.

use crate::dat::{Chan, Qid};
use crate::mcslock::{Lock, LockNode};
use crate::paqfs::{PaqDir, Paqfs};
use crate::Result;

/// Plan 9 open modes (sys/include/libc.h): the low two bits are the access
/// mode, NOT a bit set.  devroot is read-only; exec opens its binary OEXEC
/// per the donor convention.  The frozen values live in dat.rs (04-06 fixed
/// the `Mode` bitflag skeleton — phase-04 deferred-items.md #1); re-exported
/// here so existing `devroot::OEXEC` call sites keep working.
pub use crate::dat::{OEXEC, OREAD};

/// paqfs dir-mode bit (Plan 9 DMDIR): marks directory PaqDir entries.
const DMDIR: u32 = 0x8000_0000;
/// Plan 9 Qid type bytes.
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// The mounted initrd root file tree.
pub struct DevRoot {
    img: &'static [u8],
}

/// The mounted root image, published once by mount_initrd (single writer,
/// pre-EL0; read thereafter from boot/syscall context).  04-06's syscall
/// handlers reach the root tree through `root()` — DevRoot is
/// stateless-per-call over the memory-resident image, so a fresh handle
/// per call is the reentrant shape.  The mcslock idiom (CONS/PAGE_ALLOC
/// pattern) guards the one-time publish; readers take it briefly.
static ROOT_IMG: Lock<Option<&'static [u8]>> = Lock::new("devroot", None);

/// Mount the initrd paqfs image as the root tree.  The image is validated
/// (header + trailer) up front — a corrupt image is a loud Err at mount,
/// never a mid-boot surprise.
pub fn mount_initrd(img: &'static [u8]) -> Result<DevRoot> {
    Paqfs::open_image(img)?;
    // Publish for the syscall-era consumers (OPEN/PREAD on devroot chans,
    // the EXEC handler's binary source).  Exactly one mount exists this era.
    let node = LockNode::new();
    *ROOT_IMG.lock(&node) = Some(img);
    Ok(DevRoot { img })
}

/// A fresh handle to the mounted root tree, or None before main9's devroot
/// mount.  Construction is cheap (the Paqfs revalidation happens per
/// operation, not here).
pub fn root() -> Option<DevRoot> {
    let node = LockNode::new();
    (*ROOT_IMG.lock(&node)).map(|img| DevRoot { img })
}

impl DevRoot {
    fn fs(&self) -> Result<Paqfs<'static>> {
        Paqfs::open_image(self.img)
    }

    /// attach: the root Chan (Plan 9's attach spec selects the tree; r9 has
    /// exactly one — the initrd — so the spec is empty).
    pub fn attach_root(&self) -> Result<Chan> {
        let root = self.fs()?.root_dir()?;
        let mut c = Chan::new(crate::dat::DEV_ROOT, qid_of(&root), 0);
        c.paqdir = Some(root);
        Ok(c)
    }

    /// Walk a single name component from `c` (which must name a directory);
    /// the returned Chan carries the matched PaqDir.  `..` and multi-element
    /// walks are out of scope for the root tree this era.
    pub fn walk(&self, c: &Chan, name: &[u8]) -> Result<Chan> {
        let dir = c.paqdir.as_ref().ok_or("devroot: walk of unattached chan")?;
        if dir.mode & DMDIR == 0 {
            return Err("devroot: walk of a non-directory");
        }
        if name.is_empty() || name.contains(&b'/') {
            return Err("devroot: walk takes a single name component");
        }
        let entries = self.fs()?.read_dir(dir)?;
        let Some(entry) = entries.into_iter().find(|e| e.name == name) else {
            return Err("devroot: name not found");
        };
        let mut nc = Chan::new(c.dev, qid_of(&entry), 0);
        nc.paqdir = Some(entry);
        nc.path = c.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(name);
        Ok(nc)
    }

    /// Open `c` OREAD or OEXEC.  Everything else (write-side modes, trunc)
    /// is a read-only-fs error; a directory opens OREAD only.
    pub fn open(&self, c: &Chan, mode: u32) -> Result<Chan> {
        let dir = c.paqdir.as_ref().ok_or("devroot: open of unattached chan")?;
        let is_dir = dir.mode & DMDIR != 0;
        match mode {
            OREAD => {}
            OEXEC if !is_dir => {}
            OEXEC => return Err("devroot: cannot exec-open a directory"),
            _ => return Err("devroot: read-only filesystem"),
        }
        let mut nc = Chan::new(c.dev, c.qid, mode);
        nc.paqdir = Some(dir.clone());
        nc.path = c.path.clone();
        Ok(nc)
    }

    /// Read via the paqfs reader's byte-granular read_at.  `offset` is the
    /// explicit file offset; `u64::MAX` (pread/pwrite's -1) means "use and
    /// advance the chan's streaming offset" (04-RESEARCH.md fd convention).
    pub fn read(&self, c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
        let dir = c.paqdir.as_ref().ok_or("devroot: read of unattached chan")?;
        if dir.mode & DMDIR != 0 {
            return Err("devroot: read of a directory");
        }
        let off = if offset == u64::MAX { c.offset } else { offset };
        let n = self.fs()?.read_at(dir, off, buf)? as u64;
        if offset == u64::MAX {
            c.offset += n;
        }
        Ok(n)
    }

    /// Minimal stat: the Plan 9 stat(5) wire shape — fixed fields plus the
    /// name/uid/gid strings from the PaqDir (qid path+vers+type, mode,
    /// mtime, length).  `sb` must be large enough; the size prefix is
    /// written last.
    pub fn stat(&self, c: &Chan, sb: &mut [u8]) -> Result<()> {
        let dir = c.paqdir.as_ref().ok_or("devroot: stat of unattached chan")?;
        // Fixed part: size[2] type[2] dev[4] qid[13] mode[4] atime[4]
        // mtime[4] length[8] = 41 bytes, then three length-prefixed strings.
        let fixed = 41;
        let need = fixed
            + (2 + dir.name.len())
            + (2 + dir.uid.len())
            + (2 + dir.gid.len());
        if sb.len() < need {
            return Err("devroot: stat buffer too small");
        }
        let mut w = StatWriter { buf: sb, p: 2 }; // size prefix filled in last
        w.u16(0); // type: unused this era
        w.u32(c.dev as u32);
        w.u8(c.qid.typ);
        w.u32(c.qid.vers);
        w.u64(c.qid.path);
        w.u32(dir.mode);
        w.u32(dir.mtime); // atime
        w.u32(dir.mtime);
        w.u64(u64::from(dir.length));
        w.string(&dir.name);
        w.string(&dir.uid);
        w.string(&dir.gid);
        let n = (w.p - 2) as u16;
        sb[0..2].copy_from_slice(&n.to_le_bytes());
        Ok(())
    }

    /// The PaqDir an opened chan names — exec's byte-source handle for the
    /// loader (a.out header read) and the fault handler (text/data page-in).
    pub fn entry(&self, c: &Chan) -> Result<PaqDir> {
        c.paqdir.clone().ok_or("devroot: chan carries no paqfs entry")
    }

    /// The mounted image slice — the EXEC handler's byte-source input
    /// (04-06; exec sources binaries through devroot, one mechanism).
    pub fn image(&self) -> &'static [u8] {
        self.img
    }
}

/// The chan's Qid from its PaqDir: path = the paqfs qid, vers = mtime (Plan
/// 9 convention), typ = dir/file.
fn qid_of(dir: &PaqDir) -> Qid {
    Qid {
        path: u64::from(dir.qid),
        vers: dir.mtime,
        typ: if dir.mode & DMDIR != 0 { QTDIR } else { QTFILE },
    }
}

/// Little-endian field writer for the stat wire buffer (stat(5) is LE).
struct StatWriter<'a> {
    buf: &'a mut [u8],
    p: usize,
}

impl StatWriter<'_> {
    fn u8(&mut self, v: u8) {
        self.buf[self.p] = v;
        self.p += 1;
    }
    fn u16(&mut self, v: u16) {
        self.buf[self.p..self.p + 2].copy_from_slice(&v.to_le_bytes());
        self.p += 2;
    }
    fn u32(&mut self, v: u32) {
        self.buf[self.p..self.p + 4].copy_from_slice(&v.to_le_bytes());
        self.p += 4;
    }
    fn u64(&mut self, v: u64) {
        self.buf[self.p..self.p + 8].copy_from_slice(&v.to_le_bytes());
        self.p += 8;
    }
    fn string(&mut self, s: &[u8]) {
        self.u16(s.len() as u16);
        self.buf[self.p..self.p + s.len()].copy_from_slice(s);
        self.p += s.len();
    }
}
