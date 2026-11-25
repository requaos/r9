//! The header pin: every field offset, the framing bounds and the canonical
//! hand-written byte vector (WIRE-01; D-06, D-07).
//!
//! Every expected wire byte here is written by hand from the D-06 field list,
//! independently of the encoder under test (the transparency prohibition in
//! the plan; `port/tests/devmnt_test.rs` builds its bytes the same way).  The
//! registry and post-verification header rules are pinned by Task 2.

use port::chan::msg::{self, Msg};
use port::chan::wire::{self, Header};
use port::chan::{
    EXT_CAP_TOKEN, EXT_DEADLINE, EXT_KNOWN, EXT_OBINARY, EXT_QID_VOLATILITY, EXT_STATELESS_CAP,
    EXT_STREAM_PRIO, HEADER_LEN, MAC_LEN, MIN_PACKET_LEN, OFF_EXTLEN, OFF_EXTMASK, OFF_FIDHANDLE,
    OFF_PAD, OFF_SEQ, OFF_SIZE, OFF_TAG, OFF_TYPE, PAGE_LEN,
};
use port::dat::Qid;

#[test]
fn aead_wire_header_offsets_and_length() {
    assert_eq!(HEADER_LEN, 28, "the header is 28 bytes, not 24 (D-06, Pitfall 1)");
    assert_eq!(MAC_LEN, 32, "the packet MAC is 32 bytes (D-01)");
    assert_eq!(MIN_PACKET_LEN, 60, "an empty packet is the header plus the MAC");
    assert_eq!(
        [OFF_SIZE, OFF_TYPE, OFF_TAG, OFF_PAD, OFF_FIDHANDLE, OFF_SEQ, OFF_EXTMASK, OFF_EXTLEN],
        [0, 4, 5, 7, 8, 16, 24, 26],
        "the eight field offsets in D-06 order"
    );
}

#[test]
fn aead_wire_header_hand_vector_decodes() {
    // Written by hand from D-06 with a distinct byte in every field, so a
    // swapped offset cannot pass.  Size 0x3C (60), Type 118, Tag 0x1234,
    // Pad 0, handle 01..08, sequence 0x1122334455667788, mask 0, TLV 0.
    let bytes: [u8; HEADER_LEN] = [
        0x3C, 0x00, 0x00, 0x00, // Size = 0x0000003C
        118, // Type
        0x34, 0x12, // Tag = 0x1234, little-endian
        0x00, // Pad
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // FidHandle
        0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // SeqNonce = 0x1122334455667788
        0x00, 0x00, // ExtMask
        0x00, 0x00, // ExtTLVs Length
    ];

    let header = Header::decode(&bytes);
    assert_eq!(header.size, 0x0000_003C, "Size decodes little-endian from offset 0");
    assert_eq!(header.typ, 118, "Type decodes from offset 4");
    assert_eq!(header.tag, 0x1234, "Tag decodes little-endian from offset 5");
    assert_eq!(header.pad, 0, "Pad decodes from offset 7");
    assert_eq!(header.fid_handle, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], "FidHandle is 8 bytes at offset 8");
    assert_eq!(header.seq, 0x1122_3344_5566_7788, "SeqNonce decodes little-endian from offset 16");
    assert_eq!(header.ext_mask, 0, "ExtMask decodes from offset 24");
    assert_eq!(header.ext_len, 0, "ExtTLVs Length decodes from offset 26");

    let mut out = [0u8; HEADER_LEN];
    header.encode(&mut out);
    assert_eq!(out, bytes, "encode writes the eight fields at their pinned offsets");
}

#[test]
fn aead_wire_header_roundtrip() {
    let cases = [
        Header {
            size: 0,
            typ: 0,
            tag: 0,
            pad: 0,
            fid_handle: [0u8; 8],
            seq: 0,
            ext_mask: 0,
            ext_len: 0,
        },
        Header {
            size: u32::MAX,
            typ: u8::MAX,
            tag: u16::MAX,
            pad: u8::MAX,
            fid_handle: [0xFF; 8],
            seq: u64::MAX,
            ext_mask: u16::MAX,
            ext_len: u16::MAX,
        },
        Header {
            size: 60,
            typ: 118,
            tag: 0x1234,
            pad: 0,
            fid_handle: [1, 2, 3, 4, 5, 6, 7, 8],
            seq: 0x1122_3344_5566_7788,
            ext_mask: 0,
            ext_len: 0,
        },
    ];

    for header in cases {
        let mut out = [0u8; HEADER_LEN];
        header.encode(&mut out);
        assert_eq!(Header::decode(&out), header, "encode then decode is the identity");
    }
}

#[test]
fn aead_wire_header_size_bounds() {
    let msize = 8192u32;
    let claimed = |n: u32| n.to_le_bytes();

    assert!(wire::frame_size(&claimed(59), msize).is_err(), "a claimed Size of 59 is a runt packet");
    assert_eq!(
        wire::frame_size(&claimed(60), msize).expect("60 is the empty packet"),
        60,
        "60 bytes (header plus MAC, empty body) is accepted"
    );
    assert_eq!(
        wire::frame_size(&claimed(msize), msize).expect("Size equal to msize is accepted"),
        msize as usize,
        "Size equal to the negotiated msize is accepted"
    );
    assert!(wire::frame_size(&claimed(msize + 1), msize).is_err(), "msize plus 1 is rejected");
    assert!(
        wire::frame_size(&claimed(60), 59).is_err(),
        "an msize below the minimum packet is rejected"
    );
}

#[test]
fn aead_wire_header_empty_tlv_and_body() {
    let msize = 8192u32;
    assert_eq!(
        wire::packet_len(0, 0, msize).expect("an empty packet fits any real msize"),
        60,
        "a zero-length TLV area and a zero-length body encode to exactly 60 bytes"
    );

    let mut bytes = [0u8; 60];
    bytes[..4].copy_from_slice(&60u32.to_le_bytes());
    let prefix: [u8; 4] = bytes[..4].try_into().expect("the first four bytes are Size");
    assert_eq!(wire::frame_size(&prefix, msize), Ok(60), "the empty packet frames at 60 bytes");

    let header = Header::decode((&bytes[..HEADER_LEN]).try_into().expect("the first 28 bytes"));
    let tlv_start = HEADER_LEN;
    let tlv_end = tlv_start + header.ext_len as usize;
    let body_end = 60 - MAC_LEN;
    assert_eq!(&bytes[tlv_start..tlv_end], &[] as &[u8], "the TLV area is empty");
    assert_eq!(&bytes[tlv_end..body_end], &[] as &[u8], "the body is empty");
}

#[test]
fn aead_wire_header_encode_refuses_oversize() {
    let msize = 8192u32;
    // One byte more than msize allows with an empty TLV area: refused.
    let over = msize as usize - MIN_PACKET_LEN + 1;
    assert!(wire::packet_len(0, over, msize).is_err(), "a body that would pass msize is refused");
    // A length that would not fit the 32-bit Size field: refused, not narrowed.
    assert!(
        wire::packet_len(0, u32::MAX as usize, u32::MAX).is_err(),
        "a packet longer than u32::MAX is refused, never silently narrowed"
    );

    // The refusal happens before the caller writes anything: a sentinel
    // output buffer is untouched by the failed length computation.
    let out = [0xA5u8; HEADER_LEN];
    let snapshot = out;
    assert!(wire::packet_len(0, over, msize).is_err());
    assert_eq!(out, snapshot, "no output is written when the packet length is refused");
}

// ---------------------------------------------------------------------------
// Task 2 pins: the opcode registry and the post-verification header rules
// ---------------------------------------------------------------------------

/// The registry written by hand from fcall.h (100 to 127, `Terror` 106
/// illegal) and RFC section 6.1 (130 to 137), plus P3's re-key pair.
const REGISTRY: [(u8, &str); 37] = [
    (100, "Tversion"),
    (101, "Rversion"),
    (102, "Tauth"),
    (103, "Rauth"),
    (104, "Tattach"),
    (105, "Rattach"),
    (107, "Rerror"),
    (108, "Tflush"),
    (109, "Rflush"),
    (110, "Twalk"),
    (111, "Rwalk"),
    (112, "Topen"),
    (113, "Ropen"),
    (114, "Tcreate"),
    (115, "Rcreate"),
    (116, "Tread"),
    (117, "Rread"),
    (118, "Twrite"),
    (119, "Rwrite"),
    (120, "Tclunk"),
    (121, "Rclunk"),
    (122, "Tremove"),
    (123, "Rremove"),
    (124, "Tstat"),
    (125, "Rstat"),
    (126, "Twstat"),
    (127, "Rwstat"),
    (130, "Tcompound"),
    (131, "Rcompound"),
    (132, "Tstream"),
    (133, "Rstream"),
    (134, "Tlease_revoke"),
    (135, "Rlease_ack"),
    (136, "Treclaim"),
    (137, "Rreclaim"),
    (138, "Trekey"),
    (139, "Rrekey"),
];

#[test]
fn aead_wire_registry_table_is_pinned() {
    for &(value, name) in REGISTRY.iter() {
        assert_eq!(wire::opcode_name(value), Some(name), "opcode {value} names {name}");
    }
    for value in 0u8..=u8::MAX {
        if REGISTRY.iter().all(|&(known, _)| known != value) {
            assert_eq!(wire::opcode_name(value), None, "opcode {value} is not in the registry");
        }
    }
    assert_eq!(
        EXT_KNOWN,
        EXT_DEADLINE | EXT_CAP_TOKEN | EXT_QID_VOLATILITY | EXT_STREAM_PRIO | EXT_OBINARY
            | EXT_STATELESS_CAP,
        "EXT_KNOWN is exactly the OR of the six mask constants (P1)"
    );
}

#[test]
fn aead_wire_registry_rejects_terror_and_gaps() {
    for value in [106u8, 128, 129, 140] {
        assert_eq!(wire::opcode_name(value), None, "opcode {value} must be unknown");
    }
    assert!(wire::opcode_name(107).is_some(), "Rerror (107) is in the base set (D-28)");
    assert!(
        wire::opcode_name(108).is_some() && wire::opcode_name(109).is_some(),
        "Tflush and Rflush (108/109) are in the base set (D-28)"
    );
    for value in 0u8..=u8::MAX {
        let known = wire::opcode_name(value).is_some();
        assert_eq!(
            wire::is_t_message(value),
            known && value % 2 == 0,
            "is_t_message is true exactly for the even T opcodes (opcode {value})"
        );
    }
}

#[test]
fn aead_wire_header_rules_after_verification() {
    fn h(typ: u8, pad: u8, ext_mask: u16, ext_len: u16, fid_handle: [u8; 8]) -> Header {
        Header { size: 60, typ, tag: 7, pad, fid_handle, seq: 3, ext_mask, ext_len }
    }

    // Accepted: a plain header (a non-zero handle with no capability bit is
    // legal), and the capability bit with an all-zero handle (D-12).
    assert!(wire::check_header(&h(118, 0, 0, 0, [9; 8]), 60).is_ok(), "a plain Twrite header is accepted");
    assert!(
        wire::check_header(&h(118, 0, EXT_DEADLINE, 16, [9; 8]), 76).is_ok(),
        "a deadline TLV of its padded 16 bytes is accepted"
    );
    assert!(
        wire::check_header(&h(118, 0, EXT_STATELESS_CAP, 32, [0; 8]), 92).is_ok(),
        "the capability bit with an all-zero handle is accepted"
    );

    // Rejected: a non-zero Pad.
    assert!(wire::check_header(&h(118, 1, 0, 0, [0; 8]), 60).is_err(), "a non-zero Pad is rejected");
    // Rejected: an unknown or illegal opcode.
    assert!(wire::check_header(&h(106, 0, 0, 0, [0; 8]), 60).is_err(), "Terror (106) is rejected");
    assert!(wire::check_header(&h(140, 0, 0, 0, [0; 8]), 60).is_err(), "opcode 140 is rejected");
    // Rejected: a mask bit outside EXT_KNOWN.
    assert!(
        wire::check_header(&h(118, 0, EXT_DEADLINE | 0x0040, 8, [0; 8]), 68).is_err(),
        "an ExtMask bit outside EXT_KNOWN is rejected"
    );
    // Rejected: a TLV length inconsistent with the mask.
    assert!(
        wire::check_header(&h(118, 0, 0, 8, [0; 8]), 68).is_err(),
        "a non-zero TLV length with a zero mask is rejected"
    );
    assert!(
        wire::check_header(&h(118, 0, EXT_DEADLINE, 0, [0; 8]), 60).is_err(),
        "a zero TLV length with a non-zero mask is rejected"
    );
    // Rejected: a TLV area that is not a multiple of 8, or that runs past the
    // packet.
    assert!(
        wire::check_header(&h(118, 0, EXT_STREAM_PRIO, 4, [0; 8]), 64).is_err(),
        "a TLV area length that is not a multiple of 8 is rejected"
    );
    assert!(
        wire::check_header(&h(118, 0, EXT_DEADLINE, 8, [0; 8]), 60).is_err(),
        "a TLV area that runs past the packet is rejected"
    );
    // Rejected: the capability bit with a non-zero header handle (D-12).
    assert!(
        wire::check_header(&h(118, 0, EXT_STATELESS_CAP, 32, [1, 0, 0, 0, 0, 0, 0, 0]), 92).is_err(),
        "the capability bit with a non-zero header handle is rejected"
    );
}

// ---------------------------------------------------------------------------
// Task 1 pins: the base 9P2000 message bodies (WIRE-06, D-28, P4)
// ---------------------------------------------------------------------------

fn pu8(v: &mut Vec<u8>, x: u8) {
    v.push(x);
}
fn pu16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pu32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pu64(v: &mut Vec<u8>, x: u64) {
    v.extend_from_slice(&x.to_le_bytes());
}
/// `convS2M`'s `pstring`: a 16-bit little-endian length then the raw bytes.
fn pstr(v: &mut Vec<u8>, s: &[u8]) {
    pu16(v, s.len() as u16);
    v.extend_from_slice(s);
}
/// `convS2M`'s `pqid`: type[1] vers[4] path[8], little-endian.
fn pqid(v: &mut Vec<u8>, q: &Qid) {
    pu8(v, q.typ);
    pu32(v, q.vers);
    pu64(v, q.path);
}
fn qid(typ: u8, vers: u32, path: u64) -> Qid {
    Qid { path, vers, typ }
}

/// `Qid` has no `PartialEq`; compare two qid slices field-wise.
fn qids_equal(a: &[Qid], b: &[Qid]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| x.path == y.path && x.vers == y.vers && x.typ == y.typ)
}

fn encode(msg: &Msg) -> Vec<u8> {
    let mut out = Vec::new();
    msg::encode_body(msg, &mut out).expect("the message encodes");
    out
}

fn round_trip(msg: &Msg) -> Msg {
    let body = encode(msg);
    msg::decode_body(msg::opcode(msg), &body).expect("the message decodes")
}

/// The plain 9P2000 `Rread` body: `count[4] data[count]` (Task 2 replaces this
/// with the D-09 page-aligned helper).
fn rread_plain(data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    pu32(&mut v, data.len() as u32);
    v.extend_from_slice(data);
    v
}

/// The plain 9P2000 `Twrite` body: `fid[4] offset[8] count[4] data[count]`.
fn twrite_plain(fid: u32, offset: u64, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    pu32(&mut v, fid);
    pu64(&mut v, offset);
    pu32(&mut v, data.len() as u32);
    v.extend_from_slice(data);
    v
}

/// Every base message with its body built by hand from `convS2M`.
fn base_cases() -> Vec<(u8, Msg, Vec<u8>)> {
    let mut cases = Vec::new();

    // Tversion 100
    {
        let version = b"9P2000.aead".to_vec();
        let mut b = Vec::new();
        pu32(&mut b, 8192);
        pstr(&mut b, &version);
        cases.push((wire::TVERSION, Msg::Tversion { msize: 8192, version }, b));
    }
    // Rversion 101
    {
        let version = b"9P2000".to_vec();
        let mut b = Vec::new();
        pu32(&mut b, 4096);
        pstr(&mut b, &version);
        cases.push((wire::RVERSION, Msg::Rversion { msize: 4096, version }, b));
    }
    // Tauth 102
    {
        let uname = b"glenda".to_vec();
        let aname = b"/".to_vec();
        let mut b = Vec::new();
        pu32(&mut b, 7);
        pstr(&mut b, &uname);
        pstr(&mut b, &aname);
        cases.push((wire::TAUTH, Msg::Tauth { afid: 7, uname, aname }, b));
    }
    // Rauth 103
    {
        let aqid = qid(0x80, 3, 0x0102_0304_0506_0708);
        let mut b = Vec::new();
        pqid(&mut b, &aqid);
        cases.push((wire::RAUTH, Msg::Rauth { aqid }, b));
    }
    // Tattach 104
    {
        let uname = b"glenda".to_vec();
        let aname = b"/".to_vec();
        let mut b = Vec::new();
        pu32(&mut b, 1);
        pu32(&mut b, u32::MAX);
        pstr(&mut b, &uname);
        pstr(&mut b, &aname);
        cases.push((
            wire::TATTACH,
            Msg::Tattach {
                fid: 1,
                afid: u32::MAX,
                uname,
                aname,
            },
            b,
        ));
    }
    // Rattach 105
    {
        let q = qid(0x80, 1, 0x1122_3344_5566_7788);
        let mut b = Vec::new();
        pqid(&mut b, &q);
        cases.push((wire::RATTACH, Msg::Rattach { qid: q }, b));
    }
    // Rerror 107
    {
        let ename = b"not found".to_vec();
        let mut b = Vec::new();
        pstr(&mut b, &ename);
        cases.push((wire::RERROR, Msg::Rerror { ename }, b));
    }
    // Tflush 108
    {
        let mut b = Vec::new();
        pu16(&mut b, 0x1234);
        cases.push((wire::TFLUSH, Msg::Tflush { oldtag: 0x1234 }, b));
    }
    // Rflush 109
    cases.push((wire::RFLUSH, Msg::Rflush, Vec::new()));
    // Twalk 110
    {
        let wname = vec![b"usr".to_vec(), b"glenda".to_vec()];
        let mut b = Vec::new();
        pu32(&mut b, 1);
        pu32(&mut b, 2);
        pu16(&mut b, wname.len() as u16);
        for n in &wname {
            pstr(&mut b, n);
        }
        cases.push((
            wire::TWALK,
            Msg::Twalk {
                fid: 1,
                newfid: 2,
                wname,
            },
            b,
        ));
    }
    // Rwalk 111
    {
        let wqid = vec![qid(0x80, 1, 10), qid(0x00, 2, 20)];
        let mut b = Vec::new();
        pu16(&mut b, wqid.len() as u16);
        for q in &wqid {
            pqid(&mut b, q);
        }
        cases.push((wire::RWALK, Msg::Rwalk { wqid }, b));
    }
    // Topen 112
    {
        let mut b = Vec::new();
        pu32(&mut b, 1);
        pu8(&mut b, 2);
        cases.push((wire::TOPEN, Msg::Topen { fid: 1, mode: 2 }, b));
    }
    // Ropen 113
    {
        let q = qid(0x00, 1, 10);
        let mut b = Vec::new();
        pqid(&mut b, &q);
        pu32(&mut b, 8192);
        cases.push((
            wire::ROPEN,
            Msg::Ropen {
                qid: q,
                iounit: 8192,
            },
            b,
        ));
    }
    // Tcreate 114
    {
        let name = b"x".to_vec();
        let mut b = Vec::new();
        pu32(&mut b, 1);
        pstr(&mut b, &name);
        pu32(&mut b, 0o644);
        pu8(&mut b, 2);
        cases.push((
            wire::TCREATE,
            Msg::Tcreate {
                fid: 1,
                name,
                perm: 0o644,
                mode: 2,
            },
            b,
        ));
    }
    // Rcreate 115
    {
        let q = qid(0x00, 1, 11);
        let mut b = Vec::new();
        pqid(&mut b, &q);
        pu32(&mut b, 4096);
        cases.push((
            wire::RCREATE,
            Msg::Rcreate {
                qid: q,
                iounit: 4096,
            },
            b,
        ));
    }
    // Tread 116
    {
        let mut b = Vec::new();
        pu32(&mut b, 1);
        pu64(&mut b, 0x1000);
        pu32(&mut b, 4096);
        cases.push((
            wire::TREAD,
            Msg::Tread {
                fid: 1,
                offset: 0x1000,
                count: 4096,
            },
            b,
        ));
    }
    // Rread 117 (plain, Task 1)
    {
        let data = b"hello".to_vec();
        cases.push((wire::RREAD, Msg::Rread { data: data.clone() }, rread_plain(&data)));
    }
    // Twrite 118 (plain, Task 1)
    {
        let data = b"hello".to_vec();
        cases.push((
            wire::TWRITE,
            Msg::Twrite {
                fid: 1,
                offset: 0,
                data: data.clone(),
            },
            twrite_plain(1, 0, &data),
        ));
    }
    // Rwrite 119
    {
        let mut b = Vec::new();
        pu32(&mut b, 5);
        cases.push((wire::RWRITE, Msg::Rwrite { count: 5 }, b));
    }
    // Tclunk 120
    {
        let mut b = Vec::new();
        pu32(&mut b, 1);
        cases.push((wire::TCLUNK, Msg::Tclunk { fid: 1 }, b));
    }
    // Rclunk 121
    cases.push((wire::RCLUNK, Msg::Rclunk, Vec::new()));
    // Tremove 122
    {
        let mut b = Vec::new();
        pu32(&mut b, 1);
        cases.push((wire::TREMOVE, Msg::Tremove { fid: 1 }, b));
    }
    // Rremove 123
    cases.push((wire::RREMOVE, Msg::Rremove, Vec::new()));
    // Tstat 124
    {
        let mut b = Vec::new();
        pu32(&mut b, 1);
        cases.push((wire::TSTAT, Msg::Tstat { fid: 1 }, b));
    }
    // Rstat 125
    {
        let stat = vec![0x01, 0x02, 0x03];
        let mut b = Vec::new();
        pstr(&mut b, &stat);
        cases.push((wire::RSTAT, Msg::Rstat { stat }, b));
    }
    // Twstat 126
    {
        let stat = vec![0x04, 0x05];
        let mut b = Vec::new();
        pu32(&mut b, 1);
        pstr(&mut b, &stat);
        cases.push((wire::TWSTAT, Msg::Twstat { fid: 1, stat }, b));
    }
    // Rwstat 127
    cases.push((wire::RWSTAT, Msg::Rwstat, Vec::new()));

    cases
}

#[test]
fn aead_wire_messages_base_set_roundtrip() {
    for (typ, msg, bytes) in base_cases() {
        assert_eq!(msg::opcode(&msg), typ, "opcode for {typ}");
        assert_eq!(encode(&msg), bytes, "encode_body matches the hand-built body for {typ}");
        assert_eq!(
            msg::decode_body(typ, &bytes),
            Ok(msg),
            "decode_body inverts encode_body for {typ}"
        );
    }
}

#[test]
fn aead_wire_messages_base_set_covers_the_registry() {
    let cases = base_cases();
    assert_eq!(cases.len(), 27, "the 27 base messages 100 to 127 without 106");
    for typ in 100u8..=127 {
        let present = cases.iter().any(|(t, _, _)| *t == typ);
        if typ == 106 {
            assert!(!present, "Terror 106 is absent from the table");
            assert!(
                msg::decode_body(106, &[]).is_err(),
                "decode_body has no arm for illegal Terror 106"
            );
        } else {
            assert!(present, "opcode {typ} has a round-trip case");
        }
    }
}

#[test]
fn aead_wire_messages_strings_are_raw_bytes() {
    // Bytes that are not valid UTF-8 survive unchanged.
    let raw = vec![0xff, 0xfe, 0x80, 0x00, 0x41];
    let m = Msg::Tversion {
        msize: 1024,
        version: raw.clone(),
    };
    let mut want = Vec::new();
    pu32(&mut want, 1024);
    pstr(&mut want, &raw);
    assert_eq!(encode(&m), want, "a non-UTF-8 string is written as raw bytes");
    assert_eq!(round_trip(&m), m);

    // An empty string round-trips.
    let m = Msg::Tversion {
        msize: 1,
        version: Vec::new(),
    };
    assert_eq!(round_trip(&m), m, "an empty string round-trips");

    // 65535 bytes is the encoder's limit; 65536 is refused.
    let m = Msg::Tversion {
        msize: 1,
        version: vec![b'a'; 65535],
    };
    assert_eq!(round_trip(&m), m, "a 65535-byte string round-trips");
    let too = Msg::Tversion {
        msize: 1,
        version: vec![b'a'; 65536],
    };
    let mut out = Vec::new();
    assert!(
        msg::encode_body(&too, &mut out).is_err(),
        "a 65536-byte string is an encoder error, never truncated"
    );

    // The stat blob has the same 16-bit length.
    let m = Msg::Rstat {
        stat: vec![0x5a; 65535],
    };
    assert_eq!(round_trip(&m), m, "a 65535-byte stat blob round-trips");
    let too = Msg::Rstat {
        stat: vec![0x5a; 65536],
    };
    assert!(
        msg::encode_body(&too, &mut Vec::new()).is_err(),
        "a 65536-byte stat blob is an encoder error"
    );
}

#[test]
fn aead_wire_messages_walk_order_and_limits() {
    // Twalk with 0, 1 and 16 names round-trips in order.
    for n in [0usize, 1, 16] {
        let wname: Vec<Vec<u8>> = (0..n).map(|i| vec![b'a' + i as u8]).collect();
        let m = Msg::Twalk {
            fid: 1,
            newfid: 2,
            wname: wname.clone(),
        };
        let got = round_trip(&m);
        assert_eq!(got, m, "Twalk with {n} names round-trips");
        let Msg::Twalk { wname: got_names, .. } = got else {
            panic!("Twalk decoded to the wrong variant");
        };
        assert_eq!(got_names, wname, "walk names keep their order");
    }
    // 17 names are refused by the encoder.
    let m17 = Msg::Twalk {
        fid: 1,
        newfid: 2,
        wname: (0..17).map(|i| vec![i as u8]).collect(),
    };
    assert!(
        msg::encode_body(&m17, &mut Vec::new()).is_err(),
        "17 Twalk names are an encoder error"
    );
    // and by the decoder (nwname[2] = 17).
    let mut bad = Vec::new();
    pu32(&mut bad, 1);
    pu32(&mut bad, 2);
    pu16(&mut bad, 17);
    for i in 0..17u8 {
        pstr(&mut bad, &[i]);
    }
    assert!(
        msg::decode_body(wire::TWALK, &bad).is_err(),
        "17 Twalk names are a decoder error"
    );

    // Rwalk with 0 and 16 qids round-trips in order.
    for n in [0usize, 16] {
        let wqid: Vec<Qid> = (0..n).map(|i| qid(0x00, i as u32, i as u64)).collect();
        let m = Msg::Rwalk { wqid: wqid.clone() };
        let got = round_trip(&m);
        assert_eq!(got, m, "Rwalk with {n} qids round-trips");
        let Msg::Rwalk { wqid: got_qids } = got else {
            panic!("Rwalk decoded to the wrong variant");
        };
        assert!(qids_equal(&got_qids, &wqid), "walk qids keep their order");
    }
    // 17 qids are refused by the encoder.
    let m17 = Msg::Rwalk {
        wqid: (0..17u32).map(|i| qid(0, i, i as u64)).collect(),
    };
    assert!(
        msg::encode_body(&m17, &mut Vec::new()).is_err(),
        "17 Rwalk qids are an encoder error"
    );
    // and by the decoder (nwqid[2] = 17).
    let mut bad = Vec::new();
    pu16(&mut bad, 17);
    for i in 0..17u32 {
        pqid(&mut bad, &qid(0, i, i as u64));
    }
    assert!(
        msg::decode_body(wire::RWALK, &bad).is_err(),
        "17 Rwalk qids are a decoder error"
    );
}

#[test]
fn aead_wire_messages_reject_truncated_and_trailing() {
    for (typ, _msg, bytes) in base_cases() {
        if !bytes.is_empty() {
            assert!(
                msg::decode_body(typ, &bytes[..bytes.len() - 1]).is_err(),
                "a {typ} body truncated by one byte is a decode error"
            );
        }
        let mut appended = bytes.clone();
        appended.push(0);
        assert!(
            msg::decode_body(typ, &appended).is_err(),
            "a {typ} body with one trailing byte is a decode error"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 2 pins: the new pairs 130 to 139 and the D-09 page-aligned data bodies
// ---------------------------------------------------------------------------

/// A buffer padded with zeros to a whole number of 4096-byte pages.
fn padded(data: &[u8]) -> Vec<u8> {
    let mut v = data.to_vec();
    let rem = v.len() % PAGE_LEN;
    if rem != 0 {
        v.resize(v.len() + (PAGE_LEN - rem), 0);
    }
    v
}

/// One sample message for every registry opcode, so a missing decoder arm is
/// a failing round trip rather than a silent wildcard.
fn sample_message(typ: u8) -> Msg {
    match typ {
        wire::TVERSION => Msg::Tversion {
            msize: 8192,
            version: b"9P2000.aead".to_vec(),
        },
        wire::RVERSION => Msg::Rversion {
            msize: 8192,
            version: b"9P2000.aead".to_vec(),
        },
        wire::TAUTH => Msg::Tauth {
            afid: 2,
            uname: b"u".to_vec(),
            aname: b"/".to_vec(),
        },
        wire::RAUTH => Msg::Rauth {
            aqid: qid(0, 1, 2),
        },
        wire::TATTACH => Msg::Tattach {
            fid: 1,
            afid: 2,
            uname: b"u".to_vec(),
            aname: b"/".to_vec(),
        },
        wire::RATTACH => Msg::Rattach { qid: qid(0, 1, 2) },
        wire::RERROR => Msg::Rerror {
            ename: b"e".to_vec(),
        },
        wire::TFLUSH => Msg::Tflush { oldtag: 3 },
        wire::RFLUSH => Msg::Rflush,
        wire::TWALK => Msg::Twalk {
            fid: 1,
            newfid: 2,
            wname: vec![b"a".to_vec()],
        },
        wire::RWALK => Msg::Rwalk {
            wqid: vec![qid(0, 1, 2)],
        },
        wire::TOPEN => Msg::Topen { fid: 1, mode: 2 },
        wire::ROPEN => Msg::Ropen {
            qid: qid(0, 1, 2),
            iounit: 4096,
        },
        wire::TCREATE => Msg::Tcreate {
            fid: 1,
            name: b"x".to_vec(),
            perm: 0o644,
            mode: 2,
        },
        wire::RCREATE => Msg::Rcreate {
            qid: qid(0, 1, 2),
            iounit: 4096,
        },
        wire::TREAD => Msg::Tread {
            fid: 1,
            offset: 0,
            count: 16,
        },
        wire::RREAD => Msg::Rread {
            data: b"hello".to_vec(),
        },
        wire::TWRITE => Msg::Twrite {
            fid: 1,
            offset: 0,
            data: b"hello".to_vec(),
        },
        wire::RWRITE => Msg::Rwrite { count: 5 },
        wire::TCLUNK => Msg::Tclunk { fid: 1 },
        wire::RCLUNK => Msg::Rclunk,
        wire::TREMOVE => Msg::Tremove { fid: 1 },
        wire::RREMOVE => Msg::Rremove,
        wire::TSTAT => Msg::Tstat { fid: 1 },
        wire::RSTAT => Msg::Rstat {
            stat: vec![1, 2, 3],
        },
        wire::TWSTAT => Msg::Twstat {
            fid: 1,
            stat: vec![4, 5],
        },
        wire::RWSTAT => Msg::Rwstat,
        wire::TCOMPOUND => Msg::Tcompound {
            num_ops: 2,
            op_stream: vec![1, 2, 3],
        },
        wire::RCOMPOUND => Msg::Rcompound {
            num_completed: 1,
            status: 0,
            result_stream: vec![9],
        },
        wire::TSTREAM => Msg::Tstream {
            fid_handle: [1, 2, 3, 4, 5, 6, 7, 8],
            stream_id: 4,
            flags: 0x0102,
            offset: 0x1122_3344_5566_7788,
        },
        wire::RSTREAM => Msg::Rstream {
            stream_id: 4,
            chunk_seq: 5,
            data: b"abc".to_vec(),
        },
        wire::TLEASE_REVOKE => Msg::Tlease_revoke {
            qid_path: 1,
            lease_gen: 2,
            target_mode: 0x01,
            num_blocks: 1,
            block_ranges: vec![1, 0],
        },
        wire::RLEASE_ACK => Msg::Rlease_ack {
            qid_path: 1,
            lease_gen: 2,
            client_status: 0,
        },
        wire::TRECLAIM => Msg::Treclaim {
            old_handle: [1, 2, 3, 4, 5, 6, 7, 8],
            target_epoch: 5,
            client_token: [0xab; 32],
        },
        wire::RRECLAIM => Msg::Rreclaim {
            new_handle: [9, 10, 11, 12, 13, 14, 15, 16],
            recovery_status: 7,
        },
        wire::TREKEY => Msg::Trekey {
            kind: msg::REKEY_REQUEST,
            generation: 42,
            confirm: [0x11; 32],
        },
        wire::RREKEY => Msg::Rrekey {
            kind: msg::REKEY_ACK,
            generation: 42,
            confirm: [0x22; 32],
        },
        _ => panic!("no sample message for opcode {typ}"),
    }
}

/// One data message with the given payload.
fn data_msg(typ: u8, data: Vec<u8>) -> Msg {
    match typ {
        wire::TWRITE => Msg::Twrite {
            fid: 7,
            offset: 3,
            data,
        },
        wire::RREAD => Msg::Rread { data },
        wire::RSTREAM => Msg::Rstream {
            stream_id: 9,
            chunk_seq: 11,
            data,
        },
        _ => panic!("opcode {typ} is not a data message"),
    }
}

/// The parameter-block length of a data message.
fn params_len(typ: u8) -> usize {
    match typ {
        wire::TWRITE => 16,
        wire::RREAD => 4,
        wire::RSTREAM => 16,
        _ => panic!("opcode {typ} is not a data message"),
    }
}

/// The byte offset of the true `count` inside a data message's parameters.
fn count_offset(typ: u8) -> usize {
    match typ {
        wire::TWRITE => 12,
        wire::RREAD => 0,
        wire::RSTREAM => 12,
        _ => panic!("opcode {typ} is not a data message"),
    }
}

#[test]
fn aead_wire_messages_new_pairs_roundtrip() {
    // Every registry opcode has a decoding arm: a missing arm fails here.
    for &(typ, name) in REGISTRY.iter() {
        let m = sample_message(typ);
        assert_eq!(msg::opcode(&m), typ, "opcode for {name}");
        let body = encode(&m);
        assert_eq!(
            msg::decode_body(typ, &body),
            Ok(m),
            "opcode {typ} ({name}) round-trips"
        );
    }

    // The eight new pairs and both re-key messages against hand-built bytes.
    let mut cases: Vec<(u8, Msg, Vec<u8>)> = Vec::new();

    // Tcompound 130
    {
        let op_stream = vec![0x00, 0x01, 0x02, 0x03, 0x04];
        let mut b = Vec::new();
        pu16(&mut b, 3);
        b.extend_from_slice(&op_stream);
        cases.push((
            wire::TCOMPOUND,
            Msg::Tcompound {
                num_ops: 3,
                op_stream,
            },
            b,
        ));
    }
    // Rcompound 131
    {
        let result_stream = vec![0xaa, 0xbb];
        let mut b = Vec::new();
        pu16(&mut b, 2);
        pu16(&mut b, 0);
        b.extend_from_slice(&result_stream);
        cases.push((
            wire::RCOMPOUND,
            Msg::Rcompound {
                num_completed: 2,
                status: 0,
                result_stream,
            },
            b,
        ));
    }
    // Tstream 132
    {
        let fid_handle = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut b = Vec::new();
        b.extend_from_slice(&fid_handle);
        pu32(&mut b, 0x0a0b_0c0d);
        pu16(&mut b, 0x0102);
        pu64(&mut b, 0x1122_3344_5566_7788);
        cases.push((
            wire::TSTREAM,
            Msg::Tstream {
                fid_handle,
                stream_id: 0x0a0b_0c0d,
                flags: 0x0102,
                offset: 0x1122_3344_5566_7788,
            },
            b,
        ));
    }
    // Rstream 133
    {
        let data = b"abcde".to_vec();
        let mut b = Vec::new();
        pu32(&mut b, 0x0a0b_0c0d);
        pu64(&mut b, 0x1122_3344_5566_7788);
        pu32(&mut b, data.len() as u32);
        b.extend_from_slice(&padded(&data));
        cases.push((
            wire::RSTREAM,
            Msg::Rstream {
                stream_id: 0x0a0b_0c0d,
                chunk_seq: 0x1122_3344_5566_7788,
                data,
            },
            b,
        ));
    }
    // Tlease_revoke 134
    {
        let block_ranges = vec![0x01, 0x00, 0x02, 0x00];
        let mut b = Vec::new();
        pu64(&mut b, 0x0102_0304_0506_0708);
        pu64(&mut b, 0x1112_1314_1516_1718);
        pu8(&mut b, 0x02);
        pu32(&mut b, 2);
        b.extend_from_slice(&block_ranges);
        cases.push((
            wire::TLEASE_REVOKE,
            Msg::Tlease_revoke {
                qid_path: 0x0102_0304_0506_0708,
                lease_gen: 0x1112_1314_1516_1718,
                target_mode: 0x02,
                num_blocks: 2,
                block_ranges,
            },
            b,
        ));
    }
    // Rlease_ack 135
    {
        let mut b = Vec::new();
        pu64(&mut b, 0x0102_0304_0506_0708);
        pu64(&mut b, 0x1112_1314_1516_1718);
        pu8(&mut b, 0x00);
        cases.push((
            wire::RLEASE_ACK,
            Msg::Rlease_ack {
                qid_path: 0x0102_0304_0506_0708,
                lease_gen: 0x1112_1314_1516_1718,
                client_status: 0x00,
            },
            b,
        ));
    }
    // Treclaim 136
    {
        let old_handle = [1, 2, 3, 4, 5, 6, 7, 8];
        let client_token = [0xab; 32];
        let mut b = Vec::new();
        b.extend_from_slice(&old_handle);
        pu16(&mut b, 0x1234);
        b.extend_from_slice(&client_token);
        cases.push((
            wire::TRECLAIM,
            Msg::Treclaim {
                old_handle,
                target_epoch: 0x1234,
                client_token,
            },
            b,
        ));
    }
    // Rreclaim 137
    {
        let new_handle = [9, 10, 11, 12, 13, 14, 15, 16];
        let mut b = Vec::new();
        b.extend_from_slice(&new_handle);
        pu16(&mut b, 0x0102);
        cases.push((
            wire::RRECLAIM,
            Msg::Rreclaim {
                new_handle,
                recovery_status: 0x0102,
            },
            b,
        ));
    }
    // Trekey 138
    {
        let confirm = [0x11; 32];
        let mut b = Vec::new();
        pu8(&mut b, msg::REKEY_REQUEST);
        pu32(&mut b, 0x0102_0304);
        b.extend_from_slice(&confirm);
        cases.push((
            wire::TREKEY,
            Msg::Trekey {
                kind: msg::REKEY_REQUEST,
                generation: 0x0102_0304,
                confirm,
            },
            b,
        ));
    }
    // Rrekey 139
    {
        let confirm = [0x22; 32];
        let mut b = Vec::new();
        pu8(&mut b, msg::REKEY_ACK);
        pu32(&mut b, 0x0102_0304);
        b.extend_from_slice(&confirm);
        cases.push((
            wire::RREKEY,
            Msg::Rrekey {
                kind: msg::REKEY_ACK,
                generation: 0x0102_0304,
                confirm,
            },
            b,
        ));
    }

    assert_eq!(cases.len(), 10, "eight new pairs plus the two re-key messages");
    for (typ, m, bytes) in cases {
        assert_eq!(msg::opcode(&m), typ, "opcode for {typ}");
        assert_eq!(encode(&m), bytes, "encode_body matches the hand-built body for {typ}");
        assert_eq!(
            msg::decode_body(typ, &bytes),
            Ok(m),
            "decode_body inverts encode_body for {typ}"
        );
    }
}

#[test]
fn aead_wire_messages_rekey_body() {
    let mut want = Vec::new();
    pu8(&mut want, msg::REKEY_REQUEST);
    pu32(&mut want, 0x0102_0304);
    want.extend_from_slice(&[0x11; 32]);
    assert_eq!(want.len(), 37, "the re-key body is kind[1] generation[4] confirm[32]");

    let m = Msg::Trekey {
        kind: msg::REKEY_REQUEST,
        generation: 0x0102_0304,
        confirm: [0x11; 32],
    };
    assert_eq!(msg::decode_body(wire::TREKEY, &want), Ok(m.clone()));
    assert_eq!(encode(&m), want, "encode writes the 37-byte re-key body");

    for kind in [msg::REKEY_REQUEST, msg::REKEY_MARKER, msg::REKEY_ACK] {
        let t = Msg::Trekey {
            kind,
            generation: 7,
            confirm: [0; 32],
        };
        assert_eq!(round_trip(&t), t, "Trekey kind {kind} round-trips");
        let r = Msg::Rrekey {
            kind,
            generation: 7,
            confirm: [0; 32],
        };
        assert_eq!(round_trip(&r), r, "Rrekey kind {kind} round-trips");
    }

    let mut bad = want.clone();
    bad[0] = 3;
    assert!(
        msg::decode_body(wire::TREKEY, &bad).is_err(),
        "re-key kind 3 is a decode error"
    );
}

#[test]
fn aead_wire_messages_data_bodies_are_whole_pages() {
    for typ in [wire::TWRITE, wire::RREAD, wire::RSTREAM] {
        for count in [0usize, 1, 4095, 4096, 4097] {
            let data: Vec<u8> = (0..count).map(|i| (i % 251) as u8).collect();
            let m = data_msg(typ, data.clone());
            let body = encode(&m);

            let pages = count.div_ceil(PAGE_LEN);
            assert_eq!(
                body.len() - params_len(typ),
                pages * PAGE_LEN,
                "{count} bytes of {typ} data occupy {pages} whole pages"
            );
            let off = count_offset(typ);
            let wire_count = u32::from_le_bytes(
                body[off..off + 4].try_into().expect("four count bytes"),
            );
            assert_eq!(wire_count as usize, count, "the parameters carry the true count");
            assert!(
                body[params_len(typ) + count..].iter().all(|&b| b == 0),
                "the data padding is zero"
            );

            let got = msg::decode_body(typ, &body).expect("a whole-page data body decodes");
            let got_data = match got {
                Msg::Twrite { data, .. } | Msg::Rread { data } | Msg::Rstream { data, .. } => data,
                other => panic!("{typ} decoded to {other:?}"),
            };
            assert_eq!(got_data, data, "decode returns exactly the {count} true bytes");
        }
    }
}

#[test]
fn aead_wire_messages_data_bounds() {
    let body = encode(&Msg::Twrite {
        fid: 1,
        offset: 0,
        data: vec![1, 2, 3, 4, 5],
    });

    // A data area that is not a whole number of pages.
    let mut a = body.clone();
    a.pop();
    assert!(
        msg::decode_body(wire::TWRITE, &a).is_err(),
        "a data area that is not a multiple of 4096 is a decode error"
    );

    // A count greater than the data area.
    let mut b = Vec::new();
    pu32(&mut b, 1);
    pu64(&mut b, 0);
    pu32(&mut b, 5000);
    b.extend_from_slice(&vec![0u8; PAGE_LEN]);
    assert!(
        msg::decode_body(wire::TWRITE, &b).is_err(),
        "a count above the data length is a decode error"
    );

    // A data area a whole page longer than the count can need.
    let mut c = Vec::new();
    pu32(&mut c, 1);
    pu64(&mut c, 0);
    pu32(&mut c, 100);
    c.extend_from_slice(&vec![0u8; PAGE_LEN * 2]);
    assert!(
        msg::decode_body(wire::TWRITE, &c).is_err(),
        "data a full page beyond the count is a decode error"
    );

    // A non-zero padding byte.
    let mut d = body.clone();
    d[params_len(wire::TWRITE) + 5] = 1;
    assert!(
        msg::decode_body(wire::TWRITE, &d).is_err(),
        "a non-zero padding byte is a decode error"
    );

    // A count exactly equal to the data length is accepted.
    let mut e = Vec::new();
    pu32(&mut e, 1);
    pu64(&mut e, 0);
    pu32(&mut e, PAGE_LEN as u32);
    e.extend_from_slice(&vec![7u8; PAGE_LEN]);
    assert_eq!(
        msg::decode_body(wire::TWRITE, &e),
        Ok(Msg::Twrite {
            fid: 1,
            offset: 0,
            data: vec![7u8; PAGE_LEN],
        }),
        "a count equal to the data length is accepted"
    );

    // The same bound applies to Rread and Rstream.
    let mut r = Vec::new();
    pu32(&mut r, 10);
    r.extend_from_slice(&vec![0u8; 100]);
    assert!(
        msg::decode_body(wire::RREAD, &r).is_err(),
        "a non-page Rread data area is a decode error"
    );
    let mut s = Vec::new();
    pu32(&mut s, 1);
    pu64(&mut s, 2);
    pu32(&mut s, 3);
    s.extend_from_slice(&vec![0u8; 100]);
    assert!(
        msg::decode_body(wire::RSTREAM, &s).is_err(),
        "a non-page Rstream data area is a decode error"
    );
}

#[test]
fn aead_wire_messages_data_split_matches_layout() {
    let body = encode(&data_msg(wire::TWRITE, vec![0u8; 4097]));
    assert_eq!(
        msg::data_layout(wire::TWRITE, &body),
        Some(msg::DataLayout {
            params_len: 16,
            pages: 2
        }),
        "Twrite: 16 parameter bytes then two pages"
    );
    let body = encode(&data_msg(wire::RREAD, vec![0u8; 4096]));
    assert_eq!(
        msg::data_layout(wire::RREAD, &body),
        Some(msg::DataLayout {
            params_len: 4,
            pages: 1
        }),
        "Rread: 4 parameter bytes then one page"
    );
    let body = encode(&data_msg(wire::RSTREAM, Vec::new()));
    assert_eq!(
        msg::data_layout(wire::RSTREAM, &body),
        Some(msg::DataLayout {
            params_len: 16,
            pages: 0
        }),
        "Rstream: 16 parameter bytes then zero pages"
    );

    // Exactly the three data opcodes return Some.
    let mut some = 0;
    for &(typ, _name) in REGISTRY.iter() {
        let is_data = [wire::TWRITE, wire::RREAD, wire::RSTREAM].contains(&typ);
        let layout = msg::data_layout(typ, &[]);
        if is_data {
            assert!(layout.is_some(), "opcode {typ} is a data message");
            some += 1;
        } else {
            assert!(layout.is_none(), "opcode {typ} is not a data message");
        }
    }
    assert_eq!(some, 3, "data_layout returns Some for exactly three opcodes");

    assert_eq!(msg::MAX_PARAMS_LEN, 64, "a parameter block is at most one ChaCha20 block");
    for typ in [wire::TWRITE, wire::RREAD, wire::RSTREAM] {
        let layout = msg::data_layout(typ, &[]).expect("a data opcode");
        assert!(
            layout.params_len <= msg::MAX_PARAMS_LEN,
            "opcode {typ} parameters fit one block"
        );
    }
}
