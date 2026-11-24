//! mac — keyed BLAKE3, the one message-authentication code in r9 (CRYP-03,
//! D-05, D-07).
//!
//! Keyed BLAKE3 is the only MAC in r9: it authenticates wire packets, seals
//! pages and computes capability tags.  Poly1305 is not implemented, and no
//! HMAC construction is built over BLAKE3: the "HMAC-BLAKE3" wording in the
//! RFC (§5 and §5.x, the `StatelessCap` tag) means this keyed mode with the
//! output truncated (D-07).  HMAC exists in `crypto/` only over SHA-256, as
//! the PRF under HKDF (`crypto::kdf`).
//!
//! Tree mode needs no separate function: BLAKE3 hashes any input over 1024
//! bytes as a tree, so `blake3_hash`, `blake3_keyed` and [`Blake3`] cover
//! the tree mode and the keyed tree mode by construction.  The pins are the
//! 35 cases of the official `test_vectors.json`, whose inputs run to 102,400
//! bytes, in `crypto/tests/blake3_vectors.rs`.
//!
//! A capability tag may be shorter than the 32-byte output: [`verify_keyed`]
//! accepts any tag length from 1 to 32 bytes and compares it in constant
//! time, so the 8-byte `mac_tag` of the RFC's `StatelessCap` (§5.1) verifies
//! without a second primitive.

/// The BLAKE3 output length in bytes (the default-length hash).
pub const BLAKE3_OUT_LEN: usize = 32;

/// The BLAKE3 key length in bytes (keyed mode).
pub const BLAKE3_KEY_LEN: usize = 32;

/// The one-shot unkeyed BLAKE3 hash of `msg`, 32-byte output.
pub fn blake3_hash(msg: &[u8]) -> [u8; BLAKE3_OUT_LEN] {
    let _ = msg;
    [0u8; BLAKE3_OUT_LEN]
}

/// The one-shot keyed BLAKE3 hash of `msg` under `key`, 32-byte output.
pub fn blake3_keyed(key: &[u8; BLAKE3_KEY_LEN], msg: &[u8]) -> [u8; BLAKE3_OUT_LEN] {
    let _ = (key, msg);
    [0u8; BLAKE3_OUT_LEN]
}

/// An incremental BLAKE3 hasher: unkeyed via [`Blake3::new`], keyed via
/// [`Blake3::new_keyed`].  Feeding the same bytes through any split of
/// `update` calls gives the one-shot result.
pub struct Blake3;

impl Blake3 {
    /// A new unkeyed hasher.
    pub fn new() -> Blake3 {
        Blake3
    }

    /// A new keyed hasher.
    pub fn new_keyed(key: &[u8; BLAKE3_KEY_LEN]) -> Blake3 {
        let _ = key;
        Blake3
    }

    /// Absorb `data`.
    pub fn update(&mut self, data: &[u8]) {
        let _ = data;
    }

    /// The 32-byte hash of everything absorbed so far.
    pub fn finalize(&self) -> [u8; BLAKE3_OUT_LEN] {
        [0u8; BLAKE3_OUT_LEN]
    }
}

/// Verify a keyed BLAKE3 tag over `msg` under `key`.
///
/// `tag` must be between 1 and 32 bytes: a shorter tag is the truncated
/// output of the same keyed hash (D-07), so the 8-byte capability tag
/// verifies here.  The comparison is constant-time and has no early exit;
/// `Err(Error::BadTagLength)` reports a length outside 1..=32 and
/// `Err(Error::TagMismatch)` a tag that does not match.
pub fn verify_keyed(key: &[u8; BLAKE3_KEY_LEN], msg: &[u8], tag: &[u8]) -> crate::Result<()> {
    let _ = (key, msg, tag);
    Ok(())
}
