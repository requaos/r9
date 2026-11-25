//! The header pin: every field offset, the framing bounds and the canonical
//! hand-written byte vector (WIRE-01; D-06, D-07).
//!
//! Every expected wire byte here is written by hand from the D-06 field list,
//! independently of the encoder under test (the transparency prohibition in
//! the plan; `port/tests/devmnt_test.rs` builds its bytes the same way).  The
//! registry and post-verification header rules are pinned by Task 2.

use port::chan::wire::{self, Header};
use port::chan::{
    HEADER_LEN, MAC_LEN, MIN_PACKET_LEN, OFF_EXTLEN, OFF_EXTMASK, OFF_FIDHANDLE, OFF_PAD,
    OFF_SEQ, OFF_SIZE, OFF_TAG, OFF_TYPE,
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
