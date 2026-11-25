//! session — per-connection, per-direction session keys and the packet MAC
//! (WIRE-03, WIRE-05; D-08, D-15, D-17, D-22).
//!
//! # What this module owns
//!
//! Everything that touches raw key bytes for the sealed transport lives here,
//! because [`crate::key::Key`] has no public accessor and `port` must never
//! hold key bytes (T-04-03).  The transport in `port::chan` holds a
//! [`DirectionKeys`] value and calls methods on it; it never sees `k_enc`,
//! `k_mac` or the nonce prefix.
//!
//! [`derive`] runs the P2 derivation: one HKDF-Extract with the handshake
//! transcript as salt and the exchange's 32-byte secret as input keying
//! material, then one HKDF-Expand per direction and purpose.  Each `info` is
//! the fixed 18-byte form
//! `"9P2000.aead" || 0x00 || direction || purpose || generation (LE, 4)`.
//! The generation is in `info` so that a seam returning the same secret after
//! a re-key still cannot reproduce a `(key, nonce)` pair once the sequence
//! counter returns to 0 (Pitfall 5).
//!
//! [`DirectionKeys::keystream`] builds the 12-byte ChaCha20 nonce from the
//! derived 4-byte prefix and the 8-byte wire sequence number, little-endian
//! (D-08).  The prefix is derived, never sent on the wire.
//!
//! [`PacketMac`] wraps a keyed BLAKE3 hasher created inside this crate.  The
//! packet MAC covers every byte before the MAC (D-07), so the transport feeds
//! it one contiguous slice; [`PacketMac::verify`] compares in constant time
//! with an accumulator fold and no early exit (T-04-05).
//!
//! # What is zeroized, and what is not (P6)
//!
//! `Key` holds `Zeroizing<[u8; 32]>`, and the nonce prefix is held in
//! `Zeroizing<[u8; 4]>`, so the long-lived session material is scrubbed on
//! drop.  It is **not** a complete scrubbing guarantee: each per-packet
//! `blake3::Hasher` and each `ChaCha20` cipher state holds a copy of its key
//! on the stack and is dropped without scrubbing, because the pinned
//! `blake3`'s `zeroize` feature is deliberately off (P6).  The limit is
//! stated here rather than overclaimed.
//!
//! # What this module does not decide
//!
//! The exchange that produces the secret is not decided here (D-16): the
//! transport takes a 32-byte secret behind a trait and tests inject it
//! directly.  The real authenticated exchange and the Noise versus DP9IK
//! choice are later phases' work.

use zeroize::Zeroizing;

use crate::key::Key;

/// The dialect label mixed into every derivation `info` (P2).
pub const SESSION_LABEL: &[u8; 11] = b"9P2000.aead";

/// The derivation purpose byte for the cipher key (P2).
const PURPOSE_CIPHER: u8 = 0x01;
/// The derivation purpose byte for the MAC key (P2).
const PURPOSE_MAC: u8 = 0x02;
/// The derivation purpose byte for the nonce prefix (P2).
const PURPOSE_PREFIX: u8 = 0x03;

/// The fixed derivation `info` length: label 11, separator 1, direction 1,
/// purpose 1, generation 4.
const INFO_LEN: usize = 18;
/// The derived nonce prefix length (D-08).
const PREFIX_LEN: usize = 4;

/// One direction of a connection.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Client to server.
    ClientToServer,
    /// Server to client.
    ServerToClient,
}

impl Direction {
    /// The direction byte in the derivation `info` (P2).
    fn byte(self) -> u8 {
        match self {
            Direction::ClientToServer => 0x01,
            Direction::ServerToClient => 0x02,
        }
    }
}

/// The keys of one direction: a cipher key, a MAC key and the nonce prefix.
///
/// The fields are private, there is no accessor and no `Debug` impl: a holder
/// can seal, verify and apply the keystream, not read key material
/// (T-04-03, WIRE-05).
#[derive(Clone)]
pub struct DirectionKeys {
    k_enc: Key,
    k_mac: Key,
    prefix: Zeroizing<[u8; PREFIX_LEN]>,
}

/// Both directions of one connection at one generation (D-15).
#[derive(Clone)]
pub struct SessionKeys {
    /// Client to server.
    pub c2s: DirectionKeys,
    /// Server to client.
    pub s2c: DirectionKeys,
}

/// Derive the two directions' keys from `secret`, the handshake `transcript`
/// and the `generation` (P2, D-17).
///
/// The extract step uses the transcript as the HKDF salt and the secret as
/// the input keying material.  Each of the six 18-byte `info` strings then
/// differs in its direction and purpose byte, so no two outputs can collide.
pub fn derive(secret: &Key, transcript: &[u8; 32], generation: u32) -> SessionKeys {
    let prk = crate::kdf::hkdf_extract(transcript, secret.bytes());
    SessionKeys {
        c2s: derive_direction(&prk, Direction::ClientToServer, generation),
        s2c: derive_direction(&prk, Direction::ServerToClient, generation),
    }
}

/// Build the 18-byte derivation `info` (P2) and expand `out` from `prk`.
fn expand(prk: &[u8; crate::kdf::SHA256_LEN], direction: Direction, purpose: u8, generation: u32, out: &mut [u8]) {
    let mut info = [0u8; INFO_LEN];
    info[..SESSION_LABEL.len()].copy_from_slice(SESSION_LABEL);
    // info[11] is the 0x00 separator, already zero.
    info[12] = direction.byte();
    info[13] = purpose;
    info[14..].copy_from_slice(&generation.to_le_bytes());
    crate::kdf::hkdf_expand(prk, &info, out)
        .expect("a 32-byte HKDF-SHA256 expand is within the 255 x 32 RFC 5869 limit");
}

/// Derive one direction's cipher key, MAC key and nonce prefix.
fn derive_direction(
    prk: &[u8; crate::kdf::SHA256_LEN],
    direction: Direction,
    generation: u32,
) -> DirectionKeys {
    let mut k_enc = [0u8; 32];
    let mut k_mac = [0u8; 32];
    let mut prefix = [0u8; PREFIX_LEN];
    expand(prk, direction, PURPOSE_CIPHER, generation, &mut k_enc[..]);
    expand(prk, direction, PURPOSE_MAC, generation, &mut k_mac[..]);
    expand(prk, direction, PURPOSE_PREFIX, generation, &mut prefix[..]);
    DirectionKeys {
        k_enc: Key::from_bytes(k_enc),
        k_mac: Key::from_bytes(k_mac),
        prefix: Zeroizing::new(prefix),
    }
}

/// An incremental keyed BLAKE3 packet MAC (D-07, T-04-05).
pub struct PacketMac {
    h: crate::mac::Blake3,
}

impl PacketMac {
    /// Absorb one part of the packet.
    pub fn update(&mut self, part: &[u8]) {
        self.h.update(part);
    }

    /// The 32-byte tag over everything absorbed.
    pub fn finalize(self) -> [u8; 32] {
        self.h.finalize()
    }

    /// Verify the tag in constant time (T-04-05).
    pub fn verify(self, tag: &[u8; 32]) -> crate::Result<()> {
        let computed = self.h.finalize();
        // Accumulator fold with no early exit: `acc` is zero exactly when
        // every byte matched, and the loop's running time does not depend on
        // where two tags first differ.
        let mut acc = 0u8;
        for (computed_byte, tag_byte) in computed.iter().zip(tag.iter()) {
            acc |= computed_byte ^ tag_byte;
        }
        if acc == 0 { Ok(()) } else { Err(crate::Error::TagMismatch) }
    }
}

impl DirectionKeys {
    /// A packet MAC under this direction's MAC key.
    pub fn mac(&self) -> PacketMac {
        PacketMac { h: crate::mac::Blake3::new_keyed(self.k_mac.bytes()) }
    }

    /// XOR this direction's ChaCha20 keystream for sequence `seq`, starting
    /// at block `counter`, into `buf` (D-08).
    pub fn keystream(&self, seq: u64, counter: u32, buf: &mut [u8]) -> crate::Result<()> {
        let mut nonce = [0u8; 12];
        nonce[..PREFIX_LEN].copy_from_slice(&self.prefix[..]);
        nonce[PREFIX_LEN..].copy_from_slice(&seq.to_le_bytes());
        crate::page::chacha20_xor(self.k_enc.bytes(), &nonce, counter, buf)
    }
}
