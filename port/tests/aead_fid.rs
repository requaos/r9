//! The layout pins for the `9P2000.aead` hybrid file handle (WIRE-07,
//! WIRE-10; D-11, D-12, D-14, D-27).
//!
//! Every wire vector below is a literal byte array written out by hand,
//! independently of the codec under test.  The 13-byte qid order and the three
//! volatility-class bytes are checked against Harvey `sys/include/libc.h`
//! (`QTDIR 0x80`, `QTAPPEND 0x40`, `QTEXCL 0x20`).

use ds::tpht::TinyPtr;
use port::dat::Qid;
use port::fid::{
    FID_LIVE, STATEFUL_HANDLE_LEN, STATELESS_CAP_LEN, FidEntry, FidHandle, StatelessCap,
    StatefulHandle, VolatilityClass, gpu_qid_path, qid_decode, qid_encode,
};

/// A boundary-value entry: every field at its maximum, plus the volatile class
/// byte (D-27).
fn boundary_entry() -> FidEntry {
    FidEntry {
        fid_key: u32::MAX,
        client_id: u32::MAX,
        qid_path: u64::MAX,
        offset: u64::MAX,
        qid_vers: u32::MAX,
        iounit: u32::MAX,
        lease_gen: u32::MAX,
        ring_slot: u32::MAX,
        dev_handler: u16::MAX,
        epoch: u16::MAX,
        qid_type: 0xFF,
        open_mode: 0xFF,
        flags: u16::MAX,
        volatility: 0x80,
        _reserved: [0xFF; 15],
    }
}

#[test]
fn aead_fid_layout_sizes_and_alignment() {
    assert_eq!(STATEFUL_HANDLE_LEN, 8, "a stateful handle is 8 bytes on the wire");
    assert_eq!(STATELESS_CAP_LEN, 24, "a stateless capability is 24 bytes on the wire (D-14)");
    assert_eq!(core::mem::size_of::<FidEntry>(), 64, "a FidEntry is one 64-byte cache line");
    assert_eq!(core::mem::align_of::<FidEntry>(), 64, "a FidEntry is 64-byte aligned");
    assert_eq!(
        core::mem::size_of::<StatefulHandle>(),
        STATEFUL_HANDLE_LEN,
        "the in-memory handle is exactly its wire form"
    );
    assert_eq!(FID_LIVE, 1, "the live bit is flags bit 0");
}

#[test]
fn aead_fid_layout_wire_forms_roundtrip() {
    // fid_key 0x11223344, ptr 0x2A, shard_id 0x07, epoch 0x1234 — LE fields.
    let handle_bytes: [u8; STATEFUL_HANDLE_LEN] =
        [0x44, 0x33, 0x22, 0x11, 0x2A, 0x07, 0x34, 0x12];
    let handle = StatefulHandle::from_wire(&handle_bytes).expect("a hand-written handle decodes");
    assert_eq!(handle.fid_key, 0x1122_3344, "fid_key is little-endian at offset 0");
    assert_eq!(handle.ptr.get(), 0x2A, "ptr is one byte at offset 4");
    assert_eq!(handle.shard_id, 0x07, "shard_id is one byte at offset 5");
    assert_eq!(handle.epoch, 0x1234, "epoch is little-endian at offset 6");
    assert_eq!(handle.to_wire(), handle_bytes, "a handle encodes back to the same bytes");

    // qid_path 0x0102030405060708, qid_vers 0xAABBCCDD, qid_type 0x80,
    // mode 0x02, epoch 0x1357, tag DE AD BE EF 01 02 03 04.
    let cap_bytes: [u8; STATELESS_CAP_LEN] = [
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // qid_path
        0xDD, 0xCC, 0xBB, 0xAA, // qid_vers
        0x80, // qid_type
        0x02, // mode
        0x57, 0x13, // epoch
        0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04, // mac_tag
    ];
    let cap = StatelessCap::from_wire(&cap_bytes);
    assert_eq!(cap.qid_path, 0x0102_0304_0506_0708, "qid_path is little-endian at offset 0");
    assert_eq!(cap.qid_vers, 0xAABB_CCDD, "qid_vers is little-endian at offset 8");
    assert_eq!(cap.qid_type, 0x80, "qid_type is one byte at offset 12");
    assert_eq!(cap.mode, 0x02, "mode is one byte at offset 13");
    assert_eq!(cap.epoch, 0x1357, "epoch is little-endian at offset 14");
    assert_eq!(
        cap.mac_tag,
        [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04],
        "the tag is the last 8 bytes at offset 16"
    );
    assert_eq!(cap.to_wire(), cap_bytes, "a capability encodes back to the same bytes");
    assert_eq!(
        cap.body(),
        [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0xDD, 0xCC, 0xBB, 0xAA, 0x80, 0x02, 0x57,
         0x13],
        "the tagged body is the first 16 wire bytes"
    );

    let entry = boundary_entry();
    assert_eq!(
        FidEntry::from_words(&entry.to_words()),
        entry,
        "a FidEntry round-trips through its eight words at boundary values"
    );
    let zero = FidEntry::from_words(&[0u64; 8]);
    assert_eq!(FidEntry::from_words(&zero.to_words()), zero, "the zero entry round-trips too");
}

#[test]
fn aead_fid_layout_stateful_handle_is_never_zero() {
    assert!(
        StatefulHandle::from_wire(&[0u8; STATEFUL_HANDLE_LEN]).is_err(),
        "eight zero bytes are never a stateful handle (D-12)"
    );
    // Every encodable handle carries a non-zero byte at offset 4 (the TinyPtr).
    for raw in [0x01u8, 0x2A, 0x10, 0xFF] {
        let ptr = TinyPtr::new(raw).expect("a non-zero byte is a tiny pointer");
        let handle = StatefulHandle { fid_key: 0, ptr, shard_id: 0, epoch: 0 };
        assert_ne!(handle.to_wire()[4], 0, "ptr {raw:#04x} must be non-zero at offset 4");
    }
}

#[test]
fn aead_fid_layout_capability_requires_zero_header_handle() {
    let cap_bytes: [u8; STATELESS_CAP_LEN] = [0x5A; STATELESS_CAP_LEN];
    let zero_header = [0u8; STATEFUL_HANDLE_LEN];

    assert!(
        FidHandle::from_wire(&[0x01; STATEFUL_HANDLE_LEN], Some(&cap_bytes)).is_err(),
        "a capability presented with a non-zero header field is rejected (D-12)"
    );
    assert!(
        matches!(FidHandle::from_wire(&zero_header, Some(&cap_bytes)), Ok(FidHandle::Stateless(_))),
        "a capability with a zero header field is Stateless"
    );
    assert!(
        matches!(FidHandle::from_wire(&[0x44, 0x33, 0x22, 0x11, 0x2A, 0x07, 0x34, 0x12], None), Ok(FidHandle::Stateful(_))),
        "no capability is Stateful"
    );
    assert!(
        FidHandle::from_wire(&zero_header, None).is_err(),
        "no capability and a zero header field is neither kind, so it is an error"
    );
}

#[test]
fn aead_fid_layout_identity_is_word_zero() {
    let entry = FidEntry {
        fid_key: 0x1122_3344,
        client_id: 0x5566_7788,
        ..FidEntry::from_words(&[0u64; 8])
    };
    assert_eq!(
        entry.to_words()[0],
        (0x5566_7788u64 << 32) | 0x1122_3344u64,
        "word 0 holds fid_key in its low half and client_id in its high half"
    );
    let rebuilt = FidEntry::from_words(&entry.to_words());
    assert_eq!(
        (rebuilt.fid_key, rebuilt.client_id),
        (0x1122_3344, 0x5566_7788),
        "the identity pair survives the word form unchanged"
    );
}

#[test]
fn aead_fid_qid_13_bytes_roundtrip() {
    let qid = Qid { path: 0x0102_0304_0506_0708, vers: 0xAABB_CCDD, typ: 0x80 };
    let bytes = qid_encode(&qid);
    assert_eq!(
        bytes,
        [
            0x80, // type[1]
            0xDD, 0xCC, 0xBB, 0xAA, // vers[4], little-endian
            0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // path[8], little-endian
        ],
        "the qid is type[1] vers[4] path[8], the convM2S order devmnt reads"
    );
    let back = qid_decode(&bytes);
    assert_eq!((back.typ, back.vers, back.path), (0x80, 0xAABB_CCDD, 0x0102_0304_0506_0708));
    assert_eq!(qid_decode(&qid_encode(&qid)).typ, qid.typ, "qid_decode inverts qid_encode");
}

#[test]
fn aead_fid_qid_volatility_class_values() {
    assert_eq!(VolatilityClass::from_byte(0x20), Ok(VolatilityClass::Immutable));
    assert_eq!(VolatilityClass::from_byte(0x40), Ok(VolatilityClass::Leased));
    assert_eq!(VolatilityClass::from_byte(0x80), Ok(VolatilityClass::Volatile));
    for rejected in [0x00u8, 0x60, 0xC0, 0xE0, 0x01] {
        assert!(
            VolatilityClass::from_byte(rejected).is_err(),
            "{rejected:#04x} is not one of the three class bytes (D-27)"
        );
    }
    assert_eq!(VolatilityClass::Immutable.as_byte(), 0x20);
    assert_eq!(VolatilityClass::Leased.as_byte(), 0x40);
    assert_eq!(VolatilityClass::Volatile.as_byte(), 0x80);
}

#[test]
fn aead_fid_qid_class_values_collide_with_plan9_type_bits() {
    // Harvey sys/include/libc.h:591-593: QTDIR 0x80, QTAPPEND 0x40,
    // QTEXCL 0x20.  The three class bytes are the same numbers, which is why
    // the class has its own byte and is never written into a Qid type byte
    // (D-27, T-04-30).
    assert_eq!(VolatilityClass::Immutable.as_byte(), 0x20, "QTIMMUTABLE collides with QTEXCL");
    assert_eq!(VolatilityClass::Leased.as_byte(), 0x40, "QTLEASED collides with QTAPPEND");
    assert_eq!(VolatilityClass::Volatile.as_byte(), 0x80, "QTVOLATILE collides with QTDIR");
    assert_eq!(VolatilityClass::Volatile.as_byte(), port::dat::QTDIR, "the directory bit");
    let mut qid = Qid { path: 7, vers: 1, typ: VolatilityClass::Volatile.as_byte() };
    assert_eq!(qid.typ & port::dat::QTDIR, port::dat::QTDIR, "a class in the type byte reads as a directory");
    qid.typ = 0;
    assert_eq!(qid_encode(&qid)[0], 0x00, "a plain file's type byte is QTFILE");
}

#[test]
fn aead_fid_qid_gpu_path_formula() {
    assert_eq!(gpu_qid_path(0, 0), 0, "the all-zero context and node type give 0");
    assert_eq!(gpu_qid_path(1, 1), 0x101, "ctx 1 and type 1 give 0x101");
    assert_eq!(
        gpu_qid_path(0x00FF_FFFF, 0xFF),
        0xFFFF_FFFF,
        "the widest ctx_id and node type fill 32 bits"
    );
    assert_eq!(
        gpu_qid_path(0xFFFF_FFFF, 0xFF),
        0xFFFF_FFFF,
        "ctx_id above 24 bits is masked to its low 24 bits"
    );
}
