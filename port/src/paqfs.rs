//! no_std paqfs reader (phase 04-02, D-02) over a memory-resident image.
//!
//! API skeleton for the RED phase of TDD: the types and signatures below are
//! the contract the host tests pin; bodies return `Err` until the GREEN
//! commit implements them.
//!
//! Format truth: Harvey `sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c,paqfs.c}` and
//! the pinned D-02 spec in 04-RESEARCH.md.  All on-disk integers are
//! big-endian.

use alloc::vec::Vec;

use crate::Result;

/// Block type byte at block-header offset 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    DirBlock = 0,
    DataBlock = 1,
    PointerBlock = 2,
}

/// One directory entry from a DirBlock payload (paqfs.h PaqDir).
/// Strings are raw bytes: Plan 9 names are UTF-8 by convention, but a hostile
/// image must never force a UTF-8 validation panic in the reader.
#[derive(Debug, Clone)]
pub struct PaqDir {
    pub qid: u32,
    pub mode: u32,
    pub mtime: u32,
    pub length: u32,
    /// Byte offset of this entry's PointerBlock.
    pub offset: u32,
    pub name: Vec<u8>,
    pub uid: Vec<u8>,
    pub gid: Vec<u8>,
}

/// A parsed paqfs image.  Stateless-per-call over the memory-resident slice:
/// no statics, no locks, no mutation of the image — reentrant by construction
/// (the 04-04 fault handler calls this in trap context with IRQs off).
pub struct Paqfs<'img> {
    img: &'img [u8],
    blocksize: usize,
    root_off: u32,
}

impl<'img> Paqfs<'img> {
    /// Parse the 44-byte header (small form only) and the 28-byte trailer.
    pub fn open_image(_img: &'img [u8]) -> Result<Self> {
        Err("paqfs: not implemented")
    }

    pub fn blocksize(&self) -> usize {
        self.blocksize
    }

    /// Decode the block at byte offset `off` and return its `blocksize`-byte
    /// decoded payload.  Handles NoEnc (always blocksize bytes on disk) and
    /// DeflateEnc (raw RFC 1951 inflate); verifies the adler32 of the decoded
    /// payload against the block header.
    pub fn read_block(&self, _off: u32, _expect_type: BlockType) -> Result<Vec<u8>> {
        Err("paqfs: not implemented")
    }

    /// The root directory's PaqDir: the first entry of the trailer-root
    /// DirBlock (paqfs.c:914-938 crafts the fs root from it).
    pub fn root_dir(&self) -> Result<PaqDir> {
        Err("paqfs: not implemented")
    }

    /// All entries of a directory (its PointerBlock chase + per-block scan,
    /// stopping at the first zero pointer / zero size / overrun entry).
    pub fn read_dir(&self, _dir: &PaqDir) -> Result<Vec<PaqDir>> {
        Err("paqfs: not implemented")
    }

    /// Resolve `/`-separated `path` from the root.  An empty (or all-`/`)
    /// path returns the root directory entry.
    pub fn walk(&self, _path: &str) -> Result<PaqDir> {
        Err("paqfs: not implemented")
    }

    /// Read `buf` bytes of `dir`'s file content starting at byte offset
    /// `off`; clamped to `dir.length`.  Returns the number of bytes read.
    pub fn read_at(&self, _dir: &PaqDir, _off: u64, _buf: &mut [u8]) -> Result<usize> {
        Err("paqfs: not implemented")
    }
}
