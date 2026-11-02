extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::result::Result;

use crate::paqfs::PaqDir;

/// A device/kernel error carrying a static message — the port crate's
/// `Result<T, &'static str>` convention applied at the Dev-trait boundary
/// (dat.rs's own `Error` type for the trait signatures below).
pub struct Error(pub &'static str);

/// A kernel channel — an open-file handle bound to a device.  Grown from
/// the placeholder skeleton for the Phase-4 device set (D-15): devroot
/// chans carry their paqfs directory entry in the crate-private slot.
pub struct Chan {
    /// This chan's current qid (file identity: path+vers+type).
    pub qid: Qid,
    /// Open mode (Mode bits; OEXEC marks exec-opens per donor convention).
    pub mode: u32,
    /// Channel offset — the streaming read/write position (pread/pwrite
    /// with offset -1 use and advance this).
    pub offset: u64,
    /// Device-table index of the serving device.
    pub dev: usize,
    /// Walked-path bookkeeping (device-maintained; for diagnostics/walks).
    pub path: Vec<u8>,
    /// Private devroot slot: the paqfs entry this chan currently names.
    /// (Written/read by 04-05's devroot; nothing consumes Chan yet.)
    #[allow(dead_code)]
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

bitflags! {
    pub struct Mode: u16 {
        const READ = 0;
        const WRITE = 1;
        const OEXEC = 2;
        const OTRUNC = 4;
        const OCEXEC = 5;
        const ORCLOSE = 6;
        const OEXCL = 12;
    }
}

pub trait Dev {
    fn reset();
    fn init();
    fn shutdown();
    fn attach(spec: &[u8]) -> Chan;
    fn walk(&self, c: &Chan, nc: &mut Chan, name: &[&[u8]]) -> Walkqid;
    fn stat(&self, c: &Chan, sb: &mut [u8]) -> Result<(), Error>;
    fn open(&mut self, c: &Chan, mode: u32) -> Chan;
    fn create(&mut self, c: &mut Chan, name: &[u8], mode: Mode, perms: u32);
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
