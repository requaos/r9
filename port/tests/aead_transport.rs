//! The tracer pin: one sealed packet goes from a client end to a server end
//! on the host (WIRE-01, WIRE-03, WIRE-05; D-06, D-07, D-08).
//!
//! Keys are derived with `crypto::session::derive` from a fixed secret and a
//! fixed transcript, the packet is sealed by `port::chan::seal_packet` and
//! opened by `port::chan::open_packet`.  Every expected wire byte below is
//! written out by hand, independently of the codec under test.

use crypto::key::Key;
use crypto::session;

/// The fixed 32-byte session secret: bytes 0x00..=0x1f.
fn secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = i as u8;
    }
    Key::from_bytes(bytes)
}

/// The fixed 32-byte handshake transcript: bytes 0x20..=0x3f.
fn transcript() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = 0x20 + i as u8;
    }
    bytes
}

#[test]
fn aead_transport_tracer_one_sealed_packet() {
    let keys = session::derive(&secret(), &transcript(), 0);
    let body: &[u8] = b"9P tracer";
    let fid_handle = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

    let packet = port::chan::seal_packet(&keys.c2s, 0, 104, 0x1234, fid_handle, body)
        .expect("the client seals the first packet");

    assert_eq!(
        packet.len(),
        port::chan::HEADER_LEN + body.len() + port::chan::MAC_LEN,
        "a sealed packet is the 28-byte header, the body and the 32-byte MAC"
    );
    assert_eq!(
        u32::from_le_bytes(packet[0..4].try_into().unwrap()) as usize,
        packet.len(),
        "Size at offset 0 counts the whole packet"
    );
    assert_eq!(packet[4], 104, "Type at offset 4 is the message type");
    assert_eq!(
        u16::from_le_bytes(packet[5..7].try_into().unwrap()),
        0x1234,
        "Tag at offset 5 is little-endian"
    );
    assert_eq!(packet[7], 0, "Pad at offset 7 is zero");
    assert_eq!(&packet[8..16], &fid_handle[..], "FidHandle at offset 8");
    assert_eq!(&packet[16..24], &[0u8; 8][..], "SeqNonce at offset 16 is sequence 0");
    assert_eq!(
        &packet[24..28],
        &[0u8; 4][..],
        "ExtMask and ExtTLVs Length are zero"
    );
    assert_ne!(
        &packet[28..28 + body.len()],
        body,
        "the body on the wire is ciphertext, not the plaintext"
    );

    let (typ, tag, handle, opened) = port::chan::open_packet(&keys.c2s, 0, 8192, &packet)
        .expect("the server opens the sealed packet");

    assert_eq!(typ, 104, "the opened type is the sealed type");
    assert_eq!(tag, 0x1234, "the opened tag is the sealed tag");
    assert_eq!(handle, fid_handle, "the opened handle is the sealed handle");
    assert_eq!(opened.as_slice(), body, "the opened body is the plaintext");
}

#[test]
fn aead_transport_tracer_rejects_tampering() {
    let keys = session::derive(&secret(), &transcript(), 0);
    let packet = port::chan::seal_packet(&keys.c2s, 0, 104, 7, [0u8; 8], b"tamper me")
        .expect("the client seals a packet to tamper with");

    // Flipping one byte of the cleartext FidHandle field must fail the MAC
    // before the body is decrypted (D-07, T-04-04).
    let mut flipped_handle = packet.clone();
    flipped_handle[8] ^= 0x01;
    assert!(
        port::chan::open_packet(&keys.c2s, 0, 8192, &flipped_handle).is_err(),
        "a flipped FidHandle byte fails the packet MAC"
    );

    // Flipping the last MAC byte must fail verification.
    let mut flipped_mac = packet.clone();
    let last = flipped_mac.len() - 1;
    flipped_mac[last] ^= 0x01;
    assert!(
        port::chan::open_packet(&keys.c2s, 0, 8192, &flipped_mac).is_err(),
        "a flipped MAC byte fails verification"
    );

    // The other direction's keys are different keys.
    assert!(
        port::chan::open_packet(&keys.s2c, 0, 8192, &packet).is_err(),
        "the server-to-client keys do not open a client-to-server packet"
    );

    // An unexpected sequence number is refused (WIRE-04's strict order).
    assert!(
        port::chan::open_packet(&keys.c2s, 1, 8192, &packet).is_err(),
        "a packet whose sequence is not the expected next value is refused"
    );
}
