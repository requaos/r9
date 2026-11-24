//! Host-side pins for HMAC-SHA256 (CRYP-01, RFC 4231).
//!
//! The expected values are copied from the RFC text at rfc-editor.org —
//! section 4.2 to 4.8, test cases 1 to 7 — and never re-derived from the
//! implementation under test (03-RESEARCH.md Pitfall 10).  This file imports
//! only `crypto`'s own API, never an upstream crate, so the pin proves the
//! wrapper (D-18).

use crypto::kdf::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// RFC 4231 §4.2 test case 1: key = 0x0b × 20, data = "Hi There".
#[test]
fn hmac_sha256_pin_rfc4231_case1() {
    let key = [0x0bu8; 20];
    assert_eq!(
        hex(&hmac_sha256(&key, b"Hi There")),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        "HMAC-SHA256 RFC 4231 case 1 must match"
    );
}

/// RFC 4231 §4.3 test case 2: key = "Jefe", data = "what do ya want for
/// nothing?".
#[test]
fn hmac_sha256_pin_rfc4231_case2() {
    assert_eq!(
        hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
        "HMAC-SHA256 RFC 4231 case 2 must match"
    );
}

/// RFC 4231 §4.4 test case 3: key = 0xaa × 20, data = 0xdd × 50.
#[test]
fn hmac_sha256_pin_rfc4231_case3() {
    let key = [0xaau8; 20];
    let data = [0xddu8; 50];
    assert_eq!(
        hex(&hmac_sha256(&key, &data)),
        "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
        "HMAC-SHA256 RFC 4231 case 3 must match"
    );
}

/// RFC 4231 §4.5 test case 4: key = 0x01..0x19 (25 bytes), data = 0xcd × 50.
#[test]
fn hmac_sha256_pin_rfc4231_case4() {
    let key: [u8; 25] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
        0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19,
    ];
    let data = [0xcdu8; 50];
    assert_eq!(
        hex(&hmac_sha256(&key, &data)),
        "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b",
        "HMAC-SHA256 RFC 4231 case 4 must match"
    );
}

/// RFC 4231 §4.6 test case 5: key = 0x0c × 20, data = "Test With
/// Truncation".  The RFC truncates the tag to 128 bits, so the first 16
/// bytes are compared against the RFC value.
#[test]
fn hmac_sha256_pin_rfc4231_case5() {
    let key = [0x0cu8; 20];
    let tag = hmac_sha256(&key, b"Test With Truncation");
    assert_eq!(
        hex(&tag[..16]),
        "a3b6167473100ee06e0c796c2955552b",
        "HMAC-SHA256 RFC 4231 case 5 (128-bit truncation) must match"
    );
}

/// RFC 4231 §4.7 test case 6: key = 0xaa × 131, data = "Test Using Larger
/// Than Block-Size Key - Hash Key First".
#[test]
fn hmac_sha256_pin_rfc4231_case6() {
    let key = [0xaau8; 131];
    assert_eq!(
        hex(&hmac_sha256(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First"
        )),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
        "HMAC-SHA256 RFC 4231 case 6 must match"
    );
}

/// RFC 4231 §4.8 test case 7: key = 0xaa × 131, data = the RFC's 152-byte
/// sentence about keys and data larger than the block size.
#[test]
fn hmac_sha256_pin_rfc4231_case7() {
    let key = [0xaau8; 131];
    let data = b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.";
    assert_eq!(
        hex(&hmac_sha256(&key, data)),
        "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
        "HMAC-SHA256 RFC 4231 case 7 must match"
    );
}
