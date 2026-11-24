//! Host-side pins for the key hierarchy (CRYP-04, D-22).
//!
//! `crypto::key::k_file` is the RFC §3.2 `HKDF-Expand(K_user, Qid.path ||
//! Qid.vers, 32)` with the fixed 8-byte then 4-byte little-endian encoding,
//! and `derive_labelled` is the unframed labelled expand.  Tests compare keys
//! through `Key::ct_eq`: `Key` has no public accessor that returns bytes.

use crypto::key::{Key, derive_labelled, k_file};

/// The `K_user` of the known-answer vector: the bytes `0x00..=0x1f`.
fn k_user() -> Key {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = i as u8;
    }
    Key::from_bytes(bytes)
}

/// Decode 32 bytes from a 64-digit hex literal.
fn hex32(hex: &str) -> [u8; 32] {
    assert_eq!(hex.len(), 64, "a key literal is 32 bytes of hex");
    let bytes = hex.as_bytes();
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let pair = core::str::from_utf8(&bytes[2 * i..2 * i + 2]).expect("hex is ascii");
        *byte = u8::from_str_radix(pair, 16).expect("hex digits");
    }
    out
}

/// The fixed Qid of `k_file_known_answer`.
const KAT_PATH: u64 = 0x0102_0304_0506_0708;
const KAT_VERS: u32 = 0x0a0b_0c0d;

/// The known answer for `k_user()`, `KAT_PATH`, `KAT_VERS`, computed
/// independently of this crate (Python `hmac` + `hashlib`, HKDF-Expand per
/// RFC 5869 §2.3 with `info` = path little-endian then vers little-endian).
/// The command and its output are recorded in `evidence/PINS-03-04.md`
/// (`KAT k_file`).
const KAT_EXPECTED: &str = "ce90009aebce083580333aeaf1420104c25b12b6a1151dfe8fd5b2420cf10afe";

#[test]
fn k_file_known_answer() {
    let derived = k_file(&k_user(), KAT_PATH, KAT_VERS);
    assert!(
        derived.ct_eq(&Key::from_bytes(hex32(KAT_EXPECTED))),
        "K_file must equal the independently computed vector {KAT_EXPECTED}"
    );
}

#[test]
fn k_file_binds_path_and_vers() {
    let user = k_user();
    // The plan's adjacency cases: (0x100, 0), (0, 1) and (1, 0) are three
    // different keys.
    let a = k_file(&user, 0x100, 0);
    let b = k_file(&user, 0, 1);
    let c = k_file(&user, 1, 0);
    assert!(!a.ct_eq(&b), "(path=0x100, vers=0) and (path=0, vers=1) differ");
    assert!(!a.ct_eq(&c), "(path=0x100, vers=0) and (path=1, vers=0) differ");
    assert!(!b.ct_eq(&c), "(path=0, vers=1) and (path=1, vers=0) differ");

    // One bit of `path` or of `vers` is enough to move the key.
    let base = k_file(&user, 0x100, 0);
    for bit in 0..64 {
        let path = 0x100u64 ^ (1u64 << bit);
        assert!(
            !base.ct_eq(&k_file(&user, path, 0)),
            "flipping bit {bit} of Qid.path must change the key"
        );
    }
    for bit in 0..32 {
        let vers = 1u32 << bit;
        assert!(
            !base.ct_eq(&k_file(&user, 0x100, vers)),
            "flipping bit {bit} of Qid.vers must change the key"
        );
    }
}

#[test]
fn k_file_and_derive_accept_zero_and_empty() {
    let parent = k_user();

    let zero_file = k_file(&parent, 0, 0);
    assert!(
        zero_file.ct_eq(&k_file(&parent, 0, 0)),
        "K_file with path=0, vers=0 is deterministic"
    );
    assert!(
        !zero_file.ct_eq(&parent),
        "K_file with path=0, vers=0 differs from K_user"
    );

    let empty_labelled = derive_labelled(&parent, b"", b"");
    assert!(
        empty_labelled.ct_eq(&derive_labelled(&parent, b"", b"")),
        "derive_labelled with an empty label and context is deterministic"
    );
    assert!(
        !empty_labelled.ct_eq(&parent),
        "derive_labelled with an empty label and context differs from the parent"
    );
    assert!(
        !empty_labelled.ct_eq(&zero_file),
        "the empty labelled expand and K_file(0, 0) differ from each other"
    );
}

#[test]
fn derive_labelled_concatenation_is_pinned() {
    let parent = k_user();
    // Unframed concatenation: ("ab", "c") and ("a", "bc") produce the same
    // `info`, so the same key.  This pin makes a later framing change a
    // visible advance rather than a silent behaviour change.
    assert!(
        derive_labelled(&parent, b"ab", b"c").ct_eq(&derive_labelled(&parent, b"a", b"bc")),
        "derive_labelled concatenates label then context with no framing"
    );
    // Order still matters: ("a", "b") and ("b", "a") are different info.
    assert!(
        !derive_labelled(&parent, b"a", b"b").ct_eq(&derive_labelled(&parent, b"b", b"a")),
        "swapping label and context must change the key"
    );
}

// ---------------------------------------------------------------------------
// KeyCapability: the read-write / read-only typestate (CRYP-04, D-22)
// ---------------------------------------------------------------------------

use crypto::Error;
use crypto::key::KeyCapability;

/// A fixed nonce for the keystream tests.
const NONCE: [u8; 12] = [0x5a; 12];

#[test]
fn read_write_seals_and_verifies() {
    let capability =
        KeyCapability::new(Key::from_bytes([0x11; 32]), Key::from_bytes([0x22; 32]));
    let tag = capability.seal(b"page 7");
    assert_eq!(capability.verify(b"page 7", &tag), Ok(()), "the sealed tag verifies");

    let mut flipped = tag;
    flipped[0] ^= 1;
    assert_eq!(
        capability.verify(b"page 7", &flipped),
        Err(Error::TagMismatch),
        "one flipped tag bit must fail verification"
    );
}

#[test]
fn read_only_verifies_what_read_write_sealed() {
    let capability =
        KeyCapability::new(Key::from_bytes([0x11; 32]), Key::from_bytes([0x22; 32]));
    let tag = capability.seal(b"page 7");
    let sibling = capability.read_only();
    assert_eq!(sibling.verify(b"page 7", &tag), Ok(()), "the sibling verifies the parent's tag");

    let mut flipped = tag;
    flipped[0] ^= 1;
    assert_eq!(
        sibling.verify(b"page 7", &flipped),
        Err(Error::TagMismatch),
        "the sibling rejects a flipped tag"
    );
}

#[test]
fn read_only_sibling_keeps_parent_k_dec() {
    let capability =
        KeyCapability::new(Key::from_bytes([0x33; 32]), Key::from_bytes([0x44; 32]));
    let plaintext = *b"the page plaintext";
    let mut buf = plaintext;
    capability
        .apply_keystream(&NONCE, 7, &mut buf)
        .expect("the parent encrypts");
    assert_ne!(buf, plaintext, "the parent's keystream changed the buffer");

    let sibling = capability.read_only();
    sibling
        .apply_keystream(&NONCE, 7, &mut buf)
        .expect("the sibling decrypts");
    assert_eq!(
        buf, plaintext,
        "the read-only sibling inherits the parent's k_dec and restores the plaintext"
    );
}

#[test]
fn mac_key_is_separate_from_cipher_key() {
    let k_dec = Key::from_bytes([0x55; 32]);
    let k_mac = Key::from_bytes([0x66; 32]);
    let sealing = KeyCapability::new(k_dec.clone(), k_mac);
    let tag = sealing.seal(b"page 9");

    // A capability whose MAC key is the first one's *cipher* key must not
    // verify the tag: draining k_dec into k_mac would be a key reuse
    // (T-03-18).
    let other = KeyCapability::new(Key::from_bytes([0x77; 32]), k_dec);
    assert_eq!(
        other.verify(b"page 9", &tag),
        Err(Error::TagMismatch),
        "a tag sealed under k_mac must not verify under k_dec"
    );
}
