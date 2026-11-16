extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::result::Result;

use crate::mcslock::Lock;
use crate::paqfs::PaqDir;

/// A device/kernel error carrying a static message — the port crate's
/// `Result<T, &'static str>` convention applied at the Dev-trait boundary
/// (dat.rs's own `Error` type for the trait signatures below).
pub struct Error(pub &'static str);

/// Device-table index of the serving device.  A generated devtab arrives
/// with the multi-device era; until then the device
/// set carries these index values as a convention: devroot = 0, devcons =
/// 1, devproc-lite = 2 (the per-process note channel), devenv =
/// 3 (the per-pgrp environment device).
pub const DEV_ROOT: usize = 0;
pub const DEV_CONS: usize = 1;
pub const DEV_PROC: usize = 2;
pub const DEV_ENV: usize = 3;
/// devpipe (the Arc-shared blocking queue; registry rune '|'),
/// devdup (per-fd files '#d/N'; rune 'd'), devsrv (the '#s' service
/// registry; rune 's').
pub const DEV_PIPE: usize = 4;
pub const DEV_DUP: usize = 5;
pub const DEV_SRV: usize = 6;
/// devmnt — the kernel 9P client (the mount half).  Chans in a
/// mounted tree carry this index plus the session Arc and their
/// server-side fid (the Chan `mnt`/`mnt_fid` slots land with the device).
pub const DEV_MNT: usize = 7;
/// devether — the ethernet device `#l` (NETDRV-03; the registry rune
/// 'l').  No per-chan private slot: the qid path encodes the
/// conversation (netif.rs's NETQID scheme) and the single `ether0`
/// Netif is a module static this era.
pub const DEV_ETHER: usize = 8;
/// devip — the IP protocol device `#I` (NETIP-02; the registry rune
/// 'I').  The devip-exact /net file tree over smoltcp sockets; the qid
/// path encodes proto/conv/file (devip.rs's QID(p,c,y) scheme — its
/// OWN packing, never netif's).
pub const DEV_IP: usize = 9;
/// devcap — the capability device `#¤` (18-02, AUTH-03; the registry
/// rune '¤', the donor devcap.c).  Attached only under the dev_cap
/// config key (namec's registry arm); the qid paths are the donor's
/// Qdir/Qhash/Quse enum values.
pub const DEV_CAP: usize = 10;

/// devssl — the secure-sockets-layer device `#D` (19-04, DESK-04; the
/// registry rune 'D', the donor devssl.c — pushssl hard-codes #D).
/// The qid paths carry the donor's QID(c,y)=(c<<5)|y layout.
pub const DEV_SSL: usize = 11;

/// Plan 9 open-mode values (frozen: Harvey sys/include/libc.h).  The low
/// two bits are an access-MODE field (not a bit set); the upper bits are
/// flags.  These replace the retired `Mode` bitflags skeleton, whose
/// values were wrong for this encoding.
pub const OREAD: u32 = 0;
pub const OWRITE: u32 = 1;
pub const ORDWR: u32 = 2;
pub const OEXEC: u32 = 3;
pub const OTRUNC: u32 = 16;
pub const OCEXEC: u32 = 32;
pub const ORCLOSE: u32 = 64;
pub const OEXCL: u32 = 4096;
/// Mask for the access-mode field (the low two bits).
pub const OMASK: u32 = 3;

/// Plan 9 Qid type bytes (frozen: sys/include/libc.h) — shared by the
/// syscall layer's dir checks (chdir).  (The devices keep
/// their local copies; consolidating them is cleanup, not this plan.)
pub const QTDIR: u8 = 0x80;
pub const QTFILE: u8 = 0x00;

/// Mount/bind flag bits (frozen: Harvey sys/include/libc.h:558-564).  The
/// low two bits are the ORDER field (MREPL replaces; MBEFORE/MAFTER union
/// before/after); MCREATE permits creation in a mounted directory; MMASK
/// bounds the valid set (sysfile.c bindmount's (flag&~MMASK) check).
pub const MREPL: u32 = 0x0000;
pub const MBEFORE: u32 = 0x0001;
pub const MAFTER: u32 = 0x0002;
pub const MCREATE: u32 = 0x0004;
pub const MCACHE: u32 = 0x0010;
pub const MORDER: u32 = 0x0003;
pub const MMASK: u32 = 0x0017;

/// One mounted entry in a pgrp union chain (the donor's
/// `Mount`, chan.c): the chan that appears at the mount point, the frozen
/// flag bits carried on the entry (cmount records MCREATE here), and the
/// mount id (diagnostics + pgrpcpy order bookkeeping).  The mount TABLE
/// machinery lives in pgrp.rs; this is the shared vocabulary type.
pub struct MountEntry {
    /// The mounted chan (what resolution sees at the target).
    pub to: Chan,
    /// Frozen flag bits (MCREATE; the order bits live in chain position).
    pub mflag: u32,
    /// Monotonic mount id.
    pub mountid: u64,
}

/// A kernel channel — an open-file handle bound to a device.  Grown from
/// the placeholder skeleton for the device set: devroot
/// chans carry their paqfs directory entry in the crate-private slot.
pub struct Chan {
    /// This chan's current qid (file identity: path+vers+type).
    pub qid: Qid,
    /// Open mode (the O* constants above; OEXEC marks exec-opens per donor
    /// convention).
    pub mode: u32,
    /// Channel offset — the streaming read/write position (pread/pwrite
    /// with offset -1 use and advance this).
    pub offset: u64,
    /// Device-table index of the serving device (DEV_ROOT / DEV_CONS).
    pub dev: usize,
    /// The donor's `c->devno` (chan.c): the DEVICE-INSTANCE discriminator
    /// mount-table identity needs.  0 for every single-instance device;
    /// devmnt stamps each 9P session with its own id (the donor's
    /// devmnt.c `c->devno = mntalloc.id++`) so two mounted trees whose
    /// servers hand out colliding qids never blur into one identity —
    /// eqchanddq's (dc, devno, qid.path) triple, the mount-consult key
    /// (19-10 Leg-1: the path-keyed consult hijacked rio's window cons).
    pub devno: u32,
    /// Walked-path bookkeeping (device-maintained; for diagnostics/walks).
    pub path: Vec<u8>,
    /// Private devroot slot: the paqfs entry this chan currently names.
    /// (Written/read by devroot; the syscall PREAD consumes it.)
    pub(crate) paqdir: Option<PaqDir>,
    /// The pipe this chan references (devpipe chans
    /// ONLY — the dir chan from attach and the two end chans).  The Arc
    /// is the shared-queue handle BOTH pipe ends reference: a write on
    /// one end is readable on the other after a fork/dup.  None for every
    /// non-pipe chan.
    pub(crate) pipe: Option<Arc<Lock<crate::devpipe::Pipe>>>,
    /// The donor's COPEN — set by namec::open_chan (the single
    /// open funnel; namec::create_chan sets it for creates).  Read by
    /// dup's pipe-reference rule and namec::close_chan's dispatch.
    pub(crate) copen: bool,
    /// The donor's CCEXEC (portdat.h 0x0008, "close on exec"): this fd
    /// leaves the table at EXEC.  Recorded by namec::open_chan and
    /// sys_create when the open mode carries OCEXEC (the donor chan.c
    /// Aopen/Acreate arms — the bit never reaches the device), and
    /// swept by exec_replace's close_cexec_fds (the donor sysexec
    /// `fdclose(i, CCEXEC)` loop).  The live consumer is libthread's
    /// procexec: its exec-status pipe write end is opened
    /// OWRITE|OCEXEC, and the parent's status read only sees EOF when
    /// the exec'ed child's copy closes at exec — without the sweep the
    /// read parks forever and every procexec caller wedges (rio's
    /// new() recvul park: the post-sweep mouse death, 19-10 Leg-1
    /// cont 4).
    pub cexec: bool,
    /// The devmnt 9P session — set by
    /// devmnt::fversion on the PIPE chan (the donor's `c->mux`; sys_mount
    /// finds it there) and carried by every DEV_MNT chan in the mounted
    /// tree.  dup CLONES the Arc: devsrv's posted clone of the boot pipe
    /// carries the session to settime/newns's mounts.
    pub(crate) mnt: Option<Arc<crate::devmnt::Mnt>>,
    /// The server-side fid this chan names (DEV_MNT chans only; None =
    /// fid-less).  The fid is Arc'd — dup() CLONES
    /// the Arc (the fid's reference count is real, shared across every
    /// dup/open of the same walked chan), and the close dispatch
    /// (devmnt::clunk_if_last) Tclunks only when the LAST ref leaves
    /// (Arc::strong_count == 1) — the double-Tclunk divergence retired.
    pub(crate) mnt_fid: Option<Arc<u32>>,
    /// The chan's iounit — the Ropen/Rcreate reply's iounit, clamped to
    /// the session's per-rpc data cap (the donor devmnt.c mntopencreate:
    /// `c->iounit = r->reply.iounit; if(c->iounit == 0 || c->iounit >
    /// mnt->msize-IOHDRSZ) c->iounit = mnt->msize-IOHDRSZ`).  Set ONLY by
    /// devmnt's open/create legs; 0 for every device-local chan (the
    /// donor leaves it 0 there too — the clamp is the mount's business).
    /// Read by devdup's `#d/Nctl` fd line (libc iounit(2)'s parse
    /// source), so a mounted /dev/draw reports its real rpc cap and
    /// libdraw sizes its message buffer to whole-message writes — the
    /// 19-10 desktop leg's load-bearing ABI (an iounit of 0 made libdraw
    /// fall back to 8000, over the era's 4072 transport cap, and the
    /// mid-message split killed rio at `can't open display`).
    pub iounit: u32,
}

impl Chan {
    pub fn new(dev: usize, qid: Qid, mode: u32) -> Chan {
        Chan { qid, mode, offset: 0, dev, devno: 0, path: Vec::new(), paqdir: None, pipe: None, copen: false, cexec: false, mnt: None, mnt_fid: None, iounit: 0 }
    }

    /// Mark the chan open (COPEN) OUTSIDE the funnel — the one
    /// legitimate caller is proc.rs's fd 0/1/2 preopen (those fds are
    /// open by construction; never opened through open_chan).
    pub fn mark_open(&mut self) {
        self.copen = true;
    }

    /// The devmnt 9P session on this chan (sys_mount's lookup).
    /// Set only by devmnt::fversion (in-crate); read cross-crate here.
    pub fn mnt_session(&self) -> Option<Arc<crate::devmnt::Mnt>> {
        self.mnt.clone()
    }

    /// Value-dup for rfork's fd-table copy: a fresh Chan naming the
    /// same file/device (same dev/qid/mode; the paqfs entry handle is
    /// cloned).  Plan 9's shared-Chan semantics (one offset, refcounted)
    /// are approximated by a value copy — the streaming offset diverges
    /// after the fork, which the measured smoke does not depend on.
    ///
    /// THE PIPE EXCEPTION: an OPEN PIPE END chan's
    /// value copy is a NEW OPEN REFERENCE to the shared queue (the donor's
    /// cclone+incref shape): the Arc is cloned (the bytes stay shared) and
    /// the pipe's open-end count bumps.  Without this rule a fork/dup'd
    /// pipe's ends would diverge into per-chan copies and connectembed's
    /// 9P exchange would deadlock silently.
    ///
    /// THE ETHER EXCEPTION (the same rule, devether's conversation
    /// files): an OPEN data/ctl chan's value copy bumps the
    /// conversation's inuse count — each fd copy close-dispatches
    /// independently, so the count must track copies or the last-close
    /// reset would fire early.
    pub fn dup(&self) -> Chan {
        if self.copen && let Some(pipe) = &self.pipe {
            crate::devpipe::ref_open_end(pipe, self.qid.path);
        }
        if self.copen && self.dev == DEV_ETHER {
            crate::devether::ref_open(self.qid.path);
        }
        // THE DEVIP EXCEPTION (the same rule, devip's conversation
        // files): an OPEN ctl/data/err chan's value copy bumps the
        // conv's open count — rfork/dup'd fds close-dispatch
        // independently, so the count must track copies or the
        // last-close reset (closeconv) would fire early.
        if self.copen && self.dev == DEV_IP {
            crate::devip::ref_open(self.qid.path);
        }
        // THE DEVSSL EXCEPTION (19-08, the same rule, #D's
        // conversations): an OPEN ctl/data chan's value copy bumps the
        // conv's refc — a 9P session's io dup (devmnt::fversion over a
        // pushssl fd) must keep the conversation alive past the caller's
        // close, or every post-mount rpc of the mounted session dies
        // with the hung-up conversation (cpu -R's /mnt/term).
        if self.copen && self.dev == DEV_SSL {
            crate::devssl::ref_open(self.qid.path);
        }
        Chan {
            qid: self.qid,
            mode: self.mode,
            offset: self.offset,
            dev: self.dev,
            devno: self.devno,
            path: self.path.clone(),
            paqdir: self.paqdir.clone(),
            pipe: self.pipe.clone(),
            copen: self.copen,
            cexec: self.cexec,
            mnt: self.mnt.clone(),
            mnt_fid: self.mnt_fid.clone(),
            iounit: self.iounit,
        }
    }

    /// The transient OP-REFERENCE clone (fdgrp.rs's op_ref/op_done pair —
    /// the donor's fdtochan incref/unlock shape): a value clone that
    /// shares the pipe/mnt/fid Arcs WITHOUT dup()'s open-count bumps.  A
    /// syscall holding an op-ref is not a new OPEN, so devpipe's qref /
    /// devether's ref / devip's conv inuse must NOT move — and the slot
    /// keeps its chan the whole time, so a shared-group sibling's
    /// CONCURRENT op on the same fd (libthread's whole shape: aan's
    /// blocked reader proc and writer proc ride one netfd) never sees an
    /// emptied slot.
    pub fn op_clone(&self) -> Chan {
        Chan {
            qid: self.qid,
            mode: self.mode,
            offset: self.offset,
            dev: self.dev,
            devno: self.devno,
            path: self.path.clone(),
            paqdir: self.paqdir.clone(),
            pipe: self.pipe.clone(),
            copen: self.copen,
            cexec: self.cexec,
            mnt: self.mnt.clone(),
            mnt_fid: self.mnt_fid.clone(),
            iounit: self.iounit,
        }
    }

    /// Release an op-ref (the donor's decref side of fdtochan): only
    /// devmnt's fid lifecycle observes ref drops — clunk_if_last fires
    /// the Tclunk when THIS op-ref is the last ref (the
    /// close-raced-mid-op corner: the slot chan's close already ran and
    /// saw the op-ref's extra Arc).  devpipe/devether/devip counts were
    /// never bumped by op_clone, so their close-dispatch never fires
    /// here.
    pub fn drop_op_ref(self) {
        if self.dev == DEV_MNT {
            crate::devmnt::clunk_if_last(&self);
        }
    }
}

/// Little-endian field writer for the stat(5) wire buffer (stat(5) is LE).
/// Lifted from devroot.rs so every device serves the same
/// wire shape (sys_stat/sys_fstat's dispatch consumes the byte count from
/// the size prefix).  The caller sizes `buf` (fixed part + the FOUR
/// length-prefixed strings — name/uid/gid/muid; libc's statcheck walks
/// four, convM2D.c); `finish` writes the size prefix last and
/// returns the total byte count (convD2M's contract).
pub(crate) struct StatWriter<'a> {
    pub buf: &'a mut [u8],
    pub p: usize,
}

impl<'a> StatWriter<'a> {
    /// The fixed part is 41 bytes (size[2] type[2] dev[4] qid[13] mode[4]
    /// atime[4] mtime[4] length[8]); the cursor starts past the size
    /// prefix (written by finish).
    pub(crate) fn new(buf: &'a mut [u8]) -> StatWriter<'a> {
        StatWriter { buf, p: 2 }
    }
    pub(crate) fn u8(&mut self, v: u8) {
        self.buf[self.p] = v;
        self.p += 1;
    }
    pub(crate) fn u16(&mut self, v: u16) {
        self.buf[self.p..self.p + 2].copy_from_slice(&v.to_le_bytes());
        self.p += 2;
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.buf[self.p..self.p + 4].copy_from_slice(&v.to_le_bytes());
        self.p += 4;
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.buf[self.p..self.p + 8].copy_from_slice(&v.to_le_bytes());
        self.p += 8;
    }
    pub(crate) fn string(&mut self, s: &[u8]) {
        self.u16(s.len() as u16);
        self.buf[self.p..self.p + s.len()].copy_from_slice(s);
        self.p += s.len();
    }
    /// Write the size prefix (bytes used minus the prefix itself) and
    /// return the TOTAL byte count (prefix included) — the convD2M/sysstat
    /// return contract.
    pub(crate) fn finish(self) -> u64 {
        let n = (self.p - 2) as u16;
        self.buf[0..2].copy_from_slice(&n.to_le_bytes());
        self.p as u64
    }
}

#[allow(dead_code)]
pub struct Device {
    _dc: u32,
    _name: &'static str,
    _attached: bool,
}

pub trait Dev {
    fn reset();
    fn init();
    fn shutdown();
    fn attach(spec: &[u8]) -> Chan;
    fn walk(&self, c: &Chan, nc: &mut Chan, name: &[&[u8]]) -> Walkqid;
    fn stat(&self, c: &Chan, sb: &mut [u8]) -> Result<(), Error>;
    fn open(&mut self, c: &Chan, mode: u32) -> Chan;
    fn create(&mut self, c: &mut Chan, name: &[u8], mode: u32, perms: u32);
    fn close(&mut self, c: Chan);
    fn read(&mut self, c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64, Error>;
    fn bread(&mut self, c: &mut Chan, bnum: u64, offset: u64) -> Result<Block, Error>;
    fn write(&mut self, c: &mut Chan, buf: &[u8], offset: u64) -> Result<u64, Error>;
    fn bwrite(&mut self, c: &mut Chan, block: &Block, offset: u64) -> Result<u64, Error>;
    fn remove(&mut self, c: &mut Chan);
    fn wstat(&mut self, c: &mut Chan, sb: &[u8]) -> Result<(), Error>;
    fn power(&mut self, on: bool);
    fn config(&mut self /* other args */) -> Result<(), Error>;
}

pub struct Block {
    // TODO(cross): block linkage.
    _rp: usize,
    _wp: usize,
    _lim: *const u8,
    _base: *mut u8,
    _flag: u16,
    _checksum: u16,
    _magic: u32,
}

/// Unique file identity (9P/Plan 9 Qid: path+vers+type).
#[derive(Clone, Copy, Debug)]
pub struct Qid {
    pub path: u64,
    pub vers: u32,
    pub typ: u8,
}

pub struct Walkqid {
    _clone: Arc<Chan>,
    _qids: [Qid],
}

pub struct Proc {}
