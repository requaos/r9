//! The transport pins: the plain-framed handshake with its exchange seam, the
//! independently computed wire bytes, and failing closed on any authenticated
//! byte (WIRE-03, WIRE-05, WIRE-11; D-07, D-15 to D-17).
//!
//! The two tracer pins at the bottom are unchanged from plan 04-02.  The wire
//! pins never call `Session::open`, `open_packet`, `msg::decode_body` or
//! `msg::data_layout` to obtain an expected value: they hold their own
//! `DirectionKeys`, derived in the test from the same secret and the same
//! absorbed messages, and work on the sealed bytes with `keystream` and `mac()`
//! only.  The page size 4096, the 64 blocks per page and the offsets are
//! literals here.

mod common;

use common::aead_link::{
    AeadLink, ServerEnd, TestExchange, config_with_msize, connect, connect_with, rauth_frame,
    tauth_frame, unauthenticated_session,
};
use crypto::key::Key;
use crypto::session::{self, Transcript};
use port::chan::msg::{self, Msg};
use port::chan::tlv::Exts;
use port::chan::transport::{Handshake, Role};
use port::chan::wire;
use port::chan::PAGE_LEN;
use port::dat::Qid;

/// The fixed 32-byte session secret: bytes 0x00..=0x1f.
fn secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = i as u8;
    }
    Key::from_bytes(bytes)
}

/// The server's fixed secret: bytes 0x40..=0x5f.
fn server_secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = 0x40 + i as u8;
    }
    Key::from_bytes(bytes)
}

/// A third fixed secret, for the cross-connection pin: bytes 0x80..=0x9f.
fn other_secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = 0x80 + i as u8;
    }
    Key::from_bytes(bytes)
}

/// The fixed 32-byte handshake transcript: bytes 0x20..=0x3f (the tracer's).
fn transcript() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = 0x20 + i as u8;
    }
    bytes
}

/// Encode one message body with the crate's own encoder.
fn body_of(message: &Msg) -> Vec<u8> {
    let mut out = Vec::new();
    msg::encode_body(message, &mut out).expect("the test's body encodes");
    out
}

/// A `Tattach` body with a distinctive fid.
fn tattach_body() -> Vec<u8> {
    body_of(&Msg::Tattach {
        fid: 5,
        afid: 0,
        uname: b"none".to_vec(),
        aname: Vec::new(),
    })
}

#[test]
fn aead_transport_handshake_derives_matching_keys() {
    let link = AeadLink::new();
    let (mut client, mut server) = connect(link, secret(), server_secret());

    let handle = [0x11u8; 8];
    client
        .send(wire::TATTACH, 10, handle, &Exts::default(), &tattach_body())
        .expect("the client seals its first Tattach");
    let got = server.recv().expect("the server opens the client's sealed Tattach");
    assert_eq!(
        got,
        Msg::Tattach {
            fid: 5,
            afid: 0,
            uname: b"none".to_vec(),
            aname: Vec::new(),
        },
        "the sealed Tattach body decodes at the server"
    );

    let rattach = Msg::Rattach {
        qid: Qid {
            path: 5,
            vers: 1,
            typ: 0,
        },
    };
    server
        .send(wire::RATTACH, 10, handle, &Exts::default(), &body_of(&rattach))
        .expect("the server seals its Rattach");
    assert_eq!(
        client.recv().expect("the client opens the sealed Rattach"),
        rattach,
        "the sealed Rattach body decodes at the client"
    );
}

#[test]
fn aead_transport_handshake_with_empty_exchange() {
    let link = AeadLink::new();
    let (mut client, mut server) = connect_with(
        link.clone(),
        TestExchange::new(vec![secret()]),
        TestExchange::new(vec![server_secret()]),
    );

    {
        let plain = &link.borrow().plain_wire;
        assert!(
            plain.iter().any(|frame| frame.get(4) == Some(&wire::TAUTH)),
            "Tauth is on the wire even when the exchange has no bytes"
        );
        assert!(
            plain.iter().any(|frame| frame.get(4) == Some(&wire::RAUTH)),
            "Rauth is on the wire even when the exchange has no bytes"
        );
        assert!(
            plain.iter().any(|frame| frame.get(4) == Some(&wire::TWRITE)),
            "the exchange still rides a Twrite on the auth fid"
        );
        assert!(
            plain.iter().any(|frame| frame.get(4) == Some(&wire::TREAD)),
            "the exchange still rides a Tread on the auth fid"
        );
    }

    client
        .send(wire::TATTACH, 11, [0x22u8; 8], &Exts::default(), &tattach_body())
        .expect("the client seals its first Tattach");
    assert!(
        server.recv().is_ok(),
        "the sealed Tattach opens after a zero-byte exchange"
    );
}

#[test]
fn aead_transport_handshake_refuses_attach_before_auth() {
    // A session whose exchange never finished refuses to seal.
    let mut early = unauthenticated_session(Role::Client, secret());
    assert!(
        early
            .seal(wire::TATTACH, 12, [0u8; 8], &Exts::default(), &tattach_body(), 0)
            .is_err(),
        "a session that never absorbed Tauth/Rauth refuses to seal a Tattach"
    );

    // A sealed packet arriving at a server still in the handshake is an error,
    // and nothing is decoded.
    let link = AeadLink::new();
    let (mut client, _authenticated_server) = connect(link.clone(), secret(), server_secret());
    let mut server = ServerEnd::new(unauthenticated_session(Role::Server, server_secret()), link.clone());
    client
        .send(wire::TATTACH, 12, [0u8; 8], &Exts::default(), &tattach_body())
        .expect("the authenticated client seals a Tattach");
    assert!(
        server.recv().is_err(),
        "a server still in the handshake refuses a sealed packet"
    );
    assert_eq!(
        link.borrow().decoded(),
        0,
        "nothing is decoded on a server still in the handshake"
    );
}

#[test]
fn aead_transport_handshake_altered_rversion_fails_first_packet() {
    for variant in ["msize", "version"] {
        let link = AeadLink::new();
        {
            let mut guard = link.borrow_mut();
            match variant {
                "msize" => guard.alter_rversion_msize = Some(4096),
                _ => guard.alter_rversion_version = Some(b"9P2000".to_vec()),
            }
        }
        let (mut client, mut server) = connect(link, secret(), server_secret());
        client
            .send(wire::TATTACH, 13, [0u8; 8], &Exts::default(), &tattach_body())
            .expect("the client seals its first Tattach");
        assert!(
            server.recv().is_err(),
            "an altered Rversion ({variant}) makes the first sealed packet's MAC fail"
        );
    }
}

#[test]
fn aead_transport_handshake_keys_are_per_connection() {
    let a = AeadLink::new();
    let (mut client_a, _server_a) = connect(a.clone(), secret(), server_secret());
    let b = AeadLink::new();
    let (_client_b, mut server_b) = connect(b.clone(), other_secret(), other_secret());

    client_a
        .send(wire::TATTACH, 14, [0u8; 8], &Exts::default(), &tattach_body())
        .expect("the first client seals its first Tattach");
    let packet: Vec<u8> = a.borrow().c2s.iter().copied().collect();
    b.borrow_mut().c2s.extend(packet);

    assert!(
        server_b.recv().is_err(),
        "two connections with different secrets cannot open each other's first packet"
    );
}

#[test]
fn aead_transport_wire_data_pages_decrypt_alone_at_their_block_counter() {
    // A client session built directly from the two fixed auth messages.
    let tauth = tauth_frame();
    let rauth = rauth_frame();
    let mut handshake = Handshake::new(Role::Client, config_with_msize(65536));
    handshake.absorb_plain(&tauth);
    handshake.absorb_plain(&rauth);
    let mut client = handshake.finish(secret());

    // The test's own keys, derived without the transport.
    let mut own = Transcript::new();
    own.absorb(&tauth);
    own.absorb(&rauth);
    let keys = session::derive(&secret(), &own.hash(), 0);

    let fid: u32 = 0x1122_3344;
    let offset: u64 = 0x0102_0304_0506_0708;
    let count: u32 = 2 * PAGE_LEN as u32;
    let page0: Vec<u8> = (0..PAGE_LEN).map(|i| (i % 251) as u8).collect();
    let page1: Vec<u8> = (0..PAGE_LEN).map(|i| (255 - (i % 251)) as u8).collect();
    assert_ne!(page0, page1, "the two pages are distinct");

    let mut body = Vec::new();
    body.extend_from_slice(&fid.to_le_bytes());
    body.extend_from_slice(&offset.to_le_bytes());
    body.extend_from_slice(&count.to_le_bytes());
    body.extend_from_slice(&page0);
    body.extend_from_slice(&page1);

    let packet = client
        .seal(wire::TWRITE, 0x77, [0u8; 8], &Exts::default(), &body, 0)
        .expect("the client seals a two-page Twrite at sequence 0");
    let n = packet.len() - port::chan::MAC_LEN;

    // Page 1 alone, at block counter 1 + 64.
    let mut got = packet[n - PAGE_LEN..n].to_vec();
    keys.c2s
        .keystream(0, 1 + 64, &mut got)
        .expect("page 1's counter is in range");
    assert_eq!(got, page1, "wire page 1 XOR keystream(0, 1 + 64) is page 1");

    // Page 0 alone, at block counter 1.
    let mut got = packet[n - 2 * PAGE_LEN..n - PAGE_LEN].to_vec();
    keys.c2s.keystream(0, 1, &mut got).expect("page 0's counter is in range");
    assert_eq!(got, page0, "wire page 0 XOR keystream(0, 1) is page 0");

    // The 16 parameter bytes, at block counter 0, written by hand here.
    let mut got = packet[port::chan::HEADER_LEN..n - 2 * PAGE_LEN].to_vec();
    keys.c2s.keystream(0, 0, &mut got).expect("the parameter block fits one block");
    let mut want = Vec::new();
    want.extend_from_slice(&fid.to_le_bytes());
    want.extend_from_slice(&offset.to_le_bytes());
    want.extend_from_slice(&count.to_le_bytes());
    assert_eq!(got, want, "the 16 parameter bytes are fid, offset and true count");

    // The wrong counters do not give page 1.
    let mut wrong = packet[n - PAGE_LEN..n].to_vec();
    keys.c2s.keystream(0, 64, &mut wrong).expect("in range");
    assert_ne!(wrong, page1, "block counter 64 is not page 1's counter");
    let mut wrong = packet[n - PAGE_LEN..n].to_vec();
    keys.c2s.keystream(0, 2, &mut wrong).expect("in range");
    assert_ne!(wrong, page1, "block counter 2 is not page 1's counter");

    // The next seal is sequence 1 and its page 1 opens at 1 + 64 under seq 1.
    let second = client
        .seal(wire::TWRITE, 0x78, [0u8; 8], &Exts::default(), &body, 10)
        .expect("the client seals the next Twrite");
    assert_eq!(
        u64::from_le_bytes(second[16..24].try_into().expect("SeqNonce is 8 bytes")),
        1,
        "the second sealed packet carries sequence 1"
    );
    let n2 = second.len() - port::chan::MAC_LEN;
    let mut got = second[n2 - PAGE_LEN..n2].to_vec();
    keys.c2s
        .keystream(1, 1 + 64, &mut got)
        .expect("sequence 1's page counter is in range");
    assert_eq!(got, page1, "sequence 1's page 1 opens with keystream(1, 1 + 64)");
    let mut wrong = second[n2 - PAGE_LEN..n2].to_vec();
    keys.c2s.keystream(0, 1 + 64, &mut wrong).expect("in range");
    assert_ne!(wrong, page1, "sequence 0's keystream does not open sequence 1's page");
}

#[test]
fn aead_transport_wire_tag_is_packet_mac_of_all_bytes_before_it() {
    let tauth = tauth_frame();
    let rauth = rauth_frame();
    let mut handshake = Handshake::new(Role::Client, config_with_msize(65536));
    handshake.absorb_plain(&tauth);
    handshake.absorb_plain(&rauth);
    let mut client = handshake.finish(secret());

    let mut own = Transcript::new();
    own.absorb(&tauth);
    own.absorb(&rauth);
    let keys = session::derive(&secret(), &own.hash(), 0);

    let data = vec![0x5Au8; PAGE_LEN];
    let data_body = body_of(&Msg::Twrite {
        fid: 9,
        offset: 0,
        data,
    });
    let packet = client
        .seal(wire::TWRITE, 0x79, [0u8; 8], &Exts::default(), &data_body, 0)
        .expect("the client seals a one-page Twrite");
    let n = packet.len() - port::chan::MAC_LEN;
    let tag: [u8; port::chan::MAC_LEN] = packet[n..].try_into().expect("the tag is 32 bytes");

    let mut mac = keys.c2s.mac();
    mac.update(&packet[..n]);
    assert_eq!(tag, mac.finalize(), "the tag is the MAC over every byte before it");

    let mut headerless = keys.c2s.mac();
    headerless.update(&packet[port::chan::HEADER_LEN..n]);
    assert_ne!(
        tag,
        headerless.finalize(),
        "a MAC that leaves the 28-byte header out is a different value"
    );

    let mut short = keys.c2s.mac();
    short.update(&packet[..n - 1]);
    assert_ne!(tag, short.finalize(), "a MAC over one byte fewer is a different value");

    // A sealed Tclunk carrying one TLV record: the same rule holds.
    let exts = Exts {
        obinary: true,
        ..Exts::default()
    };
    let clunk_body = body_of(&Msg::Tclunk { fid: 9 });
    let clunk = client
        .seal(wire::TCLUNK, 0x7A, [0u8; 8], &exts, &clunk_body, 0)
        .expect("the client seals a Tclunk with one TLV record");
    let cn = clunk.len() - port::chan::MAC_LEN;
    let ctag: [u8; port::chan::MAC_LEN] = clunk[cn..].try_into().expect("the tag is 32 bytes");
    let mut cmac = keys.c2s.mac();
    cmac.update(&clunk[..cn]);
    assert_eq!(
        ctag,
        cmac.finalize(),
        "the TLV packet's tag is the MAC over all bytes before it"
    );
}

// ---------------------------------------------------------------------------
// The plan 04-02 tracer pins, unchanged.
// ---------------------------------------------------------------------------

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
