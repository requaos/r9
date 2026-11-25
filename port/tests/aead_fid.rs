//! The layout pins for the `9P2000.aead` hybrid file handle (WIRE-07,
//! WIRE-10; D-11, D-12, D-14, D-27).
//!
//! Every wire vector below is a literal byte array written out by hand,
//! independently of the codec under test.  The 13-byte qid order and the three
//! volatility-class bytes are checked against Harvey `sys/include/libc.h`
//! (`QTDIR 0x80`, `QTAPPEND 0x40`, `QTEXCL 0x20`).

use crypto::key::Key;
use ds::tpht::TinyPtr;
use ds::tpht::Probe;
use port::dat::Qid;
use port::fid::{
    FID_LIVE, STATEFUL_HANDLE_LEN, STATELESS_CAP_LEN, FidEntry, FidHandle, FidTable, StatelessCap,
    StatefulHandle, VolatilityClass, gpu_qid_path, qid_decode, qid_encode,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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

// ---------------------------------------------------------------------------
// The table pins (WIRE-08, WIRE-09)
// ---------------------------------------------------------------------------

/// A [`Probe`] that counts the reader's two memory accesses, so a resolve can
/// be shown to make exactly one arena read and zero header reads (DS-03,
/// WIRE-08).  The counters sit behind `Arc` so every shard holds a clone and
/// the test keeps a handle to read them; `ds`'s own `CountingProbe` is private
/// to its test module, so this one is defined against the public `Probe` trait.
#[derive(Clone, Default)]
struct CountingProbe {
    header_reads: Arc<AtomicUsize>,
    arena_reads: Arc<AtomicUsize>,
}

impl CountingProbe {
    fn new() -> CountingProbe {
        CountingProbe::default()
    }

    fn header_reads(&self) -> usize {
        self.header_reads.load(Ordering::Relaxed)
    }

    fn arena_reads(&self) -> usize {
        self.arena_reads.load(Ordering::Relaxed)
    }

    fn reset(&self) {
        self.header_reads.store(0, Ordering::Relaxed);
        self.arena_reads.store(0, Ordering::Relaxed);
    }
}

impl Probe for CountingProbe {
    fn header_read(&self) {
        self.header_reads.fetch_add(1, Ordering::Relaxed);
    }

    fn arena_read(&self) {
        self.arena_reads.fetch_add(1, Ordering::Relaxed);
    }
}

/// A fixed 32-byte domain key: `seed` then `seed + 1`, ... so two calls with
/// the same seed give the same key and two different seeds give different keys.
fn domain_key(seed: u8) -> Key {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = seed.wrapping_add(i as u8);
    }
    Key::from_bytes(bytes)
}

/// A valid entry for one fid number; every field is distinct enough to notice a
/// mix-up, and `flags` starts at zero so `insert` sets [`FID_LIVE`] itself.
fn entry_for(fid_key: u32) -> FidEntry {
    FidEntry {
        fid_key,
        client_id: 0,
        qid_path: 0x1000 + fid_key as u64,
        offset: 0,
        qid_vers: 1,
        iounit: 8192,
        lease_gen: 0,
        ring_slot: 0,
        dev_handler: 0,
        epoch: 0,
        qid_type: 0,
        open_mode: 0x02,
        flags: 0,
        volatility: 0,
        _reserved: [0u8; 15],
    }
}

#[test]
fn aead_fid_resolve_is_one_arena_read() {
    let probe = CountingProbe::new();
    let table = FidTable::with_probe(8, 1, domain_key(0x11), 7, probe.clone())
        .expect("eight one-block shards form a table");
    let mut handles = Vec::new();
    for fid in 1..=32u32 {
        handles.push(table.insert(1, entry_for(fid)).expect("a fresh table has room for 32 fids"));
    }

    probe.reset();
    let handle = handles[0];
    let resolved = table.resolve(1, &handle);
    assert!(resolved.is_ok(), "a live handle must resolve, got {:?}", resolved.as_ref().err());
    let resolved = resolved.expect("the assertion above guarantees a resolved entry");
    assert_eq!(resolved.fid_key, handle.fid_key, "the resolved entry is the inserted fid");
    assert_eq!(resolved.qid_path, entry_for(1).qid_path, "the resolved entry is the inserted entry");
    assert_eq!(
        (probe.arena_reads(), probe.header_reads()),
        (1, 0),
        "a resolve makes exactly one arena read and zero header reads (WIRE-08, DS-03)"
    );
}

#[test]
fn aead_fid_resolve_rejects_wrong_client_epoch_and_shard() {
    let probe = CountingProbe::new();
    let mut table = FidTable::with_probe(2, 1, domain_key(0x22), 7, probe.clone())
        .expect("two one-block shards form a table");
    let handle = table.insert(1, entry_for(9)).expect("a fresh table has room for one fid");
    assert!(
        table.resolve(1, &handle).is_ok(),
        "the handle resolves for the client that inserted it"
    );
    assert!(
        table.resolve(2, &handle).is_err(),
        "another client's id does not resolve it, even with the same fid number (T-04-32)"
    );

    let mut wrong_epoch = handle;
    wrong_epoch.epoch = handle.epoch.wrapping_add(1);
    assert!(
        table.resolve(1, &wrong_epoch).is_err(),
        "a handle from another epoch does not resolve (WIRE-08)"
    );

    let mut bad_shard = handle;
    bad_shard.shard_id = 2; // the shard count, so one past the end
    probe.reset();
    assert!(
        table.resolve(1, &bad_shard).is_err(),
        "a handle whose shard id is out of range is rejected (T-04-35)"
    );
    assert_eq!(
        (probe.arena_reads(), probe.header_reads()),
        (0, 0),
        "an out-of-range shard id is rejected before any arena or header read (T-04-35)"
    );

    table.set_epoch(handle.epoch.wrapping_add(1));
    assert!(
        table.resolve(1, &handle).is_err(),
        "bumping the table's epoch revokes every handle issued before it"
    );
}

#[test]
fn aead_fid_resolve_clunked_handle_does_not_resolve() {
    let table = FidTable::with_probe(1, 1, domain_key(0x33), 3, CountingProbe::new())
        .expect("one one-block shard forms a table");
    let handle = table.insert(1, entry_for(4)).expect("a fresh table has room for one fid");
    assert!(table.resolve(1, &handle).is_ok(), "the live handle resolves before the clunk");

    table.clunk(1, &handle).expect("clunking a live handle succeeds");
    assert!(
        table.resolve(1, &handle).is_err(),
        "a clunked handle no longer resolves: its arena slot was zeroed (T-04-33)"
    );

    let other = table.insert(1, entry_for(5)).expect("the shard has room for another fid");
    assert!(table.resolve(1, &other).is_ok(), "the new fid resolves");
    assert!(
        table.resolve(1, &handle).is_err(),
        "the stale handle stays refused even after the slot may have been reused"
    );
}

#[test]
fn aead_fid_resolve_two_clients_same_fid_number() {
    let table = FidTable::with_probe(1, 1, domain_key(0x34), 1, CountingProbe::new())
        .expect("one one-block shard forms a table");
    let client_one = table.insert(1, entry_for(7)).expect("client 1 stores fid 7");
    let client_two = table.insert(2, entry_for(7)).expect("client 2 stores its own fid 7");

    assert!(table.resolve(1, &client_one).is_ok(), "client 1 resolves its own handle");
    assert!(table.resolve(2, &client_two).is_ok(), "client 2 resolves its own handle");
    assert!(
        table.resolve(2, &client_one).is_err(),
        "client 2 cannot resolve client 1's handle for the same fid number"
    );
    assert!(
        table.resolve(1, &client_two).is_err(),
        "client 1 cannot resolve client 2's handle for the same fid number"
    );
}

#[test]
fn aead_fid_resolve_falls_over_to_the_next_shard_when_home_is_full() {
    let table = FidTable::with_probe(2, 1, domain_key(0x44), 1, CountingProbe::new())
        .expect("two one-block shards form a table");

    // Sixteen distinct keys whose home shard is 0 fill its one 16-slot block.
    let mut home_zero = Vec::new();
    let mut fid = 1u32;
    while home_zero.len() < 16 {
        if table.home_shard(1, fid) == 0 {
            home_zero.push(fid);
        }
        fid += 1;
    }
    for &key in &home_zero {
        let handle = table.insert(1, entry_for(key)).expect("home shard 0 has room for 16 fids");
        assert_eq!(handle.shard_id, 0, "a key lands in its home shard while the block has room");
    }

    // The next key that would home to shard 0 must fall over to shard 1.
    let mut overflow_key = fid;
    while table.home_shard(1, overflow_key) != 0 {
        overflow_key += 1;
    }
    let fell = table
        .insert(1, entry_for(overflow_key))
        .expect("a full home shard falls over instead of failing");
    assert_eq!(fell.shard_id, 1, "the fallen-over handle records the shard it landed in (WIRE-08)");
    assert!(table.resolve(1, &fell).is_ok(), "a fallen-over handle resolves");

    // Fill shard 1 to its 16 slots too, then a further home-1 key must fail.
    let mut shard_one = Vec::new();
    let mut key = overflow_key + 1;
    while shard_one.len() < 15 {
        if table.home_shard(1, key) == 1 {
            shard_one.push(key);
        }
        key += 1;
    }
    for &home_one in &shard_one {
        table.insert(1, entry_for(home_one)).expect("shard 1 has room for 15 more fids");
    }
    let mut doomed = key;
    while table.home_shard(1, doomed) != 1 {
        doomed += 1;
    }
    assert!(
        table.insert(1, entry_for(doomed)).is_err(),
        "with every shard full an insert is an error, never a silent drop"
    );
}

