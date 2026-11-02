extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::result::Result;

use crate::paqfs::PaqDir;

/// A device/kernel error carrying a static message — the port crate's
/// `Result<T, &'static str>` convention applied at the Dev-trait boundary
/// (dat.rs's own `Error` type for the trait signatures below).
pub struct Error(pub &'static str);

/// Device-table index of the serving device.  A generated devtab arrives
/// with the multi-device era (04-07+); until then the D-15 Phase-4 device
/// set carries these two index values as a convention: devroot = 0,
/// devcons = 1.
pub const DEV_ROOT: usize = 0;
pub const DEV_CONS: usize = 1;

/// Plan 9 open-mode values (frozen: Harvey sys/include/libc.h).  The low
/// two bits are an access-MODE field (not a bit set); the upper bits are
/// flags.  These replace the pre-Phase-4 `Mode` bitflags skeleton, whose
/// values were wrong for this encoding (phase-04 deferred-items.md #1).
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

/// A kernel channel — an open-file handle bound to a device.  Grown from
/// the placeholder skeleton for the Phase-4 device set (D-15): devroot
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
    /// Walked-path bookkeeping (device-maintained; for diagnostics/walks).
    pub path: Vec<u8>,
    /// Private devroot slot: the paqfs entry this chan currently names.
    /// (Written/read by 04-05's devroot; 04-06's syscall PREAD consumes it.)
    pub(crate) paqdir: Option<PaqDir>,
}

impl Chan {
    pub fn new(dev: usize, qid: Qid, mode: u32) -> Chan {
        Chan { qid, mode, offset: 0, dev, path: Vec::new(), paqdir: None }
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
