//! no_std paqfs reader over a memory-resident image.
//!
//! Format truth: Harvey `sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c,paqfs.c}.
//! All on-disk integers are
//! big-endian.  Reference reader semantics: paqfs.c `blockRead` (:942-987),
//! `paqWalk` (:730-779), `checkDirSize`/`getDir` (:1112-1194).
//!
//! The reader is stateless-per-call over the image slice: no statics, no
//! locks, no mutation of the image — reentrant by construction, safe for the
//! fault handler's trap context.  Every offset is bounds-checked
//! against the image slice BEFORE slicing; every failure is a typed
//! `Err` with a distinct static message — nothing here panics on hostile
//! input.

use alloc::vec::Vec;

use crate::Result;

const HEADER_MAGIC: u32 = 0x529A_B12B;
const BLOCK_MAGIC: u32 = 0x198A_1CBF;
const TRAILER_MAGIC: u32 = 0x6B46_E688;
const VERSION: u16 = 1;

const HEADER_SIZE: usize = 44;
const TRAILER_SIZE: usize = 28;
const BLOCK_HEADER_SIZE: usize = 12;
/// Minimum legal PaqDir entry: 22 fixed bytes + three 2-byte string prefixes.
const MIN_DIR_SIZE: usize = 28;

/// Block type byte at block-header offset 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    DirBlock = 0,
    DataBlock = 1,
    PointerBlock = 2,
}

impl BlockType {
    fn from_u8(v: u8) -> Result<Self> {
        match v {
            0 => Ok(Self::DirBlock),
            1 => Ok(Self::DataBlock),
            2 => Ok(Self::PointerBlock),
            _ => Err("paqfs: unknown block type"),
        }
    }
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

/// A parsed paqfs image: header/trailer-validated, blocks decoded on demand.
pub struct Paqfs<'img> {
    img: &'img [u8],
    blocksize: usize,
    root_off: u32,
}

fn be16(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(off..off + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

/// Harvey libflate `adler32(0, buf, n)` — the seed-0 variant (s1 = s2 = 0),
/// NOT the RFC 1950 seed (s1 = 1): mkpaqfs writes and paqfs checks this
/// variant.  `Adler32::from_checksum(0)` seeds a=0,b=0 to reproduce it
/// exactly (fixture DeflateEnc block headers pin this byte-for-byte).
fn paq_adler32(buf: &[u8]) -> u32 {
    let mut a = adler2::Adler32::from_checksum(0);
    a.write_slice(buf);
    a.checksum()
}

/// Parse one PaqDir entry at `p` within a decoded DirBlock payload.
///
/// Returns `Ok(Some((entry, next_offset)))` for a well-formed entry, or
/// `Ok(None)` when the scan must terminate — paqfs.c `checkDirSize`
/// semantics: zero size, a size prefix that overruns the block, or strings
/// that do not fit within the entry all end the block's entries without
/// reading outside the slice.
fn parse_dir_entry(block: &[u8], p: usize) -> Result<Option<(PaqDir, usize)>> {
    let Some(size) = be16(block, p).map(|v| v as usize) else {
        return Ok(None); // fewer than 2 bytes remain
    };
    if size == 0 || p + size > block.len() || size < MIN_DIR_SIZE {
        return Ok(None);
    }
    let ep = p + size; // entry extent: [p, ep)
    let qid = be32(block, p + 2).ok_or("paqfs: dir entry truncated")?;
    let mode = be32(block, p + 6).ok_or("paqfs: dir entry truncated")?;
    let mtime = be32(block, p + 10).ok_or("paqfs: dir entry truncated")?;
    let length = be32(block, p + 14).ok_or("paqfs: dir entry truncated")?;
    let offset = be32(block, p + 18).ok_or("paqfs: dir entry truncated")?;
    // Three length-prefixed strings (u16 n = strlen+2, then n-2 bytes),
    // each fully contained in the entry extent.
    let mut strs: Vec<Vec<u8>> = Vec::with_capacity(3);
    let mut q = p + 22;
    for _ in 0..3 {
        let Some(n) = be16(block, q).map(|v| v as usize) else {
            return Ok(None);
        };
        // mkpaqfs's putstr writes strlen+2 (>= 2); anything smaller is
        // corrupt — terminate the scan rather than slice q+2..q+n backwards.
        if n < 2 || q + n > ep {
            return Ok(None);
        }
        strs.push(block[q + 2..q + n].to_vec());
        q += n;
    }
    let mut it = strs.into_iter();
    let (Some(name), Some(uid), Some(gid)) = (it.next(), it.next(), it.next()) else {
        return Ok(None); // unreachable: pushed exactly 3 above
    };
    Ok(Some((PaqDir { qid, mode, mtime, length, offset, name, uid, gid }, ep)))
}

impl<'img> Paqfs<'img> {
    /// Parse the 44-byte header (small form only — a big-form image is a
    /// producer mistake this phase never makes) and the 28-byte trailer.
    /// The trailer sha1 is NOT verified (paqfs -v semantics; host fixture
    /// verification covers integrity).
    pub fn open_image(img: &'img [u8]) -> Result<Self> {
        if img.len() < HEADER_SIZE + TRAILER_SIZE {
            return Err("paqfs: image smaller than header+trailer");
        }
        // Big-form header (u16 0x25A9 @0) fails this check too, loudly.
        if be32(img, 0) != Some(HEADER_MAGIC) {
            return Err("paqfs: bad header magic");
        }
        if be16(img, 4) != Some(VERSION) {
            return Err("paqfs: unsupported header version");
        }
        let blocksize = be16(img, 6).ok_or("paqfs: header truncated")? as usize;
        if blocksize == 0 {
            return Err("paqfs: zero blocksize");
        }
        let trailer_off = img.len() - TRAILER_SIZE;
        if be32(img, trailer_off) != Some(TRAILER_MAGIC) {
            return Err("paqfs: bad trailer magic");
        }
        let root_off = be32(img, trailer_off + 4).ok_or("paqfs: trailer truncated")?;
        Ok(Self { img, blocksize, root_off })
    }

    pub fn blocksize(&self) -> usize {
        self.blocksize
    }

    /// Decode the block at byte offset `off` and return its `blocksize`-byte
    /// decoded payload.  NoEnc blocks are ALWAYS blocksize bytes on disk
    /// (mkpaqfs.c:365); DeflateEnc blocks are raw RFC 1951 inflate with the
    /// output hard-capped at blocksize (the decompression-bomb bound).
    /// The adler32 header field covers the DECODED payload for both.
    pub fn read_block(&self, off: u32, expect_type: BlockType) -> Result<Vec<u8>> {
        let off = off as usize;
        let hdr_end = off.checked_add(BLOCK_HEADER_SIZE).ok_or("paqfs: block offset overflow")?;
        if hdr_end > self.img.len() {
            return Err("paqfs: block offset out of image");
        }
        // Big-form block magic (u16 0x91A8 @0) fails this check too, loudly.
        if be32(self.img, off) != Some(BLOCK_MAGIC) {
            return Err("paqfs: bad block magic");
        }
        let size = be16(self.img, off + 4).ok_or("paqfs: block header truncated")? as usize;
        if size > self.blocksize {
            return Err("paqfs: block size exceeds blocksize");
        }
        let btype = BlockType::from_u8(self.img[off + 6])?;
        if btype != expect_type {
            return Err("paqfs: block type mismatch");
        }
        let encoding = self.img[off + 7];
        let adler = be32(self.img, off + 8).ok_or("paqfs: block header truncated")?;
        let decoded: Vec<u8> = match encoding {
            0 => {
                // NoEnc: payload is the full blocksize window on disk.
                let end =
                    hdr_end.checked_add(self.blocksize).ok_or("paqfs: block size overflow")?;
                if end > self.img.len() {
                    return Err("paqfs: block payload out of image");
                }
                self.img[hdr_end..end].to_vec()
            }
            1 => {
                let end = hdr_end.checked_add(size).ok_or("paqfs: block size overflow")?;
                if end > self.img.len() {
                    return Err("paqfs: block payload out of image");
                }
                let v = miniz_oxide::inflate::decompress_to_vec_with_limit(
                    &self.img[hdr_end..end],
                    self.blocksize,
                )
                .map_err(|_| "paqfs: inflate failed")?;
                if v.len() != self.blocksize {
                    return Err("paqfs: inflate length mismatch");
                }
                v
            }
            _ => return Err("paqfs: unknown block encoding"),
        };
        if paq_adler32(&decoded) != adler {
            return Err("paqfs: adler32 mismatch");
        }
        Ok(decoded)
    }

    /// The root directory's PaqDir: the first entry of the trailer-root
    /// DirBlock (paqfs.c:914-938 crafts the fs root from it).
    pub fn root_dir(&self) -> Result<PaqDir> {
        let block = self.read_block(self.root_off, BlockType::DirBlock)?;
        match parse_dir_entry(&block, 0)? {
            Some((entry, _)) => Ok(entry),
            None => Err("paqfs: root dir block has no entries"),
        }
    }

    /// All entries of a directory: chase its PointerBlock (BE u32 offsets,
    /// zero-padded — stop at the first 0, paqfs.c:677/:753-754), scan each
    /// DirBlock's PaqDir entries with checkDirSize bounds discipline.
    pub fn read_dir(&self, dir: &PaqDir) -> Result<Vec<PaqDir>> {
        let ptr = self.read_block(dir.offset, BlockType::PointerBlock)?;
        let mut out = Vec::new();
        for i in 0..self.blocksize / 4 {
            let boff = be32(&ptr, i * 4).ok_or("paqfs: pointer block truncated")?;
            if boff == 0 {
                break;
            }
            let block = self.read_block(boff, BlockType::DirBlock)?;
            let mut p = 0usize;
            while let Some((entry, next)) = parse_dir_entry(&block, p)? {
                out.push(entry);
                p = next;
            }
        }
        Ok(out)
    }

    /// Resolve a `/`-separated path from the root.  An empty (or all-`/`)
    /// path returns the root directory entry.
    pub fn walk(&self, path: &str) -> Result<PaqDir> {
        let mut cur = self.root_dir()?;
        for comp in path.split('/').filter(|c| !c.is_empty()) {
            let entries = self.read_dir(&cur)?;
            let mut found: Option<PaqDir> = None;
            for e in entries {
                if e.name == comp.as_bytes() {
                    found = Some(e);
                    break;
                }
            }
            cur = found.ok_or("paqfs: path not found")?;
        }
        Ok(cur)
    }

    /// Read up to `buf.len()` bytes of `dir`'s file content starting at byte
    /// offset `off`, clamped to `dir.length`; returns the byte count.
    /// DataBlock[i] holds decoded bytes [i*blocksize, (i+1)*blocksize) of the
    /// file (data file offset is 40+textsize, not page-aligned —
    /// byte-granular windows are the contract).
    pub fn read_at(&self, dir: &PaqDir, off: u64, buf: &mut [u8]) -> Result<usize> {
        let length = u64::from(dir.length);
        if off >= length {
            return Ok(0);
        }
        let want = (length - off).min(buf.len() as u64) as usize;
        let bs = self.blocksize as u64;
        let ptr = self.read_block(dir.offset, BlockType::PointerBlock)?;
        let mut done = 0usize;
        while done < want {
            let pos = off + done as u64;
            let blk_idx = (pos / bs) as usize;
            let in_blk = (pos % bs) as usize;
            if blk_idx >= self.blocksize / 4 {
                return Err("paqfs: file offset beyond pointer block");
            }
            let boff = be32(&ptr, blk_idx * 4).ok_or("paqfs: pointer block truncated")?;
            if boff == 0 {
                return Err("paqfs: pointer block shorter than file length");
            }
            let data = self.read_block(boff, BlockType::DataBlock)?;
            let n = (want - done).min(self.blocksize - in_blk);
            buf[done..done + n].copy_from_slice(&data[in_blk..in_blk + n]);
            done += n;
        }
        Ok(done)
    }
}
