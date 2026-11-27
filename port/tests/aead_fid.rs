//! The layout pins for the `9P2000.aead` hybrid file handle and the sharded
//! generational slab `FidTable` (WIRE-07 to WIRE-10 as amended by Phase 5
//! decisions D-07 to D-13; CORR-03).
//!
//! Every wire vector below is a literal byte array written out by hand,
//! independently of the codec under test.  The 13-byte qid order and the three
//! volatility-class bytes are checked against Harvey `sys/include/libc.h`
//! (`QTDIR 0x80`, `QTAPPEND 0x40`, `QTEXCL 0x20`).

use crypto::key::Key;
use ds::slab::SlabProbe;
use port::dat::Qid;
use port::fid::{
    FidEntry, FidHandle, FidTable, STATEFUL_HANDLE_LEN, STATELESS_CAP_LEN, StatelessCap,
    StatefulHandle, VolatilityClass, gpu_qid_path, qid_decode, qid_encode,
};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

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
        _reserved: [0xFF; 7],
    }
}

#[test]
fn aead_fid_layout_sizes_and_alignment() {
    assert_eq!(STATEFUL_HANDLE_LEN, 8, "a stateful handle is 8 bytes on the wire");
    assert_eq!(STATELESS_CAP_LEN, 24, "a stateless capability is 24 bytes on the wire (D-14)");
    assert_eq!(
        core::mem::size_of::<FidEntry>(),
        56,
        "a FidEntry is the seven payload words of one 64-byte slab slot (D-11)"
    );
    assert_eq!(
        core::mem::size_of::<StatefulHandle>(),
        STATEFUL_HANDLE_LEN,
        "the in-memory handle is exactly its wire form"
    );
}

#[test]
fn aead_fid_layout_wire_forms_roundtrip() {
    // slot_idx 0x2233, gen 0x1144, shard_id 0x07, flags 0xA5, epoch 0x1234 —
    // LE fields in the D-10 order.
    let handle_bytes: [u8; STATEFUL_HANDLE_LEN] =
        [0x33, 0x22, 0x44, 0x11, 0x07, 0xA5, 0x34, 0x12];
    let handle = StatefulHandle::from_wire(&handle_bytes).expect("a hand-written handle decodes");
    assert_eq!(handle.slot_idx, 0x2233, "slot_idx is little-endian at offset 0");
    assert_eq!(handle.r#gen, 0x1144, "gen is little-endian at offset 2");
    assert_eq!(handle.shard_id, 0x07, "shard_id is one byte at offset 4");
    assert_eq!(handle.flags, 0xA5, "flags is one byte at offset 5");
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
        "a FidEntry round-trips through its seven words at boundary values"
    );
    let zero = FidEntry::from_words(&[0u64; 7]);
    assert_eq!(FidEntry::from_words(&zero.to_words()), zero, "the zero entry round-trips too");
}

#[test]
fn aead_fid_layout_stateful_handle_is_never_zero() {
    assert!(
        StatefulHandle::from_wire(&[0u8; STATEFUL_HANDLE_LEN]).is_err(),
        "eight zero bytes are never a stateful handle (D-10: gen is never 0)"
    );
    // A zero generation is rejected however the other bytes read: here
    // slot_idx 1, shard 7, epoch 0x1234 with gen 0.
    assert!(
        StatefulHandle::from_wire(&[0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x34, 0x12]).is_err(),
        "a handle with gen == 0 is rejected even when other bytes are non-zero (D-10)"
    );
    // Every encodable handle carries a non-zero generation at offset 2..4, so
    // no stateful handle is ever all zero.
    for generation in [0x0001u16, 0x0100, 0xFFFF] {
        let handle =
            StatefulHandle { slot_idx: 0, r#gen: generation, shard_id: 0, flags: 0, epoch: 0 };
        assert_ne!(
            handle.to_wire()[2..4],
            [0u8; 2],
            "gen {generation:#06x} must be non-zero at offset 2"
        );
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
        matches!(FidHandle::from_wire(&[0x33, 0x22, 0x44, 0x11, 0x07, 0xA5, 0x34, 0x12], None), Ok(FidHandle::Stateful(_))),
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
        ..FidEntry::from_words(&[0u64; 7])
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
// The table pins (CORR-03; Phase 5 D-07 to D-12)
// ---------------------------------------------------------------------------

/// A [`SlabProbe`] that counts `resolve` attempts, so a resolve can be shown
/// to perform exactly one cache-line slot read and zero metadata touches
/// (CORR-03's restatement of the WIRE-08 pin).  The counter sits behind `Arc`
/// so every shard holds a clone and the test keeps a handle to read it;
/// `ds`'s own `CountingSlabProbe` is private to its test module, so this one
/// is defined against the public `SlabProbe` trait.
#[derive(Clone, Default)]
struct CountingSlabProbe {
    reads: Arc<AtomicUsize>,
}

impl CountingSlabProbe {
    fn new() -> CountingSlabProbe {
        CountingSlabProbe::default()
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    fn reset(&self) {
        self.reads.store(0, Ordering::Relaxed);
    }
}

impl SlabProbe for CountingSlabProbe {
    fn slot_read(&self) {
        self.reads.fetch_add(1, Ordering::Relaxed);
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

/// A valid entry for one fid number; every field is distinct enough to notice
/// a mix-up, and `client_id`/`epoch` start at zero so `alloc` sets them.
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
        _reserved: [0u8; 7],
    }
}

#[test]
fn aead_fid_alloc_returns_a_server_assigned_handle() {
    let table = FidTable::with_probe(4, 8, domain_key(0x10), 7, CountingSlabProbe::new())
        .expect("four 8-slot shards form a table");
    let handle = table.alloc(1, entry_for(9)).expect("a fresh table has room for one fid");
    assert!(handle.r#gen >= 1, "the server-assigned generation is never zero (D-10)");
    assert_eq!(handle.shard_id, 0, "alloc routes from CPU 0's shard until port::smp lands (D-12)");
    assert_eq!(handle.epoch, 7, "the handle carries the table's epoch");
    assert_eq!(handle.flags, 0, "a fresh handle carries no flags");

    let resolved = table.resolve(1, &handle).expect("the live handle resolves");
    assert_eq!(resolved.fid_key, 9, "the resolved entry records the client's fid number (D-07)");
    assert_eq!(resolved.client_id, 1, "the resolved entry records its owning client (D-08)");
    assert_eq!(resolved.qid_path, entry_for(9).qid_path, "the resolved entry is the inserted entry");

    // Server-assigned handles mean no duplicate-fid check exists (D-07): the
    // same fid number allocs again into its own slot and both resolve.
    let second = table.alloc(1, entry_for(9)).expect("the same fid number allocs again (D-07)");
    assert_ne!(
        (second.shard_id, second.slot_idx),
        (handle.shard_id, handle.slot_idx),
        "two allocs of one fid number get distinct server-assigned slots (D-07)"
    );
    assert!(table.resolve(1, &handle).is_ok(), "the first handle still resolves");
    assert!(table.resolve(1, &second).is_ok(), "the second handle resolves too");
}

#[test]
fn aead_fid_alloc_routes_cpu_zero_and_spills_over_round_robin() {
    let table = FidTable::with_probe(4, 8, domain_key(0x11), 1, CountingSlabProbe::new())
        .expect("four 8-slot shards form a table");
    assert_eq!(table.shard_count(), 4);
    assert_eq!(table.slots_per_shard(), 8);

    // CPU 0's shard is 0 (`current_cpu_hint() & shard_mask` until port::smp
    // lands, D-12): the first eight allocs fill shard 0 ...
    let mut handles = Vec::new();
    for fid in 1..=8u32 {
        let handle = table.alloc(1, entry_for(fid)).expect("shard 0 has room for eight fids");
        assert_eq!(handle.shard_id, 0, "alloc {fid} routes to CPU 0's shard");
        handles.push(handle);
    }
    // ... and a full shard falls over round-robin — never a hash-clustering
    // failure, because there is no hash (D-07, D-12).
    for fid in 9..=16u32 {
        let handle = table.alloc(1, entry_for(fid)).expect("a full shard falls over to the next");
        assert_eq!(handle.shard_id, 1, "alloc {fid} fell over to shard 1");
        handles.push(handle);
    }
    for fid in 17..=24u32 {
        let handle = table.alloc(1, entry_for(fid)).expect("shard 2 has room");
        assert_eq!(handle.shard_id, 2, "alloc {fid} fell over to shard 2");
        handles.push(handle);
    }
    for fid in 25..=32u32 {
        let handle = table.alloc(1, entry_for(fid)).expect("shard 3 has room");
        assert_eq!(handle.shard_id, 3, "alloc {fid} fell over to shard 3");
        handles.push(handle);
    }
    assert!(
        table.alloc(1, entry_for(33)).is_err(),
        "with every shard full an alloc is an error, never a silent drop"
    );
    for handle in &handles {
        assert!(table.resolve(1, handle).is_ok(), "every spilled-over handle resolves");
    }
}

#[test]
fn aead_fid_sizing_tiers_and_shard_count_for_cpus() {
    let synthetic = FidTable::synthetic(domain_key(0x12), 1);
    assert_eq!(synthetic.shard_count(), 4, "the synthetic tier is 4 shards (D-12)");
    assert_eq!(synthetic.slots_per_shard(), 256, "the synthetic tier is 256 slots (D-12)");

    let workstation = FidTable::default_workstation(domain_key(0x12), 1);
    assert_eq!(workstation.shard_count(), 8, "the workstation tier is 8 shards (D-12)");
    assert_eq!(workstation.slots_per_shard(), 1024, "the workstation tier is 1,024 slots (D-12)");

    let cluster = FidTable::with_tier(4096, 16, domain_key(0x12), 1).expect("the cluster floor tier");
    assert_eq!(cluster.shard_count(), 16);
    assert_eq!(cluster.slots_per_shard(), 4096);
    let cluster_top = FidTable::with_tier(16384, 32, domain_key(0x12), 1).expect("the cluster top tier");
    assert_eq!(cluster_top.shard_count(), 32);
    assert_eq!(cluster_top.slots_per_shard(), 16384);
    assert!(FidTable::with_tier(2048, 16, domain_key(0x12), 1).is_err(), "below the cluster slot floor");
    assert!(FidTable::with_tier(4096, 8, domain_key(0x12), 1).is_err(), "below the cluster shard floor");
    assert!(FidTable::with_tier(4096, 24, domain_key(0x12), 1).is_err(), "shard count must route by mask");

    // `(num_cpus * 2).next_power_of_two().clamp(4, 256)` (D-12).
    assert_eq!(FidTable::shard_count_for_cpus(1), 4);
    assert_eq!(FidTable::shard_count_for_cpus(2), 4);
    assert_eq!(FidTable::shard_count_for_cpus(3), 8);
    assert_eq!(FidTable::shard_count_for_cpus(4), 8);
    assert_eq!(FidTable::shard_count_for_cpus(7), 16);
    assert_eq!(FidTable::shard_count_for_cpus(64), 128);
    assert_eq!(FidTable::shard_count_for_cpus(128), 256, "the clamp's top end");
    assert_eq!(FidTable::shard_count_for_cpus(1000), 256, "clamped to 256 shards");

    // The general constructor validates the power-of-two shapes.
    assert!(FidTable::new(0, 8, domain_key(0x12), 1).is_err(), "zero shards is no table");
    assert!(FidTable::new(3, 8, domain_key(0x12), 1).is_err(), "shards route by mask");
    assert!(FidTable::new(257, 8, domain_key(0x12), 1).is_err(), "shard_id is one byte");
    assert!(FidTable::new(2, 7, domain_key(0x12), 1).is_err(), "slots are a power of two (D-12)");
    assert!(FidTable::new(2, 0, domain_key(0x12), 1).is_err(), "zero slots is no shard");
}

#[test]
fn aead_fid_resolve_is_one_slot_read_and_zero_locks() {
    let probe = CountingSlabProbe::new();
    let table = FidTable::with_probe(2, 8, domain_key(0x13), 5, probe.clone())
        .expect("two 8-slot shards form a table");
    let handle = table.alloc(1, entry_for(1)).expect("a fresh table has room for one fid");

    probe.reset();
    let resolved = table.resolve(1, &handle).expect("a live handle resolves");
    assert_eq!(resolved.fid_key, 1, "the resolved entry is the inserted fid");
    assert_eq!(
        probe.reads(),
        1,
        "a resolve performs exactly one cache-line slot read and acquires no metadata lock \
         (CORR-03's restatement of WIRE-08, DS-03)"
    );

    // Rejections that never reach the slot: out-of-range shard id and
    // out-of-range slot index read nothing.
    let mut bad_shard = handle;
    bad_shard.shard_id = 2; // the shard count, so one past the end
    probe.reset();
    assert!(table.resolve(1, &bad_shard).is_err(), "an out-of-range shard id is rejected");
    assert_eq!(probe.reads(), 0, "an out-of-range shard id is rejected before any slot read");

    let mut bad_slot = handle;
    bad_slot.slot_idx = 8; // the shard's slot count, so one past the end
    probe.reset();
    assert!(table.resolve(1, &bad_slot).is_err(), "an out-of-range slot index is rejected");
    assert_eq!(probe.reads(), 0, "an out-of-range slot index reads no slot");

    // A stale generation reads the slot once and is rejected by the slab.
    table.clunk(1, &handle).expect("clunking a live handle succeeds");
    probe.reset();
    assert!(table.resolve(1, &handle).is_err(), "a clunked handle's generation is stale");
    assert_eq!(probe.reads(), 1, "a stale generation is rejected after one slot read");
}

#[test]
fn aead_fid_resolve_rejects_wrong_client_epoch_and_shard() {
    let mut table = FidTable::with_probe(2, 8, domain_key(0x22), 7, CountingSlabProbe::new())
        .expect("two 8-slot shards form a table");
    let handle = table.alloc(1, entry_for(9)).expect("a fresh table has room for one fid");
    assert!(
        table.resolve(1, &handle).is_ok(),
        "the handle resolves for the client that allocated it"
    );
    assert!(
        table.resolve(2, &handle).is_err(),
        "another client's id does not resolve it, even with the same fid number (D-08, T-05-09)"
    );
    assert!(
        table.resolve(0, &handle).is_err(),
        "client id 0 is reserved and never resolves an entry (D-08)"
    );

    let mut wrong_epoch = handle;
    wrong_epoch.epoch = handle.epoch.wrapping_add(1);
    assert!(
        table.resolve(1, &wrong_epoch).is_err(),
        "a handle from another epoch does not resolve"
    );

    table.set_epoch(handle.epoch.wrapping_add(1));
    assert!(
        table.resolve(1, &handle).is_err(),
        "bumping the table's epoch revokes every handle issued before it"
    );
}

#[test]
fn aead_fid_alloc_refuses_client_id_zero() {
    let probe = CountingSlabProbe::new();
    let table = FidTable::with_probe(2, 8, domain_key(0x68), 3, probe.clone())
        .expect("two 8-slot shards form a table");
    assert!(
        table.alloc(0, entry_for(1)).is_err(),
        "client id 0 is reserved, so alloc refuses it (D-08)"
    );
    for shard in 0..2u8 {
        assert_eq!(
            table.bitmap_lens(shard),
            Some((0, 0)),
            "the refused alloc leaves shard {shard}'s sets empty"
        );
    }

    let handle = table.alloc(1, entry_for(1)).expect("client 1 allocates fid 1");
    probe.reset();
    assert!(table.resolve(0, &handle).is_err(), "resolve refuses client id 0 (D-08)");
    assert!(table.update(0, &handle, |_| {}).is_err(), "update refuses client id 0 (D-08)");
    assert!(table.clunk(0, &handle).is_err(), "clunk refuses client id 0 (D-08)");
    assert!(
        table.mark_reclaimed(0, &handle).is_err(),
        "mark_reclaimed refuses client id 0 (D-08)"
    );
    assert_eq!(probe.reads(), 0, "client id 0 is refused before any slot read");

    // Fid number 0 stays usable by every client.
    let zero_fid = table.alloc(1, entry_for(0)).expect("client 1 may use fid number 0");
    assert_eq!(zero_fid.r#gen >= 1, true, "the server-assigned generation is non-zero");
    let resolved = table.resolve(1, &zero_fid).expect("fid number 0 for client 1 resolves");
    assert_eq!(resolved.fid_key, 0, "the resolved entry records fid number 0");
}

#[test]
fn aead_fid_resolve_two_clients_same_fid_number() {
    let table = FidTable::with_probe(1, 8, domain_key(0x34), 1, CountingSlabProbe::new())
        .expect("one 8-slot shard forms a table");
    let client_one = table.alloc(1, entry_for(7)).expect("client 1 stores fid 7");
    let client_two = table.alloc(2, entry_for(7)).expect("client 2 stores its own fid 7");

    assert!(table.resolve(1, &client_one).is_ok(), "client 1 resolves its own handle");
    assert!(table.resolve(2, &client_two).is_ok(), "client 2 resolves its own handle");
    assert!(
        table.resolve(2, &client_one).is_err(),
        "client 2 cannot resolve client 1's handle for the same fid number (D-08)"
    );
    assert!(
        table.resolve(1, &client_two).is_err(),
        "client 1 cannot resolve client 2's handle for the same fid number (D-08)"
    );
}

#[test]
fn aead_fid_update_keeps_the_wire_handle_and_enforces_the_owner() {
    let table = FidTable::with_probe(2, 8, domain_key(0x40), 9, CountingSlabProbe::new())
        .expect("two 8-slot shards form a table");
    let handle = table.alloc(1, entry_for(4)).expect("a fresh table has room for one fid");

    // An in-place update (the file cursor, the lease generation) keeps the
    // generation, so the client's wire handle stays valid (D-11).
    table
        .update(1, &handle, |entry| {
            entry.offset = 4096;
            entry.lease_gen = 77;
        })
        .expect("the owner updates its live entry");
    let resolved = table.resolve(1, &handle).expect("the same handle still resolves");
    assert_eq!(resolved.offset, 4096, "the update landed");
    assert_eq!(resolved.lease_gen, 77, "the lease generation updated");
    assert_eq!(resolved.fid_key, 4, "the fid record is unchanged");

    // The closure cannot move the entry to another owner or epoch: the table
    // forces the identity back (D-08).
    table
        .update(1, &handle, |entry| {
            entry.client_id = 999;
            entry.epoch = 0;
            entry.offset = 8192;
        })
        .expect("the owner updates its live entry");
    let resolved = table.resolve(1, &handle).expect("the handle still resolves for its owner");
    assert_eq!(resolved.client_id, 1, "the owner cannot be reassigned through update (D-08)");
    assert_eq!(resolved.epoch, 9, "the epoch stays the table's");
    assert_eq!(resolved.offset, 8192, "the payload update still landed");

    // A foreign client cannot update the entry, and the entry is unchanged.
    assert!(
        table.update(2, &handle, |entry| entry.offset = 1).is_err(),
        "another client cannot update the entry (D-08)"
    );
    assert_eq!(
        table.resolve(1, &handle).expect("the owner's handle resolves").offset,
        8192,
        "the refused update changed nothing"
    );

    // After the clunk the stale generation rejects the update.
    table.clunk(1, &handle).expect("clunking a live handle succeeds");
    assert!(
        table.update(1, &handle, |entry| entry.offset = 2).is_err(),
        "a clunked handle rejects the update (stale generation)"
    );
}

/// An entry whose every field derives from `fill`, so two fills differ in
/// every payload word after word 0 and a torn snapshot can pass for neither.
fn filled_entry(fid_key: u32, fill: u64) -> FidEntry {
    FidEntry {
        fid_key,
        client_id: 0,
        qid_path: fill,
        offset: fill.rotate_left(13),
        qid_vers: fill as u32,
        iounit: (fill >> 32) as u32 | 1,
        lease_gen: fill.wrapping_add(1) as u32,
        ring_slot: fill.wrapping_add(2) as u32,
        dev_handler: fill as u16,
        epoch: 0,
        qid_type: (fill >> 8) as u8,
        open_mode: fill as u8,
        flags: (fill >> 16) as u16,
        volatility: fill as u8,
        _reserved: [fill as u8; 7],
    }
}

#[test]
fn aead_fid_update_in_place_never_resolves_a_torn_snapshot() {
    // One writer flips a live entry between two whole-payload images while
    // readers resolve the handle lock-free; every snapshot must be exactly one
    // image or the other (D-11: the seqlock retries, never tears).  The raw
    // word-level stress lives in `ds::slab`; this pins the property through
    // the FidEntry codec and the table's update path.
    const WRITES: usize = 5_000;
    const READS: usize = 2_000;
    let table = FidTable::new(1, 8, domain_key(0x41), 3).expect("one 8-slot shard forms a table");
    let handle = table.alloc(1, filled_entry(7, 0xAAAA_0000_5555_0001)).expect("room for one fid");
    let epoch = handle.epoch;
    let image_a = FidEntry { client_id: 1, epoch, ..filled_entry(7, 0xAAAA_0000_5555_0001) };
    let image_b = FidEntry { client_id: 1, epoch, ..filled_entry(7, 0xBBBB_1111_4444_0002) };
    assert_ne!(image_a, image_b, "the two images differ");

    let barrier = Barrier::new(5);
    std::thread::scope(|scope| {
        let barrier = &barrier;
        let table = &table;
        let writer = scope.spawn(move || {
            barrier.wait();
            for round in 0..WRITES {
                let image = if round % 2 == 0 { image_a } else { image_b };
                table
                    .update(1, &handle, |entry| *entry = image)
                    .expect("the writer owns the live entry");
            }
        });
        let mut readers = Vec::new();
        for _ in 0..4 {
            readers.push(scope.spawn(move || {
                barrier.wait();
                for _ in 0..READS {
                    let resolved = table.resolve(1, &handle).expect("the live handle resolves");
                    assert!(
                        resolved == image_a || resolved == image_b,
                        "a snapshot is one whole image or the other — never torn (D-11)"
                    );
                }
            }));
        }
        writer.join().expect("the writer completes");
        for reader in readers {
            reader.join().expect("every reader completes");
        }
    });
}

#[test]
fn aead_fid_clunk_scrubs_recycles_and_enforces_the_owner() {
    let table = FidTable::with_probe(1, 8, domain_key(0x33), 3, CountingSlabProbe::new())
        .expect("one 8-slot shard forms a table");
    table.alloc(2, entry_for(8)).expect("client 2 has a live entry");
    let handle = table.alloc(1, entry_for(4)).expect("client 1 has a live entry");
    assert!(table.resolve(1, &handle).is_ok(), "the live handle resolves before the clunk");

    // A foreign clunk is refused and the entry survives (D-08).
    assert!(
        table.clunk(2, &handle).is_err(),
        "another client cannot clunk the entry (D-08, T-05-09)"
    );
    assert!(table.resolve(1, &handle).is_ok(), "the refused clunk changed nothing");

    table.clunk(1, &handle).expect("the owner clunks its live handle");
    assert!(
        table.resolve(1, &handle).is_err(),
        "a clunked handle no longer resolves: its slot was scrubbed and its generation bumped (D-11)"
    );
    assert!(
        table.clunk(1, &handle).is_err(),
        "a duplicate clunk of an already-clunked handle is rejected cleanly"
    );

    // The freed slot is the next to allocate, at the bumped generation, and
    // the stale handle stays refused forever.
    let recycled = table.alloc(1, entry_for(5)).expect("the freed slot allocates again");
    assert_eq!(
        (recycled.shard_id, recycled.slot_idx),
        (handle.shard_id, handle.slot_idx),
        "the clunked slot is the one reused"
    );
    assert_eq!(recycled.r#gen, handle.r#gen + 1, "the recycled slot carries the bumped generation (D-10)");
    assert!(table.resolve(1, &recycled).is_ok(), "the recycled handle resolves");
    assert!(
        table.resolve(1, &handle).is_err(),
        "the stale handle stays refused even though the slot is live again (D-10)"
    );
}

#[test]
fn aead_fid_mark_reclaimed_enforces_the_owner() {
    let table = FidTable::with_probe(1, 8, domain_key(0x42), 2, CountingSlabProbe::new())
        .expect("one 8-slot shard forms a table");
    let handle = table.alloc(1, entry_for(6)).expect("client 1 has a live entry");

    assert!(
        table.mark_reclaimed(2, &handle).is_err(),
        "another client cannot mark the entry reclaimed (D-08)"
    );
    assert_eq!(
        table.bitmap_lens(handle.shard_id),
        Some((1, 0)),
        "the refused mark leaves the reclaim window empty"
    );
    table.mark_reclaimed(1, &handle).expect("the owner marks its live entry reclaimed");
    assert_eq!(
        table.bitmap_lens(handle.shard_id),
        Some((1, 1)),
        "the owner's mark lands in the reclaim window"
    );
    assert!(table.sweep_unreclaimed().is_empty(), "the marked entry survives the sweep");
    assert!(table.resolve(1, &handle).is_ok(), "the reclaimed entry still resolves");
}

/// xorshift64: a tiny seeded PRNG so the randomized sweep pin reproduces from
/// its literal seed (no `rand`, no `proptest`).
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> XorShift64 {
        // A zero state would stay zero forever.
        XorShift64(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

/// The `(shard, slot)` a handle names: the sweep model's element.
fn handle_slot(handle: &StatefulHandle) -> (u8, u16) {
    (handle.shard_id, handle.slot_idx)
}

#[test]
fn aead_fid_sweep_matches_set_model_on_random_inputs() {
    for seed in 0..64u64 {
        // Four 16-slot shards: 64 slots, so 40 fids across four clients fit
        // with room for the round-robin spillover.
        let table =
            FidTable::new(4, 16, domain_key(0x60), 0).expect("four 16-slot shards form a table");
        let mut rng = XorShift64::new(0x9E37_79B9_7F4A_7C15 ^ seed);

        let mut fids: BTreeSet<u32> = BTreeSet::new();
        while fids.len() < 40 {
            fids.insert((rng.next() % 4096) as u32 + 1);
        }
        let mut handles: Vec<(u32, StatefulHandle)> = Vec::new();
        let mut model_active: BTreeSet<(u8, u16)> = BTreeSet::new();
        for &fid in &fids {
            let client_id = (rng.next() % 4) as u32 + 1;
            let handle =
                table.alloc(client_id, entry_for(fid)).expect("a fresh table has room for 40 fids");
            model_active.insert(handle_slot(&handle));
            handles.push((client_id, handle));
        }

        let mut model_reclaimed: BTreeSet<(u8, u16)> = BTreeSet::new();
        for (client_id, handle) in &handles {
            if rng.next() % 3 == 0 {
                table.mark_reclaimed(*client_id, handle).expect("a live handle can be marked reclaimed");
                model_reclaimed.insert(handle_slot(handle));
            }
        }

        let swept: BTreeSet<(u8, u16)> = table.sweep_unreclaimed().into_iter().collect();
        let expected: BTreeSet<(u8, u16)> =
            model_active.difference(&model_reclaimed).copied().collect();
        assert_eq!(
            swept, expected,
            "the sweep returns exactly the active set minus the reclaimed set (seed {seed})"
        );
    }
}

#[test]
fn aead_fid_sweep_survivors_resolve_and_orphans_do_not() {
    let table = FidTable::new(4, 16, domain_key(0x61), 0).expect("four 16-slot shards form a table");
    let mut handles = Vec::new();
    for fid in 1..=12u32 {
        handles.push(table.alloc(1, entry_for(fid)).expect("a fresh table has room for 12 fids"));
    }
    // Mark the first six reclaimed; the other six are swept.
    for handle in handles.iter().take(6) {
        table.mark_reclaimed(1, handle).expect("a live handle can be marked reclaimed");
    }
    let swept: BTreeSet<(u8, u16)> = table.sweep_unreclaimed().into_iter().collect();
    for (index, handle) in handles.iter().enumerate() {
        if index < 6 {
            assert!(
                !swept.contains(&handle_slot(handle)),
                "a reclaimed fid is not swept (handle {index})"
            );
            assert!(
                table.resolve(1, handle).is_ok(),
                "a reclaimed fid still resolves to its entry after the sweep (handle {index})"
            );
        } else {
            assert!(
                swept.contains(&handle_slot(handle)),
                "a non-reclaimed active fid is swept (handle {index})"
            );
            assert!(
                table.resolve(1, handle).is_err(),
                "a swept fid no longer resolves (handle {index})"
            );
        }
    }
}

#[test]
fn aead_fid_sweep_edges() {
    // An empty table sweeps to nothing.
    let empty = FidTable::new(2, 8, domain_key(0x62), 0).expect("two 8-slot shards form a table");
    assert!(empty.sweep_unreclaimed().is_empty(), "an empty table sweeps to nothing");

    // With nothing reclaimed, every active fid is swept.
    let all_live = FidTable::new(2, 8, domain_key(0x63), 0).expect("two 8-slot shards form a table");
    let live: Vec<StatefulHandle> =
        (1..=5u32).map(|fid| all_live.alloc(1, entry_for(fid)).expect("room for five fids")).collect();
    assert_eq!(all_live.sweep_unreclaimed().len(), 5, "with nothing reclaimed every active fid is swept");
    for handle in &live {
        assert!(all_live.resolve(1, handle).is_err(), "a swept fid no longer resolves");
    }

    // With everything reclaimed, nothing is swept.
    let all_reclaimed = FidTable::new(2, 8, domain_key(0x64), 0).expect("two 8-slot shards form a table");
    let kept: Vec<StatefulHandle> =
        (1..=5u32).map(|fid| all_reclaimed.alloc(1, entry_for(fid)).expect("room for five fids")).collect();
    for handle in &kept {
        all_reclaimed.mark_reclaimed(1, handle).expect("a live handle can be marked reclaimed");
    }
    assert!(
        all_reclaimed.sweep_unreclaimed().is_empty(),
        "with everything reclaimed nothing is swept"
    );
    for handle in &kept {
        assert!(all_reclaimed.resolve(1, handle).is_ok(), "a reclaimed fid survives the sweep");
    }

    // Marking a slot that was never active changes nothing: the handle names
    // the next slot, which no alloc has used, so resolve refuses it.
    let never = FidTable::new(2, 8, domain_key(0x65), 0).expect("two 8-slot shards form a table");
    let handle = never.alloc(1, entry_for(1)).expect("room for one fid");
    let mut ghost = handle;
    ghost.slot_idx = handle.slot_idx + 1;
    assert!(
        never.mark_reclaimed(1, &ghost).is_err(),
        "a handle to a slot that was never active cannot be marked reclaimed"
    );
    assert_eq!(
        never.bitmap_lens(handle.shard_id),
        Some((1, 0)),
        "the refused mark leaves the live set unchanged and the reclaim window empty"
    );

    // A second sweep straight after the first returns nothing.
    let twice = FidTable::new(2, 8, domain_key(0x66), 0).expect("two 8-slot shards form a table");
    twice.alloc(1, entry_for(1)).expect("room for one fid");
    twice.alloc(1, entry_for(2)).expect("room for two fids");
    assert_eq!(twice.sweep_unreclaimed().len(), 2, "the first sweep returns the live set");
    assert!(twice.sweep_unreclaimed().is_empty(), "a second sweep straight after the first returns nothing");
}

#[test]
fn aead_fid_sweep_resets_the_reclaim_window() {
    let table = FidTable::new(4, 16, domain_key(0x67), 0).expect("four 16-slot shards form a table");
    let mut handles = Vec::new();
    for fid in 1..=8u32 {
        handles.push(table.alloc(1, entry_for(fid)).expect("a fresh table has room for 8 fids"));
    }
    for (index, handle) in handles.iter().enumerate() {
        if index % 2 == 0 {
            table.mark_reclaimed(1, handle).expect("a live handle can be marked reclaimed");
        }
    }
    let swept: BTreeSet<(u8, u16)> = table.sweep_unreclaimed().into_iter().collect();
    assert!(!swept.is_empty(), "there is something to sweep");

    let mut survivors = [0u64; 4];
    for handle in &handles {
        if !swept.contains(&handle_slot(handle)) {
            survivors[handle.shard_id as usize] += 1;
        }
    }
    for shard in 0..4u8 {
        let (active, reclaimed) =
            table.bitmap_lens(shard).expect("the shard index is in range");
        assert_eq!(reclaimed, 0, "the reclaim window is empty after a sweep (shard {shard})");
        assert_eq!(
            active, survivors[shard as usize],
            "active holds exactly the survivors after a sweep (shard {shard})"
        );
    }
}

#[test]
fn aead_fid_sweep_bookkeeping_matches_live_entries_under_churn() {
    let table =
        FidTable::new(4, 32, domain_key(0x6D), 1).expect("four 32-slot shards form a table");
    let table_ref = &table;
    let stop = AtomicUsize::new(0);
    let stop_ref = &stop;

    let inserted: Vec<(u32, StatefulHandle)> = std::thread::scope(|scope| {
        let sweeper = scope.spawn(move || {
            while stop_ref.load(Ordering::SeqCst) == 0 {
                let _ = table_ref.sweep_unreclaimed();
                std::thread::yield_now();
            }
        });
        let mut churn = Vec::new();
        for client in 1..=4u32 {
            churn.push(scope.spawn(move || {
                let mut live: Vec<StatefulHandle> = Vec::new();
                for fid in 1..=12u32 {
                    if let Ok(handle) = table_ref.alloc(client, entry_for(fid)) {
                        live.push(handle);
                    }
                    if fid % 3 == 0
                        && let Some(handle) = live.pop()
                    {
                        // A concurrent sweep may already have collected it.
                        let _ = table_ref.clunk(client, &handle);
                    }
                }
                live
            }));
        }
        let mut collected = Vec::new();
        for handle_thread in churn {
            collected.push(handle_thread.join().expect("a churn thread joins"));
        }
        stop_ref.store(1, Ordering::SeqCst);
        sweeper.join().expect("the sweeper thread joins");
        collected
            .into_iter()
            .enumerate()
            .flat_map(|(index, lives)| lives.into_iter().map(move |handle| (index as u32 + 1, handle)))
            .collect()
    });

    let resolving: u64 =
        inserted.iter().filter(|(client, handle)| table.resolve(*client, handle).is_ok()).count()
            as u64;
    let active: u64 =
        (0..4u8).map(|shard| table.bitmap_lens(shard).expect("shard in range").0).sum();
    assert_eq!(
        active, resolving,
        "after churn the shards' active sets name exactly the live entries"
    );
}

// ---------------------------------------------------------------------------
// The capability pins (WIRE-07, D-13, D-14; P8)
// ---------------------------------------------------------------------------

/// A fixed qid for the capability pins: a non-zero path and version and the
/// directory type bit, so every field of the 16-byte tagged body is non-zero.
fn fixed_qid() -> Qid {
    Qid { path: 0x0102_0304_0506_0708, vers: 0xAABB_CCDD, typ: 0x80 }
}

#[test]
fn aead_fid_stateless_two_tables_one_key_verify_each_other() {
    let table_a = FidTable::new(1, 8, domain_key(0x50), 9).expect("a one-shard table forms");
    let table_b = FidTable::new(1, 8, domain_key(0x50), 9).expect("a second table with the same key");
    let cap_a = table_a.issue_cap(&fixed_qid(), 0x02);
    assert!(
        table_b.verify_cap(&cap_a).is_ok(),
        "a second table built from the same domain key verifies the capability (D-13)"
    );
    let cap_b = table_b.issue_cap(&fixed_qid(), 0x02);
    assert!(
        table_a.verify_cap(&cap_b).is_ok(),
        "and the first table verifies the second's capability too (D-13)"
    );
}

#[test]
fn aead_fid_stateless_uses_zero_fid_memory() {
    let probe = CountingSlabProbe::new();
    let table = FidTable::with_probe(2, 8, domain_key(0x51), 4, probe.clone())
        .expect("two 8-slot shards form a table");
    probe.reset();
    let cap = table.issue_cap(&fixed_qid(), 0x01);
    assert!(table.verify_cap(&cap).is_ok(), "a freshly issued capability verifies");
    assert_eq!(
        probe.reads(),
        0,
        "issue and verify touch no slot at all: zero FID memory (WIRE-08)"
    );
    for shard in 0..2u8 {
        assert_eq!(
            table.bitmap_lens(shard),
            Some((0, 0)),
            "shard {shard}'s active and reclaimed sets stay empty across issue and verify"
        );
    }
}

#[test]
fn aead_fid_stateless_different_key_rejects() {
    let table_a = FidTable::new(1, 8, domain_key(0x52), 0).expect("a one-shard table forms");
    let table_b = FidTable::new(1, 8, domain_key(0x53), 0).expect("a table with another key forms");
    let cap = table_a.issue_cap(&fixed_qid(), 0x01);
    assert!(
        table_b.verify_cap(&cap).is_err(),
        "a table with a different domain key rejects the capability (D-13)"
    );
}

#[test]
fn aead_fid_stateless_every_flipped_byte_rejects() {
    let table = FidTable::new(1, 8, domain_key(0x54), 2).expect("a one-shard table forms");
    let cap = table.issue_cap(&fixed_qid(), 0x03);
    let wire = cap.to_wire();
    for index in 0..STATELESS_CAP_LEN {
        let mut flipped = wire;
        flipped[index] ^= 0x01;
        let tampered = StatelessCap::from_wire(&flipped);
        assert!(
            table.verify_cap(&tampered).is_err(),
            "flipping wire byte {index} must fail verification (T-04-34)"
        );
    }
}

#[test]
fn aead_fid_stateless_wrong_epoch_rejects() {
    let issuer = FidTable::new(1, 8, domain_key(0x55), 5).expect("a one-shard table forms");
    let mut other = FidTable::new(1, 8, domain_key(0x55), 5).expect("a table with the same key forms");
    let cap = issuer.issue_cap(&fixed_qid(), 0x01);
    assert!(other.verify_cap(&cap).is_ok(), "the same epoch verifies the capability");
    other.set_epoch(6);
    assert!(
        other.verify_cap(&cap).is_err(),
        "a table at epoch e+1 rejects a capability issued at epoch e (WIRE-08)"
    );
}

#[test]
fn aead_fid_stateless_tag_is_keyed_blake3_of_label_and_wire_body() {
    // The domain key is bytes 0x40..=0x5f and the label is a literal in the
    // test, so nothing on the expected side goes through issue_cap,
    // verify_cap, cap_tag or StatelessCap::body (WIRE-07, D-14, P8).
    let key_bytes: [u8; 32] = core::array::from_fn(|i| 0x40 + i as u8);
    let table = FidTable::new(1, 8, Key::from_bytes(key_bytes), 0x1357)
        .expect("a one-shard table forms");
    let cap = table.issue_cap(&fixed_qid(), 0x02);

    // The 24-byte wire form of the layout, written by hand:
    // qid_path(8) qid_vers(4) qid_type(1) mode(1) epoch(2) then the tag.
    const BODY: [u8; 16] = [
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // qid_path LE
        0xDD, 0xCC, 0xBB, 0xAA, // qid_vers LE
        0x80, // qid_type
        0x02, // mode
        0x57, 0x13, // epoch LE
    ];
    let wire = cap.to_wire();
    assert_eq!(&wire[0..16], &BODY[..], "the capability's first 16 bytes are the field layout");

    // The tag is the first 8 bytes of keyed BLAKE3 under the domain key over
    // the literal label followed by those 16 bytes.
    let mut message = [0u8; 32];
    message[..16].copy_from_slice(b"r9 StatelessCap1");
    message[16..].copy_from_slice(&BODY);
    let expected = crypto::mac::blake3_keyed(&key_bytes, &message);
    assert_eq!(
        &wire[16..24],
        &expected[..8],
        "the 8-byte tag is the truncation of keyed BLAKE3 over the label and the wire body (P8)"
    );
}

// ---------------------------------------------------------------------------
// SMP-13 / D-28: the boot-switch-gated in-kernel 9P2000.aead loopback
// server, driven through the public API exactly as userland reaches '#M'.
// ---------------------------------------------------------------------------

#[test]
fn aead_loop_boot_switch_is_exact_and_starts_nothing_by_default() {
    assert!(!port::aead_loop::parse_boot_switch("console=ttyS0 root=/dev/vda"));
    assert!(!port::aead_loop::parse_boot_switch("aeadloop=0"));
    assert!(!port::aead_loop::parse_boot_switch("aeadloop=11"));
    assert!(port::aead_loop::parse_boot_switch("aeadloop=1"));
    assert!(
        port::aead_loop::start("console=ttyS0 root=/dev/vda").is_none(),
        "a default boot starts no loopback server (D-28)"
    );
}

#[test]
fn aead_loop_public_api_reads_a_deterministic_file() {
    let Some(loopback) = port::aead_loop::start("aeadloop=1") else {
        panic!("the exact aeadloop=1 token must start the loopback");
    };
    let root = loopback.attach_root(b"").expect("the active #M attach returns a root chan");
    let walked = port::namec::walk_one(&root, b"data3").expect("walk data3");
    assert_eq!(walked.qid.path, 0x1000 + 1 + 3, "data3 carries its deterministic qid path");
    let mut opened = port::namec::open_chan(&walked, port::dat::OREAD).expect("open data3");
    let mut buf = vec![0u8; 4096];
    let n = port::devmnt::read(&mut opened, &mut buf, 0).expect("read data3") as usize;
    assert_eq!(n, 4096, "the whole 4 KiB file is returned");
    for (off, byte) in buf[..n].iter().enumerate() {
        let expected = 3u8
            .wrapping_mul(37)
            .wrapping_add((off as u8).wrapping_mul(3))
            .wrapping_add(1);
        assert_eq!(*byte, expected, "data3 byte {off} matches the deterministic pattern");
    }
    drop(walked);
    port::namec::close_chan(&opened);
}
