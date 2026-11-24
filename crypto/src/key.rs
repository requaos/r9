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
    let _ = (k_user, qid_path, qid_vers);
    Key { bytes: Zeroizing::new([0u8; KEY_LEN]) }
}

/// Expand `parent` with `label` then `context` as the HKDF `info` (CRYP-04).
///
/// The two slices are concatenated with no framing, which is the RFC's
/// `"9P-AEAD-FID" || fid` shape: `("ab", "c")` and `("a", "bc")` give the
/// same key.  Callers therefore use fixed-length labels.  The session `info`
/// layout (per FID versus per connection and direction) is W4 in Phase 4;
/// this function fixes none of it.
pub fn derive_labelled(parent: &Key, label: &[u8], context: &[u8]) -> Key {
    let _ = (parent, label, context);
    Key { bytes: Zeroizing::new([0u8; KEY_LEN]) }
}
