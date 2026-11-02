//! Host-side tests for the no_std paqfs reader (phase 04-02, D-02).
//!
//! Byte-level format verification against real mkpaqfs output: the committed
//! 04-01 fixture (probe-tree.paq) plus an in-test synthetic image built to the
//! same spec (Harvey sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c,paqfs.c}).

use port::paqfs::{BlockType, Paqfs};

static FIXTURE: &[u8] = include_bytes!("fixtures/probe-tree.paq");
static HELLO_TXT: &[u8] = include_bytes!("fixtures/probe-tree/hello.txt");
static NESTED_TXT: &[u8] = include_bytes!("fixtures/probe-tree/dir/nested.txt");

// ---------------------------------------------------------------------------
// Format constants + synthetic-image builder (mkpaqfs-compatible small form)
// ---------------------------------------------------------------------------

const BSIZE: usize = 512;
const HEADER_SIZE: usize = 44;
const TRAILER_SIZE: usize = 28;
const BLOCK_HDR: usize = 12;

const HEADER_MAGIC: u32 = 0x529AB12B;
const BLOCK_MAGIC: u32 = 0x198A1CBF;
const TRAILER_MAGIC: u32 = 0x6B46E688;

const TYPE_DIR: u8 = 0;
const TYPE_DATA: u8 = 1;
const TYPE_PTR: u8 = 2;
const ENC_NO: u8 = 0;

const DMDIR: u32 = 0x8000_0000;

/// Harvey libflate adler32(0, buf, n): the seed-0 variant (s1 = s2 = 0), NOT
/// the RFC 1950 seed (s1 = 1).  mkpaqfs writes and paqfs checks this variant
/// (libflate/adler.c with adler=0), so the test builder matches it here
/// independently of the crate under test.
fn adler32_seed0(buf: &[u8]) -> u32 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    for &b in buf {
        s1 = (s1 + u32::from(b)) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    (s2 << 16) | s1
}

fn be16put(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_be_bytes());
}

fn be32put(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_be_bytes());
}

struct ImageBuilder {
    img: Vec<u8>,
}

impl ImageBuilder {
    fn new(label: &str) -> Self {
        let mut img = vec![0u8; HEADER_SIZE];
        be32put(&mut img, 0, HEADER_MAGIC);
        be16put(&mut img, 4, 1); // version
        be16put(&mut img, 6, BSIZE as u16);
        img[12..12 + label.len()].copy_from_slice(label.as_bytes());
        Self { img }
    }

    /// Append a NoEnc block holding `payload` (<= BSIZE bytes, zero-padded to
    /// BSIZE: NoEnc blocks are ALWAYS blocksize bytes on disk, mkpaqfs.c:365)
    /// and return its byte offset.
    fn add_block(&mut self, typ: u8, payload: &[u8]) -> u32 {
        assert!(payload.len() <= BSIZE);
        let off = self.img.len() as u32;
        let mut full = vec![0u8; BSIZE];
        full[..payload.len()].copy_from_slice(payload);
        let mut hdr = [0u8; BLOCK_HDR];
        be32put(&mut hdr, 0, BLOCK_MAGIC);
        be16put(&mut hdr, 4, BSIZE as u16);
        hdr[6] = typ;
        hdr[7] = ENC_NO;
        be32put(&mut hdr, 8, adler32_seed0(&full));
        self.img.extend_from_slice(&hdr);
        self.img.extend_from_slice(&full);
        off
    }

    fn finish(mut self, root_off: u32) -> Vec<u8> {
        let mut t = [0u8; TRAILER_SIZE];
        be32put(&mut t, 0, TRAILER_MAGIC);
        be32put(&mut t, 4, root_off);
        // sha1 left zeroed: the kernel reader does not verify it
        // (paqfs -v semantics; RESEARCH D-02).
        self.img.extend_from_slice(&t);
        self.img
    }
}

/// Encode a PaqDir entry at `p` in a zero-filled block payload; return size.
/// Layout: size u16, qid/mode/mtime/length/offset u32s, then three
/// length-prefixed strings (u16 n = strlen+2, then n-2 bytes).
fn put_dir_entry(
    payload: &mut [u8],
    p: usize,
    qid: u32,
    mode: u32,
    length: u32,
    offset: u32,
    name: &[u8],
    uid: &[u8],
    gid: &[u8],
) -> usize {
    let size = 28 + name.len() + uid.len() + gid.len();
    be16put(payload, p, size as u16);
    be32put(payload, p + 2, qid);
    be32put(payload, p + 6, mode);
    be32put(payload, p + 10, 0); // mtime
    be32put(payload, p + 14, length);
    be32put(payload, p + 18, offset);
    let mut q = p + 22;
    for s in [name, uid, gid] {
        be16put(payload, q, (s.len() + 2) as u16);
        payload[q + 2..q + 2 + s.len()].copy_from_slice(s);
        q += 2 + s.len();
    }
    assert_eq!(q - p, size);
    size
}

/// PointerBlock payload: BE u32 block offsets, zero-padded (readers stop at
/// the first 0 — paqfs.c:677/:753-754).
fn put_ptr_block(payload: &mut [u8], offs: &[u32]) {
    for (i, &o) in offs.iter().enumerate() {
        be32put(payload, i * 4, o);
    }
}

static SYNTH_HELLO: &[u8] = b"hello synthetic paqfs\n";

/// A minimal, fully format-legal image: header, one one-block file
/// "hello.txt" at the root, a root directory holding it, trailer.
/// All blocks NoEnc.
fn synthetic_image() -> Vec<u8> {
    let mut b = ImageBuilder::new("synthetic");
    let data_off = b.add_block(TYPE_DATA, SYNTH_HELLO);
    let mut ptr = vec![0u8; BSIZE];
    put_ptr_block(&mut ptr, &[data_off]);
    let file_ptr_off = b.add_block(TYPE_PTR, &ptr);
    let mut rootents = vec![0u8; BSIZE];
    put_dir_entry(
        &mut rootents,
        0,
        2,
        0o644,
        SYNTH_HELLO.len() as u32,
        file_ptr_off,
        b"hello.txt",
        b"u",
        b"g",
    );
    let root_ents_off = b.add_block(TYPE_DIR, &rootents);
    let mut rptr = vec![0u8; BSIZE];
    put_ptr_block(&mut rptr, &[root_ents_off]);
    let root_ptr_off = b.add_block(TYPE_PTR, &rptr);
    let mut rootblk = vec![0u8; BSIZE];
    put_dir_entry(
        &mut rootblk,
        0,
        1,
        DMDIR | 0o755,
        0,
        root_ptr_off,
        b"root",
        b"u",
        b"g",
    );
    let root_off = b.add_block(TYPE_DIR, &rootblk);
    b.finish(root_off)
}

/// First-block BlockType from fixture bytes (re-bake robust).
fn first_block_type(img: &[u8]) -> BlockType {
    match img[HEADER_SIZE + 6] {
        0 => BlockType::DirBlock,
        1 => BlockType::DataBlock,
        2 => BlockType::PointerBlock,
        t => panic!("fixture block at {HEADER_SIZE} has unknown type {t}"),
    }
}

// ---------------------------------------------------------------------------
// Test 1: synthetic NoEnc image — walk + byte-exact read_at
// ---------------------------------------------------------------------------

#[test]
fn synthetic_noenc_walk_and_read() {
    let img = synthetic_image();
    let fs = Paqfs::open_image(&img)
        .expect("open_image should succeed on a well-formed synthetic image");
    assert_eq!(fs.blocksize(), BSIZE);

    let dir = fs.walk("hello.txt").expect("walk should find hello.txt at the root");
    assert_eq!(dir.name.as_slice(), b"hello.txt".as_slice());
    assert_eq!(dir.length as usize, SYNTH_HELLO.len());

    let mut buf = vec![0u8; SYNTH_HELLO.len()];
    let n = fs.read_at(&dir, 0, &mut buf).expect("read_at should succeed");
    assert_eq!(n, SYNTH_HELLO.len());
    assert_eq!(&buf[..], SYNTH_HELLO);

    // Byte-granular window: mid-file offset, short buffer.
    let mut win = [0u8; 5];
    let n = fs.read_at(&dir, 7, &mut win).expect("window read should succeed");
    assert_eq!(n, 5);
    assert_eq!(&win[..], &SYNTH_HELLO[7..12]);

    // Window clamps to PaqDir.length (block zero-pad must never leak).
    let mut big = vec![0u8; BSIZE];
    let n = fs
        .read_at(&dir, (SYNTH_HELLO.len() - 4) as u64, &mut big)
        .expect("tail read should succeed");
    assert_eq!(n, 4);
    assert_eq!(&big[..4], &SYNTH_HELLO[SYNTH_HELLO.len() - 4..]);
}

// ---------------------------------------------------------------------------
// Test 2: committed mkpaqfs fixture — walk/read known files, both encodings
// ---------------------------------------------------------------------------

#[test]
fn committed_fixture_walk_and_read() {
    let fs = Paqfs::open_image(FIXTURE).expect("open_image should accept the mkpaqfs fixture");

    let hello = fs.walk("hello.txt").expect("walk should find hello.txt");
    assert_eq!(hello.length as usize, HELLO_TXT.len());
    let mut buf = vec![0u8; HELLO_TXT.len()];
    let n = fs.read_at(&hello, 0, &mut buf).expect("read_at hello.txt should succeed");
    assert_eq!(n, HELLO_TXT.len());
    assert_eq!(&buf[..], HELLO_TXT);

    let nested = fs.walk("dir/nested.txt").expect("walk should recurse into dir/");
    assert_eq!(nested.length as usize, NESTED_TXT.len());
    let mut nb = vec![0u8; NESTED_TXT.len()];
    let n = fs.read_at(&nested, 0, &mut nb).expect("read_at nested.txt should succeed");
    assert_eq!(n, NESTED_TXT.len());
    assert_eq!(&nb[..], NESTED_TXT);

    // Byte-granular window into a deflate-decoded block.
    let mut win = [0u8; 100];
    let n = fs.read_at(&hello, 50, &mut win).expect("window read should succeed");
    assert_eq!(n, 100);
    assert_eq!(&win[..], &HELLO_TXT[50..150]);

    // The bake was deliberately compressible: at least one DataBlock in the
    // fixture MUST be DeflateEnc, or the inflate path is not being exercised.
    // Scan block headers linearly (re-bake robust — no hardcoded offsets).
    let bsz = u16::from_be_bytes([FIXTURE[6], FIXTURE[7]]) as usize;
    let mut off = HEADER_SIZE;
    let end = FIXTURE.len() - TRAILER_SIZE;
    let mut saw_deflate_data = false;
    while off + BLOCK_HDR <= end {
        let magic = u32::from_be_bytes(FIXTURE[off..off + 4].try_into().unwrap());
        assert_eq!(magic, BLOCK_MAGIC, "fixture block at {off} has bad magic");
        let size = u16::from_be_bytes([FIXTURE[off + 4], FIXTURE[off + 5]]) as usize;
        let (typ, enc) = (FIXTURE[off + 6], FIXTURE[off + 7]);
        if typ == 1 && enc == 1 {
            saw_deflate_data = true;
        }
        let disk = if enc == 1 { size } else { bsz };
        off += BLOCK_HDR + disk;
    }
    assert_eq!(off, end, "fixture block walk must land exactly on the trailer");
    assert!(saw_deflate_data, "fixture must contain a DeflateEnc DataBlock");
}

#[test]
fn committed_fixture_root_listing() {
    let fs = Paqfs::open_image(FIXTURE).expect("open_image should accept the mkpaqfs fixture");
    let root = fs.root_dir().expect("root_dir should parse the trailer-root DirBlock");
    let entries = fs.read_dir(&root).expect("read_dir on the root should succeed");
    assert_eq!(entries.len(), 2, "fixture root yields exactly hello.txt and dir");
    let names: Vec<&[u8]> = entries.iter().map(|e| e.name.as_slice()).collect();
    assert!(names.contains(&b"hello.txt".as_slice()));
    assert!(names.contains(&b"dir".as_slice()));
}

// ---------------------------------------------------------------------------
// Test 3: negatives — corrupt magics, unknown encoding, out-of-image offsets
// Every failure is a typed Err; nothing panics (trap-context safety, T-04-03).
// ---------------------------------------------------------------------------

#[test]
fn corrupt_inputs_error_no_panic() {
    // Header magic corrupt.
    let mut bad = FIXTURE.to_vec();
    bad[0] ^= 0xFF;
    assert!(Paqfs::open_image(&bad).is_err(), "corrupt header magic must be rejected");

    // Big-form header magic (0x25A9 @0) is not supported by this reader.
    let mut big = FIXTURE.to_vec();
    big[0] = 0x25;
    big[1] = 0xA9;
    assert!(Paqfs::open_image(&big).is_err(), "big-form header must be rejected loudly");

    // Trailer magic corrupt.
    let mut badt = FIXTURE.to_vec();
    let t = badt.len() - TRAILER_SIZE;
    badt[t] ^= 0xFF;
    assert!(Paqfs::open_image(&badt).is_err(), "corrupt trailer magic must be rejected");

    let fs = Paqfs::open_image(FIXTURE).unwrap();

    // Block offset past the image end.
    assert!(fs.read_block(FIXTURE.len() as u32 + 4096, BlockType::DirBlock).is_err());
    // Block header truncated by the image end.
    assert!(fs.read_block((FIXTURE.len() - 5) as u32, BlockType::DirBlock).is_err());

    // Block magic corrupt (first block, type read from the pristine fixture).
    let mut badm = FIXTURE.to_vec();
    badm[HEADER_SIZE] ^= 0xFF;
    let fsm = Paqfs::open_image(&badm).unwrap();
    assert!(fsm.read_block(HEADER_SIZE as u32, first_block_type(FIXTURE)).is_err());

    // Unknown encoding byte (7) — paqfs.c:963-965 loud-default precedent.
    let mut bade = FIXTURE.to_vec();
    bade[HEADER_SIZE + 7] = 7;
    let fse = Paqfs::open_image(&bade).unwrap();
    assert!(fse.read_block(HEADER_SIZE as u32, first_block_type(FIXTURE)).is_err());

    // Unknown block type byte.
    let mut badt2 = FIXTURE.to_vec();
    badt2[HEADER_SIZE + 6] = 9;
    let fst = Paqfs::open_image(&badt2).unwrap();
    assert!(fst.read_block(HEADER_SIZE as u32, BlockType::DirBlock).is_err());
}

// ---------------------------------------------------------------------------
// Test 4: PaqDir size-prefix bounds — checkDirSize semantics.  An entry whose
// size overruns its block terminates the scan (walk => Err, never an
// out-of-slice read); a zero size terminates the same way.
// ---------------------------------------------------------------------------

#[test]
fn paqdir_size_overrun_terminates_scan() {
    // Root-contents DirBlock whose first entry size prefix (0xFFFF) overruns
    // the 512-byte block.
    let mut b = ImageBuilder::new("corrupt");
    let mut bogus = vec![0u8; BSIZE];
    be16put(&mut bogus, 0, 0xFFFF);
    let root_ents_off = b.add_block(TYPE_DIR, &bogus);
    let mut ptr = vec![0u8; BSIZE];
    put_ptr_block(&mut ptr, &[root_ents_off]);
    let root_ptr_off = b.add_block(TYPE_PTR, &ptr);
    let mut rootblk = vec![0u8; BSIZE];
    put_dir_entry(&mut rootblk, 0, 1, DMDIR | 0o755, 0, root_ptr_off, b"root", b"u", b"g");
    let root_off = b.add_block(TYPE_DIR, &rootblk);
    let img = b.finish(root_off);

    let fs = Paqfs::open_image(&img).expect("open_image should succeed (structure is legal)");
    assert!(fs.walk("hello.txt").is_err(), "overrun entry must terminate the scan");

    // Zero-size first entry: same termination, different byte pattern.
    let mut b2 = ImageBuilder::new("corrupt");
    let zeros = vec![0u8; BSIZE];
    let root_ents_off = b2.add_block(TYPE_DIR, &zeros);
    let mut ptr2 = vec![0u8; BSIZE];
    put_ptr_block(&mut ptr2, &[root_ents_off]);
    let root_ptr_off = b2.add_block(TYPE_PTR, &ptr2);
    let mut rootblk2 = vec![0u8; BSIZE];
    put_dir_entry(&mut rootblk2, 0, 1, DMDIR | 0o755, 0, root_ptr_off, b"root", b"u", b"g");
    let root_off = b2.add_block(TYPE_DIR, &rootblk2);
    let img2 = b2.finish(root_off);

    let fs2 = Paqfs::open_image(&img2).expect("open_image should succeed (structure is legal)");
    assert!(fs2.walk("hello.txt").is_err(), "zero-size entry must terminate the scan");
}
