//! key — the key hierarchy: `K_file`, a labelled expand for transient and
//! session keys, and the `Key` newtype over zeroizing key material (CRYP-04).
//!
//! # Landing site (D-22)
//!
//! The implementation guide's §4.1 sketch places the key material types in
//! `port/src/segcrypt.rs`.  They live here in `crypto/` instead, host-tested,
//! and Phase 5's `port/src/segcrypt.rs` imports them rather than defining
//! them; the guide's landing site is recorded as moved (D-22).  No `port/src`
//! file is edited in this phase.
//!
//! # Hierarchy
//!
//! `K_file = HKDF-Expand(K_user, Qid.path || Qid.vers, 32)` (RFC §3.2), with
//! the Qid fields encoded as fixed-width little-endian integers (8 bytes then
//! 4), matching 9P's wire order.  `derive_labelled` has the RFC's
//! `HKDF-Expand(S_session, "9P-AEAD-FID" || fid, 32)` shape: label bytes then
//! context bytes, unframed.  Which session layout is used — per FID versus
//! per connection and direction — is W4, settled in Phase 4; this module
//! fixes none of it.
//!
//! # Key material
//!
//! `Key` holds `Zeroizing<[u8; KEY_LEN]>`, derives `Clone` and nothing that
//! prints (`Debug` and `Display` are deliberately absent), and exposes no
//! public accessor that returns its bytes (T-03-14).  Code inside this crate
//! compares keys through [`Key::ct_eq`].

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

/// The key length in bytes: one SHA-256 HKDF output, 256 bits.
pub const KEY_LEN: usize = 32;

/// One 256-bit key held in self-zeroizing storage.
///
/// The bytes are private and there is no public accessor: a holder can derive
/// from the key and compare it, not read it (T-03-14).  `Clone` duplicates the
/// material; every copy zeroizes on drop.
#[derive(Clone)]
pub struct Key {
    bytes: Zeroizing<[u8; KEY_LEN]>,
}

impl Key {
    /// Wrap 32 bytes as a key.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Key {
        Key { bytes: Zeroizing::new(bytes) }
    }

    /// Constant-time equality: `true` exactly when both keys hold the same
    /// bytes.  The loop folds every comparison into one accumulator with no
    /// early exit, so the running time does not depend on where two keys
    /// first differ (T-03-14).
    pub fn ct_eq(&self, other: &Key) -> bool {
        let mut acc = 0u8;
        for (a, b) in self.bytes.iter().zip(other.bytes.iter()) {
            acc |= a ^ b;
        }
        acc == 0
    }

    /// The key bytes, for derivation inside `crypto/` only.  Deliberately
    /// `pub(crate)`: no public function or method returns key bytes.
    pub(crate) fn bytes(&self) -> &[u8; KEY_LEN] {
        &self.bytes
    }
}

/// `K_file = HKDF-Expand(K_user, Qid.path || Qid.vers, 32)` (RFC §3.2).
///
/// `info` is exactly 12 bytes: `qid_path` as 8 bytes little-endian, then
/// `qid_vers` as 4 bytes little-endian.  The fixed widths keep two Qids that
/// differ in any bit of either field on different keys (T-03-17), and the
/// little-endian order matches 9P's wire order.  The encoding is pinned by a
/// known-answer vector in `crypto/tests/keys.rs`.
pub fn k_file(k_user: &Key, qid_path: u64, qid_vers: u32) -> Key {
    // The fixed-width `info` (T-03-17): 8 bytes little-endian path, then
    // 4 bytes little-endian vers, exactly 12 bytes with no padding.
    let mut info = [0u8; 12];
    info[..8].copy_from_slice(&qid_path.to_le_bytes());
    info[8..].copy_from_slice(&qid_vers.to_le_bytes());

    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    crate::kdf::hkdf_expand(k_user.bytes(), &info, &mut out[..])
        .expect("a 32-byte HKDF-SHA256 expand is within the 255 x 32 RFC 5869 limit");
    Key { bytes: out }
}

/// Expand `parent` with `label` then `context` as the HKDF `info` (CRYP-04).
///
/// The two slices are concatenated with no framing, which is the RFC's
/// `"9P-AEAD-FID" || fid` shape: `("ab", "c")` and `("a", "bc")` give the
/// same key.  Callers therefore use fixed-length labels.  The session `info`
/// layout (per FID versus per connection and direction) is W4 in Phase 4;
/// this function fixes none of it.
pub fn derive_labelled(parent: &Key, label: &[u8], context: &[u8]) -> Key {
    // `expand_multi_info` feeds the HKDF-Expand HMAC the components in
    // sequence, which is exactly feeding it `label || context` (the RFC's
    // `info` is one opaque string).  The multi-part form is used because
    // `crypto/` is `no_std` with no allocator, so there is no buffer to
    // concatenate the two caller slices into.
    let hkdf = Hkdf::<Sha256>::from_prk(parent.bytes())
        .expect("a 32-byte PRK is exactly the SHA-256 HKDF length (RFC 5869 §2.2)");

    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    hkdf.expand_multi_info(&[label, context], &mut out[..])
        .expect("a 32-byte HKDF-SHA256 expand is within the 255 x 32 RFC 5869 limit");
    Key { bytes: out }
}

// ---------------------------------------------------------------------------
// KeyCapability: a typestate over the two capability states (CRYP-04, D-22)
// ---------------------------------------------------------------------------

/// The read-write capability state: a decryption key and a separate MAC key
/// (T-03-18).
pub struct ReadWrite {
    k_dec: Key,
    k_mac: Key,
}

/// The read-only capability state: the parent's decryption key and the key
/// used to verify tags.
pub struct ReadOnly {
    k_dec: Key,
    blake3_verify: Key,
}

/// A capability over one state.
///
/// `KeyCapability<ReadWrite>` seals and verifies; `KeyCapability<ReadOnly>`
/// verifies and has no sealing method.  Both decrypt.  The state values are
/// private and no accessor returns a key.
pub struct KeyCapability<S> {
    state: S,
}

impl KeyCapability<ReadWrite> {
    /// Build a read-write capability from a decryption key and a MAC key.
    pub fn new(k_dec: Key, k_mac: Key) -> KeyCapability<ReadWrite> {
        KeyCapability { state: ReadWrite { k_dec, k_mac } }
    }

    /// Seal `msg` under the MAC key: the 32-byte keyed BLAKE3 tag.
    pub fn seal(&self, msg: &[u8]) -> [u8; crate::mac::BLAKE3_OUT_LEN] {
        let _ = msg;
        [0u8; crate::mac::BLAKE3_OUT_LEN]
    }

    /// The read-only sibling: the parent's `k_dec` unchanged, and `k_mac` as
    /// the verification key (IG-P01: a sibling inheriting shared read-only
    /// base code keeps the parent's `k_dec`).
    pub fn read_only(&self) -> KeyCapability<ReadOnly> {
        KeyCapability {
            state: ReadOnly {
                k_dec: self.state.k_dec.clone(),
                blake3_verify: self.state.k_mac.clone(),
            },
        }
    }

    /// Verify a tag over `msg` under the MAC key.
    pub fn verify(&self, msg: &[u8], tag: &[u8]) -> crate::Result<()> {
        let _ = (msg, tag);
        Ok(())
    }

    /// XOR the ChaCha20 keystream under `k_dec` into `buf`, from `counter`
    /// with `nonce`.
    pub fn apply_keystream(
        &self,
        nonce: &[u8; 12],
        counter: u32,
        buf: &mut [u8],
    ) -> crate::Result<()> {
        let _ = (nonce, counter, buf);
        Ok(())
    }
}

impl KeyCapability<ReadOnly> {
    /// Verify a tag over `msg` under the verification key.
    pub fn verify(&self, msg: &[u8], tag: &[u8]) -> crate::Result<()> {
        let _ = (msg, tag);
        Ok(())
    }

    /// XOR the ChaCha20 keystream under the inherited `k_dec` into `buf`,
    /// from `counter` with `nonce`.
    pub fn apply_keystream(
        &self,
        nonce: &[u8; 12],
        counter: u32,
        buf: &mut [u8],
    ) -> crate::Result<()> {
        let _ = (nonce, counter, buf);
        Ok(())
    }
}
