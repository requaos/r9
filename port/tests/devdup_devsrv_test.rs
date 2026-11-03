//! Host-side tests for the 05-06 devdup + devsrv devices (the small
//! devices the boot chain fatals on): devdup's per-fd files (`#d/N` —
//! rcmain's `. -i '#d/0'`, Pitfall 9) and devsrv's `#s` service registry
//! (boot's srvcreate("boot", fd) — FATAL on failure, aux.c).
//!
//! Semantics are pinned against the 9k donor (sys/src/9k/port/devdup.c's
//! dupgen/dupopen — open resolves the OPERATING proc's fd N at open time
//! and returns ITS chan; devsrv.c's srvcreate/srvwrite/srvopen — create +
//! fd-number write + open-returns-the-posted-chan).  The fd resolution is
//! hook-injected in the kernel (port can't see PROC_TABLE); these tests
//! drive the same hooks over a fake per-test fd group.
//!
//! T-05-11 pin: the chan devsrv stores at write time is a REFERENCE CLONE
//! — closing the registrant's fd afterwards must not kill the service
//! (the Arc-shared pipe queue outlives the registrant; the donor's
//! fdtochan incref).

use std::sync::Mutex;

use port::dat::{Chan, Qid, DEV_CONS, DEV_PIPE, OREAD, ORDWR, OTRUNC, OWRITE};
use port::devdup::{self, DupHooks};
use port::devpipe;
use port::devsrv;
use port::fdgrp::FdGroup;
use port::namec;

/// The fake "calling proc's fd table" both devices' hooks resolve
/// against (the arch side's current_fd_resolve/current_fd_list mirror).
static FAKE_TABLE: Mutex<Option<FdGroup<32>>> = Mutex::new(None);

fn fake_resolve(fd: u64) -> Option<Chan> {
    FAKE_TABLE.lock().unwrap().as_ref().and_then(|g| g.ref_clone(fd))
}

fn fake_list() -> Vec<(u64, u32)> {
    FAKE_TABLE.lock().unwrap().as_ref().map(|g| g.list_open()).unwrap_or_default()
}

/// An open pipe pair installed in the fake table at fds 0/1 (the
/// connectembed shape: fd 0 = data end 0, fd 1 = data1 end 1).
fn fake_pipe_fds() -> (Chan, Chan) {
    let dir = devpipe::attach();
    let c0 = devpipe::walk(&dir, b"data").expect("walk data");
    let c1 = devpipe::walk(&dir, b"data1").expect("walk data1");
    let e0 = namec::open_chan(&c0, ORDWR).expect("open data");
    let e1 = namec::open_chan(&c1, ORDWR).expect("open data1");
    let mut table = FdGroup::<32>::new();
    assert_eq!(table.alloc_lowest(e0.dup()), Some(0));
    assert_eq!(table.alloc_lowest(e1.dup()), Some(1));
    *FAKE_TABLE.lock().unwrap() = Some(table);
    (e0, e1)
}

/// devdup walk grammar: numeric components resolve (existence is the
/// open path's business); non-numeric, empty and overflowing components
/// error loudly.
#[test]
fn devdup_walk_validation() {
    let root = devdup::attach();
    let c = devdup::walk(&root, b"0").expect("walk #d/0");
    assert_eq!(c.qid.path, 1, "the donor's qid encoding: 2*fd+1");
    let c = devdup::walk(&root, b"12").expect("walk #d/12");
    assert_eq!(c.qid.path, 25);
    assert!(devdup::walk(&root, b"x").is_err(), "non-numeric errors loudly");
    assert!(devdup::walk(&root, b"").is_err(), "empty errors loudly");
    assert!(devdup::walk(&root, b"99999999999999999999999999").is_err(), "overflow errors loudly");
    // A walk under a file errors (the tree is flat).
    assert!(devdup::walk(&c, b"0").is_err(), "walk under a file errors");
}

/// CR-01: walk one stat(5) record's FOUR length-prefixed strings (name,
/// uid, gid, muid — libc's statcheck walks four and rejects a record that
/// ends early, convM2D.c), requiring the record to END exactly after them
/// (statcheck's `buf != ebuf` rejection; devproc_test's parse_dir shape).
fn stat_strings(rec: &[u8]) -> Vec<String> {
    let mut p = 41usize;
    let mut out = Vec::new();
    for _ in 0..4 {
        let slen = u16::from_le_bytes([rec[p], rec[p + 1]]) as usize;
        p += 2;
        out.push(String::from_utf8(rec[p..p + slen].to_vec()).unwrap());
        p += slen;
    }
    assert_eq!(p, rec.len(), "the record ends exactly after FOUR strings (statcheck)");
    out
}

/// The combined device flow (one test installs both set-once hook sets —
/// the 05-05 devenv pattern):
///
/// devdup: `#d/0` resolves through the registry, opens to the CALLING
/// proc's fd 0 chan (a pipe end — writes through the `#d/0` fd cross the
// pipe), a read-only fd refuses a write open (the donor fdtochan mode
// check), an out-of-range fd errors, and the root listing names the
// open fds.
///
/// devsrv: create + fd-number write registers a service (the Arc-safe
// reference clone), the listing names it, a duplicate create errors
// Eexist, a second posting errors, OTRUNC-open on an existing service is
// the donor's "srv file already exists", and opening the posted service
// returns the POSTED chan — which still carries bytes after the
// registrant's fd closes (T-05-11).
#[test]
fn devdup_devsrv_device_flow() {
    devdup::set_hooks(DupHooks { resolve: fake_resolve, list: fake_list });
    devsrv::set_resolve_hook(fake_resolve);

    let (e0, mut e1) = fake_pipe_fds();

    // --- devdup: #d resolves through the registry (namec, the 05-04
    // shape; dummy slash/dot serve the '#'-path arm). ---
    let dummy = Chan::new(DEV_CONS, Qid { path: 0, vers: 0, typ: 0x80 }, 0);
    let pg = port::pgrp::Pgrp::new();
    let dup_root = namec::namec(b"#d", &dummy, &dummy, &pg, namec::NamecMode::Open)
        .expect("#d must resolve through the registry");
    assert_eq!(dup_root.dev, port::dat::DEV_DUP);

    // Open #d/0 — the returned chan IS the calling proc's fd 0 (the pipe
    // end): writes through it cross the pipe.
    let walked = devdup::walk(&dup_root, b"0").expect("walk 0");
    let mut via_dup = devdup::open(&walked, OWRITE).expect("open #d/0 OWRITE (the fd is ORDWR)");
    assert_eq!(via_dup.dev, DEV_PIPE, "open #d/0 returns the fd's real chan");
    devpipe::write(&mut via_dup, b"via-dup", u64::MAX).expect("write through #d/0");
    let mut buf = [0u8; 16];
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("the pipe's other end reads it");
    assert_eq!(&buf[..n as usize], b"via-dup", "the #d/0 fd writes to the referenced pipe");
    namec::close_chan(&via_dup); // balance the resolve's reference

    // Mode discipline: install a read-only chan at fd 2; a write open of
    // #d/2 must error (the donor fdtochan check).
    {
        let mut guard = FAKE_TABLE.lock().unwrap();
        let g = guard.as_mut().unwrap();
        g.fds[2] = Some(Chan::new(DEV_CONS, Qid { path: 1, vers: 0, typ: 0 }, OREAD));
    }
    let walked = devdup::walk(&dup_root, b"2").expect("walk 2");
    assert!(devdup::open(&walked, OWRITE).is_err(), "write-open of a read-only fd errors");
    assert!(devdup::open(&walked, OREAD).is_ok(), "read-open matches");

    // Out-of-range/absent fd errors loudly at open.
    let walked = devdup::walk(&dup_root, b"9").expect("walk 9");
    assert!(devdup::open(&walked, OREAD).is_err(), "an absent fd errors at open");

    // The root listing names the open fds by number.
    let mut root_rd = devdup::open(&dup_root, OREAD).expect("open #d root");
    let mut listing = [0u8; 1024];
    let n = devdup::read(&mut root_rd, &mut listing, u64::MAX).expect("dir read") as usize;
    let body = &listing[..n];
    assert!(body.windows(3).any(|w| w == &[0x00, 0x00, b'0']) || body.windows(1).any(|w| w == b"0"), "fd 0 listed");
    assert!(body.windows(1).any(|w| w == b"1"), "fd 1 listed");

    // CR-01: every listing record is the 4-string stat(5) shape and the
    // listing parses to the last byte (libc dirread's statcheck per
    // record).
    let mut i = 0usize;
    while i + 2 <= n {
        let sz = u16::from_le_bytes([body[i], body[i + 1]]) as usize;
        if sz == 0 {
            break;
        }
        stat_strings(&body[i..i + 2 + sz]);
        i += 2 + sz;
    }
    assert_eq!(i, n, "the devdup listing parses to the last byte (4-string records)");

    // --- devsrv: the #s registry (boot's srvcreate contract). ---
    let srv_root = namec::namec(b"#s", &dummy, &dummy, &pg, namec::NamecMode::Open)
        .expect("#s must resolve through the registry");
    assert_eq!(srv_root.dev, port::dat::DEV_SRV);

    // create("#s/boot", OWRITE, 0666) — the srvcreate shape.
    let mut created = namec::create_chan(&srv_root, b"boot", OWRITE, 0o666).expect("create #s/boot");
    assert_eq!(created.dev, port::dat::DEV_SRV);
    // A duplicate create errors Eexist (loud, never a silent shadow).
    assert!(namec::create_chan(&srv_root, b"boot", OWRITE, 0o666).is_err(), "dup create errors");
    // create under a non-directory errors.
    assert!(namec::create_chan(&created, b"x", OWRITE, 0o666).is_err(), "create under a file errors");
    // read-mode create errors (the donor's OWRITE-only srvcreate).
    assert!(namec::create_chan(&srv_root, b"rdonly", OREAD, 0o666).is_err(), "OWRITE-only");

    // The fd-number write registers the calling proc's fd 0 (the pipe
    // end) — the Arc-safe reference clone.
    let n = devsrv::write(&mut created, b"0", u64::MAX).expect("write fd number");
    assert_eq!(n, 1);
    // A second posting errors (the donor's Ebadusefd).
    assert!(devsrv::write(&mut created, b"0", u64::MAX).is_err(), "already-posted errors");
    namec::close_chan(&created);

    // The registrant closes its fd — the posted chan MUST keep the pipe
    // alive (T-05-11).
    {
        let mut guard = FAKE_TABLE.lock().unwrap();
        let g = guard.as_mut().unwrap();
        let c = g.take(0).expect("fd 0 occupied");
        drop(guard);
        namec::close_chan(&c); // the registrant's close
    }

    // open("#s/boot") returns the POSTED chan — writes still cross the
    // pipe (the queue survived the registrant's close).
    let walked = devsrv::walk(&srv_root, b"boot").expect("walk boot");
    let mut posted = devsrv::open(&walked, ORDWR).expect("open #s/boot");
    assert_eq!(posted.dev, DEV_PIPE, "the posted chan IS the pipe end");
    devpipe::write(&mut posted, b"posted", u64::MAX)
        .expect("the posted chan still writes after the registrant closed");
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("read via the surviving queue");
    assert_eq!(&buf[..n as usize], b"posted", "T-05-11: the clone outlives the registrant");

    // OTRUNC-open of an existing service is the donor's refusal.
    assert!(devsrv::open(&walked, OWRITE | OTRUNC).is_err(), "srv file already exists");

    // The dir listing names the registered service.
    let mut srv_rd = devsrv::open(&srv_root, OREAD).expect("open #s root");
    let mut listing = [0u8; 512];
    let n = devsrv::read(&mut srv_rd, &mut listing, u64::MAX).expect("dir read") as usize;
    assert!(listing[..n].windows(4).any(|w| w == b"boot"), "the listing names boot");

    // CR-01: the listing's records are the 4-string stat(5) shape, parsing
    // to the last byte.
    let mut i = 0usize;
    while i + 2 <= n {
        let sz = u16::from_le_bytes([listing[i], listing[i + 1]]) as usize;
        if sz == 0 {
            break;
        }
        stat_strings(&listing[i..i + 2 + sz]);
        i += 2 + sz;
    }
    assert_eq!(i, n, "the devsrv listing parses to the last byte (4-string records)");

    // stat: the service reports its name/perm — as a FOUR-string record
    // (name, uid, gid, muid; CR-01's statcheck shape).
    let mut sb = [0u8; 256];
    let n = devsrv::stat(&walked, &mut sb).expect("stat service") as usize;
    let strs = stat_strings(&sb[..n]);
    assert_eq!(strs, ["boot", "glenda", "glenda", "glenda"], "name, uid, gid, muid");

    namec::close_chan(&posted);
    let _ = e0; // the pair's end 0 (fd-table copy) is covered above
}
