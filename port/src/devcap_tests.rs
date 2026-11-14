//! The devcap host pins (18-02, AUTH-03): the donor-verbatim semantics
//! of `#¤` (sys/src/9k/port/devcap.c) proven deterministically on the
//! host — no kernel context, the stub-hook discipline.  Homed at the
//! crate root beside namec_tests.rs (the registration block in lib.rs):
//! the stub current-user/tick hooks need std while port::devcap keeps
//! its unsafe-free forbid attribute.
//!
//! THE PIN MAP (the plan's 7 classes + the boot-fixture cross-check):
//!
//! - (a) round-trip: post + consume, the user MUTATES to `to`
//! - (b) one-shot: the second present of the same cap -> `invalid
//!   capability` (remcap unlinked it)
//! - (c) FIFO trim at Maxhash=256: the 257th post evicts the OLDEST;
//!   the evicted cap no longer consumes, the newest still does
//! - (d) hostowner gate: non-eve caphash OPEN and WRITE -> Eperm-class
//!   statics
//! - (e) user-match: from@to with from != current user ->
//!   `capability must match user` (verbatim), and the donor's ORDER —
//!   the cap is CONSUMED before the mismatch fires (burned either way)
//! - (f) wrong HMAC: a bit-flipped key -> `invalid capability` with the
//!   formatted from@to@key payload
//! - (g) single-user form: no '@' in the from-to part -> the user
//!   re-installs as itself
//! - (h) the eve remove hide: caphash LAST + ncapdir shrink — walk
//!   errors, the listing drops it, a non-eve remove refuses
//! - (i) the boot-marker fixture: the precomputed cap pair the q35auth
//!   driver embeds (randread.c) cross-checked against THIS module —
//!   python-computed at implementation time, Rust-verified forever
//!
//! Harness discipline (devip_tests): every stateful pin serializes on
//! the devcons user-seam lock (the stub user + HOSTOWNER + CAPALLOC are
//! process-global) and installs a FRESH device state
//! (devcap::install_test_state) + the glenda stub user.

use crate::dat::Chan;
use crate::devcap::{self, CapErr};
use crate::devcons;
use crate::hmacsha1;

// ---------------------------------------------------------------------------
// The stub seam: the devcons tests module owns the ONE user-hook
// registration for this binary (its Once); devcap pins borrow it.
// ---------------------------------------------------------------------------

fn ready() -> std::sync::MutexGuard<'static, ()> {
    let g = devcons::tests::seam_guard();
    devcons::tests::install_user_hooks_once();
    devcons::tests::set_stub_user(b"glenda");
    devcons::tests::clear_stub_hostowner();
    devcap::install_test_state();
    g
}

/// A resolved (unopened) devcap chan for `name` via the device's own
/// attach+walk — the path namec mints for `#¤/name`.
fn chan_for(name: &[u8]) -> Chan {
    let root = devcap::attach();
    devcap::walk(&root, name).expect("walk")
}

/// The hash the userspace contract prescribes (as.c mkcap):
/// HMAC-SHA1(data = from@to, key = the base64-random text).
fn cap_hash(from_to: &[u8], key: &[u8]) -> [u8; 20] {
    hmacsha1::hmac_sha1(from_to, key)
}

/// Post a cap's hash as the hostowner (the caphash write leg).
fn post(from_to: &[u8], key: &[u8]) -> core::result::Result<u64, CapErr> {
    let mut c = chan_for(b"caphash");
    devcap::write(&mut c, &cap_hash(from_to, key))
}

/// Present a cap (the capuse write leg): the full
/// from@to@base64 string.
fn present(cap: &[u8]) -> core::result::Result<u64, CapErr> {
    let mut c = chan_for(b"capuse");
    devcap::write(&mut c, cap)
}

fn glenda_norbert_cap(key: &[u8]) -> Vec<u8> {
    let mut cap = b"glenda@norbert@".to_vec();
    cap.extend_from_slice(key);
    cap
}

// (a) Round-trip + (f's happy twin): post, consume, the user mutates.
#[test]
fn devcap_round_trip_posts_consumes_and_mutates_the_user() {
    let _g = ready();
    let key = b"bTpA7isQXxAD7mV7Y3GX3WuHVNk=";
    assert_eq!(post(b"glenda@norbert", key).unwrap(), 20);
    let cap = glenda_norbert_cap(key);
    assert_eq!(present(&cap).unwrap(), cap.len() as u64);
    assert_eq!(
        devcons::current_user().unwrap(),
        b"norbert".to_vec(),
        "capuse success mutates the consuming proc's user to `to`"
    );
}

// (b) One-shot: the second present of the SAME cap errors invalid.
#[test]
fn devcap_second_present_of_the_same_cap_is_rejected() {
    let _g = ready();
    let key = b"oneshot-key-fixture";
    post(b"glenda@norbert", key).unwrap();
    let cap = glenda_norbert_cap(key);
    present(&cap).expect("first present consumes");
    match present(&cap) {
        Err(CapErr::InvalidCapability(msg)) => assert_eq!(
            msg,
            b"invalid capability glenda@norbert@oneshot-key-fixture".to_vec(),
            "the donor's formatted error, verbatim"
        ),
        other => panic!("second present must reject: {other:?}"),
    }
    // The user STAYS at the first present's `to` — the rejected second
    // present changes nothing (norbert, not glenda).
    assert_eq!(devcons::current_user().unwrap(), b"norbert".to_vec());
}

// (c) FIFO trim at Maxhash=256: the OLDEST drops when the 257th lands.
#[test]
fn devcap_fifo_trim_at_256_evicts_the_oldest() {
    let _g = ready();
    // Distinct caps: from@to pairs user0000..user0257, each with its
    // own key — hashes computed by the module under pin.
    for i in 0..=256u32 {
        let from_to = format!("glenda@user{i:04}").into_bytes();
        let key = format!("key{i:04}");
        post(&from_to, key.as_bytes()).expect("post");
    }
    // The table held 257 -> cap 0 (the OLDEST) was evicted.
    let cap0 = b"glenda@user0000@key0000".to_vec();
    match present(&cap0) {
        Err(CapErr::InvalidCapability(_)) => {}
        other => panic!("the evicted oldest must no longer consume: {other:?}"),
    }
    // The newest (257th, index 256) is still live.
    let cap256 = b"glenda@user0256@key0256".to_vec();
    present(&cap256).expect("the newest still consumes");
    assert_eq!(devcons::current_user().unwrap(), b"user0256".to_vec());
}

// (d) Hostowner gate: a non-eve user cannot OPEN caphash and cannot
// WRITE it (both Eperm-class).
#[test]
fn devcap_caphash_is_hostowner_only_on_open_and_write() {
    let _g = ready();
    devcons::tests::set_stub_user(b"norbert");
    let caphash = chan_for(b"caphash");
    assert!(
        devcap::open(&caphash, crate::dat::OWRITE).is_err(),
        "non-eve caphash open must fail"
    );
    match devcap::write(&mut chan_for(b"caphash"), &[7u8; 20]) {
        Err(CapErr::Static(msg)) => assert!(msg.contains("hostowner"), "got {msg:?}"),
        other => panic!("non-eve caphash write must fail: {other:?}"),
    }
    // eve (the default glenda base) posts fine.
    devcons::tests::set_stub_user(b"glenda");
    post(b"glenda@norbert", b"key").expect("eve posts");
}

// (e) User-match: from != current user -> the verbatim donor string,
// and the cap BURNED (the donor's remcap-before-check order).
#[test]
fn devcap_from_to_form_must_match_the_current_user() {
    let _g = ready();
    let key = b"match-key";
    post(b"bootes@norbert", key).expect("post");
    // Current user is glenda; the cap speaks for bootes.
    match present(b"bootes@norbert@match-key") {
        Err(CapErr::Static(msg)) => {
            assert_eq!(msg, "capability must match user", "the donor literal, verbatim")
        }
        other => panic!("mismatched from must reject: {other:?}"),
    }
    // Donor order: remcap ran BEFORE the match check — the cap is gone.
    match present(b"bootes@norbert@match-key") {
        Err(CapErr::InvalidCapability(_)) => {}
        other => panic!("the burned cap must not consume twice: {other:?}"),
    }
    // A matching from consumes — the POST still needs eve (glenda
    // posts bootes' cap; bootes presents it).
    post(b"bootes@norbert", key).expect("re-post as eve");
    devcons::tests::set_stub_user(b"bootes");
    present(b"bootes@norbert@match-key").expect("matching from consumes");
    assert_eq!(devcons::current_user().unwrap(), b"norbert".to_vec());
}

// (f) Wrong HMAC: a bit-flipped key hashes differently -> invalid
// capability (the formatted payload carries the presented pair).
#[test]
fn devcap_wrong_hmac_key_is_rejected() {
    let _g = ready();
    post(b"glenda@norbert", b"correct-key").expect("post");
    // One bit flipped in the key: k -> j (0x6b -> 0x6a).
    match present(b"glenda@norbert@jorrect-key") {
        Err(CapErr::InvalidCapability(msg)) => {
            assert_eq!(msg, b"invalid capability glenda@norbert@jorrect-key".to_vec())
        }
        other => panic!("a wrong-HMAC present must reject: {other:?}"),
    }
    // The genuine key still consumes afterwards.
    present(b"glenda@norbert@correct-key").expect("the genuine key consumes");
}

// (g) Single-user form: no '@' in the from-to part -> to = from.
#[test]
fn devcap_single_user_form_reinstalls_the_same_user() {
    let _g = ready();
    post(b"glenda", b"solo-key").expect("post");
    present(b"glenda@solo-key").expect("single-user present");
    assert_eq!(devcons::current_user().unwrap(), b"glenda".to_vec());
}

// (h) The eve remove hide: caphash LAST, ncapdir shrink, permanent.
#[test]
fn devcap_eve_remove_hides_caphash_permanently() {
    let _g = ready();
    // Non-eve remove: Eperm.
    devcons::tests::set_stub_user(b"norbert");
    assert!(devcap::remove(&chan_for(b"caphash")).is_err(), "non-eve remove refuses");
    // Eve remove: the served table shrinks — walk, stat, listing lose
    // caphash; capuse stays.
    devcons::tests::set_stub_user(b"glenda");
    devcap::remove(&chan_for(b"caphash")).expect("eve removes caphash");
    assert!(devcap::walk(&devcap::attach(), b"caphash").is_err(), "caphash no longer walks");
    assert!(devcap::walk(&devcap::attach(), b"capuse").is_ok(), "capuse still walks");
    let mut root = devcap::attach();
    let mut listing = [0u8; 512];
    let n = devcap::read(&mut root, &mut listing, 0).expect("listing") as usize;
    let text = String::from_utf8_lossy(&listing[..n]).to_string();
    assert!(!text.contains("caphash"), "the listing hides caphash");
    // Permanent: eve cannot un-hide (no path back — the donor's shape).
    assert!(devcap::remove(&chan_for(b"capuse")).is_err(), "removing capuse is Eperm");
}

// (i) The boot-marker fixture: randread.c's embedded pair, pinned
// against THIS module (python-computed 2026-11-14; the Rust module must
// agree forever — a divergence means the boot leg is broken).
#[test]
fn devcap_boot_fixture_hash_matches_the_module() {
    let _g = ready();
    let key = b"bTpA7isQXxAD7mV7Y3GX3WuHVNk=";
    let digest = cap_hash(b"glenda@norbert", key);
    assert_eq!(
        digest,
        [
            0xf5, 0xec, 0xda, 0xc0, 0x18, 0x1c, 0x47, 0x13, 0x71, 0xfc, 0xf9, 0x7b, 0xf8,
            0x85, 0xf9, 0x90, 0x22, 0xb7, 0xe2, 0xe3,
        ],
        "the randread.c fixture HMAC must reproduce from this module"
    );
    // And the full device leg over the fixture (the same bytes the
    // q35auth driver writes): post -> consume -> user = norbert.
    post(b"glenda@norbert", key).expect("fixture post");
    present(&glenda_norbert_cap(key)).expect("fixture present");
    assert_eq!(devcons::current_user().unwrap(), b"norbert".to_vec());
}

// The Eshort class: no '@' at all in the capuse write, and a short
// caphash post (the POST leg takes the 20 RAW hash bytes — a 1-byte
// post is Eshort).
#[test]
fn devcap_short_writes_error() {
    let _g = ready();
    match present(b"no-at-sign-here") {
        Err(CapErr::Static(msg)) => assert!(msg.contains("too small"), "got {msg:?}"),
        other => panic!("a cap with no '@' must Eshort: {other:?}"),
    }
    match devcap::write(&mut chan_for(b"caphash"), b"short") {
        Err(CapErr::Static(msg)) => assert!(msg.contains("too small"), "got {msg:?}"),
        other => panic!("a short caphash post must Eshort: {other:?}"),
    }
}
