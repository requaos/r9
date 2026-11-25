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
    AeadLink, ServerEnd, Stage, TestExchange, config_with_msize, connect, connect_with,
    fresh_server, handshake_frames, rauth_frame, seal_bypass, session as fixture_session,
    tauth_frame, unauthenticated_session,
};
use std::sync::{Arc, Mutex};

use crypto::key::Key;
use crypto::session::{self, Transcript};
use port::chan::msg::{self, Msg};
use port::chan::tlv::Exts;
use port::chan::transport::{Handshake, Role};
use port::chan::wire;
use port::chan::PAGE_LEN;
use port::dat::Qid;
use port::devmnt::MntIo;

/// The fixed 32-byte session secret: bytes 0x00..=0x1f.
fn secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = i as u8;
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
    let (mut client, mut server) = connect(link, secret(), secret());

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
        TestExchange::new(vec![secret()]),
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

        // Tauth and Rauth keep the standard 9P2000 bodies, compared with
        // test-side bytes written out here.
        let tauth = plain
            .iter()
            .find(|frame| frame.get(4) == Some(&wire::TAUTH))
            .expect("Tauth is on the wire");
        let mut want_tauth = Vec::new();
        want_tauth.extend_from_slice(&common::aead_link::AFID.to_le_bytes());
        want_tauth.extend_from_slice(&4u16.to_le_bytes());
        want_tauth.extend_from_slice(b"none");
        want_tauth.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(&tauth[7..], &want_tauth[..], "Tauth has the standard 9P2000 body");

        let rauth = plain
            .iter()
            .find(|frame| frame.get(4) == Some(&wire::RAUTH))
            .expect("Rauth is on the wire");
        assert_eq!(
            &rauth[7..],
            &[0u8; 13][..],
            "Rauth has the standard 9P2000 qid body (typ, vers, path, all zero)"
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
    let (mut client, _authenticated_server) = connect(link.clone(), secret(), secret());
    let mut server = ServerEnd::new(unauthenticated_session(Role::Server, secret()), link.clone());
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
        let (mut client, mut server) = connect(link, secret(), secret());
        client
            .send(wire::TATTACH, 13, [0u8; 8], &Exts::default(), &tattach_body())
            .expect("the client seals its first Tattach");
        assert!(
            server.recv().is_err(),
            "an altered Rversion ({variant}) makes the first sealed packet's MAC fail"
        );
        assert!(
            server.session.is_closed(),
            "an altered Rversion ({variant}) closes the receiver"
        );
    }
}

#[test]
fn aead_transport_handshake_keys_are_per_connection() {
    let a = AeadLink::new();
    let (mut client_a, _server_a) = connect(a.clone(), secret(), secret());
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
// Failing closed on any authenticated byte (WIRE-03, D-07, D-26).
// ---------------------------------------------------------------------------

/// Deliver one altered packet to a pristine receiver and check it closed.
fn deliver_and_check(altered: &[u8], filler: &[u8], secret: &Key, what: &str) -> Stage {
    let link = AeadLink::new();
    let mut server = fresh_server(link.clone(), secret.clone());
    {
        let mut guard = link.borrow_mut();
        guard.c2s.extend(altered.iter().copied());
        guard.c2s.extend(filler.iter().copied());
    }
    let stage = match server.recv() {
        Err(stage) => stage,
        Ok(_) => panic!("{what}: the receiver accepted an altered packet"),
    };
    assert!(server.session.is_closed(), "{what}: the receiver is closed");
    assert!(server.session.keys_zeroized(), "{what}: the keys are zeroized");
    assert_eq!(link.borrow().decoded(), 0, "{what}: nothing is decoded");
    stage
}

#[test]
fn aead_transport_mac_failure_every_byte() {
    let secret = secret();
    let exts = Exts {
        deadline: Some(1_234_567),
        ..Exts::default()
    };
    let data: Vec<u8> = (0..PAGE_LEN).map(|i| (i % 253) as u8).collect();
    let body = body_of(&Msg::Twrite {
        fid: 9,
        offset: 0,
        data,
    });
    let mut client = fixture_session(Role::Client, secret.clone());
    let packet = client
        .seal(wire::TWRITE, 0x30, [0u8; 8], &exts, &body, 0)
        .expect("the client seals one TLV-bearing one-page Twrite");

    // Further validly sealed packets so every frame length the receiver
    // accepts can be filled.
    let filler_body = body_of(&Msg::Twrite {
        fid: 9,
        offset: 0,
        data: vec![0u8; PAGE_LEN],
    });
    let mut filler = Vec::new();
    for _ in 0..3 {
        filler.extend(
            client
                .seal(wire::TWRITE, 0x31, [0u8; 8], &Exts::default(), &filler_body, 0)
                .expect("the client seals a filler packet"),
        );
    }
    assert!(
        filler.len() >= common::aead_link::MSIZE as usize,
        "the filler covers every frame length the receiver accepts"
    );

    // The four Size bytes, one bit each (32 cases).  Byte 1 is walked first:
    // its high bits grow Size past msize, which is the shape that closes in
    // `frame_len`, and the test asserts both close paths are reached.
    let mut frame_len_closes = 0;
    let mut receive_closes = 0;
    for offset in [1usize, 2, 3, 0] {
        for bit in (0..8).rev() {
            let mut altered = packet.clone();
            altered[offset] ^= 1 << bit;
            let what = format!("Size byte {offset} bit {bit}");
            match deliver_and_check(&altered, &filler, &secret, &what) {
                Stage::FrameLen => frame_len_closes += 1,
                Stage::Receive => receive_closes += 1,
                other => panic!("{what}: unexpected stage {other:?}"),
            }
        }
    }
    assert!(frame_len_closes > 0, "some Size case must close in frame_len");
    assert!(receive_closes > 0, "some Size case must close at the MAC");

    // Every other byte, 4 up to size - 1: header, TLVs, ciphertext and MAC.
    for offset in 4..packet.len() {
        let mut altered = packet.clone();
        altered[offset] ^= 0xFF;
        let what = format!("byte offset {offset}");
        deliver_and_check(&altered, &filler, &secret, &what);
    }

    // One direct `Session::receive` with a slice one byte shorter than its
    // Size field: the length-disagreement close the fixture never reaches.
    let link = AeadLink::new();
    let mut server = fresh_server(link.clone(), secret.clone());
    let mut short = packet[..packet.len() - 1].to_vec();
    assert!(
        server.session.receive(&mut short, 0).is_err(),
        "a slice shorter than its Size field is refused"
    );
    assert!(server.session.is_closed(), "the short slice closes the session");
    assert!(
        server.session.keys_zeroized(),
        "the short slice zeroizes the keys"
    );
    assert_eq!(link.borrow().decoded(), 0, "the short slice decodes nothing");
}

#[test]
fn aead_transport_mac_failure_closes_both_directions() {
    let link = AeadLink::new();
    let (mut client, mut server) = connect(link.clone(), secret(), secret());
    client
        .send(wire::TATTACH, 20, [0u8; 8], &Exts::default(), &tattach_body())
        .expect("the client seals a valid packet");
    let valid: Vec<u8> = link.borrow_mut().c2s.drain(..).collect();

    let mut tampered = valid.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xFF;
    link.borrow_mut().c2s.extend(tampered);
    assert!(server.recv().is_err(), "the tampered packet is refused");
    assert!(server.session.is_closed(), "one failure closes the session");

    assert!(
        server
            .session
            .seal(wire::RWRITE, 21, [0u8; 8], &Exts::default(), &[0u8; 4], 0)
            .is_err(),
        "seal after a failure returns Err(Closed)"
    );
    link.borrow_mut().c2s.extend(valid);
    assert!(
        server.recv().is_err(),
        "a packet that was valid before the failure no longer opens"
    );
}

#[test]
fn aead_transport_mac_failure_zeroizes() {
    let link = AeadLink::new();
    let (mut client, mut server) = connect(link.clone(), secret(), secret());
    assert!(
        !client.session.keys_zeroized() && !server.session.keys_zeroized(),
        "a live session's keys are not zeroized"
    );

    client
        .send(wire::TATTACH, 22, [0u8; 8], &Exts::default(), &tattach_body())
        .expect("the client seals a packet to tamper with");
    {
        let mut guard = link.borrow_mut();
        let last = guard.c2s.len() - 1;
        guard.c2s[last] ^= 0xFF;
    }
    assert!(server.recv().is_err(), "the tampered packet is refused");
    assert!(server.session.is_closed(), "the receiver is closed");
    assert!(server.session.keys_zeroized(), "the receiver's keys are zeroized");
    assert!(
        !client.session.keys_zeroized(),
        "the peer, which saw no failure, still holds its keys"
    );

    // A packet sealed by the peer under the old keys no longer opens.
    client
        .send(wire::TATTACH, 23, [0u8; 8], &Exts::default(), &tattach_body())
        .expect("the live client still seals");
    assert!(
        server.recv().is_err(),
        "a packet sealed after the peer's failure does not open the closed side"
    );
}

#[test]
fn aead_transport_teardown_zeroizes() {
    let link = AeadLink::new();
    let (mut client, mut server) = connect(link.clone(), secret(), secret());

    // A sealed Tclunk round trip neither closes nor scrubs anything (D-15).
    client
        .send(
            wire::TCLUNK,
            24,
            [0u8; 8],
            &Exts::default(),
            &body_of(&Msg::Tclunk { fid: 9 }),
        )
        .expect("the client seals a Tclunk");
    assert!(server.recv().is_ok(), "the sealed Tclunk decodes");
    assert!(!server.session.is_closed(), "Tclunk alone does not close the session");
    assert!(
        !server.session.keys_zeroized(),
        "Tclunk alone does not scrub the keys"
    );

    server.session.teardown();
    assert!(server.session.is_closed(), "teardown closes the session");
    assert!(server.session.keys_zeroized(), "teardown zeroizes the keys");
}

/// Deliver one raw packet to a pristine receiver and check it closed.
fn deliver_raw(packet: &[u8], secret: &Key, what: &str) {
    let link = AeadLink::new();
    let mut server = fresh_server(link.clone(), secret.clone());
    link.borrow_mut().c2s.extend(packet.iter().copied());
    assert!(server.recv().is_err(), "{what}: the receiver refuses the packet");
    assert!(server.session.is_closed(), "{what}: the receiver is closed");
    assert!(server.session.keys_zeroized(), "{what}: the keys are zeroized");
    assert_eq!(link.borrow().decoded(), 0, "{what}: nothing is decoded");
}

#[test]
fn aead_transport_malformed_after_mac_closes() {
    let secret = secret();
    let mut own = Transcript::new();
    for frame in handshake_frames() {
        own.absorb(&frame);
    }
    let keys = session::derive(&secret, &own.hash(), 0);
    let clunk_body = body_of(&Msg::Tclunk { fid: 9 });

    // A valid MAC over a non-zero Pad.
    let packet = seal_bypass(
        &keys.c2s,
        0,
        wire::TCLUNK,
        30,
        1,
        [0u8; 8],
        0,
        &[],
        &clunk_body,
    );
    deliver_raw(&packet, &secret, "nonzero Pad");

    // A valid MAC over an ExtMask bit outside EXT_KNOWN.
    let packet = seal_bypass(
        &keys.c2s,
        0,
        wire::TCLUNK,
        31,
        0,
        [0u8; 8],
        0x0080,
        &[0u8; 8],
        &clunk_body,
    );
    deliver_raw(&packet, &secret, "unknown ExtMask bit");

    // A valid MAC over a TLV record whose type is not in its mask.
    let packet = seal_bypass(
        &keys.c2s,
        0,
        wire::TCLUNK,
        32,
        0,
        [0u8; 8],
        port::chan::EXT_OBINARY,
        &[5, 0, 0, 0, 0, 0, 0, 0],
        &clunk_body,
    );
    deliver_raw(&packet, &secret, "broken TLV record");
}


// ---------------------------------------------------------------------------
// Strict sequence numbers and the splice cases (WIRE-04, WIRE-11; D-15, D-21).
// ---------------------------------------------------------------------------

/// The sequence number at header offset 16.
fn seq_of(packet: &[u8]) -> u64 {
    u64::from_le_bytes(
        packet[16..24]
            .try_into()
            .expect("SeqNonce is 8 bytes at offset 16"),
    )
}

#[test]
fn aead_transport_sequence_starts_at_zero_and_increments() {
    let body = body_of(&Msg::Tclunk { fid: 9 });
    for role in [Role::Client, Role::Server] {
        let mut session = fixture_session(role, secret());
        for expected in 0..3u64 {
            let packet = session
                .seal(wire::TCLUNK, 1, [0u8; 8], &Exts::default(), &body, 0)
                .expect("the session seals a packet");
            assert_eq!(
                seq_of(&packet),
                expected,
                "{role:?} packet {expected} carries its sequence at packet offset 16"
            );
        }
    }
}

#[test]
fn aead_transport_sequence_rejects_repeat_skip_and_earlier() {
    let body = body_of(&Msg::Tclunk { fid: 9 });
    let mut client = fixture_session(Role::Client, secret());
    let packets: Vec<Vec<u8>> = (0..3)
        .map(|i| {
            client
                .seal(wire::TCLUNK, i as u16, [0u8; 8], &Exts::default(), &body, 0)
                .expect("the client seals packets 0, 1 and 2")
        })
        .collect();

    // A repeated packet: packet 0 delivers, the same bytes deliver again.
    {
        let link = AeadLink::new();
        let mut server = fresh_server(link.clone(), secret());
        link.borrow_mut().c2s.extend(packets[0].iter().copied());
        assert!(
            server.recv().is_ok(),
            "the first delivery of packet 0 decodes"
        );
        link.borrow_mut().c2s.extend(packets[0].iter().copied());
        assert!(
            server.recv().is_err(),
            "a repeated packet 0 is rejected by the sequence check"
        );
        assert!(server.session.is_closed(), "P7: the repeat closes the connection");
        assert!(server.session.keys_zeroized(), "P7: the repeat zeroizes the keys");
        assert_eq!(
            link.borrow().decoded(),
            1,
            "the repeated packet is not handed to the handler"
        );
    }

    // A skipped packet: packet 1 arrives before packet 0.
    {
        let link = AeadLink::new();
        let mut server = fresh_server(link.clone(), secret());
        link.borrow_mut().c2s.extend(packets[1].iter().copied());
        assert!(
            server.recv().is_err(),
            "packet 1 delivered before packet 0 is rejected"
        );
        assert!(server.session.is_closed(), "P7: the skip closes the connection");
        assert!(server.session.keys_zeroized(), "P7: the skip zeroizes the keys");
        assert_eq!(link.borrow().decoded(), 0, "the skipped packet decodes nothing");
    }

    // An earlier packet: 0 and 1 deliver, then packet 0 arrives again.
    {
        let link = AeadLink::new();
        let mut server = fresh_server(link.clone(), secret());
        link.borrow_mut().c2s.extend(packets[0].iter().copied());
        assert!(server.recv().is_ok(), "packet 0 decodes first");
        link.borrow_mut().c2s.extend(packets[1].iter().copied());
        assert!(server.recv().is_ok(), "packet 1 decodes second");
        link.borrow_mut().c2s.extend(packets[0].iter().copied());
        assert!(
            server.recv().is_err(),
            "packet 0 delivered after packet 1 is rejected"
        );
        assert!(
            server.session.is_closed(),
            "P7: the earlier packet closes the connection"
        );
        assert!(
            server.session.keys_zeroized(),
            "P7: the earlier packet zeroizes the keys"
        );
        assert_eq!(
            link.borrow().decoded(),
            2,
            "the earlier packet is not handed to the handler"
        );
    }
}

#[test]
fn aead_transport_splice_replayed_twrite_is_dropped() {
    let link = AeadLink::new();
    link.borrow_mut().replay_last = true;
    let (mut client, mut server) = connect(link.clone(), secret(), secret());

    client
        .send(
            wire::TWRITE,
            40,
            [0u8; 8],
            &Exts::default(),
            &body_of(&Msg::Twrite {
                fid: 9,
                offset: 0,
                data: b"captured once".to_vec(),
            }),
        )
        .expect("the client seals its Twrite");
    assert!(
        server.recv().is_ok(),
        "the first delivery of the captured Twrite decodes"
    );
    let before = link.borrow().decoded();
    assert_eq!(before, 1, "exactly one Twrite decoded before the replay");

    assert!(
        server.recv().is_err(),
        "the replayed Twrite is refused by the sequence check"
    );
    assert_eq!(
        link.borrow().decoded(),
        before,
        "the replayed Twrite is not handed to the handler (decode counter unchanged)"
    );
    assert!(server.session.is_closed(), "the replay closes the connection (P7)");
    assert!(server.session.keys_zeroized(), "the replay zeroizes the keys (P7)");
}

#[test]
fn aead_transport_splice_edited_fidhandle_fails_mac() {
    for offset in 8..16usize {
        let link = AeadLink::new();
        link.borrow_mut().flip_byte = Some(offset);
        let (mut client, mut server) = connect(link.clone(), secret(), secret());
        client
            .send(
                wire::TWRITE,
                41,
                [0x11u8; 8],
                &Exts::default(),
                &body_of(&Msg::Twrite {
                    fid: 9,
                    offset: 0,
                    data: b"payload".to_vec(),
                }),
            )
            .expect("the client seals its Twrite");
        assert!(
            server.recv().is_err(),
            "an edited cleartext FidHandle byte {offset} fails the packet MAC"
        );
        assert!(server.session.is_closed(), "offset {offset}: the receiver is closed");
        assert!(
            server.session.keys_zeroized(),
            "offset {offset}: the keys are zeroized"
        );
        assert_eq!(
            link.borrow().decoded(),
            0,
            "offset {offset}: nothing is decoded"
        );
    }
}

#[test]
fn aead_transport_splice_cross_connection_and_reflection_fail() {
    // Two connections with different session secrets.
    let link_a = AeadLink::new();
    let link_b = AeadLink::new();
    let (mut client_a, server_a) = connect(link_a.clone(), secret(), secret());
    let (_client_b, mut server_b) = connect(link_b.clone(), other_secret(), other_secret());

    client_a
        .send(
            wire::TWRITE,
            42,
            [0u8; 8],
            &Exts::default(),
            &body_of(&Msg::Twrite {
                fid: 9,
                offset: 0,
                data: b"connection A".to_vec(),
            }),
        )
        .expect("connection A's client seals a Twrite");
    let captured: Vec<u8> = link_a.borrow().c2s.iter().copied().collect();
    assert!(!captured.is_empty(), "connection A put its packet on the wire");
    link_b.borrow_mut().c2s.extend(captured);
    assert!(
        server_b.recv().is_err(),
        "connection A's packet delivered to connection B's server fails the MAC"
    );
    assert!(server_b.session.is_closed(), "the cross-connection packet closes B");
    assert_eq!(link_b.borrow().decoded(), 0, "B decodes nothing");
    drop(server_a);

    // A reflected client-to-server packet delivered back to the client.
    let link = AeadLink::new();
    link.borrow_mut().reflect = true;
    let (mut client, server) = connect(link.clone(), secret(), secret());
    client
        .send(
            wire::TWRITE,
            43,
            [0u8; 8],
            &Exts::default(),
            &body_of(&Msg::Twrite {
                fid: 9,
                offset: 0,
                data: b"reflect me".to_vec(),
            }),
        )
        .expect("the client seals its Twrite");
    assert!(
        client.recv().is_err(),
        "a client-to-server packet reflected to the client fails the MAC"
    );
    assert!(client.session.is_closed(), "the reflected packet closes the client");
    assert_eq!(link.borrow().decoded(), 0, "the reflection decodes nothing");
    drop(server);
}

// ---------------------------------------------------------------------------
// Ordered sending under one per-direction lock (WIRE-04, D-21).
// ---------------------------------------------------------------------------

/// A sink that appends every whole packet it is handed, shared across threads.
struct SinkIo(Arc<Mutex<Vec<u8>>>);

impl MntIo for SinkIo {
    fn write_msg(&mut self, buf: &[u8]) -> port::Result<u64> {
        self.0
            .lock()
            .expect("the sink is not poisoned")
            .extend_from_slice(buf);
        Ok(buf.len() as u64)
    }

    fn read_some(&mut self, _buf: &mut [u8]) -> port::Result<u64> {
        Ok(0)
    }
}

#[test]
fn aead_transport_send_order_under_contention() {
    let session = fixture_session(Role::Client, secret());
    let half = session.send_half();
    let sink = Arc::new(Mutex::new(Vec::new()));
    let body = body_of(&Msg::Tclunk { fid: 9 });

    std::thread::scope(|scope| {
        for _ in 0..2 {
            let sink = Arc::clone(&sink);
            let body = body.as_slice();
            scope.spawn(move || {
                let mut io = SinkIo(sink);
                for _ in 0..200 {
                    half.send(
                        &mut io,
                        wire::TCLUNK,
                        1,
                        [0u8; 8],
                        &Exts::default(),
                        body,
                        0,
                    )
                    .expect("the shared send half puts a packet on the wire");
                }
            });
        }
    });

    let bytes = sink.lock().expect("the sink is not poisoned").clone();
    let mut seqs = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let size =
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("Size is 4 bytes"))
                as usize;
        assert!(
            size >= port::chan::MIN_PACKET_LEN && offset + size <= bytes.len(),
            "every frame in the sink is whole"
        );
        seqs.push(u64::from_le_bytes(
            bytes[offset + 16..offset + 24]
                .try_into()
                .expect("SeqNonce is 8 bytes"),
        ));
        offset += size;
    }
    let expected: Vec<u64> = (0..400).collect();
    assert_eq!(
        seqs, expected,
        "wire order is 0 to 399 with no gap and no repeat"
    );
}

#[test]
fn aead_transport_send_order_shares_the_session_counter() {
    let mut client = fixture_session(Role::Client, secret());
    let mut server = fixture_session(Role::Server, secret());
    let body = body_of(&Msg::Tclunk { fid: 9 });

    let first = client
        .seal(wire::TCLUNK, 1, [0u8; 8], &Exts::default(), &body, 0)
        .expect("Session::seal seals the first packet");
    assert_eq!(seq_of(&first), 0, "Session::seal seals sequence 0");

    let sink = Arc::new(Mutex::new(Vec::new()));
    {
        let mut io = SinkIo(Arc::clone(&sink));
        client
            .send_half()
            .send(
                &mut io,
                wire::TCLUNK,
                2,
                [0u8; 8],
                &Exts::default(),
                &body,
                0,
            )
            .expect("SendHalf::send seals the next packet");
    }
    let second = sink.lock().expect("the sink is not poisoned").clone();
    assert_eq!(
        seq_of(&second),
        1,
        "SendHalf::send shares Session::seal's counter"
    );

    let mut first = first;
    assert!(
        server.receive(&mut first, 0).is_ok(),
        "the peer opens sequence 0"
    );
    let mut second = second;
    assert!(
        server.receive(&mut second, 0).is_ok(),
        "the peer opens sequence 1"
    );
}

#[test]
fn aead_transport_send_order_closed_session_refuses() {
    let mut client = fixture_session(Role::Client, secret());
    let mut server = fixture_session(Role::Server, secret());
    let body = body_of(&Msg::Tclunk { fid: 9 });

    let mut packet = client
        .seal(wire::TCLUNK, 1, [0u8; 8], &Exts::default(), &body, 0)
        .expect("the client seals a packet to tamper with");
    let last = packet.len() - 1;
    packet[last] ^= 0xFF;
    assert!(
        server.receive(&mut packet, 0).is_err(),
        "the failed open closes the session"
    );
    assert!(server.keys_zeroized(), "the failed open zeroizes the keys");

    let sink = Arc::new(Mutex::new(Vec::new()));
    let mut io = SinkIo(Arc::clone(&sink));
    assert!(
        server
            .send_half()
            .send(
                &mut io,
                wire::RWRITE,
                3,
                [0u8; 8],
                &Exts::default(),
                &[0u8; 4],
                0
            )
            .is_err(),
        "a closed session's SendHalf::send returns Err(Closed)"
    );
    assert!(
        sink.lock().expect("the sink is not poisoned").is_empty(),
        "a closed session writes nothing to the sink"
    );
    assert!(server.keys_zeroized(), "the keys stay zeroized");
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

