//! Host-side pins for `crypto::session` through the public API (WIRE-03,
//! WIRE-05; D-01, D-08, D-14, D-17).
//!
//! Everything here imports `crypto` only: the key bytes themselves are
//! unreadable outside the crate, so the raw-byte pins live in
//! `crypto/src/session.rs`'s in-crate `tests` module.  These tests pin the
//! observable behaviour the transport depends on: per-direction separation,
//! generation separation, the transcript order and the keyed-BLAKE3 shape of
//! the packet MAC, the capability tag and the re-key confirm.

use crypto::Error;
use crypto::key::Key;
use crypto::mac::blake3_hash;
use crypto::session::{Transcript, cap_tag, cap_verify, derive};

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

#[test]
fn session_is_zeroized_through_the_public_api() {
    let mut keys = derive(&secret(), &transcript(), 0);
    assert!(!keys.is_zeroized(), "freshly derived keys are not zeroed");
    assert!(!keys.c2s.is_zeroized(), "fresh c2s keys are not zeroed");
    assert!(!keys.s2c.is_zeroized(), "fresh s2c keys are not zeroed");

    keys.zeroize();
    assert!(keys.is_zeroized(), "zeroize scrubs both directions");
    assert!(keys.c2s.is_zeroized(), "zeroize scrubs c2s");
    assert!(keys.s2c.is_zeroized(), "zeroize scrubs s2c");
}

#[test]
fn session_packet_mac_split_updates_equal_one_shot() {
    let keys = derive(&secret(), &transcript(), 0);
    let header: &[u8] = b"header: 28 bytes of envelope";
    let tlv: &[u8] = b"tlv records";
    let ciphertext: &[u8] = b"the sealed body";

    let mut one = Vec::new();
    one.extend_from_slice(header);
    one.extend_from_slice(tlv);
    one.extend_from_slice(ciphertext);

    let mut one_shot = keys.c2s.mac();
    one_shot.update(&one);
    let one_shot = one_shot.finalize();

    let mut split = keys.c2s.mac();
    split.update(header);
    split.update(tlv);
    split.update(ciphertext);
    assert_eq!(split.finalize(), one_shot, "three updates equal one over the concatenation");

    let mut with_empty = keys.c2s.mac();
    with_empty.update(header);
    with_empty.update(b"");
    with_empty.update(tlv);
    with_empty.update(ciphertext);
    assert_eq!(with_empty.finalize(), one_shot, "an empty update changes nothing");
}

#[test]
fn session_packet_mac_verify_rejects_each_flipped_tag_byte() {
    let keys = derive(&secret(), &transcript(), 0);

    let mut mac = keys.c2s.mac();
    mac.update(MESSAGE);
    let tag = mac.finalize();

    let mut mac = keys.c2s.mac();
    mac.update(MESSAGE);
    assert_eq!(mac.verify(&tag), Ok(()), "the true tag verifies");

    for byte in 0..32 {
        let mut flipped = tag;
        flipped[byte] ^= 1;
        let mut mac = keys.c2s.mac();
        mac.update(MESSAGE);
        assert_eq!(
            mac.verify(&flipped),
            Err(Error::TagMismatch),
            "flipping tag byte {byte} must fail verification"
        );
    }
}

#[test]
fn session_cap_tag_is_truncated_keyed_blake3_of_label_and_body() {
    let domain_bytes: [u8; 32] = core::array::from_fn(|i| 0x40 + i as u8);
    let body: [u8; 16] = core::array::from_fn(|i| i as u8);

    let tag = cap_tag(&Key::from_bytes(domain_bytes), &body);

    // The expected side is a direct keyed-BLAKE3 call with the label typed
    // here as a literal (the DECISION P8 value), never a helper from
    // `crypto::session`.
    let mut message = Vec::new();
    message.extend_from_slice(b"r9 StatelessCap1");
    message.extend_from_slice(&body);
    let full = crypto::mac::blake3_keyed(&domain_bytes, &message);

    assert_eq!(tag, full[..8], "the capability tag is the first 8 bytes of keyed BLAKE3");

    let last_eight: [u8; 8] =
        full[24..32].try_into().expect("the last 8 bytes of a 32-byte tag");
    assert_eq!(
        cap_verify(&Key::from_bytes(domain_bytes), &body, &last_eight),
        Err(Error::TagMismatch),
        "the last 8 bytes are not the tag"
    );
}

#[test]
fn session_cap_tag_roundtrip_and_reject() {
    let domain_bytes: [u8; 32] = core::array::from_fn(|i| 0x40 + i as u8);
    let domain = Key::from_bytes(domain_bytes);
    let body: [u8; 16] = core::array::from_fn(|i| i as u8);

    let tag = cap_tag(&domain, &body);
    assert_eq!(cap_verify(&domain, &body, &tag), Ok(()), "the tag verifies");

    for byte in 0..16 {
        let mut flipped = body;
        flipped[byte] ^= 1;
        assert_eq!(
            cap_verify(&domain, &flipped, &tag),
            Err(Error::TagMismatch),
            "flipping body byte {byte} must fail verification"
        );
    }
    for byte in 0..8 {
        let mut flipped = tag;
        flipped[byte] ^= 1;
        assert_eq!(
            cap_verify(&domain, &body, &flipped),
            Err(Error::TagMismatch),
            "flipping tag byte {byte} must fail verification"
        );
    }

    let mut other_bytes = domain_bytes;
    other_bytes[0] ^= 1;
    assert_eq!(
        cap_verify(&Key::from_bytes(other_bytes), &body, &tag),
        Err(Error::TagMismatch),
        "a different domain key must fail verification"
    );
}

#[test]
fn session_rekey_confirm_binds_key_and_generation() {
    let keys = derive(&secret(), &transcript(), 0);
    let generation = 7;

    let confirm = keys.c2s.rekey_confirm(generation);
    assert_eq!(
        keys.c2s.verify_rekey_confirm(generation, &confirm),
        Ok(()),
        "the matching confirm verifies"
    );
    assert_eq!(
        keys.s2c.verify_rekey_confirm(generation, &confirm),
        Err(Error::TagMismatch),
        "the other direction's keys do not verify it"
    );
    assert_eq!(
        keys.c2s.verify_rekey_confirm(generation + 1, &confirm),
        Err(Error::TagMismatch),
        "another generation does not verify it"
    );

    let mut flipped = confirm;
    flipped[0] ^= 1;
    assert_eq!(
        keys.c2s.verify_rekey_confirm(generation, &flipped),
        Err(Error::TagMismatch),
        "one flipped byte must fail verification"
    );
}
