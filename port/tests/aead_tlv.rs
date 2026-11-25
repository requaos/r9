//! The TLV pin: the six `ExtMask` records, every mask combination and the
//! zero-allocation parse (WIRE-02; D-11, D-12, D-27; P1).
//!
//! Every expected wire byte is written by hand from the P1 record format,
//! independently of the encoder under test.  A counting global allocator
//! proves `parse` borrows the area and allocates nothing.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use port::chan::tlv::{
    self, Exts, QidVolatility, LEN_CAP_TOKEN, LEN_DEADLINE, LEN_OBINARY, LEN_QID_VOLATILITY,
    LEN_STATELESS_CAP, LEN_STREAM_PRIO, TLV_HEADER_LEN,
};
use port::chan::{
    EXT_CAP_TOKEN, EXT_DEADLINE, EXT_OBINARY, EXT_QID_VOLATILITY, EXT_STATELESS_CAP,
    EXT_STREAM_PRIO,
};

// ---------------------------------------------------------------------------
// A counting global allocator: per-thread, so parallel tests cannot pollute
// the count.  `parse` must leave the count unchanged (WIRE-02, zero-copy).
// ---------------------------------------------------------------------------

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn bump() {
    let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
}

fn allocations() -> usize {
    ALLOCATIONS.try_with(Cell::get).unwrap_or(0)
}

struct CountingAllocator;

// SAFETY: every method forwards to `System`, the allocator this wrapper
// replaces, with the same layout and pointer contract; the counter is a
// thread-local `Cell`, which never allocates.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        bump();
        // SAFETY: `layout` is forwarded unchanged to `System::alloc`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr`/`layout` are the pair `alloc` returned to us.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        bump();
        // SAFETY: `ptr`/`layout` come from this allocator and `new_size` is
        // forwarded unchanged to `System::realloc`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        bump();
        // SAFETY: `layout` is forwarded unchanged to `System::alloc_zeroed`.
        unsafe { System.alloc_zeroed(layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// ---------------------------------------------------------------------------
// Hand-written record vectors (P1).  Each is a distinct record with the value
// bytes spelled out, so a swapped field cannot pass.
// ---------------------------------------------------------------------------

/// `EXT_DEADLINE`: type 0, value 0x0102030405060708 little-endian, padded to 16.
const DEADLINE_RECORD: [u8; 16] = [
    0x00, 0x00, 0x08, 0x00, // type 0, reserved 0, len 8 LE
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // value = 0x0102030405060708
    0x00, 0x00, 0x00, 0x00, // padding to 16
];

/// `EXT_CAP_TOKEN`: type 1, 32 value bytes 0x00..=0x1f, padded to 40.
const CAP_TOKEN_RECORD: [u8; 40] = [
    0x01, 0x00, 0x20, 0x00, // type 1, reserved 0, len 32 LE
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, // value 0x00..=0x07
    0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, // value 0x08..=0x0f
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, // value 0x10..=0x17
    0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, // value 0x18..=0x1f
    0x00, 0x00, 0x00, 0x00, // padding to 40
];

/// `EXT_QID_VOLATILITY`: type 2, class 0x40, lease gen 0x0102030405060708,
/// mode 2, padded to 16.
const QID_VOLATILITY_RECORD: [u8; 16] = [
    0x02, 0x00, 0x0a, 0x00, // type 2, reserved 0, len 10 LE
    0x40, // class = QTLEASED
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // lease_gen = 0x0102030405060708
    0x02, // granted_mode = exclusive write
    0x00, 0x00, // padding to 16
];

/// `EXT_STREAM_PRIO`: type 3, value 0x80001234 (tier 2, credit 0x1234).
const STREAM_PRIO_RECORD: [u8; 8] = [
    0x03, 0x00, 0x04, 0x00, // type 3, reserved 0, len 4 LE
    0x34, 0x12, 0x00, 0x80, // value = 0x80001234
];

/// `EXT_OBINARY`: type 4, zero-length value, padded to 8.
const OBINARY_RECORD: [u8; 8] = [
    0x04, 0x00, 0x00, 0x00, // type 4, reserved 0, len 0 LE
    0x00, 0x00, 0x00, 0x00, // padding to 8
];

/// `EXT_STATELESS_CAP`: type 5, 24 value bytes 0xa0..=0xb7, padded to 32.
const STATELESS_CAP_RECORD: [u8; 32] = [
    0x05, 0x00, 0x18, 0x00, // type 5, reserved 0, len 24 LE
    0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, // value 0xa0..=0xa7
    0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf, // value 0xa8..=0xaf
    0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, // value 0xb0..=0xb7
    0x00, 0x00, 0x00, 0x00, // padding to 32
];

/// The 32-byte capability value in [`CAP_TOKEN_RECORD`].
const CAP_TOKEN_VALUE: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
    0x1e, 0x1f,
];

/// The 24-byte capability value in [`STATELESS_CAP_RECORD`].
const STATELESS_CAP_VALUE: [u8; 24] = [
    0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae,
    0xaf, 0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7,
];

/// Build one record from a type index and value, exactly as P1 lays it out.
/// Used only to compose the multi-record areas; the single records above pin
/// the literal bytes.
#[allow(dead_code)] // used by the Task 2 rejection tests added in the next RED step
fn build_record(type_index: u8, value: &[u8]) -> Vec<u8> {
    let mut out = vec![type_index, 0x00];
    out.extend_from_slice(&(value.len() as u16).to_le_bytes());
    out.extend_from_slice(value);
    let padded = (TLV_HEADER_LEN + value.len()).next_multiple_of(8);
    out.resize(padded, 0x00);
    out
}

#[test]
fn aead_tlv_each_extension_roundtrips() {
    // EXT_DEADLINE.
    let exts = tlv::parse(EXT_DEADLINE, &DEADLINE_RECORD).expect("deadline record parses");
    assert_eq!(exts.deadline, Some(0x0102_0304_0506_0708), "deadline decodes little-endian");
    let mut out = Vec::new();
    tlv::encode(&exts, &mut out);
    assert_eq!(out.as_slice(), DEADLINE_RECORD, "deadline re-encodes to its hand-written bytes");
    assert_eq!(out.len(), 16, "the deadline record is 16 padded bytes");

    // EXT_CAP_TOKEN (32-byte value, borrowed in place).
    let exts = tlv::parse(EXT_CAP_TOKEN, &CAP_TOKEN_RECORD).expect("cap token record parses");
    assert_eq!(exts.cap_token, Some(&CAP_TOKEN_VALUE), "the 32-byte token decodes");
    assert_eq!(
        exts.cap_token.unwrap().as_ptr(),
        CAP_TOKEN_RECORD[TLV_HEADER_LEN..].as_ptr(),
        "the token is borrowed from the parsed area, not copied"
    );
    let mut out = Vec::new();
    tlv::encode(&exts, &mut out);
    assert_eq!(out.as_slice(), CAP_TOKEN_RECORD, "cap token re-encodes to its hand-written bytes");
    assert_eq!(out.len(), 40, "the cap token record is 40 padded bytes");

    // EXT_QID_VOLATILITY.
    let exts =
        tlv::parse(EXT_QID_VOLATILITY, &QID_VOLATILITY_RECORD).expect("volatility record parses");
    assert_eq!(
        exts.qid_volatility,
        Some(QidVolatility {
            class: 0x40,
            lease_gen: 0x0102_0304_0506_0708,
            granted_mode: 2,
        }),
        "the volatility class record decodes"
    );
    let mut out = Vec::new();
    tlv::encode(&exts, &mut out);
    assert_eq!(
        out.as_slice(),
        QID_VOLATILITY_RECORD,
        "volatility re-encodes to its hand-written bytes"
    );
    assert_eq!(out.len(), 16, "the volatility record is 16 padded bytes");

    // EXT_STREAM_PRIO.
    let exts = tlv::parse(EXT_STREAM_PRIO, &STREAM_PRIO_RECORD).expect("stream prio record parses");
    assert_eq!(exts.stream_prio, Some(0x8000_1234), "the stream priority decodes");
    let mut out = Vec::new();
    tlv::encode(&exts, &mut out);
    assert_eq!(
        out.as_slice(),
        STREAM_PRIO_RECORD,
        "stream priority re-encodes to its hand-written bytes"
    );
    assert_eq!(out.len(), 8, "the stream priority record is 8 bytes");

    // EXT_OBINARY: zero-length value, still an 8-byte record.
    let exts = tlv::parse(EXT_OBINARY, &OBINARY_RECORD).expect("obinary record parses");
    assert!(exts.obinary, "the obinary marker decodes");
    let mut out = Vec::new();
    tlv::encode(&exts, &mut out);
    assert_eq!(out.as_slice(), OBINARY_RECORD, "obinary re-encodes to its hand-written bytes");
    assert_eq!(out.len(), 8, "the zero-length obinary record is still 8 bytes");

    // EXT_STATELESS_CAP (24-byte value, borrowed in place).
    let exts =
        tlv::parse(EXT_STATELESS_CAP, &STATELESS_CAP_RECORD).expect("stateless cap record parses");
    assert_eq!(exts.stateless_cap, Some(&STATELESS_CAP_VALUE), "the 24-byte capability decodes");
    assert_eq!(
        exts.stateless_cap.unwrap().as_ptr(),
        STATELESS_CAP_RECORD[TLV_HEADER_LEN..].as_ptr(),
        "the capability is borrowed from the parsed area, not copied"
    );
    let mut out = Vec::new();
    tlv::encode(&exts, &mut out);
    assert_eq!(
        out.as_slice(),
        STATELESS_CAP_RECORD,
        "stateless cap re-encodes to its hand-written bytes"
    );
    assert_eq!(out.len(), 32, "the stateless cap record is 32 padded bytes");
}

#[test]
fn aead_tlv_all_64_masks_roundtrip() {
    const RECORD_LENS: [usize; 6] = [16, 40, 16, 8, 8, 32];
    let cap = CAP_TOKEN_VALUE;
    let stateless = STATELESS_CAP_VALUE;

    for mask in 0u16..64 {
        let exts = Exts {
            deadline: (mask & EXT_DEADLINE != 0).then_some(0x0102_0304_0506_0708),
            cap_token: (mask & EXT_CAP_TOKEN != 0).then_some(&cap),
            qid_volatility: (mask & EXT_QID_VOLATILITY != 0).then_some(QidVolatility {
                class: 0x40,
                lease_gen: 0x0102_0304_0506_0708,
                granted_mode: 2,
            }),
            stream_prio: (mask & EXT_STREAM_PRIO != 0).then_some(0x8000_1234),
            obinary: mask & EXT_OBINARY != 0,
            stateless_cap: (mask & EXT_STATELESS_CAP != 0).then_some(&stateless),
        };
        assert_eq!(exts.mask(), mask, "mask() is the OR of the set fields");

        let expected_len: usize = (0..6)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| RECORD_LENS[i])
            .sum();
        assert_eq!(exts.encoded_len(), expected_len, "encoded_len is the sum of the record lengths");

        let mut area = Vec::new();
        tlv::encode(&exts, &mut area);
        assert_eq!(area.len(), expected_len, "encode writes exactly encoded_len bytes for mask {mask:#06x}");

        let back = tlv::parse(mask, &area).expect("an encoded area parses");
        assert_eq!(back, exts, "mask {mask:#06x} round-trips through encode and parse");
    }
}

#[test]
fn aead_tlv_mask_zero_is_empty_area() {
    let mut area = Vec::new();
    tlv::encode(&Exts::default(), &mut area);
    assert!(area.is_empty(), "an empty Exts encodes to a zero-length area");

    let exts = tlv::parse(0, &[]).expect("mask zero with an empty area parses");
    assert_eq!(exts, Exts::default(), "mask zero sets nothing");
}

#[test]
fn aead_tlv_parse_allocates_nothing() {
    let mut area = Vec::new();
    area.extend_from_slice(&DEADLINE_RECORD);
    area.extend_from_slice(&CAP_TOKEN_RECORD);
    area.extend_from_slice(&QID_VOLATILITY_RECORD);
    area.extend_from_slice(&STREAM_PRIO_RECORD);
    area.extend_from_slice(&OBINARY_RECORD);
    area.extend_from_slice(&STATELESS_CAP_RECORD);

    let before = allocations();
    let exts = tlv::parse(0x003f, &area).expect("the six-record area parses");
    let after = allocations();

    assert_eq!(after, before, "parse allocates nothing (zero-copy borrow)");
    assert_eq!(exts.mask(), 0x003f, "all six extensions are set");
    assert!(exts.deadline.is_some(), "deadline parsed");
    assert!(exts.cap_token.is_some(), "cap token parsed");
    assert!(exts.qid_volatility.is_some(), "volatility parsed");
    assert!(exts.stream_prio.is_some(), "stream priority parsed");
    assert!(exts.obinary, "obinary parsed");
    assert!(exts.stateless_cap.is_some(), "stateless capability parsed");
}

#[test]
fn aead_tlv_stream_prio_is_two_plus_thirty() {
    assert_eq!(tlv::stream_prio_parts(0x8000_1234), (2, 0x0000_1234), "tier is the top 2 bits");
    assert_eq!(tlv::stream_prio_parts(0x4000_0000), (1, 0), "tier 1 with zero credit");
    assert_eq!(tlv::stream_prio_parts(0x3fff_ffff), (0, 0x3fff_ffff), "tier 0 with full credit");
    assert_eq!(tlv::stream_prio_parts(0xffff_ffff), (3, 0x3fff_ffff), "top tier and credit");
}

// Silence the unused-constant lint for the fixed value lengths: the round-trip
// test above asserts the padded lengths, and these keep the contract named.
#[allow(dead_code)]
const _: [usize; 6] = [
    LEN_DEADLINE,
    LEN_CAP_TOKEN,
    LEN_QID_VOLATILITY,
    LEN_STREAM_PRIO,
    LEN_OBINARY,
    LEN_STATELESS_CAP,
];
