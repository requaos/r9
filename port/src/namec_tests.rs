//! Host-side pins for the 17-05 namec union consult (#45): sys_create's
//! final-component probe (walk_one_union) and the create dispatch
//! (create_chan_union) consult the union chain per the per-consult
//! chain_snapshot shape, so a create through a union-mounted directory
//! reaches the first-accepting member instead of missing (the original
//! #45 repro: rc's `>` is create-first, and `> file` in a union-mounted
//! dir died with "namec: device has no create" on the head device).
//!
//! Venue record (A5): the HOST venue — the devroot_rawimg/devenv
//! precedent extended to the union consult (the synthetic paqfs image
//! and the stub env-group hook), per the namespace_test.rs fixture
//! family.  The boot-gate rc `>` venue was the fallback and was not
//! needed.
//!
//! Fixtures are the namespace_test.rs shapes (same spec: Harvey
//! sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c}), duplicated per the
//! established test-file convention.  The group-dependent tests
//! serialize on a SEAM lock (the devip_tests discipline): devenv's
//! current-group hook is process-global.

use std::sync::{Arc, Mutex, Once};

use crate::dat::{Chan, DEV_ENV, DEV_PROC, DEV_ROOT, OREAD, OWRITE};
use crate::devenv::{self, EnvGroup};
use crate::devroot;
use crate::mcslock::{Lock, LockNode};
use crate::namec::{self, NamecMode};
use crate::pgrp::{self, Pgrp};

// ---------------------------------------------------------------------------
// Synthetic paqfs image (the namespace_test.rs builder pattern, same
// spec) — tree: root → dev/ → cons (file), kmsg (file)
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

/// Harvey libflate adler32(0, buf, n): the seed-0 variant (see
/// namespace_test.rs / paqfs_test.rs).
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

/// Encode a PaqDir entry (namespace_test.rs's put_dir_entry shape).
#[allow(clippy::too_many_arguments)]
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

const CONS_CONTENT: &[u8] = b"devroot's cons file\n";
const KMSG_CONTENT: &[u8] = b"devroot's kmsg file\n";

static TREE: &[Node] =
    &[Node::Dir("dev", &[Node::File("cons", CONS_CONTENT), Node::File("kmsg", KMSG_CONTENT)])];

static TEST_IMG: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

fn image() -> &'static [u8] {
    TEST_IMG.get_or_init(|| {
        let mut b = ImageBuilder::new("namec45-test");
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
// Shared fixtures: the mounted devroot tree, a root slash chan, a fresh
// pgrp.  mount_initrd is idempotent over the one shared image slice.
// ---------------------------------------------------------------------------

fn slash() -> Chan {
    devroot::mount_initrd(image()).expect("mount_initrd (idempotent over the shared slice)");
    devroot::root().expect("root mounted").attach_root().expect("attach root")
}

fn resolve(pg: &Lock<Pgrp>, slash: &Chan, path: &str) -> crate::Result<Chan> {
    namec::namec(path.as_bytes(), slash, slash, pg, NamecMode::Open)
}

/// The syscall layer's bind shape (namespace_test.rs's bind_locked):
/// resolve unlocked (namec snapshots per consult), lock for the table
/// insert only.
fn bind_locked(
    pg: &Lock<Pgrp>,
    slash: &Chan,
    src: &str,
    target: &str,
    flag: u32,
) -> crate::Result<u64> {
    let s = namec::namec(src.as_bytes(), slash, slash, pg, NamecMode::BindSource)?;
    let t = namec::namec(target.as_bytes(), slash, slash, pg, NamecMode::MountTarget)?;
    let node = LockNode::new();
    pg.lock(&node).bind(s, &t, flag)
}

// ---------------------------------------------------------------------------
// The devenv current-group hook (the devenv_test.rs idiom): registered
// ONCE per process; group-dependent tests serialize on the seam lock
// and swap the served group per test.
// ---------------------------------------------------------------------------

static SEAM_LOCK: Mutex<()> = Mutex::new(());
static TEST_CURRENT: Mutex<Option<Arc<Lock<EnvGroup>>>> = Mutex::new(None);

fn test_group_hook() -> Option<Arc<Lock<EnvGroup>>> {
    TEST_CURRENT.lock().unwrap().clone()
}

fn install_group_hook_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| devenv::set_group_hook(test_group_hook));
}

/// expect_err for Result<Chan, _> (Chan carries no Debug).
fn err_of(r: crate::Result<Chan>) -> &'static str {
    match r {
        Ok(_) => panic!("expected Err"),
        Err(msg) => msg,
    }
}

fn set_current_group(g: Arc<Lock<EnvGroup>>) {
    *TEST_CURRENT.lock().unwrap() = Some(g);
}

/// The seam guard: poison-immune (a RED assertion that panics while
/// holding the lock must not cascade a PoisonError into the sibling
/// test — each test fails or passes on its OWN assertion).
fn seam_guard() -> std::sync::MutexGuard<'static, ()> {
    SEAM_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

// ---------------------------------------------------------------------------
// #45: walk_one_union — the final-component probe consults the union
// ---------------------------------------------------------------------------

/// Without mounts the twin is the direct dispatch (the head-only
/// twin's behavior — nothing to consult).
#[test]
fn walk_one_union_without_mounts_keeps_direct_dispatch() {
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let slash = slash();

    let c = walk_one_union(&pg, &slash, b"dev").expect("dev resolves through devroot");
    assert_eq!(c.dev, DEV_ROOT);
    assert!(walk_one_union(&pg, &slash, b"nosuch").is_err());
}

fn walk_one_union(pg: &Lock<Pgrp>, slash: &Chan, name: &[u8]) -> crate::Result<Chan> {
    let parent = resolve(pg, slash, "/")?;
    namec::walk_one_union(&parent, name, pg)
}

/// The #45 repro shape: a name that lives ONLY in a non-head union
/// member resolves through the chain (the multi-component walker
/// already did this for intermediate components — the probe was the
/// blind spot).  MAFTER bind: chain order is [underlying devroot /dev,
/// devproc], so the head (the devroot dir the Open redirect lands on)
/// does NOT carry "123" — only the devproc member does.
#[test]
fn walk_one_union_finds_nonhead_member() {
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let slash = slash();
    bind_locked(&pg, &slash, "#p", "/dev", pgrp::MAFTER).expect("bind #p MAFTER /dev");

    // The parent the probe walks from: the union HEAD (the underlying
    // devroot /dev — MAFTER order).
    let parent = resolve(&pg, &slash, "/dev").expect("/dev resolves");
    assert_eq!(parent.dev, DEV_ROOT, "MAFTER: the head is the underlying dir");

    // The consult reaches the devproc member.
    let nc = namec::walk_one_union(&parent, b"123", &pg)
        .expect("the probe must consult the union chain (the #45 fix)");
    assert_eq!(nc.dev, DEV_PROC, "the name lives only in the MAFTER member");

    // The head-only twin stays head-only (the aarch64 caller's shape —
    // and the documented #45 blindness this plan closed on x86_64).
    assert!(namec::walk_one(&parent, b"123").is_err(), "the head-only twin must not consult");
}

// ---------------------------------------------------------------------------
// #45: create_chan_union — the create reaches the accepting member
// ---------------------------------------------------------------------------

/// Without mounts the twin is the head-only dispatch (today's
/// behavior — the no-create error included).
#[test]
fn create_chan_union_without_mounts_keeps_direct_dispatch() {
    let pg = Lock::new("pgrp-test", Pgrp::new());
    let slash = slash();

    let parent = resolve(&pg, &slash, "/dev").expect("/dev resolves");
    let err = err_of(namec::create_chan_union(&parent, b"newfile", OWRITE, 0o644, &pg));
    assert_eq!(err, "namec: device has no create");
}

/// The #45 repro proper: a create through a union whose head device
/// cannot create reaches the first-accepting member (here a devenv
/// member bound MAFTER over /dev) — end-to-end: the created chan
/// writes, and the key reads back through a fresh `#e` resolution.
#[test]
fn create_chan_union_reaches_accepting_member() {
    let _seam = seam_guard();
    install_group_hook_once();
    set_current_group(Arc::new(Lock::new("env", EnvGroup::new())));

    let pg = Lock::new("pgrp-test", Pgrp::new());
    let slash = slash();
    bind_locked(&pg, &slash, "#e", "/dev", pgrp::MAFTER).expect("bind #e MAFTER /dev");

    // The parent: the union HEAD (the underlying devroot /dev) — the
    // device the head-only create_chan errored on (#45).
    let parent = resolve(&pg, &slash, "/dev").expect("/dev resolves");
    assert_eq!(parent.dev, DEV_ROOT);

    let mut created = namec::create_chan_union(&parent, b"unionkey", OWRITE, 0o644, &pg)
        .expect("the create must reach the accepting union member (the #45 fix)");
    assert_eq!(created.dev, DEV_ENV);
    assert_eq!(created.path.as_slice(), b"/dev/unionkey");

    // End-to-end: write through the created chan, read back through a
    // fresh `#e` resolution (a different chan into the same group).
    assert_eq!(devenv::write(&mut created, b"union-value", u64::MAX).expect("write"), 11);
    let key = resolve(&pg, &slash, "#e/unionkey").expect("#e/unionkey resolves after the create");
    let mut key = namec::open_chan(&key, OREAD).expect("open OREAD");
    let mut buf = [0u8; 32];
    let n = devenv::read(&mut key, &mut buf, u64::MAX).expect("read back");
    assert_eq!(&buf[..n as usize], b"union-value");
}

/// The refusal arm: a member whose create op runs and FAILS propagates
/// its error — the consult must NOT fall through to later members on a
/// server-level refusal (the 15-03 server-owns-the-policy stance; a
/// fall-through would land the file in a shadowed member, the
/// wrong-union-member tamper class).  A duplicate key make devenv's
/// create error "devenv: name exists"; the only other chain member is
/// the underlying devroot dir (no create) — a fall-through-on-refusal
/// implementation would surface "namec: device has no create" instead.
#[test]
fn create_chan_union_refusal_propagates_not_falls_through() {
    let _seam = seam_guard();
    install_group_hook_once();
    let group = Arc::new(Lock::new("env", EnvGroup::new()));
    set_current_group(group.clone());

    let pg = Lock::new("pgrp-test", Pgrp::new());
    let slash = slash();
    bind_locked(&pg, &slash, "#e", "/dev", pgrp::MAFTER).expect("bind #e MAFTER /dev");

    // Pre-create the key in the group (the head member IS devenv in
    // chain order? no — MAFTER puts the underlying dir first; the
    // devenv member is the tail that accepts).
    let parent = resolve(&pg, &slash, "/dev").expect("/dev resolves");
    let mut first =
        namec::create_chan_union(&parent, b"dupkey", OWRITE, 0o644, &pg).expect("first create");
    assert_eq!(devenv::write(&mut first, b"v", u64::MAX).expect("write"), 1);

    // The duplicate create: devenv's own Eexist must propagate.
    let err = err_of(namec::create_chan_union(&parent, b"dupkey", OWRITE, 0o644, &pg));
    assert_eq!(err, "devenv: name exists", "a member refusal propagates, never falls through");
}
