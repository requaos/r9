//! Host-side tests for the 05-05 Task 2 devroot raw-image node
//! (`/rootfs.img` — the D-04 two-views counterpart): the kernel walks the
//! paqfs TREE for exec while the userspace paqfs server reads the raw
//! image BYTES through this node (bootr9.c's embed method arg is the
//! literal `/rootfs.img` — the cross-plan contract; the plan's shell
//! grep leg pins both sides).
//!
//! Pinned: the synthesized node's stat shape (0444 regular file whose
//! length is the image size), bounds-checked reads proxying the image
//! slice (paqfs read_at's clamp convention — off ≥ end is EOF = 0, never
//! an error), the named write/exec refusals, and the mount-time
//! collision refusal (an image shipping its own rootfs.img root entry
//! fails mount_initrd loudly rather than shadowing the raw node).
//!
//! The builder is the namespace_test.rs synthetic-paqfs-image pattern
//! (same spec: Harvey sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c}), duplicated
//! per the established test-file convention.

use port::dat::{Chan, OEXEC, OREAD, OWRITE};
use port::devroot;

// ---------------------------------------------------------------------------
// Synthetic paqfs image builder (the namespace_test.rs pattern, same spec)
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

/// Harvey libflate adler32(0, buf, n): the seed-0 variant (see paqfs_test.rs).
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
        self.img.extend_from_slice(&t);
        self.img
    }
}

/// Encode a PaqDir entry (paqfs_test.rs's put_dir_entry verbatim shape).
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

enum Node {
    File(&'static str, &'static [u8]),
    Dir(&'static str, &'static [Node]),
}

/// Emit a node bottom-up; returns (qid, mode, length, ptr_offset, name).
fn emit(b: &mut ImageBuilder, node: &Node, qid: &mut u32) -> (u32, u32, u32, u32, &'static str) {
    *qid += 1;
    let my_qid = *qid;
    match node {
        Node::File(name, content) => {
            assert!(content.len() <= BSIZE);
            let data_off = b.add_block(TYPE_DATA, content);
            let mut ptr = vec![0u8; BSIZE];
            for (i, &o) in [data_off].iter().enumerate() {
                be32put(&mut ptr, i * 4, o);
            }
            let ptr_off = b.add_block(TYPE_PTR, &ptr);
            (my_qid, 0o644, content.len() as u32, ptr_off, name)
        }
        Node::Dir(name, children) => {
            let mut ents = vec![0u8; BSIZE];
            let mut p = 0;
            for child in children.iter() {
                let (cqid, cmode, clen, coff, cname) = emit(b, child, qid);
                p += put_dir_entry(
                    &mut ents,
                    p,
                    cqid,
                    cmode,
                    clen,
                    coff,
                    cname.as_bytes(),
                    b"glenda",
                    b"glenda",
                );
            }
            let ents_off = b.add_block(TYPE_DIR, &ents);
            let mut ptr = vec![0u8; BSIZE];
            be32put(&mut ptr, 0, ents_off);
            let ptr_off = b.add_block(TYPE_PTR, &ptr);
            (my_qid, DMDIR | 0o755, 0, ptr_off, name)
        }
    }
}

/// Build the image whose ROOT carries `children` (the root dir itself is
/// the synthesized top-level entry block, namespace_test.rs's shape).
fn image_with(label: &str, children: &'static [Node]) -> Vec<u8> {
    let mut b = ImageBuilder::new(label);
    let mut qid = 1u32; // root's own qid is 1 (children from 2)
    let root = Node::Dir("root", children);
    let (rqid, rmode, rlen, roff, rname) = emit(&mut b, &root, &mut qid);
    let mut rootblk = vec![0u8; BSIZE];
    put_dir_entry(&mut rootblk, 0, rqid, rmode, rlen, roff, rname.as_bytes(), b"glenda", b"glenda");
    let root_off = b.add_block(TYPE_DIR, &rootblk);
    b.finish(root_off)
}

// ---------------------------------------------------------------------------
// Shared fixtures (mount_initrd is idempotent over the one shared GOOD
// image slice — the namespace_test.rs convention; the bad image never
// publishes, so the parallel tests are order-independent)
// ---------------------------------------------------------------------------

static GOOD_IMG: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

fn good_image() -> &'static [u8] {
    GOOD_IMG.get_or_init(|| {
        image_with(
            "rawimg-ok",
            &[Node::Dir("boot", &[Node::File("boot", b"#!/boot/boot -- the tree's own file\n")])],
        )
    })
}

fn mount_good() -> devroot::DevRoot {
    devroot::mount_initrd(good_image()).expect("the good image mounts")
}

// ---------------------------------------------------------------------------
// The raw-image node
// ---------------------------------------------------------------------------

/// The synthesized /rootfs.img node: resolves from the attach root, stats
/// as a 0444 regular file whose length is the image size, and reads proxy
/// the image slice (bounds-checked; EOF = 0 at/past the end, per the
/// paqfs read_at clamp convention).
#[test]
fn rawimg_node_walk_stat_read_and_bounds() {
    let root = mount_good();
    let img = root.image();
    let attach = root.attach_root().expect("attach");

    let c = root.walk(&attach, b"rootfs.img").expect("the node synthesizes at the root");
    assert_eq!(c.qid.path, u64::from(u32::MAX), "the reserved qid");
    assert_eq!(c.qid.typ, 0x00, "a regular file (QTFILE)");
    assert_eq!(c.path.as_slice(), b"/rootfs.img", "the contracted path literal");

    // stat: name, 0444, length = the image size (stat(5) wire: qid.typ at
    // 8, mode at 21, length at 33, name length-prefixed at 41).
    let mut sb = [0u8; 256];
    let n = root.stat(&c, &mut sb).expect("stat the raw node") as usize;
    let name_len = u16::from_le_bytes([sb[41], sb[42]]) as usize;
    assert_eq!(&sb[43..43 + name_len], b"rootfs.img");
    assert_eq!(u32::from_le_bytes(sb[21..25].try_into().unwrap()), 0o444, "read-only mode");
    assert_eq!(
        u64::from_le_bytes(sb[33..41].try_into().unwrap()),
        img.len() as u64,
        "stat length is the image size"
    );
    assert_eq!(n, 41 + (2 + name_len) + 2 * (2 + 6), "wire byte count (convD2M)");

    // Reads proxy the image bytes: the paqfs header magic (BE) heads the
    // slice, and the streaming offset advances.
    let mut opened = root.open(&c, OREAD).expect("open OREAD");
    let mut buf = [0u8; 4];
    let n = root.read(&mut opened, &mut buf, u64::MAX).expect("stream read 0");
    assert_eq!(n, 4);
    assert_eq!(&buf, &img[0..4], "the read proxies the image slice from byte 0");
    assert_eq!(&buf, &[0x52, 0x9a, 0xb1, 0x2b], "the paqfs header magic heads the image");
    let n = root.read(&mut opened, &mut buf, u64::MAX).expect("stream read 1");
    assert_eq!(&buf, &img[4..8], "the streaming offset advances");
    assert_eq!(n, 4);

    // Bounds: a read at or past the end is EOF = 0 (never an error); a
    // read spanning the end clamps to the image's last bytes.
    let n = root.read(&mut opened, &mut buf, img.len() as u64).expect("read at end");
    assert_eq!(n, 0, "EOF at the image end");
    let n = root.read(&mut opened, &mut buf, img.len() as u64 + 4096).expect("read past end");
    assert_eq!(n, 0, "EOF past the image end");
    let mut tail = [0u8; 8];
    let n = root.read(&mut opened, &mut tail, img.len() as u64 - 4).expect("spanning read");
    assert_eq!(n, 4, "clamped at the image end");
    assert_eq!(&tail[..4], &img[img.len() - 4..], "the image's final bytes");

    // The node synthesizes ONLY at the root: a deeper walk never finds it.
    let bootdir = root.walk(&attach, b"boot").expect("the real dir walks");
    assert_eq!(bootdir.qid.typ, 0x80, "a directory");
    assert!(root.walk(&bootdir, b"rootfs.img").is_err(), "no synthesis below the root");
}

/// The node is read-only, loudly: OWRITE/ORDWR opens fail naming
/// /rootfs.img; OEXEC names the raw image; and the namec dispatch's
/// OTRUNC rejection covers the trunc path (devroot is read-only).
#[test]
fn rawimg_node_refuses_write_and_exec_loudly() {
    let root = mount_good();
    let attach = root.attach_root().expect("attach");
    let c = root.walk(&attach, b"rootfs.img").expect("the node resolves");

    let err = root.open(&c, OWRITE).err().expect("OWRITE refused");
    assert!(err.contains("/rootfs.img") && err.contains("read-only"), "named refusal: {err}");
    let err = root.open(&c, port::dat::ORDWR).err().expect("ORDWR refused");
    assert!(err.contains("/rootfs.img"), "named refusal: {err}");
    let err = root.open(&c, OEXEC).err().expect("OEXEC refused");
    assert!(err.contains("exec"), "named exec refusal: {err}");

    // Through the namec dispatch: OTRUNC on any devroot file errors
    // before the device (the read-only-fs contract).
    assert!(
        port::namec::open_chan(&c, OREAD | port::dat::OTRUNC).is_err(),
        "OTRUNC refused at the dispatch"
    );

    // A write-side refusal leaves the node's bytes untouched.
    let mut opened = root.open(&c, OREAD).expect("open OREAD");
    let mut buf = [0u8; 4];
    root.read(&mut opened, &mut buf, u64::MAX).expect("re-read");
    assert_eq!(&buf, &good_image()[0..4]);
}

/// The mount-time collision refusal (05-05's hardening): an image whose
/// ROOT ships its own rootfs.img entry fails mount_initrd with a named
/// error — one view must never shadow the other silently.  A NESTED
/// rootfs.img (below the root) is harmless: synthesis only happens at
/// the root, so no collision is possible there.
#[test]
fn rawimg_name_collision_fails_mount_loudly() {
    let good = mount_good();
    let good_attach = good.attach_root().expect("attach good");

    let bad = image_with(
        "rawimg-bad",
        &[Node::File("rootfs.img", b"a tree entry masquerading as the raw image\n")],
    );
    let err = devroot::mount_initrd(Box::leak(bad.into_boxed_slice())).err().expect("collision");
    assert!(err.contains("rootfs.img"), "the refusal names the entry: {err}");
    assert!(err.contains("refusing to mount"), "the refusal is loud: {err}");

    // The failed mount published nothing: the good image still serves,
    // and the synthesized node is still the raw view.
    let c = good.walk(&good_attach, b"rootfs.img").expect("raw node intact");
    assert_eq!(c.qid.path, u64::from(u32::MAX));

    // A nested entry named rootfs.img collides with nothing (the node
    // lives at the root only) — the mount succeeds.
    let nested = image_with(
        "rawimg-nested",
        &[Node::Dir("boot", &[Node::File("rootfs.img", b"a nested namesake\n")])],
    );
    devroot::mount_initrd(Box::leak(nested.into_boxed_slice()))
        .expect("a nested namesake mounts fine");
    // Re-publish the shared good image for order-independence.
    mount_good();
}
