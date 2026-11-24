//! kdf — key derivation on HMAC-SHA256: the HMAC primitive (RFC 4231) and,
//! when it lands, HKDF extract/expand (RFC 5869) (CRYP-01).
//!
//! Scope note (deliberate): HMAC exists in r9 only over SHA-256, as the PRF
//! under HKDF (D-07).  "HMAC-BLAKE3" in the source documents is read as
//! BLAKE3's native keyed mode, so no HMAC construction is built over BLAKE3.
//!
//! The known-answer pins live in `crypto/tests/rfc4231.rs`: the expected
//! values are copied from the RFC text, never re-derived from this
//! implementation (03-RESEARCH.md Pitfall 10).

/// The SHA-256 output length in bytes (FIPS 180-4).
pub const SHA256_LEN: usize = 32;

/// One-shot HMAC-SHA256 (RFC 2104 §2 over SHA-256, RFC 4231 vectors).
///
/// Any key length is accepted: RFC 2104 hashes keys longer than the 64-byte
/// block and zero-pads shorter ones.  The 32-byte tag is returned by value so
/// no caller-visible allocation is needed.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; SHA256_LEN] {
    // RED skeleton (03-01): the API and the RFC 4231 pin land first; the body
    // is wrong on behaviour until the GREEN commit implements it over
    // `hmac::Hmac<sha2::Sha256>`.
    let _ = (key, msg);
    [0u8; SHA256_LEN]
}
