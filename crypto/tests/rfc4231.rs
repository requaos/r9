//! Host-side pins for HMAC-SHA256 (CRYP-01, RFC 4231).
//!
//! The expected value is copied from the RFC text at rfc-editor.org —
//! section 4.2, test case 1 — and never re-derived from the implementation
//! under test (03-RESEARCH.md Pitfall 10).  This file imports only `crypto`'s
//! own API, never an upstream crate, so the pin proves the wrapper (D-18).

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
