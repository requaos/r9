//! The header pin: every field offset, the framing bounds and the canonical
//! hand-written byte vector (WIRE-01; D-06, D-07).
//!
//! Every expected wire byte here is written by hand from the D-06 field list,
//! independently of the encoder under test (the transparency prohibition in
//! the plan; `port/tests/devmnt_test.rs` builds its bytes the same way).  The
//! registry and post-verification header rules are pinned by Task 2.

use port::chan::wire::{self, Header};
use port::chan::{
    EXT_CAP_TOKEN, EXT_DEADLINE, EXT_KNOWN, EXT_OBINARY, EXT_QID_VOLATILITY, EXT_STATELESS_CAP,
    EXT_STREAM_PRIO, HEADER_LEN, MAC_LEN, MIN_PACKET_LEN, OFF_EXTLEN, OFF_EXTMASK, OFF_FIDHANDLE,
    OFF_PAD, OFF_SEQ, OFF_SIZE, OFF_TAG, OFF_TYPE,
};

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
