//! The devssl host pins (19-04, DESK-04): the donor-verbatim semantics
//! of `#D` (sys/src/9k/port/devssl.c) proven deterministically on the
//! host — the Pitfall-11 pin the plan names: the exact mksecret-shaped
//! inputs through rc4_256+sha1 in BOTH secret orderings, over a REAL
//! pipe pair (the fd-to-fd loopback substrate), with the wire bytes
//! inspected on the way past.
//!
//! THE PIN MAP (the plan's four test classes):
//!
//! - (1) the pushssl protocol walk: #D -> ssl -> clone -> open (the
//!   chan's qid becomes the conv ctl) -> ctl read serves the decimal
//!   conv id -> the data path resolves by name -> the fd -> secretin
//!   -> secretout -> alg verb order succeeds, `alg clear` (the -e
//!   clear debug fallback) round-trips framed-but-clear, and
//!   encalgs/hashalgs read the scoped tables
//! - (2) the ctl verb errors carry the DONOR's exact strings: alg
//!   before fd is "must set fd before algorithm", an unknown verb is
//!   Ebadarg ("bad arg in system call"), secrets missing at alg is
//!   "algorithm but no secret", an unknown algorithm name is "bad
//!   algorithm"
//! - (3) the end-to-end loopback: two convs wired fd-to-fd over a real
//!   pipe, mksecret-shaped dec64 secrets armed rc4_256+sha1, the wire
//!   inspected (the |0x80 2-byte header counting digest+payload),
//!   plaintext in one data emerging from the peer's data, and a
//!   corrupted frame failing LOUDLY ("bad digest" — T-19-09)
//! - (4) the secrets-swapped server arm (srvp9auth's literal write
//!   order: secretin FS, secretout FC) round-trips BOTH directions,
//!   and the unswapped pairing (client order meets client order)
//!   FAILS the digest — the swap is load-bearing, pinned as such
//!
//! Harness discipline (devcap_tests): every stateful pin serializes on
//! the devcons user-seam lock and installs a FRESH #D state
//! (devssl::install_test_state) + the glenda stub user.  The secret
//! pair is the python-derived mksecret composition of
//! sha1("0123456789abcdef") — the exact p9auth/srvp9auth shape
//! (FE5567E8D76955085218 / 2CDF69D74BB16DFF8E29), cross-checked at
//! rc4.rs's dec64 pins.

use crate::dat::{Chan, ORDWR, OREAD};
use crate::devcons;
use crate::devpipe;
use crate::devssl;
use crate::mcslock::Lock;
use crate::namec::{self, NamecMode};
use crate::pgrp::Pgrp;

/// The mksecret pair (see the module doc): the client arm passes
/// (FC, FS) as (secretin, secretout); srvp9auth passes them SWAPPED.
const FC: &[u8] = b"FE5567E8D76955085218";
const FS: &[u8] = b"2CDF69D74BB16DFF8E29";

/// `secretin <sec>` / `secretout <sec>` as the ctl-write bytes.
fn secin(sec: &[u8]) -> Vec<u8> {
    [b"secretin ".as_slice(), sec].concat()
}
fn secout(sec: &[u8]) -> Vec<u8> {
    [b"secretout ".as_slice(), sec].concat()
}

fn ready() -> std::sync::MutexGuard<'static, ()> {
    let g = devcons::tests::seam_guard();
    devcons::tests::install_user_hooks_once();
    devcons::tests::set_stub_user(b"glenda");
    devcons::tests::clear_stub_hostowner();
    devssl::install_test_state();
    g
}

/// A fresh pipe pair's two OPEN end chans (sys_pipe's construction,
/// called directly — the registry arms are 19-04 Task 3's wiring).
fn pipe_pair() -> (Chan, Chan) {
    let dir = devpipe::attach();
    let w0 = devpipe::walk(&dir, b"data").expect("walk data");
    let w1 = devpipe::walk(&dir, b"data1").expect("walk data1");
    let mut e0 = devpipe::open(&w0, ORDWR).expect("open data");
    let mut e1 = devpipe::open(&w1, ORDWR).expect("open data1");
    e0.mark_open();
    e1.mark_open();
    (e0, e1)
}

/// The pushssl conv setup: clone open (the returned chan IS the conv
/// ctl) + the decimal id read back + the data chan opened by name.
fn new_conv() -> (Chan, Chan, u32) {
    let root = devssl::attach();
    let ssl = devssl::walk(&root, b"ssl").expect("walk ssl");
    let clone = devssl::walk(&ssl, b"clone").expect("walk clone");
    let mut ctl = devssl::open(&clone, ORDWR).expect("clone open");
    ctl.mark_open();
    let mut idbuf = [0u8; 8];
    let n = devssl::read(&mut ctl, &mut idbuf, 0).expect("ctl read") as usize;
    let id: u32 = core::str::from_utf8(&idbuf[..n])
        .expect("decimal conv id")
        .parse()
        .expect("decimal conv id");
    let convdir = devssl::walk(&ssl, format!("{id}").as_bytes()).expect("walk conv dir");
    let dataf = devssl::walk(&convdir, b"data").expect("walk data");
    let mut data = devssl::open(&dataf, ORDWR).expect("open data");
    data.mark_open();
    (ctl, data, id)
}

/// Arm a conv the pushssl way: fd, optional secrets, alg.
fn arm(ctl: &mut Chan, fd: u64, in_sec: Option<&[u8]>, out_sec: Option<&[u8]>, alg: &[u8]) {
    devssl::write(ctl, format!("fd {fd}").as_bytes()).expect("fd");
    if let Some(s) = in_sec {
        devssl::write(ctl, &secin(s)).expect("secretin");
    }
    if let Some(s) = out_sec {
        devssl::write(ctl, &secout(s)).expect("secretout");
    }
    devssl::write(ctl, &[b"alg ".as_slice(), alg].concat()).expect("alg");
}

/// Pin 1 — the pushssl protocol walk + the verb ladder, including the
/// -e clear debug fallback's framed-but-clear round-trip and the
/// scoped algorithm-table reads.
#[test]
fn pushssl_protocol_walk_serves_the_verb_order_and_alg_clear() {
    let _g = ready();
    let (e0, e1) = pipe_pair();
    devssl::test_fd_put(7, e0);
    devssl::test_fd_put(8, e1);
    let (mut ctl_a, mut data_a, id_a) = new_conv();
    let (mut ctl_b, mut data_b, id_b) = new_conv();
    assert_eq!(id_a, 0, "the first clone allocates conv 0");
    assert_eq!(id_b, 1, "the second clone allocates conv 1");

    // The pushssl verb ladder, in order (fd -> secretin -> secretout
    // -> alg) — `alg clear` succeeds with no secrets armed (the -e
    // clear debug fallback, the goto-outx arm).
    assert!(devssl::write(&mut ctl_a, b"fd 7").is_ok(), "fd");
    assert!(devssl::write(&mut ctl_a, b"alg clear").is_ok(), "alg clear needs no secrets");
    // A bare secretin (no argument) is an Ebadarg-class reject.
    assert!(devssl::write(&mut ctl_a, b"secretin ").is_err(), "bare secretin is Ebadarg-class");

    // encalgs/hashalgs read the scoped tables.
    let root = devssl::attach();
    let ssl = devssl::walk(&root, b"ssl").unwrap();
    let convdir = devssl::walk(&ssl, b"0").unwrap();
    for (fname, want) in [
        (b"encalgs".as_slice(), b"rc4_256".as_slice()),
        (b"hashalgs".as_slice(), b"sha1".as_slice()),
    ] {
        let f = devssl::walk(&convdir, fname).expect("walk alg file");
        let mut f = devssl::open(&f, OREAD).expect("alg files open OREAD");
        f.mark_open();
        let mut buf = [0u8; 32];
        let n = devssl::read(&mut f, &mut buf, 0).expect("alg read") as usize;
        assert_eq!(&buf[..n], want, "the scoped alg table read");
    }

    // The clear-mode round-trip: both convs clear, the framed-but-clear
    // wire moves plaintext.
    arm(&mut ctl_b, 8, None, None, b"clear");
    let pt = b"clear is the debug fallback";
    assert_eq!(devssl::write(&mut data_a, pt).expect("clear write"), pt.len() as u64);
    let mut got = [0u8; 64];
    let n = devssl::read(&mut data_b, &mut got, u64::MAX).expect("clear read") as usize;
    assert_eq!(&got[..n], pt, "clear mode round-trips framed plaintext");

    // Now the full encrypted ladder on the same convs (re-alg after
    // clear — the verb machine runs per write): secrets then alg, the
    // order pushssl writes them.
    devssl::write(&mut ctl_a, &secin(FC)).expect("secretin");
    devssl::write(&mut ctl_a, &secout(FS)).expect("secretout");
    assert!(devssl::write(&mut ctl_a, b"alg rc4_256 sha1").is_ok(), "alg");
    arm(&mut ctl_b, 8, Some(FS), Some(FC), b"rc4_256 sha1");
}

/// Pin 2 — the ctl verb errors carry the donor's exact strings.
#[test]
fn ctl_verb_errors_carry_the_donor_strings() {
    let _g = ready();
    let (mut ctl, _data, _) = new_conv();

    // alg before fd — the ordering enforcement (devssl.c:1153).
    assert_eq!(
        devssl::write(&mut ctl, b"alg rc4_256 sha1"),
        Err("must set fd before algorithm"),
        "the donor's exact must-set-fd refusal"
    );

    // An unknown verb is an Ebadarg-class reject.
    assert_eq!(
        devssl::write(&mut ctl, b"frobnicate 1"),
        Err("bad arg in system call"),
        "Ebadarg verbatim"
    );

    // fd then alg with no secrets.
    let (e0, _e1) = pipe_pair();
    devssl::test_fd_put(3, e0);
    assert!(devssl::write(&mut ctl, b"fd 3").is_ok(), "fd");
    assert_eq!(
        devssl::write(&mut ctl, b"alg rc4_256 sha1"),
        Err("algorithm but no secret"),
        "the donor's no-secret refusal"
    );

    // Secrets armed, an unknown algorithm name.
    devssl::write(&mut ctl, &secin(FC)).expect("secretin");
    devssl::write(&mut ctl, &secout(FS)).expect("secretout");
    assert_eq!(
        devssl::write(&mut ctl, b"alg rc4_256 sha999"),
        Err("bad algorithm"),
        "the donor's bad-algorithm literal"
    );
}

/// Pin 3 — the end-to-end loopback over a REAL pipe pair: wire-format
/// inspection, the rc4_256+sha1 round-trip, and the corruption leg
/// (T-19-09: mangled frames fail loudly, never pass through).
#[test]
fn loopback_round_trips_rc4_256_sha1_and_detects_corruption() {
    let _g = ready();
    let (mut e0, mut e1) = pipe_pair();
    devssl::test_fd_put(7, e0.dup());
    devssl::test_fd_put(8, e1.dup());
    let (mut ctl_a, mut data_a, _) = new_conv();
    let (mut ctl_b, mut data_b, _) = new_conv();

    // Client arm on A, server arm on B (the swap — pin 4 drives it
    // home; a working loopback requires it).
    arm(&mut ctl_a, 7, Some(FC), Some(FS), b"rc4_256 sha1");
    arm(&mut ctl_b, 8, Some(FS), Some(FC), b"rc4_256 sha1");

    // One record on the wire; drain it RAW and inspect the SSL-format
    // header (2-byte |0x80 form, len counting digest+payload), then
    // reinject UNMODIFIED for the round-trip.
    let pt = b"harvey devssl loopback record";
    assert_eq!(devssl::write(&mut data_a, pt).expect("write"), pt.len() as u64);
    let mut wire = vec![0u8; 4096];
    let n = devpipe::read(&mut e1, &mut wire, u64::MAX).expect("drain the raw record") as usize;
    assert!(n >= 2 + 20 + pt.len(), "the record is header+digest+payload");
    assert_eq!(wire[0] & 0x80, 0x80, "the 2-byte header form (no pad)");
    assert_eq!(
        (((wire[0] & 0x7f) as usize) << 8) | wire[1] as usize,
        20 + pt.len(),
        "len counts digest+payload (the donor's SSL format)"
    );
    assert_ne!(&wire[2..], &pt[..], "the body is digest+ENCRYPTED payload, not plaintext");
    devpipe::write(&mut e0, &wire[..n], u64::MAX).expect("reinject");
    let mut got = vec![0u8; 256];
    let m = devssl::read(&mut data_b, &mut got, u64::MAX).expect("the peer decodes") as usize;
    assert_eq!(&got[..m], pt, "plaintext emerges from the peer's data");

    // The corruption leg: one flipped ciphertext bit (inside the
    // digest region) — the frame must fail loudly.
    let pt2 = b"a second record, mid=1";
    assert_eq!(devssl::write(&mut data_a, pt2).expect("write 2"), pt2.len() as u64);
    let n2 = devpipe::read(&mut e1, &mut wire, u64::MAX).expect("drain 2") as usize;
    wire[5] ^= 0x01;
    devpipe::write(&mut e0, &wire[..n2], u64::MAX).expect("reinject corrupted");
    assert_eq!(
        devssl::read(&mut data_b, &mut got, u64::MAX),
        Err("bad digest"),
        "a mangled frame fails loudly (T-19-09), never passes through"
    );
}

/// Pin 4 — the secrets-swapped server arm: srvp9auth's literal write
/// order (secretin FS then secretout FC) round-trips BOTH directions
/// against the client arm, and the unswapped pairing (client order
/// meets client order) fails the digest — the swap is load-bearing.
#[test]
fn swapped_server_arm_secrets_round_trip_both_directions() {
    let _g = ready();

    // The swapped pair: A client order, B srvp9auth order, one pipe.
    let (e0a, e1a) = pipe_pair();
    devssl::test_fd_put(7, e0a.dup());
    devssl::test_fd_put(8, e1a.dup());
    let (mut ctl_a, mut data_a, _) = new_conv();
    let (mut ctl_b, mut data_b, _) = new_conv();
    arm(&mut ctl_a, 7, Some(FC), Some(FS), b"rc4_256 sha1");
    arm(&mut ctl_b, 8, Some(FS), Some(FC), b"rc4_256 sha1");

    // Client -> server.
    let up = b"client to server over the swapped pair";
    devssl::write(&mut data_a, up).expect("a writes");
    let mut got = vec![0u8; 256];
    let n = devssl::read(&mut data_b, &mut got, u64::MAX).expect("b reads") as usize;
    assert_eq!(&got[..n], up, "client->server through the swap");

    // Server -> client (full duplex: B's out=FC meets A's in=FC).
    let down = b"and the server talks back";
    devssl::write(&mut data_b, down).expect("b writes");
    let n = devssl::read(&mut data_a, &mut got, u64::MAX).expect("a reads") as usize;
    assert_eq!(&got[..n], down, "server->client through the swap");

    // The negative: an UNSWAPPED peer — client order meets client
    // order (what the server would be WITHOUT srvp9auth's swap).
    let (e0n, e1n) = pipe_pair();
    devssl::test_fd_put(9, e0n);
    devssl::test_fd_put(10, e1n);
    let (mut ctl_c, mut data_c, _) = new_conv();
    let (mut ctl_d, mut data_d, _) = new_conv();
    arm(&mut ctl_c, 9, Some(FC), Some(FS), b"rc4_256 sha1");
    arm(&mut ctl_d, 10, Some(FC), Some(FS), b"rc4_256 sha1");
    devssl::write(&mut data_c, b"this must not decode").expect("c writes");
    let mut got2 = vec![0u8; 256];
    assert_eq!(
        devssl::read(&mut data_d, &mut got2, u64::MAX),
        Err("bad digest"),
        "client order meets client order: without the swap the digest fails"
    );
}

/// Task 3's registry-level proof (the plan's acceptance criterion): the
/// guest's name-resolution surface — namec's '#D' arm, the per-device
/// walk dispatch, and the open funnel — resolves #D/ssl/clone
/// end-to-end, the exact path a guest process's open() rides (no boot
/// profile declares a dev key for #D this era; this pin is the
/// substrate evidence, the live encrypted proof is 19-08's -G gate).
#[test]
fn registry_level_name_resolution_of_the_d_walk() {
    let _g = ready();
    let anchor = devssl::attach();
    let pg = Lock::new("devsslpgtest", Pgrp::new());
    let clone = namec::namec(b"#D/ssl/clone", &anchor, &anchor, &pg, NamecMode::Open)
        .expect("#D/ssl/clone resolves through namec's registry arm");
    assert_eq!(clone.qid.path & 0xf, 3, "the clone qid (QID(0, Qclonus))");
    let mut ctl = namec::open_chan(&clone, ORDWR).expect("clone opens through the funnel");
    let mut buf = [0u8; 8];
    let n = devssl::read(&mut ctl, &mut buf, 0).expect("ctl read") as usize;
    assert_eq!(&buf[..n], b"0", "the fresh conv's decimal id");
}
