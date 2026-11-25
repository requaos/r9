//! Host-side pins for `crypto::session` through the public API (WIRE-03,
//! WIRE-05; D-01, D-08, D-14, D-17).
//!
//! Everything here imports `crypto` only: the key bytes themselves are
//! unreadable outside the crate, so the raw-byte pins live in
//! `crypto/src/session.rs`'s in-crate `tests` module.  These tests pin the
//! observable behaviour the transport depends on: per-direction separation,
//! generation separation, the transcript order and the keyed-BLAKE3 shape of
//! the packet MAC, the capability tag and the re-key confirm.

use crypto::key::Key;
use crypto::mac::blake3_hash;
use crypto::session::{Transcript, derive};

/// The fixed 32-byte session secret: bytes 0x00..=0x1f.
fn secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = i as u8;
    }
    Key::from_bytes(bytes)
}

/// The fixed 32-byte handshake transcript: bytes 0x20..=0x3f.
fn transcript() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = 0x20 + i as u8;
    }
    bytes
}

/// A fixed message the MAC and keystream comparisons run over.
const MESSAGE: &[u8] = b"9P2000.aead session pin";

/// One 64-byte keystream block under `keys`, sequence 0 from block 0.
fn keystream_block(keys: &crypto::session::DirectionKeys) -> [u8; 64] {
    let mut buf = [0u8; 64];
    keys.keystream(0, 0, &mut buf).expect("one block is within the counter limit");
    buf
}

/// The packet MAC of `MESSAGE` under `keys`.
fn mac_of_message(keys: &crypto::session::DirectionKeys) -> [u8; 32] {
    let mut mac = keys.mac();
    mac.update(MESSAGE);
    mac.finalize()
}

#[test]
fn session_directions_differ() {
    let keys = derive(&secret(), &transcript(), 0);

    assert_ne!(
        mac_of_message(&keys.c2s),
        mac_of_message(&keys.s2c),
        "the two directions' MAC keys differ"
    );
    assert_ne!(
        keystream_block(&keys.c2s),
        keystream_block(&keys.s2c),
        "the two directions' cipher keys and prefixes differ"
    );
}

#[test]
fn session_generation_changes_every_key() {
    let generation_0 = derive(&secret(), &transcript(), 0);
    let generation_1 = derive(&secret(), &transcript(), 1);

    for (label, old, new) in [
        ("c2s", &generation_0.c2s, &generation_1.c2s),
        ("s2c", &generation_0.s2c, &generation_1.s2c),
    ] {
        assert_ne!(
            mac_of_message(old),
            mac_of_message(new),
            "generation 1 must move the {label} MAC key"
        );
        assert_ne!(
            keystream_block(old),
            keystream_block(new),
            "generation 1 must move the {label} cipher key and prefix"
        );
    }
}

#[test]
fn session_transcript_bit_changes_keys() {
    let base = derive(&secret(), &transcript(), 0);
    let base_mac = mac_of_message(&base.c2s);

    for bit in 0..256 {
        let mut flipped = transcript();
        flipped[bit / 8] ^= 1 << (bit % 8);
        let keys = derive(&secret(), &flipped, 0);
        assert_ne!(
            mac_of_message(&keys.c2s),
            base_mac,
            "flipping transcript bit {bit} must change the c2s MAC key"
        );
    }
}

#[test]
fn session_transcript_is_blake3_of_messages_in_order() {
    let a: &[u8] = b"Tversion size=20 msize=8192 version=9P2000.aead";
    let b: &[u8] = b"Rversion size=20 msize=8192 version=9P2000.aead";

    let mut absorbed = Transcript::new();
    absorbed.absorb(a);
    absorbed.absorb(b);

    let mut ab = Vec::new();
    ab.extend_from_slice(a);
    ab.extend_from_slice(b);
    assert_eq!(
        absorbed.hash(),
        blake3_hash(&ab),
        "absorbing A then B is BLAKE3 of A followed by B"
    );

    let mut reversed = Transcript::new();
    reversed.absorb(b);
    reversed.absorb(a);
    assert_ne!(
        reversed.hash(),
        absorbed.hash(),
        "the absorption order is part of the contract"
    );

    assert_eq!(
        Transcript::new().hash(),
        blake3_hash(b""),
        "an empty transcript is BLAKE3 of the empty string"
    );
}

#[test]
fn session_transcript_chains_across_rekeys() {
    let previous = blake3_hash(b"generation 0 handshake");
    let message: &[u8] = b"Rrekey marker";

    let mut chained = Transcript::chain(&previous);
    chained.absorb(message);

    let mut expected = Vec::new();
    expected.extend_from_slice(&previous);
    expected.extend_from_slice(message);
    assert_eq!(
        chained.hash(),
        blake3_hash(&expected),
        "chain(prev) then absorbing M is BLAKE3 of prev followed by M"
    );
}
