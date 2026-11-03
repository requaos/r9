//! Host-side tests for the 05-04 namespace core (D-03): namec-style path
//! resolution with the hand-built device registry, the pgrp mount table,
//! union-directory resolution in 9k order, and the rfork group semantics.
//!
//! Semantics are pinned against the 9k donor (sys/src/9k/port/chan.c
//! namec/walk + cmount/cunmount, pgrp.c pgrpcpy) and the frozen flag
//! values (sys/include/libc.h:558-564 M*, :613-623 RF*).
//!
//! DEVIATION RECORDED (05-04 Task 2): the plan's <behavior> bullet says
//! "rfork(RFNAMEG) shares the same mount table; rfork(RFCNAMEG)
//! deep-copies" — that is INVERTED relative to fork(2) ("RFNAMEG: the new
//! process inherits a COPY of the parent's name space; otherwise the new
//! process SHARES the parent's name space.  RFCNAMEG: the new process
//! starts with a CLEAN name space"), 9k's sysproc.c sysrfork (RFNAMEG →
//! newpgrp+pgrpcpy; RFCNAMEG → newpgrp empty; neither → incref-share), and
//! the phase's own binding 05-03 finding ("newns's RFCNAMEG wipe discards
//! boot's mount table").  These tests pin the DONOR/fork(2) semantics:
//! neither-flag share, RFNAMEG copy, RFCNAMEG wipe.

use std::sync::Arc;

use port::dat::{Chan, DEV_CONS, DEV_PROC, DEV_ROOT};
use port::devroot;
use port::mcslock::{Lock, LockNode};
use port::namec::{self, NamecMode};
use port::pgrp::{self, Pgrp};

// ---------------------------------------------------------------------------
// Synthetic paqfs image (the paqfs_test.rs builder pattern, same spec:
// Harvey sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c}) — tree:
//   root
//   ├── boot/ → boot (file)
//   ├── dev/  → cons (file), kmsg (file)
//   └── proc/ (empty dir)
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

fn put_ptr_block(payload: &mut [u8], offs: &[u32]) {
    for (i, &o) in offs.iter().enumerate() {
        be32put(payload, i * 4, o);
    }
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
            put_ptr_block(&mut ptr, &[data_off]);
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
            put_ptr_block(&mut ptr, &[ents_off]);
            let ptr_off = b.add_block(TYPE_PTR, &ptr);
            (my_qid, DMDIR | 0o755, 0, ptr_off, name)
        }
    }
}

const BOOT_CONTENT: &[u8] = b"#!/boot/boot -- the real chain pid 1\n";
const CONS_CONTENT: &[u8] = b"devroot's cons file\n";
const KMSG_CONTENT: &[u8] = b"devroot's kmsg file\n";

static TREE: &[Node] = &[
    Node::Dir("boot", &[Node::File("boot", BOOT_CONTENT)]),
    Node::Dir(
        "dev",
        &[Node::File("cons", CONS_CONTENT), Node::File("kmsg", KMSG_CONTENT)],
    ),
    Node::Dir("proc", &[]),
];

static TEST_IMG: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

fn image() -> &'static [u8] {
    TEST_IMG.get_or_init(|| {
        let mut b = ImageBuilder::new("ns-test");
        let mut qid = 1u32; // root's own qid is 1 (children from 2)
        let root = Node::Dir("root", TREE);
        let (rqid, rmode, rlen, roff, rname) = emit(&mut b, &root, &mut qid);
        let mut rootblk = vec![0u8; BSIZE];
        put_dir_entry(
            &mut rootblk,
            0,
            rqid,
            rmode,
            rlen,
            roff,
            rname.as_bytes(),
            b"glenda",
            b"glenda",
        );
        let root_off = b.add_block(TYPE_DIR, &rootblk);
        b.finish(root_off)
    })
}

// ---------------------------------------------------------------------------
// Shared fixtures: the mounted devroot tree, a root slash chan, and a fresh
// pgrp.  mount_initrd is idempotent over the one shared image slice (every
// test publishes the SAME bytes).
// ---------------------------------------------------------------------------

fn slash() -> Chan {
    devroot::mount_initrd(image()).expect("mount_initrd");
    devroot::root().expect("root mounted").attach_root().expect("attach root")
}

fn resolve(pg: &Pgrp, slash: &Chan, path: &str) -> port::Result<Chan> {
    namec::namec(path.as_bytes(), slash, slash, pg, NamecMode::Open)
}

// ---------------------------------------------------------------------------
// namec: registry + devroot walk
// ---------------------------------------------------------------------------

#[test]
fn namec_resolves_device_path_through_registry() {
    let pg = Pgrp::new();
    let slash = slash();

    let c = resolve(&pg, &slash, "#c/cons").expect("#c/cons must resolve through the registry");
    assert_eq!(c.dev, DEV_CONS, "#c/cons must land on the devcons device");
    assert_eq!(c.qid.path, 1, "cons is qid path 1 (the preopen convention)");

    let p = resolve(&pg, &slash, "#p").expect("#p must resolve to the devproc root");
    assert_eq!(p.dev, DEV_PROC);

    assert!(resolve(&pg, &slash, "#z").is_err(), "unknown device char must error loudly");
}

#[test]
fn namec_devroot_multi_component_walk() {
    let pg = Pgrp::new();
    let slash = slash();

    let c = resolve(&pg, &slash, "/boot/boot").expect("/boot/boot must resolve");
    assert_eq!(c.dev, DEV_ROOT);
    assert_eq!(c.path.as_slice(), b"/boot/boot");
    let dir = devroot::root().unwrap().entry(&c).expect("devroot chan carries its paqfs entry");
    assert_eq!(dir.name.as_slice(), b"boot");
    assert_eq!(dir.length as usize, BOOT_CONTENT.len());

    // A relative path walks from dot (here dot == the root).
    let rel = namec::namec(b"boot/boot", &slash, &slash, &pg, NamecMode::Open)
        .expect("relative boot/boot must resolve from dot");
    assert_eq!(rel.qid.path, c.qid.path);

    assert!(resolve(&pg, &slash, "/boot/nosuch").is_err());
    assert!(resolve(&pg, &slash, "/boot/boot/cons").is_err(), "walk through a file must error");
}

// ---------------------------------------------------------------------------
// bind + union resolution (9k order: MBEFORE, underlying, MAFTER)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn bind_locked(
    pg: &mut Pgrp,
    slash: &Chan,
    src: &str,
    target: &str,
    flag: u32,
) -> port::Result<u64> {
    let s = namec::namec(src.as_bytes(), slash, slash, pg, NamecMode::BindSource)?;
    let t = namec::namec(target.as_bytes(), slash, slash, pg, NamecMode::MountTarget)?;
    pg.bind(s, &t, flag)
}

#[test]
fn bind_mbefore_then_resolution_prefers_mounted() {
    let mut pg = Pgrp::new();
    let slash = slash();

    bind_locked(&mut pg, &slash, "#c", "/dev", pgrp::MBEFORE).expect("bind #c /dev MBEFORE");

    // The mounted tree wins over the underlying /dev's own cons file.
    let c = resolve(&pg, &slash, "/dev/cons").expect("/dev/cons through the union");
    assert_eq!(c.dev, DEV_CONS, "MBEFORE-mounted #c must shadow the underlying /dev/cons");

    // The underlying directory remains in the union: names only it has
    // still resolve through devroot.
    let k = resolve(&pg, &slash, "/dev/kmsg").expect("underlying /dev/kmsg still resolves");
    assert_eq!(k.dev, DEV_ROOT);
}

#[test]
fn bind_mrepl_replaces_union() {
    let mut pg = Pgrp::new();
    let slash = slash();

    bind_locked(&mut pg, &slash, "#p", "/dev", pgrp::MREPL).expect("bind #p /dev MREPL");

    // MREPL removes the underlying directory from the union: devroot's
    // /dev/cons and /dev/kmsg are both unreachable now.
    assert!(resolve(&pg, &slash, "/dev/cons").is_err(), "MREPL must replace the underlying dir");
    assert!(resolve(&pg, &slash, "/dev/kmsg").is_err());

    let n = resolve(&pg, &slash, "/dev/123/note").expect("the MREPL mount serves its own names");
    assert_eq!(n.dev, DEV_PROC);
}

#[test]
fn bind_mafter_appends_after_underlying() {
    let mut pg = Pgrp::new();
    let slash = slash();

    bind_locked(&mut pg, &slash, "#p", "/dev", pgrp::MAFTER).expect("bind #p /dev MAFTER");

    // Underlying first: /dev/cons resolves through devroot, not the union tail.
    let c = resolve(&pg, &slash, "/dev/cons").expect("/dev/cons resolves");
    assert_eq!(c.dev, DEV_ROOT, "MAFTER entries resolve only after the underlying dir");

    // A name only the MAFTER entry serves still resolves (fallback order).
    let n = resolve(&pg, &slash, "/dev/123/note").expect("MAFTER tail fallback");
    assert_eq!(n.dev, DEV_PROC);
}

#[test]
fn union_walk_order_mbefore_underlying_mafter() {
    let mut pg = Pgrp::new();
    let slash = slash();

    bind_locked(&mut pg, &slash, "#c", "/dev", pgrp::MBEFORE).expect("bind #c MBEFORE");
    bind_locked(&mut pg, &slash, "#p", "/dev", pgrp::MAFTER).expect("bind #p MAFTER");

    // 9k order: MBEFORE entries first, then the underlying dir, then MAFTER.
    assert_eq!(resolve(&pg, &slash, "/dev/cons").unwrap().dev, DEV_CONS, "MBEFORE head first");
    assert_eq!(resolve(&pg, &slash, "/dev/kmsg").unwrap().dev, DEV_ROOT, "underlying second");
    assert_eq!(resolve(&pg, &slash, "/dev/123/note").unwrap().dev, DEV_PROC, "MAFTER tail last");
}

#[test]
fn bind_to_nonexistent_target_errors() {
    let mut pg = Pgrp::new();
    let slash = slash();

    // Pitfall-8 contract: MCREATE permits later creation IN the mounted
    // directory; it does not conjure the mount target itself.
    assert!(bind_locked(&mut pg, &slash, "#c", "/nosuchdir", pgrp::MBEFORE).is_err());
    assert!(bind_locked(&mut pg, &slash, "#c", "/nosuchdir", pgrp::MBEFORE | pgrp::MCREATE).is_err());
    assert!(bind_locked(&mut pg, &slash, "#c", "/dev/kmsg", pgrp::MAFTER).is_err(),
        "a union bind (non-MREPL) onto a non-directory must error (9k cmount)");
}

// ---------------------------------------------------------------------------
// rfork group semantics (fork(2) + 9k sysrfork — see the header deviation
// note: the plan's behavior bullet inverted RFNAMEG/RFCNAMEG)
// ---------------------------------------------------------------------------

#[test]
fn rfork_group_semantics_share_copy_wipe() {
    let slash = slash();

    // neither RFNAMEG nor RFCNAMEG (with RFPROC): the child SHARES the
    // parent's group — one Arc, one table; a bind through the child's
    // handle is visible through the parent's.
    let parent = Arc::new(Lock::new("pgrp-test", Pgrp::new()));
    let child = parent.clone(); // the Arc-share (proc.rs wires the flags)
    {
        let node = LockNode::new();
        let mut guard = child.lock(&node);
        bind_locked(&mut guard, &slash, "#c", "/dev", pgrp::MBEFORE).expect("child bind");
    }
    {
        let node = LockNode::new();
        let guard = parent.lock(&node);
        assert_eq!(
            resolve(&guard, &slash, "/dev/cons").unwrap().dev,
            DEV_CONS,
            "shared group: the child's bind is visible to the parent"
        );
    }

    // RFNAMEG: the child inherits a COPY of the parent's group — the
    // child's later binds are invisible to the parent (9k pgrpcpy).
    let node = LockNode::new();
    let copied = Arc::new(Lock::new("pgrp-test", parent.lock(&node).deep_copy()));
    {
        let node = LockNode::new();
        let mut guard = copied.lock(&node);
        // the copy carries the parent's mounts...
        assert_eq!(resolve(&guard, &slash, "/dev/cons").unwrap().dev, DEV_CONS,
            "the copied group starts with the parent's mounts");
        bind_locked(&mut guard, &slash, "#p", "/proc", pgrp::MREPL).expect("copy bind");
    }
    {
        let node = LockNode::new();
        let guard = parent.lock(&node);
        assert!(resolve(&guard, &slash, "/proc/1/note").is_err(),
            "RFCOPY (RFNAMEG): the child's bind is invisible to the parent");
    }

    // RFCNAMEG: the child starts with a CLEAN (empty) group — the wipe
    // (9k sysrfork: newpgrp, no pgrpcpy; the 05-03 finding's mechanism).
    let wiped = Pgrp::new();
    assert!(resolve(&wiped, &slash, "/dev/cons").unwrap().dev == DEV_ROOT,
        "a fresh group has no mounts: /dev/cons resolves to the underlying dir");
}

// ---------------------------------------------------------------------------
// unmount
// ---------------------------------------------------------------------------

#[test]
fn unmount_removes_and_errors_on_never_mounted() {
    let mut pg = Pgrp::new();
    let slash = slash();

    assert!(pg.unmount(b"/dev", None).is_err(), "unmount of a never-mounted point must error");

    bind_locked(&mut pg, &slash, "#c", "/dev", pgrp::MBEFORE).expect("bind");
    assert_eq!(resolve(&pg, &slash, "/dev/cons").unwrap().dev, DEV_CONS);

    // Remove the named entry: the #c mount leaves the union; the
    // underlying /dev is all that remains.
    let cons_root = namec::namec(b"#c", &slash, &slash, &pg, NamecMode::BindSource).unwrap();
    pg.unmount(b"/dev", Some(&cons_root)).expect("unmount the #c entry");
    assert_eq!(resolve(&pg, &slash, "/dev/cons").unwrap().dev, DEV_ROOT,
        "after unmount the underlying directory serves /dev again");

    // The same unmount a second time errors (the entry is gone).
    assert!(pg.unmount(b"/dev", Some(&cons_root)).is_err(), "a removed entry errors on re-unmount");

    // Whole-point unmount (nil old): everything at /dev goes.
    bind_locked(&mut pg, &slash, "#c", "/dev", pgrp::MBEFORE).expect("rebind");
    pg.unmount(b"/dev", None).expect("whole-point unmount");
    assert_eq!(resolve(&pg, &slash, "/dev/cons").unwrap().dev, DEV_ROOT);
}
