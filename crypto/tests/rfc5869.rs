//! Host-side pins for HKDF-SHA256 (CRYP-01, RFC 5869).
//!
//! Every expected value is copied from the RFC text at rfc-editor.org —
//! appendix A.1 to A.3, test cases 1 to 3 — and never re-derived from the
//! implementation under test (03-RESEARCH.md Pitfall 10).  This file imports
//! only `crypto`'s own API, never an upstream crate, so the pins prove the
//! wrapper (D-18).

use crypto::kdf::*;
use crypto::Error;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// RFC 5869 §A.1 test case 1 (SHA-256, basic): IKM = 0x0b × 22,
/// salt = 0x000102030405060708090a0b0c, info = 0xf0..0xf9, L = 42.
#[test]
fn hkdf_sha256_pin_rfc5869_case1() {
    let ikm = [0x0bu8; 22];
    let salt = [0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c];
    let info = [0xf0u8, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];

    let prk = hkdf_extract(&salt, &ikm);
    assert_eq!(
        hex(&prk),
        "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5",
        "HKDF-SHA256 RFC 5869 case 1 PRK must match"
    );

    let mut okm = [0u8; 42];
    hkdf_expand(&prk, &info, &mut okm).expect("42 bytes is within the RFC 5869 limit");
    assert_eq!(
        hex(&okm),
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865",
        "HKDF-SHA256 RFC 5869 case 1 OKM must match"
    );
}

/// RFC 5869 §A.2 test case 2 (SHA-256, longer inputs and outputs):
/// IKM = 0x00..0x4f, salt = 0x60..0xaf, info = 0xb0..0xff, L = 82.
#[test]
fn hkdf_sha256_pin_rfc5869_case2() {
    let ikm: Vec<u8> = (0x00u8..=0x4f).collect();
    let salt: Vec<u8> = (0x60u8..=0xaf).collect();
    let info: Vec<u8> = (0xb0u8..=0xff).collect();

    let prk = hkdf_extract(&salt, &ikm);
    assert_eq!(
        hex(&prk),
        "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244",
        "HKDF-SHA256 RFC 5869 case 2 PRK must match"
    );

    let mut okm = [0u8; 82];
    hkdf_expand(&prk, &info, &mut okm).expect("82 bytes is within the RFC 5869 limit");
    assert_eq!(
        hex(&okm),
        "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c\
         59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71\
         cc30c58179ec3e87c14c01d5c1f3434f1d87",
        "HKDF-SHA256 RFC 5869 case 2 OKM must match"
    );
}

/// RFC 5869 §A.3 test case 3 (SHA-256, zero-length salt and info):
/// IKM = 0x0b × 22, salt = 0 octets, info = 0 octets, L = 42.
#[test]
fn hkdf_sha256_pin_rfc5869_case3() {
    let ikm = [0x0bu8; 22];

    let prk = hkdf_extract(&[], &ikm);
    assert_eq!(
        hex(&prk),
        "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04",
        "HKDF-SHA256 RFC 5869 case 3 PRK must match"
    );

    let mut okm = [0u8; 42];
    hkdf_expand(&prk, &[], &mut okm).expect("42 bytes is within the RFC 5869 limit");
    assert_eq!(
        hex(&okm),
        "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8",
        "HKDF-SHA256 RFC 5869 case 3 OKM must match"
    );
}

/// RFC 5869 §2.3's length rule: L <= 255 × HashLen.  8160 bytes (255 × 32)
/// succeeds, 8161 bytes returns `OutputTooLong` and writes nothing, and a
/// zero-length output is `Ok`.
#[test]
fn hkdf_expand_length_limits() {
    let prk = [0x5au8; 32];

    let mut at_limit = vec![0u8; 8160];
    assert_eq!(
        hkdf_expand(&prk, b"length-limits", &mut at_limit),
        Ok(()),
        "8160 bytes is exactly 255 x 32, the RFC 5869 maximum"
    );

    let mut over_limit = vec![0xa5u8; 8161];
    assert_eq!(
        hkdf_expand(&prk, b"length-limits", &mut over_limit),
        Err(Error::OutputTooLong),
        "8161 bytes must be refused, not truncated"
    );
    assert!(
        over_limit.iter().all(|&b| b == 0xa5),
        "an over-long request must leave the output buffer untouched"
    );

    let mut empty: [u8; 0] = [];
    assert_eq!(
        hkdf_expand(&prk, b"length-limits", &mut empty),
        Ok(()),
        "a zero-length output is Ok"
    );
}

/// RFC 5869 §2.2/§2.3 ordering: `hkdf_extract(salt, ikm)` is not symmetric
/// in its arguments, and `info` is an ordered concatenation, so `a||b`
/// differs from `b||a`.
#[test]
fn hkdf_argument_order_is_pinned() {
    let salt = [0x11u8; 13];
    let ikm = [0x22u8; 22];

    let forward = hkdf_extract(&salt, &ikm);
    let swapped = hkdf_extract(&ikm, &salt);
    assert_ne!(
        forward, swapped,
        "hkdf_extract(salt, ikm) must differ from hkdf_extract(ikm, salt)"
    );

    let mut ab = [0u8; 32];
    let mut ba = [0u8; 32];
    hkdf_expand(&forward, &[0xa1, 0xb2], &mut ab).expect("32 bytes is within the limit");
    hkdf_expand(&forward, &[0xb2, 0xa1], &mut ba).expect("32 bytes is within the limit");
    assert_ne!(ab, ba, "info is an ordered concatenation: a||b differs from b||a");
}

/// RFC 2104 §2 / RFC 4231: an empty key and an empty message are both legal
/// and still produce a 32-byte tag.  No value is asserted from memory.
#[test]
fn hmac_sha256_accepts_empty_key_and_message() {
    let tag = hmac_sha256(&[], &[]);
    assert_eq!(
        tag.len(),
        32,
        "an empty key and an empty message still produce a 32-byte tag"
    );
}
