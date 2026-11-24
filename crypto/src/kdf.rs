//! kdf — key derivation on HMAC-SHA256: the HMAC primitive (RFC 4231) and
//! HKDF extract/expand (RFC 5869) (CRYP-01).
//!
//! Scope note (deliberate): HMAC exists in r9 only over SHA-256, as the PRF
//! under HKDF (D-07).  "HMAC-BLAKE3" in the source documents is read as
//! BLAKE3's native keyed mode, so no HMAC construction is built over BLAKE3.
//!
//! Pinned standards: HMAC-SHA256 against RFC 4231 §4.2 to §4.8 (seven test
//! cases), and HKDF-SHA256 against RFC 5869 §A.1 to §A.3 (three test cases)
//! on both the host default backend and the scalar backend the kernels run.
//! The known-answer pins live in `crypto/tests/rfc4231.rs` and
//! `crypto/tests/rfc5869.rs`: the expected values are copied from the RFC
//! text, never re-derived from this implementation (03-RESEARCH.md
//! Pitfall 10).  The pin results are recorded in
//! `evidence/PINS-03-03.md`.

use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// The SHA-256 output length in bytes (FIPS 180-4).
pub const SHA256_LEN: usize = 32;

/// One-shot HMAC-SHA256 (RFC 2104 §2 over SHA-256, RFC 4231 vectors).
///
/// Any key length is accepted: RFC 2104 hashes keys longer than the 64-byte
/// block and zero-pads shorter ones.  The 32-byte tag is returned by value so
/// no caller-visible allocation is needed.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; SHA256_LEN] {
    // A thin wrapper (03-RESEARCH.md Pattern 1): `Hmac`'s keyed state is the
    // upstream type and never appears in this signature (D-18).
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key)
        .expect("HMAC accepts a key of any length (RFC 2104 §3)");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// The largest HKDF-SHA256 output: 255 × 32 bytes, the RFC 5869 §2.3 limit
/// (`L <= 255 * HashLen`).  A longer request is refused, never truncated.
pub const HKDF_MAX_OUTPUT: usize = 255 * SHA256_LEN;

/// HKDF-Extract (RFC 5869 §2.2): `PRK = HMAC-SHA256(salt, IKM)`.
///
/// `salt` is the optional per-protocol salt; an empty slice is the RFC 5869
/// "no salt" case, which HMAC handles as an all-zero key.  The 32-byte PRK
/// is returned by value, and its argument order is part of the contract:
/// `hkdf_extract(salt, ikm)`, never the reverse.
pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; SHA256_LEN] {
    // A thin wrapper (03-RESEARCH.md Pattern 1): the upstream `Hkdf` value is
    // dropped; only the PRK bytes cross this API boundary (D-18).
    let (prk, _) = Hkdf::<Sha256>::extract(Some(salt), ikm);
    prk.into()
}

/// HKDF-Expand (RFC 5869 §2.3): fill `okm` from `prk` with `info` as the
/// context string.
///
/// `okm`'s length is the RFC's `L`; more than [`HKDF_MAX_OUTPUT`] bytes is
/// an error and leaves `okm` untouched.  A zero-length `okm` is `Ok`.
pub fn hkdf_expand(
    prk: &[u8; SHA256_LEN],
    info: &[u8],
    okm: &mut [u8],
) -> crate::Result<()> {
    // The RFC 5869 §2.3 length rule is checked here, before `okm` is touched:
    // an impossible request is an error, never a truncated or weaker output
    // (T-03-11).  There is no fallback derivation on any path.
    if okm.len() > HKDF_MAX_OUTPUT {
        return Err(crate::Error::OutputTooLong);
    }
    // `from_prk` refuses a PRK shorter than the hash length; the array
    // parameter makes that impossible, so this cannot fire.
    let hkdf = Hkdf::<Sha256>::from_prk(prk)
        .expect("a 32-byte PRK is exactly the SHA-256 HKDF length (RFC 5869 §2.2)");
    // `expand`'s only error is the same length rule just enforced.
    hkdf.expand(info, okm)
        .map_err(|_| crate::Error::OutputTooLong)
}
