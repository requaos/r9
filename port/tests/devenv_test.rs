//! Host-side tests for the 05-05 devenv device (D-07): the per-pgrp
//! environment group (`EnvGroup` create/write/trunc/read/deep_copy
//! semantics) and the `#e` device surface (attach/walk/open/read/write/
//! create/stat over the OPERATING process's group — the donor devenv.c
//! envgrp(c) → up->egrp shape).
//!
//! Semantics are pinned against the 9k donor (sys/src/9k/port/devenv.c:
//! envlookup/envcreate/envopen's OTRUNC arm/envread/envwrite's
//! grow-at-offset/Maxenvsize, envcpy's renumbering deep copy) and
//! fork(2)/9k sysproc.c for the rfork group semantics.
//!
//! DEVIATION RECORDED (05-05 Task 1) — the plan's <behavior> bullet says
//! "RFENVG share — a child's setenv is visible to the parent; RFCENVG
//! copy — the child's setenv is invisible to the parent and the child's
//! prior values survive intact".  That is INVERTED relative to fork(2)
//! (sys/man/2/fork: "RFENVG: the environment variables are COPIED;
//! otherwise the two processes SHARE environment variables.  RFCENVG:
//! the new process starts with an EMPTY environment"), 9k's sysproc.c
//! sysrfork (RFENVG → envcpy copy; RFCENVG → fresh empty Egrp; neither
//! → incref-share), and D-07's own operative clause ("init's setenv and
//! rc's var export to children behave EXACTLY LIKE PLAN 9" — newns's
//! rfork(RFENVG|RFCNAMEG) depends on RFENVG being the copy arm so the
//! login child's user=/home= overrides never leak back into init's
//! group).  This is the 05-04 RFNAMEG/RFCNAMEG inversion, part two —
//! the donor governs.  These tests pin the DONOR/fork(2) semantics:
//! neither-flag SHARE, RFENVG COPY, RFCENVG EMPTY.

use std::sync::{Arc, Mutex};

use port::dat::{Chan, Qid, DEV_ENV, OREAD, OTRUNC, OWRITE};
use port::devenv::{self, EnvGroup};
use port::mcslock::{Lock, LockNode};
use port::namec::{self, NamecMode};
use port::pgrp::Pgrp;

/// The kernel seed contract (05-05 must_haves; main.rs prints the same
/// triple in its `devenv: seeded …` gate line).
const SEED: &[(&[u8], &[u8])] =
    &[(b"cputype", b"aarch64"), (b"nobootprompt", b"embed"), (b"readparts", b"0")];

fn seed_group() -> EnvGroup {
    EnvGroup::seeded(SEED).expect("the seed pairs are well-formed")
}

// ---------------------------------------------------------------------------
// Group-level semantics (no device layer, no hook)
// ---------------------------------------------------------------------------

/// Behavior bullet 1, group half: a seeded key reads back its seeded
/// value; an unset key is ABSENT (the device half — the open failure
/// boot's readfile fails soft on — is the device flow test below).
#[test]
fn seeded_keys_read_back_and_unset_is_absent() {
    let g = seed_group();
    assert_eq!(g.get(b"cputype").map(|e| e.value.as_slice()), Some(&b"aarch64"[..]));
    assert_eq!(g.get(b"nobootprompt").map(|e| e.value.as_slice()), Some(&b"embed"[..]));
    assert_eq!(g.get(b"readparts").map(|e| e.value.as_slice()), Some(&b"0"[..]));
    assert!(g.get(b"nosuchvar").is_none(), "an unset key must be absent (fail-soft)");
}

/// Behavior bullet 2, group half: create makes a new key in the caller's
/// group; write-at-0 stores the value; trunc+write REPLACES (the libc
/// setenv flow — init's setenv("#e/objtype", …)); the donor's
/// grow-at-offset write semantics; create on a duplicate errors Eexist
/// (namec's Acreate OTRUNC-open fallback handles create-on-existing —
/// 05-06's sys_create side, not the device's).
#[test]
fn write_replace_and_create_round_trip() {
    let mut g = EnvGroup::new();
    let q = g.create(b"objtype").expect("create objtype");
    assert_eq!(g.write(q, b"aarch64", 0).expect("write"), 7);
    assert_eq!(g.get(b"objtype").map(|e| e.value.as_slice()), Some(&b"aarch64"[..]));

    // Replace via the setenv shape: trunc (create-on-existing's OTRUNC
    // open arm) then write at 0.
    g.trunc(q).expect("trunc");
    assert_eq!(g.get(b"objtype").map(|e| e.value.len()), Some(0), "trunc empties");
    assert_eq!(g.write(q, b"r9", 0).expect("rewrite"), 2);
    assert_eq!(g.get(b"objtype").map(|e| e.value.as_slice()), Some(&b"r9"[..]));

    // The donor's envwrite grow-at-offset: writing past the current end
    // extends (zero-gap), never truncates the surviving prefix.
    assert_eq!(g.write(q, b"-lab", 2).expect("grow write"), 4);
    assert_eq!(g.get(b"objtype").map(|e| e.value.as_slice()), Some(&b"r9-lab"[..]));

    // envcreate: Eexist on a duplicate name (loud, never a silent
    // shadow); qid paths are 1-based per group in insertion order.
    assert!(g.create(b"objtype").is_err(), "duplicate create must error (Eexist)");
    let q2 = g.create(b"service").expect("create service");
    assert!(q2 != q, "qid paths are unique within the group");
    assert_eq!(g.get(b"service").map(|e| e.qid_path), Some(q2));
}

/// Behavior bullet 3 (DONOR-corrected — see the header): rfork's env
/// semantics are NEITHER=share, RFENVG=copy, RFCENVG=empty.  Mirrors the
/// proc.rs wiring (the arch side's flag → branch expression) with the
/// port-crate primitives; the gate exercises the boot path end-to-end.
#[test]
fn rfork_env_semantics_share_copy_empty() {
    let parent = Arc::new(Lock::new("env", seed_group()));

    // NEITHER flag: the child SHARES the parent's handle (Arc clone) —
    // a child's setenv is visible to the parent (rc's var export to
    // children, D-07's operative contract).
    let shared = parent.clone();
    {
        let node = LockNode::new();
        let mut g = shared.lock(&node);
        let q = g.create(b"objtype").expect("create in shared group");
        g.write(q, b"aarch64", 0).expect("write in shared group");
    }
    {
        let node = LockNode::new();
        let g = parent.lock(&node);
        assert_eq!(
            g.get(b"objtype").map(|e| e.value.as_slice()),
            Some(&b"aarch64"[..]),
            "neither-flag fork SHARES: the child's setenv is visible to the parent"
        );
    }

    // RFENVG: the child gets a fresh deep COPY — its setenv is
    // INVISIBLE to the parent, and its prior (copied) values survive
    // intact (newns's per-login overrides never leak back to init).
    let copied = {
        let node = LockNode::new();
        Arc::new(Lock::new("env", parent.lock(&node).deep_copy()))
    };
    {
        let node = LockNode::new();
        let mut g = copied.lock(&node);
        assert_eq!(
            g.get(b"cputype").map(|e| e.value.as_slice()),
            Some(&b"aarch64"[..]),
            "RFENVG copy: prior values survive intact"
        );
        let q = g.create(b"user").expect("create in copied group");
        g.write(q, b"glenda", 0).expect("write in copied group");
        let qc = g.get(b"cputype").map(|e| e.qid_path).expect("cputype in copy");
        g.write(qc, b"riscv64", 0).expect("overwrite in copied group");
    }
    {
        let node = LockNode::new();
        let g = parent.lock(&node);
        assert!(
            g.get(b"user").is_none(),
            "RFENVG copy: the child's setenv is invisible to the parent"
        );
        assert_eq!(
            g.get(b"cputype").map(|e| e.value.as_slice()),
            Some(&b"aarch64"[..]),
            "RFENVG copy: the child's overwrite never touches the parent's value"
        );
    }

    // RFCENVG: the child starts EMPTY.
    let emptied = EnvGroup::new();
    assert!(emptied.get(b"cputype").is_none(), "RFCENVG: the child starts empty");
    assert!(emptied.entries().is_empty());
}

/// Behavior bullet 4: two procs in different groups hold different
/// values for the same key simultaneously.
#[test]
fn two_groups_hold_same_key_independently() {
    let mut a = seed_group();
    let mut b = seed_group();
    let qa = a.get(b"cputype").map(|e| e.qid_path).expect("a cputype");
    let qb = b.get(b"cputype").map(|e| e.qid_path).expect("b cputype");
    a.write(qa, b"aarch64-hosta", 0).expect("write a");
    b.write(qb, b"aarch64-hostb", 0).expect("write b");
    assert_eq!(a.get(b"cputype").map(|e| e.value.as_slice()), Some(&b"aarch64-hosta"[..]));
    assert_eq!(b.get(b"cputype").map(|e| e.value.as_slice()), Some(&b"aarch64-hostb"[..]));
}

/// The donor's bounds: writes beyond Maxenvsize error Etoobig; names are
/// a single component under the genbuf ceiling (empty/'/'/too-long all
/// error loudly).
#[test]
fn env_bounds_and_name_validation() {
    let mut g = EnvGroup::new();
    let q = g.create(b"k").expect("create");
    assert_eq!(g.write(q, b"v", devenv::MAXENVSIZE - 1).expect("edge write fits"), 1);
    assert!(g.write(q, b"v", devenv::MAXENVSIZE).is_err(), "offset at Maxenvsize errors");
    assert!(g.write(q, b"vv", devenv::MAXENVSIZE - 1).is_err(), "extent crossing errors");
    assert!(g.create(b"").is_err(), "empty name errors");
    assert!(g.create(b"a/b").is_err(), "a '/' in a name errors (single-component tree)");
    let long = [b'x'; 128];
    assert!(g.create(&long).is_err(), "names at/over the genbuf ceiling error");
    assert!(g.write(9999, b"v", 0).is_err(), "write to an unknown qid errors");
    assert!(g.trunc(9999).is_err(), "trunc of an unknown qid errors");
    assert_eq!(g.write(q, b"", 0).expect("empty write is a 0 no-op"), 0);
}

// ---------------------------------------------------------------------------
// The device surface over the OPERATING process's group (the hook).
// One test installs the set-once group hook and drives every op through
// it — the donor envgrp(c) dynamic-resolution contract.
// ---------------------------------------------------------------------------

/// The test-side "current group" the installed hook serves (device ops
/// resolve it per call, exactly like the donor's up->egrp).
static TEST_CURRENT: Mutex<Option<Arc<Lock<EnvGroup>>>> = Mutex::new(None);

fn test_group_hook() -> Option<Arc<Lock<EnvGroup>>> {
    TEST_CURRENT.lock().unwrap().clone()
}

fn set_current(g: Arc<Lock<EnvGroup>>) {
    *TEST_CURRENT.lock().unwrap() = Some(g);
}

fn locked_new_group(pairs: &[(&[u8], &[u8])]) -> Arc<Lock<EnvGroup>> {
    Arc::new(Lock::new("env", EnvGroup::seeded(pairs).expect("seed")))
}

/// The end-to-end device flow: registry resolution of `#e`, attach
/// (empty spec only — the #ec fail-soft contract), walk hit/miss, open +
/// read a seeded value, OTRUNC-open + write replace, create + write +
/// read a new key, stat shapes, the directory listing, and the donor's
/// dynamic group resolution (the SAME chan resolves in the OPERATING
/// group — swap the hook's group and the walked chan reads the other
/// group's value for that qid path).
#[test]
fn devenv_device_flow_over_dynamic_groups() {
    devenv::set_group_hook(test_group_hook);

    let group_a = locked_new_group(SEED);
    let group_b = locked_new_group(&[(b"cputype", b"riscv64")]);
    set_current(group_a.clone());

    // `#e` resolves through the device registry (05-04's table) — the
    // must_have's "resolves through the device registry" pinned.  The
    // '#'-path arm never consults slash/dot, so dummy chans serve.
    let dummy = Chan::new(port::dat::DEV_ROOT, Qid { path: 0, vers: 0, typ: 0x80 }, 0);
    let pg = Pgrp::new();
    let via_namec = namec::namec(b"#e", &dummy, &dummy, &pg, NamecMode::Open)
        .expect("#e must resolve through the registry");
    assert_eq!(via_namec.dev, DEV_ENV, "#e must land on the devenv device");

    // attach: empty spec serves the root dir; a non-empty spec (the
    // donor's `#ec` config group) errors loudly — boot's bind("#ec", …)
    // is the designed fail-soft path.
    let root = devenv::attach(b"").expect("attach");
    assert_eq!(root.qid.typ, 0x80, "the env root is a directory");
    assert!(devenv::attach(b"c").is_err(), "no #ec this era — loud error, fail-soft bind");

    // Seeded key: walk → open OREAD → read back the seeded value.
    let mut c = devenv::walk(&root, b"cputype").expect("walk cputype");
    let mut c = devenv::open(&c, OREAD).expect("open OREAD");
    let mut buf = [0u8; 64];
    let n = devenv::read(&mut c, &mut buf, u64::MAX).expect("read seeded value");
    assert_eq!(&buf[..n as usize], b"aarch64", "a seeded key reads back its seeded value");
    // EOF after the value (streaming offset advanced).
    let n = devenv::read(&mut c, &mut buf, u64::MAX).expect("read at end");
    assert_eq!(n, 0, "EOF is a 0 read, never an error");

    // Behavior bullet 1, device half: an unset key errors on the
    // open(2) path (the walk inside namec) — boot's readfile fails
    // soft on exactly this error.
    assert!(devenv::walk(&root, b"nosuchvar").is_err(), "unset key: the open path errors");

    // Write/replace via the setenv shape: OTRUNC open, write, re-read.
    let c = devenv::walk(&root, b"cputype").expect("re-walk cputype");
    let mut c = devenv::open(&c, OWRITE | OTRUNC).expect("open OWRITE|OTRUNC");
    let n = devenv::write(&mut c, b"aarch64-r9", u64::MAX).expect("replace write");
    assert_eq!(n, 11);
    let mut c = devenv::open(&devenv::walk(&root, b"cputype").unwrap(), OREAD).unwrap();
    let n = devenv::read(&mut c, &mut buf, u64::MAX).expect("re-read");
    assert_eq!(&buf[..n as usize], b"aarch64-r9", "write replaces the value");

    // Create: a new key in the OPERATING group, open for write
    // immediately (the init setenv("#e/objtype", …) flow).
    let mut c = devenv::create(&root, b"objtype", OWRITE, 0o644).expect("create objtype");
    assert_eq!(devenv::write(&mut c, b"aarch64", u64::MAX).expect("write new key"), 7);
    let mut c = devenv::open(&devenv::walk(&root, b"objtype").unwrap(), OREAD).unwrap();
    let n = devenv::read(&mut c, &mut buf, u64::MAX).expect("read new key");
    assert_eq!(&buf[..n as usize], b"aarch64");
    assert!(devenv::create(&root, b"objtype", OWRITE, 0o644).is_err(), "dup create errors");
    assert!(
        devenv::create(&c, b"x", OWRITE, 0o644).is_err(),
        "create under a non-directory errors"
    );

    // stat shapes: a key serves its name, 0666, and the value's length;
    // the root serves "#e", DMDIR|0755-ish (donor 0775), QTDIR.
    let c = devenv::walk(&root, b"cputype").expect("walk for stat");
    let mut sb = [0u8; 256];
    let n = devenv::stat(&c, &mut sb).expect("stat key") as usize;
    let name_len = u16::from_le_bytes([sb[41], sb[42]]) as usize;
    assert_eq!(&sb[43..43 + name_len], b"cputype");
    let length = u64::from_le_bytes(sb[33..41].try_into().unwrap());
    assert_eq!(length, 11, "stat length is the value's length");
    assert_eq!(n, 41 + (2 + name_len) + 2 * (2 + 6), "wire byte count (convD2M)");
    let n = devenv::stat(&root, &mut sb).expect("stat root") as usize;
    let name_len = u16::from_le_bytes([sb[41], sb[42]]) as usize;
    assert_eq!(&sb[43..43 + name_len], b"#e");
    assert_eq!(sb[8], 0x80, "the root stats as QTDIR");
    assert!(n > 41);

    // The directory listing: a read of the root serves the stat-wire
    // records of the current group's entries (insertion order).
    let mut root_rd = devenv::open(&root, OREAD).expect("open root dir");
    let mut listing = [0u8; 1024];
    let n = devenv::read(&mut root_rd, &mut listing, u64::MAX).expect("dir read") as usize;
    let body = &listing[..n];
    assert!(n > 41 * 4, "the listing holds every entry's record");
    assert!(
        body.windows(7).any(|w| w == b"cputype")
            && body.windows(7).any(|w| w == b"objtype")
            && body.windows(12).any(|w| w == b"nobootprompt")
            && body.windows(9).any(|w| w == b"readparts"),
        "the listing names the group's keys"
    );

    // The donor's dynamic resolution: the SAME walked chan resolves in
    // the OPERATING group.  Swap the hook's group — group B's entry 1
    // is its own cputype ("riscv64"), and the chan walked under A now
    // reads B's value (envgrp(c) → up->egrp at every op).
    let c_a = devenv::walk(&root, b"cputype").expect("walk cputype under A");
    let c_b = {
        let node = LockNode::new();
        let g = group_b.lock(&node);
        g.get(b"cputype").map(|e| e.qid_path).expect("b cputype")
    };
    assert_eq!(c_a.qid.path, c_b, "both groups' first key shares qid path 1 (renumbered)");
    set_current(group_b);
    let mut c2 = devenv::open(&c_a, OREAD).expect("open the A-walked chan under B");
    let n = devenv::read(&mut c2, &mut buf, u64::MAX).expect("read under group B");
    assert_eq!(
        &buf[..n as usize],
        b"riscv64",
        "dynamic resolution: the operating process's group governs (donor envgrp)"
    );
    assert!(
        devenv::walk(&root, b"objtype").is_err(),
        "group B never had objtype created — per-group keyspace"
    );
}
