//! devroot — the root file tree over the initrd paqfs image.
//! The one mechanism exec sources binaries through: attach → root Chan, walk
//! a single name component, open OREAD/OEXEC, read via the paqfs reader's
//! byte-granular read_at, stat minimal (name/qid/length from the PaqDir).
//! Directories read as the stat(5)-wire listing of their children
//! (the donor devroot's devdirread; the root listing appends the synthesized
//! /rootfs.img node so a listing names every walkable entry).
//!
//! The tree additionally exposes `/rootfs.img` — the
//! RAW view of the same image the tree walks (the two-views note): the
//! kernel execs from the walked tree while the userspace paqfs server
//! serves the raw bytes.  A baked image shipping its own rootfs.img root
//! entry is REJECTED at mount_initrd with a named error (one view must
//! never shadow the other silently).  See the RAWIMG_NAME block.
//!
//! Like the paqfs reader it wraps, the device is stateless-per-call over the
//! memory-resident image: no statics, no locks, no mutation — safe to call
//! from process/boot context this era (the trap-context fault handler reads
//! the image directly through its own Paqfs, never through here).
//!
//! The dat.rs `Dev` trait is NOT implemented here: its skeleton is
//! undispatched and partially non-constructible as written (`Walkqid` has an
//! unsized `_qids: [Qid]` field, and `attach` is a static that could not see
//! a mounted image).  This module implements the device OPERATIONS
//! (attach/walk/open/read/stat) as inherent methods; trait conformance
//! lands when the devtab dispatch era grows the skeleton into a
//! constructible shape.

use alloc::vec::Vec;

use crate::dat::{Chan, Qid, StatWriter};
use crate::mcslock::{Lock, LockNode};
use crate::paqfs::{PaqDir, Paqfs};
use crate::Result;

/// Plan 9 open modes (sys/include/libc.h): the low two bits are the access
/// mode, NOT a bit set.  devroot is read-only; exec opens its binary OEXEC
/// per the donor convention.  The frozen values live in dat.rs;
/// re-exported
/// here so existing `devroot::OEXEC` call sites keep working.
pub use crate::dat::{OEXEC, OREAD};

/// paqfs dir-mode bit (Plan 9 DMDIR): marks directory PaqDir entries.
const DMDIR: u32 = 0x8000_0000;
/// Plan 9 Qid type bytes.
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// The raw initrd
/// image served as the root-tree file `/rootfs.img` — the exact literal
/// bootr9.c's embed method arg names (the cross-repo contract).  The
/// kernel walks the paqfs TREE for exec; the userspace paqfs server reads
/// the raw BYTES through this node — two views of the one image.
/// Synthesized at the root only (a deeper or re-rooted chan never
/// synthesizes).  Reads proxy the image slice with the paqfs reader's own
/// clamp convention (read_at: off ≥ length is EOF = 0, never an error);
/// every write-side mode and OEXEC are refused with NAMED errors.  The
/// reserved qid never collides with mkpaqfs qids (sequential from 1).
///
/// Name-collision discipline: if the baked image ever ships
/// its OWN rootfs.img entry at the root, mount_initrd FAILS with a named
/// error rather than letting one view shadow the other — a real entry
/// would masquerade as the raw image to the paqfs server (its bytes
/// would be the tree's file, not the image), an impossible-to-debug
/// divergence served loudly at mount instead.
const RAWIMG_QID: u32 = u32::MAX;
const RAWIMG_NAME: &[u8] = b"rootfs.img";

/// The mounted initrd root file tree.
pub struct DevRoot {
    img: &'static [u8],
}

/// The mounted root image, published once by mount_initrd (single writer,
/// pre-EL0; read thereafter from boot/syscall context).  The syscall
/// handlers reach the root tree through `root()` — DevRoot is
/// stateless-per-call over the memory-resident image, so a fresh handle
/// per call is the reentrant shape.  The mcslock idiom (CONS/PAGE_ALLOC
/// pattern) guards the one-time publish; readers take it briefly.
static ROOT_IMG: Lock<Option<&'static [u8]>> = Lock::new("devroot", None);

/// Mount the initrd paqfs image as the root tree.  The image is validated
/// (header + trailer) up front — a corrupt image is a loud Err at mount,
/// never a mid-boot surprise.  The collision refusal joins
/// the validation: an image shipping its own rootfs.img entry at the
/// root is rejected BEFORE anything is published (the raw-image node
/// would silently shadow / be shadowed by it; see RAWIMG_NAME).
pub fn mount_initrd(img: &'static [u8]) -> Result<DevRoot> {
    let fs = Paqfs::open_image(img)?;
    let root = fs.root_dir()?;
    let entries = fs.read_dir(&root)?;
    if entries.iter().any(|e| e.name == RAWIMG_NAME) {
        return Err(
            "devroot: image ships its own /rootfs.img entry — refusing to mount (would shadow the raw-image node)",
        );
    }
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
    /// walks are out of scope for the root tree this era.  At the ROOT only,
    /// a failed lookup of the raw-image name synthesizes the /rootfs.img
    /// node (see RAWIMG_NAME); the mount-time collision refusal
    /// means a real entry of that name cannot exist post-mount (the real-
    /// entry branch below stays as belt-and-braces).
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
            if c.path.is_empty() && name == RAWIMG_NAME {
                return Ok(self.rawimg_chan(c));
            }
            return Err("devroot: name not found");
        };
        let mut nc = Chan::new(c.dev, qid_of(&entry), 0);
        nc.paqdir = Some(entry);
        nc.path = c.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(name);
        Ok(nc)
    }

    /// The synthesized /rootfs.img chan (the raw initrd image node).
    fn rawimg_chan(&self, root: &Chan) -> Chan {
        let raw = self.rawimg_paqdir();
        let mut nc = Chan::new(root.dev, qid_of(&raw), 0);
        nc.paqdir = Some(raw);
        nc.path = root.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(RAWIMG_NAME);
        nc
    }

    /// The synthesized node's PaqDir (walk's chan synthesis and the
    /// root listing share the shape).
    fn rawimg_paqdir(&self) -> PaqDir {
        PaqDir {
            qid: RAWIMG_QID,
            mode: 0o444,
            mtime: 0,
            length: self.img.len() as u32,
            offset: 0,
            name: RAWIMG_NAME.to_vec(),
            uid: b"glenda".to_vec(),
            gid: b"glenda".to_vec(),
        }
    }

    /// Open `c` OREAD or OEXEC.  Everything else (write-side modes, trunc)
    /// is a read-only-fs error; a directory opens OREAD only.  The raw-image
    /// node opens OREAD only (it is not a binary) — its write-side and
    /// exec refusals carry NAMED errors (the node's
    /// read-only contract is a load-bearing part of the two-views story,
    /// never a generic fs error).
    pub fn open(&self, c: &Chan, mode: u32) -> Result<Chan> {
        let dir = c.paqdir.as_ref().ok_or("devroot: open of unattached chan")?;
        let is_dir = dir.mode & DMDIR != 0;
        let is_rawimg = dir.qid == RAWIMG_QID;
        match mode {
            OREAD => {}
            OEXEC if !is_dir && !is_rawimg => {}
            OEXEC if is_rawimg => return Err("devroot: cannot exec the raw image /rootfs.img"),
            OEXEC => return Err("devroot: cannot exec-open a directory"),
            _ if is_rawimg => return Err("devroot: /rootfs.img is read-only"),
            _ => return Err("devroot: read-only filesystem"),
        }
        let mut nc = Chan::new(c.dev, c.qid, mode);
        nc.paqdir = Some(dir.clone());
        nc.path = c.path.clone();
        Ok(nc)
    }

    /// Read via the paqfs reader's byte-granular read_at.  `offset` is the
    /// explicit file offset; `u64::MAX` (pread/pwrite's -1) means "use and
    /// advance the chan's streaming offset".
    /// The raw-image node reads the image slice directly (its PaqDir's
    /// block offset names nothing — paqfs blocks are the TREE's encoding).
    /// A DIRECTORY read serves the stat(5)-wire listing of the
    /// PaqDir's children (the donor devroot's devdirread over the root
    /// tree — any dirreadall of a devroot dir, e.g. reading `/` before
    /// newns binds the mounted union over it); the records are the fixed
    /// 4-string shape via write_entry.  At the ROOT the synthesized
    /// /rootfs.img node appends (walk synthesizes it there; a listing
    /// names every walkable entry).
    pub fn read(&self, c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
        let dir = c.paqdir.as_ref().ok_or("devroot: read of unattached chan")?;
        let off = if offset == u64::MAX { c.offset } else { offset };
        let n: u64 = if dir.mode & DMDIR != 0 {
            let mut listing = Vec::new();
            for e in self.fs()?.read_dir(dir)? {
                Self::push_entry(&mut listing, c.dev, qid_of(&e), &e)?;
            }
            if c.path.is_empty() {
                let raw = self.rawimg_paqdir();
                Self::push_entry(&mut listing, c.dev, qid_of(&raw), &raw)?;
            }
            let start = (off as usize).min(listing.len());
            let avail = &listing[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        } else if dir.qid == RAWIMG_QID {
            // Bounds-checked slice read (clamp at the image end; a read at
            // or past the end is EOF = 0).
            let start = (off as usize).min(self.img.len());
            let avail = &self.img[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        } else {
            self.fs()?.read_at(dir, off, buf)? as u64
        };
        if offset == u64::MAX {
            c.offset += n;
        }
        Ok(n)
    }

    /// Append one entry's stat(5) wire record to a directory listing
    /// (the record shape is stat's — see write_entry).
    fn push_entry(listing: &mut Vec<u8>, dev: usize, qid: Qid, dir: &PaqDir) -> Result<()> {
        let start = listing.len();
        let need = 41
            + (2 + dir.name.len())
            + (2 + dir.uid.len())
            + (2 + dir.gid.len())
            + (2 + dir.uid.len()); // muid = uid
        listing.resize(start + need, 0);
        let n = Self::write_entry(&mut listing[start..], dev, qid, dir)?;
        debug_assert_eq!(n as usize, need);
        Ok(())
    }

    /// One stat(5) wire record for a PaqDir (stat + the directory listing
    /// share the shape): the 41-byte fixed part + FOUR length-prefixed
    /// strings — name/uid/gid/muid (libc's statcheck walks four and
    /// rejects a record that ends early, convM2D.c); muid = uid per the
    /// donor's devdir (db->muid = user).  Returns the byte count (the
    /// convD2M/sysstat contract).
    fn write_entry(sb: &mut [u8], dev: usize, qid: Qid, dir: &PaqDir) -> Result<u64> {
        let fixed = 41;
        let need = fixed
            + (2 + dir.name.len())
            + (2 + dir.uid.len())
            + (2 + dir.gid.len())
            + (2 + dir.uid.len()); // muid = uid (read-only tree; devdir's db->muid = user)
        if sb.len() < need {
            return Err("devroot: stat buffer too small");
        }
        let mut w = StatWriter::new(sb);
        w.u16(0); // type: unused this era
        w.u32(dev as u32);
        w.u8(qid.typ);
        w.u32(qid.vers);
        w.u64(qid.path);
        w.u32(dir.mode);
        w.u32(dir.mtime); // atime
        w.u32(dir.mtime);
        w.u64(u64::from(dir.length));
        w.string(&dir.name);
        w.string(&dir.uid);
        w.string(&dir.gid);
        w.string(&dir.uid); // muid — the fourth string statcheck walks
        Ok(w.finish())
    }

    /// Minimal stat: the Plan 9 stat(5) wire shape — fixed fields plus the
    /// name/uid/gid/muid strings from the PaqDir (qid path+vers+type, mode,
    /// mtime, length).  `sb` must be large enough; the size prefix is
    /// written last.  Returns the byte count (prefix included — the
    /// convD2M/sysstat contract).  The record shape is
    /// write_entry's (shared with the directory listing).
    pub fn stat(&self, c: &Chan, sb: &mut [u8]) -> Result<u64> {
        let dir = c.paqdir.as_ref().ok_or("devroot: stat of unattached chan")?;
        Self::write_entry(sb, c.dev, c.qid, dir)
    }

    /// The PaqDir an opened chan names — exec's byte-source handle for the
    /// loader (a.out header read) and the fault handler (text/data page-in).
    pub fn entry(&self, c: &Chan) -> Result<PaqDir> {
        c.paqdir.clone().ok_or("devroot: chan carries no paqfs entry")
    }

    /// The mounted image slice — the EXEC handler's byte-source input
    /// (exec sources binaries through devroot, one mechanism).
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
